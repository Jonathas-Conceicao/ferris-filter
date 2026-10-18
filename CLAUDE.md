# CLAUDE.md — FerrisFilter

This repo is **FerrisFilter**: a Rust CLI that compiles a human-editable config into a
Path of Exile 2 loot filter (`generate`) and fetches item prices from poe.ninja (`sync`).

- **Project plan & workflow:** managed with GSD in `.planning/` — see `.planning/PROJECT.md`
  (context), `.planning/ROADMAP.md` (4 phases), `.planning/REQUIREMENTS.md`, and
  `.planning/STATE.md`. Git uses a **milestone branching** workflow (see PROJECT.md → Git Workflow).
- **Milestone close-out:** milestones are atomic, and each one ends with a **Release Review**
  phase (code cleanup, README screenshots, `Cargo.toml` version, changelog draft, then the
  author's review of `README.md` and `CHANGELOG.md`). See PROJECT.md → Milestone Close-out.
  A bug introduced and fixed inside a milestone is never mentioned outside `.planning/`
  (changelog, README, this file, code comments, commit messages on `main`).
- **Changelog / README:** `CHANGELOG.md` and `README.md` are read by people and mostly written by
  the author. Agents don't add changelog entries during a milestone (the Release Review drafts
  them once); work landed between milestones may add a short `[Unreleased]` entry.
- **Domain spec (below):** the filter grammar, the Dracula style-tier system, and the design
  philosophy. `config.json` + `tests/fixtures/seed-styles.json` are the executable spec the `generate` command compiles. The
  **byte-for-byte acceptance baseline** is now the committed fixture
  `tests/fixtures/seed-universal.filter` (generated from `config.json` + `tests/fixtures/seed-styles.json` (the frozen pre-v0.7 default styles) + `tiers-default.json` + the seed
  `tests/fixtures/seed-prices.json`); the `artifact_guard` test enforces it. The repo-root
  `universal.filter` / `prices.json` are **generated, git-ignored, and not committed**.
  Everything from "Repository layout" down is that reference — keep it accurate.

## Repository layout

