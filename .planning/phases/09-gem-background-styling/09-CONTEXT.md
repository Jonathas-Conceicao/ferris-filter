# Phase 9: Gem Background Styling - Context

**Gathered:** 2026-06-24
**Status:** Ready for planning

<domain>
## Phase Boundary

Priced gem blocks in the generated filter carry a per-category **fixed background** while the
**value tier** still drives the text + border color — identical in mechanism to the
priced-unique bronze background. Scope is the pricer → emitter background-emission path only.
The `background_override` field already exists on `GemFilter` (mapping) and `CategoryBlock`
(pricer) from Phase 8; this phase wires it through to actual `SetBackgroundColor` emission.
The `corrupted` condition + exceptional gear is Phase 10.
</domain>

<decisions>
## Implementation Decisions

### Gem background emission
- **Mechanism = the priced-unique bronze pattern** (`emit_unique_sections`): clone the value
  tier's `Style`, set its `background` to `None`, call `emit_style_actions(clone)`, then append
  one explicit `SetBackgroundColor <R> <G> <B> 255`. Exactly one `SetBackgroundColor` line per block.
- **Text + Border = value tier color** (S=pink, A=purple, B=red, C=yellow, D=cyan, E/F=white) —
  UNCHANGED from normal tier styling. Only the background is overridden.
- **Background = per-category fixed color:** Uncut Gems `80 70 55`, Lineage Support Gems `10 150 150`.
- **Alpha = 255** (opaque) → emit `SetBackgroundColor 80 70 55 255` / `SetBackgroundColor 10 150 150 255`.
- **All tiers** a gem can fall into (S–F) get the per-category background — the background is a
  property of the category, independent of the value tier.
- **Only the two gem categories** get a background override. Verisium and all other currency
  categories keep the plain Dracula background (no override). Driven by `CategoryBlock.background_override`
  being `Some(...)` only for the gem categories (set in Phase 8 from `mapping.json`).

### Claude's Discretion
- Exact emitter function/branch wiring (where in `emit_currency_sections` the override is applied).
- Whether to factor a shared helper out of the existing unique bronze-bg code or duplicate the
  small clone-null-append snippet — either is fine if output is correct and the suite stays green.
- Guard-test structure.
</decisions>

<code_context>
## Existing Code Insights

### Reusable Assets
- `src/emitter.rs::emit_unique_sections` — the canonical clone-null-append bronze-background
  pattern (`SetBackgroundColor 175 96 37 240`). Mirror it; gems use alpha 255 and the per-category color.
- `src/emitter.rs::emit_currency_sections` — where gem/currency category blocks are emitted; this
  is where the `background_override` branch goes.
- `src/pricer.rs::CategoryBlock.background_override: Option<[u8;3]>` — already populated for the gem
  categories in Phase 8 (carried from `mapping.json` `gemFilters[].backgroundOverride`). Phase 9 consumes it.
- `TierGroup.is_multi_class` (added Phase 8 review fix) — uncut gem blocks are multi-class; emission must
  keep working for both single- and multi-class blocks when adding the background.

### Established Patterns
- `emit_style_actions` emits the canonical action order; the bronze pattern nulls the cloned style's
  background so the explicit override is the sole `SetBackgroundColor`.
- Determinism: identical inputs → identical output. The fixture suite (`artifact_guard`,
  `generate_snapshot`, `generate_block_counts`) locks output.

### Integration Points
- `src/emitter.rs` (emit_currency_sections) consumes `CategoryBlock.background_override`.
- Fixtures: `tests/fixtures/seed-universal.filter` + insta snapshot will change (gem blocks gain a
  `SetBackgroundColor` line) → deliberate re-baseline THIS phase so `cargo test` stays green.
- `tests/fixtures/seed-prices.json` already has representative gem entries (Phase 8) — no new seed data needed.
</code_context>

<specifics>
## Specific Ideas

- Uncut gem block background: `SetBackgroundColor 80 70 55 255`.
- Lineage Support Gem block background: `SetBackgroundColor 10 150 150 255`.
- Verify against the priced-unique blocks already in `seed-universal.filter` for the exact emission shape.
</specifics>

<deferred>
## Deferred Ideas

- `corrupted` `when` condition + exceptional-gear rules + the `exceptional` style → Phase 10.
- Any change to which gems are priced/tiered (that was Phase 8).
</deferred>
