# Roadmap: FerrisFilter

## Milestones

- ✅ **v0.1 Initial Release** — Phases 1-4 (shipped 2026-05-28) — [archive](milestones/v0.1-ROADMAP.md)
- ✅ **v0.2 Declarative Rule Model** — Phases 5-7 (shipped 2026-06-12) — [archive](milestones/v0.2-ROADMAP.md)
- ✅ **v0.3 Economy Items & Exceptional Gear** — Phases 8-10 (shipped 2026-06-27) — [archive](milestones/v0.3-ROADMAP.md)
- ✅ **v0.4 Overlay Filter Editor Rework** — Phases 11-15 (shipped 2026-09-17) — [archive](milestones/v0.4-ROADMAP.md)
- ✅ **v0.5 Generic Filter Editor & Initial App Configuration** — Phases 16-28 (shipped 2026-10-15) — [archive](milestones/v0.5-ROADMAP.md)
- 🚧 **v0.6 New PoE2-based Tool UI** — Phases 29-35 (in progress)

## Phases

<details>
<summary>✅ v0.1 Initial Release (Phases 1-4) — SHIPPED 2026-05-28</summary>

- [x] Phase 1: Foundation (3/3 plans) — completed 2026-05-24
- [x] Phase 2: Static Generate (3/3 plans) — completed 2026-05-24
- [x] Phase 3: Pricing Layer (4/4 plans) — completed 2026-05-25
- [x] Phase 4: Sync (4/4 plans) — completed 2026-05-28

Full detail: [milestones/v0.1-ROADMAP.md](milestones/v0.1-ROADMAP.md)

</details>

<details>
<summary>✅ v0.2 Declarative Rule Model (Phases 5-7) — SHIPPED 2026-06-12</summary>

- [x] Phase 5: Rule Schema (2/2 plans) — completed 2026-06-05
- [x] Phase 6: Emitter Rewrite + Migration Parity (2/2 plans) — completed 2026-06-05
- [x] Phase 7: New Capabilities + Docs (3/3 plans) — completed 2026-06-12

Full detail: [milestones/v0.2-ROADMAP.md](milestones/v0.2-ROADMAP.md)

</details>

<details>
<summary>✅ v0.3 Economy Items & Exceptional Gear (Phases 8-10) — SHIPPED 2026-06-27</summary>

- [x] Phase 8: Sync Pipeline (2/2 plans) — completed 2026-06-24
- [x] Phase 9: Gem Background Styling (1/1 plan) — completed 2026-06-24
- [x] Phase 10: Corrupted Condition + Exceptional Gear (2/2 plans) — completed 2026-06-24

Full detail: [milestones/v0.3-ROADMAP.md](milestones/v0.3-ROADMAP.md)

</details>

<details>
<summary>✅ v0.4 Overlay Filter Editor Rework (Phases 11-15) — SHIPPED 2026-09-17</summary>

- [x] Phase 11: Callable Compile API (2/2 plans) — completed 2026-09-03
- [x] Phase 12: Dynamic Rule Store (2/2 plans) — completed 2026-09-04
- [x] Phase 13: Overlay Shell (5/5 plans) — completed 2026-09-06
- [x] Phase 14: Rule Editor + Apply (4/4 plans) — completed 2026-09-08
- [x] Phase 15: BaseType Lists and Rule Names (3/3 plans) — completed 2026-09-16

Full detail: [milestones/v0.4-ROADMAP.md](milestones/v0.4-ROADMAP.md)

</details>

<details>
<summary>✅ v0.5 Generic Filter Editor & Initial App Configuration (Phases 16-28) — SHIPPED 2026-10-15</summary>

