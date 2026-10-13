//! Per-user overlay settings: the selected game-folder filter (STORE-02).
//!
//! `ff-settings.json` lives at the app root and is git-ignored. The library is pure: every path is
//! a parameter and nothing here prints. A filter name is only ever a bare `*.filter` file name, so
//! neither a hand-edited settings file nor a typed name can escape the game folder.

use std::fmt;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

/// File name of the settings file at the app root.
pub const SETTINGS_FILE: &str = "ff-settings.json";
/// The filter used when nothing (valid) is stored.
pub const DEFAULT_FILTER: &str = crate::dynamic::FILTER_FILE;

const MAX_NAME_LEN: usize = 200;

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Settings {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub filter: Option<String>,
    /// Unknown top-level keys survive a load/save round trip.
    #[serde(flatten)]
    pub extra: serde_json::Map<String, serde_json::Value>,
}

impl Settings {
    /// The stored filter name, or `dynamic.filter`.
    pub fn filter_name(&self) -> &str {
        self.filter.as_deref().unwrap_or(DEFAULT_FILTER)
    }
}

/// `<root>/ff-settings.json`.
pub fn path(root: &Path) -> PathBuf {
    root.join(SETTINGS_FILE)
}

/// Load the settings. A missing file is the default with no warning; an unusable file (unreadable,
/// invalid JSON, or an invalid stored name) is the default plus a warning. Never writes.
pub fn load(root: &Path) -> (Settings, Option<String>) {
    let warn = |reason: String| {
        Some(format!(
            "{SETTINGS_FILE} could not be used ({reason}); using {DEFAULT_FILTER}."
        ))
    };
    let text = match fs::read_to_string(path(root)) {
        Ok(t) => t,
        Err(e) if e.kind() == io::ErrorKind::NotFound => return (Settings::default(), None),
        Err(e) => return (Settings::default(), warn(e.to_string())),
    };
    let mut settings: Settings = match serde_json::from_str(&text) {
        Ok(s) => s,
        Err(e) => {
            // A wrongly typed `filter` must not discard the other keys: salvage them.
            return match serde_json::from_str::<serde_json::Map<String, serde_json::Value>>(&text) {
                Ok(mut extra) => {
                    extra.remove("filter");
                    (
                        Settings {
                            filter: None,
                            extra,
                        },
                        warn(e.to_string()),
                    )
                }
                Err(_) => (Settings::default(), warn(e.to_string())),
            };
        }
    };
    if let Some(name) = settings.filter.clone()
        && let Err(problem) = valid_filter_name(&name)
    {
        settings.filter = None;
        return (settings, warn(format!("`{name}`: {problem}")));
    }
    (settings, None)
}

/// Persist the settings atomically (same-directory temp file, then replace).
pub fn save(root: &Path, settings: &Settings) -> io::Result<()> {
    let mut text = serde_json::to_string_pretty(settings).map_err(io::Error::other)?;
    text.push('\n');
    let mut t = tempfile::NamedTempFile::new_in(root)?;
    t.write_all(text.as_bytes())?;
    t.as_file().sync_all()?;
    t.persist(path(root)).map_err(|e| e.error)?;
    Ok(())
}

/// Why a file name was refused.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NameProblem {
    Empty,
    NotFilter,
    Separator,
    Reserved,
    BadCharacter,
    TooLong,
}

impl fmt::Display for NameProblem {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            NameProblem::Empty => "Enter a new file name",
            NameProblem::NotFilter => "The name must end in .filter",
            NameProblem::Separator => "Enter a file name, not a folder path",
            NameProblem::Reserved => "That name is reserved by Windows",
            NameProblem::BadCharacter => {
                "File names can't contain < > : \" / \\ | ? * or control characters, or end in a dot or space"
            }
            NameProblem::TooLong => "That name is too long",
        })
    }
}

fn ends_with_ci(s: &str, suffix: &str) -> bool {
    s.len() >= suffix.len()
        && s.is_char_boundary(s.len() - suffix.len())
        && s[s.len() - suffix.len()..].eq_ignore_ascii_case(suffix)
}

fn is_device_name(stem: &str) -> bool {
    let s = stem.trim_end().to_ascii_uppercase();
    if matches!(s.as_str(), "CON" | "PRN" | "AUX" | "NUL") {
        return true;
    }
    let b = s.as_bytes();
    b.len() == 4 && (s.starts_with("COM") || s.starts_with("LPT")) && (b'1'..=b'9').contains(&b[3])
}

/// Accept only a bare `*.filter` file name that is safe to join onto the game folder.
pub fn valid_filter_name(name: &str) -> Result<(), NameProblem> {
    if name.is_empty() {
        return Err(NameProblem::Empty);
    }
    if name.chars().count() > MAX_NAME_LEN {
        return Err(NameProblem::TooLong);
    }
    if name.contains(['/', '\\']) {
        return Err(NameProblem::Separator);
    }
    if name
        .chars()
        .any(|c| c.is_control() || "<>:\"|?*".contains(c))
        || name.ends_with('.')
        || name.ends_with(' ')
    {
        return Err(NameProblem::BadCharacter);
    }
    if !ends_with_ci(name, ".filter") || name.len() == ".filter".len() {
        return Err(NameProblem::NotFilter);
    }
    let stem = name.split('.').next().unwrap_or("");
    if is_device_name(stem) {
        return Err(NameProblem::Reserved);
    }
    Ok(())
}

