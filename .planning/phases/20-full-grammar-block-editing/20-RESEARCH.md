# Phase 20: Full-Grammar Block Editing - Research

**Researched:** 2026-10-12
**Domain:** Rust library (lossless filter edit API) + egui 0.36.1 overlay (data-driven rule body) + `styles.json` linkage
**Confidence:** HIGH on codebase facts and egui API names (read from source); MEDIUM on UX edge cases not covered by the UI-SPEC (flagged in Open Questions)

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

**Condition editing (intuitive and accessible, author emphasis)**
- **+ condition** picker listing every condition keyword from Phase 16's keyword table, in readable groups. Each condition row has three parts: the keyword; an operator dropdown offering only the operators valid for that keyword, with plain-English labels as in v0.4 ("At least", "Exactly", ...); a value control chosen by value type.
- The same keyword may appear more than once, e.g. `ItemLevel >= 70` and `ItemLevel <= 80`.
- Value controls (pickers and sliders, not raw text, where the value has structure):
  - **Class:** v0.4's grouped class picker (JEWELLERY etc. from `config.json`'s `jewelleryClasses`; never hardcode groups in the GUI).
  - **BaseType and other quoted string lists:** v0.4's list entry (type + Enter / **+ ADD**, per-entry Remove, quoted-paste splitting).
  - **Rarity:** a dropdown, with multi-value support for `Rarity Normal Magic Rare`.
  - **Numeric conditions** (ItemLevel, DropLevel, AreaLevel, Quality, Sockets, StackSize, WaystoneTier, Base defences, GemLevel, UnidentifiedItemTier, ...): sliders bounded by sensible per-keyword ranges, with an exact-number entry alongside.
  - **Booleans** (Corrupted, Mirrored, Identified, ...): a toggle.
  - **Enums:** dropdowns.
  - **Socket strings** (e.g. `5GGG`) and mod lists (`HasExplicitMod >=1 "..."`): structured fields that render valid grammar.
- **v0.4's three-checkbox defence picker is replaced** by the generic BaseArmour, BaseEvasion and BaseEnergyShield rows, which use the numeric slider controls.

**Action editing (EDIT-02)**
- Colours (SetTextColor, SetBorderColor, SetBackgroundColor): egui colour picker with alpha.
- SetFontSize: slider, 1-45.
- MinimapIcon: dropdowns for size, colour and shape.
- PlayEffect: colour, plus a `Temp` toggle.
- Sounds: `PlayAlertSound` / `PlayAlertSoundPositional`: id and volume sliders. `CustomAlertSound` (and `CustomAlertSoundOptional`): file name and volume.
- DisableDropSound / EnableDropSound (and the other drop-sound toggles in the grammar): toggles.

**Editing rules**
- Every edit renders valid grammar. Only the edited value's lines change (PARSE-04). An existing operator is never normalized (e.g. `Class "x"` never becomes `Class == "x"`) unless the user changes it.

**Styles on rules (STYLE-03, EDIT-06)**
- **Applying a style** to a rule writes a `#ff:style <name>` line into the rule's FerrisFilter metadata comment run (Phase 16 convention: header-style comment, not a tag inside the `Show #` text). It sets the rule's style actions from `styles.json`, and the style's Show/Hide overrides the rule's own (Phase 17).
- **On Apply, a rule's style actions are rewritten from its style** whenever its `#ff:style` line names a known style. This is how style edits in the config tab reach existing rules.
- **Hand-edited actions on a styled rule are overwritten by the style on Apply** (author decision: "if the style comment is not also hand-edited, we can just rewrite it"). Removing or changing the `#ff:style` line is how a rule leaves its style.
- The UI must make this visible: a styled rule's style-owned actions show as coming from the style, with an obvious way to remove the style when the user wants custom actions.
- A `#ff:style` naming an unknown style (deleted or typo) is reported. The rule keeps its current actions, and no rewrite happens for it.

### Claude's Discretion
- The grouping of the condition picker, slider ranges per keyword, widget layout within the narrow stripe, and how the accordion body is restructured from v0.4's fixed control set into the data-driven one.

### Deferred Ideas (OUT OF SCOPE)
- Raw-text block editing ("Edit as text"): backlog 999.10 (author: not now).
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| EDIT-01 | View and edit every condition on any block: all operators, multi-value lists, none read-only | New line-patch API on `filter::Document` (section "Edit API gap"); value codec per `ValueShape` with raw-token fallback (section "Value codec"); data-driven body (section "Overlay model") |
| EDIT-02 | View and edit every action: colours, font size, built-in/custom sounds, minimap icon, beam, drop-sound toggles | Same patch API; action codecs for `Rgba`, `FontSize`, `SoundId`, `CustomSound`, `MinimapIcon`, `PlayEffect`, `OptionalBool`; egui `color_edit_button_srgba` with `Alpha::OnlyBlend` |
| EDIT-06 | Apply a named style to a rule, setting its style actions | `ApplyStyle` RowAction + `emitter::style_action_lines` + `Document::set_meta("style")` (section "Styles") |
| STYLE-03 | Rule remembers its style via a comment tag; actions update on next Apply after the style changes | `rewrite_styled` pass hooked into `apply_working` before `apply_edited` (section "Styles") |
</phase_requirements>

## Summary

The Phase 16/18/19 stack already gives a lossless `Document` with a stable block index, `#ff:` metadata get/set/remove, structural block ops, and a guarded Apply writer. What it does **not** give is any way to change one body line. The only body mutator is `replace_block_lines` (replaces the whole owned range), and it has two properties that matter here: it rejects any enabled block that carries a Warning (including a pre-existing unknown-keyword line) [VERIFIED: src/filter/edit.rs:320], and it re-terminates every line in the block with one EOL [VERIFIED: src/filter/edit.rs:284-296]. Phase 20 therefore needs a small, validated **line-patch API** in `filter` first, then everything else is built on it. The overlay currently models a row as `RuleRow` with a flat `conditions: Vec<String>` display summary [VERIFIED: src/overlay/rules.rs:84]; the editor needs a typed per-line view of the expanded row (computed lazily for that one row).

The v0.4 class picker, base-type list and defence picker were deleted in Phase 19-01 (only `parse_base_type_entry` survived in `overlay/text_entry.rs`) [VERIFIED: 19-01-SUMMARY, src/overlay/text_entry.rs]. They must be rebuilt as generic controls; v0.4's `class_groups` grouping logic is recoverable from git (`git show 911666f^:src/overlay/edit.rs`, fn `class_groups`). The overlay currently never loads `config.json`, so the class catalogue (`weaponClasses`, `armourClasses`, `flaskCharmClasses`, `jewelleryClasses`) needs a small loader through `loader::load_config(root/config.json, root/styles.json)` [VERIFIED: src/loader.rs:132, src/models/config.rs:676-687].

Styles are the second large piece. `StyleSet`/`Style` exist (Phase 17) but `WorkingFilter` knows nothing of them; Phase 19 only displays the `#ff:style` tag (flag `style wins`, locked ACTION selector) [VERIFIED: src/overlay/rule_list.rs:109, :401]. The Apply path (`apply_working` -> `dynamic::apply_edited(loaded, &working.text(), op)`) is the single place to hook the rewrite [VERIFIED: src/overlay/app.rs:132-190]. A `Style` must be turned into action line texts; today that exists only as the private `emitter::emit_style_actions` writing into a `String` [VERIFIED: src/emitter.rs:209]. Extract a pure `style_action_lines` and have the emitter call it, so `generate` stays byte-identical (guarded by `artifact_guard`/`generate_snapshot`).

