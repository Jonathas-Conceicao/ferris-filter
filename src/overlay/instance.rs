//! Single-instance gate for the `ff` overlay (INST-01, Phase 22).
//!
//! A second `ff` launch reveals the running stripe and exits; it never opens a
//! second stripe.
//!
//! Mechanism: a named mutex in the `Local\` namespace is held for the process
//! lifetime and released by the OS on exit or crash, so there is no stale-lock
//! state. An auto-reset `-show` event is polled by the first instance each frame;
//! an auto-reset `-ack` event is waited on by the second for [`ACK_TIMEOUT_MS`].
//! If the first does not answer, a MessageBox is shown (the release `ff` has no
//! console).
//!
//! This module is the second home of overlay `unsafe` alongside `win.rs`,
//! extending the T-13-09 "raw Win32 isolated" convention; every `unsafe` block
//! carries a `// SAFETY:` comment.
//!
//! Debug builds use a separate instance name so `cargo run` can run beside a
//! release `ff`.

/// Fixed base name (GUID literal avoids accidental collisions).
const RELEASE_BASE: &str = "Local\\FerrisFilter-7c1d4e0a-3b52-4f6e-9a8d-2f0c5b9e1a47-overlay";

/// How long a second instance waits for the first to acknowledge, in ms.
pub const ACK_TIMEOUT_MS: u32 = 2000;

/// Shown when the first instance does not answer the show signal.
pub const NOT_RESPONDING_MESSAGE: &str = "FerrisFilter is already running but not responding";

/// WAIT_OBJECT_0: the only wait result that counts as an answer.
const WAIT_OBJECT_0_CODE: u32 = 0;

/// Names of the three kernel objects making up one instance gate.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstanceNames {
    pub mutex: String,
    pub show: String,
    pub ack: String,
}

/// Build the three object names from a base.
pub fn names_for(base: &str) -> InstanceNames {
    InstanceNames {
        mutex: base.to_string(),
        show: format!("{base}-show"),
        ack: format!("{base}-ack"),
    }
}

/// Names for the debug or release profile.
pub fn names(debug: bool) -> InstanceNames {
    if debug {
        names_for(&format!("{RELEASE_BASE}-debug"))
    } else {
        names_for(RELEASE_BASE)
    }
}

/// Names for the current build profile.
pub fn instance_names() -> InstanceNames {
    names(cfg!(debug_assertions))
}

/// Outcome of waiting for the first instance's ack.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Wait {
    Signaled,
    TimedOut,
}

/// Map a `WaitForSingleObject` code. Anything but WAIT_OBJECT_0 (timeout,
/// failure, abandoned) is "not responding", which only ever leads to the message
/// box, never to a second stripe.
pub fn classify_wait(code: u32) -> Wait {
    if code == WAIT_OBJECT_0_CODE {
        Wait::Signaled
    } else {
        Wait::TimedOut
    }
}

/// Result of trying to become the single instance.
pub enum Acquired {
    First(Guard),
    AlreadyRunning,
}