/// Normalise a typed name: trim, and append `.filter` unless already present (any case).
pub fn new_filter_name(input: &str) -> String {
    let t = input.trim();
    if ends_with_ci(t, ".filter") {
        t.to_string()
    } else {
        format!("{t}.filter")
    }
}

/// Regular `*.filter` files in `dir` (backups and folders excluded), sorted case-insensitively.
pub fn list_filters(dir: &Path) -> io::Result<Vec<String>> {
    let mut out = Vec::new();
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        if !entry.file_type()?.is_file() {
            continue;
        }
        if let Ok(name) = entry.file_name().into_string()
            && ends_with_ci(&name, ".filter")
        {
            out.push(name);
        }
    }
    out.sort_by_key(|n| n.to_lowercase());
    Ok(out)
}

/// Case-insensitive membership (NTFS file names are case-insensitive).
pub fn contains_name(list: &[String], name: &str) -> bool {
    list.iter().any(|n| n.eq_ignore_ascii_case(name))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_file_is_default_and_not_created() {
        let t = tempfile::tempdir().unwrap();
        let (s, w) = load(t.path());
        assert_eq!(s.filter, None);
        assert_eq!(w, None);
        assert_eq!(s.filter_name(), "dynamic.filter");
        assert!(!path(t.path()).exists());
    }

    #[test]
    fn non_string_filter_keeps_other_keys() {
        let t = tempfile::tempdir().unwrap();
        fs::write(path(t.path()), r#"{"filter": 5, "future": {"a": 1}}"#).unwrap();
        let (s, w) = load(t.path());
        assert!(w.is_some());
        assert_eq!(s.filter, None);
        assert_eq!(s.extra["future"]["a"], 1);
    }

    #[test]
    fn round_trip_preserves_unknown_keys() {
        let t = tempfile::tempdir().unwrap();
        fs::write(path(t.path()), r#"{"future": {"a": 1}}"#).unwrap();
        let (mut s, w) = load(t.path());
        assert_eq!(w, None);
        s.filter = Some("ssf.filter".into());
        save(t.path(), &s).unwrap();
        let (s2, w2) = load(t.path());
        assert_eq!(w2, None);
        assert_eq!(s2.filter_name(), "ssf.filter");
        assert_eq!(s2.extra["future"]["a"], 1);
        assert_eq!(fs::read_dir(t.path()).unwrap().count(), 1);
    }

    #[test]
    fn corrupt_file_warns_and_is_not_touched() {
        let t = tempfile::tempdir().unwrap();
        fs::write(path(t.path()), "{not json").unwrap();
        let (s, w) = load(t.path());
        assert_eq!(s.filter_name(), "dynamic.filter");
        assert!(w.unwrap().contains("ff-settings.json"));
        assert_eq!(fs::read(path(t.path())).unwrap(), b"{not json");
    }

    #[test]
    fn traversal_names_in_the_file_fall_back() {
        for bad in [r"..\\..\\evil.filter", "C:/x.filter"] {
            let t = tempfile::tempdir().unwrap();
            let body = format!(r#"{{"filter":"{bad}"}}"#);
            fs::write(path(t.path()), &body).unwrap();
            let (s, w) = load(t.path());
            assert_eq!(s.filter_name(), "dynamic.filter", "{bad}");
            assert!(w.is_some(), "{bad}");
            assert_eq!(fs::read(path(t.path())).unwrap(), body.as_bytes());
        }
    }

    #[test]
    fn name_validation() {
        for ok in ["dynamic.filter", "My Filter.FILTER"] {
            assert_eq!(valid_filter_name(ok), Ok(()), "{ok}");
        }
        let long = format!("{}.filter", "a".repeat(200));
        for bad in [
            "",
            "x.txt",
            ".filter",
            "a/b.filter",
            r"a\b.filter",
            r"..\x.filter",
            "con.filter",
            "NUL.filter",
            "com1.filter",
            "a?.filter",
            "x.filter.",
            long.as_str(),
        ] {
            assert!(valid_filter_name(bad).is_err(), "{bad:?}");
        }
        assert_eq!(valid_filter_name("a/b.filter"), Err(NameProblem::Separator));
        assert_eq!(valid_filter_name("con.filter"), Err(NameProblem::Reserved));
    }

    #[test]
    fn new_name_normalisation() {
        assert_eq!(new_filter_name("my-filter"), "my-filter.filter");
        assert_eq!(new_filter_name("  x.filter "), "x.filter");
        assert_eq!(new_filter_name("X.Filter"), "X.Filter");
    }

    #[test]
    fn listing_filters_only() {
        let t = tempfile::tempdir().unwrap();
        for f in ["b.filter", "A.FILTER", "a.filter.ffbak", "notes.txt"] {
            fs::write(t.path().join(f), "").unwrap();
        }
        fs::create_dir(t.path().join("d.filter")).unwrap();
        let list = list_filters(t.path()).unwrap();
        assert_eq!(list, vec!["A.FILTER", "b.filter"]);
        assert!(contains_name(&list, "a.filter"));
        assert!(!contains_name(&list, "c.filter"));
        assert!(contains_name(
            &["dynamic.filter".to_string()],
            "DYNAMIC.filter"
        ));
    }
}
