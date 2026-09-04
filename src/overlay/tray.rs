//! System tray icon, menu, and the pure event->command translation for the
//! overlay (D-16, D-17, D-18).
//!
//! Thread affinity is the one hard constraint (RESEARCH "Architecture
//! Patterns" -> Pattern 3): `build()` must be called on the thread that runs
//! the Win32 message loop -- i.e. before `eframe::run_native` starts it, or
//! inside its app-creation closure. Never from a spawned thread: a thread
//! with no message loop never receives the tray's click/menu messages
//! ("Anti-Patterns to Avoid").

use tray_icon::menu::{Menu, MenuId, MenuItem};
use tray_icon::{Icon, TrayIcon, TrayIconBuilder};

/// Commands the tray can request of the overlay. Pure data --
/// `OverlayApp::update` is the only place that interprets it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OverlayCommand {
    /// Toggle the stripe's visibility (left-click on the icon, D-17).
    Toggle,
    /// Show the stripe (tray menu "Show").
    Show,
    /// End the process (tray menu "Quit" -- D-14: the only exit).
    Quit,
}

/// Everything the tray owns for the process lifetime.
///
/// **Must be kept alive for the process lifetime** (store it in
/// `OverlayApp`) -- dropping the `TrayIcon` removes the icon from the
/// notification area.
///
/// `tray_icon` is `Option` only so `command_for_menu`'s pure id-matching
/// logic can be unit-tested against a `TrayHandles` built with known ids and
/// no live OS tray (see the tests below). The running app always holds
/// `Some`.
pub struct TrayHandles {
    // Never read after construction -- held purely for its `Drop` impl, which
    // removes the icon from the notification area. `#[allow(dead_code)]`
    // documents that this is intentional RAII, not an oversight.
    #[allow(dead_code)]
    tray_icon: Option<TrayIcon>,
    show_id: MenuId,
    quit_id: MenuId,
}

/// Decodes the compiled-in Ferris PNG into a `tray_icon::Icon`.
///
/// A failure here is a programming error on a compiled-in asset (a corrupt
/// or mis-encoded `assets/ferris.png`), not a runtime condition to recover
/// from -- `expect` naming the asset is correct, not a silent fallback.
fn load_icon() -> Icon {
    let bytes = include_bytes!("../../assets/ferris.png");
    let image = image::load_from_memory(bytes)
        .expect("assets/ferris.png must decode as a valid image")
        .into_rgba8();
    let (width, height) = image.dimensions();
    Icon::from_rgba(image.into_raw(), width, height)
        .expect("assets/ferris.png must have valid icon dimensions")
}

/// Builds the tray icon and its Show/Quit menu (D-17).
///
/// `status_line`, when `Some`, is appended to the menu as a third, disabled
/// item carrying the same text as a failed hotkey registration (D-10) — so
/// the failure is visible from the tray alone, with no console. `tooltip`
/// reflects the same state: the working combo on success, or the combo and
/// the reason on failure.
///
/// Must be called on the thread that runs the Win32 message loop -- see the
/// module docs.
pub fn build(tooltip: &str, status_line: Option<&str>) -> Result<TrayHandles, tray_icon::Error> {
    let menu = Menu::new();
    let show_item = MenuItem::new("Show", true, None);
    let quit_item = MenuItem::new("Quit", true, None);
    let show_id = show_item.id().clone();
    let quit_id = quit_item.id().clone();
    menu.append(&show_item)
        .expect("appending a freshly-created MenuItem to a freshly-created Menu cannot fail");
    menu.append(&quit_item)
        .expect("appending a freshly-created MenuItem to a freshly-created Menu cannot fail");
    if let Some(status_line) = status_line {
        // `enabled: false` — informational only, not a clickable command.
        let status_item = MenuItem::new(status_line, false, None);
        menu.append(&status_item)
            .expect("appending a freshly-created MenuItem to a freshly-created Menu cannot fail");
    }

    let tray_icon = TrayIconBuilder::new()
        .with_icon(load_icon())
        .with_menu(Box::new(menu))
        .with_tooltip(tooltip)
        // A left click emits a `TrayIconEvent::Click` instead of opening the
        // menu, so it can drive the toggle (D-17).
        .with_menu_on_left_click(false)
        .build()?;

    Ok(TrayHandles {
        tray_icon: Some(tray_icon),
        show_id,
        quit_id,
    })
}

/// A pure id -> command mapping, unit-testable without a live tray.
pub fn command_for_menu(id: &MenuId, handles: &TrayHandles) -> Option<OverlayCommand> {
    if *id == handles.show_id {
        Some(OverlayCommand::Show)
    } else if *id == handles.quit_id {
        Some(OverlayCommand::Quit)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    impl TrayHandles {
        /// Test-only constructor: known ids, no live OS tray. Lets
        /// `command_for_menu` be exercised headlessly, matching this
        /// module's stated design (a pure mapping, testable without a tray).
        fn for_test(show_id: MenuId, quit_id: MenuId) -> Self {
            Self {
                tray_icon: None,
                show_id,
                quit_id,
            }
        }
    }

    #[test]
    fn command_for_menu_maps_show_and_quit_and_rejects_unknown() {
        let handles = TrayHandles::for_test(MenuId::new("show"), MenuId::new("quit"));

        assert_eq!(
            command_for_menu(&MenuId::new("show"), &handles),
            Some(OverlayCommand::Show)
        );
        assert_eq!(
            command_for_menu(&MenuId::new("quit"), &handles),
            Some(OverlayCommand::Quit)
        );
        assert_eq!(command_for_menu(&MenuId::new("something-else"), &handles), None);
    }
}
