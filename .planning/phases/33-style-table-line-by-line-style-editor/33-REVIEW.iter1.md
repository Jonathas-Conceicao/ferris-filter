---
phase: 33-style-table-line-by-line-style-editor
reviewed: 2026-10-16T17:41:38Z
depth: standard
files_reviewed: 10
files_reviewed_list:
  - src/overlay/app.rs
  - src/overlay/body.rs
  - src/overlay/config_view.rs
  - src/overlay/screenshot.rs
  - src/overlay/widgets.rs
  - src/skin/colour_picker.rs
  - src/skin/editor.rs
  - src/skin/minimap.rs
  - src/skin/mod.rs
  - src/skin/popup.rs
findings:
  critical: 2
  warning: 3
  info: 5
  total: 10
status: issues_found
---

# Phase 33: Code Review Report

**Reviewed:** 2026-10-16T17:41:38Z
**Depth:** standard
**Files Reviewed:** 10
**Status:** issues_found

## Narrative Findings (AI reviewer)

## Summary

I reviewed the phase diff (`33a41e9..HEAD -- src`) and the code it calls: `grammar::encode`
for `Value::Rgba`, `body::rgba_edit` / `values_edit`, `ConfigState::edit_style` / `add_style`,
`StyleSet::validate`, egui 0.36 `Popup` / `Memory` popup lifecycle and the app's
`handle_config_actions`.

What checks out:
- An open but untouched picker emits nothing. Each live edit is compared with `current`.
- Escape restores the opening value.
- The style dirty count is computed by comparing values, so a live edit that returns to the
  original value leaves CONFIG clean.
- Picker state is keyed by the trigger's popup id. Revert, Reload and expand/collapse bump the
  epoch, which orphans that state.
- The custom sound file check matches the loader's `InvalidCustomSound` rule.
- The screenshot hooks (`request_open`, the `ff-colour-picker-shot` block, `shot_step`) are all
  behind `cfg(debug_assertions)`.
- Delete, rename and reorder still go through `rename_drafts` keyed by id and through
  name-based actions on unique names.

Two defects break the picker's stated semantics:
- On an **unset** style slot, the colour the picker opens on cannot be committed.
- **Escape** on a 3-token rule colour line does not restore the original bytes.

There are also smaller problems:
- The editor's "switch back" memory is lost in some paths.
- A pre-existing toggle bug collapses a newly added style.

## Critical Issues

### CR-01: An unset style slot cannot take the colour the picker opens on (Use colour, the selected quick pick and a matching hex are all no-ops)

**File:** `src/skin/colour_picker.rs:519-526, 536-540, 643-658` (with `src/overlay/config_view.rs:1167-1188`)

**Issue:** For an unset slot (`current == None`), `colour_popup` seeds the working colour from
`seed`. The style editor passes Dracula Foreground `[248,248,242]` as the Text and Border seeds and
Dracula Background `[40,42,54]` as the Background seed. Those are exactly two of the quick picks.
The seed colour shows in the picker, but nothing commits it:
- **Use colour** only sets `res.done`. `done` never emits a `PickerEdit`, so the popup closes and
  the slot stays `(none)`.
- The matching quick pick is drawn as `selected`, and `if r.clicked() && !selected` ignores the
  click.
- Typing the same hex or R/G/B value is dropped by `v != rgb`.

So a user who opens an empty Text slot, sees the off-white seed and presses Use colour (or clicks
the highlighted Foreground tile) gets no edit. To set the seed colour, they must pick some other
colour first and then come back. The tests never cover Use colour or a selected quick pick on an
unset slot: `use_colour_closes_without_an_edit` only uses a set slot.

**Fix:** Treat an explicit commit on an empty slot as a Set of the working colour:
```rust
// colour_picker.rs, quick picks: let a click on the selected tile through when the slot is empty
// (pass `slot_empty: bool` into ff_colour_picker, or report `res.picked = true` and decide in colour_popup)
if r.clicked() && (!selected || slot_empty) { ...; changed = true; }

// colour_popup, after the escape / none / changed chain:
} else if (res.done || res.changed) && current.is_none() {
    let mut now = st.hsva.to_srgba_unmultiplied();
    if !mode.alpha { now[3] = 255; }
    out = Some(PickerEdit::Set(now));
}
```
Add a test: open with `current = None`, click `use`, expect `Set(seed)`.

