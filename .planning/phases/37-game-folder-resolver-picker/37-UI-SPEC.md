---
phase: 37
slug: game-folder-resolver-picker
status: approved
reviewed_at: 2026-10-18
shadcn_initialized: false
preset: none
created: 2026-10-18
---

# Phase 37 — UI Design Contract

> Visual and interaction contract. Native egui overlay with the PoE2 skin (`src/skin/`), not a web app: there is no shadcn, Tailwind or registry. Units are egui points (pt), the window is the fixed 660pt left-docked one. Sources: `37-CONTEXT.md` (locked), `src/overlay/config_view.rs` (`show_filter`, ~L301), `src/skin/widgets.rs`, `src/skin/palette.rs`, `src/overlay/shell.rs` (`status_for`).

---

## Design System

| Property | Value |
|----------|-------|
| Tool | none (custom skin, `src/skin/`) |
| Preset | not applicable |
| Component library | egui / eframe with `ff_*` widgets in `src/skin/widgets.rs` |
| Icon library | PoE2 game art extracted by `tools/ff-art` into `assets/poe2/` (compiled in with `include_bytes!`); painted flag icons in `src/skin/icons.rs` |
| Font | Fontin Regular + Fontin SmallCaps (`src/skin/fonts.rs`) |

---

## Spacing Scale

The overlay uses the existing pt values; this phase adds none. Reuse:

| Token | Value | Usage (cite) |
|-------|-------|-------|
| Frame inner margin | 8pt | Filter section frame (`Margin::same(8)`, `show_filter`) |
| Item spacing y | 4pt | Rows inside the Filter frame (`item_spacing.y = 4.0`) |
| Row height | 34pt | `widgets::ROW_HEIGHT`; also the `ff_button` height |
| Label column | 128pt + 16pt gap | `widgets::LABEL_COL_W`, `LABEL_GAP` (two-column label / control layout) |
| Control column | 252pt | `widgets::CONTROL_WIDTH` |
| Field gap | 10pt | `widgets::FIELD_GAP`, gap between a control and the button beside it |
| Section gap | 16pt | `ui.add_space(16.0)` between Filter and Styles |

Exceptions: the refresh button is a 34 x 34pt square (matches `ROW_HEIGHT` and the `ff_button` height of 34pt), off the 4pt grid; it is the existing skin button height. Plans reference the skin constants by name (`ROW_HEIGHT`, `FIELD_GAP`, `LABEL_COL_W`, `CONTROL_WIDTH`), never literals.

---

## Typography

No new sizes. Existing roles:

| Role | Size | Weight | Line Height |
|------|------|--------|-------------|
| Row label (game folder / filter) | Fontin SmallCaps via `ff_field_row` label column | single face | egui default |
| Path / value text | `SUMMARY_SIZE` 15pt (`fonts::SUMMARY_SIZE`), `pal::VALUE` | Fontin Regular | egui default |
| Button caption | 17pt SmallCaps (`ff_button_ex`) | Fontin SmallCaps | n/a |
| Status line | existing `ff_status` | Fontin Regular | one line, cut with ellipsis |

Weights: the game fonts have no weights; the two faces above are the only ones.

---

## Color

Palette constants from `src/skin/palette.rs` only:

| Role | Value | Usage |
|------|-------|-------|
| Dominant | `GROUND` 0A0A09 / `PANEL` 0F0E0D | window and page |
| Secondary | `ROW_EVEN` (black a=96) | Filter section frame fill (as today) |
| Accent | `GOLD` C8A25A | unchanged use only |
| Destructive | `ERROR` E06252 | not used by this phase's controls |

Text: path `VALUE` ECEAE4; row labels `LABEL` D8C18E; disabled refresh/Browse `DISABLED` 88856E; status "Game folder not found" uses `StatusKind::Error` (`ERROR`).

Accent reserved for: nothing new. No control in this phase uses gold.

---

## Layout and Components

Filter section (the section title "Filter" stays; the in-frame caption `GAME FOLDER FILTER` is replaced by the two labelled rows, no caption text). Both rows use `ff_field_row`-style two-column layout (label left in the 128pt column, control right):

