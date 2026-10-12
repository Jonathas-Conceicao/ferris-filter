---
phase: 16-lossless-filter-parser
fixed_at: 2026-10-12T00:00:00Z
review_path: .planning/phases/16-lossless-filter-parser/16-REVIEW.md
iteration: 2
findings_in_scope: 4
fixed: 4
skipped: 0
status: all_fixed
---

# Phase 16: Code Review Fix Report (iteration 2)

**Fixed at:** 2026-10-12
**Source review:** .planning/phases/16-lossless-filter-parser/16-REVIEW.md
**Iteration:** 2 (the iteration-1 report is kept as `16-REVIEW-FIX.iter2.md`)

**Summary:**
- Findings in scope: 4 (Warnings WR-01 to WR-04; the 7 Info findings are out of scope)
- Fixed: 4
- Skipped: 0

**Process note:** As in iteration 1, the fixes were committed directly on
`milestone/v0.5-generic-filter-editor-initial-app-configuration`, with no worktree or temporary
branch, because the user asked for that. Builds used
`CARGO_TARGET_DIR=C:/Users/jonat/AppData/Local/Temp/ff-target`.

The order of work was WR-01, then WR-04, WR-02 and WR-03, because WR-04 and WR-02 build on the
classification rule from WR-01.

**Final test run:** `cargo test` passed in full with 475 tests and 0 failures.
- The lib suite has 417 tests. It had 412 before this iteration.
- `filter_roundtrip` has 18 tests.
- `artifact_guard` and `generate_snapshot` pass.
- Neither protected fixture was touched.
- `cargo clippy --all-targets` reports no new lints. The lints it does report in `src/filter` (two
  collapsible `if`s and items after a test module) were all there before this iteration.

I also ran a temporary corpus sweep. It was not committed and has been deleted. It covered 26
filters on this machine: the repo root, `reference-old-filters/`, `tests/fixtures/**` and the
game folder. Every file serialized byte-identically. Every enabled block survived disable then
enable byte-identically, and every disabled block survived enable then disable. That was 0
failures, and 0 disabled blocks came out `Invalid`.

## Fixed Issues

### WR-01: Prose that starts with a grammar word, or is indented by two spaces, is classified as a disabled body line

**Files modified:** `src/filter/disabled.rs`, `src/filter/parse.rs`
**Commit:** fa70484
**Applied fix:** There is now one layout rule, `disabled::indent_ok`. Through `classify_inner`, it
serves both the live trailing run and disabled-block detection. A commented line that is not a
header counts as a disabled body line only when it is laid out like the block's body.
- **Once the block has a body line:** the line's leading whitespace after the `#` must equal that
  body line's indentation, which is the established indentation.
- **Lines with a problem** (bad values, an unknown keyword, or text that cannot be tokenized) must
  also be indented at all.
- **Before the first body line:** a clean grammar line counts at any indentation. A line with a
  problem counts only when it is body-indented (a tab or two spaces).

Everything else is prose. This replaces the old `prose_shaped` special case.

**Tests:**
- `trailing_headings_with_grammar_words_stay_with_next_block` (live) covers every shape in the
  review: `# Corrupted items`, `# Quality gear below`, `# ItemLevel >= 80 only`,
  `# Continue below`, `# Sockets 3 is good`, `#  Note: ...`, `#   [[0100]] Section` and
  `#Corrupted True`.
- `headings_with_grammar_words_do_not_invalidate_disabled_rules` (disabled) covers the same shapes.
  Each block stays `Valid`, and enabling it succeeds.

**Status:** fixed: requires human verification (classification logic). See design decision 1.

### WR-04: The extent rules of `detect` and `scan_live_body` disagree

**Files modified:** `src/filter/disabled.rs`, `src/filter/parse.rs`, `src/filter/edit.rs`
**Commit:** 9a807ef
**Applied fix:**
- **Prose ends a disabled block.** `detect` stops at the first prose line, at exactly the point
  where the live trailing run stops. The `pending` mechanism is gone. Prose can no longer sit
  inside a disabled block's range, so CR-01 (enabling prose into a live line) is now prevented by
  construction rather than by an `Invalid` mark.
- **The live trailing run passes neutral comment lines** (`#`, `## ...`) and extends to the last
  commented body line. It stops at a non-comment line, a heading/prose line, or a header.
- **`detect` mirrors that run one `#` deeper.** A `##\tKw` line extends the block only while the
  run is still open. A `#` (blank) line or a doubly commented heading closes the run.
- **Backstop: an extent check.** A disabled block counts as enableable only when its enabled
  extent equals its disabled extent. The enabled extent is computed by the live scanner itself
  (`scan_live_body_with`) over an uncommented view of the lines, so no new lines are built. This
  guarantees that `can_enable()` agrees with `set_enabled(true)`. It also resolves Info finding
  IN-01: a stray condition after a blank line now makes the block `Invalid` with a reason, instead
  of `can_enable()` returning true while `set_enabled` fails.
- `set_enabled(true)` and the extent check share `disabled::enabled_text`.

**Tests:**
- The review's input `Show\n\tRarity Rare\n# note\n#\tClass "Bows"\nHide\n` was added to
  `disable_blocks_with_bad_or_unknown_last_line`.
