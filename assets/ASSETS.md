# assets/ — provenance and license

Every binary asset bundled into a FerrisFilter binary is recorded here: what it is, where it
came from, and under what license it may be redistributed. An asset whose license cannot be
stated is not committed to this repository.

| File | What it is | Source | License |
|------|------------|--------|---------|
| `uwu.mp3` | Custom alert sound played by the `S`/`A` tier styles in generated filters (`CustomAlertSound "uwu.mp3"`). | Author-supplied. | Author-owned; bundled for personal/filter use. |
| `ferris.png` | 64×64 RGBA tray icon for the `ff` overlay binary (system tray presence, D-16/D-18). | Cropped and resized (square crop around the opaque content, then Lanczos3-resized to 64×64) from `rustacean-flat-happy.png`, downloaded from [rustacean.net](https://rustacean.net/) — the community site for the Ferris/Rustacean mascot artwork. | [CC0 1.0 Universal (Public Domain Dedication)](https://creativecommons.org/publicdomain/zero/1.0/) — rustacean.net's own footer states its mascot assets are released under CC0; no attribution required. (Note: the *site's favicon*, a different asset not used here, is separately licensed CC BY-NC-SA — not applicable to `ferris.png`.) |

## Regenerating `ferris.png`

The crop/resize was a one-off transform done with the project's own `image` crate dependency
(no new dependency added). If a different source crop is ever wanted, the source PNG is
`rustacean-flat-happy.png` at `https://rustacean.net/assets/rustacean-flat-happy.png` (460×307,
RGBA, CC0) — crop to a square around the non-transparent content with a little padding, then
resize to 64×64 (or 32×32) with a decent filter (e.g. Lanczos3).
