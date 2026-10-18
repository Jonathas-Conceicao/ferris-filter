//! App-root resolver (STORE-05, D-12).
//!
//! `ff.exe` must work when double-clicked from Explorer, where the working directory is not the
//! repo. One resolver finds the FerrisFilter folder (the one holding the committed
//! `styles-default.json`) in this order:
//!
//! 1. the `FERRISFILTER_HOME` environment variable (must be an existing directory, otherwise an
//!    error: never a silent fall-through);
//! 2. walk up from the executable's directory until a root marker is found;
//! 3. the working directory, only if it holds a root marker;
//! 4. otherwise a visible error listing what was tried.
//!
//! [`resolve_from`] is the pure core: every input is a parameter. [`app_root`] is the thin
//! wrapper and the only function here that reads the environment, exe path and working directory.
//!
//! Deliberate asymmetry: `generate`, `install` and `sync` stay working-directory relative.

use std::ffi::OsStr;
use std::path::{Path, PathBuf};

pub use crate::locations::HOME_ENV;

/// Why the app root could not be resolved.
#[derive(Debug, thiserror::Error, miette::Diagnostic)]
pub enum RootError {
    /// `FERRISFILTER_HOME` is set but does not name an existing directory.
    #[error("FERRISFILTER_HOME is set to `{path}`, which is not an existing directory")]
    HomeNotADirectory { path: String },

    /// No candidate held a root marker.
    #[error("Cannot find the FerrisFilter folder (the one holding styles-default.json): {tried}")]
    #[diagnostic(help(
        "Set FERRISFILTER_HOME to the FerrisFilter folder, or keep ff.exe inside it."
    ))]
    NotFound { tried: String },
}

/// A directory is a root when it holds the committed `styles-default.json` (DEF-04). The file
/// name comes from `InputPaths` so it is spelled in exactly one place.
///
/// `styles-default.json` alone is the marker. The live `styles.json` is a git-ignored per-user
/// file that may not exist yet (reads fall back to the default), so it does not mark a root.
/// `config.json` is deliberately not accepted either (the Phase 18 context allowed it):
/// `config.json` is a very common file name, so an unrelated one above the exe would stop the
/// walk-up at the wrong folder and hide a correct working-directory fallback. And `apply`/the
/// overlay never read `config.json` -- they need the styles, so a folder holding only
/// `config.json` could not serve them anyway.
fn is_root(dir: &Path) -> bool {
    crate::compile::InputPaths::relative_to(dir)
        .styles_default
        .is_some_and(|p| p.is_file())
}

/// Pure resolver: see the module docs for the order.
pub fn resolve_from(
    env: Option<&OsStr>,
    exe_dir: Option<&Path>,
    cwd: Option<&Path>,
) -> Result<PathBuf, RootError> {
    if let Some(value) = env.filter(|v| !v.is_empty()) {
        let dir = PathBuf::from(value);
        if dir.is_dir() {
            return Ok(dir);
        }
        return Err(RootError::HomeNotADirectory {
            path: dir.display().to_string(),
        });
    }

    if let Some(exe_dir) = exe_dir {
        if let Some(found) = exe_dir.ancestors().find(|d| is_root(d)) {
            return Ok(found.to_path_buf());
        }
    }

    if let Some(cwd) = cwd {
        if is_root(cwd) {
            return Ok(cwd.to_path_buf());
        }
    }

    let exe_part = match exe_dir {
        Some(d) => format!("no styles-default.json above `{}`", d.display()),
        None => "the executable's directory is unknown".to_string(),
    };
    let cwd_part = match cwd {
        Some(d) => format!("none in the working directory `{}`", d.display()),
        None => "the working directory is unknown".to_string(),
    };
    Err(RootError::NotFound {
        tried: format!("{HOME_ENV} is not set; {exe_part}; {cwd_part}"),
    })
}

