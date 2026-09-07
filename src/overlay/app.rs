//! `OverlayApp` — the `eframe::App` implementation for the stripe window.
//!
//! What this file owns: docking the stripe to the right screen edge on the first
//! frame where a monitor size is known (D-12); polling the tray and the global
//! hotkey for their events (plan 13-02); turning those events into an actual
//! show/hide of the window that does not steal keyboard focus from the game, by
//! delegating the decision to [`super::visibility`] and the OS call to
//! [`super::win`] (plan 13-03); and — starting with plan 14-02 — the editable
//! accordion: [`super::edit::WorkingRules`] loaded once at startup via
//! [`super::store`], rendered in the PoE2-idiom chrome from [`super::theme`] and
//! [`super::widgets`], with expansion state keyed by [`super::edit::RuleId`]
//! (D-19) so a reorder never leaves the accordion pointed at the wrong rule.
//! This file itself contains no rule-mutation logic and no palette values of
//! its own — every mutation is a call into [`super::edit`], and every
//! summarizing string comes from [`super::view_model::rule_rows`], recomputed
//! from the current [`super::edit::WorkingRules`] each frame (cheap at this
//! file's scale — a handful of rules) rather than cached and hand-kept in
//! sync. `PITFALLS.md` §8: if a bug can only be found by clicking, it belongs
//! in one of those library modules, not here.
//!
//! Plan 14-04 adds Apply: a button below the rule list that is the second caller of Phase 12's
//! dynamic-module compile-and-write pair (D-15, both unchanged by this plan — grep the diff),
//! reading whatever autosave most recently wrote to `self.store_path`. [`super::apply`] holds the
//! pure `ApplyState`/`ApplyOutcome`/message types (D-18); this file only decides *when* to call
//! into the dynamic module and how to fold its `Result`s into that state — the same "library
//! owns the logic, this file owns the calling" split every other control in this file already
//! follows.

use std::time::Duration;

use global_hotkey::GlobalHotKeyManager;

use crate::compile::InputPaths;
use crate::dynamic;
use crate::models::config::Config;

use super::apply::{self, ApplyOutcome, ApplyState};
use super::edit::{RuleId, WorkingRules};
use super::hotkey::HotkeyStatus;
use super::layout;
use super::rule_editor;
use super::store;
use super::theme;
use super::tray::{self, OverlayCommand, TrayHandles};
use super::view_model;
use super::visibility::{self, Visibility};
use super::widgets;
use super::win::{self, OverlayWindow};

/// The eframe application state for the overlay stripe.
pub struct OverlayApp {
    /// Latches `true` once the first-frame docking has run, so it never runs
    /// again (position/width are not persisted or re-derived per frame — D-15).
    docked: bool,

    /// The tray icon, menu, and id->command mapping. Kept alive here for the
    /// process lifetime — dropping it would remove the icon from the tray.
    tray: TrayHandles,

    /// The global hotkey manager. Never read after construction — held
    /// purely so its `Drop` impl unregisters the hotkey when the app ends.
    #[allow(dead_code)]
    hotkey_manager: Option<GlobalHotKeyManager>,

    /// Registration outcome for `Ctrl+Shift+Z` (D-09, D-10). Read every frame to
    /// filter incoming `GlobalHotKeyEvent`s to the one combo this app
    /// registered.
    hotkey_status: HotkeyStatus,

    /// Set by the tray's left click, a `Ctrl+Shift+Z` press, or a tray "Show"
    /// while currently hidden. Consumed once per frame by
    /// [`visibility::advance`].
    toggle_requested: bool,

    /// The show/hide decision (D-05, D-06) — see `super::visibility` for why
    /// this is three states, not two.
    visibility: Visibility,

    /// The stripe's own cached window handle (see `super::win`), populated
    /// on the first frame one is available. `None` on the very first few
    /// frames, and always `None` off Windows.
    window: Option<OverlayWindow>,

    /// The editable working model (D-18/D-19), loaded once at startup via [`store::load`] and
    /// built with [`WorkingRules::from_rules`]. `Err` holds the rendered message a malformed or
    /// missing `dynamic.json` produced — the stripe shows this text in place of the list rather
    /// than crashing (T-13-13). Every mutation (add/delete/move/toggle/edit-a-condition) is a
    /// call into `super::edit`'s pure functions; this file only decides *when* to call them.
    rules: Result<WorkingRules, String>,

