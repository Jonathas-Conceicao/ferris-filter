# Phase 23: Auto-Reload Filter in Game on Apply - Pattern Map

**Mapped:** 2026-10-13
**Files analyzed:** 9 (2 new, 7 modified) plus 1 manual doc
**Analogs found:** 9 / 10 (the SendInput/clipboard Win32 helpers have no in-repo analog; use RESEARCH.md)

## File Classification

| New/Modified File | Role | Data Flow | Closest Analog | Match Quality |
|---|---|---|---|---|
| `src/overlay/chat.rs` (new) | service (pure step list + trigger decision + sink trait) | event-driven (one-shot sequence) | `src/overlay/economy.rs` (pure state/worker seam) + `src/overlay/apply.rs` (pure outcome/messages) | role-match |
| `src/overlay/win.rs` (modify: `is_game_title`, `find_game_window`, `WinSink`, stand-ins) | utility (raw Win32) | request-response | itself, `foreground_is_game_or_overlay` / `work_area` | exact |
| `src/overlay/app.rs` (modify: `run_apply` hook, worker + poll, `ConfigAction` handler, `settings_with_filter`, ctx construction) | controller | event-driven (worker + mpsc) | `EconomySync` use at app.rs L1276 / L1506-1531, `switch_to` L743 | exact |
| `src/overlay/apply.rs` (modify: message builders) | utility | transform | `success_message` / `unchanged_message` | exact |
| `src/settings.rs` (modify: `reload_after_apply`) | model | CRUD (file I/O) | `Settings.filter` + `filter_name()` | exact |
| `src/overlay/config_view.rs` (modify: `ConfigAction`, `ConfigViewCtx`, `show_filter`) | component | request-response | `show_filter` + `ConfigAction::SelectFilter` | exact |
| `src/overlay/mod.rs` (modify: `mod chat;`) | config | n/a | existing `mod` list | exact |
| `Cargo.toml` (modify: windows-sys features) | config | n/a | line 42 | exact |
| `CLAUDE.md`, `CHANGELOG.md` | docs | n/a | existing entries | exact |
| `23-MANUAL-VERIFICATION.md` (new) | test (manual) | n/a | `.planning/phases/22-*/22-MANUAL-VERIFICATION.md` | exact |

## Pattern Assignments

### `src/overlay/chat.rs` (service, event-driven, pure seam)

**Analogs:** `src/overlay/economy.rs` (worker + `poll`, tests with injected closure so no I/O), `src/overlay/apply.rs` (pure, no GUI types, `ApplyOutcome`).

No egui or Win32 types in this file (apply.rs module doc: "Pure -- no GUI-framework types of any kind anywhere in this file"). Use the `Step` / `Key` / `InputSink` / `reload_steps()` / `execute()` / `decide()` shapes from RESEARCH.md Patterns 1 and 2 verbatim.

**Worker-thread + mpsc pattern to copy** (`economy.rs` L83-123, L145-176):
```rust
pub struct EconomySync { state: SyncState, rx: Option<Receiver<Result<String, String>>> }

pub fn start<F>(&mut self, work: F) where F: FnOnce() -> Result<String, String> + Send + 'static {
    if self.is_running() { return; }          // double-click guard -> use for reload_in_flight
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        // A send failure means the app shut down mid-sync; dropping it is correct.
        let _ = tx.send(work());
    });
    self.rx = Some(rx);
    self.state = SyncState::Running;
}

pub fn poll(&mut self) -> bool {
    let Some(rx) = self.rx.as_ref() else { return false; };
    match rx.try_recv() {
        Ok(Ok(league)) => { /* set Finished */ self.rx = None; true }
        Ok(Err(e)) => { /* set Failed */ self.rx = None; true }
        Err(TryRecvError::Empty) => false,
        Err(TryRecvError::Disconnected) => {
            // worker died without sending (panic): report, never leave in-flight forever
            self.rx = None; true
        }
    }
}
```
Recommendation: model the reload as a sibling small struct (e.g. `ReloadSend { rx: Option<Receiver<Result<(), String>>> }`) with the same `start`/`poll`/`is_running` and the same `Disconnected` arm. The worker closure takes the `Vec<Step>` plus a `Box<dyn InputSink + Send>` (production: `WinSink`; tests: a fake). `reload_in_flight` is `is_running()`.

**Test pattern** (`economy.rs` tests ~L180-190): wall-clock-deadline `settle()` loop polling until a result lands rather than a yield count; closures injected in place of real work. Copy for the worker test. `RecordingSink` / `FailingSink` per RESEARCH Pattern 1.

