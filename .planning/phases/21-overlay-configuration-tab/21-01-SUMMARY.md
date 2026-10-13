---
phase: 21-overlay-configuration-tab
plan: 01
subsystem: models
tags: [styles, tiers, serde, loader]
requires: []
provides:
  - "StyleSet::{is_builtin, contains, get_mut, iter, insert_new, remove_custom}, StyleNameError, Style::blank_show"
  - "FilterColor/MinimapShape ALL, as_str, from_name"
  - "loader::render_styles, loader::render_tiers, Unit::as_str"
affects: [21-02, 21-04]
tech-stack:
  added: []
  patterns: ["one compact JSON line per entry for tracked config files"]
key-files:
  created: []
  modified: [src/models/config.rs, src/models/tiers.rs, src/loader.rs]
key-decisions:
  - "Spaced style names are allowed: the #ff:style tag round trip test passes for 'My Style'"
  - "render_tiers never sorts; loaded order is preserved so ValidatedTiers still passes"
requirements-completed: [CONF-02, CONF-03]
duration: ~15min
completed: 2026-10-13
---

# Phase 21 Plan 01: Style and tier model layer Summary

Public StyleSet add/edit/delete API with built-in protection and name validation, plus pure `render_styles`/`render_tiers` renderers that round-trip the committed files.

## Accomplishments
- `StyleSet::insert_new` refuses empty, padded/control-char, duplicate (case-sensitive) and built-in case-shadow names with UI-copy errors; `remove_custom` refuses the nine built-ins and preserves order.
- Enum vocabularies (`ALL`, `as_str`, `from_name`) for colours and shapes; `Sound` omits unset `id`/`custom`.
- `render_styles` (one line per style) and `render_tiers` (loaded order) re-parse value-equal and re-validate.

## Spaced-name round-trip outcome
`set_meta(id, "style", "My Style")` serialize/re-parse returns `Some("My Style")`. Spaced names are therefore accepted; no whitespace refusal was added.

## Commits
- 8549be8: StyleSet editing API, name validation, enum vocabularies
- 6c24301: render_styles / render_tiers / Unit::as_str

## Deviations from Plan
None. Verification: lib tests 660 pass, `cli_generate` passes (artifact_guard/generate_snapshot byte-identical), `cargo fmt --check` clean, no fixture or config file touched.

## Self-Check: PASSED
