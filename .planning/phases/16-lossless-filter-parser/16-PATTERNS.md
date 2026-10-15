# Phase 16: Lossless Filter Parser - Pattern Map

**Mapped:** 2026-10-12
**Files analyzed:** 14 new/modified
**Analogs found:** 9 exact/role matches / 14 (the rest are greenfield; RESEARCH.md sketches apply)

Phase is a new std-only library module `src/filter/`. Nothing in generate/dynamic/overlay changes.

## Orchestrator Decision (recorded)

The `.gitattributes` `-text` rule is **scoped to `tests/fixtures/parser/*.filter`** (new directory holding the FilterBlade copy and synthetic fixtures), NOT a global `*.filter -text`. Existing `include_str!` fixtures (`seed-universal.filter`, `parity-*.filter`) are not renormalized. Current `.gitattributes` is `* text=auto`, `*.filter text`, `*.md text`, `*.mp3 binary`. The new line must come AFTER `*.filter text` (later patterns win in git). Verify with `git check-attr text tests/fixtures/parser/filterblade.filter` and `git ls-files --eol`.

## File Classification

| New/Modified File | Role | Data Flow | Closest Analog | Match Quality |
|---|---|---|---|---|
| `src/lib.rs` (modify: add `pub mod filter;`) | config | n/a | `src/lib.rs` itself | exact |
| `src/filter/mod.rs` | module root / API | transform | `src/compile.rs` (library API returning values) | role-match |
| `src/filter/lines.rs` | utility | transform (text split) | none (RESEARCH splitter sketch) | no analog |
| `src/filter/keywords.rs` | config (static table) | lookup | `src/emitter.rs` `as_filter_str` enum helpers + `resolve_style` | partial |
| `src/filter/tokenize.rs` | utility | transform | none | no analog |
| `src/filter/parse.rs` | service | transform | none (emitter is the inverse direction) | partial |
| `src/filter/disabled.rs` | service | transform | none | no analog |
| `src/filter/meta.rs` | utility | transform | none | no analog |
| `src/filter/edit.rs` | service | CRUD (in-memory) | `src/overlay/edit.rs` (edit API over in-memory model; not read, check at plan time) | partial |
| `src/filter/diag.rs` | model (errors) | n/a | `src/error.rs`, `src/compile.rs` `CompileError`, `src/emitter.rs` `GenerateError` | role-match |
| `tests/filter_roundtrip.rs` | test | file-I/O + transform | `tests/dynamic_filter.rs`, `tests/cli_generate.rs` | role-match |
| `src/filter/*` inline `#[cfg(test)]` | test | unit | `src/emitter.rs` tests (line 591+), `src/dynamic.rs` tests (~line 296) | exact |
| `tests/fixtures/parser/filterblade.filter` | fixture | file-I/O | `tests/fixtures/seed-universal.filter` (but with `-text`) | role-match |
| `.gitattributes` (modify) | config | n/a | `.gitattributes` itself | exact |
| `CHANGELOG.md` (modify `[Unreleased]`) | docs | n/a | existing `CHANGELOG.md` | exact |

## Pattern Assignments

### `src/lib.rs` (modify)

**Analog:** `src/lib.rs` lines 9-22. Modules are declared flat, alphabetical-ish, with a header comment. Add one line:

```rust
pub mod emitter;
pub mod error;
pub mod filter;   // <- new, between error and install
pub mod install;
```

---

### `src/filter/diag.rs` (model, errors)

**Analogs:** `src/error.rs`, `src/compile.rs` lines 153-185 (`CompileError`), and `src/emitter.rs` imports (`miette::Diagnostic`, `thiserror::Error`).

**Imports/derive pattern** (`src/emitter.rs` lines 8-9, `src/error.rs` lines 1-5):
```rust
use miette::Diagnostic;
use thiserror::Error;

#[derive(Error, Debug, Diagnostic)]
#[error("Failed to load `{file}`: {field_path}: {cause}")]
#[diagnostic(code(ferrisfilter::config::parse_error))]
pub struct JsonLoadError { ... }
```
Apply: `LoadError` (only non-UTF-8 input) uses `#[derive(Debug, Error, Diagnostic)]` with `#[diagnostic(code(ferrisfilter::filter::invalid_utf8))]`. The `Diagnostic { line, severity, message }` struct and `Severity { Error, Warning, Info }` are plain data (not thiserror types); name the struct something other than `Diagnostic` or alias the miette import (`use miette::Diagnostic as MietteDiagnostic`) to avoid a clash. Note `src/compile.rs` documents "No `println!`; return values, `run.rs` presents" (CORE-04): the filter module must never print.

---

### `src/filter/mod.rs` (module root, transform)