1. **Game folder row**
   - Label: `Game folder`.
   - Control: path text (`pal::VALUE`, 15pt) on a single line, truncated with an ellipsis (`…`) at the end of the available width minus the Browse button; hover shows the full path. Path is the resolved folder (env override, saved `game_folder`, or default). When the folder does not exist the path stays shown in `pal::WARN`.
   - Right of the path, `FIELD_GAP` (10pt) away: `ff_button` **Browse** (`ff_button(ui, skin, "Browse", 96.0)`), right-aligned to the control column edge.
   - Browse tooltip: `Default: <default path>` (the resolved `Documents\My Games\Path of Exile 2`). When `FERRISFILTER_GAME_DIR` is set: Browse is disabled (`ff_button_ex` enabled=false) and its tooltip reads `Set by FERRISFILTER_GAME_DIR`, path tooltip likewise unchanged (full path).
2. **Filter row**
   - Label: `Filter`.
   - Control: the existing filter dropdown (combo, `cfg-filter`, behaviour unchanged incl. `New empty filter…`) taking the control width minus 34pt minus `FIELD_GAP`, then **refresh button** directly at its right edge, 10pt gap.
   - Existing dropdown hover (`<full filter path>`; `Saved on this computer. Default: dynamic.filter.`) becomes just the full filter path: the "Default: dynamic.filter" claim is wrong once the first-found rule applies. Disabled dropdown text when folder missing stays `(game folder not found)`.
3. Unchanged below: the draft row (when open) and the Reload-after-Apply toggle row.

**Refresh button**
- 34 x 34pt square, game button art (`skin.button[state]` states: normal, hover, pressed, disabled tint `from_white_alpha(90)`) with the glyph centred.
- Preferred glyph: the game's round-arrow refresh art, located at execution time with `cargo ff-art list` / `dump`, added as an `assets/poe2/manifest.toml` entry, drawn at about 18pt (`FLAG_ICON_SIZE`) centred, tinted `pal::BUTTON_TEXT` (disabled: `pal::DISABLED`). Finding it is not a blocker: if no suitable art is found within one short search, ship the fallback.
- Fallback: same square button with caption `R` (capital, 17pt SmallCaps, `pal::BUTTON_TEXT`).
- Tooltip: `Refresh the file lists` (single short line; accessible name `Refresh`, `WidgetInfo::labeled(Button, ..., "Refresh")`).
- Disabled when the game folder is missing (nothing to list); tooltip stays the same.
- Behaviour (locked): re-lists `.filter` and `.mp3`/`.wav` only (`settings::list_filters`, `sound_files::refresh`); no selection change, no write, no edit discarded, does not mark any tab dirty. No status message on success; the dropdown simply shows the new list. If the active filter name has vanished from the folder it stays selected and shown (it can still be created by Apply), exactly as today.

---

## Interaction Contract

**Browse**
- Click opens the native `rfd` folder dialog, titled `Select your Path of Exile 2 folder`, starting at the current folder (or its nearest existing parent, else Documents).
- The dialog is parented to the overlay window (`set_parent` with the eframe window handle) so it appears above the always-on-top overlay; if parenting is unavailable, bring it to the front. Run on a worker thread so the egui frame loop never blocks; while the dialog is open the Browse button is disabled and the overlay does not auto-hide. Cancel changes nothing.
- A pick that equals the current folder does nothing.

**Applying a pick**
- Applies at once and writes `game_folder` to `ff-settings.json` (unknown keys preserved); never marks CONFIG dirty (same class as the filter choice).
- With unapplied rule edits, reuse the **Switch filter?** popup (`Switch and discard` / `Stay`) before anything is written; Stay discards the pick. No new popup, title and buttons unchanged.
- After the pick, re-list filters and sound files from the new folder, then select the filter: the **same name** if present; else the **first `.filter`** found (list order of `list_filters`); if the folder has no `.filter` at all keep the current name as a new file the first Apply creates (existing "absent filter" status applies). `dynamic.filter` is not special.
- A folder pick that makes the missing-folder status disappear shows no extra message.

