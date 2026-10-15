---
phase: 21
slug: overlay-configuration-tab
status: approved
reviewed_at: 2026-10-13
shadcn_initialized: false
preset: none
created: 2026-10-13
---

# Phase 21 — UI Design Contract

> Visual and interaction contract for the CONFIG tab: filter selection, named-style editor, tier-cutoff editor, and its own Apply/Revert. Native Rust eframe/egui 0.36 stripe, not a web app. Extends `20-UI-SPEC.md` (and through it 19): tokens, widgets, typography, row anatomy and copy tone are inherited and NOT restated unless changed.
>
> Fully autonomous run. 21-CONTEXT.md decisions are locked. Open points are marked "Decided (autonomous)" and collected at the end.

---

## Design System

| Property | Value |
|----------|-------|
| Tool | none (egui immediate-mode; `theme::poe2_visuals()`) |
| Preset | not applicable |
| Component library | egui 0.36 built-ins (`ComboBox`, `DragValue`, `CollapsingHeader`, `color_edit_button_srgba`) + `src/overlay/widgets.rs` (`header_plate`, `zebra_row`, `circular_toggle`, `arrow_button`, `remove_x_button`, `toggle_chip`, `badge`, `themed_slider`, `number_field`, `colour_swatch`, `rgba_caption`, `chip_list`). Phase 21 reuses these; the only new widget is `style_preview_chip` (below), added to `widgets.rs`. The new view lives in a new module `src/overlay/config_view.rs` beside `rule_list.rs` |
| Icon library | none; text and painted shapes only |
| Font | Cinzel for header plate, tab labels and the three section headers; egui default proportional for all body content |
| Window | fixed 460px `STRIPE_WIDTH`. Usable body width about 424px (460 - 2x8 frame - scrollbar); every control must fit it |

---

## Spacing Scale

Multiples of 4 only. Same scale and exceptions as Phase 20.

| Token | Value | Usage |
|-------|-------|-------|
| xs | 4px | Gap between chips/badges, caption-to-control, between the two lines of a card |
| sm | 8px | Gap between field rows, card inner margin (`Margin::same(8)`), gap between sections' content blocks |
| md | 16px | Gap between sections (Filter / Styles / Tiers); gap between scroll area and footer |
| lg-3xl | 24 / 32 / 48 / 64 | not used |

Exceptions (inherited): 24x24 click targets on painted controls (hit rect only; glyph paint stays 16px); body-scope `interact_size.y` = 24px; collapsed row height `interact_size.y + 4`. Tab strip segments are 28px high (24px target + 4px padding). No new off-scale values.

---

## Typography

No new sizes. Four sizes, two faces, inherited from Phase 20.

| Role | Size | Weight | Line Height |
|------|------|--------|-------------|
| Body (values, buttons, dropdown text, number fields, style names) | 14px | Regular | egui default (about 1.4) |
| Summary / helper / flags / small buttons / error lines | 13px | Regular | egui default |
| Caption (field captions: `ACTION`, `TEXT`, `FONT SIZE`, unit label) | 11px (`.small()`), LABEL_TAN | Regular | egui default |
| Heading (Cinzel: header plate, `RULES`/`CONFIG` tabs at 14px `row_label`, section headers `FILTER`/`STYLES`/`TIERS` at 18px) | 18px / 14px | Cinzel | egui default |

Weights: exactly two (Regular, Cinzel). No bold. Emphasis by colour and position only.

---

## Color

Palette is `theme.rs`, unchanged. 60% `PANEL_GROUND`/`ROW_BASE`/`ROW_ALT`; 30% `HEADER_PLATE` (active tab segment, section banner), `INPUT_FILL` (text/number fields), tan/cream text; 10% accent.

| Role | Value | Usage |
|------|-------|-------|
| Dominant | `PANEL_GROUND` #141414, `ROW_BASE` #0D0D0D / `ROW_ALT` #121212 | Panel and zebra rows (style rows, tier rows alternate) |
| Secondary | `HEADER_PLATE` #3B3326, `INPUT_FILL` #090909, `LABEL_TAN` #E3CD97, `VALUE_TEXT` #FEFEFE, `SECTION_CREAM` #F1E4C2, `TAB_ACTIVE` #B3AF93 | Active tab segment, fields, captions, values, headings, helper text |
| Accent | `WARM_ACCENT` #A36C3C (fills/strokes), `WARM_ACCENT_TEXT` #BF8548 (text) | See reserved list |
| Destructive | same `WARM_ACCENT` | Fill of the four destructive inline confirm buttons only: `Delete style`, `Switch and discard`, `Discard` (Revert), `Overwrite` |
| Disabled | `DISABLED_TEXT` #88856E | Locked built-in `Delete` (rendered as absent, see below), read-only unit label |

