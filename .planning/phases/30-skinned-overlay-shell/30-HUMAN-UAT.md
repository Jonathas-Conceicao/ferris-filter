---
status: partial
phase: 30-skinned-overlay-shell
source: [30-VERIFICATION.md]
started: 2026-10-16T10:24:25.469Z
updated: 2026-10-16T10:24:25.469Z
---

## Current Test

[awaiting human testing]

## Tests

### 1. [A] A1 Config > Filter dropdown. Old: standing line "Saved on this computer. Default:…
section: A. Standing captions that became tooltips or short headings
expected: A1 Config > Filter dropdown. Old: standing line "Saved on this computer. Default: dynamic.filter." Now: tooltip on the dropdown, line 1 the full filter path, line 2 that sentence (no game folder: only the sentence). Code: `config_view.rs:360-368`.
result: [pending]

### 2. [A] A2 Config > Filter, **Reload filter in game after Apply** toggle. Old: paragraph under…
section: A. Standing captions that became tooltips or short headings
expected: A2 Config > Filter, **Reload filter in game after Apply** toggle. Old: paragraph under it. Now: tooltip on the toggle (and label) = paragraph + blank line + short tip (`config_view.rs:~380`).
result: [pending]

### 3. [A] A3 Config > Filter, new-filter name field. Old: helper line. Now: tooltip on the empty…
section: A. Standing captions that became tooltips or short headings
expected: A3 Config > Filter, new-filter name field. Old: helper line. Now: tooltip on the empty field; a non-empty invalid name still shows an inline hint.
result: [pending]

### 4. [A] A3b Config > Styles, **+ Add style** name field. Same treatment as…
section: A. Standing captions that became tooltips or short headings
expected: A3b Config > Styles, **+ Add style** name field. Same treatment as A3.
result: [pending]

### 5. [A] A4 Config > Styles with no styles loaded. Old: "No styles loaded. Revert to reload…
section: A. Standing captions that became tooltips or short headings
expected: A4 Config > Styles with no styles loaded. Old: "No styles loaded. Revert to reload styles.json." Now: amber inline `No styles loaded`, hover for "Revert to reload styles.json." (`config_view.rs:637`).
result: [pending]

### 6. [A] A5 Config > custom style with custom sound, **File** field. Old: caption "The file must…
section: A. Standing captions that became tooltips or short headings
expected: A5 Config > custom style with custom sound, **File** field. Old: caption "The file must be in the game folder." Now: tooltip on the field (`config_view.rs:1136`). Also in RULES: a rule's custom-sound **File** field now carries the same tooltip instead of the caption (`body.rs:2814`).
result: [pending]

### 7. [A] A6 Rules, empty filter. Old: heading plus "Press + ADD RULE to write your first one..."…
section: A. Standing captions that became tooltips or short headings
expected: A6 Rules, empty filter. Old: heading plus "Press + ADD RULE to write your first one..." Now: heading `No rules yet` only (`app.rs:1936`); hover **+ Add rule** for the sentence.
result: [pending]

### 8. [A] A7 Rules, search with no hits. Old: heading plus "Check the spelling, or press Clear...".…
section: A. Standing captions that became tooltips or short headings
expected: A7 Rules, search with no hits. Old: heading plus "Check the spelling, or press Clear...". Now: heading `No rules match "<query>"` only; the field's Clear is the way out.
result: [pending]

### 9. [A] A8 Rules, unloadable filter. Old: heading, error paragraph, "Fix the file and press…
section: A. Standing captions that became tooltips or short headings
expected: A8 Rules, unloadable filter. Old: heading, error paragraph, "Fix the file and press RELOAD." Now: heading `The filter could not be loaded.` (`app.rs:1955`); detail in the red status line `The filter could not be loaded: <msg>. Fix the file and press Reload.` (`shell.rs:107`).
result: [pending]

