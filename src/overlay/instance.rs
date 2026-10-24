//! Single-instance gate for the `ff` overlay (INST-01, Phase 22).
//!
//! A second `ff` launch reveals the running window and exits; it never opens a
//! second window.
//!
//! Mechanism: a named mutex in the `Local\` namespace is held for the process
//! lifetime and released by the OS on exit or crash, so there is no stale-lock
//! state. A manual-reset `-ready` event is set on the first instance's first frame;
//! a second instance waits up to [`STARTUP_TIMEOUT_MS`] for it before asking. An
//! auto-reset `-show` event is polled by the first instance each frame; an
//! auto-reset `-ack` event is waited on by the second for [`ACK_TIMEOUT_MS`],
//! which re-asserts its request every [`RETRY_INTERVAL_MS`] so several launchers
//! waiting at once are each answered. If the first does not answer, the request is
//! withdrawn and the name is checked once more: if the first instance has meanwhile
//! quit (tray Quit releases the name at once) or crashed, this launch runs as the new
//! first instance; otherwise a MessageBox is shown (the release `ff` has no console).
//!
//! `ff --quit` sets an auto-reset `-quit` event, then waits for the instance's own process to
//! exit: the instance writes its process id into a small named `-pid` section once it holds
//! the mutex.
//!
//! Failure policy (WR-03): if the mutex itself cannot be created, the launch fails
//! closed with a MessageBox naming the Win32 error, because the same failure would
//! repeat on every launch and each would open another window. If the first instance
//! cannot create its events it keeps running (it holds the mutex) with no events at
//! all, and a later launch sees the missing `-ready` event and says it cannot reach
//! the running instance instead of reporting it "not responding".
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

/// How long `ff --quit` waits for the instance to exit once it is ready, in ms. A user who
/// takes longer with the Unapplied changes popup is covered by the installer's Retry box.
pub const QUIT_TIMEOUT_MS: u32 = 10_000;

/// Outcome of `ff --quit`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QuitOutcome {
    /// No instance holds the name.
    NotRunning,
    /// The instance went away after the request.
    Exited,
    /// Still running when the wait ran out (for example the user has not answered the
    /// Unapplied changes popup).
    StillRunning,
    /// The running instance cannot be reached (events missing, access denied).
    Unreachable,
}

/// Process exit code for a [`QuitOutcome`]: 0 = not running / exited, 1 = still running,
/// 2 = unreachable.
pub fn quit_exit_code(outcome: QuitOutcome) -> i32 {
    match outcome {
        QuitOutcome::NotRunning | QuitOutcome::Exited => 0,
        QuitOutcome::StillRunning => 1,
        QuitOutcome::Unreachable => 2,
    }
}

/// True only for exactly one argument equal to `--quit` (the installer's hidden flag).
pub fn is_quit_args(args: impl IntoIterator<Item = String>) -> bool {
    let mut args = args.into_iter();
    args.next().as_deref() == Some("--quit") && args.next().is_none()
}

/// Names of the six kernel objects making up one instance gate.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstanceNames {
    pub mutex: String,
    pub show: String,
    pub ack: String,
    pub ready: String,
    pub quit: String,
    /// A small shared-memory section holding the running instance's process id, so
    /// `ff --quit` can wait for that exact process to exit.
    pub pid: String,
}

/// Build the six object names from a base.
pub fn names_for(base: &str) -> InstanceNames {
    InstanceNames {
        mutex: base.to_string(),
        show: format!("{base}-show"),
        ack: format!("{base}-ack"),
        ready: format!("{base}-ready"),
        quit: format!("{base}-quit"),
        pid: format!("{base}-pid"),
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

/// Outcome of waiting on an event, and of asking the first instance to show.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Wait {
    Signaled,
    TimedOut,
    /// The running instance has no show channel this launch can reach (WR-03): opening
    /// an event failed with the given Win32 error (for example access denied by an
    /// elevated instance), or `None` when the running instance never created its events.
    /// Only `signal_running` produces this; `classify_wait` never does.
    Unreachable(Option<u32>),
}

/// The message box a second launch shows for a `signal_running` outcome, if any.
pub fn signal_message(outcome: Wait) -> Option<String> {
    match outcome {
        Wait::Signaled => None,
        Wait::TimedOut => Some(NOT_RESPONDING_MESSAGE.to_string()),
        Wait::Unreachable(code) => {
            let reason = match code {
                Some(code) => format!("Windows error {code}"),
                None => "its show signal is unavailable".to_string(),
            };
            Some(format!(
                "FerrisFilter is already running, but this launch could not ask it to show its \
                 window ({reason}). Use its tray icon or its show / hide hotkey instead."
            ))
        }
    }
}

/// The message box shown when the single-instance check itself fails (WR-03): the launch
/// fails closed rather than risk a second window.
pub fn check_failed_message(code: u32) -> String {
    format!(
        "FerrisFilter could not check whether it is already running (Windows error {code}), so \
         it did not start, to avoid opening a second window. Try again; if this keeps \
         happening, sign out and back in."
    )
}

/// Map a `WaitForSingleObject` code. Anything but WAIT_OBJECT_0 (timeout,
/// failure, abandoned) is "not responding", which only ever leads to the message
/// box, never to a second window.
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
    /// `CreateMutexW` failed with this Win32 error (other than access denied, which
    /// means an elevated instance is running). The launch fails closed.
    Failed(u32),
}

