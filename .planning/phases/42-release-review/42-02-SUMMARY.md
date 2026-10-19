---
phase: 42-release-review
plan: 02
subsystem: docs
tags: [readme, screenshots, release-review]
requires: [42-01]
provides: [re-taken README screenshots, corrected README settings-folder fact]
affects: [README.md, docs/screenshots]
key-files:
  modified:
    - docs/screenshots/rules.png
    - docs/screenshots/rule-editing.png
    - docs/screenshots/rule-style.png
    - docs/screenshots/config-style.png
    - docs/screenshots/economy.png
    - docs/screenshots/economy-rule.png
    - README.md
decisions:
  - "Rendered from a temp copy of tests/ui/test_ff.filter with case 5 restored to the old valid 'Socketed armour' (Sockets >= 3), because the current sample's 'Grammar error' rule makes the status line a red 'Apply refused' instead of the amber unknown-style line the old frames had."
  - "styles.json in the temp home = current styles-default.json built-ins + Recycle and kite taken from v0.6, so the Custom table and the expanded kite editor exist as before."
metrics:
  completed: 2026-10-18
---

# Phase 42 Plan 02: README screenshots and fact fixes Summary

Six README screenshots re-taken with the debug screenshot mode on v0.7 screens, and the README's one wrong fact (settings folder) corrected.

## Screenshots (all 660x1032, same names, each Read before commit)

Environment: FERRISFILTER_HOME, FERRISFILTER_GAME_DIR and USERPROFILE all under `%TEMP%\ff42-02-oGFz` (home/, game/, profile/); CARGO_TARGET_DIR `%TEMP%\fftgt42-shots`. Nothing touched the real %APPDATA%\FerrisFilter or the real game folder. Home: tiers-default.json, seed-prices.json as prices.json, styles.json as described above.

| File | Flags | Shows |
|------|-------|-------|
| rules.png | `--tab rules` | Rules list top; rule 3 is now the built-in `C` chip (cyan star), status "1 rule uses an unknown style" |
| rule-style.png | `--expand 2` | Preset S expanded, same framing |
| rule-editing.png | `--expand 14` | Between, booleans, sockets expanded (Endgame section) |
| economy-rule.png | `--expand-economy 1` | Economy section open, "A tier Currency" expanded; purple star chip |
| config-style.png | `--tab config --expand-style kite` | Config scrolled to C..hidden built-ins, Custom table (Recycle, kite), kite editor; new star-icon built-in defaults |
| economy.png | `--tab economy --leagues "Forbidden Rites,Runes of Aldur,Standard,Hardcore Forbidden Rites" --league "Forbidden Rites" --price-source live` | v0.7 Economy: Price source Live/Fallback switch, League dropdown with refresh, Last sync, Priced items (358, seed data), Reset to defaults on Tiers |

All six differed from the committed files, so all were replaced. Not retaken: none. No new file was added under docs/screenshots (6 files). The sample's status is the expected amber unknown-style line (the personal-path tooltip is not rendered).

Extra render for the author (not committed, not for README, shows a temp game-folder path): `C:\Users\jonat\AppData\Local\Temp\ff42-02-oGFz\new\config-top.png` (Config tab top: Game folder row with Browse, Filter, reload toggle, Styles with Reset to defaults and the new preset strip Custom / Flat Broke / Scrouging / Getting By / Comfy / Loaded / Div Farmer). No separate row labelled Strictness appears in that frame; the preset strip is the only new selector in the Styles section (the author should confirm it is the strictness selector).

## README facts changed

- Before: "Your settings live in the FerrisFilter folder (the one holding `styles-default.json`): ..."
- After: "Your settings live in `%APPDATA%\FerrisFilter` (created on the first save): ..." (same four files, same delete-to-reset sentence; Reset to defaults buttons not mentioned).

Sweep of README.md for styles-default, OneDrive, Documentos, uwu, Forbidden, Recycle, kite, exceptional, review ids, "regression": no matches. "Pre-compiled binaries from the release tab" kept.

## README candidates left for the author (not written)

- Config now has a Game folder row with Browse, and a preset strip (Custom plus six presets) with Reset to defaults; the Config bullet still says only "picks which filter file".
- Economy: Live/Fallback price source switch, league picker with refresh, Reset to defaults for tiers.
- Strictness selector, installer and update check (see 42-01/42-03 notes); GitHub release downloads.
- About tab now shows 0.7.0 (no About screenshot in the README).

## Deviations from Plan

- [Rule 3 - Blocking] The screenshot guard refuses a game folder it did not create, so the staged game dir is removed before each run (inside the temp tree only).
- Sample filter copy used for rendering (see decisions); tests/ui/test_ff.filter itself untouched.

## Commits

- 555f9ff docs(42-02): re-take README screenshots for v0.7
- 0d311a0 docs(42-02): correct README facts for v0.7

## Self-Check: PASSED
