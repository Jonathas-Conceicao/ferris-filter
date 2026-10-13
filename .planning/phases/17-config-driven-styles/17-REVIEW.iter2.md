---
phase: 17-config-driven-styles
reviewed: 2026-10-12T00:00:00Z
depth: standard
files_reviewed: 21
files_reviewed_list:
  - src/models/config.rs
  - src/loader.rs
  - src/emitter.rs
  - src/compile.rs
  - src/cli.rs
  - src/run.rs
  - src/dynamic.rs
  - src/overlay/edit.rs
  - src/overlay/rule_editor.rs
  - src/overlay/store.rs
  - src/overlay/view_model.rs
  - styles.json
  - config.json
  - dynamic_base.json
  - Cargo.toml
  - tests/cli_errors.rs
  - tests/cli_generate.rs
  - tests/dynamic_filter.rs
  - tests/filter_roundtrip.rs
  - tests/rule_editor.rs
  - tests/sync_prices.rs
findings:
  critical: 0
  warning: 4
  info: 5
  total: 9
status: issues_found
---

# Phase 17: Code Review Report

**Reviewed:** 2026-10-12
**Depth:** standard
**Files Reviewed:** 21
**Status:** issues_found

## Narrative Findings (AI reviewer)

## Summary

I reviewed the Phase 17 diff (`e116acc..HEAD`). It moves named styles from `config.json` and `dynamic_base.json` into `styles.json` (`StyleSet` over `IndexMap`), rejects duplicate names, validates required styles, checks unknown styles at load, rejects legacy inline `styles` sections, and adds `--styles`.

The core works:
- Duplicate keys are rejected inside the custom visitor.
- `deny_unknown_fields` still applies to each `Style`.
- The skipped `styles` field and the `legacy_styles` field, renamed to `styles`, coexist correctly under `deny_unknown_fields`.
- `store::save` never writes styles.
- `tests/fixtures/` is untouched, so the byte-identity guarantee (STYLE-04) holds without re-baselining.

I found no blockers. The main concerns:
1. The type no longer guarantees the invariant that S..F exist. A `Config` built by plain deserialization now panics inside `generate`.
2. `StyleSet::validate` is now the gate for a user-editable file, but it checks almost nothing about the values a style sets. A valid-looking `styles.json` can produce a filter the game rejects, or silently lose a sound.
3. The documented OD-1 migration leaves the overlay unable to start against any pre-17 `dynamic.json`. The user's own store is one of these: it has an inline `styles` section with 13 entries, including `craftable`. The error message also suggests a remedy (`--styles`) that does not fix the error and does not exist in the overlay.

## Warnings

### WR-01: `generate` panics on a `Config` whose `StyleSet` was never loaded

**File:** `src/emitter.rs:219-227` (`style_for_tier`), `src/models/config.rs:567-568` (`#[serde(skip)] pub styles: StyleSet`)
**Issue:** Before Phase 17, deserializing a `Config` guaranteed that S..F existed, because `Styles` had required named fields. Now `Config.styles` is `#[serde(skip)]` and falls back to `StyleSet::default()`, which is empty. Two code paths fill it in (`compile::load_inputs` and `overlay::store::load`), and only `loader::load_styles` runs `validate()`. `emitter::generate` and `generate_as` are `pub` in a `pub mod` of the library crate. If they are called with a `Config` from `serde_json::from_str`/`load_json` (as several in-crate tests do before patching `styles`), the call panics at `style_for_tier` with "Unknown tier 'S' in priced data" as soon as priced data has a category or unique block. The message also misdiagnoses the cause. The comment's claim that "`StyleSet::validate` guarantees those are defined" holds only by convention.
**Fix:** Make the failure a typed error instead of a panic, and validate where `generate` starts:
```rust
pub fn generate_as(config: &Config, priced: &PricedData, id: &Identity) -> Result<String, GenerateError> {
    config.styles.validate().map_err(GenerateError::Styles)?; // new variant wrapping StyleError
    ...
}
fn style_for_tier<'a>(styles: &'a StyleSet, tier: &str) -> Result<&'a Style, GenerateError> { ... }
```
Alternatively, make `StyleSet` constructible only through a validated path (private field plus `TryFrom`), so an empty or unvalidated set cannot reach the emitter.

### WR-02: `StyleSet::validate` lets a styles.json through that emits an invalid or silently degraded filter

**File:** `src/models/config.rs:207-238` (`validate`), `src/models/config.rs:70-78` (`Sound` doc), `src/emitter.rs:196-203`
**Issue:** `styles.json` is now the supported way to add styles (STYLE-01), and `validate()` is the only semantic gate. It checks only required names, case-shadowing and `sound.custom` characters. Every numeric field is a bare `u8`/`u16` that serde range-checks only against the integer type. These all pass:
- `font: 0` or `font: 200` (the game range is 1..45)
- `minimap.size: 7` (the game range is 0..2)
- `sound.id: 0` or `99` (the game range is 1..16)
- `sound.vol: 5000` (the game range is 0..300)

These are emitted verbatim, so PoE2 can reject the whole filter on load or clamp the values, and it is the in-game load that fails, not `generate`. A `sound` with neither `id` nor `custom` (`{"vol":100}`) emits nothing. A `sound` with both silently drops `id` (`emitter.rs:197-201`). The `Sound` doc comment says "Mutual-exclusivity of `id` and `custom` is enforced at generate-time", which is false.
**Fix:** Extend `validate()` with range and exclusivity checks that name the style and field. For example:
```rust
if let Some(f) = style.font { if !(1..=45).contains(&f) { return Err(StyleError::OutOfRange { style: name.clone(), field: "font", value: f as u32, range: "1..=45" }); } }
if let Some(m) = &style.minimap { if m.size > 2 { ... "minimap.size" ... } }
if let Some(s) = &style.sound {
    match (s.id, &s.custom) {
        (Some(_), Some(_)) | (None, None) => return Err(StyleError::BadSound { style: name.clone() }),
        (Some(id), None) if !(1..=16).contains(&id) => ...,
        _ => {}
    }
    if s.vol > 300 { ... }
}
```
Then fix the `Sound` doc comment.

