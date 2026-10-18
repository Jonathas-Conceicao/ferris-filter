---
phase: 39-league-picker-price-source
fixed_at: 2026-10-18T00:00:00Z
review_path: .planning/phases/39-league-picker-price-source/39-REVIEW.md
iteration: 3
findings_in_scope: 1
fixed: 1
skipped: 0
status: all_fixed
---

# Phase 39: Code Review Fix Report

**Fixed at:** 2026-10-18
**Source review:** .planning/phases/39-league-picker-price-source/39-REVIEW.md
**Iteration:** 3

**Summary:**
- Findings in scope: 1 (WR-01). The Info findings are out of scope; IN-08 was also fixed because it
  was a small, clean change in the same code (see below). IN-01 to IN-07 and IN-09 are unchanged.
- Fixed: 1 (plus IN-08)
- Skipped: 0

Verification after both commits: `cargo test --lib -- overlay:: settings:: sync::` passed (977 tests,
with `FERRISFILTER_HOME` / `FERRISFILTER_GAME_DIR` set to empty dirs under the temp folder and no
network). `cargo build --all-targets` produced no warnings. `cargo clippy --all-targets` reports the
same count as before, and none of it is on a changed line. `rustfmt --check` on the changed files
shows only the existing screenshot hunk in `app.rs`, which I left alone.

Process note: as in iterations 1 and 2, the commits were made directly in the given worktree
(`.claude/worktrees/milestone-closeout`). No nested worktree, temp branch or recovery sentinel was
created.

## Fixed Issues

### WR-01: Apply & quit with other work plus unused tier edits loses both the Apply outcome and the "Not quit" warning when an in-game reload is sent

**Files modified:** `src/overlay/app.rs`
**Commit:** e86ca8b
**Applied fix:** I followed the reviewer's suggestion. A new free function,
`show_not_quit_warning(&mut apply_message, reload_pending.as_mut())`, replaces the direct overwrite
in the Apply & quit branch:
- When a reload is pending and its interim line is still showing, the warning is appended to both
  `interim` and `extras`, and the status line becomes the extended interim line. It still equals
  `reload_pending.interim`, so `clear_events_on_conceal` keeps it through the hide that the reload's
  focus change causes. `landed_reload_message` still matches it, and the result line (success or
  `could not reload in game: <cause>` with the Refresh reminder) ends with the warning.
- `PendingReload` has a new `warning` flag, set by the fold. When it is set, the landed line is shown
  as a Warning instead of the kind `applied_message` would pick (Ok for a successful reload).
- With no reload pending, or once its interim line has been replaced, the warning is shown on its own,
  as before.

**Test:** `the_not_quit_warning_rides_on_a_pending_reload_line` simulates `reload_pending` directly.
It checks that the warning is folded into the interim line (Warning kind), survives
`apply_message_after_conceal`, and ends both the success and the failure landed messages. It also
checks that with no matching interim line the warning stands alone and the pending reload is left
unchanged.

**Status:** fixed: requires human verification (message state handling across the reload's
conceal; the `Decision::Send` path itself only runs with a real game window).

### IN-08 (Info, fixed as well): with only unused tier edits, the Quit confirm offered an Apply & quit that cannot apply

**Files modified:** `src/overlay/shell.rs`, `src/overlay/app.rs`, `CLAUDE.md`
**Commit:** d3d94b5
**Applied fix:**
- `PopupData::Quit` now carries `tier_edits_only`. When it is set, the **Unapplied changes** popup
  reads `Tier edits apply only with Live prices. Quit and lose them?` and offers only **Quit without
  applying** (danger, same tooltip) and **Cancel**. Cancel is still last, so Escape or a click outside
  still picks it.
- `shell::quit_popup_choice(tier_edits_only, clicked)` maps the clicked index back to the full
  three-button set, following the `changed_on_disk_choice` pattern.
- `OverlayApp::quit_tier_edits_only` is refreshed every frame next to `overwrite_live`, by
  `refresh_quit_tier_edits_only()`: true only while the confirm shows, with unwritable tier edits and
  no applicable work. Both the popup spec and the dispatch read that one field, so the buttons drawn
  and the choice mapping always agree.
- The Apply & quit guard from iteration 2 is unchanged. It still applies when the confirm offers Apply
  & quit with other work pending.
- CLAUDE.md: the Economy-tab clause now says that when those tier edits are all that is pending, the
  Quit popup offers only Quit without applying / Cancel, and that with other work, Apply & quit applies
  it and stays open.

**Tests:** `apply_and_quit_under_fallback_keeps_unused_tier_edits_and_stays_open` now checks the
two-button spec and body, Cancel (stays open, edits kept), the first button (quits without applying),
the unchanged Apply & quit guard path, and that the mixed case still offers Apply & quit. The shell
popup tests cover both `Quit` variants, the index mapping, and the safe-last-button rule.

**Status:** fixed.

## Iteration 2 (record)

Both iteration-2 findings were fixed. WR-01 `8611871`: under Fallback, unused tier edits are kept
(Economy ` *`) but are not counted as unapplied work, Quit still guards them, and Apply & quit stays
open with a warning instead of losing them. WR-02 `9b93f85`: `reload()` saves and restores a filter's
applied price source through the shared `rebind_applied_policy`, like a filter switch. The full report
is in `39-REVIEW-FIX.iter2.md`.

## Iteration 1 (record)

All six iteration-1 findings were fixed: CR-01 `8382e3a` (unused out-of-order cutoffs neither refuse
Apply nor get written), WR-01 `8b9d2cf` (an Apply under Fallback keeps the pending cutoff text and
errors), WR-02 `32add33` (`left_policies`: a filter switch or folder pick keeps each filter's applied
price source), WR-03 `de006a9` (`save_keys`: league and price-source saves write only their own key
and never replace an unusable file), WR-04 `d4e6bf1` (`RefreshFailed` keeps the last loaded list
usable), WR-05 `14dd6a6` (an empty poe.ninja league list is a failed fetch). The full report is in
`39-REVIEW-FIX.iter1.md`.

---

_Fixed: 2026-10-18_
_Fixer: Claude (gsd-code-fixer)_
_Iteration: 3_
