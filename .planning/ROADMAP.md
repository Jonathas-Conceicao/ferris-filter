# Roadmap: FerrisFilter

## Milestones

- ✅ **v0.1 Initial Release** — Phases 1-4 (shipped 2026-05-28) — [archive](milestones/v0.1-ROADMAP.md)
- ✅ **v0.2 Declarative Rule Model** — Phases 5-7 (shipped 2026-06-12) — [archive](milestones/v0.2-ROADMAP.md)
- ✅ **v0.3 Economy Items & Exceptional Gear** — Phases 8-10 (shipped 2026-06-27) — [archive](milestones/v0.3-ROADMAP.md)
- ✅ **v0.4 Overlay Filter Editor Rework** — Phases 11-15 (shipped 2026-09-17) — [archive](milestones/v0.4-ROADMAP.md)
- 🚧 **v0.5 Generic Filter Editor & Initial App Configuration** — Phases 16-22 (in progress)

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

## v0.5 Generic Filter Editor & Initial App Configuration (Phases 16-22)

**Milestone goal:** FerrisFilter can open, edit and save any working PoE2 filter, not just its own
output, with the style palette and economy tier cutoffs configurable from the overlay.

- [ ] **Phase 16: Lossless Filter Parser** - Parse any PoE2 filter and write it back byte-identical
- [x] **Phase 17: Config-Driven Styles** - Named styles live in `styles.json`; `generate` output unchanged (completed 2026-10-13)
- [ ] **Phase 18: Filter as Store & Economy Section** - The `.filter` file is the store; fenced, regenerable economy section (executed 2026-10-13; awaiting human UAT)
- [ ] **Phase 19: Overlay Rule List Over Parsed Blocks** - Open, search, reorder, enable/disable and Apply any filter's blocks
- [ ] **Phase 20: Full-Grammar Block Editing** - Edit every condition and action, and apply named styles
- [ ] **Phase 21: Overlay Configuration Tab** - Edit styles and tier cutoffs, and pick the filter, from the overlay
- [ ] **Phase 22: Single-Instance Overlay** - A second `ff` launch surfaces the running stripe

### Phase 16: Lossless Filter Parser

**Goal**: Any working PoE2 filter can be parsed and written back without changing a byte the user did not edit
**Depends on**: Nothing (first phase)
**Requirements**: PARSE-01, PARSE-02, PARSE-03, PARSE-04
**Success Criteria** (what must be TRUE):

  1. The real FilterBlade/NeverSink filter, the `reference-old-filters/` files, poe2filter.com exports and a FerrisFilter-generated filter all open without error; an unrecognized keyword is kept as an opaque line instead of failing the load
  2. Every block type, condition, operator and action documented at pathofexile.com/item-filter/about is recognized, including `Continue`, `Import` and `Minimal`
  3. Parsing a filter and saving it with no edits writes a byte-identical file, for CRLF, LF, mixed endings, a BOM and a missing final newline, and for the real FilterBlade sample (the phase exit gate)
  4. Editing one block and saving changes only that block's lines; every other byte of the file is untouched

**Plans**: 4 plans
Plans:
**Wave 1**

- [ ] 16-01-PLAN.md — Raw-line document model + byte-identical serializer; FilterBlade fixture committed under a scoped `-text` rule

**Wave 2** *(blocked on Wave 1 completion)*

- [ ] 16-02-PLAN.md — The single GGG keyword table and quote-aware line tokenizer/shape validator

**Wave 3** *(blocked on Wave 2 completion)*

- [ ] 16-03-PLAN.md — Block index (stable BlockIds, attached comments, `#ff:` metadata, disabled-block classification) + real-filter corpus gate

**Wave 4** *(blocked on Wave 3 completion)*

- [ ] 16-04-PLAN.md — Block-scoped edit API (enable/disable, replace lines, set_meta) proving PARSE-04 + CHANGELOG

### Phase 17: Config-Driven Styles

**Goal**: Named styles are data in a committed `styles.json`, not a hardcoded vocabulary, and the static `generate` path is unaffected
**Depends on**: Nothing (independent of Phase 16; can run in parallel with it)
**Requirements**: STYLE-01, STYLE-02, STYLE-04
**Success Criteria** (what must be TRUE):

  1. User adds a new named style to `styles.json`, uses it from a rule, and `generate` emits it with no code change
  2. A rule naming an unknown style, or an invalid `styles.json`, stops with a clear error at load time that names the problem
  3. `generate` output is unchanged: `artifact_guard` and `generate_snapshot` pass with the `seed-universal.filter` fixture byte-identical and not re-baselined

