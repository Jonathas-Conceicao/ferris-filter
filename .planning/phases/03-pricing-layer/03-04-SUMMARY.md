---
phase: 03-pricing-layer
plan: 04
subsystem: testing/snapshot
tags: [snapshot, re-baseline, guardrail, test-isolation, filter-artifact]
dependency_graph:
  requires: [03-03]
  provides: [re-baselined-snapshot, currency-retiering-guardrail, diff-artifact]
  affects: [tests/cli_generate.rs, tests/snapshots/cli_generate__generate_snapshot.snap, universal.filter]
tech_stack:
  added: []
  patterns: [insta-rebaseline, INSTA_UPDATE=always, guardrail-test, temp-output-test-isolation]
key_files:
  created:
    - .planning/phases/03-pricing-layer/03-04-REBASELINE-DIFF.md
  modified:
    - tests/snapshots/cli_generate__generate_snapshot.snap
    - universal.filter
    - tests/cli_generate.rs
decisions:
  - D-10 re-baseline executed: insta snapshot accepted once for price-driven output; INSTA_UPDATE=always; both .snap and universal.filter committed together
  - generate_in_repo_root_succeeds now uses --output tmpfile to prevent clobbering committed universal.filter during test runs
  - currency_retiering_present test pins the 5 Currency block edits so any future drift is caught before reaching committed artifacts
metrics:
  duration: 30
  completed: "2026-05-24"
  tasks_completed: 2
  tasks_total: 3
  files_changed: 4
---

# Phase 03 Plan 04: Snapshot Re-Baseline Summary

**One-liner:** Re-baselined insta snapshot + universal.filter for price-driven anchor-orb re-tiering; guardrail test proves only the 5 intended Currency block edits landed.

---

## What Was Built

The Phase 3 price-driven `generate` pipeline (built in plans 03-01 through 03-03) produces output that differs from the Phase 2 snapshot in exactly 5 places — the three anchor orbs (Divine, Chaos, Exalted) landing in new tiers per their realistic prices. This plan accepted that deliberate change under controlled review:

1. **Snapshot re-baselined** (`INSTA_UPDATE=always cargo test --test cli_generate generate_snapshot`): the insta oracle now reflects the Phase 3 output.
2. **universal.filter regenerated** (`cargo run --bin ff -- generate`): the committed artifact is byte-identical to generator output. 83 Show + 2 Hide blocks, no `Rarity == Unique` lines.
3. **Test isolation fixed** (`generate_in_repo_root_succeeds`): previously ran without `--output`, clobbering the committed `universal.filter` during tests. Now uses `--output <tmpfile>`. The `artifact_guard` test always compared the committed file against fresh generator output — not affected by this clobber — but the isolation is now correct for defense in depth.
4. **Diff artifact produced** (`.planning/phases/03-pricing-layer/03-04-REBASELINE-DIFF.md`): contains the full `git diff 89547c8 -- universal.filter` output plus the guardrail verification record.
5. **`currency_retiering_present` test added**: pins the 5 anchor-orb moves (Currency A has Divine, S does not; D has Exalted; E has Chaos; C has neither). Any future generator or prices.json drift that changes these assignments will fail this test before reaching committed artifacts.

---

## Commits

| Task | Commit | Description |
|------|--------|-------------|
| 1 | `61f6a1b` | test(03-04): re-baseline snapshot + fix test isolation + produce diff artifact |
| 2 | `e1342a7` | test(03-04): add currency_retiering_present guardrail test + verification record |

---

## Verification Results

**Full test suite after both tasks:**
- Unit tests: 53/53 pass
- Integration (cli_generate): 8/8 pass — includes new `currency_retiering_present`
- 0 failures

**Guardrail diff check:**
- `git diff 89547c8 -- universal.filter`: exactly 10 changed lines (5 - / 5 +)
- All in the `# --- Currency ---` section, all `BaseType ==` lines
- No other section or category touched
- Block counts: 83 Show + 2 Hide (unchanged from Phase 2)
- No `Rarity == Unique` blocks

---

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Test isolation fix applied to generate_in_repo_root_succeeds**
- **Found during:** Task 1 (per critical reminder in plan prompt)
- **Issue:** `generate_in_repo_root_succeeds` ran `ferrisfilter generate` without `--output`, which overwrites the committed `universal.filter` during test runs. While `artifact_guard` already used temp output (so the clobber didn't break it in the current run), the test's behavior was unreliable: running it before `artifact_guard` would put a freshly-generated file in the working tree, making the artifact-guard comparison trivially self-referential in some orderings.
- **Fix:** Added `--output <tmpfile>` to `generate_in_repo_root_succeeds` matching the pattern used by `generate_snapshot`, `artifact_guard`, and `generate_is_deterministic`.
- **Files modified:** `tests/cli_generate.rs`
- **Commit:** `61f6a1b`

---

## Task 3 Status: AWAITING HUMAN VERIFICATION

Task 3 is a `checkpoint:human-verify` gate. Automation (Tasks 1–2) is fully complete and committed. The plan pauses here for:

1. Human review of `.planning/phases/03-pricing-layer/03-04-REBASELINE-DIFF.md` — confirm ONLY the 5 Currency block edits appear in the diff.
2. Optional in-game spot-check: install `universal.filter` and confirm the re-tiered orbs render at their new styles (Divine at A/purple, Chaos at E/white, Exalted at D/cyan).

See CHECKPOINT REACHED message for details.

---

## Known Stubs

None. The generator produces a complete, functional filter.

---

## Threat Flags

None. No new network endpoints, auth paths, or trust-boundary changes introduced. The diff artifact and guardrail test directly mitigate T-03-09 (tampering) and T-03-10 (repudiation) from the plan's threat register.

---

## Self-Check: PASSED

- [x] `.planning/phases/03-pricing-layer/03-04-REBASELINE-DIFF.md` exists
- [x] `tests/snapshots/cli_generate__generate_snapshot.snap` updated and committed (`61f6a1b`)
- [x] `universal.filter` regenerated and committed (`61f6a1b`)
- [x] `currency_retiering_present` test added and passing (`e1342a7`)
- [x] All 8 cli_generate tests pass; 0 failures in full suite
- [x] Diff limited to 5 Currency block edits only (83 Show + 2 Hide preserved)
