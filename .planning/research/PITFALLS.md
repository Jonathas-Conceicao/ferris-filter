# Pitfalls Research

**Domain:** Rust CLI — config-to-PoE2-filter compiler with poe.ninja price sync
**Researched:** 2026-05-23
**Confidence:** MEDIUM (poe.ninja PoE2 API is undocumented; behavior inferred from community reverse-engineering and wrapper libraries. Filter grammar confirmed via CLAUDE.md + NeverSink research. Unique-matching limitation confirmed by multiple independent sources.)

---

## Critical Pitfalls

### Pitfall 1: The Unique-by-Base Matching Problem (DESIGN RISK — flag prominently)

**What goes wrong:**
poe.ninja prices a unique item by **name** (e.g., "Temporalis", "Cloak of Flame"). The PoE2 filter system can only match a unique by **BaseType** (the shared base, e.g., "Silk Robe") plus `Rarity == Unique`. A cheap unique and an expensive unique that share the same base type **cannot be distinguished by any filter condition**. Emitting a highlight rule for the expensive one will equally highlight the cheap one — and hiding the cheap one will hide the expensive one too.

Concrete example: if "Temporalis" (S-tier) and "Cloak of Flame" (D-tier) both drop from a "Silk Robe" base, a block like:

```
Show # Unique S-tier: Temporalis
  Rarity == Unique
  BaseType == "Silk Robe"
  PlayEffect Pink
```

...will play the pink beam for every unique Silk Robe regardless of which unique it is.

**Why it happens:**
Developers copy the poe.ninja `name` field directly into a `BaseType ==` match without verifying that no other uniques share the base. The filter emitter has no awareness of the base-name-to-unique-name cardinality.

**How to avoid:**
1. During `sync`, for every unique returned by poe.ninja, look up its base type from the API response (`baseType` field in the itemoverview schema). Group by base type.
2. When multiple uniques share a base type, apply the **highest tier** of any member of that group to the base. Document in a comment which names informed the tier.
3. Consider emitting a comment in the generated filter like `# WARNING: shared base — also matches [other names]` for any block where ≥2 uniques share the base.
4. Never emit a `Hide` rule for a base type that has any high-value unique sharing it.
5. At the extreme, treat any base that has both a high-value and a low-value unique as "unknown" and default to Show with minimal styling (treat as tier E or D, no hide).

**Warning signs:**
- poe.ninja response for `UniqueWeapon`, `UniqueArmour`, `UniqueAccessory` contains items where `baseType` appears more than once across entries.
- Unit tests that assert block count will pass even when base-type collisions exist — they won't catch the semantic error.

**Phase to address:**
Phase covering `sync` implementation and the `generate` unique-tiering logic. Must be addressed before any unique highlight blocks are emitted. This is the single most design-critical constraint.

---

### Pitfall 2: poe.ninja League Name Is Not Discoverable from poe.ninja Itself

**What goes wrong:**
poe.ninja's PoE2 economy endpoints require an exact, case-sensitive `leagueName` parameter (e.g., `"Rise of the Abyssal"`, `"Dawn of the Hunt"`, `"Standard"`). poe.ninja provides no API endpoint to enumerate active leagues. The league name changes every 3–4 months at league launch and the exact display name cannot be guessed from the slug or a date.

If FerrisFilter hardcodes the league name, it silently fetches stale or empty data after the next league launch without any error — poe.ninja may return an empty `lines` array for an expired league name rather than a 404.

**Why it happens:**
Developers know the current league name when writing the code and hardcode it as a constant. The problem surfaces only at the next league transition, weeks or months later.

**How to avoid:**
1. Resolve the current challenge league name from the **GGG official API** before hitting poe.ninja. The endpoint `GET https://www.pathofexile.com/api/league?realm=poe2` (or the developer docs endpoint `GET /league` with `realm=poe2`) returns an array of active leagues. The first non-Standard, non-Hardcore result is the current challenge league. (MEDIUM confidence — verified from developer docs that `realm=poe2` is a valid parameter; exact response shape needs live verification.)
2. Cache the resolved league name to disk alongside `prices.json` so re-runs within a league don't re-query GGG.
3. If the GGG league endpoint returns 0 challenge leagues (rare, e.g., between leagues), fall back to `--league` CLI flag and emit a clear error.
4. For Hardcore variants: the HC league name is typically `"Hardcore <ChallengeName>"` — test by appending "Hardcore " prefix and verifying the poe.ninja response is non-empty. Do not guess other formats.

