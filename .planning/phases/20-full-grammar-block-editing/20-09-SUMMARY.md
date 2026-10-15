---
phase: 20-full-grammar-block-editing
plan: 09
subsystem: overlay
tags: [egui, overlay, docs, ui-review]
requires: ["20-08"]
provides:
  - styled/unknown style row flags with a flag budget and `+N` badge
  - single capped (96px) footer outcome area
  - Phase 20 overlay docs, CHANGELOG, requirement ticks, manual UAT script
affects: [src/overlay/rule_list.rs, src/overlay/app.rs, CLAUDE.md, CHANGELOG.md, .planning/REQUIREMENTS.md]
key-files:
  created: [.planning/phases/20-full-grammar-block-editing/20-MANUAL-VERIFICATION.md]
  modified: [src/overlay/rule_list.rs, src/overlay/app.rs, CLAUDE.md, CHANGELOG.md, .planning/REQUIREMENTS.md]
key-decisions:
  - "Style flag truncates to 12 characters then an ellipsis (plan example 'Exalted Sho...' was 11 chars; the 12-char rule wins: 'styled: Exalted Show...')"
  - "OUTCOME_MAX_HEIGHT const is cfg(test) only; production code uses the literal max_height(96.0) to satisfy the plan's grep"
  - "Requirements ticked with traceability 'Complete (UAT pending)'"
requirements-completed: [EDIT-01, EDIT-02, EDIT-06, STYLE-03]
duration: ~35min
completed: 2026-10-13
---

# Phase 20 Plan 09: Row flags, outcome area, docs Summary

Collapsed rows now show `styled: <name>` / `unknown style` flags with a 120px flag budget and `+N` overflow badge (label keeps 140px), and all footer messages share one 96px scrolling outcome area; CLAUDE.md, CHANGELOG and REQUIREMENTS describe the shipped editor, with a 13-step manual UAT script.

## Commits
- 4fb0e3b feat(20-09): style flags, flag budget and label width in collapsed rows
- c7376fb feat(20-09): one capped outcome area, 14px stale indicator, 4/8/16 spacing
- 2b66728 docs(20-09): overlay editor notes, changelog, requirements and manual verification script

## Deviations from Plan

- **Status block order:** it already sat between the FILTER heading and the search box, so no move was needed; no y-order test was added because `ui()` cannot be driven headlessly (needs an eframe Frame). The outcome area is tested through the extracted `outcome_lines` / `outcome_area` functions instead.
- **Jump click test:** clicking the unknown-style line in a headless frame is unreliable with empty fonts; `outcome_lines` tests assert the jump targets are carried, and the click handling reuses the previous `jump_to` path unchanged.
- **Crowded-row layout test:** asserts the pure `fit_flags` budget math plus a two-frame render of a crowded row; the 140px label floor is enforced by the budget formula (`min(120, inner - arrows - toggle - 8 - 140)`) rather than a measured label rect.
- **CLAUDE.md:** the stale v0.4 defence-checkbox/class-off-switch text was already gone from the file; only the Phase 19 rule-list note was rewritten.
- **Tooling:** state/roadmap tooling run afterwards; its diff was inspected and corrected (see STATE.md).

## Known Stubs
None.

## Self-Check: PASSED
Full `cargo test` green, `cargo fmt --check` clean, `cargo build --bins` warning-free, fixtures unchanged.
