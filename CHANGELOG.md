# Changelog

All notable changes to **FerrisFilter** are documented here.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

> **Maintenance:** keep `[Unreleased]` current as work lands, and **whenever a milestone is
> completed** (e.g. via `/gsd-complete-milestone`) move its entries into a new dated version
> section below with a one-line milestone summary.

## [Unreleased]

### Added

- **Phase 27 (UAT4-01..04).**
  - **Simple Defences** condition: AR / EV / ES rows of Any | Has | No (`X > 0` / `X 0`), a strip of
    the 7 combinations, mutually exclusive with Defences in the picker. Zero-based defence lines load
    as Simple Defences with bytes kept; anything else stays Defences, and a Defences card does not
    flip while editing or on Apply.
  - The CONFIG Custom styles Order column and arrows are right-aligned.
  - Colour dropdowns no longer cover colour names with the scroll bar (the rows were shrink-wrapped, so
    the floating bar was painted over the longest name; they now fill the popup width).
  - UAT results of phases 16 and 22-26 recorded; two items moved to the backlog.

- **Phase 26 (UAT3-01..08).**
  - Stable style ids: every `styles.json` style has an `id` (`builtin:<name>` for built-ins, a random
    6-8 character id for custom styles; id-less files get deterministic ids at load, persisted by the
    next Apply). Rules are tagged `# ff:style <id> <name>`; old `# ff:style <name>` tags still resolve
    by name and are rewritten on the next Apply (a tag-only migration does not mark the filter
    unapplied).
  - Custom styles can be renamed inline in CONFIG; rules stay linked, including rules with a legacy
    name-only tag. `config.json` still references styles by name, so `generate` fails until it is updated.
  - CONFIG styles are one table per group (Built-in / Custom with an Order column); the RULES style
    picker follows the unapplied CONFIG style set.
  - One **Defences** condition (Armour / Evasion / Energy Shield toggles) per rule.
  - **Between** operator for whole-number conditions: two sliders, min clamped never above max; a loaded
    lower/upper pair shows as one card with its bytes kept.
  - Already-present conditions are greyed in the picker and focus the existing card; pending conditions
    show their operator before a value; picker sections start collapsed.
  - Tray Quit with unapplied edits asks **Apply & quit** (waits for an in-game reload) / Quit without
    applying / Cancel.

- **OVERWRITE after a changed-on-disk refusal** (RULES tab). The refusal now reads `Filter changed on
  disk since it was loaded.` and offers OVERWRITE beside RELOAD (tooltips explain both): it replaces
  the outside change with the in-memory rules through the same guarded write (current disk bytes
  become the baseline; atomic write, retry and one-time `.ffbak` unchanged), then counts as an Apply,
  in-game reload included. Since Phase 25 the same RELOAD/OVERWRITE pair is offered on both tabs.
- **In-game reload after Apply** (Phase 23, RELOAD-01). A successful RULES Apply now sends
  `/reloaditemfilter` to a running Path of Exile 2 (focus, Enter, paste, Enter, with no Ctrl+A, as EE2 does for `/` commands; clipboard
  text restored), status `Reloaded in game` (the keys were sent; ff cannot confirm the game ran the
  command, so Refresh by hand if the game did not reload). Nothing is sent on a refused or CONFIG Apply,
  with the game closed, or with the new CONFIG setting **Reload filter in game after Apply** off
  (default on, `reload_after_apply` in `ff-settings.json`; an unusable value or settings file
  turns it off, with a warning); the Refresh reminder then names the
  reason. The game window is matched by title and `PathOfExile*` process name. Caveats: it reloads the
  filter selected in game; close chat first (if the chat box is already open with text, that text is
  sent as a message and the command is not, because nothing outside the game can see whether chat
  is open); run ff and the game at the same privilege level (an elevated game silently drops the
  keys while the status still reads `Reloaded in game`); non-text clipboard content is replaced.
  `ff-cli apply` is unchanged. No new dependency (three `windows-sys` features).
