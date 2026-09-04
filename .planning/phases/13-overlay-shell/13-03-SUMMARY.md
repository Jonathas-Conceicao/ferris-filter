---
phase: 13-overlay-shell
plan: 03
subsystem: ui
tags: [win32, windows_sys, egui, eframe, focus-management, state-machine, rust]

# Dependency graph
requires:
  - phase: 13-02
    provides: "tray icon, global Shift+Z hotkey, and the shared toggle_requested event-delivery flag feeding OverlayApp's frame loop"
provides:
  - "src/overlay/visibility.rs — pure, framework-free Hidden/ShownUnfocused/ShownFocused state machine (advance + effect_for) with full #[test] coverage, including the Pitfall-2 never-focused guard"
  - "src/overlay/win.rs — cached HWND + raw ShowWindow(SW_SHOWNOACTIVATE)/ShowWindow(SW_HIDE) reveal/conceal, the only module in the crate touching windows_sys"
  - "OverlayApp wired end to end: one visibility::advance call per frame drives win::reveal/win::conceal, ViewportCommand::Visible is never used post-creation"
  - "Live author verification on real hardware that repeated Shift+Z summons never steal keyboard focus from another application (closes RESEARCH Open Question 2 with a negative finding)"
affects: [13-04, 13-05, 14-rule-editor]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Pure decision / impure effect split: visibility::advance (state transition) and visibility::effect_for (transition -> single side effect) are both framework-free and unit-tested; app.rs contains no show/hide branching of its own, only 'advance, ask for effect, perform it'"
    - "All windows_sys / raw Win32 calls confined to one module (src/overlay/win.rs) with exactly two public functions (reveal, conceal), enforced by a grep criterion in acceptance criteria"
    - "Raw ShowWindow on a self-cached HWND replaces ViewportCommand::Visible entirely (never mixed) because winit 0.30.13's MARKER_ACTIVATE latch makes with_active(false) only suppress activation on the first reveal, not subsequent ones"
    - "ShownUnfocused as a distinct third state (not just Hidden/Shown) exists solely to prevent a window revealed via SW_SHOWNOACTIVATE — which has never been focused — from reading focused == false on its very next frame as 'user clicked away' and re-hiding itself instantly"

key-files:
  created:
    - src/overlay/visibility.rs
    - src/overlay/win.rs
  modified:
    - src/overlay/app.rs
    - src/overlay/mod.rs

key-decisions:
  - "Task 3's checkpoint was cleared via live author verification on real hardware (not simulated) — all eight script steps passed, including the load-bearing third/fourth repeated-summon case that a single-summon test would not have caught, and several hide/reveal cycles with no stale or blank repaints observed (closes RESEARCH Open Question 2 negatively: the raw-HWND bypass does not desync winit's internal state)"
  - "Deferred to 13-04 (not fixed here): the header's close control renders as an empty tofu box because ui.button(\"\\u{2715}\") (U+2715 MULTIPLICATION X) has no glyph in egui's default font. Substituting a different Unicode glyph is not a durable fix because the planned Cinzel display face (all-caps) also lacks it — the correct fix is painting two strokes with egui::Painter, which is 13-04's territory (it owns the header plate and custom painting)."

requirements-completed: [SHELL-02]

# Metrics
duration: ~20min (this continuation session — Task 3 checkpoint evidence recording and SUMMARY only; Tasks 1-2 implementation ran in the prior session)
completed: 2026-09-04
---

# Phase 13 Plan 03: Overlay Shell — Focus-Safe Show/Hide Summary

**A cached-HWND raw `ShowWindow(SW_SHOWNOACTIVATE)`/`ShowWindow(SW_HIDE)` reveal/conceal pair, driven by a unit-tested three-state visibility machine, replaces `ViewportCommand::Visible` entirely and is confirmed on real hardware not to steal keyboard focus on repeated summons.**

## Performance

- **Tasks:** 3 (2 auto/TDD + 1 checkpoint)
- **Files modified:** 4 (2 created, 2 modified)
- **Commits:** 3 task commits (RED, GREEN, and the raw-HWND wiring) — Task 3 is a verification-only checkpoint with no code change, same pattern as 13-01 and 13-02's Task 3

