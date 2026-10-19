# ff-art

Dev tool that extracts the Path of Exile 2 UI art FerrisFilter's skin uses. It reads the game
install **read-only** and writes the files listed in `assets/poe2/manifest.toml` (which game file
becomes which PNG) into `assets/poe2/`: PNGs, the packed minimap icon atlas and the Fontin fonts.
It also renders the Ferris background (`assets/ui/ferris-bg.png`) and the orange ABOUT Ferris
(`assets/ui/ferris-about.png`) from `assets/ferris/`.

It is never a dependency of `ff`. It is a member of the repo's Cargo workspace but not a default
member, so `cargo build` / `cargo test` at the root never build it. It uses the `poe_data_tools`
library, which needs nightly; the root `rust-toolchain.toml` selects the nightly channel (set a
dated nightly there if a new one breaks `poe_data_tools`).

## Running

From any folder in the repo (`cargo ff-art` is an alias for `cargo run -p ff-art --release --`,
in `.cargo/config.toml`):

```bash
cargo ff-art                                       # extract everything in the manifest
cargo ff-art --game "D:\Games\Path of Exile 2"     # non-default install folder
cargo ff-art list optionsmenu                      # find virtual paths
cargo ff-art dump "uiimages/common/" <scratch-dir> # convert matches to PNG to look at
cargo ff-art dump minimap/player <scratch-dir>     # the whole minimap icon sheet
cargo ff-art ferris                                # only re-render the Ferris images
cargo ff-art icon                                  # render assets/ferris/ferris.ico (7 sizes)
```

`sheet`, `info` and `zoom` help inspect candidates (see `--help`). Outputs always land in the
repo's `assets/` (override with `--root`). The default install folder is
`C:\Program Files (x86)\Steam\steamapps\common\Path of Exile 2`. Re-run it after a game patch.

If `ff.exe` is running, set `CARGO_TARGET_DIR` to a folder outside the repo: the workspace shares
one `target/`, which the running exe locks.

`assets/custom/` holds hand-made art and is never written by this tool.