Accent reserved for (extends the Phase 20 list; nothing else):
1. `Delete style` confirm button.
2. Validation errors in the Tiers section (`WARM_ACCENT_TEXT` at 13px, inline under the offending row) and Config Apply refusal lines.
3. Unparsable-number hint (`Enter a number`) while mid-edit.
4. **`Config changed on disk` conflict line** (outside edit detected; see Apply).
5. The 4px left-edge stroke on a tier row that currently has an error (a state mark, paired with the text error; never colour-only).
6. The fill of the four destructive inline confirm buttons: `Delete style`, `Switch and discard`, `Discard` (Revert), `Overwrite`.
The CONFIG tab's `Apply` button uses the same fill as the RULES tab's Apply (unchanged button style); no new accent use there.

The style preview chip and colour swatches show real RGBA the filter will emit. That is content, exempt from the palette rule.

State is never colour-only: dirty state has text, errors have text twins, built-in styles carry a `built-in` badge.

---

## Layout

**Focal point (CONFIG tab):** the eye lands first on the STYLES list, specifically the preview chips, because they are the only content-coloured elements on an otherwise dark tan/black tab and the section holds the most rows. The active `CONFIG` tab segment (`HEADER_PLATE` fill) anchors the top; the pinned `Apply` button anchors the bottom. Accent fill appears only on transient confirms and errors, so it never competes with the chips.

### Top-level tab strip

- Sits directly under `header_plate`, above the existing `FILTER` heading block. Two segments, `RULES` and `CONFIG`, full width (each 50% = 212px minus 4px gap), 28px high, Cinzel 14px (`row_label` look).
- Active segment: `HEADER_PLATE` fill, `SECTION_CREAM` text. Inactive: `ROW_BASE` fill, `TAB_ACTIVE` text. Hover lightens text to `SECTION_CREAM`. Selection state is conveyed by fill AND a 2px `SECTION_CREAM` underline under the active segment (so it is not colour-fill-only). Keyboard: Left/Right arrows move between tabs when the strip is focused; Enter/Space activates.
- A small dirty marker: when the **other** tab has unapplied edits, its label shows a trailing ` *` and tooltip `Unapplied changes on this tab`. This is the only cross-tab signal; it prevents losing track of in-memory edits (rules edits and config edits are both held in memory and survive tab switches within the session).
- Switching tabs never discards or applies anything and keeps each tab's scroll position and expanded rows.

### CONFIG tab body (top to bottom)

1. **Status block** (13px): `Config: <clean | N unapplied changes>` plus the out-of-date line (below). Placed first, matching the Phase 20 rule "status above the content".
2. **Scroll area** holding the three sections, `md` (16px) between them, each introduced by a Cinzel 18px `section_header` (`FILTER`, `STYLES`, `TIERS`). Sections are always expanded (no accordion at section level); only individual style rows expand.
3. **Footer (pinned)**: outcome area (single `ScrollArea` capped at 96px, newest first, same component as RULES) above the button row `Apply` | `Revert`, left to right, 8px gap, each button full row height 28px.

### Section: FILTER

A single card (`ROW_BASE`, 8px margin, 4px gap):
- Line 1: caption `GAME FOLDER FILTER`.
- Line 2: `ComboBox` full width (424px) listing every `*.filter` in the game folder, sorted by name, then a separator and the entry `New empty filter…`. The currently active file is selected. The file name is shown without folder path; full path in the hover tooltip. If the game folder cannot be read, the dropdown is disabled and shows `(game folder not found)`.
- Line 3 (13px TAB_ACTIVE): `Saved on this computer. Default: dynamic.filter.`
- `New empty filter…` opens an inline name row in the same card: text field (hint `my-filter.filter`, appended `.filter` if missing) + `Create filter` and `Discard new filter` buttons. Empty name, a name that already exists, or one with path separators disables `Create filter` and shows 13px hint `Enter a new file name` / `That file already exists`. `Create filter` switches to the new empty file (Phase 18 STORE-03 semantics).
- Filter selection is **not** part of Config Apply/Revert: it takes effect immediately on selection and persists to `ff-settings.json` (it is a pointer, not a document edit).