**Primary recommendation:** Build bottom-up in three waves: (1) egui-free `filter` line-patch API + egui-free value codec + style rewrite library, all proven against the FilterBlade corpus for byte-exact no-op edits; (2) overlay row model, RowActions, widgets and controls; (3) assemble the new `expanded_body`, banner/pickers, carry-over polish, docs. Keep every grammar decision in testable non-egui modules.

## Project Constraints (from CLAUDE.md)

- Build/test only with `CARGO_TARGET_DIR=C:/Users/jonat/AppData/Local/Temp/ff-target` (the running `ff.exe` locks the default target dir). Do not modify source during research.
- `tests/fixtures/seed-universal.filter` and `tests/snapshots/cli_generate__generate_snapshot.snap` must stay byte-identical (`artifact_guard` / `generate_snapshot`). Any refactor of `emit_style_actions` must keep them green.
- Class groups are picker convenience only and come from `config.json`'s `jewelleryClasses` etc.; never hardcoded in the GUI (PITFALLS section 10). The emitter never reads them.
- Base names are not validated; PoE2 warns at load.
- Keep `CHANGELOG.md` `[Unreleased]` current; update the CLAUDE.md overlay notes (they currently describe the editor as read-only for conditions and the style tag as "not applied until Phase 20").
- PoE2 filter grammar facts (CLAUDE.md): `Class "x"` bare = substring/prefix match, `==` = exact; PoE2 has no link mechanic; `MinimapIcon` size 0 largest to 2 smallest; font size 1-45; sound id 1-16, volume 0-300.
- Windows/egui constraints: window is a fixed 460px stripe; Cinzel has no symbol glyphs (paint shapes, never use arrow/cross glyphs); PoE2 must be windowed for the stripe to be visible.
- Git: milestone branch workflow; commits end with the attribution lines supplied in the session. Research file is not committed.

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| Patch one body line, add/remove a line, preserve bytes | `filter` library (`Document`) | - | Lossless guarantee lives where raw lines live; overlay must never build line text for untouched lines |
| Value codec (tokens <-> structured value), operator sets, labels, ranges, defaults, keyword groups | Pure module (egui-free), e.g. `overlay::grammar` | `filter::keywords` (source of truth for which keywords exist) | PITFALLS 8: logic in the library, presentation at the edge; unit-testable without a window |
| Style -> action lines, Show/Hide mapping | `emitter` (shared helper) | `filter` rewrite pass | One definition of "what a style emits" for generate and for rule rewrite |
| Styled-rule rewrite on Apply, unknown-style report | Library pass over `Document` (called from `overlay::apply_working`) | `WorkingFilter` | Same guarded writer as CLI; gate and rewrite stay out of egui code |
| Row view model for expanded row, RowActions, pending (empty) rows | `overlay::rules` (egui-free) | - | Phase 19 pattern: model produces rows + dispatches actions; view only emits actions |
| Controls (sliders, chips, pickers, colour swatch, popups) | `overlay` (egui, `widgets.rs` + `rule_list.rs`/new `body.rs`) | - | Presentation only |
| Class catalogue (groups) | `loader`/`models::config` read at overlay start/reload | `overlay::app` holds it | Source is `config.json`; overlay currently has no Config |
| styles.json read | `overlay::app` via `loader::load_styles` at start, reload, and Apply | - | Apply must see the file as it is now |

## Standard Stack

No new dependencies. Everything needed is in the existing lockfile.

### Core
| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| egui / eframe | 0.36.1 (Cargo.lock; 0.36.2 also in registry) | Immediate-mode UI | Already the overlay stack [VERIFIED: Cargo.lock] |
| indexmap | 2 (serde) | Ordered `StyleSet` | Already used [VERIFIED: Cargo.toml] |
| thiserror | 2.0 | `EditError` extension | Already used |
| tempfile | 3 | Hermetic apply tests | Already used |

### Supporting (egui built-ins to use)
| API | Purpose | Notes |
|-----|---------|-------|
| `egui::color_picker::color_edit_button_srgba(ui, &mut Color32, Alpha::OnlyBlend) -> Response` | Colour swatch + popup with RGBA fields | [VERIFIED: egui-0.36.1/src/widgets/color_picker.rs:543, enum Alpha :267] |
| `egui::Slider::new(&mut n, range).integer().logarithmic(true).clamping(SliderClamping::Never)` | Numeric slider; `Never` lets the value box accept out-of-range numbers while the rail clamps visually | [VERIFIED: widgets/slider.rs:59-75, :229, :290, :594] |
| `egui::DragValue::new(&mut n).range(..)` | Exact-number field | [VERIFIED: widgets/drag_value.rs:99] |
| `egui::ComboBox::from_id_salt(..).selected_text(..).show_ui(ui, ..)` | Operator / enum / colour / shape dropdowns | [VERIFIED: containers/combo_box.rs:85, :207] |
| `egui::Popup::from_toggle_button_response(&resp).close_behavior(PopupCloseBehavior::CloseOnClickOutside).width(w).show(|ui| ..)` | `+ condition` / `+ action` pickers | `popup_below_widget` no longer exists in 0.36 [VERIFIED: grep, containers/popup.rs:230, :326, :399, :508]. The default close behavior `CloseOnClick` would close the popup when the search field is clicked, so use `CloseOnClickOutside`; Esc closes by default. |
| `Response::widget_info(\|\| WidgetInfo::labeled(WidgetType::Button, true, "Remove this condition"))` | Accessible names for painted controls | [VERIFIED: response.rs:868, data/output.rs:658] |
| `Response::scroll_to_me(Some(Align::Center))`, `request_focus()` | Scroll/focus a newly added row | [VERIFIED: response.rs:841, :376] |

### Alternatives Considered
| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| `DragValue` for the number field | `TextEdit` buffer | `DragValue` silently ignores unparsable text and exposes no invalid state, so the UI-SPEC hint `Enter a whole number` cannot be shown with it. Use a small `number_field` widget (TextEdit with a draft string held in egui temp memory keyed by widget id, commit only on a successful parse) for Int controls; use `DragValue` only where no hint is needed (volume, ids). |
| Slider + separate 64px field | Slider with built-in value box (`show_value(true)`) | Built-in is simpler but its box cannot show the hint; the UI-SPEC's side-by-side layout needs `show_value(false)` + the custom field. |

**Installation:** none.

**Version verification:** egui API names above were read from `~/.cargo/registry/src/*/egui-0.36.1/src` (the locked version), not from training data.

## Package Legitimacy Audit

No external packages are added by this phase, so no install-time gate applies. slopcheck was not run (nothing to check).

**Packages removed due to slopcheck [SLOP] verdict:** none
**Packages flagged as suspicious [SUS]:** none

## Architecture Patterns

### System Architecture Diagram

```
                 styles.json ----load_styles----> StyleSet ---------------+
                 config.json ----load_config----> ClassCatalog            |
                                                    |                     |
 filter file --> LoadedFilter --> WorkingFilter(doc: Document) <-- set_styles()
                                      |   ^                               |
                                      |   | RowAction (deferred)          |
              lazy body view (one row)|   |  EditLine / AddLine /         |
                                      v   |  RemoveLine / ApplyStyle /    |
                         egui expanded_body   RemoveStyle / SetKeyword ... |
   pending (empty) rows: ListUi ------+   |                               |
                                          v                               |
                      Document patch API (validate -> commit, keep EOLs)  |
                                                                          |
 APPLY: apply_working --> style rewrite pass on a CLONE of doc <----------+
          (known style: replace action lines + keyword; unknown: report)
          --> economy op --> dynamic::apply_edited (grammar gate, guarded write)
          --> on_applied(re-parse) --> report "Updated N styled rules." / unknown summary
```

