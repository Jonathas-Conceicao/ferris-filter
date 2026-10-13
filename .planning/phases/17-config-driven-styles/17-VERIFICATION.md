---
phase: 17-config-driven-styles
verified: 2026-10-12T00:00:00Z
status: passed
score: 3/3 roadmap criteria verified (plus plan must-haves)
overrides_applied: 0
---

# Phase 17: Config-Driven Styles Verification Report

**Phase Goal:** Named styles are data in a committed `styles.json`, not a hardcoded vocabulary, and the static `generate` path is unaffected.
**Status:** passed. **Re-verification:** No.

## Observable Truths

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | New style in styles.json, used from a rule, is emitted with no code change | VERIFIED | `Config.styles` is a `StyleSet` (ordered map). The hardcoded `Styles` struct and the `FUTURE` notes are gone (grep finds none). The emitter looks styles up by name. Plan 04 adds an end-to-end CLI test, and the full suite passes. |
| 2 | Unknown style in a rule, or invalid styles.json, stops at load with a clear error | VERIFIED | `loader::load_styles`, `StyleSet::validate` and `check_rules` exist. Covered by the CLI tests in Plan 04: unknown style in an enabled rule, missing required style, duplicate name, bad colour, inline styles section. A disabled rule naming an unknown style still generates. |
| 3 | generate output unchanged; fixture byte-identical | VERIFIED | `git diff f398f19 -- tests/fixtures` is empty. `artifact_guard` and `generate_snapshot` pass in `cargo test` (462 unit tests, plus 11, 4, 16, 9, 18, 2, 8 and 1 in the integration binaries; 0 failures). |

## Artifacts and Wiring

- `styles.json` exists with 12 styles in file order: S A B C D E F Recycle unchanged hidden kite exceptional. This matches D-02.
- `config.json` and `dynamic_base.json` have 0 `"styles"` keys, so styles come only from styles.json (D-03).
- `Cargo.toml` has `indexmap`, and `StyleSet` preserves key order, which the overlay picker uses.
- `--styles` is wired on `generate` and `apply` (`InputPaths::with_styles`), per the CLI tests.

## CONTEXT Decisions

- D-01 (shape, ordered map): honored.
- D-02 (initial content equals the old styles): honored.
- D-03 (config.json loses styles): honored.
- D-04 (equality asserted before deletion): this was a Plan 01 task, and the old struct is now deleted.
- D-06 (required styles S-F, unchanged, hidden): honored.
- D-08 and D-09 (the style's action overrides the block's Show/Hide, and Hide styles can carry full visuals): covered by emitter unit tests per the Plan 02 must-haves.

**Deviation, acceptable:** the planned OD-1 behaviour was to reject any inline `styles` section. Review-fix work relaxed this. An inline section is accepted only if every entry equals the same-named styles.json entry, and is then dropped. Anything else is rejected with an error naming the file and the differing styles. CLAUDE.md and CHANGELOG describe this accurately. It does not conflict with the roadmap criteria or the CONTEXT decisions (the `ssf_*` configs were declared unsupported).

## Requirements Coverage

| Req | Status | Evidence |
|-----|--------|----------|
| STYLE-01 | SATISFIED | Truth 1; marked complete in REQUIREMENTS.md |
| STYLE-02 | SATISFIED | Truth 2 |
| STYLE-04 | SATISFIED | Truth 3 |

STYLE-03 belongs to Phase 20 and is correctly Pending. No orphaned requirements.

## Documentation

- CLAUDE.md has a `styles.json` layout row, a `config.json` row without styles, `dynamic_base.json` noted as having no styles, `--styles` documented for generate and apply, and a Named styles section listing all 12 styles including `exceptional`, plus the required styles. Accurate.
- CHANGELOG `[Unreleased]` records the styles.json addition, `--styles`, and the **BREAKING** removal of the `styles` section. Accurate. The older `craftable` entry at line 142 is historical.

## Anti-Patterns

None observed. The FUTURE notes are removed and no TBD/FIXME markers were found in the touched code during spot checks.

## Probes

No probe scripts apply to this phase.

## Human Verification

None required. The phase is data and CLI work covered by automated tests. Overlay picker order is unit-tested through `available_styles`.

## Gaps

None.

_Verifier: Claude (gsd-verifier)_