**Switch guard.** If the RULES tab has unapplied edits when the user picks a different filter (or `New empty filter…` → Create), show an inline confirm replacing line 3 in the card, `ROW_ALT` background:
`You have unapplied rule changes. Switching filters will discard them.` with buttons `Switch and discard` (accent fill, destructive) and `Stay`. Selection reverts to the previous file in the dropdown until confirmed. Never silently discarded. Confirm disarms if the selection changes again.

### Section: STYLES

Header line: `STYLES` heading left, `+ ADD STYLE` button right (13px). Below it a helper line (13px TAB_ACTIVE): `Named looks that rules can share. Changes reach rules on the RULES tab's Apply.`

List: zebra rows in **file order** (`zebra_row`, `ROW_BASE`/`ROW_ALT`), not virtualized (about 12 to 25 styles, accordion-bounded). Only one style row is expanded at a time (matches the rules accordion); expanding another collapses the current one.

**Collapsed style row** (height `interact_size.y + 4`, 8px inner margin), left to right:
- Name, 14px `VALUE_TEXT`, min width 96px, ellipsis truncation, full name in tooltip.
- `style_preview_chip` (see below), 120px wide, 24px high.
- Flags region (max 120px, same overflow rule `+N` as rules): `Hide` (13px, TAB_ACTIVE badge) for `action: Hide`; `built-in` badge for built-ins; `new` badge for styles added this session and not yet applied; `edited` badge for modified-since-last-apply.
- Click anywhere on the row toggles expansion. No arrows or enable toggle here (styles are not ordered by precedence and have no enabled state).

**Preview chip** (`style_preview_chip`): a rounded-4 rect filled with the style's background RGBA over a small checkerboard (so alpha is visible), 2px border in the border RGBA, and the sample text `Item Name` in the text RGBA at a font size mapped from the style font size (clamped 12 to 18px so it fits the chip; real size is in the editor). For `Hide` styles the chip is drawn with text and border at 50% alpha plus a 1px diagonal strike line in `DISABLED_TEXT`, and has hover tooltip `Hidden in game; Alt reveals it.` Tooltip on all chips: the three RGBA captions. An accessibility name is set via `widget_info`: `Preview of style <name>`.

**Expanded style editor** (card body below the collapsed row, `ROW_BASE`, 8px margin, 8px between field rows, same anatomy as Phase 20 action rows: caption 11px above or beside, 24px high controls; every control fits 424px):

| Field | Control |
|-------|---------|
| ACTION | Segmented `Show | Hide` (two `toggle_chip`s, 24px high). For Hide styles all other fields remain visible and editable (Phase 17: Hide gets the full set) |
| TEXT / BORDER / BACKGROUND | Each: caption + `colour_swatch` (24x24, checkerboard, opens `color_edit_button_srgba` popup with alpha) + `rgba_caption` text (`R 255 G 121 B 198 A 255`). An `Off` toggle (circular) beside each removes the colour entirely (e.g. `unchanged` has none); when off the swatch is hidden and the caption reads `Not set (game default)`. Turning on seeds: Text/Border `248 248 242 255`, Background `40 42 54 255` |
| FONT SIZE | `Off` toggle + `themed_slider` 1 to 45 + `number_field` 64px. Hint `Readable in game: 18-45` (13px TAB_ACTIVE) outside 18 to 45, as in Phase 20 |
| SOUND | Segmented `None | Alert | Custom`. Alert: ID slider 1 to 16 + number, Volume slider 0 to 300 + number. Custom: file name text field (hint `uwu.mp3`) + Volume slider 0 to 300; caption `The file must be in the game folder.` |
| DROP SOUND | Circular toggle labelled `Silence the default drop sound` (maps to `DisableDropSound`) |
| MINIMAP ICON | `Off` toggle, then three dropdowns on one line: Size (`Large`/`Medium`/`Small`), Colour (12px colour dot + name), Shape. Fits 424px with 8px gaps (each about 130px) |
| BEAM | `Off` toggle, Colour dropdown (dot + name), `Temporary` toggle |
| Row actions | `Delete style` (13px) right-aligned; hidden entirely for built-ins (see Deleting) |

