---
phase: 26-uat-feedback-fixes-3
fixed_at: 2026-10-14T00:00:00Z
review_path: .planning/phases/26-uat-feedback-fixes-3/26-REVIEW.md
iteration: 1
findings_in_scope: 10
fixed: 9
skipped: 1
status: partial
---

# Phase 26: Code Review Fix Report

**Fixed at:** 2026-10-14
**Source review:** .planning/phases/26-uat-feedback-fixes-3/26-REVIEW.md
**Iteration:** 1

**Summary:**
- Findings in scope: 10 (2 critical, 8 warning; Info out of scope)
- Fixed: 9
- Skipped: 1

Verification: full `cargo test` passes (976 lib tests plus every integration suite, `artifact_guard` / `generate_snapshot` included, so fixtures are byte-identical). `cargo fmt --check` is clean. No generator, CLI, config.json, styles.json or tiers.json changes.

## Fixed Issues

### CR-01: RULES Revert after a CONFIG rename re-orphans legacy tags

**Files modified:** `src/overlay/rules.rs`, `src/overlay/app.rs`, `src/overlay/config_state.rs`
**Commit:** a197a40
**Applied fix:** The legacy-tag rename migration is now a pure function, `migrate_legacy_tags(doc, olds, new)`. `retag_renamed_legacy` applies it to the document and to the applied baseline (`applied_text`). The migration is therefore not a user edit: it never makes RULES dirty, and a RULES Revert, which re-parses the baseline, keeps it. `WorkingFilter` also remembers the applied style set (`legacy_base`, passed in by `sync_working_styles` and after Apply through the new `set_styles_over` and `ConfigState::applied_styles`). This lets a freshly parsed document still follow a pending rename after a RELOAD or filter switch. Migration skips any tag that the new set resolves by id. It resolves legacy tags against the applied set first. As a result, a newly added style with the old name cannot take over the renamed style's tags (the "worse" case in the review). Regression tests: legacy tag -> rename -> sync -> RULES revert -> Apply writes `<id> locator`; RELOAD with a pending rename; newcomer named `kite`.
**Status:** fixed: requires human verification (state/logic change)

### CR-02: Between can write an impossible pair silently when a bound is strict

**Files modified:** `src/overlay/conditions.rs`, `src/overlay/body.rs`
**Commit:** 8cfa27a
**Applied fix:** Added `inclusive_lo` / `inclusive_hi`, a strictness-aware `between_ok(lo_op, lo, hi_op, hi)`, and `bound_clamps(...)`. `between_switch` now inserts `<= max(limit, v+1)` after `>` and `>= min(limit, v-1)` after `<`. The min/max sliders clamp against the strict-adjusted other bound. The min > max hint now flags a loaded pair that matches nothing, such as `> 50` / `<= 50`. Tests cover a strict switch at the slider edge, the exclusive check, and the clamps.
**Status:** fixed: requires human verification (logic change)

### WR-01: CONFIG reorder never reaches the RULES style picker before Apply

**Files modified:** `src/overlay/app.rs`
**Commit:** cd8da1e
**Applied fix:** `sync_working_styles` now compares name order as well as set equality, because `IndexMap` equality ignores order. Regression test: reorder -> sync returns true and the RULES names match the CONFIG order.

### WR-02: A second tray Quit with the confirm open does not reveal a hidden stripe

**Files modified:** `src/overlay/app.rs`, `src/overlay/apply.rs`
**Commit:** 74f4a6f
**Applied fix:** New pure `apply::quit_step_shows_confirm` returns true for both `OpenConfirm` and `KeepConfirm`. The tray Quit handler uses it to reveal a hidden stripe in both cases. Unit test added.

### WR-03: Between offered on a line whose keyword already has other lines

**Files modified:** `src/overlay/body.rs`, `src/overlay/conditions.rs`
**Commit:** be52086
**Applied fix:** New pure `conditions::sole_line` and `numeric_op_choices_for(kw, cur, can_between)`. `Card` carries `can_between`, computed for plain condition cards and for single lines inside the Defences card. The dropdown lists Between only for a keyword's only line, and `op_controls` ignores a Between pick otherwise. Unit test added.

