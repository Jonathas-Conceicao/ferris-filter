# Phase 11: Callable Compile API — Discussion Log

**Date:** 2026-09-03
**Mode:** default (interactive, prose-first)
**Participants:** author + Claude

> Human reference only — audits and retrospectives. Downstream agents read
> `11-CONTEXT.md`, not this file.

## Pre-discussion finding (drove the whole session)

Codebase scout contradicted the roadmap's premise before any question was asked.

**Roadmap assumed:** the crate has no `[lib]` target, so the models/pricer/emitter need
extracting and the CLI needs thinning.

**Actually on disk (verified 2026-09-03):**

| Claim | Reality |
|---|---|
| No library target | `Cargo.toml` declares `[lib] name = "ferrisfilter" path = "src/lib.rs"` |
| Modules not exported | `src/lib.rs` exports `cli`, `emitter`, `error`, `install`, `loader`, `run`, `models`, `pricer`, `sync` |
| Logic lives in the binary | `src/main.rs` = 7 lines, `src/bin/ff.rs` = 6 lines; both call `ferrisfilter::run()` |
| `seed_prices` blocked on missing lib | Genuinely still duplicates 5 structs — CORE-03 was accurate |

**Source of the stale belief:** a v0.1 STATE.md decision note ("seed_prices duplicates minimal
currency structs locally, no lib target in crate") plus `src/bin/seed_prices.rs`'s own header
comment, which still says "the crate has no `[lib]` target (only `[[bin]]` targets)". A lib target
was added later and neither note was updated. The v0.4 research inherited the error.

**The real blocker, found by reading `src/run.rs`:** `cmd_generate` is private, hardcodes
`"prices.json"` / `"tiers.json"` / `"prices-fallback.json"` / `"tiers-fallback.json"` as
CWD-relative strings, calls `std::fs::write` itself, and prints via `println!`/`eprintln!`. An
overlay launched from a shortcut has an arbitrary working directory and no stdout, so Phase 12
would have had to reimplement the entire orchestration.

This is the second time in two milestones that a stale note cost real time — the v0.3
retrospective recorded the same lesson about `parity_ingame`.

## Areas discussed

### 1. API shape

**Question:** What does the overlay actually call? A free function taking explicit paths, a struct
holding pre-loaded inputs, or a two-stage split (resolve inputs → compile)?

**Author:** *"you can choose what ever you thing suits it best"*

**Decision (Claude's discretion, D-01/D-02):** Two-stage split — an IO stage that resolves and
loads inputs, then a pure compile stage. Reasoning given: nearly all expected errors live in stage
one, so stage two stays trivially pure; and the overlay compiles on every Apply while only needing
to read `prices.json` once, so the split lets it cache. `emitter::generate` is already pure and is
left untouched — the phase builds the missing layer above it. Exact naming left open.

### 2. Path resolution

**Question:** Explicit `&Path`s from the caller, or a "project root" concept the library resolves
against?

**Author:** *"I'm fine with whatever too"*

**Decision (Claude's discretion, D-03/D-04/D-05):** Caller-supplied paths grouped in one struct
with a `relative_to(root)`-style constructor. The library performs no implicit CWD reads. The CLI
passes today's same relative defaults so its behavior stays byte-identical — which is what keeps
the no-fixture-movement acceptance bar reachable.

### 3. Diagnostics and errors

**Question:** The fallback chain emits warnings a GUI needs to show. Structured return values, a
caller-supplied callback, or silence?

**Author:** *"have a error type to be handled for expected/possible error paths, then the gui can
handle it as it sees fit and the cli logs the error"*

**Decision (D-06 … D-09):** A typed `thiserror` enum returned from the library; each front end
decides presentation. Two refinements Claude added and flagged:

- **D-07 constraint discovered mid-discussion.** `tests/cli_errors.rs` asserts on rendered stderr
  containing `Cannot read`, `config.json`, `styles.S.font`, `expected u8`, `missing required tier`
  and `not strictly ascending` — all produced by `JsonLoadError`'s miette `Diagnostic` with source
  spans. The enum must wrap existing errors transparently (`#[error(transparent)]` +
  `#[diagnostic(transparent)]`) or those tests break. Checked before locking the design.
- **D-09.** The price-source fallback is not an error, so the same principle is applied to the
  non-error half: the load stage *returns* which source it selected, the CLI derives its existing
  warning text from that value, and the overlay renders it as it likes.

### 4. Roadmap correction

**Question:** Amend Phase 11's goal and criteria to match the real work, or leave the roadmap and
note the discrepancy only in CONTEXT.md?

**Author:** *"yes, uppdate it accordingly"*

**Decision (D-12):** Amended. Phase 11 renamed *Core Library Extraction* → **Callable Compile
API**; goal rewritten; success criteria replaced (5 → 8, now covering the error type, the
unmodified `cli_errors.rs` run, and the price-source value). REQUIREMENTS.md CORE-01 and CORE-02
re-aimed at the actual blocker rather than deleted — the underlying intent was always unmet — and
**CORE-04 added** for the typed-error/price-source requirement, which had no home before. A
provenance note was left in both files. Milestone total: 23 → 24 requirements.

## Claude's discretion (recorded)

- API naming, signatures, struct/enum layout, module placement.
- Whether the two stages live in `run.rs`, a new module, or `lib.rs`.
- Error enum granularity, subject to D-07's transparency constraint.
- Plan count and task decomposition.

## Deferred ideas

Captured in `11-CONTEXT.md` `<deferred>`; none were scope creep from the author — all surfaced by
Claude while scouting:

- Generalized game-folder / project-root resolution (`directories` crate) — Phase 13's problem.
- `Cargo.toml` still at `version = "0.1.0"` while tags read v0.1/v0.2/v0.3.
- Untracked `ssf_*` / scratch files awaiting a tracked-vs-ignored decision.
- `10-01-SUMMARY.md` prose stale on `exceptional` being required vs optional.

## Process note

No `AskUserQuestion` menus were used this session. Per
`memory/prefers-discussion-over-option-votes.md`, open design questions are put in prose — three
consecutive option-votes were declined earlier in this session because the author's context kept
reframing the question itself. The prose form worked: the answer to area 3 arrived as a directive
that settled areas 1 and 2 by delegation.

---

*Phase: 11-core-library-extraction*
*Logged: 2026-09-03*
