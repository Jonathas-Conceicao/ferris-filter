---
phase: 30-skinned-overlay-shell
plan: 05
subsystem: ui
tags: [docs, screenshots, verification-inventory, overlay]
requires: [30-01, 30-02, 30-03, 30-04]
provides:
  - CLAUDE.md overlay notes rewritten for the docked skinned window, status line, popups, four tabs and the debug screenshot mode
  - CHANGELOG [Unreleased] entry for the shell, the dev-profile speedup and the docking fix
  - Eight viewed screenshots under shots/
  - The 30-LESS-TEXTY-INVENTORY (itemised human-verification checklist)
affects: [phase-30-verification]
tech-stack:
  added: []
  patterns: [temp-home screenshot runs, inventory sourced from code and prior summaries]
key-files:
  created:
    - .planning/phases/30-skinned-overlay-shell/shots/rules.png
    - .planning/phases/30-skinned-overlay-shell/shots/config.png
    - .planning/phases/30-skinned-overlay-shell/shots/economy.png
    - .planning/phases/30-skinned-overlay-shell/shots/about.png
    - .planning/phases/30-skinned-overlay-shell/shots/popup-quit.png
    - .planning/phases/30-skinned-overlay-shell/shots/popup-changed.png
    - .planning/phases/30-skinned-overlay-shell/shots/popup-delete-rule.png
    - .planning/phases/30-skinned-overlay-shell/shots/status-truncated.png
  modified: [CLAUDE.md, CHANGELOG.md]
decisions:
  - "README needed no change: it never described the stripe, the outcome area or all-caps buttons (grep for stripe/outcome returns nothing)"
  - "The RULES screenshot uses a hand-written sample filter in a temp home (not the committed fixture alone) so every flag icon that a static file can produce is visible"
metrics:
  tasks: 3
  completed: 2026-10-16
---

# Phase 30 Plan 05: Docs, screenshots and the less-texty inventory Summary

CLAUDE.md, CHANGELOG and the screenshot set now describe the shell as shipped, and this file carries the item-by-item `30-LESS-TEXTY-INVENTORY` for VERIFICATION.md. No source code changed in this plan (the renders showed no defect that was clearly in scope).

## Commits
- e1c25a9 docs(30-05): document the skinned overlay shell (CLAUDE.md, CHANGELOG.md)
- 4e257c7 docs(30-05): final screenshot set of the skinned shell (eight PNGs)
- (this SUMMARY and the state files are committed separately)

