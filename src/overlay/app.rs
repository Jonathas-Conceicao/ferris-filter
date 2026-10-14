//! `OverlayApp` -- the `eframe::App` implementation for the stripe window.
//!
//! What this file owns: docking the stripe to the right screen edge on the first frame where a
//! monitor size is known; polling the tray and the global hotkey for their events; turning those
//! events into an actual show/hide of the window that does not steal keyboard focus from the game,
//! by delegating the decision to [`super::visibility`] and the OS call to [`super::win`]; and the
//! filter-as-store footer (Phase 18).
//!
//! **Filter-as-store (Phase 18) and the rule list (Phase 19).** The game-folder `dynamic.filter`
//! is the store. On launch the overlay resolves the app root ([`crate::paths::app_root`], never
//! the working directory, so a double-click launch works) and loads the filter
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
//! write refuses because the file changed on disk, the stripe offers RELOAD, which re-reads the
//! file and discards unapplied edits, and OVERWRITE, which writes the edits over the outside change.
//!
//! The decisions (which button is live, what the status says) are pure functions in
//! [`super::apply`]; this file only draws them and performs the calls.
//! `PITFALLS.md` §8: if a bug can only be found by clicking, it belongs in a library module.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::time::Duration;

use global_hotkey::GlobalHotKeyManager;

use crate::compile::{InputPaths, PriceSourcePolicy};
use crate::dynamic::{self, LoadedFilter};
use crate::filter::{BlockId, LinePatch};
use crate::loader;
use crate::models::config::StyleSet;
use crate::settings::{self, Settings};

use super::apply::{self, ApplyOutcome, ApplyState, FilterState};
use super::apply_all::{self, ApplyAllError, DiskFile, FilterInput};
use super::chat;
use super::classes::ClassCatalog;
use super::config_state::{self, ConfigState, OUT_OF_DATE_MESSAGE, OutsideEdit};
use super::config_view::{self, ConfigAction, ConfigUi, ConfigViewCtx};
use super::economy;
use super::hotkey::{self, HotkeyStatus};
use super::instance;
use super::layout;
use super::rule_list::{self, ListUi};
use super::rules::{self, EconomyView, RowAction, WorkingFilter};
use super::theme;
use super::tray::{self, OverlayCommand, TrayHandles};
use super::visibility::{self, Visibility};
use super::widgets;
use super::win::{self, OverlayWindow};

/// The eframe application state for the overlay stripe.
pub struct OverlayApp {
    /// Latches `true` once the first-frame docking has run, so it never runs
    /// again (position/width are not persisted or re-derived per frame).
    docked: bool,

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
    /// finally drops the app: a relaunch right after Quit then starts a new stripe instead of
    /// signalling a process that is going away (review iteration 2, WR-01).
    instance: Option<instance::Guard>,

    /// Set by the tray's left click, a `Ctrl+Shift+Z` press, or a tray "Show"
    /// while currently hidden. Consumed once per frame by [`visibility::advance`].
    toggle_requested: bool,

    /// The show/hide decision -- see `super::visibility` for why this is three states, not two.
    visibility: Visibility,

    /// The stripe's own cached window handle (see `super::win`), populated
    /// on the first frame one is available. `None` on the very first few
    /// frames, and always `None` off Windows.
    window: Option<OverlayWindow>,

    /// The resolved FerrisFilter folder (styles.json, tiers.json, prices.json live here). `Err`
    /// holds the rendered message; the stripe shows it instead of panicking (STORE-05).
    root: Result<std::path::PathBuf, String>,

    /// The loaded game-folder filter. `Err` holds the rendered load failure.
    filter: Result<LoadedFilter, String>,

    /// The files found changed on disk (since they were loaded) by the last refused Apply;
    /// RELOAD or a successful OVERWRITE clears it.
    changed_on_disk: Vec<DiskFile>,

    /// Whether the filter on disk matches what the last Apply wrote, as far as this session
    /// knows. A failed Apply leaves it exactly as it was.
    apply_state: ApplyState,

    /// The most recent Apply outcome's rendered text -- a success message naming the reload path,
    /// or an error's display string.
    apply_message: Option<String>,

    /// The economy-sync affordance: a `prices.json` refresh run on a worker thread so a stripe
    /// drawn over a live game never freezes on network I/O. It writes prices only -- the game sees
    /// them on the next **Apply**, which is what makes it safe to press mid-map.
    economy: economy::EconomySync,

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

    /// Whether an Apply has succeeded this session; decides what Revert returns the indicator to.
    applied_this_session: bool,

    /// The class catalogue for the class picker, from `config.json`. `Err` degrades the picker
    /// to the generic chip list; it never blocks editing.
    classes: Result<ClassCatalog, String>,

    /// Modification time of `styles.json` when it was last read; a change reloads the styles.
    styles_mtime: Option<std::time::SystemTime>,

    /// `ctx.input(|i| i.time)` of the last `styles.json` poll.
    styles_checked: f64,

    /// Which top-level tab is showing. Switching never applies or discards anything.
    tab: Tab,

    /// The CONFIG tab's model (styles + tiers edits). `Err` holds the rendered load failure.
    config: Result<ConfigState, String>,

    /// View state of the CONFIG tab.
    config_ui: ConfigUi,

    /// The CONFIG footer's latest message and whether it is an accent (problem) line.
    config_message: Option<(String, bool)>,

    /// A config file changed outside the overlay (checked on activation and at Apply only).
    config_outside: OutsideEdit,

    /// A CONFIG Apply landed that no RULES Apply has regenerated the filter for yet.
    config_changed_pending: bool,

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
}

/// The two top-level tabs of the stripe.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Tab {
    Rules,
    Config,
}

/// Seconds between `styles.json` modification-time polls while the stripe runs.
const STYLES_POLL_SECS: f64 = 2.0;

fn load_styles_at(root: &std::path::Path) -> Result<StyleSet, String> {
    let path = InputPaths::relative_to(root).styles;
    loader::load_styles(&path.to_string_lossy()).map_err(|e| e.to_string())
}

/// The class catalogue and the styles `config.json` references, from one load of `config.json`.
fn load_classes_at(root: &std::path::Path) -> Result<(ClassCatalog, BTreeSet<String>), String> {
    let paths = InputPaths::relative_to(root);
    loader::load_config(
        &paths.config.to_string_lossy(),
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

fn tiers_mtime_at(root: &std::path::Path) -> Option<std::time::SystemTime> {
    std::fs::metadata(InputPaths::relative_to(root).tiers)
        .and_then(|m| m.modified())
        .ok()
}

/// Draws the RULES | CONFIG strip and returns the tab the user picked this frame.
/// `*` on a label means the OTHER tab holds unapplied edits.
fn tab_strip(
    ui: &mut egui::Ui,
    active: Tab,
    rules_marker: bool,
    config_marker: bool,
) -> Option<Tab> {
    let mut picked = None;
    let seg_w = ((ui.available_width() - 4.0) / 2.0).max(0.0);
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 4.0;
        for (tab, name, marker) in [
            (Tab::Rules, "RULES", rules_marker),
            (Tab::Config, "CONFIG", config_marker),
        ] {
            let (rect, resp) =
                ui.allocate_exact_size(egui::vec2(seg_w, 28.0), egui::Sense::click());
            let is_active = tab == active;
            let colour = if is_active || resp.hovered() {
                theme::SECTION_CREAM
            } else {
                theme::TAB_ACTIVE
            };
            let fill = if is_active {
                theme::HEADER_PLATE
            } else {
                theme::ROW_BASE
            };
            let mut job = theme::row_label(&apply::tab_label(name, marker));
            for section in &mut job.sections {
                section.format.color = colour;
            }
            let painter = ui.painter();
            painter.rect_filled(rect, 0.0, fill);
            let galley = painter.layout_job(job);
            painter.galley(rect.center() - galley.size() / 2.0, galley, colour);
            if is_active {
                painter.rect_filled(
                    egui::Rect::from_min_max(
                        egui::pos2(rect.left(), rect.bottom() - 2.0),
                        rect.right_bottom(),
                    ),
                    0.0,
                    theme::SECTION_CREAM,
                );
            }
            let resp = if marker {
                resp.on_hover_text(apply::OTHER_TAB_DIRTY_TIP)
            } else {
                resp
            };
            resp.widget_info(|| {
                egui::WidgetInfo::selected(egui::WidgetType::Button, true, is_active, name)
            });
            if resp.clicked() && !is_active {
                picked = Some(tab);
            }
            if resp.has_focus() {
                let (left, right) = ui.input(|i| {
                    (
                        i.key_pressed(egui::Key::ArrowLeft),
                        i.key_pressed(egui::Key::ArrowRight),
                    )
                });
                if tab == Tab::Rules && right {
                    picked = Some(Tab::Config);
                } else if tab == Tab::Config && left {
                    picked = Some(Tab::Rules);
                }
            }
        }
    });
    picked
}

