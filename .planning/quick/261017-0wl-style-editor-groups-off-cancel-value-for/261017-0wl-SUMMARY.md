---
phase: quick-261017-0wl
plan: 01
status: complete
subsystem: overlay style editor, styles model, emitter
tags: [style-editor, styles-json, emitter, sound, overlay]
requires: [quick-261016-qi7 shared style_lines]
provides:
  - Cancelable<T> style fields (off / cancel / value) for minimap, beam, sound
  - sound.positional, enableDropSound in styles.json
  - grouped style_lines UI shared by CONFIG and Custom rules
  - sound_files: cached read-only filter-folder sound listing
  - skin::widgets::ff_dropdown_items (sectioned dropdown)
  - screenshot flag --dropdown sound
affects: [generate, economy section, styled-rule rewrite, CONFIG Styles, RULES Custom STYLE]
tech-stack:
  added: []
  patterns: [hand-written serde visitor for "None"-or-object, egui ctx temp cache, one zebra index per group]
key-files:
  created:
    - src/overlay/sound_files.rs
  modified:
    - src/models/config.rs
    - src/emitter.rs
    - src/loader.rs
    - src/styled.rs
    - src/overlay/style_lines.rs
    - src/overlay/body.rs
    - src/overlay/config_state.rs
    - src/overlay/config_view.rs
    - src/overlay/rules.rs
    - src/overlay/app.rs
    - src/overlay/mod.rs
    - src/overlay/screenshot.rs
    - src/skin/widgets.rs
    - CLAUDE.md
    - CHANGELOG.md
decisions:
  - Cancelable<T> serialises Set as T's object and Cancel as the string "None"; deserialised via a visitor (visit_str "None" / visit_map -> T) so field-level errors (unknown colour, unknown field) keep their message
  - New Style/Sound fields use skip_serializing_if (written only when set) so existing styles.json files re-serialise byte-identically
  - Exact rule-line readings - bare `MinimapIcon -1`, `PlayEffect None`, bare `PlayAlertSound None`, `PlayAlertSoundPositional <1-16>`, `DisableDropSound`/`EnableDropSound` (bare/True); `MinimapIcon -1 <c> <s>`, `PlayAlertSound None <vol>`, `PlayAlertSoundPositional None`, `... False` drop lines are inexact (shown + listed as leftover)
  - `DisableDropSound False` now reads as Normal (inexact) and `EnableDropSound False` as Silent (inexact); a drop-sound pick rewrites the bound line to the bare keyword
  - Drop sound tick defaults to Silent; Alert sound tick defaults to Alert 1 vol 100; Minimap tick to 0 White Circle; Beam to White
  - Sound files matched to the folder listing case-insensitively (Windows file names); only .mp3/.wav (no evidence for .ogg)
  - Sound folder = parent of the selected filter's path (filter_target), else the game folder; App sets a sound_dir_stale flag on startup / reset_filter_view (reload + switch) and lists on the next frame where the egui ctx is available
  - The sectioned dropdown scrolls its selected row into view on the first open frame (so the Filter folder section is visible); ff_dropdown / ff_dropdown_ex behaviour unchanged
  - Minimap colour line sizes its tiles for 13 slots so the shapes line (indented one tile under the colours) fits on one row
metrics:
  duration: ~2h40m
  completed: 2026-10-17
  tasks: 3
  commits: 3
---

# Quick 261017-0wl: style editor groups (off / cancel / value), folder sound picker, Positional

The shared style editor now draws Minimap, Beam, Alert sound and Drop sound as one-band groups with an on/off check, a `/` cancel tile (`MinimapIcon -1` / `PlayEffect None` / `PlayAlertSound None`), Large/Medium/Small sizes, a Positional toggle, a `[Silent][Normal]` drop switch and a folder-sourced sound dropdown; `styles.json`, `generate`, the economy section and the styled-rule rewrite store and emit the new states, and both backends (CONFIG `StyleEdit`, lossless Custom-rule lines) drive the same UI.

## Commits

| Task | Commit | Message |
|------|--------|---------|
| 1 | 65be6c8 | feat(styles): off / cancel / value for minimap, beam, sound and drop sound |
| 2 | 054a09e | feat(overlay): style editor groups with off / cancel / value and a folder sound picker |
| 3 | 0e6d628 | docs(quick-261017-0wl): style editor groups, folder sound picker, screenshot dropdown flag |

(An unrelated `8fa0c6e docs(uat): ...` commit landed on the branch between Task 1 and Task 2; not mine, untouched.)

## What was built

