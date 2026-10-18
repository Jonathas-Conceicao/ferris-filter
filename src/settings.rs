//! Per-user overlay settings: the selected game-folder filter (STORE-02) and whether Apply
//! reloads the filter in game (RELOAD-01).
//!
//! `ff-settings.json` lives at the app root and is git-ignored. The library is pure: every path is
//! a parameter and nothing here prints. A filter name is only ever a bare `*.filter` file name, so
//! neither a hand-edited settings file nor a typed name can escape the game folder.

use std::collections::BTreeMap;
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
    /// Send /reloaditemfilter to the game after a RULES Apply that wrote the filter; absent means on.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reload_after_apply: Option<bool>,
    /// Collapsed state of the RULES list sections: filter name -> section name -> `true` when
    /// collapsed. Absent means the default (user sections expanded, the economy section collapsed).
    /// Display state only: it never marks a filter unapplied.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub collapsed_sections: BTreeMap<String, BTreeMap<String, bool>>,
    /// Unknown top-level keys survive a load/save round trip.
    #[serde(flatten)]
    pub extra: serde_json::Map<String, serde_json::Value>,
}

/// Reserved key of the economy section in [`Settings::collapsed_sections`]. The `ff:` prefix keeps
/// a user section that happens to be named "Economy" from sharing its state.
pub const ECONOMY_SECTION_KEY: &str = "ff:economy";

/// The key a section is stored under: the reserved economy key, or the user section name. A name
/// that starts with `ff:` or `user:` gets a `user:` prefix so it can never equal the reserved key
/// (or another section's escaped key).
pub fn section_key(name: &str, economy: bool) -> String {
    if economy {
        ECONOMY_SECTION_KEY.to_string()
    } else if name.starts_with("ff:") || name.starts_with("user:") {
        format!("user:{name}")
    } else {
        name.to_string()
    }
}

impl Settings {
    /// Whether the section `name` of `filter` is collapsed: the stored value, else `economy`
    /// (the economy section starts collapsed, user sections expanded). Pass
    /// [`ECONOMY_SECTION_KEY`] as `name` for the economy section.
    pub fn section_collapsed(&self, filter: &str, name: &str, economy: bool) -> bool {
        self.collapsed_sections
            .get(filter)
            .and_then(|m| m.get(name))
            .copied()
            .unwrap_or(economy)
    }

    /// A copy with the collapsed flag of one section set.
    pub fn set_section_collapsed(&self, filter: &str, name: &str, collapsed: bool) -> Settings {
        let mut next = self.clone();
        next.collapsed_sections
            .entry(filter.to_string())
            .or_default()
            .insert(name.to_string(), collapsed);
        next
    }

    /// A copy with the stored flag of section `key` dropped (it falls back to the default).
    pub fn forget_section_state(&self, filter: &str, key: &str) -> Settings {
        let mut next = self.clone();
        if let Some(m) = next.collapsed_sections.get_mut(filter) {
            m.remove(key);
        }
        next
    }

    /// A copy where section `to` carries the stored flag of section `from` (a rename); `from` is
    /// kept. Nothing stored for `from` gives an unchanged copy.
    pub fn copy_section_state(&self, filter: &str, from: &str, to: &str) -> Settings {
        let mut next = self.clone();
        if let Some(v) = self
            .collapsed_sections
            .get(filter)
            .and_then(|m| m.get(from))
        {
            next.collapsed_sections
                .entry(filter.to_string())
                .or_default()
                .insert(to.to_string(), *v);
        }
        next
    }

    /// The stored filter name, or `dynamic.filter`.
    pub fn filter_name(&self) -> &str {
        self.filter.as_deref().unwrap_or(DEFAULT_FILTER)
    }

    /// Whether Apply should reload the filter in game; on unless explicitly turned off.
    pub fn reload_after_apply(&self) -> bool {
        self.reload_after_apply.unwrap_or(true)
    }
}

/// `<root>/ff-settings.json`.
pub fn path(root: &Path) -> PathBuf {
    root.join(SETTINGS_FILE)
}

