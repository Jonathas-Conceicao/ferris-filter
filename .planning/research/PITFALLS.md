# Pitfalls Research

**Domain:** FerrisFilter v0.3 — Economy Items & Exceptional Gear
**Researched:** 2026-06-23
**Confidence:** HIGH (all findings are grounded in the existing codebase, prior incidents, and PoE2 filter grammar documented in CLAUDE.md)

---

## Critical Pitfalls

### Pitfall 1: Uncut Gem BaseType Includes Level Suffix, Which the Filter Cannot Match

**What goes wrong:**
The poe.ninja display name for an uncut gem entry is `"Uncut Skill Gem (Level 20)"`. The current `strip_gem_level` function in `src/sync/prices.rs` already removes this suffix before writing to `prices.json`, so the stored `base_type` is correctly `"Uncut Skill Gem"`. The bug is the inverse: if the level suffix is ever accidentally left in the emitted `BaseType ==` line (e.g. if `strip_gem_level` is bypassed, or if a new code path writes the raw poe.ninja name), the filter block will never match any item in-game — PoE2 base types never carry level suffixes.

The v0.3 Lineage Support Gem path is a second opportunity for this class of bug if it goes through a different code branch that does not call `strip_gem_level`. The same risk applies to Verisium if it carries any suffix in poe.ninja data.

**Why it happens:**
The fix (stripping) exists on the sync side, but the emitter trusts whatever `base_type` is in `prices.json`. A new fetch path that writes raw names to `prices.json` silently breaks all matching without a compile error.

**How to avoid:**
- Add a unit test asserting `!base_type.contains("(Level")` on every item written to `prices.json` from the gem and lineage categories. This test should live in `src/sync/prices.rs` alongside the existing `gem_level_filter_keeps_only_min_level_and_above` test.
- When adding the Lineage Support Gems path, reuse or extract `strip_gem_level` into a shared helper and call it in both branches — do not inline a new strip call, which risks diverging behavior.
- The existing `parse_gem_level` + `strip_gem_level` pair handles `"(Level N)"` from the poe.ninja name; verify the Lineage Gem name format on poe.ninja is identical (if it uses a different suffix format, the function needs extension).

**Warning signs:**
- The emitted filter contains a `BaseType ==` line with `(Level` in the string — visually obvious on inspection of `universal.filter`.
- In-game: the gem highlight rule exists in the filter but no items ever trigger it.
- A unit test checking `!base_type.contains("(Level")` fails.

**Phase to address:**
The sync phase (Phase 1) when adding Lineage Support Gems. Add the regression test before wiring up the new fetch branch.

---

### Pitfall 2: poe.ninja `type=` Spelling — Silent Empty Fetch, No Error

**What goes wrong:**
poe.ninja's exchange endpoint returns an empty `lines` array (not an HTTP error) when the `type=` query parameter is misspelled or is the wrong plurality. The existing guard `overview_types_use_correct_plural_spellings` in `src/sync/mapping.rs` already enforces correct spellings for the current set. If `Verisium` or `LineageSupportGems` are added with wrong spellings (e.g. `"Verisiums"`, `"LineageSupportGem"`, `"LineageGems"`), sync will silently fetch empty data and write a `prices.json` with no Verisium or lineage gem entries. `generate` will then emit no blocks for those categories. No error is raised.

The prior documented instance of this was "Essence" vs "Essences" — the plural fetches data, the singular does not.

**Why it happens:**
The `build_prices` function has a documented guard for empty `lines` (it skips with a warning) but does not distinguish between "genuinely no data this league" and "misspelled type returns nothing". Misspellings are indistinguishable from off-season emptiness at the code level.

**How to avoid:**
- Verify the exact spelling by curling the live poe.ninja endpoint before adding to `mapping.json`. The pattern for the current league (Runes of Aldur) is: `curl "https://poe.ninja/poe2/api/economy/exchange/current/overview?league=Runes%20of%20Aldur&type=Verisium"`.
- Extend the `overview_types_use_correct_plural_spellings` test in `src/sync/mapping.rs` to enumerate every new type added:
  ```rust
  for t in ["Essences", "Runes", "SoulCores", "Fragments", "Idols", "Verisium", "LineageSupportGems"] {
      assert!(types.contains(&t), "overviewTypes must use correct type '{t}'");
  }
  for bad in ["Essence", "Rune", "SoulCore", "Fragment", "Verisiums", "LineageSupportGem"] {
      assert!(!types.contains(&bad), "'{bad}' is wrong — use the correct spelling");
  }
  ```
