//! The show / hide hotkey (configurable, default `Ctrl+Shift+Z`): registration through the
//! [`HotkeyRegistrar`] seam, the live [`HotkeyRuntime`] the overlay keeps (start-up fallback,
//! register-new-before-unregister-old swap at Apply, release at Quit), and a status value a
//! caller can surface without a console.
//!
//! Thread affinity: `GlobalHotKeyManager::new()` creates a hidden window on the calling thread,
//! and Windows refuses `RegisterHotKey` for a window created by another thread. Start-up
//! registration and every swap at Apply must run on the main thread that created the manager
//! (the Win32 message loop thread, the same constraint `tray::build` documents). Never move
//! registration to a worker.

use global_hotkey::GlobalHotKeyManager;
use global_hotkey::hotkey::HotKey;

use crate::overlay::chord::{self, Chord};

/// Why a chord could not be registered.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RegisterError {
    /// Another application (or this one) already owns the keystrokes.
    InUse,
    /// Any other failure, with its text.
    Other(String),
}

/// Registers and unregisters chords. The real implementation is `GlobalHotKeyManager`; tests use
/// a fake, so no test ever registers a real OS hotkey.
pub trait HotkeyRegistrar {
    fn register(&mut self, chord: Chord) -> Result<(), RegisterError>;
    /// Gives the chord up; a failure means it is already gone and is ignored.
    fn unregister(&mut self, chord: Chord);
}

impl HotkeyRegistrar for GlobalHotKeyManager {
    fn register(&mut self, chord: Chord) -> Result<(), RegisterError> {
        let hotkey = HotKey::new(Some(chord.modifiers()), chord.code());
        match GlobalHotKeyManager::register(self, hotkey) {
            Ok(()) => Ok(()),
            Err(global_hotkey::Error::AlreadyRegistered(_)) => Err(RegisterError::InUse),
            Err(err) => Err(RegisterError::Other(err.to_string())),
        }
    }

    fn unregister(&mut self, chord: Chord) {
        let _ = GlobalHotKeyManager::unregister(
            self,
            HotKey::new(Some(chord.modifiers()), chord.code()),
        );
    }
}

/// Reason shown when no registrar exists (the manager could not be created).
const NO_SERVICE: &str = "the hotkey service is unavailable";
/// Reason shown when a chord is owned elsewhere.
const IN_USE: &str = "in use by another app";

/// The live show / hide hotkey: which chord is registered (if any), which chord the UI calls the
/// applied one, and why registration failed. Owns the registrar, so keeping the runtime keeps the
/// hotkey; a failed registration keeps the registrar so a later Apply can still register.
pub struct HotkeyRuntime {
    registrar: Option<Box<dyn HotkeyRegistrar>>,
    live: Option<Chord>,
    shown: Chord,
    failure: Option<String>,
}

impl std::fmt::Debug for HotkeyRuntime {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HotkeyRuntime")
            .field("registrar", &self.registrar.is_some())
            .field("live", &self.live)
            .field("shown", &self.shown)
            .field("failure", &self.failure)
            .finish()
    }
}

