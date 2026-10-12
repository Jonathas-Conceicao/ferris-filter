# Phase 22: Single-Instance Overlay - Context

**Gathered:** 2026-10-12
**Status:** Ready for planning
**Mode:** Smart discuss (autonomous, all v0.5 phases discussed upfront)

<domain>
## Phase Boundary

Launching `ff` while it is already running brings up the existing stripe instead of opening a
second window. Requirement: INST-01. Promoted from backlog 999.2. It touches only the overlay
entry point (`ferrisfilter::overlay::launch()`) and startup.

</domain>

<decisions>
## Implementation Decisions

### Behaviour
- The second launch **shows** the running stripe (a reveal, never a toggle), including when it is
  hidden to the tray, and then exits.
- The reveal must bypass the hotkey's foreground-window gate, because the signal comes from
  outside the game and the stripe. A hidden egui window needs a repaint wake-up
  (`request_repaint`), or the signal isn't processed until the next input.
- If the first instance doesn't respond within about 2 seconds, the second shows a short message
  ("FerrisFilter is already running but not responding") and exits. It never opens a second
  stripe.
- After repeated launches there is exactly one stripe and one working hotkey.

### Mechanism (from research; planner may refine)
- A Windows named mutex in the `Local\` namespace, held for all of `main` and checked with
  `GetLastError` immediately. Plus a named event the first instance waits on.
- It uses the existing `windows-sys` dependency with the extra features
  `Win32_System_Threading`/`Win32_Security`. No new crate and no lock files.
- The check runs before the hotkey and tray are created.

### Debug vs release
- Debug builds use a **separate instance name**, so a `cargo run` build can run beside a release
  `ff`.

### Claude's Discretion
- Watcher thread vs per-frame event polling, the GUID in the name, and the message-box API.

</decisions>

<code_context>
## Existing Code Insights

### Reusable Assets
- `src/overlay/tray.rs` `OverlayCommand` (show/hide path), `src/overlay/visibility.rs`,
  `src/overlay/hotkey.rs` (`HotkeyStatus`, which already reports registration conflicts in the
  tray tooltip, D-10), and `src/overlay/win.rs` (HWND wrapper).

### Established Patterns
- Phase 13 decisions: the stripe is shown without stealing focus, the ✕ only hides it, and tray
  Quit is the only exit.

### Integration Points
- The overlay entry point / `ff` binary main, before eframe starts.

</code_context>

<specifics>
## Specific Ideas

- Manual test matrix: launch twice while visible, while hidden, and while the game is
  foreground. Double-click the exe (relies on Phase 18's path resolver).

</specifics>

<deferred>
## Deferred Ideas

None. The discussion stayed within the phase scope.

</deferred>
