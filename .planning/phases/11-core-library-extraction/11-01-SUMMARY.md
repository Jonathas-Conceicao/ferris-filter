---
phase: 11-core-library-extraction
plan: 01
subsystem: core/compile-api
tags: [rust, thiserror, miette, api-design, refactor, error-handling]

requires:
  - phase: 06-emitter-rewrite-migration-parity
    provides: emit_rules — the pure rule compiler this API layers over
  - phase: 03-pricing-layer
    provides: derive_rates + price_tiers — the pricing steps load_inputs orchestrates
provides:
  - compile::load_inputs — stage one, all I/O, returns the price source used
  - compile::compile — stage two, pure, filter text from loaded inputs
  - compile::InputPaths — caller-supplied paths; the only place conventional filenames live
  - compile::CompileError — typed, matchable failure paths for non-CLI front ends
  - loader::LoadError — typed loader errors replacing miette::Result
affects: [12-dynamic-rule-store, 13-overlay-shell, 14-rule-editor]

tech-stack:
  added: []
  patterns:
    - "Two-stage pipeline: I/O + decisions in stage one, pure compute in stage two"
    - "Transparent error wrapping to preserve an existing CLI's rendered diagnostics"
    - "Outcome-as-value instead of library-side printing"

key-files:
  created:
    - src/compile.rs
  modified:
    - src/loader.rs
    - src/run.rs
    - src/lib.rs

key-decisions:
  - "InputPaths::conventional() (bare names) rather than relative_to(\".\") — the latter renders .\\config.json on Windows and changes every file-naming error message"
  - "ForcedFallbackMissing carries no #[diagnostic(code(...))] — the old miette!() macro emitted no code line, and adding one changed rendered stderr"
  - "The --early-game note is printed before load_inputs, gated on the flag, not derived from the returned PriceSource — it must still appear when the load then fails"
  - "LoadError lives in loader.rs and CompileError in compile.rs, beside their only consumers — matching the v0.1 decision that put PriceRateError in pricer.rs"

patterns-established:
  - "Transparent diagnostic wrapping: #[error(transparent)] + #[diagnostic(transparent)] lets a new typed enum wrap existing errors with zero change to rendered output"
  - "Capture a CLI behavior baseline from the pre-change binary before refactoring, then diff stdout/stderr/exit-code/artifact per case"
  - "Library unit tests build inputs in a temp dir from committed fixtures — never the git-ignored repo-root prices.json"

requirements-completed: [CORE-01, CORE-02, CORE-04]

duration: ~35min
completed: 2026-09-03
---

# Phase 11 Plan 01: Callable Compile API Summary

**The generate pipeline is now callable as a library API — two stages, caller-supplied paths, a typed error enum, and the price-source decision returned as a value — with the CLI proven byte-identical across six behavioral cases.**

## Performance

- **Duration:** ~35 min
- **Tasks:** 3 completed
- **Files modified:** 4 (1 created, 3 modified)
- **Tests:** 166 → 176 (10 added, 0 failures)

## Tasks Completed

| Task | Name | Commit | Key Changes |
|------|------|--------|-------------|
| 1 | Typed loader errors | `d1d36ce` | `LoadError` enum; three loader fns return `Result<_, LoadError>`; 8 call sites in run.rs adapted |
| 2 | Callable two-stage compile API | `4b92e8b` | New `src/compile.rs`: `InputPaths`, `PriceSourcePolicy`, `PriceSource`, `CompileInputs`, `CompileError`, `load_inputs`, `compile` + 9 unit tests |
| 3 | Rewire the CLI onto the API | `d20ae2f` | `cmd_generate` rebuilt on the API; parity proven across 6 cases; 2 corrections found by the baseline |

## What Was Built

### The blocker this removes

`run::cmd_generate` was private, hardcoded four CWD-relative paths, wrote its own output and
printed its own warnings. Any second front end would have had to reimplement the price-source
fallback ladder, rate derivation and tier pricing. It no longer does.

### Task 1 — `loader::LoadError`

Three variants: `Read { path, source }` (message still opens with ``Cannot read``, which
`tests/cli_errors.rs` asserts on), `Parse(JsonLoadError)` and `Tiers(TierValidationError)` — the
latter two `#[error(transparent)]` + `#[diagnostic(transparent)]`, which is what preserves
miette's source-span rendering.

`loader` turned out to be called **only from `run.rs`** (verified across `src/` and `tests/`), so
retyping it touched exactly one caller.

### Task 2 — `src/compile.rs`

- `InputPaths` — the single place the five conventional filenames are spelled. `conventional()`
  for bare relative names, `relative_to(root)` for a caller-supplied root, `with_config()` for
  `--config`.