**Cleanup rule:** after `execute` returns `Err`, run `Step::RestoreClipboard` once if `SaveClipboard` ran (RESEARCH Pattern 1 note).

---

### `src/overlay/win.rs` (utility, raw Win32)

**Analog:** itself. Keep the structure `#[cfg(windows)] mod imp { ... }`, `#[cfg(not(windows))] mod imp { stand-ins }`, then `pub use imp::{...}` (L170-172).

**Import style** (L41-52):
```rust
use windows_sys::Win32::Foundation::{HWND, RECT};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    GetForegroundWindow, GetWindowTextW, SPI_GETWORKAREA, SW_HIDE, SW_SHOWNOACTIVATE,
    ShowWindow, SystemParametersInfoW,
};
```
Add `EnumWindows`, `IsWindowVisible`, `IsIconic`, `SetForegroundWindow`, `GetWindowThreadProcessId`, `SendInput` etc. to these lists.

**Title match to factor out** (L133-140, currently inline in `foreground_is_game_or_overlay`):
```rust
let mut buf = [0u16; 256];
// SAFETY: `GetWindowTextW` writes at most `buf.len()` UTF-16 units into the buffer we
// pass and returns how many it wrote, excluding the terminator. ...
let len = unsafe { GetWindowTextW(fg, buf.as_mut_ptr(), buf.len() as i32) };
if len <= 0 { return false; }
let title = String::from_utf16_lossy(&buf[..len as usize]);
title.to_lowercase().contains("path of exile")
```
Extract `pub fn is_game_title(title: &str) -> bool` as a pure fn OUTSIDE `mod imp` (so it is testable on any platform; the existing `tests` module is `#[cfg(all(test, windows))]`, so add a separate `#[cfg(test)]` test module for it). Have `foreground_is_game_or_overlay` call it. Tests: "Path of Exile 2" yes, "Exiled Exchange 2" no, case-insensitive.

**SAFETY comment discipline** (every `unsafe` block, e.g. L68-75, L96-110): one comment stating handle provenance / buffer capacity / checked return. Follow for every new `unsafe` (EnumWindows callback context pointer, SendInput array length, GlobalLock).

**`cfg(not(windows))` stand-in style** (L146-168): trivial no-op fns with doc comment. Add stand-ins for `find_game_window` (returns `None`) and a `WinSink` that returns `Err("not supported on this platform".into())`.

**Send-handle note:** `OverlayWindow` has an `unsafe impl Send` with a justifying SAFETY comment (L56-62). A `GameWindow(HWND)` newtype moved into the worker thread needs the same.

**No in-repo analog (use RESEARCH.md "Pattern 3", "Focus rules", "Text entry decision"):** `SendInput` with wVk + `MapVirtualKeyW` wScan, clipboard save/set/restore with `GetClipboardSequenceNumber` guard, `EnumWindows` callback, `IsIconic`/`SW_RESTORE`/`SetForegroundWindow`/verify, modifier check via `GetAsyncKeyState`. Skip the browser-tab exe check unless cheap (RESEARCH Open Question 2).

---

### `src/overlay/app.rs` (controller, event-driven)

**Analog 1 - the hook point, `run_apply`** (L780-835). Insert the decision in the `Written` arm only:
```rust
let (outcome, extras, needs_reload) = apply_working(root, loaded, working);
match outcome {
    ApplyOutcome::Written(path) => {
        self.apply_message = Some(format!("{}{extras}", apply::success_message(&path)));
        self.apply_state = self.apply_state.on_apply_success();
        self.config_changed_pending = false;
        // <- chat::decide(...) here; Send => spawn worker, interim message
        //    "Wrote <path>. Reloading in game..."; Skip(reason) => success_message + reason
    }
    ApplyOutcome::Unchanged(path) => { ... }   // never sends
    ApplyOutcome::Failed(message) => { ...; return; }  // never sends
}
```
Note `root`/`loaded`/`working` are borrowed from `self` fields at the top of `run_apply` (let-else on `&self.root, &mut self.filter, &mut self.working`); do the setting read and `win::find_game_window` after the borrows end or read the bool into a local first. Keep `{extras}` appended as the existing arm does.

