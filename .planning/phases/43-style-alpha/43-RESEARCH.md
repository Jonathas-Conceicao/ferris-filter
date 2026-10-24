# Phase 43: Style Alpha - Research

**Researched:** 2026-10-23
**Domain:** Internal Rust change (FerrisFilter): style colour type, serde, emitter, egui overlay
**Confidence:** HIGH (every claim below was read from this repo in this session, `[VERIFIED: codebase grep/read]`)

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions
- Reading: a colour array may have 3 or 4 values. A 3-value colour (older files) is read as alpha 255, as today.
- Writing: from this version on, colours are **always saved with alpha present** (4 values), even when alpha is 255. (The author overrode the "write alpha only when not 255" proposal.)
- A `styles.json` with 3-value colours loads **without showing as an edit** (no ` *`, no unapplied indicator, no Quit popup), like the id migration: it is rewritten in the 4-value form by the next Apply that writes `styles.json` for another reason. `styles.json` is still written only when the CONFIG tab has edits.
- The committed `styles-default.json` moves to the 4-value form for consistency. `tests/fixtures/seed-styles.json` stays frozen as is (3-value) and keeps proving the 3-value read path.
- Filter output is unchanged for alpha 255: the emitter already writes `R G B 255`, so `generate` output, the fixtures and `artifact_guard` stay byte-identical. The unique bronze background (`175 96 37 240`) stays a pricer/emitter constant, not a style.
- Editor: the CONFIG style colour picker gets the same Alpha slider the RULES colour picker has, for all three slots. The CLAUDE.md note "styles store RGB ... the style colour picker has no alpha" becomes false and must be updated.
- Reset to defaults, strictness and the "greyed at the defaults" check compare colours including alpha.
- Previews: label preview and style table preview paint translucent colours over the dark panel ground. No checkerboard.

### Claude's Discretion
- The Rust colour type (e.g. a `Colour { rgb, alpha }` newtype or `[u8; 4]`) and its serde.
- Whether Custom rule colour editing (RULES style lines) needs any change beyond sharing the type.

### Deferred Ideas (OUT OF SCOPE)
None.
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| ALPHA-01 | Style text/border/background can carry alpha; picker has Alpha slider | Sites 1, 8, 9, 10 below: `colour_slot_rgba` already exists and is the path to reuse |
| ALPHA-02 | Alpha reaches the filter in `generate`, economy section, styled rewrite | One site: `emitter::style_action_lines` (src/emitter.rs:241-253); all three callers go through it |
| ALPHA-03 | 3-value file loads as 255, no edit; next styles write saves 4-value; fixtures unchanged | Custom serde on the new type; dirty counting is value-based so no migration flag is needed |
| ALPHA-04 | Label preview and style table show translucent colour | `ff_item_label` already blends; the gap is `StyleView::from_style` forcing 255, and the card swatch / `on_color` |
</phase_requirements>

## Summary

The change is smaller than the type change suggests. The data path is: `Style` (src/models/config.rs:243) holds three `Option<[u8; 3]>`; the single emitter `style_action_lines` hard-codes `255`; the overlay already has a complete alpha-capable picker path used by RULES (`widgets::colour_slot_rgba`, `PickerMode { alpha: true, allow_none: true }`) and `ff_item_label` already paints `Color32::from_rgba_unmultiplied`, so translucent previews mostly come for free once `StyleView::from_style` stops forcing 255.

The 3 -> 4 migration needs **no migration flag**. Unlike ids (`ids_unpersisted`, config_state.rs:105/159/702, which forces a write), the locked decision says the 4-value form is saved only when Apply writes `styles.json` anyway. `ConfigState` already keeps the loaded bytes untouched when `styles_changed()` is false (config_state.rs:694-716) and counts dirtiness by comparing `Style` values (`styles_dirty_count`, :481). If the deserialised colour type maps 3 values to alpha 255 and equality covers all four channels, a 3-value file is automatically "not an edit", and `render_styles` (serde) automatically writes 4 values on the next write.

