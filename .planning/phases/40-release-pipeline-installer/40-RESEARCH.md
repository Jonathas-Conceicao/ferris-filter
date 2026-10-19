# Phase 40: Release Pipeline & Installer - Research

**Researched:** 2026-10-18
**Domain:** GitHub Actions release CI (Windows runner, floating nightly), Inno Setup 6 per-user installer, Win32 single-instance `--quit`, Windows exe resources
**Confidence:** HIGH for repo facts, runner contents and Inno/gh behaviour (verified this session); MEDIUM for a few Win32/Inno details marked inline

## Summary

Everything this phase needs is either already on the `windows-latest` runner (Inno Setup 6.7.1, Git Bash, `gh` 2.102, rustup 1.29.1, `tar.exe` that writes zips, `sha256sum`) or is one small build dependency (`winresource` for the exe icon and version info). The release is one `bash` packaging script shared by CI and the local dry run, two workflow files (`ci.yml` for pull requests, `release.yml` for `v*.*.*` tags), one Inno script, and a `--quit` mode for `ff` that reuses the single-instance channel in `src/overlay/instance.rs`.

`ff.exe` alone is a complete program after Phase 36: `src/defaults.rs` `include_str!`s `styles-default.json`, `tiers-default.json`, `prices-fallback.json`, `tiers-fallback.json`, `config.json` and `mapping.json`; `src/skin` and `src/overlay/tray.rs` `include_bytes!` all art and fonts; `src/locations.rs` resolves the per-user folder from `dirs::config_dir()` (= `%APPDATA%`, the same folder as Inno's `{userappdata}`) with no walk-up from the exe. So the installer and the zip ship only `ff.exe` (plus LICENSE/readme text), which matches CONTEXT and PUB-01. [VERIFIED: codebase grep]

Two findings change how the plan should look. First, **this agent session cannot run `powershell`/`pwsh` through the Bash tool** (the worktree isolation guard refuses it), so the packaging script must be `bash`, not PowerShell, or the executor cannot dry-run it locally. Git Bash already has everything needed: `sha256sum`, `C:\Windows\System32\tar.exe` (bsdtar, makes `.zip` with `-a`) and `ISCC.exe` called directly. Second, **`/D` switches passed to `ISCC.exe` from Git Bash get mangled by MSYS path conversion**; set `MSYS2_ARG_CONV_EXCL='*'` for that call. [VERIFIED: tool refusal observed this session; bsdtar zip creation tested]

**Primary recommendation:** Write `installer/package.sh <version>` (build check, zip, ISCC, SHA256SUMS) and run it unchanged from CI (`shell: bash`) and locally; validate the tag/version/CHANGELOG in a first CI step; build with `cargo build --release --locked --bin ff`; publish with `gh release create --draft ... && gh release edit --draft=false`; implement `ff --quit` as a new `-quit` event in the instance gate plus a wait on the target process handles.

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

#### Toolchain and versions
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

#### What a release contains
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

#### Installer (Inno Setup)
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

#### Dry run
- Build and check the installer and zip locally first, then push the `v0.7.0-rc.1` tag to GitHub to
  prove the workflow. The author authorised using the local `gh` CLI for this and for querying runs
  / releases; keep the number of triggered builds small (fix locally, re-tag sparingly as
  `-rc.2`, ...). Making the repository public is the author's action.

#### Specific Ideas
- No code signing (SmartScreen warning accepted).

### Claude's Discretion
- Workflow file layout, how the CHANGELOG section is extracted, the resource-embedding crate, Inno
  script details, how `--quit` is wired into `src/overlay/instance.rs`.

### Deferred Ideas (OUT OF SCOPE)
- CI on pushes to main (author: tags and PRs are enough).
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| REL-01 | Tag push runs tests + release build, produces installer, zip, `SHA256SUMS.txt` | Release workflow job order (Pattern 1), `installer/package.sh` (Pattern 2), runner contents (Environment) |
| REL-02 | Fail on tag != Cargo version, or missing/empty CHANGELOG section | Validation step (Code Examples: `release-check.sh`), CRLF-safe awk extraction |
| REL-03 | Release with CHANGELOG notes, draft until all assets uploaded, then published | `gh release create --draft --verify-tag ... files` then `gh release edit --draft=false` (verified flags) |
| REL-04 | Tags + PRs only; floating nightly; auto publish | `on:` blocks, `rustup toolchain install` reading `rust-toolchain.toml` |
| REL-05 | `vX.Y.Z` tags from v0.7.0; close-out check updated | PROJECT.md Git Workflow step 2.3 (`annotated vX.Y`) and the `/gsd-complete-milestone` note; the final `v0.7.0` tag push on main *is* the release trigger |
| REL-06 | Release profile optimised | Already done (`Cargo.toml` `[profile.release]` lto fat / cgu 1 / strip). Nothing to do; keep it |
| INST-02 | Per-user installer, `ff` only, config folder, Start menu, uninstaller | Inno script (Pattern 3) |
| INST-03 | Uninstall keeps config folder | `[Dirs] ... Flags: uninsneveruninstall`; nothing in `[UninstallDelete]` |
| INST-04 | Install over running `ff` closes it, keeps settings | `ff --quit` (Pattern 4) called from `PrepareToInstall`, which Inno calls *before* its Restart Manager in-use check |
| PUB-01 | `assets/README.md` records GGG art + Fontin decision | Wording guidance (Pattern 6); add the new `.ico` row |
| PUB-02 | One repo name `Jonathas-Conceicao/ferris-filter` everywhere | Only offender in code: `src/sync/client.rs` user agent (`poe2_myfilter`). Also CHANGELOG link refs (`poe2-myfilter`), author-owned file |
</phase_requirements>

## Project Constraints (from CLAUDE.md)

- `CLAUDE.md` is the domain reference and must stay accurate: new rows/notes for `.github/workflows/`, `installer/`, the exe resource and `ff --quit` belong in the "Repository layout" table (the `ff` row and a new row). [CITED: CLAUDE.md]
- `CHANGELOG.md` and `README.md` are human-written; agents edit them lightly and only as directed (memory + PROJECT.md Milestone Close-out: "During a milestone, agents do not add `CHANGELOG.md` entries"). So this phase must **not** add CHANGELOG entries; the rc release notes come from the existing `[Unreleased]` text.
- `ff-art` is a non-default workspace member needing nightly + the game install; root `cargo build`/`cargo test` never build it. CI must never pass `--workspace` or `-p ff-art`.
- Byte-exact fixtures (`artifact_guard`, `tests/fixtures/parser/*.filter -text`) must survive the Windows runner checkout; the runner's Git for Windows has the same `core.autocrlf=true` system default as this machine, so CI behaves like local. [VERIFIED: `git config --show-origin core.autocrlf` -> `C:/Program Files/Git/etc/gitconfig true`; runner Git 2.56.0.windows.2]
- Raw Win32 stays isolated in `src/overlay/instance.rs` / `win.rs`, every `unsafe` block with a `// SAFETY:` comment; pure logic in testable functions, egui as a thin view (overlay layer rule in `src/overlay/mod.rs`).
- Tests and screenshot mode must never touch the real config or game folder (`FERRISFILTER_HOME`, `FERRISFILTER_GAME_DIR` isolation, Phase 36/37).
- Debug builds use a separate instance name (`-overlay-debug`), so `cargo run --bin ff -- --quit` targets only the debug instance: a safe way to test `--quit` locally.
- No emojis; GSD milestone branching (`milestone/v0.7-...`); commits end with the attribution lines.

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| Tag/version/CHANGELOG validation | CI (release.yml, bash script) | - | Must fail before anything is built or published |
| Build + test | CI runner (cargo, nightly) | Local (same commands) | Provenance: release artifacts come only from CI |
| Packaging (zip, installer, checksums) | `installer/package.sh` (bash) | Inno Setup ISCC | One script for CI and the local dry run |
| Publishing | CI (`gh` CLI, `GITHUB_TOKEN` contents:write) | - | Draft, upload, publish in one job |
| Install / uninstall / shortcuts / config dir | Inno Setup script | - | Declarative sections |
| Closing a running `ff` | `ff --quit` (Rust, Win32 events) | Inno `[Code] PrepareToInstall`, `[UninstallRun]` | Only `ff` knows its single-instance channel and quit rules |
| Exe icon + version info | `build.rs` (winresource, rc.exe) | committed `.ico` | Linked into the `ff` binary only |
| Repo name, art/font decision | `src/sync/client.rs`, `assets/README.md` | PROJECT.md | Text edits |

## Standard Stack

### Core
| Tool / Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| GitHub-hosted `windows-latest` | maps to Windows Server 2025 (VS2026 image 20261007) | Build, test, package, publish | Ships InnoSetup 6.7.1, Git 2.56, GitHub CLI 2.102.0, Rust 1.99 stable, Rustup 1.29.1, Chocolatey 2.7.4, PowerShell 7.6 [VERIFIED: actions/runner-images README + Windows2025-VS2026-Readme.md via gh api] |
| Inno Setup | 6.x (runner 6.7.1; local winget `JRSoftware.InnoSetup` 6.7.3) | Per-user installer + uninstaller | One `.iss`, `PrivilegesRequired=lowest`, `[Icons]`, `[Dirs]`, Pascal `[Code]` [VERIFIED: runner readme; `winget search` this session] |
| `gh` CLI | runner 2.102 / local 2.83.1 | Create draft release with assets, publish | Preinstalled, no third-party action, auth via `GH_TOKEN: ${{ github.token }}` [VERIFIED: `gh release create/edit --help`] |
| `actions/checkout` | v7.0.1, SHA `3d3c42e5aac5ba805825da76410c181273ba90b1` | Checkout with full history + tags | `fetch-depth: 0` fetches all history and tags (needed by `build.rs` `git describe`) [VERIFIED: gh api tag ref + README] |
| `Swatinem/rust-cache` | v2.9.2, commit SHA `6323deb102c322ba6fcbdcafc7e3dddab59af2b6` (annotated tag object `63fed3e2...` dereferenced) | Cache `~/.cargo` + `target` deps | De-facto Rust cache action [VERIFIED: gh api] |
| `winresource` (build-dependency) | 0.1.34 | Compile icon + VERSIONINFO, link into one bin | `WindowsResource::new()` pre-fills FileVersion/ProductVersion from Cargo; `compile_for("ff")` emits `cargo:rustc-link-arg-bin=ff=...` so only `ff.exe` gets it [VERIFIED: docs.rs source; `cargo info winresource`, repo github.com/BenjaminRi/winresource, MIT] |

### Supporting
| Tool | Purpose | When to Use |
|------|---------|-------------|
| `C:\Windows\System32\tar.exe` (bsdtar 3.8) | Create the portable `.zip` (`tar.exe -a -c -f x.zip -C stage files`) | Both CI and local; **call by full path**, Git Bash's `tar` is GNU tar and cannot write zip [VERIFIED: tested in a temp dir] |
| `sha256sum` (Git Bash coreutils) | Hashes for `SHA256SUMS.txt` | Write the file yourself with `printf '%s  %s\n'` (Git Bash prints `*name` binary markers otherwise) |
| `cargo pkgid -p ferrisfilter` | Read the root package version without `jq` | Output `...#ferrisfilter@0.6.0`; take text after the last `@`/`#` [VERIFIED: ran locally] |
| `rc.exe` (Windows SDK) | Resource compiler used by winresource | Present locally (`Windows Kits\10\bin\10.0.22000.0\x64\rc.exe`) and on the VS runner image; host is `x86_64-pc-windows-msvc` [VERIFIED: ls + `rustc -vV`] |
| `image` crate `ico` feature (in `tools/ff-art` only) | Write the multi-size `ferris.ico` once | `IcoEncoder::encode_images(&[IcoFrame])` behind feature `ico` [VERIFIED: docs.rs] |

### Alternatives Considered
| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| `winresource` | `embed-resource` 3.0.12 (`compile_for(rc, &["ff"], NONE).manifest_optional()`) | Equally valid; needs a hand-written `.rc` (VERSIONINFO block, numeric version macros). winresource writes the `.rc` for you. Pick winresource |
| `gh release` | `softprops/action-gh-release` v3 | Third-party action with write token; `gh` already does draft + upload + publish |
| bash `package.sh` | PowerShell `Compress-Archive`/`Get-FileHash` | Natural on Windows, but this agent environment refuses `powershell`/`pwsh` from the Bash tool, so the executor could not dry-run it. PowerShell is fine for one-line CI-only checks (`shell: pwsh` steps) |
| Run `--quit` from Inno | Inno `AppMutex` directive | AppMutex only checks and nags at setup start, *before* PrepareToInstall, so the user would see "close FerrisFilter" before `--quit` could do it. Do not set `AppMutex` |

**Installation:**
```toml
# Cargo.toml
[build-dependencies]
winresource = { version = "0.1.34", default-features = false }   # default feature only adds `toml` metadata parsing
```
```toml
# tools/ff-art/Cargo.toml (only if the .ico is generated by ff-art)
image = { version = "0.25", default-features = false, features = ["png", "ico"] }
```
Local, author action (not automatic): `winget install --id JRSoftware.InnoSetup -e` (Inno Setup 6, matches the runner's major version; its installer offers "Install for me only", which puts ISCC under `%LOCALAPPDATA%\Programs\Inno Setup 6`). Note: the orchestrator's note that winget lacks it is outdated; `winget search` lists `JRSoftware.InnoSetup 6.7.3` and `JRSoftware.InnoSetup.7 7.1.0`. Prefer 6 to match CI. [VERIFIED: winget search this session]

## Package Legitimacy Audit

slopcheck could not run (no Python on this machine: `python` resolves to the Microsoft Store stub). Per protocol every new package is `[ASSUMED]` and the planner should gate its install behind a `checkpoint:human-verify` (or the author's OK).

| Package | Registry | Age | Downloads | Source Repo | slopcheck | Disposition |
|---------|----------|-----|-----------|-------------|-----------|-------------|
| `winresource` 0.1.34 | crates.io | years (maintained fork of `winres`) | high (used by many Rust GUI apps) [ASSUMED] | github.com/BenjaminRi/winresource | unavailable | [ASSUMED] Approved pending human-verify |
| `embed-resource` 3.0.12 (alternative only) | crates.io | years | high [ASSUMED] | github.com/nabijaczleweli/rust-embed-resource [ASSUMED] | unavailable | Not recommended for install |
| `image` feature `ico` | crates.io (crate already in lock) | n/a | n/a | github.com/image-rs/image | n/a | Feature flag only, no new crate |
| `actions/checkout`, `Swatinem/rust-cache` | GitHub Actions | years | very high | github.com/actions/checkout, github.com/Swatinem/rust-cache | n/a | Pin by commit SHA (above) |

**Packages removed due to slopcheck [SLOP] verdict:** none
**Packages flagged as suspicious [SUS]:** none (slopcheck unavailable; all tagged [ASSUMED])

## Architecture Patterns

### System Architecture Diagram

```
 git push tag vX.Y.Z[-pre]            pull_request
          |                                 |
          v                                 v
  release.yml (windows-latest)        ci.yml (windows-latest)
          |                                 |
  checkout fetch-depth:0                checkout, rustup, cache
  rustup toolchain install (nightly)        |
  rust-cache                            cargo test --locked  --> pass/fail
          |
  release-check.sh: tag regex? == "v"+Cargo version?   --no--> FAIL (nothing built)
          |  pre-release ('-' in version)? -> notes = [Unreleased] : notes = [X.Y.Z]
          |  notes empty/missing? ------------------------------------> FAIL
          v
  cargo test --locked  ---------------------------------------------> FAIL
          v
  git status --porcelain --untracked-files=no empty? ---------------> FAIL (would bake -dev)
          v
  cargo build --release --locked --bin ff   (build.rs: git describe + winresource -> ff.exe)
          v
  verify: describe == tag exactly; ff.exe VersionInfo.ProductVersion == version
          v
  installer/package.sh <ver>
      stage ff.exe + LICENSE + README.txt -> tar.exe -a -> dist/FerrisFilter-<ver>-windows-x64.zip
      ISCC /DAppVersion=<ver> /DAppNumVersion=X.Y.Z installer/ferrisfilter.iss -> dist/FerrisFilter-<ver>-setup.exe
      sha256sum both -> dist/SHA256SUMS.txt
          v
  gh release create <tag> --draft [--prerelease] --verify-tag --notes-file notes.md dist/*
          v
  gh release edit <tag> --draft=false     -> public release (prerelease never becomes "latest")

 Installer run (user):
  setup.exe (no UAC) -> PrepareToInstall: if {app}\ff.exe exists: Exec "{app}\ff.exe --quit"
        exit 0 -> continue | exit !=0 -> Retry/Cancel box (running ff may be showing Unapplied changes)
     -> Restart Manager in-use check (backstop) -> copy ff.exe -> [Dirs] %APPDATA%\FerrisFilter
     -> Start menu (+ optional desktop) shortcut -> finish page "Launch FerrisFilter" (ticked)

 ff --quit:
  OpenMutex(name) absent -> exit 0
  find processes running the same image path (not self) -> open SYNCHRONIZE handles
  wait -ready (<=10 s) -> SetEvent(-quit), re-assert every 100 ms
  running ff polls -quit each frame -> same path as tray Quit (QuitNow, or Unapplied changes popup)
  wait for the handles to signal (<= timeout) -> exit 0 | timeout -> exit 1 | unreachable -> exit 2
```

### Recommended Project Structure
```
.github/
  workflows/
    ci.yml              # pull_request: cargo test --locked
    release.yml         # push tags v*.*.*: validate, test, build, package, publish
installer/
  ferrisfilter.iss      # Inno Setup 6 script (ASCII only)
  package.sh            # bash: zip + ISCC + SHA256SUMS into dist/ (CI and local)
  release-check.sh      # bash: tag/version/CHANGELOG validation, writes notes + outputs
  portable-readme.txt   # README.txt shipped in the zip
assets/ferris/ferris.ico  # multi-size icon (16..256), committed
dist/                   # package output, git-ignored (add `/dist/` to .gitignore)
```

### Pattern 1: Release workflow
**What:** One job on `windows-latest`, `defaults.run.shell: bash`, `permissions: contents: write` (repo default token is read-only, verified `default_workflow_permissions: read`), `concurrency` per ref, `timeout-minutes: 90` (fat LTO on a 4-vCPU runner plus a debug test build).
**Example:**
```yaml
# Source: patterns verified against actions/checkout README, gh release --help, runner readme
name: release
on:
  push:
    tags: ['v*.*.*']          # glob matches v0.7.0 and v0.7.0-rc.1; the script enforces the exact regex
permissions:
  contents: write
concurrency:
  group: release-${{ github.ref }}
  cancel-in-progress: false
jobs:
  release:
    runs-on: windows-latest
    timeout-minutes: 90
    defaults:
      run:
        shell: bash
    env:
      CARGO_TERM_COLOR: always
      FF_REQUIRE_RESOURCE: '1'     # build.rs: a failed icon/version resource is fatal here
    steps:
      - uses: actions/checkout@3d3c42e5aac5ba805825da76410c181273ba90b1  # v7.0.1
        with:
          fetch-depth: 0
      - run: rustup toolchain install && rustc -vV     # reads rust-toolchain.toml (floating nightly)
      - uses: Swatinem/rust-cache@6323deb102c322ba6fcbdcafc7e3dddab59af2b6  # v2.9.2
      - id: check
        run: installer/release-check.sh "${GITHUB_REF_NAME}" "$RUNNER_TEMP/notes.md" >> "$GITHUB_OUTPUT"
      - run: cargo test --locked
      - run: test -z "$(git status --porcelain --untracked-files=no)" || { git status; exit 1; }
      - run: cargo build --release --locked --bin ff
      - name: Verify baked version
        run: |
          d=$(git describe --tags --match 'v*' --dirty)
          [ "$d" = "${GITHUB_REF_NAME}" ] || { echo "describe=$d, tag=${GITHUB_REF_NAME}"; exit 1; }
      - name: Verify exe version resource
        shell: pwsh
        run: |
          $v = (Get-Item target/release/ff.exe).VersionInfo.ProductVersion
          if ($v -ne '${{ steps.check.outputs.version }}') { throw "ProductVersion $v" }
      - run: installer/package.sh "${{ steps.check.outputs.version }}"
      - name: Publish
        env:
          GH_TOKEN: ${{ github.token }}
        run: |
          pre=(); [ "${{ steps.check.outputs.prerelease }}" = true ] && pre=(--prerelease)
          gh release create "${GITHUB_REF_NAME}" --draft --verify-tag "${pre[@]}" \
            --title "FerrisFilter ${{ steps.check.outputs.version }}" \
            --notes-file "$RUNNER_TEMP/notes.md" dist/*
          gh release edit "${GITHUB_REF_NAME}" --draft=false
```
`ci.yml` is the first four steps (no `FF_REQUIRE_RESOURCE`, no `contents: write`; `permissions: contents: read`) plus `cargo test --locked`, `on: pull_request`.

Notes:
- `cargo build`/`cargo test` at the root build only `default-members = ["."]`, so `ff-art` (nightly `poe_data_tools`, a registry crate already in `Cargo.lock`) is never compiled. `--locked` still validates the whole lock, which is fine. [VERIFIED: Cargo.toml, Cargo.lock]
- Tag-push runs use the workflow file **from the tagged commit**, so `release.yml` must be committed before the rc tag is created. The milestone branch need not be pushed: pushing the tag carries its commits. [ASSUMED: standard GitHub behaviour]
- Build only `--bin ff` for the release: the CLIs are not shipped and fat LTO per binary is the slow part. Tests still build every bin in the debug profile.
- `rustup toolchain install` with no argument installs the toolchain named by `rust-toolchain.toml` (rustup >= 1.28); auto-install on first `cargo` is also on by default since 1.28.1. The explicit step keeps the download out of the timed build log. [CITED: blog.rust-lang.org/2025/03/04/Rustup-1.28.1/]

### Pattern 2: `installer/package.sh` (CI and local)
```bash
#!/usr/bin/env bash
# Usage: installer/package.sh <version>   (run from the repo root after `cargo build --release --bin ff`)
set -euo pipefail
ver="$1"
num="${ver%%-*}"                        # 0.7.0-rc.1 -> 0.7.0 (VERSIONINFO must be numeric)
root="$(cd "$(dirname "$0")/.." && pwd)"
dist="$root/dist"; stage="$dist/stage"
rm -rf "$dist"; mkdir -p "$stage"
exe="$root/target/release/ff.exe"; [ -f "$exe" ] || { echo "missing $exe"; exit 1; }

find_iscc() {
  local c
  for c in "$(command -v iscc.exe || true)" \
           "/c/Program Files (x86)/Inno Setup 6/ISCC.exe" \
           "$(cygpath -u "${LOCALAPPDATA:-C:/}")/Programs/Inno Setup 6/ISCC.exe"; do
    [ -n "$c" ] && [ -f "$c" ] && { echo "$c"; return; }
  done
  echo "ISCC.exe not found: install Inno Setup 6 (winget install --id JRSoftware.InnoSetup -e)" >&2
  return 1
}

cp "$exe" "$stage/ff.exe"
cp "$root/LICENSE" "$stage/LICENSE.txt"
cp "$root/installer/portable-readme.txt" "$stage/README.txt"
zip="FerrisFilter-$ver-windows-x64.zip"
/c/Windows/System32/tar.exe -a -c -f "$dist/$zip" -C "$stage" ff.exe LICENSE.txt README.txt

iscc="$(find_iscc)"
MSYS2_ARG_CONV_EXCL='*' "$iscc" /Q "/DAppVersion=$ver" "/DAppNumVersion=$num" \
  "$(cygpath -w "$root/installer/ferrisfilter.iss")"
setup="FerrisFilter-$ver-setup.exe"; [ -f "$dist/$setup" ] || { echo "ISCC produced no $setup"; exit 1; }

rm -rf "$stage"
( cd "$dist" && for f in "$setup" "$zip"; do
    printf '%s  %s\n' "$(sha256sum "$f" | cut -d' ' -f1)" "$f"; done ) > "$dist/SHA256SUMS.txt"
ls -l "$dist"
```
The script must be committed executable or invoked as `bash installer/package.sh` (Windows checkouts lose the `x` bit unless `git update-index --chmod=+x`). Use `bash installer/...` in the workflow to sidestep it.

### Pattern 3: Inno Setup script
```ini
; installer/ferrisfilter.iss -- ASCII only (Inno 6 reads non-BOM files as ANSI)
#ifndef AppVersion
  #error Pass /DAppVersion=X.Y.Z[-pre]
#endif
#ifndef AppNumVersion
  #error Pass /DAppNumVersion=X.Y.Z
#endif

[Setup]
AppId={{NEW-GUID-GENERATED-ONCE-AND-NEVER-CHANGED}
AppName=FerrisFilter
AppVersion={#AppVersion}
AppVerName=FerrisFilter {#AppVersion}
AppPublisher=Jonathas Conceicao
AppPublisherURL=https://github.com/Jonathas-Conceicao/ferris-filter
AppSupportURL=https://github.com/Jonathas-Conceicao/ferris-filter/issues
AppUpdatesURL=https://github.com/Jonathas-Conceicao/ferris-filter/releases
VersionInfoVersion={#AppNumVersion}
VersionInfoProductTextVersion={#AppVersion}
DefaultDirName={autopf}\FerrisFilter
DisableDirPage=yes
DisableProgramGroupPage=yes
PrivilegesRequired=lowest
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
OutputDir=..\dist
OutputBaseFilename=FerrisFilter-{#AppVersion}-setup
SetupIconFile=..\assets\ferris\ferris.ico
UninstallDisplayIcon={app}\ff.exe
UninstallDisplayName=FerrisFilter
WizardStyle=modern
Compression=lzma2/max
SolidCompression=yes
CloseApplications=yes
RestartApplications=no

[Tasks]
Name: "desktopicon"; Description: "{cm:CreateDesktopIcon}"; GroupDescription: "{cm:AdditionalIcons}"; Flags: unchecked

[Dirs]
Name: "{userappdata}\FerrisFilter"; Flags: uninsneveruninstall

[Files]
Source: "..\target\release\ff.exe"; DestDir: "{app}"; Flags: ignoreversion

[Icons]
Name: "{autoprograms}\FerrisFilter"; Filename: "{app}\ff.exe"
Name: "{autodesktop}\FerrisFilter"; Filename: "{app}\ff.exe"; Tasks: desktopicon

[Run]
Filename: "{app}\ff.exe"; Description: "{cm:LaunchProgram,FerrisFilter}"; Flags: nowait postinstall skipifsilent

[UninstallRun]
Filename: "{app}\ff.exe"; Parameters: "--quit"; Flags: waituntilterminated; RunOnceId: "QuitFerrisFilter"

[Code]
function QuitRunningFerrisFilter(): Boolean;
var
  Exe: String;
  Code: Integer;
begin
  Result := True;
  Exe := ExpandConstant('{app}\ff.exe');
  if not FileExists(Exe) then exit;          { fresh install: nothing of ours can be running from {app} }
  repeat
    if not Exec(Exe, '--quit', '', SW_HIDE, ewWaitUntilTerminated, Code) then exit;  { RM backstop }
    if Code = 0 then exit;
  until SuppressibleMsgBox('FerrisFilter is still running. If it is asking about unapplied ' +
      'changes, answer it, then click Retry.', mbError, MB_RETRYCANCEL, IDCANCEL) <> IDRETRY;
  Result := False;
end;

function PrepareToInstall(var NeedsRestart: Boolean): String;
begin
  if QuitRunningFerrisFilter() then Result := ''
  else Result := 'FerrisFilter is still running. Quit it from its tray icon, then run Setup again.';
end;
```
Verified Inno facts: in non-admin mode `{autopf}` -> `{userpf}` (`%LOCALAPPDATA%\Programs`), `{autoprograms}` -> user Start menu Programs, `{autodesktop}` -> user desktop, `{userappdata}` -> roaming AppData (the same folder `dirs::config_dir()` returns) [CITED: jrsoftware.org/ishelp/topic_consts.htm]. `PrepareToInstall` "is called before Setup checks for files being in-use if CloseApplications is set to yes", and a non-empty return stops at the Preparing to Install page [CITED: jrsoftware.org/ishelp/topic_scriptevents.htm]. `[UninstallRun]` runs "as the first step of uninstallation"; `RunOnceId` keeps one entry from running twice; `postinstall` flags run as the original user by default [CITED: jrsoftware.org/ishelp/topic_runsection.htm]. With CloseApplications=yes Inno uses Restart Manager and, when silent, closes and **restarts** apps unless told not to, hence `RestartApplications=no` (otherwise RM plus the Launch checkbox could start `ff` twice) [CITED: jrsoftware.org/ishelp/topic_setup_closeapplications.htm].

Generate the AppId once (`[guid]::NewGuid()` or `uuidgen`) and commit it; never reuse the instance-name GUID from `instance.rs`. In the `.iss` the leading `{` is doubled (`{{`).

### Pattern 4: `ff --quit` through the single-instance channel
**Current channel (read from `src/overlay/instance.rs`):** a non-owned named mutex `Local\FerrisFilter-7c1d...-overlay` (existence = instance running), and three events built by `names_for(base)`: `-show` (auto-reset, polled each frame by `Guard::take_show_request`), `-ack` (auto-reset), `-ready` (manual-reset, set on the first frame by `Guard::mark_ready`). `acquire_named` creates the events **before** the mutex and resets them; `Guard::drop` resets `-ready` then closes everything; `OverlayApp::begin_shutdown` drops the guard (`self.instance = None`) in the same frame as the single `ViewportCommand::Close`.

**Recommended design:**
1. Add `quit: format!("{base}-quit")` to `InstanceNames` (auto-reset event), created/reset/closed exactly like `-show` (also in the "no events at all" failure branch). Add `Guard::take_quit_request()` mirroring `take_show_request`. Update the name tests (`names_live_in_the_local_namespace`, `show_and_ack_share_the_mutex_base`) to include it.
2. In `OverlayApp`'s per-frame update, poll `take_quit_request()` next to the tray `MenuEvent` poll and route it into the **same** code as `OverlayCommand::Quit` (extract the tray branch into one method, e.g. `request_quit(&mut self) -> bool` returning `quit_requested`). That gives exactly the CONTEXT behaviour: `apply::quit_step(dirty, quit_confirm, QuitEvent::TrayQuit)` -> `QuitNow` (begin_shutdown + Close) or the **Unapplied changes** popup with the window revealed. Poll it before the show-request poll, like the other quit paths (WR-08 ordering comment).
3. New launcher mode: `ff --quit` (exact single argument), checked at the top of `overlay::launch()` before the screenshot parse and before `instance::acquire()`; it must **never create** the mutex (that would make it the first instance). Return an exit code with `std::process::exit` (works under the GUI subsystem; Inno reads it via `Exec`'s ResultCode).
4. `instance::quit_running(names, startup_ms, wait_ms) -> QuitOutcome` (Windows impl):
   - `OpenMutexW(SYNCHRONIZE, FALSE, name)`; null with `ERROR_FILE_NOT_FOUND` -> `NotRunning` (exit 0). Close the handle immediately: holding it keeps the name alive.
   - Collect target processes **before** signalling: every process other than self whose image path equals `std::env::current_exe()` (compare case-insensitively; normalise both, e.g. `std::fs::canonicalize`, since `GetModuleFileNameW` reflects how the exe was launched). Enumerate with Toolhelp32 (`CreateToolhelp32Snapshot` / `Process32FirstW` / `Process32NextW`, filter `szExeFile == "ff.exe"`, then `QueryFullProcessImageNameW`, already used by `win.rs::process_image_path`) and `OpenProcess(SYNCHRONIZE | PROCESS_QUERY_LIMITED_INFORMATION)`. Needs the windows-sys feature `Win32_System_Diagnostics_ToolHelp` [ASSUMED: feature/constant names; confirm at compile].
   - Open `-ready`/`-quit` without creating (open-with-`CreateEventW` + `ERROR_ALREADY_EXISTS` check like `signal_running`, or `OpenEventW`); wait `-ready` up to `STARTUP_TIMEOUT_MS`.
   - `SetEvent(-quit)`, re-assert every `RETRY_INTERVAL_MS`, until all target handles are signalled or `wait_ms` passes; on timeout `ResetEvent(-quit)` (withdraw, same reason as IN-02 for `-show`).
   - Outcomes -> exit codes: `NotRunning`/`Exited` -> 0, `StillRunning` (timeout, e.g. the Unapplied changes popup is up) -> 1, `Unreachable` (events missing / access denied, e.g. an elevated instance) -> 2. A pure `quit_exit_code(QuitOutcome) -> i32` keeps it testable.
   - If no target process matched (instance runs from another path, e.g. a portable copy), still signal it but fall back to waiting until the mutex name is gone. That copy does not lock the installed file, so 0 is correct.
   - Suggested `wait_ms`: about 10 s; the installer's Retry loop covers a user who takes longer with the popup.
5. Non-Windows `imp`: `quit_running` returns `NotRunning`.

Why wait on process handles rather than the name: `begin_shutdown` releases the name *before* eframe tears down the window and the process exits, so "name gone" does not mean `ff.exe` is unlocked yet. Waiting on the handles closes that race; Restart Manager stays as the backstop.

**Testability:** `launch_named`-style tests already exercise the gate with unique names in-process. Add: `quit_running` on a fresh unique base returns `NotRunning`; a thread holding `acquire_named(&names)` that polls `take_quit_request()` and drops the guard proves the signal path (the process-wait part can sit behind a small trait/closure so the test waits on "guard dropped" instead of a PID); `quit_exit_code` table test; `QuitEvent::TrayQuit` reuse needs no new `quit_step` tests. Manual check: `cargo run --bin ff` in one shell, `target/debug/ff --quit` in another (debug instance name), with and without unapplied edits.

### Pattern 5: Exe icon + version resource in `build.rs`
```rust
// Source: docs.rs/winresource (WindowsResource::new pre-fills from Cargo; compile_for links one bin)
fn embed_resource() {
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") {
        return; // build scripts run on the host: check the *target* via env, not cfg!
    }
    println!("cargo:rerun-if-changed=assets/ferris/ferris.ico");
    let mut res = winresource::WindowsResource::new();
    res.set_icon("assets/ferris/ferris.ico")
        .set("ProductName", "FerrisFilter")
        .set("FileDescription", "FerrisFilter")      // Task Manager and the Start menu show this
        .set("InternalName", "ff")
        .set("OriginalFilename", "ff.exe")
        .set("LegalCopyright", "GPL-3.0-or-later");
    if let Err(e) = res.compile_for("ff") {
        if std::env::var_os("FF_REQUIRE_RESOURCE").is_some() {
            panic!("Windows resource: {e}");
        }
        println!("cargo:warning=no exe icon/version resource: {e}");
    }
}
```
Facts: numeric `FILEVERSION`/`PRODUCTVERSION` = major.minor.patch.0 from `CARGO_PKG_VERSION_*` (pre-release part dropped), string `FileVersion`/`ProductVersion` = full `CARGO_PKG_VERSION` (`0.7.0-rc.1`); rc.exe is found via `RC_PATH`, `WindowsSdkVerBinPath` or the `KitsRoot10` registry key; `compile_for(bin)` prints `cargo:rustc-link-arg-bin=<bin>=...` [VERIFIED: winresource 0.1.34 source on docs.rs]. Do not add an application manifest (`set_manifest`): DPI awareness is set at runtime (`src/skin/window`, `Win32_UI_HiDpi`) and a manifest could change it. `build.rs` already prints `rerun-if-changed=src` and `Cargo.toml`, so the rc step reruns with them (cheap).

**The `.ico`:** the repo has no `.ico` yet (`assets/ferris.png` is a 64x64 tray PNG). Generate a multi-size icon (16, 24, 32, 48, 64, 128, 256; PNG-compressed frames) from `assets/ferris/rustacean-flat-happy.svg` with a new `ff-art icon` subcommand (ff-art already renders this SVG with `resvg` for `ferris`/`ferris_about`; add the `ico` feature to its `image` dependency), commit `assets/ferris/ferris.ico`, and add an `assets/README.md` row (derived from the CC0 SVG). Small sizes of a detailed crab read poorly; a square crop around the body (as `ferris.png` did) helps.

### Pattern 6: PUB-01 / PUB-02 text
- `assets/README.md`: add a short "Distribution decision" paragraph: the GGG UI art (`poe2/`) and the Fontin fonts are shipped credited and **compiled into `ff.exe`** (via `include_bytes!`), no asset file is installed into the install folder or the config folder; the GGG non-affiliation disclaimer stays (README Credits, ABOUT); assets will be replaced if GGG or exljbris object. Add the `ferris/ferris.ico` row. Keep the existing rows.
- `src/sync/client.rs` user agent: `"FerrisFilter/0.1 (github.com/Jonathas-Conceicao/poe2_myfilter; ...)"` -> `ferris-filter`, and use the real version, e.g. `concat!("FerrisFilter/", env!("CARGO_PKG_VERSION"), " (github.com/Jonathas-Conceicao/ferris-filter; contact: ...)")`. Phase 41's updater should reuse one constant (put it next to the client, `pub const USER_AGENT`). [VERIFIED: `git grep` finds no other code mention]
- `Cargo.toml` `repository` already says `ferris-filter`; `build.rs` derives the ABOUT link from `origin` (`https://github.com/Jonathas-Conceicao/ferris-filter.git`) with the Cargo value as fallback.
- `CHANGELOG.md` link references (lines ~614-620) still point at `.../poe2-myfilter/compare/v0.6...HEAD` etc. Author-owned file: either a minimal factual fix of the host path (allowed as an accurate light edit) or leave it to the Phase 42 author review, but record it. Note the new tags are `vX.Y.Z`, so the `0.7.0` compare link becomes `v0.6...v0.7.0`.

### Anti-Patterns to Avoid
- **PowerShell packaging script:** cannot be run by agents in this environment (Bash tool refuses `powershell`/`pwsh`); keep packaging in bash, use `shell: pwsh` only for CI-only one-liners.
- **`AppMutex=` in the Inno script:** nags before `--quit` can run.
- **`ff --quit` calling `acquire_named`:** it creates the mutex and can become the instance. Open, never create.
- **Holding the mutex handle while waiting** for the name to disappear: the handle keeps the name alive.
- **`cargo build --workspace` / `cargo test --workspace`:** builds `ff-art`.
- **Building before checking a clean tree:** `git describe --dirty` bakes `-dev` into ABOUT.
- **Locally built assets uploaded to the release:** releases come only from CI (provenance; SHA256SUMS then means something).
- **Adding CHANGELOG entries in this phase:** milestone rule; the rc notes are the existing `[Unreleased]` text.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Installer, uninstaller, ARP entry, shortcuts | custom self-extractor | Inno Setup 6 | Upgrade identity (AppId), uninstall registry, RM backstop |
| `.rc` file + VERSIONINFO | hand-written rc text | `winresource` | Finds rc.exe, numeric/string versions from Cargo, per-bin linking |
| Release create/upload/publish | REST calls with curl | `gh release create --draft ... files` + `gh release edit --draft=false` | Auth, retries, asset upload in one tool |
| Zip creation | custom Rust/zip crate step | `C:\Windows\System32\tar.exe -a` | Built into Windows, tested |
| Single-instance IPC for quit | a new pipe/socket | one more event in the existing gate | Same names, same failure policy, same tests |

## Runtime State Inventory

Not a rename/migration phase in code, but PUB-02 renames a public identifier, so briefly:

| Category | Items Found | Action Required |
|----------|-------------|------------------|
| Stored data | None: no store keyed by the repo name (verified: `git grep`) | none |
| Live service config | GitHub repo already named `ferris-filter` (public, default `main`, Actions enabled `allowed_actions: all`, default token `read`, immutable releases off, tags v0.1..v0.6, zero releases/runs) [VERIFIED: gh api] | Workflow sets `contents: write` itself; no settings change needed |
| OS-registered state | None yet (no installer has ever run). After this phase: Inno writes an uninstall key under HKCU keyed by AppId, so the AppId must never change | Commit AppId once |
| Secrets/env vars | None needed (`GITHUB_TOKEN` only) | none |
| Build artifacts | `dist/` (new output) | add `/dist/` to `.gitignore` |

## Common Pitfalls

### Pitfall 1: `-dev` / `-dirty` baked into the release exe
**What goes wrong:** ABOUT shows `0.7.0-rc.1-dev` and Phase 41 compares wrong.
**Why:** `build.rs` uses `git describe --tags --match v* --dirty`; shallow checkout loses tags, a tracked file changed by an earlier step marks the tree dirty.
**How to avoid:** `fetch-depth: 0`; assert a clean tree right before `cargo build --release`; assert `git describe ... == $GITHUB_REF_NAME` after. `build_info::version_from_describe("v0.7.0-rc.1")` returns `0.7.0-rc.1` (verified by reading the rsplit logic: two parts only, so no `-dev`). `origin` on the runner is the plain https URL (checkout keeps the token outside the URL, and `web_url` strips credentials anyway).

### Pitfall 2: Cargo.lock not updated for the rc version
**What goes wrong:** `--locked` fails on the runner after `version = "0.7.0-rc.1"` is set in `Cargo.toml`.
**How to avoid:** bump with a local `cargo build` (or `cargo update -p ferrisfilter --offline`) and commit `Cargo.toml` + `Cargo.lock` together before tagging. `ff-art` keeps its own version.

### Pitfall 3: CRLF in CHANGELOG extraction
**What goes wrong:** headings do not match (`## [Unreleased]\r`) or notes carry `\r`.
**Why:** CHANGELOG.md is `i/lf w/crlf` on Windows checkouts (verified locally; the runner has the same autocrlf).
**How to avoid:** strip `\r` in awk (`sub(/\r$/, "")`) before matching. Stop at the next `## [` heading **or** at the link-reference block (`^\[[^]]+\]: `), otherwise extracting the last section would include the link list.

### Pitfall 4: VERSIONINFO needs numbers
**What goes wrong:** ISCC refuses `VersionInfoVersion=0.7.0-rc.1`; rc rejects non-numeric FILEVERSION.
**How to avoid:** pass `AppNumVersion=${ver%%-*}` separately; winresource already drops the pre-release from numeric fields.

### Pitfall 5: MSYS path conversion of `/D...` arguments
**What goes wrong:** Git Bash rewrites `/DAppVersion=...` as a path; ISCC errors or the define is missing (the `#error` guard catches it).
**How to avoid:** `MSYS2_ARG_CONV_EXCL='*'` on the ISCC call; pass the `.iss` path through `cygpath -w`. [ASSUMED: ISCC also accepts `-D`, unverified, do not rely on it]

### Pitfall 6: Wrong `tar`
Git Bash `tar` is GNU tar 1.35 (no zip). Call `/c/Windows/System32/tar.exe` explicitly. [VERIFIED]

### Pitfall 7: Re-running a failed tag
**What goes wrong:** a rerun's `gh release create` fails because a draft already exists, or a fix needs code and the tag points at old code.
**How to avoid:** infra-only failures before publish: use "Re-run failed jobs" (no new tag). If a draft was left behind, deleting it is a manual, documented author step: `gh release delete v0.7.0-rc.1 --yes` (add `--cleanup-tag` only when the tag is also to be removed; `git push origin :refs/tags/v0.7.0-rc.1` and `git tag -d` locally otherwise). Code fixes go out as `v0.7.0-rc.2` after bumping Cargo.toml/lock (CONTEXT: re-tag sparingly). Optionally make the publish step delete an existing **draft** of the same tag first (`gh release view "$tag" --json isDraft -q .isDraft` = true) so reruns are idempotent; never touch a published release.

### Pitfall 8: Floating nightly breaks the tag build
Accepted by the author. Mitigation inside the decision: print `rustc -vV` in the log; run `cargo test --locked` locally on the current nightly right before tagging (`rustup update nightly` first, since local is `nightly-2026-09-03` while CI pulls today's). rust-cache keys on the rustc version, so a new nightly means a cold cache (slower, not broken).

### Pitfall 9: SmartScreen / Mark-of-the-Web
Unsigned `setup.exe` and a `ff.exe` extracted from the downloaded zip both get "Windows protected your PC" -> More info -> Run anyway. Say so in `portable-readme.txt` (and the author's README in Phase 42). No UPX. The version resource (Pattern 5) also lowers heuristic noise. [ASSUMED: SmartScreen behaviour from general knowledge]

### Pitfall 10: Tests on the runner
All tests isolate homes (`FERRISFILTER_HOME`, `USERPROFILE`, `FERRISFILTER_GAME_DIR` to temp), `tests/sync_prices.rs` uses a FakeClient (no network), and instance tests use unique object names. Unverified until the first run: overlay tests that render with Fontin on a GPU-less runner, and timing. Run the full suite locally first and expect the first PR/tag run to be the real proof. [VERIFIED: test headers; ASSUMED: runner behaviour]

### Pitfall 11: Agent cannot run PowerShell; executor/dry-run planning
This session's Bash tool refuses `powershell.exe`/`pwsh`. Plans must not require the executor to run PowerShell locally; CI may use it. ISCC itself is a normal exe; if the local Inno install is missing, the local dry-run step is an author checkpoint (`winget install --id JRSoftware.InnoSetup -e`).

### Pitfall 12: Uninstall while `ff` is open
`[UninstallRun] ff.exe --quit` runs first; if the user leaves the Unapplied changes popup open, `--quit` returns 1 after its timeout and Inno reports it could not remove `ff.exe`. Acceptable and rare; mention it in the verification notes. (A `[Code]` Retry loop in `InitializeUninstall` is possible but runs before the uninstall confirmation, which would quit `ff` even if the user then cancels.)

### Pitfall 13: Setup.exe from a non-ASCII `.iss`
Inno 6 reads a script without BOM as ANSI; keep the script ASCII (no smart quotes, no em dashes in messages). Inno 7 changed encoding rules too, so ASCII is safe on both.

## Code Examples

### `installer/release-check.sh` (REL-02, notes)
```bash
#!/usr/bin/env bash
# Usage: release-check.sh <tag> <notes-out>; prints version=, prerelease= lines for $GITHUB_OUTPUT
set -euo pipefail
tag="$1"; out="$2"
[[ "$tag" =~ ^v([0-9]+)\.([0-9]+)\.([0-9]+)(-[0-9A-Za-z.-]+)?$ ]] \
  || { echo "::error::tag '$tag' is not vX.Y.Z[-pre]" >&2; exit 1; }
ver="${tag#v}"
cargo_ver="$(cargo pkgid -p ferrisfilter | sed 's/.*[#@]//')"
[ "$ver" = "$cargo_ver" ] || { echo "::error::tag $tag != Cargo.toml version $cargo_ver" >&2; exit 1; }
if [[ "$ver" == *-* ]]; then key="Unreleased"; pre=true; else key="$ver"; pre=false; fi
awk -v key="$key" '
  { sub(/\r$/, "") }
  /^## \[/ { if (found) exit; if (index($0, "## [" key "]") == 1) { found = 1; next } }
  /^\[[^]]+\]: / { if (found) exit }
  found { print }
' CHANGELOG.md > "$out"
[ -n "$(tr -d '[:space:]' < "$out")" ] \
  || { echo "::error::CHANGELOG.md has no non-empty '## [$key]' section" >&2; exit 1; }
echo "version=$ver"
echo "prerelease=$pre"
```
(`cargo pkgid` needs the registry index only when the lock is stale; on the runner it runs after `rustup` and works offline against `Cargo.lock`.)

### `portable-readme.txt` content outline
FerrisFilter <ver> portable: run `ff.exe`; settings live in `%APPDATA%\FerrisFilter` (same as the installed version); PoE2 must run windowed / windowed fullscreen; Ctrl+Shift+Z; tray Quit exits; SmartScreen note; project URL; GPL-3.0-or-later (LICENSE.txt); GGG non-affiliation sentence. Short, ASCII.

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| Tags `vX.Y` (v0.1..v0.6) | `vX.Y.Z[-pre]` from v0.7.0 | This phase (REL-05) | `build_info` already handles both; PROJECT.md Git Workflow step "annotated `vX.Y`" must say `vX.Y.Z`, and the close-out note that `/gsd-complete-milestone` checks Cargo version vs tag stays, now with the 3-part tag. The final `v0.7.0` push on `main` at milestone completion **triggers the release**, so the `## [0.7.0]` section must exist by then (complete-milestone moves `[Unreleased]` into it) |
| rustup 1.28.0 no auto-install | auto-install on by default again (1.28.1+), `rustup toolchain install` installs the file's toolchain | 2025-03 | Explicit install step is optional but clearer |
| `windows-latest` = Server 2022 | `windows-latest` = Server 2025 (VS2026 image); InnoSetup 6.7.1 on 2022, 2025 and 2025-VS2026 images, installed from Chocolatey `innosetup` | 2026 | Install path `C:\Program Files (x86)\Inno Setup 6\ISCC.exe`; fallback `choco install innosetup -y` if a future image drops it |
| Inno Setup 6 | Inno Setup 7.0-7.1 released (Jul-Aug 2026; mostly 64-bit-mode and default changes) | 2026 | Runner still has 6.7.1. Keep the script compatible with both; install 6 locally |

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | Tag-push workflows read the workflow file from the tagged commit; pushing a tag carries its commits | Pattern 1 | Workflow does not run; fix by committing the workflow first (already the plan) |
| A2 | windows-sys 0.61 feature `Win32_System_Diagnostics_ToolHelp` and constants `SYNCHRONIZE`/`PROCESS_SYNCHRONIZE`, `OpenMutexW` paths | Pattern 4 | Compile error, fixed by looking up the right module |
| A3 | Inno `Exec(..., ewWaitUntilTerminated)` keeps the wizard responsive enough during a ~10 s wait | Pattern 3 | Wizard looks frozen for a few seconds; cosmetic |
| A4 | `winresource` / `embed-resource` popularity numbers; slopcheck not run | Package audit | Human-verify before adding the build-dependency |
| A5 | SmartScreen/MOTW behaviour for unsigned exe and zip-extracted exe | Pitfall 9 | Readme wording only |
| A6 | Overlay tests pass on a GPU-less runner in reasonable time | Pitfall 10 | First CI run fails; fix locally, re-run |
| A7 | Shipping `LICENSE.txt` in the zip is required (CONTEXT says so); the installer installs `ff.exe` only, and GPL text is reachable from the release page/repo | Pattern 3 | If the author wants the license in the install folder or on a wizard page, add one `[Files]` line or `InfoBeforeFile`; GPL-3 asks that recipients get a copy of the license with object code, so the planner may ask the author |

## Open Questions (RESOLVED)

1. **License text in the installer.** CONTEXT: installer installs only `ff.exe`; the zip has `LICENSE`. GPL-3 section 4/6 expects the license to accompany object code. Recommendation: ask the author whether to add `LICENSE.txt` to `{app}` (not a program, so it does not contradict "no CLIs") or show it with `InfoBeforeFile` (no "I accept" page). Default if not asked: add `LICENSE.txt` to `{app}`.
2. **Relaunch after a silent install (Phase 41).** `skipifsilent` on the Launch entry means `/SILENT` updates do not restart `ff`. Phase 41 decides how it runs the installer; nothing to change now, but note it in the Phase 41 handoff.
3. **Timeout for `--quit`.** 10 s plus the installer Retry loop is the recommendation; the author may prefer waiting longer while the Unapplied changes popup is up.
4. **CHANGELOG link references** (`poe2-myfilter`): fix now as a factual one-line-per-link edit, or leave to Phase 42 author review. Record in the plan either way (PUB-02 says "everywhere").

Resolutions:
1. RESOLVED in 40-05 (Decision: LICENSE.txt installed into `{app}`), confirmed by the author in 40-07 Task 3 step 8 (drop it there if they choose).
2. RESOLVED in 40-05 (Decision: keep `skipifsilent`); handoff note for Phase 41 in 40-05's SUMMARY.
3. RESOLVED in 40-01 (Decision: `QUIT_TIMEOUT_MS = 10_000`, request sent once, installer Retry box in 40-05).
4. RESOLVED in 40-03 (Decision: not edited in Phase 40, no CHANGELOG edits; recorded for the Phase 42 author review in 40-03's SUMMARY).

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| Rust nightly (local) | build/test | yes | rustc 1.100.0-nightly 2026-09-03, rustup 1.29.1 | `rustup update nightly` before the dry run |
| rc.exe (Windows SDK) | winresource | yes | 10.0.22000.0 x64 | warning-only locally (FF_REQUIRE_RESOURCE unset) |
| Inno Setup ISCC (local) | local dry run of setup.exe | **no** | - | Author installs `winget install --id JRSoftware.InnoSetup -e` (6.7.3); until then the local dry run covers zip + checksums only |
| Inno Setup ISCC (CI) | release.yml | yes | 6.7.1 on windows-latest | `choco install innosetup -y` |
| bsdtar `C:\Windows\System32\tar.exe` | zip | yes (local), yes on Server 2025 [ASSUMED for runner, ships with Windows since 1803] | 3.8.9 | `7z a` (7-Zip is on the runner image) |
| sha256sum, awk, sed, cygpath (Git Bash) | scripts | yes | Git for Windows | - |
| jq | - | no locally (runner has it) | - | not needed (`cargo pkgid`) |
| gh CLI | publish, run queries | yes | 2.83.1 local, 2.102.0 runner | - |
| PowerShell from agent Bash tool | - | **refused by sandbox** | - | bash scripts; `shell: pwsh` only in CI |
| Python / slopcheck | package audit | no | - | packages tagged [ASSUMED] |

**Missing dependencies with no fallback:** none for CI. Local `setup.exe` dry run needs the author to install Inno Setup.

## Verification Map (nyquist disabled; checks the planner can wire into tasks)

| Req | Check | How |
|-----|-------|-----|
| REL-02 | release-check.sh rejects mismatches | run locally: `bash installer/release-check.sh v9.9.9 /tmp/n` -> exit 1; matching rc tag with current `[Unreleased]` -> exit 0 and non-empty notes; a final `v0.7.0` before Phase 42 -> exit 1 (no section) |
| REL-01/03 | release produced | after the tag push: `gh run watch -R Jonathas-Conceicao/ferris-filter`, `gh release view v0.7.0-rc.1 -R ... --json isDraft,isPrerelease,assets,body` (3 assets, prerelease true, draft false, body = Unreleased text) |
| REL-04 | triggers | `on:` blocks contain only `push.tags` / `pull_request` (grep) |
| INST-02/03/04 | installer behaviour | manual UAT: run setup (no UAC prompt), Start menu "FerrisFilter", `%APPDATA%\FerrisFilter` exists, ARP entry; install again while `ff` runs (clean, then with unapplied edits -> popup); uninstall keeps `%APPDATA%\FerrisFilter` |
| exe resource | icon + version | `(Get-Item ff.exe).VersionInfo` in CI; Explorer properties manually |
| `--quit` | unit + manual | instance tests (NotRunning, signal path), debug-instance manual run |
| PUB-02 | names | `git grep -i -E "poe2.myfilter" -- . ':!.planning'` returns only CHANGELOG link refs (or nothing if fixed) |

## Security Domain

| ASVS Category | Applies | Standard Control |
|---------------|---------|-----------------|
| V2/V3 Authentication, Sessions | no | - |
| V4 Access Control | yes (CI token) | `permissions: contents: write` only in release.yml, `contents: read` in ci.yml; PR workflow never gets write; no secrets |
| V5 Input Validation | yes | Tag regex + exact version equality before any build; `--quit` takes no input beyond the flag |
| V6 Cryptography | yes (checksums) | SHA-256 via `sha256sum`; never hand-rolled |
| V10/V14 Supply chain / config | yes | Actions pinned by commit SHA; `--locked`; artifacts built only in CI from the tag; no UPX; per-user install, no elevation |

| Pattern | STRIDE | Mitigation |
|---------|--------|------------|
| Malicious PR modifying workflow to publish | Elevation | `pull_request` (not `pull_request_target`) runs with a read token |
| Tampered download | Tampering | SHA256SUMS.txt published with the assets (Phase 41 verifies it); does not protect against account compromise (documented) |
| Another process spoofing the instance events | Spoofing | Objects live in `Local\` with the default same-user DACL; `--quit` only asks, the instance applies its own quit rules |
| Elevated `ff` or game | Elevation | Per-user install, `postinstall` launches as original user |

## Sources

### Primary (HIGH confidence)
- Repo: `Cargo.toml`, `build.rs`, `src/build_info.rs`, `src/overlay/instance.rs`, `src/overlay/mod.rs`, `src/overlay/app.rs` (tray Quit branch, `begin_shutdown`, show-request poll), `src/overlay/apply.rs` (`quit_step`), `src/overlay/win.rs` (`process_image_path`), `src/defaults.rs`, `src/locations.rs`, `src/sync/client.rs`, `.gitattributes`, `.gitignore`, `CHANGELOG.md`, `assets/README.md`, `.planning/PROJECT.md`
- gh api: actions/runner-images README, `Windows2025-Readme.md`, `Windows2025-VS2026-Readme.md`, `toolset-2025-vs2026.json` (InnoSetup via choco), `Windows2022-Readme.md`; tags/SHAs for actions/checkout and Swatinem/rust-cache; jrsoftware/issrc releases; repo settings of Jonathas-Conceicao/ferris-filter
- https://jrsoftware.org/ishelp/topic_consts.htm, https://jrsoftware.org/ishelp/topic_scriptevents.htm, https://jrsoftware.org/ishelp/topic_runsection.htm, https://jrsoftware.org/ishelp/topic_setup_closeapplications.htm
- https://docs.rs/winresource/latest/winresource/ (source of `new`, `compile_for`, rc lookup), https://docs.rs/embed-resource/latest/embed_resource/, https://docs.rs/image/0.25/image/codecs/ico/struct.IcoEncoder.html
- Local `gh release create/edit/upload --help`, `winget search`, `cargo info winresource`, bsdtar zip test

### Secondary (MEDIUM confidence)
- [Announcing rustup 1.28.1](https://blog.rust-lang.org/2025/03/04/Rustup-1.28.1/), [rustup 1.28.0](https://blog.rust-lang.org/2025/03/02/Rustup-1.28.0/), [rustup 1.28.2](https://blog.rust-lang.org/2025/05/05/Rustup-1.28.2/)
- https://jrsoftware.org/files/is7-whatsnew.htm (Inno 7 changes)
- Milestone research `.planning/research/STACK.md`, `PITFALLS.md` (Pitfalls 1, 2, 8, 11, 12, 14)

### Tertiary (LOW confidence)
- [Font Squirrel Fontin license page](https://www.fontsquirrel.com/license/Fontin): exljbris EULA restricts passing the font file to third parties; the author has decided to ship Fontin compiled in and credited, with replacement if exljbris objects. Recorded, not re-litigated.

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH (runner contents, action SHAs, gh flags, winresource source all checked this session)
- Architecture: HIGH for workflow/packaging, MEDIUM for `--quit` Win32 details (constant/feature names unverified until compile)
- Pitfalls: HIGH for repo-specific ones (describe, CRLF, MSYS, tar, PowerShell refusal), MEDIUM for SmartScreen/runner test behaviour

**Research date:** 2026-10-18
**Valid until:** 2026-11-18 (runner images change monthly; re-check the InnoSetup line if the first run cannot find ISCC)
