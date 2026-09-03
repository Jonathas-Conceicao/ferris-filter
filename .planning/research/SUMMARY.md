# Research Summary — v0.4 Overlay Filter Editor Rework

**Researched:** 2026-09-03
**Confidence:** MEDIUM-HIGH. The integration shape and the PoE2 constraints are solid; what remains
unverified is how `eframe`'s always-on-top actually behaves over PoE2 on this machine, which is a
Phase-3 verification with the author present, not a research question.

## The one-paragraph answer

Build it in Rust with `eframe`/`egui` 0.36 and `global-hotkey` 0.8 — not Electron, despite Electron
being what the reference tool uses — because our UI is one opaque panel rather than a browser, and
the author's stack is Rust. Extract the existing compiler into a `lib.rs` first so the overlay and
the CLI become two front ends over one core, leaving `config.json → universal.filter` and its
byte-for-byte oracle completely untouched. Make the dynamic feature work headlessly *before*
any GUI exists, so the risky, untestable surface is isolated in the final phases and a failure
there still leaves a working feature.

## Stack additions

| Crate | Version | Role |
|-------|---------|------|
| `eframe` | 0.36.1 | Overlay window + app shell |
| `egui` | 0.36.1 | Widgets (same minor as `eframe`, always) |
| `global-hotkey` | 0.8.0 | System-wide show/hide hotkey |

Nothing is removed. `notify` is explicitly *not* needed — there is nothing to watch.

## Feature table stakes

1. Library extraction with zero fixture churn
2. `dynamic.json` — modeled, persisted, round-tripped
3. `dynamic.json → dynamic.filter`, atomically written to the game folder
4. Always-on-top overlay window with a global hotkey
5. Ordinal rule list: add / delete / reorder / enable
6. Per-rule controls: style, class multi-select, rarity, three-checkbox defence, sockets, quality,
   corrupted — each condition independently togglable, disabled emitting nothing
7. Apply, plus an explicit "now hit Refresh in-game" affordance

## Watch out for

- **PoE2 cannot hot-reload.** Options → Game → Item Filter → **Refresh**, by hand, every time.
  Design Apply around that and say it in the UI. No watcher, no auto-apply.
- **Exclusive fullscreen defeats every overlay.** The game must be windowed or windowed-fullscreen.
- **Five of six known egui Windows overlay defects are avoided by being opaque and
  non-passthrough.** The sixth (#4458) is avoided by never sizing the window to the full screen.
  Also: do not name the binary `overlay` (#5512 is exactly that).
- **The oracle is the crown jewel.** If `artifact_guard` moves during the library extraction, the
  extraction is wrong — do not re-baseline it.
- **Regenerate fixtures hermetically** (temp dir + absolute binary path). Running `cargo run` from
  the repo root picks up a live `prices.json`; this cost time twice during v0.3.
- **`dynamic.filter` probably needs the economy sections too**, or selecting it in-game silently
  loses all currency/gem/unique highlighting. Decide deliberately.
- **Ordinal identity breaks under reorder** — re-resolve any "currently editing" selection when a
  rule moves.
- **Never hardcode class names.** `config.json` already enumerates them; a hand-typed list caused a
  real in-game load failure once already (`Class "Tablet"`, v0.2).
- **GUI has no test story** in a repo built on asserting emitted text. Keep every mutation,
  validation and compile step in the library behind unit-testable functions; let egui be a dumb view.

## ToS position

No GGG-approved third-party tools exist. The community standard that keeps Awakened PoE Trade and
Exiled Exchange 2 viable is: one action per keypress, no memory access, no injection, no
automation. This tool clears that bar by a wide margin — it never reads game memory or the
clipboard, never sends input, and performs no server actions; it writes a text file the game reads
when the player asks it to. Worth stating plainly in the README, without claiming approval.

## Suggested build order

1. Library extraction (oracle-proven, unblocks all)
2. Dynamic store + headless compile to `dynamic.filter`
3. Overlay shell (window, always-on-top, hotkey)
4. Rule editor UI + Apply

## Sources

- [Exiled Exchange 2](https://github.com/Kvan7/Exiled-Exchange-2) · [site](https://exiledexchange2.net/) · [FAQ](https://exiledexchange2.net/faq/)
- [Awakened PoE Trade FAQ](https://snosme.github.io/awakened-poe-trade/faq) · [PoE forum ToS thread](https://www.pathofexile.com/forum/view-thread/3474232)
- [egui `ViewportBuilder`](https://doc.servo.org/egui/viewport/struct.ViewportBuilder.html) · [egui issues](https://github.com/emilk/egui/issues)
- [`global-hotkey`](https://crates.io/crates/global-hotkey) · [`win-hotkeys`](https://github.com/dnlmlr/windows-hotkeys)
- [PoE2 loot filter guide](https://www.bamboogaming.net/poe2/loot-filters) · [Item filter guide](https://pathofexile.fandom.com/wiki/Guide:Item_filter_guide)
