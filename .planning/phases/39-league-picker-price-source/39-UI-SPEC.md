---
phase: 39
slug: league-picker-price-source
status: approved
reviewed_at: 2026-10-18
shadcn_initialized: false
preset: none
created: 2026-10-18
---

# Phase 39 - UI Design Contract

> Visual and interaction contract. FerrisFilter is a Rust egui overlay with the PoE2 game skin
> (`src/skin/`); there is no web design system. Every value below is an existing skin widget,
> palette constant or font. Sources: 39-CONTEXT.md (locked), 37-UI-SPEC.md (refresh button),
> 38-UI-SPEC.md (Tiers "Reset to defaults"), CLAUDE.md overlay notes, `src/overlay/shell.rs`
> (`economy_tab`, `EconomyTabInputs`), `src/overlay/app.rs` (~L3245), `src/skin/{widgets,editor,palette}.rs`,
> memory note "overlay UI not texty". Phases execute 37, 38, 39: this spec applies on top of both.

---

## Design System

| Property | Value |
|----------|-------|
| Tool | none (egui + hand-built skin, not shadcn) |
| Preset | not applicable |
| Component library | `src/skin/` `ff_*` widgets over egui |
| Icon library | game art (Phase 37 `Skin.refresh`, `widgets::ff_icon_button`) |
| Font | Fontin Regular (`fonts::regular`), Fontin SmallCaps (`fonts::small_caps`) |

Registry safety: not applicable (no shadcn, no third-party registries).

---

## Scope of UI change

All on the **Economy tab** (`shell::economy_tab`) plus the status line:

| Surface | Change |
|---------|--------|
| Economy section rows | Rows become: **Price source** (new `Live | Fallback` switch), **League** (dropdown + Phase 37 refresh button), **Last sync**, **Priced items**. The read-only "Price source" text row is replaced by the switch. |
| Sync economy button | Greys under Fallback, when the league is ended, and while the list is unusable (tooltips below). |
| Tiers section | Editor and Phase 38 **Reset to defaults** grey under Fallback. |
| Status line | One new standing message (ended league). |
| Tab marker | Economy ` *` when a price-source change alters what Apply writes. |
| Debug screenshot mode | New fake-league-list flags (below). |

No new tab, popup, colour, font or size. The "League (pinned)" hover `Pinned in this build.` is deleted.

---

## Spacing Scale

Existing skin constants only (cite by name, never literals):

| Token | Value | Usage |
|-------|-------|-------|
| Row height | `widgets::ROW_HEIGHT` 34pt | every Economy row; refresh button is 34 x 34pt |
| Label column | `LABEL_COL_W` 128pt + `LABEL_GAP` | row labels (existing `ff_row`) |
| Control column | `CONTROL_WIDTH` 252pt | switch width; dropdown + button share it |
| Field gap | `FIELD_GAP` 10pt | between the league dropdown and the refresh button |
| Section gap | 16pt | existing `add_space(16.0)` between blocks (unchanged) |
| Switch height | `editor::SEGMENT_HEIGHT` 30pt | the Live / Fallback switch, vertically centred in its 34pt row |

Exceptions: 34pt button and 30pt switch are existing skin heights off the 4pt grid.

---

## Typography

No new sizes.

| Role | Font / Size | Usage |
|------|-------------|-------|
| Section heading | `small_caps(21)` (`ff_section`) | "Economy", "Tiers" (unchanged) |
| Row label | existing `ff_row` label face | "Price source", "League", "Last sync", "Priced items" |
| Row value / dropdown caption | `regular(15)` | league name, relative time, count |
| Switch caption | `small_caps(14)` as `ff_segmented` | `Live`, `Fallback` |
| Button caption | `small_caps(17)` (`ff_button_ex`) | `Sync economy` (unchanged) |

---

## Color

Palette constants (`src/skin/palette.rs`) only.

| Role | Constant | Usage here |
|------|----------|------------|
| Dominant | `GROUND` / `PANEL` | window surface |
| Secondary | `ROW_EVEN` / `ROW_ODD` | Economy row striping (rows renumbered 0..3) |
| Accent (gold) | `HEADING` | the selected switch segment caption only (existing `ff_segmented` rule) |
| Value | `VALUE` | league name, times, counts |
| Warning | `WARN` E8B052 | the `(ended)` league caption; the status line warning |
| Disabled | `DISABLED` 88856E | greyed controls (existing `ff_*_ex` disabled look) |

