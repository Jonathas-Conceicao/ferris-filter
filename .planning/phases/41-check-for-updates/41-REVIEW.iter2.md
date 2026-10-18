---
phase: 41-check-for-updates
reviewed: 2026-10-18T00:00:00Z
depth: standard
iteration: 2
files_reviewed: 11
files_reviewed_list:
  - Cargo.toml
  - src/lib.rs
  - src/update.rs
  - src/overlay/update_job.rs
  - src/overlay/about_view.rs
  - src/overlay/app.rs
  - src/overlay/instance.rs
  - src/overlay/mod.rs
  - src/overlay/screenshot.rs
  - src/overlay/shell.rs
  - src/overlay/win.rs
findings:
  critical: 0
  warning: 2
  info: 8
  total: 10
status: issues_found
---

# Phase 41: Code Review Report (iteration 2)

**Reviewed:** 2026-10-18
**Depth:** standard
**Files Reviewed:** 11
**Status:** issues_found

## Narrative Findings (AI reviewer)

## Summary

This pass re-reviewed the Phase 41 diff (`ef84189..HEAD`) and focused on the five fix commits
(`9d93376..8e65f93`).

- **CR-01 (install intent kept open): fixed.** Every `Popup::Quit` outcome that leaves the window
  open now calls `cancel_install()`: `CloseConfirm`, the stay-open branch for unwritable tier
  edits, and a refused Apply. `QuitNow` and `QuitWhenIdle` always end in `begin_shutdown`. I found
  no remaining path that leaves the intent armed in a running overlay.
- **WR-02 (hash again before start): fixed in substance.** The digest travels with the hand-off,
  and the hash runs through a handle that refuses writers and deleters. Because the installer is
  `PrivilegesRequired=lowest`, the plain `CreateProcess` path is the one that runs, and the fixer
  checked that path with the lock held. One robustness gap is new (WR-02 below).
- **WR-03 (RC builds): fixed.** `0.7.0-rc.1` against `v0.7.0` is now `Newer`, and the up-to-date
  message names the running pre-release.
- **WR-04 (folder leak and race): fixed.** The folder is created, published and kept under the
  slot lock. Shutdown sets `cancelled` in the same lock, and the download sink stops on it. `start()`
  never clears `cancelled`, which is correct because shutdown is final.
- **WR-01 (late update message): only partly fixed.** A sync that is still running when the
  worker's message arrives is not counted, so its outcome is still hidden (WR-01 below).

No Critical issues remain.

## Warnings

### WR-01: A worker message that arrives while a sync is still running still hides that sync's outcome

**File:** `src/overlay/app.rs:2244-2250` (`receive_update_message`), `src/overlay/app.rs:3513-3516`
**Issue:** `update_behind` is decided once, when the worker's message arrives. It is true only
if an `apply_message`, `config_message` or `list_status` is showing, or if the sync is already
`SyncState::Finished`. A sync that is `Running` at that moment does not count.

Sequence:
1. Press **Check for updates**.
2. Press **Sync economy**. This clears `update_message` (line 4041), and the sync is now `Running`.
3. GitHub answers within a second. `receive_update_message` stores "Up to date (…)" or
   "Downloading 0.8.0…" with `update_behind = false`, because the sync is not `Finished` yet.
4. The sync finishes about 30 s later and becomes `Finished(outcome)`. `event_from` still ranks
   the update message first, so the sync result never shows, including a sync **failure**.

This is the sync half of the original WR-01 scenario. The new tests do not cover it:
`a_late_update_message_waits_behind_a_newer_apply_outcome` only exercises an Apply message that
is already present, and the `shell.rs` rows only pass a `Finished` sync.

**Fix:** Any event that lands after the update message must outrank it. The smallest change
covers both moments:
```rust
fn receive_update_message(&mut self, text: String, kind: crate::update::UpdateKind) {
    self.update_behind = self.apply_message.is_some()
        || self.config_message.is_some()
        || self.list_status.is_some()
        || matches!(
            self.economy.state(),
            economy::SyncState::Running | economy::SyncState::Finished(_)
        );
    self.update_message = Some((text, update_kind(kind)));
}
// and where the sync result is collected:
if self.economy.poll() {
    if self.update_message.is_some() {
        self.update_behind = true; // the sync outcome is newer
    }
    self.prices_changed();
    ui.ctx().request_repaint();
}
```
A per-message sequence counter, where `event_from` picks the newest, would remove the whole
class of bug. Add an app test: start a check, start a sync, deliver the update message, finish
the sync, then assert that the sync outcome is the event.

### WR-02: Every open failure at launch is reported as "installer is no longer there", with no retry for a sharing violation

**File:** `src/update.rs:255-271` (`launch_verified_with`), `src/update.rs:294-301` (`open_read_locked`)
**Issue:** The new lock opens the file with `share_mode(FILE_SHARE_READ)`. That open fails with
`ERROR_SHARING_VIOLATION` (32) whenever another process holds a handle with write or delete
access, which an on-access scanner or a backup or sync agent can do to a fresh `.exe` in `%TEMP%`.
`spawn_installer` (line 920) already expects exactly this and retries error 32 three times.
The new open is now the stricter step, but it does not retry. It also maps every
`std::io::Error` (sharing violation, `ERROR_ACCESS_DENIED`, anything else) to
`InstallerCheck::Gone`. The box then says "The FerrisFilter X installer is no longer there…
Check for updates again." The file is still on disk, the process has already exited, and the
user is sent through a full re-download for what was a transient lock.
**Fix:** Retry the open on raw OS error 32, the same way `spawn_installer` does. Only `NotFound`
should map to `Gone`. Report any other open error with its cause and the saved path:
```rust
let mut file = {
    let mut attempt = 0;
    loop {
        match open_read_locked(path) {
            Ok(f) => break f,
            Err(e) if e.raw_os_error() == Some(32) && attempt < 3 => {
                attempt += 1;
                std::thread::sleep(Duration::from_millis(250));
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                return Err(LaunchError::Check(InstallerCheck::Gone));
            }
            Err(e) => return Err(LaunchError::Spawn(e.to_string())), // or a new Open(String)
        }
    }
};
```