- [x] Phase 16: Lossless Filter Parser (4/4 plans) — completed 2026-10-15
- [x] Phase 17: Config-Driven Styles (4/4 plans) — completed 2026-10-13
- [x] Phase 18: Filter as Store & Economy Section (7/7 plans) — completed 2026-10-15
- [x] Phase 19: Overlay Rule List Over Parsed Blocks (7/7 plans) — completed 2026-10-13
- [x] Phase 20: Full-Grammar Block Editing (9/9 plans) — completed 2026-10-15
- [x] Phase 21: Overlay Configuration Tab (6/6 plans) — completed 2026-10-13
- [x] Phase 22: Single-Instance Overlay (2/2 plans) — completed 2026-10-15
- [x] Phase 23: Auto-Reload Filter in Game on Apply (4/4 plans) — completed 2026-10-15
- [x] Phase 24: UAT Feedback Fixes (6/6 plans) — completed 2026-10-15
- [x] Phase 25: UAT Feedback Fixes 2 (4/4 plans) — completed 2026-10-15
- [x] Phase 26: UAT Feedback Fixes 3 (9/9 plans) — completed 2026-10-15
- [x] Phase 27: UAT Feedback Fixes 4 (5/5 plans) — completed 2026-10-15
- [x] Phase 28: Default config files (4/4 plans) — completed 2026-10-15

Full detail: [milestones/v0.5-ROADMAP.md](milestones/v0.5-ROADMAP.md)

</details>

### 🚧 v0.6 New PoE2-based Tool UI (In Progress)

**Milestone Goal:** `ff` is rebuilt in Path of Exile 2's own Options-window style on the shared
skin proven by `ff-proto` (quick tasks 261015-tz2 / 261016-1de), every v0.5 editor capability
survives the move, and the prototype is retired. `generate` / `sync` output and the committed
fixtures stay byte-identical; the only file-format change is the divider / economy banner.

- [x] **Phase 29: Workspace & Asset Tool from the Root** - One Cargo workspace; `ff-art` runs from the repo root without being built with `ff` (completed 2026-10-16)
- [x] **Phase 30: Skinned Overlay Shell** - The new left-docked, game-styled window with four tabs, bottom bar, status line and skinned popups, carrying today's RULES / CONFIG content (completed 2026-10-16)
- [x] **Phase 31: RULES Tab on the Skin** - Rule list and full editor redrawn with skin controls; + Add rule inserts at the top (completed 2026-10-16)
- [ ] **Phase 32: Rule Dividers & Economy Banner** - `# ff:section` banners become named, collapsible sections; the economy section becomes the locked last divider with banner markers
- [ ] **Phase 33: Style Table & Line-by-Line Style Editor** - CONFIG style table with column headers, a rule-editor-like style editor and a skinned colour picker
- [ ] **Phase 34: ECONOMY & ABOUT Tabs** - League / sync info, SYNC ECONOMY, add/remove economy section and the tiers editor on ECONOMY; identity and credits on ABOUT
- [ ] **Phase 35: Prototype Retirement & Docs** - `ff-proto` and `overlay::theme` removed; README / CLAUDE.md / CHANGELOG updated; version 0.6.0

## Phase Details

### Phase 29: Workspace & Asset Tool from the Root

**Goal**: The repo is one Cargo workspace in which `ff` builds, runs and tests exactly as before, and the asset tool runs from the repo root
**Depends on**: Nothing (first phase of v0.6; independent of the UI work)
**Requirements**: WS-01, WS-02
**Success Criteria** (what must be TRUE):

  1. `cargo build`, `cargo run` and `cargo test` at the repo root build / launch / test `ff` as in v0.5 and do not compile `ff-art` or its nightly-only dependencies
  2. `cargo run -p ff-art -- <cmd>` and the short `cargo ff-art <cmd>` alias both work from the repo root without changing directory
  3. `ff-art extract` run from any working directory writes into the repo's `assets/poe2/` (output paths resolve against the repo root), leaving the committed assets byte-identical when the game files are unchanged

**Plans**: 1 plan
Plans:

- [x] 29-01-PLAN.md — Root Cargo workspace (ff default member), committed lock, nightly pin, `cargo ff-art` alias, repo-root paths, docs

### Phase 30: Skinned Overlay Shell

