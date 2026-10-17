---
phase: 31-rules-tab-on-the-skin
verified: 2026-10-16T00:00:00Z
status: human_needed
score: 3/3 roadmap truths verified at code level (RULES-01, RULES-02, RULES-03); interactive and visual checks remain
overrides_applied: 0
re_verification:
  previous_status: none
gaps: []
warnings:
  - "WR-02 (accepted, author decision): + Add rule inserts a bare `Show` as the first block, so until conditions are added it matches every item and shadows the whole filter if applied. RULES-03 mandates top placement; mitigations were declined (UI-not-texty)."
  - "Re-review remaining warning: the log-slider Left/Right arrow handling (WR-04 fix, src/overlay/widgets.rs) lowers/raises an out-of-range hand-written value by clamping on the first key press. Only affects values outside the slider range; accepted."
human_verification:
  - test: "Run 31-CAPABILITY-CHECKLIST (section below) against `ff` with FERRISFILTER_HOME and USERPROFILE in a temp copy"
    expected: "Every item behaves as listed"
    why_human: "Interactive editing, hover, keyboard, focus, scrolling feel and real game cannot be proven from stills or unit tests"
  - test: "Less-texty changes (itemised in the report body)"
    expected: "Tooltips and icons present and legible"
    why_human: "Hover tooltips and icon legibility are visual/interactive"
---

# Phase 31: Rules Tab on the Skin Verification Report

**Phase Goal:** The rule list and the expanded rule editor are drawn entirely with the skin, keep every v0.5 editing capability, and new rules land at the top of the file.
**Status:** human_needed (no FAILED truths, no blockers)
**Re-verification:** No (initial). Review fixes (WR-01, 03-08) were verified in their current state.

## Goal Achievement

| # | Truth (ROADMAP SC) | Status | Evidence |
|---|--------------------|--------|----------|
| 1 | Rows (round check, Show/Hide, name, summary, flags, arrows), search and + Add rule drawn with skin widgets; ~500-block sample responsive | VERIFIED (feel pending) | `src/overlay/rule_list.rs` (1752 lines) uses skin widgets/icons (ff_* / skin:: references) and virtualizes with `ScrollArea::show_viewport` over a cached offset table (`virtual_list.rs`). Tests `only_visible_rows_are_drawn_for_511_rules` and `painted_row_heights_match_the_virtual_list` pass. `shots/rules.png` shows skin rows (round checks, index, Show, name, summary line, flag icons, reorder arrows, game scrollbar), skin search and `+ Add rule`; `shots/rules-large.png` shows the 511-rule FilterBlade sample. |
| 2 | Every v0.5 editing path works through skin controls; untouched filter round-trips byte-for-byte | VERIFIED (interactive pending) | Editor in `src/overlay/body.rs` (6171 lines, skin-based cards); shots show numeric, Between + Yes/No, Defences, Simple Defences chips, quoted list, raw row, custom actions, style picker, Continue, Duplicate/Delete, delete popup. Full `cargo test` run by verifier: 1184 lib tests pass, all integration suites pass (including `tests/filter_roundtrip.rs` filterblade round trip and the group byte-identity test). No editing logic was removed in `rules.rs` (2704 lines). |
| 3 | + Add rule inserts the new empty rule as the first block of the file (ahead of any divider), not before the economy section | VERIFIED | `RowAction::AddRule` -> `doc.insert_rule_at_top(vec!["Show"])` (`src/overlay/rules.rs:1097`, `src/filter/structure.rs:207`). Tests: `add_rule_inserts_at_top_of_file` (row label `1  Show`, existing rows shift, economy still Present), `add_rule_above_a_group_keeps_the_group_one_rule`, and `insert_top_*` (basic, meta, preamble, disabled block, economy after rules, economy only, group run, no blocks, CRLF). Status text `RULE_ADDED_MESSAGE` = "Rule added at the top of the file. Press Apply to save it." (`apply.rs:217`). Dividers belong to Phase SECT; no divider code exists yet, so "ahead of any divider" holds by construction (first block of file). |

**Score:** 3/3

### Requirements Coverage

| Requirement | Source Plans | Status | Evidence |
|-------------|--------------|--------|----------|
| RULES-01 | 31-02, 31-03, 31-05, 31-07 | SATISFIED | Truth 1; REQUIREMENTS.md marks Complete |
| RULES-02 | 31-02, 31-04, 31-05, 31-06, 31-07 | SATISFIED | Truth 2 |
| RULES-03 | 31-01, 31-07 | SATISFIED | Truth 3 |

All three IDs declared in PLAN frontmatter appear in REQUIREMENTS.md (lines 54-62, traceability 182-184). No orphaned requirements for Phase 31.

### Behavioral Spot-Checks

| Check | Command | Result | Status |
|-------|---------|--------|--------|
| Full suite | `cargo test` (CARGO_TARGET_DIR in temp) | 1184 lib + integration all ok, 0 failed | PASS |
| Fixtures untouched | `git status --short tests/fixtures` | empty | PASS |
| No tracked modifications | `git status --short \| grep -v '^??'` | empty | PASS |

### Anti-Patterns

No `TBD`/`FIXME`/`XXX` in the phase's main files (rule_list.rs, body.rs, widgets.rs, skin/*.rs, filter/structure.rs). No debt-marker blockers.

### Review Status

31-REVIEW-FIX: 7 of 8 warnings fixed (WR-01, 03, 04, 05, 06, 07, 08); WR-02 skipped as an author decision. Re-review: one remaining warning (log-slider arrow on an out-of-range hand-written value). Both recorded above as accepted, not blockers.

