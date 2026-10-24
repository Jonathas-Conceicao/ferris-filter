# FerrisFilter

## What This Is

FerrisFilter is a Rust tool for Path of Exile 2 loot filters. Its CLI compiles a human-editable configuration into a working `.filter` and fetches live item prices from poe.ninja so highlight tiers are driven by real economy values. Its overlay, `ff`, is drawn in Path of Exile 2's own UI style beside the running game and edits any working PoE2 filter in place — its own output or a FilterBlade / NeverSink / hand-written one — losslessly, with named styles, tier cutoffs and a regenerable economy section configurable from the overlay. Since v0.7 it is a public release: a per-user Windows installer and a portable zip published by a tag-triggered GitHub workflow, with an economy visibility (strictness) preset, a league picker and fallback prices for players other than the author.

## Core Value

`generate` turns the config into a correct, deterministic PoE2 loot filter — the same translation currently done by hand, done by the tool instead. If nothing else works, this must.

## Next Milestone

Not defined yet; run `/gsd-new-milestone`. Candidates: the Deferred list below and the ROADMAP
backlog (999.15 sound preview, 999.16 move rules far at once, 999.21 / 999.22 crash and run logs,
999.28 economy item review, 999.18 NVIDIA overlay).

<details>
<summary>✅ v0.7.2 Small UI Improvements — SHIPPED 2026-10-24</summary>

**Goal:** Polish the overlay with game-art controls and drag reordering, make styles complete
(alpha) and able to adopt a filter's existing styling, and make economy names and network use
friendlier.

Delivered: alpha on style colours (picker slider, translucent previews, written by `generate`, the
economy section and styled rules; 3-value `styles.json` files still load); Auto Detect on CONFIG
turning a filter's existing styling into tags and `Custom Style N` styles as pending edits; economy
rule names with their tier's price range; overlay requests to poe.ninja and GitHub at most once an
hour per kind (`ff-state.json`) and a separate CLI user agent; a configurable show / hide hotkey in
a game-style keybind row; Social-panel section headers and search fields with the game's magnifier
and clear X; drag-and-drop reordering of rules and custom styles replacing the arrows; version
0.7.2.

**Audit:** `tech_debt` — 38/38 requirements, 8/8 phases, 8/8 integration, 4/4 flows; debt
accepted. Archives: [v0.7.2-ROADMAP.md](milestones/v0.7.2-ROADMAP.md) ·
[v0.7.2-REQUIREMENTS.md](milestones/v0.7.2-REQUIREMENTS.md) ·
[v0.7.2-MILESTONE-AUDIT.md](milestones/v0.7.2-MILESTONE-AUDIT.md)

</details>

<details>
<summary>✅ v0.7 Initial Public Release and Strictness Selector — SHIPPED 2026-10-18</summary>

**Goal:** Ship FerrisFilter as a public tool anyone can download, install and use, and add the
first features aimed at players other than the author: a strictness preset, a league picker and
fallback prices.

Delivered: one per-user folder (`%APPDATA%\FerrisFilter`) with every default built into the exe;
the game folder from the OS Documents folder with a Browse picker and the game's reload button;
the built-in styles cut to the nine required ones, an Economy Visibility row of seven levels and
Reset to defaults for styles and tiers; a poe.ninja league picker and a Live / Fallback price
source; a tag-triggered release workflow publishing a per-user Inno Setup installer, a portable
zip and checksums (dry-run as `v0.7.0-rc.1`); Check for updates on the About tab; version 0.7.0
and a CHANGELOG restarted at 0.7.0.

**Audit:** none run; closed on the author's UAT. Archives:
[v0.7-ROADMAP.md](milestones/v0.7-ROADMAP.md) ·
[v0.7-REQUIREMENTS.md](milestones/v0.7-REQUIREMENTS.md)

</details>

<details>
<summary>✅ v0.6 New PoE2-based Tool UI — SHIPPED 2026-10-17</summary>

**Goal:** `ff` rebuilt in Path of Exile 2's own Options-window style on the shared skin proven by
the `ff-proto` prototype, every v0.5 editor capability kept, and the prototype retired.

Delivered: one Cargo workspace with `ff-art` runnable from the root; the skinned, left-docked
shell with four tabs, one-line status bar and skinned popups; RULES on the skin with rules added at
the top; `# ff:section` sections with the economy section as the locked last one (browsable
read-only, `# ff:name` / `# ff:style` headers on every generated block); the CONFIG style table and
line-by-line style editor (shared with Custom rule styling); ECONOMY and ABOUT tabs; `ff-proto`,
the old theme and Cinzel removed; version 0.6.0.

**Audit:** none run; closed on the author's in-game UAT (open HUMAN-UAT / VERIFICATION files
deferred). Archives: [v0.6-ROADMAP.md](milestones/v0.6-ROADMAP.md) ·
[v0.6-REQUIREMENTS.md](milestones/v0.6-REQUIREMENTS.md)

</details>

<details>
<summary>✅ v0.5 Generic Filter Editor & Initial App Configuration — SHIPPED 2026-10-15</summary>

**Goal:** FerrisFilter can open, edit and save any working PoE2 filter, not just its own output,
with the style palette and economy tier cutoffs configurable from the overlay.