### 10. [A] A9 Expanded rule with no conditions. Old: "No conditions: this rule matches every item."…
section: A. Standing captions that became tooltips or short headings
expected: A9 Expanded rule with no conditions. Old: "No conditions: this rule matches every item." Now: muted `No conditions` (`body.rs:449`), the sentence on hover.
result: [pending]

### 11. [A] A10 Expanded rule with no actions. Old: "No actions: matching items keep the game's…
section: A. Standing captions that became tooltips or short headings
expected: A10 Expanded rule with no actions. Old: "No actions: matching items keep the game's look." Now: muted `No actions` (`body.rs:2537`), sentence on hover.
result: [pending]

### 12. [A] A11 Disabled rule with invalid grammar, expanded. Old: long "can't enable" paragraph.…
section: A. Standing captions that became tooltips or short headings
expected: A11 Disabled rule with invalid grammar, expanded. Old: long "can't enable" paragraph. Now: amber `Can't enable`, long text on hover; the enable toggle carries the same tooltip (`rule_list.rs:610`).
result: [pending]

### 13. [A] A12 Disabled Apply with a grammar error. Old tooltip: `Fix or disable the rules marked…
section: A. Standing captions that became tooltips or short headings
expected: A12 Disabled Apply with a grammar error. Old tooltip: `Fix or disable the rules marked "grammar error" first.` Now: `Fix or disable the rules marked with the red ! first.` (`apply.rs:202`).
result: [pending]

### 14. [A] A13 Filter path. Old: line in the status block. Now: hover the gold "FerrisFilter" plate;…
section: A. Standing captions that became tooltips or short headings
expected: A13 Filter path. Old: line in the status block. Now: hover the gold "FerrisFilter" plate; tooltip = full path only (`app.rs:2347-2354`).
result: [pending]

### 15. [A] A14 "Economy section found." / "The filter has no economy section." Old: status-block…
section: A. Standing captions that became tooltips or short headings
expected: A14 "Economy section found." / "The filter has no economy section." Old: status-block line. Now: dropped; the economy footer row (`+ N locked rule blocks (economy)` / `No economy section`) says it.
result: [pending]

### 16. [A] A15 v0.4 notice, absent-filter notice, damaged-markers line. Old: status-block…
section: A. Standing captions that became tooltips or short headings
expected: A15 v0.4 notice, absent-filter notice, damaged-markers line. Old: status-block paragraphs. Now: status line only (C12, C13, C10).
result: [pending]

### 17. [A] A16 Apply indicator ("Filter out of date..." / "Filter matches saved rules"). Old: line…
section: A. Standing captions that became tooltips or short headings
expected: A16 Apply indicator ("Filter out of date..." / "Filter matches saved rules"). Old: line above the buttons. Now: the default status line (C14).
result: [pending]

### 18. [A] A17 Rule count. Old: "N rules (shown of N)" label and a "FILTER" header. Now: search hint…
section: A. Standing captions that became tooltips or short headings
expected: A17 Rule count. Old: "N rules (shown of N)" label and a "FILTER" header. Now: search hint `Search N rules, classes, base types…` (`rule_list.rs:815`).
result: [pending]

### 19. [A] A18 Economy remove. Old: inline "Remove the economy section? Yes, remove / Keep". Now:…
section: A. Standing captions that became tooltips or short headings
expected: A18 Economy remove. Old: inline "Remove the economy section? Yes, remove / Keep". Now: the Remove economy popup carrying "Your own rules are kept. Add the section back any time." (D6).
result: [pending]

### 20. [A] A19 Delete rule, Delete style, Revert on Config, Switch filter. Old: inline confirm…
section: A. Standing captions that became tooltips or short headings
expected: A19 Delete rule, Delete style, Revert on Config, Switch filter. Old: inline confirm frames. Now: popups D4, D5, D7, D2. No inline confirm should remain anywhere.
result: [pending]