    /// The rest of the loaded store — everything in `Config` except `rules`, which
    /// [`Self::rules`] owns. Autosave (Task 3) writes this back with `rules` replaced by
    /// [`WorkingRules::to_rules`]; plan 14-03's condition pickers read their option lists
    /// (styles, class groups) from here. `None` exactly when [`Self::rules`] is `Err` — the
    /// store either loaded as a whole or it didn't.
    config: Option<Config>,

    /// Where [`Self::config`]/[`Self::rules`] were loaded from, and where autosave (Task 3)
    /// writes back to.
    store_path: std::path::PathBuf,

    /// Which rule's accordion body is open, or `None` if the list is fully collapsed. Keyed by
    /// `RuleId`, never an index or array position (D-19, `PITFALLS.md` §9) — a reorder swaps
    /// `WorkingRules`' array positions but never invalidates a `RuleId`, so this field is never
    /// re-resolved after a move.
    expanded: Option<RuleId>,

    /// The rule a delete click is asking to confirm (D-08): while `Some(id)`, that row's normal
    /// controls (arrows/toggle/delete) are replaced with `Delete? [Yes] [Cancel]` instead of a
    /// second window or modal. Opening a confirm on a different row silently replaces the
    /// pending one — only one confirm can be live at a time, by construction (a single
    /// `Option`, not a per-row bool).
    confirm_delete: Option<RuleId>,

    /// Set by any row control that mutates `rules` (move/toggle/delete/add) or, in plan 14-03,
    /// an edited condition. Consumed exactly once per frame by the coalesced autosave (D-02,
    /// RESEARCH Pattern 5) — never per keystroke, never on a timer.
    dirty: bool,

    /// The most recent autosave failure, if any, rendered in the stripe rather than printed
    /// (T-14-01: a write failure must produce a message the player can see, not take the
    /// process down or vanish into a console the GUI subsystem has none of). Plan 14-04 owns
    /// the wider error/status surface this eventually joins.
    last_save_error: Option<String>,

    /// Whether `dynamic.filter` currently matches the saved `dynamic.json`, as far as this
    /// session knows (D-03). Every frame that sets [`Self::dirty`] also moves this to
    /// [`ApplyState::Unapplied`] — see the single consuming block near the end of [`ui`](
    /// eframe::App::ui) that both flushes the autosave and makes that transition, so the two can
    /// never drift apart. A successful Apply moves it to [`ApplyState::Applied`]; a failed one
    /// leaves it exactly as it was (D-17).
    apply_state: ApplyState,

    /// The most recent Apply outcome's rendered text — [`apply::success_message`] on success
    /// (D-16's reload instruction), or a `DynamicError`'s display string on failure (D-17).
    /// Cleared on the next edit alongside [`Self::apply_state`], so a stale "Wrote ..." message
    /// never lingers once the stripe already says the filter is out of date again.
    apply_message: Option<String>,
}

impl OverlayApp {
    /// Builds the app state around a tray and hotkey already constructed on
    /// the main thread (thread affinity — see `overlay::launch`).
    pub fn new(
        tray: TrayHandles,
        hotkey_manager: Option<GlobalHotKeyManager>,
        hotkey_status: HotkeyStatus,
    ) -> Self {
        // Loaded once at startup (D-20: nothing in this phase reloads the store mid-session --
        // every subsequent change to it flows the other direction, through autosave). Seeding
        // an absent store is `store::load`'s own job (Phase 12's `ensure_store`), not this
        // constructor's.
        let (base, store_path) = store::default_paths();
        let (config, rules) = match store::load(&base, &store_path) {
            Ok(view) => {
                let rule_list = view.config.rules.clone().unwrap_or_default();
                (Some(view.config), Ok(WorkingRules::from_rules(rule_list)))
            }
            Err(message) => (None, Err(message)),
        };

        Self {
            docked: false,
            tray,
            hotkey_manager,
            hotkey_status,
            toggle_requested: false,
            // The window created in `overlay::launch` starts visible
            // (`with_active(false)` only suppresses activation, not
            // visibility) and has never been focused — `ShownUnfocused` is
            // the state that matches that physical reality. Starting here
            // (rather than `Hidden`) is what makes the very first `Ctrl+Shift+Z`
            // press after launch hide the stripe, matching the checkpoint
            // script's step 3.
            visibility: Visibility::ShownUnfocused,
            window: None,
            rules,
            config,
            store_path,
            expanded: None,
            confirm_delete: None,
            dirty: false,
            last_save_error: None,
            apply_state: ApplyState::NeverApplied,
            apply_message: None,
        }
    }
}

