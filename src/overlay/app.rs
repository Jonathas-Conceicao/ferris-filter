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
use crate::models::config::{Config, Rule};

use super::apply::{self, ApplyOutcome, ApplyState};
use super::economy;
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

    /// The economy-sync affordance: a `prices.json` refresh run on a worker thread so a stripe
    /// drawn over a live game never freezes on network I/O. It writes prices only — the game sees
    /// them on the next **Apply**, which is what makes it safe to press mid-map.
    economy: economy::EconomySync,

    /// The last rule set actually written to `dynamic.json` — the state Revert restores to.
    ///
    /// Set at load and refreshed on every successful Apply. Since 2026-09-08 edits live only in
    /// memory until Apply (see the note on the dirty block below), so this is the difference
    /// between what is on disk and what is on screen.
    baseline: Vec<Rule>,
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
            // The on-disk state Revert restores to. Taken before `rules` is moved into the
            // struct, so a load failure simply yields an empty baseline rather than diverging
            // from what is on screen.
            baseline: rules
                .as_ref()
                .map(|working| working.to_rules())
                .unwrap_or_default(),
            rules,
            config,
            store_path,
            expanded: None,
            confirm_delete: None,
            dirty: false,
            last_save_error: None,
            apply_state: ApplyState::NeverApplied,
            apply_message: None,
            economy: economy::EconomySync::new(),
        }
    }
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
            self.last_save_error = None;
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

            // The footer is a bottom panel, and it is claimed BEFORE the rule list is laid out.
            //
            // It used to render after the ScrollArea in the same vertical flow, and the
            // ScrollArea is `auto_shrink([false, true])` — it grows to fit its rows. With few
            // rules that was fine and the footer sat beneath them. With enough rules (collapsed
            // or expanded) the list claimed the whole panel and pushed the indicator, REVERT,
            // SYNC ECONOMY and APPLY off the bottom of the stripe, unreachable at any scroll
            // position — the controls that write the filter became unclickable precisely when
            // the filter had the most edits waiting in it (author, 2026-09-10).
            //
            // A bottom panel reserves its height first and hands the remainder to the central
            // content, so the footer is always on screen and the list scrolls inside what is
            // left. Order is the whole fix: called after the list, `show_inside` would find the
            // space already taken.
            //
            // Rendering the footer before the list means its controls read state as of the
            // start of this frame. That is safe because a click lands on exactly one widget per
            // frame — you cannot edit a rule and press APPLY in the same frame — and the row
            // loop below requests a repaint when it changes anything, so nothing here is more
            // than one frame stale.
            // egui 0.36.1 unified SidePanel/TopBottomPanel into one `Panel` type; `bottom` is
            // non-resizable by construction.
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
                            bottom: 10,
                        }),
                )
                .show(ui, |ui| {
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
                    if self.dirty && ui.button("REVERT").clicked() {
                        // Restores the last state written to disk. `from_rules` mints fresh
                        // RuleIds, so any expansion or pending delete keyed by an old id would
                        // dangle -- both are cleared rather than left pointing at nothing.
                        self.rules = Ok(WorkingRules::from_rules(self.baseline.clone()));
                        self.expanded = None;
                        self.confirm_delete = None;
                        self.dirty = false;
                        self.apply_message = None;
                        self.apply_state = ApplyState::Applied;
                    }
                    // Economy sync sits beside Apply because that is the order it is used in:
                    // refresh prices, then compile them. It is deliberately NOT part of Apply — a
                    // sync is dozens of network round trips and can fail, and folding that into the
                    // action that writes the game's filter would make a routine Apply slow and
                    // failure-prone (author, 2026-09-10).
                    //
                    // No --standard/--hc/--league/--tiers switches here: the GUI syncs with the tiers
                    // file already in place, unlike the CLI.
                    let syncing = self.economy.is_running();
                    let sync_label = if syncing { "SYNCING\u{2026}" } else { "SYNC ECONOMY" };
                    if ui
                        .add_enabled(!syncing, egui::Button::new(sync_label))
                        .clicked()
                    {
                        // The store's directory, not the process working directory: the overlay can
                        // be launched from anywhere, and prices.json must land where Apply will read
                        // it from.
                        let base_dir = self
                            .store_path
                            .parent()
                            .map(std::path::Path::to_path_buf)
                            .unwrap_or_else(|| std::path::PathBuf::from("."));
                        self.economy.start(move || {
                            crate::sync::sync_economy_blocking(&base_dir).map_err(|e| e.to_string())
                        });
                    }
                    if ui.button("APPLY").clicked() {
                        // Since edits are in-memory (D-02, revised 2026-09-08), Apply is the only
                        // writer: persist the store, THEN compile the filter from it. In that order,
                        // because compile_store reads dynamic.json from disk — compiling first would
                        // reproduce exactly the stale-Apply bug fixed in 3375ad4, where the filter
                        // was built from the previous state.
                        if let (Some(config), Ok(working)) =
                            (self.config.as_mut(), self.rules.as_ref())
                        {
                            config.rules = Some(working.to_rules());
                        }
                        // A failed save is TERMINAL -- `apply::guard_save` turns it into the
                        // outcome, and the compile below is then structurally unreachable. It has
                        // to be: `compile_store` reads dynamic.json back from disk, so compiling
                        // after a failed save builds the filter from the PREVIOUS store, writes it
                        // successfully, and reports success -- while advancing `baseline` and
                        // clearing `dirty`, destroying the only copy of the edits. That is the
                        // v0.4 milestone audit's BLOCKER-1; the save result is no longer merely
                        // recorded and stepped over.
                        let save = match &self.config {
                            Some(config) => store::save(config, &self.store_path),
                            None => Ok(()),
                        };
                        if let Err(message) = &save {
                            self.last_save_error = Some(message.clone());
                        }

                        let store_dir = self
                            .store_path
                            .parent()
                            .unwrap_or_else(|| std::path::Path::new("."));
                        let paths = InputPaths::relative_to(store_dir);

                        let outcome = match apply::guard_save(save) {
                            Err(stopped) => stopped,
                            Ok(()) => match dynamic::compile_store(&paths, &self.store_path) {
                                Ok(compiled) => match dynamic::default_destination() {
                                    Some(dest_dir) => {
                                        match dynamic::write_filter(&dest_dir, &compiled.text) {
                                            Ok(written) => ApplyOutcome::Written(written),
                                            Err(err) => ApplyOutcome::Failed(err.to_string()),
                                        }
                                    }
                                    // D-17: an unresolvable default destination is a rendered
                                    // failure, not a panic -- USERPROFILE is unset off Windows,
                                    // and this codebase has no `--dest`-style override UI this
                                    // phase (out of scope).
                                    None => ApplyOutcome::Failed(
                                        "Cannot resolve the default PoE2 config folder \
                                         (USERPROFILE is not set)."
                                            .to_string(),
                                    ),
                                },
                                Err(err) => ApplyOutcome::Failed(err.to_string()),
                            },
                        };

                        match outcome {
                            ApplyOutcome::Written(written) => {
                                // Only reachable when the save succeeded, so clearing the stale
                                // save error here cannot hide a live one.
                                self.last_save_error = None;
                                self.apply_message = Some(apply::success_message(&written));
                                self.apply_state = self.apply_state.on_apply_success();
                                if let Ok(working) = self.rules.as_ref() {
                                    self.baseline = working.to_rules();
                                }
                                self.dirty = false;
                            }
                            ApplyOutcome::Failed(message) => {
                                self.apply_message = Some(message);
                                self.apply_state = self.apply_state.on_apply_failure();
                            }
                        }
                    }
                });
                // The sync's own status line. Rendered before the Apply message so a fresh sync
                // result is not hidden underneath a stale Apply one.
                match self.economy.state() {
                    economy::SyncState::Idle => {}
                    economy::SyncState::Running => {
                        ui.add_space(4.0);
                        ui.horizontal(|ui| {
                            ui.add_space(10.0);
                            ui.add(
                                egui::Label::new(
                                    egui::RichText::new("Fetching economy prices\u{2026}")
                                        .color(theme::TAB_ACTIVE),
                                )
                                .wrap(),
                            );
                        });
                    }
                    economy::SyncState::Finished(outcome) => {
                        ui.add_space(4.0);
                        let colour = if outcome.is_failure() {
                            theme::WARM_ACCENT
                        } else {
                            theme::TAB_ACTIVE
                        };
                        let text = outcome.message().to_string();
                        ui.horizontal(|ui| {
                            ui.add_space(10.0);
                            ui.add(
                                egui::Label::new(egui::RichText::new(text).color(colour)).wrap(),
                            );
                        });
                    }
                }

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

                                // Set by a click on the controls line's empty remainder; folded
                                // into the same toggle the header click drives, so both lines
                                // behave identically.
                                let mut row_click = false;

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

                                        // The empty remainder of the controls line toggles the
                                        // row too. Allocated AFTER the buttons, so it only ever
                                        // covers space they did not claim — the arrows, the
                                        // enable toggle and Delete keep their own hit areas
                                        // rather than competing with a full-width overlay.
                                        let rest = ui.available_size();
                                        if rest.x > 0.0
                                            && ui
                                                .allocate_response(rest, egui::Sense::click())
                                                .clicked()
                                        {
                                            row_click = true;
                                        }
                                    });

                                    // `collapsing_header_frame` is what makes the header span the
                                    // full row width; without it egui sizes the header to its
                                    // text, so only the text itself was clickable and the empty
                                    // space beside it did nothing (author, 2026-09-07).
                                    //
                                    // It also switches on the header's background painting, so
                                    // the inactive fill is made transparent here — leaving a
                                    // faint warm tint on hover, which is the affordance that
                                    // tells you the whole line is clickable.
                                    let header = ui
                                        .scope(|ui| {
                                            let visuals = ui.visuals_mut();
                                            visuals.collapsing_header_frame = true;
                                            visuals.widgets.inactive.weak_bg_fill =
                                                egui::Color32::TRANSPARENT;
                                            visuals.widgets.inactive.bg_stroke =
                                                egui::Stroke::NONE;
                                            visuals.widgets.hovered.weak_bg_fill =
                                                theme::HEADER_PLATE;
                                            visuals.widgets.hovered.bg_stroke = egui::Stroke::NONE;
                                            visuals.widgets.active.weak_bg_fill =
                                                theme::HEADER_PLATE;
                                            visuals.widgets.active.bg_stroke = egui::Stroke::NONE;

                                            egui::CollapsingHeader::new(header_text)
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
                                                })
                                        })
                                        .inner;

                                    // The wrapped summary sits under the header while the row is
                                    // collapsed. It cannot live in the header itself
                                    // (`CollapsingHeader` hardcodes `TextWrapMode::Extend`), and
                                    // it is not repeated when open because the editors below
                                    // already show every condition in full.
                                    if !is_open {
                                        if let Some(row) = display_rows.get(ordinal) {
                                            row_summary_label(ui, row);
                                        }
                                    }

                                    if header.header_response.clicked() || row_click {
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

                            // + ADD RULE belongs INSIDE the scroll area, at the end of the
                            // list it appends to. Rendered after it, the button had nowhere to
                            // go once the list filled the remaining height and drew on top of
                            // the footer's first line — the overlap in screenshot
                            // poe2_print007 (author, 2026-09-11). Inside, it scrolls with the
                            // rules, which is also where a reader looks for it.
                            ui.add_space(6.0);
                            ui.horizontal(|ui| {
                                ui.add_space(10.0);
                                if ui.button("+ ADD RULE").clicked() {
                                    pending_action = Some(RowAction::Add);
                                }
                            });
                        });

                    self.expanded = next_expanded;
                    if body_dirty {
                        self.dirty = true;
                    }

                    // The one place `working`'s array structure is actually mutated this frame --
                    // `row_ids`/`working.rule_mut` above no longer borrow it once the closures
                    // that used them have returned.
                    if let Some(action) = pending_action {
                        // The footer renders before this loop, so anything changed here (dirty,
                        // and therefore whether REVERT is shown) would otherwise not appear until
                        // the next repaint — which, over a game generating no input events, may
                        // be a long time coming.
                        ui.ctx().request_repaint();
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
            // NOTE: this block no longer writes anything. Edits stay in memory until Apply
            // (D-02, revised 2026-09-08 at the author's request, alongside the Revert button)
            // -- Apply is now the single writer of both dynamic.json and dynamic.filter.
            //
            // `dirty` therefore stays true until an Apply or a Revert clears it, rather than
            // being consumed each frame: it is what makes those two buttons appear, and what
            // `ApplyState::Unapplied` reflects.
            //
            // The trade this accepts: a crash or a Quit now loses unsaved edits. Autosave was
            // originally chosen to avoid exactly that, so the Revert button and the
            // unsaved-changes indicator are what replace it -- the state is visible, and
            // discarding it is deliberate rather than incidental.
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

/// One row's header line: ordinal, style, and an explicit `[DISABLED]` marker for a disabled
/// rule (D-19/`PITFALLS.md` §9: still listed, never omitted, so a row's position in the
/// accordion always matches its position in `dynamic.json`).
///
/// The condition summary is deliberately NOT here. `CollapsingHeader` builds its header galley
/// with a hardcoded `TextWrapMode::Extend`, so header text can never wrap — a long summary ran
/// off the 460px stripe and was simply unreadable (reported from live use 2026-09-07, screenshot
/// poe2_print004). Widening the stripe for a handful of long rules was explicitly not wanted, so
/// the summary moved to [`row_summary_label`], a wrapped label the caller renders beneath the
/// header while the row is collapsed.
/// D-10: `<index>  <style>[ - name]`, in that order — the name is appended only when the rule
/// has one, and never replaces the style. An unnamed row's text is unchanged from before Phase
/// 15: this is display text only, formatted from [`view_model::RuleRow::name`]; nothing about a
/// rule's identity, expansion state, deletion, or reordering is ever keyed off it (D-13,
/// `PITFALLS.md` §9) — `RuleId` stays the identity and array position stays the precedence.
fn row_header_text(row: &view_model::RuleRow) -> String {
    let marker = if row.enabled { "" } else { " [DISABLED]" };
    match &row.name {
        Some(name) if !name.is_empty() => {
            format!("{:02}  {}{} - {}", row.ordinal, row.style, marker, name)
        }
        _ => format!("{:02}  {}{}", row.ordinal, row.style, marker),
    }
}

/// The wrapped condition summary shown under a **collapsed** row.
///
/// Only while collapsed: an expanded row's editors already show every condition in full, so
/// repeating the summary above them would be redundant and cost scarce vertical space.
fn row_summary_label(ui: &mut egui::Ui, row: &view_model::RuleRow) {
    ui.add(egui::Label::new(theme::row_summary(&row.summary)).wrap());
}

#[cfg(test)]
mod tests {
    use super::*;

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
                            // Far more rows than fit — the condition that triggered the
                            // defect. Allocated geometry rather than labels: the headless
                            // context has no fonts, so text would measure zero-height and the
                            // list would never grow (which is exactly what the control
                            // assertion below caught on the first attempt).
                            for _ in 0..60 {
                                ui.allocate_space(egui::vec2(400.0, 30.0));
                            }
                        });
                };
                if footer_first {
                    egui::Panel::bottom(egui::Id::new("test-footer"))
                        .show(ui, |ui| {
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

    /// APPLY must stay on screen however many rules the list holds.
    ///
    /// The footer used to be laid out *after* an `auto_shrink([false, true])` `ScrollArea`, which
    /// grows to fit its rows. Past a certain number of rules the list claimed the whole panel and
    /// pushed APPLY off the bottom edge, unreachable at any scroll offset — the control that
    /// writes the filter became unclickable exactly when the most edits were waiting to be written
    /// (author, 2026-09-10).
    ///
    /// The second assertion is the one that gives the first its meaning: it shows the old
    /// arrangement really does push the button off-screen with this many rows, so the test would
    /// have failed before the fix rather than passing for unrelated reasons.
    #[test]
    fn the_apply_button_stays_on_screen_however_long_the_rule_list_is() {
        let (footer_bottom, screen_bottom) = footer_bottom_edge(true);
        assert!(
            footer_bottom <= screen_bottom,
            "APPLY is below the stripe's bottom edge ({footer_bottom} > {screen_bottom}) — \
             it cannot be clicked at any scroll position"
        );

        let (old_bottom, screen_bottom) = footer_bottom_edge(false);
        assert!(
            old_bottom > screen_bottom,
            "the pre-fix arrangement was expected to push APPLY off-screen, but it did not \
             ({old_bottom} <= {screen_bottom}) — this test is no longer exercising the defect"
        );
    }

    fn row(ordinal: usize, style: &str, enabled: bool, summary: &str) -> view_model::RuleRow {
        view_model::RuleRow {
            ordinal,
            enabled,
            style: style.to_string(),
            name: None,
            summary: summary.to_string(),
        }
    }

    fn row_named(ordinal: usize, style: &str, enabled: bool, name: &str) -> view_model::RuleRow {
        view_model::RuleRow {
            ordinal,
            enabled,
            style: style.to_string(),
            name: Some(name.to_string()),
            summary: String::new(),
        }
    }

    #[test]
    fn the_header_carries_identity_only_never_the_summary() {
        // Regression: the summary used to live in the header, where `CollapsingHeader`'s
        // hardcoded `TextWrapMode::Extend` ran it off the 460px stripe unreadably. It belongs
        // in `row_summary_label`, which wraps. Putting it back here would silently reintroduce
        // the clipping, and nothing about the header's own rendering would look wrong.
        let long = "Rings, Amulets, Belts, Body Armours +17 more \u{b7} Unidentified Tier \u{2265} 4";
        let header = row_header_text(&row(8, "unidentified", true, long));
        assert_eq!(header, "08  unidentified");
        assert!(
            !header.contains("Rings"),
            "the summary must not be in the header: {header}"
        );
    }

    #[test]
    fn a_disabled_row_is_marked_in_the_header() {
        // D-19 / PITFALLS §9: a disabled rule stays listed in place, so its ordinal still
        // matches its index in dynamic.json — the marker is how it reads as disabled.
        let header = row_header_text(&row(3, "hidden", false, "anything"));
        assert_eq!(header, "03  hidden [DISABLED]");
    }

    #[test]
    fn the_ordinal_is_zero_padded_so_rows_stay_aligned() {
        assert!(row_header_text(&row(0, "S", true, "")).starts_with("00  "));
        assert!(row_header_text(&row(12, "S", true, "")).starts_with("12  "));
    }

    #[test]
    fn a_named_row_shows_its_name_after_the_style() {
        // D-10: `<index> <style>[ - name]` -- the style stays, the name is appended.
        let header = row_header_text(&row_named(0, "unchanged", true, "Change Bases"));
        assert_eq!(header, "00  unchanged - Change Bases");
    }

    #[test]
    fn a_disabled_named_row_keeps_the_marker_before_the_name() {
        let header = row_header_text(&row_named(3, "hidden", false, "Change Bases"));
        assert_eq!(header, "03  hidden [DISABLED] - Change Bases");
    }
}
