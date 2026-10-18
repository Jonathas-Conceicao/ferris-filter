---
phase: 39-league-picker-price-source
fixed_at: 2026-10-18T00:00:00Z
review_path: .planning/phases/39-league-picker-price-source/39-REVIEW.md
iteration: 2
findings_in_scope: 2
fixed: 2
skipped: 0
status: all_fixed
---

# Phase 39: Code Review Fix Report

**Fixed at:** 2026-10-18
**Source review:** .planning/phases/39-league-picker-price-source/39-REVIEW.md
**Iteration:** 2

**Summary:**
- Findings in scope: 2 (WR-01, WR-02; the Info findings IN-01 to IN-07 are out of scope)
- Fixed: 2
- Skipped: 0

Verification after both fixes: `cargo test --lib -- overlay:: settings:: sync::` passed (976 tests,
with `FERRISFILTER_HOME` / `FERRISFILTER_GAME_DIR` set to empty dirs under the temp folder and no
network). `cargo build --all-targets` produced no warnings. `cargo clippy --all-targets` still
reports the 125 lib-test warnings that were already there, and none is on a changed line. `cargo fmt`
also touched `src/sync/league.rs`, `tests/isolation_guard.rs` and one screenshot hunk in `app.rs`; I
reverted those.

Process note: as in iteration 1, the commits were made directly in the given worktree
(`.claude/worktrees/milestone-closeout`). No nested worktree, temp branch or recovery sentinel was
created.

## Fixed Issues

### WR-01: Under Fallback, pending tier edits keep Apply live but no Apply clears them, and "Apply & quit" silently discards them

**Files modified:** `src/overlay/config_state.rs`, `src/overlay/app.rs`, `CLAUDE.md`
**Commit:** 8611871
**Chosen behaviour:** under Fallback, pending cutoff edits are kept but are not counted as unapplied
work. The Quit confirm still guards them.
- `ConfigState::dirty_count()` (and therefore `is_dirty()` and `can_apply()`) counts tier rows only
  while the cutoffs are in use. This one change covers every caller: the unapplied indicator, Apply
  enablement on every tab, `set_price_source`, and the Revert/Reload state recomputation. An Apply
  under Fallback therefore no longer reports success and then flips back to `Unapplied changes` on
  the next frame.
- New `ConfigState::has_unwritable_tier_edits()` (`!tiers_in_use && tiers_dirty()`). `tiers_dirty()`
  is unchanged, so the Economy ` *` marker, the Economy Revert and its Discard popup still see the
  edits, as UI-SPEC requires ("stay in memory, are not lost, and the Economy ` *` marker stays").
- In `app.rs`, `quit_dirty()` is now `applicable_dirty() || has_unwritable_tier_edits()`, so tray
  Quit still asks before the edits can be lost. On **Apply & quit**, the overlay runs Apply only when
  there is applicable work, which avoids a no-op write and a needless `/reloaditemfilter`. If
  unwritable tier edits remain afterwards, it does not quit. The window stays open and shows the
  warning `Not quit: tier edits apply only with Live prices. Switch to Live and Apply, or quit without
  applying.` **Quit without applying** still exits at once.
- When the user switches back to Live, the edits count as unapplied again and the next Apply writes
  them.

**Why this behaviour:** it is the smallest change that fits the existing unapplied-state model.
"Unapplied" now means "an Apply would write something", so Apply can never be live while it has
nothing to do. The edits stay visible where UI-SPEC puts them, on the Economy marker. Re-opening the
Quit confirm, which the reviewer suggested, was rejected: it would loop on Apply & quit forever. A
warning status leaves the user free to switch to Live or to quit again and choose Quit without
applying.

**CLAUDE.md:** one clause was added to the Economy-tab paragraph. It says that tier edits made before
switching to Fallback stay pending (Economy ` *`), but are neither written nor counted as unapplied
until Live is back, and that Apply & quit then stays open to keep them.

**Tests:**
- New app tests: `an_apply_under_fallback_leaves_unused_tier_edits_pending_but_not_unapplied`
  (Apply under Fallback leaves the state Applied with the cutoffs kept and `tiers.json` untouched,
  Quit still guarded, Unapplied again on Live) and
  `apply_and_quit_under_fallback_keeps_unused_tier_edits_and_stays_open` (with only tier edits and
  with other work pending: no quit, the edits are kept, the warning is shown; Quit without applying
  still quits).
- Extended `an_apply_with_unused_tiers_keeps_their_pending_text_and_errors` in `config_state`.

**Status:** fixed, requires human verification (logic and state handling).

### WR-02: `reload()` can move to another filter without saving or restoring its applied policy

**Files modified:** `src/overlay/app.rs`
**Commit:** 9b93f85
**Applied fix:** the store-and-restore pair moved out of `adopt_settings` into a shared helper,
`rebind_applied_policy(left)`. `adopt_settings`, used by filter switches and folder picks, calls it
just as before. `reload()` now captures `open_filter_path()` before it replaces the settings and the
game folder. When the filter that is open afterwards differs, it calls the same helper. A reload that
keeps the same filter leaves `applied_policy` alone.

**Test:** `a_pending_source_switch_survives_a_reload_onto_another_filter_and_back`. The test switches
to Fallback, hand-edits `ff-settings.json` to `other.filter` and reloads (the policy is the current
one), then switches back to `dynamic.filter` and reloads. The original `Auto` comes back and the
switch is still pending. A further same-filter reload leaves it unchanged.

**Status:** fixed, requires human verification (state handling).

## Iteration 1 (record)

All six iteration-1 findings were fixed: CR-01 `8382e3a` (unused out-of-order cutoffs neither refuse
Apply nor get written), WR-01 `8b9d2cf` (an Apply under Fallback keeps the pending cutoff text and
errors), WR-02 `32add33` (`left_policies`: a filter switch or folder pick keeps each filter's applied
price source), WR-03 `de006a9` (`save_keys`: league and price-source saves write only their own key
and never replace an unusable file), WR-04 `d4e6bf1` (`RefreshFailed` keeps the last loaded list
usable), WR-05 `14dd6a6` (an empty poe.ninja league list is a failed fetch). The full iteration-1
report is in `39-REVIEW-FIX.iter1.md`.

---

_Fixed: 2026-10-18_
_Fixer: Claude (gsd-code-fixer)_
_Iteration: 2_