### WR-04: CONFIG Revert after a rename leaves a phantom RULES edit

**Files modified:** `src/overlay/rules.rs`, `src/overlay/app.rs`
**Commit:** a197a40 (same change as CR-01)
**Applied fix:** The migration is applied to the baseline too (see CR-01), so `doc == applied_text` stays true after rename and after a CONFIG revert. `t5hw2k locator` resolves by id under the reverted set, and no migration runs. Regression test `a_config_revert_after_a_rename_leaves_the_rules_clean`. The existing rules test that asserted `is_dirty()` after a rename migration now asserts the opposite, which is intended.

### WR-05: Rename drafts outlive Revert, reload and Apply

**Files modified:** `src/overlay/config_view.rs`, `src/overlay/app.rs`
**Commit:** b5c429d
**Applied fix:** New `ConfigUi::reset_for_new_styles` runs `disarm_all` and clears `rename_drafts`. It now replaces `disarm_all` at CONFIG Revert, CONFIG reload, `reload_changed`, and successful Apply. It also runs when `poll_styles`' `follow_disk()` reports a re-read. Tab switches still only disarm. Unit test added.

### WR-06: Filters get load-assigned ids that styles.json may never receive

**Files modified:** `src/overlay/rules.rs`, `src/overlay/app.rs`
**Commit:** 8c811c3
**Applied fix:** New `WorkingFilter::text_with_styles_keeping_tags` (retag = false). The `None`-config branch of `apply_everything` uses it, so an Apply that does not write styles.json rewrites actions but never puts load-assigned ids into tags. Legacy name tags keep resolving by name. Unit test added. One gap remains: picking a style in the rule editor while CONFIG failed to load still writes `<id> <name>` with the in-memory id. That path is unchanged.

### WR-08: Modal Quit paths shut down after this frame's instance show-request poll

**Files modified:** `src/overlay/app.rs`
**Commit:** a2572b2
**Applied fix:** The confirm's "Quit without applying" now sets `quit_next_frame` and requests a repaint. The shutdown (`begin_shutdown` -> Close) runs at the top of the next frame, next to the `quit_after_apply` check and before the INST-01 show-request poll. That is the same Phase 22 order the other quit paths use. "Apply & quit" was already deferred through `quit_after_apply`. No automated test: it needs a live `App`/window, which the test suite never builds.
**Status:** fixed: requires human verification (frame-ordering change, covered by manual UAT)

## Skipped Issues

### WR-07: Deleting and re-adding a same-named style drops all its tags on Apply

**File:** `src/models/config.rs:582-590`, `src/styled.rs:194-215`
**Reason:** Not fixed. The review offered two fixes, and neither applies:
- **Name fallback for `<unknown-id> <name>` tags:** this contradicts the locked UAT3-02 resolution rule in CONTEXT, which says such a tag is unknown. It would also break the existing `tag_status_resolves_through_the_style_id` contract.
- **Warning in the delete confirm:** this already exists. `delete_confirm_text` reads "N rules use this style. Rules using "<name>" keep their current actions but lose their style link on the next Apply."

The behaviour is the documented consequence of id-based links. Revisit it only if the author wants re-adding a style to relink by name.
**Original issue:** After delete + re-add of a same-named style, the new id does not match `<old-id> <name>` tags, so the rules show unknown and lose their tag on Apply.

---

_Fixed: 2026-10-14_
_Fixer: Claude (gsd-code-fixer)_
_Iteration: 1_

## Author decision (2026-10-15)

Recorded at the v0.5 wrap-up.

- **WR-07** (deleting and re-adding a same-named style drops its tags on Apply): accepted as the documented consequence of id-based links.
- **WR-06 remaining gap** (picking a style while CONFIG failed to load writes a tag with an in-memory-only id): accepted.
- Remaining Info findings in 26-REVIEW.md (IN-01 to IN-06) are accepted as tech debt.
