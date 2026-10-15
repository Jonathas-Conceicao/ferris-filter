# Stack Research

**Domain:** Desktop game-overlay GUI on Windows, driving an existing headless Rust compiler
**Researched:** 2026-09-03
**Confidence:** MEDIUM-HIGH (versions verified against crates.io; overlay behavior verified against
egui issue tracker and the reference tool's docs, not yet against this machine)

## What is already settled

The existing crate is a headless CLI. Nothing in the current dependency set changes:

| Existing | Version | Keep? | Note |
|----------|---------|-------|------|
| `clap` | 4.6 | yes | CLI front end survives (`generate`/`sync`/`install`) |
| `serde` / `serde_json` / `serde_path_to_error` | 1.0 / 1.0 / 0.1 | yes | `dynamic.json` uses the same modeling + field-path errors |
| `miette` | 7.6 | yes | CLI diagnostics; the overlay renders errors itself |
| `thiserror` | 2.0 | yes | |
| `reqwest` + `rustls` + `tokio` | 0.13 / 0.23 / 1 | yes | `sync` only |
| `tempfile` | 3 | yes | atomic writes |

## Recommended additions

### Core

| Technology | Version | Purpose | Why |
|------------|---------|---------|-----|
| `eframe` | 0.36.1 (2026-08-07) | Window + app shell for the overlay | The batteries-included wrapper over `egui` + `winit` + `wgpu`. Immediate-mode fits a rule editor whose entire UI is a pure function of `dynamic.json` — no widget-state/model desync, which is exactly the failure mode a retained-mode toolkit would introduce against a file that is also editable by hand. |
| `egui` | 0.36.1 | Widgets | Same release train as `eframe`; pin them together. |
| `global-hotkey` | 0.8.0 (2026-05-01) | System-wide show/hide hotkey | Cross-platform, actively maintained, and the same family used by Tauri. Registers a Win32 hotkey so the overlay can be summoned while PoE2 has focus. |

### Considered and rejected

| Option | Why not |
|--------|---------|
| **Electron + Vue** (what Exiled Exchange 2 actually uses) | It is the proven path for this exact problem, and worth respecting as prior art. But it means a second language, a second toolchain, an IPC boundary to the Rust core, and a ~100 MB distributable bolted onto a ~5 MB single binary. The author prefers Rust (documented constraint), and our UI is one panel, not a browser. |
| `egui_overlay` / `screen_overlay` | Purpose-built for *transparent click-through* overlays — the Exiled Exchange 2 shape, where the tool paints tooltips over the game and must not eat clicks. Our editor is an opaque panel you summon, edit and dismiss. Adopting a passthrough crate buys complexity we do not need (see ARCHITECTURE for why this matters). |
| Raw Win32 layered windows via the `windows` crate | Maximum control, maximum surface area. Only reach for this if `eframe`'s always-on-top proves unreliable over PoE2 — which is a Phase-1 verification, not an assumption. |
| `tauri` | Brings back the web stack we rejected, plus a webview dependency. |
| `notify` (filesystem watching) | **Explicitly unnecessary.** PoE2 does not re-read filters on its own, so there is nothing to react to. Apply is user-driven. |

### Supporting

| Library | Version | Purpose | When |
|---------|---------|---------|------|
| `directories` | latest | Resolve the game folder / config paths portably | Only if the current hardcoded `%USERPROFILE%\OneDrive\Documentos\My Games\Path of Exile 2` default needs generalizing. Not required for v0.4. |

## Version notes

`egui`/`eframe` move fast and make breaking changes on nearly every 0.x minor. **Pin exact minor
versions** (`eframe = "0.36"`) and treat an upgrade as a deliberate task, not a `cargo update`
side effect. `egui` and `eframe` must always be the same minor.

## Sources

- [Exiled Exchange 2 — GitHub](https://github.com/Kvan7/Exiled-Exchange-2) and its
  [DEVELOPING.md](https://github.com/Kvan7/Exiled-Exchange-2/blob/master/DEVELOPING.md) — confirms
  Electron + Vue, main/renderer split, forked from Awakened PoE Trade
- [egui `ViewportBuilder` API](https://doc.servo.org/egui/viewport/struct.ViewportBuilder.html)
- [`global-hotkey` on crates.io](https://crates.io/crates/global-hotkey)
- [`win-hotkeys`](https://github.com/dnlmlr/windows-hotkeys) /
  [`win-hotkey`](https://crates.io/crates/win-hotkey) — Windows-only alternatives if
  `global-hotkey` disappoints
