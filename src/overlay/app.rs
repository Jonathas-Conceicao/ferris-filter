//! `OverlayApp` -- the `eframe::App` implementation for the overlay window.
//!
//! What this file owns: docking the window to the right screen edge on the first frame where a
//! monitor size is known; polling the tray and the global hotkey for their events; turning those
//! events into an actual show/hide of the window that does not steal keyboard focus from the game,
//! by delegating the decision to [`super::visibility`] and the OS call to [`super::win`]; and the
//! filter-as-store footer (Phase 18).
//!
//! **Filter-as-store (Phase 18) and the rule list (Phase 19).** The game-folder `dynamic.filter`
//! is the store. On launch the overlay resolves the app root ([`crate::locations::AppDirs`]:
//! `FERRISFILTER_HOME`, else the per-user folder; no walk-up from the exe, and never the
//! working directory, so a double-click launch works) and loads the filter
//! ([`dynamic::LoadedFilter::load`], which never writes); RELOAD repeats both, and is offered
//! whenever the root, the load, or the economy markers are broken, so every such state can be
//! recovered without a restart.
//!
//! The rule list is back, now over parsed filter blocks: a searchable, virtualized list
//! ([`super::rule_list`]) over a [`WorkingFilter`] ([`super::rules`]). Every row operation edits
//! that in-memory document only. APPLY is the single writer: it writes the edited document through
//! [`dynamic::apply_edited`] (grammar gate, guarded atomic write) and is refused while an enabled
//! rule has a grammar error. REVERT re-parses the last applied text. The economy section is a
//! pinned footer row whose Add/Remove are staged and written by the next APPLY. When the guarded
//! write refuses because the file changed on disk, the window offers RELOAD, which re-reads the
//! file and discards unapplied edits, and OVERWRITE, which writes the edits over the outside change.
//!
//! The decisions (which button is live, what the status says) are pure functions in
//! [`super::apply`]; this file only draws them and performs the calls.
//! `PITFALLS.md` §8: if a bug can only be found by clicking, it belongs in a library module.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::time::Duration;

use global_hotkey::GlobalHotKeyManager;

use crate::compile::{InputPaths, PriceSourcePolicy, Source};
use crate::dynamic::{self, LoadedFilter};
use crate::filter::{BlockId, LinePatch};
use crate::loader;
use crate::models::config::StyleSet;
use crate::settings::{self, Settings};

use super::apply::{self, ApplyOutcome, ApplyState, FilterState};
use super::apply_all::{self, ApplyAllError, DiskFile, FilterInput};
use super::chat;
use super::classes::ClassCatalog;
use super::config_state::{self, ConfigState, RevertScope};
use super::config_view::{self, ConfigAction, ConfigUi, ConfigViewCtx};
use super::economy;
use super::hotkey::{self, HotkeyStatus};
use super::instance;
use super::leagues::ListState;
use super::rule_list::{self, ListUi};
use super::rules::{self, EconomyView, RowAction, WorkingFilter};
use super::shell::{self, ConfigBarInputs, Popup, PopupData, ShellTab, StatusInputs, StatusLine};
use super::tray::{self, OverlayCommand, TrayHandles};
use super::update_job::{InstallClick, UpdateJob, UpdateState};
use super::visibility::{self, Visibility};
use super::win::{self, OverlayWindow};
use crate::skin::Skin;
use crate::skin::fonts;
use crate::skin::palette as pal;
use crate::skin::widgets as w;
use egui::{Align, Layout, Rect, UiBuilder, pos2, vec2};

/// The eframe application state for the overlay window.
pub struct OverlayApp {
    /// Latches `true` once the first-frame docking has run, so it never runs
    /// again (position/width are not persisted or re-derived per frame).
    docked: bool,

    /// The `native_pixels_per_point` seen at the last dock; a different value later (the window
    /// crossed to a monitor of another scale) re-docks once so the width stays 660pt.
    dock_ppp: Option<f32>,

    /// The PoE2 skin: every texture, uploaded once at startup.
    skin: Skin,

    /// The debug screenshot state (`None` in a normal run; absent from release builds).
    #[cfg(debug_assertions)]
    shot: Option<ShotState>,

    /// What the ECONOMY tab shows about prices.json; re-read on switching to ECONOMY, at start
    /// and after a sync.
    economy_info: EconomyInfo,

    /// The tray icon, menu, and id->command mapping. Kept alive here until tray Quit, which
    /// removes the icon at once (`TrayHandles::release`) before it releases the instance name.
    tray: TrayHandles,

    /// The global hotkey manager, held so the hotkey stays registered. Tray Quit takes it and
    /// unregisters the hotkey (`hotkey::release_toggle`) before it releases the instance name,
    /// so a relaunch can register `Ctrl+Shift+Z` itself.
    hotkey_manager: Option<GlobalHotKeyManager>,

    /// Registration outcome for `Ctrl+Shift+Z`. Read every frame to filter incoming
    /// `GlobalHotKeyEvent`s to the one combo this app registered.
    hotkey_status: HotkeyStatus,

    /// The single-instance guard (INST-01); dropping it releases the instance name so the next
    /// `ff` launch becomes the running one. Polled once per frame for a show request from a
    /// second launch. Held until tray Quit, which drops it at once (`None`), not when eframe
    /// finally drops the app: a relaunch right after Quit then starts a new window instead of
    /// signalling a process that is going away (review iteration 2, WR-01).
    instance: Option<instance::Guard>,

    /// Set by the tray's left click, a `Ctrl+Shift+Z` press, or a tray "Show"
    /// while currently hidden. Consumed once per frame by [`visibility::advance`].
    toggle_requested: bool,

    /// The tray-Quit confirm ("Unapplied changes") is showing.
    quit_confirm: bool,
    /// The Quit confirm's only pending work is tier edits Fallback cannot write (refreshed each
    /// frame while it shows): it then offers no Apply & quit.
    quit_tier_edits_only: bool,

    /// The RULES Revert was clicked: the "Discard rule changes?" popup asks first.
    revert_rules_armed: bool,

    /// Apply & quit succeeded; the shutdown runs once the in-game reload worker has finished.
    quit_after_apply: bool,

    /// A quit request found nothing unapplied but came while an in-game reload was still being
    /// sent. Once that reload has finished the request is made again, so edits made meanwhile
    /// still get the Unapplied changes popup.
    quit_after_reload: bool,

    /// "Quit without applying" was chosen; the shutdown runs at the top of the next frame.
    quit_next_frame: bool,

    /// The show/hide decision -- see `super::visibility` for why this is three states, not two.
    visibility: Visibility,

    /// The overlay's own cached window handle (see `super::win`), populated
    /// on the first frame one is available. `None` on the very first few
    /// frames, and always `None` off Windows.
    window: Option<OverlayWindow>,

    /// The resolved FerrisFilter folder (styles.json, tiers.json, prices.json live here). `Err`
    /// holds the rendered message; the window shows it instead of panicking (STORE-05).
    root: Result<std::path::PathBuf, String>,

    /// The loaded game-folder filter. `Err` holds the rendered load failure.
    filter: Result<LoadedFilter, String>,

    /// The files found changed on disk (since they were loaded) by the last refused Apply;
    /// RELOAD or a successful OVERWRITE clears it.
    changed_on_disk: Vec<DiskFile>,

    /// Whether Overwrite is live this frame (the bar slot's decision), so the changed-on-disk
    /// popup offers it exactly when the bar does (review WR-03).
    overwrite_live: bool,

    /// The changed-on-disk popup is open. `changed_on_disk` stays non-empty until a reload or an
    /// overwrite, so "Keep editing" closes the popup through this flag alone; the bottom-bar
    /// Overwrite slot opens it again.
    changed_popup: bool,

    /// Whether the filter on disk matches what the last Apply wrote, as far as this session
    /// knows. A failed Apply leaves it exactly as it was.
    apply_state: ApplyState,

    /// The most recent Apply outcome's rendered text -- a success message naming the reload path,
    /// or an error's display string -- with the colour class it was produced with: a failure is
    /// always `Error`, whatever the apply state (review CR-02).
    apply_message: Option<(String, w::StatusKind)>,

    /// The economy-sync affordance: a `prices.json` refresh run on a worker thread so a window
    /// drawn over a live game never freezes on network I/O. It writes prices only -- the game sees
    /// them on the next **Apply**, which is what makes it safe to press mid-map.
    economy: economy::EconomySync,
    /// The About tab's update check; Ready survives message clears.
    update: UpdateJob,
    /// The update outcome shown in the status line (an event message).
    update_message: Option<(String, w::StatusKind)>,
    /// Where `begin_shutdown` leaves the verified installer for `launch()` to start once the
    /// event loop has returned (only after an explicit Install click).
    pending_install: std::sync::Arc<std::sync::Mutex<Option<PathBuf>>>,

    /// poe.ninja's league list for the Economy tab's picker: fetched on a worker the first time
    /// the tab is shown in a session and on refresh, never at start-up, in tests or in
    /// screenshot mode.
    leagues: super::leagues::LeagueList,

    /// The one-shot `/reloaditemfilter` worker. At most one in flight: one Apply click, one
    /// command.
    reload: chat::ReloadSend,
    /// The Apply whose reload result has not landed yet.
    reload_pending: Option<PendingReload>,

    /// The edited document (Some exactly when `filter` loaded). Every row operation edits this
    /// in memory; Apply is the single writer.
    working: Option<WorkingFilter>,

    /// View state of the rule list (expanded row, pending scroll, confirmations).
    list: ListUi,

    /// The search box contents.
    query: String,

    /// The latest row-operation status (refusals, revert, hidden add).
    list_status: Option<String>,

    /// The pending styles and tiers the economy preview (a staged add, or a pending refresh of the
    /// section in the file) was priced with, so it is re-priced only when they change (never per
    /// frame).
    staged_econ_inputs: Option<(
        StyleSet,
        Vec<crate::models::tiers::TierEntry>,
        PriceSourcePolicy,
    )>,
    /// The last economy preview for `staged_econ_inputs` could not be built; not retried until
    /// the inputs or the prices change.
    econ_preview_failed: bool,

    /// Whether an Apply has succeeded this session; decides what Revert returns the indicator to.
    applied_this_session: bool,

    /// The price-source policy the filter on disk was last written under (start-up: the saved
    /// one, assumed to match the file; then every Apply, and a switch to another filter). A
    /// differing effective source makes the economy section stale ([`Self::source_pending`]).
    applied_policy: PriceSourcePolicy,

    /// `applied_policy` of every filter left this session, by path, so switching away and back
    /// keeps a pending price-source change on that filter.
    left_policies: std::collections::HashMap<PathBuf, PriceSourcePolicy>,

    /// The class catalogue for the class picker, from `config.json`. `Err` degrades the picker
    /// to the generic chip list; it never blocks editing.
    classes: Result<ClassCatalog, String>,

    /// The styles source (`styles.json`, or the built-in default when it is absent) and, for a
    /// file, its modification time when last read; a change reloads the styles, so creating or
    /// deleting the live file switches the source.
    styles_stamp: Option<StylesStamp>,

    /// Keeps a unit-test app's per-test home alive as long as the app.
    #[cfg(test)]
    test_home: Option<tempfile::TempDir>,

    /// `ctx.input(|i| i.time)` of the last `styles.json` poll.
    styles_checked: f64,

    /// Which top-level tab is showing. Switching never applies or discards anything.
    tab: ShellTab,

    /// The CONFIG tab's model (styles + tiers edits). `Err` holds the rendered load failure.
    config: Result<ConfigState, String>,

    /// View state of the CONFIG tab.
    config_ui: ConfigUi,

    /// The CONFIG footer's latest message and whether it is an accent (problem) line.
    config_message: Option<(String, bool)>,

    /// Styles `config.json` itself references, for the delete warning.
    config_refs: BTreeSet<String>,

    /// The per-user settings (the chosen game-folder filter), from `ff-settings.json`.
    settings: Settings,

    /// The in-game config folder, resolved once.
    game_dir: Option<std::path::PathBuf>,

    /// `*.filter` names in the game folder (`None` = missing or unreadable). Refreshed on CONFIG
    /// activation, create, switch and reload -- never per frame (OneDrive reads are slow).
    filters_cache: Option<Vec<String>>,

    /// A filter picked while the rules have unapplied edits, awaiting confirmation.
    pending_switch: Option<String>,

    /// A game folder picked while the rules have unapplied edits, awaiting confirmation.
    pending_folder: Option<std::path::PathBuf>,

    /// The CONFIG tab asked for the folder dialog; started where the frame is at hand.
    want_browse: bool,

    /// The native folder dialog worker.
    folder_pick: super::folder_dialog::FolderPick,

    /// `FERRISFILTER_GAME_DIR` pins the folder, so a pick is ignored.
    game_dir_pinned: bool,

    /// The default game folder (Documents\My Games\Path of Exile 2), for the Config tab.
    default_game_dir: Option<std::path::PathBuf>,

    /// The game folder does not exist on disk. Read only when the lists are refreshed.
    game_dir_missing: bool,

    /// The sound-file listing of the filter folder needs re-reading (startup, reload, switch);
    /// done on the next frame, where the egui context is at hand.
    sound_dir_stale: bool,
}

/// Progress of the debug screenshot run.
#[cfg(debug_assertions)]
struct ShotState {
    args: super::screenshot::ShotArgs,
    frames: u32,
    requested: bool,
}

/// Seconds between `styles.json` modification-time polls while the window runs.
const STYLES_POLL_SECS: f64 = 2.0;

/// The id of the `n`-th row (1-based), for the debug screenshot `--expand`.
#[cfg(debug_assertions)]
fn nth_row_id(rows: &[rules::RuleRow], n: usize) -> Option<BlockId> {
    n.checked_sub(1).and_then(|i| rows.get(i)).map(|r| r.id)
}

/// The `n`-th user section (1-based, the economy section skipped), for `--rename-section`.
#[cfg(debug_assertions)]
fn nth_user_section(sections: &[rules::SectionRow], n: usize) -> Option<&rules::SectionRow> {
    n.checked_sub(1)
        .and_then(|i| sections.iter().filter(|s| !s.economy).nth(i))
}

/// Point the rules at the CONFIG tab's in-memory style set. A no-op (false) when they already
/// hold exactly that set, so it is cheap to call every frame.
fn sync_working_styles(config: &ConfigState, working: &mut WorkingFilter) -> bool {
    // `IndexMap` equality ignores order, so a pure reorder needs the name order compared too.
    let pending = config.pending_styles();
    if working
        .style_set()
        .is_some_and(|w| w == pending && w.names().eq(pending.names()))
    {
        return false;
    }
    working.set_styles_over(config.pending_styles().clone(), config.applied_styles());
    true
}

/// The saved `game_folder` of a resolved app root (none for an unresolved root).
fn saved_game_folder(root: &Result<PathBuf, String>) -> Option<PathBuf> {
    let root = root.as_ref().ok()?;
    settings::load(root)
        .0
        .game_folder_path()
        .map(Path::to_path_buf)
}

/// The game folder at startup, with every input injected: `env` (FERRISFILTER_GAME_DIR) wins,
/// then the `game_folder` saved in the root's `ff-settings.json` (restored on the next launch),
/// then the Documents default. Reads only; creates nothing.
#[cfg_attr(not(test), allow(dead_code))]
fn startup_game_dir_from(
    root: &Result<PathBuf, String>,
    env: Option<&std::ffi::OsStr>,
    documents: impl FnOnce() -> Option<PathBuf>,
) -> Option<PathBuf> {
    let saved = saved_game_folder(root);
    crate::locations::game_dir_from(env, saved.as_deref(), documents)
}

/// [`startup_game_dir_from`] with the real environment; never called from unit tests.
fn startup_game_dir(root: &Result<PathBuf, String>) -> Option<PathBuf> {
    crate::locations::game_dir(saved_game_folder(root).as_deref())
}

/// The FerrisFilter folder: `FERRISFILTER_HOME`, else the per-user folder. Unit tests never
/// resolve it (they build apps on a per-test temp home instead).
#[cfg(not(test))]
fn resolve_root() -> Result<PathBuf, String> {
    crate::locations::from_env()
        .map(|d| d.root)
        .map_err(|e| e.to_string())
}

#[cfg(test)]
fn resolve_root() -> Result<PathBuf, String> {
    Err("unit tests never resolve the FerrisFilter folder".to_string())
}

/// poe.ninja's economy league list. Unit tests never reach the network.
#[cfg(not(test))]
fn fetch_leagues() -> Result<Vec<crate::sync::models::EconomyLeague>, String> {
    crate::sync::fetch_leagues_blocking().map_err(|e| e.to_string())
}

#[cfg(test)]
fn fetch_leagues() -> Result<Vec<crate::sync::models::EconomyLeague>, String> {
    Err("unit tests never fetch the league list".to_string())
}

/// The economy body a staged **Add economy section** previews: priced with the CONFIG tab's pending
/// styles and tiers when loaded (what `apply_all::prepare` will write), else from disk.
/// Shown when the priced economy body cannot be previewed as a section (nothing is staged).
const PREVIEW_FAILED_STATUS: &str =
    "Could not build the economy section preview. Nothing was staged.";

/// Why a league pick or price-source switch was not saved: the file exists but cannot be used,
/// and is never replaced (the choice holds for this session).
const UNUSABLE_SETTINGS_NOT_SAVED: &str = "the file cannot be used, fix or delete it";

/// Apply & quit with tier edits pending under Fallback: the window stays open with them kept.
const UNWRITABLE_TIERS_QUIT_STATUS: &str = "Not quit: tier edits apply only with Live prices. Switch to Live and Apply, or quit without applying.";

fn staged_economy_body(
    root: &Path,
    config: Option<&ConfigState>,
    policy: PriceSourcePolicy,
) -> Result<String, String> {
    let paths = InputPaths::in_home(root);
    let priced = match config {
        Some(c) => {
            dynamic::economy_body_with(&paths, policy, c.pending_styles(), c.pending_tiers())
        }
        None => dynamic::economy_body(&paths, policy),
    };
    priced.map(|(body, _)| body).map_err(|e| e.to_string())
}

/// The styles source under `root` loaded with the name its errors use: `styles.json` when it
/// exists, else the built-in default. Never writes.
fn load_styles_named(root: &std::path::Path) -> Result<(StyleSet, String), String> {
    match InputPaths::in_home(root)
        .styles_source()
        .map_err(|e| e.to_string())?
    {
        Source::File(p) => {
            let name = p.to_string_lossy().into_owned();
            loader::load_styles(&name)
                .map(|s| (s, name))
                .map_err(|e| e.to_string())
        }
        Source::Embedded(b) => {
            let name = b.label();
            loader::load_styles_str(b.text, &name)
                .map(|s| (s, name))
                .map_err(|e| e.to_string())
        }
    }
}

/// The styles under `root`: `styles.json`, else the built-in default. Never writes.
fn load_styles_at(root: &std::path::Path) -> Result<StyleSet, String> {
    load_styles_named(root).map(|(s, _)| s)
}

/// The class catalogue and the styles the built-in `config.json` references (the overlay never
/// reads a `config.json` from the folder), with styles from the effective source.
fn load_classes_at(root: &std::path::Path) -> Result<(ClassCatalog, BTreeSet<String>), String> {
    let paths = InputPaths::in_home(root);
    let (styles, styles_name) = load_styles_named(root)?;
    loader::load_config_str_with_live(
        crate::defaults::CONFIG.text,
        &crate::defaults::CONFIG.label(),
        styles,
        &styles_name,
        &paths.styles.to_string_lossy(),
    )
    .map(|config| {
        (
            ClassCatalog::from_config(&config),
            config_state::config_style_refs(&config),
        )
    })
    .map_err(|e| e.to_string())
}

/// What the styles were last read from: the built-in default, or a live file and its
/// modification time. A change of either reloads the styles.
#[derive(Debug, Clone, PartialEq, Eq)]
enum StylesStamp {
    Embedded,
    File(std::path::PathBuf, std::time::SystemTime),
}

/// The effective styles source under `root`; `None` when the source cannot be resolved or a
/// live file cannot be stat'ed. Never creates a file.
fn styles_stamp_at(root: &std::path::Path) -> Option<StylesStamp> {
    match InputPaths::in_home(root).styles_source().ok()? {
        Source::Embedded(_) => Some(StylesStamp::Embedded),
        Source::File(path) => {
            let mtime = std::fs::metadata(&path).and_then(|m| m.modified()).ok()?;
            Some(StylesStamp::File(path, mtime))
        }
    }
}

/// Builds the working document from a loaded filter (the migrated text, never the raw bytes).
fn working_from(filter: &Result<LoadedFilter, String>) -> Option<WorkingFilter> {
    filter
        .as_ref()
        .ok()
        .map(|loaded| WorkingFilter::from_text(loaded.text()))
}

/// What one combined Apply did.
#[derive(Debug, Clone, PartialEq, Eq)]
enum AllOutcome {
    /// Applied. `filter` is the written filter's path, or `None` when there was no filter part
    /// (none loaded) and only the config files were handled.
    Applied { filter: Option<PathBuf> },
    /// Refused or failed: a finished sentence. Nothing was changed (or a failed write was rolled
    /// back).
    Failed(String),
}

/// The result of [`apply_everything`].
struct AllResult {
    outcome: AllOutcome,
    /// Sentences to append to the success message.
    extras: String,
    styles_written: bool,
    tiers_written: bool,
    /// On a changed-on-disk refusal, every file found changed (recovered by RELOAD or OVERWRITE).
    changed: Vec<DiskFile>,
}

impl AllResult {
    fn failed(message: String) -> Self {
        AllResult {
            outcome: AllOutcome::Failed(message),
            extras: String::new(),
            styles_written: false,
            tiers_written: false,
            changed: Vec::new(),
        }
    }
}

/// A refusal as a result: the changed files named by their real names.
fn refusal(err: ApplyAllError, filter_path: Option<&Path>) -> AllResult {
    match err {
        ApplyAllError::ChangedOnDisk(files) => {
            let fallback = Path::new("the filter");
            let names: Vec<String> = files
                .iter()
                .map(|f| f.label(filter_path.unwrap_or(fallback)))
                .collect();
            let mut result = AllResult::failed(apply::changed_on_disk_message(&names));
            result.changed = files;
            result
        }
        other => AllResult::failed(other.message()),
    }
}

/// RELOAD after a changed-on-disk refusal: re-reads only the files in `changed`, discarding the
/// edits of the side that owns each (the filter: RULES edits; styles.json and tiers.json: CONFIG
/// edits, reloaded together). The other side keeps its edits. The rules always pick up the styles
/// now on disk.
fn reload_changed_files(
    root: &Path,
    changed: &[DiskFile],
    config: &mut Result<ConfigState, String>,
    filter: &mut Result<LoadedFilter, String>,
    working: &mut Option<WorkingFilter>,
) {
    if changed.contains(&DiskFile::Filter)
        && let Ok(loaded) = &*filter
    {
        let path = loaded.path().to_path_buf();
        *filter = LoadedFilter::load(&path).map_err(|e| e.to_string());
        *working = working_from(filter);
    }
    if changed
        .iter()
        .any(|f| matches!(f, DiskFile::Styles | DiskFile::Tiers))
    {
        *config = ConfigState::load(root);
    }
    if let Some(w) = working {
        w.set_styles(load_styles_at(root));
    }
}

/// What the CONFIG tab's reload asks for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ConfigReload {
    /// Re-read only the config files; RULES edits and the filter are left alone.
    ConfigOnly,
    /// The shared RELOAD after a changed-on-disk refusal ([`reload_changed_files`]).
    Changed,
}

/// The load-error view's "Reload from disk" (config failed to load) always retries the config
/// itself and never touches the filter; the footer RELOAD after a refusal re-reads what changed.
fn config_reload_plan(config_failed: bool, changed: &[DiskFile]) -> ConfigReload {
    if config_failed || changed.is_empty() {
        ConfigReload::ConfigOnly
    } else {
        ConfigReload::Changed
    }
}

/// Where a bottom-bar Reload click goes (review CR-01).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BarReload {
    /// The CONFIG tab after a changed-on-disk refusal: the shared re-read of what changed
    /// (`ConfigAction::Reload`).
    ConfigRefusal,
    /// Everything else, the CONFIG tab included: the RULES reload (`reload_pressed`), which
    /// re-reads the filter and keeps CONFIG edits unless the root moved or the config had failed
    /// to load. Without a refusal the bar's Reload is only live because of the filter (damaged
    /// markers, a load failure, no app root), so it must re-read the filter and must not drop
    /// CONFIG edits.
    Shared,
}

fn bar_reload_target(tab: ShellTab, changed: &[DiskFile]) -> BarReload {
    if tab == ShellTab::Config && !changed.is_empty() {
        BarReload::ConfigRefusal
    } else {
        BarReload::Shared
    }
}

/// The one Apply path (UAT2-10): the CONFIG edits (`styles.json`, `tiers.json`) and the RULES edits
/// (the filter, rendered with the pending styles and tier cutoffs) validated together, then
/// written all-or-none by [`apply_all`]. `config` is `None` when the CONFIG state failed to load;
/// `filter` is `None` when no filter is loaded, in which case only the config part is applied.
///
/// Bytes identical to the file on disk are not rewritten, but still count as applied: pressing
/// Apply always counts, so the in-game reload still runs. With `overwrite`, files that changed on
/// disk since they were loaded are written over (OVERWRITE after a changed-on-disk refusal); the
/// baselines move only on success, so a failed overwrite leaves the refusal in place.
fn apply_everything(
    root: &Path,
    mut config: Option<&mut ConfigState>,
    mut filter: Option<(&mut LoadedFilter, &mut WorkingFilter)>,
    policy: PriceSourcePolicy,
    overwrite: bool,
) -> AllResult {
    let filter_path = filter.as_ref().map(|(l, _)| l.path().to_path_buf());

    // A config file without pending edits follows the disk, so an outside edit is rendered with
    // and never written back over.
    if let Some(c) = config.as_deref_mut()
        && let Err(e) = c.follow_disk()
    {
        return AllResult::failed(format!("Nothing was applied: {}.", e.trim_end_matches('.')));
    }

    // Styled rules are rewritten on a clone with the pending styles; a failure here happens
    // before anything is written and leaves the user's edits exactly as they were.
    let mut report = None;
    let input = match &filter {
        Some((loaded, working)) => {
            let styled = match config.as_deref() {
                Some(c) => {
                    let (text, r) = working.text_with_style_set(c.pending_styles());
                    report = Some(r);
                    Ok(text)
                }
                // No config part: styles.json is not written, so load-assigned ids must not reach
                // the filter's tags (WR-06).
                None => working.text_with_styles_keeping_tags().map(|(text, r)| {
                    report = Some(r);
                    text
                }),
            };
            Some(FilterInput {
                loaded: &**loaded,
                styled,
                plan: working.economy_plan(),
                policy,
            })
        }
        None => None,
    };

    let prepared = match apply_all::prepare(root, config.as_deref(), input, overwrite) {
        Ok(p) => p,
        Err(err) => return refusal(err, filter_path.as_deref()),
    };
    let used_fallback = prepared.used_fallback();
    let committed = match apply_all::commit(
        prepared,
        config.as_deref_mut(),
        filter.as_mut().map(|(l, _)| &mut **l),
    ) {
        Ok(c) => c,
        Err(err) => return refusal(err, filter_path.as_deref()),
    };

    // The working document picks up what was written: the new text, then the applied styles.
    if let Some((loaded, working)) = filter.as_mut() {
        if committed.filter.is_some() {
            working.on_applied(loaded.text());
        }
        if let Some(c) = config.as_deref() {
            working.set_styles_over(c.pending_styles().clone(), c.applied_styles());
        }
    }

    let mut extras = String::new();
    if used_fallback {
        extras.push_str(" Used the fallback prices.");
    }
    if let (Some(r), Some(_)) = (&report, &committed.filter) {
        extras.push_str(&apply::styled_rules_message(r.rewritten));
        if let Some((first_rule, _)) = r.dropped.first() {
            extras.push(' ');
            extras.push_str(&apply::dropped_styles_message(r.dropped.len(), *first_rule));
        } else if let Some((first_rule, _)) = r.unknown.first() {
            extras.push(' ');
            extras.push_str(&apply::unknown_styles_message(r.unknown.len(), *first_rule));
        }
        if let Some((first_rule, err)) = r.failed.first() {
            extras.push(' ');
            extras.push_str(&apply::failed_styles_message(
                r.failed.len(),
                *first_rule,
                err,
            ));
        }
    }
    if let Some(f) = &committed.filter {
        if f.migrated {
            extras.push_str(" Converted from v0.4.");
        }
        if let Some(backup) = &f.backup {
            extras.push_str(&format!(" Kept the original as {}.", backup.display()));
        }
    }
    AllResult {
        outcome: AllOutcome::Applied {
            filter: committed.filter.map(|f| f.path),
        },
        extras,
        styles_written: committed.styles_written,
        tiers_written: committed.tiers_written,
        changed: Vec::new(),
    }
}

/// A line's first word, lowercased, past its indent and a disabled block's `#`. Keyword lookup
/// is case-insensitive, so `basetype` and `BaseType` compare equal.
fn line_keyword(text: &str) -> String {
    let t = text.trim_start();
    let t = t.strip_prefix('#').unwrap_or(t).trim_start();
    t.split_whitespace()
        .next()
        .unwrap_or("")
        .to_ascii_lowercase()
}

/// The settings after the section actions of one batch: a rename moves the folded flag to the new
/// name, a removed section's flag is dropped, and a new section starts expanded. A flag is only
/// touched when no other section shares its name (duplicate names share one flag).
fn section_state_after(
    working: &WorkingFilter,
    settings: &Settings,
    actions: &[RowAction],
) -> Settings {
    let filter = settings.filter_name().to_string();
    let mut next = settings.clone();
    let shared = |name: &str, except: Option<usize>| {
        working
            .sections()
            .iter()
            .any(|s| !s.economy && s.name == name && Some(s.ordinal) != except)
    };
    let user = |ordinal: usize| {
        working
            .sections()
            .iter()
            .find(|s| s.ordinal == ordinal && !s.economy)
    };
    for action in actions {
        match action {
            RowAction::RenameSection { ordinal, name } => {
                let Some(sec) = user(*ordinal) else { continue };
                let to = crate::filter::section::sanitize_name(name);
                if to == sec.name {
                    continue;
                }
                let (from_key, to_key) = (
                    settings::section_key(&sec.name, false),
                    settings::section_key(&to, false),
                );
                if !shared(&to, Some(*ordinal)) {
                    next = next
                        .forget_section_state(&filter, &to_key)
                        .copy_section_state(&filter, &from_key, &to_key);
                }
                if !shared(&sec.name, Some(*ordinal)) {
                    next = next.forget_section_state(&filter, &from_key);
                }
            }
            RowAction::RemoveSection(ordinal) => {
                if let Some(sec) = user(*ordinal)
                    && !shared(&sec.name, Some(*ordinal))
                {
                    next = next
                        .forget_section_state(&filter, &settings::section_key(&sec.name, false));
                }
            }
            RowAction::AddSection | RowAction::AddSectionAbove(_) => {
                let name = crate::overlay::rules::DEFAULT_SECTION_NAME;
                if !shared(name, None) {
                    next = next.forget_section_state(&filter, &settings::section_key(name, false));
                }
            }
            _ => {}
        }
    }
    next
}

/// Name of the collapsed user section that holds rule `id`, if any.
fn section_to_reveal(working: &WorkingFilter, settings: &Settings, id: BlockId) -> Option<String> {
    let ordinal = working.rows().iter().find(|r| r.id == id)?.section?;
    let sec = working
        .sections()
        .iter()
        .find(|s| s.ordinal == ordinal && !s.economy)?;
    settings
        .section_collapsed(
            settings.filter_name(),
            &settings::section_key(&sec.name, false),
            false,
        )
        .then(|| sec.name.clone())
}

/// Applies the frame's collected row actions once, after the list rendered. Returns whether the
/// document changed and the last status worth showing.
#[cfg(test)]
fn apply_row_actions(
    working: &mut WorkingFilter,
    list: &mut ListUi,
    query: &mut String,
    actions: Vec<RowAction>,
) -> (bool, Option<String>) {
    apply_row_actions_folding(working, list, query, actions, None)
}

/// [`apply_row_actions`], also keeping the stored folded state of the sections in `fold` in step
/// (WR-04). Each section action's fold change is computed against the document just before it
/// runs and kept only when the action changed the document, so a refused rename or remove leaves
/// the flags alone and a batch of structural actions follows the shifting ordinals (WR-06).
fn apply_row_actions_folding(
    working: &mut WorkingFilter,
    list: &mut ListUi,
    query: &mut String,
    actions: Vec<RowAction>,
    mut fold: Option<&mut Settings>,
) -> (bool, Option<String>) {
    let mut changed = false;
    let mut status = None;
    for action in rules::coalesce_edits(actions) {
        let fold_next = match (&fold, &action) {
            (
                Some(settings),
                RowAction::RenameSection { .. }
                | RowAction::RemoveSection(_)
                | RowAction::AddSection
                | RowAction::AddSectionAbove(_),
            ) => Some(section_state_after(
                working,
                settings,
                std::slice::from_ref(&action),
            )),
            _ => None,
        };
        let is_add = matches!(action, RowAction::AddRule);
        let is_add_section = matches!(action, RowAction::AddSection);
        let section_above = match &action {
            RowAction::AddSectionAbove(id) => Some(*id),
            _ => None,
        };
        let drops_rename = matches!(
            action,
            RowAction::RemoveSection(_) | RowAction::MoveSection(..)
        );
        let is_duplicate = matches!(action, RowAction::Duplicate(_));
        let is_move = matches!(action, RowAction::Move(..));
        let deleted = match &action {
            RowAction::Delete(id) => Some(*id),
            _ => None,
        };
        let edited_block = match &action {
            RowAction::EditLine { id, .. }
            | RowAction::AddLine { id, .. }
            | RowAction::InsertLineAfter { id, .. }
            | RowAction::RemoveLine { id, .. }
            | RowAction::SetDefenceSelection { id, .. } => Some(*id),
            _ => None,
        };
        let added_in = match &action {
            RowAction::AddLine { id, .. } | RowAction::InsertLineAfter { id, .. } => Some(*id),
            _ => None,
        };
        // Edits that can shift the keyword ordinals of the cards after them: removing or adding
        // a line, and a raw whole-line edit that changes the line's keyword (WR-07, IN-07).
        let shifts_ordinals = match &action {
            RowAction::RemoveLine { .. }
            | RowAction::AddLine { .. }
            | RowAction::InsertLineAfter { .. }
            | RowAction::SetDefenceSelection { .. } => true,
            RowAction::EditLine {
                expect,
                patch: LinePatch::Whole(text) | LinePatch::Content(text),
                ..
            } => line_keyword(expect) != line_keyword(text),
            _ => false,
        };
        let refused_edit = match &action {
            RowAction::EditLine {
                id, line, patch, ..
            } if matches!(
                patch,
                LinePatch::Whole(_) | LinePatch::Values(_) | LinePatch::Content(_)
            ) =>
            {
                Some((*id, *line))
            }
            _ => None,
        };
        let outcome = working.apply_action(action);
        if !outcome.changed
            && let (Some((id, line)), Some(msg)) = (refused_edit, outcome.status.clone())
        {
            list.line_error = Some((id, line, msg));
        }
        if outcome.changed {
            changed = true;
            if let (Some(settings), Some(next)) = (fold.as_deref_mut(), fold_next) {
                *settings = next;
            }
            if is_add_section {
                // The new divider is at the very top: clear the search so it shows, scroll
                // there and put the cursor in its name box.
                query.clear();
                list.scroll_top = true;
            }
            if drops_rename {
                list.editing_section = None;
            }
            if let Some(ordinal) = outcome.section {
                list.editing_section = Some(ordinal);
                list.section_draft = "Section".to_string();
                list.section_focus = true;
            }
            if let Some(id) = section_above {
                // The rule stays open and in view next to its new header.
                list.pending_scroll = Some(id);
            }
            if let Some(b) = edited_block {
                if list.line_error.as_ref().is_some_and(|(e, _, _)| *e == b) {
                    list.line_error = None;
                }
            }
            if let (Some(b), Some(line)) = (added_in, outcome.line) {
                list.focus_line = Some((b, line));
            }
            // A shifted ordinal would move the later cards' egui temp state onto their
            // neighbours (WR-07).
            if shifts_ordinals {
                list.forget_card_state();
            }
            if let Some(id) = deleted {
                if list.expanded == Some(id) {
                    list.expanded = None;
                }
                if list.confirm_delete == Some(id) {
                    list.confirm_delete = None;
                }
            }
            if let Some(focus) = outcome.focus {
                if is_add {
                    // The new rule is row 1: clear the search so it is visible, open it and
                    // put the cursor in its NAME field.
                    query.clear();
                    list.expanded = Some(focus);
                    list.pending_scroll = Some(focus);
                    list.focus_name = Some(focus);
                    status = Some(apply::RULE_ADDED_MESSAGE.to_string());
                } else if is_duplicate {
                    list.expanded = Some(focus);
                    list.pending_scroll = Some(focus);
                } else if is_move {
                    list.pending_scroll = Some(focus);
                }
            }
        }
        if outcome.status.is_some() {
            status = outcome.status;
        }
    }
    (changed, status)
}

/// A heading only, used for the empty and error states (detail lives in tooltips and the status line).
fn centered_notice(ui: &mut egui::Ui, heading: &str) {
    ui.add_space(16.0);
    ui.vertical_centered(|ui| {
        ui.label(
            egui::RichText::new(heading)
                .font(fonts::small_caps(21.0))
                .color(pal::HEADING),
        );
    });
}

