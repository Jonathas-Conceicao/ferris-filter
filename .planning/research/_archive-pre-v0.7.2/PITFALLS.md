# Pitfalls Research

**Domain:** First public release of an existing Rust/egui Windows overlay (FerrisFilter v0.7): release CI, unsigned installer, config relocation, self-updater, strictness presets, league picker
**Researched:** 2026-10-17
**Confidence:** MEDIUM. Repo-specific findings (HIGH) come from reading `CLAUDE.md`, `PROJECT.md`, `rust-toolchain.toml`, `build.rs`, `src/paths.rs`, `.gitattributes`, `assets/README.md`, `src/overlay/screenshot.rs`. Ecosystem claims (SmartScreen, GitHub API limits, runner behaviour, Inno Setup behaviour) are from training knowledge and were not re-verified online in this pass; they are marked MEDIUM/LOW and should be confirmed in the phase that uses them.

## Critical Pitfalls

### Pitfall 1: Tag name `vX.Y` vs Cargo version `X.Y.0` vs CHANGELOG heading

**What goes wrong:**
The project's tags are `v0.6`, `v0.5`, ... (two components; `git tag` confirms), while `Cargo.toml` is `0.6.0` and CHANGELOG headings are `## [X.Y.0]`. A workflow that triggers on `v*` will fire, but anything that strips the `v` and compares, parses with a semver parser, or greps CHANGELOG for the tag text gets `0.7` and fails or mismatches. The release can publish with empty notes, a wrong installer version, or an updater that thinks `0.7` is not newer than `0.7.0`.

**Why it happens:**
Three naming schemes grew independently. `build.rs` already derives `FF_VERSION` from `git describe --match v*`, so the ABOUT tab version and Cargo version can also disagree.

**How to avoid:**
- Decide once: either keep `vX.Y` tags and normalise to `X.Y.0` in exactly one helper (shared by workflow script and the in-app updater), or switch to `vX.Y.0` tags starting v0.7. Recommended: switch to `vX.Y.0` (matches Cargo and GitHub Releases convention, semver crates parse it) and document the one-time break.
- First workflow step: fail if tag (minus `v`) != root `Cargo.toml` version, and fail if CHANGELOG has no `## [<version>]` section. Extract notes with that same version.
- `/gsd-complete-milestone` already checks Cargo version vs tag; update it to the new tag format.
- Updater parses both `FF_VERSION` and the release tag with the same function; treat `0.7` as `0.7.0`; ignore `-dev` / `-N-gSHA` suffixes of local builds (a dev build must compare as a pre-release of its base, not as newer than the release).

**Warning signs:**
Release page shows empty or whole-file notes; installer filename says `0.7`; "Check for updates" says up to date on an older build, or offers the release you already run.

**Phase to address:** Release pipeline phase (workflow + tag policy), re-checked in the updater phase and the Release Review phase.

---

### Pitfall 2: Floating `nightly` toolchain in CI breaks (or silently changes) the release

**What goes wrong:**
`rust-toolchain.toml` says `channel = "nightly"`. CI installs whatever nightly is current on tag day. A nightly regression, a `poe_data_tools`-style nightly break, a new clippy/rustc lint, or a rustfmt change can fail the tag build, and re-tagging is awkward with milestone-squash flow. Two releases built from the same commit weeks apart can differ.