### 21. [A] A20 Outcome area. Old: 96px scrolling block with several lines and jump links. Now: one…
section: A. Standing captions that became tooltips or short headings
expected: A20 Outcome area. Old: 96px scrolling block with several lines and jump links. Now: one status line (section C): one message at a time, full text on hover only when cut.
result: [pending]

### 22. [A] A21 Kept on purpose (should still be inline): field-validation hints, `Not set (game…
section: A. Standing captions that became tooltips or short headings
expected: A21 Kept on purpose (should still be inline): field-validation hints, `Not set (game default)`, short field captions, the two tier helper lines (`Minimum price for each tier...`, `Only your live tiers.json is edited...`).
result: [pending]

### 23. [B] B1 Grammar error: red filled circle with `!`. Tooltip: one `Line N: message` per…
section: B. Flags that became icons (collapsed Rules rows)
expected: B1 Grammar error: red filled circle with `!`. Tooltip: one `Line N: message` per diagnostic. Try `Sockets >= abc`. Never folded into `+N`.
result: [pending]

### 24. [B] B2 Grammar warning: amber outline triangle with `!`. Tooltip: `Line N: message`. Try…
section: B. Flags that became icons (collapsed Rules rows)
expected: B2 Grammar warning: amber outline triangle with `!`. Tooltip: `Line N: message`. Try `LinkedSockets >= 4`.
result: [pending]

### 25. [B] B3 Unknown style: amber ring with `?`. Tooltip: "No style with this name exists. This…
section: B. Flags that became icons (collapsed Rules rows)
expected: B3 Unknown style: amber ring with `?`. Tooltip: "No style with this name exists. This rule keeps its current actions, and its style link is removed on the next Apply." Try `# ff:style nosuch` above `Show`; the status line also reads `1 rule uses an unknown style (first: rule N).` and clicking it jumps to the rule.
result: [pending]

### 26. [B] B4 Group broken: amber broken-link glyph (two open ovals with a gap; reads small at 14pt…
section: B. Flags that became icons (collapsed Rules rows)
expected: B4 Group broken: amber broken-link glyph (two open ovals with a gap; reads small at 14pt per the 30-05 observation). Tooltip: "Group broken: these blocks were one rule (# ff:group) but no longer match, so each is shown on its own. Left as written until you edit it." Try a lone `# ff:group abc123 1/2` block.
result: [pending]

### 27. [B] B5 Moved across Continue: amber up/down arrows with a bar. Tooltip: "This move changes…
section: B. Flags that became icons (collapsed Rules rows)
expected: B5 Moved across Continue: amber up/down arrows with a bar. Tooltip: "This move changes which rules still apply after a Continue. Check the order." Reproduce by moving a rule across a `Continue` rule (cannot be rendered by the screenshot mode).
result: [pending]

### 28. [B] B6 Continue: stays the gold small-caps word `Continue`. Tooltip: "Matching goes on to the…
section: B. Flags that became icons (collapsed Rules rows)
expected: B6 Continue: stays the gold small-caps word `Continue`. Tooltip: "Matching goes on to the rules below".
result: [pending]

### 29. [B] B7 Styled: gold diamond + style name in muted text, cut at 10 characters plus `…` (was…
section: B. Flags that became icons (collapsed Rules rows)
expected: B7 Styled: gold diamond + style name in muted text, cut at 10 characters plus `…` (was 12, no diamond). Tooltip: `Style "<full name>" sets this rule's actions on Apply`.
result: [pending]

### 30. [B] B8 Can't enable: amber padlock alone on a disabled rule with invalid grammar (the only…
section: B. Flags that became icons (collapsed Rules rows)
expected: B8 Can't enable: amber padlock alone on a disabled rule with invalid grammar (the only flag shown for it). Tooltip: "Can't enable: these lines are not valid filter grammar. You can still move or delete it."
result: [pending]

