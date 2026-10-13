---
phase: 20
slug: full-grammar-block-editing
status: approved
reviewed_at: 2026-10-12
shadcn_initialized: false
preset: none
created: 2026-10-12
---

# Phase 20 — UI Design Contract

> Visual and interaction contract for the data-driven rule body: every condition and every action editable, styles linked to rules. Native Rust eframe/egui 0.36 stripe, not a web app. Extends `19-UI-SPEC.md` (tokens, widgets, row anatomy, copy tone are inherited and NOT restated unless changed).
>
> Fully autonomous run. Every item under "Decisions made autonomously" was chosen without user input, guided by the author's stated priority: an intuitive, accessible UI with pickers for categorical values, sliders for numbers and structured controls for structured values. 20-CONTEXT.md decisions are locked and used as given.

---

## Design System

| Property | Value |
|----------|-------|
| Tool | none (egui immediate-mode; `theme::poe2_visuals()`) |
| Preset | not applicable |
| Component library | egui 0.36 built-ins (`Slider`, `DragValue`, `ComboBox`, `color_edit_button_srgba`, `CollapsingHeader`) + `src/overlay/widgets.rs` (`header_plate`, `zebra_row`, `circular_toggle`, `arrow_button`). New shared widgets live in `widgets.rs`: `field_row`, `remove_x_button`, `chip_list` |
| Icon library | none; text labels and painted shapes only (Cinzel has no glyph coverage for symbols) |
| Font | Cinzel only for header plate and `FILTER` section header; egui default proportional for everything in the rule body |
| Window | fixed 460px `STRIPE_WIDTH`; never resizes horizontally. Usable body width inside an expanded row is about 424px (460 - 2x8 frame - scrollbar) and every control must fit it |

---

## Spacing Scale

Multiples of 4 only. Phase 19's off-scale values are retired in code touched by this phase.

| Token | Value | Usage |
|-------|-------|-------|
| xs | 4px | Gap between chips, gap between flag badges, caption-to-control gap |
| sm | 8px | Gap between condition/action rows, between body groups, frame inner margin |
| md | 16px | Gap between the list and the footer; gap between the CONDITIONS and ACTIONS groups |
| lg-3xl | 24 / 32 / 48 / 64 | not used |

Exceptions (inherited, documented):
- Row height `interact_size.y + 4` for collapsed rows (virtualization).
- Painted toggle/arrow glyph size 16px stays, but its **click target is padded to 24x24** (hit rect only; paint size unchanged).
- Every off-scale `add_space(6.0)` (app.rs, rule_list.rs) and `add_space(10.0)` is snapped to 8.0 / 4.0. Frame inner margin becomes `Margin::same(8)` in both the collapsed row (shrink 8) and the body, so toggle and body left edges align (fixes 19-UI-REVIEW spacing finding).
- Minimum interactive target in the body: 24px high (`interact_size.y` raised to 24 inside the body scope). Slider rail height 16px.

---

## Typography

No new sizes. Four sizes, two faces.

| Role | Size | Weight | Line Height |
|------|------|--------|-------------|
| Body (values, buttons, dropdown text, exact-number fields) | 14px | Regular | egui default (about 1.4) |
| Row summary / flags / helper notes | 13px | Regular | egui default |
| Caption (group captions `CONDITIONS`, `ACTIONS`, field captions) | 11px (`.small()`) | Regular, LABEL_TAN | egui default |
| Heading (Cinzel `FILTER`, header plate) | 18px | Cinzel | egui default |

Rules:
- Condition keyword names are shown in 14px body, not Cinzel and not `.small()`.
- Small action buttons (`Remove`, `Duplicate`, `Delete`, chip remove) use 13px, not 11px (carry-over: 11px buttons were too small). 11px is for captions only.
- The stale indicator (`Filter out of date...`) moves from Cinzel `row_label` to 14px body (carry-over fix).
- Weights: exactly two, Regular plus the Cinzel face. Emphasis is colour and position, never bold.

---

## Color

Palette is `theme.rs`, unchanged. 60% `PANEL_GROUND`/`ROW_BASE`/`ROW_ALT`; 30% `HEADER_PLATE`, `INPUT_FILL`, tan/cream text; 10% accent.

