---
phase: 29-workspace-asset-tool-from-the-root
plan: 01
subsystem: build
tags: [cargo, workspace, ff-art, toolchain]
requires: []
provides:
  - Root Cargo workspace (ff default member, ff-art member)
  - Committed root Cargo.lock, nightly rust-toolchain.toml, cargo ff-art alias
  - ff-art default paths resolved from the repo root regardless of cwd
affects: [build, docs]
tech-stack:
  added: []
  patterns: [workspace default-members, cargo alias]
key-files:
  created: [rust-toolchain.toml, .cargo/config.toml, Cargo.lock]
  modified: [Cargo.toml, tools/ff-art/Cargo.toml, tools/ff-art/src/main.rs, .gitignore, README.md, CLAUDE.md, CHANGELOG.md]
decisions:
  - "No [workspace.dependencies]: the crates share only image/clap/serde with different features"
  - "Alias uses --release because DDS extraction is slow in debug"
  - "repo_root() uses two parent() calls, not canonicalize, to avoid verbatim paths on Windows"
metrics:
  tasks: 3
  completed: 2026-10-16
---

# Phase 29 Plan 01: Workspace and asset tool from the root Summary

The repo root is now one Cargo workspace with `ff` as the only default member, and `ff-art` runs from any folder via `cargo ff-art` or `cargo run -p ff-art`, writing into the repo's `assets/`.

## Commits
- 745ab25 build(29): workspace, root lock, nightly pin, alias
- 1b12f5b fix(29): ff-art resolves default paths from the repo root
- 34c19ef docs(29): README, CLAUDE.md, CHANGELOG

## Verification
- `workspace_default_members` is only ferrisfilter; `cargo build -v` has 0 lines for poe_data_tools/resvg/image-extras/ff-art; `cargo tree -p ferrisfilter` has 0 matches.
- `cargo test`: all green (1089 unit tests plus integration suites, 0 failed); fixtures unchanged.
- `cargo fmt --all --check` clean. Clippy warnings: 130 baseline (`-p ferrisfilter`) vs 130 (`--workspace`), none new.
- `cargo ff-art --help` OK from root and `src/`; `cargo ff-art list optionsmenu` from `src/` printed virtual paths; `cargo run -p ff-art -- --help` OK.
- `cargo ff-art ferris` run from `src/` produced a byte-identical `assets/ui/ferris-bg.png` (`git status assets/` empty).

## Deviations from Plan

**1. [Rule 3 - Blocking] Lockfile merge needed manual construction**
- **Issue:** a fresh resolve of `image-extras` failed ("version unavailable"), and naively unioning the two old locks conflicted on lockstep crates.
- **Fix:** merged the old ff-art lock into the old root lock with a scratchpad script (ff-art versions only added where no semver-compatible root version existed), then let cargo resolve conflicts iteratively.
- **Result:** ff's tree drifted by semver-compatible patch bumps that ff-art's deps require: futures-channel/core/executor/macro/task/util 0.3.32 to 0.3.34, crc32fast 1.5.1 to 1.5.2, serde_json 1.0.150 to 1.0.154. All other old root name+version pairs are unchanged. Tests are green with these.
- **Commit:** 745ab25

## Known Stubs
None.

## Self-Check: PASSED
