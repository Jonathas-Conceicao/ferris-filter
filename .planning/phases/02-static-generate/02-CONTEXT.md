# Phase 2: Static Generate - Context

**Gathered:** 2026-05-24
**Status:** Ready for planning

<domain>
## Phase Boundary

Phase 2 implements `ferrisfilter generate` so it compiles `config.json` + `currency.json`
into `universal.filter` **deterministically**, reproducing the established filter structure,
locked by an **insta** snapshot test. This phase **re-baselines** the output: the hand-built
`universal.filter` is regenerated with a canonical emission order and committed as the new
golden master (behaviorally equivalent to the original).

**In scope:** the full static emitter (all block types: recycle, gear area-level bands,
flask/charm ladder + fallbacks, currency/category tier blocks), the `config.json` schema
refactor needed to drive a clean emitter, the model-path comment tags, determinism, and the
insta snapshot + artifact-guard tests.

**Out of scope (later phases):** price-driven tiering of currency/uniques and `prices.json`
consumption (Phase 3); the `sync` subcommand / networking (Phase 4). Phase 2 uses the
**curated** tier groupings already in `currency.json`.

</domain>

<decisions>
## Implementation Decisions

### Output fidelity & re-baseline
- **D-01:** **Re-baseline approach.** Define a single canonical action-emission order, regenerate
  `universal.filter`, and commit the regenerated file as the **new golden master**. Block order,
  conditions, `BaseType` lists, and all style values (colors, fonts, minimap, beam, sound) are
  preserved; only intra-block **action order** is normalized.
- **D-02:** **Diff guardrail = behavioral equivalence (minor cleanup allowed).** Beyond action
  reordering, small consistency cleanups are permitted (uniform spacing, comment-header tweaks)
  **as long as the filter's in-game behavior is unchanged** — no block added/removed, no
  condition / `BaseType` / color / numeric value altered.
- **D-03:** Canonical action order is **Claude's discretion** — pick one consistent order and apply
  it to every block (a sensible default is the order most existing blocks already use, e.g. the
  currency-block order). The Recycle block's current divergent order is normalized to match.
- **D-04:** The **original** hand-built `universal.filter` is preserved via **git history** only
  (no working-tree reference copy).

### Acceptance testing
- **D-05:** **Regression lock = insta snapshot** of the generated output string (committed `.snap`,
  updated deliberately via `cargo insta review`). Add `insta` as a dev-dependency.
- **D-06:** **Artifact-guard test:** assert the repo-root committed `universal.filter` byte-equals
  the generator output, so the shipped artifact cannot silently drift from the generator. The
  `.snap` is a separate file, so running `generate` (which writes `universal.filter`) never
  clobbers the snapshot oracle.
- **D-07:** **Determinism test:** generating twice with identical inputs yields identical output
  (GEN-01).
- **D-08:** Exact string / snapshot **equality is the acceptance** — no separate SHA-256 assertion.
  Roadmap's "SHA-256 matches" describes the byte-exact intent, satisfied by equality.
- **D-09:** `ferrisfilter generate` with no `--output` writes to **`universal.filter` at repo root**
  (the live, committed, `install.bat`-copied artifact). `--output <path>` overrides (CLI-03).

### config.json schema refactor (uses D-14 latitude from Phase 1)
- **D-10:** **Refactor the `areaLevelBands` rarity rules** from freeform strings (`"any rarity"`,
  `"Magic + Rare"`, `"Rare only"`, `"Magic"`, `"Quality only"`) into a **typed, closed enum**
  (e.g. `Any | RareOnly | MagicPlusRare | MagicOnly | QualityOnly` — exact variants derived from
  the real values), validated on load. The emitter dispatches on the enum, not a string parser.
- **D-11:** **Model `styles` as a typed struct of named fields** (`S, A, B, C, D, E, F, Recycle,
  unchanged, hidden`) — deterministic, type-safe, complete-by-construction — replacing the Phase 1
  `HashMap<String, Style>` and removing the nondeterminism footgun.
- **D-12:** The Phase 1 `config.rs` models and their round-trip/inline tests **evolve to match** the
  refactored schema; `config.json` is rewritten to the new shape (exact JSON spelling is Claude's
  discretion). The `universal.filter` output remains the frozen (behavioral) baseline.

### Verification & human review (the acceptance gate for the re-baseline)
- **D-13:** The one-time **old → new equivalence is validated by a MANUAL in-game smoke test** (a
  human-UAT item): the user loads the regenerated `universal.filter` in PoE2 and confirms
  highlights/hides look right. This is the **authoritative** equivalence gate — no automated
  semantic-equivalence check is required. (The insta snapshot independently locks the generator
  against *future* regressions.)