| Role | Value | Usage |
|------|-------|-------|
| Dominant | `PANEL_GROUND` #141414, `ROW_BASE`/`ROW_ALT` | Panel and zebra rows; condition/action rows sit on `ROW_BASE`, alternate with `ROW_ALT` |
| Secondary | `HEADER_PLATE` #3B3326 (selected segment, style-owned banner fill), `INPUT_FILL` #090909 (text/number fields), `LABEL_TAN` (captions), `VALUE_TEXT` (values) | Controls |
| Accent | `WARM_ACCENT` #A36C3C for fills/strokes; **text use lightens to `WARM_ACCENT_TEXT` #BF8548** (about 5.6:1 on #0D0D0D, passes AA at 13px; carry-over fix) | See reserved list |
| Destructive | same `WARM_ACCENT` | Delete confirms only |
| Disabled | `DISABLED_TEXT` #88856E | Style-owned (locked) action values, disabled operators |

Accent reserved for (extends 19 list, nothing else):
1. Delete / Remove-economy confirm buttons.
2. Error flags and failure lines: `grammar error`, `can't enable`, Apply refusals.
3. `moved across Continue` warning.
4. **Unknown style warning** (`Unknown style "X"`), new.
5. **Invalid value hint** on a field the user is mid-editing (e.g. an unparsable number), new.
The `Remove style` button is NOT accent (it is a normal button; it is reversible by re-picking a style).

The colour swatch of a colour action shows the real RGBA the filter will use. That is content, not chrome, so it is exempt from the palette rule.

State is never colour-only: every state has a text twin (flag words, `from style` badge, `(none)` text).

---

## Layout: expanded row body

Replaces the Phase 19 body. Order, top to bottom, `sm` (8px) between groups, `md` (16px) before ACTIONS:

1. `NAME` row (Phase 19/15 pattern, unchanged: toggle + text field in one `horizontal`).
2. `STYLE` group (new, see Styles).
3. `CONDITIONS` group: header line `CONDITIONS` caption left, `+ condition` button right. Below, one **condition row** per condition line, in file order. Empty: helper text `No conditions: this rule matches every item.` (13px, TAB_ACTIVE).
4. `ACTIONS` group: caption left, `+ action` button right. Disposition selector (`Show | Hide | Minimal` segmented + `Continue to later rules` checkbox, Phase 19 unchanged) is the first line of this group. Then one **action row** per action line.
5. Row actions line: `Duplicate`, `Delete` (Phase 19 unchanged, 13px).

The whole body is a plain vertical flow. Condition and action rows are NOT individually collapsible. Long value controls (class picker, base-type list, mod list) are the only parts that grow vertically; each is capped (see below) so one row can never exceed about 260px and push the rest off screen.

**Focal point:** in an expanded body the eye lands first on the STYLE banner when the rule is styled (it explains why actions are locked), otherwise on the first CONDITIONS card. Nothing else in the body uses the accent fill.

### Shared row anatomy (condition and action rows)

Two-line card when the value control is wide; single line when it fits. Card: `ROW_BASE` fill, 8px inner margin, 4px between lines.

Line 1 (always): `[keyword label ......] [operator ▾] [remove X]`
- Keyword label: 14px `VALUE_TEXT`, plain-English name where useful with the exact keyword as tooltip (see Keyword names). Truncates with ellipsis, min width 120px (reserve, never squeezed to nothing).
- Operator dropdown: only for conditions, only operators valid for the keyword (`Ops::Compare`: `=`, `==`, `!=`, `<`, `<=`, `>`, `>=`, filtered per value shape). Label width fixed 104px. Plain-English labels:
  `At least` (>=), `At most` (<=), `More than` (>), `Less than` (<), `Exactly` (==), `Exactly (=)` (shown only when the existing line already uses bare `=`, so the operator is never normalized; see Decisions), `Not` (!=), `Matches` (bare/`=` on lists, i.e. substring/prefix), `Matches exactly` (== on lists).
- Remove: `remove_x_button`, painted X, 24x24 target, tooltip `Remove this condition` / `Remove this action`, accessible name set via `widget_info`. No confirm (deleting one line is cheap and Revert exists).

Line 2: the value control, full width of the card. For short values (toggle, dropdown, one number) the control is placed on line 1 right of the operator instead and line 2 is omitted.

An **existing operator is never normalized** (PARSE-04): the dropdown shows the file's operator as selected even if it is the bare form; the bare form is listed as its own entry only when already present.

### Keyword names

Display name = keyword as written (CamelCase split with spaces: `Item Level`, `Base Energy Shield`, `Unidentified Item Tier`), with the exact grammar keyword in the tooltip. Class, BaseType, Rarity, Sockets are shown as-is. This keeps names searchable by someone reading GGG docs while staying readable.

