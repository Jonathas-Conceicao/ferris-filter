# Phase 41: Check for Updates - Research

**Researched:** 2026-10-18
**Domain:** Rust / egui desktop overlay: GitHub Releases API client on a worker thread, streamed SHA-256 download into %TEMP%, hand-off to a per-user Inno Setup installer after the process released its single-instance name
**Confidence:** HIGH (API contract, codebase seams), MEDIUM (Windows spawn / delete-while-open details, to be confirmed by the deferred manual check)

<user_constraints>
## User Constraints (from 41-CONTEXT.md)

### Locked Decisions
**Checking**
- A Check for updates button on the About tab, under the version. A click calls GitHub's
  `GET /repos/Jonathas-Conceicao/ferris-filter/releases/latest` (with a User-Agent) on a worker
  thread; that endpoint never returns drafts or pre-releases, so `-rc` builds are invisible.
- The release tag is compared with the exe's `Cargo.toml` version (semver); a dev build compares on
  its base version.
- Up to date: the status line reads "Up to date (<version>, released <date>)". A repo with no
  releases yet (404) also counts as up to date (no date then).
- Only on click: never at start-up, never in the background.

**Newer version found**
- The installer `FerrisFilter-<ver>-setup.exe` and `SHA256SUMS.txt` are downloaded right away into a
  fresh folder under `%TEMP%`, and the installer's SHA-256 is checked against the sums file.
- The status line names the version and its release date ("<version> available, released <date>");
  the button becomes "Install <version> and quit".
- Install and quit goes through the normal Quit path (Unapplied changes popup: Apply & quit / Quit
  without applying / Cancel). After the overlay has released its hotkey, tray icon and instance
  name, it starts the installer (normal visible wizard) and exits.
- Portable-zip users are offered the installer too; there is no in-place exe swap.

**Failures (UPD-03)**
- Offline, GitHub rate limit, a release without the installer asset, a checksum mismatch or a failed
  download: one status-line message each, any partial file deleted, nothing else changed; the button
  returns to Check for updates.

### Claude's Discretion
- Date format (follow the Economy tab's Last sync style), worker reuse (Phase 39's generic worker),
  module layout (`update.rs`), how tests fake the GitHub responses.

### Deferred Ideas (OUT OF SCOPE)
- Automatic or start-up update checks (out of scope for v0.7).

**Also binding:** `41-UI-SPEC.md` (approved 2026-10-18) fixes all copy, states, kinds and the
install-and-quit sequence; the plan must reproduce its message table verbatim.
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| UPD-01 | Check for updates on the About tab reports "up to date" or the newer version | `update::run_check` + `classify_latest`, semver base-version compare, `published_at` -> local `YYYY-MM-DD`, status events (Standard Stack, Patterns 1-3) |
| UPD-02 | For a newer version it downloads the installer, verifies SHA-256, offers Install and quit through the Unapplied changes popup | Streamed ring SHA-256 download into a kept tempdir, `parse_sums`, `request_quit` reuse, `install_on_quit` intent, spawn after `run_native` returns (Patterns 4-6) |
| UPD-03 | Offline / rate-limited / missing-asset failures show a status message and change nothing | `Failure` enum -> message table with kinds, RAII tempdir deletion, no state change on failure (Pattern 3, Pitfalls) |
</phase_requirements>

## Summary

Phase 41 is almost entirely new, framework-free Rust plus a thin wiring layer. The network part is one
GET of the latest-release JSON, one small GET of `SHA256SUMS.txt`, and one streamed GET of the
installer. None of it needs a new reqwest feature: `Response::chunk()` streams without the `stream`
feature, redirects are on by default, and `https_only`, `connect_timeout`, `read_timeout` exist in
0.13 [VERIFIED: reqwest-0.13.3 source in the cargo registry]. The existing `HttpClient` trait
(`sync/client.rs`) returns only `String` after `error_for_status`, so it cannot express 404 vs 403 vs
429 or headers; Phase 41 needs its own small synchronous trait (`UpdateHttp`) with a reqwest
implementation that owns a tokio runtime, and a fake for tests.

Two corrections to the brief: (1) Phase 39 did **not** extract a generic worker. 39-RESEARCH/39-03
deliberately kept `EconomySync` as is and added `overlay/leagues.rs::LeagueList` as a second copy of
the thread + `mpsc` + `try_recv` pattern. Phase 41 needs a third copy with one difference: the job
reports a mid-flight stage (Checking -> Downloading), so its channel carries messages, not one result.
Do not refactor the two existing workers. (2) `semver` is in `Cargo.lock` (1.0.28) but not in the host
dependency graph (`cargo tree -i semver` prints nothing), and `sha2` is **not** in the lock at all.
`ring` 0.17.14 is already compiled (rustls provider) and gives streaming SHA-256, so the recommended
dependency delta is `semver = "1"` and `ring = "0.17"` as direct dependencies: no new package enters
`Cargo.lock`, only the `ferrisfilter` dependency list changes. The lock file must be committed (Phase
40 CI builds with `--locked`).

The risky seam is shutdown. The installer must start only after `begin_shutdown` released the hotkey,
tray and instance name, and only when the quit was started by the Install button (a later plain tray
Quit must never launch it). Recommended: an `install_on_quit` intent flag on the app, cleared on
Cancel / refused Apply, copied into a shared `Arc<Mutex<Option<PathBuf>>>` at `begin_shutdown`, and the
spawn performed in `launch()` right after `eframe::run_native` returns (eframe 0.36 `run_and_return`
defaults to true, so it does return). That avoids the `App::on_exit` signature that changes with the
`glow` feature.

**Primary recommendation:** Add `src/update.rs` (pure + `UpdateHttp` trait + `run_check`, all tested with
a fake and a test-owned temp root), `src/overlay/update_job.rs` (worker, button state, install intent;
no egui), change `about_tab` to take a view struct and return an action, set the status event through a
new `update_message` slot, and spawn the installer from `launch()` after `run_native` returns.

