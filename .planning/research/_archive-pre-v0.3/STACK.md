# Stack Research

**Domain:** Rust CLI tool — JSON config compiler + external API sync
**Researched:** 2026-05-23
**Confidence:** HIGH (Rust crates) / MEDIUM (poe.ninja PoE2 API — undocumented, community-verified)

---

## Recommended Stack

### Core Technologies

| Technology | Version | Purpose | Why Recommended |
|------------|---------|---------|-----------------|
| clap | 4.6.1 | CLI arg parsing — subcommands, flags, help | The Rust ecosystem standard. Derive API eliminates boilerplate; compile-time validation; automatic help/version generation. No serious contender at this scope. |
| serde + serde_derive | 1.0.228 | Struct ↔ JSON (de)serialization | Universal Rust serialization layer; zero-cost abstractions; derive macros on every config struct. |
| serde_json | 1.0.149 | Read/write JSON files | Direct serde backend for JSON; `preserve_order` feature flag for controlling key ordering in prices.json. |
| ureq | 3.3.0 | HTTP client for poe.ninja sync | Blocking-only; no Tokio/async runtime needed for a CLI that makes sequential requests; tiny dependency footprint vs reqwest. |
| anyhow | 1.0.102 | Error handling | Application binary: callers display errors, not match on types. `context()`/`with_context()` adds callsite context as errors bubble. |
| insta | 1.47.2 | Snapshot testing of `.filter` output | Determinism guarantee: commit `.snap` files; CI fails on any regression in generated filter text. |

### Supporting Libraries

| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| indexmap | 2.x | Ordered map backing for prices.json | Needed when `serde_json`'s `preserve_order` feature is enabled — it uses `indexmap::IndexMap` internally; also useful for your own `PriceMap` type. |
| cargo-insta (CLI) | latest | Interactive snapshot review | Dev workflow: `cargo insta review` to accept new/changed snapshots; install once as a cargo subcommand, not a Cargo.toml dep. |

### Development Tools

| Tool | Purpose | Notes |
|------|---------|-------|
| `cargo test` | Unit + snapshot tests | Run with `INSTA_UPDATE=unseen` on first run to seed snapshots |
| `cargo insta review` | Accept / reject snapshot diffs | Install via `cargo install cargo-insta`; not a Cargo.toml dep |
| `rustfmt` | Code formatting | Standard; no configuration needed for this project size |
| `clippy` | Linting | `cargo clippy -- -D warnings` in CI |

---

## Installation (Cargo.toml)

```toml
[dependencies]
clap     = { version = "4", features = ["derive"] }
serde    = { version = "1", features = ["derive"] }
serde_json = { version = "1", features = ["preserve_order"] }
ureq     = { version = "3", features = ["json"] }
anyhow   = "1"
indexmap = { version = "2", features = ["serde"] }

[dev-dependencies]
insta = { version = "1", features = ["filters"] }
```

---

## Alternatives Considered

| Recommended | Alternative | When to Use Alternative |
|-------------|-------------|-------------------------|
| clap derive | clap builder API | Never for this project — derive is strictly less boilerplate with no tradeoff at this CLI size |
| clap derive | argh, lexopt, pico-args | Only when binary size is a hard constraint (e.g., embedded); clap's feature set (help, subcommands, enum flags) is worth its cost here |
| ureq (blocking) | reqwest (async) | If the tool ever needs concurrent fetches (e.g., parallel league sync) — but poe.ninja sequential fetching is fine and async adds tokio/hyper deps that bloat compile time |
| anyhow | thiserror | If `ferris_filter` were a library crate consumed by others; it is a binary, so anyhow is correct |
| insta | assert_eq! on strings | Fine for one or two cases; insta's diff rendering and `cargo insta review` workflow pays off quickly when the filter output is hundreds of lines |
| serde_json preserve_order | Vec of (String, Value) pairs | Manual ordering is error-prone; the `preserve_order` feature is the idiomatic path |

---

## What NOT to Use

| Avoid | Why | Use Instead |
|-------|-----|-------------|
| reqwest (async) for sync commands | Pulls in tokio, hyper, futures; compile time +10–30s; async ceremony for sequential API calls is pure overhead | ureq 3 blocking |
| tokio / async-std entirely | This CLI does no concurrent I/O; async runtime is dead weight | Synchronous stdlib + ureq |
| serde_json without `preserve_order` for prices.json | Default BTreeMap backend silently alpha-sorts keys; Exalt/Chaos/Divine ordering contract broken | `serde_json = { features = ["preserve_order"] }` |
| structopt | Superseded by clap 4 derive; archived | clap 4 with `features = ["derive"]` |
| failure / error-chain | Both deprecated; no active maintenance | anyhow |

---

## Stack Patterns

