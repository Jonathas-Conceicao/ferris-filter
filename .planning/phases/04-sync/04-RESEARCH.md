# Phase 4: Sync - Research

**Researched:** 2026-05-25
**Domain:** poe.ninja PoE2 economy API, async Rust HTTP, atomic file writes, CLI subcommand design
**Confidence:** HIGH (poe.ninja API confirmed live; poe2db finding is a blocker — see Open Questions)

---

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

- **D-01:** `sync` has two modes: `sync prices` (default — bare `ferrisfilter sync` runs it) fetches poe.ninja economy and writes `prices.json`; `sync metadata` fetches poe2db (`https://poe2db.tw/sp/pob`) and builds/refreshes the committed category/class/baseType mapping artifact in the repo. League flags apply to `prices`; `metadata` is league-agnostic.
- **D-02:** Implement CLI-02 (`sync` runnable) and CLI-04 (combinable `--standard` + `--hc`; neither → current softcore trade league) on the `prices` mode. Each subcommand keeps `--help`.
- **D-03:** Rebuild `prices.json` from poe.ninja overviews each `sync prices` run, categorizing every item via the committed overview → {filter category, `Class==` value} mapping. Uniques get category/class from their overview + `baseType`. This DISCOVERS new valuable items + uniques automatically. The curated Phase-3 seed was the bootstrap; `sync prices` replaces it with live, mapped data.
- **D-04 (mapping source):** The mapping is a committed repo artifact, NOT hand-typed — `sync metadata` populates/refreshes it from poe2db `/sp/pob`. `sync prices` reads ONLY the committed map + poe.ninja (single runtime price API; deterministic; poe2db touched only by `metadata`).
- **D-05:** Async HTTP. Dependency choice is weighted by WASM-compatibility. `reqwest` is a candidate; the researcher selects the concrete crate(s) and confirms wasm32 viability. WASM is a forward guideline, not a Phase 4 target (native is the deliverable).
- **D-06:** Live-probe spike of BOTH surfaces before any HTTP client code: poe.ninja and poe2db `/sp/pob`. Capture confirmed findings in this RESEARCH artifact.
- **D-07:** Atomic, failure-safe writes (SYNC-04): fetch fully, validate non-empty and parseable, write to temp file, atomically rename over `prices.json`.
- **D-08:** Live league resolution (CLI-04, SYNC-05): resolve current league NAME dynamically from poe.ninja's league-resolution endpoint per the flags.
- **D-09:** Uncut gems included only at level ≥ 18 (SYNC-03). `prices.json` schema: `{league, fetchedAt, items:[{name, baseType, class, category, exalt, chaos, div}]}`, Exalt/Chaos/Divine first.

### Claude's Discretion

- Exact async crate(s) + TLS backend (research-selected, wasm-aware); poe.ninja/poe2db rate-limiting + User-Agent etiquette; clap modeling of the default `prices` subcommand; the mapping artifact's file name/shape; how overviews are enumerated; retry/timeout policy.

### Deferred Ideas (OUT OF SCOPE)

- WASM compilation of the tool — future release only.
- SYNX-01 multiple build profiles, SYNX-02 alternate price source, GENX-04 stale-price warning — v2.
- Live per-item poe2db lookups during `sync prices`.
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| CLI-02 | User can run `ferrisfilter sync` to fetch prices | poe.ninja API confirmed; clap subcommand pattern documented |
| CLI-04 | `sync` accepts combinable `--standard` and `--hc` flags; neither → current softcore trade | League names from poe.ninja index-state endpoint; flag→league mapping documented |
| SYNC-01 | `sync` fetches PoE2 economy data from poe.ninja and writes `prices.json` | Base URL, endpoint path, all 21 type values confirmed live |
| SYNC-02 | `prices.json` ordered list; first three entries always Exalt, Chaos, Divine | Already in PricesFile/PriceItem model; ordering is implementation responsibility |
| SYNC-03 | Includes currency, uniques, other tracked drops; gems only at level ≥ 18 | All 21 type values confirmed; UncutGem type exists but currently empty (league-end timing issue; see notes) |
| SYNC-04 | Handles missing/empty responses without corrupting existing `prices.json` | `tempfile::NamedTempFile::persist` pattern documented; atomic on Windows |
| SYNC-05 | Records fetched league name and timestamp in `prices.json` | `PricesFile.league` + `PricesFile.fetched_at` fields already exist in model |
</phase_requirements>

---

## Summary

This research confirms the poe.ninja PoE2 economy API surface via live HTTP probing and documents the complete data model needed to implement `ferrisfilter sync`. The spike (criterion 1) is resolved for poe.ninja. The poe2db surface has a critical finding: `https://poe2db.tw/sp/pob` is a **Path of Building code-sharing page**, not an item metadata API — there is no machine-readable item class/baseType/category endpoint at that URL. This is a blocker for `sync metadata` as specified in D-01/D-04 and must be resolved with the user before implementation.

