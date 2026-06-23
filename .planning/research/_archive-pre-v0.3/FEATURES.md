# Feature Research

**Domain:** CLI loot-filter compiler + price-sync tool for Path of Exile 2
**Researched:** 2026-05-23
**Confidence:** HIGH (core features), MEDIUM (poe.ninja API specifics — undocumented PoE2 endpoints)

---

## Feature Landscape

### Table Stakes (Users Expect These)

Features any filter generator must have to feel complete. Missing one makes the tool
feel unfinished or unusable.

| Feature | Why Expected | Complexity | Notes |
|---------|--------------|------------|-------|
| `generate` emits a valid `.filter` file | Core value proposition — nothing else works without it | MEDIUM | Must match PoE2 grammar exactly; same grammar as PoE1 but different valid class/base-type names |
| Deterministic output | Same inputs → identical output; required for version control diffs to be meaningful | LOW | Sort all collections before rendering; never use HashMap iteration order |
| `--output <path>` flag on `generate` | Users need to redirect output or write to the game folder | LOW | Default to `universal.filter` in cwd; accept absolute or relative paths |
| `--help` on every subcommand | Standard CLI contract; users expect it before reading docs | LOW | Clap's derive API provides this for free with good doc strings |
| Actionable error messages | Config typos or bad values surface immediately with context | MEDIUM | Include field path + actual value + what was expected; use `thiserror` for domain errors, `anyhow` for app errors |
| Config schema validation on `generate` | Catches malformed `config.json`/`tiers.json` before generating 400-line filter | MEDIUM | Validate required fields, enum values (style names, class names), numeric ranges at startup |
| `sync` writes `prices.json` | The fetch half of the generate/sync loop | MEDIUM | File must be human-readable JSON; on failure leave prior file intact |
| League selection flags (`--hc`, `--standard`) on `sync` | Users play different leagues; the tool must target the right economy data | LOW | Default (no flags) = current softcore trade league; `--standard` = Standard; `--hc` = current HC trade; `--standard --hc` = Standard HC |
| Exalt/Chaos/Divine always first in `prices.json` | Required by `generate`'s cross-rate logic | LOW | Hardcode these three at indices 0-2 with explicit fetch; null-safe (untraded currency gets `null` chaos value) |
| Null-safe price handling | Some items are untraded or newly introduced; the file must not crash `generate` | LOW | `{exalt: null, chaos: null, div: null}` is valid; `generate` treats null-priced items as tier F or omits them from rules |
| Uncut gems only at level >= 18 | Project requirement; lower-level uncut gems are low-value and create rule noise | LOW | Filter gem rows in `sync` by level field before writing; levels 1-17 are dropped |
| Exit codes | Scripts and CI need 0 = success, nonzero = failure | LOW | Standard: 0 success, 1 logic/config error, 2 network/IO error |

### Differentiators (Competitive Advantage)

Features that are not universal across comparable tools but add real value for this project.

| Feature | Value Proposition | Complexity | Notes |
|---------|-------------------|------------|-------|
| Dry-run / diff on `generate` | See exactly what will change before overwriting the filter; safe to iterate quickly | MEDIUM | `--dry-run` prints the filter to stdout; `--diff` diffs against the existing output file using a line-level diff (similar to `diff` output); no file is written in either mode |
| Block-level source comments in output | Every block tagged with its model path (`[areaLevelBands "6-25".weapons -> styles.unchanged]`) | LOW | Already in the current hand-built filter; `generate` must reproduce this tagging; invaluable for debugging mismatches |
| Tiering from `tiers.json` (configurable cutoffs) | User can recalibrate what counts as "A-tier" without touching code | LOW | Seven thresholds in chaos: F <0.1 · E 0.1–<0.5 · D 0.5–<2 · C 2–<5 · B 5c–<1div · A 1div–<5div · S ≥5div; `generate` reads this at compile time, not hardcoded |
| Chaos cross-rate conversion at generate time | All three reference currencies (Exalt/Chaos/Divine) in `prices.json` let `generate` normalize any price to chaos for threshold comparisons | MEDIUM | Cross-rates derived from the first three `prices.json` entries; if Chaos entry has `chaos: 1.0` and Divine entry has `chaos: 250.0`, then a 300c item that poe.ninja reports at 1.2 div is treated as 300c |
| Verbose mode (`-v` / `--verbose`) | Shows which rules were generated from which model keys; useful when debugging why an item isn't highlighted | LOW | Print to stderr; one level is sufficient for v1 |
| Caching of `prices.json` with a staleness check | Avoid hammering poe.ninja on every `generate` run; warn if prices are older than N hours | LOW | Embed a `fetched_at` ISO-8601 timestamp in `prices.json`; `sync` always overwrites; `generate --warn-stale 24h` emits a warning but proceeds |
| Price-tiered uniques (only tracked ones get rules) | Avoids bloating the filter with rules for low-value uniques; untracked fall through | MEDIUM | `generate` emits one `Show` block per unique in `prices.json` whose tier is >= configured minimum; untracked uniques use game default (CLAUDE.md design philosophy) |
| Separation of human-edited config from machine-generated prices | `config.json` (human) + `tiers.json` (human) + `prices.json` (machine) keeps merge conflicts minimal | LOW | `prices.json` should be in `.gitignore` or at least documented as machine-generated; `config.json` and `tiers.json` are version-controlled |