Accent reserved for: the selected Live / Fallback segment. Nothing else uses gold. No red in this
phase.

---

## Economy tab layout (top to bottom)

`Economy` heading, then four rows in the `ff_row` two-column form (label left, control right):

| # | Label | Control | Notes |
|---|-------|---------|-------|
| 0 | `Price source` | `ff_segmented` `Live | Fallback`, width `CONTROL_WIDTH` (252pt) | placed first because it governs the rows and buttons below it |
| 1 | `League` | league dropdown then refresh button | dropdown = `CONTROL_WIDTH - ROW_HEIGHT - FIELD_GAP` wide (208pt), button right-aligned at the control column edge, exactly the Phase 37 Filter-row geometry |
| 2 | `Last sync` | relative time text (unchanged) | hover: exact local time, plus ` - <league>` when `prices.json` records the league it priced |
| 3 | `Priced items` | thousands-formatted count (unchanged) | unchanged |

Then (unchanged order) 16pt gap, centred **Sync economy** button, 16pt gap, the `FF Economy Section`
heading + Filter section row, 16pt gap, `Tiers` heading (+ Phase 38 Reset button), the cutoff editor.

### Price source switch (LEAGUE-04)

- `skin::editor::ff_segmented(ui, skin, &mut idx, &["Live", "Fallback"], CONTROL_WIDTH)`; reuse as is,
  no new widget. Index 0 = Live, 1 = Fallback. Radio semantics and accessible names come from the
  widget.
- Click on the other segment: saves the choice at once to `ff-settings.json` (new key, name left to
  the executor, e.g. `price_source`: `"live"` / `"fallback"`; absent or unknown value reads Live;
  unknown keys preserved on save) and re-prices the economy view. It never marks CONFIG dirty and is
  not a rule or config edit.
- Tooltip (whole switch, two lines): `Live: prices from your last sync. Fallback: the built-in early-league prices.` /
  `Fallback is recommended at league start, and if you don't trade.`
- Live with no usable `prices.json` (fresh install, empty or unreadable file): the switch still reads
  Live and still lights Live; the pricing falls back as today. The tooltip then gets a third line
  carrying the reason, reusing the existing strings: `No prices.json yet. Using the built-in early-league prices until you sync.` or
  `prices.json could not be used (<problem>). Using the built-in early-league prices until you sync.`
  The segment captions do not change colour (no standing text added; Last sync reads `Never` for the
  missing-file case).
- If saving `ff-settings.json` fails: the choice still applies for the session and the status line
  event reads `Could not save ff-settings.json: <err>` (same wording Phase 37 uses).

### League dropdown (LEAGUE-01 / LEAGUE-02 / LEAGUE-03)

`skin::widgets::ff_dropdown_ex(ui, skin, "economy-league", &mut sel, &options, width, caption_color)`.

Options are the names from `economyLeagues`, **in poe.ninja's order, all of them** (including
`HC ...`, `Standard`, `Hardcore`). The league sent to sync is the `name` field (as the existing sync
code uses it); the slug is not shown. Selection is written to `ff-settings.json` (new key, e.g.
`league`) **only when the user picks an entry**; the default is computed, never saved.

Default (nothing saved): the first entry whose `hardcore` is false and whose slug is neither
`standard` nor `hardcore`; the `indexed` flag is ignored. With the observed list that is Forbidden
Rites. If no entry qualifies, there is no default (state "no league" below).

States:

| State | Dropdown caption | Colour | Dropdown | Refresh | Sync economy | Tooltip on dropdown |
|-------|------------------|--------|----------|---------|--------------|---------------------|
| Loading, league known (saved) | saved name | `VALUE` | **disabled** until the list arrives | disabled | enabled | `Loading the league list.` |
| Loading, nothing saved | `Loading…` | `DISABLED` | disabled | disabled | disabled, tooltip `Loading the league list.` | `Loading the league list.` |
| Loaded, selected in list | name | `VALUE` | enabled | enabled | enabled | `The league Sync economy prices against.` |
| Failed, league known (saved) | saved name | `VALUE` | **enabled** but lists only that one entry | enabled | enabled | `The league list could not be loaded. Press refresh to try again.` |
| Failed, nothing saved, `prices.json` records a league | that league name | `VALUE` | enabled, single entry | enabled | enabled | `The league list could not be loaded. Press refresh to try again.` |
| Failed, nothing saved, no league in `prices.json` | `(no league)` | `DISABLED` | disabled | enabled | disabled, tooltip `Press refresh to load the league list.` | `The league list could not be loaded. Press refresh to try again.` |
| Loaded, saved league missing (ended) | `<name> (ended)` | `WARN` | enabled | enabled | disabled, tooltip `Pick a league first.` | `<name> is no longer on poe.ninja.` |
| Loaded, nothing saved, no default | `(no league)` | `DISABLED` | enabled | enabled | disabled, tooltip `Pick a league first.` | `Pick a league.` |
| Price source = Fallback | (any of the above) | `DISABLED` | disabled | disabled | disabled | `Not used with Fallback prices.` |