- `PriceSourcePolicy { Auto, ForceFallback }` — the caller's intent, deliberately not a bare bool.
- `PriceSource { Live, Fallback(FallbackReason), Absent }` — what happened, returned not printed.
- `CompileError` — wraps `LoadError`, `PriceRateError` and `GenerateError` transparently, plus
  `ForcedFallbackMissing`.
- `load_inputs` (all I/O, the full fallback ladder lifted verbatim in behavior) and `compile`
  (pure wrapper over `emitter::generate`).

The two-stage split exists because a GUI compiles once per Apply but only needs to read
`prices.json` once — it can hold a `CompileInputs` and call `compile` repeatedly.

### Task 3 — CLI rewired

`cmd_generate` builds `InputPaths`, calls the two stages, and renders. It owns presentation only.

## Verification Results

**Full suite:** 176 passed, 0 failed (was 166). `artifact_guard`, `generate_snapshot`,
`generate_block_counts`, `parity_ingame`, `cli_errors`, `cli_filter_copy`, `sync_prices` all green.

**`git diff --exit-code tests/` → clean.** Not one fixture, snapshot or test file changed. This
was the phase's acceptance bar.

**CLI parity — six cases, byte-identical on stdout, stderr, exit code and generated filter:**

| Case | Scenario | Result |
|------|----------|--------|
| A | live prices | identical (27245 bytes) |
| B | `--early-game` | identical (18776 bytes) |
| C | prices absent, fallback present | identical + warning |
| D | no prices, no fallback | identical + both warnings |
| E | `--early-game`, fallback missing | identical (note + error, exit 1) |
| F | `--config alt.json` | identical |

Method: build the pre-change binary, run all six in a temp dir, save stdout/stderr/exit/artifact;
repeat after; diff. Hermetic per PITFALLS §5 — never `cargo run` from the repo root.

## Acceptance Criteria Check

- [x] CORE-01: a caller outside the CLI can load inputs and produce filter text through public
      functions with its own paths — the 9 `compile` unit tests do exactly this against temp dirs
- [x] CORE-02: no hardcoded CWD-relative **input** path in the compile pipeline (see Deviations
      for the two remaining literals, both writes in untouched code)
- [x] CORE-04: expected failures are distinct matchable variants; `PriceSource` is returned
- [x] `tests/cli_errors.rs` passes unmodified
- [x] Fixtures and snapshot unchanged
- [x] `generate` / `--early-game` / `--config` / `sync` / `install` behavior unchanged

## Deviations from Plan

Four, all recorded rather than silently absorbed:

1. **`InputPaths::conventional()` replaced the plan's `relative_to(".")`.** The plan specified
   `relative_to(".")` for the CLI. The captured baseline showed that renders as
   `.\prices-fallback.json` on Windows, changing every error message that names a file. Added
   `conventional()` (bare names, `relative_to("")`) and a test asserting no directory prefix.
   *Caught by the baseline, not by the test suite — no existing test would have failed.*

2. **`ForcedFallbackMissing` lost its `#[diagnostic(code(...))]`.** With it, miette printed an
   extra `Error: ferrisfilter::compile::forced_fallback_missing` line that the old
   `miette::miette!` macro never emitted. Case E caught it. The reason is documented on the
   variant so it does not get re-added.

3. **Task 2 acceptance criteria AC3/AC4 were mis-specified**, not the implementation. AC3 wanted
   zero `println!|eprintln!` matches in `compile.rs`; there is one — inside the module doc comment
   *stating that rule*. AC4 wanted exactly 4 filename literals; there are 21 — 5 in the `const`
   declarations and 16 in `#[cfg(test)]`, where tests legitimately name files they copy. Verified
   the intent holds: production code has zero print calls, and names the files only in the consts.

4. **One untested edge case now behaves differently.** Previously, when live prices were absent,
   the "falling back to prices-fallback.json" warning printed *before* the fallback was read — so
   a **malformed** (not missing) `prices-fallback.json` printed that warning and then errored. Now
   loading completes before any warning renders, so that path errors without the preceding
   warning. No test covers it; arguably an improvement (no misleading "falling back to X"
   immediately before "X is broken"); recorded because it is a real divergence from
   byte-identical.

## Issues Encountered

None blocking. The two behavioral corrections (deviations 1 and 2) were both surfaced by the
baseline-capture step rather than by tests — the suite passed at every point, including while
the error rendering was subtly wrong. Worth remembering: **for a refactor whose contract is
"observable behavior is unchanged", a green suite is not evidence.** The tests assert on
substrings; the divergences were in surrounding rendering.

## Next Steps

Plan 11-02 (wave 2): retire the `seed_prices` duplicated structs and the stale `[lib]` comment.
