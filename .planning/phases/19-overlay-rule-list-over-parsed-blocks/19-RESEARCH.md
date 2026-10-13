# Phase 19: Overlay Rule List Over Parsed Blocks - Research

**Researched:** 2026-10-12
**Domain:** Rust, eframe/egui 0.36 (immediate-mode) editor over the lossless `filter::Document` (Phases 16 and 18)
**Confidence:** HIGH (everything below was read from the repo or measured in this session; egui API checked in the vendored 0.36.2 source)

## Summary

The library layer already provides most of what the rule list needs: a lossless `Document` with stable `BlockId`s, `#ff:` metadata read/write (`meta_get`, `set_meta`, `remove_meta`), per-block enable/disable (`set_enabled`), whole-block replacement (`replace_block_lines`), the economy section as text-in/text-out functions (`filter::section`), and a guarded single writer (`dynamic::apply_file`). **No new dependency is needed.** The overlay currently has only a status block and footer (Phase 18 deleted the rule list); `overlay::edit`, `overlay::rule_editor` and `overlay::view_model` are v0.4 modules built on `models::config::Rule` (JSON store) and **do not map onto parsed blocks**.

Four things the CONTEXT/UI-SPEC assume exist but **do not**, and must be built in this phase: (1) `Document` has no insert / delete / duplicate / move-block operations (`next_id` is marked `allow(dead_code)` "for block insertion in later phases"); (2) `dynamic::apply_file` can only apply an `ApplyOp` to `loaded.text`, i.e. it cannot write an *edited* document; (3) there is no "refuse Apply when an enabled block has grammar errors" check anywhere (CONTEXT says "from Phase 18" but it was not built); (4) nothing consumes the `#ff:style` tag on Apply (STYLE-03 is Phase 20), so the "style wins" flag in this phase is purely informational.

Performance is a non-issue for the data layer: measured on the committed 258 KB FilterBlade/NeverSink fixture (511 blocks: 460 enabled, 51 commented-out), a full `parse` is 1.7 ms release / 6.5 ms debug, `set_enabled` / `set_meta` (which rebuild the whole index) are the same, `serialize` 0.2-0.4 ms. So "re-parse after every edit" is acceptable. Rendering cost is the real risk, handled by `ScrollArea::show_viewport` with a cached offset table.

**Primary recommendation:** Add a small set of whole-block ops to `filter::Document` (`insert_block`, `remove_block`, `duplicate_block`, `move_block`, `set_block_keyword`, `set_continue`) tested against the FilterBlade fixture; add a pure (egui-free) `overlay::rules` module holding `WorkingFilter` (Document + applied baseline + cached `RuleRow`s + pending economy op) and a `RowAction` enum; render with `show_viewport` + cumulative offsets; add `apply_edited` to `dynamic.rs`; delete the v0.4 `edit.rs` / `rule_editor.rs` / `view_model.rs` JSON-rule modules.

## User Constraints (from CONTEXT.md)

<user_constraints>

### Locked Decisions

