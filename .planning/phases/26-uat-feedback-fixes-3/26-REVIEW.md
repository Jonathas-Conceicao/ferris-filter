---
phase: 26-uat-feedback-fixes-3
reviewed: 2026-10-14T00:00:00Z
depth: standard
files_reviewed: 18
files_reviewed_list:
  - src/dynamic.rs
  - src/emitter.rs
  - src/filter/keywords.rs
  - src/loader.rs
  - src/models/config.rs
  - src/overlay/app.rs
  - src/overlay/apply.rs
  - src/overlay/apply_all.rs
  - src/overlay/body.rs
  - src/overlay/conditions.rs
  - src/overlay/config_state.rs
  - src/overlay/config_view.rs
  - src/overlay/grammar.rs
  - src/overlay/mod.rs
  - src/overlay/rule_list.rs
  - src/overlay/rules.rs
  - src/styled.rs
  - styles.json
findings:
  critical: 2
  warning: 8
  info: 6
  total: 16
status: issues_found
---

# Phase 26: Code Review Report

**Reviewed:** 2026-10-14
**Depth:** standard (diff `28a7677..HEAD`)
**Files Reviewed:** 18
**Status:** issues_found

## Narrative Findings (AI reviewer)

## Summary

I reviewed the Phase 26 diff: style ids, `<id> <name>` tags, rename, the in-memory style picker, Defences and Between cards, the collapsed picker, the class-group fix, and the Quit confirm. The id model itself is sound. Validation, deterministic load-time ids, and persisting `ids_unpersisted` silently through `prepare_write_inner` all work. `quit_step` is a clean pure function, and the hotkey -> tray -> instance -> Close order is kept in `begin_shutdown`.

The main problems are in two places:

1. **Revert paths around rename.** The in-memory legacy-tag migration (`retag_renamed_legacy`) is a one-shot mutation of the RULES document. A RULES Revert undoes it while the CONFIG rename stays, so the tag is orphaned again and dropped on Apply.
2. **Between with strict operators.** Two code paths can still write a pair that matches nothing, without any warning. This breaks the locked rule "never write an impossible pair silently".

The rest are robustness and UX gaps: picker order, a hidden modal, stale drafts, and a narrow shutdown race.

## Critical Issues

### CR-01: RULES Revert after a CONFIG rename re-orphans legacy tags, and Apply then drops them (or relinks them to a different style)

**File:** `src/overlay/rules.rs:510-551`, `src/overlay/rules.rs:1109-1114`, `src/overlay/app.rs:198-206`, `src/overlay/app.rs:1167-1177`
**Issue:** `retag_renamed_legacy` runs only inside `set_styles`. It rewrites a legacy `# ff:style kite` tag to `t5hw2k locator` at the moment the RULES tab first receives the renamed set. That rewrite lives in the editable document, so it is an unapplied RULES edit. Then:
1. The user renames `kite` -> `locator` on CONFIG. `sync_working_styles` calls `set_styles` and the legacy tags are migrated in memory. RULES becomes dirty.
2. The user presses the RULES **Revert**. `app::revert` keeps CONFIG edits ("CONFIG edits survive a RULES revert"), and `WorkingFilter::revert` re-parses `applied_text`, which brings back `# ff:style kite`.
3. `working.styles` already equals the pending set, so `sync_working_styles` is a no-op (`style_set() == Some(pending)`) and `set_styles` never runs again. Nothing re-migrates the tag.
4. The row shows `Unknown("kite")`. On Apply, `text_with_style_set` -> `drop_unknown_style_tags` deletes the tag, and the rule loses its style link permanently. This is exactly the "rename orphans rules" outcome UAT3-02 forbids.

It gets worse if the user then adds a new custom style named `kite`. The legacy tag resolves to it by name through the `resolve_tag` fallback, and Apply silently rewrites the rule's actions with the new style's actions.

