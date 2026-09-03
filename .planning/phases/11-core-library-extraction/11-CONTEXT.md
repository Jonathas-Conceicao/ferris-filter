# Phase 11: Core Library Extraction - Context

**Gathered:** 2026-09-03
**Status:** Ready for planning

<domain>
## Phase Boundary

Make the generate pipeline callable as a library API so a second front end (the Phase 13/14
overlay) can drive it without duplicating orchestration, without depending on the process
working directory, and without stdout.

**Critical correction to the roadmap's premise.** The roadmap was written believing the crate had
no library target. It does. Verified on disk 2026-09-03:

- `Cargo.toml` already declares `[lib] name = "ferrisfilter" path = "src/lib.rs"`.
- `src/lib.rs` already exports `cli`, `emitter`, `error`, `install`, `loader`, `run`, `models`,
  `pricer`, `sync`.
- `src/main.rs` is 7 lines and `src/bin/ff.rs` is 6 — both are pure shims calling
  `ferrisfilter::run()`. There is no logic in any binary to extract.

The stale claim came from a v0.1 STATE.md decision note ("seed_prices duplicates minimal currency
structs locally, no lib target in crate") and from `src/bin/seed_prices.rs`'s own header comment,
which still asserts "the crate has no `[lib]` target (only `[[bin]]` targets)". Both are wrong now.

**The real gap.** The library exists but its *generate pipeline is not callable*.
`run::cmd_generate` — which orchestrates load → price-source fallback → `derive_rates` →
`price_tiers` → `emitter::generate` → write — is:

- **private** (`fn cmd_generate`, not `pub`),
- **hardcoding CWD-relative paths** (`"prices.json"`, `"tiers.json"`, `"prices-fallback.json"`,
  `"tiers-fallback.json"`),
- **doing its own output IO** (`std::fs::write(output, …)`),
- **printing** (`println!("wrote {} bytes to {output}")`, `eprintln!("warning: prices.json absent
  or empty — falling back …")`).

An overlay launched from a shortcut has an arbitrary working directory and no stdout, so as things
stand Phase 12 would have to reimplement that orchestration. This phase removes that blocker.

**In scope:** a callable compile API, caller-supplied paths, a typed error enum, structured
reporting of which price source was used, and retiring the `seed_prices` struct duplication.

**Out of scope:** anything touching `dynamic.json`, the overlay, or emitted filter *content*.
Filter output must not change at all — see the acceptance bar below.

</domain>

<decisions>
## Implementation Decisions

### API shape

- **D-01:** The pipeline splits into **two stages** — an IO stage that resolves and loads inputs,
  and a pure compile stage that turns loaded inputs into filter text. Rationale: nearly every
  expected error lives in stage one (read / parse / validate tiers / derive rates), so stage two
  stays trivially pure; and the overlay compiles on every Apply while only needing to read
  `prices.json` once, so the split lets it cache loaded inputs. `emitter::generate(&Config,
  &PricedData) -> Result<String, GenerateError>` is already pure and stays exactly as it is —
  this phase builds the missing layer *above* it, not a replacement for it.
- **D-02:** Exact function names, signatures and struct layout are **Claude's discretion**
  (author: "you can choose whatever you think suits it best"). The two-stage split and the
  properties below are the contract; the spelling is not.

### Path resolution

- **D-03:** All input paths come **from the caller**. The library performs no implicit
  CWD-relative reads. Group them in a single struct (rather than a long positional argument list)
  with a constructor that derives the conventional set from a caller-supplied root directory.
- **D-04:** The CLI passes exactly the relative defaults it uses today, so `generate`,
  `generate --early-game` and `generate --config` keep byte-identical behavior and identical
  working-directory semantics. This is what makes the acceptance bar (no fixture movement)
  achievable.
- **D-05:** Output path handling stays the caller's business too — the library returns filter
  text; writing it is the front end's job. (The CLI keeps its current `std::fs::write` behavior
  and its `wrote N bytes to X` line.)

### Errors and diagnostics

- **D-06:** The library returns a **typed error enum** covering the expected/possible failure
  paths, built with `thiserror` to match the existing `src/error.rs` style. Author's directive:
  *"have an error type to be handled for expected/possible error paths, then the gui can handle it
  as it sees fit and the cli logs the error."* The GUI matches on variants; the CLI renders.
