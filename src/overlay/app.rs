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
//! file and discards unapplied edits.
//!
//! The decisions (which button is live, what the status says) are pure functions in
//! [`super::apply`]; this file only draws them and performs the calls.
//! `PITFALLS.md` §8: if a bug can only be found by clicking, it belongs in a library module.

use std::time::Duration;

use global_hotkey::GlobalHotKeyManager;

use crate::compile::{InputPaths, PriceSource, PriceSourcePolicy};
use crate::dynamic::{self, EconomyOp, LoadedFilter};
use crate::filter::{BlockId, LinePatch};
use crate::loader;
use crate::models::config::StyleSet;
use crate::safe_write::WriteOutcome;

use super::apply::{self, ApplyOutcome, ApplyState, FilterState};
use super::classes::ClassCatalog;
use super::economy;
use super::hotkey::HotkeyStatus;
use super::layout;
use super::rule_list::{self, ListUi};
use super::rules::{self, EconomyPlan, EconomyView, RowAction, WorkingFilter};
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

    /// The tray icon, menu, and id->command mapping. Kept alive here for the
    /// process lifetime -- dropping it would remove the icon from the tray.
    tray: TrayHandles,

    /// The global hotkey manager. Never read after construction -- held
    /// purely so its `Drop` impl unregisters the hotkey when the app ends.
    #[allow(dead_code)]
    hotkey_manager: Option<GlobalHotKeyManager>,

    /// Registration outcome for `Ctrl+Shift+Z`. Read every frame to filter incoming
    /// `GlobalHotKeyEvent`s to the one combo this app registered.
    hotkey_status: HotkeyStatus,

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

    /// Set when Apply was refused because the file changed on disk; only RELOAD clears it.
    needs_reload: bool,

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
}

/// Seconds between `styles.json` modification-time polls while the stripe runs.
const STYLES_POLL_SECS: f64 = 2.0;

fn load_styles_at(root: &std::path::Path) -> Result<StyleSet, String> {
    let path = InputPaths::relative_to(root).styles;
    loader::load_styles(&path.to_string_lossy()).map_err(|e| e.to_string())
}