**Warning signs:**
- `lines` array is empty or shorter than expected on a poe.ninja response.
- Prices fetched look identical to the previous league.
- The `core.timestamp` in the poe.ninja response is weeks old.

**Phase to address:**
`sync` subcommand implementation phase. Add league resolution as the first step before any economy requests.

---

### Pitfall 3: poe.ninja PoE2 API Is Undocumented and Subject to Silent Schema Drift

**What goes wrong:**
The poe.ninja PoE2 economy API (`https://poe.ninja/poe2/api/economy/...`) is not officially documented. The schema used here was reverse-engineered from network traffic and community wrappers. Field names, the structure of the `lines` vs `items` split, and available `overviewName` values may change without notice. A schema change that removes or renames `primaryValue` (or introduces a new pricing field) will silently produce wrong price data — not a deserialization error, because serde's default behavior in Rust drops unknown fields.

Specific risks:
- The `overviewName` / `type` parameter values for PoE2 categories (e.g., what string to pass for unique weapons vs. currency) are not authoritative — confirmed "Currency" and category names like "UniqueWeapon" inferred from PoE1 behavior.
- The PoE2 currency endpoint uses `primaryValue`/`volumePrimaryValue` (not `chaosValue`/`divineValue` as in PoE1). If poe.ninja aligns PoE2 to the PoE1 schema in a future update, the mapping code breaks.
- Rate limit is approximately 12 requests per 5 minutes based on community documentation (LOW confidence — not officially stated). There are no documented rate limit response headers from poe.ninja. Exceeding the limit likely produces a 429 or a Cloudflare block.

**Why it happens:**
Developers rely on examples in GitHub README files that document the PoE1 API and assume PoE2 is identical.

**How to avoid:**
1. Keep the poe.ninja response deserialization in a single, clearly-bounded module (`src/ninja/schema.rs`) with explicit version comments and a last-verified date.
2. Use `#[serde(deny_unknown_fields)]` only on the inner price structs to catch schema additions early in tests. Keep the outer envelope permissive.
3. Log the raw JSON body (at debug level) on any deserialization failure — never silently drop parse errors.
4. Add a canary assertion: after deserializing, assert that the `lines` array is non-empty and that at least one of the three reference currencies (Chaos, Exalt, Divine) was found. Fail loudly if not.
5. Set a meaningful `User-Agent` header. GGG's own API requires `"User-Agent: OAuth <app>/<version> (contact: <email>)"` format; apply the same courtesy to poe.ninja even though it is not required. Use `reqwest::ClientBuilder::user_agent("FerrisFilter/0.1 (contact: jonathasaoc@gmail.com)")`.
6. Space requests with at least 5-second gaps. With ~10 categories to fetch (currency + 8 unique categories + gems), 10 requests at 5s gaps = 50 seconds per sync — acceptable for an infrequent CLI operation.

**Warning signs:**
- Any field used in price calculations (`primaryValue`, `chaosValue`) deserialized as 0.0 or missing on items expected to have value.
- `lines` array length drops suddenly from prior syncs.
- HTTP 429 or 503 responses.

**Phase to address:**
`sync` implementation phase. The schema module and canary assertions should be in the same phase as the first `sync` implementation, not deferred.

---

### Pitfall 4: Missing or Zero Reference Currency Breaks All Tiering Math

**What goes wrong:**
`generate` derives chaos cross-rates from the first three entries in `prices.json` (Exalt, Chaos, Divine per the project spec). If any of these is null, zero, or absent — which poe.ninja can return for items with low listing volume, especially early in a league — all tier threshold calculations either divide by zero, produce NaN/infinity, or silently apply nonsense values.

Cascading failure: a zero Exalt rate means every item priced in Exalts gets normalized to 0 chaos, landing in tier F and being hidden. No Rust panic, no error message — wrong output.

**Why it happens:**
Null handling is written for the common case (mid-league, all three reference currencies actively traded). Early-league or off-meta league states (SSF, Hardcore low-pop) have sparse data.

**How to avoid:**
1. In `prices.json`, store each of the three reference values as `Option<f64>` (or equivalent) — never coerce null to 0.0 during sync.
2. In `generate`, at startup validate that all three reference cross-rates are present and > 0. If any is absent or ≤ 0:
   - Emit a named error (`MissingReferenceRate { currency: "Exalt" }`) and abort with a clear message.
   - Do not fall back silently to 1.0 or any other default. The user must decide how to handle it (re-sync, use `--league standard`, etc.).
