# Phase 36: Per-user Locations & Embedded Defaults - Context

**Gathered:** 2026-10-18
**Status:** Ready for planning

<domain>
## Phase Boundary

A fresh install with no files of its own works: every shipped data file is built into the exe, and
the user's styles, tiers, settings and synced prices live in **one** per-user folder that tests and
screenshot mode can never confuse with the real one. Covers LOC-01..LOC-05. The game folder resolver
and picker are Phase 37; this phase keeps today's game-folder default.

</domain>

<decisions>
## Implementation Decisions

### Folder layout and resolution
- One folder for everything (no config / cache split): `%APPDATA%\FerrisFilter` on Windows,
  `~/.config/ferrisfilter` on Unix (`dirs::config_dir()` + the app name). `prices.json` lives there
  too.
- Resolution order: `FERRISFILTER_HOME` if set (must be an existing directory, else an error, as
  today), otherwise the per-user folder. The walk-up from the exe to `styles-default.json` and the
  working-directory fallback are **removed** (an installed build must never stop at a stray file;
  the author runs from the repo with `FERRISFILTER_HOME`).
- The folder is created by the installer (Phase 40) and by Apply / Sync when it is missing. Reads
  with nothing on disk use the built-in defaults; a first launch writes nothing.
- File names unchanged: `styles.json`, `tiers.json`, `ff-settings.json`, `prices.json`.

### Built-in defaults
- Embedded with `include_str!` of the committed repo files (one copy; `artifact_guard` and the
  fixtures keep reading the same bytes): `styles-default.json`, `tiers-default.json`,
  `prices-fallback.json`, `tiers-fallback.json`, `config.json` (overlay class catalog) and
  `mapping.json` (sync).
- Only `styles.json` and `tiers.json` can be overridden on disk, as today. `config.json` and
  `mapping.json` are always the built-in copies for the overlay, `apply` and `sync`.
- Unchanged write rule: an unedited file is never written, so a fresh install's folder stays empty
  until the first Apply / Sync.
- A broken live file still fails with a message naming the file, now with its full per-user path.

### CLI (dev tools; not installed by the Phase 40 installer)
- `ff-cli apply` reads styles / tiers from the per-user folder (or `FERRISFILTER_HOME`); its
  `--filter` default stays the current game-folder path until Phase 37's resolver replaces it.
- `ff-cli sync` writes `prices.json` to the per-user folder; explicit output / path flags keep
  working.
- `ff-cli install` still copies the `universal.filter` that `generate` wrote in the working
  directory into the game folder.
- `generate` is unchanged and working-directory relative (byte-for-byte fixtures unchanged).

### Test isolation (LOC-05)
- One `AppDirs`-style resolver is the only code that asks the OS for a folder; everything else is
  handed paths. Tests build it on temp folders and never call the OS lookup.
- Screenshot mode keeps requiring `FERRISFILTER_HOME` under temp; with it set the per-user folder is
  never consulted. Game-folder isolation (`FERRISFILTER_GAME_DIR`) comes with Phase 37's resolver.
- A first-run test: an empty temp home loads the defaults; Apply then writes exactly the expected
  files in the expected folder.

### Claude's Discretion
- Type and module names (`AppDirs`, `defaults.rs`, `Source::Embedded`), how `InputPaths` is
  reshaped, and the file-watcher adaptation.

</decisions>

<code_context>
## Existing Code Insights

### Reusable Assets
- `src/paths.rs`: `resolve_from` (pure core) / `app_root` (env + exe + cwd wrapper), `HOME_ENV`,
  `RootError`. The pure-core-plus-thin-wrapper split is the pattern to keep.
- `src/compile.rs:73` `InputPaths` and `:171` `read_source`: live-else-default reads; a
  `Source::Embedded` variant slots in here. `relative_to` stays for the CLI and tests.
- `PriceSourcePolicy::{Auto, ForceFallback}` (`compile.rs:189`) already selects the fallback pair.

### Established Patterns
- Atomic writes via `tempfile::NamedTempFile::new_in` + persist; changed-on-disk guards keep a
  content baseline and a live baseline per config file (Phase 28).
- Screenshot mode refuses unless `FERRISFILTER_HOME`, `USERPROFILE` and the derived game folder are
  under a temp folder (`src/overlay/screenshot.rs:262-296`).

### Integration Points
- Overlay class catalog reads `config.json` from the root (`src/overlay/app.rs:308`).
- Sync reads `mapping.json` from its base dir (`src/sync/prices.rs:462`).
- `ff-settings.json` load / save (`src/overlay/settings.rs`); the overlay's `styles.json` watcher.
- CLI help text and `src/cli.rs:67` describe the old app-root rule and need updating.

</code_context>

<specifics>
## Specific Ideas

- The author starts fresh with the installed build (no migration code) and runs from the repo
  with `FERRISFILTER_HOME` pointing at the repo.
- See `.planning/research/ARCHITECTURE.md` and `PITFALLS.md` (test isolation is the top data-loss
  risk) for the call-site inventory and line references.

</specifics>

<deferred>
## Deferred Ideas

- Game folder resolver / picker and `FERRISFILTER_GAME_DIR` (Phase 37).
- Installer-created folder (Phase 40).

</deferred>
