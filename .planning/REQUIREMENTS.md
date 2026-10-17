# Requirements: FerrisFilter — Milestone v0.6 New PoE2-based Tool UI

**Defined:** 2026-10-16
**Core Value:** `generate` turns the config into a correct, deterministic PoE2 loot filter. This
milestone is overlay-first: `generate` / `sync` output and the committed fixtures stay
byte-identical; the only file-format change is the divider banner (SECT-01, SECT-04).

**Milestone goal:** `ff` is rebuilt in Path of Exile 2's own Options-window style on the shared
skin proven by the `ff-proto` prototype (quick tasks 261015-tz2 / 261016-1de), and the prototype is
retired. Every v0.5 editor capability survives the move.

## v0.6 Requirements

### Workspace (WS)

- [x] **WS-01**: The repo root is a Cargo workspace. `ff` stays the default package: `cargo build`,
  `cargo run` and `cargo test` at the root behave as before and do not build `ff-art` or its
  dependencies.

- [x] **WS-02**: User can run the asset tool from the repo root (`cargo run -p ff-art -- <cmd>`, plus
  a short `cargo ff-art` alias) without changing directory; its output paths resolve against the
  repo root, not the working directory.

### Overlay Shell (SHELL)

- [x] **SHELL-01**: `ff` opens as the new skinned window instead of the stripe: 660pt wide, docked
  to the left edge at full work-area height, borderless, with the static Ferris background behind
  scrolling content.

- [x] **SHELL-02**: The window docks on the monitor the game window is on (the primary monitor
  when the game is not running) and is sized correctly at any Windows display scale (100-200%).

- [x] **SHELL-03**: The header shows the game-art title plate with "FerrisFilter" in a slot that can
  later take custom header art; the close button hides the window as the stripe's did.

- [x] **SHELL-04**: The tabs RULES / CONFIG / ECONOMY / ABOUT sit like the game's Options tabs; a tab
  with unapplied edits on another tab keeps today's ` *` marker behaviour.

- [x] **SHELL-05**: The bottom bar holds REVERT / RELOAD / APPLY and one status line: one line, cut
  with "…" when too long, full text on hover, coloured by kind (ok / warning / error). Every message
  that today lands in the outcome area uses it.

- [x] **SHELL-06**: Hotkey (`Ctrl+Shift+Z`), tray icon and Quit, single-instance behaviour and the
  in-game reload after Apply work exactly as in v0.5.

- [x] **SHELL-07**: Every confirmation and choice the overlay asks (changed-on-disk RELOAD /
  OVERWRITE, switch filter and discard, Quit with unapplied edits, deletes) uses the skinned popup.

- [x] **SHELL-08**: The UI reads less as text (backlog 999.11): standing captions replaced by
  controls, icons, tooltips or the status line; no wall-of-text panels.

### Rules Tab (RULES)

- [x] **RULES-01**: The RULES tab is drawn with the skin (rows, round enable check, Show/Hide,
  name, condition summary, flags, reorder arrows, search, + Add rule) and stays responsive on the
  ~500-block NeverSink/FilterBlade sample.

- [x] **RULES-02**: The expanded rule editor keeps every v0.5 capability (all conditions and actions,
  operators, Between, Defences / Simple Defences, grouped rules, raw rows, style picker, Continue,
  duplicate, delete) using the skin's controls.

- [x] **RULES-03**: + Add rule inserts the new rule at the top of the rule list (first block of the
  file, ahead of any divider), not before the economy section. It always lands outside every
  section, even when a divider is the first thing in the list, collapsed or not.

### Rule Dividers (SECT)

- [x] **SECT-01**: A divider is written in the filter as an explicit three-line banner readable in
  raw text:
  ```
  ################################################################################
  # ff:section <name>
  ################################################################################
  ```
  (80 `#` per rule line). Loading a filter shows each banner as a named section header in the rule
  list; rules above the first divider have no header. Foreign comment banners are not dividers.

- [x] **SECT-02**: User can add a divider (a toolbar **+ Add section** inserts one at the top with its
  name field open; **Section above** in an expanded rule inserts one directly above that rule),
  rename it inline, move the divider line up/down one rule at a time, and delete it (its rules stay
  and join the section above); each is an in-memory edit written by Apply and undone by Revert.
  Each section header shows a collapse arrow, the name, its rule count, rename, arrows and remove.

- [x] **SECT-03**: User can collapse and expand a section, hiding every rule up to the next divider;
  the collapsed state is remembered per filter in `ff-settings.json`, never written into the filter.

- [x] **SECT-04**: The economy section is shown as the last divider, a locked "Economy" section
  (collapsed by default, read-only rows), replacing the `+ N locked rule blocks` footer row. The
  divider is display-only: it can be collapsed and expanded, but adding or removing the economy
  section is done from the ECONOMY tab (ECO-01), not from the rule list. It has a begin banner only
  (same 80-`#` three-line format, tag `# ff:economy-begin Economy (generated by FerrisFilter, do
  not edit)`), no end marker: like every section it runs to the next section banner or EOF.
  Add economy section (ECONOMY tab) and `ff-cli apply --add-economy` always append it at the end of
  the file; Remove economy section / `--remove-economy` deletes the banner and its blocks; Apply
  regenerates the blocks under the banner. The old one-line `# ff:economy-begin/end` markers are no
  longer recognised: no migration (the app is not publicly released; the author converts their own
  `dynamic.filter` by hand).

