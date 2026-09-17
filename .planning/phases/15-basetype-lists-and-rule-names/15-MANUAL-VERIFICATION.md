# Phase 15 — BaseType Lists and Rule Names: In-Game Manual Verification

**Purpose:** most of this phase is already proven by tests — the base-type parse/dedup/remove
vocabulary, the empty-list-collapses-to-`None` guard, the empty-name-collapses-to-`None` guard,
the emitted `Show # Change Bases` block with a `BaseType ==` line and no `Class` line, the derived
`rule {idx}` fallback for unnamed rules, the row-header format, and one three-frame headless click
test per new/changed control proving each panel stays open on the repaint after its toggle was
switched on (`PITFALLS.md` §8: if a bug can only be found by clicking, it belongs in the library,
not here — and by that same rule, most of this phase's correctness *is* in the library). This
checklist is deliberately narrow, four items, because most of this phase's risk was automatable
and was automated in plans 15-01 and 15-02. It covers only what genuinely needs a human eye and a
running game: the felt behaviour of the three toggles, keyboard-only base-type entry, layout at
the stripe's fixed 460px width, and whether the author's own hand-written `Change Bases` rule —
built entirely in the overlay this time — actually reaches the game.

**Before you start:**
- Build the release binary: `cargo build --release --bin ff`.
- Launch Path of Exile 2 in **windowed** or **windowed-fullscreen** mode (Options → Graphics →
  Window Mode). **Exclusive fullscreen will not show the stripe and is not detected**
  (`PITFALLS.md` §2) — that is a display-mode issue, not a bug, if the stripe seems to have
  vanished.
- Enter a town or map so the game holds normal keyboard/mouse focus, then summon the stripe with
  `Ctrl+Shift+Z`.
- Have a `dynamic.json` loaded with at least one rule to expand (the seeded `dynamic_base.json`
  template qualifies).

**Expected, not a defect:** the two cosmetic round-trip quirks documented in `CLAUDE.md` — an
edited rule's `sockets`/`quality`/`areaLevel` objects growing explicit `"eq": null`/`"min": null`
keys, and `classGroups`/`build.armourTypes` reshuffling key order — can still appear in
`dynamic.json` after Apply. Not something to report here.

---

## 1. The three new toggles stay open, and the two text fields commit differently

Serves: D-07/D-09/D-11 (nothing pre-selected, nothing auto-filled, a panel does not flicker shut
the frame after it opens), and RESEARCH Pitfall 1. The state mechanism itself is already covered
by three automated three-frame click tests, one per control (`the_name_field_survives_the_frame_
after_its_toggle_was_clicked` and its two siblings) — this half of the check is for how it feels
in a real window, not whether the underlying state machine is correct.

1. Expand a rule. Tick **has a name**. Wait a beat without touching anything else. **Expected:**
   the name text field is still there, empty, with the hint text `unnamed` — it did not close
   itself on the next repaint.
2. Tick **match classes**. Wait a beat. **Expected:** the four class groups (WEAPONS / ARMOUR /
   JEWELLERY / FLASKS & CHARMS) appear, collapsed, with nothing pre-checked inside any of them.
3. Tick **match base types**. Wait a beat. **Expected:** an empty list and the add field appear,
   with no default entry.

**First veto invitation — the base-type control's own on/off toggle:** it was added for
consistency with every other condition and to keep a freshly-added rule's body clean, but it may
be one switch too many — a base-type list could instead always be visible, present-or-empty, with
no separate toggle at all. Dropping it is a small, contained change to one function
(`render_base_types`). Say in the Result below if it feels like the right call or like clutter.

Then the deliberate asymmetry, which is written here to be **vetoed**, not announced: nothing in
`15-CONTEXT.md`'s D-01..D-16 decided how the name field commits, and it is not one of the
D-14/D-15/D-16 discretion items either — it is a planner call, made without your sign-off.

4. Type half a name into the name field (e.g. `Exceptio`) and click somewhere else in the
   stripe — not Enter, just a click away. **Expected:** the name **commits**, on the reasoning
   that a name you typed and clicked away from should not silently vanish.
5. Do the same in the base-type add field: type half a base name and click away without pressing
   Enter or `+ ADD`. **Expected:** the half-typed base type is **discarded** — D-03 pins that field
   to Enter or the add button only.

**Second veto invitation — the name-field-vs-base-type-field commit asymmetry:** below, in
addition to steps 1-5's pass/fail, record whether the
name-field-vs-base-type-field difference (name commits on any focus loss; base type needs Enter
or `+ ADD`) reads as sensible or as a trap. If the name field should also require Enter — or
should require it exclusively, matching the base-type field — say so: it is a one-line change to
the commit condition in `render_name`. This phase has already reversed two planner leanings
(textarea → item-by-item entry, name-replaces-style → name-appended in the row header), so an
unflagged interaction default is a live risk here rather than a theoretical one.

**Result:** PASS -- author, 2026-09-09.

**Both veto invitations declined — the planner's calls stand:**

- *The base-type control's own on/off toggle* stays. It is not clutter; a freshly-added rule opens
  clean, which is what D-09 asked for.
- *The name-field-vs-base-type-field commit asymmetry* stays as shipped: the name commits on any
  focus loss, the base-type field requires Enter or `+ ADD`. Recorded here as **approved**, not
  merely unobjected-to, so a later reader does not mistake it for an undecided default.

Verified after `d2a52ed`, which restored the button chrome the row-header scope was blanking —
before that fix every control in the body rendered as bare text rather than as buttons.

---

## 2. Keyboard-only base-type entry

Serves: D-01/D-02/D-03, RESEARCH Assumption A1 (focus/scroll while typing).

1. With **match base types** open on a rule, type `Heavy Belt`, press Enter, type `Utility Belt`,
   press Enter, type `Armoured Cap`, press Enter, type `Sacred Focus`, press Enter — without
   touching the mouse between any of them.
2. **Expected:** all four entries are listed, the field cleared itself after each Enter, focus
   never needed to be reclaimed by hand, and no stray scroll jump happened mid-typing.
3. Click **Remove** on the second entry (`Utility Belt`). **Expected:** exactly that one entry
   disappears; the other three remain, in their original order.

**Failure would look like:** the field not clearing after Enter, focus being dropped so a
subsequent keystroke does nothing until you click the field again, the list scrolling out of view
while typing, or `Remove` removing the wrong entry or more than one.

**Result:** PASS -- author, 2026-09-09. Four base names typed with only Enter between them; focus
is retained across commits, so entry never needs the mouse. Removal from any position works.

---

## 3. Layout at 460px

Serves: RESEARCH Open Question 2, D-10 (the row header format).

1. On a rule with **match base types** open, add a long base name: `Expert Feathered Sallet`.
2. Give the same rule a name (e.g. `Long Name Test`).
3. **Expected:** the base-type entry's wrapped label and its `Remove` button read cleanly side by
   side — the label wraps onto a second line rather than overlapping or pushing the button off the
   edge of the stripe.
4. Collapse the rule and look at its header line, `NN  style - name`. **Expected:** it is not
   clipped or truncated by the stripe's 460px width.

**Failure would look like:** the long base name's text overlapping the `Remove` button, running
past the stripe's right edge, or the header line being cut off mid-word.

**Result:** PASS -- author, 2026-09-09. Verified against the layout as it stands after two
corrections: rows initially stretched to fill the panel (`ac1b985`), then the names sat
right-aligned against their buttons (`d2a52ed`). Names now read from the left edge, Remove sits at
the right, and rows size to their content.

---

## 4. The Change Bases rule, end to end

Serves: roadmap success criteria 1, 2, and 4 (base-type matching is expressible in the overlay,
a rule can be base-type-only with no `Class` line, and the change is provable against a running
game) — the motivating example from `15-CONTEXT.md`:

```
Show # Change Bases
  Rarity == Normal
  BaseType == "Heavy Belt" "Utility Belt" "Armoured Cap" "Sacred Focus"
```

1. Build it **entirely in the overlay**: expand a rule (a fresh `+ ADD RULE`d one is fine), switch
   **match classes** off (or leave it off if it was never on), set **rarity** to exactly `Normal`,
   add the four bases (`Heavy Belt`, `Utility Belt`, `Armoured Cap`, `Sacred Focus`) under
   **match base types**, and give the rule the name `Change Bases`.
2. Click **APPLY**. **Expected:** the stripe reports success and shows the reload instruction
   `Options → Game → Item Filter → Refresh`.
3. In PoE2, open **Options → Game → Item Filter** and click **Refresh**.
4. Inspect the written `dynamic.filter`. **Expected:** it contains a block reading
   `Show # Change Bases` with a `Rarity == Normal` line and a `BaseType ==` line listing all four
   bases, and **no** `Class` line anywhere in that block.
5. Find or generate a drop of one of the four bases at Normal rarity. **Expected:** it behaves
   according to the `Change Bases` rule in game (whatever style the rule was given), confirming
   the written filter is the one the game is actually reading.

**Failure would look like:** a stray `Class` line appearing in the emitted block, a base name
missing from the `BaseType ==` line, or the in-game drop not reflecting the rule after Refresh.

**Result:** PASS -- author, 2026-09-09. The author's own *Change Bases* rule, built entirely in
the overlay, reaches the game as a `BaseType ==` block with no `Class` line. This is the phase
goal and the thing base-type-only matching existed to make possible.

---

### If something fails

Route a FAIL to the plan that owns the behavior — do not re-diagnose from scratch:

| Symptom area | Owning plan | What it built |
|---|---|---|
| A toggle flickers shut the frame after being switched on | 15-02 | `panel_open`/`set_panel_open`, the three `render_*` toggles in `overlay/rule_editor.rs` |
| Base-type parse/add/remove/clear behaves wrong (dedup, quote-stripping, paste-splitting) | 15-01 | `overlay::edit::parse_base_type_entry`/`add_base_type`/`remove_base_type`/`clear_base_type` |
| The name doesn't show in the row header, or an unnamed rule's label is wrong | 15-01 | `row_header_text` in `overlay/app.rs`, the emitter's existing `derived_label` fallback |
| The emitted block has a stray `Class` line, or is missing `BaseType ==` | 15-01 | the `set_name`/`clear_class` vocabulary and its emitted-filter-text tests in `overlay::edit` |
| Apply doesn't write the filter or doesn't report the reload path | 14-04 (unchanged by this phase) | `overlay::apply`, the Apply button in `app.rs` |

---

*Checklist authored: 2026-09-08. Phase: 15-basetype-lists-and-rule-names, Plan 03.*

---

## Not separately confirmed

Three changes landed **after** this checklist was written, in response to the author looking at the
running build. Each carries automated tests, and each was visible on screen while items 1-4 were
being run, but none was given its own recorded check. Listed so a later reader is not misled into
thinking every line below was exercised deliberately.

- **Summary ordering (`6786622`).** A collapsed rule now reads in the same order as its expanded
  body — classes, bases, rarity, defences, sockets, quality, corrupted, unidentified tier. Pinned
  by `the_summary_order_mirrors_the_editor_body_order`, verified by mutation (swapping sockets and
  quality fails it).
- **Sockets and quality became sliders (`6786622`), 1..=6 and 1..=30.** Both scales start at 1
  because `>= 0` matches every item that has ever dropped. **Quality's ceiling of 30 rather than
  20 is a Claude call the author has not ruled on** — corrupted items exceed the normal cap, so a
  filter capped at 20 could not describe them. One constant in `edit.rs` (`QUALITY_MAX`) if the
  author wants 20 instead.
- **Button chrome in rule bodies (`d2a52ed`).** `app.rs`'s row-header scope blanks
  `weak_bg_fill`/`bg_stroke` so the header's own frame stays invisible; the body renders inside
  that scope and inherited it, so every control in an expanded rule drew as bare text. The fix
  restores the palette in `render_rule_body`, which affects the style dropdown and class group
  headers as well as the two base-type buttons. Item 1's Result was recorded after this fix.
