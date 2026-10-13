//! Pure model of the CONFIG tab: working and last-applied snapshots of `styles.json` and
//! `tiers.json`, tier text buffers, dirty counting and validation.
//!
//! Layer rule (same as `overlay/apply.rs`): no GUI-framework types live here, so every CONF-02/03/04
//! rule is provable with temp dirs -- `OverlayApp::new` needs a real tray and cannot be unit-tested.
//! File paths come only from `InputPaths::relative_to(root)`. This module writes only `styles.json`
//! and `tiers.json`; the committed early-league fallback pair is never
//! touched.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use crate::compile::InputPaths;
use crate::models::config::{
    Beam, Config, Minimap, Sound, Style, StyleAction, StyleNameError, StyleSet,
};
use crate::models::tiers::{TierEntry, TierValidationError, ValidatedTiers};
use crate::safe_write::{AllOrNoneError, Target, WriteOutcome, write_all_or_none};

/// Display order of the tier rows. Presentation only: rows map to entries by tier name.
const DISPLAY_ORDER: [&str; 7] = ["S", "A", "B", "C", "D", "E", "F"];

/// Shown beside a tier row whose text is not a finite number >= 0.
const ENTER_A_NUMBER: &str = "Enter a number";

/// Out-of-date notice repeated after a config Apply.
pub const OUT_OF_DATE_MESSAGE: &str =
    "Filter out of date: config changed. Press Apply on the RULES tab to regenerate it.";

/// Tooltip on a disabled Apply button while tier errors exist.
pub const APPLY_BLOCKED_TOOLTIP: &str = "Fix the tier errors first.";

/// `Config: no unapplied changes` / `Config: 1 unapplied change` / `Config: N unapplied changes`.
pub fn dirty_status(n: usize) -> String {
    match n {
        0 => "Config: no unapplied changes".to_string(),
        1 => "Config: 1 unapplied change".to_string(),
        n => format!("Config: {n} unapplied changes"),
    }
}

/// Which of a style's three colour slots an edit targets.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColourSlot {
    Text,
    Border,
    Background,
}

/// One edit to a style. Colours are opaque RGB triples (the emitter keeps alpha 255). `Font` is
/// `i64` so the view's number field can pass out-of-range values; values outside 0..=255 are
/// refused, 0..=255 are stored and left for `StyleSet::validate` to flag.
#[derive(Debug, Clone, PartialEq)]
pub enum StyleEdit {
    Action(StyleAction),
    Colour {
        slot: ColourSlot,
        value: Option<[u8; 3]>,
    },
    Font(Option<i64>),
    Sound(Option<Sound>),
    Minimap(Option<Minimap>),
    Beam(Option<Beam>),
    DisableDropSound(bool),
}

/// Badges shown on a style row.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StyleBadges {
    pub builtin: bool,
    pub new: bool,
    pub edited: bool,
    pub hide: bool,
}

/// One row of the tier editor.
#[derive(Debug, Clone, PartialEq)]
pub struct TierRow {
    pub tier: String,
    pub buffer: String,
    pub unit: &'static str,
    pub error: Option<String>,
    /// True on both rows of an out-of-order pair.
    pub edge: bool,
}

/// Cached validation outcome for the tier rows.
#[derive(Debug, Clone, Default)]
struct TierErrors {
    /// tier -> message
    row: BTreeMap<String, String>,
    edges: Vec<String>,
    general: Option<String>,
}

pub struct ConfigState {
    root: PathBuf,
    styles: StyleSet,
    styles_applied: StyleSet,
    /// In loaded file order.
    tiers: Vec<TierEntry>,
    tiers_applied: Vec<TierEntry>,
    buffers: BTreeMap<String, String>,
    errors: TierErrors,
    styles_bytes: Vec<u8>,
    tiers_bytes: Vec<u8>,
    styles_mtime: Option<SystemTime>,
    tiers_mtime: Option<SystemTime>,
}

fn file_label(path: &Path) -> String {
    path.file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| path.display().to_string())
}

fn mtime_of(path: &Path) -> Option<SystemTime> {
    fs::metadata(path).and_then(|m| m.modified()).ok()
}

fn tiers_equal(a: &[TierEntry], b: &[TierEntry]) -> bool {
    a.len() == b.len()
        && a.iter()
            .zip(b)
            .all(|(x, y)| x.tier == y.tier && x.min == y.min && x.unit == y.unit)
}

