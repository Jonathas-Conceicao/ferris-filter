---
phase: 44-auto-detect-styles
plan: 03
subsystem: ui
tags: [rust, overlay, styles, auto-detect]
requires:
  - phase: 44-01
    provides: style_detect planner
  - phase: 44-02
    provides: add_detected_styles, detect_inputs, tag_styles
provides:
  - Auto Detect button on the CONFIG Custom heading
  - ConfigAction::AutoDetect handler (pending edits only, Ok / Info status)
affects: []
key-files:
  modified: [src/overlay/config_view.rs, src/overlay/app.rs, CLAUDE.md, .planning/phases/AUTONOMOUS-V072-DEFERRED.md]
key-decisions:
  - "Handler order: plan, add_detected_styles, sync_working_styles, tag_styles; no disk write, no hand-set dirty flag"
  - "Button width is the label's natural 139.5pt (112 is only a minimum)"
requirements-completed: [DETECT-01, DETECT-02, DETECT-03, DETECT-05, DETECT-06, DETECT-07]
completed: 2026-10-23
---

# Phase 44 Plan 03: Auto Detect wiring Summary

The CONFIG Custom heading now has an `Auto Detect` button (greyed with tooltip `No filter loaded.` when no filter is loaded) that runs the planner over the loaded filter and applies styles plus tags as one batch of pending edits, with status `Auto Detect: N styles, M rules tagged` (Ok) or `Auto Detect: nothing new` (Info).

## Tasks
1. Button, `ConfigAction::AutoDetect`, `can_auto_detect` ctx field, 2 layout tests
2. Handler plus 7 `auto_detect_*` app tests (pending-only with disk bytes unchanged, idempotent second click, nothing-new sets no unapplied state, Revert Rules, Revert Config, Reload, Quit confirm, Apply writing ids and group-wide tags with a no-op second Apply)
3. CLAUDE.md note, debug screenshots, deferred items

Tasks 1 and 2 landed in one commit (30f6d70) because the handler and button share the same two files and the tests; Task 3 in the docs commit.

## Deviations / findings
- Button width 139.5pt instead of 112pt: `ff_button_ex`'s 112 is a minimum and the label needs more; gap is 8pt via item spacing (no extra `add_space`). Test asserts width >= 112 and the 8pt gap.
- Reload finding: with nothing changed on disk the CONFIG Reload re-reads only the styles; rule edits (tags) stay and show `unknown style` until Apply drops them. Differs from "rules edits discarded" in the plan text; asserted as is.
- Fixture produces 3 styles / 6 rules tagged (font-20+colour, font-23, group font-30), not the plan's example.
- Screenshot `config-custom.png` cuts the Custom heading off below the fold (no scroll flag), so the check is deferred (item 5). `config-status.png` also produced.
- `cargo fmt` run on the two edited files only.

## Verification
`cargo test --locked` (temp target dir) green: 1737 lib tests plus integration tests; fixtures, CHANGELOG.md and README.md untouched.

## Self-Check: PASSED
