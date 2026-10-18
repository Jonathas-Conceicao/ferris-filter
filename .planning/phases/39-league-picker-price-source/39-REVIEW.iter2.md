---
phase: 39-league-picker-price-source
reviewed: 2026-10-18T00:00:00Z
depth: standard
iteration: 2
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
  warning: 2
  info: 7
  total: 9
status: issues_found
---

# Phase 39: Code Review Report (re-review after fixes)

**Reviewed:** 2026-10-18
**Depth:** standard
**Files Reviewed:** 11
**Status:** issues_found

## Summary

This pass re-reviewed the Phase 39 diff (`9e532d6..HEAD`), with most attention on the six fix commits
after `53c4bee` (8382e3a, 8b9d2cf, 32add33, de006a9, d4e6bf1, 14dd6a6).

Status of the earlier findings:

- **CR-01: fixed.** `validate()` now skips the `ValidatedTiers` re-check while the tiers are unused, and
  `prepare_write_inner` never marks `tiers.json` changed under Fallback. In `apply_all::prepare`, the
  guard and the write both key off `tiers_changed()`, so an unused `tiers.json` is neither guarded nor
  written. `load_priced_with` ignores `live_tiers` under `ForceFallback`, so the out-of-order working
  tiers never reach pricing.
- **WR-01: fixed as specified.** `adopt` keeps the tier snapshot, the bytes and the buffers while the
  tiers are unused.
- **WR-02: fixed for `switch_to` / `apply_folder`.** `left_policies` is filled before `game_dir`
  changes and read back on arrival. The fix does not cover `reload()` (WR-02 below).
- **WR-03: fixed.** `save_keys` writes only the named keys. `league` and `price_source` are
  `skip_serializing_if = "Option::is_none"`, so "absent means remove" holds. An unusable file is left
  alone.
- **WR-04: fixed.** `RefreshFailed` keeps the list. `start` keeps it visible while a refresh runs,
  `fetching` disables the refresh button, and a success goes back to `Loaded`.
- **WR-05: fixed.** An empty list is an error, and only the overlay's worker calls `list_leagues`.

The fixes for CR-01 and WR-01 add one new problem (WR-01 below). Under Fallback, pending tier edits
are now in a state that no Apply can clear, and **Apply & quit** silently throws them away. IN-01 to
IN-04 are still present and are carried over as Info.

## Warnings

### WR-01: Under Fallback, pending tier edits keep Apply live but no Apply clears them, and "Apply & quit" silently discards them

**File:** `src/overlay/config_state.rs:691-692`, `src/overlay/config_state.rs:725-729`, `src/overlay/app.rs:2343-2352`, `src/overlay/app.rs:3455-3458`
**Issue:** After 8382e3a and 8b9d2cf, a tier edit made before switching to Fallback stays in
`tiers_dirty_count()`. `is_dirty()`, `combined_dirty` and `quit_dirty()` therefore stay true. Under
Fallback, though, `prepare_write_inner` forces `tiers_changed = false` and `adopt` leaves the snapshot
alone, so no Apply can ever clear those edits. This has two consequences:

1. Apply stays enabled while the only thing pending is unused tier edits. Pressing it reports
   `Wrote <path>.` and sends `/reloaditemfilter` to the game (the identical-bytes path still counts as
   applied). `on_apply_success()` sets `Applied`, and on the next frame `combined_dirty` flips it back
   to `Unapplied changes`. The user gets a success message and an unchanged "Unapplied changes"
   indicator, with nothing they can do to resolve it except Revert or switch to Live.
2. Tray Quit opens **Unapplied changes**. **Apply & quit** calls `run_apply`, which returns `ok =
   true`, so `quit_after_apply` is set and the process exits. The typed cutoffs are lost, even though
   the user picked the option that promises to apply, and UI-SPEC says these edits "are not lost".
   Before the fix they would at least have been written to `tiers.json`.

No test covers either path. `an_apply_with_unused_tiers_keeps_their_pending_text_and_errors` checks
only `ConfigState`.
**Fix:** Recognise "unused pending tier edits" as their own state, so they neither count as
applicable work nor get dropped by Apply & quit. For example:
```rust
// config_state.rs
/// Pending tier edits that Apply will not write (Fallback): kept, but not applicable.
pub fn has_unwritable_tier_edits(&self) -> bool {
    !self.tiers_in_use && self.tiers_dirty()
}

// app.rs, Popup::Quit / RunApply
let ok = self.run_apply(ctx, false);
let kept = self.config.as_ref().is_ok_and(|c| c.has_unwritable_tier_edits());
if ok && kept {
    // Re-open the confirm: Apply cannot write the cutoffs while Fallback is on.
    self.quit_confirm = true;
    self.apply_message = Some((
        "Tier edits are kept for Live prices; switch to Live and Apply, or quit without applying."
            .into(),
        w::StatusKind::Warning,
    ));
} else if apply::quit_step(dirty, false, apply::QuitEvent::ApplyFinished { ok })
    == apply::QuitStep::QuitWhenIdle
{
    self.quit_after_apply = true;
}
```
Also consider leaving unwritable tier edits out of the dirty input to the footer's Apply enablement,
while keeping the Economy ` *` marker, so that Apply is not live when it can do nothing. Add an app
level test: tier edit, then `set_price_source(false)`, then Quit, then Apply & quit. It should assert
that the process does not quit and that the edits remain.

