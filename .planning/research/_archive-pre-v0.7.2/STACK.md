# Technology Stack: v0.7 additions

**Project:** FerrisFilter, milestone v0.7 "Initial Public Release and Strictness Selector"
**Researched:** 2026-10-17
**Scope:** only the NEW capabilities (release CI, installer, per-user dirs, Documents folder and folder picker, league list, in-app update check). The parser/editor/skin/egui UI/poe.ninja sync are validated and untouched.

## Verdict in one paragraph

The milestone needs very little new code from outside. Add **three crates** (`dirs`, `semver`, `rfd`), reuse the **reqwest 0.13 + rustls + serde_json + tempfile** already in `Cargo.toml` for the update check and installer download, reuse the **existing `INDEX_STATE_URL` fetch** for the league list (no new endpoint), use **Inno Setup 6** (preinstalled on `windows-latest`) for the installer, and a **hand-written GitHub Actions workflow** with `softprops/action-gh-release` plus a ~10-line awk changelog extraction. Do not add `self_update`, `cargo-wix`/WiX, `directories`, `confy`, or `native-dialog`.

## Current dependency baseline (from Cargo.toml / Cargo.lock)

| Fact | Evidence | Consequence |
|------|----------|-------------|
| `reqwest 0.13.3`, `default-features=false`, features `rustls-no-provider, json, query`; `rustls 0.23` with `ring` | Cargo.toml | Already does HTTPS GET + JSON. GitHub Releases API needs no new HTTP stack. A **streaming binary download needs no extra feature** (`Response::bytes()` / `chunk()` work without `stream`); only set a `User-Agent`. |
| `windows-sys 0.61` (cfg windows) with a feature list that has **no** `Win32_UI_Shell` / `Win32_System_Com` | Cargo.toml | Do NOT hand-roll `SHGetKnownFolderPath` (would need those two features plus `CoTaskMemFree` handling); `dirs` already does it correctly. |
| `dirs 6.0.0` in the lock | transitive via `tray-icon` (Linux gtk path) and `poe_data_tools` (ff-art) | Adding `dirs = "6.0"` as a direct dep reuses a locked version (no duplicate). |
| `semver 1.0.28` in the lock | transitive build-dep only (`rustc_version`) | Adding `semver = "1"` as a direct dep costs nothing new in the lock. |
| `tempfile 3`, `serde`/`serde_json`, `tokio` (rt-multi-thread) | Cargo.toml | Temp download dir, release JSON parsing, worker-thread pattern all exist (`overlay/economy.rs` is the model for a background network job). |
| `zip 5.1.1` only via `ff-art` (image-extras) | cargo tree | Not needed in `ff`. The release zip is made in CI (PowerShell `Compress-Archive`), never in the app. |
| Rust code uses **no `#![feature]`**; only `tools/ff-art` needs nightly (`poe_data_tools`, comment at tools/ff-art/Cargo.toml:14) | grep | `ff`, `ff-cli`, `ferrisfilter` build on stable. See the CI toolchain section. |
| `build.rs` bakes the version via `git describe` | build.rs | CI checkout MUST use `fetch-depth: 0` (tags), or About shows the Cargo.toml fallback / wrong `-dev` suffix. |

## Recommended Stack

### New Rust dependencies

| Library | Version | Purpose | Why | Confidence |
|---------|---------|---------|-----|------------|
| `dirs` | `6.0` (7.0.0 is the crates.io max; see note) | `config_dir()` (Windows Roaming `%APPDATA%`, Linux `$XDG_CONFIG_HOME` / `~/.config`), `cache_dir()` (Windows `%LOCALAPPDATA%`, Linux `~/.cache`), `document_dir()` (Windows `FOLDERID_Documents` via `SHGetKnownFolderPath`) | One tiny crate covers requirements 3 and 4. `document_dir()` asks the shell for the Documents **known folder**, so OneDrive Known Folder Move and localized names ("Documentos") resolve exactly as the game does. Pin 6.0 because it is already locked via tray-icon (no duplicate crate); 7.0.0 exists, API for these three fns is the same, take it only if a 6->7 diff shows no issue. | HIGH for behavior (dirs docs), MEDIUM for 6 vs 7 choice |
| `semver` | `1` (1.0.28) | Parse `tag_name` minus leading `v` and compare with `env!("CARGO_PKG_VERSION")` / the baked build version | Correct ordering including prerelease (`0.7.0-rc.1 < 0.7.0`), already in the lock. Hand-comparing version strings is the classic bug (`0.10` vs `0.9`). | HIGH |
| `rfd` | `0.17` (0.17.2) | Native folder picker (`FileDialog::pick_folder`) for "choose game folder" | The de-facto cross-platform native dialog crate; on Windows uses the native `IFileDialog` (Vista-style, shows OneDrive/Documents, supports typing a path) through windows-sys, so no GTK. Its Windows deps are windows-sys 0.61 (same major as ours, no duplicate). Does not depend on egui/eframe, so no version coupling to egui 0.36. | HIGH (crate), MEDIUM (integration details below) |

