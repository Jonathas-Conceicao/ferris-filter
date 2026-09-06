//! `OverlayApp` — the `eframe::App` implementation for the stripe window.
//!
//! What this file owns: docking the stripe to the right screen edge on the first
//! frame where a monitor size is known (D-12); polling the tray and the global
//! hotkey for their events (plan 13-02); turning those events into an actual
//! show/hide of the window that does not steal keyboard focus from the game, by
//! delegating the decision to [`super::visibility`] and the OS call to
//! [`super::win`] (plan 13-03); and — this plan (13-04) — rendering the
//! read-only rule list loaded once at startup via [`super::store`] and
//! [`super::view_model`], in the PoE2-idiom chrome from [`super::theme`] and
//! [`super::widgets`]. This file itself contains no show/hide branching logic,
//! no rule-summarizing logic, and no palette values of its own — it advances
//! the state machine, asks it for the effect, and performs that one effect; it
//! holds the rows [`super::view_model::rule_rows`] already computed and paints
//! them through [`super::widgets`]. `PITFALLS.md` §8: if a bug can only be
//! found by clicking, it belongs in one of those library modules, not here.

use std::time::Duration;

use global_hotkey::GlobalHotKeyManager;

use super::hotkey::HotkeyStatus;
use super::layout;
use super::store;
use super::theme;
use super::tray::{self, OverlayCommand, TrayHandles};
use super::view_model::{self, RuleRow};
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

    /// The read-only rule list (D-19), loaded once at startup via
    /// [`store::load`] and mapped by [`view_model::rule_rows`]. `Err` holds
    /// the rendered message a malformed or missing `dynamic.json` produced —
    /// the stripe shows this text in place of the list rather than crashing
    /// (T-13-13). Never reloaded during a session; nothing in this phase
    /// mutates the store, so there is nothing to refresh from (D-20).
    rows: Result<Vec<RuleRow>, String>,
}

impl OverlayApp {
    /// Builds the app state around a tray and hotkey already constructed on
    /// the main thread (thread affinity — see `overlay::launch`).
    pub fn new(
        tray: TrayHandles,
        hotkey_manager: Option<GlobalHotKeyManager>,
        hotkey_status: HotkeyStatus,
    ) -> Self {
        // D-19/D-20: read-only, one shot, at startup. Nothing here seeds a filter or writes
        // anything beyond `store::load`'s own seed-if-absent (Phase 12's `ensure_store`).
        let (base, store_path) = store::default_paths();
        let rows = store::load(&base, &store_path).map(|view| view_model::rule_rows(&view.config));

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
            rows,
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

        // Poll the tray menu's Show/Quit events.
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
                    // D-14: Quit is the only path that ends the process.
                    ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
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

            // D-19: read-only, in ordinal order, no status chrome (no store path, no price
            // source, no destination folder). Content fills only the top of the stripe; the
            // remainder stays empty panel ground rather than shrinking the window (D-23) --
            // the ScrollArea below only grows as tall as its rows, it does not stretch to
            // fill the CentralPanel.
            match &self.rows {
                Ok(rows) => {
                    egui::ScrollArea::vertical()
                        .auto_shrink([false, true])
                        .show(ui, |ui| {
                            for row in rows {
                                widgets::zebra_row(ui, row.ordinal, |ui| {
                                    render_rule_row(ui, row);
                                });
                            }
                        });
                }
                Err(message) => {
                    ui.horizontal_wrapped(|ui| {
                        ui.add_space(10.0);
                        ui.colored_label(theme::LABEL_TAN, message);
                    });
                }
            }
        });
    }
}

/// Renders one rule row's ordinal, style, and condition summary. A disabled rule (D-19: still
/// listed, never omitted — `PITFALLS.md` §9's ordinal-identity guard) renders in
/// `theme::DISABLED_TEXT` with an explicit `DISABLED` marker rather than being skipped.
fn render_rule_row(ui: &mut egui::Ui, row: &RuleRow) {
    let text_color = if row.enabled {
        theme::LABEL_TAN
    } else {
        theme::DISABLED_TEXT
    };

    ui.horizontal(|ui| {
        ui.label(egui::RichText::new(format!("{:02}", row.ordinal)).color(text_color).monospace());
        ui.label(egui::RichText::new(&row.style).color(theme::VALUE_TEXT));
        if !row.enabled {
            ui.label(egui::RichText::new("DISABLED").color(theme::DISABLED_TEXT).small());
        }
    });
    ui.label(egui::RichText::new(&row.summary).color(text_color).small());
}