The field set mirrors Phase 20's action controls exactly so users learn one vocabulary. The style editor and the rule's per-action rows must call the same widgets (no second implementation).

**Adding.** `+ ADD STYLE` inserts an inline name row at the top of the list (same card look as New empty filter): text field (hint `Style name`) + `Add style` and `Discard new style`. `Add style` is disabled with 13px hint until the name is non-empty after trim and unique (case-sensitive, matching rule `#ff:style` lookup): `Enter a style name` / `A style with this name already exists`. On `Add style`, the new style is appended at the end of the list (file order), expanded, and scrolled into view, with defaults: action Show, no colours, nothing else set. Name cannot be changed afterwards (rename deferred); an info tooltip on the name in the editor says `Names can't be changed. Add a new style and delete this one.`

**Deleting.**
- Built-ins `S, A, B, C, D, E, F, unchanged, hidden`: no Delete button; the `built-in` badge tooltip says `Built-in styles can't be deleted.`
- Others: `Delete style` arms an inline confirm replacing the button: `Rules using "<name>" keep their current actions but lose their style link on the next Apply.` with `Delete style` (accent fill) and `Keep style`. If any currently loaded rules reference the style, show the count first: `N rules use this style. ...` (count from the in-memory rule set; omit when zero). Confirm disarms when the expanded row changes (Phase 20 convention).
- Deleting only removes the style from the in-memory config; the file changes on Config Apply and rules update on RULES Apply.

### Section: TIERS

Helper line (13px TAB_ACTIVE): `Minimum price for each tier. Items worth at least this much get the tier.` Header caption row (11px LABEL_TAN): `TIER` | `MIN VALUE` | `UNIT`.

One row per tier in order **S, A, B, C, D, E, F** (S on top), `zebra_row`, 32px high (24px control + 8px), each:
- Tier letter: 14px `VALUE_TEXT`, a 24px-wide fixed column, with no colour dot (the letter alone, to avoid colour-only coding).
- Min value: `number_field` text buffer 112px wide (separate String buffer per row, kept apart from the parsed `f64`). Accepts decimals (`2.5`). Buffer text is never rewritten while focused.
- Unit: read-only 13px `DISABLED_TEXT` text (`chaos`, `div`, or `exalt`, from the entry), 64px column, tooltip `Set in tiers.json; not editable here.`
- Right side of the row: error text area (below).

Validation (single source: `ValidatedTiers::try_from`):
- Parse: a buffer that is not a finite number >= 0 shows `Enter a number` (13px, `WARM_ACCENT_TEXT`) under that row and does not write; the last good value stays in the model.
- Order and completeness: after any edit run `ValidatedTiers::try_from` on the parsed set; map `NotAscending { prev, curr, .. }` to inline text under the **curr** row: `Must be higher than <prev> (<prev_min>).` with the left-edge accent stroke on both rows; `MissingTiers` is not reachable from the editor (seven rows always exist) but, if returned, shows in the outcome area. Errors clear as soon as the set validates.
- Because entries are validated per unit group in file order, the display order S to F is presentation only; the row-to-entry mapping is by tier name, never by index.
- While any error exists: Config Apply is disabled (see below), with the disabled-state tooltip `Fix the tier errors first.`

No `Reset` per row. The footer `Revert` covers it.

The Tiers section footer note (13px TAB_ACTIVE): `Only your live tiers.json is edited. The built-in early-league fallback is never changed.`

---

## Apply, Revert and out-of-date indicator

