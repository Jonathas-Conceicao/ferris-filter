# Phase 15: BaseType Lists and Rule Names - Research

**Researched:** 2026-09-08
**Domain:** egui 0.36.1 text entry (commit-on-Enter, focus retention, id-keyed ephemeral state)
layered onto the existing `overlay::edit`/`rule_editor.rs` pure-mutation architecture from Phase 14.
**Confidence:** HIGH on every API claim — all confirmed against the installed
`egui-0.36.1` source, not memory (Phase 13's lesson). MEDIUM on the two layout/visual
questions that only a running stripe can fully settle (long base names wrapping,
the exact toggle-flicker fix's feel), both routed to a short manual-check list.

This research is deliberately narrow per the orchestrator's brief: it does **not** re-derive the
model/emitter groundwork already verified in `15-CONTEXT.md`'s `<code_context>` (`When::base_type`,
`Rule::name`, their emitter paths). It focuses on the five `<research_questions>` the orchestrator
asked for, in priority order, plus one load-bearing finding (Q3) that changes how the planner should
scope Task work.

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

**Base-type entry (R1)**
- **D-01:** Entry is item by item, not one big text box — a list of already-added base types, each
  with its own remove control, plus a single-line field that appends one entry at a time. A
  multi-line textarea is explicitly rejected.
- **D-02:** Removal is per item, at any position — not "clear all", not "remove last".
- **D-03:** Committing an entry is `Enter` or an add button; commit clears the field so the next
  name can be typed straight away without reaching for the mouse.
- **D-04:** Claude's discretion on the rest of the entry mechanics (locked as D-14/D-15/D-16 below).

**Typos and validation (R1)**
- **D-05: No base-name validation. Accept what is typed, verbatim.** No spell-check, no warning
  icon, no known-bases fixture. Trimming whitespace, dropping empty entries, and ignoring exact
  duplicates are hygiene, not validation, and are allowed.

**`class` as an optional condition (R1)**
- **D-06:** `class` gets the same independent off switch every other condition has (EDIT-08/D-14
  from Phase 14) — it joins the uniform enable-toggle-plus-value vocabulary.
- **D-07:** The class picker is hidden entirely while the toggle is off, and appears when ticked —
  not greyed out, not collapsed-but-present.
- **D-08:** Unticking `class` discards the class list (matches every other off switch's
  `clear_*` = `None` behavior). `edit::set_class` already collapses a fully-deselected list to
  `None`, so this is the same end state reached by a different route.
- **D-09:** A new rule still has nothing checked — D-07 from Phase 14 stands; only the body's
  *opening state* changes (no class picker expanded by default).

**Rule names (R2)**
- **D-10:** Row header format is `<index> <style>[ - name]` — bracketed part present only when
  named. Keeps the style, does not replace it. `[DISABLED]` marker unaffected.
- **D-11:** No auto-fill. A name is empty until typed. The emitter's existing `rule {idx}` fallback
  already handles the unnamed case at emit time.
- **D-12:** The name has an off switch like everything else; unticking clears the stored name.
- **D-13:** A name is a label, never an identity. `RuleId` stays the identity, the array index
  stays precedence. Nothing may key off a name.

### Claude's Discretion (locked, vetoable on sight)
- **D-14:** The add field strips surrounding double quotes and splits a pasted quoted list
  (`"Heavy Belt" "Utility Belt"` → two entries; `Heavy Belt` → one entry).
- **D-15:** Base types render in the collapsed summary with an explicit label (`Bases: Heavy Belt,
  Utility Belt +2 more`) — `summarize_list` already renders the raw names, just unlabelled.
- **D-16:** The base-type control sits directly after the class control; the name field sits at
  the very top, above `style`.

### Deferred Ideas (OUT OF SCOPE)
- Base-name validation / autocomplete against a known base list — no base-name source exists that
  wouldn't go stale each league.
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|-------------------|
| EDIT-10 | Base-type list entered item by item, individually removable, `class` optional so a base-type-only rule emits no `Class` line | Pattern 1 (ephemeral toggle) resolves the `class` off-switch design; Pattern 2 (deferred per-item removal) resolves D-01/D-02; Pattern 4 (`parse_base_type_entry`) resolves D-14; all base-type setters must replicate `set_class`'s empty-vec→`None` collapse (Risk 1) |
| EDIT-11 | Optional rule name, shown in row header, emitted as block comment, unnamed rule keeps derived label | Pattern 1 also covers the `name` off-switch (same flicker hazard as `class`); Pattern 3 (commit-on-Enter) is the text-entry mechanism; Risk 2 covers why no sanitization layer is needed for filter-syntax safety |
</phase_requirements>

## Summary

The two features are individually simple — a text field and a labeled list are not novel egui
work — but **both hit the same non-obvious design trap**, and it is the single most important
finding of this research: **`class` and `name` cannot reuse the existing enable-toggle pattern
verbatim**, even though D-06/D-12 ask for "the same independent off switch every other condition
has." Every existing toggle (`rarity`, `defence`, `sockets`, `quality`, `corrupted`,
`unidentified_tier`) derives its checked-state fresh each frame from `when.<field>.is_some()`, and
seeds a *non-empty, sensible default value* the instant it is switched on (`Rarity >= Magic`,
`min: 1`, `Some(false)`, tier `4`). `class` and `name` have no such default — D-09 explicitly
forbids auto-selecting a class, and D-11 explicitly forbids auto-filling a name. If their toggles
are implemented the same way (checked-state = `Option::is_some()`, recomputed every frame, nothing
written on enable), the panel will **visibly snap shut one frame after being switched on**: the
click flips the local `bool` for that one frame (so the picker briefly appears), but on the very
next repaint the toggle re-derives `false` from the still-empty `Option` field and the picker
vanishes before the player can click a single checkbox or type a single character. This is not a
polish concern — it would make both controls non-functional. The fix (Pattern 1) is to give
`class` and `name` their own small piece of *ephemeral, egui-id-keyed* toggle state
(`ui.data_mut(|d| d.get_temp_mut_or(id, seed))`) that is written once by the click and then left
alone by the render loop, decoupled from whether the underlying `Option` field has been populated
yet.

The second finding is that egui 0.36.1's commit-on-Enter idiom is not a pattern to invent — it is
**the exact code sample in `TextEdit`'s own doc comment**
(`egui-0.36.1/src/widgets/text_edit/builder.rs:39`):
`response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter))`. Better still,
`TextEdit::singleline` structurally cannot contain a newline — not just on Enter (which the source
confirms calls `surrender_focus` and breaks out of the event loop rather than inserting `\n`), but
also on paste (`text_to_insert.replace(['\r', '\n'], " ")`, confirmed at builder.rs:1116-1119). This
closes Q5 (the filter-syntax-safety question) for free: a rule name typed or pasted into a
`TextEdit::singleline` can never contain the one character (`\n`) that would actually corrupt the
emitted `Show # {label}` comment line, so **no sanitization layer is needed beyond using
`singleline`, not `multiline`, for both the name field and the base-type add field** — this is a
structural guarantee from the widget choice, not something the plan needs to write code for.

