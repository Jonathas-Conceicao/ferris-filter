---
phase: 16-lossless-filter-parser
fixed_at: 2026-10-12T00:00:00Z
review_path: .planning/phases/16-lossless-filter-parser/16-REVIEW.md
iteration: 1
findings_in_scope: 7
fixed: 7
skipped: 0
status: all_fixed
---

# Phase 16: Code Review Fix Report

**Fixed at:** 2026-10-12
**Source review:** .planning/phases/16-lossless-filter-parser/16-REVIEW.md
**Iteration:** 1

**Summary:**
- Findings in scope: 7 (1 Critical, 6 Warning; Info findings out of scope)
- Fixed: 7
- Skipped: 0

**Process note:** fixes were committed directly on
`milestone/v0.5-generic-filter-editor-initial-app-configuration`, with no temporary worktree or
branch, because the user said not to create or switch branches. Builds used
`CARGO_TARGET_DIR=C:/Users/jonat/AppData/Local/Temp/ff-target`.

**Final test run:** full `cargo test` passed, 473 tests and 0 failures. The baseline before the fixes
was 457. Lib went from 401 to 412 and `filter_roundtrip` from 16 to 18. `artifact_guard` and
`generate_snapshot` pass, and both protected fixtures are unchanged. `cargo clippy --all-targets`
reports nothing in `src/filter` or `tests/filter_roundtrip.rs`.

## Fixed Issues

### CR-01: Enabling a "Valid" disabled block turns prose comments into live unknown-keyword lines

**Files modified:** `src/filter/disabled.rs`, `src/filter/edit.rs`
**Commit:** 6e5356c
**Applied fix:** `disabled::detect` keeps a `pending` list of non-grammar lines seen after the
current `end`. When a later grammar line pulls them inside the owned range, they become `Invalid`
reasons ("`note` is not filter grammar"). Trailing prose stays an ordinary comment outside the
block, which keeps the existing `trailing_banner_not_owned` behavior. As defense in depth,
`commit` now also rejects `Warning` diagnostics in the edited span when enabling.
**Tests:** `interior_prose_blocks_enabling`, `trailing_prose_keeps_block_valid` and
`enable_with_interior_prose_refused`. The last one covers both the parsed case and the
`replace_block_lines` path.
**Status:** fixed (logic change). Please confirm the classification rule (see the design notes below).

### WR-01: `can_enable()` returns true for blocks that `set_enabled(true)` then refuses

**Files modified:** `src/filter/disabled.rs`
**Commit:** 674c013
**Applied fix:** Three kinds of line are now deferred reasons on the same pending mechanism:
- commented `Import` directives
- header words with trailing text (`#Show Continue` inside a block)
- tokenizer failures that do not start with a grammar word (`# $tag`)

**Tests:** `can_enable_agrees_with_enable` covers `$tag`, `#\t123`, `#Import`, `#Show Continue`,
and checks that trailing tags stay ordinary comments.

### WR-02: Disabling fails with `NotOneBlock` for several ordinary live blocks

**Files modified:** `src/filter/disabled.rs`, `src/filter/edit.rs`
**Commit:** 79ccc43
**Applied fix:**
- A disabled line that is **body-indented** after its `#` (a tab, or at least two spaces) is now
  owned as a body line. This includes unknown keywords and non-keyword bad tokens such as
  `#\tFutureKw 3` and `#\t123`. These lines always carry a reason, so the block is `Invalid` and
  can never be enabled. This mirrors the live extent, so the broken blocks that disabling is meant
  to fix can now be disabled.
- The commented-condition and grammar-word-comment cases are fixed by WR-03.
- The compact header case (`Show Continue`) is not representable. Accepting `#Show <tokens>` as a
  disabled header would also swallow prose such as `# Show items above 80`. Disabling that block
  now returns the new dedicated `EditError::CannotDisable` instead of `NotOneBlock`.

**Tests:** `disable_blocks_with_bad_or_unknown_last_line` covers six shapes, including
space-indented ones. `disable_compact_header_is_dedicated_error` covers the compact header.
**Status:** fixed (logic change). Please confirm.

