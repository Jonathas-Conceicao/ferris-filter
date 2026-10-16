# poe2_myfilter

Version-controlled **Path of Exile 2 loot filters**.

## Structure
- `*.filter` — active filters maintained here.
- `reference-old-filters/` — older filters copied from the game folder for reference
  (git-ignored, read-only history).
- `CLAUDE.md` — PoE2 filter syntax reference and repo conventions.

## Using a filter in-game
Copy the `.filter` into:
`C:\Users\jonat\OneDrive\Documentos\My Games\Path of Exile 2`
then select it in the game's UI (Options → Game → Item Filter).

## References
- Filter syntax: <https://www.pathofexile.com/item-filter/about>
- Exact item names / values: <https://poe2db.tw/us/>

## UI prototype (`ff-proto`)
A look-only prototype of the overlay in Path of Exile 2's own UI style, built on the shared skin
in `src/skin/` (fake data; nothing is read or written):

```bash
cargo run --bin ff-proto                    # left-docked, 660 px wide, full height; the X closes it
cargo run --bin ff-proto -- --tab config    # start on the CONFIG (or `economy`) tab
cargo run --bin ff-proto -- --screenshot shot.png   # render, save one frame, exit
```

## Game art tool (`ff-art`)
`tools/ff-art/` extracts the UI art listed in `assets/poe2/manifest.toml` from the game install
into `assets/poe2/` (PNGs and the Fontin fonts) and renders the Ferris background
(`assets/ui/ferris-bg.png`). It reads the install **read-only** and is its own Cargo workspace,
so its dependencies never enter `ff`. It uses the `poe_data_tools` library, which needs a
nightly toolchain. Re-run it after a game patch:

```bash
cd tools/ff-art
cargo run --release                                  # extract everything in the manifest
cargo run --release -- --game "D:\Games\Path of Exile 2"   # non-default install folder
cargo run --release -- list optionsmenu              # find virtual paths
cargo run --release -- dump "uiimages/common/" <scratch-dir>   # convert matches to PNG to look at
cargo run --release -- ferris                        # only re-render the Ferris background
```

The default install folder is `C:\Program Files (x86)\Steam\steamapps\common\Path of Exile 2`.
`sheet`, `info` and `zoom` help inspect candidates and screenshots (see `--help`).

## Credits and disclaimer
FerrisFilter is a third-party tool made for Path of Exile 2 players. This product isn't affiliated with or endorsed by Grinding Gear Games in any way.

- The UI art in `assets/poe2/` is extracted from Path of Exile 2 and belongs to Grinding Gear
  Games. It is used for a non-commercial fan tool and will be replaced if GGG objects.
- The Fontin fonts (`assets/poe2/fonts/`) are by exljbris (Jos Buivenga) and ship with Path of
  Exile 2.
- Ferris (`assets/ferris/` and the derived `assets/ui/ferris-bg.png`) is the unofficial community
  Rust mascot by Karen Rustad Tölva, released to the public domain
  ([rustacean.net](https://rustacean.net/)).
