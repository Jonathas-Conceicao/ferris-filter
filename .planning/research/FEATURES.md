# Feature Research

**Domain:** First public release of a Windows desktop overlay and filter editor for Path of Exile 2 (milestone v0.7)
**Researched:** 2026-10-17
**Confidence:** MEDIUM overall. The web sources were thin. NeverSink's seven-level structure and Inno Setup's per-user mode were confirmed. FilterBlade's per-level contents, the league UIs of Exiled Exchange 2 and Awakened PoE Trade, and the updater conventions come from training knowledge and are marked as such.

Scope: only the v0.7 features. The rule editor, styles, economy section, sync and Apply are already built.

## Feature Landscape

### 1. Strictness Selector (Config tab)

**How the reference tools behave**

| Fact | Source / confidence |
|------|---------------------|
| NeverSink's PoE2 filter has seven levels, Soft to Uber Plus Strict. Each step up hides more. | Confirmed by search. MEDIUM. |
| New players are told to start at Semi-Strict. Players raise the level as the clutter starts to bother them. | Confirmed by search. MEDIUM. |
| The Soft level exists because the game changed and Regular had to become stricter. Level names therefore drift between patches. | Confirmed by search. MEDIUM. |
| For a specific item the user wants kept, the standard advice is to go up one level and add an override at the top of the file. | Confirmed by search. MEDIUM. |
| Strictness is one global axis that touches every category: currency, gems, gear bases, flasks, waystones and uniques. Each level is a separate generated file, and switching means loading another file in game. FilterBlade adds per-section overrides on top. | Training knowledge. LOW to MEDIUM. The FilterBlade home page did not show the per-level contents. |
| The levels are item-count labels (Soft, Regular, Strict) that mean nothing until previewed. | Training knowledge. LOW to MEDIUM. |

**What users expect**
- One control, and one obvious "more or less clutter" direction.
- A fresh filter that is safe to start with.
- Hidden items still reachable (Alt-reveal in PoE2).
- A higher level never shows something a lower level hides.
- Stable names.

**Table stakes**

| Feature | Why expected | Complexity | Notes |
|---------|--------------|------------|-------|
| Exclusive button row, one state shown at a time | This is the specified UI, and it is the same single-axis model as FilterBlade and NeverSink | LOW | Reuse the skin button and switch widgets from `src/skin/`. |
| Selection derived from S-F `action` flags, with no stored "strictness" field | Specified. It means a hand edit shows Custom with no sync bug. | LOW | Pure function: styles → enum. A level matches only when the hidden set is exactly {} / {F} / {E,F} / {D,F-prefix}… up to {B..F}. Anything else, including S or A hidden or a gap such as E hidden and F shown, is Custom. |
| Clicking a level writes the S-F visibility in memory, with the same Apply and Revert semantics as any style edit | The all-or-none Apply is the existing contract. The Config tab already shows ` *` and the Discard popup. | LOW | It must go through the existing Config edit model so Revert, the unapplied indicator and the pending-style rendering of the Rules tab and economy section all work. |
| Level labels explain themselves on hover (which tiers are hidden) | A name like "Comfy" is opaque. NeverSink users always need the explainer. | LOW | A tooltip, not captions. The project memory says the overlay UI must not be texty. |
| Only S-F are touched | The other built-ins (unchanged, hidden, Recycle, kite, exceptional) have their own meaning, and custom styles belong to the user | LOW | The tooltip or the status line should say "price tiers only". |
| Clicking the already-selected level changes nothing | Idempotent | LOW | |
| S and A are never hidden by any level | Hiding the top tier is never wanted, and the specified levels agree (Div Farmer hides B-F) | LOW | |

**Differentiators**

| Feature | Value proposition | Complexity | Notes |
|---------|-------------------|------------|-------|
| Strictness measured in wealth (price tiers), not item count | The level follows the economy, so it adapts every league and every sync. NeverSink's lists are static and curated per patch. | LOW | It comes from the existing design, where tiers are price-driven. Do not expose a "what does Comfy hide" item list. |
| Reflects hand edits live (Custom, with no lock) | Neither NeverSink nor FilterBlade lets you edit the result and still see where you stand | LOW | Free from the derived design. |
| Per-style Show/Hide switches stay available under the selector | The user can go one level up and then keep one tier visible. This is the NeverSink "override" workaround built in. | none | Already built. |

