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

## Milestone: v0.5 — Generic Filter Editor & Initial App Configuration

**Shipped:** 2026-10-15
**Phases:** 13 (16-28) | **Plans:** 71 | **Commits:** 452 | **Tests:** 1142

### What Was Built

- A lossless filter parser (`filter::Document`) and line-level edit operations that keep every
  untouched byte.
- `styles.json` as data, with stable style ids and committed `-default.json` fallbacks.
- The `.filter` file as the only store, with a fenced, regenerable economy section and guarded writes.
- A virtualized rule list and full-grammar editor in `ff`, a CONFIG tab, one all-or-none Apply,
  single-instance launch and in-game reload after Apply.

### What Worked

- **Parser first, everything else on top.** Phase 16's byte-identical round trip made every later
  editor feature safe to ship against foreign ~500-block filters.
- **Hands-on UAT rounds as their own phases** (24-27). Each round of the author's feedback became a
  small, autonomously executed phase with its own requirement IDs, so feedback never leaked into
  half-finished work.
- **The oracle held.** `artifact_guard` / `generate_snapshot` flagged the only fixture change
  (the `# ====` banner spelling), which was deliberate.

### What Was Inefficient

- **Scope grew after the first audit.** Phases 23-28 (six of thirteen) were added after the
  2026-10-13 audit, so the audit had to be refreshed and bookkeeping drift (unchecked roadmap
  rows, missing `requirements-completed` frontmatter) fixed at wrap-up.
- **Phase 28 skipped the verifier step** (review + UAT only), recorded as tech debt.

### Patterns Established

- Metadata lines are `# ff:...`; old `#ff:` spellings are read and normalized on the next Apply.
- Rules link to styles by id; names are informational and refreshed on Apply.
- Shipped defaults are committed `*-default.json`; per-user live files are git-ignored.
- One Apply validates everything first, then writes all files or none.

### Key Lessons

- Treat the author's UAT as a planned stage of the milestone, not an afterthought: four rounds of
  feedback were the bulk of the polish.
- Run the verifier on every phase, even small ones, so the close audit does not have to reconstruct it.

### Cost Observations

- Model mix: `balanced` profile, `yolo` gate mode; UAT feedback phases run autonomously.
- Notable: 13 phases and 452 commits in four calendar days (2026-10-12 → 2026-10-15).

## Milestone: v0.6 — New PoE2-based Tool UI

**Shipped:** 2026-10-17
**Phases:** 7 (29-35) | **Plans:** 30 (+7 quick tasks) | **Commits:** 281 | **Tests:** 1392 lib + integration

### What Was Built

- One Cargo workspace; the `ff-art` dev tool extracts the game's UI art read-only from the root.
- The shared `src/skin/` and a skinned, left-docked `ff` shell with four tabs, one status line and
  skinned popups; RULES redrawn on it with rules added at the top.
- `# ff:section` sections, with the economy section as the locked last one, browsable read-only and
  shown as Apply will write it.
- A CONFIG style table and line-by-line style editor (off / cancel / value groups, folder sounds),
  reused for Custom rule styling; ECONOMY and ABOUT tabs; the prototype retired.

### What Worked

- **Prototype first, then the real thing.** Two quick-task rounds of `ff-proto` settled the look on
  fake data, so research was skipped and the phases only ported proven widgets.
- **Pixel diffs for a refactor.** Retiring the old theme was checked with 12 before/after screenshots
  diffing to 0 pixels, which made a broad UI cleanup safe without hand review.
- **Debug screenshot mode in every UI phase.** Each change came with viewed screenshots, and the
  author's feedback rounds could point at exact frames.

### What Was Inefficient

- **UAT bookkeeping lagged the real UAT.** The author walked every phase in game, but the
  HUMAN-UAT / VERIFICATION files of phases 30-35 were never updated, so the close had 14 open
  artifacts to acknowledge, and the ECO / ABOUT requirement boxes were still unticked.
- **Feedback landed as quick tasks after the phases.** Three rounds (2rf, 7lp and the post-phase
  fixes) changed behaviour the phases had already verified (economy headers, staged previews,
  pending-style display); no milestone audit was run on top of them.

### Patterns Established

- Everything visual comes from `src/skin/` (palette, fonts, textures, `ff_*` widgets); no second theme.
- Screenshot runs use a temp `FERRISFILTER_HOME` / `USERPROFILE` and the built exe, settled until two
  captures are byte-identical.