Everything else is straightforward, unremarkable egui composition matching what `rule_editor.rs`
already does for its seven existing conditions, plus one deferred-mutation detail (Pattern 2)
directly analogous to `app.rs`'s existing `RowAction` collect-then-apply discipline, scaled down to
a single render function instead of the whole row loop.

**Primary recommendation:** implement `class` and `name` as two more entries in the uniform
enable-toggle vocabulary, but back their toggle state with `ui.data_mut`'s id-keyed ephemeral
storage (Pattern 1) instead of deriving it from `Option::is_some()` every frame; render the
base-type list as an owned per-frame clone with a single deferred `remove_at: Option<usize>`
(Pattern 2, mirroring `app.rs`'s `RowAction`); use `TextEdit::singleline` plus
`response.lost_focus() && Key::Enter` for commit, immediately followed by `response.request_focus()`
on the same response object to keep the keyboard in the field (Pattern 3); and give `edit.rs` a
`parse_base_type_entry(raw: &str) -> Vec<String>` pure function for D-14's quote-stripping/
paste-splitting, following the exact "pure logic in `edit.rs`, presentation calls it" split every
other condition in this file already uses.

## Architectural Responsibility Map

Unchanged from Phase 13/14's own mapping — no new tier is introduced. Restated for the two new
capabilities:

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| Base-type add/remove/parse logic | Application/Library (`overlay::edit`, extended) | — | Pure `Vec<String>` transforms plus the D-14 parse function, unit-testable exactly like the existing per-condition setters |
| `name` set/clear | Application/Library (`overlay::edit`, extended) | — | A one-line `Option<String>` setter, same shape as `clear_rarity`/`clear_sockets` |
| `class`/`name` toggle-open ephemeral state | Presentation (egui `Ui::data_mut`, id-keyed) | — | Deliberately **not** in `overlay::edit` or `WorkingRules` — it is UI-session state with no correctness stake in the emitted filter (Pattern 1); keeping it out of the pure layer avoids inventing a "third identity scheme" alongside `RuleId` and array position |
| Row header name display | Presentation (`app.rs::row_header_text`) | Library (`view_model::RuleRow.name`, already exists) | D-10's one-line format change; the data is already threaded through, only the formatting changes |
| Per-item base-type row rendering + deferred removal | Presentation (`rule_editor.rs`) | — | Mirrors `app.rs`'s existing `RowAction` collect-then-apply shape, scoped to one render function instead of the whole accordion |

## Standard Stack

No new dependencies. Every API used below (`TextEdit`, `Ui::data_mut`/`IdTypeMap`,
`Response::request_focus`) ships in the already-pinned `egui` 0.36 — confirmed present by reading
the installed source, not assumed from a changelog.

**Version verification:** `Cargo.toml` already pins `egui`/`eframe` `"0.36"`; the installed crate
at `~/.cargo/registry/src/index.crates.io-*/egui-0.36.1/` matches. No `cargo add` needed this
phase.

## Package Legitimacy Audit

Not applicable — this phase adds zero new external packages. `slopcheck` was not invoked because
there is nothing for it to check.

## Architecture Patterns

### System Architecture Diagram

```
render_rule_body(ui, rule, config, id)      <- unchanged entry point (rule_editor.rs)
  │
  ├─ render_name(ui, rule, id)              <- NEW, sits first (D-16)
  │    reads:  ui.data_mut(|d| d.get_temp_mut_or(name_open_id, rule.name.is_some()))
  │    click:  ON  -> flips ephemeral bool true only, rule.name left untouched
  │            OFF -> edit::clear_name(rule), ephemeral bool false
  │    body:   TextEdit::singleline(&mut buf) ──Enter/lost_focus──▶ edit::set_name(rule, buf)
  │                                                                  buf.clear(); response.request_focus()
  │
  ├─ render_style(...)                      <- unchanged
  ├─ render_classes(ui, rule, config, id)   <- EXTENDED: gains the same toggle shape as render_name
  │    reads:  ui.data_mut(|d| d.get_temp_mut_or(class_open_id, when.class.is_some()))
  │    OFF -> hides the 4 CollapsingHeader groups entirely (D-07), edit::clear_class(when)
  │    ON  -> groups appear, nothing pre-selected (D-09), unchanged checkbox loop underneath
  │
  ├─ render_base_types(ui, rule, id)        <- NEW, sits directly after classes (D-16)
  │    per frame: let items = when.base_type.clone().unwrap_or_default();  // owned, no live borrow
  │    for (i, name) in items.iter().enumerate():
  │        row: wrapped label + remove button  ──clicked──▶  remove_at = Some(i)   // NOT applied yet
  │    after the loop: if let Some(i) = remove_at { edit::remove_base_type(when, i) }  // Pattern 2
  │    add field: TextEdit::singleline(&mut add_buf) ──Enter/button──▶
  │        edit::parse_base_type_entry(&add_buf) -> Vec<String>   // pure, D-14
  │        for name in parsed { edit::add_base_type(when, name) }  // dedup + empty-drop, D-05 hygiene
  │        add_buf.clear(); response.request_focus()
  │
  ├─ render_rarity / render_defence / render_sockets / render_quality
  │    / render_corrupted / render_unidentified_tier            <- unchanged
  │
  └─ row_header_text(row) in app.rs         <- EXTENDED: `{ordinal:02}  {style}[ - {name}]{marker}`
```

### Pattern 1: Ephemeral, id-keyed toggle state for conditions with no natural non-empty default
(answers Q3, and is the reason Q1/Q2 need a slightly different shape than Phase 14's uniform
pattern)

**What:** For a condition whose "enable" toggle has a natural seed value (rarity → `>=Magic`,
sockets/quality → `1`, corrupted → `false`, unidentified tier → `4`), the toggle's checked-state
can be safely *derived* every frame from `when.<field>.is_some()`, because switching it on always
writes a non-empty value in the same click handler — there is no frame where "toggled on" and
"field still empty" are simultaneously true. `class` and `name` are structurally different: D-09
and D-11 both forbid writing anything on enable (no pre-selected class, no auto-filled name), so a
derived toggle would read back `false` the very next frame and the panel would appear to flicker
shut before the player could act.

**The fix:** back these two toggles with egui's own id-keyed temp storage
(`egui-0.36.1/src/util/id_type_map.rs:488`, `pub fn get_temp_mut_or<T: 'static + Any + Clone + Send
+ Sync>(&mut self, id: Id, or_insert: T) -> &mut T`, confirmed present and unchanged from Phase
14's own use of `CollapsingHeader`'s internal open/closed memory, which uses the same mechanism
under the hood via `insert_persisted`/`get_persisted` at `collapsing_header.rs:32-43`). `Ui`
exposes this directly — confirmed `impl Deref for Ui { type Target = Context; ... }` at
`egui-0.36.1/src/ui.rs:91`, so `ui.data_mut(...)` resolves to `Context::data_mut` (`context.rs:1032`)
through deref coercion, exactly the call shape already used at `menu.rs:586` inside egui's own
`CollapsingHeader`/menu implementation.

**Code sketch:**

```rust
// src/overlay/rule_editor.rs — new helper shared by render_name and render_classes
/// Ephemeral, presentation-only "is this optional-value panel open" state, keyed by an id salted
/// with the rule's `RuleId` and a control discriminator. Deliberately NOT derived from
/// `Option::is_some()` every frame (Pattern 1) — `class` and `name` have no natural non-empty
/// seed value to write on enable (D-09/D-11 forbid one), so a derived toggle would read back
/// `false` the very next frame and the panel would flicker shut before the player could act.
/// Deliberately NOT stored in `overlay::edit`/`WorkingRules` either: it carries no correctness
/// stake in the emitted filter (PITFALLS.md §8's accepted "no test story" surface) and inventing
/// a second persisted identity scheme alongside `RuleId` would violate D-13's spirit.
fn panel_open(ui: &mut egui::Ui, salt_id: egui::Id, seed: bool) -> bool {
    *ui.data_mut(|d| d.get_temp_mut_or(salt_id, seed))
}

fn set_panel_open(ui: &mut egui::Ui, salt_id: egui::Id, open: bool) {
    ui.data_mut(|d| d.insert_temp(salt_id, open));
}

fn render_name(ui: &mut egui::Ui, rule: &mut Rule, id: RuleId) -> bool {
    let mut changed = false;
    ui.label(egui::RichText::new("NAME").color(theme::LABEL_TAN).small());

    let open_id = egui::Id::new((id, "name-open"));
    let mut enabled = panel_open(ui, open_id, rule.name.is_some());

    ui.horizontal(|ui| {
        if widgets::circular_toggle(ui, &mut enabled).changed() {
            set_panel_open(ui, open_id, enabled);
            if !enabled {
                edit::clear_name(rule);          // D-12: unticking discards
                changed = true;
            }
            // Ticking ON writes nothing (D-11) -- the field stays empty until typed.
        }
        ui.label("has a name");
    });

    if enabled {
        // A per-rule text buffer, keyed the same way -- see Pattern 3 for why this can't
        // just be `rule.name.clone().unwrap_or_default()` re-read fresh every frame.
        let buf_id = egui::Id::new((id, "name-buf"));
        let mut buf = ui
            .data_mut(|d| d.get_temp_mut_or_insert_with(buf_id, || rule.name.clone().unwrap_or_default()))
            .clone();

        let response = ui.add(
            egui::TextEdit::singleline(&mut buf)
                .id_salt((id, "name-edit"))
                .hint_text("unnamed"),
        );
        if response.changed() {
            ui.data_mut(|d| d.insert_temp(buf_id, buf.clone()));
        }
        if response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
            edit::set_name(rule, &buf);
            changed = true;
            response.request_focus(); // Pattern 3: keep typing without reaching for the mouse
        }
    }

    changed
}
```

**Why `class`'s fix is the same shape but does not need a text buffer:** `render_classes` gains an
identical `panel_open`/`set_panel_open` pair gating whether the four `CollapsingHeader` groups
render at all (D-07). The checkbox loop underneath is **completely unchanged** from Phase 14 — only
the wrapping `if enabled { ... }` and the toggle's own state source are new. `edit::set_class`'s
existing empty-vec-collapses-to-`None` behavior is untouched and still does the real work of
representing "no classes selected."

### Pattern 2: Deferred per-item removal for the base-type list (answers Q2's mutation hazard)

**What:** Render the base-type list from an **owned clone** of `when.base_type` taken once at the
top of the function, not a live borrow — this sidesteps the borrow-checker conflict of iterating
`&when.base_type` while wanting to call `&mut when.base_type` from inside the same loop, and
(more importantly) avoids the exact hazard `app.rs`'s own header comment already names for the row
list: *"the array itself must not be mutated while it is being walked... mutations happen once,
after the loop ends"* (`app.rs:370-378`, describing `RowAction`). The list here is short (a
handful of item names, each a small `String`) and rendered every frame anyway, so cloning it is not
a performance concern — the same reasoning `RESEARCH Pattern 5`'s autosave section already applied
to `dynamic.json`'s full size (well under the threshold where cloning matters).

