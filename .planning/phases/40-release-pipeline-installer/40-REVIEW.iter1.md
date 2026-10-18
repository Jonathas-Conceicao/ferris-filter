---
phase: 40-release-pipeline-installer
reviewed: 2026-10-18T00:00:00Z
depth: standard
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
  critical: 0
  warning: 6
  info: 8
  total: 14
status: issues_found
---

# Phase 40: Code Review Report

**Reviewed:** 2026-10-18
**Depth:** standard
**Files Reviewed:** 19
**Status:** issues_found

## Summary

I reviewed the release pipeline (ci.yml and release.yml), the installer tooling (Inno script,
package, release-check, verify-exe), the exe resource in `build.rs`, the `ff --quit` channel
(`instance.rs`, `mod.rs`, `app.rs` hunks), the user-agent change and the ff-art icon renderer,
all against `149d030`.

The design holds up. The tag and Cargo version gate, the CHANGELOG extraction (CRLF safe, stops
at link references), publishing as a draft first, the per-user install, the single-request quit
with its withdraw on timeout, and the "only open the mutex" rule are all sound, and I traced
them through their edge cases.

No blockers. The main problems:
- The write-scoped token is reachable from every build and test step.
- The release job assumes Inno Setup is already on the runner.
- The uninstaller ignores the `--quit` result.
- Two edge cases in how `--quit` detects that the instance has exited.
- `--quit` can cut off an in-game reload that is still running.
- The shell test suite is never run by CI.

## Warnings

### WR-01: The write-scoped GITHUB_TOKEN is reachable from every build and test step

**File:** `.github/workflows/release.yml:13-14, 29-32, 49-50, 60-61`
**Issue:** `permissions: contents: write` is set for the whole job, and `actions/checkout` keeps
its default `persist-credentials: true`. The token is therefore stored in `.git/config` for every
later step, including `cargo test --locked` and `cargo build --release`. Those steps run every
dependency's build script and proc macros. A compromised crate, or a test that runs `git push`,
could push to the repo or move tags. Only the final `gh release` step needs write access.
**Fix:** Stop the checkout from saving the token, and pass it only to the publish step (which
already uses `GH_TOKEN`):
```yaml
      - name: Check out the repository
        uses: actions/checkout@3d3c42e5aac5ba805825da76410c181273ba90b1 # v7.0.1
        with:
          fetch-depth: 0
          persist-credentials: false
```
For stronger isolation, split the job: one job builds and tests with `contents: read` and
uploads `dist/` as an artifact, and a second publish job with `contents: write` only downloads
it and runs `gh release`.

### WR-02: The release job assumes Inno Setup is preinstalled on the runner

**File:** `.github/workflows/release.yml:74-78`, `installer/package.sh:24-40`
**Issue:** No step installs Inno Setup. `package.sh` searches only `iscc.exe` on PATH,
`C:\Program Files (x86)\Inno Setup 6` and `%LOCALAPPDATA%\Programs\Inno Setup 6`. Whether
`windows-latest` has Inno Setup, and at which path, depends on the image, and the image
changes underneath the workflow. This has not been checked yet: the planning notes say plan 40-07
has not compiled the installer anywhere. If the image lacks Inno Setup, or installs it under
`C:\Program Files\Inno Setup 6`, every tag fails at the packaging step, after a full test and
release build. The failure is loud, but it costs a tag (rc re-tags are meant to be rare).
**Fix:** Install a pinned version explicitly before packaging, and add the 64-bit path to the
search list:
```yaml
      - name: Install Inno Setup
        run: choco install innosetup --version=6.x.y --no-progress -y
```
```bash
  for c in "$(command -v iscc.exe || true)" \
           "/c/Program Files (x86)/Inno Setup 6/ISCC.exe" \
           "/c/Program Files/Inno Setup 6/ISCC.exe" \
           "$local_app"; do
```

### WR-03: Uninstall ignores the `--quit` outcome, so a running ff.exe is left behind

