---
phase: 16-lossless-filter-parser
verified: 2026-10-12T00:00:00Z
status: passed
score: 4/4 must-haves verified (criterion 2 has one residual human confirmation)
overrides_applied: 0
human_verification:
  - test: "Compare src/filter/keywords.rs KEYWORDS against the live pathofexile.com/item-filter/about page (block types, conditions, actions)"
    expected: "Every keyword on the page has a table entry. The table was built from RESEARCH, because the page could not be fetched during execution."
    why_human: "The authoritative grammar source is not reachable offline. Code can only prove the table is internally consistent and covers every keyword in the real filters."
---

# Phase 16: Lossless Filter Parser Verification Report

**Phase Goal:** Any working PoE2 filter can be parsed and written back without changing a byte the user did not edit.
**Status:** human_needed (all automated checks pass; one grammar-completeness confirmation remains)
**Re-verification:** No, initial verification. The final review-fix iteration (iteration 3) was not re-reviewed, so I weighed it by exercising the code directly.

## Goal Achievement

### Observable Truths (ROADMAP success criteria)

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | Real FilterBlade/NeverSink, reference-old-filters, poe2filter.com exports and FerrisFilter output all open without error; unknown keyword stays opaque | VERIFIED | I ran my own throwaway harness (a crate in the scratchpad that depends on `ferrisfilter`) over 25 files. These were: the 10 `reference-old-filters/*.filter`, the 3 in-game folder filters (including the real FilterBlade file, 511 blocks), the 4 repo-root generated filters, the SSF/scratch filters, and the test fixtures. Every file parsed with `LoadError` never raised and 0 Error and 0 Warning diagnostics. `corpus_unknown_keyword_kept_opaque` shows that `FutureKeyword 3` gives one Warning and the block is retained. The only load failure is non-UTF-8, which is a typed `LoadError`. |
| 2 | Every documented block type, condition, operator and action is recognized, including Continue, Import, Minimal | VERIFIED in code, one human confirmation open | `KEYWORDS` in `src/filter/keywords.rs` has `Show`/`Hide`/`Minimal` (BlockHeader), `Continue` (Modifier) and `Import` (Directive). It also has about 60 conditions and 17 actions, and all eight operators are handled in the tokenizer. The tests assert that every keyword on a live line of the FilterBlade fixture is in the table. The official page was not reachable at execution time, so completeness against it is not independently confirmed (see human item). |
| 3 | No-edit save is byte-identical for CRLF, LF, mixed, BOM, no final newline, and the real FilterBlade sample | VERIFIED | `roundtrip_filterblade_fixture` asserts 258,094 bytes of CRLF. `git ls-files --eol` shows `i/crlf w/crlf attr/-text`, so the scoped `.gitattributes` rule works and the fixture is not renormalized. `roundtrip_synthetic_cases` covers LF, CRLF, mixed, BOM+CRLF, no final newline, only-BOM, empty, lone CR and tabs/trailing whitespace. My harness confirmed `to_bytes() == input` for all 25 files. |
| 4 | Editing one block changes only that block's lines | VERIFIED | `edit_replace_one_block_filterblade` replaces block #99 of the real FilterBlade file and asserts the prefix and suffix are identical to the original byte for byte. The region uses CRLF. The tests also cover disabling and re-enabling all 460 enabled blocks with an exact byte restore, CRLF preservation, no-final-newline edits, stable BlockIds, and a no-panic sweep over prefixes. My harness disabled and re-enabled one block in every file, restoring the bytes exactly. |

**Score:** 4/4

### Required Artifacts

| Artifact | Status | Details |
|----------|--------|---------|
| `src/filter/{lines,mod,keywords,tokenize,parse,block,disabled,meta,diag,edit}.rs` | VERIFIED | All are substantive, 3,540 lines in total. `parse()` and `Document` are exported from `lib.rs` and used by `tests/filter_roundtrip.rs`. |
| `tests/fixtures/parser/filterblade.filter` | VERIFIED | 258,094 bytes, begins with the NeverSink header. |
| `.gitattributes` (`tests/fixtures/parser/*.filter -text`) | VERIFIED | Present. |
| `tests/filter_roundtrip.rs` | VERIFIED | 18 tests, all passing. |

