---
phase: 17-config-driven-styles
fixed_at: 2026-10-12T00:00:00Z
review_path: .planning/phases/17-config-driven-styles/17-REVIEW.md
iteration: 1
findings_in_scope: 4
fixed: 4
skipped: 0
status: all_fixed
---

# Phase 17: Code Review Fix Report

**Fixed at:** 2026-10-12
**Source review:** .planning/phases/17-config-driven-styles/17-REVIEW.md
**Iteration:** 1

**Summary:**
- Findings in scope: 4 (fix_scope: critical_warning; IN-01..IN-05 not attempted)
- Fixed: 4
- Skipped: 0

Verification: full `cargo test` passes on the main checkout after the fast-forward (458 lib tests
plus every integration suite, including `cli_errors`, `cli_generate`, `dynamic_filter`).
`tests/fixtures/` is untouched, so the `seed-universal.filter` byte-identity holds. The committed
`styles.json` passes the new WR-02 checks.

## Fixed Issues

### WR-01: `generate` panics on a `Config` whose `StyleSet` was never loaded

**Files modified:** `src/emitter.rs`, `src/loader.rs`
**Commit:** 55500e5
**Applied fix:** `generate_as` now calls `config.styles.validate()?` first. The new variant is
`GenerateError::Styles(#[from] StyleError)` and is transparent for diagnostics. `style_for_tier`
returns `Result` with a new `GenerateError::UnknownTier { tier }` instead of panicking.
`emit_currency_sections` and `emit_unique_sections` now propagate that `Result`. The loader doc
comment that cited the panic is updated. A new test checks that an empty `StyleSet` gives
`GenerateError::Styles(MissingRequired)` rather than a panic.

### WR-02: `StyleSet::validate` lets a styles.json through that emits an invalid or silently degraded filter

**Files modified:** `src/models/config.rs`
**Commit:** e8983bd
**Applied fix:** A new `StyleSet::check_ranges` runs for every style. It rejects:
- `font` outside 1..=45
- `minimap.size` above 2
- `sound.id` outside 1..=16
- `sound.vol` above 300
- a sound that sets both `id` and `custom`, or neither

Range failures use the new `StyleError::OutOfRange { style, field, value, range }`. Sound-source
failures use `StyleError::AmbiguousSound { style, problem }`. The `Sound` doc comment no longer
claims generate-time enforcement and now describes what `validate` checks. A new test covers each
rejection and the inclusive boundaries.

### WR-03: Any pre-Phase-17 `dynamic.json` stops the overlay from starting, and the error suggests a fix that does not work

**Files modified:** `src/loader.rs`, `src/models/config.rs`, `src/compile.rs`, `src/overlay/store.rs`, `tests/cli_errors.rs`, `CLAUDE.md`, `CHANGELOG.md`
**Commit:** 0b57ecb
**Status:** fixed: requires human verification (behaviour change within locked decision OD-1)
**Applied fix:**
- `loader::load_config(path, styles_path)` now loads the styles file itself. An inline `styles`
  section is checked by content:
  - If every inline entry deserializes to a `Style` equal to the same-named `styles.json` entry,
    the section is dropped. This covers an untouched seeded store and an empty `{}`.
    `store::save` never writes `styles`, so the next Apply removes the section from disk.
  - Otherwise the load fails with `InlineStylesSection { path, styles_path, differing }`.
- The rewritten message names the config file, the styles file and the differing styles (for
  example `craftable`). It says "Move any styles you changed or added into <styles file>, then
  delete the `styles` section from <file>". It no longer mentions `--styles`.
- `compile::load_inputs` and `overlay::store::load` both use the new signature.
- Tests updated: the rejection tests now use a genuinely differing entry and assert that it is
  named. New tests show a matching section is accepted, on the CLI, in `compile` and in the
  overlay store, and that the next save drops it.
- The OD-1 wording in CLAUDE.md and the CHANGELOG `[Unreleased]` BREAKING entry are updated to
  match.

The user's own `dynamic.json` (13 inline styles, including `craftable`) will still be rejected,
now with a message naming `craftable`. That matches OD-1: an edit is never silently ignored.

### WR-04: `"styles": null` bypasses the legacy-section rejection

**Files modified:** `src/models/config.rs`, `src/loader.rs`
**Commit:** 504f7e9
**Applied fix:** `legacy_styles` keeps the type `Option<serde_json::Value>`, but now has
`deserialize_with = "deserialize_present_value"`. A present key, even `null`, becomes
`Some(Value::Null)`; an absent key still defaults to `None`. `load_config` rejects a non-object
section with the generic `InlineStylesSection` message. Two tests were added: one checks that
`null` is captured as present, the other that `load_config` rejects it.

## Notes

- `loader::tests::load_styles_rejects_duplicate_name` fails in a CRLF checkout (for example a
  fresh git worktree with `core.autocrlf=true`). It splices text after `"{\n"`, which never
  matches `"{\r\n"`. The failure predates these fixes and is unrelated to them; the test passes in
  the LF main checkout. Consider making it line-ending agnostic.

---

_Fixed: 2026-10-12_
_Fixer: Claude (gsd-code-fixer)_
_Iteration: 1_