## Project Constraints (from CLAUDE.md)

- Rust CLI + `ff` GUI overlay; overlay `unsafe` is confined to `win.rs` / `instance.rs`, every `unsafe`
  block carries a `// SAFETY:` comment (13-RESEARCH convention, restated in 40-01). A new
  `MessageBoxW` error variant belongs in `instance.rs`; a `ShellExecuteW` fallback belongs in `win.rs`.
- Tests must not touch the real `%APPDATA%\FerrisFilter`, the real game folder or the network;
  screenshot mode only runs from a temp home (`guard_env`). The check/download is never started in
  screenshot mode.
- `README.md` and `CHANGELOG.md` must NOT be edited by this phase's plans (task brief; they are
  hand-written, memory note). `CLAUDE.md` is the domain reference and should gain the `--update-state`
  screenshot flag, the About-tab update behaviour and the new modules.
- Overlay UI "not texty": no standing explanatory text on the About page; explanations are tooltips.
- Git: milestone branching; do not commit from the researcher.
- `nyquist_validation: false` in `.planning/config.json`, so no Validation Architecture section;
  test expectations are still listed under Patterns for the planner.

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| Release lookup, asset download, SHA-256 | Worker thread (`update.rs`, blocking facade over reqwest) | - | Never on the UI thread (a 30 s timeout would freeze the window) |
| Version / date / sums / response classification | Pure functions in `update.rs` | - | Table-testable with no network or egui |
| Button state, check result, install intent | Overlay model (`overlay/update_job.rs`, framework-free) | `app.rs` fields | Same split as `economy.rs` / `leagues.rs` |
| Button drawing, action return | `about_view.rs` (`about_tab`) | `skin::widgets::ff_button_ex` | About tab stays a pure view |
| Status messages | `app.rs` event path (`event_from` / `status_for`) | `shell.rs` | Existing one-line status bar |
| Quit with unapplied-changes safety | `apply::quit_step` + `OverlayApp::request_quit` | popup `Popup::Quit` | Reused unchanged |
| Releasing hotkey / tray / instance name | `OverlayApp::begin_shutdown` | `instance.rs` | Existing order |
| Starting the installer | `launch()` after `run_native` returns | `update::spawn_installer`, `instance::notify` (error box) | Strictly after every release |
| Temp storage | `%TEMP%\FerrisFilter-update-*` (OS) | - | Fresh dir per check, kept only when verified |

## Standard Stack

### Core
| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| reqwest | 0.13 (lock 0.13.3, registry also has 0.13.5) | GET JSON, stream the installer | Already a dependency; `rustls-no-provider` + `json` + `query` features are enough. `Response::chunk()` needs no extra feature [VERIFIED: reqwest-0.13.3/src/async_impl/response.rs] |
| tokio | 1 (rt-multi-thread) | `Runtime::new().block_on` inside the worker thread | Same scaffold as `sync_economy_blocking` |
| serde_json | 1.0 | Parse the release JSON | Already a dependency; parse into a private `#[derive(Deserialize)]` struct (no `deny_unknown_fields`) |
| tempfile | 3 (lock 3.27) | `Builder::new().prefix("FerrisFilter-update-").tempdir()`; `TempDir::keep()` on success | Already a (normal) dependency. Use `keep()`; `into_path()` is deprecated in 3.27 [VERIFIED: tempfile-3.27.0/src/dir/mod.rs] |
| semver | 1 (lock 1.0.28) | Parse tag and `CARGO_PKG_VERSION`, compare | Cargo's own flavour; already in `Cargo.lock` but not yet a direct dependency [VERIFIED: Cargo.lock, `cargo info semver`] |
| ring | 0.17 (lock 0.17.14) | `ring::digest::{Context, SHA256}` streaming hash | Already compiled via rustls (`cargo tree -i ring`); avoids adding `sha2` (not in the lock) [VERIFIED: ring-0.17.14/src/digest.rs has `Context::new/update/finish`, `SHA256`] |

### Supporting
| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| windows-sys | 0.61 (existing) | Optional `ShellExecuteW` fallback needs feature `Win32_UI_Shell`; `MessageBoxW` already enabled | Only for the ERROR_ELEVATION_REQUIRED (740) fallback |
| `sync::prices::days_to_ymd` | in-repo | Day number -> (y, m, d) for the local date | Reuse; add only the inverse (`days_from_civil`) for `published_at` |
| `win::utc_offset_minutes` | in-repo | Local offset for the date | Same helper the Economy tab uses (`last_sync_text` takes it as a parameter) |

### Alternatives Considered
| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| `ring` for SHA-256 | `sha2 = "0.10"` (RustCrypto) | More idiomatic API but a new package in the lock, and slopcheck was unavailable this session (no `pip`), so it would be `[ASSUMED]` legitimacy. Use only if the author dislikes depending on `ring` directly |
| Own `UpdateHttp` trait | Reuse `HttpClient::get_json` | `get_json` calls `error_for_status` and returns `String`: loses 404 vs 403 vs 429, headers and binary bodies. Cannot satisfy UPD-03 |
| `reqwest` `blocking` feature | Async facade + own runtime | `blocking` adds a feature and a second runtime story; the repo already owns the runtime-in-worker pattern |
| `App::on_exit` for the spawn | Spawn after `run_native` | `on_exit`'s signature depends on eframe's `glow` feature; after-`run_native` is signature-independent |
| Hand-written semver | `semver` crate | Pre-release / build-metadata edge cases; crate is already locked |

**Installation (Cargo.toml `[dependencies]`):**
```toml
semver = "1"
ring   = "0.17"
```
Then run one non-`--locked` build so `Cargo.lock` records the two new direct edges, and commit the lock.
If the `ShellExecuteW` fallback is included, add `"Win32_UI_Shell"` to the `windows-sys` feature list.

