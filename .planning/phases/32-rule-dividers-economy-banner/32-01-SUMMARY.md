---
phase: 32-rule-dividers-economy-banner
plan: 01
subsystem: filter-model
tags: [rust, parser, banners, economy-section]

requires: []
provides:
  - "section::banners / banner_starts / is_rule_line / RULE_LINE / SECTION_KEY, Banner + BannerKind"
  - "begin-only economy banner: find / refresh / add / remove, runs to next banner or EOF"
  - "parser fences: no block owns or absorbs a banner line"
affects: [32-02, 32-03, 32-04, 32-05, 32-06]

tech-stack:
  added: []
  patterns:
    - "Banner = rule line / # ff:<key> middle line / rule line, found by key via Line::ff_marker"

key-files:
  created:
    - .planning/phases/32-rule-dividers-economy-banner/shots/01-banner-format.png
  modified:
    - src/filter/section.rs
    - src/filter/parse.rs
    - src/filter/structure.rs
    - src/dynamic.rs
    - src/styled.rs
    - src/cli.rs
    - src/overlay/apply_all.rs
    - src/overlay/rules.rs
    - tests/dynamic_filter.rs
    - tests/filter_roundtrip.rs
    - tests/filter_structure.rs

key-decisions:
  - "v0.4 conversion kept and retargeted: it now inserts the economy banner above the old economy (per orchestrator decision)"
  - "SectionSpan.end is exclusive (start of next banner or line count); callers using h > begin && h < end stay correct, rebuild's after-side test became >= end"
  - "An old one-line `# ff:economy-begin` outside a banner is IncompleteBanner (damaged); `# ff:economy-end` is an ordinary comment"

requirements-completed: [SECT-01, SECT-04]

duration: ~1h
completed: 2026-10-16
---

# Phase 32 Plan 01: Banner model and begin-only economy banner Summary

**Three-line `# ff:section` / `# ff:economy-begin` banners in the lossless filter model, with the economy section re-expressed as a begin-only banner running to the next banner or EOF.**

## Accomplishments
- `src/filter/section.rs` rewritten: `RULE_LINE` (80 `#`), `banner_starts` (greedy, bounds-checked single pass), `banners`, `Banner`/`BannerKind`, `SectionSpan::body_start`, `IncompleteBanner`; `find`/`refresh`/`add`/`remove` work over the banner; `remove(add(x)) == x` for LF and CRLF; `refresh` keeps a following banner and is idempotent; `body_lines` rejects `section`/`economy-begin` keys.
- Parser: `lead_start` stops at a banner's closing rule line (fence set computed once per parse); live and commented-out blocks directly above a banner never own banner lines (tests for all three shapes, no blank line between).
- `structure::move_neighbour` still refuses to swap across any banner (Plan 03 adds hops); `rules::rebuild` uses the exclusive end.
- `migrate_v04` now writes the banner; `is_v04` unchanged.
- Callers and tests migrated (dynamic, styled, apply_all, rules, cli help text); no `END_LINE`/`END_KEY` left in `src/`.
- Integration tests: banner add/remove/refresh via the CLI, `remove_keeps_a_following_section`, `refresh_keeps_a_following_section`, `old_one_line_markers_are_damaged`, `banners_round_trip_byte_for_byte` (LF/CRLF/BOM), `foreign_banner_is_not_a_section`.

## Task Commits
1. Task 1: banner model, parser fences, callers - `d5dacca`
2. Task 2: integration tests and regression shot - `e974262`

## Gates
- `cargo test`: all green (1194 lib tests + integration suites; `artifact_guard`/`generate_snapshot` unmodified).
- `git status --porcelain tests/fixtures`: empty.
- clippy `^warning` count: 126 (baseline).
- `cargo fmt --all --check`: only the pre-existing diff in `src/overlay/widgets.rs` (log-slider test array, from phase 31, not touched here); all files changed by this plan are clean.

## Regression screenshot
`shots/01-banner-format.png`: the overlay loads a temp `sample.filter` (two rules, `# ff:section Leveling`, two rules, `# ff:section Endgame`, two rules, then the economy banner added by `ff-cli apply --add-economy`). The Rules tab lists six rules (indices 1-6, section banners are not shown yet, which is expected until Plan 04), the search hint reads `Search 6 rules, ...`, and the footer reads `+ 60 locked rule blocks (economy)` with Remove. The status line reads "Filter out of date - press Apply" (the regenerated economy differs from the temp copy's), not a damaged-section message. The sample stays at `C:/Users/jonat/AppData/Local/Temp/ff-shot-32/sample.filter`.

## Deviations from Plan

**1. [Rule 1 - Bug] Tests that appended rules after the economy section**
- Found during: Task 1 (`gate_numbering_skips_economy...`, `any_refusal_writes_nothing`, `rules.rs` `with_economy` fixture)
- Issue: with begin-only banners, anything after the economy banner belongs to the economy, so fixtures appending user rules after it changed meaning.
- Fix: those fixtures now put the user rules under their own `# ff:section After` banner, or above the economy.
- Files: src/dynamic.rs, src/overlay/apply_all.rs, src/overlay/rules.rs

**2. structure.rs test consts**: `END` is now the empty string and `BEGIN` the literal 3-line banner (asserted equal to `RULE_LINE` + `BEGIN_LINE`), keeping the existing multi-line fixtures intact; one empty-economy-then-rule sub-case was dropped (that rule would now be inside the economy).

No architectural changes.

## Known Stubs
None.

## Threat Flags
None. T-32-01..04 mitigations are covered by tests (refresh keeps a following banner, lone begin is damaged, bounds-checked scan, body marker rejection).

## Self-Check: PASSED
- src/filter/section.rs, tests/dynamic_filter.rs, shots/01-banner-format.png present; commits d5dacca and e974262 exist.
