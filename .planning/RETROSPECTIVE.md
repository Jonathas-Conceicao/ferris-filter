# Retrospective: FerrisFilter

A living record of what worked, what didn't, and the patterns established across milestones.

## Milestone: v0.1 — Initial Release

**Shipped:** 2026-05-28
**Phases:** 4 | **Plans:** 14 | **Commits:** 127 | **LOC:** ~4,970 Rust | **Span:** 2026-05-21 → 2026-05-28

### What Was Built

A Rust CLI that compiles a human-editable policy model into a deterministic PoE2 loot filter
(`generate`), prices currency from poe.ninja (`sync`), and installs the result into the game
folder (`install`). Plus an early-league price fallback (`--early-game`).

### What Worked

- **Snapshot + `artifact_guard` as a hard contract.** Locking `universal.filter` byte-for-byte early
  made every later refactor (pricing layer, fallback) safe — regressions surfaced instantly.
- **Pure-logic / IO split.** `sync` fetches; `generate` does all conversion + tiering. Kept the
  generator deterministic and unit-testable; the network layer hid behind an injectable `HttpClient`,
  so the whole sync pipeline is tested with `FakeClient` + recorded fixtures (zero live calls).
- **Spike before HTTP code.** Resolving the poe.ninja PoE2 API surface first avoided rework.

### What Was Inefficient

- **Stale ROADMAP checkboxes.** Phase 4 stayed `[ ]` through completion; caught only at milestone close.
- **An explicit `rates` field detour** in the fallback work was added then reverted in favor of deriving
  rates from the anchor orbs — a cleaner design reached after a round-trip.

### Patterns Established

- Cross-rates are **derived from the three anchor orbs**, read in whatever single unit they're quoted in
  (relative to the Chaos Orb) — unit-agnostic, backward-compatible with live chaos-quoted data.
- Committed, paired fallback data (`prices-fallback.json` + `tiers-fallback.json`) used together.
- `CHANGELOG.md` updated at milestone completion (convention recorded in `CLAUDE.md`).

### Key Lessons

- Keep `ROADMAP.md` checkboxes/progress in sync as phases complete, not at close.
- External/environmental blockers (a league rollover) are not code defects — fail safe and sign off.

### Cost Observations

- Sessions: multiple (context resets mid-Phase-4). Model: Opus-class for design discussions.
- Notable: byte-for-byte snapshot test paid for itself repeatedly during refactors.

## Milestone: v0.3 — Economy Items & Exceptional Gear

**Shipped:** 2026-06-27
**Phases:** 3 (8-10) | **Plans:** 5 | **Tasks:** 13 | **Commits:** 54 | **LOC:** ~7,534 Rust | **Span:** 2026-06-12 → 2026-06-27

### What Was Built

Three poe.ninja economy categories tracked end-to-end: Verisium (new), Lineage Support Gems
(new, `Class == "Support Gems"`), and Uncut Gems (fixed — the level now stays in the `BaseType`
so the blocks actually match in-game). Priced gem blocks carry a per-category background over the
value tier's text and border. The rule vocabulary gained a `corrupted` boolean, which drives three
exceptional-gear rules (uncorrupted, sockets >= 3 / >= 2, or quality >= 24) in a new pink
`exceptional` style.

### What Worked

- **Reusing a proven mechanism instead of adding a concept.** Gem backgrounds went through the
  priced-unique clone-null-append path as a data-driven `background_override`, not a new named
  style. That single choice kept the config-driven-styles refactor (999.1) out of scope, and
  Phase 9 landed in ~15 minutes with a guard test.
- **The blocking spike as the first task, not a separate phase.** Lineage Support Gems' filter
  `Class` was MEDIUM-confidence (poe2db said "Support Gems", NeverSink implied dual-class).
  Resolving it in the Phase 8 CONTEXT — before `mapping.json` was committed — meant zero rework.
  Same pattern that paid off for the poe.ninja API surface in v0.1.
- **Deferring the fixture re-baseline to one consolidated pass.** Phases 8, 9 and 10 all change
  emitter output; re-baselining only at Phase 10 avoided three rounds of oracle churn. Phase 10
  was explicitly sequenced last for this reason.
- **Splitting each phase into a code half and a policy half.** Phase 10 Plan 01 added
  `corrupted` + the `exceptional` style with the oracle **byte-identical** (nothing referenced
  them yet); Plan 02 authored the rules and re-baselined. The re-baseline diff was therefore
  provably policy-only.
