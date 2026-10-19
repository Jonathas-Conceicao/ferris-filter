---
phase: 40-release-pipeline-installer
reviewed: 2026-10-18T00:00:00Z
depth: standard
iteration: 2
files_reviewed: 19
files_reviewed_list:
  - .github/workflows/ci.yml
  - .github/workflows/release.yml
  - .gitattributes
  - .gitignore
  - Cargo.toml
  - build.rs
  - installer/ferrisfilter.iss
  - installer/package.sh
  - installer/portable-readme.txt
  - installer/release-check.sh
  - installer/release-check.test.sh
  - installer/verify-exe.sh
  - src/overlay/app.rs
  - src/overlay/instance.rs
  - src/overlay/mod.rs
  - src/sync/client.rs
  - tools/ff-art/Cargo.toml
  - tools/ff-art/src/main.rs
  - assets/poe2/manifest.toml
findings:
  critical: 1
  warning: 3
  info: 9
  total: 13
status: issues_found
---

# Phase 40: Code Review Report (iteration 2)

**Reviewed:** 2026-10-18
**Depth:** standard
**Files Reviewed:** 19
**Status:** issues_found

## Summary

This is a re-review after the iteration-1 fix commits `ecaf684..ddb0d07`, against `149d030`.
I checked each fix and looked for regressions.

- **WR-01 (token):** fixed. `persist-credentials: false` is set, and no later step needs git
  credentials.
- **WR-02 (Inno Setup):** fixed in the workflow (pinned `choco install`, extra ISCC path). Whether
  `innosetup 6.7.1` exists on the Chocolatey feed can only be confirmed by the 40-07 run.
- **WR-03 (uninstall):** the quit is now checked. But it runs before the uninstall confirmation
  (WR-02 below).
- **WR-04 (PID section):** the logic is sound. I traced the cases: an old instance's section
  still open while a new one starts (the new one overwrites the pid), a fast exit between
  `-ready` and the read (falls back to the mutex), an exit after begin_shutdown (the exact
  process is waited for). All behave correctly. The weakness is that the new owner path is never
  exercised by a test (WR-01 below).
- **WR-05 (quit during reload):** correct. The poll at `app.rs:3508` runs after the check at
  `app.rs:3406`, so the shutdown lags one frame, the same as Apply & quit. There is one data-loss
  edge in the deferral window (WR-03 below).
- **WR-06 (CI runs the tests):** done. The known concern is confirmed and is a blocker (CR-01):
  the milestone close this worktree is preparing will turn every PR red.

All iteration-1 Info items except IN-03 are still open and are carried forward.
`bash installer/release-check.test.sh` passes on the current tree.

## Critical Issues

### CR-01: `real-changelog-rc` fails PR CI once the milestone close empties `[Unreleased]`

**File:** `installer/release-check.test.sh:95-97`, `.github/workflows/ci.yml:39-40`
**Issue:** The last case runs `release-check.sh` against the real `CHANGELOG.md` as
`v0.7.0-rc.1`. That reads `## [Unreleased]` and fails unless the section has non-blank text.
CLAUDE.md requires `[Unreleased]` to be moved into a dated version section when a milestone
completes, and the CHANGELOG header says `[Unreleased]` holds only changes made between
milestones. So right after the v0.7 close, the section is empty, `release-check.sh` exits 1
("no non-empty '## [Unreleased]' section"), and the case fails. Since WR-06 this suite runs on
every pull request, so every PR (including the milestone PR itself) goes red until someone adds
an `[Unreleased]` entry. The case checks the state of the repo's content, not a property of the
script. The script's own behaviour is already fully covered by the synthetic `cl.md`/`cl2.md`
cases.
**Fix:** Test the real file against its newest released section, which always exists, instead
of `[Unreleased]`:
```bash
# Real CHANGELOG: the newest released section extracts to non-empty notes.
latest="$(sed -n 's/\r$//; s/^## \[\([0-9][0-9.]*\)\] - .*/\1/p' CHANGELOG.md | head -n1)"
if [ -z "$latest" ]; then
  fail "real-changelog-final" "no released section in CHANGELOG.md"
else
  run CHANGELOG.md "$latest" "v$latest"
  if [ "$rc" -eq 0 ] && grep -q '[^[:space:]]' "$tmp/notes.md"; then pass "real-changelog-final"
  else fail "real-changelog-final" "$(cat "$tmp/err")"; fi
fi
```
Or drop the case. Separately, keep in mind that the `v0.7.0-rc.1` dry run (40-07) itself needs a
non-empty `[Unreleased]`. If the milestone is closed before 40-07 runs, the rc tag fails
`release-check.sh` by design.

