# Phase 42: Release Review - Context

**Gathered:** 2026-10-18
**Status:** Ready for planning

<domain>
## Phase Boundary

The milestone close-out defined in `.planning/PROJECT.md` → Milestone Close-out: code cleanup,
README screenshots, version `0.7.0`, a CHANGELOG draft, then the author's review. Covers
REV-01..REV-05. REV-05 (the author's review and rewrite of README.md and the CHANGELOG section) is a
human step stacked at the end of the autonomous run: this phase's automated work stops before it and
the phase is not marked complete until the author confirms.

</domain>

<decisions>
## Implementation Decisions

### Milestones are atomic
- Bugs introduced and fixed inside v0.7 are never mentioned outside `.planning/`: not in CHANGELOG,
  README, CLAUDE.md, code / doc comments or test names. Sweep for phase / plan / review ids
  (`WR-01`, `CR-01`, `IN-03`, `36-0x`, `Phase 3x`, `UAT...`) and "regression" / "fixed the ..."
  wording added during Phases 36-41 and reword or remove them. Bugs from released versions (e.g.
  FIX-01, the duplicated minimap tooltip that shipped in v0.6) are changelog material.

### REV-01 code cleanup
- Remove duplication added during the milestone where it is low risk: the three copies of the
  background-worker pattern (`EconomySync`, `LeagueList` in `src/overlay/leagues.rs`,
  `src/overlay/update_job.rs`) are the main candidate for one generic worker, only if the change is
  mechanical and fully covered by existing tests; otherwise record it as deferred. Drop dead code and
  scaffolding (unused `Cargo.toml` feature flagged in 40-REVIEW, stale comments listed in the
  phase REVIEW files' info items).
- Run `cargo fmt` (the drift in `src/overlay/app.rs`, `src/sync/league.rs`,
  `tests/isolation_guard.rs` predates this phase) and commit it on its own.
- Do **not** mass-fix the ~125 pre-existing clippy errors; record the count and the lint classes
  for a later cleanup. New code must not add clippy errors.
- No behaviour change: the full `cargo test --no-fail-fast` passes before and after (with
  `FERRISFILTER_HOME` / `FERRISFILTER_GAME_DIR` under temp).

### REV-02 README screenshots
- Re-take the README screenshots in `docs/screenshots/` whose screens changed in v0.7 (at least
  Config with the game-folder row and Strictness row, Economy with league picker and price source,
  the style editor if it changed), with the debug `--screenshot` mode under a temp home and temp game
  folder, keeping file names and framing. Inspect each PNG before committing.

### REV-03 version
- Set the root `Cargo.toml` version to `0.7.0` and refresh `Cargo.lock` (`ff-art` keeps its own).
  The `v0.7.0-rc.1` dry-run tag (Phase 40-08, deferred) will point at the last commit before this
  phase (`c560333`, version `0.7.0-rc.1`, notes from `[Unreleased]`), so changing the version and
  CHANGELOG here does not affect it.

### REV-04 CHANGELOG draft
- Write `## [0.7.0] - <date>` from the net changes since v0.6, folding in the current `[Unreleased]`
  entries (the GPL licence and the `uwu.mp3` removal), with a one-line milestone summary, then
  Added / Changed / Removed / Fixed, user-facing and short. Source: the "For the Release Review"
  notes in `.planning/phases/AUTONOMOUS-V07-DEFERRED.md` and the phase SUMMARY files. Fixed lists
  only bugs that existed in v0.6. Leave `[Unreleased]` empty. Fix the compare links to the
  `ferris-filter` repo and add `v0.6...v0.7.0`. Leave the date as the drafting date; the author will
  adjust it.
- This is a **draft for the author**; keep the existing CHANGELOG style (see the v0.6 section).

### README
- Agents change README.md only for facts that are now wrong (e.g. lines describing the old app-root
  folder, `styles-default.json` as the settings folder marker, the hard-coded OneDrive path, the
  pinned league) and for the screenshots. Keep the author's wording and voice; mark nothing as
  final. The author rewrites it in REV-05.

### Also align
- ROADMAP Phase 40 criterion 1 says "draft Release" while the workflow auto-publishes: align the
  wording. CLAUDE.md facts that changed in v0.7 (already updated per phase) get a final consistency
  pass.

### Claude's Discretion
- Plan split, which info items are worth fixing as part of cleanup.

</decisions>

<code_context>
## Existing Code Insights

- Collected Release Review notes: `.planning/phases/AUTONOMOUS-V07-DEFERRED.md` → "For the Release
  Review"; each phase's `*-SUMMARY.md` "For the Release Review" sections; open info items in
  `36-REVIEW.md` .. `41-REVIEW.md`.
- Screenshot mode: `cargo run --bin ff -- --screenshot <png> ...` (debug only; needs
  `FERRISFILTER_HOME` and `FERRISFILTER_GAME_DIR` under temp).

</code_context>

<specifics>
## Specific Ideas

- The author: README and CHANGELOG are human-read and mostly human-written; the draft should be
  easy to rewrite, not polished prose.

</specifics>

<deferred>
## Deferred Ideas

- Clippy cleanup of the pre-existing errors (later milestone).

</deferred>