impl ConfigState {
    /// Read `styles.json` and `tiers.json` under `root`. Errors read `<file name>: <reason>`.
    pub fn load(root: &Path) -> Result<ConfigState, String> {
        let paths = InputPaths::relative_to(root);
        let read = |p: &Path| fs::read(p).map_err(|e| format!("{}: {e}", file_label(p)));

        let styles_bytes = read(&paths.styles)?;
        let styles: StyleSet = serde_json::from_slice(&styles_bytes)
            .map_err(|e| format!("{}: {e}", file_label(&paths.styles)))?;
        styles
            .validate()
            .map_err(|e| format!("{}: {e}", file_label(&paths.styles)))?;

        let tiers_bytes = read(&paths.tiers)?;
        let tiers: Vec<TierEntry> = serde_json::from_slice(&tiers_bytes)
            .map_err(|e| format!("{}: {e}", file_label(&paths.tiers)))?;
        ValidatedTiers::try_from(tiers.clone())
            .map_err(|e| format!("{}: {e}", file_label(&paths.tiers)))?;

        let mut state = ConfigState {
            root: root.to_path_buf(),
            styles_applied: styles.clone(),
            styles,
            tiers_applied: tiers.clone(),
            tiers,
            buffers: BTreeMap::new(),
            errors: TierErrors::default(),
            styles_mtime: mtime_of(&paths.styles),
            tiers_mtime: mtime_of(&paths.tiers),
            styles_bytes,
            tiers_bytes,
        };
        state.resync_buffers();
        Ok(state)
    }

    fn resync_buffers(&mut self) {
        self.buffers = self
            .tiers
            .iter()
            .map(|e| (e.tier.clone(), format!("{}", e.min)))
            .collect();
        self.errors = TierErrors::default();
    }

    // -- styles ---------------------------------------------------------------------------------

    /// Style names in file order.
    pub fn style_names(&self) -> Vec<String> {
        self.styles.names().map(str::to_string).collect()
    }

    pub fn style(&self, name: &str) -> Option<&Style> {
        self.styles.get(name)
    }

    pub fn badges(&self, name: &str) -> StyleBadges {
        let applied = self.styles_applied.get(name);
        let now = self.styles.get(name);
        StyleBadges {
            builtin: StyleSet::is_builtin(name),
            new: applied.is_none(),
            edited: matches!((applied, now), (Some(a), Some(b)) if a != b),
            hide: matches!(now, Some(s) if s.action == StyleAction::Hide),
        }
    }

    /// Trim, then add a blank Show style. Returns the stored name.
    pub fn add_style(&mut self, input: &str) -> Result<String, StyleNameError> {
        let trimmed = input.trim();
        self.styles.insert_new(trimmed, Style::blank_show())?;
        Ok(trimmed.to_string())
    }

    /// False (and no change) for built-ins and unknown names.
    pub fn delete_style(&mut self, name: &str) -> bool {
        self.styles.remove_custom(name).is_some()
    }

    /// Apply one edit. False (model unchanged) for an unknown style or a font that cannot fit.
    pub fn edit_style(&mut self, name: &str, edit: StyleEdit) -> bool {
        let font = match &edit {
            StyleEdit::Font(Some(v)) => match u8::try_from(*v) {
                Ok(v) => Some(Some(v)),
                Err(_) => return false,
            },
            StyleEdit::Font(None) => Some(None),
            _ => None,
        };
        let Some(style) = self.styles.get_mut(name) else {
            return false;
        };
        match edit {
            StyleEdit::Action(a) => style.action = a,
            StyleEdit::Colour { slot, value } => match slot {
                ColourSlot::Text => style.text = value,
                ColourSlot::Border => style.border = value,
                ColourSlot::Background => style.background = value,
            },
            StyleEdit::Font(_) => style.font = font.flatten(),
            StyleEdit::Sound(s) => style.sound = s,
            StyleEdit::Minimap(m) => style.minimap = m,
            StyleEdit::Beam(b) => style.beam = b,
            StyleEdit::DisableDropSound(b) => style.disable_drop_sound = b,
        }
        true
    }

    /// The `StyleSet::validate` message when the working styles are invalid.
    pub fn styles_error(&self) -> Option<String> {
        self.styles.validate().err().map(|e| e.to_string())
    }

    // -- tiers ----------------------------------------------------------------------------------

    /// Rows in display order S..F. Unknown/missing tiers are skipped.
    pub fn tier_rows(&self) -> Vec<TierRow> {
        DISPLAY_ORDER
            .iter()
            .filter_map(|t| {
                let entry = self.tiers.iter().find(|e| e.tier == *t)?;
                Some(TierRow {
                    tier: entry.tier.clone(),
                    buffer: self.buffers.get(*t).cloned().unwrap_or_default(),
                    unit: entry.unit.as_str(),
                    error: self.errors.row.get(*t).cloned(),
                    edge: self.errors.edges.iter().any(|e| e == t),
                })
            })
            .collect()
    }

