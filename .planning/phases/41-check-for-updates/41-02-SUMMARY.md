---
phase: 41-check-for-updates
plan: 02
subsystem: update
tags: [reqwest, github-releases, sha256, tempfile, shellexecute]
requires: [41-01]
provides:
  - "run_check, sweep_stale, ReqwestUpdateHttp, check_for_update_blocking (+ offline cfg(test) twin), spawn_installer in src/update.rs"
  - "overlay::win::shell_execute_open, overlay::instance::notify_error"
affects: [41-03, 41-05]
tech-stack:
  added: []
  patterns: [RAII temp folder with keep() on success, injectable HTTP fake]
key-files:
  created: []
  modified: [src/update.rs, src/overlay/win.rs, src/overlay/instance.rs, Cargo.toml]
key-decisions:
  - "Oversized download body is simulated in the fake by an oversize flag instead of a 200 MB buffer"
  - "run_check clears the work_dir slot at its start so a stale path from an earlier check never lingers"
requirements-completed: [UPD-01, UPD-02, UPD-03]
duration: 25min
completed: 2026-10-18
---

# Phase 41 Plan 02: Network and file side of the update check Summary

`run_check` downloads a newer release's installer and `SHA256SUMS.txt` into a fresh `FerrisFilter-update-*` folder under an injected temp root, streams it through a ring SHA-256, and returns Ready only on a match; every failure leaves the temp root empty.

## Tasks

1. run_check and sweep_stale against a fake GitHub: 09c84e8
2. Reqwest client, production entry point, installer spawn, error box: 719dab2

## Verification

- `cargo test --locked --lib update::` : 21 passed (fake GitHub covers ready, same, older, dev, prerelease, 404, offline, 429, 403 with and without exhausted limit, 500, bad JSON, bad tag, missing/open installer, untrusted URL, oversized asset, sums 404/transport/short hash/no line/non-UTF-8/oversized, mismatch, download 404, mid-stream reset, oversized body, sweep keep/delete rules, sweep at start of a check).
- `cargo test --locked --lib overlay::instance` : 38 passed. `cargo build --locked --bin ff` OK.
- `cargo clippy --locked --all-targets`: no diagnostics in update.rs, win.rs, instance.rs. Cargo.lock unchanged.
- The plan's `-D warnings` clippy form was not run as a gate because of the ~125 pre-existing errors elsewhere.

## Deviations from Plan

- None in behaviour. Tests live in the existing `mod tests`; `spawn_installer` and entry-point twin tests added there too. No test constructs `ReqwestUpdateHttp`.

## Decisions

- Windows-only parts of the spawn fallback call `crate::overlay::win::shell_execute_open`; the non-Windows twin returns Err.
- Installer redirects use reqwest's default policy under `https_only(true)`.

## For the Release Review

Nothing yet for README/CHANGELOG; the user-visible button lands in later plans of this phase. Live behaviour against a real published release is still the manual check in plan 41-05.

## Self-Check: PASSED

src/update.rs, src/overlay/win.rs, src/overlay/instance.rs modified; commits 09c84e8 and 719dab2 exist.