**Rule naming (author decision: FerrisFilter's own pattern only)**
- A rule's displayed name comes **only** from FerrisFilter's metadata comment run above the block (`#ff:name …`, Phase 16's convention).
- Foreign header text, such as NeverSink's `Show # %D4 $type->currency $tier->s !slug` and poe2filter.com slugs, is **never** shown as the name. The author wants no imported jargon in the editor.
- An unnamed rule shows a derived label: its index, Show/Hide/Minimal, and a short condition summary (e.g. `12  Show · Currency · BaseType ×14`), in the same spirit as v0.4's `rule <index>`.
- Naming a rule writes or updates its `#ff:name` line. The v0.4 name toggle and field carry over, retargeted to the `#ff:name` line.
- Foreign comments on the block are preserved verbatim on disk. They are just not used as the name.

**Large filters**
- A **flat list**, with search and row virtualization: only visible rows are drawn, and row summaries are cached after parsing.
- **No grouping by foreign banner comments**, because tool-specific comment styles aren't parsed (Phase 16).
- It must stay responsive at about 460 blocks, the size of the NeverSink sample (EDIT-07).
- **Search** matches the rule's `#ff:name`, the derived label, and Class / BaseType values.

**List operations**
- **Reorder:** up and down arrows, as in v0.4 (no drag-and-drop). Moving a rule across a block that has `Continue` shows a warning indicator but is not blocked.
- **Enable/disable:** the v0.4 circular toggle. Disabling comments the block out, and enabling uncomments it (Phase 16 semantics).
  - Commented-out blocks in a loaded filter appear as disabled rules.
  - A disabled rule whose commented lines are not valid grammar shows as disabled with an "invalid, can't enable" reason. It can still be moved or deleted, but enabling is refused.
- **Add / Delete / Duplicate:**
  - **+ ADD RULE** appends an empty, valid rule before the economy section.
  - Delete keeps v0.4's inline confirm.
  - Duplicate copies a block and gives it a fresh identity.
- **Show/Hide/Minimal and Continue** are set per rule (EDIT-03). If the rule has a style tag, the style's action wins on Apply (Phase 17 decision), and the UI should make that visible.
- **Enabled blocks with grammar errors** (Phase 16) are flagged on their row, and Apply is refused until they are fixed or disabled.

**Economy section and Apply**
- **Economy footer row:** "+ N locked rule blocks (economy)", with **Add economy section** when absent and **Remove** when present (ECON-02 + the ECON-04 UI). The blocks themselves are never listed or editable.
- **Apply and Revert** keep v0.4's model (EDIT-08): edits are held in memory, and Apply is the single writer; Revert re-parses the last applied text; the stale indicator and the "Options → Game → Item Filter → Refresh" reminder after Apply both stay.
- **Apply's safety outcomes** come from Phase 18 and are surfaced in the stripe: changed on disk; damaged markers; invalid enabled block.

### Claude's Discretion
Virtualization technique (egui `ScrollArea::show_rows` or equivalent), the summary text format, and how stable rule identity carries over from v0.4's session-only `RuleId`.

### Deferred Ideas (OUT OF SCOPE)
- Grouping by banner/section comments.
- Drag-and-drop reordering.

(Also out of scope per the phase boundary: per-condition/per-action editing = Phase 20; style application EDIT-06/STYLE-03 = Phase 20; config tab and choosing a different filter = Phase 21.)
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| EDIT-03 | Switch a block Show/Hide/Minimal; toggle Continue | `replace_block_lines` already permits a header-keyword change (only "exactly one block, same header line" is enforced); wrap as `set_block_keyword` / `set_continue` (see Gaps G1) |
| EDIT-04 | Disable comments out; commented-out blocks load as disabled rules, enableable | `Document::set_enabled`, `Block::{is_enabled, can_enable, disabled_state}` exist and are fixture-proven |
| EDIT-05 | Add, delete, duplicate, reorder | **Not in the library.** New `Document` ops required (G1) |
| EDIT-07 | Search; responsive on ~460 blocks | `show_viewport` + offset table + cached haystacks; fixture has 511 blocks for the perf test |
| EDIT-08 | Apply writes, Revert discards | Needs `apply_edited` (G2) + invalid-block gate (G3); Revert = re-parse the last applied text |
| ECON-02 | Economy not listed; "+ N locked rule blocks" | `section::find` gives marker line span; blocks with `header_line()` inside it are economy |
</phase_requirements>

## Project Constraints (from CLAUDE.md)

- Layer rule (`overlay/mod.rs`, PITFALLS §8): pure logic in library/pure modules, egui is a thin view. A bug findable only by clicking is in the wrong layer.
- Tier colours/Dracula belong to generated filters only; overlay chrome uses the `theme.rs` warm-grey palette (UI-SPEC correction).
- Deferred actions: row controls record a `RowAction` during render; mutate once after the loop. Quit stays deferred until after render.
- Plain-English labels instead of symbols; no emojis in docs/CLI output.
- Keep `CHANGELOG.md` `[Unreleased]` current as work lands; update CLAUDE.md overlay notes (the "Rule editing returns" text and the repository table rows for `ff`) when this phase lands.
- The real game folder must never be written in tests: use `tempfile` dirs, `FERRISFILTER_HOME` / `USERPROFILE` overrides.
- Build/test with `CARGO_TARGET_DIR=C:/Users/jonat/AppData/Local/Temp/ff-target` (ff.exe locks `target/debug`).
- Fixtures: `tests/fixtures/parser/*.filter` are `-text` (CRLF preserved); never regenerate `seed-universal.filter` for this phase.
- Memory: author prefers open design questions in prose; this run is autonomous, so decisions are recorded rather than asked.

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| Block add/delete/duplicate/move/keyword/Continue edits | Library (`filter::Document`) | — | Text-lossless edits with index rebuild; must be unit-testable without egui |
| Working state (Document + baseline + pending economy op + dirty flag) | Pure overlay module (`overlay::rules`) | — | No egui types; testable; Revert = re-parse baseline |
| Row summary / label / search haystack | Pure overlay module | Library (tokens) | Computed once per mutation, cached |
| Offset table / visible range / scroll-to | Pure overlay module | egui view | Pure arithmetic is unit-testable; egui only consumes `Range<usize>` |
| Row painting, search box, footer band | egui view (`app.rs`, `rule_list.rs`) | `widgets.rs` | Presentation only |
| Grammar-error gate, apply of edited text, changed-on-disk guard | Library (`dynamic.rs`, `safe_write.rs`) | Overlay maps error to status line | CLI parity; single writer |
| Economy add/remove/count | Library (`filter::section`) | Overlay | Text ops already exist; count from parse |

## Standard Stack

### Core (all already in `Cargo.toml`; no additions)
| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| egui / eframe | 0.36 (lock resolves 0.36.1/0.36.2 in registry) | Immediate-mode UI | Already the overlay stack [VERIFIED: Cargo.toml, registry source] |
| `ferrisfilter::filter` | in-repo | Lossless document, block index, `#ff:` meta, edits, `section` | Phase 16/18 deliverable [VERIFIED: source read] |
| `ferrisfilter::dynamic` | in-repo | `LoadedFilter`, `apply_file`, `economy_body` | Phase 18 single writer [VERIFIED: source read] |
| `tempfile` | 3 (already a normal dependency) | Hermetic tests | Existing pattern |

### Alternatives Considered
| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| `show_viewport` + offset table | `ScrollArea::show_rows` | `show_rows` assumes uniform row height; one expanded row breaks it. Use `show_rows` only if the expanded row is rendered outside the scroll list (not what UI-SPEC says) |
| Hand-rolled headless egui tests | `egui_kittest` | Not in the repo; adding a dev-dependency is avoidable. Existing pattern is `Context::run_ui` + synthetic `RawInput` (see below). Not adding a dep keeps the supply-chain surface zero |

**Installation:** none.

## Package Legitimacy Audit

No external packages are installed in this phase. Packages removed: none. Packages flagged: none. (slopcheck not run; not applicable.)

## Architecture Patterns

### System Architecture Diagram

```
 game-folder dynamic.filter ──LoadedFilter::load──▶ bytes (disk baseline) + text (maybe v0.4-migrated)
                                                      │
                                       filter::parse  ▼
                                    ┌─────────── WorkingFilter ───────────┐
   user input (egui frame)          │ doc: Document (edited in memory)    │
        │                           │ applied_text: String (Revert target)│
        ▼                           │ rows: Vec<RuleRow> (cache, no       │
  render pass records               │        economy blocks)              │
  Vec<RowAction> ───after loop────▶ │ economy: count/present/damaged      │
        │                           │ pending_economy_op: None|Add|Remove │
        │                           │ dirty: bool                         │
        │                           └──────┬───────────────┬──────────────┘
        │   rebuild rows + offsets on mutate│               │ Apply
        ▼                                   ▼               ▼
  search query ──filter──▶ visible index list ──▶ show_viewport    gate: no enabled block with
  (haystack substring)      + cumulative offsets   (draw only      Error/Warning diagnostics
                                                    intersecting)  ──▶ economy_body (fresh prices)
                                                                   ──▶ section refresh/add/remove on edited text
                                                                   ──▶ safe_write::write_guarded(baseline = loaded.disk)
                                                                   ──▶ success: applied_text = new text; dirty=false
                                                                       refusal: ChangedOnDisk ⇒ needs_reload
 Revert: doc = parse(applied_text); pending op cleared; dirty=false
```

### Recommended Project Structure
```
src/filter/
├── edit.rs        # EXTEND: insert_block, remove_block, duplicate_block, move_block,
│                  #   set_block_keyword, set_continue (or new src/filter/structure.rs)
src/dynamic.rs     # ADD: apply_edited(loaded, edited_text, EconomyOp) ; DynamicError::InvalidBlocks
src/overlay/
├── rules.rs       # NEW pure: WorkingFilter, RuleRow, RowAction, RuleFlags, label/summary/haystack
├── rule_list.rs   # NEW view: offset table (pure fn) + show_viewport rendering + row/collapsed/expanded
├── app.rs         # MODIFY: own WorkingFilter instead of bare LoadedFilter text; remove the
│                  #   "Rule editing returns" line and the ADD ECONOMY SECTION footer button
├── apply.rs       # MODIFY: new status outcomes (InvalidBlocks), footer_actions loses add_economy
├── widgets.rs     # keep (zebra_row may need a fixed-height variant; see Pitfall 3)
├── edit.rs / rule_editor.rs / view_model.rs   # DELETE (JSON-Rule based; see "What to keep/replace")
tests/
└── overlay_rules.rs (or lib tests)  # WorkingFilter on the FilterBlade fixture
```

### What to keep vs replace (overlay)
| Module | Verdict | Notes |
|---|---|---|
| `widgets.rs` (`header_plate`, `circular_toggle`, `arrow_button`, `zebra_row`) | KEEP | `arrow_button(enabled=false)` already uses `Sense::hover()`, so it cannot report clicks; add tooltip via `.on_hover_text` for "Clear the search to reorder". `zebra_row` uses `inner_margin(10,6)` and `set_width`; for virtualization you need a known row height, so paint the zebra fill with `ui.painter().rect_filled` over a rect of `row_h` rather than a growing `Frame` (see Pitfall 3) |
| `theme.rs` | KEEP | All colour roles in UI-SPEC already exist (`DISABLED_TEXT`, `WARM_ACCENT`, `ROW_SUMMARY_SIZE`, `row_summary`, `section_header`) |
| `apply.rs` | KEEP + extend | `ApplyState`, `indicator`, `success_message`, `unchanged_message`, `is_changed_on_disk`, `FilterState`, `footer_actions` stay; remove `add_economy` from `FooterActions` (UI-SPEC: one affordance); add outcome for invalid blocks. Note existing `filter_state_message` mentions "ADD ECONOMY SECTION" in text and a test asserts it: update both |
| `economy.rs` (SyncState) | KEEP unchanged | |
| `edit.rs` (`WorkingRules`, `RuleId`, setters for `When`) | DELETE | Every function targets `models::config::{Rule, When}`. Only two things are worth carrying: `parse_base_type_entry` (quoted-list splitting, Phase 15, needed again in Phase 20 for base-type entry; **move it, with its tests, to a small pure module rather than deleting**) and the `RuleId` idea (replaced by `BlockId`, see Identity) |
| `rule_editor.rs` | DELETE | 1326 lines of `Rule`/`Config` body controls. UI-SPEC explicitly says the v0.4 condition controls are NOT re-mounted. Salvage the *patterns* only: accordion expand, deferred `RowAction`, inline delete confirm, caption `.small()` LABEL_TAN style, name toggle+field |
| `view_model.rs` | DELETE | `RuleRow`/`row_header_text`/`summarize_list` are `Config`-based. Re-create a new `RuleRow` in `rules.rs` |

Deleting is safe: `models::config::Rule/When` stay for `generate`; nothing outside `overlay::` imports these three modules (Phase 18 summary: untouched but only referenced by themselves; verify with `grep -rn "overlay::\(edit\|rule_editor\|view_model\)" src tests` before deleting). This also resolves the leftover "overlay test names mentioning `dynamic_base.json`" because those tests (`edit.rs:757`, `view_model.rs:642`, comments in all three) go away with the modules.

### Pattern 1: Library whole-block operations (the Phase 19 critical path)

**What:** New `Document` methods that rebuild the line vector, re-run `parse::build`, and reassign `BlockId`s explicitly. The existing `commit()` remaps ids "by position" which only works when the block count and order are unchanged, so it cannot be reused for insert/remove/move; write a sibling `commit_structural` that takes an explicit old-id -> new-position mapping, or build the new `Vec<Item>` then walk blocks assigning ids from a `Vec<Option<BlockId>>` (`None` -> allocate from `self.next_id`, remove the `allow(dead_code)`).

Recommended signatures (names are guidance, not locked):
```rust
// all return Result<_, EditError>; on Err the Document is untouched (validate-then-commit, same as Phase 16)
fn insert_block(&mut self, before: Option<BlockId>, lines: Vec<String>) -> Result<BlockId, EditError>;
    // before=None => append; caller (overlay) passes the economy begin-marker's first block or
    // uses an "insert at line index" variant so ADD RULE lands before the economy section
fn remove_block(&mut self, id: BlockId) -> Result<(), EditError>;
fn duplicate_block(&mut self, id: BlockId) -> Result<BlockId, EditError>; // after the source
fn move_block(&mut self, id: BlockId, up: bool) -> Result<(), EditError>; // swap with neighbour *rule*
fn set_block_keyword(&mut self, id: BlockId, kw: BlockKeyword) -> Result<(), EditError>;
fn set_continue(&mut self, id: BlockId, on: bool) -> Result<(), EditError>;
```

**Design facts the implementer must respect (all verified in source):**
- A block's *unit* for delete/duplicate/move should be `span()` = leading comments + owned lines, plus (recommended) the run of blank-only trivia lines that directly follows, so deleting/moving a block does not leave doubled blank lines or orphan blank lines. NeverSink-style banner comments directly above a block are *attached* leading comments and will travel with that block on move/duplicate/delete; that is accepted (no banner grouping is a locked decision) but should be noted in the UI text/tests.
- `lead_start` never attaches an economy marker to a block (fixed in Phase 18 review), so moving the block just above the economy section cannot drag a marker.
- Blocks inside `section::find(&doc)` (between `begin` and `end` marker lines) are economy blocks. They must be excluded from move neighbour selection: the last rule's down-arrow is disabled, and `move_block` must never swap with an economy block. Blocks *after* the end marker (user-added by hand) are ordinary rules.
- `replace_block_lines` already accepts a changed header keyword (checks one block, same header line, same count) so `set_block_keyword` can be a thin wrapper: rewrite only the header token (preserve indentation and trailing `# comment` text using the tokenizer ranges; for a **disabled** block the header text starts with `#` and ranges are relative to the text after the first `#`). Add/remove a `Continue` body line only when it is a standalone line (`Kind::Modifier`). Refuse (new `EditError`) if the header is a compact form such as `Show Continue` (already classified as trailing tokens); the UI shows the control disabled for such a block.
- `duplicate_block` must copy foreign comments verbatim, give a fresh `BlockId`, and (per UI-SPEC) append ` (copy)` to an existing `#ff:name` through `set_meta` on the new block. For a duplicated *disabled* block, the lines stay commented.
- New empty rule text (ADD RULE): a header line `Show` with the document's EOL. This must pass `replace_block_lines`-style isolated validation (no Error/Warning). Do **not** invent default conditions.
- Inserting before the economy section: insert at the line index of the begin marker's *preceding blank run* so the economy keeps its leading blank line; when there is no economy section, append at EOF (ensure the previous last line is terminated, as `section::add` already does).
- Reuse the existing post-edit invariants: after every structural op, assert `to_bytes()` of untouched regions is identical (property-style test on the fixture: for each op, bytes outside the touched span are equal, op followed by inverse op returns original bytes: move up then down, duplicate then remove, remove then nothing).
- Per-edit cost is a full reparse: measured 1.7 ms release / 6.5 ms debug on the 258 KB fixture. Acceptable (<1 frame); do not add incremental indexing.

### Pattern 2: Pure working state with a row cache

```rust
pub struct WorkingFilter {
    doc: Document,                // edited copy
    applied_text: String,         // Revert target; = loaded.text at load, = written text after Apply
    rows: Vec<RuleRow>,           // rules only (economy excluded), in file order
    economy: EconomyView,         // Present{blocks:n} | Absent | Damaged(String)
    pending_economy: Option<EconomyOp>, // Add | Remove, applied to doc text at Apply... see note
    dirty: bool,                  // doc.serialize() != applied_text, tracked by mutation not by diffing each frame
}
pub struct RuleRow {
    id: BlockId, index: usize,    // real position among *rules*, 1-based for display
    enabled: bool, keyword: BlockKeyword, has_continue: bool,
    name: Option<String>,         // meta_get(id,"name") only
    style_tag: Option<String>,    // meta_get(id,"style")
    label: String,                // "12  Show · Currency · BaseType ×14" or "12  Show - My name"
    haystack: String,             // lowercase: name + label + class values + basetype values
    grammar_errors: Vec<FilterDiagnostic>, // Error|Warning in span, enabled blocks only
    cannot_enable: bool,          // disabled && !can_enable()
    conditions_text: Vec<String>, // for the minimal expanded body ("Class: Rings, Amulets")
}
```
- Rebuild `rows` (O(n) tokens, fast) after every mutation; never per frame. 511 rows of short strings is trivially cheap and avoids bookkeeping bugs from incremental update. Cache the `Vec<usize>` of search matches keyed on `(query, generation)`.
- **Economy pending ops (decision):** do the economy Add/Remove on the *working text* immediately at click time, not lazily at Apply: `Add` calls `dynamic::economy_body` (fresh prices; on error show the message and do nothing) then `section::add(doc.serialize(), body)` and re-parse into `doc` preserving rule `BlockId`s is awkward (re-parse reassigns ids). Simpler and safe: keep `pending_economy: Option<EconomyOp>` and have the footer row show the *projected* state (present/absent) plus the block count; at Apply compute `section` ops on the serialized doc text. For the "+ N locked rule blocks" count while an Add is pending, compute N from `economy_body`'s generated text by parsing it once at click time and caching N. Record this as a decision so the planner does not debate it.
- Search: `query.to_lowercase()`, substring over `haystack`. Index shown in labels stays the real position (UI-SPEC), so `RuleRow.index` is assigned from file order, not from the filtered list.
- Class/BaseType value extraction: `BodyLine.keyword.name` equal (ignore case) to `Class` or `BaseType`; slice `doc.line_text(...)` with `tokens.values` ranges (quoted values include quotes; strip them; for a disabled block ranges are relative to the text after the first `#`). Note: `Document::line_text` is `cfg(test)`-only today; use `doc.lines()[line].text()`.
- Derived label: `<index>  <Disposition> · <first Class>[ +N more] · BaseType ×N`; omit missing parts; no conditions -> `matches everything`. Use `Label::truncate()` and put the full text in `on_hover_text`.

### Pattern 3: Virtualized variable-height list with `show_viewport`

Verified API (egui 0.36.2 `containers/scroll_area.rs`): `ScrollArea::vertical().id_salt(..).auto_shrink([false,false]).show_viewport(ui, |ui, viewport: Rect| ..)`. `viewport` is the visible region in *content coordinates* (min.y = scroll offset). `show_rows` is itself implemented as `show_viewport` + `ui.set_height(total)` + a `scope_builder` at the first visible row, so the same recipe generalizes:

```rust
// pure, unit-testable (no egui): offsets[i] = top of visible-item i; offsets[n] = total height
fn build_offsets(n: usize, row_h: f32, gap: f32, expanded: Option<(usize, f32)>) -> Vec<f32>;
fn visible_range(offsets: &[f32], top: f32, bottom: f32) -> Range<usize>; // binary search (partition_point)

ScrollArea::vertical().auto_shrink([false, false]).show_viewport(ui, |ui, viewport| {
    ui.set_height(*offsets.last().unwrap());                       // full virtual height
    let r = visible_range(&offsets, viewport.min.y, viewport.max.y);
    let top = ui.max_rect().top() + offsets[r.start];
    let rect = Rect::from_x_y_ranges(ui.max_rect().x_range(), top..=ui.max_rect().top() + offsets[r.end]);
    ui.scope_builder(UiBuilder::new().max_rect(rect), |ui| {
        ui.skip_ahead_auto_ids(r.start);                           // keep auto ids stable like show_rows
        for vis_idx in r { /* draw row; give widgets ids from BlockId, not position */ }
    });
});
```
- The expanded row's height: store `measured_expanded_h: f32` from the previous frame (the `Response.rect.height()` of the expanded row's frame) in app state; request a repaint (`ctx.request_repaint()`) when it changes by more than 0.5px so layout converges in one extra frame. First frame after expanding uses a default estimate (e.g. 180px).
- Scroll-to (after Add, Duplicate, Reorder, jump-to-first-error): compute `offsets[vis_idx]` and set the **next frame's** scroll via `ScrollArea::vertical_scroll_offset(y)` (it applies once when set; track a `pending_scroll_to: Option<BlockId>` in state and consume it), or from inside the content closure call `ui.scroll_to_rect(rect, Some(Align::Center))` for the target row **only when that row is in the drawn range**; since the target may not be drawn yet, the offset-based approach is the reliable one. Use `Align::Center`-equivalent offset: `y - viewport_h/2`.
- Per-frame cost is O(visible rows) for drawing plus O(log n) for the range. `build_offsets` is O(n) but only recomputed when row count, search result set, or expanded id/height changes (cache it; n=511 floats is negligible even if rebuilt each frame, but cache anyway to honor "per-frame work proportional to visible rows").
- Ids: use `ui.push_id(row.id.0-based-hash, ..)` / `egui::Id::new(("rule", block_id_u64))`. `BlockId.0` is `pub(crate)` inside the lib crate; the overlay lives in the same crate, so it is accessible, but prefer a public `BlockId::get() -> u64` accessor or derive `Hash` (it already derives `Hash`, so `Id::new(("rule", id))` works with no accessor).
- Accordion expanded state: `Option<BlockId>` in app state, survives reorder and filtering because it is keyed by identity. If the expanded rule is filtered out or deleted, clear it.
- With a search active, offsets are built over the *filtered* list; reorder arrows are disabled (UI-SPEC).

### Pattern 4: Deferred RowAction (established pattern)
```rust
enum RowAction {
    Toggle(BlockId), Move(BlockId, Dir), Expand(Option<BlockId>), Delete(BlockId), ConfirmDelete(Option<BlockId>),
    Duplicate(BlockId), SetKeyword(BlockId, BlockKeyword), SetContinue(BlockId, bool),
    SetName(BlockId, String), ClearName(BlockId),
}
```
Render pushes into a `Vec<RowAction>`; after the loop `WorkingFilter::apply_action` runs each, rebuilds rows/offsets once, and returns an optional status string (errors from `EditError` rendered through `Display`, never a panic; no `unwrap`/`expect` in non-test code, per Phase 18 convention). Name text edits: hold the in-progress string in egui `TextEdit` against a per-frame copy and emit `SetName` when `.changed()`; each keystroke triggers one ~2-6 ms reparse, which is acceptable, but avoid `set_meta` with an empty value (it errors: toggle-off or empty text must call `remove_meta`).

### Identity (RuleId)
Use `filter::BlockId` directly. It is stable across `set_enabled`/`set_meta`/`replace_block_lines` (remapped by position), and the new structural ops must preserve it for moved blocks and allocate fresh ids (`next_id`) for inserted and duplicated ones. Drop `overlay::edit::RuleId`. After Revert the whole `Document` is re-parsed so ids reset; clear `expanded`, `confirming_delete`, `pending_scroll`, and "moved across Continue" flags at that moment.

### "Moved across Continue" flag
Compute in `apply_action(Move)`: crosses if the moved block `has_continue()` or the neighbour it jumped over does. Store `moved_flags: HashSet<BlockId>`; clear the set on Apply success and Revert, and remove an entry when that block is moved again without crossing.

### Apply path (library changes needed)
`apply_file(loaded, op)` computes the new text from `loaded.text` (the unedited text), so it cannot write edits. Add (names guidance):

```rust
pub enum EconomyOp<'a> { Keep, Refresh(&'a str), Add(&'a str), Remove }
pub fn apply_edited(loaded: &mut LoadedFilter, edited_text: &str, op: EconomyOp<'_>) -> Result<ApplyReport, DynamicError>;
```
Implementation: `check_grammar(edited_text)` (G3) -> `section::refresh/add/remove` on `edited_text` (map `NoSection` exactly as `apply_file` does) -> `safe_write::write_guarded(&loaded.path, loaded.disk(), new_text.as_bytes())` -> on success update `loaded.disk/text/migrated` like `apply_file` does. Refactor `apply_file` to call it (`apply_file(loaded, op)` = `apply_edited(loaded, &loaded.text.clone(), op)`) so the CLI and the overlay share one writer and the Phase 18 tests keep passing. `LoadedFilter` fields are private; implement inside `dynamic.rs`. For an **absent** file (`!loaded.existed()`), `apply_edited` with `Keep`/`Refresh`/`Remove` should keep the `FilterNotFound` refusal only when the edited text is empty AND no rules exist; otherwise allow writing (a user can build rules in an empty filter). Decision: with `Keep` and no economy section, writing is allowed (economy is optional now), but footer_actions currently requires `HasSection` for APPLY; relax to "root ok, filter loaded or absent, not damaged, not needs_reload".

### G3: invalid-enabled-block gate
Compute from the parsed edited text: `doc.diagnostics()` with `Severity::Error | Severity::Warning` whose line falls within an *enabled* block's `span()` (Warnings count: the edit API already treats unknown keywords / trailing header tokens as errors for tool-written blocks, D-11). Return count + first offending (block position among rules, line) for the UI-SPEC message `Apply refused: N enabled rules have grammar errors (first: rule 12, line 3)...`. Implement as a pure function in `overlay::rules` or `filter` (`Document::enabled_block_errors() -> Vec<(BlockId, FilterDiagnostic)>`); `RuleRow.grammar_errors` is derived from the same function. Also call it again inside `apply_edited` (belt and braces: library never writes a file with an enabled-block error that the *tool* introduced; but note a loaded foreign filter with pre-existing Warnings, e.g. an unknown keyword, would also block Apply until fixed or disabled. This matches the locked decision "Apply is refused until they are fixed or disabled", but the planner should decide the severity threshold; recommendation: block on `Error` and `Warning`, consistent with Phase 16 D-11; see Open Question 2).

### Anti-Patterns to Avoid
- **Rebuilding rows/labels in the paint loop** (EDIT-07): all string formatting for labels happens in `rebuild_rows`.
- **Position-keyed UI state**: reorder/search/delete invalidate indices. Key everything by `BlockId`.
- **Using the foreign header comment as a name** (locked decision). `migrate_v04` is the only place that turns a header comment into `#ff:name`, and only for the v0.4 file.
- **Writing in `LoadedFilter::load` or on open** (Phase 18 D-10).
- **Mutating inside the render loop**: use `RowAction`.
- **A second edit path that bypasses the Document** (e.g. string-splicing the text in the overlay). All edits go through `Document` ops so the lossless guarantees and tests apply.
- **Re-mounting v0.4 condition controls** (UI-SPEC explicitly forbids; Phase 20 owns it).

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Comment-out / un-comment a block | Manual `#` prefixing in the overlay | `Document::set_enabled` | Has D-08/CR-01 guards: refuses disabling a block that could not be re-enabled, refuses enabling invalid grammar |
| `#ff:name` read/write | Own `#ff:` parsing/formatting | `meta_get`, `set_meta`, `remove_meta` | Trims, validates keys, preserves EOL, inserts above header |
| Economy add/remove/refresh/damage detection | Searching for markers by substring | `filter::section::{find, add, remove, refresh}` | Marker-by-key, damage errors, "marker inside block" hard error |
| Guarded file write, backup, atomic replace, lock retry | `std::fs::write` | `safe_write::write_guarded` via the apply function | Changed-on-disk guard, one-time `.ffbak`, Windows retry |
| App root / file locations | `current_dir()` | `paths::app_root()`, `InputPaths::relative_to` | Double-click launch has an arbitrary cwd |
| Quoted-list splitting for base types | Regex/ad-hoc | `parse_base_type_entry` (move it out of the deleted `edit.rs`) | Phase 15 behaviour with tests |
| Virtual scrolling | Own scroll container | `ScrollArea::show_viewport` | Handles bars, input, momentum |
| Row zebra | Per-row `Frame` that grows | `painter().rect_filled` over a fixed-height rect | Needed for deterministic offsets |

**Key insight:** every lossless guarantee in this milestone lives in `Document`. The overlay must stay a client of it; any shortcut that touches text outside `Document` forfeits the byte-exact guarantee proven in `tests/filter_roundtrip.rs`.

## Runtime State Inventory

Not a rename/refactor of identifiers; however Phase 19 deletes modules and changes behaviour of persisted state:

| Category | Items Found | Action Required |
|----------|-------------|------------------|
| Stored data | `dynamic.json` (v0.4 JSON store) is no longer read by anything after Phase 18; overlay edits now live in `dynamic.filter` only. None in repo (git-ignored per-user file) | None; do not add a migration. CLAUDE.md still describes `dynamic.json`/`dynamic_base.json` in the layout table and the overlay note: update docs (code edit) |
| Live service config | None - verified: overlay has no external service config | None |
| OS-registered state | None - tray icon / hotkey registration unchanged | None |
| Secrets/env vars | `FERRISFILTER_HOME`, `USERPROFILE` used by `paths`/`default_destination`; unchanged | None |
| Build artifacts | None expected; deleting `tests`-adjacent modules leaves no stale artifacts | None |

## Common Pitfalls

### Pitfall 1: Id remap-by-position breaks on structural edits
**What goes wrong:** Reusing `Document::commit` for insert/delete/move maps ids by block *position*, so after a move every id would silently point at the wrong block, and selection/expansion/flags jump.
**Why:** `commit()` was designed for count-preserving, order-preserving edits.
**Avoid:** Separate structural commit with explicit id assignment; unit test "move A below B: ids follow the blocks, `meta_get(id,'name')` still returns each block's own name".
**Warning signs:** After Move the expanded row changes to a different rule.

### Pitfall 2: Moving a block across trivia/leading comments corrupts neighbours
**What goes wrong:** Leading comment runs belong to the block below them; blank lines are separate trivia. Swapping only `owned()` ranges strands comments or yields doubled blank lines.
**Avoid:** Treat `span()` (+ following blank run) as the unit; test on the fixture that `move_block(up)` then `move_block(down)` returns the original bytes, and that all bytes outside the two involved units are unchanged.

### Pitfall 3: `show_viewport` row height drift
**What goes wrong:** `zebra_row`'s `Frame` + `inner_margin(10,6)` + `item_spacing` makes real row height differ from the `row_h` used for offsets; rows overlap or gap and the scrollbar length is wrong.
**Avoid:** Allocate each collapsed row with `ui.allocate_ui_with_layout(vec2(w, row_h), ..)` (or `allocate_exact_size`), paint zebra with the painter, and set `ui.spacing_mut().item_spacing.y = 0` inside the list (use an explicit `gap` in `build_offsets`). Verify with a headless test that drawn row rects tile exactly against `offsets`.

### Pitfall 4: Expanded-row height is one frame stale
**What goes wrong:** First frame after expanding, the offset table uses the estimate; rows below jump.
**Avoid:** Store measured height; `request_repaint()` on change; accept one-frame settle (stripe already repaints at 50 ms).

### Pitfall 5: Auto-ids shift under virtualization
**What goes wrong:** egui auto ids come from creation order; skipping rows changes ids of widgets, losing focus/text-edit state.
**Avoid:** `ui.skip_ahead_auto_ids(range.start)` (as `show_rows` does) AND explicit `push_id(BlockId)` for every row. The name `TextEdit` must keep focus while the user types even as other rows scroll in/out.

### Pitfall 6: Toggling a name on/off vs. empty value
`set_meta` rejects an empty value (`InvalidMeta`). Name toggle off or cleared text => `remove_meta`; toggle on with empty text should only flip UI state (show the field) without writing; write on first non-empty text.

### Pitfall 7: Disable can fail
`set_enabled(false)` returns `CannotDisable` for compact headers (`Show Continue`) and some comment-adjacent layouts; `set_enabled(true)` returns `CannotEnable`. These are expected outcomes on real filters. Show the message in the status area and leave the toggle unchanged; do not `unwrap`.

### Pitfall 8: Keyword change on a disabled block / compact header
Header text for a disabled block begins with `#`; tokenizer ranges are relative to the text after the first `#`. Test `set_block_keyword` on a disabled block from the fixture (51 exist) and on a header with a trailing comment (`Show # %D4 ...`) so the comment survives.

### Pitfall 9: Search must not hide the pending-scroll target
Add rule while searching: UI-SPEC says scroll only if it matches; otherwise status line `Rule added at position N (hidden by the current search)`. Evaluate the new rule against the query before deciding.

### Pitfall 10: Apply writes the *edited* text; baseline for the guard stays the disk bytes
The guard compares against `loaded.disk` (exact bytes read), not against the in-memory text. After a successful Apply both become the written bytes. After a failed Apply nothing in `loaded` changes and `dirty` stays true. After "changed on disk": `needs_reload` => RELOAD discards edits (UI-SPEC copy says so).

### Pitfall 11: v0.4 migration interplay
`LoadedFilter::load` migrates a v0.4 file in memory (adds markers and `#ff:name` lines) and keeps the original bytes as the guard baseline. `WorkingFilter` must start from `loaded.text()` (migrated), and the first Apply writes the migrated + edited text with a `.ffbak`. Dirty should be true from the start only if the user edits; the existing "will be converted on next APPLY" status line stays.

### Pitfall 12: Window sizing / bottom panel
Keep the footer as `egui::Panel::bottom` claimed *before* the central content (existing regression test `the_apply_button_stays_on_screen_...`). The economy footer row is pinned "directly under the list": put it in the same bottom panel above the action row, or as its own `Panel::bottom` claimed before the list, so it never scrolls. Search box and `FILTER` header are above the list in the central panel.

## Code Examples

### Selecting economy vs. rule blocks
```rust
// Source: src/filter/section.rs (find) + block.rs (owned) - in-repo
let span = section::find(&doc);               // Result<Option<SectionSpan{begin,end}>, SectionError>
let is_economy = |b: &Block| matches!(&span, Ok(Some(s)) if b.header_line() > s.begin && b.header_line() < s.end);
let rules: Vec<&Block> = doc.blocks().filter(|b| !is_economy(b)).collect();
let economy_blocks = doc.blocks().filter(|b| is_economy(b)).count();
// On Err(SectionError) => economy shown as Damaged; list the blocks you can (all non-marker blocks),
// but disable Apply and the Add/Remove affordances (existing FilterState::Damaged behaviour).
```

### Name, style, errors for a row
```rust
let name = doc.meta_get(id, "name");            // only #ff:name; never the header comment
let style = doc.meta_get(id, "style");          // informational this phase
let errs: Vec<_> = doc.diagnostics().iter()
    .filter(|d| matches!(d.severity, Severity::Error | Severity::Warning)
        && d.line > b.span().start && d.line <= b.span().end)   // diagnostics are 1-based
    .collect();                                  // only for enabled blocks; disabled: use b.disabled_state()
```
(Diagnostics are 1-based; spans are 0-based half-open: `d.line > start && d.line <= end` is the same test `edit.rs` uses.)

### Headless egui test pattern already used in the repo
```rust
// Source: src/overlay/app.rs tests
let ctx = egui::Context::default();
ctx.set_fonts(egui::FontDefinitions::empty());         // headless: text measures zero height!
let input = egui::RawInput { screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(460.0, 600.0))), ..Default::default() };
ctx.run_ui(input, |ui| { /* build UI */ }).drop_without_applying_deltas();
```
Caveat recorded in that file: with empty fonts, labels have zero height, so tests that need real geometry use `allocate_space` / fixed-size rects. Consequence for Phase 19: **test the virtualization maths in pure functions** (`build_offsets`, `visible_range`, scroll-to offsets) and test the view only for coarse properties (number of rows drawn <= visible+slack; footer on screen), using fixed-height allocated rows. `widgets.rs` tests show the click-injection pattern (`RawInput.events` with `PointerMoved`/`PointerButton`, one frame for press/release); reuse it for the toggle/arrow tests. egui_kittest is not used anywhere.

### Perf test on the real fixture
```rust
// tests/overlay_rules.rs  (library test; no egui)
let bytes = std::fs::read("tests/fixtures/parser/filterblade.filter").unwrap();   // 511 blocks, 460 enabled
let mut w = WorkingFilter::from_text(String::from_utf8(bytes).unwrap());
assert_eq!(w.rows().len(), 511);               // no economy section in this fixture
// build rows, search, 100 random actions: assert elapsed per action < 50ms (debug) - generous budget;
// measured raw edit cost is ~6.5ms debug, so this guards against accidental O(n^2) in rebuild_rows
```
Note the fixture is the FilterBlade/NeverSink export (the "NeverSink sample"); there is no separate NeverSink file. 511 total blocks include 51 commented-out valid/invalid disabled blocks, which is exactly the EDIT-04 population.

## State of the Art

| Old Approach (v0.4) | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| `Vec<Rule>` in `dynamic.json`, `WorkingRules`, `RuleId` | `Document` over `dynamic.filter`, `BlockId` | Phases 16/18 | Overlay editor must be rebuilt on blocks |
| `rule <index>` derived label / config `name` | `#ff:name` only; derived label from conditions | Phase 19 decision | Foreign header text never shown |
| `ADD ECONOMY SECTION` footer button | Economy footer *row* with Add/Remove | Phase 19 UI-SPEC | `footer_actions.add_economy` removed |
| Rule `enabled:false` in JSON | Commented-out block | Phase 16 | Disabled blocks are first-class rows |

**Deprecated within this phase:** `overlay::edit`, `overlay::rule_editor`, `overlay::view_model` (delete); the "Rule editing returns in the next update" status line; the `ADD ECONOMY SECTION` action-row button.

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | Blocks located after the `economy-end` marker should be treated as ordinary rules | Pattern 1 / Code Examples | Low: a hand-edited file only; alternative is to hide them or flag them |
| A2 | Apply should be refused for enabled blocks with `Warning` diagnostics (unknown keyword, trailing header tokens), not only `Error` | G3 | Medium: real third-party filters may carry Warnings the game accepts; refusing would block Apply until the user disables those blocks. See Open Question 2 |
| A3 | Moving/deleting/duplicating treats the following blank-line run as part of the block's unit | Pattern 1 | Low (cosmetic) |
| A4 | Pending economy Add/Remove is applied to the serialized text at Apply time and the footer shows a projected state | Pattern 2 | Low; alternative (apply at click) loses ids on re-parse |
| A5 | With APPLY relaxed to "no economy section required", a rules-only filter may be written | Apply path | Medium: UI-SPEC/Phase 18 gate APPLY on `HasSection`; confirm this relaxation is wanted (planner may keep the Phase 18 gating and only enable APPLY when a section exists) |
| A6 | `egui_kittest` should not be added | Alternatives | Low |

## Open Questions

1. **Style-wins flag has no behaviour behind it yet.**
   - Known: STYLE-03 / EDIT-06 (style tags and applying a style's actions on Apply) are Phase 20; no Apply code reads `#ff:style`. CONTEXT says "style's action wins on Apply (Phase 17 decision)".
   - Unclear: whether Phase 19 should actually rewrite the header keyword on Apply for tagged blocks.
   - Recommendation: Phase 19 renders the flag and the read-only selector for any block that has a `#ff:style` line (it can only exist from a hand edit or v0.4 data today) and does **not** implement the Apply-time rewrite; record it as a Phase 20 dependency in the plan and CHANGELOG. This avoids implementing half of STYLE-03.
2. **Which diagnostics block Apply?** Recommend `Error` + `Warning` inside enabled blocks (consistent with Phase 16 D-11), but show Warnings with the same `grammar error` flag and tooltip text so the user knows why. If the corpus run on FilterBlade shows zero Warnings/Errors (Phase 16 verification says zero Errors on all fixtures; Warnings were not asserted), no fixture is blocked. Planner: add a test asserting the fixture yields zero blocking diagnostics.
3. **ADD RULE when the filter does not exist yet** (Absent state): allowed in memory; first Apply creates the file (the Phase 18 add-economy-creates-file path). Confirm APPLY without a section is allowed (A5).
4. **Name for duplicated unnamed rule:** no name is added (only append ` (copy)` when a name exists), per UI-SPEC.

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| Rust toolchain / cargo | build, tests | yes | edition 2024 project builds (a release+debug test run completed this session) | — |
| egui 0.36 sources (registry) | API verification | yes | 0.36.1 / 0.36.2 | — |
| Fixture `tests/fixtures/parser/filterblade.filter` | perf + op tests | yes | 258,094 bytes, CRLF, 511 blocks | — |
| Display / game for manual checks | stripe visual UAT | no (autonomous run) | — | Record as `*-HUMAN-UAT.md` items, as Phases 16/18 did |
| `cargo fmt` | tidy | yes | — | — |

## Validation Architecture

`workflow.nyquist_validation` is `false` in `.planning/config.json`; the formal section is skipped. Practical test plan for the planner:

- Framework: built-in `cargo test` (unit tests in `#[cfg(test)]` modules, integration in `tests/`). Quick: `CARGO_TARGET_DIR=C:/Users/jonat/AppData/Local/Temp/ff-target cargo test --lib filter:: overlay::`; full: `cargo test`.
- Wave 0 / first plan: library ops with fixture round-trip tests (move up/down inverse, duplicate/remove inverse, insert-before-economy on a file with markers and on the v0.4 fixture `tests/fixtures/v04-dynamic.filter`, keyword/Continue edits on enabled, disabled, header-with-comment blocks, ids follow blocks).
- Pure tests: `build_offsets`/`visible_range`; row cache (labels per UI-SPEC examples); search matches name/label/class/basetype, case-insensitive, and not the foreign header comment; gate message counts.
- Perf guard: 100 mixed actions on the 511-block fixture within a generous budget; assert rows drawn in a headless frame <= visible count + slack for 511 rows.
- `apply_edited` tests with `tempfile` dirs (never the real game folder): writes edited text; ChangedOnDisk after external modification; refuses on an enabled-block error; Remove/Add economy; `.ffbak` created once; second Apply is `Unchanged`.
- Manual (HUMAN-UAT): stripe renders at 460 px, scrolling feel on the NeverSink sample, expanded row layout, flags, Revert/Apply messages, in a scratch folder via `FERRISFILTER_HOME`/`USERPROFILE`.

## Security Domain

`security_enforcement` not set to false; local desktop tool, no network on this path (SYNC ECONOMY unchanged).

| ASVS Category | Applies | Standard Control |
|---------------|---------|-----------------|
| V2/V3 Auth/Session | no | — |
| V4 Access Control | no (single-user local) | — |
| V5 Input Validation | yes | User-typed rule names go through `set_meta` (rejects line breaks and bad keys; trims). Pasted/typed text can never inject extra filter lines because `EmbeddedNewline`/`InvalidMeta` refuse CR/LF. Search text is never used in file output |
| V6 Cryptography | no | — |
| V12 File handling | yes | All writes via `safe_write::write_guarded` (changed-on-disk guard, one-time backup, atomic persist). Never write the real game folder in tests |

| Threat | STRIDE | Mitigation |
|--------|--------|------------|
| Filter-line injection through a rule name or imported text | Tampering | `set_meta` validation (existing); new ops reject embedded newlines; keyword changes only emit `Show`/`Hide`/`Minimal` literals |
| Overwriting a file edited by another program | Tampering | Guard baseline = exact disk bytes (existing) |
| Data loss through unconfirmed delete | — | Inline delete confirm; Revert as safety net until Apply; `.ffbak` on first write |
| Economy markers corrupted by block moves | Tampering | Markers never attach as leading comments (fixed in Phase 18 review); moves exclude economy blocks; re-run `section::find` after every structural op in tests |

## Leftovers to tidy

- **`cargo fmt --check` is not just `src/cli.rs`.** It reports diffs in ~50 files (all of `src/`, and `tests/*.rs`; no `rustfmt.toml`). Recommendation: either (a) do one mechanical, standalone `chore: cargo fmt` commit as the **first** task of the phase, verify `cargo test` is unchanged, and keep touched files formatted afterwards; or (b) leave the repo unformatted and only format new files. Choose (a) only if the orchestrator is comfortable with a ~300-hunk diff; the cost of (a) is merge noise now rather than later. Default recommendation: (a), done first, so later diffs in this phase stay reviewable. The cli.rs diff is a single `#[command(...)]` wrap.
- **Overlay tests/comments naming `dynamic_base.json`:** located in `edit.rs` (test `dynamic_base_json_offers_exactly_twelve_styles`, comments at 273/300/385/391/578/1001/1081), `rule_editor.rs` (738, 1040), `view_model.rs` (33, 269, 642-646). All disappear when the three modules are deleted. If the planner prefers to keep any, rename them to `v04_dynamic_base` (the template lives at `tests/fixtures/v04-dynamic-base.json`).
- Pre-existing `unused import: super::*` test-build warning in `app.rs` `mod tests` (Phase 18 note; the commit log shows IN-04 fixed it, so re-check with `cargo test --no-run`).
- CLAUDE.md: update the `ff` row and the overlay note, remove "Rule editing is temporarily absent"; CHANGELOG `[Unreleased]` entry.
- `app.rs` module doc still says rule editing is absent; update.
- `Document::next_id` `#[allow(dead_code)]` can go once insertion exists.
- `Document::line_text/line_at/line_count` are `cfg(test)`-only; the overlay should use `doc.lines()`.

## Suggested plan decomposition (guidance, planner decides)

1. Wave 0: (optional) `cargo fmt` commit; delete dead v0.4 modules after salvaging `parse_base_type_entry`.
2. Library: structural `Document` ops + fixture tests (G1).
3. Library: `dynamic::apply_edited`, grammar gate, `DynamicError` variant + CLI unaffected (G2, G3).
4. Pure overlay: `WorkingFilter`, `RuleRow`, labels/search/haystack, `RowAction` application, economy projection, Revert, flags; tests incl. perf on fixture.
5. View: offsets/visible-range pure fns, `rule_list.rs` rows (collapsed/expanded), search box, economy footer row, footer actions; wire into `app.rs`; remove obsolete notice/button.
6. Docs/UAT: CLAUDE.md, CHANGELOG, `19-HUMAN-UAT.md` items (windowed-mode check, scroll feel, expand/collapse, messages).

Steps 2 and 3 are independent of each other and of the pure overlay module's row cache; 4 depends on 2; 5 depends on 4.

## Sources

### Primary (HIGH confidence) - read in this session
- `src/filter/{mod,block,edit,meta,section,parse,tokenize,keywords,diag,lines}.rs` - APIs, edit invariants, economy marker handling
- `src/dynamic.rs`, `src/safe_write.rs`, `src/paths.rs` - Phase 18 store/apply functions
- `src/overlay/{app,apply,widgets,theme,mod}.rs`; signature scans of `edit.rs`, `rule_editor.rs`, `view_model.rs`
- Phase 16 and 18 SUMMARY files; Phase 17 SUMMARYs (style tags are Phase 20)
- egui 0.36.2 source (`containers/scroll_area.rs`: `show_rows`, `show_viewport`, `vertical_scroll_offset`; `ui.rs`: `scroll_to_rect`)
- Measurement run (temporary test, removed): FilterBlade fixture parse/edit/serialize timings and block counts
- `cargo fmt --check` output

### Secondary / Tertiary
- None. No web sources were needed.

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH - no new dependencies; all APIs read from source.
- Architecture: HIGH for the library gaps (verified absent) and the virtualization recipe (mirrors egui's own `show_rows`); MEDIUM for exact helper signatures (guidance).
- Pitfalls: HIGH for 1, 2, 5, 6, 7, 10, 11 (derived from code); MEDIUM for 3, 4 (egui layout behaviour reasoned, to be confirmed by the pure/headless tests).

**Research date:** 2026-10-12
**Valid until:** 2026-11-12 (internal codebase; stable until Phase 19 changes it)