    /// Store the text; if it parses to a finite number >= 0, write it into the model.
    pub fn set_tier_text(&mut self, tier: &str, text: &str) {
        if !self.tiers.iter().any(|e| e.tier == tier) {
            return;
        }
        self.buffers.insert(tier.to_string(), text.to_string());
        if let Some(v) = parse_cutoff(text) {
            if let Some(entry) = self.tiers.iter_mut().find(|e| e.tier == tier) {
                entry.min = v;
            }
        }
        self.recompute_tier_errors();
    }

    fn recompute_tier_errors(&mut self) {
        let mut errors = TierErrors::default();
        for (tier, text) in &self.buffers {
            if parse_cutoff(text).is_none() {
                errors.row.insert(tier.clone(), ENTER_A_NUMBER.to_string());
            }
        }
        match ValidatedTiers::try_from(self.tiers.clone()) {
            Ok(_) => {}
            Err(TierValidationError::NotAscending {
                prev,
                prev_min,
                curr,
                ..
            }) => {
                errors
                    .row
                    .entry(curr.clone())
                    .or_insert_with(|| format!("Must be higher than {prev} ({prev_min})."));
                errors.edges = vec![prev, curr];
            }
            Err(e @ TierValidationError::MissingTiers(_)) => {
                errors.general = Some(e.to_string());
            }
        }
        self.errors = errors;
    }

    pub fn has_tier_errors(&self) -> bool {
        !self.errors.row.is_empty()
    }

    /// A problem not tied to one row (a required tier is missing).
    pub fn tiers_error(&self) -> Option<String> {
        self.errors.general.clone()
    }

    // -- dirty / revert -------------------------------------------------------------------------

    /// Styles added + deleted + edited (by name, value compare) + tier rows that differ.
    pub fn dirty_count(&self) -> usize {
        let added = self
            .styles
            .names()
            .filter(|n| !self.styles_applied.contains(n))
            .count();
        let deleted = self
            .styles_applied
            .names()
            .filter(|n| !self.styles.contains(n))
            .count();
        let edited = self
            .styles
            .iter()
            .filter(|(n, s)| matches!(self.styles_applied.get(n), Some(a) if a != *s))
            .count();
        let tiers = self
            .tiers
            .iter()
            .filter(|e| {
                let Some(base) = self.tiers_applied.iter().find(|a| a.tier == e.tier) else {
                    return true;
                };
                let buffer_differs = self
                    .buffers
                    .get(&e.tier)
                    .is_some_and(|b| *b != format!("{}", base.min));
                buffer_differs || e.min != base.min
            })
            .count();
        added + deleted + edited + tiers
    }

    pub fn can_apply(&self) -> bool {
        self.dirty_count() > 0
            && !self.has_tier_errors()
            && self.tiers_error().is_none()
            && self.styles_error().is_none()
    }

    /// Discard in-memory edits back to the last-applied snapshot.
    pub fn revert(&mut self) {
        self.styles = self.styles_applied.clone();
        self.tiers = self.tiers_applied.clone();
        self.resync_buffers();
    }

    // -- apply / outside edits ------------------------------------------------------------------

    /// Cheap check for tab activation: compare current mtimes to the remembered ones.
    pub fn outside_by_mtime(&self) -> OutsideEdit {
        let paths = InputPaths::relative_to(&self.root);
        outside_from(
            mtime_of(&paths.styles) != self.styles_mtime,
            mtime_of(&paths.tiers) != self.tiers_mtime,
        )
    }

    /// Authoritative check: current bytes against the loaded baselines (mtime can be coarse or
    /// touched without a content change).
    pub fn outside_by_bytes(&self) -> OutsideEdit {
        let paths = InputPaths::relative_to(&self.root);
        let differs = |p: &Path, base: &[u8]| fs::read(p).map(|b| b != base).unwrap_or(true);
        outside_from(
            differs(&paths.styles, &self.styles_bytes),
            differs(&paths.tiers, &self.tiers_bytes),
        )
    }

    /// Validate both files, then write `styles.json` and `tiers.json` all-or-none. A file whose
    /// working value equals its applied snapshot keeps its loaded bytes (no reformatting).
    pub fn apply(&mut self) -> Result<ConfigApplyReport, ConfigApplyError> {
        self.apply_inner(false)
    }