**File:** `installer/ferrisfilter.iss:76-77`
**Issue:** `[UninstallRun]` runs `ff.exe --quit` and ignores its exit code. If the instance has
unapplied edits, its popup opens. After 10 s `--quit` returns 1 (StillRunning), and the
uninstaller goes on to delete `{app}\ff.exe` while that file is still running. With
`PrivilegesRequired=lowest` the uninstaller cannot schedule a delete on reboot (that needs
admin). The exe stays behind next to a half-removed install, and the user sees an "some elements
could not be removed" message. The installer path (`PrepareToInstall`) has a Retry loop for
this case; the uninstall path does not.
**Fix:** Do the uninstall quit in `[Code]`, reusing the installer's loop, and drop the
`[UninstallRun]` entry:
```pascal
function InitializeUninstall(): Boolean;
begin
  Result := QuitRunningFerrisFilter();
  if not Result then
    MsgBox('FerrisFilter is still running. Quit it from its tray icon, then uninstall again.',
      mbError, MB_OK);
end;
```
`QuitRunningFerrisFilter` uses `{app}`, which is available during uninstall.

### WR-04: `--quit` exit detection can be wrong in both directions

**File:** `src/overlay/instance.rs:530-577, 579-585, 612-640`
**Issue:**
1. **Too strict:** when `targets` is not empty, success means that *every* `ff.exe` process with
   the same image path has exited, not that the instance has. Other processes can share the
   path: a second launch blocked in its "already running but not responding" message box, a
   concurrent `--quit` from an uninstall started at the same time, or a launcher that took over
   as the new instance once the old one quit (CLAUDE.md says such a launch "runs as the new
   instance"). Any of these makes `all_exited` stay false, so the result is StillRunning (exit 1)
   even though the instance quit. The installer then shows a misleading Retry box.
2. **Too early:** when `targets` is empty (the instance runs from a different path than the
   `--quit` exe, or its image path cannot be read), success means the mutex is gone. But
   `begin_shutdown` (`app.rs:2558`) drops the guard *before* the viewport closes, so `Exited` is
   reported while the process is still alive.
**Fix:** Wait for the process that actually owns the instance. One approach: have the instance
write its PID somewhere `--quit` can read (for example the payload of a named file mapping, or a
`-pid` object name). Then `OpenProcess(SYNCHRONIZE, pid)` once and `WaitForSingleObject` on that
handle. At a minimum, report success only when both the mutex is gone *and* the matching
processes have exited, and leave out processes that never held the mutex.

### WR-05: `ff --quit` can kill the process while an in-game reload is still running

**File:** `src/overlay/app.rs:2494-2510, 3386-3404`
**Issue:** `request_quit` calls `begin_shutdown` and returns true whenever `quit_dirty()` is
false. It does not check `self.reload.is_running()`. Just after a successful Apply the state is
clean, and the reload worker may still be pasting `/reloaditemfilter` or waiting about 120 ms to
restore the clipboard. Closing the process at that point stops the sequence partway. The
user's clipboard can be left holding `/reloaditemfilter`, or the chat line can be left open with
the command typed. The Apply & quit path (`app.rs:3398`) waits for this worker on purpose. Tray
Quit already had the same gap, but `--quit` now lets the installer trigger it with no user
action.
**Fix:** Defer the shutdown while the reload worker runs, as the Apply & quit path does:
```rust
if step == apply::QuitStep::QuitNow {
    if self.reload.is_running() {
        self.quit_after_apply = true; // reuse the "quit when idle" path
        return false;
    }
    self.begin_shutdown();
    return true;
}
```

### WR-06: The release-check test suite is never executed

**File:** `.github/workflows/ci.yml:39-40`, `installer/release-check.test.sh`
**Issue:** `release-check.test.sh` covers the tag regex, the CHANGELOG extraction and CRLF
handling, but no workflow and no Rust test runs it (a grep for `release-check.test` finds only
the file itself). A regression in `release-check.sh` would only show up when a version tag is
pushed, which is the exact failure the dry-run budget is meant to avoid.
**Fix:** Run the suite in the PR workflow (it needs only bash, awk and grep):
```yaml
      - name: Test the release scripts
        run: bash installer/release-check.test.sh
```

## Info

### IN-01: `--quit` creates kernel objects, but the comments say it only opens them

**File:** `src/overlay/mod.rs:45-48`, `src/overlay/instance.rs:587-588, 604-605`
**Issue:** The comments say `--quit` "only opens the instance objects, never creates them". That
holds for the mutex. For `-ready` and `-quit`, `open_event` calls `CreateEventW`, which creates
each event if it is missing; the `ERROR_ALREADY_EXISTS` check only notices this afterwards. The
effect is harmless: a pre-`--quit` instance, or one whose events failed, is reported as
Unreachable. If the instance exits between `open_mutex` and `open_event`, the result is also
Unreachable (exit 2) instead of NotRunning.
**Fix:** Use `OpenEventW(SYNCHRONIZE | EVENT_MODIFY_STATE, 0, name)` for the quit path, or reword
the comments to say that only the mutex is never created.

### IN-02: `OpenMutexW` is given a process access-right constant

**File:** `src/overlay/instance.rs:493`
**Issue:** `PROCESS_SYNCHRONIZE` happens to equal `SYNCHRONIZE` (0x00100000), but it is a
process access right being passed for a mutex, which misleads readers.
**Fix:** Use `windows_sys::Win32::Foundation::SYNCHRONIZE` (or `Win32::Storage::FileSystem::SYNCHRONIZE`,
depending on the windows-sys version).

### IN-03: A `const` sits between the `use` declarations

**File:** `src/overlay/instance.rs:204-208`
**Issue:** `MAXIMUM_WAIT_OBJECTS` is declared between two `use` blocks of `mod imp`. rustfmt does
not move it.
**Fix:** Move it below the last `use`.

### IN-04: The profile comment now sits above `[build-dependencies]`

**File:** `Cargo.toml:58-66`
**Issue:** The new `[build-dependencies]` table was inserted between the comment that explains
`[profile.dev.package."*"]` ("Optimise dependencies ... Our own crate stays at opt-level 0") and
that profile. The comment now appears to describe winresource.
**Fix:** Move `[build-dependencies]`, with its own comment, above the profile comment block.

### IN-05: `LegalCopyright` holds a license identifier

**File:** `build.rs:85`
**Issue:** Explorer shows `LegalCopyright` as "Copyright". `GPL-3.0-or-later` is a license, not a
copyright notice.
**Fix:** `.set("LegalCopyright", "Copyright (C) 2026 Jonathas Conceicao. GPL-3.0-or-later.")`.

### IN-06: `verify-exe.sh` matches the version as a substring

**File:** `installer/verify-exe.sh:21-29`
**Issue:** `grep -F "ProductVersion0.7.0"` also matches an exe stamped `0.7.0-rc.1` or `0.7.01`.
In CI the exe is always freshly built after the Cargo gate, so the risk is low. As a gate,
though, it can pass on a stale exe (for example a local `FF_EXE`).
**Fix:** Check that the value ends right after the version. After the NULs are stripped, the next
byte is the next key's length/type word, so match that `$version` is followed by a non-version
character, for example `grep -a -E -q "ProductVersion${version//./\\.}([^0-9A-Za-z.+-]|$)"`.

### IN-07: The icon downscale uses straight alpha, and frame sizes are not validated

**File:** `tools/ff-art/src/main.rs:584-612`
**Issue:** The frames are resized with Lanczos3 on straight (demultiplied) RGBA. This bleeds the
colour of transparent pixels (black) into the edges, which leaves dark fringes in the small
16/24 px frames. Sizes of 0 or above 256 are not rejected up front: 0 makes a degenerate frame,
and above 256 fails inside `IcoFrame::as_png` with a vague error.
**Fix:** Resize the premultiplied pixmap data and demultiply afterwards, and
`anyhow::ensure!((1..=256).contains(&s), ...)` for each size.

### IN-08: Silent installs abort whenever ff is still running

**File:** `installer/ferrisfilter.iss:97-99`
**Issue:** Under `/SUPPRESSMSGBOXES`, `SuppressibleMsgBox` returns its default `IDCANCEL`. A silent
upgrade over an ff waiting on the Unapplied changes popup therefore fails with the
PrepareToInstall message and never retries. This may be intended, but nothing documents it.
**Fix:** Document it in the script header, or pass `IDRETRY` as the default once.

---

_Reviewed: 2026-10-18_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
