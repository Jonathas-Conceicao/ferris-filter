# Phase 34: ECONOMY & ABOUT Tabs - Context

**Gathered:** 2026-10-16
**Status:** Ready for planning

<domain>
## Phase Boundary

Real ECONOMY and ABOUT tabs: ECONOMY holds sync status, SYNC ECONOMY, the Add / Remove economy
section control and the TIERS cutoffs (moved from CONFIG); ABOUT shows identity, version, repo,
disclaimer and credits.

</domain>

<decisions>
## Implementation Decisions

### ECONOMY
- Rows: League (pinned `sync::OVERLAY_LEAGUE`), Last sync (relative, e.g. "3 h ago", exact time on
  hover), Priced items, Price source ("Fallback prices" when `prices.json` is missing / empty — from
  the deferred "surface price_source" item), then SYNC ECONOMY (worker thread, writes only
  `prices.json`).
- One Add economy section / Remove economy section button (label follows state). Remove asks for
  confirmation (skinned popup). Add always appends the begin-only economy banner at the **end of the
  file** (Phase 32 format). Both are in-memory edits written by the shared Apply, undone by Revert.
- The TIERS cutoffs editor moves here from CONFIG with its inline validation, same Apply / Revert
  model.

### ABOUT
- Top to bottom: app name, one-line description, a small happy orange Ferris, version (from Cargo,
  `env!("CARGO_PKG_VERSION")`), GitHub link (opens in the browser), a divider line, empty space down
  to the bottom, then the ownership disclaimer ("This product isn't affiliated with or endorsed by
  Grinding Gear Games in any way." + the game art and fonts belong to their owners; FerrisFilter is a
  free third-party tool for PoE2 players), then the third-party credits in smaller text after it
  (GGG art and fonts, Fontin by exljbris, Ferris by Karen Rustad Tölva, public domain), so they are
  not confused with ownership of the tool.
- Orange Ferris: `ff-art` renders `assets/ferris/rustacean-flat-happy.svg` in its original colours to
  a small committed PNG (e.g. `assets/ui/ferris-about.png`).

### Claude's Discretion
- Spacing, sizes, relative-time wording.

</decisions>

<code_context>
## Existing Code Insights

- `src/overlay/economy.rs` (sync worker); SYNC ECONOMY currently sits in the RULES footer; the tiers
  editor lives in `config_view.rs` / `config_state.rs`; prices metadata (league, timestamp) in
  `prices.json`.
- Prototype ECONOMY / ABOUT tabs in `src/bin/ff-proto.rs` (fake data).
- `tools/ff-art` `ferris` subcommand renders the pale background; extend it for the colour variant.

</code_context>

<specifics>
## Specific Ideas

- Author-specified ABOUT order: appname, description, art, version, github info, dividing line,
  large empty space until the bottom, ownership disclaimer, credit to other stuff.

</specifics>

<deferred>
## Deferred Ideas

- League picker (still pinned) — deferred list.

</deferred>
