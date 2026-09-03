---
phase: 11-core-library-extraction
phase_name: Callable Compile API
verified: 2026-09-03
status: passed
scores:
  success_criteria: 8/8
  requirements: 4/4
  plans_executed: 2/2
gaps: []
---

# Phase 11 Verification — Callable Compile API

**Status: PASSED** — 8/8 success criteria, 4/4 requirements, 2/2 plans.

## Goal Achievement

**Phase goal:** the generate pipeline is callable as a library API — caller-supplied paths, a
typed error enum, and the price-source decision returned as a value — so the Phase 13/14 overlay
can drive it without duplicating orchestration, depending on the working directory, or needing
stdout.

**Achieved.** `compile::load_inputs` + `compile::compile` are public, take an `InputPaths` the
caller constructs, return `CompileError` variants a caller can match on, and report the price
source as `CompileInputs::price_source`. The library prints nothing. The CLI is now one of two
possible front ends rather than the only one.

## Success Criteria

| # | Criterion | Result | Evidence |
|---|-----------|--------|----------|
| 1 | A caller outside the CLI can load inputs and produce filter text through public library functions with its own paths, without reimplementing the fallback, rate derivation or tier pricing | ✅ | 10 `compile` unit tests drive the full pipeline against temp dirs; `compile_produces_filter_text_from_loaded_inputs` goes end-to-end |
| 2 | No library code reads a hardcoded working-directory-relative input path | ✅ | The five filenames appear only as `const`s inside `InputPaths::relative_to`. Remaining repo matches are the CLI's clap `--config` default (caller-side, intended), test assertions, and two **write** paths in `sync`/`seed_prices` — untouched code, and outputs not inputs |
| 3 | Expected failure paths are distinct matchable variants of one error type | ✅ | `LoadError::{Read, Parse, Tiers}` and `CompileError::{Load, Rates, Generate, ForcedFallbackMissing}` — covering unreadable config, malformed config, malformed prices, unreadable/malformed/invalid tiers, reference-rate failure, unknown style, and missing forced fallback |
| 4 | `tests/cli_errors.rs` passes unmodified, with miette source spans intact | ✅ | 4/4 pass; `git diff --exit-code tests/cli_errors.rs` clean. `Cannot read`, `config.json`, `styles.S.font`, `expected u8`, `missing required tier`, `not strictly ascending` all still render |
| 5 | The price source used is available to the caller as a value; the CLI derives its warnings from it and its stderr is unchanged | ✅ | `CompileInputs::price_source`; `run.rs` matches on it. Cases B/C/D/E stderr byte-identical |
| 6 | `artifact_guard` and `generate_snapshot` pass with zero changes to the fixture or snapshot; full suite passes | ✅ | 176 passed / 0 failed. `git diff --exit-code tests/` clean — **not one byte moved** |
| 7 | `generate`, `--early-game`, `--config`, `sync`, `install` produce the same output, stderr and exit codes | ✅ | 6-case parity harness, byte-identical on stdout, stderr, exit code and generated filter. `sync`/`install` untouched and their suites green |
| 8 | `seed_prices` imports the real types; its five duplicate structs and the stale `[lib]` comment are gone | ✅ | 2 `use ferrisfilter::models` lines; 0 duplicate structs; 0 stale claims; output byte-identical (77,374 bytes) |

## Requirements Coverage

| ID | Requirement | Plan | Status |
|----|-------------|------|--------|
| CORE-01 | Pipeline callable through public library functions | 11-01 | ✅ Complete |
| CORE-02 | Every input path comes from the caller | 11-01 | ✅ Complete |
| CORE-03 | `seed_prices` duplication removed | 11-02 | ✅ Complete |
| CORE-04 | Typed error enum + price source as a value | 11-01 | ✅ Complete |

## Test Results

```
176 passed / 0 failed
  lib             146  (was 136 — 10 added in src/compile.rs)
  cli_errors        4
  cli_filter_copy   4
  cli_generate     13   (artifact_guard, generate_snapshot,
                         generate_block_counts, parity_ingame)
  sync_prices       8
  doc               1
```

Pre-existing warnings (carried from the v0.3 audit, not introduced here): unreachable match arm
in the emitter unknown-style test; unused `FakeClient::ok` in `sync/metadata.rs`.

## Integration Check

The Phase 12 dynamic compile and the Phase 14 Apply action both need to compile a filter from a
caller-chosen location with no stdout and matchable errors. All three are now available:

- `InputPaths::relative_to(root)` — an overlay with an arbitrary working directory
- `CompileInputs` held across repeated `compile()` calls — one price read per session, not per Apply
- `CompileError` variants — a GUI can render each failure its own way

The `config.json → universal.filter` path is untouched and still oracle-locked, so the dynamic
path can be built additively as the milestone's acceptance bar requires.

## Deviations Carried Forward

Five recorded across the two plans. Two are real behavioral corrections that **no test would have
caught** — worth remembering:

1. `InputPaths::relative_to(".")` (as planned) renders `.\config.json` on Windows, changing every
   error message naming a file. Replaced with `conventional()`; guarded by a test.
2. A `#[diagnostic(code(...))]` on `ForcedFallbackMissing` made miette emit a code line the old
   `miette!()` macro never did. Removed; the reason is documented on the variant.

Both were found by the before/after CLI baseline, while the suite was green throughout.

3. Plan 11-01 task 2's criteria AC3/AC4 were mis-specified as naive grep counts (they matched a
   doc comment and the test module). Implementation satisfied the intent.
4. An untested edge case changed: a *malformed* (not missing) `prices-fallback.json` no longer
   prints the "falling back to prices-fallback.json" warning before erroring. Arguably an
   improvement; recorded as a real divergence.
5. Plan 11-02 deliberately added no regression test — the duplication is structurally prevented
   by import collision, and the natural home for such a test is `src/models/`, which that plan's
   verification block required stay untouched.

## Lesson

**For a refactor whose contract is "observable behavior is unchanged", a green suite is not
evidence.** The suite passed at every commit, including while the error rendering was subtly
wrong in two places. Capturing a behavioral baseline from the pre-change binary — stdout, stderr,
exit code and artifact, per case — is what actually verified the claim. That harness is worth
reaching for again in Phase 12, where `dynamic.filter` must match `universal.filter`'s economy
sections.

---

*Verified 2026-09-03. Phase 11 complete.*
