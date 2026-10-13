# Phase 22: Single-Instance Overlay - Research

**Researched:** 2026-10-13
**Domain:** Win32 named mutex/event single-instance gate in a Rust eframe overlay
**Confidence:** HIGH (codebase and windows-sys source verified locally; runtime behaviour of hidden-window repaint is MEDIUM until manually checked)

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions
- The second launch **shows** the running stripe (a reveal, never a toggle), including when it is hidden to the tray, and then exits.
- The reveal must bypass the hotkey's foreground-window gate. A hidden egui window needs a repaint wake-up (`request_repaint`), or the signal isn't processed until the next input.
- If the first instance doesn't respond within about 2 seconds, the second shows a short message ("FerrisFilter is already running but not responding") and exits. It never opens a second stripe.
- After repeated launches there is exactly one stripe and one working hotkey.
- Mechanism: Windows named mutex in `Local\`, held for all of `main`, checked with `GetLastError` immediately. Plus a named event the first instance waits on. Uses existing `windows-sys` with extra features `Win32_System_Threading`/`Win32_Security`. No new crate, no lock files. Check runs before hotkey and tray are created.
- Debug builds use a **separate instance name**, so `cargo run` can run beside a release `ff`.

### Claude's Discretion
- Watcher thread vs per-frame event polling, the GUID in the name, and the message-box API.

### Deferred Ideas (OUT OF SCOPE)
None.
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| INST-01 | Launching `ff` twice never yields two stripes; second launch reveals the running one and exits | Mutex gate + show/ack events; per-frame poll in `OverlayApp::update`; MessageBoxW on timeout |
</phase_requirements>

## Summary

All needed Win32 APIs exist in the already-pinned `windows-sys 0.61.2` [VERIFIED: ~/.cargo/registry windows-sys-0.61.2 source, Cargo.lock]. The only Cargo change is adding two features to the existing `[target.'cfg(windows)'.dependencies]` line: `Win32_System_Threading` (CreateMutexW, CreateEventW, SetEvent, ResetEvent, WaitForSingleObject) and `Win32_Security` (the `SECURITY_ATTRIBUTES` type in the signatures; pass null). `CloseHandle`, `GetLastError`, `ERROR_ALREADY_EXISTS`, `WAIT_OBJECT_0`, `WAIT_TIMEOUT` are in `Win32_Foundation` (already enabled). `MessageBoxW`, `MB_OK`, `MB_ICONINFORMATION`/`MB_ICONWARNING`, `MB_SETFOREGROUND`, `MB_TOPMOST` are in `Win32_UI_WindowsAndMessaging` (already enabled). No new crate, so no slopcheck/package audit is needed.

The overlay already solves "wake a hidden window without input": `OverlayApp::update` (src/overlay/app.rs ~1337) calls `request_repaint_after(50ms)` every frame, and the tray Show / hotkey / tray click events are all *polled* with `try_recv()` inside `update` (no waker thread, no event-handler callback). A comment there states eframe >= 0.34 gives invisible windows a ~10 Hz heartbeat. The tray "Show" while Hidden already works on this basis, so a per-frame `WaitForSingleObject(show_event, 0)` poll sits on identical machinery with identical latency (<= ~100 ms, far under the 2 s budget). The reveal path is `toggle_requested = true` when `visibility == Hidden`, which calls `win::reveal` (`SW_SHOWNOACTIVATE`, no focus steal). Because the poll is its own branch, not the hotkey branch, it naturally bypasses `win::foreground_is_game_or_overlay`.

**Primary recommendation:** New module `src/overlay/instance.rs` (cfg(windows) impl + cfg(not(windows)) stub, same `mod imp` convention as `win.rs`), `acquire()` called as the very first line of `launch()`; per-frame poll in `update`, no watcher thread; names `Local\FerrisFilter-<GUID>-overlay` (release) / `...-overlay-debug` (debug) plus `-show` and `-ack` event suffixes.

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| Detect existing instance | OS kernel object (named mutex) | `overlay::instance` | Mutex is process-lifetime, auto-released on crash |
| Signal "show" / ack | OS named events | `overlay::instance` | Cross-process, no sockets/files |
| Apply reveal | `OverlayApp::update` (egui thread) | `win::reveal` | Only the window-owning thread may touch visibility state (visibility.rs state machine) |
| Not-responding message | `overlay::instance` (MessageBoxW) | — | No egui exists in the second process |
| Name selection (debug/release) | pure fn in `instance.rs` | — | Unit-testable |

## Standard Stack

### Core
| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| windows-sys | 0.61.2 (locked) | CreateMutexW/CreateEventW/SetEvent/WaitForSingleObject/MessageBoxW | Already a dependency; `win.rs` is the existing home of raw Win32 [VERIFIED: Cargo.toml, Cargo.lock] |

**Cargo.toml change (only):**
```toml
[target.'cfg(windows)'.dependencies]
windows-sys = { version = "0.61", features = ["Win32_Foundation", "Win32_Security", "Win32_System_Threading", "Win32_UI_WindowsAndMessaging"] }
```
`Win32_Security = ["Win32"]` and `Win32_System_Threading = ["Win32_System"]` exist in windows-sys-0.61.2's Cargo.toml [VERIFIED: registry source].

### Verified signatures (windows-sys 0.61.2, all `windows_link::link!` extern fns, `unsafe`)
| Fn | Module path | Signature note |
|----|-------------|----------------|
| `CreateMutexW(*const SECURITY_ATTRIBUTES, BOOL, PCWSTR) -> HANDLE` | `Win32::System::Threading` | pass `null()`, `0`, name |
| `CreateEventW(*const SECURITY_ATTRIBUTES, bManualReset: BOOL, bInitial: BOOL, PCWSTR) -> HANDLE` | `Win32::System::Threading` | auto-reset: `0`, initial `0` |
| `SetEvent(HANDLE) -> BOOL`, `ResetEvent` | `Win32::System::Threading` | |
| `WaitForSingleObject(HANDLE, u32) -> WAIT_EVENT(u32)` | `Win32::System::Threading` | compare to `WAIT_OBJECT_0`(0) / `WAIT_TIMEOUT`(258) from Foundation |
| `GetLastError() -> u32`, `CloseHandle`, `ERROR_ALREADY_EXISTS`(183) | `Win32::Foundation` | |
| `MessageBoxW(HWND, PCWSTR, PCWSTR, u32)` | `Win32::UI::WindowsAndMessaging` | hwnd = `null_mut()`; style `MB_OK \| MB_ICONINFORMATION \| MB_SETFOREGROUND \| MB_TOPMOST` |

Names are NUL-terminated UTF-16 (`encode_utf16().chain(once(0))`). Handle null = failure.

## Architecture Patterns

### Flow
```
ff.exe main -> overlay::launch()
   |
   v
