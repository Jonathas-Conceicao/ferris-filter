# FerrisFilter

FerrisFilter is a game-styled overlay for Path of Exile 2 loot filters. It docks beside the game,
edits a filter file in the game folder in place, and keeps a price-tiered economy section in it up
to date from poe.ninja.

<table>
  <tr>
    <td align="center"><img src="docs/screenshots/rules.png" width="270" alt="Rules tab"></td>
    <td align="center"><img src="docs/screenshots/rule-editing.png" width="270" alt="A rule's conditions being edited"></td>
    <td align="center"><img src="docs/screenshots/rule-style.png" width="270" alt="A rule using a shared style"></td>
  </tr>
  <tr>
    <td align="center">General rule view</td>
    <td align="center">Rule editing</td>
    <td align="center">Styles on rules</td>
  </tr>
  <tr>
    <td align="center"><img src="docs/screenshots/config-style.png" width="270" alt="Style editor"></td>
    <td align="center"><img src="docs/screenshots/economy.png" width="270" alt="Economy tab"></td>
    <td align="center"><img src="docs/screenshots/economy-rule.png" width="270" alt="A generated economy rule"></td>
  </tr>
  <tr>
    <td align="center">Shared style support</td>
    <td align="center">Economy sync</td>
    <td align="center">Generated economy rules</td>
  </tr>
</table>

## What it does

The window is docked to the left edge of the screen and has four tabs.

- **Rules** lists the rules of the filter in the game folder (`dynamic.filter` by default). Rules
  can be added, deleted, duplicated, reordered, enabled or disabled, named and given a style.
  Clicking a rule opens its editor: every condition and action has its own control. Rules can be
  grouped into collapsible sections. The economy section is locked, sits last, and is regenerated
  on Apply.
- **Config** picks which filter file the Rules tab edits and holds the named styles: colours,
  font size, minimap icon, beam and sounds. A rule using a style follows every change to it.
- **Economy** syncs prices from poe.ninja, adds or removes the economy section and edits the tier
  cutoffs.
- **About** shows the version, credits and the disclaimer.

Edits stay in memory until **Apply**, which writes the styles, tiers and filter together and, if
the game is running, sends `/reloaditemfilter` to it. **Revert** and **Reload** undo edits.

## Using it

- Run Path of Exile 2 in windowed or windowed-fullscreen mode; the overlay cannot draw over
  exclusive fullscreen.
- `Ctrl+Shift+Z` shows or hides the window, only while the game or the overlay is the foreground
  window. The close button only hides it; **Quit** in the system tray icon exits.
- Only one `ff` runs at a time; starting it again brings up the running window.
- Close the chat box before pressing Apply, so the in-game reload is not typed into chat.

Your settings live in the FerrisFilter folder (the one holding `styles-default.json`):
`styles.json`, `tiers.json`, `prices.json` and `ff-settings.json`. Delete `styles.json` or
`tiers.json` to go back to the defaults.

## Building

```bash
cargo build --release
```

This produces `target/release/ff.exe`. `rust-toolchain.toml` selects the nightly channel.

## References

- Filter syntax: <https://www.pathofexile.com/item-filter/about>
- Exact item names / values: <https://poe2db.tw/us/>

## Credits and disclaimer

FerrisFilter is a third-party tool made for Path of Exile 2 players. This product isn't affiliated
with or endorsed by Grinding Gear Games in any way.

- The UI art in `assets/poe2/` is extracted from Path of Exile 2 and belongs to Grinding Gear
  Games. It is used for a non-commercial fan tool and will be replaced if GGG objects.
- The Fontin fonts (`assets/poe2/fonts/`) are by exljbris (Jos Buivenga) and ship with Path of
  Exile 2.
- Ferris (`assets/ferris/` and the derived `assets/ui/ferris-bg.png`) is the unofficial community
  Rust mascot by Karen Rustad Tölva, released to the public domain
  ([rustacean.net](https://rustacean.net/)).
