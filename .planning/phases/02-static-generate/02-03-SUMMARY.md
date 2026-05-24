---
phase: 02-static-generate
plan: 03
subsystem: acceptance-tests
tags: [rust, insta, snapshot-testing, artifact-guard, determinism, crlf, poe2]

# Dependency graph
requires:
  - phase: 02-static-generate/02-02
    provides: "src/emitter.rs pub fn generate, regenerated universal.filter (golden master)"

provides:
  - "tests/cli_generate.rs: generate_snapshot (insta D-05), artifact_guard (D-06), generate_is_deterministic (D-07)"
  - "tests/snapshots/cli_generate__generate_snapshot.snap: committed regression oracle"
  - ".planning/phases/02-static-generate/config.diff: human-reviewable config.json old→refactored"
  - ".planning/phases/02-static-generate/diffs/universal_filter.diff: human-reviewable filter old→regenerated"

affects: [phase-3-price-driven-tiering, future-ci]

# Tech tracking
tech-stack:
  added:
    - "insta 1.47.2 (dev-dep, already added in 02-01)"
  patterns:
    - "CRLF-safe snapshot/guard: normalize(s.replace(\"\\r\\n\", \"\\n\")) before compare on both sides"
    - "Approach A: drive binary via assert_cmd + --output to temp path; read + normalize + snapshot/compare"
    - "Insta snapshot file-based oracle: tests/snapshots/cli_generate__generate_snapshot.snap"
    - "Artifact-guard: committed universal.filter must == generator output (CRLF-normalized)"

key-files:
  created:
    - tests/snapshots/cli_generate__generate_snapshot.snap
    - .planning/phases/02-static-generate/config.diff
  modified:
    - tests/cli_generate.rs

key-decisions:
  - "Approach A chosen (binary-driven via assert_cmd): consistent with existing test conventions, clean separation"
  - "CRLF normalization applied on both sides of every string comparison and before snapshotting"
  - "INSTA_UPDATE=new + rename .snap.new to .snap used to accept snapshot (cargo-insta CLI not installed)"
  - "config.diff: git diff 31d1d7b..HEAD -- config.json; shows only the tier-C beam bug fix (one change)"
  - "universal_filter.diff: already committed by 02-02; kept as-is (recycle reorder + header source-indicator)"

requirements:
  - GEN-01

# Metrics
duration: 20min
completed: 2026-05-24
---

# Phase 02 Plan 03: Acceptance Tests + D-14 Diffs Summary

**Insta snapshot, CRLF-safe artifact-guard, and determinism tests locking the generator output; two human-reviewable D-14 diffs produced for in-game equivalence gate (D-13)**

## Performance

- **Duration:** ~20 min
- **Started:** 2026-05-24
- **Completed:** 2026-05-24
- **Tasks:** 2 automated + 1 checkpoint (human-verify, not yet reached)
- **Files modified:** 2 (tests/cli_generate.rs, tests/snapshots/ created)

## Accomplishments

- Extended `tests/cli_generate.rs` with three CRLF-safe acceptance tests (all pass on Windows):
  - `generate_snapshot`: insta regression oracle locked to current generator output (D-05)
  - `artifact_guard`: asserts committed `universal.filter` == generator output (D-06)
  - `generate_is_deterministic`: two runs produce byte-identical output (D-07 / GEN-01)
- `normalize()` helper (`s.replace("\r\n", "\n")`) applied on every string before compare/snapshot
- Committed `tests/snapshots/cli_generate__generate_snapshot.snap` — the regression oracle
- All 43 tests pass (32 unit + 4 cli_errors + 7 cli_generate)
- Produced `.planning/phases/02-static-generate/config.diff` (13 lines — only tier-C beam fix)
- `diffs/universal_filter.diff` already committed by plan 02-02; reused as-is (contains recycle reorder)

## Task Commits

1. **Task 1: Snapshot + artifact-guard + determinism tests** — `ea76f2b` (test)
2. **Task 2: D-14 diff artifacts** — `d6eb888` (docs)

## Files Created/Modified