**Goal**: Launching `ff` opens the new game-styled window instead of the stripe, with the existing RULES and CONFIG content working inside it and every v0.5 overlay behaviour (hotkey, tray, single instance, in-game reload) intact
**Depends on**: Phase 29
**Requirements**: SHELL-01, SHELL-02, SHELL-03, SHELL-04, SHELL-05, SHELL-06, SHELL-07, SHELL-08
**Success Criteria** (what must be TRUE):

  1. `ff` opens a borderless 660pt-wide window docked to the left edge at full work-area height of the monitor the game is on (primary monitor without the game), correctly sized at 100-200% display scale, with the Ferris background behind scrolling content and the game-art header showing "FerrisFilter"; its close button hides it
  2. The RULES / CONFIG / ECONOMY / ABOUT tabs sit like the game's Options tabs and switch content; the v0.5 RULES and CONFIG editors work inside the new window, and a tab label shows ` *` when another tab has unapplied edits
  3. REVERT / RELOAD / APPLY sit in the bottom bar beside one status line that is cut with "…" when too long, shows the full text on hover, and is coloured ok / warning / error; every message from the old outcome area appears there
  4. Every confirmation (changed-on-disk RELOAD / OVERWRITE, switch filter and discard, Quit with unapplied edits, deletes) appears as a skinned popup, and standing captions / wall-of-text panels are replaced by controls, icons, tooltips or the status line
  5. `Ctrl+Shift+Z`, the tray icon and Quit, single-instance relaunch and `/reloaditemfilter` after Apply behave exactly as in v0.5

**Plans**: 5 plansPlans:
**Wave 1**

- [x] 30-01-PLAN.md — Skin + Win32 foundations: monitor-aware DPI-correct dock, button/tab/status/header extensions, global restyle + theme re-point, ff_modal popup, painted flag icons

**Wave 2** *(blocked on Wave 1 completion)*

- [x] 30-02-PLAN.md — Shell composition: docked 660pt window, header, Rules/Config/Economy/About tabs, bottom bar + status line (priority fn), Economy/About placeholders, debug screenshot mode

**Wave 3** *(blocked on Wave 2 completion)*

- [x] 30-03-PLAN.md — All seven confirmations as skinned popups (changed-on-disk, switch filter, quit, delete rule/style, remove economy, discard config)

**Wave 4** *(blocked on Wave 3 completion)*

- [x] 30-04-PLAN.md — Less-texty pass: row flags to icons, captions to tooltips, short empty states

**Wave 5** *(blocked on Wave 4 completion)*

- [x] 30-05-PLAN.md — Docs (CLAUDE.md, README, CHANGELOG), viewed screenshot set, 30-LESS-TEXTY-INVENTORY review checklist

**UI hint**: yes

### Phase 31: RULES Tab on the Skin

**Goal**: The rule list and the expanded rule editor are drawn entirely with the skin, keep every v0.5 editing capability, and new rules land at the top of the file
**Depends on**: Phase 30
**Requirements**: RULES-01, RULES-02, RULES-03
**Success Criteria** (what must be TRUE):

  1. Rule rows (round enable check, Show/Hide, name, condition summary, flags, reorder arrows), search and + Add rule are drawn with skin widgets, and scrolling / searching the ~500-block NeverSink/FilterBlade sample stays responsive
  2. Every v0.5 editing path still works through skin controls: all conditions and actions, operators, Between, Defences / Simple Defences, grouped rules, raw rows, the style picker, Continue, duplicate and delete; an untouched filter still round-trips byte-for-byte
  3. + Add rule inserts the new empty rule as the first block of the file (ahead of any divider), not before the economy section

**Plans:** 7/7 plans complete
Plans:
**Wave 1**

- [x] 31-02-PLAN.md — Skin additions: skin::of, icon fixes, viewport scroll area, two-column field row, chip, X button, id text field, coloured dropdown

**Wave 2** *(blocked on Wave 1 completion)*

- [x] 31-01-PLAN.md — + Add rule inserts at the top of the file (Document::insert_rule_at_top) and the overlay expands/focuses it

**Wave 3** *(blocked on Wave 2 completion)*

