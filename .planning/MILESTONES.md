# Milestones

## v0.7.2 — Small UI Improvements

**Shipped:** 2026-10-24
**Phases:** 43-50 (8) · **Plans:** 24 (+ the Release Review run inline) · **Commits:** 200 · **Tests:** 1921 lib + integration suites green
**Timeline:** 2026-10-23 → 2026-10-24 · **Code:** ~92,100 lines Rust (+10,819/-1,442 in src/tests/tools)

**Delivered:** overlay polish after the first public release: alpha style colours, Auto Detect
for a filter's existing styling, a configurable show / hide hotkey, drag-and-drop reordering,
game art on section headers and search fields, priced economy names and throttled requests.

**Key accomplishments:**

1. **Style alpha** — text, border and background colours carry alpha (picker slider, translucent
   previews) through `generate`, the economy section and styled rules; 3-value `styles.json`
   files still load, and the fixtures are byte-identical.
2. **Auto Detect** — one click on CONFIG tags every untagged rule with the style it already
   matches and turns the rest into `Custom Style N` styles, as ordinary pending edits.
3. **Configurable hotkey** — a game-style keybind row on CONFIG captures the show / hide chord;
   Apply registers it before writing and swaps it live; reserved and in-use chords are refused.
4. **Drag-and-drop reorder** — rules and custom styles move by the author's drag handle (shadow,
   marker, wheel and edge auto-scroll, Escape cancels), replacing the up / down arrows; section
   headers keep theirs, padlocked when they cannot move.
5. **Game-art widgets** — Social-panel section headers (toggle plate + stone bar), magnifier and
   clear X in the search fields, art painted by its visible box (the refresh buttons are centred).
6. **Economy names and throttling** — economy rules are named with their price range
   (`S tier Currency >= 30 chaos`); the overlay asks poe.ninja and GitHub at most once an hour
   per kind (`ff-state.json`), and the CLI sends its own user agent. Version 0.7.2.

**Audit:** `tech_debt` — 38/38 requirements, 8/8 phases, 8/8 integration, 4/4 flows; debt accepted
(see milestones/v0.7.2-MILESTONE-AUDIT.md). Phases 45-48 ran in parallel worktrees; the author
verified every manual check of phases 44-49 (three skipped on purpose) through two feedback
rounds.

**Known deferred items at close:** 30 (see STATE.md Deferred Items) — the phase 44-49
VERIFICATION files left `human_needed` (author-verified at close) plus the phase 14-41 UAT and
verification leftovers carried from earlier milestones.

## v0.7 — Initial Public Release and Strictness Selector

**Shipped:** 2026-10-18
**Phases:** 36-42 (7) · **Plans:** 45 · **Commits:** 261
**Timeline:** 2026-10-17 → 2026-10-18 · **Code:** ~82,800 lines Rust (+12,860/-957 in src/tests/tools)

**Delivered:** the first public release: FerrisFilter installs per user from a Windows installer
(or a portable zip) published by a tag-triggered GitHub workflow, keeps its files in one per-user
folder, and gains an Economy Visibility preset, a league picker, fallback prices and an update check.

**Key accomplishments:**

1. **Per-user folder, built-in defaults** — styles, tiers, settings and prices in
   `%APPDATA%\FerrisFilter` (`FERRISFILTER_HOME` overrides); every default compiled into the exe;
   tests and screenshot mode can never touch the real folders.
2. **Game folder picker** — default from the OS Documents folder (OneDrive / localized), Browse on
   the Config tab, the game's reload button re-listing filters and sounds.
3. **Economy Visibility and Reset to defaults** — seven exclusive levels over the S-F styles'
   visibility (Custom for hand edits); Reset for styles and tiers; only the nine required built-ins
   ship.
4. **League picker and Live / Fallback prices** — poe.ninja's league list with an ended-league
   warning; Fallback greys Sync and the Tiers editor and prices the economy section from the
   built-in pair.