impl eframe::App for OverlayApp {
    fn ui(&mut self, ui: &mut egui::Ui, frame: &mut eframe::Frame) {
        if !self.docked {
            let monitor_size = ui.ctx().input(|i| i.viewport().monitor_size);
            if let Some(monitor_size) = monitor_size {
                let rect = layout::dock_rect(monitor_size, layout::STRIPE_WIDTH);
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
                    self.toggle_requested = true;
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

        // Poll the tray menu's Show/Quit events. Quit itself is only *requested* here — the
        // actual close-the-viewport send is deferred to after the body renders and the autosave
        // runs (RESEARCH Risk 4 / T-14-06): D-14 still makes Quit the only path that ends the
        // process, but the *order* within this frame is what guarantees the last edit is not
        // lost — see the comment at the deferred send site near the end of this function.
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

        egui::CentralPanel::default().frame(panel_frame).show(ui, |ui| {
            // D-14: the header's close control hides the stripe; it is never the way the
            // process exits (that is tray -> Quit alone). widgets::header_plate does the
            // painting; this file only decides what a click on it does.
            let header = widgets::header_plate(ui, "FERRISFILTER");
            if header.close_clicked && self.visibility != Visibility::Hidden {
                if let Some(window) = &self.window {
                    win::conceal(window);
                }
                self.visibility = Visibility::Hidden;
            }

            ui.add_space(10.0);
            ui.horizontal(|ui| {
                ui.add_space(10.0);
                ui.label(theme::section_header("RULES"));
            });
            ui.add_space(6.0);

            // D-01: an accordion, not a detail pane -- clicking a row expands its controls in
            // place, rows above/below stay visible, and the stripe never resizes (controls
            // stack vertically inside the row instead). D-19: expansion is keyed by `RuleId`,
            // never an index or array position, so an up/down move (Task 3) can never leave the
            // accordion pointed at the wrong rule. Content fills only the top of the stripe; the
            // remainder stays empty panel ground rather than shrinking the window (D-23) -- the
            // ScrollArea below only grows as tall as its rows, it does not stretch to fill the
            // CentralPanel.
            if let Some(message) = &self.last_save_error {
                ui.horizontal_wrapped(|ui| {
                    ui.add_space(10.0);
                    ui.colored_label(theme::WARM_ACCENT, message);
                });
                ui.add_space(4.0);
            }

            match &mut self.rules {
                Ok(working) => {
                    // Summaries are recomputed from the live working model every frame, reusing
                    // `view_model::rule_rows`'s existing summary derivation (the same one-line
                    // collapsed text 13-04 rendered) rather than duplicating that logic here.
                    // `config.rules` is kept in sync with `working` first so the two can never
                    // drift once a control below mutates it.
                    if let Some(config) = self.config.as_mut() {
                        config.rules = Some(working.to_rules());
                    }
                    let display_rows = self
                        .config
                        .as_ref()
                        .map(view_model::rule_rows)
                        .unwrap_or_default();
                    // Held across the whole row loop so plan 14-03's condition editors can read
                    // the style/class option lists (`edit::available_styles`/`class_groups`) --
                    // a disjoint borrow of `self.config`, independent of `self.rules` (`working`,
                    // below), exactly like `self.config.as_mut()` two lines above already is.
                    let config_ref = self.config.as_ref();

                    // Snapshot this frame's starting expansion state once, so every row's
                    // `is_open` reflects the same moment regardless of what an earlier row in
                    // this same frame's loop does to `next_expanded`.
                    let expanded_at_frame_start = self.expanded;
                    let mut next_expanded = expanded_at_frame_start;
                    let confirm_delete = self.confirm_delete;

                    // Every row-structure control below (arrows/toggle/delete/add) only
                    // *records* what it was asked to do -- `working`'s array itself must not be
                    // mutated while `row_ids` (captured just below) is being walked, so those
                    // mutations happen once, after the loop ends, in the standard immediate-mode
                    // "collect intent, mutate after" shape (mirrors `next_expanded` above, one
                    // level up). Condition edits inside a row's own body (plan 14-03) are
                    // different: they mutate only that one rule's fields via `working.rule_mut`,
                    // which never touches array structure/order, so those apply immediately
                    // rather than through `RowAction`.
                    let mut pending_action: Option<RowAction> = None;
                    let mut body_dirty = false;

                    let row_ids: Vec<RuleId> = working.rows().iter().map(|(id, _)| *id).collect();
                    let row_count = row_ids.len();

                    egui::ScrollArea::vertical()
                        .auto_shrink([false, true])
                        .show(ui, |ui| {
                            for (ordinal, id) in row_ids.iter().copied().enumerate() {
                                let is_open = expanded_at_frame_start == Some(id);
                                let is_confirming = confirm_delete == Some(id);
                                let Some(rule) = working.rule_mut(id) else {
                                    continue;
                                };
                                let enabled = rule.enabled != Some(false);
                                let header_text = display_rows
                                    .get(ordinal)
                                    .map(row_header_text)
                                    .unwrap_or_else(|| rule.style.clone());

                                widgets::zebra_row(ui, ordinal, |ui| {
                                    ui.horizontal(|ui| {
                                        if is_confirming {
                                            ui.colored_label(theme::WARM_ACCENT, "Delete?");
                                            if ui
                                                .button(
                                                    egui::RichText::new("Yes")
                                                        .color(theme::WARM_ACCENT),
                                                )
                                                .clicked()
                                            {
                                                pending_action =
                                                    Some(RowAction::ConfirmDelete(id));
                                            }
                                            if ui.button("Cancel").clicked() {
                                                pending_action = Some(RowAction::CancelDelete);
                                            }
                                        } else {
                                            if widgets::arrow_button(
                                                ui,
                                                widgets::ArrowDirection::Up,
                                                ordinal != 0,
                                            )
                                            .clicked()
                                            {
                                                pending_action = Some(RowAction::MoveUp(id));
                                            }
                                            if widgets::arrow_button(
                                                ui,
                                                widgets::ArrowDirection::Down,
                                                ordinal + 1 != row_count,
                                            )
                                            .clicked()
                                            {
                                                pending_action = Some(RowAction::MoveDown(id));
                                            }
                                            let mut checked = enabled;
                                            if widgets::circular_toggle(ui, &mut checked)
                                                .changed()
                                            {
                                                pending_action =
                                                    Some(RowAction::ToggleEnabled(id));
                                            }
                                            if ui.small_button("Delete").clicked() {
                                                pending_action =
                                                    Some(RowAction::RequestDelete(id));
                                            }
                                        }
                                    });

                                    let header = egui::CollapsingHeader::new(header_text)
                                        .id_salt(id)
                                        .open(Some(is_open))
                                        .show(ui, |ui| {
                                            if let Some(config) = config_ref {
                                                if rule_editor::render_rule_body(
                                                    ui, rule, config, id,
                                                ) {
                                                    body_dirty = true;
                                                }
                                            }
                                        });

                                    if header.header_response.clicked() {
                                        next_expanded = if is_open { None } else { Some(id) };
                                    }

                                    // Scroll the just-opened row into view -- fires exactly once
                                    // on the frame a row transitions closed -> open, never on
                                    // every frame it stays open (which would fight the player's
                                    // own scrolling).
                                    if !is_open && next_expanded == Some(id) {
                                        header
                                            .header_response
                                            .scroll_to_me(Some(egui::Align::TOP));
                                    }
                                });
                            }
                        });

                    ui.add_space(6.0);
                    ui.horizontal(|ui| {
                        ui.add_space(10.0);
                        if ui.button("+ ADD RULE").clicked() {
                            pending_action = Some(RowAction::Add);
                        }
                    });

                    self.expanded = next_expanded;
                    if body_dirty {
                        self.dirty = true;
                    }

                    // The one place `working`'s array structure is actually mutated this frame --
                    // `row_ids`/`working.rule_mut` above no longer borrow it once the closures
                    // that used them have returned.
                    if let Some(action) = pending_action {
                        match action {
                            RowAction::MoveUp(id) => {
                                if working.move_up(id) {
                                    self.dirty = true;
                                }
                            }
                            RowAction::MoveDown(id) => {
                                if working.move_down(id) {
                                    self.dirty = true;
                                }
                            }
                            RowAction::ToggleEnabled(id) => {
                                working.toggle_enabled(id);
                                self.dirty = true;
                            }
                            RowAction::RequestDelete(id) => {
                                // D-08: opening a confirm on a different row silently replaces
                                // whichever one was pending -- a single `Option`, not a per-row
                                // bool, makes "only one confirm live at a time" true by
                                // construction rather than something to enforce separately.
                                self.confirm_delete = Some(id);
                            }
                            RowAction::ConfirmDelete(id) => {
                                working.delete(id);
                                self.confirm_delete = None;
                                if self.expanded == Some(id) {
                                    self.expanded = None;
                                }
                                self.dirty = true;
                            }
                            RowAction::CancelDelete => {
                                self.confirm_delete = None;
                            }
                            RowAction::Add => {
                                // D-06/D-07: appended at the end, enabled, `unchanged`/no-op --
                                // safe the moment it lands because nothing an incomplete rule
                                // does can reach the game until Apply *and* an in-game reload.
                                let new_id = working.add();
                                self.expanded = Some(new_id);
                                self.dirty = true;
                            }
                        }
                    }
                }
                Err(message) => {
                    ui.horizontal_wrapped(|ui| {
                        ui.add_space(10.0);
                        ui.colored_label(theme::LABEL_TAN, message);
                    });
                }
            }

            // D-15/D-16/D-17: Apply, below the rule list. Reads whatever autosave most recently
            // wrote to `self.store_path` -- deliberately independent of whether `self.rules`
            // parsed for editing this session, since the on-disk store is the thing being
            // compiled, not the in-memory working model. `compile_store` and `write_filter` are
            // called here exactly once each and never re-implemented (D-15): `write_filter`'s
            // temp-file-in-the-destination-directory rename is what keeps the game from ever
            // reading a half-written filter, and the destination is on OneDrive, where a
            // cross-volume rename would fail -- reusing it, rather than writing the bytes
            // directly, is what keeps that guarantee here too.
            ui.add_space(10.0);
            ui.horizontal(|ui| {
                ui.add_space(10.0);
                ui.label(theme::row_label(apply::indicator(self.apply_state)));
            });
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                ui.add_space(10.0);
                if ui.button("APPLY").clicked() {
                    let store_dir = self
                        .store_path
                        .parent()
                        .unwrap_or_else(|| std::path::Path::new("."));
                    let paths = InputPaths::relative_to(store_dir);

                    let outcome = match dynamic::compile_store(&paths, &self.store_path) {
                        Ok(compiled) => match dynamic::default_destination() {
                            Some(dest_dir) => match dynamic::write_filter(&dest_dir, &compiled.text)
                            {
                                Ok(written) => ApplyOutcome::Written(written),
                                Err(err) => ApplyOutcome::Failed(err.to_string()),
                            },
                            // D-17: an unresolvable default destination is a rendered failure,
                            // not a panic -- USERPROFILE is unset off Windows, and this codebase
                            // has no `--dest`-style override UI this phase (out of scope).
                            None => ApplyOutcome::Failed(
                                "Cannot resolve the default PoE2 config folder \
                                 (USERPROFILE is not set)."
                                    .to_string(),
                            ),
                        },
                        Err(err) => ApplyOutcome::Failed(err.to_string()),
                    };

                    match outcome {
                        ApplyOutcome::Written(written) => {
                            self.apply_message = Some(apply::success_message(&written));
                            self.apply_state = self.apply_state.on_apply_success();
                        }
                        ApplyOutcome::Failed(message) => {
                            self.apply_message = Some(message);
                            self.apply_state = self.apply_state.on_apply_failure();
                        }
                    }
                }
            });
            if let Some(message) = &self.apply_message {
                let text_color = if self.apply_state == ApplyState::Applied {
                    theme::LABEL_TAN
                } else {
                    theme::WARM_ACCENT
                };
                ui.horizontal_wrapped(|ui| {
                    ui.add_space(10.0);
                    ui.colored_label(text_color, message);
                });
            }
        });

        // Task 3 / D-02, D-04: exactly one coalesced, atomic write per frame -- never per
        // keystroke, never on a timer/background thread. `self.dirty` is set by every row
        // control above (and, in plan 14-03, every condition editor) via the deferred-action
        // apply step; this is the single place that consumes it.
        //
        // RESEARCH Risk 4 / T-14-06: this call MUST precede the deferred viewport-close send
        // below in *program order* within this same `ui()` call. eframe cannot begin tearing
        // down the process until this function returns, so a save issued earlier in this same
        // function body is guaranteed to run to completion before the close command is even
        // dispatched. Reordering these two blocks (or moving the save back into the tray-poll
        // branch above, ahead of the body) silently reintroduces "lose the last edit on Quit" --
        // there is no automated test that would catch that regression, only this ordering itself.
        if self.dirty {
            // D-03: every frame that sets `dirty` also sets `ApplyState::Unapplied` -- doing it
            // here, in the single block that already consumes `dirty`, means it can never be
            // forgotten at one of the several call sites that set `dirty = true` above (row
            // moves, toggle, delete, add, and every condition edit). The stale outcome text from
            // a prior Apply is cleared too, so the stripe never shows "Wrote ..." next to an
            // indicator that already says the filter is out of date again.
            self.apply_state = ApplyState::Unapplied;
            self.apply_message = None;

            // Re-project the working model into `config` immediately before writing it.
            //
            // The sync near the top of this function runs BEFORE the row controls below have
            // mutated `working`, so by the time we get here `self.config` still holds the
            // pre-edit snapshot. Saving it wrote the state as of the *previous* frame, which
            // put `dynamic.json` permanently one edit behind: the rule you just added only
            // reached disk on the save triggered by your NEXT edit. Symptom, reported from
            // live use 2026-09-07: add a rule -> Apply (rule missing from the filter) ->
            // delete it -> Apply (the deleted rule now appears).
            //
            // Deriving here rather than trusting the frame-top sync makes the save independent
            // of where in the frame the edit happened, so no future control can reintroduce
            // this by being added in the wrong place.
            if let (Some(config), Ok(working)) = (self.config.as_mut(), self.rules.as_ref()) {
                config.rules = Some(working.to_rules());
            }

            if let Some(config) = &self.config {
                if let Err(message) = store::save(config, &self.store_path) {
                    self.last_save_error = Some(message);
                }
            }
            self.dirty = false;
        }

        if quit_requested {
            // D-14: Quit is the only path that ends the process -- deferred to here (see the
            // comment above) so the autosave above always has a chance to run first.
            ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close); // the only Close send site
        }
    }
}

/// What a row's controls requested this frame (D-05/D-06/D-08), applied once after the whole
/// list has finished rendering -- see the "collect intent, mutate after" comment at the call
/// site. Deliberately not `egui`-typed: this is the thin seam between the presentation loop and
/// `super::edit`'s pure mutation functions, not a widget itself.
enum RowAction {
    MoveUp(RuleId),
    MoveDown(RuleId),
    ToggleEnabled(RuleId),
    RequestDelete(RuleId),
    ConfirmDelete(RuleId),
    CancelDelete,
    Add,
}

/// One collapsed-row header line: ordinal, style, an explicit `[DISABLED]` marker for a
/// disabled rule (D-19/`PITFALLS.md` §9: still listed, never omitted, so a row's position in
/// the accordion always matches its position in `dynamic.json`), and the `view_model`-derived
/// condition summary.
fn row_header_text(row: &view_model::RuleRow) -> String {
    let marker = if row.enabled { "" } else { " [DISABLED]" };
    format!("{:02}  {}{}  —  {}", row.ordinal, row.style, marker, row.summary)
}