### 31. [B] B9 Order right to left: error, warning, unknown style, group broken, moved across…
section: B. Flags that became icons (collapsed Rules rows)
expected: B9 Order right to left: error, warning, unknown style, group broken, moved across Continue, Continue, styled. Overflow folds the least important into a muted `+N` badge (hover lists `name: tip`); the label keeps at least 140px.
result: [pending]

### 32. [B] B10 Accessible names: each icon keeps the old flag word as its accessible name (UI…
section: B. Flags that became icons (collapsed Rules rows)
expected: B10 Accessible names: each icon keeps the old flag word as its accessible name (UI inspector; not visible).
result: [pending]

### 33. [C] C1 Apply success: `Wrote <path>.` plus `Reloaded in game` when the game runs and the…
section: C. Where each former outcome-area, status-block or footer message now appears
expected: C1 Apply success: `Wrote <path>.` plus `Reloaded in game` when the game runs and the setting is on. Green; amber when the text carries the Options > Game > Item Filter > Refresh reminder. Stays until the next edit, Revert, or hiding the window (correction).
result: [pending]

### 34. [C] C2 Written but not reloaded (setting off, reload in progress, window not ready, game not…
section: C. Where each former outcome-area, status-block or footer message now appears
expected: C2 Written but not reloaded (setting off, reload in progress, window not ready, game not running, `could not reload in game: <cause>`): amber, with the Refresh reminder and the reason.
result: [pending]

### 35. [C] C3 Apply refused, write failed or a file changed on disk: red, names the file(s); a…
section: C. Where each former outcome-area, status-block or footer message now appears
expected: C3 Apply refused, write failed or a file changed on disk: red, names the file(s); a changed file also opens the File changed on disk popup (D1).
result: [pending]

### 36. [C] C4 Overwrite outcome: same texts as Apply, in-game reload…
section: C. Where each former outcome-area, status-block or footer message now appears
expected: C4 Overwrite outcome: same texts as Apply, in-game reload included.
result: [pending]

### 37. [C] C5 Reload outcome (re-read from disk): status message from the reload. Rules-tab Revert…
section: C. Where each former outcome-area, status-block or footer message now appears
expected: C5 Reload outcome (re-read from disk): status message from the reload. Rules-tab Revert message `Reverted to the last applied filter.` is Info (muted), not green (correction).
result: [pending]

### 38. [C] C6 Sync economy: `Fetching economy prices…` (Info) while the button reads `Syncing…` and…
section: C. Where each former outcome-area, status-block or footer message now appears
expected: C6 Sync economy: `Fetching economy prices…` (Info) while the button reads `Syncing…` and is disabled; then `Economy prices refreshed from <league>. Press Apply to compile them into the filter.` (green) or `Economy sync failed: <err>` (red). Shown on every tab.
result: [pending]

### 39. [C] C7 List status, e.g. `Rule added at position N (hidden by the current search)`, and…
section: C. Where each former outcome-area, status-block or footer message now appears
expected: C7 List status, e.g. `Rule added at position N (hidden by the current search)`, and `Style removed. The actions are now yours to edit.`: Info, replaced by the next edit.
result: [pending]

### 40. [C] C8 Grammar gate: `Apply refused: 1 enabled rule has a grammar error (first: rule N, line…
section: C. Where each former outcome-area, status-block or footer message now appears
expected: C8 Grammar gate: `Apply refused: 1 enabled rule has a grammar error (first: rule N, line M). Fix them or turn them off.` Red; clicking it switches to RULES, expands and scrolls to the rule (clearing a hiding search); hand cursor while a target exists; Apply disabled with the A12 tooltip.
result: [pending]

### 41. [C] C9 Unknown-style summary: amber, click jumps to the…
section: C. Where each former outcome-area, status-block or footer message now appears
expected: C9 Unknown-style summary: amber, click jumps to the rule.
result: [pending]

