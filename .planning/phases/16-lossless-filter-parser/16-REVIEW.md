---
phase: 16-lossless-filter-parser
reviewed: 2026-10-12T22:32:37Z
depth: standard
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
  warning: 6
  info: 6
  total: 13
status: issues_found
---

# Phase 16: Code Review Report

**Reviewed:** 2026-10-12T22:32:37Z
**Depth:** standard
**Files Reviewed:** 13
**Status:** issues_found

## Summary

I reviewed the lossless line model, tokenizer, keyword table, block grouping, disabled-block classification, `#ff:` metadata and the edit API. The byte-identity layer holds up. `lines::split` and `to_bytes` reproduce LF, CRLF, mixed endings, lone CR, BOM and a missing final newline exactly. The tokenizer only slices on ASCII delimiters, so it never splits a multi-byte character. I found no panics on arbitrary UTF-8 input. Every edit validates before it mutates, so bytes outside the edited block do not change.

The problems are in what the edit API accepts and how it classifies lines. I confirmed every behavior below by running a scratch probe crate against the library:

- **Enabling a disabled block can produce invalid grammar.** Prose comment lines inside a "Valid" disabled block become live unknown-keyword lines. This breaks the locked rule "Nothing the tool produces may yield an invalid enabled block."
- **`can_enable()` is not reliable.** It returns `true` for blocks that `set_enabled(true)` then refuses.
- **Disabling fails for several ordinary block shapes.** `set_enabled(false)` returns `NotOneBlock` for them. That blocks the remedy the phase context names for broken blocks: "fixed or disabled".
- **Some comment lines are attached to the wrong block.** Later reordering would carry them along with the wrong rule.

A full sweep of the FilterBlade fixture passed: all 460 enabled blocks disable and re-enable byte-identically, and all 51 disabled blocks enable cleanly. So these defects come from inputs the corpus does not contain, not from the fixture.

## Critical Issues

### CR-01: Enabling a "Valid" disabled block turns prose comments into live unknown-keyword lines

**File:** `src/filter/disabled.rs:70-71`, `src/filter/edit.rs:101-110`, `src/filter/edit.rs:139-153`
**Issue:** `disabled::detect` treats a `#` line whose first word is not in the keyword table as prose. It does not add the line to `body` and does not record a reason, but the line stays inside the block's owned range when later grammar lines follow it. The block is therefore classified `DisabledState::Valid`. `set_enabled(true)` strips one `#` from every owned line, so `# note here` becomes ` note here`. The re-parse reports that line only as a `Severity::Warning` ("unknown keyword `note` kept as-is"). `commit` rejects only `Severity::Error` (`edit.rs:104`), so the edit succeeds and writes an enabled block that the game will refuse to load.

Probe (confirmed):
```
parse("#Show\n#\tRarity Rare\n# note here\n#\tClass \"Bows\"\n")
can_enable() == true
set_enabled(id, true) == Ok(())
-> "Show\n\tRarity Rare\n note here\n\tClass \"Bows\"\n"   // Warning: unknown keyword `note`
```
`replace_block_lines(id, ["#Show", "# hello world", "#\tRarity Rare"])` followed by `set_enabled(id, true)` produces the same broken output, ` hello world`.

This contradicts the locked decision: "Enabled rules must always be grammatically correct. Nothing the tool produces may yield an invalid enabled block."

**Fix:** Any `#`-prefixed owned line that is neither empty after uncommenting nor a pure comment (`##…`) nor a recognized grammar line should make the block un-enableable. Do this in `detect`:
```rust
None => {
    // Prose inside the owned range cannot be enabled as grammar.
    reasons.push(reason(format!("`{}` is not filter grammar", &inner[r.clone()])));
}
```
That alone makes trailing prose (after the last keyword line) un-enableable too. To avoid that, record prose lines in a pending list and turn them into reasons only once a later keyword line extends `end` past them. As defense in depth, have `commit` with `reject_errors` also reject `Warning` diagnostics inside the edited span when enabling.

## Warnings

### WR-01: `can_enable()` returns true for blocks that `set_enabled(true)` then refuses

**File:** `src/filter/disabled.rs:77`, `src/filter/disabled.rs:91-108`, `src/filter/block.rs:70-72`
**Issue:** Two kinds of line are skipped silently instead of becoming reasons:
- A tokenizer failure whose first word is not a known keyword, such as FilterBlade-style `# $tag` or `#\t123`.
- A commented `Import` directive.

