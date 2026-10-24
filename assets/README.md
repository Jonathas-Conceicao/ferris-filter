# assets/ — provenance and license

Every binary asset bundled into a FerrisFilter binary is recorded here: what it is, where it
came from, and under what license it may be redistributed. An asset whose license cannot be
stated is not committed to this repository.

The project's GPL-3.0-or-later license (`LICENSE` at the repo root) covers only the files
whose License column below says so. Every other file here keeps its own owner and terms.

| File | What it is | Source | License |
|------|------------|--------|---------|
| `ferris.png` | 64×64 RGBA tray icon for the `ff` overlay binary (system tray presence, D-16/D-18). | Cropped and resized (square crop around the opaque content, then Lanczos3-resized to 64×64) from `rustacean-flat-happy.png`, downloaded from [rustacean.net](https://rustacean.net/) — the community site for the Ferris/Rustacean mascot artwork. | [CC0 1.0 Universal (Public Domain Dedication)](https://creativecommons.org/publicdomain/zero/1.0/) — rustacean.net's own footer states its mascot assets are released under CC0; no attribution required. (Note: the *site's favicon*, a different asset not used here, is separately licensed CC BY-NC-SA — not applicable to `ferris.png`.) |
| `poe2/options/*.png`, `poe2/common/*.png`, `poe2/minimap/icons.png` | Path of Exile 2 UI art for the `src/skin` module: the Options window header, page frame (gear cleared), tabs, slider, scrollbar, popup box, title rule; round checkboxes, close button, generic button, combo arrow; the Social panel's section toggle plates and stone bar, the round clear button and the search magnifier; the Options keybind button; the 12 minimap icon shapes in the 11 filter colours (Large cells only, packed from the game's `minimap/player.dds` sheet). | Extracted from the local game install by `tools/ff-art` per `poe2/manifest.toml` (game virtual path -> output file). | Owned by Grinding Gear Games. Used in a non-commercial fan tool; will be replaced if GGG objects. "This product isn't affiliated with or endorsed by Grinding Gear Games in any way." (README -> Credits). |
| `poe2/fonts/Fontin-SmallCaps.ttf`, `poe2/fonts/Fontin-Regular.ttf` | The game's UI faces: SmallCaps for headings/labels, Regular for body text (`skin::fonts`). | Copied byte-for-byte from the game install (`art/2dart/fonts/`) by `tools/ff-art`. | Fontin by exljbris (Jos Buivenga), as shipped with Path of Exile 2. |
| `ferris/rustacean-flat-happy.svg` | Ferris source vector. | [rustacean.net](https://rustacean.net/). | Public domain (CC0), Ferris by Karen Rustad Tölva. |
| `ferris/ferris.ico` | Multi-size Windows icon (16, 24, 32, 48, 64, 128, 256 px) for `ff.exe`, the installer and the Start menu. | Rendered from the SVG above by `tools/ff-art icon` (original colours, square crop around the artwork). | Derived from the CC0 SVG above. |
| `custom/titlebar.png`, `custom/titlebar.ase` | The overlay's header: the game's Options title bar redrawn with three Ferris crabs, 970x154 (exported PNG plus its Aseprite source). | Author-drawn over the extracted `poe2/options/titlebar.png`. | The author's own drawing is GPL-3.0-or-later; the underlying title-bar art is owned by Grinding Gear Games and is not (see the `poe2/` row). |
| `custom/icon_reoder_normal.png`, `custom/icon_reoder_hover.png`, `custom/icon_reoder_pressed.png` (each with its `.ase` source) | The reorder drag handle of the rule list and the style table, 92x92 (plate in the top-left 84x84), exported PNGs plus Aseprite sources. | Author-drawn. | GPL-3.0-or-later. |
| `custom/button-normal-palllet.ase`, `custom/button-hover-palllet.ase`, `custom/button-prassed-pallet.ase` | Aseprite palettes of the handle's three states (not bundled into a binary). | Author-made. | GPL-3.0-or-later. |
| `custom/base-palette.ase` | Aseprite palette used for the custom art (not bundled into a binary). | Author-made. | GPL-3.0-or-later. |
| `ui/ferris-bg.png` | Static background for skinned windows: Ferris as pale white line art on transparency, 1200x2000 (2x). | Rendered from the SVG by `tools/ff-art ferris` (edges of the filled shapes, thickened, low alpha). | Derived from the CC0 SVG above. |

## Distribution decision

The Path of Exile 2 UI art (`poe2/`) and the Fontin fonts ship credited and are compiled into
`ff.exe` (embedded with `include_bytes!`). The installer and the portable zip install no asset
files. Nothing is written into the install folder or the config folder
`%APPDATA%\FerrisFilter` for them. The Grinding Gear Games non-affiliation disclaimer stays
(README -> Credits, and the ABOUT tab). The assets will be replaced if Grinding Gear Games or
exljbris object.

## Regenerating `ferris.png`

The crop/resize was a one-off transform done with the project's own `image` crate dependency
(no new dependency added). If a different source crop is ever wanted, the source PNG is
`rustacean-flat-happy.png` at `https://rustacean.net/assets/rustacean-flat-happy.png` (460×307,
RGBA, CC0) — crop to a square around the non-transparent content with a little padding, then
resize to 64×64 (or 32×32) with a decent filter (e.g. Lanczos3).
