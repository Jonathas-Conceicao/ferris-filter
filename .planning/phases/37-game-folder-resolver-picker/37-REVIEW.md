---
phase: 37-game-folder-resolver-picker
reviewed: 2026-10-18T00:00:00Z
depth: standard
files_reviewed: 22
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
  - assets/poe2/manifest.toml
  - CLAUDE.md
findings:
  critical: 1
  warning: 5
  info: 5
  total: 11
status: issues_found
---

# Phase 37: Code Review Report

**Reviewed:** 2026-10-18
**Depth:** standard
**Files Reviewed:** 22
**Status:** issues_found

## Summary

I reviewed the phase diff `994d560..HEAD`. That covers the pure game-folder resolver, the
`game_folder` settings key and `choose_filter`, the CLI defaults, the `rfd` worker, the Browse and
refresh rows, the missing-folder status line and the screenshot guard. The resolver, the
settings salvage path, the screenshot guard and the dialog worker are sound and well tested.

The main defect is in the new "fresh install" filter choice. It is applied only once, in
`build`, and only in memory. The Reload path does not apply it, so Reload switches the
overlay to a different file. The same in-memory choice also makes the existing settings
warnings false, because they still say `dynamic.filter` is in use. Two smaller problems: the
refresh button cannot clear a stale "Game folder not found" state, and the CLI resolves the
picked folder but still hard-codes `dynamic.filter`. CLAUDE.md also has a literal `$1` left over
from a failed sed substitution, which deleted a sentence of the `ff-settings.json` row.

## Critical Issues

### CR-01: Reload drops the fresh-install filter choice and switches to `dynamic.filter`

**File:** `src/overlay/app.rs:1311-1319` (build), `src/overlay/app.rs:2363-2395` (reload), `src/overlay/app.rs:1044-1057` (`settings_after_reload`)

**Issue:** When `ff-settings.json` has no `filter` key, `build` sets `settings.filter` in memory
to `choose_filter(list, "dynamic.filter")`, i.e. the folder's first `.filter`. Nothing writes it
to disk. `save_section_state` (used by every section fold) writes only the fold key. On
**Reload**, `settings::load` returns a clean `Settings` with `filter: None`, and
`settings_after_reload` returns that loaded value unchanged (`!warned`). So `filter_name()`
becomes `dynamic.filter` again, and `reset_filter_state` loads `<game>/dynamic.filter`. In the
case this rule exists for, a folder without `dynamic.filter`, that file is absent.

Effect: the user was editing `x.filter`, presses Reload, and is now on an empty, non-existent
`dynamic.filter`. The Config dropdown shows `dynamic.filter`, and the next Apply creates a new
file instead of writing the filter the user had open. This is the GAME rule "dynamic.filter is
not special anywhere" failing on an ordinary action. It is not tested:
`fresh_install_selects_the_folders_filter_in_memory_only` only checks `build`.

**Fix:** Apply the choice in one helper and call it from `build`, from `reload` (after the game
folder is re-resolved) and anywhere else the settings are replaced:
```rust
fn choose_unsaved_filter(settings: &mut Settings, game_dir: Option<&Path>) {
    if settings.filter.is_none()
        && let Some(list) = game_dir.and_then(|d| settings::list_filters(d).ok())
        && !list.is_empty()
    {
        settings.filter = Some(settings::choose_filter(&list, settings.filter_name()));
    }
}
// in reload(), after self.game_dir is re-resolved and before reset_filter_state():
choose_unsaved_filter(&mut self.settings, self.game_dir.as_deref());
```
Add a test: build on a folder with only `x.filter`, call `reload()`, then assert
`filter_name() == "x.filter"`.

## Warnings

### WR-01: Settings warnings say "using dynamic.filter" when the overlay actually loads another filter

**File:** `src/settings.rs:143-160`, `src/overlay/app.rs:1311-1319`

