# Phase 17: Config-Driven Styles - Research

**Researched:** 2026-10-12
**Domain:** Rust refactor (serde, indexmap, miette) of a deterministic filter compiler
**Confidence:** HIGH (all findings come from reading this repo's code; one dependency detail is MEDIUM)

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions
**styles.json shape**
- A JSON object mapping style name -> style, using the **same per-style fields as today's `Style`**. File order is preserved through an ordered map (`indexmap`, a small, standard dependency, accepted). It is committed at the repo root. Moving it to a per-user config location is a future item; installation isn't a concern yet.
- Initial content is exactly today's `config.json` styles (S, A, B, C, D, E, F, Recycle, unchanged, hidden, kite, exceptional), so output is unchanged.

**One source of truth**
- `config.json` **loses its `styles` section**. Both `generate` (static path) and the dynamic path read `styles.json`. `artifact_guard` and `generate_snapshot` prove `generate` output is byte-identical, with no fixture re-baselining.
- Before deleting the hardcoded `Styles` struct, assert that the loaded default `styles.json` equals the old hardcoded/deserialized values. Emission order of style actions stays fixed in the emitter and never depends on JSON key order.
- Untracked experimental configs (`ssf_*_config.json`, which have inline styles such as `craftable`) are not supported migration targets. A clear error, or an optional `--styles` path, is fine (planner's discretion).

**Required styles and validation**
- **Required, undeletable:** S, A, B, C, D, E, F (the economy emitter needs them), plus `unchanged` and `hidden`. Everything else is optional and user-definable.
- A rule naming an unknown style, or an invalid `styles.json` (missing required style, bad colour, bad enum), is a clear load-time error naming the style or field (STYLE-02). Adding a new style requires no code change (STYLE-01).

**Show/Hide and styles**
- A style keeps its **action** (Show/Hide). A block may have its own Show/Hide, but **when a style is applied, the style's action overrides it** (author decision).
- **Hide styles can carry full visuals.** A Hide-action style may set any action (colours, font, sounds, minimap, beam, drop sound), since hidden items are revealed with Alt and the author wants to style them. Today's `hidden` (font 21 + DisableDropSound) is one such style, and the model must not restrict Hide styles to a subset.

### Claude's Discretion
- Type name (`StyleSet` suggested), a `from_pairs`-style helper so the many test literals don't churn, error variants, and loader placement in `compile::`.

### Deferred Ideas (OUT OF SCOPE)
- Style rename (future requirement).
- Per-user `styles.json` location (future requirement).
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| STYLE-01 | Named styles live in committed repo-root `styles.json`; adding one needs no code change | `StyleSet(IndexMap<String, Style>)` + name lookup replaces the two hardcoded `match` sites (Use-Site Map) |
| STYLE-02 | Unknown style in a rule, or invalid `styles.json`, gives a clear load-time error | New `loader::load_styles` + `StyleError` diagnostic + a rule-reference check inside `compile::load_inputs` (Validation section) |
| STYLE-04 | `generate` output unchanged; `seed-universal.filter` byte-identical | Side-by-side equality test, fixed emitter action order, test-harness updates (Pitfalls, Validation) |
</phase_requirements>

## Summary

The styles vocabulary is hardcoded at exactly two code sites: the `Styles` struct (`src/models/config.rs:119`, with a FUTURE note at line 110) and two `match` functions in `src/emitter.rs` (`resolve_style` at line 103 with a FUTURE note at line 99, and `style_for_tier` at line 247). Both are plain name-to-`&Style` lookups, so a map-backed `StyleSet` is a near-mechanical swap. `emit_style_actions` (emitter.rs:192) already emits in a fixed order independent of any key order, so the byte-identical guarantee is cheap to keep. The `Style` struct itself needs no change. Rules already pick `Show`/`Hide` from `style.action` (emitter.rs:340), and Hide styles already receive `emit_style_actions`, so "style action overrides" and "Hide styles can carry visuals" are already true behavior. Phase 17 should lock them with tests, not new code.

The real cost is not the emitter. It is the plumbing around moving styles out of `Config`: `Config` is deserialized with `deny_unknown_fields` from `config.json`, `dynamic_base.json`, and the user's `dynamic.json`, all of which currently carry a `styles` key. At least 15 in-source unit tests build a `Config` via `include_str!` + `serde_json::from_str`. About 6 integration tests copy `config.json` into temp dirs. One test (`cli_errors.rs`) asserts the error path string `styles.S.font`. The planner must budget a test-harness wave, not just a src wave.

`indexmap` 2.14.0 is in `Cargo.lock` with `serde` in its dependency list, but only as a transitive dependency. It is not in `Cargo.toml`, so it must be added as a direct dependency with `features = ["serde"]`.

**Primary recommendation:** Add `StyleSet(IndexMap<String, Style>)` in `models/config.rs`. Keep a `styles: StyleSet` field on `Config`, but mark it `#[serde(default, skip_serializing)]` and have the loaders fill it from `styles.json`. This keeps every emitter and overlay call site (`config.styles`) unchanged and keeps legacy `dynamic.json` files parseable. Do it in two commits: first add `styles.json` and the side-by-side equality test while the old struct still exists, then delete the struct.

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| Style data (definitions) | Data file `styles.json` | — | Single source of truth, committed |
| Parse and validate styles.json | Library `loader` / `models::config` | — | Load-time errors; no I/O in emitter |
| Name resolution for rules | Library `emitter` | `compile::load_inputs` (pre-check) | Emitter keeps hard error; loader gives the early, clear error |
| Economy tier styles (S-F) | Library `emitter` via `pricer` tier letters | — | Requires S-F to exist, hence the required set |
| Style picker list | Overlay `edit::available_styles` | — | Reads `StyleSet` keys in file order |
| Path resolution for styles.json | `compile::InputPaths` | CLI flag | The one place conventional names live (module doc, compile.rs:51) |

## Standard Stack

### Core
| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| indexmap | 2.14.0 (in Cargo.lock) | Insertion-ordered `String -> Style` map | Locked by CONTEXT; its serde feature is already enabled transitively |
| serde / serde_json / serde_path_to_error | existing | Parse and field-path errors | Already used by `loader::load_json` |
| miette + thiserror | existing | Diagnostic error type | Same pattern as `TierValidationError` |

**Installation:** add `indexmap = { version = "2", features = ["serde"] }` to `[dependencies]` in `Cargo.toml`. Dependencies are currently `serde`, `serde_json`, `serde_path_to_error`, `miette`, `thiserror`, `reqwest`, etc., and no `indexmap`. [VERIFIED: Cargo.toml read; Cargo.lock lines 2275-2283 show indexmap 2.14.0 with `serde` in its deps]

**Version verification:** run `cargo tree -i indexmap` and `cargo add indexmap --features serde` (offline-safe because it is already locked). [ASSUMED] that the existing transitive resolve will unify onto 2.14.0 without a lockfile bump; confirm via `git diff Cargo.lock` after the build.

## Package Legitimacy Audit

| Package | Registry | Age | Downloads | Source Repo | slopcheck | Disposition |
|---------|----------|-----|-----------|-------------|-----------|-------------|
| indexmap | crates.io | long-established | very high | github.com/indexmap-rs/indexmap | not run | Approved by user decision in CONTEXT.md; already in Cargo.lock as a transitive dependency |

slopcheck was not run. The package is user-approved and already present in the lockfile, so risk is minimal. Packages removed: none. Packages flagged: none.

## Use-Site Map (every place `Styles` / `config.styles` is touched)

| Site | File:line | Change |
|------|-----------|--------|
| `Styles` struct (+ FUTURE note) | `src/models/config.rs:100-143` | Replace with `StyleSet`; delete FUTURE note |
| `Config.styles: Styles` | `src/models/config.rs:434` | Type -> `StyleSet`, `#[serde(default, skip_serializing)]` (see Architecture) |
| `resolve_style` (+ FUTURE note) | `src/emitter.rs:94-135` | `styles.get(style).ok_or(UnknownStyle{..})`; delete FUTURE note |
| `style_for_tier` | `src/emitter.rs:247-258` | `styles.get(tier).unwrap_or_else(panic)`; S-F guaranteed by load validation |
| Callers | `emitter.rs:338, 457, 523` | Unchanged (`&config.styles`) |
| `GenerateError::UnknownStyle` message | `src/emitter.rs:21-24` | Hardcodes "valid styles: S, A, ... craftable". Make it list actual keys (add `valid: String` field) or drop the list |
| `use ... Styles` import | `src/emitter.rs:12` | -> `StyleSet` |
| `available_styles(&Styles) -> Vec<&'static str>` | `src/overlay/edit.rs:187` | Return `Vec<&str>` of keys in file order; delete hardcoded names + optional pushes |
| Caller | `src/overlay/rule_editor.rs:246` (`render_style`) | Lifetime change only; loop already iterates names |
| view_model doc comment | `src/overlay/view_model.rs:31` | Comment only; no code use of `Styles` |
| `edit.rs` tests | `edit.rs:766-781` | "exactly twelve" stays valid if styles.json has 12; the `exceptional = None; craftable = None` test must become `shift_remove("exceptional")` -> 11 |
| Pricer | `src/pricer.rs`, `src/sync/*` | NO use of `Styles` (grepped). Pricer only emits tier letters; the emitter resolves them |
| `compile::load_inputs` | `src/compile.rs:187-191` | Add `styles` path, load styles, attach to `Config`, run the rule check |
| `InputPaths` | `src/compile.rs:53-85` | Add `pub styles: PathBuf` + `STYLES_FILE = "styles.json"`; update `relative_to`, tests at compile.rs:305, 323 |
| `dynamic::compile_store` | `src/dynamic.rs:146` | Gets styles for free through `load_inputs`; `paths.clone().with_config(store)` keeps `styles` |
| Overlay config load | `src/overlay/store.rs:60-73` (`store::load`) + `app.rs:167` | Overlay holds a `Config` in memory for the picker. It must also load `styles.json` (from `store.parent()` joined with `styles.json`, matching `app.rs:509-513`'s `InputPaths::relative_to(store_dir)`) and set `config.styles` |
| `store::save` | `src/overlay/store.rs:29` | `skip_serializing` on `styles` keeps `dynamic.json` clean of styles |
| CLI | `src/cli.rs:14-` (`Generate`), `src/run.rs:50-76` (`cmd_generate`), `run.rs:143` (`cmd_apply`) | Optional `--styles <path>` on `generate` (and `apply`), via a new `InputPaths::with_styles`, mirroring `with_tiers` |
| `config.json` | repo root | Delete `styles` key. Rules still reference `Recycle`, `kite`, `exceptional`, etc. by name |
| `dynamic_base.json` | repo root | Delete `styles` key too (identical to config.json's, verified) |
| `CLAUDE.md` | repo root | Update the layout table: add `styles.json`, remove styles text from the `config.json` row, update "Named styles" |

[VERIFIED: grep and file reads in this session]

### Value equality of the sources
`config.json` and `dynamic_base.json` styles objects are byte-identical as JSON (key order and values), 12 keys: S,A,B,C,D,E,F,Recycle,unchanged,hidden,kite,exceptional. So `styles.json` is exactly that object (the same top-level shape, a map, not wrapped). The user's local `dynamic.json` additionally has `craftable` and differs. The `ssf_*_config.json` files have `craftable` and `unidentified` and no `kite` or `exceptional`. [VERIFIED: node comparison]

## Architecture Patterns

### Recommended shape
```rust
// models/config.rs
#[derive(Debug, Clone, Default, PartialEq, Deserialize, Serialize)]
#[serde(transparent)]
pub struct StyleSet(IndexMap<String, Style>);   // Style keeps deny_unknown_fields

impl StyleSet {
    pub const REQUIRED: [&'static str; 9] = ["S","A","B","C","D","E","F","unchanged","hidden"];
    pub fn get(&self, name: &str) -> Option<&Style> { self.0.get(name) }
    pub fn names(&self) -> impl Iterator<Item = &str> { self.0.keys().map(String::as_str) }
    pub fn is_empty(&self) -> bool { self.0.is_empty() }
    pub fn from_pairs<I: IntoIterator<Item = (&'static str, Style)>>(i: I) -> Self { .. } // test literals
    pub fn validate(&self) -> Result<(), StyleError> { /* required present */ }
}
```
`Config`:
```rust
#[serde(default, skip_serializing)]
pub styles: StyleSet,
```
Rationale for keeping the field on `Config` instead of moving styles into `CompileInputs`:
- `emitter::generate_as(&Config, ..)` and every `config.styles` read keep working untouched. `PartialEq` and round-trip tests still compile.
- `deny_unknown_fields` on `Config` would otherwise reject the user's existing `dynamic.json` and old checkouts' configs the moment `styles` disappears from the struct. With the field kept, a legacy file parses, and its inline map is then overwritten by `styles.json` at load.
- `skip_serializing` means `store::save` no longer writes styles, so `dynamic.json` converges.

Trade-off to surface to the user: a legacy `dynamic.json` inline `craftable` style would be silently dropped (the overlay's own `craftable` is only in the user's local file; `dynamic_base.json` never had it). A rule using `craftable` would then fail with the clear unknown-style error. Recommend: when the file carries a non-empty inline `styles` that differs from the loaded `styles.json`, ignore it silently (documented), since CONTEXT says one source of truth. This is a planner/user call; see Open Questions.

### Load flow
```
styles.json --loader::load_styles--> StyleSet --validate()--> required S..F,unchanged,hidden present
config/dynamic.json --load_json--> Config  (styles := loaded StyleSet)
          |
          v   check every enabled rule.style in StyleSet (STYLE-02)  -> StyleError::UnknownInRule{rule_index,name,style,available}
          v
CompileInputs{config,priced,..} --emitter::generate_as--> filter text (action order fixed in emit_style_actions)
```

### Where validation lives
- Parse errors (bad colour, bad enum, unknown field in a style, wrong type) come free from `loader::load_json::<StyleSet>`, which already yields `JsonLoadError` with a dotted field path (now `S.font`, not `styles.S.font`) and a source span. Because `StyleSet` is `transparent` over a map, the path begins with the style name. [VERIFIED: loader.rs:54-72]
- Missing required styles: new `StyleError::MissingRequired { names: Vec<String> }`. Model on `TierValidationError`; wrap in a new `LoadError::Styles(#[from] StyleError)` variant with `#[diagnostic(transparent)]` exactly like `LoadError::Tiers` (loader.rs:37-40). Add `loader::load_styles(path) -> Result<StyleSet, LoadError>` mirroring `load_tiers` (loader.rs:104).
- Unknown style in a rule: do this in `compile::load_inputs` right after loading, iterating `config.rules` and skipping `enabled == Some(false)` rules, so it matches the emitter, which deliberately ignores disabled rules (emitter.rs:329-335 comment). Report rule index, rule name or `rule N`, offending style, and the available names. Keep the emitter's `GenerateError::UnknownStyle` as a defence-in-depth (the existing unit test `generate_rejects_unknown_style_name`, emitter.rs:765, still needs it).
- Required: S-F, `unchanged`, `hidden`. Note `kite`, `Recycle`, and `exceptional` are NOT required by CONTEXT, but `config.json`'s rules reference them, so deleting them from `styles.json` yields the rule-reference error, which is correct.
- PITFALLS §23 (reserved-name shadowing, case-insensitive): reject, in `validate()`, any key that equals a required or built-in name case-insensitively but not exactly (e.g. `hidden2` is fine, `Hidden` or `s` is not). Cheap and consistent with CONTEXT's "undeletable" intent. [CITED: .planning/research/PITFALLS.md #23]
- `serde_json` into `IndexMap` silently keeps the LAST duplicate key. A duplicate style name in the file would be swallowed. Cheap to detect with a custom visitor, otherwise note it as an accepted gap. Mark as optional.

### Path resolution
- CLI `generate`: `InputPaths::conventional()` yields bare relative `styles.json` in the CWD, exactly like `config.json`. Add `--styles <path>` (optional, default `styles.json`) via `with_styles`, parallel to `--config`/`--tiers`. This also covers the `ssf_*_config.json` case if the user passes a custom path. Without `--styles`, those configs' inline styles are ignored (not an error), because the field is still accepted by serde.
- CLI `apply` and the overlay: `InputPaths::relative_to(store_dir)` (app.rs:513) already points `styles` at the store's directory. So `styles.json` is found beside `dynamic.json` with no new path logic. `store::load` needs the same dir for its own load.
- Do not hardcode `"styles.json"` anywhere outside `compile.rs` (the module doc says `InputPaths` is the only place conventional names appear). Export a `pub const` or an accessor for `store::load`.
- Error message path: bare `styles.json` for the CLI, as with `config.json`; keep `conventional()` rather than `relative_to(".")` (compile.rs:62-71 warns this changes rendered messages).

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Ordered map with serde | A `Vec<(String, Style)>` + linear lookup + custom Deserialize | `indexmap::IndexMap` with `serde` | Locked decision; handles order and O(1) lookup |
| Field-path JSON errors | Manual error text | `loader::load_json` | Already yields dotted path + span, asserted by tests |
| Diagnostic wiring | A new error rendering mechanism | `thiserror` + `miette::Diagnostic` with `transparent` | Same as `LoadError::Tiers` |
| Colour/enum validation | Custom checks | Existing `FilterColor`, `MinimapShape`, `StyleAction` enums plus `deny_unknown_fields` | Already reject "Mauve" etc. |

## Common Pitfalls

### Pitfall 1: Test harnesses that build a Config now have no styles
**What goes wrong:** about 15 unit tests do `serde_json::from_str::<Config>(include_str!(config.json | dynamic_base.json))` (emitter.rs:597, edit.rs:588, rule_editor.rs:1039, store.rs:181 and 259, view_model.rs:268, plus emitter's many `load_inputs()` callers). With `#[serde(default)]`, they compile and parse but get an EMPTY `StyleSet`, so emission fails with `UnknownStyle` or the picker shows zero styles.
**How to avoid:** add one shared helper (e.g. `pub fn StyleSet::from_default_json()` using `include_str!(".../styles.json")`, or a `#[cfg(test)]` helper) and have each of those loaders set `config.styles = ...`. This is the "from_pairs-style helper so test literals don't churn" in CONTEXT. Search for `include_str!` of `config.json`/`dynamic_base.json` to find every site.
**Warning signs:** `UnknownStyle` in tests that previously passed; `available_styles` returning 0.

### Pitfall 2: Integration tests copy config.json into temp dirs without styles.json
**What goes wrong:** the CLI is spawned with `current_dir(&dir)`; with no `styles.json` it fails "Cannot read `styles.json`".
**Sites:** `tests/cli_generate.rs:33` and `:68` (file lists), `:196-220` (`--config custom-config.json` test, needs styles copied too), `:462-480` (`cap_is_rule_driven`, check its copy list); `tests/dynamic_filter.rs:31`, `tests/rule_editor.rs:37`, `tests/filter_roundtrip.rs:165` (all `["dynamic_base.json","tiers.json","config.json"]`); `tests/sync_prices.rs:639-650`; `tests/cli_errors.rs` (`valid_config()`, temp-dir builders); in-source `compile.rs` `temp_inputs` calls (compile.rs:369-473, about 8 call sites).
**How to avoid:** add `styles.json` to each copy list. Do this in the same commit that makes `styles.json` mandatory.

### Pitfall 3: cli_errors path assertion
**What goes wrong:** `tests/cli_errors.rs:55-90` mutates `config.json` (`"font": 37` -> `"BROKEN"`, first occurrence = style S) and asserts stderr contains `config.json`, `styles.S.font`, `expected u8`. After the move, that font lives in `styles.json`, the path is `S.font`, and the file name is `styles.json`. The assertion must be rewritten to mutate `styles.json` and assert `styles.json`, `S.font`, `expected u8`. Note the file's own doc comment says "must keep passing unmodified" for this suite; this one test legitimately changes, so call that out in the plan.
Also verify `config.json` has no other `"font": 37` before relying on `replacen`; after the move, replacing in `config.json` hits nothing.

### Pitfall 4: Byte-identical output (PITFALLS §8)
**What goes wrong:** emission drifts if anything iterates the map to emit style lines, or `skip_serializing_if` / key reorder changes serde defaults.
**How to avoid:** `emit_style_actions` is the sole emitter of style lines, in fixed order (beam, minimap, text, background, border, font, sound, drop sound); do not touch it. Only lookup changes. Do not regenerate `tests/fixtures/seed-universal.filter`. Run `cargo test --test cli_generate` (contains `artifact_guard` at :314 and `generate_snapshot` at :301) after each commit. Also check `tests/snapshots/` (insta) stays untouched.
**Ordering of commits (PITFALLS §8c/d):** (1) add `styles.json` + `StyleSet` + a test asserting `StyleSet` parsed from `styles.json` equals the old `Styles` converted to a `StyleSet` and also equals the styles currently in `config.json` and `dynamic_base.json` (all three equal); (2) switch consumers; (3) delete the struct and the config.json/dynamic_base.json `styles` keys. The equality test is the one that must run before the struct disappears; once it is gone, assert instead that `styles.json` has the 12 expected keys in order and spot-check values.

### Pitfall 5: Legacy inline styles
`config.json` hand-edited by users, the git-ignored `dynamic.json`, and `ssf_*_config.json` keep an inline `styles` object. With `deny_unknown_fields` and the field retained, they parse; their styles are ignored. Without the retained field they fail with an unknown-field error. If the user prefers loudness: emit a `LoadError`-style warning from `load_inputs` when an inline `styles` is non-empty. Silent drop of an extra style is the surprising case (user's `dynamic.json` has `craftable`).

### Pitfall 6: Economy emitter panics on missing tier
`style_for_tier` panics on unknown tier. After the change, the panic path is reachable only if `validate()` is skipped. Ensure `validate()` always runs inside `load_styles` (and in any constructor used by tests).

### Pitfall 7: GenerateError message lists a fixed vocabulary
`emitter.rs:21-24` hardcodes "valid styles: S, A, ... craftable (craftable must be defined...)". It becomes wrong the moment styles are data. Update it to list the actual names. Check whether any test asserts that string (grep `valid styles` in tests; not found in the greps run, verify).

### Pitfall 8: Overlay picker order and lifetimes
`available_styles` returns `Vec<&'static str>` today; with a borrowed map it must return `Vec<&str>` tied to the `StyleSet`. `render_style` (rule_editor.rs:246) borrows `config` and then mutably `rule` in the same function; the `config` borrow is shared, so this is fine, but confirm it compiles because `rule` is a separate `&mut`.

### Pitfall 9: Windows build lock
`ff.exe` can lock `target\debug`. Use `CARGO_TARGET_DIR=C:/Users/jonat/AppData/Local/Temp/ff-target` for every cargo command.

## Code Examples

### Lookup replacing the match (emitter.rs)
```rust
fn resolve_style<'a>(styles: &'a StyleSet, style: &str, idx: usize, name: &str)
    -> Result<&'a Style, GenerateError>
{
    styles.get(style).ok_or_else(|| GenerateError::UnknownStyle {
        rule_index: idx, rule_name: name.to_string(), style: style.to_string(),
        // optionally: valid: styles.names().collect::<Vec<_>>().join(", ")
    })
}

fn style_for_tier<'a>(styles: &'a StyleSet, tier: &str) -> &'a Style {
    styles.get(tier).unwrap_or_else(|| panic!("Unknown tier '{}' in priced data", tier))
}
```

### Loader mirroring load_tiers (loader.rs)
```rust
pub fn load_styles(path: &str) -> Result<StyleSet, LoadError> {
    let set: StyleSet = load_json(path)?;
    set.validate()?;          // StyleError -> LoadError::Styles via #[from]
    Ok(set)
}
```

### Rule reference check (compile.rs, after loading config + styles)
```rust
for (idx, rule) in config.rules.as_deref().unwrap_or(&[]).iter().enumerate() {
    if rule.enabled == Some(false) { continue; }   // matches emitter semantics
    if styles.get(&rule.style).is_none() {
        return Err(StyleError::UnknownInRule { rule_index: idx, rule_name: rule.name.clone(),
            style: rule.style.clone(), available: styles.names().map(str::to_owned).collect() }.into());
    }
}
```
Place this in `compile::load_inputs` (wrapped by `CompileError::Load`/a new arm), so it runs for `generate`, `apply`, and the overlay Apply. The overlay can also run the same check on the in-memory rules before writing, but only the Apply path is required by STYLE-02.

## Behaviors to lock with tests (already true in code, must stay true)
- A Hide style with colours/font/sound/minimap/beam/drop-sound emits `Hide # name` followed by those lines (emitter.rs:340-343 header, `emit_style_actions` unconditional). Add a unit test with a synthetic Hide style carrying everything.
- A new style added to `styles.json` and used by a rule emits with no code change (success criterion 1): test by building a `StyleSet` from `styles.json` plus an extra entry, or an integration test with a temp-dir `styles.json` containing e.g. `"craftable"` and a rule using it.
- Unknown style in an enabled rule -> load-time error naming rule and style; disabled rule referencing unknown style -> no error (existing semantic).
- `styles.json` missing `E` -> error naming `E`. Bad colour `"Mauve"` -> path-bearing parse error (adapt `config_rejects_bad_enum_color`, config.rs:526, which currently replaces `"Pink"` in `config.json`; Pink also appears in rules/other fields? After the move `config.json` has no Pink, so retarget to `styles.json`).
- Extra field in a style (typo `"fnot"`) is rejected by `deny_unknown_fields` (already on `Style`).
- Config tests to rewrite: `config_styles_named_fields` (config.rs:477, uses `.s/.a/...` fields), `config_rejects_unknown_key` (config.rs:506; its `"styles": {}` still parses now, so its reason for erroring is the `minimip` key plus `alwaysShow` etc.; fix the stale comment), the `exceptional`-optional regression test at config.rs:911-932 (`Styles` omitting `exceptional` parses) — replace with a `StyleSet`-based test.
- Test env note: in `config.rs` tests, `CONFIG_JSON` is `include_str!` of `config.json`; it needs the styles merged for any test that emits.

## State of the Art / Existing context
- Style `exceptional` already exists in both `config.json` and `dynamic_base.json` and is optional in the old struct (`Option<Style>`, config.rs:136-137). In the new model it is just another map entry. `craftable` is optional in the old struct and absent from the committed files; it is NOT in the initial `styles.json` (CONTEXT lists exactly 12 styles).
- `kite` was renamed from `unidentified` on 2026-09-07 (CLAUDE.md); `ssf_*` configs still use the old name, which is another reason they are unsupported migration targets.
- Old `Styles.kite` was a required field; under the new rules it is optional, but `config.json` rules reference it, so a missing `kite` yields the rule-reference error.

## Runtime State Inventory

| Category | Items Found | Action Required |
|----------|-------------|------------------|
| Stored data | `dynamic.json` (git-ignored, user state) has an inline `styles` object including `craftable`; `store::save` rewrote it on every Apply | Field retained with `skip_serializing`; first Apply after the change drops the inline map. A rule using `craftable` would then error. Tell the user |
| Live service config | None — verified, no external service references style names | none |
| OS-registered state | None — verified | none |
| Secrets/env vars | None — verified | none |
| Build artifacts | None. `target/` rebuilds; use the alternate `CARGO_TARGET_DIR` | none |

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| cargo/rustc | build, tests | assumed present (project builds) | — | — |
| node | one-off JSON comparison/generation of `styles.json` | yes (used in this session) | — | write by copy-paste |
| python | — | no (not installed) | — | use node |

## Validation Architecture

`workflow.nyquist_validation` is `false` in `.planning/config.json`, so no formal Validation Architecture section is required. Practical commands:
- Quick: `CARGO_TARGET_DIR=C:/Users/jonat/AppData/Local/Temp/ff-target cargo test --lib`
- Byte guarantee: `... cargo test --test cli_generate` (contains `artifact_guard`, `generate_snapshot`) after every commit
- Full: `... cargo test`
Wave 0 gaps: none, existing framework (cargo test, assert_cmd, insta) covers everything.

## Security Domain

Local, offline-config tool; `security_enforcement` is not disabled but the attack surface is small.
| ASVS Category | Applies | Standard Control |
|---------------|---------|-----------------|
| V5 Input Validation | yes | serde typed deserialization with `deny_unknown_fields`, closed enums, `validate()`; `sound.custom` is a free string emitted into the filter (`CustomAlertSound "{}"`, emitter.rs:228). A name containing `"` or a newline in `styles.json` would inject filter lines. This was already true for config styles; mention as a low-severity gap and optionally reject `"`/control characters in `sound.custom` during `validate()` |
| Others | no | n/a |

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | Adding `indexmap = { version = "2", features = ["serde"] }` unifies with the locked 2.14.0 without a lockfile churn | Standard Stack | Minor Cargo.lock diff |
| A2 | Silently ignoring a legacy inline `styles` section is acceptable (vs warn or error) | Architecture | User loses `craftable` from local dynamic.json unnoticed; decision for user |
| A3 | Case-insensitive shadow check on required names is wanted | Validation | Over-strict rejection of a style like `s`; PITFALLS §23 recommends it |

## Open Questions (RESOLVED)

1. **Legacy inline `styles` handling**
   - Known: the user's `dynamic.json` carries `craftable`; `ssf_*` configs carry inline styles.
   - Unclear: silent ignore vs warning vs hard error.
   - Recommendation: silent ignore plus a one-line note in the CLAUDE.md/CHANGELOG; hard error is more faithful to "clear error" but breaks the overlay's existing store on first launch.
   - **RESOLVED (OD-1):** hard load error naming the file, overriding the silent-ignore recommendation. Phase 18 removes `dynamic.json` anyway.
2. **Should `--styles` be added?** CONTEXT leaves it to the planner. Recommendation: yes on `generate` and `apply` — one `with_styles` method and one clap arg; it also removes the need to say "unsupported" for ssf configs. **RESOLVED (OD-2):** added on `generate` and `apply`.
3. **Duplicate keys in styles.json** are swallowed by `IndexMap` deserialization. Recommendation: accept the gap unless a custom visitor is cheap; note in the plan. **RESOLVED (OD-3):** rejected at load by a custom visitor naming the duplicate.
4. **Rule-reference check location** (loader vs `load_inputs`). Recommendation: `load_inputs`, since both configs (static and dynamic) pass through it. **RESOLVED:** `compile::load_inputs` (`check_rules`).

## Sources

### Primary (HIGH confidence, this repo)
- `src/models/config.rs`, `src/emitter.rs`, `src/compile.rs`, `src/loader.rs`, `src/dynamic.rs`, `src/run.rs`, `src/cli.rs`, `src/overlay/{edit,rule_editor,store,app}.rs`
- `Cargo.toml`, `Cargo.lock` (indexmap 2.14.0 with serde)
- `config.json`, `dynamic_base.json`, `dynamic.json` (style key sets compared with node)
- `tests/cli_generate.rs`, `tests/cli_errors.rs`, other integration tests (copy lists)
- `.planning/phases/17-config-driven-styles/17-CONTEXT.md`, `.planning/research/PITFALLS.md` §8, §9, §23, `.planning/REQUIREMENTS.md`

### Secondary / Tertiary
- None. No external web research was needed; this is an internal refactor.

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH (indexmap presence in lockfile verified; user-approved)
- Architecture: HIGH (all call sites read)
- Pitfalls: HIGH for the repo-specific test breakage; MEDIUM for exact test counts (grep-based, not exhaustive; re-grep `include_str!` of config/dynamic_base when planning)

**Research date:** 2026-10-12
**Valid until:** 2026-11-12 (stable; invalid if Phases 16-18 land large changes to `Config` first)