#[cfg(windows)]
mod imp {
    use super::{
        ACK_TIMEOUT_MS, Acquired, InstanceNames, QUIT_TIMEOUT_MS, QuitOutcome, RETRY_INTERVAL_MS,
        STARTUP_TIMEOUT_MS, WAIT_OBJECT_0_CODE, Wait, check_failed_message, classify_wait,
        instance_names, signal_message,
    };
    use std::time::{Duration, Instant};
    use windows_sys::Win32::Foundation::{
        CloseHandle, ERROR_ACCESS_DENIED, ERROR_ALREADY_EXISTS, ERROR_FILE_NOT_FOUND, GetLastError,
        HANDLE, INVALID_HANDLE_VALUE,
    };
    use windows_sys::Win32::System::Memory::{
        CreateFileMappingW, FILE_MAP_READ, FILE_MAP_WRITE, MapViewOfFile, OpenFileMappingW,
        PAGE_READWRITE, UnmapViewOfFile,
    };
    use windows_sys::Win32::System::Threading::{
        CreateEventW, CreateMutexW, GetCurrentProcessId, OpenMutexW, OpenProcess,
        PROCESS_SYNCHRONIZE, ResetEvent, SYNCHRONIZATION_SYNCHRONIZE, SetEvent,
        WaitForSingleObject,
    };
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        MB_ICONERROR, MB_ICONINFORMATION, MB_OK, MB_SETFOREGROUND, MB_TOPMOST, MessageBoxW,
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
        /// Auto-reset; set by `ff --quit`, consumed by the instance's frame poll.
        quit: HANDLE,
        /// The `-pid` section holding this process id (null if it could not be created).
        pid: HANDLE,
        ready_marked: bool,
    }

    impl Drop for Guard {
        fn drop(&mut self) {
            if !self.ready.is_null() {
                // SAFETY: `ready` is a live event handle owned by this Guard. An exiting
                // instance stops polling, so it withdraws its readiness first.
                unsafe { ResetEvent(self.ready) };
            }
            // The mutex goes first: while it is held, this instance's `-ready` event
            // exists, which is how a launcher tells a starting instance from one that has
            // no show channel (WR-03).
            for h in [
                self.mutex, self.show, self.ack, self.ready, self.quit, self.pid,
            ] {
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

        /// Poll (zero timeout) for a quit request from `ff --quit`; auto-reset consumes it.
        pub fn take_quit_request(&self) -> bool {
            if self.quit.is_null() {
                return false;
            }
            // SAFETY: `quit` is a live event handle owned by this Guard.
            let code = unsafe { WaitForSingleObject(self.quit, 0) };
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

    /// Create or open an event; returns the handle (null on failure) and the
    /// `GetLastError` read immediately after the call (`ERROR_ALREADY_EXISTS` when it
    /// opened an existing one).
    fn open_event(name: &[u16], manual_reset: bool) -> (HANDLE, u32) {
        // SAFETY: `name` is a NUL-terminated UTF-16 buffer alive across the call;
        // null security attributes give the default same-user DACL.
        let h =
            unsafe { CreateEventW(std::ptr::null(), i32::from(manual_reset), 0, name.as_ptr()) };
        // SAFETY: plain thread-local error read, immediately after the call.
        let err = unsafe { GetLastError() };
        (h, err)
    }

    /// `-show` and `-ack` are auto-reset.
    fn create_event(name: &[u16]) -> HANDLE {
        open_event(name, false).0
    }

    /// The `-ready` event is manual-reset (it stays set while the instance runs), and
    /// every party creates it with the same parameters, whoever comes first.
    fn create_ready_event(name: &[u16]) -> HANDLE {
        open_event(name, true).0
    }

    fn close_all(handles: &[HANDLE]) {
        for &h in handles {
            if !h.is_null() {
                // SAFETY: each handle came from Create* in this module and its owner
                // closes it exactly once, here.
                unsafe { CloseHandle(h) };
            }
        }
    }

    /// Size of the `-pid` section: one `u32` process id.
    const PID_BYTES: usize = std::mem::size_of::<u32>();

    /// Create the `-pid` section and write this process id into it. Returns the section
    /// handle (kept open by the Guard), or null when it cannot be created; `ff --quit` then
    /// falls back to waiting for the mutex to be released.
    fn publish_pid(name: &[u16]) -> HANDLE {
        // SAFETY: `name` is a NUL-terminated UTF-16 buffer alive across the call; a
        // pagefile-backed section (INVALID_HANDLE_VALUE) with the default same-user DACL.
        let h = unsafe {
            CreateFileMappingW(
                INVALID_HANDLE_VALUE,
                std::ptr::null(),
                PAGE_READWRITE,
                0,
                PID_BYTES as u32,
                name.as_ptr(),
            )
        };
        if h.is_null() {
            return h;
        }
        // SAFETY: `h` is a live section handle of at least PID_BYTES bytes.
        let view = unsafe { MapViewOfFile(h, FILE_MAP_WRITE, 0, 0, PID_BYTES) };
        if view.Value.is_null() {
            close_all(&[h]);
            return std::ptr::null_mut();
        }
        // SAFETY: `view` maps at least PID_BYTES writable bytes and is unmapped once, here.
        unsafe {
            view.Value
                .cast::<u32>()
                .write_unaligned(GetCurrentProcessId());
            UnmapViewOfFile(view);
        }
        h
    }

    /// The process id the running instance published, if its `-pid` section exists.
    pub(super) fn published_pid(name: &[u16]) -> Option<u32> {
        // SAFETY: `name` is a NUL-terminated UTF-16 buffer alive across the call.
        let h = unsafe { OpenFileMappingW(FILE_MAP_READ, 0, name.as_ptr()) };
        if h.is_null() {
            return None;
        }
        // SAFETY: `h` is a live section handle; the instance created it PID_BYTES long.
        let view = unsafe { MapViewOfFile(h, FILE_MAP_READ, 0, 0, PID_BYTES) };
        let pid = if view.Value.is_null() {
            None
        } else {
            // SAFETY: `view` maps at least PID_BYTES readable bytes and is unmapped once, here.
            let pid = unsafe { view.Value.cast::<u32>().read_unaligned() };
            // SAFETY: as above.
            unsafe { UnmapViewOfFile(view) };
            Some(pid)
        };
        close_all(&[h]);
        pid.filter(|&p| p != 0)
    }

    /// Tests only: make an existing `-pid` section name another process, so `ff --quit` can be
    /// pointed at a real process other than the test binary. False when there is no section.
    #[cfg(test)]
    pub(super) fn overwrite_published_pid(name: &[u16], pid: u32) -> bool {
        // SAFETY: `name` is a NUL-terminated UTF-16 buffer alive across the call.
        let h = unsafe { OpenFileMappingW(FILE_MAP_WRITE, 0, name.as_ptr()) };
        if h.is_null() {
            return false;
        }
        // SAFETY: `h` is a live section handle; the instance created it PID_BYTES long.
        let view = unsafe { MapViewOfFile(h, FILE_MAP_WRITE, 0, 0, PID_BYTES) };
        let ok = !view.Value.is_null();
        if ok {
            // SAFETY: `view` maps at least PID_BYTES writable bytes and is unmapped once, here.
            unsafe {
                view.Value.cast::<u32>().write_unaligned(pid);
                UnmapViewOfFile(view);
            }
        }
        close_all(&[h]);
        ok
    }

    /// Open (SYNCHRONIZE) the process that published itself as the running instance. `None`
    /// when there is no `-pid` section, it names this very process (the in-process tests),
    /// or the process cannot be opened. The caller closes the handle.
    fn owner_process(pid_name: &[u16]) -> Option<HANDLE> {
        let pid = published_pid(pid_name)?;
        // SAFETY: plain call.
        if pid == unsafe { GetCurrentProcessId() } {
            return None;
        }
        // SAFETY: plain call; a null result (access denied, gone) means no owner handle.
        let h = unsafe { OpenProcess(PROCESS_SYNCHRONIZE, 0, pid) };
        (!h.is_null()).then_some(h)
    }

    pub fn acquire_named(names: &InstanceNames) -> Acquired {
        // Build every wide name before CreateMutexW so nothing runs between it
        // and GetLastError.
        let mutex_name = wide(&names.mutex);
        let show_name = wide(&names.show);
        let ack_name = wide(&names.ack);
        let ready_name = wide(&names.ready);
        let quit_name = wide(&names.quit);
        let pid_name = wide(&names.pid);

        // Events BEFORE the mutex (WR-03): a running instance's `-ready` event then
        // exists for as long as it holds the mutex, so a launcher that finds the mutex
        // held but has to create `-ready` itself knows the instance has no show channel,
        // instead of waiting out the startup budget for a signal that can never come.
        let show = create_event(&show_name);
        let ack = create_event(&ack_name);
        let ready = create_ready_event(&ready_name);
        let quit = create_event(&quit_name);

        // SAFETY: `mutex_name` is NUL-terminated and alive across the call.
        let mutex = unsafe { CreateMutexW(std::ptr::null(), 0, mutex_name.as_ptr()) };
        // SAFETY: plain thread-local error read, immediately after the call.
        let err = unsafe { GetLastError() };

        if mutex.is_null() {
            close_all(&[show, ack, ready, quit]);
            // An elevated first instance denies access to the object: treat as
            // running (the signal path then reports it as unreachable, never a
            // second window).
            if err == ERROR_ACCESS_DENIED {
                return Acquired::AlreadyRunning;
            }
            // Fail closed (WR-03): any other failure (e.g. ERROR_INVALID_HANDLE when
            // another object type holds the name) would repeat on every launch, so
            // starting anyway would open a new window each time.
            return Acquired::Failed(err);
        }
        if err == ERROR_ALREADY_EXISTS {
            // A launcher: the events belong to the running instance; leave their
            // state alone and let `signal_running` reopen them.
            close_all(&[mutex, show, ack, ready, quit]);
            return Acquired::AlreadyRunning;
        }

        let (show, ack, ready, quit) =
            if show.is_null() || ack.is_null() || ready.is_null() || quit.is_null() {
                // Keep running (this process holds the mutex, so it is the one window),
                // but with no events at all, so the next launch reports that it cannot
                // reach this instance instead of a false "not responding".
                eprintln!("single-instance: CreateEventW failed; show requests disabled");
                close_all(&[show, ack, ready, quit]);
                (
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                )
            } else {
                (show, ack, ready, quit)
            };
        // The events may outlive a previous instance while a launcher still holds them
        // (e.g. mid-retry when it quit): start from a clean, not-ready state.
        for h in [show, ack, ready, quit] {
            if !h.is_null() {
                // SAFETY: live event handles created just above.
                unsafe { ResetEvent(h) };
            }
        }
        // Only the process that won the mutex publishes its id, and it does so before it can
        // be marked ready, which is when `ff --quit` reads it.
        let pid = publish_pid(&pid_name);
        if pid.is_null() {
            eprintln!("single-instance: CreateFileMappingW failed; ff --quit waits on the name");
        }
        Acquired::First(Guard {
            mutex,
            show,
            ack,
            ready,
            quit,
            pid,
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
            let remaining = deadline.saturating_duration_since(Instant::now());
            // Never send a request there is no time left to wait for (review iteration 2,
            // IN-02): one sent with a zero slice could still be consumed by a recovering
            // instance after this launch has decided to report "not responding".
            // (Under 1 ms counts as none: the wait below takes whole milliseconds.)
            if remaining < Duration::from_millis(1) {
                return Wait::TimedOut;
            }
            // SAFETY: `show` is a live event handle owned by the caller.
            unsafe { SetEvent(show) };
            let slice = remaining.min(interval);
            // `slice` <= RETRY_INTERVAL_MS, so the cast cannot truncate.
            // SAFETY: `ack` is a live event handle owned by the caller.
            let code = unsafe { WaitForSingleObject(ack, slice.as_millis() as u32) };
            if classify_wait(code) == Wait::Signaled {
                return Wait::Signaled;
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
            // request so an instance that recovers later does not reveal the window after
            // the user was told it is not responding (IN-02).
            unsafe { ResetEvent(show) };
        }
        result
    }

    /// Ask the running instance to show its window. First wait up to `startup_ms` for it to
    /// be ready (WR-02: a launch during its startup is not "not responding"); no request is
    /// sent before then, so a never-ready instance is not left holding one. Then request and
    /// wait up to `ack_ms` for the ack.
    ///
    /// Returns [`Wait::Unreachable`] without waiting when an event cannot be opened, or when
    /// `-ready` did not already exist: the running instance creates it before taking the
    /// mutex and keeps it until after releasing it, so its absence means that instance has
    /// no show channel (its own event creation failed, WR-03).
    pub fn signal_running(names: &InstanceNames, startup_ms: u32, ack_ms: u32) -> Wait {
        let ready_name = wide(&names.ready);
        let show_name = wide(&names.show);
        let ack_name = wide(&names.ack);
        let (ready, ready_err) = open_event(&ready_name, true);
        let (show, show_err) = open_event(&show_name, false);
        let (ack, ack_err) = open_event(&ack_name, false);
        let result = if ready.is_null() {
            Wait::Unreachable(Some(ready_err))
        } else if show.is_null() {
            Wait::Unreachable(Some(show_err))
        } else if ack.is_null() {
            Wait::Unreachable(Some(ack_err))
        } else if ready_err != ERROR_ALREADY_EXISTS {
            Wait::Unreachable(None)
        } else {
            request_show(show, ack, ready, startup_ms, ack_ms)
        };
        close_all(&[show, ack, ready]);
        result
    }

    /// Open the instance mutex without creating it. `Ok(Some(handle))` = held by an
    /// instance, `Ok(None)` = no such mutex, `Err(code)` = it exists but cannot be opened.
    fn open_mutex(name: &[u16]) -> Result<Option<HANDLE>, u32> {
        // SAFETY: `name` is a NUL-terminated UTF-16 buffer alive across the call.
        let h = unsafe { OpenMutexW(SYNCHRONIZATION_SYNCHRONIZE, 0, name.as_ptr()) };
        // SAFETY: plain thread-local error read, immediately after the call.
        let err = unsafe { GetLastError() };
        if !h.is_null() {
            Ok(Some(h))
        } else if err == ERROR_FILE_NOT_FOUND {
            Ok(None)
        } else {
            Err(err)
        }
    }

    /// True once the instance mutex no longer exists (every holder closed it).
    fn mutex_gone(name: &[u16]) -> bool {
        match open_mutex(name) {
            Ok(None) => true,
            Ok(Some(h)) => {
                close_all(&[h]);
                false
            }
            Err(_) => false,
        }
    }

    /// True when the process handle is signalled (the process has exited).
    fn has_exited(process: HANDLE) -> bool {
        // SAFETY: `process` is a live process handle with SYNCHRONIZE access.
        let code = unsafe { WaitForSingleObject(process, 0) };
        code == WAIT_OBJECT_0_CODE
    }

    /// Ask the running instance to quit (`ff --quit`). It never creates the mutex, so it can
    /// never become the instance itself.
    ///
    /// Success means the process that owns the instance has exited: the instance publishes
    /// its process id in the `-pid` section, and that one process is waited for. Other `ff.exe`
    /// processes (a launch showing a message box, another `--quit`, a launch that becomes the
    /// next instance) do not count, and the instance releasing its name early in its shutdown
    /// is not mistaken for its exit. Without a readable `-pid` section the wait falls back to
    /// the mutex being released.
    pub fn quit_running_named(names: &InstanceNames, startup_ms: u32, wait_ms: u32) -> QuitOutcome {
        let mutex_name = wide(&names.mutex);
        match open_mutex(&mutex_name) {
            Ok(None) => return QuitOutcome::NotRunning,
            Err(_) => return QuitOutcome::Unreachable,
            // Holding it would keep the name alive past the instance's exit.
            Ok(Some(h)) => close_all(&[h]),
        }

        let (ready, ready_err) = open_event(&wide(&names.ready), true);
        let (quit, quit_err) = open_event(&wide(&names.quit), false);
        let mut owner: Option<HANDLE> = None;
        let outcome = if ready.is_null() || quit.is_null() {
            QuitOutcome::Unreachable
        } else if ready_err != ERROR_ALREADY_EXISTS || quit_err != ERROR_ALREADY_EXISTS {
            QuitOutcome::Unreachable
        } else {
            // SAFETY: `ready` is a live event handle owned by this function.
            let code = unsafe { WaitForSingleObject(ready, startup_ms) };
            if classify_wait(code) != Wait::Signaled {
                QuitOutcome::Unreachable
            } else {
                // The instance publishes its id before it is marked ready. Read it BEFORE
                // signalling so a fast exit cannot be missed.
                owner = owner_process(&wide(&names.pid));
                // One request only: a re-asserted one would reopen the Unapplied changes
                // popup right after the user picked Cancel in it.
                // SAFETY: `quit` is a live event handle owned by this function.
                unsafe { SetEvent(quit) };
                let deadline = Instant::now() + Duration::from_millis(u64::from(wait_ms));
                let interval = Duration::from_millis(u64::from(RETRY_INTERVAL_MS));
                loop {
                    let done = match owner {
                        Some(process) => has_exited(process),
                        None => mutex_gone(&mutex_name),
                    };
                    if done {
                        break QuitOutcome::Exited;
                    }
                    let remaining = deadline.saturating_duration_since(Instant::now());
                    if remaining.is_zero() {
                        // Withdraw the request so a later frame does not quit unexpectedly.
                        // SAFETY: `quit` is a live event handle owned by this function.
                        unsafe { ResetEvent(quit) };
                        break QuitOutcome::StillRunning;
                    }
                    std::thread::sleep(remaining.min(interval));
                }
            }
        };
        close_all(&[ready, quit]);
        if let Some(process) = owner {
            close_all(&[process]);
        }
        outcome
    }

    /// `ff --quit` against the real instance of this build profile.
    pub fn quit_running() -> QuitOutcome {
        quit_running_named(&instance_names(), STARTUP_TIMEOUT_MS, QUIT_TIMEOUT_MS)
    }

    /// An informational message box (the release `ff` has no console).
    pub fn notify(message: &str) {
        let text = wide(message);
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

    /// An error message box (same as `notify`, with the error icon).
    pub fn notify_error(message: &str) {
        let text = wide(message);
        let title = wide("FerrisFilter");
        // SAFETY: both buffers are NUL-terminated and alive across the call; a
        // null owner window is allowed.
        unsafe {
            MessageBoxW(
                std::ptr::null_mut(),
                text.as_ptr(),
                title.as_ptr(),
                MB_OK | MB_ICONERROR | MB_SETFOREGROUND | MB_TOPMOST,
            );
        }
    }

    /// The decision of one launch, before any message box is shown.
    pub enum Launch {
        /// This process is the single instance: run the window.
        Run(Guard),
        /// Exit; show this message first, if any.
        Exit(Option<String>),
    }

    /// The launch decision over explicit names and budgets (testable without a MessageBox).
    ///
    /// When asking the running instance fails, the name is checked once more before any
    /// message (review iteration 2, WR-01): the instance may have quit, or crashed, while this
    /// launch was asking, and then this launch is the one that should run. The mutex is still
    /// the arbiter, so this can never open a second window.
    pub fn launch_named(names: &InstanceNames, startup_ms: u32, ack_ms: u32) -> Launch {
        match acquire_named(names) {
            Acquired::First(g) => Launch::Run(g),
            Acquired::AlreadyRunning => {
                let outcome = signal_running(names, startup_ms, ack_ms);
                if outcome == Wait::Signaled {
                    return Launch::Exit(signal_message(outcome));
                }
                // The signal failed: re-check the name once. A failed re-check reports its own
                // error rather than the stale signal outcome (review iteration 3, IN-02).
                match acquire_named(names) {
                    Acquired::First(g) => Launch::Run(g),
                    Acquired::Failed(code) => Launch::Exit(Some(check_failed_message(code))),
                    Acquired::AlreadyRunning => Launch::Exit(signal_message(outcome)),
                }
            }
            Acquired::Failed(code) => Launch::Exit(Some(check_failed_message(code))),
        }
    }

    /// Composed entry for `launch()`: `Some(guard)` = run the window, `None` =
    /// exit 0. Call before creating the hotkey or the tray, and keep the Guard
    /// alive until exit (or until tray Quit, which drops it early).
    pub fn acquire() -> Option<Guard> {
        match launch_named(&instance_names(), STARTUP_TIMEOUT_MS, ACK_TIMEOUT_MS) {
            Launch::Run(g) => Some(g),
            Launch::Exit(message) => {
                if let Some(message) = message {
                    notify(&message);
                }
                None
            }
        }
    }
}

#[cfg(not(windows))]
mod imp {
    use super::{Acquired, InstanceNames, QuitOutcome, Wait};

    /// Non-Windows stand-in; the overlay is not supported off Windows.
    pub struct Guard;

    impl Guard {
        pub fn mark_ready(&mut self) {}
        pub fn take_show_request(&self) -> bool {
            false
        }
        pub fn take_quit_request(&self) -> bool {
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

    pub fn quit_running_named(
        _names: &InstanceNames,
        _startup_ms: u32,
        _wait_ms: u32,
    ) -> QuitOutcome {
        QuitOutcome::NotRunning
    }

    pub fn quit_running() -> QuitOutcome {
        QuitOutcome::NotRunning
    }

    pub fn notify(_message: &str) {}

    pub fn notify_error(_message: &str) {}

    /// No single-instance enforcement off Windows, where the overlay is not
    /// supported anyway.
    pub fn acquire() -> Option<Guard> {
        Some(Guard)
    }
}

pub use imp::{
    Guard, acquire, acquire_named, notify, notify_error, quit_running, quit_running_named,
    signal_running,
};
#[cfg(windows)]
pub use imp::{Launch, launch_named};

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
            for s in [&n.mutex, &n.show, &n.ack, &n.ready, &n.quit, &n.pid] {
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
        assert_eq!(n.quit, "Local\\x-quit");
        assert_eq!(n.pid, "Local\\x-pid");
        assert_ne!(n.quit, n.show);
        assert_ne!(n.ready, n.ack);
        assert_ne!(n.mutex, n.show);
        assert_ne!(n.show, n.ack);
        assert_ne!(n.mutex, n.ack);
    }

    #[test]
    fn quit_exit_code_maps_each_outcome() {
        assert_eq!(quit_exit_code(QuitOutcome::NotRunning), 0);
        assert_eq!(quit_exit_code(QuitOutcome::Exited), 0);
        assert_eq!(quit_exit_code(QuitOutcome::StillRunning), 1);
        assert_eq!(quit_exit_code(QuitOutcome::Unreachable), 2);
    }

    #[test]
    fn quit_flag_is_exactly_one_argument() {
        let args = |v: &[&str]| v.iter().map(|s| s.to_string()).collect::<Vec<_>>();
        assert!(!is_quit_args(args(&[])));
        assert!(is_quit_args(args(&["--quit"])));
        assert!(!is_quit_args(args(&["--quit", "x"])));
        assert!(!is_quit_args(args(&["-quit"])));
        assert!(!is_quit_args(args(&["--QUIT"])));
        assert!(!is_quit_args(args(&["--screenshot", "a.png"])));
    }

    #[test]
    fn quit_timeout_is_ten_seconds() {
        assert_eq!(QUIT_TIMEOUT_MS, 10_000);
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

    #[test]
    fn each_signal_outcome_maps_to_its_message() {
        assert_eq!(signal_message(Wait::Signaled), None);
        assert_eq!(
            signal_message(Wait::TimedOut).as_deref(),
            Some(NOT_RESPONDING_MESSAGE)
        );
        let denied = signal_message(Wait::Unreachable(Some(5))).unwrap();
        assert!(denied.contains("already running"), "{denied}");
        assert!(denied.contains("Windows error 5"), "{denied}");
        assert!(!denied.contains("not responding"), "{denied}");
        let deaf = signal_message(Wait::Unreachable(None)).unwrap();
        assert!(deaf.contains("show signal is unavailable"), "{deaf}");
        assert!(deaf.contains("tray icon"), "{deaf}");
    }

    #[test]
    fn a_failed_check_names_the_error_and_says_it_did_not_start() {
        let msg = check_failed_message(6);
        assert!(msg.contains("Windows error 6"), "{msg}");
        assert!(msg.contains("did not start"), "{msg}");
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
    /// a hidden overlay's ~50 ms frame rate until `stop` is set. Returns the number of
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
        // `>= 1`, not `== 1` (review iteration 2, IN-01): a re-sent request that lands
        // between the responder's consume and its ack is answered once more, harmlessly.
        assert!(t.join().unwrap() >= 1);
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
    fn quit_on_a_free_name_reports_not_running() {
        let n = test_names("quitfree");
        assert_eq!(quit_running_named(&n, 100, 100), QuitOutcome::NotRunning);
        // Quit never created the name, so the next acquire is still the first instance.
        assert!(matches!(acquire_named(&n), Acquired::First(_)));
    }

    #[test]
    fn no_quit_request_without_a_signal() {
        let n = test_names("noquit");
        let g = first(&n);
        assert!(!g.take_quit_request());
    }

    #[test]
    fn a_polling_instance_quits_on_request() {
        let n = test_names("quitpoll");
        let (acquired_tx, acquired_rx) = mpsc::channel();
        let t = {
            let n = n.clone();
            std::thread::spawn(move || {
                let mut g = first(&n);
                g.mark_ready();
                acquired_tx.send(()).unwrap();
                let deadline = Instant::now() + Duration::from_secs(5);
                while Instant::now() < deadline {
                    if g.take_quit_request() {
                        break;
                    }
                    std::thread::sleep(Duration::from_millis(10));
                }
                drop(g);
            })
        };
        acquired_rx.recv().unwrap();
        assert_eq!(quit_running_named(&n, 2000, 3000), QuitOutcome::Exited);
        t.join().unwrap();
    }

    #[test]
    fn the_instance_publishes_its_process_id_until_it_drops() {
        let n = test_names("pid");
        let pid_name = n
            .pid
            .encode_utf16()
            .chain(std::iter::once(0))
            .collect::<Vec<_>>();
        assert_eq!(imp::published_pid(&pid_name), None);
        let g = first(&n);
        assert_eq!(imp::published_pid(&pid_name), Some(std::process::id()));
        // A launcher never touches it.
        assert!(matches!(acquire_named(&n), Acquired::AlreadyRunning));
        assert_eq!(imp::published_pid(&pid_name), Some(std::process::id()));
        drop(g);
        assert_eq!(imp::published_pid(&pid_name), None);
    }

    fn wide_name(name: &str) -> Vec<u16> {
        name.encode_utf16().chain(std::iter::once(0)).collect()
    }

    /// A harmless long-running child process (a system `PING.EXE` with no output), killed
    /// when dropped so a failing test never leaves it behind.
    struct Sleeper(std::process::Child);

    impl Sleeper {
        fn spawn() -> Self {
            let root = std::env::var("SystemRoot").unwrap_or_else(|_| r"C:\Windows".to_string());
            let child = std::process::Command::new(format!(r"{root}\System32\PING.EXE"))
                .args(["-n", "60", "127.0.0.1"])
                .stdin(std::process::Stdio::null())
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .spawn()
                .expect("spawn PING.EXE");
            Self(child)
        }

        /// Ends the process; the `Child` (and so its handle, which keeps the pid from being
        /// reused) stays alive until the Sleeper drops.
        fn end(&mut self) {
            let _ = self.0.kill();
            self.0.wait().expect("wait for PING.EXE");
        }
    }

    impl Drop for Sleeper {
        fn drop(&mut self) {
            let _ = self.0.kill();
            let _ = self.0.wait();
        }
    }

    #[test]
    fn quit_follows_the_published_owner_process_not_the_name() {
        let n = test_names("owner");
        let mut child = Sleeper::spawn();
        let mut g = first(&n);
        assert!(imp::overwrite_published_pid(
            &wide_name(&n.pid),
            child.0.id()
        ));
        g.mark_ready();
        // The owner is alive: still running.
        assert_eq!(quit_running_named(&n, 2000, 300), QuitOutcome::StillRunning);
        // The owner has exited while the name is still held: that exit is what counts.
        child.end();
        assert_eq!(quit_running_named(&n, 2000, 3000), QuitOutcome::Exited);
        drop(g);
    }

    #[test]
    fn an_instance_releasing_its_name_early_is_not_taken_as_its_exit() {
        let n = test_names("earlyname");
        let mut child = Sleeper::spawn();
        let child_pid = child.0.id();
        let (ready_tx, ready_rx) = mpsc::channel();
        let t = {
            let n = n.clone();
            std::thread::spawn(move || {
                let mut g = first(&n);
                assert!(imp::overwrite_published_pid(&wide_name(&n.pid), child_pid));
                g.mark_ready();
                ready_tx.send(()).unwrap();
                let deadline = Instant::now() + Duration::from_secs(5);
                while Instant::now() < deadline {
                    if g.take_quit_request() {
                        break;
                    }
                    std::thread::sleep(Duration::from_millis(10));
                }
                // Release the name (and every other object) while the owner process runs on,
                // the way a shutting-down instance gives up its name before it exits.
                drop(g);
            })
        };
        ready_rx.recv().unwrap();
        assert_eq!(quit_running_named(&n, 2000, 600), QuitOutcome::StillRunning);
        t.join().unwrap();
        child.end();
    }

    #[test]
    fn an_instance_that_never_polls_times_out_and_the_request_is_withdrawn() {
        let n = test_names("quitstuck");
        let mut g = first(&n);
        g.mark_ready();
        assert_eq!(quit_running_named(&n, 2000, 300), QuitOutcome::StillRunning);
        assert!(!g.take_quit_request());
    }

    #[test]
    fn a_timed_out_request_is_withdrawn() {
        // IN-02: after the launcher gives up, a late-recovering instance must not
        // reveal the window the user was just told is not responding.
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
        // `>= 1` for the same reason as in the responder test above (IN-01).
        assert!(responder.join().unwrap() >= 1);
    }

    #[test]
    fn a_launch_whose_instance_quits_mid_request_runs_instead() {
        // Review iteration 2, WR-01: the running instance is ready but stops answering (it
        // was told to Quit) and releases the name while the launcher is asking. The launcher
        // must become the new first instance, not report "not responding" and leave nothing.
        //
        // The quitting side drops its guard only after it has seen a show request (review
        // iteration 3, IN-03). That proves the launcher was already inside `request_until_acked`
        // when the name was released, so this passes only through the re-check, never through a
        // first `acquire_named` that happened to land after the drop.
        let n = test_names("quitmid");
        let (acquired_tx, acquired_rx) = mpsc::channel();
        let quitting = {
            let n = n.clone();
            std::thread::spawn(move || {
                let mut g = first(&n);
                g.mark_ready();
                acquired_tx.send(()).unwrap();
                let deadline = Instant::now() + Duration::from_secs(5);
                let mut saw_request = false;
                while Instant::now() < deadline {
                    if g.take_show_request() {
                        saw_request = true;
                        break;
                    }
                    std::thread::sleep(Duration::from_millis(5));
                }
                // Never acked: this instance is quitting.
                drop(g);
                saw_request
            })
        };
        acquired_rx.recv().unwrap();
        let launch = launch_named(&n, 1000, 500);
        let saw_request = quitting.join().unwrap();
        assert!(
            saw_request,
            "the launcher never asked the quitting instance"
        );
        assert!(matches!(launch, Launch::Run(_)));
        // And it really holds the name now.
        assert!(matches!(acquire_named(&n), Acquired::AlreadyRunning));
        drop(launch);
    }

    #[test]
    fn a_launch_to_a_responding_instance_exits_without_a_message() {
        let n = test_names("launchok");
        let stop = Arc::new(AtomicBool::new(false));
        let responder = spawn_responder(&n, Duration::ZERO, Arc::clone(&stop));
        let launch = launch_named(&n, 1000, ACK_TIMEOUT_MS);
        stop.store(true, Ordering::SeqCst);
        responder.join().unwrap();
        assert!(matches!(launch, Launch::Exit(None)));
    }

    #[test]
    fn a_launch_to_a_still_running_unresponsive_instance_reports_it() {
        // The re-check only runs a launch whose instance is really gone; one that still
        // holds the name keeps the "not responding" message and never a second window.
        let n = test_names("launchstuck");
        let mut g = first(&n);
        g.mark_ready();
        let launch = launch_named(&n, 100, 100);
        assert!(
            matches!(&launch, Launch::Exit(Some(m)) if m == NOT_RESPONDING_MESSAGE),
            "expected the not-responding message"
        );
        drop(g);
    }

    #[test]
    fn a_never_ready_instance_times_out_without_leaving_a_request() {
        // WR-02: no request is sent before ready, so an instance that finishes
        // starting after the launcher gave up does not reveal the window late.
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
        let lingering = Hold::ready_event(&n.ready);
        drop(old);
        let _new = first(&n);
        assert_eq!(signal_running(&n, 100, 100), Wait::TimedOut);
        drop(lingering);
    }

    #[test]
    fn a_name_held_by_another_object_type_fails_closed() {
        // WR-03: CreateMutexW fails with ERROR_INVALID_HANDLE (6) when an event
        // already owns the name. That must not become a second "first" instance.
        let n = test_names("failclosed");
        let squatter = Hold::ready_event(&n.mutex);
        assert!(matches!(acquire_named(&n), Acquired::Failed(6)));
        assert!(matches!(acquire_named(&n), Acquired::Failed(6)));
        drop(squatter);
        assert!(matches!(acquire_named(&n), Acquired::First(_)));
    }

    #[test]
    fn an_instance_without_events_is_reported_unreachable_at_once() {
        // WR-03: a running instance (it holds the mutex) whose events were never
        // created must not cost the launcher the startup budget plus a false
        // "not responding".
        let n = test_names("deaf");
        let _mutex = Hold::mutex(&n.mutex);
        let started = Instant::now();
        assert_eq!(signal_running(&n, 5000, 5000), Wait::Unreachable(None));
        assert!(started.elapsed() < Duration::from_secs(2));
    }

    #[test]
    fn an_instance_whose_event_creation_failed_keeps_running_and_is_unreachable() {
        // WR-03: the first instance keeps the mutex (it is still the one window),
        // and a later launch reports the failure instead of opening a second one.
        let n = test_names("eventfail");
        let squatter = Hold::mutex(&n.ready);
        let mut g = first(&n);
        g.mark_ready();
        assert!(!g.take_show_request());
        assert!(matches!(acquire_named(&n), Acquired::AlreadyRunning));
        assert_eq!(signal_running(&n, 100, 100), Wait::Unreachable(Some(6)));
        drop(squatter);
        // The squatter is gone, but the instance still has no events, so a launch
        // must still see it as unreachable rather than wait for a ready that never comes.
        assert_eq!(signal_running(&n, 5000, 5000), Wait::Unreachable(None));
    }

    /// A raw named kernel object held open by a test (a launcher mid-wait, a deaf
    /// instance's mutex, or a squatter of another type on one of the names).
    struct Hold(windows_sys::Win32::Foundation::HANDLE);

    impl Hold {
        fn wide(name: &str) -> Vec<u16> {
            name.encode_utf16().chain(std::iter::once(0)).collect()
        }

        /// A manual-reset, unset event, as the module creates `-ready`.
        fn ready_event(name: &str) -> Self {
            let name = Self::wide(name);
            // SAFETY: `name` is NUL-terminated and alive across the call.
            let h = unsafe {
                windows_sys::Win32::System::Threading::CreateEventW(
                    std::ptr::null(),
                    1,
                    0,
                    name.as_ptr(),
                )
            };
            assert!(!h.is_null());
            Hold(h)
        }

        fn mutex(name: &str) -> Self {
            let name = Self::wide(name);
            // SAFETY: `name` is NUL-terminated and alive across the call.
            let h = unsafe {
                windows_sys::Win32::System::Threading::CreateMutexW(
                    std::ptr::null(),
                    0,
                    name.as_ptr(),
                )
            };
            assert!(!h.is_null());
            Hold(h)
        }
    }

    impl Drop for Hold {
        fn drop(&mut self) {
            // SAFETY: created by a `Hold` constructor, closed once.
            unsafe { windows_sys::Win32::Foundation::CloseHandle(self.0) };
        }
    }
}