Delivered: lossless parser, `styles.json` with stable ids and committed defaults, the filter file as
the store with a fenced economy section and guarded writes, a rule list + full-grammar editor, a
CONFIG tab with one Apply, single-instance `ff`, in-game reload after Apply.

**Audit:** `tech_debt` — 73/73 requirements, 13/13 phases, no gaps. Archives:
[v0.5-ROADMAP.md](milestones/v0.5-ROADMAP.md) · [v0.5-REQUIREMENTS.md](milestones/v0.5-REQUIREMENTS.md) · [v0.5-MILESTONE-AUDIT.md](milestones/v0.5-MILESTONE-AUDIT.md)

</details>

<details>
<summary>✅ v0.4 Overlay Filter Editor Rework — SHIPPED 2026-09-17</summary>

See Current State below and [v0.4-ROADMAP.md](milestones/v0.4-ROADMAP.md).

</details>

<details>
<summary>✅ v0.3 Economy Items & Exceptional Gear — SHIPPED 2026-06-27</summary>

**Goal:** Track three new/broken economy categories with correct styling, and highlight "exceptional" gear.

**Delivered features:**
- **Verisium** — new synced category (poe.ninja exchange `type=Verisium`), emitted with the regular tiered currency style.
- **Uncut Gems fix** — emitted `BaseType` includes the level (`Uncut Skill Gem (Level N)`) so the rule actually matches; drop the `minLevel: 18` floor; price-tiered with a fixed background `80 70 55`.
- **Lineage Support Gems** — new synced category (`type=LineageSupportGems`), price-tiered with a fixed background `10 150 150`.
- **Exceptional gear highlight** — gear with extra sockets OR extra quality and NOT corrupted (filter mechanism unknown → resolved by research).

**Key context:** League is "Runes of Aldur" (Fate of the Vaal ended). Gem styling = tier color (border+text, by value) + a per-category background override — the same mechanism as priced uniques' bronze background, **not** a new named style (so the config-driven-styles backlog item was not pulled in). Tablets remain in the backlog (out of scope for v0.3).

**Audit:** `passed` — 9/9 requirements, 3/3 phases, 8/8 integration, 5/5 E2E flows. Archives:
[v0.3-ROADMAP.md](milestones/v0.3-ROADMAP.md) · [v0.3-REQUIREMENTS.md](milestones/v0.3-REQUIREMENTS.md) · [v0.3-MILESTONE-AUDIT.md](milestones/v0.3-MILESTONE-AUDIT.md)

</details>

## Current State

**Shipped:** v0.7.2 Small UI Improvements (2026-10-24). Style colours carry alpha (3-value
`styles.json` files still load; saved files are 4-value and unreadable by older releases). CONFIG
gains Auto Detect, which adopts a filter's existing styling as style tags and `Custom Style N`
styles, and an Overlay section whose keybind row sets the show / hide hotkey (applied live,
registered before anything is written). Rules and custom styles are reordered by dragging a handle;
the up / down arrows remain only on section headers, which now use the game's Social-panel art, as
the search fields use its magnifier and clear X. Economy rule names state their price range, the
overlay throttles its poe.ninja and GitHub requests to once an hour per kind (`ff-state.json`), and
the CLI identifies itself as `FerrisFilter-CLI`. `generate` output and the seed fixture are
unchanged. ~92,100 LOC Rust; 1921 lib tests plus integration suites green; package version 0.7.2.

**Prior:** v0.7 Initial Public Release and Strictness Selector (2026-10-18), the first public
release. `ff` installs per user from `FerrisFilter-0.7.0-setup.exe` (or runs from the portable
zip), keeps styles, tiers, settings and prices in `%APPDATA%\FerrisFilter` (`FERRISFILTER_HOME`
overrides) and needs no files beside the exe. Config adds the game folder picker and the Economy
Visibility row (Flat Broke … Div Farmer, Custom for hand edits) plus Reset to defaults; Economy adds
the league picker and Live / Fallback prices; About checks GitHub for a newer release and hands off
to its verified installer. Only the nine required built-in styles ship. Releases come from
`release.yml` on `vX.Y.Z` tags. ~82,800 LOC Rust; package version 0.7.0.

**Prior:** v0.6 New PoE2-based Tool UI (2026-10-17) — `ff` is now drawn with Path of Exile 2's
own UI art (extracted locally by the `ff-art` dev tool; GGG's art, credited, replaced if they
object): a 660pt window docked to the game's monitor, RULES / CONFIG / ECONOMY / ABOUT tabs, one
status line beside Revert / Reload / Apply. Filters can be split into collapsible `# ff:section`
sections; the economy section is the locked last one, browsable read-only and shown as Apply will
write it (pending styles and tiers included). Styles are edited line by line in a table (with a
skinned colour picker, off / cancel / value groups, folder sounds, Positional), and the same editor
styles Custom rules. `generate` / `sync` output and the committed fixtures are unchanged.
~70,900 LOC Rust; 1392 lib tests plus integration suites green; package version 0.6.0.

