---
phase: 20-full-grammar-block-editing
reviewed: 2026-10-13T04:24:33Z
depth: standard
files_reviewed: 19
files_reviewed_list:
  - src/emitter.rs
  - src/filter/edit.rs
  - src/filter/lineedit.rs
  - src/filter/mod.rs
  - src/filter/structure.rs
  - src/lib.rs
  - src/overlay/app.rs
  - src/overlay/apply.rs
  - src/overlay/body.rs
  - src/overlay/classes.rs
  - src/overlay/grammar.rs
  - src/overlay/mod.rs
  - src/overlay/rule_list.rs
  - src/overlay/rules.rs
  - src/overlay/theme.rs
  - src/overlay/widgets.rs
  - src/styled.rs
  - tests/filter_roundtrip.rs
  - tests/overlay_rules.rs
findings:
  critical: 1
  warning: 7
  info: 4
  total: 12
status: issues_found
---

# Phase 20: Code Review Report

**Reviewed:** 2026-10-13T04:24:33Z
**Depth:** standard
**Files Reviewed:** 19
**Status:** issues_found

## Summary

I reviewed the diff against `bb253b4` for the line-patch API (`lineedit.rs`, `commit_owned`), the styled-rule rewrite (`styled.rs`, `style_action_lines`), the working-model line actions and style state (`rules.rs`), the codec (`grammar.rs`), the body editor (`body.rs`), widgets, and the Apply and style-poll wiring in `app.rs`. The full `cargo test` suite passes.

Most of the invariants you asked about hold:
- **Edits stay on one line, operators are kept, EOLs survive.** Edits replace only the tokenizer's operator and value ranges, candidates keep each line's `Eol`, and operators are never normalized when the user did not pick one (the sweeps confirm this).
- **No empty keyword lines.** `values_edit` turns an empty value into `RemoveLine`, and `check_text` refuses empty text.
- **Style rewrite.** It runs on a clone, skips disabled and economy blocks, compares lines semantically so it is idempotent, and leaves unknown styles untouched.
- **Style locks and class groups.** Styled rules are locked in the model and in the view. Class groups come from `config.json`.
- **Tests.** The tests use temp dirs and never write to the game folder.

There is one correctness defect: editing a negated rarity condition can silently flip its meaning. The remaining warnings cover the style lock being too wide in one place and too narrow in another, how style-rewrite failures are reported, colour round-trip loss, and state leaking between rules.

## Narrative Findings (AI reviewer)

## Critical Issues

### CR-01: Adding a rarity chip to a negated `Rarity` line drops the `!=` and flips the condition

**File:** `src/overlay/body.rs:106-122`, `src/overlay/grammar.rs:279`
**Issue:** `rarity_edit` treats every operator other than none, `=` and `==` as an "ordering" operator:
```rust
let ordering = !matches!(entry.op, None | Some(Operator::Eq) | Some(Operator::EqEq));
let patch = if selected.len() >= 2 && ordering { LinePatch::Both(None, text) } ...
```
`!=` and `!` are not ordering operators. `Rarity != Normal Magic` is valid grammar (the validator accepts any operator with several rarities) and means "neither Normal nor Magic". Suppose a user has `Rarity != Normal` and clicks the Magic chip. The line is rewritten as `Rarity Normal Magic`, which matches exactly the items it used to exclude. That breaks PARSE-04 ("an existing operator is never normalized unless the user changes it") and silently inverts what the filter shows.

The list-form operator dropdown makes this worse. `operator_choices` returns `vec![oc(current, "Matches")]` for any multi-value rarity line. A file line `Rarity != Normal Magic` is therefore labelled **"Matches"**, the opposite of what it does, and the user cannot change the operator from there.

**Fix:**
```rust
// body.rs
let ordering = matches!(
    entry.op,
    Some(Operator::Lt | Operator::Le | Operator::Gt | Operator::Ge)
);
// grammar.rs (list form)
ValueShape::Rarity if list_form => match current {
    Some(Operator::Ne) => vec![oc(current, "Not any of")],
    Some(Operator::Bang) => vec![oc(current, "Not any of (!)")],
    _ => vec![oc(current, "Matches")],
},
```
Add a unit test: `Rarity != Normal` plus Magic must give `Rarity != Normal Magic`.

## Warnings

### WR-01: The `Continue` checkbox is locked on styled rules even though styles never own `Continue`