- **Single-instance overlay** (Phase 22, INST-01). Launching `ff` while it is running reveals the
  existing stripe (also when hidden to the tray, without stealing focus) and exits instead of opening
  a second one; a not-responding instance produces a "FerrisFilter is already running but not
  responding" message box after about 2 seconds. A launch made while the running instance is still
  starting up waits up to 10 seconds for it to become ready before that window starts, and several
  launches at once are each answered. Tray Quit releases the hotkey, the tray icon and then the instance name immediately, so a
  relaunch right after Quit starts a fresh stripe, and a launch whose running instance quits or
  crashes while it is asking starts instead of showing a message. If the single-instance check
  itself fails, `ff` does not start and names the Windows error in a message box, so it never opens
  a second stripe. Debug and release builds use separate instance names.
  Mechanism: a `Local` named mutex plus show/ack/ready events, no new dependency.
- Overlay **CONFIG tab** (Phase 21). The stripe now has RULES | CONFIG tabs; switching never applies
  or discards anything, and a tab shows ` *` when the other one holds unapplied edits.
  - **FILTER:** pick any `*.filter` in the game folder, or create a new empty one (it is written by
    the first RULES Apply). The choice is remembered in the git-ignored per-user `ff-settings.json`
    (default `dynamic.filter`) and restored on the next launch. Switching while the RULES tab has
    unapplied edits asks first (`Switch and discard` / `Stay`).
  - **STYLES:** add, edit and delete named styles with preview chips; the built-ins S-F, `unchanged`
    and `hidden` cannot be deleted. Deleting a style used by rules shows how many, and warns when
    `config.json` itself references it. The colour pickers are opaque: styles store RGB and the
    filter always emits alpha 255.
  - **TIERS:** edit the `tiers.json` cutoffs with inline validation; Apply is disabled while any
    cutoff is invalid.
  - Its own **Revert**. CONFIG edits are saved by the one shared **Apply** (see the UAT round 2
    entry below): `styles.json`, `tiers.json` and the filter are written all or none, and a file
    edited outside the overlay is refused rather than overwritten silently (RELOAD / OVERWRITE).

- Full-grammar block editing in the overlay: every condition and action of any block is editable
  with structured controls (grouped searchable **+ condition** / **+ action** pickers, operator
  dropdown with plain-English labels, sliders with exact numbers, class groups from `config.json`,
  chip lists, colour picker with alpha, minimap/beam/sound controls). Edits rewrite only the edited
  value's lines; unknown-keyword lines are editable raw rows.
- Style linking: a STYLE group applies a named style from `styles.json` to a rule (`#ff:style
  <name>`). Apply rewrites styled rules from the current `styles.json` and reports `Updated N
  styled rules.`; `Remove style` keeps the actions as the rule's own; unknown style names are
  flagged (`unknown style`) and reported without blocking Apply or being rewritten. Collapsed rows
  show `styled: <name>`.

- `styles.json` (committed, repo root) is now the single source of named styles. Adding a style
  needs no code change: define it there and use it from a rule. Validated at load (required styles
  S, A, B, C, D, E, F, unchanged and hidden; duplicate names; names differing only by case from a
  required one; unsafe custom sound names; font size 1-45, minimap size 0-2, sound id 1-16 and
  volume 0-300; a sound must set exactly one of `id` or `custom`; unknown styles in enabled
  rules). The overlay style
  picker lists styles.json in file order.
- `--styles <path>` on `generate` and `apply` to compile against another styles file instead of
  styles.json.
- Lossless filter parser library module `ferrisfilter::filter`: parses any PoE2 filter in GGG's
  grammar (full keyword table including Continue/Import/Minimal; unknown keywords are kept with a
  warning) and writes it back byte-identical when unedited (CRLF/LF/mixed endings, BOM, no final
  newline), verified on the committed FilterBlade/NeverSink fixture under `tests/fixtures/parser/`.
  Edits touch only the edited block's lines (enable/disable, replace lines, set or remove a
  metadata key) and never produce an enabled block with an Error or Warning diagnostic, nor move
  lines between other blocks (such an edit is refused unchanged);
  commented-out blocks are recognized as disabled rules (valid or invalid), a disabled rule is
  enabled only whole (one whose commented conditions are split by a comment line, or which would
  enable as a bare catch-all, is refused), and FerrisFilter's own
  `#ff:<key> <value>` metadata comments are exposed. Library and tests only; no CLI or overlay
  change.
