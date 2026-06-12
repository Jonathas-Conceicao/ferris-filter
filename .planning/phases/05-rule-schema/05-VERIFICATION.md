---
phase: 05-rule-schema
verified: 2026-06-05T12:00:00Z
status: passed
score: 3/4 must-haves verified (1 deferral accepted by author override)
re_verification: false
override:
  accepted_by: author
  date: 2026-06-05
  reason: "Unknown-style-name rejection is a generate-time check intentionally deferred to Phase 6 (emitter rewrite resolves style names against the tier palette), per CONTEXT.md and PLAN 05-01 scope guard. config.json migration to the rules schema is likewise Phase 6 (MIG-02). Both deferrals were pre-agreed; Phase 5 is data-model only."
deferred:
  - truth: "An unknown style tier name is rejected with an actionable error at parse/load time"
    addressed_in: "Phase 6"
    evidence: "Phase 6 emitter rewrite will validate style names at generate-time (RULE-03 + PLAN 05-01 scope guard: 'unknown-style HARD ERROR is a generate-time check deferred to Phase 6'). CONTEXT.md decision: 'An unknown style name is a HARD ERROR with a field-path-aware message at generate-time.'"
  - truth: "config.json is fully migrated to the new rule schema (contains a rules array)"
    addressed_in: "Phase 6"
    evidence: "Phase 6 goal: 'config.json is migrated to the new schema and produces a filter semantically equivalent to the currently installed in-game filter' (MIG-02). PLAN 05-02 scope guard explicitly: 'Do NOT migrate the live config.json to use rules (it stays as-is)'"
human_verification:
  - test: "Confirm unknown style name at generate-time produces a meaningful error"
    expected: "Running `ferrisfilter generate` with a config.json rule containing style: 'NotATier' should fail with an error identifying the unknown style name and field path before Phase 6 ships; this is currently a generate-time responsibility not yet implemented"
    why_human: "The serde layer intentionally accepts any string for Rule.style (Phase 6 deferred). The Phase 5 scope only models the schema. A human needs to confirm the Phase 6 milestone contract for this item is understood and accepted as deferred."
---

# Phase 5: Rule Schema Verification Report

**Phase Goal:** The new rule schema exists in serde — every condition type (class, rarity, area level, base-defence, quality, sockets, UnidentifiedItemTier, BaseType) is modeled, validated, and parseable from config.json. DATA-MODEL ONLY (emitter untouched, config.json NOT migrated to rules).
**Verified:** 2026-06-05
**Status:** human_needed
**Re-verification:** No — initial verification

---

## Goal Achievement

### Observable Truths (from ROADMAP Success Criteria)

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | `config.json` can be extended with an ordered `rules` array and `cargo test` still passes (serde models parse the new schema with field-path-aware errors on bad input) | VERIFIED | `Config.rules: Option<Vec<Rule>>` with `serde(default)` added; `config_parses_real_file` passes on unchanged config.json; `config_with_rules_and_class_groups_parses` confirms injection works; `cargo test` shows 123 lib + 9 integration tests all pass |
| 2 | A rule referencing a named style tier (S–F, unchanged, hidden, Recycle) deserializes correctly | VERIFIED | `Rule.style: String` with tests `rule_style_only` (style "F"), `rule_full_round_trip` (style "E"), `rules_preserve_authored_order` (styles F/S/unchanged) — all pass |
| 2b | An unknown tier name is rejected with an actionable error | DEFERRED (Phase 6) | `Rule.style` is a plain `String`; serde accepts any string value. PLAN 05-01 scope guard explicitly defers this to Phase 6 generate-time. CONTEXT.md confirms: "An unknown style name is a HARD ERROR at generate-time." |
| 3 | Every condition field (class, rarity comparator, area-level min/max, base-defence, quality, sockets, UnidentifiedItemTier, BaseType) deserializes from its JSON representation | VERIFIED | `when_all_condition_fields_populate` test exercises all 10 When fields; individual type tests for each condition type all pass (see artifact detail below) |
| 4 | A round-trip serialize/deserialize of a rule list produces identical values (no data loss) | VERIFIED | `config_with_rules_round_trips` test: full Config with REPRESENTATIVE_RULES_JSON (all condition types) deserializes, serializes, re-deserializes, asserts `first == second`; `rarity_comparator_round_trip` for the custom serde path |

**Score:** 3/4 truths verified (SC2-second-clause deferred to Phase 6 by plan design)

---

### Deferred Items

Items not yet met but explicitly addressed in later milestone phases.

