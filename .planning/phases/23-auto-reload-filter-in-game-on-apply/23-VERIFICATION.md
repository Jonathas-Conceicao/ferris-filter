---
phase: 23-auto-reload-filter-in-game-on-apply
verified: 2026-10-13T00:00:00Z
status: passed
score: 5/5 must-haves verified (in-game behaviour pending UAT)
overrides_applied: 0
human_verification:
  - test: "Command exists in the game: type /reloaditemfilter by hand in chat (MANUAL-VERIFICATION check 0)"
    expected: "The game reloads the filter; no unknown-command message"
    why_human: "Real game command; cannot be tested headless"
  - test: "RULES Apply with the game running and the setting on (checks 1, 2, 3)"
    expected: "Game takes focus, command sent exactly once, edit visible in game, stripe later reads 'Wrote <path>. Reloaded in game', clipboard restored afterwards"
    why_human: "Focus, SendInput keystrokes, clipboard and the game's chat box need the real game"
  - test: "Not-sent cases: game closed, browser tab titled 'Path of Exile 2', setting off, unchanged Apply, refused Apply, CONFIG Apply (checks 4, 5, 6, 8, 9, 10)"
    expected: "Nothing typed anywhere; Refresh reminder with the reason shown where applicable"
    why_human: "Real window enumeration and the absence of real input cannot be observed headless"
  - test: "Chat already open with text (check 7, 7b) and same-privilege-level requirement"
    expected: "Documented limitation (CR-02): text in an open chat box is sent first; a pre-filled channel prefix is replaced by Ctrl+A"
    why_human: "In-game chat behaviour"
---

# Phase 23: Auto-Reload Filter in Game on Apply Verification Report

**Phase Goal:** After a successful RULES-tab Apply, the game reloads the filter without the user opening Options -> Game -> Item Filter -> Refresh
**Verified:** 2026-10-13
**Status:** human_needed
**Re-verification:** No, initial verification

## Goal Achievement

All code paths exist, are substantive and wired. Real in-game behaviour (focus, keystrokes, clipboard,
the command itself) cannot be tested headless, so the goal is verified in code and pending UAT
(REQUIREMENTS.md already marks RELOAD-01 "Complete (UAT pending)").

### Observable Truths (ROADMAP success criteria)

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | Written RULES Apply, game window exists, setting on: `/reloaditemfilter` sent once (focus, open chat, clear partial input, enter command, send); status "Reloaded in game" | VERIFIED (code) / human for real send | `chat::reload_steps()` (chat.rs:~105-135): modifiers check, FocusGame, VerifyForeground, SaveClipboard, SetClipboard(`RELOAD_COMMAND`="/reloaditemfilter"), Enter, Ctrl+A, Ctrl+V, Enter, restore clipboard; no Escape. `app.rs` run_apply (l.902-935) calls `decide` -> `with_owner` -> `ReloadSend::start` (single in-flight guard) with `WinSink` + `execute`; result replaced via `landed_reload_message` containing "Reloaded in game". The wording means keys were sent (accepted decision). |
| 2 | Nothing sent on CONFIG Apply, Unchanged, Failed, game not running, setting off; Refresh reminder with reason | VERIFIED | `decide` returns `None` unless `ApplyOutcome::Written`; setting checked before `find_game` (FnOnce); `Skip(SettingOff/Busy/GameNotRunning)`; `NoWindow` via `with_owner`. `decide` is called at exactly one site (app.rs:904, RULES apply path), so CONFIG Apply cannot send. `skipped_reload_message` uses `apply::not_reloaded_message` (Refresh reminder + reason). `decide_truth_table` unit test passes. |
| 3 | CONFIG setting "Reload filter in game after Apply", persisted in ff-settings.json, default on | VERIFIED | `settings.rs`: `reload_after_apply: Option<bool>`, accessor `unwrap_or(true)`; tests for default-on, false round-trip, unknown keys, wrong type. `ConfigAction::SetReloadAfterApply` -> `set_reload_after_apply` persists at once (app.rs:854, 1114); view receives `reload_after_apply` (app.rs:1221). WR-01/WR-02 fixes keep a reload-off from a warned load (`settings_after_reload`). |
| 4 | Keystroke sequence built behind a seam, unit-tested without real input | VERIFIED | `InputSink` trait + `Step` list + `execute(&mut dyn InputSink, ..)`; tests `plan_is_exact`, `plan_invariants`, failure-stops-sequence, clipboard-restored-once, worker error/panic. Real `WinSink`/`win.rs` is the only SendInput user. |
| 5 | Overlay docs state `/reloaditemfilter` reloads the filter selected in game | VERIFIED | CLAUDE.md overlay notes (~l.143-145) state it explicitly, plus chat-open caveat, privilege level, clipboard trade-off; CHANGELOG [Unreleased] entry present (l.17-21). |

