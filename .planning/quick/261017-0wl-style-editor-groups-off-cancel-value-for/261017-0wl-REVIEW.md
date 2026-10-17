---
phase: quick-261017-0wl
reviewed: 2026-10-17T00:00:00Z
depth: standard
files_reviewed: 16
files_reviewed_list:
  - src/emitter.rs
  - src/loader.rs
  - src/models/config.rs
  - src/overlay/app.rs
  - src/overlay/body.rs
  - src/overlay/config_state.rs
  - src/overlay/config_view.rs
  - src/overlay/mod.rs
  - src/overlay/rules.rs
  - src/overlay/screenshot.rs
  - src/overlay/sound_files.rs
  - src/overlay/style_lines.rs
  - src/skin/widgets.rs
  - src/styled.rs
  - CLAUDE.md
  - CHANGELOG.md
findings:
  critical: 0
  warning: 3
  info: 6
  total: 9
status: issues_found
---

# Quick 261017-0wl: Code Review Report

**Reviewed:** 2026-10-17
**Depth:** standard (cross-file where needed)
**Files Reviewed:** 16
**Status:** issues_found

## Summary

Scope: `git diff 9e02730..HEAD` for commits 65be6c8, 054a09e and 0e6d628. Commits c420cd8, 2fa7c3b and 8fa0c6e were left out.

The six locked rules in CONTEXT.md are met:

- **Rule 1 (groups):** one zebra index per group, and off folds the group to its check line.
- **Rule 2 (`/` cancel tile):** the dependent pickers are greyed. The colour row and the dropdown stay clickable, as the PLAN's "`/` semantics while on" section specifies.
- **Rule 3 (sound picker):** the dropdown lists Alert 1-16 and then the folder's files. There is no free-text field, and `CustomAlertSoundOptional` is never newly written.
- **Rule 4 (icons):** the Icon column and the collapsed-row chip draw an icon only for `Cancelable::Set`. The preview has no icon.
- **Rule 5 (`styles.json`):** existing files stay byte-identical. The new fields use `skip_serializing_if`, and `Cancelable` delegates to `T` through `MapAccessDeserializer`, so field errors and their positions survive.
- **Rule 6 (regression test):** `single_alert_sound_line_has_no_leftover_row` is present.

Other points checked and found sound:

- `generate`, the economy section and the styled rewrite all go through `style_action_lines`.
- Screenshot mode safety is unchanged: the new `--dropdown` request is `cfg(debug_assertions)` only and reads a temp folder.
- The folder listing is read-only, has no per-frame I/O, and keeps only names that pass `safe_file_name`. No path separators can reach `CustomAlertSound "<file>"` from the picker.
- The targeted lib tests pass (they were run in the temp target dir).

No blockers. The three warnings:

1. The missing-file flag is misleading when the folder is unknown or unreadable.
2. A kind switch between an alert and a file silently changes the audible volume.
3. One test assertion can never fail.

## Warnings

### WR-01: Every custom sound is flagged "missing" when the filter folder is unknown or unreadable

**File:** `src/overlay/style_lines.rs:870-886` (with `src/overlay/sound_files.rs:31-33`, `src/overlay/app.rs:1362-1367`)

**Issue:** `sound_files::list` returns an empty `Vec` both for an empty folder and for a folder that is missing or unreadable. `set_dir(ctx, None)` also caches `dir: None`, which happens when there is no game folder (`sound_dir()` returns `None`). In all of these cases `sound_menu` turns every file sound into `SoundItem::Missing`.

For example, the CONFIG editor then flags the shipped `S` and `A` styles (`uwu.mp3`) in amber, with the tooltip `uwu.mp3 is not in the filter folder (unknown); the game plays no sound until it is there`. The check cannot know this, so the claim is false, and the amber caption plus flag on every file style is alarm noise. CLAUDE.md's statement ("a file the style or rule names that is not in the folder ... shows in amber") holds only when the listing succeeded.

**Fix:** Keep listing failures apart from empty folders, and do not flag when nothing is known:

```rust
// sound_files.rs
pub struct SoundFolder { pub dir: Option<PathBuf>, pub files: Vec<String>, pub listed: bool }
pub fn list(dir: &Path) -> Option<Vec<String>> { let read = std::fs::read_dir(dir).ok()?; ... Some(files) }
// set_dir: let (files, listed) = dir.as_deref().and_then(list).map_or((vec![], false), |f| (f, true));

// style_lines.rs, sound_menu caller
let missing_known = folder.listed;
// draw the Warn row / flag_icon only when `missing_known`; otherwise add the file as a plain Option
```

### WR-02: Switching between an alert and a file on a rule line without a volume changes the audible volume silently

**File:** `src/overlay/body.rs:3057-3085` (`sound_intent`)

**Issue:** A volume is written only when `s.vol != binding.sound_shown`, but the two kinds have different game defaults: 50 for `PlayAlertSound`, 100 for `CustomAlertSound` (`ALERT_DEFAULT_VOLUME` / `CUSTOM_DEFAULT_VOLUME`).

