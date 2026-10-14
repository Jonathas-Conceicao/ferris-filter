# Phase 24 - UAT Feedback Fixes: Manual Verification

**Purpose:** The author's hands-on recheck of the 13 UAT items; requirements stay "UAT pending" until this is run.

**Before you start:**
- Build: `CARGO_TARGET_DIR=C:/Users/jonat/AppData/Local/Temp/ff-target cargo build --release --bin ff`
- PoE2 in windowed mode.
- Point `FERRISFILTER_HOME` and `USERPROFILE` at a temp copy before any CONFIG Apply (CLAUDE.md development warning), so the committed `styles.json`/`tiers.json` are not rewritten.
- Use a test filter containing a `LinkedSockets 6` line, a `Minimal` block and a `Corrupted != True` line.

1. **UAT-01** - game closed, RULES Apply - Expected: status is just "Wrote <path>." - Result: ____
2. **UAT-02** - CONFIG Styles - Expected: Built-in heading, divider, Custom heading, no built-in badge, delete only on custom - Result: ____
3. **UAT-03** - flip a tier style's switch off, CONFIG Apply, RULES Apply - Expected: those styled rules and economy blocks become Hide - Result: ____
4. **UAT-04** - rule Show/Hide control - Expected: only Show and Hide offered; the Minimal block reads Minimal and its text is unchanged after Apply - Result: ____
5. **UAT-05** - expand several rules with conditions - Expected: condition X buttons line up at the right edge across rules - Result: ____
6. **UAT-06** - BaseType condition - Expected: vertical list, long list scrolls at about 7 rows, pasting `"Heavy Belt" "Utility Belt"` adds two entries - Result: ____
7. **UAT-07** - Class condition - Expected: only the grouped picker; an unknown class stays and is removable - Result: ____
8. **UAT-08** - style picker - Expected: no caption beneath it; an unknown style shows a warning tooltip - Result: ____
9. **UAT-09** - the `LinkedSockets 6` line - Expected: raw row, not in the condition picker, Apply succeeds - Result: ____
10. **UAT-10** - add GemLevel - Expected: starts at 20 and the slider knob is inside its track - Result: ____
11. **UAT-11** - rule body STYLE section - Expected: last in the body; Custom shows actions, a preset hides them, preset to Custom keeps the actions and drops the tag - Result: ____
12. **UAT-12** - Corrupted condition - Expected: shows `Yes | No`; the `!= True` line reads No and is unchanged until clicked - Result: ____
13. **UAT-13** - collapsed rows - Expected: condition summary under the header, hidden when expanded; scrolling stays smooth on a ~460-block filter - Result: ____