impl HotkeyRuntime {
    /// Registers the saved chord (absent: `Ctrl+Shift+Z`). A saved chord that cannot be
    /// registered falls back to `Ctrl+Shift+Z`, with a warning for the status line. When nothing
    /// registers, the runtime holds no live chord and [`HotkeyRuntime::failure_line`] names the
    /// chord tried last. Never panics.
    pub fn startup(
        registrar: Option<Box<dyn HotkeyRegistrar>>,
        saved: Option<Chord>,
    ) -> (HotkeyRuntime, Option<String>) {
        let wanted = saved.unwrap_or(chord::DEFAULT);
        let Some(mut reg) = registrar else {
            return (HotkeyRuntime::inert(wanted, NO_SERVICE), None);
        };
        let first = reg.register(wanted);
        let mut rt = HotkeyRuntime {
            registrar: None,
            live: None,
            shown: wanted,
            failure: None,
        };
        let warning = match first {
            Ok(()) => {
                rt.live = Some(wanted);
                None
            }
            Err(err) if wanted == chord::DEFAULT => {
                rt.failure = Some(reason(&err));
                None
            }
            Err(err) => match reg.register(chord::DEFAULT) {
                Ok(()) => {
                    rt.live = Some(chord::DEFAULT);
                    rt.shown = chord::DEFAULT;
                    Some(match err {
                        RegisterError::InUse => format!(
                            "`hotkey` {wanted} is in use by another app, so {} is used",
                            chord::DEFAULT
                        ),
                        RegisterError::Other(e) => format!(
                            "`hotkey` {wanted} could not be registered ({e}), so {} is used",
                            chord::DEFAULT
                        ),
                    })
                }
                Err(err) => {
                    rt.shown = chord::DEFAULT;
                    rt.failure = Some(reason(&err));
                    None
                }
            },
        };
        rt.registrar = Some(reg);
        (rt, warning)
    }

    /// A runtime that registers nothing (screenshot mode, no manager): shows `shown` and reports
    /// `reason` as the failure.
    pub fn inert(shown: Chord, reason: &str) -> HotkeyRuntime {
        HotkeyRuntime {
            registrar: None,
            live: None,
            shown,
            failure: Some(reason.to_string()),
        }
    }

    /// The registered chord, if any.
    pub fn live(&self) -> Option<Chord> {
        self.live
    }

    /// Whether a later Apply can register a chord at all (a registrar exists): false for an
    /// inert runtime and after [`HotkeyRuntime::release`].
    pub fn can_register(&self) -> bool {
        self.registrar.is_some()
    }

    /// The `GlobalHotKeyEvent::id` of a press of the registered chord.
    pub fn live_id(&self) -> Option<u32> {
        self.live.map(|c| c.id())
    }

    /// The chord the UI calls the applied one: the registered chord, else the one tried last.
    pub fn shown(&self) -> Chord {
        self.shown
    }

    /// First half of an Apply swap: registers `new` while the old chord stays registered, so a
    /// refusal leaves the old binding untouched. `new` equal to the live chord is a no-op (the OS
    /// would refuse the same keystrokes twice). The error is the finished status sentence.
    pub fn begin_swap(&mut self, new: Chord) -> Result<(), String> {
        if self.live == Some(new) {
            return Ok(());
        }
        let Some(reg) = self.registrar.as_mut() else {
            return Err(format!(
                "{new} could not be registered ({NO_SERVICE}); nothing was applied."
            ));
        };
        match (reg.register(new), self.live) {
            (Ok(()), _) => Ok(()),
            (Err(RegisterError::InUse), Some(old)) => Err(format!(
                "{new} is in use by another app. {old} still works; nothing was applied."
            )),
            (Err(RegisterError::InUse), None) => Err(format!(
                "{new} is in use by another app; nothing was applied."
            )),
            (Err(RegisterError::Other(e)), Some(old)) => Err(format!(
                "{new} could not be registered ({e}). {old} still works; nothing was applied."
            )),
            (Err(RegisterError::Other(e)), None) => Err(format!(
                "{new} could not be registered ({e}); nothing was applied."
            )),
        }
    }

    /// Second half of a successful swap (every write succeeded): unregisters the old chord and
    /// makes `new` the live one.
    pub fn finish_swap(&mut self, new: Chord) {
        if self.live == Some(new) {
            return;
        }
        if let (Some(reg), Some(old)) = (self.registrar.as_mut(), self.live) {
            reg.unregister(old);
        }
        self.live = Some(new);
        self.shown = new;
        self.failure = None;
    }

    /// A failed swap (a write failed after [`HotkeyRuntime::begin_swap`]): gives `new` back and
    /// keeps the old chord.
    pub fn abort_swap(&mut self, new: Chord) {
        if self.live == Some(new) {
            return;
        }
        if let Some(reg) = self.registrar.as_mut() {
            reg.unregister(new);
        }
    }

