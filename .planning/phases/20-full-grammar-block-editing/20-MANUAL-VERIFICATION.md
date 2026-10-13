# Phase 20 — Full-Grammar Block Editing: Manual Verification

**Purpose:** the checks headless tests cannot see (layout at real stripe width, popups, the colour
picker, nested scrolling, a live styles.json edit, an in-game reload). Requirements EDIT-01,
EDIT-02, EDIT-06 and STYLE-03 stay "Complete (UAT pending)" until this script has been run.

**Before you start (do not skip):**
- Work on a COPY, never the real game folder first. Copy your game filter folder to a scratch
  directory (for example `C:\Temp\poe2-copy`) and launch the overlay with `FERRISFILTER_HOME` and
  `USERPROFILE` pointing at it, so Apply writes into the copy:
  `set USERPROFILE=C:\Temp\poe2-copy-home` (with the filter at
  `Documentos\My Games\Path of Exile 2\dynamic.filter` inside it), then `cargo run --bin ff`.
  Keep an untouched backup of `dynamic.filter`.
- Path of Exile 2 in windowed or windowed-fullscreen mode (see `13-MANUAL-VERIFICATION.md`).
- Use a filter with a few dozen rules, including a named rule, one with `Continue`, one with a
  `#ff:style` tag, and an invalid commented-out block.

Record Result as PASS / FAIL with a note for every step.

---

1. **Condition controls at 460px.** Expand a rule with Class, BaseType, Rarity, Sockets, Quality,
   ItemLevel and a defence value.
   **Expected:** every slider sits side by side with its number field; the class picker shows
   groups from `config.json`; chip lists wrap and each chip has a 24px remove X; the sockets
   control shows its slots; nothing is clipped at the right edge. **Result:**

2. **+ condition popup.** Press **+ condition**. Type in the search box; press Esc; reopen; click
   outside; then shrink the window height to about 320px and scroll the popup.
   **Expected:** search filters the grouped list; Esc and an outside click close it without
   adding anything; the popup scrolls and stays on screen at 320px. **Result:**

3. **Colour picker with alpha.** Open a text colour swatch, change alpha, close it. Then open a
   translucent colour and close it WITHOUT changing anything, then press Apply.
   **Expected:** the edit changes only that action's line; opening and closing with no change
   leaves the file untouched and Apply says "Nothing changed". **Result:**

4. **Minimap, beam and sound controls.** Add and edit MinimapIcon, PlayEffect, PlayAlertSound,
   CustomAlertSound, DisableDropSound via **+ action**.
   **Expected:** dropdowns list the valid colours/shapes; sound sliders respect their ranges
   (id 1-16, volume 0-300); each edit rewrites only its own line. **Result:**

5. **Nested scrolling.** Expand a rule with a long BaseType chip list inside the scrolling rule
   list; scroll with the wheel over the chips and over the rule list.
   **Expected:** the chip list scrolls on its own while hovered, the rule list scrolls elsewhere,
   and neither traps the wheel. **Result:**

6. **Apply a style and the locked rows.** In a rule's STYLE group pick a style; then try to edit
   one of the style's actions.
   **Expected:** the rule gains a `#ff:style <name>` line and the style's actions; those rows read
   `from style` and are locked; the collapsed row shows `styled: <name>` (truncated at 12
   characters, tooltip has the full name). **Result:**

7. **Remove style.** Press **Remove style** on that rule.
   **Expected:** the tag disappears, the actions remain as the rule's own and are now editable.
   **Result:**

8. **Edit styles.json while the overlay runs.** Change a colour of a style used by a tagged rule
   in `styles.json` and save.
   **Expected:** within a couple of seconds the stale indicator shows unapplied changes; Apply
   reports `Updated N styled rules.` and the rule's lines now carry the new colour. A hand edit
   of a styled rule's action in the file is overwritten by the same Apply. **Result:**

9. **Unknown style tag.** Put `#ff:style DoesNotExist` above a block (edit the copy, press
   RELOAD).
   **Expected:** the row shows an accent `unknown style` flag, the outcome area lists the
   unknown-style summary (clicking it jumps to that rule), Apply is NOT blocked, and the rule's
   actions are not rewritten. **Result:**

10. **Raw edit of an unknown-keyword line.** Add a block containing an unknown keyword line.
    Edit it and press Enter; then edit a bare `Class` line so it has no value.
    **Expected:** a valid raw edit commits on Enter; an edit that adds a grammar Error shows the
    inline hint and changes nothing. **Result:**

11. **ACCEPTED EXCEPTION: invalid commented-out (cannot-enable) block.** Expand a commented-out
    block that is not valid grammar.
    **Expected:** its lines are shown read-only with the can't-enable explanation, the toggle
    refuses to enable it, and the row is flagged `can't enable`. Record the Result as
    "exception confirmed" (not a defect). **Result:**

12. **Crowded row and outcome area.** Make a rule with Continue, a style, a grammar warning and a
    move across Continue; produce several long messages (Apply on a changed-on-disk file, a
    sync failure, an unknown style).
    **Expected:** the label keeps at least 140px; extra flags fold into `+N` whose tooltip lists
    them all; the outcome area never grows beyond 96px (it scrolls) and APPLY / REVERT / RELOAD
    and the economy band stay visible; the status block (root, path, state, stale indicator)
    sits between the FILTER heading and the search box, with the stale indicator in readable
    14px text. **Result:**

13. **In-game reload.** On the copy, Apply an edited filter, then point the game at the copy (or,
    once satisfied, repeat on the real folder after backing it up). In game: Options → Game →
    Item Filter → Refresh.
    **Expected:** the filter loads with no errors and an edited rule behaves as set (colour, beam,
    minimap icon) on a dropped item. **Result:**