**Prior:** v0.5 Generic Filter Editor & Initial App Configuration (2026-10-15) — `ff` now edits
any working PoE2 filter, not just FerrisFilter's own output. A lossless parser round-trips foreign
filters byte-for-byte; `dynamic.json` is gone and the game-folder `.filter` is the store, with a
fenced economy section regenerated on Apply and guarded writes (changed-on-disk refusal, atomic
write, one-time `.ffbak`). Every condition and action is editable; named styles live in
`styles.json` (stable ids, `# ff:style` links rewritten on Apply) with committed
`styles-default.json` / `tiers-default.json` defaults; a CONFIG tab edits styles, tier cutoffs and
the target filter under one all-or-none Apply. `ff` runs as a single instance, launches from
Explorer (app root resolved from the exe), and sends `/reloaditemfilter` after Apply. ~50,244 LOC
Rust; 1142 tests green. This closes v0.4's "launch by double-click" gap.

**Prior:** v0.4 Overlay Filter Editor Rework (2026-09-17) — `ff`, a GUI overlay that draws
over a running Path of Exile 2 and edits filter rules in place. A rule expands into controls for
style, classes, base types, rarity, defences, sockets, quality, corrupted and unidentified tier;
Apply is the single writer, compiling `dynamic.json` into `dynamic.filter` straight into the game
folder, with Revert to discard unsaved edits and a SYNC ECONOMY button that refreshes prices on a
worker thread. Underneath it: `compile::` as a real library API (Phase 11) and a complete headless
`dynamic.json → dynamic.filter` pipeline reachable from the CLI alone (Phase 12). ~16,093 LOC Rust;
368 tests green, build warning-free.

**Prior:** v0.3 Economy Items & Exceptional Gear (2026-06-27) — `sync` now tracks Verisium and
Lineage Support Gems, Uncut Gems finally match in-game (level kept in `BaseType`, per-subtype plural
`Class`, no `minLevel` floor), priced gem blocks carry a per-category background over the value-tier
text/border, and the rule vocabulary gained `corrupted` — driving three exceptional-gear rules
(sockets >= 3 / >= 2, quality >= 24, all uncorrupted) in the new pink `exceptional` style.
`generate` + `sync` + `install` all live. ~7,534 LOC Rust; 133 lib tests + integration suite green.

**Prior:** v0.2 Declarative Rule Model (2026-06-12) replaced the generator's hardcoded gear/flask
band logic with a declarative `rules` schema, making the full in-game filter reproducible from config.

**Since v0.7.0:** v0.7.1 (2026-10-19) links the C runtime statically, so `ff.exe` no longer
needs the Visual C++ Redistributable.

**Next milestone:** not defined yet; candidates under Next Milestone above.

**Open at close (v0.7.2):** no blockers. Accepted debt from the audit: economy name formatting
edge cases (`-0`, cutoffs below 0.005, mixed-unit tier files), Phase 46 review Info items, hotkey
capture limits (numpad digits, AltGr, chords Windows refuses), UI review follow-ups (search hint
truncation, refresh plate height, drag shadow ground) and 126 clippy warnings on lines older than
the milestone. 30 open GSD artifacts acknowledged (STATE.md Deferred Items): the phase 44-49
VERIFICATION files (author-verified) and older UAT / verification leftovers. The live Check for
updates install run still waits for a release newer than the running one.

## Requirements

### Validated

