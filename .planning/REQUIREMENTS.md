# Requirements: FerrisFilter v0.7 Initial Public Release and Strictness Selector

**Defined:** 2026-10-17
**Core Value:** `generate` turns the config into a correct, deterministic PoE2 loot filter.
**Research:** `.planning/research/SUMMARY.md`

## v0.7 Requirements

### Per-user locations (LOC)

- [ ] **LOC-01**: A fresh install with no files of its own works: the default styles, tiers,
  fallback prices, the class catalog (`config.json`) and `mapping.json` are built into the exe.
- [ ] **LOC-02**: The user's styles, tiers and settings are saved in the per-user config dir
  (`%APPDATA%\FerrisFilter`, `~/.config/ferrisfilter`); synced prices in the cache dir
  (`%LOCALAPPDATA%\FerrisFilter`, `~/.cache/ferrisfilter`).
- [ ] **LOC-03**: `FERRISFILTER_HOME` still overrides both, as one folder (the dev-from-repo
  workflow).
- [ ] **LOC-04**: `ff-cli apply` / `sync` / `install` use the same folders and game folder as the
  overlay; `generate` stays working-directory relative.
- [ ] **LOC-05**: Tests and the debug screenshot mode can never read or write the real config,
  cache or game folder.

### Game folder (GAME)

- [ ] **GAME-01**: The default game folder is the OS Documents folder + `My Games\Path of Exile 2`
  (OneDrive redirection and localized folder names work).
- [ ] **GAME-02**: The Config tab shows the game folder and a Browse button to pick another; the
  choice is saved and restored on the next launch.
- [ ] **GAME-03**: Changing the game folder with unapplied rule edits asks first (the Switch
  filter? popup).
- [ ] **GAME-04**: A missing game folder shows a clear status pointing to the picker.

### Built-in styles (BSTY)

- [ ] **BSTY-01**: The built-in styles are the author's reworked set (snapshot:
  `.planning/research/author-styles-2026-10-17.json`): its `S`-`F`, `unchanged`, `hidden`,
  `Recycle`, `kite` and `exceptional` entries become `styles-default.json`; the author's personal
  `Build Items` and `White bases` are not shipped. `tiers-default.json` is unchanged.
- [ ] **BSTY-02**: No built-in style uses a custom sound file.
- [ ] **BSTY-03**: A user's own `styles.json` is kept; built-in ids stay `builtin:S`..`builtin:F`.

### Strictness Selector (STRICT)

- [ ] **STRICT-01**: The Config tab shows one exclusive button row: `Custom | Flat Broke |
  Scrounging | Getting By | Comfy | Loaded | Div Farmer`.
- [ ] **STRICT-02**: Clicking a level sets the built-in tier styles' visibility, hiding nothing, F,
  E-F, D-F, C-F, B-F respectively, as one pending edit (Revert undoes, Apply writes).
- [ ] **STRICT-03**: The highlighted button is derived from the current S-F visibility; any other
  pattern shows Custom, which is display-only.
- [ ] **STRICT-04**: Non-tier built-ins and custom styles are untouched; a tooltip says rules using
  tier styles (user rules included) are affected.

### League and price source (LEAGUE)

- [ ] **LEAGUE-01**: The Economy tab has a league picker listing poe.ninja's current leagues; the
  choice is saved and replaces the pinned `sync::OVERLAY_LEAGUE`.
- [ ] **LEAGUE-02**: Sync economy uses the chosen league and its outcome names it.
- [ ] **LEAGUE-03**: A saved league no longer listed (league end) is reported explicitly, never
  switched silently.
- [ ] **LEAGUE-04**: A price source choice, Live or Fallback; Fallback disables Sync and the league
  picker (tooltip: recommended for league start and non-trade players) and prices the economy
  section from the fallback pair on Apply.
- [ ] **LEAGUE-05**: Under Fallback the Tiers editor is greyed with a tooltip (the fallback tiers
  are fixed).

