//! `OverlayApp` -- the `eframe::App` implementation for the stripe window.
//!
//! What this file owns: docking the stripe to the right screen edge on the first frame where a
//! monitor size is known; polling the tray and the global hotkey for their events; turning those
//! events into an actual show/hide of the window that does not steal keyboard focus from the game,
//! by delegating the decision to [`super::visibility`] and the OS call to [`super::win`]; and the
//! filter-as-store footer (Phase 18).
//!
//! **Filter-as-store (Phase 18).** The game-folder `dynamic.filter` is the store. On launch the
//! overlay resolves the app root ([`crate::paths::app_root`], never the working directory, so
//! a double-click launch works) and loads the filter ([`dynamic::LoadedFilter::load`], which
//! never writes); RELOAD repeats both, and is offered whenever the root, the load, or the
//! economy markers are broken, so every such state can be recovered without a restart. APPLY refreshes only the economy section through [`dynamic::economy_body`] and
//! [`dynamic::apply_file`] -- the same functions the CLI's `apply` uses. ADD ECONOMY SECTION is a
//! separate, explicit button; APPLY never creates a section. When the guarded write refuses
//! because the file changed on disk, the stripe offers RELOAD, which re-reads the file and
//! discards in-memory state. Rule editing is temporarily absent (it returns in Phase 19, which
//! rebuilds the editor over parsed filter blocks); `edit`, `rule_editor` and `view_model` stay
//! compiled and tested meanwhile.
//!
//! The decisions (which button is live, what the status says) are pure functions in
//! [`super::apply`]; this file only draws them and performs the calls.
//! `PITFALLS.md` §8: if a bug can only be found by clicking, it belongs in a library module.

use std::time::Duration;

use global_hotkey::GlobalHotKeyManager;

use crate::compile::{InputPaths, PriceSource, PriceSourcePolicy};
use crate::dynamic::{self, ApplyOp, LoadedFilter};
use crate::safe_write::WriteOutcome;

