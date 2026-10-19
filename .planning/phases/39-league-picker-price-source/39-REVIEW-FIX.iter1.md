---
phase: 39-league-picker-price-source
fixed_at: 2026-10-18T00:00:00Z
review_path: .planning/phases/39-league-picker-price-source/39-REVIEW.md
iteration: 1
findings_in_scope: 6
fixed: 6
skipped: 0
status: all_fixed
---

# Phase 39: Code Review Fix Report

**Fixed at:** 2026-10-18
**Source review:** .planning/phases/39-league-picker-price-source/39-REVIEW.md
**Iteration:** 1

**Summary:**
- Findings in scope: 6 (CR-01, WR-01..WR-05; Info findings out of scope)
- Fixed: 6
- Skipped: 0

Verification after all fixes: `cargo test --lib -- overlay:: settings:: sync::` passed (973 tests,
`FERRISFILTER_HOME` / `FERRISFILTER_GAME_DIR` set to empty dirs under `%TEMP%`, no network).
`cargo build --all-targets` produced no warnings. `cargo clippy --all-targets` still reports the
125 lib-test warnings that were already there, and none of them is on a changed line.

Process note: the commits were made directly in the given worktree
(`.claude/worktrees/milestone-closeout`, branch `milestone/v0.7-...`). I did not create a nested
reviewfix worktree, a temp branch or a recovery sentinel, because the instructions were to work only
here and never merge.

## Fixed Issues

### CR-01: Under Fallback, a non-ascending tier cutoff still blocks Apply, and the editor that could fix it is greyed out

**Files modified:** `src/overlay/config_state.rs`
**Commit:** 8382e3a
**Applied fix:** `validate()` now runs the defensive `ValidatedTiers` re-check only when the tiers
are in use. `prepare_write_inner` marks `tiers.json` as changed only when the tiers are in use, so
under Fallback the file keeps its bytes. New test:
`unused_out_of_order_tiers_do_not_refuse_apply_and_are_not_written`, which uses a cutoff that parses
but breaks the order (`A = 0.0001`).
**Status:** fixed, requires human verification (logic change).

### WR-01: An Apply under Fallback throws away the pending cutoff text that UI-SPEC says must survive

**Files modified:** `src/overlay/config_state.rs`
**Commit:** 8b9d2cf
**Applied fix:** `adopt()` now moves `tiers_applied` / `tiers_bytes` and rebuilds the buffers only
when `tiers_in_use || w.tiers_changed`. Pending cutoff text (valid or not) and its inline errors
therefore survive an Apply under Fallback, and they come back when Live is chosen again. New test:
`an_apply_with_unused_tiers_keeps_their_pending_text_and_errors`.
**Decision:** pending tier edits keep counting as dirty under Fallback, as UI-SPEC requires ("the
Economy ` *` marker stays while they exist"). After an Apply under Fallback the indicator therefore
still reads `Unapplied changes` while those edits are pending, and they are written on the first
Apply after switching back to Live.
**Status:** fixed, requires human verification (logic change).

### WR-02: Switching to another filter and back forgets a pending price-source switch

**Files modified:** `src/overlay/app.rs`
**Commit:** 32add33
**Applied fix:** added `left_policies: HashMap<PathBuf, PriceSourcePolicy>`. When a filter switch or
folder pick leaves a filter, its `applied_policy` is stored under that filter's path. The path is
captured before `apply_folder` changes `game_dir`. On arrival, a filter left earlier in the session
gets its stored policy back, and any other filter keeps the old assumption (written under the policy
now in force). Added a helper `open_filter_path()`. New test:
`a_pending_source_switch_survives_switching_to_another_filter_and_back`.
**Status:** fixed, requires human verification (logic change).

### WR-03: A league pick or price-source toggle rewrites an unusable `ff-settings.json` with fallback defaults

**Files modified:** `src/settings.rs`, `src/overlay/app.rs`, `CLAUDE.md`
**Commit:** de006a9
**Applied fix:** `save_section_state`'s body became a generic writer, `save_keys(root, settings,
keys)`, which writes only the named keys and keeps every other key byte-for-byte. A missing file is
still created whole, and an unusable file is left alone (`Ok(false)`). New writers `save_league` /
`save_price_source` use it. `set_league` / `set_price_source` now go through a new
`save_view_key`. When the file is unusable, the choice still applies for the session and the status
line reads `Could not save ff-settings.json: the file cannot be used, fix or delete it`. That is the
UI-SPEC save-failure wording, with a reason added. The one-line CLAUDE.md `ff-settings.json` note
was updated. New tests: `league_and_price_source_saves_write_only_their_own_key` (settings),
`a_league_pick_or_source_switch_never_overwrites_an_unusable_settings_file` and
`a_league_pick_writes_only_its_own_key` (app).
**Decisions:**
- The fold save is silent, but a league or source choice that cannot be persisted shows the
  save-failure event, because UI-SPEC prescribes it for a failed save.
- The reload toggle (`set_reload_after_apply`) still uses the whole-file `save`. The finding was
  scoped to the league and the price source. If the same treatment is wanted there, it can reuse
  `save_keys` with `"reload_after_apply"`.

### WR-04: A failed refresh discards a good list, which can block Sync on a fresh install

**Files modified:** `src/overlay/leagues.rs`, `CLAUDE.md`
**Commit:** d4e6bf1
**Applied fix:** added the variant `ListState::RefreshFailed(Vec<EconomyLeague>, String)`. When a
fetch fails after a list had loaded, `poll` keeps that list, and `start` keeps it visible during a
further refresh. `view` renders the kept list exactly like `Loaded`: default league, Sync enabled,
and ended detection against the last good list. The one difference is the dropdown tooltip, which
becomes `TIP_FAILED`, matching UI-SPEC's "only the dropdown tooltip changes". A later successful
refresh returns the state to `Loaded`. The test `a_failed_refresh_is_failed` became
`a_failed_refresh_keeps_the_loaded_list_usable`. CLAUDE.md got one clause.
**Decision:** a saved league that the last good list did not contain stays "(ended)" after a failed
refresh. That was already known before the refresh, so it is not a case of a failed fetch producing
"ended".
**Status:** fixed, requires human verification (state handling).

### WR-05: An empty `economyLeagues` list marks the saved league as ended and blocks Sync

**Files modified:** `src/sync/league.rs`, `CLAUDE.md`
**Commit:** 14dd6a6
**Applied fix:** `list_leagues` now returns an error (`poe.ninja listed no economy leagues`) for an
empty array, so the overlay takes the failed-fetch path: Failed, or RefreshFailed when a list had
loaded. Only the overlay's league worker calls `list_leagues`, so CLI resolution is unaffected. New
test: `an_empty_league_list_is_an_error`. CLAUDE.md got one clause.

---

_Fixed: 2026-10-18_
_Fixer: Claude (gsd-code-fixer)_
_Iteration: 1_
