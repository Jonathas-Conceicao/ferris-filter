//! Overlay GUI entry point.
//!
//! Layer rule (per `PITFALLS.md` §8): pure logic lives in the library, egui is a
//! thin view over it. If a bug can only be found by clicking, it is in the wrong
//! layer — `app.rs` and friends should stay presentation-only, delegating to
//! `ferrisfilter::dynamic` / `ferrisfilter::compile` for anything that touches
//! `dynamic.json` or the filter compiler.

pub mod app;
pub mod layout;

/// Launches the overlay GUI. Called from the `ff` binary's `main`.
pub fn launch() -> eframe::Result<()> {
    // The stripe: opaque, undecorated, always-on-top, never screen-sized.
    //
    // Deliberately NOT configured (D-13, dodging documented egui/eframe Windows
    // overlay defects by omission — see 13-RESEARCH.md "Anti-Patterns to Avoid"):
    //   - transparency        — egui #4451 / #4446
    //   - mouse click-through — egui #2537
    //   - exclusive fullscreen — egui #4091
    let viewport = egui::ViewportBuilder::default()
        .with_title("FerrisFilter")
        .with_always_on_top()
        .with_decorations(false)
        .with_inner_size([layout::STRIPE_WIDTH, 1000.0])
        .with_resizable(false)
        .with_taskbar(false)
        .with_active(false);

    let options = eframe::NativeOptions {
        viewport,
        ..Default::default()
    };

    eframe::run_native(
        "ferrisfilter",
        options,
        Box::new(|_cc| Ok(Box::new(app::OverlayApp::default()))),
    )
}
