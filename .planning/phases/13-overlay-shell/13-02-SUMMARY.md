---
phase: 13-overlay-shell
plan: 02
subsystem: ui
tags: [tray-icon, global-hotkey, egui, eframe, windows, rust]

# Dependency graph
requires:
  - phase: 13-01
    provides: "ff GUI binary, OverlayApp/eframe::App scaffold, the docked stripe viewport, and the update-loop layer boundary (pure logic vs. presentation)"
provides:
  - "src/overlay/tray.rs — tray icon, Show/Quit menu, pure command_for_menu id->OverlayCommand mapping (D-16, D-17, D-18)"
  - "src/overlay/hotkey.rs — system-wide Shift+Z registration with HotkeyStatus as a value, never a panic (D-09, D-10)"
  - "assets/ferris.png + assets/ASSETS.md — licensed tray icon asset with provenance recorded for every binary asset in assets/"
  - "Proven event delivery: both the tray and the hotkey reach OverlayApp::update()/ui() through one shared toggle_requested flag, confirmed live on the author's machine while another application held keyboard focus"
affects: [13-03, 13-04, 13-05, 14-rule-editor]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Tray and hotkey are both constructed inside eframe::run_native's app-creation closure on the main thread (RESEARCH Pattern 3's thread-affinity constraint) — never on a spawned thread, verified by grep across src/overlay/"
    - "Registration/construction failures are values (HotkeyStatus::Failed(String)), never unwrap/expect on external state — the caller (tray tooltip) presents, the library reports"
    - "One shared toggle_requested boolean flag consumed once per frame, fed by two independent input sources (tray left-click, Shift+Z Pressed event) — isolates event-delivery correctness from the (later) visibility mechanism"
    - "HotKeyState::Pressed filter on GlobalHotKeyEvent — without it, press+release both fire and double the toggle count"

key-files:
  created:
    - src/overlay/tray.rs
    - src/overlay/hotkey.rs
    - assets/ferris.png
    - assets/ASSETS.md
  modified:
    - src/overlay/mod.rs
    - src/overlay/app.rs

key-decisions:
  - "Task 3's checkpoint was cleared via live author verification on real hardware (not simulated/automated) — all seven steps passed, including the double-fire check (exactly one toggle per Shift+Z press) and the second-instance registration-failure tooltip"
  - "No single-instance guard exists or is planned for Phase 13 — a second `ff` opens a second stripe window. Observed during Task 3 verification; explicitly out of scope for SHELL-02 (no requirement covers it); recorded as a deferred idea, not implemented here"

requirements-completed: [SHELL-02]

# Metrics
duration: ~10min (this continuation session — Task 3 checkpoint review and SUMMARY only; Tasks 1-2 implementation ran in the prior session)
completed: 2026-09-04
---

# Phase 13 Plan 02: Overlay Shell — Tray Icon, Global Hotkey, Event Delivery Summary

**A Ferris tray icon with a working Show/Quit menu and a system-wide `Shift+Z` hotkey both deliver toggle events into `OverlayApp`'s frame loop through one shared flag, confirmed live by the author on real hardware while another application held keyboard focus.**

## Performance

