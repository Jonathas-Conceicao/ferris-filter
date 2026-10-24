# Changelog

All notable changes to **FerrisFilter** are documented here.

The format is based on [Keep a
Changelog](https://keepachangelog.com/en/1.1.0/), and this project
adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

> **Maintenance:** each milestone's section is drafted once, at the
> end of the milestone, and reviewed by the author. `[Unreleased]`
> holds only changes made between milestones.

## [Unreleased]

## [0.7.2] - 2026-10-24

**Small UI Improvements** — alpha style colours, custom style
detection, overlay hotkey configuration, drag-and-drop reordering,
game art on section headers and search boxes.

### Added

- Alpha for style colours: the Config tab's colour picker has an Alpha
  slider for Text, Border and Background, and the previews show a
  translucent colour as it will look in game.
- **Auto Detect**: turns the styling already in your rules into named
  styles (`Custom Style 1`, ...) and tags the rules with them. Apply
  saves it, Revert undoes it.
- Config tab **Overlay** section: set your own show / hide hotkey
  (default `Ctrl+Shift+Z`), active right after Apply.
- Drag-and-drop reordering for rules and custom styles, by the handle
  at the right of each row; the list scrolls when you drag near its
  top or bottom.

### Changed

- Economy rule names state their tier's price range, such as `S tier
  Currency >= 30 chaos` or `E tier Essences < 2 chaos`. The first
  Apply updates their names.
- Updated art for section headers and search boxes.

### Removed

- The up / down arrows on rules and custom styles, replaced by the
  drag handle. Section headers keep theirs.
- The "moved across Continue" flag on moved rules.

### Fixed

- The refresh buttons beside the Config tab's filter list and the
  Economy tab's league list sat slightly off-centre.

## [0.7.1] - 2026-10-19

### Changed

- `ff.exe` links the C runtime statically, so it no longer needs the
  Visual C++ Redistributable (`VCRUNTIME140.dll`) installed.

## [0.7.0] - 2026-10-18

**Initial Public Release and Strictness Selector** — FerrisFilter
ships as a Windows installer and a portable zip, keeps its files in
per-user folders, finds your game folder by itself, gains a one-click
strictness selector and a league picker, and can check for updates.

### Added

- Windows installer (per user, no admin, Start menu entry, an
  uninstaller that keeps your settings; installing over a running `ff`
  closes it first) and a portable zip, with `SHA256SUMS.txt`, on
  GitHub Releases.
- Config tab **Game folder** row: found from your Documents folder
  (OneDrive and localized names included), **Browse** to pick another,
  remembered, and a refresh button that re-lists filters and sounds.
- **Strictness** row on the Styles section: six levels (Flat Broke to
  Div Farmer) that hide the lower tier styles in one click; **Custom**
  when the S-F visibility is mixed.
- **Reset to defaults** for styles and for tiers.
- Economy tab **League** picker filled from poe.ninja (with a refresh
  button, remembered, and a warning when the league has ended) and a
  **Live / Fallback** price source switch.
- About tab **Check for updates**: manual only, hash-verified
  download, installed through the normal Quit.
- Ferris icon and version info on `ff.exe`.

### Changed

- Settings, styles, tiers and synced prices now live in
  `%APPDATA%\FerrisFilter`, created on the first save. The defaults,
  fallback prices and class catalog are built into the exe, so a fresh
  install needs no files. Old files are not migrated.
- `ff-cli apply` and `install` follow the saved game folder and filter
  instead of a fixed OneDrive path.
- The built-in styles are now S-F, `unchanged` and
  `hidden`. `Recycle`, `kite` and `exceptional` no longer ship; rules
  still using them show "unknown style" until re-styled.
- The league is chosen on the Economy tab instead of being pinned in
  the code.
- The About tab shows the version from the git tag (`0.7.0`, or
  `0.7.0-dev` for unreleased builds) and links to the repository the
  build was cloned from; the poe.ninja user agent carries the same
  version and repository.
- FerrisFilter is licensed under GPL-3.0-or-later (`LICENSE`),
  including the custom art in `assets/custom/`; the other files in
  `assets/` keep their own terms, listed in `assets/README.md`
  (renamed from `ASSETS.md`).

### Fixed

- Hovering a minimap icon in the style editor (Config tab and Custom
  rules) showed its tooltip twice.

[Unreleased]: https://github.com/Jonathas-Conceicao/ferris-filter/compare/v0.7.2...HEAD
[0.7.2]: https://github.com/Jonathas-Conceicao/ferris-filter/releases/tag/v0.7.2
[0.7.1]: https://github.com/Jonathas-Conceicao/ferris-filter/releases/tag/v0.7.1
[0.7.0]: https://github.com/Jonathas-Conceicao/ferris-filter/releases/tag/v0.7.0
