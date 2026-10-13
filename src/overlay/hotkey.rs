//! Global `Ctrl+Shift+Z` toggle hotkey registration, and a status value a caller
//! can surface without a console (D-09, D-10).
//!
//! Thread affinity: `register_toggle()` must be called on the thread that
//! runs the Win32 message loop -- the same constraint `tray::build` documents
//! (RESEARCH "Architecture Patterns" -> Pattern 3). Call it alongside the
//! tray build, before `eframe::run_native` starts the loop.

use global_hotkey::GlobalHotKeyManager;
use global_hotkey::hotkey::{Code, HotKey, Modifiers};

/// `Ctrl+Shift+Z`, hardcoded this phase (D-09).
///
/// Was plain `Shift+Z` through most of Phase 13. The author changed it after
/// in-game verification (2026-09-06): `global-hotkey` registers system-wide,
/// so `Shift+Z` was swallowed everywhere on the machine — including PoE2's
/// own chat box — which cost a common typing combo for no benefit. Adding
/// Control keeps the same mnemonic while leaving plain `Shift+Z` free.
///
/// The system-wide capture itself is inherent to a global hotkey, not a
/// defect: `Ctrl+Shift+Z` is still owned by this process everywhere while the
/// overlay runs (notably, it is redo in some editors). Changing it remains a
/// one-constant edit.
pub const TOGGLE_MODIFIERS: Modifiers = Modifiers::CONTROL.union(Modifiers::SHIFT);
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
/// Never panics: another application already owning `Ctrl+Shift+Z` is an
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

/// Gives up the toggle hotkey now, rather than when the process exits.
///
/// Tray Quit calls this before it releases the single-instance name, so a relaunch that wins
/// the name can register `Ctrl+Shift+Z` itself (review iteration 3, WR-01).
///
/// The hotkey is unregistered explicitly before the manager is dropped. `global-hotkey` 0.8's
/// `Drop` only calls `DestroyWindow` on its hidden window. Windows does release hotkeys bound
/// to a destroyed window, but the `RegisterHotKey` docs do not say so, so this does not rely
/// on it. Must run on the thread that registered the hotkey, the same as `register_toggle`.
pub fn release_toggle(manager: GlobalHotKeyManager) {
    // A failure here means the hotkey is already gone. Dropping the manager is still correct.
    let _ = manager.unregister(HotKey::new(Some(TOGGLE_MODIFIERS), TOGGLE_CODE));
    drop(manager);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn toggle_combo_is_shift_z() {
        // D-09: stays a single-constant change if this ever needs to move.
        assert_eq!(TOGGLE_MODIFIERS, Modifiers::CONTROL.union(Modifiers::SHIFT));
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
