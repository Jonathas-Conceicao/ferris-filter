# Phase 14: Rule Editor + Apply - Research

**Researched:** 2026-09-07
**Domain:** egui 0.36.1 immediate-mode editing widgets over a 460px stripe; lossless JSON
round-tripping of a hand-authored `serde` model; pure-function mutation of an ordered rule list
under reordering
**Confidence:** HIGH on everything verified against the installed `egui-0.36.1` source and the
existing `src/models/config.rs` test suite (this is the strongest evidence available short of
running the editor live). MEDIUM on visual/ergonomic questions that can only be judged by looking
at the rendered stripe (accordion feel, glyph rendering in Cinzel). LOW/ASSUMED items are called
out individually, per Phase 13's own lesson that sketches drift from the real API.

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

**Editing surface**
- **D-01: Rules expand in place — an accordion, not a detail pane or a separate edit mode.**
  Clicking a rule opens its controls inline; the rows above and below stay visible.
  - **The stripe does NOT resize when a rule is expanded.** Controls stack vertically inside the
    expanded row (label above control) rather than sitting side by side, fitting the existing
    460px. Keeps SHELL-03/D-12's "never sized to the full screen" invariant trivially true.

**Persistence — the store and the filter move independently**
- **D-02: `dynamic.json` autosaves on every edit; `dynamic.filter` changes ONLY on Apply.**
- **D-03: Because of D-02 the two can diverge, and the UI must say so.** The Apply affordance must
  indicate saved-but-unapplied state — required as part of EDIT-09, not decoration.
- **D-04: Autosave must not corrupt the store.** Same atomic write-then-rename discipline
  `dynamic::write_filter` already uses (temp file in the destination directory).

**Reordering, adding, deleting**
- **D-05: Reordering is up/down arrow buttons per row, not drag-and-drop.**
- **D-06: A new rule is appended at the END of the list.** Safe because nothing an incomplete rule
  does can reach the game until the player deliberately Applies *and* reloads in-game.
- **D-07: A new rule is `{ "style": "unchanged", "when": {} }`, enabled.**
- **D-08: Deleting requires a quick confirm** — Claude's design: an inline confirm (the row itself
  becomes `Delete? [Yes] [Cancel]`), no modal, no undo stack this phase.

**Controls (the EDIT-03..08 vocabulary)**

All options come from `config.json`, never hardcoded (`PITFALLS.md` §10).

- **D-09: Style selector** — a dropdown of the 12 styles currently in `config.json`
  (`S A B C D E F Recycle unchanged hidden unidentified exceptional`), read from the file.
- **D-10: Class picker** — a grouped, collapsible checkbox list sourced from `weaponClasses` (12),
  `armourClasses` (6), `flaskCharmClasses` (3). Groups collapsed by default.
- **D-11: Rarity selector** — comparator + value, emitting the existing compact forms
  (`>=Magic`, `<=Rare`, `==Rare`, …). Not free text.
- **D-12: Defence picker** — a group toggle plus three checkboxes (armour / evasion / energy
  shield). Ticked emits `> 0`, unticked emits `0`, group off emits no defence line at all.
- **D-13: Sockets and quality** are minimum-value fields with their own enable toggles; corrupted
  is a control whose unset state omits the condition entirely. Typed numeric entry is viable
  (Phase 13 D-06/D-08: focus follows a click into the stripe).
- **D-14: Every condition control toggles off individually, and a disabled control contributes
  nothing to the emitted block.** EDIT-08, the UI expression of DYN-04 — `When`'s fields are
  already all `Option<…>`.

**Apply**
- **D-15: Apply compiles the store and writes `dynamic.filter`**, reusing `dynamic::compile_store`
  + `dynamic::write_filter` unchanged.
- **D-16: Apply states that the filter must be reloaded via Options → Game → Item Filter →
  Refresh.**
- **D-17: Apply failures surface in the stripe**, not a console the player cannot see.

**Layering**
- **D-18: Rule mutation, validation and compilation are library functions covered by unit tests;
  the egui layer holds no logic that can only be exercised by clicking.** `PITFALLS.md` §8.
- **D-19: Selection must survive reordering.** `PITFALLS.md` §9.

### Claude's Discretion
- Exact accordion layout details, provided no dynamic resize.
- Whether to reproduce circular toggles with custom painting vs. approximate with egui's own
  checkbox — a mechanism question, answered below (Q1).
- Exact inline-confirm visual treatment for delete (D-08: "can look like whatever you want").

### Deferred Ideas (OUT OF SCOPE)
- Undo / undo stack.
- Editing the style palette, class lists, or economy tiering from the overlay.
- Drag-and-drop reordering.
- Rule search / filtering.
- A settings file (hotkey binding, stripe position/width persistence).
- Dimming the game behind the stripe (closed, not deferred — see 13-CONTEXT.md).
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|-------------------|
| EDIT-01 | List rules in ordinal order; add, delete, reorder, reordering changes precedence | Pattern "RuleId working model" below gives a mutation API where add/delete/move are pure, index-returning functions; `view_model::rule_rows` already derives ordinal purely from array position |
| EDIT-02 | Enable/disable a rule (one toggle suppresses a category) | `Rule.enabled: Option<bool>` already exists and round-trips (`rule_enabled_round_trips`, verified) — this is a checkbox over an already-modeled, already-tested field |
| EDIT-03 | Style selector from `config.json`'s styles | `available_styles(&Styles) -> Vec<&'static str>` pure function (Code Examples) plus `ComboBox::new(id_salt, label)` (verified against installed source) |
| EDIT-04 | Class picker from `config.json`'s class lists | `CollapsingHeader` per group (verified `.id_salt`/`.open` API) nested in the row's own space; class lists read from `Config.weapon_classes`/`armour_classes`/`flask_charm_classes`, never hardcoded |
| EDIT-05 | Rarity selector emitting compact comparator forms | `RarityComparator`'s existing custom `Serialize`/`Deserialize` already emits exactly `>=Magic`/`<=Rare`/bare-`Rare` — the picker only needs to set `RarityOp`+`Rarity`, the model does the string work |
| EDIT-06 | Three-checkbox defence picker, all seven PoE2 armour types reachable | `emit_numeric`/`NumericCondition` semantics confirmed in `src/emitter.rs`: `min: Some(1)` → `> 0`, `eq: Some(0)` → literal `0`, absent field → no line. The picker only needs to set/clear these two variants per checkbox |
| EDIT-07 | Sockets/quality minimums + corrupted (omit when unset) | `DragValue` confirmed to enter keyboard-edit mode on a single click (source-verified); corrupted resolved as an enable-toggle + binary value control, not a bespoke tri-state widget (Q1 below) |
| EDIT-08 | Every condition control toggles off individually | Direct consequence of `When`'s all-`Option` fields plus the uniform "enable toggle + value widget" architecture recommended below |
| EDIT-09 | Apply writes `dynamic.filter` + reload instruction | `dynamic::compile_store`/`write_filter` unchanged (Phase 12); D-03's unapplied-changes indicator is new UI state, mechanism given below |
</phase_requirements>

## Summary

The highest-risk question (Q4, round-tripping) turns out to be the **best-supported** claim in this
research, not the weakest: `Config` already has an existing, passing test
(`config_with_rules_round_trips` in `src/models/config.rs`) that deserializes the real
`config.json`, re-serializes it, re-deserializes the result, and asserts full structural equality
— and a second one (`rule_enabled_round_trips`) doing the same for `Rule.enabled`'s
`skip_serializing_if` behavior specifically. Every field in `Config`, `Rule`, and `When` is either
a plain typed field or an `Option<T>` with `#[serde(skip_serializing_if = "Option::is_none")]`
(or, for `RarityComparator`, a hand-written `Serialize`/`Deserialize` pair whose own doc comment
states the exact round-trip normalization: `"==Rare"` → `"Rare"`). There is no
`#[serde(flatten)]` catch-all anywhere in the model and every struct carries
`#[serde(deny_unknown_fields)]`, so nothing can silently pass through unparsed and nothing can be
silently dropped on the way back out — the type system is closed by construction. The one genuine,
previously-undocumented finding this research adds: **`Config.class_groups` and
`Build.armour_types` are `HashMap<String, Vec<String>>`, and Rust's std `HashMap` iterates in an
arbitrary, per-instance-random order** `[CITED: doc.rust-lang.org std::collections::HashMap]` — so
every autosave that touches the whole `Config` (which every autosave does, since the store is one
JSON document) can silently reshuffle those two maps' key order on disk. This is not a fidelity
loss (`Config`'s `PartialEq` doesn't care about map order, and dynamic.json is git-ignored per-user
state that nothing diffs), but it is a fact worth stating rather than discovering by surprise, and
the recommended guard (a round-trip test asserting value equality, not byte equality) already
accounts for it correctly.

