---
phase: 34-economy-about-tabs
reviewed: 2026-10-16T00:00:00Z
depth: standard
iteration: 2
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
  warning: 0
  info: 0
  total: 0
status: clean
---

# Phase 34: Code Review Report (iteration 2)

**Status:** clean

## Iteration-1 verification

| Finding | Status | Evidence |
|---|---|---|
| WR-01 Revert dropped the staged economy op before the tier-discard popup | Fixed | `economy_revert_pressed` (app.rs) only arms the popup when tiers are dirty. It unstages directly only when there are no tier edits. The `DiscardConfig` choice 0 branch unstages when the scope is `Tiers`. Choice 1 touches nothing. The popup count adds the staged op. Test `economy_revert_with_tier_edits_keeps_the_staged_op_until_confirmed` covers Keep and Discard. |
| WR-02 A staged economy op marked RULES instead of ECONOMY | Fixed | `WorkingFilter::rules_edited` is recomputed alongside `dirty` at the single recompute site (rules.rs:873-874). `tab_markers` takes `economy_staged` and marks ECONOMY with `tiers_dirty \|\| economy_staged`. The caller passes `rules_edited()`. Tests `a_staged_economy_op_is_not_a_rules_edit` and `a_staged_economy_op_marks_economy_not_rules`. |
| WR-03 Broken or empty prices.json shown as a fresh sync | Fixed | `EconomyInfo.problem` is set for a load error or an empty item list, and `mtime_secs` is `None` in that case. The tooltip carries the reason. A missing file keeps the old text. Tests extended for malformed, empty and missing files. |
| IN-01 to IN-05 | Open, not in the fix scope | Non-blocking notes from iteration 1, unchanged. |

## Regression check of the three fix commits

- **Marker split:** `dirty` (used for Apply, Revert and switch-filter decisions) is unchanged. Only the tab marker uses `rules_edited`, so a staged op still counts as unapplied for Apply and quit. No behaviour is lost.
- **Revert flow:** the popup disarms itself when the tier scope has no edits. It is armed only when `tiers_dirty` is true, so the staged-only case cannot open an empty popup.
- **Popup confirm:** `RevertScoped(Tiers)` runs first and then `unstage_economy_pressed`. Apply state is recomputed in the second call, so it ends consistent.
- **Problem display:** an empty `items` list is now reported as a problem. The fallback path still applies in `generate`, which is the same as before, so only the display changed.
- No new defects found in the three commits.
