---
status: complete
phase: 15-basetype-lists-and-rule-names
source: 15-01-SUMMARY.md, 15-02-SUMMARY.md, 15-03-SUMMARY.md
started: 2026-09-17T00:00:00Z
updated: 2026-09-17T00:00:00Z
---

## Scope note

Phase 15's four manual checks are **already approved** (author, 2026-09-09 — the three toggles and
two text-field commit behaviours, keyboard-only base-type entry, layout at 460px, and the
*Change Bases* rule built end to end). **They are not re-asked here.**

This session tests exactly the three items `15-MANUAL-VERIFICATION.md` lists under *"Not separately
confirmed"* — changes that landed after that checklist was written, were on screen while items 1–4
ran, but were never given a recorded check of their own. Nothing else in Phase 15's surface has
changed since: `git log --since=2026-09-09 -- src/overlay/{rule_editor,edit,view_model}.rs` returns
a single commit, `6ba59af` (Jewels in the picker), already covered as test 8 of the Phase 14 UAT.

One of the three carries an **open decision the author has never ruled on** — see test 2.

## Current Test

[testing complete]

## Tests

### 1. Collapsed summary reads in body order
expected: A collapsed rule's summary runs in the same order as its expanded body (classes, bases, rarity, defences, sockets, quality, corrupted, unidentified tier), and a rule with no classes leads with `Bases:`. Landed in `6786622`; pinned by `the_summary_order_mirrors_the_editor_body_order` and verified by mutation.
result: pass

### 2. Sockets and quality are sliders with the right ranges
expected: Sockets and quality render as sliders like unidentified tier, ranging 1-6 and 1-30 respectively; neither starts at 0, because `>= 0` matches every item that has ever dropped. Landed in `6786622`. Carries an open decision — the quality ceiling of 30 rather than 20 was a Claude call the author has not ruled on.
result: pass
note: >-
  The open decision on QUALITY_MAX was put to the author with this test and passed without a
  request to change it, so **30 stands** and is no longer an unruled Claude call. It remains one
  constant in edit.rs if that is ever revisited.

### 3. Button chrome inside an expanded rule body
expected: The style dropdown, class-group headers, and base-type `+ ADD` / `Remove` controls all render with real button chrome rather than bare text. Fix in `d2a52ed`: `app.rs`'s row-header scope blanks `weak_bg_fill`/`bg_stroke` so the header frame stays invisible, and the body rendered inside that scope inherited it; `render_rule_body` now restores the palette.
result: pass

## Summary

total: 3
passed: 3
issues: 0
pending: 0
skipped: 0
blocked: 0

## Gaps

[none yet]
