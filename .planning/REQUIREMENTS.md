# Requirements: FerrisFilter v0.7 Initial Public Release and Strictness Selector

**Defined:** 2026-10-17
**Core Value:** `generate` turns the config into a correct, deterministic PoE2 loot filter.
**Research:** `.planning/research/SUMMARY.md`

## v0.7 Requirements

### Per-user locations (LOC)

- [x] **LOC-01**: A fresh install with no files of its own works: the default styles, tiers,
  fallback prices, the class catalog (`config.json`) and `mapping.json` are built into the exe.

- [x] **LOC-02**: The user's styles, tiers, settings and synced prices are saved in one per-user
  folder (`%APPDATA%\FerrisFilter`, `~/.config/ferrisfilter`), created by the installer and by
  Apply / Sync when missing.

- [x] **LOC-03**: `FERRISFILTER_HOME` still overrides it (the dev-from-repo workflow).
- [x] **LOC-04**: `ff-cli apply` / `sync` / `install` (a dev tool, not installed) use the same
  folder and game folder as the overlay; `generate` stays working-directory relative.

- [x] **LOC-05**: Tests and the debug screenshot mode can never read or write the real config or
  game folder.

### Game folder (GAME)

- [x] **GAME-01**: The default game folder is the OS Documents folder + `My Games\Path of Exile 2`
  (OneDrive redirection and localized folder names work).

- [x] **GAME-02**: The Config tab shows the game folder and a Browse button to pick another; the
  choice is saved and restored on the next launch.

- [x] **GAME-03**: Changing the game folder with unapplied rule edits asks first (the Switch
  filter? popup).

- [x] **GAME-04**: A missing game folder shows a clear status pointing to the picker.
- [x] **GAME-05**: A refresh button beside the filter selector re-reads the game folder, so
  `.filter` files and `.mp3` / `.wav` sound files added since load appear in the filter dropdown and
  the Alert sound dropdowns without a restart; it never writes and never discards edits.

- [x] **GAME-06**: The refresh button uses the game's own refresh art (round arrow on a square
  button), extracted by `ff-art` through a `manifest.toml` entry into `assets/poe2/` and compiled
  into the binary like the rest of the skin.

### Built-in styles (BSTY)