### CR-02: Escape on a 3-token rule colour line rewrites it as 4 tokens and leaves the filter unapplied

**File:** `src/skin/colour_picker.rs:636-642`, `src/overlay/widgets.rs:411-416`, `src/overlay/body.rs:2402-2412`, `src/overlay/grammar.rs:557-562`

**Issue:** Live edits change the line's form, and the Escape restore reads `had_alpha` from the
**current** line, not from the original one:
1. A line `SetTextColor 1 2 3` opens with `opening = Some([1,2,3,255])`.
2. Dragging Alpha to 128 emits `Set([1,2,3,128])`. `rgba_edit` encodes this as `1 2 3 128`
   (`had_alpha || rgba[3] != 255`), so the line now has 4 values.
3. Pressing Escape emits `Set([1,2,3,255])`. `rgba_edit` now computes
   `had_alpha = entry.values.len() >= 4 == true` and writes `1 2 3 255`.

The result never matches the original bytes. `RulesState` computes `dirty` from
`doc.serialize() != applied_text` (rules.rs:870), so the filter stays "unapplied":
- Apply stays enabled.
- The ` *` tab marker shows.
- Quit asks for confirmation.

The next Apply then rewrites a line the user explicitly reverted. This breaks the project rule
that "an edit rewrites only the edited value's lines" and that disable/enable is byte-exact, and
it contradicts the picker's documented "Escape returns the value the picker opened with".

**Fix:** Make Escape restore the original line rather than re-encode a value. Two options:
- When the popup opens, capture the 3-or-4-token form in `colour_chip`, and pass `had_alpha`
  through to `rgba_edit`:
```rust
// widgets.rs colour_chip: remember the opening token count with the popup state
let opened_had_alpha_id = popup_id.with("had-alpha");
// on open: insert_temp(opened_had_alpha_id, entry_had_alpha)
// body.rs: rgba_edit(id, entry, n, had_alpha_at_open)
//   -> Value::Rgba { rgba: new, had_alpha: had_alpha_at_open }
```
- Alternatively, have `rgba_edit` write the 3-token form whenever `new[3] == 255` and the line
  was 3-token in the applied text.

Add a test: 3-token line, alpha edit, Escape, then `serialize() == applied_text`.

## Warnings

### WR-01: Adding a style collapses it instead of expanding it (expand toggled twice)

**File:** `src/overlay/config_view.rs:1040-1045`, `src/overlay/app.rs:1940-1946`

**Issue:** On commit, `style_draft_row` sets `cfg.expanded = Some(name)` itself. After the frame,
the `ConfigAction::AddStyle` handler calls `self.config_ui.set_expanded(&name)`. `set_expanded` is
a **toggle**: because `expanded` already equals `name`, it sets `expanded = None`. The new custom
style is scrolled to but shown collapsed, so the user has to click it again to reach the editor.
This defeats the "+ Add style opens its editor" flow that the line-by-line editor depends on. The
two lines predate this phase, but the flow now lives in the rewritten draft row, and no test
covers the view and the handler together.

**Fix:** Let only one side expand, and do not use the toggle in the handler:
```rust
// config_view.rs style_draft_row: drop `cfg.expanded = Some(name.clone());`
// app.rs AddStyle handler:
self.config_ui.expanded = Some(name.clone());
self.config_ui.delete_armed = None;
self.config_ui.epoch = self.config_ui.epoch.wrapping_add(1);
```

### WR-02: Sound "switch back" memory loses the custom file name when switching through an Alert

**File:** `src/overlay/config_view.rs:1477-1491, 1433-1451`

**Issue:** The memory is only written when the choice becomes None (`sel == 0`), and it is only
read when the style has no sound (`style.sound.or_else(recall)`). Two paths lose the custom file:
- **Custom to Alert to Custom:** Custom `boss.mp3` becomes Alert 3, which builds
  `Sound { id: Some(3), custom: None }` and drops the file. Switching back to Custom then finds
  `base.custom == None` and writes `uwu.mp3`.