- [x] 31-03-PLAN.md — Toolbar, 52pt collapsed rows, flag cluster and game-scrollbar list (virtualised)

**Wave 4** *(blocked on Wave 3 completion)*

- [x] 31-04-PLAN.md — Legacy wrappers delegate to skin widgets; skin slider + exact-number field

**Wave 5** *(blocked on Wave 4 completion)*

- [x] 31-05-PLAN.md — Two-column expanded editor: card chrome, section buttons, segmented booleans/Show-Hide, Continue, Name, Duplicate/Delete

**Wave 6** *(blocked on Wave 5 completion)*

- [x] 31-06-PLAN.md — Dropdowns, + condition/+ style pickers, class picker, quoted lists, raw/draft fields; no egui-default widgets left

**Wave 7** *(blocked on Wave 6 completion)*

- [x] 31-07-PLAN.md — Screenshot --expand/--search, viewed shots, CLAUDE.md + CHANGELOG, capability checklist

**UI hint**: yes

### Phase 32: Rule Dividers & Economy Banner

**Goal**: Users can organise a filter into named, collapsible sections written as readable `#` banners, and the economy section shows as a locked last divider using the same banner format
**Depends on**: Phase 31
**Requirements**: SECT-01, SECT-02, SECT-03, SECT-04, SECT-05
**Success Criteria** (what must be TRUE):

  1. A filter containing the three-line `# ff:section <name>` banner (80 `#` per rule line) loads with each banner as a named section header; rules above the first banner have no header, and foreign comment banners stay ordinary comments
  2. User can add, rename, move and delete a divider (deleting keeps its rules); each change is written only by Apply and undone by Revert, and bytes outside the edited lines are unchanged
  3. User can collapse and expand a section; the state survives a restart (stored per filter in `ff-settings.json`, never in the filter); search still finds matches inside collapsed sections, and reordering and grouped rules work across section boundaries
  4. The economy section shows as a locked "Economy" divider at the end of the list (collapsed by default, read-only rows, collapse/expand only) instead of the `+ N locked rule blocks` footer; `ff-cli apply --add-economy` / `--remove-economy` append / remove a begin-only economy banner at the end of the file (the section runs to the next banner or EOF), Apply regenerates the blocks under it, and old one-line `# ff:economy-begin/end` markers are no longer recognised. (Between this phase and Phase 34 the overlay has no add/remove control; it arrives on the ECONOMY tab.)
  5. `generate` output and the committed fixtures (`seed-universal.filter`, `artifact_guard`, `generate_snapshot`) are unchanged

**Plans**: 6 plansPlans:
**Wave 1**

- [x] 32-01-PLAN.md — Filter model: three-line `# ff:section` / begin-only economy banners, parser fences, old markers dropped, v0.4 conversion retargeted, tests migrated

**Wave 2** *(blocked on Wave 1 completion)*

- [x] 32-02-PLAN.md — Foundations: per-filter collapsed sections in ff-settings.json, mixed-height list offsets, chevron/rename icons
- [x] 32-03-PLAN.md — Divider edits on Document (insert top/above, rename, remove, move over groups, rule hops) and + Add rule above every divider

**Wave 3** *(blocked on Wave 2 completion)*

- [x] 32-04-PLAN.md — Rule model: sections, read-only economy rows, display entries with collapse/search, section actions, WR-02 set

**Wave 4** *(blocked on Wave 3 completion)*

- [x] 32-05-PLAN.md — RULES tab UI: section headers, locked Economy section, + Add section, Section above, collapse persistence

**Wave 5** *(blocked on Wave 4 completion)*

- [ ] 32-06-PLAN.md — --rename-section shot flag, verification shots, CLAUDE.md / CHANGELOG, verification checklist

**UI hint**: yes

### Phase 33: Style Table & Line-by-Line Style Editor