## Warnings

### WR-01: The owner-process path of the WR-04 fix has no automated test

**File:** `src/overlay/instance.rs:395-404, 645-669`, tests at `1036-1096`
**Issue:** `owner_process` returns `None` whenever the published pid equals the current process.
Every `quit_running_named` test runs the instance in-process, so all of them take the
`mutex_gone` fallback. The `Some(process) => has_exited(process)` branch, which is the whole
point of WR-04, is never run by any test. The new test
`the_instance_publishes_its_process_id_until_it_drops` checks only the section's contents. If
`OpenProcess` access, `has_exited` or the handle close regressed, the tests would not notice,
and the only check is a manual one in 40-07.
**Fix:** Let a test publish another process's pid. Give `publish_pid` a pid parameter, or add a
`#[cfg(test)]` helper that creates the section with a chosen pid. Then:
```rust
#[test]
fn quit_waits_for_the_published_owner_process() {
    let n = test_names("owner");
    let mut child = std::process::Command::new("cmd")
        .args(["/c", "ping -n 30 127.0.0.1 >nul"]).spawn().unwrap();
    let mut g = first_with_pid(&n, child.id()); // section names the child, not us
    g.mark_ready();
    drop_mutex_only(&mut g); // or keep it: the result must follow the child either way
    assert_eq!(quit_running_named(&n, 2000, 300), QuitOutcome::StillRunning);
    child.kill().unwrap(); child.wait().unwrap();
    assert_eq!(quit_running_named(&n, 2000, 3000), QuitOutcome::Exited);
}
```
The first assertion has to hold even after the guard's mutex is gone. That proves an early name
release is no longer taken as an exit.

### WR-02: The uninstall quit runs before the user confirms the uninstall

**File:** `installer/ferrisfilter.iss:110-119`
**Issue:** Inno calls `InitializeUninstall` during uninstaller start-up, before its "Are you sure
you want to completely remove FerrisFilter?" prompt. A user who starts Uninstall and then clicks
**No** has already had a clean ff quit underneath them. With unapplied edits, the user first sees
ff's Unapplied changes popup, and possibly a Retry box, before even being asked whether to
uninstall. The installer path does not have this problem, because `PrepareToInstall` runs after
the wizard pages.
**Fix:** Do the quit after the confirmation. Use `CurUninstallStepChanged` at `usAppMutexCheck`
(the first step after the prompt), and call `Abort` when the quit fails. Check in the Inno 6
docs that `Abort` is honoured at that step during the 40-07 compile:
```pascal
procedure CurUninstallStepChanged(CurUninstallStep: TUninstallStep);
begin
  if (CurUninstallStep = usAppMutexCheck) and not QuitRunningFerrisFilter() then
  begin
    SuppressibleMsgBox('FerrisFilter is still running. Quit it from its tray icon, then ' +
      'uninstall again.', mbError, MB_OK, IDOK);
    Abort;
  end;
end;
```
If `Abort` turns out not to be allowed there, keep `InitializeUninstall` but show the
confirmation yourself first and run silent-aware (`UninstallSilent`).

### WR-03: A deferred clean quit discards edits made while it waits

**File:** `src/overlay/app.rs:2500-2504, 3406-3410`
**Issue:** `request_quit` decides that nothing is dirty, then defers the shutdown while the
reload worker runs. When the worker finishes, the frame loop calls `begin_shutdown`
unconditionally, with no new `quit_dirty()` check. Any edit made during the deferral is lost
without the Unapplied changes popup. Examples: the user summons the window with the hotkey or a
second launch and changes something, or a `styles.json` change on disk marks a style rewrite
pending. The window is short (one reload sequence, well under a second), but the outcome is
silent data loss. That is exactly what the quit confirm exists to prevent. The Apply & quit path
has the same unconditional shutdown. There the user has just chosen to quit, but the same
re-check would be harmless.
**Fix:** Record why the quit was deferred, and check again when it fires:
```rust
if self.quit_after_apply && !self.reload.is_running() {
    self.quit_after_apply = false;
    if self.quit_dirty() {
        self.quit_confirm = true; // edits arrived meanwhile: ask, as a fresh tray Quit would
        if self.visibility == Visibility::Hidden { self.toggle_requested = true; }
    } else {
        quit_requested = true;
        self.begin_shutdown();
    }
}
```
If an Apply & quit can legitimately leave `quit_dirty()` true, keep a separate
`quit_after_reload` flag for the clean-quit deferral and re-check only that one.

