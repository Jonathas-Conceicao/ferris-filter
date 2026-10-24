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

use crate::overlay::chord::Chord;

/// File name of the settings file at the app root.
pub const SETTINGS_FILE: &str = "ff-settings.json";
/// The filter used when nothing (valid) is stored.
pub const DEFAULT_FILTER: &str = crate::dynamic::FILTER_FILE;

const MAX_NAME_LEN: usize = 200;

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Settings {
    /// The filter the user selected (a filter choice or a game-folder pick); saved.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub filter: Option<String>,
    /// The filter the overlay opened on its own because none is selected (the folder's
    /// `dynamic.filter`, else its first `.filter`). [`Settings::filter_name`] reads it after
    /// `filter`, but it is never serialized: saving another preference does not turn it into a
    /// selection, so `ff-cli apply` only ever targets a filter the user picked.
    #[serde(skip)]
    pub auto_filter: Option<String>,
    /// The game folder the user picked on the Config tab; absent means the default
    /// (`FERRISFILTER_GAME_DIR`, else Documents\My Games\Path of Exile 2).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub game_folder: Option<String>,
    /// Send /reloaditemfilter to the game after a RULES Apply that wrote the filter; absent means on.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reload_after_apply: Option<bool>,
    /// The league Sync economy prices against, written only when the user picks one; absent means
    /// the computed default.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub league: Option<String>,
    /// `"live"` or `"fallback"`; absent or any other string means Live. Read by the overlay only,
    /// the CLI ignores it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub price_source: Option<String>,
    /// Collapsed state of the RULES list sections: filter name -> section name -> `true` when
    /// collapsed. Absent means the default (user sections expanded, the economy section collapsed).
    /// Display state only: it never marks a filter unapplied.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub collapsed_sections: BTreeMap<String, BTreeMap<String, bool>>,
    /// The show / hide hotkey in FerrisFilter's own chord text (`Ctrl+Alt+K`), written only by
    /// Apply; absent means `Ctrl+Shift+Z`. Always the saved value, never a pending or fallback
    /// chord. Read by the overlay only, `ff-cli` ignores it. An unusable value is dropped at load
    /// with a warning (see [`Settings::hotkey_invalid`]).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hotkey: Option<String>,
    /// A `hotkey` value was present in the file but dropped as unusable at load (absent or valid:
    /// `false`). Never serialized.
    #[serde(skip)]
    pub hotkey_invalid: bool,
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

    /// The selected filter name, else the automatic pick, else `dynamic.filter`.
    pub fn filter_name(&self) -> &str {
        self.filter
            .as_deref()
            .or(self.auto_filter.as_deref())
            .unwrap_or(DEFAULT_FILTER)
    }

    /// The saved game folder, only when it is a non-empty absolute path.
    pub fn game_folder_path(&self) -> Option<&Path> {
        self.game_folder
            .as_deref()
            .filter(|s| usable_folder(s))
            .map(Path::new)
    }

    /// Whether Apply should reload the filter in game; on unless explicitly turned off.
    pub fn reload_after_apply(&self) -> bool {
        self.reload_after_apply.unwrap_or(true)
    }

    /// The saved league name: the trimmed value when non-empty.
    pub fn saved_league(&self) -> Option<&str> {
        self.league
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
    }

    /// The price source policy: Fallback only when saved as `fallback` (any case), else Auto.
    pub fn price_policy(&self) -> crate::compile::PriceSourcePolicy {
        match self.price_source.as_deref() {
            Some(s) if s.eq_ignore_ascii_case("fallback") => {
                crate::compile::PriceSourcePolicy::ForceFallback
            }
            _ => crate::compile::PriceSourcePolicy::Auto,
        }
    }

    /// The saved show / hide chord, when the stored text is a usable chord.
    pub fn saved_hotkey(&self) -> Option<Chord> {
        self.hotkey.as_deref().and_then(usable_hotkey)
    }
}

/// The warning for a `hotkey` value that cannot be used.
const BAD_HOTKEY: &str = "`hotkey` is not a usable key combination, so Ctrl+Shift+Z is used";