### 42. [C] C10 Damaged economy markers: `The economy section markers are damaged. Fix them by hand,…
section: C. Where each former outcome-area, status-block or footer message now appears
expected: C10 Damaged economy markers: `The economy section markers are damaged. Fix them by hand, then Reload.` Red; footer row label red with the same tooltip; Apply disabled with that tooltip.
result: [pending]

### 43. [C] C11 Load failure: `The filter could not be loaded: <msg>. Fix the file and press Reload.`…
section: C. Where each former outcome-area, status-block or footer message now appears
expected: C11 Load failure: `The filter could not be loaded: <msg>. Fix the file and press Reload.` Red.
result: [pending]

### 44. [C] C12 v0.4 filter: `v0.4 filter: converted on the next Apply (a .ffbak backup is kept).`…
section: C. Where each former outcome-area, status-block or footer message now appears
expected: C12 v0.4 filter: `v0.4 filter: converted on the next Apply (a .ffbak backup is kept).` Info.
result: [pending]

### 45. [C] C13 Absent filter: `The filter does not exist yet. Apply creates it once it has rules.`…
section: C. Where each former outcome-area, status-block or footer message now appears
expected: C13 Absent filter: `The filter does not exist yet. Apply creates it once it has rules.` Info.
result: [pending]

### 46. [C] C14 Default indicator: `Filter out of date — press Apply, then reload in-game` amber when…
section: C. Where each former outcome-area, status-block or footer message now appears
expected: C14 Default indicator: `Filter out of date — press Apply, then reload in-game` amber when unapplied or never applied; `Filter matches saved rules` Info when applied. Same on every tab.
result: [pending]

### 47. [C] C15 Missing app root: red, highest…
section: C. Where each former outcome-area, status-block or footer message now appears
expected: C15 Missing app root: red, highest priority.
result: [pending]

### 48. [C] C16 Config message (e.g. `Could not save ff-settings.json: <err>` with a read-only…
section: C. Where each former outcome-area, status-block or footer message now appears
expected: C16 Config message (e.g. `Could not save ff-settings.json: <err>` with a read-only settings file): red on every tab.
result: [pending]

### 49. [C] C17 Long text: cut with `…` on one line, full text on hover only then (max width 420),…
section: C. Where each former outcome-area, status-block or footer message now appears
expected: C17 Long text: cut with `…` on one line, full text on hover only then (max width 420), same colour. Reproduce with `ff --screenshot x.png --status "<300 chars>"` and compare `shots/status-truncated.png`.
result: [pending]

### 50. [C] C18 Button tooltips: Reload "Discard your edits and load the file from disk." (disabled:…
section: C. Where each former outcome-area, status-block or footer message now appears
expected: C18 Button tooltips: Reload "Discard your edits and load the file from disk." (disabled: "Nothing to reload."), Overwrite "Replace the file with your edits.", Apply disabled reasons per `shell::bar_state` (tier errors, load error, red `!` rules, damaged markers, nothing to apply).
result: [pending]

### 51. [D] D1 `changed`: title "File changed on disk"; body names the file(s) and explains Reload…
section: D. Popups
expected: D1 `changed`: title "File changed on disk"; body names the file(s) and explains Reload and Overwrite. Buttons: Reload, Overwrite (red), Keep editing (the last, so Escape = Keep editing). When Overwrite is unavailable the Overwrite button is absent and one sentence explains why (see G3). After Keep editing the Apply slot reads **Overwrite** (red) and reopens the popup. Trigger: edit the filter on disk, then Apply.
result: [pending]

### 52. [D] D2 `switch`: "Switch filter?", body "Unapplied rule changes will be discarded. Switching…
section: D. Popups
expected: D2 `switch`: "Switch filter?", body "Unapplied rule changes will be discarded. Switching to <file>." Buttons: Switch and discard (red), Stay. Escape = Stay. Trigger: Config > Filter, pick another filter with unapplied rule edits.
result: [pending]

