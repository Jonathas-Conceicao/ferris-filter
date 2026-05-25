# Phase 4: Sync - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-05-25
**Phase:** 4-Sync
**Areas discussed:** prices.json refresh strategy, HTTP client, the API spike, failure-safety & league resolution

---

## prices.json refresh strategy

| Option | Description | Selected |
|--------|-------------|----------|
| Rebuild via overview→category/class map | Fetch poe.ninja overviews, categorize via a committed mapping; discovers new items + uniques | ✓ |
| Merge prices onto existing structure | Only update prices for known items; never discovers uniques | |
| category = poe.ninja overview name | No map; loses curated category names | |

**User's choice:** Rebuild via overview→category/class map

### Mapping maintenance
| Option | Description | Selected |
|--------|-------------|----------|
| Static per-overview map in repo | Committed map keyed by overview → {category, class} | ✓ (variant) |
| Infer/heuristic mapping | Derive programmatically, no table | |

**User's choice:** "static map in repo, but not hand-maintained, we can fetch any missing info from
poe2db https://poe2db.tw/sp/pob" — committed map, populated from poe2db.

### poe2db timing → became a sync subcommand
| Option | Description | Selected |
|--------|-------------|----------|
| Offline map-build step → committed map | Separate step fetches poe2db once; sync prices hits only poe.ninja | ✓ (as a subcommand) |
| Live during sync for missing items | sync hits poe2db at runtime | |

**User's choice:** "lets add a subcommand to sync with prices and metadata, where prices is the
default and syncs poe ninja economy, and metadata fetches the poe2db info. Map is then committed to
the repo." → `sync prices` (default) + `sync metadata`.

### Spike scope
| Option | Description | Selected |
|--------|-------------|----------|
| Yes — spike both surfaces | Live-probe + document poe.ninja AND poe2db before any HTTP code | ✓ |
| poe.ninja only; poe2db later | Defer poe2db | |

**User's choice:** Yes — spike both surfaces

---

## HTTP client

| Option | Description | Selected |
|--------|-------------|----------|
| ureq (blocking, minimal) | No async runtime | |
| reqwest + tokio (async) | Async, heavier | (direction) |

**User's choice:** "check which dependencies work better in case we decide to compile to WASM in a
future release. But in general we prioritize async." → async; HTTP stack weighted by WASM-compatibility;
research selects the concrete crate (reqwest a candidate via its wasm backend).

---

## The API spike

| Option | Description | Selected |
|--------|-------------|----------|
| Live probe → findings doc that gates code | Hit the real APIs, capture shapes, document; no HTTP code until doc exists | ✓ |
| Docs/research only | Rely on documented shapes | |

**User's choice:** Live probe → findings doc that gates code

---

## Failure-safety

| Option | Description | Selected |
|--------|-------------|----------|
| Atomic: temp file + validate + rename | Validate non-empty/parseable, then atomic rename | ✓ |
| In-place write with backup | Backup + restore on error | |

**User's choice:** Atomic: temp file + validate + rename

---

## League resolution

| Option | Description | Selected |
|--------|-------------|----------|
| Live via poe.ninja league endpoint | Resolve current league name dynamically per flags; record in prices.json | ✓ |
| Configured / hardcoded league names | Config string, goes stale each league | |

**User's choice:** Live via poe.ninja league endpoint

---

## Claude's Discretion
- Exact async crate + TLS backend (wasm-aware); rate-limiting/User-Agent etiquette; clap modeling of
  the default `prices` subcommand; mapping artifact file name/shape; overview enumeration; retry/timeout.

## Deferred Ideas
- WASM compilation (future release; influences dep choice only).
- SYNX-01/02, GENX-04 (v2).
- Live per-item poe2db lookups during sync prices (rejected for the offline metadata map-build).
