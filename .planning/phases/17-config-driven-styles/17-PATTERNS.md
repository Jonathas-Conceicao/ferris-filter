# Phase 17: Config-Driven Styles - Pattern Map

**Mapped:** 2026-10-12
**Files analyzed:** 18 new/modified (source + data + test harness)
**Analogs found:** 16 / 18

## File Classification

| New/Modified File | Role | Data Flow | Closest Analog | Match Quality |
|-------------------|------|-----------|----------------|---------------|
| `styles.json` (new, repo root) | config (data) | file-I/O | `tiers.json` / `config.json` `styles` object (verbatim copy) | exact |
| `src/models/config.rs` (`StyleSet` replaces `Styles`; `Config.styles` field) | model | transform | existing `Styles` (config.rs:100-143) + `ValidatedTiers` (models/tiers.rs) | exact |
| `src/models/config.rs` or `models/styles.rs` (`StyleError`) | model (error) | request-response | `TierValidationError` (models/tiers.rs:37-90) | exact |
| `src/loader.rs` (`load_styles`, `LoadError::Styles`) | utility (loader) | file-I/O | `load_tiers` + `LoadError::Tiers` (loader.rs:37-40, 104-107) | exact |
| `src/compile.rs` (`InputPaths.styles`, `STYLES_FILE`, `with_styles`, load + rule check in `load_inputs`) | service | file-I/O | `InputPaths.tiers` / `with_tiers` / `TIERS_FILE` (compile.rs:53-102) | exact |
| `src/emitter.rs` (`resolve_style`, `style_for_tier`, `GenerateError::UnknownStyle`) | service | transform | self (match -> map lookup) | exact |
| `src/overlay/edit.rs` (`available_styles`) | utility | transform | self (edit.rs:187) | exact |
| `src/overlay/rule_editor.rs` (`render_style` lifetime) | component | event-driven | self (rule_editor.rs:246) | exact |
| `src/overlay/store.rs` (`load` must also load styles.json) | service | file-I/O | `store::load` (store.rs:60-73) + `app.rs:509-513` `InputPaths::relative_to(store_dir)` | role-match |
| `src/overlay/app.rs` (startup load, ~167) | component | event-driven | self | role-match |
| `src/cli.rs` (`--styles` on Generate and Apply) | config (CLI) | request-response | `--tiers` arg (cli.rs:25-31, 72-78) | exact |
| `src/run.rs` (`cmd_generate`, `cmd_apply`) | controller | request-response | `--tiers` handling (run.rs:60-63, 139-142) | exact |
| `config.json`, `dynamic_base.json` (delete `styles` key) | config | file-I/O | n/a (deletion) | n/a |
| `Cargo.toml` (`indexmap` with `serde`) | config | n/a | existing dependency lines | n/a |
| `CLAUDE.md`, `CHANGELOG.md` | docs | n/a | existing layout table | n/a |
| unit tests in `config.rs`, `emitter.rs`, `edit.rs`, `compile.rs`, `store.rs`, `view_model.rs`, `rule_editor.rs` | test | transform | `base_config()` and `include_str!` loaders (edit.rs:760-781) | role-match |
| `tests/cli_errors.rs`, `cli_generate.rs`, `dynamic_filter.rs`, `rule_editor.rs`, `filter_roundtrip.rs`, `sync_prices.rs` | test | file-I/O | existing copy lists (`for f in ["config.json","tiers.json"]`, cli_generate.rs:33) | exact |
| new `styles.json` error and Hide-style tests | test | request-response | `malformed_config_json_reports_field_path_and_expected_type` (cli_errors.rs:55-90) | role-match |

## Pattern Assignments

### `src/loader.rs` -- `load_styles` and `LoadError::Styles` (utility, file-I/O)

**Analog:** `load_tiers` and `LoadError::Tiers` in the same file.

**Variant pattern** (loader.rs:37-40):
```rust
    /// `tiers.json` parsed but failed completeness/ordering validation.
    #[error(transparent)]
    #[diagnostic(transparent)]
    Tiers(#[from] TierValidationError),
```
Add an identical `Styles(#[from] StyleError)` variant. Import beside line 3: `use crate::models::tiers::{...}`.

