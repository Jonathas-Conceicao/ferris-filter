---
phase: 18-filter-as-store-economy-section
verified: 2026-10-12T00:00:00Z
status: human_needed
score: 5/5 must-haves verified
overrides_applied: 0
human_verification:
  - test: "Launch ff.exe by double-click from Explorer (cwd not the repo) with PoE2 windowed"
    expected: "Stripe opens, resolves the app root from the exe folder, loads the game-folder dynamic.filter, shows status notice; APPLY / ADD ECONOMY / RELOAD / SYNC ECONOMY buttons work"
    why_human: "GUI cannot be launched in verification; overlay wiring was only checked by grep and unit tests"
  - test: "In the overlay, press APPLY, then edit dynamic.filter externally and press APPLY again"
    expected: "First Apply writes (or says up to date); second is refused with a 'changed on disk' message and RELOAD is offered"
    why_human: "GUI path; the underlying apply_file guard is covered by library and CLI tests"
  - test: "Run the real v0.4 game-folder dynamic.filter through the first Apply and reload it in PoE2 (Options > Game > Item Filter > Refresh)"
    expected: "Filter loads without warnings; all user rules look identical in game"
    why_human: "In-game behavior. Verification used the committed fixture copy only, never the real game folder."
---

# Phase 18: Filter as Store & Economy Section Verification Report

**Phase Goal:** The filter file is the only store, and the economy-priced blocks live in a fenced section FerrisFilter can regenerate without touching anything else.
**Status:** human_needed (all automated checks pass; GUI and in-game checks remain)
**Re-verification:** No, initial verification (post-review-fix state, HEAD 9388f56)

## Observable Truths (ROADMAP success criteria)

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | apply and overlay edit game-folder `dynamic.filter` directly; `dynamic.json`, `dynamic_base.json`, overlay JSON store and `--reset/--base/--store` are gone; breaking change in CHANGELOG | VERIFIED | `src/cli.rs` Apply has only `--filter/--add-economy/--remove-economy/--tiers/--styles`. `tests/cli_errors.rs` asserts `--reset`, `--store x`, `--base x` are rejected. No `dynamic_base.json` in the tree (only fixture `tests/fixtures/v04-dynamic-base.json`). `src/overlay/app.rs` uses `LoadedFilter` / `dynamic::apply_file`. `src/overlay/store.rs` is absent. CHANGELOG `[Unreleased]` carries a **BREAKING** entry. Leftover repo-root `dynamic.json` is an untracked, git-ignored, unread file. |
| 2 | `ff` launched from anywhere finds styles.json/tiers.json/prices.json without relying on cwd | VERIFIED (GUI part human) | `src/paths.rs::app_root`: `FERRISFILTER_HOME`, then walk up from the exe dir, then cwd, then error. Used by `run.rs` and `overlay/app.rs`. Spot check: ran `ff-cli apply` from an unrelated cwd with `FERRISFILTER_HOME` pointing at a temp folder, and it read that folder's inputs. Double-click launch is a human item. |
| 3 | Apply regenerates the economy section at the end, between markers, changing no byte outside; re-apply is identical | VERIFIED | Spot check on the v0.4 fixture: first apply converts and writes; second apply reports "already up to date", `cmp` identical. Markers `#ff:economy-begin` / `#ff:economy-end` sit at the end of the file. Tests in `dynamic_filter.rs` and `filter::section` pass. Damaged markers hard-stop (checked with a corrupted end marker, rc=1, "economy section is damaged ... nothing was written"). |
| 4 | User can add an economy section to a new empty filter or a foreign filter and remove it; a new filter starts empty | VERIFIED | `--add-economy` on a non-existent file created it (48090 bytes) with the section; `--remove-economy` left 0 bytes. Plain apply on a file with no section refuses with a hint and writes nothing. A missing file without `--add-economy` is refused; `LoadedFilter::load` treats an absent file as an empty filter. |
| 5 | Apply refuses if the filter changed on disk since load, and says why; first write keeps a backup | VERIFIED (library) | `safe_write.rs` (476 lines, hash guard, temp-file backup, fsync, atomic rename with retry), tests pass. Spot check: after the first apply, `dynamic.filter.ffbak` is byte-identical to the original. The overlay RELOAD path is a human item. |

**Score:** 5/5 truths verified (GUI aspects deferred to human).

## Artifacts