**Issue:** The new rule tests `settings.filter.is_none()`. That is true for a fresh install, but
also in two other cases:
- **Unusable file:** `load` returns `Settings::default()` with
  "could not be used (...); using dynamic.filter, and reload after Apply is off."
- **Invalid stored name:** the name is dropped, with "...; using dynamic.filter."

In both cases `build` then picks the folder's first `.filter` whenever the folder has no
`dynamic.filter`. The status line still says `dynamic.filter`, so the user is told one file is
in use while another is loaded, and Apply would write it. This also breaks the documented
fail-closed fallback (CLAUDE.md line 279: an unusable file "means `dynamic.filter`").

**Fix:** Use one of these:
- Apply `choose_filter` only for a genuinely fresh install: no file at all, or a file with no
  `filter` key and no warning.
- Or build the warning text from the name that was actually chosen.

The simplest change is to gate the block on `settings_warning.is_none()`.

### WR-02: Refresh is greyed while the folder is missing, so the missing state cannot be refreshed away

**File:** `src/overlay/config_view.rs:394`, `src/overlay/app.rs:1467-1475`

**Issue:** `game_dir_missing` is recomputed only in `refresh_filters`, which runs at build, folder
pick, Reload and Refresh. But the Refresh button is disabled whenever `missing` is true
(`enabled = ctx.filters.is_some() && !missing`). Example: the game creates `My Games\Path of
Exile 2` on its first launch after the overlay has started. The status line keeps reading
"Game folder not found", and the user's only ways out are:
- **Reload**, which discards rule edits;
- **Browse**, re-picking the same default folder. That one is a no-op too, because
  `on_folder_picked` returns early when the pick equals the current `game_dir`, so even
  `refresh_filters` is not run.

**Fix:** Keep Refresh enabled whenever `game_dir.is_some()`. It is read-only and only re-lists.
Also, in `on_folder_picked`, call `self.refresh_filters()` before the `same` early return, so
re-picking the current folder re-checks it.

### WR-03: CLAUDE.md `ff-settings.json` row corrupted by a literal `$1`; stale default-filter text

**File:** `CLAUDE.md:34`, `CLAUDE.md:211`, `CLAUDE.md:279`

**Issue:** Line 34 now reads "Written by the overlay's CONFIG tab; $1 also `game_folder` ...". A
sed back-reference was not expanded, and the original clause was deleted with it: "holds the
selected game-folder filter name (a bare `*.filter` file name, validated before it is ever
joined onto the game folder), default `dynamic.filter` when absent or invalid". CLAUDE.md is the
project's declared spec ("keep it accurate"). Lines 211 ("restored on the next launch (default
`dynamic.filter`)") and 279 (an unusable file "means `dynamic.filter`") are also no longer true
under the new fresh-install rule, which uses the folder's first `.filter` when there is no
`dynamic.filter`.

**Fix:** Restore the deleted clause in place of `$1`, updated to the new rule. For example: "holds
the selected filter name (a bare `*.filter` file name, validated before it is ever joined onto
the game folder); when absent, the folder's `dynamic.filter` if present, else its first
`.filter`, else `dynamic.filter` as a new file". Then align lines 211 and 279 with the CR-01 and
WR-01 fixes.

### WR-04: `ff-cli apply` uses the picked folder but ignores the picked filter

**File:** `src/run.rs:159`, `src/dynamic.rs:26,104-106`, `src/cli.rs:76`

**Issue:** The CLI now reads `game_folder` from `ff-settings.json` but still targets the
hard-coded `FILTER_FILE` (`dynamic.filter`). Example: the user picks a folder whose filter is
`ssf.filter`, and the overlay saves `{"game_folder": ..., "filter": "ssf.filter"}`. A bare
`ff-cli apply` then fails with "not found" (or refreshes a different `dynamic.filter` in that
folder), even though the same settings file names the filter the user is working on. This
contradicts the phase decision "`dynamic.filter` is not special anywhere", and half-honouring
the file is surprising. The help text does say `dynamic.filter`, so this is documented, but it
is inconsistent.

