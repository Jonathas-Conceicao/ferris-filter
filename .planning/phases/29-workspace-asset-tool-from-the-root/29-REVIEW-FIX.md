---
phase: 29-workspace-asset-tool-from-the-root
fixed_at: 2026-10-16T00:00:00Z
review_path: .planning/phases/29-workspace-asset-tool-from-the-root/29-REVIEW.md
iteration: 1
findings_in_scope: 3
fixed: 3
skipped: 0
status: all_fixed
---

# Phase 29: Code Review Fix Report

**Fixed at:** 2026-10-16
**Source review:** .planning/phases/29-workspace-asset-tool-from-the-root/29-REVIEW.md
**Iteration:** 1

**Summary:**
- Findings in scope: 3 (WR-01, plus IN-01 and IN-02 by orchestrator instruction)
- Fixed: 3
- Skipped: 0 in scope (IN-03 and IN-04 out of scope, listed below)

## Fixed Issues

### WR-01: `rust-toolchain.toml` uses a floating `nightly`, so it pins nothing

**Files modified:** `README.md`, `CLAUDE.md`, `CHANGELOG.md`
**Commit:** 34e3a85
**Applied fix:** `rust-toolchain.toml` is unchanged: the CONTEXT locks `channel = "nightly"`, and
the author wants everything on nightly. Only the wording changed. The docs now say the file
*selects the nightly channel* rather than pinning a compiler. README and CLAUDE.md also note that a
dated nightly can be set there if a new nightly breaks `poe_data_tools`.

### IN-01: The ff-art module doc still says the crate is "its own workspace"

**Files modified:** `tools/ff-art/src/main.rs`
**Commit:** c488eca
**Applied fix:** The module doc now says the crate is a non-default member of the repo-root
workspace. `cargo fmt --all --check` and `cargo build -p ff-art` both pass, with `CARGO_TARGET_DIR`
outside the repo.

### IN-02: The `[Unreleased]` CHANGELOG entry still describes ff-art as "own workspace"

**Files modified:** `CHANGELOG.md`
**Commit:** 4de78a3
**Applied fix:** Removed "own workspace," from the prototype entry. The Phase 29 entry already
describes the workspace layout.

## Skipped Issues

### IN-03: The `repo_root()` "fix" does not change behaviour

**File:** `tools/ff-art/src/main.rs:150-158,164`
**Reason:** Out of scope per orchestrator. The review says no code change is required.
**Original issue:** The old `CARGO_MANIFEST_DIR/../..` default never depended on the working
directory either, so the commit message overstates the change. Both versions bind the root at
compile time.

### IN-04: Every subcommand loads the manifest, even ones that never use it

**File:** `tools/ff-art/src/main.rs:166-171`
**Reason:** Out of scope per orchestrator. The behaviour predates this phase.
**Original issue:** `main` parses `manifest.toml` before dispatch, so `list`/`dump`/`sheet`/`info`/`zoom`
fail when the manifest is broken or `--root` is wrong.

---

_Fixed: 2026-10-16_
_Fixer: Claude (gsd-code-fixer)_
_Iteration: 1_
