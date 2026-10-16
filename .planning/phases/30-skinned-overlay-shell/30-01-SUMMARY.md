---
phase: 30-skinned-overlay-shell
plan: 01
subsystem: ui
tags: [egui, skin, win32, dpi, popup, icons]
requires: []
provides:
  - Monitor-aware DPI-correct docking (dock_rect_px, win::dock_left, target_monitor, monitor_scale, utc_offset_minutes)
  - Skin widget variants (ff_button_ex, ff_tabs_ex, ff_status_ex, header_plate_rect)
  - Global restyle of legacy widgets (apply_style) and overlay::theme re-pointed to the skin palette
  - skin::popup::ff_modal + PopupButton
  - skin::icons FlagIcon / paint_flag_icon / flag_icon
affects: [30-02, 30-03, 30-04, 30-05]
tech-stack:
  added: []
  patterns: [pure geometry fn + thin Win32 wrapper, wrapper keeps old signature for ff-proto]
key-files:
  created: [src/skin/popup.rs, src/skin/icons.rs]
  modified: [Cargo.toml, src/overlay/win.rs, src/skin/window.rs, src/skin/widgets.rs, src/skin/mod.rs, src/overlay/theme.rs]
decisions:
  - "No new crates: only windows-sys features Win32_Graphics_Gdi, Win32_UI_HiDpi, Win32_System_Time added; Cargo.lock unchanged"
  - "TIME_ZONE_ID_DAYLIGHT is not exported by windows-sys 0.61 Win32::System::Time, so utc_offset_minutes compares against 2 with a comment"
  - "ff_modal records button rects in egui temp data as a test hook"
metrics:
  tasks: 3
  completed: 2026-10-16
---

# Phase 30 Plan 01: Skin and Win32 groundwork Summary

Everything the shell needs on the skin and Win32 side now exists and is tested, with the running overlay untouched apart from the global style constants it reads.

## What was built
- **Docking:** `skin::window::dock_rect_px` (pure, tested at 100/150/200%, negative and non-zero origins, narrow work area, left taskbar, bad scale). `overlay::win` gained `Monitor`, `target_monitor` (game's monitor, else primary), `monitor_work_area_px`, `monitor_scale` (GetDpiForMonitor), `dock_left` (SetWindowPos twice, SWP_NOACTIVATE, no show) and `utc_offset_minutes`, with non-Windows stubs. `primary_work_area_px` now uses the primary monitor's `rcWork`. `win::work_area` is left in place for Plan 02 to retire.
- **Widgets:** `ff_button_ex` (disabled/danger), `ff_tabs_ex` (per-tab tooltips, Left/Right when focused), `ff_status_ex` (clickable, pointing hand), `header_plate_rect`; the old functions are thin wrappers so ff-proto compiles unchanged.
- **Restyle:** `apply_style` sets text styles 14/15/17/21, spacing, tooltip/popup frame and no shadows; hard-coded `regular(16.0)` became 15. `overlay::theme` constants point at `skin::palette`, tests updated.
- **Popup and icons:** `ff_modal` (Modal backdrop alpha 160, 9-slice game box, game buttons, Escape/backdrop = last button) and six painted `FlagIcon`s with the 18x18 `flag_icon` widget.

## Commits
- a3f55bf feat(30-01): monitor-aware DPI-correct docking
- 8d539d3 feat(30-01): skin widget variants and global restyle
- 8010626 feat(30-01): skinned modal popup and painted flag icons

## Verification
- `cargo test`: all green (1106 unit tests plus integration suites); `tests/fixtures` byte-identical; `cargo fmt --all --check` clean; `cargo build --bins` OK.
- Clippy warnings (`cargo clippy -p ferrisfilter --all-targets`, count of `^warning` lines): baseline 130, end 128.

## Deviations from Plan
None of substance. The icon test checks the accessible label through the AccessKit tree; the modal tests click via recorded button rects.

## Known Stubs
None.

## Threat Flags
None. All Win32 calls check null handles / zero returns and fall back (T-30-01); `dock_left` passes only our own HWND with SWP_NOACTIVATE (T-30-02).

## Self-Check: PASSED
