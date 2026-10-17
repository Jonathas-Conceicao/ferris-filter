---
phase: 35-prototype-retirement-docs
status: human_needed
score: 3/3
verified: 2026-10-17
---
# Phase 35 Verification

| # | Success criterion | Status | Evidence |
|---|---|---|---|
| 1 | `ff-proto` and `overlay::theme` gone, build + tests green, `ff-art` and `assets/` remain | VERIFIED | `src/bin` holds `ff.rs`, `ff-cli.rs` only; no `theme` module; grep for ff-proto / overlay::theme / Cinzel clean outside CHANGELOG history; `tools/ff-art`, `assets/poe2`, `assets/custom` present; `cargo test` 1389 lib + all integration suites pass; 35-01 before/after pixel diff 0 on 12 pairs |
| 2 | README, CLAUDE.md, CHANGELOG describe the new window, tabs, dividers, economy banner, workspace, asset tool | VERIFIED | README rewritten with `docs/screenshots/` (5); CLAUDE.md `ff-proto` row and theme/Cinzel mentions removed; CHANGELOG `[Unreleased]` tidied |
| 3 | Cargo version `0.6.0` | VERIFIED | `Cargo.toml` / `Cargo.lock` |

Requirements: CLEAN-01, CLEAN-02 satisfied.

## Human verification
1. Read README.md with its screenshots (placeholder; user rewrites it by hand later).
2. Optional: launch `ff` and confirm it looks the same as before the theme move.
