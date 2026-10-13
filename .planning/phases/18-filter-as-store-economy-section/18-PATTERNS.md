# Phase 18: Filter as Store & Economy Section - Pattern Map

**Mapped:** 2026-10-12
**Files analyzed:** 14 new/modified (plus deletions)
**Analogs found:** 13 / 14

## File Classification

| New/Modified File | Role | Data Flow | Closest Analog | Match Quality |
|-------------------|------|-----------|----------------|---------------|
| `src/paths.rs` (NEW, AppRoot) | utility | request-response (pure resolver) | `src/compile.rs` `InputPaths` (:55-118) + `overlay/store.rs::default_paths` (:93) | role-match |
| `src/section.rs` (NEW, markers + v0.4 migrate) | utility | transform (pure text) | `src/filter/meta.rs` + `src/filter/lines.rs` (marker/line API); splice shape from RESEARCH | partial |
| `src/safe_write.rs` (NEW, guard/backup/atomic) | service | file-I/O | `src/dynamic.rs::write_filter` (:156-185) | exact (extend) |
| `src/dynamic.rs` (REWRITE) | service | file-I/O + transform | itself (`DynamicError`, module header rules, `default_destination`) | exact |
| `src/compile.rs` (SPLIT `load_inputs`) | service | batch/transform | itself (:195-268) | exact |
| `src/emitter.rs` (SPLIT economy emitters) | service | transform | itself (`generate_as` :92-105, `emit_currency_sections` :429, `emit_unique_sections` :501) | exact |
| `src/cli.rs` (Apply args) | config | request-response | `Commands::Apply` (:60-96) and `Install` | exact |
| `src/run.rs` (`cmd_apply`) | controller | request-response | `cmd_apply` (:127-198), `cmd_generate` (:62-125) | exact |
| `src/lib.rs` | config | n/a | existing `pub mod` list | exact |
| `src/overlay/app.rs`, `apply.rs`, `mod.rs` (re-point; delete `store.rs`) | component | event-driven | `overlay/apply.rs` (`ApplyOutcome`, `success_message`) | role-match |
| `tests/dynamic_filter.rs` (REWRITE) | test | request-response | itself (`workspace`, `apply`, `economy_section`) | exact |
| `tests/cli_errors.rs` (flag-removal test) | test | request-response | existing cli_errors tests | role-match |
| `tests/fixtures/v04-dynamic.filter` + `.gitattributes` | fixture | n/a | Phase 16 `-text` fixtures | role-match |
| `CLAUDE.md`, `CHANGELOG.md`, `.gitignore`, `loader.rs:121` comment | config/docs | n/a | existing rows/sections | exact |

## Pattern Assignments

### `src/safe_write.rs` (service, file-I/O)

**Analog:** `src/dynamic.rs::write_filter` (lines 156-185). Keep: same-dir temp, `is_dir()` guard (never create), `persist`. Add: guard, backup, retry using `e.file` from `PersistError`.

```rust
pub fn write_filter(dest_dir: &Path, text: &str) -> Result<PathBuf, DynamicError> {
    if !dest_dir.is_dir() {
        return Err(DynamicError::DestinationMissing { path: dest_dir.display().to_string() });
    }
    let mut temp = tempfile::NamedTempFile::new_in(dest_dir).map_err(|e| DynamicError::FilterUnwritable { path: target.display().to_string(), source: e })?;
    std::io::Write::write_all(&mut temp, text.as_bytes()).map_err(...)?;
    temp.persist(&target).map_err(|e| DynamicError::FilterUnwritable { path: ..., source: e.error })?;
```

Error enum style to copy (dynamic.rs:57-98): `#[derive(Debug, thiserror::Error, miette::Diagnostic)]`, `#[error("Cannot write `{path}`: {source}")]` with `path: String` + `#[source] source: std::io::Error`, optional `#[diagnostic(help(...))]`, and `Compile(#[from] CompileError)` with `#[error(transparent)] #[diagnostic(transparent)]`.
Module-header rules to repeat (dynamic.rs:1-22): no `println!`/`eprintln!`; no implicit cwd reads; all paths are parameters. Tests use `tempfile::tempdir()`.

### `src/dynamic.rs` (rewrite)

**Analog:** its own current shape. Keep `FILTER_FILE`, `default_destination()` (delegates to `crate::install::default_dest()`, returns `Option<PathBuf>`), `DestinationMissing`. Delete `BASE_FILE`, `STORE_FILE`, `IDENTITY`, `StoreState`, `ensure_store`, `reset_store`, `copy_template`, `CompiledFilter`, `compile_store`, `write_filter`, `TemplateUnreadable`, `StoreUnwritable`. New `LoadedFilter` (keeps raw `disk: Option<Vec<u8>>`, `migrated`), `apply_text`, `apply_file`. `disk` is always the original bytes (Pitfall 8). Parse via `filter::parse(&[u8]) -> Result<Document, LoadError>` (src/filter/mod.rs:37); it only fails on invalid UTF-8.

