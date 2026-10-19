---
phase: 40-release-pipeline-installer
plan: 04
subsystem: build
tags: [winresource, windows-resource, icon, versioninfo]
requires: [40-02, 40-03]
provides: [ff.exe Ferris icon and VERSIONINFO]
affects: [release workflow, installer]
tech-stack:
  added: [winresource 0.1.34 (build-dependency, default-features = false)]
key-files:
  modified: [Cargo.toml, Cargo.lock, build.rs]
requirements: [REL-01, INST-02]
metrics:
  completed: 2026-10-18
---

# Phase 40 Plan 04: Exe icon and version info Summary

`ff.exe` now links the Ferris icon and a VERSIONINFO resource through `winresource`; `ff-cli.exe` and `ferrisfilter.exe` carry none.

## Checkpoint result (Task 1, winresource legitimacy)

The author pre-authorised resolving the checkpoint through `cargo info` (2026-10-18). Output of
`cargo info winresource@0.1.34` (the unversioned form printed the same):

```
winresource
Create and set windows icons and metadata for executables
version: 0.1.34
license: MIT
documentation: https://docs.rs/winresource/*/winresource/
repository: https://github.com/BenjaminRi/winresource
crates.io: https://crates.io/crates/winresource/0.1.34
features: +default = [toml], toml = [dep:toml]
```

Matches the expected crate, repository and license. Cargo.toml and Cargo.lock were unchanged at that point.

## Tasks

1. Legitimacy check: resolved as above (no commit).
2. `embed_resource()` in build.rs, `[build-dependencies]` in Cargo.toml: commit 14837f6.

## Verification

- `FF_REQUIRE_RESOURCE=1 cargo build --locked` passed (CARGO_TARGET_DIR outside the repo).
- `installer/verify-exe.sh <ff.exe> 0.6.0` printed `ok: ... carries FerrisFilter 0.6.0`.
- ProductNameFerrisFilter count: ff-cli.exe 0, ferrisfilter.exe 0.
- `compile_for("ff")` 1, winresource in Cargo.lock 1, `set_manifest` 0, `[profile.release]` untouched.
- `cargo test --locked --lib build_info`: 6 passed.
- Observed version info in ff.exe: ProductVersion/FileVersion 0.6.0, ProductName FerrisFilter, OriginalFilename ff.exe.

## Decisions

- Built into /tmp/fftgt40 so a running repo ff.exe was never overwritten.
- Debug exe only checked; the release exe is gated by verify-exe.sh in CI.
- `rustfmt --check build.rs` clean.

## Deviations from Plan

None. The checkpoint was resolved under the author's pre-authorisation rather than by an "approved" reply.

## For the Release Review

- Explorer Properties/icon of ff.exe is still to be eyeballed by the author in plan 40-07.
- New third-party build-dependency: winresource 0.1.34 (MIT), runs at build time.
- README/CHANGELOG untouched per instructions.

## Self-Check: PASSED