**Primary recommendation:** Introduce `StyleColour` (a `Copy` newtype over `[u8; 4]`, derived `PartialEq/Eq`, hand-written serde: read 3 or 4, write 4) in `src/models/config.rs`; change `Style.text/border/background` to `Option<StyleColour>`; edit `style_action_lines`, `StyleView::from_style`, `StyleEdit::Colour`, `config_view::style_edit_for`; delete the opaque picker path; convert `styles-default.json` to 4-value; update CLAUDE.md.

Note: `.planning/research/SUMMARY.md` (lines 27, 61, 77) proposed "alpha 255 normalised to absent / written only when not 255". CONTEXT.md overrides that (always 4 values). Follow CONTEXT.md.

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary | Rationale |
|------------|-------------|-----------|-----------|
| Colour type, serde, validation | models + loader (`src/models/config.rs`, `src/loader.rs`) | — | Pure data; no egui |
| Filter text emission | `src/emitter.rs` | `src/styled.rs`, economy path (same fn) | One definition of what a style writes |
| Dirty/defaults/strictness | `src/overlay/config_state.rs` (pure model) | — | Value comparison on `Style` |
| Picker + Alpha slider | `src/skin/colour_picker.rs`, `src/overlay/widgets.rs` | `style_lines.rs` | Already built for RULES |
| Previews | `src/skin/editor.rs` (`ff_item_label`, `ff_colour_card_ex`) | `config_view.rs`, `style_lines.rs::label_look` | Painting only |

## Standard Stack

No new dependencies (locked). Uses existing `serde 1`, `serde_json`, `egui`. `[VERIFIED: codebase]` Precedent for hand-written serde in the same file: `Cancelable<T>` (src/models/config.rs ~164-200) with a `de::Visitor`.

## Code Site Map (file:line, current behaviour, required change)

### Model and serde
1. **`Style` struct** src/models/config.rs:243-266 (`text/border/background: Option<[u8; 3]>` at 247-249; doc comment at 241-242 says "RGB triples; alpha is added at generate-time" - update). `#[serde(deny_unknown_fields, rename_all = "camelCase")]`, no `default` on the colours, so a missing key is a parse error today; keep that.
2. **`Style::blank_show`** :285-287 (`None`s; no change). Other `Style { .. }` literals: emitter.rs:989-991, overlay/body.rs:2656-2658 and 2670-2672 (these two are body `StyleLines`-like structs with `[u8;4]`/`Rgba`, check type before editing; they are rule-body structs, not `Style`).
3. **`Meta.background: [u8; 3]`** :1237 is `config.json` meta, unrelated. Leave.
4. **`pricer.rs:52,59` and `sync/mapping.rs:73` `background_override: Option<[u8; 3]>`** are the unique bronze/category background, a separate constant path. Leave as `[u8; 3]` (locked: bronze stays a constant).
5. **No custom styles serializer exists**: `render_styles` (src/loader.rs:316) calls `serde_json::to_string(style)` per style, one compact line each. So the "one-style-per-line serializer" needs no change; the colour type's `Serialize` alone yields `[r,g,b,a]` with no spaces (compact). Field order follows struct order, `id` first.
6. **Loader** `load_styles_str` (loader.rs:~290) = `load_json_str` -> `assign_missing_ids` -> `validate`. Colour range/length errors surface as serde parse errors (`LoadError::Parse`, dotted path starts at style name, e.g. `S.text`). Today an out-of-range value fails as a `u8` serde error and a wrong length as "invalid length". No test greps those strings (checked "array of size" / "invalid length": no hits), so the visitor's `expecting` can say `an array of 3 or 4 values (0-255)` (UI-SPEC copywriting item). `StyleSet::validate` (config.rs:~566-700) has no colour checks and needs none.
7. **`StyleSet::shipped_defaults`** config.rs:~537 and `committed_for_tests` :950 parse via `serde_json::from_str` directly; no change besides the file content.
8. **Legacy inline `styles` check** loader.rs:~246 `check_legacy_styles` deserialises inline JSON into `Style` and compares with `!=`; a 3-value inline config equals the 4-value loaded style. No change; a test already covers it at tests/cli_errors.rs:294 (3-value `craftable`, stays valid input).

