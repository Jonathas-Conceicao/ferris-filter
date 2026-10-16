---
phase: 29-workspace-asset-tool-from-the-root
reviewed: 2026-10-16T00:00:00Z
depth: standard
files_reviewed: 9
files_reviewed_list:
  - Cargo.toml
  - .cargo/config.toml
  - rust-toolchain.toml
  - .gitignore
  - tools/ff-art/Cargo.toml
  - tools/ff-art/src/main.rs
  - README.md
  - CLAUDE.md
  - CHANGELOG.md
findings:
  critical: 0
  warning: 1
  info: 4
  total: 5
status: issues_found
---

# Phase 29: Code Review Report

**Reviewed:** 2026-10-16
**Depth:** standard
**Files Reviewed:** 9
**Status:** issues_found

## Summary

I reviewed `git diff 1f20867..HEAD` for the workspace conversion: the root `[workspace]` with
`default-members = ["."]`, the removed standalone `[workspace]` in `tools/ff-art/Cargo.toml`, the
`cargo ff-art` alias, `rust-toolchain.toml`, the `.gitignore` change that lets `Cargo.lock` be
committed, the `repo_root()` change in ff-art, and the doc updates.

What I checked:
- `cargo metadata --locked` succeeds, so the committed `Cargo.lock` matches both manifests.
- `cargo fmt --all --check` is clean, including ff-art.
- `rustup show active-toolchain` confirms `rust-toolchain.toml` takes effect.
- No stray `tools/ff-art/Cargo.lock` or `tools/ff-art/target/` is left behind.
- The alias expands correctly, including global flags: `cargo ff-art --game X` becomes
  `run -p ff-art --release -- --game X`.
- `cargo run` keeps the caller's working directory, so the relative `dump <dir>`, `sheet` and
  `zoom` arguments still resolve as the user expects.

I found no blockers. The one real weakness is that the toolchain pin floats. The rest are stale
wording and small notes.

## Warnings

### WR-01: `rust-toolchain.toml` uses a floating `nightly`, so it pins nothing and now applies to the shipped `ff` build

**File:** `rust-toolchain.toml:2` (also described as "pinned" in `README.md:37`, `CLAUDE.md:46`, `CHANGELOG.md:39`)
**Issue:** `channel = "nightly"` points to whatever nightly rustup has installed or fetches. This
causes three problems:
1. The phase commits `Cargo.lock` for reproducible builds, but the compiler still changes from
   day to day. Two machines, or the same machine a month apart, can build `ff` with different
   compilers. A nightly regression or new deny-by-default lint can break `cargo test` or
   `artifact_guard` without any repo change.
2. The file covers the whole workspace, so the release overlay `ff` now builds on an unpinned
   unstable compiler. Before this phase it built on whatever the machine default was. `ff` does
   not need nightly; only ff-art's `poe_data_tools` does.
3. `poe_data_tools` 4.0.1 uses `#![feature(...)]`. If a later nightly removes or changes that
   feature, `cargo ff-art` stops compiling. The tool runs rarely (after game patches), so this
   will show up long after the cause. A dated pin is the usual fix for nightly-only dependencies.

The docs say nightly is "pinned", which suggests a reproducibility the file does not give.
**Fix:** Pin a dated nightly that is known to build both packages. Bump it on purpose.
```toml
[toolchain]
channel = "nightly-2026-10-15"   # bump deliberately; must build ff and ff-art (poe_data_tools #![feature])
components = ["rustfmt", "clippy"]
```
Then change the doc wording to "pinned to a dated nightly". If that is too restrictive, at least
change "pinned" to "selects nightly" in README, CLAUDE.md and CHANGELOG so the docs do not
overstate it.

## Info

### IN-01: The ff-art module doc still says the crate is "its own workspace"

**File:** `tools/ff-art/src/main.rs:9`
**Issue:** `//! This crate is its own workspace and is never a dependency of the ff overlay`. This is
no longer true: the `[workspace]` table was removed and the crate is now a root workspace member.
The diff updated the comments in `Cargo.toml` and the README but missed this one.
**Fix:** `//! This crate is a non-default member of the repo-root workspace and is never a dependency of the ff overlay; ...`

### IN-02: The `[Unreleased]` CHANGELOG entry still describes ff-art as "own workspace"

**File:** `CHANGELOG.md:22`
**Issue:** The prototype entry under the same `[Unreleased]` section says `tools/ff-art` is
"(own workspace, ...)". The new Phase 29 entry a few lines below says the opposite. When the
milestone ships, both lines land in one version section and contradict each other.
**Fix:** Drop "own workspace," from the line-22 entry. The Phase 29 entry already covers the
current layout.

### IN-03: The `repo_root()` "fix" does not change behaviour

**File:** `tools/ff-art/src/main.rs:150-158,164`
**Issue:** The old default, `Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")`, was already
independent of the working directory: `CARGO_MANIFEST_DIR` is an absolute compile-time path. The
new code only removes the `..` components from displayed paths. Commit `1b12f5b` ("resolves its
default paths from the repo root from any cwd") suggests a bug was fixed, but none existed. The
`.expect()` cannot fire in practice. No code change is needed. Note it so nobody later assumes a
cwd bug was present.

Both versions share a limitation: the root is baked in at compile time. A `target/release/ff-art.exe`
copied elsewhere, or left over after the repo moved, writes into the old location unless `--root`
is passed. That is acceptable for a dev tool run through `cargo ff-art`, which always rebuilds.
**Fix:** None required. Optionally, mention the compile-time binding in the `--root` help text.

### IN-04: Every subcommand loads the manifest, even ones that never use it

**File:** `tools/ff-art/src/main.rs:166-171`
**Issue:** `main` reads and parses `assets/poe2/manifest.toml` before dispatching. `list`, `dump`,
`sheet`, `info` and `zoom` never use it. A wrong `--root`, a stale baked-in root (see IN-03), or a
manifest broken mid-edit therefore makes unrelated inspection commands fail. That gets in the way
when you are using `list`/`dump` to fix the manifest. This predates the phase, but the phase now
advertises "runs from any folder".
**Fix:** Load the manifest lazily, only in the `Extract` and `Ferris` arms:
```rust
let load_manifest = || -> Result<Manifest> { /* read + parse manifest_path */ };
match cli.cmd.unwrap_or(Cmd::Extract) {
    Cmd::Extract => { let m = load_manifest()?; let fs = open_game(&cli.game)?; extract(&fs, &m, &root)?; ferris(&m, &root) }
    Cmd::Ferris => ferris(&load_manifest()?, &root),
    // other arms unchanged, no manifest
    ...
}
```

---

_Reviewed: 2026-10-16_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
