# Phase 14: Rule Editor + Apply - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-09-06
**Phase:** 14-Rule Editor + Apply
**Areas discussed:** Editing surface, persistence timing, reordering interaction, new-rule
defaults, delete safety

**Format note:** presented in prose rather than option menus, per the recorded preference that
open design questions be asked as prose. The tables below reconstruct the alternatives laid out.

---

## Editing surface

| Option | Description | Selected |
|--------|-------------|----------|
| Accordion — expand a rule in place | Ordinal position stays visible while editing; stripe scrolls more | ✓ (Claude's judgment) |
| List on top, editor pane below | The EE2 reference shape; hides position while editing | |
| Separate edit mode replacing the list | Most room per control; loses all list context | |

**User's choice:** *"fine by me, use your best judgment"*
**Notes:** Chosen because ordinal position *is* precedence in this model — seeing where a rule sits
while changing it is part of understanding the change. Paired sub-decision: **the stripe does not
resize when a rule expands**; controls stack vertically inside the row instead. That keeps Phase
13's "never sized to the full screen" invariant trivially true rather than something to re-verify,
and avoids a dynamic resize interacting with the raw-HWND show/hide path. The offer to widen the
stripe in edit mode was raised and declined by the same reasoning.

---

## Persistence timing

| Option | Description | Selected |
|--------|-------------|----------|
| Autosave everything, filter included | Simplest mental model; writes the game's file unasked | |
| Autosave `dynamic.json`; filter only on Apply | Store is safe from crashes; the game's file changes only deliberately | ✓ |
| Save only on Apply | One "I mean it" button; a crash loses tuning work | |

**User's choice:** *"autosave the json, change the .filter only on apply"*
**Notes:** Raised as a consequence and accepted into the design: the store and the filter can now
diverge, so the UI must show that there are saved-but-unapplied changes (D-03). Without it, "did I
apply that?" becomes guesswork and the player blames the filter for not changing when it was in
fact never written. Also required autosave to use the existing atomic write-then-rename discipline
(D-04) — a half-written `dynamic.json` would be worse than a lost edit.

---

## Reordering interaction

| Option | Description | Selected |
|--------|-------------|----------|
| Drag-and-drop | Expected, but fiddly in egui and worse in a narrow scrolling stripe | |
| Up/down arrows per row | Unglamorous, always works | ✓ |

**User's choice:** *"arrows"*

---

## New-rule defaults

| Option | Description | Selected |
|--------|-------------|----------|
| Append at the end, enabled | A catch-all lands where first-match-wins makes it harmless | ✓ |
| Insert after the selected rule, disabled | Claude's suggestion — cannot affect the filter until switched on | |
| Insert at the top | Maximum danger: an empty `when` would swallow everything | |

**User's choice:** *"add to the end of list, i.e. bottom, it doesn't take effect until applied and
reloaded in-game by the user so a catch all/incomplete rule is a non-issue"*

**Notes:** The author's reasoning defused a concern Claude had raised as one of the two sharpest
edges in the phase. Nothing an incomplete rule does can reach the game until the player deliberately
Applies *and* reloads in-game — so the disabled-by-default guard rail Claude proposed was
unnecessary. Claude chose the remaining detail: a new rule is `{"style": "unchanged", "when": {}}`,
`unchanged` being the no-op style, so even after Apply it changes nothing until configured. Worth
remembering more generally — it is the reason several otherwise-scary edit operations need no
guards.

---

## Delete safety

| Option | Description | Selected |
|--------|-------------|----------|
| Quick confirm | Cheap; stops a misclick destroying a hand-tuned rule | ✓ |
| Undo stack | Nicer, more state to carry | |
| Neither | Fastest, least forgiving | |

**User's choice:** *"have a quick confirm for deleting, can look like whatever you want"*
**Notes:** Claude's design within that latitude — an **inline** confirm, where the row itself
becomes `Delete? [Yes] [Cancel]`, rather than a modal dialog. Suits a narrow stripe and needs no
new window (this project has spent a whole phase on window configuration; adding a second one
casually would be unwise). No undo stack this phase.

---

## Claude's Discretion

- Accordion layout and the vertical control stacking inside an expanded row.
- Class picker as a grouped collapsible checkbox list (Weapons / Armour / Flasks & Charms).
- Inline delete confirmation styling.
- New rule contents (`style: "unchanged"`, empty `when`).
- Which egui widgets realise each control (a research question).

## Deferred Ideas

- Undo / undo stack.
- Drag-and-drop reordering.
- Rule search or filtering.
- A settings file (hotkey binding, stripe geometry persistence).
- Editing the style palette, class lists or economy tiering from the overlay — pinned for v0.4.

## Closed, not deferred

- Dimming the game behind the stripe — declined by the author during Phase 13 verification
  (*"not really necessary"*). Noted so it is not resurrected as a Phase 14 styling idea.
