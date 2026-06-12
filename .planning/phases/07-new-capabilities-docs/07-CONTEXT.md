# Phase 7: New Capabilities + Docs - Context

**Gathered:** 2026-06-05
**Status:** Ready for planning

<domain>
## Phase Boundary

Express the author's 7 in-game filter edits (CAP-01..06) as rules in `config.json` on top of the
Phase 6 rule engine, so `generate --early-game` reproduces the full manual in-game filter
(`foo.filter`) on its POLICY blocks. Add the one new style the unidentified highlight needs.
Realign `CLAUDE.md` to the declarative rule model and mark previously-"unverified" filter
conditions as confirmed (DOC-01). No emitter logic changes are expected — the Phase 6 engine
already supports every condition these rules use.

</domain>

<decisions>
## Implementation Decisions

### The 7 edits → CAP rules (encode exactly per the foo.filter vs baseline diff)
- **CAP-01** (build weapons, AreaLevel 26+): change the build-weapons-26+ rule's `when` from `rarity "==Rare"` to `rarity ">=Magic"` + `areaLevel.max = 75`. (Magic+Rare up to area 75, then fall through.)
- **CAP-02** (build armour pure-ES 46+): add `areaLevel.max = 75` to the pure-ES armour rule.
- **CAP-03** (flask/charm): mid-band flask/charm rule `rarity "==Magic"` → `">=Magic"`; AND the flask/charm Hide fallback rule gains `rarity "<=Magic"` (so Rare flasks/charms fall through instead of being hidden).
- **CAP-04** (charms independent): NEW rule — `when: { class: ["Charms"], area_level: { min: 78 }, rarity: "==Normal" }`, style `unchanged`. Surfaces white charms in deep endgame.
- **CAP-05** (jewellery hide): NEW rule — `when: { class: ["Rings","Amulets","Belts"], rarity: "<=Rare" }`, style `hidden` (font 21 + DisableDropSound, no repaint — matches existing `hidden` style).
- **CAP-06** (unidentified highlight): TWO NEW rules, both `when.unidentifiedTier.min = 4`, style `unidentified` (see below) — one rule for the weapon+armour class list, one for `["Rings","Amulets","Belts"]`.

### New `unidentified` style
- Add ONE new named style `unidentified` to `config.json` `styles`. It holds ONLY the original in-game actions and nothing extra: `beam: Orange`, `minimap: { size: 2, color: Orange, shape: Kite }`, `sound: { id: 2, vol: 200 }`, `action: Show`. No text/border/background/font (no repaint — keep native rarity color). This is required because a rule references its actions by style name (RULE-02); it adds no styling beyond what foo.filter already shows.
- This means `Styles` gains an 11th named field. Update the serde `Styles` struct + any exhaustive style match (e.g. `style_for_tier` / `resolve_style`) to include `unidentified`.

### Parity oracle & scope (Phase 7)
- Oracle: `foo.filter` (author's frozen snapshot of the manual in-game filter WITH the 7 edits). Copy it to `tests/fixtures/parity-ingame.filter` and commit as the stable oracle (do not rely on the repo-root `foo.filter` or the live game-dir file, which will keep changing).
- Comparison is SEMANTIC and **POLICY-BLOCKS-ONLY** (same normalized, order/comment-insensitive approach as Phase 6; cut at the `# --- Currency ---` marker). The pricer/currency section is EXCLUDED: its `foo.filter` differences (`Vaal Infuser` absent, `Tablets` category absent) are pricer DATA drift from the new game version, NOT policy edits — out of scope for the rule-driven policy parity.
- Add a Phase 7 parity test: `generate --early-game` policy blocks == `parity-ingame.filter` policy blocks (semantic). Plus a "cap is rule-driven" test: changing a cap value (e.g. CAP-01 area-level max) changes the emitted block.

### Re-baseline note
- Adding policy rules changes the generated policy output, so `tests/fixtures/seed-universal.filter` (artifact_guard) and the insta snapshot must be re-baselined again, deliberately, after the new rules land. Block counts will change (new unID/charms/jewellery rules add blocks).

### DOC-01 (CLAUDE.md realignment)
- Remove the "unverified"/"verify before use" caveats for `UnidentifiedItemTier` and the base-defence filters (`BaseArmour`/`BaseEvasion`/`BaseEnergyShield`) — confirmed working in-game by the author.
- Document the declarative rule model (`rules` array + `classGroups` + the `when` condition vocabulary + named styles incl. `unidentified`) as the authoritative generator input. Update/replace the now-stale hardcoded-AreaLevel-band prose so the model description matches the code.

</decisions>

<code_context>
## Existing Code Insights

### Reusable Assets
- `src/models/config.rs` — `Styles` struct (add `unidentified`), `Rule`/`When`/`RarityComparator`/`NumericCondition`/`AreaLevelRange`/`UnidentifiedTier`. All condition types the CAP rules need already exist (Phase 5).
- `src/emitter.rs` — `emit_rules` already compiles every `When` field, incl. `unidentifiedTier` → `UnidentifiedItemTier >= N` and class lists → bare `Class "..."`. `resolve_style` must learn the new `unidentified` name. No other emitter change expected.
- `config.json` — the 13 migrated rules from Phase 6; add/modify rules for the 7 edits.
- `tests/cli_generate.rs` — Phase 6 parity test pattern (`normalize_block`, BTreeSet, currency cut) to reuse for the Phase 7 oracle; re-baseline helpers.

### Integration Points
- `Styles` struct + serde `deny_unknown_fields` (the new key must be added or config.json fails to parse). Any exhaustive match over styles must handle `unidentified`.
- `config.json` `styles` object gains `"unidentified": {...}`.

</code_context>

<specifics>
## Specific Ideas

- Author note: the new game version cut some items (e.g. Tablets), which is why `foo.filter`'s currency section differs from the repo baseline. That is fallback-DATA drift handled outside the rule model — flag updating `prices-fallback.json`/`tiers-fallback.json` to drop game-cut categories as a SEPARATE follow-up, not part of this phase.
- Author acceptance bar: generated policy output semantically equivalent to `foo.filter`; reordering OK; surface any non-reorder difference for review before completion.

</specifics>

<deferred>
## Deferred Ideas

- Updating `prices-fallback.json` / `tiers-fallback.json` to remove game-cut items (Tablets etc.) — separate data cleanup, post-phase follow-up.
- A standalone semantic-filter-diff CLI tool (the test helper suffices for now).

</deferred>