## Accomplishments

- Built `src/overlay/visibility.rs`: `Visibility` (`Hidden` / `ShownUnfocused` / `ShownFocused`), `Effect` (`Reveal` / `Conceal`), `advance()` implementing every transition in the plan's `<behavior>` block, and `effect_for()` mapping a transition to the single side effect the caller performs. Framework-free — no `egui`/`eframe`/`winit`/`windows_sys` symbol appears in the file. 12 `#[test]` functions, including the Pitfall-2 guard (twenty consecutive unfocused frames leave `ShownUnfocused` unchanged).
- Built `src/overlay/win.rs`: `OverlayWindow` caches a `windows_sys::Win32::Foundation::HWND` obtained once via `from_frame(&eframe::Frame)` matching `RawWindowHandle::Win32`; `reveal()`/`conceal()` call `ShowWindow` with `SW_SHOWNOACTIVATE`/`SW_HIDE`. Non-Windows stub arm compiles with the same public signatures as no-ops. This is the only module in the crate referencing `windows_sys`.
- Wired both into `OverlayApp::ui()`: one `visibility::advance(self.visibility, focused, toggle_requested)` call per frame, followed by one `visibility::effect_for(...)` dispatch to `win::reveal`/`win::conceal`. No show/hide branching logic lives in `app.rs` itself.
- Removed plan 13-02's temporary `toggles: N` scaffolding label; the tray "Show" command now only sets `toggle_requested` while currently `Hidden` (a no-op while already shown, per D-17); the header `✕` sets `visibility = Hidden` and calls `win::conceal` directly, never `ViewportCommand::Close` (D-14 — only tray Quit exits).
- **Task 3 checkpoint cleared by the author on real hardware.** See table below.

## Task 3: Live Verification (checkpoint — cleared by the author)

The author built and ran `cargo run --bin ff` and executed the eight-step script by hand with Notepad focused and typing throughout, per the plan's `<how-to-verify>`.

| Verification step | Result |
|---|---|
| Stripe hides and summons repeatedly via `Shift+Z` — multiple cycles, not one | Working |
| Keyboard focus stays with Notepad across **every** summon (steps 3 and 4 — the load-bearing case; a single-summon test would have passed even with the winit `MARKER_ACTIVATE` defect present) | Working — focus never stolen on the second, third, or fourth summon |
| Clicking inside the stripe activates it and moves focus to it (D-06, D-08) | Working |
| Clicking away afterwards auto-hides the stripe on focus loss | Working |
| Stripe never flashes-and-vanishes on summon (the never-focused / Pitfall 2 guard) | Working — no misbehavior observed |
| Repaint after several hide/reveal cycles (RESEARCH Open Question 2) | Working — no stale or blank frames reported |
| Header `✕` hides without exiting; tray icon remains | Working |
| Tray → Quit exits the process | Working |