- The pluralization pattern is inconsistent in poe.ninja: some types are plural (`Essences`, `Runes`, `SoulCores`), some are singular (`Verisium` if it is an uncountable noun). Do not assume plural; verify each one.

**Warning signs:**
- `sync` prints `Warning: poe.ninja type 'X' returned empty lines, skipping` for the new category.
- `generate` output has no section header for the category.
- The live `prices.json` after sync has no items with `category: "Verisium"` or `category: "Lineage Support Gems"`.

**Phase to address:**
Phase 1 (sync additions). Verify live before committing the `mapping.json` entry.

---

### Pitfall 3: Gem Class Name — "Stackable Currency" May Be Wrong for Lineage Support Gems

**What goes wrong:**
The current `GemFilter` in `mapping.json` sets `"class": "Stackable Currency"` for uncut gems and emits `Class == "Stackable Currency"` in the filter block. This was likely correct for Uncut Skill/Spirit/Charm Gems in prior leagues. Lineage Support Gems may belong to a different in-game class. If the emitted `Class ==` line uses "Stackable Currency" but Lineage Support Gems are classified differently in-game, the block silently matches nothing.

This is the exact mechanism of the Tablets failure in v0.2: `Class "Tablet"` was emitted but failed to load in-game because no such class exists. That incident forced removing the tablet block entirely. For gems, the failure mode is subtler — the block loads but matches nothing (instead of failing to parse).

**Why it happens:**
The class name comes from `mapping.json` and is trusted by the emitter without in-game validation. poe.ninja's response does not reliably indicate the in-game `Class` value (its category field is its own taxonomy).

**How to avoid:**
- Verify the in-game class for Lineage Support Gems before writing the `mapping.json` entry. Use poe2db.tw or the in-game item tooltip. Check whether they are "Stackable Currency", "Gems", or something else.
- If the class is unknown or unverifiable pre-launch, consider omitting the `Class` line and matching only on `BaseType ==` for the gem blocks — this is wider but correct, versus a narrow-but-broken class filter.
- Add a test in `mapping.rs` that explicitly asserts the gem-related class values, naming the expected string. This fails loudly if someone changes it without updating the comment.
- If the Lineage Support Gems require a different class, the `GemFilter` struct is insufficient to represent them — they need a separate `OverviewType` entry with their own `class` field.

**Warning signs:**
- The filter block for Lineage Support Gems exists in `universal.filter` but no items highlight in-game.
- Running `ferrisfilter generate` after sync produces a block with `Class == "Stackable Currency"` and `BaseType == "..."`, but those gems never trigger it.

**Phase to address:**
Phase 1 (sync) — resolve the class before wiring up `mapping.json`.

---

### Pitfall 4: Exceptional Gear — OR Conditions Require Two Separate Filter Blocks

**What goes wrong:**
The v0.3 requirement is to highlight uncorrupted gear that has extra sockets OR extra quality. The PoE2 filter grammar does not have OR logic within a block. All conditions in a block are AND. Trying to express `(Sockets >= 2 OR Quality >= 1) AND Corrupted False` in a single block is impossible — one block can only represent one conjunction.

If the implementation writes a single rule with both `Sockets >= 2` and `Quality >= 1`, that block highlights only items that have both, missing the "either" intent. This is a silent logic error: the block parses and matches but has different semantics than intended.

**Why it happens:**
The declarative rule model (one rule = one filter block = one conjunction) does not expose OR-within-block syntax, and there is no warning when a developer writes two conditions intending them as OR. The config is AND-only by design.

**How to avoid:**
- Use two separate rules in `config.json`, both before any hide rule:
  ```json
  { "name": "exceptional: sockets", "style": "C",
    "when": { "class": [...gear classes...], "sockets": { "min": 2 }, "corrupted": false } },
  { "name": "exceptional: quality", "style": "C",
    "when": { "class": [...gear classes...], "quality": { "min": 1 }, "corrupted": false } }
  ```
  Because the engine is first-match-wins, either rule fires independently.
- Note: `corrupted` is not currently in the `When` vocabulary (the `config.rs` `When` struct has no `corrupted` field). Verify whether `Corrupted False` is needed and if so, add the `corrupted: Option<bool>` field to `When` and the corresponding emitter branch before authoring the rules.
- If `Corrupted False` cannot be expressed via the rule model, the alternative is to rely on drop mechanics (corrupted items come from specific sources) and accept the false positive. Document the tradeoff explicitly.