instance::acquire(names)  --CreateMutexW(Local\...-overlay[-debug])
   |-- GetLastError()==0 (First) --> keep Guard {mutex, show_ev, ack_ev} alive in launch() locals
   |        |-> hotkey::register_toggle, tray::build, eframe::run_native
   |        |-> OverlayApp::update each frame: Wait(show_ev,0)==OBJECT_0
   |                 -> if Hidden { toggle_requested = true }  (reveal, never toggle)
   |                 -> SetEvent(ack_ev)
   |-- ERROR_ALREADY_EXISTS (Second) --> ResetEvent(ack); SetEvent(show_ev); Wait(ack, 2000ms)
            |-- OBJECT_0 --> return Ok(()) (exit 0)
            |-- TIMEOUT  --> MessageBoxW("FerrisFilter is already running but not responding") -> return Ok(())
```

### Recommended structure
`src/overlay/instance.rs` registered in `mod.rs` (`pub mod instance;`):
- Pure (cross-platform, unit-tested): `fn names(debug: bool) -> InstanceNames { mutex, show, ack }`, `fn instance_names() -> InstanceNames` using `cfg!(debug_assertions)`, `enum Wait { Signaled, TimedOut }` / classify of raw wait code, `const ACK_TIMEOUT_MS: u32 = 2000`.
- `mod imp` under `#[cfg(windows)]`: `acquire`, `Guard`, `Guard::take_show_request() -> bool`, `Guard::ack()`, `signal_running(names, timeout_ms) -> Wait`, `notify_unresponsive()`.
- `#[cfg(not(windows))] mod imp`: `acquire` always returns `First` with a no-op guard, `take_show_request` returns false (matches `win.rs` convention; `win.rs` has `#[cfg(not(windows))] mod imp` stubs and the crate must still compile off Windows; there is no CI, so keep it compile-clean by inspection, optionally `cargo check --target x86_64-unknown-linux-gnu` if the target is installed - not verified).

