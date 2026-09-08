---
phase: 14-rule-editor-apply
plan: 04
subsystem: overlay
tags: [rust, egui, immediate-mode-gui, filter-editor, apply]

# Dependency graph
requires:
  - phase: 14-rule-editor-apply
    plan: 03
    provides: the full condition editor body (render_rule_body) whose edits Apply must land in the game
  - phase: 14-rule-editor-apply
    plan: 02
    provides: the accordion, WorkingRules/RuleId, the dirty flag, and overlay::store::save
  - phase: 12-dynamic-filter-store
    plan: 02
    provides: dynamic::compile_store and dynamic::write_filter, built print-free and fully parameterised for exactly this second caller
provides:
  - "src/overlay/apply.rs — the pure apply-outcome layer: ApplyState, ApplyOutcome, success_message, indicator; no egui, no filesystem"
  - "the APPLY and REVERT affordances in app.rs, the unapplied-changes indicator, and the in-memory edit model that replaced autosave"
  - ".planning/phases/14-rule-editor-apply/14-MANUAL-VERIFICATION.md — 11 in-game checks across two passes, all PASS"
affects: []

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Apply is the single writer: it persists the store and THEN compiles from it. The order is load-bearing — compile_store reads the store from disk, so compiling first reintroduces the stale-Apply off-by-one through a different route."
    - "The pure/impure split holds for Apply as it did for the editor: apply.rs decides what Apply reports and what state it leaves behind; app.rs does the I/O. The state machine is unit-tested without a window (PITFALLS.md section 8)."
    - "Failures surface in the stripe, never only on stderr — the release binary runs under the Windows GUI subsystem with no console (D-17)."

key-files:
  created:
    - src/overlay/apply.rs
    - .planning/phases/14-rule-editor-apply/14-MANUAL-VERIFICATION.md
  modified:
    - src/overlay/app.rs
    - src/overlay/mod.rs
    - src/overlay/view_model.rs
    - src/overlay/edit.rs
    - src/overlay/rule_editor.rs
    - src/overlay/theme.rs
    - src/models/config.rs
    - config.json
    - dynamic_base.json
    - CLAUDE.md
    - .planning/phases/14-rule-editor-apply/14-CONTEXT.md

key-decisions:
  - "D-02 REVERSED after live use: edits are held in memory and Apply is the only writer, with a Revert to discard. The author asked for it directly; the original autosave rationale (no unsaved-work risk from closing the overlay) is preserved in 14-CONTEXT.md as superseded, along with the accepted trade-off — unsaved edits now die with the process, and what replaces the protection is visibility plus deliberate discard."
  - "Revert clears the expanded row and any pending delete confirm, because rebuilding the working model from the baseline mints fresh RuleIds and a retained expansion would point at a rule that no longer exists."
  - "A fully-selected class group collapses to 'Any <group>', not 'All <group>' — a summary reads as the rule's own matching test, and any one class in the list satisfies it (author, reversing the first cut the same day)."
  - "The unidentified-tier control is a 1..=5 slider, not an unbounded numeric entry: PoE2 never assigns a tier outside that range, and the first cut's 0..=10 let the editor express filters the game can never match."
  - "The `unidentified` style is renamed `kite`, after its minimap icon. The old name described the rules that happened to use it rather than what it does, which read as nonsense when the picker offered it for an unrelated rule."
  - "Group membership for summaries is read from config.json's own class lists, never a hardcoded literal (PITFALLS.md section 10) — so adding a class to the config correctly stops an existing rule reading as 'Any <group>' until that class is selected too."

requirements-completed: [EDIT-09]

# Metrics
duration: implementation ~90min; the live-use fix stream spanned 2026-09-07..08
completed: 2026-09-08
---

# Phase 14 Plan 04: Apply, the Indicator, and the Live-Use Fix Stream Summary

