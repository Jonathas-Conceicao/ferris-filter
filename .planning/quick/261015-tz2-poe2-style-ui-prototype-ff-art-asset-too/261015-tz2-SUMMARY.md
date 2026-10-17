---
quick_id: 261015-tz2
status: complete
date: 2026-10-16
commits: 206c6bb..(this docs commit)
---

# Summary: PoE2-style UI prototype (ff-art, src/skin, ff-proto)

## Built
- **`tools/ff-art`** (own workspace; `poe_data_tools` 4.0.1 library on nightly, `image-extras`
  for DDS, `resvg`): extracts `assets/poe2/manifest.toml` entries from the Steam install
  (read-only) into `assets/poe2/`, renders `assets/ui/ferris-bg.png`. Subcommands: extract
  (default), list, dump, sheet, info, zoom, ferris.
- **`src/skin/`**: palette, fonts, embedded textures (LINEAR + mipmaps), pixel-snapped
  nine/three-slice, `ff_*` widgets, DPI-correct left dock. 14 unit tests.
- **`ff-proto`**: left-docked 660pt, full work-area height, borderless prototype on fake data;
  `--screenshot`, `--tab`.
- Docs: README (run instructions + credits/GGG disclaimer), CLAUDE.md layout rows, CHANGELOG,
  assets/ASSETS.md.

## Game files used
| Element | Game file (`art/textures/interface/2d/2dart/uiimages/...`) |
|---|---|
| Header (angels, hooded figure, gold plate) | `ingame/options menu/titlebar.dds` (no 4K) |
| Page frame + button bar (gear cleared) | `ingame/optionsmenu/4k/optionspagesframe.dds` |
| Tabs (stone) | `ingame/options menu/toptabs/gametab/{left,centre,right}.dds` |
| Slider | `ingame/optionsmenu/4k/optionsslider{bg,fill}{left,middle,right}.dds`, `optionssliderthumb.dds` |
| Scrollbar | `ingame/options menu/scroller/{scrollbararea,top,centre,bottom}.dds` |
| Dropdown popup box | `ingame/options menu/9slicemenu.dds` |
| Section rule | `ingame/options menu/underliningfortitles.dds` |
| Round check | `common/4k/checkbox{checked,unchecked}.dds` |
| Close | `common/4k/buttonclose{normal,hover,down}.dds` |
| Buttons | `common/4k/buttongeneric{normal,hover,pressed}{left,middle,right}.dds` |
| Dropdown / reorder arrows | `common/4k/buttoncombobox{normal,hover}.dds` |
| Minimap sheet (unused) | `art/2dart/minimap/player.dds` |
| Fonts | `art/2dart/fonts/fontin-{smallcaps,regular}.ttf` |

Not found as art: the dropdown box itself (painted: dark fill + thin border, as in the game).

## Verification
- `cargo test`: all pass (1083 lib + integration suites); `cargo fmt --check` clean; no clippy
  warnings in new code; fixtures untouched; `src/overlay/*` untouched.
- Screenshots: `ff-proto-rules.png`, `ff-proto-config.png`, `ff-proto-economy.png` (this folder).

## Known gaps
- Dock uses the primary monitor's work area (not the monitor under a moved window).
- DPI path only exercised at 100% on this machine (math unit-tested for 125%/150%/200%).
- Minimap sheet is 4.7 MB; the rest of `assets/poe2` is ~1 MB.
