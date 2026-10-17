---
status: partial
phase: 34-economy-about-tabs
source: [34-VERIFICATION.md]
started: 2026-10-16T22:00:00.000Z
updated: 2026-10-16T22:00:00.000Z
---

## Current Test

[awaiting human testing]

Run the overlay with `FERRISFILTER_HOME` and `USERPROFILE` on a temp copy (CLAUDE.md development warning), so Apply does not write your live `styles.json` / `tiers.json` / filter.

## Tests

### 1. ABOUT-01: ABOUT at 100 / 150 / 200% DPI
expected: Top to bottom: FerrisFilter, "Loot filter editor for Path of Exile 2", the orange Ferris (crisp), Version x.y.z, the GitHub link, a centred gold divider, empty space, the GGG disclaimer, then the smaller credits pinned to the bottom
result: [pending]

### 2. ABOUT-01: GitHub link
expected: Hover shows "Open the repository in your browser."; a click opens https://github.com/Jonathas-Conceicao/poe2-myfilter in the default browser
result: [pending]

### 3. ECO-01: Add economy section
expected: On a filter without one, the button reads Add economy section; a click stages it without writing the file, the button flips to Remove economy section and ECONOMY shows ` *` from another tab; Revert undoes it; Apply writes the begin-only banner at the end of the file
result: [pending]

### 4. ECO-01: Remove economy section
expected: The Remove economy section? popup asks first; Keep changes nothing; Remove stages it; Apply removes the banner and its blocks
result: [pending]

### 5. ECO-02: tier edit on ECONOMY
expected: Editing a cutoff marks ECONOMY (not CONFIG) with ` *`; an invalid cutoff gets a red outline, its reason on hover, and Apply disabled; Revert on ECONOMY asks via Discard config changes? and leaves style edits (CONFIG) intact; Keep editing keeps a staged economy add/remove too
result: [pending]

### 6. ECO-01: Sync economy (real run on a temp home)
expected: The window stays responsive, only prices.json is written, Last sync becomes Just now, Priced items and Price source (Live prices) update; the status names the league
result: [pending]

### 7. ECO-01: Last sync / Price source without or with a broken prices.json
expected: Last sync is relative with the exact time on hover; no prices.json reads Fallback prices in the warning colour; a malformed / empty prices.json reads Never + Fallback prices with the reason on hover
result: [pending]

### 8. In-game reload after Apply from ECONOMY
expected: The outcome reads Reloaded in game and the game refreshes the filter
result: [pending]

### 9. RULES Economy divider and CONFIG
expected: The RULES Economy divider only collapses / expands (no x, no Add economy section footer, tooltip points to ECONOMY); CONFIG shows Filter and Styles only (no Tiers)
result: [pending]

### 10. Not-texty acceptance of ECONOMY and ABOUT
expected: Author accepts the density (four info rows, one Sync button, one section row, the tiers table) and the pale background Ferris showing behind the empty ABOUT middle; "60 locked blocks" beside the Remove button reads fine
result: [pending]

### 11. fix(33) 96d5861: long field-row labels
expected: A RULES label like Set Background Color is cut with "…" in the label column and shows the full name on hover
result: [pending]

### 12. fix(33) 2fa9875: status indicator
expected: The standing indicator reads "Unapplied changes" (fits on one line); hovering it explains: press Apply to write the filter, then it reloads in game
result: [pending]

### 13. fix(33) 6ca7ba7: Font size off
expected: With the font override off the line shows the check plus a dimmed slider reading "Default"; both explain on/off on hover; ticking restores the last size
result: [pending]

### 14. fix(33) 2816f13: colour picker frame
expected: The game-box frame wraps the whole picker popup with room around the square, fields and Use colour, like the modal popups
result: [pending]

### 15. 2475c48: ABOUT divider
expected: A symmetric gold rule centred under the GitHub link, fading at both ends
result: [pending]

## Summary

total: 15
passed: 0
issues: 0
pending: 15
skipped: 0
blocked: 0

## Gaps

## UAT session 2026-10-17 (author)

Passed: ABOUT layout, DPI and link; ECONOMY rows, Sync economy, fallback source; Add / Remove
economy section with Revert and Apply; tier marker, validation and gated Apply. The pale background
crab and the description wording are accepted.

Feedback: the tier input boxes are too wide, so the `chaos` unit clips; make them narrower (values
are not expected past about 6 digits) with a gap before the unit (queued as a follow-up quick task).
League picker / price source UI went to backlog 999.17.