Cargo.toml delta:

```toml
dirs    = "6.0"
semver  = "1"
rfd     = "0.17"     # Windows-only app: defaults are fine; xdg-portal/gtk features only matter on Linux
```

### Integration points and gotchas

**Per-user dirs (feature 3).** Add one `paths` module in the lib crate that is the single place that knows locations:
`config_dir()/FerrisFilter` (styles.json, tiers.json, ff-settings.json), `cache_dir()/FerrisFilter` (prices.json). Use `FerrisFilter` on Windows and `ferrisfilter` on Linux (the milestone text lists both spellings). Replace the current app-root walk (`FERRISFILTER_HOME` -> walk up to `styles-default.json` -> cwd) with: `FERRISFILTER_HOME` (keep: tests, screenshot mode, dev warning in CLAUDE.md all depend on it) -> per-user dirs. Keep the walk-up only as a dev convenience if wanted, but it must not outrank the per-user dir in an installed build, else a `styles-default.json` next to the exe would shadow the user's config. "Defaults embedded in exe": use `include_str!` of `styles-default.json` / `tiers-default.json` (and the prices/tiers fallback pair, `config.json`, `mapping.json` if `generate` is shipped). That removes the `styles-default.json` marker role, so the app-root concept needs a replacement marker (or none). The `-default.json` files stay committed because tests and `artifact_guard` read them. Note `FERRISFILTER_HOME` should then be a single override for both config and cache (screenshot mode asserts it lives under temp).
- Config in Roaming vs cache in Local is the correct split; prices are re-downloadable so Local/cache is right, and OneDrive/roaming profiles will not sync them.
- Create dirs lazily with `create_dir_all` at first write. Reads must tolerate absence (current code already falls back to `-default`).
- One-time migration: users on 0.6 have `styles.json`/`tiers.json`/`ff-settings.json` in the repo/app root. A "copy once if the new location is empty and the old exists" step is cheap; decide in the phase (dev-only audience so far, probably skip).
- Atomic writes already exist for filter files; reuse the same helper for config.

**Documents known folder (feature 4).** Default game folder = `dirs::document_dir()?.join("My Games").join("Path of Exile 2")`. PoE2 writes to the same known folder, so this is correct under OneDrive redirect and localization. If `document_dir()` is `None`, fall back to `%USERPROFILE%\Documents`, then to "ask the user". Always check `is_dir()` and surface "not found, pick folder" rather than silently creating a wrong path. Persist the user's choice in `ff-settings.json` (override beats default). The hardcoded `%USERPROFILE%\OneDrive\Documentos\...` in `install`/`apply` defaults and in the debug screenshot-mode isolation check must be rewritten to use the same function; screenshot mode derives the game folder from `USERPROFILE`, and `dirs` on Windows ignores `USERPROFILE` (it asks the shell), so **screenshot mode and tests need an explicit override** (e.g. env var `FERRISFILTER_GAME_DIR`, or take the path through the settings file) or they will touch the real folder. This is the main integration trap.