### Anti-Features (Commonly Requested, Often Problematic)

Features to explicitly not build in v1 (and potentially ever).

| Feature | Why Requested | Why Problematic | Alternative |
|---------|---------------|-----------------|-------------|
| GUI / web interface | Lowers barrier for non-technical users; FilterBlade shows demand | Enormous scope; defeats the "Rust CLI" design goal; any web layer adds deployment, auth, and maintenance | FilterBlade already exists for GUI workflows; FerrisFilter targets authors who want version-controlled, reproducible configs |
| Auto-install into game folder | Convenience; avoids manual copy step | Couples the tool to a specific OS path; breaks on OneDrive/cloud sync variants; `install.bat` already covers it for this repo | Keep `install.bat`; document the output path; user copies when ready |
| Rules for gems, jewels, jewellery, quest items, waystones, trials, unidentified items | Completeness instinct | These fall through to game default by design (CLAUDE.md philosophy); adding rules creates false confidence and bloat without value; no PoE2 mechanic requires filtering them | Document the fall-through explicitly in generated filter header comment (already done in `universal.filter`) |
| Blanket unique styling | "Highlight all uniques" is common ask | Would repaint uniques the game already shows natively; hides the useful signal that a specific base is economically tiered; NeverSink notes the engine cannot distinguish between two uniques on the same base | Only price-tracked uniques with tier >= configured minimum get rules; the rest fall through |
| Strictness levels (NeverSink-style 0–6) | Flexible aggression control is popular | Adds a hidden dimension to the config that interacts with every rule; hard to test; the area-level progression already handles "show more while leveling, show less in endgame" | The `areaLevelBands` model already encodes the equivalent of strictness progression without a separate dial |
| Historical price trends / sparklines | "Is this item trending up?" | poe.ninja PoE2 API does not currently return sparkline/trend data (confirmed: undocumented endpoint, no historical data); adds complexity with no available data source | Track only current snapshot; user checks poe.ninja directly for trends |
| Interactive / REPL mode | "Just run and ask me questions" | Complicates piping and automation; hard to test | `--help` + clear error messages with fix hints cover the same UX need |
| Auto-update of `config.json` from new game patches | Sounds convenient | Config is author-intent; auto-mutation violates the principle that generate is deterministic from known inputs | Changelog docs + `sync` handles price updates; class/base-type changes require deliberate author edit |

---

## Feature Dependencies

```
prices.json (written by sync)
    └──required by──> generate: currency cross-rate conversion
    └──required by──> generate: price-tiered unique rules

tiers.json (human-edited, thresholds in chaos)
    └──required by──> generate: tier assignment for all priced items

config.json (human-edited, policy model)
    └──required by──> generate: ALL rule generation

generate (core compilation)
    └──enables──> --dry-run (stdout instead of file write)
    └──enables──> --diff (compare against existing output)
    └──enables──> --warn-stale (check prices.json age before compiling)

sync (price fetch)
    └──enables──> prices.json freshness
    └──depends on──> poe.ninja PoE2 API (external, undocumented, may change)

block-level source comments
    └──enhances──> generate output (readability, debugging)

tiers.json configurable cutoffs
    └──enhances──> generate: tiering logic (without it, cutoffs are hardcoded)

--verbose flag
    └──enhances──> generate and sync (both benefit independently)
```

### Dependency Notes

