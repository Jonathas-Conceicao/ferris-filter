---
phase: "07-new-capabilities-docs"
plan: "03"
subsystem: "docs"
tags: [docs, CLAUDE.md, rule-model, DOC-01]

dependency_graph:
  requires: [07-01]
  provides: [DOC-01]
  affects: [CLAUDE.md]

tech_stack:
  added: []
  patterns: []

key_files:
  modified:
    - CLAUDE.md

decisions:
  - "Repository-layout table source-of-truth entry renamed from filter-model.json to config.json, describing the actual v0.2 shape"
  - "Stale 'Single-filter AreaLevel progression' section replaced by 'Declarative rule model' section documenting rules/when/styles/classGroups"
  - "UnidentifiedItemTier confirmed working in-game added to Conditions list with no hedge"
  - "Base-defence filters confirmed note placed in both the Conditions list (Task 1) and the Declarative rule model section"
  - "Design-philosophy blockquote updated to declare reconciliation complete as of v0.2"
  - "Flasks/charms and jewellery behaviour prose corrected to match actual config.json rules (>=Magic mid-band, normal charms deep endgame, jewellery now hidden not shown)"

metrics:
  duration_minutes: 3
  completed_date: "2026-06-05"
  tasks_completed: 2
  files_modified: 1
---

# Phase 7 Plan 03: CLAUDE.md Declarative Rule Model Realignment Summary

CLAUDE.md realigned to the v0.2 declarative rule model: stale `filter-model.json`/`areaLevelBands` prose replaced by an accurate `rules`/`when`/`styles` reference, and `UnidentifiedItemTier` + base-defence filter caveats cleared to confirmed status.

## What Was Built

Two targeted documentation edits to `CLAUDE.md`, docs-only (no source code touched):

**Task 1 — Remove unverified caveats:**
- Added `UnidentifiedItemTier` to the Numeric/ordinal conditions list as a confirmed PoE2 condition (author-tested, v0.2), with usage example (`>= 4`)
- Replaced the "Open verification items" blockquote (questioning whether base-defence filters work) with a "Confirmed working in-game" note covering `BaseArmour`/`BaseEvasion`/`BaseEnergyShield` and `UnidentifiedItemTier`

**Task 2 — Document the declarative rule model:**
- Repository-layout table: `filter-model.json` row (which no longer exists) renamed to `config.json`; description now accurately lists all v0.2 top-level sections (`styles`, `build`, `weaponClasses`/`armourClasses`/`flaskCharmClasses`, `classGroups`, `rules`, `alwaysShow`)
- `universal.filter` row: removed stale `[areaLevelBands "6-25".weapons -> ...]` block-tag example; updated to `Show # <rule name>` format
- Design-philosophy blockquote: removed "that section needs reconciling" — reconciliation is complete as of v0.2; `config.json` is the authoritative model
- "Single-filter AreaLevel progression" section replaced by "Declarative rule model" section covering:
  - `rules` array semantics (ordered, first-match-wins, each rule = one Show/Hide block)
  - Complete `when` condition vocabulary table (class, rarity, areaLevel, baseArmour/Evasion/EnergyShield, quality, sockets, baseType, unidentifiedTier) with JSON types and emitted filter conditions
  - Named styles table (all 11 styles: S/A/B/C/D/E/F/Recycle/unchanged/hidden/unidentified) with action and effect descriptions
  - `classGroups` documentation-only note
  - Fallthrough note
- Corrected flasks/charms and jewellery prose in the Tier/color section to match actual config.json rules (mid-band is `>=Magic` not `==Magic`, normal charms surface at AreaLevel 78, jewellery now uses `hidden` style not de-emphasized Show)

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Accuracy] Corrected flasks/charms and jewellery prose in Tier & color section**
- **Found during:** Task 2 — full read of CLAUDE.md before editing
- **Issue:** The "Tier & color system" section still described the old pre-v0.2 behaviour: mid-band flask/charm as "Magic only" (now `>=Magic`), jewellery as kept Show (now `hidden`), no mention of the deep-endgame normal-charms rule (CAP-04). These were not stale-model references but inaccurate behavioural claims that would mislead future work.
- **Fix:** Updated the flasks/charms and jewellery paragraphs to match the actual config.json rules that were committed in Phase 7 Plan 01.
- **Files modified:** CLAUDE.md
- **Commit:** 91da8de

## Known Stubs

None.

## Threat Flags

None — documentation-only change, no new code or config paths.

## Self-Check: PASSED

- CLAUDE.md modified: confirmed (git status clean after commits)
- Task 1 commit cdc8e85: confirmed via `git log`
- Task 2 commit 91da8de: confirmed via `git log`
- `rg "Open verification items" CLAUDE.md` returns nothing: PASS
- `rg "areaLevelBands" CLAUDE.md` returns nothing: PASS
- `rg "UnidentifiedItemTier" CLAUDE.md` returns confirmed-status lines: PASS
- `rg "unidentified" CLAUDE.md` shows named style documented: PASS
- Filter grammar, Conditions, Actions, Dracula tier table sections still present: PASS
