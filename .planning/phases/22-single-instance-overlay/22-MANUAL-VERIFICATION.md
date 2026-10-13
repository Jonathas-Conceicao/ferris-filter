# Phase 22 - Single-Instance Overlay: Manual Verification

**Purpose:** the checks headless tests cannot see (the real stripe revealing itself, the tray, the
game in the foreground, Explorer double-click, process counts). Requirement INST-01 stays "Complete
(UAT pending)" until this script has been run.

**Before you start (do not skip):**
- Use a release build: `cargo build --release --bin ff` with
  `CARGO_TARGET_DIR=C:/Users/jonat/AppData/Local/Temp/ff-target`, then launch `ff.exe` from that
  target dir's `release` folder. Alternatively use two launches of the same debug build. Never mix
  debug and release for checks 1-6: they use different instance names by design.
- Path of Exile 2 in windowed or windowed-fullscreen mode (see `13-MANUAL-VERIFICATION.md`).
- Open Task Manager to count `ff.exe` processes, and watch the system tray.

Record Result as PASS / FAIL with a note for every step.

---

1. **Visible.** With the stripe shown, launch `ff` again.
   **Expected:** the stripe stays shown (not hidden: a second launch is a reveal, never a toggle),
   the second process is gone within about 2 s, one tray icon, one `ff.exe`. **Result:**

2. **Hidden to tray.** Hide the stripe with Ctrl+Shift+Z or its close button, then launch again.
   **Expected:** the stripe reappears within about a second and the second process exits. If it does
   not reappear until the mouse moves over where it was (or not at all), the hidden-window heartbeat
   is not running: the fix is a watcher thread that waits on the show event and calls
   `ctx.request_repaint()`. **Result:**

3. **Game foreground.** Focus Path of Exile 2, hide the stripe, launch `ff` again (shortcut or
   Win+R).
   **Expected:** the stripe reappears over the game; the reveal still happens although the hotkey's
   foreground gate would not apply; keyboard focus stays where Windows put it (the stripe itself
   steals none). **Result:**

4. **Double-click.** While `ff` is running (hidden to the tray), double-click the exe in Explorer
   (relies on Phase 18's path resolver).
   **Expected:** same as check 2, and no error dialog. **Result:**

5. **Repeated launches.** Launch 5 times in a row.
   **Expected:** exactly one stripe, one tray icon, one `ff.exe`; Ctrl+Shift+Z still toggles
   in-game; the tray tooltip shows no hotkey failure. **Result:**

6. **Crash recovery.** End the running `ff.exe` in Task Manager, then launch.
   **Expected:** a normal new stripe starts (the OS released the mutex; no stale lock). **Result:**

7. **Debug beside release.** With a release `ff` running, run `cargo run --bin ff`.
   **Expected:** a second, debug stripe opens (separate name, by design) and its tray tooltip
   reports Ctrl+Shift+Z unavailable; quit it from its tray. **Result:**

8. **Not responding (optional, hard to provoke).** If a launch ever shows "FerrisFilter is already
   running but not responding", note what the first instance was doing (for example still starting
   up, RESEARCH Pitfall 3).
   **Expected:** message box, OK closes it, no second stripe. **Result:**

Tray **Quit** remains the only way to exit the process.
