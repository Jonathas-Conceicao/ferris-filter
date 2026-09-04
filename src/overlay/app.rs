//! `OverlayApp` — the `eframe::App` implementation for the stripe window.
//!
//! Deliberately empty this plan: it exists so `overlay::launch` has something to
//! run and a window opens. Docking geometry lands in plan 13-01 Task 3; read-only
//! rule-list content lands in a later plan (13-04).

/// The eframe application state for the overlay stripe.
#[derive(Default)]
pub struct OverlayApp {}

impl eframe::App for OverlayApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ui, |ui| {
            ui.label("FerrisFilter overlay");
        });
    }
}