### 53. [D] D3 `quit`: "Unapplied changes", body "Apply them before quitting?" Buttons: Apply & quit,…
section: D. Popups
expected: D3 `quit`: "Unapplied changes", body "Apply them before quitting?" Buttons: Apply & quit, Quit without applying (red, tooltip "Unapplied edits are lost"), Cancel. Escape = Cancel. Apply & quit quits only after a successful Apply and waits for an in-game reload; with nothing pending Quit exits at once.
result: [pending]

### 54. [D] D4 `delete-rule`: "Delete this rule?", body `Rule N: <name or Show/Hide>` plus `Deletes…
section: D. Popups
expected: D4 `delete-rule`: "Delete this rule?", body `Rule N: <name or Show/Hide>` plus `Deletes all N blocks of this rule.` for a group. Buttons: Delete (red), Keep. Trigger: expand a rule, Delete.
result: [pending]

### 55. [D] D5 `delete-style`: "Delete style?", body the rule count and the `config.json` warning…
section: D. Popups
expected: D5 `delete-style`: "Delete style?", body the rule count and the `config.json` warning when referenced. Buttons: Delete style (red), Keep style.
result: [pending]

### 56. [D] D6 `remove-economy`: "Remove economy section?", body "Your own rules are kept. Add the…
section: D. Popups
expected: D6 `remove-economy`: "Remove economy section?", body "Your own rules are kept. Add the section back any time." Buttons: Remove (red), Keep.
result: [pending]

### 57. [D] D7 `discard-config`: "Discard config changes?", body `Discard N unapplied config…
section: D. Popups
expected: D7 `discard-config`: "Discard config changes?", body `Discard N unapplied config changes?` Buttons: Discard (red), Keep editing.
result: [pending]

### 58. [D] D8 Stacking: with a quit request and a changed-on-disk refusal both pending, Quit shows…
section: D. Popups
expected: D8 Stacking: with a quit request and a changed-on-disk refusal both pending, Quit shows first, then File changed on disk.
result: [pending]

### 59. [D] D9 Keyboard: Escape = last button (no action); Tab cycles buttons; Enter/Space activates…
section: D. Popups
expected: D9 Keyboard: Escape = last button (no action); Tab cycles buttons; Enter/Space activates the focused one; no button focused on open; backdrop click = last button.
result: [pending]

### 60. [D] D10 Ctrl+Shift+Z while a popup is open, summon again: the same popup is still…
section: D. Popups
expected: D10 Ctrl+Shift+Z while a popup is open, summon again: the same popup is still there.
result: [pending]

### 61. [E] E1 Game on the primary monitor at 100%: window 660pt wide, flush left, full work-area…
section: E. Docking, DPI and monitors
expected: E1 Game on the primary monitor at 100%: window 660pt wide, flush left, full work-area height (above the taskbar), borderless, not movable, not resizable, always on top.
result: [pending]

### 62. [E] E2 Repeat at 125%, 150%, 200% (sign out/in if needed): still 660 logical points (grows in…
section: E. Docking, DPI and monitors
expected: E2 Repeat at 125%, 150%, 200% (sign out/in if needed): still 660 logical points (grows in pixels), full height above the taskbar, no gap or overlap with the screen edge. 125% was the old failure.
result: [pending]

### 63. [E] E3 Game (windowed or windowed-fullscreen) on a secondary monitor: the window docks on…
section: E. Docking, DPI and monitors
expected: E3 Game (windowed or windowed-fullscreen) on a secondary monitor: the window docks on that monitor's left edge at that monitor's own scale.
result: [pending]

### 64. [E] E4 Game not running: docks on the primary…
section: E. Docking, DPI and monitors
expected: E4 Game not running: docks on the primary monitor.
result: [pending]

### 65. [E] E5 Re-dock on summon: hide the window, move the game to the other monitor, press…
section: E. Docking, DPI and monitors
expected: E5 Re-dock on summon: hide the window, move the game to the other monitor, press Ctrl+Shift+Z (game in front): it appears on the game's monitor. Repeat via a tray icon click and via launching `ff` a second time.
result: [pending]