/// What the ECONOMY tab shows about prices.json; re-read on switching to ECONOMY, at start and
/// after a sync.
#[derive(Debug, Clone, Default, PartialEq)]
struct EconomyInfo {
    mtime_secs: Option<u64>,
    priced_items: usize,
    live: bool,
    /// Why an existing prices.json is unusable (unreadable, malformed or without items). The
    /// file's mtime is then not a sync time, so `mtime_secs` is `None`.
    problem: Option<String>,
    /// The league the file was synced for (None when absent, unusable or blank).
    league: Option<String>,
}

impl EconomyInfo {
    /// Reads `<root>/prices.json`. A missing, empty or malformed file is not live (never panics).
    fn read(root: &std::path::Path) -> EconomyInfo {
        let path = root.join("prices.json");
        let mtime_secs = std::fs::metadata(&path)
            .ok()
            .and_then(|meta| meta.modified().ok())
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|d| d.as_secs());
        let (priced_items, problem, league) =
            match crate::loader::load_prices_optional(&path.to_string_lossy()) {
                Ok(Some(p)) if p.items.is_empty() => (
                    0,
                    Some("prices.json has no priced items.".to_string()),
                    None,
                ),
                Ok(Some(p)) => {
                    let league = (!p.league.trim().is_empty()).then(|| p.league.clone());
                    (p.items.len(), None, league)
                }
                Ok(None) => (0, None, None),
                Err(err) => (0, Some(err.to_string()), None),
            };
        EconomyInfo {
            mtime_secs: if problem.is_some() { None } else { mtime_secs },
            priced_items,
            live: priced_items > 0,
            problem,
            league,
        }
    }
}

/// What picking a filter in the dropdown should do.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SwitchDecision {
    /// The picked filter is already the active one.
    Noop,
    /// Unapplied rule edits exist: ask first.
    Confirm,
    /// Switch now.
    Switch,
}

fn switch_decision(rules_dirty: bool, current: &str, target: &str) -> SwitchDecision {
    if current.eq_ignore_ascii_case(target) {
        SwitchDecision::Noop
    } else if rules_dirty {
        SwitchDecision::Confirm
    } else {
        SwitchDecision::Switch
    }
}

/// The path of a game-folder filter by name. The name is re-validated here, before any join, so a
/// hand-edited settings file can never point outside the game folder.
fn filter_target(
    game_dir: Option<&std::path::Path>,
    name: &str,
) -> Result<std::path::PathBuf, String> {
    settings::valid_filter_name(name).map_err(|problem| problem.to_string())?;
    match game_dir {
        Some(dir) => Ok(dir.join(name)),
        None => Err(
            "Cannot find the Documents folder; pick the game folder on the Config tab.".to_string(),
        ),
    }
}

/// The settings to save when `name` becomes the active filter; unknown keys are preserved.
fn settings_with_filter(current: &Settings, name: &str) -> Settings {
    Settings {
        filter: Some(name.to_string()),
        auto_filter: None,
        ..current.clone()
    }
}

/// The settings to save when `path` becomes the game folder and `name` the filter in it; unknown
/// keys are preserved. The path is stored exactly as picked (never canonicalized).
fn settings_with_folder(current: &Settings, path: &std::path::Path, name: &str) -> Settings {
    Settings {
        game_folder: Some(path.to_string_lossy().into_owned()),
        filter: Some(name.to_string()),
        auto_filter: None,
        ..current.clone()
    }
}

/// The settings RELOAD keeps after re-reading `ff-settings.json`.
///
/// A clean load, or a load from a different root, replaces the session's settings. A load that
/// warned keeps the in-memory filter and unknown keys (a momentarily unreadable file must not
/// reset them), but an "off" from the file is still honoured: `settings::load` reports a broken
/// or wrongly typed file as reload-off, and the warning says so, so keeping the old "on" would
/// send keystrokes the window has just said it won't send.
fn settings_after_reload(
    current: &Settings,
    loaded: Settings,
    warned: bool,
    same_root: bool,
) -> Settings {
    if !warned || !same_root {
        return loaded;
    }
    if loaded.reload_after_apply == Some(false) {
        return settings_with_reload(current, false);
    }
    current.clone()
}

/// With no saved filter, selects one from `game_dir`'s listing: `current` when listed, else the
/// first `.filter`; an empty or unreadable folder leaves the name as it is. The pick goes to
/// [`Settings::auto_filter`], which is never written (not even by a later save of another
/// preference), so every place that replaces the settings from disk (startup, RELOAD) applies it
/// again and the overlay stays on the filter it had open.
fn choose_unsaved_filter(settings: &mut Settings, game_dir: Option<&Path>, current: &str) {
    if settings.filter.is_none()
        && let Some(list) = game_dir.and_then(|dir| settings::list_filters(dir).ok())
        && !list.is_empty()
    {
        settings.auto_filter = Some(settings::choose_filter(&list, current));
    }
}

/// `current` with the reload-after-Apply preference replaced; the filter and unknown keys stay.
fn settings_with_reload(current: &Settings, on: bool) -> Settings {
    Settings {
        reload_after_apply: Some(on),
        ..current.clone()
    }
}

/// `current` with the economy league replaced; every other field and unknown key stays.
fn settings_with_league(current: &Settings, name: &str) -> Settings {
    Settings {
        league: Some(name.to_string()),
        ..current.clone()
    }
}

/// `current` with the price source replaced (`true` = live prices, `false` = fallback pair).
fn settings_with_price_source(current: &Settings, live: bool) -> Settings {
    Settings {
        price_source: Some(if live { "live" } else { "fallback" }.to_string()),
        ..current.clone()
    }
}

/// A reload that was started and has not reported yet.
struct PendingReload {
    path: PathBuf,
    /// The config part of the message ("Saved styles.json. "), which leads it.
    prefix: String,
    extras: String,
    /// The interim status line; the result replaces it only while it is still showing.
    interim: String,
    /// The landed line is a warning whatever the reload result (it carries the "Not quit" note).
    warning: bool,
}

/// Shows the "Not quit" warning of an Apply & quit that stays open for unused tier edits. While a
/// reload's interim line is showing, the warning is folded into it and into the line the reload
/// result will become, so it survives the conceal the reload causes and the result still lands.
fn show_not_quit_warning(
    message: &mut Option<(String, w::StatusKind)>,
    pending: Option<&mut PendingReload>,
) {
    match pending {
        Some(p) if message.as_ref().is_some_and(|(t, _)| *t == p.interim) => {
            let note = format!(" {UNWRITABLE_TIERS_QUIT_STATUS}");
            p.interim.push_str(&note);
            p.extras.push_str(&note);
            p.warning = true;
            *message = Some((p.interim.clone(), w::StatusKind::Warning));
        }
        _ => {
            *message = Some((
                UNWRITABLE_TIERS_QUIT_STATUS.to_string(),
                w::StatusKind::Warning,
            ));
        }
    }
}

/// The status colour of an update message.
fn update_kind(kind: crate::update::UpdateKind) -> w::StatusKind {
    use crate::update::UpdateKind;
    match kind {
        UpdateKind::Info => w::StatusKind::Info,
        UpdateKind::Ok => w::StatusKind::Ok,
        UpdateKind::Warning => w::StatusKind::Warning,
        UpdateKind::Error => w::StatusKind::Error,
    }
}

/// The Apply status line after the window is concealed. Hiding normally clears it, but not while
/// a reload is in flight and its interim line is still showing: the reload's own focus change is
/// what hides the window (the click-away rule sees the game take focus), so clearing here would
/// drop every reload result, success or failure, before it could land (review CR-01). The kept
/// interim line is replaced by [`landed_reload_message`] and read on the next reveal.
fn apply_message_after_conceal(
    current: Option<String>,
    pending: Option<&PendingReload>,
) -> Option<String> {
    match (current, pending) {
        (Some(line), Some(p)) if line == p.interim => Some(line),
        _ => None,
    }
}

/// The status line once the reload worker reports, or `None` when the interim line is gone
/// (an edit, a RELOAD/switch or a later message replaced it) and the later message stays. A
/// conceal does not remove the interim line ([`apply_message_after_conceal`]), so a result that
/// lands while the window is hidden is still shown on the next reveal.
fn landed_reload_message(
    current: Option<&str>,
    pending: &PendingReload,
    result: &Result<(), String>,
) -> Option<String> {
    if current != Some(pending.interim.as_str()) {
        return None;
    }
    Some(format!(
        "{}{}{}",
        pending.prefix,
        apply::reload_result_message(&pending.path, result),
        pending.extras
    ))
}

/// A successful Apply's status line, with its colour class decided now (review CR-02).
fn applied_message(text: String) -> Option<(String, w::StatusKind)> {
    let kind = shell::apply_success_kind(&text);
    Some((text, kind))
}

/// The status line when no reload was sent: the success line, the reason and the Refresh reminder.
fn skipped_reload_message(path: &Path, extras: &str, reason: chat::SkipReason) -> String {
    // Game not running: the game reads the filter at start, so only the written path is shown.
    match reason {
        chat::SkipReason::GameNotRunning => format!("{}{extras}", apply::written_message(path)),
        _ => format!(
            "{}{extras}",
            apply::not_reloaded_message(path, reason.text())
        ),
    }
}

impl OverlayApp {
    /// Builds the app state around a tray and hotkey already constructed on
    /// the main thread (thread affinity -- see `overlay::launch`).
    pub fn new(
        tray: TrayHandles,
        hotkey_manager: Option<GlobalHotKeyManager>,
        hotkey_status: HotkeyStatus,
        instance: instance::Guard,
        skin: Skin,
        pending_install: std::sync::Arc<std::sync::Mutex<Option<PathBuf>>>,
    ) -> Self {
        let root = resolve_root();
        let game_dir = startup_game_dir(&root);
        let mut app = Self::build(
            tray,
            hotkey_manager,
            hotkey_status,
            Some(instance),
            skin,
            root,
            game_dir,
        );
        app.pending_install = pending_install;
        app
    }

    /// The debug screenshot app: an inert tray, no hotkey, no instance guard, the requested tab
    /// shown and the window kept visible until the frame is saved.
    #[cfg(debug_assertions)]
    pub(crate) fn new_screenshot(skin: Skin, shot: super::screenshot::ShotArgs) -> Self {
        let root = resolve_root();
        let game_dir = startup_game_dir(&root);
        let mut app = Self::build(
            TrayHandles::inert(),
            None,
            HotkeyStatus::Failed("screenshot mode".to_string()),
            None,
            skin,
            root,
            game_dir,
        );
        app.tab = shot.tab;
        app.visibility = Visibility::ShownFocused;
        if shot.tab == ShellTab::Config {
            app.refresh_filters();
        }
        app.open_shot_popup(shot.popup.as_deref());
        if let Some(q) = &shot.search {
            app.query = q.clone();
        }
        match shot.update_state.as_deref() {
            Some("checking") => app.update.inject(UpdateState::Checking),
            Some("downloading") => app.update.inject(UpdateState::Downloading {
                version: "0.8.0".to_string(),
            }),
            Some(s) if s.starts_with("ready:") => {
                let mut parts = s.splitn(3, ':').skip(1);
                let version = parts.next().unwrap_or_default().to_string();
                let date = parts.next().unwrap_or_default().to_string();
                app.update
                    .inject(UpdateState::Ready(crate::update::ReadyUpdate {
                        version: version.clone(),
                        published_unix: crate::update::parse_published(&format!(
                            "{date}T12:00:00Z"
                        )),
                        installer: PathBuf::new(),
                        sha256: String::new(),
                    }));
                app.update_message = Some((
                    format!("{version} available, released {date}"),
                    w::StatusKind::Info,
                ));
            }
            _ => {}
        }
        match shot.stage_economy.as_deref() {
            Some("add") => app.stage_add_economy(),
            Some("remove") => app.stage_remove_economy(),
            _ => {}
        }
        if let Some(n) = shot.expand {
            let id = app.working.as_ref().and_then(|w| nth_row_id(w.rows(), n));
            if id.is_some() {
                app.list.expanded = id;
                app.list.pending_scroll = id;
            }
        }
        if let Some(n) = shot.expand_economy {
            let id = app
                .working
                .as_ref()
                .and_then(|w| w.economy_rows().get(n - 1).map(|r| r.id));
            if id.is_some() {
                let name = crate::overlay::rules::ECONOMY_DISPLAY_NAME;
                app.set_section_collapsed(name, true, false);
                app.list.expanded = id;
                app.list.pending_scroll = id;
            }
        }
        if let Some(n) = shot.rename_section {
            let found = app
                .working
                .as_ref()
                .and_then(|w| nth_user_section(w.sections(), n).cloned());
            if let Some(sec) = found {
                app.list.editing_section = Some(sec.ordinal);
                app.list.section_draft = sec.name;
                app.list.section_focus = true;
            }
        }
        if let Some(name) = &shot.expand_style {
            app.config_ui.expanded = Some(name.clone());
            app.config_ui.scroll_to = Some(name.clone());
        }
        if let (Some((tier, text)), Ok(state)) = (&shot.tier_text, &mut app.config) {
            state.set_tier_text(tier, text);
        }
        use crate::overlay::screenshot::ShotLeagues;
        match &shot.leagues {
            Some(ShotLeagues::List(names)) => {
                app.leagues
                    .inject(ListState::Loaded(crate::overlay::screenshot::fake_leagues(
                        names,
                    )));
            }
            Some(ShotLeagues::Failed) => {
                app.leagues
                    .inject(ListState::Failed("screenshot mode: no league list".into()));
            }
            Some(ShotLeagues::Loading) => app.leagues.inject(ListState::Loading),
            None => {}
        }
        if let Some(name) = &shot.league {
            app.set_league(name);
        }
        if let Some(src) = &shot.price_source {
            app.set_price_source(src == "live");
        }
        app.shot = Some(ShotState {
            args: shot,
            frames: 0,
            requested: false,
        });
        app
    }

    /// Opens the popup named by `--popup` with harmless sample data. Choices are never
    /// dispatched in screenshot mode (no input arrives).
    #[cfg(debug_assertions)]
    fn open_shot_popup(&mut self, name: Option<&str>) {
        match name {
            Some("changed") => {
                self.changed_on_disk = vec![DiskFile::Filter];
                self.changed_popup = true;
            }
            Some("switch") => self.pending_switch = Some("other.filter".to_string()),
            Some("quit") => self.quit_confirm = true,
            Some("delete-rule") => {
                // The row is expanded too: the list drops a pending delete of a collapsed row.
                let id = self
                    .working
                    .as_ref()
                    .and_then(|w| w.rows().first().map(|r| r.id));
                self.list.confirm_delete = id;
                self.list.expanded = id;
            }
            Some("delete-style") => {
                let name = self.config.as_ref().ok().and_then(|c| {
                    c.style_names()
                        .into_iter()
                        .find(|n| !c.badges(n).builtin)
                        .map(|n| n.to_string())
                });
                self.config_ui.delete_armed = Some(name.unwrap_or_else(|| "kite".to_string()));
            }
            Some("remove-economy") => self.list.confirm_economy_remove = true,
            Some("reset-styles") => {
                if let Ok(state) = &mut self.config {
                    state.edit_style("S", config_state::StyleEdit::Font(Some(41)));
                }
                self.config_ui.reset_armed = Some(RevertScope::Styles);
            }
            Some("reset-tiers") => self.config_ui.reset_armed = Some(RevertScope::Tiers),
            Some("discard-config") => {
                if let Ok(state) = &mut self.config {
                    state.edit_style("S", config_state::StyleEdit::Font(Some(41)));
                }
                self.config_ui.revert_armed = true;
            }
            Some("discard-rules") => {
                // A harmless in-memory edit (never applied in this mode) so the popup has
                // something to discard.
                let row = self
                    .working
                    .as_ref()
                    .and_then(|w| w.rows().first().map(|r| (r.id, r.has_continue)));
                if let Some((id, cont)) = row {
                    self.run_row_actions(vec![RowAction::SetContinue(id, !cont)]);
                }
                if !self.rules_revertible() {
                    eprintln!(
                        "--popup discard-rules: could not stage a rule edit (no filter, no rules, \
                         or the first rule is not editable); the popup will not open"
                    );
                }
                self.revert_rules_armed = true;
            }
            _ => {}
        }
    }

    /// True in the debug screenshot mode (never in release builds).
    fn is_shot(&self) -> bool {
        #[cfg(debug_assertions)]
        {
            self.shot.is_some()
        }
        #[cfg(not(debug_assertions))]
        {
            false
        }
    }

    fn build(
        tray: TrayHandles,
        hotkey_manager: Option<GlobalHotKeyManager>,
        hotkey_status: HotkeyStatus,
        instance: Option<instance::Guard>,
        skin: Skin,
        root: Result<PathBuf, String>,
        game_dir: Option<PathBuf>,
    ) -> Self {
        // Settings: the remembered filter. Without a root nothing is read and nothing is saved.
        let (mut settings, settings_warning, filter_fell_back) = match &root {
            Ok(root) => {
                let loaded = settings::load_detailed(root);
                (loaded.settings, loaded.warning, loaded.filter_fell_back)
            }
            Err(_) => (Settings::default(), None, false),
        };
        // A fresh install (no saved filter): use the folder's filter instead of a fixed name.
        // Held in memory only; opening the overlay never writes.
        // A stored name or a file that could not be used says it falls back to `dynamic.filter`,
        // so that is what is opened then; a warning about another key changes nothing here.
        if !filter_fell_back {
            let current = settings.filter_name().to_string();
            choose_unsaved_filter(&mut settings, game_dir.as_deref(), &current);
        }
        // Loading never writes (D-10).
        let filter = Self::load_filter(game_dir.as_deref(), settings.filter_name());
        let working = working_from(&filter);
        let config = match &root {
            Ok(root) => ConfigState::load(root),
            Err(message) => Err(message.clone()),
        };

        let mut app = Self {
            docked: false,
            dock_ppp: None,
            skin,
            economy_info: EconomyInfo::default(),
            tray,
            hotkey_manager,
            hotkey_status,
            instance,
            #[cfg(debug_assertions)]
            shot: None,
            toggle_requested: false,
            quit_confirm: false,
            quit_tier_edits_only: false,
            revert_rules_armed: false,
            quit_after_apply: false,
            quit_after_reload: false,
            quit_next_frame: false,
            // The window created in `overlay::launch` starts visible
            // (`with_active(false)` only suppresses activation, not
            // visibility) and has never been focused -- `ShownUnfocused` is
            // the state that matches that physical reality. Starting here
            // (rather than `Hidden`) is what makes the very first `Ctrl+Shift+Z`
            // press after launch hide the window, matching the checkpoint
            // script's step 3.
            visibility: Visibility::ShownUnfocused,
            window: None,
            root,
            filter,
            changed_on_disk: Vec::new(),
            changed_popup: false,
            overwrite_live: false,
            apply_state: ApplyState::NeverApplied,
            apply_message: None,
            economy: economy::EconomySync::new(),
            update: UpdateJob::new(),
            update_message: None,
            pending_install: std::sync::Arc::new(std::sync::Mutex::new(None)),
            leagues: super::leagues::LeagueList::default(),
            reload: chat::ReloadSend::default(),
            reload_pending: None,
            working,
            list: ListUi::default(),
            query: String::new(),
            list_status: None,
            staged_econ_inputs: None,
            econ_preview_failed: false,
            applied_this_session: false,
            applied_policy: settings.price_policy(),
            left_policies: std::collections::HashMap::new(),
            classes: Err("not loaded".to_string()),
            styles_stamp: None,
            styles_checked: 0.0,
            tab: ShellTab::Rules,
            config,
            config_ui: ConfigUi::default(),
            config_message: None,
            config_refs: BTreeSet::new(),
            settings,
            game_dir,
            filters_cache: None,
            pending_switch: None,
            pending_folder: None,
            want_browse: false,
            folder_pick: super::folder_dialog::FolderPick::default(),
            // Tests never reach the real environment or Documents folder.
            #[cfg(test)]
            game_dir_pinned: false,
            #[cfg(not(test))]
            game_dir_pinned: crate::locations::game_dir_is_pinned(),
            #[cfg(test)]
            default_game_dir: None,
            #[cfg(not(test))]
            default_game_dir: crate::locations::default_game_dir(),
            game_dir_missing: false,
            sound_dir_stale: true,
            #[cfg(test)]
            test_home: None,
        };
        app.refresh_inputs();
        app.refresh_filters();
        app.refresh_economy_info();
        if settings_warning.is_some() {
            app.list_status = settings_warning;
        }
        app
    }

    /// One frame of the screenshot run. Returns true when the process should leave.
    #[cfg(debug_assertions)]
    fn shot_step(&mut self, ctx: &egui::Context) -> bool {
        let docked = self.docked;
        let Some(shot) = self.shot.as_mut() else {
            return false;
        };
        shot.frames += 1;
        ctx.request_repaint();
        if let Some(tag) = &shot.args.picker
            && (3..=5).contains(&shot.frames)
        {
            crate::skin::colour_picker::request_open(ctx, tag);
        }
        if let Some(tag) = &shot.args.dropdown
            && (3..=5).contains(&shot.frames)
        {
            crate::skin::widgets::request_dropdown_open(ctx, tag);
        }
        let image = ctx.input(|i| {
            i.raw.events.iter().find_map(|e| match e {
                egui::Event::Screenshot { image, .. } => Some(image.clone()),
                _ => None,
            })
        });
        if let Some(image) = image {
            match super::screenshot::save_png(&image, &shot.args.png) {
                Ok(()) => eprintln!("ff: saved {}", shot.args.png.display()),
                Err(message) => eprintln!("ff: {message}"),
            }
            return true;
        }
        if docked && shot.frames >= 20 && !shot.requested {
            ctx.send_viewport_cmd(egui::ViewportCommand::Screenshot(Default::default()));
            shot.requested = true;
        } else if shot.frames > 600 {
            eprintln!("ff: no screenshot arrived; exiting");
            return true;
        }
        false
    }

    /// A sync landed new prices: re-read the ECONOMY tab info and forget which inputs the staged
    /// economy-add preview was priced with, so `refresh_staged_economy` re-prices it (WR-02).
    fn prices_changed(&mut self) {
        self.refresh_economy_info();
        self.staged_econ_inputs = None;
        self.econ_preview_failed = false;
    }

    /// Re-reads `prices.json` info (modified time, priced items, live/fallback) for the ECONOMY tab.
    fn refresh_economy_info(&mut self) {
        self.economy_info = match self.root.as_ref() {
            Ok(root) => EconomyInfo::read(root),
            Err(_) => EconomyInfo::default(),
        };
    }

    /// Re-reads the game folder's filter list into the cache.
    fn refresh_filters(&mut self) {
        self.filters_cache = self
            .game_dir
            .as_deref()
            .and_then(|dir| settings::list_filters(dir).ok());
        // The one place the folder's existence is read (never per frame).
        self.game_dir_missing = self.game_dir.as_deref().is_some_and(|d| !d.is_dir());
    }

    /// The status text for a game folder that is not on disk.
    fn game_folder_missing_text(&self) -> Option<String> {
        let dir = self.game_dir.as_deref()?;
        self.game_dir_missing
            .then(|| format!("Game folder not found: {}", dir.display()))
    }

    /// A game folder came back from the dialog. A pick equal to the current folder only re-lists
    /// it; unapplied rule edits make it wait for confirmation (the Switch filter? popup).
    fn on_folder_picked(&mut self, path: std::path::PathBuf) {
        if self.game_dir_pinned {
            return;
        }
        let same = self.game_dir.as_deref().is_some_and(|cur| {
            cur.to_string_lossy()
                .eq_ignore_ascii_case(&path.to_string_lossy())
        });
        if same {
            // Re-picking the current folder re-checks it (it may have appeared since).
            self.refresh_filters();
            return;
        }
        if self.working.as_ref().is_some_and(|w| w.is_dirty()) {
            self.pending_folder = Some(path);
        } else {
            self.apply_folder(path);
        }
    }

    /// Makes `path` the game folder now: the filter is chosen from its listing (same name, else the
    /// first `.filter`, else the current name), and both are saved in one write.
    fn apply_folder(&mut self, path: std::path::PathBuf) {
        self.pending_folder = None;
        let left = self.open_filter_path();
        self.game_dir = Some(path.clone());
        self.refresh_filters();
        let name = settings::choose_filter(
            self.filters_cache.as_deref().unwrap_or(&[]),
            self.settings.filter_name(),
        );
        let next = settings_with_folder(&self.settings, &path, &name);
        self.adopt_settings(next, left);
    }

    /// The path of the filter the settings select, when the game folder is known.
    fn open_filter_path(&self) -> Option<PathBuf> {
        filter_target(self.game_dir.as_deref(), self.settings.filter_name()).ok()
    }

    /// Saves `next` as the settings in force (a failed save still applies it for the session and
    /// says so), then reloads everything that follows the filter and folder. `left` is the
    /// filter being left, whose `applied_policy` is kept for a later return.
    fn adopt_settings(&mut self, next: Settings, left: Option<PathBuf>) {
        let save_error = match &self.root {
            Ok(root) => settings::save(root, &next).err(),
            Err(_) => None,
        };
        self.settings = next;
        self.rebind_applied_policy(left);
        self.reset_filter_state();
        self.refresh_inputs();
        self.refresh_filters();
        if let Some(err) = save_error {
            self.config_message = Some((format!("Could not save ff-settings.json: {err}"), true));
        }
    }

    /// Call once the settings and game folder name the filter now open: `left` (the filter open
    /// before) keeps its `applied_policy` for a later return. A filter left earlier this session
    /// gets back the policy it was written under; any other is assumed written under the policy
    /// now in force.
    fn rebind_applied_policy(&mut self, left: Option<PathBuf>) {
        if let Some(left) = left {
            self.left_policies.insert(left, self.applied_policy);
        }
        self.applied_policy = self
            .open_filter_path()
            .and_then(|p| self.left_policies.get(&p).copied())
            .unwrap_or_else(|| self.settings.price_policy());
    }

    /// Whether the folder dialog is open (read by the Config tab to grey Browse).
    pub(crate) fn folder_dialog_open(&self) -> bool {
        self.folder_pick.is_open()
    }

    /// (Re)reads `styles.json` into the working filter and `config.json` into the class
    /// catalogue. A config failure is shown once in the list status and never blocks editing.
    fn refresh_inputs(&mut self) {
        match &self.root {
            Ok(root) => {
                let root = root.clone();
                self.styles_stamp = styles_stamp_at(&root);
                if let Some(working) = &mut self.working {
                    working.set_styles(load_styles_at(&root));
                }
                match load_classes_at(&root) {
                    Ok((catalog, refs)) => {
                        self.classes = Ok(catalog);
                        self.config_refs = refs;
                    }
                    Err(err) => {
                        self.list_status =
                            Some(format!("Class groups unavailable (config.json): {err}"));
                        self.classes = Err(err);
                        self.config_refs.clear();
                    }
                }
            }
            Err(message) => {
                self.styles_stamp = None;
                if let Some(working) = &mut self.working {
                    working.set_styles(Err(message.clone()));
                }
                self.classes = Err(message.clone());
            }
        }
    }

    /// Reloads the styles when `styles.json`, or `styles-default.json` when it is absent, changed
    /// on disk (polled, not watched; creating or deleting the live file counts as a change), and
    /// lets clean config files follow the disk.
    fn poll_styles(&mut self, now: f64) {
        if now - self.styles_checked < STYLES_POLL_SECS {
            return;
        }
        self.styles_checked = now;
        // Config files without pending edits follow outside edits. A file that does not load
        // right now is left as it is; the next Apply reports it.
        if let Ok(c) = &mut self.config
            && c.follow_disk() == Ok(true)
        {
            self.config_ui.reset_for_new_styles();
        }
        let (Ok(root), Some(working)) = (&self.root, &mut self.working) else {
            return;
        };
        let stamp = styles_stamp_at(root);
        if stamp != self.styles_stamp {
            self.styles_stamp = stamp;
            working.set_styles(load_styles_at(root));
        }
    }

    /// Reads the named game-folder filter. Never writes; an absent file loads as an empty filter.
    fn load_filter(game_dir: Option<&std::path::Path>, name: &str) -> Result<LoadedFilter, String> {
        let path = filter_target(game_dir, name)?;
        LoadedFilter::load(&path).map_err(|e| e.to_string())
    }

    /// Re-reads the active filter and resets every piece of per-filter session state. RELOAD and
    /// switching filters both go through here.
    fn reset_filter_state(&mut self) {
        self.filter = Self::load_filter(self.game_dir.as_deref(), self.settings.filter_name());
        self.working = working_from(&self.filter);
        self.reset_filter_view();
    }

    /// Resets the per-filter session state after the filter and working document were replaced.
    fn reset_filter_view(&mut self) {
        // A switched or reloaded filter makes an interim reload line meaningless.
        self.reload_pending = None;
        self.list.clear_ids();
        self.query.clear();
        self.list_status = None;
        self.changed_on_disk.clear();
        self.apply_message = None;
        self.applied_this_session = false;
        self.apply_state = ApplyState::NeverApplied;
        self.pending_switch = None;
        self.pending_folder = None;
        self.sound_dir_stale = true;
    }

    /// The folder the style editor lists sound files from: the loaded filter's folder, else the
    /// game folder.
    fn sound_dir(&self) -> Option<std::path::PathBuf> {
        filter_target(self.game_dir.as_deref(), self.settings.filter_name())
            .ok()
            .and_then(|p| p.parent().map(std::path::Path::to_path_buf))
            .or_else(|| self.game_dir.clone())
    }

    /// Makes `name` the active filter now and remembers it. A failed save still switches for
    /// this session and says so.
    fn switch_to(&mut self, name: String) {
        let left = self.open_filter_path();
        let next = settings_with_filter(&self.settings, &name);
        self.adopt_settings(next, left);
    }

    /// Saves the open filter as the selected one, without reloading anything (it is already
    /// open). A failed save still selects it for this session and says so.
    fn remember_open_filter(&mut self) {
        let name = self.settings.filter_name().to_string();
        let next = settings_with_filter(&self.settings, &name);
        let save_error = match &self.root {
            Ok(root) => settings::save(root, &next).err(),
            Err(_) => None,
        };
        self.settings = next;
        if let Some(err) = save_error {
            self.config_message = Some((format!("Could not save ff-settings.json: {err}"), true));
        }
    }

    /// Saves `next` as the settings now in force. A failed save still applies it for this
    /// session and says so, like the other view preferences.
    fn store_settings(&mut self, next: Settings) {
        // Only the fold key is written; an unusable file is left alone (WR-03, WR-05).
        let save_error = match &self.root {
            Ok(root) => settings::save_section_state(root, &next).err(),
            Err(_) => None,
        };
        self.settings = next;
        if let Some(err) = save_error {
            self.config_message = Some((format!("Could not save ff-settings.json: {err}"), true));
        }
    }

    /// Remember that a section of the active filter is folded or open. A view preference: it
    /// never touches the loaded filter, the dirty flag or the CONFIG pending state.
    fn set_section_collapsed(&mut self, name: &str, economy: bool, collapsed: bool) {
        let key = settings::section_key(name, economy);
        let next =
            self.settings
                .set_section_collapsed(self.settings.filter_name(), &key, collapsed);
        self.store_settings(next);
    }

    /// Open the collapsed section holding rule `id`, so a jump or a move that targets it shows
    /// something (WR-02).
    fn reveal_rule(&mut self, id: BlockId) {
        let Some(working) = &self.working else {
            return;
        };
        let name = section_to_reveal(working, &self.settings, id);
        if let Some(name) = name {
            self.set_section_collapsed(&name, false, false);
        }
    }

    /// Persist the reload-after-Apply preference at once. A view preference, not a CONFIG draft
    /// edit: it never touches the loaded filter or the CONFIG pending state.
    fn set_reload_after_apply(&mut self, on: bool) {
        let next = settings_with_reload(&self.settings, on);
        self.save_view_setting(next);
    }

    /// Saves `next` at once; a failed save still applies it for the session and says so.
    fn save_view_setting(&mut self, next: Settings) {
        let save_error = match &self.root {
            Ok(root) => settings::save(root, &next).err(),
            Err(_) => None,
        };
        self.settings = next;
        if let Some(err) = save_error {
            self.config_message = Some((format!("Could not save ff-settings.json: {err}"), true));
        }
    }

    /// Saves one key of `next` at once, every other key of the file left as it is. An unusable
    /// file is never replaced: the choice applies for the session and the status line says it
    /// was not saved, as does a failed save.
    fn save_view_key(
        &mut self,
        next: Settings,
        save: fn(&Path, &Settings) -> std::io::Result<bool>,
    ) {
        let save_error = match &self.root {
            Ok(root) => match save(root, &next) {
                Ok(true) => None,
                Ok(false) => Some(UNUSABLE_SETTINGS_NOT_SAVED.to_string()),
                Err(e) => Some(e.to_string()),
            },
            Err(_) => None,
        };
        self.settings = next;
        if let Some(err) = save_error {
            self.config_message = Some((format!("Could not save ff-settings.json: {err}"), true));
        }
    }

    /// Persist the picked economy league at once. Never touches the loaded filter or CONFIG
    /// edits.
    fn set_league(&mut self, name: &str) {
        let next = settings_with_league(&self.settings, name);
        self.save_view_key(next, settings::save_league);
    }

    /// Persist the price source at once (`true` = Live) and re-gate the tier cutoffs.
    fn set_price_source(&mut self, live: bool) {
        let next = settings_with_price_source(&self.settings, live);
        self.save_view_key(next, settings::save_price_source);
        self.sync_tiers_in_use();
        let rules_dirty = self.working.as_ref().is_some_and(|w| w.is_dirty());
        let style_pending = self
            .working
            .as_mut()
            .is_some_and(|w| w.style_rewrite_pending());
        let config_dirty = self.config.as_ref().is_ok_and(|c| c.is_dirty());
        let other_dirty = apply::combined_dirty(rules_dirty, style_pending, config_dirty);
        let pending = self.source_pending();
        if (pending || other_dirty) && self.apply_state == ApplyState::Applied {
            self.apply_state = ApplyState::Unapplied;
        } else if !pending && !other_dirty {
            self.apply_state = apply::state_after_revert(self.applied_this_session, false);
        }
    }

    /// Whether the loaded filter has an economy section (in the file, or staged to be added).
    fn has_economy_section(&self) -> bool {
        self.working
            .as_ref()
            .is_some_and(|w| matches!(w.economy(), EconomyView::Present { .. }))
    }

    /// A price-source switch that makes Apply write different economy blocks than the file has.
    /// Counts as an unapplied change (marker, indicator, Apply, Quit prompt) but never as
    /// something Revert undoes: the setting is saved at once.
    fn source_pending(&self) -> bool {
        super::leagues::source_pending(
            self.applied_policy,
            self.settings.price_policy(),
            self.economy_info.live,
            self.has_economy_section(),
        )
    }

    /// The decided league UI for the Economy tab.
    fn league_view(&self) -> super::leagues::LeagueView {
        super::leagues::view(&super::leagues::ViewInputs {
            saved: self.settings.saved_league(),
            state: self.leagues.state(),
            fetching: self.leagues.is_fetching(),
            prices_league: self.economy_info.league.as_deref(),
            fallback: self.settings.price_policy() == PriceSourcePolicy::ForceFallback,
            syncing: self.economy.is_running(),
        })
    }

    /// What the footer knows about the filter, for [`apply::footer_actions`]. The economy part
    /// comes from the projected working view, so a staged add/remove is reflected immediately.
    fn filter_state(&self) -> FilterState {
        match (&self.filter, &self.working) {
            (Ok(loaded), Some(working)) => match working.economy() {
                EconomyView::Damaged(_) => FilterState::Damaged,
                EconomyView::Present { .. } => FilterState::HasSection,
                EconomyView::Absent => {
                    if !loaded.existed() && working.rows().is_empty() {
                        FilterState::Absent
                    } else {
                        FilterState::NoSection
                    }
                }
            },
            _ => FilterState::LoadFailed,
        }
    }