**Apply compiles the edited store and lands `dynamic.filter` in the game folder, says how to make
the game read it, says out loud when it fails, and never leaves the player guessing whether what
they just changed is actually in the game — plus the eleven-commit stream of fixes that came back
from the author actually using the editor against a running game.**

## Performance

- **Duration:** implementation ~90 min; the live-use fix stream spanned two days
- **Tasks:** 3 completed (2 auto, 1 blocking human-verify checkpoint)
- **Files modified:** 15 across the plan and its follow-on stream (2 created)
- **Tests:** 317 passing, 0 failing. `config.json` to `universal.filter` is byte-for-byte unchanged
  across the entire plan — `artifact_guard` and `generate_snapshot` were never re-baselined, which
  was the author's non-negotiable acceptance bar for the whole milestone.

## Accomplishments

### The plan as written

- **`src/overlay/apply.rs`** — the pure layer: `ApplyState` (`Applied`/`Unapplied`), `ApplyOutcome`,
  `success_message`, and the indicator wording. No `egui`, no filesystem, fully unit-tested
  without a window.
- **Apply (EDIT-09/D-15)** reuses Phase 12's `dynamic::compile_store` and `dynamic::write_filter`
  completely unchanged — the second caller those functions were built print-free and
  parameterised for, a full phase before it existed.
- **The reload instruction (D-16)** is part of the success message, not a nearby label: PoE2 never
  re-reads a filter on its own (`PITFALLS.md` section 1), so a successful Apply that does not say
  *Options > Game > Item Filter > Refresh* is a half-finished action.
- **Failures surface in the stripe (D-17)** — the release binary runs under the Windows GUI
  subsystem with no console, so an error written only to stderr is an error the player never sees.
- **The unapplied-changes indicator (D-03)** states divergence in words, not colour alone.
- **`CLAUDE.md`** documents the editor and Apply as they actually shipped, including the two
  cosmetic `dynamic.json` round-trip quirks that are expected and not defects.
- **`14-MANUAL-VERIFICATION.md`** — the in-game record for what only a human at a running game can
  judge, deliberately narrow because most of this phase's correctness lives in the library.

### The live-use fix stream

Eleven commits landed after the checkpoint, each from the author using the editor against a live
game. They are part of what shipped and are recorded here so the phase record matches reality:

- **`3375ad4` — Apply wrote the previous frame's state.** The working model was projected into the
  config at frame top, *before* the row mutations ran, so a create-then-Apply, delete-then-Apply
  cycle showed the created rule only on the second Apply. Fixed by re-projecting immediately
  before writing. The author's original report is check 7 in the manual record.
- **`6b7c46c` — Revert, and the reversal of D-02.** See `key-decisions`.
- **`c9b62b9` — the `kite` rename, and the tofu at its root.** The Apply confirmation rendered
  placeholder boxes where its arrows should have been — the same defect class as Phase 13's tofu
  close glyph, and the same root cause: `FontDefinitions`' `entry().or_default()` creates an
  *empty* family, so Cinzel had no fallback chain at all. Fixed in `theme::install_fonts` by
  seeding the display family with the default proportional chain before inserting Cinzel at the
  front.
- **`35b695e` — a jewellery group** (Rings/Amulets/Belts) in the class picker, sourced from a new
  `jewelleryClasses` list in the config rather than a literal.
- **`3b30821` / `89d94f5` / `c900a95` / `d589822` — the unidentified-tier control**, which had no
  editor at all: added, then made a 1..=5 slider, then made visible (the theme's `#090909` rail was
  painting on a `#0D0D0D` row), then width-bounded so its end and value are readable in a 460px
  stripe.
- **`93225f1` / `d589822` — the collapsed summary made readable.** The first cut used `.small()` in
  `DISABLED_TEXT`, which made the line that actually identifies a rule the hardest thing in the
  stripe to read, and painted live content in the palette's *disabled* colour.
- **`017b643` — both header lines fully clickable.** The filler is allocated *after* the buttons so
  they keep their own hit areas.
