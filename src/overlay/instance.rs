//! Single-instance gate for the `ff` overlay (INST-01, Phase 22).
//!
//! A second `ff` launch reveals the running stripe and exits; it never opens a
//! second stripe.
//!
//! Mechanism: a named mutex in the `Local\` namespace is held for the process
//! lifetime and released by the OS on exit or crash, so there is no stale-lock
//! state. A manual-reset `-ready` event is set on the first instance's first frame;
//! a second instance waits up to [`STARTUP_TIMEOUT_MS`] for it before asking. An
//! auto-reset `-show` event is polled by the first instance each frame; an
//! auto-reset `-ack` event is waited on by the second for [`ACK_TIMEOUT_MS`],
//! which re-asserts its request every [`RETRY_INTERVAL_MS`] so several launchers
//! waiting at once are each answered. If the first does not answer, the request is
//! withdrawn and a MessageBox is shown (the release `ff` has no console).
//!
//! This module is the second home of overlay `unsafe` alongside `win.rs`,
//! extending the T-13-09 "raw Win32 isolated" convention; every `unsafe` block
//! carries a `// SAFETY:` comment.
//!
//! Debug builds use a separate instance name so `cargo run` can run beside a
//! release `ff`.

/// Fixed base name (GUID literal avoids accidental collisions).
const RELEASE_BASE: &str = "Local\\FerrisFilter-7c1d4e0a-3b52-4f6e-9a8d-2f0c5b9e1a47-overlay";

/// How long a second instance waits for a ready first instance to acknowledge, in ms.
pub const ACK_TIMEOUT_MS: u32 = 2000;

/// How long a second instance waits for a still-starting first instance to become ready
/// (its first frame) before the [`ACK_TIMEOUT_MS`] window starts, in ms (WR-02). Startup
/// builds the tray, the window and the GPU context and loads and compiles the filter, which
/// can take longer than the ack window on a cold start.
pub const STARTUP_TIMEOUT_MS: u32 = 10_000;

/// How often a waiting second instance re-asserts its show request, in ms (WR-01).
pub const RETRY_INTERVAL_MS: u32 = 100;

/// Shown when the first instance does not answer the show signal.
pub const NOT_RESPONDING_MESSAGE: &str = "FerrisFilter is already running but not responding";

/// WAIT_OBJECT_0: the only wait result that counts as an answer.
const WAIT_OBJECT_0_CODE: u32 = 0;

/// Names of the four kernel objects making up one instance gate.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstanceNames {
    pub mutex: String,
    pub show: String,
    pub ack: String,
    pub ready: String,
}