The second load-bearing finding is architectural rather than a library gap: egui 0.36.1's
`CollapsingHeader` supports both an explicit `.id_salt(...)` (decoupling widget identity from
display position or title text) and an explicit `.open(Some(bool))` (letting the *caller* drive
open/closed state instead of egui's internal per-id memory). Combined, these mean Q5's "ordinal
identity under mutation" problem has a clean answer that needs no egui-specific workaround: assign
a session-only `RuleId` (a simple incrementing counter, never persisted to `dynamic.json`) to each
rule at load time, keep the working model as `Vec<(RuleId, Rule)>`, and track "which rule is
expanded" as `Option<RuleId>` rather than an index. Add/delete/move become pure functions over
`Vec<(RuleId, Rule)>` with zero egui types in their signatures — directly satisfying D-18 — and
"selection survives reordering" (D-19) becomes true by construction: a `RuleId` doesn't change
when the vector around it is permuted, so there is nothing to re-resolve after a move.

Widget vocabulary is unremarkable by comparison: `CollapsingHeader`, `ComboBox`, `DragValue`,
`checkbox`, and `selectable_value` all exist with the shapes needed, each confirmed by reading the
installed source rather than trusted from memory. The circular-toggle question (PoE2's UI idiom)
does **not** require new hit-testing work — it needs only new *painting* on top of the existing
`egui::Checkbox`-adjacent interaction primitives (`ui.interact` + `Sense::click()`, exactly the
pattern `widgets::header_plate`'s `✕` already uses in this codebase), which sidesteps writing a
custom `Widget` impl from scratch.

**Primary recommendation:** build the editor as one new pure library module
(`overlay::edit` — `RuleId`, `Vec<(RuleId, Rule)>`, add/delete/move/toggle functions, each unit
tested with no `egui` import) plus a thin presentation layer that renders that model through
`CollapsingHeader`/`ComboBox`/`DragValue`/hand-painted circular toggles; autosave by writing the
whole `Config` back through `serde_json::to_string_pretty` + the same atomic
`tempfile::NamedTempFile::new_in` + `.persist()` pattern `dynamic::write_filter` already
establishes, triggered once per frame via a `dirty` flag rather than per keystroke or on a timer;
and wire Apply directly to the unchanged `dynamic::compile_store`/`write_filter` pair, tracking a
separate in-memory `applied: bool` for D-03's indicator.

## Architectural Responsibility Map

Native Windows desktop process, no browser/server/CDN tiers — using the same native-app
substitution Phase 13's research established.

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| Rule add/delete/move/toggle mutation | Application/Library (`overlay::edit`, new) | — | Pure `Vec<(RuleId, Rule)>` transforms, D-18 mandates no egui types in the signature |
| Condition editing (class/rarity/defence/sockets/quality/corrupted) value transforms | Application/Library (`overlay::edit`) | Presentation feeds it raw input | Each "set/clear this `When` field" operation is a pure function over `Rule`/`When`, unit-testable exactly like the mutation ops above |
| Available-styles / available-classes enumeration | Application/Library (`overlay::edit` or extend `view_model`) | — | Must read from the loaded `Config`, never a hardcoded list (`PITFALLS.md` §10) |
| Accordion expand/collapse state, which row is selected | Application/Library (`Option<RuleId>` held in `OverlayApp`) | Presentation drives `CollapsingHeader::open(...)` from it | D-19: must survive reordering; an index would not, a `RuleId` does |
| Autosave trigger + atomic write | Application/Library (`overlay::store::save`, new, extending `store.rs`) | Presentation sets the `dirty` flag | Reuses `dynamic.rs`'s established atomic-write discipline (D-04); belongs beside `store::load` |
| Widget rendering (`CollapsingHeader`, `ComboBox`, `DragValue`, custom-painted circular toggles) | Presentation (egui) | — | No library involvement; pure UI, `PITFALLS.md` §8's accepted "least testable" surface |
| Apply (compile + write) | Application/Library (`dynamic::compile_store`/`write_filter`, unchanged from Phase 12) | — | Already built as print-free, fully-parameterised callables for exactly this purpose |
| "Unapplied changes" indicator state | Presentation (`OverlayApp::applied: bool`, transient, in-memory only) | — | Not persisted — a UI-session concept, not a store concept; D-03 requirement, no library involvement needed |

## Standard Stack

### Core

No new *required* dependencies. Every crate this phase needs is already pinned in `Cargo.toml`
from Phase 11-13: `eframe`/`egui` 0.36 (widgets), `serde`/`serde_json` (mutation + autosave),
`tempfile` (atomic write, already used by `dynamic::write_filter`).

### Supporting (optional, discretionary)

| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| `egui_kittest` | 0.36.1 `[VERIFIED: crates.io — matches egui/eframe's exact minor]` | Headless automation testing (simulated clicks via AccessKit) for the presentation layer | **Optional, not required.** See "Testing" (Q6) below — recommended only as a thin wiring-smoke-test layer, not for pixel/snapshot assertions, and only if the planner wants presentation-layer coverage beyond the pure `overlay::edit` unit tests |

**Version verification performed:**
```bash
cargo search egui_kittest   # egui_kittest = "0.36.1"
```
Ran clean against the live crates.io index; version matches `egui`/`eframe`'s pinned `"0.36"`
minor exactly, which is the actual compatibility constraint (not just "recent").

**Installation (only if the planner adopts `egui_kittest`):**
```bash
cargo add egui_kittest@0.36 --dev
```
No `wgpu`/`snapshot` feature needed unless pixel-level snapshot testing is also wanted — those
features pull in a full GPU rendering stack and are not justified by this phase's scope (D-18
already keeps the testable logic in the library).

### Alternatives Considered

| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| Hand-painted circular toggle (`ui.interact` + `Painter`, reusing the `widgets::header_plate` pattern already in this codebase) | A custom `egui::Widget` impl from scratch, or `egui_demo_lib`'s `toggle_switch.rs` pill shape unmodified | The pill-shaped official example (cited in 13-RESEARCH.md's Don't Hand-Roll) is the right *pattern* reference but the wrong *shape* — PoE2's toggles are circular. Adapting the pattern (same four-step: size → `allocate_exact_size` → interact → paint) with a circle instead of a rounded rect is a few lines, not a rewrite |
| A bespoke tri-state widget for `corrupted` | A `ComboBox`/two-`selectable_value` pair gated by the same enable-toggle every other condition uses | A true tri-state (unset/true/false) custom widget would be the only three-state control in the editor, breaking the otherwise-uniform "enable toggle + value widget" vocabulary every other condition follows (D-14). Reusing the uniform pattern is both less code and more consistent — see Q1 below |
| Writing `dynamic.json` with `serde_json::to_writer` directly to the final path | `tempfile::NamedTempFile::new_in(store_dir)` + `.persist()`, mirroring `dynamic::write_filter` exactly | D-04 explicitly requires this; a direct write can leave a truncated/malformed `dynamic.json` if the process is killed mid-write, which is strictly worse than losing the single unsaved edit that triggered it |

## Package Legitimacy Audit

No new *required* packages this phase — `slopcheck`'s absence (confirmed again this session; `pip`
is not on `PATH` in this environment, same finding as 13-RESEARCH.md) has no bearing on the
mandatory dependency list.

| Package | Registry | Age | Downloads | Source Repo | slopcheck | Disposition |
|---------|----------|-----|-----------|--------------|-----------|-------------|
| `egui_kittest` | crates.io | since ~2024, same release cadence as `egui` | high (bundled release with every `egui` 0.3x) | github.com/emilk/egui (`crates/egui_kittest`) — same monorepo as the already-trusted `eframe`/`egui` | N/A (tool unavailable) | Approved (manual audit) — **only if adopted**; same publisher/repo as dependencies already in the tree, official first-party testing crate, not a new trust boundary |

**Packages removed due to slopcheck `[SLOP]` verdict:** none — slopcheck did not run, and no package
was flagged by manual audit either.
**Packages flagged as suspicious `[SUS]`:** none.

*Per protocol, since `slopcheck` did not run, `egui_kittest` should still be gated behind a
lightweight `checkpoint:human-verify` before its first `cargo add` if the planner adopts it —
consistent with how 13-RESEARCH.md gated `tray-icon`/`windows-sys` under the same circumstance.*

## Architecture Patterns

### System Architecture Diagram

```
┌──────────────────────────────────────────────────────────────────────────┐
│  OverlayApp::ui() — one egui frame                                       │
│                                                                            │
│  1. Poll hotkey/tray events (unchanged from Phase 13)                    │
│  2. Render header + "RULES" section (unchanged from Phase 13)            │
│  3. ScrollArea::vertical()                                               │
│     └─ for (id, rule) in working_rules (Vec<(RuleId, Rule)>):            │
│          zebra_row(ordinal) {                                            │
│            CollapsingHeader::new(...).id_salt(id).open(Some(expanded))   │
│              .show(ui, |ui| {                                           │
│                 render_style_combo(&mut rule, &available_styles)  ──┐   │
│                 render_class_picker(&mut rule, &config.*_classes) ──┤   │
│                 render_rarity_selector(&mut rule)                 ──┤   │
│                 render_defence_picker(&mut rule)                  ──┼─▶ each
│                 render_sockets_quality(&mut rule)                 ──┤   returns
│                 render_corrupted(&mut rule)                       ──┘   Changed
│              })                                                          │
│            [▲][▼][Delete? Yes/Cancel]  → overlay::edit::{move_*, delete}│
│          }                                                                │
│     └─ [+ Add Rule] → overlay::edit::add(&mut working_rules)             │
│  4. if any control reported Changed this frame: dirty = true            │
│  5. if dirty: overlay::store::save(&config, &store_path); dirty = false │  ← D-02/D-04
│  6. [Apply] button → dynamic::compile_store + write_filter              │  ← D-15
│     → on success: applied = true, show "reload via Options→Game→        │
│       Guidance→Item Filter→Refresh" (D-16)                              │
│     → on failure: applied unchanged, error text rendered in the stripe  │  ← D-17
│  7. any control reporting Changed also sets applied = false             │  ← D-03
└──────────────────────────────────────────────────────────────────────────┘
                         │                                    │
                         ▼                                    ▼
              overlay::store::save()                dynamic::compile_store()
              (new: Config → dynamic.json,           + write_filter()
               atomic write-then-rename,             (Phase 12, UNCHANGED —
               same dir as store)                     compiles the store the
                                                       autosave just wrote)
```

### Recommended Project Structure

```
src/overlay/
├── edit.rs              # NEW — pure mutation module (D-18). RuleId, working-model
│                         # transforms (add/delete/move/toggle-enabled), per-condition
│                         # set/clear functions, available_styles()/available_classes().
│                         # Zero egui/eframe imports. Every function unit-tested.
├── store.rs              # EXTENDED — add `save(&Config, &Path) -> Result<(), String>`
│                         # beside the existing `load`, atomic write-then-rename (D-04).
├── view_model.rs          # UNCHANGED signature-wise; may gain a thin adapter from
│                         # Vec<(RuleId, Rule)> to the existing RuleRow shape, or the
│                         # editor renders directly from the working model — planner's call.
├── widgets.rs              # EXTENDED — circular toggle painter (Don't Hand-Roll), reusing
│                         # the header_plate `✕`'s interact+paint pattern.
├── theme.rs                 # UNCHANGED — WARM_ACCENT (#A36C3C) was reserved here in Phase
│                         # 13 specifically for this phase's controls (Reset/destructive
│                         # actions — e.g. the delete confirm and, if added, a Reset button).
└── app.rs                    # EXTENDED — owns Vec<(RuleId, Rule)>, Option<RuleId> expanded,
                            # bool dirty, bool applied; the per-frame save/apply glue.
```

### Pattern 1: The `RuleId` working model (answers Q5)

**What:** Assign a session-only, monotonically-increasing `RuleId` to each rule at load time.
Keep the editable working model as `Vec<(RuleId, Rule)>` rather than `Vec<Rule>`. Never persist
`RuleId` to `dynamic.json` — on save, strip it back to a plain `Vec<Rule>` (order is the only
identity that survives to disk, exactly as today).

**Why this is the right shape:** `PITFALLS.md` §9 says any "currently editing" state indexed by
position breaks under reordering unless the move operation itself repairs it. A `RuleId` sidesteps
the repair problem entirely — it is invariant under permutation by construction. This also
composes cleanly with egui's own `CollapsingHeader::id_salt`/`::open` (both confirmed against the
installed 0.36.1 source, see Pattern 3), which want a stable identity and an externally-driven
open/closed bool, not an index.

