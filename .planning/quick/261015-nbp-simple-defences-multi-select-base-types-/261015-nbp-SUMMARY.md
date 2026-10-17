---
quick_id: 261015-nbp
status: complete
date: 2026-10-15
commits: 65a3c67..52ff64e
---

# Quick 261015-nbp: Simple Defences as multi-select base types, written as grouped blocks

## What was built

- **`src/filter/group.rs` (new):** the seven combos (chip order), `cover(sel)`. That is the minimum
  exact cover by product-set constraints, with fewer lines on ties and a fixed order. It is precomputed
  for all 128 selections and exhaustively tested against an independent BFS: every selection is exact
  and minimal, and the maximum is 4 blocks. Also `# ff:group <id> <k>/<n>` parsing, `analyze()`
  (Leader / Follower / Broken), `resync`, `set_selection`, `duplicate` and `fresh_gid`.
- **`src/filter/structure.rs`:** `replace_blocks_after` (drop or insert blocks after an anchor, ids
  reused by position, CRLF and missing final newline kept) and `swap_runs` (`move_block` now
  delegates to it).
- **`WorkingFilter`:** a valid group is one row (`group_size`, `group_broken`, `simple_mask`), and its
  summary reads `Defences: ...`. Every action acts on the whole group. Enable/Delete/Duplicate/Move are
  group-aware, and other edits run on the leader and then re-sync the followers (each follower keeps
  its own defence lines verbatim). The new `RowAction::SetDefenceSelection` re-covers the rule. Every
  group op is atomic through a snapshot. Blocking errors and the style report count a group once, with
  row numbers.
- **UI:** one wrapped line of seven `toggle_chip`s, each with a tooltip. The X sends selection 0. A
  pick of all chips or none writes no line and keeps the card open (`ListUi.simple_all`). Broken groups
  show the `group broken` flag.
- **Docs:** CLAUDE.md (dynamic.filter metadata list, rule list = rules not blocks, Simple Defences
  paragraph) and the CHANGELOG `[Unreleased]` section.

## Verification

- 1069 lib tests plus integration tests all pass, including `artifact_guard` and the snapshot tests.
  The fixtures are untouched.
- `cargo fmt` is clean. Clippy shows only the pre-existing `result_large_err` warnings.

## Review (`261015-nbp-REVIEW.md`) and the fix pass

- **Fixed:**
  - WR-01: a group-breaking edit is now refused.
  - WR-02: a follower id is refused.
  - WR-03: `simple_all` is cleared when the row collapses.
  - WR-04: the style passes use a precomputed rule-index map.
  - WR-05: the module doc is corrected.
  - IN-05: the legacy `#ff:` spelling no longer breaks a group.
- **CR-01, kept as designed:** blocks whose defence lines are only `0` also match items that have no
  defences, because the approved brief ignores the "no defence" combination ("No EV" is one block,
  `BaseEvasion 0`). CLAUDE.md now says to pair the card with a Class condition.
- **Open:**
  - WR-06: a move at the end of the file can drop a blank line or the final newline. This was
    already true of `move_block`.
  - IN-01: header comments are copied into every block.
  - IN-02: the first chip click normalizes the spelling and trailing comments of the defence lines.
  - IN-03: the card bookkeeping is updated before the write result is known.
  - IN-04: a chip click and line edits in the same frame give stale-line refusals.

## Decisions beyond the brief

- "Full AR" and similar chips use "Any" for the other types when possible, for example
  `[Any, No, No]`. That follows the brief's own "non-empty subsets" rule.
- The leader always takes cover part 1. A block that already reads as its part keeps its bytes.
- No chip and all chips both write no line. The card remembers "all" until reload or collapse.
- A copy of a block whose group is broken loses its group line.
- An edit on a grouped rule that would break the group is refused, with the message "change its
  defences with the Simple Defences chips".
- A broken group's blocks are also excluded from any grouping.

See `261015-nbp-MANUAL-CHECKS.md` for the author's manual checklist.
