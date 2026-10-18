# Research Summary — v0.7 Initial Public Release and Strictness Selector

**Researched:** 2026-10-17 · **Inputs:** STACK.md, FEATURES.md, ARCHITECTURE.md, PITFALLS.md ·
**Overall confidence:** MEDIUM-HIGH

## Executive Summary

v0.7 turns a repo-bound tool into one a stranger can install. The editor, parser, skin and
poe.ninja sync stay as they are. The work is one wide refactor (the single "app root" becomes
per-user config and cache dirs with defaults embedded in the exe), small features on top of it
(game folder picker, league and price source, strictness selector, update check), and new
infrastructure (release CI, an Inno Setup installer, the Release Review phase).

## Stack Additions

- **Crates:** `dirs` 6.0 (config / cache / Documents; already in the lock), `semver` 1 (already in
  the lock), `rfd` 0.17 (native folder dialog), `sha2` for installer verification.
- **Reused:** reqwest 0.13 + rustls, serde_json, tempfile; the league list comes from the existing
  `sync::league::INDEX_STATE_URL` (`economyLeagues`: name, slug, hardcore, indexed), live-checked.
- **Installer:** Inno Setup 6, per-user (`PrivilegesRequired=lowest`), Start menu `[Icons]` entry,
  fixed `AppId`, `CloseApplications`; preinstalled on the GitHub Windows runner (verify).
- **CI:** hand-written workflow on version tags, `fetch-depth: 0` (`build.rs` runs `git describe`),
  `--locked`, `gh release create --draft` then upload then publish.
- **Rejected:** `self_update` (in-place exe swap fights an installer-managed install), WiX
  (runner only has 3.x), NSIS, `cargo-dist`, `directories`, `confy`.

## Architecture

- Replace `InputPaths` / `paths::app_root` (`src/compile.rs:73`, `src/paths.rs:98`) with
  `AppDirs { config_dir, cache_dir }`; `FERRISFILTER_HOME` collapses both into one folder as today.
- Embed every shipped data file with `include_str!` of the committed JSON (a `Source::Embedded` in
  `read_source`, `compile.rs:171`): styles/tiers defaults, the fallback pair, **and `config.json`
  (overlay class catalog, `app.rs:308`) and `mapping.json` (sync, `sync/prices.rs:462`)**, which the
  milestone text missed. The embedded files are the same files `artifact_guard` reads.
- `generate` stays cwd-relative (protects the byte-for-byte baseline); `apply`, `sync`, `install`
  move to the per-user dirs and one shared game-folder resolver (closes v0.5 tech debt W1).
- Strictness is a pure derive / set over the pending S-F style `action` values; no stored state, no
  emitter change.
- League / price source: delete `OVERLAY_LEAGUE`; league and cache dir become sync parameters;
  `PriceSourcePolicy::{Auto, ForceFallback}` already exists; extract a generic worker from
  `EconomySync` and reuse it for the updater.
- Updater: About tab gains state and an action; "install and quit" goes through the existing quit
  popup, and the installer is spawned only after the instance name, hotkey and tray are released.

## Watch Out For

1. **Test isolation:** the Windows known-folder API ignores `USERPROFILE`, which defeats the
   env-var isolation the tests and `--screenshot` guard rely on. One injectable path provider plus a
   `FERRISFILTER_GAME_DIR` override, with the guard extended to every new directory. Top data-loss
   risk.
2. **Version / tag mismatch:** tags are `v0.6`, Cargo is `0.6.0`, CHANGELOG headings `X.Y.0`. Move to
   `vX.Y.0` tags from v0.7.0 and gate CI on tag == `v` + Cargo version.
3. **Floating nightly** on a tag build: pin a dated nightly; spike whether the root crates build on
   stable (only `ff-art` appears to need nightly).
4. **Updater vs a tray-resident `ff.exe`:** never overwrite in place; installer + `AppMutex` /
   close-applications; verify SHA-256; draft-first releases so `latest` is never incomplete.