### Recommended Project Structure
```
src/filter/
├── lineedit.rs      # NEW: LinePatch ops (operator, values, whole-line replace), insert/remove body line, EOL-preserving commit
├── edit.rs          # relax Warning rejection to "no NEW diagnostics"; share commit helper
├── styled.rs        # NEW (or in dynamic.rs): rewrite_styled(&mut Document, &StyleSet) -> StyleReport
src/emitter.rs       # extract pub fn style_action_lines(&Style) -> Vec<String>; emit_style_actions calls it
src/overlay/
├── grammar.rs       # NEW egui-free: keyword groups, operators_for, labels, ranges, defaults, codec (decode/encode)
├── rules.rs         # BodyEntry view, new RowActions, pending-aware helpers, set_styles, style state per row
├── classes.rs       # NEW: ClassCatalog from Config (groups: WEAPONS/ARMOUR/JEWELLERY/FLASKS & CHARMS)
├── widgets.rs       # + field_row, remove_x_button, chip_list, number_field, colour_swatch
├── body.rs          # NEW egui: condition rows, action rows, pickers, style banner (keeps rule_list.rs from growing past ~1500 lines)
└── rule_list.rs     # expanded_body delegates to body.rs; flags renamed
```

### Pattern 1: Optimistic line patch (guards against stale line indexes)
**What:** RowActions that touch a body line carry `(BlockId, doc_line, expected_old_text)`. The model refuses with a status if the current text differs. `apply_row_actions` applies several actions per frame sequentially (src/overlay/app.rs:197), and an earlier remove/insert shifts later line indexes.
**When to use:** every `EditLine`/`RemoveLine`.
```rust
// Source: pattern derived from src/overlay/rules.rs RowAction + app.rs apply_row_actions
pub enum RowAction {
    // ...existing...
    EditLine { id: BlockId, line: usize, expect: String, patch: LinePatch },
    AddLine  { id: BlockId, kind: LineKind, text: String },   // appended after last condition / last action
    RemoveLine { id: BlockId, line: usize, expect: String },
    ApplyStyle(BlockId, String),
    RemoveStyle(BlockId),
}
pub enum LinePatch { Operator(Option<Operator>), Values(String), Whole(String) }
```

### Pattern 2: Round-trip-or-raw decoding (the no-normalization guarantee)
**What:** `decode(kw, op, tokens) -> Decoded`. A structured control is used only if `encode(decode(x)) == original value text`. Otherwise the row falls back to a generic "raw tokens" control (token chips or a text field) with the same operator dropdown. This satisfies "no read-only condition" for any odd real-world line (`Sockets >= 5GGG` variants, multi-int `ItemLevel 60 65`, `HasExplicitMod "A"` without a count) while guaranteeing a structured widget never rewrites a value the user did not change.
**Proof obligation:** a corpus test that decodes every body line of `tests/fixtures/parser/filterblade.filter` and asserts identity or raw fallback, and counts raw fallbacks (informational).

### Pattern 3: Pending (empty) rows
**What:** A condition whose value is currently empty cannot exist in the filter text (a bare `Class` line is an Error; `replace_block_lines` rejects it). Both "`+ condition` -> Class/BaseType" and "user removes the last chip" need a row that is visible but not written. Hold it in `ListUi.pending: Vec<PendingRow { block, keyword, op }>` (cleared by `clear_ids`, on collapse, on Revert/Apply/Reload). Render it after the real conditions with caption "Add an entry to create this condition"; when the first entry is added, emit `AddLine`; its X discards it. Same mechanism covers Rarity with zero chips (UI-SPEC: "not written, warning hint").
**Why:** UI-SPEC says "Selecting an item adds the row ... with a sensible default value", but list shapes (`Class`, `BaseType`, `ArchnemesisMod`, `EnchantmentPassiveNode`, `HasExplicitMod`, `HasEnchantment`) have no sensible default value. This is a gap in the contract (see Open Questions Q1).

### Pattern 4: Style rewrite on a clone
**What:** `apply_working` builds the text to write from a clone of the document with styled rules rewritten, not by mutating `working.doc`. A failed Apply (changed on disk, grammar gate) leaves the user's in-memory edits exactly as they were; `on_applied(loaded.text())` re-parses anyway on success [VERIFIED: src/overlay/rules.rs:563].
Use the same `rewrite_styled` function for the immediate `ApplyStyle` action on a single block so the row shows the style's actions at once.

### Anti-Patterns to Avoid
- **Rebuilding a line from its parsed model.** Always patch token ranges (operator range, value span) and keep indent, keyword text, spacing and trailing comment verbatim. Re-rendering a whole line is what turns `Class "x"` into `Class == "x"`.
- **Calling `replace_block_lines` with a full re-rendered body.** It re-terminates all lines with one EOL (mixed-EOL blocks change bytes) and refuses blocks with pre-existing Warnings.
- **Writing from a control every frame.** Write only on `response.changed()` / `drag_stopped` with a value different from the current one. Colour in particular: egui `Color32` is premultiplied [VERIFIED: ecolor-0.36.1/src/color32.rs:8], so converting an unchanged translucent colour through the picker can alter the bytes by +/-1. Build the swatch value with `Color32::from_rgba_unmultiplied`, read back with `to_srgba_unmultiplied()`, and never write unless `changed()`.
- **Hardcoding class groups in the GUI.** Use the catalogue from `config.json`.
- **Per-frame full-document work.** `Document::block(id)` is a linear scan and every edit re-parses the whole index; only the expanded row's view is built, once per generation.

## Edit API gap (what the current path can and cannot express)

| Capability | Today | Needed |
|------------|-------|--------|
| Replace whole owned range | `Document::replace_block_lines(id, Vec<String>)` [src/filter/edit.rs:276] | keep as the low-level primitive |
| Change one value / operator in one line | none | `LinePatch` ops |
| Insert / remove one body line | none (only `set_continue` does it by rebuilding all lines) [src/filter/structure.rs:176] | `insert_body_line`, `remove_body_line` |
| Preserve each line's own terminator | no: one `fresh_eol(header)` for every line [edit.rs:284-296] | per-line EOL kept; new line takes the neighbour's EOL |
| Edit a block that already has a Warning (unknown keyword line, trailing header tokens) | refused: `Severity::Error | Severity::Warning` filter [edit.rs:320] | allow if the candidate has **no more** Error/Warning diagnostics than the block already had (compare by count and by message), always reject new Errors |
| Disabled-block edits | token ranges are relative to the text after the first `#` [block.rs:36]; `set_continue` already computes the prefix [structure.rs:176-205] | patch ops must be prefix-aware |
| Metadata | `set_meta`, `remove_meta`, `meta_get` [edit.rs:348, :397; mod.rs] | sufficient for `#ff:style` |
| Block keyword | `set_block_keyword` touches only the keyword [structure.rs:160] | sufficient for style Show/Hide |