    fn apply_inner(&mut self, force: bool) -> Result<ConfigApplyReport, ConfigApplyError> {
        if !self.can_apply() {
            let reason = self
                .styles_error()
                .or_else(|| self.tiers_error())
                .unwrap_or_else(|| "Fix the tier errors first".to_string());
            return Err(ConfigApplyError::Invalid(reason));
        }
        // Defensive re-check: never render something the loaders would reject.
        self.styles
            .validate()
            .map_err(|e| ConfigApplyError::Invalid(e.to_string()))?;
        ValidatedTiers::try_from(self.tiers.clone())
            .map_err(|e| ConfigApplyError::Invalid(e.to_string()))?;

        let styles_new = if force || self.styles != self.styles_applied {
            crate::loader::render_styles(&self.styles).into_bytes()
        } else {
            self.styles_bytes.clone()
        };
        let tiers_new = if force || !tiers_equal(&self.tiers, &self.tiers_applied) {
            crate::loader::render_tiers(&self.tiers).into_bytes()
        } else {
            self.tiers_bytes.clone()
        };

        let paths = InputPaths::relative_to(&self.root);
        let targets = [
            Target {
                path: &paths.styles,
                loaded: Some(&self.styles_bytes),
                new: &styles_new,
            },
            Target {
                path: &paths.tiers,
                loaded: Some(&self.tiers_bytes),
                new: &tiers_new,
            },
        ];
        let outcomes = match write_all_or_none(&targets) {
            Ok(o) => o,
            Err(e) => {
                if e.changed_on_disk_path().is_some() {
                    let which = self.outside_by_bytes();
                    if which != OutsideEdit::None {
                        return Err(ConfigApplyError::OutsideEdit(which));
                    }
                }
                return Err(ConfigApplyError::Write(write_message(&e)));
            }
        };
        let written = |i: usize| matches!(outcomes.get(i), Some(WriteOutcome::Written { .. }));
        let (styles_written, tiers_written) = (written(0), written(1));

        self.styles_applied = self.styles.clone();
        self.tiers_applied = self.tiers.clone();
        self.styles_bytes = styles_new;
        self.tiers_bytes = tiers_new;
        self.styles_mtime = mtime_of(&paths.styles);
        self.tiers_mtime = mtime_of(&paths.tiers);
        self.resync_buffers();
        Ok(ConfigApplyReport {
            styles: self.styles.len(),
            tiers: self.tiers.len(),
            styles_written,
            tiers_written,
        })
    }

    /// Discard edits and return to the on-disk content (including any outside change).
    pub fn reload(&mut self) -> Result<(), String> {
        *self = ConfigState::load(&self.root)?;
        Ok(())
    }

    /// Apply the in-memory edits over an outside change. Only call after the user's explicit
    /// Overwrite confirm; both files are rewritten from the in-memory state.
    pub fn overwrite(&mut self) -> Result<ConfigApplyReport, ConfigApplyError> {
        let paths = InputPaths::relative_to(&self.root);
        let read = |p: &Path| {
            fs::read(p).map_err(|e| {
                ConfigApplyError::Write(not_applied_message(&format!("{}: {}", file_label(p), e)))
            })
        };
        self.styles_bytes = read(&paths.styles)?;
        self.tiers_bytes = read(&paths.tiers)?;
        self.styles_mtime = mtime_of(&paths.styles);
        self.tiers_mtime = mtime_of(&paths.tiers);
        self.apply_inner(true)
    }
}

fn outside_from(styles: bool, tiers: bool) -> OutsideEdit {
    match (styles, tiers) {
        (false, false) => OutsideEdit::None,
        (true, false) => OutsideEdit::Styles,
        (false, true) => OutsideEdit::Tiers,
        (true, true) => OutsideEdit::Both,
    }
}

/// The user-facing sentence for a failed write. A failed rollback must not claim that nothing
/// changed, so it uses the error's own text.
fn write_message(e: &AllOrNoneError) -> String {
    match e {
        AllOrNoneError::NothingWritten(w) => not_applied_message(&w.to_string()),
        AllOrNoneError::RolledBack { failed } => not_applied_message(&failed.to_string()),
        AllOrNoneError::RollbackFailed { .. } => e.to_string(),
    }
}

/// Which config file(s) changed outside the overlay.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutsideEdit {
    None,
    Styles,
    Tiers,
    Both,
}

/// Why a config Apply did not happen. `Write` carries the finished sentence.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConfigApplyError {
    Invalid(String),
    OutsideEdit(OutsideEdit),
    Write(String),
}