**Analog 2 - worker poll each frame** (L1276-1281):
```rust
// Collect a finished economy sync. The state machine only advances here, so this must
// run every frame. A landed result also forces a repaint ...
if self.economy.poll() {
    ui.ctx().request_repaint();
}
```
Add the reload poll next to it; on completion replace `apply_message` (Sent -> `"Wrote <path>. Reloaded in game"`; Err(cause) -> `success_message` + `(could not reload: <cause>)`). Store the written path for the replacement message. The worker may also call `ctx.request_repaint()` via a cloned `egui::Context` (the stripe is drawn over a game that may not generate input events).

**Analog 3 - save-on-change handler, `switch_to`** (L743-760) and `settings_with_filter` (L570-575):
```rust
fn settings_with_filter(current: &Settings, name: &str) -> Settings {
    Settings { filter: Some(name.to_string()), extra: current.extra.clone() }
}
fn switch_to(&mut self, name: String) {
    let next = settings_with_filter(&self.settings, &name);
    let save_error = match &self.root {
        Ok(root) => settings::save(root, &next).err(),
        Err(_) => None,
    };
    self.settings = next;
    ...
    if let Some(err) = save_error {
        self.config_message = Some((format!("Could not save ff-settings.json: {err}"), true));
    }
}
```
New `ConfigAction::SetReloadAfterApply(v)` handler: `Settings { reload_after_apply: Some(v), ..self.settings.clone() }`, same save/error pattern, must NOT touch `config_changed_pending` / `compute_config_pending`. **Pitfall:** `settings_with_filter` must be updated to copy `reload_after_apply` (it builds `Settings` from scratch); extend its existing test at app.rs ~L1994-2000.

**Action dispatch site** (L905-985): add the new arm beside `ConfigAction::SelectFilter(name) => self.select_filter(name),`.

**ConfigViewCtx construction sites (both must gain the new field):** L1058-1067 (production) and L1909-1918 (test, `ConfigViewCtx { filters: None, active_filter: "dynamic.filter", ... out_of_date: false }`). Production value: `reload_after_apply: self.settings.reload_after_apply()`.

---

### `src/overlay/apply.rs` (utility, transform)

