---
phase: 19-overlay-rule-list-over-parsed-blocks
reviewed: 2026-10-12T00:00:00Z
depth: standard
files_reviewed: 18
files_reviewed_list:
  - src/dynamic.rs
  - src/filter/block.rs
  - src/filter/edit.rs
  - src/filter/mod.rs
  - src/filter/structure.rs
  - src/overlay/app.rs
  - src/overlay/apply.rs
  - src/overlay/economy.rs
  - src/overlay/mod.rs
  - src/overlay/rule_list.rs
  - src/overlay/rules.rs
  - src/overlay/text_entry.rs
  - src/overlay/virtual_list.rs
  - tests/filter_structure.rs
  - tests/overlay_rules.rs
findings:
  critical: 0
  warning: 5
  info: 4
  total: 9
status: issues_found
---

# Phase 19: Code Review Report

**Reviewed:** 2026-10-12
**Depth:** standard
**Files Reviewed:** 18 requested (15 distinct source files; `block.rs`, `edit.rs`, `mod.rs` and `economy.rs` changed only in doc comments or visibility)
**Status:** issues_found

## Summary

The structural edit layer (`filter/structure.rs`) and the grammar gate (`dynamic::blocking_errors`) hold up. I traced append, remove, duplicate and move across economy markers, unterminated final lines and disabled blocks, and found no data-loss path. The line-attribution check in `commit_structural` guards every mutation. Economy markers stop the leading-comment run in `parse::lead_start`, so a marker cannot travel with a moved or deleted block. The index arithmetic in `rules.rs` and `blocking_errors` agrees (1-based, economy blocks skipped, same open interval on the section markers).

No security issues were found. There are no new file-path inputs, and all writes go through the existing guarded writer.

The defects are in the view and controller layer: a name field that disappears while typing, an "expand the added rule" behaviour that cannot work, an Apply that silently reprices a live economy section, and a stale-indicator rule. There is also one dead module and some flaky-test risk.

## Warnings

### WR-01: Clearing a loaded rule name collapses the NAME editor mid-edit

**File:** `src/overlay/rule_list.rs:353-372`
**Issue:** `name_on = row.name.is_some() || list.name_open.contains(&id)`. For a rule whose name was read from the file, `name_open` never contains its id. When the user deletes all the text, the per-keystroke `SetName(id, "")` calls `remove_meta`. `row.name` becomes `None`, so `name_on` turns false and the text field vanishes under the cursor. The empty draft stays in `name_drafts`. The user cannot retype without first clicking the toggle again.
**Fix:** When the field is drawn, register the id so it stays open until the toggle is switched off:
```rust
if name_on {
    list.name_open.insert(id);
    let draft = list.name_drafts.entry(id).or_insert_with(|| row.name.clone().unwrap_or_default());
    ...
}
```

### WR-02: "Add rule while search hides it" does not actually expand or scroll to it

**File:** `src/overlay/app.rs:224-236`, `src/overlay/rule_list.rs:163-166`
**Issue:** `apply_row_actions` sets `list.expanded` and `pending_scroll` to the new rule. On the next frame `show_list` runs `pos_of(expanded)` over the filtered `visible` slice, finds nothing, and resets `list.expanded = None`. `pending_scroll` is also dropped by `take()`. The status line says "Rule added at position N (hidden by the current search)", which is accurate, but the code and its test (`adding_a_rule_the_search_hides_says_so_and_expands_it`) imply the rule is expanded. The test only inspects state before any render, so it cannot see this.
**Fix:** Either stop setting `expanded` and `pending_scroll` in the hidden case, or drop the "and expands it" claim from the test name and comments. The first option is simpler:
```rust
let hidden = is_add && !query.trim().is_empty() && !working.row_matches(focus, query);
if !hidden { list.expanded = Some(focus); list.pending_scroll = Some(focus); }
```

### WR-03: Every Apply with an economy section reprices it, and falls back silently

