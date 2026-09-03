---
phase: 08-sync-pipeline
reviewed: 2026-06-23T00:00:00Z
depth: standard
files_reviewed: 11
files_reviewed_list:
  - src/sync/prices.rs
  - src/sync/mapping.rs
  - src/sync/metadata.rs
  - src/pricer.rs
  - src/emitter.rs
  - tests/sync_prices.rs
  - tests/cli_generate.rs
  - tests/fixtures/seed-prices.json
  - tests/fixtures/seed-universal.filter
  - tests/snapshots/cli_generate__generate_snapshot.snap
  - mapping.json
findings:
  critical: 0
  warning: 4
  info: 4
  total: 8
status: issues_found
---

# Phase 8: Code Review Report

**Reviewed:** 2026-06-23
**Depth:** standard
**Files Reviewed:** 11
**Status:** issues_found

## Summary

Phase 8 delivers the rewritten sync pipeline: per-level uncut gem BaseType dispatch,
`gemFilters` array schema (mapping.json v2), derive_uncut_class prefix resolution,
multi-class UNCUT_GEM_MULTICLASS emission, and the stash/item unique endpoint. The
core data path is correct and the artifact guard + snapshot tests confirm the generated
filter is byte-stable against the oracle. No critical (security/crash/data-loss)
issues found.

Four warnings are raised: a behavioral gap in the FakeClient happy-path test (the
`happy_path()` factory is defined but unused in the test file, meaning the actual
happy-path tests drive `build_prices` directly and bypass `run_sync_prices`), a
category-name hardcoding in `pricer.rs` that is fragile to mapping.json drift, a
missing guard that allows LineageSupportGems items to silently land with an empty class
if `gf.classes` is accidentally cleared, and a `derive_uncut_class` edge case for
future gem naming patterns. Four info items cover dead code, a magic literal, naming,
and a test coverage gap.

No scope creep was found: background emission is correctly deferred (the field is
stored but the emitter ignores `background_override` in Phase 8), and no `corrupted`
condition appears.

---

## Warnings

### WR-01: `happy_path()` FakeClient factory never used — tests bypass `run_sync_prices`

**File:** `tests/sync_prices.rs:48`

**Issue:** `FakeClient::happy_path()` is defined (lines 48-81) but the two "happy path"
integration tests (`happy_path_anchors_first_and_correct_values`, line 176, and
`happy_path_writes_schema_correct_prices_json`, line 290) call `build_prices` directly
instead. The network-dispatching `run_sync_prices` path — which is where the
`gem_filters` loop, the stash/item endpoint, and anchor validation all interact — has
no integration-test coverage in this file. If `run_sync_prices` regresses (e.g. by
reversing the order of overviewTypes vs gemFilters fetches, or silently skipping the
stash endpoint), nothing here would catch it.

Additionally, the `all_empty()` and `network_error_for_overview()` factories (lines
84-101) are also never called.

**Fix:** Either remove the unused factories (and rename the tests to make their direct
scope clearer), or add one async integration test that drives `run_sync_prices` through
the FakeClient. A minimal version:

```rust
#[tokio::test]
async fn run_sync_prices_happy_path_writes_prices_json() {
    let client = FakeClient::happy_path();
    let mapping = ferrisfilter::sync::mapping::load_mapping("mapping.json").unwrap();
    // ... drive run_sync_prices with a temp prices.json path
    // assert file written, items[0]="Exalted Orb", gems present
}
```

At minimum, suppress the dead-code lint or document why the factories are intentionally
left for future use.

---

### WR-02: "Uncut Gems" category name hardcoded in `pricer.rs` — fragile to mapping.json drift

**File:** `src/pricer.rs:343`

**Issue:** `price_tiers` uses a string literal to detect the special merge case:

```rust
let is_uncut_gems = cat == "Uncut Gems";
```

This category name originates from `GemFilter.category` in `mapping.json`
(`"category": "Uncut Gems"`). If the category value is ever renamed in `mapping.json`
(e.g. to "Gems" or "Uncut"), the three subtype classes would silently stop being merged
into a single `TierGroup` and would instead emit three separate `Class ==` blocks —
one per subtype. The generated filter would still be syntactically valid, but each
block would use a single-class line instead of the intended multi-class line, and the
`artifact_guard` test would catch it only after the mapping changes.

The Phase 9 deferred work (background emission) also uses the category name at
`category_background` (line 50-56), compounding the brittleness.

**Fix:** Define a const for this sentinel value and reference both sites from it, or
derive the merge condition from a field on `GemFilter` (e.g. a boolean `merge_classes:
true`):

```rust
const UNCUT_GEMS_CATEGORY: &str = "Uncut Gems";
// ...
let is_uncut_gems = cat == UNCUT_GEMS_CATEGORY;
```

Or better, model the intent explicitly in `GemFilter` so the pricer does not need to
know category names:

```json
{ "type": "UncutGems", "mergeClasses": true, ... }
```

---

### WR-03: `derive_uncut_class` silently returns `None` for any unknown uncut gem prefix — falls through to `gf.classes[0]` which may be wrong

**File:** `src/sync/prices.rs:185-201`

**Issue:** When `derive_uncut_class` returns `None` for a gem item, the code falls back
to `gf.classes.first()`. For the `UncutGems` gem filter, `classes[0]` is `"Uncut Skill
Gems"`. If poe.ninja ever introduces a new uncut subtype (e.g. `"Uncut Aura Gem
(Level 20)"`) whose prefix is not yet in the match table, that item would silently
receive class `"Uncut Skill Gems"` instead of a derived `"Uncut Aura Gems"`. In the
emitter this means the item lands in the merged multi-class block (which is correct),
but its `TieredItem.class` field is wrong — it says `"Uncut Skill Gems"` for an aura
gem. Phase 9 code that reads `TieredItem.class` for per-subtype background logic would
misattribute the item.

