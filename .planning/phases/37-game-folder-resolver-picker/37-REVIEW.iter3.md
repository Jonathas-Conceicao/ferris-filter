---
phase: 37-game-folder-resolver-picker
reviewed: 2026-10-18T00:00:00Z
depth: standard
iteration: 3
files_reviewed: 21
files_reviewed_list:
  - Cargo.toml
  - src/cli.rs
  - src/dynamic.rs
  - src/install.rs
  - src/locations.rs
  - src/overlay/app.rs
  - src/overlay/config_view.rs
  - src/overlay/folder_dialog.rs
  - src/overlay/mod.rs
  - src/overlay/screenshot.rs
  - src/overlay/shell.rs
  - src/run.rs
  - src/settings.rs
  - src/skin/textures.rs
  - src/skin/widgets.rs
  - tests/cli_errors.rs
  - tests/cli_generate.rs
  - tests/dynamic_filter.rs
  - tests/isolation_guard.rs
  - tests/ui/README.md
  - CLAUDE.md
findings:
  critical: 0
  warning: 2
  info: 5
  total: 7
status: issues_found
---

# Phase 37: Code Review Report (re-review after fixes)

**Reviewed:** 2026-10-18
**Depth:** standard
**Files Reviewed:** 21
**Status:** issues_found

## Summary

This re-review covers the phase diff `994d560..HEAD`, focusing on the fix commits
`12d2fd4..dc1c80c` (CR-01, WR-01..WR-05 of the previous report).

Verification of the earlier findings:

| Finding | Verdict |
|---|---|
| CR-01 Reload drops the unsaved filter choice | **Fixed.** `choose_unsaved_filter` runs in `build` and `reload` (after the folder is re-resolved, before `reset_filter_state`), and Reload prefers the name that was open. Two regression tests cover it. |
| WR-01 warnings say `dynamic.filter` while another filter is open | **Fixed for the two cited cases, but over-gated.** See WR-01 below. |
| WR-02 refresh cannot clear the missing state | **Fixed.** Refresh is live whenever `game_dir.is_some()`, and a same-folder re-pick calls `refresh_filters()`. Both are tested. |
| WR-03 CLAUDE.md `$1` corruption | **Fixed.** The clause is restored and the related lines are aligned. One sentence is now inaccurate because of the WR-01 gate (see WR-01). |
| WR-04 CLI ignores the selected filter | **Fixed.** `saved_choice_in` reads `game_folder` and `filter` in one load, and `default_filter_path` takes the name. Unit and integration tests cover it. |
| WR-05 silent fallback when the root or key is unusable | **Fixed.** `saved_choice_from` returns an error for an unresolved root unless the folder is pinned, warnings go to stderr, and `install --dest` no longer reads settings (this also fixes the old IN-05). |

Two new warnings come from the fixes:
- The WR-01 gate is too broad.
- The "held in memory only" unsaved pick is written to disk by unrelated preference saves, so what the CLI targets depends on how the overlay was used.

The earlier Info items IN-01 to IN-04 were out of fix scope and are still present. They are listed again below, together with one new Info item.

## Warnings

### WR-01: Any settings warning, not only a filter fallback, suppresses the fresh-install filter pick

**File:** `src/overlay/app.rs:1328-1331` (build), `src/overlay/app.rs:2408-2410` (reload), `src/settings.rs:176-215,252-272`, `CLAUDE.md:34`

**Issue:** Both call sites gate `choose_unsaved_filter` on `settings_warning.is_none()`. But
`settings::load` also warns about keys that have nothing to do with the filter, and in those cases the filter
is left `None` and no message mentions `dynamic.filter`:
- `{"game_folder":"relative"}`: warning "...; using the default game folder."
- `{"reload_after_apply":"yes"}`: salvage path, warning "...reload after Apply is off."
- `{"collapsed_sections":[]}`: warning "...every section uses its default."

Suppose the user's folder holds only `x.filter` (the case the rule exists for). The overlay then opens a
non-existent `dynamic.filter`, the dropdown shows it, and the next Apply creates a new file. The status line
gives no hint why. That is the same symptom as the original CR-01, reached through an unrelated bad key. The
fix report calls this a "known edge". CLAUDE.md line 34 now says the fallback applies only to "A stored name
that is invalid, or a file that cannot be used", which is not what the code does.

