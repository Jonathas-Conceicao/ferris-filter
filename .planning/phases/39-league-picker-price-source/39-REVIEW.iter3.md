---
phase: 39-league-picker-price-source
reviewed: 2026-10-18T00:00:00Z
depth: standard
iteration: 3
files_reviewed: 11
files_reviewed_list:
  - src/overlay/app.rs
  - src/overlay/apply_all.rs
  - src/overlay/config_state.rs
  - src/overlay/leagues.rs
  - src/overlay/mod.rs
  - src/overlay/screenshot.rs
  - src/overlay/shell.rs
  - src/settings.rs
  - src/sync/league.rs
  - src/sync/mod.rs
  - src/sync/models.rs
findings:
  critical: 0
  warning: 1
  info: 9
  total: 10
status: issues_found
---

# Phase 39: Code Review Report (re-review after iteration-2 fixes)

**Reviewed:** 2026-10-18
**Depth:** standard
**Files Reviewed:** 11
**Status:** issues_found

## Summary

This pass re-reviewed the Phase 39 diff (`9e532d6..HEAD`), with most attention on the two
iteration-2 fix commits (`14dd6a6..HEAD`: 8611871, 9b93f85).

Status of the iteration-2 findings:

- **WR-01: mostly fixed.** `ConfigState::dirty_count()` now counts tier rows only while the cutoffs
  are in use. That one change reaches every caller of `is_dirty()`: the footer, `set_price_source`,
  the Revert paths, `reload_changed` and `overwrite_decision`. So an Apply under Fallback no longer
  reports success and then flips back to `Unapplied changes`. `tiers_dirty()` is unchanged, so the
  Economy ` *` marker, Economy Revert and the staged-preview trigger still see the edits.
  `ConfigState::refresh` still follows disk only when `tiers_dirty_count() == 0`, so a `tiers.json`
  change on disk cannot silently replace unused edits. `can_apply()` is only used by tests.
  `quit_dirty()` still guards the edits, and **Apply & quit** no longer quits while they would be
  lost. One new problem remains in the mixed case (WR-01 below).
- **WR-02: fixed.** `rebind_applied_policy` is shared by `adopt_settings` and `reload()`. `reload()`
  captures `open_filter_path()` before it replaces the settings or the game folder, and rebinds only
  when the open filter changed. A same-filter reload keeps `applied_policy`, and the new test covers
  both directions. One small edge is recorded as IN-09.

The iteration-1 fixes (CR-01, WR-01 to WR-05) are untouched by these commits and still hold. IN-01 to
IN-07 are still present and are carried over.

## Warnings

### WR-01: Apply & quit with other work plus unused tier edits loses both the Apply outcome and the "Not quit" warning when an in-game reload is sent

**File:** `src/overlay/app.rs:2353-2372` (with `src/overlay/app.rs:2026-2038`, `1162-1170`, `1176-1189`)
**Issue:** In the mixed case (rule, style or source work pending plus unused tier edits under
Fallback), `run_apply` writes the filter. When reload-after-apply is on and the game is running, it
also starts the `/reloaditemfilter` sequence: it sets `apply_message` to the interim line and records
`reload_pending.interim`. The quit branch then overwrites `apply_message` with
`UNWRITABLE_TIERS_QUIT_STATUS`. After that:

1. The reload focuses the game, and the overlay hides (click-away). `clear_events_on_conceal`
   keeps the message only when it equals `reload_pending.interim`. The warning does not, so it is
   cleared.
2. When the reload worker reports, `landed_reload_message` returns `None` for the same reason, so
   the reload result is never shown either. That includes a failure such as
   `could not reload in game: <cause>`, which tells the user to Refresh by hand.

Net effect: the user clicks **Apply & quit**, the window disappears (the game took focus), the
process quietly stays in the tray with its hotkey registered, and the next reveal shows neither why
it did not quit nor whether the in-game reload worked. The new test
`apply_and_quit_under_fallback_keeps_unused_tier_edits_and_stays_open` cannot catch this because
no game window exists in tests, so the `Decision::Send` path never runs.

**Fix:** Do not replace the interim line while a reload is pending. Fold the warning into the
pending reload instead, so that the conceal rule and the landed message both carry it:
```rust
if ok && self.has_unwritable_tier_edits() {
    let note = format!(" {UNWRITABLE_TIERS_QUIT_STATUS}");
    match &mut self.reload_pending {
        Some(p) if self.apply_message.as_ref().is_some_and(|(t, _)| *t == p.interim) => {
            p.extras.push_str(&note);          // carried into landed_reload_message
            p.interim.push_str(&note);         // still equals apply_message, so it survives a conceal
            self.apply_message = Some((p.interim.clone(), w::StatusKind::Warning));
        }
        _ => {
            self.apply_message =
                Some((UNWRITABLE_TIERS_QUIT_STATUS.to_string(), w::StatusKind::Warning));
        }
    }
}
```
(Alternatively, skip the in-game reload for an Apply run from the quit confirm that will not quit.)
Add a unit test that sets `reload_pending` and an interim `apply_message` before the quit branch,
and asserts that the warning survives `clear_events_on_conceal` and appears in the landed message.

## Info

### IN-01: `reload()` lost its doc comment to `sync_tiers_in_use` (carried)