- **Tasks:** 3 (2 auto + 1 checkpoint)
- **Files modified:** 6 (4 created, 2 modified)
- **Commits:** 2 task commits (Task 3 is a verification-only checkpoint, no code change — same pattern as 13-01's Task 1)

## Accomplishments

- Built `src/overlay/tray.rs`: `load_icon()` embeds `assets/ferris.png` via `include_bytes!` (no runtime disk read), `build()` constructs the `TrayIcon` + Show/Quit `Menu` on the main thread, and `command_for_menu()` is a pure, unit-tested id→`OverlayCommand` mapping.
- Sourced and licensed `assets/ferris.png` (rustacean.net's "Original Ferris") and created `assets/ASSETS.md` recording provenance for both `ferris.png` and the pre-existing `uwu.mp3`.
- Built `src/overlay/hotkey.rs`: `register_toggle()` registers `Shift+Z` via `global-hotkey`, returning `(Option<GlobalHotKeyManager>, HotkeyStatus)` — `HotkeyStatus::Failed(String)` on any failure, never `unwrap`/`expect` on external registration state.
- Wired both into `OverlayApp::update`/`ui()`: tray click, tray menu, and hotkey events are polled once per frame and funneled into one `toggle_requested` flag, rendered as a temporary `toggles: N` label (removed in 13-03). `Quit` is the sole `ViewportCommand::Close` call site (D-14).
- Surfaced hotkey registration failure through the tray tooltip and a disabled menu line (D-10) — verified live: a second `ff` instance's tray tooltip reported `Shift+Z` already unavailable instead of the process dying silently.
- **Task 3 checkpoint cleared by the author on real hardware** (see table below) — all seven verification steps passed, including the double-fire check that proves the `HotKeyState::Pressed` filter is correct.

## Task 3: Live Verification (checkpoint — cleared by the author)

The author ran `cargo run --bin ff` and executed the seven-step script by hand. Results:

| Verification step | Result |
|---|---|
| Tray icon appears; right-click menu shows Show / Quit | Working |
| Left-click tray toggles the counter | Working |
| `Shift+Z` with focus in Notepad increments the counter | Working — by exactly one per press, confirming the `HotKeyState::Pressed` filter is correct and release events are not double-firing |
| Typing continues to reach Notepad while the hotkey fires | Confirmed — focus stayed with the other window |
| Tray tooltip reads `FerrisFilter — Shift+Z to toggle` | Confirmed |
| Second instance reports the hotkey unavailable in its tray tooltip | Confirmed — tooltip reported the key was already registered (D-10: registration failure must be visible, never silent) |
| Tray → Quit exits the process and removes the icon | Confirmed (author quit both instances this way) |

No steps failed; no partial pass. Per the plan's checkpoint instruction ("Do not proceed to plan 13-03 on a partial pass"), this is a clean, full pass and the phase may proceed.

**Observation, not a defect:** launching a second `ff` opens a second stripe window — there is no single-instance guard. This is out of scope for Phase 13 (no SHELL-02 requirement covers process singleton behavior); not implemented here. Recorded under Issues Encountered below and tracked by the orchestrator as a deferred idea.

## Task Commits

1. **Task 1: Tray icon, menu, and the Ferris asset** — `f9acd3f` (feat)
2. **Task 2: Global Shift+Z hotkey with a visible registration failure** — `c3ca847` (feat)
3. **Task 3: Verify tray and hotkey event delivery** — checkpoint, no commit (cleared by the author; verification-only, matching 13-01's Task 1 pattern)

## Files Created/Modified

- `src/overlay/tray.rs` — `load_icon()`, `OverlayCommand`, `TrayHandles`, `build()`, `command_for_menu()`
- `src/overlay/hotkey.rs` — `TOGGLE_MODIFIERS`/`TOGGLE_CODE` constants, `HotkeyStatus`, `register_toggle()`
- `assets/ferris.png` — licensed tray icon (rustacean.net, public domain / CC0)
- `assets/ASSETS.md` — provenance/license table for `ferris.png` and `uwu.mp3`
- `src/overlay/mod.rs` — `launch()` builds the tray and hotkey on the main thread before `eframe::run_native`, derives the tooltip/status line from `HotkeyStatus`
- `src/overlay/app.rs` — `OverlayApp` owns `TrayHandles` + `Option<GlobalHotKeyManager>` + `HotkeyStatus`; polls hotkey/tray-click/tray-menu events each frame into `toggle_requested`; renders the temporary `toggles: N` counter; `Quit` sends `ViewportCommand::Close`

## Decisions Made

- Task 3's live checkpoint was verified by the author on real hardware rather than simulated — this is the only reliable way to prove global input delivery while another application holds focus, and matches the plan's explicit instruction not to proceed on a partial pass.
- No single-instance guard was added; it is out of scope for SHELL-02 and is being tracked separately as a deferred idea rather than folded into this plan's scope.

## Deviations from Plan

None — plan executed exactly as written. Task 3 required no code changes (no `<files>` were specified for it in the plan, consistent with it being a pure verification checkpoint); the only action taken in this continuation session was recording the author's verification results and writing this SUMMARY.

## Issues Encountered

- **Single-instance behavior (observation, not a defect):** a second `ff` process opens a second, independent stripe window rather than focusing/toggling the first. Discovered during Task 3's live verification. No requirement in this phase covers process singleton behavior; not fixed here. The hotkey registration failure is still correctly surfaced in the second instance's tray tooltip (D-10 holds), so the second instance is not silently broken — it is simply a second, functioning overlay with its own hotkey attempt.

## User Setup Required

None — no external service configuration required.

## Next Phase Readiness

- SHELL-02's input half is proven end-to-end: tray and system-wide `Shift+Z` both reach `OverlayApp`'s frame loop while another application holds focus.
- The temporary `toggle_count`/`toggles: N` label is explicitly scaffolding for 13-03 to remove once the real show/hide mechanism lands.
- No blockers. The deferred single-instance-guard idea is the only open thread, and it does not block 13-03 (which owns show/hide, not process lifecycle).

---
*Phase: 13-overlay-shell*
*Completed: 2026-09-04*
