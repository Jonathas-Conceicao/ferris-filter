# Phase 4: Sync - Context

**Gathered:** 2026-05-25
**Status:** Ready for planning

<domain>
## Phase Boundary

Phase 4 implements `ferrisfilter sync` — fetching live PoE2 economy data from poe.ninja and
writing a well-formed `prices.json` (the schema locked in Phase 3), plus a `metadata` mode that
fetches item metadata from poe2db to build the committed category/class mapping. The poe.ninja
**and** poe2db API surfaces are confirmed via a **live spike before any HTTP code is written**
(success criterion 1). This closes the loop: `sync` → `prices.json` → `generate`.

**In scope:** the spike (both surfaces); the `sync prices` + `sync metadata` subcommands; the async
HTTP fetch layer; league resolution + flags (CLI-02, CLI-04); the overview→category/class mapping
(built from poe2db, committed); gem-level≥18 filtering; atomic failure-safe writes (SYNC-04);
recording league + timestamp (SYNC-05).

**Out of scope (v2):** multiple build profiles (SYNX-01); alternate price source (SYNX-02); stale
price warnings (GENX-04). Actual WASM compilation is a *future* release — it only influences
dependency selection now (see D-04), it is not a Phase 4 deliverable.

</domain>

<decisions>
## Implementation Decisions