## What changed in the docs
- **CLAUDE.md:** `ff` row (docked 660pt window on the game's monitor, re-dock on every summon, DPI-correct, the four tabs, the debug `--screenshot` mode and its temp-folder guard), `src/skin/` row (the overlay now uses it; `overlay::theme` re-pointed until Phase 35), every "stripe" that named the UI is now "window", the Rules notes (flag icons, `styled` diamond with a 10-character name, search hint with the rule count and **+ Add rule**, heading-only empty states), a new **Bottom bar and status line** paragraph (replaces the 96px outcome area), a new **Popups** paragraph (the seven), the tabs paragraph (Rules | Config | Economy | About, Sync economy on Economy), mixed-case button names. Greps: `capped at 96px` 0, `outcome area` 0, `--screenshot` 2, `status line` 4.
- **CHANGELOG.md `[Unreleased]`:** `### Changed` gains **Skinned overlay shell (Phase 30)** and **Faster dev and test builds** (`[profile.dev.package."*"] opt-level = 2`, commit 01f5b9e, `cargo test --lib` from about 1000 s to about 57 s); new `### Fixed` records the docking monitor/units bug and the Visibility/Name overlap.
- **README.md:** unchanged (no stripe, outcome area or all-caps button text to rewrite).

## Screenshots (all viewed with the Read tool)

Rendered by the debug `ff --screenshot` mode with `FERRISFILTER_HOME` and `USERPROFILE` under `%TEMP%\ff-shot-30` (home = copies of `styles-default.json`, `tiers-default.json`, `config.json`, and the seed prices as `prices.json`; `sample.filter` = 9 hand-written rules plus the seed fixture, edited only in the temp copy). Nothing outside the temp folder was read or written; no keys were sent; the user's `ff` was not touched. Each process exited by itself.

- viewed: `shots/rules.png` -- 660pt-wide window at full height, header plate "FerrisFilter" readable, four stone tabs with Rules selected and bright, search row with `Search 89 rules, classes, base types…` and **+ Add rule**, Ferris faint behind the rows, economy footer (`No economy section` / Add economy section), bottom bar with Revert / Reload / Apply and a red status line (`Apply refused: 1 enabled rule has a…`). Icons visible and correct: gold diamond + `S` (styled, rule 1), red filled `!` (grammar error, rule 2), amber triangle (grammar warning, rule 3), amber ring with `?` (unknown style, rule 4), gold CONTINUE word (rules 5, 7), broken-link glyph (group broken, rule 6), amber padlock (disabled, cannot enable, rule 8). The long name on rule 7 is cut with … and keeps room for diamond, `A` and CONTINUE. No horizontal overflow. Not renderable statically: the **moved across Continue** icon (it only appears after a move in the running overlay); covered by unit tests and listed in the checklist.
- viewed: `shots/config.png` -- Filter / Styles section headings with the gold rule, filter dropdown and the Reload-in-game toggle with no caption lines, Built-in table with Visibility / Name / Preview readable (the 30-04 overlap fix holds), Custom heading with **+ Add style** and the Order column; content inside the frame, scrollbar on the right.
- viewed: `shots/economy.png` -- Economy heading, League `Forbidden Rites`, Last sync `2026-10-16 06:31` (the prices.json mtime), centred **Sync economy** button, Ferris below.
- viewed: `shots/about.png` -- FerrisFilter, `Version 0.5.0`, the one-line description, Credits rows (GGG, exljbris, Karen Rustad Tolva), the two-line disclaimer pinned at the bottom.
- viewed: `shots/popup-quit.png` -- window dimmed, centred game-box popup "Unapplied changes", body "Apply them before quitting?", three buttons on one line: Apply & quit, Quit without applying (red), Cancel.
- viewed: `shots/popup-changed.png` -- "File changed on disk", body names dynamic.filter, buttons Reload, Overwrite (red), Keep editing; behind it the bottom bar shows Revert / Reload / red **Overwrite** in the Apply slot.
- viewed: `shots/popup-delete-rule.png` -- "Delete this rule?", body `Rule 1: Chaos orbs shout`, Delete (red) and Keep, over the expanded row (NAME, CONDITIONS, BaseType editor).
- viewed: `shots/status-truncated.png` -- a ~290-character message in the status slot cut with … on one line (`Wrote C:/Users/jonat/OneDrive/Do…`), Revert / Reload / Apply intact. The full-text hover tooltip cannot be captured in a still; it is in the checklist.

Observations, no source change made (all minor and outside "clearly a bug"):
- The broken-link flag icon reads small at 14pt (looks like two joined rings).
- The status line is cut early when buttons are wide (about 250pt of text); intended by the contract, full text on hover.
- The Changed-on-disk and Switch-filter popup bodies are one wrapped paragraph rather than the two lines in UI-SPEC (a 30-03 choice, text unchanged).

## Verification
- Full `cargo test` green: 1131 unit tests (57.9 s, thanks to the dev-profile change) plus all integration suites; `cargo fmt --all --check` clean; `tests/fixtures` unchanged (`git status` empty).
- Clippy (`cargo clippy -p ferrisfilter --all-targets`, count of `^warning` lines): 126, unchanged (no source touched; 30-04 reported the same figure), 0 errors.
- Acceptance greps: `Skinned overlay shell (Phase 30)` in CHANGELOG 1; `--screenshot` in CLAUDE.md 2; `status line` 4; `capped at 96px` 0; `outcome area` 0; README `stripe` 0; eight PNGs in shots/.

## 30-LESS-TEXTY-INVENTORY

Copy this section into VERIFICATION.md `human_verification`. Source: UI-SPEC "Less Texty Inventory", 30-02/03/04 SUMMARYs and the shipped code (`shell.rs`, `apply.rs`, `rule_list.rs`, `config_view.rs`, `body.rs`, `app.rs`). Run the overlay from a temp copy (`FERRISFILTER_HOME`, `USERPROFILE`) as in the CLAUDE.md development warning.

### A. Captions that became tooltips or short headings

- [ ] A1 Config > Filter, game-folder filter dropdown. Old: standing line "Saved on this computer. Default: dynamic.filter." Now: hover the dropdown; the tooltip's first line is the full filter path, the second line is that sentence (with no game folder it is the only line). No caption under the dropdown.
- [ ] A2 Config > Filter, **Reload filter in game after Apply**. Old: a paragraph under the toggle (sends /reloaditemfilter, game selection, chat box). Now: hover the toggle or its label; the tooltip is that paragraph, a blank line, then the short tip. No paragraph under the toggle.
- [ ] A3 Config > Filter, new-filter name field (draft). Old: a helper line naming the new file. Now: tooltip on the empty field only. A non-empty invalid name still shows an inline hint (kept on purpose).
- [ ] A3b Config > Styles, **+ Add style** name field. Old: always-on helper line. Now: tooltip on the empty "Style name" field; a non-empty invalid name keeps the inline hint. (Shipped beyond the contract.)
- [ ] A4 Config > Styles with no styles loaded (delete `styles.json` and `styles-default.json` from the temp home, or break the file). Old: "No styles loaded. Revert to reload styles.json." Now: inline `No styles loaded` in amber; hover it for "Revert to reload styles.json."
- [ ] A5 Config > a custom style with a custom sound, **File** field. Old: caption "The file must be in the game folder." Now: tooltip on the File text field.
- [ ] A6 Rules with an empty filter. Old: heading plus "Press + ADD RULE to write your first one. Blocks you add are saved with the filter when you press APPLY." Now: heading `No rules yet` only; hover **+ Add rule** for the sentence (mixed-case wording).
- [ ] A7 Rules, search with no hits. Old: heading plus "Check the spelling, or press Clear to see every rule." Now: heading `No rules match "<query>"` only; the field's Clear button is the way out.
- [ ] A8 Rules, a filter that cannot be loaded (invalid UTF-8 file). Old: heading, the error paragraph and "Fix the file and press RELOAD." Now: heading `The filter could not be loaded.` only; the detail is the red status line `The filter could not be loaded: <msg>. Fix the file and press Reload.`
- [ ] A9 Rules, expand a rule with no conditions. Old: sentence "No conditions: this rule matches every item." Now: muted `No conditions`; hover for the sentence.
- [ ] A10 Rules, expand a rule with no actions. Old: "No actions: matching items keep the game's look." Now: muted `No actions`; hover for the sentence.
- [ ] A11 Rules, expand a disabled rule whose lines are not valid grammar. Old: the long "can't enable" paragraph. Now: amber `Can't enable`; hover for the long text.
- [ ] A12 Rules, Apply while a rule has a grammar error. Old tooltip on the disabled Apply: `Fix or disable the rules marked "grammar error" first.` Now: `Fix or disable the rules marked with the red ! first.` (names what the row shows).
- [ ] A13 Filter path. Old: path line in the status block. Now: hover the gold "FerrisFilter" plate in the header; the tooltip is the loaded filter's full path and nothing else.
- [ ] A14 "Economy section found." / "The filter has no economy section." Old: state line in the status block. Now: dropped from the status; the economy footer row (`+ N locked rule blocks (economy)` / `No economy section`) says it.
- [ ] A15 v0.4 notice, absent-filter notice, damaged-markers line. Old: paragraphs in the status block. Now: status line only (see C8, C9, C10).
- [ ] A16 Apply indicator ("Filter out of date..." / "Filter matches saved rules"). Old: line above the buttons on both tabs. Now: the default status line (see C13).
- [ ] A17 Rule count. Old: "N rules (shown of N)" label and a "FILTER" section header. Now: the search box hint reads `Search N rules, classes, base types…`; no label, no header.
- [ ] A18 Economy remove. Old: inline "Remove the economy section? Yes, remove / Keep" with the sentence "Your own rules are kept. Add the section back any time." Now: the Remove economy popup (see D6) carries that sentence.
- [ ] A19 Delete rule, Delete style, Revert on Config, Switch filter. Old: inline confirm frames and the Config footer prompt. Now: popups (D4, D5, D7, D2). No inline confirm frame remains anywhere.
- [ ] A20 Outcome area. Old: a 96px scrolling block of several lines with jump links. Now: one status line (section C). Behaviour change to confirm: only one message at a time; the whole text appears on hover only when the line was cut.
- [ ] A21 Retained on purpose (should still be inline): field-validation hints, `Not set (game default)`, the short field captions, and the two tier helper lines (`Minimum price for each tier...`, `Only your live tiers.json is edited...`).

### B. Flags that became icons (collapsed Rules rows)

Paint order, right to left, starting next to the reorder arrows; each icon is an 18pt hover target and the tooltip appears on hover. Flag words no longer appear in collapsed rows.

- [ ] B1 Grammar error: red filled circle with `!`, nearest the arrows. Tooltip: one `Line N: message` per diagnostic. Expect it on any enabled rule with a bad line (for example `Sockets >= abc`). Never folded into `+N`.
- [ ] B2 Grammar warning: amber outline triangle with `!`. Tooltip: the diagnostics, `Line N: message`. Example: `LinkedSockets >= 4`.
- [ ] B3 Unknown style: amber ring with `?`. Tooltip: "No style with this name exists. This rule keeps its current actions, and its style link is removed on the next Apply." Example: `# ff:style nosuch` above `Show`. The status line also says `1 rule uses an unknown style (first: rule N).` and clicking it jumps to the rule.
- [ ] B4 Group broken: amber broken-link icon (two open ovals with a gap). Tooltip: "Group broken: these blocks were one rule (# ff:group) but no longer match, so each is shown on its own. Left as written until you edit it." Example: a lone `# ff:group abc123 1/2` block.
- [ ] B5 Moved across Continue: amber up/down arrows with a bar. Tooltip: "This move changes which rules still apply after a Continue. Check the order." Reproduce by moving a rule across a rule that has `Continue` (not renderable by the screenshot mode; unit-tested).
- [ ] B6 Continue: stays the gold word `Continue` in small caps. Tooltip: "Matching goes on to the rules below".
- [ ] B7 Styled: gold diamond followed by the style name in muted text, cut at 10 characters plus an ellipsis (was 12 characters, no diamond). Tooltip: `Style "<full name>" sets this rule's actions on Apply`.
- [ ] B8 Can't enable: amber padlock, alone, on a disabled rule whose lines are not valid grammar. Tooltip: "Can't enable: these lines are not valid filter grammar. You can still move or delete it."
- [ ] B9 Right-to-left order next to the arrows reads: error, warning, unknown style, group broken, moved across Continue, Continue, styled. A rule with many flags folds the least important into a muted `+N` badge at the far left of the group (hover lists them as `name: tip`), and the rule label keeps at least 140px.
- [ ] B10 Screen-reader / accessible names: each icon still carries the old flag word as its accessible name (check with a UI inspector if desired; not visible).

### C. Where each former outcome-area, status-block and footer message now appears

All of these are the one status line left of Revert / Reload / Apply. Colours: Info muted, Ok green, Warning amber, Error red. The priority is: root error, event message, grammar gate, load failure / damaged markers, sync running, unknown styles, v0.4 notice, absent filter, indicator.

- [ ] C1 Apply success: `Wrote <path>.` and, with the game running and the setting on, `Reloaded in game`. Green (amber when the text carries the Options > Game > Item Filter > Refresh reminder). Shows until the next edit.
- [ ] C2 Apply written but not reloaded (setting off, a reload in progress, window not ready, game not running reason, `could not reload in game: <cause>`): amber, keeps the Refresh reminder plus the reason.
- [ ] C3 Apply refused, write failed, or a file changed on disk: red, names the file(s); a changed file also opens the File changed on disk popup (D1).
- [ ] C4 Overwrite outcome: same texts as Apply (it counts as an Apply, in-game reload included).
- [ ] C5 Reload outcome (re-read from disk): status message from the reload; Revert message `Reverted to the last applied filter.` (Rules tab Revert) in green/neutral event colour.
- [ ] C6 Sync economy running: `Fetching economy prices…` (Info) while the Sync economy button reads `Syncing…` and is disabled; outcome `Economy prices refreshed from <league>. Press Apply to compile them into the filter.` (green) or `Economy sync failed: <err>` (red). Appears on every tab.
- [ ] C7 List status, for example `Rule added at position N (hidden by the current search)` after **+ Add rule** with a search that hides it, and the style-removed status `Style removed. The actions are now yours to edit.`: event message, replaced by the next edit.
- [ ] C8 Grammar gate: `Apply refused: 1 enabled rule has a grammar error (first: rule N, line M). Fix them or turn them off.` red. Click it: the first blocking rule expands and scrolls into view (the search is cleared if it hides the rule); the pointer is a hand while a target exists. Apply is disabled with the tooltip from A12.
- [ ] C9 Unknown-style summary: `1 rule uses an unknown style (first: rule N).` amber; click jumps to the rule.
- [ ] C10 Damaged economy markers: `The economy section markers are damaged. Fix them by hand, then Reload.` red; the economy footer row label is red with the same tooltip; Apply disabled with this tooltip.
- [ ] C11 Load failure: `The filter could not be loaded: <msg>. Fix the file and press Reload.` red.
- [ ] C12 v0.4 filter: `v0.4 filter: converted on the next Apply (a .ffbak backup is kept).` Info.
- [ ] C13 Absent filter: `The filter does not exist yet. Apply creates it once it has rules.` Info.
- [ ] C14 Apply-state indicator (default): `Filter out of date — press Apply, then reload in-game` amber when unapplied or never applied; `Filter matches saved rules` Info when applied. Same on every tab.
- [ ] C15 Missing app root error: red, highest priority.
- [ ] C16 Config message (for example `Could not save ff-settings.json: <err>` when the settings file is read-only): red, shown as an event message on every tab.
- [ ] C17 Long text: a message longer than the slot is cut with … on one line; hover shows the whole text only then (max width 420), in the same colour. Reproduce with `ff --screenshot x.png --status "<300 characters>"` and compare with `shots/status-truncated.png`, or with a long path in C1.
- [ ] C18 Filter path moved to the header plate tooltip (A13); economy section state moved to the economy footer row (A14); Revert / Reload / Apply tooltips: Reload "Discard your edits and load the file from disk." (disabled: "Nothing to reload."), Overwrite "Replace the file with your edits.", Apply disabled reasons per the chain in `shell::bar_state`.

### D. Popups

One at a time; Escape or a click outside the box picks the last (safe) button; the popup stays open when the window is hidden and summoned again. Button order is left to right. Reproduce each in the running overlay or with `ff --screenshot x.png --popup <name>` (debug build, temp home).

- [ ] D1 `changed` File changed on disk. Trigger: edit the filter file on disk while the overlay has it loaded, then press Apply. Buttons: Reload, Overwrite (red), Keep editing. After Keep editing the Apply slot reads **Overwrite** (red) and reopens this popup.
- [ ] D2 `switch` Switch filter? Trigger: Config > Filter, pick another filter while Rules has unapplied edits. Buttons: Switch and discard (red), Stay.
- [ ] D3 `quit` Unapplied changes. Trigger: tray Quit with anything unapplied. Buttons: Apply & quit, Quit without applying (red, tooltip "Unapplied edits are lost"), Cancel. Apply & quit quits only after a successful Apply and waits for an in-game reload; with nothing pending Quit exits at once.
- [ ] D4 `delete-rule` Delete this rule? Trigger: expand a rule, Delete. Body `Rule N: <name or Show/Hide>` (plus `Deletes all N blocks of this rule.` for a grouped rule). Buttons: Delete (red), Keep.
- [ ] D5 `delete-style` Delete style? Trigger: Config > Styles > a custom style's delete. Body: the rule count and the `config.json` warning when referenced. Buttons: Delete style (red), Keep style.
- [ ] D6 `remove-economy` Remove economy section? Trigger: Remove on the economy footer row. Body: "Your own rules are kept. Add the section back any time." Buttons: Remove (red), Keep.
- [ ] D7 `discard-config` Discard config changes? Trigger: Revert on the Config tab with edits. Body: `Discard N unapplied config changes?` Buttons: Discard (red), Keep editing.
- [ ] D8 Stacking: with a quit request and a changed-on-disk refusal both pending, Quit shows first, then File changed on disk, then the others.
- [ ] D9 Keyboard: Escape = last button (no action taken); Tab cycles the buttons; Enter or Space activates the focused one; no button is focused when the popup opens. Click on the dim area = last button.
- [ ] D10 Hide with Ctrl+Shift+Z while a popup is open, then summon again: the same popup is still there.

### E. Docking, DPI and monitors

- [ ] E1 Launch with the game on the primary monitor at 100% scale: the window is 660pt wide, flush left, full work-area height (above the taskbar), borderless, not movable, always on top.
- [ ] E2 Repeat at 125%, 150% and 200% display scale (Windows Settings > Display; sign out/in if needed): still 660pt wide in logical points (it grows in pixels), full height above the taskbar, no gap or overlap with the screen edge. 125% was the old failure.
- [ ] E3 Game on a secondary monitor (windowed or windowed-fullscreen): the window docks on that monitor's left edge and full work-area height, at that monitor's own scale.
- [ ] E4 Game not running: the window docks on the primary monitor.
- [ ] E5 Re-dock on summon: with the window hidden, move the game window to the other monitor, then press Ctrl+Shift+Z (with the game in front): the window appears on the game's monitor. Repeat via the tray icon click and via launching `ff` a second time.
- [ ] E6 Move the game between monitors of different scale, then summon: the window is correctly sized on the new monitor (no mixed physical/logical sizes); a scale change while shown re-docks once.
- [ ] E7 A taskbar on the left: the window starts right of the taskbar (x = work-area left); a taskbar on top: the window starts below it and ends above any bottom edge.
- [ ] E8 Close button (round red, top right): hides the window, never quits; tooltip "Hide (Ctrl+Shift+Z)". The tray icon's Quit is the only exit.
- [ ] E9 Ctrl+Shift+Z works only while Path of Exile 2 or ff is the foreground window; tray Show/Quit work; a second launch reveals the running window and exits.
- [ ] E10 Apply with "Reload filter in game after Apply" on and the game running sends `/reloaditemfilter` and the status reads `Reloaded in game`; with the game not running it reads `Wrote <path>.`
- [ ] E11 The windowed or windowed-fullscreen requirement is unchanged (exclusive fullscreen hides the window); a blank screen means checking the game's display mode first.
- [ ] E12 Content fits: every Rules and Config row lays out at about 535pt with no horizontal scrolling (check an expanded rule with Class, Rarity and Defences cards, and the Custom styles table).

## Deviations from Plan

**1. README unchanged.** The plan allowed it: no stripe/outcome-area/all-caps text existed there (`grep -i stripe\|outcome README.md` is empty). Only the existing `ff-proto` paragraph mentions the overlay.

**2. The sample filter for the RULES shot is hand-written, not only the committed fixture.** The fixture's rules carry no `# ff:` metadata, so none of the flag icons would show. Metadata lines go above the `Show` header (an early attempt with them inside the block produced no names or tags); a real grammar Error needed a bad value (`Sockets >= abc`), because an unknown keyword is only a warning. The sample lives in the temp folder and is not committed.

**3. No source defects fixed.** The plan's "fix any defect" step found nothing clearly in scope (see the observations above).

**4. Deviations carried from the other four plans (for the verifier):**
- 30-01: none of substance.
- 30-02: the `shell` module was missing from the Task 1 commit (CRLF `mod.rs`, fixed in the Task 2 commit); an interim Discard-config modal was added (folded into the popup system by 30-03); Task 2 went in as one commit.
- 30-03: `--popup delete-rule` also expands the row (the list drops a pending delete on a collapsed row); popup box widened 520 to 580pt so the three-button Quit row fits; some intermediate cargo builds used the repo `target/` (git-ignored).
- 30-04: Visibility column widened 48 to 80pt (a requested fix); overlay test contexts now install the real skin fonts (made the full test run slow, since fixed by the dev-profile change, commit 01f5b9e); the styled diamond is a vector shape, not the slider-thumb texture; the draft-hint tooltip rule was applied to the new-style field as well.

## Known Stubs
None.

## Threat Flags
None. T-30-18 (screenshot runs): the env guard held (temp-only home, profile and derived game folder), the fixture was not modified (`git diff` on `tests/fixtures` empty), T-30-19 (PNGs): seed data and default styles only, the status shot shows a sample path string, T-30-20: no keys sent, the user's running `ff` was never contacted.

## Self-Check: PASSED