**Warning signs:**
- Only items with both extra sockets AND extra quality are highlighted (AND instead of OR).
- A single config rule has both `sockets` and `quality` conditions with the comment "either-or" — this is wrong.

**Phase to address:**
Phase 2 (exceptional gear rule authoring). Establish the two-rule pattern in `config.json` and verify in-game before locking.

---

### Pitfall 5: Corrupted Condition Not in the Rule Vocabulary

**What goes wrong:**
The `When` struct in `src/models/config.rs` has no `corrupted` field. If the exceptional-gear rules must exclude corrupted items (to avoid wasting attention on items that cannot be crafted), there is currently no way to express `Corrupted False` in a declarative rule. The emitter will silently omit the condition, and the rule will fire on corrupted items too. This is a false-positive issue: corrupted gear with extra sockets gets highlighted as "exceptional" even though it cannot be improved.

**Why it happens:**
The v0.2 rule vocabulary was built for the existing policy (area level, class, rarity, base defence, quality, sockets, base type, unidentified tier). Boolean filter conditions (Corrupted, Mirrored, Identified) were not needed by any prior rule and were never added.

**How to avoid:**
- Decide first whether corrupted exclusion is required for the exceptional highlight. If yes, add `corrupted: Option<bool>` to `When` in `config.rs` and emit `Corrupted False` / `Corrupted True` in `emit_rules`. This is a small, low-risk addition.
- If corrupted exclusion is deferred (accept false positives), document the design decision explicitly in the config comment for the exceptional-gear rule so a future reader does not add it as a silent omission.
- Either way, add a unit test asserting the emitter does or does not emit `Corrupted` — to prevent regression in both directions.

**Warning signs:**
- Attempting to add `"corrupted": false` to a rule's `when` object causes a `deny_unknown_fields` deserialization error at generate-time. This surfaces immediately but only if tested.
- Corrupted gear with extra sockets appears highlighted in-game when only uncorrupted exceptional gear was intended.

**Phase to address:**
Phase 2 (before authoring exceptional-gear rules). Add the field or document the omission.

---

### Pitfall 6: Gem Per-Category Background Override Clashes With Emitter's Style-Based Pipeline

**What goes wrong:**
Gem blocks need a fixed per-category background (e.g. `80 70 55` for uncut gems, `10 150 150` for lineage gems) that overrides the tier style's Dracula background. The current `emit_currency_sections` function calls `emit_style_actions(style)` which emits whatever `background` is in the named style. For gems, the per-category background must replace the tier style's background — not be appended after it (the last `SetBackgroundColor` wins, but two conflicting lines is messy).

The same mechanism was used successfully for priced uniques (bronze background `175 96 37 240`): `emit_unique_sections` clones the style, sets `background = None`, calls `emit_style_actions`, then appends `SetBackgroundColor 175 96 37 240` explicitly. Gems need the same pattern — but the currency pipeline in `emit_currency_sections` does not currently support a per-category background override.

**Why it happens:**
The currency pipeline was designed for homogeneous categories where every tier block uses the tier's style verbatim. Per-category background overrides require category-level metadata that the pipeline does not carry.

**How to avoid:**
- Model the override explicitly: add an `Option<[u8; 3]>` background override to `CategoryBlock` (in `pricer.rs`) and propagate it from wherever gems are classified (the sync/mapping layer).
- In `emit_currency_sections`, after calling `emit_style_actions`, check if `category.background_override.is_some()` and emit the explicit `SetBackgroundColor` line (mirroring the unique section pattern).
- Do not use two `SetBackgroundColor` lines in the same block — redundant lines are harmless (last wins) but confusing on inspection. Set `style.background = None` in the clone before calling `emit_style_actions`.