5. **Release pipeline and installer** — `release.yml` on `vX.Y.Z` tags checks the version and
   CHANGELOG, tests, builds and publishes the per-user Inno Setup installer, zip and checksums;
   dry-run as `v0.7.0-rc.1` (since deleted).
6. **Check for updates** — the About tab finds a newer GitHub release, downloads and hash-checks its
   installer and hands off through the normal Quit path; version 0.7.0, CHANGELOG restarted at 0.7.0.

**Audit:** none run; closed on the author's UAT.

**Known deferred items at close:** 6 (see STATE.md Deferred Items) — mainly the live update-check
run, which needs a release newer than 0.7.0.

## v0.6 — New PoE2-based Tool UI

**Shipped:** 2026-10-17
**Phases:** 29-35 (7) · **Plans:** 30 · **Quick tasks:** 7 · **Commits:** 281 · **Tests:** 1392 lib + integration suites green
**Timeline:** 2026-10-15 → 2026-10-17 · **Code:** ~70,900 lines Rust (+26,255/-5,263 in src/tests/tools)

**Delivered:** `ff` is now a game-styled overlay drawn with Path of Exile 2's own UI art, with
collapsible rule sections, a line-by-line style editor shared by styles and custom rules, and
dedicated Economy and About tabs; the prototype it grew from is retired.

**Key accomplishments:**

1. **One Cargo workspace, game art from the repo root** — `ff` stays the default crate; the
   `ff-art` dev tool (non-default member, `cargo ff-art`) extracts the Options-window art, minimap
   atlas and Fontin fonts read-only from the install.
2. **The skinned overlay shell** — a 660pt window docked to the game's monitor (DPI-correct), four
   stone tabs, a one-line status bar with Revert / Reload / Apply on every tab, skinned popups, and
   every v0.5 behaviour (hotkey, tray, single instance, in-game reload) intact.
3. **RULES on the skin** — fixed two-line rows with painted flag icons and style chips, a two-column
   editor, rules added at the top, and Simple Defences as multi-select base types written as
   `# ff:group` block groups.
4. **Rule sections** — `# ff:section` banners as collapsible, renamable, movable headers; the
   economy section is the locked last section, browsable read-only, with `# ff:name` / `# ff:style`
   headers on every generated block, even before a staged add is applied.
5. **Style table and line-by-line style editor** — aligned Built-in / Custom tables, a skinned
   colour picker, Minimap / Beam / Alert sound / Drop sound groups with off, cancel (`/`) and value,
   a filter-folder sound picker and Positional; the same editor drives Custom rule styling.
6. **Economy and About tabs, prototype retired** — sync, add/remove economy section and the tiers
   editor on ECONOMY; ABOUT with Ferris, version, disclaimer and credits; `ff-proto`, the old theme
   and Cinzel removed with a 0-pixel before/after diff; package version 0.6.0.

**Audit:** none run; closed on the author's in-game UAT through three feedback rounds (quick tasks
2rf, 7lp and the post-phase fixes).

**Known deferred items at close:** 14 (see STATE.md Deferred Items) — HUMAN-UAT files of phases
30-35 left `partial` / `pending` and their VERIFICATION reports `human_needed` (walked by hand in the
feedback rounds, files not updated), plus the Phase 14 / 16 leftovers carried from v0.5.

**Verification at close:** full suite green including `artifact_guard` and `generate_snapshot`;
`cargo fmt` clean; clippy at its 121-warning baseline. `generate` output and the seed fixture are byte-identical (the
economy `# ff:` headers go only into the game-folder economy section).

---

## v0.5 — Generic Filter Editor & Initial App Configuration

**Shipped:** 2026-10-15
**Phases:** 16-28 (13) · **Plans:** 71 · **Commits:** 452 · **Tests:** 1142 green
**Timeline:** 2026-10-12 → 2026-10-15 · **Code:** ~50,244 lines Rust (+46,456/-5,977 in src/tests)

**Delivered:** FerrisFilter opens, edits and saves any working PoE2 filter, not just its own
output, with the style palette, tier cutoffs and target filter configurable from the overlay.

**Key accomplishments:**

