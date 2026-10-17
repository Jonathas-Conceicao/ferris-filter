---
phase: quick-261017-7lp
plan: 01
status: complete
completed: 2026-10-17
requirements: [UAT-7LP-1, UAT-7LP-2, UAT-7LP-3, UAT-7LP-4]
commits: [63f5c10, 53e690e, 41cecd6]
---

# Quick 261017-7lp: UAT feedback round 3 (economy headers, staged rows, marker, field gap)

Four UAT items for the `ff` overlay, each done as planned.

## Commits

Range `2305cca..41cecd6`:

- `63f5c10` feat: economy blocks carry `# ff:name` / `# ff:style`; the read-only rows resolve them
- `53e690e` feat: a staged Add economy section shows browsable read-only preview rows
- `41cecd6` feat: marker on the selected tab, text-field trailing gap, docs and UI test filter

## What changed

1. **Economy headers.** `emit_currency_sections` / `emit_unique_sections` take a `tags` flag
   (false from `generate`, true from `generate_economy`). Category tier blocks get
   `# ff:name <T> tier <Group>` and `# ff:style <id> <T>` (legacy `# ff:style <T>` for an id-less
   StyleSet). Unique blocks get `# ff:name <T> tier Uniques` only. `WorkingFilter::style_of` is the
   one style/chip resolver for user and economy rows; the collapsed economy row draws the style chip.
   Search matches the bare name.
2. **Staged preview.** `WorkingFilter` holds a preview `Document` built from
   `section::add("", body)` with block ids starting at `1 << 48` (new `filter::parse_with_id_base`),
   so ids never collide with the working document. `economy_rows()`, `economy_source()` and
   `entries()` use it while an Add is staged. `stage_add_economy` takes the body.
   `App::refresh_staged_economy` re-prices it only when the pending styles/tiers change (`TierEntry`
   gained `PartialEq`). New debug flag `--stage-economy add|remove`.
3. **Marker.** `tab_markers` lost its `active` parameter; the selected tab is marked too.
4. **Field gap.** `ff_text_field_ex` puts the cursor back past the whole frame and adds
   `FIELD_GAP` (10pt) in a horizontal layout with room left. `TIER_UNIT_GAP = FIELD_GAP`; the tier
   boxes still sit exactly 10pt from their unit.

Docs: CLAUDE.md (dynamic.filter row, rule-list note, tabs note, screenshot flags), CHANGELOG
`[Unreleased]`, `tests/ui/test_ff.filter` (economy blocks tagged, plus a unique block) and
`tests/ui/README.md`.

## Gates

- `cargo test`: 1390 lib tests (was 1387 before this task, 1497 total in the plan's count) plus all
  integration suites, 0 failed.
- `cargo fmt --check`: clean.
- `cargo clippy --all-targets`: 126 warnings (limit 126; one new one in my test was fixed).
- `git diff --stat -- tests/fixtures`: empty. `artifact_guard` and `generate_snapshot` green.

## Screenshots

In `shots/` (debug build, temp `FERRISFILTER_HOME`/`USERPROFILE`), each viewed:

- `a-tier-row.png`: tier row with name, style chip, expanded read-only body
- `a2-tier-row-preset.png`: tier body with the Style preset (B), no Custom lines
- `b-unique-row.png`: unique row (name, no preset, Custom lines)
- `c-staged-preview.png`: staged Add on a filter without economy, 60 browsable rows, one expanded
- `d-marker-selected-tab.png`: `ECONOMY *` on the selected ECONOMY tab
- `e-field-gap.png`: quoted-list field and **+ Add** with a visible gap

## Deviations from Plan

- **[Rule 1] Test adjustments, not behaviour.** `tests/dynamic_filter.rs::economy_section_matches_generate`
  now strips `# ff:` lines before comparing with `generate` (same as the emitter parity tests).
  `tier_boxes_are_narrow_and_sit_at_the_value_column` sorts widgets by row centre, because the unit
  label now shares the field frame's vertical extent (it is vertically aligned with the frame, an
  improvement); the 10pt gap assertion is unchanged.
- The plan's "pending custom tier style id appears in the tag" check was replaced by an assertion
  on the built-in `builtin:S` tag: tier styles are the built-ins, whose ids are fixed and rejected
  if changed.
- Build used `CARGO_TARGET_DIR` outside the repo because a running `ff.exe` locked `target/debug`.

## Known Stubs

None.

## Self-Check: PASSED

Commits `63f5c10`, `53e690e`, `41cecd6` exist; all six screenshots exist under `shots/`.