- [x] **CLI shell** — `ferrisfilter` crate builds; `generate` and `sync` subcommands are visible with `--help`; `generate --output` documented; unknown commands exit non-zero. *(Validated in Phase 1: Foundation)*
- [x] **Static filter generation** — `ferrisfilter generate` deterministically compiles `config.json` + `currency.json` into `universal.filter`, reproducing the established block structure (model-path tags, AreaLevel bands, flask/charm ladder, recycle, currency tiers), locked by an insta snapshot + artifact-guard test; uniques/gems/jewels/etc. fall through. *(Validated in Phase 2: Static Generate)*
- [x] **Price-driven tiering** — `generate` derives currency/category (and, dormant, unique) tiers from `prices.json` via cross-rate normalization + `tiers.json` cutoffs, aborts on null/zero reference rates (TIER-04), and degrades gracefully without prices.json (GEN-05); `currency.json` is dropped as a generate input (reference/seed only). Re-baseline limited to the 3 reference orbs, guardrail-tested. *(Validated in Phase 3: Pricing Layer)*
- [x] **Input models** — `config.json` (renamed from `filter-model.json`), `tiers.json`, and `prices.json` are modeled in serde, round-trip, and validate with actionable field-path + expected-type errors; `tiers.json` completeness/ordering checked on load. *(Validated in Phase 1: Foundation)*
- [x] **Currency data extraction** — curated currency base-lists pulled out of `universal.filter` into a tracked `currency.json` and modeled. *(Validated in Phase 1: Foundation)*
- [x] **Live price sync** — `ferrisfilter sync` fetches PoE2 economy data from poe.ninja (currency, uniques, tracked drops; uncut gems ≥ 18), resolves the league per `--standard`/`--hc` (none = softcore trade), records league + timestamp, and writes `prices.json` atomically (failure-safe; never corrupts an existing file). *(Validated in Phase 4: Sync — v0.1)*
- [x] **Install** — `ferrisfilter install` copies a generated filter into the game config folder (`--dest`/`--input` overrides), replacing the old `install.bat`. *(Shipped in v0.1)*
- [x] **Early-league fallback** — committed `prices-fallback.json` + `tiers-fallback.json` (exalt-denominated) drive `generate` when `prices.json` is absent/empty (with a warning) or `--early-game` is passed; cross-rates are derived from the anchor orbs. *(Shipped in v0.1)*
- [x] **Declarative rule engine** — `config.json` `rules` array (ordered, first-match-wins) compiled by `emit_rules`; the hardcoded gear/armour/otherGear/flask special-casing is gone. *(Validated in Phase 6 — v0.2)*
- [x] **Condition vocabulary** — rules combine class list, compact rarity comparator, areaLevel min/max, base-defence/Quality/Sockets, BaseType list, and UnidentifiedItemTier; serde-validated with field-path errors. *(Validated in Phase 5 — v0.2)*
- [x] **The 7 in-game edits expressible as config** — weapon/armour caps, flask/charm Magic+Rare + rarity-bounded Hide, Normal charms ≥78, jewellery hide, unidentified-tier highlight (new `unidentified` style). *(Validated in Phase 7 — v0.2)*
- [x] **Config migration + semantic parity** — `config.json` migrated to rules; `generate --early-game` reproduces the in-game filter with zero non-reorder policy differences; `seed-universal.filter` re-baselined; price-driven sections untouched. *(Validated in Phase 6/7 — v0.2)*
- [x] **Verisium sync + tiering** — `sync` fetches Verisium from poe.ninja and `generate` tiers it as regular stackable currency, no special background. *(Validated in Phase 8 — ECON-01, v0.3)*
- [x] **Uncut Gems base-type fix** — emitted base type carries the level (`Uncut Skill Gem (Level N)`), per-subtype plural `Class`; `minLevel` floor removed; subtypes merge into one multi-class block. *(Validated in Phase 8 — ECON-03, v0.3)*
- [x] **Lineage Support Gems sync + tiering** — new synced category (`Class "Support Gems"`, spike-confirmed), price-tiered. *(Validated in Phase 8 — ECON-02, v0.3)*
- [x] **Gem per-category background styling** — uncut (`80 70 55`) / lineage (`10 150 150`) gems show value-tier text+border over a fixed per-category background, via the priced-unique clone-null-append pattern. *(Validated in Phase 9 — STYLE-01/02, v0.3)*
- [x] **`corrupted` rule condition** — `when.corrupted` emits the bare boolean `Corrupted True`/`Corrupted False`, no operator. *(Validated in Phase 10 — EXC-01, v0.3)*
- [x] **Exceptional gear highlight** — three rules ahead of recycle surface uncorrupted gear with extra sockets (>= 3 2h/body, >= 2 1h) or `Quality >= 24`, in the dedicated pink `exceptional` style. *(Validated in Phase 10 — EXC-02/03/04, v0.3)*

- [x] **Lossless filter parsing and editing** — any PoE2 filter loads and saves byte-identical; every condition and action editable; unknown/PoE1-only lines kept as raw rows. *(Validated in Phases 16, 19-20, 24-27 — PARSE, EDIT, v0.5)*
- [x] **Filter file as the store** — `dynamic.json` removed; fenced, regenerable economy section; guarded atomic writes with a one-time `.ffbak`. *(Validated in Phase 18 — STORE, ECON, v0.5)*
- [x] **Config-driven styles** — `styles.json` with stable ids, resolved by name in `generate` and by id in `# ff:style` tags; committed `-default.json` fallbacks. *(Validated in Phases 17, 26, 28 — STYLE, DEF, v0.5)*
- [x] **Overlay CONFIG tab** — filter choice, styles and tier cutoffs under one all-or-none Apply. *(Validated in Phases 21, 25 — CONF, v0.5)*
- [x] **Single-instance `ff` and in-game reload after Apply.** *(Validated in Phases 22-23 — INST-01, RELOAD-01, v0.5)*
- [x] **`ff` launchable from Explorer** — app root resolved via `FERRISFILTER_HOME` or by walking up from the exe (closes the v0.4 known gap). *(Validated in Phase 18 — STORE-05, v0.5)*
- [x] **Game-styled overlay** — `ff` drawn with the PoE2 skin: docked shell, four tabs, one-line status, skinned popups, every v0.5 capability kept. *(Validated in Phases 30-31 — SHELL, RULES, v0.6)*
- [x] **Rule sections** — `# ff:section` banners as collapsible sections; the economy section is the locked last one, browsable read-only. *(Validated in Phase 32 and quick 2rf / 7lp — SECT, v0.6)*
- [x] **Style table and line-by-line style editor**, shared with Custom rule styling. *(Validated in Phase 33 and quick qi7 / 0wl — STY, v0.6)*
- [x] **ECONOMY and ABOUT tabs; prototype retired; one workspace with `ff-art` from the root.** *(Validated in Phases 29, 34-35 — WS, ECO, ABOUT, CLEAN, v0.6)*

- [x] **Per-user folder and built-in defaults** — one `%APPDATA%\FerrisFilter` folder, every shipped data file compiled into the exe, tests and screenshot mode isolated from the real folders. *(Validated in Phase 36 — LOC, v0.7)*
- [x] **Game folder picker** — Documents-based default, Browse on the Config tab, the game's reload button re-listing filters and sounds. *(Validated in Phase 37 — GAME, v0.7)*
- [x] **Economy Visibility (strictness) and Reset to defaults**; only the nine required built-in styles ship. *(Validated in Phase 38 — BSTY, STRICT, RESET, FIX, v0.7)*
- [x] **League picker and Live / Fallback price source.** *(Validated in Phase 39 — LEAGUE, v0.7)*
- [x] **Public release** — tag-triggered release workflow, per-user installer, portable zip, checksums, one public repo name. *(Validated in Phase 40 — REL, INST, PUB, v0.7)*
- [x] **Check for updates** on the About tab. *(Validated in Phase 41 — UPD, v0.7)*

