---
phase: 06-emitter-rewrite-migration-parity
verified: 2026-06-05T14:30:00Z
status: passed
score: 7/7 must-haves verified
overrides_applied: 1
override:
  accepted_by: author
  date: 2026-06-05
  reason: "human-verify checkpoint approved by author at the Phase 6 parity gate. Parity test reports ZERO non-reorder differences vs the pre-edit oracle; full suite 153 tests green; pricer sections byte-identical. Code-review WR-03 (UnknownStyle test coverage) fixed; remaining WR-01/WR-02/IN-03 accepted as minor (WR-02/IN-03 are pre-existing in unchanged unique/header code; deferred to Phase 7 DOC realignment)."
human_verification:
  - test: "Run `cargo run -- generate --early-game` from the repo root and eyeball the top of the produced filter"
    expected: "Policy blocks appear in rule order (recycle, gear bands, flask/charm bands, then the currency section); no old #==== section banners present; the pricer currency section starts at '# --- Currency ---'"
    why_human: "The automated `parity_policy_blocks_match_oracle` test confirms semantic equivalence, but the human checkpoint in Plan 02 (Task 4 gate) requires the author to sign off that the policy blocks read as expected and no unintended behavioral differences remain before marking the phase complete"
---

# Phase 6: Emitter Rewrite + Migration Parity Verification Report

**Phase Goal:** The hardcoded emit_gear_bands/emit_flask_charm_bands special-casing is gone; generate compiles the rule list in order; config.json is migrated to the new schema and produces a filter semantically equivalent to the pre-edit oracle (tests/fixtures/parity-baseline-earlygame.filter); the artifact-guard fixture is re-baselined; pricer currency/unique sections unchanged. The 7 in-game edits are NOT in this phase (Phase 7).
**Verified:** 2026-06-05T14:30:00Z
**Status:** human_needed
**Re-verification:** No — initial verification

## Goal Achievement

### Observable Truths

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | src/emitter.rs no longer contains emit_gear_bands / emit_flask_charm_bands / emit_recycle; contains emit_rules driving all policy blocks from config.rules | ✓ VERIFIED | Grep confirms zero matches for all three old function names; `fn emit_rules` present at line 258 |
| 2 | generate() is fallible and returns a GenerateError at generate-time for unknown styles | ✓ VERIFIED | `pub fn generate(...) -> Result<String, GenerateError>` at line 41; `GenerateError::UnknownStyle` defined at lines 20-30; `resolve_style` at lines 59-84 returns Err for any unknown style name |
| 3 | config.json contains the migrated `rules` array (+ classGroups); no `areaLevelBands` | ✓ VERIFIED | `"rules"` array with 13 rules at line 51; `"classGroups"` at line 39; no `areaLevelBands` key present anywhere in config.json |
| 4 | The semantic parity test (parity vs parity-baseline-earlygame.filter) exists and passes | ✓ VERIFIED | `parity_policy_blocks_match_oracle` present in tests/cli_generate.rs lines 381-403; `cargo test` confirms it passes (PASS in full suite run) |
| 5 | seed-universal.filter re-baselined; artifact_guard + generate_snapshot pass | ✓ VERIFIED | Both files present (seed-universal.filter and cli_generate__generate_snapshot.snap); `artifact_guard` and `generate_snapshot` pass in full test suite |
| 6 | Pricer currency/unique sections unchanged — `pricer_section_present_and_unchanged` passes | ✓ VERIFIED | Test present at lines 413-458 in tests/cli_generate.rs; passes in full suite; asserts `Class == "Stackable Currency"` only in pricer section, bare `Class` only in policy section |
| 7 | `cargo build` and `cargo test` pass (full suite) | ✓ VERIFIED | Full `cargo test` run: 124 lib tests + 4 cli_errors + 4 cli_filter_copy + 11 cli_generate + 8 sync_prices + 1 doc-test = 152 total, 0 failed |

**Score:** 7/7 truths verified

### Required Artifacts

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `src/emitter.rs` | emit_rules + When->filter-line + resolve_style + GenerateError | ✓ VERIFIED | All four components present at their stated lines; fallible signature confirmed |
| `config.json` | migrated rules array + classGroups | ✓ VERIFIED | 13 rules, 5 classGroups (buildWeapons, buildArmour, allArmour, allGear, flaskCharm) |
| `tests/cli_generate.rs` | semantic parity test against frozen oracle | ✓ VERIFIED | `parity_policy_blocks_match_oracle`, `generate_early_game`, `split_blocks`, `normalize_block`, `policy_blocks`, `pricer_section_present_and_unchanged` all present |
| `tests/fixtures/seed-universal.filter` | re-baselined golden master | ✓ VERIFIED | File present; shows new rule-driven format (no section banners, rule-named blocks); `artifact_guard` passes against it |
| `tests/snapshots/cli_generate__generate_snapshot.snap` | updated insta snapshot | ✓ VERIFIED | Present; `generate_snapshot` passes |

### Key Link Verification