- Footer buttons: `Apply` and `Revert`, 28px high. Both disabled (greyed per egui default `add_enabled`) when there are no unapplied config edits. `Apply` additionally disabled while any tier error exists or any pending name field is invalid. `Revert` shows an inline confirm only when there are edits (copy: `Discard N unapplied config changes?` with `Discard` (accent) and `Keep editing`); it disarms when tab changes.
- **Dirty tracking:** edits count per style (added, deleted, modified) and per tier row; status line reads `Config: 3 unapplied changes` (singular `1 unapplied change`) or `Config: no unapplied changes`. Dirty is computed by value-compare against the last-applied snapshot, so reverting a field by hand clears it.
- **Apply** = validate both files, write both temp files, rename both. Success outcome line: `Applied config: N styles, 7 tiers.` followed by the out-of-date notice. Failure lines name the file and the reason; nothing is renamed and the in-memory edits are kept (`Config not applied: <reason>. Nothing was changed on disk.`).
- **Out-of-date indicator (Decided, autonomous):** after a successful Config Apply, and also on load whenever `styles.json` or `tiers.json` is newer than `dynamic.filter`, a 14px body line (not Cinzel, `VALUE_TEXT`) appears in the status block on BOTH tabs: `Filter out of date: config changed. Press Apply on the RULES tab to regenerate it.` It clears when RULES Apply succeeds. The RULES tab's existing stale indicator copy is unchanged; this line is additional and uses the same stale placement (status block, above the search box on RULES).
- **Outside-edit guard (mtime):** the loaded mtime of `styles.json` and `tiers.json` is remembered. On Config Apply, and on tab focus (checked when the CONFIG tab becomes active), if either file's mtime differs from the remembered value: outcome line in `WARM_ACCENT_TEXT`: `styles.json changed outside the overlay.` (or `tiers.json`, or `Both config files changed outside the overlay.`), and the footer buttons switch to `Reload from disk` (discards in-memory config edits, then reloads) and `Overwrite` (applies anyway, accent fill) with `Keep editing`. Apply never overwrites silently. Same pattern as Phase 18's guard.
- Config edits survive switching between RULES and CONFIG, and are lost on closing the process (accepted v0.4 trade-off, now signalled by the `*` tab marker and the status line).
- After a successful Config Apply the overlay does not touch `dynamic.json`, `dynamic.filter`, `prices.json` or either fallback file.

---

## Interaction rules

- All mutations are collected during render and applied after the loop (inherited `RowAction` pattern); a separate `ConfigAction` enum for this tab.
- Tab/keyboard: Tab moves between controls; sliders respond to arrow keys; popups close on Esc; Enter commits `Add style` and `Create filter` name rows; Esc triggers `Discard new style` / `Discard new filter`. Focus returns to the originating control.
- Adding a style scrolls the new row into view and focuses its first control (ACTION segment).
- Expanded style row height changes feed the same measured-height cache pattern as rules.
- Colour picker edits write u8 sRGBA `Color32` values; no float conversion, so round-trip is exact.
- Config tab uses the same single outcome area for all messages; max 96px, wrapped lines.

---

## Copywriting Contract

| Element | Copy |
|---------|------|
| Tab labels | `RULES`, `CONFIG` (Cinzel is all caps) |
| Section headers | `FILTER`, `STYLES`, `TIERS` |
| Primary CTA (tab) | `Apply` (config) |
| Secondary actions | `Revert`, `+ ADD STYLE`, `Add style`, `Create filter`, `Discard new filter`, `Discard new style`, `Delete style`, `Keep style`, `Keep editing` |
| Filter dropdown caption | `GAME FOLDER FILTER` |
| Filter dropdown extra entry | `New empty filter…` |
| Filter helper | `Saved on this computer. Default: dynamic.filter.` |
| Folder missing | `(game folder not found)` |
| Switch guard | `You have unapplied rule changes. Switching filters will discard them.` / `Switch and discard` / `Stay` |
| Styles helper | `Named looks that rules can share. Changes reach rules on the RULES tab's Apply.` |
| Empty styles state | (cannot be empty: nine built-ins) if somehow zero loaded: `No styles loaded. Revert to reload styles.json.` |
| Name hints | `Enter a style name` / `A style with this name already exists` / `Enter a new file name` / `That file already exists` |
| Rename tooltip | `Names can't be changed. Add a new style and delete this one.` |
| Built-in tooltip | `Built-in styles can't be deleted.` |
| Hide chip tooltip | `Hidden in game; Alt reveals it.` |
| Delete style confirm | `Rules using "<name>" keep their current actions but lose their style link on the next Apply.` (prefixed `N rules use this style. ` when N > 0) / `Delete style` / `Keep style` |
| Tiers helper | `Minimum price for each tier. Items worth at least this much get the tier.` |
| Tier errors | `Enter a number` / `Must be higher than <prev> (<prev_min>).` |
| Tiers footnote | `Only your live tiers.json is edited. The built-in early-league fallback is never changed.` |
| Apply disabled tooltip | `Fix the tier errors first.` |
| Dirty status | `Config: no unapplied changes` / `Config: N unapplied changes` |
| Other-tab marker tooltip | `Unapplied changes on this tab` |
| Apply success | `Applied config: N styles, 7 tiers.` |
| Out-of-date | `Filter out of date: config changed. Press Apply on the RULES tab to regenerate it.` |
| Apply error | `Config not applied: <reason>. Nothing was changed on disk.` |
| Outside edit | `styles.json changed outside the overlay.` / `tiers.json changed outside the overlay.` / `Both config files changed outside the overlay.` with `Reload from disk` / `Overwrite` / `Keep editing` |
| Revert confirm | `Discard N unapplied config changes?` / `Discard` / `Keep editing` |
| Destructive confirmations | Delete style, Revert (with edits), Switch and discard, Overwrite. All inline, accent-fill confirm button, `Keep style` / `Stay` / `Keep editing` beside it. Everything else is non-destructive |