**Folder picker (feature 4).**
- The overlay is a borderless **always-on-top** window. A dialog with no owner will open behind it. Pass the eframe window as parent via `FileDialog::set_parent(&frame)` (eframe's `Frame` implements `HasWindowHandle`/`HasDisplayHandle`; rfd accepts raw-window-handle 0.6 types). Needs a live check; fallback is to temporarily drop topmost while the dialog is open.
- A blocking dialog on the UI thread stops egui repainting. Run it off the UI thread (pattern: `std::thread::spawn` + channel + `ctx.request_repaint()`, as `overlay/economy.rs` does for sync) or use `rfd::AsyncFileDialog`. Construct the dialog (with parent) on the UI thread, then move it into the worker; verify `FileDialog: Send` when implementing.
- Use only `pick_folder`/`pick_file`; avoid `rfd::MessageDialog` (our skinned popups cover that, and message dialogs on Windows want comctl v6 manifest activation).
- Dialog opening steals focus from the game; the hotkey is gated on game/overlay being foreground, so re-check the "summon" logic after the dialog closes.

### League list (feature 5): no new dependency

Use the endpoint the repo already calls: `https://poe.ninja/poe2/api/data/index-state` (`sync::league::INDEX_STATE_URL`, `fetch_index_state`). Verified live on 2026-10-17: it returns `economyLeagues[]` with `name`, `url` (slug), `displayName`, `hardcore`, `indexed`, plus `oldEconomyLeagues[]` (ended leagues, e.g. "Fate of the Vaal") and `snapshotVersions`. Current content: Forbidden Rites, Runes of Aldur, their HC variants, Standard, Hardcore. `models::EconomyLeague` currently deserializes name/url/hardcore/indexed; add `displayName` only if the UI wants it (`name` is what the stash/exchange endpoints accept, per the existing "return `name`, not the slug" note). The picker should list `economyLeagues` (not `oldEconomyLeagues`), not trust `indexed` for the default selection (CLAUDE.md documents it flipping between leagues; in the live response right now `Runes of Aldur` is the only indexed one while the pinned overlay league is Forbidden Rites), store the choice in `ff-settings.json`, and replace `OVERLAY_LEAGUE`'s hard pin with "user selection, default = last saved, else a sensible guess, always shown in the status line". Failure mode: offline -> keep the saved league, allow typing/keeping it. Confidence HIGH (live-verified; shape is unofficial/undocumented, so keep the parse tolerant: `#[serde(default)]` on new fields).

### Update check (feature 6): no `self_update`

Hand-roll on reqwest + semver (about 150 lines):
1. `GET https://api.github.com/repos/Jonathas-Conceicao/ferris-filter/releases/latest` with headers `User-Agent: FerrisFilter/<ver>` (GitHub rejects requests without one) and `Accept: application/vnd.github+json`. `/releases/latest` excludes drafts and prereleases, which is the behavior wanted. Unauthenticated limit is 60 requests/hour/IP; check on demand (button) and at most once per launch/day, never in a loop. Today the API returns 404 for this repo (no release published yet): treat 404 as "no releases yet / up to date", not as an error.
2. Parse `tag_name` (strip `v`), `semver::Version::parse`, compare to the running version. The running version should come from Cargo's `CARGO_PKG_VERSION`, not the `git describe` string baked for About (`0.6-dev` is not valid semver). **Cargo.toml `version` must be bumped to match each tag** (CI step: fail if tag != `v` + Cargo version).
3. Find the asset named `FerrisFilter-<ver>-setup.exe` in `assets[]`, download `browser_download_url` into `tempfile::tempdir()` (keep the dir alive: use `into_path`/persist, since the installer runs after ff exits). Verify size and, if present, the asset `digest` field (`sha256:...`) that GitHub now exposes on release assets; that needs a SHA-256 implementation, so add `sha2` only if the verify step is wanted (optional, MEDIUM: confirm the field is populated for assets uploaded by the Actions workflow; it is for API-uploaded assets since 2025).
4. Launch with `std::process::Command::new(installer).spawn()` (detached, no wait), then trigger the normal tray-Quit path so `ff.exe` is not locked. Let Inno close/restart via `CloseApplications` as a backstop. Respect the existing Quit rules (unapplied-changes popup first).
5. No code signing: SmartScreen warns for a browser download of an unsigned exe. A file fetched by reqwest carries **no Mark-of-the-Web**, so the in-app update path does not trigger SmartScreen; only the manual first install does. State this in the README (expected warning, "More info -> Run anyway") rather than buying a certificate.
6. Network on a worker thread, result into the status line, same as sync. Never auto-download or auto-install without a click.

Rejected: `self_update` 1.3.1 (swaps the running exe in place and brings its own reqwest/zip/semver tree; this app is installer-managed in `%LOCALAPPDATA%\Programs`, and exe self-replacement fights Inno's uninstall registry/`CloseApplications`; it also does not run the installer).

### Installer (feature 2): Inno Setup 6, not WiX

| Tool | Version | Why |
|------|---------|-----|
| **Inno Setup** | 6.7.1 preinstalled on the GitHub `windows-2025` image (`ISCC.exe`); verify on the `windows-latest` label at implementation (if absent: `choco install innosetup -y`) | One `.iss` text file, per-user install with no admin prompt, Start menu shortcut, uninstaller, silent flags for the updater, `CloseApplications` for upgrades. No MSI authoring knowledge needed. |

Script essentials (`installer/ferrisfilter.iss`):
- `PrivilegesRequired=lowest` (per-user), `DefaultDirName={autopf}\FerrisFilter` (resolves to `%LOCALAPPDATA%\Programs\FerrisFilter` for non-admin installs), `DefaultGroupName=FerrisFilter`, `[Icons] Name: "{autoprograms}\FerrisFilter"; Filename: "{app}\ff.exe"`.
- A fixed `AppId` GUID (upgrades replace in place), `AppVersion` passed in via `ISCC /DAppVersion=0.7.0`, `OutputBaseFilename=FerrisFilter-{#AppVersion}-setup`, `SetupIconFile` from the ico already used for the tray if one exists (else add an `.ico`; the app currently builds the tray image from PNG, so an `.ico` for the exe/shortcut may be a small new asset; embedding it in the exe is a `build.rs` job via `winres`/`embed-resource`, which is optional and only cosmetic).
- `CloseApplications=yes`, `RestartApplications=no`, optional `[Run]` "launch FerrisFilter" checkbox (`postinstall skipifsilent`).
- Ship `ff.exe`, `ff-cli.exe` (and `ferrisfilter.exe` only if the shorter-name duplicate is still wanted), `LICENSE`, README. `uwu.mp3` goes to the game folder, not the install dir: decide whether the installer offers copying it (needs the game folder, which is user-resolved; probably do it from the app, not the installer).
- Do NOT write user data under `{app}` and do NOT delete `%APPDATA%\FerrisFilter` on uninstall (it holds the user's styles); the uninstaller must leave config alone.
- ff-cli on PATH is optional; skip in v0.7 (`ChangesEnvironment` + PATH editing is a common source of installer bugs).

Rejected: **cargo-wix 0.3.9 / WiX 3.14** (the runner's WiX is the legacy 3.x line; MSI per-user installs, `Package/@InstallScope`, and shortcut components are far more ceremony, and MSI per-user install has well-known ICE-validation quirks), **NSIS** (works, but Inno's declarative sections and `[Icons]` handle exactly this scope with less scripting; no preinstalled advantage), **MSIX** (signing required), **portable zip only** (kept as a second asset, not the main path).

### Release CI (feature 1)

`.github/workflows/release.yml` (there is no `.github/` yet):

```yaml
on:
  push:
    tags: ["v*"]
permissions:
  contents: write            # needed to create the release
jobs:
  release:
    runs-on: windows-latest
    steps:
      - uses: actions/checkout@v7        # v7.0.1 is current; fetch-depth 0 for build.rs git describe
        with: { fetch-depth: 0 }
      - uses: Swatinem/rust-cache@v2     # v2.9.2 current
      - run: cargo test --locked                       # default-members = ["."] so ff-art is not built
      - run: cargo build --release --locked --bins
      - name: Check tag matches Cargo.toml version
      - name: Extract release notes   # awk over CHANGELOG.md for "## [X.Y.Z]" ... next "## [" ; write RELEASE_NOTES.md
      - name: Build installer         # & "ISCC.exe" /DAppVersion=$ver installer\ferrisfilter.iss
      - name: Zip portable            # Compress-Archive ff.exe, ff-cli.exe, README, LICENSE, CHANGELOG
      - uses: softprops/action-gh-release@v3   # v3.0.3 current; body_path: RELEASE_NOTES.md; files: setup.exe + zip (+ SHA256SUMS.txt)
        with: { prerelease: ${{ contains(github.ref_name, '-') }} }
```

Decisions:
- **Release notes**: awk/PowerShell extraction from the `## [X.Y.Z] - date` heading to the next `## [`; fail the job if the section is empty (this enforces the "update CHANGELOG at milestone completion" rule from CLAUDE.md). An alternative, `taiki-e/create-gh-release-action` with `changelog:`, parses Keep a Changelog natively, but it only creates the release (not upload on Windows jobs the way we want) and adds a second action to trust; keep the 10-line script. Confidence MEDIUM for the alternative, HIGH for the script.
- **Release action**: `softprops/action-gh-release` (v3.0.3 verified latest tag via GitHub API) is the standard; `gh release create` from the preinstalled `gh` CLI is an equally valid zero-dependency alternative (`gh release create $tag --notes-file RELEASE_NOTES.md files...`) and has no third-party-action supply-chain surface. Recommendation: use `gh release create` (it is already on the runner, auth via `GITHUB_TOKEN`), fall back to softprops if you want its prerelease/file-glob niceties. Pin third-party actions to a commit SHA either way.
- **Tests before build**: `cargo test --locked` on `windows-latest`. The overlay tests render with real fonts and are slow; the repo already sets `opt-level=2` for dependencies in `[profile.dev.package."*"]`. Expect minutes; cache with rust-cache. Some tests may touch real user dirs (game folder, `USERPROFILE`); make sure the CI environment sets `FERRISFILTER_HOME`/`USERPROFILE` to temp for the run, matching the screenshot-mode precautions.
- **Release profile**: add `[profile.release]` with `lto = "thin"`, `strip = true`, `codegen-units = 1` only if the exe size matters (egui + rustls binary is large); optional, measure first.
- Assets: `FerrisFilter-<ver>-setup.exe`, `FerrisFilter-<ver>-windows-x64.zip`, `SHA256SUMS.txt`.
- Upload the installer under a stable, predictable name; the in-app updater keys on it.

### Nightly toolchain in CI (risk)

`rust-toolchain.toml` says `channel = "nightly"` (floating) and the repo notes say "set a dated nightly there if a new one breaks `poe_data_tools`". Facts: the main crates use no `#![feature]`; only `ff-art` (excluded from `default-members`, never built by root `cargo build`/`cargo test`) needs nightly. Consequences and recommendation:
1. A floating nightly on a tag-triggered release job is a **reproducibility and break-on-release-day risk** (a regression on any nightly night fails the tag build, with the tag already pushed).
2. Recommendation: pin a dated nightly in `rust-toolchain.toml` (e.g. `channel = "nightly-2026-09-03"`, which matches the local `rustc 1.100.0-nightly (a69a63265 2026-09-03)` known to build today) with `profile = "minimal"`-compatible components. Because rustup reads the file, CI needs no toolchain action at all: the first `cargo` call auto-installs it (or run `rustup show` as an explicit step with `rustup toolchain install` first). Bump deliberately; this also gives developers and CI the same compiler. The preinstalled runner Rust is stable 1.99.0, so the file-driven install downloads a nightly each run (adds roughly a minute; rust-cache keys on rustc version).
3. Alternative (most robust for a release job): build and test release binaries on **stable** (`RUSTUP_TOOLCHAIN=stable` env on the job, or `cargo +stable`), since nothing needs nightly, and keep nightly only for ff-art and dev rustfmt/clippy. Caveat: edition 2024 needs >= 1.85 (fine on 1.99); `rustfmt.toml`/clippy lints that differ between channels are not part of the release job. Choose this if you want the binary built by a toolchain that never breaks overnight. Recommended compromise: pin the dated nightly in the file (dev/CI parity) AND keep the stable path documented as the fallback if the pinned nightly ever becomes unobtainable.
4. `dtolnay/rust-toolchain@master` with `toolchain: nightly-2026-09-03` is only needed if you want the workflow to be explicit rather than file-driven; not required. Its branches are `1.xx` versions plus `stable`/`nightly`; a dated nightly goes through `@master`.
5. Add `--locked` everywhere so `Cargo.lock` is honored (it is committed).

## Alternatives Considered

| Category | Recommended | Alternative | Why Not |
|----------|-------------|-------------|---------|
| Installer | Inno Setup 6 | cargo-wix / WiX 3.14 (MSI) | Heavier authoring, legacy 3.x on runner, no gain for per-user + Start menu |
| Installer | Inno Setup 6 | NSIS | Same capability, more imperative scripting, not preinstalled |
| Dirs | `dirs` 6 | `directories` (ProjectDirs) | `directories` would also work and creates `Org/App/config` nesting on Windows (`...\Roaming\Org\App\config`), which fights the plain `%APPDATA%\FerrisFilter` the milestone specifies |
| Dirs | `dirs` 6 | raw `windows-sys` `SHGetKnownFolderPath` | Needs two new windows-sys features, COM string freeing, and a non-Windows branch; `dirs` does it already |
| Settings | existing serde_json | `confy` | Writes TOML into its own location, bypasses the existing atomic/`unknown keys preserved` logic |
| Folder picker | `rfd` | `native-dialog`, hand `IFileDialog` via windows-sys | rfd is the maintained standard and handles parent windows; hand COM is avoidable risk |
| Update | hand-rolled (reqwest+semver) | `self_update` 1.3.1 | In-place exe swap conflicts with installer-managed installs; heavy transitive tree |
| League list | existing index-state | PoE trade API / ladder | Prices come from poe.ninja, so the league names must be the ones poe.ninja accepts |
| Release publish | `gh release create` or softprops v3.0.3 | release-please / cargo-dist | cargo-dist generates its own installers (msi/powershell), conflicting with the Inno + changelog-section flow; release-please wants conventional commits and owns CHANGELOG |
| Version compare | `semver` | string compare | Wrong for `0.10` vs `0.9` and prereleases |

## What NOT to add

- `self_update`, `cargo-dist`, `cargo-wix`, `confy`, `directories`, `native-dialog`, `winres`/`embed-resource` (unless an exe icon is explicitly wanted; purely cosmetic and optional).
- A second HTTP client (`ureq`, `hyper` directly). Reuse reqwest 0.13.
- `windows` crate (0.62.2 is in the lock via other deps, but do not add it as a direct dep; windows-sys 0.61 is the repo's choice).
- `zip` in the app (zip is built by CI).
- Code signing / SignPath in v0.7 (explicitly out of scope; document SmartScreen behavior instead).
- Auto-update without a user click, or background polling.

## Installation

```bash
cargo add dirs@6.0 semver@1 rfd@0.17
# CI/runner: Inno Setup 6 is preinstalled on windows-latest images; no install step expected
# (fallback: choco install innosetup -y)
```

## Confidence

| Area | Level | Notes |
|------|-------|-------|
| New crates and versions | HIGH | crates.io API checked 2026-10-17 (rfd 0.17.2, dirs 7.0.0 max / 6.0.0 locked, semver 1.0.28, self_update 1.3.1, cargo-wix 0.3.9); lock inspected via cargo tree |
| league endpoint | HIGH | Live GET of `/poe2/api/data/index-state` on 2026-10-17; already used by the codebase. Undocumented API, so shape can change |
| Runner contents (Inno 6.7.1, Rust 1.99.0, WiX 3.14) | MEDIUM | From the runner-images Windows2025 readme on main; `windows-latest` label may map to a different image, re-check at implementation |
| Action versions (checkout v7.0.1, rust-cache v2.9.2, softprops v3.0.3) | MEDIUM | From GitHub releases API 2026-10-17; pin to SHAs when writing the workflow |
| `rfd` parent-window + Send behavior with eframe | LOW-MEDIUM | Needs a spike (topmost overlay vs dialog stacking); fallback described above |
| `digest` field on release assets for hash verification | LOW-MEDIUM | Verify on the first real release; otherwise publish SHA256SUMS.txt and check that |
| dirs 6 vs 7 | MEDIUM | Same API for used functions to the best of my knowledge, diff not inspected |

## Sources

- crates.io API (rfd, dirs, semver, self_update, cargo-wix, windows-sys max versions), 2026-10-17
- Repo: `Cargo.toml`, `Cargo.lock`, `rust-toolchain.toml`, `build.rs`, `src/sync/league.rs`, `CLAUDE.md`
- Live: https://poe.ninja/poe2/api/data/index-state
- https://github.com/actions/runner-images/blob/main/images/windows/Windows2025-Readme.md (Inno Setup 6.7.1, WiX 3.14.1, Rust 1.99.0)
- GitHub releases API for softprops/action-gh-release (v3.0.3), actions/checkout (v7.0.1), Swatinem/rust-cache (v2.9.2)
- https://docs.rs/crate/rfd/0.17.2 (features: common-controls-v6, file-handle-inner, gtk3, xdg-portal; Windows deps include windows-sys 0.61 Win32_UI_Shell/Com)
- https://docs.github.com/rest/releases/releases#get-the-latest-release (latest excludes drafts/prereleases; User-Agent required)
