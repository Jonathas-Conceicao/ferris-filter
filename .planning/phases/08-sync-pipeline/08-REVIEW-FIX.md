---
phase: 08-sync-pipeline
fixed_at: 2026-06-23T00:00:00Z
review_path: .planning/phases/08-sync-pipeline/08-REVIEW.md
iteration: 1
findings_in_scope: 4
fixed: 4
skipped: 0
status: all_fixed
---

# Phase 8: Code Review Fix Report

**Fixed at:** 2026-06-23
**Source review:** .planning/phases/08-sync-pipeline/08-REVIEW.md
**Iteration:** 1

**Summary:**
- Findings in scope: 4 (WR-01 through WR-04; IN-01..IN-04 out of scope)
- Fixed: 4
- Skipped: 0

All four warnings were applied without changing any emitted filter output. The full
`cargo test` suite (152 tests + 1 doc-test) passes green after every commit.

## Fixed Issues

### WR-01: `happy_path()` FakeClient factory never used — tests bypass `run_sync_prices`

**Files modified:** `tests/sync_prices.rs`
**Commit:** b0ffbdf
**Applied fix:** Added `#[allow(dead_code)]` to all three unused factory methods
(`happy_path`, `all_empty`, `network_error_for_overview`) with a doc comment on
`happy_path` explaining that the factories are reserved for a future `run_sync_prices`
integration test, and noting the type-key mismatch (singular vs plural spellings) that
must be corrected when that test is added. This is the "at minimum" option from the
reviewer — suppresses the lint and documents intent without adding an async test that
would require non-trivial I/O plumbing (`load_mapping("mapping.json")` hardcodes a
filesystem path, `write_prices_atomic` writes to CWD). A proper async end-to-end test
is deferred as a follow-up.

---

### WR-02: "Uncut Gems" category name hardcoded in `pricer.rs` — fragile to mapping.json drift

**Files modified:** `src/pricer.rs`
**Commit:** 378691d
**Applied fix:** Introduced `const UNCUT_GEMS_CATEGORY: &str = "Uncut Gems";` with a
doc comment explaining its origin in `GemFilter.category` and why a named const is
preferred over scattered literals. Updated both usage sites: `category_background`
(the Phase 9 background lookup) and `price_tiers` (the merge-case detection). No
emitted output change; the `artifact_guard` and `generate_snapshot` tests confirm
the filter is byte-stable.

---

### WR-03: `derive_uncut_class` silently returns `None` for unknown uncut gem prefix

**Files modified:** `src/sync/prices.rs`
**Commit:** 44fb958
**Applied fix:** In the `None` arm of the `derive_uncut_class` match (lines 185-202),
added an `if gf.r#type == "UncutGems"` guard that emits a `Warning:` message via
`eprintln!` naming the unrecognized base-type and the fallback class that will be used.
The message format is consistent with the other `Warning:` messages in the file. The
fallback path (`gf.classes.first()`) is unchanged, so known Skill/Spirit/Support prefixes
continue to produce correct output. `LineageSupportGems` items are unaffected because
`gf.r#type` is `"LineageSupportGems"` (not `"UncutGems"`) — the warning is suppressed
for them, as `None` from `derive_uncut_class` is correct for Lineage gems.

---

### WR-04: Multi-class emitter check uses `contains('"')` — fragile against edge cases

**Files modified:** `src/pricer.rs`, `src/emitter.rs`
**Commit:** 6d3e044
**Applied fix:** Added `is_multi_class: bool` field to `TierGroup` (with a doc comment
explaining its purpose). The Uncut Gems merge path in `price_tiers` sets
`is_multi_class: true`; all regular single-class paths set `is_multi_class: false`. The
emitter's class-emission branch now reads `tier_group.is_multi_class` instead of the
fragile `tier_group.class.contains('"')` heuristic. This is a clean typed signal at
the pricer→emitter boundary. No emitted output change — the `artifact_guard` test
confirms byte-stability.

Note: the doc comment on `UNCUT_GEM_MULTICLASS` was also updated to reference
`is_multi_class` rather than the now-removed `contains('"')` check.

## Skipped Issues

None.

---

_Fixed: 2026-06-23_
_Fixer: Claude (gsd-code-fixer)_
_Iteration: 1_