**Fix:** Make the migration a function of the document plus the applied baseline set, not a one-shot side effect of `set_styles`. The smallest fix: keep the set the document was last applied/loaded against, and re-run the migration whenever the document is re-parsed:
```rust
// WorkingFilter
baseline_styles: Option<StyleSet>, // set in from_text/set_styles(disk)/on_applied

pub fn revert(&mut self) {
    self.doc = parse_text(&self.applied_text);
    self.pending = None;
    self.moved.clear();
    if let (Some(old), Some(new)) = (self.baseline_styles.clone(), self.styles.clone()) {
        self.retag_between(&old, &new); // same body as retag_renamed_legacy, explicit old set
    }
    self.rebuild();
}
```
A cheaper alternative: in `app::revert`, call `working.set_styles(load_styles_at(root))` (the disk set) before the next frame's `sync_working_styles`. The sync then passes through `set_styles(pending)` with old = disk set and migrates again. Add a regression test: legacy tag -> rename -> sync -> RULES revert -> Apply keeps the tag as `t5hw2k locator`.

### CR-02: Between can write an impossible pair silently when a bound is strict (`>` / `<`)

**File:** `src/overlay/conditions.rs:221-248`, `src/overlay/conditions.rs:250-253`, `src/overlay/body.rs:523-552` (diff lines; `clamped_bound` / `between_sliders` / `bound_control`)
**Issue:** CONTEXT UAT3-06 says "never write an impossible pair silently". Every check treats both bounds as inclusive:
- `between_switch` for a lower bound inserts `<= max.max(value)`. For `Quality > 30` (slider max 30) it inserts `Quality <= 30`, so the pair `> 30` / `<= 30` matches nothing. For an upper bound, `ItemLevel < 1` gets `>= 1`, which is also empty. The 26-02 summary claims the clamp keeps the pair satisfiable, but that only holds for `>=` / `<=`.
- `clamped_bound` clamps the new min to `<= hi` and the new max to `>= lo`. With a loaded `ItemLevel > 50` / `ItemLevel <= 60` pair (the exact `>`/`<=` case the spec requires to load as one card), dragging max down to 50 writes `<= 50`. The rule now matches nothing.
- `between_ok(lo, hi)` is `lo <= hi`, so neither case shows the `min > max` hint.

A Show rule that silently matches nothing lets valuable drops fall through to a later Hide rule.

**Fix:** Make the satisfiability check strictness-aware and use it for both the clamp and the hint:
```rust
/// Effective inclusive bounds for integer lines.
fn inclusive_lo(op: Option<Operator>, v: i64) -> i64 { if op == Some(Operator::Gt) { v + 1 } else { v } }
fn inclusive_hi(op: Option<Operator>, v: i64) -> i64 { if op == Some(Operator::Lt) { v - 1 } else { v } }
pub fn between_ok(lo_op: Option<Operator>, lo: i64, hi_op: Option<Operator>, hi: i64) -> bool {
    inclusive_lo(lo_op, lo) <= inclusive_hi(hi_op, hi)
}
```
In `between_switch`, insert `max.max(value + 1)` after `>` and `min.min(value - 1)` after `<`. In `bound_control`, clamp against the strict-adjusted other bound (`hi - 1` when either side is strict, and so on).

## Warnings

### WR-01: CONFIG reorder never reaches the RULES style picker before Apply

**File:** `src/overlay/app.rs:198-206`, `src/models/config.rs:340-343`
**Issue:** `sync_working_styles` skips the update when `working.style_set() == Some(config.pending_styles())`. `StyleSet` derives `PartialEq` over an `IndexMap`, and `IndexMap` equality ignores order (the old `ConfigState::reordered` comment said so explicitly). A pure `MoveStyle` on CONFIG therefore never reaches `WorkingFilter`, and the RULES style dropdown keeps the old order until an Apply calls `set_styles` directly. UAT3-03 requires "in display order".
**Fix:** Compare order as well:
```rust
let same = working.style_set().is_some_and(|w| {
    w == config.pending_styles() && w.names().eq(config.pending_styles().names())
});
if same { return false; }
```

### WR-02: A second tray Quit with the confirm open does not reveal a hidden stripe

**File:** `src/overlay/app.rs:1714-1730`
**Issue:** `OpenConfirm` sets `toggle_requested` when hidden, but `KeepConfirm` does nothing. If the user hides the stripe with `Ctrl+Shift+Z` while the modal is up, `quit_confirm` stays true and an invisible modal is waiting. Every later tray Quit hits `KeepConfirm`, so Quit looks dead until the user re-shows the stripe by other means.
**Fix:** Treat `KeepConfirm` like `OpenConfirm` for visibility:
```rust
apply::QuitStep::OpenConfirm | apply::QuitStep::KeepConfirm => {
    self.quit_confirm = true;
    if self.visibility == Visibility::Hidden { self.toggle_requested = true; }
}
```

