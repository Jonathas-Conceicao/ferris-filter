# Roadmap: FerrisFilter

## Milestones

- ✅ **v0.1 Initial Release** — Phases 1-4 (shipped 2026-05-28) — [archive](milestones/v0.1-ROADMAP.md)
- ✅ **v0.2 Declarative Rule Model** — Phases 5-7 (shipped 2026-06-12) — [archive](milestones/v0.2-ROADMAP.md)
- ✅ **v0.3 Economy Items & Exceptional Gear** — Phases 8-10 (shipped 2026-06-27) — [archive](milestones/v0.3-ROADMAP.md)
- ✅ **v0.4 Overlay Filter Editor Rework** — Phases 11-15 (shipped 2026-09-17) — [archive](milestones/v0.4-ROADMAP.md)

## Phases

<details>
<summary>✅ v0.1 Initial Release (Phases 1-4) — SHIPPED 2026-05-28</summary>

- [x] Phase 1: Foundation (3/3 plans) — completed 2026-05-24
- [x] Phase 2: Static Generate (3/3 plans) — completed 2026-05-24
- [x] Phase 3: Pricing Layer (4/4 plans) — completed 2026-05-25
- [x] Phase 4: Sync (4/4 plans) — completed 2026-05-28

Full detail: [milestones/v0.1-ROADMAP.md](milestones/v0.1-ROADMAP.md)

</details>

<details>
<summary>✅ v0.2 Declarative Rule Model (Phases 5-7) — SHIPPED 2026-06-12</summary>

- [x] Phase 5: Rule Schema (2/2 plans) — completed 2026-06-05
- [x] Phase 6: Emitter Rewrite + Migration Parity (2/2 plans) — completed 2026-06-05
- [x] Phase 7: New Capabilities + Docs (3/3 plans) — completed 2026-06-12

Full detail: [milestones/v0.2-ROADMAP.md](milestones/v0.2-ROADMAP.md)

</details>

<details>
<summary>✅ v0.3 Economy Items & Exceptional Gear (Phases 8-10) — SHIPPED 2026-06-27</summary>

- [x] Phase 8: Sync Pipeline (2/2 plans) — completed 2026-06-24
- [x] Phase 9: Gem Background Styling (1/1 plan) — completed 2026-06-24
- [x] Phase 10: Corrupted Condition + Exceptional Gear (2/2 plans) — completed 2026-06-24

Full detail: [milestones/v0.3-ROADMAP.md](milestones/v0.3-ROADMAP.md)

</details>

<details>
<summary>✅ v0.4 Overlay Filter Editor Rework (Phases 11-15) — SHIPPED 2026-09-17</summary>

- [x] Phase 11: Callable Compile API (2/2 plans) — completed 2026-09-03
- [x] Phase 12: Dynamic Rule Store (2/2 plans) — completed 2026-09-04
- [x] Phase 13: Overlay Shell (5/5 plans) — completed 2026-09-06
- [x] Phase 14: Rule Editor + Apply (4/4 plans) — completed 2026-09-08
- [x] Phase 15: BaseType Lists and Rule Names (3/3 plans) — completed 2026-09-16

Full detail: [milestones/v0.4-ROADMAP.md](milestones/v0.4-ROADMAP.md)

</details>


## Backlog

### Phase 999.1: Config-driven styles (no hardcoded vocabulary) (BACKLOG)

**Goal:** Any config should be able to define and use new named styles without a code change.
Today the style vocabulary is hardcoded in the `Styles` struct (`src/models/config.rs`) and
`emitter::resolve_style` (`src/emitter.rs`) — adding `craftable` (for `ssf_icewolf_config.json`)
required editing both. Replace the named-field `Styles` struct with a name→`Style` map (or keep
S–F/unchanged/hidden/unidentified as the known palette plus an open `extra` map), resolve styles
by lookup instead of a `match`, and validate at load-time that every `rule.style` resolves.
`FUTURE` comments already mark both code locations.
**Requirements:** TBD
**Plans:** 3/4 plans executed

Plans:

- [ ] TBD (promote with /gsd-review-backlog when ready)

### Phase 999.2: Overlay single-instance guard (BACKLOG)

**Goal:** Launching `ff` while it is already running should surface the existing stripe instead of
opening a second one. Discovered by the author during Phase 13 wave-2 verification (2026-09-04):
a second launch opens a second stripe window, and only the first instance owns the `Shift+Z`
registration — the second correctly reports the conflict in its tray tooltip (D-10), so the
failure is visible rather than silent, but two stripes and one working hotkey is not the intended
shape for a tray app that gets double-clicked. Fix shape: acquire a named mutex at startup; if it
is already held, signal the running instance to show itself and exit rather than creating a
window. Small, self-contained, and touches only the overlay entry point.
**Requirements:** TBD
**Plans:** TBD

Plans:

- [ ] TBD (promote with /gsd-review-backlog when ready)
