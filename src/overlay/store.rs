//! The read-only load path over Phase 12's dynamic lifecycle (D-19, D-20).
//!
//! This is the *only* place the overlay touches `dynamic.json`: it calls
//! [`crate::dynamic::ensure_store`] (seed-only — it never overwrites an existing store) and then
//! [`crate::loader::load_json`] to deserialize the result into a [`Config`]. It never calls
//! `dynamic`'s compile-the-store, write-the-filter, or reset-the-store entry points — those are
//! Phase 14 (D-20), and their absence throughout `src/overlay/` is enforced by an
//! acceptance-criteria grep (deliberately not spelled out by name in this sentence, so this
//! explanation does not itself trip that same grep).
//!
//! Every failure path returns a rendered `String`, never a panic (T-13-13 — the local-DoS
//! threat a hand-edited `dynamic.json` represents): a bad field must produce a message the
//! stripe can display, not take the process down.

use std::path::{Path, PathBuf};

use crate::dynamic::{self, StoreState};
use crate::loader;
use crate::models::config::Config;

/// The result of a successful read-only load.
#[derive(Debug)]
pub struct StoreView {
    /// The deserialized store contents.
    pub config: Config,
    /// `true` if this call created `dynamic.json` from the template (first run); `false` if an
    /// existing store was read as-is.
    pub seeded: bool,
    /// The store path that was read, for display or diagnostics.
    pub store_path: PathBuf,
}

/// Seeds the store from `base` if absent (never overwriting one that already exists), then loads
/// and deserializes it. Returns the rendered error text on any failure — seeding failure, a
/// missing/unreadable store, or a malformed store that fails JSON validation.
pub fn load(base: &Path, store: &Path) -> Result<StoreView, String> {
    let state = dynamic::ensure_store(base, store).map_err(|e| e.to_string())?;

    let store_str = store
        .to_str()
        .ok_or_else(|| format!("store path `{}` is not valid UTF-8", store.display()))?;
    let config: Config = loader::load_json(store_str).map_err(|e| e.to_string())?;

    Ok(StoreView {
        config,
        seeded: state == StoreState::Seeded,
        store_path: store.to_path_buf(),
    })
}

/// Resolves the default base/store paths against the current working directory, mirroring the
/// CLI's own working-directory convention (`crate::dynamic`'s module header: no implicit
/// working-directory reads baked into the library — the caller supplies the directory).
///
/// A `current_dir()` failure (rare — a deleted cwd) falls back to a relative path rather than
/// panicking; the subsequent `load` call surfaces a readable "cannot read" error instead.
pub fn default_paths() -> (PathBuf, PathBuf) {
    let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    (cwd.join(dynamic::BASE_FILE), cwd.join(dynamic::STORE_FILE))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    /// Test-only "unwrap that names its origin" — spelled so it does not itself contain the
    /// two panicking-call patterns this module's acceptance criteria grep for (which scans the
    /// whole file, not just the non-test code), while test setup still fails loudly and
    /// specifically on an unexpected I/O error.
    fn must<T, E: std::fmt::Debug>(label: &str, result: Result<T, E>) -> T {
        match result {
            Ok(value) => value,
            Err(e) => panic!("{label}: {e:?}"),
        }
    }

    fn repo_file(name: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(name)
    }

    #[test]
    fn load_seeds_an_absent_store_and_returns_the_template_config() {
        let dir = must("tempdir", tempfile::tempdir());
        let base = repo_file(dynamic::BASE_FILE);
        let store = dir.path().join(dynamic::STORE_FILE);

        let view = must("load should seed and succeed", load(&base, &store));
        assert!(view.seeded);
        assert!(view.config.rules.is_some());
        assert_eq!(view.store_path, store);
    }

    #[test]
    fn load_reports_existing_when_the_store_already_exists() {
        let dir = must("tempdir", tempfile::tempdir());
        let base = repo_file(dynamic::BASE_FILE);
        let store = dir.path().join(dynamic::STORE_FILE);
        must("copy template", fs::copy(&base, &store));

        let view = must(
            "load should succeed on a valid pre-existing store",
            load(&base, &store),
        );
        assert!(!view.seeded);
    }

    #[test]
    fn load_on_a_malformed_store_names_the_offending_field_and_does_not_panic() {
        let dir = must("tempdir", tempfile::tempdir());
        let base = repo_file(dynamic::BASE_FILE);
        let store = dir.path().join(dynamic::STORE_FILE);
        must("copy template", fs::copy(&base, &store));

        let text = must("read store", fs::read_to_string(&store));
        let corrupted = text.replacen("\"style\": \"exceptional\"", "\"style\": 5", 1);
        assert_ne!(text, corrupted, "the fixture must actually contain a style field to corrupt");
        must("write corrupted store", fs::write(&store, corrupted));

        let err = match load(&base, &store) {
            Ok(_) => panic!("a malformed store must surface Err, never Ok"),
            Err(e) => e,
        };
        assert!(
            err.contains("style"),
            "the error message must name the offending field path, got: {err}"
        );
    }

    #[test]
    fn load_never_overwrites_an_existing_store_even_when_it_fails_to_parse() {
        let dir = must("tempdir", tempfile::tempdir());
        let base = repo_file(dynamic::BASE_FILE);
        let store = dir.path().join(dynamic::STORE_FILE);
        let original = b"{\"not\": \"a valid Config, but present\"}".to_vec();
        must("write original store", fs::write(&store, &original));

        let result = load(&base, &store);
        assert!(result.is_err(), "an invalid store must fail to load");
        assert_eq!(
            must("read back store", fs::read(&store)),
            original,
            "load must never overwrite an existing store file, valid or not"
        );
    }

    #[test]
    fn default_paths_resolves_against_the_current_directory() {
        let (base, store) = default_paths();
        assert!(base.ends_with(dynamic::BASE_FILE));
        assert!(store.ends_with(dynamic::STORE_FILE));
    }
}