| Artifact | Status | Details |
|----------|--------|---------|
| `src/paths.rs` | VERIFIED | Pure `resolve_from` plus `app_root`. Review fix applied: `styles.json` is the only root marker. |
| `src/safe_write.rs` | VERIFIED | Guarded, backed-up, atomic write. |
| `src/filter/section.rs` | VERIFIED | find/refresh/add/remove, damaged-marker errors, v0.4 migration. |
| `src/dynamic.rs` (490 lines) | VERIFIED | `LoadedFilter`, `ApplyOp`, `economy_body`, `apply_file`. Legacy store API removed. |
| `src/emitter.rs` / `src/compile.rs` | VERIFIED | `generate_economy`, `load_economy`. Generate output unchanged. |
| `src/cli.rs` / `src/run.rs` | VERIFIED | `--tiers`/`--styles` resolve against the app root; `--filter` stays cwd-relative as documented. |
| `src/overlay/app.rs` | VERIFIED (wiring by grep) | Drives `LoadedFilter` / `apply_file` / `app_root`. |
| `.gitignore` | VERIFIED | Has `/dynamic.json`, `/dynamic.filter`, `*.ffbak`. |

## Key Links

| From | To | Status |
|------|----|--------|
| `cli.rs` Apply to `run.rs::cmd_apply` to `dynamic::apply_file` | WIRED (run end to end) |
| `overlay/app.rs` to `dynamic::apply_file` / `LoadedFilter::load` / `paths::app_root` | WIRED (grep) |
| `dynamic::economy_body` to `compile::load_economy` and `emitter::generate_economy` | WIRED (apply output contains priced blocks) |

## Behavioral Spot-Checks (temp dirs only; the game folder was never written)

| Behavior | Result | Status |
|----------|--------|--------|
| `cargo test` (CARGO_TARGET_DIR=ff-target): 515 lib tests, cli_errors 12, cli_filter_copy 4, cli_generate 16, dynamic_filter 15, filter_roundtrip 18, sync_prices 8, doc 1 | all pass, 0 failures | PASS |
| v0.4 migration then second apply | converted once; idempotent; `.ffbak` equals original | PASS |
| `git diff ad5e3ff..HEAD` over `tests/fixtures/seed-universal.filter`, `tests/snapshots`, `seed-prices.json`, `config.json` | empty, so artifact_guard and generate_snapshot fixtures are byte-identical | PASS |
| Add, remove, damaged marker, missing file, no-section refusals | as specified | PASS |

## Migration style-loss check

The `craftable` entry in the user's per-user `dynamic.json` is `"craftable": null` in `styles` and appears nowhere else (`grep -c craftable` is 1). No rule uses it, so nothing is lost. More generally, the migration works on rendered text. Compared the fixture before the economy banner with the migrated output after removing the added `#ff:name` lines: the only difference is the first `#====` banner line, which was absorbed into the regenerated economy section. All rendered action lines (SetTextColor, SetBackgroundColor, PlayEffect, MinimapIcon, font size) are preserved byte for byte, and the block count is 93 before and after. Derived `rule N` labels are not turned into `#ff:name` (`Show # rule 4` stays as is). No user-visible style is lost.
Caveat: the old economy section is regenerated from the current styles.json, prices and tiers on the first Apply, which is intended (ECON-03).

## Requirements Coverage

All nine IDs appear in PLAN frontmatter (plans 01 to 07) and in REQUIREMENTS.md, marked `[x]` and Complete for Phase 18. There are no orphaned Phase 18 requirements.

| Requirement | Status | Evidence |
|-------------|--------|----------|
| STORE-01 | SATISFIED | Truth 1 |
| STORE-03 | SATISFIED | Truth 4 |
| STORE-04 | SATISFIED | Truth 1; flags rejected by test |
| STORE-05 | SATISFIED (double-click is human) | Truth 2 |
| ECON-01 | SATISFIED | Truth 3 |
| ECON-03 | SATISFIED | Truth 3 |
| ECON-04 | SATISFIED | Truth 4 |
| EDIT-09 | SATISFIED | Truth 5 |
| EDIT-10 | SATISFIED | Truth 5 |

(REQUIREMENTS.md still lists EDIT-03..08 and ECON-02 as unchecked; those belong to Phase 19.)

## Docs

- CLAUDE.md layout table: the `styles.json`, `dynamic.filter`, `ferrisfilter apply`, `ff` and `ff-cli` rows match the code (apply flags, app-root order, markers quoted exactly, ffbak, RELOAD). The pipe in the table cell is escaped.
- CHANGELOG `[Unreleased]`: contains Added entries (economy section, `--filter` / `--add-economy` / `--remove-economy`, ffbak and guard, app-root, v0.4 conversion) and the BREAKING Changed entry. Accurate.
- Minor note: the `*.ffbak` / `/dynamic.json` ignore notes are present and consistent.

## Anti-Patterns

No TBD/FIXME/XXX debt markers were introduced in the checked files (spot grep). `cargo fmt --check` reports differences in `src/cli.rs` (formatting only, not a goal blocker; the "Info" level). Overlay stale test names still reference `dynamic_base.json` (cosmetic, Phase 19 will rework them).

## Gaps Summary

No gaps. Remaining items are GUI and in-game checks listed in the frontmatter.

---

_Verified: 2026-10-12_
_Verifier: Claude (gsd-verifier)_
