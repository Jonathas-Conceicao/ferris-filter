# Phase 48: Art discovery notes (ff-art, 2026-10-23)

Found with `cargo ff-art list|dump|info|zoom` against the default Steam install. **All four pieces
exist in the game files; no substitute is needed.** Every candidate also has a `4K/` sibling, so
the manifest entries use `prefer_4k = true`. Sizes below are the 4K files.

| Piece | Game path (base; 4K sibling used) | 4K size | Opaque box `[x0,y0,x1,y1]` | Notes |
|-------|-----------------------------------|---------|----------------------------|-------|
| Toggle plate, expanded (chevron up above dot) | `art/textures/interface/2d/2dart/uiimages/common/buttoncollapse{normal,hover,pressed}.dds` | 76x88 | `[4,5,57,68]` (54x64 plate) | The game names it by the click's effect: "collapse" is shown while the group is open. Exact match of `social-section-buttons.png`. Transparent margin: 4 left, 5 top, 18 right, 19 bottom. |
| Toggle plate, collapsed (dot above chevron down) | `.../common/buttonexpand{normal,hover,pressed}.dds` | 76x88 | `[4,5,57,68]` | Same plate, other glyph. Hover is brighter, pressed is dimmer. |
| Stone bar | `art/textures/interface/2d/2dart/uiimages/ingame/npcmenubar{left,middle,right}.dds` | left 180x100, middle 120x100, right 180x100 | left `[19,0,151,75]`, middle `[0,0,111,75]`, right `[0,0,151,75]` | Dark stone with a double bronze line top and bottom, ornate swirls in the left cap (right cap mirrors them, greener). Matches the Social panel group bar in `social-sections.png` (the game reuses the NPC menu bar art; no social-specific bar texture exists). 3-slice: left cap + stretched middle + right cap, ~76px tall visible -> scales to the 36pt row. Left cap has a 19px transparent left gutter; all three have a ~24px transparent bottom gutter (crop it). |
| Round clear X | `.../common/buttoncleartext{,hover,pressed}.dds` | 44x44 | `[0,0,36,36]` (37x37 disc) | Cream disc with a dark X, three states. Matches `social-search-fields.png`. 7px right/bottom gutter (trim). |
| Magnifying glass | `.../common/buttonsearch{normal,hover,pressed}.dds` | 72x72 | normal `[6,4,54,53]` | Cream-grey outline lens + handle, glyph only (no plate). `normal` is the muted resting colour, `hover` the bright cream of the reference. Also exist: `common/iconsearch.dds` (smaller, grey) and `iconsearchactive.dds` (cream). |

Other candidates looked at and rejected: `emailbuttonexpand/collapse` (orange eye plates),
`fourexpandbutton*` (empty plate), `invexpand/invcollapse` (red side tabs), `characterbar*` (taller
ornate panel), `accounttitlebar*`, `infofillbox*`.

Scratch dumps (not in the repo): `C:\Users\jonat\AppData\Local\Temp\claude\ffdump*`.

## ART-04: transparent margins in the art already used

`ff-art info` on the committed reload button art (extracted **without** `trim`):
`assets/poe2/common/icon_button_*.png` is 92x92 with the plate at `[0,0,84,84]` (7px transparent
gutter right and bottom), and the glyph `assets/poe2/common/reload.png` is 56x56 with the arrow at
`[1,1,46,45]` (~10px gutter right and bottom; it is drawn mirrored, so the gutter moves to the left
after the flip). Other committed art: `check_on.png` 54x56 opaque `[0,0,54,54]` (2px bottom gutter),
`close_normal.png` none, `combo_arrow.png` 46x46 glyph `[2,13,41,25]` (centred). `ff_reload_button` paints the
whole image into its rect, so the visible plate sits up-left of the rect's centre: this is the
Config -> Filter refresh offset the author sees, and the Economy league refresh uses the same
widget. Compensate by painting the image so its visible plate (not the image) fills / centres in the
rect (or trim the gutter at extraction).
