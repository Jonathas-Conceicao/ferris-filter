# Phase 45: Economy Tier-Range Names - Context

**Gathered:** 2026-10-23
**Status:** Ready for planning
**Mode:** Smart discuss (autonomous run 43-50; author answered every question up front in prose)

<domain>
## Phase Boundary

The economy section's `# ff:name` lines state each tier's price range. Requirements ENAME-01..03.
Only the game-folder economy section changes; `generate` output (which has no `# ff:name`) and its
fixtures do not.

</domain>

<decisions>
## Implementation Decisions

### Name format (author)
- `{T} tier {Group} < {next tier floor} {unit}` for every tier except the top one, e.g.
  `E tier Essences < 2 chaos`, `F tier Currency < 0.5 chaos`.
- The top tier states its floor with ASCII `>=`: `S tier Currency >= 30 chaos`.
- The range comes from the tier cutoffs (`tiers.json` / the fallback pair), never from item prices.
- Unit: the tiers' own unit word, `chaos` or `exalt`, never pluralised.
- Numbers: at most 2 decimals, trailing zeros trimmed (`0.5`, `2`, `12`, `0.15`).
- Price-tiered unique blocks and the gem categories with a background override get the same
  treatment (`A tier Uniques < 30 chaos`).

### Behaviour
- Pending tier edits on the Economy tab rename the staged / previewed economy rows on the RULES tab,
  as they already re-price them.
- The first Apply after upgrading rewrites every economy name once — expected.

### Claude's Discretion
- Where the label is computed (research: a `tier_labels` map from `pricer`, used only on the tagged
  economy-section path) and the formatter's shape.

</decisions>

<code_context>
## Existing Code Insights

### Reusable Assets
- `src/emitter.rs` writes `# ff:name {tier} tier {category}` (two sites, ~554 and ~632).
- `src/pricer.rs` tier assignment; `compile::load_economy_with` used by the pending preview.

### Established Patterns
- Names are pinned in tests: `emitter.rs`, `dynamic.rs`, `rules.rs`; `app.rs` checks
  `.contains(" tier ")` — re-grep and update all readers.

### Integration Points
- Economy section render (Apply and the read-only preview), RULES search (finds names).

</code_context>

<specifics>
## Specific Ideas

None beyond the format above.

</specifics>

<deferred>
## Deferred Ideas

None — discussion stayed within phase scope.

</deferred>
