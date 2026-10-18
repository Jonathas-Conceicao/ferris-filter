# Phase 39: League Picker & Price Source - Context

**Gathered:** 2026-10-18
**Status:** Ready for planning

<domain>
## Phase Boundary

The Economy tab lets the user choose the league Sync economy prices against (from poe.ninja's
current list, replacing the pinned `sync::OVERLAY_LEAGUE`) and whether the economy section is priced
Live (synced) or from the committed Fallback pair. Covers LEAGUE-01..LEAGUE-05. Builds on Phase 36
(per-user folder, embedded fallback pair) and reuses Phase 37's refresh button.

</domain>

<decisions>
## Implementation Decisions

### League picker
- The Economy tab's "League (pinned)" row becomes a dropdown of poe.ninja's PoE2 economy leagues,
  from the `index-state` endpoint sync already uses (`economyLeagues`: name, slug, hardcore,
  indexed). A refresh button beside it (Phase 37's) re-fetches the list.
- The list is fetched on a worker thread the first time the Economy tab is opened in a session and
  on refresh; nothing is fetched at start-up.
- Default when nothing is saved: the first league in poe.ninja's list that is neither Standard nor
  Hardcore. The `indexed` flag is not trusted. The league is written to `ff-settings.json` only when
  the user picks one.
- Offline / fetch failed: the picker shows the saved (or default) league name and stays usable for
  Sync; its tooltip says the list could not be loaded.

### League end (LEAGUE-03)
- A saved league no longer in the list: the picker shows "<name> (ended)" in amber, Sync economy is
  disabled, and the status line says "<name> is no longer on poe.ninja; pick a league". Nothing is
  switched automatically.
- Prices already synced for the old league keep working for Apply until the next sync; Last sync
  still shows the league it priced.

### Price source (LEAGUE-04 / LEAGUE-05)
- A `Live | Fallback` segmented switch on the Economy tab; tooltip recommends Fallback for league
  start and non-trade players.
- Saved at once in `ff-settings.json` (like the reload toggle). When the loaded filter has an
  economy section, a change marks unapplied changes, because Apply will write different prices.
- Under Fallback: Sync economy, the league picker, the league refresh button and the Tiers editor
  (with its Reset to defaults) are greyed, each with a tooltip (the fallback tiers are fixed).
- Live with no `prices.json` yet (fresh install): unchanged behaviour, priced from fallback with the
  reason on hover, until the first Sync.

### Defaults and CLI
- New installs start on Live.
- The CLI keeps its own `--league` and automatic resolution (dev tool); it does not read the
  overlay's league setting.

### Claude's Discretion
- Worker extraction (a generic worker from `EconomySync`), settings key names, the exact status
  texts beyond those above, how the "(ended)" state is modelled.

</decisions>

<code_context>
## Existing Code Insights

### Reusable Assets
- `sync::league::INDEX_STATE_URL` / `fetch_index_state` (`src/sync/league.rs:93`); the `EconomySync`
  worker pattern; `PriceSourcePolicy::{Auto, ForceFallback}` (`src/compile.rs:189`).
- `skin::` dropdown (`ff_dropdown_ex`), segmented switch, Phase 37's refresh icon button.

### Established Patterns
- Settings that are not edits save immediately and never mark CONFIG dirty (reload toggle); the
  `styles.json` watcher marks the filter unapplied when Apply output changes.
- Economy tab rows: League, Last sync, Priced items, Price source.

### Integration Points
- `src/overlay/app.rs:2996` passes `crate::sync::OVERLAY_LEAGUE` to the sync worker; delete the
  constant and the CLAUDE.md note about updating it each league.
- The economy rendering / Apply pricing path picks the fallback pair through `PriceSourcePolicy`.

</code_context>

<specifics>
## Specific Ideas

- poe.ninja's `indexed` flag was observed flipping between Forbidden Rites and Runes of Aldur within
  six hours (2026-09-10/11) — the reason the default ignores it.

</specifics>

<deferred>
## Deferred Ideas

None — discussion stayed within phase scope.

</deferred>