**Code sketch:**

```rust
// src/overlay/rule_editor.rs
fn render_base_types(ui: &mut egui::Ui, rule: &mut Rule, id: RuleId) -> bool {
    let mut changed = false;
    ui.label(egui::RichText::new("BASE TYPES").color(theme::LABEL_TAN).small());

    // Owned clone -- NOT a borrow of `rule.when.base_type` -- so the remove button below can
    // freely call `edit::remove_base_type(when, i)` without fighting the borrow checker or
    // mutating the array mid-walk (mirrors app.rs's RowAction collect-then-apply discipline,
    // scoped to this one function instead of the whole row loop).
    let items: Vec<String> = rule
        .when
        .as_ref()
        .and_then(|w| w.base_type.clone())
        .unwrap_or_default();

    let mut remove_at: Option<usize> = None;
    for (i, name) in items.iter().enumerate() {
        ui.horizontal(|ui| {
            if ui.small_button("\u{2715}").clicked() {
                remove_at = Some(i); // recorded, not applied -- Pattern 2
            }
            ui.add(egui::Label::new(name).wrap());
        });
    }
    if let Some(i) = remove_at {
        if let Some(when) = rule.when.as_mut() {
            edit::remove_base_type(when, i);
        }
        changed = true;
    }

    // The add field -- see Pattern 3 for the commit-on-Enter mechanics and Pattern 4 for
    // parse_base_type_entry (D-14's quote-stripping/splitting).
    let buf_id = egui::Id::new((id, "base-type-buf"));
    let mut buf = ui.data_mut(|d| d.get_temp_mut_or_default::<String>(buf_id)).clone();
    let response = ui.add(
        egui::TextEdit::singleline(&mut buf)
            .id_salt((id, "base-type-edit"))
            .hint_text("Add base type\u{2026}"),
    );
    if response.changed() {
        ui.data_mut(|d| d.insert_temp(buf_id, buf.clone()));
    }
    let commit = (response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)))
        || ui.small_button("+ Add").clicked();
    if commit {
        let parsed = edit::parse_base_type_entry(&buf);
        if !parsed.is_empty() {
            let when = edit::when_mut(rule);
            for name in parsed {
                edit::add_base_type(when, &name);
            }
            changed = true;
        }
        ui.data_mut(|d| d.insert_temp(buf_id, String::new()));
        response.request_focus();
    }

    changed
}
```

