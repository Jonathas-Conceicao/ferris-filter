# Phase 14 — Rule Editor + Apply: In-Game Manual Verification

**Purpose:** most of Phase 14 is already proven by tests — the mutation semantics (`overlay::edit`,
37 tests), the criterion-5 defence emission and criterion-6 single-condition removal against real
emitted filter text, the criterion-2 flask case, the round-trip guard, and 303 pre-existing tests
overall (`PITFALLS.md` §8: if a bug can only be found by clicking, it belongs in the library, not
here — and by that same rule, most of this phase's correctness *is* in the library). This checklist
is deliberately narrow: it covers only what genuinely needs a human eye and a running game —
accordion feel, scroll-into-view, painted glyphs, and Apply's in-game round trip.

**Before you start:**
- Build the release binary: `cargo build --release --bin ff`.
- Launch Path of Exile 2 in **windowed** or **windowed-fullscreen** mode (Options → Graphics →
  Window Mode). **Exclusive fullscreen will not show the stripe and is not detected**
  (`PITFALLS.md` §2) — that is a display-mode issue, not a bug, if the stripe seems to have
  vanished.
- Enter a town or map so the game holds normal keyboard/mouse focus, then summon the stripe with
  `Ctrl+Shift+Z`.
- Have a `dynamic.json` with at least ten rules loaded (the seeded `dynamic_base.json` template
  qualifies, or use whatever working set you already have from Phase 13's testing).

**Expected, not a defect:** if `dynamic.json` looks different from how you left it after any edit
below — new `"eq": null`/`"min": null` keys inside a condition object, or `classGroups`/
`build.armourTypes` in a different key order — that is documented in `CLAUDE.md` (RESEARCH
Pitfalls 2 and 3) and is not something to report here.

---

## 1. The accordion's feel (D-01)

Roadmap/decision: rules expand in place; the stripe never resizes; rows above and below stay
visible.

1. With the stripe summoned and at least ten rules listed, click a rule somewhere in the middle
   of the list to expand it. **Expected:** its condition controls appear inline, directly below
   its header; the rules above and below it are still visible (not hidden, not scrolled away by
   the expansion itself); the stripe's own width and screen position do not change.
2. Click the same rule's header again to collapse it. **Expected:** it collapses back to a single
   summary line with no lingering visual snap or flicker against the rules around it.
3. Expand a second, different rule while the first stays collapsed. **Expected:** only one row's
   full body is open at a time reads naturally (nothing enforces this structurally, but this
   confirms it doesn't feel broken if you leave two expanded, either).

**Failure would look like:** the stripe growing/shrinking/moving when a row expands, rows above or
below the expanded one disappearing, or a visible one-frame flicker between collapsed and
expanded states (RESEARCH Assumption A1).

**Result:** PASS -- author, 2026-09-08. Verified against a row layout that had changed substantially since this checklist was written (full-width clickable header with hover highlight, the summary moved to its own wrapped line beneath).
---

## 2. Scroll-into-view near the bottom (RESEARCH Assumption A2)

Roadmap: an expanded row's whole body — not just its header line — must be reachable without
fighting the scroll, even near the bottom of a long list.

1. With a ten-rule (or longer) list loaded, scroll to the bottom of the stripe's rule list.
2. Click the **last** rule in the list to expand it.
3. Without manually scrolling yourself, look at what's visible. **Expected:** the row's full body —
   every condition control inside it, not just its header/summary line — is either already in view
   or was scrolled into view automatically by the expand action itself.
4. If any part of the body is cut off, try scrolling down manually. **Expected:** that content is
   reachable by scrolling; it is not clipped or cut off by the stripe's own layout.

**Failure would look like:** the header line scrolls into view but the last one or two condition
controls (e.g. the corrupted toggle) remain below the visible area with no way to scroll to them,
or the scroll position ends up wrong entirely (e.g. scrolled to the top instead of the opened row).

**Result:** PASS -- author, 2026-09-08. Closes RESEARCH Assumption A2 / Open Question 2: `scroll_to_me` on the header response does bring the expanded body into view, not just the header line.
---

## 3. Painted glyphs — reorder arrows and circular toggles (RESEARCH Pitfall 4 / Assumption A3)

Roadmap: Phase 13 shipped a tofu `✕` for a whole wave before anyone looked at a running window;
this phase's new painted controls (arrows, circular toggle) are the same risk category.

1. Look at any rule row's up/down reorder arrows. **Expected:** two small filled triangles
   pointing up and down respectively — not empty boxes, not missing glyphs, not obviously the
   wrong shape.
2. Look at the row's enable/disable control. **Expected:** a circle, not a square — PoE2's own
   toggle shape, per D-24. Toggle it on a rule and off again. **Expected:** the checked and
   unchecked states are visually distinguishable at a glance (a filled inner dot vs. an empty
   ring, or similar) — not just barely different, and not relying on color alone.
3. Try the first row's **up** arrow and the last row's **down** arrow (both structurally
   disabled — there's nowhere for them to move). **Expected:** the arrow is still visible (greyed
   out or otherwise visually inert) rather than missing or misaligned; clicking it does nothing.

**Failure would look like:** any tofu/placeholder box in place of an arrow or toggle, the toggle
rendering as a square, or the checked/unchecked states being indistinguishable without color.

**Result:** PASS -- author, 2026-09-08. Arrows and circular toggles render as shapes. Note the separate tofu defect found the same day in the Apply confirmation line, fixed at its root in `theme::install_fonts` (commit c9b62b9).
---

## 4. Apply end to end, in-game (D-15/D-16)

Roadmap: Apply compiles the store and writes `dynamic.filter` into the game folder, then the
player must reload it manually — PoE2 never re-reads a filter on its own.

1. Pick a rule with a visible in-game effect (e.g. one covering a currency or equipment class you
   can find quickly) and change something about it — its style, or toggle a condition.
2. Click **APPLY**. **Expected:** the stripe shows a success message naming the written file and
   containing the exact instruction `Options → Game → Item Filter → Refresh`.
3. In PoE2, open **Options → Game → Item Filter** and select **`dynamic`** in the filter dropdown
   if it is not already selected (first time only), then click **Refresh**.
4. Find or generate a drop that the edited rule covers. **Expected:** its appearance in-game
   matches what the edited rule now specifies (color/style change, or a hide/show change if you
   toggled the rule's `enabled` state).

**Failure would look like:** Apply reporting success but the in-game filter not changing after a
Refresh, or no reload instruction appearing at all.

**Result:** PASS -- author, 2026-09-08. Apply writes the filter and the game picks it up after Options > Game > Item Filter > Refresh.
---

## 5. The unapplied-changes indicator (D-03)

Roadmap/decision: because the store autosaves independently of Apply, the two can diverge, and the
stripe must always say so.

1. Note the indicator text near the Apply button right after a successful Apply (from item 4
   above, if already done) or after a fresh Apply now. **Expected:** it reads as "up to date" /
   "matches saved rules" — some wording that clearly says the filter matches what's saved, not
   just a color change.
2. Make any edit — toggle a condition, flip a rule's enable state, reorder two rules. **Expected:**
   the indicator immediately changes to wording that says the filter is out of date / does not
   match the saved rules (not just a color shift — read the words, not just the color).
3. Click **APPLY** again. **Expected:** the indicator flips back to "up to date," and the prior
   "out of date" wording is gone.

**Failure would look like:** the indicator not changing after an edit, the indicator only changing
color without different wording, or the indicator still saying "out of date" after a successful
Apply.

**Result:** PASS -- author, 2026-09-08, **against the autosave persistence model**. Superseded the same day: the author asked for changes to stay in memory until Apply, with a Revert alongside it. **Re-verified 2026-09-08 against the replacement model** -- see check 8 in the second pass below, which is the authoritative result for this behaviour.
---

## 6. Roadmap criterion 2, by hand — the flask case

Roadmap success criterion 2: disabling a single rule covering the flask and charm classes removes
flask drops from the generated filter, in one click. (The compiled-text version of this is already
covered by an automated test — `disabling_one_flask_rule_removes_the_flask_block_from_the_compiled_
filter`. This item is the in-game confirmation of the same behavior.)

1. Find the rule (or rules) covering flask and charm classes in your loaded `dynamic.json` and
   note how flasks/charms currently display in-game (color, or hidden).
2. Expand that rule and switch it off with the circular toggle (do not delete it).
3. Click **APPLY**, then reload in-game (**Options → Game → Item Filter → Refresh**).
4. Find or generate a flask or charm drop. **Expected:** its display no longer follows the rule you
   disabled — it now falls through to whatever the next matching rule (or the game default)
   produces, a visibly different result from step 1.
5. Re-enable the rule, Apply, and reload again to restore your prior setup. **Expected:** flask/
   charm display returns to what it was in step 1.

**Failure would look like:** flask/charm drops displaying identically before and after disabling
the rule, or the toggle not surviving Apply (i.e., the disabled state doesn't make it into the
compiled filter).

**Result:** PASS -- author, 2026-09-08. The milestone motivating example works by hand, end to end.
---

---

# Second pass — the out-of-band fix stream (2026-09-08)

Checks 1-6 above were written against the state of the editor at the end of plan 14-04's
implementation. Between that checklist being authored and the phase closing, eleven further
commits landed in response to the author using the editor against a live game: a jewellery class
group, the missing unidentified-tier control and its slider, a readable wrapped summary, full-row
click targets, the `kite` rename, a font-fallback fix, the stale-Apply fix, Revert with an
in-memory persistence model, and the "Any" summary wording.

Those are not covered by checks 1-6, and one of them (Revert) invalidated check 5 outright. This
second pass covers what the first could not have.

**Everything below requires relaunching the overlay** — a running `ff.exe` predates these fixes.

---

## 7. Apply writes the state you pressed it on (regression: 3375ad4, re-risked by 6b7c46c)

The defect: Apply compiled the store *before* the frame's edits reached it, so each Apply wrote
the previous edit's state — a create-then-apply, delete-then-apply cycle showed the created rule
only on the *second* Apply. Fixed in `3375ad4` by re-projecting the working model into the config
immediately before writing, and put back at risk by `6b7c46c`, which made Apply the single writer
and therefore made the persist-then-compile ordering load-bearing (`compile_store` reads the store
from disk; compiling first would reintroduce the same off-by-one through a different route).

1. Add a rule with **+ ADD RULE**, then click **APPLY**.
2. Delete that rule, then click **APPLY** again.
3. Inspect `dynamic.filter` (or the in-game result) after each Apply. **Expected:** each write
   reflects the store as of the moment Apply was pressed — the new rule present after step 1,
   absent after step 2.

**Failure would look like:** the added rule missing from the first Apply's output and appearing in
the second's — the original symptom exactly.

**Result:** PASS -- author, 2026-09-08. The original repro no longer reproduces.
---

## 8. Revert, and the indicator under the in-memory model (supersedes check 5)

D-02 was reversed on 2026-09-08: edits are held in memory and Apply is the only writer, with a
Revert to discard. Check 5 above tested the indicator against the autosave model it replaced, so
its pass does not carry over.

1. With no unsaved changes, look at the controls near Apply. **Expected:** no Revert button — it
   appears only when there is something to discard.
2. Make any edit. **Expected:** Revert appears, and the indicator's *wording* says the written
   filter no longer matches the rules.
3. Click **REVERT**. **Expected:** the rule list returns to its last-applied state, the indicator
   reads clean, and Revert disappears again.
4. Make another edit and click **APPLY** instead. **Expected:** the indicator reads clean, Revert
   disappears, and both `dynamic.json` and `dynamic.filter` now hold the edit.

**Expected, not a defect:** Revert collapses any expanded row and cancels a pending delete
confirm. Rebuilding the rule list mints fresh `RuleId`s, so a retained expansion would point at a
rule that no longer exists.

**Known trade-off, accepted:** unsaved edits do not survive the process. D-02 originally chose
autosave specifically to protect against that; the replacement trades crash-safety for deliberate
discard. Recorded in `14-CONTEXT.md` under D-02 as superseded.

**Result:** PASS -- author, 2026-09-08.
---

## 9. The Apply confirmation renders without tofu (c9b62b9)

The confirmation line came out with placeholder boxes where its arrows should have been
(screenshot `poe2_print005`) — the same defect class as Phase 13's tofu `✕`, and the same root
cause: `FontDefinitions`' `entry().or_default()` creates an *empty* family, so Cinzel was
registered with no fallback chain at all. Fixed at the root, plus the instruction text itself was
moved to plain ASCII.

1. Click **APPLY** and read the confirmation line. **Expected:** `Options > Game > Item Filter >
   Refresh`, every character rendered — no boxes, no missing glyphs.

**Result:** PASS -- author, 2026-09-08. Now reads with `>` separators and renders clean.
---

## 10. The jewellery group and the "Any <group>" summary (35b695e, f43dcc2)

1. Expand a rule and open its class picker. **Expected:** a **JEWELLERY** group alongside
   WEAPONS / ARMOUR / FLASKS & CHARMS, holding Rings, Amulets and Belts.
2. Select all three, then collapse the rule. **Expected:** the summary line reads
   `Any Jewellery`, not the three class names listed out.
3. Deselect one. **Expected:** the summary stops collapsing and lists the remaining classes.

**On the wording:** "Any", not "All" — a summary reads as the rule's own matching test, and any
one class in the list satisfies it (author, 2026-09-08, reversing the first cut).

**Result:** PASS -- author, 2026-09-08.
---

## 11. The `kite` style rename (c9b62b9)

`unidentified` named the rules that happened to use the style rather than what it does, which read
as nonsense when offered for an unrelated rule. Renamed to `kite`, after its minimap icon.

1. Open any rule's style dropdown. **Expected:** `kite` is listed; `unidentified` is not.
2. Confirm the overlay loaded at all. An un-migrated `dynamic.json` naming a style that no longer
   exists is a hard error at compile time by design, so a clean launch is itself the check that
   the rename reached `config.json`, `dynamic_base.json` and `dynamic.json` together.

**Result:** PASS -- author, 2026-09-08.
---

## 12. Full-row click targets (017b643)

Clicking the empty space on either header line — the arrows/title/delete line or the rule-index
line — expands and collapses the rule, not just the text itself. The layout invariant this depends
on is that the filler is allocated *after* the buttons, so they keep their own hit areas.

1. Click the empty space to the right of a rule's title. **Expected:** the rule expands.
2. Click the empty space on the index line. **Expected:** the rule collapses again.
3. Click an arrow, the toggle, and the delete control. **Expected:** each still does its own job
   rather than expanding the row.

**Result:** PASS -- author, 2026-09-08.
---

## Not separately confirmed

Nothing outstanding. All twelve checks across both passes are approved.

---

### If something fails

Route a FAIL to the plan that owns the behavior — do not re-diagnose from scratch:

| Symptom area | Owning plan | What it built |
|---|---|---|
| Accordion doesn't expand/collapse right, scroll doesn't reach an expanded row's bottom | 14-02 | The accordion, `RuleId`-keyed expansion, `CollapsingHeader`/`ScrollArea` wiring |
| Arrows or toggle render wrong, disabled arrows behave wrong | 14-02 | `widgets::circular_toggle`, `widgets::arrow_button` |
| A condition control (style/class/rarity/defence/sockets/quality/corrupted) doesn't change the rule as expected | 14-03 | `overlay::rule_editor::render_rule_body` and its seven per-condition renderers |
| Apply doesn't write the filter, doesn't report the reload path, or doesn't surface a failure | 14-04 | `overlay::apply`, the Apply button in `app.rs` |
| The unapplied-changes indicator doesn't update on edit or Apply | 14-04 | `overlay::apply::ApplyState`/`indicator`, the `if self.dirty` block in `app.rs` |
| Autosave doesn't persist an edit, or `dynamic.json` fails to load afterward | 14-01 / 14-02 | `overlay::store::save`, the per-frame coalesced autosave |
| Revert doesn't restore, or an edit survives the process unexpectedly | 14-04 (post-plan) | the in-memory `baseline` + Revert button in `app.rs` (`6b7c46c`, superseding D-02) |
| A collapsed summary lists classes that should read as `Any <group>` | 14-04 (post-plan) | `overlay::view_model::summarize_classes` |

---

*Checklist authored: 2026-09-07. Second pass (checks 7-11) added 2026-09-08. Phase: 14-Rule Editor + Apply, Plan 04.*
