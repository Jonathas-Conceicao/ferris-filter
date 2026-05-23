# Phase 1: Foundation - Context

**Gathered:** 2026-05-23
**Status:** Ready for planning

<domain>
## Phase Boundary

Phase 1 delivers the Rust crate skeleton for `FerrisFilter`:

- A `cargo build`-able crate with a clap-based CLI shell exposing `generate` and `sync`
  subcommands (both *visible* with `--help`; their real logic is NOT implemented here).
- `generate` exposes its `--output <path>` flag (default `universal.filter`).
- Non-zero exit with a clear message on unknown commands and on load failures.
- serde data models for all three JSON inputs — `config.json`, `tiers.json`, `prices.json` —
  that round-trip through serde and validate with actionable (field-path + expected-type) errors.

**Explicitly out of scope (later phases):** the actual filter-generation logic (Phase 2),
cross-rate / tiering math (Phase 3), and real poe.ninja networking for `sync` (Phase 4).
Phase 1 only makes the subcommands *exist* and models the *shapes*.

</domain>

<decisions>
## Implementation Decisions

### CLI shell & error reporting
- **D-01:** Use **clap with derive macros** (`#[derive(Parser)]` structs/enums), not the builder API.
- **D-02:** Binary is invoked as **`ferrisfilter`**, with a short **`ff` alias** for dev ergonomics.
- **D-03:** Validation/load errors use **`miette` + `thiserror`**: typed errors (thiserror) rendered
  by miette with the offending JSON span highlighted and a help line. This is how success
  criterion 3 ("bad field path + expected type") is satisfied — cleanly, with good UX.
- **D-04:** Unknown subcommands and load failures exit **non-zero** with a clear message
  (success criteria 3 & 5). clap handles the unknown-command case natively.

### JSON modeling & strictness
- **D-05:** **Strict deserialization** — apply `#[serde(deny_unknown_fields)]` to config.json and
  tiers.json models so a typo'd key (e.g. `minimip`) errors loudly instead of being silently dropped.
  These are hand-edited policy files; silent drops are a bad failure mode.
- **D-06:** **Semantic round-trip** is the bar (deserialize → serialize → deserialize is stable/
  equivalent), verified by a per-model serde test. Byte-identical JSON re-serialization is NOT
  required — byte-fidelity is `universal.filter`'s concern in Phase 2, not the input files'.
- **D-07:** Parse-depth is **Claude's discretion, leaning hybrid**: type the fixed/closed
  vocabularies as Rust enums and validate them on load (minimap colors & shapes, beam colors,
  style `action` Show/Hide, `armourType` keys); keep the `areaLevelBands` rarity-rule DSL strings
  ("any rarity", "Magic + Rare", "Rare only", "Quality only") as **strings** for now — that
  grammar is Phase 2's job to interpret.

### File schemas (both files are invented in this phase)
- **D-08:** `tiers.json` = an **ordered list of lower-bound entries** (e.g. `[{tier, min, unit}, …]`),
  ascending. Validation = all 7 tiers **S–F present** and bounds **strictly increasing**; order is
  read from the data, not imposed by code. This makes "missing cutoff / out of order" (success
  criterion 4) directly checkable.
- **D-09:** Tier cutoffs are **unit-tagged** — `unit: "chaos"` or `unit: "div"` per entry (e.g.
  `{tier:"B", min:5, unit:"chaos"}`, `{tier:"A", min:1, unit:"div"}`). Divine-denominated cutoffs
  get normalized to chaos at *generate*-time using the synced divine rate, so a `≥5div` tier tracks
  the live economy rather than freezing to a static chaos number. (Normalization itself is Phase 3;
  Phase 1 only models + validates the shape.)
- **D-10:** `prices.json` = a **wrapper object** `{ league, fetchedAt, items: [...] }` where each item
  is `{ name, baseType?, exalt, chaos, div }` (any of exalt/chaos/div may be null). The first three
  `items` are always **Exalt, Chaos, Divine** (ordering carried by the list). `baseType` is carried so
  Phase 3 can resolve uniques that share a base type. `league` + `fetchedAt` satisfy SYNC-05's
  metadata requirement at the model level.