- [x] **SECT-05**: Search shows matches inside collapsed sections; reordering and grouped rules keep
  working across section boundaries, and a filter's bytes outside edited lines are unchanged.

### Styles (STY)

- [x] **STY-01**: The CONFIG style tables have a column header row (Visibility / Name / Icon /
  Preview / Order); built-in styles show the Order arrows greyed out so the columns line up; the
  textual Show/Hide caption before the icon is gone (the visibility switch says it).

- [x] **STY-02**: Clicking a style expands a line-by-line editor like the rule editor, with no boxed
  panels: `Visibility  <Show><Hide>` on one line, the label preview with Text / Border / Background
  swatches, font size, minimap (size, colour, game-icon shape), beam (colour, Temp), sound and
  volume.

- [x] **STY-03**: Clicking a colour swatch opens a colour-picker popup (hue/saturation gradient plus
  exact RGB/hex input, alpha where the target supports it) whose frame and controls use the game
  art; the same picker serves rule colour actions.

- [x] **STY-04**: Custom styles keep rename, reorder and delete; Delete sits at the bottom of the
  expanded editor and asks for confirmation (showing how many rules use the style, as today).

- [x] **STY-05**: Every style and tier edit keeps v0.5's model: in memory until the one Apply,
  written all or none, undone by Revert.

### Economy Tab (ECO)

- [ ] **ECO-01**: The ECONOMY tab shows the pinned league, the last sync time and the number of
  priced items, with SYNC ECONOMY (moved from the RULES footer); sync still runs off the UI thread
  and writes only `prices.json`. The tab also holds the Add economy section / Remove economy section
  control for the loaded filter: it writes or removes the banner-format markers of SECT-04 as an
  in-memory edit, written by the shared Apply and undone by Revert.

- [ ] **ECO-02**: The TIERS cutoffs editor lives on the ECONOMY tab with inline validation, applied
  by the shared Apply.

### About Tab (ABOUT)

- [ ] **ABOUT-01**: The ABOUT tab shows, top to bottom: app name, one-line description, a small
  happy orange Ferris, the version (from Cargo), the GitHub link, a divider line, empty space, then
  at the bottom the ownership disclaimer ("This product isn't affiliated with or endorsed by
  Grinding Gear Games in any way." and the art/fonts belonging to their owners), followed by the
  smaller third-party credits (GGG art and fonts, Fontin by exljbris, Ferris by Karen Rustad Tölva,
  public domain).

### Cleanup (CLEAN)

- [x] **CLEAN-01**: `ff-proto` and the old `overlay::theme` stripe styling are removed; the skin
  module is the only overlay look. `ff-art` and the committed `assets/` stay.

- [x] **CLEAN-02**: README, CLAUDE.md and CHANGELOG describe the new UI, dividers, workspace and the
  asset tool; the Cargo version is bumped to `0.6.0` when the milestone is tagged.

## Future Requirements (deferred)

- Sound preview in the style editor (play the selected alert sound or custom file) — backlog.
- Moving a rule more than a few positions at once (drag, move-to-top/bottom, move to a section) —
  backlog; v0.6 keeps the arrows.

- ~~Custom header art~~ — done 2026-10-16: the author's header (`assets/custom/titlebar.png`) is in the slot.

## Out of Scope

- Changes to `generate` / `sync` output or the committed fixtures — this is a UI milestone (the
  economy markers written by `apply` change per SECT-04).

- A skin switch or alternative themes — one look, the PoE2 one.
- Turning foreign filters' own comment banners into dividers — only `# ff:section` banners count.
- Migrating filters that still carry the old one-line economy markers (not publicly released).

## Traceability

| Requirement | Phase | Status |
|-------------|-------|--------|
| WS-01 | Phase 29 | Complete |
| WS-02 | Phase 29 | Complete |
| SHELL-01 | Phase 30 | Complete |
| SHELL-02 | Phase 30 | Complete |
| SHELL-03 | Phase 30 | Complete |
| SHELL-04 | Phase 30 | Complete |
| SHELL-05 | Phase 30 | Complete |
| SHELL-06 | Phase 30 | Complete |
| SHELL-07 | Phase 30 | Complete |
| SHELL-08 | Phase 30 | Complete |
| RULES-01 | Phase 31 | Complete |
| RULES-02 | Phase 31 | Complete |
| RULES-03 | Phase 31 | Complete |
| SECT-01 | Phase 32 | Complete |
| SECT-02 | Phase 32 | Complete |
| SECT-03 | Phase 32 | Complete |
| SECT-04 | Phase 32 | Complete |
| SECT-05 | Phase 32 | Complete |
| STY-01 | Phase 33 | Complete |
| STY-02 | Phase 33 | Complete |
| STY-03 | Phase 33 | Complete |
| STY-04 | Phase 33 | Complete |
| STY-05 | Phase 33 | Complete |
| ECO-01 | Phase 34 | Pending |
| ECO-02 | Phase 34 | Pending |
| ABOUT-01 | Phase 34 | Pending |
| CLEAN-01 | Phase 35 | Complete |
| CLEAN-02 | Phase 35 | Complete |

**Coverage:** 28/28 v1 requirements mapped, no orphans, no duplicates.

---
*Requirements defined: 2026-10-16*
*Last updated: 2026-10-16 — roadmap created (traceability filled); add/remove economy section moved to the ECONOMY tab (ECO-01, SECT-04)*