**File:** `src/overlay/body.rs:1518-1528`
**Issue:** `disposition_line` disables the Continue checkbox whenever the style is `Known`. However:
- `style_action_lines` never emits `Continue`.
- `rewrite_block` leaves it in place (`Kind::Modifier`, as the `rewrites_actions_and_keyword_keeping_conditions` test shows).
- The model does not lock `SetContinue`.

The user has to remove the style just to toggle a setting the style does not control. This goes beyond the CONTEXT rule that only "style-owned actions show as coming from the style".
**Fix:** Keep `locked` for the Show/Hide/Minimal buttons only, and build the checkbox with `ui.add(cb)` (always enabled).

### WR-02: One styled rule that cannot be rewritten blocks Apply for the whole filter, and the error does not say which rule

**File:** `src/styled.rs:125-130`, `src/overlay/rules.rs` (`text_with_styles`)
**Issue:** `rewrite_styled` uses `?` on every `rewrite_block`. The first `EditError` stops the pass, for example:
- `CompactHeader` from `set_block_keyword` on a header such as `Hide Continue`;
- a refused splice.

`text_with_styles` then turns this into `Could not update styled rules: <error>`. Apply refuses everything, and the message names neither the rule nor its index, so the user cannot find the culprit. In that state `style_rewrite_pending` returns `false`, so nothing else hints at the problem.
**Fix:** Record per-rule failures in `StyleReport`, for example `failed: Vec<(usize, String)>` filled using `rule_index_of`. Skip those rules (they keep their current actions, like unknown styles) and report `rule N` in the Apply message and the footer's jump line. Or, at minimum, prefix the error with `rule {index}: `.

### WR-03: The model style lock can be bypassed, and the bypassing edit is then silently deleted on Apply

**File:** `src/overlay/rules.rs` (`RowAction::AddLine`, `RowAction::EditLine` arms; `action_line_lock`)
**Issue:** The lock looks only at the target line's current kind and at the caller-declared `LineKind`:
- `AddLine { kind: LineKind::Condition, text: "SetFontSize 40" }` is accepted on a `Known`-styled rule. The text is never checked against the declared kind.
- On the same rule, a `LinePatch::Whole` raw edit of an opaque or unknown-keyword line, which `raw_row` offers in CONDITIONS, can turn it into an action line, for example by fixing the typo `SetFontSiz 40`.

Either way the edit lands. On the next Apply, `rewrite_block` removes every action line that the style does not emit, so the user's visible edit disappears without any message.
**Fix:**
- In `AddLine`, tokenize `text` and refuse when the keyword's `Kind` does not match `kind`.
- In `EditLine` with `LinePatch::Whole` on a `Known`-styled rule, tokenize the new text and return `style_locked_status` if it would become a `Kind::Action` line.

### WR-04: Editing a colour with alpha 0 loses its RGB

**File:** `src/overlay/widgets.rs:431-457`
**Issue:** `colour_swatch` builds a premultiplied `Color32::from_rgba_unmultiplied(r, g, b, a)`. With `a == 0` premultiplication gives `(0, 0, 0, 0)`, so the original RGB is gone before the picker opens. A common pattern is the transparent background `SetBackgroundColor 40 42 54 0`. Raising its alpha in the picker writes `0 0 0 A` instead of `40 42 54 A`. Very low alpha values also drift in RGB through the round trip. The idle-frame guard only stops spurious writes; it does not prevent this loss.
**Fix:** Edit an unmultiplied colour with `egui::color_picker::color_edit_button_rgba_unmultiplied` (or keep `[f32; 4]` unmultiplied state in temp memory) and convert to `u8` only on commit.

### WR-05: A mod condition added from the picker gets an exact count of 1, which changes its meaning once more names are added

**File:** `src/overlay/body.rs:519-545` (`pending_first_entry`), `src/overlay/body.rs:1200-1210`
**Issue:** For `ValueShape::CountThenNames`, the first entry writes `count: Some(1)` with no operator, for example `HasExplicitMod 1 "Tyrannical"`. A bare count is an equality test. Once the user adds a second name, the line means "exactly one of these", so an item carrying both listed mods no longer matches. The card shows `Count 1 of these:` rather than `At least`. This contradicts the natural "any of these" intent of the picker, and the existing "Any of these:" (no count) form is never produced from the picker.
**Fix:** Write the pending line without a count (`count: None`, matching the existing "Any of these:" path), or with `>=` (`PendingRow { op: Some(Operator::Ge), .. }` for CountThenNames) so the line reads `HasExplicitMod >= 1 "..."`.

