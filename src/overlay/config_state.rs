//! Pure model of the CONFIG tab: working and last-applied snapshots of `styles.json` and
//! `tiers.json`, tier text buffers, dirty counting and validation.
//!
//! Layer rule (same as `overlay/apply.rs`): no GUI-framework types live here, so every CONF-02/03/04
//! rule is provable with temp dirs -- `OverlayApp::new` needs a real tray and cannot be unit-tested.
//! File paths come only from `InputPaths::in_home(root)`. Reads go through
//! `InputPaths::styles_source` / `tiers_source` (the live file, else the built-in default);
//! this module only ever hands out the live `styles.json` / `tiers.json` as write targets, so the
//! `-default.json` files and the early-league fallback pair are never touched.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

use crate::compile::{InputPaths, Source};
use crate::loader::LoadError;
use crate::models::config::{
    Beam, Cancelable, Config, DropSound, Minimap, Sound, Style, StyleAction, StyleNameError,
    StyleSet,
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
    /// `None` = off (no line), `Cancel` = the filter's cancel value.
    Sound(Option<Cancelable<Sound>>),
    Minimap(Option<Cancelable<Minimap>>),
    Beam(Option<Cancelable<Beam>>),
    DropSound(Option<DropSound>),
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
    /// Bytes of the source actually read (live file or default): the content baselines.
    styles_bytes: Vec<u8>,
    tiers_bytes: Vec<u8>,
    /// Bytes of the LIVE file at load / last follow / last write; `None` = it did not exist. The
    /// changed-on-disk baselines (the guard never looks at the `-default.json` files).
    styles_live: Option<Vec<u8>>,
    tiers_live: Option<Vec<u8>>,
    /// Ids were assigned at load (the live file on disk lacks them): the next Apply writes
    /// styles.json even with no CONFIG edit. Never counts as an edit. Always false while the
    /// source is the `-default.json` (never written; its ids derive from the names anyway).
    ids_unpersisted: bool,
}

/// Read `path`, `None` when it does not exist. Errors read `<full path>: <reason>`.
fn read_optional(path: &Path) -> Result<Option<Vec<u8>>, String> {
    match fs::read(path) {
        Ok(b) => Ok(Some(b)),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(format!("{}: {e}", path.display())),
    }
}

/// One read of the effective source of a live file: `(source, content bytes, live bytes)`.
///
/// The live file is read once and, when present, supplies both the content and the guard
/// baseline, so the two can never disagree (WR-01). Only when it is absent is the source resolved
/// (normally the built-in default, which has no live bytes) and read; should the live file appear
/// in between, the resolver picks it and that single read again feeds both baselines.
fn read_effective(
    live: &Path,
    resolve: impl FnOnce() -> Result<Source, LoadError>,
) -> Result<(Source, Vec<u8>, Option<Vec<u8>>), String> {
    if let Some(bytes) = read_optional(live)? {
        return Ok((Source::File(live.to_path_buf()), bytes.clone(), Some(bytes)));
    }
    let src = resolve().map_err(|e| format!("{}: {e}", live.display()))?;
    let bytes = src.bytes().map_err(|e| format!("{}: {e}", src.label()))?;
    let live_bytes = (src.as_file() == Some(live)).then(|| bytes.clone());
    Ok((src, bytes, live_bytes))
}

fn tiers_equal(a: &[TierEntry], b: &[TierEntry]) -> bool {
    a.len() == b.len()
        && a.iter()
            .zip(b)
            .all(|(x, y)| x.tier == y.tier && x.min == y.min && x.unit == y.unit)
}

