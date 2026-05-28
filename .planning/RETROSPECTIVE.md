# Retrospective: FerrisFilter

A living record of what worked, what didn't, and the patterns established across milestones.

## Milestone: v0.1 — Initial Release

**Shipped:** 2026-05-28
**Phases:** 4 | **Plans:** 14 | **Commits:** 127 | **LOC:** ~4,970 Rust | **Span:** 2026-05-21 → 2026-05-28

### What Was Built

A Rust CLI that compiles a human-editable policy model into a deterministic PoE2 loot filter
(`generate`), prices currency from poe.ninja (`sync`), and installs the result into the game
folder (`install`). Plus an early-league price fallback (`--early-game`).

### What Worked

- **Snapshot + `artifact_guard` as a hard contract.** Locking `universal.filter` byte-for-byte early
  made every later refactor (pricing layer, fallback) safe — regressions surfaced instantly.
- **Pure-logic / IO split.** `sync` fetches; `generate` does all conversion + tiering. Kept the
  generator deterministic and unit-testable; the network layer hid behind an injectable `HttpClient`,
  so the whole sync pipeline is tested with `FakeClient` + recorded fixtures (zero live calls).
- **Spike before HTTP code.** Resolving the poe.ninja PoE2 API surface first avoided rework.

### What Was Inefficient

- **Stale ROADMAP checkboxes.** Phase 4 stayed `[ ]` through completion; caught only at milestone close.
- **An explicit `rates` field detour** in the fallback work was added then reverted in favor of deriving
  rates from the anchor orbs — a cleaner design reached after a round-trip.

### Patterns Established

- Cross-rates are **derived from the three anchor orbs**, read in whatever single unit they're quoted in
  (relative to the Chaos Orb) — unit-agnostic, backward-compatible with live chaos-quoted data.
- Committed, paired fallback data (`prices-fallback.json` + `tiers-fallback.json`) used together.
- `CHANGELOG.md` updated at milestone completion (convention recorded in `CLAUDE.md`).

### Key Lessons

- Keep `ROADMAP.md` checkboxes/progress in sync as phases complete, not at close.
- External/environmental blockers (a league rollover) are not code defects — fail safe and sign off.

### Cost Observations

- Sessions: multiple (context resets mid-Phase-4). Model: Opus-class for design discussions.
- Notable: byte-for-byte snapshot test paid for itself repeatedly during refactors.

## Cross-Milestone Trends

*(Populated as more milestones complete.)*

| Milestone | Phases | Plans | LOC | Span |
|-----------|--------|-------|-----|------|
| v0.1 Initial Release | 4 | 14 | ~4,970 | 7 days |
