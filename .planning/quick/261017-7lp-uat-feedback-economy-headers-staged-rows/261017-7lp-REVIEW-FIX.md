---
phase: quick-261017-7lp
review_path: .planning/quick/261017-7lp-uat-feedback-economy-headers-staged-rows/261017-7lp-REVIEW.md
iteration: 1
findings_in_scope: 6
fixed: 6
skipped: 0
status: all_fixed
---

# Quick 261017-7lp: Code Review Fix Report

Worked directly on the milestone branch (no separate worktree: the untracked quick-task files and
the requested direct commits live in the main tree).

## Fixed

- **WR-01** (`5f603ac`, fmt follow-up `08b8cbe`): `add_field` now sizes the field from `FIELD_GAP` and
  the button's real width (the caption can push it past 84pt); the RULES search row uses
  `8 + FIELD_GAP` too. Audit: the other `ff_text_field*` callers either use the full width, a fixed
  width, or sit last in a right-to-left row (no gap added). Tests: new
  `add_field_row_fits_its_available_width` (fails without the fix); `search_field_and_add_button_do_not_touch`
  updated to the 10pt gap. `shots/e-field-gap.png` re-taken and viewed (field, gap, + Add all inside the row).
- **WR-02** (`2dccb7d`): a landed sync calls the new `prices_changed()`, which clears
  `staged_econ_inputs`, so the preview is re-priced. Test `staged_economy_preview_is_re_priced_after_a_sync`
  (fails without the reset).
- **WR-03** (`bf77762`): `stage_add_economy` / `set_staged_economy_body` return `bool`; a body that
  cannot be previewed stages nothing (or keeps the previous preview) and the app sets a
  `list_status` message. Test `a_body_that_cannot_be_previewed_stages_nothing`.
- **Unrated, preview ids** (`e452a5a`): a re-price collapses an expanded row whose id is a preview id
  (`rules::is_preview_id`). Asserted in `staged_economy_preview_follows_pending_tier_edits`.
- **IN-01** (`47c3d2d`): search-row comment and `generate_economy` doc comment rewritten.
- **IN-02** (`e5bfa48`): removed `count_blocks` and its test.
- **IN-03** (`83107ad`): `\n` escapes in the two test string sites.

Not addressed: pricing failure still caches the inputs (no retry until inputs change or a sync lands;
a sync now resets it).

## Gates

- `cargo test`: 1392 lib tests plus all integration tests passed.
- `cargo fmt --check`: clean.
- `cargo clippy --all-targets`: 75 (lib) + 44 (lib test, excluding duplicates) + 2 + 1 = 122 unique, at most 126.
- `git diff 2305cca -- tests/fixtures`: empty.
- Built with `CARGO_TARGET_DIR` outside the repo (ff.exe was locking `target/`).