**Why it happens:**
The floating channel was a conscious dev convenience (the file's own comment says to pin a dated nightly if a nightly breaks `poe_data_tools`). A release pipeline needs reproducibility.

**How to avoid:**
- Pin a dated nightly (`nightly-YYYY-MM-DD`) in `rust-toolchain.toml` for the release; bump deliberately. Verify whether the root package really needs nightly (only `ff-art`'s dependency is documented as nightly-only); if not, ship the release from stable or a pinned nightly and keep `ff-art` out of CI.
- Use `cargo build --release --locked` and `cargo test --locked`; commit `Cargo.lock` (already committed). Build only the root package (`ff`, `ff-cli`, `ferrisfilter`); `ff-art` is a non-default workspace member and needs a game install.
- Do not use `-D warnings` in CI: clippy is at a 121-warning baseline. Run fmt check only, or clippy without deny.
- Cache with `Swatinem/rust-cache`, but never publish from a cache-poisoned job without a clean `--locked` build.
- Run a dry-run workflow (manual `workflow_dispatch` or a `v0.7.0-rc.N` tag) before the real tag.

**Warning signs:**
CI green last week, red today with no commit; `rustup` downloading components mid-job; release binary size differs between identical commits.

**Phase to address:** Release pipeline phase (first task: pin toolchain + dry-run).

---

### Pitfall 3: Build-time version/`git describe` and dirty-tree in CI

**What goes wrong:**
`build.rs` bakes `FF_VERSION` from `git describe --tags --match v* --dirty`. In CI: (a) a shallow checkout (`fetch-depth: 1`) may lack the tag history, silently falling back to `CARGO_PKG_VERSION`; (b) if any step before the build modifies tracked files (tests that regenerate fixtures, `cargo` refreshing `Cargo.lock`, CRLF normalisation), `--dirty` appends `-dev` and the shipped ABOUT tab and updater compare see `0.7.0-dev`; (c) `git remote get-url origin` yields the Actions token-bearing URL form, which `web_url` must not leak into the ABOUT link text.

**How to avoid:**
Checkout with `fetch-depth: 0` and `fetch-tags`; build the release artifacts first (clean tree, `--locked`), then run tests; assert in CI that `ff --version`-equivalent (or a tiny smoke) prints exactly the tag version with no suffix; check the ABOUT repo link in a built exe contains no credentials. Prefer injecting the version from the workflow env over `describe` for release builds.

**Warning signs:**
Release exe reports `-dev`; ABOUT link text contains `x-access-token`; update check offers the same version as installed.

**Phase to address:** Release pipeline phase; verified again in updater phase.

---

### Pitfall 4: Tests that depend on the real Windows profile, and a new path layer that ignores the env vars the tests set

**What goes wrong:**
Existing code and tests key off `USERPROFILE` (`src/install.rs`, `src/run.rs`, `src/cli.rs`, `tests/cli_errors.rs`, `tests/dynamic_filter.rs`, screenshot guard) and `FERRISFILTER_HOME`. If the new config dir / game folder resolution calls the Windows known-folder API (`SHGetKnownFolderPath`, or the `dirs`/`directories` crates, which do this on Windows), it ignores `USERPROFILE`, `APPDATA` and `LOCALAPPDATA` overrides. Result: tests silently read/write the developer's (or CI runner's) real `%APPDATA%\FerrisFilter` and real Documents folder; and the debug `--screenshot` mode, whose safety guard only checks that `FERRISFILTER_HOME`, `USERPROFILE` and the derived game folder are under temp, would now write the author's real live `styles.json` / settings. This is the worst data-loss path in the milestone.

**Why it happens:**
OS-correct resolution (OneDrive/localized Documents) and env-var-based test isolation are opposites. The v0.5/v0.6 isolation story is built entirely on env vars.

**How to avoid:**
- One path module with an injectable provider: `Dirs { config, cache, documents }` resolved once; production uses known-folder APIs, tests and screenshot mode inject temp dirs. `FERRISFILTER_HOME` keeps overriding everything (config and cache both under it, so existing tests and the dev workflow keep working).
- Extend the screenshot guard (`paths_under_temp`) to the new directories (config dir, cache dir, resolved documents/game folder) and make it refuse if any resolves outside temp. Add a test that fails if screenshot mode can resolve a real known folder.
- Add a lint-style test: no production code path reads config/cache/documents except through the provider (grep test for `APPDATA`, `dirs::`).
- On the CI runner assume no PoE2 install and an arbitrary profile path; no test may assume `Documents\My Games\Path of Exile 2` exists.

**Warning signs:**
`%APPDATA%\FerrisFilter` appears after `cargo test`; tests pass locally but differ in CI; screenshot run changes the author's live files.

**Phase to address:** Config relocation phase (provider + guard first, before any feature uses the new dirs). Game-folder picker phase reuses it.

---

### Pitfall 5: Migrating the author's (and v0.6 users') live app-root files

**What goes wrong:**
Today `styles.json`, `tiers.json`, `prices.json`, `ff-settings.json` live beside the exe/repo root, and the app root is located by walking up to `styles-default.json`. After the change: (a) the author's edited `styles.json`/`tiers.json` are "lost" because the new location is empty and defaults are embedded, silently resetting their palette; (b) an auto-migration that moves or deletes the old files destroys the only copy; (c) running the dev build from the repo (`cargo run`) now reads the real `%APPDATA%` instead of the repo copies, mixing dev and installed state; (d) the old walk-up marker (`styles-default.json` beside the exe) no longer exists in an installed layout, so `resolve_from` returns `NotFound` or latches onto an unrelated ancestor; (e) `ff-settings.json` holds a game-folder filter name relative to the old game folder assumption, now must coexist with a new game-folder field.

**How to avoid:**
- One-time, copy-only migration: if the new config dir has no `styles.json`/`tiers.json`/`ff-settings.json` and a legacy app-root copy exists (legacy resolution = old `FERRISFILTER_HOME` / walk-up / cwd), copy it, leave the original untouched, record a `migrated_from` note, and say so once in the status line. Never overwrite existing new-location files. Never delete legacy files.
- Keep ids intact: files with ids must round-trip byte-identically (existing guarantee); migration copies bytes, it does not reparse and rewrite.
- Prices cache is regenerable: do not migrate `prices.json`; just resync (but see Pitfall 9 on stale league).
- Define the dev story explicitly: repo runs set `FERRISFILTER_HOME=<repo>` (a `.cargo/config.toml` `[env]` entry or documented), keeping the existing "Development warning" true.
- Remove the `styles-default.json` app-root marker semantics from `paths.rs`; defaults come from `include_str!`. Keep a compatibility read path only for the migration.
- Unknown-key preservation in `ff-settings.json` must survive the move (existing guarantee); add a test with an old-format file.
- Test on the author's real files (copy to a temp home) before release.

**Warning signs:**
First launch after upgrade shows default colours; `FerrisFilter` folder created empty; migration test uses synthetic files only.

**Phase to address:** Config relocation phase (migration is its own task with a real-file fixture).

---

### Pitfall 6: Embedded defaults drift from the committed fixtures and `artifact_guard`

**What goes wrong:**
`styles-default.json` / `tiers-default.json` are read from disk by tests and `artifact_guard` (in `tests/cli_generate.rs`) and are the app-root marker. Embedding them with `include_str!` while tests keep reading the file path creates two sources that can diverge (e.g. someone adds a second copy under `assets/`). The style rework (no custom sounds, revised built-ins) changes the generated filter, so `tests/fixtures/seed-universal.filter` and snapshots must be re-baselined; doing it piecemeal across phases triggers repeated churn, and a half-done rework can leave `generate` output and the economy section disagreeing about `uwu.mp3`/`CustomAlertSound`. `CLAUDE.md` also documents `CustomAlertSound "uwu.mp3"` in the S/A tiers and the `assets/uwu.mp3` requirement in many places.

**How to avoid:**
- Single source: keep the repo-root `styles-default.json` and embed that exact file with `include_str!`; tests and runtime then read the same bytes. Add a test asserting the embedded string equals the file (cheap guard).
- Land the style rework in one phase and re-baseline fixtures once, deliberately, as the last emitter-affecting task (precedent: v0.3 "one consolidated fixture re-baseline").
- Treat `generate` (reads beside the inputs, uses `-default` fallback) and the overlay differently only in resolution, not content.
- Sweep docs/tests for `uwu.mp3` and `CustomAlertSound`; keep the custom-sound capability in the editor (users still may pick folder sounds) but not in built-ins. Check the sound validation tests that reference built-in custom sounds.
- Resist the temptation to edit `styles-default.json` for strictness reasons; strictness only changes the live `styles.json` visibility, never defaults.

**Warning signs:**
`artifact_guard` diff after an unrelated change; two `styles-default.json` files in the tree; `grep uwu` still hits `styles-default.json` or tests.

**Phase to address:** Style rework phase (must precede or be coordinated with Strictness Selector); config relocation phase owns embedding.

---

### Pitfall 7: Strictness Selector derived-state edge cases

**What goes wrong:**
The selection is derived from S-F built-in visibility (`Show`/`Hide` `action`), with seven modes (Custom + hide nothing, F, E-F, D-F, C-F, B-F). Failure modes: (a) the matcher treats "F hidden only" the same as the `hidden` required style or confuses the `F` style (Show action, font 25) with the `hidden` style; (b) a user who hid only `S` or a non-suffix set sees Custom, but clicking a preset overwrites their hand-chosen visibility without a warning; (c) the preset also flips nothing about custom styles or rules that carry their own Hide; (d) clicking a preset marks the CONFIG tab as edited (` *`) but "Custom" has no defined click effect (should it revert to the last custom state? do nothing?); (e) Revert/Apply/Discard interplay: preset click is an in-memory style edit that must go through the same all-or-none Apply and undo as a switch toggle; (f) economy tier blocks for hidden tiers become `Hide` blocks, and the generated economy for a hidden tier S must not be possible via presets, but is via hand edit; (g) it silently changes `generate` output only through `styles.json`, so the byte-for-byte fixture baseline must use defaults (hide nothing).

**How to avoid:**
- Implement as a pure function `strictness_from_styles(&Styles) -> Mode` and `apply_mode(&mut Styles, Mode)` with unit tests for all 7 modes plus each non-suffix pattern, and property-style test: apply_mode then derive is identity for the six presets.
- Define Custom click: no-op (it is a state, not an action), tooltip says so; or have it restore the pre-preset snapshot. Pick one in the plan.
- Only touch the `action` of the nine required built-ins S, A, B, C, D, E, F by id (`builtin:S` ...), not by name, and never `unchanged`, `hidden`, `Recycle`, `kite`, `exceptional`.
- Show a tooltip on each button naming exactly which tiers it hides. Do not add a text wall (overlay UI is intentionally not texty).
- Tests: preset change is one revertible edit; Apply writes `styles.json` with the existing byte-stable formatting; rules styled with S-F update on Apply (existing mechanism).

**Warning signs:**
Selector shows Custom right after clicking a preset; hand edit of one switch flips to a wrong preset; fixture diff after adding the selector.

**Phase to address:** Strictness Selector phase.

---

### Pitfall 8: Self-updater replacing a running, tray-resident exe

**What goes wrong:**
`ff` hides to the tray and keeps running; the user rarely quits it. An updater that downloads and launches the installer while `ff.exe` is running hits a locked exe (installer fails or schedules a reboot replace), or the installer succeeds but the old process keeps running (single-instance guard then makes the new launch just summon the old window). Overwriting the exe in place from within itself fails on Windows. Worse, quitting `ff` to let the installer proceed must follow the existing Quit rules (unapplied changes popup, waiting for an in-game reload).

**How to avoid:**
- Do not self-overwrite. Download the installer to a temp dir, verify, spawn it detached, then run the existing Quit path (respecting the Unapplied-changes popup; only quit after Apply or explicit discard). The installer must handle "already running" itself.
- Installer: use Inno Setup `AppMutex` (the overlay already has a named single-instance object; expose or create a dedicated mutex) with `CloseApplications=yes`/`RestartApplications` so the old `ff.exe` is closed; add a prompt-free `/SILENT`-like flow only if tested.
- Keep a stable installer `AppId` and install location so upgrades are in-place, shortcuts persist and Add/Remove Programs has one entry.
- Verify the downloaded file: HTTPS only, expected asset name pattern, size > 0, and compare the SHA-256 published in the release (generate `SHA256SUMS` in CI and download it). Without signing, the checksum is the only integrity signal; note it does not defend against a compromised release account.
- Never auto-run the installer; the requirement already says "offers to run it". Make the confirm button explicit and state the version.
- Test the upgrade path (install v0.7.0 -> run -> update to a test v0.7.1) on a clean VM before release, with `ff` running in the tray.

**Warning signs:**
"File in use" dialog from installer; after update the About tab still shows the old version; two Start menu entries; update flow leaves the tray icon of the old process.

**Phase to address:** Updater phase (after installer phase exists so both can be tested together).

---

### Pitfall 9: poe.ninja league list instability (the `indexed` flag problem) leaks into the picker

**What goes wrong:**
The pinned `OVERLAY_LEAGUE` exists because `indexed` flips between leagues. Replacing the pin with a picker populated from poe.ninja can: (a) default-select the outgoing league during a transition; (b) list leagues with no PoE2 economy data (empty price sets that look like a successful sync); (c) persist a league name that disappears at league end, so every later sync fails with "no such league", or worse a cached `prices.json` for last league is silently used; (d) offer SSF/hardcore/Standard variants with confusing names; (e) the list fetch failing leaves the picker empty and Sync disabled with no recovery; (f) league + price source choices interact: "fallback prices" is exclusive with Sync, and tier cutoffs edited in CONFIG already have no effect with fallback (documented debt).

**How to avoid:**
- Default = persisted choice if still in the fetched list; else pick by an explicit rule (newest non-Standard softcore trade league, not by `indexed` alone); never silently change a persisted league: show "league X is no longer available" and require a pick.
- Record the league in `prices.json` (already done) and show a mismatch between picker and last-synced league in the status line/economy panel.
- Treat a sync that returns an empty/very small item set as failure (the lenient parser makes success likely). Keep failing loudly, as the pin did.
- Cache the league list (short TTL) so an offline start still offers the persisted league; handle timeouts without blocking the UI thread (existing worker-thread pattern).
- Price source toggle is stored; Sync disabled with a clear tooltip when fallback is selected; Apply with fallback must say so in the status line (the deferred "surface `price_source`" debt becomes user-facing now; do it here).
- Network layer: keep `reqwest` + rustls ring (existing); set a real `User-Agent`; sensible timeouts.

**Warning signs:**
Sync "succeeds" with few items; picker lists a league you cannot sync; post-league-end Sync errors on first launch.

**Phase to address:** League/price source phase. Needs a short live-API spike at phase start (`needs research`).

---

### Pitfall 10: GitHub Releases API: rate limits, prereleases, drafts, assets

**What goes wrong:**
Unauthenticated GitHub API calls are limited per IP (60/hour, MEDIUM confidence, verify); shared IPs (campus, VPN, CGNAT) hit 403/429 fast. Requests without a `User-Agent` are rejected. `/releases/latest` excludes prereleases and drafts, so an `-rc` release is never offered (fine) but a beta tester on a prerelease is never told about newer prereleases. A newer release whose installer asset is not yet uploaded (workflow still running) is offered but has no asset. Asset name matching by substring picks the zip instead of the installer. Parsing errors on an unexpected JSON shape produce a crash on the UI thread.

**How to avoid:**
- Only check on explicit click (as specified, About tab), never at startup; cache the last result for the session; treat 403/429 as "could not check, try later" with the reset time, not as an error dialog.
- Use `https://api.github.com/repos/<owner>/<repo>/releases/latest` with `User-Agent: FerrisFilter/<version>` and `Accept: application/vnd.github+json`; ignore `draft`/`prerelease` explicitly; require the expected installer asset (`FerrisFilter-<version>-setup.exe`) and the checksums asset; if absent say "release is still being published".
- Workflow: create the release as a draft, upload all assets, publish last, so `latest` never points at an incomplete release.
- Repository link comes from build info; make sure it points at the public repo and the repo is public before the tag; a private repo makes every unauthenticated check 404.
- Keep the HTTP code in a function with the transport injectable so tests use canned JSON (rate-limited, no assets, prerelease first, malformed).

**Warning signs:**
"Up to date" when a newer release exists (prerelease filtering, wrong tag compare); 404 in testing before the repo is public.

**Phase to address:** Updater phase.

---

### Pitfall 11: Unsigned Rust GUI exe and installer: SmartScreen and antivirus false positives

**What goes wrong:**
A new unsigned installer from an unknown publisher triggers the SmartScreen "Windows protected your PC" screen (stated out of scope but still the first user experience), and unsigned Rust executables that register a global hotkey, send synthetic keystrokes (`SendInput`), touch the clipboard, enumerate windows by process name, and download and run an installer are heuristic bait for antivirus (Defender and third-party engines) and some Rust-built binaries are flagged more often than average (MEDIUM, community reports). A self-updater that downloads an exe and runs it looks like a dropper.

**How to avoid:**
- Be honest in README and release notes: tell users about the SmartScreen "More info -> Run anyway" step, give the SHA-256, and state that the project is open source with CI-built artifacts. Keep the Out of Scope decision, but document it.
- Build artifacts only in public GitHub Actions from the tag (no locally built uploads) so provenance is verifiable; consider GitHub artifact attestations (check availability; LOW confidence).
- Reduce heuristic triggers: `strip`/`lto` are fine but avoid UPX packing (strongly raises detection); embed a Windows resource with product name, version, company and icon (via `winres`/`embed-resource`); add an application manifest (DPI awareness already set at runtime; a manifest also avoids compat shims).
- Submit the release exes to Microsoft's false-positive portal and VirusTotal after each release and note results; budget time for it.
- Updater: HTTPS-only fixed repo URL, no arbitrary URL from the response (derive from the asset `browser_download_url` host check `github.com`/`githubusercontent`), checksum check, user confirmation.

**Warning signs:**
Defender quarantines the zip on first download; VirusTotal shows more than a couple of generic ML detections; users report the installer vanishing.

**Phase to address:** Installer phase (resources/manifest), Release pipeline phase (provenance, checksums), Release Review (docs wording).

---

### Pitfall 12: Installer scope, uninstall behaviour and per-user config

**What goes wrong:**
(a) Installing to Program Files needs elevation, and then the `ff` process (user level) and the game must "run at the same privilege level" or the in-game reload silently fails; an installer that launches `ff` elevated at the end of setup (the default "Run after install" checkbox from an elevated installer) starts an elevated overlay that can no longer be seen at same level... or the reverse. (b) Uninstall deletes `%APPDATA%\FerrisFilter` (users lose their styles) or leaves it forever with no mention; deleting the game-folder filter or `.ffbak` would be catastrophic. (c) Start menu shortcut and "Run at startup" confusion; a tray app that is never quit blocks uninstall. (d) No upgrade-in-place identity, producing duplicates. (e) The zip portable distribution has no config location story (it must use the same `%APPDATA%` unless `FERRISFILTER_HOME` is set or a `portable` marker is present).

**How to avoid:**
- Prefer a per-user install (Inno Setup `PrivilegesRequired=lowest`, install under `{localappdata}\Programs\FerrisFilter`) so no elevation, matches the user-level game and avoids the elevated-child problem; launch `ff` after install with `runasoriginaluser`.
- Uninstall removes only the installed program directory and shortcuts. Never touch `%APPDATA%\FerrisFilter`, `%LOCALAPPDATA%` cache (optional prompt "also remove settings?" default No is acceptable), and never anything in the game folder. State this in the uninstall page.
- Use `AppId`, `AppMutex` and `CloseApplications` (Pitfall 8). Pick Inno Setup (scriptable, easy CI via `iscc`, available on windows runners via choco; MEDIUM) unless the author prefers WiX; avoid MSI complexity for a "basic installer".
- Zip build: document that it writes to `%APPDATA%` too, or add a documented `FERRISFILTER_HOME`/portable marker.
- Include LICENSE and third-party notices in the installer (Pitfall 14).

**Warning signs:**
UAC prompt at install; overlay running elevated after setup; uninstall leaves `Programs\FerrisFilter` because ff is running; user config gone after reinstall.

**Phase to address:** Installer phase.

---

### Pitfall 13: Exclusive-fullscreen and first-run experience for strangers

**What goes wrong:**
New users who run PoE2 in exclusive fullscreen see nothing when pressing `Ctrl+Shift+Z`; the app cannot detect it. The hotkey only works while the game or overlay is foreground, so pressing it on the desktop does nothing. First launch with no game folder found (non-default Documents location, Steam/Standalone differences, non-English "My Games") shows a failing state. Autostart expectations: they double-click, a tray icon appears, no window (window may start hidden?). Elevated game blocks the hotkey and reload silently.

**How to avoid:**
- First-run state (once, not a text wall): the game-folder row in CONFIG shows the resolved path with a found/not-found indicator; the status line names the exact next step ("Set windowed mode in PoE2: Options -> Graphics", via tooltip). README gets a short "Requirements" box: windowed or windowed-fullscreen, same privilege level, press `Ctrl+Shift+Z`.
- Show the window on first launch (not tray-only) and on every launch (existing summon behaviour on second launch).
- Game-folder picker: resolve default via `SHGetKnownFolderPath(FOLDERID_Documents)` (handles OneDrive/localized), validate that the folder exists and contains or can create filters; accept a user pick via a native folder dialog (new dependency, e.g. `rfd`; check it works with egui/eframe event loop and does not freeze the overlay) or a text field with Browse; save in settings; do not create the `My Games` tree on a guess. `ferrisfilter install`/`apply` default `--filter` still hardcode `OneDrive\Documentos`: replace with the same resolver or users with English paths break (existing code in `install.rs`, `cli.rs`, `run.rs`).
- Keep the existing test discipline: resolver is pure with injected known-folder result.

**Warning signs:**
"I press the hotkey and nothing happens" issues; CLI help text still says `OneDrive\Documentos`; path with spaces/accents breaks `Path`-to-string conversions.

**Phase to address:** Game-folder picker phase; README wording in Release Review.

---

### Pitfall 14: Redistributing GGG art and Fontin fonts in a public binary

**What goes wrong:**
The repo is private-ish today; making it public and shipping binaries embeds GGG-owned UI art (`assets/poe2/`, minimap atlas, edited title bar) and the Fontin fonts (exljbris, shipped with the game). `assets/README.md` records "will be replaced if GGG objects" but the license of Fontin is not stated as redistributable (Fontin is distributed by exljbris under its own terms; the free fonts have a EULA that, MEDIUM/LOW, generally allows free use but restricts redistribution of the font files themselves, so copying from the game and shipping may be a violation independent of GGG). GPL-3.0 on the source plus embedded non-GPL assets also needs a clear statement that assets are excluded. A takedown after release means rebuilding every release.

**How to avoid:**
- Decision before the first public tag, not after: check GGG's published fan-content/asset policy and Fontin's license text (read, don't assume). If Fontin redistribution is not clearly allowed, plan a replacement face (an OFL font that looks similar, or keep Fontin out and load it from the user's local game install at runtime with a fallback font) and make the skin tolerate a missing file.
- Keep art isolated behind `skin::` so a replacement set can ship in a point release; mark the exact swap path in docs. Keep the disclaimer ("not affiliated with or endorsed by GGG") in README, installer license page and ABOUT (already there).
- Ship `LICENSE`, `assets/README.md` provenance and third-party notices in installer and zip.
- Mention in the release notes that art belongs to GGG.
- The `ff-art` tool and manifest are fine (extraction is local), but ensure the release workflow does not need the game files.

**Warning signs:**
No explicit permission found in a quick search; font license mentions "embedding" restrictions; the public repo contains the whole game-art set.

**Phase to address:** A short legal/provenance task in the Release pipeline phase (blocking before the tag); font fallback implementation in a skin task if needed. Likely needs research.

---

### Pitfall 15: In-game reload (key sending) surprises for new users

**What goes wrong:**
Existing behaviour (focus game, Enter, paste `/reloaditemfilter`, Enter) is on by default. For strangers: (a) if chat is open with text, the first Enter sends that text as a chat message; (b) it clobbers non-text clipboard content; (c) other tools' global hotkeys or keyloggers-lookalike behaviour: synthetic input is an AV heuristic (Pitfall 11) and might be construed as botting by players worrying about GGG's third-party tool policy (it sends no gameplay input, but strangers will ask); (d) a different filter than the one selected in game is reloaded; (e) elevated game drops the keys but status says `Reloaded in game`.

**How to avoid:**
- Default decision: consider shipping the toggle default **off** for first run (or ask once) since the opt-out default is author-tuned; at minimum put the clipboard/chat/privilege caveats in README and the toggle's tooltip, and name the exact keys sent.
- Document it in the README as "sends Enter, Ctrl+V, Enter to the game window; nothing else", and link GGG's third-party policy statement after checking it.
- Detect elevation mismatch if feasible (compare process integrity level of the game vs `ff` with `OpenProcessToken`) and change the status from "Reloaded in game" to a warning; if not feasible, keep wording hedged (already: "every key was sent").
- Never retry; keep as is.

**Warning signs:**
Bug reports about stray chat messages; users claiming "reload does nothing" (elevation); AV reports citing keyboard hooks.

**Phase to address:** Game-folder picker / first-run phase (default), Release Review (README).

---

### Pitfall 16: CRLF/line-ending and OS differences on the Windows runner break byte-exact tests

**What goes wrong:**
Fixtures are compared byte-for-byte (`seed-universal.filter`, parser fixtures with exact CRLF/LF). `.gitattributes` has `* text=auto`, so a CI checkout converts LF to CRLF on Windows runners when `core.autocrlf` is true; `*.filter text` makes the committed fixtures subject to conversion, so `artifact_guard` and snapshot (`insta`) tests can fail only in CI, or pass only because both sides are converted. Windows runners also have no display, no clipboard owner, and a shared session, so tests that register the global hotkey, create the named single-instance mutex, or touch the clipboard may fail or leak across tests, and parallel tests sharing a named object collide.

**How to avoid:**
- Make the test platform explicit: the release workflow runs the suite on `windows-latest` (the shipped target) and compare byte output with `text eol=lf` for fixtures that `generate` compares (set `*.filter text eol=lf` for non-parser fixtures; parser fixtures already `-text`). Run the suite once on the author's machine and once in CI from a clean clone before the first tag.
- Tests touching OS singletons (mutex name, hotkey, clipboard) are `#[ignore]`d or use unique per-test names (debug builds already use a separate instance name); confirm none run in CI by default or that they pass headless.
- Locally, `cargo test` while `ff.exe` runs: the linker cannot overwrite `target\debug\ff.exe` (existing note: use `CARGO_TARGET_DIR`); in CI this does not happen, but the release job must not start the built exe as a smoke test and then try to overwrite it. Smoke-test the installer by installing to a temp dir with `/VERYSILENT /DIR=...` and then uninstalling.
- Use `--test-threads` defaults but isolate any test that sets process env (env vars are process-global; `USERPROFILE`/`APPDATA` changes race with other tests). Prefer the injected provider (Pitfall 4).

**Warning signs:**
Snapshot diffs that show only `\r`; tests flaky only in CI; "Access is denied" when copying `ff.exe`.

**Phase to address:** Release pipeline phase (first green CI run on a throwaway branch/tag before relying on it).

---

## Technical Debt Patterns

| Shortcut | Immediate Benefit | Long-term Cost | When Acceptable |
|----------|-------------------|----------------|-----------------|
| Keep `rust-toolchain.toml` floating nightly in release CI | Zero config | Release breaks or changes on tag day | Never for releases; pin a dated nightly |
| `dirs` crate called directly in many modules | Fast to write | Untestable, breaks env-var isolation and screenshot guard | Never; one provider module |
| Updater overwrites `ff.exe` in place | No installer needed | Locked file, partial writes, AV flags | Never |
| Skip checksum file in release | Simpler workflow | No integrity signal for unsigned downloads | Never; trivial to add |
| Auto-delete legacy app-root files after migration | Tidy | Loses author's only copy if migration has a bug | Never in v0.7; revisit later |
| Hardcode `releases/latest` + one asset name | Quick | Breaks when asset naming changes (zip vs setup) | OK if tested by canned-JSON unit tests |
| Embed defaults by copying JSON into a Rust const | Avoids `include_str!` path | Drift from `styles-default.json` and fixtures | Never |
| Ship Fontin/GGG art without checking terms | Avoids delay | Takedown forces re-release | Only if the author accepts the risk knowingly, documented |
| Leave `--filter`/`--dest` defaults at `OneDrive\Documentos` | No CLI change | Wrong for every other user | Never; reuse the known-folder resolver |

## Integration Gotchas

| Integration | Common Mistake | Correct Approach |
|-------------|----------------|------------------|
| GitHub Actions | Default `GITHUB_TOKEN` lacks write; release step 403s | `permissions: contents: write` on the release job only; upload with `gh release` or `softprops/action-gh-release` |
| GitHub Actions | Tag trigger `v*` also fires for `v0.7-rc1` or odd tags | Validate tag shape and version match first; mark rc tags prerelease |
| GitHub Releases API | No `User-Agent`; treats 403 as hard error; ignores prerelease/draft flags | Send UA, handle rate-limit as soft failure, filter draft/prerelease |
| poe.ninja leagues | Trusting `indexed` or list order; assuming list is stable | Persisted choice, explicit default rule, empty-result = failure |
| Windows known folders | `std::env::var("USERPROFILE").join("Documents")` | `SHGetKnownFolderPath` (or crate using it) behind an injectable provider |
| Installer (Inno) | Install to Program Files, elevate, leave `ff` running | Per-user install, `AppMutex` + `CloseApplications`, stable `AppId` |
| Windows Defender / SmartScreen | Assuming "out of scope" means no user comms | README note, checksums, FP submissions, public CI provenance |
| Existing `ff-settings.json` | Rewriting known keys only and dropping unknown | Keep the "unknown keys preserved" guarantee through the move; test with old file |
| Existing single-instance + tray Quit | Updater kills the process | Use the app's Quit path including Unapplied-changes popup |

## Performance Traps

| Trap | Symptoms | Prevention | When It Breaks |
|------|----------|------------|----------------|
| Network call (league list, update check) on the UI thread | Overlay freezes over the game | Worker thread + channel as Sync already does; timeouts of a few seconds | Any slow/offline network |
| League list fetched every frame/open of the Economy tab | Rate limiting, jank | Fetch on explicit refresh and first open, cache for the session | Immediately with repaint loops |
| Installer download without progress/cancel | UI appears hung on slow links | Stream to temp file with progress and cancel | Downloads of several MB on slow connections |
| Release exe growth from embedded art + defaults | Large download, slower AV scan | Already ~0.5 MB atlas; check `cargo bloat` once, `opt-level`/`lto`/`strip` | Not a scale issue; matters for AV heuristics |

## Security Mistakes

| Mistake | Risk | Prevention |
|---------|------|------------|
| Updater runs a downloaded exe without verification | Supply-chain/MITM or account-compromise executes arbitrary code | HTTPS, fixed repo, host allowlist on download URL, SHA-256 check, explicit user confirm, temp dir with unique name |
| Download URL taken verbatim from API JSON | Redirect to attacker host if response tampered | Validate host and expected asset name; do not follow redirects off GitHub CDN hosts |
| Predictable temp installer path | Another local process swaps the file | Per-run random temp dir, verify hash after write and before run |
| Release notes/changelog text rendered unfiltered | Odd content in UI | Show plain text only, truncated |
| `ff-settings.json` game folder path joined unvalidated | Path traversal when picker adds free paths | Keep the bare-filename validation for filter names; canonicalize and require directory for the game folder |
| Tokens or credentials in the baked `origin` URL | Leaks into the shipped binary | Strip userinfo in `web_url`; assert in CI |
| Committing a signing/installer secret later | Leakage | None needed now (unsigned); keep it out of scope |

## UX Pitfalls

| Pitfall | User Impact | Better Approach |
|---------|-------------|-----------------|
| Strictness row text-heavy (descriptions of each preset) | Clutter, contradicts the "not texty" UI preference | Seven buttons, tooltips only; names carry the meaning |
| Picker changes silently overwrite hand-tuned style visibility | User loses custom Hide setup | Custom state clearly shown; presets are in-memory edits that Revert undoes |
| Update check shows raw errors (HTTP 403) | Looks broken | One status-line message: "Could not check (rate limited), try again later" |
| Fallback prices selected but Economy tab still looks live | Wrong expectations at league start | Visible price source in status line; Sync greyed with tooltip |
| Game folder not found shows an empty Rules tab | Dead end on first run | Config tab opens with the game-folder row focused and a clear status line |
| Install finishes with no visible window | "Did it work?" | Launch window on first run, tray tooltip names hotkey |
| Uninstall removes config without asking | Lost customizations | Keep config; offer opt-in removal |

## "Looks Done But Isn't" Checklist

- [ ] **Release workflow:** often missing tag-vs-Cargo-version check, `--locked`, draft-then-publish, checksum asset — verify with a throwaway `v0.7.0-rc.1` tag.
- [ ] **Release notes:** often an empty or whole-file CHANGELOG body — verify the extracted section equals the `## [0.7.0]` text only.
- [ ] **Installer:** often leaves a running `ff` unhandled — verify install over a running tray instance, upgrade over the previous version, and uninstall; confirm config under `%APPDATA%\FerrisFilter` survives.
- [ ] **Config relocation:** often tested only with synthetic files — verify with a copy of the author's real `styles.json`/`tiers.json`/`ff-settings.json`, byte-identical after migration, originals untouched.
- [ ] **Isolation:** `cargo test` and `--screenshot` leave `%APPDATA%`, `%LOCALAPPDATA%` and Documents untouched — verify by listing them before and after.
- [ ] **Embedded defaults:** exe works from an empty folder with no `styles-default.json` beside it — verify by running the release exe from a temp dir with `FERRISFILTER_HOME` unset.
- [ ] **CLI defaults:** `install`/`apply` no longer say `OneDrive\Documentos` — grep help text and defaults.
- [ ] **Game folder picker:** works with OneDrive-redirected, localized and non-ASCII paths — test with at least one.
- [ ] **Strictness:** all seven states round-trip; hand edit shows Custom; Revert and Apply behave like other CONFIG edits.
- [ ] **Style rework:** no `CustomAlertSound`/`uwu.mp3` in `styles-default.json`, fixtures re-baselined once, `CLAUDE.md` tables updated.
- [ ] **League picker:** persisted league that vanished is handled; empty sync result is an error.
- [ ] **Updater:** tested against canned JSON for rate limit, prerelease, missing asset, newer/older/equal/dev-suffix versions.
- [ ] **Public repo:** license files, asset provenance, and no personal paths/emails or `reference-old-filters` content in the tree or release artifacts.
- [ ] **Version surfaces:** Cargo, tag, installer, ABOUT and CHANGELOG all show the same version.

## Recovery Strategies

| Pitfall | Recovery Cost | Recovery Steps |
|---------|---------------|----------------|
| Wrong tag/version mismatch published | LOW | Delete the release and tag, fix, retag (before anyone downloads); otherwise cut a patch version |
| Nightly broke the tag build | LOW | Pin a dated nightly, rerun with `workflow_dispatch` on the tag |
| Migration lost the author's styles | LOW if originals kept | Copy back from the legacy location (migration never deletes) |
| Tests wrote to real `%APPDATA%` | MEDIUM | Restore from legacy copies; add guard test; delete stray files |
| Updater bricked an install | MEDIUM | Document manual reinstall from the Releases page; keep older installers available |
| AV/SmartScreen flags a release | MEDIUM | Submit FP report, mention in release notes, re-release only if packaging changed |
| GGG/Fontin takedown request | HIGH | Swap art set/font behind `skin::`, ship patch release, rewrite git history only if asked |
| Wrong league defaulted/stale prices | LOW | Re-sync with the right league; add the mismatch warning |

## Pitfall-to-Phase Mapping

Suggested phase ordering implied by dependencies: path provider and embedded defaults first (everything else reads config), then style rework, strictness, game-folder picker, league/price source, release pipeline and installer, updater last (needs installer and published asset names), Release Review.

| Pitfall | Prevention Phase | Verification |
|---------|------------------|--------------|
| 1 Tag/version/CHANGELOG mismatch | Release pipeline (+ updater, Release Review) | Workflow fails on mismatch in dry run; updater unit tests for `0.7` vs `0.7.0` |
| 2 Floating nightly | Release pipeline | Dated nightly in `rust-toolchain.toml`; rerun of the same tag yields same result |
| 3 `git describe` / dirty in CI | Release pipeline | Built exe version equals tag; ABOUT link has no credentials |
| 4 Env-var tests vs known folders | Config relocation (first) | No real-profile writes after tests/screenshot; guard test |
| 5 Migration of live files | Config relocation | Real-file migration test, originals untouched |
| 6 Embedded defaults vs fixtures | Config relocation + Style rework | `include_str!` equality test; single fixture re-baseline |
| 7 Strictness derivation | Strictness Selector | Pure-function tests for all modes and non-suffix sets |
| 8 Updater and running exe | Installer + Updater | Upgrade over running tray instance on a clean VM |
| 9 League list instability | League/price source (spike first) | Vanished-league and empty-sync tests; manual check across a league transition if available |
| 10 GitHub API limits/prereleases | Updater | Canned-JSON tests; draft-then-publish workflow |
| 11 SmartScreen/AV | Installer + Release pipeline + Release Review | VirusTotal/Defender scan results recorded; README note |
| 12 Installer scope/uninstall | Installer | Per-user install, config survives uninstall |
| 13 Fullscreen/first-run | Game-folder picker + Release Review | Fresh-profile VM run-through |
| 14 GGG art/Fontin redistribution | Release pipeline (blocking pre-tag task) | Written decision recorded in `assets/README.md` |
| 15 Reload key sending | Game-folder picker/first-run + Release Review | Default and README wording reviewed by author |
| 16 CRLF and runner differences | Release pipeline | First green `windows-latest` run on a throwaway branch |

Phases likely needing deeper research: League/price source (live poe.ninja behaviour), Installer (Inno Setup vs WiX, mutex/close behaviour), legal/provenance of Fontin and GGG art, Updater (API behaviour, attestation options).

## Sources

- Repository files read: `CLAUDE.md`, `.planning/PROJECT.md`, `rust-toolchain.toml`, `build.rs`, `Cargo.toml`, `src/paths.rs`, `src/overlay/screenshot.rs`, `.gitattributes`, `assets/README.md`, `README.md` credits, `git tag` (tags are `v0.2`..`v0.6`). (HIGH)
- GitHub REST API docs (rate limits, `releases/latest` semantics, User-Agent requirement) — from training knowledge, not re-fetched in this pass. (MEDIUM; verify at phase start)
- Inno Setup documentation (`AppMutex`, `CloseApplications`, `PrivilegesRequired`) — training knowledge. (MEDIUM)
- Windows known-folder API behaviour (ignores `USERPROFILE` overrides; follows OneDrive Known Folder Move) — training knowledge. (MEDIUM)
- SmartScreen/AV heuristics for unsigned Rust binaries — community reports, not verified. (LOW)
- Fontin and GGG asset redistribution terms — not verified; must be read from primary sources. (LOW)

---
*Pitfalls research for: FerrisFilter v0.7 public release and new features*
*Researched: 2026-10-17*