### Emitter (ALPHA-02)
9. **`style_action_lines`** src/emitter.rs:~222-290: lines 241-253 write `SetTextColor {r} {g} {b} 255`, `SetBackgroundColor ... 255`, `SetBorderColor ... 255`. Change `255` to `colour.alpha()`. Canonical order (beam, minimap, text, background, border, font, sound, drop sound) must not change.
10. **Callers** (all via this fn, so alpha propagates with no further edit): `emit_style_actions` emitter.rs:292 (generate and economy tier blocks); `styled::rewrite_block` src/styled.rs:102 (compares normalised lines, so a style whose alpha changes is rewritten; a 3-token rule line vs `... 255` already differs and is rewritten today, unchanged behaviour); overlay/body.rs:2483 (test-only `style_owned`); overlay/style_lines.rs:1137 (test).
11. **Bronze unique block** emitter.rs:575-577 and 640-642 clone the style, set `background = None`, then emit the bronze background separately. Keep; it uses `None`, which stays valid with `Option<StyleColour>`.
12. **Byte identity:** seed 3-value -> alpha 255 -> `... 255` as before. `artifact_guard` (tests/cli_generate.rs) and `tests/fixtures/seed-universal.filter` stay identical. Must be re-run, not edited.

### CONFIG state (ALPHA-03, equality)
13. **`StyleEdit::Colour { slot, value: Option<[u8; 3]> }`** config_state.rs:46-49 (doc at 41-43 says opaque). Change to `Option<StyleColour>`. Applied at :246-248.
14. **`styles_dirty_count`** :481-515 compares `Style` with `!=` (derived `PartialEq` -> includes colours). **`styles_changed`** :815. **`styles_at_defaults`** :308-315 compares `Vec<(&str,&Style)>`. **`reset_styles`** :294-305 clones defaults. All value-based: correct for alpha with no code change once the type is `PartialEq` over four channels.
15. **`prepare_write_inner`** :698-722: `styles_changed = force || self.styles_changed() || self.ids_unpersisted`; unchanged bytes keep `styles_bytes` (the loaded 3-value bytes). This is exactly the "rewritten by the next Apply that writes for another reason" behaviour. Do **not** add a 3->4 flag to `ids_unpersisted`-style logic (it would force a write and violate the locked "no unapplied indicator" rule).
16. **Strictness** `strictness_of` / `set_strictness` (:260-285) touch only `action`; no colour change needed. A test should pin that a strictness pick leaves colours (incl. alpha) untouched.
17. **Revert** to `styles_applied` snapshot: value clones, alpha included automatically.