### Key Links

The lines are the source of truth and `to_bytes()` writes them directly. The parse layer builds items over the lines without mutating them. Edits go through `edit.rs`, which rewrites only the block's line range. All of this was confirmed by the byte-offset test.

### Requirements Coverage

| Requirement | Source plans | Status | Evidence |
|-------------|--------------|--------|----------|
| PARSE-01 | 16-02, 16-03, 16-04 | SATISFIED | Truth 1. |
| PARSE-02 | 16-02, 16-03 | SATISFIED (see human item) | Truth 2. |
| PARSE-03 | 16-01, 16-03, 16-04 | SATISFIED | Truth 3. |
| PARSE-04 | 16-04 | SATISFIED | Truth 4. |

All four IDs appear in PLAN frontmatter and in REQUIREMENTS.md, and there are no orphaned IDs. REQUIREMENTS.md has the checkboxes ticked, but the traceability table (lines 147-150) still says "Pending". This is bookkeeping only.

### Behavioral Spot-Checks

| Behavior | Command | Result |
|----------|---------|--------|
| Filter unit tests | `cargo test filter` | 102 passed, 0 failed |
| Roundtrip, corpus and edit integration tests | `cargo test --test filter_roundtrip` | 18 passed, 0 failed |
| Independent corpus run (25 real files) | scratchpad harness | rt=true, 0 errors, 0 warnings, disable/enable restores bytes, for all 25 |

Whole-suite status of 480 passed was reported by the caller. I re-ran only the filter-scoped tests.

### Anti-Patterns

There are no TODO, FIXME, XXX or TBD markers in `src/filter`. No stubs were found.

### Review-fix weighting

Iteration 3 introduced a `CannotDisable` refusal. It applies when disabling would produce a rule with a cut-off condition, and it also applies to a live block whose first body line is unknown or untokenizable. The documented cost is that such blocks cannot be disabled, but their bytes are untouched. That is consistent with PARSE-04 and does not breach the lossless contract. The real-corpus run shows no refusals: all 460 blocks in the FilterBlade fixture disable and re-enable cleanly, and the same holds for the other files I tried.

### Human Verification Required

1. **Grammar completeness against the official page.** Open pathofexile.com/item-filter/about and diff its keyword list against `KEYWORDS`. If a keyword is missing it still loads, as an opaque line with a Warning, so no byte is lost. Only criterion 2's recognition claim would be affected.

### Gaps Summary

No blocking gaps. The phase's lossless and non-failing-load guarantees are demonstrated against real files, not just fixtures. One human confirmation is outstanding for keyword-table completeness (criterion 2).

---

_Verified: 2026-10-12_
_Verifier: Claude (gsd-verifier)_

## Orchestrator Note: Keyword Table vs. Live Grammar Page (2026-10-12)

After verification, the orchestrator fetched https://www.pathofexile.com/item-filter/about and checked every keyword on the page against `src/filter/keywords.rs`:

- 5 block types: Show, Hide, Minimal, Continue, Import
- 61 conditions, from AlternateQuality to ZanaMemory
- 14 actions, from PlayAlertSound to SetTextColor

All 80 have a quoted entry in the table, and none is missing. The table also has exactly 80 entries.

This is strong automated evidence for criterion 2. The status stays `human_needed` only because sign-off belongs to the author: approve the UAT item to close it.

## Human UAT outcome (2026-10-15)

UAT complete (v0.5 wrap-up). Partial pass accepted: the keyword table and PoE1-only raw rows not yet confirmed in game moved to backlog 999.12. Status changed from `human_needed` to `passed`.
