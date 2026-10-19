# v0.7 autonomous run — items deferred to the end

Collected during `/gsd-autonomous --from 36 --to 42` (started 2026-10-18). The author asked for every
human review and input to be stacked here and presented after Phase 41, before Phase 42's review.

## Deferred manual checks

- **Phase 36** (`36-HUMAN-UAT.md`): (1) release `ff` against an absent `%APPDATA%\FerrisFilter`
  with no data files beside the exe shows the defaults; Apply creates the folder with
  `styles.json` / `tiers.json` / `ff-settings.json` and writes nothing beside the exe or in the
  working directory. (2) A real Sync economy with and without `FERRISFILTER_HOME` writes
  `prices.json` only in the matching folder and names the league.

- **Phase 37** (`37-HUMAN-UAT.md`, 9 checks): folder dialog above the always-on-top overlay (overlay
  stays up, hotkey ignored while open); default folder on OneDrive / localized Documents; refresh
  button hover / pressed art at 100% and 150% DPI; refresh lists a newly dropped `.filter` and
  `.mp3` with no write or lost edit; a folder pick with unapplied rule edits opens Switch filter?;
  missing-folder status and its click-through to Config; fresh install opens the folder's filter
  (also after Reload) and `ff-cli apply` follows the filter saved in ff; screenshot guard refuses
  without `FERRISFILTER_GAME_DIR` under temp; `tests/ui/README.md` screenshot steps.
- **Phase 38** (`38-HUMAN-UAT.md`, 4 checks): strictness row fits one line at 660pt, captions and
  tooltips read well, Flat Broke on a fresh install, Custom dim and unclickable; a level click marks
  Config ` *`, Revert asks and restores, Apply writes, each level hides / shows the expected tiers in
  game; minimap tile tooltip shows once (Config style editor and Custom rule style lines); both
  Reset buttons and popups (greyed tooltip "Already at the defaults.", custom styles kept, Revert /
  Apply).
- **Phase 39** (`39-HUMAN-UAT.md`, 8 checks): live poe.ninja list, choice restored after relaunch,
  Sync outcome names the league; Fallback greying and tooltips (Reset to defaults looked slightly
  brighter in one capture though computed disabled); ended-league status line (ellipsis at window
  width, full text on hover); price-source unapplied marker; tier edits pending under Fallback; Quit
  popup and "Not quit" warning sharing the line with an in-game reload; a pending source switch
  surviving a filter switch / Reload; offline refresh and a broken `ff-settings.json` left as is.
- **Phase 40-07** (manual): install Inno Setup (`winget install --id JRSoftware.InnoSetup -e`),
  rebuild `ff.exe` (the dry-run build predates the review fixes), build the three assets locally,
  run the 8-step installer test, confirm or drop installing `LICENSE.txt` next to `ff.exe` (step 8).
  Also check: uninstall answered No leaves a running `ff` alone; a failed quit during uninstall shows
  its message and removes nothing.
- **rc tag target:** `v0.7.0-rc.1` goes on commit `c560333` (last commit before Phase 42: version
  `0.7.0-rc.1`, notes from `[Unreleased]`); the workflow builds the tagged commit, so Phase 42's later
  version / CHANGELOG changes do not affect it.
- **Phase 41** (`41-MANUAL-VERIFICATION.md` / `41-HUMAN-UAT.md`, 8 live checks, need a published
  final release newer than the build): check / download / ready; install with and without unapplied
  edits; already current; offline; `%TEMP%` cleanup on quit; real GitHub field formats; rate limit.
  Also hand-check: which status message wins when an update message arrives late; an rc build is
  offered the final; the installer starting while its file is locked (admin-rights fallback untested).
- **Phase 40-08** (outward, cleared by the author, run after 40-07): read-only checks, then one push
  of `refs/tags/v0.7.0-rc.1`, watch the run and release, verify checksums; at most one rerun /
  one `-rc.2`. Note: the tag's source archives include `.planning/` (the repo is already public).
  Optional actionlint download for workflow linting needs a yes.
- **Phase 41 live check**: the update check against the real published pre-release / release (after
  40-08; pre-releases are ignored by `releases/latest`, so a real "newer version" test needs a
  non-rc release).

- **Phase 42 REV-05** (author): review / rewrite README.md and the drafted `## [0.7.0]` CHANGELOG
  section; README still needs the Config game-folder / strictness / reset text, the Economy changes,
  the installer, update check and downloads. Optional Config-top render:
  `%TEMP%\ff42-02-oGFz\new\config-top.png`.

## Open gaps after automatic gap closure

None: every phase verified 100% of its success criteria in code; only manual checks remain.

## Judgement calls made without asking

- Phase 36: no UI-SPEC (no visible UI change); planner chose `styles-default.json (built in)` labels
  for errors on built-in copies; folding a section may create the config folder.
