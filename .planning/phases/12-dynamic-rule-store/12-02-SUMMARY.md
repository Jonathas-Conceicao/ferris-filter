---
phase: 12-dynamic-rule-store
plan: 02
subsystem: core/dynamic-lifecycle
tags: [rust, cli, atomic-write, persistence, integration-tests]

requires:
  - phase: 12-dynamic-rule-store
    provides: plan 12-01's Rule.enabled and compile_as/FilterIdentity
  - phase: 11-core-library-extraction
    provides: load_inputs + compile — the dynamic path reuses them unchanged
provides:
  - src/dynamic.rs — seed, reset, compile and atomic write for the dynamic store
  - ferrisfilter apply — the headless end-to-end dynamic flow
  - tests/dynamic_filter.rs — integration coverage for the phase's criteria
affects: [13-overlay-shell, 14-rule-editor]

tech-stack:
  added: []
  patterns:
    - "Atomic write with the temp file in the destination directory, because the game folder is on another volume"
    - "Seed-if-absent kept strictly separate from reset, so the destructive path is always deliberate"

key-files:
  created:
    - src/dynamic.rs
    - tests/dynamic_filter.rs
  modified:
    - src/cli.rs
    - src/run.rs
    - src/lib.rs
    - src/install.rs
    - .gitignore
    - CLAUDE.md

key-decisions:
  - "Named the subcommand `apply` — it compiles and installs in one step, unlike generate which only compiles"
  - "install::default_dest made pub(crate) and delegated to, so exactly one place knows where PoE2 keeps filters"
  - "apply's price-source warnings reuse cmd_generate's exact wording rather than a second spelling of the same condition"

patterns-established:
  - "Integration assertions compare two generated artifacts against each other rather than against hand-written substrings"

requirements-completed: [DYN-03, DYN-06, DYN-07, DYN-08]

duration: ~30min
completed: 2026-09-04
---

# Phase 12 Plan 02: Dynamic Lifecycle and Headless Command Summary

**`ferrisfilter apply` now seeds `dynamic.json` from the committed template, compiles it into a complete playable `dynamic.filter`, and writes it atomically into the game folder — with 8 integration tests pinning the behavior.**

## Performance

- **Duration:** ~30 min
- **Tasks:** 3 completed
- **Files:** 2 created, 6 modified
- **Tests:** 189 → 204 (15 added, 0 failures)

## Tasks Completed

| Task | Name | Commit | Key Changes |
|------|------|--------|-------------|
| 1 | Dynamic store lifecycle | `2077452` | `src/dynamic.rs`: seed/reset/compile/atomic-write + 7 unit tests |
| 2 | CLI subcommand + gitignore + docs | `13d67fe` | `apply` subcommand, `.gitignore`, CLAUDE.md layout rows |
| 3 | Integration tests | `9b675ea` | `tests/dynamic_filter.rs`, 8 cases |

## What Was Built

### `src/dynamic.rs` — the lifecycle

Compiling needed no new emitter path: `dynamic.json` is a full `Config`, so `compile_store` is
Phase 11's `load_inputs` pointed at a different file plus `compile_as` for the header identity.
**The economy sections come along automatically** because the same pricer runs — which is what
makes `dynamic.filter` a complete, playable filter rather than a policy-only fragment.

Three details that carry weight:

- **`ensure_store` never overwrites an existing store.** That file holds the user's edits, and
  clobbering it is the only destructive failure mode in the module. `reset_store` is a separate,
  deliberately-called function. Both directions are tested.
- **`write_filter` creates its temp file in the *destination* directory.** The game folder is
  typically under OneDrive; a `NamedTempFile` in the system temp dir would make the atomic rename
  a cross-volume move, which fails. It errors rather than creating a missing destination, matching
  `install`'s guard, so a typo'd path fails loudly instead of scattering filters.
- **`default_destination` delegates to `install::default_dest`** (now `pub(crate)`) rather than
  re-spelling the path segments — one place in the crate knows where PoE2 keeps its filters.

No printing in the module; it returns `StoreState` and `PriceSource` and lets callers present.

### `ferrisfilter apply` (DYN-08)

Seeds if needed, compiles, writes into the game folder. `--store`, `--base`, `--dest`, `--reset`.
The price-source warnings reuse `cmd_generate`'s exact wording for the same conditions rather than
inventing a second spelling. The success output ends with the reload instruction — not a nicety,
since PoE2 never re-reads a filter on its own and a tool that writes silently reads as broken.

### `tests/dynamic_filter.rs`

Eight cases, all hermetic. Two were written deliberately to assert what the roadmap criteria
actually claim:

- **Economy parity** compares the economy half of `dynamic.filter` against the economy half of a
  `universal.filter` generated from the same `prices.json` in the same run — not a hand-written
  substring list that would pass while being subtly wrong.
- **AreaLevel** asserts zero occurrences in dynamic output **and non-zero in universal output**.
  The dynamic half alone would pass even if `AreaLevel` emission were broken everywhere.

## Verification Results

- **Full suite:** 204 passed, 0 failed (was 189).
- **`git diff --exit-code tests/fixtures/ tests/snapshots/ tests/cli_generate.rs tests/cli_errors.rs` → clean.**
  This plan only *added* a test file.
- **`universal.filter` byte-identical at 27,245 bytes** across the entire phase — generated before
  plan 12-01 began and again after 12-02 finished, then diffed. The milestone acceptance bar holds.
- **End-to-end by hand:** first `apply` reports `created dynamic.json from dynamic_base.json`,
  second reports `using existing dynamic.json`, both write 25,453 bytes and print the reload line.
- **`grep -cE "^\s*(println!|eprintln!)" src/dynamic.rs` → 0.**

## Acceptance Criteria Check

- [x] DYN-03: edits survive across runs; the second run reads the store rather than re-seeding
- [x] DYN-06: `dynamic.filter`'s economy half is byte-equal to `universal.filter`'s
- [x] DYN-07: atomic write, temp file in the destination directory, missing destination refused
- [x] DYN-08: the whole flow runs headlessly from the CLI
- [x] `dynamic_base.json` committed; `dynamic.json` / `dynamic.filter` git-ignored (verified with
      `git check-ignore`)
- [x] CLAUDE.md documents all three files plus the subcommand

## Deviations from Plan

One, and it is an addition rather than a departure:

**`default_destination` was added to `src/dynamic.rs` as a delegating wrapper.** The plan required
`grep -n "default_dest" src/dynamic.rs` to match, but the natural implementation put the
resolution entirely in `run.rs`, leaving `dynamic.rs` without a reference. Rather than satisfy the
criterion cosmetically, the wrapper was added so the dynamic module genuinely owns its default
and the path segments still live in exactly one place.

## Issues Encountered

None. All 8 integration tests passed on their first run — helped by the plan having pinned the
exact assertion shapes in advance, so there was no drift between what the criteria claimed and what
the tests checked.

## Next Steps

Phase 12 is complete. Next is phase verification, then Phase 13 (Overlay Shell) — the last phase
whose success can only be judged with the game actually running.
