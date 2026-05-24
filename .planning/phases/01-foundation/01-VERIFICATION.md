---
phase: 01-foundation
verified: 2026-05-23T00:00:00Z
status: passed
score: 5/5 must-haves verified
overrides_applied: 0
---

# Phase 1: Foundation Verification Report

**Phase Goal:** The Rust crate exists, compiles, and can be invoked as a CLI with both subcommands visible; all JSON input shapes are modeled and round-trip through serde.
**Verified:** 2026-05-23
**Status:** PASSED
**Re-verification:** No — initial verification

---

## Goal Achievement

### Observable Truths (ROADMAP Success Criteria)

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | `cargo build` succeeds and `ferrisfilter --help` lists `generate` and `sync` subcommands | ✓ VERIFIED | `cargo build` exits 0 (only expected dual-bin warning); `ferrisfilter --help` stdout contains both "generate" and "sync"; `ff --help` also exits 0. |
| 2 | `ferrisfilter generate --help` shows `--output <path>` flag documentation | ✓ VERIFIED | `generate --help` stdout: `--output <OUTPUT>  Output file path [default: universal.filter]`. Cosmetic double-default is a known clap derive artefact; functionally correct. |
| 3 | `ferrisfilter generate` with a missing or malformed `config.json` exits non-zero and prints a message containing the bad field path AND expected type | ✓ VERIFIED | Empirically tested: `"font": "BROKEN"` produces `styles.S.font: invalid type: string "BROKEN", expected u8` with miette source-span rendering; missing `config.json` produces `Cannot read \`config.json\``; both exit 1. The `cli_errors` integration test suite (4 tests) covers all these paths and passes. |
| 4 | `tiers.json` is validated on load and produces an actionable error when a cutoff is missing or out of order | ✓ VERIFIED | `ValidatedTiers::try_from` enforces completeness (all 7 tiers S–F) and within-unit ascending order. Empirically tested: missing tier B → `tiers.json missing required tier(s): B`; non-ascending `D min=0.05 < E min=0.5` → `tiers.json bounds not strictly ascending …`. Integration tests `missing_tier_in_tiers_json_exits_nonzero` and `non_ascending_tiers_json_exits_nonzero` pass. |
| 5 | `ferrisfilter nonexistent` exits non-zero with a clear unknown-command message | ✓ VERIFIED | Empirically: exits code 2 with `error: unrecognized subcommand 'nonexistent'`. Integration test `unknown_subcommand_exits_code_2` passes. |

**Score:** 5/5 truths verified

---

### Required Artifacts

| Artifact | Plan | Status | Details |
|----------|------|--------|---------|
| `Cargo.toml` | 01-01 | ✓ VERIFIED | Dual `[[bin]]` (ferrisfilter + ff), 6 runtime deps (clap 4.6, serde 1.0, serde_json 1.0, serde_path_to_error 0.1, miette 7.6, thiserror 2.0), 2 dev deps (assert_cmd 2.2, predicates 3.1), `edition = "2024"`. |
| `src/cli.rs` | 01-01 | ✓ VERIFIED | `#[derive(Parser)] Cli` + `#[derive(Subcommand)] Commands` with `Generate { --output }` and `Sync` variants; uses `Subcommand`. |
| `src/error.rs` | 01-01 | ✓ VERIFIED | `JsonLoadError` with `#[source_code]`, `#[label]`, `NamedSource<String>`, `SourceOffset`; derives `Diagnostic` via miette. |
| `src/loader.rs` | 01-01 / 01-03 | ✓ VERIFIED | Generic `load_json<T: DeserializeOwned>` using `serde_path_to_error::deserialize`; `load_tiers` wrapper calling `ValidatedTiers::try_from`. |
| `src/main.rs` | 01-01 / 01-03 | ✓ VERIFIED | `miette::Result` main, `Cli::parse()`, dispatches to `cmd_generate` (loads `Config` + `ValidatedTiers`). Uses full typed `Config` — the `serde_json::Value` deviation noted in 01-03-SUMMARY was resolved at merge time. |
| `src/models/mod.rs` | 01-01 | ✓ VERIFIED | Declares all four sub-modules: `config`, `currency`, `prices`, `tiers`. |
| `config.json` | 01-02 | ✓ VERIFIED | Renamed from `filter-model.json` via `git mv` (rename recorded in git as `filter-model.json => config.json`). Contains `styles`, `build`, `areaLevelBands`, `alwaysShow`. |
| `currency.json` | 01-02 | ✓ VERIFIED | 15 currency categories extracted from `universal.filter`; contains `categories`; Mirror of Kalandra present in Currency S; parses and round-trips. |
| `src/models/config.rs` | 01-02 | ✓ VERIFIED | Full `Config` model with `deny_unknown_fields`, `FilterColor`/`MinimapShape`/`StyleAction` enums, `Option<[u8;3]>` RGB triples, 6 inline tests. No `#[serde(flatten)]`. |
| `src/models/currency.rs` | 01-02 | ✓ VERIFIED | `CurrencyData`, `CurrencyCategory`, `CurrencyEntry` all with `deny_unknown_fields`; 5 inline tests including 15-category count and Mirror sanity. |
| `tiers.json` | 01-03 | ✓ VERIFIED | 7 ordered entries covering S–F, split into `div` (A=1.0, S=5.0) and `chaos` (F=0.1 → B=30.0) groups; within-unit ascending; contains `unit` key. |
| `prices.json` | 01-03 | ✓ VERIFIED | `{ league, fetchedAt, items }` with first 3 items = Exalt/Chaos/Divine, plus Mirror with `baseType`; nullable `exalt`/`chaos`/`div` fields. |
| `src/models/tiers.rs` | 01-03 | ✓ VERIFIED | `Unit` enum, `TierEntry`, `ValidatedTiers`, `TierValidationError` (MissingTiers + NotAscending), `TryFrom<Vec<TierEntry>>` validation; 5 inline tests. |
| `src/models/prices.rs` | 01-03 | ✓ VERIFIED | `PricesFile` + `PriceItem` with `deny_unknown_fields`, `camelCase`, `Option<f64>`; 4 inline tests. |
| `tests/cli_generate.rs` | 01-03 | ✓ VERIFIED | 4 assert_cmd integration tests: `--help` subcommands, `generate --help --output`, unknown-subcommand exits code 2, `generate` in repo root succeeds. |
| `tests/cli_errors.rs` | 01-03 | ✓ VERIFIED | 4 assert_cmd integration tests: missing config, malformed config (field path + expected type), missing tier, non-ascending tiers. All pass. |

