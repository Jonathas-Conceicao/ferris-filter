//! The dynamic filter lifecycle: where the store comes from, and where the filter goes.
//!
//! `dynamic.json` is a full [`Config`](crate::models::config::Config), exactly like `config.json`
//! — so compiling it needs no new emitter path, just [`crate::compile`] pointed at a different
//! file. What this module adds is the lifecycle around that:
//!
//! - **seed** — create the working copy from the committed template when it is absent
//! - **reset** — restore the working copy from the template on demand
//! - **compile** — load and compile the working copy, naming itself in the header
//! - **write** — land the result in the game folder without the game ever seeing a partial file
//!
//! Two rules inherited from [`crate::compile`]:
//!
//! - **No `println!` / `eprintln!`.** Functions here return what happened; `run.rs` presents it.
//! - **No implicit working-directory reads.** Every path is a parameter, so the overlay can point
//!   at its own root.
//!
//! Note that PoE2 never re-reads a filter on its own — after writing, the player must reload it
//! via Options → Game → Item Filter → Refresh. Nothing here can change that, so nothing here
//! watches files or writes on a timer.

use std::path::{Path, PathBuf};

use crate::compile::{self, CompileError, InputPaths, PriceSource, PriceSourcePolicy};
use crate::emitter::FilterIdentity;

/// Committed template — the starting point for a new store, and the source for a reset.
/// Never written to by the tool.
pub const BASE_FILE: &str = "dynamic_base.json";
/// The working copy the overlay edits. Git-ignored: per-user state, not committed policy.
pub const STORE_FILE: &str = "dynamic.json";
/// The generated filter's conventional name.
pub const FILTER_FILE: &str = "dynamic.filter";

/// How the filter names itself and its source in the generated header.
pub const IDENTITY: FilterIdentity<'static> = FilterIdentity {
    output_name: FILTER_FILE,
    source_name: STORE_FILE,
};

/// The in-game config folder `dynamic.filter` lands in by default.
///
/// Delegates to the same resolution `install` uses rather than re-spelling the path segments —
/// there should be exactly one place in the crate that knows where Path of Exile 2 keeps its
/// filters. Returns `None` when `USERPROFILE` is unset (non-Windows), in which case the caller
/// must require an explicit destination.
pub fn default_destination() -> Option<PathBuf> {
    crate::install::default_dest()
}

/// What [`ensure_store`] did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StoreState {
    /// The working copy already existed and was left untouched.
    Existing,
    /// The working copy was absent and has been created from the template.
    Seeded,
}

/// Everything that can go wrong in the dynamic lifecycle.
#[derive(Debug, thiserror::Error, miette::Diagnostic)]
pub enum DynamicError {
    /// The template is missing, so a store cannot be seeded or reset.
    #[error("Cannot read `{path}`: {source}")]
    #[diagnostic(help("`{path}` is committed to the repository — run from the repo root, or pass an explicit path."))]
    TemplateUnreadable {
        path: String,
        #[source]
        source: std::io::Error,
    },

    /// The working copy could not be created or replaced.
    #[error("Cannot write `{path}`: {source}")]
    StoreUnwritable {
        path: String,
        #[source]
        source: std::io::Error,
    },