3. For non-reference items with null prices: assign tier E (show, no highlight) by convention — the item is known to exist but is unpriced. Never assign tier F (hide) based on a null price.
4. Add a `--dry-run` flag to `generate` that prints the resolved cross-rates before emitting any filter blocks, so the user can spot wrong rates before overwriting their filter.

**Warning signs:**
- `prices.json` first three entries have `null` in `chaos`, `exalt`, or `div` fields.
- All uniques land in tier F after `generate`.
- Currency tier thresholds look implausible (e.g., 0.0 or 1e308).

**Phase to address:**
`generate` subcommand implementation, specifically the price-normalization and tier-assignment logic. Must be designed before writing tiering code, not retrofitted.

---

### Pitfall 5: Non-Deterministic Output Breaks Diffs and Version Control

**What goes wrong:**
`generate` emits a different `universal.filter` on successive runs from identical inputs because:
- Items are iterated over a `HashMap` whose iteration order is not guaranteed in Rust (and changes between runs in newer Rust versions / hash randomization).
- Floating-point comparison of chaos values produces different sort orders for items that round to the same tier boundary (e.g., 4.9999... vs 5.0001... both normalize to tier C vs B, but boundary items may flip).
- `prices.json` entries are written in HashMap iteration order by `sync`, so the "first three entries are Exalt/Chaos/Divine" invariant is not guaranteed unless explicitly sorted.

The filter still works in-game, but `git diff` shows constant noise, and the author cannot tell whether a re-sync changed the filter meaningfully or not.

**Why it happens:**
Developers use `HashMap` for O(1) lookup during tier assignment, then iterate it directly when emitting. Sorting is added as an afterthought only after the noise is noticed.

**How to avoid:**
1. Define a canonical block-emission order in code (mirrors the order in `universal.filter`): recycle → gear bands → flask bands → currency (S→F each category alphabetically) → uniques (by tier desc, then base type name asc).
2. Collect all items into a `BTreeMap` or sort into a `Vec` before emission. Use `BTreeMap` for the tier → items grouping.
3. In `prices.json`, serialize as a JSON array (not object) with a guaranteed order: Exalt first, Chaos second, Divine third, then all other items sorted by name. Use `serde_json::to_string_pretty` with items collected into a `BTreeMap` or pre-sorted `Vec`.
4. For floating-point tier boundaries, use integer chaos-value rounding (`.round() as u64`) before tier comparison to eliminate flip-flop at exact thresholds.
5. Add a test: run `generate` twice from the same inputs, assert the outputs are byte-for-byte identical.

**Warning signs:**
- `git diff universal.filter` after re-running `generate` with unchanged inputs shows block reordering.
- The three reference currencies appear in different positions across `prices.json` syncs.

**Phase to address:**
`generate` implementation. Establish determinism from the first working implementation — retrofitting it after block emission is wired is painful.

---

### Pitfall 6: First-Match-Wins Block Ordering — Unique Items Accidentally Hidden

**What goes wrong:**
The gear fallback block:
```
Hide  # gear fallback
  Class "Body Armours" "Helmets" ...
  Rarity <= Rare
```
uses `Rarity <= Rare` which matches Normal, Magic, and Rare — but **not Unique** (Unique > Rare in PoE ordinal ordering). This is intentional and correct in `universal.filter`. However, when `generate` emits price-tiered unique highlight blocks, if those blocks are placed **after** any `Hide` block that inadvertently matches Unique (e.g., a block without a Rarity upper bound), those uniques will be hidden by the earlier block and the highlight will never fire.

Specific traps:
- Any `Hide` block that omits a `Rarity` condition will match Uniques.
- Any `Hide` block using `Rarity != Unique` as an exclusion works in PoE1 but behavior in PoE2 should be verified.
- A unique-tier highlight block placed after the flask/charm fallback `Hide` but before the gear fallback `Hide` has no effect because the flask block doesn't match gear — but getting the order wrong during code generation is easy.

**Why it happens:**
The emitter generates sections independently (currency section, unique section, gear band section) and concatenates them without verifying that a later `Hide` cannot shadow an earlier `Show`.