**Analog:** `src/dynamic.rs` lines 1-40 (module-level `//!` doc explaining lifecycle and inherited rules) and `src/compile.rs` (pure functions returning values, paths as parameters, no CWD reads).

**Doc/convention pattern** (`src/dynamic.rs` lines 11-15):
```rust
//! Two rules inherited from [`crate::compile`]:
//!
//! - **No `println!` / `eprintln!`.** Functions here return what happened; `run.rs` presents it.
//! - **No implicit working-directory reads.** Every path is a parameter, so the overlay can point
//!   at its own root.
```
Apply: `//!` header stating lossless contract; parser takes `&[u8]`, does no file IO. Public surface: `parse(&[u8]) -> Result<Document, LoadError>`, `Document::serialize()/to_bytes()`, `Document::block(id)`, edit methods.

---

### `src/filter/keywords.rs` (static keyword table)

**Analog:** `src/emitter.rs` lines 139-175 (enum -> filter-string mapping, the opposite direction) and `resolve_style` lines 82-130 (closed-vocabulary lookup with a comment pointing out the "single hardcoded vocabulary" tradeoff).

**Colour/shape vocabulary to reuse as the Enum value lists** (`src/emitter.rs` lines 142-175): Red Green Blue Brown White Yellow Cyan Grey Orange Pink Purple; Circle Diamond Hexagon Square Star Triangle Cross Moon Raindrop Kite Pentagon UpsideDownHouse. These match the official page. Do NOT depend on `models::config` enums for parsing (they lack unknown-value tolerance); duplicate the word lists as `&'static [&'static str]` in the table. `Rarity` ordering (`Normal<Magic<Rare<Unique`, `src/models/config.rs` 168-175) can be mirrored by a word list `["Normal","Magic","Rare","Unique"]`.

**Table shape:** use RESEARCH.md "Keyword table shape" (`Kind`, `Ops`, `Value`, `Keyword`, `KEYWORDS`, `lookup` with `eq_ignore_ascii_case`). Include unit tests: names unique (case-insensitive), table covers every keyword in FilterBlade fixture.

---

### `src/filter/lines.rs`, `tokenize.rs`, `parse.rs`, `disabled.rs`, `meta.rs` (no codebase analog)

Greenfield. Use RESEARCH.md sections verbatim: "Splitter (sketch)" (`split_inclusive('\n')`, `strip_suffix("\r\n")`, BOM via `strip_prefix('\u{feff}')`), "Line tokenizing", "Blocks and ownership", "Disabled (commented-out) blocks", "`#ff:` metadata". Only convention inheritance from the repo: std-only (no new deps), typed errors via thiserror, no panics on user text (no `unwrap` on slices; operate on char-boundary-safe indices), `//!` module docs.

Seam to reuse for expectations: `src/emitter.rs` is the writer for the same grammar. Its output shape (`Show # <name>`, indented body lines, LF-only, one trailing newline; `debug_assert!(out.ends_with('\n'))` at line ~84) is what these files must parse with zero Error diagnostics (generated-dynamic test case).

---

### `src/filter/edit.rs` (service, in-memory CRUD)

**Analog:** `src/overlay/edit.rs` (in-memory edit API over a model; not read in detail by this mapper - planner should skim its function signatures and test style before writing). Behavioural spec comes from RESEARCH.md "Minimal edit API": `set_enabled(id, bool)`, `replace_block_lines(id, Vec<String>)`, `set_meta(id, key, value)`, stable `BlockId(u64)` from a document counter, never indices. Edits touch only the block's line range; new lines inherit the header's `Eol`; last-line-`Eol::None` case handled (RESEARCH Pitfalls 6, 7).

---

### `tests/filter_roundtrip.rs` (integration test)

**Analogs:** `tests/dynamic_filter.rs` lines 1-60 and `tests/cli_generate.rs` lines 1-50.

**Manifest-path and fixture pattern** (`tests/dynamic_filter.rs` 8-30):
```rust
use std::fs;
use std::path::{Path, PathBuf};

fn manifest() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
}
```
Apply: read fixtures as bytes: `fs::read(manifest().join("tests/fixtures/parser/filterblade.filter"))`. Do NOT use `normalize()` (CRLF->LF) here as the existing tests do; that helper exists because working-tree `.filter` files are CRLF on Windows, and it would defeat byte-identity. Instead assert `serialize(parse(bytes)) == bytes` on whatever bytes were read (RESEARCH Pitfall 2).

