---
phase: 10-corrupted-condition-exceptional-gear
plan: "01"
subsystem: emitter/model
tags: [corrupted-condition, exceptional-style, schema, emitter, EXC-01, EXC-04]
dependency_graph:
  requires: []
  provides: [corrupted-condition-emit, exceptional-style-definition]
  affects: [src/models/config.rs, src/emitter.rs, config.json]
tech_stack:
  added: []
  patterns: [serde-option-bool-default, TDD-red-green, boolean-emit-no-operator]
key_files:
  created: []
  modified:
    - src/models/config.rs
    - src/emitter.rs
    - config.json
decisions:
  - "corrupted uses serde(default) + skip_serializing_if=Option::is_none for zero-cost backward compat"
  - "Corrupted emits as bare boolean (no == operator) matching PoE2 filter grammar"
  - "exceptional is a required field on Styles (modeled on unidentified, not optional craftable)"
  - "exceptional style def added to config.json in Plan 01 (enabling edit) — rules deferred to Plan 02"
metrics:
  duration_minutes: 3
  completed_date: "2026-06-24"
  tasks_completed: 2
  files_modified: 3
---

# Phase 10 Plan 01: Corrupted Condition + Exceptional Style Summary

**One-liner:** `corrupted: Option<bool>` on `When` with no-operator emit arm + required `exceptional` pink-Cross style added to `Styles` struct and `resolve_style`, with `config.json` style definition enabling parse.

## Tasks Completed

| Task | Name | Commit | Key Changes |
|------|------|--------|-------------|
| 1 | Add corrupted condition to When + emit arm (EXC-01) | ef6d1a1 | `When.corrupted: Option<bool>`, emit arm in `emit_rules`, 7 unit tests |
| 2 | Add exceptional named style (struct + resolver) (EXC-04) | cfd61a6 | `Styles.exceptional: Style`, `resolve_style` arm, `UnknownStyle` error update, `config.json` style def, 1 unit test |

## What Was Built

### Task 1 — Corrupted Condition (EXC-01)

Added `pub corrupted: Option<bool>` to `When` in `src/models/config.rs` with `#[serde(default, skip_serializing_if = "Option::is_none")]`. Placed after `sockets` and before `base_type` to match the deterministic emit order.

In `src/emitter.rs` `emit_rules`, added emit arm (step 8b) between Sockets and BaseType:
- `Some(true)` → `  Corrupted True`
- `Some(false)` → `  Corrupted False`
- `None` → nothing

No operator — correct PoE2 boolean syntax. Updated the deterministic-order doc comment to mention `corrupted` between sockets and baseType.

**Tests added:** 4 config tests (`when_corrupted_false_parses`, `when_corrupted_true_parses`, `when_corrupted_absent_is_none`, `when_corrupted_round_trips`) + 3 emitter tests (`emit_rules_emits_corrupted_false`, `emit_rules_emits_corrupted_true`, `emit_rules_omits_corrupted_when_none`).

### Task 2 — Exceptional Named Style (EXC-04)

Added `pub exceptional: Style` as a required field on `Styles` in `src/models/config.rs`, placed after `unidentified` and before the optional `craftable`. Includes doc comment explaining its pink Recycle-shape design and independence from `S`.

Updated the minimal test JSON in `config_with_rules_and_class_groups_parses` to include `"exceptional"` (required field — all deserializations of `Styles` need it).

In `src/emitter.rs`:
- Added `"exceptional" => &styles.exceptional` arm in `resolve_style` (after `unidentified`, before `craftable`)
- Updated `GenerateError::UnknownStyle` error message to list `exceptional` in the valid-styles set

Added `exceptional` style definition to `config.json` `styles` block (after `unidentified`):
- `text`/`border`: `[255, 121, 198]` (pink)
- `background`: `[40, 42, 54]` (Dracula)
- `font`: `null` (no override)
- `minimap`: `{ size: 2, color: "Pink", shape: "Cross" }`
- `beam`: `{ color: "Pink", temp: false }`
- `sound`: `null`
- `action`: `"Show"`

**Tests added:** `config_has_exceptional_style` — asserts `StyleAction::Show`, pink text/border, no sound, no font, Pink Cross minimap.

## Verification Results

- `cargo test` fully green: 131 lib tests + 13 CLI tests + 8 sync tests + 1 doc test
- `artifact_guard` passed — seed-universal.filter BYTE-IDENTICAL (no rule references `exceptional` or `corrupted` yet)
- `generate_snapshot` passed — snapshot unchanged
- `generate_block_counts` passed — 74 Show / 3 Hide unchanged

## Acceptance Criteria Check

- [x] `grep "pub corrupted: Option<bool>" src/models/config.rs` — line 335
- [x] `grep 'Corrupted {}' src/emitter.rs` — line 355, token is `True`/`False` (no `==`)
- [x] `cargo test --lib` passes
- [x] `cargo test --test cli_generate artifact_guard generate_snapshot` passes
- [x] `grep "pub exceptional: Style" src/models/config.rs` — line 141 (required field, not Option)
- [x] `grep '"exceptional" => &styles.exceptional' src/emitter.rs` — line 83
- [x] `exceptional` listed in UnknownStyle error message — line 23
- [x] `grep '"exceptional":' config.json` — Pink Cross, [255,121,198], bg [40,42,54], sound null, font null
- [x] `config_round_trips` passed
- [x] `config_parses_real_file` passed

## Deviations from Plan

None — plan executed exactly as written. The TDD RED/GREEN cycle was followed for Task 1. Task 2 did not require TDD (no `tdd="true"` attribute) and was implemented directly.

## Threat Surface Scan

No new network endpoints, auth paths, file access patterns, or schema changes at trust boundaries. The `exceptional` field is author-only config input, covered by `deny_unknown_fields` + `config_parses_real_file` (T-10-02 accepted).

The `Corrupted` emit arm is covered by unit tests asserting exact `Corrupted False`/`True` tokens with no `==` operator, preventing the silently-wrong `Corrupted == False` regression (T-10-01 mitigated).

## Self-Check: PASSED

- `src/models/config.rs` — modified and committed (ef6d1a1, cfd61a6)
- `src/emitter.rs` — modified and committed (ef6d1a1, cfd61a6)
- `config.json` — modified and committed (cfd61a6)
- Commits ef6d1a1 and cfd61a6 verified in git log