    /// The one Apply path, for APPLY and OVERWRITE on either tab: [`apply_everything`] saves the
    /// CONFIG edits and the filter together, then the in-game reload is decided as before and the
    /// list's view state is re-keyed to the re-parsed document. `overwrite` is OVERWRITE after a
    /// changed-on-disk refusal; it otherwise behaves exactly like APPLY, in-game reload included.
    fn run_apply(&mut self, ctx: &egui::Context, overwrite: bool) -> bool {
        self.update_message = None;
        self.sync_tiers_in_use();
        let Ok(root) = &self.root else {
            return false;
        };
        // Ids reset on re-parse, so remember the expanded row and the Defences markers by
        // position instead (review WR-01: Apply must not flip a Defences card to Simple).
        let kept = self
            .working
            .as_ref()
            .map(|w| self.list.kept_positions(w))
            .unwrap_or_default();
        // Without an in-memory CONFIG the styles are re-read at every Apply, so a styles.json
        // edited a moment ago counts.
        if self.config.is_err()
            && let Some(working) = &mut self.working
        {
            self.styles_stamp = styles_stamp_at(root);
            working.set_styles(load_styles_at(root));
        }
        let filter_part = match (&mut self.filter, &mut self.working) {
            (Ok(loaded), Some(working)) => Some((loaded, working)),
            _ => None,
        };
        let result = apply_everything(
            root,
            self.config.as_mut().ok(),
            filter_part,
            self.settings.price_policy(),
            overwrite,
        );
        let AllResult {
            outcome,
            extras,
            styles_written,
            tiers_written,
            changed,
        } = result;
        let applied_path = match outcome {
            AllOutcome::Failed(message) => {
                self.apply_message = Some((message, w::StatusKind::Error));
                self.apply_state = self.apply_state.on_apply_failure();
                if !changed.is_empty() {
                    self.changed_on_disk = changed;
                    self.changed_popup = true;
                }
                return false;
            }
            AllOutcome::Applied { filter } => filter,
        };
        let prefix = apply::config_saved_prefix(styles_written, tiers_written);
        match applied_path {
            // Every successful filter Apply decides, identical bytes included; the setting is
            // checked before any window search.
            Some(path) => {
                let decision = chat::decide(
                    &ApplyOutcome::Written(path.clone()),
                    self.settings.reload_after_apply(),
                    self.reload.is_running(),
                    win::find_game_window,
                );
                // No cached overlay window means no clipboard owner: skip rather than fall back
                // to an ownerless clipboard, whose write fails after emptying it (review WR-02).
                let decision =
                    chat::with_owner(decision, self.window.as_ref().map(win::clipboard_owner));
                match decision {
                    Some(chat::Decision::Send((game, owner))) => {
                        let interim =
                            format!("{prefix}{}{extras}", apply::reloading_message(&path));
                        self.apply_message = applied_message(interim.clone());
                        let repaint = ctx.clone();
                        let started = self.reload.start(
                            move || {
                                let mut sink = chat::WinSink::new(game, owner);
                                chat::execute(&mut sink, &chat::reload_steps())
                            },
                            move || repaint.request_repaint(),
                        );
                        if started {
                            self.reload_pending = Some(PendingReload {
                                path: path.clone(),
                                prefix: prefix.clone(),
                                extras: extras.clone(),
                                interim,
                                warning: false,
                            });
                        } else {
                            self.apply_message = applied_message(format!(
                                "{prefix}{}",
                                skipped_reload_message(&path, &extras, chat::SkipReason::Busy)
                            ));
                        }
                    }
                    Some(chat::Decision::Skip(reason)) => {
                        self.apply_message = applied_message(format!(
                            "{prefix}{}",
                            skipped_reload_message(&path, &extras, reason)
                        ));
                    }
                    None => {
                        self.apply_message = applied_message(format!(
                            "{prefix}{}{extras}",
                            apply::success_message(&path)
                        ));
                    }
                }
            }
            // No filter part: the config files alone, nothing to reload in game.
            None => {
                self.apply_message = applied_message(format!(
                    "{}{extras}",
                    apply::config_only_message(styles_written, tiers_written)
                ));
            }
        }
        self.apply_state = self.apply_state.on_apply_success();
        self.applied_policy = self.settings.price_policy();
        self.config_message = None;
        self.config_ui.reset_for_new_styles();
        self.styles_stamp = styles_stamp_at(root);
        self.changed_on_disk.clear();
        self.applied_this_session = true;
        self.list_status = None;
        self.list.rekey_after_apply(kept, self.working.as_ref());
        true
    }

    /// Applies deferred row operations to the working filter and updates the status the way a
    /// list click does.
    fn run_row_actions(&mut self, row_actions: Vec<RowAction>) {
        let mut fold = self.settings.clone();
        if let Some(working) = &mut self.working {
            let (changed, status) = apply_row_actions_folding(
                working,
                &mut self.list,
                &mut self.query,
                row_actions,
                Some(&mut fold),
            );
            if changed {
                // An action can report `changed` without altering the bytes (clearing an
                // absent name, moving a rule and back); only a real difference from the
                // applied text makes the indicator stale.
                if working.is_dirty() {
                    self.apply_state = ApplyState::Unapplied;
                }
                // An event is cleared by the next edit, a CONFIG message included (WR-01).
                self.clear_events_on_edit();
            }
            if status.is_some() {
                self.list_status = status;
            }
        }
        // Keeps the stored folded state in step with the section actions that ran (WR-04).
        if fold != self.settings {
            self.store_settings(fold);
        }
        // A rule moved into a collapsed neighbour stays in view (WR-02).
        if let Some(id) = self.list.pending_scroll {
            self.reveal_rule(id);
        }
    }

    /// Hiding the window clears every event message (the Apply outcome, the CONFIG message, the
    /// list status, a finished sync), so a stale report never outlives its question or hides the
    /// standing lines behind it (grammar gate, load error, the out-of-date indicator; review
    /// WR-01). The one exception is a reload's interim line ([`apply_message_after_conceal`]).
    fn clear_events_on_conceal(&mut self) {
        self.apply_message = self.apply_message.take().and_then(|(text, kind)| {
            apply_message_after_conceal(Some(text), self.reload_pending.as_ref())
                .map(|text| (text, kind))
        });
        self.config_message = None;
        self.list_status = None;
        self.update_message = None;
        self.economy.clear_message();
    }

    /// Start the update check. The only place a check begins, and only the About tab's button
    /// calls it: never at start-up, never on a timer. A new user action supersedes older event
    /// messages.
    fn start_update_check(&mut self) {
        self.clear_events_on_edit();
        self.economy.clear_message();
        if self.update.start(|progress| {
            crate::update::check_for_update_blocking(progress.work_dir(), &mut |v| {
                progress.downloading(v)
            })
        }) {
            let (text, kind) = crate::update::checking_message();
            self.update_message = Some((text, update_kind(kind)));
        }
    }

    /// An edit or a revert on either tab clears every event message (the Apply outcome, the list
    /// status, the CONFIG message) before setting its own, so a stale report never hides the
    /// standing lines or the action's own message (review WR-01, iterations 1 and 2).
    fn clear_events_on_edit(&mut self) {
        self.apply_message = None;
        self.update_message = None;
        self.list_status = None;
        self.config_message = None;
    }

    /// Whether Overwrite is live: the filter's own decision, or a refusal only the config files
    /// can resolve, which stays overwritable even when the filter part is not writable (WR-03).
    fn overwrite_decision(&mut self) -> bool {
        let state = self.filter_state();
        let style_pending = self
            .working
            .as_mut()
            .is_some_and(|w| w.style_rewrite_pending());
        let style_pending = style_pending || self.source_pending();
        let config_dirty = self.config.as_ref().is_ok_and(|c| c.is_dirty());
        let rules_dirty = self.working.as_ref().is_some_and(|w| w.is_dirty());
        let blocking = self.working.as_ref().map_or(0, |w| w.blocking().len());
        let changed = !self.changed_on_disk.is_empty();
        let filter_overwrite = apply::footer_actions_combined(
            self.root.is_ok(),
            state,
            changed,
            rules_dirty || style_pending,
            config_dirty,
            blocking,
        )
        .overwrite;
        apply::overwrite_enabled(
            filter_overwrite,
            self.root.is_ok(),
            self.config.as_ref().is_ok_and(|c| c.validate().is_ok()),
            self.config_invalid(),
            state,
            changed,
            apply::combined_dirty(rules_dirty, style_pending, config_dirty),
            blocking,
        )
    }

    /// A loaded config that fails validation: every Apply (Overwrite included) would refuse it.
    fn config_invalid(&self) -> bool {
        self.config.as_ref().is_ok_and(|c| c.validate().is_err())
    }

    /// Why Overwrite is unavailable, for the changed-on-disk popup (WR-02, iteration 2); the
    /// same reason chain as the bar slot's tooltip.
    fn overwrite_block(&self) -> shell::OverwriteBlock {
        shell::OverwriteBlock::from_inputs(ConfigBarInputs {
            tier_errors: self.config.as_ref().is_ok_and(|c| c.has_tier_errors()),
            config_invalid: self.config_invalid(),
            rules_blocking: self
                .working
                .as_ref()
                .is_some_and(|w| !w.blocking().is_empty()),
            damaged: self.filter_state() == FilterState::Damaged,
            ..ConfigBarInputs::default()
        })
    }

    /// Which popups want to be open this frame.
    fn popup_flags(&self, quit_requested: bool) -> shell::PopupFlags {
        shell::PopupFlags {
            quit: self.quit_confirm && !quit_requested,
            changed: self.changed_popup && !self.changed_on_disk.is_empty(),
            switch: self.pending_switch.is_some() || self.pending_folder.is_some(),
            delete_rule: self.list.confirm_delete.is_some(),
            delete_style: self.config_ui.delete_armed.is_some(),
            remove_economy: self.list.confirm_economy_remove,
            reset_styles: self.config_ui.reset_armed == Some(RevertScope::Styles),
            reset_tiers: self.config_ui.reset_armed == Some(RevertScope::Tiers),
            discard_config: self.config_ui.revert_armed && self.config_discard_count() > 0,
            discard_rules: self.revert_rules_armed && self.rules_revertible(),
        }
    }

    /// What the armed Discard-config popup would discard: the scope's config edits, plus one
    /// for a staged economy add/remove on the ECONOMY scope.
    fn config_discard_count(&self) -> usize {
        let scope = self.config_ui.revert_scope;
        let edits = self
            .config
            .as_ref()
            .map_or(0, |c| c.scope_dirty_count(scope));
        edits + usize::from(scope == RevertScope::Tiers && self.economy_staged())
    }

    fn economy_staged(&self) -> bool {
        self.working.as_ref().is_some_and(|w| w.economy_staged())
    }

    /// Whether the RULES Revert has anything to discard: rule edits only. A staged economy
    /// add/remove belongs to the ECONOMY Revert, and a pending style rewrite comes from
    /// `styles.json`, which a rule revert cannot undo. The one predicate behind both the bar's
    /// RULES Revert enablement and the Discard-rule-changes popup, so a click always opens it.
    fn rules_revertible(&self) -> bool {
        self.working.as_ref().is_some_and(|w| w.rules_edited())
    }

    /// The RULES bar Revert enablement: [`Self::rules_revertible`], except in the states where no
    /// Revert applies (a changed-on-disk refusal, a filter that failed to load).
    fn rules_revert_live(&self) -> bool {
        apply::footer_actions(
            self.root.is_ok(),
            self.filter_state(),
            !self.changed_on_disk.is_empty(),
            self.rules_revertible(),
            0,
        )
        .revert
    }

    /// The text and buttons of an open popup, from the current state.
    fn popup_spec_for(&self, popup: Popup) -> shell::PopupSpec {
        match popup {
            Popup::Quit => shell::popup_spec(PopupData::Quit {
                tier_edits_only: self.quit_tier_edits_only,
            }),
            Popup::ChangedOnDisk => {
                let fallback = Path::new("the filter");
                let path = self.filter.as_ref().ok().map_or(fallback, |l| l.path());
                let files: Vec<String> =
                    self.changed_on_disk.iter().map(|f| f.label(path)).collect();
                shell::popup_spec(PopupData::ChangedOnDisk {
                    files: &files,
                    overwrite: self.overwrite_live,
                    block: self.overwrite_block(),
                })
            }
            Popup::SwitchFilter => {
                let target = match &self.pending_folder {
                    Some(dir) => dir.display().to_string(),
                    None => self.pending_switch.clone().unwrap_or_default(),
                };
                shell::popup_spec(PopupData::SwitchFilter { target: &target })
            }
            Popup::DeleteRule => {
                let row = self
                    .list
                    .confirm_delete
                    .and_then(|id| self.working.as_ref().and_then(|w| w.row(id)));
                match row {
                    Some(row) => {
                        let label = row.name.clone().unwrap_or_else(|| {
                            row.label
                                .trim_start_matches(|c: char| c.is_ascii_digit() || c == ' ')
                                .to_string()
                        });
                        shell::popup_spec(PopupData::DeleteRule {
                            index: row.index,
                            label: &label,
                            blocks: row.group_size,
                        })
                    }
                    None => shell::popup_spec(PopupData::DeleteRule {
                        index: 0,
                        label: "",
                        blocks: 1,
                    }),
                }
            }
            Popup::DeleteStyle => {
                let name = self.config_ui.delete_armed.as_deref().unwrap_or("");
                let users = self
                    .working
                    .as_ref()
                    .and_then(|w| w.style_users().get(name).copied())
                    .unwrap_or(0);
                let mut prompt = config_state::delete_confirm_text(name, users);
                if self.config_refs.contains(name) {
                    prompt.push('\n');
                    prompt.push_str(&config_state::config_warning(name));
                }
                shell::popup_spec(PopupData::DeleteStyle { prompt })
            }
            Popup::RemoveEconomy => shell::popup_spec(PopupData::RemoveEconomy),
            Popup::ResetStyles => shell::popup_spec(PopupData::ResetStyles),
            Popup::ResetTiers => shell::popup_spec(PopupData::ResetTiers),
            Popup::DiscardConfig => {
                let scope = self.config_ui.revert_scope;
                let economy = scope == RevertScope::Tiers;
                shell::popup_spec(PopupData::DiscardConfig {
                    count: self
                        .config
                        .as_ref()
                        .map_or(0, |c| c.scope_dirty_count(scope)),
                    economy,
                    staged: economy && self.economy_staged(),
                })
            }
            Popup::DiscardRules => {
                let name = self
                    .filter
                    .as_ref()
                    .ok()
                    .and_then(|l| l.path().file_name())
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or_default();
                shell::popup_spec(PopupData::DiscardRules { filter: &name })
            }
        }
    }

    /// Acts on a popup button (its index in the spec). Every popup closes on any choice; only
    /// the explicit destructive or confirming button runs the v0.5 handler.
    fn dispatch_popup(&mut self, ctx: &egui::Context, popup: Popup, choice: usize) {
        let on_config = self.tab == ShellTab::Config;
        match popup {
            Popup::Quit => {
                let choice = shell::quit_popup_choice(self.quit_tier_edits_only, choice);
                let quit_choice = match choice {
                    0 => apply::QuitChoice::ApplyAndQuit,
                    1 => apply::QuitChoice::QuitWithoutApplying,
                    _ => apply::QuitChoice::Cancel,
                };
                let dirty = self.quit_dirty();
                match apply::quit_step(dirty, true, apply::QuitEvent::Choice(quit_choice)) {
                    apply::QuitStep::CloseConfirm => {
                        self.quit_confirm = false;
                        self.update.cancel_install();
                    }
                    apply::QuitStep::QuitNow => {
                        // Deferred to the top of the next frame so the Phase 22 order holds:
                        // shutdown before that frame's instance show-request poll (WR-08).
                        self.quit_confirm = false;
                        self.quit_next_frame = true;
                        ctx.request_repaint();
                    }
                    apply::QuitStep::RunApply => {
                        self.quit_confirm = false;
                        // Unused tier edits alone leave nothing to write: no Apply, no reload.
                        let ok = !self.applicable_dirty() || self.run_apply(ctx, false);
                        if ok && self.has_unwritable_tier_edits() {
                            // No Apply can write cutoffs while Fallback is on: stay open with
                            // them kept rather than quit and lose them.
                            show_not_quit_warning(
                                &mut self.apply_message,
                                self.reload_pending.as_mut(),
                            );
                        // A refusal leaves the window open showing it (Overwrite/Reload as today).
                        } else if apply::quit_step(
                            dirty,
                            false,
                            apply::QuitEvent::ApplyFinished { ok },
                        ) == apply::QuitStep::QuitWhenIdle
                        {
                            self.quit_after_apply = true;
                        } else {
                            // A refused Apply keeps the window open: no install on a later quit.
                            self.update.cancel_install();
                        }
                    }
                    _ => {}
                }
            }
            Popup::ChangedOnDisk => {
                self.changed_popup = false;
                let choice = shell::changed_on_disk_choice(self.overwrite_live, choice);
                match (choice, on_config) {
                    (0, true) => self.handle_config_actions(ctx, vec![ConfigAction::Reload]),
                    (0, false) => self.reload_pressed(),
                    (1, true) => self.handle_config_actions(ctx, vec![ConfigAction::Overwrite]),
                    (1, false) => {
                        self.run_apply(ctx, true);
                    }
                    _ => {}
                }
            }
            Popup::SwitchFilter => {
                let action = if choice == 0 {
                    ConfigAction::ConfirmSwitch
                } else {
                    ConfigAction::CancelSwitch
                };
                self.handle_config_actions(ctx, vec![action]);
            }
            Popup::DeleteRule => {
                if let Some(id) = self.list.confirm_delete.take()
                    && choice == 0
                {
                    self.run_row_actions(vec![RowAction::Delete(id)]);
                }
            }
            Popup::DeleteStyle => {
                if let Some(name) = self.config_ui.delete_armed.take()
                    && choice == 0
                {
                    self.handle_config_actions(ctx, vec![ConfigAction::DeleteStyle(name)]);
                }
            }
            Popup::RemoveEconomy => {
                self.list.confirm_economy_remove = false;
                if choice == 0 {
                    self.stage_remove_economy();
                }
            }
            Popup::ResetStyles | Popup::ResetTiers => {
                self.config_ui.reset_armed = None;
                if choice == 0 {
                    let action = if popup == Popup::ResetStyles {
                        ConfigAction::ResetStyles
                    } else {
                        ConfigAction::ResetTiers
                    };
                    self.handle_config_actions(ctx, vec![action]);
                }
            }
            Popup::DiscardConfig => {
                self.config_ui.revert_armed = false;
                if choice == 0 {
                    let scope = self.config_ui.revert_scope;
                    // ECONOMY with only a staged op has no tier edits to revert (and no tier
                    // message to show).
                    let scoped_dirty = self
                        .config
                        .as_ref()
                        .is_ok_and(|c| c.scope_dirty_count(scope) > 0);
                    if scope == RevertScope::Styles || scoped_dirty {
                        self.handle_config_actions(ctx, vec![ConfigAction::RevertScoped(scope)]);
                    }
                    // The ECONOMY Revert owns a staged economy op too; it goes with the tiers.
                    if scope == RevertScope::Tiers && self.economy_staged() {
                        self.unstage_economy_pressed();
                    }
                }
            }
            Popup::DiscardRules => {
                self.revert_rules_armed = false;
                if choice == 0 {
                    self.revert();
                }
            }
        }
    }

    /// A quit request (tray Quit or the installer's `ff --quit`): quits at once when nothing
    /// is unapplied, otherwise opens the Unapplied changes popup. Returns true when the
    /// shutdown began. While an in-game reload is still being sent (right after an Apply) the
    /// shutdown waits for it, like Apply & quit, so the paste and the clipboard restore are
    /// never cut off.
    fn request_quit(&mut self) -> bool {
        let dirty = self.quit_dirty();
        let step = apply::quit_step(dirty, self.quit_confirm, apply::QuitEvent::TrayQuit);
        if step == apply::QuitStep::QuitNow {
            if self.reload.is_running() {
                self.quit_after_reload = true;
                return false;
            }
            self.quit_after_reload = false;
            self.begin_shutdown();
            return true;
        }
        if apply::quit_step_shows_confirm(step) {
            // The popup now owns the decision: its Cancel must not be followed by a quit.
            self.quit_after_reload = false;
            self.quit_confirm = true;
            // Reveal the window the way tray Show does (also when the window was hidden while
            // the confirm was already up).
            if self.visibility == Visibility::Hidden {
                self.toggle_requested = true;
            }
        }
        false
    }

    /// Runs a quit that waited for the in-game reload, once that reload has finished. Apply &
    /// quit leaves at once; a deferred clean quit is asked again, so edits made while it waited
    /// (the window summoned again, a `styles.json` change on disk) open the Unapplied changes
    /// popup instead of being lost. Returns true when the shutdown began.
    fn run_deferred_quit(&mut self) -> bool {
        if self.reload.is_running() {
            return false;
        }
        if self.quit_after_apply {
            self.quit_after_apply = false;
            self.quit_after_reload = false;
            self.begin_shutdown();
            return true;
        }
        if self.quit_after_reload {
            self.quit_after_reload = false;
            return self.request_quit();
        }
        false
    }

    /// Anything the Quit confirm guards: unapplied work, plus tier edits kept unused under
    /// Fallback (no Apply writes them, but quitting loses them).
    fn quit_dirty(&mut self) -> bool {
        self.applicable_dirty() || self.has_unwritable_tier_edits()
    }

    /// Decides, while the Quit confirm shows, whether its only pending work is unused tier edits
    /// (the popup then offers no Apply & quit, which could only stay open).
    fn refresh_quit_tier_edits_only(&mut self) {
        self.quit_tier_edits_only =
            self.quit_confirm && self.has_unwritable_tier_edits() && !self.applicable_dirty();
    }

    /// Tier cutoff edits that stay pending because the Fallback price source does not use them.
    fn has_unwritable_tier_edits(&self) -> bool {
        self.config
            .as_ref()
            .is_ok_and(|c| c.has_unwritable_tier_edits())
    }

    /// Unapplied work an Apply would write: rule edits, a pending style rewrite or source
    /// switch, CONFIG edits.
    fn applicable_dirty(&mut self) -> bool {
        let style_pending = self
            .working
            .as_mut()
            .is_some_and(|w| w.style_rewrite_pending());
        let style_pending = style_pending || self.source_pending();
        let config_dirty = self.config.as_ref().is_ok_and(|c| c.is_dirty());
        let rules_dirty = self.working.as_ref().is_some_and(|w| w.is_dirty());
        apply::combined_dirty(rules_dirty, style_pending, config_dirty)
    }

    /// The Phase 22 release order: give up what a new instance must own (the hotkey, then the
    /// tray icon) before the name it races for. If the name went first, a relaunch could win it
    /// while `Ctrl+Shift+Z` was still registered here, fail to register it, and run with no
    /// hotkey (review iteration 3, WR-01). The caller then sets `quit_requested` for the single
    /// deferred viewport Close.
    fn begin_shutdown(&mut self) {
        if let Some(manager) = self.hotkey_manager.take() {
            hotkey::release_toggle(manager);
        }
        self.tray.release();
        // Release the instance name now, before this frame's show-request poll: from here on a
        // relaunch becomes the new first instance rather than waiting on (or being acked by) a
        // process that is about to close.
        self.instance = None;
        // Hand a clicked-for installer to `launch()`, which starts it after the event loop has
        // returned; any other quit deletes the download here.
        if let Some(path) = self.update.shutdown()
            && let Ok(mut slot) = self.pending_install.lock()
        {
            *slot = Some(path);
        }
    }

    /// The Install click: re-checks the installer, sets the install-on-quit intent and asks to
    /// quit through the normal path (quits at once when clean, else the Unapplied changes popup).
    /// Returns true when the shutdown began. Never starts the installer.
    fn install_and_quit(&mut self) -> bool {
        self.clear_events_on_edit();
        match self.update.begin_install() {
            InstallClick::Failed((text, kind)) => {
                self.update_message = Some((text, update_kind(kind)));
                false
            }
            InstallClick::NotReady => false,
            InstallClick::Proceed => self.request_quit(),
        }
    }

    /// RULES REVERT: discard the rule edits. A staged economy add/remove is the ECONOMY tab's and
    /// survives (WR-04).
    fn revert(&mut self) {
        if self.working.is_none() {
            return;
        }
        self.clear_events_on_edit();
        let Some(working) = &mut self.working else {
            return;
        };
        working.revert_rules();
        let still_dirty = working.is_dirty();
        self.list.clear_ids();
        self.list_status = Some(apply::REVERTED_MESSAGE.to_string());
        // Per tab: CONFIG edits and a staged economy op survive a RULES revert, so the state may
        // stay out of date.
        let config_dirty = self.config.as_ref().is_ok_and(|c| c.is_dirty());
        self.apply_state = apply::state_after_revert(
            self.applied_this_session,
            config_dirty || still_dirty || self.source_pending(),
        );
    }

    /// The ECONOMY bar Revert: opens the Discard popup whenever there are tier edits or a staged
    /// economy add/remove; both go together when the user confirms. Nothing is dropped unasked.
    fn economy_revert_pressed(&mut self) {
        let tiers_dirty = self.config.as_ref().is_ok_and(|c| c.tiers_dirty());
        if tiers_dirty || self.economy_staged() {
            self.config_ui.revert_scope = RevertScope::Tiers;
            self.config_ui.revert_armed = true;
        }
    }

    /// ECONOMY Revert, staged part: drops a staged economy add/remove only.
    fn unstage_economy_pressed(&mut self) {
        self.clear_events_on_edit();
        let Some(working) = &mut self.working else {
            return;
        };
        working.unstage_economy();
        let rules_dirty = working.is_dirty();
        let style_pending = working.style_rewrite_pending();
        let style_pending = style_pending || self.source_pending();
        let config_dirty = self.config.as_ref().is_ok_and(|c| c.is_dirty());
        self.apply_state = apply::state_after_revert(
            self.applied_this_session,
            rules_dirty || config_dirty || style_pending,
        );
    }

    /// Stages adding the economy section; it is written by the next APPLY.
    fn stage_add_economy(&mut self) {
        if self.root.is_err() || self.working.is_none() {
            return;
        }
        self.clear_events_on_edit();
        let (Ok(root), Some(working)) = (&self.root, &mut self.working) else {
            return;
        };
        let config = self.config.as_ref().ok();
        let policy = self.settings.price_policy();
        match staged_economy_body(root, config, policy) {
            Ok(body) => {
                if working.stage_add_economy(&body) {
                    self.staged_econ_inputs = config.map(|c| {
                        (
                            c.pending_styles().clone(),
                            c.pending_tiers().to_vec(),
                            policy,
                        )
                    });
                    self.apply_state = ApplyState::Unapplied;
                } else {
                    self.list_status = Some(PREVIEW_FAILED_STATUS.to_string());
                }
            }
            Err(err) => self.list_status = Some(err),
        }
    }

    /// Re-prices the staged economy-add preview when the pending styles or tiers changed since it
    /// was built. On a pricing error the previous preview stays and the error is shown.
    fn refresh_staged_economy(&mut self) {
        let source_pending = self.source_pending();
        let (Ok(root), Ok(config), Some(working)) =
            (&self.root, &self.config, self.working.as_mut())
        else {
            return;
        };
        let plan = working.economy_plan();
        // A section already in the file is shown re-priced only while CONFIG has style or tier
        // edits; without them it is exactly the file's blocks.
        let refresh = plan == rules::EconomyPlan::Refresh;
        let wanted = match plan {
            rules::EconomyPlan::Add => true,
            rules::EconomyPlan::Refresh => {
                config.styles_dirty() || config.tiers_dirty() || source_pending
            }
            _ => false,
        };
        if !wanted {
            self.staged_econ_inputs = None;
            self.econ_preview_failed = false;
            if refresh && working.has_economy_preview() {
                let econ_open = self.list.expanded.is_some_and(rules::is_preview_id);
                working.set_economy_refresh(None);
                if econ_open {
                    self.list.expanded = None;
                }
            }
            return;
        }
        let (styles, tiers) = (config.pending_styles(), config.pending_tiers());
        let policy = self.settings.price_policy();
        if (working.has_economy_preview() || self.econ_preview_failed)
            && self
                .staged_econ_inputs
                .as_ref()
                .is_some_and(|(s, t, p)| s == styles && t.as_slice() == tiers && *p == policy)
        {
            return;
        }
        self.staged_econ_inputs = Some((styles.clone(), tiers.to_vec(), policy));
        // Rows switch documents (file to preview, or one preview to the next, whose ids restart):
        // an open economy row would point elsewhere.
        let econ_open = self.list.expanded.is_some_and(|id| {
            rules::is_preview_id(id) || working.economy_rows().iter().any(|r| r.id == id)
        });
        let shown = match staged_economy_body(root, Some(config), policy) {
            Ok(body) if refresh => working.set_economy_refresh(Some(&body)),
            Ok(body) => working.set_staged_economy_body(&body),
            Err(err) => {
                // Refresh: the file's blocks keep showing.
                if refresh {
                    working.set_economy_refresh(None);
                }
                self.econ_preview_failed = true;
                self.list_status = Some(err);
                return;
            }
        };
        self.econ_preview_failed = !shown;
        if !shown {
            self.list_status = Some(PREVIEW_FAILED_STATUS.to_string());
        }
        if econ_open {
            self.list.expanded = None;
        }
    }

    /// Stages removing the economy section; it is written by the next APPLY.
    fn stage_remove_economy(&mut self) {
        if self.working.is_some() {
            self.clear_events_on_edit();
        }
        if let Some(working) = &mut self.working {
            working.stage_remove_economy();
            self.apply_state = ApplyState::Unapplied;
        }
    }

    /// RELOAD: re-resolves the app root when it could not be resolved, re-reads the filter and
    /// discards in-memory state (D-08). A resolved root is kept (the environment cannot change
    /// under a running process); the game folder is re-resolved outside unit tests.
    /// Marks the tier cutoffs unused while the saved price source is Fallback, so their errors
    /// cannot gate Apply (they return when Live is chosen again).
    fn sync_tiers_in_use(&mut self) {
        let in_use = self.settings.price_policy() != PriceSourcePolicy::ForceFallback;
        if let Ok(config) = &mut self.config {
            config.set_tiers_in_use(in_use);
        }
    }

    fn reload(&mut self) {
        let old_root = self.root.clone();
        let open_filter = self.settings.filter_name().to_string();
        let left = self.open_filter_path();
        if self.root.is_err() {
            self.root = resolve_root();
        }
        let mut filter_fell_back = false;
        let settings_warning = match &self.root {
            Ok(root) => {
                let loaded = settings::load_detailed(root);
                let (settings, warning) = (loaded.settings, loaded.warning);
                filter_fell_back = loaded.filter_fell_back;
                // A momentarily unreadable file must not reset the session's chosen filter (nor
                // drop unknown keys held in memory), but a reload-off from the file always wins.
                self.settings = settings_after_reload(
                    &self.settings,
                    settings,
                    warning.is_some(),
                    old_root.as_ref() == Ok(root),
                );
                warning
            }
            Err(_) => {
                self.settings = Settings::default();
                None
            }
        };
        // The game folder follows the (just re-read) settings; unit tests keep the folder they
        // were built with.
        #[cfg(not(test))]
        {
            self.game_dir = crate::locations::game_dir(self.settings.game_folder_path());
        }
        // No saved filter: keep the one that was open when the folder still lists it. A load
        // whose filter name fell back either kept the session's filter or says it uses
        // `dynamic.filter`; a warning about another key changes nothing here.
        if !filter_fell_back {
            choose_unsaved_filter(&mut self.settings, self.game_dir.as_deref(), &open_filter);
        }
        // The re-read settings or folder may name another filter than the one open before.
        if self.open_filter_path() != left {
            self.rebind_applied_policy(left);
        }
        self.reset_filter_state();
        self.refresh_inputs();
        if settings_warning.is_some() {
            self.list_status = settings_warning;
        }
        self.refresh_filters();
        // Config edits survive a RULES reload unless the root moved or the load had failed.
        if self.root != old_root || self.config.is_err() {
            self.config = match &self.root {
                Ok(root) => ConfigState::load(root),
                Err(message) => Err(message.clone()),
            };
        }
        self.sync_tiers_in_use();
    }

    /// RELOAD on either tab: after a changed-on-disk refusal only the files that changed are
    /// re-read; otherwise everything is ([`Self::reload`]).
    fn reload_pressed(&mut self) {
        if self.changed_on_disk.is_empty() {
            self.reload();
        } else {
            self.reload_changed();
        }
    }

    /// Re-reads the files a refused Apply found changed, discarding only the edits of the tab
    /// that owns each one (the filter: RULES edits; styles.json/tiers.json: CONFIG edits).
    fn reload_changed(&mut self) {
        let changed = std::mem::take(&mut self.changed_on_disk);
        let Ok(root) = self.root.clone() else {
            return;
        };
        reload_changed_files(
            &root,
            &changed,
            &mut self.config,
            &mut self.filter,
            &mut self.working,
        );
        self.sync_tiers_in_use();
        if changed.contains(&DiskFile::Filter) {
            self.reset_filter_view();
        }
        if changed
            .iter()
            .any(|f| matches!(f, DiskFile::Styles | DiskFile::Tiers))
        {
            self.config_ui.reset_for_new_styles();
            self.config_message = None;
            self.apply_message = None;
        }
        self.styles_stamp = styles_stamp_at(&root);
        // The tab whose edits were kept may still hold unapplied changes.
        let kept_dirty = self.config.as_ref().is_ok_and(|c| c.is_dirty())
            || self.working.as_ref().is_some_and(|w| w.is_dirty());
        self.apply_state = apply::state_after_revert(
            self.applied_this_session,
            kept_dirty || self.source_pending(),
        );
    }

    /// Applies the CONFIG tab's collected actions. Filter actions are handled by the filter
    /// selection code; everything else edits the `ConfigState`.
    fn handle_config_actions(&mut self, ctx: &egui::Context, actions: Vec<ConfigAction>) {
        // A config edit makes the last Apply message stale, as a rule edit does on RULES.
        if actions.iter().any(|a| {
            matches!(
                a,
                ConfigAction::AddStyle(_)
                    | ConfigAction::EditStyle { .. }
                    | ConfigAction::DeleteStyle(_)
                    | ConfigAction::RenameStyle { .. }
                    | ConfigAction::MoveStyle { .. }
                    | ConfigAction::TierText { .. }
                    | ConfigAction::Revert
                    | ConfigAction::RevertScoped(_)
                    | ConfigAction::ResetStyles
                    | ConfigAction::ResetTiers
            )
        }) {
            // A revert sets its own CONFIG message below, after this clear.
            self.clear_events_on_edit();
        }
        for action in actions {
            match action {
                ConfigAction::AddStyle(input) => {
                    if let Ok(state) = &mut self.config {
                        match state.add_style(&input) {
                            Ok(name) => {
                                // Not the toggle: a new style always opens its editor (WR-01).
                                self.config_ui.expand(&name);
                                self.config_ui.scroll_to = Some(name);
                                self.config_ui.style_draft = None;
                                self.config_message = None;
                            }
                            Err(err) => self.config_message = Some((err.to_string(), true)),
                        }
                    }
                }
                ConfigAction::EditStyle { name, edit } => {
                    if let Ok(state) = &mut self.config
                        && state.edit_style(&name, edit)
                    {
                        self.config_message = None;
                    }
                }
                ConfigAction::SetStrictness(level) => {
                    if let Ok(state) = &mut self.config
                        && state.set_strictness(level)
                    {
                        self.clear_events_on_edit();
                    }
                }
                ConfigAction::DeleteStyle(name) => {
                    if let Ok(state) = &mut self.config
                        && state.delete_style(&name)
                    {
                        if self.config_ui.expanded.as_deref() == Some(name.as_str()) {
                            self.config_ui.expanded = None;
                        }
                        self.config_ui.delete_armed = None;
                        self.config_message = None;
                    }
                }
                ConfigAction::RenameStyle { from, to } => {
                    if let Ok(state) = &mut self.config {
                        let id = state.style_id(&from).map(str::to_string);
                        match state.rename_style(&from, &to) {
                            Ok(()) => {
                                if let Some(id) = id {
                                    self.config_ui.rename_drafts.remove(&id);
                                }
                                if self.config_ui.expanded.as_deref() == Some(from.as_str()) {
                                    self.config_ui.expanded = Some(to);
                                }
                                self.config_message = None;
                                if self.apply_state == ApplyState::Applied {
                                    self.apply_state = ApplyState::Unapplied;
                                }
                            }
                            Err(err) => self.config_message = Some((err.to_string(), true)),
                        }
                    }
                }
                ConfigAction::MoveStyle { name, up } => {
                    if let Ok(state) = &mut self.config
                        && state.move_style(&name, up)
                    {
                        self.config_message = None;
                        if self.apply_state == ApplyState::Applied {
                            self.apply_state = ApplyState::Unapplied;
                        }
                    }
                }
                ConfigAction::TierText { tier, text } => {
                    if let Ok(state) = &mut self.config {
                        state.set_tier_text(&tier, &text);
                        self.config_message = None;
                    }
                }
                ConfigAction::Revert => {
                    if let Ok(state) = &mut self.config {
                        state.revert();
                        self.config_ui.reset_for_new_styles();
                        self.config_message = Some(("Reverted config changes.".to_string(), false));
                    }
                    // Per tab: rule edits survive a CONFIG revert, so the state may stay stale.
                    let rules_dirty = self.working.as_ref().is_some_and(|w| w.is_dirty());
                    self.apply_state = apply::state_after_revert(
                        self.applied_this_session,
                        rules_dirty || self.source_pending(),
                    );
                }
                ConfigAction::RevertScoped(scope) => {
                    if let Ok(state) = &mut self.config {
                        state.revert_scope(scope);
                        let text = match scope {
                            RevertScope::Styles => {
                                self.config_ui.reset_for_new_styles();
                                "Reverted config changes."
                            }
                            RevertScope::Tiers => "Reverted tier changes.",
                        };
                        self.config_message = Some((text.to_string(), false));
                    }
                    let rules_dirty = self.working.as_ref().is_some_and(|w| w.is_dirty());
                    let config_dirty = self.config.as_ref().is_ok_and(|c| c.is_dirty());
                    self.apply_state = apply::state_after_revert(
                        self.applied_this_session,
                        rules_dirty || config_dirty || self.source_pending(),
                    );
                }
                ConfigAction::ResetStyles => {
                    if let Ok(state) = &mut self.config {
                        state.reset_styles();
                        self.config_ui.reset_for_new_styles();
                        self.config_ui.expanded = None;
                        self.config_message = None;
                    }
                }
                ConfigAction::ResetTiers => {
                    if let Ok(state) = &mut self.config {
                        state.reset_tiers();
                        self.config_message = None;
                    }
                }
                ConfigAction::Apply => {
                    self.run_apply(ctx, false);
                }
                ConfigAction::Overwrite => {
                    self.run_apply(ctx, true);
                }
                ConfigAction::Reload => {
                    match config_reload_plan(self.config.is_err(), &self.changed_on_disk) {
                        ConfigReload::ConfigOnly => {
                            self.config = match &self.root {
                                Ok(root) => ConfigState::load(root),
                                Err(message) => Err(message.clone()),
                            };
                            // A pending filter refusal stays for the RULES tab to resolve.
                            self.changed_on_disk.retain(|f| *f == DiskFile::Filter);
                            self.sync_tiers_in_use();
                            self.config_message = None;
                            self.config_ui.reset_for_new_styles();
                        }
                        ConfigReload::Changed => self.reload_changed(),
                    }
                }
                ConfigAction::SelectFilter(name) => self.select_filter(name),
                ConfigAction::SetReloadAfterApply(on) => self.set_reload_after_apply(on),
                ConfigAction::CreateFilter(input) => {
                    let name = settings::new_filter_name(&input);
                    if let Err(problem) = settings::valid_filter_name(&name) {
                        self.config_message = Some((problem.to_string(), true));
                    } else if self
                        .filters_cache
                        .as_deref()
                        .is_some_and(|list| settings::contains_name(list, &name))
                    {
                        self.config_message =
                            Some((format!("{name} already exists in the game folder."), true));
                    } else {
                        // The file is not created here; the first RULES Apply writes it.
                        self.select_filter(name);
                        if self.pending_switch.is_none() {
                            self.config_ui.filter_draft = None;
                        }
                    }
                }
                ConfigAction::ConfirmSwitch => {
                    if let Some(dir) = self.pending_folder.take() {
                        self.pending_switch = None;
                        self.apply_folder(dir);
                    } else if let Some(name) = self.pending_switch.take() {
                        self.switch_to(name);
                    }
                }
                ConfigAction::CancelSwitch => {
                    self.pending_switch = None;
                    self.pending_folder = None;
                }
                ConfigAction::BrowseFolder => {
                    if !self.game_dir_pinned
                        && !self.folder_pick.is_open()
                        && self.pending_switch.is_none()
                        && self.pending_folder.is_none()
                    {
                        self.want_browse = true;
                    }
                }
                ConfigAction::RefreshLists => {
                    self.refresh_filters();
                    super::sound_files::refresh(ctx);
                }
            }
        }
    }