1. **A lossless filter parser** — any PoE2 filter round-trips byte-identical; edits patch only the
   edited lines and are validated against the diagnostics they introduce.
2. **Styles became data** — `styles.json` with stable ids, rules linked by `# ff:style <id> <name>`
   and rewritten on Apply; committed `styles-default.json` / `tiers-default.json` defaults with
   per-user, git-ignored live files.
3. **The filter is the store** — `dynamic.json` is gone; `dynamic.filter` is edited in place with a
   fenced, regenerable economy section and guarded writes (changed-on-disk refusal, atomic write,
   one-time `.ffbak`).
4. **A rule list and full-grammar editor over any filter** — virtualized, searchable list that
   stays responsive on ~500-block foreign filters; every condition and action editable, including
   Defences, Simple Defences and a Between operator.
5. **A CONFIG tab with one Apply** — filter choice, styles and tier cutoffs, written all-or-none
   together with the filter.
6. **Overlay polish** — single running instance, and `/reloaditemfilter` sent to the game after a
   successful Apply.

**Audit:** `tech_debt` — requirements 73/73, phases 13/13, integration 20/20, E2E flows 15/15, no
gaps, no pending UAT.

**Known deferred items at close:** 2 (see STATE.md Deferred Items) — Phase 14 UAT file left
`diagnosed`, Phase 16 UAT partial pass (rest in backlog 999.12).

**Verification at close:** full suite green — 1142 tests including `artifact_guard` and
`generate_snapshot`; `cargo fmt --check` clean. The only fixture change in the milestone is the
deliberate `# ====` banner spelling.

---

## v0.4 — Overlay Filter Editor Rework

**Shipped:** 2026-09-17
**Phases:** 11-15 (5) · **Plans:** 16 · **Tasks:** 34 · **Commits:** 146 · **Tests:** 368 green
**Timeline:** 2026-09-03 → 2026-09-17 (14 days) · **Code:** ~16,093 lines Rust (+8,971/-404 in src/tests)

**Delivered:** An in-game overlay that edits filter rules directly and writes `dynamic.filter` on
demand, replacing pre-generated per-build presets — built on a callable compile API and a complete
headless dynamic pipeline, with `config.json → universal.filter` byte-for-byte untouched throughout.

**Key accomplishments:**

1. **The generate pipeline became a library API** — `compile::load_inputs` + `compile::compile`
   take caller-supplied paths, return matchable error variants, and report the price source as a
   value rather than printing it. Proven behavior-preserving across six invocation cases.
2. **A complete second filter pipeline, headless first** — `dynamic.json` reuses the existing
   `Rule`/`When` types; `ferrisfilter apply` seeds, compiles and atomically writes
   `dynamic.filter` into the game folder, economy sections included. Sequenced ahead of the GUI so
   it was testable on the project's existing terms before any window existed.
3. **An always-on-top overlay that never steals focus** — raw `ShowWindow(SW_SHOWNOACTIVATE)`
   rather than `ViewportCommand::Visible`, which is what makes a focus-preserving summon possible
   over a running game at all.
4. **A full rule editor with Apply as the single writer** — every condition editable and
   independently togglable. D-02 reversed mid-phase after live use: edits held in memory, Apply
   the only writer, Revert to discard.
5. **Base-type-only rules and rule names** — `class` gained the off switch every other condition
   had, making the author's hand-written *Change Bases* rule expressible in the editor.
6. **Economy sync from the overlay, league pinned** — poe.ninja's `indexed` flag was observed
   flipping between leagues within six hours, so automatic resolution cannot identify the current
   league and a wrong pick is silent.

**Audit:** `tech_debt` — requirements 26/26 verified (23 outright, 3 partial), phases 5/5,
integration 10/10, E2E flows 3/4.

**Known gap at close:** `ff` cannot be launched by double-clicking the exe (path resolution from
the process CWD). Deferred to v0.5 deliberately — the fix is a design decision about where the
overlay anchors its root, not a patch.

