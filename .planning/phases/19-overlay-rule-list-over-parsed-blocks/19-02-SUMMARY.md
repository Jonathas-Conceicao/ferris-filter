---
phase: 19-overlay-rule-list-over-parsed-blocks
plan: 02
subsystem: filter
tags: [filter, document, structural-edits, economy]
requires: ["19-01"]
provides:
  - "Document::{append_rule, remove_block, duplicate_block, move_neighbour, move_block, set_block_keyword, set_continue}"
  - "EditError::{NoNeighbour, EconomyDamaged, CompactHeader}"
affects: [overlay]
tech-stack:
  added: []
  patterns:
    - "commit_structural: caller states the id and expected span text of every block in the new order; the rebuilt index is refused on any mismatch"
    - "block unit = span + following blank-only lines; moves swap units and leave lines between them in place (self-inverse)"
key-files:
  created: [src/filter/structure.rs, tests/filter_structure.rs]
  modified: [src/filter/edit.rs, src/filter/mod.rs]
decisions:
  - "Ids follow blocks: commit_structural assigns existing ids verbatim and fresh ids from next_id; edit::commit (by-position remap) is not reused"
  - "move_neighbour returns None for economy blocks and across marker lines, so moves never touch the economy section"
  - "set_block_keyword / set_continue are thin replace_block_lines wrappers; compact headers are refused with CompactHeader"
metrics:
  completed: 2026-10-13
requirements-completed: [EDIT-03, EDIT-04, EDIT-05, ECON-02]
---

# Phase 19 Plan 02: Structural Document Edits Summary

`filter::Document` now supports append-before-economy, remove, duplicate, move up/down, Show/Hide/Minimal switching and Continue toggling, all validate-then-commit, id-preserving, economy-safe and self-inverse on the FilterBlade fixture.

## Commits
- 8f6459a: feat(19-02): structural block ops on filter::Document
- 337dcf8: feat(19-02): keyword and Continue edits plus fixture-level structural proofs

## Verification
- `cargo test --lib filter::`: 143+ tests pass (18 new in `structure.rs`); `tests/filter_structure.rs` has 8 tests; full `cargo test` green (442 lib tests); `cargo fmt --check` clean.
- Fixtures and snapshots unchanged (`git diff --exit-code tests/fixtures tests/snapshots`).
- On FilterBlade (about 20 evenly spaced blocks): move/inverse, duplicate/remove and keyword/back are byte-exact; Continue add/remove returns to the original bytes when the block had no Continue (a pre-existing Continue may be re-added at the end of the block).
- On the migrated v0.4 fixture: append lands above the economy section, economy lines are byte-identical, and the last rule cannot move down.

## Deviations from Plan

1. **[Rule 1 - edge case] Blank last line after moving an unterminated final block.** The plan says the new last line gets `Eol::None`. When the new last line is a blank line (e.g. `A\n\nB` with B moved up, giving `B\nA\n\n`) a blank cannot be unterminated (the parser never produces one), so it takes the document terminator. The inverse move then ends terminated. When the new last line is non-blank the plan's rule holds and the move is exactly self-inverse. Tested in `move_unterminated_final_block`.
2. **Duplicate/remove inverse** is exact when the source is followed by a blank line (the normal layout, and every pick in the FilterBlade test). A source with no trailing blank gets a separating blank before its copy, so removing the copy leaves that blank behind (per the plan text).
3. **Append candidate span.** `append_rule` accepts lines whose block span covers every line (leading `#ff:` or comment lines allowed, as the overlay will want to write `#ff:name`), not only header-first lines.

## Known Stubs
None.

## Self-Check: PASSED
- src/filter/structure.rs, tests/filter_structure.rs exist; commits 8f6459a and 337dcf8 present.
