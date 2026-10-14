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

use crate::compile::InputPaths;
use crate::models::config::{
    Beam, Config, Minimap, Sound, Style, StyleAction, StyleNameError, StyleSet,
};
use crate::models::tiers::{TierEntry, TierValidationError, ValidatedTiers};

/// Display order of the tier rows. Presentation only: rows map to entries by tier name.
const DISPLAY_ORDER: [&str; 7] = ["S", "A", "B", "C", "D", "E", "F"];

/// Shown beside a tier row whose text is not a finite number >= 0.
const ENTER_A_NUMBER: &str = "Enter a number";

/// Tooltip on a disabled Apply button while tier errors exist.
pub const APPLY_BLOCKED_TOOLTIP: &str = "Fix the tier errors first.";

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
}

fn file_label(path: &Path) -> String {
    path.file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| path.display().to_string())
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
        let now = self.styles.get(name);
        StyleBadges {
            builtin: StyleSet::is_builtin(name),
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
        added + deleted + edited + self.tiers_dirty_count() + usize::from(self.reordered())
    }

    /// Tier rows whose value or text differs from the applied snapshot.
    fn tiers_dirty_count(&self) -> usize {
        self.tiers
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
            .count()
    }

    /// Re-read every config file the user has not edited whose bytes on disk differ from its
    /// baseline, so a clean file follows the disk instead of being guarded or written back. A file
    /// with pending edits keeps its baseline (changed-on-disk refusal, OVERWRITE). On error (a
    /// clean file that no longer loads) nothing changes. Returns whether anything was re-read.
    pub fn follow_disk(&mut self) -> Result<bool, String> {
        let paths = InputPaths::relative_to(&self.root);
        let read = |p: &Path| fs::read(p).map_err(|e| format!("{}: {e}", file_label(p)));
        let mut styles = None;
        if !self.styles_changed() {
            let now = read(&paths.styles)?;
            if now != self.styles_bytes {
                let set: StyleSet = serde_json::from_slice(&now)
                    .map_err(|e| format!("{}: {e}", file_label(&paths.styles)))?;
                set.validate()
                    .map_err(|e| format!("{}: {e}", file_label(&paths.styles)))?;
                styles = Some((set, now));
            }
        }
        let mut tiers = None;
        if self.tiers_dirty_count() == 0 {
            let now = read(&paths.tiers)?;
            if now != self.tiers_bytes {
                let entries: Vec<TierEntry> = serde_json::from_slice(&now)
                    .map_err(|e| format!("{}: {e}", file_label(&paths.tiers)))?;
                ValidatedTiers::try_from(entries.clone())
                    .map_err(|e| format!("{}: {e}", file_label(&paths.tiers)))?;
                tiers = Some((entries, now));
            }
        }
        let changed = styles.is_some() || tiers.is_some();
        if let Some((set, bytes)) = styles {
            self.styles_applied = set.clone();
            self.styles = set;
            self.styles_bytes = bytes;
        }
        if let Some((entries, bytes)) = tiers {
            self.tiers_applied = entries.clone();
            self.tiers = entries;
            self.tiers_bytes = bytes;
            self.resync_buffers();
        }
        Ok(changed)
    }

    /// `StyleSet` equality ignores order, so a pure reorder needs its own check: do the names
    /// present in both snapshots appear in a different relative order?
    fn reordered(&self) -> bool {
        let common = |a: &StyleSet, b: &StyleSet| -> Vec<String> {
            a.names()
                .filter(|n| b.contains(n))
                .map(str::to_string)
                .collect()
        };
        common(&self.styles, &self.styles_applied) != common(&self.styles_applied, &self.styles)
    }

    /// The styles differ from the applied snapshot in value, membership or order.
    fn styles_changed(&self) -> bool {
        self.styles != self.styles_applied || !self.styles.names().eq(self.styles_applied.names())
    }

    /// Any unapplied CONFIG edit (styles, tiers or style order).
    pub fn is_dirty(&self) -> bool {
        self.dirty_count() > 0
    }

    /// Move a custom style up or down among the customs. False (no change) for built-ins,
    /// unknown names and the ends of the custom group.
    pub fn move_style(&mut self, name: &str, up: bool) -> bool {
        self.styles.move_custom(name, up)
    }

    /// The working styles, including edits not yet applied.
    pub fn pending_styles(&self) -> &StyleSet {
        &self.styles
    }

    /// The working tier entries (file order), including edits not yet applied.
    pub fn pending_tiers(&self) -> &[TierEntry] {
        &self.tiers
    }

    pub fn styles_path(&self) -> PathBuf {
        InputPaths::relative_to(&self.root).styles
    }

    pub fn tiers_path(&self) -> PathBuf {
        InputPaths::relative_to(&self.root).tiers
    }

    /// The bytes read at load (or written by the last apply): the changed-on-disk baselines.
    pub fn baselines(&self) -> (&[u8], &[u8]) {
        (&self.styles_bytes, &self.tiers_bytes)
    }

    pub fn can_apply(&self) -> bool {
        self.dirty_count() > 0
            && !self.has_tier_errors()
            && self.tiers_error().is_none()
            && self.styles_error().is_none()
    }

    /// Everything that would stop the working state from being written, without writing.
    pub fn validate(&self) -> Result<(), ConfigApplyError> {
        if self.has_tier_errors() || self.tiers_error().is_some() || self.styles_error().is_some() {
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
        Ok(())
    }

    /// Validate and render the bytes both files should hold, without writing. A file whose
    /// working value equals its applied snapshot keeps its loaded bytes (no reformatting), so the
    /// all-or-none writer drops it.
    pub fn prepare_write(&self) -> Result<ConfigWrite, ConfigApplyError> {
        self.prepare_write_inner(false)
    }

    fn prepare_write_inner(&self, force: bool) -> Result<ConfigWrite, ConfigApplyError> {
        self.validate()?;
        let styles_changed = force || self.styles_changed();
        let tiers_changed = force || !tiers_equal(&self.tiers, &self.tiers_applied);
        let styles_new = if styles_changed {
            crate::loader::render_styles(&self.styles).into_bytes()
        } else {
            self.styles_bytes.clone()
        };
        let tiers_new = if tiers_changed {
            crate::loader::render_tiers(&self.tiers).into_bytes()
        } else {
            self.tiers_bytes.clone()
        };
        Ok(ConfigWrite {
            styles_new,
            tiers_new,
            styles_changed,
            tiers_changed,
        })
    }

    /// Adopt `w` as the new applied state after its bytes were written to both files by the
    /// caller: snapshots and baselines move, tier buffers are rebuilt.
    pub fn adopt(&mut self, w: ConfigWrite) {
        self.styles_applied = self.styles.clone();
        self.tiers_applied = self.tiers.clone();
        self.styles_bytes = w.styles_new;
        self.tiers_bytes = w.tiers_new;
        self.resync_buffers();
    }

    /// Discard in-memory edits back to the last-applied snapshot.
    pub fn revert(&mut self) {
        self.styles = self.styles_applied.clone();
        self.tiers = self.tiers_applied.clone();
        self.resync_buffers();
    }
}

