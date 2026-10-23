# Phase 48: Game-Art Widgets - Context

**Gathered:** 2026-10-23
**Status:** Ready for planning
**Mode:** Smart discuss (autonomous run 43-50; author answered every question up front in prose)

<domain>
## Phase Boundary

Section headers redrawn in the game's Social-panel style, the game's clear X and magnifying glass in
search fields, and correct positioning of button art with transparent margins (fixing the existing
Config → Filter refresh button offset). Requirements ART-01..04. Phase 49 builds on the redrawn rows.

</domain>

<decisions>
## Implementation Decisions

### Art (author)
- Extract the Social-panel toggle plate (and its hover state if the game has one), the stone bar,
  the round clear X and the magnifying glass with `ff-art` into `assets/poe2/` (manifest entries).
  The PoE2 install is at the default Steam path on this machine.
- If any of these is **not found**, paint a substitute matching the reference crops and **record it
  for the author's review** (list which piece was substituted and why) — in the phase SUMMARY and the
  end-of-run review list.
- References: `.planning/research/ui-refs/social-*.png`, ROADMAP backlog 999.23-999.25.

### Section headers
- 36pt bar (unchanged height): toggle plate at the far left — **chevron up above a dot = expanded**,
  **dot above a chevron down = collapsed** (the game's convention) — then the stone bar with its
  bronze frame. Title stays **left-aligned**; `{n} rules`, rename pencil, up / down arrows, remove x
  stay on the right. The whole bar stays clickable to toggle.
- The economy header gets the same look and keeps its padlock.

### Search fields
- Every search field (RULES search, the + condition picker, and any other search box found): the
  round clear X inside the right end while the field holds text — a click empties it and keeps focus
  (same effect as Escape in the RULES toolbar); the magnifying glass inside the left end, with the
  placeholder / text shifted right.

### Art margins (author — ART-04)
- Game and custom button art carries transparent margins (left and bottom gutters). Position every
  new button by its visible plate, compensating those margins, so it sits centred where intended.
- Fix the existing offset of the **Config → Filter refresh button** (the round reload button beside
  the filter dropdown), which is visibly off because of this. Check the other uses of the same art
  (e.g. the Economy league refresh) for the same offset.

### Claude's Discretion
- Slicing / painting approach (three-slice like the tabs), hover / pressed visuals for the plate.

</decisions>

<code_context>
## Existing Code Insights

### Reusable Assets
- `draw_section_header` in `src/overlay/rule_list.rs` (36pt `SECTION_HEADER_HEIGHT`).
- RULES toolbar search (`ff_text_field` in `rule_list.rs`), the + condition picker search
  (`src/overlay/conditions.rs` or `body.rs`), `src/skin/widgets.rs` (`ff_text_field*`,
  `ff_reload_button`), `src/skin/textures.rs` (embedded textures), `assets/poe2/manifest.toml`,
  `tools/ff-art` (`cargo ff-art list|dump|extract`).

### Established Patterns
- Art is committed as PNGs and embedded with `include_bytes!`; `ff-art` writes only `assets/poe2/`;
  credits in README.

### Integration Points
- RULES list (headers), CONFIG Filter section (refresh), Economy tab (league refresh), condition
  picker.

</code_context>

<specifics>
## Specific Ideas

- Author: "the refresh button is actually a bit offset because of this" — fix it here.

</specifics>

<deferred>
## Deferred Ideas

None — discussion stayed within phase scope.

</deferred>