- **D-07:** **Existing diagnostics must survive unchanged.** Variants wrap the current error types
  transparently (`#[error(transparent)]` + `#[diagnostic(transparent)]`, `#[from]`) so
  `JsonLoadError`'s miette source-span rendering and the tier-validation messages are preserved
  verbatim. This is a hard constraint, not a preference — `tests/cli_errors.rs` asserts on the
  rendered stderr and must keep passing untouched. See the acceptance bar.
- **D-08:** Expected failure paths to cover, at minimum: config unreadable, config malformed
  (field-path diagnostic), prices malformed, tiers unreadable/malformed, tier validation failure
  (missing tier, non-ascending), reference-rate derivation failure (TIER-04), and unknown style
  (the existing `GenerateError`). Absence of `prices.json` is **not** an error — it is the
  fallback path, see D-09.
- **D-09:** The price-source fallback decision becomes a **returned value, not an `eprintln!`**.
  The load stage reports which source it selected (live prices / the committed fallback pair /
  none available). The CLI derives today's exact warning text from that value and prints it; the
  overlay renders it however it wants. Same principle as D-06, applied to the non-error half.
  The CLI's stderr text must not change.

### seed_prices deduplication

- **D-10:** `src/bin/seed_prices.rs` imports the real types from the library instead of its five
  local mirror structs (`CurrencyEntry`, `CurrencyCategory`, `CurrencyData`, `PricesFile`,
  `PriceItem`). Its header comment claiming the crate has no `[lib]` target is deleted — it is
  factually wrong and actively misleading.
- **D-11:** `seed_prices` is a one-time migration tool. If the real types cannot be reused without
  distorting them (e.g. the local `PriceItem` is `Serialize`-only while the library's is
  `Deserialize` too), prefer adjusting the *binary* over widening library types for a
  single-use tool. Do not let this small cleanup reshape the models.

### Roadmap correction

- **D-12:** ROADMAP.md Phase 11 and REQUIREMENTS.md CORE-01/CORE-02 are **amended** to describe
  this work (author approved). Left as written, a planner reads criteria that are already
  satisfied and concludes the phase is a no-op. CORE-03 stands as-is — it was always accurate.

### Claude's Discretion

- Exact API naming, signatures, struct/enum layout, and module placement (D-02).
- Whether the two stages live in `run.rs`, a new module, or `lib.rs` — no preference expressed.
- How finely to split the error enum, provided D-07's transparency constraint holds and D-08's
  paths are all representable.
- Plan count and task decomposition.

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Phase scope and requirements
- `.planning/ROADMAP.md` § Phase 11 — goal, dependencies, success criteria (amended per D-12)
- `.planning/REQUIREMENTS.md` — CORE-01, CORE-02, CORE-03 and the milestone acceptance bar
- `.planning/PROJECT.md` — v0.4 milestone scope, pinned/dynamic split, key decisions table

### Research (v0.4)
- `.planning/research/ARCHITECTURE.md` — target front-end/core split; **note its "no lib.rs target
  today" claim is corrected by this CONTEXT**, and its `dynamic.json` sections are Phase 12's
  concern, not this phase's
- `.planning/research/PITFALLS.md` §4 — do not let the GUI work regress the tested core; §5 —
  hermetic fixture regeneration (temp dir + absolute binary path, never `cargo run` from the repo
  root, which picks up a live `prices.json`)

### Code that constrains this phase
- `src/run.rs` — `run()` and the private `cmd_generate`; the orchestration being made callable
- `src/lib.rs` — the existing module exports
- `src/loader.rs` — `load_json`, `load_prices_optional`, `load_tiers`; where path arguments enter
- `src/error.rs` — `JsonLoadError`, the `thiserror` + miette `Diagnostic` pattern to follow (D-07)
- `src/emitter.rs` — `pub fn generate(&Config, &PricedData) -> Result<String, GenerateError>`,
  already pure; `GenerateError` is one of the variants to wrap
- `src/pricer.rs` — `derive_rates` (TIER-04 abort) and `price_tiers`
- `src/bin/seed_prices.rs` — the duplicated structs and the stale `[lib]` comment (D-10)
- `src/cli.rs` — clap surface that must not change
- `tests/cli_errors.rs` — **the constraint on D-07.** Asserts stderr contains `Cannot read`,
  `config.json`, `styles.S.font`, `expected u8`, `missing required tier`, `not strictly ascending`