### sync command structure
- **D-01:** `sync` has two modes: **`sync prices`** (the default — bare `ferrisfilter sync` runs it)
  fetches poe.ninja economy and writes `prices.json`; **`sync metadata`** fetches the **RePoE-fork
  PoE2 JSON exports** (`base_items.json` + `uniques.json`, see D-04) and builds/refreshes the committed
  category/class/baseType **mapping artifact** in the repo. League flags apply to `prices`;
  `metadata` is league-agnostic.
  > **CORRECTION (plan-phase spike, 2026-05-25):** the original poe2db `/sp/pob` source was found
  > invalid (it is a Path-of-Building code-sharing page, not an item API; poe2db offers HTML only,
  > and poedb's "Developer API" is just a GGG-OAuth wrapper). Replaced with the RePoE-fork exports.
- **D-02:** Implement CLI-02 (`sync` runnable) and CLI-04 (combinable `--standard` + `--hc`; neither
  → current **softcore trade** league) on the `prices` mode. Each subcommand keeps `--help`.

### Refresh strategy & the mapping
- **D-03:** **Rebuild** `prices.json` from poe.ninja overviews each `sync prices` run, categorizing
  every item via the committed **overview → {filter category, `Class==` value}** mapping. Uniques get
  category/class from their overview + `baseType`. This DISCOVERS new valuable items + uniques
  automatically (activating Phase 3's dormant unique-block highlighting). The curated Phase-3 seed was
  the bootstrap; `sync prices` replaces it with live, mapped data.
- **D-04 (mapping source — REVISED after spike):** The mapping is a **committed repo artifact**, but
  NOT hand-typed — `sync metadata` builds it from the **RePoE-fork PoE2 JSON exports**:
  - `https://repoe-fork.github.io/poe2/base_items.json` — every base item → `item_class` (e.g. "Divine
    Orb" → `StackableCurrency`); the source of the `class` field. Keyed by metadata path.
  - `https://repoe-fork.github.io/poe2/uniques.json` — uniques → `name` + `item_class` (~400 entries).
  These are clean structured JSON (unlike poe2db, which is HTML-only). `sync metadata` must **normalize
  RePoE's `item_class`** (PascalCase, e.g. `StackableCurrency`) to the filter's `Class==` string
  (spaced, e.g. `Stackable Currency`). The **`category`** (Currency / Breach / Vault Keys …) comes from
  **poe.ninja's overview `type`** at `prices` time (not from RePoE). `sync prices` reads ONLY the
  committed map + poe.ninja (single runtime price API; deterministic; RePoE touched only by `metadata`).
  - Bootstrap note: the existing `currency.json` already holds class+category+baseType for the 358
    curated items, so the initial committed map can be seeded from it; `sync metadata` then refreshes/
    extends it from RePoE. Either path produces the same committed artifact shape.

### HTTP / dependencies
- **D-05:** **Async** HTTP (not blocking). Dependency choice is **weighted by WASM-compatibility** for
  a possible future WASM build — prefer a stack with a working `wasm32` target and avoid crates that
  hard-block it. `reqwest` is a candidate (its wasm backend uses browser fetch, sidestepping tokio);
  the researcher selects the concrete crate(s) and confirms wasm32 viability. WASM is a *forward
  guideline*, not a Phase 4 target (native is the deliverable).

### Spike (criterion 1 — gates all HTTP code)
- **D-06:** **Live-probe spike — DONE during plan-phase** (captured in `04-RESEARCH.md`). Confirmed:
  - **poe.ninja PoE2:** `GET https://poe.ninja/poe2/api/economy/exchange/current/overview?league={name}&type={type}`
    (21 type values); values via `primaryValue` (in primary currency = Divine) + `core.rates` for
    chaos/exalt conversion (NO direct `chaosValue`); league resolution via `/poe2/api/data/index-state`
    (do NOT use the GGG leagues API — it returns PoE1). Uniques + UncutGem currently return empty.
  - **RePoE-fork:** `base_items.json` + `uniques.json` confirmed valid JSON with name→item_class (D-04).
  - **poe2db / poedb Developer API: rejected** (HTML-only / GGG-OAuth wrapper, no item-metadata JSON).
  Remaining live-confirmation the executor must still do (LOW-confidence, off-season): the exact gem
  level field + unique `baseType` shape, once poe.ninja has data — handle leniently + fixture-test (D-09).
  No HTTP client code beyond what the spike confirmed; treat `04-RESEARCH.md` as the authoritative surface.

### Failure-safety & league resolution
- **D-07:** **Atomic, failure-safe writes (SYNC-04):** fetch fully, validate the response is non-empty
  and parses into the expected shape, write to a temp file, then atomically rename over `prices.json`
  (and over the mapping artifact for `metadata`). A network failure / empty / malformed response leaves
  the existing file untouched.
- **D-08:** **Live league resolution (CLI-04, SYNC-05):** resolve the current league NAME dynamically
  from poe.ninja's league-resolution endpoint per the flags (softcore trade / `--hc` / `--standard` /
  combined), so it survives league rollovers without code edits. Record the resolved league name + a
  fetch timestamp in `prices.json`.

### Gem filtering & output shape
- **D-09:** **Uncut gems included only at level ≥ 18 (SYNC-03):** filter poe.ninja gem data by gem
  level; below 18 → excluded. `prices.json` keeps the Phase-3 schema exactly: `{league, fetchedAt,
  items:[{name, baseType, class, category, exalt, chaos, div}]}`, with **Exalt/Chaos/Divine as the
  first three items** (SYNC-02) and currency/uniques/other tracked drops included (SYNC-03).

### Claude's Discretion
- Exact async crate(s) + TLS backend (research-selected, wasm-aware); poe.ninja/RePoE-fork
  rate-limiting + User-Agent etiquette; clap modeling of the default `prices` subcommand; the mapping
  artifact's file name/shape; the RePoE `item_class` → filter `Class==` normalization table; how
  overviews are enumerated; retry/timeout policy.

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### The output contract & inputs
- `prices.json` — the schema `sync prices` MUST produce (Phase 3 D-03): `{league, fetchedAt,
  items:[{name, baseType, class, category, exalt, chaos, div}]}`; Exalt/Chaos/Divine first.
- `src/models/prices.rs` — the `PricesFile`/`PriceItem` model the output must deserialize into.
- `src/cli.rs` — the clap `Commands` enum to extend (`Sync` → modes); `src/main.rs` `cmd_sync` stub.
- `src/loader.rs`, `src/error.rs` — miette/thiserror patterns to reuse for fetch/parse errors.
- `currency.json` — the curated 15-category structure (reference for what the mapping should cover).

### Requirements & criteria
- `.planning/REQUIREMENTS.md` §CLI/Sync — CLI-02, CLI-04, SYNC-01..05 (+ v2 SYNX-01/02 = out of scope).
- `.planning/ROADMAP.md` → "Phase 4: Sync" — the 5 success criteria (criterion 1 = the spike gate).
- `.planning/phases/03-pricing-layer/03-CONTEXT.md` + `03-VERIFICATION.md` — the prices.json schema,
  the dormant unique-block capability that sync activates, and the "Notes for Phase 4".

### External (spike-confirmed — see 04-RESEARCH.md, criterion 1)
- **poe.ninja PoE2 economy API** — `https://poe.ninja/poe2/api/economy/exchange/current/overview?league={name}&type={type}`;
  `primaryValue` + `core.rates` (no direct `chaosValue`); league via `/poe2/api/data/index-state`.
- **RePoE-fork PoE2 metadata** — `https://repoe-fork.github.io/poe2/base_items.json` (name→item_class)
  and `https://repoe-fork.github.io/poe2/uniques.json` (uniques → name+item_class). The `sync metadata` source.
- ~~poe2db `/sp/pob`~~ / poedb Developer API — **rejected** (HTML-only / GGG-OAuth wrapper; no item JSON).
- `CLAUDE.md` external refs and the "price source = poe.ninja" decision.

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- `cmd_sync` is a one-line stub; `Commands::Sync` is a flagless variant — extend to subcommands +
  league flags.
- `src/models/prices.rs` (`PricesFile`, `PriceItem`) is the exact serialization target — reuse for
  building + writing the synced file (serde_json to_string_pretty or similar).
- `loader::load_json` + `JsonLoadError` (miette/thiserror) — pattern to mirror for fetch/parse errors
  and the TIER-04-style "name the problem" diagnostics.
- `src/bin/seed_prices.rs` (Phase 3) is a precedent for a separate data-building binary — but the
  user chose a `sync metadata` SUBCOMMAND (not a separate bin) to build the map.

### Constraints
- No HTTP/async deps yet — Phase 4 introduces the first network dependency (D-05, async, wasm-aware).
- The crate is a single binary with `default-run = "ff"` + dual `[[bin]]` (ferrisfilter/ff); adding
  an async runtime must not break the existing sync `generate` path or the bins.
- Output must remain valid for `generate` to consume (same `prices.json` schema).

### Integration Points
- After `sync prices`, `generate` consumes the refreshed `prices.json` (Phase 3 pipeline) — and
  newly-synced valuable uniques will start emitting unique blocks (Phase 3's dormant capability).
- `sync metadata` writes the committed mapping the `prices` mode and (potentially) `generate` rely on.

</code_context>

<specifics>
## Specific Ideas

- User's design: split sync into `prices` (poe.ninja, default) and `metadata` (RePoE-fork → committed
  map) so the map isn't hand-maintained but is still a deterministic committed artifact.
- Metadata source (after spike): **RePoE-fork PoE2 exports** (`base_items.json` + `uniques.json`),
  found by the user — chosen over poe2db (HTML-only) and poedb's Developer API (GGG-OAuth wrapper).
- Prioritize async, and choose the HTTP stack with a future WASM build in mind (research to confirm
  wasm32-viable crates; reqwest's wasm backend is a likely fit).
- The spike was a hard gate and is now DONE (04-RESEARCH.md) — poe.ninja + RePoE surfaces confirmed.

</specifics>

<deferred>
## Deferred Ideas

- **WASM compilation** of the tool — a future release; only influences dependency selection now (D-05),
  not a Phase 4 deliverable.
- **SYNX-01** multiple build profiles, **SYNX-02** alternate price source (poe2scout fallback),
  **GENX-04** stale-price warning — v2, out of scope.
- Live per-item metadata lookups during `sync prices` — rejected in favor of the offline `sync metadata`
  map-build (single runtime price API).
- **poe2db scraping** and **poedb Developer API** as metadata sources — rejected after the spike
  (HTML-only / GGG-OAuth wrapper); RePoE-fork JSON exports chosen instead.

</deferred>

---

*Phase: 04-sync*
*Context gathered: 2026-05-25*