- `can_enable_agrees_with_enable` was rewritten as a table of nine cases, including IN-01 and the
  `##` run-closing shapes.
- `prose_ends_disabled_block` and `enable_never_turns_prose_into_grammar` replace the two
  iteration-1 tests that asserted the old `Invalid` 0..4 extent.
- New `exhaustive_small_documents_agree` enumerates all 41,472 documents built as `Show`/`#Show`,
  then 4 lines from a pool of 12 tricky lines, then `Hide`. For every block, `can_enable()` must
  agree with `set_enabled(true)`, and disable/enable must round-trip byte-identically. Any refused
  edit must leave the bytes untouched. The test runs in about 2 s in debug.

**Status:** fixed: requires human verification (extent logic). See design decision 2.

### WR-02: A `#ff:` metadata line is pulled into the preceding disabled block

**Files modified:** `src/filter/disabled.rs`, `src/filter/parse.rs`
**Commit:** f83a2b4
**Applied fix:** `detect` and the live trailing run in `scan_live_body` both stop explicitly at a
line that `meta::parse_ff_line` recognizes. A `#ff:` line therefore always stays in the header run
of the block below it (D-05).

The WR-01 and WR-04 changes had already fixed the reproduced input, because `#ff:...` is never
laid out like a body line and prose now ends a block. The explicit stop makes the rule independent
of the classification heuristic.

**Tests:** `ff_line_is_never_pulled_into_a_block` covers the review's disabled input, the same
input with a live block, and the input with an indented `  #ff:` line. In each case `meta_get`
still finds the name on `Hide`.

### WR-03: `commit` validates only the edited block, so an edit can silently move lines between other blocks

**Files modified:** `src/filter/edit.rs`
**Commit:** 6c22b72 (plus `CHANGELOG.md` in the docs commit d3c93d2)
**Applied fix:** `commit` now calls `check_neighbours`. It compares the rebuilt item list with the
old one. Every item other than the edited block must keep its signature. That covers blocks,
comment runs, imports and opaque lines. For a block, the signature is its range (leading start,
header and end), keyword, enabled state, `can_enable` and body length. Items below the edit are
shifted by the line-count change. The edited block must also keep its leading-run start.

Any difference refuses the edit with the new `EditError::WouldReattributeLines`, and the document
is left untouched. `set_meta` and `remove_meta` now report that error instead of `NotOneBlock`.

**Tests:**
- `edit_never_reattributes_neighbour_lines` covers the review's `remove_meta` repro. The edit is
  refused, the bytes do not change, and the name is kept. Rewriting the value in place still works,
  and so does a removal that does not affect a neighbour.
- `exhaustive_small_documents_agree` also exercises `set_meta`/`remove_meta` on every block of all
  41,472 documents:
  - A successful `set_meta` must leave every other block's span unchanged, apart from the shift.
  - `remove_meta` is either refused with `WouldReattributeLines` and the bytes unchanged, or it
    restores the original bytes.

## Design decisions

1. **One layout rule, and lines with a problem can still extend a block.** The guidance suggested
   requiring a commented line to validate fully *and* match the indentation. I kept the indentation
   requirement but did not require validity, and the review's own `is_commented_body` suggestion
   (clean lines only) was not adopted either. The reason is that the live trailing run and
   `detect` must use the same rule.

   `detect` has to own the disabled form of a broken live body line, such as `#\tFutureKw 3`.
   Iteration-1 WR-02 made disabling the remedy for a broken block, and that only works if `detect`
   owns those lines. If the live run took only clean lines, then
   `Show\n\tRarity Rare\n#\tFutureKw 3\nHide` would have a live extent of 0..2 but a disabled
   extent of 0..3, and disabling it would fail.

   The exact-indentation requirement is what removes the false positives in the review. The
   remaining ambiguity is accepted: a prose comment that sits directly after a block and is
   indented exactly like that block's body (e.g. `#\tNote here` under a tab-indented body) still
   reads as a disabled line. It moves with the block, and it can never be enabled, because it
   carries a problem.

2. **Prose ends a disabled block, rather than making it `Invalid`.** This is how the review's WR-04
   recommendation reads in practice. It changes two iteration-1 expectations:
   - `#Show\n#\tRarity Rare\n# note\n#\tClass "Bows"` is now a valid block on lines 0..2. The rest
     is standalone comment text, where before it was an `Invalid` block on lines 0..4.
   - A `replace_block_lines` candidate with prose between body lines is now refused with
     `NotOneBlock`, where before it was accepted as an `Invalid` disabled block.

   Both outcomes are safer, because no prose can ever be inside a block's range. The tests were
   updated to match.

3. **Remaining edge cases fail safely.** Some hand-made layouts still cannot be disabled, and the
   tool refuses them with an error rather than changing anything:
   - a live block whose first body line is space-indented and broken, such as
     `Show\n  FutureKw 3`;
   - an unindented live body that ends in a broken line.

   These are consistent with 16-CONTEXT.md D-08 to D-11. Nothing in the local corpus has these
   shapes.

## Out of scope (Info, not attempted)

- IN-02 to IN-07 are unchanged.
- IN-01 is resolved as a side effect of the WR-04 extent check (see above).

---

_Fixed: 2026-10-12_
_Fixer: Claude (gsd-code-fixer)_
_Iteration: 2_