Notes for the implementer:
- Tokenizer gives byte ranges for `indent`, `keyword`, `operator`, `values`, `comment` [VERIFIED: src/filter/tokenize.rs:54-60]. Operators can be glued (`Rarity>=Magic`, `HasExplicitMod >=1 "A"`) so patching the operator range and the span `values.first().start .. values.last().end` is safe; adding an operator where none exists inserts `"<op> "` before the first value; removing it (back to bare) deletes the operator range plus the following run of spaces.
- `Operator` has eight members: `=`, `==`, `!`, `!=`, `<`, `<=`, `>`, `>=` [VERIFIED: tokenize.rs:12-21]. `Ops` in the table is only `None | Compare`, so per-shape operator subsets must live in the presentation module.
- Run the new validator on the candidate block with the same machinery as `replace_block_lines` (isolated one-block parse, `commit` neighbour check). Refactor that function body into a shared `commit_lines(id, Vec<(String, Eol)>)`.
- Edits must refuse embedded `\n`/`\r` (existing `EmbeddedNewline`) and the codec must reject `"` inside a quoted list entry and control characters (grammar injection).

## Value codec (egui-free, `overlay::grammar`)

Source of truth for *which* keywords exist and their shapes: `filter::keywords::KEYWORDS` (80 entries: 5 structural, 61 conditions, 14 actions) [VERIFIED: keywords.rs table, test `documented_vocabulary_present`]. Shapes: `Bool`, `Int`, `QuotedList`, `Enum`, `Rarity`, `Sockets`, `CountThenNames`, `BoolOrName`, `OptionalBool`, `Rgba`, `SoundId`, `CustomSound`, `MinimapIcon`, `PlayEffect`, `FontSize` [keywords.rs:27-45].

| Shape | Decode to | Encode | Fallback to raw when |
|-------|-----------|--------|----------------------|
| Int | `Vec<i64>` (1 = slider; 2+ = chip list of ints) | space-joined | any token not `i64` (cannot happen after `validate_shape`) |
| Bool | `bool` | `True`/`False` (keep original case if unchanged) | n/a |
| Rarity | set of {Normal,Magic,Rare,Unique} | scalar if one chip, list form if several; operator fixed (none/`=`/`==`) for list form | unknown word |
| QuotedList (Class, BaseType, ArchnemesisMod, EnchantmentPassiveNode) | `Vec<String>` unquoted | `"A" "B"` | entry contains `"` |
| CountThenNames | `Option<u32>` + `Vec<String>` | `N "A" "B"`; operator glued/spaced as in the original | first token not int and not quoted |
| Enum | `Vec<String>` from the table list | joined | word not in list |
| Sockets | `count: u8` + colour slots `[R,G,B,W,A,D]` | `5GGG` shape | token where letters count exceeds digits, or any letter outside `RGBADW`; exact semantics of the digit/letter mix are [ASSUMED] (see A3) |
| BoolOrName | Any / Named(String) | `True`/`False`/quoted name | n/a |
| Rgba | `[u8;4]` (alpha default 255 when 3 tokens; keep 3-token form unless alpha edited) | `R G B A` | n/a |
| FontSize | `i64` | integer | n/a |
| SoundId | `Id(u8)` or `None` + `Option<vol>` | `id vol` | n/a |
| CustomSound | name + `Option<vol>` | `"name" vol` | name contains `"` or control char |
| MinimapIcon | `Hidden` (`-1`, up to 2 trailing tokens) or `(size, colour, shape)` | space-joined | n/a |
| PlayEffect | `None` or `(colour, temp)` | `Colour [Temp]` | n/a |
| OptionalBool | `Absent`/`True`/`False` | keep the trailing token only if it was there (UI-SPEC) | n/a |

Operator sets for presentation (suggested; the UI-SPEC label table is authoritative for words):
- Int / FontSize-like numeric: `>=`, `<=`, `>`, `<`, `==`, `!=`; plus the file's existing `=`, `!` or absent operator (shown, never offered otherwise).
- Rarity (scalar): same numeric set (ordinal). Rarity (list form): fixed.
- QuotedList / CountThenNames / Enum: absent or `=` ("Matches"), `==` ("Matches exactly"), `!=`/`!` ("Not").
- Bool: absent/`=`/`==` ("Is") and `!=` ("Is not", only if present).
- Actions: no operator (`Ops::None`); the dropdown is not rendered.

Defaults when adding a condition (Claude's discretion; recommended): Int `>=` with the keyword-specific default from the UI-SPEC (ItemLevel 65, AreaLevel 65, Quality 1, Sockets 1, StackSize 1, WaystoneTier 1, defences 1); Rarity `==` Rare... **but** do not pre-select a rarity (pending row, zero chips) to avoid silently adding restrictive rules; Bool `== True`... use bare `True` to match the emitter (`Corrupted True` is what `emit_rules` writes) [VERIFIED: src/emitter.rs:446-448]; Class bare, BaseType `==` (matches the emitter's `BaseType ==` form per CLAUDE.md).

Keyword groups (UI-SPEC table) are presentation data in `grammar.rs`; add a unit test that **every condition keyword in `KEYWORDS` is reachable** (in a named group or `Other`) and every grouped name exists in `KEYWORDS`. Cross-check done against the table: all names in the UI-SPEC groups exist; `AlwaysShow`, `ElderItem`, `ShaperItem`, `SynthesisedItem`, `Scourged`, `HasCruciblePassiveTree`, `HasEaterOfWorldsImplicit`, `HasSearingExarchImplicit`, `BlightedMap`, `ShapedMap`, `UberBlightedMap`, `MirageMap`, `ElderMap` fall to `Other` or the legacy-map entry of Endgame.

`(PoE 1)` picker suffix: the keyword table itself does not carry PoE1/PoE2 applicability. CLAUDE.md states only that `LinkedSockets` has no PoE2 meaning and says to "verify PoE2 applicability" for others. A static `POE1_ONLY` list in `grammar.rs` is [ASSUMED]; keep it short (LinkedSockets, ElderMap, ShapedMap, BlightedMap, UberBlightedMap, MirageMap, ElderItem, ShaperItem, SynthesisedItem, Scourged, HasCruciblePassiveTree, HasEaterOfWorldsImplicit, HasSearingExarchImplicit, ArchnemesisMod) and never hide a keyword.

## Overlay model

Current state [VERIFIED]:
- `RowAction` enum: SetEnabled, Move, Delete, Duplicate, SetKeyword, SetContinue, SetName, ClearName, AddRule [src/overlay/rules.rs:34-45]. All dispatched in `WorkingFilter::apply_action` [rules.rs ~390-480]; rows rebuilt once per mutation (`rebuild`).
- `RuleRow` carries `style_tag: Option<String>` (display only) and `conditions: Vec<String>` (display strings) [rules.rs:62-85].
- `rule_list.rs`: `flags_for` (flag `style wins` at :109), `draw_row`, `expanded_body` (:354; NAME, ACTION + Continue, read-only CONDITIONS with the text "Editing conditions arrives in a later update.", Duplicate/Delete). Body frame margin symmetric(10, 8); measured-height cache `expanded_h` updated with a one-frame repaint [rule_list.rs:343-345].
- `ListUi` holds expanded, confirm_delete, name_open, name_drafts, expanded_h, pending_scroll, confirm_economy_remove; `clear_ids()` resets id-keyed state after Revert/Apply/Reload [rule_list.rs:21-55].