/// Resolve the app root from the real environment, executable location and working directory.
/// Never canonicalizes (`\\?\` paths would change every rendered message).
pub fn app_root() -> Result<PathBuf, RootError> {
    let exe = std::env::current_exe().ok();
    let cwd = std::env::current_dir().ok();
    resolve_from(
        std::env::var_os(HOME_ENV).as_deref(),
        exe.as_deref().and_then(Path::parent),
        cwd.as_deref(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn marker(dir: &Path, name: &str) {
        fs::write(dir.join(name), "{}").unwrap();
    }

    #[test]
    fn env_dir_wins_without_marker() {
        let t = tempfile::tempdir().unwrap();
        let r = resolve_from(Some(t.path().as_os_str()), None, None).unwrap();
        assert_eq!(r, t.path());
    }

    #[test]
    fn env_missing_dir_is_error_and_does_not_fall_through() {
        let t = tempfile::tempdir().unwrap();
        marker(t.path(), "styles-default.json");
        let missing = t.path().join("nope");
        let err = resolve_from(Some(missing.as_os_str()), Some(t.path()), Some(t.path()));
        assert!(matches!(err, Err(RootError::HomeNotADirectory { .. })));
    }

    #[test]
    fn env_pointing_at_file_is_error() {
        let t = tempfile::tempdir().unwrap();
        marker(t.path(), "styles-default.json");
        let f = t.path().join("styles-default.json");
        let err = resolve_from(Some(f.as_os_str()), None, None);
        assert!(matches!(err, Err(RootError::HomeNotADirectory { .. })));
    }

    #[test]
    fn empty_env_is_unset() {
        let t = tempfile::tempdir().unwrap();
        marker(t.path(), "styles-default.json");
        let r = resolve_from(Some(OsStr::new("")), None, Some(t.path())).unwrap();
        assert_eq!(r, t.path());
    }

    #[test]
    fn exe_walk_up_finds_nearest_ancestor_with_styles() {
        let t = tempfile::tempdir().unwrap();
        marker(t.path(), "styles-default.json");
        let exe = t.path().join("target").join("debug");
        fs::create_dir_all(&exe).unwrap();
        let r = resolve_from(None, Some(&exe), None).unwrap();
        assert_eq!(r, t.path());
    }

    #[test]
    fn config_json_alone_does_not_stop_the_walk_up() {
        // An unrelated config.json above the exe must not capture the root: the walk-up passes
        // it and the working directory holding styles-default.json wins.
        let t = tempfile::tempdir().unwrap();
        marker(t.path(), "config.json");
        let exe = t.path().join("a").join("b");
        fs::create_dir_all(&exe).unwrap();
        let cwd = tempfile::tempdir().unwrap();
        marker(cwd.path(), "styles-default.json");
        let r = resolve_from(None, Some(&exe), Some(cwd.path())).unwrap();
        assert_eq!(r, cwd.path());
    }

    #[test]
    fn exe_nearest_ancestor_wins() {
        let t = tempfile::tempdir().unwrap();
        marker(t.path(), "styles-default.json");
        let inner = t.path().join("inner");
        fs::create_dir_all(&inner).unwrap();
        marker(&inner, "styles-default.json");
        let exe = inner.join("bin");
        fs::create_dir_all(&exe).unwrap();
        assert_eq!(resolve_from(None, Some(&exe), None).unwrap(), inner);
    }

    #[test]
    fn falls_back_to_cwd_with_marker() {
        let exe = tempfile::tempdir().unwrap();
        let cwd = tempfile::tempdir().unwrap();
        marker(cwd.path(), "styles-default.json");
        // The exe tempdir sits under the system temp dir, which holds no marker above it.
        let r = resolve_from(None, Some(exe.path()), Some(cwd.path())).unwrap();
        assert_eq!(r, cwd.path());
    }

    #[test]
    fn cwd_without_marker_is_not_found_and_lists_attempts() {
        let cwd = tempfile::tempdir().unwrap();
        let exe = tempfile::tempdir().unwrap();
        let err = resolve_from(None, Some(exe.path()), Some(cwd.path())).unwrap_err();
        assert!(matches!(err, RootError::NotFound { .. }));
        let msg = err.to_string();
        assert!(msg.contains("FERRISFILTER_HOME"), "{msg}");
        assert!(msg.contains(&exe.path().display().to_string()), "{msg}");
        assert!(msg.contains(&cwd.path().display().to_string()), "{msg}");
    }

    /// DEF-04: the live `styles.json` is a per-user file and no longer marks the root; only the
    /// committed `styles-default.json` does.
    #[test]
    fn live_styles_json_alone_is_not_a_root() {
        let exe = tempfile::tempdir().unwrap();
        marker(exe.path(), "styles.json");
        let cwd = tempfile::tempdir().unwrap();
        marker(cwd.path(), "styles.json");
        let err = resolve_from(None, Some(exe.path()), Some(cwd.path())).unwrap_err();
        assert!(matches!(err, RootError::NotFound { .. }));
        assert!(err.to_string().contains("styles-default.json"), "{err}");
    }

    #[test]
    fn nothing_known_is_not_found() {
        let err = resolve_from(None, None, None).unwrap_err();
        assert!(matches!(err, RootError::NotFound { .. }));
    }
}
