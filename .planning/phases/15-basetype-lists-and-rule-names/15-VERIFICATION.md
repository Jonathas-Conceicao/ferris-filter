---
phase: 15-basetype-lists-and-rule-names
phase_name: BaseType Lists and Rule Names
verified: 2026-09-17T00:00:00Z
status: passed
scores:
  success_criteria: 5/5
  requirements: 2/2
  plans_executed: 3/3
gaps: []
---

# Phase 15: BaseType Lists and Rule Names — Verification Report

**Phase Goal:** A rule can match on a raw `BaseType` list with no `Class` line at all — `class`
becomes an optional condition like every other, so turning it off both enables base-type-only rules
and makes a freshly added rule open clean. A rule can also carry an optional name, shown in the
editor's row header and emitted as its block comment.
**Verified:** 2026-09-17
**Status:** passed
**Re-verification:** No — initial verification, written after the fact

> **Why this was written late.** The same reason as Phase 14: all three plans completed and the
> four manual checks were approved 2026-09-09, but no VERIFICATION.md followed, and the session
> moved straight into a live-use fix stream. The v0.4 milestone audit flagged EDIT-10 and EDIT-11
> as orphaned. Evidence here is the four manual checks, the `15-UAT.md` session run 2026-09-17,
> the audit's cross-phase integration pass, and direct reads of the code.

## Goal Achievement

**Achieved, and confirmed by the thing that motivated it.** The author's hand-written
*Change Bases* rule — `Rarity == Normal` plus a `BaseType ==` list and **no** `Class` line — was
rebuilt entirely inside the overlay and landed in `dynamic.filter` from a running game (manual
check 4, 2026-09-09). That was the request as originally stated, and it is the criterion that
matters most, because it is the one that was impossible before this phase.

The phase is almost entirely overlay-side by design. `When::base_type` and `Rule::name` already
existed in the model and were already emitted; what was missing was any way to reach them from the
editor. Nothing in `src/emitter.rs` or `src/models/config.rs` was modified, which is what makes
criterion 5 hold trivially rather than by careful arrangement.

## Success Criteria

| # | Criterion | Result | Evidence |
|---|-----------|--------|----------|
| 1 | A rule with base types and no classes emits `BaseType ==` and **no** `Class` line — the *Change Bases* rule is expressible in the editor | ✓ SATISFIED | `edit::clear_class` collapses the list to `None`, and `None` emits no line; `an_emptied_base_type_list_emits_no_base_type_line` guards the inverse. Manual check 4: built in the editor, landed in the game |
| 2 | `class` has the same independent off switch as every other condition, and its picker hides when off | ✓ SATISFIED | `render_classes`' `circular_toggle` backed by `panel_open` (id-keyed ephemeral state, not derived from `when.class.is_some()` — a derived toggle flickers shut the frame after being switched on, since D-09 forbids pre-selecting a class). Hidden entirely, not greyed (D-07). Manual check 1 |
| 3 | A named rule emits `Show # <name>`; an unnamed one falls back to the derived `rule {idx}` | ✓ SATISFIED | `a_named_rule_emits_its_name_as_the_block_comment`; `set_name_on_whitespace_only_yields_none_so_the_derived_label_still_wins`. Existing stores carry no names and are unaffected |
| 4 | A name shows in the row header without becoming a second identity scheme | ✓ SATISFIED | `RuleId` has no `Serialize`/`Deserialize` derive anywhere and is dropped by `to_rules()`; expansion and confirm-delete state are keyed by it, never by index. Verified independently by the milestone audit's integration pass (PITFALLS §9, DYN-02) |
| 5 | `config.json → universal.filter` byte-for-byte unchanged; no fixture re-baselining | ✓ SATISFIED | `artifact_guard` + `generate_snapshot` pass; `git diff --name-only $(git merge-base main HEAD) HEAD` touches no file under `fixtures/` or `snapshots/` |

## Requirements Coverage

| Requirement | Source Plan | Description | Status | Evidence |
|---|---|---|---|---|
| EDIT-10 | 15-01, 15-02 | Base-type list entered one item at a time, with individual removal | ✓ SATISFIED | `parse_base_type_entry` / `add_base_type` / `remove_base_type` / `clear_base_type` with eight unit tests, including a pasted quoted list splitting into separate entries and an unknown name kept verbatim. Manual check 2: four names typed with only Enter between them |
| EDIT-11 | 15-01, 15-02 | Optional per-rule name, shown in the row header and emitted as the block comment | ✓ SATISFIED | `set_name` / `clear_name` + `row_header_text`; criterion 3's tests |

**No orphaned requirements** — REQUIREMENTS.md lists exactly EDIT-10 and EDIT-11 for Phase 15, both
claimed and both satisfied.

## Design decisions worth carrying forward

- **Base names are not validated against anything.** PoE2 itself warns on an unknown base when the
  filter loads, which is a better and more current oracle than any list this repo could keep in step
  across leagues. `parse_base_type_entry_keeps_an_unknown_name_verbatim` pins the choice so it reads
  as deliberate rather than missing.
- **`panel_open` (id-keyed ephemeral state) is the general answer to the toggle-flicker problem**,
  not a one-off for classes. Any future condition whose "on" state does not immediately write a
  value to the rule needs it.
- **`QUALITY_MAX = 30`, not 20.** Recorded in `15-MANUAL-VERIFICATION.md` as "a Claude call the
  author has not ruled on". Put to the author as test 2 of `15-UAT.md` on 2026-09-17 and passed
  without a request to change it, so **30 stands**: corrupted items exceed the normal 20 cap, and a
  filter capped at 20 could not describe them. No longer an open decision.

## Anti-Patterns Found

None. No `TODO`/`FIXME`/`XXX`/`HACK` markers in the files this phase touched, no stubbed handlers,
no placeholder strings on a render path.

## Test Results

Full suite in a scratch `CARGO_TARGET_DIR`: **328 unit + 36 integration + 1 doctest, 0 failures**,
including `artifact_guard` and `generate_snapshot`. Build warning-free.

Two of this phase's tests were verified by mutation rather than assumed effective: the summary
ordering test fails when sockets and quality are swapped, and the base-type row layout test fails
against the unbounded `with_layout` it replaced. An earlier attempt at that layout test measured
zero height under empty fonts and proved nothing — caught by its own control assertion, which is
why the surviving tests carry one.

## Manual Verification

Four in-game checks, all PASS, author-signed 2026-09-09 — see `15-MANUAL-VERIFICATION.md`.

That checklist honestly listed three changes it had *not* separately confirmed (summary ordering,
the sliders, and button chrome in rule bodies). All three were tested and passed in `15-UAT.md` on
2026-09-17: **3 passed, 0 issues**. Nothing in Phase 15's surface remains unconfirmed.

## Gaps Summary

**No gaps.** Every roadmap success criterion, both requirements, the milestone acceptance bar, and
the scope fence against `src/emitter.rs` / `src/models/config.rs` hold under direct inspection of
the code, the suite and the git history — not only the summaries' narrative.

The one item this phase leaves behind is not a gap in it: the class picker's option list comes from
the store's own `Config`, so an existing `dynamic.json` seeded before `jewelleryClasses` existed
parses with an empty list and silently omits the group (`edit.rs` has a test asserting exactly
that). The remedy today is `--reset`, which discards all rules. That is the milestone audit's
WARNING-2 and belongs to the store's upgrade story, which no phase in v0.4 owned.