**Note on the remove glyph:** `\u{2715}` (`✕`) is the exact codepoint `widgets::header_plate`
already found renders as a tofu box in both egui's default font and Cinzel (Phase 13's own
documented lesson, restated in `widgets.rs`'s module doc and `RESEARCH Pitfall 4` from Phase 14).
**Do not reuse `ui.small_button("\u{2715}")` as written above without checking the glyph first** —
this sketch names the *semantic* choice (a close/remove glyph); if it tofus, the fix is the same
one already applied twice in this codebase: either fall back to a short text label
(`ui.small_button("Remove")`, matching `app.rs`'s existing plain-text `Delete` button) or a
hand-painted glyph via `egui::Painter` (`widgets::header_plate`'s own crossed-line-segments
approach). Flagging this explicitly so it is checked once during implementation rather than
rediscovered — see Common Pitfalls below.

### Pattern 3: Commit-on-Enter, confirmed against the `TextEdit` doc comment itself (answers Q1)

**What:** `egui-0.36.1/src/widgets/text_edit/builder.rs:26-43` ships this exact pattern as its own
usage example:

```rust
// Quoted verbatim from egui-0.36.1's own TextEdit doc comment (builder.rs:33-43)
let response = ui.add(egui::TextEdit::singleline(&mut my_string));
if response.changed() {
    // …
}
if response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
    // …
}
```

This is not a sketch — it is the API author's own documented usage. `response.lost_focus()` alone
is insufficient (clicking away, Tab, or the accordion collapsing would also trigger it); gating on
`key_pressed(Key::Enter)` in the same frame is what distinguishes "committed by Enter" from "lost
focus for some other reason."

**Confirmed: `singleline` mechanically cannot produce a newline**, closing D-05's "no validation
needed" claim for the specific risk of a name/base-type breaking `Show # {label}` filter syntax
(Risk 2 below). Source-read at `builder.rs:100-137` (doc: *"No newlines (`\n`) allowed. Pressing
enter key will result in the `TextEdit` losing focus"*) and confirmed in the actual event-handling
code at `builder.rs:1108-1170`:
- `Event::Text(text_to_insert)`: `if ... text_to_insert != "\n" && text_to_insert != "\r"` — a
  typed newline character is dropped before insertion.
- `Event::Paste(text_to_insert)`, non-multiline branch: `let single_line =
  text_to_insert.replace(['\r', '\n'], " ");` — a pasted multi-line string has every newline
  replaced with a space before insertion.
- The Enter key itself, non-multiline branch: `ui.memory_mut(|mem| mem.surrender_focus(id));
  break;` — no character is inserted at all; the widget simply loses focus, which is exactly what
  `response.lost_focus()` observes on the next check.

**Keeping focus after commit:** `Response::request_focus()`
(`egui-0.36.1/src/response.rs:376-379`) is a thin wrapper —
`self.ctx.memory_mut(|mem| mem.request_focus(self.id))` — and `Memory::request_focus`
(`memory/mod.rs:918-920`) writes `self.focus_mut().focused_widget = Some(FocusWidget::new(id))`
**synchronously**, into the same `Memory` egui's *next* interaction pass reads from. Calling it
immediately after detecting the Enter-commit (same frame, right after `.lost_focus()` was true) is
correct and sufficient — **no "must be deferred to the following frame" workaround is needed**,
because the widget that just lost focus this frame will not be interacted with again until the
next frame regardless; by the time that next frame's `TextEdit::show` runs, `Memory` already says
this id is the focused one, so it draws with an active cursor and accepts the next keystroke. This
directly answers the second half of Q1: typing four base names in a row without touching the mouse
works because every commit re-requests focus on the same id it just cleared.

**Salting:** `TextEdit`'s default id (when no `.id_salt()`/`.id()` is given) falls back to
`ui.next_auto_id()` (`builder.rs:506-512`) — position-in-widget-tree-based, not title-based like
`CollapsingHeader`'s default. This is safer than `CollapsingHeader`'s title-collision risk, but
still **use an explicit `.id_salt((id, "name-edit"))` / `.id_salt((id, "base-type-edit"))`**
(matching the existing convention every other control in `rule_editor.rs` already follows for
`ComboBox::from_id_salt((id, "style"))`), because `next_auto_id()`'s stability depends on widgets
being added in the same order every frame — the base-type list's own item count changes as entries
are added/removed, and if the add-field's id were derived from auto-ordering rather than a fixed
salt, removing an item earlier in the list could shift the auto-id sequence and silently reassign
the add-field's persisted cursor/focus state to a different id between frames. A fixed
`(RuleId, "base-type-edit")` salt has no such dependency — it is stable regardless of how many
items are currently in the list.

**No pitfall found for `TextEdit` nested inside `CollapsingHeader` inside `ScrollArea`.** Reading
`text_edit/builder.rs`, `collapsing_header.rs`, and `scroll_area.rs` together shows no
container-specific interaction: `TextEdit` interacts via ordinary `ui.interact`/focus-memory
machinery identical to any other widget, and nothing in `CollapsingHeader` or `ScrollArea` filters
or intercepts keyboard events bound for a focused child. This mirrors Phase 14 RESEARCH Pattern 3's
own finding for nested `CollapsingHeader`+`ScrollArea` (no version-specific issue found in source) —
extended here to confirm `TextEdit` specifically. MEDIUM confidence on the *felt* experience (does
scrolling ever steal focus mid-type) — no window was opened this session; flagged as a short manual
check below, not a blocker.

### Pattern 4: `parse_base_type_entry` — pure, testable quote/paste hygiene (answers D-14)

**What:** A pure function in `overlay::edit`, following the exact "logic in the library, presentation
calls it" split every other setter in this module already uses. Handles D-14's two cases (typing a
bare name; pasting a quoted, space-separated list copied straight out of a hand-written `.filter`
file) plus D-05's allowed hygiene (trim, drop-empty, no duplicate — checked by the caller via
`add_base_type`, mirroring `set_class`'s existing `if !classes.iter().any(...)` dedup guard).

```rust
// src/overlay/edit.rs — pure, no egui types
/// D-14: splits a pasted quoted list (`"Heavy Belt" "Utility Belt"`) into separate entries, and
/// strips a single pair of surrounding quotes from a bare typed name (`"Heavy Belt"` ->
/// `Heavy Belt`). Trims whitespace and drops empty results (D-05: hygiene, not validation --
/// no base name is rejected or altered beyond quote/whitespace stripping).
pub fn parse_base_type_entry(raw: &str) -> Vec<String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Vec::new();
    }
    if trimmed.contains('"') {
        // Quoted-list form: extract every "..." span, in order, ignoring anything between them
        // (matches the exact `BaseType == "A" "B"` shape the author copies from hand-written
        // filters and CLAUDE.md's own vocabulary table).
        let mut out = Vec::new();
        let mut chars = trimmed.char_indices().peekable();
        while let Some((start, c)) = chars.next() {
            if c == '"' {
                if let Some(end) = trimmed[start + 1..].find('"') {
                    let inner = trimmed[start + 1..start + 1 + end].trim();
                    if !inner.is_empty() {
                        out.push(inner.to_string());
                    }
                    // Skip past the closing quote.
                    while let Some((i, _)) = chars.peek() {
                        if *i > start + 1 + end { break; }
                        chars.next();
                    }
                }
            }
        }
        out
    } else {
        vec![trimmed.to_string()]
    }
}

/// EDIT-10: appends `name` unless it is already present verbatim (D-05: "ignoring exact
/// duplicates" is hygiene, mirrors `set_class`'s own dedup guard exactly).
pub fn add_base_type(when: &mut When, name: &str) {
    let list = when.base_type.get_or_insert_with(Vec::new);
    if !list.iter().any(|existing| existing == name) {
        list.push(name.to_string());
    }
}

/// EDIT-10/EDIT-08: removes the entry at `index`. Collapses to `None` on the last removal --
/// MUST replicate `set_class`'s empty-vec guard (Risk 1): `src/emitter.rs`'s BaseType emission
/// has no emptiness check either, so `Some(vec![])` would emit a malformed, empty
/// `BaseType == ` line with the same failure mode the invalid `Class "Tablet"` rows had in v0.2.
pub fn remove_base_type(when: &mut When, index: usize) {
    if let Some(list) = when.base_type.as_mut() {
        if index < list.len() {
            list.remove(index);
        }
        if list.is_empty() {
            when.base_type = None;
        }
    }
}

/// EDIT-08: clears the base-type condition entirely (used if a future "off switch" for the whole
/// base-type block is added; not required by D-01..D-16 as scoped, since removal is per-item).
pub fn clear_base_type(when: &mut When) {
    when.base_type = None;
}

/// EDIT-11: sets the rule's name. Trims but does not otherwise validate (D-05's hygiene
/// principle extended to names, Risk 3): an all-whitespace name collapses to no name at all
/// rather than a rule that is "named" with nothing visible in it.
pub fn set_name(rule: &mut Rule, name: &str) {
    let trimmed = name.trim();
    rule.name = if trimmed.is_empty() { None } else { Some(trimmed.to_string()) };
}

/// EDIT-11/EDIT-08: clears the name (D-12's off switch).
pub fn clear_name(rule: &mut Rule) {
    rule.name = None;
}
```

**Why `set_name` trims to `None` rather than storing `Some("")`:** if the toggle is switched on
(Pattern 1 flips the ephemeral panel-open bool) and the player commits an empty or all-whitespace
buffer, storing `Some("")` would be syntactically safe (`Show # ` — an empty comment, not malformed
filter syntax, since `singleline` already forecloses the one genuinely unsafe character) but would
silently defeat D-11's derived-label fallback: `rule.name.as_deref().unwrap_or(&derived_label)` in
`src/emitter.rs:336-337` only falls back to `rule {idx}` when `rule.name` is `None`, not when it is
`Some("")`. Normalizing empty-after-trim to `None` in the setter keeps the derived-label fallback
meaningful regardless of what the toggle's ephemeral open-state is doing. The panel can stay
visually "open" (empty text field, hint text showing) while `rule.name` itself stays `None` until
real text is committed — these are deliberately decoupled (Pattern 1's whole point).

### Anti-Patterns to Avoid

- **Deriving the `class`/`name` toggle's checked-state from `Option::is_some()` every frame**, the
  same way `rarity`/`sockets`/`quality`/`corrupted`/`unidentified_tier` do. This is the flicker bug
  in the Summary — looks like a natural extension of the existing uniform pattern, and is exactly
  wrong for these two controls specifically.
- **Seeding `when.class = Some(vec![])` or `rule.name = Some(String::new())` on toggle-on** to make
  a derived-toggle design "work" without ephemeral state. For `class` this is a hard filter-syntax
  defect (empty `Class`/`BaseType` line, same class as the v0.2 `"Tablet"` bug); for `name` it is
  syntactically safe but silently defeats D-11's fallback. Pattern 1 avoids needing either.
- **A live borrow-then-mutate loop over `rule.when.base_type` for the remove buttons.** Clone first
  (Pattern 2); the list is small enough that cost is not a real concern, and the alternative either
  fights the borrow checker or reintroduces the mutate-while-iterating hazard `app.rs`'s own
  `RowAction` comment already names for exactly this class of problem.
- **A text glyph (`✕`, `▲`/`▼`, etc.) for the remove button without checking it renders first.**
  This codebase has hit this exact failure twice already (Phase 13's header close control, Phase
  14 Pitfall 4's reorder arrows) — see the note under Pattern 2.
- **A `TextEdit::multiline` for the name or base-type add field.** Beyond D-01's explicit rejection
  of a textarea for base types, `multiline` allows literal `\n` (Enter inserts a newline by default
  rather than committing) — the exact character `singleline` exists to forbid, and the one thing
  that could actually corrupt `Show # {label}`'s filter syntax (Risk 2).

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Commit-on-Enter text entry | A custom key-event listener tracking Enter presses across frames | `response.lost_focus() && ui.input(\|i\| i.key_pressed(Key::Enter))` | This is `TextEdit`'s own documented usage pattern (builder.rs:33-43), not a workaround |
| Preventing newlines in a name/base-type field | A custom input filter stripping `\n` from the buffer on every keystroke | `TextEdit::singleline` | Already does this, for both typed and pasted input, confirmed in the event-handling source (Pattern 3) |
| "Panel stays open independent of the model" state | A new field threaded through `OverlayApp`/`WorkingRules`, mirroring `expanded`/`confirm_delete` | `ui.data_mut(\|d\| d.get_temp_mut_or(id, seed))` | egui already ships exactly this id-keyed ephemeral store — it is the same mechanism `CollapsingHeader` itself uses internally for open/closed memory when not given an explicit `.open()` |
| Deferred-removal-from-a-list-while-rendering-it | A custom generation/versioning scheme to detect "did the list change under me" | Clone the list for display, collect one `Option<usize>`, apply after the loop (Pattern 2) | Directly mirrors the `RowAction` pattern `app.rs` already established for the same class of problem one file over |

**Key insight:** this phase's only genuinely new mechanism is Pattern 1 (ephemeral toggle state) —
everything else is either a documented `TextEdit` usage pattern or a direct rescaling of a pattern
`app.rs`/`rule_editor.rs` already use elsewhere in this codebase.

## Common Pitfalls

### Pitfall 1: The `class`/`name` toggle flickers shut one frame after being switched on

**What goes wrong:** The picker/text field appears for exactly one frame after the toggle click,
then vanishes before the player can interact with it.
**Why it happens:** The toggle's checked-state is derived fresh each frame from
`Option::is_some()`, and switching on writes nothing to that `Option` (by design — D-09/D-11 forbid
a default class/name), so the very next frame's derivation reads `false` again.
**How to avoid:** Pattern 1 — id-keyed ephemeral state (`ui.data_mut`) that is written once by the
click handler and left alone by the render loop, not re-derived from the model every frame.
**Warning signs:** A manual test that toggles the control on and immediately checks a box/types a
character will not catch this if the click and the keystroke land in the same frame (a fast
scripted test could mask it); the honest reproduction is "toggle on, wait one repaint before doing
anything else, observe whether the panel is still there."

### Pitfall 2: `remove_base_type`/`add_base_type` missing the empty-vec-to-`None` collapse

**What goes wrong:** Removing the last base type from a rule leaves `when.base_type = Some(vec![])`
instead of `None`; the emitter (no emptiness guard, confirmed at `emitter.rs:411-419`, same as the
`class` line) writes a malformed `BaseType == ` line with nothing after it, which fails to load
in-game — the exact defect class the invalid `Class "Tablet"` rows had in v0.2.
**Why it happens:** Unlike `class`, there is no existing `set_base_type` function to copy this
discipline from — it must be written fresh for this phase (`code_context`'s claim that the
model/emitter groundwork is "already built" does not extend to this specific edit.rs guard, which
is new presentation-adjacent logic, not model/emitter logic).
**How to avoid:** Pattern 4's `remove_base_type` collapses to `None` on the last removal, mirroring
`set_class` exactly. Cover it with the same shape of test `set_class`'s own suite already has
(`deselecting_the_last_remaining_class_yields_none_never_an_empty_vec`).
**Warning signs:** `generate`/`artifact_guard` would not catch this unless a plan-added test
specifically exercises "remove the only base type, then generate, assert no `BaseType` line at
all" — the byte-for-byte fixture oracle only guards `config.json`'s own fixed content, not new
dynamic-editor code paths.

### Pitfall 3: `rule.name = Some("")` silently defeating the derived-label fallback

**What goes wrong:** A rule shows as "named" (toggle checked, header shows the ` - ` separator with
nothing after it, or nothing at all) but the emitted filter comment reads `Show # ` instead of the
helpful `Show # rule 3` an unnamed rule would have gotten.
**Why it happens:** `rule.name.as_deref().unwrap_or(&derived_label)` only falls back when `name` is
literally `None` — `Some("")` (or `Some("   ")`) is not caught by that check.
**How to avoid:** Pattern 4's `set_name` trims and normalizes empty-after-trim to `None` in the
setter itself, so the model can never end up in this state regardless of what the ephemeral
toggle's open/closed presentation state is doing.
**Warning signs:** A `dynamic.json` with `"name": ""` on a rule after a save — should never occur
if `set_name` is the only write path; worth a test asserting `set_name(rule, "   ")` yields
`rule.name == None`.

### Pitfall 4: Reusing `✕` (U+2715) for the base-type remove button without checking it renders

**What goes wrong:** A repeat of Phase 13's header-close-control and Phase 14's reorder-arrow
tofu-box defects, on a third control.
**Why it happens:** Neither egui's default font nor the Cinzel display face is guaranteed to
contain arbitrary Miscellaneous Symbols glyphs — this has now failed twice in this exact codebase.
**How to avoid:** Verify the glyph renders before committing to it (a look-at-it check, not
something a unit test catches); fall back to a short text label (`"Remove"`, matching the existing
`Delete` button's own established idiom in `app.rs`) or a hand-painted glyph via `egui::Painter`
if a compact icon is still wanted.
**Warning signs:** None at compile time — purely visual. Route to the same short manual-check list
Phase 14 used for its own glyph risk.

### Pitfall 5: Forgetting to update `rule_editor.rs`'s own module-doc EDIT-08 inventory

**What goes wrong:** The file's header doc comment (lines 19-42) currently states, as a documented
fact with reasoning attached: *"classes — off by deselecting every class in every group;
`edit::set_class` already collapses the last deselection to `None`... so no separate 'classes off'
toggle is needed here."* This sentence becomes **false** the moment this phase ships `class`'s own
explicit `circular_toggle` (D-06/D-07) — the inventory is exactly the kind of self-documenting
audit trail this file's own doc comment says exists ("this inventory is auditable by reading this
file, not by clicking through the accordion"), and it must be edited in the same plan that adds the
toggle, not left to silently drift into a wrong statement about the very file it documents.
**Why it happens:** The doc block was accurate when written (Phase 14) and nothing about adding a
new function forces an editor to revisit unrelated prose above it.
**How to avoid:** Treat updating this doc block as an explicit task/acceptance-criterion line item,
not an incidental side effect — the same way Phase 14's own plans grepped for anti-patterns as
acceptance criteria.
**Warning signs:** A reviewer reading the module doc after this phase ships and finding it
contradicts the actual `render_classes` code.

## Code Examples

### The commit-on-Enter pattern, quoted verbatim from egui's own source

```rust
// egui-0.36.1/src/widgets/text_edit/builder.rs:33-43 (TextEdit's own doc example)
let response = ui.add(egui::TextEdit::singleline(&mut my_string));
if response.changed() {
    // …
}
if response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
    // …
}
```

### Row header format for D-10

```rust
// src/overlay/app.rs — row_header_text, extended
fn row_header_text(row: &view_model::RuleRow) -> String {
    let marker = if row.enabled { "" } else { " [DISABLED]" };
    match &row.name {
        Some(name) if !name.is_empty() => {
            format!("{:02}  {}{} - {}", row.ordinal, row.style, marker, name)
        }
        _ => format!("{:02}  {}{}", row.ordinal, row.style, marker),
    }
}
```

`view_model::RuleRow.name: Option<String>` already exists and is already populated from
`rule.name.clone()` (`view_model.rs:61`) — this is purely a formatting change, no new plumbing.
Note the CONTEXT's literal format is `<index> <style>[ - name]` (D-10) — the sketch above keeps the
`[DISABLED]` marker positioned exactly where it already sits today (immediately after style, before
the name), since D-10 only specifies where the name goes relative to `<style>`, and moving the
`[DISABLED]` marker was not asked for.

## State of the Art

| Old Approach (Phase 14) | Current Approach (Phase 15) | When Changed | Impact |
|--------------------------|------------------------------|---------------|--------|
| Every condition's toggle derives checked-state from `Option::is_some()` every frame | `class`/`name` back their toggle with `ui.data_mut`'s id-keyed ephemeral storage instead | This phase | New, narrowly-scoped exception to an otherwise-uniform pattern — Pattern 1 explains exactly why these two (and only these two) need it |
| `rule_editor.rs` has no `TextEdit` usage at all (only `ComboBox`/`DragValue`/`Slider`/checkboxes) | Two `TextEdit::singleline` fields (name, base-type add) | This phase | First text-entry control in the editor; Pattern 3 is the first-use reference for any future free-text field |
| `row_header_text` shows `{ordinal} {style}{marker}` only | Gains an optional ` - {name}` suffix (D-10) | This phase | One-line formatting change, no new data plumbing (`RuleRow.name` already existed, unused) |

**Deprecated/outdated:** Nothing from 14-RESEARCH.md is stale — this phase's controls sit inside
the exact `render_rule_body` vertical stack Phase 14 established, using the same
`circular_toggle`/`edit::when_mut` idioms for the parts that do reuse the uniform pattern.

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|----------------|
| A1 | `TextEdit` nested inside `CollapsingHeader` inside `ScrollArea` has no version-specific focus-stealing or scroll-interaction quirk in egui 0.36.1 | Pattern 3 | Low — no window was opened this session to observe the *felt* interaction; source reading found no container-specific event interception, and Phase 14's own research made an analogous claim for `CollapsingHeader`+`ScrollArea` without a `TextEdit` in the mix. Worth a specific manual check: expand a rule, type into the name field, and confirm no stray scroll or focus jump happens mid-keystroke |
| A2 | Cloning `when.base_type` once per frame for display (Pattern 2) has no perceptible performance cost | Pattern 2 | Very low — the list is at most a handful of short strings, rendered at 460px scale; 14-RESEARCH's Pattern 5 already established that whole-`Config` clones/serializes at this data scale are sub-millisecond on local NTFS, and this is a far smaller structure |
| A3 | `ui.data_mut`'s temp storage (Pattern 1) is not garbage-collected mid-session in a way that would reset an open panel while the player is actively looking at it | Pattern 1 | Low-Medium — egui's `IdTypeMap` temp-data GC prunes entries not *touched* for some number of frames, but this data is touched every frame the control renders (i.e., every frame the rule's accordion is expanded), so it cannot be pruned while visibly in use. It could reset to the model-derived seed if the row is collapsed and left untouched for a long time and then reopened — treated here as acceptable, matching the author's own "either way works for now" tone on D-08's data-loss question, not as a defect |

## Open Questions

1. **Exact visual treatment of the base-type remove control** (glyph vs. text label, size,
   placement relative to the wrapped name).
   - What we know: `\u{2715}` is a near-certain repeat of a glyph-rendering defect this codebase
     has already hit twice; a text label (`"Remove"`) is the proven-safe fallback already used for
     row deletion in `app.rs`.
   - What's unclear: Whether a compact icon is worth a hand-painted `Painter` glyph (matching
     `header_plate`'s and `widgets::arrow_button`'s own precedent) or whether a small text button
     reads fine at this list's typical length (a handful of items).
   - Recommendation: Start with the proven-safe text label; treat a hand-painted glyph as a
     polish upgrade, not a blocking requirement — this is explicitly a look-at-it call
     (`PITFALLS.md` §8), not a mechanism question this research can resolve further without a
     running window.
2. **Whether a long base name (e.g. "Expert Feathered Sallet") wrapped via `egui::Label::new(name)
   .wrap()` reads comfortably next to a remove button inside 460px**, or needs the remove control
   moved to its own line below the wrapped name instead of beside it.
   - What we know: `egui::Label::wrap()` is already proven working in this exact file's row-summary
     rendering (`theme::row_summary` + `.wrap()` in `app.rs`'s `row_summary_label`), and a
     `ui.horizontal` layout gives each subsequent widget the *remaining* available width (which
     naturally shrinks after the remove button claims its own fixed width first), so the wrap width
     the label receives should already exclude the button's space.
   - What's unclear: Whether the visual result (button top-aligned beside a two-line wrapped label)
     reads cleanly or needs the stacked-vertical layout the file's own convention comment
     recommends for "the rest of this file" instead.
   - Recommendation: Try the horizontal layout first (Pattern 2's sketch); if a look-at-it check
     finds it awkward, fall back to a two-line-per-item layout (name on its own wrapped line,
     remove button right-aligned beneath it) — a small, contained change with no effect on the
     pure `edit.rs` layer either way.

## Environment Availability

No new external dependencies, tools, or services. Everything needed (Rust toolchain, the pinned
`egui`/`eframe` 0.36.1, Windows 11) is already established and verified available by Phases 11-14.

## Validation Architecture

Skipped — `workflow.nyquist_validation` is `false` in `.planning/config.json`.

## Security Domain

`security_enforcement` is absent (treated as enabled), but as in Phases 13-14 this phase has
essentially no traditional attack surface: no network, no auth, the only "untrusted" input is a
local, single-user player typing into their own overlay.

### Applicable ASVS Categories

| ASVS Category | Applies | Standard Control |
|----------------|---------|-------------------|
| V2 Authentication | No | No accounts, no auth surface |
| V3 Session Management | No | No sessions |
| V4 Access Control | No | Single-user local process |
| V5 Input Validation | Marginal — two new free-text fields | Both route through `TextEdit::singleline`, which is structurally incapable of producing the one character (`\n`) that could corrupt the emitted `Show # {label}` filter syntax (Pattern 3); `set_name`/`add_base_type` normalize whitespace but perform no content validation, matching D-05's explicit instruction not to |
| V6 Cryptography | No | No secrets, no network calls |

### Known Threat Patterns for this stack

| Pattern | STRIDE | Standard Mitigation |
|---------|--------|-----------------------|
| A player types/pastes a name or base-type entry containing a raw newline, intending to inject a second, unprefixed line into the emitted filter (e.g. a stray `Hide` or malformed condition line masquerading as part of the comment) | Tampering (self-inflicted — the player is editing their own local file) | Structurally foreclosed by `TextEdit::singleline`'s own event handling (Pattern 3) — not a mitigation this phase's code needs to implement, since the widget choice already prevents the character from ever reaching the buffer, on both typed and pasted input |
| A base-type list is emptied to zero entries via repeated removal, and the resulting `Some(vec![])` (if the empty-collapse guard were missing) reaches `dynamic.json` via autosave and then `dynamic.filter` via Apply, producing a malformed `BaseType == ` line the game fails to parse | Denial of Service (local, in-game — a broken filter) | `remove_base_type`'s `None`-collapse guard (Pattern 4/Pitfall 2), mirroring `set_class`'s existing, tested discipline |

## Sources

### Primary (HIGH confidence — read directly against the pinned version / this repo's own code)
- `egui-0.36.1` installed source
  (`C:/Users/jonat/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/egui-0.36.1/`):
  `src/widgets/text_edit/builder.rs` (doc example at lines 33-43; `singleline`/`multiline` at
  100-137; `id_salt`/`hint_text`/`desired_width` at 160-330; event handling — `Event::Paste`,
  `Event::Text`, Enter-key branch — at 1090-1170; default id derivation via `next_auto_id` at
  506-512; `impl Widget for TextEdit<'_>` at 414), `src/response.rs` (`request_focus` at 376-379,
  `lost_focus`/`has_focus`/`gained_focus` at 348-376), `src/memory/mod.rs` (`Memory::request_focus`
  at 918-920, `Focus`/`focused_widget` machinery at 502-684), `src/util/id_type_map.rs`
  (`get_temp_mut_or`/`insert_temp`/`get_persisted_mut_or` at 424-536), `src/ui.rs` (`impl Deref for
  Ui { type Target = Context; ... }` at 91-97), `src/context.rs` (`Context::data_mut` at 1032),
  `src/containers/collapsing_header.rs` (internal open/closed memory via `insert_persisted`/
  `get_persisted` at 32-43, confirming the same mechanism this research reuses for Pattern 1),
  `src/containers/menu.rs` (line 586, an in-tree example of `ui.data_mut` called directly on a
  `&mut Ui`, confirming the deref-coercion call shape compiles and is used by egui's own code)
- This repo: `src/overlay/rule_editor.rs`, `src/overlay/edit.rs`, `src/overlay/view_model.rs`,
  `src/overlay/app.rs` (rows 1-260, 330-610, 760-838), `src/overlay/widgets.rs`,
  `src/overlay/theme.rs`, `src/models/config.rs` (`When`/`Rule` field definitions and doc comments
  at 300-380), `src/emitter.rs` (rule-label derivation at 336-337, condition emission order and the
  `BaseType`/`Class` lines' lack of an emptiness guard at 320-430), `src/overlay/layout.rs`
  (`STRIPE_WIDTH = 460.0`)
- `.planning/phases/14-rule-editor-apply/14-RESEARCH.md` (this repo) — Pattern 5's file-size/clone-
  cost reasoning, reused for Assumption A2; Pattern 3's `CollapsingHeader`+`ScrollArea` nesting
  finding, extended here for `TextEdit`
- `.planning/research/PITFALLS.md` (this repo) — §8 (GUI has no test story), §9 (ordinal identity
  is fragile), §10 (class lists must not be hardcoded — not touched by this phase, restated only
  because `class`'s new toggle sits directly beside the untouched config-sourced picker)

### Secondary (MEDIUM confidence)
- None — every claim in this research was verified against the installed source directly.

### Tertiary (LOW confidence)
- None retained as fact — the two remaining uncertainties (glyph choice, exact wrap-layout feel)
  are called out in Open Questions rather than stated as findings.

## Metadata

**Confidence breakdown:**
- Commit-on-Enter / focus retention (Q1): HIGH — the pattern is `TextEdit`'s own documented usage
  example, and the focus-retention mechanism was traced through `Response::request_focus` into
  `Memory::request_focus`'s actual mutation, not inferred from behavior
- Newline exclusion / filter-syntax safety (Q5, Risk 2): HIGH — confirmed in the literal
  event-handling source for both typed (`Event::Text`) and pasted (`Event::Paste`) input paths
- The `class`/`name` toggle-flicker finding and its ephemeral-state fix (Q3): HIGH on the API
  (`get_temp_mut_or` confirmed present and exactly shaped for this use, and its underlying
  mechanism is literally what `CollapsingHeader` itself uses); MEDIUM on whether the exact GC/reset
  behavior (Assumption A3) is ever visually noticeable, which only a running session can fully
  settle, though the reasoning suggests it functionally cannot occur while a row is expanded
- Deferred per-item removal (Q2): HIGH — direct rescaling of `app.rs`'s own already-shipped
  `RowAction` pattern, not a new technique
- Layout in 460px (Q2's width question): MEDIUM — `Label::wrap()` is proven working elsewhere in
  this file, but the specific "button beside a two-line wrapped label" composition was not rendered
  this session; routed to Open Questions
- Testing strategy (Q4): HIGH — a direct, narrow extension of Phase 14's already-established
  pure/impure split, with no new categories of untestable logic introduced

**Research date:** 2026-09-08
**Valid until:** ~30 days for the version pin (unchanged from Phase 13/14's own estimate); the
API-shape findings (`TextEdit`, `IdTypeMap`, `Response::request_focus`) are stable within the
pinned `0.36` minor and should hold for the rest of this milestone, since no further `egui`/
`eframe` upgrade is planned before v0.4 ships.

---

## Risks for the planner

1. **Pattern 1 (ephemeral toggle state) is the one piece of genuinely new architecture in this
   phase.** Everything else is either a documented `TextEdit` usage pattern or a rescaling of an
   existing in-codebase pattern (`RowAction`, the uniform enable-toggle vocabulary). Budget it as
   its own task with an explicit acceptance criterion (e.g. "toggle `class`/`name` on with an
   automated headless test spanning two frames, assert the panel is still visible on the second
   frame without anything having been selected/typed") — this is exactly the kind of interaction
   `widgets.rs`'s existing `egui::Context::run_ui`-based headless tests (`circular_toggle_flips_
   and_reports_changed_on_click`) already prove is possible to test without opening a real window,
   so "GUI has no test story" (`PITFALLS.md` §8) is not a reason to leave this specific risk
   unverified by automation.
2. **`remove_base_type`'s empty-vec-to-`None` collapse (Pitfall 2) is a new invariant, not an
   inherited one.** Unlike `class`, there is no existing function to copy this discipline from
   verbatim — it must be written and tested fresh this phase, with the same rigor `set_class`'s own
   test suite already demonstrates (`deselecting_the_last_remaining_class_yields_none_never_an_
   empty_vec` is the exact test shape to mirror for `remove_base_type`).
3. **`rule_editor.rs`'s own module-doc EDIT-08 inventory (lines 19-42) will become incorrect the
   moment `class` gains its own toggle** (Pitfall 5) — include updating it as an explicit task
   line item, not an incidental cleanup.
4. **The glyph-rendering risk (Pitfall 4) is a near-certain third repeat of a defect this project
   has already found and fixed twice.** Default to the proven-safe text-label fallback
   (`"Remove"`) rather than a Unicode glyph, unless a quick check confirms the glyph renders —
   treat this as settled in the plan rather than a surprise found during polish.
5. **Q2's exact wrap-layout feel (button beside a two-line label) is the only remaining
   look-at-it item this research could not fully resolve from source alone.** Route it to a short
   manual-check list alongside Pitfall 4's glyph check and Assumption A1's focus/scroll check —
   three small, concrete items, far fewer than Phase 14's twelve, matching the orchestrator's own
   expectation in `<research_questions>` Q4.

## Recommended task shape

Given the risk concentration above and the existing `render_rule_body` structure, a natural split:

**Task/Plan A — Pure library layer, no widgets.**
1. `overlay::edit`: `parse_base_type_entry`, `add_base_type`, `remove_base_type`,
   `clear_base_type`, `set_name`, `clear_name` (Pattern 4) — fully unit-tested, zero egui imports,
   explicitly including the empty-collapse regression test (Pitfall 2) and the empty-after-trim
   regression test (Pitfall 3).
2. `view_model`/`app.rs::row_header_text` extension for D-10's format (Code Examples) — a small,
   already-plumbed change (`RuleRow.name` exists), with the existing header-format unit tests
   extended rather than replaced.

**Task/Plan B — Presentation layer.**
3. `render_name` and the `class` toggle extension to `render_classes` (Pattern 1) — the one place
   worth a dedicated headless test per Risk 1 above.
4. `render_base_types` (Pattern 2 + Pattern 3), including the glyph check (Pitfall 4) as an
   explicit sub-step rather than an afterthought, and the module-doc update (Pitfall 5).
5. Manual-check list: (a) toggle `class`/`name` on, wait a beat, confirm the panel is still open
   with nothing selected/typed; (b) type several base names in a row using only Enter between them,
   confirm focus never has to be reclaimed by mouse; (c) expand a rule with a long base name,
   confirm the wrapped-label-plus-remove-button layout reads cleanly at 460px; (d) confirm the
   remove glyph renders (or apply the text-label fallback pre-emptively and skip this check).

If the roadmap's plan count lands on one, Task A's item 2 (the header format) is small enough to
fold into Task B without materially changing the risk profile; Task A's item 1 (the new `edit.rs`
functions) should stay a distinct, reviewable unit regardless, since it is where Pitfalls 2 and 3
are actually closed.
