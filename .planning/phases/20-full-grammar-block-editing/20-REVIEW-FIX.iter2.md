---
phase: 20-full-grammar-block-editing
fixed_at: 2026-10-13T05:10:00Z
review_path: .planning/phases/20-full-grammar-block-editing/20-REVIEW.md
iteration: 1
findings_in_scope: 8
fixed: 8
skipped: 0
status: all_fixed
---

# Phase 20: Code Review Fix Report

**Fixed at:** 2026-10-13T05:10:00Z
**Source review:** .planning/phases/20-full-grammar-block-editing/20-REVIEW.md
**Iteration:** 1

**Summary:**
- Findings in scope: 8 (CR-01, WR-01..WR-07)
- Fixed: 8
- Skipped: 0
- Also fixed (optional Info, trivial): IN-01, IN-02. Not done: IN-03 (refactor), IN-04 (timing assertion kept, since it may encode a plan performance criterion).

Every in-scope fix has a regression test. `cargo fmt --check` is clean and the full `cargo test` passes (644 lib tests plus all integration tests, including `artifact_guard` and `generate_snapshot`). No fixture changed.

## Fixed Issues

### CR-01: Adding a rarity chip to a negated `Rarity` line drops the `!=` and flips the condition

**Files modified:** `src/overlay/body.rs`, `src/overlay/grammar.rs`
**Commit:** 0a84a2c
**Applied fix:**
- `rarity_edit` now drops only the ordering operators (`<`, `<=`, `>`, `>=`) on a multi-rarity edit. `!=`, `!`, `=` and `==` are kept as written (PARSE-04).
- In list form, the operator label for `!=` is now "Not any of" and for `!` it is "Not any of (!)", instead of "Matches".
- The "Several rarities ..." note under the chips now reads "exclude all of them" for a negated line.

**Tests:** `rarity_edit_keeps_a_negation`; extended `rarity_operator_sets`.
**Status:** fixed: requires human verification (logic change)

### WR-01: The `Continue` checkbox is locked on styled rules

**Files modified:** `src/overlay/body.rs`
**Commit:** 4ee2c03
**Applied fix:** The Continue checkbox is always enabled. Only the Show/Hide/Minimal buttons stay locked on a known style.
**Tests:** `known_style_row_keeps_continue_editable` is a click sweep. It fails without the fix and passes with it.

### WR-02: One styled rule that cannot be rewritten blocks Apply for the whole filter

