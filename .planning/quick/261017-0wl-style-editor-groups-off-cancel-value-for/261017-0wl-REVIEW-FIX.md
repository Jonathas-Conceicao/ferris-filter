---
phase: quick-261017-0wl
fixed_at: 2026-10-17T00:00:00Z
review_path: .planning/quick/261017-0wl-style-editor-groups-off-cancel-value-for/261017-0wl-REVIEW.md
iteration: 1
findings_in_scope: 7
fixed: 7
skipped: 0
status: all_fixed
---

# Quick 261017-0wl: Code Review Fix Report

**Fixed at:** 2026-10-17
**Source review:** .planning/quick/261017-0wl-style-editor-groups-off-cancel-value-for/261017-0wl-REVIEW.md
**Iteration:** 1

**Summary:**
- Findings in scope: 7 (WR-01, WR-02, WR-03, IN-02, IN-03, IN-05, IN-06)
- Fixed: 7
- Skipped: 0
- Out of scope, left as is: IN-01, IN-04

Gates after the last commit: full `cargo test` green (1490 tests: 1379 lib plus 111 in the integration
and bin targets), `cargo fmt --check` clean, clippy count 126 (the baseline), `tests/fixtures`
unchanged.

## Fixed Issues

### WR-01: Every custom sound is flagged "missing" when the filter folder is unknown or unreadable

**Files modified:** `src/overlay/sound_files.rs`, `src/overlay/style_lines.rs`, `src/overlay/config_view.rs`, `CLAUDE.md`, `CHANGELOG.md`
**Commit:** c5d6c5c
**Applied fix:** `sound_files::list` returns `None` for a missing or unreadable folder.
`SoundFolder` gained `listed`. `sound_menu` takes `listed`. A file that is absent from a folder that
was not listed becomes the new `SoundItem::Unlisted`: a plain dropdown row with no amber flag, and the
box has a neutral tooltip saying the folder could not be listed. New and adjusted tests cover the
listing (`None` vs `Some(empty)`), the cache's `listed`, the menu with an unlisted folder, and the
rendered editor:
- no folder: no flag
- unreadable folder: no flag
- listed empty folder: one flag

CLAUDE.md (STYLE paragraph) and the CHANGELOG entry now say this.

### WR-02: Switching between an alert and a file on a line without a volume changes the audible volume silently

**Files modified:** `src/overlay/body.rs`, `CHANGELOG.md`
**Commit:** 7d06219, 1afed2c (test expectation follow-up)
**Applied fix:** In `sound_intent`, a kind switch writes the shown volume explicitly whenever it
differs from the new kind's game default (50 for an alert, 100 for a file). The edit still rewrites
only the one line.
- `PlayAlertSound 6` + a file now gives `CustomAlertSound "f" 50`.
- `CustomAlertSound "a.mp3"` + an alert now gives `PlayAlertSound 4 100`.

Three existing expectations encoded the old silent change and were updated: the bare-alert-to-file
case, the inline-comment case, and the one-sound-line test's `CustomAlertSoundOptional` case.
**Status:** fixed: requires human verification (logic change in the volume rule)

### WR-03: Test assertion that cannot fail (`err.contains('x')`)

**Files modified:** `src/models/config.rs`, `src/loader.rs`
**Commit:** 37c8ec3
**Applied fix:** The test now asserts ``unknown field `zzbad` `` inside a minimap object and inside
a sound object. A new loader test, `load_styles_reports_an_unknown_field_inside_a_minimap`, checks
that the `styles.json` load error names `S.minimap`, the field and the file.

### IN-02: A listed file name with leading or trailing whitespace is written trimmed

**Files modified:** `src/overlay/sound_files.rs`
**Commit:** 1845e0d
**Applied fix:** The folder listing skips names where `name.trim() != name`. The listing test adds
` boss.mp3` to cover this.

### IN-03: The dropdown's reveal-on-open flag can go stale

**Files modified:** `src/skin/widgets.rs`
**Commit:** 92561c8
**Applied fix:** The bool was replaced by an `(opened_at, last_shown)` pair of frame numbers, handled
by the pure helper `reveal_state`:
- A list that was not drawn on the previous frame starts again, so a state left behind by a list
  that closed while its widget was hidden cannot suppress the next reveal.
- Every pass of the opening frame still reveals the selected row.

Unit test `dropdown_reveal_restarts_after_a_frame_not_drawn`.

### IN-05: The D-06 regression test misses the volume-less and optional variants

**Files modified:** `src/overlay/body.rs`
**Commit:** 1839d5a
**Applied fix:** `single_alert_sound_line_has_no_leftover_row` now also covers `PlayAlertSound 6`
and `CustomAlertSound "uwu.mp3"` (0 leftover rows) and `CustomAlertSoundOptional "x.mp3"` (exactly 1).

### IN-06: Minor wording inconsistency in the `Cancelable` errors

**Files modified:** `src/models/config.rs`
**Commit:** 891e550
**Applied fix:** `visit_str` reports the visitor itself as the expected value, so a wrong string
and a wrong type both read `expected an object or the string "None"`. This is tested for `"none"`
and `3`.

## Not in scope (left as is)

### IN-01: Case-insensitive file match is ASCII-only

**File:** `src/overlay/style_lines.rs:742`
**Reason:** Not in this pass's scope (only IN-02/03/05/06 were requested). It is a one-line change
to `to_lowercase()` comparison if wanted later.
**Original issue:** `eq_ignore_ascii_case` treats `Été.mp3` and `été.mp3` as different files.
Windows treats them as the same, so such a file can be wrongly flagged missing.

### IN-04: Leaving `/` on a `MinimapIcon -1 <colour> <shape>` line ignores that line's own colour and shape

**File:** `src/overlay/body.rs:2738-2741`, `src/overlay/style_lines.rs:559-563`
**Reason:** Not in scope, and it is a behaviour choice rather than a cheap fix. Seeding `shown` from
the inexact cancel line's trailing tokens touches the binding decode and needs a design decision.
**Original issue:** With no memo, leaving `/` falls back to White Circle instead of the colour and
shape the author wrote after `-1`.

---

_Fixed: 2026-10-17_
_Fixer: Claude (gsd-code-fixer)_
_Iteration: 1_
