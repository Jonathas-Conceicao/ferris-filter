---
phase: 16-lossless-filter-parser
reviewed: 2026-10-12T00:00:00Z
depth: standard
iteration: 2
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
  critical: 0
  warning: 4
  info: 7
  total: 11
status: issues_found
---

# Phase 16: Code Review Report (iteration 2)

**Reviewed:** 2026-10-12
**Depth:** standard
**Files Reviewed:** 13
**Status:** issues_found

## Summary

This pass re-checked the seven iteration-1 fixes (CR-01 and WR-01 to WR-06) and probed for
regressions. I used a scratch crate that path-depends on the library, plus `cargo test`, which
passes: 84 `filter::` unit tests and 18 `filter_roundtrip` tests.

**Prior findings:** All seven are fixed as described.
- **CR-01:** interior prose now makes a disabled block `Invalid`.
- **WR-01:** the `$tag`, `#\t123`, `#Import` and `#Show Continue` shapes now agree between
  `can_enable()` and `set_enabled(true)`.
- **WR-02:** body-indented bad or unknown last lines can now be disabled.
- **WR-03:** a trailing commented condition now moves with its block.
- **WR-04:** enabled replacements are refused on Warning-level problems.
- **WR-05:** `set_meta` trims its value, and `remove_meta` exists.
- **WR-06:** the fixture sweeps are strict.

**D-11 invariant:** I found no path that writes an invalid enabled block. `set_enabled(true)`
re-checks Error and Warning diagnostics inside the block's span. `replace_block_lines` checks the
enabled candidate in isolation. The line set is identical in context, so the diagnostics match.

**Byte identity:** this holds on every filter on this machine: 25 files, including
`reference-old-filters/*`, the root `ssf_*` and `*-leaguestart` filters, and the game-folder
filters. A full disable/enable or enable/disable sweep over every block in those files was
byte-identical with zero failures. No real file has an `Invalid` disabled block.

**New problems:** The WR-03 fix introduced new misclassifications. `classify_inner` returns `Body`
for any line whose first word is a grammar keyword, even when the rest of the line fails the shape
check. It also returns `Body` for any unknown word indented by two spaces. That over-matches
ordinary prose. As a result:
- the live trailing run takes the next block's leading comments;
- valid disabled rules become un-enableable;
- `#ff:` metadata can be pulled into the previous block.

Separately, `commit` only checks the edited block, so an edit can silently re-attribute lines
between *other* blocks. None of this changes bytes or produces an invalid enabled block, so there
is no Critical. All four warnings are correctness defects against D-03, D-05 and D-09, each
reproduced with a short input.

## Warnings

### WR-01: Prose that starts with a grammar word (or is indented by two spaces) is classified as a disabled body line, so it is stolen from the next block and makes valid disabled rules un-enableable

**File:** `src/filter/disabled.rs:112-122`, `src/filter/disabled.rs:145-152`, `src/filter/disabled.rs:88-97`, `src/filter/disabled.rs:159-164`, `src/filter/parse.rs:278-280`

**Issue:** `classify_inner` returns `Inner::Body` in three cases:
- a known keyword whose values fail `validate_shape` (it attaches `problem: Some(..)`);
- any line that fails to tokenize but starts with a condition or action word;
- any unknown word after `#` followed by two spaces or a tab.

Only the multi-value `QuotedList` shape is excluded as prose (`prose_shaped`). Ordinary English
comments therefore count as grammar. Two code paths treat that `Body` as "extends the block":
- `is_commented_body`, used by the new WR-03 trailing run in `scan_live_body`;
- `detect`, where `Body` pulls the line, and any pending prose before it, inside the range.

Reproduced:

```
Show\n\tRarity Rare\n# Corrupted items\nShow\n\tCorrupted True\n
  -> block 0 owned 0..3; block 1 leading 3..3   (the comment above block 1 now belongs to block 0, D-03)

#Show\n#\tRarity Rare\n# Corrupted items\nShow\n
  -> block 0 Invalid: "`Corrupted` expects True or False"; set_enabled(true) -> CannotEnable   (D-09)
```

The same happens with these lines:
- `# Quality gear below`
- `# ItemLevel >= 80 only`
- `# Continue below`
- `# Sockets 3 is good`
- `#  Note: these are weapons`, with two spaces, read as an unknown keyword
- `#   [[0100]] Section`, with a body indent, read as a bad token

Before WR-03, the live case left these as the next block's leading comments. The disabled case
now reports a false "cannot be enabled" for a rule that is valid. No committed fixture or local
filter has this shape today, which is why the corpus tests pass.