### WR-03: Any pre-Phase-17 `dynamic.json` stops the overlay from starting, and the error suggests a fix that does not work

**File:** `src/loader.rs:120-126` (`load_config`), `src/overlay/store.rs:67`, `src/models/config.rs:434-438` (`InlineStylesSection` message)
**Issue:** Every existing store was seeded from the old `dynamic_base.json`, so every one has an inline `styles` section. The repo's own git-ignored `dynamic.json` has 13 styles, including `craftable`. `store::load` now returns `Err`, and `OverlayApp::new` (`src/overlay/app.rs:167-173`) sets `config: None`. The GUI therefore has no editor and no in-app recovery. The user must hand-edit JSON, or run `apply --reset`, which discards every rule edit. OD-1 records this as a known decision. Two defects remain within that decision:
1. The message says "move its entries into styles.json (or pass --styles <file>)". `--styles` does not bypass the rejection: `load_config` fails before styles are consulted. The overlay has no `--styles` at all, and its users see this message as a plain string.
2. The safe automatic migration is cheap and is not taken. `store::save` already never writes `styles`. A legacy section whose entries all equal the loaded `styles.json` entries (the common case: an untouched seeded store) could be dropped without losing anything. Erroring only when the section actually differs would still meet OD-1's "do not silently ignore an edit".

**Fix:**
- Reword the message: "...move any entries you changed or added into styles.json, then delete the `styles` section from {path}". Mention `--styles` only on CLI paths.
- In `load_config`, or in `store::load` only, accept the legacy section when every entry `== styles.get(name)` and reject it otherwise, naming the differing style (e.g. `craftable`). The next Apply then rewrites the store without the section.

### WR-04: `"styles": null` bypasses the legacy-section rejection

**File:** `src/models/config.rs:571-572`, `src/loader.rs:122`
**Issue:** `legacy_styles: Option<serde_json::Value>` with `#[serde(default)]` deserializes JSON `null` to `None`. A config containing `"styles": null` is therefore accepted silently. The OD-1 contract ("an inline `styles` key is a load error naming the file") is not met for that spelling, and a hand-edited config that nulled the section to "turn it off" gets no signal.
**Fix:** Detect key presence rather than value, e.g. `#[serde(default, rename = "styles", deserialize_with = "present")] legacy_styles: Option<Option<serde_json::Value>>`, using the usual double-option pattern. Then reject `Some(_)`.

## Info

### IN-01: `StyleSet::from_pairs` is dead public API that silently keeps the last duplicate

**File:** `src/models/config.rs:199-205`
**Issue:** It has no callers in `src/` or `tests/`. It collects into an `IndexMap`, so a duplicate name is last-wins, which contradicts OD-3's duplicate rejection, and it runs no `validate()`.
**Fix:** Delete it, or make it `#[cfg(test)]`, or have it return `Result` and reject duplicates.

### IN-02: The unknown-style check lives in two places with two error types

**File:** `src/models/config.rs:242-258` (`check_rules` → `StyleError::UnknownInRule`), `src/emitter.rs:105-111` (`resolve_style` → `GenerateError::UnknownStyle`)
**Issue:** Both lists are built with the same expression, and both skip disabled rules separately (`config.rs:244`, `emitter.rs:301`). They can drift: for example, one could change its label format (`rule {idx}`) or its disabled-rule semantics without the other. The emitter path is now reachable only through direct library calls (see WR-01).
**Fix:** Have `resolve_style` reuse `StyleSet` lookup plus a shared formatter, or keep a single check and have the emitter treat a lookup miss as an internal error.

### IN-03: CLI `apply` and the overlay resolve `styles.json` from different directories

**File:** `src/run.rs:145` (`InputPaths::conventional()`, cwd) vs `src/overlay/store.rs:72-73` and `src/overlay/app.rs:513` (`relative_to(store_dir)`)
**Issue:** With `apply --store other/dynamic.json`, the CLI reads `./styles.json`, but the overlay would read `other/styles.json`. This existing split for prices and tiers now also covers styles, so the same store can compile against different style sets depending on the entry point.
**Fix:** Either document it, or have `cmd_apply` use `InputPaths::relative_to(store_path.parent())` to match the overlay.

### IN-04: Non-required style names may collide by case

**File:** `src/models/config.rs:216-226`
**Issue:** Case-shadowing is rejected only against `REQUIRED`. `Recycle` and `recycle`, or `kite` and `Kite`, can coexist, and both appear in the overlay picker as near-identical entries. An empty name `""` is also accepted and renders as a blank picker row.
**Fix:** Reject case-insensitive duplicates across all names, and reject empty or whitespace-only names.

### IN-05: The `overlay::store` module doc is stale

**File:** `src/overlay/store.rs:3-9`
**Issue:** The doc says `load` uses `loader::load_json` (it now uses `load_config` plus `load_styles`) and that `save` is called "once per frame from the editor's autosave (D-02)". Apply has been the single writer since Phase 14.
**Fix:** Update the module doc to describe `load_config` + `load_styles` and Apply as the only writer.

---

_Reviewed: 2026-10-12_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
