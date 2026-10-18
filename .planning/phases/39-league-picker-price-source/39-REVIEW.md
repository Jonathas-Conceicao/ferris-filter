---
phase: 39-league-picker-price-source
reviewed: 2026-10-18T00:00:00Z
depth: standard
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
  critical: 1
  warning: 5
  info: 4
  total: 10
status: issues_found
---

# Phase 39: Code Review Report

**Reviewed:** 2026-10-18
**Depth:** standard
**Files Reviewed:** 11
**Status:** issues_found

## Summary

Reviewed the Phase 39 diff (`9e532d6..HEAD`): the league list worker and pure view model
(`leagues.rs`), the `league` / `price_source` settings keys, the policy threading through
`apply_all::prepare` / `staged_economy_body`, the `tiers_in_use` gate in `ConfigState`, the
Economy tab redraw and status-line row in `shell.rs`, the screenshot flags, and the removal of
`OVERLAY_LEAGUE`.

The league view state table follows the UI-SPEC closely and is well covered by tests. The main
defect is in the "tier errors do not block Apply under Fallback" contract: it holds only for
unparseable text. A cutoff that parses but is out of order still blocks Apply, and the editor that
could fix it is greyed out. A second problem is that an Apply under Fallback silently throws away
the pending cutoff text that UI-SPEC says must survive. The other findings are about the
price-source staleness tracking, which is assumption-based and has holes (a filter switch,
`ff-settings.json` overwrite, a failed refresh, an empty list).

## Critical Issues

### CR-01: Under Fallback, a non-ascending tier cutoff still blocks Apply, and the editor that could fix it is greyed out