**Score:** 5/5 truths verified in code

### Required Artifacts

| Artifact | Status | Details |
|----------|--------|---------|
| `src/overlay/chat.rs` (872 lines) | VERIFIED | steps, sink seam, decide, with_owner, ReloadSend, WinSink |
| `src/overlay/win.rs` (737 lines) | VERIFIED | `find_game_window` (title + exe prefix `pathofexile`, skips own pid), `focus_game`, `game_is_foreground`, `modifiers_held`, `send_tap`/`send_chord` (SendInput), clipboard wrappers; non-Windows stubs; windows-sys features added in Cargo.toml |
| `src/overlay/apply.rs` | VERIFIED | `reloading_message`, `not_reloaded_message`, `reload_result_message` |
| `src/settings.rs` | VERIFIED | `reload_after_apply` |
| `src/overlay/app.rs` | VERIFIED | wiring, pending reload, click-away exception while a reload is in flight (CR-01) |
| `23-MANUAL-VERIFICATION.md` | VERIFIED | 11+ checks including limitation checks |

### Key Link Verification

| From | To | Via | Status |
|------|----|-----|--------|
| run_apply | decide / with_owner | `chat::decide(.., win::find_game_window)` | WIRED |
| decision Send | worker | `self.reload.start(.. WinSink .. execute(reload_steps()))` | WIRED |
| worker result | status line | `reload.poll()` -> `landed_reload_message` (app.rs:1541) | WIRED |
| CONFIG toggle | settings file | `SetReloadAfterApply` -> `set_reload_after_apply` | WIRED |

### Behavioral Spot-Checks

| Check | Command | Result | Status |
|-------|---------|--------|--------|
| Formatting | `cargo fmt --check` | exit 0, no diff | PASS |
| Full test suite | `CARGO_TARGET_DIR=C:/Users/jonat/AppData/Local/Temp/ff-target cargo test` | lib 791 passed; integration suites 12, 4, 16, 15, 22, 8, 4, 8, 1 passed; 0 failed (881 total), no warnings | PASS |

No test sends input; ff was never launched, no game folder or process touched.

### Requirements Coverage

| Requirement | Source Plans | Status | Evidence |
|-------------|--------------|--------|----------|
| RELOAD-01 | 23-01..04 (all declare it) | SATISFIED in code, UAT pending | Truths 1-5; the only Phase 23 ID in REQUIREMENTS.md; no orphans |

### Anti-Patterns Found

None blocking. No TBD/FIXME/XXX in chat.rs, win.rs, apply.rs, settings.rs. Known, documented limitations (not gaps):
- CR-02: text already in an open chat box is sent first (accepted; matches Exiled Exchange 2 / Awakened PoE Trade; documented in CLAUDE.md and the UI caption).
- "Reloaded in game" means the command was sent, not that the game confirmed it.
- Open review Info items IN-01 (modifier pressed mid-sequence can leave chat open) and IN-02 (invalid filter-name fallback is silent) are out of scope.

### Human Verification Required

See frontmatter: run `23-MANUAL-VERIFICATION.md` (checks 0-10, 7b) with a release build, PoE2 windowed, same privilege level. Check 0 (command exists) is the gating one.

### Gaps Summary

No code gaps. The goal is implemented and wired with full unit coverage of the plan, decision and settings logic; achievement of the user-visible outcome in the real game awaits the manual UAT.

---

_Verified: 2026-10-13_
_Verifier: Claude (gsd-verifier)_

## Human UAT outcome (2026-10-15)

UAT complete (v0.5 wrap-up). Results in 23-HUMAN-UAT.md; untested items accepted as assumed passes by the author. Status changed from `human_needed` to `passed`.
