---
phase: 42-release-review
fixed_at: 2026-10-18T00:00:00Z
review_path: .planning/phases/42-release-review/42-REVIEW.md
iteration: 1
findings_in_scope: 1
fixed: 1
skipped: 0
status: all_fixed
---

# Phase 42: Code Review Fix Report

**Fixed at:** 2026-10-18
**Source review:** .planning/phases/42-release-review/42-REVIEW.md
**Iteration:** 1

**Summary:**
- Findings in scope: 1 (critical_warning); the one Info item was also fixed, as allowed
- Fixed: 1 (+1 Info)
- Skipped: 0

## Fixed Issues

### WR-01: `--quit` comment suggests the events are only opened, but `CreateEventW` creates them when they are missing

**Files modified:** `src/overlay/mod.rs`
**Commit:** 74dc99f
**Applied fix:** Rewrote the `--quit` comment in `launch()`: it never creates the instance
mutex (so it cannot become the running instance), and the `-ready` / `-quit` events are opened
through CreateEventW, which creates a fresh event if the instance vanished in between. That
case is detected (no ERROR_ALREADY_EXISTS) and reported as Unreachable. Checked against
`quit_running_named` (instance.rs:656-662). Comment only; `cargo fmt --check` passes.

### IN-01: The "Optimise dependencies" comment is now separated from the section it describes

**Files modified:** `Cargo.toml`
**Commit:** aa0eaa5
**Applied fix:** Removed the blank line between the comment and `[profile.dev.package."*"]`.
`cargo metadata` still parses the manifest.

---

_Fixed: 2026-10-18_
_Fixer: Claude (gsd-code-fixer)_
_Iteration: 1_

## Iteration 2

Re-review after the fixes: status clean (0 critical, 0 warning, 0 info). The first-round review is kept as 42-REVIEW.iter1.md.