#[cfg(windows)]
mod imp {
    use super::{
        ACK_TIMEOUT_MS, Acquired, InstanceNames, NOT_RESPONDING_MESSAGE, Wait, classify_wait,
        instance_names,
    };
    use windows_sys::Win32::Foundation::{
        CloseHandle, ERROR_ACCESS_DENIED, ERROR_ALREADY_EXISTS, GetLastError, HANDLE,
    };
    use windows_sys::Win32::System::Threading::{
        CreateEventW, CreateMutexW, ResetEvent, SetEvent, WaitForSingleObject,
    };
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        MB_ICONINFORMATION, MB_OK, MB_SETFOREGROUND, MB_TOPMOST, MessageBoxW,
    };

    fn wide(s: &str) -> Vec<u16> {
        s.encode_utf16().chain(std::iter::once(0)).collect()
    }

    /// Held for the process lifetime; dropping releases the single-instance name.
    pub struct Guard {
        mutex: HANDLE,
        show: HANDLE,
        ack: HANDLE,
    }

    // SAFETY: kernel handles are process-wide and usable from any thread.
    unsafe impl Send for Guard {}

    impl Drop for Guard {
        fn drop(&mut self) {
            for h in [self.show, self.ack, self.mutex] {
                if !h.is_null() {
                    // SAFETY: the handle came from Create* in this module and is
                    // closed exactly once, here.
                    unsafe { CloseHandle(h) };
                }
            }
        }
    }

    impl Guard {
        /// Poll (zero timeout) for a show request; auto-reset consumes it.
        pub fn take_show_request(&self) -> bool {
            if self.show.is_null() {
                return false;
            }
            // SAFETY: `show` is a live event handle owned by this Guard.
            let code = unsafe { WaitForSingleObject(self.show, 0) };
            classify_wait(code) == Wait::Signaled
        }

        /// Tell a waiting second instance that the show request was handled.
        pub fn ack(&self) {
            if !self.ack.is_null() {
                // SAFETY: `ack` is a live event handle owned by this Guard.
                unsafe { SetEvent(self.ack) };
            }
        }
    }

    fn create_event(name: &[u16]) -> HANDLE {
        // SAFETY: `name` is a NUL-terminated UTF-16 buffer alive across the call;
        // null security attributes give the default same-user DACL.
        unsafe { CreateEventW(std::ptr::null(), 0, 0, name.as_ptr()) }
    }

    pub fn acquire_named(names: &InstanceNames) -> Acquired {
        // Build every wide name before CreateMutexW so nothing runs between it
        // and GetLastError.
        let mutex_name = wide(&names.mutex);
        let show_name = wide(&names.show);
        let ack_name = wide(&names.ack);

        // SAFETY: `mutex_name` is NUL-terminated and alive across the call.
        let mutex = unsafe { CreateMutexW(std::ptr::null(), 0, mutex_name.as_ptr()) };
        // SAFETY: plain thread-local error read, immediately after the call.
        let err = unsafe { GetLastError() };

        if mutex.is_null() {
            // An elevated first instance denies access to the object: treat as
            // running (the signal path then ends in the not-responding message,
            // never a second stripe).
            if err == ERROR_ACCESS_DENIED {
                return Acquired::AlreadyRunning;
            }
            // Fail open: refusing to start over an exotic kernel-object error is
            // worse than a rare duplicate.
            eprintln!("single-instance: CreateMutexW failed (error {err}); continuing");
            return Acquired::First(Guard {
                mutex,
                show: std::ptr::null_mut(),
                ack: std::ptr::null_mut(),
            });
        }
        if err == ERROR_ALREADY_EXISTS {
            // SAFETY: handle just returned by CreateMutexW, closed once.
            unsafe { CloseHandle(mutex) };
            return Acquired::AlreadyRunning;
        }

        let show = create_event(&show_name);
        let ack = create_event(&ack_name);
        if show.is_null() || ack.is_null() {
            eprintln!("single-instance: CreateEventW failed; show requests disabled");
        }
        Acquired::First(Guard { mutex, show, ack })
    }

    pub fn signal_running(names: &InstanceNames, timeout_ms: u32) -> Wait {
        let show = create_event(&wide(&names.show));
        let ack = create_event(&wide(&names.ack));
        let result = if show.is_null() || ack.is_null() {
            Wait::TimedOut
        } else {
            // SAFETY: both handles are live events created above. Reset the ack
            // BEFORE signalling show so a stale ack from an earlier timed-out
            // launch is dropped.
            let code = unsafe {
                ResetEvent(ack);
                SetEvent(show);
                WaitForSingleObject(ack, timeout_ms)
            };
            classify_wait(code)
        };
        for h in [show, ack] {
            if !h.is_null() {
                // SAFETY: created above, closed once.
                unsafe { CloseHandle(h) };
            }
        }
        result
    }

    pub fn notify_unresponsive() {
        let text = wide(NOT_RESPONDING_MESSAGE);
        let title = wide("FerrisFilter");
        // SAFETY: both buffers are NUL-terminated and alive across the call; a
        // null owner window is allowed.
        unsafe {
            MessageBoxW(
                std::ptr::null_mut(),
                text.as_ptr(),
                title.as_ptr(),
                MB_OK | MB_ICONINFORMATION | MB_SETFOREGROUND | MB_TOPMOST,
            );
        }
    }

    /// Composed entry for `launch()`: `Some(guard)` = run the stripe, `None` =
    /// exit 0. Call before creating the hotkey or the tray, and keep the Guard
    /// alive until exit.
    pub fn acquire() -> Option<Guard> {
        match acquire_named(&instance_names()) {
            Acquired::First(g) => Some(g),
            Acquired::AlreadyRunning => {
                if signal_running(&instance_names(), ACK_TIMEOUT_MS) == Wait::TimedOut {
                    notify_unresponsive();
                }
                None
            }
        }
    }
}

#[cfg(not(windows))]
mod imp {
    use super::{Acquired, InstanceNames, Wait};

    /// Non-Windows stand-in; the overlay is not supported off Windows.
    pub struct Guard;

    impl Guard {
        pub fn take_show_request(&self) -> bool {
            false
        }
        pub fn ack(&self) {}
    }

    pub fn acquire_named(_names: &InstanceNames) -> Acquired {
        Acquired::First(Guard)
    }

    pub fn signal_running(_names: &InstanceNames, _timeout_ms: u32) -> Wait {
        Wait::TimedOut
    }

    pub fn notify_unresponsive() {}

    /// No single-instance enforcement off Windows, where the overlay is not
    /// supported anyway.
    pub fn acquire() -> Option<Guard> {
        Some(Guard)
    }
}

