# Phase 10: Corrupted Condition + Exceptional Gear - Context

**Gathered:** 2026-06-24
**Status:** Ready for planning

<domain>
## Phase Boundary

The rule `when` vocabulary gains a `corrupted` boolean condition, `config.json` gains
exceptional-gear rules (extra sockets OR high quality, uncorrupted) styled with a new dedicated
`exceptional` named style, and all fixtures are re-baselined. This is the final v0.3 phase —
EXC-01/02/03/04. Closes the milestone.
</domain>

<decisions>
## Implementation Decisions

### Corrupted condition (EXC-01)
- Add `corrupted: Option<bool>` to the `When` struct (`src/models/config.rs`), serde
  `#[serde(default)]`, honoring `deny_unknown_fields`.
- Emit arm in `emit_rules` (`src/emitter.rs`): `Some(true)` → `Corrupted True`, `Some(false)` →
  `Corrupted False`, `None` → no line. Correct PoE2 syntax: `Corrupted False` (boolean, NO operator).

### Exceptional `exceptional` style (EXC-04)
- NEW named style added to the hardcoded `Styles` vocabulary (`src/models/config.rs`) and
  `emitter::resolve_style` — same precedent as `unidentified`/`craftable` (the config-driven-styles
  refactor 999.1 stays in backlog).
- Modeled on `Recycle` but PINK: action = Show; text + border `255 121 198`; background = Dracula
  `40 42 54 255`; PlayEffect Pink; MinimapIcon Pink Cross; NO custom sound; NO font override.
- Kept as its OWN style (same pink as S today) so its color can be retuned independently later.

### Exceptional gear rules (EXC-02, EXC-03)
Three rules, ALL styled `exceptional`, ALL with `corrupted: false`, ANY rarity (NO rarity bound —
exceptional uniques are highlighted too), placed at the VERY TOP of the `rules` array, BEFORE the
Recycle rule (exceptional wins for notable items; Recycle still catches the rest):
- **Rule A — exceptional sockets (2H / body):** `class` = [Body Armours, Two Hand Maces, Bows,
  Crossbows, Quarterstaves, Staves], `sockets: { min: 3 }`, `corrupted: false`.
- **Rule B — exceptional sockets (1H / smaller):** `class` = [Helmets, Gloves, Boots, Shields,
  Bucklers, Foci, One Hand Maces, Sceptres, Wands, Spears], `sockets: { min: 2 }`, `corrupted: false`.
- **Rule C — exceptional quality:** `class` = the UNION of all gear classes in A + B, `quality: { min: 24 }`, `corrupted: false`.
- OR-logic is expressed as separate rules (filter blocks are AND-only) — do NOT combine sockets+quality into one rule.

### Re-baseline
- `config.json` (the source policy) gains the 3 rules + the `exceptional` style.
- Re-baseline `tests/fixtures/seed-universal.filter`, the insta snapshot, and `generate_block_counts`
  (block counts WILL increase by 3 Show blocks) THIS phase so `cargo test` ends green. Add seed
  coverage only if needed to exercise the new blocks (the rules emit regardless of priced data, so
  the generated filter gains the blocks deterministically).

### Claude's Discretion
- Exact placement/structure of the new `Style` field and `resolve_style` arm.
- Whether to name the rules' `name` fields exactly as above (kebab slugs in the `Show #` comment).
- Test structure for the corrupted-emit unit test and the exceptional-block guard.
</decisions>

<code_context>
## Existing Code Insights

### Reusable Assets
- `src/models/config.rs` `When` struct — add `corrupted: Option<bool>` alongside `quality`/`sockets`/`class`.
- `src/models/config.rs` `Styles` struct + `Style` — add the `exceptional` named style (mirror how
  `unidentified`/`craftable` were added). `FUTURE` comments already mark the two code locations for 999.1.
- `src/emitter.rs` `emit_rules` — add the `corrupted` emit arm (follow `emit_numeric`/boolean patterns).
- `src/emitter.rs` `resolve_style` — add the `exceptional` arm.
- `Recycle` style definition is the structural template (text/border + PlayEffect + MinimapIcon Cross, no sound) — copy it, swap green→pink `255 121 198`.

### Established Patterns
- Rules are ordered, first-match-wins; `Recycle` is currently first. Insert the 3 exceptional rules
  ABOVE it.
- `corrupted: false` emits `Corrupted False` (no operator) — matches CLAUDE.md boolean-condition grammar.
- Determinism + fixture oracle (`artifact_guard`/`generate_snapshot`/`generate_block_counts`).

### Integration Points
- `config.json` (repo root) — the policy source; add style + rules.
- `src/models/config.rs`, `src/emitter.rs` — schema + emission.
- `tests/fixtures/seed-universal.filter` + snapshot + `generate_block_counts` — deliberate re-baseline (+3 Show blocks).
</code_context>

<specifics>
## Specific Ideas

- `exceptional` style RGB: `255 121 198` (pink, same as S today, but a SEPARATE style).
- Quality threshold: `24`. Socket thresholds: `3` (2H/body), `2` (1H/smaller).
- Exceptional rules come BEFORE Recycle in the `rules` array.
- Reference the existing `Recycle` and `unidentified` style/rule emission in `seed-universal.filter` for exact shape.
</specifics>

<deferred>
## Deferred Ideas

- Config-driven style vocabulary (Phase 999.1 backlog) — `exceptional` is added in code per the
  established pattern; the general refactor stays parked.
- Per-slot quality thresholds or rarity-bounded variants — not requested (any rarity, flat 24).
</deferred>
