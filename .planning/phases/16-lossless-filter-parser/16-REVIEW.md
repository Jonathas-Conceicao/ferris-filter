---
phase: 16-lossless-filter-parser
reviewed: 2026-10-12T23:13:47Z
depth: standard
iteration: 3
files_reviewed: 13
files_reviewed_list:
  - .gitattributes
  - src/lib.rs
  - src/filter/mod.rs
  - src/filter/lines.rs
  - src/filter/diag.rs
  - src/filter/keywords.rs
  - src/filter/tokenize.rs
  - src/filter/block.rs
  - src/filter/parse.rs
  - src/filter/disabled.rs
  - src/filter/meta.rs
  - src/filter/edit.rs
  - tests/filter_roundtrip.rs
findings:
  critical: 1
  warning: 2
  info: 7
  total: 10
status: issues_found
---

# Phase 16: Code Review Report (iteration 3)

**Reviewed:** 2026-10-12
**Depth:** standard
**Files Reviewed:** 13
**Status:** issues_found

## Summary

This pass checks the four iteration-2 fixes (fa70484, 9a807ef, f83a2b4 and 6c22b72) and looks
for regressions. I used `cargo test` (all suites pass: 417 lib tests and 18 `filter_roundtrip`
tests) and a scratch crate that path-depends on the library. For comparison, I also ran the same
probes against a temporary checkout of `cb3fc6a`, the state before iteration 2. That checkout has
since been removed.

**Iteration-2 findings:**
- **WR-01 (headings with grammar words):** fixed for blocks that have a body. It is still open for
  header-only blocks; see WR-02 below.
- **WR-02 (`#ff:` pulled into a block):** fixed. `detect` and the live trailing run both stop at a
  `#ff:` line.
- **WR-03 (neighbour re-attribution):** fixed. `check_neighbours` compares the signature of every
  other item, shifted by the line delta. I found no edit that moves another block's lines and
  still returns `Ok`.
- **WR-04 (extent disagreement):** fixed for the shapes the review gave. The `enabled_extent`
  backstop holds: `can_enable()` agreed with `set_enabled(true)` in every probe.

**Byte identity:** this still holds. On all 26 local filters (`reference-old-filters/*`, the
repo-root filters, `tests/fixtures/**`, and the game folder), every enabled block can be disabled.
No disabled block in those files is cut off above a clean commented grammar line.

**D-11 (no invalid enabled block):** this still holds. `set_enabled(true)` rejects Error and
Warning diagnostics inside the span, and `replace_block_lines` checks the candidate in isolation.

**Regression:** the new "one layout rule" (`indent_ok`) requires a disabled body line to be
indented *exactly* like the previous body line, even when the line is clean grammar. Before
iteration 2, a clean grammar line extended a disabled block at any indentation. This has two
effects:
- **Enabling truncates the rule.** A disabled rule whose lines are indented inconsistently, or
  whose body has an interior prose line, is now classified `Valid` but cut short. Enabling it
  writes a live block with fewer conditions than the user's text. For a `Hide` rule, that hides
  far more loot than the user wrote. At `cb3fc6a` the same input was either enabled whole or
  refused.
- **Disabling is refused.** A valid live block with mixed indentation can no longer be disabled.

None of the local filters has these shapes, and the exhaustive test's line pool is entirely
tab-indented, so the test suite does not see the problem.

## Critical Issues

### CR-01: Enabling a disabled rule silently drops its later conditions when indentation is inconsistent or an interior comment line sits between body lines (regression from iteration 2)

**File:** `src/filter/disabled.rs:92-98` (`indent_ok`), `src/filter/disabled.rs:218-219` (`detect` breaks on `Prose`), `src/filter/parse.rs:338-349` (backstop only compares extents)

**Issue:** When `est` is set, `indent_ok` returns `lead == e && ...`, so a *clean* commented
grammar line with a different indentation becomes `Inner::Prose`. In `detect`, that ends the
block. The extent backstop (`enabled_extent`) does not catch it, because the live trailing run
applies the same rule and stops at the same line. The block is therefore classified `Valid` with a
truncated extent. `set_enabled(true)` then writes only the lines before the cut, and the rest of
the rule stays behind as standalone comment text.

Reproduced on the current HEAD, with the result at `cb3fc6a` for comparison:

```
#Show\n#\tRarity Rare\n#    Class "Bows"\n
  HEAD:    Valid, owned 0..2; enable -> "Show\n\tRarity Rare\n#    Class \"Bows\"\n"   (now matches every Rare item)
  cb3fc6a: owned 0..3;        enable -> "Show\n\tRarity Rare\n    Class \"Bows\"\n"

#Show\n#Rarity Rare\n#\tClass "Bows"\n
  HEAD:    enable -> "Show\nRarity Rare\n#\tClass \"Bows\"\n"   (Class dropped)
  cb3fc6a: enable -> "Show\nRarity Rare\n\tClass \"Bows\"\n"

#Hide\n#\tRarity Normal\n# only these bases\n#\tBaseType == "X"\n
  HEAD:    Valid; enable -> "Hide\n\tRarity Normal\n# only these bases\n#\tBaseType == \"X\"\n"   (hides ALL normal items)
  cb3fc6a: CannotEnable (refused)
```

The first two inputs contain no prose at all. They are unambiguous disabled blocks of
clean grammar, written by someone who indented one line differently, which is normal in
hand-edited filters. The third input is the iteration-2 "design decision 2" trade-off. It was
accepted as "safer", but the result is a live rule that is broader than its text. The library
offers no signal: the enable returns `Ok`, `diagnostics()` is empty, and the dropped lines carry
no diagnostic.

This violates D-09, which says a commented-out block is either enabled as written or refused with a
reason. A Phase 18-20 "enable" button on such a rule would silently change what the filter does,
and in the `Hide` case hide loot.

**Fix:** Keep the extents agreeing (WR-04), but refuse to enable a block whose text continues past
its detected end.
1. Relax `indent_ok` for **clean** lines. When `est` is set, accept a clean line whose indentation
   is any non-empty run, or equals `est`. Keep the exact-match requirement only for lines with a
   problem. That keeps `# Corrupted items` and `#  Note:` as prose, because they have problems,
   while `#    Class "Bows"` stays a body line. Apply the same change to the live trailing run so
   the extents still agree.
2. In `detect`, when the scan ends on `Inner::Prose` (or a `#ff:` line), look ahead through the
   contiguous run of commented lines up to the next header, non-comment line or blank `#`. If any
   line in that run classifies as a clean grammar line at *any* indentation, mark the block
   `Invalid`. Do not change its extent:

```rust
Inner::Header | Inner::Prose => {
    if let Some(j) = clean_grammar_ahead(ls, k) {
        reasons.push(FilterDiagnostic {
            line: j + 1,
            severity: Severity::Warning,
            message: "a commented condition below a comment line would be left behind when enabled".into(),
        });
    }
    break;
}
```

3. Add mixed-indentation shapes to the `exhaustive_small_documents_agree` pool, for example
   `"#    Class \"Bows\""`, `"    Class \"Bows\""` and `"#Rarity Rare"`. Then assert that a
   successful enable leaves no clean commented grammar line directly below the block.

## Warnings

### WR-01: A valid live block with mixed body indentation can no longer be disabled (regression from iteration 2)

**File:** `src/filter/disabled.rs:92-98`, `src/filter/edit.rs:235-236`

**Issue:** The live main loop (`scan_live_body_with`, parse.rs:254-305) takes body lines at any
indentation. Their disabled form goes through `indent_ok`, which needs an exact indentation match,
so `detect` ends early. `commit` then sees a different owned length and returns
`EditError::CannotDisable`. Reproduced on HEAD; all three were accepted at `cb3fc6a`:

```
Show\n\tRarity Rare\n    Class "Bows"\nHide\n   -> CannotDisable
Show\n    Rarity Rare\n\tClass "Bows"\nHide\n   -> CannotDisable
Show\n\tRarity Rare\nClass "Bows"\nHide\n       -> CannotDisable
```

These blocks have no diagnostics. Iteration-2 "design decision 3" says the only blocks that cannot
be disabled are ones whose layout has a broken line. That is no longer true, because a fully valid
block is also refused. The failure is safe, since the bytes are untouched, but disabling is how the
overlay will turn a rule off. A user who pasted lines with spaces into a tab-indented block cannot
turn that rule off.

**Fix:** This has the same root cause as CR-01, and step 1 of that fix (accept clean lines at any
non-empty indentation once `est` is set) resolves it. Add the three inputs above to
`disable_blocks_with_bad_or_unknown_last_line`, or to a new `disable_mixed_indentation` test, and
assert that disabling and then enabling is byte-identical.