## Info

### IN-01: `--quit` still creates `-ready`/`-quit` events, and the comments still say it does not

**File:** `src/overlay/mod.rs:45-48`, `src/overlay/instance.rs:635-636`
**Issue:** Carried over from iteration 1. `open_event` calls `CreateEventW`. The comment in
`mod.rs` ("It only opens the instance objects, never creates them") is unchanged. The
`instance.rs` doc comment was corrected.
**Fix:** Use `OpenEventW(SYNCHRONIZE | EVENT_MODIFY_STATE, 0, name)` on the quit path, or reword
`mod.rs` to say only the mutex is never created.

### IN-02: `OpenMutexW` is still given `PROCESS_SYNCHRONIZE`

**File:** `src/overlay/instance.rs:580`
**Issue:** Carried over. The value is equal, but it is the wrong constant family. Now that
`OpenProcess(PROCESS_SYNCHRONIZE, ...)` sits a few lines away, the mutex use reads even more like
a copy slip.
**Fix:** Use `windows_sys::Win32::Foundation::SYNCHRONIZE` (or the `Storage::FileSystem` one) for
the mutex.

### IN-03: The `Win32_System_Diagnostics_ToolHelp` feature is now unused

**File:** `Cargo.toml:50`
**Issue:** WR-04 removed the only ToolHelp user (the process scan). The feature still compiles an
unused module and suggests the process scan still exists.
**Fix:** Remove `"Win32_System_Diagnostics_ToolHelp"` from the feature list (and re-lock).

### IN-04: PID read and `OpenProcess` are not atomic

**File:** `src/overlay/instance.rs:395-404`
**Issue:** If the instance exits between `published_pid` and `OpenProcess`, and Windows reuses
the pid at once, `--quit` waits for an unrelated process and reports StillRunning. This is very
unlikely (pid reuse that fast is rare). When `OpenProcess` fails, the code silently falls back to
the mutex, which brings back the iteration-1 "too early" case without any trace.
**Fix:** After opening the handle, check the mutex/section once more (re-read the pid and compare
it). At least `eprintln!` when the fallback is taken while a section exists.

### IN-05: The profile comment still sits above `[build-dependencies]`

**File:** `Cargo.toml:57-63`
**Issue:** Carried over: the "Optimise dependencies ..." comment still introduces winresource.
**Fix:** Move `[build-dependencies]` above that comment.

### IN-06: `LegalCopyright` still holds a license identifier

**File:** `build.rs:85`
**Issue:** Carried over.
**Fix:** `"Copyright (C) 2026 Jonathas Conceicao. GPL-3.0-or-later."`

### IN-07: `verify-exe.sh` still matches the version as a substring

**File:** `installer/verify-exe.sh:21-29`
**Issue:** Carried over: `ProductVersion0.7.0` also matches `0.7.0-rc.1`.
**Fix:** Anchor the version, for example
`grep -a -E -q "ProductVersion${version//./\\.}([^0-9A-Za-z.+-]|$)"`.

### IN-08: The icon downscale uses straight alpha; frame sizes are not validated

**File:** `tools/ff-art/src/main.rs:584-612`
**Issue:** Carried over: Lanczos3 on demultiplied RGBA leaves dark fringes at 16/24 px, and sizes
0 or >256 are not rejected up front.
**Fix:** Resize premultiplied data; `anyhow::ensure!((1..=256).contains(&s), ...)`.

### IN-09: Silent install and uninstall abort whenever ff is still running

**File:** `installer/ferrisfilter.iss:97-98, 113-119`
**Issue:** Carried over, and now it also covers uninstall. Under `/SUPPRESSMSGBOXES` the Retry
box returns `IDCANCEL`, so a silent install or uninstall over a running ff with edits fails
without a retry. That may be intended, but it is not documented.
**Fix:** Document it in the script header.

---

_Reviewed: 2026-10-18_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
