---
phase: 14-rule-editor-apply
phase_name: Rule Editor + Apply
verified: 2026-09-17T00:00:00Z
status: passed
scores:
  success_criteria: 8/8
  requirements: 9/9
  plans_executed: 4/4
gaps: []
---

# Phase 14: Rule Editor + Apply — Verification Report

**Phase Goal:** Every dynamic rule is editable from the overlay — style, classes, rarity,
defences, sockets, quality, corrupted, each condition independently togglable — and Apply writes
`dynamic.filter` and tells the player to reload it in-game.
**Verified:** 2026-09-17
**Status:** passed
**Re-verification:** No — initial verification, written after the fact

> **Why this was written late.** Phase 14's four plans completed 2026-09-08 and its twelve manual
> in-game checks were all approved the same day, but no VERIFICATION.md was produced, and Phase 15
> began immediately. The v0.4 milestone audit (2026-09-17) flagged EDIT-01..09 as orphaned as a
> result — present in the traceability table, absent from every verification. This report closes
> that, and is written against evidence gathered *after* the fact rather than at the time: the
> twelve manual checks, the `14-UAT.md` session run 2026-09-17, the milestone audit's own
> cross-phase integration pass, and direct reads of the shipped code.

## Goal Achievement

**Achieved.** Every condition named in the goal has a control in the expanded rule body, and each
one is independently switchable. Apply writes `dynamic.filter` through the same
`dynamic::compile_store` / `dynamic::write_filter` pair the CLI's own `apply` subcommand uses —
`write_filter` has exactly two call sites crate-wide (`run.rs:177`, `app.rs:507`) and nothing else
in the tree writes those bytes — and the confirmation names the file and states the reload path
verbatim.

One deliberate reversal is recorded rather than smoothed over: **D-02 was reversed during the
phase** (2026-09-08, author request after live use). The phase was planned around autosave;
what shipped holds edits in memory with Apply as the single writer and a Revert to discard. The
superseded decision is preserved in `14-CONTEXT.md`. Manual check 5 was recorded against the
autosave model and then re-run as check 8 against the replacement, which is the authoritative
result for that behaviour.

## Success Criteria

| # | Criterion | Result | Evidence |
|---|-----------|--------|----------|
| 1 | Add/delete/move a rule; moving changes which of two overlapping rules wins | ✓ SATISFIED | `tests/rule_editor.rs::move_up_changes_which_style_wins_for_overlapping_rules` — asserts the emitted filter, not just the array |
| 2 | Disabling one flask/charm rule removes flask drops — the motivating example, one click | ✓ SATISFIED | `tests/rule_editor.rs::disabling_one_flask_rule_removes_the_flask_block_from_the_compiled_filter`; manual check 6 ("the milestone motivating example works by hand, end to end") |
| 3 | Style dropdown offers exactly `config.json`'s styles; picking one changes the emitted actions | ✓ SATISFIED | `edit::available_styles(&Styles)` is derived from the config, never a literal; the emitter's `resolve_style` rejects anything outside the closed set at generate time |
| 4 | Class picker options come from `config.json`'s class lists — no code change to add one | ✓ SATISFIED | `edit::class_groups(&Config)` (PITFALLS §10). Demonstrated end-to-end on 2026-09-16: `Jewels` was added to `jewelleryClasses` as data only and appeared in the picker (`6ba59af`; `14-UAT.md` test 8 PASS) |
| 5 | Armour + ES ticked, evasion unticked emits `BaseArmour > 0`, `BaseEnergyShield > 0`, `BaseEvasion 0`; all seven armour types reachable; group off emits no defence lines | ✓ SATISFIED | `edit`'s defence set/clear tests plus the emitter's `min: 1 → > 0` rule; manual check 6 exercised the combination by hand |
| 6 | Turning any individual condition off removes exactly its line, leaving the others untouched | ✓ SATISFIED | Every `when` field is `Option` with `skip_serializing_if`; `edit::clear_*` collapses to `None`, and `None` emits nothing. Covered per-condition in `overlay::edit`'s unit tests |
| 7 | Apply writes `dynamic.filter` and states the reload path | ✓ SATISFIED | `apply::RELOAD_INSTRUCTION` is a named constant asserted verbatim; manual check 4 confirmed the game picks the filter up after Options > Game > Item Filter > Refresh |
| 8 | Mutation/validation/compilation are tested library functions; the egui layer holds no click-only logic | ✓ SATISFIED | `overlay::edit`, `overlay::view_model`, `overlay::visibility` and `overlay::apply` are framework-free and unit-tested; `rule_editor.rs`/`app.rs` hold presentation only (PITFALLS §8, D-18) |

## Requirements Coverage

