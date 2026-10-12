---
phase: 16-lossless-filter-parser
fixed_at: 2026-10-12T00:00:00Z
review_path: .planning/phases/16-lossless-filter-parser/16-REVIEW.md
iteration: 3
findings_in_scope: 3
fixed: 3
skipped: 0
status: all_fixed
---

# Phase 16: Code Review Fix Report (iteration 3)

**Fixed at:** 2026-10-12
**Source review:** .planning/phases/16-lossless-filter-parser/16-REVIEW.md
**Iteration:** 3 (final). The iteration-1 report is kept as `16-REVIEW-FIX.iter2.md` and the
iteration-2 report as `16-REVIEW-FIX.iter3.md`.

**Summary:**
- Findings in scope: 3 (CR-01, WR-01, WR-02). The 7 Info findings are out of scope.
- Fixed: 3
- Skipped: 0

**Process note:** The fixes were committed directly on
`milestone/v0.5-generic-filter-editor-initial-app-configuration`, with no worktree or temporary
branch, because the user asked for that. Builds used
`CARGO_TARGET_DIR=C:/Users/jonat/AppData/Local/Temp/ff-target`.

**Final test run:** `cargo test` passed in full with 480 tests and 0 failures.
- The lib suite has 422 tests. It had 417 before this iteration.
- `filter_roundtrip` has 18 tests.
- `artifact_guard` and `generate_snapshot` pass.
- `tests/fixtures/seed-universal.filter` and `tests/fixtures/parser/filterblade.filter` were not
  touched.
- `cargo clippy --lib --tests` reports nothing new in `src/filter`. The three lints it does report
  there (edit.rs collapsible `if`, tokenize.rs collapsible `if`, mod.rs items after a test module)
  were all there before.

**Real-filter sweep:** I ran a temporary sweep. It was not committed and has been deleted. It
covered 26 filters: the repo-root `*.filter`, `reference-old-filters/`, `tests/fixtures/**` and the
game folder (read-only). The checks were:
- Every file serialized byte-identically.
- Every enabled block (3,598) was disabled on a fresh copy. Each disabled form could be enabled,
  and enabling it restored the original bytes. No disable was refused.
- Every disabled block (115) was enabled, then disabled again, byte-identically. None was
  `Invalid`.

That was 0 failures.

## Fixed Issues

### CR-01: Enabling a disabled rule silently drops its later conditions when indentation is inconsistent or a comment line sits between body lines

**Files modified:** `src/filter/disabled.rs`, `src/filter/edit.rs`, `src/filter/parse.rs`
**Commit:** 46a1393
**Applied fix:** There are four parts.
1. **Clean lines extend a block at any indentation.** A commented condition, action or `Continue`
   that validates fully now extends the block whatever its indentation: tabs, spaces, mixed or
   none. This is the user's guidance, and it is broader than the reviewer's "non-empty
   indentation". It mirrors the live scanner, which takes a live body line at any indentation.
   Indentation now only decides for lines with a problem. Those still need the exact established
   indentation, and it must be non-empty. The rule is in `disabled::indent_ok` and serves both the
   live trailing run and `detect`, so the live and disabled extents still agree.
2. **Look-ahead after prose.** When prose ends a disabled block, `detect` now scans the rest of the
   commented run. If a clean commented condition follows at any indentation, the block keeps its
   extent but becomes `Invalid`, with a reason tied to that line, and the enable is refused. The
   scan stops at a header, a `#ff:` line or a non-comment line. The function is
   `clean_grammar_ahead`. The review's third input (`#Hide` / `Rarity Normal` / prose / `BaseType`)
   is now refused, not enabled as a broader rule.
3. **Disable guard.** `set_enabled(false)` on a live block with no Error or Warning diagnostic in its
   span must now produce a disabled form that `can_enable()`. Otherwise it returns `CannotDisable`
   and the bytes are untouched. Without this, part 2 would let a clean block be disabled into a
   rule that can never be turned back on. An example is
   `Show\n\tRarity Rare\n# note\n#\tClass "Bows"\nHide`, where the last two lines are Hide's leading
   comments.
4. **`prose_shaped` is back, as a problem.** Part 1 alone would make `# Class weapons below` a clean
   condition, because `Class` accepts bare words. A list-valued line with several values, some of
   them unquoted, and no body indentation therefore counts as a line with a problem again. The old
   iteration-1 heuristic was reinstated in that form. It can no longer extend a block at an unusual
   indentation, and the look-ahead never counts it.

**Tests:**
- The `exhaustive_small_documents_agree` pool grew from 12 to 16 lines. The new lines are
  `    Class "Bows"`, `#    Class "Bows"`, `#Rarity Rare` and `Rarity Rare`. The pool already had
  prose between conditions (`# note`, `# Corrupted items`, `#  Note: x`). The test now covers
  131,072 documents in about 11 s in debug.
- The test asserts that:
  - every successful enable yields at least as many body lines as there are clean commented
    conditions in the run the block came from;
  - a clean live block that disables successfully is always enableable.
- New `enable_never_drops_a_commented_condition` uses the review's inputs.
- New `prose_between_conditions_refuses_enable` covers the refusals, plus the cases where a `#ff:`
  line, a header or a blank line ends the look-ahead.
- New `mixed_indentation_enables_whole_rule` checks the review's first two inputs. Each enables
  whole and disables back byte-identically.
- These iteration-2 expectations changed on purpose: `can_enable_agrees_with_enable`,
  `prose_ends_disabled_block`, `enable_never_turns_prose_into_grammar`,
  `disable_blocks_with_bad_or_unknown_last_line`, and `#Corrupted True` in
  `trailing_headings_with_grammar_words_stay_with_next_block`. That last line is now a disabled
  condition of the block above.

