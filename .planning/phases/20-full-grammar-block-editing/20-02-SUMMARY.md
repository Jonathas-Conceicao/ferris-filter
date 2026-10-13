---
phase: 20-full-grammar-block-editing
plan: 02
subsystem: overlay-grammar
tags: [rust, overlay, grammar, codec, parse-04]

requires:
  - phase: 16-filter-document
    provides: keyword table (KEYWORDS, ValueShape) and tokenizer
provides:
  - overlay::grammar (picker groups, operator choices/labels, slider ranges, add-defaults, Value codec)
  - overlay::classes::ClassCatalog (class groups sourced from config.json)
affects: [20-07, 20-08]

tech-stack:
  added: []
  patterns:
    - "Round-trip-or-raw decode: structured Value only if encode_tokens reproduces the input tokens"
    - "Current operator is always an operator choice (never normalized)"

key-files:
  created: [src/overlay/grammar.rs, src/overlay/classes.rs]
  modified: [src/overlay/mod.rs]

key-decisions:
  - "Tasks 1 and 2 share grammar.rs and landed in one commit (5e8f1bf)"
  - "Bool with current != / !: offer both `Is` (bare) and the existing `Is not` form so the user can flip back"
  - "Sockets/CountThenNames conditions use the numeric operator set; Enum and QuotedList use the list set"
  - "encode also rejects whitespace in Enum values, socket colours and named sounds (BadText)"
  - "decode treats empty quoted names as Raw so a structured control never has to emit an empty entry"
  - "Action groups chosen as Colours / Text / Map / Sound / Drop sound"

requirements-completed: []
requirements-progress: [EDIT-01, EDIT-02]

duration: ~20min
completed: 2026-10-13
---

# Phase 20 Plan 02: Presentation grammar and class catalogue Summary

**Egui-free grammar module (groups, operators, ranges, defaults, round-trip-or-raw value codec) plus a config.json-driven ClassCatalog.**

## Accomplishments
- `condition_groups()` covers all condition keywords exactly once (Other is computed); `action_groups()` covers all 14 actions.
- `operator_choices` offers only valid operators per value shape and always includes the file's current form (`Exactly (no operator)`, `Exactly (=)`, `Not (!)`).
- `decode`/`encode`/`encode_tokens` cover every ValueShape; the FilterBlade corpus test asserts every condition/action line round-trips and raw fallbacks stay under 5%.
- `encode` refuses empty values and any quote/control character (T-20-05).
- `ClassCatalog` groups WEAPONS / ARMOUR / JEWELLERY / FLASKS & CHARMS from the Config; a guard test forbids class literals in non-test source.

## Task Commits
1. Tasks 1-2 - grammar module and codec: 5e8f1bf
2. Task 3 - ClassCatalog: 55ee3df

## Deviations from Plan
None of substance. Tasks 1 and 2 were committed together because both live in one new file.

## Issues Encountered
None. `cargo fmt --check` clean, `cargo build --bins` warning-free, full `cargo test` green (540 lib tests), fixtures untouched.

## Self-Check: PASSED