### WR-02: `reload()` can move to another filter without saving or restoring its applied policy (gap in the WR-02 fix)

**File:** `src/overlay/app.rs:2637-2690`
**Issue:** The WR-02 fix records `applied_policy` per filter only in `switch_to` and `apply_folder`
(through `adopt_settings`). `reload()` also changes which filter is open, in two ways:
- It re-reads `ff-settings.json` (a hand-edited or externally changed `filter`).
- Outside tests, it re-resolves `game_dir` from `game_folder_path()`.

It does neither `left_policies.insert` nor the lookup. After such a reload, `applied_policy` still
holds the previous filter's value and is applied to the new one. The same reload also drops the
previous filter's pending switch. This is the same stale "Filter matches saved rules" or false
"Unapplied changes" that WR-02 was about, reached through **Reload**.
**Fix:** In `reload()`, capture `let left = self.open_filter_path();` before the settings and
`game_dir` are replaced. Afterwards, when `self.open_filter_path() != left`, do the same
insert-and-lookup that `adopt_settings` does. Moving that pair into a small helper,
`fn rebind_applied_policy(&mut self, left: Option<PathBuf>)`, lets all three paths share it.

## Info

### IN-01: `reload()` lost its doc comment to `sync_tiers_in_use` (carried)

**File:** `src/overlay/app.rs:2625-2637`
**Issue:** This is still present. The three `/// RELOAD: …` lines document `sync_tiers_in_use`, and
`fn reload` has no doc.
**Fix:** Move the RELOAD paragraph down onto `fn reload`.

### IN-02: Ended-league detection is case-sensitive while Sync matching is case-insensitive (carried)

**File:** `src/overlay/leagues.rs:283`, `src/sync/league.rs:77`
**Issue:** `names.iter().position(|n| n == s)` still uses exact matching, so a hand-edited
`"forbidden rites"` shows as `(ended)` and blocks Sync, while `resolve_league_with_override` would
accept it.
**Fix:** Match by name or slug with `eq_ignore_ascii_case`, the same way sync does.

### IN-03: `source_pending` compares both sides with today's `prices_live` (carried)

**File:** `src/overlay/leagues.rs:326-334`
**Issue:** This is unchanged. A filter applied under Auto with no `prices.json` (so effectively on
Fallback), followed by a sync and then a switch to Fallback, is flagged stale even though Apply would
write identical bytes.
**Fix:** Record `effective_fallback(policy, live)` at Apply time (and in `left_policies`), and compare
it against the current effective source.

### IN-04: The staged economy preview is not re-priced when CONFIG failed to load (carried)

**File:** `src/overlay/app.rs:2546-2552`
**Issue:** `refresh_staged_economy` still returns early when `self.config` is `Err`, so after a
source switch the preview keeps the old policy's prices while Apply writes the new policy's.
**Fix:** In the `Err` case, re-price with `staged_economy_body(root, None, policy)` when the policy in
`staged_econ_inputs` differs.

### IN-05: The ended-league status still shows after a failed refresh

**File:** `src/overlay/leagues.rs:199-205`, `src/overlay/app.rs:3695-3699`
**Issue:** `RefreshFailed` runs `loaded_view` on the old list, so `ended` is set, and the
`<name> is no longer on poe.ninja; pick a league` warning appears on every tab. UI-SPEC item 7 says
the ended status is "suppressed under Fallback, loading and failed". The fix report records this as a
deliberate decision, so the spec and the code now disagree. Also, when the saved league has ended
(or is `(no league)`), the `TIP_LEAGUE` → `TIP_FAILED` swap never happens, so nothing in the UI shows
that the refresh failed.
**Fix:** Either update UI-SPEC/CLAUDE.md to match the decision, or clear `v.ended` for
`RefreshFailed`. In both cases, surface the failure somewhere, for example on the refresh button
tooltip.

### IN-06: A transient read error on `ff-settings.json` is reported as "the file cannot be used, fix or delete it"

**File:** `src/settings.rs:416-424`, `src/overlay/app.rs:1848-1860`
**Issue:** `save_keys` maps every read error except NotFound (a sharing violation from
OneDrive/antivirus, permission denied) to `Ok(false)`. `save_view_key` then reports that as the
unusable-file message, which tells the user to delete a file that is fine.
**Fix:** Return `Err(e)` for read errors other than NotFound, and keep `Ok(false)` for JSON that does
not parse or is not an object. `save_view_key` already formats `Err` as `Could not save
ff-settings.json: <cause>`.

### IN-07: The reload toggle still writes the whole settings file

**File:** `src/overlay/app.rs:1823-1838`
**Issue:** `set_reload_after_apply` still goes through `save_view_setting`, which calls the
whole-file `settings::save`. With an unusable `ff-settings.json`, toggling **Reload filter in game
after Apply** replaces the user's file with fallback defaults, which is the exposure WR-03 closed for
the league and the source. The fix report acknowledges this was out of scope.
**Fix:** Add `save_reload_after_apply` using `save_keys(root, settings, &["reload_after_apply"])` and
route the toggle through `save_view_key`. `save_view_setting` then has no callers left and can be
removed.

---

_Reviewed: 2026-10-18_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