**Code sketch (pure, no egui types):**

```rust
// src/overlay/edit.rs
use crate::models::config::Rule;

/// Session-only identity for a rule in the working model. Never serialized —
/// `dynamic.json`'s only identity is array position (unchanged from Phase 12).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct RuleId(u64);

/// The editor's in-memory working model: stable identity paired with the rule data.
/// Array order here IS precedence order, exactly as `Vec<Rule>` always was —
/// `RuleId` adds an identity layer on top, it does not change the ordering semantics.
pub struct WorkingRules {
    next_id: u64,
    rows: Vec<(RuleId, Rule)>,
}

impl WorkingRules {
    /// Builds the working model from a freshly loaded `Vec<Rule>`, assigning each
    /// rule a fresh `RuleId` in array order.
    pub fn from_rules(rules: Vec<Rule>) -> Self {
        let mut next_id = 0;
        let rows = rules
            .into_iter()
            .map(|rule| {
                let id = RuleId(next_id);
                next_id += 1;
                (id, rule)
            })
            .collect();
        Self { next_id, rows }
    }

    /// Strips identity back to the plain `Vec<Rule>` `dynamic.json` expects.
    pub fn to_rules(&self) -> Vec<Rule> {
        self.rows.iter().map(|(_, r)| r.clone()).collect()
    }

    pub fn rows(&self) -> &[(RuleId, Rule)] {
        &self.rows
    }

    /// D-06: appended at the end, per D-07's exact shape.
    pub fn add(&mut self) -> RuleId {
        let id = RuleId(self.next_id);
        self.next_id += 1;
        self.rows.push((
            id,
            Rule {
                enabled: None, // D-07: "enabled" (absence == true)
                name: None,
                style: "unchanged".to_string(),
                when: Some(crate::models::config::When::default()), // emits `"when": {}`
            },
        ));
        id
    }

    /// Returns `true` if a rule with `id` was found and removed.
    pub fn delete(&mut self, id: RuleId) -> bool {
        let before = self.rows.len();
        self.rows.retain(|(rid, _)| *rid != id);
        self.rows.len() != before
    }

    /// Swaps `id` one position earlier. No-op (returns `false`) if already first
    /// or not found — callers use the return value to decide whether to disable
    /// the "move up" button, not to guess from index 0.
    pub fn move_up(&mut self, id: RuleId) -> bool {
        let Some(pos) = self.rows.iter().position(|(rid, _)| *rid == id) else {
            return false;
        };
        if pos == 0 {
            return false;
        }
        self.rows.swap(pos, pos - 1);
        true
    }

    pub fn move_down(&mut self, id: RuleId) -> bool {
        let Some(pos) = self.rows.iter().position(|(rid, _)| *rid == id) else {
            return false;
        };
        if pos + 1 >= self.rows.len() {
            return false;
        }
        self.rows.swap(pos, pos + 1);
        true
    }

    /// EDIT-02: toggles `Rule.enabled`. `None`/`Some(true)` -> `Some(false)` -> `None`
    /// (round-trips to the "absent = enabled" convention rather than ever writing
    /// `"enabled": true` explicitly — keeps authored files minimal, matches how
    /// `dynamic_base.json`'s own rules are written).
    pub fn toggle_enabled(&mut self, id: RuleId) {
        if let Some((_, rule)) = self.rows.iter_mut().find(|(rid, _)| *rid == id) {
            rule.enabled = match rule.enabled {
                Some(false) => None,
                _ => Some(false),
            };
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rule(style: &str) -> Rule {
        Rule { enabled: None, name: None, style: style.to_string(), when: None }
    }

    #[test]
    fn selection_survives_a_move_because_it_is_keyed_by_id_not_index() {
        let mut wr = WorkingRules::from_rules(vec![rule("A"), rule("B"), rule("C")]);
        let b_id = wr.rows()[1].0;

        assert!(wr.move_up(b_id));
        // B is now at index 0 -- the CALLER's "currently expanded" state, if it
        // stored `b_id` (not an index), still points at the same rule.
        assert_eq!(wr.rows()[0].0, b_id);
        assert_eq!(wr.rows()[0].1.style, "B");
    }

    #[test]
    fn move_up_at_the_top_is_a_no_op_reported_as_false() {
        let mut wr = WorkingRules::from_rules(vec![rule("A"), rule("B")]);
        let a_id = wr.rows()[0].0;
        assert!(!wr.move_up(a_id));
        assert_eq!(wr.rows()[0].1.style, "A");
    }

    #[test]
    fn add_appends_the_unchanged_noop_rule_at_the_end() {
        let mut wr = WorkingRules::from_rules(vec![rule("A")]);
        let new_id = wr.add();
        assert_eq!(wr.rows().len(), 2);
        let (id, added) = &wr.rows()[1];
        assert_eq!(*id, new_id);
        assert_eq!(added.style, "unchanged");
        assert_eq!(added.enabled, None, "D-07: new rules are enabled");
        assert_eq!(added.when, Some(crate::models::config::When::default()));
    }

    #[test]
    fn delete_removes_only_the_targeted_rule_others_keep_their_id() {
        let mut wr = WorkingRules::from_rules(vec![rule("A"), rule("B"), rule("C")]);
        let b_id = wr.rows()[1].0;
        let c_id = wr.rows()[2].0;
        assert!(wr.delete(b_id));
        assert_eq!(wr.rows().len(), 2);
        assert_eq!(wr.rows()[1].0, c_id, "C keeps its own id after B is removed");
    }

    #[test]
    fn to_rules_strips_identity_and_preserves_order() {
        let wr = WorkingRules::from_rules(vec![rule("A"), rule("B")]);
        let plain = wr.to_rules();
        assert_eq!(plain, vec![rule("A"), rule("B")]);
    }
}
```

