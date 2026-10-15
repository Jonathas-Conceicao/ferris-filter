---
phase: 27-uat-feedback-fixes-4
plan: 02
subsystem: overlay
tags: [overlay, conditions, defences, simple-defences, pure-logic, tdd]
requires: []
provides:
  - "conditions::DefenceSeg / zero_based / SimpleSet / simple_defences"
  - "conditions::DefencesKind / defences_kind (advanced marker)"
  - "conditions::SegWrite / seg_write"
  - "conditions::DEFENCE_COMBOS / DEFENCE_SHORT / combo_matches / lit_combos"
affects: [27-04]
tech-stack:
  added: []
  patterns: ["pure classifier + single-write helper, consumed by the egui card in 27-04"]
key-files:
  created: []
  modified: [src/overlay/conditions.rs]
decisions:
  - "The advanced marker is a plain bool argument to defences_kind; the rule list (27-04) owns storing and clearing it"
  - "seg_write treats Some(Any) like None (no line), so callers can pass a row's segment directly"
  - "A non-integer or multi-value defence line is not zero based (classifies as Defences), never a panic (T-27-04)"
metrics:
  duration: "~15 min"
  completed: 2026-10-15
  tasks: 2
  files: 1
requirements: [UAT4-03]
---

# Phase 27 Plan 02: Simple Defences pure logic Summary

Pure, egui-free logic for the Simple Defences card (UAT4-03): zero-based line classification
(bare/`=`/`==` 0 -> No, `> 0`/`>= 1` -> Has), Simple-vs-Defences classification with a per-rule
advanced marker for card stability, the single canonical write per segment change, and the
7-chip combination strip matching, tested across all 27 settings.

## What was built

- `DefenceSeg { Any, Has, No }`, `zero_based(&BodyEntry) -> Option<DefenceSeg>`.
- `SimpleSet { rows: [(DefenceSeg, Option<usize>); 3] }` with `settings()`, and
  `simple_defences(&[&BodyEntry]) -> Option<SimpleSet>`: None on any non-zero-based defence line or
  two lines for one type (Between pair included); no defence lines gives all Any.
- `DefencesKind { Simple(SimpleSet), Advanced }`, `defences_kind(entries, advanced)`: None without
  defence lines, Advanced when `advanced` is set or the set is not simple.
- `SegWrite { Nothing, Add(String), Replace(String), Remove }`, `seg_write(kw_name, current, want)`.
- `DEFENCE_SHORT = ["AR", "EV", "ES"]`, `DEFENCE_COMBOS` (AR, EV, ES, AR+ES, AR+EV, EV+ES, AR+EV+ES),
  `combo_matches`, `lit_combos`.
- Module doc comment extended with a Simple Defences bullet.

No existing item or signature changed (`card_plan`, `CardPlan`, `Part`, `present_entries` untouched).

## Commits

| Task | Gate | Commit | Message |
|------|------|--------|---------|
| 1 | RED | 8406a00 | test(27-02): add failing tests for Simple Defences classification |
| 1 | GREEN | 0bab60f | feat(27-02): classify defence lines as Simple Defences or Defences |
| 2 | RED | 6dd6bf8 | test(27-02): add failing tests for Simple Defences writes and combo strip |
| 2 | GREEN | d0eae2d | feat(27-02): Simple Defences segment writes and combination strip |

## Verification

- `cargo test --lib overlay::conditions`: 29 passed, 0 failed (13 new tests).
- `cargo build --lib`: no warnings (no `#[allow(dead_code)]` needed; items are `pub`).
- `cargo clippy --lib --tests`: nothing reported for conditions.rs. `cargo fmt --check` clean.
- Only `src/overlay/conditions.rs` changed; no generator change, fixtures untouched.

## TDD Gate Compliance

Both tasks have a `test(...)` commit (compile-failing RED) followed by a `feat(...)` commit (GREEN).
No refactor commits were needed.

## Deviations from Plan

None - plan executed exactly as written. (Extra test: lines written by `seg_write` classify back to
their segment through `zero_based`, guaranteeing a Simple card stays Simple.)

## Known Stubs

None. The items are not yet consumed by the UI; 27-04 wires them into body.rs / the rule list.

## Self-Check: PASSED

- FOUND: src/overlay/conditions.rs (pub fn zero_based, simple_defences, defences_kind, seg_write, lit_combos; pub const DEFENCE_COMBOS, DEFENCE_SHORT)
- FOUND commits: 8406a00, 0bab60f, 6dd6bf8, d0eae2d