### 66. [E] E6 Move the game between monitors of different scale, then summon: correctly sized (no…
section: E. Docking, DPI and monitors
expected: E6 Move the game between monitors of different scale, then summon: correctly sized (no mixed physical/logical sizes); a scale change while shown re-docks once.
result: [pending]

### 67. [E] E7 Taskbar on the left: window starts right of it. Taskbar on top: window starts below it…
section: E. Docking, DPI and monitors
expected: E7 Taskbar on the left: window starts right of it. Taskbar on top: window starts below it and ends above any bottom bar.
result: [pending]

### 68. [E] E8 Close button (round red, top right) hides, never quits; tooltip "Hide (Ctrl+Shift+Z)".…
section: E. Docking, DPI and monitors
expected: E8 Close button (round red, top right) hides, never quits; tooltip "Hide (Ctrl+Shift+Z)". Only tray Quit exits.
result: [pending]

### 69. [E] E9 Window cannot be dragged or resized; it hides when the game takes focus (v0.5…
section: E. Docking, DPI and monitors
expected: E9 Window cannot be dragged or resized; it hides when the game takes focus (v0.5 behaviour).
result: [pending]

### 70. [E] E10 Content fits at about 535pt with no horizontal scrolling (expanded rule with Class,…
section: E. Docking, DPI and monitors
expected: E10 Content fits at about 535pt with no horizontal scrolling (expanded rule with Class, Rarity and Defences cards; the Custom styles table).
result: [pending]

### 71. [F] F1 Ctrl+Shift+Z toggles only while Path of Exile 2 or ff is the foreground window; in a…
section: F. v0.5 behaviour checks (SHELL-06)
expected: F1 Ctrl+Shift+Z toggles only while Path of Exile 2 or ff is the foreground window; in a browser or editor the key does nothing and is not replayed later.
result: [pending]

### 72. [F] F2 Tray: Show works; tray Quit with nothing unapplied exits at once; with unapplied edits…
section: F. v0.5 behaviour checks (SHELL-06)
expected: F2 Tray: Show works; tray Quit with nothing unapplied exits at once; with unapplied edits (either tab) it opens the Quit popup: Apply & quit (quits only after a successful Apply, waits for the in-game reload), Quit without applying, Cancel.
result: [pending]

### 73. [F] F3 Single instance: launching `ff` again (also while the first is hidden to tray, or…
section: F. v0.5 behaviour checks (SHELL-06)
expected: F3 Single instance: launching `ff` again (also while the first is hidden to tray, or still starting) shows the running window and the new process exits; after tray Quit an immediate relaunch starts a fresh window.
result: [pending]

### 74. [F] F4 Apply with "Reload filter in game after Apply" on and the game running (same privilege…
section: F. v0.5 behaviour checks (SHELL-06)
expected: F4 Apply with "Reload filter in game after Apply" on and the game running (same privilege level, chat closed): `/reloaditemfilter` is sent and the status reads `Reloaded in game`; with the game not running it reads `Wrote <path>.` With the setting off the status keeps the Options > Game > Item Filter > Refresh reminder.
result: [pending]

### 75. [F] F5 Exclusive fullscreen still hides the window (unchanged requirement: windowed or…
section: F. v0.5 behaviour checks (SHELL-06)
expected: F5 Exclusive fullscreen still hides the window (unchanged requirement: windowed or windowed-fullscreen).
result: [pending]

### 76. [F] F6 Tab ` *` markers: edit on RULES, look at CONFIG (marked), and the reverse; Economy and…
section: F. v0.5 behaviour checks (SHELL-06)
expected: F6 Tab ` *` markers: edit on RULES, look at CONFIG (marked), and the reverse; Economy and About never marked; switching tabs applies and discards nothing.
result: [pending]

