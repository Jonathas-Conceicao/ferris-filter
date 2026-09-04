---
phase: 12-dynamic-rule-store
phase_name: Dynamic Rule Store
verified: 2026-09-04
status: passed
scores:
  success_criteria: 8/8
  requirements: 8/8
  plans_executed: 2/2
gaps: []
---

# Phase 12 Verification — Dynamic Rule Store

**Status: PASSED** — 8/8 success criteria, 8/8 requirements, 2/2 plans.

## Goal Achievement

**Phase goal:** a persisted `dynamic.json` compiles to a complete, playable `dynamic.filter` —
policy blocks from the dynamic rules, economy sections from `prices.json` — entirely without a GUI.

**Achieved.** `ferrisfilter apply` seeds the store from the committed template, compiles it, and
writes the result atomically into the game folder, printing where it went and that the filter must
be reloaded in-game. The feature is fully usable and scriptable today, before any overlay exists —
which was the reason for sequencing this phase ahead of Phase 13.

## Success Criteria

| # | Criterion | Result | Evidence |
|---|-----------|--------|----------|
| 1 | `dynamic.json` parses, round-trips, and reports a malformed field with its path — reusing the existing `Rule`/`When` types | ✅ | `malformed_store_reports_the_field_path` asserts stderr names `dynamic.json` and `rules[0].style`; 4 `rule_enabled_*` round-trip tests. No new rule types were introduced |
| 2 | A rule setting only `sockets` emits `Sockets >= N` and no `Class`/`Rarity`/`BaseArmour`/`Quality`/`Corrupted` | ✅ | `a_sockets_only_rule_emits_sockets_and_nothing_else` checks all seven absent conditions |
| 3 | Reordering two rules reorders their blocks | ✅ | `reordering_rules_reorders_their_blocks` swaps the array and asserts the block order flips |
| 4 | `dynamic.filter` contains the economy sections, matching what `universal.filter` emits from the same `prices.json` | ✅ | `dynamic_filter_carries_the_same_economy_sections_as_universal` compares the two economy halves **for equality**, not against a substring list |
| 5 | No dynamic rule emits `AreaLevel`, while `config.json`'s rules still do | ✅ | `dynamic_emits_no_area_level_while_universal_still_does` asserts **both** halves |
| 6 | Interrupting the write leaves the previous `dynamic.filter` intact | ✅ | Atomic write-then-rename with the temp file in the destination directory; `write_filter_leaves_the_previous_file_intact_on_failure`, `write_filter_refuses_a_missing_destination`, `write_filter_second_write_wins` |
| 7 | Edits to `dynamic.json` are present after the process exits and restarts | ✅ | `edits_to_the_store_survive_across_runs` — the second run reports `using existing dynamic.json` and its output reflects the edit |
| 8 | `artifact_guard` and `generate_snapshot` still pass with no fixture changes | ✅ | Both green; `git diff --exit-code tests/fixtures/ tests/snapshots/` clean; `universal.filter` byte-identical at 27,245 bytes across the whole phase |

## Requirements Coverage

| ID | Requirement | Plan | Status |
|----|-------------|------|--------|
| DYN-01 | `dynamic.json` modeled, round-trips, field-path errors | 12-01 | ✅ Complete |
| DYN-02 | Ordinal identity; array order is precedence | 12-01 | ✅ Complete |
| DYN-03 | Persists to disk, reloaded next run | 12-02 | ✅ Complete |
| DYN-04 | Every condition independently optional | 12-01 | ✅ Complete |
| DYN-05 | No `areaLevel` in dynamic rules | 12-01 | ✅ Complete |
| DYN-06 | Complete playable filter with economy sections | 12-02 | ✅ Complete |
| DYN-07 | Atomic write into the game folder | 12-02 | ✅ Complete |
| DYN-08 | Reachable headlessly | 12-02 | ✅ Complete |

## Test Results

```
204 passed / 0 failed
  lib              166   (was 136 at the start of v0.4 — 30 added across phases 11-12)
  dynamic_filter     8   (new this phase)
  cli_errors         4
  cli_filter_copy    4
  cli_generate      13
  sync_prices        8
  doc                1
```

Pre-existing warnings (carried from the v0.3 audit, not introduced here): unreachable match arm in
the emitter unknown-style test; unused `FakeClient::ok` in `sync/metadata.rs`.

## Integration Check

Phase 14's Apply action needs exactly what this phase built, as callable functions rather than a
subcommand — and it has them: `dynamic::ensure_store`, `compile_store` and `write_filter` are
public, print nothing, and take every path as a parameter. `run.rs`'s `cmd_apply` is one caller;
the overlay will be another.

The `enabled` flag Phase 14's EDIT-02 depends on is in place, with its emit semantics settled and
tested — a disabled rule stays in the store, emits nothing, and does not renumber its neighbours.

`config.json → universal.filter` remains untouched and oracle-locked, so the dynamic path is
purely additive as the milestone's acceptance bar requires.

## What Was Simpler Than Planned

The author supplying `dynamic_base.json` during the discussion removed the phase's largest assumed
task. Because it is a full `Config` rather than a bespoke envelope, DYN-06 needed **no new emitter
code at all** — `compile_store` is `load_inputs` pointed at a different file. The research's
`{version, rules: [...]}` sketch would have required splicing two configs together and a separate
economy path.

Worth noting for future phases: that was discovered by *asking and then verifying the answer
immediately* — running `generate --config dynamic_base.json` during the discussion, before writing
any plan. Ten seconds of execution replaced a plan's worth of assumption.

## Deviations Carried Forward

Three, all recorded in the plan summaries, none affecting behavior:

1. Five existing `Rule` struct literals needed `enabled: None` (no `Default` derive). Mechanical.
2. `generate`'s doc comment was orphaned onto `FilterIdentity` by an insertion and had to be moved.
3. `default_destination` was added to `src/dynamic.rs` as a delegating wrapper so the module
   genuinely owns its default, rather than satisfying the plan's grep criterion cosmetically.

## Lesson

**`cargo build` is not a sufficient compile check on this repo when an edit touches tests.** During
plan 12-01 a `perl -0pi` patch silently no-opped — the files are CRLF and the pattern used `\n` —
and `cargo build` then reported success because it does not compile test code. `cargo test --no-run`
caught it. Combined with Phase 11's lesson (a green suite is not evidence for "output unchanged"),
the pattern is the same: **verify the specific claim, not a proxy for it.**

---

*Verified 2026-09-04. Phase 12 complete.*
