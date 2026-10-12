# Roadmap: FerrisFilter

## Milestones

- ✅ **v0.1 Initial Release** — Phases 1-4 (shipped 2026-05-28) — [archive](milestones/v0.1-ROADMAP.md)
- ✅ **v0.2 Declarative Rule Model** — Phases 5-7 (shipped 2026-06-12) — [archive](milestones/v0.2-ROADMAP.md)
- ✅ **v0.3 Economy Items & Exceptional Gear** — Phases 8-10 (shipped 2026-06-27) — [archive](milestones/v0.3-ROADMAP.md)
- ✅ **v0.4 Overlay Filter Editor Rework** — Phases 11-15 (shipped 2026-09-17) — [archive](milestones/v0.4-ROADMAP.md)

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


## Backlog

### Phase 999.1: Config-driven styles (no hardcoded vocabulary) (BACKLOG)

**Goal:** Any config should be able to define and use new named styles without a code change.
Today the style vocabulary is hardcoded in the `Styles` struct (`src/models/config.rs`) and
`emitter::resolve_style` (`src/emitter.rs`) — adding `craftable` (for `ssf_icewolf_config.json`)
required editing both. Replace the named-field `Styles` struct with a name→`Style` map (or keep
S–F/unchanged/hidden/unidentified as the known palette plus an open `extra` map), resolve styles
by lookup instead of a `match`, and validate at load-time that every `rule.style` resolves.
`FUTURE` comments already mark both code locations.
**Requirements:** TBD
**Plans:** 3/4 plans executed

Plans:

- [ ] TBD (promote with /gsd-review-backlog when ready)

### Phase 999.2: Overlay single-instance guard (BACKLOG)

**Goal:** Launching `ff` while it is already running should surface the existing stripe instead of
opening a second one. Discovered by the author during Phase 13 wave-2 verification (2026-09-04):
a second launch opens a second stripe window, and only the first instance owns the `Shift+Z`
registration — the second correctly reports the conflict in its tray tooltip (D-10), so the
failure is visible rather than silent, but two stripes and one working hotkey is not the intended
shape for a tray app that gets double-clicked. Fix shape: acquire a named mutex at startup; if it
is already held, signal the running instance to show itself and exit rather than creating a
window. Small, self-contained, and touches only the overlay entry point.
**Requirements:** TBD
**Plans:** TBD

Plans:

- [ ] TBD (promote with /gsd-review-backlog when ready)

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
**Plans:** TBD

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

### Phase 999.5: Drop `dynamic.json`; styles move to `styles.json` (BACKLOG)

**Goal:** Remove `dynamic.json` (and its `dynamic_base.json` template) as the dynamic filter's
source of truth. The **filter file itself** holds the rules, and it is edited through 999.6's
parse-and-edit support. The other inputs stay as separate files: economy cache `prices.json`, tier
cutoffs `tiers.json`, the `prices-fallback.json`/`tiers-fallback.json` early-league pair, and
`mapping.json` for sync. The named style definitions (the Dracula tier palette plus
`unchanged`/`hidden`/`kite` and custom styles) move into a new **`styles.json` at the repo root**,
so styles stay shared and editable without a rule store.

**Intentional trade-off:** with no rule store, a filter can no longer be fully regenerated from
config. It is edited in place instead. This is the point: it goes together with 999.6, where any
filter (ours or not) can be loaded, edited and used. `config.json` → `universal.filter` (the static
`generate` path and its byte-for-byte fixture) is out of scope unless we decide otherwise. Open
questions: how economy-priced sections are found and refreshed inside an edited filter (markers or
comments around generated blocks?), and whether `styles.json` is split out of `config.json` or
copied from it (relates to 999.1).

**Requirements:** TBD
**Plans:** TBD

Plans:

- [ ] TBD (promote with /gsd-review-backlog when ready)

### Phase 999.6: Parse and edit arbitrary filters (full grammar) (BACKLOG)

**Goal:** Load any working PoE2 filter, not just FerrisFilter output (e.g. FilterBlade or
poe2filter.com exports, or hand-written ones), and edit it. The rule model has to cover the full
filter grammar: every condition, operator, and action, plus `Continue`, `Import`, and
`Minimal`. Anything the editor doesn't understand must still round-trip losslessly instead of
being dropped. This is the foundation for 999.5 and 999.7 and is probably big enough to be its own
milestone. The current `when` vocabulary and named-style model (see 999.1) would have to grow
into, or sit beside, a general block representation.
**Requirements:** TBD
**Plans:** TBD

Plans:

- [ ] TBD (promote with /gsd-review-backlog when ready)

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

### Phase 999.8: Overlay configuration tab (BACKLOG)

**Goal:** Add a configuration tab to the overlay, separate from the rule editor, where users
can configure (a) the load message (999.3, once online publishing exists) and the overlay economy summary (999.4), (b) the loot styles in `styles.json` (the tier
palette and any custom styles; builds on 999.1/999.5 so styles are data rather than a hardcoded
vocabulary), and (c) the economy tiering, meaning the tier cutoffs (`tiers.json`) and not the
prices themselves. Edits should follow the overlay's existing model: held in memory, persisted
only on Apply, and discardable with Revert.

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