### `src/section.rs` (utility, pure text)

**Analog APIs** (no direct section analog exists):
```rust
// src/filter/lines.rs:40
pub fn ff_marker(&self) -> Option<(&str, Option<&str>)>   // key-only marker => (key, None)
// src/filter/meta.rs tests: parse_ff_line("#ff:economy-begin") == Some(("economy-begin", None))
// Line::text(), Line::eol() -> Eol{Lf,CrLf,None}.as_str(); Document::lines(), has_bom(), to_bytes()
// Block::owned() -> Range<usize> (header..end), Block::leading(), Block::header_line()
// Document::set_meta(id, "name", value) -> Result<(), EditError>   (edit.rs:336)
```
Splice shape (from RESEARCH): copy `lines[..=begin]`, emit new body lines with the begin line's `eol`, copy `lines[end..]`; BOM via `has_bom()`. Never `str::find` for markers. Marker scan: loop `doc.lines().iter().enumerate()` matching `l.ff_marker().map(|(k,_)| k)` on `"economy-begin"` / `"economy-end"`; reject marker index inside any `block.owned()`. Blank line after begin descriptor and before end marker is load-bearing (parser attaches comment runs directly above a header as `leading()`).
Test style: table-driven `rt(b)` helper in `filter/mod.rs` tests (LF, CRLF, no final newline, BOM inputs).

### `src/paths.rs` (utility)

**Analog:** `InputPaths` doc/constructor style (compile.rs:55-90) and `overlay/store.rs::default_paths` (:93-96), the only `current_dir()` call:
```rust
pub fn default_paths() -> (PathBuf, PathBuf) {
    let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    (cwd.join(dynamic::BASE_FILE), cwd.join(dynamic::STORE_FILE))
}
```
Replace with pure `resolve_from(env, exe_dir, cwd)` + thin `app_root()`. Hand the result to `InputPaths::relative_to(&root)` (compile.rs:79). Do not `canonicalize`. Conventional file names are only spelled in compile.rs (`CONFIG_FILE`, `STYLES_FILE`, ...); use `STYLES_FILE`/`CONFIG_FILE` consts for the walk-up marker, don't re-spell.

### `src/compile.rs` (split `load_inputs`)

**Analog:** `load_inputs` lines 198-262. Move the `match policy { ... }` + `priced` derivation verbatim into a private fn / `load_economy(paths, policy) -> EconomyInputs { styles, priced, price_source }`; keep `load_inputs` assembling `CompileInputs { config, priced, price_source }` identically. Config part:
```rust
let config: Config = loader::load_config(&path_str(&paths.config), &path_str(&paths.styles))?;
config.styles.check_rules(config.rules.as_deref().unwrap_or(&[]), &path_str(&paths.styles)).map_err(LoadError::from)?;
```
Economy part needs only `loader::load_styles(...)` (loader.rs:~190) rather than `load_config`. Existing tests (compile.rs :398-600) using `InputPaths::relative_to(dir.path())` + `PriceSourcePolicy::Auto` are the template for `load_economy` tests. `PriceSource::Absent` must make Refresh refuse (do not emit an empty body).

### `src/emitter.rs` (split economy)

**Analog:** `generate_as` (92-105):
```rust
config.styles.validate()?;
let mut out = String::with_capacity(32_000);
emit_header(&mut out, identity);
emit_rules(&mut out, config)?;
emit_currency_sections(&mut out, config, &priced.category_blocks)?;
emit_unique_sections(&mut out, config, &priced.unique_blocks)?;
```
Change only the parameter `config: &Config` to `styles: &StyleSet` in the two emitters (they use only `style_for_tier(&config.styles, ..)` at :453 and :518), then call with `&config.styles`. Do not restructure buffers: both rely on `if out.ends_with("\n\n") { out.pop(); }` and unique section assumes the currency trim already happened. Add `pub fn generate_economy(styles, priced)` calling both into one fresh `String`. Guard: `artifact_guard`, `generate_snapshot` (tests/cli_generate.rs) must stay unchanged.

### `src/cli.rs` and `src/run.rs` (Apply)

