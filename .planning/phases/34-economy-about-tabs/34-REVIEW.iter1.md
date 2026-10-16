---
phase: 34-economy-about-tabs
reviewed: 2026-10-16T00:00:00Z
depth: standard
files_reviewed: 19
files_reviewed_list:
  - assets/poe2/manifest.toml
  - src/bin/ff-proto.rs
  - src/overlay/about_view.rs
  - src/overlay/app.rs
  - src/overlay/apply.rs
  - src/overlay/body.rs
  - src/overlay/config_state.rs
  - src/overlay/config_view.rs
  - src/overlay/mod.rs
  - src/overlay/rule_list.rs
  - src/overlay/rules.rs
  - src/overlay/screenshot.rs
  - src/overlay/shell.rs
  - src/skin/colour_picker.rs
  - src/skin/mod.rs
  - src/skin/popup.rs
  - src/skin/textures.rs
  - src/skin/widgets.rs
  - tools/ff-art/src/main.rs
findings:
  critical: 0
  warning: 3
  info: 5
  total: 8
status: issues_found
---

# Phase 34: Code Review Report

**Reviewed:** 2026-10-16
**Depth:** standard (diff `fd2d1fd..HEAD` hunks plus surrounding code)
**Files Reviewed:** 19
**Status:** issues_found

## Summary

The Phase 34 changes are largely sound. The riskiest areas hold up:

- **Writes outside Apply:** Staging economy add/remove and the scoped reverts only touch in-memory state (`WorkingFilter.pending`, `ConfigState`). `stage_add_economy` reads `styles.json`, `tiers.json` and the prices through `dynamic::economy_body` but writes nothing. The new tests also assert this.
- **Scoped revert:** `ConfigState::revert_scope` keeps the other scope's edits and tier-validation errors. `scope_dirty_count` and `revert_armed` are consistent, and the popup disarms itself when its scope has no edits.
- **Relative-time formatting:** `relative_sync_text` handles a future mtime (saturating subtract gives "Just now") and a negative local offset (clamped). It does not panic.

The defects below are about consistency between the tabs' unapplied markers and the Revert flow, plus a few misleading displays on the new ECONOMY tab. There are no blockers.

## Warnings

### WR-01: ECONOMY Revert drops the staged economy op before the tier-discard popup is confirmed

**File:** `src/overlay/app.rs:2834-2850` (the `ShellTab::Economy` arm of the Revert click), `src/overlay/app.rs:1841-1855` (`unstage_economy_pressed`)
**Issue:** When both `economy_staged` and `tiers_dirty` are true, a single Revert click unstages the economy add/remove immediately and unconditionally. It then opens the "Discard config changes?" popup for the tiers. If the user picks "Keep editing", the staged economy op is already lost. The popup count and text only mention tier edits, so the user is never told the staged op was discarded. Tier discards are guarded by a confirmation and the economy op is not, so the same button behaves inconsistently.
**Fix:** Either defer `unstage_economy_pressed()` to the popup's "Discard" branch when `tiers_dirty` is also true, or unstage only in the `else` case:
```rust
ShellTab::Economy => {
    if tiers_dirty {
        self.config_ui.revert_scope = RevertScope::Tiers;
        self.config_ui.revert_armed = true; // dispatch_popup(DiscardConfig) also calls unstage_economy_pressed()
    } else if economy_staged {
        self.unstage_economy_pressed();
    }
}
```
Then make the `Popup::DiscardConfig` choice 0 branch also unstage when `scope == Tiers`, and count the staged op in the popup text.

### WR-02: A staged economy add/remove marks the RULES tab with ` *`, never ECONOMY

**File:** `src/overlay/shell.rs:77-94` (`tab_markers`), `src/overlay/app.rs:2593,2679`
**Issue:** `tab_markers` takes `rules_dirty = working.is_dirty()`. `WorkingFilter.dirty` is set when `pending.is_some()` (`rules.rs:870`), so a staged economy op counts as a rules edit. The edit is made and reverted on the ECONOMY tab, but:
- On ECONOMY with only a staged add, the strip shows `Rules *` and `Economy` unmarked.
- From CONFIG or ABOUT the RULES tab is marked.
- The ECONOMY tab is marked only for tier edits.