---

## Condition picker (`+ condition`)

- Button `+ condition` opens a popup (egui `popup_below_widget` / `ComboBox` style) anchored under the button, width = body width, max height 320px, internally scrollable. Top: a search field (hint `Find a condition…`, autofocus), then groups as `CollapsingHeader`s (all open while searching, first group open otherwise). Selecting an item adds the row at the **end of CONDITIONS**, with a sensible default value, opens it for editing and closes the popup. Esc closes.
- Keywords already present stay selectable (the same keyword may repeat; e.g. `ItemLevel >= 70` and `ItemLevel <= 80`).
- Group membership comes from a static table keyed by keyword name in the overlay module (presentation only). The Phase 16 keyword table remains the source of truth for which conditions exist and their value shapes: any condition keyword in that table not in a group falls into **Other** so none is ever unreachable (guarded by a test, same spirit as PITFALLS 10).
- Groups, in order:

| Group | Conditions |
|-------|-----------|
| Item | Class, BaseType, Rarity, ItemLevel, DropLevel, Quality, Width, Height, StackSize |
| Defences | BaseArmour, BaseEvasion, BaseEnergyShield, BaseWard, BaseDefencePercentile |
| Sockets | Sockets, SocketGroup, LinkedSockets |
| Status | Identified, Corrupted, Mirrored, FracturedItem, Replica, Foulborn, Imbued, Vestigial, TwiceCorrupted, CorruptedMods, UnidentifiedItemTier |
| Mods | HasExplicitMod, HasImplicitMod, HasEnchantment, AnyEnchantment, HasInfluence, HasVaalUniqueMod, IsVaalUnique, EnchantmentPassiveNode, EnchantmentPassiveNum, ArchnemesisMod, AlternateQuality, Exceptional |
| Endgame | AreaLevel, WaystoneTier, MapTier, GemLevel, GemQualityType, TransfiguredGem, MemoryStrands, ZanaMemory, and the legacy map flags |
| Other | everything else from the keyword table |

- Keywords the PoE2 docs do not honour (PoE1-only such as `LinkedSockets`, `ElderMap`) stay listed (no read-only conditions, author bar) but carry a 13px suffix `(PoE 1)` in the picker only.

## Value controls by value shape

All numbers: **slider + exact-number field side by side** (`Slider` takes remaining width, `DragValue` 64px wide). Both write the same value; typing outside the slider range is allowed (slider clamps visually, the field does not). An unparsable intermediate entry shows the hint `Enter a whole number` in `WARM_ACCENT_TEXT` and does not write.

| Value shape | Control | Detail |
|-------------|---------|--------|
| `Int` | Slider + number | Per-keyword range, below |
| `Bool` | Circular toggle + text `Yes` / `No` | Writes `True` / `False`. Operator dropdown limited to `Is` (bare) / `Is not` (!=, only if already present) |
| `BoolOrName` (TransfiguredGem) | `Any` / `Named` segmented; `Named` reveals a text field | |
| `Rarity` | Multi-select chip row: `Normal` `Magic` `Rare` `Unique` as toggle chips, each 24px high. Operator dropdown (`Exactly`, `At least`, `At most`, `More than`, `Less than`) | Single selected chip: scalar syntax `Rarity >= Magic`. 2+ chips: renders `Rarity Normal Magic Rare` list form and the operator is fixed to `Matches` (hint text `Several rarities match any of them`). Zero chips: not written, warning hint `Pick at least one rarity` |
| `QuotedList` Class | Phase 14/15 grouped class picker (groups from `jewelleryClasses`/config, never hardcoded), in a body capped at 200px high with its own scroll; selected classes show as chips above it (first 6, then `+N more`) | Empty selection collapses to removing the line, never emitting a bare keyword |
| `QuotedList` BaseType and other quoted lists | `chip_list`: text field + `+ ADD`, Enter commits, chips with per-entry `x`, quoted-paste splitting. Chips wrap, capped at 160px high with scroll; counter caption `14 items` | Not validated (PoE2 warns at load) |
| `CountThenNames` (HasExplicitMod, HasEnchantment) | Row 1: `At least [n] of these:` with a 0-6 slider+number for count (operator dropdown on line 1 also applies to the count). Row 2: `chip_list` for the mod names | Renders `HasExplicitMod >=1 "A" "B"` |
| `Enum` (HasInfluence, GemQualityType) | `ComboBox` listing the enum values from the keyword table | |
| `Sockets` / `SocketGroup` | Structured: slider+number for count (1-6) plus an optional colour strip of up to six slots: `R` `G` `B` `W` `A` per slot via a small dropdown per slot, `+ socket` / `Remove` below. Renders `Sockets >= 5GGG` shape; plain count when no colours are chosen | Slots with `Any` colour are omitted from the string |
| `ImportPath` | Not a condition, not editable here (Import is a block-level directive) | |