Changes needed:
1. `WorkingFilter` gains `styles: StyleSet` (`set_styles`, triggers `rebuild`) and per-row `style: StyleState { None, Known(name), Unknown(name) }` replacing the bool-ish `style_tag` use. The ACTION lock must depend on `Known` only (UI-SPEC: unknown-style rules stay fully editable).
2. `WorkingFilter::body_view(id) -> Vec<BodyEntry>` (lazy; called for the one expanded row per frame, cached by `generation`). `BodyEntry { line, text, kind: Condition|Action|Modifier|Opaque, keyword: Option<&'static Keyword>, op: Option<(Operator)>, values: Vec<String> /*raw token text*/, comment: Option<String>, prefix_len }`. Opaque covers unknown keywords (Warning) and `Bad` lines (`keyword: None, tokens: None`) [block.rs:36-43; parse.rs scan_live_body]: show as a raw-text row with Remove only (they are not editable meaningfully, but they are visible, not hidden).
3. Row actions listed in Pattern 1. `apply_row_actions` in app.rs needs no structural change; after `AddLine` it should set `pending_scroll` / focus.
4. Interleaved conditions and actions: render groups by `kind` but in file order within each group; insert new conditions after the last condition line (or directly after the header), new actions after the last action line (or at the end of the body), copying indent from the first body line like `set_continue` does [structure.rs:176-205].
5. Keep one pass per frame over a handful of lines; do not widen `RuleRow` for all 511 rows. Existing perf test: load + 100 mixed edits on the 511-block fixture runs in about 1.9 s in debug (`tests/overlay_rules.rs`), i.e. roughly 10-15 ms per edit in debug. A slider emits an edit per changed frame, so in release this is fine; in debug it may stutter. Mitigation if needed: coalesce multiple `EditLine` actions to the same line within one frame (keep the last) and/or commit slider edits on `drag_stopped` plus live preview in the widget only.
6. Class catalogue: `ClassCatalog::from_config(&Config)` reproducing v0.4's groups (`WEAPONS`, `ARMOUR`, `JEWELLERY`, `FLASKS & CHARMS`, empty groups omitted) [git: 911666f^:src/overlay/edit.rs `class_groups`]. Loaded at overlay start and on RELOAD via `loader::load_config`. If config.json is unreadable, degrade to the generic quoted-list chip control (never block the editor); show the error in the outcome area.
7. Classes selected in the file but absent from the catalogue (foreign filters, custom classes) must still display and be removable: render them as chips above the picker; the grouped checkboxes only toggle catalogue entries.

## Styles (STYLE-03, EDIT-06)

APIs [VERIFIED]: `StyleSet::get(&str) -> Option<&Style>`, `names()`, `len()`; `Style { text, border, background: Option<[u8;3]>, font: Option<u8>, minimap, beam, sound, action: StyleAction(Show|Hide), disable_drop_sound }` [src/models/config.rs:91-102, :219-247]. `loader::load_styles(path)` validates [src/loader.rs:190]. Alpha is not stored in styles; the emitter writes 255 [src/emitter.rs:209-262]. Emission order is fixed: PlayEffect, MinimapIcon, SetTextColor, SetBackgroundColor, SetBorderColor, SetFontSize, Custom/PlayAlertSound, DisableDropSound.

Design:
1. `emitter::style_action_lines(&Style) -> Vec<String>` (no indent). `emit_style_actions` becomes `for l in lines { writeln!(out, "  {l}") }`. `generate` output must remain byte-identical (run `artifact_guard`, `generate_snapshot`).
2. `rewrite_styled(doc: &mut Document, styles: &StyleSet) -> StyleReport { rewritten: usize, unknown: Vec<(rule_index, name)> }`: for each **enabled, non-economy** block with `meta_get(id, "style") = Some(name)`:
   - `Known`: set keyword to `style.action` (`set_block_keyword`), replace **all `Kind::Action` body lines** by the style's lines placed at the first action line's position (or after the last body line if none); leave conditions, `Continue` and opaque lines alone. If the existing action lines already equal the style's lines semantically (same keyword + token values, ignoring whitespace), do nothing, so a second Apply writes `Unchanged` and does not normalize whitespace of an already-correct rule.
   - `Unknown`: no change; counted in the report.
   - The economy section blocks are skipped (generated, never tagged).
3. UI actions: `ApplyStyle(id, name)` = `set_meta(id, "style", name)` + `rewrite_styled` for that block; `RemoveStyle(id)` = `remove_meta(id, "style")` only (actions stay, per UI-SPEC). Both refuse a style name where `name != name.trim()`, empty, or containing control characters, because `set_meta` trims and `parse_ff_line` trims the read side [meta.rs:21-35], so such a name would read back as Unknown. `StyleSet::validate` does not forbid them [config.rs:262-285]; add the guard at the call site and a note for Phase 21.
4. Apply hook: in `apply_working` [app.rs:132] before `dynamic::apply_edited(loaded, &working.text(), op)`: `let (text, report) = working.text_with_styles(&styles)`; pass `text` to `apply_edited`; on success add `" Updated N styled rules."` to `extras` (omit if 0) and `" N rules use unknown styles (first: rule K)."`-style summary. Re-read `styles.json` at this point (fresh `load_styles`) so hand edits are picked up; on a load error, refuse the rewrite with the `StyleError` text and still allow Apply of un-styled changes? Recommended: refuse Apply with the styles error (consistent with STYLE-02, an invalid styles.json is a clear error).
5. Stale indicator (UI-SPEC: changed `styles.json` counts as unapplied): compute `would_change = text_with_styles(&styles) != working.text()` cached by `(generation, styles revision)`; fold into the existing dirty/indicator logic (`apply.rs` `ApplyState`). Do not run it every frame uncached (clone + rewrite of a 250 KB document).
6. Duplicate of a styled rule copies the tag (leading comment run is part of the span) [ASSUMED from `Block::span`, verify in a test].
7. `Remove style` + `+ action` hidden while `Known`: model-level, not only UI-level: `AddLine(kind=Action)` on a Known-styled block should be refused by `apply_action` with a status, so a bug in the view cannot create actions that vanish on Apply.
8. A duplicated `#ff:style` line: `meta_get` returns the first, `set_meta` rewrites the first, `remove_meta` removes all [edit.rs:348-420]. `RemoveStyle` therefore always clears fully.

Where Phase 19's display-only indicator lives (to be replaced): `rule_list.rs:109-116` (flag `style wins`, tooltip), `:401` (`locked = row.style_tag.is_some()`), `:439-446` (note under ACTION), test `expanded_style_tagged_row_renders_without_actions` at `:766`, `rules.rs:79,286,330,665` (field, build, test).

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Colour picker with alpha | custom RGBA widget | `egui::color_picker::color_edit_button_srgba(.., Alpha::OnlyBlend)` | Includes popup, RGBA drag fields, copy button, HSV cache that avoids drift [color_picker.rs:405-445] |
| Numeric slider with log scale and out-of-range entry | custom slider | `Slider` + `logarithmic(true)` + `SliderClamping::Never` | Already handles zero-including log ranges |
| Dropdowns / popups with Esc and outside-click | custom overlay windows | `ComboBox`, `Popup` | Focus, Esc, repositioning inside the viewport |
| Line tokenization and shape validation | a second parser for edited lines | `filter::tokenize::{tokenize, validate_shape}` | The candidate block is validated by the same code the loader uses |
| Block round-trip safety (neighbour checks, id preservation) | new commit logic | existing `commit` (edit.rs) via a shared helper | Four review iterations hardened it (16-REVIEW*, 19-REVIEW*) |
| Quoted-list entry splitting | new parser | `overlay::text_entry::parse_base_type_entry` | Preserved with tests in Phase 19-01 |
| Meta tag handling | direct line editing | `Document::{set_meta, remove_meta, meta_get}` | Handles insert position, trimming, validation |

