# v0.7 autonomous run — items deferred to the end

Collected during `/gsd-autonomous --from 36 --to 42` (started 2026-10-18). The author asked for every
human review and input to be stacked here and presented after Phase 41, before Phase 42's review.

## Deferred manual checks

- **Phase 36** (`36-HUMAN-UAT.md`): (1) release `ff` against an absent `%APPDATA%\FerrisFilter`
  with no data files beside the exe shows the defaults; Apply creates the folder with
  `styles.json` / `tiers.json` / `ff-settings.json` and writes nothing beside the exe or in the
  working directory. (2) A real Sync economy with and without `FERRISFILTER_HOME` writes
  `prices.json` only in the matching folder and names the league.

## Open gaps after automatic gap closure

(none yet)

## Judgement calls made without asking

- Phase 36: no UI-SPEC (no visible UI change); planner chose `styles-default.json (built in)` labels
  for errors on built-in copies; folding a section may create the config folder.
- Phase 36-07: CHANGELOG / README edits skipped per Milestone Close-out; the wrong README sentence is
  recorded in the 36 SUMMARY for the Release Review.

## For the Release Review (Phase 42)

- Phase 36: README.md lines 66-69 still describe the old app-root folder ("the one holding
  `styles-default.json`"); CHANGELOG entry for the per-user folder still to write (see
  `36-07-SUMMARY.md` → For the Release Review). `generate` still reads `./prices.json` while `sync`
  writes to the per-user folder (CLI dev tools; mention in CLAUDE.md / help).
- Phase 36: clippy `result_large_err` warnings grew with the new loaders, plus one
  `type_complexity` at `src/overlay/config_state.rs:125` (pre-existing warning class, not fixed).