Ended modelling (the author left it to discretion): when the saved name is not in the loaded list,
prepend one synthetic option `<name> (ended)` and select it, pass `caption_color = Some(WARN)`.
Picking any real league replaces the saved value and the synthetic option disappears on the next
frame; nothing is switched automatically and nothing is written until a pick. A failed fetch never
produces "ended" (unknown is not ended).

The fetch (worker thread, only on first opening of the Economy tab in a session, and on refresh;
never at start-up, never in tests or screenshot mode):
- First visit to the tab starts it; tab switching afterwards never refetches.
- Refresh button (Phase 37's `ff_icon_button`, the same art and `R` fallback, tooltip `Reload the league list`, accessible name `Refresh`) starts a new fetch; it is disabled while one runs.
  Refresh does not change the selection and writes nothing. No status message on success; the dropdown shows the new list.
- A fetch that fails shows no status message and no popup; only the dropdown tooltip changes (state
  "Failed" above). Offline never blocks Apply or the rest of the tab.
- A running Sync economy does not block the league list fetch and vice versa; but while Sync runs
  the dropdown and refresh are disabled (the league must not change under a running sync).

### Sync economy button

`w::ff_button_ex(... "Sync economy", 180.0, enabled, false)` unchanged. `enabled` additionally
requires: price source Live, a known non-ended league, and the app root (existing). Disabled tooltip
priority (first that applies): app-root error (existing) > `Not used with Fallback prices.` >
`Loading the league list.` > `Press refresh to load the league list.` > `Pick a league first.`
Label while syncing stays `Syncing…`. The sync uses the dropdown's league (replacing
`OVERLAY_LEAGUE`, which is deleted along with the CLAUDE.md "update each league" note). The success
status line still names the league it priced (existing).

### Tiers under Fallback (LEAGUE-05)

- Phase 38's `Reset to defaults` button: `enabled = false`; tooltip (replaces `Already at the defaults.`): `The fallback tiers are fixed.`
- The cutoff boxes: drawn in a disabled `Ui` (no focus, no typing); hover tooltip on the box block: `The fallback tiers are fixed.`
  Values shown are the user's `tiers.json` cutoffs, unchanged and not edited. Pending cutoff edits already typed before switching
  stay in memory, are not lost, and the Economy ` *` marker stays while they exist (existing tiers-dirty rule).
- While Fallback is on, Apply's tier validation ignores cutoff validation errors (the cutoffs are unused); the inline error text
  still shows when the editor is enabled again. (Researcher choice, see below.)

---

## Unapplied-change marking (LEAGUE-04)

The price source is a saved setting, not an edit. Rule, mirroring the `styles.json` watcher: after a
switch, if the loaded filter has an economy section **and** the economy blocks Apply would now write
differ from the blocks in the file, the unapplied state turns on:
- Economy tab label shows ` *` (tip: existing `OTHER_TAB_DIRTY_TIP` on the other tabs).
- Status line standing condition reads `Unapplied changes` (existing indicator, hover explains), and
  Apply is live.
- Switching back to the original source clears the marker (it is derived from comparing output, not
  a counter).
- **Revert does not undo it** (it is a setting, like the reload toggle) and does not open
  **Discard economy changes?** because of it. Tooltip on the switch third line is not needed; the
  existing indicator hover covers it.
- No economy section in the loaded filter (or no filter loaded): no marker.
- A league pick or a sync never marks anything (prices already synced keep working until the next
  sync; Last sync still shows the league it priced).

---

## Status line (one new standing condition)

| Condition | Text | Kind | Click |
|-----------|------|------|-------|
| Saved league not in the loaded list, price source Live | `<name> is no longer on poe.ninja; pick a league` (locked) | warning (`StatusKind::Warning`, `WARN`) | switches to the Economy tab |