**Key insight:** every "obvious" shortcut here (re-render the line, re-render the body, normalize operators, normalize colours through a float round-trip) violates PARSE-04. The cheap, safe primitive is "replace a byte range inside one line and revalidate the block".

## Runtime State Inventory

Not a rename/refactor/migration phase. Existing user data note only: a user's existing filter may already carry `#ff:style` lines written by hand (Phase 19 displayed them, nothing applied them). After this phase the first Apply will rewrite every styled rule's actions. That is the specified behavior but is a one-time visible change in a real filter; the outcome line `Updated N styled rules.` is the disclosure. No stored data, services, OS registrations or secrets are affected; no build artifacts carry a renamed string.

## Common Pitfalls

### Pitfall 1: A block with a pre-existing Warning cannot be edited at all
**What goes wrong:** `replace_block_lines` (and therefore `set_continue`, `set_block_keyword`) returns `InvalidBlock` for any enabled block whose candidate parse yields a Warning, even when the warning is an untouched unknown-keyword line or trailing header tokens [VERIFIED: edit.rs:320, parse.rs:165-170, :290-295].
**Why:** D-11 "an enabled block the tool writes must be fully valid".
**How to avoid:** compare against the block's current diagnostics: reject only diagnostics that are new. Add fixture tests on a block with `FutureKeyword x`.
**Warning signs:** editing a NeverSink rule that contains a non-table keyword reports "would produce an invalid enabled block".

### Pitfall 2: Mixed line endings change on edit
**What goes wrong:** `replace_block_lines` gives every line of the block `fresh_eol(header)`. A block with mixed `\r\n`/`\n` loses that on any edit, breaking "only the edited value changes".
**How to avoid:** the new patch/insert/remove ops carry each line's own `Eol`; inserted lines take the previous line's `Eol`. Test with a CRLF fixture block and a mixed block.

### Pitfall 3: Colour round-trip drift
**What goes wrong:** `Color32` is premultiplied; HSV picker conversions can shift components by 1.
**How to avoid:** write only on `changed()`; construct with `from_rgba_unmultiplied`; test that a no-interaction frame writes nothing and that opening the picker without edits leaves bytes unchanged. For alpha < 255 a user edit may differ by +/-1 from the typed value; accept, but prefer the RGBA drag fields over the HSV area in tests.

### Pitfall 4: Stale line indexes across actions in one frame
**What goes wrong:** two actions queued in one frame (e.g. remove a line then edit the next) address shifted lines.
**How to avoid:** `expect` guard (Pattern 1); apply in order; refuse with a status instead of editing the wrong line.

### Pitfall 5: Drafts keyed by line index
**What goes wrong:** text drafts (add-entry buffers, number fields) keyed by `(BlockId, line)` point at the wrong line after add/remove.
**How to avoid:** key by `(BlockId, kind, keyword occurrence ordinal)` or keep the draft in egui temp memory under the widget's own id inside `push_id((row.id, entry_ordinal))`; clear on `clear_ids`.

### Pitfall 6: Empty values produce invalid grammar
**What goes wrong:** removing the last Class or BaseType entry, or adding a list condition, would create `Class` with no values (an Error, which blocks Apply).
**How to avoid:** removal of the last entry removes the line; additions go through a pending row (Pattern 3). Every encode function returns `None` for an empty value, and `EditLine` with `None` becomes `RemoveLine`.

### Pitfall 7: Unknown style hidden by an unrelated failure
**What goes wrong:** if `styles.json` fails to load at overlay start, every styled rule would look "Unknown".
**How to avoid:** distinguish "styles unavailable" (one banner message, no per-rule unknown flags, rewrite skipped, Apply refused with the load error) from "style name not defined".