**Analog:** `Commands::Apply` (cli.rs:60-96) and `cmd_apply` (run.rs:127-198). Keep `--tiers` and `--styles` args verbatim; drop `store`, `base`, `dest`, `reset`; add `--filter <path>`, `--add-economy`, `--remove-economy` (mutually exclusive, clap `conflicts_with`). Dispatch arm pattern (run.rs:28-41). Reuse verbatim:
```rust
if let Some(t) = tiers { eprintln!("note: --tiers {t} — using its cutoffs instead of tiers.json"); paths = paths.with_tiers(t); }
```
and the `match compiled.price_source { Fallback(LiveMissingOrEmpty) => eprintln!("warning: prices.json absent or empty — falling back to ..."), Absent => {...}, Live | Fallback(Forced) => {} }` block (run.rs:160-176), and the closing `println!("Reload it in-game: Options > Game > Item Filter > Refresh");`. Default path error: `dynamic::default_destination().ok_or_else(|| miette::miette!("Cannot resolve the default PoE2 config folder (USERPROFILE is not set). Pass --filter <path>."))`. Replace `InputPaths::conventional()` with `InputPaths::relative_to(&paths::app_root()?)` in apply only (`generate`/`install`/`sync` stay cwd-relative).

### `src/overlay/*` (minimal compiling re-point)

**Analog:** `overlay/apply.rs`: `ApplyOutcome { Written(PathBuf), Failed(String) }`, `success_message(written)` (keep `RELOAD_INSTRUCTION`), `ApplyState`. `guard_save` and its 4 tests are obsolete. Delete `overlay/store.rs` and `pub mod store;` (overlay/mod.rs:12). In `app.rs`: remove `use super::store` (43), `store_path` (101), load in `new()` (166-173), sync dir from `store_path.parent()` (~470-480) -> app root; Apply (484-547) -> `apply_file` with `Op::Refresh`. Leave `edit.rs`, `rule_editor.rs`, `view_model.rs` untouched. Never `expect`/panic on root failure; show in status line.

### `tests/dynamic_filter.rs` (rewrite)

**Analog:** itself. Keep helpers `normalize`, `manifest()`, `workspace(tag)` (timestamped dir under `temp_dir()`, copies `tiers.json`, `styles.json`, `seed-prices.json` as `prices.json`; drop `dynamic_base.json`/`config.json`), `filter_text`, and `economy_section` (find first `#============`). Change `apply` to:
```rust
Command::cargo_bin("ferrisfilter").expect("binary").current_dir(dir)
    .env("FERRISFILTER_HOME", dir).args(["apply", "--filter", "game/dynamic.filter"]).assert()
```
`FERRISFILTER_HOME` is mandatory in every spawned test (exe walk-up would otherwise find the repo `styles.json`). Keep the economy-parity test (text between markers == `generate` economy tail on same inputs). Delete `edit_store`.

## Shared Patterns

### Library returns values, `run.rs` prints
**Source:** `src/dynamic.rs:1-22`, `src/run.rs:1-10`. Apply to `paths`, `section`, `safe_write`, `dynamic`.

### Error types
**Source:** `DynamicError` (dynamic.rs:57-98), thiserror + miette Diagnostic, `String` paths via `.display().to_string()`, typed variants naming 1-based line numbers for marker damage.

### Never create missing directories
**Source:** `write_filter` `is_dir()` guard; `install::cmd_install` guard. Apply to `safe_write` and the new-file `--add-economy` path.

### Default PoE2 path
**Source:** `install::default_dest()` (install.rs:14), via `dynamic::default_destination()`. Do not re-spell the path.

### Test hermeticity
**Source:** `tests/dynamic_filter.rs` header: always a temp dir seeded from committed files, never repo root; mutate an input in the temp copy and assert output changed.

### Byte-stability gate
`cargo test --test cli_generate` after any `emitter.rs`/`compile.rs` edit; `git diff --stat tests/fixtures tests/snapshots` stays empty. Build with `CARGO_TARGET_DIR=C:/Users/jonat/AppData/Local/Temp/ff-target`.

## No Analog Found

| File | Role | Data Flow | Reason |
|------|------|-----------|--------|
| `src/section.rs` marker splice and v0.4 migration | utility | transform | No line-range replace exists in the Phase 16 edit API (`replace_block_lines` is single-block only); build on the `Document::lines()`/`ff_marker()`/`owned()` primitives and the RESEARCH splice shape. Use `Document::set_meta` only for `#ff:name` insertion during migration (skip `^rule \d+$`, only blocks before the banner). |
| Persist-retry with injected persist closure | utility | file-I/O | Nothing retries today; follow RESEARCH Pattern 5 (os errors 5/32, `PersistError.file` reuse). |

## Metadata

**Analog search scope:** `src/`, `src/filter/`, `src/overlay/`, `tests/`
**Files scanned:** about 14 read in full or by range
**Pattern extraction date:** 2026-10-12