When the block is enabled, the first becomes a live `Error` line and the second ends the live block early. The block stays `Valid`, but enabling fails:
```
"#Show\n#\tRarity Rare\n# $tag\n#\tClass \"Bows\"\n"       -> can_enable true, enable -> Err(CannotEnable{"expected keyword"})
"#Show\n#\tRarity Rare\n#Import \"x\"\n#\tClass \"Bows\"\n" -> can_enable true, enable -> Err(NotOneBlock)
```
Later phases (the overlay toggle) will rely on `can_enable()` to show or grey out the control. A predicate that lies gives a user-facing error for an action the UI offered.
**Fix:** In `detect`, push a reason for directive lines (`Some(kw) if kw.kind == Kind::Directive`) and for tokenizer errors that sit between keyword lines. That is the same deferred-reason approach as CR-01.

### WR-02: Disabling fails with `NotOneBlock` for several ordinary live blocks

**File:** `src/filter/edit.rs:118-159`, `src/filter/disabled.rs:63-111`
**Issue:** `set_enabled(false)` prefixes `#` to each owned line. `commit` then requires the re-parsed disabled block to have exactly the same `owned_len`. Disabled detection has different extent rules from live detection, so the check fails for:
- A last body line with an unknown keyword (`\tFutureKw 3`) or a non-keyword bad line (`\t123`). Disabled detection drops these as prose, so `end` shrinks.
- A compact header with trailing tokens (`Show Continue`). `detect_header` requires no values.
- A commented-out condition directly after the last keyword line (`#\tSetFontSize 40`). Disabled detection absorbs it, so `end` grows.
- A following leading comment that starts with a grammar word (`# Class weapons below`). It is absorbed for the same reason.

All five cases were confirmed to return `Err(NotOneBlock)`. The phase context names disabling as the remedy for an enabled block that fails to parse ("Apply is refused until it is fixed or disabled"). Exactly those blocks, the ones with a bad or unknown line, often cannot be disabled.
**Fix:** Make disabled extent match the live extent when the block was produced by `set_enabled(false)`. For example, compute the new disabled block's `end` as the old live `end` and store it explicitly, instead of re-deriving it from keyword lines. Alternatively, have `detect` count every `#`-prefixed line that is non-prose (including unknown words and bad tokens) up to the next blank, non-comment or header line, and mark such blocks `Invalid`. At minimum, return a dedicated error, not `NotOneBlock`, so callers can explain why.

### WR-03: Comment lines are attached to the wrong block

**File:** `src/filter/parse.rs:67-73`, `src/filter/parse.rs:225-231`, `src/filter/disabled.rs:78-88`
**Issue:**
1. A live block's own trailing commented-out line becomes the next block's leading comment. The live `end` stops at the last keyword line, and `lead_start` then claims every directly-adjacent comment line. Probe: `"Show\n\tRarity Rare\n#\tSetFontSize 40\nHide\n"` gives Show span `0..2` and Hide span `2..4`.
2. A disabled block absorbs a following prose comment that happens to start with a grammar word. `# Class weapons below` tokenizes as `Class` with values `weapons below`, which passes the lenient `QuotedList` check. Probe: `"#Show\n#\tRarity Rare\n# Class weapons below\nHide\n"` gives the disabled Show owning `0..3`. Enabling it would emit `Class weapons below` as a live condition.

The context's comment-attachment rule was meant to make comments move with their rule on reorder (Phases 18-20). These cases move them with the wrong rule, or turn prose into a live condition.
**Fix:**
- For (1), when the comment run directly after a live block's last keyword line consists of `#`-prefixed grammar lines (`uncomment_once` plus a known keyword), give them to the preceding block.
- For (2), require disabled-body lines to look like grammar the way the header does: indentation after the `#` (as `disable` itself produces, since `#\t…` is always indented when the source was), or reject unquoted multi-word values for `QuotedList` keywords inside disabled bodies.

### WR-04: `replace_block_lines` accepts enabled blocks that carry Warning-level grammar problems

**File:** `src/filter/edit.rs:200-211`
**Issue:** Candidate validation rejects only `Severity::Error`. An enabled replacement such as `["Show", "\tFooBar 1"]` (unknown keyword) or `["Show Rarity Rare"]` (trailing tokens after the header) is accepted and written. Both are Warnings at parse time, but both are content the tool itself produced. The no-invalid-enabled-blocks rule applies here as well. If carrying an unknown keyword through from a loaded file is intended, allow only lines that byte-match an existing opaque line of the same block.
**Fix:**
```rust
.filter(|d| matches!(d.severity, Severity::Error | Severity::Warning))
```
or an explicit allow-list of preserved opaque lines.

### WR-05: `set_meta` value does not round-trip, and there is no way to clear a key