| # | Item | Addressed In | Evidence |
|---|------|-------------|----------|
| 1 | Unknown style tier name rejected at parse/load time | Phase 6 | PLAN 05-01 task 3 action: "the unknown-style HARD ERROR is a generate-time check deferred to Phase 6"; Phase 6 emitter rewrite will validate style names when compiling rules |
| 2 | `config.json` contains a live `rules` array (full migration) | Phase 6 | PLAN 05-02 scope guard: "Do NOT migrate the live config.json to use rules"; Phase 6 goal covers MIG-02 (semantic-equivalence parity after migration) |

---

### Required Artifacts

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `src/models/config.rs` | Rule, When, RarityComparator, NumericCondition, AreaLevelRange, UnidentifiedTier types + Config.rules/class_groups fields | VERIFIED | All 8 types present at lines 148-419; `pub rules: Option<Vec<Rule>>` at line 414; `pub class_groups: Option<HashMap<String, Vec<String>>>` at line 418 |

Specific type verification:

| Type | Present | deny_unknown_fields | Substantive |
|------|---------|---------------------|-------------|
| `pub enum Rarity` | Yes (line 148) | N/A (enum) | 4 variants with PartialOrd/Ord |
| `pub enum RarityOp` | Yes (line 156) | N/A (enum) | Eq/Gte/Lte |
| `pub struct RarityComparator` | Yes (line 179) | N/A (custom serde) | Custom Deserialize + Serialize; handles ==/>=/<=/ bare prefix |
| `pub struct NumericCondition` | Yes (line 256) | Yes (line 255) | eq/min/max Option<i64> |
| `pub struct AreaLevelRange` | Yes (line 270) | Yes (line 269) | min/max Option<u32> |
| `pub struct UnidentifiedTier` | Yes (line 282) | Yes (line 281) | min: u32 |
| `pub struct When` | Yes (line 296) | Yes (line 295) | All 10 optional condition fields; Default derived |
| `pub struct Rule` | Yes (line 340) | Yes (line 339) | name/style/when with skip_serializing_if |
| `Config.rules` | Yes (line 414) | serde(default) | Option<Vec<Rule>>, skip when None |
| `Config.class_groups` | Yes (line 418) | serde(default) | Option<HashMap<String, Vec<String>>>, skip when None |

Dead type removal:

| Type | Present in src/ | Expected |
|------|-----------------|----------|
| `pub enum RarityRule` | No (confirmed grep: 0 matches) | Removed |
| `pub struct AreaLevelBand` | No (confirmed grep: 0 matches) | Removed |
| `pub struct BandShow` | No (confirmed grep: 0 matches) | Removed |
| `pub struct ArmourShow` | No (confirmed grep: 0 matches) | Removed |
| `Config.area_level_bands` field | No (confirmed grep: 0 matches) | Removed |
| `"areaLevelBands"` in config.json | No (grep exit 1) | Removed |

---

### Key Link Verification

| From | To | Via | Status | Details |
|------|----|-----|--------|---------|
| `When.rarity` | `RarityComparator` (op + Rarity enum) | Custom Deserialize parsing compact string | WIRED | `rarity: Option<RarityComparator>` at line 302; custom impl<'de> Deserialize at line 185 |
| `Rule.when` | `Option<When>` | Optional nested condition object | WIRED | `when: Option<When>` at line 350; `skip_serializing_if = "Option::is_none"` |
| `Config.rules` | `Vec<Rule>` | Optional ordered array | WIRED | `rules: Option<Vec<Rule>>` at line 414; `serde(default)` ensures backward compatibility |
| `Config.class_groups` | `HashMap<String, Vec<String>>` | Optional alias map | WIRED | `class_groups: Option<HashMap<String, Vec<String>>>` at line 418; camelCase rename via struct-level `rename_all` |

---

### Data-Flow Trace (Level 4)

Not applicable. Phase 5 is a pure data-model phase — no rendering, no UI components, no API endpoints. All types are serde structs with no dynamic data sources.

---

### Behavioral Spot-Checks

| Behavior | Command | Result | Status |
|----------|---------|--------|--------|
| All 123 lib tests pass | `cargo test --lib` | 123 passed; 0 failed | PASS |
| artifact_guard (byte-identical filter output) | `cargo test --test cli_generate artifact_guard` | ok | PASS |
| generate_snapshot | `cargo test --test cli_generate generate_snapshot` | ok | PASS |
| `src/emitter.rs` not modified | `git diff e064ee1~1..HEAD -- src/emitter.rs` | (empty — no output) | PASS |
| RarityComparator rejects "Legendary" | Verified by `rarity_comparator_rejects_unknown_rarity` test | ok | PASS |
| RarityComparator rejects "><Rare" | Verified by `rarity_comparator_rejects_bad_op` test | ok | PASS |
| NumericCondition rejects unknown key "lt" | Verified by `numeric_condition_rejects_unknown_key` test | ok | PASS |
| Rule round-trip lossless | `config_with_rules_round_trips` test | ok | PASS |
| When rejects unknown field | `when_rejects_unknown_field` test | ok | PASS |
| Rule rejects unknown field | `rule_rejects_unknown_field` test | ok | PASS |

