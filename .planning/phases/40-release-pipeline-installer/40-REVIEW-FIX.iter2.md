---
phase: 40-release-pipeline-installer
fixed_at: 2026-10-18T00:00:00Z
review_path: .planning/phases/40-release-pipeline-installer/40-REVIEW.md
iteration: 2
findings_in_scope: 4
fixed: 4
skipped: 0
status: all_fixed
---

# Phase 40: Code Review Fix Report

**Fixed at:** 2026-10-18
**Source review:** .planning/phases/40-release-pipeline-installer/40-REVIEW.md
**Iteration:** 2

**Summary:**
- Findings in scope: 4 (1 critical, 3 warnings; the 9 Info findings are out of scope)
- Fixed: 4
- Skipped: 0

Verification: `bash installer/release-check.test.sh` passes (13 cases). `cargo test --locked --lib
overlay` passes (880 tests, FERRISFILTER_HOME / FERRISFILTER_GAME_DIR in an empty temp dir,
CARGO_TARGET_DIR=/tmp/fftgt40). rustfmt is clean on `instance.rs`. The one rustfmt diff left in
`app.rs` (around line 1333) was already there at HEAD and is outside the edited code. Clippy
reports nothing new in the touched code. The Inno script change is **uncompiled** (Inno Setup is
not installed locally); plan 40-07 compiles it. CHANGELOG.md and README.md were not edited.

## Fixed Issues

### CR-01: `real-changelog-rc` fails PR CI once the milestone close empties `[Unreleased]`

**Files modified:** `installer/release-check.test.sh`
**Commit:** 12e9771
**Applied fix:** The real-CHANGELOG case now finds the newest `## [X.Y.Z] - <date>` heading
(CR-tolerant `sed`) and runs `release-check.sh` as that final version. It passes when the
extracted notes are non-empty and fails clearly when no released section exists. Renamed to
`real-changelog-final`. `[Unreleased]` is no longer read, so an empty section between milestones
cannot turn PR CI red. The synthetic `cl.md` / `cl2.md` / CRLF cases are unchanged and still
cover the pre-release path. Today it checks `[0.6.0]`. Note for 40-07, as the review says: the
`v0.7.0-rc.1` dry run itself needs a non-empty `[Unreleased]` by design.

### WR-03: A deferred clean quit discards edits made while it waits

**Files modified:** `src/overlay/app.rs`
**Commit:** 4668e8f
**Status:** fixed: requires human verification (quit-state logic)
**Applied fix:** A separate `quit_after_reload` flag (the review's fallback option) now handles
the clean-quit deferral, so Apply & quit (`quit_after_apply`) keeps its unconditional shutdown.
The new `run_deferred_quit()` replaces the inline frame-loop check. Once the reload worker is
idle, Apply & quit shuts down. A deferred clean quit calls `request_quit()` again: it re-checks
`quit_dirty()` and either shuts down, or opens the Unapplied changes popup and reveals a hidden
window. Whenever `request_quit` opens or keeps the popup, it clears `quit_after_reload`. The
popup then owns the decision, so its Cancel is never followed by a quit. The existing test now
drives `run_deferred_quit`. New test `edits_made_while_a_clean_quit_waits_open_the_quit_popup`:
a tier edit lands during the deferral, and the popup opens with the edit kept.

### WR-02: The uninstall quit runs before the user confirms the uninstall

**Files modified:** `installer/ferrisfilter.iss`
**Commit:** 7475055
**Status:** fixed: requires human verification (uncompiled)
**Applied fix:** I replaced `InitializeUninstall` with
`CurUninstallStepChanged(CurUninstallStep: TUninstallStep)`. It acts at **`usUninstall`**, not
the `usAppMutexCheck` the review suggests, because the 40-05 forbidden-word grep is
case-insensitive and `usAppMutexCheck` contains `AppMutex`. `usUninstall` also fires after the
"Are you sure" prompt and before the uninstall log is processed, so no file is removed yet and
`[UninstallRun]` has not run. On a failed quit it shows the same suppressible message and calls
`Abort`. Per the Inno 6 `Abort` documentation, `CurUninstallStepChanged(usUninstall)` is one of
the event points where Abort ends the uninstaller (with `InitializeUninstall` and
`usAppMutexCheck`). ASCII only, and every 40-05 acceptance grep still holds (forbidden words 0,
the single-occurrence items 1 each, 2 `Source:` lines, 1 `Flags: unchecked`).
**Question for 40-07:** confirm in the compiled uninstaller three things. Answering No at the
prompt leaves a running ff alone. A failed quit (leave the Unapplied changes popup open, then
Cancel on the Retry box) shows the message and removes nothing. Abort at `usUninstall` exits
cleanly. If Abort is not honoured there, go back to `InitializeUninstall` with a self-shown,
`UninstallSilent`-aware confirmation, as the review suggests.

### WR-01: The owner-process path of the WR-04 fix has no automated test

**Files modified:** `src/overlay/instance.rs`
**Commit:** 0e6d8ca
**Applied fix:** Added a `#[cfg(test)]` helper `imp::overwrite_published_pid` that rewrites an
existing `-pid` section. The instance stays in-process and the section is pointed at a child
`%SystemRoot%\System32\PING.EXE -n 60 127.0.0.1`: no GUI and no ff instance, wrapped so the child
is killed on drop. Test names are per-process `Local\FerrisFilter-test-...`, never the production
ones. Two tests:
- `quit_follows_the_published_owner_process_not_the_name`: StillRunning while the child lives,
  then Exited once it ends, even though the name is still held (the mutex fallback would say
  StillRunning).
- `an_instance_releasing_its_name_early_is_not_taken_as_its_exit`: the instance thread drops its
  whole guard on the quit request while the child runs on, and `--quit` still reports
  StillRunning.

Mutation check: with `owner_process` forced to return `None`, both tests fail. The child `Child`
handle stays open until the end of each test, so its pid cannot be reused mid-test.

## Iteration 1 (record)

Commits `ecaf684..ddb0d07`, 6 warnings, all fixed (the full report is kept in
`40-REVIEW-FIX.iter1.md`):

| Finding | Commit | Fix |
|---------|--------|-----|
| WR-01 write token reachable from build steps | ecaf684 | `persist-credentials: false` on the release checkout |
| WR-02 Inno Setup assumed on the runner | bf23cd7 | pinned `choco install innosetup --version=6.7.1`; extra ISCC path in `package.sh` |
| WR-03 uninstall ignored the `--quit` outcome | 89fa20f | uninstall-time Retry loop and abort (moved after the confirmation in this iteration's WR-02) |
| WR-04 `--quit` exit detection | ddb0d07 | instance publishes its pid in a `-pid` section; `--quit` waits for that process |
| WR-05 quit during an in-game reload | c125059 | quit deferred until the reload worker is idle (refined in this iteration's WR-03) |
| WR-06 release-check tests never run | 4d31040 | `bash installer/release-check.test.sh` step in PR CI (its real-CHANGELOG case fixed by this iteration's CR-01) |

---

_Fixed: 2026-10-18_
_Fixer: Claude (gsd-code-fixer)_
_Iteration: 2_
