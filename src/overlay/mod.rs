//! Overlay GUI entry point.
//!
//! Layer rule (per `PITFALLS.md` §8): pure logic lives in the library, egui is a
//! thin view over it. If a bug can only be found by clicking, it is in the wrong
//! layer — `app.rs` and friends should stay presentation-only, delegating to
//! `ferrisfilter::dynamic` / `ferrisfilter::compile` for anything that touches
//! `dynamic.json` or the filter compiler.

pub mod app;
pub mod layout;
pub mod tray;

/// Launches the overlay GUI. Called from the `ff` binary's `main`.
pub fn launch() -> eframe::Result<()> {
    // Thread affinity (RESEARCH Pattern 3, the one hard constraint): the tray
    // must be built on the thread that runs the Win32 message loop. `launch()`
    // runs on the main thread (called directly from `ff`'s `main`), and
    // `eframe::run_native` below drives that same thread's message loop, so
    // building here -- before the loop starts -- satisfies the constraint.
    // Never build on a spawned thread.
    let tray = tray::build("FerrisFilter — Shift+Z to toggle")
        .expect("failed to build the system tray icon");
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
        Box::new(move |_cc| Ok(Box::new(app::OverlayApp::new(tray)))),
    )
}