### Overlay editor path (ALPHA-01)
18. **`StyleView::from_style`** src/overlay/style_lines.rs:71-95 maps `[u8;3]` to `Rgba { rgba: [r,g,b,255], had_alpha: false }` and sets `alpha: false` (line 93). Change: use the style's alpha, `had_alpha: true` (irrelevant for styles; written form is always 4 values), `alpha: true`.
19. **`StyleView.alpha`** (:62 doc) is `true` only in `body.rs:2666` (rules) and `false` in `from_style`. After the change both are `true`: remove the field and the opaque branch in the colour loop (style_lines.rs:290-325) rather than keep dead code. Test at body.rs:6670 `assert!(view.alpha)` must be updated if the field goes.
20. **Style colour cards** style_lines.rs:278-325: opaque branch calls `widgets::colour_slot` (returns `SlotResult`). The alpha branch calls `colour_slot_rgba(ui, tag, label, cur, had, seed, width)`; for styles pass seed alpha 255 (already how `seed` is widened at :293). Note `colour_slot_rgba` returns `Set` only when the colour differs from `current`, which satisfies "opening without a change edits nothing".
21. **`widgets::colour_slot` + `SlotResult`** widgets.rs:345-395 (picker mode `alpha: false`, :378) become unused; **`colour_swatch_rgb` / `rgb_caption`** widgets.rs:456-476 have no non-test caller (grep: only widgets.rs tests at :1523, :1636). Remove them with their tests (:1523-1524, :1636-1710) or port the `slot_frame` tests (:1661-1710: unchanged-on-open, Set([255,121,198]), Cleared) to `colour_slot_rgba`. Porting is recommended: they are the only tests of "open/idle emits nothing" for a slot.
22. **`style_edit_for`** src/overlay/config_view.rs:1184-1198 drops alpha (`value.map(|c| [c[0],c[1],c[2]])`) and its doc says "styles are opaque". Change to pass `value.map(StyleColour::from_array)`. **This is the line that would silently drop alpha**; test it.
23. **Picker** src/skin/colour_picker.rs: `PickerMode.alpha` (:31) doc says "styles are opaque" - update wording. `PickerEdit::Set([u8;4])` already RGBA. Escape-restores-opening-colour includes alpha already (RULES tests at :832-932 cover idle and quick-pick-keeps-alpha). No functional change needed here.
24. **`colour_chip`** widgets.rs:~310 is the rule path; untouched.

### Previews (ALPHA-04)
25. **`label_look`** style_lines.rs:~213-225 already builds `Color32::from_rgba_unmultiplied(r,g,b,a)` from `Rgba.rgba`; `config_view::label_look` (:1106-1108) goes through `StyleView::from_style`. So fixing #18 fixes both the editor preview and the table Preview column (config_view.rs:1042-1044).
26. **`ff_item_label`** src/skin/editor.rs:444-490 paints the dark ground first (`#14120E`), then fills the label background with `look.background...gamma_multiply(fade)`, strokes the border, and draws the galley in `text_c`. Straight-alpha colours blend over the ground already. Caveat: the label rect sits on the ground, but the *table row* behind is irrelevant (ground is painted per preview). No change required; add a pixel test (see Validation).
27. **`ff_colour_card_ex`** editor.rs:331-398 fills the card with `c` directly over whatever is behind (panel), so a translucent swatch blends with the panel, not the card ground. UI-SPEC item 5: "swatch painted with that alpha over the card ground". Paint `pal::INPUT_FILL` first, then the colour. Also **`on_color(c)`** (editor.rs:312) ignores alpha when picking caption contrast; compute luminance of `c` composited over `INPUT_FILL` so a faint colour gets a readable caption.
28. **Swatch tooltip**: today the card has a tooltip only when its caption is cut (editor.rs:~392 `on_hover_text(&text)`). UI-SPEC item 5 wants `R G B A` (not 255 only) on the card. `rgba_caption` exists (widgets.rs ~296) and formats `R {} G {} B {} A {}`; the card helper is in `skin::editor` (no access to `widgets`), so add the tooltip in `style_lines.rs`/`widgets::colour_slot_rgba` where the `Response` is available (`card.on_hover_text(...)` only when `a != 255`), watching that the cut-caption tooltip and this one do not stack (pick one: alpha text when `a != 255`, else the existing cut text).
29. Screenshot mode `--picker text|border|background` (existing flag) is the visual check; the debug-only screenshot harness needs a style with alpha < 255 to show the slider is on the style slots (the slider shows whenever mode.alpha, regardless of value).