`launch()` shape:
```rust
pub fn launch() -> eframe::Result<()> {
    let guard = match instance::acquire(&instance::instance_names()) {
        instance::Acquired::First(g) => g,
        instance::Acquired::AlreadyRunning => return Ok(()), // after signal/ack/messagebox
    };
    // ...hotkey, tray, viewport as today...
    // pass guard into OverlayApp::new (field), so it lives for the whole run_native and drops at exit
}
```
Hold the guard in `OverlayApp` (or a `launch()` local moved into the creator closure) - it must outlive the window; the OS also releases on process death, so there is no stale-lock case (a major reason to use a mutex rather than a lock file).

In `update`, next to the hotkey/tray polls (before `visibility::advance`):
```rust
if self.instance.take_show_request() {          // WaitForSingleObject(show, 0) == WAIT_OBJECT_0
    if self.visibility == Visibility::Hidden { self.toggle_requested = true; }
    self.instance.ack();                         // SetEvent(ack)
}
```
Do not route through the hotkey branch (foreground gate) and do not call `toggle` when already shown. Ack after setting the request is fine: reveal happens later in the same frame via `advance`/`effect_for`/`win::reveal`.

### Watcher thread vs poll: recommend per-frame poll
Reasons: (1) tray Show, hotkey and tray click already use this exact polled pattern; (2) a watcher thread needs an `egui::Context` clone and then still relies on `request_repaint` waking a window hidden through raw `ShowWindow(SW_HIDE)` - the same uncertainty, plus thread lifecycle, plus a channel; (3) a zero-timeout wait is a cheap syscall at 20 Hz. If manual testing shows hidden-to-tray latency is unacceptable (> ~1 s), fallback is a watcher thread doing `WaitForSingleObject(show, INFINITE)` + `ctx.request_repaint()` + an `AtomicBool`. `ctx` is available in `OverlayApp::new` via `cc.egui_ctx` if needed.

