---
phase: 35
fixed_at: 2026-10-17
review_path: .planning/phases/35-prototype-retirement-docs/35-REVIEW.md
iteration: 1
findings_in_scope: 1
fixed: 1
skipped: 0
status: all_fixed
---

# Phase 35: Code Review Fix Report

**Scope:** IN-02 only (IN-01 deliberately skipped by request).

## Fixed Issues

### IN-02: "stripe" vocabulary survives in many doc comments

**Status:** fixed
**Commit:** 27679ba
**Files modified:** `src/bin/ff.rs`, `src/overlay/{app,apply,chat,economy,instance,layout,mod,tray,visibility,win}.rs`
**Applied fix:** 97 comment lines reworded ("the stripe" -> "the window", "overlay stripe" -> "overlay window", "stripe's" -> "overlay's"/"window's"). Comments only.

**Deliberately left unchanged (not comments):**
- User-visible strings in `src/overlay/instance.rs` (lines ~110 and ~121, the "not responding" and "did not start" message boxes) still say "stripe". Reported, not changed.
- Test function names (`hiding_the_stripe_...`, `a_failed_sync_surfaces_the_reason_in_the_stripe`, `docking_into_a_work_area_keeps_the_stripe_...`, `a_taskbar_on_the_left_shifts_the_stripe_...`, `stripe_is_never_screen_sized_...`), test assertion messages in `economy.rs` and `layout.rs`, and the `STRIPE_WIDTH` constant.
- Unrelated "stripes" meaning zebra rows in `src/skin/widgets.rs` and `src/overlay/style_lines.rs`.

## Gate results

- `cargo fmt --check`: clean
- `cargo test` (CARGO_TARGET_DIR outside repo): all suites pass (1389 unit tests, 0 failed, plus integration suites)

---

_Fixed: 2026-10-17_
_Fixer: Claude (gsd-code-fixer)_
