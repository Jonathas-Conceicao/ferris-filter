# Manual UI test filter

`test_ff.filter` is a hand-made filter that puts the overlay's UI cases on one screen. No
automated test loads it; it is for walking UAT checklists by hand. Keep it current when a new
UI case appears (one rule per case, short `# ff:name`, and a line in the table below).

## Using it without touching your live files

Run `ff` against a temp home and a temp game folder, so Apply never writes your real
`styles.json`, `tiers.json` or game-folder filter. PowerShell:

```powershell
$t = "$env:TEMP\ff-uitest"
New-Item -ItemType Directory -Force "$t\home", "$t\up\OneDrive\Documentos\My Games\Path of Exile 2" | Out-Null
Copy-Item styles-default.json, tiers-default.json, config.json, prices-fallback.json, tiers-fallback.json "$t\home"
Copy-Item tests\ui\test_ff.filter "$t\up\OneDrive\Documentos\My Games\Path of Exile 2\"
$env:FERRISFILTER_HOME = "$t\home"; $env:USERPROFILE = "$t\up"
cargo run --bin ff
```

Then pick `test_ff.filter` in CONFIG → Filter. Copy the filter again to reset it after an Apply.
With `USERPROFILE` pointing at the temp folder, the in-game reload goes to the real game only if
it is running; close it or turn **Reload filter in game after Apply** off.

## Cases

| # | Rule | What to look for |
|---|------|------------------|
| 1 | Bare Show | Amber triangle: an enabled rule with no conditions hides every rule below it. |
| 2 | Preset S | Style chip: the S minimap icon in its colour plus `S`. |
| 3 | Preset kite | Chip with the orange kite icon; the legacy name-only tag is rewritten to `<id> kite` on Apply. |
| 4 | Unknown style | Amber ring with `?`; the status line reports it and a click jumps here. |
| 5 | Grammar error | Red `!`; Apply is refused until it is fixed or disabled. |
| 6 | Grammar warning | Amber triangle; `LinkedSockets` is PoE1-only and shows as a raw row. |
| 7 | Disabled valid | Dimmed row; the round check enables it. |
| 8 | Disabled invalid | Padlock: can't be enabled. |
| 9 | Broken group | Amber broken link; a one-member `# ff:group` run is left as written. |
| 10 | Continue | Gold `Continue` word. |
| 11 | Move across Continue | Move it up past rule 10 with the arrows: the up/down-arrows flag appears. |
| 12 | Custom style | Chip shows only the blue star; expanded, every style line is filled in. |
| 13 | Simple Defences group | Two blocks shown as one row; Simple Defences card with Full AR + Full EV. |
| 14 | Between, booleans, sockets | One Between card (75-82), Yes/No switches, `Sockets >= 5GGG` keeps its letters. |
| 15 | Defences card | Armour and Energy Shield toggles with their operators. |
| 16 | Quoted list | Nine bases: the list scrolls after about seven. |
| 17 | Class outside groups | `Fishing Rods` listed on the compact line outside the grouped picker. |
| 18 | Extra action rows | Positional and optional sounds, `EnableDropSound`, `MinimapIcon -1`, `PlayEffect None` kept as rows. |
| 19 | Hidden preset | `Hide` header with the hidden style. |
| 20 | (unnamed) | Header reads just the index and `Show`; the summary line gives the context. |
| - | Sections | Rules 1-9 have no header; `Leveling` and `Endgame` are sections (rename, move, remove, collapse). |
| - | Economy | Last, locked, collapsed by default, two read-only rules. |
