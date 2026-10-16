---
phase: 32-rule-dividers-economy-banner
reviewed: 2026-10-16T00:00:00Z
depth: standard
files_reviewed: 24
files_reviewed_list:
  - src/cli.rs
  - src/dynamic.rs
  - src/filter/divider.rs
  - src/filter/edit.rs
  - src/filter/mod.rs
  - src/filter/parse.rs
  - src/filter/section.rs
  - src/filter/structure.rs
  - src/overlay/app.rs
  - src/overlay/apply_all.rs
  - src/overlay/body.rs
  - src/overlay/rule_list.rs
  - src/overlay/rules.rs
  - src/overlay/screenshot.rs
  - src/overlay/virtual_list.rs
  - src/overlay/widgets.rs
  - src/settings.rs
  - src/skin/icons.rs
  - src/styled.rs
  - tests/dynamic_filter.rs
  - tests/filter_roundtrip.rs
  - tests/filter_structure.rs
findings:
  critical: 2
  warning: 4
  info: 0
  total: 6
status: issues_found
---

# Phase 32: Code Review Report

**Reviewed:** 2026-10-16
**Depth:** standard
**Files Reviewed:** 24 (22 distinct paths listed; two of the 24 requested were unchanged or test-only)
**Status:** issues_found

## Summary

I checked these invariants by reading the code and tracing the call paths. I did not run cargo.

- Banner parsing, `lead_start` fencing, `refresh`/`add`/`remove`, `insert_banner_at`, CRLF and unterminated-last-line handling all look sound.
- `+ Add rule` lands above every divider, and `top_insertion_line` is correct.
- A divider never splits a `# ff:group` run, because group detection requires blank-only gaps and the hop logic moves whole groups.
- Collapsed state is stored only in `ff-settings.json` and never touches the filter or its dirty flag.

The invariant "nothing user-editable can move into or below the economy section" is broken for the divider operations. A user section that follows the economy section (a legal layout, since the economy section runs to the next banner) can lose its rules to the economy section, and Apply then deletes them.

## Critical Issues

### CR-01: Removing the divider directly below the economy section silently turns its rules into economy rules (data loss on Apply)

**File:** `src/filter/divider.rs:136-153` (`remove_section`); UI entry `src/overlay/rules.rs:1421-1435`, `src/overlay/rule_list.rs:662`
**Issue:** `remove_section` only rejects the economy banner itself (`user_banner`). The economy section runs from its banner to the next banner. If a user section sits right after it (`economy banner, economy blocks, # ff:section After, user rules`), removing `After` deletes the banner that ends the economy section. The user rules then fall inside the economy span. They become read-only economy rows, and the next Apply regenerates that region from `prices.json` and discards them. The status text claims "Its rules joined the section above", which is not what happens. The x button is always shown for non-economy headers, and no test covers this layout. `tests/filter_structure.rs:420-445` only tests a user section before the economy section.
**Fix:** In `remove_section`, refuse when the previous banner is the economy banner, and hide or disable the x in `draw_section_header`. Also set `SectionRow.can_remove`.
```rust
pub fn remove_section(&mut self, ordinal: usize) -> Result<(), EditError> {
    let start = self.user_banner(ordinal)?;
    let b = section::banners(self);
    if ordinal > 0 && b[ordinal - 1].kind == BannerKind::Economy {
        return Err(EditError::SectionLocked); // or a dedicated error
    }
    ...
```
Alternative: allow it, but only after the rules are moved out first (not recommended).

### CR-02: Moving the divider below the economy section down puts user rules inside the economy section

**File:** `src/filter/divider.rs:184-233` (`hop`), `:243-276` (`move_section`); `src/overlay/rules.rs:767-787` (`SectionRow.can_move_down`)
**Issue:** For `up == false`, `hop` only rejects a nearest rule that is itself in the economy section. For a banner sitting right below the economy section, the nearest rule after it is a user rule (after the banner), so the hop is allowed. `move_section` then swaps the banner below that rule, which leaves the rule between the economy banner body and the divider, so it is now in the economy section. `section_can_move(k, false)` returns true, so the down arrow is enabled. The result is the same as CR-01: the rule becomes locked and is deleted by the next Apply. `rule_step`/the layout pass block the mirror case for rules (`below_economy`), but dividers have no equivalent guard.
**Fix:** In `hop`, return `Ok(None)` when `!up` and the banner at `ordinal - 1` is the economy banner.
```rust
if !up && ordinal > 0 && banners[ordinal - 1].kind == BannerKind::Economy {
    return Ok(None);
}
```
Add a test with the layout `economy banner, blocks, user divider, rule` for `section_can_move(k, false)`, `move_section` and `remove_section`.