**Analog:** `success_message` / `unchanged_message` (L74-93) and `RELOAD_INSTRUCTION` (L22):
```rust
pub const RELOAD_INSTRUCTION: &str = "Options > Game > Item Filter > Refresh";

pub fn success_message(written: &Path) -> String {
    format!("Wrote {}. Reload it in-game: {RELOAD_INSTRUCTION}", written.display())
}
```
Keep `success_message` unchanged (existing tests assert the Refresh substring verbatim). Add `reloaded_message(path)` -> `"Wrote {path}. Reloaded in game"` (plus the wrong-filter hint per RESEARCH Pitfall 8 if desired), `reloading_message(path)` (interim), and `not_reloaded_message(path, reason)` = `format!("{} ({reason})", success_message(path))`. Reason strings as `pub const`s (ASCII only: the stripe's Cinzel font lacks many glyphs, see the comment above `RELOAD_INSTRUCTION`). Unit-test each string in the existing `tests` module. Update the module doc claim "PoE2 never re-reads it on its own" only if it becomes misleading.

---

### `src/settings.rs` (model, file I/O)

**Analog:** the `filter` field (L22-37):
```rust
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Settings {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub filter: Option<String>,
    /// Unknown top-level keys survive a load/save round trip.
    #[serde(flatten)]
    pub extra: serde_json::Map<String, serde_json::Value>,
}
impl Settings {
    pub fn filter_name(&self) -> &str { self.filter.as_deref().unwrap_or(DEFAULT_FILTER) }
}
```
Add `#[serde(default, skip_serializing_if = "Option::is_none")] pub reload_after_apply: Option<bool>` and `pub fn reload_after_apply(&self) -> bool { self.reload_after_apply.unwrap_or(true) }`. Existing key convention is the bare lowercase `filter`; use snake_case `reload_after_apply` unless a camelCase key exists (none does here).

**Second construction site that will break (compile error is the prompt):** `load`'s salvage path (L61-78) builds `Settings { filter: None, extra }`; add `reload_after_apply: None`, and ALSO `extra.remove("reload_after_apply")` there for symmetry with `extra.remove("filter")` (otherwise a wrongly typed value would be duplicated in `extra` and the named field on save).

**Tests to copy** (settings.rs L198+): `round_trip_preserves_unknown_keys` (L222-236) and `non_string_filter_keeps_other_keys` (L211-219) using `tempfile::tempdir()`, `fs::write(path(t.path()), ...)`, `load`, `save`. Add: absent key -> `reload_after_apply()` true; `false` round-trips with an unknown key intact; wrong-typed value salvages other keys; a never-touched file serializes byte-identical.

---

### `src/overlay/config_view.rs` (component, request-response)

**Analog:** `show_filter` (L276-352), `ConfigAction` (L38-52), `ConfigViewCtx` (L102-115), `helper` (L214).

Enum: add `SetReloadAfterApply(bool)` (derive `PartialEq` already present).

Ctx: add `pub reload_after_apply: bool,`.

UI insertion point is the `else` branch end of `show_filter`, just before the final caption:
```rust
} else {
    helper(ui, "Saved on this computer. Default: dynamic.filter.");
}
```
Place the toggle after the `pending_switch` block / before this helper (or after it), inside the same `egui::Frame`. Use `widgets::toggle_chip(ui, label, &mut bool)` (widgets.rs L258; "callers write only on `.changed()`", exactly like `ui.checkbox`) or `ui.checkbox`; push the action on change:
```rust
let mut on = ctx.reload_after_apply;
if toggle_chip(ui, "Reload filter in game after Apply", &mut on).changed() {
    actions.push(ConfigAction::SetReloadAfterApply(on));
}
helper(ui, "Sends /reloaditemfilter to the game. It reloads the filter selected in game, so select the same filter there. Close the chat box first.");
```
Check that `toggle_chip` and `helper` are in scope (helper is file-private; toggle_chip import from `super::widgets`). Keep text ASCII.

---

### `Cargo.toml` (config)

Current (L42):
```toml
windows-sys = { version = "0.61", features = ["Win32_Foundation", "Win32_Security", "Win32_System_Threading", "Win32_UI_WindowsAndMessaging"] }
```
Append `"Win32_UI_Input_KeyboardAndMouse"`, `"Win32_System_DataExchange"`, `"Win32_System_Memory"`. Prefer a local `const CF_UNICODETEXT: u32 = 13;` over adding `Win32_System_Ole` (RESEARCH).

### `src/overlay/mod.rs`
Add `mod chat;` (or `pub mod`) matching how `economy`/`apply` are declared there.

### Docs
- `CLAUDE.md`: overlay notes at "the stripe also repeats the reload path after every successful Apply" and "PoE2 never re-reads a filter on its own" in the rule-editor paragraph; update the `apply.rs` doc wording as needed; keep the `ferrisfilter apply` row's manual Refresh note; add the wrong-selected-filter caveat and the "close chat first / same privilege level" notes.
- `CHANGELOG.md`: `[Unreleased]` entry.

### `23-MANUAL-VERIFICATION.md`
**Analog:** `.planning/phases/22-*/22-MANUAL-VERIFICATION.md` (also `13-MANUAL-VERIFICATION.md`). Step 0: type `/reloaditemfilter` by hand and confirm the command exists (RESEARCH Open Question 1). Then the CONTEXT "Specifics" matrix.

## Shared Patterns

### Seam-first, no real input in tests
**Source:** `economy.rs` (`start(work)` takes a closure) and RESEARCH Pattern 1. **Apply to:** `chat.rs`, app wiring. Production passes `WinSink`; tests pass a recording/failing sink. `WinSink` is never constructed in a test.

### Win32 only in `win.rs`
**Source:** `win.rs` module docs and SAFETY comments. **Apply to:** all new unsafe. `chat.rs` and `app.rs` call only safe wrappers.

### Save-on-change settings with preserved `extra`
**Source:** `switch_to` + `settings_with_filter` (app.rs L570, L743). **Apply to:** the reload toggle handler.

### Footer/status messages are ASCII and live in `apply.rs`
**Source:** `apply.rs` L16-22 comment and `success_message`. **Apply to:** all new message strings.

### Build/test
`CARGO_TARGET_DIR=C:/Users/jonat/AppData/Local/Temp/ff-target cargo test` (a running ff.exe may lock `target\debug`). Fixtures untouched.

## No Analog Found

| File/Part | Role | Data Flow | Reason |
|---|---|---|---|
| `SendInput` key injection, clipboard save/restore, `EnumWindows` in `win.rs` | utility | request-response | No synthetic input or clipboard code exists in the repo; use RESEARCH.md Pattern 3 and the cited EE2/APT/libuiohook sources |

## Metadata

**Analog search scope:** `src/overlay/{win,app,apply,economy,config_view,widgets}.rs`, `src/settings.rs`, `Cargo.toml`, phase 22 directory
**Files scanned:** about 8 read in targeted ranges
**Pattern extraction date:** 2026-10-13