/// Why the CONFIG edits cannot be written: the finished reason.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConfigApplyError {
    Invalid(String),
}

impl ConfigApplyError {
    pub fn message(&self) -> String {
        match self {
            ConfigApplyError::Invalid(reason) => not_applied_message(reason),
        }
    }
}

/// The validated, rendered bytes of the two config files, ready to be written by the caller and
/// then handed back to [`ConfigState::adopt`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfigWrite {
    pub styles_new: Vec<u8>,
    pub tiers_new: Vec<u8>,
    styles_changed: bool,
    tiers_changed: bool,
}

impl ConfigWrite {
    /// The styles differ from the applied snapshot (value, membership or order).
    pub fn styles_changed(&self) -> bool {
        self.styles_changed
    }

    /// The tier cutoffs differ from the applied snapshot.
    pub fn tiers_changed(&self) -> bool {
        self.tiers_changed
    }
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

pub fn not_applied_message(reason: &str) -> String {
    format!(
        "Config not applied: {}. Nothing was changed on disk.",
        reason.trim_end_matches('.')
    )
}

/// A cutoff is a finite number >= 0.
fn parse_cutoff(text: &str) -> Option<f64> {
    let v = text.trim().parse::<f64>().ok()?;
    (v.is_finite() && v >= 0.0).then_some(v)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::safe_write::{Target, write_all_or_none};

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
        assert_eq!(s.dirty_count(), 1);
        assert!(s.edit_style("S", StyleEdit::Font(Some(37))));
        assert_eq!(s.dirty_count(), 0);
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

    /// Write the prepared bytes with the real all-or-none writer, then adopt them: what the
    /// combined Apply does for the two config files.
    fn save(s: &mut ConfigState) -> ConfigWrite {
        let w = s.prepare_write().unwrap();
        let (sp, tp) = (s.styles_path(), s.tiers_path());
        let (sb, tb) = (s.baselines().0.to_vec(), s.baselines().1.to_vec());
        write_all_or_none(&[
            Target {
                path: &sp,
                loaded: Some(&sb),
                new: &w.styles_new,
            },
            Target {
                path: &tp,
                loaded: Some(&tb),
                new: &w.tiers_new,
            },
        ])
        .unwrap();
        s.adopt(w.clone());
        w
    }

    #[test]
    fn save_writes_only_the_two_files() {
        let dir = full_root();
        let mut s = ConfigState::load(dir.path()).unwrap();
        let styles_before = fs::read(dir.path().join("styles.json")).unwrap();
        let others: Vec<_> = EXTRA
            .iter()
            .map(|f| (f, fs::read(dir.path().join(f)).unwrap()))
            .collect();
        let names_before = listing(dir.path());

        s.set_tier_text("S", "35");
        let w = save(&mut s);
        assert!(w.tiers_changed() && !w.styles_changed());
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
    fn save_new_style_writes_it_last() {
        let (dir, mut s) = loaded();
        s.add_style("Gold").unwrap();
        let w = save(&mut s);
        assert!(w.styles_changed());
        let text = fs::read_to_string(dir.path().join("styles.json")).unwrap();
        let lines: Vec<_> = text.lines().collect();
        assert!(lines[lines.len() - 2].trim_start().starts_with("\"Gold\""));
        assert_eq!(
            ConfigState::load(dir.path()).unwrap().style_names().len(),
            13
        );
    }

    #[test]
    fn prepare_write_refuses_invalid_edits_and_writes_nothing() {
        let (dir, mut s) = loaded();
        let before = fs::read(dir.path().join("tiers.json")).unwrap();
        s.set_tier_text("S", "5");
        let e = s.prepare_write().unwrap_err();
        assert!(matches!(e, ConfigApplyError::Invalid(_)));
        assert!(e.message().starts_with("Config not applied: "));
        assert_eq!(fs::read(dir.path().join("tiers.json")).unwrap(), before);
    }

    #[test]
    fn loading_again_discards_edits_and_takes_outside_change() {
        let (dir, mut s) = loaded();
        s.set_tier_text("S", "35");
        s.add_style("Gold").unwrap();
        let tiers_path = dir.path().join("tiers.json");
        let text = fs::read_to_string(&tiers_path)
            .unwrap()
            .replace("30.0", "31.0");
        fs::write(&tiers_path, text).unwrap();
        s = ConfigState::load(dir.path()).unwrap();
        assert_eq!(s.dirty_count(), 0);
        assert_eq!(s.tier_rows()[0].buffer, "31");
        assert_eq!(s.style_names().len(), 12);
    }

    #[test]
    fn revert_after_save_returns_to_saved_values() {
        let (_d, mut s) = loaded();
        s.set_tier_text("S", "35");
        save(&mut s);
        s.set_tier_text("S", "40");
        assert_eq!(s.dirty_count(), 1);
        s.revert();
        assert_eq!(s.tier_rows()[0].buffer, "35");
        assert_eq!(s.dirty_count(), 0);
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
        assert_eq!(
            not_applied_message("x"),
            "Config not applied: x. Nothing was changed on disk."
        );
    }

    /// A temp root whose styles.json is canonical `render_styles` output, with two customs added
    /// so there is something to reorder.
    fn reorder_root() -> (tempfile::TempDir, ConfigState, Vec<String>) {
        let dir = temp_root();
        let mut set: StyleSet =
            serde_json::from_slice(&fs::read(dir.path().join("styles.json")).unwrap()).unwrap();
        set.insert_new("Gold", Style::blank_show()).unwrap();
        set.insert_new("Silver", Style::blank_show()).unwrap();
        fs::write(
            dir.path().join("styles.json"),
            crate::loader::render_styles(&set),
        )
        .unwrap();
        let s = ConfigState::load(dir.path()).unwrap();
        let customs: Vec<String> = s
            .style_names()
            .into_iter()
            .filter(|n| !StyleSet::is_builtin(n))
            .collect();
        (dir, s, customs)
    }

    #[test]
    fn reorder_is_dirty_and_reverts() {
        let (_dir, mut s, customs) = reorder_root();
        let before = s.style_names();
        assert!(!s.is_dirty());
        let last = customs.last().unwrap();
        assert!(s.move_style(last, true));
        assert!(s.dirty_count() >= 1);
        assert!(s.is_dirty());
        assert!(s.can_apply());
        assert_ne!(s.style_names(), before);
        s.revert();
        assert_eq!(s.style_names(), before);
        assert_eq!(s.dirty_count(), 0);
        assert!(!s.move_style("S", true));
    }

    #[test]
    fn reorder_persists_in_file_order() {
        let (dir, mut s, customs) = reorder_root();
        let path = dir.path().join("styles.json");
        let old = fs::read_to_string(&path).unwrap();
        let last = customs.last().unwrap().clone();
        assert!(s.move_style(&last, true));
        let expect = s.style_names();

        let w = s.prepare_write().unwrap();
        assert!(w.styles_changed());
        assert!(!w.tiers_changed());
        assert_eq!(w.tiers_new, s.baselines().1);
        let targets = [
            Target {
                path: &s.styles_path(),
                loaded: Some(s.baselines().0),
                new: &w.styles_new,
            },
            Target {
                path: &s.tiers_path(),
                loaded: Some(s.baselines().1),
                new: &w.tiers_new,
            },
        ];
        write_all_or_none(&targets).unwrap();
        s.adopt(w);
        assert_eq!(s.dirty_count(), 0);

        let new = fs::read_to_string(&path).unwrap();
        for line in old.lines() {
            let line = line.trim_end_matches(',');
            assert!(new.contains(line), "lost line: {line}");
        }
        let reloaded = ConfigState::load(dir.path()).unwrap();
        assert_eq!(reloaded.style_names(), expect);
        assert_eq!(reloaded.dirty_count(), 0);
    }

    #[test]
    fn prepare_write_clean_keeps_loaded_bytes() {
        let (_dir, mut s) = loaded();
        let w = s.prepare_write().unwrap();
        let (sb, tb) = s.baselines();
        assert_eq!(w.styles_new, sb);
        assert_eq!(w.tiers_new, tb);
        assert!(!w.styles_changed() && !w.tiers_changed());

        s.set_tier_text("S", "abc");
        assert!(matches!(
            s.prepare_write(),
            Err(ConfigApplyError::Invalid(_))
        ));
    }

    #[test]
    fn follow_disk_adopts_only_clean_files() {
        let (dir, mut s) = loaded();
        assert_eq!(s.follow_disk(), Ok(false), "nothing changed on disk");

        // A clean tiers.json follows an outside edit; an edited styles.json keeps its baseline.
        assert!(s.edit_style("S", StyleEdit::Font(Some(41))));
        let styles_base = s.baselines().0.to_vec();
        let mut styles_disk = styles_base.clone();
        styles_disk.push(b'\n');
        fs::write(s.styles_path(), &styles_disk).unwrap();
        let tiers: Vec<TierEntry> =
            serde_json::from_slice(&fs::read(s.tiers_path()).unwrap()).unwrap();
        let mut outside = tiers.clone();
        outside.iter_mut().find(|e| e.tier == "S").unwrap().min = 31.0;
        let tiers_disk = crate::loader::render_tiers(&outside).into_bytes();
        fs::write(s.tiers_path(), &tiers_disk).unwrap();

        assert_eq!(s.follow_disk(), Ok(true));
        assert_eq!(
            s.baselines().0,
            &styles_base[..],
            "edited file keeps its baseline"
        );
        assert_eq!(s.baselines().1, &tiers_disk[..]);
        assert_eq!(
            s.pending_tiers()
                .iter()
                .find(|e| e.tier == "S")
                .unwrap()
                .min,
            31.0
        );
        assert_eq!(s.style("S").unwrap().font, Some(41), "edits survive");
        assert_eq!(s.dirty_count(), 1);

        // A clean file that no longer loads is an error and changes nothing.
        fs::write(dir.path().join("tiers.json"), b"not json").unwrap();
        assert!(s.follow_disk().unwrap_err().starts_with("tiers.json"));
        assert_eq!(s.baselines().1, &tiers_disk[..]);
    }
}