| Path | Purpose |
|------|---------|
| `config.json` | **Source of truth.** Declarative policy model the filter is compiled from. Top-level sections: `build` (weapon/armour classes + defence-type taper), `weaponClasses`/`armourClasses`/`flaskCharmClasses` (complete class lists), `classGroups` (documentation aliases), the ordered `rules` array (the policy — first-match-wins), and `alwaysShow` (currency, recycle). Edit `rules` (and `styles.json` for how a style looks, on the CONFIG tab's style table and editor), then regenerate. A `styles` section here is rejected at load unless every entry in it equals the same-named `styles.json` entry (then it is dropped). |
| `styles-default.json` + `tiers-default.json` (repo root) | **Committed defaults** for the live `styles.json` / `tiers.json`. Never written by the app. `styles-default.json` ships the nine built-ins only (no custom styles); it is the app default and the source of the overlay's **Reset to defaults** (the embedded copy). It is no longer the `generate` spec (the tests and `artifact_guard` use `tests/fixtures/seed-styles.json`; `tiers-default.json` is still read by them). Consequence: `ferrisfilter generate` on a checkout with no live `styles.json` fails with the unknown-style error for `Recycle` / `kite` / `exceptional`, which `config.json` still references (reworking `generate` / `config.json` is deferred); with a live `styles.json` holding them it works as before. Built into the exe at build time (`include_str!`), so an installed build needs no copy beside it; they are no longer a folder marker and a copy in the FerrisFilter folder is ignored. |
| `styles.json` | **Per-user live file of named styles** (in the FerrisFilter folder, see the `ferrisfilter apply` row), **git-ignored and not committed**. When it is absent every reader uses the built-in copy of `styles-default.json` instead (same for `tiers.json` → `tiers-default.json`); a broken live file is named by its full path, a broken built-in one as `<file> (built in)`. Delete it, or use the Config tab's **Reset to defaults**, to get the defaults back. A map of style name to style object (text/border/background colours, font, minimap, beam, sound, `action` Show/Hide, `disableDropSound`; `minimap`, `beam` and `sound` are `null` = off (no line), an object = the value, or the string `"None"` = the filter's cancel value written as `MinimapIcon -1` / `PlayEffect None` / `PlayAlertSound None`; a built-in alert may add `"positional": true` for `PlayAlertSoundPositional` (refused with `custom`); `"enableDropSound": true` writes `EnableDropSound` (refused together with `disableDropSound`); the new fields are written only when set, so existing files load and re-serialise byte-identically), each carrying an `"id"` written first: `builtin:<name>` for the nine built-ins, a random 6-8 character lowercase alphanumeric id for custom styles (+ ADD STYLE). A file without ids still loads (ids are assigned at load, deterministically from a hash of the name so they are stable across loads, and persisted by the next overlay Apply without showing as an edit); duplicate ids, a wrong built-in id or a custom id starting with `builtin:` are load errors. The generator and `config.json` ignore ids and keep using names. File order is the overlay picker order. `S`, `A`, `B`, `C`, `D`, `E`, `F`, `unchanged` and `hidden` are required; every other style is optional, and adding one needs no code change. Read by `generate` (beside the inputs) and by `apply`/the overlay (from the FerrisFilter folder); `--styles <path>` on `generate` and `apply` overrides it (an explicit path, no fallback; `--tiers` likewise). A config that still has an inline `styles` section loads only if every inline entry equals the same-named entry here (an untouched pre-v0.5 seed): the section is then dropped in memory (the file itself is not rewritten). Otherwise it is rejected with a message naming the file and the differing styles; move those here and delete the section. The overlay's Apply (either tab) writes this file when the CONFIG tab has edits (one style per line, file order kept, which includes a custom-style reorder; the first such Apply creates it); `tiers.json` is written the same way (entry order kept) — and the `-default` and `-fallback` pairs are never touched. |
| `universal.filter` (repo root) | **Generated** by `ferrisfilter generate` and **git-ignored** — regenerate on demand, do not commit or hand-edit. Each policy block is headed `Show # <rule name>` (from the `name` field in `config.json`). |
| `prices.json` (FerrisFilter folder; a repo-root copy is the author's when `FERRISFILTER_HOME` is the repo) | **Generated** by `ferrisfilter sync` from poe.ninja and **git-ignored** — your live economy data, regenerated per sync, never committed. `ff-cli sync` writes it into the FerrisFilter folder (no longer the working directory). When it is absent the built-in `prices-fallback.json` / `tiers-fallback.json` pair is used. `sync --league <name\|slug>` targets a specific poe.ninja league (useful during a league transition, when the automatic pick resolves to the outgoing league). |
| `ff-settings.json` (FerrisFilter folder) | **Per-user overlay settings, git-ignored.** Written by the overlay's CONFIG tab; holds the selected filter name (a bare `*.filter` file name, validated before it is ever joined onto the game folder). With no `filter` key the overlay opens the folder's `dynamic.filter` when present, else its first `.filter`, else `dynamic.filter` as a new file the first Apply creates; opening the overlay does not save that pick (Reload keeps it in memory), nor does saving another preference (the reload toggle, a section fold); picking it in the Filter list, or picking a folder, saves it. A stored name that is invalid, or a file that cannot be used, means `dynamic.filter` with a warning saying so (a warning about another key leaves the pick as above). It also holds `game_folder` (the picked game folder, an absolute path written only by the Config tab's Browse; absent means the default, an empty or relative value is ignored with a warning, and `FERRISFILTER_GAME_DIR` overrides it); also `league` (the league picked on the Economy tab; absent means the computed default) and `price_source` (`"live"` / `"fallback"`; absent or unknown reads Live, a wrong type reads the default with a warning), both saved at once and read only by the overlay (the source switch marks something unapplied only as the Economy tab describes); also `collapsed_sections` (per filter name, section name to collapsed true/false; the Economy section defaults to collapsed, user sections to expanded; saved the moment a header is toggled and never written into the filter, so it never marks the filter unapplied). Unknown keys are preserved on save. A section fold or unfold never rewrites a file that could not be used (it stays in memory for the session). Never committed. |
| `dynamic.filter` | **The store.** Since Phase 18 the filter file itself is the store — there is no `dynamic.json` (a leftover repo-root copy from v0.4 is obsolete and safe to delete; `/dynamic.json` stays git-ignored only so it is never committed). It lives in the game folder and is edited **in place**; general rules in it are the user's and are never touched. The file is a list of **sections**: a section runs from its banner to the next banner or the end of the file, and blocks above the first banner belong to no section. A **banner** is three lines, 80 `#`, `# ff:section <name>` (a user divider) or `# ff:economy-begin Economy (generated by FerrisFilter, do not edit)` (the economy), then 80 `#` again. The **economy section** is always the last one, appended at the end of the file; it is begin-only (no end marker), Every generated block carries `# ff:name <T> tier <Group>` (e.g. `S tier Currency`, `A tier Uniques`) and tier blocks also `# ff:style <id> <name>` (price-tiered unique blocks, with their bronze background, have the name only); each block is headed `Show` or `Hide` from its tier style's visibility; `generate` output has neither. Apply regenerates the blocks under its banner (from `prices.json`, `tiers.json` and `styles.json`) and leaves everything above untouched, and a lone `# ff:economy-begin` line that is not a full banner is reported as damaged and stops apply. The old one-line `# ff:economy-begin <text>` / `# ff:economy-end <text>` pair is **no longer recognised** (convert by hand: delete the pair or replace it with the banner). A foreign `#=====` line stays a plain comment. Every FerrisFilter metadata line (`# ff:name`, `# ff:style`, `# ff:group <id> <k>/<n>` (one rule written as several blocks, see the Simple Defences note), `# ff:section`, `# ff:economy-begin`) is written with a space after `#`; the pre-v0.5 spelling `#ff:` is still read and every Apply rewrites a remaining `#ff:` line to `# ff:`, leaving all other bytes alone. Disabled blocks are written the same way: disabling puts `# ` in front of every line (`# Show`, `# \tRarity Rare`; an empty line becomes a bare `#`), and enabling removes the `#` plus one space after it when present, otherwise just the `#` — so the older `#Show` form (earlier FerrisFilter versions, foreign filters) still enables, and disable→enable is byte-exact. Old `#Show` blocks are **not** normalized on Apply (they cannot be told apart from a foreign filter's own disabled blocks); a block takes the `# ` form the next time it is disabled. The first write keeps `<name>.filter.ffbak` next to the file — created once, never overwritten (delete it to take a fresh one); `*.ffbak` is git-ignored. Apply refuses if the file changed on disk since it was loaded (the overlay then offers **RELOAD** and **OVERWRITE**). A v0.4 file is converted once, on its first apply: the economy banner added, `Show # <name>` names kept as `# ff:name`, the stale `# GENERATED FROM dynamic.json` header line left as it is. **Git-ignored** at the repo root. PoE2 does not hot-reload — after `ff-cli apply`, reload via Options → Game → Item Filter → Refresh; the overlay's Apply can send `/reloaditemfilter` instead (see the overlay notes). |
| `tests/fixtures/seed-prices.json`, `tests/fixtures/seed-styles.json`, `tests/fixtures/seed-universal.filter` | **Committed test fixtures** — the frozen seed prices, the frozen pre-v0.7 default styles (the `generate` spec) + the generated filter. These are the test oracle (`include_str!` unit tests + `generate_snapshot`/`artifact_guard`). Regenerate the `.filter` fixture only after a deliberate generator change. |
| `prices-fallback.json` + `tiers-fallback.json` (repo root) | **Committed early-league fallback pair**, always used *together* (their values are calibrated to each other). `generate` loads them instead of `prices.json`/`tiers.json` when `prices.json` is absent/empty (with a warning) or when `--early-game` is passed. Tier membership comes from a poe2filter.com export; cutoffs are **exalt-denominated** (`unit: "exalt"`) so Divine isn't pinned to the A floor. Every base — orbs included — is priced in **exalt at its tier floor** (Divine 20→S, Chaos 2.5→A, Exalt 1→B). No stored rates: `pricer::derive_rates` reads the three anchor orbs (items[0..2]) and expresses them relative to the Chaos Orb (`exalt_in_chaos = value(Exalt)/value(Chaos)` etc.), so quoting the orbs in exalt is enough. The same code on live chaos-quoted data (Chaos = 1c) reproduces the old numbers. The three anchors must share one unit. `ff-cli apply` ignores the overlay's `price_source` (Auto: live prices when usable; `generate --early-game` still forces this pair). |
| other `*.filter` (repo root) | Active, version-controlled filters we author/maintain. |
| `reference-old-filters/` | Older/stale filters copied from the game's config folder. **Git-ignored**, kept only as reference. Do not edit these as if they were live; treat them as read-only history. |
| `ferrisfilter apply` | `ff-cli apply [--filter <path>] [--add-economy \| --remove-economy] [--tiers <path>] [--styles <path>]`. Refreshes the economy section of the filter at `--filter` (default: the filter selected in the overlay, saved as `filter` in `ff-settings.json`, else `dynamic.filter`, in the game folder (`FERRISFILTER_GAME_DIR`, else the folder picked on the overlay's Config tab and saved as `game_folder` in `ff-settings.json`, else the OS Documents folder + `My Games\Path of Exile 2`, which follows OneDrive redirection and localized names)). For that default (and `install` without `--dest`) an unresolvable FerrisFilter folder is an error unless `FERRISFILTER_GAME_DIR` is set, and an unusable `ff-settings.json` key is reported as a `warning:` on stderr. Plain apply only refreshes an existing section and **never adds one**; `--add-economy` appends the economy banner and blocks at the end of the file, `--remove-economy` deletes the banner and its blocks (a following user section banner is kept). The old reset, base, store and dest flags no longer exist. **FerrisFilter folder:** apply, sync and the overlay find `styles.json` / `tiers.json` / `ff-settings.json` / `prices.json` in `FERRISFILTER_HOME` if set (it must be an existing directory, else an error), otherwise `%APPDATA%\FerrisFilter` (`~/.config/ferrisfilter` on Linux). There is no walk-up from the exe and no working-directory fallback. The folder is created by the first Apply that writes styles/tiers, the first settings save (a section fold counts) or the first Sync; a launch writes nothing. `styles.json` / `tiers.json` absent means the built-in copies of `styles-default.json` / `tiers-default.json`; `prices.json` absent means the built-in fallback pair. `config.json` (the overlay's class catalog) and `mapping.json` (sync) are always the built-in copies for the overlay, apply and sync (`sync metadata` still rewrites the repo `mapping.json`; rebuild to embed it). A relative `--tiers`/`--styles` given to apply resolves against the folder, and a bare name such as `tiers-fallback.json` with no such file there uses the built-in copy (`--filter` stays working-directory relative). `generate` and `install` stay working-directory relative and unchanged. Tests pin `FERRISFILTER_HOME`/`FERRISFILTER_GAME_DIR` on every spawn (enforced by `tests/isolation_guard.rs`) and lib tests never resolve the real folder; only `src/locations.rs` asks the OS for a folder. |
| `ferrisfilter install` | Subcommand that copies `universal.filter` into the in-game config folder (replaces the old `install.bat`). Defaults to the game folder (`FERRISFILTER_GAME_DIR`, else the folder picked on the overlay's Config tab and saved as `game_folder` in `ff-settings.json`, else the OS Documents folder + `My Games\Path of Exile 2`, which follows OneDrive redirection and localized names); pass `--dest <dir>` to override and `--input <file>` for a non-default filter. Copy-only — run `generate` first. |
| `ff` (binary, v0.4) | **The GUI overlay** — `cargo build`'s default binary (`default-run = "ff"`). Opens the game-styled, always-on-top window over the game: a fixed 660pt borderless window docked flush to the left edge of the work area at its full height, on the monitor the game is on (the primary monitor when the game is not running), re-docked on every summon (hotkey, tray, second launch) and DPI-correct at 100-200%, drawn with `src/skin/`; Windows GUI subsystem in release builds (no console flash), debug builds keep a console for `eprintln!`/panic visibility. Entry point is `ferrisfilter::overlay::launch()`. Since Phase 18 it drives the game-folder `dynamic.filter` as a rule list over its parsed blocks (add, delete, duplicate, reorder, enable/disable, Show/Hide + Continue, naming; Apply / Revert / Reload) and a Config tab (filter choice, `styles.json`, `tiers.json`), with Economy (Sync economy) and About tabs — see the overlay notes below. **Debug-only screenshot mode:** `cargo run --bin ff -- --screenshot <png> [--tab rules|config|economy|about] [--popup changed|switch|quit|delete-rule|delete-style|remove-economy|reset-styles|reset-tiers|discard-config|discard-rules] [--status <text>] [--filter <path>] [--expand <n>] [--expand-economy <n>] [--search <text>] [--rename-section <n>] [--expand-style <name>] [--picker text|border|background|rule] [--dropdown sound] [--tier-text TIER=TEXT] [--stage-economy add|remove] [--leagues <names>|none|loading] [--league <name>] [--price-source live|fallback]` (`--leagues` injects the league list and never fetches it, `none` = the fetch failed, `loading` = still loading; `--league` and `--price-source` are saved into the temp home's `ff-settings.json` after start-up; `--dropdown sound` opens the first drawn style editor's Alert sound list, scrolled to the current entry; pair it with `--expand-style` or `--expand`; `--expand-economy <n>` opens the economy section and expands its n-th rule, read-only) renders the shell once, saves the PNG and exits. It refuses to run unless `FERRISFILTER_HOME` and `FERRISFILTER_GAME_DIR` both live under the temp folder (whose path must contain a `Temp` or `tmp` component, since `TMP`/`TEMP` could point anywhere), stages `--filter` under the filter name the temp home's `ff-settings.json` selects and never overwrites a file in a game folder it did not create itself, skips the single-instance guard, hotkey and tray, never applies, reloads or syncs, and is compiled out of release builds. |
| `ff-cli` (binary, v0.4) | The short-name console CLI — identical behavior to `ferrisfilter`, just a shorter name to type. Holds the console shim body `ff` used to have before it became the GUI binary. |
| `ferrisfilter` (binary) | The long-name console CLI, unchanged: `generate` / `sync` / `install` / `apply`. |
| `src/skin/` | **Shared PoE2 skin** (pub, used by the `ff` overlay: shell chrome, popups via `skin::popup::ff_modal`, flag icons via `skin::icons`): palette, Fontin SmallCaps/Regular fonts, embedded textures (`include_bytes!`, LINEAR + mipmaps), pixel-snapped nine/three-slice painting, the `ff_*` widgets (frame, header, close, tabs via the pure `tab_layout`, section, row, round check, slider, dropdown, button, arrows, text field, game-art scrollbar, `ff_status` one-line status with `StatusKind`, `ff_icon_button` (square three-state art button, used for the game's refresh art); for the RULES tab also `skin::of`, `ff_scroll_area_viewport`, `ff_field_row`, `ff_chip`, `ff_x_button`, `ff_text_field_id` / `_ex`, `ff_dropdown_ex`), `skin::editor` (segmented switch, colour swatch / card, item-label preview, minimap icon tiles, cards), `skin::minimap` (the 11 filter colours, 12 shapes, atlas UVs) and `skin::window` (DPI-correct, monitor-aware left dock). |
| `assets/poe2/` | **Game UI art + Fontin fonts extracted by `ff-art`** (committed): `options/` (Options window header, page frame with the gear cleared, tabs, slider, scrollbar, popup box, title rule), `common/` (round checks, close, button 3-slice, combo arrow), `minimap/icons.png` (the 12 minimap shapes x 11 filter colours, Large cells only, 768x704, ~0.5 MB, packed by an `[[atlas]]` manifest entry from the game's `minimap/player.dds`; the whole 4.7 MB sheet is not committed, `ff-art dump minimap/player <scratch>` shows it), `fonts/`, and `manifest.toml` (which game file becomes which PNG). The art belongs to Grinding Gear Games (README → Credits). Regenerate with `ff-art`; do not hand-edit. |
| `assets/custom/` | **Hand-made FerrisFilter art** (committed), each PNG next to its Aseprite source: `titlebar.png` / `titlebar.ase` (the header the overlay shows: the game title bar redrawn with Ferris, same 970x154 size and plate position, so it is a drop-in for `poe2/options/titlebar.png`; the one switch is `overlay::shell::header_art`) and `base-palette.ase` (the drawing palette). Edit the `.ase` in Aseprite and re-export the PNG over the old one; `ff-art` never writes here. |
| `assets/ferris/`, `assets/ui/ferris-bg.png`, `assets/ui/ferris-about.png` | Ferris SVG (public domain, rustacean.net), the pale line-art background and the orange Ferris on the ABOUT tab (`ferris-about.png`) that `ff-art ferris` renders from it. |
| `tools/ff-art/` | **Dev tool, a non-default member of the root Cargo workspace** (never a dependency of `ff`; uses the nightly-only `poe_data_tools` library, `image-extras` for DDS, `resvg`). Reads the PoE2 install read-only (`--game <dir>`, default the Steam folder) and writes the manifest entries into `assets/poe2/`, plus the Ferris background. Subcommands: `extract` (default; also packs `[[atlas]]` entries: chosen cells of a grid sheet into a smaller grid), `list`, `dump`, `sheet`, `info`, `zoom`, `ferris` (also renders the orange ABOUT Ferris, the `[ferris_about]` manifest entry: original colours, 336 px, no game install needed). Run with `cargo ff-art <cmd>` (or `cargo run -p ff-art -- <cmd>`) from anywhere in the repo; default output paths resolve against the repo root; root `cargo build`/`cargo test` never build it; set `CARGO_TARGET_DIR` outside the repo if `ff.exe` is running (the workspace shares one `target/`). Usage: `tools/ff-art/README.md`. |
| `Cargo.lock`, `rust-toolchain.toml`, `.cargo/config.toml` | One committed lockfile for the workspace (`ff` + `ff-art`), the nightly channel selected with rustfmt and clippy (a floating channel, not a specific compiler; set a dated nightly there if a new one breaks `poe_data_tools`), and the `cargo ff-art` alias (`run -p ff-art --release --`). |
| `CLAUDE.md` | This file — the filter-syntax reference so we rarely need to re-fetch the docs. |

> **Overlay (v0.4, `ff`):** Path of Exile 2 must be running in **windowed** or
> **windowed-fullscreen** mode — exclusive fullscreen prevents any external window from drawing
> above it, and `ff` does not detect this, so a blank/missing window means checking the game's
> display mode first (see `.planning/phases/13-overlay-shell/13-MANUAL-VERIFICATION.md`). Press
> **`Ctrl+Shift+Z`** (system-wide, hardcoded) to summon or hide the window without alt-tabbing —
> **only while Path of Exile 2 or the overlay window itself is the foreground window**, so the binding stays
> free for other applications; the
> window's own close button only **hides** it, and the system tray icon's **Quit** is the only way to exit
> the process. Only one `ff` runs at a time (`src/overlay/instance.rs`): launching it again
> (including by double-clicking the exe) brings up the running window, even when it is hidden to
> the tray, and the new process exits; if the running one does not answer within about 2 seconds a
> message box says "FerrisFilter is already running but not responding". A launch made while the
> running one is still starting up first waits up to 10 seconds for its first frame before that
> 2-second window starts, and several launches at once are each answered. Tray **Quit** gives up the
> hotkey, then the tray icon, then the single-instance name at once, so relaunching right after Quit starts a fresh window instead of
> asking the closing one (and a launch whose running instance quits or crashes while it is asking
> runs as the new instance rather than showing a message). If the single-instance
> check itself fails, `ff` does not start (a message box names the Windows error) rather than risk
> a second window. Debug builds use a
> separate instance name (`-overlay-debug`), so a `cargo run` build can run beside a release `ff`
> (their hotkeys then conflict, which the tray tooltip already reports).
>
> **The rule list and editor (Phases 19-20):** the overlay treats the game-folder `dynamic.filter` as its store (see
> the `dynamic.filter` row above); there is no `dynamic.json`. Every rule of the loaded filter (one block,
> or one `# ff:group` of blocks, see Simple Defences below) is a row in a flat, virtualized list inside the docked window's Rules tab (responsive on the ~500-block NeverSink/FilterBlade sample), with a
> search box (its hint carries the rule count: `Search N rules, classes, base types…`; **+ Add rule** sits beside it) matching the `# ff:name` and Class/BaseType values. A rule's name comes
> only from its `# ff:name` line; foreign header text such as NeverSink's `$type->` tags is kept on disk
> but never shown. An unnamed rule's header reads just `<index>  Show` (or `Hide`); the collapsed
> summary line gives the context. Rows expand in
> place (one at a time) with NAME, CONDITIONS, then STYLE, then Duplicate / Delete (Delete asks in a popup).
> NAME is a plain text box (no toggle): empty means no `# ff:name` line, clearing it removes the line,
> typing adds or updates it. STYLE is one picker of **Custom** plus every `styles.json` preset. For
> **Custom** only, the picker is followed by the same lines as the CONFIG style editor (Visibility,
> label preview + Text / Border / Background with alpha, Font size, then the Minimap, Beam, Alert
> sound and Drop sound groups described in the Config note, with the same options), read from and
> written to the rule's own action lines. A later action line of the
> same kind overrides an earlier one in game, so each line binds the **last** line of its keyword
> family (one per colour keyword; `SetFontSize`; `MinimapIcon`; `PlayEffect`; the sound family
> `PlayAlertSound` / `PlayAlertSoundPositional` / `CustomAlertSound` / `CustomAlertSoundOptional`;
> `DisableDropSound` / `EnableDropSound`), and an edit rewrites only that line in place, removes it
> (an earlier line of the family then shows through, as in game), or adds one line when the family
> has none: a family never gets a second line. A 3-value colour stays 3-value while alpha is 255; a
> colour added to an empty slot is written with 4 values; a sound kind switch keeps a trailing
> `# comment`. `MinimapIcon -1`, `PlayEffect None` and `PlayAlertSound None` are shown exactly as the
> `/` tile, `PlayAlertSoundPositional` with Positional ticked, `DisableDropSound` / `EnableDropSound`
> as Silent / Normal; the Positional toggle flips the keyword in place, keeping the volume form and
> a comment. When the bound line is one the editor cannot show exactly (`MinimapIcon -1 <colour>
> <shape>`, `PlayAlertSound None <volume>`, `PlayAlertSoundPositional None`, another named sound, an
> out-of-range font size or volume, `CustomAlertSoundOptional`, a `... False` drop sound line) its
> line shows the nearest honest state (the `/` tile; the group off; the value clamped into the
> slider; the plain file kind, whose keyword is kept on edit; the opposite drop sound), an edit
> rewrites that line, and the line is also listed below as a separate row with its exact text.
> Every other action line (earlier lines of a family, `DisableDropSoundIfAlertSound`,
> `EnableDropSoundIfAlertSound`) follows as an editable / removable row. There is no **+ style**
> button, so an action the style lines cannot express (`CustomAlertSoundOptional`, which the
> overlay never writes, and the two `...IfAlertSound` keywords) can no longer be **added** from the
> overlay; one already in the file is kept, shown and editable. A preset hides all of
> it, decides Show/Hide itself and writes `# ff:style <id> <name>`.
> Switching a preset back to Custom keeps the current actions and drops the tag. Only Show and Hide
> are offered: a foreign `Minimal` block is shown as Minimal (Custom only) and left as is unless
> changed. **Continue to later rules** is its own line, last in the body. No caption
> text; an unknown style is flagged on the picker with a tooltip, and standing explanations are tooltips on the control they explain. Every condition and every action line in the file is editable with structured
> controls (new actions are added through the style lines only, see above): a grouped, searchable **+ condition** picker, an operator dropdown with
> plain-English labels (an existing operator is never normalized), sliders paired with exact numbers,
> a class picker whose groups come from `config.json`, quoted lists, the skinned colour picker with
> an Alpha slider (on the style lines it opens from the Text / Border / Background cards, which have
> a None tile that removes the line; a leftover colour row is a chip whose `R G B A` text is its
> tooltip, without None; a 3-token colour line stays 3-token while alpha is 255), and minimap / beam / sound controls. An edit rewrites only the edited value's
> lines. A list condition added with **+ condition** is an unwritten row until its first entry.
> Quoted lists (BaseType, mod names) are a vertical list, one entry per line with its own remove, a
> type + Enter / **+ ADD** field that splits a pasted quoted list, in a scroll area of about 7 rows.
> Class is the grouped picker only (classes outside `config.json`'s groups are kept and listed on one
> compact line). Booleans are a compact **Yes | No** control (no operator; remove the condition to
> match either; an existing operator such as `!=` is kept). Condition remove buttons are right-aligned.
> **Sockets** is a plain numeric condition (operator plus a 0-6 slider); the overlay never edits socket
> colours, but a hand-written `Sockets >= 5GGG` still loads and keeps its letters when the count is
> changed.
> **Defences** is one condition (one card per rule) with Armour / Evasion / Energy Shield toggles, each
> with its own operator and slider; `BaseDefencePercentile` is gone from the keyword table (a line using
> it loads as a raw row; Ward is not supported yet). **Simple Defences** is the alternative card (same
> slot: both picker entries sit right after Rarity and are greyed when either is on the rule; clicking
> focuses the shown card): one wrapped line of seven multi-select chips, **Full AR**, **Full EV**,
> **Full ES**, **AR/EV**, **AR/ES**, **EV/ES**, **AR/EV/ES** (a tooltip names the base type). The
> selected chips are exactly what the rule matches. A block can only say, per type, has it
> (`BaseArmour > 0`), lacks it (bare `BaseEvasion 0`) or either (no line), so a pick is written as the
> minimum number of such blocks whose union is exactly the pick (fewer lines on ties): "no EV" (Full AR,
> Full ES, AR/ES) is one block with `BaseEvasion 0`, the three Full chips are three blocks, those plus
> AR/EV/ES are four (the most). No chip and all seven both write no defence line (the card stays open,
> showing that pick until reload); the X removes every defence line. The chips cover defence *base*
> types only: an item with no defences at all (weapon, jewellery, currency) also passes a block whose
> lines are only `... 0`, so pair the card with a Class condition. **Grouped rules:** a pick needing
> n > 1 blocks is written as n consecutive blocks identical in everything (header, `# ff:name`,
> `# ff:style`, other conditions, actions, Continue, enabled state, comments) except their defence
> lines, each carrying `# ff:group <id> <k>/<n>` (`<id>` random 6-8 lowercase alphanumerics, unique in
> the file). On load such a run (consecutive, only blank lines between, k = 1..n, same n, id used
> nowhere else, zero-based defence lines, otherwise identical) is **one rule**: one row, one index,
> count, search hit and summary (`Defences: Full AR, Full EV`), shown with the Simple card holding the
> union. Every edit acts on the whole group and keeps the copies identical (enable/disable, delete,
> duplicate with a new id, moving as one unit, style, name, conditions, actions; a chip change adds or
> removes blocks, and the group line goes away at one block); the grammar gate and the style report
> count it once. A run that fails those checks is shown as separate ordinary rules, each flagged
> `group broken`, and left as written until edited (Apply is not blocked). A single block whose defence
> lines are all zero-based (`0`, `= 0`, `== 0`, `> 0`, `>= 1`, at most one per type) loads as Simple
> Defences, its chips showing what it matches, with its bytes kept until a chip is clicked; any other
> defence line loads as Defences. A Defences card stays Defences while editing (even if its lines become
> zero-based) and Apply keeps it; only Revert, Reload or removing the card re-classify from the file.
> There is no armour-class check. Whole-number conditions (and each defence) offer a
> **Between** operator: one card with two sliders writing `X >= min` and `X <= max`, clamped so min never
> exceeds max; a loaded one-lower/one-upper pair shows as one card with its bytes kept until edited
> (a loaded impossible pair is flagged and left as written), while other duplicates (two `>=`) stay
> separate cards. In the **+ condition** picker, sections start collapsed (a search opens matches), a
> condition already on the rule is greyed (clicking it focuses the existing card instead of adding
> another), and a pending condition shows its operator dropdown before the first value.
> Lines with an unknown keyword show as raw rows (Enter commits; an edit that would add a grammar
> Error shows an inline hint and changes nothing). PoE1-only conditions (e.g. `LinkedSockets`,
> `SocketGroup`, `HasInfluence`, `MapTier`, `Exceptional`) are not in the keyword table or picker; a filter using
> them still loads byte-for-byte, shows them as raw rows, and Apply is not blocked (warnings never
> block), and an invalid commented-out block is shown
> read-only. Arrows reorder
> (disabled while searching; a move across a Continue rule is flagged `moved across Continue`). The
> circular toggle comments a block out (`# ` before each line) or back in; commented-out blocks load as disabled rules, and a
> commented block that is not valid grammar is flagged `can't enable` (padlock icon). Enabled rules with grammar
> errors are flagged and **Apply is refused** until they are fixed or disabled (warnings are flagged
> but do not block). **+ Add rule** inserts an empty `Show` rule as the first block of the file (above the first rule's own comments, outside the economy section), expands it with the Name field focused, clears the search and reports `Rule added at the top of the file. Press Apply to save it.`. The list is drawn with the PoE2 skin (Phase 31): fixed two-line rows (round check, index, Show/Hide, name, muted summary line), flags as below with the styled chip left of the gold `Continue` word, and the game scrollbar. The editor is two columns (labels left, skin controls right): booleans are a Yes | No switch, Custom has a Show | Hide switch, Continue is a round check, Duplicate / Delete are buttons (Delete via its popup), quoted lists have a **+ Add** field. The
> **Sections (Phase 32):** a `# ff:section` banner is a 36pt header row (chevron, name, `{n} rules`, rename pencil, up / down, remove x) that collapses its rules; rules above the first banner have no header. **+ Add section** (toolbar) inserts a banner at the top of the file with its name box focused; **Section above** (beside Duplicate / Delete on an expanded rule) puts a banner above that rule (above the whole run for a grouped rule). Rename trims, an empty name becomes `Section`, newlines are stripped. A section's arrows move its divider past one rule at a time (a whole `# ff:group` run counts as one rule; dimmed when no rule is left to step over); a rule's arrows hop a divider with the rule. Remove deletes only the banner (its rules join the section above); the divider right below the economy section can neither be removed nor move down, since that would hand its rules to the economy section. Search shows matching rules inside collapsed sections (header expanded for display, `m of n rules`). Collapsed state is per filter in `ff-settings.json` (`collapsed_sections`), keyed by section name (a name starting with `ff:` or `user:` is stored with a `user:` prefix so it never shares the economy key); renaming moves the flag, removing drops it, a new section starts expanded, and jumping to a rule opens its section. A `+ Add rule` still inserts above everything, including a leading banner. A rule with no conditions, enabled, not `Continue`, and followed by more rules carries an amber triangle (WR-02: it matches everything, so later rules never fire). The economy section is the locked last header, shown as **FF Economy Section** (display only: the banner in the file keeps its `# ff:economy-begin Economy (generated by FerrisFilter, do not edit)` text and the collapsed flag keeps the economy key; padlock, collapsed by default, `{n} rules`), whose rows are muted and read-only. A click on an economy rule expands it (one row open at a time, shared with the user rules) into the same editor body as a user rule (Name only when the block has one, Conditions, Style: a tier block shows its preset, a unique block the Custom lines), and its collapsed row shows the name and the style chip (minimap icon and style name) like a user rule, every control drawn disabled in a disabled `Ui`: no clicks, drags, focus or keys, no picker or popup opens, quoted lists show every entry (a disabled scroll area cannot scroll), and there is no check, no arrows and no Section above / Duplicate / Delete. The body runs against a throwaway copy of the list state and a throwaway action list, and the model refuses any action aimed at an economy block anyway. Search still finds economy rules (by name too). A staged **Add economy section** already lists its rows here, read-only and priced with the pending styles and tiers, exactly what Apply will write (re-priced when they change); a section already in the file is shown the same way while the CONFIG tab has style or tier edits, and as the file's blocks otherwise. An enabled preset-styled user rule shows the pending style's Show / Hide in its row (what Apply will write); a disabled or Custom rule shows the file's keyword. The header is display-only: no remove x and no footer button; the section is added and removed on the **Economy** tab. All edits stay in memory until **Apply** writes them (a
> filter without an economy section can be applied); **Revert** re-reads the last applied text (after the **Discard rule changes?** popup; it is live only with rule edits and keeps a staged economy add/remove, which the Economy tab owns),
> **Reload** re-reads the file and discards unapplied edits (also needed when the folder was not
> found, the filter could not be loaded, or Apply was refused because the file changed on disk), and
> **Sync economy** (Economy tab) is described below. **Apply** is one action shared with the Config tab (see the
> Config note). **Revert / Reload / Apply** live in the bottom bar of the window, on every tab. When Apply is refused because a file changed on disk the refusal names the file(s), the **File changed on disk** popup opens (Reload / Overwrite / Keep editing) and the Apply slot reads **Overwrite** until the file is reloaded or overwritten (it reopens the popup); Overwrite
> counts as an Apply, in-game reload included (the normal guarded write: same atomic write,
> retry and one-time `.ffbak`). An Apply
> whose result equals the file on disk is not rewritten but still counts as applied: same status,
> same in-game reload. Opening the overlay never writes: a v0.4 filter is announced as
> "will be converted on the next Apply" and a `.ffbak` backup is kept by that first write. Picking a preset in the STYLE section writes
> `# ff:style <id> <name>` (the id links the rule to the style; the name is informational and refreshed on Apply; an old `# ff:style <name>` tag still resolves by name and is rewritten to the new form on the next Apply; that tag-only migration does not mark the filter unapplied or trigger the Quit confirm) and sets the rule's actions from `styles.json`. On every Apply, styled rules are
> rewritten from the current `styles.json` (re-read at Apply, and watched while the overlay runs: a
> change on disk marks the filter as having unapplied changes), and the outcome says
> `Updated N styled rules.`. A preset's actions are not shown in the UI; a hand edit
> of a styled rule's actions in the file is overwritten on the next Apply. Switching to **Custom**
> keeps the current actions as the rule's own. A tag naming a style that
> does not exist is flagged `unknown style` (ring-with-? icon), reported in the status line (click jumps to the first such rule), and never rewritten or
> blocking. A disabled styled rule is rewritten once it is enabled. A collapsed rule shows a one-line condition summary under its header (hidden when expanded). Collapsed rows show their flags as small painted icons with tooltips (the word flags are only accessible names), right to left from the reorder arrows: grammar error (red filled circle with !), grammar warning (amber triangle with !), unknown style (amber ring with ?), group broken (amber broken link), moved across Continue (amber up/down arrows), `Continue` (gold word), and `styled` (the style's minimap icon in game art and its colour, plus the style name truncated to 10 characters; the name only when the style has no minimap icon, i.e. Minimap off or `/`; a Custom rule whose effective (last) `MinimapIcon` line shows an icon shows just that icon, none when it is `MinimapIcon -1`, tooltip `Minimap icon: <Size Colour Shape>`). A disabled rule that cannot be enabled shows an amber padlock. Flags beyond a 120px budget fold into a `+N` badge (at the far left of the group; errors are never folded away), so the
> rule label always keeps at least 140px. The empty states are a heading only (`No rules yet`, `No rules match "<query>"`, `The filter could not be loaded.`).
>
> **Bottom bar and status line (Phase 30):** every message that v0.5 stacked in a scrolling 96px footer block (Apply outcome, list status, sync status, the grammar-gate and unknown-style lines, filter state, indicator) is now **one status line** to the left of **Revert / Reload / Apply**, one message at a time. An event message (Apply, Overwrite, Reload, Revert or Sync outcome, a list status) shows until the next edit; otherwise the highest-priority standing condition shows: unusable FerrisFilter folder, event, grammar gate, damaged markers or load failure, game folder not found (a click opens Config), sync running, ended league (`<name> is no longer on poe.ninja; pick a league`, warning, click opens Economy; not under Fallback), unknown styles, v0.4 notice, absent filter, then the indicator (`Unapplied changes`, its explanation on hover, or `Filter matches saved rules`). It is coloured info / ok / warning / error, is cut with … on one line, and the whole text appears on hover only when it was cut. Clicking a grammar-gate or unknown-style message jumps to the first such rule. The filter's full path is the tooltip of the header title plate. A successful Apply on either tab (or Overwrite) sends
> `/reloaditemfilter` to a running Path of Exile 2 when the Config setting **Reload filter in game
> after Apply** is on (the default), and the outcome reads `Reloaded in game` -- meaning every key was
> sent to the game window; ff cannot see whether the game ran the command (an elevated game silently
> drops the keys, see the caveats below), so if the game did not reload, Refresh by hand. The result
> lands after the game has taken focus (which hides the window) and is shown on the next reveal. A
> refused Apply sends nothing. With the game not running the outcome is just `Wrote <path>.` (the game reads the filter when it starts). For the other reasons the outcome keeps the **Options → Game → Item
> Filter → Refresh** reminder plus the reason (`reload setting is off`, `a reload
> is already in progress`, `the overlay window is not ready`, `could not reload in game: <cause>`).

> **The tabs and the Config tab (Phases 21, 30, 34):** the window has four stone tabs under the header: **Rules | Config | Economy | About**. Switching tabs
> never applies or discards anything; a tab label shows ` *` for its own unapplied edits (the selected tab included): Rules for rule edits, Config for style and filter-choice edits, Economy for tier edits, a staged economy add/remove and a pending price-source switch; About never. **Economy** holds **Price source** (a `Live | Fallback` switch, saved at once; its tooltip recommends Fallback at league start and for non-trade players; Live without a usable `prices.json` (unreadable, empty or item-less) still prices from the fallback pair, reason on hover, Last sync Never), **League** (dropdown plus refresh), Last sync (exact time and the priced league on hover), Priced items. Under Fallback the league dropdown, refresh, Sync economy, the Tiers editor and its Reset to defaults are greyed (`Not used with Fallback prices.` / `The fallback tiers are fixed.`), tier errors do not block Apply, and the next Apply prices the economy section from the fallback pair; a source switch that changes what Apply writes (a filter with an economy section) marks the Economy tab and the unapplied indicator, is not undone by Revert, and clears when switched back or applied, **Sync economy**, the one **Add economy section / Remove economy section** button (Add stages the begin-only banner at the end of the file; Remove asks in the **Remove economy section?** popup; both are written by Apply and undone by Revert), and the **Tiers** cutoffs editor (with a **Reset to defaults** button on its heading: the tiers back to `tiers-default.json` as one pending edit, after the **Reset tiers to defaults?** popup, greyed at the defaults; the `tiers.json` cutoffs, inline validation; an invalid cutoff blocks Apply; each box is 88pt, about 8 digits, starts at the value column of the rows above and has a 10pt gap before its unit). **About**: name, description, orange Ferris, version (baked in by `build.rs` from `git describe --tags --match v* --dirty`: the tag without its `v` on the tagged commit with no tracked edits, `<tag>-dev` past it or with edits, the full describe on hover; the `Cargo.toml` version when built without git or a tag), the repository link (the `origin` remote as an https web URL, credentials dropped; `Cargo.toml` `repository` when there is none; parsing in `src/build_info.rs`), a divider, then the GGG non-affiliation disclaimer and the credits at the bottom. Config has two sections, **Filter** and **Styles**, with skin headings. **Filter** picks which `*.filter` in the game folder the RULES tab edits
> (or creates a new empty one, written by the first Apply); the choice applies at once, is saved
> in `ff-settings.json` and is restored on the next launch (with none saved: the folder's `dynamic.filter`, else its first `.filter`, else a new `dynamic.filter`; see the `ff-settings.json` row). A **Game folder** row above it shows the folder (cut with an ellipsis, full path on hover, amber when it does not exist) with a **Browse** button: the native folder dialog, parented to the overlay, applies and saves the pick at once (the same filter name is kept when the new folder has it, else its first `.filter`, else the current name; with unapplied rule edits the **Switch filter?** popup asks first). Browse is greyed while the dialog is open or when `FERRISFILTER_GAME_DIR` is set (its tooltip says so, otherwise it shows the default folder). The round game-art **refresh** button beside the filter dropdown re-lists the folder's filters and the sound files (it never writes or discards anything); with the folder missing the dropdown is greyed and the status line reads `Game folder not found: <path>` (clicking it opens Config), while refresh stays live so a folder that appears later is found (re-picking the same folder with Browse re-checks it too). Filter also holds
> the **Reload filter in game after Apply** toggle (saved on change). Picking another
> filter while the RULES tab has unapplied edits asks first in the **Switch filter?** popup (`Switch and discard` / `Stay`). **Styles**
> opens with the **strictness** row (`Custom | Flat Broke | Scrounging | Getting By | Comfy | Loaded | Div Farmer`; the levels hide nothing, F, E-F, D-F, C-F, B-F of `builtin:S`..`builtin:F`, found by id; a pick is one pending edit; the level is derived from the S-F visibility, any other pattern reads **Custom**, which is display-only; the other built-ins and custom styles are untouched; a fresh install reads Flat Broke) and a **Reset to defaults** button on its heading (built-ins back to the default values, names and order, custom styles kept after them, greyed at the defaults; one pending edit behind the **Reset styles to defaults?** popup). Below that it is one table per group, each with its own header row `Visibility | Name | Icon | Preview | Order` (Icon = the style's minimap icon,
> blank when Minimap is off or `/`; no Show/Hide caption, the switch says it): **Built-in** (switch and actions editable; no rename or delete; the
> Order arrows are shown greyed so the columns line up), then the **Custom** heading, which carries the
> **+ Add style** button and is shown even with no custom styles, with live Order arrows (at the table's right edge, like the RULES arrows).
> Clicking a row expands a **line-by-line editor** without frames (a label too long for its column is cut with an ellipsis, full text on hover): Visibility Show/Hide, Label preview, Text / Border / Background colour cards,
> Font size (check + slider 1-45; off shows a dimmed slider reading `Default`), then four **groups**, each on one tinted band whose first line is its label and an on/off round check (off folds the group to that line and writes nothing, the game default): **Minimap** (a `/` tile = `MinimapIcon -1`, then the 11 colours; the 12 game shapes; Large / Medium / Small), **Beam** (`/` = `PlayEffect None`, 11 colours, Temp),
> **Alert sound** (`/` = `PlayAlertSound None`, then a dropdown of Alert 1-16 and a **Filter folder** section listing the `.mp3` / `.wav` files of the loaded filter's folder; Volume; **Positional**, which writes `PlayAlertSoundPositional` and is greyed for a file) and **Drop sound** (on: a `Silent | Normal` switch writing `DisableDropSound` / `EnableDropSound`); with `/` picked the other pickers stay visible but greyed (a colour or sound pick leaves `/`); a loaded `MinimapIcon -1 <colour> <shape>` shows that colour on the shapes under `/`, and leaving `/` keeps that shape (it wins over a value remembered this session, and unticking the group keeps it for later; otherwise the remembered value, then White Circle). Every `/` tile has the same framed box, and a colour card whose `<Label> (none)` caption does not fit shows the bare label, still dim, with the full text on hover. The folder is listed read-only on load, Reload, a filter switch and whenever the dropdown opens, never per frame and never written; a file the style or rule names that is not in the folder keeps its line and shows in amber with the warning flag (tooltip: the game plays no sound until it is there); when the folder is unknown or cannot be listed nothing is flagged, and the file shows plainly with a tooltip saying the folder could not be listed. There is no free-text file field, and the overlay never writes `CustomAlertSoundOptional`. For custom styles **Delete style** sits at the bottom (the Delete style? popup);
> turning a group or Font size back on, or leaving `/`, restores the last values (kept when the row is folded, forgotten on Revert, Reload or Apply). Edits stay in memory until Apply. A custom style's name is an inline text field: renaming keeps its rules linked by id (rules
> still carrying a legacy name-only tag for it are moved to the new name in memory too) and is one
> revertible edit. Caveat: `config.json` references styles by name, so renaming a custom style it uses
> makes `generate` fail with the unknown-style error until `config.json` is updated. Custom styles can be
> deleted and reordered with up/down arrows (order = `styles.json` order, saved by Apply, undone by
> Revert; a custom style never moves above the built-ins). There is no new/edited
> tag. Each style has an on/off
> slide switch (on = Show, off = Hide) that reaches the filter on the next Apply (styled rules
> and the economy section are regenerated), which hides a whole group of styled rules at once (styles store RGB and the filter always emits alpha 255, so the style colour picker has no alpha). Deleting a
> style shows how many rules use it, and warns when `config.json` itself references it
> (`generate` would then fail); Delete opens the **Delete style?** popup. **Colour picker** (shared by the style colour cards and the RULES colour actions): game box frame, saturation/brightness square, hue bar, Hex and R/G/B fields, the nine Dracula quick picks; **None** on style slots (game default), no alpha for styles; live edits, Escape restores the opening colour, a click outside or **Use colour** keeps it, opening without a change edits nothing; on an empty slot **Use colour**, any quick pick (even the highlighted one) or a Hex / R / G / B value confirmed with Enter sets the colour shown.
> There is **one Apply**, the same on either tab (the shared bottom bar; there is no Config footer). It validates everything first (tier cutoffs, rule
> grammar, economy markers and pricing, style rewrites, and the changed-on-disk guard of the filter and of any config file
> with CONFIG edits; an unedited `styles.json`/`tiers.json` just follows the disk and is never
> written), then writes `styles.json`, `tiers.json` and the filter (rendered with the pending styles and
> tiers) all or none, then reloads in game under the existing rules. With no filter loaded it writes
> the config files only. A refusal writes nothing. When a file changed on disk the refusal names it
> and the **File changed on disk** popup offers **Reload** (re-reads the changed files; a changed filter discards rule edits, a
> changed `styles.json`/`tiers.json` discards config edits), **Overwrite** (writes everything over
> them) and **Keep editing**; nothing is overwritten silently. **Revert** stays per tab and always asks first (Rules discards rule edits after **Discard rule changes?**, Config
> discards style edits after **Discard config changes?**, Economy discards tier edits and a staged economy add/remove after the same popup titled **Discard economy changes?**, even when only the staged op is pending), and every tab shows the same unapplied state and the same status message.
> The explanatory captions of the Filter section (saved-on-this-computer, the reload-toggle paragraph, empty-draft hints) are tooltips on their control.
> The RULES style picker and tag checks follow the unapplied CONFIG style set. Tray **Quit** with anything
> unapplied opens the **Unapplied changes** popup: **Apply & quit** (quits only after a successful Apply, and waits for an in-game reload to
> finish) / **Quit without applying** / **Cancel** (Escape or a click outside picks Cancel); with nothing pending it exits at once.
> After Apply the Rules tab sees the new styles at once. Rules whose
> `# ff:style` names a deleted style lose that tag on that Apply and keep their actions.
>
> **Popups (Phase 30):** every confirmation is one skinned game-box popup over a dimmed window, one at a time (stacking: Quit, then File changed on disk, then the rest). Escape or a click outside picks the last, safe button, and a popup stays open when the window is hidden and summoned again. The ten: **Unapplied changes** (Apply & quit / Quit without applying / Cancel), **File changed on disk** (Reload / Overwrite / Keep editing), **Switch filter?** (Switch and discard / Stay), **Delete this rule?** (Delete / Keep), **Delete style?** (Delete style / Keep style), **Remove economy section?** (Remove / Keep), **Reset styles to defaults?** (Reset / Keep), **Reset tiers to defaults?** (Reset / Keep), **Discard config changes?** (Discard / Keep editing; titled **Discard economy changes?** from the Economy tab), **Discard rule changes?** (Discard / Keep editing). Every Revert opens one of the last two. Destructive buttons have a red caption.
>
> **Development warning:** running `ff` from the repo without `FERRISFILTER_HOME` uses the real
> `%APPDATA%\FerrisFilter`, so Apply (from either tab, once CONFIG has edits) writes your live `styles.json` /
> `tiers.json` there. Set `FERRISFILTER_GAME_DIR` as well to keep the game folder out of reach. Set `FERRISFILTER_HOME` to the repo (the author's workflow; the git-ignored live files
> then sit at the repo root) or to a temp copy to keep that folder intact. Nothing committed changes
> (`artifact_guard` and the tests read the `-default.json` files).
>
> **In-game reload after Apply (Phase 23, RELOAD-01):** the mechanism is: focus the game, Enter,
> paste `/reloaditemfilter` from the clipboard (Ctrl+V), Enter -- the same clipboard-paste method
> Exiled Exchange 2 and Awakened PoE Trade use for `/` commands. There is deliberately **no Ctrl+A**
> (EE2 skips select-all for `/` commands too): a synthetic Ctrl+A fired another tool's global
> hotkey. Ctrl+V and Enter are the only chords/keys sent. One Apply click runs one send sequence, never repeated
> or retried. The game window is found by title ("path of exile", so "Exiled Exchange 2" does not count)
> AND by a process image name starting with `PathOfExile`, so a browser tab titled "Path of Exile 2"
> is not the game. Keys are sent only after checking the game is the foreground window; otherwise
> nothing is sent and the reason is shown. Caveats: `/reloaditemfilter` reloads the filter **selected
> in game**, so if the overlay edits a different filter than the one picked in Options → Game → Item
> Filter, the reload will not show the edit (the overlay does not switch the game's filter). Close the
> chat box before pressing Apply: nothing outside the game can tell whether chat is open (EE2 and
> Awakened PoE Trade share this), so if it was already open with text, the first Enter sends that
> text as a chat message and `/reloaditemfilter` is not sent. No key is pressed before that Enter
> on purpose (with chat closed it would reach the game's own bindings). Run `ff` and the
> game at the same privilege level (Windows silently drops input into an elevated game, undetectably,
> so the status still reads `Reloaded in game`). The clipboard's
> text is restored about 120 ms later unless something else was copied meanwhile; non-text clipboard
> content (an image, copied files) is replaced. The setting is the `reload_after_apply` key in
> `ff-settings.json`, saved as soon as it is toggled; absent means on, and a value that is not
> `true`/`false` means **off** with a warning naming the key (an unreadable opt-out fails closed).
> The same holds for the whole file: an `ff-settings.json` that exists but cannot be used (invalid
> JSON such as a trailing comma, or unreadable) means `dynamic.filter` (even when the folder has
> other filters) **and** reload off, with a warning that says both; only a missing file means the
> default (on).
> `ff-cli apply` does not send
> the command.

> **Economy sync from the overlay (v0.4):** a **Sync economy** button on the Economy tab refreshes
> `prices.json` in the FerrisFilter folder (see the FerrisFilter folder note in the `ferrisfilter apply` row), on a worker thread so the window never freezes on network
> I/O. It writes prices and nothing else — `dynamic.filter` still changes only on **Apply**, so the
> button is safe to press mid-map and the new economy lands on the next Apply you choose to make.
> The GUI exposes no `--standard`/`--hc`/`--league`/`--tiers` switches: it syncs with the tiers file
> already in place, for the league picked on the Economy tab. The list is poe.ninja's `economyLeagues`
> (index-state), fetched on a worker the first time the Economy tab is opened in a session and by the
> refresh button beside the dropdown, never at start-up. With nothing saved the default is the first
> non-hardcore league that is neither Standard nor Hardcore; poe.ninja's `indexed` flag is ignored
> because it flipped between Forbidden Rites and Runes of Aldur within six hours on 2026-09-10/11. The
> pick is saved only when the user picks. Offline, the saved league (else the league `prices.json` last
> priced) stays usable. A saved league that is no longer listed shows as `<name> (ended)`, greys
> **Sync economy** and is never switched silently. The outcome (status line) names the league it priced
> against. The CLI keeps automatic resolution and its `--league` override, and reads neither overlay key.
>
> **Design philosophy (current):** Hide what we don't want, style the few highlights, and let
> everything else fall through to the game default. Uniques, jewels, gems, quest items,
> waystones, and trials have **no rules** — they show natively. Jewellery, unidentified
> high-tier items, and equipment are covered by explicit rules in the `config.json` `rules`
> array. `config.json` is the authoritative model — the reconciliation of the old
> hardcoded-band description with the actual code is complete as of v0.2.

Source folder for filters in-game (where they must be copied to be usable):
`C:\Users\jonat\OneDrive\Documentos\My Games\Path of Exile 2` (the author's Documents folder; ff resolves it from the OS)

External references (fetch only when something here is insufficient):
- Filter syntax: https://www.pathofexile.com/item-filter/about
- Exact item names / base values: https://poe2db.tw/us/

> **Caveat:** The official `item-filter/about` page documents the PoE1 filter language.
> PoE2 uses the *same grammar* but a different set of valid values (classes, base types,
> tiers) and omits some PoE1 concepts (see "PoE2 differences" below). When in doubt about
> an exact base name, class, or numeric range, verify against poe2db.tw or the live game.

---

## Filter grammar

A filter is an ordered list of **blocks**. Each item that drops is tested against blocks
**top to bottom**; the **first matching block wins** (unless `Continue` is used). A block is
a header (`Show` / `Hide` / etc.) followed by indented **conditions** and **actions**.

```
Show # Human description (optional-slug)
  Rarity == Unique
  Class == "Body Armours"
  SetTextColor 53 30 14 255
  MinimapIcon 1 Brown Kite
  PlayEffect Brown
```

### Block types
- `Show` — display matching items.
- `Hide` — conceal matching items.
- `Minimal` — minimal label (Ruthless filters only; rarely relevant for PoE2).
- `Continue` — placed inside a block so matching does **not** stop here; subsequent blocks
  may also apply (lets you layer styling rules).
- `Import "file.filter" [Optional]` — include another filter file.

### Comments
- `#` starts a comment to end of line. A `#` after a block header is a common convention for
  naming the block (e.g. `Show # Excellent Uniques (uniques-excellent)`).

### Operators
- `=` equal · `==` exact match · `!=` / `!` not equal
- `<` `<=` `>` `>=` for numeric/ordinal conditions
- For named/list conditions (e.g. `BaseType`, `Class`), listing multiple quoted values means
  **match any**. `==` enforces *exact* full-name match; without `==` it matches as a
  substring/prefix (e.g. `Class "Bows"` also matches `"Bow"`-containing class names). Prefer
  `==` with full names to avoid accidental matches.

---

## Conditions (common / PoE2-relevant)

**Numeric / ordinal**
- `ItemLevel` — item level.
- `DropLevel` — base item's drop level.
- `AreaLevel` — level of the area where it dropped.
- `Quality` — 0–20+ (corruption can exceed).
- `Rarity` — `Normal` < `Magic` < `Rare` < `Unique` (supports `<`/`>` ordering and `==`).
- `Sockets` — socket count (e.g. `Sockets >= 3`). PoE2 sockets hold runes/soul cores.
- `Height` / `Width` — item's inventory dimensions.
- `StackSize` — for stackable currency.
- `WaystoneTier` — PoE2 map (Waystone) tier.
- `BaseArmour` / `BaseEvasion` / `BaseEnergyShield` — base defence values.
- `UnidentifiedItemTier` — confirmed PoE2 (author-tested, v0.2): highlights high-tier
  unidentified items (e.g. `UnidentifiedItemTier >= 4`). Used by the unidentified rules, which
  use the custom `kite` style (not shipped; the author's own). Note the condition and the style are independent: the condition is what
  matches, `kite` is only how a match is drawn.
- `GemLevel` etc. — verify PoE2 applicability before relying on them. PoE1-only conditions are
  not in FerrisFilter's keyword table (kept as raw lines when met).

**Boolean (value `True`/`False`)**
- `Corrupted`, `Mirrored`, `Identified`, `FracturedItem`, `HasImplicitMod`, `AnyEnchantment`
- `IsVaalUnique`, `HasVaalUniqueMod` (seen in the user's PoE2 uniques rules)

**Named / string**
- `Class "..."` — item class (PoE2 classes include: `Bows`, `Crossbows`, `Quarterstaves`,
  `Spears`, `Wands`, `Sceptres`, `Staves`, `Foci`, `One Hand Maces`, `Two Hand Maces`,
  `Quivers`, `Talismans`, `Body Armours`, `Helmets`, `Gloves`, `Boots`, `Shields`,
  `Bucklers`, `Rings`, `Amulets`, `Belts`, `Stackable Currency`, ...). Verify exact spelling.
- `BaseType "..."` — exact base item name (e.g. `"Sapphire Ring"`, `"Gold"`). The most
  precise lever; pull exact names from poe2db.tw.
- `Rarity` — see above.
- `HasExplicitMod`, `HasEnchantment`, `HasInfluence` — verify PoE2 support before use.

---

## Actions (styling & alerts)

**Colors** — `R G B [A]`, each `0–255`:
- `SetTextColor R G B A`
- `SetBorderColor R G B A`
- `SetBackgroundColor R G B A` (default alpha 240)

**Text size**
- `SetFontSize N` — `1–45` (in-game usable range ~18–45; the user's filters use up to 37).

**Sounds**
- `PlayAlertSound Id Volume` — `Id` 1–16, `Volume` 0–300 (default 50).
- `PlayAlertSoundPositional Id Volume` — positional variant.
- `CustomAlertSound "file" [Volume]` — custom sound file (e.g. `uwu.mp3`),
  volume 0–300 (default 100). `CustomAlertSoundOptional` to tolerate a missing file.
- `DisableDropSound` / `EnableDropSound` — toggle the default drop sound.

**Map / world**
- `MinimapIcon Size Colour Shape`
  - `Size`: `0` (largest) – `2` (smallest).
  - `Colour`: Red, Green, Blue, Brown, White, Yellow, Cyan, Grey, Orange, Pink, Purple.
  - `Shape`: Circle, Diamond, Hexagon, Square, Star, Triangle, Cross, Moon, Raindrop, Kite,
    Pentagon, UpsideDownHouse.
- `PlayEffect Colour [Temp]` — colored beam on the ground. Same colour set as above.
  `Temp` shows only briefly when the item drops.
- Cancel values: `MinimapIcon -1` (no icon), `PlayEffect None` (no beam) and `PlayAlertSound None`
  (no sound) undo what an earlier `Continue` block set; the overlay's `/` tiles write them.

---

## PoE2 differences to remember
- **No linked sockets / colors-as-links.** PoE2 has no `LinkedSockets`/`SocketGroup` link
  mechanic; `Sockets N` is just a count. Don't author link-based rules.
- **Waystones, not maps.** Use `WaystoneTier` for endgame map tiering.
- **Different base/class names.** Many PoE1 base types don't exist; always confirm via
  poe2db.tw before adding a `BaseType`.
- Existing filters in this repo were generated/edited with **poe2filter.com** and use a
  `Show # Description (slug)` naming convention — keep that style for consistency.

## Tier & color system (Dracula palette)

Color signals **value/desirability**. Text + border use the tier color; background is the
basic Dracula bg `40 42 54 255`. Beams/sounds only on the most valuable. These tier colors
are stable across leagues — only *which items* sit in each tier changes per season.

> **Native rarity color is preserved on uncurated equipment.** Tier *colors* apply only to
> **curated** drops — specific named bases (`BaseType ==` lists), currency, gems, and the
> Recycle/unique rules. Generic equipment/jewellery/flask/charm blocks that match by
> `Class` + `Rarity` alone must **not** repaint the item: omit `SetTextColor` /
> `SetBorderColor` / `SetBackgroundColor` so the game's native rarity color (Normal white,
> Magic blue, Rare yellow) shows through. For those blocks, **emphasis is conveyed by font
> size only** — bigger = more relevant, font 21 = de-emphasized.

| Tier | Meaning | Color | RGB (text+border) | Alert |
|---|---|---|---|---|
| **S** | top | Pink | `255 121 198 255` | alert sound 6 at 300 + beam + minimap, pale `248 248 242` background |
| **A** | excellent | Purple | `189 147 249 255` | alert sound 6 at 300 + beam + minimap, pale background |
| **B** | high | Red | `255 85 85 255` | alert sound 2 at 200 + beam + minimap |
| **C** | notable | Cyan | `139 233 253 255` | alert sound 2 at 200 + Yellow beam + minimap |
| **D** | minor | Green | `80 250 123 255` | White beam + minimap |
| **E** | baseline | Yellow | `241 250 140 255` | — |
| **F** | de-emphasized | White | `248 248 242 255` | — (the `hidden` style is the Hide one: native colour, font 21 + `DisableDropSound`, Alt-reveals it) |
| **Recycle** ★ | salvage value | Green | `80 250 123 255` | custom style, not shipped (the author's own); green minimap icon + beam; overrides tier |

Full Dracula RGB reference: bg `40 42 54` · fg/white `248 248 242` · cyan `139 233 253` ·
green `80 250 123` · orange `255 184 108` · pink `255 121 198` · purple `189 147 249` ·
red `255 85 85` · yellow `241 250 140`. (Source: dracula/dracula-theme.)

**Rules:**
- **Uniques are highlighted by price tier, never blanket-restyled.** `sync` fetches per-unique
  prices from poe.ninja's `stash/current/item/overview` endpoint and records each with its
  filter **base type** (e.g. `Voidforge → Infernal Sword`). `generate` groups those bases into
  `Show # Unique <tier>` blocks (`Rarity == Unique` + `BaseType ==`, deduped per base to the
  highest tier). Price-tiered unique blocks get the tier's text/border/beam **plus a distinct
  bronze background `SetBackgroundColor 175 96 37 240`** so they read as "valuable unique"
  regardless of tier color. Uniques whose base is *not* priced get no rule — they fall through
  to native game style. There is still **no blanket restyle of all `Rarity == Unique`**; only
  curated/priced bases are touched. (Caveat: a `BaseType` match highlights every unique on that
  base, since PoE2 filters cannot target a unique by name.)
- **Unwanted items are hidden, not deleted.** The `hidden` style uses `Hide` — off the screen
  by default but **revealable by holding Alt** in-game. Hidden blocks keep no color overrides
  (native rarity color), font 21, and `DisableDropSound`, so an Alt-reveal renders them
  de-emphasized, not in the game default. The `hidden` style currently applies to equipment
  (gear fallback rule), jewellery (rings/amulets/belts fallback rule), and the flask/charm
  fallback rule.

**Flasks & charms** have three show rules (native rarity color throughout): quality `≥1` always
show (highest priority); `AreaLevel ≤25` show all rarities; `AreaLevel ≤65` show `Magic` or
better (`>=Magic`). Deep endgame (AreaLevel ≥78) surfaces Normal-rarity charms. Everything else
falls to the `hidden` flask/charm fallback rule.

**Jewels** (`Class "Jewels"`) have **no rule in `config.json`** — they fall through to the game
default (GEN-04), which is what the emitter's `generate_no_unique_or_jewels_blocks` test holds.
The older "always shown, font 32, cyan diamond (`jewels-all`)" description came from the
hand-authored poe2filter.com filters in `reference-old-filters/` and has not described
`universal.filter` since v0.2. The class *is* offered in the overlay's class picker (below), so a
dynamic rule can target it — that is the editor, not the generated baseline.
- **Recycle** = items with **both `Sockets >= 1` and `Quality >= 1`**. Shown green regardless
  of zone level; the Recycle rule is first in the rules array so it wins before any hide rule.

## Declarative rule model

`universal.filter` is generated by compiling the `rules` array in `config.json`. This is the
authoritative description of how the filter is produced (v0.2+). There is no separate
leveling/endgame file — a single generated `universal.filter` handles all area levels because
the rules themselves carry `areaLevel` conditions.

### Rules array

`rules` is an **ordered array** of rule objects; the first matching rule wins
(first-match-wins, identical to how the filter engine evaluates blocks). Each rule compiles to
exactly one `Show` or `Hide` block — the disposition is determined by the `action` field of the
referenced style. A rule with no `when` is a catch-all that matches everything.

```json
{ "name": "band <=5: all armour any rarity", "style": "unchanged",
  "when": { "areaLevel": { "max": 5 }, "class": ["Body Armours", "Helmets", ...] } }
```

### `when` condition vocabulary

All fields are optional. All present conditions must match (AND semantics within a rule).

| Field | JSON type | Emitted filter condition |
|---|---|---|
| `class` | `["ClassName", ...]` | `Class "ClassName" "ClassName" ...` (bare, match-any) |
| `rarity` | compact string | `Rarity == Rare` / `Rarity >= Magic` / `Rarity <= Rare` (bare value = `==`) |
| `areaLevel` | `{ "min": N, "max": N }` | `AreaLevel >= N` and/or `AreaLevel <= N` |
| `baseArmour` | `{ "eq": N }` / `{ "min": N }` | `BaseArmour N` / `BaseArmour >= N` |
| `baseEvasion` | `{ "eq": N }` / `{ "min": N }` | `BaseEvasion N` / `BaseEvasion >= N` |
| `baseEnergyShield` | `{ "eq": N }` / `{ "min": N }` | `BaseEnergyShield N` / `BaseEnergyShield >= N` |
| `quality` | `{ "min": N }` | `Quality >= N` |
| `sockets` | `{ "min": N }` | `Sockets >= N` |
| `baseType` | `["Base Name", ...]` | `BaseType == "Base Name" "Base Name" ...` |
| `unidentifiedTier` | `{ "min": N }` | `UnidentifiedItemTier >= N` |

Note: `min: 1` on a defence attribute emits `> 0` (e.g. `BaseEnergyShield > 0`), matching the
oracle filter syntax. The `max` field on `NumericCondition` emits `<= max` (available for
future use).

> Confirmed working in-game (author-tested, v0.2): PoE2 honors
> `BaseArmour`/`BaseEvasion`/`BaseEnergyShield` as base-defence filters and
> `UnidentifiedItemTier` as a threshold condition. (The recycle `Sockets>=1 & Quality>=1`
> threshold remains a tuning knob, not a correctness question.)

### Named styles

`styles.json` (or `styles-default.json` when it is absent) defines all named styles; the overlay's style picker lists them in file order.
`S`, `A`, `B`, `C`, `D`, `E`, `F`, `unchanged` and `hidden` are required; any other style is
optional. An unknown style in an enabled rule, a missing required style, a duplicate name, a name
differing only by case from a required one, or a bad field (unknown colour, unsafe custom sound
name) is a load-time error naming the problem. A style's `action` decides Show/Hide for the rule
using it, and Hide styles may set any visual (shown on Alt-reveal). Besides a value, `minimap`,
`beam` and `sound` take `"None"`, the filter's cancel value (`MinimapIcon -1`, `PlayEffect None`,
`PlayAlertSound None`: no icon / beam / sound even when an earlier `Continue` rule set one); a
built-in `sound` may set `"positional": true` (`PlayAlertSoundPositional`), and
`"enableDropSound": true` emits `EnableDropSound` (at most one of it and `disableDropSound`).
`generate`, the economy section and the styled-rule rewrite all emit them through the one action
emitter; no shipped style uses them. Current styles:

| Name | Action | What it does |
|---|---|---|
| `S` | Show | Pink text/border, pale `248 248 242` bg, font 37, Pink beam, size-0 Pink Star minimap, alert 6 / 300, EnableDropSound |
| `A` | Show | Purple text/border, pale bg, font 37, Purple beam, size-1 Purple Star minimap, alert 6 / 300, EnableDropSound |
| `B` | Show | Red text/border, Dracula bg, font 37, Red beam, Red Star minimap, alert 2 / 200, EnableDropSound |
| `C` | Show | Cyan text/border, Dracula bg, no font override, Yellow beam, Cyan Star minimap, alert 2 / 200 |
| `D` | Show | Green text/border, Dracula bg, no font override, White beam, Green Star minimap |
| `E` | Show | Yellow text/border, Dracula bg, font 23 |
| `F` | Show | White text/border, Dracula bg, font 20 (de-emphasized) |
| `unchanged` | Show | No color/font/minimap/beam overrides — native rarity color and game defaults |
| `hidden` | Hide | No color overrides, font 21, DisableDropSound — Alt-reveals as de-emphasized |

No custom styles ship. `Recycle`, `kite` (a locator style: native rarity colour, Orange beam, Orange Kite minimap, alert sound) and `exceptional` (the exceptional-gear highlight) are the author's own custom styles, still referenced by name from `config.json`, so `generate` needs a live `styles.json` that holds them (the frozen set is `tests/fixtures/seed-styles.json`).

### `classGroups`

`classGroups` is a documentation-only alias map (e.g. `"buildWeapons": ["Staves","Wands","Foci"]`).
It is **not consumed by the compiler** — rules inline their full class lists in `when.class`.
Use `classGroups` as a reference when authoring new rules to keep class lists consistent.

### Fallthrough

Any item not matched by a rule is left to the game default (uniques, jewels, gems, quest
items, waystones, trials, etc.). See the `"fallthrough"` field in `config.json` for the note.

## Authoring conventions for this repo
- New filters derive from the curated item lists in `reference-old-filters/end_endgame.filter`
  (membership reshuffled per league), restyled into the tier system above.
- **Ignore the trailing config block** that poe2filter.com appended to the old filters
  (`### Filter Configuration` / `# [[Config Start]]` … commented JSON). Do not carry it into
  new filters — `universal.filter` already has it stripped.
- Order blocks **most specific → most general**; tier F is the catch-all.
- Keep block comments descriptive: `Show # What it matches (kebab-slug)`.
- After editing a `.filter`, copy it into the game's config folder (path above) to take effect.
  We will not auto-copy unless asked.
