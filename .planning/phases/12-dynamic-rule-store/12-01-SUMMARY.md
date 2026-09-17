---
phase: 12-dynamic-rule-store
plan: 01
subsystem: core/model-emitter
tags: [rust, serde, emitter, additive-schema, refactor]

requires:
  - phase: 11-core-library-extraction
    provides: compile::compile — the function compile_as is added beside
provides:
  - Rule.enabled — a rule can be switched off without being deleted
  - disabled-rule suppression that preserves ordinal labels
  - FilterIdentity + generate_as/compile_as — caller-named filter header
  - dynamic_base.json cleaned for ordinal identity
affects: [12-02, 14-rule-editor]

tech-stack:
  added: []
  patterns:
    - "Delegate-with-default: keep the old function as a one-line call into the parameterized one, so the old path runs the same code rather than being believed equivalent"

key-files:
  created: []
  modified:
    - src/models/config.rs
    - src/emitter.rs
    - src/compile.rs
    - dynamic_base.json

key-decisions:
  - "Skip disabled rules with `continue` inside the loop, before resolve_style — filtering the iterator would renumber later rules"
  - "generate() keeps its exact signature and delegates, so the universal path runs unchanged code rather than equivalent code"
  - "Remove names from all 10 base rules, not just the empty ones — mixed name/index labelling defeats the traceability"

patterns-established:
  - "A guard test asserting old-API and new-API outputs are identical, as the standing signal that a parameterization moved the oracle"
  - "Additive Option field with serde(default, skip_serializing_if) + a test that the absent case never grows the key on serialize"

requirements-completed: [DYN-01, DYN-02, DYN-04, DYN-05]

duration: ~25min
completed: 2026-09-03
---

# Phase 12 Plan 01: Store Schema and Emit Semantics Summary

**A rule can now be switched off without being deleted, block labels track a rule's index in the store rather than its emitted position, and the filter header names its own source — with `universal.filter` byte-identical throughout.**

## Performance

- **Duration:** ~25 min
- **Tasks:** 3 completed
- **Files modified:** 4
- **Tests:** 176 → 189 (13 added, 0 failures)

## Tasks Completed

| Task | Name | Commit | Key Changes |
|------|------|--------|-------------|
| 1 | Rule.enabled and disabled-rule suppression | `e0f3a5f` | `enabled: Option<bool>`; `continue` in `emit_rules`; 9 tests |
| 2 | Parameterize the filter header | `7687d8e` | `FilterIdentity`, `generate_as`, `compile_as`; 4 tests |
| 3 | Clean dynamic_base.json | `806e580` | 10 `name` keys removed; `meta.name` → `dynamic` |

## What Was Built

### `Rule.enabled` (DYN-01)

`Option<bool>` with `serde(default, skip_serializing_if)`, following `When.corrupted`'s v0.3
precedent. Absent means enabled, so `config.json` and every fixture round-trip unchanged — with a
test asserting `enabled: None` never grows the key on serialize.

### Disabled-rule suppression, and why `continue` (DYN-02)

`emit_rules` skips a disabled rule with `continue` **inside** the loop, **before** `resolve_style`.
Both details are load-bearing:

- `enumerate()` still walks the whole array, so labels stay tied to a rule's index in the source
  file. Filtering the iterator would renumber later rules — disable rule 2 and rule 3's block
  silently becomes `rule 2`, breaking the trace from a filter block back to its position in the
  store. Covered by `disabling_a_rule_does_not_renumber_later_rules`.
- Skipping before style resolution means a disabled rule with an unknown style is **inert**, not a
  hard error. A rule the author switched off should not be able to fail generation. Covered by
  `disabled_rule_with_unknown_style_is_inert`.

### `FilterIdentity` (12-CONTEXT D-10)

`generate_as(config, priced, identity)` carries the body; `generate(config, priced)` is now a
one-line delegation with `FilterIdentity::default()` — the exact pair the header hardcoded. Its
signature is unchanged, so the universal path is not *believed* unaffected, it **runs the same
code**. `compile_as` mirrors it in `src/compile.rs`.

The header's remaining 8 lines are untouched for both paths (D-11).

### `dynamic_base.json` (DYN-05 verified, ordinal identity enabled)

All 10 `name` keys removed and `meta.name` set to `"dynamic"`. Five rules carried `"name": ""` —
key *present*, empty string — which is why the `unwrap_or("rule {idx}")` fallback never fired and
those blocks emitted `Show # ` with nothing after them. Data only; no rule's style or conditions
were touched.

## Verification Results

- **Full suite:** 189 passed, 0 failed (was 176).
- **`git diff --exit-code tests/ config.json` → clean.** No fixture, snapshot, test file or oracle
  input moved.
- **`universal.filter` byte-identical before and after: 27,245 bytes.** Generated hermetically from
  the pre-change binary and again after, then diffed — the check Phase 11's verification called for,
  since a green suite alone did not catch two rendering bugs last phase.
- **`dynamic_base.json` generates cleanly:** 25,454 bytes, policy blocks labelled `rule 0`…`rule 9`,
  zero empty labels.

## Acceptance Criteria Check

- [x] DYN-01: `enabled` parses, round-trips, and absent-serializes without the key
- [x] DYN-02: disabling a rule leaves later rules' index labels unchanged
- [x] DYN-04: verified — `When`'s all-`Option` shape already gives it; the sockets-only case is
      asserted end-to-end in plan 12-02
- [x] DYN-05: `dynamic_base.json` has zero `areaLevel` conditions (author-supplied that way)
- [x] `generate` and `generate_as(default)` produce identical output
- [x] Fixtures, snapshot, tests and `config.json` untouched

## Deviations from Plan

Two, both mechanical:

1. **Five existing `Rule` struct literals needed `enabled: None`.** `Rule` derives no `Default`, so
   adding a field breaks every literal. The plan did not call this out. Mechanical, no behavior
   change, and `cargo build` alone did not catch it — `cargo test --no-run` did, because the
   literals are all in test code.
2. **`generate`'s doc comment was orphaned onto `FilterIdentity`** by the insertion in task 2 and
   had to be moved onto `generate_as`. Caught by reading the file after the edit, not by the
   compiler.

## Issues Encountered

`perl -0pi` silently failed to patch the `Rule` literals because the files are CRLF and the pattern
used `\n`. `cargo build` then reported success — because it does not compile test code — which
briefly looked like the patch had worked. Re-doing it in node with explicit CRLF handling, and
checking with `cargo test --no-run` rather than `cargo build`, resolved it. Worth remembering: on
this repo, **`cargo build` is not a sufficient compile check when the edit touches tests.**

## Next Steps

Plan 12-02 (wave 2): `src/dynamic.rs` lifecycle (seed / reset / compile / atomic write), the CLI
subcommand, and `tests/dynamic_filter.rs`.
