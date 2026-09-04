//! Overlay GUI entry point.
//!
//! Layer rule (per `PITFALLS.md` §8): pure logic lives in the library, egui is a
//! thin view over it. If a bug can only be found by clicking, it is in the wrong
//! layer — `app.rs` and friends should stay presentation-only, delegating to
//! `ferrisfilter::dynamic` / `ferrisfilter::compile` for anything that touches
//! `dynamic.json` or the filter compiler.

pub mod app;

/// Launches the overlay GUI. Called from the `ff` binary's `main`.
pub fn launch() -> eframe::Result<()> {
    let options = eframe::NativeOptions::default();
    eframe::run_native(
        "ferrisfilter",
        options,
        Box::new(|_cc| Ok(Box::new(app::OverlayApp::default()))),
    )
}
