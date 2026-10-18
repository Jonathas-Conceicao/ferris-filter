---
phase: 41-check-for-updates
verified: 2026-10-18T00:00:00Z
status: human_needed
score: 3/3 must-haves verified
overrides_applied: 0
human_verification:
  - test: "Live check, download, ready (MANUAL-VERIFICATION #1)"
    expected: "Against a real published release newer than the build: Checking, Downloading <ver>, then '<ver> available, released <date>' and the Install <ver> and quit button"
    why_human: "releases/latest returns 404 today; needs a real release and real GitHub"
  - test: "Install with nothing unapplied, and with unapplied edits (#2, #3)"
    expected: "Overlay exits, installer wizard runs with no UAC/SmartScreen prompt, Launch FerrisFilter reopens ff; Cancel keeps Ready and a later tray Quit launches nothing"
    why_human: "Needs the real installer and a real process hand-off"
  - test: "Already current, offline, rate limit, quit cleanup, GitHub JSON formats (#4-#8)"
    expected: "Messages as in 41-MANUAL-VERIFICATION.md; no %TEMP%\\FerrisFilter-update-* left; real published_at and browser_download_url match the accepted formats"
    why_human: "Real network behaviour and real GitHub field formats"
---

# Phase 41: Check for updates Verification Report

**Phase Goal:** Users can learn about and install a newer version from the About tab, safely.
**Verified:** 2026-10-18
**Status:** human_needed
**Re-verification:** No (initial verification)

## Goal Achievement

### Observable Truths

| # | Truth (ROADMAP SC) | Status | Evidence |
|---|---|---|---|
| 1 | Check on About tab reports "up to date" or names the newer version | VERIFIED | `src/update.rs` has `compare_release`, `message_for` and `checking_message`/`downloading_message`. `about_view.rs` has the button and an `AboutAction::Check`. `app.rs` ~4049 routes the click to the check, which runs on a worker in `overlay/update_job.rs` (644 lines). |
| 2 | Newer version: downloads installer, verifies SHA-256, offers Install and quit through the Unapplied changes popup | VERIFIED | `run_check` downloads SHA256SUMS.txt and the installer, uses `parse_sums` and `sha256_file`/`verify_installer`, and returns Ready only after a match (Mismatch is a Failure). `asset_url_is_trusted` is checked and the installer name is computed. `app.rs` `install_and_quit` calls `request_quit`, so the Unapplied changes popup applies. Cancel and refused-Apply paths call `cancel_install` (app.rs 2502/2523/2534). `begin_shutdown` hands the installer over via `update.shutdown()`. `overlay/mod.rs` 132 calls `launch_verified_installer` (re-hash, spawn, 740 fallback) only after `run_native` returns, with a notify_error message box on failure. |
| 3 | Offline / rate-limited / missing-asset failures show a status message and change nothing | VERIFIED | `Failure` enum has Offline, RateLimited, NoInstaller, NoSums, BadSums, Mismatch, DownloadFailed, BadResponse. Each return path in `run_check` goes through the work-folder cleanup. `message_for` gives one status message per failure. State returns to Idle (update_job tests). |

**Score:** 3/3

### Requirements Coverage

| Requirement | Source Plans | Status | Evidence |
|---|---|---|---|
| UPD-01 | 41-01..05 | SATISFIED (code); live check deferred | Truth 1 |
| UPD-02 | 41-01..05 | SATISFIED (code); live install deferred | Truth 2 |
| UPD-03 | 41-01..05 | SATISFIED (code) | Truth 3 |

All three IDs appear in plan frontmatter and in REQUIREMENTS.md (lines 113-119, 217-219). No orphaned requirements.

### Behavioral Spot-Checks

| Behavior | Command | Result | Status |
|---|---|---|---|
| Update logic, job state machine, app wiring tests | `cargo test --locked --lib update` | 55 passed, 0 failed | PASS |
| Whole suite (orchestrator, HEAD a8b8f38) | `cargo test --locked --no-fail-fast` | 1811 passed, 0 failed | PASS |

### Anti-Patterns Found

No TODO/FIXME/XXX/TBD in `src/update.rs` or `src/overlay/update_job.rs`. The review (3 iterations) fixed all Critical and Warning findings; 10 Info items remain and are non-blocking.

### Human Verification Required

The live end-to-end checks are listed in `41-MANUAL-VERIFICATION.md` (8 checks) and summarised in the frontmatter. The author deferred them to the end of the milestone run, because `releases/latest` returns 404. They need a published final release with `FerrisFilter-<X.Y.Z>-setup.exe` and `SHA256SUMS.txt`.

### Gaps Summary

No automated gaps. Every success criterion is backed by implemented, wired and tested code, and every requirement ID is accounted for. Status is human_needed only because of the deferred live checks.

---

_Verified: 2026-10-18_
_Verifier: Claude (gsd-verifier)_