**Version verification:** `cargo search semver` -> 1.0.28; `cargo search ring` -> 0.17.14 (both match
`Cargo.lock`). [VERIFIED: cargo CLI, 2026-10-18]

## Package Legitimacy Audit

slopcheck could not be installed (`pip` not available in this shell), so no `[OK]/[SUS]/[SLOP]` verdict
exists. Both packages are already resolved in `Cargo.lock` and compiled into the shipped binary
(`ring` through rustls; `semver` through the lock), which is stronger evidence than a registry lookup,
but the formal gate is not satisfied.

| Package | Registry | Age | Downloads | Source Repo | slopcheck | Disposition |
|---------|----------|-----|-----------|-------------|-----------|-------------|
| semver | crates.io | years (dtolnay) | very high | github.com/dtolnay/semver | unavailable | [ASSUMED] legitimacy; already in `Cargo.lock` 1.0.28. Planner: one `checkpoint:human-verify` on the Cargo.toml edit is cheap |
| ring | crates.io | years | very high | github.com/briansmith/ring | unavailable | [ASSUMED] legitimacy; already compiled via rustls, `cargo tree -i ring` shows rustls and reqwest paths |

**Packages removed due to slopcheck [SLOP] verdict:** none
**Packages flagged as suspicious [SUS]:** none (not run)
No `postinstall` concern: Rust crates; `ring` has a `build.rs` already built in every current build.

## Architecture Patterns

### System Architecture Diagram

```
 About tab click                                   (UI thread, per frame)
 [Check for updates] --Action::Check--> OverlayApp --UpdateJob::start(work)--+
                                          ^  poll() each frame (try_recv)    |
                                          |                                  v
   status line <-- update_message <-- UpdateMsg::{Downloading, Done}   WORKER THREAD
   button caption <-- UpdateState                                      Runtime::block_on
                                                                          |
        GET api.github.com/.../releases/latest  --(UpdateHttp)-->  classify_latest
          |-- transport err ............................. Failure::Offline
          |-- 404 ....................................... UpToDate(no date)
          |-- 403/429 + ratelimit hdrs .................. Failure::RateLimited
          |-- other non-2xx / bad JSON / bad tag ........ Failure::BadResponse
          `-- 200 -> tag > current base ?
                 no  -> UpToDate(current, released = date only if tag == current)
                 yes -> find assets FerrisFilter-<v>-setup.exe + SHA256SUMS.txt
                          missing -> NoInstaller / NoSums
                        send UpdateMsg::Downloading{version}
                        GET SHA256SUMS.txt (<= 64 KiB) -> parse_sums -> BadSums
                        TempDir::new(%TEMP%\FerrisFilter-update-*)   (RAII: any failure deletes it)
                        GET browser_download_url (redirect -> CDN) stream -> file + ring SHA-256
                          error / over cap -> DownloadFailed ; digest != expected -> Mismatch
                        TempDir::keep() -> Ready{version, published, installer path}

 [Install <v> and quit] -> exists? no -> Error "no longer there", state Idle
                        -> install_on_quit = true ; request_quit()   (quit_step TrayQuit)
                              QuitNow ------------------------------+
                              OpenConfirm -> Unapplied changes popup |
                                 Apply&quit -> RunApply -> QuitWhenIdle -> begin_shutdown
                                 Quit w/o applying -> QuitNow  --------+
                                 Cancel / Apply refused -> intent cleared, stays Ready
                                                                     v
 begin_shutdown: release hotkey, tray, instance name; if intent: PendingInstall slot = path
 quit_requested -> ViewportCommand::Close -> eframe::run_native RETURNS
 launch(): take slot -> spawn_installer(path) (CreateProcess; 740 -> ShellExecuteW) ; Err -> MessageBoxW
 process exits ; installer wizard runs; its `ff --quit` finds no instance (Phase 40)
```

### Recommended Project Structure
```
src/
├── update.rs                 # NEW, lib-level, no egui: consts, UpdateHttp + ReqwestUpdateHttp,
│                             #   Reply/FetchError, parse/compare/classify/format fns, run_check,
│                             #   Failure + message_for, spawn_installer
├── lib.rs                    # pub mod update;
├── sync/client.rs            # USER_AGENT (exists after 40-03); reuse, do not redefine
└── overlay/
    ├── update_job.rs         # NEW, no egui: UpdateState (button), UpdateJob (thread + mpsc),
    │                         #   InstallIntent, caption/enabled/tooltip tables
    ├── about_view.rs         # about_tab(ui, skin, &UpdateView) -> Option<AboutAction>
    ├── app.rs                # fields, poll, click handling, event slot, begin_shutdown hook
    ├── instance.rs           # + notify_error (MB_ICONERROR) next to notify
    ├── screenshot.rs         # + --update-state
    └── mod.rs                # launch(): PendingInstall slot, spawn after run_native