**Verification at close:** full suite green — 368 tests including `artifact_guard` and
`generate_snapshot` with zero fixture re-baselining across the entire milestone. Build
warning-free.

---

## v0.3 — Economy Items & Exceptional Gear

**Shipped:** 2026-06-27
**Phases:** 8-10 (3) · **Plans:** 5 · **Tasks:** 13 · **Commits:** 54 · **Tests:** 133 lib + integration green
**Timeline:** 2026-06-12 → 2026-06-27 · **Code:** ~7,534 lines Rust (+1,453/-726 in src/tests)

**Delivered:** Tracked three new/broken poe.ninja economy categories with correct in-game
matching and per-category gem styling, and added a `corrupted` rule condition plus
exceptional-gear highlighting for uncorrupted extra-socket / high-quality drops.

**Key accomplishments:**

1. **`mapping.json` schema v2** — a `gemFilters` array (UncutGems + LineageSupportGems, each
   with `backgroundOverride`) plus the `Verisium` overview type; `Mapping`/`GemFilter` Rust
   structs reshaped to match, `minLevel` dropped.
2. **Uncut Gems actually match in-game** — the `build_prices` gem branch was rewritten to emit
   one `PriceItem` per gem level (`Uncut Skill Gem (Level N)`) with a prefix-derived plural
   class; subtypes merge in the pricer into one multi-class `TierGroup`, and the emitter gained
   multi-class emit support. The old `strip_gem_level` + `gem_best` dedup — the root bug — is gone.
3. **Per-category gem backgrounds** — `emit_currency_sections` carries
   `CategoryBlock.background_override` through the priced-unique clone-null-append pattern:
   Uncut Gem blocks emit `SetBackgroundColor 80 70 55 255`, Lineage Support Gems
   `10 150 150 255`, with no category-name string checks in the emitter and a guard test
   against duplicate background lines.
4. **`corrupted` when-condition** — `When.corrupted: Option<bool>` emitting the bare boolean
   `Corrupted True` / `Corrupted False` (no `==`), slotted between `sockets` and `baseType` in
   the deterministic emit order, `serde(default)` for backward compatibility.
5. **Exceptional-gear highlighting** — three rules at the top of `config.json` `rules` (ahead of
   recycle): sockets >= 3 for 2h/body, sockets >= 2 for 1h/smaller, quality >= 24 — all
   `corrupted: false`, all using the new pink `exceptional` style (Pink Cross minimap, no sound),
   with a guard test asserting both the styling and the ordering vs recycle.
6. **One consolidated fixture re-baseline** — deferred to Phase 10 (the last emitter-output
   phase): `seed-universal.filter` + the insta snapshot re-baselined, `generate_block_counts`
   bumped 74 → 77 Show / 3 Hide.

**Audit:** `passed` — requirements 9/9, phases 3/3, integration 8/8, E2E flows 5/5.

**Verification at close:** full suite green on `main` at tag `v0.3` — 136 lib tests plus the CLI,
install and sync integration suites (166 total, 0 failures), `parity_ingame` included.

**Known deferred items at close:** 0 open GSD artifacts (audit clear). Carried tech debt: two
test-only compiler warnings (pre-existing — unreachable match arm in the emitter unknown-style
test, unused `FakeClient::ok` in `sync/metadata.rs`); `10-01-SUMMARY.md` prose stale on
`exceptional` being a required vs optional `Styles` field. Backlog Phase 999.1 (config-driven
style vocabulary) deliberately not pulled in.

**Archives:** [v0.3-ROADMAP.md](milestones/v0.3-ROADMAP.md) · [v0.3-REQUIREMENTS.md](milestones/v0.3-REQUIREMENTS.md) · [v0.3-MILESTONE-AUDIT.md](milestones/v0.3-MILESTONE-AUDIT.md)

## v0.2 — Declarative Rule Model

**Shipped:** 2026-06-12
**Phases:** 5-7 (3) · **Plans:** 7 · **Commits:** ~38 · **Tests:** 154 green
**Timeline:** 2026-06-05 → 2026-06-12

