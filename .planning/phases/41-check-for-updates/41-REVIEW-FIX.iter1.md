---
phase: 41-check-for-updates
fixed_at: 2026-10-18T00:00:00Z
review_path: .planning/phases/41-check-for-updates/41-REVIEW.md
iteration: 1
findings_in_scope: 5
fixed: 5
skipped: 0
status: all_fixed
---

# Phase 41: Code Review Fix Report

**Fixed at:** 2026-10-18
**Source review:** .planning/phases/41-check-for-updates/41-REVIEW.md
**Iteration:** 1

**Summary:**
- Findings in scope: 5 (CR-01, WR-01..WR-04; the Info findings IN-01..IN-05 are out of scope)
- Fixed: 5
- Skipped: 0

Work was done directly on `milestone/v0.7-initial-public-release-and-strictness-selector` in the
`milestone-closeout` worktree, as the orchestrator instructed. No extra worktree or branch was
made. Verification: `cargo test --locked --lib update` (51 passed) and
`cargo test --locked --lib overlay::` (913 passed), `cargo build --locked --bins`, rustfmt on the
touched files only, and no new clippy warnings in touched code. Every warning clippy reports in
`app.rs` is in code that existed before these fixes. Tests use no network, only temp dirs, and
never start an installer.

## Fixed Issues

### CR-01: "Apply & quit" that stays open (unwritable tier edits) keeps the install intent

**Files modified:** `src/overlay/app.rs`
**Commit:** 9d93376
**Applied fix:** The `RunApply` branch that stays open for unused tier edits (Fallback prices) now
calls `self.update.cancel_install()`, the same as the refused-Apply branch. I extracted a
`make_ready(&mut app)` test helper and added
`install_click_then_apply_and_quit_kept_open_by_unused_tier_edits_clears_the_intent`. It covers
both routes (other work plus unused tier edits, and only unused tier edits with Apply & quit) and
then calls `begin_shutdown()`. It asserts that nothing is handed off and that the installer is
deleted. I confirmed the test fails without the fix.

### WR-01: A late worker message masks newer Apply / Sync outcomes

**Files modified:** `src/overlay/app.rs`, `src/overlay/shell.rs`
**Commit:** 6c07a47
**Status:** fixed: requires human verification (logic change in message precedence)
**Applied fix:** There is a new `update_behind` flag. A message the worker delivers through
`poll` goes through `receive_update_message`. That function sets the flag when an Apply, CONFIG,
sync (`Finished`) or list message is showing. `event_from` takes the flag. A message that arrived
late ranks below every other event, and it still shows when nothing else is showing. A message
set on the UI thread (the Checking line on click, an Install-click failure) clears the flag and
still ranks first. Tests: new `event_from` rows in `shell.rs`, and
`a_late_update_message_waits_behind_a_newer_apply_outcome` in `app.rs`.
**Trade-off:** I chose this over a full event sequence counter because it is smaller. One
consequence: a late "available" or "failed" message that waits behind another event is cleared by
the next edit together with that event. The About button still shows the Ready / Idle state.

### WR-02: The installer is not hashed again right before it is started

**Files modified:** `src/update.rs`, `src/overlay/update_job.rs`, `src/overlay/app.rs`, `src/overlay/mod.rs`, `CLAUDE.md`
**Commit:** d723e35
**Applied fix:** `UpdateJob::shutdown()` and the `pending_install` slot now carry the whole
`ReadyUpdate` (path plus expected SHA-256). `launch()` calls the new
`update::launch_verified_installer`. That function opens the file with `share_mode(FILE_SHARE_READ)`
(no write or delete sharing), hashes through that handle, and starts the installer while the
handle is still open. A missing file or a mismatch shows an error box (`launch_error_text`) and
the file is not run. Tests: through a `launch_verified_with` spawn seam, with no real process. They
cover a matching file (and check that writing or deleting it fails while it is locked), a changed,
missing or directory path, a spawn failure, and the error texts. I checked once, with a throwaway
test that I did not commit, that `CreateProcess` starts a copied system exe while this handle is
open. The ShellExecute elevation fallback (error 740) was not tested with the handle held.
CLAUDE.md's update note now mentions the second check.

### WR-03: Release-candidate builds are told they are up to date when their final release is out

**Files modified:** `src/update.rs`, `CLAUDE.md`
**Commit:** 8339209
**Status:** fixed: requires human verification (version-rule logic)
**Applied fix:** In `compare_release`, a running pre-release with the same major.minor.patch as
the stable tag now counts as older (`Newer(tag)`), as semver orders it. A `-dev` / `-dev.N`
build (`is_dev_build`) still compares on its base version and gets `Same`. The up-to-date
message names the running version without build metadata (`running_version`, e.g.
`0.7.0-rc.1`), not the base version.
**CONTEXT reasoning:** 41-CONTEXT.md says "compared with the exe's Cargo.toml version (semver);
a dev build compares on its base version". An `-rc` build is a pre-release, not a dev build. Semver
puts `0.7.0-rc.1` below `0.7.0`, so offering the final release to an RC tester follows the locked
decision. The existing `0.7.0-dev` vs `v0.7.0` == `Same` row (the dev-build rule) is kept and
still passes. New rows: `0.7.0-rc.1` vs `v0.7.0` is Newer, vs `v0.6.2` is Older, `0.7.0-dev.2+abc`
and `0.7.0+build.5` are Same. There is also a `run_check` test in which an RC build is offered and
downloads its final release, and one in which it names itself when it is up to date.

### WR-04: Quit while Downloading can leak the work folder, and delete it while the worker is still writing

**Files modified:** `src/update.rs`, `src/overlay/update_job.rs`
**Commit:** 8e65f93
**Applied fix:** The shared slot is now `Mutex<WorkSlot { dir, cancelled }>`. `run_check`
creates and publishes the folder while holding that lock, and gives up if `cancelled` is set.
`shutdown()` (via `abandon(true)`) sets `cancelled` and takes the folder in the same lock. So a
shutdown either finds the folder or no folder is ever created. The download sink checks
`cancelled` before each chunk and returns an `Interrupted` error, so the download ends and the
`TempDir` drop deletes the folder. The final `keep()` also runs under the lock and does not keep
the folder after a stop. Tests: `a_stopped_check_creates_no_folder`,
`a_check_stopped_mid_download_ends_it_and_deletes_its_folder` (a stop after the first chunk and
after the last one), and
`shutdown_while_the_download_file_is_open_deletes_it_and_stops_the_worker`. That last test holds
the partial file open and writes to it while `shutdown()` runs, and checks that the folder is gone
and the worker saw the stop. A throwaway probe (not committed) confirmed that `remove_dir_all`
succeeds on this machine while std's default handle is open.
**Not done:** shutdown does not wait for the worker to finish. The deletion that shutdown does
itself is enough on NTFS (POSIX delete semantics), and if the process lives long enough the
worker's own cleanup handles the rest.

---

_Fixed: 2026-10-18_
_Fixer: Claude (gsd-code-fixer)_
_Iteration: 1_
