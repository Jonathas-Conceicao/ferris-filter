# Project Research Summary

**Project:** FerrisFilter
**Domain:** Rust CLI — config-to-PoE2-filter compiler with poe.ninja price sync
**Researched:** 2026-05-23
**Confidence:** HIGH (stack, architecture, filter semantics) / MEDIUM (poe.ninja PoE2 API)

---

## Executive Summary

FerrisFilter is a deterministic filter compiler: a human edits `config.json` + `tiers.json`, runs `sync` to pull live prices, and `generate` produces the exact `universal.filter` the game consumes. The executable spec already exists in this repo — `filter-model.json` and `universal.filter` define every block ordering, style, and progression rule the tool must reproduce. Research confirmed the Rust crate selection, the module decomposition, the canonical emit order, and the full set of correctness invariants. The architecture separates all network I/O into `sync` and keeps `generate` a pure function of its input files — the core design decision that makes determinism and testability achievable.

The most consequential open question is the poe.ninja PoE2 API surface. **The four research streams disagree and this is UNRESOLVED.** STACK.md documents `/api/data/currencyoverview` with `chaosValue`/`divineValue` fields; PITFALLS.md/FEATURES.md document `/poe2/api/economy/` with `primaryValue`/`secondaryValue` fields. Both come from community reverse-engineering, not official docs. Resolve via a focused API spike before `sync` is built.

The second class of risk is correctness in the generated filter: unique-by-base collisions, non-deterministic block ordering, hide-before-show shadowing of uniques, and null/zero reference rates silently corrupting tier math. All have clear prevention strategies, but must be addressed from the first working implementation.

---

## Recommended Stack

```toml
[dependencies]
clap       = { version = "4", features = ["derive"] }
serde      = { version = "1", features = ["derive"] }
serde_json = "1"
ureq       = { version = "3", features = ["json"] }   # blocking HTTP — no async runtime
anyhow     = "1"
indexmap   = { version = "2", features = ["serde"] }  # ordered styles map

[dev-dependencies]
insta = { version = "1", features = ["filters"] }     # snapshot-test deterministic output
```

- `clap 4` derive — subcommands (`generate`/`sync`) + flags (`--standard`/`--hc`/`--output`), free `--help`.
- `serde`/`serde_json` — all JSON I/O. Use `Vec<(String, ItemPrice)>` for `prices.json` so insertion order (Exalt/Chaos/Divine first) is guaranteed.
- `ureq 3` blocking — right for a sequential CLI making ~8–12 calls; avoids tokio/reqwest compile cost.
- `anyhow` — binary error handling with `.context()`.
- `insta` — snapshot the generated filter; guarantees determinism.

## Expected Features

- **v1 table stakes:** `generate` (deterministic compile + `--output`), config/tiers validation with actionable errors, block-level source comments, graceful handling of absent `prices.json`; `sync` (poe.ninja fetch, `--hc`/`--standard`, Exalt/Chaos/Divine as entries 0/1/2, null-safe values, uncut gems ≥ level 18); `--help`, exit codes.
- **v1.x:** `--dry-run`, `--diff`, `-v/--verbose`, `prices.json` staleness warning.
- **v2+:** multiple build profiles, `--validate-only`, parallel category fetch, poe2scout fallback.
- **Anti-features (never):** GUI/web (FilterBlade covers it), auto-install (install.bat covers it), blanket `Rarity == Unique` styling, NeverSink-style strictness levels, historical price trends.

## Architecture Approach

Two-pipeline CLI with a strict I/O boundary: `generate` is pure (files in, one file out, zero network); `sync` is impure (all HTTP, writes `prices.json`). Module dependency graph:

1. `config/` — `FilterConfig` serde model (mirrors `filter-model.json`; `IndexMap` for ordered styles)
2. `tiers/` — `TierCutoffs` model + `assign_tier(chaos_value, &cutoffs)`
3. `prices/` + `prices/poe_ninja.rs` — `PriceList = Vec<(String, ItemPrice)>`; all HTTP isolated here
4. `pricing/` — `CrossRates::from_price_list` (Err on null/zero), `to_chaos`, tier assignment; pure
5. `emit/` — `Block` IR, `render_block`, `style_to_actions`, one `emit_*` fn per section returning `Vec<Block>`
6. `generate.rs` — pure orchestrator: load → cross-rates → tiers → emit → write
7. `sync.rs` — network orchestrator: parse flags → resolve league → fetch → write `prices.json`