    /// Loading or compiling the store failed.
    #[error(transparent)]
    #[diagnostic(transparent)]
    Compile(#[from] CompileError),

    /// The destination folder does not exist. Deliberately not created — a typo'd path should
    /// fail loudly rather than scatter filter files somewhere unexpected.
    #[error("Destination folder not found: `{path}` — make sure Path of Exile 2 has been run at least once, or pass an explicit destination.")]
    DestinationMissing { path: String },

    /// The filter could not be written to the destination.
    #[error("Cannot write `{path}`: {source}")]
    FilterUnwritable {
        path: String,
        #[source]
        source: std::io::Error,
    },
}

/// Create the working copy from the template if it does not exist yet.
///
/// An existing store is **never** overwritten — it holds the user's edits. Use [`reset_store`]
/// to deliberately discard them.
pub fn ensure_store(base: &Path, store: &Path) -> Result<StoreState, DynamicError> {
    if store.exists() {
        return Ok(StoreState::Existing);
    }
    copy_template(base, store)?;
    Ok(StoreState::Seeded)
}

/// Replace the working copy with the template, discarding whatever was there.
///
/// This is the "start a new filter from scratch" path — the reason the template is a separate
/// committed file rather than something generated once.
pub fn reset_store(base: &Path, store: &Path) -> Result<(), DynamicError> {
    copy_template(base, store)
}

fn copy_template(base: &Path, store: &Path) -> Result<(), DynamicError> {
    let contents = std::fs::read(base).map_err(|e| DynamicError::TemplateUnreadable {
        path: base.display().to_string(),
        source: e,
    })?;
    std::fs::write(store, contents).map_err(|e| DynamicError::StoreUnwritable {
        path: store.display().to_string(),
        source: e,
    })
}

/// The outcome of compiling the dynamic store.
#[derive(Debug)]
pub struct CompiledFilter {
    /// The filter text, LF-only, ending in exactly one newline.
    pub text: String,
    /// Which price source backed the economy sections — returned, not printed, so the caller
    /// decides how to surface it.
    pub price_source: PriceSource,
}

/// Load the working copy and compile it into filter text.
///
/// `paths` supplies every input location; its `config` field is overridden with `store` so the
/// dynamic store is what gets compiled while prices and tiers still resolve normally. The economy
/// sections come along automatically — the same pricer runs — which is why `dynamic.filter` is a
/// complete, playable filter rather than a policy-only fragment.
pub fn compile_store(paths: &InputPaths, store: &Path) -> Result<CompiledFilter, DynamicError> {
    let paths = paths.clone().with_config(store);
    let inputs = compile::load_inputs(&paths, PriceSourcePolicy::Auto)?;
    let text = compile::compile_as(&inputs, &IDENTITY)?;
    Ok(CompiledFilter {
        text,
        price_source: inputs.price_source,
    })
}

/// Write filter text into `dest_dir` as `dynamic.filter`, atomically.
///
/// The temp file is created **in the destination directory**, not the system temp dir: the game
/// folder is typically on a different volume (OneDrive), and `rename` across volumes fails. The
/// game may read the filter at any moment the player hits Refresh, so it must never observe a
/// partially written file.
///
/// Returns the path written. Errors rather than creating a missing destination, matching
/// `install`'s guard.
pub fn write_filter(dest_dir: &Path, text: &str) -> Result<PathBuf, DynamicError> {
    if !dest_dir.is_dir() {
        return Err(DynamicError::DestinationMissing {
            path: dest_dir.display().to_string(),
        });
    }

    let target = dest_dir.join(FILTER_FILE);

    let mut temp = tempfile::NamedTempFile::new_in(dest_dir).map_err(|e| {
        DynamicError::FilterUnwritable {
            path: target.display().to_string(),
            source: e,
        }
    })?;
    std::io::Write::write_all(&mut temp, text.as_bytes()).map_err(|e| {
        DynamicError::FilterUnwritable {
            path: target.display().to_string(),
            source: e,
        }
    })?;
    temp.persist(&target)
        .map_err(|e| DynamicError::FilterUnwritable {
            path: target.display().to_string(),
            source: e.error,
        })?;

    Ok(target)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn repo_file(name: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(name)
    }

    fn fixture(name: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests")
            .join("fixtures")
            .join(name)
    }

    #[test]
    fn ensure_store_seeds_when_absent() {
        let dir = tempfile::tempdir().unwrap();
        let base = repo_file(BASE_FILE);
        let store = dir.path().join(STORE_FILE);

        let state = ensure_store(&base, &store).expect("seed");
        assert_eq!(state, StoreState::Seeded);
        assert_eq!(
            fs::read(&store).unwrap(),
            fs::read(&base).unwrap(),
            "a seeded store must be byte-identical to the template"
        );
    }

    /// The only destructive failure mode in this module: seeding must never clobber the user's
    /// edits.
    #[test]
    fn ensure_store_never_overwrites_an_existing_store() {
        let dir = tempfile::tempdir().unwrap();
        let base = repo_file(BASE_FILE);
        let store = dir.path().join(STORE_FILE);

        fs::write(&store, b"{\"edited by the user\": true}").unwrap();
        let state = ensure_store(&base, &store).expect("ensure");
        assert_eq!(state, StoreState::Existing);
        assert_eq!(
            fs::read_to_string(&store).unwrap(),
            "{\"edited by the user\": true}",
            "an existing store must be left exactly as it was"
        );
    }

    #[test]
    fn reset_store_overwrites_an_altered_store() {
        let dir = tempfile::tempdir().unwrap();
        let base = repo_file(BASE_FILE);
        let store = dir.path().join(STORE_FILE);

        fs::write(&store, b"{\"stale\": true}").unwrap();
        reset_store(&base, &store).expect("reset");
        assert_eq!(
            fs::read(&store).unwrap(),
            fs::read(&base).unwrap(),
            "reset must restore the template exactly"
        );
    }

    #[test]
    fn write_filter_refuses_a_missing_destination() {
        let dir = tempfile::tempdir().unwrap();
        let missing = dir.path().join("no-such-folder");
        let err = write_filter(&missing, "# filter\n").expect_err("must refuse");
        assert!(matches!(err, DynamicError::DestinationMissing { .. }));
        assert!(
            !missing.exists(),
            "a missing destination must not be created"
        );
    }

    #[test]
    fn write_filter_leaves_the_previous_file_intact_on_failure() {
        let dir = tempfile::tempdir().unwrap();
        let dest = dir.path().join("game");
        fs::create_dir(&dest).unwrap();
        write_filter(&dest, "first\n").expect("first write");

        // A missing destination fails before any temp file is created, so the previously written
        // filter is untouched.
        let _ = write_filter(&dir.path().join("gone"), "second\n");
        assert_eq!(
            fs::read_to_string(dest.join(FILTER_FILE)).unwrap(),
            "first\n"
        );
    }

    #[test]
    fn write_filter_second_write_wins() {
        let dir = tempfile::tempdir().unwrap();
        write_filter(dir.path(), "first\n").expect("first");
        let path = write_filter(dir.path(), "second\n").expect("second");
        assert_eq!(fs::read_to_string(&path).unwrap(), "second\n");
        assert_eq!(path.file_name().unwrap(), FILTER_FILE);
    }

    #[test]
    fn compile_store_produces_a_complete_filter_with_economy_sections() {
        let dir = tempfile::tempdir().unwrap();
        fs::copy(repo_file("tiers.json"), dir.path().join("tiers.json")).unwrap();
        fs::copy(fixture("seed-prices.json"), dir.path().join("prices.json")).unwrap();
        let store = dir.path().join(STORE_FILE);
        ensure_store(&repo_file(BASE_FILE), &store).expect("seed");

        let compiled =
            compile_store(&InputPaths::relative_to(dir.path()), &store).expect("compile");

        assert_eq!(compiled.price_source, PriceSource::Live);
        assert!(
            compiled.text.starts_with("# dynamic.filter\n"),
            "the filter must name itself, got: {:?}",
            compiled.text.lines().next()
        );
        assert!(
            compiled.text.contains("# GENERATED FROM dynamic.json"),
            "the filter must name its source"
        );
        assert!(
            compiled.text.contains("Currency"),
            "economy sections must be present — a policy-only filter would lose every currency highlight"
        );
        assert!(compiled.text.contains("Show # rule 0"));
    }
}