pub use imp::{Guard, acquire, acquire_named, notify_unresponsive, signal_running};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn release_and_debug_names_differ() {
        assert_ne!(names(true), names(false));
        assert!(names(true).mutex.ends_with("-overlay-debug"));
        assert!(names(false).mutex.ends_with("-overlay"));
    }

    #[test]
    fn names_live_in_the_local_namespace() {
        for n in [names(true), names(false)] {
            for s in [&n.mutex, &n.show, &n.ack] {
                assert!(s.starts_with("Local\\"), "{s}");
            }
        }
    }

    #[test]
    fn show_and_ack_share_the_mutex_base() {
        let n = names_for("Local\\x");
        assert_eq!(n.mutex, "Local\\x");
        assert_eq!(n.show, "Local\\x-show");
        assert_eq!(n.ack, "Local\\x-ack");
        assert_ne!(n.mutex, n.show);
        assert_ne!(n.show, n.ack);
        assert_ne!(n.mutex, n.ack);
    }

    #[test]
    fn instance_names_follow_the_build_profile() {
        assert_eq!(instance_names(), names(cfg!(debug_assertions)));
    }

    #[test]
    fn timeout_is_about_two_seconds() {
        assert_eq!(ACK_TIMEOUT_MS, 2000);
    }

    #[test]
    fn only_wait_object_0_counts_as_an_answer() {
        assert_eq!(classify_wait(0), Wait::Signaled);
        assert_eq!(classify_wait(258), Wait::TimedOut);
        assert_eq!(classify_wait(0x80), Wait::TimedOut);
        assert_eq!(classify_wait(0xFFFF_FFFF), Wait::TimedOut);
    }

    #[test]
    fn not_responding_message_is_the_locked_text() {
        assert_eq!(
            NOT_RESPONDING_MESSAGE,
            "FerrisFilter is already running but not responding"
        );
    }
}

#[cfg(all(test, windows))]
mod windows_tests {
    use super::*;
    use std::sync::mpsc;
    use std::time::{Duration, Instant};

    /// Unique per-process names; never the production ones, so a running ff is
    /// never signalled or blocked by tests.
    fn test_names(label: &str) -> InstanceNames {
        names_for(&format!(
            "Local\\FerrisFilter-test-{}-{label}",
            std::process::id()
        ))
    }

    #[test]
    fn second_acquire_in_process_reports_already_running() {
        let n = test_names("second");
        let first = acquire_named(&n);
        assert!(matches!(first, Acquired::First(_)));
        assert!(matches!(acquire_named(&n), Acquired::AlreadyRunning));
        drop(first);
    }

    #[test]
    fn dropping_the_guard_releases_the_name() {
        let n = test_names("drop");
        let first = acquire_named(&n);
        assert!(matches!(first, Acquired::First(_)));
        drop(first);
        assert!(matches!(acquire_named(&n), Acquired::First(_)));
    }

    #[test]
    fn a_responding_first_instance_acks_the_show_signal() {
        let n = test_names("responder");
        let n2 = n.clone();
        let (ready_tx, ready_rx) = mpsc::channel();
        let (seen_tx, seen_rx) = mpsc::channel();
        let t = std::thread::spawn(move || {
            let Acquired::First(g) = acquire_named(&n2) else {
                panic!("expected First");
            };
            ready_tx.send(()).unwrap();
            let start = Instant::now();
            let mut count = 0;
            while start.elapsed() < Duration::from_secs(3) {
                if g.take_show_request() {
                    count += 1;
                    g.ack();
                    break;
                }
                std::thread::sleep(Duration::from_millis(10));
            }
            seen_tx.send(count).unwrap();
        });
        ready_rx.recv().unwrap();
        assert_eq!(signal_running(&n, ACK_TIMEOUT_MS), Wait::Signaled);
        assert_eq!(seen_rx.recv().unwrap(), 1);
        t.join().unwrap();
    }

    #[test]
    fn an_unresponsive_first_instance_times_out() {
        let n = test_names("unresponsive");
        let _g = acquire_named(&n);
        assert_eq!(signal_running(&n, 100), Wait::TimedOut);
    }

    #[test]
    fn a_stale_ack_does_not_count_as_an_answer() {
        let n = test_names("stale");
        let Acquired::First(g) = acquire_named(&n) else {
            panic!("expected First");
        };
        g.ack();
        assert_eq!(signal_running(&n, 100), Wait::TimedOut);
    }

    #[test]
    fn no_show_request_without_a_signal() {
        let n = test_names("noreq");
        let Acquired::First(g) = acquire_named(&n) else {
            panic!("expected First");
        };
        assert!(!g.take_show_request());
        assert_eq!(signal_running(&n, 50), Wait::TimedOut);
        assert!(g.take_show_request());
        assert!(!g.take_show_request());
    }
}
