---
phase: 40-release-pipeline-installer
fixed_at: 2026-10-18T00:00:00Z
review_path: .planning/phases/40-release-pipeline-installer/40-REVIEW.md
iteration: 1
findings_in_scope: 6
fixed: 6
skipped: 0
status: all_fixed
---

# Phase 40: Code Review Fix Report

**Fixed at:** 2026-10-18
**Source review:** .planning/phases/40-release-pipeline-installer/40-REVIEW.md
**Iteration:** 1

**Summary:**
- Findings in scope: 6 (warnings; Info findings were out of scope)
- Fixed: 6
- Skipped: 0

Verification: `bash installer/release-check.test.sh` passes; `cargo test --locked --lib overlay`
passes (877 tests, FERRISFILTER_HOME / FERRISFILTER_GAME_DIR in an empty temp dir,
CARGO_TARGET_DIR=/tmp/fftgt40); clippy reports nothing in the touched Rust files. The workflow
YAML was checked by re-reading only (no YAML parser on this machine). The Inno script change is
**uncompiled** (Inno Setup is not installed locally); plan 40-07 compiles it.

## Fixed Issues

### WR-01: The write-scoped GITHUB_TOKEN is reachable from every build and test step

**Files modified:** `.github/workflows/release.yml`
**Commit:** ecaf684
**Applied fix:** `persist-credentials: false` on the checkout step, so the token is never
written to `.git/config`. The publish step already gets it through `GH_TOKEN`; no later step
needs git credentials (`git describe` and `git status` are local). The job split into a
read-only build job and a write publish job was not done (the smaller fix the review offers).

### WR-02: The release job assumes Inno Setup is preinstalled on the runner

**Files modified:** `.github/workflows/release.yml`, `installer/package.sh`
**Commit:** bf23cd7
**Applied fix:** New step `choco install innosetup --version=6.7.1 --allow-downgrade
--no-progress -y` right before packaging (6.7.1 is the newest 6.x on the Chocolatey feed today;
`--allow-downgrade` covers an image that ships a newer one). `package.sh` also searches
`/c/Program Files/Inno Setup 6/ISCC.exe`. The run body has no `${{` and no PowerShell; the
40-02 greps (pwsh/powershell/--workspace/ff-art, pinned action SHAs, fetch-depth) still hold.
Needs a first real run in 40-07 to confirm the choco package installs on `windows-latest`.

### WR-03: Uninstall ignores the `--quit` outcome, so a running ff.exe is left behind

**Files modified:** `installer/ferrisfilter.iss`
**Commit:** 89fa20f
**Status:** fixed: requires human verification (uncompiled)
**Applied fix:** Added `InitializeUninstall`, which runs the same `QuitRunningFerrisFilter`
Retry loop as the installer and aborts the uninstall with a (suppressible) message when ff is
still running. The `[UninstallRun]` entry was **kept** rather than dropped: the 40-05
acceptance criteria require `RunOnceId: "QuitFerrisFilter"` once, and after the new check it
finds nothing running, so it is harmless. File stays ASCII only; the forbidden-word grep
(AppMutex, ff-cli, ...) still returns 0.

### WR-04: `--quit` exit detection can be wrong in both directions

**Files modified:** `src/overlay/instance.rs`
**Commit:** ddb0d07
**Status:** fixed: requires human verification
**Applied fix:** The PID approach the review suggests. The instance that wins the mutex now
writes its process id into a 4-byte named file mapping `<base>-pid` (`InstanceNames.pid`, kept
by the Guard, closed on drop). `ff --quit` reads it after `-ready` is signalled and before
sending the request, opens that one process with SYNCHRONIZE, and reports Exited only when it
has exited. Other ff.exe processes no longer count, and releasing the name early in shutdown
is no longer taken as an exit. With no readable section (creation failed, or the id is this
process, as in the in-process tests) it falls back to waiting for the mutex to go. The
image-path process scan (`same_exe_processes`, `all_exited`, `MAXIMUM_WAIT_OBJECTS`) was
removed. That also removes the const that sat between the `use` lines (IN-03). New test
`the_instance_publishes_its_process_id_until_it_drops`. The cross-process case (instance
in another process) has no automated test, since that would mean spawning `ff`. Worth a manual
check in 40-07. The `Win32_System_Diagnostics_ToolHelp` windows-sys feature in Cargo.toml is
now unused; it was left in place to keep this fix out of Cargo.toml.

### WR-05: `ff --quit` can kill the process while an in-game reload is still running

**Files modified:** `src/overlay/app.rs`
**Commit:** c125059
**Status:** fixed: requires human verification
**Applied fix:** In `request_quit`, when the step is QuitNow but `self.reload.is_running()`,
set `quit_after_apply` and return false. The existing "quit when the reload worker is idle"
path in the frame loop then begins the shutdown once the paste and clipboard restore have
finished. This covers tray Quit and `ff --quit` alike. New test
`a_clean_quit_waits_for_a_running_in_game_reload`.

### WR-06: The release-check test suite is never executed

**Files modified:** `.github/workflows/ci.yml`
**Commit:** 4d31040
**Applied fix:** Added a `Test the release scripts` step (`bash installer/release-check.test.sh`)
before `cargo test` in the PR workflow. Note: its `real-changelog-rc` case runs against the
real CHANGELOG.md and expects non-empty pre-release notes from `[Unreleased]`. Once a milestone
close empties `[Unreleased]`, that case (and so PR CI) would fail until the case is adjusted.

---

_Fixed: 2026-10-18_
_Fixer: Claude (gsd-code-fixer)_
_Iteration: 1_
