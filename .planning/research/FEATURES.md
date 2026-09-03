# Features Research

**Domain:** Loot-filter rule editor delivered as a game overlay
**Researched:** 2026-09-03
**Confidence:** HIGH (scope came from the author directly; comparables verified)

## Comparable tools

| Tool | Shape | What it does | Relevance |
|------|-------|--------------|-----------|
| **Exiled Exchange 2** | Electron overlay, PoE2 | Clipboard-driven price check, poe.ninja trends, currency exchange, map search, OCR mode | The interaction model the author named: an overlay you summon over the game. Not a filter editor. |
| **Awakened PoE Trade** | Electron overlay, PoE1 | Its predecessor; same shape | Established that overlays are tolerated at scale |
| **FilterBlade** | Web app (NeverSink) | Full filter customization, strictness presets, styling | The dominant filter-editing UX — but web, out-of-game, and preset-oriented (the exact model v0.4 rejects) |
| **poe2filter.com** | Web app | Filter generation | Already prior art in this repo (the old filters and `tiers-fallback.json` came from it) |

**The gap v0.4 fills:** every filter editor is a web app you leave the game to visit, and every
overlay is a price checker. Nobody edits filter rules *from inside the game session*. That is the
differentiator, and it only works because filters are a plain text file the game re-reads on demand.

## Feature categories

### A. Core library extraction — table stakes

| Feature | Complexity | Notes |
|---------|-----------|-------|
| `lib.rs` exporting models/pricer/emitter | Low | Pure refactor, oracle-proven |
| CLI reduced to a thin shell | Low | `generate`/`sync`/`install` behavior unchanged |
| Retire the `seed_prices` duplicated-structs workaround | Low | Only possible once a lib target exists |

### B. Dynamic rule store — table stakes

| Feature | Complexity | Notes |
|---------|-----------|-------|
| `dynamic.json` schema + serde model | Low | Reuses existing `Rule`/`When`; no new condition types |
| Persistence across restarts | Low | Plain file read/write; the author's explicit requirement |
| Compile `dynamic.json` → `dynamic.filter` | Medium | Same `emit_rules`; decide the economy-sections question |
| Atomic write into the game folder | Low | Reuse the `tempfile` discipline from `sync` |
| Headless entry point (before any GUI exists) | Low | Makes phase B independently verifiable and shippable |

### C. Overlay shell — table stakes

| Feature | Complexity | Notes |
|---------|-----------|-------|
| Always-on-top window over PoE2 | Medium | Opaque panel; see PITFALLS for the configurations to avoid |
| Global show/hide hotkey | Medium | `global-hotkey`; needs a Win32 event loop on the right thread |
| Verified against a running PoE2 in windowed-fullscreen | Medium | The one genuinely unverifiable-by-test item — needs the author at the machine |

### D. Rule editor UI — the milestone's substance

| Feature | Complexity | Notes |
|---------|-----------|-------|
| Ordinal rule list with add / delete / reorder | Medium | Order *is* the semantics (first-match-wins) |
| Per-rule enable/disable | Low | The "hide all flask drops" one-click case |
| Style selector | Low | Dropdown over the pinned `styles` palette |
| Multi-select class picker | Medium | Source options from `config.json` class lists, never hardcode |
| Rarity selector | Low | Maps to the existing compact comparator (`>=Magic`, `==Rare`, …) |
| Three-checkbox defence picker | Medium | Ticked `>0` / unticked `0`; 2³−1 = 7 combos = PoE2's 7 armour types; group off ⇒ no defence lines |
| Sockets / quality minimums | Low | |
| Corrupted tri-state | Low | yes / no / not-set — the third state is "omit the condition" |
| **Per-condition enable toggles** | Medium | The author's refinement: disabled ⇒ emits nothing. Maps onto `When`'s all-`Option` shape. |
| Apply button + "now hit Refresh in-game" affordance | Low | Non-negotiable given the reload constraint |

### E. Deliberately out of scope for v0.4

| Feature | Why |
|---------|-----|
| Editing economy tiers / cutoffs | Author: pinned this milestone |
| Editing the style palette | Author: pinned this milestone |
| `areaLevel` conditions in the dynamic editor | Live editing replaces level bands |
| Named presets / profile save-load | Author: single persisted live state; presets are what v0.4 removes |
| Undo history | Not requested; persistence was the actual need behind the question |
| Filesystem watching / auto-apply | The game cannot hot-reload — see PITFALLS |
| Item preview / "what would this match" simulation | Genuinely useful, genuinely a milestone of its own |
| Clipboard or OCR item reading | That is a price checker; different product |
| macOS / Linux | Game folder resolution and overlay behavior are Windows-shaped here |

## Anti-features

- **Anything resembling automation.** No input sent to the game, no memory reads. The feature does
  not need either, so there is nothing to weigh.
- **Implying live reload.** Any UI that suggests changes take effect instantly will be read as
  broken. Be explicit that the player must hit Refresh.
- **A second source of truth for `universal.filter`.** `config.json` stays the CLI's input, with
  its fixtures intact. `dynamic.json` is additive.

## Sources

- [Exiled Exchange 2](https://exiledexchange2.net/) and [its FAQ](https://exiledexchange2.net/faq/)
- [Awakened PoE Trade FAQ](https://snosme.github.io/awakened-poe-trade/faq)
- [PoE2 loot filter guide — FilterBlade and the Refresh flow](https://www.bamboogaming.net/poe2/loot-filters)
- [PoE Overlay](https://www.poeoverlay.com/) — another comparable in the overlay-companion space