**How to avoid:**
1. Codify the canonical section order in the emitter as a type-level guarantee (enum variants in emission order, not a loose string concatenation).
2. Unique highlight blocks must always appear **before** any `Hide` that covers equipment classes and before any gear fallback blocks.
3. Emit `Rarity <= Rare` (not `Rarity < Unique`, which is ambiguous) on every gear `Hide` block to explicitly exclude Uniques from hide rules.
4. Write a post-generation validator that parses the emitted filter and checks: for each unique BaseType in a `Show` block, verify no preceding `Hide` block would match `Rarity == Unique` with that BaseType.
5. Snapshot-test: the generated filter, when the entire `universal.filter` test case is used, must have all unique blocks appearing before the gear-hide block.

**Warning signs:**
- In-game: valuable items you know dropped are not visible, not even on Alt-reveal.
- `generate` tests pass but in-game testing shows missing uniques.
- A `Hide` block in the emitter output contains no `Rarity` condition.

**Phase to address:**
`generate` block-ordering logic. Also apply the post-generation validator in the same phase.

---

### Pitfall 7: GemLevel Condition for Uncut Gems — Unverified in PoE2

**What goes wrong:**
The project spec requires `sync` to include uncut gems only at level ≥ 18. The filter must then emit a `GemLevel >= 18` (or equivalent) condition to match only high-level uncut gems. However, `GemLevel` as a filter condition is **not documented in the NeverSink PoE2 filter syntax reference** (confirmed via DeepWiki as of April 2025) and its behavior in PoE2 is unverified.

If `GemLevel` does not work in PoE2 (which is different from PoE1 in its gem system), the emitted block either matches all gems regardless of level (if the condition is silently ignored) or causes a filter parse error.

**Why it happens:**
The condition exists in the PoE1 grammar documented at `pathofexile.com/item-filter/about`, and developers assume PoE2 supports the same conditions. CLAUDE.md explicitly notes that PoE2 uses the same grammar but different valid values.

**How to avoid:**
1. Before emitting any `GemLevel` condition, verify in-game that the condition parses and behaves correctly against an uncut gem at a known level. This is a one-time manual check.
2. As a safe fallback until verified: use `BaseType == "Uncut Skill Gem" "Uncut Support Gem" "Uncut Spirit Gem"` without a GemLevel filter for the `generate` block, and add a `# TODO: verify GemLevel >= 18 works in PoE2 before adding level filter` comment. This is preferable to silently broken behavior.
3. `sync` should still fetch only level ≥ 18 gems from poe.ninja (that filtering is on the data side, not the filter side). The filter block can then use `ItemLevel >= 70` as a rough proxy (level 18 uncut gems require area/item level ~70) while `GemLevel` is verified. Document this approximation.
4. File the verification as a required in-game test case in the phase that implements gem highlighting.

**Warning signs:**
- The game client shows a filter error on load when `GemLevel` is present.
- All uncut gems match the block regardless of their level.
- No gems match the block at all.

**Phase to address:**
`generate` gem-highlight implementation. Must explicitly flag this as a requires-in-game-verification item before shipping.

---

## Technical Debt Patterns

| Shortcut | Immediate Benefit | Long-term Cost | When Acceptable |
|----------|-------------------|----------------|-----------------|
| Hardcode current league name (e.g., `"Abyss"`) in `sync` | Saves league-resolution logic | Silent stale data next league; user has no indication sync failed | Never — always resolve from GGG `/league` endpoint |
| Use `HashMap` iteration order for filter block emission | Simple code | Non-deterministic output, noisy git diffs | Never — use `BTreeMap` or explicit sort from day one |
| Treat poe.ninja null price as 0.0 in tier math | Avoids `Option` unwrapping | All unpriced items silently land in tier F and get hidden | Never — propagate `None` and error loudly |
| Skip post-generation filter validation | Faster to implement | Broken hide/show ordering ships without detection | Only acceptable in a throwaway prototype; never in a versioned release |
| Emit `BaseType == <unique name>` instead of `BaseType == <base name>` | Seems more precise | Filter condition silently never matches (filter uses base type, not display name) | Never — PoE2 filters always use the base type name for BaseType matching |
| One-shot `sync` that always hits poe.ninja fresh | Simplest code | Rate limit hits if called repeatedly (CI, debugging); unnecessary load on poe.ninja | Acceptable in MVP; add caching (write timestamp to `prices.json`) before any automated use |

---

## Integration Gotchas