### Numeric slider ranges (per keyword; all values still enterable beyond)

| Keyword | Range | Notes |
|---------|-------|-------|
| ItemLevel, DropLevel, AreaLevel, GemLevel | 1-100 | |
| Quality | 0-30 | corrupt/rune can exceed 20 |
| Sockets / LinkedSockets / SocketGroup count | 0-6 | |
| StackSize | 1-5000 | logarithmic slider |
| WaystoneTier, MapTier | 1-17 | |
| UnidentifiedItemTier | 1-5 | |
| BaseArmour, BaseEvasion, BaseEnergyShield, BaseWard | 0-1000 | logarithmic slider |
| BaseDefencePercentile | 0-100 | |
| Width / Height | Width 1-2, Height 1-4 | |
| CorruptedMods, MemoryStrands, EnchantmentPassiveNum, Has*Implicit | 0-10 | |
| any other Int | 0-100 | default |

Slider default value on add: keyword-specific midpoint of the table's sensible default (ItemLevel 65, AreaLevel 65, Quality 1, Sockets 1, StackSize 1, WaystoneTier 1, defences 1, others range minimum).

Defence picker: v0.4's three checkboxes are gone; Defences group rows replace them. `BaseEnergyShield > 0` (existing `min: 1` form) is shown as operator `More than` value `0`.

---

## Action rows and `+ action`

- `+ action` opens the same style of popup (no search needed, about 14 entries), listing every action keyword from the table in groups: **Colours** (SetTextColor, SetBorderColor, SetBackgroundColor), **Text** (SetFontSize), **Map** (MinimapIcon, PlayEffect), **Sound** (PlayAlertSound, PlayAlertSoundPositional, CustomAlertSound, CustomAlertSoundOptional), **Drop sound** (DisableDropSound, EnableDropSound, DisableDropSoundIfAlertSound, EnableDropSoundIfAlertSound). An action already present is shown disabled with suffix `(added)`: each action is single-instance in a block, and adding a duplicate would be a no-op semantically.
- Action row anatomy is the shared one without an operator dropdown.

| Action | Control |
|--------|---------|
| SetTextColor / SetBorderColor / SetBackgroundColor | Swatch button (24x24, shows RGBA over a checkerboard) opening `color_edit_button_srgba` popup with alpha, plus caption `R 255 G 121 B 198 A 255`. Hover: tooltip with the same text. The swatch for Background defaults to Dracula bg `40 42 54 255` on add; text/border default to white `248 248 242 255` |
| SetFontSize | Slider 1-45 + number; below 18 or above 45 shows hint `Readable in game: 18-45` (13px TAB_ACTIVE, not an error) |
| MinimapIcon | Three dropdowns in one line: Size (`Large` 0, `Medium` 1, `Small` 2), Colour, Shape. Colour dropdown entries show a 12px colour dot + name. A `Hide icon` option writes size `-1`. |
| PlayEffect | Colour dropdown (same list as above) + `Temporary` toggle |
| PlayAlertSound / PlayAlertSoundPositional | Sound ID slider 1-16 (+ number) and Volume slider 0-300 (+ number). Note: ID dropdown also accepts the `None` form if the table allows it |
| CustomAlertSound / Optional | File name text field (hint `uwu.mp3`) + Volume slider 0-300. Caption: `The file must be in the game folder.` |
| Drop-sound toggles | Single circular toggle labelled `On` plus the keyword; the optional `False` argument is represented by toggle off (`Disable… False` rendered only when the file already had it) |

---

## Styles on rules (STYLE-03, EDIT-06)

### Without a style
`STYLE` group shows caption `STYLE`, a dropdown `(none)` plus every style from `styles.json` (entries show the style name and a 12px swatch of its text colour), and helper text `Apply a style to set this rule's look from your Styles tab.` Picking one writes `#ff:style <name>`, sets actions from the style, and the rule is now styled.

