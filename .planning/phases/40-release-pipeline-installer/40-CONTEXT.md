# Phase 40: Release Pipeline & Installer - Context

**Gathered:** 2026-10-18
**Status:** Ready for planning

<domain>
## Phase Boundary

Pushing a version tag builds, tests and publishes a GitHub Release with a per-user Windows
installer, a portable zip and checksums; the repo carries one public name and records the art /
font decision. Covers REL-01..REL-06, INST-02..INST-04, PUB-01, PUB-02. Builds on Phases 36-39.

</domain>

<decisions>
## Implementation Decisions

### Toolchain and versions
- CI uses the project's floating `nightly` (`rust-toolchain.toml` unchanged). The author accepts the
  risk for a pet project. (Supersedes the "pinned toolchain" wording of REL-04.)
- Tags are `vX.Y.Z` and must equal `v` + the root `Cargo.toml` version exactly, or the workflow
  fails. A dry run uses version `0.7.0-rc.1` / tag `v0.7.0-rc.1`; any version containing `-` is
  published as a GitHub **pre-release** (the Phase 41 updater ignores pre-releases).
- Release notes: a final tag requires its own `## [X.Y.Z]` CHANGELOG section (missing or empty
  fails the workflow); a pre-release tag uses the `[Unreleased]` section (Phase 42 writes the
  `0.7.0` section later).
- Workflows run on **version tags and pull requests only** (no push-to-main runs). The PR workflow
  runs the tests; the tag workflow runs the tests, builds and publishes.

### What a release contains
- Assets: `FerrisFilter-<ver>-setup.exe` (installer), `FerrisFilter-<ver>-windows-x64.zip` (portable
  `ff.exe` + `LICENSE` + a short readme; it still uses `%APPDATA%\FerrisFilter`), `SHA256SUMS.txt`.
- The workflow **publishes automatically**: create the release as a draft, upload every asset, then
  publish it in the same run, so `latest` never points at a release missing assets. The author
  reviews afterwards and fixes anything by hand.
- `ff.exe` gets an icon (Ferris) and version info (Windows resource), shown by Explorer, the Start
  menu and the installer.
- One repo name everywhere: `Jonathas-Conceicao/ferris-filter` (`Cargo.toml`, the HTTP user agent,
  installer metadata, Phase 41 updater). `assets/README.md` records that the GGG art and Fontin fonts
  ship credited and compiled into the binary.

### Installer (Inno Setup)
- Per-user install to `%LOCALAPPDATA%\Programs\FerrisFilter`, no admin prompt
  (`PrivilegesRequired=lowest`), fixed `AppId`; installs only `ff.exe` (CLIs are dev tools) and
  creates `%APPDATA%\FerrisFilter`.
- Start menu entry "FerrisFilter"; optional desktop shortcut (unticked by default); "Launch
  FerrisFilter" ticked on the finish page; no run-at-Windows-start.
- Upgrading over a running `ff`: `ff` gains a hidden `--quit` flag that asks the running instance to
  quit through the existing single-instance channel (the tray-resident window does not answer
  Windows' close-applications reliably); the installer runs it before copying files. With unapplied
  edits the running instance shows its normal Unapplied changes popup.
- Uninstall removes the program and shortcuts and keeps `%APPDATA%\FerrisFilter`.

### Dry run
- Build and check the installer and zip locally first, then push the `v0.7.0-rc.1` tag to GitHub to
  prove the workflow. The author authorised using the local `gh` CLI for this and for querying runs
  / releases; keep the number of triggered builds small (fix locally, re-tag sparingly as
  `-rc.2`, ...). Making the repository public is the author's action.

### Claude's Discretion
- Workflow file layout, how the CHANGELOG section is extracted, the resource-embedding crate, Inno
  script details, how `--quit` is wired into `src/overlay/instance.rs`.

</decisions>

<code_context>
## Existing Code Insights

### Reusable Assets
- `build.rs` derives the About version from `git describe` (CI needs full history /
  `fetch-depth: 0`); `src/overlay/instance.rs` single-instance channel (second launch brings the
  window up; tray Quit releases hotkey, tray and the instance name).
- `assets/ferris/` SVG and `assets/ferris.png` (tray icon) for the exe icon.

### Established Patterns
- Release profile already optimised (REL-06, `Cargo.toml` `[profile.release]`); a clean release
  build takes about 10 minutes locally.
- `.gitattributes` has `* text=auto`; byte-exact fixtures must survive a Windows runner checkout.

### Integration Points
- New: `.github/workflows/`, an installer script folder, the exe resource.
- HTTP user agent currently names `poe2_myfilter`; `Cargo.toml` `repository` names `ferris-filter`.

</code_context>

<specifics>
## Specific Ideas

- No code signing (SmartScreen warning accepted).

</specifics>

<deferred>
## Deferred Ideas

- CI on pushes to main (author: tags and PRs are enough).

</deferred>
