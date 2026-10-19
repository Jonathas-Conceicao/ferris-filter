# Phase 37: Game Folder Resolver & Picker - Context

**Gathered:** 2026-10-18
**Status:** Ready for planning

<domain>
## Phase Boundary

The game folder is found from the OS Documents folder or picked by the user on the Config tab, and
a refresh button beside the filter selector re-lists the folder's filters and sound files. Covers
GAME-01..GAME-06. Builds on Phase 36's per-user folder and `AppDirs` resolver.

</domain>

<decisions>
## Implementation Decisions

### Default and storage
- Default game folder: OS Documents folder (`dirs::document_dir()`: follows OneDrive redirection and
  localized names) + `My Games\Path of Exile 2`. For the author this resolves to the current
  `OneDrive\Documentos` path.
- Stored as a `game_folder` key in `ff-settings.json`, written only when the user picks a folder;
  absent means the default.
- `FERRISFILTER_GAME_DIR` overrides everything. Screenshot mode requires it to be under temp
  (replaces the `USERPROFILE`-derived game-folder check).
- `ff-cli apply --filter` and `ff-cli install --dest` defaults use the same resolver; the hard-coded
  `OneDrive\Documentos` path (`src/install.rs:13-21`, screenshot tests) and the help text naming it
  are removed.

### Config tab layout (Filter section)
- Game folder row: label, the path cut with an ellipsis (full path on hover), a Browse button.
- Filter row: the existing filter dropdown with the refresh button directly beside it.
- Refresh re-lists the folder's `.filter` and `.mp3` / `.wav` files only: it does not re-read the
  loaded filter's contents (that is Reload), changes no selection, never writes, never discards
  edits.
- No "Use default" button; Browse's tooltip names the default path. (The default itself is
  unchanged.)

### Picking a folder
- Native folder dialog (`rfd`), opened at the current folder, shown above the always-on-top overlay
  (owned by the overlay window or otherwise brought to the front).
- A pick applies and is saved at once, like the filter choice; with unapplied rule edits the
  Switch filter? popup asks first.
- Filter selection when the folder changes (and on a fresh install): keep the filter with the
  **same name** if the new folder has one; otherwise select the **first `.filter` found** in the
  folder. `dynamic.filter` is not special anywhere. Only when the folder has no `.filter` at all is
  the current name used as a new file that the first Apply creates.
- A folder with no `.filter` files is allowed (the rule above).

### Missing folder and the refresh art
- Missing game folder: the status line reads "Game folder not found: <path>"; clicking it opens the
  Config tab.
- Refresh art: located in the game files with `ff-art` (`list` / `dump`) at development time and
  added as an `assets/poe2/manifest.toml` entry, extracted into `assets/poe2/` and compiled into the
  binary like the rest of the skin (nothing is read at run time). If no suitable game art is found,
  the button is a text button reading a capital **R**; work does not stop for this, and the
  Release Review can pick a replacement.

### Claude's Discretion
- Exact widget composition, the `rfd` threading / parenting approach, and how the settings key is
  migrated in tests.

</decisions>

<code_context>
## Existing Code Insights

### Reusable Assets
- `src/settings.rs` `list_filters(dir)` (used at `src/overlay/app.rs:1325`); `ff-settings.json`
  load / save with unknown keys preserved.
- `src/overlay/sound_files.rs`: cached sound listing with `set_dir` / `refresh` / `get` (the picker
  already re-lists when its dropdown opens); refresh can call `refresh`.
- `skin::popup::ff_modal` and the existing Switch filter? popup; `skin::` button / texture helpers
  for a game-art icon button; `tools/ff-art` with `[[...]]` manifest entries.

### Established Patterns
- Config choices that are not edits (filter choice, reload toggle) save immediately and never mark
  CONFIG dirty.
- Screenshot mode's temp-folder guard (`src/overlay/screenshot.rs:262-296`, `:467-511`).

### Integration Points
- `src/install.rs` `default_dest()`; CLI `apply` default `--filter`; the overlay's game-folder
  derivation; status-line priority list (missing app root / absent filter entries).

</code_context>

<specifics>
## Specific Ideas

- The author's description of the game's refresh control: a round arrow on a square button.

</specifics>

<deferred>
## Deferred Ideas

None — discussion stayed within phase scope.

</deferred>