**Fix:** A line that has a problem should extend a block only when it is unmistakably a disabled
body line. Concretely:
- If a known keyword line fails its shape check and has no body indent, return `Inner::Prose`.
- If tokenizing fails on a known word with no body indent, return `Inner::Prose`.
- Tighten `body_indented` for unknown or bad lines to require a tab, or to match the indentation
  of the block's own previous body line, instead of "two spaces".

```rust
Some(kw) => match validate_shape(kw, inner, &t) {
    Err(m) if !body_indented(inner) => Inner::Prose(m),
    r => Inner::Body { problem: r.err(), line: BodyLine { line: k, keyword: Some(kw), tokens: Some(t) } },
},
```

Separately, make `is_commented_body` (the live trailing run) accept only `Body { problem: None, .. }`.
A trailing commented line that would not be valid if enabled is far more likely to be a heading for
the next block than a disabled condition. Add regression tests for the `# Corrupted items` and
`#  Note:` shapes, covering both the live and the disabled case.

### WR-02: A `#ff:` metadata line is pulled into the preceding disabled block, so the next block loses its metadata and the disabled block becomes Invalid

**File:** `src/filter/disabled.rs:186-204`

**Issue:** In `detect`, a `#ff:name x` line is `Prose` (`ff` is not a keyword), so it goes into
`pending`. If any commented body line follows before the next header, `reasons.append(&mut pending)`
pulls the metadata line inside the disabled block. Reproduced:

```
#Show\n#\tRarity Rare\n#ff:name x\n#\tSetFontSize 40\nHide\n
  -> block 0 owned 0..4, Invalid ("`ff` is not filter grammar"); meta_get(Hide, "name") == None
```

By D-05, a `#ff:` line belongs to the header run of the block below it. It should never sit inside
another block. Today this only bites in combination with WR-01 or a commented condition placed above
a block. But Phases 18-20 will write `#ff:` lines above blocks programmatically, and `set_meta`
inserts them directly above the header. Any commented line the user keeps between the metadata and
the header (`#ff:name x` / `#\tSetFontSize 40` / `Hide`) silently detaches the name.

**Fix:** Treat a `#ff:` line as a terminator in `detect`, the same way a header is. The same
applies to the trailing run in `scan_live_body`, where it already stops because `#ff:` is `Prose`.
Make that explicit:

```rust
// in detect, before classify_inner:
if super::meta::parse_ff_line(text).is_some() {
    break;
}
```

Add a test that `meta_get` still finds the name in the reproduced input.

### WR-03: `commit` validates only the edited block, so an edit can silently move lines between *other* blocks

**File:** `src/filter/edit.rs:69-104` (used by `remove_meta` at `src/filter/edit.rs:289-316`, and by `set_meta`, `set_enabled` and `replace_block_lines`)

**Issue:** `commit` checks the block count and the edited block's `header` and owned length. It
never checks that every other block kept its span. The module doc (lines 6-8) promises "Only the
edited block's lines change" and stable identity by position. Line ownership is the model that
later phases move and reorder blocks by, and it can change under them. Reproduced with
`remove_meta`:

```
Show\n\tRarity Rare\n#ff:name x\n#\tSetFontSize 40\nHide\n
  before: Show owned 0..2, Hide leading 2..4
  remove_meta(Hide, "name") -> Ok
  after:  Show owned 0..3 (now owns `#\tSetFontSize 40`), Hide leading 3..3
```

The commented line moved from Hide to Show. A later "move Hide" would leave it behind, and a later
"disable Show" would double-comment it. The edit returned `Ok` with no signal. `set_enabled` and
`replace_block_lines` can shift neighbours in the same way, for example a trailing run that grows
or shrinks when the edited block's last line changes.

**Fix:** In `commit`, before re-parsing, record `(lead_start, header, end)` for every block other
than `pos`. Shift by `delta = new_len - old_len` the entries that lie after the edited region. For
`remove_meta`, which removes lines inside the edited block's leading run, the shift applies only to
blocks after it. Return `Fail::Structure` if any re-parsed block differs. `remove_meta` and
`set_meta` should then report a dedicated error rather than `NotOneBlock`.

### WR-04: Disabling an ordinary block fails when the next block's leading comments are prose followed by a commented condition (extent rules of `detect` and `scan_live_body` disagree)

**File:** `src/filter/disabled.rs:196-204` vs `src/filter/parse.rs:275-280`

**Issue:** The two scanners use different rules after the last body line:
- `scan_live_body` takes only a *contiguous* run of commented body lines after the last keyword
  line, so the first prose line stops it.
- `detect` keeps prose in `pending` and pulls it in when a later `#`-body line appears.

So the same lines get a different extent before and after disabling. Reproduced:

```
Show\n\tRarity Rare\n# note\n#\tClass "Bows"\nHide\n
  -> Show owned 0..2; set_enabled(Show, false) -> Err(CannotDisable)
```