### Pattern 2: The uniform "enable toggle + value widget" architecture (answers Q1, Q7/EDIT-08)

**What:** Every condition in `When` — including `corrupted`, which the CONTEXT open questions
worried needed a bespoke tri-state control — is rendered as *(a)* a toggle that decides `Some`
vs. `None` for that field, plus *(b)* (only shown/enabled when the toggle is on) the widget that
sets the `Some` value. This is not a new decision; it is D-14 taken literally and applied uniformly
to every field including the boolean one, rather than treating `corrupted` as a special case.

**Why this resolves the "tri-state" question without new widget work:** a true tri-state widget
(unset / true / false as one cycling control) is one bespoke interaction pattern used nowhere else
in the editor. Reusing the enable-toggle pattern means `corrupted`'s "unset" state is exactly the
same mechanism as every other condition's "off" state (an `Option::None`), and its "value" state is
an ordinary two-way choice (`selectable_value` or a 2-item `ComboBox`) — no new widget-interaction
code, and one fewer concept for a future maintainer to learn.

**Code sketch:**

```rust
// src/overlay/app.rs (presentation only — calls into overlay::edit for the actual mutation)
fn render_corrupted(ui: &mut egui::Ui, rule: &mut Rule) -> bool {
    let mut changed = false;
    let mut enabled = rule.when.as_ref().and_then(|w| w.corrupted).is_some();

    ui.horizontal(|ui| {
        // The circular enable toggle (Pattern 4) — same control every other
        // condition uses to flip Some/None.
        if super::widgets::circular_toggle(ui, &mut enabled).changed() {
            changed = true;
            let when = rule.when.get_or_insert_with(Default::default);
            when.corrupted = if enabled { Some(false) } else { None };
        }
        ui.label("Corrupted");

        if enabled {
            if let Some(when) = rule.when.as_mut() {
                let mut value = when.corrupted.unwrap_or(false);
                ui.add_enabled_ui(true, |ui| {
                    if ui.selectable_value(&mut value, false, "False").clicked()
                        || ui.selectable_value(&mut value, true, "True").clicked()
                    {
                        when.corrupted = Some(value);
                        changed = true;
                    }
                });
            }
        }
    });
    changed
}
```

`ui.selectable_value` is confirmed at `egui-0.36.1/src/ui.rs:1938` (`pub fn selectable_value<'a,
Value: PartialEq>(...)`). `ui.add_enabled_ui` is confirmed at `ui.rs:1619` and is the same
mechanism the defence-picker group toggle (D-12) should use to gray out its three checkboxes when
the group is off.

### Pattern 3: Accordion via `CollapsingHeader` keyed by `RuleId`, scrolled into view on open (answers Q1's class picker, Q2)

**Confirmed against `egui-0.36.1/src/containers/collapsing_header.rs`:**
- `CollapsingHeader::new(text)` — `text` doubles as the `Id` source unless overridden.
- `.id_salt(impl AsIdSalt)` (line 431) — **use this**, keyed by `RuleId`, not by title text or
  array index (title text isn't unique — several rules can share a style name; index isn't stable
  under reordering, per Pattern 1).
- `.open(Option<bool>)` (line 419) — `Some(true)`/`Some(false)` forces the state every frame;
  `None` lets egui's own per-id memory drive it. **Use `Some(expanded_id == Some(this_id))`** so
  the app's own `Option<RuleId>` is the single source of truth, not a second, egui-internal copy
  that could drift from it.
- `.show(ui, |ui| { ... })` returns a `CollapsingResponse<R>` whose `.header_response` is an
  ordinary `egui::Response` — this is what scrolling into view (Q2) hooks into.

```rust
// src/overlay/app.rs
let is_open = self.expanded == Some(id);
let header = egui::CollapsingHeader::new(&rule.style)
    .id_salt(id)                // Pattern 1's RuleId, not title text or index
    .open(Some(is_open))
    .show(ui, |ui| {
        render_conditions(ui, rule); // Pattern 2's uniform widgets, one call per condition
    });

if header.header_response.clicked() {
    self.expanded = if is_open { None } else { Some(id) };
}

// Q2: scroll the just-opened row into view. `just_opened` compares this frame's
// decision to last frame's, so this fires exactly once per open, not every frame
// while already open (which would fight the user's own manual scrolling).
if !is_open && self.expanded == Some(id) {
    header.header_response.scroll_to_me(Some(egui::Align::TOP));
}
```

