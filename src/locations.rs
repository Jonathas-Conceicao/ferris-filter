//! The per-user FerrisFilter folder.
//!
//! One folder holds `styles.json`, `tiers.json`, `ff-settings.json` and `prices.json`. It is
//! `FERRISFILTER_HOME` when that is set (it must be an existing directory), otherwise the OS
//! per-user config folder plus the app name. There is no walk-up from the executable and no
//! working-directory fallback.
//!
//! The folder is never created here: writers create it on a real write, so a first launch writes
//! nothing. This module is the only code in the crate that asks the OS for a folder.
//!
//! It also resolves the game folder (where the `.filter` files live): `FERRISFILTER_GAME_DIR`,
//! else the folder the user picked (`ff-settings.json`), else the OS Documents folder plus
//! `My Games\Path of Exile 2`. That folder is never created here either.
//!
//! [`resolve_from`] is the pure core (every input is a parameter); [`from_env`] is the thin
//! wrapper that reads the environment and the OS. Paths are never canonicalized (`\\?\` paths
//! would change every rendered message).

use std::ffi::OsStr;
use std::path::{Path, PathBuf};

/// Environment variable that pins the FerrisFilter folder.
pub const HOME_ENV: &str = "FERRISFILTER_HOME";

/// Folder name under the OS per-user config folder.
#[cfg(windows)]
pub const APP_DIR_NAME: &str = "FerrisFilter";
/// Folder name under the OS per-user config folder.
#[cfg(not(windows))]
pub const APP_DIR_NAME: &str = "ferrisfilter";

/// Why the folder could not be resolved.
#[derive(Debug, thiserror::Error, miette::Diagnostic)]
pub enum RootError {
    /// `FERRISFILTER_HOME` is set but does not name an existing directory.
    #[error("FERRISFILTER_HOME is set to `{path}`, which is not an existing directory")]
    HomeNotADirectory { path: String },

    /// The OS has no per-user config folder.
    #[error("Cannot find the per-user config folder for FerrisFilter")]
    #[diagnostic(help("Set FERRISFILTER_HOME to the folder FerrisFilter should use."))]
    NoConfigDir,
}

/// The resolved per-user folder.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppDirs {
    pub root: PathBuf,
}

impl AppDirs {
    /// Wrap a folder that is already known (tests, callers holding a root).
    pub fn at(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }
}

/// Pure resolver. An empty `home` counts as unset; a set `home` must be an existing directory and
/// never falls through. `platform` is called lazily, only when `home` is unset.
pub fn resolve_from(
    home: Option<&OsStr>,
    platform: impl FnOnce() -> Option<PathBuf>,
) -> Result<AppDirs, RootError> {
    if let Some(value) = home.filter(|v| !v.is_empty()) {
        let dir = PathBuf::from(value);
        if dir.is_dir() {
            return Ok(AppDirs { root: dir });
        }
        return Err(RootError::HomeNotADirectory {
            path: dir.display().to_string(),
        });
    }
    match platform() {
        Some(base) => Ok(AppDirs {
            root: base.join(APP_DIR_NAME),
        }),
        None => Err(RootError::NoConfigDir),
    }
}

/// Resolve from the real environment and OS. The only place the crate asks the OS for a folder.
pub fn from_env() -> Result<AppDirs, RootError> {
    #[cfg(test)]
    {
        let set = std::env::var_os(HOME_ENV).is_some_and(|v| !v.is_empty());
        if !set {
            panic!(
                "tests must not resolve the real per-user folder: build AppDirs::at(<temp dir>) instead"
            );
        }
    }
    resolve_from(std::env::var_os(HOME_ENV).as_deref(), dirs::config_dir)
}

/// Environment variable that pins the game folder (it wins over everything else).
pub const GAME_DIR_ENV: &str = "FERRISFILTER_GAME_DIR";

/// Folders under the OS Documents folder that make up the default game folder.
pub const GAME_SUBDIRS: [&str; 2] = ["My Games", "Path of Exile 2"];

fn documents_game_dir(documents: impl FnOnce() -> Option<PathBuf>) -> Option<PathBuf> {
    documents().map(|d| GAME_SUBDIRS.iter().fold(d, |p, s| p.join(s)))
}

/// Pure game-folder resolver. A non-empty `env` wins and is returned verbatim, even when it does
/// not exist (so a missing override reaches the missing-folder status). Otherwise a `saved` path
/// is used when absolute; otherwise the Documents folder (asked lazily) plus [`GAME_SUBDIRS`].
pub fn game_dir_from(
    env: Option<&OsStr>,
    saved: Option<&Path>,
    documents: impl FnOnce() -> Option<PathBuf>,
) -> Option<PathBuf> {
    if let Some(value) = env.filter(|v| !v.is_empty()) {
        return Some(PathBuf::from(value));
    }
    if let Some(p) = saved.filter(|p| p.is_absolute()) {
        return Some(p.to_path_buf());
    }
    documents_game_dir(documents)
}

/// Resolve the game folder from the real environment and OS.
pub fn game_dir(saved: Option<&Path>) -> Option<PathBuf> {
    #[cfg(test)]
    {
        let set = std::env::var_os(GAME_DIR_ENV).is_some_and(|v| !v.is_empty());
        if !set {
            panic!("tests must not resolve the real game folder: pass a temp game_dir instead");
        }
    }
    game_dir_from(
        std::env::var_os(GAME_DIR_ENV).as_deref(),
        saved,
        dirs::document_dir,
    )
}

