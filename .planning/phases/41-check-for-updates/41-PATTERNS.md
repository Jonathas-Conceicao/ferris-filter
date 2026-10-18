# Phase 41: Check for Updates - Pattern Map

**Mapped:** 2026-10-18
**Files analyzed:** 14 new/modified
**Analogs found:** 14 / 14 (all in-repo, symbol-anchored; phases 37-40 are not all executed, so re-read each file at execution time)

Anchor rule: every excerpt is cited by file plus symbol, never by line number. Where an analog only exists after an earlier phase lands, the producing plan file is named next to the current-code analog.

## File Classification

| New/Modified File | Role | Data Flow | Closest Analog | Match Quality |
|-------------------|------|-----------|----------------|---------------|
| `src/update.rs` (NEW: consts, `UpdateHttp` + `ReqwestUpdateHttp`, `classify_latest`, `parse_sums`, `run_check`, `message_for`, `spawn_installer`, date fns) | service | request-response + streaming file-I/O | `src/sync/client.rs` `HttpClient`/`ReqwestClient` (+ `src/sync/mod.rs` `sync_economy_blocking` runtime scaffold) | role-match |
| `src/lib.rs` (`pub mod update;`) | config | n/a | existing `pub mod` list | exact |
| `Cargo.toml` / `Cargo.lock` (`semver`, `ring`, optional `windows-sys` `Win32_UI_Shell`) | config | n/a | existing `[dependencies]` block | exact |
| `src/sync/client.rs` (reuse `USER_AGENT`, no edit) | config | n/a | created by `40-03-PLAN.md` | n/a |
| `src/overlay/update_job.rs` (NEW: `UpdateState`, `UpdateJob`, `UpdateMsg`, caption/enabled/tooltip tables, install intent) | model (framework-free) | event-driven (worker + channel, multi-message) | `src/overlay/economy.rs` `EconomySync`; after 39-03 also `src/overlay/leagues.rs` `LeagueList` | exact (plus stage messages) |
| `src/overlay/mod.rs` (`pub(crate) mod update_job;`, `launch()` pending-install slot and spawn after `run_native`; `run_screenshot` unchanged) | controller | event-driven | same file `launch()` / `run_screenshot` | exact |
| `src/overlay/about_view.rs` (`about_tab` takes view, returns action) | component | request-response | same file `about_tab`; return-an-action shape from `shell::economy_tab` + `EconomyClicks` | exact |
| `src/overlay/app.rs` (fields, `poll`, click handling, `update_message`, `begin_shutdown` hook, intent clearing, call-site change) | controller | event-driven | same file: `self.economy.poll()` block, `OverlayCommand::Quit` arm (`request_quit` after 40-01), `begin_shutdown`, `dispatch_popup` `Popup::Quit` arm | exact |
| `src/overlay/shell.rs` (`event_from` gains an update slot) | utility | transform | same file `event_from` | exact |
| `src/overlay/instance.rs` (`notify_error`, MB_ICONERROR variant) | utility (Win32 FFI) | request-response | same file `notify` | exact |
| `src/overlay/win.rs` (optional `shell_execute_open` for error 740) | utility (Win32 FFI) | request-response | existing `// SAFETY:`-commented fns in `win.rs` | role-match |
| `src/overlay/screenshot.rs` (`--update-state`) | utility | transform | same file `--tier-text` / `--stage-economy` | exact |
| `CLAUDE.md` (About-tab update behaviour, new modules, `--update-state`) | docs | n/a | existing Economy-sync paragraph | exact |
| tests (inline `mod tests` in `update.rs`, `update_job.rs`, `about_view.rs`, `app.rs`) | test | n/a | `economy.rs` tests (`settle`), `sync/metadata.rs` fake client, `about_view.rs` `render` | exact |

README.md and CHANGELOG.md are NOT edited by this phase (hand-written, per project memory).

## Pattern Assignments

### `src/update.rs` (service, request-response + streaming)

**Analog:** `src/sync/client.rs` (`ReqwestClient::new`, trait seam) and `src/sync/mod.rs` `sync_economy_blocking` (runtime inside a worker).