- [x] **BSTY-01**: The built-in styles are the author's reworked set (snapshot:
  `.planning/research/author-styles-2026-10-17.json`): only its built-ins `S`-`F`, `unchanged`
  and `hidden` become `styles-default.json`; no custom style ships (`Recycle`, `kite`,
  `exceptional`, `Build Items`, `White bases` are the author's own). `tiers-default.json` is
  unchanged.

- [x] **BSTY-02**: No built-in style uses a custom sound file.
- [x] **BSTY-03**: A user's own `styles.json` is kept; built-in ids stay `builtin:S`..`builtin:F`.

### Strictness Selector (STRICT)

- [x] **STRICT-01**: The Config tab shows one exclusive button row: `Custom | Flat Broke |
  Scrounging | Getting By | Comfy | Loaded | Div Farmer`.

- [x] **STRICT-02**: Clicking a level sets the built-in tier styles' visibility, hiding nothing, F,
  E-F, D-F, C-F, B-F respectively, as one pending edit (Revert undoes, Apply writes).

- [x] **STRICT-03**: The highlighted button is derived from the current S-F visibility; any other
  pattern shows Custom, which is display-only.

- [x] **STRICT-04**: Non-tier built-ins and custom styles are untouched; a tooltip says rules using
  tier styles (user rules included) are affected.

### League and price source (LEAGUE)

- [x] **LEAGUE-01**: The Economy tab has a league picker listing poe.ninja's current leagues; the
  choice is saved and replaces the pinned `sync::OVERLAY_LEAGUE`.

- [x] **LEAGUE-02**: Sync economy uses the chosen league and its outcome names it.
- [x] **LEAGUE-03**: A saved league no longer listed (league end) is reported explicitly, never
  switched silently.

- [x] **LEAGUE-04**: A price source choice, Live or Fallback; Fallback disables Sync and the league
  picker (tooltip: recommended for league start and non-trade players) and prices the economy
  section from the fallback pair on Apply.

- [x] **LEAGUE-05**: Under Fallback the Tiers editor is greyed with a tooltip (the fallback tiers
  are fixed).

### Release pipeline (REL)

- [ ] **REL-01**: Pushing a version tag runs the tests and the release build and produces the
  installer, a portable zip and `SHA256SUMS.txt`.

- [ ] **REL-02**: The workflow fails when the tag differs from the `Cargo.toml` version or the
  version's CHANGELOG section is missing or empty.

- [ ] **REL-03**: A GitHub Release is created with that CHANGELOG section as its notes, kept a
  draft until every asset is uploaded, then published by the workflow.

- [ ] **REL-04**: Workflows run on version tags and pull requests only, on the project's floating nightly toolchain (the author accepts the risk); a release is published automatically once every asset is uploaded.
- [ ] **REL-05**: Tags are `vX.Y.Z` from `v0.7.0` on; the milestone close-out check is updated.
- [x] **REL-06**: The release profile is optimised (`lto = "fat"`, `codegen-units = 1`,
  `strip = true`). Done at milestone start.

### Installer (INST)

- [ ] **INST-02**: A per-user installer (no admin prompt) that installs only the `ff` overlay (the
  CLIs are dev tools and are not shipped in it), creates the config folder, adds a Start menu entry
  for FerrisFilter and an uninstaller.

- [ ] **INST-03**: Uninstalling keeps the user's config folder.
- [ ] **INST-04**: Installing over a running `ff` closes it first and keeps the settings.

### Check for updates (UPD)

- [x] **UPD-01**: Check for updates on the About tab reports "up to date" or the newer version
  found (GitHub Releases of `Jonathas-Conceicao/ferris-filter`).

- [x] **UPD-02**: For a newer version it downloads the installer, verifies its SHA-256 and offers
  Install and quit (through the existing Unapplied changes popup).

- [x] **UPD-03**: Offline, rate-limited or missing-asset failures show a status message and change
  nothing.

### Public release (PUB)

- [ ] **PUB-01**: `assets/README.md` records the decision to ship the GGG art and Fontin fonts,
  credited and compiled into the binary (no asset files in the install or config folders).

- [ ] **PUB-02**: One public repo name, `Jonathas-Conceicao/ferris-filter`, is used everywhere
  (`Cargo.toml`, HTTP user agent, updater).

### Reset to defaults (RESET)

- [x] **RESET-01**: A Reset to defaults button on the Config tab's Styles heading restores the
  built-in styles to their defaults and keeps every custom style; it asks first in a popup and is one pending edit (Revert
  undoes, Apply writes).

- [x] **RESET-02**: A Reset to defaults button on the Economy tab's Tiers heading restores the
  default tier cutoffs, with the same popup and pending-edit behaviour.

### Fixes (FIX)

- [x] **FIX-01**: Hovering a minimap icon shows its tooltip once (today the shape name, e.g.
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
| LOC-01 | Phase 36 | Complete |
| LOC-02 | Phase 36 | Complete |
| LOC-03 | Phase 36 | Complete |
| LOC-04 | Phase 36 | Complete |
| LOC-05 | Phase 36 | Complete |
| GAME-01 | Phase 37 | Complete |
| GAME-02 | Phase 37 | Complete |
| GAME-03 | Phase 37 | Complete |
| GAME-04 | Phase 37 | Complete |
| GAME-05 | Phase 37 | Complete |
| GAME-06 | Phase 37 | Complete |
| BSTY-01 | Phase 38 | Complete |
| BSTY-02 | Phase 38 | Complete |
| BSTY-03 | Phase 38 | Complete |
| STRICT-01 | Phase 38 | Complete |
| STRICT-02 | Phase 38 | Complete |
| STRICT-03 | Phase 38 | Complete |
| STRICT-04 | Phase 38 | Complete |
| RESET-01 | Phase 38 | Complete |
| RESET-02 | Phase 38 | Complete |
| FIX-01 | Phase 38 | Complete |
| LEAGUE-01 | Phase 39 | Complete |
| LEAGUE-02 | Phase 39 | Complete |
| LEAGUE-03 | Phase 39 | Complete |
| LEAGUE-04 | Phase 39 | Complete |
| LEAGUE-05 | Phase 39 | Complete |
| REL-01 | Phase 40 | Pending |
| REL-02 | Phase 40 | Pending |
| REL-03 | Phase 40 | Pending |
| REL-04 | Phase 40 | Pending |
| REL-05 | Phase 40 | Pending |
| REL-06 | Phase 40 | Complete |
| INST-02 | Phase 40 | Pending |
| INST-03 | Phase 40 | Pending |
| INST-04 | Phase 40 | Pending |
| PUB-01 | Phase 40 | Pending |
| PUB-02 | Phase 40 | Pending |
| UPD-01 | Phase 41 | Complete |
| UPD-02 | Phase 41 | Complete |
| UPD-03 | Phase 41 | Complete |
| REV-01 | Phase 42 | Pending |
| REV-02 | Phase 42 | Pending |
| REV-03 | Phase 42 | Pending |
| REV-04 | Phase 42 | Pending |
| REV-05 | Phase 42 | Pending |

**Coverage:** 45/45 v0.7 requirements mapped, no orphans, no duplicates.

---
*Requirements defined: 2026-10-17*