---

### Key Link Verification

| From | To | Via | Status | Details |
|------|----|-----|--------|---------|
| `src/main.rs` | `src/cli.rs` | `Cli::parse()` then match on `Commands` | ✓ WIRED | `use cli::{Cli, Commands}; Cli::parse()` in main; match arms dispatch to `cmd_generate`/`cmd_sync`. |
| `src/main.rs` | `src/loader.rs` | `loader::load_json::<Config>` + `loader::load_tiers` | ✓ WIRED | Both calls present; return values used (`config.styles.len()`, `tiers.0.len()`). |
| `src/error.rs` | miette | `#[source_code]` + `#[label]` + `Diagnostic` derive | ✓ WIRED | `JsonLoadError` derives `Diagnostic` from miette; `NamedSource<String>` and `SourceOffset` fields attached. Confirmed by live miette error rendering in criterion 3 test. |
| `src/loader.rs` | `src/error.rs` | `JsonLoadError { … }.into()` on parse failure | ✓ WIRED | `serde_path_to_error::deserialize` failure constructs `JsonLoadError` and calls `.into()` via miette's `From<E: Diagnostic>`. |
| `src/models/tiers.rs` | `ValidatedTiers::try_from` | Completeness + within-unit ascending check | ✓ WIRED | `TryFrom<Vec<TierEntry>>` impl present; returns `Err(TierValidationError::MissingTiers)` or `Err(TierValidationError::NotAscending { … })`; called from `loader::load_tiers`. |
| `tests/cli_errors.rs` | `ferrisfilter generate` binary | `Command::cargo_bin("ferrisfilter")` subprocess | ✓ WIRED | Uses `assert_cmd`; drives binary against temp-dir fixtures; `cargo_bin` present; field-path assertions pass. |
| `config.json` round-trip | `src/models/config.rs` | `serde_json::from_str::<Config>` | ✓ WIRED | `config_parses_real_file` + `config_round_trips` unit tests pass; `generate_in_repo_root_succeeds` integration test passes. |

---

### Data-Flow Trace (Level 4)

Not applicable. Phase 1 produces no rendering or output pipeline — `cmd_generate` is an intentional stub that prints a message. The data that flows (Config, ValidatedTiers) is used to print style/tier counts, confirming both structs are populated from real files. No hollow props or static data path.

---

### Behavioral Spot-Checks