- Phase 36-07: CHANGELOG / README edits skipped per Milestone Close-out; the wrong README sentence is
  recorded in the 36 SUMMARY for the Release Review.
- Phase 36 review: IN-01..IN-03 (info) left unfixed, recorded in `36-REVIEW.md`.
- Phase 37 UI-SPEC / plans: the CLI honours a saved `game_folder`; the game's
  `webtoolbarrefresh` art is used as the whole button (not an icon on a skin button); refresh is
  disabled when the folder is missing and shows no success message; Browse disabled while the
  dialog is open and when `FERRISFILTER_GAME_DIR` is set; dialog title "Select your Path of Exile 2
  folder"; on a fresh install the filter choice stays in memory (nothing written at launch);
  `rfd` vetted by `cargo info` (PolyMeilex/rfd, MIT, 0.17.x) instead of slopcheck (pip missing).
- Phase 37 execution: real game refresh art used (68x68, untrimmed so the three states match);
  Browse widened to 120; **clicking the highlighted but unsaved filter entry now saves it as the
  selection** (`active_filter_selected`, `Noop` branch in `select_filter`; undo if unwanted); the
  overlay's own filter pick lives in a never-saved `Settings::auto_filter`; `ff-cli apply` without
  `--filter` targets the filter saved in ff, else `dynamic.filter`; the CLI errors instead of
  falling back to Documents; leftover IN-06: with an unusable `ff-settings.json` that same click
  overwrites it with defaults.
- Phase 38 execution: fixture `tests/fixtures/seed-styles.json`; a tier reset whose file order would
  not validate restores the default F-to-S order; a CRLF-sensitive test normalises line endings
  instead of a `.gitattributes` change; advisory leftovers IN-01 (live strictness levels look as dim
  as Custom until hovered), IN-02 (Custom tooltip wording), IN-04 (Reset buttons over the heading's
  gold underline), IN-05 (a hand-made `tiers.json` with unknown / repeated tiers can stay invalid
  after Reset).
- Phase 39 execution: tier edits pending under Fallback are kept (Economy ` *`) but Apply is not
  live for them; switching back to Live makes the next Apply write them; a league / source choice
  that cannot be saved applies for the session with a status message; when tier edits are the only
  pending work, the Quit popup offers only Quit without applying / Cancel; the review loop hit its
  3-iteration cap, so the last fix was read by hand, not re-reviewed (`39-REVIEW.md` shows the
  iteration-3 state); open info items IN-01..IN-07, IN-09 (e.g. ended-league detection is
  case-sensitive; the reload toggle still rewrites the whole settings file).
- Phase 38 UI-SPEC / plans: strictness segments sized to their captions (usable width ~517pt,
  two-word names stacked, no label above the row); clicking the lit level or Custom does nothing; a
  level click also sets S and A to Show; custom styles never count toward "differs from defaults";
  resets restore from the built-in copies; tooltip / button / popup wording chosen by the agent; test
  sample case 3 in `tests/ui/test_ff.filter` moves from `kite` to built-in `C` (and its row in
  `tests/ui/README.md`); `generate` on a fresh checkout fails with unknown style (documented in
  CLAUDE.md, no code change).
- Phase 39 UI-SPEC / plans: default league = first `economyLeagues` entry that is softcore and not
  Standard (HC challenge leagues skipped too; today that is Forbidden Rites although poe.ninja's
  `indexed` points at Runes of Aldur); offline with nothing saved shows the league `prices.json`
  last priced, else `(no league)` with Sync disabled; the dropdown lists every `economyLeagues`
  entry (HC included), not `oldEconomyLeagues`; the price-source unapplied marker compares the
  source last applied with the current one (not a re-render) and is not undone by Revert; under
  Fallback, tier cutoff errors are not checked; a failed league fetch is a tooltip only; Live |
  Fallback switch replaces the read-only Price source row; `ff-cli apply` stays Auto.
- Phase 40 plans: `ff --quit` waits 10 s, exit codes 0/1/2; no Inno `AppMutex`, Restart Manager as
  fallback with `RestartApplications=no`; a silent (`/SILENT`) update does not relaunch `ff`;
  uninstall has no Retry loop; fixed AppId `{1FDA92ED-8F71-4558-B482-3EC93A7AA550}`; no automatic
  `rustup update nightly`; packaging scripts in bash; `winresource` 0.1.34 for the exe icon /
  version (vetted with `cargo info` instead of a stop); CHANGELOG link references still say
  `poe2-myfilter` (Phase 42); ROADMAP Phase 40 criterion 1 says "draft Release" though the
  workflow auto-publishes (align in Phase 42).
