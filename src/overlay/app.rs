//! `OverlayApp` — the `eframe::App` implementation for the stripe window.
//!
//! Content is deliberately a placeholder this plan; read-only rule-list content
//! lands in a later plan (13-04). What this file owns: docking the stripe to the
//! right screen edge on the first frame where a monitor size is known (D-12),
//! and — this plan (13-02) — polling the tray and (Task 2) the global hotkey
//! for their events, proving both reach `update()` before any show/hide
//! mechanism exists (that is plan 13-03's subject).

use std::time::Duration;

use super::layout;
use super::tray::{self, OverlayCommand, TrayHandles};

/// The eframe application state for the overlay stripe.
pub struct OverlayApp {
    /// Latches `true` once the first-frame docking has run, so it never runs
    /// again (position/width are not persisted or re-derived per frame — D-15).
    docked: bool,

    /// The tray icon, menu, and id->command mapping. Kept alive here for the
    /// process lifetime — dropping it would remove the icon from the tray.
    tray: TrayHandles,

    /// Set by the tray's left click (and, from Task 2, the hotkey). Consumed
    /// once per frame into `toggle_count`. The real show/hide mechanism is
    /// plan 13-03; this plan proves delivery only.
    toggle_requested: bool,

    /// Set by the tray menu's "Show" item. Not yet consumed by anything —
    /// the visibility mechanism that reads this lands in plan 13-03.
    #[allow(dead_code)]
    show_requested: bool,

    /// Temporary scaffolding: increments once per `toggle_requested` and is
    /// rendered on the stripe as `"toggles: N"` so event arrival is
    /// observable before any show/hide behavior exists. Plan 13-03 removes
    /// this label along with the real visibility mechanism.
    toggle_count: u32,
}

impl OverlayApp {
    /// Builds the app state around a tray already constructed on the main
    /// thread (thread affinity — see `overlay::launch`).
    pub fn new(tray: TrayHandles) -> Self {
        Self {
            docked: false,
            tray,
            toggle_requested: false,
            show_requested: false,
            toggle_count: 0,
        }
    }
}

impl eframe::App for OverlayApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
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
                Some(OverlayCommand::Show) => self.show_requested = true,
                Some(OverlayCommand::Toggle) => self.toggle_requested = true,
                Some(OverlayCommand::Quit) => {
                    // D-14: Quit is the only path that ends the process.
                    ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
                }
                None => {}
            }
        }

        if self.toggle_requested {
            self.toggle_count += 1;
            self.toggle_requested = false;
        }

        // Explicit polling cadence rather than relying on eframe's internal
        // heartbeat (RESEARCH Pattern 3).
        ui.ctx()
            .request_repaint_after(Duration::from_millis(50));

        egui::CentralPanel::default().show(ui, |ui| {
            ui.label("FerrisFilter overlay");
            ui.label(format!("toggles: {}", self.toggle_count));
        });
    }
}
