---
phase: 40-release-pipeline-installer
plan: 06
subsystem: release
tags: [docs, versioning, dry-run]
requires: [40-01, 40-02, 40-03, 40-04, 40-05]
provides: [release pipeline docs, version 0.7.0-rc.1]
key-files:
  modified:
    - CLAUDE.md
    - .planning/PROJECT.md
    - Cargo.toml
    - Cargo.lock
decisions:
  - Version bumped to 0.7.0-rc.1 in Cargo.toml with Cargo.lock in the same commit
  - Pre-existing cargo fmt drift left alone (out of scope)
metrics:
  tasks: 2
  completed: 2026-10-18
---

# Phase 40 Plan 06: Release docs, version bump and local dry run Summary

Release pipeline documented (CLAUDE.md, PROJECT.md), root version set to 0.7.0-rc.1, and every local check that does not need Inno Setup run green. Plans 40-07 (Inno Setup install and installer test) and 40-08 (push the rc tag) remain PENDING, deferred by the author to the end of the milestone. The installer was NOT compiled or tested.

## Commits
- ca248c0: docs(40-06): document release pipeline and vX.Y.Z tag convention
- f9dc8fe: chore(40-06): set version 0.7.0-rc.1 for the release dry run

## What was built
- PROJECT.md: Git Workflow tag is an annotated `vX.Y.Z` from v0.7.0 on, and pushing it runs `release.yml`; close-out step 3 says tag `vX.Y.0`; the closing paragraph states the tag = `v` + Cargo version equality, the required `## [X.Y.Z]` section, and rc notes from `[Unreleased]` published as pre-releases.
- CLAUDE.md: new `.github/workflows/` and `installer/` rows (with `dist/`, `FF_SKIP_INSTALLER`, `FF_EXE`, AppId warning), `ff --quit` exit codes in the `ff` row, `ferris.ico` / winresource / `FF_REQUIRE_RESOURCE` in the assets row, `icon` in the ff-art subcommands.
- Cargo.toml 0.7.0-rc.1, Cargo.lock `ferrisfilter` entry in step. ff-art keeps 0.1.0. Release profile untouched (lto fat, codegen-units 1, strip), REL-06.

## Local dry run (CARGO_TARGET_DIR=/tmp/fftgt40, isolated FERRISFILTER_HOME / FERRISFILTER_GAME_DIR)
- rustc 1.100.0-nightly (a69a63265 2026-09-03).
- `FF_REQUIRE_RESOURCE=1 cargo build --release --locked --bin ff`: ok, 8m41s. ff.exe 20,896,768 bytes.
- `installer/verify-exe.sh ff.exe 0.7.0-rc.1`: `ok: ... carries FerrisFilter 0.7.0-rc.1`. (The repo-root target/ ff.exe was not touched; the release ff.exe is at /tmp/fftgt40/release/ff.exe, so plan 40-07 must use `FF_EXE` or rebuild.)
- `release-check.sh v0.7.0-rc.1 <notes>`: exit 0, `version=0.7.0-rc.1`, `prerelease=true`; notes (494 bytes) identical to the `[Unreleased]` body (`diff` empty). No CHANGELOG section is needed for the rc, so no failure to report.
- `release-check.sh v0.7.0`: fails, `tag v0.7.0 != Cargo.toml version 0.7.0-rc.1`. With `FF_CARGO_VERSION=0.7.0`: fails, `CHANGELOG.md has no non-empty '## [0.7.0]' section` (expected until Phase 42). `v0.7.0-rc.2`: fails, tag != Cargo version.
- `FF_EXE=... FF_SKIP_INSTALLER=1 bash installer/package.sh 0.7.0-rc.1`: ok (loud "not a releasable set" warning).
  - dist/FerrisFilter-0.7.0-rc.1-windows-x64.zip, 9,607,966 bytes; contents: `ff.exe`, `LICENSE.txt`, `README.txt` only.
  - dist/SHA256SUMS.txt (106 bytes, zip line only, no setup since installer skipped):
    `c606ae922f48bc3384e1933e60bfa859ebbf1803681c8d0ef790f9dfba75049e  FerrisFilter-0.7.0-rc.1-windows-x64.zip`; `sha256sum -c` OK.
  - Extracted ff.exe is byte-identical (`cmp`) to the release build.
- Exe version info: verified through verify-exe.sh only (FerrisFilter 0.7.0-rc.1); a PowerShell VersionInfo dump was not possible in this sandbox.
- `cargo test --locked --no-fail-fast`: 1740 passed, 0 failed (lib 1617; integration 12, 4, 20, 26, 25, 18, 5, 4, 8, 1; doctests 0).
- No tag created, nothing pushed. dist/ is git-ignored.

## Deviations from Plan
- `cargo fmt --check` fails (exit 1), but only on pre-existing drift in files this phase did not touch: `src/overlay/app.rs:1328`, `src/sync/league.rs:197`, `tests/isolation_guard.rs` (108, 132, 143). Not fixed (out of scope; CI runs `cargo test` only, not fmt). Deferred.
- Release build and tests used `/tmp/fftgt40` instead of `target/`, so `target/release/ff.exe` in the repo does not exist; the plan's acceptance commands that reference it were run against `/tmp/fftgt40/release/ff.exe`.

## For the Release Review
- Fix the fmt drift above (rustfmt only those files) before the milestone closes if fmt is wanted clean.
- Plan 40-07 needs a release ff.exe: reuse /tmp/fftgt40/release/ff.exe via `FF_EXE`, or rebuild.
- When the real version is set (0.7.0), release-check requires a non-empty `## [0.7.0]` CHANGELOG section; the author writes it in Phase 42.
- README.md and CHANGELOG.md were not touched.

## Self-Check: PASSED