- `tests/cli_generate.rs` — Extended with 3 new tests + `normalize()` helper (7 total tests)
- `tests/snapshots/cli_generate__generate_snapshot.snap` — Committed insta oracle (848-line filter)
- `.planning/phases/02-static-generate/config.diff` — config.json old→refactored (tier-C beam fix only)
- `.planning/phases/02-static-generate/diffs/universal_filter.diff` — Reused from 02-02 (recycle reorder + header)

## Human-Verification Checkpoint Details

### What to review

Two diffs are ready for inspection:

1. **`.planning/phases/02-static-generate/config.diff`** (13 lines)
   - Only change: tier-C `"beam": null` → `"beam": { "color": "Yellow", "temp": false }`
   - This is the Rule 1 bug fix from plan 02-02: the original filter has `PlayEffect Yellow` on all
     tier-C blocks but config.json had `beam: null`. Fixed to match original behavior.
   - No other structural changes.

2. **`.planning/phases/02-static-generate/diffs/universal_filter.diff`** (26 lines)
   - Header: `filter-model.json` → `config.json` (source-indicator update, D-02 minor cleanup)
   - Recycle block: action order normalized per D-03 (SetTextColor/SetBorderColor/SetBackgroundColor/
     MinimapIcon/PlayEffect → PlayEffect/MinimapIcon/SetTextColor/SetBackgroundColor/SetBorderColor)
   - No block added/removed, no conditions, BaseType lists, colors, or numeric values changed

### In-game smoke test (D-13)

To validate behavioral equivalence, copy the regenerated filter to the PoE2 config folder and test:

```bat
install.bat
```
or copy manually to:
`C:\Users\jonat\OneDrive\Documentos\My Games\Path of Exile 2\universal.filter`

**Verification checklist:**
- [ ] Currency highlights/sounds correct (S=pink/uwu, A=purple/uwu, B=red/beep, C=yellow/beep, D=white beam/no sound, E=white small, F=white smaller)
- [ ] Gear hides correctly at endgame zone (only build weapons Rare + pure ES armour Magic+Rare show)
- [ ] Recycle items (sockets >= 1 AND quality >= 1) appear green
- [ ] Uniques/jewels/gems fall through to native game style (no filter block touching them)

Once in-game equivalence is confirmed, type "approved" to complete the phase gate.

## Decisions Made

- CRLF normalization: `s.replace("\r\n", "\n")` applied on ALL string comparisons/snapshots
  — the working-tree `universal.filter` is CRLF on Windows (`.gitattributes text` + `core.autocrlf=true`)
  while `writeln!` emits LF-only; normalizing both sides makes tests pass on both Windows and Linux
- Approach A (binary-driven): run `ferrisfilter generate --output <tmp>` via `assert_cmd`, read output,
  normalize, then snapshot/compare — consistent with existing `tests/cli_generate.rs` pattern
- config.diff baseline: `31d1d7b` (the commit immediately before this phase's first implementation
  commit, recorded in the plan) — shows only data changes introduced during Phase 2

## Deviations from Plan

None — plan executed exactly as written. The `universal_filter.diff` was already produced by plan
02-02 and reused; no regeneration needed since `universal.filter` has not changed since then.

## Known Stubs

None — all three acceptance tests are fully wired; diffs are human-readable and complete.

## Threat Flags

None — no new network endpoints, auth paths, file access patterns, or schema changes. The threat
mitigations T-02-06 (artifact-guard prevents silent drift) and T-02-07 (D-14 diffs provide
reviewable record) are now implemented.

## Self-Check: PASSED

| Item | Status |
|------|--------|
| `tests/cli_generate.rs` | FOUND |
| `tests/snapshots/cli_generate__generate_snapshot.snap` | FOUND |
| `.planning/phases/02-static-generate/config.diff` | FOUND |
| `.planning/phases/02-static-generate/diffs/universal_filter.diff` | FOUND |
| Commit `ea76f2b` (test: snapshot + artifact-guard + determinism) | FOUND |
| Commit `d6eb888` (docs: D-14 diff artifacts) | FOUND |
