---
phase: 20-full-grammar-block-editing
plan: 04
subsystem: styled-rules
tags: [rust, filter, styles]

requires:
  - phase: 20-01
    provides: splice_body, set_block_keyword, meta_get, EditError
provides:
  - emitter::style_action_lines (single definition of a style's action lines)
  - styled::{StyleReport, rewrite_styled, rewrite_block, style_name_ok, rule_index_of}
affects: [20-06, 21]

key-files:
  created: [src/styled.rs]
  modified: [src/emitter.rs, src/lib.rs]

key-decisions:
  - "styled.rs is a top-level module because filter must not depend on models::config"
  - "Semantic comparison (lowercased keyword + value tokens, sorted multiset, plus Show/Hide) makes the rewrite idempotent"
  - "New action lines are inserted where the first old action line was, or after the last body line when none existed"

requirements-completed: []
requirements-progress: [EDIT-06, STYLE-03]

completed: 2026-10-13
---

# Phase 20 Plan 04: Styled-rule rewrite Summary

**`style_action_lines` is now the one source of a style's actions (shared by `generate`), and `styled::rewrite_styled` rewrites every enabled, non-economy rule tagged `#ff:style <known>` idempotently, reporting unknown names.**

## Accomplishments
- Emitter refactor leaves `artifact_guard` and `generate_snapshot` byte-identical.
- 12 styled tests cover keyword switch, append, idempotency, unknown style, disabled/economy skips, unknown-keyword lines, mixed EOLs, `unchanged` style, `style_name_ok`, `rule_index_of`.
- Full `cargo test` green (571 lib tests), fmt clean, bins warning-free, fixtures untouched.

## Task Commits
1. Task 1 - extract style_action_lines: f604b54
2. Task 2 - styled.rs rewrite: see git log (`feat(20-04)`)

## Deviations from Plan
- Plan's example listed `MinimapIcon 0 Pink Circle`; the committed S style uses size 1, so the test asserts the real file value.

## Self-Check: PASSED