**Goal**: Users can read and edit styles on the CONFIG tab through an aligned table and an inline, line-by-line editor with a game-styled colour picker, under the v0.5 Apply / Revert model
**Depends on**: Phase 30 (and Phase 31 for the colour picker on rule colour actions)
**Requirements**: STY-01, STY-02, STY-03, STY-04, STY-05
**Success Criteria** (what must be TRUE):

  1. The Built-in and Custom style tables have a column header row (Visibility / Name / Icon / Preview / Order), built-in rows show greyed Order arrows so columns line up, and no textual Show/Hide caption remains before the icon
  2. Clicking a style expands a line-by-line editor without boxed panels: `Visibility <Show><Hide>` on one line, the label preview with Text / Border / Background swatches, font size, minimap (size, colour, game-icon shape), beam (colour, Temp), sound and volume
  3. Clicking any colour swatch, in the style editor or on a rule's colour action, opens a skinned colour-picker popup with a hue/saturation gradient, exact RGB/hex input and alpha where the target supports it
  4. Custom styles can still be renamed, reordered and deleted; Delete sits at the bottom of the expanded editor and confirms in a popup showing how many rules use the style
  5. Every style and tier edit stays in memory until the one Apply, is written all or none with the filter, and is undone by Revert

**Plans**: TBD
**UI hint**: yes

### Phase 34: ECONOMY & ABOUT Tabs

**Goal**: The ECONOMY tab is the home for everything economy (sync, the economy section, tier cutoffs) and the ABOUT tab tells users what the app is and who owns the art
**Depends on**: Phase 32 (economy banner markers), Phase 33 (tiers editor leaves CONFIG)
**Requirements**: ECO-01, ECO-02, ABOUT-01
**Success Criteria** (what must be TRUE):

  1. The ECONOMY tab shows the pinned league, the last sync time and the number of priced items; SYNC ECONOMY lives there (gone from the RULES footer), runs without freezing the window and writes only `prices.json`
  2. The ECONOMY tab's Add economy section / Remove economy section control adds or removes the Phase 32 banner-format markers in the loaded filter as an in-memory edit, written by the shared Apply and undone by Revert
  3. The tier cutoffs editor sits on the ECONOMY tab (no longer on CONFIG) with inline validation, applied by the shared Apply
  4. The ABOUT tab shows, top to bottom, the app name, one-line description, a small happy orange Ferris, the Cargo version, the GitHub link, a divider, empty space, then the GGG non-affiliation / ownership disclaimer and the smaller third-party credits (GGG art and fonts, Fontin by exljbris, Ferris by Karen Rustad Tölva)

**Plans**: TBD
**UI hint**: yes

### Phase 35: Prototype Retirement & Docs

**Goal**: The skin is the only overlay look, the prototype is gone, and the docs describe what v0.6 ships
**Depends on**: Phase 34
**Requirements**: CLEAN-01, CLEAN-02
**Success Criteria** (what must be TRUE):

  1. `ff-proto` and the old `overlay::theme` stripe styling no longer exist, the build and full test suite stay green, and `ff-art` plus the committed `assets/` remain
  2. README, CLAUDE.md and CHANGELOG describe the new window and tabs, dividers and the economy banner, the workspace and running the asset tool from the root
  3. The Cargo version reads `0.6.0` when the milestone is tagged

**Plans**: TBD

## Progress

**Execution Order:** 29 → 30 → 31 → 32 → 33 → 34 → 35

| Phase | Milestone | Plans Complete | Status | Completed |
|-------|-----------|----------------|--------|-----------|
| 29. Workspace & Asset Tool from the Root | v0.6 | 1/1 | Complete   | 2026-10-16 |
| 30. Skinned Overlay Shell | v0.6 | 5/5 | Complete   | 2026-10-16 |
| 31. RULES Tab on the Skin | v0.6 | 7/7 | Complete   | 2026-10-16 |
| 32. Rule Dividers & Economy Banner | v0.6 | 5/6 | In Progress|  |
| 33. Style Table & Line-by-Line Style Editor | v0.6 | 0/TBD | Not started | - |
| 34. ECONOMY & ABOUT Tabs | v0.6 | 0/TBD | Not started | - |
| 35. Prototype Retirement & Docs | v0.6 | 0/TBD | Not started | - |

## Backlog