- [x] **Style alpha** — style colours carry alpha in the picker, previews and every emitter; 3-value files still load, fixtures unchanged. *(Validated in Phase 43 — ALPHA, v0.7.2)*
- [x] **Auto Detect styles** — a filter's existing styling becomes style tags and `Custom Style N` styles, as pending edits. *(Validated in Phase 44 — DETECT, v0.7.2)*
- [x] **Economy names with price ranges** (`S tier Currency >= 30 chaos`), previewed with pending tiers. *(Validated in Phase 45 — ENAME, v0.7.2)*
- [x] **Request throttling and CLI user agent** — overlay requests at most once an hour per kind, persisted in `ff-state.json`; the CLI unthrottled with `FerrisFilter-CLI/<ver>`. *(Validated in Phase 46 — THR, v0.7.2)*
- [x] **Configurable show / hide hotkey** — keybind row on CONFIG, live swap on Apply, reserved / in-use chords refused, Reset to defaults. *(Validated in Phase 47 — KEY, v0.7.2)*
- [x] **Game-art widgets** — Social-panel section headers, magnifier and clear X in search fields, art painted by its visible box. *(Validated in Phase 48 — ART, v0.7.2)*
- [x] **Drag-and-drop reorder** of rules and custom styles, replacing the arrows (section headers keep theirs). *(Validated in Phase 49 — DND, v0.7.2)*

### Active

None until the next milestone defines them.

### Deferred (next-milestone candidates)

- [ ] **v0.7.2 audit debt:** economy name formatting edge cases; hotkey capture on non-US
  punctuation, numpad and AltGr; the drag shadow's solid ground; the RULES search hint truncation.
- [ ] Move rules far at once (move to top / bottom, into a named section) (999.16); sound preview
  in the style editor (999.15); crash log and optional run log (999.21, 999.22).

- [ ] **v0.5 tech debt** (audit W1-W2): CLI `generate`/`install` resolve against the working
  directory while `apply`/`sync`/`ff` use the FerrisFilter folder; CLI `apply` does not rewrite
  styled rules; class picker disabled when `config.json`/`styles.json` is invalid.
- [ ] **`generate` without a live `styles.json`** fails on the author's custom styles that
  `config.json` still names (`Recycle`, `kite`, `exceptional`).
- [ ] A SIGTERM/Ctrl+C console-control handler so `ff` can be stopped from a terminal.
- [ ] Code signing for the installer (SmartScreen warning today).

- [ ] **BaseType-matched, economy-tiered priced category** (e.g. Tablets): support a priced category that emits `BaseType ==` with **no `Class` line** (tablets have no usable PoE2 filter class). Needs a `Tablets` type in `mapping.json` (spike: confirm poe.ninja serves tablet prices), pricer/emitter support for class-less category blocks, and corrected base names (`Ritual Tablet`/`Abyss Tablet`/…, no "Precursor"). Tablets currently fall through to game default after the v0.2 removal of the invalid `Class "Tablet"` rows.
- [ ] Generate QoL: `--dry-run`, `--diff`, `-v/--verbose` rule logging, stale-`prices.json` warning (GENX-01..04)
- [ ] Sync extensions: multiple build profiles (`--config builds/<name>.json`); alternate price source fallback, e.g. poe2scout (SYNX-01..02)
- [ ] Lenient-parse guard for Standard-league unique endpoints (different/empty shape); optional no-flag `sync` → `--standard` hint when no challenge league is active

### Out of Scope

- Hand-maintaining currency/tier base-lists — replaced by `sync` + price-driven tiering
- Filter rules for jewels, quest items, waystones, trials — these fall through to game default (established design philosophy). *(Jewellery hide and unidentified-tier highlight moved IN-scope in v0.2 — the in-game filter already curates them. **Uncut and Lineage Support gems moved IN-scope in v0.3** as priced, economy-tiered categories; all other gems still fall through.)*
- Blanket unique styling — only price-tracked uniques get rules; the rest fall through
- ~~GUI / web interface — CLI only for now~~ — **invalidated in v0.4**: `ff` is a GUI overlay. The
  reasoning held right up until the CLI's real limitation showed itself — a filter cannot be edited
  while the game is running without leaving the game. A web interface remains out of scope
- ~~Auto-installing into the game folder~~ — **shipped in v0.1** as the `install` subcommand (copy-only; `install.bat` removed)
- Linux / macOS overlay builds — Windows is the shipped target; path code stays cross-platform
- Update checks at start-up or on a timer, and in-place exe swaps — on demand only, through the installer
- The CLIs in the installer — `ff-cli` / `ferrisfilter` are dev tools
- Keyboard reordering of rules and styles — never a required feature (author, v0.7.2)
- Drag handles on section headers — header arrows stay; they move only the divider
- A throttle bypass button — the overlay's once-an-hour limit has no override in the UI
- Disabling the show / hide hotkey — a chord is always bound
- The `moved across Continue` flag — removed in v0.7.2; order is up to the user