## Human Verification Required

### A. 31-CAPABILITY-CHECKLIST (from 31-07-SUMMARY.md)

Run `ff` with `FERRISFILTER_HOME` and `USERPROFILE` pointing into a temp copy (styles-default.json, tiers-default.json, config.json, prices) and a sample filter in the game folder under that profile.

- [ ] Rule rows: one row per rule, round check enables/disables (disabled dims), arrows reorder (disabled while searching), click expands one row at a time.
- [ ] Conditions: **+ condition** picker is grouped and searchable, sections start collapsed, an added condition greys out and focuses the existing card.
- [ ] Actions: **+ style** picker adds an action card; X removes it; each action editable (font size, colours, sound, minimap, beam).
- [ ] Operators: dropdown with plain-English labels; an existing operator (e.g. `!=`) is kept on edit.
- [ ] Numeric: slider and exact number stay paired (ItemLevel).
- [ ] Between: choose Between on a numeric; Min never exceeds Max; a loaded `>=`/`<=` pair shows as one card with bytes kept until edited.
- [ ] Defences: Armour / Evasion / Energy Shield toggles each with operator and slider; BaseDefencePercentile loads as raw row.
- [ ] Simple Defences: seven chips; selection writes the minimum number of blocks; groups carry `# ff:group`; X removes all defence lines.
- [ ] Group behaviour: group edits (enable, delete, duplicate with a new id, move) act on the whole group; a broken run is flagged `group broken` and left as written.
- [ ] Class picker: grouped, counts per group, classes outside config groups listed on one line.
- [ ] Quoted lists: type + Enter and + ADD, a pasted quoted list splits into entries, per-entry X, list scrolls at about 7 rows.
- [ ] Raw rows: unknown keyword line shown in a text field; Enter commits; an edit that adds a grammar Error shows an inline hint and changes nothing.
- [ ] Sockets: numeric 0-6 slider; a hand-written `Sockets >= 5GGG` keeps its letters when the count changes.
- [ ] Booleans: Yes | No switch, no operator; remove the condition to match either.
- [ ] Pickers: colour picker (old look, alpha), minimap, beam and sound controls work.
- [ ] Style picker: Custom + presets; preset writes `# ff:style <id> <name>` and hides actions; unknown style flagged `unknown style` with tooltip.
- [ ] Custom keeps actions: switching a preset back to Custom keeps the current actions and drops the tag.
- [ ] Show/Hide on Custom only: switch on its own line below the picker; Minimal blocks shown as Minimal and left alone.
- [ ] Continue: round check, last line of the body; the Continue word appears in the collapsed row.
- [ ] Duplicate creates a copy right below (new group id for a group); Delete asks via the popup (Delete / Keep).
- [ ] + Add rule: inserts an empty `Show` as the first block of the file (above the first rule's comments), expands it with Name focused, clears search, shows `Rule added at the top of the file. Press Apply to save it.`.
- [ ] Flag budget: flags beyond 120 px fold into `+N`; errors never fold away; the styled chip sits left of Continue with a gap; the rule name keeps at least 140 px.
- [ ] Empty states: `No rules yet`, `No rules match "<query>"`, `The filter could not be loaded.` (heading only).
- [ ] Apply gate: with an enabled grammar error Apply is refused; clicking the status message jumps to the first such rule.
- [ ] Colour actions: SetTextColor / Border / Background still use the legacy colour picker (Phase 33 restyles it).
- [ ] 500-block sample: open the FilterBlade sample (511 rules) and scroll; it stays responsive and the scrollbar tracks.
- [ ] Byte-for-byte round trip: open an untouched filter, press Apply; the file on disk is unchanged (apart from the first-write `.ffbak`).

### B. Less-texty changes made in this phase

- [ ] Captions moved to tooltips: hover each control that formerly had a standing caption (e.g. unknown-style picker, Class/Defences explanations, list add fields) and confirm the explanation appears and no wall of text remains in the editor.
- [ ] Flag icon changes: error red disc with !, warning amber triangle, unknown style amber ring ?, group broken two-link icon (no longer reads "0/0"), padlock for can't-enable, moved-across-Continue arrows; confirm each is legible at 100% and at 150-200% DPI and shows a tooltip.
- [ ] Add-rule tooltip / status text: hovering **+ Add rule** explains top placement; clicking shows `Rule added at the top of the file. Press Apply to save it.`.
- [ ] Add-field hints read "Type a base type, press Enter" / "Type a mod, press Enter" / "Type a name, press Enter" / "Type a whole number, press Enter".
- [ ] Arrow click target (24x22) feels right and clicking the blank area under the arrows still expands the row.
- [ ] Number fields show 4-digit values; log sliders respond to Left/Right.
- [ ] Long dropdown option lists scroll inside the window.

### Known accepted items

1. WR-02: a freshly added bare `Show` at the top matches every item until conditions are added (author decision, RULES-03).
2. Re-review warning: log-slider arrow key on an out-of-range hand-written value clamps it into range.

## Gaps Summary

No gaps. All three roadmap truths and all three requirement IDs are supported by code, passing tests (1184 lib + integration) and viewed screenshots. Status is human_needed only because interactive, hover, keyboard, DPI and in-game behaviour need the author's hands-on check.

---

_Verified: 2026-10-16_
_Verifier: Claude (gsd-verifier)_
