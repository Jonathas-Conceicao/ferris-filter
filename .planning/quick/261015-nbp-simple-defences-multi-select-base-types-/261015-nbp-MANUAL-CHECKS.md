# Quick 261015-nbp: manual checks (Simple Defences chips + `# ff:group`)

Run against a scratch copy (set `FERRISFILTER_HOME` and `USERPROFILE` to a temp folder holding a
copy of the game folder) so nothing real is overwritten.

1. **Chips render.** Expand a rule, `+ condition` → Simple Defences. One line of seven chips
   (Full AR, Full EV, Full ES, AR/EV, AR/ES, EV/ES, AR/EV/ES) wraps when the stripe is narrow; hovering
   a chip names the base type. Nothing is written until a chip is clicked.
2. **One block.** Pick Full AR, Full ES, AR/ES ("no Evasion"): Apply, open the file: one block with
   `BaseEvasion 0`, no `# ff:group` line.
3. **Group.** Pick Full AR + Full EV + Full ES: still ONE row in the list (same index, rule count
   unchanged), summary reads `Defences: Full AR, Full EV, Full ES`. Apply: the file has three
   consecutive identical blocks, each with `# ff:group <id> 1/3` .. `3/3` next to `# ff:name`.
   Add AR/EV/ES: four blocks. Deselect back to one chip: one block, group line gone.
4. **Edits follow the group.** On a grouped rule change the name, a condition, the style preset,
   Show/Hide, Continue, disable/enable; Apply and confirm every block changed the same way.
5. **Move / duplicate / delete.** Arrows move the whole group past single rules and past another group;
   Duplicate puts a copy (new group id, name `(copy)`) right after it; Delete removes all its blocks.
6. **Broken group.** Hand-edit one block of a group in the file (e.g. its font size), RELOAD: the
   blocks show as separate rules flagged `group broken` (tooltip explains); Apply works and leaves them
   as written.
7. **Byte identity.** Load a filter containing a group, change an unrelated rule, Apply: the group's
   bytes are unchanged (diff the file).
8. **All / none.** Select all seven chips: no defence line is written, the card stays open with every
   chip lit; deselecting every chip also writes nothing and keeps the empty card. The card's X removes
   every defence line (a group collapses to one block).
9. **In game.** Load the filter in PoE2 and check a grouped rule highlights exactly the picked base
   types (e.g. pure AR/EV/ES body armours, no hybrids).

**Result (2026-10-15):** general usability tested by the author (chip clicks, multi-block rules,
tooltips) and working; all checks accepted as pass without a full sweep. Open: review CR-01 (chips
built only from "no X" lines also match items without defences unless the rule has a Class
condition); the strict alternative was offered and not yet chosen.