---

## Registry Safety

| Registry | Blocks Used | Safety Gate |
|----------|-------------|-------------|
| shadcn official | none (native Rust UI) | not required |
| Third-party | none | not applicable |

---

## Decided (autonomous)

1. **Tab strip placement and look:** under the header plate, two Cinzel segments, active = `HEADER_PLATE` fill plus a 2px cream underline (non-colour twin); `*` marker on the other tab when it has unapplied edits.
2. **Edits persist across tab switches** within the session (both tabs hold in-memory edits); nothing is lost by switching tabs.
3. **Filter choice applies immediately** and is not part of Config Apply/Revert (it is a pointer persisted to `ff-settings.json`, not a document).
4. **Switch guard** is an inline two-button confirm, never a modal.
5. **One style row expands at a time**; sections themselves are always open (no section accordion).
6. **Optional style fields have an `Off` toggle** so styles like `unchanged` (no colours) are representable without writing placeholder values.
7. **Preview chip** composited over a checkerboard with 2px border and sample text `Item Name`; Hide styles drawn at half alpha with a strike line.
8. **Add-style name row** inline at the list top; new style appended to the end in file order and opened for editing; no rename (tooltip explains workaround).
9. **Delete** hidden for built-ins (badge explains); for others, inline confirm with count of affected rules and the exact consequence from CONTEXT.
10. **Tier rows ordered S to F** top to bottom, no colour dots (avoid colour-only coding); errors attach to the `curr` row of `NotAscending` with an accent edge stroke and text; row-to-entry mapping by tier name.
11. **Tier unit read-only**, shown in `DISABLED_TEXT` with tooltip; the fallback pair is never touched (footnote states it).
12. **Out-of-date copy** (`Filter out of date: config changed. Press Apply on the RULES tab to regenerate it.`) shows on both tabs after Config Apply and when config files are newer than the filter.
13. **mtime guard** offers `Reload from disk` / `Overwrite` / `Keep editing`; never silently overwrites.
14. **Shared widgets:** the style editor reuses the Phase 20 action-row widgets; only `style_preview_chip` is new.

---

## Pre-Populated From

| Source | Decisions used |
|--------|----------------|
| 21-CONTEXT.md | RULES/CONFIG tabs, Filter/Styles/Tiers sections, filter dropdown + New empty filter, `ff-settings.json`, switch guard, style list + preview chips, editable fields, add/delete/built-in rules, no rename, tier rows S to F with read-only unit and `ValidatedTiers` inline errors, atomic two-file Apply, filter out of date after Apply, Revert, mtime guard |
| 20-UI-SPEC.md | tokens, spacing exceptions, typography, palette and accent list, action-control vocabulary, outcome area, accordion, confirm patterns, copy tone |
| `src/overlay/theme.rs` | palette constants, `section_header` 18px Cinzel, `row_label` 14px, `ROW_SUMMARY_SIZE` 13px |
| `src/overlay/widgets.rs` | existing widget inventory |
| `src/models/tiers.rs` | `TierEntry`, `Unit` (chaos/div/exalt), `ValidatedTiers`, `TierValidationError::{MissingTiers, NotAscending}` |
| Decided autonomously | everything under "Decided (autonomous)" |

---

## Checker Sign-Off

- [ ] Dimension 1 Copywriting: PASS
- [ ] Dimension 2 Visuals: PASS
- [ ] Dimension 3 Color: PASS
- [ ] Dimension 4 Typography: PASS
- [ ] Dimension 5 Spacing: PASS
- [ ] Dimension 6 Registry Safety: PASS

**Approval:** pending