- Filter-as-store economy section: the game-folder `dynamic.filter` is edited in place and its
  economy section is fenced by `#ff:economy-begin ...` / `#ff:economy-end ...` marker lines.
  Apply regenerates only that section (from prices.json, tiers.json, styles.json) and leaves
  every other line alone; damaged markers stop apply.
- `apply --filter <path>` to target any filter file, `--add-economy` to create the section at the
  end of the file and `--remove-economy` to delete it. Plain `apply` refreshes an existing section
  and never adds one.
- First write keeps a `<name>.filter.ffbak` backup (created once, never overwritten); writes are
  atomic with retry; apply refuses a file that changed on disk since it was loaded (the overlay
  offers RELOAD).
- App-root resolver: apply and the overlay find styles.json, tiers.json and prices.json via
  `FERRISFILTER_HOME`, else by walking up from the exe's folder, else the working directory, else
  an error. The overlay's SYNC ECONOMY writes prices.json there. A relative `apply --tiers` /
  `--styles` resolves against that same folder, so `apply --tiers tiers-fallback.json` works from
  any working directory.
- Overlay rule list over the parsed blocks of the game-folder filter: a virtualized list with search,
  add, delete, duplicate, reorder, enable/disable via comment-out, Show/Hide/Minimal and Continue,
  `#ff:name` naming, an economy footer row (Add / Remove section), Apply / Revert, and a grammar-error
  gate that refuses Apply. `filter::Document` gained structural edits (append before the economy
  section, remove, duplicate, move, keyword, Continue); `dynamic::apply_edited` and
  `dynamic::blocking_errors` back Apply. A rule's `#ff:style` tag is displayed, but its action is not
  yet applied on Apply (Phase 20).
- One-time conversion of a v0.4 `dynamic.filter` on its first apply (markers added, `Show # <name>`
  names kept as `#ff:name`).

### Changed

- **Overlay UAT fixes, round 2** (Phase 25, UAT2-01..UAT2-10). Continue to later rules is the last
  line of the rule body; the Custom action button reads **+ style**; Show/Hide appears below the style
  picker for Custom only; the rule name is a plain text box and an unnamed rule header shows only the
  index and Show/Hide (no derived label); Sockets is a plain 0-6 slider (colour letters in existing
  lines are kept, never edited); `Exceptional` leaves the keyword table and loads as a raw row; CONFIG
  has no unapplied-changes line and no "new" tag; Add style sits on the Custom heading; the styles
  table has a Visibility | Name | Preview header and custom styles reorder with arrows; one
  all-or-none APPLY on both tabs writes `styles.json`, `tiers.json` and the filter together, with
  OVERWRITE/RELOAD on both tabs and per-tab Revert (an unedited `styles.json`/`tiers.json` follows
  the disk and is never written or guarded). No generator change.
- **Overlay UAT fixes** (Phase 24, UAT-01..UAT-13). No reload text when the game is closed; Styles
  grouped Built-in/Custom with no badge; per-style Show/Hide slide switch; no Minimal option, a foreign
  Minimal block is untouched; condition remove button right-aligned; quoted lists vertical and capped;
  Class picker without chips; no style caption; 25 PoE1-only conditions removed from the keyword table
  and picker (still loaded as raw rows, Apply not blocked); GemLevel default 20 and every default inside
  its slider range; style section moved to the end with Custom or a preset; Yes/No booleans; collapsed
  rows show a condition summary. No generator change.

- **Comment spacing: every comment FerrisFilter writes has a space after `#`.** Metadata and economy
  marker lines are now written `# ff:<key> <value>` (`# ff:name`, `# ff:style`,
  `# ff:economy-begin`, `# ff:economy-end`); the old `#ff:` spelling is still read everywhere (a
  `#ff:economy-begin` pairs with a `# ff:economy-end`, and one of each counts as a duplicate), and
  every Apply (`ff-cli apply` and the overlay) rewrites any remaining `#ff:` line to `# ff:` while
  leaving all other bytes alone. The `generate` economy banner is now `# ====...` (the only change
  to `generate` output; the seed fixture and snapshot were regenerated).