The doc comment says "RULES owns ... a staged economy add/remove", but the Revert and scoping logic in the same phase makes ECONOMY the owner: `bar_state` gives ECONOMY the Revert for `economy_staged`, and RULES Revert also drops it. The marker and the Revert owner disagree, and so does the "scoped Revert" story.
**Fix:** Pass `economy_staged` to `tab_markers` and mark ECONOMY with `tiers_dirty || economy_staged`. Use `rules_dirty && !(only the pending economy op)` for RULES, for example via a `WorkingFilter::rules_edited()` that ignores `pending`. If the intent really is to mark both, update the comment and add a test that pins the choice.

### WR-03: ECONOMY "Price source" and "Last sync" misreport an empty or malformed prices.json

**File:** `src/overlay/app.rs` (`EconomyInfo::read`), `src/overlay/shell.rs` (`economy_tab` rows 1 and 3)
**Issue:** `EconomyInfo::read` maps any parse error to `priced_items = 0` and `live = false`, but still returns the file's mtime. For an empty or corrupt `prices.json` (for example a sync interrupted mid-write), the tab shows "Last sync: Just now", "Priced items: 0" and "Fallback prices". The tooltip on that row says "No prices.json yet", which is false. The user is told a sync just happened while the filter silently falls back, and no hint is given that the file is unreadable.
**Fix:** Keep the load error and use it. For example, add `error: Option<String>` to `EconomyInfo` (from `load_prices_optional(...)`'s `Err`). Show "Unreadable prices.json" in `pal::ERROR` with the message on hover. Report mtime as `None`, or label it "File modified", when the file is not live.

## Info

### IN-01: `ConfigAction::Revert` is now dead in production code

**File:** `src/overlay/config_view.rs:75`, `src/overlay/app.rs:1981,2058-2067`
**Issue:** The CONFIG and ECONOMY Revert paths now use `RevertScoped`. `ConfigAction::Revert` (and `ConfigState::revert` with its handler) is only produced by two tests, which exercise a path the UI no longer takes.
**Fix:** Remove the variant and handler and port the two tests to `RevertScoped`, or document it as the "all" revert if it is kept on purpose.

### IN-02: `show_tiers_section` takes an unused `skin` parameter

**File:** `src/overlay/config_view.rs:160-166`
**Issue:** `let _ = skin;` only silences the unused-parameter warning.
**Fix:** Drop the parameter, or use it. The caller already has `skin` in scope, so the change is trivial.

### IN-03: `now_secs` falls back to 0 when the system clock is before the epoch

**File:** `src/overlay/app.rs` (`ShellTab::Economy` arm, `now_secs` computation)
**Issue:** `.map_or(0, ...)` makes every mtime read as "Just now" if the clock is set before 1970. This is an unlikely edge, but a future-dated mtime is already collapsed the same way, so clock skew in either direction hides the real age.
**Fix:** Optional: when `mtime > now`, show the absolute date from the tooltip text instead of "Just now".

### IN-04: ECONOMY info is only refreshed on tab switch and after an in-app sync

**File:** `src/overlay/app.rs` (the `ShellTab::Economy` arm of tab switching; Reload handler)
**Issue:** A `ferrisfilter sync` run from a terminal while the overlay is open, or the Reload button, does not refresh `economy_info` while the ECONOMY tab stays selected. The displayed item count and age stay stale until the user leaves and re-enters the tab.
**Fix:** Call `refresh_economy_info()` from `reload()`, or re-read when the file's mtime differs on the 60 s repaint tick (a cheap `metadata` call).

### IN-05: Small robustness and cosmetic notes

- `src/overlay/about_view.rs:108-125`: the footer is laid out from the bottom without a scroll area. In a window shorter than about 450pt it overlaps the header block. This is fine for the full-height docked window, but the `about_renders_in_a_cramped_page` test only checks that it does not panic.
- `tools/ff-art/src/main.rs:760-785`: the second pass can yield a bounding box 1px narrower than `width` due to rounding. The output then has a transparent right column (`f.width.min(x1 - x0 + 1)` hides this). Use the real bbox width for the image, or re-scale until it equals `width`.
- `src/overlay/app.rs:1858-1873` (`stage_add_economy`): sets `ApplyState::Unapplied` even when `working.stage_add_economy` was a no-op (the section is already present). The button is disabled in that state, so this is unreachable today; guard it on `economy_staged()` changing.
- `src/overlay/app.rs:2066-2090` (`RevertScoped`): recomputes `state_after_revert` from `rules_dirty || config_dirty`, but `unstage_economy_pressed` also includes `style_pending`. The next frame self-heals it, but one helper would keep the two in step.

---

_Reviewed: 2026-10-16_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