**Warning signs:**
- Generated filter has two `SetBackgroundColor` lines in a gem block (the tier's Dracula bg plus the per-category override).
- The per-category background is absent from gem blocks (background override was not wired up, only the tier background is used).

**Phase to address:**
Phase 1 (gem styling, when adding Lineage Support Gems and fixing Uncut Gems). Wire this up before merging the category blocks into the emitter.

---

### Pitfall 7: Native-Rarity-Color Boundary Violated by Gem Background Override

**What goes wrong:**
CLAUDE.md mandates: "generic equipment/jewellery/flask/charm blocks that match by Class + Rarity alone must NOT repaint the item — omit SetTextColor/SetBorderColor/SetBackgroundColor so the game's native rarity color shows through." Gems, however, ARE explicitly curated and price-driven, so the Dracula text/border + per-category background override is correct for them. The risk is accidentally applying a background override to a rule that is NOT a curated gem/currency rule — particularly if a new developer sees the per-category override pattern and applies it to the `unchanged` or `hidden` style blocks.

The constraint is: background + text/border overrides are ONLY for curated/priced blocks (currency, gems, priced uniques). They must NOT appear on rules using the `unchanged` or `hidden` styles (which are for equipment policy blocks where native rarity color must be preserved).

**Why it happens:**
The native-rarity rule is enforced by convention (the `unchanged` and `hidden` styles have `text: None`, `border: None`, `background: None` in `config.json`), not by a type-system invariant. Adding a per-category background override in the emitter creates a new code path that, if misapplied to a policy block, silently violates the design principle.

**How to avoid:**
- Keep the per-category background override strictly in `emit_currency_sections` (the pricer-driven pipeline), not in `emit_rules` (the policy pipeline). Never add a `background_override` field to the `Rule` struct.
- Add a test asserting that policy blocks (output of `emit_rules`) contain no `SetBackgroundColor` line for any rule using the `unchanged` or `hidden` style.
- Document the boundary in the code: the comment in `emit_currency_sections` should state why per-category backgrounds are permitted there but forbidden in `emit_rules`.

**Warning signs:**
- A policy block in `universal.filter` (before the `# --- Currency ---` marker) contains a `SetBackgroundColor` line.
- The existing `pricer_section_present_and_unchanged` test detects `Class == ` in the policy section — a similar test could check for `SetBackgroundColor` in the policy section.

**Phase to address:**
Phase 1 (gem styling). Add the guard test when wiring up the background override.

---

### Pitfall 8: Snapshot and Artifact Guard Tests Will Fail After Any Generator Change

**What goes wrong:**
Two tests lock the generated output to a committed baseline:
1. `artifact_guard` in `tests/cli_generate.rs` — compares against `tests/fixtures/seed-universal.filter`.
2. `generate_snapshot` in `tests/cli_generate.rs` — compares against an insta snapshot file in `tests/snapshots/`.

Adding Verisium, Lineage Support Gems, or the gem background override to the generator will change `universal.filter`'s content whenever `prices.json` contains those categories. The seed fixture `tests/fixtures/seed-prices.json` does NOT contain Verisium or lineage gems yet, so the artifact guard will pass on the seed — BUT if the seed is updated to include them (which it should be, to give the new categories test coverage), the fixture re-baseline is mandatory. Forgetting to re-baseline produces a permanently failing CI.

The `generate_block_counts` test in `src/emitter.rs` hardcodes `84 Show blocks` and `3 Hide blocks` — new categories or rules will change these counts.

**Why it happens:**
Snapshot tests are a deliberate trade-off: they catch accidental drift but require intentional re-baselining on every deliberate change. The danger is treating a passing artifact guard as "no re-baseline needed" — it only passes if the seed fixture does not exercise the new categories.

**How to avoid:**
- Decide explicitly before each code change: does this change affect the seed fixture's output? If yes, re-baseline first (or in the same commit as the change).
- Update `tests/fixtures/seed-prices.json` to include representative entries for each new category (Verisium, lineage gems) as part of the gem/sync phases. This ensures the new code paths are exercised by the snapshot.
- After re-baselining: `cp <generated output on seed inputs> tests/fixtures/seed-universal.filter`, then review the diff carefully.
- Update the `generate_block_counts` assertion to the new counts after all categories are added.
- Add an insta snapshot update step to the milestone completion checklist.

**Warning signs:**
- `artifact_guard` fails with "generator output does not match tests/fixtures/seed-universal.filter".
- `generate_snapshot` fails (insta shows a diff).
- `generate_block_counts` panics with "expected 84 Show blocks, got N".

**Phase to address:**
Every phase that changes generator output. Make re-baselining an explicit exit criterion for each phase's success checklist.

---

### Pitfall 9: League Name URL Encoding for Leagues With Spaces

**What goes wrong:**
The current league is "Runes of Aldur" — it contains spaces. The league name is passed as a `league=` query parameter to poe.ninja. The `reqwest` client encodes query parameters using `reqwest::RequestBuilder::query(&[("league", name)])`, which percent-encodes spaces as `%20`. poe.ninja accepts `%20`-encoded spaces. This is already working correctly (the v0.1 `sync` using `reqwest::query()` did not have an encoding bug).

The risk is lower than it was for single-word leagues, but can emerge if: (a) someone switches to manual URL concatenation instead of `.query()`, or (b) a future league name contains characters reqwest does not encode the same way poe.ninja expects (e.g. accented characters, `&`, `#`).

**Why it happens:**
It is tempting to build URLs manually with `format!("{OVERVIEW_URL}?league={league}&type={type}")`, which does not encode spaces — producing a malformed URL that either errors or silently queries the wrong league.

**How to avoid:**
- Never manually concatenate query parameters. Always use `client.get_json(url, &[("league", name), ("type", type)])` which passes through `reqwest::RequestBuilder::query()`.
- The existing `display_name_not_url_slug` test in `src/sync/league.rs` guards against passing the URL slug (`vaal`) instead of the display name (`Runes of Aldur`) — keep that test current when the league changes.
- Add a test for `resolve_league` against a fixture that includes "Runes of Aldur" as the active league name, to confirm the current league is correctly resolved.

**Warning signs:**
- `sync` returns HTTP 400 or an empty/wrong response when the league has spaces.
- The league name in `prices.json` is a slug (`runes-of-aldur`) instead of the display name (`Runes of Aldur`).

**Phase to address:**
No dedicated phase needed — the pattern is already correct. Add the Runes of Aldur fixture to the league tests as a regression guard.

---

## Technical Debt Patterns

| Shortcut | Immediate Benefit | Long-term Cost | When Acceptable |
|----------|-------------------|----------------|-----------------|
| Hardcode gem class as "Stackable Currency" without live verification | Faster implementation | Block silently matches nothing if class is wrong; impossible to detect without in-game testing | Never — always verify class against poe2db.tw first |
| Single rule for exceptional (sockets AND quality) | Simpler config | Wrong semantics; misses items that only have one of the two | Never — OR logic always requires two rules |
| Skip `corrupted` condition on exceptional rule | No code change needed | Highlights corrupted gear that cannot be crafted | Acceptable if design decision is documented |
| Not updating seed-prices.json with new categories | No fixture churn during development | Artifact guard does not exercise new code paths; regression goes undetected | Acceptable during development only; must update before milestone completion |
| Adding Lineage gems under a new `GemFilter2` field in mapping.json | Minimal mapping schema change | Two separate gem code paths diverge over time; `strip_gem_level` called inconsistently | Never — extract a shared gem spec type or make the mapping support multiple gems |

---

## Integration Gotchas

| Integration | Common Mistake | Correct Approach |
|-------------|----------------|------------------|
| poe.ninja exchange endpoint | Wrong `type=` spelling (singular/plural mismatch) returns empty `lines`, no error | Verify spelling live against current league before adding to `mapping.json`; guard with a plural-spelling test |
| poe.ninja exchange endpoint | Using URL slug as `league=` parameter (`runes-of-aldur`) | Use the display name from `resolve_league` (`Runes of Aldur`); `.query()` handles encoding |
| poe.ninja exchange endpoint | Adding a new gem category under `overviewTypes` instead of a gem-specific path | Gem categories need level-stripping logic; `overviewTypes` path does not call `strip_gem_level` |
| PoE2 filter grammar | Writing `Corrupted False` without verifying it is a valid PoE2 condition | Verify via CLAUDE.md "Conditions (Boolean)" section; `Corrupted` is listed as a supported boolean |
| PoE2 filter grammar | Expressing OR within a single block (`Sockets >= 2 OR Quality >= 1`) | Use two separate `Show` blocks; PoE2 filter AND-only within a block |
| PoE2 filter grammar | Using `Class "Gems"` or `Class "Support Gems"` for uncut/lineage gems | Verify exact class name in-game; "Stackable Currency" was right for Uncut gems in prior leagues but may not hold for Lineage gems |

---

## "Looks Done But Isn't" Checklist

- [ ] **Gem BaseType in filter:** Verify emitted `BaseType ==` lines contain no `(Level` suffix — inspect `universal.filter` after generate.
- [ ] **Verisium/Lineage type spellings:** After adding to `mapping.json`, run `ferrisfilter sync` against the live league and confirm `prices.json` contains entries for both categories.
- [ ] **Lineage gem class:** Confirmed against poe2db.tw or in-game tooltip — not assumed from prior gem categories.
- [ ] **Exceptional gear OR logic:** Two separate rules exist in `config.json` for sockets and quality conditions — not one combined rule.
- [ ] **Corrupted condition decision documented:** Either `corrupted: Option<bool>` is in `When` and tested, or a comment in `config.json` explains why it is omitted.
- [ ] **Per-category gem background override:** Emitted `SetBackgroundColor` line in gem blocks uses the per-category color, not the tier's Dracula background.
- [ ] **No duplicate `SetBackgroundColor` in gem blocks:** Each gem block has exactly one `SetBackgroundColor` line.
- [ ] **Seed fixture re-baselined:** `tests/fixtures/seed-prices.json` includes new categories; `tests/fixtures/seed-universal.filter` and snapshot re-generated and reviewed.
- [ ] **`generate_block_counts` updated:** The hardcoded Show/Hide count assertion matches the new total after all phases complete.
- [ ] **Plural-spelling test extended:** `overview_types_use_correct_plural_spellings` in `mapping.rs` covers `Verisium` and `LineageSupportGems` (or whatever the verified spellings are).

---

## Recovery Strategies

| Pitfall | Recovery Cost | Recovery Steps |
|---------|---------------|----------------|
| Wrong gem BaseType (level suffix leaked) | LOW | Fix `strip_gem_level` call in the new code path, re-sync, re-generate, re-baseline fixture |
| Wrong poe.ninja `type=` spelling | LOW | Update `mapping.json` with correct spelling, re-sync to get real data |
| Wrong gem class (block never fires in-game) | LOW | Correct `class` in `mapping.json` or remove the `Class` line, re-generate |
| Single rule for exceptional (AND instead of OR) | LOW | Split into two rules in `config.json`, re-generate |
| Missing `corrupted` condition in `When` | MEDIUM | Add `corrupted: Option<bool>` to `When`, emit `Corrupted True/False` in `emit_rules`, add tests |
| Artifact guard fails after generator change | LOW | Re-generate on seed inputs, copy output to fixture, review diff, commit |
| Background override emitted on policy blocks | LOW | Move override out of `emit_rules`, restrict to `emit_currency_sections`, add guard test |

---

## Pitfall-to-Phase Mapping

| Pitfall | Prevention Phase | Verification |
|---------|------------------|--------------|
| Gem BaseType level-suffix leak | Sync phase (Lineage gem addition) | Unit test: `!base_type.contains("(Level")` for all gem `prices.json` entries |
| poe.ninja type spelling | Sync phase (before `mapping.json` commit) | Live curl; plural-spelling guard test extended |
| Gem Class name for Lineage gems | Sync phase (before `mapping.json` commit) | poe2db.tw lookup; in-game verification |
| OR logic needing two rules | Exceptional gear phase | Two rules visible in `config.json`; in-game items with only sockets OR only quality both highlight |
| Corrupted condition missing from `When` | Exceptional gear phase | Decision documented; either field present in `When` and tested, or comment in config |
| Background override double-emit | Gem styling phase | Assert one `SetBackgroundColor` per gem block |
| Background override on policy blocks | Gem styling phase | Policy-section guard test: no `SetBackgroundColor` before `# --- Currency ---` |
| Snapshot/fixture not re-baselined | End of every phase that changes emitter output | `artifact_guard` passes; snapshot reviewed and approved |
| League name URL encoding | Sync phase (Runes of Aldur fixture) | League test fixture updated; `display_name_not_url_slug` test passes |

---

## Sources

- Codebase: `src/sync/prices.rs` (strip_gem_level, build_prices, gem_best dedup logic)
- Codebase: `src/sync/mapping.rs` (overview_types_use_correct_plural_spellings test; prior Essences/plural-spelling incident documented in test comments)
- Codebase: `src/emitter.rs` (emit_currency_sections, emit_unique_sections — unique bronze-bg override pattern)
- Codebase: `src/models/config.rs` (When struct — Corrupted field absent)
- Codebase: `tests/cli_generate.rs` (artifact_guard, generate_snapshot, generate_block_counts)
- `.planning/MILESTONES.md` — v0.2 deferred Tablets due to `Class "Tablet"` in-game load failure
- `.planning/PROJECT.md` — v0.3 target: Verisium, Lineage Support Gems, Uncut Gems fix, exceptional gear
- `CLAUDE.md` — native-rarity-color design rule; filter grammar (OR-within-block impossible; Corrupted is a valid boolean condition; class list)

---
*Pitfalls research for: FerrisFilter v0.3 — Economy Items & Exceptional Gear*
*Researched: 2026-06-23*
