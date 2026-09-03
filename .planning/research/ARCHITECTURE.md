# Architecture Research

**Domain:** Game overlay + live rule editor over an existing deterministic compiler
**Researched:** 2026-09-03
**Confidence:** HIGH on the integration shape, MEDIUM on Windows overlay specifics

## Existing architecture (what must not break)

```
config.json ─┐
tiers.json  ─┼─→ models (serde) ─→ pricer ─→ emitter ─→ universal.filter
prices.json ─┘                                   ▲
                                                 └── locked by artifact_guard
                                                     + insta snapshot
                                                     against seed fixtures
```

`sync` (network, tokio/reqwest) writes `prices.json`. `generate` is pure and deterministic.
`install` copies. The whole thing is a library-shaped program trapped in a binary crate — there is
**no `lib.rs` target today** (noted in a v0.1 decision: "seed_prices duplicates minimal currency
structs locally, no lib target in crate").

## Target architecture

```
                    ┌──────────────────────────────┐
                    │  ferrisfilter-core  (lib)    │
                    │  models · pricer · emitter   │
                    └──────────────────────────────┘
                         ▲                    ▲
             ┌───────────┘                    └───────────┐
     ┌───────────────┐                            ┌────────────────┐
     │  CLI (bin)    │                            │ overlay (bin)  │
     │ generate      │                            │ eframe window  │
     │ sync          │                            │ rule editor    │
     │ install       │                            │ Apply button   │
     └───────────────┘                            └────────────────┘
        config.json                                  dynamic.json
             ↓                                            ↓
      universal.filter                             dynamic.filter
      (game folder)                                 (game folder)
```

**The two paths never touch each other.** `config.json → universal.filter` keeps its fixtures,
its snapshot and `artifact_guard` untouched — this is the single most important structural
property of the milestone, because it means the GUI work cannot regress the tested core.

### The library extraction is the real first phase

Everything else depends on it, and it is a pure refactor with a byte-for-byte oracle already in
place to prove it changed nothing. Shape:

- Add `src/lib.rs` exporting `models`, `pricer`, `emitter`, and the `generate` entry point.
- `src/main.rs` becomes a thin `clap` shell over the library.
- `artifact_guard` + `generate_snapshot` must stay green with **zero** re-baselining. If a
  fixture changes during extraction, the extraction is wrong.
- Bonus: the "seed_prices duplicates minimal currency structs locally (no lib target)" workaround
  from v0.1 can finally be deleted.

### Dynamic rule store

`dynamic.json` lives in the repo directory and holds an **ordered array of rules only** — no
styles, no economy. Style names are references into the pinned `config.json` `styles` block.

```jsonc
{
  "version": 1,
  "rules": [
    { "style": "hidden",      "when": { "class": ["Life Flasks", "Mana Flasks", "Charms"] } },
    { "style": "exceptional", "when": { "sockets": { "min": 3 }, "corrupted": false } }
  ]
}
```

Rules are identified **by ordinal position** (index in the array). No `name` field. This is
already how the filter engine thinks (first-match-wins), so the UI ordering *is* the semantics.

Reuses the existing `Rule` / `When` types wholesale. `When`'s fields are all `Option<…>`
already, which is exactly the "every condition independently togglable, disabled emits nothing"
requirement — **the model needs no change to support it.** The only divergence is that the
dynamic editor never sets `area_level`.

### Compile path

`dynamic.json → emit_rules → dynamic.filter`. This is the *same* `emit_rules` the CLI uses, so
dynamic output is grammatically identical to `universal.filter` blocks by construction. Two open
design points for the phase that builds it:

1. **Does `dynamic.filter` include the economy sections?** The pinned currency/gem/unique blocks
   come from `prices.json` via the pricer. If `dynamic.filter` is meant to be the filter you
   actually play with, it needs them — otherwise selecting it in-game loses all currency
   highlighting. Recommendation: yes, emit economy sections from the same pricer, with only the
   `rules` half coming from `dynamic.json`.
2. **Atomic write.** Reuse the existing `tempfile` write-then-rename discipline from `sync`, so a
   half-written `dynamic.filter` can never be left in the game folder.

### Overlay window

The critical insight: **this overlay does not need transparency or click-through.**

Exiled Exchange 2 needs both because it paints price tooltips over items in the game world and
must not swallow the player's clicks. A rule editor is a panel you summon, edit, Apply, and
dismiss. That means:

- `ViewportBuilder::with_always_on_top(true)` + `with_decorations(false)` — an ordinary opaque
  window that sits above the game.
- **No** `with_transparent`, **no** `with_mouse_passthrough`. This sidesteps the entire cluster of
  known eframe Windows transparency defects listed in PITFALLS.
- A `global-hotkey` registration toggles visibility so it can be summoned without alt-tabbing.

If always-on-top proves unreliable over PoE2 specifically, the fallback is a Win32
`SetWindowPos(HWND_TOPMOST)` via the `windows` crate on the raw window handle eframe exposes.

## Integration points

| Point | Direction | Note |
|-------|-----------|------|
| `emitter::emit_rules` | overlay → core | Reused unchanged; guarantees grammar parity |
| `pricer` + `prices.json` | overlay → core | For the economy half of `dynamic.filter` |
| `config.json` `styles` | overlay reads | Style dropdown is populated from the pinned palette |
| Class list | overlay reads | `weaponClasses`/`armourClasses`/`flaskCharmClasses` in `config.json` already enumerate every valid PoE2 class — the class picker should source from these, not a hardcoded list |
| Game folder path | overlay writes | Same default as `install`; reuse that resolution logic |

## Suggested build order

1. **Library extraction** — pure refactor, oracle-proven, unblocks everything.
2. **Dynamic store + compile** — `dynamic.json` model, persistence, `→ dynamic.filter`, all
   headless and unit-testable. At the end of this phase the feature works via a CLI command with
   no GUI at all.
3. **Overlay shell** — window, always-on-top, hotkey, verified against a running PoE2.
4. **Rule editor UI + Apply** — the widgets, wired to the store from step 2.

Steps 1–2 carry the project's existing testing discipline. Steps 3–4 are where the untested
surface lives, and isolating them last means a failure there leaves a working headless feature.

## Sources

- [egui `ViewportBuilder`](https://doc.servo.org/egui/viewport/struct.ViewportBuilder.html)
- [Exiled Exchange 2](https://github.com/Kvan7/Exiled-Exchange-2) — architecture reference
- [PoE2 filter location + refresh flow](https://www.bamboogaming.net/poe2/loot-filters)