| Integration | Common Mistake | Correct Approach |
|-------------|----------------|------------------|
| poe.ninja PoE2 economy API | Using PoE1 endpoint format (`/api/data/itemoverview?league=...&type=...`) | Use PoE2-specific base URL `https://poe.ninja/poe2/api/economy/`; the path structure differs from PoE1 |
| poe.ninja currency endpoint | Using `chaosValue`/`divineValue` field names from PoE1 schema | PoE2 currency endpoint returns `primaryValue` / `volumePrimaryValue`; verify field mapping before use |
| poe.ninja — Hardcore league | Sending `leagueName=Hardcore` | HC league for PoE2 is `"Hardcore <LeagueName>"` — resolve from GGG league list and filter for `category: {"id": "hardcore"}` |
| GGG league endpoint | Calling `api.pathofexile.com/api/leagues` (PoE1 path) | Use `GET /league?realm=poe2` from the developer API; `realm=poe2` is an officially documented parameter |
| poe.ninja — missing items | Assuming an item absent from the response is worthless | Items absent = no trade data; treat as `None` (untiered / show with no highlight), never as tier F |
| PoE2 filter BaseType | Using the unique's display name (e.g., `"Temporalis"`) as the BaseType value | The `BaseType` condition matches the **base item name** (e.g., `"Silk Robe"`). The unique name is not a valid BaseType value. Always map via poe.ninja `baseType` field. |

---

## Performance Traps

| Trap | Symptoms | Prevention | When It Breaks |
|------|----------|------------|----------------|
| Fetching all poe.ninja categories sequentially with no delay | Hitting the ~12 req/5 min limit mid-sync | 5-second sleep between requests; stay well under rate limit | Approximately request 12+ in a 5-minute window |
| Re-fetching poe.ninja on every `generate` run | Unnecessary network calls, slower generate | `generate` reads only from `prices.json` (never calls poe.ninja); sync is a separate explicit operation | Every run after the first if not separated |
| Loading entire `universal.filter` into memory as a string for diffing | Fine for this use case | N/A — file is ~850 lines | Never an issue at this scale |
| Parsing `prices.json` repeatedly per block in `generate` | Slow generation | Load and index once at startup into a `HashMap<String, PriceEntry>` | Never an issue for this scale (~hundreds of items) |

---

## Security Mistakes

| Mistake | Risk | Prevention |
|---------|------|------------|
| Storing `POESESSID` or GGG OAuth token in `config.json` | Token leaked if config is committed to git | poe.ninja PoE2 API requires no auth token; do not accept or store one. GGG league endpoint is unauthenticated read-only. If auth is ever needed, use env var, never config file. |
| Writing `install.bat` or `generate` output directly to the game folder without confirmation | Overwrites a working filter with a broken generated one | `generate` writes to repo root only; user manually copies (or runs `install.bat`). Never auto-install without explicit `--install` flag. |
| Following an unvalidated URL from poe.ninja response (e.g., icon URLs) | SSRF if icon URLs are ever fetched | FerrisFilter never fetches icon URLs from poe.ninja responses. Only price fields are consumed. |

---

## UX Pitfalls

| Pitfall | User Impact | Better Approach |
|---------|-------------|-----------------|
| Silent success when poe.ninja returns empty data | User runs `generate`, filter looks right, but all items are untiered because sync fetched nothing | Print item counts per category after sync: `"Fetched 342 currency, 89 unique items"`. Zero counts are an error. |
| No indication which league was used for prices | User unsure if prices are current-league or stale | Write `"league": "Rise of the Abyssal", "synced_at": "<ISO timestamp>"` into `prices.json`; print these on every `generate` run. |
| `generate` overwrites `universal.filter` with broken output due to a cross-rate error | Working filter destroyed | Write to a temp file first; only rename to `universal.filter` if post-generation validation passes. |
| No `--dry-run` output | User can't preview what tier assignments look like before regenerating | Implement `generate --dry-run` that prints the tier → item mapping table without writing the filter file. |

---

## "Looks Done But Isn't" Checklist

- [ ] **sync league resolution:** Does the tool actually resolve the current challenge league from GGG, or is a league name hardcoded somewhere?
- [ ] **null price propagation:** Are null poe.ninja prices stored as `null` in `prices.json` and handled as `None` in Rust, or silently coerced to `0.0`?
- [ ] **unique base-type collision detection:** Does the emitter detect and handle base types shared by multiple uniques, or does it emit one block per unique name with that name misused as a BaseType?
- [ ] **determinism test:** Does `generate` produce byte-for-byte identical output on two consecutive runs from the same inputs?
- [ ] **GemLevel verification:** Has `GemLevel >= 18` been tested in-game in PoE2 before shipping the gem block?
- [ ] **Rarity guard on Hide blocks:** Does every equipment `Hide` block have `Rarity <= Rare` to prevent it from swallowing uniques?
- [ ] **cross-rate validation:** Does `generate` abort with a clear error (not produce wrong output) when any of the three reference rates is null or zero?
- [ ] **poe.ninja rate limit headroom:** Are requests spaced to stay under the ~12/5min limit, with a 429 retry handler?
- [ ] **schema canary:** Does `sync` assert that Chaos Orb / Exalted Orb / Divine Orb are all present in the currency response?
- [ ] **User-Agent set:** Is a descriptive `User-Agent` header sent on all HTTP requests?