**Loader pattern** (loader.rs:104-107):
```rust
pub fn load_tiers(path: &str) -> Result<ValidatedTiers, LoadError> {
    let entries: Vec<TierEntry> = load_json(path)?;
    Ok(ValidatedTiers::try_from(entries)?)
}
```
New: `let set: StyleSet = load_json(path)?; set.validate()?; Ok(set)`. `load_json` (loader.rs:54-72) already yields `JsonLoadError` with dotted path (`S.font` since `StyleSet` is `serde(transparent)`) and span. Keep the `Cannot read` prefix behaviour (comes free from `LoadError::Read`).

---

### `StyleError` (model, error)

**Analog:** `TierValidationError` (models/tiers.rs:37-47).
```rust
#[derive(Debug, Error, Diagnostic)]   // imports: miette::Diagnostic, thiserror::Error (tiers.rs:1-3)
pub enum TierValidationError {
    #[error("tiers.json missing required tier(s): {}", .0.join(", "))]
    #[diagnostic(code(ferrisfilter::tiers::missing))]
    MissingTiers(Vec<String>),
```
Copy: `StyleError::MissingRequired(Vec<String>)` with `code(ferrisfilter::styles::missing)`, plus `UnknownInRule { rule_index, rule_name, style, available }` with `code(ferrisfilter::styles::unknown_in_rule)`, and optionally a shadowed-name variant. Missing computation mirrors tiers.rs:62-71 (iterate `REQUIRED`, filter not present, collect `String`s).

---

### `StyleSet` in `src/models/config.rs` (model, transform)

**Analog (shape being replaced):** `Styles` (config.rs:119-143). Per-style `Style` struct and its `deny_unknown_fields` stay untouched. Replace the struct and delete the FUTURE note (config.rs:110-117). Note `exceptional`/`craftable` were `#[serde(default)] Option<Style>` and become ordinary map entries.

**Config field** (config.rs:430-434) currently `pub styles: Styles`. Change to:
```rust
#[serde(default, skip_serializing)]
pub styles: StyleSet,
```
Config has `#[serde(deny_unknown_fields, rename_all = "camelCase")]`, so keeping the field lets legacy `dynamic.json` / `ssf_*` configs still parse (inline map then overwritten by `styles.json`).

Recommended type (from RESEARCH): `#[derive(Debug, Clone, Default, PartialEq, Deserialize, Serialize)] #[serde(transparent)] pub struct StyleSet(IndexMap<String, Style>);` with `REQUIRED`, `get`, `names`, `is_empty`, `from_pairs`, `validate`. Convention for new optional fields: `serde(default)` + `skip_serializing_if`.

---

### `src/compile.rs` -- `InputPaths` and `load_inputs` (service, file-I/O)

**Analog:** the `tiers` field in the same struct (compile.rs:53-102).

**Struct and constructor** (compile.rs:53-72):
```rust
pub struct InputPaths {
    pub config: PathBuf,
    pub prices: PathBuf,
    pub tiers: PathBuf,
    pub prices_fallback: PathBuf,
    pub tiers_fallback: PathBuf,
}
...
        InputPaths {
            config: root.join(CONFIG_FILE),
            prices: root.join(PRICES_FILE),
            tiers: root.join(TIERS_FILE),
```
Add `pub styles: PathBuf`, a `STYLES_FILE = "styles.json"` const next to the others (const block ends at compile.rs:~40), and `styles: root.join(STYLES_FILE)`. The module doc says `InputPaths` is the only place conventional names appear; do not hardcode `"styles.json"` elsewhere (give `store::load` an accessor such as `InputPaths::relative_to(dir).styles`).

**Override builder** (compile.rs:~85-102, `with_tiers`):
```rust
    pub fn with_tiers(mut self, path: impl Into<PathBuf>) -> Self {
        self.tiers = path.into();
        self
    }
```
Add `with_styles` identically. Update the equality tests at compile.rs:~305, 323 that enumerate all fields.

**Load site** (compile.rs:187-191):
```rust
    let config: Config = loader::load_json(&path_str(&paths.config))?;
```
Becomes `let mut config: Config = ...; config.styles = loader::load_styles(&path_str(&paths.styles))?;`, then the rule-reference check (skip `rule.enabled == Some(false)`, matching emitter semantics at emitter.rs:329-335). Errors flow through existing `CompileError::Load(#[from] LoadError)`. `StyleError::UnknownInRule` needs either a `LoadError::Styles` conversion (`.into()`) or a new `CompileError` arm in the same transparent style (compile.rs:~150-160).

