---
phase: 30-skinned-overlay-shell
reviewed: 2026-10-16T00:00:00Z
depth: standard
iteration: 2
files_reviewed: 19
files_reviewed_list:
  - Cargo.toml
  - src/overlay/app.rs
  - src/overlay/apply.rs
  - src/overlay/body.rs
  - src/overlay/config_view.rs
  - src/overlay/economy.rs
  - src/overlay/mod.rs
  - src/overlay/rule_list.rs
  - src/overlay/screenshot.rs
  - src/overlay/shell.rs
  - src/overlay/theme.rs
  - src/overlay/tray.rs
  - src/overlay/win.rs
  - src/skin/icons.rs
  - src/skin/mod.rs
  - src/skin/popup.rs
  - src/skin/widgets.rs
  - src/skin/window.rs
  - src/sync/prices.rs
findings:
  critical: 0
  warning: 2
  info: 8
  total: 10
status: issues_found
---

# Phase 30: Code Review Report (iteration 2)

**Reviewed:** 2026-10-16
**Depth:** standard (fix commits `86a84a1..be95c96` traced against the iteration-1 findings, plus the
phase diff `f34cb86..HEAD` re-checked for regressions)
**Files Reviewed:** 19
**Status:** issues_found

## Summary

All ten iteration-1 Critical/Warning fixes are in place. I traced each one through the per-frame
`ui()` flow; none brings back the data loss or wrong colours of iteration 1. `cargo test --lib
overlay::` passes (563 tests).

- **CR-01 (reload routing): correct.** `bar_reload_target` sends a CONFIG-tab bar Reload to
  `ConfigAction::Reload` only while a changed-on-disk refusal is pending. Otherwise it goes to
  `reload_pressed` → `reload()`, which re-reads the filter and reloads the config only when the
  root moved or the config had failed (`app.rs:1600-1606`). CONFIG edits survive the
  damaged-markers path. During a refusal, both tabs reach `reload_changed`; the one exception is a
  failed config, which retries only the config. That matches v0.5.
- **CR-02 (status colours): correct.** Every assignment to `apply_message` now stores its kind.
  Failures are stored as `Error` (`app.rs:1190`). Successes go through `applied_message`, which
  picks Warning for messages that carry `RELOAD_INSTRUCTION`, so the skipped-reload and
  failed-reload results are Warning and "Wrote <path>." is Ok. The conceal and landed-reload paths
  keep or recompute the kind. `event_from` no longer infers anything.
- **WR-03 (Overwrite enablement): correct.** Bar and popup agree. `overwrite_live` is computed
  again just before the popup is drawn, and the clicked index is mapped back through
  `changed_on_disk_choice` in the same frame, so the drawn buttons and the dispatch cannot drift
  apart. Escape still answers Keep editing. A config-only refusal with a `LoadFailed` filter can now
  be overwritten, as in v0.5. One gap remains (WR-02 below).
- **WR-04 (CONFIG Revert during a refusal), WR-05 (ABOUT spaces), WR-06 (switch track and tier
  bar), WR-08 (debug argument parsing):** correct.
- **WR-07 (screenshot guard):** correct. The `Temp`/`tmp` component check runs on the
  canonicalised temp path. `stage_into` never overwrites a file in a folder it did not create.
  Staging follows the temp home's `ff-settings.json` filter name.
- **WR-01 (stale events):** fixed for conceal and for rule edits. Several other edit and revert
  paths still leave an older event on the single status line (WR-01 below).

No new Critical issues. Two Warnings remain. All five iteration-1 Info items are still present, and
three Info items are new.

## Warnings

### WR-01: Some edit and revert paths still leave a stale event on the one status line

**File:** `src/overlay/app.rs:1535-1545` (`revert`), `src/overlay/app.rs:1548-1567`
(`stage_add_economy` / `stage_remove_economy`), `src/overlay/app.rs:1655-1667` (the
`handle_config_actions` clear list)
**Issue:** The iteration-1 fix clears events on conceal and on a row edit. These paths still skip
the rule that "an event is cleared by the next edit":
- **RULES Revert** (`revert`) sets `list_status = REVERTED_MESSAGE` but leaves `apply_message`.
  Since `apply_message` outranks `list_status` in `event_from`, a failed Apply such as "Apply
  refused: 1 invalid block…" (Error) stays on the line after the user reverted the edits it was
  about. "Reverted to the last applied filter." never shows.