- **Disabled blocks are written with `# ` too.** Disabling a block now writes `# ` in front of each
  line (`# Show`, `# \tRarity Rare`, `# \tSetFontSize 40`; an empty line becomes a bare `#`), and
  each line keeps its own line ending. Enabling removes the `#` and one space after it when there
  is one, otherwise just the `#`, so the older `#Show` form (blocks FerrisFilter disabled earlier,
  and foreign filters) still enables correctly; disable then enable is byte-identical for any line,
  including lines that start with a space or a tab. Detecting disabled blocks reads both forms the
  same way: `# Show only rares` stays prose, `# Show` / `# Hide # name` are block headers, and a
  `# ff:` line is metadata that ends a block. Existing `#Show` blocks are **not** rewritten on
  Apply (FerrisFilter cannot tell its own old disabled blocks from a foreign filter's, and Apply
  only normalizes lines that are unambiguously its own); a block takes the `# ` form the next time
  it is disabled.
- A RULES Apply whose result equals the file on disk no longer reports "already up to date; nothing
  was written": it counts as applied (same status as a write, in-game reload sent). The identical
  file is still not rewritten. `ff-cli apply` keeps its message.
- A rule whose `#ff:style` tag names a style that no longer exists now loses that tag on the next
  RULES Apply and keeps its actions (previously the tag was kept and only reported). Deleting a
  style in the CONFIG tab therefore detaches its rules instead of leaving dangling tags.
- Overlay layout: all footer messages share one outcome area capped at 96px, the stale indicator
  is 14px body text, controls use 24px click targets, spacing follows a 4/8/16 scale, accent text
  uses a higher-contrast token, and a rule row's label keeps at least 140px (extra flags fold into
  a `+N` badge).
- Parser line edits: a single-line patch keeps that line's own line ending, and edits to blocks
  with pre-existing warnings are allowed unless they add new problems.
- APPLY no longer requires an economy section; the ADD ECONOMY SECTION button moved into the
  economy footer row; the overlay's v0.4 JSON-rule modules (edit, rule editor, view model) were removed.
- **BREAKING:** the game-folder filter is now the only store. `dynamic.json` and
  `dynamic_base.json` are gone (the template survives only as the test fixture
  `tests/fixtures/v04-dynamic-base.json`); `apply --reset`, `--base`, `--store` and `--dest` are
  removed; plain `apply` refreshes only the economy section of an existing filter. The overlay's
  rule editor returned as the rule list over the filter's parsed blocks (see Added). A leftover
  repo-root `dynamic.json` is no longer read and can be deleted; it stays git-ignored so it is
  never committed. `*.ffbak` backups are git-ignored too.
- **BREAKING:** `config.json` and `dynamic_base.json` no longer have a `styles` section. A config
  or store (`dynamic.json`, custom `*_config.json`) that still has one loads only if every inline
  entry equals the same-named styles.json entry; the section is then dropped (the overlay's next
  Apply saves the store without it). Otherwise it fails to load with a message naming the file and
  the differing styles: move those into styles.json, then delete the section (or `apply --reset`
  the store). An existing `dynamic.json` that used a custom style such as `craftable` needs that
  style added to styles.json first. `generate` output is
  unchanged (the seed fixture is byte-identical). (The `dynamic.json` store, the overlay saving
  it, and `apply --reset` named here are superseded by the filter-as-store change above: no JSON
  store exists any more and Apply never writes a config.)

### Removed

- `BaseDefencePercentile` from the keyword table (Phase 26): a line using it loads as a raw row and
  is kept byte for byte.

### Fixed

- The class group closing on the first value click in a new condition (Phase 26, UAT3-04). The open
  class group's egui id changed when the pending row became a real line (an epoch bump on AddLine plus
  different pending and real salts); its open state now lives under a row-epoch-keyed absolute id.

## [0.4.0] - 2026-09-17