- Phase 41 UI-SPEC / plans: one 240pt button between the version line and the repo link; results
  only on the status line; dates local `YYYY-MM-DD`; a latest release older than the running build
  reads `Up to date (<running base version>)` without a date; a verified installer is deleted on a
  normal quit, old `%TEMP%` update folders swept after 24h; no generic worker exists (Phase 39 did
  not extract one), so a third copy of the worker pattern is added; the download URL must be exactly
  the repo's `releases/download/<tag>/<name>`, https-only redirects; Install re-hashes the file;
  **versions compare on major.minor.patch, so a `0.7.0-rc.1` build is not offered the final
  `v0.7.0`** (rc testers install 0.7.0 by hand) — *reversed in execution (review WR-03): an rc build
  is now offered the final*; `semver` / `ring` not slopchecked (pip missing; both already in
  Cargo.lock). Open info items in `41-REVIEW.md`, e.g. IN-07: a late "check failed" message queued
  behind another is cleared by the next edit.

## For the Release Review (Phase 42)

- Phase 36: README.md lines 66-69 still describe the old app-root folder ("the one holding
  `styles-default.json`"); CHANGELOG entry for the per-user folder still to write (see
  `36-07-SUMMARY.md` → For the Release Review). `generate` still reads `./prices.json` while `sync`
  writes to the per-user folder (CLI dev tools; mention in CLAUDE.md / help).
- Phase 36: clippy `result_large_err` warnings grew with the new loaders, plus one
  `type_complexity` at `src/overlay/config_state.rs:125` (pre-existing warning class, not fixed).
- Phase 37: CHANGELOG — default game folder = OS Documents + `My Games\Path of Exile 2` (order:
  `FERRISFILTER_GAME_DIR`, picked folder, Documents); Game folder row with Browse, remembered;
  refresh button re-lists filters and sounds; "Game folder not found: <path>"; CLI apply / install
  defaults follow the saved folder and filter. README — new dependency `rfd` 0.17; new GGG art
  `assets/poe2/common/refresh_*.png` (covered by Credits). PROJECT.md deferred bullet "A
  destination override in the GUI" is now covered. `cargo fmt --check` diff in
  `tests/isolation_guard.rs` (pre-existing).
- Phase 38: CHANGELOG — new built-in styles (only S-F, unchanged, hidden ship), the Strictness row,
  Reset to defaults for styles and tiers, the single minimap tooltip. Users whose rules use the
  removed `Recycle` / `kite` / `exceptional` see "unknown style" (IN-03); `generate` without a live
  `styles.json` fails on those names. `cargo clippy` fails with ~125 errors in untouched code
  (`result_large_err`, collapsible `if`, ...): consider a cleanup or a lint config in REV-01.
- Phase 39: CHANGELOG — league picker from poe.ninja with refresh (no per-league code change);
  Live / Fallback price source (Fallback greys league, Sync, Tiers); new `ff-settings.json` keys
  `league`, `price_source`; ended-league status line; a source switch is an unapplied change that
  Revert does not undo; CLI keeps automatic league resolution and `--league`.
- Phase 40: CHANGELOG — the installer, portable zip, `ff --quit` (installer use), the real version
  and repo in the user agent, Ferris icon + version info in `ff.exe`; new build dependency
  `winresource` 0.1.34 (MIT). CHANGELOG link references still point at `poe2-myfilter`; the 0.7.0
  compare link should be `v0.6...v0.7.0`. `release.yml` installs Inno Setup 6.7.1 via Chocolatey on
  the runner (unproven until 40-08). `cargo fmt --check` fails in `src/overlay/app.rs`,
  `src/sync/league.rs`, `tests/isolation_guard.rs` (CI doesn't check fmt). 12 info items open in
  `40-REVIEW.md`. Version is now `0.7.0-rc.1`; REV-03 sets `0.7.0`.

## Author results (2026-10-18)

- Phase 36: fresh user folder, Apply creating it, Sync writing `prices.json` there: pass.
- Phase 37: all checks pass (reload button re-done twice to match the game's Item Filter button,
  game-folder tooltip de-duplicated).
- Phase 38: Economy Visibility levels in game, single minimap tooltip, both Reset buttons: pass.
- Phase 39: league pick, Fallback, unapplied source switch, tier edits under Fallback, pending
  switch surviving a filter switch / Reload: pass. Offline refresh not tested, assumed pass.
- Phase 40: local installer UAT approved (LICENSE.txt kept); `v0.7.0-rc.1` dry run published and
  checksums verified.
- Phase 41: "Up to date" pass; the live newer-release flow stays deferred until a release newer
  than 0.7.0 exists.
- Judgement calls: clicking the highlighted unsaved filter saves it (keep); the price-source switch
  is unapplied and not Revert-able (keep).
- Remaining: Phase 42 REV-05 (author review of README.md and CHANGELOG.md).