### Release pipeline (REL)

- [ ] **REL-01**: Pushing a version tag runs the tests and the release build and produces the
  installer, a portable zip and `SHA256SUMS.txt`.
- [ ] **REL-02**: The workflow fails when the tag differs from the `Cargo.toml` version or the
  version's CHANGELOG section is missing or empty.
- [ ] **REL-03**: A GitHub Release is created with that CHANGELOG section as its notes, kept a
  draft until every asset is uploaded.
- [ ] **REL-04**: Release builds use a pinned toolchain, not a floating nightly.
- [ ] **REL-05**: Tags are `vX.Y.Z` from `v0.7.0` on; the milestone close-out check is updated.
- [x] **REL-06**: The release profile is optimised (`lto = "fat"`, `codegen-units = 1`,
  `strip = true`). Done at milestone start.

### Installer (INST)

- [ ] **INST-02**: A per-user installer (no admin prompt) with a Start menu entry for FerrisFilter
  and an uninstaller.
- [ ] **INST-03**: Uninstalling keeps the user's config and cache.
- [ ] **INST-04**: Installing over a running `ff` closes it first and keeps the settings.

### Check for updates (UPD)

- [ ] **UPD-01**: Check for updates on the About tab reports "up to date" or the newer version
  found (GitHub Releases of `Jonathas-Conceicao/ferris-filter`).
- [ ] **UPD-02**: For a newer version it downloads the installer, verifies its SHA-256 and offers
  Install and quit (through the existing Unapplied changes popup).
- [ ] **UPD-03**: Offline, rate-limited or missing-asset failures show a status message and change
  nothing.

### Public release (PUB)

- [ ] **PUB-01**: `assets/README.md` records the decision to ship the GGG art and Fontin fonts,
  credited and compiled into the binary (no asset files in the install or config folders).
- [ ] **PUB-02**: One public repo name, `Jonathas-Conceicao/ferris-filter`, is used everywhere
  (`Cargo.toml`, HTTP user agent, updater).

### Fixes (FIX)

- [ ] **FIX-01**: Hovering a minimap icon shows its tooltip once (today the shape name, e.g.
  "Triangle", is drawn twice, one on top of the other).

### Release Review (REV)

- [ ] **REV-01**: Code cleanup: duplication and mentions of in-milestone bugs removed, tests green.
- [ ] **REV-02**: README screenshots refreshed where screens changed.
- [ ] **REV-03**: `Cargo.toml` version set to `0.7.0`.
- [ ] **REV-04**: A `CHANGELOG.md` draft for `0.7.0` from the net changes since v0.6.
- [ ] **REV-05**: The author's review and rewrite of `README.md` and `CHANGELOG.md` closes the
  milestone.

## Future Requirements

- Online filter publishing and the in-game load message (999.3).
- Economy change summary (999.4).
- Code signing for the installer.

## Out of Scope

| Feature | Reason |
|---------|--------|
| Code signing | Cost and setup; the unsigned installer's SmartScreen warning is accepted for now |
| Linux / macOS overlay builds | Windows is the shipped target; path code stays cross-platform |
| Auto-update / update check on start-up | On-demand only; no background network use for an unsigned tool |
| In-place exe swap (`self_update`) | Fights an installer-managed install and a tray-resident `ff.exe` |
| Migration of the author's repo-root files | The author starts fresh and uses `FERRISFILTER_HOME` for dev |
| `generate` reading per-user dirs | Keeps the byte-for-byte fixture baseline working-directory relative |

## Decisions

- Reload filter in game after Apply stays **on** by default for new users.
- GGG art and Fontin fonts ship credited, compiled into the binary.
- Public repo: `Jonathas-Conceicao/ferris-filter`.

## Traceability

| Requirement | Phase | Status |
|-------------|-------|--------|

**Coverage:** filled in by the roadmap.

---
*Requirements defined: 2026-10-17*