- **Model (D-05):** `Cancelable<T> { Set(T), Cancel }` with hand-written serde; `Style.minimap/beam/sound: Option<Cancelable<_>>`; `Sound.positional`; `Style.enable_drop_sound`; `DropSound { Silent, Normal }` with `Style::drop_sound` / `set_drop_sound`; validation refuses positional + custom (`AmbiguousSound`) and disable + enable (`ConflictingDropSound`); range checks only on `Set`.
- **Emitter:** `style_action_lines` emits `PlayEffect None`, `MinimapIcon -1`, `PlayAlertSound None`, `PlayAlertSoundPositional id vol`, `EnableDropSound` in the D-03 canonical order; generate / economy / styled rewrite share it.
- **Rule backend (body.rs):** reads/writes the new states, lossless (an intent rewrites only its bound line, adds one line to an empty family, removes on off). Positional toggle flips the keyword in place keeping volume form and trailing comment; leaving Cancel writes the full line with volume. `CustomAlertSoundOptional` is never written (an existing one keeps its keyword while the kind stays a file).
- **UI (style_lines.rs, D-01..D-04):** groups with one zebra index per group, memo keys `minimap`/`minimap-set`/`beam`/`beam-set`/`sound`/`sound-set`/`drop`; greyed dependent pickers under `/`; the free-text file field, `SOUND_OPTIONS`, `SOUND_CUSTOM`, `SOUND_FILE_DEFAULT`, `FILE_TIP`, `BAD_FILE_TIP`, `INVALID_RED`, `SoundMemo`, `switch_sound`, `sound_for_choice` removed; replaced by pure `sound_menu` / `sound_for_item`.
- **sound_files.rs:** read-only, non-recursive listing of `.mp3`/`.wav` (case-insensitive, names passing `safe_file_name`), cached in egui ctx; `set_dir` on startup / reload / filter switch, `refresh` when the dropdown opens; no per-frame I/O.
- **skin::widgets:** `ff_dropdown_items` with `DropItem::{Option, Header, Warn}` (Warn rows amber + warning flag), sharing a `dropdown_shell` with `ff_dropdown_ex` (signature unchanged); debug-only `request_dropdown_open` / `take_dropdown_request`.
- **Icons (D-04):** CONFIG Icon column and the collapsed-row chip draw an icon only for `Set`; the label preview stays icon-free.
- **Screenshot:** `--dropdown sound` (`DROPDOWNS`, parse + error tests), requested from `shot_step` like `--picker`.

## Tests

- Total: **1487 passed** (lib 1376, was 1357 + new; integration suites unchanged).
- New/updated: Cancelable round-trip + field errors + `"none"` refusal; validate positional/drop conflicts; emitter canonical order for cancel/positional/enable; styled rewrite of the new lines passes the grammar gate and is idempotent; loader assert (no `positional`/`enableDropSound` in the committed set); rule backend exact/inexact readings, cancel intents (bound/absent/already), leaving Cancel, positional flip, drop sound states; **D-06 regression `single_alert_sound_line_has_no_leftover_row`** (also covers positional, `PlayAlertSound None`, a file, and a sound just added through the intent; a test-only `ed-leftover` probe counts drawn leftover rows); CONFIG group tests (off = one line, tick seeds/restores, `/` greys shapes/size/Temp/Volume/Positional, colour from Cancel restores, Large/Medium/Small, Silent/Normal on one line, missing-file flag, positional disabled for a file); RULES UI tests (colour over `MinimapIcon -1`, `/` writes it back, group checks add/remove one line, positional flip); `config_and_rule_editors_draw_the_same_lines` (identical ordered probe tags + idle); `off_groups_draw_only_their_check`; `sound_files` listing/cache tests; untouched-rule idle/byte-identical test extended with cancel / positional / missing-file sources.

## Verification