/// The default game folder (Documents plus [`GAME_SUBDIRS`]), ignoring the override and any saved
/// choice. Shown in the Browse tooltip.
pub fn default_game_dir() -> Option<PathBuf> {
    #[cfg(test)]
    {
        panic!("tests must not resolve the real game folder: pass a temp game_dir instead");
    }
    #[cfg(not(test))]
    documents_game_dir(dirs::document_dir)
}

/// Whether `FERRISFILTER_GAME_DIR` pins the game folder (the user cannot pick another).
pub fn game_dir_is_pinned() -> bool {
    std::env::var_os(GAME_DIR_ENV).is_some_and(|v| !v.is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn game_env_wins_verbatim_even_when_missing() {
        let got = game_dir_from(Some(OsStr::new("C:\\tmp\\g")), Some(Path::new("/x")), boom);
        assert_eq!(got, Some(PathBuf::from("C:\\tmp\\g")));
    }

    #[test]
    fn game_empty_env_is_unset() {
        let got = game_dir_from(Some(OsStr::new("")), None, || Some(PathBuf::from("D")));
        assert_eq!(
            got,
            Some(PathBuf::from("D").join("My Games").join("Path of Exile 2"))
        );
    }

    #[test]
    fn game_saved_absolute_is_used_verbatim() {
        let tmp = tempfile::tempdir().unwrap();
        let saved = tmp.path().join("not-created");
        assert_eq!(game_dir_from(None, Some(&saved), boom), Some(saved));
    }

    #[test]
    fn game_saved_relative_is_ignored() {
        let got = game_dir_from(None, Some(Path::new("rel/dir")), || {
            Some(PathBuf::from("D"))
        });
        assert_eq!(
            got,
            Some(PathBuf::from("D").join("My Games").join("Path of Exile 2"))
        );
    }

    #[test]
    fn game_default_is_documents_plus_subdirs() {
        let got = game_dir_from(None, None, || Some(PathBuf::from("Docs")));
        assert_eq!(
            got,
            Some(
                PathBuf::from("Docs")
                    .join("My Games")
                    .join("Path of Exile 2")
            )
        );
    }

    #[test]
    fn game_no_documents_is_none() {
        assert_eq!(game_dir_from(None, None, || None), None);
    }

    #[test]
    fn game_resolver_creates_nothing() {
        let tmp = tempfile::tempdir().unwrap();
        let base = tmp.path().to_path_buf();
        let got = game_dir_from(None, None, || Some(base.clone())).unwrap();
        assert!(!got.exists());
        assert_eq!(std::fs::read_dir(tmp.path()).unwrap().count(), 0);
    }

    fn boom() -> Option<PathBuf> {
        panic!("platform lookup must not be called");
    }

    #[test]
    fn env_dir_wins_and_is_returned_exactly() {
        let tmp = tempfile::tempdir().unwrap();
        let got = resolve_from(Some(tmp.path().as_os_str()), boom).unwrap();
        assert_eq!(got.root, tmp.path());
    }

    #[test]
    fn platform_not_called_when_home_set() {
        let tmp = tempfile::tempdir().unwrap();
        assert!(resolve_from(Some(tmp.path().as_os_str()), boom).is_ok());
    }

    #[test]
    fn env_missing_dir_is_error_and_does_not_fall_through() {
        let tmp = tempfile::tempdir().unwrap();
        let missing = tmp.path().join("nope");
        let err = resolve_from(Some(missing.as_os_str()), boom).unwrap_err();
        match err {
            RootError::HomeNotADirectory { path } => {
                assert_eq!(path, missing.display().to_string())
            }
            other => panic!("unexpected {other:?}"),
        }
    }

    #[test]
    fn env_pointing_at_file_is_error() {
        let tmp = tempfile::tempdir().unwrap();
        let file = tmp.path().join("f.txt");
        std::fs::write(&file, "x").unwrap();
        let err = resolve_from(Some(file.as_os_str()), boom).unwrap_err();
        assert!(matches!(err, RootError::HomeNotADirectory { .. }));
    }

    #[test]
    fn empty_env_is_unset() {
        let tmp = tempfile::tempdir().unwrap();
        let base = tmp.path().to_path_buf();
        let got = resolve_from(Some(OsStr::new("")), || Some(base.clone())).unwrap();
        assert_eq!(got.root, base.join(APP_DIR_NAME));
        let got = resolve_from(None, || Some(base.clone())).unwrap();
        assert_eq!(got.root, base.join(APP_DIR_NAME));
    }

    #[test]
    fn per_user_folder_is_not_created() {
        let tmp = tempfile::tempdir().unwrap();
        let base = tmp.path().to_path_buf();
        let got = resolve_from(None, || Some(base.clone())).unwrap();
        assert!(!got.root.exists());
    }

    #[test]
    fn no_config_dir_is_an_error() {
        let err = resolve_from(None, || None).unwrap_err();
        assert!(matches!(err, RootError::NoConfigDir));
        let help = miette::Diagnostic::help(&err).unwrap().to_string();
        assert!(help.contains("FERRISFILTER_HOME"));
    }

    #[test]
    fn app_dir_name_per_platform() {
        if cfg!(windows) {
            assert_eq!(APP_DIR_NAME, "FerrisFilter");
        } else {
            assert_eq!(APP_DIR_NAME, "ferrisfilter");
        }
    }
}