The disabled form would own 0..4 and carry a reason, so `commit` rejects it. The same text in an
already-disabled file (`#Show\n#\tRarity Rare\n# note\n#\tClass "Bows"\nHide`) is classified
`Invalid`, even though the last two lines are the leading comments of `Hide`. Under WR-02 of
iteration 1, disabling is the remedy for a broken block, and here it fails for a perfectly valid one.

**Fix:** Make the two extent rules agree. The most consistent option keeps the CR-01 protection.
In `detect`, once a `Prose` line has been seen after `end`, stop treating later lines as part of
this block when the run between them and the next header or non-comment line is all comments.
That run is the next block's leading run, exactly as `lead_start` would compute it. In practice:
- end the scan at the first prose line that follows `end`;
- keep the "interior prose makes it Invalid" rule only for prose that sits *between* two body lines
  inside `end`.

Add `Show\n\tRarity Rare\n# note\n#\tClass "Bows"\nHide\n` to
`disable_blocks_with_bad_or_unknown_last_line`.

## Info

### IN-01: `can_enable()` is true but `set_enabled(true)` fails with `NotOneBlock` when a stray condition follows after a blank line

**File:** `src/filter/disabled.rs:166-219`, `src/filter/edit.rs:165-169`
**Issue:** In `#Show\n#\tRarity Rare\n\n\tClass "Bows"\n`, the disabled block is `Valid` (0..2).
Once enabled, `scan_live_body` scans across the blank line and takes the stray `\tClass` line, so
the extent changes and `commit` returns `NotOneBlock`. The file already carries an "outside any
block" warning for that line, so the impact is low. This is still a `can_enable()`/`set_enabled`
mismatch of the kind iteration-1 WR-01 set out to remove.
**Fix:** When `detect` sees a non-comment line that is not a header, also look ahead the way the
live scan would. If the enabled extent would differ, mark the block `Invalid` with a reason ("a
stray condition below would join this block when enabled"). Alternatively, return a dedicated
`CannotEnable` reason from `set_enabled` instead of `NotOneBlock`.

### IN-02: A stray `\r` is not whitespace to the tokenizer (carried over from iteration 1, IN-01, still open)

**File:** `src/filter/tokenize.rs:69-71`
**Issue:** `is_ws` accepts only space and tab, so `\r\r\n` files produce false Error diagnostics
(`Show\r` reads as a header with a trailing token).
**Fix:** Treat `\r` as whitespace in `is_ws`.

### IN-03: Unreachable `Class::Comment` arms (carried over, still open)

**File:** `src/filter/parse.rs:119`, `src/filter/parse.rs:234`
**Issue:** Comment lines are diverted by `is_comment_line` before `classify` runs, so these arms are
dead.
**Fix:** Remove the variant from the match arms, or add `unreachable!` with a comment.

### IN-04: Inconsistent severity for disabled-block reasons (carried over, still open)

**File:** `src/filter/disabled.rs:181-185`, `src/filter/parse.rs:91-97`
**Issue:** `DisabledState::Invalid` and `EditError::CannotEnable` carry the reasons as `Warning`,
while `Document::diagnostics()` reports the same lines as `Info`.
**Fix:** Pick one severity, `Info`, for both.

### IN-05: `.gitattributes` narrows the context's `*.filter -text` to the parser fixtures (carried over, still open)

**File:** `.gitattributes:5-9`
**Issue:** D-14 says `*.filter -text`. The file keeps `*.filter text` and exempts only
`tests/fixtures/parser/*.filter`. On a checkout with `core.autocrlf=true`, which is this machine's
setting, `tests/fixtures/seed-universal.filter` can still be renormalized. That file is the
byte-identity oracle for `artifact_guard` and `filter_roundtrip`.
**Fix:** Also mark `tests/fixtures/*.filter -text`, or record the deviation in CONTEXT.md.

### IN-06: Section markers placed directly above a block become block metadata (carried over, still open)

**File:** `src/filter/mod.rs:76-84`
**Issue:** `block_meta` returns every `#ff:` line in the leading run, including key-only section
markers such as `#ff:economy-begin`.
**Fix:** Filter out the marker keys reserved in Phase 18, or return only lines that have a value.

### IN-07: The header tokenizer accepts `Show1` / `Show_x` as a `Show` header with only a warning (carried over, still open)

**File:** `src/filter/tokenize.rs:100-110`
**Issue:** The keyword is the alphabetic run, so `Show1` tokenizes as `Show` plus the value `1`.
**Fix:** Require whitespace, an operator, `#`, `"` or end of line after the keyword. Otherwise
return "expected keyword".

---

_Reviewed: 2026-10-12_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