fn load_classes_at(root: &std::path::Path) -> Result<ClassCatalog, String> {
    let paths = InputPaths::relative_to(root);
    loader::load_config(
        &paths.config.to_string_lossy(),
        &paths.styles.to_string_lossy(),
    )
    .map(|config| ClassCatalog::from_config(&config))
    .map_err(|e| e.to_string())
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

/// The one Apply path over the edited document: price the economy section when the plan needs it,
/// then write through [`dynamic::apply_edited`] (grammar gate, guarded atomic write). Returns the
/// outcome, sentences to append to the success message, and whether only RELOAD can recover.
fn apply_working(
    root: &std::path::Path,
    loaded: &mut LoadedFilter,
    working: &mut WorkingFilter,
) -> (ApplyOutcome, String, bool) {
    let mut extras = String::new();
    // Styled rules are rewritten on a clone; a failure here happens before anything is written
    // and leaves the user's edits exactly as they were.
    let (text, style_report) = match working.text_with_styles() {
        Ok(pair) => pair,
        Err(message) => return (ApplyOutcome::Failed(message), extras, false),
    };
    let plan = working.economy_plan();
    let body = match plan {
        EconomyPlan::Refresh | EconomyPlan::Add => {
            match dynamic::economy_body(&InputPaths::relative_to(root), PriceSourcePolicy::Auto) {
                Ok((body, source)) => {
                    if matches!(source, PriceSource::Fallback(_)) {
                        extras.push_str(" Used the fallback prices.");
                    }
                    Some(body)
                }
                Err(err) => return (ApplyOutcome::Failed(err.to_string()), extras, false),
            }
        }
        EconomyPlan::Keep | EconomyPlan::Remove => None,
    };
    let op = match (plan, body.as_deref()) {
        (EconomyPlan::Refresh, Some(b)) => EconomyOp::Refresh(b),
        (EconomyPlan::Add, Some(b)) => EconomyOp::Add(b),
        (EconomyPlan::Remove, _) => EconomyOp::Remove,
        _ => EconomyOp::Keep,
    };
    match dynamic::apply_edited(loaded, &text, op) {
        Ok(report) => {
            working.on_applied(loaded.text());
            extras.push_str(&apply::styled_rules_message(style_report.rewritten));
            if let Some((first_rule, _)) = style_report.unknown.first() {
                extras.push(' ');
                extras.push_str(&apply::unknown_styles_message(
                    style_report.unknown.len(),
                    *first_rule,
                ));
            }
            if let Some((first_rule, err)) = style_report.failed.first() {
                extras.push(' ');
                extras.push_str(&apply::failed_styles_message(
                    style_report.failed.len(),
                    *first_rule,
                    err,
                ));
            }
            if report.migrated {
                extras.push_str(" Converted from v0.4.");
            }
            let outcome = match report.outcome {
                WriteOutcome::Unchanged => ApplyOutcome::Unchanged(report.path),
                WriteOutcome::Written { backup } => {
                    if let Some(backup) = backup {
                        extras.push_str(&format!(" Kept the original as {}.", backup.display()));
                    }
                    ApplyOutcome::Written(report.path)
                }
            };
            (outcome, extras, false)
        }
        Err(dynamic::DynamicError::InvalidBlocks {
            count,
            first_rule,
            first_line,
            ..
        }) => (
            ApplyOutcome::Failed(apply::invalid_blocks_message(count, first_rule, first_line)),
            extras,
            false,
        ),
        Err(err) if apply::is_changed_on_disk(&err) => (
            ApplyOutcome::Failed(apply::CHANGED_ON_DISK_MESSAGE.to_string()),
            extras,
            true,
        ),
        Err(err) => (ApplyOutcome::Failed(err.to_string()), extras, false),
    }
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

impl OverlayApp {
    /// Builds the app state around a tray and hotkey already constructed on
    /// the main thread (thread affinity -- see `overlay::launch`).
    pub fn new(
        tray: TrayHandles,
        hotkey_manager: Option<GlobalHotKeyManager>,
        hotkey_status: HotkeyStatus,
    ) -> Self {
        let root = crate::paths::app_root().map_err(|e| e.to_string());
        // Loading never writes (D-10).
        let filter = Self::load_filter();
        let working = working_from(&filter);

        let mut app = Self {
            docked: false,
            tray,
            hotkey_manager,
            hotkey_status,
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
            needs_reload: false,
            apply_state: ApplyState::NeverApplied,
            apply_message: None,
            economy: economy::EconomySync::new(),
            working,
            list: ListUi::default(),
            query: String::new(),
            list_status: None,
            applied_this_session: false,
            classes: Err("not loaded".to_string()),
            styles_mtime: None,
            styles_checked: 0.0,
        };
        app.refresh_inputs();
        app
    }

    /// The class catalogue, when `config.json` loaded.
    #[allow(dead_code)]
    pub(crate) fn classes(&self) -> Option<&ClassCatalog> {
        self.classes.as_ref().ok()
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
                self.classes = load_classes_at(&root);
                if let Err(err) = &self.classes {
                    self.list_status =
                        Some(format!("Class groups unavailable (config.json): {err}"));
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

    /// Reads the default game-folder filter. Never writes.
    fn load_filter() -> Result<LoadedFilter, String> {
        match dynamic::default_filter_path() {
            Some(path) => LoadedFilter::load(&path).map_err(|e| e.to_string()),
            None => Err(
                "Cannot resolve the default PoE2 config folder (USERPROFILE is not set)."
                    .to_string(),
            ),
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

    /// The one Apply path: write the edited document (and any staged economy op) through
    /// [`apply_working`], then re-key the list's view state to the re-parsed document.
    fn run_apply(&mut self) {
        let (Ok(root), Ok(loaded), Some(working)) =
            (&self.root, &mut self.filter, &mut self.working)
        else {
            return;
        };
        // Ids reset on re-parse, so remember the expanded row by position instead.
        let expanded_index = self
            .list
            .expanded
            .and_then(|id| working.row(id))
            .map(|r| r.index);
        // The styles are re-read at every Apply, so a styles.json edited a moment ago counts.
        self.styles_mtime = styles_mtime_at(root);
        working.set_styles(load_styles_at(root));
        let (outcome, extras, needs_reload) = apply_working(root, loaded, working);
        match outcome {
            ApplyOutcome::Written(path) => {
                self.apply_message = Some(format!("{}{extras}", apply::success_message(&path)));
                self.apply_state = self.apply_state.on_apply_success();
            }
            ApplyOutcome::Unchanged(path) => {
                self.apply_message = Some(format!("{}{extras}", apply::unchanged_message(&path)));
                self.apply_state = self.apply_state.on_apply_success();
            }
            ApplyOutcome::Failed(message) => {
                self.apply_message = Some(message);
                self.apply_state = self.apply_state.on_apply_failure();
                if needs_reload {
                    self.needs_reload = true;
                }
                return;
            }
        }
        self.needs_reload = false;
        self.applied_this_session = true;
        self.list_status = None;
        self.list.clear_ids();
        self.list.expanded = expanded_index.and_then(|i| working.rows().get(i - 1).map(|r| r.id));
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
        self.root = crate::paths::app_root().map_err(|e| e.to_string());
        self.filter = Self::load_filter();
        self.working = working_from(&self.filter);
        self.list.clear_ids();
        self.query.clear();
        self.list_status = None;
        self.needs_reload = false;
        self.apply_message = None;
        self.applied_this_session = false;
        self.apply_state = ApplyState::NeverApplied;
        self.refresh_inputs();
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
#[cfg(test)]
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
pub(crate) fn outcome_area(ui: &mut egui::Ui, lines: &[OutcomeLine]) -> Option<BlockId> {
    if lines.is_empty() {
        return None;
    }
    let mut jump = None;
    egui::ScrollArea::vertical()
        .id_salt("outcome")
        .max_height(96.0)
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
                }
                None => {}
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
        if matches!(
            visibility::effect_for(self.visibility, next),
            Some(visibility::Effect::Conceal)
        ) {
            self.apply_message = None;
            self.economy.clear_message();
        }

        self.visibility = next;
        self.toggle_requested = false;

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
        if style_pending && self.apply_state == ApplyState::Applied {
            self.apply_state = ApplyState::Unapplied;
        }
        let dirty = style_pending || self.working.as_ref().is_some_and(|w| w.is_dirty());
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
        let actions =
            apply::footer_actions(self.root.is_ok(), state, self.needs_reload, dirty, blocking);

        // Everything the panels want to change is collected here and applied once, after both have
        // rendered (deferred-action rule): row operations, the economy footer and the jump.
        let mut row_actions: Vec<RowAction> = Vec::new();
        let mut economy_action: Option<rule_list::EconomyFooterAction> = None;
        let mut jump_to: Option<BlockId> = None;

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
                            let can_edit_economy = self.root.is_ok() && !self.needs_reload;
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
                            if actions.reload && ui.button("RELOAD").clicked() {
                                self.reload();
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
                                self.run_apply();
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
                            let ok = self.apply_state == ApplyState::Applied && !self.needs_reload;
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
                        if let Some(target) = outcome_area(ui, &lines) {
                            jump_to = Some(target);
                        }
                    });

                ui.add_space(8.0);
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
                        outcome_area(ui, &lines);
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

    use super::{apply_row_actions, apply_working};
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

    #[test]
    fn apply_working_writes_edits_then_reports_unchanged() {
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
        let (again, _, _) = apply_working(dir.path(), &mut loaded, &mut working);
        assert!(matches!(again, ApplyOutcome::Unchanged(_)), "{again:?}");
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
            ApplyOutcome::Failed(apply::CHANGED_ON_DISK_MESSAGE.to_string())
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
        assert!(matches!(again, ApplyOutcome::Unchanged(_)), "{again:?}");
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
    fn apply_working_style_unknown_is_reported_and_untouched() {
        let root = styles_root();
        let text = "Show\n  Class \"Bows\"\n\n#ff:style Nope\nHide\n  SetFontSize 20\n";
        let (path, mut loaded) = loaded_in(&root, text);
        let mut working = working_with_styles(root.path(), loaded.text());
        let (outcome, extras, _) = apply_working(root.path(), &mut loaded, &mut working);
        assert_eq!(outcome, ApplyOutcome::Unchanged(path.clone()), "{extras}");
        assert!(
            extras.contains("1 rule uses an unknown style (first: rule 2)."),
            "{extras}"
        );
        assert_eq!(std::fs::read_to_string(&path).unwrap(), text);
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
}
