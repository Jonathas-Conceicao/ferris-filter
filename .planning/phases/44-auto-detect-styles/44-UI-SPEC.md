---
phase: 44
slug: auto-detect-styles
status: approved
reviewed_at: 2026-10-23
shadcn_initialized: false
preset: none
created: 2026-10-23
---

# Phase 44 - UI Design Contract

> egui desktop overlay, not a web app: no CSS tokens or shadcn. Everything reuses the existing PoE2 skin. This phase adds one button and one status message.

## Design System

| Property | Value |
|----------|-------|
| Tool | none (egui + `src/skin/`) |
| Preset | not applicable |
| Component library | `skin::widgets` (`ff_button`, `ff_status` / `StatusKind`) |
| Icon library | none new (game art in `assets/poe2/`) |
| Font | Fontin SmallCaps / Regular (existing skin fonts) |

## Spacing, Typography, Color

Nothing new. The button inherits the skin: pt spacing already used by the Custom heading row (4/8 gaps), Fontin button caption at the size `ff_button` draws, skin palette (stone button plate, gold caption). No new palette constant, no accent colour, no destructive colour (the action is non-destructive and fully undoable by Revert).

## Component Contract: Auto Detect button

| Property | Contract |
|----------|----------|
| Widget | `skin::widgets::ff_button`, same as **+ Add style** |
| Label | `Auto Detect` |
| Width | 112pt, so it fits beside the 124pt **+ Add style** (the ~660pt window has room) |
| Placement | CONFIG tab, Styles section, the **Custom** heading row (`config_view.rs`, the `ui.horizontal` with `group_heading(ui, "Custom")`). Inside the existing `right_to_left` layout, added **after** `+ Add style` so it sits immediately **left** of it: `Custom ...... [Auto Detect] [+ Add style]`, 8pt gap |
| Enabled | Only when a filter is loaded (the Rules model exists) and is not in a load-failure state. Otherwise greyed (`add_enabled`, via `ff_button_ex` with `enabled=false`) |
| Not blocked by | Fallback price source, an open style draft, or other pending edits |
| Tooltip (enabled) | `Name the styling already in this filter: tag rules that match a style and add the rest as custom styles.` |
| Tooltip (disabled) | `No filter loaded.` |
| Click | One batch of pending edits, nothing written to disk: add pending styles, re-sync the rules model's style set, tag rules |
| Probe | register `auto-detect` rect next to `add-style` for the screenshot tests |

No popup, no confirmation (nothing destructive, nothing written until Apply).

## How the result appears

- **New styles**: rows appended to the Custom table after existing custom styles, in order of first appearance in the file, named `Custom Style N` (smallest free N, case-insensitive vs all pending names). They look and behave like any custom style row (Visibility switch, Icon, Preview, Order arrows, expandable editor, rename, delete). Rows are collapsed, not auto-expanded. The Custom table header shows if it was hidden.
- **Tagged rules**: the RULES tab shows them as ordinary styled rules (style picker on the preset, `styled` chip with the style's minimap icon / name, `# ff:style <id> <name>` written on Apply). No special "new" badge.
- **Unapplied state**: Config tab ` *` and Rules tab ` *`, the `Unapplied changes` indicator and the Quit popup count it, exactly like other edits. No new indicator.
- **After a CONFIG-only Revert**: tagged rules show the existing amber `unknown style` flag; no new UI.
- **Idempotence**: a second click with nothing new changes nothing and sets no unapplied state.

## Copywriting Contract

Status line (`ff_status`, one line, event message that stays until the next edit):

| Case | Kind | Copy |
|------|------|------|
| Styles added and/or rules tagged | `StatusKind::Ok` | `Auto Detect: 3 styles, 41 rules tagged` |
| Nothing to do | `StatusKind::Info` | `Auto Detect: nothing new` |

Singular forms: `1 style`, `1 rule tagged`. When only existing styles matched (0 new): `Auto Detect: 0 styles, 12 rules tagged`. Nothing is shown for skipped rules (economy, already tagged, `can't enable`, leftover Custom); explanations stay in the button tooltip.

| Element | Copy |
|---------|------|
| Primary CTA | `Auto Detect` |
| Empty state | none new (status `Auto Detect: nothing new`) |
| Error state | none: the action cannot fail (pure in-memory); a disabled button covers the no-filter case |
| Destructive confirmation | not applicable |

## Registry Safety

| Registry | Blocks Used | Safety Gate |
|----------|-------------|-------------|
| none | none | not applicable |

## Judgement calls

1. **Button width 112pt, label `Auto Detect`**: matches the author's wording and keeps the heading row on one line next to the 124pt Add style.
2. **Left of + Add style**: Add style keeps its right-edge position (muscle memory); the new button is the secondary one.
3. **Disabled with no filter loaded** (rather than a "nothing new" message): there is nothing to read, and a greyed button with a tooltip is quieter than a status line.
4. **Not disabled by Fallback / drafts / pending edits**: detection reads only rules plus pending styles, so none of these conflict.
5. **Status kinds**: Ok for a change, Info for "nothing new" (a no-op is not a warning).
6. **New rows collapsed, no highlight**: terse UI; the Custom table and the status count are enough, and the user can expand any row.
7. **No confirmation popup**: all edits are pending and revertible per tab (CONTEXT: Revert matrix accepted).
8. **Tooltip carries all explanation**, per the "not texty" overlay rule; no caption text added to the section.