**Overlay Filter Editor Rework** — FerrisFilter gains an in-game overlay that edits filter rules
directly and writes `dynamic.filter` on demand, replacing pre-generated per-build presets. Built on
a callable compile API and a complete headless dynamic pipeline, with `config.json →
universal.filter` byte-for-byte untouched throughout: `artifact_guard` and `generate_snapshot`
passed with zero fixture re-baselining across all five phases. Tagged `v0.4`.

### Added

- **`ff` — the overlay GUI binary**, now `cargo build`'s default output (`default-run = "ff"`).
  Opens an opaque, undecorated, always-on-top vertical stripe docked to the right screen edge
  over a running Path of Exile 2, styled in PoE2's own Options-menu palette (Cinzel display font,
  bronze header plate, zebra-striped rows). A Ferris tray icon with a Show/Quit menu keeps the
  process reachable while the stripe is hidden; a system-wide `Ctrl+Shift+Z` hotkey summons and hides
  it without alt-tabbing and without stealing keyboard focus from the game. The stripe lists the
  rules in `dynamic.json` in ordinal order and is a full editor for them — see below.
- **The overlay rule editor** — every rule in the stripe expands in place into its own controls for
  style, item classes (grouped Weapons / Armour / Jewellery / Flasks & Charms), rarity, base
  defences, sockets, quality, corrupted state, and unidentified tier. Rules reorder with up/down
  arrows, enable and disable without being deleted, append with **+ ADD RULE**, and delete behind
  an inline confirm. A collapsed rule shows a one-line summary of what it matches, with a
  fully-selected class group collapsing to `Any Weapons` / `Any Armour` / `Any Jewellery` /
  `Any Flasks & Charms`.
- **APPLY and REVERT** — edits are held in memory until **Apply**, which saves `dynamic.json` and
  compiles `dynamic.filter` straight into the game folder, then repeats the reload path the game
  requires (*Options > Game > Item Filter > Refresh*), because PoE2 never re-reads a filter on its
  own. **Revert** appears only while there are unsaved changes and discards them. The stripe always
  states, in words, whether the written filter still matches the saved rules.
- **`sync --league <name|slug>`** — target a specific poe.ninja league by name or url slug. Needed
  when the automatic pick resolves to the outgoing league during a league transition, or when a new
  league is live but poe.ninja has not yet flagged it as indexed.
- **`generate --tiers <path>` / `apply --tiers <path>`** — override tier cutoffs while keeping live
  prices, so an exalt-denominated early-league tier table can be used against a young economy
  without editing the committed `tiers.json`.
- **`ff-cli` — the short-name console CLI**, identical to `ferrisfilter`, just shorter to type.
  Holds the console entry point `ff` used before it became the GUI binary. `ferrisfilter` itself
  is unchanged.
- **`Jewels` in the overlay's class picker**, under the **JEWELLERY** group. A jewel is its own
  item class, not a piece of jewellery, but it is the only class that would ever sit in a group
  alone, and a one-item header would spend a whole collapsible section on a single checkbox. This
  is picker data only — sourced from `config.json`'s `jewelleryClasses`, never read by the
  emitter — so jewels still fall through to the game default in `universal.filter` (GEN-04) and
  the generated output is byte-for-byte unchanged.

### Removed

- **The `seed_prices` migration binary, `src/models/currency.rs`, and `currency.json`.** The
  binary was a one-shot from v0.1 Phase 3: before `sync` existed it flattened the hand-maintained
  `currency.json` tier table into the first `prices.json`, whose output became
  `tests/fixtures/seed-prices.json`. It ran once and nothing has invoked it since. Worse, its
  input was never kept in step with the fixture — re-running it today reintroduced `Vaal Infuser`
  and seven Precursor Tablets, deliberately dropped in Phase 7 for carrying an invalid
  `Class "Tablet"`, and dropped eight entries added later (Verisium, uncut gems, support gems).
  So it was not a reproducible-provenance tool but a footgun that still compiled, and it was the
  source of the `bin.name` kebab-case build warning. `currency.json` had no runtime reader at all
  (verified: `generate` runs to completion in a directory without it) and only survived because
  four test helpers copied it into their temp dirs. The five `currency.rs` tests that went with it
  validated nothing but the deleted file. All recoverable from git history at `6ba59af`.

### Changed

