---
phase: 32-rule-dividers-economy-banner
plan: 06
subsystem: overlay-docs-evidence
tags: [rust, egui, screenshots, docs]

requires: [32-05]
provides:
  - "--rename-section <n> debug screenshot flag"
  - "Seven viewed section shots under shots/"
  - "CLAUDE.md and CHANGELOG describing the banner format, CLI, settings and section UI"
affects: []

key-files:
  modified:
    - src/overlay/screenshot.rs
    - src/overlay/app.rs
    - src/skin/icons.rs
    - CLAUDE.md
    - CHANGELOG.md
  created:
    - .planning/phases/32-rule-dividers-economy-banner/shots/sections.png
    - .planning/phases/32-rule-dividers-economy-banner/shots/economy-expanded.png
    - .planning/phases/32-rule-dividers-economy-banner/shots/rename.png
    - .planning/phases/32-rule-dividers-economy-banner/shots/search-collapsed.png
    - .planning/phases/32-rule-dividers-economy-banner/shots/section-above.png
    - .planning/phases/32-rule-dividers-economy-banner/shots/leading-divider.png
    - .planning/phases/32-rule-dividers-economy-banner/shots/economy-absent.png

key-decisions:
  - "Padlock shackle is now a full half circle (closed): the old icon deliberately left the right leg open, which read as unlocked for both the Economy header and the can't-enable flag"

requirements-completed: [SECT-01, SECT-02, SECT-03, SECT-04, SECT-05]

completed: 2026-10-16
---

# Phase 32 Plan 06: Evidence and docs Summary

**Debug-only `--rename-section <n>`, seven viewed section screenshots, a closed-padlock fix, and CLAUDE.md / CHANGELOG brought up to the banner format and section UI.**

## Task Commits
1. `--rename-section` (parse, debug-only seeding via `nth_user_section`, tests) - `acf121e`
2. Padlock fix - `7619982`; shots - `2a031fd`
3. CLAUDE.md + CHANGELOG - see `git log` (`docs(32-06): CLAUDE.md and CHANGELOG for rule sections`)

## Gates
`cargo test` green (1239 lib + integration); `cargo fmt --all --check` clean; clippy `^warning` 126 (baseline); `tests/fixtures` unchanged.

## Verification shots (all viewed, sample filter in `%TEMP%\ff-shot-32`)
- `sections.png`: toolbar (field, `+ ADD RULE`, `+ ADD SECTION`) fits at 535pt; two headerless rules on top, rule 1 (no conditions, rules follow) shows the amber WR-02 triangle; LEVELING expanded with `3 rules` (the 2-block Simple Defences group counts once, row 4 shows `Defences: Full AR, Full EV, AR/ES, EV/ES` and its name); ENDGAME collapsed (chevron right, `4 rules`); ECONOMY last, collapsed, `60 rules`, closed padlock, no x; no `locked rule blocks` row; 36pt headers clearly distinct from 52pt rows; a divider's arrows step it past one rule, so Leveling up and Endgame down are lit (rules to hop over). Pass.
- `economy-expanded.png`: Economy expanded after Endgame, padlock plus the interim x, muted read-only rows with no check or arrows. Pass.
- `rename.png`: LEVELING header shows a text field with the name selected (focus seeded by the new flag) and the count/cluster still visible. Pass.
- `search-collapsed.png` (`--search ruby`, Endgame saved collapsed): only ENDGAME shown, expanded for display, `1 of 4 rules`, its arrows dimmed (reorder disabled while searching), the Ruby rule below; Leveling and Economy hidden. Pass.
- `section-above.png` (`--expand 4`): expanded grouped rule with Name, Conditions, Class, Simple Defences chips and Style visible. The bottom row (`Section above` / Duplicate / Delete) is below the frame (the editor is taller than the window), so it is not visible: the row itself is evidenced only by the Plan 05 headless tests. Partial.
- `leading-divider.png`: file starting with a banner: header `START` first, no headerless rules, `2 rules`, up arrow dimmed (nothing above), `Add economy section` footer button (no economy in the file). The `+ Add rule` above a leading divider is covered by the Plan 03 tests, no shot.
- `economy-absent.png`: no Economy header, `Add economy section` button at the bottom, Endgame collapsed as the last section. Pass.