**Missing folder**
- Status line text: `Game folder not found: <path>` (one line, ellipsis cut, full text on hover when cut, per `ff_status`). Kind Error. Clicking it switches to the Config tab (new click target on the status line, like the jump-to-rule click; carries a tab instead of a `BlockId`).
- Priority in `status_for` (`src/overlay/shell.rs`): directly after `filter_error` and before `syncing`. Resulting order: missing app root, event, grammar gate, damaged markers / load failure, **game folder not found**, sync running, unknown styles, v0.4 notice, absent filter, indicator. It outranks `absent` because it explains it; it yields to an Apply/Reload/Sync event so outcomes are not hidden.
- Config tab in this state: path shown `WARN`, dropdown disabled with `(game folder not found)`, refresh disabled, Browse enabled.

---

## Copywriting Contract

| Element | Copy |
|---------|------|
| Row labels | `Game folder`, `Filter` |
| Browse button | `Browse` |
| Browse tooltip | `Default: <default path>` (env set: `Set by FERRISFILTER_GAME_DIR`) |
| Path tooltip | full path |
| Refresh tooltip | `Refresh the file lists` |
| Refresh fallback caption | `R` |
| Folder dialog title | `Select your Path of Exile 2 folder` |
| Missing-folder status | `Game folder not found: <path>` |
| Dropdown when folder missing | `(game folder not found)` (unchanged) |
| Empty folder (no `.filter`) | existing absent-filter status (unchanged) |
| Destructive actions | none new; the Switch filter? popup is reused as is |
| Primary CTA | `Browse` |

No captions or paragraphs added; every explanation is a tooltip.

---

## Screenshot-mode states (debug `ff --screenshot`, for executor verification)

All under a temp `FERRISFILTER_HOME` and a temp `FERRISFILTER_GAME_DIR` (the new guard requires temp). Use `--tab config`:

1. Normal: temp game dir with 3 filters; Game folder row with a short path, Filter row with dropdown plus refresh button (square art or `R`).
2. Long path: temp dir nested deep so the path is cut with an ellipsis; confirm Browse and the path never overlap.
3. Missing folder: `FERRISFILTER_GAME_DIR` to a nonexistent temp subpath; capture Config tab (warn path, disabled dropdown and refresh) and any tab for the status line `Game folder not found: <path>`.
4. Switch popup on a folder pick: `--popup switch` (existing) to confirm reuse; no new popup flag needed.
5. Empty folder (no `.filter`): dropdown shows the kept current name.
Pixel-check: refresh button 34 x 34pt, 10pt gap to the dropdown, dropdown and button right edge aligned with the Browse button's right edge.

---

## Registry Safety

| Registry | Blocks Used | Safety Gate |
|----------|-------------|-------------|
| none | not applicable | not applicable (no component registry; `rfd` is a crate dependency vetted in plan, game art is extracted from the local install) |

---

## Decisions made by researcher (non-interactive run; author may revise)

1. Section caption `GAME FOLDER FILTER` removed in favour of the two labelled rows (author prefers minimal text).
2. Refresh button is a 34pt square matching `ff_button` height; art glyph drawn on the existing button art; `R` fallback.
3. Refresh has no status message and is disabled when the folder is missing.
4. Browse is disabled while the dialog is open and when `FERRISFILTER_GAME_DIR` is set (with a tooltip saying why).
5. Dropdown tooltip drops "Default: dynamic.filter" because the default is no longer special.
6. Missing-folder status ranks just below the damaged / load failure row and above sync running; events still outrank it.
7. Dialog title `Select your Path of Exile 2 folder`; path shown in warn colour when missing.
8. Dialog parenting via the overlay window handle on a worker thread (exact `rfd` approach left to the executor, per CONTEXT discretion).

---

## Checker Sign-Off

- [x] Dimension 1 Copywriting: FLAG (non-blocking)
- [x] Dimension 2 Visuals: FLAG (non-blocking: no new focal point on the Config tab)
- [x] Dimension 3 Color: PASS
- [x] Dimension 4 Typography: FLAG (non-blocking: line height is the egui default)
- [x] Dimension 5 Spacing: FLAG (non-blocking: existing skin constants off the 4pt grid)
- [x] Dimension 6 Registry Safety: PASS

**Approval:** approved 2026-10-18 (gsd-ui-checker)