### 77. [G] G1 CONFIG-tab Reload routing (CR-01, `app.rs` `bar_reload_target`): on CONFIG with a…
section: G. Review-fix behaviours flagged "requires human verification"
expected: G1 CONFIG-tab Reload routing (CR-01, `app.rs` `bar_reload_target`): on CONFIG with a changed-on-disk refusal pending, the bar's Reload re-reads only the changed files (a changed `styles.json`/`tiers.json` discards CONFIG edits; a changed filter discards RULES edits). On CONFIG with no refusal, Reload re-reads the filter and keeps CONFIG edits.
result: [pending]

### 78. [G] G2 Overwrite enable/disable (WR-03 and WR-02): during a refusal the Apply slot reads…
section: G. Review-fix behaviours flagged "requires human verification"
expected: G2 Overwrite enable/disable (WR-03 and WR-02): during a refusal the Apply slot reads **Overwrite** (red). It is enabled only when Apply would be; the bar and the popup agree (the popup then has the Overwrite button; otherwise it has Reload and Keep editing only). A loaded `styles.json`/`tiers.json` that fails validation also disables it.
result: [pending]

### 79. [G] G3 Disabled Overwrite reason (WR-02): the bar tooltip and one popup sentence name the…
section: G. Review-fix behaviours flagged "requires human verification"
expected: G3 Disabled Overwrite reason (WR-02): the bar tooltip and one popup sentence name the reason, in this order: tier errors ("Fix the tier errors to overwrite instead."), invalid config ("styles.json or tiers.json is invalid. Fix it first." / "Fix styles.json or tiers.json to overwrite instead."), flagged rules ("Fix or disable the flagged rules to overwrite instead."), damaged markers ("Overwrite is off while the economy markers are damaged."), nothing to write ("Nothing to overwrite." and no popup sentence). Check the wording reads well in the skinned popup.
result: [pending]

### 80. [G] G4 Apply status colours (CR-02): success green, success with the Refresh reminder amber,…
section: G. Review-fix behaviours flagged "requires human verification"
expected: G4 Apply status colours (CR-02): success green, success with the Refresh reminder amber, refusal and failure red; colour is fixed when the message is produced and does not change when the apply state later flips to out-of-date.
result: [pending]

### 81. [G] G5 Stale messages (WR-01, both iterations): after any edit, Revert (either tab), economy…
section: G. Review-fix behaviours flagged "requires human verification"
expected: G5 Stale messages (WR-01, both iterations): after any edit, Revert (either tab), economy add/remove staging, or hiding and re-summoning, no old `Wrote ...` / `Rule added ...` / config message hides the standing lines (grammar gate, load error, out-of-date indicator). Exception: an in-progress reload line survives a hide.
result: [pending]

### 82. [G] G6 Status-line jump (WR-02 iter 1): from CONFIG or ECONOMY click the grammar-gate or…
section: G. Review-fix behaviours flagged "requires human verification"
expected: G6 Status-line jump (WR-02 iter 1): from CONFIG or ECONOMY click the grammar-gate or unknown-style line: it switches to RULES and expands the rule.
result: [pending]

### 83. [G] G7 CONFIG Revert (WR-04): disabled while a changed-on-disk refusal is pending; style…
section: G. Review-fix behaviours flagged "requires human verification"
expected: G7 CONFIG Revert (WR-04): disabled while a changed-on-disk refusal is pending; style switch track and tier-error bar are visibly coloured (WR-06); ABOUT disclaimer has no run of spaces (WR-05).
result: [pending]

## Summary

total: 83
passed: 0
issues: 0
pending: 83
skipped: 0
blocked: 0

## Gaps

## UAT session 2026-10-17 (author)

Passed: docking / DPI / multi-monitor / re-dock (E), v0.5 behaviours (F: hotkey, tray, single
instance, in-game reload), every popup incl. the new Discard rule changes? (D), status line (C).
Style-related items (A captions on the style editor) are re-checked after the style-groups quick task.