    /// A filter picked in the CONFIG dropdown (or just named). Unapplied rule edits make it wait
    /// for confirmation; the dropdown keeps showing the current name until then. Only real edits
    /// count: a pending style rewrite is derived, not an edit.
    fn select_filter(&mut self, name: String) {
        let rules_dirty = self.working.as_ref().is_some_and(|w| w.is_dirty());
        match switch_decision(rules_dirty, self.settings.filter_name(), &name) {
            SwitchDecision::Noop => {
                self.pending_switch = None;
                // Picking the filter the overlay opened on its own makes it the selection.
                if self.settings.filter.is_none() {
                    self.remember_open_filter();
                }
            }
            SwitchDecision::Switch => self.switch_to(name),
            SwitchDecision::Confirm => self.pending_switch = Some(name),
        }
    }

    /// Draws the CONFIG tab inside the shell's scroll area and returns what the user asked for.
    fn config_body(
        &mut self,
        ui: &mut egui::Ui,
        skin: &Skin,
        footer: ConfigFooterState,
    ) -> Vec<ConfigAction> {
        let mut actions = Vec::new();
        // Fresh every frame: it must be right after a switch or a reload.
        let users = self
            .working
            .as_ref()
            .map(|w| w.style_users())
            .unwrap_or_default();
        let filters = self.filters_cache.clone();
        let ctx = ConfigViewCtx {
            filters: filters.as_deref(),
            active_filter: self.settings.filter_name(),
            active_filter_selected: self.settings.filter.is_some(),
            game_dir: self.game_dir.as_deref(),
            game_dir_missing: self.game_dir_missing,
            game_dir_pinned: self.game_dir_pinned,
            default_game_dir: self.default_game_dir.as_deref(),
            folder_dialog_open: self.folder_dialog_open(),
            rule_users: &users,
            config_refs: &self.config_refs,
            changed_on_disk: !self.changed_on_disk.is_empty(),
            apply_enabled: footer.apply_enabled,
            rules_blocking: footer.rules_blocking,
            filter_damaged: footer.filter_damaged,
            indicator: apply::indicator(self.apply_state),
            reload_after_apply: self.settings.reload_after_apply(),
            skin: Some(skin),
        };
        w::ff_scroll_area(ui, skin, "config", |ui| match &self.config {
            Ok(state) => {
                actions.extend(config_view::show_sections(
                    ui,
                    state,
                    &mut self.config_ui,
                    &ctx,
                ));
            }
            Err(message) => {
                actions.extend(config_view::show_load_error(ui, message));
            }
        });
        actions
    }

    /// Draws the RULES tab: the search row and the rule list (or its notice).
    fn rules_content(
        &mut self,
        ui: &mut egui::Ui,
        skin: &Skin,
        actions: apply::FooterActions,
        row_actions: &mut Vec<RowAction>,
    ) {
        if let Some(working) = &self.working {
            let total = working.rows().len();
            match rule_list::search_row(ui, skin, &mut self.query, actions.add_rule, total) {
                rule_list::ToolbarClick::AddRule => row_actions.push(RowAction::AddRule),
                rule_list::ToolbarClick::AddSection => row_actions.push(RowAction::AddSection),
                rule_list::ToolbarClick::None => {}
            }
            ui.add_space(8.0);
        }
        let avail = ui.available_rect_before_wrap();
        let list_rect = avail;
        let mut lui = ui.new_child(
            UiBuilder::new()
                .max_rect(list_rect)
                .layout(Layout::top_down(Align::Min)),
        );
        lui.set_clip_rect(list_rect.intersect(ui.clip_rect()));
        let filter_name = self.settings.filter_name().to_string();
        let mut toggled: Vec<(String, bool, bool)> = Vec::new();
        match (&mut self.working, &self.filter) {
            (Some(working), _) => {
                let settings = &self.settings;
                let entries = working.entries(&self.query, |sec| {
                    let key = settings::section_key(&sec.name, sec.economy);
                    settings.section_collapsed(&filter_name, &key, sec.economy)
                });
                let searching = !self.query.trim().is_empty();
                let has_rules = entries.iter().any(|e| {
                    matches!(
                        e,
                        rules::ListEntry::Rule(_) | rules::ListEntry::EconomyRule(_)
                    )
                });
                if entries.is_empty() && !searching {
                    centered_notice(&mut lui, "No rules yet");
                } else if !has_rules && searching {
                    centered_notice(
                        &mut lui,
                        &format!("No rules match \"{}\"", self.query.trim()),
                    );
                } else if working.rows().is_empty() && !searching {
                    // Only dividers (and maybe the economy section): list them, then say so.
                    let h = rule_list::entries_height(&entries).min(lui.available_height());
                    let w = lui.available_width();
                    let frame = lui
                        .allocate_ui(vec2(w, h), |ui| {
                            rule_list::show_list(
                                ui,
                                working,
                                &entries,
                                &mut self.list,
                                true,
                                self.classes.as_ref().ok(),
                            )
                        })
                        .inner;
                    row_actions.extend(frame.actions);
                    toggled = frame.toggled;
                    if lui.available_height() > 60.0 {
                        centered_notice(&mut lui, "No rules yet");
                    }
                } else {
                    let frame = rule_list::show_list(
                        &mut lui,
                        working,
                        &entries,
                        &mut self.list,
                        !searching,
                        self.classes.as_ref().ok(),
                    );
                    row_actions.extend(frame.actions);
                    toggled = frame.toggled;
                }
            }
            (None, Err(_)) => {
                centered_notice(&mut lui, "The filter could not be loaded.");
            }
            (None, Ok(_)) => {}
        }
        for (name, economy, collapsed) in toggled {
            self.set_section_collapsed(&name, economy, collapsed);
        }
    }
}

/// What the CONFIG footer needs from the per-frame Apply decision.
#[derive(Debug, Clone, Copy)]
struct ConfigFooterState {
    apply_enabled: bool,
    rules_blocking: bool,
    filter_damaged: bool,
}

/// What a frame of the bottom bar reported.
#[derive(Debug, Clone, Copy)]
pub(crate) struct BarClicks {
    pub revert: bool,
    pub reload: bool,
    pub apply: bool,
    /// The status line was clicked (jump to its rule, when it has one).
    pub status: bool,
    /// The Revert, Reload and Apply button rectangles.
    #[cfg_attr(not(test), allow(dead_code))]
    pub rects: [Rect; 3],
}

/// Draws the bottom bar inside `bar`: Revert, Reload and the Apply slot right to left, then the
/// one status line in the room left of them.
pub(crate) fn bottom_bar(
    ui: &mut egui::Ui,
    skin: &Skin,
    bar: Rect,
    state: &shell::BarState,
    status: &StatusLine,
) -> BarClicks {
    let inner = bar.shrink2(vec2(10.0, 0.0));
    let mut bui = ui.new_child(
        UiBuilder::new()
            .max_rect(inner)
            .layout(Layout::right_to_left(Align::Center)),
    );
    bui.spacing_mut().item_spacing.x = 8.0;
    let slot = &state.apply;
    let mut apply = w::ff_button_ex(&mut bui, skin, slot.label, 84.0, slot.enabled, slot.danger);
    if let Some(tip) = slot.tip {
        apply = apply.on_hover_text(tip);
    }
    let reload = w::ff_button_ex(&mut bui, skin, "Reload", 84.0, state.reload, false)
        .on_hover_text(state.reload_tip);
    let revert = w::ff_button_ex(&mut bui, skin, "Revert", 84.0, state.revert, false);
    let status_rect = Rect::from_min_max(
        inner.min,
        pos2((bui.min_rect().min.x - 10.0).max(inner.min.x), inner.max.y),
    );
    let line = w::ff_status_ex(
        ui,
        status_rect,
        &status.text,
        status.kind,
        status.jump.is_some() || status.open_tab.is_some(),
    );
    let line = match status.tip {
        Some(tip) => line.on_hover_text(tip),
        None => line,
    };
    BarClicks {
        revert: state.revert && revert.clicked(),
        reload: state.reload && reload.clicked(),
        apply: slot.enabled && apply.clicked(),
        status: (status.jump.is_some() || status.open_tab.is_some()) && line.clicked(),
        rects: [revert.rect, reload.rect, apply.rect],
    }
}

impl eframe::App for OverlayApp {
    fn ui(&mut self, ui: &mut egui::Ui, frame: &mut eframe::Frame) {
        // Cache the overlay's own HWND the first frame one is available
        // (RESEARCH Pattern 1). Retried every frame until it succeeds.
        if self.window.is_none() {
            self.window = win::from_frame(frame);
        }
        if self.sound_dir_stale {
            self.sound_dir_stale = false;
            super::sound_files::set_dir(ui.ctx(), self.sound_dir());
        }
        // A folder the dialog worker handed back is applied here, on the UI thread.
        if let Some(path) = self.folder_pick.poll() {
            self.on_folder_picked(path);
        }

        // Dock on the game's monitor (the primary one without the game): once as soon as the
        // window handle exists, again on every summon (below), and once more when the window's
        // scale changes after a move between monitors.
        if let Some(window) = &self.window {
            let ppp = ui.ctx().input(|i| i.viewport().native_pixels_per_point);
            if (!self.docked || (ppp.is_some() && ppp != self.dock_ppp))
                && win::dock_left(window, shell::WIDTH)
            {
                self.docked = true;
                self.dock_ppp = ppp;
            }
        }

        // Poll the global hotkey (D-09). Accept only our own registered id
        // and only the Pressed state — without the state filter, a single
        // keypress toggles twice (once on press, once on release).
        if let Ok(event) = global_hotkey::GlobalHotKeyEvent::receiver().try_recv() {
            if let HotkeyStatus::Registered { id } = &self.hotkey_status {
                if event.id == *id && event.state == global_hotkey::HotKeyState::Pressed {
                    // Only while the user is actually in the game — or in the window itself, so
                    // the same key can dismiss it once it has focus. The binding is registered
                    // system-wide, which is what lets it reach a fullscreen game, but a key that
                    // also fires in a browser or an editor is a key the user cannot use for
                    // anything else (author, 2026-09-15).
                    //
                    // The event is still drained either way: leaving it queued would make the
                    // next in-game press replay a stale one.
                    if win::foreground_is_game_or_overlay(self.window.as_ref()) {
                        self.toggle_requested = true;
                    }
                }
            }
        }

        // Poll the tray's click events (D-17: left-click toggles, same as
        // the hotkey).
        if let Ok(event) = tray_icon::TrayIconEvent::receiver().try_recv() {
            if let tray_icon::TrayIconEvent::Click {
                button,
                button_state,
                ..
            } = event
            {
                if button == tray_icon::MouseButton::Left
                    && button_state == tray_icon::MouseButtonState::Up
                {
                    self.toggle_requested = true;
                }
            }
        }

        // Collect a finished economy sync. The state machine only advances here, so this must
        // run every frame. A landed result also forces a repaint: the window is drawn over a game
        // that may not be generating input events, so without this the "SYNCING…" label could sit
        // stale until the player happened to move the mouse over the window.
        if self.economy.poll() {
            self.prices_changed();
            ui.ctx().request_repaint();
        }
        if self.leagues.poll() {
            ui.ctx().request_repaint();
        }
        if let Some((text, kind)) = self.update.poll(win::utc_offset_minutes()) {
            self.update_message = Some((text, update_kind(kind)));
            ui.ctx().request_repaint();
        }
        if matches!(
            self.update.state(),
            UpdateState::Checking | UpdateState::Downloading { .. }
        ) {
            ui.ctx()
                .request_repaint_after(std::time::Duration::from_millis(100));
        }

        // Poll the tray menu's Show/Quit events. Quit itself is only *requested* here; the
        // actual close-the-viewport send is deferred to after the body renders. Quit is still the
        // only path that ends the process.
        let mut quit_requested = false;
        if let Ok(event) = tray_icon::menu::MenuEvent::receiver().try_recv() {
            match tray::command_for_menu(&event.id, &self.tray) {
                Some(OverlayCommand::Show) => {
                    // D-17: "Show" only means something while currently
                    // hidden — while already shown (focused or not), it is
                    // a no-op rather than a hide, unlike the hotkey/tray
                    // click, which always flips.
                    if self.visibility == Visibility::Hidden {
                        self.toggle_requested = true;
                    }
                }
                Some(OverlayCommand::Toggle) => self.toggle_requested = true,
                Some(OverlayCommand::Quit) => quit_requested = self.request_quit(),
                None => {}
            }
        }
        // INST-04: the installer's `ff --quit` is the same request as tray Quit, so unapplied
        // edits show the normal Unapplied changes popup.
        if !quit_requested
            && self
                .instance
                .as_ref()
                .is_some_and(|g| g.take_quit_request())
        {
            quit_requested = self.request_quit();
        }
        // Apply & quit succeeded, or a quit request arrived mid-reload: leave once the in-game
        // reload (if one started) has finished, so it is never cut off mid-sequence. The quit
        // itself sends no keystrokes.
        if !quit_requested && self.run_deferred_quit() {
            quit_requested = true;
        }
        // "Quit without applying" was chosen in the confirm last frame. It runs here, before the
        // show-request poll below, like every other quit path (WR-08).
        if self.quit_next_frame {
            self.quit_next_frame = false;
            if !quit_requested {
                quit_requested = true;
                self.begin_shutdown();
            }
        }

        // INST-01 (CONTEXT "Behaviour"): a second launch is a reveal, never a toggle (same rule
        // as the tray's Show, D-17). It deliberately bypasses the hotkey's foreground gate
        // because the signal comes from outside the game and the window. The reveal itself goes
        // through `visibility::advance` and `win::reveal` (`SW_SHOWNOACTIVATE`, no focus steal).
        // The existing `request_repaint_after(50ms)` plus eframe's hidden-window heartbeat is
        // what wakes a window hidden to the tray, so no extra waker thread is needed.
        //
        // The first frame is where polling starts, so it is also where the instance tells a
        // waiting launcher it is ready (WR-02). Until then a launch during startup waits up to
        // `instance::STARTUP_TIMEOUT_MS` instead of reporting "not responding". Later calls
        // are no-ops. After tray Quit the guard is gone (`None`) and nothing is polled.
        if let Some(guard) = self.instance.as_mut() {
            guard.mark_ready();
            if guard.take_show_request() {
                if self.visibility == Visibility::Hidden {
                    self.toggle_requested = true;
                }
                // An already-shown window is still an answer.
                guard.ack();
            }
        }

        // The frame decision: one call to the tested state machine, one
        // call to translate the transition into the single effect to
        // perform. No branching of our own.
        let real_focus = ui.ctx().input(|i| i.viewport().focused).unwrap_or(false);
        if real_focus {
            self.folder_pick.note_focused();
        }
        // The overlay neither hides while the folder dialog is open nor in the short grace after
        // it closes, before focus comes back; the hotkey is ignored while the dialog is open.
        let focused = super::folder_dialog::keep_visible(
            real_focus,
            self.folder_pick.is_open(),
            self.folder_pick.since_close(),
        );
        if self.folder_pick.since_close().is_some() {
            ui.ctx()
                .request_repaint_after(super::folder_dialog::CLOSE_GRACE);
        }
        let toggle = self.toggle_requested && !self.folder_pick.is_open();
        // The screenshot mode keeps the window shown: nothing may hide it before the frame is saved.
        let next = if self.is_shot() {
            Visibility::ShownFocused
        } else {
            visibility::advance(self.visibility, focused, toggle)
        };
        if let Some(effect) = visibility::effect_for(self.visibility, next) {
            if let Some(window) = &self.window {
                match effect {
                    visibility::Effect::Reveal => {
                        // Every summon re-docks first: the game may have moved monitors.
                        let _ = win::dock_left(window, shell::WIDTH);
                        win::reveal(window)
                    }
                    visibility::Effect::Conceal => win::conceal(window),
                }
            }
        }
        // Concealing the window clears every event message, so the next reveal starts clean.
        //
        // They are reports on an action the player has already seen the result of: "Wrote
        // <path>…" and "Economy prices refreshed from <league>" answer a question asked minutes
        // ago, and left standing they are indistinguishable from a report about whatever the
        // player is about to do next (author, 2026-09-11). A message that outlives its own
        // question is worse than no message.
        //
        // Conceal, not Reveal: clearing on the way out means the result stays readable for as
        // long as the player keeps the window open, which is the whole point of showing it.
        //
        // The one exception is a reload in flight: sending it focuses the game, which is itself
        // the click-away that hides the window. Its interim line is kept so the result can
        // replace it below and be read on the next reveal (review CR-01).
        if matches!(
            visibility::effect_for(self.visibility, next),
            Some(visibility::Effect::Conceal)
        ) {
            self.clear_events_on_conceal();
        }

        self.visibility = next;
        self.toggle_requested = false;

        // The reload result lands after the worker's last step; the line is only replaced if it
        // is still the interim one. Polled after the conceal above, so a result and a hide in the
        // same frame cannot clear the result it just wrote: it lands into a hidden window and is
        // shown on the next reveal.
        if let Some(result) = self.reload.poll() {
            if let Some(p) = self.reload_pending.take()
                && let Some(msg) = landed_reload_message(
                    self.apply_message.as_ref().map(|(text, _)| text.as_str()),
                    &p,
                    &result,
                )
            {
                self.apply_message = if p.warning {
                    Some((msg, w::StatusKind::Warning))
                } else {
                    applied_message(msg)
                };
            }
            ui.ctx().request_repaint();
        }

        // Explicit polling cadence rather than relying on eframe's internal
        // heartbeat (RESEARCH Pattern 3). eframe >= 0.34 already gives
        // invisible windows a ~10 Hz heartbeat keyed off a live
        // `IsWindowVisible` call, so the raw-HWND bypass in `super::win`
        // does not starve this loop — this just makes the interval
        // explicit in our own code.
        ui.ctx().request_repaint_after(Duration::from_millis(50));

        // Debug screenshot mode: wait for the dock and a few frames, ask for one capture, save it,
        // then leave through the one Close send site below.
        #[cfg(debug_assertions)]
        if self.shot.is_some() && self.shot_step(ui.ctx()) {
            quit_requested = true;
        }

        // Computed once per frame; every button below is shown/enabled strictly from it.
        self.poll_styles(ui.ctx().input(|i| i.time));
        // The RULES tab judges tags and offers styles from the CONFIG tab's in-memory set, so an
        // added or renamed style is usable before Apply (UAT3-03).
        if let (Ok(config), Some(working)) = (&self.config, self.working.as_mut()) {
            sync_working_styles(config, working);
        }
        self.sync_tiers_in_use();
        self.refresh_staged_economy();
        let state = self.filter_state();
        // A changed styles.json makes the written filter stale even with no rule edited, and so
        // does a price-source switch that changes the economy blocks.
        let source_pending = self.source_pending();
        let style_pending = self
            .working
            .as_mut()
            .is_some_and(|w| w.style_rewrite_pending())
            || source_pending;
        let config_dirty = self.config.as_ref().is_ok_and(|c| c.is_dirty());
        let styles_dirty = self.config.as_ref().is_ok_and(|c| c.styles_dirty());
        let tiers_dirty = self.config.as_ref().is_ok_and(|c| c.tiers_dirty());
        let rules_dirty = self.working.as_ref().is_some_and(|w| w.is_dirty());
        let economy_staged = self.working.as_ref().is_some_and(|w| w.economy_staged());
        // One unapplied state for both tabs: rule edits, a pending style rewrite or CONFIG edits.
        let combined_dirty = apply::combined_dirty(rules_dirty, style_pending, config_dirty);
        if combined_dirty && self.apply_state == ApplyState::Applied {
            self.apply_state = ApplyState::Unapplied;
        }
        let unknown_styles = self
            .working
            .as_ref()
            .map(|w| w.unknown_styles())
            .unwrap_or_default();
        let first_blocking = self
            .working
            .as_ref()
            .and_then(|w| w.blocking().first().cloned());
        let blocking = self.working.as_ref().map_or(0, |w| w.blocking().len());
        let changed = !self.changed_on_disk.is_empty();
        let mut actions = apply::footer_actions_combined(
            self.root.is_ok(),
            state,
            changed,
            rules_dirty || style_pending,
            config_dirty,
            blocking,
        );
        actions.revert = self.rules_revert_live();
        // One Overwrite decision for the bar slot and the popup (WR-03).
        actions.overwrite = self.overwrite_decision();
        self.overwrite_live = actions.overwrite;
        let config_footer = ConfigFooterState {
            apply_enabled: apply::config_apply_enabled(
                self.root.is_ok(),
                self.config.as_ref().is_ok_and(|c| c.validate().is_ok()),
                state,
                changed,
                combined_dirty,
                blocking,
            ),
            rules_blocking: blocking > 0,
            filter_damaged: state == FilterState::Damaged,
        };

        // Everything the panels want to change is collected here and applied once, after both have
        // rendered (deferred-action rule): row operations, the economy footer and the jump.
        let mut row_actions: Vec<RowAction> = Vec::new();
        let mut jump_to: Option<BlockId> = None;
        let mut config_actions: Vec<ConfigAction> = Vec::new();
        let mut picked_tab: Option<ShellTab> = None;
        let mut economy_clicks = shell::EconomyClicks::default();

        // ---- The shell: background, frame, header, tabs, content, bottom bar ----
        let skin = self.skin.clone();
        let win_rect = ui.max_rect();
        let painter = ui.painter().clone();
        w::ff_background(&painter, &skin, win_rect);
        let header_h = w::header_height(&skin, win_rect.width());
        let frame_outer = Rect::from_min_max(
            pos2(win_rect.min.x, win_rect.min.y + header_h - 14.0),
            win_rect.max,
        );
        let shell_frame = w::ff_frame(&painter, &skin, frame_outer);

        // The header's close control hides the window; it is never the way the process exits
        // (that is tray -> Quit alone).
        let header = Rect::from_min_size(win_rect.min, vec2(win_rect.width(), header_h));
        let close = w::ff_header(ui, &skin, header, shell::header_art(&skin), "FerrisFilter")
            .on_hover_text("Hide (Ctrl+Shift+Z)");
        if let Ok(loaded) = &self.filter {
            // The loaded filter's full path, where the old status block used to print it.
            ui.interact(
                w::header_plate_rect(header),
                egui::Id::new("ff-title-plate"),
                egui::Sense::hover(),
            )
            .on_hover_text(loaded.path().display().to_string());
        }
        if close.clicked() && self.visibility != Visibility::Hidden {
            if let Some(window) = &self.window {
                win::conceal(window);
            }
            self.visibility = Visibility::Hidden;
        }

        // Tabs: ` *` marks a tab when it holds unapplied edits (the selected tab included).
        // A staged economy add/remove belongs to ECONOMY, not to the RULES marker.
        let rules_edited = self.working.as_ref().is_some_and(|w| w.rules_edited());
        let markers = shell::tab_markers(
            rules_edited,
            styles_dirty,
            tiers_dirty,
            economy_staged || source_pending,
        );
        let labels: Vec<String> = ShellTab::ALL
            .iter()
            .map(|t| apply::tab_label(t.label(), markers[t.index()]))
            .collect();
        let label_refs: Vec<&str> = labels.iter().map(String::as_str).collect();
        let tips: Vec<Option<&str>> = markers
            .iter()
            .map(|m| m.then_some(apply::OTHER_TAB_DIRTY_TIP))
            .collect();
        let mut selected = self.tab.index();
        w::ff_tabs_ex(
            ui,
            &skin,
            w::tab_strip(&shell_frame),
            &label_refs,
            &mut selected,
            &tips,
        );
        if selected != self.tab.index() {
            picked_tab = ShellTab::from_index(selected);
        }

        // Content, clipped to the area inside the gold line.
        let content = shell_frame.content.shrink2(vec2(10.0, 8.0));
        let mut cui = ui.new_child(
            UiBuilder::new()
                .max_rect(content)
                .layout(Layout::top_down(Align::Min)),
        );
        cui.set_clip_rect(shell_frame.content);
        let mut about_action: Option<super::about_view::AboutAction> = None;
        match self.tab {
            ShellTab::Rules => self.rules_content(&mut cui, &skin, actions, &mut row_actions),
            ShellTab::Config => {
                config_actions = self.config_body(&mut cui, &skin, config_footer);
            }
            ShellTab::Economy => {
                // The list is fetched the first time the tab is shown in a session (never in
                // screenshot mode; unit tests get an error from the `fetch_leagues` twin).
                if !self.is_shot() && matches!(self.leagues.state(), ListState::NotStarted) {
                    self.leagues.start(fetch_leagues);
                }
                let view = self.league_view();
                let fallback = self.settings.price_policy() == PriceSourcePolicy::ForceFallback;
                let now_secs = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map_or(0, |d| d.as_secs());
                let inputs = shell::EconomyTabInputs {
                    league: &view,
                    fallback,
                    prices_league: self.economy_info.league.as_deref(),
                    mtime_secs: self.economy_info.mtime_secs,
                    now_secs,
                    utc_offset_minutes: win::utc_offset_minutes(),
                    priced_items: self.economy_info.priced_items,
                    live: self.economy_info.live,
                    problem: self.economy_info.problem.as_deref(),
                    syncing: self.economy.is_running(),
                    sync_enabled: actions.sync && view.sync_league.is_some() && !fallback,
                    sync_disabled_tip: self
                        .root
                        .as_ref()
                        .err()
                        .map(String::as_str)
                        .or(view.sync_block_tip),
                    tiers_reset_enabled: self.config.as_ref().is_ok_and(|c| !c.tiers_at_defaults()),
                    section: shell::section_button(
                        self.working.as_ref().map(|w| w.economy()),
                        self.root.is_ok(),
                        !self.changed_on_disk.is_empty(),
                    ),
                };
                let mut tier_actions: Vec<ConfigAction> = Vec::new();
                economy_clicks =
                    shell::economy_tab(&mut cui, &skin, &inputs, |ui| match &self.config {
                        Ok(state) => {
                            tier_actions = config_view::show_tiers_section(ui, &skin, state);
                        }
                        Err(message) => {
                            tier_actions.extend(config_view::show_load_error(ui, message));
                        }
                    });
                config_actions.extend(tier_actions);
                cui.ctx()
                    .request_repaint_after(std::time::Duration::from_secs(60));
            }
            ShellTab::About => {
                let view = super::about_view::UpdateView {
                    caption: self.update.caption(),
                    enabled: self.update.enabled(),
                    ready: matches!(self.update.state(), UpdateState::Ready(_)),
                    tooltip: self.update.tooltip(),
                };
                about_action = super::about_view::about_tab(&mut cui, &skin, &view);
            }
        }

        // Bottom bar: the one status line plus Revert / Reload / Apply.
        let bar_state = shell::bar_state(
            self.tab,
            actions,
            ConfigBarInputs {
                styles_dirty,
                tiers_dirty,
                economy_staged,
                apply_enabled: config_footer.apply_enabled,
                tier_errors: self.config.as_ref().is_ok_and(|c| c.has_tier_errors()),
                load_error: self.config.is_err()
                    || self
                        .config
                        .as_ref()
                        .is_ok_and(|c| c.styles_error().is_some() || c.tiers_error().is_some()),
                rules_blocking: blocking > 0,
                damaged: state == FilterState::Damaged,
                config_invalid: self.config_invalid(),
            },
            changed,
        );
        let sync_outcome = match self.economy.state() {
            economy::SyncState::Finished(outcome) => {
                Some((outcome.message().to_string(), outcome.is_failure()))
            }
            _ => None,
        };
        let event = shell::event_from(
            self.update_message.clone(),
            self.apply_message.clone(),
            self.config_message.clone(),
            sync_outcome,
            self.list_status.clone(),
        );
        #[cfg(debug_assertions)]
        let event = match self.shot.as_ref().and_then(|s| s.args.status.clone()) {
            Some(text) => Some((text, w::StatusKind::Warning)),
            None => event,
        };
        let filter_error = match (&self.filter, state) {
            (Err(message), _) => Some(shell::load_failed_status(message)),
            (Ok(loaded), FilterState::Damaged) => Some(match loaded.has_section() {
                Err(err) => err.to_string(),
                Ok(_) => apply::DAMAGED_MARKERS_MESSAGE.to_string(),
            }),
            _ => None,
        };
        let status = shell::status_for(StatusInputs {
            root_error: self.root.as_ref().err().cloned(),
            event,
            grammar_gate: first_blocking.as_ref().map(|first| {
                (
                    apply::invalid_blocks_message(
                        blocking,
                        first.rule_index,
                        first.diagnostic.line,
                    ),
                    first.block,
                )
            }),
            filter_error,
            game_folder_missing: self.game_folder_missing_text(),
            syncing: self.economy.is_running(),
            ended_league: self
                .league_view()
                .ended
                .as_deref()
                .map(crate::overlay::leagues::ended_status),
            unknown: unknown_styles.first().map(|(rule, id)| {
                (
                    apply::unknown_styles_message(unknown_styles.len(), *rule),
                    *id,
                )
            }),
            migrated: self.filter.as_ref().is_ok_and(|f| f.migrated()),
            absent: state == FilterState::Absent,
            indicator: (
                apply::indicator(self.apply_state),
                shell::indicator_kind(self.apply_state),
            ),
        });
        let clicks = bottom_bar(ui, &skin, shell_frame.button_bar, &bar_state, &status);
        if clicks.status {
            jump_to = status.jump;
            if let Some(tab) = status.open_tab {
                picked_tab = Some(tab);
            }
        }

        // Bar clicks run before the tab switch, against the tab they were drawn for.
        let egui_ctx = ui.ctx().clone();
        if clicks.revert {
            match self.tab {
                // The Discard-rules popup opens from this flag.
                ShellTab::Rules => self.revert_rules_armed = true,
                // The Discard-config popup opens from this flag.
                ShellTab::Config => {
                    self.config_ui.revert_scope = RevertScope::Styles;
                    self.config_ui.revert_armed = true;
                }
                ShellTab::Economy => self.economy_revert_pressed(),
                ShellTab::About => {}
            }
        }
        if clicks.reload {
            match bar_reload_target(self.tab, &self.changed_on_disk) {
                BarReload::ConfigRefusal => config_actions.push(ConfigAction::Reload),
                BarReload::Shared => self.reload_pressed(),
            }
        }
        if clicks.apply {
            // The Apply slot reads Overwrite while a file changed on disk; it re-opens the
            // popup, whose Overwrite button runs the guarded write.
            if bar_state.apply.overwrite {
                self.changed_popup = true;
            } else {
                self.run_apply(&egui_ctx, false);
            }
        }
        if let Some(super::about_view::AboutAction::Check) = about_action {
            self.start_update_check();
        }
        if let Some(super::about_view::AboutAction::InstallAndQuit) = about_action
            && self.install_and_quit()
        {
            quit_requested = true;
        }
        if economy_clicks.sync
            && self.settings.price_policy() != PriceSourcePolicy::ForceFallback
            && let Some(league) = self.league_view().sync_league
            && let Ok(root) = &self.root
        {
            self.update_message = None;
            // The app root, not the process working directory: the overlay can be launched from
            // anywhere, and prices.json must land where Apply will read it from. A sync is not
            // part of Apply (dozens of network round trips that can fail); it only refreshes
            // prices, and the game sees them on the next Apply (author, 2026-09-10).
            let base_dir = root.clone();
            self.economy.start(move || {
                crate::sync::sync_economy_blocking(&base_dir, &league).map_err(|e| e.to_string())
            });
        }

        // Tab switch and CONFIG actions come first. Switching never applies or discards anything.
        if let Some(tab) = picked_tab {
            self.tab = tab;
            match tab {
                ShellTab::Config => {
                    self.config_ui.disarm_all();
                    self.refresh_filters();
                }
                ShellTab::Economy => {
                    self.config_ui.disarm_all();
                    self.refresh_economy_info();
                }
                _ => {}
            }
        }
        if !config_actions.is_empty() {
            let egui_ctx = ui.ctx().clone();
            self.handle_config_actions(&egui_ctx, config_actions);
        }
        // The dialog is owned by the overlay window, which is expected to keep it above the
        // topmost overlay. If the in-game check shows it behind, send
        // `egui::ViewportCommand::WindowLevel(egui::WindowLevel::Normal)` right after start and
        // `egui::WindowLevel::AlwaysOnTop` when `poll()` returns (pick or cancel).
        if std::mem::take(&mut self.want_browse) && !self.is_shot() {
            let egui_ctx = ui.ctx().clone();
            self.folder_pick
                .start(frame, self.game_dir.as_deref(), &egui_ctx);
        }

        // Deferred mutations: jump from the grammar line, then row operations, then the economy
        // footer. Quit stays last.
        if let (Some(target), Some(working)) = (jump_to, &self.working) {
            // The line is clickable on every tab; the rule it names lives on RULES (WR-02).
            self.tab = ShellTab::Rules;
            if !working.row_matches(target, &self.query) {
                self.query.clear();
            }
            self.list.expanded = Some(target);
            self.list.pending_scroll = Some(target);
            self.reveal_rule(target);
        }
        if !row_actions.is_empty() {
            self.run_row_actions(row_actions);
        }
        if economy_clicks.add {
            self.stage_add_economy();
        }
        if economy_clicks.remove {
            self.list.confirm_economy_remove = true;
        }
        if economy_clicks.reset_tiers {
            self.config_ui.reset_armed = Some(RevertScope::Tiers);
        }
        let fallback_now = self.settings.price_policy() == PriceSourcePolicy::ForceFallback;
        if let Some(source) = economy_clicks.source
            && (source == 1) != fallback_now
        {
            self.set_price_source(source == 0);
        }
        if let Some(pick) = economy_clicks.league_pick {
            let name = self.league_view().pick_name(pick).map(str::to_string);
            if let Some(name) = name
                && self.settings.saved_league() != Some(name.as_str())
            {
                self.set_league(&name);
            }
        }
        if economy_clicks.refresh_leagues
            && !self.is_shot()
            && !self.leagues.is_fetching()
            && !fallback_now
        {
            self.leagues.start(fetch_leagues);
        }

        // One skinned popup at a time (Quit, then Changed-on-disk, then the rest). The flags
        // survive hiding and re-summoning the window, so an open popup is still there.
        // An armed Revert with nothing left to discard disarms itself.
        if self.config_ui.revert_armed && self.config_discard_count() == 0 {
            self.config_ui.revert_armed = false;
        }
        if self.revert_rules_armed && !self.rules_revertible() {
            self.revert_rules_armed = false;
        }
        // A refused Apply this frame opens the popup now: decide its Overwrite from that state.
        self.overwrite_live = self.overwrite_decision();
        self.refresh_quit_tier_edits_only();
        if let Some(popup) = shell::active_popup(self.popup_flags(quit_requested)) {
            let spec = self.popup_spec_for(popup);
            let buttons: Vec<crate::skin::popup::PopupButton> = spec
                .buttons
                .iter()
                .map(|(label, danger, tip)| crate::skin::popup::PopupButton {
                    label,
                    danger: *danger,
                    tip: *tip,
                })
                .collect();
            let choice = crate::skin::popup::ff_modal(
                ui.ctx(),
                &skin,
                egui::Id::new(("ff-popup", format!("{popup:?}"))),
                &spec.title,
                &spec.body,
                &buttons,
            );
            if let Some(choice) = choice {
                let egui_ctx = ui.ctx().clone();
                self.dispatch_popup(&egui_ctx, popup, choice);
                egui_ctx.request_repaint();
            }
        }

        if quit_requested {
            // Quit is the only path that ends the process.
            ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close); // the only Close send site
        }
    }
}

#[cfg(test)]
mod tests {
    use std::ffi::OsStr;
    use std::path::PathBuf;

    fn boom() -> Option<PathBuf> {
        panic!("Documents must not be asked for");
    }

    fn root_with_game_folder(
        game: Option<&std::path::Path>,
    ) -> (tempfile::TempDir, Result<PathBuf, String>) {
        let t = tempfile::tempdir().unwrap();
        if let Some(g) = game {
            let json = serde_json::json!({ "game_folder": g.to_string_lossy() });
            std::fs::write(t.path().join("ff-settings.json"), json.to_string()).unwrap();
        }
        let root = Ok(t.path().to_path_buf());
        (t, root)
    }