`Response::scroll_to_me` is confirmed at `egui-0.36.1/src/response.rs:841` — it works by writing a
scroll target into pass-state that the enclosing `ScrollArea` reads on its next layout pass, so it
requires no direct reference to the `ScrollArea` itself; calling it on the header response from
inside the `ScrollArea::show` closure is the documented, version-matched usage
(`egui-0.36.1/src/ui.rs:1436`'s own doc example uses the same call shape on a `Response`).
**Nested `CollapsingHeader` inside `ScrollArea` is standard, unremarkable egui usage** — no
version-specific interaction issue was found in the source for this combination; this is
MEDIUM-HIGH confidence (API-verified, but the *feel* of scrolling only fully confirms by running
it, same caveat Phase 13's research gave its own theming-reach claims).

### Pattern 4: Circular toggle — paint on top of existing interaction primitives, don't reimplement hit-testing (answers Q1's "how much custom Painter work")

**What:** Reuses the exact `ui.allocate_exact_size` → `ui.interact` → `ui.painter()` sequence
`widgets::header_plate`'s `✕` already established in this codebase for a hand-painted control —
the only change is drawing a filled/outlined circle instead of crossed line segments, and driving
a `&mut bool` instead of reporting a one-shot click.

```rust
// src/overlay/widgets.rs
/// A PoE2-idiom circular toggle. Visually distinct from egui's default square
/// `Checkbox`, but built on the same `Sense::click()` + `Response::clicked()`
/// primitives — no custom hit-testing, no reimplemented widget-state machine.
pub fn circular_toggle(ui: &mut egui::Ui, checked: &mut bool) -> egui::Response {
    let size = 16.0;
    let (rect, mut response) =
        ui.allocate_exact_size(egui::vec2(size, size), egui::Sense::click());

    if response.clicked() {
        *checked = !*checked;
        response.mark_changed();
    }

    let painter = ui.painter();
    let center = rect.center();
    let radius = size / 2.0 - 1.0;
    let stroke = egui::Stroke::new(1.5, theme::CONTROL_GLYPH);
    painter.circle_stroke(center, radius, stroke);
    if *checked {
        painter.circle_filled(center, radius - 3.0, theme::CONTROL_GLYPH);
    }

    response
}
```

`Response::mark_changed()` plus checking `.changed()` at the call site is the standard egui pattern
for a widget function that mutates through a `&mut` reference rather than returning the new value
— the same shape `Checkbox`'s own `Widget for Checkbox` impl uses internally
(`egui-0.36.1/src/widgets/checkbox.rs`, confirmed above). This keeps `circular_toggle` a drop-in
substitute anywhere the editor would otherwise reach for `ui.checkbox(&mut bool, "")`.

### Pattern 5: Autosave as a per-frame coalesced write, not per-keystroke or timer-debounced (answers Q3)

**What:** Every mutation-producing widget call in the frame sets a `dirty: bool` on `OverlayApp`
(via each render function's `bool` return, as in Patterns 2-3's sketches). After all widgets for
the frame have been processed, **once**: `if dirty { store::save(...); dirty = false; }`.

**Why this needs no timer or background thread:**
1. **It already coalesces.** Multiple edits landing in the same frame (rare, but e.g. a class
   picker's "select all in group" button) produce one write, not N.
2. **It already guarantees flush-before-Quit.** `OverlayCommand::Quit` is handled in the same
   `ui()` call, and (per D-14/`app.rs`'s existing structure) the save step runs earlier in that
   same function body than the `ViewportCommand::Close` send — Win32's message loop cannot begin
   tearing down the process until the current `ui()` call returns, so the write has already
   completed synchronously on the same thread before `Close` is even dispatched. No async runtime,
   no `Drop`-based flush-on-exit hook, no race is possible here as long as save happens
   before the Quit branch's `send_viewport_cmd(Close)` in *program order* within the same frame —
   which the existing `app.rs` structure (poll events → render body → act on tray menu event)
   already naturally provides if `save` is called from inside the body, not the tray-poll branch.
3. **The file is small.** `dynamic_base.json` today is ~5KB (137 lines); a `dynamic.json` with
   twice as many rules is still well under 20KB. An atomic write-then-rename of that size on local
   NTFS is sub-millisecond — even a worst-case "one write per drag-frame while dragging a
   `DragValue`" (Pattern 1's numeric widgets can fire `.changed()` on every frame while dragging,
   confirmed by reading `drag_value.rs`'s `response.dragged()` branch) is not a performance
   concern at this file size, so no debounce window is needed for correctness *or* performance.

**Code sketch:**

```rust
// src/overlay/app.rs — inside `ui()`, after all row rendering for the frame
if self.dirty {
    if let Err(message) = store::save(&self.config, &self.store_path) {
        self.last_save_error = Some(message); // rendered in the stripe, D-17's sibling concern
    }
    self.dirty = false;
    self.applied = false; // D-03: any successful or attempted edit invalidates "applied"
}
```

```rust
// src/overlay/store.rs — new function beside the existing `load`
/// Atomically overwrites `store` with `config`'s current contents (D-02, D-04).
/// Mirrors `dynamic::write_filter`'s temp-file-in-destination-directory discipline exactly —
/// `store`'s parent directory is the repo root, not a cross-volume game folder, but the
/// same reasoning applies: never let the game (or a human `cat`ing the file mid-edit)
/// observe a truncated `dynamic.json`.
pub fn save(config: &Config, store: &Path) -> Result<(), String> {
    let text = serde_json::to_string_pretty(config)
        .map_err(|e| format!("failed to serialize the store: {e}"))?;
    let dir = store.parent().ok_or_else(|| "store path has no parent directory".to_string())?;
    let mut temp = tempfile::NamedTempFile::new_in(dir)
        .map_err(|e| format!("cannot create a temp file in {}: {e}", dir.display()))?;
    std::io::Write::write_all(&mut temp, text.as_bytes())
        .map_err(|e| format!("cannot write the store: {e}"))?;
    temp.persist(store)
        .map_err(|e| format!("cannot save {}: {}", store.display(), e.error))?;
    Ok(())
}
```

### Anti-Patterns to Avoid

- **Indexing the "currently expanded" state by array position.** This is the exact hazard
  `PITFALLS.md` §9 names; Pattern 1's `RuleId` exists specifically to make it structurally
  impossible rather than "must remember to fix up on every move."
- **A bespoke tri-state widget for `corrupted`.** Breaks the uniform enable-toggle vocabulary for
  no benefit — see Pattern 2.
- **Debouncing autosave with a timer/background thread.** Unnecessary complexity for a file this
  small (Pattern 5) and introduces exactly the kind of "does it flush before Quit" race condition
  the CONTEXT question worried about, which per-frame synchronous coalescing avoids by construction.
  Do not reach for `tokio::time::sleep`/a debounce crate here.
- **Writing `dynamic.json` with a bare `File::create` + `write_all`.** D-04 explicitly forbids
  this; use the same `NamedTempFile::new_in` + `.persist()` shape `dynamic::write_filter` already
  established, not a new one.
- **Assuming `NumericCondition`/`AreaLevelRange`'s lack of `skip_serializing_if` is a bug to fix.**
  It isn't in scope this phase (touching `src/models/config.rs`'s serde attributes is a
  cross-cutting change affecting `config.json` fixtures too) — it only means saved files show
  explicit `"eq": null` keys after an edit, which is cosmetically different but not semantically
  lossy. Do not "fix" this as a drive-by; flag it in the plan's notes if it bothers a reviewer, but
  it does not block Phase 14.
- **Assuming Cinzel (the all-caps display face) has arrow glyphs (▲/▼) for the reorder buttons.**
  Phase 13 already hit exactly this failure mode once, with the header's `✕` rendering as a tofu
  box in both egui's default font and Cinzel, and fixed it by hand-painting two line segments
  instead of relying on a Unicode glyph. The up/down reorder buttons (D-05) are new text content
  Phase 13 never rendered — **verify the glyphs render before committing to text buttons**; if they
  tofu, reuse the same hand-painted-triangle fix shape `widgets::header_plate` already uses for its
  close control, rather than rediscovering the same lesson.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|--------------|-----|
| Grouped, collapsible multi-select checkbox list (21 classes / 3 groups) | A custom expand/collapse container with hand-rolled animation and click-outside handling | `egui::CollapsingHeader` per group, `ui.checkbox` per class inside | egui already ships exactly this container; verified present and API-stable in the installed 0.36.1 source |
| Style/rarity/corrupted dropdowns | A hand-rolled popup + hit-testing for "click outside closes it" | `egui::ComboBox` (style), `ui.selectable_value` (rarity op, corrupted value) | Both are core, well-exercised egui widgets; `ComboBox::show_ui`'s popup-close behavior is already handled internally |
| Numeric entry with click-to-type | A custom `TextEdit` + parse-on-blur implementation | `egui::DragValue` | Source-confirmed (`drag_value.rs`) to already implement exactly "single click enters edit mode, typed digits parse directly" — this is the built-in behavior, not something to reimplement |
| A tri-state (unset/true/false) toggle for `corrupted` | A bespoke three-way-cycling custom widget | The uniform enable-toggle + `selectable_value` pattern (Pattern 2) already used for every other condition | One fewer interaction pattern to build, test, and explain; see Anti-Patterns |
| Circular toggle hit-testing | A custom `Widget` impl reimplementing click detection, hover state, and keyboard-focus semantics from scratch | `ui.interact(rect, id, Sense::click())` + hand-painted circle (Pattern 4), the same primitive `widgets::header_plate`'s `✕` already uses | egui's `interact`/`Response` machinery already gives correct click/hover/focus semantics "for free"; only the paint call needs to differ from a square checkbox |
| Atomic JSON store writes | `std::fs::write` direct-to-target, or a bespoke lockfile scheme | `tempfile::NamedTempFile::new_in` + `.persist()`, exactly mirroring `dynamic::write_filter` | Already proven correct (and unit-tested) for this exact "never let the reader see a partial write" requirement one file over in `src/dynamic.rs` |

**Key insight:** this phase's egui-specific "don't hand-roll" list is short because most of what
looked novel in the open questions (grouped checkboxes, numeric entry, tri-state) turns out to
already have a first-party egui widget or an in-codebase precedent (the `✕` paint pattern) that
covers it — the only genuinely new painting code is the circle shape itself.

## Common Pitfalls

### Pitfall 1: Trusting a widget-local `bool`/index as "the" selection state instead of a `RuleId`

**What goes wrong:** After moving a rule with the up/down arrows, the accordion that was open
snaps to show a *different* rule's controls (or an empty/wrong panel), and the player edits the
wrong rule without realizing it.
**Why it happens:** If "currently expanded" is stored as an index (`expanded: Option<usize>`)
rather than a stable id, a move operation that swaps positions leaves the stale index pointing at
whatever rule now occupies that slot.
**How to avoid:** Pattern 1's `RuleId`, threaded through `CollapsingHeader::id_salt` (Pattern 3).
**Warning signs:** A manual test that only opens one rule and never reorders will not catch this —
the test must expand a rule, then move a *different* rule past it, and confirm the same rule is
still shown expanded.

### Pitfall 2: `NumericCondition`/`AreaLevelRange` round-tripping to explicit `null` keys after any edit

**What goes wrong:** A hand-authored `dynamic.json` with `{"sockets": {"min": 3}}` becomes
`{"sockets": {"eq": null, "min": 3, "max": null}}` after the very first autosave, even if the
editor never touched the sockets field on that particular rule (because autosave serializes the
whole `Config`, not just the changed rule).
**Why it happens:** `NumericCondition` and `AreaLevelRange` derive `Serialize` without
`#[serde(skip_serializing_if = "Option::is_none")]` on their fields (confirmed by reading
`src/models/config.rs` — contrast with `When`'s fields, which all have it).
**How to avoid:** Nothing to *fix* this phase — it is not lossy (the value re-parses to the same
`NumericCondition`, confirmed by the existing `config_with_rules_round_trips`-style equality
tests), and `dynamic.json` is git-ignored per-user state nothing diffs byte-for-byte. Document it
so a future reviewer doesn't mistake the `null` keys for a bug. If it matters later (e.g. a
hand-editing workflow that expects terse JSON), that's a `src/models/config.rs`-level change
affecting `config.json` fixtures too — out of scope for this phase.
**Warning signs:** A round-trip test asserting byte-identical JSON text (rather than `Config`
value equality) would fail here and should not be written that way — assert on parsed values, not
serialized bytes.

### Pitfall 3: `classGroups`/`build.armourTypes` key order shuffling on every autosave

**What goes wrong:** A player who opens `dynamic.json` in a text editor between sessions sees the
`classGroups` (and `build.armourTypes`) key order change even though they made no edits to either
section — purely because *some other* edit anywhere in the file triggered a full-`Config`
re-serialize.
**Why it happens:** Both fields are `HashMap<String, Vec<String>>`. Rust's std `HashMap` is
explicitly documented to iterate "in arbitrary order," reseeded per-instance from a random source
`[CITED: doc.rust-lang.org std::collections::HashMap]` — every fresh `HashMap` built during JSON
deserialization gets a new random iteration order, and `serde_json`'s map serialization just walks
the `HashMap` in whatever order that turns out to be.
**How to avoid:** Nothing actionable for correctness (semantic equality holds regardless of key
order, and `Config`'s hand-derived `PartialEq` — via `#[derive(PartialEq)]` on a `HashMap` field —
already compares by key/value, not by iteration order). Worth stating explicitly so a "the file
changed but I didn't touch classGroups" report isn't mistaken for a real bug during Phase 14
verification.
**Warning signs:** A byte-diff of `dynamic.json` across two autosaves showing `classGroups`/
`armourTypes` reordered with otherwise-identical content — expected, not a defect.

### Pitfall 4: Assuming Cinzel has the reorder-arrow glyphs (repeat of Phase 13's `✕` lesson)

**What goes wrong:** Up/down reorder buttons rendered as `ui.button("▲")`/`ui.button("▼")` show
tofu boxes instead of arrows, exactly as U+2715 (`✕`) did for the header close control in Phase 13
(13-03 → fixed in 13-04 via hand-painted strokes, per `13-VERIFICATION.md`'s "What Went Well" and
`widgets.rs`'s own doc comment).
**Why it happens:** Cinzel is a narrow, all-caps Latin display face; it very likely does not
contain Miscellaneous Symbols / Arrows block glyphs (U+25B2/U+25BC), and egui's default fallback
font may or may not either, depending on which family the button text actually resolves through.
**How to avoid:** Verify the glyphs render (a five-minute manual check) before committing to
text-glyph buttons; if they tofu, apply the exact same fix Phase 13 already validated —
`ui.painter().line_segment`/a small filled triangle via `Painter`, keyed off the same
`ui.interact(rect, id, Sense::click())` shape as Pattern 4's circular toggle.
**Warning signs:** None at compile time — purely visual, so this needs a look-at-it step in the
plan's verification, not just "it compiled" (same caveat 13-RESEARCH.md's Pitfall 6 gave for
small-caps rendering).

### Pitfall 5: Forgetting that `Rule.enabled: Some(true)` and `None` are both "enabled" but not equal

**What goes wrong:** A naive "toggle enabled" implementation that does `rule.enabled =
Some(!rule.enabled.unwrap_or(true))` will, on a freshly-added rule (`enabled: None`), write
`Some(true)` explicitly on the very first disable→re-enable cycle — not wrong per se (both mean
"enabled"), but it silently grows the JSON with a redundant key the model was specifically designed
to avoid (`rule_without_enabled_serializes_without_the_key`'s own test name states the intent).
**Why it happens:** `Option<bool>` has three states but "enabled" only cares about two of them;
it's easy to write a toggle that treats `None` and `Some(true)` as different rather than
equivalent.
**How to avoid:** Pattern 1's `toggle_enabled` collapses both `None` and `Some(true)` to the same
outgoing state (`None`) when re-enabling, matching the existing test's intent even though no test
currently asserts this specific toggle behavior (it would be a good addition to the module's own
test suite, not just this research doc).
**Warning signs:** A diff of `dynamic.json` showing `"enabled": true` appear on a rule that was
never explicitly authored with that key — a sign the toggle function doesn't collapse `None`.

## Code Examples

### Reading the style list from the loaded `Config` (EDIT-03, `PITFALLS.md` §10)

```rust
// src/overlay/edit.rs — pure, no egui types
use crate::models::config::Styles;

/// The style names selectable from the picker, in the fixed display order the
/// game's own tier system uses (S..F, then the named styles). Reads directly
/// from the loaded `Styles` struct — `exceptional`/`craftable` only appear if
/// the config actually defines them, so this can never offer a style name that
/// would fail `emitter::resolve_style`'s generate-time check.
pub fn available_styles(styles: &Styles) -> Vec<&'static str> {
    let mut names = vec![
        "S", "A", "B", "C", "D", "E", "F", "Recycle", "unchanged", "hidden", "unidentified",
    ];
    if styles.exceptional.is_some() {
        names.push("exceptional");
    }
    if styles.craftable.is_some() {
        names.push("craftable");
    }
    names
}

#[cfg(test)]
mod style_list_tests {
    use super::*;
    use crate::models::config::Config;

    #[test]
    fn dynamic_base_json_offers_exactly_twelve_styles() {
        let config: Config =
            serde_json::from_str(include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/dynamic_base.json")))
                .unwrap();
        let names = available_styles(&config.styles);
        assert_eq!(names.len(), 12, "dynamic_base.json defines exceptional but not craftable");
        assert!(names.contains(&"exceptional"));
        assert!(!names.contains(&"craftable"));
    }
}
```

### Setting the defence picker's three checkboxes (EDIT-06)

```rust
// src/overlay/edit.rs — pure
use crate::models::config::{NumericCondition, When};

/// One of the three defence attributes a rule's `when` can condition on.
pub enum Defence { Armour, Evasion, EnergyShield }

/// EDIT-06: `ticked` -> `min: Some(1)` (emits `> 0`), `!ticked` -> `eq: Some(0)`
/// (emits the literal `0`). Mirrors `emitter::emit_numeric`'s own semantics
/// exactly (confirmed in `src/emitter.rs`), so the picker cannot produce a
/// combination the emitter interprets differently than the picker intended.
pub fn set_defence(when: &mut When, defence: Defence, ticked: bool) {
    let nc = Some(if ticked {
        NumericCondition { eq: None, min: Some(1), max: None }
    } else {
        NumericCondition { eq: Some(0), min: None, max: None }
    });
    match defence {
        Defence::Armour => when.base_armour = nc,
        Defence::Evasion => when.base_evasion = nc,
        Defence::EnergyShield => when.base_energy_shield = nc,
    }
}

/// The defence *group* toggle (D-12): turning the group off clears all three
/// fields to `None`, emitting no defence line at all.
pub fn clear_defence_group(when: &mut When) {
    when.base_armour = None;
    when.base_evasion = None;
    when.base_energy_shield = None;
}

#[cfg(test)]
mod defence_tests {
    use super::*;

    #[test]
    fn armour_and_energy_shield_ticked_evasion_unticked_matches_ee06_exactly() {
        let mut when = When::default();
        set_defence(&mut when, Defence::Armour, true);
        set_defence(&mut when, Defence::EnergyShield, true);
        set_defence(&mut when, Defence::Evasion, false);

        assert_eq!(when.base_armour.as_ref().unwrap().min, Some(1));
        assert_eq!(when.base_energy_shield.as_ref().unwrap().min, Some(1));
        assert_eq!(when.base_evasion.as_ref().unwrap().eq, Some(0));
    }

    #[test]
    fn clearing_the_group_emits_no_defence_lines_at_all() {
        let mut when = When::default();
        set_defence(&mut when, Defence::Armour, true);
        clear_defence_group(&mut when);
        assert!(when.base_armour.is_none());
        assert!(when.base_evasion.is_none());
        assert!(when.base_energy_shield.is_none());
    }
}
```

### A concrete round-trip guard for `dynamic_base.json` (answers Q4's "recommend a concrete guard")

```rust
// src/dynamic.rs or src/overlay/store.rs — extends the existing round-trip test family
// already proven for config.json (config_with_rules_round_trips, rule_enabled_round_trips)
#[test]
fn dynamic_base_json_round_trips_through_config_losslessly() {
    let json = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/dynamic_base.json"));
    let first: Config = serde_json::from_str(json).expect("dynamic_base.json must parse");
    let serialized = serde_json::to_string_pretty(&first).expect("serialize");
    let second: Config =
        serde_json::from_str(&serialized).expect("re-deserialize after round-trip");
    // Value equality, NOT byte equality -- Pitfall 3 (HashMap key order) means the raw
    // JSON text is allowed to differ even when the parsed value is identical.
    assert_eq!(first, second, "dynamic_base.json did not round-trip losslessly through Config");
}
```

This is a direct extension of the existing `config_with_rules_round_trips` test's own pattern
(`src/models/config.rs`), applied to `dynamic_base.json` specifically because that is the file
`overlay::store::save` will actually be writing back — closing the one gap in the existing test
suite (which proves `config.json` round-trips, but never exercised `dynamic_base.json` through the
same assertion until now).

## State of the Art

| Old Approach (Phase 13) | Current Approach (Phase 14) | When Changed | Impact |
|--------------------------|------------------------------|---------------|--------|
| `store.rs` only reads (`load`), never writes | `store.rs` gains `save`, the write half of the same lifecycle | This phase | D-20's Phase 13 boundary ("all mutation stays in Phase 14") is exactly what's being lifted |
| `rows: Result<Vec<RuleRow>, String>` — a one-shot, read-only snapshot taken once at startup | A live, mutable `WorkingRules` (Pattern 1), re-derived-for-display every frame, saved on every change | This phase | The read-only view model isn't discarded — `view_model::rule_rows`-style summarization logic is still useful for a collapsed row's one-line summary; it's just no longer the *only* view of the data |
| No `RuleId`-equivalent concept existed (nothing needed one — nothing mutated) | `RuleId` introduced as a new, session-only identity layer | This phase | New concept, but small and fully isolated to `overlay::edit` — does not touch `Rule`/`When`/`Config` in `src/models/config.rs` at all |

**Deprecated/outdated:** Nothing from 13-RESEARCH.md's or PITFALLS.md's findings is stale — this
phase builds directly on top of the same `eframe`/`egui` 0.36.1 pin, the same visibility mechanism,
and the same theme constants, none of which this research found any reason to revisit.

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|----------------|
| A1 | Nested `CollapsingHeader` inside `ScrollArea`, combined with `.open(Some(bool))` driven externally every frame, behaves exactly like the simpler internally-memoized case with no extra egui-version-specific interaction quirk | Pattern 3 | Low — both APIs are individually confirmed against the installed source; the *combination* was not literally executed in this research session (no window was opened), so a genuinely surprising interaction (e.g. an animation glitch) can only be ruled out by the manual verification step the roadmap already expects for this UI-heavy phase |
| A2 | `Response::scroll_to_me` called on a `CollapsingHeader`'s `header_response` correctly scrolls the *whole expanding row* (including its now-visible body) into view, not just the header line itself | Pattern 3 (Q2) | Low-Medium — the API scrolls the given `Rect` into view; if only the header rect (not the body that appears below it) is what's passed, a tall expanded row's bottom controls could still land out of view. Worth a specific manual-verification step: expand a rule near the bottom of a ten-rule list and confirm its controls are fully visible, not just its header |
| A3 | egui's default fallback font (not just Cinzel) also lacks arrow glyphs (▲/▼), making the hand-painted-triangle fallback necessary rather than optional | Pitfall 4 | Low — even if the glyphs do render, the hand-painted approach is strictly safe as a first implementation given Phase 13's own precedent; this assumption only affects whether a simpler `ui.button("▲")` can be tried first before falling back |
| A4 | No `slopcheck` verification ran against `egui_kittest` (tool unavailable in this environment, same as Phase 13) | Package Legitimacy Audit | Low — `egui_kittest` is optional this phase (not a required dependency), published by the same org/repo as the already-trusted `egui`/`eframe`, and gated behind a `checkpoint:human-verify` per protocol if the planner adopts it |

## Open Questions

1. **Does the visual result of `.open(Some(bool))` driven every frame from `OverlayApp`'s own state
   ever visibly "fight" egui's internal collapse animation** (e.g. a one-frame flicker when the
   externally-driven `open` value and the animation's internal openness float disagree)?
   - What we know: `.open()`'s doc comment (`collapsing_header.rs`) states it "will make the
     collapsing header open/close this frame (or stay open/closed)" — this reads as authoritative
     and immediate, not merely a hint to the animation.
   - What's unclear: Whether there's a one-frame visual snap vs. a smooth animated transition when
     `open` flips from `Some(false)` to `Some(true)` under external control, vs. the header's own
     internal click-driven toggle (which the demo/tests exercise more commonly).
   - Recommendation: Not a blocker — worst case is a slightly less smooth expand animation than
     egui's own click-driven default, which is a polish concern, not a correctness one. Confirm
     during the same manual verification pass the roadmap already schedules for this phase.

2. **Exact visual weight/size for the circular toggle** (Pattern 4's `size = 16.0`, `radius - 3.0`
   for the filled dot) relative to the D-24 reference screenshot's own control sizing.
   - What we know: D-24 says "circular, not square" and reserves specific colors
     (`CONTROL_GLYPH #C4C4A8` for the checked glyph).
   - What's unclear: The exact pixel proportions PoE2's own toggle uses — not measured in this
     research session (would require re-examining `reference/poe2-options-palette.png` at pixel
     level, which is a design-fidelity question, not a mechanism question this research is scoped
     to answer).
   - Recommendation: Treat the `16.0`/`radius - 3.0` figures in Pattern 4 as a reasonable starting
     point, adjustable by eye during implementation — this is explicitly a "look at it" concern
     per `PITFALLS.md` §8's own framing of visual fidelity work.

## Environment Availability

No new external dependencies, tools, or services beyond what Phases 11-13 already established and
verified available (Rust toolchain, Windows 11, the pinned crate versions already in `Cargo.toml`).
`egui_kittest` (optional, discretionary) was confirmed to exist on crates.io at the matching
`0.36.1` version via `cargo search` in this session; it was not installed or exercised (adopting it
is the planner's call, per Q6 below).

## Validation Architecture

`workflow.nyquist_validation` is `false` in `.planning/config.json` — this section is skipped per
the research protocol.

## Security Domain

`security_enforcement` is absent from `.planning/config.json`, so treated as enabled — but, as in
13-RESEARCH.md, this phase has essentially no traditional attack surface: no network listeners, no
authentication, and the only "untrusted" input is a local, single-user file the player edits
through their own overlay.

### Applicable ASVS Categories

| ASVS Category | Applies | Standard Control |
|----------------|---------|-------------------|
| V2 Authentication | No | No accounts, no auth surface |
| V3 Session Management | No | No sessions |
| V4 Access Control | No | Single-user local process |
| V5 Input Validation | Marginal — the editor now *writes* `dynamic.json`, not just reads it | Every write goes through the same typed `Config`/`Rule`/`When` model `serde` already validates on the read side (`deny_unknown_fields` everywhere) — the editor cannot produce a malformed field because it only ever sets already-typed struct fields, never raw strings into the JSON |
| V6 Cryptography | No | No secrets, no network calls this phase |

### Known Threat Patterns for this stack

| Pattern | STRIDE | Standard Mitigation |
|---------|--------|-----------------------|
| A crash mid-autosave leaving `dynamic.json` truncated, which then fails to load on next launch (a local DoS against the player's own tool) | Denial of Service (local) | D-04's atomic write-then-rename (Pattern 5) — the temp file, not the target, absorbs a crash mid-write; `dynamic.json` itself is only ever replaced by a single atomic rename once the full write succeeds |
| A crash mid-Apply leaving `dynamic.filter` truncated, which the game then fails to parse on Refresh | Denial of Service (local, in-game) | Already mitigated — `dynamic::write_filter` (Phase 12, unchanged) uses the identical atomic pattern; this phase's Apply button is a caller of that existing, already-tested function, not new write logic |
| A player hand-edits `dynamic.json` to something that parses as valid JSON but violates a closed enum (e.g. `"style": "Legendary"`) between overlay sessions | Tampering (self-inflicted, not adversarial) | `deny_unknown_fields` + closed enums already reject this at `serde` deserialize time (`store::load`'s existing error path, confirmed by `load_on_a_malformed_store_names_the_offending_field_and_does_not_panic`) — Phase 14 doesn't change this, it just means the editor is now also a *second* writer of the same file the hand-editor could touch, so the existing malformed-store error path becomes reachable slightly more often in practice (not a new vulnerability, an existing one exercised more) |

## Sources

### Primary (HIGH confidence — read directly against the pinned version / this repo's own code)
- `egui-0.36.1` installed source (`~/.cargo/registry/src/index.crates.io-*/egui-0.36.1/`):
  `src/containers/collapsing_header.rs` (`.new`, `.id_salt`, `.open`, `.show`), `src/containers/
  combo_box.rs` (`.new`, `.show_ui`), `src/containers/scroll_area.rs`, `src/widgets/drag_value.rs`
  (click-to-edit-mode behavior), `src/widgets/checkbox.rs` (`.indeterminate`, `Widget` impl
  pattern), `src/ui.rs` (`selectable_value`, `add_enabled_ui`, `scroll_to_rect`), `src/response.rs`
  (`scroll_to_me`)
- `src/models/config.rs` (this repo) — `Config`/`Rule`/`When`/`NumericCondition`/`AreaLevelRange`/
  `RarityComparator` field definitions and `serde` attributes, plus the existing
  `config_with_rules_round_trips`, `rule_enabled_round_trips`, `when_corrupted_round_trips` tests
- `src/emitter.rs` (this repo) — `emit_numeric`, `resolve_style`'s closed style-name match
- `src/dynamic.rs`, `src/overlay/{app,store,view_model,widgets,theme,mod}.rs` (this repo) — the
  existing lifecycle, atomic-write pattern, and layering this phase extends
- `doc.rust-lang.org/std/collections/struct.HashMap.html` — iteration-order guarantee (or lack
  thereof), fetched and quoted this session
- `.planning/phases/13-overlay-shell/13-RESEARCH.md` and `13-VERIFICATION.md` (this repo) — the
  `MARKER_ACTIVATE`/tofu-glyph/theming-reach precedents this research builds on

### Secondary (MEDIUM confidence)
- `cargo search egui_kittest` (live crates.io index, this session) — version confirmation only
- WebSearch summary of `egui_kittest`'s `Harness`/snapshot-testing feature set (not independently
  verified against its own source in this session, since it is not adopted as a hard requirement)

### Tertiary (LOW confidence)
- None retained as fact — everything not independently verified this session is called out in the
  Assumptions Log instead.

## Metadata

**Confidence breakdown:**
- Round-tripping (`Config`/`Rule`/`When` fidelity, Q4): HIGH — backed by existing, passing tests in
  this exact codebase, not just documentation reading
- Widget vocabulary (Q1): HIGH — every cited API confirmed against the installed 0.36.1 source
- Ordinal identity / selection under mutation (Q5): HIGH — `RuleId` pattern is a straightforward
  application of confirmed `CollapsingHeader` APIs plus ordinary Rust `Vec` semantics, and is fully
  unit-testable (demonstrated with runnable-shaped tests above)
- Autosave mechanics (Q3): HIGH on the "no timer needed" architectural claim; MEDIUM on the exact
  Quit-ordering guarantee, which depends on `app.rs`'s call-order discipline being maintained by
  whoever writes the actual `ui()` body (a plan-level concern, not a research uncertainty)
- Accordion/scrolling feel (Q2): MEDIUM — APIs confirmed present and correctly shaped; the
  experiential "does it feel right" question is explicitly a look-at-it verification, same caveat
  Phase 13's own research gave its theming work
- Testing strategy (Q6): HIGH on "pure functions cover the mutation logic" (direct continuation of
  Phase 13's own established, verified pattern); MEDIUM on `egui_kittest` specifically, since it
  was not exercised hands-on this session

**Research date:** 2026-09-07
**Valid until:** ~30 days for the version pins (unchanged from Phase 13's own estimate — egui moves
roughly monthly); the API-shape findings (CollapsingHeader/ComboBox/DragValue/checkbox/
selectable_value signatures, round-trip test evidence) are stable within the pinned `0.36` minor
and should hold for the rest of this milestone regardless of elapsed time, since no further
`egui`/`eframe` upgrade is planned before v0.4 ships.

---

## Risks for the planner

1. **The `RuleId`/`WorkingRules` module (Pattern 1) is the one piece of genuinely new architecture
   in this phase** — everything else is "call an existing egui widget" or "call an existing
   `dynamic.rs` function." Budget a dedicated task for it with its own unit-test acceptance
   criterion (the five tests sketched above are a reasonable starting shape), separate from the
   presentation-layer tasks that consume it. This is also the piece most directly answering D-18
   and D-19, so it is worth verifying in isolation before wiring it to any widget.
2. **The accordion's actual feel (Q1's A1, Q2's A2) cannot be fully confirmed without opening the
   overlay over a live or mocked ten-rule list.** Budget a manual-verification checklist item
   specifically for "expand a rule near the bottom of the list, confirm its full body — not just
   its header — scrolls into view," matching the specificity Phase 13's own manual checklist used
   for the repeated-summon focus check (a single-open test would not catch a body-clipping bug any
   more than a single-summon test caught the `MARKER_ACTIVATE` defect).
3. **Reorder-button glyph rendering (Pitfall 4) is a near-certain repeat of a defect this project
   already found and fixed once.** Treat "do ▲/▼ render, or do they need hand-painted triangles"
   as a known risk to check early in implementation, not a surprise to discover during polish —
   the fix pattern (`widgets::header_plate`'s stroke-painting approach) already exists in this
   codebase and should be reused directly if needed, not reinvented.
4. **Autosave's Quit-ordering guarantee (Pattern 5) depends on where in `app.rs`'s `ui()` body the
   save call sits relative to the Quit-handling branch.** This is a code-review-level concern for
   whichever plan implements it — a future refactor that moves the Quit branch earlier than the
   save step would silently reintroduce a lose-the-last-edit-on-Quit bug with no test to catch it
   (this is presentation-layer glue, the one category `PITFALLS.md` §8 accepts has no automated
   test story). Worth a one-line comment at the call site pointing at this constraint, so a future
   editor of `app.rs` doesn't reorder it by accident.
5. **`egui_kittest` adoption is genuinely optional, not a hidden requirement.** Do not let "Phase
   13's research deferred this decision to Phase 14" read as "Phase 14 must adopt it" — given D-18
   already pushes essentially all testable logic into `overlay::edit` (pure functions), the
   marginal test-coverage benefit of adding a GPU-adjacent test harness for the remaining thin
   presentation glue is genuinely small. Recommend treating it as a stretch task, not a blocking
   Wave 0 dependency.
6. **`NumericCondition`/`AreaLevelRange`'s missing `skip_serializing_if` (Pitfall 2) and
   `HashMap` key-order instability (Pitfall 3) are both real, both harmless, and both easy to
   mistake for regressions during manual verification** if a reviewer diffs `dynamic.json` before
   and after an edit expecting a minimal diff. Flag this explicitly in whatever verification
   checklist Phase 14 produces, so it isn't logged as a new defect.

## Recommended task shape

The roadmap estimates 2-3 plans. Given the risk concentration above, the natural split is:

**Plan A — Pure mutation + autosave library layer, no widgets.**
1. `overlay::edit::WorkingRules` (Pattern 1: `RuleId`, `from_rules`/`to_rules`, `add`/`delete`/
   `move_up`/`move_down`/`toggle_enabled`) — fully unit-tested, zero egui imports. This is the
   piece EDIT-01/EDIT-02 and D-18/D-19 actually hinge on.
2. Per-condition set/clear functions over `Rule`/`When` (Pattern 2's `set_defence`/
   `clear_defence_group`, plus the equivalent for class list, rarity comparator, sockets/quality,
   corrupted) and `available_styles`/`available_classes` enumeration — all pure, all unit-tested
   against `dynamic_base.json` and hand-built fixtures, covering EDIT-03..08's semantics
   independent of any widget existing yet.
3. `overlay::store::save` (Pattern 5's atomic write) plus the `dynamic_base.json` round-trip guard
   test (Code Examples' `dynamic_base_json_round_trips_through_config_losslessly`) — closes Q4's
   risk concretely, in code, before any UI depends on it.

**Plan B — Presentation layer: accordion, per-condition widgets, add/delete/reorder buttons.**
4. `CollapsingHeader`-based accordion wired to Plan A's `WorkingRules` and `Option<RuleId>`
   expansion state (Pattern 3), including the scroll-into-view behavior (Q2) and the reorder
   arrow buttons (with Pitfall 4's glyph check as an explicit sub-step, not an afterthought).
5. The per-condition widgets themselves (style `ComboBox`, class-group `CollapsingHeader`s,
   rarity comparator, defence three-checkbox group, sockets/quality `DragValue`, corrupted —
   Pattern 2's uniform enable-toggle architecture) plus the circular toggle painter (Pattern 4) —
   each wired to call into Plan A's pure functions, never mutating `Rule`/`When` fields directly
   from widget code.
6. Autosave wiring (the `dirty` flag and the per-frame save call, Pattern 5) and the inline
   delete-confirm (D-08).

**Plan C — Apply + the D-03 unapplied-changes indicator.**
7. Wire the Apply button to the unchanged `dynamic::compile_store`/`write_filter` pair (D-15),
   render the Options → Game → Item Filter → Refresh instruction (D-16), surface compile/write
   failures in the stripe rather than a console (D-17), and add the `applied: bool` indicator that
   Pattern 5 already sets `false` on every edit and Apply sets back to `true` on success (D-03).

If the roadmap's "2-3 plans" lands on 2, Plan C's scope (a handful of straightforward wiring calls
against already-tested library functions) is the one most safely foldable into the end of Plan B
without materially changing the risk profile — it introduces no new mechanism this research didn't
already resolve. Plan A and Plan B should stay separate regardless: Plan A is where D-18's
unit-test bar is actually met, and reviewing/merging it cleanly before any widget code exists makes
it trivial to confirm that bar independent of how the presentation layer eventually looks.
