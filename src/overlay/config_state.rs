//! Pure model of the CONFIG tab: working and last-applied snapshots of `styles.json` and
//! `tiers.json`, tier text buffers, dirty counting and validation.
//!
//! Layer rule (same as `overlay/apply.rs`): no GUI-framework types live here, so every CONF-02/03/04
//! rule is provable with temp dirs -- `OverlayApp::new` needs a real tray and cannot be unit-tested.
//! File paths come only from `InputPaths::relative_to(root)`. This module writes only `styles.json`
//! and `tiers.json`; the committed `prices-fallback.json` / `tiers-fallback.json` pair is never
//! touched.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use crate::compile::InputPaths;
use crate::models::config::{Beam, Minimap, Sound, Style, StyleAction, StyleNameError, StyleSet};
use crate::models::tiers::{TierEntry, TierValidationError, ValidatedTiers};

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

    #[allow(dead_code)]
    fn paths(&self) -> InputPaths {
        InputPaths::relative_to(&self.root)
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
}