- The economy section is shown from an in-memory preview whenever Apply would write something
  different (staged add, pending style or tier edits).

### Key Lessons

- Record the author's UAT verdict in the UAT file the same day it is given; otherwise the close
  has to reconstruct it.
- When feedback changes what a verified phase does, re-run that phase's verification (or the
  milestone audit) before closing.

### Cost Observations

- Model mix: `balanced` profile, `yolo` gate mode; planner on opus, executors / reviewers / checkers
  on sonnet; phases and feedback rounds run largely autonomously.
- Notable: 7 phases plus 7 quick tasks and 281 commits in three calendar days (2026-10-15 → 2026-10-17).

## Milestone: v0.7 — Initial Public Release and Strictness Selector

**Shipped:** 2026-10-18
**Phases:** 7 (36-42) | **Plans:** 45 | **Commits:** 261

### What Was Built

- One per-user folder with every default compiled into the exe, and an isolation guard so tests and
  screenshot mode never touch the real folders.
- A game folder picker with the game's own reload button; the Economy Visibility (strictness) row;
  Reset to defaults for styles and tiers; only the nine required built-in styles ship.
- A league picker and a Live / Fallback price source on the Economy tab.
- A tag-triggered release workflow, a per-user Inno Setup installer, zip and checksums, and an
  on-demand Check for updates on the About tab.

### What Worked

- **One autonomous run, reviews stacked at the end.** `/gsd-autonomous --from 36 --to 42` ran all
  seven phases, and the author answered one numbered list of checks instead of per-phase stops.
- **A release-candidate dry run before the real tag.** `v0.7.0-rc.1` exercised the whole workflow
  (release-check, build, version resource, packaging, publish) in one push, so the real `v0.7.0`
  tag carries no workflow risk.
- **Tests that reproduce a UI crash.** An egui id clash (tooltip id equal to a child segment id)
  was pinned by hover tests that crashed on the old id formula before the fix.

### What Was Inefficient

- **Game-art matching took several rounds.** The reload button went through three art choices and
  two tuning passes (glyph, mirror, tint, size, turn) before matching the in-game button; a
  side-by-side render against the author's screenshot from the start would have saved rounds.
- **The rc dry run needed a side commit.** Phase 42 had already emptied `[Unreleased]`, which an rc
  tag needs for its notes; ordering the dry run before the changelog draft avoids that.
- **UAT files again lagged the verdicts** for per-scenario lines (statuses were updated at close).

### Patterns Established

- `release.yml` is the only source of release assets; `installer/package.sh` is for local checks.
- Tags are `vX.Y.Z` and must equal `v` + the Cargo version; rc tags publish pre-releases.
- README.md and CHANGELOG.md prose wraps at 70 characters; the CHANGELOG starts at 0.7.0.
- Hand manual checks to the author as one numbered list, answerable per item.

### Key Lessons

- For anything that must look like the game, compare against a screenshot of the game first.
- Run release-pipeline dry runs before the Release Review edits the changelog.

### Cost Observations

- Model mix: `balanced` profile, `yolo` gate mode; phases run autonomously with planner,
  executor, reviewer and verifier agents; close-out and UAT fixes in the main session.
- Notable: 7 phases, 45 plans and 261 commits in two calendar days (2026-10-17 → 2026-10-18).

## Milestone: v0.7.2 — Small UI Improvements

**Shipped:** 2026-10-24
**Phases:** 8 (43-50) | **Plans:** 24 (+ the Release Review inline) | **Commits:** 200

### What Was Built

- Alpha on style colours, through the picker, the previews and every emitter, with 3-value
  `styles.json` files still loading and the fixtures byte-identical.
- Auto Detect on CONFIG: a filter's existing styling becomes style tags and `Custom Style N`
  styles, as pending edits.
- Economy rule names with their price range; overlay requests throttled to once an hour per kind
  (`ff-state.json`) and a separate CLI user agent.
- A configurable show / hide hotkey in a game-style keybind row, swapped live on Apply.
- Social-panel section headers, magnifier and clear X in search fields, art painted by its
  visible box; drag-and-drop reordering of rules and custom styles in place of the arrows.

### What Worked

