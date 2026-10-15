# Phase 27 - UAT Feedback Fixes 4: Manual Verification

**Purpose:** The author's hands-on recheck of the 3 UAT4 items; requirements stay "UAT pending" until this is run.

**Before you start:**
- Build: `CARGO_TARGET_DIR=C:/Users/jonat/AppData/Local/Temp/ff-target cargo build --release --bin ff`
- PoE2 in windowed mode.
- Point `FERRISFILTER_HOME` and `USERPROFILE` at a temp copy before any Apply (Apply writes `styles.json` too), so the committed files are not rewritten.
- Use a test filter containing a rule with `BaseArmour >= 1` + `BaseEvasion == 0`, a rule with `BaseArmour >= 100`, and a rule with no defences; keep a custom style in `styles.json`.

1. **UAT4-01** - CONFIG, Styles, Custom table - Expected: the Order header and the up/down arrows sit at the table's right edge like the RULES arrows; Built-in has no Order column; arrows still reorder - Result: pass
2. **UAT4-02** - open a minimap colour, a beam colour and a CONFIG style editor colour dropdown and scroll them - Expected: the scroll bar sits right of every colour name, like the minimap shape dropdown - Result: pass
3. **UAT4-03** - on the rule without defences add Simple Defences, set AR Has / EV No, check the strip, open the picker; open the zero-based rule and the `>= 100` rule; on a rule pick Defences and toggle Armour on; Apply - Expected: Simple card writes `BaseArmour > 0` and `BaseEvasion 0`; strip lights AR and AR+ES only; both picker entries greyed and clicking focuses the card; the zero-based rule opens as Simple Defences with its lines unchanged until edited; the `>= 100` rule opens as Defences; the Defences card stays Defences after toggling and after Apply; Apply succeeds - Result: pass

Results recorded at the v0.5 wrap-up (2026-10-15) from the author's report that Phases 24-27 passed UAT.
