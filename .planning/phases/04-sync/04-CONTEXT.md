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
  fetches poe.ninja economy and writes `prices.json`; **`sync metadata`** fetches poe2db
  (`https://poe2db.tw/sp/pob`) and builds/refreshes the committed category/class/baseType **mapping
  artifact** in the repo. League flags apply to `prices`; `metadata` is league-agnostic.
- **D-02:** Implement CLI-02 (`sync` runnable) and CLI-04 (combinable `--standard` + `--hc`; neither
  → current **softcore trade** league) on the `prices` mode. Each subcommand keeps `--help`.

### Refresh strategy & the mapping
- **D-03:** **Rebuild** `prices.json` from poe.ninja overviews each `sync prices` run, categorizing
  every item via the committed **overview → {filter category, `Class==` value}** mapping. Uniques get
  category/class from their overview + `baseType`. This DISCOVERS new valuable items + uniques
  automatically (activating Phase 3's dormant unique-block highlighting). The curated Phase-3 seed was
  the bootstrap; `sync prices` replaces it with live, mapped data.
- **D-04 (mapping source):** The mapping is a **committed repo artifact**, but NOT hand-typed —
  `sync metadata` populates/refreshes it from **poe2db `/sp/pob`** (item class/baseType/category).
  `sync prices` reads ONLY the committed map + poe.ninja (single runtime price API; deterministic;
  poe2db touched only by `metadata`). The map also drives WHICH poe.ninja overviews `prices` fetches.

### HTTP / dependencies
- **D-05:** **Async** HTTP (not blocking). Dependency choice is **weighted by WASM-compatibility** for
  a possible future WASM build — prefer a stack with a working `wasm32` target and avoid crates that
  hard-block it. `reqwest` is a candidate (its wasm backend uses browser fetch, sidestepping tokio);
  the researcher selects the concrete crate(s) and confirms wasm32 viability. WASM is a *forward
  guideline*, not a Phase 4 target (native is the deliverable).

### Spike (criterion 1 — gates all HTTP code)
- **D-06:** **Live-probe spike** of BOTH surfaces before any HTTP client code: actually hit
  poe.ninja (base URL, league-resolution endpoint, per-overview `type`/`overviewName`, response
  field names like `chaosValue`/`primaryValue`, `baseType`) and poe2db `/sp/pob` (format, the
  class/baseType/category fields, query shape). Capture confirmed findings in a SPIKE/RESEARCH
  artifact in the phase dir. No fetch/HTTP code is written until that doc exists; the plan's HTTP
  tasks depend on it.

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
- Exact async crate(s) + TLS backend (research-selected, wasm-aware); poe.ninja/poe2db rate-limiting
  + User-Agent etiquette; clap modeling of the default `prices` subcommand; the mapping artifact's
  file name/shape; how overviews are enumerated; retry/timeout policy.

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

### External (the spike confirms these — criterion 1)
- poe.ninja PoE2 economy API — base URL, league endpoint, per-overview `type`/`overviewName`,
  response fields (`chaosValue`/`primaryValue`/`divineValue`), `baseType`, gem level field.
- `https://poe2db.tw/sp/pob` — item metadata (class/baseType/category) for the mapping.
- `CLAUDE.md` external refs (poe2db.tw, pathofexile.com/item-filter) and the "price source = poe.ninja" decision.

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

- User's design: split sync into `prices` (poe.ninja, default) and `metadata` (poe2db → committed map)
  so the map isn't hand-maintained but is still a deterministic committed artifact.
- poe2db source confirmed: `https://poe2db.tw/sp/pob`.
- Prioritize async, and choose the HTTP stack with a future WASM build in mind (research to confirm
  wasm32-viable crates; reqwest's wasm backend is a likely fit).
- The spike is a hard gate — live-probe both APIs and document before writing any HTTP code.

</specifics>

<deferred>
## Deferred Ideas

- **WASM compilation** of the tool — a future release; only influences dependency selection now (D-05),
  not a Phase 4 deliverable.
- **SYNX-01** multiple build profiles, **SYNX-02** alternate price source (poe2scout fallback),
  **GENX-04** stale-price warning — v2, out of scope.
- Live per-item poe2db lookups during `sync prices` — rejected in favor of the offline `sync metadata`
  map-build (single runtime price API).

</deferred>

---

*Phase: 04-sync*
*Context gathered: 2026-05-25*