This closes RESEARCH Open Question 2 (whether the raw-HWND bypass desyncs winit's internal visibility/window state) with a **negative finding** on real hardware: no repaint or resize misbehavior was observed after repeated hide/reveal cycles.

**One defect the author found — recorded here, not fixed in this plan:** the header's close control renders as an empty tofu box instead of `✕`. Root cause: `src/overlay/app.rs` calls `ui.button("\u{2715}")` (U+2715 MULTIPLICATION X), and egui's default font ships no glyph for that codepoint. This is deliberately deferred to **13-04**, which owns the header plate and custom painting — substituting a different Unicode glyph would not be durable because the planned Cinzel display face is an all-caps font that also lacks U+2715; painting two strokes with `egui::Painter` is the correct fix and belongs with 13-04's custom-painting work, not this plan's focus-safety scope.

## Task Commits

1. **Task 1 (RED): failing tests for the visibility state machine** - `62efe25` (test)
2. **Task 1 (GREEN): implement the pure visibility state machine** - `acd723b` (feat)
3. **Task 2: raw-HWND reveal/conceal wired into the update loop** - `c6f9c03` (feat — mislabeled, see Deviations)
4. **Task 3: live verification checkpoint** - no code change (verification-only, same pattern as 13-01/13-02)

**Plan metadata:** committed as part of this SUMMARY's own commit.

## Files Created/Modified

- `src/overlay/visibility.rs` - Pure `Visibility`/`Effect` state machine (`advance`, `effect_for`), 12 unit tests
- `src/overlay/win.rs` - Cached-HWND raw `ShowWindow` reveal/conceal, the crate's sole `windows_sys` call site
- `src/overlay/app.rs` - Frame loop now drives `visibility::advance` → `effect_for` → `win::reveal`/`win::conceal`; `toggles: N` scaffolding removed; header `✕` wired to hide-only
- `src/overlay/mod.rs` - Declares the new `visibility` and `win` modules

## Decisions Made

- `ShownUnfocused` kept as a distinct third state rather than collapsing to a boolean — this is the entire mechanism that prevents the never-focused hazard (Pitfall 2), and the twenty-frame test proves it holds under sustained polling, not just a single frame.
- Both reveal and conceal go through the same raw `ShowWindow` path so there is exactly one source of truth for window visibility; `ViewportCommand::Visible` is never called after window creation, per RESEARCH's anti-pattern warning against mixing the two mechanisms.
- `WS_EX_NOACTIVATE` was deliberately not set — `SW_SHOWNOACTIVATE` is scoped to the one reveal call, which is what lets a subsequent click still activate the stripe normally (D-06) without extra code.

## Deviations from Plan

### Auto-fixed Issues

None — Tasks 1 and 2 were executed as written in the prior session; this session only completed the Task 3 checkpoint and produced this SUMMARY.

### Labeling Note (not a code deviation)

**1. Commit `c6f9c03` is tagged `feat(13-02):` but belongs to plan 13-03**
- **Found during:** Continuation session, reviewing `git log` before resuming Task 3
- **Issue:** The commit subject reads `feat(13-02): raw-HWND reveal/conceal wired into the update loop`, but this work is plan 13-03's Task 2 (`src/overlay/win.rs`, `src/overlay/visibility.rs`'s consumer, the `toggle_requested` wiring) — a copy/paste labeling typo from the prior plan number.
- **Fix:** Not amended. The commit is no longer `HEAD` (two later commits exist on top of it: `9a3e4a9` docs and the current session's work), so fixing it would require a rebase, which risks disturbing already-reviewed history for no functional benefit.
- **Impact:** Any GSD tooling that greps commit subjects by plan ID (resume-gate matching, `roadmap update-plan-progress` plan counting) will attribute `c6f9c03` to plan 13-02's commit count rather than 13-03's. This is a cosmetic bookkeeping skew only — the file contents and task boundaries are correct; only the label is wrong.
- **Committed in:** N/A — informational only, recorded here per the resume instructions.

---

**Total deviations:** 0 auto-fixed; 1 labeling note (informational, not fixed).
**Impact on plan:** None on correctness or scope. The mislabel is a bookkeeping artifact for future plan-progress tooling to be aware of.

## Issues Encountered

None beyond the two items already recorded above (the deferred `✕` tofu-box glyph defect, and the `c6f9c03` commit mislabel) — both are documented, neither blocks this plan's success criteria.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- SHELL-02 now holds behaviorally end to end: the hotkey/tray summon the stripe while another application keeps keyboard focus, on every summon and not just the first, confirmed live on real hardware.
- The `visibility` and `win` modules are stable building blocks 13-04 (header styling, Cinzel font, custom-painted `✕`) and 13-05 (in-game verification) can build on without further changes to the show/hide mechanism itself.
- One known cosmetic defect carries forward to 13-04: the header `✕` renders as a tofu box (missing glyph) and needs to become custom-painted strokes rather than a Unicode character, since the planned Cinzel face won't contain U+2715 either.
- No single-instance guard exists (carried decision from 13-02, still true here) — out of scope for SHELL-02.

---
*Phase: 13-overlay-shell*
*Completed: 2026-09-04*