fn styles_mtime_at(root: &std::path::Path) -> Option<std::time::SystemTime> {
    std::fs::metadata(InputPaths::relative_to(root).styles)
        .and_then(|m| m.modified())
        .ok()
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
    overwrite: bool,
) -> AllResult {
    let filter_path = filter.as_ref().map(|(l, _)| l.path().to_path_buf());

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
                None => working.text_with_styles().map(|(text, r)| {
                    report = Some(r);
                    text
                }),
            };
            Some(FilterInput {
                loaded: &**loaded,
                styled,
                plan: working.economy_plan(),
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
            working.set_styles(Ok(c.pending_styles().clone()));
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

/// Applies the frame's collected row actions once, after the list rendered. Returns whether the
/// document changed and the last status worth showing.
fn apply_row_actions(
    working: &mut WorkingFilter,
    list: &mut ListUi,
    query: &str,
    actions: Vec<RowAction>,
) -> (bool, Option<String>) {
    let mut changed = false;
    let mut status = None;
    for action in rules::coalesce_edits(actions) {
        let is_add = matches!(action, RowAction::AddRule);
        let is_duplicate = matches!(action, RowAction::Duplicate(_));
        let is_move = matches!(action, RowAction::Move(..));
        let deleted = match &action {
            RowAction::Delete(id) => Some(*id),
            _ => None,
        };
        let edited_block = match &action {
            RowAction::EditLine { id, .. }
            | RowAction::AddLine { id, .. }
            | RowAction::RemoveLine { id, .. } => Some(*id),
            _ => None,
        };
        let added_in = match &action {
            RowAction::AddLine { id, .. } => Some(*id),
            _ => None,
        };
        // Edits that can shift the keyword ordinals of the cards after them: removing or adding
        // a line, and a raw whole-line edit that changes the line's keyword (WR-07, IN-07).
        let shifts_ordinals = match &action {
            RowAction::RemoveLine { .. } | RowAction::AddLine { .. } => true,
            RowAction::EditLine {
                expect,
                patch: LinePatch::Whole(text),
                ..
            } => line_keyword(expect) != line_keyword(text),
            _ => false,
        };
        let refused_edit = match &action {
            RowAction::EditLine {
                id, line, patch, ..
            } if matches!(patch, LinePatch::Whole(_) | LinePatch::Values(_)) => Some((*id, *line)),
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
                // A rule the search hides cannot be expanded or scrolled to: the list resets
                // both on the next frame because the row is not in the filtered slice.
                let hidden =
                    is_add && !query.trim().is_empty() && !working.row_matches(focus, query);
                if hidden {
                    if let Some(row) = working.row(focus) {
                        status = Some(apply::added_hidden_message(row.index));
                    }
                } else if is_add || is_duplicate {
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

/// The rule count beside the FILTER header: `142 rules`, `1 rule`, or `7 of 142 rules` while a
/// search is active.
fn rule_count_text(total: usize, shown: usize, filtering: bool) -> String {
    let noun = if total == 1 { "rule" } else { "rules" };
    if filtering {
        format!("{shown} of {total} {noun}")
    } else {
        format!("{total} {noun}")
    }
}

/// A heading plus a wrapped explanation, used for the empty and error states of the list.
fn centered_notice(ui: &mut egui::Ui, heading: &str, body: &str) {
    ui.add_space(16.0);
    ui.vertical_centered(|ui| {
        ui.label(theme::section_header(heading));
        ui.add_space(8.0);
        ui.add(egui::Label::new(egui::RichText::new(body).color(theme::TAB_ACTIVE)).wrap());
    });
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
            "Cannot resolve the default PoE2 config folder (USERPROFILE is not set).".to_string(),
        ),
    }
}

/// The settings to save when `name` becomes the active filter; unknown keys are preserved.
fn settings_with_filter(current: &Settings, name: &str) -> Settings {
    Settings {
        filter: Some(name.to_string()),
        reload_after_apply: current.reload_after_apply,
        extra: current.extra.clone(),
    }
}

/// The settings RELOAD keeps after re-reading `ff-settings.json`.
///
/// A clean load, or a load from a different root, replaces the session's settings. A load that
/// warned keeps the in-memory filter and unknown keys (a momentarily unreadable file must not
/// reset them), but an "off" from the file is still honoured: `settings::load` reports a broken
/// or wrongly typed file as reload-off, and the warning says so, so keeping the old "on" would
/// send keystrokes the stripe has just said it won't send.
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

/// `current` with the reload-after-Apply preference replaced; the filter and unknown keys stay.
fn settings_with_reload(current: &Settings, on: bool) -> Settings {
    Settings {
        reload_after_apply: Some(on),
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
}

/// The Apply status line after the stripe is concealed. Hiding normally clears it, but not while
/// a reload is in flight and its interim line is still showing: the reload's own focus change is
/// what hides the stripe (the click-away rule sees the game take focus), so clearing here would
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
/// lands while the stripe is hidden is still shown on the next reveal.
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
    ) -> Self {
        let root = crate::paths::app_root().map_err(|e| e.to_string());
        let game_dir = dynamic::default_destination();
        // Settings: the remembered filter. Without a root nothing is read and nothing is saved.
        let (settings, settings_warning) = match &root {
            Ok(root) => settings::load(root),
            Err(_) => (Settings::default(), None),
        };
        // Loading never writes (D-10).
        let filter = Self::load_filter(game_dir.as_deref(), settings.filter_name());
        let working = working_from(&filter);
        let config = match &root {
            Ok(root) => ConfigState::load(root),
            Err(message) => Err(message.clone()),
        };

        let mut app = Self {
            docked: false,
            tray,
            hotkey_manager,
            hotkey_status,
            instance: Some(instance),
            toggle_requested: false,
            // The window created in `overlay::launch` starts visible
            // (`with_active(false)` only suppresses activation, not
            // visibility) and has never been focused -- `ShownUnfocused` is
            // the state that matches that physical reality. Starting here
            // (rather than `Hidden`) is what makes the very first `Ctrl+Shift+Z`
            // press after launch hide the stripe, matching the checkpoint
            // script's step 3.
            visibility: Visibility::ShownUnfocused,
            window: None,
            root,
            filter,
            changed_on_disk: Vec::new(),
            apply_state: ApplyState::NeverApplied,
            apply_message: None,
            economy: economy::EconomySync::new(),
            reload: chat::ReloadSend::default(),
            reload_pending: None,
            working,
            list: ListUi::default(),
            query: String::new(),
            list_status: None,
            applied_this_session: false,
            classes: Err("not loaded".to_string()),
            styles_mtime: None,
            styles_checked: 0.0,
            tab: Tab::Rules,
            config,
            config_ui: ConfigUi::default(),
            config_message: None,
            config_outside: OutsideEdit::None,
            config_changed_pending: false,
            config_refs: BTreeSet::new(),
            settings,
            game_dir,
            filters_cache: None,
            pending_switch: None,
        };
        app.refresh_inputs();
        if settings_warning.is_some() {
            app.list_status = settings_warning;
        }
        app.config_changed_pending = app.compute_config_pending();
        app
    }

    /// Re-reads the game folder's filter list into the cache.
    fn refresh_filters(&mut self) {
        self.filters_cache = self
            .game_dir
            .as_deref()
            .and_then(|dir| settings::list_filters(dir).ok());
    }

    /// Whether styles.json / tiers.json are newer than the loaded filter file. Read once per
    /// load, never per frame.
    fn compute_config_pending(&self) -> bool {
        let (Ok(root), Ok(loaded)) = (&self.root, &self.filter) else {
            return false;
        };
        let filter_mtime = std::fs::metadata(loaded.path())
            .and_then(|m| m.modified())
            .ok();
        apply::config_newer_than_filter(filter_mtime, styles_mtime_at(root), tiers_mtime_at(root))
    }

    /// (Re)reads `styles.json` into the working filter and `config.json` into the class
    /// catalogue. A config failure is shown once in the list status and never blocks editing.
    fn refresh_inputs(&mut self) {
        match &self.root {
            Ok(root) => {
                let root = root.clone();
                self.styles_mtime = styles_mtime_at(&root);
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
                self.styles_mtime = None;
                if let Some(working) = &mut self.working {
                    working.set_styles(Err(message.clone()));
                }
                self.classes = Err(message.clone());
            }
        }
    }

    /// Reloads the styles when `styles.json` changed on disk (polled, not watched).
    fn poll_styles(&mut self, now: f64) {
        if now - self.styles_checked < STYLES_POLL_SECS {
            return;
        }
        self.styles_checked = now;
        let (Ok(root), Some(working)) = (&self.root, &mut self.working) else {
            return;
        };
        let mtime = styles_mtime_at(root);
        if mtime != self.styles_mtime {
            self.styles_mtime = mtime;
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
    }

    /// Makes `name` the active filter now and remembers it. A failed save still switches for
    /// this session and says so.
    fn switch_to(&mut self, name: String) {
        let next = settings_with_filter(&self.settings, &name);
        let save_error = match &self.root {
            Ok(root) => settings::save(root, &next).err(),
            Err(_) => None,
        };
        self.settings = next;
        self.reset_filter_state();
        self.refresh_inputs();
        self.config_changed_pending = self.compute_config_pending();
        self.refresh_filters();
        if let Some(err) = save_error {
            self.config_message = Some((format!("Could not save ff-settings.json: {err}"), true));
        }
    }

    /// Persist the reload-after-Apply preference at once. A view preference, not a CONFIG draft
    /// edit: it never touches the loaded filter or the CONFIG pending state.
    fn set_reload_after_apply(&mut self, on: bool) {
        let next = settings_with_reload(&self.settings, on);
        let save_error = match &self.root {
            Ok(root) => settings::save(root, &next).err(),
            Err(_) => None,
        };
        self.settings = next;
        if let Some(err) = save_error {
            self.config_message = Some((format!("Could not save ff-settings.json: {err}"), true));
        }
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
    fn run_apply(&mut self, ctx: &egui::Context, overwrite: bool) {
        let Ok(root) = &self.root else {
            return;
        };
        // Ids reset on re-parse, so remember the expanded row by position instead.
        let expanded_index = self
            .working
            .as_ref()
            .and_then(|w| self.list.expanded.and_then(|id| w.row(id)))
            .map(|r| r.index);
        // Without an in-memory CONFIG the styles are re-read at every Apply, so a styles.json
        // edited a moment ago counts.
        if self.config.is_err()
            && let Some(working) = &mut self.working
        {
            self.styles_mtime = styles_mtime_at(root);
            working.set_styles(load_styles_at(root));
        }
        let filter_part = match (&mut self.filter, &mut self.working) {
            (Ok(loaded), Some(working)) => Some((loaded, working)),
            _ => None,
        };
        let result = apply_everything(root, self.config.as_mut().ok(), filter_part, overwrite);
        let AllResult {
            outcome,
            extras,
            styles_written,
            tiers_written,
            changed,
        } = result;
        let applied_path = match outcome {
            AllOutcome::Failed(message) => {
                self.apply_message = Some(message);
                self.apply_state = self.apply_state.on_apply_failure();
                if !changed.is_empty() {
                    self.changed_on_disk = changed;
                }
                return;
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
                // No cached stripe window means no clipboard owner: skip rather than fall back
                // to an ownerless clipboard, whose write fails after emptying it (review WR-02).
                let decision =
                    chat::with_owner(decision, self.window.as_ref().map(win::clipboard_owner));
                match decision {
                    Some(chat::Decision::Send((game, owner))) => {
                        let interim =
                            format!("{prefix}{}{extras}", apply::reloading_message(&path));
                        self.apply_message = Some(interim.clone());
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
                            });
                        } else {
                            self.apply_message = Some(format!(
                                "{prefix}{}",
                                skipped_reload_message(&path, &extras, chat::SkipReason::Busy)
                            ));
                        }
                    }
                    Some(chat::Decision::Skip(reason)) => {
                        self.apply_message = Some(format!(
                            "{prefix}{}",
                            skipped_reload_message(&path, &extras, reason)
                        ));
                    }
                    None => {
                        self.apply_message =
                            Some(format!("{prefix}{}{extras}", apply::success_message(&path)));
                    }
                }
            }
            // No filter part: the config files alone, nothing to reload in game.
            None => {
                self.apply_message = Some(format!(
                    "{}{extras}",
                    apply::config_only_message(styles_written, tiers_written)
                ));
            }
        }
        self.apply_state = self.apply_state.on_apply_success();
        self.config_changed_pending = false;
        self.config_outside = OutsideEdit::None;
        self.config_message = None;
        self.config_ui.disarm_all();
        self.styles_mtime = styles_mtime_at(root);
        self.changed_on_disk.clear();
        self.applied_this_session = true;
        self.list_status = None;
        self.list.clear_ids();
        self.list.expanded = expanded_index.and_then(|i| {
            self.working
                .as_ref()
                .and_then(|w| w.rows().get(i - 1).map(|r| r.id))
        });
    }

    /// REVERT: discard every in-memory edit and staged economy op.
    fn revert(&mut self) {
        let Some(working) = &mut self.working else {
            return;
        };
        working.revert();
        self.list.clear_ids();
        self.list_status = Some(apply::REVERTED_MESSAGE.to_string());
        self.apply_state = if self.applied_this_session {
            ApplyState::Applied
        } else {
            ApplyState::NeverApplied
        };
    }

    /// Stages adding the economy section; it is written by the next APPLY.
    fn stage_add_economy(&mut self) {
        let (Ok(root), Some(working)) = (&self.root, &mut self.working) else {
            return;
        };
        match dynamic::economy_body(&InputPaths::relative_to(root), PriceSourcePolicy::Auto) {
            Ok((body, _)) => {
                working.stage_add_economy(rules::count_blocks(&body));
                self.apply_state = ApplyState::Unapplied;
            }
            Err(err) => self.list_status = Some(err.to_string()),
        }
    }

    /// Stages removing the economy section; it is written by the next APPLY.
    fn stage_remove_economy(&mut self) {
        if let Some(working) = &mut self.working {
            working.stage_remove_economy();
            self.apply_state = ApplyState::Unapplied;
        }
    }

    /// RELOAD: re-resolves the app root, re-reads the filter and discards in-memory state (D-08).
    /// Re-resolving the root here is what lets a stripe launched before the FerrisFilter folder
    /// was in place recover without a restart.
    fn reload(&mut self) {
        let old_root = self.root.clone();
        self.root = crate::paths::app_root().map_err(|e| e.to_string());
        self.game_dir = dynamic::default_destination();
        let settings_warning = match &self.root {
            Ok(root) => {
                let (settings, warning) = settings::load(root);
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
            self.config_outside = OutsideEdit::None;
        }
        self.config_changed_pending = self.compute_config_pending();
    }

    /// Applies the CONFIG tab's collected actions. Filter actions are handled by the filter
    /// selection code; everything else edits the `ConfigState`.
    fn handle_config_actions(&mut self, ctx: &egui::Context, actions: Vec<ConfigAction>) {
        for action in actions {
            match action {
                ConfigAction::AddStyle(input) => {
                    if let Ok(state) = &mut self.config {
                        match state.add_style(&input) {
                            Ok(name) => {
                                self.config_ui.set_expanded(&name);
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
                        self.config_ui.disarm_all();
                        self.config_message = Some(("Reverted config changes.".to_string(), false));
                    }
                }
                ConfigAction::Apply => self.run_apply(ctx, false),
                ConfigAction::Overwrite => self.run_apply(ctx, true),
                ConfigAction::Reload => {
                    self.config = match &self.root {
                        Ok(root) => ConfigState::load(root),
                        Err(message) => Err(message.clone()),
                    };
                    self.config_outside = OutsideEdit::None;
                    self.config_message = None;
                    self.config_ui.disarm_all();
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
                    if let Some(name) = self.pending_switch.take() {
                        self.switch_to(name);
                    }
                }
                ConfigAction::CancelSwitch => self.pending_switch = None,
            }
        }
    }

    /// A filter picked in the CONFIG dropdown (or just named). Unapplied rule edits make it wait
    /// for confirmation; the dropdown keeps showing the current name until then. Only real edits
    /// count: a pending style rewrite is derived, not an edit.
    fn select_filter(&mut self, name: String) {
        let rules_dirty = self.working.as_ref().is_some_and(|w| w.is_dirty());
        match switch_decision(rules_dirty, self.settings.filter_name(), &name) {
            SwitchDecision::Noop => self.pending_switch = None,
            SwitchDecision::Switch => self.switch_to(name),
            SwitchDecision::Confirm => self.pending_switch = Some(name),
        }
    }

    /// Draws the CONFIG tab below the tab strip and returns what the user asked for.
    fn config_body(&mut self, ui: &mut egui::Ui) -> Vec<ConfigAction> {
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
            game_dir: self.game_dir.as_deref(),
            pending_switch: self.pending_switch.as_deref(),
            rule_users: &users,
            config_refs: &self.config_refs,
            outside: self.config_outside,
            out_of_date: self.config_changed_pending,
            reload_after_apply: self.settings.reload_after_apply(),
        };
        // The footer is claimed first so it keeps its height whatever the sections hold; the
        // view's own scroll area then takes the rest (no ScrollArea nested around it).
        egui::Panel::bottom(egui::Id::new("ff-config-footer"))
            .frame(
                egui::Frame::NONE
                    .fill(theme::PANEL_GROUND)
                    .inner_margin(egui::Margin {
                        left: 8,
                        right: 8,
                        top: 0,
                        bottom: 8,
                    }),
            )
            .show(ui, |ui| {
                // The shared Apply result shows on whichever tab is open.
                let apply_line = self.apply_message.as_ref().map(|m| {
                    let ok =
                        self.apply_state == ApplyState::Applied && self.changed_on_disk.is_empty();
                    (m.clone(), !ok)
                });
                let lines: Vec<OutcomeLine> = self
                    .config_message
                    .iter()
                    .cloned()
                    .chain(apply_line)
                    .map(|(text, accent)| OutcomeLine {
                        text,
                        accent,
                        jump: None,
                    })
                    .collect();
                outcome_area(ui, "config-outcome", &lines);
                if let Ok(state) = &self.config {
                    actions.extend(config_view::show_footer(
                        ui,
                        state,
                        &mut self.config_ui,
                        &ctx,
                    ));
                }
            });
        egui::Frame::NONE
            .inner_margin(egui::Margin::symmetric(8, 0))
            .show(ui, |ui| match &self.config {
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
}

/// One message in the footer's outcome area.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct OutcomeLine {
    pub text: String,
    pub accent: bool,
    pub jump: Option<BlockId>,
}

/// Height cap of the outcome area; longer content scrolls inside it.
pub(crate) const OUTCOME_MAX_HEIGHT: f32 = 96.0;

/// The present footer messages, in display order: list status, Apply outcome, sync status, then
/// the persistent grammar-gate and unknown-style lines.
pub(crate) fn outcome_lines(
    list_status: Option<String>,
    apply_message: Option<(String, bool)>,
    sync_status: Option<(String, bool)>,
    grammar_gate: Option<(String, BlockId)>,
    unknown_summary: Option<(String, BlockId)>,
) -> Vec<OutcomeLine> {
    let mut out = Vec::new();
    if let Some(text) = list_status {
        out.push(OutcomeLine {
            text,
            accent: false,
            jump: None,
        });
    }
    for (text, accent) in [apply_message, sync_status].into_iter().flatten() {
        out.push(OutcomeLine {
            text,
            accent,
            jump: None,
        });
    }
    for (text, id) in [grammar_gate, unknown_summary].into_iter().flatten() {
        out.push(OutcomeLine {
            text,
            accent: true,
            jump: Some(id),
        });
    }
    out
}

/// Draws the outcome lines inside one capped scroll area. Returns the rule a clicked line jumps to.
pub(crate) fn outcome_area(
    ui: &mut egui::Ui,
    id_salt: &str,
    lines: &[OutcomeLine],
) -> Option<BlockId> {
    if lines.is_empty() {
        return None;
    }
    let mut jump = None;
    egui::ScrollArea::vertical()
        .id_salt(id_salt)
        .max_height(OUTCOME_MAX_HEIGHT)
        .auto_shrink([false, true])
        .show(ui, |ui| {
            for line in lines {
                ui.horizontal_wrapped(|ui| {
                    ui.add_space(8.0);
                    let color = if line.accent {
                        theme::WARM_ACCENT_TEXT
                    } else {
                        theme::TAB_ACTIVE
                    };
                    let mut label =
                        egui::Label::new(egui::RichText::new(&line.text).size(13.0).color(color))
                            .wrap();
                    if line.jump.is_some() {
                        label = label.sense(egui::Sense::click());
                    }
                    let resp = ui.add(label);
                    if let Some(target) = line.jump
                        && resp
                            .on_hover_cursor(egui::CursorIcon::PointingHand)
                            .clicked()
                    {
                        jump = Some(target);
                    }
                });
            }
        });
    jump
}

impl eframe::App for OverlayApp {
    fn ui(&mut self, ui: &mut egui::Ui, frame: &mut eframe::Frame) {
        if !self.docked {
            // The work area (screen minus taskbar) when Windows will tell us, falling back to
            // the raw monitor size elsewhere. Docking against the monitor put the APPLY row
            // behind the taskbar (screenshot poe2_print008).
            let monitor_size = ui.ctx().input(|i| i.viewport().monitor_size);
            if let Some(monitor_size) = monitor_size {
                let rect = match win::work_area() {
                    Some(area) => layout::dock_rect_in(area, layout::STRIPE_WIDTH),
                    None => layout::dock_rect(monitor_size, layout::STRIPE_WIDTH),
                };
                ui.ctx()
                    .send_viewport_cmd(egui::ViewportCommand::OuterPosition(rect.min));
                ui.ctx()
                    .send_viewport_cmd(egui::ViewportCommand::InnerSize(rect.size()));
                self.docked = true;
            }
            // When monitor_size is None, leave the builder defaults in place —
            // never guess a monitor size. Retried on a later frame.
        }

        // Cache the stripe's own HWND the first frame one is available
        // (RESEARCH Pattern 1). Retried every frame until it succeeds.
        if self.window.is_none() {
            self.window = win::from_frame(frame);
        }

        // Poll the global hotkey (D-09). Accept only our own registered id
        // and only the Pressed state — without the state filter, a single
        // keypress toggles twice (once on press, once on release).
        if let Ok(event) = global_hotkey::GlobalHotKeyEvent::receiver().try_recv() {
            if let HotkeyStatus::Registered { id } = &self.hotkey_status {
                if event.id == *id && event.state == global_hotkey::HotKeyState::Pressed {
                    // Only while the user is actually in the game — or in the stripe itself, so
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
        // run every frame. A landed result also forces a repaint: the stripe is drawn over a game
        // that may not be generating input events, so without this the "SYNCING…" label could sit
        // stale until the player happened to move the mouse over the window.
        if self.economy.poll() {
            ui.ctx().request_repaint();
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
                Some(OverlayCommand::Quit) => {
                    quit_requested = true;
                    // Give up what a new instance must own (the hotkey, then the tray icon)
                    // before the name it races for. If the name went first, a relaunch could win
                    // it while `Ctrl+Shift+Z` was still registered here, fail to register it, and
                    // run with no hotkey (review iteration 3, WR-01).
                    if let Some(manager) = self.hotkey_manager.take() {
                        hotkey::release_toggle(manager);
                    }
                    self.tray.release();
                    // Release the instance name now, before this frame's show-request poll:
                    // from here on a relaunch becomes the new first instance rather than
                    // waiting on (or being acked by) a process that is about to close.
                    self.instance = None;
                }
                None => {}
            }
        }

        // INST-01 (CONTEXT "Behaviour"): a second launch is a reveal, never a toggle (same rule
        // as the tray's Show, D-17). It deliberately bypasses the hotkey's foreground gate
        // because the signal comes from outside the game and the stripe. The reveal itself goes
        // through `visibility::advance` and `win::reveal` (`SW_SHOWNOACTIVATE`, no focus steal).
        // The existing `request_repaint_after(50ms)` plus eframe's hidden-window heartbeat is
        // what wakes a stripe hidden to the tray, so no extra waker thread is needed.
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
                // An already-shown stripe is still an answer.
                guard.ack();
            }
        }

        // The frame decision: one call to the tested state machine, one
        // call to translate the transition into the single effect to
        // perform. No branching of our own.
        let focused = ui.ctx().input(|i| i.viewport().focused).unwrap_or(false);
        let next = visibility::advance(self.visibility, focused, self.toggle_requested);
        if let Some(effect) = visibility::effect_for(self.visibility, next) {
            if let Some(window) = &self.window {
                match effect {
                    visibility::Effect::Reveal => win::reveal(window),
                    visibility::Effect::Conceal => win::conceal(window),
                }
            }
        }
        // Concealing the stripe clears both status lines, so the next reveal starts clean.
        //
        // They are reports on an action the player has already seen the result of: "Wrote
        // <path>…" and "Economy prices refreshed from <league>" answer a question asked minutes
        // ago, and left standing they are indistinguishable from a report about whatever the
        // player is about to do next (author, 2026-09-11). A message that outlives its own
        // question is worse than no message.
        //
        // Conceal, not Reveal: clearing on the way out means the result stays readable for as
        // long as the player keeps the stripe open, which is the whole point of showing it.
        //
        // The one exception is a reload in flight: sending it focuses the game, which is itself
        // the click-away that hides the stripe. Its interim line is kept so the result can
        // replace it below and be read on the next reveal (review CR-01).
        if matches!(
            visibility::effect_for(self.visibility, next),
            Some(visibility::Effect::Conceal)
        ) {
            self.apply_message = apply_message_after_conceal(
                self.apply_message.take(),
                self.reload_pending.as_ref(),
            );
            self.economy.clear_message();
        }

        self.visibility = next;
        self.toggle_requested = false;

        // The reload result lands after the worker's last step; the line is only replaced if it
        // is still the interim one. Polled after the conceal above, so a result and a hide in the
        // same frame cannot clear the result it just wrote: it lands into a hidden stripe and is
        // shown on the next reveal.
        if let Some(result) = self.reload.poll() {
            if let Some(p) = self.reload_pending.take()
                && let Some(msg) = landed_reload_message(self.apply_message.as_deref(), &p, &result)
            {
                self.apply_message = Some(msg);
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

        let panel_frame = egui::Frame::central_panel(&ui.style())
            .fill(theme::PANEL_GROUND)
            .inner_margin(egui::Margin::ZERO);

        // Computed once per frame; every button below is shown/enabled strictly from it.
        self.poll_styles(ui.ctx().input(|i| i.time));
        let state = self.filter_state();
        // A changed styles.json makes the written filter stale even with no rule edited.
        let style_pending = self
            .working
            .as_mut()
            .is_some_and(|w| w.style_rewrite_pending());
        let config_dirty = self.config.as_ref().is_ok_and(|c| c.is_dirty());
        if (style_pending || config_dirty) && self.apply_state == ApplyState::Applied {
            self.apply_state = ApplyState::Unapplied;
        }
        let dirty =
            style_pending || config_dirty || self.working.as_ref().is_some_and(|w| w.is_dirty());
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
        let actions = apply::footer_actions(
            self.root.is_ok(),
            state,
            !self.changed_on_disk.is_empty(),
            dirty,
            blocking,
        );

        // Everything the panels want to change is collected here and applied once, after both have
        // rendered (deferred-action rule): row operations, the economy footer and the jump.
        let mut row_actions: Vec<RowAction> = Vec::new();
        let mut economy_action: Option<rule_list::EconomyFooterAction> = None;
        let mut jump_to: Option<BlockId> = None;
        let mut config_actions: Vec<ConfigAction> = Vec::new();
        let mut picked_tab: Option<Tab> = None;
        let rules_dirty = self.working.as_ref().is_some_and(|w| w.is_dirty());

        egui::CentralPanel::default()
            .frame(panel_frame)
            .show(ui, |ui| {
                // The header's close control hides the stripe; it is never the way the process exits
                // (that is tray -> Quit alone). widgets::header_plate does the painting; this file
                // only decides what a click on it does.
                let header = widgets::header_plate(ui, "FERRISFILTER");
                if header.close_clicked && self.visibility != Visibility::Hidden {
                    if let Some(window) = &self.window {
                        win::conceal(window);
                    }
                    self.visibility = Visibility::Hidden;
                }

                picked_tab = tab_strip(
                    ui,
                    self.tab,
                    self.tab == Tab::Config && rules_dirty,
                    self.tab == Tab::Rules && config_dirty,
                );
                ui.add_space(4.0);
                if self.tab == Tab::Config {
                    config_actions = self.config_body(ui);
                    return;
                }

                // The footer is a bottom panel, claimed BEFORE the central content is laid out, so it
                // reserves its height first and is always on screen however much the content above
                // it holds. Called after the content, the panel would find the space already taken
                // and APPLY could be pushed off the bottom edge, unreachable (author, 2026-09-10).
                egui::Panel::bottom(egui::Id::new("ff-footer"))
                    // An inner margin so the APPLY row is not flush against the window's bottom
                    // edge, where it read as clipped (screenshot poe2_print008).
                    .frame(
                        egui::Frame::NONE
                            .fill(theme::PANEL_GROUND)
                            .inner_margin(egui::Margin {
                                left: 0,
                                right: 0,
                                top: 0,
                                bottom: 8,
                            }),
                    )
                    .show(ui, |ui| {
                        // The pinned economy summary row: a locked block count, with Add/Remove
                        // staged in memory and written by the next APPLY.
                        if let Some(working) = &self.working {
                            let can_edit_economy =
                                self.root.is_ok() && self.changed_on_disk.is_empty();
                            economy_action = rule_list::economy_footer(
                                ui,
                                working.economy(),
                                &mut self.list,
                                can_edit_economy,
                            );
                        }
                        ui.add_space(8.0);
                        ui.horizontal(|ui| {
                            ui.add_space(8.0);
                            ui.label(
                                egui::RichText::new(apply::indicator(self.apply_state))
                                    .size(14.0)
                                    .color(theme::TAB_ACTIVE),
                            );
                        });
                        ui.add_space(4.0);
                        ui.horizontal_wrapped(|ui| {
                            ui.add_space(8.0);
                            if ui
                                .add_enabled(actions.add_rule, egui::Button::new("+ ADD RULE"))
                                .clicked()
                            {
                                row_actions.push(RowAction::AddRule);
                            }
                            if actions.reload
                                && ui
                                    .button("RELOAD")
                                    .on_hover_text(apply::RELOAD_TOOLTIP)
                                    .clicked()
                            {
                                self.reload();
                            }
                            // Beside RELOAD after a changed-on-disk refusal: the other way out,
                            // keeping the edits. Accent fill like CONFIG's Overwrite.
                            if actions.overwrite
                                && ui
                                    .add(egui::Button::new("OVERWRITE").fill(theme::WARM_ACCENT))
                                    .on_hover_text(apply::OVERWRITE_TOOLTIP)
                                    .clicked()
                            {
                                let egui_ctx = ui.ctx().clone();
                                self.run_apply(&egui_ctx, true);
                            }
                            // Economy sync sits beside Apply because that is the order it is used in:
                            // refresh prices, then apply them. It is deliberately NOT part of Apply --
                            // a sync is dozens of network round trips and can fail, and folding that
                            // into the action that writes the game's filter would make a routine Apply
                            // slow and failure-prone (author, 2026-09-10).
                            //
                            // No --standard/--hc/--league/--tiers switches here: the GUI syncs with the
                            // tiers file already in place, unlike the CLI.
                            let syncing = self.economy.is_running();
                            let sync_label = if syncing {
                                "SYNCING\u{2026}"
                            } else {
                                "SYNC ECONOMY"
                            };
                            if ui
                                .add_enabled(
                                    actions.sync && !syncing,
                                    egui::Button::new(sync_label),
                                )
                                .clicked()
                            {
                                // The app root, not the process working directory: the overlay can be
                                // launched from anywhere, and prices.json must land where Apply will
                                // read it from.
                                if let Ok(root) = &self.root {
                                    let base_dir = root.clone();
                                    self.economy.start(move || {
                                        crate::sync::sync_economy_blocking(&base_dir)
                                            .map_err(|e| e.to_string())
                                    });
                                }
                            }
                            if ui
                                .add_enabled(actions.revert, egui::Button::new("REVERT"))
                                .clicked()
                            {
                                self.revert();
                            }
                            let apply_button =
                                ui.add_enabled(actions.apply, egui::Button::new("APPLY"));
                            let apply_button = if blocking > 0 {
                                apply_button.on_disabled_hover_text(apply::APPLY_BLOCKED_TOOLTIP)
                            } else {
                                apply_button
                            };
                            if apply_button.clicked() {
                                let egui_ctx = ui.ctx().clone();
                                self.run_apply(&egui_ctx, false);
                            }
                        });
                        // Every message lives in one capped area so a long one can never push
                        // the buttons or the list off screen.
                        let sync_status = match self.economy.state() {
                            economy::SyncState::Idle => None,
                            economy::SyncState::Running => {
                                Some(("Fetching economy prices\u{2026}".to_string(), false))
                            }
                            economy::SyncState::Finished(outcome) => {
                                Some((outcome.message().to_string(), outcome.is_failure()))
                            }
                        };
                        let apply_line = self.apply_message.as_ref().map(|m| {
                            let ok = self.apply_state == ApplyState::Applied
                                && self.changed_on_disk.is_empty();
                            (m.clone(), !ok)
                        });
                        let gate = first_blocking.as_ref().map(|first| {
                            (
                                apply::invalid_blocks_message(
                                    blocking,
                                    first.rule_index,
                                    first.diagnostic.line,
                                ),
                                first.block,
                            )
                        });
                        let unknown = unknown_styles.first().map(|(rule, id)| {
                            (
                                apply::unknown_styles_message(unknown_styles.len(), *rule),
                                *id,
                            )
                        });
                        let lines = outcome_lines(
                            self.list_status.clone(),
                            apply_line,
                            sync_status,
                            gate,
                            unknown,
                        );
                        if let Some(target) = outcome_area(ui, "outcome", &lines) {
                            jump_to = Some(target);
                        }
                    });

                ui.add_space(8.0);
                // Status block: root, filter path and state, v0.4 notice.
                let migrated = self.filter.as_ref().map(|f| f.migrated()).unwrap_or(false);
                let mut lines: Vec<(egui::Color32, String)> = Vec::new();
                if let Err(message) = &self.root {
                    lines.push((theme::WARM_ACCENT_TEXT, message.clone()));
                }
                if let Ok(loaded) = &self.filter {
                    lines.push((theme::LABEL_TAN, loaded.path().display().to_string()));
                    let text = match (state, loaded.has_section()) {
                        (FilterState::Damaged, Err(err)) => err.to_string(),
                        _ => apply::filter_state_message(state, false),
                    };
                    let colour = if state == FilterState::Damaged {
                        theme::WARM_ACCENT_TEXT
                    } else {
                        theme::LABEL_TAN
                    };
                    lines.push((colour, text));
                }
                if self.config_changed_pending {
                    lines.push((theme::VALUE_TEXT, OUT_OF_DATE_MESSAGE.to_string()));
                }
                if migrated {
                    lines.push((
                        theme::LABEL_TAN,
                        "This is a v0.4 dynamic.filter. It will be converted on the next APPLY (a \
                         .ffbak backup of the original is kept)."
                            .to_string(),
                    ));
                }
                for (colour, text) in lines {
                    ui.horizontal(|ui| {
                        ui.add_space(8.0);
                        ui.add(egui::Label::new(egui::RichText::new(text).color(colour)).wrap());
                    });
                    ui.add_space(4.0);
                }
                ui.add_space(4.0);
                let total_rules = self.working.as_ref().map_or(0, |w| w.rows().len());
                let shown_rules = match &mut self.working {
                    Some(w) => w.matches(&self.query).len(),
                    None => 0,
                };
                ui.horizontal(|ui| {
                    ui.add_space(8.0);
                    ui.label(theme::section_header("FILTER"));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.add_space(8.0);
                        ui.label(
                            egui::RichText::new(rule_count_text(
                                total_rules,
                                shown_rules,
                                !self.query.trim().is_empty(),
                            ))
                            .color(theme::LABEL_TAN),
                        );
                    });
                });
                ui.add_space(8.0);

                match (&mut self.working, &self.filter) {
                    (Some(working), _) => {
                        ui.horizontal(|ui| {
                            ui.add_space(8.0);
                            rule_list::search_bar(ui, &mut self.query);
                        });
                        ui.add_space(8.0);
                        let visible = working.matches(&self.query);
                        if working.rows().is_empty() {
                            centered_notice(
                                ui,
                                "No rules in this filter yet",
                                "Press + ADD RULE to write your first one. Blocks you add are \
                                 saved with the filter when you press APPLY.",
                            );
                        } else if visible.is_empty() {
                            centered_notice(
                                ui,
                                &format!("No rules match \"{}\"", self.query.trim()),
                                "Check the spelling, or press Clear to see every rule.",
                            );
                        } else {
                            let frame = rule_list::show_list(
                                ui,
                                working,
                                &visible,
                                &mut self.list,
                                self.query.trim().is_empty(),
                                self.classes.as_ref().ok(),
                            );
                            row_actions.extend(frame.actions);
                        }
                    }
                    (None, Err(message)) => {
                        centered_notice(ui, "The filter could not be loaded.", message);
                        ui.horizontal(|ui| {
                            ui.add_space(8.0);
                            ui.label(
                                egui::RichText::new("Fix the file and press RELOAD.")
                                    .color(theme::LABEL_TAN),
                            );
                        });
                    }
                    (None, Ok(_)) => {}
                }
            });

        // Tab switch and CONFIG actions come first. Switching never applies or discards anything;
        // arriving on CONFIG re-checks the files for outside edits (never per frame).
        if let Some(tab) = picked_tab {
            self.tab = tab;
            if tab == Tab::Config {
                self.config_ui.disarm_all();
                self.config_outside = self
                    .config
                    .as_ref()
                    .map_or(OutsideEdit::None, |c| c.outside_by_mtime());
                self.refresh_filters();
            }
        }
        if !config_actions.is_empty() {
            let egui_ctx = ui.ctx().clone();
            self.handle_config_actions(&egui_ctx, config_actions);
        }

        // Deferred mutations: jump from the grammar line, then row operations, then the economy
        // footer. Quit stays last.
        if let (Some(target), Some(working)) = (jump_to, &self.working) {
            if !working.row_matches(target, &self.query) {
                self.query.clear();
            }
            self.list.expanded = Some(target);
            self.list.pending_scroll = Some(target);
        }
        if !row_actions.is_empty() {
            if let Some(working) = &mut self.working {
                let (changed, status) =
                    apply_row_actions(working, &mut self.list, &self.query, row_actions);
                if changed {
                    // An action can report `changed` without altering the bytes (clearing an
                    // absent name, moving a rule and back); only a real difference from the
                    // applied text makes the indicator stale.
                    if working.is_dirty() {
                        self.apply_state = ApplyState::Unapplied;
                    }
                    self.apply_message = None;
                    self.list_status = None;
                }
                if status.is_some() {
                    self.list_status = status;
                }
            }
        }
        match economy_action {
            Some(rule_list::EconomyFooterAction::Add) => self.stage_add_economy(),
            Some(rule_list::EconomyFooterAction::Remove) => self.stage_remove_economy(),
            None => {}
        }

        if quit_requested {
            // Quit is the only path that ends the process.
            ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close); // the only Close send site
        }
    }
}

#[cfg(test)]
mod tests {
    /// A stripe-sized headless context.
    fn stripe_ctx() -> egui::Context {
        let ctx = egui::Context::default();
        ctx.set_fonts(egui::FontDefinitions::empty());
        ctx
    }

    fn stripe_input() -> egui::RawInput {
        egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(460.0, 600.0),
            )),
            ..Default::default()
        }
    }

    /// Renders a long scrolling list plus a footer, in the two possible orders, and reports where
    /// the footer landed. `footer_first` is the bottom-panel arrangement `ui()` uses.
    fn footer_bottom_edge(footer_first: bool) -> (f32, f32) {
        let ctx = stripe_ctx();
        let mut footer_rect = egui::Rect::NOTHING;
        let mut screen_bottom = 0.0_f32;

        ctx.run_ui(stripe_input(), |ui| {
            // The viewport we handed in via RawInput.screen_rect.
            screen_bottom = ui.max_rect().max.y;
            egui::CentralPanel::default().show(ui, |ui| {
                let list = |ui: &mut egui::Ui| {
                    egui::ScrollArea::vertical()
                        .auto_shrink([false, true])
                        .show(ui, |ui| {
                            // Far more rows than fit. Allocated geometry rather than labels: the
                            // headless context has no fonts, so text would measure zero-height
                            // and the list would never grow.
                            for _ in 0..60 {
                                ui.allocate_space(egui::vec2(400.0, 30.0));
                            }
                        });
                };
                if footer_first {
                    egui::Panel::bottom(egui::Id::new("test-footer")).show(ui, |ui| {
                        footer_rect = ui.button("APPLY").rect;
                    });
                    list(ui);
                } else {
                    list(ui);
                    footer_rect = ui.button("APPLY").rect;
                }
            });
        })
        .drop_without_applying_deltas();

        (footer_rect.max.y, screen_bottom)
    }

    /// APPLY must stay on screen however much content the stripe holds.
    ///
    /// The second assertion gives the first its meaning: it shows the old arrangement (footer
    /// laid out after an `auto_shrink([false, true])` `ScrollArea`) really does push the button
    /// off-screen with this much content, so the test would have failed before the fix.
    #[test]
    fn the_apply_button_stays_on_screen_however_long_the_rule_list_is() {
        let (footer_bottom, screen_bottom) = footer_bottom_edge(true);
        assert!(
            footer_bottom <= screen_bottom,
            "APPLY is below the stripe's bottom edge ({footer_bottom} > {screen_bottom}) -- \
             it cannot be clicked at any scroll position"
        );

        let (old_bottom, screen_bottom) = footer_bottom_edge(false);
        assert!(
            old_bottom > screen_bottom,
            "the pre-fix arrangement was expected to push APPLY off-screen, but it did not \
             ({old_bottom} <= {screen_bottom}) -- this test is no longer exercising the defect"
        );
    }

    #[test]
    fn outcome_lines_order_and_jump_targets() {
        use crate::overlay::app::outcome_lines;
        let a = WorkingFilter::from_text(
            "Show
",
        )
        .rows()[0]
            .id;
        let lines = outcome_lines(
            Some("list".into()),
            Some(("apply".into(), false)),
            Some(("sync".into(), true)),
            Some(("gate".into(), a)),
            Some(("unknown".into(), a)),
        );
        let texts: Vec<&str> = lines.iter().map(|l| l.text.as_str()).collect();
        assert_eq!(texts, ["list", "apply", "sync", "gate", "unknown"]);
        assert!(lines[3].jump.is_some() && lines[4].jump.is_some());
        assert!(lines[0].jump.is_none() && !lines[0].accent && lines[2].accent);
        assert!(outcome_lines(None, None, None, None, None).is_empty());
    }

    /// Long messages scroll inside the capped outcome area; APPLY stays on screen below it.
    #[test]
    fn outcome_area_is_capped_and_keeps_apply_on_screen() {
        use crate::overlay::app::{OUTCOME_MAX_HEIGHT, OutcomeLine, outcome_area};
        let long = "x ".repeat(95);
        let lines: Vec<OutcomeLine> = (0..6)
            .map(|_| OutcomeLine {
                text: long.clone(),
                accent: true,
                jump: None,
            })
            .collect();
        let ctx = egui::Context::default();
        let mut area_h = 0.0_f32;
        let mut apply_bottom = 0.0_f32;
        let mut screen_bottom = 0.0_f32;
        for _ in 0..4 {
            ctx.run_ui(stripe_input(), |ui| {
                screen_bottom = ui.max_rect().max.y;
                egui::CentralPanel::default().show(ui, |ui| {
                    egui::Panel::bottom(egui::Id::new("f")).show(ui, |ui| {
                        let top = ui.cursor().min.y;
                        outcome_area(ui, "outcome", &lines);
                        area_h = ui.cursor().min.y - top;
                        apply_bottom = ui.button("APPLY").rect.max.y;
                    });
                });
            })
            .drop_without_applying_deltas();
        }
        assert!(area_h > 0.0, "outcome area drew nothing");
        assert!(area_h <= OUTCOME_MAX_HEIGHT + 8.0, "area {area_h}");
        assert!(
            apply_bottom <= screen_bottom,
            "{apply_bottom} {screen_bottom} {area_h}"
        );
    }

    /// The CONFIG footer is pinned in its own bottom panel: Apply stays on screen however long the
    /// styles list above it grows.
    #[test]
    fn the_config_footer_keeps_apply_on_screen_with_a_long_styles_list() {
        use crate::overlay::app::{OutcomeLine, outcome_area};
        use crate::overlay::config_state::{ConfigState, OutsideEdit};
        use crate::overlay::config_view::{self, ConfigUi, ConfigViewCtx};
        use std::collections::{BTreeSet, HashMap};

        let state = ConfigState::load(std::path::Path::new(env!("CARGO_MANIFEST_DIR"))).unwrap();
        let users = HashMap::new();
        let refs = BTreeSet::new();
        let ctx = ConfigViewCtx {
            filters: None,
            active_filter: "dynamic.filter",
            game_dir: None,
            pending_switch: None,
            rule_users: &users,
            config_refs: &refs,
            outside: OutsideEdit::None,
            out_of_date: false,
            reload_after_apply: true,
        };
        let mut cfg = ConfigUi::default();
        let long = "x ".repeat(95);
        let lines: Vec<OutcomeLine> = (0..6)
            .map(|_| OutcomeLine {
                text: long.clone(),
                accent: true,
                jump: None,
            })
            .collect();
        let rctx = egui::Context::default();
        let mut footer_bottom = 0.0_f32;
        let mut screen_bottom = 0.0_f32;
        for _ in 0..4 {
            rctx.run_ui(stripe_input(), |ui| {
                screen_bottom = ui.max_rect().max.y;
                egui::CentralPanel::default().show(ui, |ui| {
                    egui::Panel::bottom(egui::Id::new("ff-config-footer")).show(ui, |ui| {
                        outcome_area(ui, "config-outcome", &lines);
                        config_view::show_footer(ui, &state, &mut cfg, &ctx);
                        footer_bottom = ui.min_rect().max.y;
                    });
                    egui::ScrollArea::vertical()
                        .auto_shrink([false, false])
                        .show(ui, |ui| {
                            for _ in 0..80 {
                                ui.allocate_space(egui::vec2(400.0, 30.0));
                            }
                        });
                });
            })
            .drop_without_applying_deltas();
        }
        assert!(footer_bottom > 0.0, "footer drew nothing");
        assert!(
            footer_bottom <= screen_bottom,
            "{footer_bottom} > {screen_bottom}"
        );
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
            "Cannot resolve the default PoE2 config folder (USERPROFILE is not set)."
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
        }
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
        // A reload in flight with its interim line showing: the focus change hides the stripe,
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
            // Apply sets the interim line; focusing the game conceals the stripe.
            let after_hide = apply_message_after_conceal(Some("interim".into()), Some(&p));
            // The worker reports while the stripe is hidden.
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

    #[test]
    fn rule_count_copy() {
        assert_eq!(super::rule_count_text(142, 142, false), "142 rules");
        assert_eq!(super::rule_count_text(1, 1, false), "1 rule");
        assert_eq!(super::rule_count_text(0, 0, false), "0 rules");
        assert_eq!(super::rule_count_text(142, 7, true), "7 of 142 rules");
    }

    /// EDIT-07 at stripe scale: a 511-block filter in the real list keeps the footer on screen
    /// and draws only a bounded window of rows.
    #[test]
    fn a_511_block_filter_keeps_the_footer_on_screen_and_draws_a_bounded_window() {
        use crate::overlay::rule_list::{self, ListUi};
        use crate::overlay::rules::WorkingFilter;

        let bytes = include_bytes!("../../tests/fixtures/parser/filterblade.filter");
        let text = String::from_utf8_lossy(bytes).into_owned();
        let mut working = WorkingFilter::from_text(&text);
        assert!(
            working.rows().len() > 400,
            "fixture has a few hundred rules"
        );
        let visible = working.matches("");
        let mut list = ListUi::default();

        let ctx = stripe_ctx();
        let mut input = stripe_input();
        input.screen_rect = Some(egui::Rect::from_min_size(
            egui::Pos2::ZERO,
            egui::vec2(460.0, 1000.0),
        ));
        let mut footer_bottom = 0.0_f32;
        let mut screen_bottom = 0.0_f32;
        let mut drawn = 0usize;
        ctx.run_ui(input, |ui| {
            screen_bottom = ui.max_rect().max.y;
            egui::CentralPanel::default().show(ui, |ui| {
                egui::Panel::bottom(egui::Id::new("test-footer")).show(ui, |ui| {
                    footer_bottom = ui.button("APPLY").rect.max.y;
                });
                drawn = rule_list::show_list(ui, &working, &visible, &mut list, true, None).drawn;
            });
        })
        .drop_without_applying_deltas();

        assert!(
            footer_bottom <= screen_bottom,
            "{footer_bottom} > {screen_bottom}"
        );
        assert!(drawn > 0, "something must be drawn");
        assert!(
            drawn < 80,
            "virtualization drew {drawn} of {} rows",
            working.rows().len()
        );
    }

    // ---- controller helpers (Phase 19-06) ----

    use super::{AllOutcome, apply_everything, apply_row_actions};
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
        let r = apply_everything(root, None, Some((loaded, working)), overwrite);
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

    /// A temp app root holding copies of the committed styles.json and tiers.json.
    fn config_root() -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        for f in ["styles.json", "tiers.json"] {
            std::fs::copy(
                std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(f),
                dir.path().join(f),
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
        let root = config_root();
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
    fn no_filter_apply_writes_config_only() {
        let root = config_root();
        let mut config = ConfigState::load(root.path()).unwrap();
        assert!(config.edit_style("S", StyleEdit::Font(Some(41))));
        let tiers = std::fs::read(root.path().join("tiers.json")).unwrap();
        let r = apply_everything(root.path(), Some(&mut config), None, false);
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
        working.stage_add_economy(crate::overlay::rules::count_blocks(&body));
        let (outcome, _, _) = apply_working(&root, &mut loaded, &mut working);
        assert!(matches!(outcome, ApplyOutcome::Written(_)), "{outcome:?}");
        let written = std::fs::read_to_string(&path).unwrap();
        assert!(crate::filter::section::has_section(&written).unwrap());
    }

    #[test]
    fn adding_a_rule_the_search_hides_says_so_and_does_not_expand_it() {
        let mut working = WorkingFilter::from_text(RULES);
        let mut list = ListUi::default();
        let (changed, status) =
            apply_row_actions(&mut working, &mut list, "zzz", vec![RowAction::AddRule]);
        assert!(changed);
        assert_eq!(
            status.as_deref(),
            Some("Rule added at position 3 (hidden by the current search)")
        );
        assert_eq!(list.expanded, None);
        assert_eq!(list.pending_scroll, None);
    }

    #[test]
    fn deleting_the_expanded_row_collapses_it() {
        let mut working = WorkingFilter::from_text(RULES);
        let mut list = ListUi::default();
        let id = working.rows()[0].id;
        list.expanded = Some(id);
        let (changed, _) =
            apply_row_actions(&mut working, &mut list, "", vec![RowAction::Delete(id)]);
        assert!(changed);
        assert!(list.expanded.is_none());
    }

    // ---- styles on Apply (Phase 20-06) ----

    const S_RULE: &str = "#ff:style S\nHide\n  Class \"Rings\"\n  SetFontSize 20\n";

    fn styles_root() -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        std::fs::copy(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("styles.json"),
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
                m.starts_with("styles.json could not be read, so styled rules"),
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
        let (changed, status) =
            apply_row_actions(&mut working, &mut list, "", vec![ed("71"), ed("72")]);
        assert!(changed);
        assert_eq!(status, None, "the stale first edit must not run");
        assert_eq!(working.text(), "Show\n  ItemLevel >= 72\n");
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
        assert!(apply_row_actions(&mut working, &mut list, "", vec![ed]).0);
        assert_eq!(list.epoch, epoch);
        let rm = RowAction::RemoveLine {
            id,
            line: 1,
            expect: "  BaseType \"A\"".into(),
        };
        assert!(apply_row_actions(&mut working, &mut list, "", vec![rm]).0);
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
        assert!(apply_row_actions(&mut working, &mut list, "", vec![same]).0);
        assert_eq!(list.epoch, epoch, "same keyword, ordinals unchanged");
        let fix = whole(1, "  BaseTyp \"A\"", "BaseType \"A\"");
        assert!(apply_row_actions(&mut working, &mut list, "", vec![fix]).0);
        assert_ne!(list.epoch, epoch, "a changed keyword shifts ordinals");
        let epoch = list.epoch;
        let add = RowAction::AddLine {
            id,
            kind: crate::overlay::rules::LineKind::Condition,
            text: "ItemLevel >= 70".into(),
        };
        assert!(apply_row_actions(&mut working, &mut list, "", vec![add]).0);
        assert_ne!(list.epoch, epoch, "an added line can shift ordinals");
    }
}