### With a known style (the "from style" state)
- STYLE group renders as a banner: `HEADER_PLATE` fill, 8px margin, 4px gap:
  - Line 1: `Style: <name>` (14px SECTION_CREAM), a **style dropdown** to change it, and a normal button **`Remove style`** (13px, right-aligned).
  - Line 2 (13px TAB_ACTIVE): `This rule's look comes from the style. Edits to the style reach it when you press Apply. Hand edits here are overwritten.`
- Style-owned action rows (every action the style defines, plus the disposition) are shown **locked**: values rendered read-only in `DISABLED_TEXT`, a 13px badge `from style` (LABEL_TAN, bordered) in place of the remove X, controls non-interactive, with a tooltip `Set by style "<name>". Remove the style to edit it.` The disposition selector is locked with the Phase 19 note.
- `+ action` is hidden while a rule is styled, to avoid actions that vanish or conflict on the next Apply. Replaced by the banner copy. (Decision 3.)
- `Remove style` is **non-destructive and unconfirmed**: it deletes the `#ff:style` line only, keeps the current actions as the rule's own now-editable actions, and shows status `Style removed. The actions are now yours to edit.` One level of in-session Undo is not added; Revert covers it.
- Collapsed row flag `style wins` (Phase 19) is renamed `styled: <name>` (truncating to 12 chars with ellipsis, tooltip carries the full text), because the flag now means more than Show/Hide.

### Unknown style
- STYLE banner with `WARM_ACCENT_TEXT`: `Unknown style "<name>"`; body text `No style with this name exists, so this rule keeps its current actions and nothing is rewritten on Apply.` Controls: dropdown `Pick a style…` and `Remove style` button. Actions stay **editable** (not locked), since no rewrite will happen.
- Collapsed row gets flag `unknown style` (WARM_ACCENT_TEXT, tooltip same as body). It is a warning, NOT a grammar error: it never blocks Apply.
- Footer status line summarises after Apply and on load: `N rules use unknown styles (first: rule 12).` Clicking the line jumps to the first offending rule (same mechanism as the grammar gate).

### Apply feedback
After a successful Apply that rewrote styled rules, the outcome line adds `Updated N styled rules.` (omitted when zero). The stale indicator treats a changed `styles.json` as "unapplied changes" (copy unchanged).

---

## Carry-over fixes from 19-UI-REVIEW (in scope where Phase 20 touches the code)

| Issue | Contract |
|-------|----------|
| Flags squeeze the label | Row layout: toggle (24) + label min width **140px**, flags region max **120px**, arrows (2 x 24). Flags beyond the budget collapse into one `+N` badge (tooltip lists all flags). Label truncates with ellipsis, full text tooltip. |
| Footer has no height cap | Messages (list status, Apply message, sync status, grammar gate, unknown-style summary) merge into ONE **outcome area**: a `ScrollArea` capped at **96px** height, newest first, each line wrapped. Buttons and the economy band stay outside it, pinned |
| Error-red contrast | Text uses `WARM_ACCENT_TEXT` #BF8548 at 13px (see Color) |
| Status below FILTER heading | Layout order becomes: header plate, `FILTER` heading + rule count, status block (root/path/state/stale), search, list. Status block is no longer between heading and the search box |
| Off-scale spacing | See Spacing Scale; 6 and 10 snapped to 4/8 |
| Small hit targets | 24px targets on toggle, arrows, X buttons, chips |
| Stale confirm | Delete/economy confirm disarms when the expanded row changes (already fixed 96da66b; keep) |
| Accessibility names | Painted controls call `widget_info` with label text so screen readers and egui accesskit expose them |

## Interaction rules

- All mutations are `RowAction`s collected during render and applied after the loop (Phase 19 pattern). Each edit rewrites only the edited lines (PARSE-04).
- Every edit emits valid grammar. A list control with zero entries removes its line instead of emitting a bare keyword. A row whose value is momentarily invalid (empty text field) keeps the previous written value and shows the hint; it does not write.
- Keyboard: Tab moves between controls; sliders respond to arrow keys; popups close on Esc; Enter commits chip lists. Focus returns to the row's control after the picker closes.
- Adding a condition/action scrolls the row into view and focuses its primary control.
- Expanded row height changes feed the existing measured-height cache (`expanded_h`); the first-frame correction stays a one-frame repaint.
- Row order inside the block mirrors file order. Conditions and actions are not reordered in this phase (only add/remove); order is irrelevant to match semantics.

---

## Copywriting Contract