**Plans**: 4 plans
Plans:
**Wave 1**

- [x] 17-01-PLAN.md — styles.json (today's 12 styles), ordered StyleSet + StyleError validation, load_styles, equality test vs the old struct; harnesses copy styles.json

**Wave 2** *(blocked on Wave 1 completion)*

- [x] 17-02-PLAN.md — Emitter and overlay picker resolve styles by name from StyleSet; hardcoded Styles struct deleted; Hide-style/new-style behavior locked by tests

**Wave 3** *(blocked on Wave 2 completion)*

- [x] 17-03-PLAN.md — styles.json becomes the single source: config.json/dynamic_base.json lose styles; load-time rule-reference check; inline styles section rejected

**Wave 4** *(blocked on Wave 3 completion)*

- [x] 17-04-PLAN.md — --styles on generate/apply, end-to-end success-criteria tests, CLAUDE.md + CHANGELOG

### Phase 18: Filter as Store & Economy Section

**Goal**: The filter file is the only store, and the economy-priced blocks live in a fenced section FerrisFilter can regenerate without touching anything else
**Depends on**: Phase 16, Phase 17
**Requirements**: STORE-01, STORE-03, STORE-04, STORE-05, ECON-01, ECON-03, ECON-04, EDIT-09, EDIT-10
**Success Criteria** (what must be TRUE):

  1. By default `apply` and the overlay's Apply edit the game-folder `dynamic.filter` directly; `dynamic.json`, `dynamic_base.json`, the overlay JSON store and the `apply --reset/--base/--store` flags are gone, and the breaking change is in `CHANGELOG.md`
  2. `ff` launched from anywhere, including a double-click on the exe, finds `styles.json`, `tiers.json` and `prices.json` without relying on the working directory
  3. Apply regenerates the economy section, placed after the general rules between comment markers, from `prices.json`, `tiers.json` and the styles, and changes no byte outside the markers; re-applying with nothing new produces identical bytes
  4. User can add an economy section to a new empty filter or to a foreign filter that has none, and can remove it again; a new filter starts empty
  5. Apply refuses to overwrite a filter that changed on disk after it was loaded and says why; the first write to a filter keeps a backup copy of the original

**Plans**: 7 plans
Plans:
**Wave 1**

- [x] 18-01-PLAN.md — App-root resolver (FERRISFILTER_HOME / exe walk-up / cwd) and guarded, backed-up, atomic filter write
- [x] 18-02-PLAN.md — Economy split: emitter::generate_economy over StyleSet and compile::load_economy (generate byte-identical)
- [x] 18-03-PLAN.md — Economy section markers (find/refresh/add/remove, damaged-marker errors) and v0.4 dynamic.filter migration

**Wave 2** *(blocked on Wave 1 completion)*

- [x] 18-04-PLAN.md — dynamic.rs filter-as-store API: LoadedFilter, ApplyOp, economy_body, apply_file

**Wave 3** *(blocked on Wave 2 completion)*

- [x] 18-05-PLAN.md — CLI apply --filter/--add-economy/--remove-economy; obsolete flags removed; hermetic integration tests

**Wave 4** *(blocked on Wave 3 completion)*

- [x] 18-06-PLAN.md — Overlay re-pointed at the app root and loaded filter; JSON store deleted

**Wave 5** *(blocked on Wave 4 completion)*

- [x] 18-07-PLAN.md — Delete legacy JSON-store API, move dynamic_base.json to a fixture, .gitignore/CLAUDE.md/CHANGELOG

### Phase 19: Overlay Rule List Over Parsed Blocks

**Goal**: Users can open any filter in the overlay and manage its blocks as a rule list, with the economy section kept out of the way
**Depends on**: Phase 18
**Requirements**: EDIT-03, EDIT-04, EDIT-05, EDIT-07, EDIT-08, ECON-02
**Success Criteria** (what must be TRUE):

  1. The overlay lists every block of a loaded filter, including NeverSink's roughly 460 blocks, stays responsive while scrolling, and a search box narrows the list
  2. User can add, delete, duplicate and reorder rules, and switch a block between `Show`, `Hide` and `Minimal` and toggle `Continue`
  3. Disabling a rule comments its block out; commented-out blocks in a loaded filter appear as disabled rules and can be enabled
  4. The economy section does not appear as rules; the list shows a summary such as "+ 42 locked rule blocks"
  5. Edits stay in memory until Apply writes the filter; Revert discards unsaved edits

**Plans**: 7 plans
Plans:
**Wave 1**

- [x] 19-01-PLAN.md — Standalone cargo fmt commit; delete the v0.4 JSON-rule overlay modules (keep parse_base_type_entry)

**Wave 2** *(blocked on Wave 1 completion)*

- [x] 19-02-PLAN.md — filter::Document structural ops: append before economy, remove, duplicate, move, Show/Hide/Minimal, Continue
- [x] 19-03-PLAN.md — dynamic::apply_edited (EconomyOp, rules-only Apply) and the Error-only blocking_errors gate

**Wave 3** *(blocked on Wave 2 completion)*

- [x] 19-04-PLAN.md — Pure overlay::rules WorkingFilter (row cache, labels, search, actions, Revert, staged economy), virtual_list math, 511-block perf test

**Wave 4** *(blocked on Wave 3 completion)*

- [x] 19-05-PLAN.md — overlay::rule_list egui view: virtualized rows, flags, expanded body, search bar, economy footer row

**Wave 5** *(blocked on Wave 4 completion)*

- [x] 19-06-PLAN.md — Wire the rule list into the stripe; Apply/Revert over the edited document; refusal lines; Phase 18 notice and ADD ECONOMY SECTION button removed

**Wave 6** *(blocked on Wave 5 completion)*

- [ ] 19-07-PLAN.md — CLAUDE.md and CHANGELOG for the rule list; phase-end gates

**UI hint**: yes

### Phase 20: Full-Grammar Block Editing

**Goal**: Users can edit every condition and action of any block, and apply named styles that stay linked to `styles.json`
**Depends on**: Phase 19
**Requirements**: EDIT-01, EDIT-02, EDIT-06, STYLE-03
**Success Criteria** (what must be TRUE):

  1. Every condition on any block is viewable and editable with all its operators and multi-value lists; no condition is read-only and no block is "unsupported"
  2. Every action on any block is viewable and editable: colours, font size, built-in and custom alert sounds, minimap icon, beam effect and drop-sound toggles
  3. User applies a named style to a rule and the rule's style actions are set from it
  4. A rule that uses a named style keeps a comment tag in the filter; after the style changes, the rule's actions update on the next Apply
  5. Editing one value rewrites only that value (a `Class "x"` is never turned into `Class == "x"` unless the user changes it)

**Plans**: TBD
**UI hint**: yes

### Phase 21: Overlay Configuration Tab

**Goal**: Users can configure styles, tier cutoffs and the active filter from a configuration tab separate from the rule editor
**Depends on**: Phase 17, Phase 18, Phase 19
**Requirements**: CONF-01, CONF-02, CONF-03, CONF-04, STORE-02
**Success Criteria** (what must be TRUE):

  1. The overlay has a configuration tab, separate from the rule editor
  2. User adds, edits and deletes named styles (text, border and background colour picker, font size, sound, minimap icon, beam); built-in styles cannot be deleted
  3. User edits the economy tier cutoffs in `tiers.json` (not prices); an invalid cutoff shows inline and cannot be applied
  4. The tab has its own Apply and Revert, and Apply writes all of its files or none of them
  5. User selects a different filter in the game folder from this tab and the choice is remembered across launches

**Plans**: TBD
**UI hint**: yes

### Phase 22: Single-Instance Overlay

**Goal**: Launching `ff` twice never produces two stripes
**Depends on**: Nothing (independent of other phases)
**Requirements**: INST-01
**Success Criteria** (what must be TRUE):

  1. Launching `ff` while it is already running brings up the running stripe and the second process exits
  2. This works even when the running stripe is hidden to the tray
  3. Only one stripe window exists and the hotkey keeps working after repeated launches

**Plans**: TBD

### Progress

| Phase | Plans Complete | Status | Completed |
|-------|----------------|--------|-----------|
| 16. Lossless Filter Parser | 0/? | Not started | - |
| 17. Config-Driven Styles | 4/4 | Complete    | 2026-10-13 |
| 18. Filter as Store & Economy Section | 7/7 | Human UAT pending | - |
| 19. Overlay Rule List Over Parsed Blocks | 6/7 | In Progress|  |
| 20. Full-Grammar Block Editing | 0/? | Not started | - |
| 21. Overlay Configuration Tab | 0/? | Not started | - |
| 22. Single-Instance Overlay | 0/? | Not started | - |

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
**Plans:** 6/7 plans executed

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
