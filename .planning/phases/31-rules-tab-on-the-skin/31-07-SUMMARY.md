---
phase: 31-rules-tab-on-the-skin
plan: 07
subsystem: overlay
tags: [screenshots, docs, verification, skin]
requires: [31-01, 31-03, 31-05, 31-06]
provides:
  - "--expand <n> / --search <text> in the debug screenshot mode"
  - "RULES tab screenshots under shots/"
  - "CLAUDE.md and CHANGELOG describing the skinned RULES tab"
  - "31-CAPABILITY-CHECKLIST for human verification"
affects: [src/overlay/screenshot.rs, src/overlay/app.rs, src/skin/icons.rs, src/overlay/body.rs, CLAUDE.md, CHANGELOG.md]
key-files:
  created:
    - .planning/phases/31-rules-tab-on-the-skin/shots/*.png (11)
  modified:
    - src/overlay/screenshot.rs
    - src/overlay/app.rs
    - src/skin/icons.rs
    - src/overlay/body.rs
    - CLAUDE.md
    - CHANGELOG.md
metrics:
  completed: 2026-10-16
  tasks: 3
requirements-completed: [RULES-01, RULES-02, RULES-03]
---

# Phase 31 Plan 07: Prove and document the RULES tab Summary

Debug screenshot mode gained `--expand <n>` and `--search <text>`, eleven RULES shots were rendered and viewed, two small rendering defects found in them were fixed, and CLAUDE.md / CHANGELOG now describe the skinned list and editor.

## Commits

- `f43da63` feat: `--expand` / `--search` (debug-only, parsed in `screenshot.rs`, seeded in `new_screenshot`; pure helper `nth_row_id`, tests `expand_seeds_the_nth_row` and extended flag/bad-input tests)
- `2ec1edc` fix: group-broken icon no longer reads "0/0"
- `047a08b` fix: operator dropdown fits "Matches exactly"; "No actions" aligned to the control column
- `406bdce` docs: the 11 screenshots
- `f849e29` docs: CLAUDE.md overlay notes (+ Add rule inserts at the top of the file, skinned list/editor, new skin helpers, `--expand`/`--search` usage) and CHANGELOG `[Unreleased]` (Changed + Fixed)

## Screenshots (all viewed with the Read tool)

Sample: a 13-rule hand-written head (long styled+Continue rule, grammar error, warning, unknown style, broken group, disabled-invalid, numeric, Between+Yes/No, Defences, Simple Defences group, 12-name quoted list, raw row, Custom actions) followed by the earlier flag sample and the seed fixture (102 rules). Large: `tests/fixtures/parser/filterblade.filter` (511 rules). Temp home `%TEMP%\ff-shot-31`.

- `rules.png` - Header, four stone tabs, toolbar with search (`Search 102 rules, classes, base types...`) and a visible gap before **+ Add rule**. Fixed two-line rows: round check, index, Show, name, muted summary line. Row 1 shows the styled chip (diamond + `A`) with a clear gap before the gold CONTINUE word and the long name ellipsised. Flags are legible: red error disc, amber warning triangle, amber `?` ring, the group-broken icon (now two links with a gap and sparks, no longer "0/0"), amber padlock on the disabled-invalid row. Game scrollbar at right, `No economy section` footer, status line red (`Apply refused...`, cut with an ellipsis) with Revert / Reload / Apply.
- `rules-search.png` - `ring` typed: Clear affordance, matching rules only (including unnamed ones and Rings/Amulets/Belts), reorder arrows greyed while searching, Continue word shown on its row.
- `expanded-numeric.png` - Name field, **+ condition** button, Item Level card with `At least` dropdown, slider plus number box (82), STYLE `Custom` picker + `+ style`, Show | Hide switch, Continue round check, Duplicate / red Delete buttons.
- `expanded-between-bool.png` - Between card: Min / Max sliders with numeric boxes; Corrupted as a Yes | No switch (No selected); X remove buttons right-aligned.
- `expanded-defences.png` - Class card with collapsed groups (Armour (1)), Defences card with an Armour toggle and operator dropdown (the card is cut by the viewport; scroll in app).
- `expanded-simple-defences.png` - the seven chips on two wrapped lines, Full AR and Full EV selected (the group of two blocks loads as one rule, summary `Defences: Full AR, Full EV`).
- `expanded-quoted-list.png` - BaseType card, `Matches exactly` dropdown now fits, vertical list with per-entry X and an inner game scrollbar.
- `expanded-raw.png` - raw row `FooBarKeyword 12` in a text field with X; flagged warning icon on the header; `No actions` now sits under the control column.
- `expanded-custom-actions.png` - Custom style picker, Show | Hide, first action card (Set Font Size slider + number) visible.
- `popup-delete-rule.png` - dimmed window, `Delete this rule?` skin popup naming the rule, red `Delete` and `Keep` buttons.
- `rules-large.png` - 511-rule sample: `Search 511 rules...`, rows 1-13 with unnamed `Show` headers and summaries (`BaseType: Gold`, `BaseType: 23 items`), scrollbar thumb small, filter-out-of-date status in amber.

## Deviations from Plan

**1. [Rule 1 - Bug] Group-broken icon read as "0/0"** - tall rounded boxes with a slash. Redrew as two wide short links with spark ticks; gap test kept. `src/skin/icons.rs` (2ec1edc).
**2. [Rule 1 - Bug] Operator dropdown clipped "Matches exactly"** (label overlapped the arrow). `OP_W` 104 to 140 in `src/overlay/body.rs` (047a08b).
**3. [Rule 1 - Bug] "No actions" sat at the left edge** instead of the control column; indented by label column width (047a08b).

## Observations (not fixed)

- Expanded shots are cut at the viewport bottom (Defences card, custom action cards); that is scrolling, not a defect, but a taller shot or a scroll flag would show more.
- The status line in the sample shots reads the grammar-gate refusal (the sample intentionally has two errors).
- The operator dropdown at 140 is tight for "Matches exactly"; a longer localised label would need more.

## Verification

Full `cargo test` green (1178 lib tests plus integration), `cargo fmt --all --check` clean, clippy `^warning` count 126 (unchanged), `tests/fixtures` unchanged. Greps: `top of the file` in CLAUDE.md, `--expand`, `ff_scroll_area_viewport`, `RULES tab on the skin (Phase 31)` in CHANGELOG all present; the old "appends ... before the economy section" sentence is gone.

## 31-CAPABILITY-CHECKLIST

Run `ff` with `FERRISFILTER_HOME` and `USERPROFILE` pointing into a temp copy (styles-default.json, tiers-default.json, config.json, prices) and a sample filter in the game folder under that profile.

- [ ] Rule rows: rows list one per rule, round check enables/disables (disabled dims), arrows reorder (disabled while searching), click expands one row at a time.
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
- [ ] Custom keeps actions: switch a preset back to Custom keeps the current actions and drops the tag.
- [ ] Show/Hide on Custom only: switch on its own line below the picker; Minimal blocks shown as Minimal and left alone.
- [ ] Continue: round check, last line of the body; the Continue word appears in the collapsed row.
- [ ] Duplicate: creates a copy right below (new group id for a group); Delete asks via the popup (Delete / Keep).
- [ ] + Add rule: inserts an empty `Show` as the first block of the file (above the first rule's comments), expands it with Name focused, clears search, shows `Rule added at the top of the file. Press Apply to save it.`.
- [ ] Flag budget: flags beyond 120 px fold into `+N`; errors never fold away; the styled chip sits left of Continue with a gap; the rule name keeps at least 140 px.
- [ ] Empty states: `No rules yet`, `No rules match "<query>"`, `The filter could not be loaded.` (heading only).
- [ ] Apply gate: with an enabled grammar error Apply is refused; clicking the status message jumps to the first such rule.
- [ ] Colour actions: SetTextColor / Border / Background still use the legacy colour picker (Phase 33 restyles it).
- [ ] 500-block sample: open the FilterBlade sample (511 rules) and scroll the list; it stays responsive and the scrollbar tracks.
- [ ] Byte-for-byte round trip: open an untouched filter, press Apply; the file on disk is unchanged (apart from the first-write `.ffbak`).

## Self-Check: PASSED

Commits f43da63, 2ec1edc, 047a08b, 406bdce, f849e29 exist; 11 PNGs under `shots/`; tests, fmt, clippy and fixtures verified.
