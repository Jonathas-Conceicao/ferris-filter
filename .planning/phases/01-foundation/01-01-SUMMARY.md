---
phase: 01-foundation
plan: 01
subsystem: cli
tags: [rust, clap, serde, miette, thiserror, serde_path_to_error]

requires: []

provides:
  - ferrisfilter Rust crate with Cargo.toml (dual [[bin]] targets ferrisfilter + ff)
  - clap 4.6 derive CLI shell: Cli struct + Commands enum (Generate/Sync)
  - Generate subcommand with --output flag (default universal.filter)
  - JsonLoadError: thiserror+miette Diagnostic with source_code and label for field-path errors
  - load_json<T>: generic serde_path_to_error+miette JSON loader helper
  - src/models/ skeleton: mod.rs + placeholder config.rs/currency.rs/prices.rs/tiers.rs

affects: [01-02, 01-03, phase-2-generate, phase-4-sync]

tech-stack:
  added:
    - clap 4.6 (derive feature)
    - serde 1.0 (derive feature)
    - serde_json 1.0
    - serde_path_to_error 0.1
    - miette 7.6 (fancy feature)
    - thiserror 2.0
    - assert_cmd 2.2 (dev)
    - predicates 3.1 (dev)
  patterns:
    - Dual [[bin]] in Cargo.toml (ferrisfilter + ff) both pointing to src/main.rs
    - miette::Result main() for graphical error rendering + exit code 1 on Err
    - serde_path_to_error::deserialize wrapping serde_json::Deserializer for field-path errors
    - JsonLoadError struct with #[source_code] NamedSource<String> + #[label] SourceOffset

key-files:
  created:
    - Cargo.toml
    - src/main.rs
    - src/cli.rs
    - src/error.rs
    - src/loader.rs
    - src/models/mod.rs
    - src/models/config.rs
    - src/models/currency.rs
    - src/models/prices.rs
    - src/models/tiers.rs
  modified:
    - .gitignore (added /target/ and Cargo.lock)

key-decisions:
  - "edition = '2024' chosen — stabilized in Rust 1.85 (Feb 2025), running on nightly 1.90.0"
  - "Dual [[bin]] via shared src/main.rs produces expected Cargo warning (documented in RESEARCH Pitfall 4) — intentional"
  - "load_json<T> is generic (not config-specific) so downstream plans (02, 03) can call it for any JSON file"
  - "Model placeholder files contain only a comment so cargo build passes while Plans 02/03 own their content"

patterns-established:
  - "Pattern: Cargo dual [[bin]] with same path — both compile from src/main.rs, Cargo emits harmless warning"
  - "Pattern: miette::Result main() — error propagation exits code 1 with graphical diagnostic on stderr"
  - "Pattern: serde_path_to_error wrapping — always use for human-edited JSON to get dotted field paths"

requirements-completed: [CLI-01, CLI-03, CLI-05]

duration: 3min
completed: 2026-05-24
---

# Phase 1 Plan 01: Crate Skeleton Summary

**Greenfield ferrisfilter Rust crate with clap derive CLI (generate/sync stubs), miette+thiserror JsonLoadError diagnostic type, and serde_path_to_error load_json helper**

## Performance

- **Duration:** ~3 min
- **Started:** 2026-05-24T01:00:02Z
- **Completed:** 2026-05-24T01:02:24Z
- **Tasks:** 2
- **Files created:** 11, modified: 1

## Accomplishments
- Cargo.toml with dual `[[bin]]` targets (`ferrisfilter` + `ff`) and all verified dependency versions (clap 4.6, miette 7.6 fancy, thiserror 2.0, serde_path_to_error 0.1)
- Fully navigable CLI: `ferrisfilter --help` lists `generate` + `sync`; `generate --help` shows `--output`; unknown subcommand exits code 2
- JsonLoadError diagnostic struct (miette + thiserror) with `#[source_code]` and `#[label]` for rich JSON parse error rendering
- Generic `load_json<T>` helper using `serde_path_to_error::deserialize` — foundation every downstream loader builds on
- Module skeleton with four model file stubs — Plans 02 and 03 can overwrite disjoint files without touching this plan's artifacts

## Task Commits

Each task was committed atomically:

1. **Task 1: Cargo manifest with dual binaries and verified deps** - `099a6b1` (chore)
2. **Task 2: clap CLI shell + error type + loader helper + module skeleton** - `4de1da1` (feat)

