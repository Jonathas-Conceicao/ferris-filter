# Phase 47: Customizable Hotkey - Context

**Gathered:** 2026-10-23
**Status:** Ready for planning
**Mode:** Smart discuss (autonomous run 43-50; author answered every question up front in prose)

<domain>
## Phase Boundary

The player picks the system-wide show / hide chord on the CONFIG tab instead of the hardcoded
`Ctrl+Shift+Z`. Requirements KEY-01..04.

</domain>

<decisions>
## Implementation Decisions

### Layout (author)
- A new **Overlay** section on the CONFIG tab, under Filter, with one row: **Show / hide overlay**.
- Styled after the game's Options → Input keybind rows (reference
  `.planning/research/ui-refs/options-input-tab.png`, crop `options-keybind-rows.png`): the action
  name on the left in SmallCaps, and on the right a **wide button** holding the chord (dark plate,
  ornate gold corners, faint gold glow along the bottom, chord centred in white). Extract the
  keybind-button art from the Options menu with `ff-art`; fall back to the existing button art if it
  cannot be found (report which, for the author's review).
- Beside it the round game-art reset button (the refresh-style icon button) with tooltip
  `Reset to Ctrl+Shift+Z`.
- Compensate the art's transparent margins when positioning (see Phase 48 / ART-04).

### Capture (author)
- Clicking the button makes it read `Press a key…`; held modifiers show live (`Ctrl+Shift+…`);
  Escape or a click elsewhere cancels.
- At least one modifier is required, except bare F1-F24. Reject modifier-only chords and the Windows
  key.
- While capturing, the live hotkey must not fire (pause / ignore it).

### Saving (author — overrides the "save at once" proposal)
- A captured chord is a **pending CONFIG edit**: CONFIG tab ` *`, the unapplied indicator, Revert
  discards it, and the Quit popup counts it. **Apply** saves it to `ff-settings.json` and re-registers
  it at once (no restart).
- If registration fails at Apply (another app owns the chord), the old binding stays registered, the
  status line says why, and the new chord is not saved (decide how this fits the all-or-none Apply:
  validate / try registration before writing, so a refused chord refuses or skips cleanly).
- Reset restores `Ctrl+Shift+Z` (also a pending edit, applied on Apply).
- An invalid or unusable saved chord at start-up falls back to `Ctrl+Shift+Z` with a warning.

### Tray and other mentions (author)
- The tray icon's mouse-over tooltip must show the **live (applied)** chord, or stop naming the chord
  at all — never a stale `Ctrl+Shift+Z`. Same for any other UI text or messages naming the hotkey.
- Keep the foreground-only rule (acts only while the game or the overlay is the foreground window).
- Debug builds keep their separate instance name.

### Claude's Discretion
- Key mapping (egui `Key` → global-hotkey `Code`, a hand-written table with round-trip tests),
  the `ff-settings.json` key name, registration ordering (register new, then unregister old).

</decisions>

<code_context>
## Existing Code Insights

### Reusable Assets
- `src/overlay/hotkey.rs` (`TOGGLE_MODIFIERS` / `TOGGLE_CODE`, `register_toggle`, `release_toggle`),
  `HotkeyStatus::Registered { id }` and the poll in `src/overlay/app.rs`.
- `src/overlay/tray.rs` (tooltip), `src/settings.rs` (keys, warnings), the reload toggle's pending /
  saved handling as the closest model for a CONFIG preference.
- `global-hotkey` 0.8: `HotKey` implements `FromStr` / `Display`; `Code` / `Modifiers` re-exported.
  The hotkey id changes with the chord.

### Established Patterns
- Never register a real OS hotkey in tests; screenshot mode skips hotkey and tray.

### Integration Points
- CONFIG view (`src/overlay/config_view.rs`), Apply (`apply_all.rs`), Quit popup counting, tray.

</code_context>

<specifics>
## Specific Ideas

- The game's keybind button look from the Input tab screenshot.

</specifics>

<deferred>
## Deferred Ideas

None — discussion stayed within phase scope.

</deferred>