- A rule with `PlayAlertSound 6` shows Volume 50. Picking a folder file passes `vol: 50` (`sound_for_item` keeps it), which equals `sound_shown`, so `volume = None` and the line becomes `CustomAlertSound "boss.mp3"`. The game plays it at 100, and on the next frame the slider jumps to 100.
- The reverse switch, `CustomAlertSound "a.mp3"` to an alert, writes `PlayAlertSound k` and halves the volume.

The keep-no-volume logic predates this task. The new folder dropdown puts this kind switch on the main path (it is now the only way to pick a file).

**Fix:** On a kind switch, write the shown volume explicitly whenever it differs from the new kind's default:

```rust
let kind_switch = bound_custom != file.is_some();
let new_default = if file.is_some() { CUSTOM_DEFAULT_VOLUME } else { ALERT_DEFAULT_VOLUME };
let volume = if s.vol != binding.sound_shown || (kind_switch && s.vol != new_default) {
    Some(i64::from(s.vol))
} else {
    binding.sound_volume
};
```

Add a test that `PlayAlertSound 6` followed by picking a file yields `CustomAlertSound "f" 50`.

### WR-03: Test assertion that cannot fail (`err.contains('x')`)

**File:** `src/models/config.rs:2674`

**Issue:** `cancelable_keeps_field_errors_and_refuses_other_strings` asserts `err.contains('x')` to show that the unknown-field error survives `Cancelable`. serde's message for any unknown field is ``unknown field `<f>`, expected one of ...``, and "expected" always contains an `x`. Even a generic `invalid type: ..., expected an object or the string "None"` contains `x`. The test therefore does not prove that a bad field name inside a `Cancelable` object is still reported, which was one of the review's focus points.

**Fix:**

```rust
let json = style_json(r#""minimap":{"size":1,"color":"Pink","shape":"Circle","zzbad":1},"beam":null,"sound":null"#);
let err = serde_json::from_str::<Style>(&json).unwrap_err().to_string();
assert!(err.contains("unknown field `zzbad`"), "{err}");
```

Also consider one loader-level test that the `styles.json` load error for such a file still names the style and file.

## Info

### IN-01: Case-insensitive file match is ASCII-only

**File:** `src/overlay/style_lines.rs:742`

**Issue:** `eq_ignore_ascii_case` treats `Été.mp3` and `été.mp3` as different files. Windows treats them as the same file, so a style naming the lower-case variant is flagged missing even though the game would find it.

**Fix:** Compare `x.to_lowercase() == f.to_lowercase()` (as the listing's own sort already does).

### IN-02: A listed file name with leading or trailing whitespace is written trimmed

**File:** `src/overlay/body.rs:3021`, `src/overlay/body.rs:2453`

**Issue:** `sound_intent` / `custom_sound_edit` `trim()` the picked name. A folder file ` boss.mp3` (Windows allows a leading space) is listed, but the line is written as `CustomAlertSound "boss.mp3"`. The next frame flags it missing.

**Fix:** Have `sound_files::list` skip names where `name != name.trim()`. Alternatively, stop trimming names that come from the listing, since the free-text field that needed the trim is gone.

### IN-03: The dropdown's reveal-on-open flag can go stale

**File:** `src/skin/widgets.rs:998-1004`, `src/skin/widgets.rs:1031-1034`

**Issue:** `id.with("shown")` is removed only when the same dropdown is drawn while closed. Suppose the popup closes while the widget is not drawn: the group is toggled off with the list open, the row is folded, or the tab is switched. The flag then survives, and the next open does not scroll the selected or Filter-folder row into view.

**Fix:** Also clear the flag when `opened` is detected (`!was_open && open_now`), or store the frame number of the last draw instead of a bool.

### IN-04: Leaving `/` on a `MinimapIcon -1 <colour> <shape>` line ignores that line's own colour and shape

**File:** `src/overlay/body.rs:2738-2741`, `src/overlay/style_lines.rs:559-563`

**Issue:** For the inexact cancel form the trailing tokens are kept in the file but never used. With no memo, `shown` falls back to `default_minimap()` (White Circle), so clicking Pink writes `MinimapIcon 0 Pink Circle` instead of using the shape the author wrote.

**Fix:** Optionally seed `shown` from the trailing tokens when they decode to a valid size, colour and shape.

### IN-05: The D-06 regression test misses the volume-less and optional variants

**File:** `src/overlay/body.rs:7516-7560`

**Issue:** The cases covered are `PlayAlertSound 6 200`, Positional, `None`, and a file with a volume. The forms most likely to regress the exact/inexact split are not covered:

- `PlayAlertSound 6` (no volume): expected 0 leftover rows.
- `CustomAlertSound "uwu.mp3"` (no volume): expected 0 leftover rows.
- `CustomAlertSoundOptional "x.mp3"`: expected exactly 1 leftover row, since it is inexact by design.

**Fix:** Add these three sources to the first loop, with the expected counts above.

### IN-06: Minor wording inconsistency in the `Cancelable` errors

**File:** `src/models/config.rs` (`Cancelable::deserialize`, `expecting` vs `invalid_value`)

**Issue:** `expecting` says `an object or the string "None"`, while `visit_str` says `"None" or an object`. The meaning is the same but the wording differs, which makes users' error reports harder to grep.

**Fix:** Use one constant string for both.

---

_Reviewed: 2026-10-17_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