---

## Recovery Strategies

| Pitfall | Recovery Cost | Recovery Steps |
|---------|---------------|----------------|
| Unique base-type collision ships, wrong uniques highlighted | MEDIUM | Add collision detection to `generate`; re-sync + regenerate; no filter syntax change needed, just logic fix |
| Wrong league name used; stale prices in filter | LOW | Re-run `sync --league <correct-name>`; regenerate |
| poe.ninja schema drift breaks sync silently | MEDIUM | Update `src/ninja/schema.rs` to match new fields; re-sync; add regression test with a saved response snapshot |
| Non-deterministic output discovered post-ship | LOW | Replace HashMap iteration with BTreeMap/sort; rerun generate; diffs clean up |
| `GemLevel` condition not supported in PoE2 | LOW | Remove GemLevel condition; substitute ItemLevel proxy or omit gem-level filter entirely until GGG documents it |
| Cross-rate divide-by-zero ships silently | HIGH | Requires logic audit of all tier assignments; regeneration of all prices; add the missing guard and retest |
| Filter with missing `Rarity <= Rare` hides uniques | MEDIUM | Identify which Hide block is at fault from in-game symptom; fix guard in emitter template; regenerate |

---

## Pitfall-to-Phase Mapping

| Pitfall | Prevention Phase | Verification |
|---------|------------------|--------------|
| Unique base-type collision | `generate` unique-tiering implementation | Unit test: given two uniques with same base in prices.json, assert only one block emitted at the higher tier |
| League name not discoverable | `sync` implementation | Integration test: resolve league without hardcoded name; assert league name changes between test leagues |
| poe.ninja schema drift | `sync` implementation (schema module) | Canary assertion test; snapshot test of saved API response |
| Missing/zero reference rate | `generate` price-normalization | Unit test: prices.json with null Chaos rate → `generate` returns non-zero exit code with message |
| Non-determinism | `generate` core emitter | Determinism test: two runs from same inputs produce identical bytes |
| First-match-wins Hide swallowing Uniques | `generate` block-ordering logic | Post-generation validator test; snapshot test against universal.filter |
| GemLevel unverified | `generate` gem-highlight | Manual in-game verification step documented as required acceptance criterion for the phase |
| poe.ninja rate limit | `sync` HTTP client setup | Rate limit test: mock server that returns 429 on request 3+; verify backoff |

---

## Sources

- poe.ninja PoE2 API schema and rate limit: community reverse-engineering via `github.com/Davenads/poeninjaAPI-2025` (MEDIUM confidence)
- poe.ninja item schema (PoE1, largely applicable): `github.com/ayberkgezer/poe.ninja-API-Document` (HIGH confidence for field names; PoE2 schema diverges on currency pricing)
- PoE2 unique base-type matching limitation: confirmed by NeverSink filter docs ("in POE(2) you can't distinguish a specific unique on the same base"), FilterBlade FAQ, and multiple community discussions (HIGH confidence)
- GGG league endpoint with `realm=poe2`: confirmed via `pathofexile.com/developer/docs/reference` (HIGH confidence for parameter existence; live behavior needs verification)
- PoE2 filter syntax valid conditions: NeverSink DeepWiki syntax reference, April 2025 (HIGH confidence for documented conditions; GemLevel not listed = MEDIUM confidence it may not work)
- GGG User-Agent requirement: `pathofexile.com/developer/docs` (HIGH confidence for format; applies to GGG API; courtesy extension to poe.ninja is convention)
- BaseType vs. unique name in PoE2 filters: confirmed via community forum thread `pathofexile.com/forum/view-thread/3592635` and NeverSink repo (HIGH confidence)
- `universal.filter` (this repo): ground truth for correct block ordering, Rarity guard patterns, and section structure (HIGH confidence)

---
*Pitfalls research for: FerrisFilter (Rust CLI — PoE2 filter compiler + poe.ninja price sync)*
*Researched: 2026-05-23*