- **Parallel worktree phases with the checkout kept for the author.** Phases 45-48 were
  independent and ran in their own `gsd/phase-*` worktrees, merged into the milestone branch,
  while the main checkout stayed on `milestone/v0.7.2-small-ui-improvements` so the author could
  build and try it at any time. Seven feature phases finished in one day.
- **Reviews stacked at the end again.** One deferred file
  (`phases/AUTONOMOUS-V072-DEFERRED.md`) carried every manual check and judgement call; the author
  answered it plus two short feedback rounds, and the audit could cite it directly.
- **A spike before the drag code.** Phase 49 started with headless egui experiments (wheel and
  edge scroll during a drag, drag survival under virtualization, tap vs drag); they became
  behaviour tests guarding what the drag code relies on.
- **Game art found in the game files.** Every Phase 47 / 48 piece (toggle plates, stone bar, clear
  X, magnifier, keybind button) was extracted with `ff-art`; no substitutes were drawn.

### What Was Inefficient

- **Merge conflicts in long lines.** CLAUDE.md's one-line table rows and long overlay notes, and
  the shared deferred file every phase appended to, conflicted on almost every worktree merge and
  had to be resolved by hand.
- **Semantic test breakage after merges.** Branches that merged cleanly still broke tests that
  assumed the other phase's code was absent, found only by
  running the full suite on the milestone branch.
- **gsd-sdk mangled CRLF planning files.** A tool call inserted line breaks after requirement ids
  in REQUIREMENTS.md (restored once in Phase 44, left in the file until the archive repaired it),
  and `milestone complete` could not parse the roadmap, so the close-out was done by hand.
- **Screenshot framing was not repeatable.** The real cursor showed as a row hover or triggered
  auto-scroll, and the `--expand-style` scroll was still animating at the capture frame, so the
  same render landed a few points apart between takes.
- **Stale roadmap bookkeeping.** Worktree phases did not update the ROADMAP checkboxes or Progress
  table on the milestone branch; the audit flagged it and the archive fixed it.

### Patterns Established

- Independent phases may run in parallel worktrees; the main checkout follows the milestone branch.
- One shared deferred file per autonomous run, answered as a numbered list, plus numbered feedback
  rounds recorded in the same file.
- Art is painted by its opaque box (`paint_visible` / `box_uv`); `ff-art` manifest options (such as
  `key_white`) clean art at extract time when egui cannot at draw time.
- A UI behaviour the code depends on (egui drag, scroll) gets a headless behaviour test.

### Key Lessons

- Run the full test suite right after every worktree merge, before the next phase starts.
- Keep files that every phase appends to (deferred lists, CLAUDE.md notes) in short lines or
  per-phase sections, so merges do not conflict.
- After any gsd tool call that writes a planning file, diff it; prefer hand edits for CRLF files.
- For screenshots, park the cursor off the render window and wait for scroll animations to settle.

### Cost Observations

- Model mix: `balanced` profile, `yolo` gate mode; phases run autonomously with planner,
  executor, reviewer, UI auditor and verifier agents, four of them in parallel worktrees;
  feedback rounds and close-out in the main session.
- Notable: 8 phases, 24 plans and 200 commits from roadmap to close in two calendar days
  (2026-10-23 → 2026-10-24); Phase 47's code review took three fix passes.

## Cross-Milestone Trends

| Milestone | Phases | Plans | Commits | LOC (cumulative) | Span |
|-----------|--------|-------|---------|------------------|------|
| v0.1 Initial Release | 4 | 14 | 127 | ~4,970 | 7 days |
| v0.2 Declarative Rule Model | 3 | 7 | ~38 | — | 7 days |
| v0.3 Economy Items & Exceptional Gear | 3 | 5 | 54 | ~7,534 | 15 days |
| v0.4 Overlay Filter Editor Rework | 5 | 16 | 146 | ~16,093 | 14 days |
| v0.5 Generic Filter Editor & Initial App Configuration | 13 | 71 | 452 | ~50,244 | 4 days |
| v0.6 New PoE2-based Tool UI | 7 | 30 | 281 | ~70,900 | 3 days |
| v0.7 Initial Public Release and Strictness Selector | 7 | 45 | 261 | ~82,800 | 2 days |
| v0.7.2 Small UI Improvements | 8 | 24 | 200 | ~92,100 | 2 days |

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
