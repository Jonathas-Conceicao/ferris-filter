# Phase 26 - UAT Feedback Fixes 3: Manual Verification

**Purpose:** The author's hands-on recheck of the 8 UAT3 items; requirements stay "UAT pending" until this is run.

**Before you start:**
- Build: `CARGO_TARGET_DIR=C:/Users/jonat/AppData/Local/Temp/ff-target cargo build --release --bin ff`
- PoE2 in windowed mode.
- Point `FERRISFILTER_HOME` and `USERPROFILE` at a temp copy before any Apply (Apply writes `styles.json` too, including load-assigned ids), so the committed files are not rewritten.
- Use a test filter containing a legacy `# ff:style kite` rule, a `BaseDefencePercentile 50` line, a `BaseArmour >= 100` line, an `ItemLevel > 60` + `ItemLevel <= 80` pair, and a rule with two `ItemLevel >=` lines.

1. **UAT3-01** - CONFIG, Styles - Expected: one table per group (Built-in, Custom); Custom has an Order column holding the arrows - Result: ____
2. **UAT3-02** - Apply, then rename kite in CONFIG - Expected: `styles.json` has ids (`builtin:*` for built-ins); the kite rule stays linked and its tag becomes `<id> <newname>`; built-ins are not renamable; the legacy tag is rewritten - Result: ____
3. **UAT3-03** - add a style in CONFIG, do not Apply, open RULES - Expected: the style is in the rule style picker - Result: ____
4. **UAT3-04** - expand a rule, add Class, click the first class - Expected: rule and class group stay open, and the operator shows before any value is picked - Result: ____
5. **UAT3-05** - add Defences, toggle Armour/Evasion/Energy Shield, check the `BaseDefencePercentile` line, Apply - Expected: one Defences card with working toggles; the percentile line is a raw row; Apply succeeds - Result: ____
6. **UAT3-06** - open the picker with conditions present; pick Between on the `ItemLevel` pair - Expected: present conditions are greyed and focus the existing card; the pair is one Between card with bytes unchanged until edited; the two `>=` lines stay separate - Result: ____
7. **UAT3-07** - open the + condition picker - Expected: every section is collapsed - Result: ____
8. **UAT3-08** - tray Quit with unapplied edits, then with none - Expected: Apply & quit / Quit without applying / Cancel (Apply & quit quits only on success); with nothing pending Quit exits at once - Result: ____
