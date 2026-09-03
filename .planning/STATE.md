---
gsd_state_version: 1.0
milestone: none
milestone_name: "v0.3 shipped — v0.4 not yet defined"
status: milestone_complete
stopped_at: "Completed v0.3 milestone (archived 2026-06-27)"
last_updated: "2026-06-27T00:00:00.000Z"
last_activity: 2026-06-27
progress:
  total_phases: 0
  completed_phases: 0
  total_plans: 0
  completed_plans: 0
  percent: 0
---

# Project State

## Project Reference

See: .planning/PROJECT.md (updated 2026-06-27)

**Core value:** `generate` turns the config into a correct, deterministic PoE2 loot filter.
**Current focus:** Planning the next milestone — run `/gsd-new-milestone`.

## Current Position

Milestone: none active (v0.3 shipped 2026-06-27)
Phase: —
Plan: —
Status: Between milestones

Shipped so far: v0.1 (2026-05-28) · v0.2 (2026-06-12) · v0.3 (2026-06-27)

## Performance Metrics

**Velocity:**

- Total plans completed: 26 (v0.1: 14 · v0.2: 7 · v0.3: 5)

**By Milestone:**

| Milestone | Phases | Plans | Commits | Span |
|-----------|--------|-------|---------|------|
| v0.1 Initial Release | 4 | 14 | 127 | 7 days |
| v0.2 Declarative Rule Model | 3 | 7 | ~38 | 7 days |
| v0.3 Economy Items & Exceptional Gear | 3 | 5 | 54 | 15 days |

Per-plan timings for archived milestones live in the phase SUMMARY.md files under
`.planning/phases/` (see also `.planning/RETROSPECTIVE.md` for cross-milestone trends).

## Accumulated Context

### Decisions

Full decision log lives in PROJECT.md → Key Decisions. Decisions that still constrain future
work (carried forward past the v0.3 close):

- **PoE2 boolean conditions take no operator** — `Corrupted False`, never `Corrupted == False`.
- **OR across conditions must be separate rules** — filter blocks are AND-only.
- **Per-category style overrides ride on the tier style** via clone-null-append (clone the tier
  `Style`, null the overridden attribute, `emit_style_actions`, then append the override line).
  Used by priced uniques (bronze bg) and priced gems (per-category bg).
- **New named styles are added as `Option<Style>`** (like `craftable`, `exceptional`) so
  `--config` files predating the style keep parsing.
- **New optional `when` fields use `serde(default)` + `skip_serializing_if`** for zero-cost
  backward compatibility and clean round-trips.
- **Each uncut gem level is a distinct PoE2 base type** — never strip the level from `BaseType`.
- **Fixtures are regenerated hermetically** (temp dir + binary by absolute path, mirroring
  `generate_on_seed()`); never `cargo run` from the repo root, which picks up a live
  `prices.json`.
- **The style vocabulary is still hardcoded** in `Styles` + `emitter::resolve_style`; `FUTURE`
  comments mark both sites. Backlog Phase 999.1.

### Pending Todos

- **`parity_ingame` test is failing** — pre-existing, predates Phase 10. Its oracle
  `tests/fixtures/parity-ingame.filter` was never refreshed after the v0.2 re-baseline. Decide
  refresh-or-retire in v0.4.
- **Amend `10-01-SUMMARY.md` prose** — it says `exceptional` is a required `Styles` field; it
  landed as `Option<Style>`. Code is correct and tested; only the prose is stale.
- **Test-only compiler warnings** (pre-existing): unreachable match arm in the emitter
  unknown-style test (`GenerateError` has a single variant); unused `FakeClient::ok` helper in
  `sync/metadata.rs`.
- **Untracked working-tree files** — decide whether the `ssf_*_config.json` / `ssf_*.filter`
  per-build filters are tracked deliverables or `.gitignore` fodder, and drop the `scratch_*`,
  `foo.filter`, `temptest_gen.filter` scratch artifacts.
- **`Cargo.toml` is still `version = "0.1.0"`** while tags read v0.1/v0.2/v0.3 — confirm whether
  the crate version is meant to track milestone tags.

### Blockers/Concerns

None open.

## Deferred Items

Items acknowledged and deferred at the **v0.3** close on 2026-06-27 (open-artifact audit was
clear — these are scope/debt carries, not open GSD artifacts):

| Category | Item | Status |
|----------|------|--------|
| test | `parity_ingame` failing; oracle stale since the v0.2 re-baseline | Pre-existing; refresh or retire in v0.4 |
| docs | `10-01-SUMMARY.md` prose vs `Option<Style>` reality | Cosmetic; code self-consistent and tested |
| scope | ECONX-01 Tablets as a class-less priced `BaseType ==` category | Deferred; needs pricer/emitter support + a poe.ninja spike |
| scope | Config-driven style vocabulary | Backlog Phase 999.1 |
| scope | Generate QoL (GENX-01..04): `--dry-run`, `--diff`, verbose rule logging, stale-prices warning | Deferred to a later milestone |
| scope | Sync extensions (SYNX-01..02): multiple build profiles, alternate price source | Deferred to a later milestone |

Items carried from the **v0.1** close on 2026-05-28:

| Category | Item | Status |
|----------|------|--------|
| uat | Live softcore no-flag `sync` happy path | Blocked on the PoE2 league cycle; tool fails safe. Live integration confirmed via `sync --standard`. Re-confirm on a new league launch. |
| verification | Live network sign-off for `sync` | Signed off 2026-05-28; full live re-check pending an active softcore league. |
| sync | Lenient-parse guard for Standard unique endpoints (`UniqueSanctumRelics` missing `core`) | Non-blocking; wrote 0 uniques on Standard. |
| sync | Optional no-flag `sync` → `--standard` fallback/hint when no challenge league is active | Enhancement idea. |

## Session Continuity

Last session: 2026-06-27 — v0.3 milestone closed and archived.
Stopped at: Milestone complete.
Resume file: None

**Next step:** Run `/gsd-new-milestone` to scope v0.4.