/// Build the four object names from a base.
pub fn names_for(base: &str) -> InstanceNames {
    InstanceNames {
        mutex: base.to_string(),
        show: format!("{base}-show"),
        ack: format!("{base}-ack"),
        ready: format!("{base}-ready"),
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
        ACK_TIMEOUT_MS, Acquired, InstanceNames, NOT_RESPONDING_MESSAGE, RETRY_INTERVAL_MS,
        STARTUP_TIMEOUT_MS, Wait, classify_wait, instance_names,
    };
    use std::time::{Duration, Instant};
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
        /// Manual-reset; set once the first frame runs, i.e. once show requests are polled.
        ready: HANDLE,
        ready_marked: bool,
    }

    // SAFETY: kernel handles are process-wide and usable from any thread.
    unsafe impl Send for Guard {}

    impl Drop for Guard {
        fn drop(&mut self) {
            if !self.ready.is_null() {
                // SAFETY: `ready` is a live event handle owned by this Guard. An exiting
                // instance stops polling, so it withdraws its readiness first.
                unsafe { ResetEvent(self.ready) };
            }
            for h in [self.show, self.ack, self.ready, self.mutex] {
                if !h.is_null() {
                    // SAFETY: the handle came from Create* in this module and is
                    // closed exactly once, here.
                    unsafe { CloseHandle(h) };
                }
            }
        }
    }

    impl Guard {
        /// Announce that show requests are now polled (call on the first frame; later
        /// calls are no-ops). Until then a second launch waits up to
        /// [`STARTUP_TIMEOUT_MS`] instead of timing out after [`ACK_TIMEOUT_MS`].
        pub fn mark_ready(&mut self) {
            if self.ready_marked {
                return;
            }
            self.ready_marked = true;
            if !self.ready.is_null() {
                // SAFETY: `ready` is a live event handle owned by this Guard.
                unsafe { SetEvent(self.ready) };
            }
        }

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

    /// The `-ready` event is manual-reset (it stays set while the instance runs), and
    /// every party creates it with the same parameters, whoever comes first.
    fn create_ready_event(name: &[u16]) -> HANDLE {
        // SAFETY: as `create_event`; manual reset, initially unset.
        unsafe { CreateEventW(std::ptr::null(), 1, 0, name.as_ptr()) }
    }

    pub fn acquire_named(names: &InstanceNames) -> Acquired {
        // Build every wide name before CreateMutexW so nothing runs between it
        // and GetLastError.
        let mutex_name = wide(&names.mutex);
        let show_name = wide(&names.show);
        let ack_name = wide(&names.ack);
        let ready_name = wide(&names.ready);

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
                ready: std::ptr::null_mut(),
                ready_marked: false,
            });
        }
        if err == ERROR_ALREADY_EXISTS {
            // SAFETY: handle just returned by CreateMutexW, closed once.
            unsafe { CloseHandle(mutex) };
            return Acquired::AlreadyRunning;
        }

        let show = create_event(&show_name);
        let ack = create_event(&ack_name);
        let ready = create_ready_event(&ready_name);
        if show.is_null() || ack.is_null() || ready.is_null() {
            eprintln!("single-instance: CreateEventW failed; show requests disabled");
        }
        // The events may outlive a previous instance while a launcher still holds them
        // (e.g. mid-retry when it quit): start from a clean, not-ready state.
        for h in [show, ack, ready] {
            if !h.is_null() {
                // SAFETY: live event handles created just above.
                unsafe { ResetEvent(h) };
            }
        }
        Acquired::First(Guard {
            mutex,
            show,
            ack,
            ready,
            ready_marked: false,
        })
    }

    /// Re-assert the show request every [`RETRY_INTERVAL_MS`] until an ack arrives or
    /// `timeout_ms` passes (WR-01). Both events are auto-reset, so concurrent launchers'
    /// requests coalesce into one and each ack releases one waiter; re-asserting means every
    /// waiting launcher keeps a request pending until it receives an ack of its own.
    fn request_until_acked(show: HANDLE, ack: HANDLE, timeout_ms: u32) -> Wait {
        let deadline = Instant::now() + Duration::from_millis(u64::from(timeout_ms));
        let interval = Duration::from_millis(u64::from(RETRY_INTERVAL_MS));
        loop {
            // SAFETY: `show` is a live event handle owned by the caller.
            unsafe { SetEvent(show) };
            let slice = deadline
                .saturating_duration_since(Instant::now())
                .min(interval);
            // `slice` <= RETRY_INTERVAL_MS, so the cast cannot truncate.
            // SAFETY: `ack` is a live event handle owned by the caller.
            let code = unsafe { WaitForSingleObject(ack, slice.as_millis() as u32) };
            if classify_wait(code) == Wait::Signaled {
                return Wait::Signaled;
            }
            if Instant::now() >= deadline {
                return Wait::TimedOut;
            }
        }
    }

    /// The protocol over three live event handles owned by `signal_running`.
    fn request_show(
        show: HANDLE,
        ack: HANDLE,
        ready: HANDLE,
        startup_ms: u32,
        ack_ms: u32,
    ) -> Wait {
        // SAFETY: `ready` is a live event handle owned by the caller.
        let ready_code = unsafe { WaitForSingleObject(ready, startup_ms) };
        if classify_wait(ready_code) != Wait::Signaled {
            return Wait::TimedOut;
        }
        // SAFETY: `ack` is a live event handle owned by the caller. Reset it once, BEFORE
        // the first request, so a stale ack left by an earlier launch is dropped. A
        // concurrent launcher's reset can also wipe an ack meant for this one; the retry
        // loop makes that harmless, because the request is re-asserted and answered again
        // on the first instance's next frame.
        unsafe { ResetEvent(ack) };
        let result = request_until_acked(show, ack, ack_ms);
        if result == Wait::TimedOut {
            // SAFETY: `show` is a live event handle owned by the caller. Withdraw the
            // request so an instance that recovers later does not reveal the stripe after
            // the user was told it is not responding (IN-02).
            unsafe { ResetEvent(show) };
        }
        result
    }

    /// Ask the running instance to show its stripe. First wait up to `startup_ms` for it to
    /// be ready (WR-02: a launch during its startup is not "not responding"); no request is
    /// sent before then, so a never-ready instance is not left holding one. Then request and
    /// wait up to `ack_ms` for the ack.
    pub fn signal_running(names: &InstanceNames, startup_ms: u32, ack_ms: u32) -> Wait {
        let show = create_event(&wide(&names.show));
        let ack = create_event(&wide(&names.ack));
        let ready = create_ready_event(&wide(&names.ready));
        let result = if show.is_null() || ack.is_null() || ready.is_null() {
            Wait::TimedOut
        } else {
            request_show(show, ack, ready, startup_ms, ack_ms)
        };
        for h in [show, ack, ready] {
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
                let names = instance_names();
                if signal_running(&names, STARTUP_TIMEOUT_MS, ACK_TIMEOUT_MS) == Wait::TimedOut {
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
        pub fn mark_ready(&mut self) {}
        pub fn take_show_request(&self) -> bool {
            false
        }
        pub fn ack(&self) {}
    }

    pub fn acquire_named(_names: &InstanceNames) -> Acquired {
        Acquired::First(Guard)
    }

    pub fn signal_running(_names: &InstanceNames, _startup_ms: u32, _ack_ms: u32) -> Wait {
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
            for s in [&n.mutex, &n.show, &n.ack, &n.ready] {
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
        assert_eq!(n.ready, "Local\\x-ready");
        assert_ne!(n.ready, n.ack);
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
    fn startup_budget_is_ten_seconds() {
        assert_eq!(STARTUP_TIMEOUT_MS, 10_000);
    }

    #[test]
    fn requests_are_re_asserted_every_100ms() {
        assert_eq!(RETRY_INTERVAL_MS, 100);
        assert_eq!(ACK_TIMEOUT_MS / RETRY_INTERVAL_MS, 20);
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
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::{Arc, Barrier, mpsc};
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

    fn first(n: &InstanceNames) -> Guard {
        let Acquired::First(g) = acquire_named(n) else {
            panic!("expected First");
        };
        g
    }

    /// A polling first instance on its own thread: it waits `startup` (the window, GPU
    /// context and filter load), marks itself ready, then answers every show request at
    /// a hidden stripe's ~50 ms frame rate until `stop` is set. Returns the number of
    /// requests it answered.
    fn spawn_responder(
        n: &InstanceNames,
        startup: Duration,
        stop: Arc<AtomicBool>,
    ) -> std::thread::JoinHandle<u32> {
        let n = n.clone();
        let (acquired_tx, acquired_rx) = mpsc::channel();
        let t = std::thread::spawn(move || {
            let mut g = first(&n);
            acquired_tx.send(()).unwrap();
            std::thread::sleep(startup);
            g.mark_ready();
            let start = Instant::now();
            let mut answered = 0;
            while !stop.load(Ordering::SeqCst) && start.elapsed() < Duration::from_secs(15) {
                if g.take_show_request() {
                    answered += 1;
                    g.ack();
                }
                std::thread::sleep(Duration::from_millis(50));
            }
            answered
        });
        acquired_rx.recv().unwrap();
        t
    }

    #[test]
    fn a_responding_first_instance_acks_the_show_signal() {
        let n = test_names("responder");
        let stop = Arc::new(AtomicBool::new(false));
        let t = spawn_responder(&n, Duration::ZERO, Arc::clone(&stop));
        assert_eq!(signal_running(&n, 1000, ACK_TIMEOUT_MS), Wait::Signaled);
        stop.store(true, Ordering::SeqCst);
        assert_eq!(t.join().unwrap(), 1);
    }

    #[test]
    fn an_unresponsive_first_instance_times_out() {
        let n = test_names("unresponsive");
        let mut g = first(&n);
        g.mark_ready();
        assert_eq!(signal_running(&n, 100, 100), Wait::TimedOut);
    }

    #[test]
    fn a_stale_ack_does_not_count_as_an_answer() {
        let n = test_names("stale");
        let mut g = first(&n);
        g.mark_ready();
        g.ack();
        assert_eq!(signal_running(&n, 100, 100), Wait::TimedOut);
    }

    #[test]
    fn no_show_request_without_a_signal() {
        let n = test_names("noreq");
        let g = first(&n);
        assert!(!g.take_show_request());
    }

    #[test]
    fn a_timed_out_request_is_withdrawn() {
        // IN-02: after the launcher gives up, a late-recovering instance must not
        // reveal the stripe the user was just told is not responding.
        let n = test_names("withdrawn");
        let mut g = first(&n);
        g.mark_ready();
        assert_eq!(signal_running(&n, 100, 250), Wait::TimedOut);
        assert!(!g.take_show_request());
    }

    #[test]
    fn concurrent_launchers_are_each_answered() {
        // WR-01: two launchers signalling at once coalesce into one auto-reset
        // request; the retry loop must still get an ack to each of them.
        let n = test_names("concurrent");
        let stop = Arc::new(AtomicBool::new(false));
        let responder = spawn_responder(&n, Duration::ZERO, Arc::clone(&stop));

        let barrier = Arc::new(Barrier::new(2));
        let launchers: Vec<_> = (0..2)
            .map(|_| {
                let n = n.clone();
                let barrier = Arc::clone(&barrier);
                std::thread::spawn(move || {
                    barrier.wait();
                    signal_running(&n, 1000, ACK_TIMEOUT_MS)
                })
            })
            .collect();
        let results: Vec<Wait> = launchers.into_iter().map(|t| t.join().unwrap()).collect();
        stop.store(true, Ordering::SeqCst);
        assert!(responder.join().unwrap() >= 1);
        assert_eq!(results, vec![Wait::Signaled, Wait::Signaled]);
    }

    #[test]
    fn a_launch_during_startup_waits_for_ready() {
        // WR-02: the first instance takes longer to reach its first frame than the
        // whole ack window; the launcher must wait for ready rather than time out.
        let n = test_names("startup");
        let stop = Arc::new(AtomicBool::new(false));
        let responder = spawn_responder(&n, Duration::from_millis(600), Arc::clone(&stop));
        let started = Instant::now();
        assert_eq!(signal_running(&n, 5000, 300), Wait::Signaled);
        assert!(started.elapsed() >= Duration::from_millis(300));
        stop.store(true, Ordering::SeqCst);
        assert_eq!(responder.join().unwrap(), 1);
    }

    #[test]
    fn a_never_ready_instance_times_out_without_leaving_a_request() {
        // WR-02: no request is sent before ready, so an instance that finishes
        // starting after the launcher gave up does not reveal the stripe late.
        let n = test_names("neverready");
        let mut g = first(&n);
        assert_eq!(signal_running(&n, 150, 100), Wait::TimedOut);
        g.mark_ready();
        assert!(!g.take_show_request());
    }

    #[test]
    fn a_launch_after_the_instance_quit_does_not_see_its_readiness() {
        // A launcher still holding the events keeps them alive past the instance's
        // exit; the next first instance must start not-ready.
        let n = test_names("restart");
        let mut old = first(&n);
        old.mark_ready();
        let lingering = ReadyHold::open(&n);
        drop(old);
        let _new = first(&n);
        assert_eq!(signal_running(&n, 100, 100), Wait::TimedOut);
        drop(lingering);
    }

    /// Holds the `-ready` event open the way a launcher mid-wait does.
    struct ReadyHold(windows_sys::Win32::Foundation::HANDLE);

    impl ReadyHold {
        fn open(n: &InstanceNames) -> Self {
            let name: Vec<u16> = n.ready.encode_utf16().chain(std::iter::once(0)).collect();
            // SAFETY: `name` is NUL-terminated and alive across the call; same
            // parameters as the module's own manual-reset ready event.
            let h = unsafe {
                windows_sys::Win32::System::Threading::CreateEventW(
                    std::ptr::null(),
                    1,
                    0,
                    name.as_ptr(),
                )
            };
            assert!(!h.is_null());
            ReadyHold(h)
        }
    }

    impl Drop for ReadyHold {
        fn drop(&mut self) {
            // SAFETY: created in `open`, closed once.
            unsafe { windows_sys::Win32::Foundation::CloseHandle(self.0) };
        }
    }
}