- **`generate` requires `prices.json`:** Currency cross-rates and unique tiering are computed at generate time from the prices file. If `prices.json` is absent, `generate` should warn and skip price-tiered rules (fall through to game default), not crash — makes first-run experience workable without running `sync` first.
- **`generate` requires `tiers.json`:** Without thresholds, tier assignment has no reference points. This file should have a checked-in default so `generate` works out of the box.
- **`--dry-run` and `--diff` enhance `generate`:** Both are output-mode flags that share the same compilation path; they just change where the result goes. Implement after core `generate` works.
- **`sync` depends on undocumented poe.ninja PoE2 API:** The `/poe2/api/economy/currencyexchange/overview` endpoint was discovered via network interception; it is not officially documented and may change without notice. The `sync` implementation must handle HTTP errors gracefully and report the failure without corrupting the existing `prices.json`.
- **Uncut gem level filter is a `sync`-side concern:** The `>= 18` threshold is applied when writing `prices.json`, not at generate time. This keeps generate's logic simple (it just reads the file as given).

---

## MVP Definition

### Launch With (v1)

Minimum to make the tool usable end-to-end.

- [ ] `generate` subcommand — reads `config.json` + `tiers.json` + `prices.json` and emits `universal.filter`; reproduces all current hand-built logic
- [ ] `generate --output <path>` — redirect output file
- [ ] Deterministic output — same inputs, same bytes out
- [ ] Config validation at startup — actionable errors for bad `config.json` / `tiers.json`
- [ ] Block-level source comments in generated filter — every block tagged with model path
- [ ] `generate` graceful handling of absent `prices.json` — warn, skip price-tiered rules, proceed
- [ ] `sync` subcommand — fetches poe.ninja PoE2 economy data, writes `prices.json`
- [ ] `sync` league flags — `--hc`, `--standard`
- [ ] `sync` Exalt/Chaos/Divine first in `prices.json`, null-safe for untraded items
- [ ] `sync` uncut gem level filter — only gems >= level 18 written to `prices.json`
- [ ] `--help` on all subcommands with useful descriptions
- [ ] Exit codes: 0 success, nonzero failure
- [ ] Actionable error messages (field path + actual + expected)

### Add After Validation (v1.x)

Add once the core generate/sync loop is proven correct.

- [ ] `generate --dry-run` — when the output looks right and you want a safety net before overwriting
- [ ] `generate --diff` — when you want to see exactly what changed after a sync
- [ ] `generate --warn-stale <duration>` — once the habit of running sync periodically is established
- [ ] `-v / --verbose` flag — when debugging why a specific item isn't highlighted
- [ ] `prices.json` `fetched_at` timestamp — low effort, pairs with `--warn-stale`

### Future Consideration (v2+)

Defer until the tool is in daily use and real friction surfaces.

- [ ] Multiple build profiles (e.g., `--config builds/witch.json`) — when the single build assumption breaks
- [ ] `generate --validate-only` without writing output — when CI integration is desired
- [ ] Parallel category fetches in `sync` — when poe.ninja adds more PoE2 categories and serial fetch is noticeably slow
- [ ] Support for poe2scout as an alternate price source — if poe.ninja PoE2 coverage proves insufficient for uniques

---

## Feature Prioritization Matrix

| Feature | User Value | Implementation Cost | Priority |
|---------|------------|---------------------|----------|
| `generate` core compilation | HIGH | MEDIUM | P1 |
| Deterministic output | HIGH | LOW | P1 |
| Config validation + error messages | HIGH | MEDIUM | P1 |
| Block-level source comments | HIGH | LOW | P1 |
| `sync` core fetch + `prices.json` write | HIGH | MEDIUM | P1 |
| `sync` league flags | HIGH | LOW | P1 |
| Null-safe price handling + absent prices.json graceful | HIGH | LOW | P1 |
| Exalt/Chaos/Divine first 3 entries | HIGH | LOW | P1 |
| Uncut gem level >= 18 filter in sync | MEDIUM | LOW | P1 |
| `--output` path control | MEDIUM | LOW | P1 |
| `--help` everywhere | MEDIUM | LOW | P1 |
| Exit codes | MEDIUM | LOW | P1 |
| `generate --dry-run` | MEDIUM | LOW | P2 |
| `generate --diff` | MEDIUM | LOW | P2 |
| `--verbose` flag | MEDIUM | LOW | P2 |
| `prices.json` staleness timestamp + `--warn-stale` | LOW | LOW | P2 |
| Multiple build profiles | LOW | MEDIUM | P3 |
| `generate --validate-only` | LOW | LOW | P3 |

