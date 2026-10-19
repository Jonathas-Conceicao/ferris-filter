---
phase: 40-release-pipeline-installer
plan: 03
subsystem: release
tags: [ico, ff-art, user-agent, assets, licensing]
requires:
  - phase: 40-release-pipeline-installer
    provides: plans 01-02 (ff --quit, CI/release workflows)
provides:
  - assets/ferris/ferris.ico (7 frames) and the `ff-art icon` subcommand
  - assets/README.md distribution decision (PUB-01)
  - sync::client::USER_AGENT naming Jonathas-Conceicao/ferris-filter (PUB-02)
affects: [40-04 exe icon resource, 40-05 installer SetupIconFile]
tech-stack:
  added: ["image crate `ico` feature (already locked; ff-art only)"]
  patterns: ["one USER_AGENT const for every HTTP client"]
key-files:
  created: [assets/ferris/ferris.ico]
  modified: [tools/ff-art/Cargo.toml, tools/ff-art/src/main.rs, tools/ff-art/README.md, assets/poe2/manifest.toml, assets/README.md, src/sync/client.rs]
key-decisions:
  - "Icon rendered at 1024 px wide, cropped to the artwork, centred on a transparent square, Lanczos3 down to 16/24/32/48/64/128/256, every frame PNG-encoded"
  - "Test asserts no 'myfilter' in the user agent (lowercased) so no source file names the old repo"
requirements-completed: [PUB-01, PUB-02]
duration: 20min
completed: 2026-10-18
---

# Phase 40 Plan 03: Ferris icon, art distribution note, user agent Summary

Ferris `.ico` (7 sizes) rendered reproducibly by `cargo ff-art icon`, the art/font distribution decision recorded in `assets/README.md`, and one `USER_AGENT` constant naming `ferris-filter` and the real crate version.

## Tasks

1. `ff-art icon` + `assets/ferris/ferris.ico` - commit 2fc429b. ICO header `00 00 01 00 07 00`. `ico` feature added to the `image` dependency; Cargo.lock did not change. No other art was re-rendered. No new crate, so no `cargo info` needed.
2. User agent and README decision - commit e2f3752. `USER_AGENT` const plus test `user_agent_names_the_public_repo`; `ReqwestClient::new` uses `.user_agent(USER_AGENT)`. `assets/README.md` has the `ferris/ferris.ico` row and a `## Distribution decision` section.

## Verification

- `cargo ff-art icon` (CARGO_TARGET_DIR in the temp folder): `7 sizes assets/ferris/ferris.ico`
- `cargo build --locked` at root: ok
- `cargo test --locked --lib sync::client`: 1 passed
- `git grep -i poe2.myfilter` outside `.planning` and `CHANGELOG.md`: no matches
- rustfmt check on the two touched .rs files: clean
- README.md and CHANGELOG.md untouched

## Deviations from Plan

None. (First draft of the test named the old repo literally, which would have failed the plan's own grep acceptance; reworded before commit.)

## For the Release Review (Phase 42)

- CHANGELOG.md link references still point at `Jonathas-Conceicao/poe2-myfilter`. New tags are `vX.Y.Z`, so the 0.7.0 compare link is `v0.6...v0.7.0`. Left for the author per the no-CHANGELOG-edits rule.
- Changelog-worthy: the poe.ninja sync user agent now carries the real version and the ferris-filter repo URL; a Ferris icon (`assets/ferris/ferris.ico`) now exists.
- README.md Credits may want to mirror the new `assets/README.md` Distribution decision paragraph (author's call).

## Known Stubs

None.

## Self-Check: PASSED

Files exist (ferris.ico, SUMMARY); commits 2fc429b and e2f3752 exist.