| From | To | Via | Status | Details |
|------|----|-----|--------|---------|
| `src/emitter.rs::generate` | config.rules | `emit_rules(&mut out, config)?` | ✓ WIRED | Lines 44: `emit_rules(&mut out, config)?` before pricer sections |
| `src/run.rs::cmd_generate` | emitter::generate | `.map_err(miette::Error::from)?` | ✓ WIRED | Line 75-76: `emitter::generate(&config, &priced).map_err(miette::Error::from)?` |
| `tests/cli_generate.rs::parity test` | `tests/fixtures/parity-baseline-earlygame.filter` | normalized-block-set comparison | ✓ WIRED | Lines 382-403: test reads oracle file via `concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/parity-baseline-earlygame.filter")` |
| `tests/cli_generate.rs::artifact_guard` | `tests/fixtures/seed-universal.filter` | CRLF-normalized byte comparison | ✓ WIRED | Lines 272-287: test reads fixture and byte-compares against `generate_on_seed()` |

### Data-Flow Trace (Level 4)

Not applicable — this phase produces a CLI tool and test infrastructure, not UI components rendering dynamic data. The relevant data flow (config.rules → emit_rules → filter text) is verified by the parity test and artifact_guard.

### Behavioral Spot-Checks

| Behavior | Command | Result | Status |
|----------|---------|--------|--------|
| Full test suite passes | `cargo test` | 152 passed, 0 failed | ✓ PASS |
| emit_rules present, banned functions absent | grep for `fn emit_gear_bands\|fn emit_flask_charm_bands\|fn emit_recycle` in src/emitter.rs | No matches | ✓ PASS |
| generate() signature is fallible | grep for `pub fn generate` in src/emitter.rs | Returns `Result<String, GenerateError>` at line 41 | ✓ PASS |
| config.json has `rules` and no `areaLevelBands` | grep | `"rules"` found, `areaLevelBands` absent | ✓ PASS |
| parity_policy_blocks_match_oracle | `cargo test parity_policy_blocks_match_oracle` | ok | ✓ PASS |
| artifact_guard | `cargo test artifact_guard` | ok | ✓ PASS |
| pricer_section_present_and_unchanged | `cargo test pricer_section_present_and_unchanged` | ok | ✓ PASS |

### Requirements Coverage

| Requirement | Source Plan | Description | Status | Evidence |
|-------------|------------|-------------|--------|----------|
| RULE-03 | 06-01-PLAN.md | Hardcoded gear/flask special-casing removed; all policy from rule list | ✓ SATISFIED | emit_gear_bands / emit_flask_charm_bands / emit_recycle absent; emit_rules drives all 13 policy blocks |
| MIG-02 | 06-02-PLAN.md | generate on migrated config produces semantically equivalent filter | ✓ SATISFIED | parity_policy_blocks_match_oracle passes; zero non-reorder differences found (per SUMMARY) |
| MIG-03 | 06-02-PLAN.md | Artifact-guard/snapshot fixture re-baselined deliberately; full suite passes | ✓ SATISFIED | seed-universal.filter re-baselined; artifact_guard + generate_snapshot pass; generate_is_deterministic passes |
| MIG-04 | 06-02-PLAN.md | Pricer currency/unique sections unchanged by redesign | ✓ SATISFIED | pricer_section_present_and_unchanged passes; currency_retiering_present passes; SUMMARY confirms git diff on currency section is empty |

**Note:** REQUIREMENTS.md still shows MIG-02/MIG-03/MIG-04 as `[ ]` unchecked. This is a documentation tracking issue only — all three requirements are fully implemented and tested. The checkboxes require manual update by the author.

**Note:** ROADMAP.md shows Phase 6 plan checkboxes (`06-01-PLAN.md`, `06-02-PLAN.md`) as unchecked. Same tracking issue — does not reflect code state.

### Anti-Patterns Found

| File | Line | Pattern | Severity | Impact |
|------|------|---------|----------|--------|
| None found | — | — | — | — |

Scanned: src/emitter.rs, src/run.rs, config.json, tests/cli_generate.rs. No TODOs, FIXMEs, placeholder returns, or empty implementations detected in phase-modified files.

### Human Verification Required

### 1. Author Sign-Off on Policy Block Output (Plan 02 Task 4 checkpoint)

**Test:** Run `ferrisfilter generate --early-game` from the repo root (or inspect the re-baselined `tests/fixtures/seed-universal.filter`) and review the policy blocks at the top of the output.
**Expected:** 13 Show/Hide blocks in rule order: recycle → 7 gear Show bands → 1 gear Hide → 3 flask/charm Show bands → 1 flask/charm Hide. No old `#====` section banners. The `# --- Currency ---` pricer section follows immediately after.
**Why human:** The Plan 02 design explicitly requires a `checkpoint:human-verify` gate (Task 4) for the author to review any non-reorder difference found during development and confirm the policy blocks read as intended. The automated `parity_policy_blocks_match_oracle` test confirms semantic equivalence, but the plan specifies the author must approve before the phase is marked complete.

### Gaps Summary

No gaps found. All automated must-haves are fully implemented and verified. Phase goal is achieved at the code level.

The `human_needed` status reflects the Plan 02 blocking checkpoint gate, not any deficiency in the implementation.

---

_Verified: 2026-06-05T14:30:00Z_
_Verifier: Claude (gsd-verifier)_
