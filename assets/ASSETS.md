# assets/ — provenance and license

Every binary asset bundled into a FerrisFilter binary is recorded here: what it is, where it
came from, and under what license it may be redistributed. An asset whose license cannot be
stated is not committed to this repository.

| File | What it is | Source | License |
|------|------------|--------|---------|
| `uwu.mp3` | Custom alert sound played by the `S`/`A` tier styles in generated filters (`CustomAlertSound "uwu.mp3"`). | Author-supplied. | Author-owned; bundled for personal/filter use. |
| `ferris.png` | 64×64 RGBA tray icon for the `ff` overlay binary (system tray presence, D-16/D-18). | Cropped and resized (square crop around the opaque content, then Lanczos3-resized to 64×64) from `rustacean-flat-happy.png`, downloaded from [rustacean.net](https://rustacean.net/) — the community site for the Ferris/Rustacean mascot artwork. | [CC0 1.0 Universal (Public Domain Dedication)](https://creativecommons.org/publicdomain/zero/1.0/) — rustacean.net's own footer states its mascot assets are released under CC0; no attribution required. (Note: the *site's favicon*, a different asset not used here, is separately licensed CC BY-NC-SA — not applicable to `ferris.png`.) |
| `poe2/options/*.png`, `poe2/common/*.png`, `poe2/minimap/icons.png` | Path of Exile 2 UI art for the `src/skin` module: the Options window header, page frame (gear cleared), tabs, slider, scrollbar, popup box, title rule; round checkboxes, close button, generic button, combo arrow; the 12 minimap icon shapes in the 11 filter colours (Large cells only, packed from the game's `minimap/player.dds` sheet). | Extracted from the local game install by `tools/ff-art` per `poe2/manifest.toml` (game virtual path -> output file). | Owned by Grinding Gear Games. Used in a non-commercial fan tool; will be replaced if GGG objects. "This product isn't affiliated with or endorsed by Grinding Gear Games in any way." (README -> Credits). |
| `poe2/fonts/Fontin-SmallCaps.ttf`, `poe2/fonts/Fontin-Regular.ttf` | The game's UI faces: SmallCaps for headings/labels, Regular for body text (`skin::fonts`). | Copied byte-for-byte from the game install (`art/2dart/fonts/`) by `tools/ff-art`. | Fontin by exljbris (Jos Buivenga), as shipped with Path of Exile 2. |
| `ferris/rustacean-flat-happy.svg` | Ferris source vector. | [rustacean.net](https://rustacean.net/). | Public domain (CC0), Ferris by Karen Rustad Tölva. |
| `custom/titlebar.png`, `custom/titlebar.ase` | The overlay's header: the game's Options title bar redrawn with three Ferris crabs, 970x154 (exported PNG plus its Aseprite source). | Author-drawn over the extracted `poe2/options/titlebar.png`. | The author's own drawing; the underlying title-bar art is owned by Grinding Gear Games (see the `poe2/` row). |
| `custom/base-palette.ase` | Aseprite palette used for the custom art (not bundled into a binary). | Author-made. | Author-owned. |
| `ui/ferris-bg.png` | Static background for skinned windows: Ferris as pale white line art on transparency, 1200x2000 (2x). | Rendered from the SVG by `tools/ff-art ferris` (edges of the filled shapes, thickened, low alpha). | Derived from the CC0 SVG above. |

## Regenerating `ferris.png`

The crop/resize was a one-off transform done with the project's own `image` crate dependency
(no new dependency added). If a different source crop is ever wanted, the source PNG is
`rustacean-flat-happy.png` at `https://rustacean.net/assets/rustacean-flat-happy.png` (460×307,
RGBA, CC0) — crop to a square around the non-transparent content with a little padding, then
resize to 64×64 (or 32×32) with a decent filter (e.g. Lanczos3).