## Info

### IN-01: An oversized or unreadable API reply is reported as "Could not reach GitHub" (carried)

**File:** `src/update.rs:397` (`classify_latest`), `src/update.rs:883-885`
**Issue:** `FetchError::TooLarge` / `Sink`, and a failure to build the HTTP client or the
runtime, all map to `Offline`.
**Fix:** Map these to `BadResponse` or to a dedicated failure.

### IN-02: The trusted-URL check compares against the raw tag (carried)

**File:** `src/update.rs:345-349`
**Issue:** A tag with characters that GitHub percent-encodes (such as `+`) would be refused with
a generic "could not read" message. This is harmless for `vX.Y.Z` tags.
**Fix:** Refuse such tags up front with a clear message, or encode the tag before comparing.

### IN-03: The checksum shows the file is not corrupted; it does not prove who published it (carried)

**File:** `src/update.rs:23` (`SUMS_NAME`), `run_check`
**Issue:** `SHA256SUMS.txt` comes from the same release as the installer, so anyone who can
upload one can upload both.
**Fix:** Document this limit, or sign the sums file later.

### IN-04: An exit that skips `begin_shutdown` leaves the downloaded installer behind (carried)

**File:** `src/overlay/mod.rs:126-137`
**Issue:** A logoff, an outside `WM_CLOSE` or an eframe error skips `update.shutdown()`. The
folder then stays until the 24 h sweep.
**Fix:** Add a best-effort cleanup in `OverlayApp`'s `on_exit` / `Drop` when no hand-off was
recorded.

### IN-05: ShellExecuteW without COM / SEE_MASK_NOASYNC, and the Install click hashes on the UI thread (carried)

**File:** `src/overlay/win.rs:669-694`, `src/overlay/update_job.rs:222`
**Issue:**
- Two problems on the elevation fallback path (`ShellExecuteW`), which runs right before the
  process exits:
  - COM is not initialized.
  - There is no `SEE_MASK_NOASYNC`.
  
  It is currently unreachable in practice: the installer is `PrivilegesRequired=lowest`, so
  error 740 should not occur. This fallback also releases the WR-02 lock as soon as
  `ShellExecuteW` returns.
- `begin_install` still hashes up to 200 MiB synchronously in the frame. The new launch-time hash
  runs after the window is gone, so it does not freeze anything.

**Fix:** Use `ShellExecuteExW` with `SEE_MASK_NOASYNC` (and `CoInitializeEx`), or drop the
fallback. Optionally, hash at the click on the worker.

### IN-06: `is_dev_build` cannot trigger in production, and the update message can name a different version than About

**File:** `src/update.rs:55-57`, `src/update.rs:888`, `src/overlay/about_view.rs:45`
**Issue:**
- `compare_release` gets `env!("CARGO_PKG_VERSION")`. Per the original review, `-dev` exists only
  in `FF_VERSION` (from `git describe`, `build.rs:47-49`), never in `Cargo.toml`. So the
  `is_dev_build` branch, and the `0.7.0-dev` / `0.7.0-dev.2+abc` test rows, cover a version string
  the shipped code never sees.
- A dev build of this branch is a `0.7.0-rc.1` build and will be offered `v0.7.0`. That is
  correct, but it is not what the comment says.
- The "Up to date (0.7.0-rc.1)" text comes from `CARGO_PKG_VERSION`, while About shows
  `FF_VERSION`. The two can differ on dev builds.

**Fix:** Either pass the version the rule is meant for, or drop `is_dev_build` and reword the
comment and the CONTEXT note. Consider naming the same version in both places.

### IN-07: A late "failed" or "ready" update message that waits behind another event is lost on the next edit or hide

**File:** `src/overlay/app.rs:2244-2250`, `src/overlay/app.rs:2252-2258` (`clear_events_on_edit`), `src/overlay/app.rs:2210-2218` (`clear_events_on_conceal`)
**Issue:**
- A worker message that ranks behind another event is only visible after that event goes away.
  But the edit or hide that clears the other event also clears `update_message`, so in practice
  the message is never shown.
- For Ready, the About button still says Install, so nothing is lost.
- For `Failed` ("Could not reach GitHub", "checksum mismatch"), the button goes back to Idle,
  and the user never learns that the check they started failed. The fixer flagged this trade-off.

**Fix:** Keep a behind-ranked update message across the next `clear_events_on_edit`, so it
surfaces once the other event is gone. Alternatively, show the last failure on the About button
or its tooltip.

### IN-08: A failed launch-time check leaves the installer folder in `%TEMP%`

**File:** `src/overlay/mod.rs:130-135`, `src/update.rs:255-271`
**Issue:** On `Gone`, `Mismatch` or `Spawn` errors at launch, the work folder is not deleted (for
`Spawn` this is intended, since the text points to the saved file). For a mismatched file this
leaves an unverified exe in `%TEMP%` until the 24 h sweep.
**Fix:** On `LaunchError::Check(Mismatch)`, delete the parent folder, but only if its name starts
with `WORK_PREFIX`.

---

_Reviewed: 2026-10-18_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