    /// Gives up the live chord and drops the registrar (tray Quit), so a relaunch can register
    /// it. The live chord is unregistered explicitly: `global-hotkey`'s `Drop` only destroys its
    /// hidden window. A second call does nothing.
    pub fn release(&mut self) {
        if let Some(mut reg) = self.registrar.take()
            && let Some(live) = self.live.take()
        {
            reg.unregister(live);
        }
        self.live = None;
    }

    /// The tray tooltip: `FerrisFilter — <chord> to toggle`, else the failure line.
    pub fn tooltip(&self) -> String {
        match self.failure_line() {
            Some(line) => line,
            None => format!("FerrisFilter — {} to toggle", self.shown),
        }
    }

    /// `FerrisFilter — <chord> unavailable: <reason>` when no chord is registered.
    pub fn failure_line(&self) -> Option<String> {
        if self.live.is_some() {
            return None;
        }
        let reason = self.failure.as_deref().unwrap_or(NO_SERVICE);
        Some(format!(
            "FerrisFilter — {} unavailable: {reason}",
            self.shown
        ))
    }
}

fn reason(err: &RegisterError) -> String {
    match err {
        RegisterError::InUse => IN_USE.to_string(),
        RegisterError::Other(e) => e.clone(),
    }
}

/// A registrar for tests: refuses chords in `taken` with `InUse`, and logs every call as
/// `register <chord>` / `unregister <chord>`.
#[cfg(test)]
pub(crate) struct FakeRegistrar {
    pub taken: std::collections::HashSet<Chord>,
    pub log: std::rc::Rc<std::cell::RefCell<Vec<String>>>,
}

#[cfg(test)]
impl HotkeyRegistrar for FakeRegistrar {
    fn register(&mut self, chord: Chord) -> Result<(), RegisterError> {
        self.log.borrow_mut().push(format!("register {chord}"));
        if self.taken.contains(&chord) {
            Err(RegisterError::InUse)
        } else {
            Ok(())
        }
    }