**Files modified:** `src/styled.rs`, `src/overlay/apply.rs`, `src/overlay/app.rs`
**Commit:** 20b1ee2
**Applied fix:**
- `StyleReport` gains `failed: Vec<(usize, String)>`.
- `rewrite_block` is now all-or-nothing: if the splice is refused, it undoes the keyword switch it just made.
- `rewrite_styled` records a refused rule by its rule index, leaves it byte-identical, and goes on with the other rules. It still returns `Err`, but only if a refused rewrite could not be undone (the block's bytes changed).
- The Apply message names the rule through the new `apply::failed_styles_message`.

**Tests:** `refused_rewrite_is_reported_and_does_not_block_others` (a `Hide Continue` header beside a rewritable rule); extended `style_messages`.
**Note:** The footer's persistent jump line was not extended. The failure is named in the Apply outcome message only.

### WR-03: The model style lock can be bypassed, and the bypassing edit is then deleted on Apply

**Files modified:** `src/overlay/rules.rs`
**Commit:** 53eaf0e
**Applied fix:**
- `AddLine` tokenizes the text and refuses it when the keyword kind conflicts with the declared section:
  - An action declared as a condition gets the style-lock message on a styled rule and `ACTION_IN_CONDITIONS_STATUS` otherwise.
  - A condition declared as an action gets `CONDITION_IN_ACTIONS_STATUS`.
  - Unknown keywords and modifiers are let through.
- On a known-styled rule, a `LinePatch::Whole` edit whose new text would be an action line is refused with the style-lock message.

**Tests:** `known_style_lock_cannot_be_bypassed`.
**Status:** fixed: requires human verification (logic change)

### WR-04: Editing a colour with alpha 0 loses its RGB

**Files modified:** `src/overlay/widgets.rs`
**Commit:** 62a96d4
**Applied fix:** `colour_swatch` now edits an unmultiplied `Hsva`, built with `Hsva::from_rgba_unmultiplied` straight from the line's sRGB values, using `color_edit_button_hsva`. Before, it built a premultiplied `Color32`. Every alpha round-trips exactly, and raising the alpha of `40 42 54 0` gives `40 42 54 255`.
**Tests:** `colour_model_keeps_rgb_at_zero_and_low_alpha`; the idle-frame test now also covers alpha 0.
**Status:** fixed: requires human verification. Please check the picker in the GUI. Without egui's colour cache, the hue slider of a pure grey may snap to 0 between frames, which is cosmetic.

### WR-05: A mod condition added from the picker gets an exact count of 1

**Files modified:** `src/overlay/body.rs`
**Commit:** 9855b0d
**Applied fix:** `pending_first_entry` writes `CountThenNames` lines without a count (`HasExplicitMod "Tyrannical"`), which is the existing "Any of these:" form.
**Tests:** extended `pending_first_entry_builds_the_full_line`, including a check that the model accepts the count-less line.
**Status:** fixed: requires human verification (logic change)

### WR-06: A rejected first entry on a pending row is dropped without any message

**Files modified:** `src/overlay/widgets.rs`, `src/overlay/body.rs`
**Commit:** 2104e85
**Applied fix:**
- `chip_list` refuses any draft whose quotes are not a clean quoted paste, such as `Foo"Bar`, an unclosed `"Foo`, or text outside the quotes. It also refuses entries that contain quotes or control characters.
- When it refuses, it now sets `rejected` and keeps the draft. Before, it cleared the draft.
- `pending_card` shows `BAD_TEXT_HINT` when the first entry still cannot be written. The flag lives in egui temp memory and is cleared on success or discard.

**Tests:** `chip_list_stray_quotes_are_refused_and_the_draft_kept`.
**Note:** On a control-character refusal, the draft is now kept rather than cleared, as the review intended.

### WR-07: Per-card UI state is keyed by reusable `BlockId`/ordinal and leaks onto other rules

**Files modified:** `src/overlay/rule_list.rs`, `src/overlay/app.rs`
**Commits:** df5abc0 (and c7d9833, a warning cleanup in the new test)
**Applied fix:** `ListUi` gains an `epoch`, which is mixed into every row's `push_id`. The epoch is bumped:
- in `clear_ids` (Apply, Revert, RELOAD);
- when the expanded row collapses or switches;
- after a successful `RemoveLine`, since a removal shifts keyword ordinals.

Stale bad-text and bad-int flags, the `Named` mode and raw drafts are therefore orphaned instead of resurfacing on another card. A value edit does not bump the epoch, so focus and drafts survive ordinary editing.

**Tests:** `collapsing_a_row_orphans_card_state`, `remove_line_orphans_card_state`; extended `clear_ids_resets_every_id_keyed_field`.

### IN-01 (optional): `OverlayApp::classes()` dead code

**Files modified:** `src/overlay/app.rs`
**Commit:** 7bc2934
**Applied fix:** Removed the accessor and its `#[allow(dead_code)]`.

### IN-02 (optional): Duplicated outcome-area height

**Files modified:** `src/overlay/app.rs`
**Commit:** e2b8a69
**Applied fix:** `OUTCOME_MAX_HEIGHT` is now unconditional and used in `max_height(...)`.

---

_Fixed: 2026-10-13T05:10:00Z_
_Fixer: Claude (gsd-code-fixer)_
_Iteration: 1_