**Anti-features**

| Anti-feature | Why requested | Why problematic | Alternative |
|--------------|---------------|-----------------|-------------|
| Strictness that also changes non-tier rules (gear fallback hiding, flask rules, area-level bands) | NeverSink's levels do this, so users may expect it | It couples the selector to `config.json` rules. It breaks the "derived from S-F visibility" invariant. It also makes Custom ambiguous. | Keep it tier-only and say so in the tooltip. Revisit for a later milestone only if users ask. |
| A stored `strictness` value in `ff-settings.json` or the filter | Looks easier than deriving | It drifts from the real styles and needs a reconciliation path | Derive every frame. |
| A clickable Custom that resets or "restores" the previous custom set | Looks symmetrical | Needs a saved copy of the previous visibility, which is more state to persist and keep honest | Custom is a display state. Revert already undoes a staged selection. |
| Extra levels or a free slider | More granularity | Contradicts the specified seven-state row, and F-to-B is the only meaningful range | None. |
| Hiding S or A | Completeness | It silences the most valuable items | Not offered. |

**Complexity:** LOW to MEDIUM. The logic is a small pure function plus one widget row. The care is in wiring (see dependencies).

**Dependencies on existing code**
- CONFIG tab style model: the pending-edit styles, the unapplied state, the Revert and Discard popup, and the `S`..`F` required styles in `styles.json`.
- Economy section regeneration: hidden tier styles produce `Hide` blocks, because Apply already derives each block's header from its style's visibility.
- Styled user rules: they follow the pending styles, so a level also hides user rules that use those tiers. This is correct behaviour but should be mentioned in the release notes.
- Style table "Visibility" switches: the same state, which is what keeps the two views consistent.

### 2. League selector and fallback prices (Economy tab)

**How other tools present it** (training knowledge, LOW to MEDIUM; the search returned nothing usable for Exiled Exchange 2)
- Price checkers (Exiled Exchange 2, Awakened PoE Trade) keep a league drop-down fed from GGG's league list and cache the last choice. A league that disappears at the end of an event forces a re-pick.
- poe.ninja uses a league switch at the top of the page and lists Standard, Hardcore and the current temporary leagues. SSF variants have no market and so no prices. SSF players use the matching trade league's prices as a guide.
- FilterBlade's home page showed only a "Game mode: Softcore" setting and a paid economy auto-sync. It has no per-league picker that I could verify.
- League transitions are the pain point for everyone. At league start data is sparse and volatile. At the end of the league the old league vanishes or Standard becomes the default. The project's own note records `indexed` flipping within hours.

**Table stakes**

| Feature | Why expected | Complexity | Notes |
|---------|--------------|------------|-------|
| Drop-down fed live from poe.ninja's league list, plus a default selection | Specified. A hardcoded league is the bug this removes. | MEDIUM | Network call on a worker thread, as Sync already does. Cache the last list so the UI works offline. |
| The selection persisted per user and used by Sync | Without persistence every launch re-picks | LOW | Goes in `ff-settings.json` (the per-user settings file). Today's pin is `sync::OVERLAY_LEAGUE`. |
| Selected league named in the Sync outcome and in "League" on the tab | Already the visible guard against a wrong pin | LOW | Keep. |
| Selected league missing from the new list (event ended) shows a clear message and a fallback pick, never a silent switch | The league-end transition is the main failure mode | MEDIUM | Reuse the existing "pin no longer matches, list available ones" failure path. |
| Standard, Hardcore and the temporary leagues all selectable; SSF treated honestly | Players of every mode will install this | LOW | Show what poe.ninja offers. If SSF variants are absent from the feed, say prices come from the trade league. Do not invent SSF data. |
| A price source choice: Live (Sync) or Fallback, exclusive | Specified. At league start live data is thin. | MEDIUM | Choosing Fallback disables the Sync button and the League drop-down becomes irrelevant (grey it with a tooltip). The existing `price_source` and Fallback readout on the tab already show the state. |
| Fallback is identified as recommended for league start and non-trade players | These users are the audience for the option | LOW | A tooltip, not a paragraph. |

**Differentiators**

| Feature | Value proposition | Complexity | Notes |
|---------|-------------------|------------|-------|
| Warning when the synced `prices.json` league differs from the selected league | Closes a silent failure the pin comment already worries about | LOW | The data already carries the league name. |
| Fallback pair works offline and with no setup | A new player on day one has a working tiered filter at once | none | Already built. |
| "Last sync" age shown next to the selection | Stale prices are visible | none | Already shown. |