**Fix:** Gate on "the filter key was dropped or the file was unusable", not on "any warning".
`settings::load` already knows which keys fell back. Expose that, for example by returning a
`filter_fell_back: bool` (or a small struct), and use it at both sites:
```rust
let (mut settings, settings_warning, filter_fell_back) = settings::load_detailed(root);
if !filter_fell_back {
    let current = settings.filter_name().to_string();
    choose_unsaved_filter(&mut settings, game_dir.as_deref(), &current);
}
```
Add a test: `{"game_folder":"relative"}` plus a folder holding only `x.filter` should open `x.filter`.

### WR-02: The unsaved in-memory pick is saved by unrelated preference writes, so the CLI target changes without notice

**File:** `src/overlay/app.rs:1693-1703` (`set_reload_after_apply`), `src/overlay/app.rs:1657-1667` + `src/settings.rs:304-309` (`store_settings` -> `save_section_state`), `src/run.rs:281-289`

**Issue:** On a fresh install, `settings.filter` becomes `Some("x.filter")` in memory, which is documented
as "not saved on open". Two unrelated actions then write it to disk:
- Toggling **Reload filter in game after Apply** calls `settings::save(root, &next)` with the whole
  `Settings`, so `filter: "x.filter"` is written.
- Folding or unfolding a section when `ff-settings.json` does not exist yet goes through
  `save_section_state`, whose `NotFound` branch calls `save(root, settings)` and writes the whole struct,
  `filter` included.

When the file already exists, a fold writes only the fold key and the pick stays unsaved. Whether the
overlay's automatic pick becomes "the filter selected in ff" therefore depends on whether the user clicked a
section header before or after some other save created the file. That decides what a bare `ff-cli apply`
writes. The WR-04 fix states explicitly that "a CLI write goes only to a filter the user actually selected",
and this path breaks that promise without the user doing anything that selects a filter.

**Fix:** Pick one behaviour and apply it consistently. Either:
- Keep the auto-pick out of every save, for example by tracking it in a separate field
  (`auto_filter: Option<String>`) that `filter_name()` reads but `Serialize` skips. Only `switch_to` and
  `apply_folder` then set `filter`.
- Or treat any save as adoption, and document in CLAUDE.md that the first saved preference persists the
  open filter.

The first option matches the documented intent. Add a test: fresh install with only `x.filter`, call
`set_reload_after_apply(false)`, and assert that the saved file has no `filter` key.

## Info

### IN-01: The close-grace timer never clears if focus does not return (carried over)

**File:** `src/overlay/folder_dialog.rs:101-114`

**Issue:** `closed_at` is cleared only by `note_focused()`. If the dialog closes while the game has focus,
`since_close()` stays `Some` forever, and the hidden overlay keeps repainting every `CLOSE_GRACE`.

**Fix:** Clear `closed_at` once the grace period has elapsed.

### IN-02: The startup tests exercise a test-only twin (carried over)

**File:** `src/overlay/app.rs:312-324`

**Issue:** `startup_game_dir` duplicates `startup_game_dir_from` instead of calling it, so the
`startup_game_dir_*` tests do not cover the code that runs in production.

**Fix:** Have `startup_game_dir` call `startup_game_dir_from(root, std::env::var_os(GAME_DIR_ENV).as_deref(), dirs::document_dir)`.

### IN-03: The "no Documents folder" load error does not open the Config tab (carried over)

**File:** `src/overlay/shell.rs:175-177`

**Issue:** `filter_error` is a `plain` status line with `open_tab: None`. When `game_dir` is `None`, its
text tells the user to pick the folder on the Config tab, but clicking it does nothing.

**Fix:** Set `open_tab: Some(ShellTab::Config)` when there is no game folder.

### IN-04: The bad-`game_folder` warning says "using the default game folder" even when `FERRISFILTER_GAME_DIR` pins it (carried over)

**File:** `src/settings.rs:189-190,263-270`

**Issue:** This text now also reaches stderr through `saved_choice()` in the CLI, where it is wrong under a
pinned folder.

**Fix:** Word it neutrally, for example "the saved game folder is ignored".

### IN-05: `install` prints filter-name warnings that do not apply to it

**File:** `src/run.rs:70-76,309-318`

**Issue:** `install` without `--dest` uses only `game_folder`, but `saved_choice()` prints every load
warning. For example, `{"filter":"../evil.filter"}` makes `install` print "...; using dynamic.filter.",
although `install` never touches a filter of that name.

**Fix:** Let `install` print only folder-related warnings, or word the warning per consumer.

---

_Reviewed: 2026-10-18_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