### WR-03: Between is offered on a line whose keyword already has other lines, producing a state the UI cannot show or undo

**File:** `src/overlay/body.rs` (`op_controls`, diff lines 449-465), `src/overlay/conditions.rs:68-84`
**Issue:** `op_controls` offers Between for any single-integer `Int` line. `card_plan` only groups exactly two lines (one lower, one upper). Take a rule with `ItemLevel >= 60` and `ItemLevel != 70`: picking Between on the first line inserts `ItemLevel <= 100`. Now there are three ItemLevel lines and they render as three single cards. The user picked "Between" and gets no Between card, the dropdown shows `<=` on the new card, and there is no single action that switches back.
**Fix:** Only offer `NumericOp::Between` when the keyword has exactly one condition line in the rule. Pass a `can_between: bool` computed from `lines` into `op_controls` / `numeric_op_choices`.

### WR-04: CONFIG Revert after a rename leaves a phantom RULES edit, which triggers the quit prompt and an "unapplied" state

**File:** `src/overlay/rules.rs:530-551`, `src/overlay/app.rs` (ConfigAction::Revert arm)
**Issue:** After a rename has migrated legacy tags in memory (see CR-01), a CONFIG Revert restores `kite`. The document still holds `t5hw2k locator`, which resolves by id, so RULES stays dirty even though the user changed nothing on RULES. The Apply button stays lit, the RULES tab shows unapplied changes, and tray Quit opens the confirm.
**Fix:** Treat the migration as part of the baseline, not as a user edit. Either apply the migration to `applied_text` as well (it is a no-dirty migration by design, like `style_rewrite_pending`'s `retag = false`), or exclude pure tag differences from `dirty`.

### WR-05: Rename drafts outlive Revert, reload and Apply

**File:** `src/overlay/config_view.rs:686-731`, `src/overlay/app.rs:1330-1352`
**Issue:** `rename_drafts` (keyed by style id) is cleared only after a successful `RenameStyle`. An invalid draft (red outline) left after focus-out survives a CONFIG Revert, RELOAD and Apply. The text field keeps showing the stale text instead of the real name. If the conflict later disappears (the clashing style is deleted), the stale draft becomes valid but only commits on another focus-out. A user can easily end up with a mismatch between what the field shows and what is applied.
**Fix:** Clear `config_ui.rename_drafts` wherever `style_draft` / `disarm_all` is reset (CONFIG Revert, Reload, successful Apply, `follow_disk` changes).

### WR-06: Filters get load-assigned ids that styles.json may never receive

**File:** `src/overlay/app.rs` (`apply_everything`, `None` config branch -> `working.text_with_styles()`), `src/overlay/rules.rs:576-605`
**Issue:** When `ConfigState` failed to load (for example a bad `tiers.json`) but `styles.json` is readable, Apply still goes through `text_with_styles` -> `text_with_style_set` with `retag = true`. Legacy tags become `<hash-id> <name>` using ids that exist only in memory, and styles.json is not written because there is no config part. The ids come from `fnv1a(name)`, so they look stable, but only while the name and set composition stay identical. If the user later renames that custom style by hand in styles.json, the next load hashes the new name to a different id. Every tag carrying the old id becomes unknown and is dropped on the next Apply.
**Fix:** Do not retag when the ids are not persisted. Pass `retag = false` in the `None`-config branch, or keep the legacy name tag whenever the resolved style's id was assigned at load and not yet written.

### WR-07: Deleting and re-adding a same-named style now silently drops all its tags on Apply

**File:** `src/models/config.rs:582-590` (`insert_new` always mints a fresh id), `src/styled.rs:194-215`
**Issue:** Before this phase, `# ff:style kite` tags followed the name, so delete + re-add `kite` (for example to reset its look) kept rules linked. Now every tag written by this version is `t5hw2k kite`. After delete + re-add, `kite` has a new id, `resolve_tag` finds no id match, and the name fallback only applies when the whole tag equals a name, which `t5hw2k kite` does not. Every such rule shows unknown and loses its tag on Apply. This is a behaviour regression, and the delete confirm does not mention it.
**Fix:** Either fall back to the name part of a canonical tag when the id is unknown (`<unknown-id> <name>` -> resolve by `<name>`, then retag), or have the delete confirm state that N rules will lose their style on Apply.

### WR-08: The Quit paths in the modal shut down after this frame's instance show-request poll

**File:** `src/overlay/app.rs:2226-2276` vs `src/overlay/app.rs:1712-1745`
**Issue:** The tray `QuitNow` path, and the `quit_after_apply` path, call `begin_shutdown` before the INST-01 show-request poll. The comment there explains why: a relaunch must become the new first instance rather than be acked by a process that is about to close. The modal's `QuitNow` / Apply & quit choices run `begin_shutdown` at the end of `update`, after that poll. A second launch whose show request is handled in the same frame gets acked and exits, and this process then closes, leaving no overlay. The window is one frame, but CONTEXT asks for the Phase 22 ordering "exactly".
**Fix:** Do not shut down from the modal directly. Record the decision (`self.quit_now = true`) and act on it at the top of the next frame's tray-poll block, next to the `quit_after_apply` check, so it runs before the show-request poll as the other paths do.

## Info

### IN-01: Class-group open state now survives keyword-ordinal shifts

**File:** `src/overlay/body.rs` (`class_picker`, diff lines 948-982)
**Issue:** `open_id` is `("ff-class-group-open", row_epoch, salt, label)`, where `salt` includes the card's keyword ordinal, and `row_epoch` is no longer bumped on AddLine/RemoveLine. If a rule has two Class cards and the first is removed, the second (ordinal 1 -> 0) inherits the removed card's open groups. This is the stale-state reuse WR-07 (Phase 2x) used the epoch to prevent. The temp entries are also never removed.
**Fix:** Acceptable for UX, but consider keying on the line's text hash or clearing open state for the removed ordinal.

### IN-02: Searching the picker for a defence keyword name finds nothing

**File:** `src/overlay/grammar.rs` (`ConditionEntry::search_text`)
**Issue:** The Defences search text is `"defences armour evasion energy shield defence"`. A user typing `BaseArmour` or `base` (the names shown in filter files and on card headers) gets "Nothing matches".
**Fix:** Append `"basearmour baseevasion baseenergyshield base"` to the search text.

### IN-03: The id-first lookup in `resolve_tag` can steal a legacy name tag

**File:** `src/models/config.rs:360-376`
**Issue:** A legacy tag is resolved by id first, using its first whitespace token. Custom style names are not restricted from looking like ids (or from starting with `builtin:`). A legacy tag `# ff:style q8m3vz extra`, for a style actually named `q8m3vz extra`, resolves to `Recycle`. This is unlikely, but nothing prevents it.
**Fix:** In `check_new_name`, reject names whose first token equals an existing id or starts with `builtin:`. Alternatively, try the whole-name match first when the tag is not in canonical form.

### IN-04: A loader test uses a fixed, shared temp path

**File:** `src/loader.rs:125-141`
**Issue:** `std::env::temp_dir().join("ff-26-01-load-ids")` is shared between concurrent `cargo test` runs (worktree agents run in parallel in this repo). It also runs `remove_dir_all` on that path, so one run can delete another run's file.
**Fix:** Use `tempfile::tempdir()`, as the other tests do.

### IN-05: `assign_missing_ids_with` duplicates `assign_missing_ids`

**File:** `src/models/config.rs:441-478`
**Issue:** The two loops are identical except for the generator, so the test hook can drift from the production path.
**Fix:** Have `assign_missing_ids` call a private `assign_with(&mut self, gen: impl FnMut(&str) -> ...)`, and have the test wrapper inject its feed.

### IN-06: Malformed defence lines do not count as "present"

**File:** `src/overlay/conditions.rs:138-163`, `src/overlay/body.rs` (`defence_lines` filter)
**Issue:** An opaque (unparseable) `BaseArmour ...` line is neither in `present_entries` nor in `defence_lines`. The picker therefore offers Defences as absent, and the Armour toggle shows as off. Turning it on adds a second `BaseArmour` line next to the broken one.
**Fix:** Count opaque lines whose first word is a defence keyword as present, and show the toggle as on and read-only.

---

_Reviewed: 2026-10-14_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
