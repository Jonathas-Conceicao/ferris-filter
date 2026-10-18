//! Overlay GUI entry point.
//!
//! Layer rule (per `PITFALLS.md` §8): pure logic lives in the library, egui is a
//! thin view over it. If a bug can only be found by clicking, it is in the wrong
//! layer — `app.rs` and friends should stay presentation-only, delegating to
//! `ferrisfilter::dynamic` / `ferrisfilter::compile` for anything that touches
//! the filter file or the filter compiler.

pub mod about_view;
pub mod app;
pub mod apply;
pub mod apply_all;
pub mod body;
pub mod chat;
pub mod classes;
pub mod conditions;
pub mod config_state;
pub mod config_view;
pub mod economy;
pub(crate) mod folder_dialog;
pub mod grammar;
pub mod hotkey;
pub mod instance;
pub mod layout;
pub mod rule_list;
pub mod rules;
#[cfg(debug_assertions)]
pub(crate) mod screenshot;
pub(crate) mod shell;
pub mod sound_files;
pub mod style_lines;
pub mod text_entry;
pub mod tray;
pub mod virtual_list;
pub mod visibility;
pub mod widgets;
pub mod win;

use hotkey::HotkeyStatus;

/// Launches the overlay GUI. Called from the `ff` binary's `main`.
pub fn launch() -> eframe::Result<()> {
    // Debug builds only: `ff --screenshot <png> ...` renders the shell from a temp home and
    // exits, with no instance guard, hotkey or tray (see `screenshot`).
    #[cfg(debug_assertions)]
    match screenshot::parse(std::env::args().skip(1)) {
        Ok(Some(shot)) => return run_screenshot(shot),
        Ok(None) => {}
        Err(message) => {
            eprintln!("ff: {message}");
            return Ok(());
        }
    }

    // INST-01 / Phase 22 -- a second `ff` signals the running one to show its window (or shows
    // the not-responding message after `instance::STARTUP_TIMEOUT_MS` for a still-starting
    // instance plus `instance::ACK_TIMEOUT_MS` for the answer) and exits here, before it
    // registers a hotkey or builds a tray icon, so repeated launches never leave a second
    // window, icon or hotkey registration. If the single-instance check itself fails, it also
    // exits here (fail closed, with a message box naming the error). The guard lives in
    // `OverlayApp` until tray Quit, which drops it at once so an immediate relaunch can start.
    // Quit unregisters the hotkey and removes the tray icon first, so that relaunch can
    // register `Ctrl+Shift+Z` itself.
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
    let options = native_options();

    eframe::run_native(
        "ferrisfilter",
        options,
        Box::new(move |cc| {
            // Fonts, the skin's style and every texture, before the first frame paints, so
            // nothing ever renders in egui's defaults even for one frame.
            let skin = crate::skin::install(&cc.egui_ctx);

            Ok(Box::new(app::OverlayApp::new(
                tray,
                hotkey_manager,
                hotkey_status,
                instance_guard,
                skin,
            )))
        }),
    )
}

/// The window: opaque, undecorated, always-on-top, fixed at the shell width (660pt) and docked
/// by `win::dock_left` once its handle exists.
///
/// Deliberately NOT configured (D-13, dodging documented egui/eframe Windows overlay defects by
/// omission -- see 13-RESEARCH.md "Anti-Patterns to Avoid"):
///   - transparency        -- egui #4451 / #4446
///   - mouse click-through -- egui #2537
///   - exclusive fullscreen -- egui #4091
fn native_options() -> eframe::NativeOptions {
    let viewport = egui::ViewportBuilder::default()
        .with_title("FerrisFilter")
        .with_always_on_top()
        .with_decorations(false)
        .with_inner_size([shell::WIDTH, 1000.0])
        .with_resizable(false)
        .with_taskbar(false)
        .with_active(false);
    eframe::NativeOptions {
        viewport,
        ..Default::default()
    }
}

/// The debug screenshot mode: refuses unless the environment is a temp copy, then runs the shell
/// with an inert tray and no instance guard or hotkey until the frame is saved.
#[cfg(debug_assertions)]
fn run_screenshot(shot: screenshot::ShotArgs) -> eframe::Result<()> {
    let prepared = screenshot::guard_env().and_then(|()| screenshot::stage_filter(&shot));
    if let Err(message) = prepared {
        eprintln!("ff: {message}");
        return Ok(());
    }
    eframe::run_native(
        "ferrisfilter-screenshot",
        native_options(),
        Box::new(move |cc| {
            let skin = crate::skin::install(&cc.egui_ctx);
            Ok(Box::new(app::OverlayApp::new_screenshot(skin, shot)))
        }),
    )
}