- **Custom to None to Alert to Custom:** this loses the file the same way, because the remembered
  sound is ignored once `style.sound` is `Some`.

So the switch-back memory this phase added for minimap, beam and sound does not hold for the sound
file, which is the only free-text value in the editor.

**Fix:** Remember the custom file name whenever a custom sound is replaced, and prefer it when
switching to Custom:
```rust
if let Some(s) = &style.sound && s.custom.is_some() && sel != SOUND_CUSTOM {
    remember(ui, name, "sound-file", epoch, s.custom.as_ref().unwrap());
}
// in sound_for_choice(SOUND_CUSTOM, ..): base.custom.or(recalled_file).unwrap_or(SOUND_FILE_DEFAULT)
```

### WR-03: Turning Font size off and on again replaces the style's font with 32

**File:** `src/overlay/config_view.rs:1201-1206`

**Issue:** `StyleEdit::Font(on.then_some(32))` hard-codes 32 when the check is turned back on. A
style with font 37 (S, A, B) that is toggled off and on again ends up at 32 and stays dirty.
Minimap, beam and sound all restore their last value through `remember`/`recall`; font is the only
optional field that does not, so a quick off/on silently changes the style.

**Fix:**
```rust
if check.changed() {
    let next = if on {
        Some(recall::<u8>(ui, name, "font", epoch).map_or(32, i64::from))
    } else {
        if let Some(f) = style.font { remember(ui, name, "font", epoch, &f); }
        None
    };
    edit(actions, name, StyleEdit::Font(next));
}
```

## Info

### IN-01: `colour_swatch_rgb` and `rgb_caption` are dead in production code

**File:** `src/overlay/widgets.rs:483-498`
**Issue:** The style editor now uses `colour_slot`, and the only remaining callers of
`colour_swatch_rgb` (and therefore of `rgb_caption`) are tests. They are `pub`, so the compiler
does not warn about them. The trailing `.filter(|c| *c != rgb)` also repeats a check that
`colour_chip` already makes.
**Fix:** Remove both, along with their tests, or mark them `#[cfg(test)]` if the tests should stay.

### IN-02: Styles-table columns overflow the row below about 394pt plus padding

**File:** `src/overlay/config_view.rs:557-596`, `1266-1270`
**Issue:** `table_columns` holds Name at 110 and Preview at 100 or more, but nothing makes the
total fit `width`. Below `2*ROW_PAD + 394` the Order arrows are drawn past the row's right edge,
outside the row background and the click rect. `tile_size` similarly clamps to at least 14pt, so
the 12 beam tiles plus Temp overflow on narrow widths. The window is fixed at 660pt, so this is
latent today.
**Fix:** Clamp the last column to `width - pad`, or let `name` drop below `NAME_MIN` as a last
resort. Add a test at a narrow width that asserts `order.x + order.w <= width - pad`.

### IN-03: Editor memory and drafts are keyed by style name, not id

**File:** `src/overlay/config_view.rs:1106-1108`, `1496`, `822`
**Issue:** `memo_id`, the sound-file draft id and the row `push_id` all use `name`:
- A rename loses the remembered None-values.
- Within one epoch, renaming another custom style to a freed name inherits the old style's
  memory.

Everywhere else this phase keys by id (`rename_drafts`).
**Fix:** Key these on `state.style_id(name).unwrap_or(name)`.

### IN-04: Invalid or emptied custom sound file text is silently discarded on focus loss

**File:** `src/overlay/config_view.rs:1508-1535`
**Issue:** Text that is bad (contains `"` or a control character) or empty is never committed.
On focus loss the draft is removed and the field snaps back to the old file without any message.
The red outline exists only while the field is focused.
**Fix:** Keep the draft, and its red outline, until it becomes valid or the row collapses, or put
a short status hint ("File name not changed") when the draft is dropped.

### IN-05: Unused `skin` parameter in `beam_line`

**File:** `src/overlay/config_view.rs:1363-1372`
**Issue:** `let _ = skin;` silences an unused parameter.
**Fix:** Drop the parameter from `beam_line` and from its call site.

---

_Reviewed: 2026-10-16T17:41:38Z_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
