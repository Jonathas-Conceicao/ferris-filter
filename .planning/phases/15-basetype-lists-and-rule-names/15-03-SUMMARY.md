---
phase: 15-basetype-lists-and-rule-names
plan: 03
subsystem: docs
tags: [documentation, overlay, manual-verification, egui]

# Dependency graph
requires:
  - phase: 15-basetype-lists-and-rule-names
    plan: 02
    provides: "render_name, render_classes' own off switch, and render_base_types in overlay::rule_editor — the interactive controls this plan documents and hands to the author for in-game verification"
provides:
  - "CLAUDE.md describing the shipped name/class-off-switch/base-type-list controls, with the Phase-14-superseded autosave claim corrected in both places it appeared"
  - "15-MANUAL-VERIFICATION.md — the four-item in-game checklist for the residue automation cannot reach, including both planner-call veto invitations"
affects: []

# Tech tracking
tech-stack:
  added: []
  patterns: []

key-files:
  created:
    - .planning/phases/15-basetype-lists-and-rule-names/15-MANUAL-VERIFICATION.md
  modified:
    - CLAUDE.md

key-decisions:
  - "EDIT-09's REQUIREMENTS.md bookkeeping was verified already correct (ticked checkbox, Complete traceability row, commit 864cbee predates this plan) — confirmed rather than re-touched, per the plan's own instruction not to go hunting for a change to make."
  - "CLAUDE.md's two corrected sentences avoid bold-markdown splitting the literal phrases the plan's acceptance-criteria greps require ('held in memory', 'Apply is the single writer') — written as plain text rather than partially bolded, so the grep and the prose agree."

requirements-completed: []

# Metrics
duration: 25min
completed: 2026-09-08
---

# Phase 15 Plan 03: BaseType Lists and Rule Names — Documentation and Manual Verification Summary

**CLAUDE.md corrected to describe Phase 14's Apply-is-the-single-writer model (replacing a stale autosave claim in two places) and Phase 15's three new controls; a four-item in-game checklist written for the residue automation cannot reach — checkpoint (Task 2) now pending the author's live run.**

## Performance

- **Duration:** 25 min (Task 1 only; Task 2 is the blocking checkpoint below)
- **Started:** 2026-09-08T (see commit `26bf94a` timestamp)
- **Completed:** Task 1 complete; Task 2 (live check) PENDING — awaiting the author
- **Tasks:** 1 of 2 complete (Task 2 is `checkpoint:human-verify`, gate="blocking")
- **Files modified:** 2 (CLAUDE.md, new 15-MANUAL-VERIFICATION.md)

## Accomplishments

- Corrected CLAUDE.md's two stale autosave claims — the `dynamic.json` repository-layout row and
  the overlay blockquote both said "autosaves every edit straight to this file" / "no separate
  save step and no unsaved-work risk," superseded by Phase 14's `6b7c46c` (edits held in memory,
  Apply the single writer that persists the store then compiles from it, Revert to discard).
  Both now describe the current model and name the accepted trade-off (unsaved edits do not
  survive the process).
- Documented Phase 15's three shipped controls in CLAUDE.md's overlay note: the optional rule
  name (row header `<index> <style> - <name>`, emitted as the block's `Show # <name>` comment,
  no auto-fill), the class off switch (hides the picker entirely, discards the selection, makes
  base-type-only rules possible), and the item-by-item base-type list (Enter/`+ ADD` commits,
  per-entry `Remove`, quoted-list paste-splitting, explicitly **not validated** against anything).
- Wrote `15-MANUAL-VERIFICATION.md` in Phase 14's checklist format: a before-you-start block
  (release build, windowed/windowed-fullscreen PoE2), and exactly four sections — the three
  toggles' felt behaviour (with both required veto invitations: the base-type control's own
  on/off switch, and the name-field-vs-base-type-field commit asymmetry), keyboard-only
  base-type entry, 460px layout with a long base name, and the end-to-end Change Bases round
  trip. All four `**Result:**` lines are left blank for the author.
- Verified `EDIT-09`'s `.planning/REQUIREMENTS.md` bookkeeping is already correct (ticked
  checkbox, `Complete` traceability row, from commit `864cbee` predating this plan) — did nothing
  to it, as instructed.
- Confirmed the oracle held: `git status --porcelain tests/fixtures tests/snapshots` empty,
  `git diff --stat src/emitter.rs src/models/config.rs` empty, `cargo test --lib` 306/0,
  full `cargo test` 346/0 (306 lib + 39 integration + 1 doctest) — unchanged from the 15-02
  baseline, as expected for a docs-only plan.

## Task Commits

Each task was committed atomically:

1. **Task 1: Document the two controls, correct the superseded autosave claim, and write the checklist** - `26bf94a` (docs)

Task 2 (`checkpoint:human-verify`, gate="blocking") is **not yet performed** — see below. No
plan-metadata commit has been made; this SUMMARY and any final `docs` commit are deferred until
the checkpoint resolves.

## Files Created/Modified

- `CLAUDE.md` - corrected the `dynamic.json` layout-table row and the overlay blockquote's
  stale autosave claim; added description of the name field, class off switch, and base-type
  list to the overlay note