**Provider + client construction** (copy the first two lines verbatim; change the builder per 41-RESEARCH Pattern 1: `USER_AGENT`, `https_only(true)`, `connect_timeout`, `read_timeout`, and NO client-wide `.timeout()`):
```rust
// Install the ring crypto provider for rustls (required when using rustls-no-provider).
// Safe to call multiple times — the provider is set once globally.
let _ = rustls::crypto::ring::default_provider().install_default();

let inner = reqwest::Client::builder()
    .user_agent(/* USER_AGENT */)
    .timeout(std::time::Duration::from_secs(30))   // do NOT keep this: it bounds the whole installer body
    .build()
    .map_err(|e| miette::miette!("Cannot build HTTP client: {e}"))?;
```
`USER_AGENT` does not exist yet: `pub const USER_AGENT` is created by `.planning/phases/40-release-pipeline-installer/40-03-PLAN.md` (Task with `concat!("FerrisFilter/", env!("CARGO_PKG_VERSION"), ...)`). Import `crate::sync::client::USER_AGENT`; do not redefine. Current code has an inline literal in `ReqwestClient::new`, so execute after 40-03.

**Why not `HttpClient`:** `ReqwestClient::get_json` ends in `.error_for_status()` and returns `String`, losing 404 vs 403 vs 429, headers and binary bodies. Copy only the trait-seam idea:
```rust
pub trait HttpClient {
    fn get_json(&self, url: &str, query: &[(&str, &str)])
        -> impl std::future::Future<Output = miette::Result<String>>;
}
```
Phase 41 defines its own synchronous `UpdateHttp { get(..) -> Result<Reply, FetchError>; download(.., sink) -> Result<u16, FetchError> }` (shape in 41-RESEARCH Pattern 1), reqwest impl owns a runtime, tests use a `HashMap`-backed fake.