**Fix:** Pick one and document it:
- Use `Settings::filter_name()` from the same load. Read `game_folder` and `filter` together, for
  example `saved_game_folder_in` returning `(Option<PathBuf>, String)`.
- Or state explicitly in the help that the CLI default is always `dynamic.filter`, whatever is
  selected in ff.

### WR-05: CLI silently falls back to the Documents folder when the app root or the saved key is unusable

**File:** `src/run.rs:254-265`, `src/run.rs:71`

**Issue:** `saved_game_folder()` does `from_env().ok()?`, and `saved_game_folder_in` discards the
`settings::load` warning. Two cases:
- `FERRISFILTER_HOME` points at a non-directory.
- `ff-settings.json` is malformed, or `game_folder` is relative.

In either case `ff-cli install` (no `--dest`) quietly copies `universal.filter` into
`Documents\My Games\Path of Exile 2` instead of the folder the user picked, and nothing says the
pick was ignored. `apply` fails later on `from_env` in the first case, except with
`--remove-economy`, which does not load inputs, so it edits the default folder's
`dynamic.filter`.

**Fix:** When `FERRISFILTER_GAME_DIR` is unset, propagate the `from_env` error, and print the
settings warning to stderr (`eprintln!("warning: {w}")`) before resolving the default. Do this
at least when `--dest` / `--filter` is omitted.

## Info

### IN-01: The close-grace timer never clears if focus does not return, so the hidden overlay repaints every 500 ms

**File:** `src/overlay/app.rs:3027-3030`, `src/overlay/folder_dialog.rs:112-114`

**Issue:** `closed_at` is cleared only by `note_focused()`. If the user clicks into the game as
the dialog closes, the overlay hides after the grace period, but `since_close()` stays `Some`
forever. `request_repaint_after(CLOSE_GRACE)` then keeps scheduling frames until the next
summon.

**Fix:** Clear `closed_at` once `since_close() >= CLOSE_GRACE`, for example in
`since_close()` itself or right after `keep_visible` is evaluated.

### IN-02: The startup tests exercise a test-only twin, not the production function

**File:** `src/overlay/app.rs:308-319`

**Issue:** `startup_game_dir_from` exists only for tests (`allow(dead_code)` outside them).
`startup_game_dir`, which production uses, duplicates its logic instead of calling it, so the five
`startup_game_dir_*` tests do not cover the code that runs.

**Fix:** Make `startup_game_dir(root)` call
`startup_game_dir_from(root, std::env::var_os(GAME_DIR_ENV).as_deref(), dirs::document_dir)`,
or route it through `locations::game_dir`.

### IN-03: The "no Documents folder" load error does not open the Config tab

**File:** `src/overlay/shell.rs:175-177`, `src/overlay/app.rs:1011-1015`

**Issue:** With `game_dir == None`, the `filter_error` status line says "pick the game folder on
the Config tab". It is a `plain` line with no `open_tab`, unlike the new missing-folder row,
whose click does open Config.

**Fix:** Give that case `open_tab: Some(ShellTab::Config)`, for example by passing it through
`game_folder_missing` when `game_dir` is `None`.

### IN-04: The bad-`game_folder` warning says "using the default game folder" even when `FERRISFILTER_GAME_DIR` pins it

**File:** `src/settings.rs:258-271`

**Issue:** The message assumes the Documents default, but the environment override wins.

**Fix:** Word it neutrally, for example "the saved game folder is ignored".

### IN-05: `install --dest` still reads `ff-settings.json` eagerly

**File:** `src/run.rs:71`

**Issue:** `saved_game_folder()` is evaluated before `cmd_install`, even when `--dest` is given.
It is read-only and harmless, but it resolves the real per-user folder needlessly. That is why
the isolation guard had to special-case `--dest`.

**Fix:** Pass a closure, or resolve inside the `None` arm, as `cmd_apply` does.

---

_Reviewed: 2026-10-18_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