Priority in `status_for`: directly after **sync running** and before **unknown styles**. Resulting
order: missing app root, event, grammar gate, damaged markers / load failure, game folder not found
(Phase 37), sync running, **ended league**, unknown styles, v0.4 notice, absent filter, indicator.
Not shown under Fallback or while the list failed or is loading. Shown on every tab (like unknown
styles) until a league is picked.

---

## Copywriting Contract

| Element | Copy |
|---------|------|
| Row labels | `Price source`, `League`, `Last sync`, `Priced items` |
| Switch segments | `Live`, `Fallback` |
| Switch tooltip | `Live: prices from your last sync. Fallback: the built-in early-league prices.` / `Fallback is recommended at league start, and if you don't trade.` (+ Live-without-prices reason line, above) |
| League caption (ended) | `<name> (ended)` |
| Loading caption | `Loading…` |
| No league caption | `(no league)` |
| Dropdown tooltips | `The league Sync economy prices against.` / `Loading the league list.` / `The league list could not be loaded. Press refresh to try again.` / `<name> is no longer on poe.ninja.` / `Pick a league.` |
| Refresh tooltip | `Reload the league list` (Fallback: `Not used with Fallback prices.`; loading: `Loading the league list.`) |
| Sync disabled tooltips | `Not used with Fallback prices.`, `Loading the league list.`, `Press refresh to load the league list.`, `Pick a league first.` |
| Tiers under Fallback | `The fallback tiers are fixed.` (editor and Reset button) |
| Status line (ended) | `<name> is no longer on poe.ninja; pick a league` |
| Save failure event | `Could not save ff-settings.json: <err>` |
| Primary CTA | `Sync economy` |
| Empty state | `(no league)` caption; no paragraph |
| Error state | list fetch failure is tooltip-only; sync failure uses the existing sync error status |
| Destructive actions | none in this phase; no popup is added or changed |

No standing explanatory paragraph is added (author preference); everything explanatory is a tooltip.

---

## Interaction states summary

| Control | Idle | Hover | Active | Disabled |
|---------|------|-------|--------|----------|
| Live / Fallback switch | dim art, `DISABLED` caption on the idle segment | caption `LABEL`, tooltip | lit art, `HEADING` caption | never disabled (also while syncing: allowed; the sync result still lands in `prices.json`) |
| League dropdown | `ff_dropdown_ex` closed | tooltip | opens list (all leagues, poe.ninja order) | dim, tooltip via disabled hover |
| Refresh | Phase 37 art | tooltip | starts fetch | dim tint `from_white_alpha(90)` |
| Sync economy | `ff_button` | `ff_button` hover | starts sync | dim art, `DISABLED` caption, tooltip |
| Tiers editor + Reset | per Phase 38 | per Phase 38 | per Phase 38 | dim, `The fallback tiers are fixed.` |

Accessibility: the switch reports radio buttons with selected state (as `ff_segmented`); disabled
controls report `enabled = false`.

---

## Debug screenshot mode (executor verification; compiled out of release builds)

Existing guard applies: `FERRISFILTER_HOME`, `USERPROFILE`, the game folder **and** the settings/
AppData folder (Phase 36 per-user folder, `%APPDATA%\FerrisFilter`) must all be under the temp
folder; the run must never touch the network, the real `%APPDATA%\FerrisFilter` or the real game
folder. The league fetch is **never started** in screenshot mode; the list is injected.

New flags on `ff --screenshot <png> --tab economy`:

| Flag | Effect |
|------|--------|
| `--leagues <names>` | Fake list, comma-separated names in order. A name prefixed `HC ` sets hardcore; `Standard` and `Hardcore` get their slugs. `none` = state "Failed" (fetch error). `loading` = state "Loading". |
| `--league <name>` | Saved league (written to the temp `ff-settings.json` before the first frame). |
| `--price-source live|fallback` | Saved price source. |

Required captures (each a PNG the executor looks at):

