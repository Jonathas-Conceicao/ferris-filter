# Phase 25 - UAT Feedback Fixes 2: Manual Verification

**Purpose:** The author's hands-on recheck of the 10 UAT2 items; requirements stay "UAT pending" until this is run.

**Before you start:**
- Build: `CARGO_TARGET_DIR=C:/Users/jonat/AppData/Local/Temp/ff-target cargo build --release --bin ff`
- PoE2 in windowed mode.
- Point `FERRISFILTER_HOME` and `USERPROFILE` at a temp copy before any Apply (Apply now writes `styles.json`/`tiers.json` too, from either tab), so the committed files are not rewritten.
- Use a test filter containing a `Sockets >= 5GGG` line, an `Exceptional True` line, a `Minimal` block and a `# ff:style S` rule.

1. **UAT2-01** - expand a rule - Expected: Continue to later rules is the last line of the body, above Duplicate / Delete - Result: ____
2. **UAT2-02** - Custom rule with actions - Expected: the add button reads "+ style" and opens the same picker as before - Result: ____
3. **UAT2-03** - pick a preset, then back to Custom - Expected: Show/Hide disappears for the preset, reappears below the picker for Custom; the Minimal block shows Minimal there - Result: ____
4. **UAT2-04** - type a name then clear it, Apply - Expected: no `# ff:name` line after clearing; an unnamed header shows only the index and Show/Hide - Result: ____
5. **UAT2-05** - Sockets condition - Expected: 0-6 slider, no "+ socket"; changing the 5GGG count to 3 makes the line read `Sockets >= 3GGG` - Result: ____
6. **UAT2-06** - the `Exceptional True` line - Expected: raw row, not in the condition picker, Apply succeeds - Result: ____
7. **UAT2-07** - CONFIG, add a style - Expected: no "unapplied changes" line at the top and no "new" tag - Result: ____
8. **UAT2-08** - CONFIG Styles, with and without custom styles - Expected: Add style sits on the Custom heading both times - Result: ____
9. **UAT2-09** - styles table - Expected: Visibility | Name | Preview header aligned for both groups; custom arrows reorder (ends disabled); built-ins have no arrows or delete; the order survives Apply in `styles.json` - Result: ____
10. **UAT2-10** - edit a tier cutoff and a rule, press APPLY on CONFIG - Expected: `styles.json`, `tiers.json` and the filter written together and the game reloads; then edit `tiers.json` by hand and Apply: the refusal names it, OVERWRITE writes all, RELOAD discards only config edits; RULES Revert leaves config edits - Result: ____