**Delivered:** Replaced the generator's hardcoded gear/flask band logic with a general,
declarative rule schema in `config.json`, making the fully-functional in-game filter
reproducible from config alone.

**Key accomplishments:**

1. **Rule schema** — an ordered `rules` array (first-match-wins) with a full `when` condition
   vocabulary (class, compact rarity comparator, areaLevel min/max, base-defence/quality/sockets,
   BaseType, UnidentifiedItemTier) + `classGroups` aliases; serde-validated, unknown-style hard error.
2. **Emitter rewrite** — generic `emit_rules` compiler replaces the hardcoded
   `emit_gear_bands`/`emit_flask_charm_bands`/`emit_recycle`; `config.json` migrated to rules;
   the dead `areaLevelBands` model removed.
3. **Migration parity** — `generate --early-game` reproduces the pre-edit baseline and (with the
   capability rules) the author's in-game filter with zero non-reorder policy differences;
   `seed-universal.filter` re-baselined; pricer currency/unique sections untouched.
4. **New capabilities as config** — the 7 in-game edits (weapon/armour caps, flask/charm
   Magic+Rare + rarity-bounded Hide, Normal charms ≥78, jewellery hide, unidentified-tier
   highlight via a new `unidentified` style).
5. **Docs + fix** — CLAUDE.md realigned to the rule model (UnidentifiedItemTier + base-defence
   confirmed); dropped the invalid `Class "Tablet"` rows that failed to load in-game.

**Known deferred items at close:** BaseType-matched, economy-tiered Tablets category (needs
pricer/emitter support for class-less category blocks). Generate QoL + sync extensions deferred.

**Archives:** [v0.2-ROADMAP.md](milestones/v0.2-ROADMAP.md) · [v0.2-REQUIREMENTS.md](milestones/v0.2-REQUIREMENTS.md) · [v0.2-MILESTONE-AUDIT.md](milestones/v0.2-MILESTONE-AUDIT.md)

## v0.1 — Initial Release

**Shipped:** 2026-05-28
**Phases:** 1-4 (4) · **Plans:** 14 · **Commits:** 127 · **LOC:** ~4,970 Rust
**Timeline:** 2026-05-21 → 2026-05-28

**Delivered:** A Rust CLI that compiles a human-editable policy model into a deterministic PoE2
loot filter and prices currency from poe.ninja.

**Key accomplishments:**

1. **Foundation** — `ferrisfilter` crate + clap CLI (`generate`/`sync`) + serde models for `config.json`/`tiers.json`/`prices.json` with field-path-aware validation errors.
2. **Static generate** — deterministic emitter reproducing the hand-built `universal.filter` byte-for-byte, locked by an insta snapshot + `artifact_guard` test (model-path block tags, AreaLevel bands, flask/charm ladder, recycle, fallthrough).
3. **Pricing layer** — cross-rate normalization to chaos + `tiers.json` cutoffs, TIER-04 abort on bad reference rates, graceful no-`prices.json` degradation; `currency.json` dropped from the generate path.
4. **Sync** — poe.ninja PoE2 API spike resolved, then `sync` (prices/metadata) with live league resolution, gem≥18 filter, anchors-first ordering, and atomic failure-safe writes.
5. **Extras** — `install` subcommand (replaces `install.bat`); early-league fallback (`prices-fallback.json` + `tiers-fallback.json` + `--early-game`) with cross-rates derived from the anchor orbs.

**Known deferred items at close:**

- Live softcore no-flag `sync` happy path awaits a new active PoE2 league ("Fate of the Vaal" ended; tool fails safe). Verification + UAT signed off as working on 2026-05-28 via the live `--standard` run (318 items). See STATE.md → Deferred Items.
- Lenient-parse guard for Standard-league unique endpoints (non-blocking); optional no-flag `sync` → `--standard` fallback/hint.

**Archives:** [v0.1-ROADMAP.md](milestones/v0.1-ROADMAP.md) · [v0.1-REQUIREMENTS.md](milestones/v0.1-REQUIREMENTS.md)
