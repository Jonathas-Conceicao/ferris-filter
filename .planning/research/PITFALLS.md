# Pitfalls Research

**Domain:** Adding a Windows game overlay + GUI to a headless deterministic Rust compiler
**Researched:** 2026-09-03
**Confidence:** HIGH on the documented egui defects and the PoE2 constraints; MEDIUM on how they
manifest on this specific machine

## Blocking constraints (not bugs — facts to design around)

### 1. PoE2 never reloads a filter by itself

**Confirmed by the author and corroborated by community guides:** you edit the filter while the
game runs, then go **Options → Game → Item Filter → Refresh** to apply it. There is no watcher, no
hot-reload, and nothing an external process can do to force one.

**Consequence:** Apply is a deliberate, user-triggered write, and the UI must *say so* — after
Apply the overlay should tell the player to hit Refresh in-game. A tool that silently writes and
implies "done" will read as broken when drops do not change. Do not add a filesystem watcher,
debounced auto-write, or "live sync" affordance; they promise something the game cannot deliver.

### 2. The game must run in windowed or windowed-fullscreen

Exiled Exchange 2 documents this requirement, and it is inherent to how Windows composites
overlays: an exclusive-fullscreen DirectX swapchain will not let another top-level window draw
above it. **Exclusive fullscreen breaks any overlay, ours included.** This must be documented for
the user and ideally detected, not silently failed.

### 3. GGG's stance is conditional, not an approval

There are no GGG-approved third-party tools. The community line — and the reason Awakened PoE
Trade and Exiled Exchange 2 are used at scale without bans — is compliance: one action per
keypress, no memory reads or writes, no injection into the game process, no automation.

**We are comfortably inside that line, and further inside it than the price checkers:** this tool
never reads game memory, never reads the clipboard, never sends input to the game, and makes no
server actions at all. It writes a text file. Worth stating explicitly in the README so users can
assess it themselves — but do not claim it is "approved" or "ban-safe", because no one can.

## egui / eframe defects to avoid

The overlay-shaped configurations have a documented history of breaking on Windows specifically:

| Issue | Symptom | Our exposure |
|-------|---------|--------------|
| [#4451](https://github.com/emilk/egui/issues/4451) | Window transparency does not work on Windows | **Avoided** — we do not use transparency |
| [#4446](https://github.com/emilk/egui/discussions/4446) | Transparent window renders black instead | **Avoided** — same |
| [#5512](https://github.com/emilk/egui/issues/5512) | A package literally *named* `overlay` gets a black background | **Avoided**, but note the trap: do not name the crate/binary `overlay` |
| [#4091](https://github.com/emilk/egui/issues/4091) | `fullscreen` + `transparent` + `always_on_top` together crashed the immediate viewport | Partially avoided — we use `always_on_top` but neither of the other two |
| [#2537](https://github.com/emilk/egui/issues/2537) | `mouse_passthrough` makes the window invisible | **Avoided** — no passthrough |
| [#4458](https://github.com/emilk/egui/issues/4458) | `always_on_top` displays incorrectly when window size equals screen size | **Live risk** — keep the overlay a panel, never size it to the full screen |

**Prevention:** the decision in ARCHITECTURE to build an *opaque, non-passthrough, sub-screen-sized*
panel is not an aesthetic choice — it is what dodges five of these six. The one that remains
(#4458) is avoided by never making the window screen-sized.

### egui breaks its API on nearly every 0.x minor

Pin `eframe` and `egui` to the same exact minor (`0.36`) and treat upgrades as deliberate tasks.
A stray `cargo update` across a minor will not compile.

## Project-specific traps

### 4. Do not let the GUI regress the tested core

This repo's strongest asset is a byte-for-byte oracle (`artifact_guard` + insta snapshot). The
library extraction must produce **zero** fixture changes; if a fixture moves during a refactor
that is supposed to be behavior-preserving, that is a defect, not a re-baseline. Resist the
temptation to "just re-baseline" — it is how the byte-for-byte guarantee dies.

### 5. Fixture regeneration from the repo root (recurring, twice in v0.3)

Running `cargo run` from the repo root picks up a live `prices.json` from a prior `sync` instead
of the seed fixture. Regenerate hermetically: temp dir + the binary by absolute path, mirroring
`generate_on_seed()`. This bit the project in both Phase 9 and Phase 10 — it will bite again the
moment `dynamic.filter` needs a fixture.

### 6. `dynamic.filter` without economy sections is a trap

If Apply writes only the `rules` half, the player selects `dynamic.filter` in-game and silently
loses every currency, gem and unique highlight. Decide this deliberately (see ARCHITECTURE) rather
than discovering it in-game.

### 7. Half-written filter files

The game may read the file at any moment the player hits Refresh. Use the existing atomic
write-then-rename discipline (`tempfile`) that `sync` already applies to `prices.json`. A
truncated filter fails to load in-game and looks like a tool bug.

### 8. GUI code has no test story here

Every prior milestone was verifiable by asserting on emitted text. Widget layout is not. Mitigate
by keeping *all* logic — rule mutation, validation, compilation — in the library behind plain
functions that unit tests can drive, and letting the egui layer be a thin, dumb view. If a bug can
only be found by clicking, it is in the wrong layer.

### 9. Ordinal identity is fragile under reordering

Rules are indexed by position, and reordering changes every index. Any UI state that remembers "I
am editing rule 3" must be re-resolved after a move, or the user will silently edit the wrong
rule. Prefer holding a transient selection that is updated by the move operation itself.

### 10. Class lists must not be hardcoded

`config.json` already enumerates every valid PoE2 class in `weaponClasses` / `armourClasses` /
`flaskCharmClasses`. A hand-typed list in the GUI will drift, and PoE2 class names have already
caused a real production defect (the invalid `Class "Tablet"` rows that failed to load in-game,
removed in v0.2). Source the picker from config.

## Sources

- [egui issue tracker](https://github.com/emilk/egui/issues) — issues 2537, 4091, 4446, 4451, 4458, 5512
- [Awakened PoE Trade FAQ](https://snosme.github.io/awakened-poe-trade/faq) — ToS compliance reasoning
- ["Is Awakened PoE Trade allowed?" — official PoE forum thread](https://www.pathofexile.com/forum/view-thread/3474232)
- [Exiled Exchange 2 FAQ](https://exiledexchange2.net/faq/) — windowed-mode requirement, ban questions
- [PoE2 loot filter guide](https://www.bamboogaming.net/poe2/loot-filters) — Options → Game → Refresh flow