**Subcommand structure with clap derive:**
```rust
#[derive(Parser)]
#[command(name = "ferris-filter", version, about)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Generate(GenerateArgs),
    Sync(SyncArgs),
}

#[derive(Args)]
struct SyncArgs {
    #[arg(long, help = "Fetch Hardcore league data")]
    hc: bool,
    #[arg(long, help = "Fetch Standard (permanent) league data")]
    standard: bool,
}
```
`--standard` and `--hc` are both false by default. Both true → Standard Hardcore.

**JSON read/write with ordered prices.json:**
```rust
// prices.json is a Vec<PriceEntry> — Vec preserves insertion order inherently.
// No IndexMap needed at the top level; use Vec<PriceEntry> not Map<String, PriceEntry>.
// Nullable currency values:
#[derive(Serialize, Deserialize)]
struct PriceEntry {
    name: String,
    exalt: Option<f64>,   // null when untraded
    chaos: Option<f64>,
    div:   Option<f64>,
}
```
Using `Vec<PriceEntry>` guarantees first-three-entry ordering without any `preserve_order` complexity at the struct level. `preserve_order` is only needed if you use `serde_json::Value` maps or the config structures use object-valued JSON maps that must round-trip in input order.

**ureq JSON fetch:**
```rust
let body: serde_json::Value = ureq::get(&url)
    .call()?
    .body_mut()
    .read_json()?;
```
Enable `ureq = { features = ["json"] }`.

---

## poe.ninja PoE2 API — Research Findings

**Confidence: MEDIUM** — The PoE2 endpoints are **not officially documented** by poe.ninja. All findings come from community network-interception, wrapper libraries, and URL pattern analysis. A spike-and-verify pass is required before building `sync`.

### Base URL (PoE2)

The PoE2 economy API lives under a **separate path** from PoE1:

```
https://poe.ninja/api/data/currencyoverview?league=<LeagueName>&type=<Type>
https://poe.ninja/api/data/itemoverview?league=<LeagueName>&type=<Type>
```

The `/poe2/api/` prefix (`https://poe.ninja/poe2/api/economy/currencyexchange/overview?leagueName=…`) was observed in an early announcement but **returned 404** in direct verification attempts during this research. The `/api/data/` path is what community tools actually use. **Verify both during the spike.**

### League Name Parameter

The `league` query parameter accepts the **full league name as a string**, case-sensitive. Known working values from URL evidence and community wrappers:

| CLI flags | API `league=` value | Site URL slug |
|-----------|--------------------|----|
| (neither) | `"Fate of the Vaal"` | `vaal` |
| `--hc` | `"HC Fate of the Vaal"` | `vaalhc` |
| `--standard` | `"Standard"` | `standard` |
| `--standard --hc` | `"HC Standard"` (unverified — may be `"Hardcore"`) | unknown |

The site URL slug pattern (`/poe2/economy/vaal/`, `/poe2/economy/vaalhc/`, `/poe2/economy/standard/`) is confirmed from live page URLs. The **full league name** that maps to each slug must be validated in the spike by fetching a known endpoint.

The current league (as of 2026-05-23) is **Fate of the Vaal** (ending ~May 29 2026). The next league is "Runes of Aldur". League names change every 3–4 months; the tool must allow configuring or auto-detecting the current league name.

### Confirmed Category URLs (from live site URL discovery)

These page URLs confirm the categories exist and their slugs:

| Category | Site slug | API `type=` (estimated) |
|----------|-----------|------------------------|
| Currency | `currency` | `Currency` |
| Fragments | `fragments` | `Fragment` |
| Essences | `essences` | `Essence` |
| Soul Cores | `soul-cores` | `SoulCore` (unverified exact casing) |
| Runes | — (observed in community data) | `Rune` (unverified) |
| Omens | — | `Omen` |
| Unique Weapons | — | `UniqueWeapon` |
| Unique Armours | — | `UniqueArmour` |
| Unique Accessories | — | `UniqueAccessory` |
| Unique Flasks | — | `UniqueFlask` |
| Unique Jewels | — | `UniqueJewel` |
| Unique Charms | — | `UniqueCharm` (unverified) |
| Uncut Gems | — | `UniquGem` / `SkillGem` (unverified) |

**The `type=` string values for PoE2 are NOT officially documented.** They may differ from PoE1 type strings. Treat all `type=` values above as candidates to test during the spike.

For FerrisFilter's `sync`, the minimum required categories are:
- **Currency** (for Exalt/Chaos/Divine cross-rates — mandatory first)
- **UniqueWeapon, UniqueArmour, UniqueAccessory, UniqueFlask, UniqueJewel** (for unique tiering)
- **Omen, Essence, SoulCore, Rune** (for non-unique currency tiering)
- **Uncut Gems** (filtered to level ≥ 18 per PROJECT.md requirements)

### Response Schema

Community-documented schema for both endpoint types:

**currencyoverview response:**
```json
{
  "lines": [
    {
      "currencyTypeName": "Divine Orb",
      "chaosEquivalent": 234.56,
      "detailsId": "divine-orb",
      "pay": { "value": 234.56, "count": 1200, "listing_count": 800 },
      "receive": { "value": 0.00426, "count": 950, "listing_count": 600 }
    }
  ],
  "currencyDetails": [
    { "id": 1, "name": "Divine Orb", "icon": "https://..." }
  ]
}
```

**itemoverview response:**
```json
{
  "lines": [
    {
      "id": 12345,
      "name": "Headhunter",
      "icon": "https://...",
      "chaosValue": 4500.0,
      "exaltedValue": 19.2,
      "divineValue": 19.2,
      "count": 45,
      "detailsId": "headhunter",
      "listingCount": 38,
      "sparkline": { "data": [4200, 4350, 4500] }
    }
  ]
}
```

Key mapping to `prices.json`:
- Currency lines → `chaosEquivalent` field
- Item lines → `chaosValue`, `exaltedValue`, `divineValue` fields (all may be absent/null when item has no listings)

### Authentication and Rate Limits

- **No authentication required** — all endpoints are public, no API key.
- **Rate limits:** Not officially documented. Community practice is to add a small delay between requests (100–500ms). poe.ninja has historically not rate-limited well-behaved scrapers. Implement a configurable inter-request delay; default 200ms.
- **Data freshness:** Prices update approximately every few minutes from the Currency Exchange. No stale-data guarantee.

### Spike Tasks Before Building `sync`

The following must be validated before the `sync` command is implemented:

1. Confirm the correct base URL: `/api/data/` vs `/poe2/api/data/` vs other
2. Confirm the exact `league=` string for current trade league ("Fate of the Vaal" or different)
3. Enumerate all available PoE2 `type=` values by fetching known-good categories and observing 404 vs 200
4. Confirm `type=` for Soul Cores, Runes, Uncut Gems (PoE2-specific; no PoE1 equivalents)
5. Confirm `divineValue` and `exaltedValue` fields are present in PoE2 item responses
6. Check if Standard league uses `league=Standard` or a PoE2-specific variant

---

## Version Compatibility

| Package | Compatible With | Notes |
|---------|-----------------|-------|
| clap 4.6.x | Rust 1.74+ (MSRV) | Stable; semver-compatible within 4.x |
| serde 1.0.228 + serde_json 1.0.149 | Any recent Rust | Always upgrade together to avoid derive mismatch |
| ureq 3.3.x | Rust stable | 3.x is a breaking change from 2.x; use `ureq = "3"` in Cargo.toml |
| anyhow 1.0.102 | Rust 1.65+ for backtrace support | Backtrace captured automatically on panic if RUST_BACKTRACE=1 |
| insta 1.47.x | Rust stable | dev-dep only; not in release binary |

---

## Sources

- [docs.rs/clap/latest](https://docs.rs/clap/latest/clap/index.html) — version 4.6.1, derive API (HIGH confidence)
- [docs.rs/ureq/latest](https://docs.rs/crate/ureq/latest) — version 3.3.0, blocking + json feature (HIGH confidence)
- [docs.rs/reqwest/latest](https://docs.rs/crate/reqwest/latest) — version 0.13.3, rejected for sync CLI (HIGH confidence)
- [docs.rs/anyhow/latest](https://docs.rs/anyhow/latest/anyhow/) — version 1.0.102 (HIGH confidence)
- [docs.rs/insta/latest](https://docs.rs/insta/latest/insta/) — version 1.47.2 (HIGH confidence)
- [serde-rs/json #54](https://github.com/serde-rs/json/issues/54) — preserve_order feature flag (HIGH confidence)
- [5k-mirrors/misc-poe-tools poe-ninja-api.md](https://github.com/5k-mirrors/misc-poe-tools/blob/master/doc/poe-ninja-api.md) — PoE1 base endpoint schema; PoE2 adapted (MEDIUM confidence)
- [poe.ninja/posts/poe2-unique-items](https://poe.ninja/posts/poe2-unique-items) — 8 unique categories for PoE2 confirmed (MEDIUM confidence)
- [Davenads/poeninjaAPI-2025](https://github.com/Davenads/poeninjaAPI-2025) — PoE2 endpoint discovery via network interception (MEDIUM confidence)
- Live poe.ninja URL patterns (`/poe2/economy/vaal/`, `/poe2/economy/vaalhc/`, `/poe2/economy/standard/soul-cores`) — category and league slug evidence (MEDIUM confidence)
- WebSearch: multiple community wrappers confirming `/api/data/currencyoverview` and `/api/data/itemoverview` patterns (MEDIUM confidence)

---

*Stack research for: FerrisFilter — Rust CLI, PoE2 filter compiler + poe.ninja price sync*
*Researched: 2026-05-23*