---

### `src/emitter.rs` (service, transform)

**Analog:** itself. Replace both matches with lookups.

**Current `resolve_style`** (emitter.rs:103-135): `Ok(match style { "S" => &styles.s, ... other => return Err(GenerateError::UnknownStyle{ rule_index: idx, rule_name: name.to_string(), style: other.to_string() }) })`. New body per RESEARCH:
```rust
styles.get(style).ok_or_else(|| GenerateError::UnknownStyle {
    rule_index: idx, rule_name: name.to_string(), style: style.to_string(),
})
```
**Current `style_for_tier`** (emitter.rs:247-258) panics on unknown tier (`panic!("Unknown tier '{}' in priced data", other)`); keep the panic via `styles.get(tier).unwrap_or_else(|| panic!(...))`. Safe because `validate()` guarantees S-F.

**Error message** (emitter.rs:21-24) hardcodes the vocabulary "valid styles: S, A, ... craftable (craftable must be defined...)". Make it data-driven (add a `valid: String` field) or drop the list. Grep tests for `valid styles` first. Keep `generate_rejects_unknown_style_name` (emitter.rs:~765) passing. Import change at emitter.rs:11-13: `Styles` -> `StyleSet`.

Do NOT touch `emit_style_actions` (emitter.rs:~192): fixed action order is the byte-identity guarantee. Callers at emitter.rs:338, 457, 523 pass `&config.styles` unchanged. Show/Hide comes from `style.action` (emitter.rs:~340), so "style action overrides block action" and "Hide styles carry visuals" are already true; lock with tests only.

---

### `src/overlay/edit.rs` -- `available_styles` (utility, transform)

**Current** (edit.rs:187-199): hardcoded `vec!["S",...,"kite"]` plus optional pushes for `exceptional`/`craftable`, returns `Vec<&'static str>`.
**New:** `pub fn available_styles(styles: &StyleSet) -> Vec<&str> { styles.names().collect() }` (file order). Caller `rule_editor.rs:246` (`render_style`) only needs the lifetime change.

**Tests to adjust** (edit.rs:766-781): `dynamic_base_json_offers_exactly_twelve_styles` stays valid once `base_config()` populates `config.styles` from `styles.json`. `a_styles_struct_with_neither_optional_style_offers_eleven` sets `styles.exceptional = None; styles.craftable = None;` and must become a `shift_remove("exceptional")` style operation (needs a small test-visible mutator on `StyleSet`) -> 11.

---

### `src/overlay/store.rs` and `app.rs` (service, file-I/O)

**Analog:** `store::load` (store.rs:60-73):
```rust
    let config: Config = loader::load_json(store_str).map_err(|e| e.to_string())?;
    Ok(StoreView { config, seeded: ..., store_path: store.to_path_buf() })
```
Add, after loading: `config.styles = loader::load_styles(<store.parent()/styles.json>)...map_err(|e| e.to_string())?`. Resolve the directory exactly as app.rs:509-513 does:
```rust
let store_dir = self.store_path.parent().unwrap_or_else(|| std::path::Path::new("."));
let paths = InputPaths::relative_to(store_dir);
```
`store::save` needs no change because `skip_serializing` keeps styles out of `dynamic.json`. Round-trip test in store.rs (~181, 259) loads via `include_str!`, so it needs the styles helper.

---

### `src/cli.rs` and `src/run.rs` -- `--styles` (controller, request-response)

**Analog:** `--tiers` (cli.rs:25-31 for Generate, cli.rs:72-78 for Apply):
```rust
        #[arg(long)]
        tiers: Option<String>,
```
Add `styles: Option<String>` with doc comment "Styles file to use instead of styles.json".

**run.rs** (lines 60-63 in `cmd_generate`, 139-142 in `cmd_apply`):
```rust
    if let Some(t) = tiers {
        eprintln!("note: --tiers {t} — using its cutoffs instead of tiers.json");
        paths = paths.with_tiers(t);
    }
```
Copy with `--styles`/`with_styles`. Extend the destructuring at run.rs:29-37 and the function signatures (run.rs:50-53, ~123). Optional per CONTEXT; RESEARCH recommends adding.

---

### Test harness updates (test, file-I/O)

