# Phase 28: Default config files - Context

**Gathered:** 2026-10-15
**Status:** Ready for planning
**Mode:** Locked decisions approved by the author; run autonomously, no questions. No research, no
UI-SPEC: the decisions below are the spec.

<domain>
## Phase Boundary

Split the shipped defaults from the author's live config (requirements DEF-01..DEF-05). Today the
committed repo-root `styles.json` / `tiers.json` are both the defaults and the live files the
overlay's Apply rewrites, so the author's local edits show up as git diffs and the test suite
(`artifact_guard`, `generate` snapshot, many unit tests) reads whatever the author has locally.

After this phase: committed `styles-default.json` / `tiers-default.json` hold the defaults; the live
`styles.json` / `tiers.json` are git-ignored per-user files that fall back to the defaults when
absent.

Out of scope: `prices-fallback.json` / `tiers-fallback.json` and `--early-game` (unchanged),
`config.json` (stays committed), any "reset to defaults" UI, generator output changes
(`tests/fixtures/seed-universal.filter` stays byte-identical), installer/packaging work.

</domain>

<decisions>
## Implementation Decisions (all locked)

### DEF-01: committed default files

- New repo-root files `styles-default.json` and `tiers-default.json` with exactly the bytes of the
  COMMITTED (HEAD) `styles.json` / `tiers.json`: create them with
  `git show HEAD:styles.json > styles-default.json` and `git show HEAD:tiers.json > tiers-default.json`.
  **Never** copy the working-tree files (the author has local edits there).
- Nothing in the app ever writes the `-default.json` files (overlay Apply writes only the live
  path; add a test that an Apply with CONFIG edits starting from defaults leaves the default file's
  bytes unchanged).

### DEF-02: live files leave git

- Add `/styles.json` and `/tiers.json` to `.gitignore`; untrack with
  `git rm --cached styles.json tiers.json` (index only).