/// A load warning re-worded for a RELOAD, which never re-registers the hotkey (review WR-06):
/// the `hotkey` sentence names the chord that really stays live (`live`, `None` when none is
/// registered) instead of claiming Ctrl+Shift+Z is used. Other warnings pass through unchanged.
pub fn reload_hotkey_warning(warning: String, live: Option<Chord>) -> String {
    if !warning.contains(BAD_HOTKEY) {
        return warning;
    }
    let replacement = match live {
        Some(chord) => format!(
            "`hotkey` is not a usable key combination; {} stays in use until the next launch",
            chord.text()
        ),
        None => "`hotkey` is not a usable key combination".to_string(),
    };
    warning.replace(BAD_HOTKEY, &replacement)
}

/// The chord a stored `hotkey` text names, when it parses and may be registered.
fn usable_hotkey(text: &str) -> Option<Chord> {
    Chord::parse(text).ok().filter(Chord::is_bindable)
}

fn usable_folder(s: &str) -> bool {
    !s.is_empty() && Path::new(s).is_absolute()
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
    let loaded = load_detailed(root);
    (loaded.settings, loaded.warning)
}

/// What [`load_detailed`] read.
#[derive(Debug, Clone, PartialEq)]
pub struct Loaded {
    pub settings: Settings,
    /// The warning [`load`] returns.
    pub warning: Option<String>,
    /// The `filter` key was present but unusable, or the whole file was: the filter name fell
    /// back to `dynamic.filter`, as the warning says. A warning about other keys only leaves it
    /// `false`.
    pub filter_fell_back: bool,
}

/// [`load`], also saying whether the filter name itself fell back.
pub fn load_detailed(root: &Path) -> Loaded {
    let (settings, warning, filter_fell_back) = load_parts(root);
    Loaded {
        settings,
        warning,
        filter_fell_back,
    }
}

fn load_parts(root: &Path) -> (Settings, Option<String>, bool) {
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
            true,
        )
    };
    let text = match fs::read_to_string(path(root)) {
        Ok(t) => t,
        Err(e) if e.kind() == io::ErrorKind::NotFound => {
            return (Settings::default(), None, false);
        }
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
                    let mut filter_fell_back = false;
                    let filter = match extra.remove("filter") {
                        Some(serde_json::Value::String(n)) if valid_filter_name(&n).is_ok() => {
                            Some(n)
                        }
                        None => None,
                        Some(_) => {
                            fallbacks.push(format!("using {DEFAULT_FILTER}"));
                            filter_fell_back = true;
                            None
                        }
                    };
                    let game_folder = match extra.remove("game_folder") {
                        Some(serde_json::Value::String(s)) if usable_folder(&s) => Some(s),
                        None => None,
                        Some(_) => {
                            fallbacks.push(
                                "`game_folder` is not an absolute folder path, so the default game folder is used"
                                    .to_string(),
                            );
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
                    let league = match extra.remove("league") {
                        Some(serde_json::Value::String(s)) => Some(s),
                        None => None,
                        Some(_) => {
                            fallbacks.push(
                                "`league` is not text, so the default league is used".to_string(),
                            );
                            None
                        }
                    };
                    let price_source = match extra.remove("price_source") {
                        Some(serde_json::Value::String(s)) => Some(s),
                        None => None,
                        Some(_) => {
                            fallbacks.push(
                                "`price_source` is not text, so Live prices are used".to_string(),
                            );
                            None
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
                    // A value that is not a usable chord (wrong type included) must not ride
                    // along in `extra`: drop it with a warning.
                    let (hotkey, hotkey_invalid) = match extra.remove("hotkey") {
                        Some(serde_json::Value::String(s)) if usable_hotkey(&s).is_some() => {
                            (Some(s), false)
                        }
                        None => (None, false),
                        Some(_) => {
                            fallbacks.push(BAD_HOTKEY.to_string());
                            (None, true)
                        }
                    };
                    if fallbacks.is_empty() {
                        fallbacks.push("using the defaults".to_string());
                    }
                    (
                        Settings {
                            filter,
                            auto_filter: None,
                            game_folder,
                            reload_after_apply,
                            league,
                            price_source,
                            collapsed_sections,
                            hotkey,
                            hotkey_invalid,
                            extra,
                        },
                        Some(format!(
                            "{SETTINGS_FILE} could not be fully used ({e}); {}.",
                            fallbacks.join("; ")
                        )),
                        filter_fell_back,
                    )
                }
                Err(_) => unusable(e.to_string()),
            };
        }
    };
    let mut bad_filter = None;
    if let Some(name) = settings.filter.clone()
        && let Err(problem) = valid_filter_name(&name)
    {
        settings.filter = None;
        bad_filter = Some(format!("`{name}`: {problem}"));
    }
    let mut bad_folder = None;
    if let Some(folder) = settings.game_folder.clone()
        && !usable_folder(&folder)
    {
        settings.game_folder = None;
        bad_folder = Some(format!(
            "`game_folder` `{folder}` is not an absolute folder path"
        ));
    }
    if let Some(text) = settings.hotkey.as_deref()
        && usable_hotkey(text).is_none()
    {
        settings.hotkey = None;
        settings.hotkey_invalid = true;
    }
    let (settings, warning, fell_back) = typed_warning(settings, bad_filter, bad_folder, warn);
    if !settings.hotkey_invalid {
        return (settings, warning, fell_back);
    }
    // The hotkey warning composes after the others, in the same `could not be fully used` form.
    let warning = match warning {
        None => format!("{SETTINGS_FILE} could not be fully used; {BAD_HOTKEY}."),
        Some(w) => format!("{}; {BAD_HOTKEY}.", w.trim_end_matches('.')),
    };
    (settings, Some(warning), fell_back)
}

