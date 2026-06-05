# Phase 6: Emitter Rewrite + Migration Parity - Context

**Gathered:** 2026-06-05
**Status:** Ready for planning

<domain>
## Phase Boundary

Replace the hardcoded `emit_gear_bands` / `emit_flask_charm_bands` (and recycle) special-casing
in `src/emitter.rs` with a generic rule compiler that turns the Phase 5 `rules` array into
filter blocks, in authored order (first-match-wins). Migrate `config.json` to the new rule
schema so the rule-driven generator reproduces the **pre-edit baseline** filter. Re-baseline the
artifact-guard fixture deliberately. Implement the unknown-style validation deferred from Phase 5.

The price-driven currency/category and unique sections are NOT rules — they stay appended by the
pricer, unchanged. The 7 in-game edits (CAP-01..06) are NOT added here — they are Phase 7.

</domain>

<decisions>
## Implementation Decisions

### Parity target & scope
- **Phase 6 reproduces the PRE-EDIT baseline**, NOT the full current in-game filter. The 7 manual edits land in Phase 7 as rule changes. This keeps the phases cleanly separable.
- **Phase 6 parity oracle (frozen, committed):** `tests/fixtures/parity-baseline-earlygame.filter` — the OLD (hardcoded) generator's `generate --early-game` output, captured before the rewrite (53 Show/Hide blocks). The new rule-driven generator's `--early-game` output must be semantically equivalent to this.
- **Phase 7 parity oracle (already provided by author):** `foo.filter` at repo root — the author's snapshot of the manual in-game filter WITH the 7 edits (54 blocks). The live game-dir `universal.filter` will keep changing as the author plays, so `foo.filter` is the frozen snapshot to validate Phase 7 against. (Do not use the game-dir file directly.)
- **Parity comparison is SEMANTIC**, not byte-for-byte: compare the rule-generated policy blocks as a normalized set — order-insensitive and comment/label-insensitive (normalize each block to its conditions+actions, ignore the `# ...` header text and block ordering). Surface any difference that is NOT pure reordering for author review.
- **Currency/category + unique sections stay pricer-driven and unchanged.** Their parity is covered by the existing `artifact_guard` re-baseline (data-driven), not by rules. Confirm `git diff` shows the pricer-appended sections are byte-identical aside from the re-baseline.

### Emitter design & re-baseline
- Replace `emit_gear_bands` / `emit_flask_charm_bands` with a generic `emit_rules(&[Rule])` that compiles each rule → one `Show`/`Hide` block via the existing canonical action order (`emit_style_actions`). **Recycle, gear, flask, charm, jewellery, and unidentified policy all become rules** in `config.json`. The pricer-driven currency/unique emission is retained as-is.
- **Implement unknown-style validation now** (deferred from Phase 5): resolving a rule's `style` name against `Styles` is a generate-time HARD ERROR with a field-path-aware message if the name is unknown (covers RULE-03 + the Phase 5 deferral).
- **Re-baseline `tests/fixtures/seed-universal.filter`** from the new generator and update the `artifact_guard` / `generate_snapshot` tests deliberately (MIG-03). The re-baseline is expected; the test must still pin a stable golden master afterward.
- **Block comments/labels** are derived from a rule's optional `name`; where absent, a simple derived label. Comments do NOT affect semantic parity.

### Condition → filter-line mapping (must reproduce today's output)
- The migrated rules must reproduce every current policy block. Cross-check the new emitter's
  output for these exact conditions used by the old generator: `Class == "..."`, `Rarity == Rare`,
  `Rarity >= Magic`, `AreaLevel <= N` / `>= N`, `BaseEvasion 0`, `BaseEnergyShield > 0`,
  `BaseArmour 0`, `Quality >= 1`, `Sockets >= 1`, `BaseType == "..."`. Use `classGroups` aliases
  for the build weapon/armour class lists to avoid repetition.

</decisions>

<code_context>
## Existing Code Insights

### Reusable Assets
- `src/emitter.rs` — `emit_style_actions` (canonical D-03 action order), `style_for_tier`, `FilterColor`/`MinimapShape` display impls, the pricer-driven `emit_currency_sections` / `emit_unique_sections` (KEEP these unchanged). `emit_recycle`, `emit_gear_bands`, `emit_flask_charm_bands` are the hardcoded functions being REPLACED by `emit_rules`.
- `src/models/config.rs` — Phase 5 types: `Rule`, `When`, `RarityComparator`/`RarityOp`/`Rarity`, `NumericCondition`, `AreaLevelRange`, `UnidentifiedTier`, `Styles` (style-name lookup target), `Config.rules`, `Config.class_groups`.
- `config.json` — current model with `styles`, `build`, `weaponClasses`, `armourClasses`, `flaskCharmClasses` (these inform the rules + classGroups; the migration replaces the band-driven structure with explicit rules).
- Tests: `generate_snapshot` + `artifact_guard` in `tests/cli_generate.rs`; CRLF-normalize (`s.replace("\r\n","\n")`) on both sides of filter comparisons (Windows working tree is CRLF, generator emits LF).

### Established Patterns
- Deterministic generation (no HashMap iteration at output time); LF-only output ending in one newline.
- Field-path/expected-type validation errors; `deny_unknown_fields`.

### Integration Points
- `emitter::generate(&Config, &PricedData)` — its body changes from calling the hardcoded band functions to calling `emit_rules(&config.rules, ...)` (with classGroups resolution) then the unchanged pricer sections.
- `config.json` is rewritten to the rules schema; `config_parses_real_file` and all generate tests must pass.

</code_context>

<specifics>
## Specific Ideas

- Author acceptance bar (milestone-level): generated output semantically equivalent to the filter the author uses; reordering is fine; any other behavioral change must be surfaced for author review BEFORE marking complete. Get it close before querying.
- Two frozen oracles: `tests/fixtures/parity-baseline-earlygame.filter` (Phase 6, pre-edit) and `foo.filter` (Phase 7, with edits).

</specifics>

<deferred>
## Deferred Ideas

- Adding the 7 in-game edits as rules (CAP-01..06) and CLAUDE.md realignment — Phase 7.
- A reusable semantic-filter-diff helper could be promoted to a small tool later; for now an internal test helper is enough.

</deferred>