- **The `unidentified` style is renamed `kite`**, after the minimap icon it paints. The old name
  described the rules that happened to use it rather than what it does, which read as nonsense when
  the overlay offered it for an unrelated rule. Any config naming `unidentified` must be updated —
  an unknown style name is a hard error at generate time, by design.

## [0.3.0] - 2026-06-27

**Economy Items & Exceptional Gear** — three new/broken poe.ninja economy categories are tracked
with correct in-game matching and per-category gem styling, and the rule vocabulary gained a
`corrupted` condition driving exceptional-gear highlighting. Tagged `v0.3`.

### Added

- **Verisium** — new synced category (poe.ninja `type=Verisium`), emitted with the regular tiered
  currency style (`Class "Stackable Currency"` + `BaseType ==`, no background override).
- **Lineage Support Gems** — new synced category (`type=LineageSupportGems`), price-tiered and
  emitted with `Class == "Support Gems"` (confirmed by a poe2db spike before the mapping landed).
- **Per-category gem backgrounds** — priced gem blocks keep the value tier's text and border color
  but replace the Dracula background: Uncut Gems `80 70 55`, Lineage Support Gems `10 150 150`.
  Implemented with the same clone-null-append mechanism as the priced-unique bronze background, as
  a data-driven `background_override` rather than a new named style.
- **`corrupted` rule condition** — `when.corrupted` emits the bare boolean `Corrupted True` /
  `Corrupted False` (PoE2 boolean conditions take no operator). Absent by default, so existing
  configs are unaffected.
- **`exceptional` style + exceptional-gear rules** — three rules at the top of `rules` (ahead of
  recycle) surface uncorrupted gear with extra sockets (`Sockets >= 3` for two-hand/body classes,
  `Sockets >= 2` for one-hand/smaller) or `Quality >= 24`, in a dedicated pink style
  (`255 121 198`, Dracula background, Pink beam, Pink Cross minimap, no sound). Kept separate from
  `S` so its color can be retuned independently.
- **`generate --config <path>`** — point the `generate` subcommand at a config file other than
  the default `config.json`. Defaults to `config.json`, so existing invocations are unchanged.
- **Optional `craftable` style** — the `styles` object accepts an additional `craftable` style
  (green text/border on a light background, etc.) that rules can reference. It is optional and
  absent from the default `config.json`, so the committed filter and fixtures are unchanged. The
  style vocabulary is still hardcoded in the model — making styles fully config-driven is tracked
  as backlog Phase 999.1.

### Fixed

- **Uncut Gems now actually match in-game.** The emitted `BaseType` keeps the level
  (`Uncut Skill Gem (Level N)`, one priced entry per level) and the `Class` is the per-subtype
  plural (`Uncut Skill Gems` / `Uncut Spirit Gems` / `Uncut Support Gems`). The `strip_gem_level`
  + `gem_best` dedup that erased the level — the root cause — is gone, and the `minLevel: 18`
  floor no longer silences lower-level gems that price above a tier cutoff.
- **`sync` classifies the Expedition logbook and Sagas correctly** from their mixed-class feed.
- **`sync` no longer requests PoE1-only `UniqueMaps`**, which does not exist in the PoE2 dataset.

### Changed

- `mapping.json` moved to **schema v2**: a `gemFilters` array (one entry per gem category, each
  with an optional `backgroundOverride`) replaces the single gem filter, and `minLevel` is dropped.
- The emitter supports **multi-class blocks**, so merged uncut-gem subtypes share one block instead
  of duplicating.
- Generated block counts: **74 → 77 `Show`** (3 exceptional-gear blocks), 3 `Hide` unchanged.
  `tests/fixtures/seed-universal.filter` and the insta snapshot were re-baselined in one
  consolidated pass.

### Verification

- Full suite green on `main` at tag `v0.3`: 136 lib tests plus the CLI, install and sync
  integration suites (166 total, 0 failures) — `artifact_guard`, `generate_snapshot` and
  `parity_ingame` included.

## [0.2.0] - 2026-06-12