impl ConfigState {
    /// Read `styles.json` and `tiers.json` under `root`, each falling back to its
    /// `-default.json` when absent (DEF-03). Errors read `<file name>: <reason>`. Never writes.
    pub fn load(root: &Path) -> Result<ConfigState, String> {
        let paths = InputPaths::in_home(root);

        let (styles_src, styles_bytes, styles_live) =
            read_effective(&paths.styles, || paths.styles_source())?;
        let mut styles: StyleSet = serde_json::from_slice(&styles_bytes)
            .map_err(|e| format!("{}: {e}", styles_src.label()))?;
        // Ids assigned to a default source are derived from the names, and the default is never
        // written: persisting them would create a live styles.json nobody edited (WR-02).
        let ids_unpersisted = styles.assign_missing_ids() && styles_live.is_some();
        styles
            .validate()
            .map_err(|e| format!("{}: {e}", styles_src.label()))?;

        let (tiers_src, tiers_bytes, tiers_live) =
            read_effective(&paths.tiers, || paths.tiers_source())?;
        let tiers: Vec<TierEntry> = serde_json::from_slice(&tiers_bytes)
            .map_err(|e| format!("{}: {e}", tiers_src.label()))?;
        ValidatedTiers::try_from(tiers.clone())
            .map_err(|e| format!("{}: {e}", tiers_src.label()))?;

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
            styles_live,
            tiers_live,
            ids_unpersisted,
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
            StyleEdit::DropSound(d) => style.set_drop_sound(d),
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
        self.styles_dirty_count() + self.tiers_dirty_count()
    }