**Anti-features**

| Anti-feature | Why requested | Why problematic | Alternative |
|--------------|---------------|-----------------|-------------|
| Automatic league detection with no override | Convenient | `indexed` is unreliable, which is the original reason for the pin | Default to the best guess, always show the choice, name the league in the result. |
| Background auto-sync on a timer or at startup | Fresh prices | Surprise network traffic. It also changes the price data between Applies, and the filter only changes on Apply. | Keep Sync a deliberate button. |
| Blending fallback and live prices | Fills sparse early data | The fallback pair is calibrated together (exalt-denominated). Mixing breaks the cutoffs. | Exclusive choice only. |
| Our own SSF price model | SSF players | There is no source for it | Use fallback or the parent league and say so. |

**Complexity:** MEDIUM. The UI is simple. The work is the poe.ninja league-list parse (the feed shape is a risk, so spike it first), the settings, and the removal of the constant.

**Dependencies on existing code**
- `sync::OVERLAY_LEAGUE` and the existing league resolution, including the CLI `--league` override, which stays.
- `ff-settings.json`, which also has to move to the per-user config location (feature 4).
- The `price_source` Fallback logic. Note the deferred W3 tech debt: tier edits have no effect while fallback prices are in use. Choosing Fallback explicitly makes that visible to many more users, so decide whether to fix it or put a tooltip on it.
- The Economy tab tier editor, which is unaffected.

### 3. Check for updates (About tab)

**Typical behaviour of small open-source desktop tools** (training knowledge, MEDIUM; the search did not cover it)
- A button on the About page: "Check for updates". Results are "You are up to date (v0.7.0)", "Version X is available" or an error line.
- GitHub Releases API: `GET https://api.github.com/repos/<owner>/<repo>/releases/latest`. This returns the latest non-draft, non-prerelease release. It needs a `User-Agent` header and is rate limited to 60 requests per hour per IP without a token. A 403 or 429 means "try again later".
- Version compare is semver against the running package version.
- The asset is found by file-name pattern. The tool downloads it to a temp file and then runs it.
- Most tools of this size do not self-patch the exe. They download the installer and hand over to it, because a running exe is locked on Windows.
- Many tools also check once on start-up. A check on demand is the less intrusive default and has fewer failure modes.

**Table stakes**

| Feature | Why expected | Complexity | Notes |
|---------|--------------|------------|-------|
| On-demand check button, run on a worker thread | Specified, and the UI must never freeze | LOW | Same worker-thread pattern as Sync. |
| Clear results: up to date / newer version X / failed with a reason | The user needs to know which | LOW | Use the one status line. Distinguish offline, rate-limited, no release found and no matching asset. |
| Semver comparison against `CARGO_PKG_VERSION`; ignore drafts and prereleases | A tag like `v0.7.0-rc1` must not be offered as an update | LOW | The `/latest` endpoint already skips both. |
| Download the installer asset, then ask before running it (a popup) | Specified. No silent install. | MEDIUM | Skinned popup, in the same style as the eight existing ones. |
| Close `ff` before running the installer | The exe is locked, and the overlay has a tray and a hotkey, so closing is not trivial | MEDIUM | Reuse the Quit path including the Unapplied changes popup (Apply and quit / Quit without applying / Cancel). Start the installer detached, then exit. |
| Failure never destroys the install | Download to a temp file, check size, delete on failure | LOW | Verify against the asset size. Also verify SHA-256 if the release publishes a `SHA256SUMS` file. |
| HTTPS only; the tool sends nothing but the request | Trust, since this is a tool that runs near a game | LOW | |

**Differentiators**

| Feature | Value proposition | Complexity | Notes |
|---------|-------------------|------------|-------|
| Release notes of the new version shown in the popup | The CHANGELOG section already becomes the release body, so users see what they get | LOW | `body` from the same JSON response. |
| Link to the release page as the fallback when the download fails | The user can always update by hand | LOW | |

**Anti-features**