/// The warning of a typed load for the `filter` and `game_folder` problems found.
fn typed_warning(
    settings: Settings,
    bad_filter: Option<String>,
    bad_folder: Option<String>,
    warn: impl Fn(String) -> Option<String>,
) -> (Settings, Option<String>, bool) {
    match (bad_filter, bad_folder) {
        (None, None) => (settings, None, false),
        (Some(f), None) => (settings, warn(f), true),
        (None, Some(g)) => (
            settings,
            Some(format!(
                "{SETTINGS_FILE} could not be fully used ({g}); using the default game folder."
            )),
            false,
        ),
        (Some(f), Some(g)) => (
            settings,
            Some(format!(
                "{SETTINGS_FILE} could not be fully used ({f}; {g}); using {DEFAULT_FILTER} and the default game folder."
            )),
            true,
        ),
    }
}

/// The filter to select after the game folder changed: the listed entry with the same name (listed
/// spelling), else the first listed `.filter`, else `current` (a new file the first Apply creates).
/// `dynamic.filter` is not special: it is just the default current name.
pub fn choose_filter(list: &[String], current: &str) -> String {
    list.iter()
        .find(|n| n.eq_ignore_ascii_case(current))
        .or_else(|| list.first())
        .cloned()
        .unwrap_or_else(|| current.to_string())
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
    save_keys(root, settings, &["collapsed_sections"])
}

/// Persist only the `league` key (a league pick); same rules as [`save_section_state`].
pub fn save_league(root: &Path, settings: &Settings) -> io::Result<bool> {
    save_keys(root, settings, &["league"])
}

/// Persist only the `price_source` key (a Live / Fallback switch); same rules as
/// [`save_section_state`].
pub fn save_price_source(root: &Path, settings: &Settings) -> io::Result<bool> {
    save_keys(root, settings, &["price_source"])
}

/// Write `keys` of `settings` into the file on disk, every other key left as it is. A key that
/// `settings` does not serialize (unset, empty) is removed. A missing file is created whole; an
/// unusable one is left alone (`Ok(false)`).
fn save_keys(root: &Path, settings: &Settings, keys: &[&str]) -> io::Result<bool> {
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
    let serde_json::Value::Object(mut wanted) =
        serde_json::to_value(settings).map_err(io::Error::other)?
    else {
        return Err(io::Error::other("settings did not serialize to an object"));
    };
    for key in keys {
        match wanted.remove(*key) {
            Some(value) => map.insert((*key).to_string(), value),
            None => map.remove(*key),
        };
    }
    let out = render_object(&map).map_err(io::Error::other)?;
    let mut t = tempfile::NamedTempFile::new_in(root)?;
    t.write_all(out.as_bytes())?;
    t.as_file().sync_all()?;
    t.persist(path(root)).map_err(|e| e.error)?;
    Ok(true)
}

