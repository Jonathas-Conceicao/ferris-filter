---
phase: 05-rule-schema
reviewed: 2026-06-05T00:00:00Z
depth: standard
files_reviewed: 2
files_reviewed_list:
  - src/models/config.rs
  - config.json
findings:
  critical: 0
  warning: 1
  high: 0
  medium: 1
  low: 2
  nit: 2
  total: 5
status: issues_found
---

# Phase 05: Code Review Report

**Reviewed:** 2026-06-05  
**Depth:** standard (with focused edge-case tracing on `RarityComparator`)  
**Files Reviewed:** 2 (`src/models/config.rs`, `config.json`)  
**Status:** issues_found — one correctness bug (panic on non-ASCII input), one semantic gap (empty-object no-op), two low/nit items. No blockers; the schema is sound and all 123 tests pass.

---

## Build and Test Status

- `cargo build` — clean, zero errors.
- `cargo clippy` — zero errors in `config.rs` new code; existing `uninlined_format_args` warnings in `config.rs` lines 203–204, 217–219, 238–239 are pre-existing style nits from the rest of the codebase, not introduced bugs.
- `cargo test` — 123/123 passing including all new `rarity_comparator_*`, `numeric_condition_*`, `area_level_range_*`, `rule_*`, and `when_*` tests.

---

## Summary

Phase 5 added the v0.2 Declarative Rule Model data types to `src/models/config.rs` and removed the dead `areaLevelBands` field + its four associated types from both the config and the Rust model. The removal is clean — no dangling references exist anywhere in the codebase (`AreaLevelBand`, `BandShow`, `ArmourShow`, `RarityRule` are fully gone from source and JSON).

The `RarityComparator` hand-written `Deserialize`/`Serialize` impl is largely correct: operator dispatch is sound, round-trips are correct, all tested operators and rarity words behave as documented, and the `>` < `!` single-char operators are properly rejected. One correctness issue was found: a direct byte-slice `s[..2]` that panics on multi-byte UTF-8 input instead of returning a serde error. Two semantic issues were identified that require attention at Phase 6 emit time: empty-object degenerate `NumericCondition`/`AreaLevelRange` values, and a misleading error message for lowercase rarity names.

---

## Medium Issues

### MD-01: `s[..2]` panics on 3-or-4-byte UTF-8 characters in RarityComparator deserializer

**File:** `src/models/config.rs:200-201`  
**Severity:** MEDIUM  
**Issue:** The fallback operator-detection branch guards on `s.len() >= 2` (byte length) then unconditionally indexes `&s[..2]`. This panics with `byte index 2 is not a char boundary` when the trimmed string starts with any Unicode character encoded in 3 or 4 bytes (U+0800 and above: CJK ideographs, emoji, etc.). The `strip_prefix` calls earlier all match on ASCII literal strings — they correctly handle multi-byte input without panicking — but the fallback branch does not inherit that safety.

The panic replaces what should be a clean serde `Err`. For a CLI tool loading a user-owned config file this has no security impact, but it produces an ugly backtrace rather than a fieldpath-annotated error from `serde_path_to_error`.

```rust
// CURRENT (panics on CJK/emoji input):
if s.len() >= 2 {
    let prefix = &s[..2];  // ← panics if first char is >= 3 bytes wide
    if !matches!(prefix, "No" | "Ma" | "Ra" | "Un") { ... }
}

// FIX — use .get() which returns None on invalid char boundaries:
if let Some(prefix) = s.get(..2) {
    if !matches!(prefix, "No" | "Ma" | "Ra" | "Un") { ... }
}
// No guard on s.len() needed — s.get(..2) handles all lengths safely.
```

The `.get(..2)` alternative returns `None` when byte index 2 is not a char boundary (and when the string is shorter than 2 bytes), so the `if let` naturally skips the check for multi-byte-first-char inputs. Those strings will reach the `rarity_str` match and produce a clean "unknown rarity" error.

---

## Low Issues

### LW-01: Misleading diagnostic for lowercase rarity names

**File:** `src/models/config.rs:198-207`  
**Severity:** LOW  
**Issue:** Input like `"rare"` or `"magic"` (valid words, wrong case) trips the operator-prefix guard rather than the rarity-word match. The error says `unknown rarity operator prefix "ra" in "rare"` — which implies the user typed an unknown operator, when the actual problem is capitalisation. The message `"expected ==, >=, <= or a bare rarity word"` does not explain that rarity words are case-sensitive or hint at the correct spelling.

This is a pure diagnostic quality issue — the error fires correctly and no incorrect parse occurs.

**Fix:** After the operator-prefix check rejects the prefix, add a case-insensitive secondary check to detect the "probably meant a rarity word, just wrong case" scenario:

```rust
// After the prefix guard rejects `prefix`, additionally check:
let up = s.to_ascii_uppercase();
let maybe_rarity = matches!(up.as_str(), "NORMAL" | "MAGIC" | "RARE" | "UNIQUE");
if !matches!(prefix, "No" | "Ma" | "Ra" | "Un") {
    let hint = if maybe_rarity {
        format!("; rarity names are case-sensitive — did you mean \"{}{}\"?",
            &s[..1].to_uppercase(), &s[1..])
    } else { String::new() };
    return Err(de::Error::custom(format!(
        "unknown rarity operator or word {:?}{}; \
         expected ==, >=, <= followed by Normal/Magic/Rare/Unique, or a bare rarity word",
        s, hint
    )));
}
```

(Or simply change the message to say "rarity names are case-sensitive: Normal, Magic, Rare, Unique" without trying to guess the correction, which is simpler.)

---

## Nit Issues

### NT-01: Empty-object `NumericCondition` / `AreaLevelRange` deserialise to silent no-ops

**File:** `src/models/config.rs:255-275`  
**Severity:** NIT (semantic trap for Phase 6)  
**Issue:** Both `NumericCondition` and `AreaLevelRange` have all fields as `Option`, so an empty JSON object `{}` deserialises successfully to an all-`None` struct. A `when` that contains `"baseEvasion": {}` or `"areaLevel": {}` will parse without error but emit zero filter conditions for that field — it acts as a no-op, which is surprising.

This is not a parse bug but a semantic trap for Phase 6's emitter: it must guard against all-`None` conditions and either warn or treat them as absent. Consider documenting this explicitly on the structs:

```rust
/// Note: an all-`None` value (from an empty `{}` in JSON) is semantically a no-op —
/// it emits no conditions. The Phase 6 emitter should warn or reject this case.
pub struct NumericCondition { ... }
```

No code change required now; the note prevents a silent emitter bug later.

---

### NT-02: `uninlined_format_args` clippy warnings in new code

**File:** `src/models/config.rs:203-206, 217-220, 238-239`  
**Severity:** NIT  
**Issue:** Three `format!(...)` calls in the new `RarityComparator` impl trigger `clippy::uninlined_format_args`. These are style nits consistent with the rest of the codebase (identical warning pattern exists throughout `emitter.rs`), not bugs. No behavioural impact. The `cargo clippy` output already shows the suggested inline-arg rewrites.

---

## Scope Confirmation: Dead-Type Removal

The removal of `area_level_bands: Vec<AreaLevelBand>` from `Config`, and the deletion of `RarityRule`, `ArmourShow`, `BandShow`, and `AreaLevelBand`, is clean:

- `config.json` — `areaLevelBands` key removed; file parses correctly against the updated model.
- All callers — `src/emitter.rs`, `src/run.rs`, `src/sync/prices.rs` — never imported these types and are unaffected.
- No `pub use` re-exports of the dead types existed.
- `cargo build` and `cargo test` confirm zero compile errors and zero test regressions.

The four removed test functions (`rarity_rule_*`) are gone with their types; no orphaned test stubs remain.

---

## Expressibility of Current Filter Conditions

The `When` struct can represent all the filter conditions used by the existing hand-authored filter:

| Filter line | `When` field | Encoding |
|---|---|---|
| `BaseEvasion 0` | `base_evasion: { eq: 0 }` | `NumericCondition { eq: Some(0) }` |
| `BaseEnergyShield > 0` | `base_energy_shield: { min: 1 }` | `NumericCondition { min: Some(1) }` — correct for integer attrs |
| `BaseArmour 0` | `base_armour: { eq: 0 }` | `NumericCondition { eq: Some(0) }` |
| `Quality >= 1` | `quality: { min: 1 }` | `NumericCondition { min: Some(1) }` |
| `Sockets >= 1` | `sockets: { min: 1 }` | `NumericCondition { min: Some(1) }` |
| `AreaLevel <= 25` | `area_level: { max: 25 }` | `AreaLevelRange { max: Some(25) }` |
| `Rarity >= Magic` | `rarity: ">=Magic"` | `RarityComparator { Gte, Magic }` |
| `Class == "Staves"` | `class: ["Staves"]` | `Vec<String>` |

All conditions in the design spec can be expressed. The `Identified` boolean condition is absent from `When` — this is intentional (scope of Phase 6+) and noted in the doc comments that `style: "hidden"` / `"unchanged"` semantics are deferred.

---

_Reviewed: 2026-06-05_  
_Reviewer: Claude (gsd-code-reviewer)_  
_Depth: standard_