| Requirement | Source Plan | Description | Status | Evidence |
|---|---|---|---|---|
| EDIT-01 | 14-01, 14-02 | Lists rules in ordinal order; add/delete/reorder | ✓ SATISFIED | `WorkingRules` + deferred `RowAction`; criterion 1's test |
| EDIT-02 | 14-01, 14-02 | Enable/disable suppresses a category without deleting | ✓ SATISFIED | `Rule.enabled` + emitter skip preserving ordinal labels; criterion 2's test |
| EDIT-03 | 14-03 | Style selector populated from `config.json` | ✓ SATISFIED | `edit::available_styles`; criterion 3 |
| EDIT-04 | 14-03 | Class picker sourced from the config's class lists | ✓ SATISFIED | `edit::class_groups`; criterion 4, demonstrated by the `Jewels` addition |
| EDIT-05 | 14-03 | Rarity selector emitting the compact comparator forms | ✓ SATISFIED | `RarityComparator` round-trip through serde into `Rarity op X` |
| EDIT-06 | 14-03 | Three-checkbox defence picker | ✓ SATISFIED | Criterion 5 |
| EDIT-07 | 14-03 | Sockets minimum, quality minimum, corrupted control | ✓ SATISFIED | Criterion 6; both numeric controls became bounded sliders in Phase 15 |
| EDIT-08 | 14-03 | Every condition control individually togglable | ✓ SATISFIED | Criterion 6; the off-switch inventory covers seven conditions, and Phase 15 added the eighth (class) |
| EDIT-09 | 14-04 | Apply writes the filter and tells the player to reload | ✓ SATISFIED | Criterion 7. **See the note below** — a defect on this requirement's error path was found and fixed after the phase closed |

### EDIT-09's error path — found late, fixed before the milestone closed

The v0.4 milestone audit found that Apply reported success after a *failed* store save
(BLOCKER-1): `app.rs` recorded the error and fell through with no early return, so the compile read
the previous store off disk, the write succeeded, and the success branch advanced the Revert
baseline to edits that had never been persisted. The happy path — what the twelve manual checks and
criterion 7 exercised — was always correct; the error path inverted the contract.

Fixed in `b613676` via `apply::guard_save`, a pure function in the framework-free module, with the
compile nested inside its `Ok` arm so it is structurally unreachable after a failed save. Four
tests, two of which fail against the original behaviour (verified by injecting it). EDIT-09 is
recorded as satisfied because it is satisfied *now*; the sequence is recorded because a reader
should know this was caught by audit rather than by the phase's own verification.

## Anti-Patterns Found

None blocking. `grep` for `TODO`/`FIXME`/`XXX`/`HACK` across `src/overlay/` returns no markers. Two
items of genuine debt, neither a stub:

- **Stale in-source doc comments contradict the shipped design.** `store.rs:5-9` still describes
  `save` as "called once per frame from the editor's autosave (D-02)"; `app.rs:23`, `:428-430` and
  `:121-124` likewise still speak of autosave. D-02 was reversed in 14-04. `15-03-SUMMARY` claims
  this wording was corrected "in both places it appeared" — that was `CLAUDE.md`; the source
  comments were missed. A maintainer reading `store.rs` would believe a second writer exists.
- **`dynamic.json` cosmetic round-trip quirks** (explicit `eq`/`min` null keys; `HashMap` key
  reshuffling) — documented in `CLAUDE.md` as accepted and asserted value-equal by `store`'s own
  round-trip test.

## Test Results

Full suite, run in a scratch `CARGO_TARGET_DIR` (a running `ff.exe` locks `target\debug\ff.exe`):
**328 unit + 36 integration + 1 doctest, 0 failures**, including `artifact_guard` and
`generate_snapshot` with no fixture re-baselining. Build is warning-free.

## Manual Verification

Twelve in-game checks across two passes, all PASS, author-signed 2026-09-08 — see
`14-MANUAL-VERIFICATION.md`, which records "Nothing outstanding. All twelve checks across both
passes are approved."

A second session, `14-UAT.md` (2026-09-17), covered the surface that changed *after* those checks —
the live-use fix stream — with 7 passed and 1 issue.

## Gaps Summary

**No gaps against this phase's success criteria or requirements.**

One blocker was found by the UAT session, but it belongs to CORE-02 and the overlay's path
resolution, not to Phase 14's criteria: launching `ff` by double-clicking the built exe fails,
because `store.rs:81-84` resolves every path from the process working directory, which for a
GUI-subsystem binary started from Explorer is the exe's own folder. It is recorded in `14-UAT.md`
with a full diagnosis and carried as a v0.4 milestone gap. Phase 14 neither introduced it nor was
scoped to fix it — the overlay inherited the CLI's working-directory convention in Phase 13.
