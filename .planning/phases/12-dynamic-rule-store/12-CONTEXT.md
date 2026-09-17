# Phase 12: Dynamic Rule Store - Context

**Gathered:** 2026-09-03
**Status:** Ready for planning

<domain>
## Phase Boundary

A persisted `dynamic.json` compiles to a complete, playable `dynamic.filter` — dynamic policy
blocks plus the pinned economy sections — entirely headlessly, so the feature is verifiable on the
project's existing testing terms before any GUI exists.

**The author supplied `dynamic_base.json` during this discussion, and it changes the shape of the
phase.** It is a hand-edited copy of `config.json`: structurally identical (same ten top-level
keys), 10 rules instead of 20, **zero `areaLevel` conditions**, and the full pinned `styles` block.

Verified live during discussion: `generate --config dynamic_base.json` already produces a valid
**69 Show / 1 Hide** filter *including the currency, gem and unique economy sections*. Phase 11's
`load_inputs` + `compile` compiles it with **no new emitter code**.

So this phase is not "build a compile path" — that already works. It is:

- add an `enabled` flag so a rule can be switched off without being deleted,
- seed and persist `dynamic.json`, with a reset that restores from `dynamic_base.json`,
- write `dynamic.filter` atomically into the game folder,
- give the header honest text,
- expose the whole thing headlessly.

**In scope:** the `enabled` field and its emit behavior; `dynamic.json` seeding, persistence and
reset; atomic write to the game folder; parameterized header; a headless entry point.

**Out of scope:** the overlay window (Phase 13), any editing UI (Phase 14), and any change to
emitted economy content. `config.json → universal.filter` must stay byte-identical — the milestone
acceptance bar.

</domain>

<decisions>
## Implementation Decisions

### The dynamic store is a full Config, not a rules array

- **D-01:** `dynamic.json` is a **complete `Config`-shaped file**, exactly like `dynamic_base.json`
  — not the `{ version, rules: [...] }` envelope sketched in `.planning/research/ARCHITECTURE.md`.
  That sketch predates the author's file and is **superseded**. Compiling the dynamic store is
  therefore `load_inputs(InputPaths::conventional().with_config("dynamic.json"), Auto)` followed by
  `compile` — the Phase 11 API, unchanged.
- **D-02:** This means the pinned halves (styles palette, class lists, `alwaysShow`, `build`) live
  *inside* `dynamic.json` rather than being read from `config.json`. They are pinned by *policy*
  this milestone — the overlay does not edit them — not by living in a separate file.
- **D-03:** Requirement DYN-06's "economy sections from `prices.json`" is already satisfied by the
  existing pricer path; no splicing of two configs is needed.

### Rule identity: array order, no `name`

- **D-04:** Ordinal position in the `rules` array **is** the identity and the precedence. No `name`
  field and no stored index. The author raised a persistent index and then deferred
  ("the json's own list ordering is fine by me if you think so") — array order wins because a
  stored index is a second source of truth that silently desyncs the moment a rule is reordered,
  which is a first-class operation in Phase 14.
- **D-05:** Strip the `name` key from every rule in `dynamic_base.json`. Today they carry
  `"name": ""` (empty string, key present), which is why blocks currently emit `Show # ` with
  nothing after it — `emit_rules`'s `unwrap_or("rule {idx}")` fallback only fires when the key is
  **absent**. Removing the key makes the existing fallback produce `Show # rule 4`, giving each
  emitted block a traceable pointer back to its index in `dynamic.json`. **No emitter change is
  needed for this** — just fix the data.
- **D-06:** `Rule.name` stays `Option<String>` in the model. `config.json` still uses it and its
  readable block comments must not change (the oracle). Dynamic files simply omit it.

### Disabled rules

- **D-07:** Add `enabled` to `Rule` as `Option<bool>` with `#[serde(default, skip_serializing_if
  = "Option::is_none")]`. **Absent means enabled** — so `config.json` and every existing fixture
  parse and serialize unchanged, and the oracle does not move.