## Context

- **Prior art lives in this repo.** `filter-model.json` (the policy model) and `universal.filter` (the hand-generated output) already exist and encode the exact logic `generate` must reproduce. `CLAUDE.md` documents the filter grammar, the Dracula style-tier system, and the design philosophy (hide unwanted, style highlights, fall through the rest). These are the executable spec for the generator.
- **The model was built and refined across a long session:** `styles` (S–F + `unchanged`/`hidden`), `build.weapons` (Staves/Wands/Foci) and `build.armour` (Body Armours/Helmets/Gloves/Boots) with defence-type taper, `areaLevelBands` (1-5 / 6-25 / 26-45 / 46-65 / 66+), and `alwaysShow` (currency tiers, recycle). Weapons are build-only across all bands; rares of any class show ≤45 via the otherGear rule.
- **Currency tiers are currently curated base-lists inside the filter.** `sync` + `generate` will replace that curation with price-driven tiering from poe.ninja.
- **Platform:** Windows 11, PowerShell primary shell. Author prefers Rust.
- **poe.ninja** is the price source; its PoE2 API surface was resolved via the Phase 4 spike (exchange + stash/item overview endpoints, plural `type` values, league index-state resolution) and is now consumed by `sync`.
- **Shipped state (v0.1, 2026-05-28):** ~4,970 LOC Rust across 4 phases / 14 plans / 127 commits. `generate` + `sync` + `install` all live; full test suite green (~113 tests). Live softcore no-flag `sync` happy path awaits a new active league (fails safe meanwhile).
- **Shipped state (v0.3, 2026-06-27):** ~7,534 LOC Rust; 133 lib tests plus the CLI/sync integration suites green. `mapping.json` is at schema v2 (`gemFilters` array with per-category `backgroundOverride`, `Verisium` overview type). The generated filter is 77 `Show` / 3 `Hide` blocks. Live league is "Runes of Aldur"; post-close `sync` fixes landed for the Expedition logbook/Sagas mixed-class feed and PoE1-only `UniqueMaps`.

## Constraints

- **Tech stack**: Rust — author preference; crate named `FerrisFilter`.
- **Output format**: Must emit valid PoE2 filter grammar (same grammar as PoE1, different valid classes/base types — verify against poe2db.tw / live game).
- **Determinism**: `generate` must be deterministic — same inputs always produce the same `universal.filter`.
- **Dependency**: `sync` depends on poe.ninja's PoE2 economy API (external, may change, per-league).
- **Reference unit**: All tiering math normalizes to Chaos Orb value via cross-rates from the three reference currencies.

## Key Decisions

