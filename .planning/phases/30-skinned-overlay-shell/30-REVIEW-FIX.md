---
phase: 30-skinned-overlay-shell
fixed_at: 2026-10-16T00:00:00Z
review_path: .planning/phases/30-skinned-overlay-shell/30-REVIEW.md
iteration: 2
findings_in_scope: 2
fixed: 2
skipped: 0
status: all_fixed
---

# Phase 30: Code Review Fix Report

**Fixed at:** 2026-10-16
**Source review:** .planning/phases/30-skinned-overlay-shell/30-REVIEW.md
**Iteration:** 2

**Summary:**
- Findings in scope: 2 (2 warning; Info IN-01..IN-08 out of scope)
- Fixed: 2
- Skipped: 0

Verification after both fixes: `cargo test` green (lib 1142 passed, all integration suites ok),
`cargo fmt --all --check` clean, clippy count 126 (unchanged), `tests/fixtures` untouched.

## Fixed Issues

### WR-01: Some edit and revert paths still leave a stale event on the one status line

**Files modified:** `src/overlay/app.rs`
**Commit:** 4f601f2
**Applied fix:** New `clear_events_on_edit` clears `apply_message`, `list_status` and
`config_message`. It is called at the start of RULES `revert`, `stage_add_economy` and
`stage_remove_economy` (before each sets its own message), in `run_row_actions` (replacing the
inline clears), and in `handle_config_actions` for every config edit plus `ConfigAction::Revert`
(so CONFIG edits now clear the list status too). Test:
`reverts_staging_and_config_edits_clear_stale_events` (refused-Apply Error line, then each tab's
Revert; a success line, then economy staging; a list status, then a tier edit).

### WR-02: A disabled Overwrite is never explained, and an invalid config still enables it

**Files modified:** `src/overlay/apply.rs`, `src/overlay/shell.rs`, `src/overlay/app.rs`
**Commit:** 2a5d6fa
**Applied fix:**
- `apply::overwrite_enabled` takes `config_blocks` (a loaded config that fails `validate()`) and
  returns false while it is set. Every Apply validates a loaded config, so this matches v0.5's
  CONFIG OVERWRITE gating. A config that failed to load is not validated by Apply and does not block.
- New `shell::OverwriteBlock` (TierErrors, ConfigInvalid, RulesBlocking, Damaged, Nothing), worked
  out from `ConfigBarInputs` (new `config_invalid` field) in the same order as the Apply slot's
  reason chain. A disabled Overwrite slot's tooltip is `block.tip()` and no longer "Replace the file
  with your edits.".
- `PopupData::ChangedOnDisk` gained `block`. With Overwrite unavailable, the popup adds one short
  sentence, e.g. "Fix or disable the flagged rules to overwrite instead." (none for Nothing).
- `OverlayApp::overwrite_block()` feeds the popup from the same inputs.
Tests: `a_disabled_overwrite_explains_itself` (bar tips and popup clause),
`tier_errors_block_overwrite_and_the_popup_says_why` (app level: real rig, styles.json refusal,
tier error -> no Overwrite and the popup explains why), `overwrite_covers_config_only_refusals`
extended.
Status: fixed: requires human verification (enable logic; check the wording in the skinned popup).

## Iteration 1

All 10 iteration-1 findings were fixed (details in git history and the iteration-1 report):

| Finding | Commit |
|---------|--------|
| CR-01 CONFIG-tab Reload routing | 86a84a1 |
| CR-02 Apply status colours | e90f6c0 |
| WR-01 stale events on conceal / rule edits | db42ebd |
| WR-02 status-line jump switches to RULES | 470ce79 |
| WR-03 one Overwrite decision for bar and popup | 131b68b |
| WR-04 CONFIG Revert off during a refusal | 12a0d75 |
| WR-05 ABOUT footer spaces | 10dfb46 |
| WR-06 switch track and tier-error bar colours | 5888173 |
| WR-07 screenshot-mode guard and staging | 78ba579 |
| WR-08 debug `ff` argument parsing | be95c96 |

---

_Fixed: 2026-10-16_
_Fixer: Claude (gsd-code-fixer)_
_Iteration: 2_