- **D-08:** A disabled rule **stays in `dynamic.json`** and is **omitted entirely from the emitted
  filter** — not emitted as a commented-out block. The author left this open ("not be generated
  (or be generated as comment only)"); omitting wins because the filter is machine-generated and
  never read by hand, so a commented block is pure noise in a file the game parses.
- **D-09:** Deleting a rule is a separate operation from disabling it — removal from the array.
  The array supports it natively; the UI affordance is Phase 14's.

### Header

- **D-10:** Parameterize `emit_header` to take the output filename and the source filename, so
  `universal.filter` keeps its exact current header and `dynamic.filter` says
  `# dynamic.filter` / `# GENERATED FROM dynamic.json`.
  The author was willing to just change the hardcoded strings and re-baseline, and did not want
  time spent here ("it's not an important part"). Parameterizing is chosen anyway because it is
  ~2 lines and it is the only option that does **not** move `seed-universal.filter` — the
  milestone's acceptance bar and the guarantee that made Phase 11 verifiable. Cost is the same;
  risk is not.
- **D-11:** The header's remaining body text (the block-tag legend, the philosophy note) is left
  exactly as-is for both paths. Rewording it for the dynamic case is deferred.

### Seeding, persistence and reset

- **D-12:** `dynamic_base.json` is committed at the repo root and is the **template**: the source
  for first-run seeding *and* for a future "start from scratch" reset. It is never written to by
  the tool.
- **D-13:** `dynamic.json` is the **working copy**, at the repo root, created by copying
  `dynamic_base.json` when absent. It is the file the overlay will edit, and it is what persists
  across restarts (DYN-03). It should be git-ignored like `prices.json` and `universal.filter` —
  it is per-user state, not committed policy.
- **D-14:** Reset = re-copy `dynamic_base.json` over `dynamic.json`. Exposing that as a user
  action in the overlay is Phase 14's; Phase 12 only needs the seeding path to exist and be
  reachable.

### Headless entry point

- **D-15:** Add a CLI subcommand for the dynamic flow (author: *"I'm fine with whatever here"*).
  A subcommand rather than library-only because it makes the whole feature usable and scriptable
  the moment this phase lands — which is the point of sequencing Phase 12 before the overlay: if
  Phase 13 turns out to be a fight, something already shipped.
- **D-16:** It writes `dynamic.filter` into the game folder by default, reusing
  `install::default_dest()` (already resolves `%USERPROFILE%\OneDrive\Documentos\My Games\Path of
  Exile 2`), with an override flag in the same spirit as `install --dest`. The write is atomic
  (write-then-rename via `tempfile`, the discipline `sync` already uses for `prices.json`).
- **D-17:** Exact subcommand name, flags and module placement are **Claude's discretion**.

### Claude's Discretion

- Subcommand naming and flag surface (D-17).
- Where the seeding/reset helper lives.
- How finely to split the two plans.
- Whether `dynamic_base.json`'s `meta.name` changes from `"universal"` to something honest —
  cosmetic, no behavioral effect, but it is currently misleading.

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Phase scope and requirements
- `.planning/ROADMAP.md` § Phase 12 — goal, dependencies, 8 success criteria
- `.planning/REQUIREMENTS.md` — DYN-01…DYN-08 and the milestone acceptance bar
- `.planning/PROJECT.md` — v0.4 scope, the pinned-vs-dynamic split

### Prior phase (this phase builds directly on it)
- `.planning/phases/11-core-library-extraction/11-CONTEXT.md` — the two-stage API's design decisions
- `.planning/phases/11-core-library-extraction/11-VERIFICATION.md` — **read the Lesson section.**
  A green suite is not evidence for a behavior-preserving change; capture a baseline and diff it.

### Research — with one correction
- `.planning/research/ARCHITECTURE.md` — target front-end/core split. **Its `dynamic.json` sketch
  (`{version, rules: [...]}`) is superseded by D-01**: the store is a full `Config`.
- `.planning/research/PITFALLS.md` §5 — regenerate fixtures hermetically (temp dir + absolute
  binary path, never `cargo run` from the repo root, which picks up a live `prices.json`); §7 —
  atomic writes so the game never sees a truncated filter

### Code that constrains this phase
- `dynamic_base.json` (repo root) — the author's base: 10 rules, no `areaLevel`, full styles
- `src/compile.rs` — `InputPaths`, `load_inputs`, `compile`; the dynamic path reuses these as-is
- `src/emitter.rs` — `emit_header` (the two hardcoded strings, D-10); `emit_rules` (the
  `unwrap_or("rule {idx}")` fallback at the `name` lookup, and where `enabled` must be honored)
- `src/models/config.rs` — `Config`, `Rule`, `When`; `Rule` gains `enabled` (D-07)
- `src/install.rs` — `default_dest()`, the game-folder resolution to reuse (D-16)
- `src/sync/prices.rs` — `write_prices_atomic`, the atomic-write pattern to mirror (D-16)
- `src/cli.rs` — the clap surface the new subcommand joins
- `tests/cli_generate.rs` — `artifact_guard`, `generate_snapshot`, `generate_block_counts`
- `.gitignore` — where `dynamic.json` needs an entry (D-13)

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- **The entire Phase 11 compile API.** `load_inputs(&InputPaths, PriceSourcePolicy)` + `compile()`
  already do the dynamic path; `.with_config("dynamic.json")` is the whole integration.
- **`emit_rules`'s existing name fallback** produces `rule {idx}` for nameless rules — D-05 needs
  only a data fix, not a code change.
- **`install::default_dest()`** already resolves the game folder.
- **`sync`'s atomic write-then-rename** is the pattern for DYN-07; `tempfile` is already a dep.
- **`Rule`/`When` are reused wholesale** — DYN-01's "reusing the existing types rather than new
  ones" is satisfied by construction, and `When`'s all-`Option` shape already gives DYN-04
  (a condition absent emits nothing) with no new work.

### Established Patterns
- **Additive optional fields with `serde(default, skip_serializing_if)`** — exactly how
  `corrupted` landed in v0.3 and how `enabled` should land, so existing files round-trip unchanged.
- **Byte-for-byte oracle discipline.** `artifact_guard` + insta snapshot must not move.
- **Behavioral baselines over green suites** (Phase 11's lesson) — for the header change
  especially, diff generated output before and after rather than trusting the suite.
- **CRLF normalization** in every test string comparison.

### Integration Points
- New subcommand → `compile::load_inputs`/`compile` → atomic write → game folder.
- Phase 14's Apply action calls the same path; Phase 12 should leave it callable as a function,
  not only as a subcommand.
- `enabled` is written by Phase 14's UI and read by `emit_rules` here.

</code_context>

<specifics>
## Specific Ideas

- Author on the base file: *"i've created dynamic_base.json copied from config.json with some hand
  alterations to serve as a base for the dynamic config I want, it can also be used as a base for
  a 'reset' we can have to start a new filter from 'scratch' without losing everything."*
- Author on disabled rules: *"add an enabled label, a disabled rule should still be in the json but
  not be generated … and we should have a delete option in case we actually want the rule gone."*
- Author on the header: *"it's not an important part we shouldn't dwell on it too long"* — hence
  D-10 taking the cheapest option that protects the oracle, and D-11 deferring the body text.
- Author on schema compatibility: *"we can break json compatibility for this new design"* — noted,
  but **not needed**: `name` is already `Option`, and `enabled` defaults. Nothing breaks.

**Acceptance bar (unchanged, non-negotiable):** `tests/fixtures/seed-universal.filter` and
`tests/snapshots/cli_generate__generate_snapshot.snap` must not change by a single byte, and the
full suite (176 tests as of Phase 11) must stay green.

</specifics>

<deferred>
## Deferred Ideas

- **Reset exposed as a user action** — Phase 14 (the overlay's UI). Phase 12 only needs the seed
  path to exist.
- **Rewording the header's body text** (block-tag legend, philosophy note) for the dynamic case —
  D-11.
- **Fully parameterized header / config-driven header text** — the author explicitly deferred this
  until the flow is validated.
- **`Cargo.toml` still reads `version = "0.1.0"`** while tags read v0.3 — carried from the v0.3
  close, still unresolved.
- **Untracked `ssf_*` / scratch files** awaiting a tracked-vs-ignored decision. Related: the four
  `ssf_*_config.json` presets are what this milestone retires, so this decision gets easier once
  the dynamic flow works.

</deferred>

---

*Phase: 12-Dynamic Rule Store*
*Context gathered: 2026-09-03*