- **CONFIG Revert** is not in the `handle_config_actions` clear list (`AddStyle | EditStyle |
  DeleteStyle | RenameStyle | MoveStyle | TierText`). A refused Apply caused by tier errors keeps
  its Error line after Revert has discarded those tiers. "Reverted config changes." is hidden
  behind it.
- **Economy Add/Remove staging** marks the state `Unapplied` but clears neither `apply_message`
  nor `list_status`. After a successful Apply, the green "Wrote <path>." stays while there is now
  an unapplied change. That hides the "Filter out of date" indicator (row 9), the tab's main
  state signal, until the stripe is hidden.
- **CONFIG edits** clear `apply_message` but not `list_status`. A RULES-side "Reverted to the last
  applied filter." therefore stays above the grammar gate and the indicator while the user edits
  styles.

With one status line, each of these hides the standing state the line exists to show.
**Fix:** Clear the event fields in one place on every edit or revert:
```rust
fn clear_events_on_edit(&mut self) {
    self.apply_message = None;
    self.list_status = None;
    self.config_message = None;
}
```
Call it at the start of `revert`, `stage_add_economy` and `stage_remove_economy`, before each of
them sets its own message. In `handle_config_actions`, add `ConfigAction::Revert` to the clear
list and clear `list_status` there too. Add an app test: a refused Apply, then Revert on each tab,
must show the revert message.

### WR-02: A disabled Overwrite is never explained, and an invalid config still enables it

**File:** `src/overlay/apply.rs:453-466` (`overwrite_enabled`), `src/overlay/shell.rs:289-295`
(Overwrite slot), `src/overlay/shell.rs:595-611` (`popup_spec` ChangedOnDisk)
**Issue:** The WR-03 fix brought the bar and the popup into agreement, but neither tells the user
why Overwrite is missing, and the enable rule has a gap:
1. When `overwrite` is false (enabled-rule grammar errors, damaged markers), the popup drops the
   button and the sentence. The bar slot shows a dead Overwrite whose tooltip is still "Replace the
   file with your edits." The only explained action left is Reload, whose text says "Reload
   discards your edits." A user with grammar errors facing a changed-on-disk refusal is led toward
   the destructive choice. Fixing the grammar errors, which re-enables Overwrite, is never
   mentioned. v0.5 kept the reason visible in the multi-line outcome area. The non-refusal Apply
   slot already has a reason chain (`tier_errors` → `load_error` → `rules_blocking` → `damaged`);
   the Overwrite slot does not.
2. `overwrite_enabled` returns true whenever `filter_overwrite` is true, without checking
   `config_valid`. On CONFIG with tier errors and a `styles.json` refusal, Overwrite is live. The
   click opens the popup, its Overwrite runs `run_apply(true)`, and that fails on tier validation.
   v0.5's CONFIG OVERWRITE was gated like the CONFIG Apply, validity included.
**Fix:** Pass a reason into the slot and the popup. In `bar_state`'s `needs_reload` branch, reuse
the reason chain when `!actions.overwrite`:
```rust
tip: if actions.overwrite { Some(apply::OVERWRITE_TOOLTIP) }
     else if config.tier_errors { Some(config_state::APPLY_BLOCKED_TOOLTIP) }
     else if config.rules_blocking { Some(apply::APPLY_BLOCKED_TOOLTIP) }
     else if config.damaged { Some(apply::DAMAGED_MARKERS_MESSAGE) }
     else { Some(NOTHING_TO_RELOAD /* or a dedicated text */) },
```
When `overwrite == false`, add a short clause to the popup body, for example "Fix the flagged
rules to overwrite instead." Also require `config_valid` when the config has edits:
`needs_reload && config_ok_or_clean && (filter_overwrite || config_apply_enabled(..))`.

## Info

### IN-01: Dead or duplicated code left by the migration (still present)