**File:** `src/overlay/app.rs:139-149`, `src/overlay/rules.rs:379-388`
**Issue:** When the section exists and nothing is staged, `economy_plan()` returns `Refresh`. `apply_working` then always calls `economy_body(..., PriceSourcePolicy::Auto)`.
- With `Auto`, a missing or empty `prices.json` in the app root swaps in the fallback prices. A user whose filter holds live-priced economy blocks, and who only reorders rules, gets the economy section overwritten with early-league fallback values.
- The only signal is the trailing " Used the fallback prices." in the status message.
- If `economy_body` fails outright (for example `styles.json` is missing), a rule-only Apply is refused with an unrelated error.

CONTEXT says the economy is a locked section, and ECON-02 implies rule edits should not touch it. If refreshing on every Apply is intended, the fallback case should be loud.
**Fix:** Make Refresh opt-in (only refresh when `prices.json` is newer than the section, or when the user presses Apply after SYNC). Alternatively, when `Refresh` would use a fallback source, either skip the refresh and use `EconomyOp::Keep`, or require confirmation.

### WR-04: Timing-based and count-pinned assertions are flaky or brittle

**File:** `tests/overlay_rules.rs:43-56, 82-83, 18-22`
**Issue:**
- The test asserts wall-clock bounds: load under 2 s, search under 50 ms, and an average edit under 60 ms. On a loaded CI runner or under a debug build these fail with no code change. Each edit re-parses the document, rebuilds all 511 rows and calls `serialize()` for the dirty flag.
- The same file hard-codes 511 / 460 / 51 against a fixture, so any fixture refresh breaks it.

**Fix:** Keep the correctness checks (revert restores bytes, search results). Make the timing checks generous, or mark them `#[ignore]` / gate them on an env var. Derive the enabled/disabled counts from the fixture instead of pinning them.

### WR-05: `text_entry.rs` is a dead module shipped as public API

**File:** `src/overlay/text_entry.rs:1-93`, `src/overlay/mod.rs:16`
**Issue:** `parse_base_type_entry` is not referenced anywhere. A repo-wide grep finds only the `pub mod` line. The base-type entry UI was removed with `rule_editor.rs`, and the module comment says it will be "reused by Phase 20". Until then it is unused public surface that must be kept compiling.
**Fix:** Remove it and re-add it in Phase 20 from history, or mark it `#[allow(dead_code)]` with a Phase 20 TODO reference. Do not export a speculative module.

## Info

### IN-01: Indicator goes "out of date" for edits that leave the document unchanged

**File:** `src/overlay/app.rs:895-899`, `src/overlay/rules.rs:410-506`
**Issue:** Any `ActionOutcome.changed` sets `ApplyState::Unapplied`. Several actions report `changed` without altering bytes. `ClearName` on a rule with no name is the clearest case, because `remove_meta` returns `Ok(())`. Moving a rule and moving it back is another. The stripe then says "Filter out of date" while `is_dirty()` is false and REVERT is disabled.
**Fix:** After applying actions, derive the state from the document. If `!working.is_dirty()` and an apply has happened, use `Applied`. Otherwise use `Unapplied`.

### IN-02: Dead branch in `flags_for`

**File:** `src/overlay/rule_list.rs:94-101, 130-136`
**Issue:** `cannot_enable` is only true for disabled rows, and the first `if` already returns for `!row.enabled && row.cannot_enable`. The later `if row.cannot_enable` push is unreachable.
**Fix:** Delete the second block.

### IN-03: Search haystack includes the row index and the Show/Hide keyword

**File:** `src/overlay/rules.rs:292-297`
**Issue:** The label is part of the haystack, as the spec requires. The label starts with `"{index}  Show"`, so searching "show" or "hide" matches nearly every row, and "1" matches rules 1, 10-19, 100-199 and so on. This is noise rather than a bug, but it makes the "N of M rules" count misleading for short queries.
**Fix:** Build the haystack from the derived pieces (class / base-type summary) rather than the full label, or strip the index and keyword prefix.

### IN-04: `moved_across_continue` is stale-able

**File:** `src/overlay/rules.rs:459-469`
**Issue:** The `moved` set is keyed by id and cleared only when that same rule is moved again without crossing a Continue. If the user afterwards removes the Continue line (`SetContinue(false)`), the "moved across Continue" warning stays on the row until Apply or Revert.
**Fix:** Recompute or prune `moved` in `rebuild()`. Keep an id only if that row or an adjacent row still has Continue, or clear it on any `SetContinue`.

---

_Reviewed: 2026-10-12_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