### Phase 999.1: Config-driven styles — promoted to Phase 17 (v0.5)

### Phase 999.2: Overlay single-instance guard — promoted to Phase 22 (v0.5)

### Phase 999.3: Filter load info message — online filters only (BACKLOG, deferred)

**Status (2026-10-12):** deferred. The in-game load message **is only possible for online
filters**, i.e. filters stored on the player's pathofexile.com account. FerrisFilter's local
`.filter` output cannot show it (see the spike results below). This item stays parked until we decide to
publish filters to the account, which is a later, outward-facing feature (see "Next spikes"). The
economy-overview half of the original idea moved to the overlay (999.4) and no longer depends on
this item.

**Goal:** Show an info message when the filter is loaded in-game, the way FilterBlade does
(observed 2026-10-11: `Name: filterblade` / `Version: 0.10.4.2026.283.13` / `Last update: 11 October
2026 at 04:29 PM`), by publishing FerrisFilter output as an online filter. Message content should
come from configuration (see 999.8).

**Research (2026-10-11).** The same FilterBlade filter was installed in two ways:
`OnlineFilters/pgWoKpuQ` (a GGG online-filter subscription, which *shows* the message) and
`filterblade.filter` (downloaded into the game folder by hand, which *does not*). Apart from
trivial comment noise the filter bodies are the same. The message is not a filter directive. It
comes from a 10-line metadata header that the game's online-filter sync puts at the top of the
file:

```
#Online Item Filter
#name:filterblade
#version:0.5.5e
#realm:poe2
#errors:
#hash:f6b696a2a8858865309b37cddab4e40a
#filterVersion:0.10.4.2026.283.13
#filterType:Normal
#lastUpdate:2026-10-11T17:15:26Z
#monthTimeStamp:1522
```

The header lines end in LF and the body uses CRLF. `#hash` is the **MD5 of the body bytes as-is**
(everything after line 10, CRLF kept); this was verified to match. The online files have no `.filter`
extension and are named by the online filter's id.

**Spike results (2026-10-12, author-tested in-game).** The test filters were the current
`dynamic.filter` with a FerrisFilter version of the online header and a correct body hash.

- **The offline header is ignored.** `ff-header-test.filter`, `ff-badhash-test.filter` (wrong hash)
  and `ff-freetext-test.filter` (free text in `#name`/`#filterVersion`/`#errors`) all appeared in
  the dropdown and loaded with only the stock "Item Filter loaded successfully." message. An offline
  file cannot produce the load message, whatever its header says.

- **`OnlineFilters/` is a cache, not a source.** Two hand-written files placed there (`ffTEST01`,
  `ffTEST02`) never appeared in the dropdown, and the game left them untouched. The online list
  comes from the account's filters on the server; the folder only holds downloaded copies.

- **Local edits to a subscribed online filter are overwritten.** The header of the real FilterBlade
  copy (`OnlineFilters/pgWoKpuQ`) was rewritten with marker values (`#name`, `#filterVersion`,
  `#errors`, `#filterType`, a 2030 `#lastUpdate`), with the body and its hash left intact. When the
  filter was loaded, the game replaced the file with the server's version, byte-for-byte. So the
  game checks against the server, not against `#hash`, and the local file is not something we can
  patch.

**Consequence:** the load message is only available to filters that really are **online**, meaning
filters stored on the player's pathofexile.com account. Getting it means publishing the
FerrisFilter output to the account instead of (or as well as) writing a local file.

**Next spikes (unverified):**