- **`f43dcc2` — "Any", not "All"** for a collapsed class group.

## Task Commits

1. **Task 1: Apply, the reload instruction, the failure surface, and the unapplied indicator** - `4e522ef` (feat)
2. **Task 2: Document the editor in CLAUDE.md and write the manual verification record** - `5da186b` (docs)
3. **Task 3: Live check — the accordion, the glyphs, and Apply against a running game** - blocking checkpoint; all six checks approved by the author, 2026-09-08

Follow-on stream: `3375ad4`, `35b695e`, `3b30821`, `89d94f5`, `c900a95`, `d589822`, `93225f1`,
`017b643`, `c9b62b9`, `6b7c46c`, `f43dcc2`.

**Plan metadata:** committed with this SUMMARY.

## Decisions Made

See `key-decisions` in the frontmatter. The load-bearing one is the **reversal of D-02** — the
persistence model changed after the checkpoint had already passed against the old one, which is
why check 5 of the manual record is annotated as superseded rather than quietly left standing.

## Deviations from Plan

**1. [Rule 3 — the author changed a locked decision after seeing it run] D-02's autosave replaced by an in-memory model with Revert**
- **Found during:** the Task 3 checkpoint, after the author had used the editor against a live game
- **Issue:** D-02 chose autosave so a crash or a stray Quit could not cost tuning work. In practice
  the author wanted edits to be discardable as a unit, and asked for Revert alongside Apply.
- **Fix:** `baseline: Vec<Rule>` in the app, the dirty path no longer writes, Apply persists then
  compiles, Revert restores. `14-CONTEXT.md`'s D-02 annotated SUPERSEDED with the original
  reasoning preserved and the accepted trade-off stated.
- **Committed in:** `6b7c46c`

**2. [Rule 2 — defects found by the author in live use, fixed in place] Ten further commits**
- **Found during:** the same checkpoint and the days around it
- **Issue:** the stale-Apply off-by-one, a missing condition editor, an invisible and then
  unreadable slider, an unreadable summary, a too-small click target, tofu glyphs, a missing class
  group, and a style name that read as nonsense in the picker.
- **Fix:** enumerated under *The live-use fix stream* above, each committed atomically.
- **Impact:** no scope creep beyond EDIT-09's own surface — every one of these is the editor this
  plan shipped, corrected. The oracle never moved.

---

**Total deviations:** 2 categories, 11 commits. **Impact on plan:** the plan's objective is met as
written; one of its inherited decisions (D-02) was reversed by the author on evidence.

## Issues Encountered

- **The debug binary holds a file lock while the overlay runs.** `cargo test` fails with
  `Access is denied` on `target\debug\ff.exe` if the stripe is up, because cargo builds all bin
  targets when compiling integration tests. Workaround: `cargo test --lib` alone skips bin targets,
  or point `CARGO_TARGET_DIR` at a scratch path for the integration run. Worth knowing before
  assuming a build break.
- **Two executor agents reported filtered test subsets as full-suite results.** Caught by
  independently re-running the whole suite after every wave. Not a code defect, but the reason the
  317-test figure above was verified directly rather than taken from a report.

## User Setup Required

None. Note for the player: PoE2 must run in **windowed** or **windowed-fullscreen** mode for the
stripe to draw at all, and the game never re-reads a filter on its own — Apply's own confirmation
line carries the reload path.

## Next Phase Readiness

- Phase 14 is functionally complete: all six checkpoint checks plus five second-pass checks are
  approved in `14-MANUAL-VERIFICATION.md`.
- One item is implemented and tested but not separately confirmed in-game — the full-row click
  targets (`017b643`). Recorded under *Not separately confirmed* in the manual record. Low risk:
  a regression is obvious the first time anyone opens a rule.
- Ready for phase verification, then `/gsd-audit-milestone 0.4` and `/gsd-complete-milestone`.

---
*Phase: 14-rule-editor-apply*
*Completed: 2026-09-08*
