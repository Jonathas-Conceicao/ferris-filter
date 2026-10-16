---
phase: 29-workspace-asset-tool-from-the-root
verified: 2026-10-16T00:00:00Z
status: passed
score: 8/8 must-haves verified
overrides_applied: 0
---

# Phase 29: Workspace & Asset Tool from the Root Verification Report

**Phase Goal:** The repo is one Cargo workspace in which `ff` builds, runs and tests exactly as before, and the asset tool runs from the repo root.
**Status:** passed. **Re-verification:** No.

## Observable Truths

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | SC1: root build/run/test is ff only, no ff-art deps | VERIFIED | Root `Cargo.toml` has `[workspace] members=[".","tools/ff-art"]`, `default-members=["."]`, `resolver="3"`. ff `[package]`, `[[bin]]` and dependency lines are unchanged. The orchestrator ran `cargo test` (1185 pass), `cargo tree` and `cargo build -v` and found no ff-art, poe_data_tools, resvg or image-extras. |
| 2 | SC2: `cargo run -p ff-art` and `cargo ff-art` work from the root | VERIFIED | `.cargo/config.toml` has `ff-art = "run -p ff-art --release --"`. I ran `cargo ff-art ferris` from `src/` and it succeeded. The orchestrator ran `--help` and `list` from `src/`. |
| 3 | SC3: outputs resolve against the repo root from any cwd, assets byte-identical | VERIFIED (by inspection plus ferris) | `repo_root()` in `tools/ff-art/src/main.rs:152` uses `CARGO_MANIFEST_DIR/../..`, not canonicalized. The manifest path, the `extract` out_dir (`root.join("assets/poe2")`) and the ferris svg/out paths are all joined on `root`. `--root` still overrides. After `cargo ff-art ferris` from `src/`, `git status --porcelain assets/ tests/` was empty (ferris-bg.png byte-identical). |
| 4 | One committed root Cargo.lock, no ff-art lock, no `[workspace]` in ff-art | VERIFIED | `git ls-files` lists `Cargo.lock`. `tools/ff-art/Cargo.lock` is absent. `tools/ff-art/Cargo.toml` has no `[workspace]` table. The lock contains poe_data_tools 4.0.1 (730 packages). `cargo metadata --locked` succeeds. `.gitignore` no longer ignores `Cargo.lock`. |
| 5 | `rust-toolchain.toml` pins nightly with rustfmt and clippy | VERIFIED | File present, tracked. Fmt clean and tests green per the orchestrator. |

## Requirements Coverage

| Requirement | Status | Evidence |
|-------------|--------|----------|
| WS-01 (workspace, ff default) | SATISFIED | Truths 1, 4, 5. |
| WS-02 (asset tool from root, alias) | SATISFIED | Truths 2, 3. |

No orphaned requirements: REQUIREMENTS.md maps only WS-01 and WS-02 to Phase 29, and both are claimed by 29-01-PLAN.

## Anti-Patterns

No TBD, FIXME or XXX markers in `tools/ff-art/src/main.rs`. No blockers.

## Human Verification

None required. A real `ff-art extract` was deliberately not run, because it needs the game install and would rewrite committed assets. Its path logic is the same `root`-joined code that `ferris` exercises, so the residual risk is low. Optionally, run `cargo ff-art extract` from a subdirectory once and confirm `git status assets/` stays clean.

## Gaps Summary

None. The phase goal is achieved.

_Verifier: Claude (gsd-verifier)_