    fn unregister(&mut self, chord: Chord) {
        self.log.borrow_mut().push(format!("unregister {chord}"));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_chord_is_ctrl_shift_z() {
        assert_eq!(chord::DEFAULT.text(), "Ctrl+Shift+Z");
    }

    use crate::overlay::chord::{ChordKey, DEFAULT};
    use std::cell::RefCell;
    use std::collections::HashSet;
    use std::rc::Rc;

    type Log = Rc<RefCell<Vec<String>>>;

    const CTRL_ALT_K: Chord = Chord {
        ctrl: true,
        shift: false,
        alt: true,
        key: ChordKey::Letter(b'K'),
    };
    const ALT_J: Chord = Chord {
        ctrl: false,
        shift: false,
        alt: true,
        key: ChordKey::Letter(b'J'),
    };

    fn fake(taken: &[Chord]) -> (Box<dyn HotkeyRegistrar>, Log) {
        let log: Log = Rc::default();
        let reg = FakeRegistrar {
            taken: taken.iter().copied().collect::<HashSet<_>>(),
            log: log.clone(),
        };
        (Box::new(reg), log)
    }

    fn entries(log: &Log) -> Vec<String> {
        log.borrow().clone()
    }

    #[test]
    fn startup_default_registers_ctrl_shift_z() {
        let (reg, log) = fake(&[]);
        let (rt, w) = HotkeyRuntime::startup(Some(reg), None);
        assert_eq!(w, None);
        assert_eq!(rt.live(), Some(DEFAULT));
        assert_eq!(rt.live_id(), Some(DEFAULT.id()));
        assert_eq!(rt.shown(), DEFAULT);
        assert_eq!(entries(&log), ["register Ctrl+Shift+Z"]);
        assert_eq!(rt.tooltip(), "FerrisFilter — Ctrl+Shift+Z to toggle");
        assert_eq!(rt.failure_line(), None);
    }

    #[test]
    fn startup_saved_chord_registers() {
        let (reg, log) = fake(&[]);
        let (rt, w) = HotkeyRuntime::startup(Some(reg), Some(CTRL_ALT_K));
        assert_eq!(w, None);
        assert_eq!(rt.live(), Some(CTRL_ALT_K));
        assert_eq!(rt.shown(), CTRL_ALT_K);
        assert_eq!(entries(&log), ["register Ctrl+Alt+K"]);
    }

    #[test]
    fn startup_saved_chord_taken_falls_back_with_warning() {
        let (reg, log) = fake(&[CTRL_ALT_K]);
        let (rt, w) = HotkeyRuntime::startup(Some(reg), Some(CTRL_ALT_K));
        assert_eq!(
            w.as_deref(),
            Some("`hotkey` Ctrl+Alt+K is in use by another app, so Ctrl+Shift+Z is used")
        );
        assert_eq!(rt.live(), Some(DEFAULT));
        assert_eq!(rt.shown(), DEFAULT);
        assert_eq!(
            entries(&log),
            ["register Ctrl+Alt+K", "register Ctrl+Shift+Z"]
        );
    }

    #[test]
    fn startup_saved_default_taken_is_not_retried() {
        let (reg, log) = fake(&[DEFAULT]);
        let (rt, w) = HotkeyRuntime::startup(Some(reg), Some(DEFAULT));
        assert_eq!(w, None);
        assert_eq!(rt.live(), None);
        assert_eq!(entries(&log), ["register Ctrl+Shift+Z"]);
    }

    #[test]
    fn startup_both_taken_fails_naming_the_default() {
        let (reg, _log) = fake(&[CTRL_ALT_K, DEFAULT]);
        let (rt, w) = HotkeyRuntime::startup(Some(reg), Some(CTRL_ALT_K));
        assert_eq!(w, None);
        assert_eq!(rt.live(), None);
        assert_eq!(rt.live_id(), None);
        assert_eq!(rt.shown(), DEFAULT);
        let line = rt.failure_line().unwrap();
        assert_eq!(
            line,
            format!("FerrisFilter — Ctrl+Shift+Z unavailable: {IN_USE}")
        );
        assert!(line.starts_with("FerrisFilter — Ctrl+Shift+Z unavailable: "));
        assert_eq!(rt.tooltip(), line);
    }

    #[test]
    fn startup_without_a_manager_registers_nothing() {
        let (rt, w) = HotkeyRuntime::startup(None, Some(CTRL_ALT_K));
        assert_eq!(w, None);
        assert_eq!(rt.live(), None);
        let line = rt.failure_line().unwrap();
        assert!(
            line.contains("unavailable: the hotkey service is unavailable"),
            "{line}"
        );
    }

    #[test]
    fn swap_registers_new_before_unregistering_old() {
        let (reg, log) = fake(&[]);
        let (mut rt, _) = HotkeyRuntime::startup(Some(reg), None);
        log.borrow_mut().clear();
        assert_eq!(rt.begin_swap(CTRL_ALT_K), Ok(()));
        assert_eq!(entries(&log), ["register Ctrl+Alt+K"]);
        assert_eq!(rt.live(), Some(DEFAULT));
        rt.finish_swap(CTRL_ALT_K);
        assert_eq!(
            entries(&log),
            ["register Ctrl+Alt+K", "unregister Ctrl+Shift+Z"]
        );
        assert_eq!(rt.live(), Some(CTRL_ALT_K));
        assert_eq!(rt.live_id(), Some(CTRL_ALT_K.id()));
        assert_eq!(rt.tooltip(), "FerrisFilter — Ctrl+Alt+K to toggle");
    }

    #[test]
    fn swap_to_a_taken_chord_is_refused_and_keeps_the_old_one() {
        let (reg, log) = fake(&[CTRL_ALT_K]);
        let (mut rt, _) = HotkeyRuntime::startup(Some(reg), None);
        log.borrow_mut().clear();
        assert_eq!(
            rt.begin_swap(CTRL_ALT_K),
            Err("Ctrl+Alt+K is in use by another app. Ctrl+Shift+Z still works; nothing was applied."
                .to_string())
        );
        assert!(!entries(&log).iter().any(|e| e.starts_with("unregister")));
        assert_eq!(rt.live(), Some(DEFAULT));
    }

    #[test]
    fn swap_refusal_without_a_live_chord() {
        let (reg, _log) = fake(&[DEFAULT, CTRL_ALT_K]);
        let (mut rt, _) = HotkeyRuntime::startup(Some(reg), None);
        assert_eq!(
            rt.begin_swap(CTRL_ALT_K),
            Err("Ctrl+Alt+K is in use by another app; nothing was applied.".to_string())
        );
    }

    #[test]
    fn swap_from_failed_start_up_registers_and_clears_the_failure() {
        let (reg, log) = fake(&[DEFAULT]);
        let (mut rt, _) = HotkeyRuntime::startup(Some(reg), None);
        assert!(rt.failure_line().is_some());
        assert_eq!(rt.begin_swap(ALT_J), Ok(()));
        rt.finish_swap(ALT_J);
        assert_eq!(rt.live(), Some(ALT_J));
        assert_eq!(rt.failure_line(), None);
        assert!(!entries(&log).iter().any(|e| e.starts_with("unregister")));
    }

    #[test]
    fn swap_to_the_live_chord_makes_no_call() {
        let (reg, log) = fake(&[]);
        let (mut rt, _) = HotkeyRuntime::startup(Some(reg), None);
        log.borrow_mut().clear();
        assert_eq!(rt.begin_swap(DEFAULT), Ok(()));
        rt.finish_swap(DEFAULT);
        rt.abort_swap(DEFAULT);
        assert!(entries(&log).is_empty());
        assert_eq!(rt.live(), Some(DEFAULT));
    }

    #[test]
    fn abort_swap_releases_only_the_new_chord() {
        let (reg, log) = fake(&[]);
        let (mut rt, _) = HotkeyRuntime::startup(Some(reg), None);
        log.borrow_mut().clear();
        rt.begin_swap(CTRL_ALT_K).unwrap();
        rt.abort_swap(CTRL_ALT_K);
        assert_eq!(
            entries(&log),
            ["register Ctrl+Alt+K", "unregister Ctrl+Alt+K"]
        );
        assert_eq!(rt.live(), Some(DEFAULT));
    }

    #[test]
    fn swap_without_a_registrar_is_refused() {
        let mut rt = HotkeyRuntime::inert(DEFAULT, "screenshot mode");
        let err = rt.begin_swap(CTRL_ALT_K).unwrap_err();
        assert!(err.contains("nothing was applied"), "{err}");
        assert!(
            err.starts_with("Ctrl+Alt+K could not be registered"),
            "{err}"
        );
    }

    #[test]
    fn release_unregisters_the_live_chord_once() {
        let (reg, log) = fake(&[]);
        let (mut rt, _) = HotkeyRuntime::startup(Some(reg), Some(CTRL_ALT_K));
        log.borrow_mut().clear();
        rt.release();
        assert_eq!(entries(&log), ["unregister Ctrl+Alt+K"]);
        assert_eq!(rt.live(), None);
        rt.release();
        assert_eq!(entries(&log), ["unregister Ctrl+Alt+K"]);
    }

    #[test]
    fn inert_runtime_shows_without_registering() {
        let rt = HotkeyRuntime::inert(CTRL_ALT_K, "screenshot mode");
        assert_eq!(rt.live(), None);
        assert_eq!(rt.shown(), CTRL_ALT_K);
        assert_eq!(
            rt.failure_line().as_deref(),
            Some("FerrisFilter — Ctrl+Alt+K unavailable: screenshot mode")
        );
    }
}