**File:** `src/filter/edit.rs:248`, `src/filter/meta.rs:29-30`
**Issue:** `set_meta(id, "name", "  padded ")` writes `#ff:name   padded ` and `meta_get` returns `"padded"`. `set_meta(id, "name", "")` writes `#ff:name ` with a trailing space, and `meta_get` returns `None`. So a written value is not what is read back, an "empty" setting leaves a stale line with trailing whitespace in the file, and there is no removal operation. Phase 20 will set names from user text fields, so leading or trailing spaces are likely.
**Fix:** Reject or trim leading and trailing whitespace in `set_meta`, and either reject an empty value or add a `remove_meta(id, key)` that deletes the line:
```rust
let value = value.trim_matches([' ', '\t']);
if value.is_empty() { return Err(EditError::InvalidMeta("empty value; use remove_meta".into())); }
```

### WR-06: The disable/enable fixture test silently tolerates failures

**File:** `tests/filter_roundtrip.rs:256-263`
**Issue:** `if doc.set_enabled(id, false).is_ok()` skips any block that fails to disable, and the test only asserts `done >= 5` out of about 9 attempts. A regression that broke disabling for half the blocks would still pass. Today all 460 enabled blocks in the fixture disable and re-enable cleanly (verified by a full sweep), so the test can be strict. There is also no test for enabling a disabled block that contains prose (CR-01) or a `$tag` line (WR-01).
**Fix:** Iterate every enabled block and assert both `set_enabled` calls succeed:
```rust
for id in enabled_ids(&doc) {
    doc.set_enabled(id, false).unwrap();
    doc.set_enabled(id, true).unwrap();
}
assert_bytes_eq("disable/enable", &doc.to_bytes(), &orig);
```
Add unit tests for the CR-01 and WR-01 inputs.

## Info

### IN-01: A stray `\r` is not whitespace to the tokenizer, causing false Errors on `\r\r\n` files

**File:** `src/filter/tokenize.rs:69-71`
**Issue:** `\r\r\n` endings come from a double CRLF conversion. They leave `\r` at the end of the line text, which becomes part of the last value. `"\tRarity Rare\r"` gives the Error "`Rarity` expects Normal, Magic, Rare or Unique", and `"Show\r"` gives a trailing-token warning. Bytes are preserved correctly, but later phases will refuse Apply for such files.
**Fix:** Treat `\r` as whitespace in `is_ws` (only for tokenizing; stored text stays untouched), or emit a dedicated diagnostic for a stray CR.

### IN-02: Unreachable `Class::Comment` arms

**File:** `src/filter/parse.rs:119`, `src/filter/parse.rs:233`
**Issue:** Both callers check `is_comment_line` before calling `classify`, so `classify` can never return `Class::Comment` there. These are dead arms.
**Fix:** Drop the `Comment` variant, or route comment detection through `classify` only.

### IN-03: Inconsistent severity for disabled-block reasons

**File:** `src/filter/disabled.rs:58-62`, `src/filter/parse.rs:90-97`
**Issue:** Reasons stored in `DisabledState::Invalid` are `Severity::Warning`, but the same problems appear in `Document::diagnostics()` as `Severity::Info`. Consumers that filter by severity will see different answers depending on where they look.
**Fix:** Use one severity for both.

### IN-04: `.gitattributes` narrows the context's `*.filter -text` to the parser fixtures only

**File:** `.gitattributes:5`, `.gitattributes:9`
**Issue:** The context asked for `*.filter -text`. The implementation keeps `*.filter text` and exempts only `tests/fixtures/parser/*.filter`. `tests/fixtures/seed-universal.filter` is a round-trip corpus file, but its checked-out line endings still depend on `core.autocrlf`. The test compares the file against itself, so it still passes, but the fixture does not have the stable byte content the other fixtures have. This is likely deliberate, to keep `artifact_guard` untouched, but it should be documented.
**Fix:** Add a comment explaining why the scope is narrower than the context.

### IN-05: Section markers placed directly above a block become block metadata

**File:** `src/filter/mod.rs:76-92`
**Issue:** `block_meta` and `meta_get` read every `#ff:` line in the leading comment run. A `#ff:economy-begin` marker with no blank line before the first economy block becomes that block's metadata (`("economy-begin", "")`). Phase 18 needs to know this, or separate block keys from section markers.
**Fix:** Exclude marker-only keys (lines with no value), or reserve a key namespace for section markers.

### IN-06: The header tokenizer accepts `Show1` / `Show_x` as a `Show` header with only a warning

**File:** `src/filter/tokenize.rs:100-110`
**Issue:** The keyword scan stops at the first non-alphabetic byte, so `Show1` is parsed as `Show` with value `1` and flagged only as "trailing tokens" (Warning). The game most likely rejects this line. Separately, a single-word comment `# Show` is classified as a disabled empty block, which is acceptable but undocumented.
**Fix:** Require the keyword to end at whitespace, an operator, `#`, `"` or end of line. Otherwise emit an `Error` for an unknown keyword.

---

_Reviewed: 2026-10-12T22:32:37Z_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