5. **Fixture churn:** do the style rework as one re-baseline; keep `builtin:S..F` ids.
6. **Legal:** redistributing GGG art and the Fontin fonts in a public binary needs an explicit
   decision before the first public tag (Fontin's terms unverified).
7. Line endings on the Windows runner vs byte-exact fixtures (`.gitattributes` has
   `* text=auto`); poe.ninja's `indexed` flag is not a reliable default league; a persisted league
   that disappears at league end needs an explicit message; an empty sync must not look like success.

## Suggested Phases (8)

1. **Per-user locations and embedded defaults** (widest blast radius, first), incl. path provider,
   screenshot guard, empty-home first-run test.
2. **Game folder resolver and picker** (Documents default, Config row with Browse via `rfd`, reuse
   the Switch filter popup, remove the hard-coded OneDrive path from CLI defaults and help).
3. **Built-in style rework** + single fixture re-baseline (no custom sounds).
4. **Strictness Selector** (seven-button row, one revertible edit, table-driven tests, 660pt layout
   screenshot check).
5. **League picker and price source** (worker extraction, settings, Sync / league / Tiers greyed under
   Fallback, mismatch warning, empty sync = failure).
6. **Release pipeline and installer** (`release.yml`, Inno script, zip, `SHA256SUMS.txt`, version
   gates, pinned toolchain, exe version resource, rc-tag dry run, GGG/Fontin decision recorded).
7. **Check for updates** (needs a published release to test against).
8. **Release Review** (PROJECT.md → Milestone Close-out).

Phases 2 and 3 are independent; 3 and 4 could merge.

## Disagreements and Recommendations

| Topic | Recommendation |
|---|---|
| Phase order | ARCHITECTURE's order (above); all files agree on first and last three |
| Strictness "Custom" button | Display-only: no click effect, no stored snapshot (Revert undoes a staged level) |
| Migrating the author's app-root files | No migration code; author sets `FERRISFILTER_HOME` or copies by hand; keep a real-file round-trip test, never delete originals |
| Tag format | `vX.Y.0` from v0.7.0; update the complete-milestone check |
| Toolchain | Pinned dated nightly; build the release on stable if the root crates allow |
| Publish step | `gh release create --draft`, upload, publish; pin third-party actions by SHA |
| Asset names | `FerrisFilter-<ver>-setup.exe`, `FerrisFilter-<ver>-windows-x64.zip`, `SHA256SUMS.txt` |
| Update download location | random subdirectory of `cache_dir/updates` |
| SHA-256 check | Do it: the only integrity signal for an unsigned download |
| Tiers editor under Fallback | Greyed with a tooltip (embedded fallback tiers are read-only) |
| Reload-after-Apply default | Keep on; name the keys sent in the tooltip / README; author decides |
| `generate` and per-user dirs | Stays cwd-relative |

## Research Flags

- **Needs phase research:** 2 (`rfd` parenting / threading under eframe, guard redesign), 5 (live
  poe.ninja league spike, default-league rule), 6 (Inno on the runner, `AppMutex` with a tray-hidden
  app, stable vs nightly, Fontin / GGG terms), 7 (GitHub API details, installer handoff).
- **Standard patterns:** 1 (start with a grep inventory of `app_root()` / `relative_to` call
  sites), 3, 4, 8.

## Gaps

- Style rework details (pending from the author).
- Fontin / GGG redistribution decision (blocking before the first public tag).
- Final public `owner/repo` name: `Cargo.toml` says `ferris-filter`, the HTTP user agent
  `poe2_myfilter`. The repo must be public before the updater can be tested.
- Smaller: `rfd` parent window / `Send`; `dirs` 6 vs 7; default-league rule (proposed: keep the
  persisted pick, else the first non-HC temporary league with one confirming click); whether the
  installer needs a `--quit` hook for a tray-hidden overlay; a CHANGELOG note that a live
  `styles.json` survives the style rework.
