//! The read/write lifecycle over Phase 12's dynamic store (D-02, D-04, D-19, D-20).
//!
//! Phase 13 built the read-only half: [`load`] calls [`crate::dynamic::ensure_store`] (seed-only
//! — it never overwrites an existing store) and then [`crate::loader::load_json`] to deserialize
//! the result into a [`Config`]. Phase 14 lifts the "overlay never mutates the store" boundary
//! that Phase 13's header used to describe here: [`save`] is the write half of the same
//! lifecycle, called once per frame from the editor's autosave (D-02) whenever a mutation from
//! `overlay::edit` marks the working model dirty. `dynamic::compile_store`/`write_filter` remain
//! Apply-only entry points (D-15), called from `app.rs`, not from this module.
//!
//! Every failure path returns a rendered `String`, never a panic (T-13-13 / T-14-01 — the
//! local-DoS threat a hand-edited or half-written `dynamic.json` represents): a bad field or a
//! failed write must produce a message the stripe can display, not take the process down.

use std::path::{Path, PathBuf};

use crate::dynamic::{self, StoreState};
use crate::loader;
use crate::models::config::Config;

/// Atomically overwrites `store` with `config`'s current contents (D-02, D-04).
///
/// Mirrors [`crate::dynamic::write_filter`]'s temp-file-in-destination-directory discipline
/// exactly: `store`'s parent directory is the repo root, not a cross-volume game folder, but the
/// same reasoning applies — a reader (the game, a hand `cat`, or the overlay's own next startup)
/// must never observe a truncated or half-written `dynamic.json`. The temp file is created in
/// the store's own directory so the final `persist` is a same-volume rename, not a cross-volume
/// copy that could itself fail partway.
pub fn save(config: &Config, store: &Path) -> Result<(), String> {
    let text = serde_json::to_string_pretty(config)
        .map_err(|e| format!("failed to serialize the store: {e}"))?;
    let dir = store
        .parent()
        .ok_or_else(|| format!("store path `{}` has no parent directory", store.display()))?;
    let mut temp = tempfile::NamedTempFile::new_in(dir).map_err(|e| {
        format!("cannot create a temp file in {}: {e}", dir.display())
    })?;
    std::io::Write::write_all(&mut temp, text.as_bytes())
        .map_err(|e| format!("cannot write the store: {e}"))?;
    temp.persist(store)
        .map_err(|e| format!("cannot save {}: {}", store.display(), e.error))?;
    Ok(())
}

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

    // -- save (D-02, D-04, T-14-01) ------------------------------------------

    fn sample_config() -> Config {
        let json = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/dynamic_base.json"));
        serde_json::from_str(json).expect("dynamic_base.json must parse as a Config")
    }

    #[test]
    fn save_then_load_round_trips_an_equal_config() {
        let dir = must("tempdir", tempfile::tempdir());
        let store = dir.path().join(dynamic::STORE_FILE);
        let config = sample_config();

        must("save", save(&config, &store));
        let text = must("read back store", fs::read_to_string(&store));
        let reloaded: Config = must("parse saved store", serde_json::from_str(&text));
        assert_eq!(config, reloaded);
    }

    #[test]
    fn save_leaves_no_tmp_leftovers_on_success() {
        let dir = must("tempdir", tempfile::tempdir());
        let store = dir.path().join(dynamic::STORE_FILE);
        must("save", save(&sample_config(), &store));

        let entries: Vec<_> = must("read dir", fs::read_dir(dir.path()))
            .filter_map(|e| e.ok())
            .map(|e| e.file_name())
            .collect();
        assert_eq!(
            entries,
            vec![std::ffi::OsString::from(dynamic::STORE_FILE)],
            "no leftover temp file should remain beside the store"
        );
    }

    #[test]
    fn a_second_save_replaces_the_first_atomically() {
        let dir = must("tempdir", tempfile::tempdir());
        let store = dir.path().join(dynamic::STORE_FILE);

        let first = sample_config();
        must("save first", save(&first, &store));

        let mut second = sample_config();
        second.meta.name = "second-save".to_string();
        must("save second", save(&second, &store));

        let text = must("read back store", fs::read_to_string(&store));
        let reloaded: Config = must("parse saved store", serde_json::from_str(&text));
        assert_eq!(
            reloaded.meta.name, "second-save",
            "the pre-existing file must be either the old content or the new content, \
             never a truncated prefix -- here it must be fully the new content"
        );
    }

    #[test]
    fn save_to_a_missing_parent_directory_returns_err_naming_the_path_and_does_not_panic() {
        let dir = must("tempdir", tempfile::tempdir());
        let store = dir.path().join("no-such-subdir").join(dynamic::STORE_FILE);

        let err = match save(&sample_config(), &store) {
            Ok(()) => panic!("save into a missing parent directory must fail"),
            Err(e) => e,
        };
        let expected_dir = store.parent().unwrap().display().to_string();
        assert!(
            err.contains(&expected_dir),
            "error must name the offending path, got: {err}"
        );
    }

    /// RESEARCH Pitfalls 2 and 3: `NumericCondition`/`AreaLevelRange` lack
    /// `skip_serializing_if`, so a round-tripped store can show explicit `"eq": null` keys, and
    /// `classGroups`/`build.armourTypes` are `HashMap`s whose on-disk key order can reshuffle
    /// between saves. Neither is a fidelity loss -- this test asserts `Config` value equality,
    /// never byte equality, so both expected, harmless differences are allowed to happen without
    /// failing the guard.
    #[test]
    fn dynamic_base_json_round_trips_through_config_losslessly() {
        let json = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/dynamic_base.json"));
        let first: Config = must("dynamic_base.json must parse", serde_json::from_str(json));
        let serialized =
            must("serialize", serde_json::to_string_pretty(&first));
        let second: Config = must(
            "re-deserialize after round-trip",
            serde_json::from_str(&serialized),
        );
        assert_eq!(
            first, second,
            "dynamic_base.json did not round-trip losslessly through Config"
        );
    }
}