| Anti-feature | Why requested | Why problematic | Alternative |
|--------------|---------------|-----------------|-------------|
| Silent auto-update or an in-place exe swap | Seamless | The installer is unsigned. Replacing a running exe is fragile, and silent installs of unsigned code erode trust. | Ask first, then run the normal installer. |
| Check on every start-up with a notification | Awareness | It is network traffic the user did not ask for, and it can interrupt the game. It also risks a rate-limit error on shared IPs. | On demand only for v0.7. Later: an opt-in start-up check that shows a badge, not a popup. |
| Update channels (beta / nightly) | Testers | Needs prerelease handling and extra settings | Not now. |
| A GitHub token in the app | Avoids rate limits | A secret in the binary | Unnecessary at on-demand frequency. |

**Complexity:** MEDIUM. The HTTP and JSON parts are small. The close-then-run handover and its failure messages carry the risk.

**Dependencies on existing code**
- The HTTP client already used by `sync` (reuse it; no second TLS stack).
- Tray Quit and the single-instance guard (`src/overlay/instance.rs`): the installer must start after the old process has released the instance name. The installer should also wait for it (Inno `AppMutex`).
- The release pipeline (feature 5): the asset name must be stable (for example `FerrisFilter-<ver>-setup.exe`) and the tag must match the package version.
- The About tab layout, which must stay free of walls of text.

### 4. Windows installer, per-user config and game folder picker

**Typical expectations for a gamer tool**
- Download a `-setup.exe`, double-click, accept an unsigned-publisher warning, find the tool in the Start menu, and it works with no admin prompt.
- Uninstall appears in Settings → Apps.
- Settings survive an update and an uninstall.
- If the game folder is not found, the tool says so and offers to pick it, rather than failing.

**Installer**