### WR-03: Comment lines are attached to the wrong block

**Files modified:** `src/filter/disabled.rs`, `src/filter/parse.rs`, `src/filter/edit.rs` (test)
**Commit:** 404c792
**Applied fix:**
- Disabled-line classification is now one function, `disabled::classify_inner`, returning
  Neutral / Header / Body / Prose.
- (1) `scan_live_body` now takes a contiguous run of commented-out body lines (`#\tSetFontSize 40`)
  that directly follows the last keyword line as part of the live block. A blank line or prose
  ends the run. `detect` applies the same rule one `#` deeper (`##\t...` at `k == end`), so a
  block's extent is the same before and after disabling or enabling.
- (2) In disabled bodies, a `QuotedList` line with several values, some of them unquoted, and no
  body indentation (`# Class weapons below`) is treated as prose. `#\tClass Rings Amulets`, which
  is indented, is still grammar.

**Tests:** `trailing_commented_condition_stays_with_its_block`,
`disabled_block_does_not_absorb_prose_with_grammar_word` and
`disable_enable_with_trailing_commented_condition`.
**Status:** fixed (logic change). Please confirm.

### WR-04: `replace_block_lines` accepts enabled blocks that carry Warning-level grammar problems

**Files modified:** `src/filter/edit.rs`
**Commit:** 6135d86
**Applied fix:** Validation of an enabled candidate now rejects both `Error` and `Warning`. This
covers unknown keywords and trailing header tokens. Disabled candidates are unaffected. I chose
the plain reject over the opaque-line allow-list: under D-11 the tool never writes an invalid
enabled block, and a loaded file's unknown line can still be carried by disabling the block.
**Tests:** `replace_with_warning_level_problems_refused`.

### WR-05: `set_meta` value does not round-trip, and there is no way to clear a key

**Files modified:** `src/filter/edit.rs`, plus `CHANGELOG.md` in a separate docs commit, cb3fc6a
**Commit:** c109155
**Applied fix:**
- `set_meta` trims surrounding spaces and tabs, so the stored value is exactly what `meta_get`
  returns. An empty or whitespace-only value is refused with `InvalidMeta`.
- New `Document::remove_meta(id, key)` deletes every `#ff:<key>` line attached to the block. It
  does nothing when the key is absent, and block IDs stay stable.

**Tests:** `meta_value_round_trips_and_remove`.

### WR-06: The disable/enable fixture test silently tolerates failures

**Files modified:** `tests/filter_roundtrip.rs`
**Commit:** b93f0d6
**Applied fix:**
- `edit_disable_enable_roundtrip_filterblade` now asserts on all 460 enabled blocks, and both
  calls must succeed.
- New `edit_enable_disable_roundtrip_filterblade`: every disabled fixture block must report
  `can_enable()`, enable, and disable again byte-identically.
- New `edit_disable_enable_roundtrip_seed_universal` runs the same sweep over the space-indented
  generator output.
- The CR-01 and WR-01 unit tests landed with those fixes.

## Design notes

These choices are consistent with 16-CONTEXT.md D-08, D-09 and D-11. A disabled line's text after
its first `#` is classified as:

- **Neutral**: blank, or a pure `##` comment. It is owned only inside the range. A `##`-commented
  body line directly at `end` extends the range (mirror of WR-03).
- **Header**: ends the block.
- **Body**: a known condition, action or modifier, unless it is prose-shaped. A body-indented
  unknown or bad line also counts, but always carries a reason.
- **Prose**: everything else. Trailing prose is not owned. Prose inside the range makes the block
  `Invalid`.

The **body-indented** heuristic (a tab, or at least two spaces after the `#`) separates a disabled
grammar line from conventional `# prose`. One block shape still cannot round-trip: a live block
indented with exactly one space whose last line is unknown or bad. It now fails safely with
`CannotDisable`.

---

_Fixed: 2026-10-12_
_Fixer: Claude (gsd-code-fixer)_
_Iteration: 1_