    /// Styles added + deleted + changed (matched by id, name for an id-less style, so a rename is
    /// one edit) + one for a pure reorder.
    fn styles_dirty_count(&self) -> usize {
        let key = |n: &str, s: &Style| s.id.clone().unwrap_or_else(|| n.to_string());
        let now: Vec<(String, &str, &Style)> =
            self.styles.iter().map(|(n, s)| (key(n, s), n, s)).collect();
        let was: Vec<(String, &str, &Style)> = self
            .styles_applied
            .iter()
            .map(|(n, s)| (key(n, s), n, s))
            .collect();
        let added = now
            .iter()
            .filter(|(k, ..)| !was.iter().any(|(w, ..)| w == k))
            .count();
        let deleted = was
            .iter()
            .filter(|(k, ..)| !now.iter().any(|(n, ..)| n == k))
            .count();
        let changed = now
            .iter()
            .filter(|(k, n, s)| {
                was.iter()
                    .find(|(w, ..)| w == k)
                    .is_some_and(|(_, wn, ws)| wn != n || ws != s)
            })
            .count();
        let common = |a: &[(String, &str, &Style)], b: &[(String, &str, &Style)]| -> Vec<String> {
            a.iter()
                .filter(|(k, ..)| b.iter().any(|(o, ..)| o == k))
                .map(|(k, ..)| k.clone())
                .collect()
        };
        let reordered = common(&now, &was) != common(&was, &now);
        added + deleted + changed + usize::from(reordered)
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
    /// baseline, so a clean file follows the disk instead of being guarded or written back. The
    /// source switches with the live file: creating `styles.json` makes it the source, deleting it
    /// falls back to `styles-default.json` (same for tiers). A file with pending edits keeps both
    /// baselines (changed-on-disk refusal, OVERWRITE). On error (a clean file that no longer
    /// loads) nothing changes. Never writes. Returns whether any content was re-read.
    pub fn follow_disk(&mut self) -> Result<bool, String> {
        let paths = InputPaths::in_home(&self.root);
        let mut styles = None;
        let mut styles_live = None;
        if !self.styles_changed() {
            let (src, now, live) = read_effective(&paths.styles, || paths.styles_source())?;
            if now != self.styles_bytes {
                let mut set: StyleSet =
                    serde_json::from_slice(&now).map_err(|e| format!("{}: {e}", src.label()))?;
                let assigned = set.assign_missing_ids();
                set.validate()
                    .map_err(|e| format!("{}: {e}", src.label()))?;
                styles = Some((set, now, assigned));
            }
            if live != self.styles_live {
                styles_live = Some(live);
            }
        }
        let mut tiers = None;
        let mut tiers_live = None;
        if self.tiers_dirty_count() == 0 {
            let (src, now, live) = read_effective(&paths.tiers, || paths.tiers_source())?;
            if now != self.tiers_bytes {
                let entries: Vec<TierEntry> =
                    serde_json::from_slice(&now).map_err(|e| format!("{}: {e}", src.label()))?;
                ValidatedTiers::try_from(entries.clone())
                    .map_err(|e| format!("{}: {e}", src.label()))?;
                tiers = Some((entries, now));
            }
            if live != self.tiers_live {
                tiers_live = Some(live);
            }
        }
        let changed = styles.is_some() || tiers.is_some();
        // A source switch with identical content still moves the live (guard) baseline.
        if let Some(live) = styles_live {
            self.styles_live = live;
        }
        if let Some(live) = tiers_live {
            self.tiers_live = live;
        }
        if let Some((set, bytes, assigned)) = styles {
            // The disk file is the new baseline, so the flag describes it alone; a default
            // source is never persisted (WR-02).
            self.ids_unpersisted = assigned && self.styles_live.is_some();
            self.styles_applied = set.clone();
            self.styles = set;
            self.styles_bytes = bytes;
        }
        if self.styles_live.is_none() {
            // Back on the default (live file deleted): nothing of it is ever written.
            self.ids_unpersisted = false;
        }
        if let Some((entries, bytes)) = tiers {
            self.tiers_applied = entries.clone();
            self.tiers = entries;
            self.tiers_bytes = bytes;
            self.resync_buffers();
        }
        Ok(changed)
    }

    /// The styles differ from the applied snapshot in value, name, membership or order.
    fn styles_changed(&self) -> bool {
        self.styles_dirty_count() > 0
    }

    /// Rename a custom style in place (same position, same id): an in-memory edit like any other,
    /// Revert-able. `new` is trimmed like [`Self::add_style`].
    pub fn rename_style(&mut self, old: &str, new: &str) -> Result<(), StyleNameError> {
        self.styles.rename_custom(old, new.trim())
    }

    /// The stable id of the style called `name`.
    pub fn style_id(&self, name: &str) -> Option<&str> {
        self.styles.id_of(name)
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

    /// The styles as last loaded or applied (what is on disk).
    pub fn applied_styles(&self) -> &StyleSet {
        &self.styles_applied
    }

    /// The working tier entries (file order), including edits not yet applied.
    pub fn pending_tiers(&self) -> &[TierEntry] {
        &self.tiers
    }

    /// The live `styles.json`: the write target. Reads may come from `styles-default.json`.
    pub fn styles_path(&self) -> PathBuf {
        InputPaths::in_home(&self.root).styles
    }

    /// The live `tiers.json`: the write target. Reads may come from `tiers-default.json`.
    pub fn tiers_path(&self) -> PathBuf {
        InputPaths::in_home(&self.root).tiers
    }

    /// The content bytes read at load (or written by the last apply), from whichever source was
    /// read: what an unedited file keeps.
    pub fn baselines(&self) -> (&[u8], &[u8]) {
        (&self.styles_bytes, &self.tiers_bytes)
    }

    /// The live files' bytes at load / last follow / last write, `None` when the live file did
    /// not exist: the changed-on-disk guard baselines (DEF-03).
    pub fn live_baselines(&self) -> (Option<&[u8]>, Option<&[u8]>) {
        (self.styles_live.as_deref(), self.tiers_live.as_deref())
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
        let styles_changed = force || self.styles_changed() || self.ids_unpersisted;
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
        if w.styles_changed {
            self.ids_unpersisted = false;
            self.styles_live = Some(w.styles_new.clone());
        }
        if w.tiers_changed {
            self.tiers_live = Some(w.tiers_new.clone());
        }
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

    /// Discard one tab's edits only: styles (CONFIG) or tier cutoffs (ECONOMY).
    pub fn revert_scope(&mut self, scope: RevertScope) {
        match scope {
            RevertScope::Styles => self.styles = self.styles_applied.clone(),
            RevertScope::Tiers => {
                self.tiers = self.tiers_applied.clone();
                self.resync_buffers();
            }
        }
    }

    /// Whether any style differs from the applied snapshot.
    pub fn styles_dirty(&self) -> bool {
        self.styles_dirty_count() > 0
    }

    /// Whether any tier cutoff differs from the applied snapshot.
    pub fn tiers_dirty(&self) -> bool {
        self.tiers_dirty_count() > 0
    }

    /// Edit count of one scope.
    pub fn scope_dirty_count(&self, scope: RevertScope) -> usize {
        match scope {
            RevertScope::Styles => self.styles_dirty_count(),
            RevertScope::Tiers => self.tiers_dirty_count(),
        }
    }
}

/// Which tab's CONFIG edits a Revert discards: Styles (CONFIG tab) or Tiers (ECONOMY tab).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum RevertScope {
    #[default]
    Styles,
    Tiers,
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
        for (src, dst) in [
            ("tests/fixtures/seed-styles.json", "styles.json"),
            ("tiers-default.json", "tiers.json"),
        ] {
            fs::copy(
                Path::new(env!("CARGO_MANIFEST_DIR")).join(src),
                dir.path().join(dst),
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
    fn scoped_dirty_and_revert_keep_the_other_scope() {
        let (_d, mut s) = loaded();
        s.set_tier_text("S", "31");
        assert!(s.tiers_dirty() && !s.styles_dirty());
        assert_eq!(s.scope_dirty_count(RevertScope::Tiers), 1);
        assert_eq!(s.scope_dirty_count(RevertScope::Styles), 0);

        assert!(s.edit_style("S", StyleEdit::Font(Some(44))));
        assert!(s.styles_dirty());
        // Reverting styles keeps the tier edit.
        s.revert_scope(RevertScope::Styles);
        assert!(!s.styles_dirty() && s.tiers_dirty());
        // An invalid tier buffer and its error survive a styles revert.
        s.set_tier_text("A", "abc");
        assert!(s.has_tier_errors());
        s.revert_scope(RevertScope::Styles);
        assert!(s.has_tier_errors());

        // Reverting tiers clears tier edits and errors but keeps a style edit.
        assert!(s.edit_style("S", StyleEdit::Font(Some(44))));
        s.revert_scope(RevertScope::Tiers);
        assert!(!s.tiers_dirty() && !s.has_tier_errors());
        assert!(s.styles_dirty());
    }

    #[test]
    fn load_errors_name_the_full_path() {
        let d = temp_root();
        fs::write(d.path().join("styles.json"), "{ not json").unwrap();
        let e = ConfigState::load(d.path()).err().unwrap();
        let live = d.path().join("styles.json");
        assert!(e.starts_with(&format!("{}:", live.display())), "{e}");

        let d = temp_root();
        fs::write(d.path().join("tiers.json"), "[ not json").unwrap();
        let e = ConfigState::load(d.path()).err().unwrap();
        let live = d.path().join("tiers.json");
        assert!(e.starts_with(&format!("{}:", live.display())), "{e}");
    }

    #[test]
    fn empty_folder_loads_the_built_in_defaults_and_writes_nothing() {
        let d = tempfile::tempdir().unwrap();
        let s = ConfigState::load(d.path()).unwrap();
        let names = s.style_names();
        for n in ["S", "A", "B", "C", "D", "E", "F", "unchanged", "hidden"] {
            assert!(names.iter().any(|x| x == n), "missing {n}");
        }
        assert_eq!(s.pending_tiers().len(), 7);
        assert_eq!(s.dirty_count(), 0);
        assert!(!s.is_dirty());
        assert_eq!(s.live_baselines(), (None, None));
        let w = s.prepare_write().unwrap();
        assert!(!w.styles_changed() && !w.tiers_changed());
        assert_eq!(fs::read_dir(d.path()).unwrap().count(), 0);
        assert_eq!(s.styles_path(), d.path().join("styles.json"));
        assert_eq!(s.tiers_path(), d.path().join("tiers.json"));
    }

    /// A temp root holding only the committed defaults under their own names (no live file).
    fn defaults_root() -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        for f in ["styles-default.json", "tiers-default.json"] {
            fs::copy(
                Path::new(env!("CARGO_MANIFEST_DIR")).join(f),
                dir.path().join(f),
            )
            .unwrap();
        }
        dir
    }

    fn assert_no_live(dir: &Path) {
        assert!(!dir.join("styles.json").exists(), "styles.json created");
        assert!(!dir.join("tiers.json").exists(), "tiers.json created");
    }

    #[test]
    fn load_falls_back_to_defaults_and_creates_nothing() {
        let d = defaults_root();
        let s = ConfigState::load(d.path()).unwrap();
        assert_eq!(s.style_names().len(), 12);
        assert_eq!(
            s.baselines().0,
            &fs::read(d.path().join("styles-default.json")).unwrap()[..]
        );
        assert_eq!(
            s.baselines().1,
            &fs::read(d.path().join("tiers-default.json")).unwrap()[..]
        );
        assert_eq!(s.live_baselines(), (None, None));
        assert_eq!(s.dirty_count(), 0);
        // The defaults carry ids: nothing to persist, so a clean Apply writes nothing.
        let w = s.prepare_write().unwrap();
        assert!(!w.styles_changed() && !w.tiers_changed());
        assert_no_live(d.path());
    }

    #[test]
    fn load_prefers_the_live_file() {
        let d = defaults_root();
        let mut v: serde_json::Value =
            serde_json::from_slice(&fs::read(d.path().join("styles-default.json")).unwrap())
                .unwrap();
        v["S"]["font"] = serde_json::json!(44);
        let live = serde_json::to_vec_pretty(&v).unwrap();
        fs::write(d.path().join("styles.json"), &live).unwrap();
        let s = ConfigState::load(d.path()).unwrap();
        assert_eq!(s.baselines().0, &live[..]);
        assert_eq!(s.live_baselines().0, Some(&live[..]));
        assert_eq!(s.live_baselines().1, None);
        assert!(!d.path().join("tiers.json").exists());
    }

    #[test]
    fn default_files_in_the_folder_are_ignored() {
        let d = defaults_root();
        fs::remove_file(d.path().join("styles-default.json")).unwrap();
        fs::remove_file(d.path().join("tiers-default.json")).unwrap();
        let s = ConfigState::load(d.path()).unwrap();
        assert_eq!(s.style_names().len(), 12);
        assert_no_live(d.path());
    }

    #[test]
    fn follow_disk_switches_source_on_live_create_and_delete() {
        let d = defaults_root();
        let mut s = ConfigState::load(d.path()).unwrap();
        assert_eq!(s.follow_disk(), Ok(false));
        assert_no_live(d.path());

        // Creating a live styles.json (different content) switches to it.
        let mut v: serde_json::Value =
            serde_json::from_slice(&fs::read(d.path().join("styles-default.json")).unwrap())
                .unwrap();
        v["S"]["font"] = serde_json::json!(44);
        let live = serde_json::to_vec_pretty(&v).unwrap();
        fs::write(d.path().join("styles.json"), &live).unwrap();
        assert_eq!(s.follow_disk(), Ok(true));
        assert_eq!(s.style("S").unwrap().font, Some(44));
        assert_eq!(s.baselines().0, &live[..]);
        assert_eq!(s.live_baselines().0, Some(&live[..]));
        assert_eq!(s.dirty_count(), 0);

        // Deleting it switches back to the default.
        fs::remove_file(d.path().join("styles.json")).unwrap();
        assert_eq!(s.follow_disk(), Ok(true));
        assert_ne!(s.style("S").unwrap().font, Some(44));
        assert_eq!(s.live_baselines().0, None);
        assert_no_live(d.path());

        // A live tiers.json identical to the default: content unchanged, the guard baseline moves.
        let same = fs::read(d.path().join("tiers-default.json")).unwrap();
        fs::write(d.path().join("tiers.json"), &same).unwrap();
        assert_eq!(s.follow_disk(), Ok(false));
        assert_eq!(s.live_baselines().1, Some(&same[..]));
        fs::remove_file(d.path().join("tiers.json")).unwrap();
        assert_eq!(s.follow_disk(), Ok(false));
        assert_eq!(s.live_baselines().1, None);
        assert_no_live(d.path());
    }

    #[test]
    fn follow_disk_keeps_live_baseline_of_an_edited_file() {
        let d = defaults_root();
        let mut s = ConfigState::load(d.path()).unwrap();
        assert!(s.edit_style("S", StyleEdit::Font(Some(41))));
        fs::copy(
            d.path().join("styles-default.json"),
            d.path().join("styles.json"),
        )
        .unwrap();
        assert_eq!(s.follow_disk(), Ok(false));
        assert_eq!(s.live_baselines().0, None, "dirty file keeps 'absent'");
    }

    /// WR-01: the content and the guard baselines come from one read. A live file that exists
    /// supplies both; one created while the source is being resolved is picked as the source and
    /// still supplies both, so the guard can never hold bytes newer than the content.
    #[test]
    fn read_effective_takes_both_baselines_from_one_read() {
        let d = defaults_root();
        let live = d.path().join("styles.json");
        let default = d.path().join("styles-default.json");
        let paths = InputPaths::in_home(d.path());

        let (src, bytes, live_bytes) = read_effective(&live, || paths.styles_source()).unwrap();
        assert!(matches!(src, Source::Embedded(_)));
        assert_eq!(
            bytes,
            fs::read(&default).unwrap(),
            "built-in copy equals the committed default"
        );
        assert_eq!(live_bytes, None);
        assert_no_live(d.path());

        // The live file appears between the absent check and the source resolution.
        let created = b"{\"created\": true}".to_vec();
        let (src, bytes, live_bytes) = read_effective(&live, || {
            fs::write(&live, &created).unwrap();
            paths.styles_source()
        })
        .unwrap();
        assert_eq!(src, Source::File(live.clone()));
        assert_eq!(bytes, created);
        assert_eq!(live_bytes.as_deref(), Some(&created[..]));

        // Present from the start: both baselines are the live bytes.
        let (src, bytes, live_bytes) =
            read_effective(&live, || panic!("no resolution needed")).unwrap();
        assert_eq!(src, Source::File(live.clone()));
        assert_eq!(live_bytes.as_deref(), Some(&bytes[..]));
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
        assert!(s.edit_style("S", StyleEdit::DropSound(Some(DropSound::Silent))));
        assert!(s.style("S").unwrap().disable_drop_sound);
        assert!(s.edit_style("S", StyleEdit::DropSound(Some(DropSound::Normal))));
        assert_eq!(s.style("S").unwrap().drop_sound(), Some(DropSound::Normal));
        assert!(!s.edit_style("nope", StyleEdit::DropSound(Some(DropSound::Silent))));
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
        assert!(s.follow_disk().unwrap_err().contains("tiers.json"));
        assert_eq!(s.baselines().1, &tiers_disk[..]);
    }

    /// A temp root whose styles.json has no ids at all.
    fn idless_root() -> tempfile::TempDir {
        let dir = temp_root();
        let path = dir.path().join("styles.json");
        let mut v: serde_json::Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
        for (_, s) in v.as_object_mut().unwrap() {
            s.as_object_mut().unwrap().remove("id");
        }
        fs::write(&path, serde_json::to_vec_pretty(&v).unwrap()).unwrap();
        dir
    }

    #[test]
    fn load_assigned_ids_are_invisible_but_persisted_by_the_next_write() {
        let dir = idless_root();
        let mut s = ConfigState::load(dir.path()).unwrap();
        assert!(s.style_names().iter().all(|n| s.style_id(n).is_some()));
        assert_eq!(s.style_id("S"), Some("builtin:S"));
        assert!(!s.is_dirty());
        assert_eq!(s.dirty_count(), 0);
        assert!(!s.can_apply());
        let w = s.prepare_write().unwrap();
        assert!(w.styles_changed() && !w.tiers_changed());
        assert!(
            String::from_utf8(w.styles_new.clone())
                .unwrap()
                .contains("\"id\":")
        );
        let w = save(&mut s);
        assert!(w.styles_changed());
        assert!(!s.prepare_write().unwrap().styles_changed());
    }

    /// Strip every `"id"` from the styles file at `path`.
    fn strip_ids(path: &Path) {
        let mut v: serde_json::Value = serde_json::from_slice(&fs::read(path).unwrap()).unwrap();
        for (_, s) in v.as_object_mut().unwrap() {
            s.as_object_mut().unwrap().remove("id");
        }
        fs::write(path, serde_json::to_vec_pretty(&v).unwrap()).unwrap();
    }

    /// WR-02: ids assigned to an id-less DEFAULT source are never persisted, so a clean Apply
    /// does not create a live styles.json nobody edited. A real edit still does.
    #[test]
    fn idless_default_source_is_not_persisted_without_an_edit() {
        let d = defaults_root();
        strip_ids(&d.path().join("styles-default.json"));
        let mut s = ConfigState::load(d.path()).unwrap();
        assert_eq!(s.style_id("S"), Some("builtin:S"));
        let w = s.prepare_write().unwrap();
        assert!(!w.styles_changed() && !w.tiers_changed());
        assert_no_live(d.path());

        // An id-less LIVE file is persisted; deleting it puts the default back and the flag off.
        let live = d.path().join("styles.json");
        fs::copy(d.path().join("styles-default.json"), &live).unwrap();
        let mut v: serde_json::Value = serde_json::from_slice(&fs::read(&live).unwrap()).unwrap();
        v["S"]["font"] = serde_json::json!(36);
        fs::write(&live, serde_json::to_vec_pretty(&v).unwrap()).unwrap();
        assert_eq!(s.follow_disk(), Ok(true));
        assert!(s.prepare_write().unwrap().styles_changed());
        fs::remove_file(&live).unwrap();
        assert_eq!(s.follow_disk(), Ok(true));
        assert!(!s.prepare_write().unwrap().styles_changed());

        assert!(s.edit_style("S", StyleEdit::Font(Some(41))));
        assert!(s.prepare_write().unwrap().styles_changed());
    }

    /// The committed default carries every id, so loading it never assigns any.
    #[test]
    fn committed_styles_default_carries_every_id() {
        let mut set: StyleSet = serde_json::from_slice(
            &fs::read(
                Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/seed-styles.json"),
            )
            .unwrap(),
        )
        .unwrap();
        assert!(!set.assign_missing_ids());
    }

    #[test]
    fn styles_with_ids_are_not_rerendered() {
        let (_d, s) = loaded();
        let w = s.prepare_write().unwrap();
        assert!(!w.styles_changed());
        assert_eq!(w.styles_new, s.baselines().0);
    }

    #[test]
    fn follow_disk_assigns_the_same_ids_as_load() {
        let dir = idless_root();
        let mut s = ConfigState::load(dir.path()).unwrap();
        let before = s.style_id("kite").map(str::to_string);
        // Disk changes to another id-less file; the clean state follows it.
        let path = dir.path().join("styles.json");
        let mut v: serde_json::Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
        v["S"]["font"] = serde_json::json!(36);
        fs::write(&path, serde_json::to_vec_pretty(&v).unwrap()).unwrap();
        assert_eq!(s.follow_disk(), Ok(true));
        assert_eq!(s.style_id("kite").map(str::to_string), before);
        assert!(!s.is_dirty());
        assert!(s.prepare_write().unwrap().styles_changed());
    }

    #[test]
    fn rename_is_one_revertible_edit_keeping_id_and_position() {
        let (_d, mut s) = loaded();
        let id = s.style_id("kite").unwrap().to_string();
        let pos = s.style_names().iter().position(|n| n == "kite").unwrap();
        s.rename_style("kite", "locator").unwrap();
        assert_eq!(s.style_id("locator"), Some(id.as_str()));
        assert_eq!(
            s.style_names().iter().position(|n| n == "locator"),
            Some(pos)
        );
        assert_eq!(s.dirty_count(), 1);
        assert!(s.is_dirty() && s.can_apply());
        // renaming back clears the edit
        s.rename_style("locator", "kite").unwrap();
        assert!(!s.is_dirty());
        s.rename_style("kite", "locator").unwrap();
        s.revert();
        assert!(s.style("kite").is_some() && s.style("locator").is_none());
        assert_eq!(s.dirty_count(), 0);
    }

    #[test]
    fn rename_refusals() {
        let (_d, mut s) = loaded();
        assert_eq!(s.rename_style("S", "X"), Err(StyleNameError::BuiltIn));
        assert_eq!(s.rename_style("kite", ""), Err(StyleNameError::Empty));
        assert_eq!(s.rename_style("kite", "S"), Err(StyleNameError::Duplicate));
        assert_eq!(s.rename_style("kite", "a\tb"), Err(StyleNameError::Invalid));
        assert_eq!(s.rename_style("nope", "x"), Err(StyleNameError::Unknown));
        assert_eq!(s.dirty_count(), 0);
    }
}