- `cargo test`: all green (1487).
- `git diff --exit-code -- tests/fixtures`: clean (generate_snapshot / artifact_guard green).
- `cargo fmt --check`: clean.
- clippy: **126** warnings at HEAD before Task 1 (re-verified baseline; the plan's 122 was stale) and **126** after each task.
- Committed blobs are LF.
- Eyeballed screenshots (temp `FERRISFILTER_HOME`/`USERPROFILE` under the scratchpad, never the real game folder): CONFIG `--expand-style kite`, RULES `--expand 1/2`, and `--dropdown sound` showing Alert 1-16, the Filter folder header, `boss.wav`, `uwu.mp3`, and a missing `gone.mp3` in amber with the flag (`notes.txt` excluded).

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Clippy baseline is 126, not 122**
- Found during: pre-Task-1 baseline check. Used the re-verified 126; no new warnings added.

**2. [Rule 1 - Bug] Minimap shapes wrapped to a second row**
- Found during: Task 2 (`minimap_colour_and_shape_keep_the_other_fields`, `expanded_editor_draws_no_frame_fill`).
- Issue: in egui's wrapping row, `available_width()` does not shrink after the leading `add_space`, so the indented shapes line sized 12 tiles for the full width and wrapped.
- Fix: the colour line sizes tiles for 13 slots and the shapes line reuses that size; whole-line greying uses `ui.disable()` instead of an `add_enabled_ui` scope.

**3. [Rule 2 - UX correctness] Sectioned dropdown reveals its selection on open**
- Found during: Task 3 screenshot: the list opened scrolled to the top, hiding the Filter folder section and the current file.
- Fix: `ff_dropdown_items` scrolls the selected row into view on the first open frame (tracked per popup id); `ff_dropdown` / `ff_dropdown_ex` unchanged.

**4. [Plan detail] Sound folder wiring via a stale flag**
- `App::new` has no egui context, so instead of calling `set_dir` directly the app sets `sound_dir_stale` at startup and in `reset_filter_view` (reload and filter switch both go through it) and lists on the next frame.

**5. [Plan detail] style_lines.rs touched in Task 3**
- The `--dropdown sound` request is consumed in the sound group (opens the popup by its persistent id and refreshes the listing), so `style_lines.rs` is part of the Task 3 commit.

### Semantics changes (intended by the locked design, noted for review)

- `DisableDropSound False` (previously shown exactly as "unticked") now reads as Normal and is listed as an inexact leftover; picking Silent rewrites it to bare `DisableDropSound` (previously `DisableDropSound True`).
- `MinimapIcon -1`, `PlayEffect None`, `EnableDropSound` and `PlayAlertSoundPositional` lines are no longer leftover rows; they show exactly on their group.

## Known Stubs

None.

## Threat Flags

None beyond the plan's register (T-0wl-01..04 mitigated: listing passes `safe_file_name` and `sound_intent` re-checks; `Cancelable` accepts only `"None"`; listing is read-only `read_dir`; cached, no per-frame I/O).

## Unfinished

Nothing. Final screenshots are left to the orchestrator; STATE.md / ROADMAP.md not touched (per instructions).

## Self-Check: PASSED

- FOUND: src/overlay/sound_files.rs
- FOUND: commits 65be6c8, 054a09e, 0e6d628

## Review fixes

One fix pass over `261017-0wl-REVIEW.md` (report: `261017-0wl-REVIEW-FIX.md`); 0 critical, 3 warnings, 6 info.

- `c5d6c5c` **WR-01:** a sound file is flagged missing only when the filter folder was actually listed; an unknown or unreadable folder gives a neutral tooltip.
- `7d06219`, `1afed2c` **WR-02:** a volumeless sound line switching kind (alert <-> file) writes the shown volume explicitly unless it equals the new kind's game default, so loudness is kept.
- `37c8ec3` **WR-03:** the unknown-field test asserts the real error; new loader test names `S.minimap`.
- `891e550` IN-06, `1845e0d` IN-02, `92561c8` IN-03, `1839d5a` IN-05.
- Left: IN-01 (ASCII-only case-insensitive match), IN-04 (leaving `/` on `MinimapIcon -1 <colour> <shape>` ignores that colour/shape).

Verification: `cargo test` 1490 green, `cargo fmt --check` clean, clippy 126 (baseline re-measured at 9e02730 = 126 with the same count command; the 122 in earlier notes is stale), `tests/fixtures` untouched.

## Screenshots (orchestrator, after review fixes)

`shots/` (debug `ff.exe --screenshot`, temp home with `uwu.mp3`, `chime.wav`, `notes.txt` in the game folder):
`rule-all-groups-on.png`, `rule-minimap-off.png`, `rule-cancel-values.png`, `rule-missing-file.png`,
`rule-sound-dropdown.png` (Alert 1-16, Filter folder: chime.wav, uwu.mp3; notes.txt excluded),
`rules-list-chips.png` (no chip for the cancel-values rule), `config-kite.png`, `config-quiet-cancel.png`
(`/` picked, shapes/sizes greyed, no Icon column icon), `config-pos.png` (Positional on).
Nits seen: the Beam `/` tile has no tile box when unselected while Minimap's does; "Background (" caption still cut at 660pt; screenshot mode shows "Unapplied changes" on load (pre-existing, also in the qi7 shots).