/// Load the settings. A missing file is the default (reload on) with no warning. A file that exists
/// but cannot be used at all (unreadable, or not a JSON object) gives `dynamic.filter` with reload
/// after Apply **off**, plus a warning that says both: the file may hold an opt-out of the
/// synthetic keystrokes that can no longer be read, so it fails closed (review iteration 2, WR-02).
/// A wrongly typed key or an invalid stored name falls back per key, with a warning. Never writes.
pub fn load(root: &Path) -> (Settings, Option<String>) {
    let warn = |reason: String| {
        Some(format!(
            "{SETTINGS_FILE} could not be used ({reason}); using {DEFAULT_FILTER}."
        ))
    };
    let unusable = |reason: String| {
        (
            Settings {
                reload_after_apply: Some(false),
                ..Settings::default()
            },
            Some(format!(
                "{SETTINGS_FILE} could not be used ({reason}); using {DEFAULT_FILTER}, \
                 and reload after Apply is off."
            )),
        )
    };
    let text = match fs::read_to_string(path(root)) {
        Ok(t) => t,
        Err(e) if e.kind() == io::ErrorKind::NotFound => return (Settings::default(), None),
        Err(e) => return unusable(e.to_string()),
    };
    let mut settings: Settings = match serde_json::from_str(&text) {
        Ok(s) => s,
        Err(e) => {
            // A wrongly typed value must not discard the other keys: salvage each separately, and
            // say which fallback each unusable key got.
            return match serde_json::from_str::<serde_json::Map<String, serde_json::Value>>(&text) {
                Ok(mut extra) => {
                    let mut fallbacks = Vec::new();
                    let filter = match extra.remove("filter") {
                        Some(serde_json::Value::String(n)) if valid_filter_name(&n).is_ok() => {
                            Some(n)
                        }
                        None => None,
                        Some(_) => {
                            fallbacks.push(format!("using {DEFAULT_FILTER}"));
                            None
                        }
                    };
                    // The reload sends synthetic keystrokes, so an opt-out that cannot be read
                    // fails closed: off, not the default on (review WR-04).
                    let reload_after_apply = match extra.remove("reload_after_apply") {
                        Some(serde_json::Value::Bool(b)) => Some(b),
                        None => None,
                        Some(_) => {
                            fallbacks.push(
                                "`reload_after_apply` is not true or false, so reload after Apply is off"
                                    .to_string(),
                            );
                            Some(false)
                        }
                    };
                    let collapsed_sections = match extra.remove("collapsed_sections") {
                        None => BTreeMap::new(),
                        Some(v) => match serde_json::from_value(v) {
                            Ok(m) => m,
                            Err(_) => {
                                fallbacks.push(
                                    "`collapsed_sections` is not a map of filter -> section -> true/false, \
                                     so every section uses its default"
                                        .to_string(),
                                );
                                BTreeMap::new()
                            }
                        },
                    };
                    if fallbacks.is_empty() {
                        fallbacks.push("using the defaults".to_string());
                    }
                    (
                        Settings {
                            filter,
                            reload_after_apply,
                            collapsed_sections,
                            extra,
                        },
                        Some(format!(
                            "{SETTINGS_FILE} could not be fully used ({e}); {}.",
                            fallbacks.join("; ")
                        )),
                    )
                }
                Err(_) => unusable(e.to_string()),
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
    std::fs::create_dir_all(root)?;
    let mut t = tempfile::NamedTempFile::new_in(root)?;
    t.write_all(text.as_bytes())?;
    t.as_file().sync_all()?;
    t.persist(path(root)).map_err(|e| e.error)?;
    Ok(())
}

/// Persist only the `collapsed_sections` key (a fold click), leaving every other key of the file
/// on disk exactly as it is, even one that is wrongly typed. A missing file is created with the
/// whole of `settings`. A file that cannot be used at all (unreadable, or not a JSON object) is
/// left alone and `Ok(false)` is returned, so a hand-edited file is never replaced by fallback
/// defaults. `Ok(true)` means the key was written.
pub fn save_section_state(root: &Path, settings: &Settings) -> io::Result<bool> {
    let text = match fs::read_to_string(path(root)) {
        Ok(t) => t,
        Err(e) if e.kind() == io::ErrorKind::NotFound => {
            save(root, settings)?;
            return Ok(true);
        }
        Err(_) => return Ok(false),
    };
    // The file exists, so its folder does: no `create_dir_all` needed on this path.
    let Ok(mut map) = serde_json::from_str::<serde_json::Map<String, serde_json::Value>>(&text)
    else {
        return Ok(false);
    };
    if settings.collapsed_sections.is_empty() {
        map.remove("collapsed_sections");
    } else {
        let value = serde_json::to_value(&settings.collapsed_sections).map_err(io::Error::other)?;
        map.insert("collapsed_sections".to_string(), value);
    }
    let mut out = serde_json::to_string_pretty(&map).map_err(io::Error::other)?;
    out.push('\n');
    let mut t = tempfile::NamedTempFile::new_in(root)?;
    t.write_all(out.as_bytes())?;
    t.as_file().sync_all()?;
    t.persist(path(root)).map_err(|e| e.error)?;
    Ok(true)
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
    fn save_creates_missing_folders() {
        let t = tempfile::tempdir().unwrap();
        let root = t.path().join("a").join("b");
        save(&root, &Settings::default()).unwrap();
        assert!(path(&root).is_file());
    }

    #[test]
    fn save_section_state_creates_missing_folder() {
        let t = tempfile::tempdir().unwrap();
        let root = t.path().join("fresh");
        assert!(save_section_state(&root, &Settings::default()).unwrap());
        assert!(path(&root).is_file());
    }

    #[test]
    fn load_from_missing_folder_creates_nothing() {
        let t = tempfile::tempdir().unwrap();
        let root = t.path().join("missing");
        let (s, w) = load(&root);
        assert_eq!(s.filter, None);
        assert_eq!(w, None);
        assert!(!root.exists());
    }

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
        let w = w.unwrap();
        assert!(w.contains("ff-settings.json"));
        // Review iteration 2, WR-02: an unusable file fails closed and says so.
        assert!(!s.reload_after_apply());
        assert!(w.contains("reload after Apply is off"), "{w}");
        assert_eq!(fs::read(path(t.path())).unwrap(), b"{not json");
    }

    #[test]
    fn trailing_comma_opt_out_stays_off() {
        // Review iteration 2, WR-02: the most common hand-edit mistake must not opt a user who
        // turned the reload off back into synthetic keystrokes.
        let t = tempfile::tempdir().unwrap();
        let body = r#"{ "filter": "ssf.filter", "reload_after_apply": false, }"#;
        fs::write(path(t.path()), body).unwrap();
        let (s, w) = load(t.path());
        let w = w.expect("an invalid file must warn");
        assert!(!s.reload_after_apply());
        assert_eq!(s.filter_name(), "dynamic.filter");
        assert!(w.contains("using dynamic.filter"), "{w}");
        assert!(w.contains("reload after Apply is off"), "{w}");
        assert_eq!(fs::read(path(t.path())).unwrap(), body.as_bytes());
    }

    #[test]
    fn json_that_is_not_an_object_fails_closed() {
        for body in ["[]", r#""x""#, "null", "5"] {
            let t = tempfile::tempdir().unwrap();
            fs::write(path(t.path()), body).unwrap();
            let (s, w) = load(t.path());
            assert!(!s.reload_after_apply(), "{body}");
            assert!(w.unwrap().contains("reload after Apply is off"), "{body}");
        }
    }

    #[test]
    fn unreadable_settings_file_fails_closed() {
        // A directory where the file should be: it exists, but reading it fails with an error
        // other than NotFound (the same branch a sharing violation takes).
        let t = tempfile::tempdir().unwrap();
        fs::create_dir(path(t.path())).unwrap();
        let (s, w) = load(t.path());
        let w = w.expect("an unreadable file must warn");
        assert!(!s.reload_after_apply());
        assert_eq!(s.filter_name(), "dynamic.filter");
        assert!(w.contains("reload after Apply is off"), "{w}");
    }

    #[test]
    fn missing_file_keeps_reload_on() {
        let t = tempfile::tempdir().unwrap();
        let (s, w) = load(t.path());
        assert_eq!(w, None);
        assert!(s.reload_after_apply());
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

    #[test]
    fn reload_after_apply_defaults_on_and_is_not_serialized() {
        let s = Settings::default();
        assert!(s.reload_after_apply());
        assert!(
            !serde_json::to_string(&s)
                .unwrap()
                .contains("reload_after_apply")
        );
    }

    #[test]
    fn reload_after_apply_false_round_trips() {
        let t = tempfile::tempdir().unwrap();
        fs::write(
            path(t.path()),
            r#"{"filter":"a.filter","reload_after_apply":false,"future":{"x":1}}"#,
        )
        .unwrap();
        let (s, w) = load(t.path());
        assert_eq!(w, None);
        assert!(!s.reload_after_apply());
        assert_eq!(s.filter_name(), "a.filter");
        assert_eq!(s.extra["future"]["x"], 1);
        save(t.path(), &s).unwrap();
        let (s2, w2) = load(t.path());
        assert_eq!(w2, None);
        assert_eq!(s2, s);
    }

    #[test]
    fn wrongly_typed_reload_flag_keeps_the_filter_and_other_keys_and_fails_closed() {
        for bad in [r#""yes""#, r#""false""#, "0", "1", "[]"] {
            let t = tempfile::tempdir().unwrap();
            let body =
                format!(r#"{{"filter":"ssf.filter","reload_after_apply":{bad},"future":1}}"#);
            fs::write(path(t.path()), &body).unwrap();
            let (s, w) = load(t.path());
            let w = w.unwrap_or_else(|| panic!("{bad}: no warning"));
            // Review WR-04: an unreadable opt-out is off, and the warning names the key and
            // does not claim the filter fell back.
            assert!(!s.reload_after_apply(), "{bad}");
            assert!(w.contains("reload_after_apply"), "{bad}: {w}");
            assert!(w.contains("off"), "{bad}: {w}");
            assert!(!w.contains("using dynamic.filter"), "{bad}: {w}");
            assert_eq!(s.filter_name(), "ssf.filter", "{bad}");
            assert!(s.extra.contains_key("future"));
            assert!(!s.extra.contains_key("filter"));
            assert!(!s.extra.contains_key("reload_after_apply"));
            assert_eq!(fs::read(path(t.path())).unwrap(), body.as_bytes());
        }
    }

    #[test]
    fn collapsed_defaults_and_per_filter_state() {
        let s = Settings::default();
        assert!(!s.section_collapsed("dynamic.filter", "Leveling", false));
        assert!(s.section_collapsed("dynamic.filter", ECONOMY_SECTION_KEY, true));
        let s = s.set_section_collapsed("a.filter", ECONOMY_SECTION_KEY, false);
        assert!(!s.section_collapsed("a.filter", ECONOMY_SECTION_KEY, true));
        assert!(s.section_collapsed("b.filter", ECONOMY_SECTION_KEY, true));
    }

    #[test]
    fn section_key_never_equals_the_economy_key() {
        assert_eq!(section_key("Gear", false), "Gear");
        assert_eq!(section_key("x", true), ECONOMY_SECTION_KEY);
        assert_ne!(section_key(ECONOMY_SECTION_KEY, false), ECONOMY_SECTION_KEY);
        assert_ne!(
            section_key("user:ff:economy", false),
            section_key("ff:economy", false)
        );
        let s = Settings::default().set_section_collapsed(
            "a.filter",
            &section_key(ECONOMY_SECTION_KEY, false),
            false,
        );
        assert!(s.section_collapsed("a.filter", ECONOMY_SECTION_KEY, true));
    }

    #[test]
    fn forget_section_state_drops_the_flag() {
        let s = Settings::default().set_section_collapsed("a.filter", "Old", true);
        let n = s.forget_section_state("a.filter", "Old");
        assert!(!n.section_collapsed("a.filter", "Old", false));
        assert_eq!(n.forget_section_state("b.filter", "Old"), n);
    }

    #[test]
    fn copy_section_state_carries_a_rename() {
        let s = Settings::default().set_section_collapsed("a.filter", "Old", true);
        let n = s.copy_section_state("a.filter", "Old", "New");
        assert!(n.section_collapsed("a.filter", "New", false));
        assert!(n.section_collapsed("a.filter", "Old", false));
        let empty = Settings::default();
        assert_eq!(empty.copy_section_state("a.filter", "Old", "New"), empty);
    }

    #[test]
    fn collapsed_sections_round_trip_with_other_keys() {
        let t = tempfile::tempdir().unwrap();
        fs::write(
            path(t.path()),
            r#"{"filter":"a.filter","reload_after_apply":false,"zzz":[1,2]}"#,
        )
        .unwrap();
        let (s, _) = load(t.path());
        let s = s.set_section_collapsed("a.filter", "Leveling", true);
        save(t.path(), &s).unwrap();
        let (s2, w) = load(t.path());
        assert_eq!(w, None);
        assert_eq!(s2, s);
        assert!(s2.section_collapsed("a.filter", "Leveling", false));
        assert_eq!(s2.filter_name(), "a.filter");
        assert!(!s2.reload_after_apply());
        assert_eq!(s2.extra["zzz"][1], 2);
    }

    #[test]
    fn wrongly_typed_collapsed_sections_keeps_other_keys() {
        let t = tempfile::tempdir().unwrap();
        fs::write(
            path(t.path()),
            r#"{"filter":"ssf.filter","collapsed_sections":"oops","future":1}"#,
        )
        .unwrap();
        let (s, w) = load(t.path());
        let w = w.expect("must warn");
        assert!(w.contains("collapsed_sections"), "{w}");
        assert_eq!(s.filter_name(), "ssf.filter");
        assert!(s.collapsed_sections.is_empty());
        assert!(s.extra.contains_key("future"));
        assert!(!s.extra.contains_key("collapsed_sections"));
    }

    #[test]
    fn empty_collapsed_sections_is_not_serialized() {
        assert!(
            !serde_json::to_string(&Settings::default())
                .unwrap()
                .contains("collapsed_sections")
        );
    }

    #[test]
    fn wrongly_typed_filter_warning_names_the_filter_fallback_only() {
        let t = tempfile::tempdir().unwrap();
        fs::write(path(t.path()), r#"{"filter":5,"reload_after_apply":false}"#).unwrap();
        let (s, w) = load(t.path());
        let w = w.unwrap();
        assert_eq!(s.filter_name(), "dynamic.filter");
        assert!(!s.reload_after_apply());
        assert!(w.contains("using dynamic.filter"), "{w}");
        assert!(!w.contains("reload_after_apply"), "{w}");
    }

    #[test]
    fn a_fold_save_writes_only_its_own_key() {
        let t = tempfile::tempdir().unwrap();
        // One wrongly typed key: the file is still a JSON object, so a fold click is saved and the
        // bad key is kept byte-for-byte (not replaced by the in-memory fallback).
        fs::write(
            path(t.path()),
            r#"{"filter":"ssf.filter","reload_after_apply":"yes","future":1}"#,
        )
        .unwrap();
        let (s, w) = load(t.path());
        assert!(w.is_some());
        let next = s.set_section_collapsed("ssf.filter", "Gear", true);
        assert!(save_section_state(t.path(), &next).unwrap());
        let v: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(path(t.path())).unwrap()).unwrap();
        assert_eq!(v["reload_after_apply"], "yes");
        assert_eq!(v["future"], 1);
        assert_eq!(v["collapsed_sections"]["ssf.filter"]["Gear"], true);
        let (again, _) = load(t.path());
        assert!(again.section_collapsed("ssf.filter", "Gear", false));
    }

    #[test]
    fn a_fold_save_never_overwrites_an_unusable_file() {
        let t = tempfile::tempdir().unwrap();
        let bad = "{ \"filter\": \"x.filter\", }";
        fs::write(path(t.path()), bad).unwrap();
        let next = Settings::default().set_section_collapsed("dynamic.filter", "Gear", true);
        assert!(!save_section_state(t.path(), &next).unwrap());
        assert_eq!(fs::read_to_string(path(t.path())).unwrap(), bad);
        // A missing file is created.
        let fresh = tempfile::tempdir().unwrap();
        assert!(save_section_state(fresh.path(), &next).unwrap());
        assert!(
            load(fresh.path())
                .0
                .section_collapsed("dynamic.filter", "Gear", false)
        );
    }
}