- **Data-driven emitter branches.** `background_override == Some([r,g,b])` with no
  category-name string checks in the emitter kept Phase 8's carry and Phase 9's consumption
  independent.

### What Was Inefficient

- **Regenerating fixtures from the repo root twice.** Both Phase 9 and Phase 10 first ran
  `cargo run` in the repo root, which picked up a live `prices.json` from a prior `sync` instead
  of the seed fixture. Both times the fix was the temp-dir + absolute-binary-path approach the
  test helper already uses. The second occurrence was avoidable — the first had already been
  written up in a SUMMARY.
- **A stale carried todo.** "Phase 08-02: rewrite prices.rs gem dispatch branch" sat in STATE.md
  as pending after Plan 08-02 had already done exactly that.
- **Summary prose drifted from code.** `10-01-SUMMARY.md` states `exceptional` is a required
  `Styles` field; a later code-review fix made it `Option<Style>` and the summary was never
  amended. The milestone audit caught it, but only at close.

### Patterns Established

- **Per-category style overrides ride on the tier style, not beside it:** clone the tier `Style`,
  null the attribute being overridden, `emit_style_actions`, then append the override line.
- **PoE2 boolean conditions take no operator** — `Corrupted False`, never `Corrupted == False`.
  New optional `when` fields use `serde(default)` + `skip_serializing_if` so old configs parse
  unchanged and round-trip cleanly.
- **OR across conditions = separate rules.** Filter blocks are AND-only, so "extra sockets OR
  extra quality" is three rules, not one.
- **New named styles are added as `Option<Style>`**, matching `craftable`, so `--config` files
  that predate the style keep loading.
- **Fixtures are regenerated hermetically**: temp dir + the binary by absolute path, mirroring
  `generate_on_seed()`. Never `cargo run` from the repo root.

### Key Lessons

- When a fixture-regeneration footgun bites once, write the hermetic recipe into the plan for the
  *next* phase — a SUMMARY note alone did not prevent the repeat.
- Reconcile SUMMARY prose when a code-review fix changes a shape the summary described; the
  audit's tech-debt list is the last line of defence, not the first.
- Clear carried todos as part of the plan that satisfies them, not at milestone close.
- **Verify a "known failing test" against `main` before recording it as debt.** `parity_ingame`
  was written up as failing in the Phase 10 SUMMARY and carried into the milestone audit's
  tech-debt list, but it passes on `main` — its oracle was refreshed during this milestone and
  nobody re-checked. A stale red-test note costs the next milestone real triage time.

### Cost Observations

- Model mix: planner Opus / executor Sonnet (`balanced` profile), `yolo` gate mode.
- Sessions: few — phases were small and research-light (no RESEARCH.md, no VALIDATION.md).
- Notable: Phase 9 was a ~15-minute single-plan phase because the mechanism it needed already
  existed. The cheapest phase in the project so far, and a direct dividend of the Phase 8
  decision to make `background_override` data-driven.

## Cross-Milestone Trends

| Milestone | Phases | Plans | Commits | LOC (cumulative) | Span |
|-----------|--------|-------|---------|------------------|------|
| v0.1 Initial Release | 4 | 14 | 127 | ~4,970 | 7 days |
| v0.2 Declarative Rule Model | 3 | 7 | ~38 | — | 7 days |
| v0.3 Economy Items & Exceptional Gear | 3 | 5 | 54 | ~7,534 | 15 days |

**Trend — phases are getting smaller and cheaper.** Plans per phase fell from 3.5 (v0.1) to 2.3
(v0.2) to 1.7 (v0.3) as the declarative rule model absorbed work that used to require code. v0.3's
Phase 9 needed one plan and ~15 minutes because Phase 8 had already made the mechanism data-driven.

**Trend — the same two practices keep paying off.** (1) Resolve unknowns with a spike *before* the
code that depends on them: the poe.ninja API surface in v0.1, the Lineage gem `Class` in v0.3.
(2) A byte-for-byte oracle (`artifact_guard` + insta snapshot) makes every refactor and
re-baseline reviewable — it caught the intent of every emitter change across all three milestones.

**Recurring friction — oracle regeneration.** Fixture regeneration from the repo root picked up a
live `prices.json` in both v0.3 phases that touched the oracle. The hermetic recipe (temp dir +
absolute binary path) is now written down under Patterns Established.

**Process gap — no v0.2 retrospective section.** v0.2 shipped without one; the row above is
reconstructed from `MILESTONES.md`. Write the retrospective at close, not later.