impl ConfigApplyError {
    pub fn message(&self) -> String {
        match self {
            ConfigApplyError::Invalid(reason) => not_applied_message(reason),
            ConfigApplyError::OutsideEdit(which) => outside_edit_message(*which).to_string(),
            ConfigApplyError::Write(text) => text.clone(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfigApplyReport {
    pub styles: usize,
    pub tiers: usize,
    pub styles_written: bool,
    pub tiers_written: bool,
}

/// Styles used by enabled rules and the two `alwaysShow` entries.
pub fn config_style_refs(config: &Config) -> BTreeSet<String> {
    let mut refs: BTreeSet<String> = config
        .rules
        .iter()
        .flatten()
        .filter(|r| r.enabled != Some(false))
        .map(|r| r.style.clone())
        .collect();
    refs.insert(config.always_show.currency.style.clone());
    refs.insert(config.always_show.recycle.style.clone());
    refs
}

/// Body of the delete confirm, prefixed with a rule count when rules use the style.
pub fn delete_confirm_text(name: &str, users: usize) -> String {
    let prefix = match users {
        0 => String::new(),
        1 => "1 rule uses this style. ".to_string(),
        n => format!("{n} rules use this style. "),
    };
    format!(
        "{prefix}Rules using \"{name}\" keep their current actions but lose their style link on the next Apply."
    )
}

/// Extra warning when `config.json` itself references the style being deleted.
pub fn config_warning(name: &str) -> String {
    format!(
        "config.json uses \"{name}\", so ferrisfilter generate will fail until you add the style back or change those rules."
    )
}

pub fn applied_message(styles: usize) -> String {
    format!("Applied config: {styles} styles, 7 tiers.")
}

pub fn not_applied_message(reason: &str) -> String {
    format!(
        "Config not applied: {}. Nothing was changed on disk.",
        reason.trim_end_matches('.')
    )
}

pub fn outside_edit_message(which: OutsideEdit) -> &'static str {
    match which {
        OutsideEdit::Styles => "styles.json changed outside the overlay.",
        OutsideEdit::Tiers => "tiers.json changed outside the overlay.",
        OutsideEdit::Both | OutsideEdit::None => "Both config files changed outside the overlay.",
    }
}

/// A cutoff is a finite number >= 0.
fn parse_cutoff(text: &str) -> Option<f64> {
    let v = text.trim().parse::<f64>().ok()?;
    (v.is_finite() && v >= 0.0).then_some(v)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_root() -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        for f in ["styles.json", "tiers.json"] {
            fs::copy(
                Path::new(env!("CARGO_MANIFEST_DIR")).join(f),
                dir.path().join(f),
            )
            .unwrap();
        }
        dir
    }

    fn loaded() -> (tempfile::TempDir, ConfigState) {
        let dir = temp_root();
        let s = ConfigState::load(dir.path()).unwrap();
        (dir, s)
    }

    #[test]
    fn load_reads_styles_and_tiers() {
        let (_d, s) = loaded();
        assert_eq!(s.style_names().len(), 12);
        let rows = s.tier_rows();
        let tiers: Vec<_> = rows.iter().map(|r| r.tier.as_str()).collect();
        assert_eq!(tiers, ["S", "A", "B", "C", "D", "E", "F"]);
        let bufs: Vec<_> = rows.iter().map(|r| r.buffer.as_str()).collect();
        assert_eq!(bufs, ["30", "20", "12", "8", "2", "0.5", "0.1"]);
        assert!(rows.iter().all(|r| r.unit == "chaos"));
        assert_eq!(s.dirty_count(), 0);
        assert!(!s.can_apply());
    }

    #[test]
    fn load_errors_name_the_file() {
        let d = temp_root();
        fs::write(d.path().join("styles.json"), "{ not json").unwrap();
        let e = ConfigState::load(d.path()).err().unwrap();
        assert!(e.starts_with("styles.json:"), "{e}");

        let d = temp_root();
        fs::remove_file(d.path().join("tiers.json")).unwrap();
        let e = ConfigState::load(d.path()).err().unwrap();
        assert!(e.starts_with("tiers.json:"), "{e}");

        let d = temp_root();
        fs::remove_file(d.path().join("styles.json")).unwrap();
        let e = ConfigState::load(d.path()).err().unwrap();
        assert!(e.starts_with("styles.json:"), "{e}");
    }

    #[test]
    fn add_style_trims_and_rejects_bad_names() {
        let (_d, mut s) = loaded();
        assert_eq!(s.add_style("  Gold ").unwrap(), "Gold");
        assert_eq!(s.dirty_count(), 1);
        assert!(s.badges("Gold").new);
        assert_eq!(s.add_style("Gold"), Err(StyleNameError::Duplicate));
        assert_eq!(s.add_style("   "), Err(StyleNameError::Empty));
    }

    #[test]
    fn delete_style_protects_builtins_and_revert_restores_position() {
        let (_d, mut s) = loaded();
        let before = s.style_names();
        assert!(!s.delete_style("S"));
        assert_eq!(s.dirty_count(), 0);
        assert!(s.delete_style("kite"));
        assert_eq!(s.dirty_count(), 1);
        assert!(!s.style_names().contains(&"kite".to_string()));
        s.revert();
        assert_eq!(s.style_names(), before);
        assert_eq!(s.dirty_count(), 0);
    }

    #[test]
    fn style_dirty_is_value_compare() {
        let (_d, mut s) = loaded();
        assert!(s.edit_style("S", StyleEdit::Font(Some(40))));
        assert!(s.badges("S").edited);
        assert_eq!(s.dirty_count(), 1);
        assert!(s.edit_style("S", StyleEdit::Font(Some(37))));
        assert_eq!(s.dirty_count(), 0);
        assert!(!s.badges("S").edited);
    }

    #[test]
    fn font_out_of_u8_is_refused_and_out_of_range_blocks_apply() {
        let (_d, mut s) = loaded();
        assert!(!s.edit_style("S", StyleEdit::Font(Some(300))));
        assert!(!s.edit_style("S", StyleEdit::Font(Some(-1))));
        assert_eq!(s.dirty_count(), 0);
        assert!(s.edit_style("S", StyleEdit::Font(Some(50))));
        let err = s.styles_error().unwrap();
        assert!(err.contains("font"), "{err}");
        assert!(!s.can_apply());
    }

    #[test]
    fn other_style_edits_are_stored_verbatim() {
        let (_d, mut s) = loaded();
        assert!(s.edit_style(
            "S",
            StyleEdit::Colour {
                slot: ColourSlot::Text,
                value: None
            }
        ));
        assert_eq!(s.style("S").unwrap().text, None);
        assert!(s.edit_style(
            "S",
            StyleEdit::Colour {
                slot: ColourSlot::Text,
                value: Some([1, 2, 3])
            }
        ));
        assert_eq!(s.style("S").unwrap().text, Some([1, 2, 3]));
        assert!(s.edit_style("S", StyleEdit::Action(StyleAction::Hide)));
        assert!(s.badges("S").hide);
        assert!(s.edit_style("S", StyleEdit::DisableDropSound(true)));
        assert!(s.style("S").unwrap().disable_drop_sound);
        assert!(!s.edit_style("nope", StyleEdit::DisableDropSound(true)));
    }

    #[test]
    fn tier_out_of_order_reports_and_blocks() {
        let (_d, mut s) = loaded();
        s.set_tier_text("S", "5");
        let rows = s.tier_rows();
        let srow = &rows[0];
        assert_eq!(srow.error.as_deref(), Some("Must be higher than A (20)."));
        assert!(srow.edge && rows[1].edge);
        assert!(!rows[2].edge);
        assert!(!s.can_apply());
        s.set_tier_text("S", "35");
        assert!(s.tier_rows()[0].error.is_none());
        assert_eq!(s.dirty_count(), 1);
        assert!(s.can_apply());
    }

    #[test]
    fn tier_bad_text_keeps_model_and_blocks() {
        for bad in ["abc", "", "nan", "inf", "-1"] {
            let (_d, mut s) = loaded();
            s.set_tier_text("D", bad);
            let row = s.tier_rows().into_iter().find(|r| r.tier == "D").unwrap();
            assert_eq!(row.error.as_deref(), Some("Enter a number"), "{bad:?}");
            assert_eq!(s.tiers.iter().find(|e| e.tier == "D").unwrap().min, 2.0);
            assert!(!s.can_apply());
            assert_eq!(s.dirty_count(), 1, "{bad:?}");
            s.set_tier_text("D", "2");
            assert!(s.tier_rows().iter().all(|r| r.error.is_none()));
            assert_eq!(s.dirty_count(), 0);
        }
    }

    #[test]
    fn tier_valid_change_keeps_file_order() {
        let (_d, mut s) = loaded();
        s.set_tier_text("C", "2.5");
        assert!(!s.has_tier_errors());
        let order: Vec<_> = s.tiers.iter().map(|e| e.tier.as_str()).collect();
        assert_eq!(order, ["F", "E", "D", "C", "B", "A", "S"]);
        s.set_tier_text("Z", "9");
        assert_eq!(s.dirty_count(), 1);
    }

    #[test]
    fn dirty_status_copy() {
        assert_eq!(dirty_status(0), "Config: no unapplied changes");
        assert_eq!(dirty_status(1), "Config: 1 unapplied change");
        assert_eq!(dirty_status(3), "Config: 3 unapplied changes");
    }

    const EXTRA: [&str; 3] = ["config.json", "prices-fallback.json", "tiers-fallback.json"];

    fn full_root() -> tempfile::TempDir {
        let dir = temp_root();
        for f in EXTRA {
            fs::copy(
                Path::new(env!("CARGO_MANIFEST_DIR")).join(f),
                dir.path().join(f),
            )
            .unwrap();
        }
        dir
    }

    fn listing(dir: &Path) -> Vec<String> {
        let mut v: Vec<_> = fs::read_dir(dir)
            .unwrap()
            .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        v.sort();
        v
    }

    #[test]
    fn apply_writes_only_the_two_files() {
        let dir = full_root();
        let mut s = ConfigState::load(dir.path()).unwrap();
        let styles_before = fs::read(dir.path().join("styles.json")).unwrap();
        let others: Vec<_> = EXTRA
            .iter()
            .map(|f| (f, fs::read(dir.path().join(f)).unwrap()))
            .collect();
        let names_before = listing(dir.path());

        s.set_tier_text("S", "35");
        let report = s.apply().unwrap();
        assert_eq!(report.styles, 12);
        assert_eq!(report.tiers, 7);
        assert!(report.tiers_written);
        assert!(!report.styles_written);
        assert_eq!(s.dirty_count(), 0);

        let tiers: Vec<TierEntry> =
            serde_json::from_slice(&fs::read(dir.path().join("tiers.json")).unwrap()).unwrap();
        assert_eq!(tiers.iter().find(|e| e.tier == "S").unwrap().min, 35.0);
        assert_eq!(
            fs::read(dir.path().join("styles.json")).unwrap(),
            styles_before
        );
        for (f, bytes) in others {
            assert_eq!(fs::read(dir.path().join(f)).unwrap(), bytes, "{f}");
        }
        assert_eq!(listing(dir.path()), names_before);

        let again = ConfigState::load(dir.path()).unwrap();
        let s_row = &again.tier_rows()[0];
        assert_eq!(s_row.buffer, "35");
    }

    #[test]
    fn apply_new_style_writes_it_last() {
        let (dir, mut s) = loaded();
        s.add_style("Gold").unwrap();
        let report = s.apply().unwrap();
        assert!(report.styles_written);
        assert_eq!(report.styles, 13);
        let text = fs::read_to_string(dir.path().join("styles.json")).unwrap();
        let lines: Vec<_> = text.lines().collect();
        assert!(lines[lines.len() - 2].trim_start().starts_with("\"Gold\""));
        assert_eq!(
            ConfigState::load(dir.path()).unwrap().style_names().len(),
            13
        );
    }

    #[test]
    fn apply_refuses_when_not_applicable() {
        let (dir, mut s) = loaded();
        let before = fs::read(dir.path().join("tiers.json")).unwrap();
        assert!(matches!(s.apply(), Err(ConfigApplyError::Invalid(_))));
        s.set_tier_text("S", "5");
        let e = s.apply().unwrap_err();
        assert!(matches!(e, ConfigApplyError::Invalid(_)));
        assert!(e.message().starts_with("Config not applied: "));
        assert_eq!(fs::read(dir.path().join("tiers.json")).unwrap(), before);
    }

    #[test]
    fn outside_edit_refuses() {
        let (dir, mut s) = loaded();
        s.set_tier_text("S", "35");
        let tiers_path = dir.path().join("tiers.json");
        let mut text = fs::read_to_string(&tiers_path).unwrap();
        text.push('\n');
        fs::write(&tiers_path, &text).unwrap();
        assert_eq!(s.outside_by_bytes(), OutsideEdit::Tiers);
        let e = s.apply().unwrap_err();
        assert_eq!(e, ConfigApplyError::OutsideEdit(OutsideEdit::Tiers));
        assert_eq!(fs::read_to_string(&tiers_path).unwrap(), text);
        assert_eq!(s.dirty_count(), 1);
    }

    #[test]
    fn overwrite_applies_edits_over_an_outside_change() {
        let (dir, mut s) = loaded();
        s.set_tier_text("S", "35");
        let tiers_path = dir.path().join("tiers.json");
        let mut text = fs::read_to_string(&tiers_path).unwrap();
        text.push('\n');
        fs::write(&tiers_path, text).unwrap();
        assert!(s.apply().is_err());
        let report = s.overwrite().unwrap();
        assert!(report.tiers_written);
        let tiers: Vec<TierEntry> =
            serde_json::from_slice(&fs::read(&tiers_path).unwrap()).unwrap();
        assert_eq!(tiers.iter().find(|e| e.tier == "S").unwrap().min, 35.0);
        assert_eq!(s.outside_by_bytes(), OutsideEdit::None);
    }

    #[test]
    fn reload_discards_edits_and_takes_outside_change() {
        let (dir, mut s) = loaded();
        s.set_tier_text("S", "35");
        s.add_style("Gold").unwrap();
        let tiers_path = dir.path().join("tiers.json");
        let text = fs::read_to_string(&tiers_path)
            .unwrap()
            .replace("30.0", "31.0");
        fs::write(&tiers_path, text).unwrap();
        s.reload().unwrap();
        assert_eq!(s.dirty_count(), 0);
        assert_eq!(s.tier_rows()[0].buffer, "31");
        assert_eq!(s.style_names().len(), 12);
    }

    #[test]
    fn revert_after_apply_returns_to_applied_values() {
        let (_d, mut s) = loaded();
        s.set_tier_text("S", "35");
        s.apply().unwrap();
        s.set_tier_text("S", "40");
        assert_eq!(s.dirty_count(), 1);
        s.revert();
        assert_eq!(s.tier_rows()[0].buffer, "35");
        assert_eq!(s.dirty_count(), 0);
    }

    #[test]
    fn mtime_guard_detects_outside_write_and_clears_after_own_apply() {
        let (dir, mut s) = loaded();
        assert_eq!(s.outside_by_mtime(), OutsideEdit::None);
        std::thread::sleep(std::time::Duration::from_millis(1100));
        let styles_path = dir.path().join("styles.json");
        let bytes = fs::read(&styles_path).unwrap();
        fs::write(&styles_path, bytes).unwrap();
        assert_eq!(s.outside_by_mtime(), OutsideEdit::Styles);
        let tiers_path = dir.path().join("tiers.json");
        let bytes = fs::read(&tiers_path).unwrap();
        fs::write(&tiers_path, bytes).unwrap();
        assert_eq!(s.outside_by_mtime(), OutsideEdit::Both);
        s.reload().unwrap();
        s.set_tier_text("S", "35");
        s.apply().unwrap();
        assert_eq!(s.outside_by_mtime(), OutsideEdit::None);
    }

    #[test]
    fn config_style_refs_covers_rules_and_always_show() {
        let text =
            fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("config.json")).unwrap();
        let mut config: Config = serde_json::from_str(&text).unwrap();
        let refs = config_style_refs(&config);
        for n in ["exceptional", "kite", "Recycle"] {
            assert!(refs.contains(n), "{n}");
        }
        assert!(refs.contains(&config.always_show.currency.style));
        let rules = config.rules.as_mut().unwrap();
        for r in rules.iter_mut().filter(|r| r.style == "kite") {
            r.enabled = Some(false);
        }
        assert!(!config_style_refs(&config).contains("kite"));
    }

    #[test]
    fn message_copy() {
        assert_eq!(
            delete_confirm_text("kite", 0),
            "Rules using \"kite\" keep their current actions but lose their style link on the next Apply."
        );
        assert!(delete_confirm_text("kite", 3).starts_with("3 rules use this style. Rules using"));
        assert!(delete_confirm_text("kite", 1).starts_with("1 rule uses this style. Rules using"));
        assert_eq!(
            config_warning("kite"),
            "config.json uses \"kite\", so ferrisfilter generate will fail until you add the style back or change those rules."
        );
        assert_eq!(applied_message(12), "Applied config: 12 styles, 7 tiers.");
        assert_eq!(
            not_applied_message("x"),
            "Config not applied: x. Nothing was changed on disk."
        );
        assert_eq!(
            outside_edit_message(OutsideEdit::Styles),
            "styles.json changed outside the overlay."
        );
        assert_eq!(
            outside_edit_message(OutsideEdit::Tiers),
            "tiers.json changed outside the overlay."
        );
        assert_eq!(
            outside_edit_message(OutsideEdit::Both),
            "Both config files changed outside the overlay."
        );
    }
}
