---
status: passed
phase: 37-game-folder-resolver-picker
source: [37-VERIFICATION.md, 37-0*-SUMMARY.md, 37-REVIEW-FIX.md]
started: 2026-10-18T00:00:00Z
updated: 2026-10-18T00:00:00Z
---

## Current Test

[awaiting human testing — deferred by the author to the end of the milestone run]

## Tests

### 1. Folder dialog above the always-on-top overlay
Click Browse on the Config tab with the overlay shown over the game.
expected: The native folder dialog appears above the overlay and is usable. The overlay does not hide while the dialog is open or in the half second after it closes, and Ctrl+Shift+Z does nothing while it is open. The picked folder is shown in the Game folder row and restored on the next launch. (If the dialog lands behind the overlay, apply the WindowLevel contingency described in src/overlay/folder_dialog.rs.)
result: [pending]

### 2. Default folder on a OneDrive-redirected / localized Documents
With FERRISFILTER_GAME_DIR unset and no saved game_folder, open the Config tab.
expected: The Game folder row shows <OS Documents>\My Games\Path of Exile 2 (on this machine the OneDrive\Documentos one, the same folder as before).
result: [pending]

### 3. Refresh button art
Hover and press the refresh button beside the Filter dropdown, at 100% and 150% DPI.
expected: Normal, hover and pressed game art (round arrow on a square bronze button) show correctly and line up with Browse.
result: [pending]

### 4. Refresh lists new files without losing edits
With unapplied rule edits, drop a new .filter and a new .mp3 into the game folder and press refresh.
expected: Both appear (filter dropdown, Alert sound "Filter folder" section); nothing is written; the rule edits and the unapplied marker survive.
result: [pending]

### 5. Folder pick with unapplied rule edits
Make a rule edit, then Browse to a different folder.
expected: Switch filter? opens and names the folder. Stay changes nothing (no write). Switch and discard loads the folder's filter (same name, else first .filter) and survives a restart.
result: [pending]

### 6. Missing game folder
Point the saved game_folder (or FERRISFILTER_GAME_DIR) at a folder that does not exist.
expected: The status line reads "Game folder not found: <path>" as an error; clicking it opens the Config tab; refresh stays clickable and clears the status once the folder exists.
result: [pending]

### 7. Fresh install filter choice and ff-cli apply default
With no filter key saved, launch ff on a folder holding only x.filter; Reload; toggle "Reload filter in game after Apply"; then run `ff-cli apply` with no --filter.
expected: ff opens x.filter (also after Reload) without writing a filter key; toggling the reload setting does not save x.filter as the choice; clicking the highlighted x.filter entry saves it; `ff-cli apply` targets the filter saved in ff (dynamic.filter when none is saved).
result: [pending]

### 8. Screenshot-mode guard
Run a debug `ff --screenshot` with FERRISFILTER_GAME_DIR unset, then pointed at a non-temp folder.
expected: Both refuse and name the variable; nothing is written.
result: [pending]

### 9. tests/ui/README.md recipe
Walk through the `$env:FERRISFILTER_GAME_DIR` screenshot recipe once.
expected: It produces the Config tab screenshot in the temp folder.
result: [pending]

## Summary

total: 9
passed: 0
issues: 0
pending: 9
skipped: 0
blocked: 0

## Gaps
