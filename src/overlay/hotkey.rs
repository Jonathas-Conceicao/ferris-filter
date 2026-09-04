//! Global `Shift+Z` toggle hotkey registration, and a status value a caller
//! can surface without a console (D-09, D-10).
//!
//! Thread affinity: `register_toggle()` must be called on the thread that
//! runs the Win32 message loop -- the same constraint `tray::build` documents
//! (RESEARCH "Architecture Patterns" -> Pattern 3). Call it alongside the
//! tray build, before `eframe::run_native` starts the loop.

use global_hotkey::hotkey::{Code, HotKey, Modifiers};
use global_hotkey::GlobalHotKeyManager;

/// `Shift+Z`, hardcoded this phase (D-09).
///
/// **Accepted trade:** `global-hotkey` registers system-wide, so this combo
/// is captured in *every* application on the machine while the overlay is
/// running — including PoE2's own chat box and any text field elsewhere on
/// the machine. Changing it is a one-constant edit (e.g. `Ctrl+Shift+Z`,
/// `Alt+Z`) if it proves annoying in play.
pub const TOGGLE_MODIFIERS: Modifiers = Modifiers::SHIFT;
pub const TOGGLE_CODE: Code = Code::KeyZ;

/// Outcome of attempting to register the toggle hotkey.
///
/// A value, not a printed message — this crate's rule is that the library
/// reports and the caller presents (here, the tray tooltip/menu, D-10).
#[derive(Debug, Clone)]
pub enum HotkeyStatus {
    /// Registration succeeded; `id` matches `GlobalHotKeyEvent::id` for a
    /// press of this combo.
    Registered { id: u32 },
    /// Registration failed; the text names the reason (e.g. another
    /// application already owns the combo).
    Failed(String),
}

/// Registers the toggle hotkey.
///
/// Never panics: another application already owning `Shift+Z` is an
/// expected outcome, not a crash (D-10).
///
/// Returns the manager alongside the status because the manager must be kept
/// alive for the process lifetime — dropping it unregisters the hotkey. On
/// failure there is nothing to keep alive, so the first element is `None`.
pub fn register_toggle() -> (Option<GlobalHotKeyManager>, HotkeyStatus) {
    let manager = match GlobalHotKeyManager::new() {
        Ok(manager) => manager,
        Err(err) => return (None, HotkeyStatus::Failed(err.to_string())),
    };

    let hotkey = HotKey::new(Some(TOGGLE_MODIFIERS), TOGGLE_CODE);
    match manager.register(hotkey) {
        Ok(()) => {
            let id = hotkey.id();
            (Some(manager), HotkeyStatus::Registered { id })
        }
        Err(err) => (None, HotkeyStatus::Failed(err.to_string())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn toggle_combo_is_shift_z() {
        // D-09: stays a single-constant change if this ever needs to move.
        assert_eq!(TOGGLE_MODIFIERS, Modifiers::SHIFT);
        assert_eq!(TOGGLE_CODE, Code::KeyZ);
    }

    #[test]
    fn failed_status_carries_non_empty_text() {
        let status = HotkeyStatus::Failed("combo already registered".to_string());
        match status {
            HotkeyStatus::Failed(msg) => assert!(!msg.is_empty()),
            HotkeyStatus::Registered { .. } => panic!("expected Failed"),
        }
    }
}