### Name scheme
`Local\FerrisFilter-7c1d4e0a-3b52-4f6e-9a8d-2f0c5b9e1a47-overlay` (release), `...-overlay-debug` (debug, via `cfg!(debug_assertions)`), events `<base>-show`, `<base>-ack`. The GUID is arbitrary and merely needs to be a fixed literal [ASSUMED - generated by hand, any fresh GUID is fine]. `Local\` is per-logon-session [CITED: learn.microsoft.com Kernel Object Namespaces], which is what we want; do not use `Global\` (needs privilege in some cases, crosses sessions).

### Event semantics
- `show`: auto-reset (`bManualReset=0`), initially non-signaled. One `SetEvent` wakes exactly one successful poll and resets itself.
- `ack`: auto-reset. Second calls `ResetEvent(ack)` before `SetEvent(show)` to drop any stale ack left by an earlier timed-out second (otherwise a later second would see an old ack and falsely report success).
- Both processes call `CreateEventW` with the same name; the second creating-or-opening is race-free. Create the events inside `acquire` immediately after the mutex check on the First path so they exist before the first frame; the Second path creates/opens its own handles anyway.
- Multiple rapid second launches coalesce into one reveal; each waits for the ack. Acceptable: the ack from the first reveal may satisfy a concurrent second, all are "shown".

### Anti-Patterns
- Calling anything (even `format!`/alloc heavy fns) between `CreateMutexW` and `GetLastError`; read `GetLastError()` as the very next statement.
- Dropping the mutex handle early (`let _ = ...` drops immediately; bind to a named variable).
- Creating hotkey/tray before the gate (a second instance would grab/steal the combo registration attempt and flash a tray icon).
- Using `Global\` or file locks.
- Toggling on the second launch (must be reveal-only).

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Single instance | PID/lock file, port bind | Named mutex | Auto-released on crash, no stale state |
| Cross-process wake | Sockets, files, named pipes | Named event | Two API calls, no server |
| Hidden-window reveal | egui `ViewportCommand::Visible` | existing `win::reveal` | winit loses `SW_SHOWNOACTIVATE` after first show (documented in win.rs) |

## Common Pitfalls

### Pitfall 1: ACCESS_DENIED when first instance is elevated
Running one `ff` as admin and another normally: `CreateMutexW` may return null with `ERROR_ACCESS_DENIED` (object exists with a DACL the lower-integrity process can't open for full access). Treat null handle + `ERROR_ACCESS_DENIED` as AlreadyRunning (go to the signal path, which will likely also be denied and end in the "not responding" message box), never as First. Any other null-handle failure: fail open (continue as First, `eprintln!` a warning) since refusing to start over an exotic error is worse. [ASSUMED: behaviour inferred from Win32 object security rules; low practical risk, flagged for the planner to keep as a documented edge case, not a test]

### Pitfall 2: Hidden window not processing the signal
Mitigated by existing 50 ms `request_repaint_after` + eframe heartbeat. This rests on the code comment at app.rs ~1330 ("eframe >= 0.34 already gives invisible windows a ~10 Hz heartbeat"); the tray "Show" path depends on the same. Manual test matrix must include "hidden to tray, launch second" [MEDIUM].

### Pitfall 3: First instance still starting up
If the second launch arrives while the first is before its first frame (eframe/wgpu init can take a second or more), the show request simply waits in the event until the first frame polls it; the second may time out at 2 s and show the unresponsive message even though the first then appears. Mitigation: acceptable per CONTEXT (2 s locked); optionally treat the timeout message wording as-is. Do not extend the timeout without asking.

### Pitfall 4: Windows GUI subsystem has no console
Release `ff` uses `windows_subsystem = "windows"`; `eprintln!` is invisible, so the unresponsive case MUST use `MessageBoxW`. Debug builds have a console but the message box works the same.

### Pitfall 5: Debug and release collision
Separate names via `cfg!(debug_assertions)` mean debug+release can both run, but their hotkey registrations will conflict (second shows `HotkeyStatus::Failed` in tray, D-10). Expected and already handled; mention in the manual test notes.

### Pitfall 6: Test name collisions
Tests must use unique per-test names (include `std::process::id()` and a test label), never the production names, or a real running `ff` would respond to the test's signals / make the test see AlreadyExists.

## Testability

- **Pure unit tests (any platform):** name builder (debug has `-debug`, release doesn't, both start with `Local\`, show/ack share the base and differ), wait-result classification, timeout constant == 2000.
- **Windows in-process tests** (`#[cfg(all(test, windows))]`, like `win.rs`): named kernel objects are keyed by name, not process, so within one process a second `CreateMutexW` of the same name returns a *new handle* to the same object with `GetLastError() == ERROR_ALREADY_EXISTS`; this lets `acquire` be called twice in one test: first -> `First`, second -> `AlreadyRunning` (provided the second call's wait is handled; see below). Dropping the first guard (closes last handle) and re-acquiring yields `First` again. Handle semantics: each call returns its own handle, each needs `CloseHandle`; the object dies when the last handle closes.
- **Round-trip protocol test:** spawn a thread acting as "first" that polls `take_show_request()` then `ack()`; call `signal_running(names, 2000)` -> `Signaled`. Timeout test: no responder, `signal_running(names, 100)` -> `TimedOut` (make the timeout a parameter so tests are fast). Keep `MessageBoxW` out of the testable fn (separate `notify_unresponsive()`), as it would block.
- `acquire` should be split as `acquire_named(&InstanceNames) -> Acquired` (no MessageBox, signaling params explicit) so `launch()` composes `acquire` + `signal_running` + `notify_unresponsive`.
- **Manual (cannot automate):** launch twice while visible; while hidden to tray; with game foreground (focus must not move); double-click exe; repeated launches (5x) then hotkey still toggles and only one tray icon/stripe; kill first with Task Manager then relaunch works (mutex auto-release).
- Existing tests live in `tests/*.rs` (assert_cmd) and inline `#[cfg(test)]`; spawning `ff.exe` in tests is forbidden by the brief (do not launch ff). In-process tests above suffice.

## Code Examples

```rust
// Source: windows-sys 0.61.2 signatures (verified locally) + Win32 docs
use std::ptr::{null, null_mut};
use windows_sys::Win32::Foundation::{CloseHandle, ERROR_ALREADY_EXISTS, GetLastError, HANDLE, WAIT_OBJECT_0};
use windows_sys::Win32::System::Threading::{CreateEventW, CreateMutexW, ResetEvent, SetEvent, WaitForSingleObject};
use windows_sys::Win32::UI::WindowsAndMessaging::{MessageBoxW, MB_ICONINFORMATION, MB_OK, MB_SETFOREGROUND, MB_TOPMOST};

fn wide(s: &str) -> Vec<u16> { s.encode_utf16().chain(std::iter::once(0)).collect() }

let name = wide(&names.mutex);
let h = unsafe { CreateMutexW(null(), 0, name.as_ptr()) };
let err = unsafe { GetLastError() };            // immediately
if h.is_null() { /* ACCESS_DENIED => AlreadyRunning, else fail open */ }
else if err == ERROR_ALREADY_EXISTS { unsafe { CloseHandle(h) }; /* Second */ }
// else First: keep h in Guard; Drop => CloseHandle

// poll
let signaled = unsafe { WaitForSingleObject(show, 0) } == WAIT_OBJECT_0;
```
Guard `Drop` closes mutex/show/ack handles.