For the `LineageSupportGems` filter, `gf.classes[0]` is `"Support Gems"`, and none of
the Lineage gem names start with `"Uncut "`, so `derive_uncut_class` always returns
`None` for them. This is intentional and correct today. The gap is only the
`UncutGems` case with future subtypes.

**Fix:** When `gf.r#type == "UncutGems"` and `derive_uncut_class` returns `None`, emit
a `Warning` that names the unrecognized prefix:

```rust
None => {
    if gf.r#type == "UncutGems" {
        eprintln!(
            "Warning: unrecognized uncut gem prefix in '{}'; \
             update derive_uncut_class — using fallback class '{}'",
            name,
            gf.classes.first().map(|s| s.as_str()).unwrap_or("(none)")
        );
    }
    match gf.classes.first() { ... }
}
```

---

### WR-04: Multi-class emitter check uses `contains('"')` — fragile against single-class values that contain a quote

**File:** `src/emitter.rs:406`

**Issue:** The emitter distinguishes a pre-formatted multi-class string from a normal
single-class value with:

```rust
if tier_group.class.contains('"') {
    writeln!(out, "  Class == {}", tier_group.class).unwrap();
} else {
    writeln!(out, "  Class == \"{}\"", tier_group.class).unwrap();
}
```

Today this is safe because single-class values like `"Stackable Currency"` and
`"Support Gems"` never contain a literal `"` character. However, the check is
semantically backwards — it tests for the presence of quotes as a sentinel for
"already formatted", rather than using a typed signal. If any future single-class name
ever contained a quote (unusual but possible in filter grammar for edge cases), it
would be emitted without wrapping quotes and produce a malformed filter line.

**Fix:** Use a typed signal at the pricer→emitter boundary. The cleanest option is a
dedicated enum or wrapper type:

```rust
pub enum ClassValue {
    Single(String),           // wrapped in quotes by emitter
    MultiPreFormatted(String), // emitted verbatim
}
```

Alternatively, since `UNCUT_GEM_MULTICLASS` is a compile-time constant, check for
pointer equality or a dedicated boolean on `TierGroup`. Any of these eliminates the
fragile heuristic without changing the emitted output.

---

## Info

### IN-01: `base_type_is_name` field is read but unused in the currency path

**File:** `src/sync/prices.rs:224`

**Issue:** The `base_type_is_name` field is destructured from `mapping.lookup()` at
line 117 and assigned to a local, but immediately discarded at line 224:

```rust
let _ = base_type_is_name;
let base_type = name.clone();
```

For the currency path `base_type == name` is always unconditionally true (the variable
is treated as a constant). The field is now vestigial — it was meaningful before the
model unified uniques under a separate stash endpoint. A comment acknowledging this
would prevent future confusion.

**Fix:** Either remove the field from `OverviewType` and the match destructure, or
replace `let _ = base_type_is_name;` with a descriptive comment explaining why it is
always true for overviewTypes now that uniques have their own pipeline.

---

### IN-02: `sync_prices.rs` FakeClient matches poe.ninja type values using stale singular spellings in comment

**File:** `tests/sync_prices.rs:62-78`

**Issue:** The `happy_path()` factory comment labels entries as `"UniqueWeapon"`,
`"UniqueArmour"`, `"UniqueAccessory"`, etc. (singular). However, the real `mapping.json`
`itemTypes` array uses plural values (`UniqueWeapons`, `UniqueArmours`, etc.), which the
`mapping_contains_all_unique_item_types` guard test enforces. Since `happy_path()` is
itself unused (WR-01 above), this mismatch has no runtime effect, but it is misleading
documentation.

**Fix:** Update the factory to use plural type keys matching `mapping.json` so it is
correct if/when the factory is activated.

---

### IN-03: `format_utc_timestamp` / `days_to_ymd` are non-trivial std-only calendar math with no unit tests

**File:** `src/sync/prices.rs:548-601`

**Issue:** The ISO-8601 timestamp formatter reimplements calendar logic (year/month/day
from epoch days, including leap-year handling) in ~50 lines of pure std code. There are
no dedicated unit tests for this helper. The `days_to_ymd` function carries a subtle
assumption: `days + 1` at line 597 assumes `days` was exhausted by subtracting month
lengths correctly. A year-boundary or leap-year edge case error here would produce
malformed timestamps silently (the only consumer is the `fetched_at` string, which is
informational only and not validated on load). The risk is low but the function is
untested.

**Fix:** Add a small `#[test]` block covering at least: epoch (1970-01-01T00:00:00Z),
a known date like 2026-06-23T12:00:00Z, a leap-year day (2000-02-29), and a
non-leap century year (1900-03-01 is the classic off-by-one trap).

---

### IN-04: `derive_block_label` has no case for "Uncut Gems" or "Lineage Support Gems"

**File:** `src/emitter.rs:488-508`

**Issue:** `derive_block_label` falls through to the default `"{category_name} {tier}"`
pattern for both "Uncut Gems" and "Lineage Support Gems". This is actually correct
(the generated oracle confirms `Show # Uncut Gems S`, `Show # Lineage Support Gems B`,
etc.), but the function's doc-comment only documents exceptions. A reader maintaining
this function might add spurious cases for these categories based on the incomplete
list, breaking the oracle without knowing it.

**Fix:** Add a comment in the function body (or in the doc-comment list) explicitly
noting that "Uncut Gems" and "Lineage Support Gems" fall to the default and produce
`"{category} {tier}"` — intentional, no override needed.

---

_Reviewed: 2026-06-23_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