**Runtime scaffold** (from `sync_economy_blocking`, copy inside `ReqwestUpdateHttp` methods or `run_check`'s caller):
```rust
let rt = tokio::runtime::Runtime::new()
    .map_err(|e| miette::miette!("Cannot create async runtime: {e}"))?;
rt.block_on(async { ... })
```
(Use `String`/`FetchError`, not `miette`, in `update.rs` signatures: the worker reports plain text.)

**Fake client for tests:** `src/sync/metadata.rs` `mod tests` `FakeClient` (HashMap of url -> canned reply). Tests pass `tempfile::tempdir()` as the temp root to `run_check` and assert the root is empty after every failure (RAII `TempDir`), holding exactly one folder after success. No test constructs `ReqwestUpdateHttp`.

**Date helper reuse:** `crate::sync::prices::days_to_ymd` (as used by `shell::last_sync_text`):
```rust
let local = (secs as i64 + i64::from(utc_offset_minutes) * 60).max(0) as u64;
let (year, month, day) = crate::sync::prices::days_to_ymd(local / 86_400);
format!("{year:04}-{month:02}-{day:02} ...")
```
Add only the inverse `days_from_civil` (41-RESEARCH Pattern 8). The worker carries `published_unix: Option<i64>`; format on the UI thread with `win::utc_offset_minutes()` (same parameter `last_sync_text` takes), keeping the function pure. Date-only variant (`YYYY-MM-DD`): do not reuse `last_sync_text` (it appends `HH:MM`); share nothing but `days_to_ymd`.

**Error/message pattern:** `economy.rs` `success_message` / `failure_message` are pure `fn -> String` producers; mirror that with `message_for(&CheckResult, offset) -> (String, StatusKind)` carrying the UI-SPEC table verbatim, table-tested for every row, including kinds (Warning: offline, rate limited; Error: other failures; Ok: up to date; Info: checking, downloading, available). `StatusKind` lives in `crate::skin::widgets` (imported as `w::StatusKind` in `app.rs`); to keep `update.rs` framework-free return a local kind enum or `update_job.rs` maps it, as `shell::event_from` does with `bool` flags.

**Semver (new dependency, no in-repo analog):** compare against `Version::parse(env!("CARGO_PKG_VERSION"))` reduced to major.minor.patch, NOT `env!("FF_VERSION")` (that is `git describe`, can be `0.6` or `0.6-dev`; see `about_view::version_text`).

### `src/overlay/update_job.rs` (model, event-driven)

**Analog:** `src/overlay/economy.rs` `EconomySync` (module doc: "Framework-free by contract"; no egui types in signatures). Third copy of the pattern: 39-RESEARCH/`39-03-PLAN.md` deliberately left `EconomySync` alone and added `LeagueList` (`ListState`, `start`/`poll`/`state`/`inject`) in `src/overlay/leagues.rs`. Phase 41 must not refactor either worker (the brief's "generic worker from Phase 39" does not exist; 41-RESEARCH correction 1).

**Worker core to mirror** (`EconomySync::start` / `poll`):
```rust
pub fn start<F>(&mut self, work: F)
where F: FnOnce() -> Result<String, String> + Send + 'static,
{
    if self.is_running() { return; }
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        // A send failure means the app shut down mid-sync; the result has nowhere to go and
        // dropping it is correct.
        let _ = tx.send(work());
    });
    self.rx = Some(rx);
    self.state = SyncState::Running;
}
pub fn poll(&mut self) -> bool {
    let Some(rx) = self.rx.as_ref() else { return false; };
    match rx.try_recv() { Ok(..) => .., Err(TryRecvError::Empty) => false, Err(TryRecvError::Disconnected) => .. }
}
```
Differences for Phase 41: (1) the work closure receives a `Sender<UpdateMsg>` so it can report `Downloading { version }` before `Done(CheckResult)`; `poll` loops `try_recv` until `Empty`, applying each message in order; (2) `Disconnected` before `Done` maps to the generic failure (`"Could not read the latest version from GitHub."`) so a panicking worker never leaves the button on Checking (`EconomySync` already does the same with "the sync worker stopped unexpectedly"; `LeagueList` uses `WORKER_STOPPED`); (3) `start` is a no-op while in flight AND while `Ready`; (4) publish the work folder early through `Arc<Mutex<Option<PathBuf>>>` so `abandon()` can delete a partial download at quit.

**Clear-on-conceal analog:** `EconomySync::clear_message` drops a `Finished` state but leaves `Running` alone. For the update job the equivalent is: the status MESSAGE is cleared on conceal/edit (in `app.rs`), the button state (`Ready`) is NOT.

**Test pattern to copy** (`economy.rs` `mod tests`): `settle()` with a 5 s wall-clock deadline, 1 ms sleep, panic on timeout; tests inject closures that return immediately; no network. Add pure table tests for `caption()`/`enabled()`/`tooltip()` per state (copy strings from UI-SPEC "Update button: states and copy": `Check for updates`, `Checking…`, `Downloading…`, `Install <version> and quit`).

**Framework-free intent state machine** (unit-testable without egui; Pitfall 3): click -> `QuitNow` keeps the flag; click -> `OpenConfirm` -> Cancel clears it; click -> Apply refused clears it; plain tray Quit with no intent never fills the slot.

### `src/overlay/about_view.rs` (component, request-response)

**Analog:** same file `about_tab(ui, skin)`; return-an-action shape from `shell::economy_tab` (`EconomyClicks`, per 39-PATTERNS).

**Current insertion point** (after the version label, before the hyperlink; the 4pt gap before the hyperlink becomes 8pt on both sides of the button per UI-SPEC):
```rust
col.add_space(8.0);
let version = col.label(text(&version_text(), 14.0, pal::MUTED));
if env!("FF_VERSION").ends_with("-dev") && !GIT_DESCRIBE.is_empty() {
    version.on_hover_text(format!("Unreleased build ({GIT_DESCRIBE})"));
}
if !crate::skin::REPO_URL.is_empty() {
    col.add_space(4.0);
    col.hyperlink_to(text(crate::skin::REPO_LINK_TEXT, 15.0, pal::GOLD), crate::skin::REPO_URL)
        .on_hover_text("Open the repository in your browser.");
}
```
Add `col.add_space(8.0)`, then the button, then `col.add_space(8.0)` (the repo link block keeps its `REPO_URL.is_empty()` guard; put the 8pt gap so the button layout does not depend on the guard).

**Button call** (signature at `skin::widgets::ff_button_ex`):
```rust
pub fn ff_button_ex(ui: &mut Ui, skin: &Skin, text: &str, min_width: f32, enabled: bool, danger: bool) -> Response
```
Use `w::ff_button_ex(&mut col, skin, view.caption(), 240.0, view.enabled(), false)`; disabled tooltip via `on_disabled_hover_text`, enabled via `on_hover_text`. Greyed-with-tooltip precedent: `shell::economy_tab` Sync button (`ff_button_ex(.., enabled, ..)` then `r.on_hover_text(tip)` when disabled).

**Signature change:** `about_tab(ui, skin, &UpdateView) -> Option<AboutAction>` with `AboutAction::{Check, InstallAndQuit}`. `UpdateView` is a plain struct (caption/enabled/tooltip) built in `app.rs` from `update_job`, so the view stays stateless. Two call sites to change: `ShellTab::About => super::about_view::about_tab(&mut cui, &skin)` in `OverlayApp::ui`, and the `render` helper in `about_view` `mod tests`:
```rust
ctx.run_ui(input, |ui| {
    let area = Rect::from_min_size(pos2(0.0, 0.0), vec2(w, h));
    let mut child = ui.new_child(UiBuilder::new().max_rect(area));
    about_tab(&mut child, &skin);
})
```
Keep `about_renders_in_a_full_page` (535x760) and `about_renders_in_a_cramped_page` (535x300); extend `render` to iterate the four states. Add a copy-matches-spec style test for the four captions (`copy_matches_the_spec` pattern).

### `src/overlay/app.rs` (controller, event-driven)

**Poll next to the economy poll** (top of `ui()`):
```rust
if self.economy.poll() {
    self.prices_changed();
    ui.ctx().request_repaint();
}
```
Add `if self.update.poll() { ...set update_message...; ui.ctx().request_repaint(); }` right after it. The repaint matters: the window is drawn over a game that may generate no input events (same comment as the economy poll).

**Install click and quit sequence.** Current code (tray arm; this body becomes `fn request_quit(&mut self) -> bool` in `.planning/phases/40-release-pipeline-installer/40-01-PLAN.md` Task 2, called from both the tray arm and the `-quit` poll; the plan says `fn request_quit(&mut self) -> bool` unchanged in behaviour):
```rust
Some(OverlayCommand::Quit) => {
    let dirty = self.quit_dirty();
    let step = apply::quit_step(dirty, self.quit_confirm, apply::QuitEvent::TrayQuit);
    if step == apply::QuitStep::QuitNow {
        quit_requested = true;
        self.begin_shutdown();
    } else if apply::quit_step_shows_confirm(step) {
        self.quit_confirm = true;
        // Reveal the window the way tray Show does (also when the window was hidden while the confirm was already up).
        if self.visibility == Visibility::Hidden { self.toggle_requested = true; }
    }
}
```
Install click: verify `std::fs::metadata(&ready.installer).is_ok_and(|m| m.is_file())`; if gone, set the Error message and reset to Idle; else `self.install_on_quit = true; if self.request_quit() { quit_requested = true; }`. If executing before 40-01 lands, reuse the arm body above instead. Never call `spawn` here.

**Intent clearing** (`dispatch_popup`, `Popup::Quit`): clear on `CloseConfirm`, keep on `QuitNow`/`RunApply` success:
```rust
match apply::quit_step(dirty, true, apply::QuitEvent::Choice(quit_choice)) {
    apply::QuitStep::CloseConfirm => self.quit_confirm = false,   // + self.install_on_quit = false
    apply::QuitStep::QuitNow => { self.quit_confirm = false; self.quit_next_frame = true; ctx.request_repaint(); }
    apply::QuitStep::RunApply => {
        self.quit_confirm = false;
        let ok = self.run_apply(ctx, false);
        if apply::quit_step(dirty, false, apply::QuitEvent::ApplyFinished { ok }) == apply::QuitStep::QuitWhenIdle {
            self.quit_after_apply = true;
        }
        // refused Apply: the window stays open, so also clear install_on_quit here
    }
```
`apply::quit_step` is pure (`QuitEvent::ApplyFinished { ok: false } => QuitStep::CloseConfirm`); reuse unchanged, no new `QuitEvent`.

**Shutdown hook** (`begin_shutdown`; all three quit routes `QuitNow`, `quit_after_apply`, `quit_next_frame` already funnel through it):
```rust
fn begin_shutdown(&mut self) {
    if let Some(manager) = self.hotkey_manager.take() { hotkey::release_toggle(manager); }
    self.tray.release();
    // Release the instance name now ...
    self.instance = None;
}
```
Append after `self.instance = None;`: if `install_on_quit` and state is `Ready`, put the installer path into the shared `Arc<Mutex<Option<PathBuf>>>` slot; otherwise `self.update.abandon()` (delete the work folder: partial download or unused verified installer). Do NOT spawn here (process still alive, event loop running).

**Event message slot.** Existing clearers:
```rust
fn clear_events_on_conceal(&mut self) { ...; self.config_message = None; self.list_status = None; self.economy.clear_message(); }
fn clear_events_on_edit(&mut self) { self.apply_message = None; self.list_status = None; self.config_message = None; }
```
Add `update_message: Option<(String, w::StatusKind)>` and clear it in BOTH (the button state is not cleared). Also clear it wherever `self.apply_message` is set (`apply_message = applied_message(..)` sites) because `event_from` is precedence-based, not recency-based. Call-site in the frame body:
```rust
let event = shell::event_from(self.apply_message.clone(), self.config_message.clone(), sync_outcome, self.list_status.clone());
```
Add the new argument, and update the other `event_from(` sites: the `crate::overlay::shell::event_from(app.apply_message.clone(), None, None, None)` test and the `StatusInputs { event: shell::event_from(..) }` test. Debug `--status` override immediately below already replaces the event (`Some(text) => Some((text, w::StatusKind::Warning))`), so no change needed there.

**Constructor arity.** `OverlayApp::new` currently takes `(tray, hotkey_manager, hotkey_status, instance_guard, skin)` and is extended by earlier phases; add one trailing `pending_install: Arc<Mutex<Option<PathBuf>>>` parameter and say "add one parameter" in the plan rather than quoting a signature. `new_screenshot` builds through `Self::build(TrayHandles::inert(), None, HotkeyStatus::Failed("screenshot mode"..), None, skin, root, game_dir)`; give `build` a throwaway slot there.

**Screenshot injection** (analog in `new_screenshot`, same style as `stage_economy`):
```rust
match shot.stage_economy.as_deref() {
    Some("add") => app.stage_add_economy(),
    Some("remove") => app.stage_remove_economy(),
    _ => {}
}
```
Add a match on `shot.update_state` that injects `UpdateState` via an `inject()` (mirrors `LeagueList::inject` from `39-03-PLAN.md`); `ready:<ver>:<date>` also sets `update_message` to `<ver> available, released <date>` with Info. The worker is never started in screenshot mode.

### `src/overlay/shell.rs` (utility, transform)

**Analog:** same file `event_from` (shown above in app.rs section). Add one more parameter and a first-position branch (the update message is a recent explicit user action; UI-SPEC says event messages show until the next edit):
```rust
pub(crate) fn event_from(
    apply_message: Option<(String, StatusKind)>,
    config_message: Option<(String, bool)>,
    sync: Option<(String, bool)>,
    list_status: Option<String>,
) -> Option<(String, StatusKind)> {
    if let Some(message) = apply_message { return Some(message); }
    ...
```
New param `update: Option<(String, StatusKind)>` carries its own kind, like `apply_message` (it needs Info/Ok/Warning/Error, which the `bool` flavours cannot express). Add a precedence test next to the existing `event_from` tests. Note `39-PATTERNS.md` and Phase 37/38 may also edit `StatusInputs`/`event_from` call sites; re-read at execution.

### `src/overlay/mod.rs` (controller)

**Analog:** same file `launch()`. The result of `eframe::run_native(...)` is currently returned directly (tail expression):
```rust
eframe::run_native(
    "ferrisfilter",
    options,
    Box::new(move |cc| {
        let skin = crate::skin::install(&cc.egui_ctx);
        Ok(Box::new(app::OverlayApp::new(tray, hotkey_manager, hotkey_status, instance_guard, skin)))
    }),
)
```
Change to: create `let pending = Arc::new(Mutex::new(None::<PathBuf>));` before, move a clone into the closure and into `OverlayApp::new`, bind `let result = eframe::run_native(...)`, then:
```rust
if let Some(path) = pending.lock().ok().and_then(|mut p| p.take()) {
    if let Err(e) = crate::update::spawn_installer(&path) {
        instance::notify_error(&format!("Could not start the FerrisFilter installer: {e}. It was saved to {}.", path.display()));
    }
}
result
```
The spawn text is verbatim from UI-SPEC. Register `pub(crate) mod update_job;` next to `pub mod economy;` (and `leagues` from 39-03 sits there too). `run_screenshot` is untouched.

### `src/overlay/instance.rs` (utility, Win32 FFI)

**Analog:** same file `notify` (Windows implementation; a no-op non-Windows twin follows at `pub fn notify(_message: &str) {}`):
```rust
pub fn notify(message: &str) {
    let text = wide(message);
    let title = wide("FerrisFilter");
    // SAFETY: both buffers are NUL-terminated and alive across the call; a null owner window is allowed.
    unsafe {
        MessageBoxW(std::ptr::null_mut(), text.as_ptr(), title.as_ptr(),
            MB_OK | MB_ICONINFORMATION | MB_SETFOREGROUND | MB_TOPMOST);
    }
}
```
Add `notify_error` identical but with `MB_ICONERROR` (extend the `windows_sys::...::{MB_ICONINFORMATION, MB_OK, MB_SETFOREGROUND, MB_TOPMOST, MessageBoxW}` import) and a no-op non-Windows twin. Keep the `// SAFETY:` comment (project convention: all overlay `unsafe` is confined to `win.rs` / `instance.rs`). If an optional `ShellExecuteW` 740 fallback is included, it belongs in `win.rs` and needs `"Win32_UI_Shell"` added to the `windows-sys` features in `Cargo.toml`.

### `src/overlay/screenshot.rs` (utility, transform)

**Analog:** `--tier-text` and `--stage-economy` (enum-valued flag with a validated set, plus `--dropdown` with its `DROPDOWNS` const and `contains` check). Pieces to copy:
```rust
// struct field
pub tier_text: Option<(String, String)>,
// local in parse()
let mut tier_text = None;
// arm
"--status" => status = Some(value("--status")?),
```
Add `pub update_state: Option<String>` to `ShotArgs`, a `let mut update_state = None;`, an arm `"--update-state"` that accepts `idle|checking|downloading|ready:<ver>:<date>` and returns `Err(format!("unknown update state `{s}` (idle|checking|downloading|ready:<ver>:<date>)"))` otherwise, and add it to the `ShotArgs { .. }` construction and the module doc usage line. Unknown flags are errors today, so the flag MUST be added here. Tests: copy `tier_text_is_read` and the `p(&[...])` helper. Update CLAUDE.md's screenshot flag list accordingly.

### `Cargo.toml` / `Cargo.lock`

**Analog:** current `[dependencies]` entries:
```toml
reqwest = { version = "0.13", default-features = false, features = ["rustls-no-provider", "json", "query"] }
tokio   = { version = "1", features = ["rt-multi-thread", "macros"] }
tempfile = "3"
```
Add `semver = "1"` and `ring = "0.17"` (both already in the lock per 41-RESEARCH; no new package). No new reqwest feature: `Response::chunk()` works without `stream`. `tempfile` is already a normal dependency (not only dev) so `update.rs` can use it; use `TempDir::keep()` (`into_path` is deprecated). One non-`--locked` build to record the edges; commit the lock (Phase 40 CI uses `--locked`). Planner: add a `checkpoint:human-verify` on this edit (slopcheck was unavailable).

## Shared Patterns

### Framework-free model plus table tests
**Source:** `src/overlay/economy.rs` module doc and tests (`settle`, `a_started_sync_is_running_until_it_reports`). **Apply to:** `update_job.rs`, `update.rs` pure fns. No egui types in signatures; `app.rs` owns the button and the status line.

### Event messages vs standing state
**Source:** `app.rs` `clear_events_on_conceal` / `clear_events_on_edit`, `shell::event_from`. **Apply to:** `update_message`. Messages clear on edit and conceal; the `Ready` button state survives both (test "conceal keeps Ready").

### Test-safe I/O seam
**Source:** `app.rs` `resolve_root` with its `#[cfg(test)]` twin (per 39-PATTERNS), `sync/metadata.rs` `FakeClient`. **Apply to:** the update fetch. App-level tests inject `UpdateState` and never call `start` with a real fetcher; add a `#[cfg(test)]` twin for whatever production fn builds `ReqwestUpdateHttp`, returning an error like `"unit tests never reach GitHub"`. Tests never write the real `%TEMP%` (pass a test-owned temp root).

### Quit path reuse
**Source:** `apply::quit_step` (`QuitEvent::TrayQuit`, `Choice(..)`, `ApplyFinished { ok }`), `begin_shutdown`, `request_quit` (40-01). **Apply to:** the Install click. No new popup, no new `QuitEvent`.

### Greying a control with a tooltip
**Source:** `shell::economy_tab` Sync button; `ff_button_ex` forces disabled drawing when `!ui.is_enabled()`. **Apply to:** the update button in Checking/Downloading.

### `unsafe` confinement
**Source:** `instance.rs` `notify` `// SAFETY:` comment. **Apply to:** `notify_error` and any `ShellExecuteW` fallback.

## No Analog Found

| File / piece | Role | Data Flow | Reason |
|--------------|------|-----------|--------|
| Streamed SHA-256 download into a kept `TempDir` (`run_check` download step) | service | streaming file-I/O | No streaming or hashing code exists; use 41-RESEARCH Patterns 4 and `parse_sums` example (`ring::digest::Context`, `tempfile::Builder::tempdir_in`, `keep()`) |
| Installer spawn (`update::spawn_installer`) | utility | process spawn | No `Command::spawn` of an external exe exists in the overlay; use 41-RESEARCH Pattern 7 |
| Semver compare, `published_at` parse, `days_from_civil` | utility | transform | New logic (only `days_to_ymd` exists, in `sync::prices`); 41-RESEARCH Patterns 3 and 8 |
| Multi-message worker (`Downloading` then `Done`) | model | event-driven | Existing workers send exactly one result; extension specified under `update_job.rs` above |

## Cross-phase hot spots (re-read at execution)

- `app.rs` `OverlayCommand::Quit` arm becomes `request_quit` in 40-01; `OverlayApp::new` arity grows in 37, 39 (`leagues`) and 41 (pending slot).
- `shell.rs` `event_from` / `StatusInputs` literals are also touched by 39 (`ended_league`); every struct-literal and call site (including tests) must gain the new field.
- `screenshot.rs` `ShotArgs` literal is also extended by 37 and 39 (`--leagues`, `--league`, `--price-source`).
- `src/sync/client.rs` `USER_AGENT` appears only after 40-03; the current inline UA literal in `ReqwestClient::new` has the old repo name.

## Metadata

**Analog search scope:** `src/overlay/{about_view,app,apply,economy,instance,mod,screenshot,shell}.rs`, `src/sync/{client,mod,metadata}.rs`, `src/skin/widgets.rs`, `Cargo.toml`; plans `39-03`, `40-01`, `40-03`; `39-PATTERNS.md`
**Files scanned:** about 16 read or grepped
**Pattern extraction date:** 2026-10-18