| Behavior | Command | Result | Status |
|----------|---------|--------|--------|
| `cargo build` exits 0 | `cargo build` | Finished dev profile; no errors; only expected dual-bin warning | ✓ PASS |
| `--help` lists both subcommands | `ferrisfilter --help` | stdout contains "generate" and "sync" | ✓ PASS |
| `generate --help` shows `--output` | `ferrisfilter generate --help` | stdout contains `--output <OUTPUT>  Output file path [default: universal.filter]` | ✓ PASS |
| Unknown subcommand exits 2 | `ferrisfilter nonexistent` | stderr `unrecognized subcommand 'nonexistent'`; exit 2 | ✓ PASS |
| `ff` alias binary works | `ff --help` | exits 0, same help text | ✓ PASS |
| `generate` with valid inputs succeeds | `ferrisfilter generate` (repo root) | `generate not yet implemented (output: universal.filter, 10 styles, 7 tiers)`; exit 0 | ✓ PASS |
| Malformed config field-path + type error | `ferrisfilter generate` (font="BROKEN") | `styles.S.font: invalid type: string "BROKEN", expected u8`; exit 1 | ✓ PASS |
| Missing config exits non-zero | `ferrisfilter generate` (no config.json) | `Cannot read \`config.json\``; exit 1 | ✓ PASS |
| Missing tier validation error | `ferrisfilter generate` (tiers.json missing B) | `tiers.json missing required tier(s): B`; exit 1 | ✓ PASS |
| All tests pass | `cargo test` | 29/29 passed (21 unit, 4 cli_errors, 4 cli_generate); 0 failed | ✓ PASS |

---

### Requirements Coverage

| Requirement | Plan | Description | Status | Evidence |
|-------------|------|-------------|--------|----------|
| CLI-01 | 01-01 | User can run `ferrisfilter generate` | ✓ SATISFIED | Binary built; `ferrisfilter generate` exits 0 with valid inputs. |
| CLI-03 | 01-01 | `generate` accepts `--output <path>` | ✓ SATISFIED | `--output` flag present in `cli.rs`; `generate --help` documents it. |
| CLI-05 | 01-01 / 01-03 | Every subcommand exposes `--help`; exits non-zero on failure | ✓ SATISFIED | `--help` works on both binary and `generate`/`sync`; unknown subcommand exits code 2; all error paths exit 1. |
| CFG-01 | 01-02 | Filter policy lives in `config.json` (renamed from `filter-model.json`) | ✓ SATISFIED | `config.json` present, git-renamed, contains `styles`/`build`/`areaLevelBands`/`alwaysShow`; deserialized via typed `Config`. |
| CFG-02 | 01-03 | `generate` validates `config.json` and reports actionable errors (field path + expected) | ✓ SATISFIED | `serde_path_to_error` + miette produce `styles.S.font: invalid type: string "BROKEN", expected u8`; integration tests prove it. |
| CFG-03 | 01-03 | Tier cutoffs live in `tiers.json` and are validated on load | ✓ SATISFIED | `ValidatedTiers::try_from` validates completeness + ordering; `load_tiers` wired into `cmd_generate`; integration tests prove both error cases. |

All 6 Phase 1 requirements satisfied. No orphaned requirements (CLI-02, CLI-04 are Phase 4; all Generate/Tier/Sync requirements are Phase 2–4).

---

### Anti-Patterns Found

| File | Pattern | Severity | Impact |
|------|---------|----------|--------|
| `src/main.rs:30` | `println!("generate not yet implemented …")` | ℹ️ Info | Intentional stub per scope note; Phase 2 replaces with actual filter generation. |
| `src/main.rs:38` | `println!("sync not yet implemented")` | ℹ️ Info | Intentional stub; Phase 4 scope. |
| `src/cli.rs:19` | `/// Fetch live PoE2 prices from poe.ninja (not yet implemented)` | ℹ️ Info | Doc-comment accurately reflects Phase 4 stub. |
| `generate --help` | `[default: universal.filter]` appears twice | ℹ️ Info | Cosmetic clap derive artefact (both `default_value` attr and clap's doc-comment rendering emit the default). No functional impact. |

No blockers. No warnings. All stub patterns are in explicitly deferred scope (generate body = Phase 2, sync = Phase 4).

---

### Human Verification Required

None. All 5 ROADMAP success criteria were verified programmatically by running the compiled binary directly and through `cargo test`.

---

## Gaps Summary

No gaps. All 5 ROADMAP success criteria and all 6 requirement IDs (CLI-01, CLI-03, CLI-05, CFG-01, CFG-02, CFG-03) are satisfied by the codebase as it stands.

The one noted deviation from plan (Plan 03 used `serde_json::Value` in `main.rs` during the parallel wave-2 worktree) was fully resolved at merge time — `main.rs` uses the full typed `Config` in the merged codebase, which is the only state that matters.

---

_Verified: 2026-05-23_
_Verifier: Claude (gsd-verifier)_