### Crate layout & migration
- **D-11:** **Single binary crate** (no separate lib target) — modules under `src/`, `src/main.rs`
  parses args and dispatches. *(Note for Phase 2: snapshot/integration tests will likely drive the
  built binary via `assert_cmd` or use in-module `#[cfg(test)]` units, since internals aren't exposed
  as a library — test-harness mechanics are Claude's discretion.)*
- **D-12:** **Rename `filter-model.json` → `config.json` now** (`git mv`) as part of Foundation;
  models and loaders target `config.json`. Establishes the canonical name so Phase 2 builds against it.
- **D-13:** **Extract the curated currency base-lists out of `universal.filter` into a new tracked
  data file now**, populated with the data currently embedded in `universal.filter`, and model it in
  Phase 1. (File name/exact shape is Claude's discretion — e.g. `currency.json`.) This makes the
  currency lists a typed generator input from the start, even though Phase 3 will eventually replace
  the curation with price-driven tiering.

### Claude's Discretion
- Hybrid parse-depth specifics (D-07) — which exact fields become enums vs strings.
- Currency base-list file name and exact schema (D-13).
- Test-harness mechanics under a single-binary crate (D-11).
- Edition/MSRV, dependency versions, module naming, exit-code values (beyond "non-zero").

### Milestone-wide latitude (applies beyond Phase 1)
- **D-14:** **The `config.json` schema, field labels, and structure are NOT frozen** for this
  milestone. Redesign them freely to fit the Rust models / cleaner design — nothing about the input
  schema is set in stone until the milestone closes. The **only** frozen artifact is the
  `universal.filter` *output* (Phase 2's byte-identical baseline). Input schema = flexible;
  output = fixed.

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Domain spec & source-of-truth data
- `CLAUDE.md` — the filter grammar, Dracula style-tier system, design philosophy, PoE2 grammar
  caveats, and the prose description of the JSON model. The authoritative domain reference.
- `filter-model.json` — current policy model; **to be renamed `config.json` in this phase**. The
  starting shape for the config serde models (subject to redesign per D-14).
- `universal.filter` — the hand-built generated output. Frozen byte-identical baseline for Phase 2;
  in Phase 1 it is the **source of the curated currency base-lists** to extract per D-13.

### Requirements & success criteria
- `.planning/REQUIREMENTS.md` §CLI (CLI-01, CLI-03, CLI-05) and §Config (CFG-01, CFG-02, CFG-03) —
  the requirements this phase satisfies.
- `.planning/ROADMAP.md` → "Phase 1: Foundation" — the 5 success criteria the phase is verified against.
- `.planning/PROJECT.md` — locked project decisions (Rust, crate name, two subcommands, three
  separate files, chaos reference unit, milestone git workflow).

### External (fetch only if the above is insufficient)
- https://poe2db.tw/us/ — verify exact PoE2 class / base-type names referenced in the models.
- https://www.pathofexile.com/item-filter/about — filter grammar (PoE1 docs; PoE2 uses the same
  grammar, different valid values — see CLAUDE.md "PoE2 differences").

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- **No Rust code exists yet** — this is a greenfield crate (no `Cargo.toml`/`src/`). Phase 1
  creates the skeleton from scratch.
- `filter-model.json` (4.5 KB) is the concrete shape the config models must capture: `meta`,
  `styles` (S/A/B/C/D/E/F/Recycle/unchanged/hidden with text/border/background RGB triples,
  nullable font, nullable minimap{size,color,shape}, nullable beam{color,temp}, nullable
  sound{custom|id, vol}, action, disableDropSound), `build` (weapons/armour/armourTypes),
  `weaponClasses`, `armourClasses`, `flaskCharmClasses`, `areaLevelBands` (range + show rules),
  `alwaysShow` (currency/recycle), `fallthrough`.
- `universal.filter` (31 KB) holds the curated currency base-lists to extract into the new
  currency data file (D-13).

### Established Patterns
- Colors in `filter-model.json` are **3-element RGB arrays** (e.g. `[40,42,54]`); alpha is added at
  filter-generation time (Phase 2), so model them as RGB triples, not RGBA.
- Many style fields are **nullable** (`font`, `minimap`, `beam`, `sound`, and text/border/background
  for `unchanged`/`hidden`) → model as `Option<T>`.

### Integration Points
- Phase 2 (Static Generate) consumes these models to emit `universal.filter`. The model design
  here directly shapes Phase 2's generator and its insta snapshot test.
- Phase 3 (Pricing) consumes `tiers.json` + `prices.json` models for cross-rate math and tier
  assignment — D-08/D-09/D-10 are designed with that consumption in mind.

</code_context>

<specifics>
## Specific Ideas

- The author wants a short `ff` alias alongside `ferrisfilter` for less typing during development.
- The author explicitly granted latitude to redesign `config.json`'s schema/labels this milestone
  (D-14) — downstream agents should optimize the schema for clean Rust models rather than slavishly
  mirroring `filter-model.json`.
- The author chose a single binary crate over a lib+bin split despite the testing-ergonomics note.

</specifics>

<deferred>
## Deferred Ideas

- **Filter-generation logic** — Phase 2.
- **Cross-rate normalization of div-denominated tier cutoffs** (the actual chaos conversion) — Phase 3.
- **Replacing curated currency base-lists with price-driven tiering** — Phase 3; the extracted
  currency data file (D-13) is transitional until then.
- **Real poe.ninja fetching for `sync`** — Phase 4 (the subcommand is only a visible stub here).
- **`--standard` / `--hc` league flags on `sync`** — belong to Phase 4 (CLI-04); Phase 1 only needs
  `sync` to exist with `--help`.
- None of the above is lost — each is scoped to its owning phase in ROADMAP.md.

</deferred>

---

*Phase: 01-foundation*
*Context gathered: 2026-05-23*