- `tests/cli_generate.rs` — `artifact_guard`, `generate_snapshot`, `generate_block_counts`,
  `parity_ingame`, and the `generate_on_seed()` helper whose hermetic pattern must be mirrored
- `Cargo.toml` — existing `[lib]` + three `[[bin]]` targets (`ferrisfilter`, `ff`, `seed_prices`)

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- **`[lib]` target already exists** — no Cargo restructuring needed. This phase adds an API
  surface to a library that is already there.
- **`emitter::generate`** is already the pure compile core. The new compile stage should be a thin
  wrapper over it, not a rewrite.
- **`loader::load_json` / `load_prices_optional` / `load_tiers`** already take a path parameter,
  so they need no signature change — the hardcoded strings are all in `run::cmd_generate`'s call
  sites, which is a much smaller change than it first appears.
- **`src/error.rs`'s `JsonLoadError`** is the established `thiserror` + miette `Diagnostic`
  pattern to extend (D-06/D-07).
- **`tempfile`** is already a dependency and `sync` already uses atomic write-then-rename — the
  pattern Phase 12 will need, no new dependency required.

### Established Patterns
- **Byte-for-byte oracle discipline.** `artifact_guard` + the insta snapshot are the project's
  strongest asset across three milestones. A behavior-preserving refactor that moves a fixture did
  not preserve behavior.
- **CRLF normalization in tests** — `s.replace("\r\n", "\n")` on both sides of every string
  comparison; the working tree is CRLF on Windows while the generator emits LF only.
- **Pure-logic / IO split** — established in v0.1 (`sync` fetches, `generate` converts). This phase
  extends that same principle one level up the stack.
- **Deterministic emit order** — unchanged by this phase; nothing here touches emission.

### Integration Points
- `run::run()` → the new load stage → the new compile stage → CLI-side write and print.
- The Phase 12 dynamic compile will call the same two stages with different inputs, which is the
  entire reason this phase exists.
- `seed_prices` → library `models::currency` / `models::prices` types (D-10).

</code_context>

<specifics>
## Specific Ideas

- Author on errors, verbatim: *"have a error type to be handled for expected/possible error paths,
  then the gui can handle it as it sees fit and the cli logs the error."*
- Author on API shape and path resolution: explicit deference — *"you can choose whatever you
  think suits it best"* / *"I'm fine with whatever too."* The contract in D-01/D-03 is Claude's
  proposal, accepted; the naming is open.
- Author on the roadmap discrepancy: *"yes, update it accordingly."*

**Acceptance bar (carried from the milestone, non-negotiable):** `config.json →
universal.filter` behaves exactly as today. `tests/fixtures/seed-universal.filter` and
`tests/snapshots/cli_generate__generate_snapshot.snap` must not change by a single byte, and the
full existing suite (166 tests as of tag `v0.3`) must pass untouched. If a fixture moves during
this phase, the refactor is wrong — do not re-baseline it.

</specifics>

<deferred>
## Deferred Ideas

- **Generalized game-folder / project-root resolution** (e.g. via the `directories` crate). D-03
  gives the library caller-supplied paths, which is enough for this phase; the overlay's own root
  resolution is Phase 13's problem. Noted in `research/STACK.md` as not required for v0.4.
- **`Cargo.toml` still reads `version = "0.1.0"`** while tags read v0.1/v0.2/v0.3. Surfaced at the
  v0.3 close; not this phase's business, but it will keep resurfacing until decided.
- **Untracked working-tree files** (`ssf_*_config.json`, `ssf_*.filter`, `scratch_*`, `foo.filter`,
  `temptest_gen.filter`) — the `ssf_*` presets are what v0.4 retires; decide tracked-vs-ignored
  before the milestone closes.
- **Amending `10-01-SUMMARY.md`** — its prose says `exceptional` is a required `Styles` field; it
  landed as `Option<Style>`. Cosmetic, carried from the v0.3 audit.

</deferred>

---

*Phase: 11-Core Library Extraction*
*Context gathered: 2026-09-03*
