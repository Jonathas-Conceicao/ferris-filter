//! `OverlayApp` — the `eframe::App` implementation for the stripe window.
//!
//! Content is deliberately a placeholder this plan; read-only rule-list content
//! lands in a later plan (13-04). What this file owns: docking the stripe to the
//! right screen edge on the first frame where a monitor size is known, per D-12.

use super::layout;

/// The eframe application state for the overlay stripe.
#[derive(Default)]
pub struct OverlayApp {
    /// Latches `true` once the first-frame docking has run, so it never runs
    /// again (position/width are not persisted or re-derived per frame — D-15).
    docked: bool,
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

        egui::CentralPanel::default().show(ui, |ui| {
            ui.label("FerrisFilter overlay");
        });
    }
}