**File:** `src/overlay/config_state.rs:660-677` (with `445-457`, `410-433`)
**Issue:** `set_tier_text` writes every *parseable* value into `self.tiers` (line ~417), so a
cutoff that parses but is below the tier before it (e.g. A = `0.0001`) leaves the model
non-ascending. `recompute_tier_errors` records that as a row error. Under Fallback,
`has_tier_errors()` / `tiers_error()` now return false/None, so the first branch of `validate()`
is skipped. But the "defensive re-check" further down,
`ValidatedTiers::try_from(self.tiers.clone())`, still runs and returns `Err(NotAscending)`. The
same thing happens for `MissingTiers`. As a result `validate()`, `prepare_write()`, and so
`apply_all::prepare` all refuse the Apply, which contradicts CLAUDE.md and UI-SPEC ("Under Fallback
… tier errors do not block Apply"). The Tiers editor and its Reset button are greyed under
Fallback, with the tooltip "The fallback tiers are fixed." The user is told the tiers do not
matter, yet Apply is refused because of them, and the only way to fix it is to switch back to Live.
`unused_tiers_ignore_a_missing_required_tier_too` does not catch this: it fakes
`errors.general` while `self.tiers` stays valid, so it never reaches the re-check. Only the
unparseable `"abc"` case is really tested, and that one works only because the model keeps the
last valid value.
**Fix:** When the tiers are not in use, validate and write the *applied* tiers rather than the
working ones. One option is to skip the tier re-check and keep `tiers.json` untouched:
```rust
// validate()
if self.tiers_in_use {
    ValidatedTiers::try_from(self.tiers.clone())
        .map_err(|e| ConfigApplyError::Invalid(e.to_string()))?;
}
// prepare_write_inner()
let tiers_changed = self.tiers_in_use
    && (force || !tiers_equal(&self.tiers, &self.tiers_applied));
```
Then make `adopt` keep the working tiers and buffers when `!w.tiers_changed && !tiers_in_use`
(see WR-01). Add a test with a parseable, non-ascending cutoff under `set_tiers_in_use(false)`.

## Warnings

### WR-01: An Apply under Fallback throws away the pending cutoff text that UI-SPEC says must survive

**File:** `src/overlay/config_state.rs:708-721`
**Issue:** UI-SPEC (Tiers under Fallback) says: "Pending cutoff edits already typed before
switching stay in memory, are not lost … the inline error text still shows when the editor is
enabled again." Two things break this:
- `tiers_dirty_count` counts a differing buffer as dirty, so an invalid buffer (`"abc"`) keeps the
  config dirty and Apply enabled.
- A successful Apply calls `adopt`, which runs `self.tiers_applied = self.tiers.clone()` and then
  `resync_buffers()`. That rebuilds every buffer from the model and clears `errors`.

So the first Apply under Fallback (for example, to apply a rule edit) silently drops the typed
text and its inline error. Valid pending cutoffs are also committed to `tiers.json` while the UI
says the tiers are "fixed" and unused. The test
`unused_tiers_do_not_gate_apply_and_return_when_used_again` checks the state only *before* Apply,
never after `adopt`.
**Fix:** While `!tiers_in_use`, leave the tier part of the config alone on adopt: do not move
`tiers_applied` and do not resync the buffers.
```rust
if self.tiers_in_use || w.tiers_changed {
    self.tiers_applied = self.tiers.clone();
    self.tiers_bytes = w.tiers_new;
    self.resync_buffers();
}
```
This should be combined with `tiers_changed = false` under Fallback (CR-01).

### WR-02: Switching to another filter and back forgets a pending price-source switch, and the indicator then reports "Filter matches saved rules" for a stale economy section

**File:** `src/overlay/app.rs:1621-1628`, `src/overlay/app.rs:2912-2925`
**Issue:** `adopt_settings` sets `self.applied_policy = self.settings.price_policy()`. The comment
says this is because "another filter is another file". `select_filter` / `switch_decision` decide
whether to confirm using only `working.is_dirty()`, so a pending `source_pending()` neither
triggers the **Switch filter?** popup nor survives the switch. The sequence that goes wrong:
1. Filter X (economy section, live prices) is open.
2. The user switches the source to Fallback, so X is marked unapplied.
3. The user picks filter Y, then picks X again.

`applied_policy` is now ForceFallback, `source_pending()` is false, and the indicator says the file
matches. X still holds live prices, so the user believes it is on Fallback. Picking a folder
(`apply_folder`) hits the same path.
**Fix:** Track the policy per filter path, e.g. `HashMap<PathBuf, PriceSourcePolicy>`, filled on
Apply and on first load. Alternatively, include `source_pending()` in the `switch_decision` dirty
input so the user is at least warned before the stale state is forgotten.

### WR-03: A league pick or price-source toggle rewrites an unusable `ff-settings.json` with fallback defaults

**File:** `src/overlay/app.rs:1805-1835` (`save_view_setting`), `src/settings.rs:382-391`
**Issue:** `set_league` and `set_price_source` go through `save_view_setting`, which calls the
whole-file `settings::save`. When `ff-settings.json` exists but cannot be used (a trailing comma, a
non-object), the session settings are the fallback defaults (`dynamic.filter`, reload off). The
first league pick then writes those defaults over the user's file, losing their `filter`,
`game_folder`, collapsed sections and unknown keys. A wrongly typed `league` / `price_source` is
also replaced, though that one at least warns at load. `save_section_state` exists precisely to
avoid this for fold clicks (CLAUDE.md: "a hand-edited file is never replaced by fallback
defaults"). The reload toggle already had the same behaviour, but a league pick is a far more
frequent action, so the exposure grows a lot.
**Fix:** Add a key-scoped writer like `save_section_state`: read the file, set only `league` /
`price_source`, and refuse with `Ok(false)` plus a status message when the file is unusable. Use it
from `set_league` / `set_price_source`.

### WR-04: A failed refresh discards a good list, which can block Sync on a fresh install

**File:** `src/overlay/leagues.rs:89-102`, `src/overlay/leagues.rs:203-218`
**Issue:** `poll` replaces `Loaded(list)` with `Failed(e)` when a refresh fails. With nothing
saved and no `league` in `prices.json` (a fresh install that has not synced yet), the view drops
from "Forbidden Rites, Sync enabled" to "(no league)", with the dropdown disabled and Sync blocked.
The default was known a moment earlier. The user is stuck until the network is back, because of a
button press that is supposed to be harmless ("Refresh does not change the selection"). The
test `a_failed_refresh_is_failed` locks this behaviour in.
**Fix:** Keep the last good list on a failed refresh and expose the failure separately, e.g.
`Loaded { list, refresh_error: Option<String> }`. Alternatively, have `ListState::Failed` carry
`Option<Vec<EconomyLeague>>` so `view` can still compute `default_league` and show `TIP_FAILED`.

### WR-05: An empty `economyLeagues` list marks the saved league as ended and blocks Sync

**File:** `src/overlay/leagues.rs:242-269`, `src/sync/league.rs:102-106`
**Issue:** `list_leagues` returns `Ok(vec![])` when poe.ninja answers with an empty array (an API
hiccup or schema drift; `economyLeagues: []` still deserializes). `loaded_view` then fails to find
the saved name, shows `<name> (ended)` in amber, disables Sync, and puts "<name> is no longer on
poe.ninja; pick a league" on the status line on every tab. There is nothing to pick. UI-SPEC says
"unknown is not ended", and an empty list carries no information.
**Fix:** Treat an empty list as a failure:
```rust
let leagues = fetch_index_state(client).await?.economy_leagues;
if leagues.is_empty() {
    miette::bail!("poe.ninja listed no economy leagues");
}
Ok(leagues)
```

## Info

### IN-01: `reload()` lost its doc comment to `sync_tiers_in_use`

**File:** `src/overlay/app.rs:2579-2584`
**Issue:** `sync_tiers_in_use` was inserted between the RELOAD doc comment and `fn reload`. The
RELOAD paragraph now documents `sync_tiers_in_use`, and `reload()` has no doc at all.
**Fix:** Move the three `/// RELOAD: …` lines down onto `fn reload`.

### IN-02: Ended-league detection is case-sensitive while Sync matching is case-insensitive

**File:** `src/overlay/leagues.rs:259`, `src/sync/league.rs:77`
**Issue:** A hand-edited `"league": "forbidden rites"` is shown as ended and Sync is blocked, even
though `resolve_league_with_override` would accept it (`eq_ignore_ascii_case` on name or slug).
The test `the_saved_name_match_is_case_sensitive` locks this in on purpose, but the two halves
disagree.
**Fix:** Match the way sync matches (name or slug, ASCII case-insensitive), and select that entry.

### IN-03: `source_pending` uses today's `prices_live` for both sides, so it can raise false positives

**File:** `src/overlay/leagues.rs:302-310`
**Issue:** Example: the filter was applied under Auto with no `prices.json` (so it was written with
fallback prices), the user then syncs, then switches to Fallback. The economy section is flagged
stale even though Apply would write identical bytes. This is harmless (the Apply is a no-op that
counts as applied), but it can confuse the user. The underlying cause is assumption A2, as
documented.
**Fix:** Record the *effective* source (`effective_fallback(policy, live)`) at Apply time instead
of the policy alone, and compare that against the current effective source.

### IN-04: The price-source preview is never re-priced when the CONFIG state failed to load

**File:** `src/overlay/app.rs:2500-2506`
**Issue:** `refresh_staged_economy` returns early when `self.config` is `Err`. A staged **Add
economy section** preview therefore keeps the old policy's prices after a source switch, while
Apply (`config = None` goes to `economy_body(paths, policy)`) writes the new policy's prices. That
breaks the "what Apply will write" promise in this degraded state.
**Fix:** In the `Err(config)` branch, re-price with `staged_economy_body(root, None, policy)` when
the policy in the key differs.

---

_Reviewed: 2026-10-18_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