| Decision | Rationale | Outcome |
|----------|-----------|---------|
| Rust CLI crate named `FerrisFilter` | Author preference; deterministic compiled tool | ✓ Good (v0.1) |
| Two subcommands: `generate` + `sync` | Separate filter compilation from price fetching | ✓ Good (v0.1) |
| `sync` fetches raw prices only; `generate` does all conversion + tiering | Keep network/IO and pure logic separate; deterministic generate | ✓ Good (v0.1) |
| Separate files: `config.json` (human) + `tiers.json` (cutoffs) + `prices.json` (synced) | Keep hand-edited policy apart from machine-generated data | ✓ Good (v0.1) |
| Chaos as the reference unit; cross-rates from first 3 `prices.json` entries | Single common unit for threshold comparisons | ✓ Good (v0.1) |
| poe.ninja as price source | Author choice | ✓ Good (v0.1) |
| Price-tier valuable uniques; untracked uniques fall through | Only relevant-value uniques deserve rules | ✓ Good (v0.1) |
| Both `generate` and `sync` in v1 | Author wants the full loop usable from the start | ✓ Good (v0.1) |
| Milestone git workflow (branch per milestone) | Clean, reviewable history per milestone | ✓ Good (v0.1) |
| Declarative `rules` array replaces hardcoded emitter bands | One general model; any class/level/rarity/condition policy expressible in config | ✓ Good (v0.2) |
| Compact rarity string (`">=Magic"`) over an object | Author preference; terser config authoring | ✓ Good (v0.2) |
| Semantic (order/comment-insensitive) parity, not byte-for-byte | Block reordering is irrelevant in PoE2; lets the rule model reshape output freely | ✓ Good (v0.2) |
| Price-driven currency/unique sections stay pricer-driven (not rules) | Keep the v0.1 economy pipeline untouched; rules cover policy only | ✓ Good (v0.2) |
| Tablets have no usable filter `Class` → removed broken rows | `Class "Tablet"` failed to load in-game; BaseType-only category deferred | ⚠️ Revisit (v0.2 — deferred re-add) |
| Each uncut gem *level* is a distinct base type; drop `strip_gem_level`/`gem_best` dedup | Level-stripped `BaseType` never matched in-game — that dedup was the bug | ✓ Good (v0.3) |
| Gem backgrounds via a data-driven `background_override`, not a new named style | Reuses the priced-unique clone-null-append path; kept the 999.1 style refactor out of scope | ✓ Good (v0.3) |
| `corrupted` emits a bare boolean (`Corrupted False`), no operator | PoE2 boolean conditions take no comparator | ✓ Good (v0.3) |
| OR-semantics as separate rules (sockets vs quality) | Filter blocks are AND-only; one combined rule would demand *both* | ✓ Good (v0.3) |
| `exceptional` kept as its own style, separate from `S` (same pink today) | Lets the exceptional-gear color be retuned independently later | ✓ Good (v0.3) |
| `exceptional` landed as `Option<Style>` rather than a required `Styles` field | Matches the `craftable` pattern so `--config` files without it keep parsing | ✓ Good (v0.3 — adjusted in code review) |
| One consolidated fixture re-baseline at the last emitter-output phase | Avoided re-baselining the oracle three times across Phases 8-10 | ✓ Good (v0.3) |
| Lossless parser; edits patch only the edited lines | A foreign filter must survive load/save byte-for-byte; no "unsupported block" class | ✓ Good (v0.5) |
| The `.filter` file is the store; `dynamic.json` dropped | One source of truth; full regeneration of the dynamic filter given up deliberately | ✓ Good (v0.5) |
| Economy blocks fenced by `# ff:economy-begin/end` markers, regenerated on Apply | Lets the economy refresh inside a hand-edited filter without touching anything else | ✓ Good (v0.5) |
| Styles linked by stable id (`# ff:style <id> <name>`), not name | Custom styles can be renamed without orphaning rules; legacy name tags still resolve | ✓ Good (v0.5) |
| One all-or-none Apply across RULES and CONFIG | Author's second UAT round (UAT2): one action, written all or none, no half-applied state | ✓ Good (v0.5) |
| In-game reload via clipboard paste, no Ctrl+A | Same method as EE2 / Awakened PoE Trade; synthetic Ctrl+A fired another tool's hotkey | ✓ Good (v0.5) |
| Committed `styles-default.json` / `tiers-default.json`; live files git-ignored | Per-user edits from the overlay must not dirty the repo or break `artifact_guard` | ✓ Good (v0.5) |
| Overlay skin built from game art extracted locally by `ff-art` | Looks like the game's own Options window; GGG's art is credited and will be replaced if they object | ✓ Good (v0.6) |
| Sections as three-line `# ff:section` banners; economy as a begin-only banner, always last | Readable in any text editor; one format for user dividers and the economy section | ✓ Good (v0.6) |
| One style editor for CONFIG styles and Custom rule styling | Same controls, same file semantics; no second editor to keep in step | ✓ Good (v0.6) |
| Economy blocks carry `# ff:name` / `# ff:style` only in the game-folder section | Economy rules read like any other rule in `ff`; `generate` output and fixtures stay byte-identical | ✓ Good (v0.6) |
| Package version tracks milestone tags (0.6.0 at v0.6) | Version shown on ABOUT matches the tag | ✓ Good (v0.6) |
| Overlay league pinned in `sync::OVERLAY_LEAGUE` | poe.ninja's `indexed` flag flips between leagues | ✓ Replaced in v0.7 by the league picker (default: first softcore non-Standard league, `indexed` ignored) |
| Milestones are atomic; each ends with a Release Review phase | README and CHANGELOG are read by people and mostly written by the author; bugs made and fixed inside a milestone never ship | ✓ Good (v0.7) |
| One per-user folder, defaults compiled into the exe, no walk-up from the exe | An installed exe needs no files beside it; tests and screenshot mode can't hit real folders | ✓ Good (v0.7) |
| Strictness as a derived view of the S-F visibility (Custom when it matches no level) | No new stored state; hand edits stay the truth | ✓ Good (v0.7) |
| Price-source switch saved at once, unapplied until Apply, not undone by Revert | Same as the league pick; the author kept it at UAT | ✓ Good (v0.7) |
| Release workflow drafts, then auto-publishes once every asset is uploaded; tag must equal `v` + Cargo version | No half-published releases; no manual step | ✓ Good (v0.7, rc dry run) |
| Inno Setup per-user installer, `ff.exe` + `LICENSE.txt` only, fixed AppId | No admin prompt; the CLIs are dev tools | ✓ Good (v0.7) |
| CHANGELOG starts at 0.7.0, the first public release; README/CHANGELOG wrap at 70 chars | Earlier sections described a pre-public tool; they stay in git history | ✓ Good (v0.7) |
| Colours read as 3 or 4 values, always written with 4; no migration flag | Old `styles.json` files load untouched; emitted lines unchanged at alpha 255, fixtures byte-identical | ✓ Good (v0.7.2; a 0.7.2 file is unreadable by older releases, noted in the CHANGELOG) |
| A bare 3-token colour line means the game's alpha for its keyword (240 background, 255 otherwise) | Decoder, Custom editor and Auto Detect agree with what the game draws (author, Round 1) | ✓ Good (v0.7.2) |
| Auto Detect makes only pending edits; action-line comments dropped on rewrite | Same Revert / Reload / Apply model as any edit; comments were not worth a warning (author) | ✓ Good (v0.7.2) |
| Economy names: tiers' own unit, ASCII `>=`, at most 2 decimals | Short, readable names; rounding misstatements accepted after review WR-01 | ⚠️ Revisit (v0.7.2 — `-0` and sub-0.005 cutoffs print oddly) |
| Throttling is overlay-only and success-only, persisted in `ff-state.json`; CLI sends its own user agent | Spare poe.ninja and GitHub from repeated clicks; scripts and dev use stay unthrottled and identifiable | ✓ Good (v0.7.2) |
| Hotkey Apply validates, registers, then writes; an owned chord refuses the whole Apply | Never save a chord that does not work; all or none, like every Apply | ✓ Good (v0.7.2) |
| Shift-only chords allowed; non-US punctuation limitation accepted | Author calls on review WR-05 / WR-02; "at least one modifier" kept simple | — Pending (v0.7.2, revisit on user reports) |
| Game art painted by its visible box (`paint_visible` / `box_uv`); keybind interior keyed out at extract time | No re-extraction of legacy PNGs; egui cannot remove a white interior at draw time | ✓ Good (v0.7.2) |
| Drag reorder replaces the arrows on rules and custom styles; arrow model deleted; no Continue flag | Moving far was painful with arrows (999.16 in part); the flag carried no real value (author, Round 2) | ✓ Good (v0.7.2) |
| Parallel worktree phases for independent work (45-48), merged into the milestone branch | Independent phases finish in one sitting; the main checkout stays on the milestone branch for the author | ✓ Good (v0.7.2; merges needed hand fixes) |

