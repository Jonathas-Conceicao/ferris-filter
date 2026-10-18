---
phase: 37-game-folder-resolver-picker
fixed_at: 2026-10-18T00:00:00Z
review_path: .planning/phases/37-game-folder-resolver-picker/37-REVIEW.md
iteration: 1
findings_in_scope: 6
fixed: 6
skipped: 0
status: all_fixed
---

# Phase 37: Code Review Fix Report

**Fixed at:** 2026-10-18
**Source review:** .planning/phases/37-game-folder-resolver-picker/37-REVIEW.md
**Iteration:** 1

**Summary:**
- Findings in scope: 6 (1 critical, 5 warnings; Info excluded by scope)
- Fixed: 6
- Skipped: 0

Full suite after all fixes: `cargo test` green (lib 1495 passed, every integration file passed),
run with `FERRISFILTER_HOME` / `FERRISFILTER_GAME_DIR` under `%TEMP%\ff-fix37-*`. `cargo fmt --check`
is clean for every touched file (the remaining diffs in `tests/isolation_guard.rs` were there
before). No new clippy warnings in the touched files.

## Fixed Issues

### CR-01: Reload drops the fresh-install filter choice and switches to `dynamic.filter`

**Files modified:** `src/overlay/app.rs`
**Commit:** 12d2fd4
**Applied fix:** New helper `choose_unsaved_filter(settings, game_dir, current)`, used by `build`
and by `reload` (after the game folder is re-resolved, before `reset_filter_state`). Reload passes
the name that was open before it ran, so an unsaved pick stays on the same file when the folder
still lists it, and otherwise follows the same rule as startup. Regression tests:
`reload_keeps_the_unsaved_folder_filter` (folder with only `x.filter`; after Reload still
`x.filter`, nothing written) and `reload_keeps_the_open_unsaved_filter_among_several`.
**Status:** fixed: requires human verification (logic change in state handling).

### WR-01: Settings warnings say "using dynamic.filter" when the overlay actually loads another filter

**Files modified:** `src/overlay/app.rs`
**Commit:** 4439152
**Applied fix:** The folder-listing pick now runs only after a clean settings load
(`settings_warning.is_none()`) in both `build` and `reload`. An unusable file or an invalid stored
name therefore opens `dynamic.filter`, as the warning says (and as CLAUDE.md documents for the
fail-closed case). Tests: `an_unusable_settings_file_opens_dynamic_filter_as_its_warning_says`,
`an_invalid_stored_filter_name_opens_dynamic_filter_as_its_warning_says`. Known edge: a file whose
only problem is a bad `game_folder` (warning about the folder, no `filter` key) also skips the
pick and opens `dynamic.filter`; the warning is not wrong there, but the folder's first filter is
not chosen.
**Status:** fixed: requires human verification (logic change).

### WR-02: Refresh is greyed while the folder is missing, so the missing state cannot be refreshed away

**Files modified:** `src/overlay/config_view.rs`, `src/overlay/app.rs`
**Commit:** 08829a6
**Applied fix:** The refresh button is enabled whenever a game folder is known
(`ctx.game_dir.is_some()`), even when it is missing; the dropdown keeps its old enable rule.
`on_folder_picked` calls `refresh_filters()` before the same-folder early return, so re-picking
the current folder re-checks it. Tests: `a_missing_folder_keeps_browse_and_refresh_live` (replaces
the test that asserted the old greying), `refresh_is_off_without_a_game_folder`, and
`a_folder_that_appears_later_is_found_by_refresh_or_a_re_pick` in app.rs.

### WR-03: CLAUDE.md `ff-settings.json` row corrupted by a literal `$1`; stale default-filter text

**Files modified:** `CLAUDE.md`
**Commit:** 46d0d43
**Applied fix:** Replaced the literal `$1` with the filter-name clause, updated to the current rule
(no `filter` key: the folder's `dynamic.filter`, else its first `.filter`, else a new
`dynamic.filter`; not saved on open, kept by Reload; an invalid name or unusable file means
`dynamic.filter` with a warning). Aligned the Config-tab "restored on the next launch" text, the
missing-folder refresh behaviour (WR-02), the fail-closed paragraph (line ~279, which stays true
after WR-01), and the `ferrisfilter apply` row (WR-04/WR-05 defaults).

### WR-04: `ff-cli apply` uses the picked folder but ignores the picked filter

**Files modified:** `src/run.rs`, `src/dynamic.rs`, `src/cli.rs`, `tests/dynamic_filter.rs`
**Commit:** c7f4ceb
**Applied fix:** Took the first option. It matches the CONTEXT decision that "`dynamic.filter`
is not special anywhere". `saved_game_folder_in` became `saved_choice_in`, which reads
`game_folder` and `filter` in one `settings::load`. `dynamic::default_filter_path` now takes the
filter name, and the bare `apply` default is `<game folder>/<saved filter or dynamic.filter>`.
The CLI deliberately does not copy the overlay's unsaved first-`.filter` pick: a CLI write goes
only to a filter the user actually selected, and with nothing saved it targets `dynamic.filter`,
which errors if absent. The `--filter` help text was updated. Tests: unit
`saved_choice_reads_the_selected_filter_with_the_folder`,
`saved_choice_without_a_usable_filter_is_dynamic_filter`; integration
`default_filter_path_uses_the_filter_selected_in_ff`.

### WR-05: CLI silently falls back to the Documents folder when the app root or the saved key is unusable

**Files modified:** `src/run.rs`, `tests/dynamic_filter.rs`
**Commits:** b974d9c, dc1c80c (rustfmt of the new tests)
**Applied fix:** New pure `saved_choice_from(pinned, root)`. An unresolvable FerrisFilter folder is
an error when `FERRISFILTER_GAME_DIR` is unset. When it is set, only the filter name falls back to
`dynamic.filter`, with a warning. `saved_choice()` prints any settings-load warning as
`warning: ...` on stderr. `install` now reads the saved folder only when `--dest` is omitted, which
also covers IN-05. Tests: `saved_choice_reports_an_unusable_key`,
`an_unresolved_root_is_an_error_unless_the_game_folder_is_pinned`,
`a_resolved_root_reads_its_settings`; integration
`default_filter_path_says_when_the_saved_choice_cannot_be_used`. The unpinned-root error path is
unit-tested only, because an integration test would need `FERRISFILTER_GAME_DIR` unset, and a bug
there could reach the real game folder.
**Status:** fixed: requires human verification (error-path logic).

## Out of scope (Info, not attempted)

IN-01 through IN-04 were not attempted because `fix_scope` is `critical_warning`. The WR-05 change
also happens to resolve IN-05.

---

_Fixed: 2026-10-18_
_Fixer: Claude (gsd-code-fixer)_
_Iteration: 1_