- `.planning/phases/15-basetype-lists-and-rule-names/15-MANUAL-VERIFICATION.md` - new, four-item
  in-game checklist, `**Result:**` lines blank

## Decisions Made

- Wrote the "held in memory" / "Apply is the single writer" phrases as plain, unbolded text in
  CLAUDE.md rather than partially bolding them (e.g. `**Apply** is the single writer`), because
  the plan's own acceptance-criteria grep requires the literal substring intact; bolding only the
  first word would have split the phrase across a `**` boundary and failed the grep while reading
  fine to a human. Verified with the plan's exact grep commands before committing.
- Left `.planning/REQUIREMENTS.md` untouched. EDIT-09's checkbox and traceability row were
  already `[x]` / `Complete` from commit `864cbee`, predating this plan's drafting — the plan's
  own instruction was to verify, not re-fix, and to say so here rather than invent a change.
- Kept both planner-call veto invitations in the checklist's item 1 rather than folding them into
  a single note: the base-type control's own on/off toggle, and the name-field-vs-base-type-field
  commit asymmetry are two independent, separately reversible calls the plan required to be
  surfaced distinctly (per the plan's own "keep both" instruction).

## Deviations from Plan

None — plan executed exactly as written for Task 1. One correction was needed mid-task: an
initial draft of the two corrected CLAUDE.md sentences used partial bold markup (`**Apply** is
the single writer`) that broke the plan's literal-substring grep criteria; reworded to plain text
before committing. This is an editorial self-correction to satisfy the plan's own acceptance
criteria, not a deviation from any instruction.

## Issues Encountered

None beyond the bold-markup grep mismatch above, caught and fixed before committing.

## User Setup Required

**Task 2 is a blocking checkpoint requiring the author.** See "Checkpoint Status" below — this
is not an auth gate or external service setup, but the plan's own mandatory in-game verification
step, which no agent can perform.

## Checkpoint Status: PENDING

Task 2 (`checkpoint:human-verify`, `gate="blocking"`) has **not** been performed and none of its
`**Result:**` lines have been filled in. Per the plan's own instruction (and this project's
repudiation threat T-15-10), an agent must not simulate, infer, or mark this checkpoint passed —
it requires the author at a running Path of Exile 2 with the overlay's release build.

**What the author needs to do:**
1. Build `cargo build --release --bin ff`.
2. Launch Path of Exile 2 in **windowed** or **windowed-fullscreen** mode — exclusive fullscreen
   shows no stripe at all and is not detected (`PITFALLS.md` §2).
3. Work through `.planning/phases/15-basetype-lists-and-rule-names/15-MANUAL-VERIFICATION.md`'s
   four sections and fill in every `**Result:**` line, including both veto-invitation responses
   in item 1.
4. Report back "approved" or describe what looked or behaved wrong.

**EDIT-10 and EDIT-11 remain unticked in `.planning/REQUIREMENTS.md`** — they are not complete
until this live check passes, matching the plan's explicit instruction. `requirements-completed`
above is intentionally empty for the same reason.

## Next Phase Readiness

Not applicable yet — this plan itself is incomplete pending Task 2. Once the author's checklist
is fully filled in and reports "approved," a continuation agent should: record the results
verbatim into `15-MANUAL-VERIFICATION.md`, mark EDIT-10/EDIT-11 complete in
`.planning/REQUIREMENTS.md`, update this SUMMARY's `requirements-completed` field and Checkpoint
Status section, run the standard state-updates (`state.advance-plan`, `state.update-progress`,
`roadmap.update-plan-progress`, etc.), and make the final metadata commit.

## Test Results

- `cargo test --lib` — **306 passed, 0 failed** (unchanged from the 15-02 baseline — docs-only
  plan, no code touched).
- `cargo test` (full suite: lib + integration + doctests) — **346 passed, 0 failed** (306 lib +
  39 integration + 1 doctest).
- `git diff --stat src/emitter.rs src/models/config.rs` — empty.
- `git status --porcelain tests/fixtures tests/snapshots` — empty.
- All of Task 1's acceptance-criteria greps verified directly (see "Decisions Made" above for the
  one correction needed): stale-autosave greps return nothing; `Revert` count 2; `held in
  memory`/`Apply is the single writer` both present; `base type` count 4; `not validated` present;
  checklist has exactly 4 `## ` sections and 4 `**Result:**` lines; reload instruction and
  `windowed` both present; `Change Bases` present with a stated no-`Class`-line requirement;
  `click somewhere else`/`clicked away`/`commits` all present, naming both sides of the asymmetry.

## Self-Check: PENDING (Task 1 portion PASSED)

- FOUND: `.planning/phases/15-basetype-lists-and-rule-names/15-03-SUMMARY.md`
- FOUND: `CLAUDE.md`
- FOUND: `.planning/phases/15-basetype-lists-and-rule-names/15-MANUAL-VERIFICATION.md`
- FOUND: `26bf94a` (Task 1 commit)
- PENDING: Task 2 — no commit exists yet; checkpoint awaits the author

---
*Phase: 15-basetype-lists-and-rule-names*
*Task 1 completed: 2026-09-08*
*Plan status: PAUSED at Task 2 (blocking human-verify checkpoint)*