---

### Requirements Coverage

| Requirement | Source Plan | Description | Status | Evidence |
|-------------|------------|-------------|--------|----------|
| RULE-01 | 05-02 | `Config.rules: Option<Vec<Rule>>` ordered array | SATISFIED | `pub rules: Option<Vec<Rule>>` at line 414; `rules_preserve_authored_order` test confirms order preserved |
| RULE-02 | 05-01 | `Rule.style: String` referencing named tier | SATISFIED (type-level) | `pub style: String` at line 347; style-name validation deferred to Phase 6 by plan design |
| COND-01 | 05-01 | Class list condition | SATISFIED | `When.class: Option<Vec<String>>` at line 299; `when_all_condition_fields_populate` confirms |
| COND-02 | 05-01 | Rarity comparator (==/>=/<=) | SATISFIED | `RarityComparator` with custom serde (lines 179-243); 8 dedicated tests all pass |
| COND-03 | 05-01 | Area-level min/max | SATISFIED | `AreaLevelRange { min: Option<u32>, max: Option<u32> }` (lines 270-275); 3 tests all pass |
| COND-04 | 05-01 | Numeric attributes (base-defence, quality, sockets) | SATISFIED | `NumericCondition { eq/min/max: Option<i64> }` (lines 256-262) + `When.base_armour/base_evasion/base_energy_shield/quality/sockets` fields |
| COND-05 | 05-01 | BaseType list | SATISFIED | `When.base_type: Option<Vec<String>>` at line 323; `rule_full_round_trip` exercises it |
| COND-06 | 05-01 | UnidentifiedItemTier threshold | SATISFIED | `UnidentifiedTier { min: u32 }` (lines 282-285); `When.unidentified_tier: Option<UnidentifiedTier>` at line 326 |
| MIG-01 | 05-02 | Rule schema validates with field-path errors | SATISFIED (serde validation) | `bad_rarity_in_rule_reports_error` test confirms invalid rarity produces Err; PLAN note: loader.rs provides full field-path context at runtime via serde_path_to_error |

All 9 Phase 5 requirement IDs from PLAN frontmatter are accounted for: RULE-01, RULE-02, COND-01, COND-02, COND-03, COND-04, COND-05, COND-06, MIG-01.

---

### Anti-Patterns Found

| File | Line | Pattern | Severity | Impact |
|------|------|---------|----------|--------|
| `src/models/config.rs` | 347 | `pub style: String` (any string accepted) | Info | Intentional deferral to Phase 6; documented in code comment; not a stub — the field is substantive and the validation decision is explicit |

No blockers, no stubs, no placeholder implementations. The `style: String` design is a deliberate scope decision, not an oversight.

---

### Human Verification Required

#### 1. SC2 Unknown Style Tier Rejection — Confirm Deferral Is Acceptable

**Test:** Attempt to deserialize a `Rule` with `style: "NotATier"` (e.g. via a quick `serde_json::from_str` call or by running `ferrisfilter generate` with a config that includes such a rule). Confirm that it currently accepts the unknown style silently (the serde layer will not reject it), and confirm the team understands this will error at generate-time in Phase 6.

**Expected:** `serde_json::from_str::<Rule>(r#"{"style":"NotATier"}"#)` returns `Ok` (no error). The Phase 6 emitter will reject it when resolving the style reference against the `Styles` struct.

**Why human:** This is a conscious scope decision, not a bug — but the ROADMAP SC2 says "an unknown tier name is rejected with an actionable error," which is literally not true yet at the serde layer. A human decision-maker should confirm the Phase 6 deferred approach is intentional and acceptable for the Phase 5 gate.

---

### Gaps Summary

No blocking gaps. All condition types are modeled, validated, and parseable. The full test suite passes (123 lib tests, 9 cli_generate integration tests including `artifact_guard` and `generate_snapshot`). The emitter is byte-for-byte unchanged (confirmed by git diff and passing `artifact_guard`). Dead band types are cleanly removed.

The single human verification item is about a known, intentional scope boundary (unknown-style validation deferred to Phase 6) that is explicitly documented in both the CONTEXT.md and the PLAN scope guard. The ROADMAP SC2 wording is technically more ambitious than what Phase 5 delivers, but the PLAN and CONTEXT documents clarify that Phase 5 provides type-level support and Phase 6 adds generate-time validation.

---

_Verified: 2026-06-05T12:00:00Z_
_Verifier: Claude (gsd-verifier)_