## State of the Art
No relevant change; named mutex/event is the long-standing idiom. `windows-sys` 0.61 uses `windows_link::link!` and module-per-feature layout (verified).

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | Hand-made GUID is fine as a name component | Name scheme | None (any unique literal) |
| A2 | Elevated-vs-normal produces ACCESS_DENIED on CreateMutexW | Pitfall 1 | Edge case only; message box path still prevents two stripes |
| A3 | `cargo check` for a non-Windows target is not run in CI/locally | Structure | Stub could rot; low risk, mirrors win.rs convention |

## Open Questions (RESOLVED)

1. **Does the 50 ms/heartbeat polling really run while the stripe is SW_HIDE'd?**
   - Known: tray Show already relies on it and the code comments assert a ~10 Hz heartbeat.
   - Unclear: not re-verified in this session (ff must not be launched).
   - Recommendation: manual test "hidden then launch second"; fallback is watcher thread + `ctx.request_repaint()`.
   - RESOLVED: 22-02 relies on the existing `request_repaint_after(50ms)` + eframe hidden-window heartbeat (same path tray Show uses); manual check 2 in 22-MANUAL-VERIFICATION.md verifies it, and the documented fallback if it fails is a watcher thread that waits on the show event and calls `ctx.request_repaint()`.
2. **Should an already-visible stripe be brought to front?** It is always-on-top, so no action needed; recommend none. RESOLVED: no action; the poll only reveals a hidden stripe and acks.

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| windows-sys 0.61.2 source | build | yes | 0.61.2 | — |
| Rust toolchain on Windows | build/test | yes (project builds) | — | — |

Build/test with `CARGO_TARGET_DIR=C:/Users/jonat/AppData/Local/Temp/ff-target` (running ff.exe may lock target\debug).

## Validation Architecture
Skipped: `workflow.nyquist_validation` is `false` in `.planning/config.json`. (Test approach is described under Testability.)

## Project Constraints (from CLAUDE.md)
- Keep `CHANGELOG.md` `[Unreleased]` current as work lands.
- Layer rule (PITFALLS §8): pure logic testable without a window; `app.rs` presentation-only. Hence `instance.rs` holds the logic, `app.rs` only calls `take_show_request`/`ack`.
- Raw Win32 lives in `win.rs` by convention (T-13-09 "every unsafe in one module"); recommend either putting the new unsafe in `instance.rs` with the same SAFETY-comment style, or a note in `win.rs` docs, and have the planner decide; do not add a new crate.
- Do not hand-edit generated files; update CLAUDE.md overlay note (one sentence on single-instance behaviour and the debug/release name split) when done.
- Docs: no emojis.

## Security Domain

| ASVS Category | Applies | Standard Control |
|---------------|---------|-----------------|
| V2/V3/V4 | no | — |
| V5 Input Validation | no | Event carries no payload |
| V6 Cryptography | no | — |

Threat: another local process in the same session can `SetEvent` the show event (causes only a reveal) or pre-create the mutex name to block `ff` from starting (denial of start; same-user, low severity, accepted). Null `SECURITY_ATTRIBUTES` gives the default same-user DACL.

## Sources

### Primary (HIGH)
- Local source: `Cargo.toml`, `Cargo.lock`, `src/overlay/{mod,app,tray,hotkey,visibility,win}.rs`, `src/bin/ff.rs`
- windows-sys 0.61.2 crate source in `~/.cargo/registry` (fn signatures, constants, feature table)

### Secondary (MEDIUM)
- Win32 docs knowledge: CreateMutexW/GetLastError ERROR_ALREADY_EXISTS semantics, `Local\` namespace [CITED: learn.microsoft.com/windows/win32/termserv/kernel-object-namespaces]; not re-fetched this session

## Metadata
- Standard stack: HIGH; Architecture: HIGH; Pitfalls: MEDIUM (hidden-repaint, elevation unverified at runtime)

**Research date:** 2026-10-13
**Valid until:** 2026-11-12