**Plan metadata:** (SUMMARY commit — see below)

## Files Created/Modified
- `Cargo.toml` - Crate manifest: edition 2024, dual [[bin]], all 6 runtime deps + 2 dev deps
- `src/main.rs` - miette::Result main(), Cli::parse(), generate/sync stub dispatch
- `src/cli.rs` - clap derive Cli + Commands enum with Generate{--output} and Sync
- `src/error.rs` - JsonLoadError: thiserror+miette Diagnostic with source_code and label fields
- `src/loader.rs` - Generic load_json<T: DeserializeOwned> using serde_path_to_error
- `src/models/mod.rs` - pub mod declarations for config/currency/prices/tiers
- `src/models/config.rs` - Placeholder (// populated in plan 02)
- `src/models/currency.rs` - Placeholder (// populated in plan 02)
- `src/models/prices.rs` - Placeholder (// populated in plan 03)
- `src/models/tiers.rs` - Placeholder (// populated in plan 03)
- `.gitignore` - Added /target/ and Cargo.lock entries

## Decisions Made
- `edition = "2024"` — stabilized Feb 2025 in Rust 1.85, correct for new 2025+ projects
- Dual `[[bin]]` via shared `src/main.rs` — intentionally produces harmless Cargo "file found in multiple build targets" warning per RESEARCH Pitfall 4; no restructure needed
- `load_json<T>` is generic, not bound to a specific type, so Plans 02/03 can call it directly
- Placeholder model files contain only a comment (empty modules compile fine) — Plans 02/03 overwrite their own files

## Deviations from Plan

None — plan executed exactly as written. All patterns followed verbatim from RESEARCH.md verified code examples.

## Known Stubs

The following items are intentional stubs for this plan (not defects):

| File | Stub | Reason |
|------|------|--------|
| `src/main.rs` | `println!("generate not yet implemented...")` | Phase 2 wires the generator |
| `src/main.rs` | `println!("sync not yet implemented")` | Phase 4 wires poe.ninja fetching |
| `src/models/config.rs` | `// populated in plan 02` | Plan 02 owns this file |
| `src/models/currency.rs` | `// populated in plan 02` | Plan 02 owns this file |
| `src/models/prices.rs` | `// populated in plan 03` | Plan 03 owns this file |
| `src/models/tiers.rs` | `// populated in plan 03` | Plan 03 owns this file |

These stubs are the explicit design of this scaffolding plan and do not prevent the plan goal (buildable, navigable CLI) from being achieved.

## Issues Encountered

None — `cargo build` succeeded on first attempt. The only warnings are expected and documented:
- `warning: file 'src/main.rs' found to be present in multiple build targets` — Pitfall 4 in RESEARCH, harmless
- `warning: struct 'JsonLoadError' is never constructed` — expected, no caller yet in Phase 1
- `warning: function 'load_json' is never used` — expected, no caller yet in Phase 1

## Verification Results

All acceptance criteria passed:
- `cargo build` exits 0
- `ferrisfilter --help` stdout contains `generate` and `sync`
- `ferrisfilter generate --help` stdout contains `--output`
- `ferrisfilter nonexistent` exits code 2 with "unrecognized subcommand" in stderr
- `ff --help` exits 0 (alias binary builds and runs correctly)
- `src/error.rs` contains `JsonLoadError` and `source_code`
- `src/loader.rs` contains `serde_path_to_error` and `DeserializeOwned`
- `src/main.rs` contains `miette::Result` and `Cli::parse`

## Threat Surface Scan

No new security-relevant surface beyond the plan's threat model. Files created establish:
- `load_json` reads local filesystem (T-01-01: mitigated via serde_path_to_error returning Err, no unwrap)
- `--output` path accepted from user argv (T-01-03: accepted for Phase 1, generate is a stub)

No additional threats identified.

## Next Phase Readiness
- Plans 01-02 and 01-03 can now run in parallel (Wave 2) against disjoint files
- 01-02 targets `src/models/config.rs` + `src/models/currency.rs`
- 01-03 targets `src/models/tiers.rs` + `src/models/prices.rs`
- Neither plan needs to touch `Cargo.toml`, `src/main.rs`, `src/cli.rs`, `src/error.rs`, or `src/loader.rs`
- `load_json<T>` interface is stable and ready for downstream loaders to call

---
*Phase: 01-foundation*
*Completed: 2026-05-24*