**Declarative Rule Model** — the generator's hardcoded gear/flask band logic is replaced by a
general, declarative rule schema in `config.json`, so any class + level-range + rarity (+
defence / quality / sockets / unidentified-tier / base-type) policy is expressible in config.
The full in-game filter is now reproducible from `config.json` alone. Tagged `v0.2`.

### Added

- **Declarative rule schema** (`config.json` `rules` array) — an ordered, first-match-wins list
  of show/hide rules. Each rule references a named style and an optional `when` block whose
  condition vocabulary covers class lists, a compact rarity comparator (`">=Magic"` etc.),
  `areaLevel` min/max, base-defence/`Quality`/`Sockets` numeric conditions, `BaseType` lists,
  and `UnidentifiedItemTier`. Optional `classGroups` aliases. Serde-validated with field-path
  errors; unknown style names are a generate-time hard error.
- **New capabilities now expressible as config** — build-weapon Magic+Rare up to a configurable
  area-level cap, build-armour upper cap, flask/charm mid-band Magic+Rare with a rarity-bounded
  Hide fallback, Normal charms surfaced at high area level, low-rarity jewellery hide, and a
  high-tier unidentified-item highlight (new `unidentified` style).

### Changed

- **Emitter rewritten** as a generic rule compiler (`emit_rules`) — the hardcoded
  `emit_gear_bands` / `emit_flask_charm_bands` / `emit_recycle` special-casing is gone; all
  policy blocks are produced by iterating the rule list. Price-driven currency/unique sections
  are unchanged. The `seed-universal.filter` artifact baseline was deliberately re-locked.
- **`config.json` migrated** to the rule schema; the dead `areaLevelBands` model
  (`AreaLevelBand`/`BandShow`/`ArmourShow`/`RarityRule`) was removed.
- **CLAUDE.md realigned** to the declarative rule model; `UnidentifiedItemTier` and the
  base-defence filters are documented as confirmed working in-game.

### Fixed

- Dropped the **Tablets** category and **Vaal Infuser** from the price data — they emitted an
  invalid `Class "Tablet"` block (tablets have no usable PoE2 filter class; they match by
  BaseType only), which caused the filter to fail to load in-game. (A BaseType-matched,
  economy-tiered tablet category is deferred — see PROJECT.md.)

## [0.1.0] - 2026-05-28

**Initial Release** — a Rust CLI that compiles a human-editable policy model into a Path of
Exile 2 loot filter and prices currency from poe.ninja. Tagged `v0.1`.

### Added

- **`generate` command** — compiles `filter-model.json` + `config.json` into `universal.filter`
  deterministically (byte-for-byte reproducible, guarded by a snapshot/`artifact_guard` test).
  Implements the Dracula tier palette, the single-file `AreaLevel` progression, build-gear
  taper, and the "hide what we don't want, style the highlights" philosophy.
- **`sync` command** — fetches live PoE2 economy data from poe.ninja and writes `prices.json`;
  resolves the current league and honors `--standard`/`--hc`. `sync metadata` rebuilds the
  committed poe.ninja-type → filter-class `mapping.json`.
- **`install` command** — copies a generated `.filter` into the in-game config folder
  (`--dest`/`--input` overrides); replaces the old `install.bat`.
- **Pricing & tier system** — `tiers.json` cross-rate math normalizes prices to chaos and
  assigns each currency a tier (S–F); price-tiered unique highlights; graceful handling of a
  missing `prices.json`.
- **Early-league fallback** — committed `prices-fallback.json` + `tiers-fallback.json` pair,
  used together when `prices.json` is absent/empty (with a warning) or when `generate
  --early-game` is passed. Cutoffs are exalt-denominated (`Unit::Exalt`) so currency tiers like
  the poe2filter.com export; cross-rates are derived from the anchor orbs (no stored rates).

[Unreleased]: https://github.com/Jonathas-Conceicao/poe2-myfilter/compare/v0.3...HEAD
[0.3.0]: https://github.com/Jonathas-Conceicao/poe2-myfilter/compare/v0.2...v0.3
[0.2.0]: https://github.com/Jonathas-Conceicao/poe2-myfilter/compare/v0.1...v0.2
[0.1.0]: https://github.com/Jonathas-Conceicao/poe2-myfilter/releases/tag/v0.1