**File:** `src/overlay/app.rs:2660-2672`
**Issue:** This is still present. The three `/// RELOAD: …` lines sit on `sync_tiers_in_use`, and
`fn reload` has no doc.
**Fix:** Move the RELOAD paragraph down onto `fn reload`.

### IN-02: Ended-league detection is case-sensitive while Sync matching is case-insensitive (carried)

**File:** `src/overlay/leagues.rs:283`, `src/sync/league.rs:77`
**Issue:** `names.iter().position(|n| n == s)` still matches exactly, so a hand-edited
`"forbidden rites"` shows as `(ended)` and blocks Sync, while `resolve_league_with_override` would
accept it.
**Fix:** Match by name or slug with `eq_ignore_ascii_case`, the same way sync does.

### IN-03: `source_pending` compares both sides with today's `prices_live` (carried)

**File:** `src/overlay/leagues.rs:326-334`
**Issue:** This is unchanged. Take a filter applied under Auto with no `prices.json` (so it was
effectively priced from Fallback). After a sync, a switch to Fallback is flagged stale even though
Apply would write identical bytes.
**Fix:** Record `effective_fallback(policy, live)` at Apply time (and in `left_policies`), and
compare it against the current effective source.

### IN-04: The staged economy preview is not re-priced when CONFIG failed to load (carried)

**File:** `src/overlay/app.rs:2580-2585`
**Issue:** `refresh_staged_economy` still returns early when `self.config` is `Err`, so after a
source switch the preview keeps the old policy's prices while Apply writes the new policy's.
**Fix:** In the `Err` case, re-price with `staged_economy_body(root, None, policy)` when the policy
in `staged_econ_inputs` differs.

### IN-05: The ended-league status still shows after a failed refresh (carried)

**File:** `src/overlay/leagues.rs:199-205`
**Issue:** `RefreshFailed` runs `loaded_view` on the old list, so `ended` is set and the
`<name> is no longer on poe.ninja; pick a league` warning still appears. UI-SPEC item 7 suppresses
it for "failed". When the saved league has ended or is `(no league)`, nothing in the UI shows that
the refresh failed.
**Fix:** Update UI-SPEC/CLAUDE.md to match the decision, or clear `v.ended` for `RefreshFailed`.
Either way, surface the failure, for example on the refresh button's tooltip.

### IN-06: A transient read error on `ff-settings.json` is reported as "the file cannot be used, fix or delete it" (carried)

**File:** `src/settings.rs:416-424`
**Issue:** `save_keys` still maps every read error other than NotFound to `Ok(false)`, which
`save_view_key` reports as the unusable-file message.
**Fix:** Return `Err(e)` for read errors other than NotFound, and keep `Ok(false)` for JSON that
does not parse or is not an object.

### IN-07: The reload toggle still writes the whole settings file (carried)

**File:** `src/overlay/app.rs:1832-1845`
**Issue:** `set_reload_after_apply` still calls `save_view_setting`, which uses the whole-file
`settings::save`. With an unusable `ff-settings.json`, toggling the setting replaces the user's
file with fallback defaults.
**Fix:** Route the toggle through `save_view_key` with a `save_keys(root, settings,
&["reload_after_apply"])` wrapper, then remove `save_view_setting`.

### IN-08: With only unused tier edits, the Quit confirm says "Apply them before quitting?" while the indicator says the filter matches, and its Apply button cannot apply

**File:** `src/overlay/app.rs:2459-2461`, `src/overlay/shell.rs:999-1011`, `src/overlay/apply.rs:409-413`
**Issue:** Since 8611871, tier edits that are unused under Fallback are deliberately not unapplied
work. The status line reads `Filter matches saved rules`. Tray Quit still opens the generic
**Unapplied changes** / "Apply them before quitting?" popup, though, and its first button can only
lead to the "Not quit" warning. Nothing in the popup tells the user that the pending items are tier
edits that need Live prices. This is not wrong, since nothing is lost, but the user only finds out
after choosing a dead-end option.
**Fix:** Pass a flag into `PopupData::Quit`. When the only pending items are unwritable tier edits,
use body text such as `Tier edits are kept only for Live prices. Quit and lose them?`, and grey out
or drop **Apply & quit**.

### IN-09: `left_policies` is keyed by exact `PathBuf`, so a case-only change of the filter name or folder loses the recorded policy

**File:** `src/overlay/app.rs:1659-1667`, `src/overlay/app.rs:2712-2715`
**Issue:** After 9b93f85, `reload()` rebinds whenever `open_filter_path() != left`. On Windows,
hand-editing `ff-settings.json` from `dynamic.filter` to `Dynamic.filter` (or changing the case of
`game_folder`) still names the same file. The comparison sees a different filter, though, so it
records the old policy under the old key, misses the lookup for the new key, and resets
`applied_policy` to the current policy. That silently drops a pending source switch for a file that
was never re-applied. `pick_folder` already compares folders with `eq_ignore_ascii_case`, so the
code knows about this case.
**Fix:** Normalise the key, for example lowercasing `to_string_lossy()` on Windows (or using a
case-insensitive compare), both for the `!=` check in `reload()` and for the `left_policies`
key.

---

_Reviewed: 2026-10-18_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