- **D-14:** **At the end of Phase 2 execution, produce human-reviewable diffs** for BOTH:
  (a) `config.json` (old → refactored) and (b) `universal.filter` (old hand-built → regenerated),
  saved as artifacts and presented to the user. These two diffs + the in-game test are the human
  acceptance gate. Expect phase verification to surface as **human_needed** until the user signs off.

### Claude's Discretion
- The exact canonical action order; enum variant names and JSON spelling of the refactored rules;
  emitter code structure/modules; how block & section order is derived (must reproduce the
  established sequence — recycle → gear bands → flask/charm fallbacks → currency/category tier
  blocks, etc.); reproduction vs cleanup of the header comment block.

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### The output baseline & generation target
- `universal.filter` — the established block structure, model-path comment tags, header comment
  block, and section order. The behavioral baseline to reproduce (then re-baseline per D-01/D-02).
- `CLAUDE.md` — filter grammar, Dracula tier/color system, the block-tag scheme (`[alwaysShow.K]`,
  `[areaLevelBands "R".K -> styles.X]`), the AreaLevel progression ladder, flask/charm ladder,
  recycle rule, design philosophy, and authoring conventions (incl. the "ignore the trailing
  poe2filter.com config block" note — already stripped).

### Inputs
- `config.json` — the policy model to refactor (D-10/D-11) and compile from.
- `currency.json` — curated currency/category tier groupings
  (`categories[].entries[]{tier,class,bases}`); the source for the tiered currency blocks.
- `src/models/config.rs`, `src/models/currency.rs`, `src/loader.rs`, `src/error.rs` — existing
  Phase 1 models + loader (miette/thiserror) to extend/refactor.

### Requirements & criteria
- `.planning/REQUIREMENTS.md` §Generate — GEN-01 (deterministic), GEN-02 (reproduce structure),
  GEN-03 (model-path comment per block), GEN-04 (no rules for uniques/gems/jewels/jewellery/quest/
  waystones/trials/unidentified — they fall through).
- `.planning/ROADMAP.md` → "Phase 2: Static Generate" — the 4 success criteria.
- `.planning/phases/01-foundation/01-CONTEXT.md` — carried-forward D-14 (schema not frozen / output
  frozen) and D-07 (band rules kept as strings, now interpreted here).

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- `Config` model (typed, `#[serde(deny_unknown_fields)]`) and `CurrencyData`
  (`categories[].entries[]{tier,class,bases}`) — `currency.json` already carries the per-tier base
  groupings, so currency block emission is well-fed (iterate categories then entries — both `Vec`,
  deterministic).
- `loader::load_json` + `JsonLoadError` (miette/thiserror, field-path aware) — reuse for loading
  config/currency in the generate path.
- Single binary crate; tests via inline `#[cfg(test)]` + `assert_cmd` integration tests. Add
  `insta` as a dev-dependency for the snapshot lock.

### Established Patterns / Constraints
- `styles` is currently a `HashMap` (nondeterministic) → D-11 converts it to a typed named-styles
  struct. Any emission that iterates collections must use deterministic order (Vec / named fields).
- Style values are RGB triples (`[u8;3]`); the filter emits `R G B 255` (alpha appended at emit time).
- Action emission order is the crux: the hand file is inconsistent across styles → D-01 normalizes
  to one canonical order.

### Integration Points
- `cmd_generate` in `src/main.rs` currently loads config (typed) + tiers and prints a stub. Phase 2
  replaces the stub body with the real emitter writing to the `--output` path (default
  `universal.filter`).
- `install.bat` copies the repo-root `universal.filter` into the game folder — so repo-root output
  (D-09) keeps the existing install flow working.

</code_context>

<specifics>
## Specific Ideas

- The user will personally **verify the regenerated filter in-game** (D-13) and wants **both diffs
  (config.json + universal.filter) ready for human review at the end of execution** (D-14).
- The bar is **behavioral equivalence**, not byte-identity with the old hand file — minor cleanup is
  acceptable (D-02).
- Going forward, the **insta snapshot is the regression oracle**; the regenerated `universal.filter`
  is the shipped artifact, kept in sync by the guard test.

</specifics>

<deferred>
## Deferred Ideas

- **Price-driven tiering** of currency and uniques (consuming `prices.json`, cross-rate math,
  unique-base collision resolution) → Phase 3. Phase 2 uses the curated tiers in `currency.json`.
- **`sync`** subcommand / poe.ninja networking → Phase 4.
- An **automated semantic-equivalence check** (old vs new filter) was considered but the user chose
  a manual in-game smoke test as the gate instead — not building the automated comparator this phase.

</deferred>

---

*Phase: 02-static-generate*
*Context gathered: 2026-05-24*
