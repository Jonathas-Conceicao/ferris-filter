# Phase 22: Single-Instance Overlay - Pattern Map

**Mapped:** 2026-10-13
**Files analyzed:** 6
**Analogs found:** 6 / 6

## File Classification

| New/Modified File | Role | Data Flow | Closest Analog | Match Quality |
|-------------------|------|-----------|----------------|---------------|
| `src/overlay/instance.rs` (new) | utility (Win32 wrapper + pure naming) | event-driven (cross-process signal) | `src/overlay/win.rs` | exact (module convention) |
| `src/overlay/mod.rs` (modify: `pub mod instance;`, `launch()` gate) | entry point | request-response | itself (`launch`, lines 1-94) | exact |
| `src/overlay/app.rs` (modify: field, `new` param, poll in `update`) | component | event-driven (polled) | tray Show poll, `app.rs` 1278-1295 | exact |
| `Cargo.toml` (modify: features) | config | n/a | line 41-42 | exact |
| `CLAUDE.md` / `CHANGELOG.md` (modify) | docs | n/a | existing overlay notes | n/a |
| tests inside `instance.rs` | test | event-driven | `win.rs` tests (lines 180-202) | role-match |

`src/bin/ff.rs` needs no change (`fn main() -> eframe::Result<()> { ferrisfilter::overlay::launch() }`).

## Pattern Assignments

### `src/overlay/instance.rs` (utility, event-driven)

**Analog:** `src/overlay/win.rs`

**Module layout** (win.rs 32-33, 151-178): `#[cfg(windows)] mod imp { ... }`, then `#[cfg(not(windows))] mod imp { stubs }`, then a re-export line.
```rust
#[cfg(windows)]
mod imp {
    use windows_sys::Win32::Foundation::{HWND, RECT};
    use windows_sys::Win32::UI::WindowsAndMessaging::{ ... };
    ...
}
#[cfg(not(windows))]
mod imp {
    /// Non-Windows stand-in so the library still compiles off Windows ...
    pub struct OverlayWindow;
    pub fn reveal(_w: &OverlayWindow) {}
}
pub use imp::{OverlayWindow, conceal, foreground_is_game_or_overlay, from_frame, reveal, work_area};
```

**Unsafe style** (win.rs 69-76): every `unsafe` block gets a `// SAFETY:` comment that states handle provenance and checks return values.
```rust
// SAFETY: `w.0` is our own window's HWND ...
unsafe { ShowWindow(w.0, SW_SHOWNOACTIVATE); }
```
Return-value checking (win.rs 103-108): `let ok = unsafe {..}; if ok == 0 { return None; }`.

**Core pattern to write** (from RESEARCH Code Examples):
- Pure, cross-platform items outside `imp`: `names(debug: bool)`, `instance_names()` using `cfg!(debug_assertions)`, `ACK_TIMEOUT_MS = 2000`.
- `wide()` helper: `s.encode_utf16().chain(once(0)).collect()`.
- `CreateMutexW(null(), 0, name)`, then `GetLastError()` as the very next statement.
- Null handle plus `ERROR_ACCESS_DENIED` means AlreadyRunning. Any other null: fail open, `eprintln!`, treat as First.
- `Guard` has a `Drop` that calls `CloseHandle` on all three handles. Provide `take_show_request()` (`WaitForSingleObject(show, 0) == WAIT_OBJECT_0`) and `ack()` (`SetEvent`).
- Split into `acquire_named(&InstanceNames) -> Acquired`, `signal_running(names, timeout_ms) -> Wait`, and `notify_unresponsive()` (`MessageBoxW`). Keep the message box out of the testable functions.
- Non-Windows stub: `acquire` returns `First` with a no-op guard, and `take_show_request` returns false.

**Tests** (win.rs 180-202): use `#[cfg(all(test, windows))] mod tests`, with a doc comment saying why the test needs no real window. Use unique names per test, such as `format!("...-{}-{label}", std::process::id())`. Never use production names. Cover:
- `acquire` twice in one process gives First then AlreadyRunning.
- Drop then re-acquire gives First.
- A responder thread gives `Signaled`.
- No responder with a 100 ms timeout gives `TimedOut`.
- Pure tests for the names.