1. Cheap manual check: create an online filter on pathofexile.com by hand, paste a FerrisFilter
   output into it, subscribe in-game, and confirm the message shows. Also note which displayed
   fields map to which values entered on the site (name/version/last update), whether any can carry
   free text (that would let a future version echo 999.4's economy summary in-game), and whether the
   `CustomAlertSound "uwu.mp3"` still works from an online filter.

2. Automated path: GGG's developer API appears to expose item-filter create/update endpoints under
   an OAuth scope (FilterBlade's auto-updater is believed to use this). Confirm the endpoints, the
   scope, and what app registration GGG requires. This would replace `install`/`apply`'s local copy
   with an account upload, which is outward-facing and needs explicit opt-in. Also check
   whether there are rate limits, or limits on how many filters an account can hold.

**Requirements:** TBD
**Plans:** 5/5 plans complete

Plans:

- [ ] TBD (promote with /gsd-review-backlog when ready)

### Phase 999.4: Economy change summary in the overlay (BACKLOG)

**Goal:** Show a short overview in the **overlay** of how the economy changed since the last sync,
either per item ("Cultivation Orb up x% since last update") or aggregated by category ("Temple
Currencies up x%"). It is overlay-only and does **not** appear in-game: the in-game load message
is limited to online filters (999.3), so this no longer depends on it. A natural home is
beside **SYNC ECONOMY**, shown after a successful sync. This needs a previous price snapshot to
diff against: `sync` currently overwrites `prices.json`, so it has to keep the prior snapshot (or
a history). Open questions: per-item vs per-category granularity, how much to show before it
becomes noise, the threshold for "notable" movement, and whether tier changes (an item moving
from B to A) are more useful to report than raw percentages.
**Requirements:** TBD
**Plans:** TBD

Plans:

- [ ] TBD (promote with /gsd-review-backlog when ready)

### Phase 999.5: Drop `dynamic.json`; styles move to `styles.json` — promoted to Phases 17-18 (v0.5)

### Phase 999.6: Parse and edit arbitrary filters — promoted to Phases 16, 19-20 (v0.5)

### Phase 999.7: Unmaintained (locked) rules and layering over a base filter (BACKLOG)

**Goal:** When FerrisFilter is pointed at an existing filter, the user can mark a set of rules
as "unmaintained": FF never modifies them and the overlay doesn't show them, so they can't be
edited by accident. The user can still add their own rules on top. A likely approach is file
inclusion: the base filter is `Import`ed untouched, and FF writes only the user's own file, with
the user's blocks placed before the import so they win under first-match. PoE2 does support
`Import` (per the author), but research the details before implementing: path resolution, the
`Optional` flag, nesting, and how imported blocks interact with `Continue`. Locked regions inside
a single emitted file are the fallback. Depends on 999.6 for reading the base filter.
**Requirements:** TBD
**Plans:** TBD

Plans:

- [ ] TBD (promote with /gsd-review-backlog when ready)

### Phase 999.8: Overlay configuration tab — partially promoted (BACKLOG)

**Promoted to Phase 21 (v0.5):** the configuration tab itself, the loot styles in `styles.json`
and the economy tier cutoffs (`tiers.json`).

**Remaining goal:** configuration of the load message (999.3, once online publishing exists) and of
the overlay economy summary (999.4). Edits should follow the overlay's existing model: held in
memory, persisted only on Apply, and discardable with Revert.

**Requirements:** TBD
**Plans:** TBD

Plans:

- [ ] TBD (promote with /gsd-review-backlog when ready)

### Phase 999.9: Starting-point filter presets (BACKLOG)

**Goal:** Offer preset filters as starting points, chosen from a few questions about where the
player is in the game (campaign / early maps / late endgame, possibly league-start vs mid-league)
and what they are playing (class/build archetype → weapon and armour classes, defence type). The
answers produce an initial filter that the user then edits as normal. The `ssf_*_config.json`
experiments already in the working tree are informal presets of this kind. Open questions: how
many questions before it becomes a chore, whether presets are full filters or layers over a base
(see 999.7), and where the question flow lives (overlay first run, CLI, or both).

**Requirements:** TBD
**Plans:** TBD

Plans:

- [ ] TBD (promote with /gsd-review-backlog when ready)

### Phase 999.10: Raw-text block editing (BACKLOG)

**Goal:** An "Edit as text" escape hatch on a rule in the overlay: edit the block's raw filter text
directly, with the edit validated by the Phase 16 parser before it's accepted (an enabled block must
always be valid grammar). Proposed during v0.5 discussion (Phase 20, 2026-10-12) and deferred by the
author. v0.5 covers the full grammar with structured controls, so this is for power users and
odd cases.
**Requirements:** TBD
**Plans:** TBD

Plans:

- [ ] TBD (promote with /gsd-review-backlog when ready)

### Phase 999.11: Full overlay UI review — less "texty" (BACKLOG)

**Goal:** A full visual and UX review of the overlay (RULES and CONFIG tabs, row headers, flags,
status/outcome area, captions, confirmations). The author's main complaint after using v0.5
(2026-10-13): parts of the UI read as a wall of text. Replace prose with visual affordances where
possible: icons/chips instead of sentences, tooltips or on-demand help instead of standing captions,
terser status lines, clearer visual hierarchy. Starting points: the per-phase UI reviews
(19/20/21-UI-REVIEW.md, each 18/24, with Typography and Spacing the weakest pillars). Deferred by the
author: v0.5 focuses on usability and minor fixes, not a visual redesign.
**Requirements:** TBD
**Plans:** TBD

Plans:

- [ ] TBD (promote with /gsd-review-backlog when ready)

### Phase 999.12: Keyword table: verify remaining keywords in game (BACKLOG)

**Goal:** The Phase 16 keyword-table sign-off was a partial pass (author's UAT, 2026-10-15): the
keywords not yet exercised in game should be checked against the live game and corrected in
`src/filter/keywords.rs`. Also unconfirmed in game: PoE1-only lines (e.g. `LinkedSockets`,
`SocketGroup`, `HasInfluence`) loading as raw rows without blocking Apply was verified in tests
only; the v0.5 UAT (2026-10-15) recorded it as partial.
**Requirements:** TBD
**Plans:** TBD

Plans:

- [ ] TBD (promote with /gsd-review-backlog when ready)

### Phase 999.13: Guide rules away from conditions that cannot match (BACKLOG)

**Goal:** Offer guidance (not blocking) when a rule combines conditions that can never match, e.g.
Defences or Simple Defences on a rule without armour classes (noted in Phase 27: UAT4-03 has no class
check).
**Requirements:** TBD
**Plans:** TBD

Plans:

- [ ] TBD (promote with /gsd-review-backlog when ready)

### Phase 999.14: Minimal support review for PoE2 (BACKLOG)

**Goal:** A general review of `Minimal` blocks in PoE2: check in game whether `Minimal` is valid
outside Ruthless and what it renders, then decide how far FerrisFilter supports it. Today a foreign
`Minimal` block loads and is kept as is (shown as Minimal, Custom only), but a `styles.json` style
with `"action": "Minimal"` fails to load, rejecting the whole file (Phase 24 review WR-02, against
the UAT-04 spec "keep loading it"; deferred here 2026-10-15). If
Minimal is valid in PoE2, allow it as a style action (loader, overlay switch, generator); if not,
keep rejecting it with a clear message.
**Requirements:** TBD
**Plans:** TBD

Plans:

- [ ] TBD (promote with /gsd-review-backlog when ready)

### Phase 999.15: Sound preview in the style editor (BACKLOG)

**Goal:** A small play icon beside the style editor's sound control previews the selected sound:
the game's alert sounds 1-16 (extracted by `ff-art` from the install) and a custom sound file such
as `uwu.mp3` from the game folder, at the chosen volume. Needs an audio playback dependency in `ff`
(e.g. rodio). Dropped from v0.6 by the author on 2026-10-16.
**Requirements:** TBD
**Plans:** TBD

Plans:

- [ ] TBD (promote with /gsd-review-backlog when ready)

### Phase 999.16: Move rules more than a few positions at once (BACKLOG)

**Goal:** Reordering with the up/down arrows is painful past about five positions. Revisit rule
placement: drag and drop, move to top/bottom, or move into a named section (v0.6 dividers), and
reconsider where + Add rule inserts (v0.6 inserts at the top of the file). Raised by the author on
2026-10-16.
**Requirements:** TBD
**Plans:** TBD

Plans:

- [ ] TBD (promote with /gsd-review-backlog when ready)
