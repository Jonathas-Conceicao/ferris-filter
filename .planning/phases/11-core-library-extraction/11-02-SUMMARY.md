---
phase: 11-core-library-extraction
plan: 02
subsystem: tooling/seed
tags: [rust, deduplication, cleanup, stale-docs]

requires:
  - phase: 11-core-library-extraction
    provides: plan 01 landed first (wave 2 sequencing — both plans assert a green suite on a shared tree)
provides:
  - seed_prices backed by the library models, with no duplicated struct definitions
  - removal of the stale "crate has no [lib] target" claim
affects: []

tech-stack:
  added: []
  patterns:
    - "Delete a stale architecture note outright rather than softening it — a wrong note costs more than no note"

key-files:
  created: []
  modified:
    - src/bin/seed_prices.rs

key-decisions:
  - "No extra regression test: re-adding a local struct PriceItem would collide with the import and fail to compile, so the duplication is structurally prevented"
  - "No library type was widened — the types were drop-in, so D-11's adjust-the-binary escape hatch went unused"

patterns-established:
  - "Prove a refactor of a generator is behavior-preserving by diffing its output before and after against identical input, captured before any edit"

requirements-completed: [CORE-03]

duration: ~8min
completed: 2026-09-03
---

# Phase 11 Plan 02: seed_prices Deduplication Summary

**`seed_prices` now imports the real models from the library, its five mirror structs and the false "crate has no `[lib]` target" note are gone, and its output is byte-identical.**

## Performance

- **Duration:** ~8 min
- **Tasks:** 2 completed
- **Files modified:** 1
- **Tests:** 176 passing, 0 failures (unchanged — this plan adds none)

## Tasks Completed

| Task | Name | Commit | Key Changes |
|------|------|--------|-------------|
| 1 | Import library models and delete the duplicates | `9e6f646` | 5 structs → 2 `use` lines; stale architecture note deleted |
| 2 | Prove the output is unchanged | `9e6f646` | Before/after diff on identical input: byte-identical |

## What Was Built

### The duplication

`src/bin/seed_prices.rs` carried local mirrors of `CurrencyEntry`, `CurrencyCategory`,
`CurrencyData`, `PricesFile` and `PriceItem`, under two "Minimal local mirror of …" banners.
They are replaced by:

```
use ferrisfilter::models::currency::CurrencyData;
use ferrisfilter::models::prices::{PriceItem, PricesFile};
```

The types were genuinely drop-in — identical field names and types, and the required derives
(`Deserialize` on the currency side, `Serialize` on the prices side) already present. **No library
type was widened**, so 11-CONTEXT.md D-11's "adjust the binary rather than the library" escape
hatch went unused. `CurrencyEntry` and `CurrencyCategory` turned out to be reachable through
`CurrencyData` without being named directly, so only three types are imported rather than five —
no dead imports.

### The stale note

The module doc carried a "Design choice (architecture note)" paragraph asserting:

> the crate has no `[lib]` target (only `[[bin]]` targets), so `use ferrisfilter::...` is not
> available to other binaries

This has been false for some time — `src/main.rs` and `src/bin/ff.rs` both do exactly that. It was
**one of the two sources** (with a v0.1 STATE.md decision note) that led this milestone's roadmap
and research to believe there was no library target, which is why Phase 11 had to be re-scoped
during `/gsd-discuss-phase`. It is deleted, not softened, and replaced with a two-line `## Models`
note saying where the models come from.

## Verification Results

- **Output byte-identical:** 77,374 bytes produced from the same `currency.json` before and after,
  `diff` empty, stdout also identical. The baseline was captured *before* any edit, then replayed
  after — both runs hermetic (temp dir, binary by absolute path), per PITFALLS §5.
- **`cargo build --bin seed_prices`:** clean, zero warnings, no unused imports.
- **Full suite:** 176 passed, 0 failed.
- **`git diff --exit-code tests/ src/models/`:** clean. This plan touched exactly one file.

## Acceptance Criteria Check

- [x] CORE-03: `seed_prices` imports the real types; all five duplicate definitions gone
- [x] `grep -c "Minimal local mirror"` → 0
- [x] `grep -c 'no \`[lib]\` target'` → 0
- [x] Build clean, no unused-import warnings
- [x] Output byte-identical on identical input
- [x] Neither `tests/` nor `src/models/` modified

## Deviations from Plan

One, and it is a deliberate non-action:

**No serde round-trip regression test was added.** The plan offered it as optional ("if it is
genuinely cheap"), but two things argue against it. First, the duplication is now *structurally*
prevented — re-declaring a local `struct PriceItem` would collide with the import and fail to
compile, which is a stronger guarantee than a test. Second, the natural home for such a test is
`src/models/prices.rs`, and this plan's own verification block requires `src/models/` stay
untouched. The module-level doc line the plan also asked for is in place and is the proportionate
guard for a one-time tool.

## Issues Encountered

None. The compatibility table in the plan had been verified during planning rather than left for
execution, so this ran without exploration.

## Next Steps

Phase 11 is complete — both plans executed. Next is phase verification, then Phase 12 (Dynamic
Rule Store) or Phase 13 (Overlay Shell), which both depend only on Phase 11 and can proceed in
either order.
