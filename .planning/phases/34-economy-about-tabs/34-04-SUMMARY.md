---
phase: 34-economy-about-tabs
plan: 04
subsystem: ui
tags: [egui, overlay, about-tab, screenshots, docs]
requires: [34-01, 34-03]
provides:
  - "ABOUT tab wired to about_view::about_tab"
  - "debug --tier-text TIER=TEXT screenshot flag"
affects: []
key-files:
  modified: [src/overlay/app.rs, src/overlay/shell.rs, src/overlay/screenshot.rs, CLAUDE.md, CHANGELOG.md]
  created: [".planning/phases/34-economy-about-tabs/shots/*.png"]
key-decisions:
  - "--tier-text lives in the cfg(debug_assertions) screenshot path only; release build verified"
metrics:
  tasks: 3
  completed: 2026-10-16
requirements: [ECO-01, ECO-02, ABOUT-01]
---

# Phase 34 Plan 04: ABOUT wiring, shots and docs Summary

The running overlay now shows the about_view page on ABOUT (old `shell::about_tab` deleted), the debug screenshot mode can seed an invalid tier cutoff, seven shots were rendered and viewed, and CLAUDE.md / CHANGELOG describe the new tabs.

## Commits
- 17f9de3: feat(34-04): ABOUT tab wired; --tier-text screenshot flag
- docs(34-04): ECONOMY and ABOUT shots
- docs(34-04): CLAUDE.md and CHANGELOG for ECONOMY and ABOUT

## Shots (all opened and viewed, under `shots/`)
- `about.png`: pass. Name, description, orange Ferris, `Version 0.5.0`, GitHub link, then disclaimer and three credit lines at the bottom. Observation: the pale global Ferris background shows behind the empty middle, and the divider under the link is faint and short; left as is (design, not a defect).
- `economy-fallback.png`: pass. League, `Never`, `0`, `Fallback prices` (warning colour), `Add economy section`, Tiers.
- `economy-synced.png`: pass. `Just now`, `358`, `Live prices`, `60 locked blocks`, red `Remove economy section`.
- `economy-invalid-tier.png`: pass. Red outline on S with `abc`; Revert enabled, Apply greyed.
- `remove-economy-popup.png`: pass. Remove (red) / Keep popup over the dimmed tab.
- `rules-economy-divider.png`: pass. Economy divider with `60 rules` and padlock, no x, no footer button.
- `config-no-tiers.png`: pass. Filter and Styles only (custom table cut by window height, scrollable).

## Deviations from Plan
None. No visual defects found in scope. Screenshot status line reads `Unapplied changes` in every shot (screenshot mode artefact, existing behaviour).

## Verification
`cargo test` green (1323 lib tests plus integration), `cargo fmt --all --check` clean, clippy totals 119 (lib test) + 2 + 1 = 122, `cargo build --release --bin ff` succeeds, `tests/fixtures` and `ferris-bg.png` unchanged. Shots used temp homes only; nothing outside them and `shots/` changed.

Docs also record the earlier fix(33) items: field labels cut with an ellipsis and Font size off showing a dimmed `Default` slider (the `Unapplied changes` status wording was already documented).

## Known Stubs
None.

## 34-VERIFICATION-CHECKLIST
Run the overlay with `FERRISFILTER_HOME` and `USERPROFILE` on a temp copy:
- ABOUT at 100 / 150 / 200% DPI: order is name, description, Ferris, version, link, divider, disclaimer, credits; Ferris crisp.
- GitHub link opens the browser.
- ECONOMY: Add economy section stages without writing, flips the label to Remove; Revert undoes it; Apply writes the banner at the end of the file.
- Remove economy section asks first (popup).
- A tier edit marks the ECONOMY tab (not CONFIG); Revert and Apply work from there; an invalid cutoff blocks Apply.
- RULES Economy divider has no x and no footer button.
- CONFIG has no Tiers section.
- Last sync is relative with the exact time on hover; Fallback prices shows without prices.json.
- Sync economy from ECONOMY does not freeze the window.

### Less-texty changes for review
- Tiers captions moved to tooltips.
- Tier error moved into the field tooltip (red outline only).
- No captions on ECONOMY; explanations are tooltips.

## Self-Check: PASSED