### WR-02: A header-only block still takes the next block's two-space or tab-indented heading (iteration-2 WR-01 only partly fixed)

**File:** `src/filter/disabled.rs:96` (`None => clean || body_indented(inner)`), `src/filter/parse.rs:313-331`

**Issue:** When the block has no body line, `est` is `None`. A line with a problem then counts as a
body line whenever it is indented by a tab or two spaces. Iteration-2 WR-01 named
`#  Note: these are weapons` as prose that must stay with the next block. That is fixed only for
blocks that have a body. Reproduced:

```
Show\n#  Note: these are weapons\nHide\n        -> Show owned 0..2, Hide leading 2..2
Show # catch-all\n#  Rarity is irrelevant here\nHide\n -> Show owned 0..2
Show\n#\tSee notes below\nHide\n                -> Show owned 0..2
```

The heading moves with the wrong block when blocks are reordered (D-03). If that block is disabled
and enabled again, the line comes back as an invalid owned line, so the block cannot be enabled.
Header-only blocks are usually catch-alls (`Show # rest`), which is exactly where a heading for
the next section sits.

**Fix:** With no established body, accept only **clean** grammar lines:
`None => clean`. The one shape that needs the old leniency is the disabled form of a header
followed by a broken first body line, such as `Show\n\tFutureKw 3`. Iteration-2 design decision 3
already lists that shape as refused-to-disable, so the change costs nothing new. Add the three
inputs above to `trailing_headings_with_grammar_words_stay_with_next_block`.

## Info

### IN-01: `set_meta` and `remove_meta` report every failure as `WouldReattributeLines`

**File:** `src/filter/edit.rs:358-359`, `src/filter/edit.rs:388-389`
**Issue:** `.map_err(|_| EditError::WouldReattributeLines)` also turns a `Fail::Structure`
(header moved, or owned length changed) into a neighbour-re-attribution message. The message then
names the wrong cause.
**Fix:** Map `Fail::Structure` to `NotOneBlock` and `Fail::Neighbours` to `WouldReattributeLines`,
as `set_enabled` and `replace_block_lines` already do.

### IN-02: A stray `\r` is not whitespace to the tokenizer (carried over, still open)

**File:** `src/filter/tokenize.rs:69-71`
**Issue:** `is_ws` accepts only space and tab, so `\r\r\n` files produce false Error diagnostics.
**Fix:** Treat `\r` as whitespace in `is_ws`.

### IN-03: Unreachable `Class::Comment` arms (carried over, still open)

**File:** `src/filter/parse.rs:139`, `src/filter/parse.rs:265`
**Issue:** `is_comment_line` diverts comment lines before `classify` runs, so these arms are dead.
**Fix:** Drop the variant from the arms, or use `unreachable!` with a comment.

### IN-04: Inconsistent severity for disabled-block reasons (carried over, still open)

**File:** `src/filter/disabled.rs:236-240`, `src/filter/parse.rs:101-117`
**Issue:** `DisabledState::Invalid` and `EditError::CannotEnable` carry `Warning`, while
`diagnostics()` reports the same lines as `Info`. The new extent-mismatch reason (parse.rs:101-105)
adds one more `Warning`.
**Fix:** Use one severity for both.

### IN-05: `.gitattributes` narrows `*.filter -text` to the parser fixtures (carried over, still open)

**File:** `.gitattributes:5-9`
**Issue:** D-14 says `*.filter -text`. With `core.autocrlf=true`, `tests/fixtures/seed-universal.filter`
can still be renormalized on checkout, and it is the byte oracle for `artifact_guard`.
**Fix:** Add `tests/fixtures/*.filter -text`, or record the deviation in CONTEXT.md.

### IN-06: Section markers directly above a block become block metadata (carried over, still open)

**File:** `src/filter/mod.rs:76-84`
**Issue:** `block_meta` returns key-only markers such as `#ff:economy-begin` as settings of the block
below them.
**Fix:** Filter out the reserved marker keys, or return only lines that have a value.

### IN-07: `Show1` / `Show_x` tokenize as a `Show` header (carried over, still open)

**File:** `src/filter/tokenize.rs:100-110`
**Issue:** The keyword is the alphabetic run, so `Show1` reads as `Show` plus the value `1`, with
only a warning.
**Fix:** Require whitespace, an operator, `#`, `"` or end of line after the keyword.

---

_Reviewed: 2026-10-12T23:13:47Z_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