**Generated-dynamic-filter input:** do not shell out. Use the library, hermetically, like `src/dynamic.rs` test lines 296-305:
```rust
let dir = tempfile::tempdir().unwrap();
fs::copy(repo_file("tiers.json"), dir.path().join("tiers.json")).unwrap();
fs::copy(fixture("seed-prices.json"), dir.path().join("prices.json")).unwrap();
let store = dir.path().join(STORE_FILE);
ensure_store(&repo_file(BASE_FILE), &store).expect("seed");
let compiled = compile_store(&InputPaths::relative_to(dir.path()), &store).expect("compile");
// compiled.text is the generated dynamic filter -> round-trip it
```
(`dynamic::{compile_store, ensure_store, BASE_FILE, STORE_FILE}`, `compile::InputPaths`; all public.) Note the doc comment at the top of `tests/dynamic_filter.rs`: never run against repo-root `prices.json`.

**Reference corpus (git-ignored, tolerate absence):** `read_dir(manifest().join("reference-old-filters"))`, filter `extension == "filter"` (excludes `#end_endgame.filter#`), `eprintln!` a skip note and return when missing. Synthetic cases are in-test `&[u8]` literals, never files.

**Helper to add:** `assert_bytes_eq(a, b)` reporting first differing offset plus a 40-byte window from each side (RESEARCH "Byte-diff helper").

---

### Inline unit tests in `src/filter/*.rs`

**Analog:** `src/emitter.rs` lines 591-607 and `src/dynamic.rs` tests.
```rust
#[cfg(test)]
mod tests {
    use super::*;
    ...
    include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/seed-prices.json"))
```
Apply: small, fast tests in-module (`cargo test --lib filter::`); use `include_bytes!`/`fs::read` for the big fixture only in the integration test. For `seed-universal.filter` prefer `fs::read` over `include_str!` so the test sees checkout bytes without any normalization.

---

### `tests/fixtures/parser/filterblade.filter` and `.gitattributes`

**Copy procedure:** binary `cp` from `C:\Users\jonat\OneDrive\Documentos\My Games\Path of Exile 2\filterblade.filter`; confirm 258,094 bytes and 5312 CRLF line endings. Add to `.gitattributes` after the `*.filter text` line:
```
# Parser fixtures must keep their exact bytes (CRLF/LF/no-final-newline); never renormalize.
tests/fixtures/parser/*.filter -text
```
Existing fixtures at `tests/fixtures/seed-universal.filter`, `parity-*.filter` stay governed by `* text=auto` + `*.filter text` and remain untouched. Do not edit `seed-universal.filter` or `artifact_guard` (CLAUDE.md).

Open item for planner (A5): fixture redistribution licence of the NeverSink/FilterBlade file is user-confirmable.

## Shared Patterns

### Library-returns-values, never prints
**Source:** `src/dynamic.rs` lines 11-15, `src/compile.rs`. **Apply to:** all `src/filter/*`.

### Typed errors, diagnostics not Err
**Source:** `src/compile.rs` `CompileError` (153-185), `src/error.rs`. **Apply to:** `diag.rs`. `Err` only for non-UTF-8 (`String::from_utf8`, no lossy); grammar problems are `Vec<Diagnostic>` on the `Document`. Enabled-block problems = Error; unknown keyword = Warning; disabled-block problems = Info/Warning.

### Hermetic fixtures
**Source:** `tests/dynamic_filter.rs` header comment + `workspace()` helper. **Apply to:** all tests. Temp dirs via `tempfile` (already a dependency), never `cargo run` from repo root, never repo-root `prices.json`.

### Preserve operators and CRLF
**Source:** CLAUDE.md filter grammar notes (`==` exact vs bare substring). The parser records operator text verbatim; tests assert every `\n` is preceded by `\r` after editing a CRLF file.

### Changelog
**Apply to:** `CHANGELOG.md` `[Unreleased]`: add a lossless filter parser entry.

## No Analog Found

| File | Role | Data Flow | Reason |
|---|---|---|---|
| `src/filter/lines.rs` | utility | transform | No existing text splitter; existing code only writes with `writeln!` (LF). Use RESEARCH splitter sketch. |
| `src/filter/tokenize.rs` | utility | transform | Repo has no filter-text reader. |
| `src/filter/parse.rs` | service | transform | Emitter is the inverse; only its output shapes are useful as test input. |
| `src/filter/disabled.rs` | service | transform | New concept. |
| `src/filter/meta.rs` | utility | transform | New concept (`#ff:` runs). |

## Metadata

**Analog search scope:** `src/*.rs`, `src/models`, `tests/*.rs`, `tests/fixtures`, `.gitattributes`, `Cargo.toml`
**Files scanned:** about 12 read (lib.rs, error.rs, emitter.rs partial, compile.rs partial, dynamic.rs partial, models/config.rs partial, tests/dynamic_filter.rs, tests/cli_generate.rs, .gitattributes, Cargo.toml); `src/overlay/edit.rs` listed but not read
**Pattern extraction date:** 2026-10-12