| Feature | Why expected | Complexity | Notes |
|---------|--------------|------------|-------|
| Per-user install, no admin (Inno Setup `PrivilegesRequired=lowest`, default under `%LOCALAPPDATA%\Programs`) | The tool needs no machine-wide files, and a UAC prompt on an unsigned installer deters people. Confirmed in the Inno docs: `lowest` never elevates and uses the HKCU uninstall key. | LOW | Recommended: Inno Setup. Alternatives: cargo-wix (MSI), NSIS. Whether the Windows CI runner has Inno preinstalled needs checking (LOW confidence), else it is a one-line install step. |
| Start menu shortcut to `ff.exe` | Specified | LOW | Optional desktop shortcut, off by default. |
| Uninstaller | Table stakes for any installer | LOW | It must not delete `%APPDATA%\FerrisFilter` (the user's styles and tiers). Leaving it is the safer default. |
| "Launch FerrisFilter" checkbox on the last page | Standard | LOW | Needed after an update so the user is back in the tool. |
| Close a running instance before replacing files | The updater depends on this | MEDIUM | Inno `CloseApplications` and `AppMutex`. The overlay has a tray and may be hidden. |
| A portable zip alongside the installer | Specified in the release pipeline, and some players avoid installers | LOW | The zip must still find defaults (built into the exe) and config in `%APPDATA%`, or `FERRISFILTER_HOME`. |
| SmartScreen note in the README and release notes | The installer is unsigned (accepted as out of scope) | LOW | "More info → Run anyway" with one sentence. No code signing. |

**Per-user config**

| Feature | Why expected | Complexity | Notes |
|---------|--------------|------------|-------|
| Config (styles, tiers, `ff-settings.json`) in `%APPDATA%\FerrisFilter`; price cache in `%LOCALAPPDATA%` | Standard Windows layout, and it survives updates | MEDIUM | The `dirs`-style crates cover this and the Linux/macOS equivalents. |
| Defaults compiled into the exe (`include_str!` of `styles-default.json` and `tiers-default.json`), the live files created on first Apply | No loose files to lose or to mis-install | MEDIUM | Today `styles-default.json` is also the app-root marker. That marker walk is replaced. The fallback pair must be embedded too (`prices-fallback.json`, `tiers-fallback.json`). |
| `FERRISFILTER_HOME` still overrides everything | Specified. Keeps tests and the developer workflow unchanged. | LOW | The CLAUDE.md "Development warning" about writing live files to the repo goes away for packaged runs, but stays for repo runs. |
| No migration step in the installer | v0.6 was never public, so there is no user base | none | The author can copy the old live files by hand. Do not build an importer. |

**Game folder picker (Config tab)**

| Feature | Why expected | Complexity | Notes |
|---------|--------------|------------|-------|
| Default folder found through the OS "Documents" known folder + `My Games\Path of Exile 2` | OneDrive redirection and localized folder names break any hardcoded path. The author's own machine is the OneDrive/`Documentos` case. | LOW to MEDIUM | `SHGetKnownFolderPath(FOLDERID_Documents)` or the `dirs` crate. Replaces the `%USERPROFILE%\OneDrive\Documentos` default. |
| Native folder dialog (Browse) and a visible current path | Specified | MEDIUM | A new dependency (for example `rfd`). Run it off the UI thread and handle the overlay being always-on-top above the dialog. |
| Validation: exists and is a directory. Warn (not block) if there are no `.filter` files | The folder may legitimately be empty | LOW | Keep the existing "folder not found" status and the Reload recovery. |
| Persisted in `ff-settings.json`, applied at once like the filter choice | Matches the existing Filter-section behaviour | LOW | Switching folders with unapplied Rules edits should use the existing **Switch filter?** popup. |
| Feeds everything that used the game folder: filter list, sound-file listing, the Apply target and the in-game reload | One source of truth | MEDIUM | `install` and the CLI `apply` default also use it. |

**First run** (a gamer on a clean machine)

| Situation | Expected behaviour | Complexity | Notes |
|-----------|--------------------|------------|-------|
| Game folder not found | One status-line message with the cause. The Config tab shows the picker. No crash. | LOW | Reuses the "folder not found" state. |
| Folder found, no filter yet | Offer to create `dynamic.filter` or a named empty filter. The first Apply creates it, with no backup (there is nothing to back up). | LOW | The Filter section already creates new empty filters. Say clearly that the user then has to select it in game under Options → Game → Item Filter. |
| Game not running | The window opens on the primary monitor. The reload step says "Wrote <path>". | none | Already handled. |
| Game in exclusive fullscreen | A blank window | LOW | A short README line. The overlay cannot detect it. |
| Elevated game | The reload keys are silently dropped | none | Already documented. Add it to the README. |
| No network at first sync | The failure message, plus the choice of fallback prices | LOW | The fallback option in feature 2 covers this case. |
| First-time user has no economy section | Rules and Economy tabs behave as today (plain Apply never adds one) | none | **Add economy section** stays an explicit action. Consider a one-line hint on the Economy tab. |

**Anti-features**

| Anti-feature | Why requested | Why problematic | Alternative |
|--------------|---------------|-----------------|-------------|
| A first-run wizard | Hand-holding | Contradicts the "not texty" overlay and adds a flow to test and maintain | Status-line messages and the Config tab. |
| Auto-installing a filter into the game or choosing the in-game filter for the user | Fewer steps | Touches files the user did not name. The game selects the filter itself. | Tell the user the one in-game step. |
| Machine-wide (admin) install | "Proper" installers | A UAC prompt on an unsigned installer, and nothing needs it | Per-user. |
| Registry-stored settings | Windows convention | Hard to inspect or reset by hand | JSON in `%APPDATA%`. |
| Bundling the game's sound files or scanning outside the chosen folder | Convenience | Scope creep | Current read-only folder listing. |
| Code signing in this milestone | Removes SmartScreen | Declared out of scope (cost) | Document the warning. |

**Complexity:** MEDIUM to HIGH overall. The path refactor is the widest-reaching change in the milestone.

**Dependencies on existing code**
- The app-root resolution (`FERRISFILTER_HOME`, walk up to `styles-default.json`). This is the core change. `generate` / `install` / `sync` are working-directory relative today (v0.5 tech debt W1), and the packaged app needs one rule.
- `styles-default.json`, `tiers-default.json`, `prices-fallback.json`, `tiers-fallback.json`: embedded. The tests and `artifact_guard` currently read the files from disk, so keep both paths.
- `ff-settings.json` and its fail-closed parsing rules, which move location but not behaviour.
- The default game-folder constants in `install` and `apply`.
- The `--screenshot` safety checks, which depend on `FERRISFILTER_HOME` and `USERPROFILE` being under the temp folder. Check that the new `%APPDATA%` and `%LOCALAPPDATA%` lookups cannot escape the temp home.
- The built-in style rework (`styles-default.json` with no custom sound files). If it changes, `seed-universal.filter` must be re-baselined, and `artifact_guard` is the check.

### 5. GitHub release with changelog notes

**Typical behaviour**
- A tag `vX.Y.Z` triggers a workflow that builds on a Windows runner, tests, produces artifacts and creates the release.
- The release body comes from the matching `CHANGELOG.md` section (Keep a Changelog format), not from auto-generated commit lists.

**Table stakes**

| Feature | Why expected | Complexity | Notes |
|---------|--------------|------------|-------|
| Workflow on `v*` tags: build (release profile), run `cargo test`, build installer, zip, publish | Specified | MEDIUM | `windows-latest`. Upload the installer, the zip and a `SHA256SUMS` file. |
| Release notes extracted from the `## [X.Y.Z]` section of `CHANGELOG.md` | Specified. The author writes the changelog by hand. | LOW | A small script, or an action, plus `gh release create --notes-file`. Fail the workflow if the section is missing or empty. |
| Check that the tag equals `Cargo.toml` `version` | The About tab shows the package version, and the updater compares against it | LOW | `/gsd-complete-milestone` already checks this. Repeat it in CI. |
| Stable asset names | The updater finds the asset by pattern | LOW | |
| A pinned Rust toolchain in CI | `rust-toolchain.toml` is a floating nightly, which the repo itself notes can break `poe_data_tools`. A tag build that fails to compile, or compiles differently, on release day is the worst failure. | LOW | Pin a dated nightly for CI, or confirm the root build does not need nightly. Needs a quick check of what actually requires nightly (`ff-art` is a non-default member, so the root crate may build on stable). |
| A dry run before the first real tag | The first public release should not be the first workflow run | LOW | `workflow_dispatch` or a prerelease tag that builds but publishes as a draft. |

**Differentiators**

| Feature | Value | Complexity | Notes |
|---------|-------|------------|-------|
| README install section with SmartScreen guidance and the windowed-mode requirement | The first user-facing text | LOW | The Release Review phase already hands the README to the author. Agents should not write it. |

**Anti-features**

| Anti-feature | Why problematic | Alternative |
|--------------|-----------------|-------------|
| Auto-generated release notes from commits | The project defines the CHANGELOG as the human-written record | Use the CHANGELOG section. |
| Publishing on every push or on main | Unreviewed releases, and the updater would offer them | Tag only. |
| Publishing the repo's `reference-old-filters/` or live data files in the zip | Git-ignored for a reason | Ship exe plus docs only. |

**Complexity:** MEDIUM. Mostly CI plumbing. The toolchain pin and the installer step are the risks.

**Dependencies:** the installer (4), the version in `Cargo.toml` (set in the Release Review phase), and the CHANGELOG draft (the same phase). The updater (3) depends on this feature's asset naming.

## Feature Dependencies

```
Per-user config location (embedded defaults, app-root refactor)
    ├──required by──> Game folder picker (stored in ff-settings.json)
    ├──required by──> League selector (selection persisted in ff-settings.json, price cache in LOCALAPPDATA)
    └──required by──> Installer (installer must not ship loose default files; uninstall keeps APPDATA)

Strictness Selector ──requires──> existing Config style edit model (pending styles, one Apply)
Built-in style rework ──affects──> Strictness derivation (the S-F set must stay required and ordered)
                      ──affects──> seed-universal.filter fixture (re-baseline once)

League selector ──conflicts (exclusive)──> Fallback price source (fallback disables Sync and the league picker)

Release pipeline ──requires──> Installer (artifact) + Cargo.toml version + CHANGELOG section
Update check ──requires──> Release pipeline asset naming + Quit path + installer AppMutex/close handling
```

### Dependency Notes

- **Per-user config is the foundation.** Three other features store state in it. Do it first, and do the style rework in the same phase window, since both touch `styles-default.json` and the fixture.
- **Strictness is independent of the path work.** It only needs the Config tab's existing style model, so it can go in any phase after the style rework settles.
- **The updater is last.** It cannot be tested against a real release until the pipeline has published one. Plan a prerelease or a throwaway tag to exercise it.
- **League and fallback share the Economy tab.** Build them together. They are one exclusive control.

## MVP Definition

### Launch with (v0.7)

- [ ] Per-user config location with embedded defaults, `FERRISFILTER_HOME` override kept. A public tool cannot require a repo checkout.
- [ ] Game folder default via the OS Documents folder, plus the Browse picker, with clear not-found messages.
- [ ] Per-user Inno Setup installer with Start menu entry and uninstaller, and the portable zip.
- [ ] League drop-down from poe.ninja plus the Live/Fallback exclusive choice.
- [ ] Strictness Selector with the derived Custom state.
- [ ] Release workflow with CHANGELOG notes, a version check, a pinned toolchain and a dry run.
- [ ] On-demand update check that downloads the installer and asks before running it.

### Add after validation (v0.7.x / v0.8)

- [ ] Optional start-up update check with a non-blocking badge. Trigger: users ask, and rate limits turn out not to matter.
- [ ] Strictness affecting non-tier rules. Trigger: feedback that "Div Farmer" still shows too much gear.
- [ ] Release asset checksum verification in the updater, if not done in the first cut.
- [ ] Surface `price_source` in the status line (carried-over backlog), and the W3 fallback tier-edit fix.

### Future consideration (v1+)

- [ ] Code signing. Cost and certificate trust.
- [ ] Update channels, winget or Scoop packaging.
- [ ] Linux and macOS overlay builds (out of scope here).

## Feature Prioritization Matrix

| Feature | User value | Implementation cost | Priority |
|---------|------------|---------------------|----------|
| Per-user config and embedded defaults | HIGH (nothing works installed without it) | MEDIUM-HIGH | P1 |
| Installer, Start menu | HIGH | MEDIUM | P1 |
| Release pipeline with notes | HIGH | MEDIUM | P1 |
| Game folder default and picker | HIGH | MEDIUM | P1 |
| League selector | HIGH (the pinned league goes stale on day one of the next league) | MEDIUM | P1 |
| Fallback price option | HIGH at league start | LOW-MEDIUM | P1 |
| Strictness Selector | HIGH (headline feature for new players) | LOW-MEDIUM | P1 |
| Update check | MEDIUM | MEDIUM | P2 |
| Release notes shown in the update popup | LOW-MEDIUM | LOW | P3 |
| League-mismatch warning on stale prices | MEDIUM | LOW | P2 |
| SHA-256 verification of downloads | MEDIUM | LOW | P2 |

## Competitor Feature Analysis

| Feature | NeverSink / FilterBlade | Exiled Exchange 2 / Awakened PoE Trade | Our approach |
|---------|-------------------------|----------------------------------------|--------------|
| Strictness | Seven static levels, whole filter, separate file per level, per-section overrides in FilterBlade | n/a | Seven wealth-named levels, tier-only, one live derived state on top of editable per-style switches |
| League choice | FilterBlade has a game-mode setting; economy sync is a paid tier. NeverSink ships per-patch lists | League drop-down, last pick remembered (unverified) | Drop-down from poe.ninja, a named league in every result, an explicit stale warning |
| Updates | Manual download | Built-in auto-updater (Electron) | On-demand check, download then ask, installer handover |
| Install | Web download, no installer | Installer or portable | Per-user unsigned installer plus portable zip |
| Config location | Files in the game folder | App data folder | `%APPDATA%\FerrisFilter`, `FERRISFILTER_HOME` override |

## Sources

- NeverSink filter and seven strictness levels, beginner advice and the Soft level history: search results for NeverSink forum threads and the filter's repository mirror (https://gitee.com/agg/NeverSink-Filter, https://maxroll.gg/poe2/news/filterblade-launch-for-path-of-exile-2). MEDIUM. The level contents were not listed in any source.
- FilterBlade home page (https://www.filterblade.xyz/), fetched 2026-10-17: shows a Strictness setting, "Get all strictnesses as ZIP", a Softcore game mode, and a premium auto-sync. It had no per-level text or league picker. MEDIUM for those facts.
- Inno Setup `PrivilegesRequired` documentation (https://jrsoftware.org/ishelp/topic_setup_privilegesrequired.htm). HIGH for the per-user mode.
- Updater behaviour, the GitHub Releases API (`/releases/latest`, the 60 requests per hour limit, `User-Agent`) and the league UIs of Exiled Exchange 2 and Awakened PoE Trade: training knowledge only; web searches returned nothing relevant. LOW to MEDIUM. Verify against https://docs.github.com/en/rest/releases/releases before implementing.
- The project's own records: `.planning/PROJECT.md`, `CLAUDE.md`. HIGH for everything about the existing code.

---
*Feature research for: the v0.7 public release of FerrisFilter*
*Researched: 2026-10-17*
