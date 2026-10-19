---
phase: 39-league-picker-price-source
plan: 06
subsystem: overlay
tags: [screenshot, debug, league-picker, price-source]
requires: [39-04, 39-05]
provides:
  - "ff --screenshot flags --leagues, --league, --price-source (debug builds only)"
  - "Visual verification of the UI-SPEC league and price-source states"
affects: [src/overlay/screenshot.rs, src/overlay/app.rs, src/overlay/leagues.rs]
key-files:
  modified:
    - src/overlay/screenshot.rs
    - src/overlay/app.rs
    - src/overlay/leagues.rs
requirements-completed: [LEAGUE-01, LEAGUE-02, LEAGUE-03, LEAGUE-04, LEAGUE-05]
duration: ~40 min
completed: 2026-10-18
---

# Phase 39 Plan 06: Screenshot flags and capture gate Summary

Debug screenshot mode can now fake the league list (`--leagues <names>|none|loading`), the saved league (`--league`) and the price source (`--price-source live|fallback`) with no network, and every UI-SPEC state was rendered and inspected.

## Task 1 (commit 10bd903)
- `ShotLeagues { List, Failed, Loading }`, `PRICE_SOURCES`, `fake_leagues(names)` (slug = lowercase without spaces, hardcore for `HC ` prefix or `Hardcore`, indexed false), three parse arms with errors, module-doc usage line, parser and fake-list tests.
- `app.rs` screenshot setup (after the App is built): injects `ListState::Loaded/Failed("screenshot mode: no league list")/Loading`, then `set_league` and `set_price_source`, so `applied_policy` keeps the build-time policy and `--price-source fallback` shows the pending switch.
- `LeagueList::inject` dead-code allow narrowed to `#[cfg_attr(not(debug_assertions), allow(dead_code))]`. `cargo build --release`: no warnings.
- Tests: `cargo test --lib -- overlay::` 859 passed.

## Task 2: captures (no code change needed)
All run with FERRISFILTER_HOME, FERRISFILTER_GAME_DIR, USERPROFILE and APPDATA under a fresh `mktemp -d` root, with styles/tiers defaults copied in. PNGs were kept in a temp folder (not committed; `git status` shows none). Window renders 660 px wide (1x).

| # | Flags | Observed |
|---|-------|----------|
| 1 | `--leagues L` | League `Forbidden Rites` (default pick, value colour), Live segment lit, refresh button right of the dropdown, Sync enabled, Tiers editable. |
| 2 | `--league "Runes of Aldur"` | Dropdown shows `Runes of Aldur`, everything else as 1. |
| 3 | `--league "Fate of the Vaal"` (economy) | Dropdown `Fate of the Vaal (ended)` in amber, Sync economy greyed, status line amber. Rules tab: same amber status line. |
| 4a | `--leagues none --league "Runes of Aldur"` | Saved league shown enabled, Sync enabled, refresh enabled. |
| 4b | `--leagues none`, seeded prices.json | `Standard` (seed fixture records league `Standard`), Last sync Just now, 358 items. |
| 4c | `--leagues none`, no prices | `(no league)` dimmed dropdown, Sync greyed, refresh live. |
| 5 | `--leagues loading` | `Loading…` dimmed dropdown, refresh dimmed, Sync enabled. |
| 6 | `--price-source fallback` | Fallback segment lit; dropdown, refresh, Sync, Tiers rows and Reset all dimmed. |
| 7 | fallback + economy-section filter + seeded prices | Economy tab label ends with ` *`, Fallback lit, section row shows `0 locked blocks` and red Remove, status `Unapplied changes`, Apply lit. |
| 8 | geometry (by constants and pixels) | Button `ROW_HEIGHT` 34, gap `FIELD_GAP` 10, dropdown width = 252 - 34 - 10, so dropdown + button = `CONTROL_WIDTH` 252, the switch width; pixel read agrees (both end near x=571). Stripes alternate across the four rows. |

Observations and caveats:
- In 3 the status text is cut with an ellipsis (`Fate of the Vaal is no longer on p...`) at the 660 px window; the full `... pick a league` text is on hover (existing one-line status rule) and covered by the unit test on `ended_status`, so the acceptance wording is met only through the tooltip, not on screen.
- Hover tooltips (including the Tiers tooltip under Fallback noted by 39-04) cannot be captured; they remain unit-tested only.
- In capture 7 the `Reset to defaults` button looked slightly brighter than in 6, although the code computes `enabled = tiers_reset_enabled && !fallback` (false in both). Judged a rendering/compression effect, not confirmed.
- No defect found; no shell.rs / app.rs fixes were needed. The disabled `Ui` dims the dropdown and the Tiers block correctly.

## Deviations from Plan
- No fixture contains an economy banner, so capture 7 used a temp filter holding only the 80-`#` banner and `# ff:economy-begin ...` line (hence `0 locked blocks`).
- Capture 3 also checked the Rules-tab status as asked; Task 2 touched no files, so it has no commit.

## For the Release Review
Nothing for CHANGELOG/README: the new flags are debug-only and compiled out of release builds.

## HUMAN-UAT
- Confirm in the real window that `Reset to defaults` is dimmed under Fallback and that the Tiers and ended-league tooltips show.
- Confirm the truncated ended-league status reads acceptably (full text on hover).

## Self-Check: PASSED
Commit 10bd903 exists; changed files present; no PNGs in the repo.