| Element | Copy |
|---------|------|
| Primary CTA (body) | `+ condition`, `+ action` |
| Picker search hint | `Find a condition…` |
| Empty conditions | `No conditions: this rule matches every item.` |
| Empty actions | `No actions: matching items keep the game's look.` |
| Empty rarity | `Pick at least one rarity` |
| Invalid number hint | `Enter a whole number` |
| Font size hint | `Readable in game: 18-45` |
| Custom sound caption | `The file must be in the game folder.` |
| Style banner | `Style: <name>`; `This rule's look comes from the style. Edits to the style reach it when you press Apply. Hand edits here are overwritten.` |
| Style-owned badge/tooltip | `from style` / `Set by style "<name>". Remove the style to edit it.` |
| Remove style | button `Remove style`; status `Style removed. The actions are now yours to edit.` |
| Apply style helper | `Apply a style to set this rule's look from your Styles tab.` |
| Unknown style | `Unknown style "<name>"`; `No style with this name exists, so this rule keeps its current actions and nothing is rewritten on Apply.` |
| Unknown summary | `N rules use unknown styles (first: rule 12).` |
| Apply with styles | `Updated N styled rules.` |
| Remove condition/action tooltip | `Remove this condition` / `Remove this action` |
| Error state | Parser/IO errors keep Phase 19 copy; invalid-field hints always say what to enter |
| Destructive confirmation | Delete rule and Remove economy: unchanged from Phase 19. Removing a single condition/action/style is not destructive (reversible via Revert), no confirm |

---

## Registry Safety

| Registry | Blocks Used | Safety Gate |
|----------|-------------|-------------|
| shadcn official | none (native Rust UI) | not required |
| Third-party | none | not applicable |

---

## Decisions made autonomously

1. **Condition rows are always visible cards** (not off-switches): add via `+ condition`, remove via X. Matches CONTEXT; picker is searchable because the table has about 65 conditions.
2. **Grouped picker with an `Other` safety group** plus a test that every condition keyword in the table is reachable; PoE1-only keywords stay listed with `(PoE 1)` suffix, honouring "no read-only conditions".
3. **Styled rules hide `+ action`** and lock style-owned rows; unknown-style rules stay fully editable, since nothing will be rewritten for them.
4. **`Remove style` keeps current actions** as the rule's own and needs no confirm.
5. **Slider ranges** per keyword as tabulated; log scale for StackSize and defences; exact-number field always allows out-of-range.
6. **Rarity** is a chip multi-select that renders scalar syntax for one, list syntax for several. Deliberate refinement of CONTEXT's "dropdown with multi-value support": chips are the multi-value dropdown laid flat (4 fixed values fit the 424px body), meeting the same intent with one click per value.
7. **Sockets** structured as a count slider plus optional per-slot colour dropdowns.
8. **Operator labels** plain English; the bare `=`/missing-operator form is only offered when the file already uses it, so no normalization happens.
9. **`style wins` flag renamed `styled: <name>`**; `unknown style` is a new non-blocking flag.
10. **Single capped outcome area** (96px) for footer messages; **WARM_ACCENT_TEXT** #BF8548 added for text contrast (fills unchanged).
11. **24px hit targets** with unchanged 16px glyph paint; 11px reserved for captions only.
12. Conditions/actions are not reorderable in this phase.

---

## Pre-Populated From

| Source | Decisions used |
|--------|----------------|
| 20-CONTEXT.md | picker + operator + value-type controls, grouped class picker, quoted list entry, rarity multi-value, sliders, colour picker with alpha, font 1-45, MinimapIcon dropdowns, PlayEffect Temp, sounds, drop-sound toggles, `#ff:style` semantics, locked/visible style-owned actions, unknown-style reporting, no-normalization rule |
| 19-UI-SPEC.md | tokens, widgets, typography, row anatomy, accordion, delete confirm, copy tone |
| 19-UI-REVIEW.md | label min width, footer cap, contrast, status order, off-scale spacing, hit targets |
| `src/filter/keywords.rs` | condition/action keyword list and value shapes |
| Decided autonomously | everything under "Decisions made autonomously" |

---

## Checker Sign-Off

- [ ] Dimension 1 Copywriting: PASS
- [ ] Dimension 2 Visuals: PASS
- [ ] Dimension 3 Color: PASS
- [ ] Dimension 4 Typography: PASS
- [ ] Dimension 5 Spacing: PASS
- [ ] Dimension 6 Registry Safety: PASS

**Approval:** pending