1. Loaded, default: `--leagues "Forbidden Rites,Runes of Aldur,HC Forbidden Rites,HC Runes of Aldur,Standard,Hardcore"` (the live list of 2026-10-18), no `--league`: caption `Forbidden Rites`, Live lit.
2. Saved other league: `--league "Runes of Aldur"`.
3. Ended: `--league "Fate of the Vaal"` with the list above: caption `Fate of the Vaal (ended)` amber, Sync greyed; status line warning text visible (also capture `--tab rules` to see the status line elsewhere).
4. Failed with saved league, and Failed with nothing saved (`--leagues none`, with/without `--league`).
5. Loading (`--leagues loading`), nothing saved: `Loading…`.
6. Fallback: `--price-source fallback`: switch lit on Fallback, league dropdown, refresh, Sync and the Tiers editor + Reset button all dim.
7. Marker: `--filter` with an economy section plus `--price-source fallback` against a prices.json-priced fixture: Economy tab label ends with ` *`.
8. Pixel checks: refresh 34 x 34pt, 10pt gap to the dropdown, dropdown + button right edge aligned with the switch's right edge; row stripes continue 0..3.

Unit tests (no network): default-league pick over the live list (Forbidden Rites) and over
`[Standard, Hardcore]` (none); ended detection; Sync-enabled / tooltip priority table; status-line
priority; Fallback greys; marker derived from output difference.

---

## Decisions made by researcher (non-interactive run; author may revise)

1. "Hardcore" in the default rule is interpreted as: skip entries with slug `standard` or `hardcore` **and** any entry with `hardcore = true` (so `HC ...` challenge leagues are never the default). The author's wording named only the permanent leagues; skipping HC challenge leagues is the safer reading (a softcore player must not default to HC).
2. The dropdown lists every `economyLeagues` entry, in poe.ninja's order, including HC, Standard and Hardcore. `oldEconomyLeagues` are not listed. The sync league string is the `name` field.
3. Row order is Price source, League, Last sync, Priced items (switch first because it governs the rows below); the old read-only "Price source" text row is replaced by the switch, and its reason texts move into the switch tooltip. Live-without-prices keeps the Live segment lit and explains in the tooltip only.
4. League dropdown is disabled while the list loads and during a running sync; enabled but single-entry when the fetch failed (so the saved league stays visible and usable).
5. No saved league and the fetch failed: the default is the league `prices.json` last priced (its `league` field), shown and usable for Sync, which honours CONTEXT's "shows the saved (or default) league name" offline; only when `prices.json` has no league either (fresh install offline), or the loaded list has no qualifying default, the caption is `(no league)`, Sync disabled with a tooltip. The CLI-style `indexed` auto-resolution is deliberately not reused (the `indexed` flip reason in CONTEXT).
6. "(ended)" modelled as a synthetic selected first option coloured `WARN`; nothing is persisted or switched until the user picks.
7. Ended-league status is a warning, shown on all tabs, suppressed under Fallback, loading and failed; click jumps to the Economy tab. Priority placed after sync running.
8. Fetch failure is silent beyond the tooltip (no status line), so offline never nags.
9. Under Fallback the Tiers cutoff boxes are disabled and tier validation is skipped (the cutoffs are unused); the editor keeps the stored values rather than showing the fallback pair's values (the fallback tiers file is a different pair and the embedded pair is not editable).
10. The ` *` for a price-source switch is derived from "Apply would now write different economy blocks", not stored; Revert does not undo it and opens no popup. Switching back clears it.
11. Setting key names (`price_source`, `league`) are suggestions; the executor may rename but both must be absent-tolerant and invalid-value tolerant (invalid price source reads Live).
12. `Last sync` hover appends the league it priced when `prices.json` records one, because the League row no longer shows the league that was synced.
13. Screenshot mode never fetches; the league list is injected through `--leagues`, saved values through `--league` and `--price-source`, all under the temp home.
14. The Refresh tooltip for this use is `Reload the league list` (Phase 37's `Refresh the file lists` is specific to files); same button, art and size.

---

## Checker notes (orchestrator, from the checker's non-blocking flags)

- Focal point of the Economy tab: the Price source switch (row 0); Sync economy is the primary action.
- Surface split: `GROUND`/`PANEL` dominant, `ROW_EVEN`/`ROW_ODD` secondary, gold accent only on the selected segment (about 10%).
- Line height: egui default, as in Phases 37 and 38.

## Checker Sign-Off

- [x] Dimension 1 Copywriting: PASS
- [x] Dimension 2 Visuals: PASS
- [x] Dimension 3 Color: PASS
- [x] Dimension 4 Typography: PASS
- [x] Dimension 5 Spacing: PASS
- [x] Dimension 6 Registry Safety: PASS

**Approval:** approved 2026-10-18 (gsd-ui-checker; dimensions 2-5 FLAG, non-blocking, addressed in the notes above)