Observations (not bugs): the "highlighted" row in 05-economy-wr02.png is a pointer-hover artefact: the screenshot window's pointer sits at a fixed spot (about y 570 to 590) and whatever row or header lies there is drawn hovered (here Economy header in `sections.png`, the Ruby rule in `economy-expanded.png`). The `Filter out of date` status in every shot is the initial NeverApplied state, expected.

## Deviations from Plan
**1. [Rule 1 - Bug] Padlock drew as an open lock.** The shackle arc stopped at 0.8 pi so the right leg never reached the body; it read as unlocked for the Economy header and `can't enable`. Now a full half circle. `src/skin/icons.rs`, commit `7619982`. Shared icon, so the can't-enable flag changes with it.
**2. `section-above.png` does not show the bottom row.** The expanded rule is taller than the window; a flag to scroll to the end was out of scope. Noted above.

## Known Stubs
None.

## 32-VERIFICATION-CHECKLIST
Use a temp home (`FERRISFILTER_HOME` and `USERPROFILE` under `%TEMP%`) and a sample filter; never the real game folder.
- [ ] SECT-01: load a filter with `# ff:section` banners; each shows as a header with the right rule count; a foreign `#====` comment line stays a comment and creates no header.
- [ ] Rules above the first banner have no header; a filter starting with a banner has no headerless rules.
- [ ] `+ Add section` inserts a banner at the top, scrolls to it and focuses the name box.
- [ ] `Section above` on an expanded rule puts the banner above it; on a grouped rule it goes above the whole run.
- [ ] Rename (pencil): Enter commits, Escape cancels; surrounding spaces trimmed, empty becomes `Section`, pasted newlines stripped.
- [ ] Move a section up and down: the divider steps past one rule per click (a `# ff:group` run counts as one rule); arrows are dimmed when no rule is left to step over.
- [ ] Remove a section (x): only the banner goes, its rules join the section above.
- [ ] Apply writes each section edit; Revert undoes it; Reload re-reads it (check for each of add, rename, move, remove).
- [ ] SECT-03: collapse and expand persist across an overlay restart (stored in `ff-settings.json` `collapsed_sections`) and never show the unapplied marker or change the filter file.
- [ ] Renaming a collapsed section keeps it collapsed.
- [ ] SECT-04: Economy is the last header, collapsed by default, padlock closed, `{n} rules`, rows read-only and muted, no up / down arrows.
- [ ] Economy expanded shows the small remove x; it opens the Remove economy popup; after removal the `Add economy section` footer button appears and re-adds it.
- [ ] `ff-cli apply --add-economy` appends the three-line banner at the end; `--remove-economy` removes the banner and blocks and keeps a following user section banner; plain apply never adds one.
- [ ] A lone old one-line `# ff:economy-end` or `# ff:economy-begin <text>` file is reported damaged / not recognised (stops apply).
- [ ] SECT-05: search finds a rule inside a collapsed section; header expands for display with `m of n rules`; clearing the search restores the collapse state.
- [ ] Rule arrows hop a divider (first rule of a section up moves into the previous section; last rule down likewise).
- [ ] RULES-03: `+ Add rule` on a file that starts with a banner inserts the rule above the banner.
- [ ] WR-02: a condition-less enabled non-Continue rule followed by more rules shows the amber triangle; the last rule, a Continue rule and a disabled rule do not.
- [ ] Round trip: open an untouched banner filter, Apply, the file is byte-identical.
- [ ] Padlock on the Economy header and on a `can't enable` rule both look closed.

## Self-Check: PASSED
Commits `acf121e`, `7619982`, `2a031fd` and the docs commit exist; seven PNGs present.