- The author's working-tree `styles.json` / `tiers.json` stay on disk untouched (they are parked
  byte-for-byte in `C:\Users\jonat\ff-parked-phase28\`; verify with sha256 after the untracking
  commit and at phase end; if anything changed, copy the parked file back).
- Do this in its own commit, after the tests have been switched to the default files (DEF-05), so
  no intermediate commit depends on the live files' content.

### DEF-03: missing live file means defaults

- One small helper decides the source, e.g. in `src/paths.rs` or `src/compile.rs`:
  given the live path (`<dir>/styles.json`) return it if it exists, else the sibling
  `<dir>/styles-default.json` if that exists, else an error naming **both** files (keep the current
  error wording/shape otherwise). Same for tiers. The default file name is derived from the live
  conventional name (`styles.json` → `styles-default.json`, `tiers.json` → `tiers-default.json`) —
  put the names next to `STYLES_FILE` / `TIERS_FILE` in `compile.rs` (`InputPaths` is "the only place
  the conventional file names appear in the library").
- `InputPaths` must know whether the styles/tiers path is conventional (fallback allowed) or an
  explicit override (`with_styles` / `with_tiers`, i.e. `--styles` / `--tiers`): explicit paths keep
  their current meaning, no fallback. Suggested shape: `styles_default: Option<PathBuf>` /
  `tiers_default: Option<PathBuf>`, set by `relative_to`, cleared by `with_styles` / `with_tiers`
  (planner may choose another shape).
- Applies to every reader: `generate` (styles + tiers, CWD-relative conventional paths), `apply`
  (app root), `sync` if it reads `tiers.json` (check `src/sync/` and `src/run.rs`), the overlay load,
  the CONFIG tab (`src/overlay/config_state.rs` `styles_path()` / `tiers_path()`), the styles
  change poll (`src/overlay/app.rs` ~926, "polled, not watched"), and the all-or-none Apply
  (`src/overlay/apply_all.rs` `read_now`).
- **Reads fall back; writes go to the live path.** The CONFIG tab loads from the effective source
  but always writes `styles.json` / `tiers.json` (the live path). An unedited CONFIG never writes
  (existing rule). The first Apply with CONFIG edits creates the live file.
- **Changed-on-disk guard:** the baseline for a live file that did not exist at load is "absent".
  At Apply: still absent → no conflict (write it); now present → conflict (someone created it),
  offered RELOAD / OVERWRITE as today. A live file that existed at load and is now absent is a
  conflict too (RELOAD then falls back to defaults). The guard compares against the live path,
  never the default file.
- **Watcher/poll:** creation of the live file switches the source to it; deletion switches back to
  the defaults (treated as a change on disk, same as an edit today: marks unapplied / reloads clean
  state per the existing poll behaviour).
- Opening the app or running any command never creates the live file (test it: load from a root
  with only `-default.json` files, run the read paths, assert the live files still do not exist).
- If neither file exists the current error behaviour is kept, but the message names both files.

### DEF-04: app-root marker

- `src/paths.rs` `resolve_from` / `is_root`: the marker moves from `styles.json` to
  `styles-default.json`. Order unchanged: `FERRISFILTER_HOME`, then walk up from the exe's folder,
  then the working directory. Update the not-found message ("no styles-default.json above ...").
- Update the resolver tests (`marker(dir, "styles.json")` → `"styles-default.json"`). Mind the
  IN-06/IN-08 note: temp-dir ancestors could contain a stray marker, so tests must not assume an
  ancestor of the temp dir lacks one (keep whatever guard the current tests use).

### DEF-05: tests use the defaults; docs

- Every test that reads the repo-root `styles.json` / `tiers.json` (`include_str!`, `repo_file(...)`,
  `load_styles(concat!(...))`, `fs::copy`) reads `styles-default.json` / `tiers-default.json`
  instead. Known sites: `src/compile.rs` (tests ~480-820), `src/dynamic.rs` (~443, ~547, ~585),
  `src/emitter.rs` (~662, ~712), `src/loader.rs` (~257, ~316), `src/models/config.rs` (~807, ~1383,
  ~2006), `src/models/tiers.rs` (~171), `src/overlay/*` test rigs (`app.rs` ~2847, `apply_all.rs`,
  `config_state.rs`), `tests/*.rs` (`artifact_guard`, `generate_snapshot`, any CLI test that runs
  `generate` in the repo root or copies inputs). Grep for `styles.json` / `tiers.json` across
  `src/` and `tests/` to find them all.
- Where a test copies the defaults into a temp dir as the *live* file (to exercise a write), it may
  keep naming the copy `styles.json`; where a test exercises the fallback it copies them as
  `-default.json`. CLI integration tests that run `generate` against the repo root must pass
  `--styles styles-default.json --tiers tiers-default.json` (or copy into a temp dir) so the local
  live files are never read.
- `tests/fixtures/seed-universal.filter` and other fixtures stay byte-identical.
- The suite must pass WITH the author's local `styles.json` / `tiers.json` present (they differ
  from the defaults) — run it that way, do not stash.
- New tests: fallback when live absent (styles, tiers, both), live preferred when present, explicit
  `--styles` / `--tiers` never fall back, neither present → error naming both, no read path creates
  the live file, Apply with CONFIG edits from defaults creates the live file and leaves the default
  bytes alone, guard baseline "absent", resolver marker.
- **CLAUDE.md:** update the repository-layout rows for `styles.json` / `tiers.json` (now git-ignored
  live files, fall back to the `-default.json`), add rows (or fold into those) for
  `styles-default.json` / `tiers-default.json` (committed, never written by the app), the App root
  note in the `ferrisfilter apply` row (marker is `styles-default.json`), the "Development warning"
  (Apply now writes the git-ignored live files, not committed ones; `artifact_guard` reads the
  defaults), and mention that deleting a live file resets it to the defaults (no reset button).
  Update the domain-spec sentence that names `config.json` + `styles.json` as the executable spec
  / acceptance baseline inputs where it now means the defaults.
- **CHANGELOG.md** `[Unreleased]`: one entry describing the split and the reset-by-delete.

</decisions>

<code_context>
## Existing Code Insights

- `src/compile.rs`: `STYLES_FILE` / `TIERS_FILE` constants (~37-41), `InputPaths` (`relative_to`,
  `conventional`, `with_styles`, `with_tiers`), tests ~400-830 with `repo_file(...)`/`temp_inputs`.
- `src/paths.rs`: `resolve_from` / `is_root` / `app_root` (~40-105) and tests.
- `src/loader.rs`: `load_config(path, styles_path)`, `load_styles`, tiers loading/rendering.
- `src/run.rs`: `generate` / `apply` / `sync` command wiring, `--tiers` / `--styles` notes (~88, ~186).
- `src/dynamic.rs`: apply path, `economy_body_with`.
- `src/overlay/config_state.rs`: `styles_path()` / `tiers_path()` (~440), load and baselines.
- `src/overlay/apply_all.rs`: `read_now` baselines and the changed-on-disk guard (~230-330).
- `src/overlay/app.rs`: styles poll (~926), test rig (~2847).

</code_context>

<specifics>
## Specific Ideas

- `28-MANUAL-VERIFICATION.md`: short, numbered `**UAT5-NN** - action - Expected: ... - Result: ____`
  (same format as 27's): defaults used with no live file, first CONFIG Apply creates the live file,
  delete live file resets, CLI generate/apply with and without live files, git status clean.
- Commits via the Bash tool with a heredoc; every message ends with the session attribution lines.

</specifics>

<deferred>
## Deferred Ideas

- A "reset to defaults" button (deleting the live file does it).
- Per-user config location outside the app root (installation story).

</deferred>
