# Phase 15: BaseType Lists and Rule Names - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-09-08
**Phase:** 15-basetype-lists-and-rule-names
**Areas discussed:** base-type entry, typo handling, class off-switch semantics, new-rule default
state, row header format, name auto-fill

**Format note:** presented as prose with a stated leaning per area rather than option menus, per
the author's recorded preference (*"ask open design questions in prose; their context often
reframes the question"*). That preference earned its keep here — the author's answers reframed two
of the six areas rather than picking from what was offered.

---

## 1. Base-type entry

| Option | Description | Selected |
|--------|-------------|----------|
| Multi-line text box, one base per line | Simple; matches how the list looks in a filter file | |
| Multi-line box that also accepts pasted filter syntax | Claude's leaning — paste from an existing filter and strip quotes | |
| Item-by-item list with per-entry removal | A row per base with its own remove control, plus a single-line add field | ✓ |

**User's choice:** item by item, explicitly *not* a single text box.
**Notes:** *"the only thing I want is to have an easy way to type item by item instead of of a
single text box with all entries, should also be able to remove idividual itens anywhere in this
list."* Rest of the mechanics delegated to Claude (*"1 is up to you"*). This **rejected Claude's
stated leaning** — a textarea would have made removing one name from the middle a text-surgery
operation, which the author spotted immediately.

---

## 2. Typos and validation

| Option | Description | Selected |
|--------|-------------|----------|
| Accept verbatim, no validation | Trim/dedupe only; a typo is invisible until the drop is not highlighted | ✓ |
| Warn on suspicious input | Flag lowercase names, stray quotes, etc. | |
| Validate against a known base list | Would require vendoring a poe2db-derived list | |

**User's choice:** accept verbatim.
**Notes:** *"for now we don't handle typos, the game already handles them and warns the user, so
for now it's up to the user to corret them."* The author supplied a better reason than the one
Claude offered: PoE2 itself reports unknown bases at filter load, so the game is the validator.
Recorded as **declined for now** rather than deferred, with the blocker named (a base list that
does not go stale each league).

---

## 3. What unticking `class` does to the list

| Option | Description | Selected |
|--------|-------------|----------|
| Discard the list | Matches every other off switch (`clear_sockets`, `clear_rarity`, …) | ✓ |
| Retain it hidden | Preserves a laborious 12-class selection across a toggle | |

**User's choice:** either is acceptable; discard chosen by Claude on consistency grounds.
**Notes:** *"unchecking class can destroy the list, but i'm not too worried about this so either
way works for now."* Decided as discard because an off switch that secretly retains state is the
inconsistency, not the data loss — and `edit::set_class` already collapses a fully-deselected list
to `None`, so discard reaches an end state the code already produces.

---

## 4. What a new rule opens with

| Option | Description | Selected |
|--------|-------------|----------|
| Nothing checked; class UI hidden until ticked | New rule is `{style: "unchanged", when: {}}` with no picker shown | ✓ |
| Nothing checked; class picker still visible | Today's behaviour | |

**User's choice:** nothing checked, and the class options hidden until the toggle is ticked.
**Notes:** *"What I want is for a new rull to have nothing checked (as it does now) and for having
class ticked to not show the class options, that part can expand once ticked by the user."*
Clarified that Phase 14's D-07 (new rule shape) is unchanged — only the class picker's visibility
changes. Hidden entirely, not greyed out and not collapsed-but-present.

---

## 5. Row header format

| Option | Description | Selected |
|--------|-------------|----------|
| Name replaces style when present | `03  Exceptional bases` — Claude's leaning; style is visible in the body anyway | |
| Name appended to style | `03  hidden - Exceptional bases` | ✓ |

**User's choice:** `<index> <style>[ - name]`, bracketed part optional.
**Notes:** author's own notation, given verbatim. **Rejected Claude's leaning** — keeping the style
in the header matters to them, and the ` - name` suffix mirrors the filter comment form
(`Show # rule 0 - Exceptional bases`) they asked for.

---

## 6. Name auto-fill

| Option | Description | Selected |
|--------|-------------|----------|
| No auto-fill; empty until typed | Emitter's existing `rule {idx}` fallback handles unnamed rules at emit time | ✓ |
| Pre-fill with `rule N` | Immediately goes stale on reorder, making the block comment wrong | |

**User's choice:** no auto-fill; a name can also be unticked and not used.
**Notes:** *"no auto-fill, a rule name can be 'untiked' and not used."* Confirms the name joins the
same enable-toggle vocabulary as every condition. Unticking clears the stored text, for consistency
with area 3.

---

## Claude's Discretion

Delegated by the author (*"1 is up to you"*), locked in CONTEXT.md as D-14..D-16 so planning does
not reopen them:

- **D-14** — the add field strips surrounding double quotes and splits a pasted quoted list, so
  both typing `Heavy Belt` and pasting `"Heavy Belt" "Utility Belt"` do the obvious thing. Input
  hygiene, not the validation area 2 declined.
- **D-15** — base types get an explicit `Bases:` label in the collapsed summary, since they are
  currently rendered as a bare name list structurally identical to a class list, and a
  base-type-only rule would otherwise be ambiguous about which condition it describes.
- **D-16** — the base-type control sits directly after the class control; the name field sits above
  `style` at the top of the body.

## Deferred Ideas

- **Base-name validation / autocomplete.** Declined for now with a reason, not merely postponed.
- **`EDIT-09` bookkeeping lag** — still unticked in `.planning/REQUIREMENTS.md:105` and `Pending`
  in the coverage table at line 177, though Phase 14 shipped and verified it. Noticed while loading
  prior context; flagged for phase verification or the milestone audit rather than fixed here.