### Defaults and fixtures
30. **`styles-default.json`** (11 lines) -> add `,255` to every colour array of the 7 coloured styles (S..F; `unchanged`/`hidden` are `null`). 21 arrays. Do it by running `render_styles` over the loaded defaults (the round-trip test `shipped_styles_round_trip_through_render_styles`, defaults.rs:182-187, demands byte equality with `render_styles(&shipped_styles())`, so generate the file from the code, do not hand-edit spacing).
31. **`defaults.rs` test `shipped_styles_match_the_author_snapshot`** :129-178 holds nine `json!({... "text":[255,121,198] ...})` literals (S..F at lines 140-164); update the seven arrays to 4 values. `unchanged`/`hidden` literals have `null`s, no change. Also `embedded_matches_disk` style tests (:76, :83) compare bytes with the disk file; follow automatically.
32. **`tests/fixtures/seed-styles.json`** (14 lines, 3-value): unchanged. `StyleSet::committed_for_tests()` (config.rs:950) parses it; every test using it exercises the 3-value read path. After the change `render_styles(&committed_for_tests())` produces 4-value, so tests comparing against seed bytes would break; grep found none (`render_styles_shape_and_round_trip` round-trips through the parser, and checks `"text":null` for `unchanged`, still true).
33. **Other literal 3-value JSON in tests (leave as 3-value, they test the read path):** tests/cli_errors.rs:294, tests/cli_generate.rs:654.