use super::apply::{self, ApplyOutcome, ApplyState, FilterState};
use super::economy;
use super::hotkey::HotkeyStatus;
use super::layout;
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

        Self {
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

    /// What the footer knows about the filter, for [`apply::footer_actions`].
    fn filter_state(&self) -> FilterState {
        match &self.filter {
            Err(_) => FilterState::LoadFailed,
            Ok(loaded) if !loaded.existed() => FilterState::Absent,
            Ok(loaded) => match loaded.has_section() {
                Ok(true) => FilterState::HasSection,
                Ok(false) => FilterState::NoSection,
                Err(_) => FilterState::Damaged,
            },
        }
    }

    /// The one Apply path: price the economy section from the app root, then refresh it
    /// (`add == false`) or create it (`add == true`, the explicit ADD ECONOMY SECTION action).
    fn run_apply(&mut self, add: bool) {
        let Ok(root) = &self.root else {
            return;
        };
        let Ok(loaded) = &mut self.filter else {
            return;
        };
        let paths = InputPaths::relative_to(root);
        // Sentences appended after the main message.
        let mut extras = String::new();
        let outcome = match dynamic::economy_body(&paths, PriceSourcePolicy::Auto) {
            Err(err) => ApplyOutcome::Failed(err.to_string()),
            Ok((body, source)) => {
                if matches!(source, PriceSource::Fallback(_)) {
                    extras.push_str(" Used the fallback prices.");
                }
                let op = if add {
                    ApplyOp::AddEconomy(&body)
                } else {
                    ApplyOp::Refresh(&body)
                };
                match dynamic::apply_file(loaded, op) {
                    Ok(report) => {
                        if report.migrated {
                            extras.push_str(" Converted from v0.4.");
                        }
                        match report.outcome {
                            WriteOutcome::Unchanged => ApplyOutcome::Unchanged(report.path),
                            WriteOutcome::Written { backup } => {
                                if let Some(backup) = backup {
                                    extras.push_str(&format!(
                                        " Kept the original as {}.",
                                        backup.display()
                                    ));
                                }
                                ApplyOutcome::Written(report.path)
                            }
                        }
                    }
                    Err(err) => {
                        self.needs_reload = apply::is_changed_on_disk(&err);
                        ApplyOutcome::Failed(err.to_string())
                    }
                }
            }
        };
        match outcome {
            ApplyOutcome::Written(path) => {
                self.needs_reload = false;
                self.apply_message = Some(format!("{}{extras}", apply::success_message(&path)));
                self.apply_state = self.apply_state.on_apply_success();
            }
            ApplyOutcome::Unchanged(path) => {
                self.needs_reload = false;
                self.apply_message = Some(format!("{}{extras}", apply::unchanged_message(&path)));
                self.apply_state = self.apply_state.on_apply_success();
            }
            ApplyOutcome::Failed(message) => {
                self.apply_message = Some(message);
                self.apply_state = self.apply_state.on_apply_failure();
            }
        }
    }

    /// RELOAD: re-resolves the app root, re-reads the filter and discards in-memory state (D-08).
    /// Re-resolving the root here is what lets a stripe launched before the FerrisFilter folder
    /// was in place recover without a restart.
    fn reload(&mut self) {
        self.root = crate::paths::app_root().map_err(|e| e.to_string());
        self.filter = Self::load_filter();
        self.needs_reload = false;
        self.apply_message = None;
        self.apply_state = ApplyState::NeverApplied;
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
        let state = self.filter_state();
        let actions = apply::footer_actions(self.root.is_ok(), state, self.needs_reload);

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
                                bottom: 10,
                            }),
                    )
                    .show(ui, |ui| {
                        ui.add_space(10.0);
                        ui.horizontal(|ui| {
                            ui.add_space(10.0);
                            ui.label(theme::row_label(apply::indicator(self.apply_state)));
                        });
                        ui.add_space(4.0);
                        ui.horizontal(|ui| {
                            ui.add_space(10.0);
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
                            if actions.add_economy && ui.button("ADD ECONOMY SECTION").clicked() {
                                self.run_apply(true);
                            }
                            if ui
                                .add_enabled(actions.apply, egui::Button::new("APPLY"))
                                .clicked()
                            {
                                self.run_apply(false);
                            }
                        });
                        // The sync's own status line. Rendered before the Apply message so a fresh
                        // sync result is not hidden underneath a stale Apply one.
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
                                        egui::Label::new(egui::RichText::new(text).color(colour))
                                            .wrap(),
                                    );
                                });
                            }
                        }

                        if let Some(message) = &self.apply_message {
                            let text_color =
                                if self.apply_state == ApplyState::Applied && !self.needs_reload {
                                    theme::LABEL_TAN
                                } else {
                                    theme::WARM_ACCENT
                                };
                            ui.horizontal_wrapped(|ui| {
                                ui.add_space(10.0);
                                ui.colored_label(text_color, message);
                            });
                        }
                        if self.needs_reload {
                            ui.horizontal_wrapped(|ui| {
                                ui.add_space(10.0);
                                ui.colored_label(
                                    theme::WARM_ACCENT,
                                    "The filter changed on disk. RELOAD re-reads it and discards \
                                 in-memory state.",
                                );
                            });
                        }
                    });

                ui.add_space(10.0);
                ui.horizontal(|ui| {
                    ui.add_space(10.0);
                    ui.label(theme::section_header("FILTER"));
                });
                ui.add_space(6.0);

                // Status block: root, filter path and state, v0.4 notice, rule-editing notice.
                let migrated = self.filter.as_ref().map(|f| f.migrated()).unwrap_or(false);
                let mut lines: Vec<(egui::Color32, String)> = Vec::new();
                if let Err(message) = &self.root {
                    lines.push((theme::WARM_ACCENT, message.clone()));
                }
                match &self.filter {
                    Ok(loaded) => {
                        lines.push((theme::LABEL_TAN, loaded.path().display().to_string()));
                        let text = match (state, loaded.has_section()) {
                            (FilterState::Damaged, Err(err)) => err.to_string(),
                            _ => apply::filter_state_message(state, false),
                        };
                        let colour = if state == FilterState::Damaged {
                            theme::WARM_ACCENT
                        } else {
                            theme::LABEL_TAN
                        };
                        lines.push((colour, text));
                    }
                    Err(message) => lines.push((theme::WARM_ACCENT, message.clone())),
                }
                if migrated {
                    lines.push((
                        theme::LABEL_TAN,
                        "This is a v0.4 dynamic.filter. It will be converted on the next APPLY (a \
                     .ffbak backup of the original is kept)."
                            .to_string(),
                    ));
                }
                lines.push((
                    theme::TAB_ACTIVE,
                    "Rule editing returns in the next update. APPLY refreshes only the economy \
                 section."
                        .to_string(),
                ));
                for (colour, text) in lines {
                    ui.horizontal(|ui| {
                        ui.add_space(10.0);
                        ui.add(egui::Label::new(egui::RichText::new(text).color(colour)).wrap());
                    });
                    ui.add_space(4.0);
                }
            });

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
}