**Canonical emit order (hard-coded, not call-order dependent):** Recycle → gear bands → gear `Hide` fallthrough → flask/charm bands → flask/charm `Hide` → curated currency categories (S→F) → price-tiered unique highlights.

## Critical Pitfalls

1. **Unique-by-base collision** — poe.ninja prices by unique name; filters match `BaseType`. Group response by `baseType`, emit one block at the highest tier among shared-base uniques; never `Hide` a base that any high-value unique shares.
2. **poe.ninja PoE2 API UNRESOLVED** — base URL and field names disagree across reports. Spike before building `sync`.
3. **Null/zero reference rates** — silently corrupt all tier math (no panic). `CrossRates::from_price_list` must `Err`; `generate` aborts loudly.
4. **Non-deterministic output** — Rust `HashMap` randomizes iteration. Use `BTreeMap` + sort names; byte-identical determinism test from day one.
5. **First-match-wins `Hide` swallowing uniques** — every gear/flask `Hide` must carry `Rarity <= Rare`; price-tiered unique blocks emit before any gear `Hide`. Add a post-gen validator.
6. **`GemLevel` unverified in PoE2** — not in the syntax reference. Use `ItemLevel` proxy until verified in-game; mark as an acceptance criterion.

---

## Implications for Roadmap

Suggested structure (granularity = coarse): a blocking spike + ~5 phases. Phases 1–3 have zero external dependencies and can proceed immediately; Phase 4 (`sync`) depends on the spike.

- **Spike — poe.ninja API:** resolve base URL, league resolution (GGG `GET /league?realm=poe2`?), exact `type`/`overviewName` per category, response field names, `baseType` field, expired-league behavior, rate-limit/429 behavior. **Blocking for `sync`.**
- **Phase: Emit Foundation** — config/tiers/prices serde models + `Block` IR + render + insta snapshots. No external deps.
- **Phase: Static Generate** — wire all section emitters to reproduce `universal.filter` byte-for-byte from `config.json` (+ embedded currency lists). insta snapshot = regression baseline.
- **Phase: Pricing Layer** — `tiers.json`, cross-rate math, unique-base-collision resolution, price-tiered emitter; fixture `prices.json` only.
- **Phase: Sync** — poe.ninja HTTP client, league flags + resolution, schema canary, rate limiting, `fetched_at`/`league` metadata, uncut-gem ≥18 filter. Depends on the spike.
- **Phase: Polish** — `--dry-run`, `--diff`, `--verbose`, staleness warning.

## Confidence Assessment

| Area | Confidence | Notes |
|------|------------|-------|
| Stack | HIGH | Crates stable, official docs; ureq-vs-reqwest tradeoff clear-cut. |
| Features | HIGH | Scope defined in PROJECT.md; research confirmed + added UX detail. |
| Architecture | HIGH | Derived from `filter-model.json` + `universal.filter` (executable spec). |
| Pitfalls | MEDIUM | Filter-semantics pitfalls HIGH; poe.ninja API pitfalls MEDIUM (reverse-engineered, conflicting). |

**Overall:** MEDIUM-HIGH. Clear, low-risk path except the single unresolved external dependency.

### Gaps to Address
- **UNRESOLVED: poe.ninja PoE2 API surface** — resolve via spike before writing `sync`.
- **`GemLevel` in PoE2 unverified** — use `ItemLevel` proxy; verify in-game.
- **`BaseEnergyShield`/`BaseArmour`/`BaseEvasion`** — work in practice; confirm exact semantics (CLAUDE.md open item). Low priority.

## Sources

- **Primary (HIGH):** repo `filter-model.json`, `universal.filter`, `CLAUDE.md`, `.planning/PROJECT.md`; docs.rs for clap/ureq/anyhow/insta/serde; NeverSink PoE2 syntax reference; FilterBlade FAQ (unique-by-base); GGG developer docs (`realm=poe2` league endpoint).
- **Secondary (MEDIUM):** Davenads/poeninjaAPI-2025 (PoE2 endpoint via interception — conflicts with STACK.md); 5k-mirrors poe-ninja-api.md (PoE1 schema); poe.ninja PoE2 economy URL patterns.
- **Tertiary (LOW):** community rate-limit estimate (~12 req/5 min).

---
*Research completed: 2026-05-23. Ready for roadmap — pending the poe.ninja spike for `sync` implementation.*