    #[test]
    fn startup_game_dir_restores_the_saved_folder() {
        let want = std::env::temp_dir().join("ff-picked-game");
        let (_t, root) = root_with_game_folder(Some(&want));
        assert_eq!(super::startup_game_dir_from(&root, None, boom), Some(want));
    }

    #[test]
    fn startup_game_dir_env_wins_over_the_saved_folder() {
        let saved = std::env::temp_dir().join("ff-picked-game");
        let (_t, root) = root_with_game_folder(Some(&saved));
        let env = std::env::temp_dir().join("ff-env-game");
        assert_eq!(
            super::startup_game_dir_from(&root, Some(OsStr::new(&env)), boom),
            Some(env)
        );
    }

    #[test]
    fn startup_game_dir_without_settings_uses_documents() {
        let (t, root) = root_with_game_folder(None);
        let docs = PathBuf::from("D");
        let got = super::startup_game_dir_from(&root, None, || Some(docs.clone()));
        assert_eq!(got, Some(docs.join("My Games").join("Path of Exile 2")));
        assert_eq!(std::fs::read_dir(t.path()).unwrap().count(), 0);
    }

    #[test]
    fn startup_game_dir_with_an_unresolved_root_uses_documents() {
        let root = Err("no root".to_string());
        let docs = PathBuf::from("D");
        let got = super::startup_game_dir_from(&root, None, || Some(docs.clone()));
        assert_eq!(got, Some(docs.join("My Games").join("Path of Exile 2")));
    }

    #[test]
    fn startup_game_dir_ignores_a_relative_saved_folder() {
        let (_t, root) = root_with_game_folder(Some(std::path::Path::new("relative")));
        let docs = PathBuf::from("D");
        let got = super::startup_game_dir_from(&root, None, || Some(docs.clone()));
        assert_eq!(got, Some(docs.join("My Games").join("Path of Exile 2")));
    }

    #[test]
    fn filter_target_refuses_traversal_and_a_missing_folder() {
        use super::filter_target;
        let dir = std::path::Path::new("game");
        assert!(filter_target(Some(dir), "..\\x.filter").is_err());
        assert!(filter_target(Some(dir), "../x.filter").is_err());
        assert!(filter_target(Some(dir), "notafilter.txt").is_err());
        assert_eq!(
            filter_target(None, "ssf.filter").unwrap_err(),
            "Cannot find the Documents folder; pick the game folder on the Config tab."
        );
        assert_eq!(
            filter_target(Some(dir), "ssf.filter").unwrap(),
            dir.join("ssf.filter")
        );
    }

    #[test]
    fn switch_decision_guards_unapplied_edits() {
        use super::{SwitchDecision, switch_decision};
        assert_eq!(
            switch_decision(true, "dynamic.filter", "Dynamic.Filter"),
            SwitchDecision::Noop
        );
        assert_eq!(
            switch_decision(true, "dynamic.filter", "ssf.filter"),
            SwitchDecision::Confirm
        );
        assert_eq!(
            switch_decision(false, "dynamic.filter", "ssf.filter"),
            SwitchDecision::Switch
        );
    }

    #[test]
    fn saved_settings_keep_unknown_keys() {
        use super::settings_with_filter;
        let mut current = crate::settings::Settings::default();
        current
            .extra
            .insert("future".into(), serde_json::json!({"a": 1}));
        let next = settings_with_filter(&current, "ssf.filter");
        assert_eq!(next.filter_name(), "ssf.filter");
        assert_eq!(next.extra.get("future"), current.extra.get("future"));
    }

    fn pending(interim: &str) -> super::PendingReload {
        super::PendingReload {
            path: std::path::PathBuf::from("C:/g/dynamic.filter"),
            prefix: String::new(),
            extras: " [x]".to_string(),
            interim: interim.to_string(),
            warning: false,
        }
    }

    #[test]
    fn the_not_quit_warning_rides_on_a_pending_reload_line() {
        use super::{
            UNWRITABLE_TIERS_QUIT_STATUS, apply_message_after_conceal, landed_reload_message,
            show_not_quit_warning,
        };
        // A reload is in flight with its interim line showing.
        let mut p = pending("interim");
        let mut message = Some(("interim".to_string(), w::StatusKind::Ok));
        show_not_quit_warning(&mut message, Some(&mut p));
        let (text, kind) = message.clone().unwrap();
        assert_eq!(kind, w::StatusKind::Warning);
        assert!(text.starts_with("interim") && text.ends_with(UNWRITABLE_TIERS_QUIT_STATUS));
        assert!(p.warning);
        // The reload's own focus change hides the window: the line is kept.
        assert_eq!(
            apply_message_after_conceal(Some(text.clone()), Some(&p)),
            Some(text.clone())
        );
        // The result still lands and still carries the warning, success or failure.
        let landed = landed_reload_message(Some(&text), &p, &Ok(())).unwrap();
        assert!(landed.contains("Reloaded in game"), "{landed}");
        assert!(landed.ends_with(UNWRITABLE_TIERS_QUIT_STATUS), "{landed}");
        let failed = landed_reload_message(Some(&text), &p, &Err("x".to_string())).unwrap();
        assert!(failed.contains("could not reload in game: x"), "{failed}");
        assert!(failed.ends_with(UNWRITABLE_TIERS_QUIT_STATUS), "{failed}");

        // No reload line showing (already replaced, or no reload at all): the warning alone.
        let mut message = Some(("other".to_string(), w::StatusKind::Ok));
        let mut p = pending("interim");
        show_not_quit_warning(&mut message, Some(&mut p));
        assert_eq!(
            message,
            Some((
                UNWRITABLE_TIERS_QUIT_STATUS.to_string(),
                w::StatusKind::Warning
            ))
        );
        assert_eq!(p.interim, "interim");
        assert!(!p.warning);
        let mut message = None;
        show_not_quit_warning(&mut message, None);
        assert_eq!(message.unwrap().0, UNWRITABLE_TIERS_QUIT_STATUS);
    }

    #[test]
    fn landed_reload_message_replaces_only_the_interim_line() {
        use super::landed_reload_message;
        let p = pending("interim");
        let ok = landed_reload_message(Some("interim"), &p, &Ok(())).unwrap();
        assert!(
            ok.contains("Reloaded in game") && ok.ends_with(" [x]"),
            "{ok}"
        );
        let err = landed_reload_message(
            Some("interim"),
            &p,
            &Err("the game window did not take focus".to_string()),
        )
        .unwrap();
        assert!(
            err.contains("Options > Game > Item Filter > Refresh"),
            "{err}"
        );
        assert!(
            err.contains("could not reload in game: the game window did not take focus"),
            "{err}"
        );
        assert!(err.ends_with(" [x]"));
        assert_eq!(landed_reload_message(None, &p, &Ok(())), None);
        assert_eq!(landed_reload_message(Some("other"), &p, &Ok(())), None);
    }

    #[test]
    fn conceal_keeps_only_an_in_flight_reload_line() {
        use super::apply_message_after_conceal;
        let p = pending("interim");
        // A reload in flight with its interim line showing: the focus change hides the window,
        // and the line must survive for the result to replace it (review CR-01).
        assert_eq!(
            apply_message_after_conceal(Some("interim".into()), Some(&p)),
            Some("interim".to_string())
        );
        // Every other case clears, as before.
        assert_eq!(
            apply_message_after_conceal(Some("interim".into()), None),
            None
        );
        assert_eq!(
            apply_message_after_conceal(Some("other".into()), Some(&p)),
            None
        );
        assert_eq!(
            apply_message_after_conceal(Some("other".into()), None),
            None
        );
        assert_eq!(apply_message_after_conceal(None, Some(&p)), None);
    }

    #[test]
    fn a_reload_result_that_lands_while_hidden_is_shown_on_the_next_reveal() {
        use super::{apply_message_after_conceal, landed_reload_message};
        for (result, want) in [
            (Ok(()), "Reloaded in game"),
            (
                Err("a modifier key (Ctrl, Shift or Alt) was held down".to_string()),
                "Options > Game > Item Filter > Refresh",
            ),
        ] {
            let p = pending("interim");
            // Apply sets the interim line; focusing the game conceals the window.
            let after_hide = apply_message_after_conceal(Some("interim".into()), Some(&p));
            // The worker reports while the window is hidden.
            let landed = landed_reload_message(after_hide.as_deref(), &p, &result)
                .expect("the result must replace the kept interim line");
            assert!(landed.contains(want), "{landed}");
            // The result is now the status line the next reveal draws; the hide after that
            // (no reload pending any more) clears it as usual.
            assert_eq!(apply_message_after_conceal(Some(landed), None), None);
        }
    }

    #[test]
    fn skipped_reload_message_keeps_the_refresh_reminder() {
        use super::skipped_reload_message;
        use crate::overlay::chat::SkipReason;
        let path = std::path::Path::new("C:/g/dynamic.filter");
        for reason in [
            SkipReason::SettingOff,
            SkipReason::Busy,
            SkipReason::NoWindow,
        ] {
            let m = skipped_reload_message(path, " [x]", reason);
            assert!(m.contains("Options > Game > Item Filter > Refresh"), "{m}");
            assert!(m.contains(reason.text()), "{m}");
            assert!(m.ends_with(" [x]"));
        }
    }

    #[test]
    fn game_not_running_shows_no_reload_message() {
        use super::skipped_reload_message;
        use crate::overlay::apply;
        use crate::overlay::chat::SkipReason;
        let path = std::path::Path::new("C:/g/dynamic.filter");
        let m = skipped_reload_message(path, " [x]", SkipReason::GameNotRunning);
        assert_eq!(m, format!("{} [x]", apply::written_message(path)));
        assert!(!m.contains("Options > Game > Item Filter > Refresh"), "{m}");
        assert!(!m.contains(apply::REASON_GAME_NOT_RUNNING), "{m}");
    }

    #[test]
    fn saved_settings_keep_reload_flag() {
        use super::settings_with_filter;
        let mut current = crate::settings::Settings {
            reload_after_apply: Some(false),
            ..Default::default()
        };
        current.extra.insert("future".into(), serde_json::json!(1));
        let next = settings_with_filter(&current, "ssf.filter");
        assert_eq!(next.reload_after_apply, Some(false));
        assert_eq!(next.extra.get("future"), current.extra.get("future"));
    }

    #[test]
    fn reload_after_a_warned_load_honours_reload_off_but_keeps_the_filter() {
        use super::settings_after_reload;
        let mut current = crate::settings::Settings {
            filter: Some("a.filter".into()),
            reload_after_apply: Some(true),
            ..Default::default()
        };
        current.extra.insert("future".into(), serde_json::json!(1));
        let off = crate::settings::Settings {
            reload_after_apply: Some(false),
            ..Default::default()
        };
        // Warned, same root: filter and unknown keys kept, the file's "off" honoured.
        let next = settings_after_reload(&current, off.clone(), true, true);
        assert_eq!(next.filter.as_deref(), Some("a.filter"));
        assert_eq!(next.extra.get("future"), current.extra.get("future"));
        assert!(!next.reload_after_apply());
        // Warned, same root, no "off" in the load: the session's settings stay as they were.
        let next = settings_after_reload(&current, Default::default(), true, true);
        assert_eq!(next, current);
        // A clean load, or a different root, replaces everything.
        assert_eq!(
            settings_after_reload(&current, off.clone(), false, true),
            off
        );
        assert_eq!(
            settings_after_reload(&current, off.clone(), true, false),
            off
        );
    }

    #[test]
    fn toggling_reload_keeps_filter_and_unknown_keys() {
        use super::settings_with_reload;
        let mut current = crate::settings::Settings {
            filter: Some("a.filter".into()),
            ..Default::default()
        };
        current.extra.insert("future".into(), serde_json::json!(1));
        for on in [false, true] {
            let next = settings_with_reload(&current, on);
            assert_eq!(next.reload_after_apply, Some(on));
            assert_eq!(next.filter.as_deref(), Some("a.filter"));
            assert_eq!(next.extra.get("future"), current.extra.get("future"));
        }
    }

    /// The shell layout at its real size over a 511-block filter: the three bar buttons lie
    /// inside the frame's button bar however long the list is, and only a bounded window of rows
    /// is drawn (EDIT-07).
    #[test]
    fn shell_bar_stays_inside_the_button_bar_with_a_long_list() {
        use crate::overlay::apply::FooterActions;
        use crate::overlay::rule_list::{self, ListUi};
        use crate::overlay::rules::WorkingFilter;
        use crate::overlay::shell::{self, ConfigBarInputs, ShellTab, StatusLine};
        use crate::skin::widgets as w;
        use egui::{Align, Layout, Rect, UiBuilder, pos2, vec2};

        let bytes = include_bytes!("../../tests/fixtures/parser/filterblade.filter");
        let text = String::from_utf8_lossy(bytes).into_owned();
        let mut working = WorkingFilter::from_text(&text);
        assert!(
            working.rows().len() > 400,
            "fixture has a few hundred rules"
        );
        let visible = working.entries("", |_| false);
        let mut list = ListUi::default();

        let ctx = egui::Context::default();
        let skin = crate::skin::install(&ctx);
        let input = egui::RawInput {
            screen_rect: Some(Rect::from_min_size(egui::Pos2::ZERO, vec2(660.0, 1000.0))),
            ..Default::default()
        };
        let actions = FooterActions {
            sync: true,
            apply: true,
            revert: true,
            add_rule: true,
            reload: true,
            overwrite: false,
        };
        let bar_state =
            shell::bar_state(ShellTab::Rules, actions, ConfigBarInputs::default(), false);
        let status = StatusLine {
            text: "a very long status message ".repeat(20),
            kind: w::StatusKind::Warning,
            jump: None,
            open_tab: None,
            tip: None,
        };
        let mut drawn = 0usize;
        let mut button_bar = Rect::NOTHING;
        let mut rects = [Rect::NOTHING; 3];
        for _ in 0..3 {
            ctx.run_ui(input.clone(), |ui| {
                let win_rect = ui.max_rect();
                let painter = ui.painter().clone();
                w::ff_background(&painter, &skin, win_rect);
                let header_h = w::header_height(&skin, win_rect.width());
                let outer = Rect::from_min_max(
                    pos2(win_rect.min.x, win_rect.min.y + header_h - 14.0),
                    win_rect.max,
                );
                let frame = w::ff_frame(&painter, &skin, outer);
                button_bar = frame.button_bar;
                let content = frame.content.shrink2(vec2(10.0, 8.0));
                let mut cui = ui.new_child(
                    UiBuilder::new()
                        .max_rect(content)
                        .layout(Layout::top_down(Align::Min)),
                );
                cui.set_clip_rect(frame.content);
                drawn =
                    rule_list::show_list(&mut cui, &working, &visible, &mut list, true, None).drawn;
                rects = super::bottom_bar(ui, &skin, frame.button_bar, &bar_state, &status).rects;
            })
            .drop_without_applying_deltas();
        }
        for r in rects {
            assert!(
                button_bar.contains_rect(r) && r.width() > 0.0,
                "button {r:?} outside the button bar {button_bar:?}"
            );
        }
        assert!(drawn > 0, "something must be drawn");
        assert!(
            drawn < 80,
            "virtualization drew {drawn} of {} rows",
            working.rows().len()
        );
    }

    // ---- controller helpers (Phase 19-06) ----

    use super::{AllOutcome, apply_everything, apply_row_actions};
    use crate::compile::PriceSourcePolicy;
    use crate::dynamic::LoadedFilter;
    use crate::overlay::apply::{self, ApplyOutcome};
    use crate::overlay::rule_list::ListUi;
    use crate::overlay::rules::{RowAction, WorkingFilter};

    const RULES: &str = "Show\n  Class \"Rings\"\n\nHide\n  Rarity Normal\n";

    fn loaded_in(dir: &tempfile::TempDir, text: &str) -> (std::path::PathBuf, LoadedFilter) {
        let path = dir.path().join("dynamic.filter");
        std::fs::write(&path, text).unwrap();
        let loaded = LoadedFilter::load(&path).unwrap();
        (path, loaded)
    }

    /// The rules-only Apply (no CONFIG state), as the pre-Phase-25 tests drove it: the outcome,
    /// the appended sentences, and whether the file changed on disk.
    fn apply_rules(
        root: &std::path::Path,
        loaded: &mut LoadedFilter,
        working: &mut WorkingFilter,
        overwrite: bool,
    ) -> (ApplyOutcome, String, bool) {
        let r = apply_everything(
            root,
            None,
            Some((loaded, working)),
            PriceSourcePolicy::Auto,
            overwrite,
        );
        let outcome = match r.outcome {
            AllOutcome::Applied { filter: Some(path) } => ApplyOutcome::Written(path),
            AllOutcome::Applied { filter: None } => panic!("a filter was passed"),
            AllOutcome::Failed(message) => ApplyOutcome::Failed(message),
        };
        (outcome, r.extras, !r.changed.is_empty())
    }

    fn apply_working(
        root: &std::path::Path,
        loaded: &mut LoadedFilter,
        working: &mut WorkingFilter,
    ) -> (ApplyOutcome, String, bool) {
        apply_rules(root, loaded, working, false)
    }

    fn overwrite_working(
        root: &std::path::Path,
        loaded: &mut LoadedFilter,
        working: &mut WorkingFilter,
    ) -> (ApplyOutcome, String, bool) {
        apply_rules(root, loaded, working, true)
    }

    // ---- the combined Apply (Phase 25-03) ----

    use crate::overlay::apply_all::DiskFile;
    use crate::overlay::config_state::{ConfigState, StyleEdit};