**File:** `src/overlay/win.rs:175,682` (`work_area`, unused outside a test);
`src/overlay/layout.rs:11-39` (`STRIPE_WIDTH`, `dock_rect*` right-edge docking);
`src/overlay/config_view.rs:152-160` (`ConfigViewCtx::{changed_on_disk, apply_enabled,
rules_blocking, filter_damaged, indicator}` written, never read); `src/overlay/config_view.rs:229`
(`delete_prompt` is test-only; `app.rs` `popup_spec_for(DeleteStyle)` reimplements it);
`src/skin/window.rs:55,82` (`primary_work_area_px` duplicates `win::monitor_work_area_px`).
**Fix:** Remove them, or route `popup_spec_for(DeleteStyle)` through `config_view::delete_prompt`.

### IN-02: Stale documentation (still present)

**File:** `src/overlay/app.rs:3` ("docking the stripe to the right screen edge"; it docks left);
`src/skin/icons.rs:22` (`Padlock` documented as "Locked rule (economy)" but marks `can't enable`).
**Fix:** Update the comments.

### IN-03: "Last sync" applies today's UTC offset to an old timestamp; magic `id == 2` (still present)

**File:** `src/overlay/shell.rs` (`last_sync_text`), `src/overlay/win.rs:290`
**Fix:** Convert the mtime with `FileTimeToLocalFileTime`/`SystemTimeToTzSpecificLocalTime`, and use
the `TIME_ZONE_ID_DAYLIGHT` constant.

### IN-04: The Delete-rule popup falls back to "Rule 0: " when its row vanished (still present)

**File:** `src/overlay/app.rs:1393-1397`
**Fix:** Clear `list.confirm_delete` when `working.row(id)` is `None` instead of opening the popup.

### IN-05: A test hook writes into egui temp memory every frame in production (still present)

**File:** `src/skin/popup.rs:174-176`
**Fix:** Gate the `insert_temp(... "ff_modal_rects")` with `#[cfg(test)]`.

### IN-06: `theme::poe2_visuals` is dead in production and maps hyperlinks to near-black

**File:** `src/overlay/theme.rs:44-48,73`
**Issue:** Since WR-06, `WARM_ACCENT` (`pal::INPUT_FILL`, `#070707`) is used only by
`poe2_visuals`, which only tests call; `skin::widgets` installs the live visuals with
`hyperlink_color = GOLD`. The doc comment still calls `WARM_ACCENT` the "red-bronze USE DEFAULT"
destructive accent. A future caller of `poe2_visuals()` would get invisible hyperlinks.
**Fix:** Delete `poe2_visuals` and its tests (and `WARM_ACCENT` if nothing else needs it), or point
`hyperlink_color` at `pal::GOLD` and fix the doc comment.

### IN-07: A CONFIG-tab bar Reload now discards RULES edits without warning

**File:** `src/overlay/app.rs:2464-2468`, `src/overlay/app.rs:1572-1607`
**Issue:** This follows from the CR-01 fix and is accepted v0.5 RELOAD semantics. With damaged
markers (or a failed load) and no refusal, a Reload pressed on CONFIG runs the full `reload()`,
which replaces the working filter and drops unapplied rule edits made earlier on RULES. The tooltip
("Discard your edits and load the file from disk.") does not say which tab's edits. The RULES tab
does show ` *` for this case.
**Fix:** Optional. Word the CONFIG-tab tooltip as "Discard your rule edits and reload the filter
from disk." when `rules_dirty`.

### IN-08: CR-01 and WR-03 are tested only at the pure-function level

**File:** `src/overlay/app.rs` tests (`bar_reload_on_config_reloads_the_filter_unless_a_refusal_is_pending`),
`src/overlay/apply.rs` tests (`overwrite_covers_config_only_refusals`)
**Issue:** No app-level test drives a CONFIG bar Reload with config edits and a Damaged filter to
check that `config.is_dirty()` survives and the filter is re-read. No test checks that
`popup_spec_for(ChangedOnDisk)` follows `overwrite_live` after a same-frame refusal. These were
the two "requires human verification" fixes.
**Fix:** Add both tests with the existing `popup_app`/`rig` helpers.

---

_Reviewed: 2026-10-16_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