/// A settings object as the file holds it: pretty JSON plus a trailing newline.
fn render_object(
    map: &serde_json::Map<String, serde_json::Value>,
) -> Result<String, serde_json::Error> {
    let mut out = serde_json::to_string_pretty(map)?;
    out.push('\n');
    Ok(out)
}

/// The bytes `ff-settings.json` should hold with `hotkey` set to `chord_text`, every other key of
/// `existing` kept as it is (unknown and wrongly typed keys included). `None` (no file) gives a
/// file with only `hotkey`. Bytes that are not a JSON object give `Err(())`, so a damaged
/// hand-edited file is never replaced. Writes nothing; Apply writes the result.
// The caller only needs "usable or not"; it shows its own fixed refusal sentence.
#[allow(clippy::result_unit_err)]
pub fn render_hotkey(existing: Option<&[u8]>, chord_text: &str) -> Result<Vec<u8>, ()> {
    let mut map = match existing {
        None => serde_json::Map::new(),
        Some(bytes) => serde_json::from_slice::<serde_json::Map<String, serde_json::Value>>(bytes)
            .map_err(|_| ())?,
    };
    map.insert(
        "hotkey".to_string(),
        serde_json::Value::String(chord_text.to_string()),
    );
    render_object(&map).map(String::into_bytes).map_err(|_| ())
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

    fn abs_folder() -> &'static str {
        if cfg!(windows) {
            r"C:\Games\PoE2"
        } else {
            "/games/poe2"
        }
    }

    #[test]
    fn game_folder_absent_round_trips_byte_identically() {
        let t = tempfile::tempdir().unwrap();
        let body = "{\n  \"filter\": \"a.filter\",\n  \"future\": 1\n}\n";
        fs::write(path(t.path()), body).unwrap();
        let (s, w) = load(t.path());
        assert_eq!(w, None);
        assert_eq!(s.game_folder, None);
        save(t.path(), &s).unwrap();
        assert_eq!(fs::read_to_string(path(t.path())).unwrap(), body);
    }

    #[test]
    fn game_folder_valid_loads() {
        let t = tempfile::tempdir().unwrap();
        let body = format!(
            r#"{{"game_folder":"{}"}}"#,
            abs_folder().replace('\\', "\\\\")
        );
        fs::write(path(t.path()), body).unwrap();
        let (s, w) = load(t.path());
        assert_eq!(w, None);
        assert_eq!(s.game_folder_path(), Some(Path::new(abs_folder())));
    }

    #[test]
    fn game_folder_empty_or_relative_is_dropped_with_warning() {
        for bad in ["", r"relative\\dir"] {
            let t = tempfile::tempdir().unwrap();
            let body = format!(
                r#"{{"game_folder":"{bad}","filter":"a.filter","reload_after_apply":false,"future":1}}"#
            );
            fs::write(path(t.path()), &body).unwrap();
            let (s, w) = load(t.path());
            let w = w.unwrap_or_else(|| panic!("{bad:?}: no warning"));
            assert!(w.contains("game_folder"), "{w}");
            assert!(w.contains("default game folder"), "{w}");
            assert_eq!(s.game_folder, None);
            assert_eq!(s.game_folder_path(), None);
            assert_eq!(s.filter_name(), "a.filter");
            assert!(!s.reload_after_apply());
            assert!(s.extra.contains_key("future"));
            assert_eq!(fs::read(path(t.path())).unwrap(), body.as_bytes());
        }
    }

    #[test]
    fn game_folder_and_filter_both_bad_names_both() {
        let t = tempfile::tempdir().unwrap();
        fs::write(
            path(t.path()),
            r#"{"game_folder":"rel","filter":"C:/x.filter"}"#,
        )
        .unwrap();
        let (s, w) = load(t.path());
        let w = w.unwrap();
        assert!(w.contains("game_folder") && w.contains("filter"), "{w}");
        assert_eq!(s.game_folder, None);
        assert_eq!(s.filter, None);
    }

    #[test]
    fn game_folder_wrong_type_is_salvaged() {
        let t = tempfile::tempdir().unwrap();
        fs::write(
            path(t.path()),
            r#"{"game_folder":42,"filter":"ssf.filter","reload_after_apply":false,"future":1}"#,
        )
        .unwrap();
        let (s, w) = load(t.path());
        let w = w.unwrap();
        assert!(w.contains("default game folder"), "{w}");
        assert_eq!(s.game_folder, None);
        assert_eq!(s.filter_name(), "ssf.filter");
        assert!(!s.reload_after_apply());
        assert!(!s.extra.contains_key("game_folder"));
    }

    #[test]
    fn game_folder_unusable_file_is_none() {
        let t = tempfile::tempdir().unwrap();
        fs::write(path(t.path()), "{not json").unwrap();
        let (s, _) = load(t.path());
        assert_eq!(s.game_folder, None);
    }

    #[test]
    fn game_folder_saved_keeps_unknown_keys() {
        let t = tempfile::tempdir().unwrap();
        fs::write(path(t.path()), r#"{"future":{"a":1}}"#).unwrap();
        let (mut s, _) = load(t.path());
        s.game_folder = Some(abs_folder().to_string());
        save(t.path(), &s).unwrap();
        let (s2, w) = load(t.path());
        assert_eq!(w, None);
        assert_eq!(s2.game_folder_path(), Some(Path::new(abs_folder())));
        assert_eq!(s2.extra["future"]["a"], 1);
    }

    #[test]
    fn choose_filter_rule() {
        let l = |v: &[&str]| v.iter().map(|s| s.to_string()).collect::<Vec<_>>();
        assert_eq!(
            choose_filter(&l(&["a.filter", "Dynamic.filter"]), "dynamic.filter"),
            "Dynamic.filter"
        );
        assert_eq!(
            choose_filter(&l(&["a.filter", "b.filter"]), "dynamic.filter"),
            "a.filter"
        );
        assert_eq!(choose_filter(&[], "mine.filter"), "mine.filter");
    }

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
    fn the_automatic_filter_is_read_but_never_saved() {
        let t = tempfile::tempdir().unwrap();
        let s = Settings {
            auto_filter: Some("x.filter".to_string()),
            ..Settings::default()
        };
        assert_eq!(s.filter_name(), "x.filter");
        save(t.path(), &s).unwrap();
        assert!(
            !fs::read_to_string(path(t.path()))
                .unwrap()
                .contains("x.filter")
        );
        assert!(save_section_state(t.path(), &s).unwrap());
        let (back, w) = load(t.path());
        assert_eq!(w, None);
        assert_eq!(back.filter_name(), "dynamic.filter");
        let chosen = Settings {
            filter: Some("y.filter".to_string()),
            ..s
        };
        assert_eq!(chosen.filter_name(), "y.filter");
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
    fn league_and_price_source_default_unset_and_unserialized() {
        use crate::compile::PriceSourcePolicy;
        let s = Settings::default();
        let json = serde_json::to_string(&s).unwrap();
        assert!(!json.contains("league") && !json.contains("price_source"));
        assert_eq!(s.price_policy(), PriceSourcePolicy::Auto);
        assert_eq!(s.saved_league(), None);
    }

    #[test]
    fn price_source_values_map_to_a_policy_without_warning() {
        use crate::compile::PriceSourcePolicy;
        for (v, want) in [
            ("fallback", PriceSourcePolicy::ForceFallback),
            ("FALLBACK", PriceSourcePolicy::ForceFallback),
            ("live", PriceSourcePolicy::Auto),
            ("bogus", PriceSourcePolicy::Auto),
        ] {
            let t = tempfile::tempdir().unwrap();
            fs::write(path(t.path()), format!(r#"{{"price_source":"{v}"}}"#)).unwrap();
            let (s, w) = load(t.path());
            assert_eq!(w, None, "{v}");
            assert_eq!(s.price_policy(), want, "{v}");
        }
    }

    #[test]
    fn league_round_trips_and_blank_reads_unset() {
        let t = tempfile::tempdir().unwrap();
        fs::write(path(t.path()), r#"{"league":"Runes of Aldur"}"#).unwrap();
        let (s, w) = load(t.path());
        assert_eq!(w, None);
        assert_eq!(s.saved_league(), Some("Runes of Aldur"));
        save(t.path(), &s).unwrap();
        let (s2, _) = load(t.path());
        assert_eq!(s2, s);
        fs::write(path(t.path()), r#"{"league":"   "}"#).unwrap();
        assert_eq!(load(t.path()).0.saved_league(), None);
    }

    #[test]
    fn wrongly_typed_league_and_price_source_are_salvaged() {
        let t = tempfile::tempdir().unwrap();
        fs::write(
            path(t.path()),
            r#"{"filter":"ssf.filter","league":5,"price_source":true,"future":1}"#,
        )
        .unwrap();
        let (s, w) = load(t.path());
        let w = w.expect("must warn");
        assert!(w.contains("league") && w.contains("price_source"), "{w}");
        assert_eq!((s.league.clone(), s.price_source.clone()), (None, None));
        assert_eq!(s.filter_name(), "ssf.filter");
        assert!(s.extra.contains_key("future"));
        assert!(!s.extra.contains_key("league") && !s.extra.contains_key("price_source"));
    }

    #[test]
    fn saving_both_keys_keeps_unknown_keys() {
        let t = tempfile::tempdir().unwrap();
        let mut s = Settings {
            league: Some("Standard".into()),
            price_source: Some("fallback".into()),
            ..Settings::default()
        };
        s.extra.insert("future".into(), serde_json::json!(1));
        save(t.path(), &s).unwrap();
        let v: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(path(t.path())).unwrap()).unwrap();
        assert_eq!(v["league"], "Standard");
        assert_eq!(v["price_source"], "fallback");
        assert_eq!(v["future"], 1);
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
    fn filter_fell_back_only_when_the_filter_name_did() {
        let cases: &[(Option<&str>, bool)] = &[
            (None, false),
            (Some(r#"{"filter":"a.filter"}"#), false),
            (Some(r#"{"game_folder":"relative"}"#), false),
            (Some(r#"{"reload_after_apply":"yes"}"#), false),
            (Some(r#"{"collapsed_sections":[]}"#), false),
            (
                Some(r#"{"filter":"a.filter","reload_after_apply":"yes"}"#),
                false,
            ),
            (Some(r#"{"filter":"../evil.filter"}"#), true),
            (
                Some(r#"{"filter":"../evil.filter","game_folder":"relative"}"#),
                true,
            ),
            (Some(r#"{"filter":5}"#), true),
            (Some(r#"{"filter":5,"reload_after_apply":"yes"}"#), true),
            (Some("{ not json"), true),
            (Some("[]"), true),
        ];
        for (text, fell_back) in cases {
            let t = tempfile::tempdir().unwrap();
            if let Some(text) = text {
                fs::write(path(t.path()), text).unwrap();
            }
            let loaded = load_detailed(t.path());
            assert_eq!(loaded.filter_fell_back, *fell_back, "{text:?}");
            assert_eq!(
                (loaded.settings, loaded.warning),
                load(t.path()),
                "{text:?}"
            );
        }
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
    fn league_and_price_source_saves_write_only_their_own_key() {
        let t = tempfile::tempdir().unwrap();
        fs::write(
            path(t.path()),
            r#"{"filter":"ssf.filter","price_source":7,"future":1}"#,
        )
        .unwrap();
        let (s, _) = load(t.path());
        let next = Settings {
            league: Some("Standard".into()),
            ..s
        };
        assert!(save_league(t.path(), &next).unwrap());
        let v: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(path(t.path())).unwrap()).unwrap();
        assert_eq!(v["league"], "Standard");
        assert_eq!(v["price_source"], 7);
        assert_eq!(v["filter"], "ssf.filter");
        assert_eq!(v["future"], 1);

        let next = Settings {
            price_source: Some("fallback".into()),
            ..next
        };
        assert!(save_price_source(t.path(), &next).unwrap());
        let (again, _) = load(t.path());
        assert_eq!(again.saved_league(), Some("Standard"));
        assert_eq!(
            again.price_policy(),
            crate::compile::PriceSourcePolicy::ForceFallback
        );

        let bad = "[1, 2]";
        fs::write(path(t.path()), bad).unwrap();
        assert!(!save_league(t.path(), &next).unwrap());
        assert!(!save_price_source(t.path(), &next).unwrap());
        assert_eq!(fs::read_to_string(path(t.path())).unwrap(), bad);
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

    fn ctrl_alt_k() -> Chord {
        Chord::parse("Ctrl+Alt+K").unwrap()
    }

    #[test]
    fn hotkey_absent_is_none_and_round_trips() {
        let t = tempfile::tempdir().unwrap();
        let body = "{\n  \"filter\": \"a.filter\",\n  \"future\": 1\n}\n";
        fs::write(path(t.path()), body).unwrap();
        let (s, w) = load(t.path());
        assert_eq!(w, None);
        assert_eq!(s.hotkey, None);
        assert!(!s.hotkey_invalid);
        assert_eq!(s.saved_hotkey(), None);
        save(t.path(), &s).unwrap();
        assert_eq!(fs::read_to_string(path(t.path())).unwrap(), body);
    }

    #[test]
    fn hotkey_valid_loads() {
        let t = tempfile::tempdir().unwrap();
        fs::write(path(t.path()), r#"{"hotkey":"Ctrl+Alt+K"}"#).unwrap();
        let (s, w) = load(t.path());
        assert_eq!(w, None);
        assert!(!s.hotkey_invalid);
        assert_eq!(s.saved_hotkey(), Some(ctrl_alt_k()));
        assert_eq!(s.hotkey.as_deref(), Some("Ctrl+Alt+K"));
    }

    #[test]
    fn hotkey_unusable_text_is_dropped_with_warning() {
        for bad in [
            "Z",
            "Win+K",
            "nonsense",
            "Ctrl+Escape",
            "",
            "Ctrl+V",
            "Alt+F4",
        ] {
            let t = tempfile::tempdir().unwrap();
            let body = format!(
                r#"{{"hotkey":"{bad}","filter":"a.filter","reload_after_apply":false,"future":1}}"#
            );
            fs::write(path(t.path()), &body).unwrap();
            let (s, w) = load(t.path());
            let w = w.unwrap_or_else(|| panic!("{bad:?}: no warning"));
            assert!(
                w.contains("`hotkey` is not a usable key combination, so Ctrl+Shift+Z is used"),
                "{w}"
            );
            assert_eq!(s.hotkey, None);
            assert!(s.hotkey_invalid, "{bad:?}");
            assert_eq!(s.saved_hotkey(), None);
            assert_eq!(s.filter_name(), "a.filter");
            assert!(!s.reload_after_apply());
            assert!(s.extra.contains_key("future"));
            assert!(!s.extra.contains_key("hotkey"));
            // Load never writes.
            assert_eq!(fs::read(path(t.path())).unwrap(), body.as_bytes());
        }
    }

    #[test]
    fn a_reload_warning_names_the_chord_still_live() {
        let t = tempfile::tempdir().unwrap();
        fs::write(path(t.path()), r#"{"hotkey":"Z"}"#).unwrap();
        let (_, w) = load(t.path());
        let w = w.unwrap();
        let reworded = reload_hotkey_warning(w.clone(), Some(ctrl_alt_k()));
        assert!(
            reworded.contains(
                "`hotkey` is not a usable key combination; Ctrl+Alt+K stays in use until the next launch"
            ),
            "{reworded}"
        );
        assert!(!reworded.contains("Ctrl+Shift+Z is used"), "{reworded}");
        let none = reload_hotkey_warning(w, None);
        assert!(
            none.contains("`hotkey` is not a usable key combination."),
            "{none}"
        );
        assert!(!none.contains("Ctrl+Shift+Z"), "{none}");
        // Another warning passes through untouched.
        let other = "ff-settings.json could not be fully used; x.".to_string();
        assert_eq!(reload_hotkey_warning(other.clone(), None), other);
    }

    #[test]
    fn hotkey_wrong_type_is_salvaged() {
        let t = tempfile::tempdir().unwrap();
        fs::write(
            path(t.path()),
            r#"{"hotkey":42,"filter":"a.filter","future":1}"#,
        )
        .unwrap();
        let (s, w) = load(t.path());
        let w = w.unwrap();
        assert!(
            w.contains("`hotkey` is not a usable key combination, so Ctrl+Shift+Z is used"),
            "{w}"
        );
        assert_eq!(s.hotkey, None);
        assert!(s.hotkey_invalid);
        assert_eq!(s.filter_name(), "a.filter");
        assert!(!s.extra.contains_key("hotkey"));
        assert!(s.extra.contains_key("future"));
    }

    #[test]
    fn hotkey_salvage_keeps_a_valid_value_beside_a_bad_key() {
        let t = tempfile::tempdir().unwrap();
        fs::write(
            path(t.path()),
            r#"{"hotkey":"Ctrl+Alt+K","game_folder":42}"#,
        )
        .unwrap();
        let (s, w) = load(t.path());
        let w = w.unwrap();
        assert!(w.contains("default game folder"), "{w}");
        assert!(!w.contains("hotkey"), "{w}");
        assert!(!s.hotkey_invalid);
        assert_eq!(s.saved_hotkey(), Some(ctrl_alt_k()));
    }

    #[test]
    fn hotkey_warning_composes_with_other_key_warnings() {
        let t = tempfile::tempdir().unwrap();
        fs::write(path(t.path()), r#"{"game_folder":"rel","hotkey":"Z"}"#).unwrap();
        let (s, w) = load(t.path());
        let w = w.unwrap();
        assert!(
            w.contains("`game_folder` `rel` is not an absolute folder path"),
            "{w}"
        );
        assert!(w.contains("default game folder"), "{w}");
        assert!(
            w.contains("`hotkey` is not a usable key combination, so Ctrl+Shift+Z is used"),
            "{w}"
        );
        assert!(w.ends_with('.') && !w.ends_with(".."), "{w}");
        assert_eq!(s.game_folder, None);
        assert_eq!(s.hotkey, None);

        // The salvage path composes the same way (wrongly typed league plus a bad hotkey).
        let t = tempfile::tempdir().unwrap();
        fs::write(path(t.path()), r#"{"league":1,"hotkey":"Z"}"#).unwrap();
        let (_, w) = load(t.path());
        let w = w.unwrap();
        assert!(w.contains("`league` is not text"), "{w}");
        assert!(
            w.contains("`hotkey` is not a usable key combination"),
            "{w}"
        );
    }

    #[test]
    fn hotkey_bad_value_alone_has_its_own_warning_and_keeps_the_filter() {
        let t = tempfile::tempdir().unwrap();
        fs::write(path(t.path()), r#"{"filter":"a.filter","hotkey":"Z"}"#).unwrap();
        let loaded = load_detailed(t.path());
        assert!(!loaded.filter_fell_back);
        assert_eq!(loaded.settings.filter_name(), "a.filter");
        assert_eq!(
            loaded.warning.as_deref(),
            Some(
                "ff-settings.json could not be fully used; `hotkey` is not a usable key \
                 combination, so Ctrl+Shift+Z is used."
            )
        );
    }

    #[test]
    fn a_dropped_hotkey_is_removed_by_a_later_whole_save() {
        let t = tempfile::tempdir().unwrap();
        fs::write(path(t.path()), r#"{"hotkey":"Z","filter":"a.filter"}"#).unwrap();
        let (s, _) = load(t.path());
        save(t.path(), &s).unwrap();
        let text = fs::read_to_string(path(t.path())).unwrap();
        assert!(!text.contains("hotkey"), "{text}");
    }

    #[test]
    fn render_hotkey_rules() {
        assert_eq!(
            render_hotkey(None, "Ctrl+Alt+K").unwrap(),
            b"{\n  \"hotkey\": \"Ctrl+Alt+K\"\n}\n".to_vec()
        );
        let out =
            render_hotkey(Some(br#"{"filter":"a.filter","future":1}"#), "Ctrl+Alt+K").unwrap();
        let v: serde_json::Value = serde_json::from_slice(&out).unwrap();
        assert_eq!(v["filter"], "a.filter");
        assert_eq!(v["future"], 1);
        assert_eq!(v["hotkey"], "Ctrl+Alt+K");
        assert!(out.ends_with(b"}\n"));

        // An existing hotkey is replaced, a wrongly typed other key kept as is.
        let out = render_hotkey(
            Some(br#"{"hotkey":"Ctrl+Shift+Z","reload_after_apply":"maybe"}"#),
            "F5",
        )
        .unwrap();
        let v: serde_json::Value = serde_json::from_slice(&out).unwrap();
        assert_eq!(v["hotkey"], "F5");
        assert_eq!(v["reload_after_apply"], "maybe");

        assert_eq!(render_hotkey(Some(b"{not json"), "F5"), Err(()));
        assert_eq!(render_hotkey(Some(b"[1]"), "F5"), Err(()));
    }
}