## Git Workflow

`git.branching_strategy = "milestone"` (config.json). Per-milestone flow:

1. **Start of milestone** → work happens on a dedicated branch `milestone/{milestone}-{slug}` (GSD creates it at the milestone's first `execute-phase`).
2. **On `complete-milestone`**, in this exact order:
   1. **Push the milestone branch to remote** (preserves full development history).
   2. **Squash-merge** the branch into `main` with a single clean commit message summarizing the milestone.
   3. **Tag** the resulting merge commit on `main` (annotated `vX.Y.Z` from v0.7.0 on; v0.1 to v0.6 were `vX.Y`), then push the tag. Pushing the tag runs `.github/workflows/release.yml`, which publishes the GitHub Release.

GSD's `complete-milestone` natively does the squash-merge and tag; the branch push and tag-on-main ordering are enforced as this project convention.

## Milestone Close-out

**Milestones are atomic.** What ships is the net difference from the previous release. A bug
that is introduced and fixed inside the same milestone never existed as far as anything outside
`.planning/` is concerned: it is not mentioned in `CHANGELOG.md`, `README.md`, `CLAUDE.md`, code
or doc comments, test names, or the squash-merge commit message on `main`. Only internal planning
files (phase SUMMARY / REVIEW / UAT files, STATE.md) record it. A bug in an already *released*
version that the milestone fixes is changelog material as usual.

**Every milestone ends with a Release Review phase.** `/gsd-new-milestone` adds it as the last
phase of the roadmap, and `/gsd-complete-milestone` does not start until it is complete. Its
steps, in order:

1. **Code cleanup.** Remove duplication added during the milestone (merge near-identical
   helpers, drop dead code and leftover scaffolding) and strip every mention of in-milestone bugs
   from code and doc comments, test names, README.md and CLAUDE.md (e.g. "fixes the regression
   from Phase N", UAT / review ids such as `UAT2-03` or `WR-02`). No behaviour change: the full
   test suite passes before and after.
2. **README screenshots.** Re-take any screenshot in `docs/screenshots/` whose screen changed,
   with the debug `--screenshot` mode under a temp home (see CLAUDE.md, `ff` row), keeping the
   file names and framing.
3. **Version.** Set `version` in the root `Cargo.toml` to the milestone's version (`X.Y.0` for
   the tag `vX.Y.0`) and refresh `Cargo.lock`. `ff-art` keeps its own version.
4. **Changelog draft.** Write the new `## [X.Y.0] - <date>` section of `CHANGELOG.md` from the
   milestone's net changes against the previous tag, folding in anything already under
   `[Unreleased]` (work landed between milestones): a one-line milestone summary, then
   Added / Changed / Removed / Fixed, user-facing and short. Fixed lists only bugs that existed in
   a released version. This is a draft for the author, not the final text.
5. **Author review (the phase's UAT, last step).** The author reviews and, as they see fit,
   rewrites `README.md` and the drafted `CHANGELOG.md` section. Agents change these two files only
   as the author directs. The author's confirmation closes the phase.

During a milestone, agents do not add `CHANGELOG.md` entries; step 4 writes them once. Then
`/gsd-complete-milestone` first checks that the tag it is about to create equals `v` plus the root
`Cargo.toml` version exactly (and stops if not) and that a non-empty `## [X.Y.Z]` `CHANGELOG.md`
section exists (the release workflow fails otherwise). Pre-release tags (`vX.Y.Z-rc.N`) take their
notes from `[Unreleased]` and are published as GitHub pre-releases. It then moves any remaining `[Unreleased]` text into the version
section, and runs the Git Workflow steps above.

## Evolution

This document evolves at phase transitions and milestone boundaries.

**After each phase transition** (via `/gsd-transition`):
1. Requirements invalidated? → Move to Out of Scope with reason
2. Requirements validated? → Move to Validated with phase reference
3. New requirements emerged? → Add to Active
4. Decisions to log? → Add to Key Decisions
5. "What This Is" still accurate? → Update if drifted

**After each milestone** (via `/gsd-complete-milestone`):
1. Full review of all sections
2. Core Value check — still the right priority?
3. Audit Out of Scope — reasons still valid?
4. Update Context with current state

---
*Last updated: 2026-10-24 after the v0.7.2 milestone*
