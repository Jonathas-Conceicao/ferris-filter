---
phase: 24-uat-feedback-fixes
plan: 01
subsystem: overlay-grammar
tags: [keywords, grammar, sliders, uat]
requires: []
provides: [poe2-only-keyword-table, slider-range-constants, default-in-range-test]
affects: [src/filter/keywords.rs, src/overlay/grammar.rs, src/overlay/body.rs, src/dynamic.rs]
key-files:
  modified:
    - src/filter/keywords.rs
    - src/filter/tokenize.rs
    - src/overlay/grammar.rs
    - src/overlay/body.rs
    - src/dynamic.rs
decisions:
  - "Enum and BoolOrName shapes and their codec/UI arms are kept; codec tests use test-local Keyword constants"
  - "MirageMap kept unlabelled; POE1_ONLY and the picker suffix deleted"
requirements-completed: [UAT-09, UAT-10]
completed: 2026-10-13
---

# Phase 24 Plan 01: Grammar trim and slider defaults Summary

KEYWORDS trimmed from 80 to 55 by removing the 25 PoE1-only conditions, and every numeric default now provably sits inside its slider (GemLevel 65 -> 20, table-wide test).

## Tasks

1. Removed 25 keywords, `INFLUENCES`, `GEM_QUALITY`, `POE1_ONLY` and the "(PoE 1)" picker suffix; picker groups and slider/default arms cleaned. Commit 36f70e9.
2. Added `FONT_SIZE_RANGE`, `SOUND_ID_RANGE`, `VOLUME_RANGE`, `MINIMAP_SIZE_RANGE`; GemLevel default 20; test `every_numeric_default_is_inside_its_slider` (RED on GemLevel first); body.rs reads ranges from grammar. Commit 32f197a.
3. Tests `gate_ignores_removed_poe1_keyword`, `apply_edited_writes_a_removed_keyword_line` (dynamic.rs), `removed_poe1_keyword_is_a_raw_row` (body.rs). Commit in git log (test(24-01)).

## UAT-09 findings

- Parser: a removed keyword is unknown, so `parse.rs` emits `Severity::Warning` "unknown keyword `LinkedSockets` kept as-is" and stores the line as a `BodyLine { keyword: None }`; serialization is byte-identical.
- Gate: `blocking_errors` counts only `Severity::Error`, so the warning never blocks. `WorkingFilter` (rules.rs) feeds `first_blocking` and the footer gate from `blocking_errors`, so Apply is not refused. Rows collect warnings separately from errors.
- Row: shows as `EntryKind::Opaque` raw row, text unchanged; an idle expanded frame emits no actions.
- Apply: `apply_edited` writes `\tLinkedSockets 6` verbatim. Not blocked.
- Code changed in the gate/parser: none (behaviour already correct; tests added to lock it).

## Deviations from Plan

- [Rule 3] `transfigured_gem_any_and_named` renamed `bool_or_name_any_and_named`; `bool_or_name_edit` is keyword-independent so it is exercised with a `Corrupted False` entry rather than deleted.
- [Rule 3] The shared `ff-target` dir hit linker lock contention (LNK 1104) and served another worktree's stale test binary; the final full `cargo test` (805 lib tests plus integration, all passing, fmt clean) ran in a private target dir `ff-target-a4d3`.
- `config_view.rs` literals left alone as instructed. The minimap size slider has no literal in body.rs, so `MINIMAP_SIZE_RANGE` is used only by the test.

## Self-Check: PASSED