## Warnings

### WR-01: Section actions batched in one frame address banners by ordinal, so a pending rename can hit the wrong section

**File:** `src/overlay/rule_list.rs:742-760`, `src/overlay/app.rs:2022-2030` and `:1390-1400`, `src/overlay/rules.rs:1421-1434`
**Issue:** The rename commits on `lost_focus`, which also fires on the same frame as any click elsewhere. All actions of one frame go into one `Vec` and run in order. The toolbar actions (`AddSection`) are pushed before the list's actions. Rows drawn above the header are also pushed first (`AddSectionAbove`), as are `RemoveSection` clicks on earlier headers. So `[AddSection, RenameSection{ordinal: 1, ...}]` applies the add first. That shifts every ordinal, and the typed name lands on the new section (or on the wrong one), while the edited section keeps its old name. `carry_section_state` also computes the old name from the pre-action ordinals, so the saved collapsed state is copied from the wrong section. If the ordinal now points at the economy banner, the rename is refused with the economy-locked message.
**Fix:** Apply the pending rename before any structural action in the same batch. For example, sort `RenameSection` to the front in `run_row_actions`/`coalesce_edits`, or commit the rename inside `draw_section_header` when any structural click happens. Alternatively, key renames by the banner's line text or position, not by ordinal.

### WR-02: A rule with a blocking grammar error inside a collapsed section cannot be reached from the status line

**File:** `src/overlay/app.rs:2720-2727` (`jump_to`), `src/overlay/rule_list.rs:489-494`, `src/overlay/rules.rs` (`entries`)
**Issue:** The status line's "click to jump" sets `list.expanded` and `pending_scroll` to the offending rule. If the rule lives in a collapsed section it is not in `entries`, so `show_list` drops the expansion and the scroll target without showing anything. Apply is blocked by that error, and the collapsed header gives no hint (no error badge). The same happens for a rule moved with the arrows into a neighbouring collapsed section: it vanishes. The jump should open the section.
**Fix:** When handling `jump_to` (and a `Move` result), un-collapse the rule's section: look up `row.section`, find its `SectionRow`, and call `set_section_collapsed(name, false, false)`. Alternatively show an error or unknown-style marker on collapsed headers.

### WR-03: Toggling a section silently overwrites an unusable `ff-settings.json`

**File:** `src/overlay/app.rs:1177-1204` (`store_settings`, `set_section_collapsed`), `src/settings.rs:121-195` (`load`)
**Issue:** When `ff-settings.json` exists but is invalid JSON (for example a trailing comma), `load` returns defaults with `reload_after_apply = off` and a warning, and the file is left alone. Before this phase the file was only rewritten by an explicit Config change. A fold or unfold click is a casual action, but it now calls `settings::save` with those fallback defaults. That discards the user's hand-edited file, including all unknown keys, and persists `reload_after_apply: false` and `filter: dynamic.filter`. This violates the stated "preserves unknown ff-settings keys" invariant for the unreadable-file case.
**Fix:** Keep a `settings_unusable` flag from `load` (the warning string is already available). In `set_section_collapsed`, update only the in-memory value and skip the write when the file was unusable, or write only when the file is missing or valid. Show the existing warning.

### WR-04: Collapsed state is keyed by section name only, so it leaks between sections

**File:** `src/settings.rs:40-80`, `src/overlay/app.rs:1189-1230`
**Issue:** Four problems follow from keying on the name:
1. `copy_section_state` keeps the `from` entry after a rename and copies even when the rename is later refused. Stale `true` flags accumulate in `ff-settings.json`.
2. A new section named "Section" (the default name) or any reused name inherits an old collapsed flag, so the spec rule "new sections start expanded" is violated. A section whose rules just moved in can start hidden.
3. Duplicate-named sections (allowed) share one flag, so toggling one folds the other.
4. A user section named exactly `ff:economy` shares the key with the economy section (`ECONOMY_SECTION_KEY`), so it toggles the economy state. `sanitize_name` does not prevent this.
**Fix:** On `RemoveSection`/rename, delete the stale entry (`remove` the old key). On `AddSection`/`AddSectionAbove`, clear the stored flag for the new name. Reject or escape the reserved `ff:economy` key, for example by prefixing user keys (`user:<name>`).

## Info

None.

---

_Reviewed: 2026-10-16_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