### WR-06: A rejected first entry on a pending row is dropped without any message, and `chip_list` does not reject quotes despite its message

**File:** `src/overlay/body.rs:519-545, 586-595`, `src/overlay/widgets.rs:563-575`
**Issue:**
- `pending_first_entry` does `grammar::encode(&value).ok()?`. When an entry contains a `"`, for example a paste like `Foo"Bar` that `parse_base_type_entry` does not split cleanly, encoding fails. `pending_card` then does nothing: the user's text is cleared from the add field, no line is written and no hint appears.
- `chip_list` only refuses control characters, yet `CHIP_REJECT_MESSAGE` says "Values cannot contain quotes or control characters".

Non-pending list cards get the hint through `Card::commit`, so the two paths behave differently.
**Fix:** Do the quote check in `chip_list` too: `entry.chars().any(|c| c == '"' || c.is_control())`, so `rejected` is set and the draft is not lost. Also make `pending_first_entry` return `Result` and show `BAD_TEXT_HINT` on the pending card.

### WR-07: Per-card UI state is keyed by reusable `BlockId`/ordinal and leaks onto other rules

**File:** `src/overlay/body.rs:640-646, 1300-1310`; `src/overlay/app.rs` (`run_apply`, `revert`)
**Issue:** Each card stores state in egui temp memory, persistent across frames:
- the `bad-text` flag;
- the `bad-int` flag;
- the `named` toggle of `bool_or_name_control`;
- raw drafts.

The state lives under `push_id((block_id, kw.name, ordinal))`. Apply and Revert re-parse the document, which renumbers `BlockId`s from scratch, and `ListUi::clear_ids` does not clear egui memory. So a stale "Values cannot contain quotes or control characters" hint, or a forced "Named" mode, can appear on a different rule that happens to get the same id. Removing an earlier line with the same keyword shifts `ordinal`, which moves the state onto the neighbouring card. The `bad-text` flag is only reset by a later successful commit on that same key.
**Fix:** Clear this transient state when ids are re-keyed. Option 1: salt the body's ids with `WorkingFilter::generation()` or a per-parse epoch. Option 2: call `ctx.data_mut(|d| d.clear())` for these keys in `clear_ids`/`run_apply`/`revert`. Also reset `bad_id` when the row collapses.

## Info

### IN-01: `OverlayApp::classes()` is still dead code behind `#[allow(dead_code)]`

**File:** `src/overlay/app.rs:396-400`
**Issue:** The 20-06 summary said the accessor was a stopgap until 20-07 used it. 20-07 reads `self.classes` directly instead, so the accessor and its allow attribute are now dead.
**Fix:** Remove the accessor, or use it at the `show_list` call site.

### IN-02: The outcome-area height is a duplicated magic number

**File:** `src/overlay/app.rs:246-247, 291`
**Issue:** `OUTCOME_MAX_HEIGHT` exists only under `#[cfg(test)]`, while production code uses the literal `96.0` (per the 20-09 summary, to satisfy a plan grep). The test constant can drift from what is actually rendered.
**Fix:** Make the const unconditional and use it in `max_height(OUTCOME_MAX_HEIGHT)`.

### IN-03: `body_view` re-implements the disabled-line offset

**File:** `src/overlay/rules.rs` (`body_view`, `text.find('#')`)
**Issue:** `lineedit::grammar_offset` uses `disabled::uncomment_once`, while `body_view` uses `text.find('#')`. They agree today only because body lines of disabled blocks start with optional whitespace followed by `#`. Two definitions of the token base can drift apart and mis-slice value ranges.
**Fix:** Expose a shared helper, such as `Document::grammar_offset` or `disabled::uncomment_once` re-exported as `pub(crate)`, and use it in both places.

### IN-04: An integration test asserts a wall-clock limit

**File:** `tests/overlay_rules.rs` (`fixture_line_edits_stay_fast_and_revert_restores_bytes`)
**Issue:** `assert!(started.elapsed() < Duration::from_secs(15))` depends on machine load and on debug or release profile, so it can fail spuriously on a slow or busy runner.
**Fix:** Drop the timing assertion or put it behind an opt-in `#[ignore]` benchmark. Keep the revert/bytes assertion.

---

_Reviewed: 2026-10-13T04:24:33Z_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