### Typed literals that must change (count)
Direct `Style` colour literals / asserts outside the files above:
- src/emitter.rs:1026 `Some([1, 2, 3])`; :1093-1095 `Some([10,20,30])`, `[40,50,60]`, `[70,80,90]`
- src/models/config.rs:1992 and :1997 (`Some([255,121,198])` asserts), :1412 (`is_none`, fine)
- src/overlay/style_lines.rs:1040 `Some([1, 2, 3])`
- src/overlay/config_view.rs:1735-1737 (asserts on `LabelLook` `Color32`, fine; values unchanged at alpha 255)
- src/overlay/app.rs:6569-6570 `StyleEdit::Colour { slot: Text, value: ... }` (test constructing the edit)
- src/overlay/config_state.rs:1274, :1282 (`StyleEdit::Colour` in tests)
- src/overlay/widgets.rs:1661-1710 (opaque slot tests, see #21)
About 12 edit sites in tests; use `StyleColour::opaque(r,g,b)` to keep them one-line. Raw `[u8; 3]` occurrences total 14 in src; none in tests/ or tools/. Integration tests (tests/) never construct `Style` colours (JSON strings only).

## Architecture Patterns

### Recommended type (also for Phase 44)
```rust
// src/models/config.rs
/// A style colour: R, G, B and alpha. Equality is over all four channels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct StyleColour([u8; 4]);

impl StyleColour {
    pub const fn new(r: u8, g: u8, b: u8, a: u8) -> Self { Self([r, g, b, a]) }
    pub const fn opaque(r: u8, g: u8, b: u8) -> Self { Self([r, g, b, 255]) }
    pub fn from_array(c: [u8; 4]) -> Self { Self(c) }
    pub fn rgba(self) -> [u8; 4] { self.0 }
    pub fn alpha(self) -> u8 { self.0[3] }
}
// Serialize: a 4-element sequence. Deserialize: Visitor::visit_seq reading u8s,
// accepting exactly 3 (alpha = 255) or 4; anything else errors with
// expecting() = "an array of 3 or 4 values (0-255)".
```
Rationale for a newtype over bare `[u8; 4]`:
- **No `Default` and no `From<[u8; 3]>`**: nobody can build an opaque colour by accident, and a 3-value colour is read as alpha 255 in exactly one place (the visitor).
- **Equality traps for Phase 44 (Auto Detect) are visible in the API**: a style colour always has an explicit alpha, so comparing a rule's `SetBackgroundColor r g b` (game default alpha 240, per research SUMMARY pitfall 2) against a style colour must be an explicit decision, never `==` on raw arrays. Phase 44 should add a `StyleColour::from_line_tokens(&[u8]) -> Option<Self>` that takes the slot's default alpha as an argument (background 240, text/border 255) rather than letting the 3-token case silently equal alpha 255. NOTE: the emitter writes `SetBackgroundColor ... 255` for an alpha-255 style, so a bare `SetBackgroundColor r g b` rule line (alpha 240 in game) is **not** equal to an alpha-255 style even though the text matches. This is a Phase 44 concern; this phase only needs to not erase the information.
- `Eq`/`Hash` derive lets Phase 44 use colours as map keys.
- The overlay's `style_lines::Rgba { rgba: [u8;4], had_alpha }` is a different concept (rule line + token form). Keep that name distinct; do not reuse `Rgba` for the model type (name clash in `style_lines.rs`).

### Anti-Patterns to Avoid
- **Normalising 255 to absent** (old SUMMARY proposal): overridden by CONTEXT; always write 4 values.
- **A 3->4 "unpersisted" flag modelled on `ids_unpersisted`**: forces a write; breaks ALPHA-03 criterion "without showing as an edit".
- **`#[serde(untagged)]` enum of `[u8;3]`/`[u8;4]`** for the colour: loses the dotted-path error and complicates equality. Use a `visit_seq` visitor.
- **Hand-editing `styles-default.json` whitespace**: the round-trip test demands render_styles bytes.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead |
|---------|-------------|-------------|
| Alpha slider + number field | New slider | `colour_picker` with `PickerMode { alpha: true, allow_none: true }` via `widgets::colour_slot_rgba` |
| Translucent painting | Manual blend maths | `Color32::from_rgba_unmultiplied` + painter over the ground (already in `ff_item_label`) |
| 3-or-4 array serde | `untagged` enum / two fields | one `visit_seq` visitor, precedent `Cancelable` in config.rs |
| Writing styles.json | New serializer | existing `loader::render_styles` |
| Dirty detection | A separate migrated flag | derived `PartialEq` on `Style` |

## Common Pitfalls

### 1. `style_edit_for` drops alpha
config_view.rs:1189-1192 truncates to RGB. Result: slider moves, preview changes (live edit goes through intent) but nothing is stored. Test: edit alpha, read `style.text.alpha()`.

### 2. Loaded-3-value shows as edit
Only if equality or snapshots compare raw bytes/JSON. They compare `Style` values; keep it that way. Test: load seed-style file as live `styles.json`, assert `dirty_count() == 0`, `prepare_write()` returns `styles_changed == false` with original bytes.

### 3. `styled::rewrite_block` churn
Rule lines written by earlier versions say `SetTextColor r g b 255` (4 tokens), identical to new output for alpha 255, so no churn. A hand-written 3-token line was already rewritten before this phase. No change; do not "fix" it.

### 4. Colour card and caption contrast on translucent colours
See #27. `on_color` ignores alpha; a 10% alpha white card would show cream caption on near-panel colour. Composite over `INPUT_FILL` before luminance.

### 5. Tooltip stacking on the card
See #28. Only one `on_hover_text` should fire.

### 6. Reset to defaults restores alpha
`reset_styles` clones `shipped_defaults()` (4-value, alpha 255). A user custom alpha on a built-in resets to 255. Correct, pin with a test.

### 7. Generate fixtures
`seed-styles.json` must stay 3-value and `seed-universal.filter` untouched: run `cargo test` `artifact_guard`/`generate_snapshot` as the gate; do not regenerate the fixture.

### 8. egui Color32 is premultiplied
Always construct from unmultiplied `[r,g,b,a]` (`from_rgba_unmultiplied`) and convert back with `to_srgba_unmultiplied` if round-tripping; never read `.r()/.g()/.b()` of a translucent `Color32` as the stored RGB.

## Code Examples

### Visitor (the one place 3 means alpha 255)
```rust
// Source: pattern of Cancelable<T> Deserialize, src/models/config.rs ~172
impl<'de> Deserialize<'de> for StyleColour {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        struct V;
        impl<'de> de::Visitor<'de> for V {
            type Value = StyleColour;
            fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                f.write_str("an array of 3 or 4 values (0-255)")
            }
            fn visit_seq<A: de::SeqAccess<'de>>(self, mut s: A) -> Result<StyleColour, A::Error> {
                let mut v = [255u8; 4];
                for (i, slot) in v.iter_mut().enumerate().take(3) {
                    *slot = s.next_element()?.ok_or_else(|| de::Error::invalid_length(i, &self))?;
                }
                if let Some(a) = s.next_element::<u8>()? { v[3] = a; }
                if s.next_element::<u8>()?.is_some() {
                    return Err(de::Error::invalid_length(5, &self));
                }
                Ok(StyleColour(v))
            }
        }
        d.deserialize_seq(V)
    }
}
impl Serialize for StyleColour {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> { self.0.serialize(s) } // [u8;4] -> 4-array
}
```

## State of the Art

Not applicable (internal change). CLAUDE.md edits required (UI-SPEC item 8): the `styles.json` row (colours read 3 or 4 values, written 4; "re-serialise byte-identically" sentence about new fields stays true for files that already have 4 values, but a 3-value file re-serialises to 4-value on the first write), and the Config tab colour-picker note (remove "styles store RGB and the filter always emits alpha 255, so the style colour picker has no alpha"; the style picker has Alpha, applies to all three slots). Also the line in the `styles.json` row listing style object fields ("text/border/background colours") and the `Colour picker` passage ("no alpha for styles" and "**None** on style slots ... no alpha for styles"). No changelog/README entries during the milestone.

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | `body.rs:2656-2672` literals are rule-body structs, not `Style` (read only the line numbers from grep) | Site map #2 | Planner finds two more `Style` literals to convert; trivial |
| A2 | Tooltip on the style card should be added in `widgets`/`style_lines`, not `skin::editor` | #28 | Minor placement choice |

Everything else in this document was read from the repo this session.

## Open Questions

1. **Does the card tooltip apply to the RULES colour cards too?**
   - Known: UI-SPEC item 5 speaks of the "colour card"; RULES uses the same `colour_slot_rgba` card, where a rule colour may already be 4-token.
   - Recommendation: apply uniformly in `colour_slot_rgba` (alpha != 255 only), since both paths share the helper; RULES tests that assert tooltips (if any) must be checked.
2. **Remove vs keep `colour_slot`/`colour_swatch_rgb`?** Recommendation: remove (dead after this phase), port their tests.

## Environment Availability

Skipped: code and config changes only (cargo toolchain already in use: nightly via `rust-toolchain.toml`).

## Validation Architecture

Skipped: `workflow.nyquist_validation` is `false` in `.planning/config.json`. Gate for the planner anyway: `cargo test --locked` (includes `artifact_guard`, `generate_snapshot`, `shipped_styles_*`, `isolation_guard`), plus `cargo clippy` and `cargo fmt --check`. Tests the planner should add: serde 3 and 4 value read, 4-value write, bad length / out-of-range error; 3-value live file gives `dirty_count()==0` and unchanged bytes; an alpha edit becomes dirty, writes 4-value, reverts, and resets; `style_edit_for` keeps alpha; emitter writes alpha 128 in `generate`, economy and styled rewrite; `ff_item_label` pixel/colour test at alpha < 255 (exposed `Color32` values); picker idle-emits-nothing for a style slot.

## Security Domain

Applies minimally: input is a user-editable local JSON. V5 input validation: reject non 3/4-length arrays and values outside 0-255 at load (serde `u8`), same as today; no new file paths, network, or parsing of free text. No ASVS control changes.

## Sources

### Primary (HIGH confidence, repo reads this session)
- src/models/config.rs, src/loader.rs, src/emitter.rs, src/styled.rs, src/defaults.rs
- src/overlay/config_state.rs, config_view.rs, style_lines.rs, widgets.rs, body.rs
- src/skin/colour_picker.rs, src/skin/editor.rs
- styles-default.json, tests/fixtures/seed-styles.json, tests/cli_errors.rs, tests/cli_generate.rs
- .planning/phases/43-style-alpha/43-CONTEXT.md, 43-UI-SPEC.md, .planning/research/SUMMARY.md, CLAUDE.md

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH (no new deps)
- Architecture / site map: HIGH (grep + reads; line numbers approximate to a few lines where marked `~`)
- Pitfalls: HIGH

**Research date:** 2026-10-23
**Valid until:** until src/overlay/ or src/models/config.rs change materially
