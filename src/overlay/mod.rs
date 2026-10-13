//! Overlay GUI entry point.
//!
//! Layer rule (per `PITFALLS.md` §8): pure logic lives in the library, egui is a
//! thin view over it. If a bug can only be found by clicking, it is in the wrong
//! layer — `app.rs` and friends should stay presentation-only, delegating to
//! `ferrisfilter::dynamic` / `ferrisfilter::compile` for anything that touches
//! the filter file or the filter compiler.

pub mod app;
pub mod apply;
pub mod body;
pub mod classes;
pub mod config_state;
pub mod config_view;
pub mod economy;
pub mod grammar;
pub mod hotkey;
pub mod instance;
pub mod layout;
pub mod rule_list;
pub mod rules;
pub mod text_entry;
pub mod theme;
pub mod tray;
pub mod virtual_list;
pub mod visibility;
pub mod widgets;
pub mod win;

use hotkey::HotkeyStatus;

/// Launches the overlay GUI. Called from the `ff` binary's `main`.
pub fn launch() -> eframe::Result<()> {
    // INST-01 / Phase 22 -- a second `ff` signals the running one to show its stripe (or shows
    // the not-responding message after `instance::STARTUP_TIMEOUT_MS` for a still-starting
    // instance plus `instance::ACK_TIMEOUT_MS` for the answer) and exits here, before it
    // registers a hotkey or builds a tray icon, so repeated launches never leave a second
    // stripe, icon or hotkey registration. The guard lives in `OverlayApp` until exit.
    let Some(instance_guard) = instance::acquire() else {
        return Ok(());
    };

    // Thread affinity (RESEARCH Pattern 3, the one hard constraint): both the
    // hotkey manager and the tray must be built on the thread that runs the
    // Win32 message loop. `launch()` runs on the main thread (called directly
    // from `ff`'s `main`), and `eframe::run_native` below drives that same
    // thread's message loop, so building here -- before the loop starts --
    // satisfies the constraint. Never build either on a spawned thread.
    let (hotkey_manager, hotkey_status) = hotkey::register_toggle();

    // Surface a registration failure through the tray (D-10): both the
    // tooltip and a disabled menu line name the combo and the reason, so the
    // failure is visible with no console open.
    let (tooltip, status_line) = match &hotkey_status {
        HotkeyStatus::Registered { .. } => {
            ("FerrisFilter — Ctrl+Shift+Z to toggle".to_string(), None)
        }
        HotkeyStatus::Failed(err) => {
            let msg = format!("FerrisFilter — Ctrl+Shift+Z unavailable: {err}");
            (msg.clone(), Some(msg))
        }
    };

    let tray = tray::build(&tooltip, status_line.as_deref())
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
        Box::new(move |cc| {
            // D-24/D-25: register the Cinzel display face and switch to the PoE2-Options
            // palette before the first frame paints, so nothing ever renders in egui's
            // defaults even for one frame.
            theme::install_fonts(&cc.egui_ctx);
            cc.egui_ctx.set_visuals(theme::poe2_visuals());

            Ok(Box::new(app::OverlayApp::new(
                tray,
                hotkey_manager,
                hotkey_status,
                instance_guard,
            )))
        }),
    )
}