**Status:** fixed: requires human verification (classification and extent logic). See design
decisions 1 to 3.

### WR-01: A valid live block with mixed body indentation can no longer be disabled

**Files modified:** `src/filter/edit.rs`
**Commit:** b2749a7. The behaviour change itself is in 46a1393, because WR-01 has the same root
cause as CR-01.
**Applied fix:** CR-01 part 1 makes the disabled form of every clean live body line a body line at
any indentation, so these blocks disable again. New `disable_mixed_indentation` covers the
review's three inputs plus a CRLF variant with mixed, space and unindented lines. Each block
disables, stays enableable with the same body length, and enables back byte-identically.

### WR-02: A header-only block still takes the next block's two-space or tab-indented heading

**Files modified:** `src/filter/disabled.rs`, `src/filter/parse.rs`, `src/filter/edit.rs`
**Commit:** 20e1f43
**Applied fix:** I did not apply the review's `None => clean` as written. See design decision 4.
- **Directly under a header** (no body yet), a line with a problem is a body line only when it is
  body-indented (tab or two spaces) **and** starts with a known condition or action word.
- **Headings stay with the next block.** `#  Note: these are weapons`, `#\tSee notes below` and
  `#\tFutureKw 3` after a header-only block are now the next block's leading comments, both live
  and disabled.
- **A refusal guard for disabled header-only blocks.** A disabled header whose next line is
  body-indented but was not taken (for example `#Hide\n#\tFutureKw 3`) is `Invalid`. Enabled, the
  bare header would match every item, and for `Hide` that would hide all loot.

**Tests:**
- `trailing_headings_with_grammar_words_stay_with_next_block` gained the review's inputs and
  `#\tFutureKw 3`.
- New `header_only_disabled_block_leaves_heading_and_never_enables_as_catch_all`.
- `replace_with_warning_level_problems_refused` now builds its disabled unknown-keyword candidate
  with a body line before the unknown keyword. The header-plus-unknown-word candidate is asserted
  `NotOneBlock`, and the bytes are untouched.

**Status:** fixed: requires human verification (classification logic). One review input stays as
before; see design decision 4.

**Also:** `CHANGELOG.md` `[Unreleased]` now says that a disabled rule is only ever enabled whole
(commit 74e7ce3).

## Design decisions

1. **The safety property comes first.** Enabling a disabled block never silently drops a clean
   commented condition. The block is enabled whole or refused with a line-tied reason. Three
   mechanisms enforce this:
   - clean lines extend a block regardless of indentation;
   - the look-ahead after prose;
   - the existing `enabled_extent` backstop, which keeps `can_enable()` and `set_enabled(true)` in
     agreement.

   The exhaustive test asserts the property directly.

2. **The disable guard, a new refusal.** A clean live block is refused `CannotDisable` when its
   disabled text would read as a rule with a cut-off condition. That happens when the lines right
   below it are a comment and then a commented condition. The disabled text is literally the CR-01
   shape, so the ambiguity cannot be resolved from the text alone. Refusing the disable is better
   than producing a disabled rule the user can never turn back on. None of the 3,598 enabled blocks
   in the local corpus was refused.

3. **What still counts as prose.**
   - Lines with a problem, such as `# Corrupted items`, `# Sockets 3 is good` or `#  Note: x`,
     under a body indented differently.
   - List lines that read as prose, such as `# Class weapons below`.
   - Directives and header words with trailing text.

   Prose ends a block, as before. A prose comment that is clean grammar, such as `#Corrupted True`
   or `# Rarity Rare` right after a block, is now a disabled condition of that block, where
   iteration 2 treated it as the next block's heading. This is the price of never truncating a
   rule, and it is the conservative direction. Such a line moves with the block above, and
   enabling turns it into the condition its text spells out.

4. **WR-02 is narrowed, not `None => clean`.** The review's suggestion assumed it cost nothing new.
   In practice it broke iteration-1 behaviour:
   - a disabled rule whose first line is a broken known condition (`#Show\n#\tRarity =< Rare`) was
     no longer owned;
   - a broken one-condition live block (`Show\n\tRarity =< Rare`) could no longer be disabled,
     which is the remedy for a broken block (iteration-1 WR-02);
   - a disabled candidate carrying such a line was refused.

   Restricting the header-only leniency to body-indented lines with a known keyword keeps all of
   those, and it still fixes the review's first and third inputs.

   The review's second input, `Show # catch-all\n#  Rarity is irrelevant here`, keeps its old
   behaviour. A body-indented line starting with the known word `Rarity` is indistinguishable from
   the disabled form of `Show\n  Rarity is irrelevant here`. It is still never enabled: it carries
   a problem, so the disabled form is `Invalid`. The single-space form `# Rarity is irrelevant
   here` stays with the next block.

   The remaining cost: a live block whose **first** body line has an unknown keyword or cannot be
   tokenized (`Show\n\tFutureKw 3`) cannot be disabled. It is refused with `CannotDisable` and the
   bytes are untouched. Such a block already fails to load in the game, and it can still be
   replaced or deleted.

5. **No CONTEXT conflicts.** All four changes stay within 16-CONTEXT.md:
   - D-01: bytes are never rewritten.
   - D-08: one `#` is added or removed per line.
   - D-09: disabled rules are either valid or invalid with a reason.
   - D-10 and D-11: the tool never writes an invalid enabled block.

## Out of scope (Info, not attempted)

IN-01 to IN-07 are unchanged.

---

_Fixed: 2026-10-12_
_Fixer: Claude (gsd-code-fixer)_
_Iteration: 3_