**Priority key:**
- P1: Must have for launch
- P2: Should have, add when possible
- P3: Nice to have, future consideration

---

## Competitor Feature Analysis

| Feature | FilterBlade (NeverSink, web) | poe-filter (flapanther33781, Python, unmaintained) | PoE Dynamic Loot Filter (Apollys, Python) | FerrisFilter (our approach) |
|---------|------------------------------|---------------------------------------------------|-------------------------------------------|-----------------------------|
| Price-driven tiering | Yes — poe.ninja EXCHANGE data, economy-only updates | Yes — poe.ninja fetch per run | No — manual tier assignment | Yes — `sync` + `generate`; chaos cross-rates |
| Config format | Web UI + saved profile | Python config module + GUI sliders | Profile.changes text file | JSON (`config.json` + `tiers.json`) |
| CLI / automation | No — web only; Patreon auto-updater service | No — GUI-driven | Yes — `python backend_cli.py <cmd> <params>` | Yes — native CLI, two subcommands |
| Deterministic output | Unknown (web service) | Unknown | Not stated | Yes — explicit design goal |
| Dry-run / diff | No | No | No | v1.x — `--dry-run` + `--diff` |
| Source comments in output | No (machine output) | No | No | Yes — block-level model-path tags |
| Unique handling | Tiered by base type; cannot distinguish variants on same base | Tier by value breakpoint | Manual | Price-tracked uniques only; untracked fall through |
| Uncut gem level filter | Implicit (gem highlighting by level band) | Not documented | Not applicable | Explicit >= 18 threshold in `sync` |
| Area-level progression | Yes — campaign vs endgame mode (area level 65 switch) | Strictness slider (not area-level aware) | Not applicable | Full band ladder (1-5/6-25/26-45/46-65/66+) |
| Strictness levels | 7 levels (soft to uber-plus-strict) | 10-tier system | Per-item tier visibility | Not a concept — area-level bands encode it |
| League selection | Web UI league picker | League type in config | Not applicable | `--standard`, `--hc` flags |
| Version-controlled config | No (web account) | Partially (Python files) | Yes (Profile.changes) | Yes — `config.json` + `tiers.json` checked in; `prices.json` machine-generated |

---

## Sources

- FilterBlade / NeverSink Filter for PoE2: https://github.com/NeverSinkDev/NeverSink-Filter-for-PoE2 — economy tiering, strictness system, unique handling
- FilterBlade deepwiki (customization features): https://deepwiki.com/NeverSinkDev/NeverSink-Filter-for-PoE2/5.2-customization-with-filterblade
- poe.ninja API (PoE1 documented): https://github.com/ayberkgezer/poe.ninja-API-Document
- poe.ninja PoE2 API (undocumented, discovered via network interception): https://github.com/Davenads/poeninjaAPI-2025 — PoE2 endpoint `/poe2/api/economy/currencyexchange/overview`, league names (Rise of the Abyssal, Dawn of the Hunt, Standard)
- poe.ninja PoE2 unique categories announcement: https://poe.ninja/posts/poe2-unique-items — 8 unique categories tracked
- NinjaPricer PoE2 categories (15 currency, 7 unique poe2scout): https://github.com/POEFixer/NinjaPricer
- xan_filter pricetool_ninja.py (PoE1 API type names): https://github.com/xanthics/xan_filter/blob/master/pricetool_ninja.py
- poe-filter Python tool (price fetch pattern): https://github.com/flapanther33781/poe-filter
- PoE Dynamic Loot Filter (CLI pattern + exit codes): https://github.com/Apollys/PoEDynamicLootFilter
- CLI design best practices (2025-2026): https://www.caduh.com/blog/make-your-cli-a-joy-to-use
- Rust/Clap CLI error handling guide (January 2026): https://oneuptime.com/blog/post/2026-01-07-rust-cli-clap-error-handling/view
- Uncut gem level information (character level 78 for level 18 gems): https://mobalytics.gg/poe-2/guides/high-level-gems
- poe2db.tw — authoritative PoE2 base type / class reference (per CLAUDE.md)
- `universal.filter` and `filter-model.json` in this repo — executable spec for what `generate` must reproduce

---

*Feature research for: FerrisFilter — Rust CLI loot-filter compiler + price-sync for PoE2*
*Researched: 2026-05-23*