    /// A temp app root holding styles-default.json / tiers-default.json as the live styles.json / tiers.json.
    fn config_root() -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        for (src, dst) in [
            ("tests/fixtures/seed-styles.json", "styles.json"),
            ("tiers-default.json", "tiers.json"),
        ] {
            std::fs::copy(
                std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(src),
                dir.path().join(dst),
            )
            .unwrap();
        }
        dir
    }

    /// Everything one combined Apply touches, in temp dirs.
    struct Rig {
        root: tempfile::TempDir,
        _game: tempfile::TempDir,
        path: std::path::PathBuf,
        loaded: LoadedFilter,
        working: WorkingFilter,
        config: ConfigState,
    }

    fn rig(filter_text: &str) -> Rig {
        rig_at(config_root(), filter_text)
    }

    fn rig_at(root: tempfile::TempDir, filter_text: &str) -> Rig {
        let game = tempfile::tempdir().unwrap();
        let (path, loaded) = loaded_in(&game, filter_text);
        let mut working = WorkingFilter::from_text(loaded.text());
        let config = ConfigState::load(root.path()).unwrap();
        working.set_styles(Ok(config.pending_styles().clone()));
        Rig {
            root,
            _game: game,
            path,
            loaded,
            working,
            config,
        }
    }

    impl Rig {
        fn apply(&mut self, overwrite: bool) -> super::AllResult {
            apply_everything(
                self.root.path(),
                Some(&mut self.config),
                Some((&mut self.loaded, &mut self.working)),
                PriceSourcePolicy::Auto,
                overwrite,
            )
        }

        fn read(&self, name: &str) -> Vec<u8> {
            std::fs::read(self.root.path().join(name)).unwrap()
        }
    }

    #[test]
    fn combined_apply_renders_rules_with_pending_styles() {
        let mut rig = rig(S_RULE);
        assert!(rig.config.edit_style("S", StyleEdit::Font(Some(41))));
        assert!(rig.config.is_dirty());
        let r = rig.apply(false);
        assert_eq!(
            r.outcome,
            AllOutcome::Applied {
                filter: Some(rig.path.clone())
            }
        );
        assert!(r.styles_written && !r.tiers_written);
        let styles = String::from_utf8(rig.read("styles.json")).unwrap();
        assert!(styles.contains("\"font\":41"), "{styles}");
        let filter = std::fs::read_to_string(&rig.path).unwrap();
        assert!(filter.contains("SetFontSize 41"), "{filter}");
        assert!(!filter.contains("SetFontSize 20"), "{filter}");
        assert!(!rig.config.is_dirty() && !rig.working.is_dirty());
        assert_eq!(rig.working.style("S").unwrap().font, Some(41));
        assert!(!rig.working.style_rewrite_pending());
    }

    // ---- defaults when the live files are absent (Phase 28, DEF-01 / DEF-03) ----

    /// A temp app root holding only the committed defaults under their own names (plus
    /// `config.json` for the class catalogue): no live `styles.json` / `tiers.json`.
    fn defaults_only_root() -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        for f in ["styles-default.json", "tiers-default.json", "config.json"] {
            std::fs::copy(
                std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(f),
                dir.path().join(f),
            )
            .unwrap();
        }
        dir
    }

    fn assert_no_live(root: &std::path::Path) {
        assert!(!root.join("styles.json").exists(), "styles.json created");
        assert!(!root.join("tiers.json").exists(), "tiers.json created");
    }

    #[test]
    fn defaults_only_root_loads_styles_and_classes_and_creates_nothing() {
        let root = defaults_only_root();
        let styles = super::load_styles_at(root.path()).unwrap();
        assert!(styles.get("S").is_some());
        assert!(super::load_classes_at(root.path()).is_ok());
        assert_eq!(
            super::styles_stamp_at(root.path()),
            Some(super::StylesStamp::Embedded)
        );
        assert_no_live(root.path());
    }

    #[test]
    fn removing_the_default_files_from_the_root_changes_nothing() {
        let root = defaults_only_root();
        std::fs::remove_file(root.path().join("styles-default.json")).unwrap();
        assert!(
            super::load_styles_at(root.path())
                .unwrap()
                .get("S")
                .is_some()
        );
        assert!(super::load_classes_at(root.path()).is_ok());
        assert_eq!(
            super::styles_stamp_at(root.path()),
            Some(super::StylesStamp::Embedded)
        );
        assert_no_live(root.path());
    }

    #[test]
    fn first_run_overlay_inputs() {
        let root = tempfile::tempdir().unwrap();
        let styles = super::load_styles_at(root.path()).unwrap();
        for name in ["S", "A", "B", "C", "D", "E", "F"] {
            assert!(styles.get(name).is_some(), "{name}");
        }
        assert!(super::load_classes_at(root.path()).is_ok());
        let body = super::staged_economy_body(root.path(), None, PriceSourcePolicy::Auto).unwrap();
        assert!(!body.is_empty());
        assert_eq!(std::fs::read_dir(root.path()).unwrap().count(), 0);
    }

    #[test]
    fn a_broken_live_styles_file_names_its_full_path() {
        let root = tempfile::tempdir().unwrap();
        let live = root.path().join("styles.json");
        std::fs::write(&live, "{ not json").unwrap();
        let err = super::load_styles_at(root.path()).unwrap_err();
        assert!(err.contains(&live.display().to_string()), "{err}");
    }

    #[test]
    fn the_class_catalogue_ignores_a_config_json_in_the_folder() {
        let root = defaults_only_root();
        let (_, expected) = super::load_classes_at(root.path()).unwrap();
        std::fs::write(root.path().join("config.json"), "{ not json").unwrap();
        let (_, refs) = super::load_classes_at(root.path()).unwrap();
        assert_eq!(refs, expected);
    }

    #[test]
    fn styles_stamp_switches_on_live_create_and_delete() {
        let root = defaults_only_root();
        let default = root.path().join("styles-default.json");
        let live = root.path().join("styles.json");
        let first = super::styles_stamp_at(root.path()).unwrap();
        assert_eq!(first, super::StylesStamp::Embedded);

        std::fs::copy(&default, &live).unwrap();
        let created = super::styles_stamp_at(root.path()).unwrap();
        assert!(
            matches!(&created, super::StylesStamp::File(p, _) if *p == live),
            "{created:?}"
        );
        assert_ne!(created, first, "creating the live file is a change");

        std::fs::remove_file(&live).unwrap();
        let back = super::styles_stamp_at(root.path()).unwrap();
        assert_eq!(back, first, "deleting it returns to the embedded stamp");
        assert_ne!(back, created);
    }

    #[test]
    fn combined_apply_from_defaults_creates_live_styles_and_keeps_defaults() {
        let mut rig = rig_at(defaults_only_root(), S_RULE);
        let defaults = (
            rig.read("styles-default.json"),
            rig.read("tiers-default.json"),
        );
        assert_no_live(rig.root.path());
        assert!(rig.config.edit_style("S", StyleEdit::Font(Some(41))));
        let r = rig.apply(false);
        assert_eq!(
            r.outcome,
            AllOutcome::Applied {
                filter: Some(rig.path.clone())
            }
        );
        assert!(r.styles_written && !r.tiers_written);
        let styles = String::from_utf8(rig.read("styles.json")).unwrap();
        assert!(styles.contains("\"font\":41"), "{styles}");
        assert!(!rig.root.path().join("tiers.json").exists());
        let filter = std::fs::read_to_string(&rig.path).unwrap();
        assert!(filter.contains("SetFontSize 41"), "{filter}");
        assert_eq!(
            (
                rig.read("styles-default.json"),
                rig.read("tiers-default.json")
            ),
            defaults,
            "the -default.json files are never written"
        );
    }

    /// A rule tagged the way Apply writes tags: `<id> <name>`. A name-only (legacy) tag cannot
    /// follow a rename, which is why Apply migrates tags to this form.
    const KITE_RULE: &str = "# ff:style t5hw2k kite\nShow\n  Class \"Rings\"\n";

    #[test]
    fn an_added_style_reaches_the_rules_tab_before_apply() {
        let mut rig = rig(S_RULE);
        assert!(!super::sync_working_styles(&rig.config, &mut rig.working));
        rig.config.add_style("Fresh").unwrap();
        assert!(super::sync_working_styles(&rig.config, &mut rig.working));
        let names = rig.working.style_names();
        assert_eq!(names.last().map(String::as_str), Some("Fresh"));
        assert_eq!(names, rig.config.style_names());
        assert!(rig.config.is_dirty());
    }

    #[test]
    fn a_reorder_reaches_the_rules_tab_before_apply() {
        let mut rig = rig(S_RULE);
        rig.config.add_style("Fresh").unwrap();
        super::sync_working_styles(&rig.config, &mut rig.working);
        assert!(rig.config.move_style("Fresh", true));
        assert!(super::sync_working_styles(&rig.config, &mut rig.working));
        assert_eq!(rig.working.style_names(), rig.config.style_names());
        assert!(!super::sync_working_styles(&rig.config, &mut rig.working));
    }

    #[test]
    fn sync_is_a_no_op_when_the_sets_match() {
        let mut rig = rig(S_RULE);
        rig.config.add_style("Fresh").unwrap();
        assert!(super::sync_working_styles(&rig.config, &mut rig.working));
        // false means set_styles (which bumps the styles revision) was not called.
        assert!(!super::sync_working_styles(&rig.config, &mut rig.working));
        assert!(!super::sync_working_styles(&rig.config, &mut rig.working));
    }

    #[test]
    fn a_rename_keeps_the_tagged_rule_linked_and_writes_nothing() {
        let mut rig = rig(KITE_RULE);
        let styles = rig.read("styles.json");
        let filter = std::fs::read(&rig.path).unwrap();
        rig.config.rename_style("kite", "locator").unwrap();
        super::sync_working_styles(&rig.config, &mut rig.working);
        assert_eq!(
            rig.working.rows()[0].style,
            crate::overlay::rules::StyleState::Known("locator".to_string())
        );
        assert!(rig.working.unknown_styles().is_empty());
        assert!(rig.config.is_dirty());
        assert_eq!(rig.read("styles.json"), styles);
        assert_eq!(std::fs::read(&rig.path).unwrap(), filter);
    }

    #[test]
    fn apply_after_a_rename_writes_the_new_name_under_the_same_id() {
        let mut rig = rig(KITE_RULE);
        let id = rig.config.style_id("kite").unwrap().to_string();
        rig.config.rename_style("kite", "locator").unwrap();
        super::sync_working_styles(&rig.config, &mut rig.working);
        let r = rig.apply(false);
        assert!(
            matches!(r.outcome, AllOutcome::Applied { .. }),
            "{:?}",
            r.outcome
        );
        let styles = String::from_utf8(rig.read("styles.json")).unwrap();
        assert!(styles.contains("\"locator\""), "{styles}");
        assert!(!styles.contains("\"kite\""), "{styles}");
        assert!(styles.contains(&id), "{styles}");
        let filter = std::fs::read_to_string(&rig.path).unwrap();
        assert!(filter.contains(&format!("{id} locator")), "{filter}");
    }

    /// CR-01: legacy tag -> rename -> sync -> RULES revert -> Apply keeps the link.
    #[test]
    fn a_rules_revert_after_a_rename_keeps_the_legacy_tag_linked() {
        let mut rig = rig("# ff:style kite\nShow\n  Class \"Rings\"\n");
        let id = rig.config.style_id("kite").unwrap().to_string();
        rig.config.rename_style("kite", "locator").unwrap();
        super::sync_working_styles(&rig.config, &mut rig.working);
        rig.working.revert();
        assert!(!super::sync_working_styles(&rig.config, &mut rig.working));
        assert!(rig.working.unknown_styles().is_empty());
        let r = rig.apply(false);
        assert!(
            matches!(r.outcome, AllOutcome::Applied { .. }),
            "{:?}",
            r.outcome
        );
        let filter = std::fs::read_to_string(&rig.path).unwrap();
        assert!(
            filter.contains(&format!("# ff:style {id} locator\n")),
            "{filter}"
        );
    }

    /// WR-04: a CONFIG revert after a rename leaves no phantom RULES edit.
    #[test]
    fn a_config_revert_after_a_rename_leaves_the_rules_clean() {
        let mut rig = rig("# ff:style kite\nShow\n  Class \"Rings\"\n");
        rig.config.rename_style("kite", "locator").unwrap();
        super::sync_working_styles(&rig.config, &mut rig.working);
        assert!(!rig.working.is_dirty());
        rig.config.revert();
        super::sync_working_styles(&rig.config, &mut rig.working);
        assert!(!rig.working.is_dirty());
        assert!(rig.working.unknown_styles().is_empty());
    }

    /// A RELOAD while a rename is pending re-parses the disk text; it still follows the rename.
    #[test]
    fn a_reloaded_document_follows_a_pending_rename() {
        let mut rig = rig("# ff:style kite\nShow\n");
        rig.config.rename_style("kite", "locator").unwrap();
        super::sync_working_styles(&rig.config, &mut rig.working);
        let mut fresh = WorkingFilter::from_text(rig.loaded.text());
        assert!(super::sync_working_styles(&rig.config, &mut fresh));
        assert_eq!(
            fresh.rows()[0].style,
            crate::overlay::rules::StyleState::Known("locator".to_string())
        );
    }

    #[test]
    fn revert_brings_the_rules_tab_back_to_the_applied_set() {
        let mut rig = rig(S_RULE);
        let applied = rig.working.style_names();
        rig.config.add_style("Fresh").unwrap();
        super::sync_working_styles(&rig.config, &mut rig.working);
        assert_ne!(rig.working.style_names(), applied);
        rig.config.revert();
        assert!(super::sync_working_styles(&rig.config, &mut rig.working));
        assert_eq!(rig.working.style_names(), applied);
    }

    #[test]
    fn combined_apply_refusal_writes_nothing() {
        let mut rig = rig("Show\n  Rarity Bogus\n");
        rig.working.apply_action(RowAction::AddRule);
        assert!(rig.config.edit_style("S", StyleEdit::Font(Some(41))));
        let (styles, tiers) = (rig.read("styles.json"), rig.read("tiers.json"));
        let filter = std::fs::read(&rig.path).unwrap();
        let r = rig.apply(false);
        match r.outcome {
            AllOutcome::Failed(m) => assert!(m.starts_with("Apply refused"), "{m}"),
            other => panic!("{other:?}"),
        }
        assert!(r.changed.is_empty());
        assert_eq!(rig.read("styles.json"), styles);
        assert_eq!(rig.read("tiers.json"), tiers);
        assert_eq!(std::fs::read(&rig.path).unwrap(), filter);
        assert!(rig.config.is_dirty(), "config edits survive a refusal");
        assert!(rig.working.is_dirty(), "rule edits survive a refusal");
    }

    #[test]
    fn changed_on_disk_then_overwrite() {
        let mut rig = rig(S_RULE);
        assert!(rig.config.edit_style("S", StyleEdit::Font(Some(41))));
        // Someone else touches styles.json.
        let mut outside = rig.read("styles.json");
        outside.push(b'\n');
        std::fs::write(rig.root.path().join("styles.json"), &outside).unwrap();
        let r = rig.apply(false);
        match &r.outcome {
            AllOutcome::Failed(m) => assert!(m.contains("styles.json"), "{m}"),
            other => panic!("{other:?}"),
        }
        assert_eq!(r.changed, vec![DiskFile::Styles]);
        assert_eq!(rig.read("styles.json"), outside, "nothing was written");
        assert!(rig.config.is_dirty());

        let r = rig.apply(true);
        assert!(
            matches!(r.outcome, AllOutcome::Applied { .. }),
            "{:?}",
            r.outcome
        );
        assert!(r.changed.is_empty());
        let styles = String::from_utf8(rig.read("styles.json")).unwrap();
        assert!(styles.contains("\"font\":41"), "{styles}");
        assert!(
            std::fs::read_to_string(&rig.path)
                .unwrap()
                .contains("SetFontSize 41")
        );
        // The written files are the new baselines: a plain Apply goes through.
        let again = rig.apply(false);
        assert!(matches!(again.outcome, AllOutcome::Applied { .. }));
        assert!(again.changed.is_empty());
    }

    #[test]
    fn config_load_error_reload_retries_the_config_and_spares_the_filter() {
        use super::{ConfigReload, config_reload_plan};
        // Config failed to load, the filter was refused as changed on disk: "Reload from disk"
        // retries the config and must not reload the filter (which would drop RULES edits).
        assert_eq!(
            config_reload_plan(true, &[DiskFile::Filter]),
            ConfigReload::ConfigOnly
        );
        assert_eq!(config_reload_plan(true, &[]), ConfigReload::ConfigOnly);
        assert_eq!(config_reload_plan(false, &[]), ConfigReload::ConfigOnly);
        // The footer RELOAD after a refusal keeps the shared semantics.
        for changed in [
            &[DiskFile::Filter][..],
            &[DiskFile::Styles],
            &[DiskFile::Tiers, DiskFile::Filter],
        ] {
            assert_eq!(config_reload_plan(false, changed), ConfigReload::Changed);
        }
    }

    #[test]
    fn bar_reload_on_config_reloads_the_filter_unless_a_refusal_is_pending() {
        use super::{BarReload, ShellTab, bar_reload_target};
        // CR-01: without a changed-on-disk refusal the bar's Reload is live only because of the
        // filter (damaged, failed, no root); on CONFIG it must re-read the filter, which keeps
        // CONFIG edits, rather than reload the config and drop them.
        assert_eq!(bar_reload_target(ShellTab::Config, &[]), BarReload::Shared);
        assert_eq!(
            bar_reload_target(ShellTab::Config, &[DiskFile::Styles]),
            BarReload::ConfigRefusal
        );
        for tab in [ShellTab::Rules, ShellTab::Economy, ShellTab::About] {
            assert_eq!(bar_reload_target(tab, &[]), BarReload::Shared);
            assert_eq!(
                bar_reload_target(tab, &[DiskFile::Filter]),
                BarReload::Shared
            );
        }
    }

    #[test]
    fn clean_config_follows_outside_edits_and_never_writes_them_back() {
        let mut rig = rig(S_RULE);
        rig.working.apply_action(RowAction::AddRule);
        // Outside edits to both config files while CONFIG holds no edits.
        let styles_path = rig.root.path().join("styles.json");
        let original = std::fs::read_to_string(&styles_path).unwrap();
        let styles = original.replacen("\"font\": 37", "\"font\": 38", 1);
        assert_ne!(styles, original);
        std::fs::write(&styles_path, &styles).unwrap();
        let tiers_path = rig.root.path().join("tiers.json");
        let mut tiers = std::fs::read_to_string(&tiers_path).unwrap();
        tiers.push('\n');
        std::fs::write(&tiers_path, &tiers).unwrap();

        // A RULES-only Apply goes through, renders with the disk styles, writes neither file.
        let r = rig.apply(false);
        assert!(
            matches!(r.outcome, AllOutcome::Applied { .. }),
            "{:?}",
            r.outcome
        );
        assert!(r.changed.is_empty() && !r.styles_written && !r.tiers_written);
        assert_eq!(rig.read("styles.json"), styles.as_bytes());
        assert_eq!(rig.read("tiers.json"), tiers.as_bytes());
        let filter = std::fs::read_to_string(&rig.path).unwrap();
        assert!(filter.contains("SetFontSize 38"), "{filter}");
        assert!(!rig.config.is_dirty());

        // OVERWRITE never rewrites a clean config file either.
        let styles = styles.replacen("\"font\": 38", "\"font\": 39", 1);
        std::fs::write(&styles_path, &styles).unwrap();
        rig.working.apply_action(RowAction::AddRule);
        let r = rig.apply(true);
        assert!(
            matches!(r.outcome, AllOutcome::Applied { .. }),
            "{:?}",
            r.outcome
        );
        assert!(!r.styles_written && !r.tiers_written);
        assert_eq!(rig.read("styles.json"), styles.as_bytes());
        assert_eq!(rig.read("tiers.json"), tiers.as_bytes());
        let filter = std::fs::read_to_string(&rig.path).unwrap();
        assert!(filter.contains("SetFontSize 39"), "{filter}");
    }

    #[test]
    fn reload_changed_discards_only_the_owning_side() {
        use super::reload_changed_files;
        let outside = "Show\n  Class \"Belts\"\n";
        for which in [DiskFile::Filter, DiskFile::Styles, DiskFile::Tiers] {
            let Rig {
                root,
                _game,
                path,
                loaded,
                mut working,
                mut config,
            } = rig(RULES);
            working.apply_action(RowAction::AddRule);
            assert!(config.edit_style("S", StyleEdit::Font(Some(41))));
            let (mut config, mut filter) = (Ok(config), Ok(loaded));
            let mut working = Some(working);
            match which {
                DiskFile::Filter => std::fs::write(&path, outside).unwrap(),
                DiskFile::Styles => {
                    let p = root.path().join("styles.json");
                    let text = std::fs::read_to_string(&p).unwrap();
                    std::fs::write(p, text.replacen("\"font\":29", "\"font\":30", 1)).unwrap();
                }
                DiskFile::Tiers => {
                    let p = root.path().join("tiers.json");
                    let mut text = std::fs::read_to_string(&p).unwrap();
                    text.push('\n');
                    std::fs::write(p, text).unwrap();
                }
            }
            reload_changed_files(
                root.path(),
                &[which],
                &mut config,
                &mut filter,
                &mut working,
            );
            let (config, filter, working) = (config.unwrap(), filter.unwrap(), working.unwrap());
            if which == DiskFile::Filter {
                // The filter's owner (RULES) lost its edits; CONFIG kept its own.
                assert_eq!(filter.text(), outside);
                assert_eq!(working.text(), outside);
                assert!(!working.is_dirty());
                assert!(config.is_dirty(), "config edits survive a filter reload");
            } else {
                // The config files' owner (CONFIG) lost its edits; RULES kept its own.
                assert!(!config.is_dirty(), "{which:?}");
                assert!(working.is_dirty(), "rule edits survive a config reload");
                assert_eq!(filter.text(), RULES);
            }
        }
    }

    #[test]
    fn per_tab_revert_is_independent() {
        let mut rig = rig(RULES);
        rig.working.apply_action(RowAction::AddRule);
        assert!(rig.config.edit_style("S", StyleEdit::Font(Some(41))));

        // RULES revert: config edits remain, so the indicator stays out of date.
        rig.working.revert();
        assert!(!rig.working.is_dirty() && rig.config.is_dirty());
        assert_eq!(
            apply::state_after_revert(true, rig.config.is_dirty()),
            apply::ApplyState::Unapplied
        );

        // CONFIG revert: rule edits remain.
        rig.working.apply_action(RowAction::AddRule);
        rig.config.revert();
        assert!(!rig.config.is_dirty() && rig.working.is_dirty());
        assert_eq!(
            apply::state_after_revert(true, rig.working.is_dirty()),
            apply::ApplyState::Unapplied
        );

        // Both reverted: back to the session's baseline.
        rig.working.revert();
        assert_eq!(
            apply::state_after_revert(true, false),
            apply::ApplyState::Applied
        );
    }

    #[test]
    fn no_filter_apply_writes_config_only() {
        let root = config_root();
        let mut config = ConfigState::load(root.path()).unwrap();
        assert!(config.edit_style("S", StyleEdit::Font(Some(41))));
        let tiers = std::fs::read(root.path().join("tiers.json")).unwrap();
        let r = apply_everything(
            root.path(),
            Some(&mut config),
            None,
            PriceSourcePolicy::Auto,
            false,
        );
        assert_eq!(r.outcome, AllOutcome::Applied { filter: None });
        assert!(r.styles_written && !r.tiers_written);
        assert!(!config.is_dirty());
        let styles = std::fs::read_to_string(root.path().join("styles.json")).unwrap();
        assert!(styles.contains("\"font\":41"));
        assert_eq!(
            std::fs::read(root.path().join("tiers.json")).unwrap(),
            tiers
        );
    }

    #[test]
    fn apply_working_writes_edits_then_identical_bytes_still_count_as_applied() {
        let dir = tempfile::tempdir().unwrap();
        let (path, mut loaded) = loaded_in(&dir, RULES);
        let mut working = WorkingFilter::from_text(loaded.text());
        working.apply_action(RowAction::AddRule);
        assert!(working.is_dirty());
        let (outcome, _, reload) = apply_working(dir.path(), &mut loaded, &mut working);
        assert!(matches!(outcome, ApplyOutcome::Written(_)), "{outcome:?}");
        assert!(!reload);
        assert!(!working.is_dirty());
        assert_eq!(std::fs::read_to_string(&path).unwrap(), working.text());
        let mtime = std::fs::metadata(&path).unwrap().modified().unwrap();
        // Identical bytes: reported as applied (so the in-game reload runs), but not rewritten.
        let (again, _, _) = apply_working(dir.path(), &mut loaded, &mut working);
        assert_eq!(again, ApplyOutcome::Written(path.clone()));
        assert_eq!(std::fs::metadata(&path).unwrap().modified().unwrap(), mtime);
    }

    #[test]
    fn overwrite_working_replaces_an_external_edit_and_adopts_the_new_baseline() {
        let dir = tempfile::tempdir().unwrap();
        let (path, mut loaded) = loaded_in(&dir, RULES);
        let mut working = WorkingFilter::from_text(loaded.text());
        working.apply_action(RowAction::AddRule);
        let outside = "Show\n  Class \"Belts\"\n";
        std::fs::write(&path, outside).unwrap();
        let (refused, _, reload) = apply_working(dir.path(), &mut loaded, &mut working);
        assert!(matches!(refused, ApplyOutcome::Failed(_)) && reload);

        let (outcome, _, reload) = overwrite_working(dir.path(), &mut loaded, &mut working);
        assert_eq!(outcome, ApplyOutcome::Written(path.clone()));
        assert!(!reload);
        assert!(!working.is_dirty());
        assert_eq!(std::fs::read_to_string(&path).unwrap(), working.text());
        // The one-time backup holds the bytes the overwrite replaced.
        assert_eq!(
            std::fs::read_to_string(crate::safe_write::backup_path(&path)).unwrap(),
            outside
        );
        // The written file is the new baseline: a plain Apply now goes through.
        let (again, _, reload) = apply_working(dir.path(), &mut loaded, &mut working);
        assert!(
            matches!(again, ApplyOutcome::Written(_)) && !reload,
            "{again:?}"
        );
    }

    #[test]
    fn failed_overwrite_keeps_the_old_baseline() {
        let dir = tempfile::tempdir().unwrap();
        let bad = "Show\n  Rarity Bogus\n";
        let (path, mut loaded) = loaded_in(&dir, RULES);
        let mut working = WorkingFilter::from_text(bad);
        std::fs::write(&path, "Show\n  Class \"Belts\"\n").unwrap();
        let (outcome, _, _) = overwrite_working(dir.path(), &mut loaded, &mut working);
        assert!(matches!(outcome, ApplyOutcome::Failed(_)), "{outcome:?}");
        assert_eq!(loaded.disk(), Some(RULES.as_bytes()));
        assert_eq!(
            std::fs::read_to_string(&path).unwrap(),
            "Show\n  Class \"Belts\"\n"
        );
    }

    #[test]
    fn apply_working_refuses_enabled_grammar_errors_and_writes_nothing() {
        let dir = tempfile::tempdir().unwrap();
        let bad = "Show\n  Rarity Bogus\n\nHide\n";
        let (path, mut loaded) = loaded_in(&dir, bad);
        let mut working = WorkingFilter::from_text(loaded.text());
        let (outcome, _, reload) = apply_working(dir.path(), &mut loaded, &mut working);
        match outcome {
            ApplyOutcome::Failed(m) => assert!(m.starts_with("Apply refused"), "{m}"),
            other => panic!("{other:?}"),
        }
        assert!(!reload);
        assert_eq!(std::fs::read_to_string(&path).unwrap(), bad);
    }

    #[test]
    fn apply_working_after_an_external_edit_demands_reload() {
        let dir = tempfile::tempdir().unwrap();
        let (path, mut loaded) = loaded_in(&dir, RULES);
        let mut working = WorkingFilter::from_text(loaded.text());
        working.apply_action(RowAction::AddRule);
        std::fs::write(&path, "Show\n  Class \"Belts\"\n").unwrap();
        let (outcome, _, reload) = apply_working(dir.path(), &mut loaded, &mut working);
        assert_eq!(
            outcome,
            ApplyOutcome::Failed(apply::changed_on_disk_message(&[
                "dynamic.filter".to_string()
            ]))
        );
        assert!(reload);
        assert!(working.is_dirty(), "edits survive until RELOAD");
        assert_eq!(
            std::fs::read_to_string(&path).unwrap(),
            "Show\n  Class \"Belts\"\n"
        );
    }

    #[test]
    fn browsing_economy_rules_then_apply_writes_the_same_bytes() {
        use crate::overlay::rule_list::economy_browse;
        // The repo root is read-only input here (styles, tiers, prices).
        let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let text = economy_browse::rich_economy_text();
        let apply_once = |browse: bool| {
            let dir = tempfile::tempdir().unwrap();
            let (path, mut loaded) = loaded_in(&dir, &text);
            let mut working = WorkingFilter::from_text(loaded.text());
            if browse {
                let report = economy_browse::browse(&working);
                assert_eq!(report.rules, 2);
                assert!(report.actions.is_empty(), "{:?}", report.actions);
                // Through the list as the RULES tab draws it: whatever its frames report is
                // applied, as the app does after each frame.
                let (actions, clicked) = economy_browse::browse_list(&mut working);
                assert!(clicked > 40, "{clicked}");
                assert!(actions.is_empty(), "{actions:?}");
                for a in actions {
                    working.apply_action(a);
                }
                assert!(!working.is_dirty());
            }
            let (outcome, _, _) = apply_working(&root, &mut loaded, &mut working);
            assert!(matches!(outcome, ApplyOutcome::Written(_)), "{outcome:?}");
            std::fs::read(&path).unwrap()
        };
        assert_eq!(apply_once(true), apply_once(false));
    }

    #[test]
    fn apply_working_adds_the_economy_section_from_the_repo_inputs() {
        let dir = tempfile::tempdir().unwrap();
        let (path, mut loaded) = loaded_in(&dir, RULES);
        let mut working = WorkingFilter::from_text(loaded.text());
        // The repo root is read-only input here (styles, tiers, fallback prices).
        let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let body = crate::dynamic::economy_body(
            &crate::compile::InputPaths::relative_to(&root),
            crate::compile::PriceSourcePolicy::Auto,
        )
        .unwrap()
        .0;
        working.stage_add_economy(&body);
        let (outcome, _, _) = apply_working(&root, &mut loaded, &mut working);
        assert!(matches!(outcome, ApplyOutcome::Written(_)), "{outcome:?}");
        let written = std::fs::read_to_string(&path).unwrap();
        assert!(crate::filter::section::has_section(&written).unwrap());
    }

    #[test]
    fn adding_a_rule_clears_the_search_expands_and_focuses_it() {
        let mut working = WorkingFilter::from_text(RULES);
        let mut list = ListUi::default();
        let mut query = String::from("zzz");
        let (changed, status) = apply_row_actions(
            &mut working,
            &mut list,
            &mut query,
            vec![RowAction::AddRule],
        );
        assert!(changed);
        assert_eq!(status.as_deref(), Some(apply::RULE_ADDED_MESSAGE));
        assert!(query.is_empty());
        let new = working.rows()[0].id;
        assert_eq!(list.expanded, Some(new));
        assert_eq!(list.pending_scroll, Some(new));
        assert_eq!(list.focus_name, Some(new));
    }

    #[test]
    fn deleting_the_expanded_row_collapses_it() {
        let mut working = WorkingFilter::from_text(RULES);
        let mut list = ListUi::default();
        let id = working.rows()[0].id;
        list.expanded = Some(id);
        let (changed, _) = apply_row_actions(
            &mut working,
            &mut list,
            &mut String::new(),
            vec![RowAction::Delete(id)],
        );
        assert!(changed);
        assert!(list.expanded.is_none());
    }

    // ---- styles on Apply (Phase 20-06) ----

    const S_RULE: &str = "#ff:style S\nHide\n  Class \"Rings\"\n  SetFontSize 20\n";

    fn styles_root() -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        std::fs::copy(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("tests/fixtures/seed-styles.json"),
            dir.path().join("styles.json"),
        )
        .unwrap();
        dir
    }

    fn working_with_styles(root: &std::path::Path, text: &str) -> WorkingFilter {
        let mut w = WorkingFilter::from_text(text);
        w.set_styles(super::load_styles_at(root));
        w
    }

    #[test]
    fn apply_working_style_rewrites_then_reports_nothing_more() {
        let root = styles_root();
        let (path, mut loaded) = loaded_in(&root, S_RULE);
        let mut working = working_with_styles(root.path(), loaded.text());
        let (outcome, extras, _) = apply_working(root.path(), &mut loaded, &mut working);
        assert!(matches!(outcome, ApplyOutcome::Written(_)), "{outcome:?}");
        assert!(extras.contains("Updated 1 styled rule."), "{extras}");
        let written = std::fs::read_to_string(&path).unwrap();
        assert!(written.contains("SetFontSize 37") && !written.contains("SetFontSize 20"));
        assert!(written.contains("\nShow\n"), "{written}");
        let (again, extras, _) = apply_working(root.path(), &mut loaded, &mut working);
        assert!(matches!(again, ApplyOutcome::Written(_)), "{again:?}");
        assert_eq!(std::fs::read_to_string(&path).unwrap(), written);
        assert!(!extras.contains("Updated"), "{extras}");
    }

    #[test]
    fn apply_working_style_change_reaches_the_next_apply() {
        let root = styles_root();
        let (path, mut loaded) = loaded_in(&root, S_RULE);
        let mut working = working_with_styles(root.path(), loaded.text());
        apply_working(root.path(), &mut loaded, &mut working);
        let styles_path = root.path().join("styles.json");
        let edited = std::fs::read_to_string(&styles_path).unwrap().replacen(
            "\"font\": 37",
            "\"font\": 40",
            1,
        );
        assert!(edited.contains("\"font\": 40"), "fixture shape changed");
        std::fs::write(&styles_path, edited).unwrap();
        working.set_styles(super::load_styles_at(root.path()));
        assert!(working.style_rewrite_pending());
        let (outcome, extras, _) = apply_working(root.path(), &mut loaded, &mut working);
        assert!(matches!(outcome, ApplyOutcome::Written(_)), "{outcome:?}");
        assert!(extras.contains("Updated 1 styled rule."), "{extras}");
        assert!(
            std::fs::read_to_string(&path)
                .unwrap()
                .contains("SetFontSize 40")
        );
    }

    #[test]
    fn apply_working_style_unknown_tag_is_dropped_keeping_actions() {
        let root = styles_root();
        let text = "Show\n  Class \"Bows\"\n\n#ff:style Gone\nHide\n  SetFontSize 40\n";
        let (path, mut loaded) = loaded_in(&root, text);
        let mut working = working_with_styles(root.path(), loaded.text());
        let (_, extras, _) = apply_working(root.path(), &mut loaded, &mut working);
        assert!(
            extras.contains(
                "Removed the style link from 1 rule whose style no longer exists (first: rule 2)."
            ),
            "{extras}"
        );
        let written = std::fs::read_to_string(&path).unwrap();
        assert!(!written.contains("ff:style Gone"), "{written}");
        assert!(written.contains("SetFontSize 40"), "{written}");
        let (outcome, extras, _) = apply_working(root.path(), &mut loaded, &mut working);
        assert_eq!(outcome, ApplyOutcome::Written(path.clone()), "{extras}");
        assert_eq!(std::fs::read_to_string(&path).unwrap(), written);
        assert!(!extras.contains("Removed the style link"), "{extras}");
    }

    #[test]
    fn apply_working_style_unreadable_styles_refuses_and_keeps_edits() {
        let root = styles_root();
        let (path, mut loaded) = loaded_in(&root, S_RULE);
        std::fs::write(root.path().join("styles.json"), "{ not json").unwrap();
        let mut working = working_with_styles(root.path(), loaded.text());
        working.apply_action(RowAction::AddRule);
        let before = working.text();
        let (outcome, _, reload) = apply_working(root.path(), &mut loaded, &mut working);
        match outcome {
            ApplyOutcome::Failed(m) => assert!(
                m.starts_with("The styles could not be read, so styled rules")
                    && m.contains("styles.json"),
                "{m}"
            ),
            other => panic!("{other:?}"),
        }
        assert!(!reload);
        assert_eq!(working.text(), before);
        assert!(working.is_dirty());
        assert_eq!(std::fs::read_to_string(&path).unwrap(), S_RULE);
    }

    #[test]
    fn apply_working_style_unreadable_styles_does_not_block_unstyled_filters() {
        let root = styles_root();
        let (_, mut loaded) = loaded_in(&root, RULES);
        std::fs::write(root.path().join("styles.json"), "{ not json").unwrap();
        let mut working = working_with_styles(root.path(), loaded.text());
        working.apply_action(RowAction::AddRule);
        let (outcome, _, _) = apply_working(root.path(), &mut loaded, &mut working);
        assert!(matches!(outcome, ApplyOutcome::Written(_)), "{outcome:?}");
    }

    #[test]
    fn apply_working_style_failed_changed_on_disk_leaves_edits_alone() {
        let root = styles_root();
        let (path, mut loaded) = loaded_in(&root, S_RULE);
        let mut working = working_with_styles(root.path(), loaded.text());
        std::fs::write(&path, "Show\n").unwrap();
        let before = working.text();
        let (outcome, _, reload) = apply_working(root.path(), &mut loaded, &mut working);
        assert!(matches!(outcome, ApplyOutcome::Failed(_)));
        assert!(reload);
        assert_eq!(working.text(), before, "rewrite must not leak into edits");
        assert!(working.text().contains("SetFontSize 20"));
    }

    #[test]
    fn a_fold_click_does_not_overwrite_an_unusable_settings_file() {
        let dir = tempfile::tempdir().unwrap();
        let bad = "{ \"filter\": \"x.filter\", }";
        std::fs::write(crate::settings::path(dir.path()), bad).unwrap();
        let (loaded, warning) = crate::settings::load(dir.path());
        assert!(warning.is_some());
        let next = loaded.set_section_collapsed("dynamic.filter", "Gear", true);
        assert!(!crate::settings::save_section_state(dir.path(), &next).unwrap());
        assert_eq!(
            std::fs::read_to_string(crate::settings::path(dir.path())).unwrap(),
            bad
        );
    }

    #[test]
    fn a_refused_section_edit_keeps_its_fold_flag() {
        let r = crate::filter::section::RULE_LINE;
        // "After" directly follows the economy section, so removing it is refused.
        let mut working = WorkingFilter::from_text(&format!(
            "{r}
# ff:section Old
{r}

Show

{r}
{}
{r}

Hide

             {r}
# ff:section After
{r}

Show
",
            crate::filter::section::BEGIN_LINE
        ));
        assert!(working.sections()[1].economy);
        let f = "dynamic.filter";
        let mut fold = crate::settings::Settings::default().set_section_collapsed(f, "After", true);
        let before = fold.clone();
        let (changed, _) = super::apply_row_actions_folding(
            &mut working,
            &mut ListUi::default(),
            &mut String::new(),
            vec![RowAction::RemoveSection(2)],
            Some(&mut fold),
        );
        assert!(!changed);
        assert_eq!(fold, before, "a refused remove must not drop the flag");
        // A rename that is refused (the economy banner is locked) moves nothing either.
        let (changed, _) = super::apply_row_actions_folding(
            &mut working,
            &mut ListUi::default(),
            &mut String::new(),
            vec![RowAction::RenameSection {
                ordinal: 1,
                name: "After".into(),
            }],
            Some(&mut fold),
        );
        assert!(!changed);
        assert_eq!(fold, before);
    }

    #[test]
    fn a_batch_of_removes_drops_the_flags_of_the_sections_it_removed() {
        let r = crate::filter::section::RULE_LINE;
        let mut working = WorkingFilter::from_text(&format!(
            "{r}
# ff:section A
{r}

Show

{r}
# ff:section B
{r}

Show

             {r}
# ff:section C
{r}

Show
"
        ));
        let f = "dynamic.filter";
        let mut fold = crate::settings::Settings::default()
            .set_section_collapsed(f, "A", true)
            .set_section_collapsed(f, "B", true)
            .set_section_collapsed(f, "C", true);
        let (changed, _) = super::apply_row_actions_folding(
            &mut working,
            &mut ListUi::default(),
            &mut String::new(),
            vec![RowAction::RemoveSection(0), RowAction::RemoveSection(0)],
            Some(&mut fold),
        );
        assert!(changed);
        assert!(!fold.section_collapsed(f, "A", false));
        assert!(!fold.section_collapsed(f, "B", false));
        assert!(fold.section_collapsed(f, "C", false), "C still exists");
    }

    #[test]
    fn section_state_follows_rename_remove_and_add() {
        let r = crate::filter::section::RULE_LINE;
        let working = WorkingFilter::from_text(&format!(
            "{r}
# ff:section Old
{r}

Show

{r}
# ff:section ff:economy
{r}

Hide
"
        ));
        let f = "dynamic.filter";
        let base = crate::settings::Settings::default()
            .set_section_collapsed(f, "Old", true)
            .set_section_collapsed(f, "Section", true);
        let rename = RowAction::RenameSection {
            ordinal: 0,
            name: "New".into(),
        };
        let n = super::section_state_after(&working, &base, &[rename]);
        assert!(n.section_collapsed(f, "New", false));
        assert!(!n.section_collapsed(f, "Old", false), "no stale flag left");
        let n = super::section_state_after(&working, &base, &[RowAction::RemoveSection(0)]);
        assert!(!n.section_collapsed(f, "Old", false));
        let n = super::section_state_after(&working, &base, &[RowAction::AddSection]);
        assert!(
            !n.section_collapsed(f, "Section", false),
            "new starts expanded"
        );
        // A user section named like the economy key does not share its state.
        let key = crate::settings::section_key(&working.sections()[1].name, false);
        assert_ne!(key, crate::settings::ECONOMY_SECTION_KEY);
    }

    #[test]
    fn a_rule_in_a_collapsed_section_is_revealed() {
        let r = crate::filter::section::RULE_LINE;
        let working = WorkingFilter::from_text(&format!(
            "Show

{r}
# ff:section Gear
{r}

Hide
"
        ));
        let settings = crate::settings::Settings::default().set_section_collapsed(
            "dynamic.filter",
            "Gear",
            true,
        );
        let top = working.rows()[0].id;
        let inner = working.rows()[1].id;
        assert_eq!(super::section_to_reveal(&working, &settings, top), None);
        assert_eq!(
            super::section_to_reveal(&working, &settings, inner).as_deref(),
            Some("Gear")
        );
        let open = crate::settings::Settings::default();
        assert_eq!(super::section_to_reveal(&working, &open, inner), None);
    }

    #[test]
    fn apply_row_actions_coalesces_repeated_line_edits() {
        let mut working = WorkingFilter::from_text("Show\n  ItemLevel >= 70\n");
        let id = working.rows()[0].id;
        let ed = |v: &str| RowAction::EditLine {
            id,
            line: 1,
            expect: "  ItemLevel >= 70".into(),
            patch: crate::filter::LinePatch::Values(v.into()),
        };
        let mut list = ListUi::default();
        let (changed, status) = apply_row_actions(
            &mut working,
            &mut list,
            &mut String::new(),
            vec![ed("71"), ed("72")],
        );
        assert!(changed);
        assert_eq!(status, None, "the stale first edit must not run");
        assert_eq!(working.text(), "Show\n  ItemLevel >= 72\n");
    }

    #[test]
    fn add_section_opens_its_name_box_and_scrolls_to_the_top() {
        let mut working = WorkingFilter::from_text(
            "Show
  Class \"Rings\"
",
        );
        let mut list = ListUi::default();
        let mut query = "rings".to_string();
        let (changed, status) = apply_row_actions(
            &mut working,
            &mut list,
            &mut query,
            vec![RowAction::AddSection],
        );
        assert!(changed);
        assert!(
            query.is_empty(),
            "the search is cleared so the divider shows"
        );
        assert!(list.scroll_top && list.section_focus);
        assert_eq!(list.section_draft, "Section");
        assert_eq!(list.editing_section, Some(working.sections()[0].ordinal));
        assert_eq!(
            status.as_deref(),
            Some("Section added at the top. Press Apply to save it.")
        );
        // Removing it ends the rename.
        let ord = working.sections()[0].ordinal;
        apply_row_actions(
            &mut working,
            &mut list,
            &mut query,
            vec![RowAction::RemoveSection(ord)],
        );
        assert!(list.editing_section.is_none());
    }

    /// WR-07: a removed line re-keys the cards after it, so their egui temp state is orphaned;
    /// a value edit keeps it.
    #[test]
    fn remove_line_orphans_card_state() {
        let mut working = WorkingFilter::from_text("Show\n  BaseType \"A\"\n  BaseType \"B\"\n");
        let id = working.rows()[0].id;
        let mut list = ListUi::default();
        let epoch = list.epoch;
        let ed = RowAction::EditLine {
            id,
            line: 2,
            expect: "  BaseType \"B\"".into(),
            patch: crate::filter::LinePatch::Values("\"C\"".into()),
        };
        assert!(apply_row_actions(&mut working, &mut list, &mut String::new(), vec![ed]).0);
        assert_eq!(list.epoch, epoch);
        let rm = RowAction::RemoveLine {
            id,
            line: 1,
            expect: "  BaseType \"A\"".into(),
        };
        assert!(apply_row_actions(&mut working, &mut list, &mut String::new(), vec![rm]).0);
        assert_ne!(list.epoch, epoch);
    }

    /// IN-07: a raw whole-line edit that changes the keyword, and any added line, also re-key
    /// the later cards; a whole-line edit that keeps the keyword does not.
    #[test]
    fn keyword_changing_edits_orphan_card_state() {
        let mut working = WorkingFilter::from_text("Show\n  BaseTyp \"A\"\n  BaseType \"B\"\n");
        let id = working.rows()[0].id;
        let mut list = ListUi::default();
        let whole = |line: usize, expect: &str, text: &str| RowAction::EditLine {
            id,
            line,
            expect: expect.into(),
            patch: crate::filter::LinePatch::Whole(text.into()),
        };
        let epoch = list.epoch;
        let same = whole(2, "  BaseType \"B\"", "basetype \"C\"");
        assert!(apply_row_actions(&mut working, &mut list, &mut String::new(), vec![same]).0);
        assert_eq!(list.epoch, epoch, "same keyword, ordinals unchanged");
        let fix = whole(1, "  BaseTyp \"A\"", "BaseType \"A\"");
        assert!(apply_row_actions(&mut working, &mut list, &mut String::new(), vec![fix]).0);
        assert_ne!(list.epoch, epoch, "a changed keyword shifts ordinals");
        let epoch = list.epoch;
        let add = RowAction::AddLine {
            id,
            kind: crate::overlay::rules::LineKind::Condition,
            text: "ItemLevel >= 70".into(),
        };
        assert!(apply_row_actions(&mut working, &mut list, &mut String::new(), vec![add]).0);
        assert_ne!(list.epoch, epoch, "an added line can shift ordinals");
        let epoch = list.epoch;
        let insert = RowAction::InsertLineAfter {
            id,
            after: 1,
            expect: "  BaseType \"A\"".into(),
            text: "ItemLevel <= 80".into(),
        };
        assert!(apply_row_actions(&mut working, &mut list, &mut String::new(), vec![insert]).0);
        assert_ne!(list.epoch, epoch, "an inserted line can shift ordinals");
        assert_eq!(list.focus_line.map(|(_, l)| l), Some(2));
    }

    // ---- popups (Phase 30-03) ----

    /// An app on temp-free in-memory state: built only to drive `dispatch_popup`, which touches
    /// nothing on disk for the choices tested here.
    fn popup_app(filter_text: &str) -> (super::OverlayApp, egui::Context) {
        use crate::overlay::hotkey::HotkeyStatus;
        use crate::overlay::tray::TrayHandles;
        let ctx = egui::Context::default();
        crate::skin::fonts::install(&ctx);
        let skin = crate::skin::install(&ctx);
        let home = tempfile::tempdir().unwrap();
        let mut app = super::OverlayApp::build(
            TrayHandles::inert(),
            None,
            HotkeyStatus::Failed("test".to_string()),
            None,
            skin,
            Ok(home.path().to_path_buf()),
            Some(home.path().join("game")),
        );
        app.test_home = Some(home);
        app.working = Some(WorkingFilter::from_text(filter_text));
        (app, ctx)
    }

    #[test]
    fn fallback_price_source_stops_a_bad_tier_from_gating_apply() {
        let (mut app, _ctx) = popup_app(RULES);
        assert!(app.config.is_ok(), "{:?}", app.config.as_ref().err());
        app.config.as_mut().unwrap().set_tier_text("A", "abc");

        app.sync_tiers_in_use();
        assert!(app.config.as_ref().unwrap().validate().is_err());

        app.settings.price_source = Some("fallback".to_string());
        app.sync_tiers_in_use();
        assert!(app.config.as_ref().unwrap().validate().is_ok());

        app.settings.price_source = Some("live".to_string());
        app.sync_tiers_in_use();
        assert!(app.config.as_ref().unwrap().validate().is_err());
    }

    #[test]
    fn the_preview_key_includes_the_price_policy() {
        let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        // A root with live prices: the frozen seed.
        let dir = tempfile::tempdir().unwrap();
        for (src, dst) in [
            ("tests/fixtures/seed-styles.json", "styles.json"),
            ("tiers-default.json", "tiers.json"),
            ("tests/fixtures/seed-prices.json", "prices.json"),
        ] {
            std::fs::copy(root.join(src), dir.path().join(dst)).unwrap();
        }
        let live = super::staged_economy_body(dir.path(), None, PriceSourcePolicy::Auto).unwrap();
        let fb =
            super::staged_economy_body(dir.path(), None, PriceSourcePolicy::ForceFallback).unwrap();
        assert_ne!(live, fb);

        // refresh_staged_economy re-prices when only the policy changed.
        let (mut app, _ctx) = popup_app(RULES);
        app.root = Ok(dir.path().to_path_buf());
        app.config = crate::overlay::config_state::ConfigState::load(dir.path());
        app.working.as_mut().unwrap().stage_add_economy(&live);
        app.refresh_staged_economy();
        let key = app.staged_econ_inputs.clone().unwrap();
        assert_eq!(key.2, PriceSourcePolicy::Auto);
        app.settings.price_source = Some("fallback".to_string());
        app.refresh_staged_economy();
        assert_eq!(
            app.staged_econ_inputs.clone().unwrap().2,
            PriceSourcePolicy::ForceFallback
        );
    }

    #[test]
    #[cfg(debug_assertions)]
    fn expand_seeds_the_nth_row() {
        let (app, _ctx) = popup_app(RULES);
        let rows = app.working.as_ref().unwrap().rows();
        assert_eq!(super::nth_row_id(rows, 1), Some(rows[0].id));
        assert_eq!(super::nth_row_id(rows, 2), Some(rows[1].id));
        assert_eq!(super::nth_row_id(rows, 0), None);
        assert_eq!(super::nth_row_id(rows, rows.len() + 1), None);
    }

    #[test]
    #[cfg(debug_assertions)]
    fn rename_section_picks_the_nth_user_section_and_skips_economy() {
        let sec = |ordinal: usize, name: &str, economy: bool| crate::overlay::rules::SectionRow {
            ordinal,
            name: name.to_string(),
            economy,
            rules: 1,
            can_move_up: false,
            can_move_down: false,
            can_remove: !economy,
        };
        let all = [sec(0, "A", false), sec(1, "B", false), sec(2, "Eco", true)];
        assert_eq!(super::nth_user_section(&all, 1).unwrap().name, "A");
        assert_eq!(super::nth_user_section(&all, 2).unwrap().ordinal, 1);
        assert!(super::nth_user_section(&all, 3).is_none());
        assert!(super::nth_user_section(&all, 0).is_none());
    }

    #[test]
    fn delete_popup_choice_deletes_the_rule_and_keep_does_not() {
        use crate::overlay::shell::Popup;
        let (mut app, ctx) = popup_app(RULES);
        let id = app.working.as_ref().unwrap().rows()[0].id;
        app.list.confirm_delete = Some(id);
        assert_eq!(
            crate::overlay::shell::active_popup(app.popup_flags(false)),
            Some(Popup::DeleteRule)
        );
        app.dispatch_popup(&ctx, Popup::DeleteRule, 1);
        assert_eq!(app.working.as_ref().unwrap().rows().len(), 2);
        assert!(app.list.confirm_delete.is_none());

        app.list.confirm_delete = Some(id);
        app.dispatch_popup(&ctx, Popup::DeleteRule, 0);
        assert_eq!(app.working.as_ref().unwrap().rows().len(), 1);
        assert!(app.list.confirm_delete.is_none());
        assert!(app.working.as_ref().unwrap().is_dirty());
    }

    /// CR-02: a failed Apply while the state is still `Applied` (from an earlier success) is
    /// shown as an Error, never in the success colour.
    #[test]
    fn apply_failure_after_a_success_is_an_error() {
        use crate::overlay::apply::ApplyState;
        use crate::skin::widgets::StatusKind;
        let (mut app, ctx) = popup_app(RULES);
        let Rig {
            root,
            _game,
            path,
            loaded,
            working,
            config,
        } = rig(S_RULE);
        app.root = Ok(root.path().to_path_buf());
        app.config = Ok(config);
        app.filter = Ok(loaded);
        app.working = Some(working);
        app.apply_state = ApplyState::Applied;
        // Someone else changes the filter: the Apply is refused.
        std::fs::write(&path, "Show\n").unwrap();
        assert!(!app.run_apply(&ctx, false));
        assert_eq!(
            app.apply_state,
            ApplyState::Applied,
            "a failure keeps the state"
        );
        let (text, kind) = app.apply_message.clone().unwrap();
        assert_eq!(kind, StatusKind::Error, "{text}");
        let event =
            crate::overlay::shell::event_from(None, app.apply_message.clone(), None, None, None);
        assert_eq!(event.map(|(_, k)| k), Some(StatusKind::Error));
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "Show\n");
    }

    /// WR-02 (iteration 2): tier errors keep Overwrite off during a styles.json refusal, and the
    /// popup says how to get it back.
    #[test]
    fn tier_errors_block_overwrite_and_the_popup_says_why() {
        use crate::overlay::shell::{OverwriteBlock, Popup};
        let (mut app, _ctx) = popup_app(RULES);
        let Rig {
            root,
            _game,
            path: _,
            loaded,
            working,
            config,
        } = rig(S_RULE);
        app.root = Ok(root.path().to_path_buf());
        app.config = Ok(config);
        app.filter = Ok(loaded);
        app.working = Some(working);
        app.changed_on_disk = vec![DiskFile::Styles];
        assert!(app.overwrite_decision(), "a valid config is overwritable");

        app.config
            .as_mut()
            .unwrap()
            .set_tier_text("S", "not a number");
        assert!(app.config.as_ref().unwrap().has_tier_errors());
        assert!(!app.overwrite_decision());
        assert_eq!(app.overwrite_block(), OverwriteBlock::TierErrors);
        app.overwrite_live = app.overwrite_decision();
        let spec = app.popup_spec_for(Popup::ChangedOnDisk);
        assert!(
            spec.body
                .contains("Fix the tier errors to overwrite instead."),
            "{}",
            spec.body
        );
        assert!(spec.buttons.iter().all(|b| b.0 != "Overwrite"));
    }

    /// 32 UI review: a fresh list event (`Section added…`) wins over the standing out-of-date
    /// indicator until the next real edit; committing the new section's name box unchanged is
    /// not an edit.
    #[test]
    fn a_section_added_message_beats_the_out_of_date_indicator() {
        use crate::overlay::shell::{self, StatusInputs};
        let (mut app, _ctx) = popup_app(RULES);
        app.run_row_actions(vec![RowAction::AddSection]);
        assert_eq!(
            app.apply_state,
            crate::overlay::apply::ApplyState::Unapplied
        );
        let added = "Section added at the top. Press Apply to save it.";
        assert_eq!(app.list_status.as_deref(), Some(added));
        // The name box commits its default name on Enter or a click away.
        app.run_row_actions(vec![RowAction::RenameSection {
            ordinal: 0,
            name: "Section".into(),
        }]);
        assert_eq!(app.list_status.as_deref(), Some(added), "not an edit");
        let status = shell::status_for(StatusInputs {
            root_error: None,
            event: shell::event_from(
                None,
                app.apply_message.clone(),
                app.config_message.clone(),
                None,
                app.list_status.clone(),
            ),
            grammar_gate: None,
            filter_error: None,
            game_folder_missing: None,
            syncing: false,
            ended_league: None,
            unknown: None,
            migrated: false,
            absent: false,
            indicator: (
                crate::overlay::apply::indicator(app.apply_state),
                shell::indicator_kind(app.apply_state),
            ),
        });
        assert_eq!(status.text, added);
        // A real rename is the next edit: the event gives way to the indicator.
        app.run_row_actions(vec![RowAction::RenameSection {
            ordinal: 0,
            name: "Gear".into(),
        }]);
        assert_eq!(app.list_status, None);
    }

    /// WR-01: hiding clears the CONFIG message and the list status too, so neither hides the
    /// grammar gate, the load error or the out-of-date indicator indefinitely; a rule edit clears
    /// the CONFIG message.
    #[test]
    fn conceal_and_rule_edits_clear_stale_event_messages() {
        let (mut app, _ctx) = popup_app(RULES);
        app.config_message = Some(("Reverted config changes.".to_string(), false));
        app.list_status = Some("Class groups unavailable (config.json): x".to_string());
        app.clear_events_on_conceal();
        assert_eq!(app.config_message, None);
        assert_eq!(app.list_status, None);

        app.config_message = Some(("Reverted config changes.".to_string(), false));
        app.run_row_actions(vec![RowAction::AddRule]);
        assert_eq!(app.config_message, None);
    }

    fn settle_update(app: &mut super::OverlayApp) {
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
        while std::time::Instant::now() < deadline {
            if let Some((text, kind)) = app.update.poll(0) {
                app.update_message = Some((text, super::update_kind(kind)));
            }
            if matches!(app.update.state(), super::UpdateState::Idle) {
                return;
            }
            std::thread::sleep(std::time::Duration::from_millis(1));
        }
        panic!("update check did not settle");
    }

    #[test]
    fn pressing_check_reports_checking_then_the_outcome() {
        use crate::skin::widgets::StatusKind;
        let (mut app, _ctx) = popup_app(RULES);
        app.start_update_check();
        assert_eq!(
            app.update_message,
            Some(("Checking for updates…".to_string(), StatusKind::Info))
        );
        settle_update(&mut app);
        assert_eq!(
            app.update_message,
            Some((
                "Could not reach GitHub. Check your connection and try again.".to_string(),
                StatusKind::Warning
            ))
        );
        assert_eq!(app.update.state(), &super::UpdateState::Idle);
    }

    /// An app in the Ready state holding a real installer in a `FerrisFilter-update-*` folder
    /// under a temp dir (never the real %TEMP%). Nothing is ever spawned.
    fn ready_app() -> (super::OverlayApp, tempfile::TempDir, std::path::PathBuf) {
        let (mut app, _ctx) = popup_app(RULES);
        let root = tempfile::tempdir().unwrap();
        let dir = root.path().join("FerrisFilter-update-test");
        std::fs::create_dir_all(&dir).unwrap();
        let installer = dir.join("FerrisFilter-0.8.0-setup.exe");
        std::fs::write(&installer, b"installer bytes").unwrap();
        let sha256 = crate::update::sha256_file(&installer).unwrap();
        app.update
            .inject(super::UpdateState::Ready(crate::update::ReadyUpdate {
                version: "0.8.0".to_string(),
                published_unix: None,
                installer: installer.clone(),
                sha256,
            }));
        (app, root, installer)
    }

    fn handed_off(app: &super::OverlayApp) -> Option<std::path::PathBuf> {
        app.pending_install.lock().unwrap().clone()
    }

    #[test]
    fn install_click_with_nothing_unapplied_quits_and_hands_off_the_installer() {
        let (mut app, _root, installer) = ready_app();
        assert!(app.install_and_quit());
        assert_eq!(handed_off(&app), Some(installer.clone()));
        assert!(installer.exists(), "the wizard runs from this file");
    }

    #[test]
    fn install_click_with_unapplied_edits_asks_and_cancel_clears_the_intent() {
        use crate::overlay::shell::Popup;
        let (mut app, _root, installer) = ready_app();
        let ctx = egui::Context::default();
        app.config.as_mut().unwrap().set_tier_text("S", "31");
        assert!(!app.install_and_quit());
        assert!(app.quit_confirm);
        assert!(app.update.install_intended());
        assert_eq!(handed_off(&app), None);

        app.dispatch_popup(&ctx, Popup::Quit, 2);
        assert!(!app.update.install_intended());
        assert!(matches!(app.update.state(), super::UpdateState::Ready(_)));
        assert_eq!(handed_off(&app), None);

        // A later plain quit never launches it.
        app.quit_confirm = false;
        app.begin_shutdown();
        assert_eq!(handed_off(&app), None);
        assert!(!installer.exists());
    }

    #[test]
    fn install_click_then_refused_apply_clears_the_intent() {
        use crate::overlay::shell::Popup;
        let (mut app, _root, _installer) = ready_app();
        let ctx = egui::Context::default();
        // A cutoff that is not a number refuses the Apply.
        app.config
            .as_mut()
            .unwrap()
            .set_tier_text("S", "not a number");
        assert!(!app.install_and_quit());
        assert!(app.quit_confirm && app.update.install_intended());
        app.dispatch_popup(&ctx, Popup::Quit, 0);
        assert!(!app.quit_after_apply, "the refused Apply keeps the window");
        assert!(!app.update.install_intended());
        assert_eq!(handed_off(&app), None);
    }

    #[test]
    fn install_click_then_quit_without_applying_hands_off() {
        use crate::overlay::shell::Popup;
        let (mut app, _root, installer) = ready_app();
        let ctx = egui::Context::default();
        app.config.as_mut().unwrap().set_tier_text("S", "31");
        assert!(!app.install_and_quit());
        app.dispatch_popup(&ctx, Popup::Quit, 1);
        assert!(app.quit_next_frame);
        app.begin_shutdown();
        assert_eq!(handed_off(&app), Some(installer));
    }

    #[test]
    fn plain_quit_while_ready_deletes_the_installer_and_launches_nothing() {
        let (mut app, _root, installer) = ready_app();
        assert!(app.request_quit());
        assert_eq!(handed_off(&app), None);
        assert!(!installer.exists());
        assert!(!installer.parent().unwrap().exists());
    }

    #[test]
    fn install_click_with_the_installer_gone_reports_it_and_stays_open() {
        use crate::skin::widgets::StatusKind;
        let (mut app, _root, installer) = ready_app();
        std::fs::remove_file(&installer).unwrap();
        assert!(!app.install_and_quit());
        assert_eq!(
            app.update_message,
            Some((
                "The installer for 0.8.0 is no longer there. Check for updates again.".to_string(),
                StatusKind::Error
            ))
        );
        assert_eq!(app.update.state(), &super::UpdateState::Idle);
        assert_eq!(handed_off(&app), None);
    }

    #[test]
    fn conceal_keeps_ready_and_clears_the_update_message() {
        use crate::skin::widgets::StatusKind;
        let (mut app, _ctx) = popup_app(RULES);
        let ready = crate::update::ReadyUpdate {
            version: "0.8.0".to_string(),
            published_unix: None,
            installer: std::path::PathBuf::from("x.exe"),
            sha256: String::new(),
        };
        app.update.inject(super::UpdateState::Ready(ready));
        app.update_message = Some(("msg".to_string(), StatusKind::Ok));
        app.clear_events_on_conceal();
        assert_eq!(app.update_message, None);
        assert!(matches!(app.update.state(), super::UpdateState::Ready(_)));

        app.update_message = Some(("msg".to_string(), StatusKind::Ok));
        app.clear_events_on_edit();
        assert_eq!(app.update_message, None);
        app.tab = ShellTab::Rules;
        assert!(matches!(app.update.state(), super::UpdateState::Ready(_)));
    }

    #[test]
    fn starting_an_apply_clears_the_update_message() {
        use crate::skin::widgets::StatusKind;
        let (mut app, ctx) = popup_app(RULES);
        app.update_message = Some(("msg".to_string(), StatusKind::Ok));
        app.run_apply(&ctx, false);
        assert_eq!(app.update_message, None);
    }

    /// WR-01 (iteration 2): a refused Apply's Error line gives way to each tab's revert message,
    /// economy staging clears a stale success, and a CONFIG edit clears the list status.
    #[test]
    fn reverts_staging_and_config_edits_clear_stale_events() {
        use crate::overlay::config_view::ConfigAction;
        use crate::skin::widgets::StatusKind;
        let (mut app, ctx) = popup_app(RULES);
        let refused = || {
            Some((
                "Apply refused: 1 invalid block".to_string(),
                StatusKind::Error,
            ))
        };

        // RULES Revert.
        app.apply_message = refused();
        app.revert();
        assert_eq!(app.apply_message, None);
        assert_eq!(
            app.list_status.as_deref(),
            Some(crate::overlay::apply::REVERTED_MESSAGE)
        );

        // CONFIG Revert (the config may be absent in this rig; the clear still runs).
        app.apply_message = refused();
        app.handle_config_actions(&ctx, vec![ConfigAction::Revert]);
        assert_eq!(app.apply_message, None);
        assert_eq!(app.list_status, None, "the RULES revert line is stale too");

        // Economy staging hides neither the out-of-date indicator nor its own state.
        app.apply_message = Some(("Wrote x.".to_string(), StatusKind::Ok));
        app.list_status = Some(crate::overlay::apply::REVERTED_MESSAGE.to_string());
        app.stage_remove_economy();
        assert_eq!(app.apply_message, None);
        assert_eq!(app.list_status, None);

        // A CONFIG edit clears a RULES-side list status.
        app.list_status = Some(crate::overlay::apply::REVERTED_MESSAGE.to_string());
        app.handle_config_actions(
            &ctx,
            vec![ConfigAction::TierText {
                tier: "S".to_string(),
                text: "1".to_string(),
            }],
        );
        assert_eq!(app.list_status, None);
    }

    #[test]
    fn style_editor_edits_stay_in_memory_until_apply_and_revert_restores() {
        use crate::models::config::{Beam, Cancelable, FilterColor, Minimap, MinimapShape, Sound};
        use crate::overlay::config_state::{ColourSlot, ConfigState, StyleEdit};
        use crate::overlay::config_view::ConfigAction;
        let (mut app, ctx) = popup_app(RULES);
        let dir = tempfile::tempdir().unwrap();
        for (src, dst) in [
            ("tests/fixtures/seed-styles.json", "styles.json"),
            ("tiers-default.json", "tiers.json"),
        ] {
            std::fs::copy(
                std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(src),
                dir.path().join(dst),
            )
            .unwrap();
        }
        let styles_path = dir.path().join("styles.json");
        let before_bytes = std::fs::read(&styles_path).unwrap();
        app.config = ConfigState::load(dir.path());
        let original = app.config.as_ref().unwrap().style("S").unwrap().clone();
        assert!(!app.config.as_ref().unwrap().is_dirty());

        let edit = |edit: StyleEdit| ConfigAction::EditStyle {
            name: "S".to_string(),
            edit,
        };
        app.handle_config_actions(
            &ctx,
            vec![
                edit(StyleEdit::Colour {
                    slot: ColourSlot::Text,
                    value: Some([1, 2, 3]),
                }),
                edit(StyleEdit::Font(Some(20))),
                edit(StyleEdit::Minimap(Some(Cancelable::Set(Minimap {
                    size: 0,
                    color: FilterColor::Cyan,
                    shape: MinimapShape::Star,
                })))),
                edit(StyleEdit::Beam(Some(Cancelable::Set(Beam {
                    color: FilterColor::Red,
                    temp: true,
                })))),
                edit(StyleEdit::Sound(Some(Cancelable::Set(Sound {
                    id: Some(4),
                    custom: None,
                    vol: 50,
                    positional: false,
                })))),
            ],
        );
        let state = app.config.as_ref().unwrap();
        assert!(state.is_dirty(), "edits make CONFIG unapplied");
        let edited = state.style("S").unwrap();
        assert_eq!(edited.text, Some([1, 2, 3]));
        assert_eq!(edited.font, Some(20));
        assert_eq!(
            std::fs::read(&styles_path).unwrap(),
            before_bytes,
            "nothing is written before Apply"
        );

        let epoch = app.config_ui.epoch;
        app.handle_config_actions(&ctx, vec![ConfigAction::Revert]);
        let state = app.config.as_ref().unwrap();
        assert!(!state.is_dirty());
        assert_eq!(state.style("S").unwrap(), &original);
        assert_ne!(app.config_ui.epoch, epoch, "Revert orphans open pickers");
        assert_eq!(std::fs::read(&styles_path).unwrap(), before_bytes);
    }

    /// WR-01: + Add style opens the new style's editor, even when the row was already marked
    /// expanded (the handler used a toggle, which collapsed it again).
    #[test]
    fn adding_a_style_expands_it() {
        use crate::overlay::config_state::ConfigState;
        use crate::overlay::config_view::ConfigAction;
        let (mut app, ctx) = popup_app(RULES);
        let dir = tempfile::tempdir().unwrap();
        for (src, dst) in [
            ("tests/fixtures/seed-styles.json", "styles.json"),
            ("tiers-default.json", "tiers.json"),
        ] {
            std::fs::copy(
                std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(src),
                dir.path().join(dst),
            )
            .unwrap();
        }
        app.config = ConfigState::load(dir.path());
        for (name, pre) in [("glow", None), ("shine", Some("shine"))] {
            app.config_ui.expanded = pre.map(str::to_string);
            app.handle_config_actions(&ctx, vec![ConfigAction::AddStyle(name.to_string())]);
            assert!(app.config.as_ref().unwrap().style(name).is_some());
            assert_eq!(app.config_ui.expanded.as_deref(), Some(name));
            assert_eq!(app.config_ui.scroll_to.as_deref(), Some(name));
        }
    }

    #[test]
    fn strictness_click_is_a_pending_style_edit_that_revert_undoes() {
        use crate::models::config::StyleAction;
        use crate::overlay::config_view::ConfigAction;
        use crate::overlay::shell::Popup;
        use crate::overlay::strictness::Strictness;
        let (mut app, ctx) = popup_app(RULES);
        let _dir = config_with_defaults(&mut app);
        assert_eq!(
            app.config.as_ref().unwrap().strictness(),
            Strictness::FlatBroke
        );
        app.handle_config_actions(&ctx, vec![ConfigAction::SetStrictness(Strictness::Loaded)]);
        let c = app.config.as_ref().unwrap();
        assert_eq!(c.strictness(), Strictness::Loaded);
        assert!(c.styles_dirty() && c.is_dirty());
        for n in ["C", "D", "E", "F"] {
            assert_eq!(c.style(n).unwrap().action, StyleAction::Hide, "{n}");
        }
        app.config_ui.revert_scope = crate::overlay::config_state::RevertScope::Styles;
        app.config_ui.revert_armed = true;
        app.dispatch_popup(&ctx, Popup::DiscardConfig, 0);
        let c = app.config.as_ref().unwrap();
        assert_eq!(c.strictness(), Strictness::FlatBroke);
        assert!(!c.styles_dirty());
    }

    #[test]
    fn strictness_apply_writes_styles_json_into_the_temp_home_only() {
        use crate::overlay::config_view::ConfigAction;
        use crate::overlay::strictness::Strictness;
        let (mut app, ctx) = popup_app(RULES);
        let dir = config_with_defaults(&mut app);
        app.root = Ok(dir.path().to_path_buf());
        // No filter loaded: Apply writes the config files only.
        app.filter = Err("no filter".to_string());
        app.working = None;
        let defaults = std::fs::read_to_string(dir.path().join("styles.json")).unwrap();
        app.handle_config_actions(
            &ctx,
            vec![ConfigAction::SetStrictness(Strictness::Scrounging)],
        );
        app.handle_config_actions(&ctx, vec![ConfigAction::Apply]);

        let written = std::fs::read_to_string(dir.path().join("styles.json")).unwrap();
        let styles: serde_json::Value = serde_json::from_str(&written).unwrap();
        assert_eq!(styles["F"]["action"], "Hide");
        assert_ne!(written, defaults);
        let before: serde_json::Value = serde_json::from_str(&defaults).unwrap();
        for k in ["unchanged", "hidden"] {
            assert_eq!(styles[k], before[k], "{k}");
        }
        assert!(!app.config.as_ref().unwrap().is_dirty());
    }

    #[test]
    fn strictness_of_the_current_level_changes_nothing() {
        use crate::overlay::config_view::ConfigAction;
        use crate::overlay::strictness::Strictness;
        let (mut app, ctx) = popup_app(RULES);
        let _dir = config_with_defaults(&mut app);
        app.config_message = Some(("kept".to_string(), true));
        app.handle_config_actions(
            &ctx,
            vec![ConfigAction::SetStrictness(Strictness::FlatBroke)],
        );
        assert!(!app.config.as_ref().unwrap().is_dirty());
        assert!(app.config_message.is_some());
    }

    fn config_with_defaults(app: &mut super::OverlayApp) -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        for (src, dst) in [
            ("tests/fixtures/seed-styles.json", "styles.json"),
            ("tiers-default.json", "tiers.json"),
        ] {
            std::fs::copy(
                std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(src),
                dir.path().join(dst),
            )
            .unwrap();
        }
        app.config = crate::overlay::config_state::ConfigState::load(dir.path());
        dir
    }

    #[test]
    fn economy_revert_discards_only_tier_edits() {
        use crate::overlay::config_state::StyleEdit;
        use crate::overlay::config_view::ConfigAction;
        use crate::overlay::shell::Popup;
        let (mut app, ctx) = popup_app(RULES);
        let _dir = config_with_defaults(&mut app);
        app.handle_config_actions(
            &ctx,
            vec![
                ConfigAction::TierText {
                    tier: "S".to_string(),
                    text: "31".to_string(),
                },
                ConfigAction::EditStyle {
                    name: "S".to_string(),
                    edit: StyleEdit::Font(Some(20)),
                },
            ],
        );
        // What the ECONOMY bar Revert click does.
        app.config_ui.revert_scope = crate::overlay::config_state::RevertScope::Tiers;
        app.config_ui.revert_armed = true;
        let spec = app.popup_spec_for(Popup::DiscardConfig);
        assert!(!spec.buttons.is_empty());
        app.dispatch_popup(&ctx, Popup::DiscardConfig, 0);
        let c = app.config.as_ref().unwrap();
        assert!(!c.tiers_dirty(), "tier edit reverted");
        assert!(c.styles_dirty(), "style edit kept");
    }

    #[test]
    fn config_revert_discards_only_style_edits() {
        use crate::overlay::config_state::StyleEdit;
        use crate::overlay::config_view::ConfigAction;
        use crate::overlay::shell::Popup;
        let (mut app, ctx) = popup_app(RULES);
        let _dir = config_with_defaults(&mut app);
        app.handle_config_actions(
            &ctx,
            vec![
                ConfigAction::TierText {
                    tier: "S".to_string(),
                    text: "31".to_string(),
                },
                ConfigAction::EditStyle {
                    name: "S".to_string(),
                    edit: StyleEdit::Font(Some(20)),
                },
            ],
        );
        app.config_ui.revert_scope = crate::overlay::config_state::RevertScope::Styles;
        app.config_ui.revert_armed = true;
        app.dispatch_popup(&ctx, Popup::DiscardConfig, 0);
        let c = app.config.as_ref().unwrap();
        assert!(!c.styles_dirty(), "style edit reverted");
        assert!(c.tiers_dirty(), "tier edit kept");
    }

    #[test]
    fn economy_add_click_stages_in_memory_and_writes_nothing() {
        use crate::overlay::rules::EconomyView;
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("t.filter");
        std::fs::write(&path, RULES).unwrap();
        let (mut app, _ctx) = popup_app(RULES);
        // The repo root is read-only input here (styles, tiers, fallback prices).
        app.root = Ok(std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")));
        assert_eq!(
            *app.working.as_ref().unwrap().economy(),
            EconomyView::Absent
        );
        app.stage_add_economy();
        let w = app.working.as_ref().unwrap();
        assert!(matches!(w.economy(), EconomyView::Present { .. }));
        assert!(w.economy_staged());
        assert_eq!(std::fs::read_to_string(&path).unwrap(), RULES);
        // The staged section is browsable before Apply: named rows, header count to match.
        assert!(!w.economy_rows().is_empty());
        assert!(
            w.economy_rows()[0]
                .name
                .as_deref()
                .unwrap()
                .contains(" tier ")
        );
        let sec = w.sections().iter().find(|s| s.economy).unwrap();
        assert_eq!(sec.rules, w.economy_rows().len());
    }

    #[test]
    fn staged_economy_preview_follows_pending_tier_edits() {
        let (mut app, _ctx) = popup_app(RULES);
        // Live-priced economy needs a prices.json: a temp root holding the frozen seed prices.
        let priced_root = tempfile::tempdir().unwrap();
        std::fs::copy(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("tests/fixtures/seed-prices.json"),
            priced_root.path().join("prices.json"),
        )
        .unwrap();
        app.root = Ok(priced_root.path().to_path_buf());
        let Ok(_) = &app.config else {
            panic!("rig has no CONFIG state");
        };
        app.stage_add_economy();
        let before = app.working.as_ref().unwrap().economy_rows().len();
        // A huge S cutoff empties tier S: the preview is re-priced from the pending tiers.
        app.config.as_mut().unwrap().set_tier_text("S", "100000000");
        app.refresh_staged_economy();
        let after = app.working.as_ref().unwrap().economy_rows().len();
        assert_ne!(before, after);
        // An expanded preview row is collapsed by a re-price (its id would point elsewhere).
        let id = app.working.as_ref().unwrap().economy_rows()[0].id;
        app.list.expanded = Some(id);
        app.config.as_mut().unwrap().set_tier_text("S", "90000000");
        app.refresh_staged_economy();
        assert_eq!(app.list.expanded, None);
        assert_eq!(
            app.working
                .as_ref()
                .unwrap()
                .sections()
                .iter()
                .find(|s| s.economy)
                .unwrap()
                .rules,
            after
        );
    }

    /// An economy section already in the file follows the pending styles before Apply: a style
    /// turned to Hide shows its economy rows as Hide, and dropping the edit shows the file again.
    #[test]
    fn economy_in_the_file_follows_pending_style_visibility() {
        use crate::models::config::StyleAction;
        use crate::overlay::config_state::StyleEdit;
        let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let body = super::staged_economy_body(&root, None, PriceSourcePolicy::Auto).unwrap();
        let text = crate::filter::section::add(RULES, &body).unwrap();
        let (mut app, _ctx) = popup_app(&text);
        app.root = Ok(root);
        let s_row = |app: &super::OverlayApp| {
            app.working
                .as_ref()
                .unwrap()
                .economy_rows()
                .iter()
                .find(|r| r.name.as_deref() == Some("S tier Currency"))
                .map(|r| (r.keyword, r.label.clone()))
                .unwrap()
        };
        app.refresh_staged_economy();
        assert!(!app.working.as_ref().unwrap().has_economy_preview());
        assert_eq!(s_row(&app).0, crate::filter::BlockKeyword::Show);

        let config = app.config.as_mut().unwrap();
        assert!(config.edit_style("S", StyleEdit::Action(StyleAction::Hide)));
        super::sync_working_styles(app.config.as_ref().unwrap(), app.working.as_mut().unwrap());
        app.refresh_staged_economy();
        assert!(app.working.as_ref().unwrap().has_economy_preview());
        let (kw, label) = s_row(&app);
        assert_eq!(kw, crate::filter::BlockKeyword::Hide);
        assert!(label.starts_with("Hide"), "{label}");
        let w = app.working.as_ref().unwrap();
        let sec = w.sections().iter().find(|s| s.economy).unwrap();
        assert_eq!(sec.rules, w.economy_rows().len());
        // Nothing is written by the preview.
        assert!(!w.is_dirty());

        app.config.as_mut().unwrap().revert();
        super::sync_working_styles(app.config.as_ref().unwrap(), app.working.as_mut().unwrap());
        app.refresh_staged_economy();
        assert!(!app.working.as_ref().unwrap().has_economy_preview());
        assert_eq!(s_row(&app).0, crate::filter::BlockKeyword::Show);
    }

    #[test]
    fn staged_economy_preview_is_re_priced_after_a_sync() {
        let (mut app, _ctx) = popup_app(RULES);
        let repo = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let dir = tempfile::tempdir().unwrap();
        for f in [
            "styles-default.json",
            "tiers-default.json",
            "prices-fallback.json",
            "tiers-fallback.json",
        ] {
            std::fs::copy(repo.join(f), dir.path().join(f)).unwrap();
        }
        app.root = Ok(dir.path().to_path_buf());
        app.stage_add_economy();
        app.refresh_staged_economy();
        let before = app.working.as_ref().unwrap().economy_rows().len();
        // A sync lands live prices (the seed fixture) where only the fallback pair existed.
        std::fs::copy(
            repo.join("tests/fixtures/seed-prices.json"),
            dir.path().join("prices.json"),
        )
        .unwrap();
        app.prices_changed();
        app.refresh_staged_economy();
        let after = app.working.as_ref().unwrap().economy_rows().len();
        assert_ne!(before, after, "the preview must follow the new prices");
    }

    #[test]
    fn reset_popups_apply_one_pending_edit_and_keep_does_nothing() {
        use crate::overlay::shell::Popup;
        let (mut app, ctx) = popup_app(RULES);
        let dir = config_root();
        app.config = ConfigState::load(dir.path());
        assert!(app.config.as_mut().unwrap().add_style("glow").is_ok());
        app.config
            .as_mut()
            .unwrap()
            .edit_style("S", StyleEdit::Font(Some(41)));
        app.config.as_mut().unwrap().set_tier_text("S", "999");
        app.config_ui.expanded = Some("S".to_string());
        let before = app.config.as_ref().unwrap().pending_styles().clone();

        // Keep: nothing changes, the arm clears.
        app.config_ui.reset_armed = Some(RevertScope::Styles);
        assert_eq!(
            shell::active_popup(app.popup_flags(false)),
            Some(Popup::ResetStyles)
        );
        app.dispatch_popup(&ctx, Popup::ResetStyles, 1);
        assert_eq!(app.config_ui.reset_armed, None);
        assert_eq!(*app.config.as_ref().unwrap().pending_styles(), before);

        // Reset: built-ins back, custom kept, row collapsed, still pending.
        app.config_ui.reset_armed = Some(RevertScope::Styles);
        app.dispatch_popup(&ctx, Popup::ResetStyles, 0);
        let state = app.config.as_ref().unwrap();
        assert!(state.styles_at_defaults());
        assert!(state.style("glow").is_some());
        assert_eq!(app.config_ui.expanded, None);
        assert_eq!(app.config_ui.reset_armed, None);
        assert!(state.is_dirty());

        // Tiers likewise.
        assert!(!app.config.as_ref().unwrap().tiers_at_defaults());
        app.config_ui.reset_armed = Some(RevertScope::Tiers);
        assert_eq!(
            shell::active_popup(app.popup_flags(false)),
            Some(Popup::ResetTiers)
        );
        app.dispatch_popup(&ctx, Popup::ResetTiers, 1);
        assert!(!app.config.as_ref().unwrap().tiers_at_defaults());
        app.config_ui.reset_armed = Some(RevertScope::Tiers);
        app.dispatch_popup(&ctx, Popup::ResetTiers, 0);
        assert!(app.config.as_ref().unwrap().tiers_at_defaults());
        assert_eq!(app.config_ui.reset_armed, None);
    }

    #[test]
    fn economy_remove_click_asks_and_the_popup_stages_the_removal() {
        use crate::overlay::rules::EconomyView;
        use crate::overlay::shell::Popup;
        let (mut app, ctx) = popup_app(RULES);
        app.root = Ok(std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")));
        app.stage_add_economy();
        // Keep: nothing changes.
        app.list.confirm_economy_remove = true;
        app.dispatch_popup(&ctx, Popup::RemoveEconomy, 1);
        assert!(matches!(
            app.working.as_ref().unwrap().economy(),
            EconomyView::Present { .. }
        ));
        // Remove: staged (the add is undone, so the section is absent again).
        app.list.confirm_economy_remove = true;
        app.dispatch_popup(&ctx, Popup::RemoveEconomy, 0);
        assert_eq!(
            *app.working.as_ref().unwrap().economy(),
            EconomyView::Absent
        );
    }

    #[test]
    fn economy_revert_with_tier_edits_keeps_the_staged_op_until_confirmed() {
        use crate::overlay::config_view::ConfigAction;
        use crate::overlay::shell::Popup;
        let (mut app, ctx) = popup_app(RULES);
        let _dir = config_with_defaults(&mut app);
        app.handle_config_actions(
            &ctx,
            vec![ConfigAction::TierText {
                tier: "S".to_string(),
                text: "31".to_string(),
            }],
        );
        app.working
            .as_mut()
            .unwrap()
            .stage_add_economy(&crate::overlay::rules::tests_econ_body(3));
        // Revert opens the popup and drops nothing yet.
        app.economy_revert_pressed();
        assert!(app.config_ui.revert_armed);
        assert!(app.working.as_ref().unwrap().economy_staged());
        // Keep editing: both edits survive.
        app.dispatch_popup(&ctx, Popup::DiscardConfig, 1);
        assert!(app.working.as_ref().unwrap().economy_staged());
        assert!(app.config.as_ref().unwrap().tiers_dirty());
        // Discard: tiers and the staged op go together.
        app.economy_revert_pressed();
        app.dispatch_popup(&ctx, Popup::DiscardConfig, 0);
        assert!(!app.working.as_ref().unwrap().economy_staged());
        assert!(!app.config.as_ref().unwrap().tiers_dirty());
    }

    #[test]
    fn economy_revert_without_tier_edits_asks_before_dropping_the_staged_op() {
        use crate::overlay::shell::{self, Popup};
        let (mut app, ctx) = popup_app(RULES);
        app.working
            .as_mut()
            .unwrap()
            .stage_add_economy(&crate::overlay::rules::tests_econ_body(3));
        app.economy_revert_pressed();
        assert!(app.config_ui.revert_armed, "the popup is armed");
        assert!(
            app.working.as_ref().unwrap().economy_staged(),
            "nothing dropped yet"
        );
        assert_eq!(
            shell::active_popup(app.popup_flags(false)),
            Some(Popup::DiscardConfig)
        );
        let spec = app.popup_spec_for(Popup::DiscardConfig);
        assert_eq!(spec.title, "Discard economy changes?");
        assert_eq!(spec.body, "Discard the staged economy section change?");
        // Keep editing keeps it.
        app.dispatch_popup(&ctx, Popup::DiscardConfig, 1);
        assert!(!app.config_ui.revert_armed);
        assert!(app.working.as_ref().unwrap().economy_staged());
        // Discard drops it, with no tier message.
        app.economy_revert_pressed();
        app.dispatch_popup(&ctx, Popup::DiscardConfig, 0);
        assert!(!app.working.as_ref().unwrap().economy_staged());
        assert!(app.config_message.is_none(), "{:?}", app.config_message);
    }

    #[test]
    fn economy_revert_with_nothing_to_discard_does_not_arm() {
        let (mut app, _ctx) = popup_app(RULES);
        app.economy_revert_pressed();
        assert!(!app.config_ui.revert_armed);
    }

    #[test]
    fn rules_revert_asks_first_and_discard_reverts() {
        use crate::overlay::shell::{self, Popup};
        let (mut app, ctx) = popup_app(RULES);
        app.run_row_actions(vec![RowAction::AddRule]);
        let before = app.working.as_ref().unwrap().rows().len();
        // What the RULES bar Revert click does: arm only.
        app.revert_rules_armed = true;
        assert_eq!(
            shell::active_popup(app.popup_flags(false)),
            Some(Popup::DiscardRules)
        );
        let spec = app.popup_spec_for(Popup::DiscardRules);
        assert_eq!(spec.title, "Discard rule changes?");
        assert_eq!(spec.buttons.len(), 2);
        assert!(
            app.working.as_ref().unwrap().is_dirty(),
            "nothing reverted yet"
        );
        // Keep editing (also Escape / click outside: the last button) keeps the edit.
        app.dispatch_popup(&ctx, Popup::DiscardRules, 1);
        assert!(!app.revert_rules_armed);
        assert!(app.working.as_ref().unwrap().is_dirty());
        assert_eq!(app.working.as_ref().unwrap().rows().len(), before);
        // Discard runs the revert.
        app.revert_rules_armed = true;
        app.dispatch_popup(&ctx, Popup::DiscardRules, 0);
        assert!(!app.revert_rules_armed);
        let w = app.working.as_ref().unwrap();
        assert!(!w.is_dirty());
        assert_eq!(w.rows().len(), before - 1);
        assert_eq!(
            app.list_status.as_deref(),
            Some(crate::overlay::apply::REVERTED_MESSAGE)
        );
    }

    #[test]
    fn discard_rules_flag_needs_something_to_revert() {
        let (mut app, _ctx) = popup_app(RULES);
        app.revert_rules_armed = true;
        assert!(!app.popup_flags(false).discard_rules);
        assert!(!app.rules_revert_live());
        app.run_row_actions(vec![RowAction::AddRule]);
        assert!(app.popup_flags(false).discard_rules);
        assert!(app.rules_revert_live());
    }

    /// WR-03 / WR-04: the RULES Revert button is live exactly when its popup opens. A staged
    /// economy op alone (ECONOMY's) leaves both off, and Discard keeps it.
    #[test]
    fn rules_revert_button_and_popup_share_one_predicate() {
        use crate::overlay::shell::{self, Popup};
        let (mut app, ctx) = popup_app(RULES);
        app.working
            .as_mut()
            .unwrap()
            .stage_add_economy(&crate::overlay::rules::tests_econ_body(3));
        assert!(app.working.as_ref().unwrap().is_dirty());
        app.revert_rules_armed = true;
        assert!(!app.rules_revert_live());
        assert!(!app.popup_flags(false).discard_rules);
        app.revert_rules_armed = false;

        app.run_row_actions(vec![RowAction::AddRule]);
        assert!(app.rules_revert_live());
        app.revert_rules_armed = true;
        assert_eq!(
            shell::active_popup(app.popup_flags(false)),
            Some(Popup::DiscardRules)
        );
        app.dispatch_popup(&ctx, Popup::DiscardRules, 0);
        let w = app.working.as_ref().unwrap();
        assert!(!w.rules_edited());
        assert!(w.economy_staged(), "the ECONOMY tab's staged op survives");
        assert!(!app.rules_revert_live());
    }

    #[test]
    #[cfg(debug_assertions)]
    fn shot_popup_discard_rules_opens_the_popup() {
        use crate::overlay::shell::{self, Popup};
        let (mut app, _ctx) = popup_app(RULES);
        app.open_shot_popup(Some("discard-rules"));
        assert!(app.working.as_ref().unwrap().is_dirty());
        assert_eq!(
            shell::active_popup(app.popup_flags(false)),
            Some(Popup::DiscardRules)
        );
    }

    #[test]
    fn economy_revert_unstages_only_the_economy_op() {
        let (mut app, _ctx) = popup_app(RULES);
        assert_eq!(
            *app.working.as_ref().unwrap().economy(),
            crate::overlay::rules::EconomyView::Absent
        );
        app.run_row_actions(vec![RowAction::AddRule]);
        app.working
            .as_mut()
            .unwrap()
            .stage_add_economy(&crate::overlay::rules::tests_econ_body(3));
        assert!(app.working.as_ref().unwrap().economy_staged());
        app.unstage_economy_pressed();
        let w = app.working.as_ref().unwrap();
        assert!(!w.economy_staged());
        assert_eq!(*w.economy(), crate::overlay::rules::EconomyView::Absent);
        assert!(w.is_dirty(), "the added rule survives");
    }

    #[test]
    fn economy_info_reads_prices_json() {
        let dir = tempfile::tempdir().unwrap();
        let empty = super::EconomyInfo::read(dir.path());
        assert_eq!(empty, super::EconomyInfo::default());
        let seed = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/seed-prices.json");
        std::fs::copy(&seed, dir.path().join("prices.json")).unwrap();
        let expected = crate::loader::load_prices_optional(&seed.to_string_lossy())
            .unwrap()
            .unwrap()
            .items
            .len();
        let info = super::EconomyInfo::read(dir.path());
        assert_eq!(info.priced_items, expected);
        assert!(info.live && info.mtime_secs.is_some());
        std::fs::write(dir.path().join("prices.json"), "{ not json").unwrap();
        let bad = super::EconomyInfo::read(dir.path());
        assert!(!bad.live && bad.priced_items == 0);
        assert!(bad.problem.is_some(), "the parse error is kept");
        assert_eq!(bad.mtime_secs, None, "a broken file is not a sync time");
        // An empty file is also unusable, not a fresh sync.
        std::fs::write(dir.path().join("prices.json"), "").unwrap();
        let blank = super::EconomyInfo::read(dir.path());
        assert!(!blank.live && blank.problem.is_some() && blank.mtime_secs.is_none());
        // A missing file is simply absent: no problem to report.
        std::fs::remove_file(dir.path().join("prices.json")).unwrap();
        assert_eq!(super::EconomyInfo::read(dir.path()).problem, None);
        assert_eq!(super::EconomyInfo::read(dir.path()).league, None);
    }

    #[test]
    fn economy_info_reads_the_prices_league() {
        let dir = tempfile::tempdir().unwrap();
        let seed = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/seed-prices.json");
        let mut text: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&seed).unwrap()).unwrap();
        text["league"] = serde_json::json!("Runes of Aldur");
        std::fs::write(dir.path().join("prices.json"), text.to_string()).unwrap();
        assert_eq!(
            super::EconomyInfo::read(dir.path()).league.as_deref(),
            Some("Runes of Aldur")
        );
        text["league"] = serde_json::json!("");
        std::fs::write(dir.path().join("prices.json"), text.to_string()).unwrap();
        assert_eq!(super::EconomyInfo::read(dir.path()).league, None);
    }

    #[test]
    fn league_and_price_source_setters_keep_the_rest() {
        let mut current = crate::settings::Settings {
            filter: Some("a.filter".into()),
            ..Default::default()
        };
        current.extra.insert("future".into(), serde_json::json!(1));
        let next = super::settings_with_league(&current, "Runes of Aldur");
        assert_eq!(next.league.as_deref(), Some("Runes of Aldur"));
        assert_eq!(next.filter.as_deref(), Some("a.filter"));
        assert_eq!(next.extra.get("future"), current.extra.get("future"));
        assert_eq!(
            super::settings_with_price_source(&current, false)
                .price_source
                .as_deref(),
            Some("fallback")
        );
        assert_eq!(
            super::settings_with_price_source(&current, true)
                .price_source
                .as_deref(),
            Some("live")
        );
    }

    fn six_leagues() -> Vec<crate::sync::models::EconomyLeague> {
        const FIXTURE: &str = include_str!("../../tests/fixtures/index-state-2026-10-18.json");
        serde_json::from_str::<crate::sync::models::IndexState>(FIXTURE)
            .unwrap()
            .economy_leagues
    }

    #[test]
    fn set_league_saves_at_once_and_leaves_the_rest() {
        let (mut app, _ctx) = popup_app(RULES);
        let dirty_before = app.config.as_ref().unwrap().is_dirty();
        app.set_league("Runes of Aldur");
        assert_eq!(app.settings.saved_league(), Some("Runes of Aldur"));
        let root = app.root.clone().unwrap();
        let text = std::fs::read_to_string(crate::settings::path(&root)).unwrap();
        assert!(text.contains("Runes of Aldur"), "{text}");
        assert_eq!(app.config.as_ref().unwrap().is_dirty(), dirty_before);
    }

    #[test]
    fn set_league_with_an_unwritable_root_keeps_the_choice_and_reports() {
        let (mut app, _ctx) = popup_app(RULES);
        let file = app.root.clone().unwrap().join("not-a-folder");
        std::fs::write(&file, "x").unwrap();
        app.root = Ok(file);
        app.set_league("Runes of Aldur");
        assert_eq!(app.settings.saved_league(), Some("Runes of Aldur"));
        let (msg, problem) = app.config_message.clone().unwrap();
        assert!(
            msg.starts_with("Could not save ff-settings.json: "),
            "{msg}"
        );
        assert!(problem);
    }

    #[test]
    fn set_price_source_saves_and_gates_the_tiers() {
        let (mut app, _ctx) = popup_app(RULES);
        app.config.as_mut().unwrap().set_tier_text("A", "abc");
        app.set_price_source(false);
        assert_eq!(
            app.settings.price_policy(),
            crate::compile::PriceSourcePolicy::ForceFallback
        );
        let root = app.root.clone().unwrap();
        let text = std::fs::read_to_string(crate::settings::path(&root)).unwrap();
        assert!(text.contains("\"price_source\": \"fallback\""), "{text}");
        assert!(app.config.as_ref().unwrap().validate().is_ok());
        app.set_price_source(true);
        assert!(app.config.as_ref().unwrap().validate().is_err());
    }

    #[test]
    fn league_view_follows_the_list_and_the_price_source() {
        use crate::overlay::leagues::ListState;
        let (mut app, _ctx) = popup_app(RULES);
        assert!(matches!(app.leagues.state(), ListState::NotStarted));
        app.leagues.inject(ListState::Loaded(six_leagues()));
        assert_eq!(
            app.league_view().sync_league.as_deref(),
            Some("Forbidden Rites")
        );
        app.set_price_source(false);
        assert_eq!(app.league_view().sync_league, None);
    }

    #[test]
    fn the_test_fetch_twin_never_reaches_the_network() {
        let err = super::fetch_leagues().unwrap_err();
        assert_eq!(err, "unit tests never fetch the league list");
    }

    #[test]
    fn changed_on_disk_popup_keep_editing_closes_but_overwrite_slot_reopens() {
        use crate::overlay::shell::{Popup, active_popup};
        let (mut app, ctx) = popup_app(RULES);
        app.changed_on_disk = vec![DiskFile::Filter];
        app.changed_popup = true;
        app.overwrite_live = true;
        assert_eq!(
            active_popup(app.popup_flags(false)),
            Some(Popup::ChangedOnDisk)
        );
        // Quit outranks it.
        app.quit_confirm = true;
        assert_eq!(active_popup(app.popup_flags(false)), Some(Popup::Quit));
        app.quit_confirm = false;
        // Keep editing: the popup closes, the refusal stays.
        app.dispatch_popup(&ctx, Popup::ChangedOnDisk, 2);
        assert_eq!(active_popup(app.popup_flags(false)), None);
        assert_eq!(app.changed_on_disk, vec![DiskFile::Filter]);
        // The bottom-bar Overwrite slot sets the flag again.
        app.changed_popup = true;
        assert_eq!(
            active_popup(app.popup_flags(false)),
            Some(Popup::ChangedOnDisk)
        );
    }

    #[test]
    fn a_clean_quit_waits_for_a_running_in_game_reload() {
        let (mut app, _ctx) = popup_app(RULES);
        assert!(!app.quit_dirty());
        let (release, gate) = std::sync::mpsc::channel::<()>();
        assert!(app.reload.start(
            move || {
                let _ = gate.recv();
                Ok(())
            },
            || {}
        ));
        // The reload is still being sent: the quit is deferred, not begun, and no popup opens.
        assert!(!app.request_quit());
        assert!(app.quit_after_reload && !app.quit_confirm);
        assert!(!app.run_deferred_quit(), "still waiting for the reload");
        drop(release);
        wait_for_reload(&mut app);
        // Idle again: the deferred quit begins the shutdown.
        assert!(app.run_deferred_quit());
        assert!(!app.quit_after_reload && !app.quit_confirm);
    }

    #[test]
    fn edits_made_while_a_clean_quit_waits_open_the_quit_popup() {
        let (mut app, _ctx) = popup_app(RULES);
        assert!(!app.quit_dirty());
        let (release, gate) = std::sync::mpsc::channel::<()>();
        assert!(app.reload.start(
            move || {
                let _ = gate.recv();
                Ok(())
            },
            || {}
        ));
        assert!(!app.request_quit());
        assert!(app.quit_after_reload);
        // An edit lands while the quit waits.
        app.config.as_mut().unwrap().set_tier_text("S", "31");
        assert!(app.quit_dirty());
        drop(release);
        wait_for_reload(&mut app);
        // The quit is asked again: no shutdown, the Unapplied changes popup instead.
        assert!(!app.run_deferred_quit());
        assert!(app.quit_confirm && !app.quit_after_reload);
        assert_eq!(s_tier_text(&app), "31");
    }

    fn wait_for_reload(app: &mut super::OverlayApp) {
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
        while app.reload.poll().is_none() {
            assert!(
                std::time::Instant::now() < deadline,
                "reload worker never finished"
            );
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
    }

    #[test]
    fn quit_popup_cancel_keeps_running_and_discard_defers_one_frame() {
        use crate::overlay::shell::Popup;
        let (mut app, ctx) = popup_app(RULES);
        app.quit_confirm = true;
        app.dispatch_popup(&ctx, Popup::Quit, 2);
        assert!(!app.quit_confirm && !app.quit_next_frame);

        app.quit_confirm = true;
        app.dispatch_popup(&ctx, Popup::Quit, 1);
        assert!(!app.quit_confirm);
        assert!(app.quit_next_frame, "shutdown waits for the next frame");
        assert!(!app.quit_after_apply);
    }

    #[test]
    fn remaining_popups_close_on_the_safe_choice_without_acting() {
        use crate::overlay::shell::Popup;
        let (mut app, ctx) = popup_app(RULES);
        app.pending_switch = Some("other.filter".to_string());
        app.dispatch_popup(&ctx, Popup::SwitchFilter, 1);
        assert!(app.pending_switch.is_none());
        app.list.confirm_economy_remove = true;
        app.dispatch_popup(&ctx, Popup::RemoveEconomy, 1);
        assert!(!app.list.confirm_economy_remove);
        app.config_ui.delete_armed = Some("kite".to_string());
        app.dispatch_popup(&ctx, Popup::DeleteStyle, 1);
        assert!(app.config_ui.delete_armed.is_none());
        app.config_ui.revert_armed = true;
        app.dispatch_popup(&ctx, Popup::DiscardConfig, 1);
        assert!(!app.config_ui.revert_armed);
    }

    // ---- game folder pick (Phase 37-05) ----

    use crate::overlay::config_state::RevertScope;
    use crate::overlay::config_view::ConfigAction;
    use crate::overlay::shell::{self, ShellTab, StatusInputs};
    use crate::skin::widgets as w;

    /// An app whose app root and game folder are fresh temp folders; `settings` is the text of
    /// `ff-settings.json` when given, and `files` are created in the game folder first.
    fn folder_app(
        settings: Option<&str>,
        files: &[&str],
    ) -> (
        super::OverlayApp,
        egui::Context,
        tempfile::TempDir,
        tempfile::TempDir,
    ) {
        use crate::overlay::hotkey::HotkeyStatus;
        use crate::overlay::tray::TrayHandles;
        let ctx = egui::Context::default();
        crate::skin::fonts::install(&ctx);
        let skin = crate::skin::install(&ctx);
        let home = tempfile::tempdir().unwrap();
        let game = tempfile::tempdir().unwrap();
        if let Some(text) = settings {
            std::fs::write(home.path().join("ff-settings.json"), text).unwrap();
        }
        for f in files {
            std::fs::write(game.path().join(f), "Show\n").unwrap();
        }
        let app = super::OverlayApp::build(
            TrayHandles::inert(),
            None,
            HotkeyStatus::Failed("test".to_string()),
            None,
            skin,
            Ok(home.path().to_path_buf()),
            Some(game.path().to_path_buf()),
        );
        (app, ctx, home, game)
    }

    fn settings_text(home: &tempfile::TempDir) -> String {
        std::fs::read_to_string(home.path().join("ff-settings.json")).unwrap_or_default()
    }

    #[test]
    fn missing_game_folder_reports_on_the_status_line() {
        let (mut app, _ctx, home, _game) = folder_app(None, &[]);
        assert!(!app.game_dir_missing);
        assert_eq!(app.game_folder_missing_text(), None);
        let gone = home.path().join("nope");
        app.game_dir = Some(gone.clone());
        app.refresh_filters();
        assert!(app.game_dir_missing);
        assert_eq!(
            app.game_folder_missing_text(),
            Some(format!("Game folder not found: {}", gone.display()))
        );
        app.game_dir = Some(home.path().to_path_buf());
        app.refresh_filters();
        assert!(!app.game_dir_missing);
    }

    #[test]
    fn a_folder_that_appears_later_is_found_by_refresh_or_a_re_pick() {
        let (mut app, ctx, home, _game) = folder_app(None, &[]);
        let later = home.path().join("later");
        app.game_dir = Some(later.clone());
        app.refresh_filters();
        assert!(app.game_dir_missing);
        std::fs::create_dir(&later).unwrap();
        app.handle_config_actions(&ctx, vec![ConfigAction::RefreshLists]);
        assert!(!app.game_dir_missing);

        std::fs::remove_dir(&later).unwrap();
        app.refresh_filters();
        assert!(app.game_dir_missing);
        std::fs::create_dir(&later).unwrap();
        app.on_folder_picked(later.clone());
        assert!(!app.game_dir_missing);
        assert_eq!(app.game_dir.as_deref(), Some(later.as_path()));
    }

    #[test]
    fn missing_game_folder_is_known_right_after_build() {
        use crate::overlay::hotkey::HotkeyStatus;
        use crate::overlay::tray::TrayHandles;
        let ctx = egui::Context::default();
        crate::skin::fonts::install(&ctx);
        let skin = crate::skin::install(&ctx);
        let home = tempfile::tempdir().unwrap();
        let app = super::OverlayApp::build(
            TrayHandles::inert(),
            None,
            HotkeyStatus::Failed("test".to_string()),
            None,
            skin,
            Ok(home.path().to_path_buf()),
            Some(home.path().join("absent")),
        );
        assert!(app.game_dir_missing);
    }

    #[test]
    fn picked_folder_applies_and_saves_in_one_write_keeping_unknown_keys() {
        let (mut app, _ctx, home, _game) =
            folder_app(Some(r#"{"filter":"old.filter","keep_me":7}"#), &[]);
        let new = tempfile::tempdir().unwrap();
        std::fs::write(new.path().join("a.filter"), "Show\n").unwrap();
        std::fs::write(new.path().join("b.filter"), "Show\n").unwrap();
        app.on_folder_picked(new.path().to_path_buf());
        assert_eq!(app.game_dir.as_deref(), Some(new.path()));
        assert_eq!(
            app.filters_cache.as_deref(),
            Some(&["a.filter".to_string(), "b.filter".to_string()][..])
        );
        let v: serde_json::Value = serde_json::from_str(&settings_text(&home)).unwrap();
        assert_eq!(v["game_folder"], new.path().to_string_lossy().as_ref());
        assert_eq!(v["filter"], "a.filter");
        assert_eq!(v["keep_me"], 7);
        assert!(app.pending_folder.is_none());
        let config_edits = app.config.as_ref().map_or(0, |c| {
            c.scope_dirty_count(RevertScope::Styles) + c.scope_dirty_count(RevertScope::Tiers)
        });
        assert_eq!(config_edits, 0, "CONFIG is never marked dirty");
        assert!(app.sound_dir_stale);
    }

    #[test]
    fn folder_pick_keeps_the_same_name_when_the_folder_has_it() {
        let (mut app, _ctx, _home, _game) = folder_app(Some(r#"{"filter":"Mine.filter"}"#), &[]);
        let new = tempfile::tempdir().unwrap();
        std::fs::write(new.path().join("a.filter"), "Show\n").unwrap();
        std::fs::write(new.path().join("mine.filter"), "Show\n").unwrap();
        app.on_folder_picked(new.path().to_path_buf());
        assert_eq!(app.settings.filter_name(), "mine.filter");
    }

    #[test]
    fn folder_pick_without_filters_keeps_the_current_name_and_creates_nothing() {
        let (mut app, _ctx, _home, _game) = folder_app(Some(r#"{"filter":"mine.filter"}"#), &[]);
        let new = tempfile::tempdir().unwrap();
        app.on_folder_picked(new.path().to_path_buf());
        assert_eq!(app.settings.filter_name(), "mine.filter");
        assert_eq!(std::fs::read_dir(new.path()).unwrap().count(), 0);
    }

    #[test]
    fn dirty_rules_route_a_folder_pick_through_the_switch_popup() {
        use crate::overlay::shell::{self, Popup};
        let (mut app, ctx, home, _game) = folder_app(Some(r#"{"filter":"old.filter"}"#), &[]);
        app.working = Some(WorkingFilter::from_text(RULES));
        app.run_row_actions(vec![RowAction::AddRule]);
        let before = settings_text(&home);
        let new = tempfile::tempdir().unwrap();
        std::fs::write(new.path().join("a.filter"), "Show\n").unwrap();
        app.on_folder_picked(new.path().to_path_buf());
        assert_eq!(app.pending_folder.as_deref(), Some(new.path()));
        assert_eq!(settings_text(&home), before, "nothing written yet");
        assert_eq!(
            shell::active_popup(app.popup_flags(false)),
            Some(Popup::SwitchFilter)
        );
        let spec = app.popup_spec_for(Popup::SwitchFilter);
        assert!(spec.body.contains(&new.path().display().to_string()));
        // Stay.
        app.dispatch_popup(&ctx, Popup::SwitchFilter, 1);
        assert!(app.pending_folder.is_none());
        assert_eq!(settings_text(&home), before);
        assert_ne!(app.game_dir.as_deref(), Some(new.path()));
        // Switch and discard.
        app.on_folder_picked(new.path().to_path_buf());
        app.dispatch_popup(&ctx, Popup::SwitchFilter, 0);
        assert_eq!(app.game_dir.as_deref(), Some(new.path()));
        assert!(app.pending_folder.is_none());
        assert!(settings_text(&home).contains("game_folder"));
    }

    #[test]
    fn picking_the_current_folder_does_nothing() {
        let (mut app, _ctx, home, game) = folder_app(None, &["x.filter"]);
        app.on_folder_picked(game.path().to_path_buf());
        assert!(!home.path().join("ff-settings.json").exists());
        assert!(app.pending_folder.is_none());
    }

    #[test]
    fn browse_is_ignored_when_pinned_open_or_pending() {
        let (mut app, ctx, _home, _game) = folder_app(None, &[]);
        app.handle_config_actions(&ctx, vec![ConfigAction::BrowseFolder]);
        assert!(app.want_browse);
        app.want_browse = false;
        app.game_dir_pinned = true;
        app.handle_config_actions(&ctx, vec![ConfigAction::BrowseFolder]);
        assert!(!app.want_browse);
        app.game_dir_pinned = false;
        app.pending_switch = Some("x.filter".into());
        app.handle_config_actions(&ctx, vec![ConfigAction::BrowseFolder]);
        assert!(!app.want_browse);
        app.pending_switch = None;
        app.pending_folder = Some(std::path::PathBuf::from("C:/x"));
        app.handle_config_actions(&ctx, vec![ConfigAction::BrowseFolder]);
        assert!(!app.want_browse);
        app.pending_folder = None;
        let (tx, rx) = std::sync::mpsc::channel::<()>();
        assert!(app.folder_pick.start_with(
            move || {
                let _ = rx.recv();
                None
            },
            || {}
        ));
        app.handle_config_actions(&ctx, vec![ConfigAction::BrowseFolder]);
        assert!(!app.want_browse);
        drop(tx);
    }

    #[test]
    fn a_pinned_game_folder_ignores_a_pick() {
        let (mut app, _ctx, home, _game) = folder_app(None, &[]);
        app.game_dir_pinned = true;
        let new = tempfile::tempdir().unwrap();
        app.on_folder_picked(new.path().to_path_buf());
        assert_ne!(app.game_dir.as_deref(), Some(new.path()));
        assert!(!home.path().join("ff-settings.json").exists());
    }

    #[test]
    fn refresh_lists_relists_without_writing_or_discarding() {
        let (mut app, ctx, home, game) = folder_app(Some(r#"{"filter":"sel.filter"}"#), &[]);
        super::super::sound_files::set_dir(&ctx, Some(game.path().to_path_buf()));
        app.working = Some(WorkingFilter::from_text(RULES));
        app.run_row_actions(vec![RowAction::AddRule]);
        let before = settings_text(&home);
        std::fs::write(game.path().join("a.filter"), "Show\n").unwrap();
        std::fs::write(game.path().join("b.mp3"), b"").unwrap();
        app.handle_config_actions(&ctx, vec![ConfigAction::RefreshLists]);
        assert!(
            app.filters_cache
                .as_deref()
                .unwrap()
                .contains(&"a.filter".to_string())
        );
        assert!(
            super::super::sound_files::get(&ctx)
                .files
                .iter()
                .any(|f| f == "b.mp3")
        );
        assert_eq!(settings_text(&home), before);
        assert!(app.working.as_ref().unwrap().is_dirty());
        assert_eq!(app.settings.filter_name(), "sel.filter");
        assert!(app.config_message.is_none());
    }

    #[test]
    fn fresh_install_selects_the_folders_filter_in_memory_only() {
        let (app, _ctx, home, _game) = folder_app(None, &["x.filter"]);
        assert_eq!(app.settings.filter_name(), "x.filter");
        assert_eq!(app.settings.filter, None);
        assert!(!home.path().join("ff-settings.json").exists());
        assert!(app.filter.is_ok());
    }

    #[test]
    fn reload_keeps_the_unsaved_folder_filter() {
        let (mut app, _ctx, home, _game) = folder_app(None, &["x.filter"]);
        assert_eq!(app.settings.filter_name(), "x.filter");
        app.reload();
        assert_eq!(app.settings.filter_name(), "x.filter");
        assert!(app.filter.is_ok());
        assert!(!home.path().join("ff-settings.json").exists());
    }

    #[test]
    fn an_unusable_settings_file_opens_dynamic_filter_as_its_warning_says() {
        let (app, _ctx, _home, _game) = folder_app(Some("{ not json"), &["x.filter"]);
        assert_eq!(app.settings.filter_name(), "dynamic.filter");
        assert!(
            app.list_status
                .as_deref()
                .is_some_and(|s| s.contains("dynamic.filter"))
        );
    }

    #[test]
    fn an_invalid_stored_filter_name_opens_dynamic_filter_as_its_warning_says() {
        let (app, _ctx, _home, _game) =
            folder_app(Some(r#"{"filter":"../evil.filter"}"#), &["x.filter"]);
        assert_eq!(app.settings.filter_name(), "dynamic.filter");
        assert!(
            app.list_status
                .as_deref()
                .is_some_and(|s| s.contains("dynamic.filter"))
        );
    }

    #[test]
    fn a_warning_about_another_key_still_selects_the_folders_filter() {
        for text in [
            r#"{"game_folder":"relative"}"#,
            r#"{"reload_after_apply":"yes"}"#,
            r#"{"collapsed_sections":[]}"#,
        ] {
            let (mut app, _ctx, _home, _game) = folder_app(Some(text), &["x.filter"]);
            assert!(app.list_status.is_some(), "{text}: warned");
            assert_eq!(app.settings.filter_name(), "x.filter", "{text}: build");
            assert!(app.filter.is_ok(), "{text}");
            app.reload();
            assert_eq!(app.settings.filter_name(), "x.filter", "{text}: reload");
        }
    }

    #[test]
    fn reload_keeps_the_open_unsaved_filter_among_several() {
        let (mut app, _ctx, _home, _game) = folder_app(None, &["a.filter", "b.filter"]);
        app.settings.auto_filter = Some("b.filter".to_string());
        app.reload();
        assert_eq!(app.settings.filter_name(), "b.filter");
        assert_eq!(app.settings.filter, None);
    }

    fn saved_keys(home: &tempfile::TempDir) -> serde_json::Map<String, serde_json::Value> {
        serde_json::from_str(&settings_text(home)).unwrap()
    }

    #[test]
    fn saving_the_reload_preference_does_not_select_the_open_filter() {
        let (mut app, _ctx, home, _game) = folder_app(None, &["x.filter"]);
        assert_eq!(app.settings.filter, None);
        app.set_reload_after_apply(false);
        let keys = saved_keys(&home);
        assert_eq!(
            keys.get("reload_after_apply"),
            Some(&serde_json::json!(false))
        );
        assert!(!keys.contains_key("filter"), "{keys:?}");
        assert_eq!(app.settings.filter_name(), "x.filter");
        app.reload();
        assert_eq!(app.settings.filter_name(), "x.filter");
        assert!(!saved_keys(&home).contains_key("filter"));
    }

    #[test]
    fn a_fold_that_creates_the_settings_file_does_not_select_the_open_filter() {
        let (mut app, _ctx, home, _game) = folder_app(None, &["x.filter"]);
        app.set_section_collapsed("Mine", false, true);
        let keys = saved_keys(&home);
        assert!(keys.contains_key("collapsed_sections"), "{keys:?}");
        assert!(!keys.contains_key("filter"), "{keys:?}");
        assert!(app.settings.section_collapsed("x.filter", "Mine", false));
        assert_eq!(app.settings.filter_name(), "x.filter");
    }

    #[test]
    fn picking_the_open_unselected_filter_saves_it() {
        let (mut app, _ctx, home, _game) = folder_app(None, &["x.filter"]);
        app.select_filter("x.filter".to_string());
        assert_eq!(app.settings.filter.as_deref(), Some("x.filter"));
        assert_eq!(
            saved_keys(&home).get("filter"),
            Some(&serde_json::json!("x.filter"))
        );
    }

    #[test]
    fn picking_another_filter_or_a_folder_saves_the_selection() {
        let (mut app, _ctx, home, _game) = folder_app(None, &["x.filter", "y.filter"]);
        app.select_filter("y.filter".to_string());
        assert_eq!(
            saved_keys(&home).get("filter"),
            Some(&serde_json::json!("y.filter"))
        );
        assert_eq!(app.settings.auto_filter, None);
        let other = tempfile::tempdir().unwrap();
        std::fs::write(other.path().join("z.filter"), "Show\n").unwrap();
        app.on_folder_picked(other.path().to_path_buf());
        let keys = saved_keys(&home);
        assert_eq!(keys.get("filter"), Some(&serde_json::json!("z.filter")));
        assert_eq!(
            keys.get("game_folder"),
            Some(&serde_json::json!(other.path().to_string_lossy()))
        );
    }

    // ---- price-source switch marker, tier gate and ended league (Phase 39-05) ----

    fn with_economy_section() -> String {
        format!(
            "{RULES}\n{bar}\n# ff:economy-begin Economy (generated by FerrisFilter, do not edit)\n{bar}\n",
            bar = "#".repeat(80)
        )
    }

    /// An app on a rig root whose filter holds an economy section and whose prices are live.
    fn economy_app() -> (super::OverlayApp, egui::Context, Rig) {
        let (mut app, ctx) = popup_app(RULES);
        let rig = rig(&with_economy_section());
        app.root = Ok(rig.root.path().to_path_buf());
        app.config = ConfigState::load(rig.root.path());
        app.working = Some(WorkingFilter::from_text(&with_economy_section()));
        app.economy_info.live = true;
        app.apply_state = crate::overlay::apply::ApplyState::Applied;
        app.applied_this_session = true;
        (app, ctx, rig)
    }

    #[test]
    fn a_source_switch_is_pending_only_while_the_effective_source_differs() {
        let (mut app, _ctx, _rig) = economy_app();
        assert!(app.has_economy_section());
        assert!(!app.source_pending());
        app.set_price_source(false);
        assert!(app.source_pending());
        assert_eq!(
            app.apply_state,
            crate::overlay::apply::ApplyState::Unapplied
        );
        assert!(app.quit_dirty());
        app.set_price_source(true);
        assert!(!app.source_pending());
        assert_eq!(app.apply_state, crate::overlay::apply::ApplyState::Applied);
        assert!(!app.quit_dirty());
    }

    #[test]
    fn switching_back_with_a_rule_edit_pending_stays_unapplied() {
        let (mut app, _ctx, _rig) = economy_app();
        app.set_price_source(false);
        let id = app.working.as_ref().unwrap().rows()[0].id;
        app.working
            .as_mut()
            .unwrap()
            .apply_action(super::RowAction::Delete(id));
        app.set_price_source(true);
        assert!(!app.source_pending());
        assert_eq!(
            app.apply_state,
            crate::overlay::apply::ApplyState::Unapplied
        );
    }

    #[test]
    fn a_source_switch_never_marks_without_a_section_or_live_prices() {
        let (mut app, _ctx, _rig) = economy_app();
        app.working = Some(WorkingFilter::from_text(RULES));
        app.set_price_source(false);
        assert!(!app.source_pending());
        assert_eq!(app.apply_state, crate::overlay::apply::ApplyState::Applied);

        let (mut app, _ctx, _rig) = economy_app();
        app.economy_info.live = false;
        app.set_price_source(false);
        assert!(!app.source_pending());
        app.set_price_source(true);
        assert!(!app.source_pending());
    }

    #[test]
    fn a_league_pick_never_marks_and_a_source_switch_stays_out_of_revert() {
        let (mut app, _ctx, _rig) = economy_app();
        app.set_league("Runes of Aldur");
        assert!(!app.source_pending());
        app.set_price_source(false);
        assert!(!app.rules_revertible());
        app.economy_revert_pressed();
        assert!(!app.config_ui.revert_armed, "no Discard popup for a switch");
        assert!(app.source_pending());
        app.revert();
        assert!(app.source_pending());
        assert_eq!(
            app.apply_state,
            crate::overlay::apply::ApplyState::Unapplied
        );
    }

    #[test]
    fn apply_clears_the_source_marker() {
        let (mut app, ctx, rig) = economy_app();
        app.filter = Ok(LoadedFilter::load(&rig.path).unwrap());
        app.set_price_source(false);
        assert!(app.source_pending());
        assert!(app.run_apply(&ctx, false), "{:?}", app.apply_message);
        assert_eq!(app.applied_policy, PriceSourcePolicy::ForceFallback);
        assert!(!app.source_pending());
    }

    /// The tier cutoff `S` as currently typed in the CONFIG state.
    fn s_tier_text(app: &super::OverlayApp) -> String {
        let config = app.config.as_ref().unwrap();
        config
            .tier_rows()
            .into_iter()
            .find(|r| r.tier == "S")
            .unwrap()
            .buffer
    }

    #[test]
    fn an_apply_under_fallback_leaves_unused_tier_edits_pending_but_not_unapplied() {
        let (mut app, ctx, rig) = economy_app();
        app.filter = Ok(LoadedFilter::load(&rig.path).unwrap());
        let tiers_before = std::fs::read(rig.root.path().join("tiers.json")).ok();
        app.config.as_mut().unwrap().set_tier_text("S", "31");
        app.set_price_source(false);
        assert!(app.has_unwritable_tier_edits());
        assert!(app.run_apply(&ctx, false), "{:?}", app.apply_message);
        // Nothing an Apply could write is left, so the state stays applied...
        assert!(!app.applicable_dirty());
        assert!(!app.config.as_ref().unwrap().is_dirty());
        assert_eq!(app.apply_state, crate::overlay::apply::ApplyState::Applied);
        // ...while the cutoffs are kept, unwritten, and still guarded by the Quit confirm.
        assert_eq!(s_tier_text(&app), "31");
        assert!(app.config.as_ref().unwrap().tiers_dirty());
        assert_eq!(
            std::fs::read(rig.root.path().join("tiers.json")).ok(),
            tiers_before
        );
        assert!(app.quit_dirty());

        // Back on Live they are unapplied work again, written by the next Apply.
        app.set_price_source(true);
        assert!(app.applicable_dirty());
        assert_eq!(
            app.apply_state,
            crate::overlay::apply::ApplyState::Unapplied
        );
    }

    #[test]
    fn apply_and_quit_under_fallback_keeps_unused_tier_edits_and_stays_open() {
        use crate::overlay::shell::Popup;
        // Only unused tier edits: the confirm offers no Apply & quit, only Quit or Cancel.
        let (mut app, ctx, rig) = economy_app();
        app.filter = Ok(LoadedFilter::load(&rig.path).unwrap());
        app.applied_policy = PriceSourcePolicy::ForceFallback;
        app.set_price_source(false);
        app.config.as_mut().unwrap().set_tier_text("S", "31");
        assert!(app.quit_dirty() && !app.applicable_dirty());
        app.quit_confirm = true;
        app.refresh_quit_tier_edits_only();
        let spec = app.popup_spec_for(Popup::Quit);
        assert_eq!(spec.body, crate::overlay::shell::QUIT_TIER_EDITS_ONLY_BODY);
        let labels: Vec<&str> = spec.buttons.iter().map(|b| b.0).collect();
        assert_eq!(
            labels,
            [
                crate::overlay::apply::QUIT_DISCARD_LABEL,
                crate::overlay::apply::QUIT_CANCEL_LABEL
            ]
        );
        // Cancel keeps the window and the edits.
        app.dispatch_popup(&ctx, Popup::Quit, 1);
        assert!(!app.quit_confirm && !app.quit_after_apply && !app.quit_next_frame);
        assert_eq!(s_tier_text(&app), "31");
        // The first button quits without applying.
        app.quit_confirm = true;
        app.refresh_quit_tier_edits_only();
        app.dispatch_popup(&ctx, Popup::Quit, 0);
        assert!(app.quit_next_frame && !app.quit_after_apply);

        // The Apply & quit path itself never quits while only those edits remain: no Apply
        // runs and the window stays open with them kept.
        let (mut app, ctx, rig) = economy_app();
        app.filter = Ok(LoadedFilter::load(&rig.path).unwrap());
        app.applied_policy = PriceSourcePolicy::ForceFallback;
        app.set_price_source(false);
        app.config.as_mut().unwrap().set_tier_text("S", "31");
        app.quit_confirm = true;
        app.quit_tier_edits_only = false;
        app.dispatch_popup(&ctx, Popup::Quit, 0);
        assert!(!app.quit_after_apply && !app.quit_next_frame);
        assert_eq!(s_tier_text(&app), "31");
        let (text, kind) = app.apply_message.clone().unwrap();
        assert_eq!(text, super::UNWRITABLE_TIERS_QUIT_STATUS);
        assert_eq!(kind, w::StatusKind::Warning);

        // With other work pending too, Apply & quit is offered: that work is applied and the
        // window still stays open.
        let (mut app, ctx, rig) = economy_app();
        app.filter = Ok(LoadedFilter::load(&rig.path).unwrap());
        app.config.as_mut().unwrap().set_tier_text("S", "31");
        app.set_price_source(false);
        assert!(app.applicable_dirty());
        app.quit_confirm = true;
        app.refresh_quit_tier_edits_only();
        assert!(!app.quit_tier_edits_only);
        assert_eq!(
            app.popup_spec_for(Popup::Quit).buttons[0].0,
            crate::overlay::apply::QUIT_APPLY_LABEL
        );
        app.dispatch_popup(&ctx, Popup::Quit, 0);
        assert!(!app.quit_after_apply && !app.quit_next_frame);
        assert_eq!(app.applied_policy, PriceSourcePolicy::ForceFallback);
        assert!(!app.applicable_dirty());
        assert_eq!(s_tier_text(&app), "31");

        // Quit without applying still leaves at once.
        app.quit_confirm = true;
        app.dispatch_popup(&ctx, Popup::Quit, 1);
        assert!(app.quit_next_frame);
    }

    #[test]
    fn a_pending_source_switch_survives_switching_to_another_filter_and_back() {
        let (mut app, _ctx, rig) = economy_app();
        let dir = rig.path.parent().unwrap().to_path_buf();
        std::fs::write(dir.join("other.filter"), RULES).unwrap();
        app.game_dir = Some(dir);
        app.set_price_source(false);
        assert!(app.source_pending());

        app.switch_to("other.filter".to_string());
        // The other filter is taken as written under the policy now in force.
        assert_eq!(app.applied_policy, PriceSourcePolicy::ForceFallback);
        app.switch_to("dynamic.filter".to_string());
        app.economy_info.live = true;
        assert!(app.has_economy_section());
        assert_eq!(app.applied_policy, PriceSourcePolicy::Auto);
        assert!(app.source_pending());
    }

    #[test]
    fn a_pending_source_switch_survives_a_reload_onto_another_filter_and_back() {
        let (mut app, _ctx, rig) = economy_app();
        let dir = rig.path.parent().unwrap().to_path_buf();
        std::fs::write(dir.join("other.filter"), RULES).unwrap();
        app.game_dir = Some(dir);
        app.set_price_source(false);
        assert!(app.source_pending());
        let file = crate::settings::path(rig.root.path());
        let select = |name: &str| {
            let mut keys: serde_json::Value =
                serde_json::from_str(&std::fs::read_to_string(&file).unwrap()).unwrap();
            keys["filter"] = serde_json::json!(name);
            std::fs::write(&file, keys.to_string()).unwrap();
        };

        // ff-settings.json now names another filter: Reload opens it.
        select("other.filter");
        app.reload();
        assert_eq!(app.settings.filter_name(), "other.filter");
        assert_eq!(app.applied_policy, PriceSourcePolicy::ForceFallback);

        select("dynamic.filter");
        app.reload();
        app.economy_info.live = true;
        assert!(app.has_economy_section());
        assert_eq!(app.applied_policy, PriceSourcePolicy::Auto);
        assert!(app.source_pending());

        // A reload that keeps the same filter leaves its applied policy alone.
        app.reload();
        assert_eq!(app.applied_policy, PriceSourcePolicy::Auto);
    }

    #[test]
    fn a_league_pick_or_source_switch_never_overwrites_an_unusable_settings_file() {
        let (mut app, _ctx, rig) = economy_app();
        let file = crate::settings::path(rig.root.path());
        let bad = "{ \"filter\": \"x.filter\", }";
        std::fs::write(&file, bad).unwrap();
        app.set_league("Runes of Aldur");
        assert_eq!(app.settings.saved_league(), Some("Runes of Aldur"));
        assert!(
            app.config_message
                .as_ref()
                .is_some_and(|(m, err)| *err && m.starts_with("Could not save ff-settings.json"))
        );
        app.set_price_source(false);
        assert_eq!(
            app.settings.price_policy(),
            PriceSourcePolicy::ForceFallback
        );
        assert_eq!(std::fs::read_to_string(&file).unwrap(), bad);
    }

    #[test]
    fn a_league_pick_writes_only_its_own_key() {
        let (mut app, _ctx, rig) = economy_app();
        let file = crate::settings::path(rig.root.path());
        std::fs::write(
            &file,
            r#"{"filter":"ssf.filter","reload_after_apply":"yes","future":1}"#,
        )
        .unwrap();
        app.set_league("Runes of Aldur");
        app.set_price_source(false);
        assert!(app.config_message.is_none(), "{:?}", app.config_message);
        let v: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&file).unwrap()).unwrap();
        assert_eq!(v["filter"], "ssf.filter");
        assert_eq!(v["reload_after_apply"], "yes");
        assert_eq!(v["future"], 1);
        assert_eq!(v["league"], "Runes of Aldur");
        assert_eq!(v["price_source"], "fallback");
    }

    #[test]
    fn the_economy_preview_is_repriced_while_a_switch_is_pending() {
        let (mut app, _ctx, _rig) = economy_app();
        app.refresh_staged_economy();
        assert!(app.staged_econ_inputs.is_none());
        app.set_price_source(false);
        app.refresh_staged_economy();
        let key = app.staged_econ_inputs.clone().expect("preview wanted");
        assert_eq!(key.2, PriceSourcePolicy::ForceFallback);
    }

    #[test]
    fn the_ended_league_line_shows_only_with_a_loaded_list_under_live() {
        use crate::overlay::leagues::ListState;
        let (mut app, _ctx) = popup_app(RULES);
        app.settings.league = Some("Fate of the Vaal".to_string());
        let ended = |app: &super::OverlayApp| {
            app.league_view()
                .ended
                .as_deref()
                .map(crate::overlay::leagues::ended_status)
        };
        assert_eq!(ended(&app), None);
        app.leagues.inject(ListState::Loading);
        assert_eq!(ended(&app), None);
        app.leagues.inject(ListState::Failed("x".into()));
        assert_eq!(ended(&app), None);
        app.leagues.inject(ListState::Loaded(six_leagues()));
        assert_eq!(
            ended(&app).as_deref(),
            Some("Fate of the Vaal is no longer on poe.ninja; pick a league")
        );
        app.settings.price_source = Some("fallback".to_string());
        assert_eq!(ended(&app), None);
    }

    #[test]
    fn the_missing_folder_status_row_opens_the_config_tab() {
        let s = shell::status_for(StatusInputs {
            root_error: None,
            event: None,
            grammar_gate: None,
            filter_error: None,
            game_folder_missing: Some("Game folder not found: C:/x".into()),
            syncing: false,
            ended_league: None,
            unknown: None,
            migrated: false,
            absent: false,
            indicator: ("i", w::StatusKind::Info),
        });
        assert_eq!(s.open_tab, Some(ShellTab::Config));
    }
}