### Pitfall 8: Nested vertical scroll areas inside the virtualized list
**What goes wrong:** the UI-SPEC caps the class picker (200px), chip lists (160px) and the condition popup (320px) with their own scroll areas inside the outer `show_viewport` list. Mouse-wheel capture between nested areas can feel sticky; heights that change as the user toggles things feed `expanded_h`.
**How to avoid:** use natural-height `CollapsingHeader` groups (as v0.4 did) rather than a capped scroller for the class picker; cap only the chip lists with `ScrollArea::max_height`; verify wheel behavior in manual UAT (confidence on egui's nested wheel handling: LOW, not verified).

### Pitfall 9: Popups and virtualization
**What goes wrong:** a popup owned by a row that scrolls out of the drawn range is no longer shown; the open-state memory persists.
**How to avoid:** close popups when the expanded row changes (`list.expanded` change) and on `clear_ids`; rows are keyed by `push_id(("rule", id))` already.

### Pitfall 10: Style rewrite drops legitimately different but equal actions every Apply
**What goes wrong:** comparing action lines textually makes whitespace/ordering differences rewrite the block on every Apply (never `Unchanged`).
**How to avoid:** compare semantically (multiset of normalized `keyword + tokens`), or compare to the canonical lines after normalization; assert idempotence (second Apply = `Unchanged`).

### Pitfall 11: Scope creep into Phase 21
`styles.json` editing, the config tab and stale detection of in-process style edits belong to Phase 21. Phase 20 only reads `styles.json` and reacts to file changes at Apply time.

## Code Examples

### Patch only the values of one line (ranges from the tokenizer)
```rust
// Source: derived from src/filter/tokenize.rs TokenizedLine (ranges are byte offsets into the line text)
fn patch_values(line: &str, t: &TokenizedLine, new_values: &str) -> String {
    match (t.values.first(), t.values.last()) {
        (Some(first), Some(last)) => {
            format!("{}{}{}", &line[..first.start], new_values, &line[last.end..])
        }
        _ => {
            // no values yet: insert after the operator (or keyword) with one space
            let at = t.operator.as_ref().map(|(r, _)| r.end)
                .or_else(|| t.keyword.as_ref().map(|r| r.end)).unwrap_or(line.len());
            format!("{} {}{}", &line[..at], new_values, &line[at..])
        }
    }
}
```
For disabled blocks add the prefix offset (`text.len() - uncomment_once(text).len()`) as `set_continue` does.

### Colour action row
```rust
// Source: egui-0.36.1 widgets/color_picker.rs:543
let [r, g, b, a] = rgba;                       // from the tokenized line, alpha 255 if 3 tokens
let mut c = egui::Color32::from_rgba_unmultiplied(r, g, b, a);
let resp = egui::color_picker::color_edit_button_srgba(ui, &mut c, egui::color_picker::Alpha::OnlyBlend);
if resp.changed() {
    let [r, g, b, a] = c.to_srgba_unmultiplied();
    actions.push(RowAction::EditLine { id, line, expect, patch: LinePatch::Values(format!("{r} {g} {b} {a}")) });
}
```

### Slider + exact field (no clamping of typed values)
```rust
// Source: egui-0.36.1 widgets/slider.rs (clamping :290, logarithmic :229, integer :594)
let mut v = value;                              // i64 from the codec
let resp = ui.add(egui::Slider::new(&mut v, 1..=100)
    .integer().show_value(false).clamping(egui::SliderClamping::Never));
if resp.changed() { /* emit EditLine once; use resp.drag_stopped() if debug-mode lag matters */ }
```

### Popup picker
```rust
// Source: egui-0.36.1 containers/popup.rs:230, :326, :399, :508
let btn = ui.button("+ condition");
egui::Popup::from_toggle_button_response(&btn)
    .close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside)
    .width(ui.available_width())
    .show(|ui| { /* search field + CollapsingHeader groups; on pick: push action, egui::Popup::close_all(ui.ctx()) or set memory closed */ });
```
(Confirm the exact close call in the egui source when implementing; `Popup::close_all` is referenced in the `PopupCloseBehavior::IgnoreClicks` doc, containers/popup.rs:88.)

## State of the Art

| Old Approach | Current Approach | Impact |
|--------------|------------------|--------|
| v0.4: edit `Rule` structs in `dynamic.json`, fixed control set, off-switch per condition | v0.5: edit parsed filter lines; data-driven rows; add/remove rows | Controls are chosen by `ValueShape`; no per-condition model code |
| egui `popup_below_widget` | `egui::Popup` builder (0.36) | Use `Popup::from_toggle_button_response`/`menu` |
| `ComboBox::show_ui` closure | unchanged name in 0.36.1 | Use `selectable_value` inside |

**Deprecated/outdated:** v0.4 `overlay::edit`, `rule_editor`, `view_model` modules are deleted (Phase 19-01); do not try to restore them wholesale, only the group logic.

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | Disabled styled rules are rewritten like enabled ones (or are skipped until enabled); UI-SPEC is silent. Recommend skipping rewrite on disabled blocks and letting the first Apply after enabling rewrite them. | Styles | A disabled styled rule shows stale actions until enabled + Apply |
| A2 | `Document::duplicate_block` copies the `#ff:style` leading line | Styles #6 | Duplicate would silently lose its style; add a test |
| A3 | Socket token semantics: `Sockets >= 5GGG` = at least 5 sockets, including at least GGG; `SocketGroup RGB` colours of a linked group; PoE2 has no links so `SocketGroup`/`LinkedSockets` may be inert | Value codec | Wrong structured rendering; mitigated by the round-trip-or-raw rule |
| A4 | The PoE1-only keyword list for the `(PoE 1)` suffix | Value codec | Cosmetic mislabel only |
| A5 | Nested scroll areas feel acceptable in egui 0.36 | Pitfall 8 | UX friction; fall back to natural-height groups |
| A6 | Typing a value past the slider range via the number field is accepted by the game (e.g. Quality 40) | UI-SPEC | PoE2 warns at load; no action needed |
| A7 | Numeric ranges and defaults come from the UI-SPEC (author-approved) not from game data | Value codec | Only affects slider feel |

## Open Questions

1. **List-shaped conditions have no sensible default value (UI-SPEC gap).**
   - What we know: UI-SPEC says picking a condition adds a row "with a sensible default value"; a `Class`/`BaseType` line with no value is an Error.
   - What's unclear: whether to pre-fill (e.g. first class) or add a visible unwritten row.
   - Recommendation: pending-row mechanism (Pattern 3); nothing is written until the first entry; documented to the user by the caption.

2. **Invalid (cannot-enable) disabled blocks and opaque lines.**
   - What we know: disabled-invalid blocks and unknown-keyword lines exist in real filters; the author bar is "no read-only condition".
   - Recommendation: unknown-keyword/`Bad` lines display as raw rows with Remove only; disabled-invalid blocks show their lines as raw read-only rows with the existing "cannot enable" explanation (editing commented prose is not meaningful). Valid-disabled blocks are fully editable via prefix-aware patches. Record this as the accepted exception.

3. **Label for an absent operator on numeric/bool conditions.**
   - Recommendation: `Exactly (no operator)` when the line has none, `Exactly (=)` for bare `=`; never auto-normalize. Both appear only if already present.

4. **Number-field invalid hint vs `DragValue`.**
   - Recommendation: build a small `number_field` (TextEdit + temp-memory draft, commit on parse) so the specified hint works; `DragValue` elsewhere.

5. **When to re-read `styles.json`.**
   - Recommendation: at start, RELOAD, and every Apply; optional mtime poll on stripe show. Phase 21 will add in-process edits.

6. **Style names with surrounding whitespace/control characters.**
   - Recommendation: guard at ApplyStyle/rewrite; add to Phase 21 validation backlog.

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| Rust toolchain / cargo | build, tests | yes | edition 2024 project builds (full suite ran) | - |
| egui sources in registry | API verification | yes | 0.36.1 and 0.36.2 | - |
| PoE2 game / windowed mode | manual UAT only | not probed | - | UAT items remain `human_needed` like Phases 18-19 |

Missing dependencies with no fallback: none.

## Validation Architecture

### Test Framework
| Property | Value |
|----------|-------|
| Framework | Rust built-in `cargo test` (unit in `src/**`, integration in `tests/*.rs`), `insta` snapshots, `tempfile`, headless egui via `egui::Context::run_ui` |
| Config file | none (Cargo.toml) |
| Quick run command | `CARGO_TARGET_DIR=C:/Users/jonat/AppData/Local/Temp/ff-target cargo test --lib filter:: overlay::` |
| Full suite command | `CARGO_TARGET_DIR=C:/Users/jonat/AppData/Local/Temp/ff-target cargo test` then `cargo fmt --check` and `cargo build --bins` (warning-free) |

**Baseline measured 2026-10-12:** full suite green in about 34 s: 498 lib tests; integration: cli_errors 12, cli_filter_copy 4, cli_generate 16 (includes `artifact_guard`/`generate_snapshot`), dynamic_filter 15, filter_roundtrip 18, filter_structure 8, overlay_rules 2, sync_prices 8; 1 doc test. Protected byte-identical artifacts: `tests/fixtures/seed-universal.filter`, `tests/snapshots/cli_generate__generate_snapshot.snap`, `tests/fixtures/parser/filterblade.filter` (CRLF, `-text`).

### Phase Requirements -> Test Map
| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| EDIT-01 | Patch operator/values of any condition; add/remove condition; byte-exact no-op patch on every condition line of FilterBlade; disabled-block and CRLF/mixed-EOL variants; block with pre-existing Warning is editable | unit + integration | `cargo test --lib filter::lineedit` / `cargo test --test filter_roundtrip` | no, Wave 0 |
| EDIT-01 | Codec round-trip identity or raw fallback for every FilterBlade body line; keyword groups cover all 61 conditions; operator sets per shape | unit/integration | `cargo test --lib overlay::grammar` | no, Wave 0 |
| EDIT-01 | Pending-row lifecycle; empty list removes line; stale `expect` refused | unit | `cargo test --lib overlay::rules` | partly (file exists) |
| EDIT-02 | Action line patches for Rgba/FontSize/SoundId/CustomSound/MinimapIcon/PlayEffect/OptionalBool; unchanged picker frame writes nothing | unit + headless egui | `cargo test --lib overlay::` | no, Wave 0 |
| EDIT-06 | `ApplyStyle` sets tag + actions + Show/Hide; `RemoveStyle` keeps actions; refuses bad names; refuses `+ action` on a Known-styled block | unit | `cargo test --lib overlay::rules` | no, Wave 0 |
| STYLE-03 | Change a style in a temp `styles.json`, Apply, actions updated; second Apply `Unchanged`; unknown style reported and untouched; economy blocks skipped; `generate` unchanged | integration (tempdir, like `apply_working_*` tests in app.rs:1081+) | `cargo test --lib overlay::app` + `cargo test --test cli_generate` | partly |
| PARSE-04 (guard) | No normalization: `Class "x"` stays; only edited line differs (diff-line count == 1) | integration | `cargo test --test filter_roundtrip` | extend |

### Sampling Rate
- **Per task commit:** `cargo test --lib filter:: overlay::` (about 12 s)
- **Per wave merge:** full `cargo test` + `cargo fmt --check` + `cargo build --bins`
- **Phase gate:** full suite green, `git diff --exit-code tests/fixtures tests/snapshots`, then `/gsd-verify-work`

### Wave 0 Gaps
- [ ] `src/filter/lineedit.rs` tests: patch/insert/remove, EOL preservation, disabled prefix, Warning tolerance
- [ ] `src/overlay/grammar.rs` tests: groups reachability, operator sets, codec identity (uses the FilterBlade fixture via `include_str!`)
- [ ] fixture-level no-op edit sweep in `tests/filter_roundtrip.rs` (patch every condition/action line with its own value; expect identical bytes)
- [ ] a styled-fixture (small) in `tests/fixtures/` for rewrite tests, or build text inline
- [ ] manual UAT list (human_needed): stripe layout at 460px of every control, popup behavior, colour picker, wheel behavior of nested scrolling, a real PoE2 reload with an edited filter

## Security Domain

`security_enforcement` is not disabled in config; the surface is local (no network in this phase), but user-typed text becomes filter-file content.

### Applicable ASVS Categories
| ASVS Category | Applies | Standard Control |
|---------------|---------|-----------------|
| V2 Authentication | no | - |
| V3 Session Management | no | - |
| V4 Access Control | no | local single-user tool |
| V5 Input Validation | yes | codec rejects `"` and control characters in quoted entries and sound names; edit API rejects `\n`/`\r` (`EmbeddedNewline`); validate-then-commit with the shared tokenizer; style names guarded |
| V6 Cryptography | no | - |

### Known Threat Patterns
| Pattern | STRIDE | Standard Mitigation |
|---------|--------|---------------------|
| Filter-grammar injection through a typed value (quote, newline, `#`) | Tampering | Encode functions refuse; candidate block re-parsed, new Errors rejected |
| Metadata injection through a style name containing a newline | Tampering | `set_meta` already rejects line breaks; add whitespace/control guard |
| Corrupting the user's real filter on a bad rewrite | Tampering/Repudiation | Rewrite on a clone; grammar gate; guarded atomic write with `.ffbak` and changed-on-disk refusal (existing) |
| Writing outside the game folder | Elevation | unchanged: Apply uses the existing writer |

## Suggested Plan Decomposition (waves)

**Wave 1 - library, egui-free (parallelizable: 20-01 and 20-02 are independent; 20-03 depends only on 20-01 for the keyword/line ops)**
- **20-01 Line-patch API** (`filter`): `LinePatch`, `insert_body_line`, `remove_body_line`, per-line EOL, Warning tolerance, disabled prefix. Fixture sweep proving no-op identity on FilterBlade. Risk: HIGH (touches the lossless core; reuse `commit`, keep 16-REVIEW lessons).
- **20-02 Grammar module** (`overlay::grammar`): keyword groups + reachability test, operator sets/labels, ranges/defaults, codec with round-trip-or-raw, `POE1_ONLY`. Risk: MEDIUM (many shapes; sockets/mods semantics assumed).
- **20-03 Styles library**: `emitter::style_action_lines` refactor (artifact_guard green), `rewrite_styled` + report, `WorkingFilter::set_styles`, `StyleState`, `ApplyStyle`/`RemoveStyle` actions, `text_with_styles`, Apply hook + `Updated N styled rules.` + unknown summary, stale detection. Risk: MEDIUM (idempotence, clone-vs-mutate, styles load errors).

**Wave 2 - overlay model and controls (depends on Wave 1)**
- **20-04 Row model**: `body_view`, new `RowAction`s, pending rows, `ClassCatalog` loader, `expect` guard, tests. Risk: MEDIUM.
- **20-05 Widgets and controls**: `field_row`, `remove_x_button` (24px target, `widget_info`), `chip_list`, `number_field`, colour swatch, rarity chips, class picker, sockets, mods; headless-egui tests that an idle frame emits no actions. Risk: MEDIUM-HIGH (egui layout in 424px, popups).

**Wave 3 - assembly and polish (depends on Wave 2)**
- **20-06 Body assembly**: new `expanded_body` (NAME, STYLE banner, CONDITIONS, ACTIONS with disposition, row actions), `+ condition`/`+ action` popups, styled lock/unknown banner, `styled: <name>` / `unknown style` flags, remove "arrives in a later update" text, update `expanded_h` behavior. Risk: HIGH (largest UI file; split into `body.rs`).
- **20-07 Carry-over + docs**: single capped outcome area (96px), flag budget (`+N` badge), status order, spacing/contrast tokens (`WARM_ACCENT_TEXT`), 24px targets; CLAUDE.md overlay note, CHANGELOG `[Unreleased]`, REQUIREMENTS ticks; `16-HUMAN-UAT`-style `20-HUMAN-UAT.md`. Risk: LOW-MEDIUM. This plan can be cut or deferred without blocking the four requirements; the UI-SPEC marks these as "in scope where Phase 20 touches the code".

Cross-cutting risk notes: (a) per-edit whole-document re-index cost with sliders in debug builds; (b) file size of `rule_list.rs` (799 lines now); (c) everything depends on the round-trip-or-raw rule being enforced in tests, not by review; (d) Phase 21 will need the style-owned rewrite API stable, so keep `rewrite_styled` pure and public.

## Sources

### Primary (HIGH confidence)
- Repository source read in this session: `src/filter/{keywords,tokenize,block,lines,meta,mod,edit,structure,parse,disabled}.rs`, `src/overlay/{rules,rule_list,app,apply,widgets,theme,text_entry}.rs`, `src/dynamic.rs`, `src/emitter.rs`, `src/models/config.rs`, `src/loader.rs`, `Cargo.lock`, `config.json`
- Phase docs: `20-CONTEXT.md`, `20-UI-SPEC.md`, `19-0[1-7]-SUMMARY.md`, `16-0[1-4]-SUMMARY.md`, `17-0[1-4]-SUMMARY.md`, `19-UI-REVIEW.md`, `ROADMAP.md`, `REQUIREMENTS.md`
- egui 0.36.1 sources in `~/.cargo/registry/src/*/egui-0.36.1/src` (color_picker, slider, drag_value, combo_box, popup, response, data/output) and `ecolor-0.36.1/src/color32.rs`
- Local `cargo test` run (baseline counts)
- Git history: `911666f^:src/overlay/edit.rs` (class_groups, set_class), `911666f^:src/overlay/rule_editor.rs` (v0.4 controls)

### Secondary (MEDIUM confidence)
- none beyond the above (no web lookup was needed; the GGG grammar page was already captured in Phase 16 research)

### Tertiary (LOW confidence)
- PoE1/PoE2 keyword applicability and socket-string semantics (A3, A4); egui nested-scroll behavior (A5)

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH, no new packages; API names read from the locked egui source
- Architecture: HIGH for the edit-API gap, style hook and model changes (cited); MEDIUM for UI composition at 460px
- Pitfalls: HIGH for 1-7 (verified in code), MEDIUM/LOW for 8-9 (egui behavior unverified)

**Research date:** 2026-10-12
**Valid until:** 2026-11-12 (stable; revisit if egui is bumped to 0.36.2+, since Cargo.lock is git-ignored and may resolve newer)