**Copy lists** (cli_generate.rs:33): `for f in ["config.json", "tiers.json"] { fs::copy(manifest.join(f), dir.join(f))... }` -> add `"styles.json"`. Same for cli_generate.rs:68, 196-220, 462-480; `dynamic_filter.rs:31`, `rule_editor.rs:37`, `filter_roundtrip.rs:165` (`["dynamic_base.json","tiers.json","config.json"]`); `sync_prices.rs:639-650`; `cli_errors.rs` temp-dir builders; in-source `compile.rs` `temp_inputs` calls (~369-473).

**`cli_errors.rs:55-90`** `malformed_config_json_reports_field_path_and_expected_type`: retarget. Mutate a `styles.json` string (`replacen("\"font\": 37", "\"font\": \"BROKEN\"", 1)`), write it via `make_temp_dir(..., [("config.json",..),("tiers.json",..),("styles.json",&bad)])`, and assert `contains("styles.json")`, `contains("S.font")`, `contains("expected u8")`. Update the file doc comment ("must keep passing unmodified") to note this single legitimate change.

**In-source Config loaders** (about 15 sites using `serde_json::from_str::<Config>(include_str!(".../config.json" | "dynamic_base.json"))`: emitter.rs:~597, edit.rs:~588, rule_editor.rs:~1039, store.rs:~181 and ~259, view_model.rs:~268; config.rs `CONFIG_JSON`): each needs `config.styles = StyleSet::from_default_json()` (or equivalent shared helper using `include_str!` of `styles.json`). Grep `include_str!` for `config.json` / `dynamic_base.json` when planning.

**Config-model tests to rewrite:** `config_styles_named_fields` (config.rs:~477), `config_rejects_unknown_key` (config.rs:~506, fix stale comment), `config_rejects_bad_enum_color` (config.rs:~526; retarget to styles.json since `Pink` no longer lives in config.json), exceptional-optional regression (config.rs:~911-932).

## Shared Patterns

### Typed error wrapped transparently
**Source:** `src/loader.rs:37-40`, `src/compile.rs` `CompileError::Load`
**Apply to:** `StyleError` wiring in loader and compile
```rust
    #[error(transparent)]
    #[diagnostic(transparent)]
    Tiers(#[from] TierValidationError),
```

### Conventional-name path resolution
**Source:** `src/compile.rs:53-102`
**Apply to:** CLI `generate`/`apply` (use `InputPaths::conventional()`, never `relative_to(".")`, per compile.rs doc), overlay (use `InputPaths::relative_to(store_dir)`).

### Load-time validation before use
**Source:** `load_tiers` calls `ValidatedTiers::try_from` inside the loader (loader.rs:104-107)
**Apply to:** `load_styles` must always call `validate()`, because `style_for_tier` panics if S-F are absent.

### Byte-identical discipline
**Source:** CONTEXT / RESEARCH Pitfall 4
**Apply to:** every commit. Do not edit `emit_style_actions`, do not regenerate `tests/fixtures/seed-universal.filter`, run `cargo test --test cli_generate` (`artifact_guard`, `generate_snapshot`) after each commit. Commit order: (1) add `styles.json` + `StyleSet` + equality test (styles.json == config.json styles == dynamic_base.json styles == old `Styles`) while old struct exists; (2) switch consumers; (3) delete `Styles` and the two JSON `styles` keys. Use `CARGO_TARGET_DIR=C:/Users/jonat/AppData/Local/Temp/ff-target`.

## No Analog Found

| File | Role | Data Flow | Reason |
|------|------|-----------|--------|
| `StyleSet` itself (ordered map newtype with `serde(transparent)`) | model | transform | No `IndexMap` or map newtype in repo; the old `Styles` is a named-field struct. Use RESEARCH "Recommended shape". |
| Rule-reference check (every enabled rule's style exists) | service | validation | No existing cross-reference validation; the emitter's per-rule `UnknownStyle` is the nearest behaviour. Use the RESEARCH snippet in `load_inputs`. |
| Optional duplicate-key detection / case-insensitive reserved-name check | model | validation | Nothing comparable; optional per RESEARCH (PITFALLS section 23). |

## Metadata

**Analog search scope:** `src/` (loader, compile, emitter, models, overlay, cli, run), `tests/`
**Files scanned:** about 14 read/sectioned
**Pattern extraction date:** 2026-10-12
