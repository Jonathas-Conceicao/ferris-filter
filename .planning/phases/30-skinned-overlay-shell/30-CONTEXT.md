# Phase 30: Skinned Overlay Shell - Context

**Gathered:** 2026-10-16
**Status:** Ready for planning

<domain>
## Phase Boundary

`ff` opens as the new game-styled window (the `ff-proto` look built on `src/skin/`) instead of the
stripe, carrying today's RULES and CONFIG content inside it with every v0.5 behaviour intact. Proper
skinned rebuilds of the RULES tab (Phase 31) and CONFIG styles (Phase 33) come later; ECONOMY and
ABOUT get their real content in Phase 34 (this phase may host placeholders / move SYNC as needed by
the shell, but ECO/ABOUT requirements belong to 34).

</domain>

<decisions>
## Implementation Decisions

### Window
- Fixed window: not movable, not resizable; 660pt wide, docked to the left edge at full work-area
  height, borderless, always on top, static Ferris background behind scrolling content (as in
  ff-proto).
- Re-docks every time it is summoned (hotkey / tray / second launch), on the monitor the game window
  is on; primary monitor when the game is not running. DPI-correct at 100-200% (the work area must be
  converted with the target monitor's scale; fix the old `SPI_GETWORKAREA`-primary-only bug).
- Hide-when-game-takes-focus, hotkey (`Ctrl+Shift+Z`, only while PoE2 or ff is foreground), tray +
  Quit flow, single instance and in-game reload after Apply behave exactly as in v0.5.

### Chrome
- Header: game title plate with "FerrisFilter" in a slot designed to take custom header art later
  (author will draw it); close button hides (tray Quit still the only exit).
- Tabs RULES / CONFIG / ECONOMY / ABOUT with the prototype's fixed stone-tab layout; ` *` marker on a
  tab label when the other tab has unapplied edits (v0.5 behaviour).
- Bottom bar: REVERT / RELOAD / APPLY plus one status line: single line, truncated with "…", full
  text on hover only when cut, coloured ok / warning / error. Every message that today goes to the
  96px outcome area goes here. Context-specific actions that the outcome area offered (RELOAD /
  OVERWRITE on changed-on-disk, the Quit confirm) move to popups.

### Interim look (author: "these things can be fixed on a later phase, no issue")
- Existing RULES/CONFIG widgets only get a global restyle (skin fonts + palette via egui Style), so
  they look close but mixed until Phases 31/33 rebuild them. Acceptable.

### Popups
- Every confirmation/choice (changed-on-disk RELOAD / OVERWRITE, switch filter + discard, Quit with
  unapplied edits — Apply & quit / Quit without applying / Cancel —, deletes) is a modal: dimmed
  window, a game-framed box (9slicemenu art) centred, game button art; Escape cancels.

### Less texty (backlog 999.11)
- Standing captions become tooltips; flags (`group broken`, `can't enable`, `unknown style`,
  `moved across Continue`, grammar errors…) become small icons with a tooltip; long outcomes go to
  the status line with hover for full text.
- **Author request:** the phase's human-verification checklist must list concretely what changed
  here (which captions became tooltips, which flags became which icons, where messages moved) so the
  author can review each in the running overlay.

### Claude's Discretion
- How the shell is wired (replace `overlay::theme` usage incrementally; `overlay::theme` itself is
  deleted in Phase 35), icon choices for flags, monitor-detection API details.

</decisions>

<code_context>
## Existing Code Insights

- `src/skin/`: palette, fonts (Fontin SmallCaps/Regular, Cinzel), textures (include_bytes!, LINEAR
  + mipmaps), nine/three-slice painting, widgets `ff_frame/ff_header/ff_close/ff_tabs/tab_strip/
  ff_section/ff_row/ff_check/ff_slider/ff_dropdown/ff_button/arrows/text field/scrollbar`, status
  line (truncate + tooltip + kinds), `skin::window` (DPI-correct left dock, primary monitor only),
  `skin::editor` + `skin::minimap` (prototype style editor pieces).
- `src/bin/ff-proto.rs` (1437 lines): reference composition of header, tabs, frame, bottom bar,
  status, Ferris background — reuse its layout code; the binary itself is removed in Phase 35.
- Overlay: `src/overlay/app.rs` (3763, the eframe App), `body.rs` (5812, rule editor body),
  `rules.rs`, `rule_list.rs`, `virtual_list.rs`, `config_view.rs`, `config_state.rs`,
  `widgets.rs` (themed_slider ×5, circular_toggle ×11, slide_switch, toggle_chip ×7 wrappers),
  `theme.rs` (current stripe styling), `layout.rs`, `visibility.rs`, `win.rs` (Win32 docking —
  uses primary work area), `tray.rs`, `hotkey.rs`, `instance.rs`, `chat.rs` (in-game reload),
  `apply.rs` / `apply_all.rs`, `economy.rs` (sync worker).
- Constraints: `artifact_guard` / fixtures byte-identical; full suite green; fmt clean.

</code_context>

<specifics>
## Specific Ideas

- References: `C:\Users\jonat\Downloads\poe2_print009.png` (game Options window),
  ff-proto screenshots in `.planning/quick/261016-1de-poe2-prototype-tab-layout-style-editor-a/`.

</specifics>

<deferred>
## Deferred Ideas

- Custom header art (author draws it later).

</deferred>
