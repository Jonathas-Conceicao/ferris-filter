---
phase: 37-game-folder-resolver-picker
reviewed: 2026-10-18T00:00:00Z
depth: standard
iteration: 4
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
  warning: 0
  info: 7
  total: 7
status: clean
---

# Phase 37: Code Review Report (final re-review)

**Reviewed:** 2026-10-18
**Depth:** standard
**Files Reviewed:** 21
**Status:** clean (Info items only)

## Summary

This pass reviews the phase diff `994d560..HEAD`, focusing on the iteration-2 fix commits
`9e21743` (WR-01) and `348db27` (WR-02).

| Finding | Verdict |
|---|---|
| WR-01: any settings warning suppresses the fresh-install pick | **Fixed.** `load_parts` returns `filter_fell_back = true` in three cases: the `unusable` closure, the salvage branch's `Some(_)` arm for `filter`, and the `(Some(f), _)` arms of the final match. A bad `game_folder`, `reload_after_apply` or `collapsed_sections` alone returns `false`. `build` (app.rs:1320-1334) and `reload` (app.rs:2403-2417) both gate on `!filter_fell_back`. `load` is a thin wrapper, and the table test checks that `load` and `load_detailed` agree. |
| WR-02: the automatic pick is saved by unrelated writes | **Fixed.** `Settings::auto_filter` is marked `#[serde(skip)]`, so the whole-struct `save` writes no `filter` key for it. That covers the reload toggle, the `NotFound` branch of `save_section_state` and `remember_open_filter`. `choose_unsaved_filter` writes only `auto_filter`. `settings_with_filter` and `settings_with_folder` set `filter` and clear `auto_filter`. The salvage-path literal sets `auto_filter: None`. The CLI reads only serialized keys (`saved_choice_in` -> `load`), and so does `screenshot::stage`. |

I traced the new paths for regressions:
- **`settings_after_reload`.** A warned load from the same root returns `current.clone()`, which still includes `auto_filter`. A clean load returns `loaded` (no `auto_filter`), and `choose_unsaved_filter` then re-picks. Both match what Reload did before.
- **Section folds keyed by `filter_name()`.** These still resolve to the auto-picked name, so fold state for an unsaved pick is stored under the filter actually open. This is unchanged.
- **`select_filter` `Noop` branch.** `remember_open_filter` saves without `reset_filter_state`, so unapplied rule edits survive. `CreateFilter` with the open name also lands in this branch and now saves the selection, which is the intended result.
- **`run_row_actions` (`fold != self.settings`).** The derived `PartialEq` now includes `auto_filter`. The `fold` clone carries the same `auto_filter`, so the comparison is unaffected.

I found no new Critical or Warning issues. IN-01 to IN-05 from the previous report are still present (the fix scope did not cover them). Two new Info items come from the WR-02 dropdown change.

## Info

### IN-01: The close-grace timer never clears if focus does not return (carried over)

**File:** `src/overlay/folder_dialog.rs:101-114`
**Issue:** `note_focused()` is the only thing that clears `closed_at`. If the dialog closes while the game has focus, `since_close()` stays `Some` forever and the hidden overlay keeps repainting every `CLOSE_GRACE`.
**Fix:** Clear `closed_at` (return `None`) once `elapsed() >= CLOSE_GRACE`.

### IN-02: The startup tests exercise a test-only twin (carried over)

**File:** `src/overlay/app.rs:312-324`
**Issue:** `startup_game_dir` calls `locations::game_dir` directly instead of `startup_game_dir_from`, so the `startup_game_dir_*` tests do not cover the production function. This is why `startup_game_dir_from` needs `#[cfg_attr(not(test), allow(dead_code))]`.
**Fix:** Use `startup_game_dir_from(root, std::env::var_os(GAME_DIR_ENV).as_deref(), dirs::document_dir)`.

### IN-03: The "no Documents folder" load error does not open the Config tab (carried over)

**File:** `src/overlay/shell.rs:175-177`
**Issue:** `filter_error` is shown through `plain(...)` with `open_tab: None`. When `game_dir` is `None` its text says "pick the game folder on the Config tab", but clicking the line does nothing. The missing-folder line below it does set `open_tab: Some(ShellTab::Config)`.
**Fix:** Set `open_tab: Some(ShellTab::Config)` when there is no game folder.

### IN-04: The bad-`game_folder` warning says "using the default game folder" even when `FERRISFILTER_GAME_DIR` pins it (carried over)

**File:** `src/settings.rs:230,308,315`
**Issue:** The CLI now prints this text to stderr through `saved_choice()`. Under a pinned folder the text is wrong.
**Fix:** Word it neutrally, for example "the saved game folder is ignored".

### IN-05: `install` prints filter-name warnings that do not apply to it (carried over)

**File:** `src/run.rs:308-318`
**Issue:** `install` without `--dest` uses only `game_folder`, but `saved_choice()` prints every load warning. For example, `{"filter":"../evil.filter"}` prints "...; using dynamic.filter." during an install.
**Fix:** Have `install` print only warnings about the folder, or word the warning per consumer.

### IN-06: Clicking the highlighted unsaved filter now replaces an unusable `ff-settings.json` with defaults

**File:** `src/overlay/app.rs:1662-1675` (`remember_open_filter`), `src/overlay/config_view.rs:406-409`
**Issue:**
- **When it happens.** The file cannot be used at all, for example a hand-edited trailing comma. `filter` is then `None`, so `active_filter_selected` is false and the active `dynamic.filter` entry becomes clickable.
- **What the click does.** It runs `settings::save` with the in-memory fallback `Settings` (defaults plus reload off). That overwrites the user's file and drops its `game_folder`, `collapsed_sections` and unknown keys.
- **Why it matters.** `save_section_state` refuses exactly this case, so that "a hand-edited file is never replaced by fallback defaults".
- **Why it is only Info.** `switch_to` to any other entry already behaved this way, so this is not a new kind of write. It is newly reachable from an entry that looks already selected, and before this change clicking it did nothing.

**Fix:** In `select_filter`'s `Noop` branch, call `remember_open_filter` only when `self.settings.auto_filter.is_some()` (a real automatic pick). The alternative is to have `remember_open_filter` refuse when the last load fell back. If the overwrite is intended, write that down.

### IN-07: The new clickable-active-entry branch has no view test

**File:** `src/overlay/config_view.rs:1252-1255`
**Issue:** The test context always sets `active_filter_selected: true`. No view test checks that an unselected active entry emits `ConfigAction::SelectFilter` or that a selected one does not. The app-level `picking_the_open_unselected_filter_saves_it` calls `select_filter` directly, so it skips the `(!is_active || !ctx.active_filter_selected)` guard.
**Fix:** Add a kittest/harness case for both values of `active_filter_selected`, or extract the guard into a small pure function and test that.

---

_Reviewed: 2026-10-18_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