---

### `src/overlay/mod.rs` `launch()` (entry point)

**Analog:** itself.

Insert the gate as the first statement, before `hotkey::register_toggle()` (line 34) and `tray::build` (line 52). The existing comment at lines 27-33 on thread affinity stays valid.
```rust
pub fn launch() -> eframe::Result<()> {
    let guard = match instance::acquire(&instance::instance_names()) {
        instance::Acquired::First(g) => g,
        instance::Acquired::AlreadyRunning => return Ok(()), // signal + ack wait + MessageBox happen inside the composed helper
    };
    let (hotkey_manager, hotkey_status) = hotkey::register_toggle();
    ...
```
Move `guard` into the creator closure (`Box::new(move |cc| { ... })`, lines 80-93) and pass it to `OverlayApp::new(tray, hotkey_manager, hotkey_status, guard)`. The guard must outlive `run_native`. Add `pub mod instance;` to the alphabetical list (after `hotkey`, before `layout`).

---

### `src/overlay/app.rs` (component, event-driven polled)

**Analog:** the tray MenuEvent Show handling, lines 1278-1295.
```rust
Some(OverlayCommand::Show) => {
    // D-17: "Show" only means something while currently hidden ...
    if self.visibility == Visibility::Hidden {
        self.toggle_requested = true;
    }
}
```
**Add the poll** after the tray MenuEvent block (ends 1295) and before `visibility::advance` (line 1301). Do not place it inside the hotkey branch (lines 1231-1247), which has the `win::foreground_is_game_or_overlay` gate.
```rust
if self.instance.take_show_request() {
    if self.visibility == Visibility::Hidden { self.toggle_requested = true; }
    self.instance.ack();
}
```
**Struct field** (struct at line 60, fields follow the doc-comment-per-field style): add `instance: instance::Guard`, documented as "held for the process lifetime; dropping releases the single-instance mutex". Add the matching `use super::instance;` next to `use super::win::{self, OverlayWindow};` (line 57).
**Constructor** (`new`, line 572, struct literal at line 592): add the `guard` parameter and the `instance: guard` field.
**Wake-up:** `ui.ctx().request_repaint_after(Duration::from_millis(50))` at line 1337 already polls every frame, and the comment at 1331-1336 documents the heartbeat for hidden windows. No extra `request_repaint` is needed. If manual testing shows hidden latency over about 1 s, fall back to a watcher thread with `cc.egui_ctx` (RESEARCH).

---

### `Cargo.toml` (config)

Current line 42:
```toml
windows-sys = { version = "0.61", features = ["Win32_Foundation", "Win32_UI_WindowsAndMessaging"] }
```
Change to add `"Win32_Security"` and `"Win32_System_Threading"`.

## Shared Patterns

### Raw Win32 isolation
**Source:** `src/overlay/win.rs` module docs (lines 1-4, T-13-09), which say all overlay `unsafe` lives in `win.rs`. Phase 22 puts a second home for `unsafe` in `instance.rs`. Add a one-line note in the `instance.rs` module docs, and optionally one in `win.rs`, so the convention is explicitly extended.

### Layer rule
**Source:** `src/overlay/mod.rs` lines 1-7. Logic lives in the library module, and `app.rs` stays presentation-only (it only calls `take_show_request` and `ack`).

### Reveal path
`visibility::advance` and `effect_for`, then `win::reveal` (`SW_SHOWNOACTIVATE`, no focus steal). Do not use egui `ViewportCommand::Visible`.

### Docs
Update `CHANGELOG.md` `[Unreleased]`. Add one sentence to the CLAUDE.md overlay note on single-instance behaviour and the debug/release name split. No emojis.

## No Analog Found

| File | Role | Reason |
|------|------|--------|
| (none) | | There is no existing named-kernel-object or MessageBox code. Use the RESEARCH signatures (windows-sys 0.61.2 verified). |

## Metadata

**Analog search scope:** `src/overlay/`, `src/bin/ff.rs`, `Cargo.toml`
**Files read:** mod.rs, win.rs, app.rs (targeted ranges), ff.rs
**Pattern extraction date:** 2026-10-13