The poe.ninja PoE2 API uses a single unified endpoint: `GET /poe2/api/economy/exchange/{version}/overview?league={leagueName}&type={type}` with `version=current` for live data. It returns a currency-exchange data model (not PoE1's chaosValue/divineValue model). Prices are expressed in the `primary` currency (currently Divine Orbs) as `primaryValue`; chaos and exalt conversions come from `core.rates`. There is no separate `chaosValue`, `exaltedValue`, or `divineValue` field — these must be computed. The item name and category come from the `core.items` array (keyed by `id`), joined with `lines` on the `id` field.

Critically, PoE2 does **not** have `baseType` as a field in the API response — items are identified by `id` (a slug) and `name`. For unique items, the `baseType` (the filter `BaseType ==` value) is NOT exposed by poe.ninja's exchange API. Uniques tracked in the UniqueWeapon/UniqueArmour/etc. type endpoints currently return empty arrays for active leagues (confirmed live). The committed mapping artifact must carry the `id → baseType/class/category` mapping from a source that provides it (e.g., the GGG public data or hand-curation).

**Primary recommendation:** Implement `sync prices` using the confirmed exchange API endpoint (all 13 currency + 8 unique type values confirmed). Defer `sync metadata` or redesign its poe2db source — the existing `/sp/pob` URL is not usable for item metadata. The mapping artifact can bootstrap from the existing `currency.json`; the `sync metadata` redesign is an open question for the user.

---

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| HTTP fetch (poe.ninja) | CLI binary (native) | — | Single binary; async runtime added for network I/O only |
| League resolution | CLI (poe.ninja index-state) | GGG leagues API (fallback) | poe.ninja index-state is authoritative for which leagues it tracks |
| Price calculation (primaryValue → chaos/div/exalt) | CLI in-memory | — | Pure math transform; no separate service |
| mapping.json write (sync metadata) | CLI binary | — | File I/O in same process |
| prices.json write (sync prices) | CLI binary, atomic via tempfile | — | Failure-safe: temp + rename |

---

## SPIKE FINDINGS: poe.ninja PoE2 API (Criterion 1)

### Confirmed Live — 2026-05-25

All findings below are from direct HTTP probing of live endpoints with the FerrisFilter user-agent.

#### 1. Index-State / League Resolution

**Endpoint:** `GET https://poe.ninja/poe2/api/data/index-state`
**Status:** 200, no authentication required
**Rate limit:** Not documented; treat as public read-only.

**Response structure (confirmed):**
```json
{
  "economyLeagues": [
    {"name": "Fate of the Vaal", "url": "vaal",     "displayName": "Fate of the Vaal", "hardcore": false, "indexed": true},
    {"name": "HC Fate of the Vaal","url":"vaalhc",  "displayName":"HC Fate of the Vaal","hardcore": true, "indexed": false},
    {"name": "Standard",          "url": "standard","displayName": "Standard",          "hardcore": false, "indexed": false},
    {"name": "Hardcore",          "url": "hardcore","displayName": "Hardcore",          "hardcore": true,  "indexed": false}
  ],
  "oldEconomyLeagues": [],
  "snapshotVersions": [
    {
      "url": "vaal",
      "name": "Fate of the Vaal",
      "version": "0243-20260525-27410",
      ...
    }
  ]
}
```

**Flag → League Name mapping:**

| Flags | League to Use | Source |
|-------|--------------|--------|
| (none) | First `economyLeagues` entry where `hardcore=false` and `indexed=true` | Softcore trade |
| `--hc` | First `economyLeagues` entry where `hardcore=true` | Hardcore trade |
| `--standard` | League with `url="standard"` | Permanent standard |
| `--standard --hc` | League with `url="hardcore"` | Permanent hardcore |

**Implementation note:** Call the index-state endpoint first, select the league name by the flag combination above, then use that `name` as the `league` query parameter for economy calls. Record the resolved `name` in `prices.json.league`.

#### 2. Economy Overview Endpoint

**Endpoint:** `GET https://poe.ninja/poe2/api/economy/exchange/{version}/overview`
- Use `version=current` for live data (hardcoded string "current" — confirmed from JS source)
- Query params: `league={leagueName}` (full display name, e.g., `Fate+of+the+Vaal`) and `type={type}`

**Example (confirmed live 2026-05-25):**
```
GET https://poe.ninja/poe2/api/economy/exchange/current/overview?league=Fate+of+the+Vaal&type=Currency
```

**Response structure:**
```json
{
  "core": {
    "items": [
      {"id": "divine",  "name": "Divine Orb",  "image": "...", "category": "Currency", "detailsId": "divine-orb"},
      {"id": "exalted", "name": "Exalted Orb", ...},
      {"id": "chaos",   "name": "Chaos Orb",   ...}
    ],
    "rates": {
      "exalted": 149.0,
      "chaos": 32.24
    },
    "primary": "divine",
    "secondary": "exalted"
  },
  "lines": [
    {
      "id": "divine",
      "primaryValue": 1.0,
      "volumePrimaryValue": 1320.0,
      "maxVolumeCurrency": "exalted",
      "maxVolumeRate": 0.006712,
      "sparkline": {"totalChange": -12.52, "data": [...]}
    },
    {
      "id": "chaos",
      "primaryValue": 0.03102,
      ...
    }
  ],
  "items": []
}
```

**IMPORTANT — There is no `chaosValue`, `divineValue`, or `exaltedValue` field.** The PoE2 API uses a currency-exchange model, not PoE1's item-price model. All prices are expressed as `primaryValue` in the `core.primary` currency.

**Price conversion (verified by live data math):**

| Target field | Formula | Example (Divine as primary) |
|-------------|---------|----------------------------|
| `chaos` (in prices.json) | `primaryValue × core.rates.chaos` | 0.03102 × 32.24 = 1.0001 ✓ |
| `div` (in prices.json) | `primaryValue` (primary IS divine) | 1.0 |
| `exalt` (in prices.json) | `primaryValue × core.rates.exalted` | 0.006712 × 149.0 = 1.0001 ✓ |

When `core.primary` is "chaos" (e.g., for a newer league or when only chaos is indexed), formulas shift: `chaos = primaryValue`, `div = primaryValue / core.rates.divine` (if present).

**Joining items and lines:** `core.items` provides name and category; `lines` provides the price. Join on `id`. If `core.items` is empty but `lines` is not (this can happen), fall back to using `id` as a displayable name (not ideal). In practice, `core.items` is populated when `lines` is non-empty.

**No `baseType` field.** The API returns `name` and `id` (a slug). For currency, `baseType == name`. For uniques, the poe.ninja exchange API does not expose `baseType` — see the unique items section below.

#### 3. Confirmed Type Values (all return HTTP 200)

All 21 values tested against `league=Fate+of+the+Vaal`:

**Currency types (exchange data — all have populated `lines` arrays for active leagues):**

| type= | Category name | Notes |
|-------|--------------|-------|
| `Currency` | Stackable Currency | Core orbs, crafting currency (87 items confirmed) |
| `Fragment` | Fragments | — |
| `Rune` | Runes | — |
| `Essence` | Essences | — |
| `Breach` | Breach | Catalysts, splinters (29 items confirmed) |
| `Delirium` | Delirium | Liquid emotions, splinters (23 items confirmed) |
| `Expedition` | Expedition | Logbooks, artifacts (13 items confirmed) |
| `Omen` | Omens | — |
| `SoulCore` | Soul Cores | — |
| `Waystone` | Waystones | — |
| `UncutGem` | Uncut Gems | Currently empty — see gem note |
| `Talisman` | Talismans | — |
| `Abyss` | Abyssal Eyes/items | 33 items confirmed |

**Unique item types (return HTTP 200 but currently empty lines arrays):**

| type= | Corresponds to |
|-------|--------------|
| `UniqueWeapon` | Unique weapons by slot |
| `UniqueArmour` | Unique armour by slot |
| `UniqueAccessory` | Rings, amulets, belts |
| `UniqueFlask` | Unique flasks |
| `UniqueCharm` | Unique charms |
| `UniqueJewel` | Unique jewels |
| `UniqueMap` | Unique waystones |
| `SanctumRelic` | Sanctum relics |

**Note on empty types:** Several type values returned empty `lines` arrays (Fragment, Rune, Essence, Omen, SoulCore, Waystone, UncutGem, Talisman, and all Unique types) for the current "Fate of the Vaal" league. This is consistent with the poe.ninja announcement: "There is no River API for Path of Exile 2 yet, so prices are estimated from the official trade API." Unique item pricing is experimental and may not be populated early in league. The `sync prices` implementation must handle empty responses gracefully (skip the type, do not produce a malformed prices.json).

#### 4. Gem Level — UncutGem type

The `UncutGem` type currently returns empty data. When populated, the `id` field in lines encodes the gem level as part of the slug (e.g., `"id": "uncut-spirit-gem-l18"` — the exact format must be confirmed when data is available). The implementation should:
- Fetch the `UncutGem` type
- Filter by extracting gem level from the item `id` or `name` field
- Include only items where gem level >= 18

The exact gem-level field format is **[UNVERIFIED: cannot be confirmed live because UncutGem type currently returns empty]**. A fallback: filter where `name` or `id` contains a level indicator >= 18. The executor must verify this when `UncutGem` data becomes available.

#### 5. No GGG API Needed for PoE2 League Names

The GGG `api.pathofexile.com/leagues?realm=poe2` endpoint currently returns PoE1 league data (confirmed: returns "Mirage" etc. with `realm: "pc"`). Use **only** the poe.ninja index-state endpoint for PoE2 league resolution.

---

## SPIKE FINDINGS: poe2db `/sp/pob` (Criterion 1 — BLOCKER)

**URL probed:** `https://poe2db.tw/sp/pob` (and `https://poe2db.tw/us/pob`)
**Status:** 200 — page serves successfully

**Critical finding:** `https://poe2db.tw/sp/pob` is the **Path of Building code sharing page** (equivalent to pobb.in), not an item class/baseType/category database API. The page renders:
- A PoB build code paste-and-import UI
- Two public integration URLs: `GET /pob/:id/raw` and `POST /pob/api/gen`
- No item class, baseType, or category data of any kind

**This directly contradicts D-04**, which states: "`sync metadata` populates/refreshes it from poe2db `/sp/pob` (item class/baseType/category)."

**poe2db.tw does provide item data** (it has `/us/Items`, `/us/Unique_item`, `/us/Gem` pages) but these are dynamically-rendered HTML pages loaded by JavaScript — there is no public JSON API endpoint for item class/baseType/category data at any confirmed URL.

**Options for the executor to discuss with the user:**
1. **Use the existing `currency.json` as the mapping source** — it already has `class`, `bases`, and category names. `sync metadata` would only need to handle new items (uniques, gems) added by live sync.
2. **Hard-code the mapping artifact** from the confirmed poe.ninja `core.items[].category` field — this field is present in live responses and carries category strings like `"Currency"`.
3. **Scrape poe2db HTML** — brittle and not appropriate for a CLI tool.
4. **Use the GGG public data files** — `https://raw.githubusercontent.com/poe-tool-dev/latest-patch-data/main/PoE2/BaseItemTypes.dat.json` (community-maintained extracts from GGG's data) provides baseType + class data.

See Open Questions below.

---

## Standard Stack

### Core
| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| `reqwest` | 0.13.3 | Async HTTP client | wasm32 target support (uses browser Fetch on wasm); rustls TLS; serde_json feature; only major async HTTP crate with confirmed wasm support |
| `tokio` | 1.52.3 | Async runtime (native only) | Required by reqwest on native; already standard in Rust async ecosystem |
| `serde_json` | 1.0.150 | JSON deserialization | Already in Cargo.toml; parse exchange API response |
| `tempfile` | 3.27.0 | Atomic file writes | `NamedTempFile::persist` is atomic on Windows (MoveFileExW semantics); cross-platform |

[VERIFIED: crates.io registry — versions confirmed 2026-05-25]

### Supporting
| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| `wiremock` | 0.6.5 | HTTP mock server for tests | Unit/integration testing sync without live API calls |
| `chrono` | 2.0 OR `time` | Timestamp for `fetchedAt` field | Formatting UTC ISO-8601 timestamp in `prices.json` |

[VERIFIED: crates.io registry]

### reqwest Feature Flags

For native (Phase 4 target):
```toml
reqwest = { version = "0.13", default-features = false, features = ["rustls-tls", "json"] }
```

**Why `default-features = false`:** The default TLS in 0.13 (`rustls` with aws-lc-rs) requires a C compiler for aws-lc-rs. On some systems this causes build issues. Use `rustls-tls` feature which selects rustls without requiring a C toolchain. [ASSUMED: may need `rustls-no-provider` or a different feature combo; executor must `cargo build` to verify on Windows]

**Why not `native-tls`:** Harder to port to wasm32 future target. Rustls is pure Rust.

**wasm32 forward compatibility:** reqwest 0.13 explicitly lists `wasm32-unknown-unknown` as a target. On wasm, reqwest uses `web-sys` and browser Fetch API — no tokio dependency on that target. The `json` feature works on both targets. The `default-tls`/`rustls` features are gated to `cfg(not(target_arch = "wasm32"))` in reqwest's own Cargo.toml.

### tokio Runtime Shape

```toml
# Only if using #[tokio::main]; alternatively use tokio::runtime::Runtime
tokio = { version = "1", features = ["rt-multi-thread", "macros"] }
```

The existing `cmd_sync` stub is `fn cmd_sync()`. It needs to become either:
- `async fn cmd_sync()` called from a `#[tokio::main] async fn main()`, which requires changing `main.rs`
- OR `fn cmd_sync()` that creates a runtime internally: `tokio::runtime::Runtime::new().unwrap().block_on(async { ... })`

The second approach (runtime-internal) avoids changing the generate path and keeps the binary entry point sync. [ASSUMED: either approach works; prefer the simpler runtime-internal to minimize diff]

### Alternatives Considered
| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| `reqwest` | `ureq` (sync, no tokio) | ureq is simpler but sync-only and no wasm future path |
| `reqwest` | `surf` | surf is less maintained; wasm support exists but ecosystem smaller |
| `tempfile` | Manual write-temp-rename | error-prone cross-platform edge cases; tempfile is well-tested |
| `wiremock` | `mockito` | Both work; wiremock's tower-based server is more composable |

**Installation:**
```bash
cargo add reqwest --no-default-features --features rustls-tls,json
cargo add tokio --features rt-multi-thread,macros
cargo add tempfile
```

---

## Architecture Patterns

### System Architecture Diagram

```
ferrisfilter sync [prices|metadata] [flags]
         │
         ├─ sync prices
         │    │
         │    ├─ GET /poe2/api/data/index-state → resolve league name
         │    │
         │    ├─ for each (type, category) in mapping.json:
         │    │    GET /poe2/api/economy/exchange/current/overview
         │    │         ?league={leagueName}&type={type}
         │    │         │
         │    │         └─ join core.items + lines on id
         │    │              convert primaryValue → chaos/div/exalt
         │    │              emit PriceItem{name, baseType, class, category, ...}
         │    │
         │    ├─ gem filtering: UncutGem items → level ≥ 18 only
         │    ├─ validate: Exalt + Chaos + Divine present and non-zero
         │    ├─ write temp file → validate → NamedTempFile::persist → prices.json
         │    └─ record league name + UTC timestamp in PricesFile header
         │
         └─ sync metadata
              │
              └─ [SOURCE TBD — see Open Questions]
                   build mapping: type → {category, class, baseType rules}
                   write temp → validate → persist → mapping.json
```

### Recommended Project Structure
```
src/
├── sync/
│   ├── mod.rs           # pub fn cmd_sync(args) -> miette::Result<()>
│   ├── client.rs        # reqwest client, fetch_overview(), fetch_index_state()
│   ├── league.rs        # resolve_league(flags, index_state) -> String
│   ├── prices.rs        # build_prices(overviews, mapping) -> Vec<PriceItem>
│   └── metadata.rs      # cmd_sync_metadata stub (or full impl pending source)
├── models/
│   ├── prices.rs        # existing PricesFile/PriceItem
│   └── ...
mapping.json             # committed artifact: type → {category, class, baseType rules}
```

### Pattern 1: reqwest + tokio::runtime (sync entry point preserved)

```rust
// src/main.rs — no change to existing generate path
fn cmd_sync(args: &SyncArgs) -> miette::Result<()> {
    let rt = tokio::runtime::Runtime::new()
        .map_err(|e| miette::miette!("Cannot create async runtime: {e}"))?;
    rt.block_on(run_sync(args))
}

async fn run_sync(args: &SyncArgs) -> miette::Result<()> {
    let client = reqwest::Client::builder()
        .user_agent("FerrisFilter/0.1 (github.com/USER/ferrisfilter; contact: EMAIL)")
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .map_err(|e| miette::miette!("Cannot build HTTP client: {e}"))?;
    // ... fetch, transform, write
    Ok(())
}
```

### Pattern 2: poe.ninja league resolution

```rust
// GET https://poe.ninja/poe2/api/data/index-state
// Response field used:
#[derive(Deserialize)]
struct IndexState {
    #[serde(rename = "economyLeagues")]
    economy_leagues: Vec<EconomyLeague>,
}
#[derive(Deserialize)]
struct EconomyLeague {
    name: String,
    url: String,
    hardcore: bool,
    indexed: bool,
}

fn resolve_league(state: &IndexState, standard: bool, hc: bool) -> miette::Result<String> {
    match (standard, hc) {
        (false, false) => state.economy_leagues.iter()
            .find(|l| !l.hardcore && l.indexed)
            .map(|l| l.name.clone())
            .ok_or_else(|| miette::miette!("No active softcore trade league found")),
        (false, true) => state.economy_leagues.iter()
            .find(|l| l.hardcore && l.url != "hardcore")
            .map(|l| l.name.clone())
            .ok_or_else(|| miette::miette!("No active hardcore trade league found")),
        (true, false) => Ok("Standard".into()),
        (true, true) => Ok("Hardcore".into()),
    }
}
```

### Pattern 3: poe.ninja exchange API fetch + price conversion

```rust
// Response models
#[derive(Deserialize)]
struct ExchangeOverview {
    core: ExchangeCore,
    lines: Vec<ExchangeLine>,
}
#[derive(Deserialize)]
struct ExchangeCore {
    items: Vec<ExchangeItem>,
    rates: std::collections::HashMap<String, f64>,
    primary: String,       // e.g. "divine"
    secondary: String,     // e.g. "exalted"
}
#[derive(Deserialize)]
struct ExchangeItem {
    id: String,
    name: String,
    category: Option<String>,
    #[serde(rename = "detailsId")]
    details_id: Option<String>,
}
#[derive(Deserialize)]
struct ExchangeLine {
    id: String,
    #[serde(rename = "primaryValue")]
    primary_value: f64,
}

// Price conversion:
// chaos_value = primary_value * core.rates["chaos"]
// div_value   = primary_value  (when primary == "divine")
// exalt_value = primary_value * core.rates["exalted"]
//
// Handle: if core.rates is empty (no trade data yet), skip type
```

### Pattern 4: Atomic write with tempfile

```rust
use tempfile::NamedTempFile;
use std::io::Write;

fn write_prices_atomic(path: &str, prices: &PricesFile) -> miette::Result<()> {
    // Validate before touching any files
    if prices.items.is_empty() {
        return Err(miette::miette!("Refusing to write empty prices.json"));
    }
    
    let json = serde_json::to_string_pretty(prices)
        .map_err(|e| miette::miette!("Cannot serialize prices: {e}"))?;
    
    // Write to temp file in SAME DIRECTORY as target (same filesystem required for rename)
    let target = std::path::Path::new(path);
    let dir = target.parent().unwrap_or(std::path::Path::new("."));
    
    let mut temp = NamedTempFile::new_in(dir)
        .map_err(|e| miette::miette!("Cannot create temp file: {e}"))?;
    temp.write_all(json.as_bytes())
        .map_err(|e| miette::miette!("Cannot write temp file: {e}"))?;
    
    // Atomic rename — replaces existing prices.json atomically on both Windows and Unix
    temp.persist(target)
        .map_err(|e| miette::miette!("Cannot persist prices.json: {e}"))?;
    
    Ok(())
}
```

**Windows atomicity:** `NamedTempFile::persist` uses `MoveFileExW` with `MOVEFILE_REPLACE_EXISTING` on Windows — this is atomic at the filesystem level on NTFS. The existing `prices.json` is never in a partial state from the reader's perspective. [CITED: tempfile docs.rs — "atomically replacing any existing file at the target path"; "Windows benefits from atomic rename semantics natively"]

**Same-directory requirement:** The temp file MUST be created in the same directory as the target (use `NamedTempFile::new_in(dir)`) because `persist` calls rename which cannot cross filesystem boundaries.

### Pattern 5: clap derive — `sync prices` as default subcommand

clap 4.x (currently 4.6 in Cargo.toml) does NOT have a `default_subcommand` derive attribute (the builder API's `Command::default_subcommand` method does not have a derive equivalent). The correct pattern is:

```rust
// src/cli.rs
#[derive(Subcommand)]
pub enum Commands {
    #[command(alias = "gen")]
    Generate { /* ... */ },
    /// Fetch live PoE2 economy data
    Sync {
        #[command(subcommand)]
        sub: Option<SyncSubcommand>,
        // League flags live HERE (visible in `sync --help` even without subcommand)
        // ... or on SyncSubcommand::Prices
    },
}

#[derive(Subcommand)]
pub enum SyncSubcommand {
    /// Fetch prices from poe.ninja (default)
    Prices {
        /// Target softcore trade league (default)
        #[arg(long)]
        standard: bool,
        /// Target hardcore league
        #[arg(long)]
        hc: bool,
    },
    /// Fetch item metadata from poe2db
    Metadata,
}

// In main.rs:
Commands::Sync { sub } => {
    let sub = sub.unwrap_or(SyncSubcommand::Prices { standard: false, hc: false });
    match sub {
        SyncSubcommand::Prices { standard, hc } => cmd_sync_prices(standard, hc)?,
        SyncSubcommand::Metadata => cmd_sync_metadata()?,
    }
}
```

**Note on flags:** If league flags (`--standard`, `--hc`) should be usable on the bare `ferrisfilter sync` (without typing `sync prices`), they must go on the `Sync` variant, not on `SyncSubcommand::Prices`. Then the dispatch logic maps the `None` subcommand + those flags to the prices behavior. Consider what looks better to the user: `sync --hc` vs `sync prices --hc`.

### Anti-Patterns to Avoid

- **Fetching all types unconditionally:** Empty responses are normal at league start/end. Always check `lines.is_empty()` before processing.
- **Using the PoE1 API paths:** The old `api/data/currencyoverview` and `api/data/itemoverview` endpoints return HTTP 404 on poe.ninja as of 2026. The PoE2 API is at a completely different path.
- **Treating `primaryValue` as chaos value:** It is in the `core.primary` currency (currently divine), not chaos. Always use `core.rates` for conversion.
- **Cross-filesystem temp files:** Create temp file with `NamedTempFile::new_in(dir)` where `dir` is the parent of `prices.json`, not `/tmp` (which may be a different mount on Unix, and a different drive on Windows).
- **Assuming `baseType` exists in the API response:** There is no `baseType` field in the exchange API. For currency, `name == baseType`. For uniques, the mapping artifact must provide the baseType.
- **Using the GGG leagues API for PoE2 leagues:** It currently returns PoE1 Mirage data when `realm=poe2` is passed. Use the poe.ninja index-state instead.

---

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Atomic file replace | Custom temp+rename logic | `tempfile::NamedTempFile::persist` | Cross-platform edge cases; Windows requires same-filesystem move; tempfile handles them |
| TLS certificate handling | Custom rustls setup | reqwest feature flags | TLS is a security-critical domain; reqwest bundles correct defaults |
| HTTP retry + timeout | Manual retry loop | reqwest builder `.timeout()` + tower retry | reqwest uses tower middleware; already correct |
| Gem level parsing from unknown format | Custom slug parser | Defer until UncutGem data is available | Cannot verify format against empty response |

**Key insight:** poe.ninja's API is not documented — every field name has been confirmed from live probing. Do not assume additional fields exist; deserialize only what is confirmed above.

---

## Mapping Artifact

### Recommended Shape: `mapping.json`

```json
{
  "version": 1,
  "overviewTypes": [
    {
      "type": "Currency",
      "category": "Currency",
      "class": "Stackable Currency",
      "baseTypeIsName": true
    },
    {
      "type": "Breach",
      "category": "Breach",
      "class": "Stackable Currency",
      "baseTypeIsName": true
    },
    {
      "type": "UniqueWeapon",
      "category": "Uniques",
      "class": null,
      "baseTypeIsName": false,
      "baseTypeFromPricesName": true
    }
  ],
  "gemFilter": {
    "type": "UncutGem",
    "category": "Uncut Gems",
    "class": "Stackable Currency",
    "minLevel": 18
  }
}
```

**Fields:**
- `type` — the poe.ninja `type=` parameter value
- `category` — the `PriceItem.category` value to emit in prices.json
- `class` — the `PriceItem.class` value (filter `Class ==` value)
- `baseTypeIsName` — if true, `baseType = name` (for all currency types)
- `baseTypeFromPricesName` — for uniques where baseType must be derived separately

**Alternative (simpler):** Bootstrap `mapping.json` directly from `currency.json` via the `seed_prices` binary pattern. The mapping only needs to cover what the `currency.json` already enumerates plus the 8 unique type slots. Given that poe2db is unusable, `sync metadata` could instead write this mapping from a bundled or hand-maintained static table.

**Recommended for Phase 4 scope:** Commit the mapping artifact as a hand-maintained JSON file (derived from `currency.json`) rather than implementing `sync metadata` against an unknown source. The mapping rarely changes league-to-league. This unblocks `sync prices` immediately.

---

## Common Pitfalls

### Pitfall 1: Empty Lines Array
**What goes wrong:** Multiple `type=` values (Rune, Essence, Fragment, all Unique types) return `{"lines": []}` for active leagues. Code that asserts non-empty or that doesn't guard `core.items` join will produce panics or empty output.
**Why it happens:** poe.ninja PoE2 unique item prices are estimated from the trade API which is not always populated. Currency types can be empty at league-start before trades occur.
**How to avoid:** Skip any type where `lines.is_empty()`. Do not fail — continue to next type. Log a warning to stderr.
**Warning signs:** prices.json missing categories that should be present.

### Pitfall 2: primaryValue Arithmetic in Wrong Currency
**What goes wrong:** Using `primaryValue` directly as chaos value when `core.primary = "divine"`. Divine ≈ 32 chaos; a 0.5 divine item would be listed at 0.5 chaos (drastically wrong tier).
**Why it happens:** Conflating the PoE1 `chaosValue` field with PoE2's `primaryValue`.
**How to avoid:** Always check `core.primary` and `core.rates` before computing values. Assert `core.rates` is non-empty before conversion.
**Warning signs:** Tier calculations giving nonsensical results; reference currency math check fails.

### Pitfall 3: League Names are Case-Sensitive and Space-Sensitive
**What goes wrong:** `league=fate+of+the+vaal` returns empty; `league=Fate+of+the+Vaal` returns data.
**Why it happens:** The URL slug `vaal` is different from the display name `Fate of the Vaal`. The API uses the display name in the `league=` parameter.
**How to avoid:** Always use the `name` field from index-state (not the `url`/slug) as the query parameter.
**Warning signs:** All types return `{"lines": []}` unexpectedly.

### Pitfall 4: Windows-Specific Tempfile Location
**What goes wrong:** `NamedTempFile::new()` creates the temp file in `%TEMP%` (possibly a different drive). `persist()` then fails with "cross-device link".
**Why it happens:** rename cannot cross filesystem/drive boundaries.
**How to avoid:** Use `NamedTempFile::new_in(parent_dir_of_prices_json)`. The repo root and the game's config folder are likely on the same drive (C:) but not guaranteed.
**Warning signs:** `persist()` returns an error about cross-device links.

### Pitfall 5: poe.ninja Undocumented API Changes
**What goes wrong:** The endpoint path or response schema changes without notice.
**Why it happens:** poe.ninja's PoE2 API is entirely undocumented and discovered through reverse engineering.
**How to avoid:** Use `#[serde(deny_unknown_fields)]` selectively (or not at all on API response structs — use lenient parsing). Log the full response body on parse failure. Keep the response models flat and minimal.
**Warning signs:** HTTP 404s or deserialization errors after a poe.ninja site update.

### Pitfall 6: Exalt/Chaos/Divine Ordering
**What goes wrong:** prices.json is written with the three reference currencies not first.
**Why it happens:** poe.ninja's Currency type returns items in volume order, not in our required Exalt/Chaos/Divine order.
**How to avoid:** After collecting all items, explicitly promote Exalted Orb, Chaos Orb, and Divine Orb to positions [0], [1], [2] before writing. The `PricesFile` model has this as a structural contract (tests verify it).
**Warning signs:** `TIER-04` error fires at generate time ("reference currency missing or zero").

---

## Code Examples

### Live API Request with User-Agent
```rust
// Source: live spike 2026-05-25 + reqwest docs
let client = reqwest::Client::builder()
    .user_agent("FerrisFilter/0.1 (github.com/OWNER/ferrisfilter; contact: jonathasaoc@gmail.com)")
    .timeout(std::time::Duration::from_secs(30))
    .build()?;

let index: IndexState = client
    .get("https://poe.ninja/poe2/api/data/index-state")
    .send().await?
    .error_for_status()?
    .json().await?;

let overview: ExchangeOverview = client
    .get("https://poe.ninja/poe2/api/economy/exchange/current/overview")
    .query(&[("league", &league_name), ("type", &"Currency")])
    .send().await?
    .error_for_status()?
    .json().await?;
```

### Timestamp for `fetchedAt`
```rust
// Using std only (no chrono dependency if desired):
let fetched_at = std::time::SystemTime::now()
    .duration_since(std::time::UNIX_EPOCH)
    .map(|d| d.as_secs().to_string())
    .unwrap_or_else(|_| "0".to_string());
// OR use RFC3339 via chrono: chrono::Utc::now().to_rfc3339()
// prices.json schema shows `fetchedAt` as a string; format is implementation choice
// (Unix timestamp int or ISO-8601 string are both readable by load_json)
```

---

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| poe.ninja PoE1 API (`/api/data/currencyoverview`, `/api/data/itemoverview`) | poe.ninja PoE2 API (`/poe2/api/economy/exchange/current/overview`) | 2024 with PoE2 EA launch | Completely different endpoint hierarchy; old endpoints return 404 |
| `chaosValue` / `divineValue` / `exaltedValue` fields | `primaryValue` + `core.rates` conversion | 2024 (PoE2 currency exchange model) | Must compute chaos/div/exalt from rate table |
| GGG leagues API for league names | poe.ninja index-state | — | GGG API returns PoE1 leagues even with `realm=poe2` |

**Deprecated/outdated:**
- All PoE1-era poe.ninja endpoint patterns: `/api/data/*` — return 404 for poe.ninja as of 2025/2026.
- GitHub repos documenting PoE2 API with `currencyexchange/overview` path — appears to be a prior version of the endpoint; current path is `exchange` not `currencyexchange`.

---

## Open Questions

1. **poe2db `/sp/pob` source for `sync metadata`**
   - What we know: `/sp/pob` is a PoB code-sharing page. No JSON item API exists at that URL.
   - What's unclear: What did the user intend as the source for item class/baseType/category mapping? Was `/sp/pob` a mistaken reference? The poe2db HTML items pages exist but have no JSON API.
   - Recommendation: Present three options to the user before implementing `sync metadata`: (A) hand-maintain `mapping.json` bootstrapped from `currency.json` (no live fetch needed for Phase 4); (B) use community-maintained GGG data extracts (`poe-tool-dev/latest-patch-data`); (C) descope `sync metadata` from Phase 4 and ship `sync prices` only against the committed seed mapping.

2. **UncutGem gem level field format**
   - What we know: `UncutGem` type returns empty lines arrays in current league (Fate of the Vaal).
   - What's unclear: The exact JSON field or `id` slug pattern that encodes gem level.
   - Recommendation: Implement a lenient parser that checks `name` for a level pattern (e.g., "Uncut Spirit Gem (Level 18)") and `id` slug (e.g., `uncut-spirit-gem-l18`). Add a test fixture once data is available.

3. **`exalt` field in prices.json**
   - What we know: `core.rates.exalted` converts primaryValue to exalt units.
   - What's unclear: Whether exalt pricing is needed/useful in prices.json — exalt is a tradeable currency, so its own price in terms of exalt would always be 1.
   - Recommendation: For non-currency items, compute `exalt = primaryValue * core.rates.exalted`. For Exalted Orb itself, hardcode `exalt: 1.0`.

---

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| Internet access | poe.ninja API calls | Assumed ✓ | — | — |
| Rust toolchain (cargo, rustc) | reqwest compilation | ✓ | Cargo.toml edition="2024" implies ≥1.85 | — |
| C compiler (cl.exe or gcc) | reqwest default TLS (aws-lc-rs) | Unknown | — | Use `rustls-tls` feature (pure Rust) |

**Missing dependencies with no fallback:**
- None identified.

**Missing dependencies with fallback:**
- C compiler: required only by reqwest's default `aws-lc-rs` crypto provider. Use `rustls-tls` feature flag to avoid it entirely.

---

## Security Domain

### Applicable ASVS Categories

| ASVS Category | Applies | Standard Control |
|---------------|---------|-----------------|
| V2 Authentication | no | poe.ninja API is public; no auth |
| V3 Session Management | no | stateless CLI; no sessions |
| V4 Access Control | no | CLI tool; no user roles |
| V5 Input Validation | yes | Validate API response shape before writing; non-empty check |
| V6 Cryptography | partial | rustls handles TLS for HTTPS fetch; no custom crypto |

### Known Threat Patterns

| Pattern | STRIDE | Standard Mitigation |
|---------|--------|---------------------|
| MITM on poe.ninja API response | Tampering | HTTPS via rustls; `error_for_status()` on HTTP errors |
| Malformed/empty response corrupts prices.json | Tampering | Validate non-empty + parseable before `persist()`; temp+rename atomicity |
| Path traversal in `--output` flag | Tampering | Inherited from Phase 1; already mitigated |

---

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | `version=current` is a stable poe.ninja endpoint parameter | SPIKE: Economy Overview Endpoint | Endpoint breaks; executor must re-probe and find new version string |
| A2 | All 21 type values tested will remain valid in future leagues | SPIKE: Type Values | Executor must re-validate at phase start; add per-type error handling |
| A3 | UncutGem gem level is encoded in `id` or `name` field | SPIKE: Gem Level | Gem filter ≥18 cannot be implemented; descope or defer |
| A4 | poe.ninja index-state `indexed: true` flag correctly identifies the current active trade league | SPIKE: League Resolution | Wrong league fetched; prices are for wrong league |
| A5 | `tempfile::persist` behavior on Windows is atomic (same as MoveFileExW with REPLACE_EXISTING) | Atomic Write Pattern | prices.json could be corrupted on Windows in edge cases |
| A6 | reqwest `rustls-tls` feature avoids C compiler dependency | Standard Stack | Build fails on Windows requiring MSVC or MinGW; use `native-tls-vendored` as fallback |

---

## Sources

### Primary (HIGH confidence)
- Live HTTP probe of `https://poe.ninja/poe2/api/data/index-state` — leagues, URL slugs (2026-05-25)
- Live HTTP probe of `https://poe.ninja/poe2/api/economy/exchange/current/overview?league=Fate+of+the+Vaal&type=Currency` — full response structure, field names, rates (2026-05-25)
- Live HTTP probing of all 21 type values (Currency through SanctumRelic) — HTTP 200 confirmed (2026-05-25)
- poe.ninja Astro JS bundle (`a.D5J5VWqj.mjs`) — `/poe2/api/economy/exchange/{version}/overview` path literal found in source (2026-05-25)
- poe.ninja Astro JS bundle (`a.BKj8Ejdu.mjs`) — `/poe2/api/data/index-state` path literal found in source (2026-05-25)
- [crates.io registry](https://crates.io) — reqwest 0.13.3, tokio 1.52.3, tempfile 3.27.0, wiremock 0.6.5 (2026-05-25)
- [reqwest Cargo.toml](https://raw.githubusercontent.com/seanmonstar/reqwest/refs/heads/master/Cargo.toml) — wasm32 target dependency structure confirmed (2026-05-25)
- [tempfile docs.rs](https://docs.rs/tempfile/latest/tempfile/struct.NamedTempFile.html) — `persist` atomicity on Windows (2026-05-25)
- Live HTTP probe of `https://poe2db.tw/us/pob` — confirmed to be PoB code-sharing page (2026-05-25)
- [poe.ninja blog post](https://poe.ninja/posts/poe2-unique-items) — "no River API for PoE2 yet" confirmation

### Secondary (MEDIUM confidence)
- [GitHub: Davenads/poeninjaAPI-2025](https://github.com/Davenads/poeninjaAPI-2025) — overviewName aliases cross-checked with live probing results
- [docs.rs: std::fs::rename](https://doc.rust-lang.org/std/fs/fn.rename.html) — platform behavior notes

### Tertiary (LOW confidence)
- [poe.ninja posts/poe2-economy-and-rise-of-the-abyssal](https://poe.ninja/posts/poe2-economy-and-rise-of-the-abyssal) — "14 currency categories" (aligns with 13 confirmed + 1 empty category pattern)

---

## Metadata

**Confidence breakdown:**
- Standard stack (reqwest, tokio, tempfile): HIGH — versions from registry, features from source
- poe.ninja API endpoint + field names: HIGH — live confirmed 2026-05-25
- poe.ninja type values (21 total): HIGH — all HTTP 200 confirmed live
- poe2db finding (blocker): HIGH — live confirmed the page is a PoB tool
- Atomic write on Windows: HIGH — documented in tempfile crate
- Gem level field format: LOW — UncutGem type currently empty
- Unique items data availability: MEDIUM — endpoints exist (200) but lines empty

**Research date:** 2026-05-25
**Valid until:** 2026-06-25 (poe.ninja API paths are undocumented; re-probe if poe.ninja updates)

---

## Spike Resolution — Metadata Source (plan-phase, 2026-05-25)

The poe2db `/sp/pob` blocker (above) was resolved with the user via a focused re-spike:

- **poe2db.tw** — REJECTED: HTML-only. Per-class pages (`/us/Stackable_Currency`, `/us/Unique_item`,
  …) are scrapable item-card grids (~169 currency bases listed) but expose no JSON API / no embedded
  data block. Scraping would add a brittle HTML-parser dependency.
- **poedb.tw `/us/Developer_API`** — REJECTED: it only wraps GGG's official OAuth API
  (accounts/characters/stash/trade/leagues) + the unofficial POESESSID site API; no static
  item-metadata JSON, and no PoE2-specific endpoint.
- **CHOSEN — RePoE-fork PoE2 JSON exports** (user-provided, live-confirmed):
  - `https://repoe-fork.github.io/poe2/base_items.json` — valid JSON keyed by metadata path; each
    entry has `name` + `item_class` (e.g. "Divine Orb" → `StackableCurrency`), `drop_level`, `tags`,
    `release_state`. Source for the base-item → class map (currency confirmed present).
  - `https://repoe-fork.github.io/poe2/uniques.json` — valid JSON, ~400 uniques keyed by numeric id;
    each has `id`, `name`, `item_class` (Amulet/Belt/…). No explicit base_type (poe.ninja supplies a
    unique's baseType when economy data exists).
  - `sync metadata` normalizes RePoE `item_class` (PascalCase) → filter `Class==` (spaced).
  - `category` (Currency/Breach/Vault Keys/…) comes from poe.ninja's overview `type`, not RePoE.
  - Initial map may be bootstrapped from `currency.json` (already has class/category/baseType for the
    358 curated items); `sync metadata` then refreshes/extends it from RePoE — same committed shape.

**Gem ≥18 + uniques (off-season):** implement leniently against the documented shapes, prove with
recorded fixtures, handle live-empty gracefully (skip, never fail); activate when poe.ninja has data.