```

### Pattern 1: Blocking `UpdateHttp` seam (fakeable)
**What:** a small synchronous trait the pure logic runs against. The reqwest implementation owns a
runtime; the fake is a `HashMap<String, FakeReply>`.
```rust
// Source: design over reqwest 0.13 (chunk(), https_only, read_timeout verified in registry source)
pub struct Reply { pub status: u16, pub ratelimit_remaining: Option<u64>, pub retry_after: Option<u64>, pub body: Vec<u8> }
pub enum FetchError { Transport(String), TooLarge, Sink(std::io::Error) }
pub trait UpdateHttp {
    /// Small bodies (API JSON, sums): whole body in memory, capped.
    fn get(&self, url: &str, accept: &str, max_bytes: u64) -> Result<Reply, FetchError>;
    /// Large body: bytes go to `sink` as they arrive; returns the HTTP status.
    fn download(&self, url: &str, max_bytes: u64, sink: &mut dyn FnMut(&[u8]) -> std::io::Result<()>)
        -> Result<u16, FetchError>;
}
```
Reqwest implementation facts:
- Build once per check: `let _ = rustls::crypto::ring::default_provider().install_default();` (same line as
  `ReqwestClient::new`), then `reqwest::Client::builder().user_agent(USER_AGENT).https_only(true)
  .connect_timeout(15s).read_timeout(30s).build()`. Do NOT use `.timeout()` on the client: it bounds the
  whole body and would kill a slow but healthy installer download. For the two small GETs add a per-request
  `.timeout(30s)`.
- API request headers: `Accept: application/vnd.github+json` and `X-GitHub-Api-Version: 2022-11-28`
  (docs now list 2026-03-10; the server selected 2022-11-28 by default in the live probe; either is
  accepted). Asset requests: `Accept: application/octet-stream`.
- Stream with `while let Some(chunk) = resp.chunk().await? { total += len; if total > max { return TooLarge } sink(&chunk)? }`.
- Do not use `error_for_status()`; read `resp.status().as_u16()` and the two headers
  (`x-ratelimit-remaining`, `retry-after`) yourself and return them in `Reply`.
- Redirects: default policy follows up to 10; `browser_download_url` redirects to a GitHub CDN host.
  `https_only(true)` makes a downgrade fail as a transport error.

### Pattern 2: Pure response classification (UPD-03 table)
```rust
// Source: GitHub REST docs (rate limits: 403 or 429, x-ratelimit-remaining 0, retry-after)
pub enum LatestResponse { Release(Release), NoReleases, RateLimited, Offline, BadResponse }
pub fn classify_latest(r: Result<Reply, FetchError>) -> LatestResponse {
    let r = match r { Ok(r) => r, Err(_) => return LatestResponse::Offline };
    match r.status {
        200 => serde_json::from_slice::<Release>(&r.body).map_or(LatestResponse::BadResponse, LatestResponse::Release),
        404 => LatestResponse::NoReleases,
        429 => LatestResponse::RateLimited,
        403 if r.ratelimit_remaining == Some(0) || r.retry_after.is_some() => LatestResponse::RateLimited,
        _ => LatestResponse::BadResponse,   // incl. a plain 403, 5xx, 301 that was not followed
    }
}
```
A transport error on the **API** call is Offline (Warning). A transport error or non-2xx on an
**asset** call is `DownloadFailed` (Error), because the release is known to exist.

Release JSON fields used (all present on a published release) [CITED: docs.github.com/en/rest/releases/releases#get-the-latest-release]:
`tag_name`, `draft`, `prerelease`, `published_at` (string or null), `assets[]` with `name`,
`browser_download_url`, `size`, `state` (`uploaded`|`open`), and `digest` (string or null; a
`sha256:<hex>` the API computes; unnecessary because the author locked `SHA256SUMS.txt`).

### Pattern 3: Version rule, outcome and message table
- Current = `Version::parse(env!("CARGO_PKG_VERSION"))` reduced to `Version::new(major, minor, patch)`
  (strips `-dev`, `-rc.N`, build metadata). Do **not** use `FF_VERSION`: it comes from `git describe`
  and can be `0.6` (two parts) or `0.6-dev`, neither valid semver.
- Tag: `tag_name.strip_prefix('v')` then `Version::parse`. Parse failure -> `BadResponse`. A tag with a
  non-empty pre-release, or `prerelease == true` / `draft == true` in the JSON, is treated as "nothing
  newer" (defence in depth; `latest` already excludes them and Phase 40 marks any `-` tag a pre-release).
- `tag > current` -> newer. Equal or older -> `UpToDate`. Date in the message: use the latest release's
  `published_at` only when `tag == current`; for an older tag omit it (UI-SPEC says "same text", but
  "0.7.0, released <date of 0.6.0>" would be false; see Open Question 1).
- Installer asset name is computed, never taken from the API: `format!("FerrisFilter-{ver}-setup.exe")`
  with `ver = tag without v`; sums asset is `SHA256SUMS.txt`. Match `assets[].name` exactly and require
  `state == "uploaded"`. The temp file name is the computed name, so an API-supplied name can never
  become a path (path traversal).
- Failure -> (text, kind) is a single pure function `message_for(&CheckResult, offset_minutes)` carrying
  the UI-SPEC table verbatim (Warning: Offline, RateLimited; Error: everything else; Ok: up to date;
  Info: checking, downloading, available). Table-test every row including the exact strings and kinds.

### Pattern 4: Download, hash, delete on failure
```rust
// Source: ring::digest (Context::new/update/finish), tempfile TempDir RAII + keep()
let dir = tempfile::Builder::new().prefix("FerrisFilter-update-").tempdir_in(tmp_root)?; // tmp_root = temp_dir() in prod, a test TempDir in tests
let path = dir.path().join(&installer_name);
let mut file = std::fs::File::create(&path)?;
let mut ctx = ring::digest::Context::new(&ring::digest::SHA256);
let status = http.download(&url, MAX_INSTALLER_BYTES, &mut |c| { ctx.update(c); file.write_all(c) })?;
file.sync_all()?; drop(file);                       // close before anyone executes it
if !(200..300).contains(&status) { /* DownloadFailed; dir drops -> deleted */ }
let got = hex(ctx.finish().as_ref());
if got != expected { /* Mismatch; dir drops -> deleted */ }
let kept: PathBuf = dir.keep();                      // survives until the spawn; dir.keep() returns the folder path
```
`TempDir` deleting itself on every early return is the "partial file deleted" guarantee for free; unit
tests assert the temp root is empty after each failure (and holds exactly one folder after success).
Caps: `MAX_INSTALLER_BYTES` 200 MiB (an Inno build of this app is a few MiB; the cap only stops
runaway bodies), `MAX_SUMS_BYTES` 64 KiB, `MAX_API_BYTES` 1 MiB. Compare the asset's `size` as a sanity
pre-check (reject `size > cap`) but let the hash be the authority.

`parse_sums(text, file_name) -> Result<String /*lowercase hex*/, SumsError>`: split lines, trim `\r`,
accept `<64 hex>` + one or two spaces (or ` *`) + exact `file_name`; hex case-insensitive, stored
lowercase; no matching line, bad hex length or a duplicate name with a different hash -> `BadSums`.
Phase 40's writer is `printf '%s  %s\n'` (hash, two spaces, plain name, setup first) [CITED: 40-RESEARCH package.sh excerpt].

### Pattern 5: Worker with stages (third copy of the existing pattern)
`EconomySync` / `LeagueList` send one `Result` and `poll()` with `try_recv`. The update job sends
`enum UpdateMsg { Downloading { version: String }, Done(CheckResult) }`; `poll()` loops `try_recv` until
`Empty`, applying each message in order; `Disconnected` before `Done` -> `Failure::BadResponse`
(text "Could not read the latest version from GitHub.") so a panicking worker never leaves the button
stuck in Checking. `start` is a no-op while a job is in flight. Publish the work folder as soon as it
exists (`Arc<Mutex<Option<PathBuf>>>`) so shutdown can delete a partial download (Pitfall 5).

`UpdateState { Idle, Checking, Downloading { version }, Ready(ReadyUpdate) }` with pure
`caption()/enabled()/tooltip()` tables copied from UI-SPEC. `ReadyUpdate { version, published_unix:
Option<i64>, installer: PathBuf }`. `Ready` is sticky; `start` while Ready is refused.

### Pattern 6: Wiring (symbol-anchored; all line numbers will drift)
- `about_tab(ui, skin)` -> `about_tab(ui, skin, &UpdateView) -> Option<AboutAction>`; `AboutAction::{Check,
  InstallAndQuit}`; draw `ff_button_ex(ui, skin, caption, 240.0, enabled, false)` after the version label
  with `add_space(8.0)` on both sides (replaces the 4pt gap before the hyperlink), tooltip via
  `on_hover_text` / `on_disabled_hover_text`. Two call sites change: `ShellTab::About => ...about_tab` in
  `OverlayApp::ui` and the `render` helper in `about_view` tests.
- Poll `self.update.poll()` in `ui()` next to `self.economy.poll()` (top of the frame). On `Downloading`
  set `update_message = ("Downloading <v>...", Info)`; on `Done` set the message from `message_for`.
- Event slot: add `update_message: Option<(String, StatusKind)>`; extend `shell::event_from` with one more
  parameter. Clear it in `clear_events_on_edit` AND `clear_events_on_conceal` (the button state is NOT
  cleared; Ready stays), and clear it wherever `apply_message` is set so the most recent event wins
  (`event_from` is precedence-based, not recency-based). Checked-in precedence recommendation: put the
  update message first in `event_from`; clear it in the Apply path.
- Install click (handled inside `ui()` where `quit_requested` is a local): verify
  `std::fs::metadata(&ready.installer).is_ok_and(|m| m.is_file())`; if gone set the Error message and
  reset to Idle; else `self.install_on_quit = true; if self.request_quit() { quit_requested = true; }`.
  `request_quit()` is created by 40-01 (extracted from the tray `OverlayCommand::Quit` arm); until 40-01
  executes use the tray arm's body.
- Intent clearing: set `install_on_quit = false` on `QuitStep::CloseConfirm` (Cancel and refused Apply,
  both in `dispatch_popup`'s `Popup::Quit` arm). `QuitNow`/`QuitWhenIdle` keep it.
- `begin_shutdown`: after `self.instance = None`, if `install_on_quit` and the state is Ready, put the
  installer path into the shared `PendingInstall` slot; otherwise `self.update.abandon()` (delete the
  work folder: partial download, or an unused verified installer).
- `launch()` (src/overlay/mod.rs): create `let pending = Arc::new(Mutex::new(None))`, pass a clone to
  `OverlayApp::new` (new last parameter; `new_screenshot` passes a throwaway), keep the
  `eframe::run_native(...)` result, then `if let Some(path) = pending.lock().take() { if let Err(e) =
  update::spawn_installer(&path) { instance::notify_error(&format!("Could not start the FerrisFilter
  installer: {e}. It was saved to {}.", path.display())) } }`, then return the saved result.
- Screenshot: `ShotArgs.update_state: Option<String>` parsed in `screenshot::parse` (unknown flags are
  errors today, so the flag must be added there), applied in `new_screenshot` by injecting
  `UpdateState` (and for `ready:<ver>:<date>` the `<ver> available, released <date>` event). The worker
  is never started in screenshot mode. The existing `--status` override already replaces the event in
  debug builds.

### Pattern 7: Spawn
```rust
// Source: std::process::Command; PrivilegesRequired=lowest => no UAC (40-CONTEXT, Inno ishelp)
pub fn spawn_installer(path: &Path) -> Result<(), String> {
    match std::process::Command::new(path).stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null()).spawn() {
        Ok(_child) => Ok(()),                         // dropping Child neither waits nor kills it
        Err(e) if e.raw_os_error() == Some(740) => win::shell_execute_open(path),  // ERROR_ELEVATION_REQUIRED
        Err(e) => Err(e.to_string()),
    }
}
```
No `/SILENT`: the visible wizard is the contract, and its "Launch FerrisFilter" finish entry (ticked,
`skipifsilent` only matters for silent runs) restarts `ff` afterwards, so the Phase 40 handoff note
about silent installs does not bite. Keep the child's cwd default (a cwd inside the temp folder would
block its later deletion). Optionally retry `spawn` up to 3 times at 250 ms when `raw_os_error() ==
Some(32)` (sharing violation while an antivirus scans the fresh exe) [ASSUMED].

### Pattern 8: Date text
`published_at` is RFC 3339 with `Z` (e.g. `2026-11-02T14:03:11Z`) [ASSUMED from API examples; confirm
against the first real release]. Parse only `YYYY-MM-DDTHH:MM:SSZ` by byte position into unix seconds
(add `days_from_civil`), return `None` otherwise, then `(secs + offset*60)` -> `days_to_ymd` -> `YYYY-MM-DD`
(same arithmetic as `last_sync_text`). Worker carries `published_unix: Option<i64>`; format on the UI
thread with `win::utc_offset_minutes()` so the offset is the user's current one and the function stays pure:
```rust
fn days_from_civil(y: i64, m: u32, d: u32) -> i64 {        // Hinnant's algorithm, y/m/d -> days since 1970-01-01
    let y = if m <= 2 { y - 1 } else { y };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let mp = (m as i64 + 9) % 12;
    let doy = (153 * mp + 2) / 5 + d as i64 - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}
```
Tests: `1970-01-01T00:00:00Z` -> 0 -> `1970-01-01`; `2026-11-02T23:30:00Z` with offset +120 -> `2026-11-03`;
offset -180 across midnight; leap day `2028-02-29`; malformed -> `None` (message then omits the date,
exactly as for a 404).

### Anti-Patterns to Avoid
- **Reusing `ReqwestClient`/`get_json`** for GitHub: it hides status and headers.
- **Client-wide `.timeout()`** with a multi-MB download.
- **Spawning the installer inside `begin_shutdown`** (or before): the process is still alive and the
  event loop still running; spawn after `run_native` returns. And never from the Install click itself.
- **Launching on any later Quit**: the intent flag must die on Cancel / refused Apply.
- **Trusting API-provided names/URLs for paths**; take only `browser_download_url` for the request.
- **Adding a debug/env override of the API URL in release code** (a redirect-to-attacker hole); tests use
  the fake trait instead.
- **Comparing against `FF_VERSION`**; it is not semver.
- **Refactoring `EconomySync` / `LeagueList`** into a shared worker in this phase (Phase 39 explicitly
  declined; 15 tests depend on it).

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Version ordering | split on `.` and compare ints | `semver::Version` | pre-release / metadata rules |
| SHA-256 | own implementation | `ring::digest` | already linked, constant behaviour |
| Temp folder naming, cleanup | `temp_dir().join(rand)` + manual `remove_dir_all` on each branch | `tempfile::Builder::tempdir_in` + `keep()` | RAII deletion on every early return and panic |
| Redirect handling | manual `Location` loop | reqwest default policy + `https_only(true)` | CDN redirect is automatic |
| Hex decode/compare | custom parser | `hex(ctx.finish())` string compare (lowercase) | one 64-char compare; no `hex` crate needed |
| Local date | `chrono`/`time` dependency | `days_to_ymd` + `days_from_civil` + `utc_offset_minutes` | matches Economy tab, no new crate |
| Error dialogs after exit | new window code | `MessageBoxW` via `instance.rs` (add an error-icon variant) | the existing mechanism UI-SPEC names |

**Key insight:** the dangerous parts (hash check, cleanup, quit ordering) each have an existing
primitive; the phase is glue plus pure, table-tested decision functions.

## Runtime State Inventory

Not a rename/refactor/migration phase. Omitted. (New runtime artefacts: `%TEMP%\FerrisFilter-update-*`
folders; see Pitfall 5 for cleanup.)

## Common Pitfalls

### Pitfall 1: A plain 403 is not always a rate limit
**What goes wrong:** treating every 403 as "GitHub is limiting requests" mislabels auth/abuse/blocked-IP responses.
**Why:** GitHub uses 403 and 429 for both primary and secondary limits [CITED: docs.github.com rate limits].
**How to avoid:** 429 always; 403 only with `x-ratelimit-remaining: 0` or a `retry-after` header (UI-SPEC rule 9); other 403 is the generic Error. Unauthenticated limit is 60 requests/hour per IP, so a shared NAT can hit it.
**Warning signs:** fake-table test missing a "403 without headers" row.

### Pitfall 2: 404 is also what the live repo returns today
**What goes wrong:** `releases/latest` currently answers 404 (live probe 2026-10-18, authenticated, `{"message":"Not Found",...}`, i.e. no published release yet; a private repo answers 404 to anonymous callers too).
**How to avoid:** 404 -> UpToDate without a date (locked). Test it. It also means the real path cannot be exercised end to end until Phase 40's non-pre-release `v0.7.0` exists, and `-rc` tags are invisible to `latest`.

### Pitfall 3: Spawning too early or on the wrong quit
**What goes wrong:** installer starts while hotkey/tray/instance name still exist (its `ff --quit` then signals a dying instance, or its Restart Manager step sees `ff.exe` in use); or a later ordinary Quit launches an installer the user cancelled.
**How to avoid:** spawn only from `launch()` after `run_native` returns; intent flag cleared on `QuitStep::CloseConfirm`; unit-test the intent state machine (click -> QuitNow keeps; click -> OpenConfirm -> Cancel clears; click -> Apply refused clears; plain TrayQuit with no intent never fills the slot).

### Pitfall 4: Close-of-file before execute / hash
**What goes wrong:** hashing while the handle is open, or executing a file whose last write is not flushed.
**How to avoid:** `sync_all()` then drop the `File` before comparing digest and before the path is ever returned.

### Pitfall 5: Partial download left behind when the process exits
**What goes wrong:** quitting while Downloading kills the worker without running `TempDir::drop`, leaving `FerrisFilter-update-*` in `%TEMP%`; a Ready installer is also orphaned when the user quits normally.
**How to avoid:** publish the folder path to the app (`Arc<Mutex<Option<PathBuf>>>`) and `remove_dir_all` it in `begin_shutdown` when no install is intended. On Windows Rust opens files with `FILE_SHARE_READ|WRITE|DELETE` by default, so deleting a folder whose file the worker still holds open normally succeeds (marked delete-pending) [ASSUMED: std `OpenOptionsExt::share_mode` default; confirm in the Downloading-quit test]. Backstop: at the start of each check, best-effort delete `FerrisFilter-update-*` folders older than 24 h in `temp_dir()` (never the current one).

### Pitfall 6: Message-event bookkeeping
**What goes wrong:** "Checking..." survives forever, or the result is hidden by an older Apply message, or the message vanishes and the button caption is lost.
**How to avoid:** the button state and the status message are separate: message in the event slot (cleared on edit and conceal), state in `UpdateJob` (never cleared by those). Add `update_message` to the conceal/edit clearers and test "conceal keeps Ready".

### Pitfall 7: Provider install and TLS
**What goes wrong:** `reqwest` with `rustls-no-provider` panics without a crypto provider.
**How to avoid:** call `rustls::crypto::ring::default_provider().install_default()` (ignore the error) before building the client, as `ReqwestClient::new` does.

### Pitfall 8: Tests that leak
**What goes wrong:** tests writing under the real `%TEMP%` leave folders, or touch settings.
**How to avoid:** `run_check(http, current, tmp_root, ...)` takes the root; tests pass their own `tempfile::tempdir()` and assert it is empty after failures. No test constructs `ReqwestUpdateHttp`. App-level tests inject `UpdateState` and never call `start` with a real fetcher (mirror how Phase 39 tests avoid `fetch_leagues_blocking`).

### Pitfall 9: Windows spawn quirks
**What goes wrong:** `CreateProcess` returns `ERROR_ELEVATION_REQUIRED` (740) if the installer ever becomes `requireAdministrator`; Defender can briefly lock a fresh exe (sharing violation 32).
**How to avoid:** the 740 -> `ShellExecuteW` "open" fallback and a short retry on 32. With `PrivilegesRequired=lowest` today neither should trigger [CITED: Inno Setup release notes: `lowest` makes Setup not request elevated rights; the embedded manifest level is not independently verified]. Files written by our own process carry no Mark-of-the-Web, so SmartScreen should not interrupt [ASSUMED]; both go in the deferred manual check.

## Code Examples

### Sums parse (pure, tested)
```rust
pub fn parse_sums(text: &str, file_name: &str) -> Result<String, SumsError> {
    let mut found: Option<String> = None;
    for line in text.lines().map(|l| l.trim_end_matches('\r')) {
        let Some((hash, rest)) = line.split_once(' ') else { continue };
        let name = rest.trim_start_matches(' ').trim_start_matches('*');
        if name != file_name { continue; }
        if hash.len() != 64 || !hash.bytes().all(|b| b.is_ascii_hexdigit()) { return Err(SumsError::Invalid); }
        let hash = hash.to_ascii_lowercase();
        match &found { Some(prev) if *prev != hash => return Err(SumsError::Invalid), _ => found = Some(hash) }
    }
    found.ok_or(SumsError::Invalid)
}
```

### Fake HTTP for tests
```rust
struct FakeHttp { replies: std::collections::HashMap<String, Result<(u16, Vec<u8>), String>> /* + headers */ }
impl UpdateHttp for FakeHttp { /* get/download look the url up; download streams the bytes in 3 chunks to the sink */ }
```
Cases: 200 newer with both assets; 200 equal; 200 older; dev base equal; invalid tag; prerelease flag;
404; 403 with remaining 0; 403 plain; 429; 500; transport error on API; missing installer asset; missing
sums asset; sums with no line / bad hex / duplicate; installer bytes mismatch; installer 404/transport
error mid-stream; oversize body; success (tempdir holds exactly the installer, hash matches).

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| `TempDir::into_path()` | `TempDir::keep()` | tempfile 3.2x | `into_path` is `#[deprecated]` in the locked 3.27 |
| GitHub API version header `2022-11-28` | docs list `2026-03-10` | docs 2026 | send `2022-11-28` (server default in probe) or omit; both work |
| Releases assets without checksums | assets carry an API-computed `digest` | GitHub 2025 | extra cross-check available; not required (locked on `SHA256SUMS.txt`) |

**Deprecated/outdated:** `into_path`; `HttpClient::get_json` for anything needing status/headers.

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | `ring` and `semver` are legitimate (slopcheck unavailable; both already in the lock/graph) | Package Audit | Low; planner may add a human-verify on the Cargo.toml edit |
| A2 | Rust std opens files with `FILE_SHARE_DELETE` so a still-open partial can be deleted at quit | Pitfall 5 | Partial folder survives until the 24 h sweep; harmless but untidy |
| A3 | `published_at` always ends in `Z` with no fractional seconds | Pattern 8 | Date omitted from the message (graceful) |
| A4 | A downloaded-by-us exe has no Mark-of-the-Web, so no SmartScreen prompt | Pitfall 9 | Extra prompt when installing; confirm in manual check |
| A5 | Inno `lowest` produces a non-elevating setup so `CreateProcess` works | Pattern 7 | 740 fallback covers it |
| A6 | A sharing violation (32) can occur right after writing the exe | Pattern 7 | Retry is a harmless no-op otherwise |
| A7 | GitHub requires a User-Agent (403 without) | Pattern 1 | `USER_AGENT` is sent anyway |

## Open Questions (RESOLVED)

1. **Up to date with an OLDER latest tag: which date?**
   - Known: UI-SPEC says "same text". Text includes "released <date>".
   - Unclear: the only date available is the older release's.
   - Recommendation: show the date only when `tag == current`; otherwise `Up to date (<version>)`. Flag to the author in the plan; trivial to change. RESOLVED: plans decision 1 (no date when the latest tag is older).
2. **Quit via tray while Ready (no install intent): delete the verified installer?**
   - Recommendation: yes, best effort in `begin_shutdown` (UI-SPEC is silent; leaving ~MBs in %TEMP% per check is untidy). The 24 h sweep is the backstop. RESOLVED: plans decision 2 (deleted at shutdown, 24 h sweep).
3. **Does Phase 40's `request_quit()` exist with that name?** It is created by 40-01 task 2. If the executor finds a different name, reuse the tray Quit arm body; the plan should say "the method 40-01 extracted from the tray Quit arm". RESOLVED: plans name request_quit (40-01) with the tray Quit arm as fallback.
4. **`OverlayApp::new` arity** changes in earlier phases (37 adds game-folder startup); the plan must say "add one parameter" rather than quote a signature. RESOLVED: plans say "add one parameter".
5. **Live end-to-end check** needs a published non-pre-release newer than the running build; defer to a manual UAT item (below), never a blocking task. RESOLVED: 41-MANUAL-VERIFICATION.md (created by 41-05).

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| cargo / rustc (nightly via rust-toolchain.toml) | build, tests | yes | repo toolchain | - |
| gh CLI (authenticated) | read-only API probe, deferred UAT | yes | works | curl |
| api.github.com | probe only (tests use the fake) | yes | `releases/latest` -> 404 now | - |
| slopcheck (`pip`) | package gate | no | - | packages tagged [ASSUMED] |
| Inno Setup / a real release | live e2e | no (Phase 40 not executed here) | - | deferred manual check |

**Live probe (one call):** `GET /repos/Jonathas-Conceicao/ferris-filter/releases/latest` -> `404`
`{"message":"Not Found","documentation_url":"https://docs.github.com/rest/releases/releases#get-the-latest-release","status":"404"}`,
headers `X-Github-Api-Version-Selected: 2022-11-28`, `X-Ratelimit-Limit/Remaining/Reset/Used/Resource`
present (authenticated 5000/h). Anonymous callers get 60/h. `Retry-After` is in `Access-Control-Expose-Headers`.
**Missing with no fallback:** none for planning. **Missing with fallback:** live release -> deferred UAT.

## Deferred Manual Verification (for the plan's final non-blocking item)

1. With a published final release newer than the running exe's `Cargo.toml` version (e.g. run a build whose
   version is lower than the latest tag): Check -> `Downloading x...` -> `x available, released <date>` ->
   button `Install x and quit`.
2. Click it with and without unapplied edits; Cancel keeps Ready; then confirm: overlay exits, the wizard
   appears, completes, "Launch FerrisFilter" reopens `ff`; no SmartScreen/UAC prompt; `%TEMP%\FerrisFilter-update-*` state.
3. Airplane mode -> offline warning; repeated checks past 60/h anonymously -> rate-limit warning (optional).
4. Quit while Downloading leaves no `FerrisFilter-update-*` behind.

## Security Domain

### Applicable ASVS Categories

| ASVS Category | Applies | Standard Control |
|---------------|---------|-----------------|
| V2 Authentication | no | no credentials sent (anonymous API) |
| V3 Session Management | no | - |
| V4 Access Control | no | per-user temp dir default ACL |
| V5 Input Validation | yes | `serde_json` into typed struct; semver-validated tag; computed (not API-supplied) asset/file names; size caps; hex length check |
| V6 Cryptography | yes | `ring::digest::SHA256`; never hand-rolled; integrity only |
| V9/V12 Communications / Files | yes | `https_only(true)`, rustls, redirects only over https; executable run only after hash match |

### Known Threat Patterns

| Pattern | STRIDE | Standard Mitigation |
|---------|--------|---------------------|
| Corrupted / truncated / MITM-altered download | Tampering | SHA-256 against `SHA256SUMS.txt`, delete on mismatch, https only |
| Compromised release (attacker controls both assets) | Tampering / Elevation | Not defeated by a checksum from the same release (documented limitation, matches 40-RESEARCH threat table); the user still sees the unsigned-installer prompts |
| Path traversal via asset name | Tampering | file name computed from validated semver, never from the API |
| Oversized body / zip-bomb style DoS | DoS | per-request byte caps enforced while streaming |
| TOCTOU between verify and spawn | Tampering | per-user temp dir; re-check existence at click; (a same-user attacker is already game over; optional re-hash at click is cheap but not required) |
| Update URL override in release builds | Spoofing | no env/CLI override; tests use the trait fake |

## Sources

### Primary (HIGH confidence)
- docs.github.com/en/rest/releases/releases#get-the-latest-release - latest = non-prerelease non-draft, fields, 200/404
- docs.github.com/en/rest/using-the-rest-api/rate-limits-for-the-rest-api - 403/429, `x-ratelimit-remaining`, `retry-after`, 60/h anonymous
- Local registry sources: reqwest-0.13.3 (`chunk`, `https_only`, `read_timeout`, `connect_timeout`), tempfile-3.27.0 (`keep`, `into_path` deprecated), ring-0.17.14 (`digest::Context`), eframe-0.36.2 (`run_and_return` default true, `on_exit` cfg-dependent signature)
- Repo: `src/overlay/{about_view,app,apply,instance,shell,economy,screenshot,mod}.rs`, `src/sync/client.rs`, `Cargo.toml`, `Cargo.lock`, `cargo tree`
- Phase docs: 40-CONTEXT/RESEARCH/01/03/05 plans (SHA256SUMS format, `PrivilegesRequired=lowest`, `USER_AGENT`, `request_quit`/`take_quit_request`, `skipifsilent`), 39-RESEARCH/PATTERNS/03 (no generic worker; `LeagueList`)
- Live read-only probe of the project's `releases/latest` (above)

### Secondary (MEDIUM confidence)
- Inno Setup release notes via web search: `PrivilegesRequired=lowest` does not request elevation

### Tertiary (LOW confidence)
- Mark-of-the-Web / Defender behaviour, `FILE_SHARE_DELETE` default (training knowledge; flagged A2, A4, A6)

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH - every crate and API verified in registry source or the lock; only slopcheck missing
- Architecture: HIGH - integration points read in code (`begin_shutdown`, `quit_step`, `event_from`, `clear_events_*`, `launch`)
- Pitfalls: MEDIUM - Windows spawn/delete details need the deferred manual check

**Research date:** 2026-10-18
**Valid until:** 2026-11-17 (GitHub API stable; re-check only if Phase 40 changes asset names or the quit path)
