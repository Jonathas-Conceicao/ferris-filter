# Phase 13: Overlay Shell - Research

**Researched:** 2026-09-04
**Domain:** Windows-only desktop overlay GUI (eframe/egui) rendered above a running game process,
summoned by a global hotkey, living in a system tray
**Confidence:** HIGH on the mechanism findings below the winit/egui source line (verified against
pinned-version source, not just docs) — this is the strongest kind of evidence available short of
running it against a live PoE2 session. MEDIUM on "how it behaves specifically over PoE2" and on
the visual/theming reach, which are inherently only provable in-game. LOW/ASSUMED items are called
out individually.

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

**Binary layout and build shape**
- **D-01: The GUI is the default build.** `eframe` / `egui` are plain, non-optional dependencies —
  no Cargo feature flag, no workspace split. The author's rationale: the overlay is how the tool
  will normally be used, so it should be what `cargo build` produces and what a double-clicked exe
  opens. This overrides the workspace-split shape sketched in `ARCHITECTURE.md`, which assumed the
  CLI stayed the primary front end.
- **D-02: `ff` becomes the GUI binary; the existing console CLI moves to `ff-cli`.** The
  long-name `ferrisfilter` binary stays a console CLI, unchanged. `default-run = "ff"` therefore
  needs no edit — it simply now resolves to the GUI.
  - **Verified during discussion:** all 23 integration-test call sites use
    `Command::cargo_bin("ferrisfilter")`; **none** reference `ff`. The rename touches zero tests.
- **D-03: The GUI binary declares the Windows GUI subsystem** so no console window flashes behind
  it. This is exactly why the CLI cannot be folded into the same binary — that subsystem has no
  usable stdout. Two binaries over one library.
- **D-04: The binary must not be named `overlay`** (egui #5512 gives a package with that name a
  black background). `ff` is clear of it.

**Summon and dismiss**
- **D-05: The hotkey reveals the stripe without activating it.** The game keeps keyboard focus and
  the character keeps responding; a summon mid-map must never eat movement keys.
- **D-06: A click inside the stripe activates it normally (takes focus); losing focus hides it.**
  Standard OS click-away behavior — the author confirmed this is what they were reaching for.
- **D-07: No global mouse hook. "Click outside dismisses" is explicitly cut as a separate
  mechanism.** Seeing clicks that land outside our own window requires a low-level system-wide
  input hook, which contradicts this project's no-input-monitoring stance and adds complexity for
  a behavior D-06 already covers. Author: "cut it if it adds too much complexity and we can
  reevaluate later."
- **D-08: Keyboard input follows overlay focus.** Once clicked into, the stripe receives typed
  text. **This lifts a constraint on Phase 14:** typed numeric entry for sockets and quality is
  viable, so its controls need not be mouse-only.
- **D-09: The toggle hotkey is `Shift+Z`, hardcoded this phase.**
  **Known trade, accepted by the author:** `global-hotkey` registers system-wide, so `Shift+Z` is
  captured in *every* application — including PoE2's chat box and any text field on the machine.
  It is a one-constant change (`Ctrl+Shift+Z`, `Alt+Z`) if it proves annoying in play.
- **D-10: Hotkey registration failure must be surfaced** (via the tray), never silent. Another
  application may already own the combo.

**Window shape**
- **D-11: A vertical stripe** — takes vertical space, uses only the horizontal space it needs. All
  filter-configuration UI eventually lives inside it.
- **D-12: Never sized to the full screen** (egui #4458: `always_on_top` displays incorrectly when
  window size equals screen size). Full-height-*ish*, deliberately under the screen height.
- **D-13: Opaque, non-passthrough, always-on-top** — the configuration that dodges five of the six
  documented egui/eframe Windows overlay defects.
- **D-14: No OS decorations, but a custom-drawn header plate.** Revised after the author supplied
  reference screenshots — both references have a header, so the window is borderless at the OS
  level (all chrome is ours to style) with a PoE-style header plate drawn inside it: title on the
  left, `✕` on the right. **The `✕` hides the stripe; tray → Quit exits the process.** Two
  different actions, deliberately.
- **D-15: Position and width are not persisted this phase.** The stripe opens the same way every
  launch. No settings file exists yet and this phase does not introduce one.

**System tray**
- **D-16: A system tray icon shows the overlay is running.** The process lives in the tray; the
  stripe is a view it shows and hides. Requested by the author so there is always a visible sign
  the tool is alive even when the stripe is hidden.
- **D-17: Right-click menu with Show and Quit; left-click toggles the stripe** the same way the
  hotkey does. Quit is the only exit (see D-14).
- **D-18: The icon is Ferris (the Rust mascot)** — "anything for now". It belongs in `assets/`,
  alongside the existing `uwu.mp3`.

**Phase 13 payload**
- **D-19: The stripe shows the rules from `dynamic.json` in ordinal order, read-only.** No status
  chrome — the author explicitly ruled out store path, price source, destination folder and any
  pricing information. The rule list is the content; everything else is noise.
- **D-20: All mutation, plus Reset and Apply, stays in Phase 14.** The editing controls land in the
  space the read-only list already occupies.

**Game awareness and conduct**
- **D-21: No PoE2 process or window detection this phase.** The windowed / windowed-fullscreen
  requirement is documented for the user, not detected. Detection is a nice-to-have that would pull
  in window enumeration for a failure the author will hit once and then know.
- **D-22: No memory reads, no input injection, no global input hooks.** Reaffirmed — the tool
  writes a text file and nothing else. Per `PITFALLS.md` §3 the author has already closed the ToS
  question: no disclaimers in the product.

**Visual design (author-supplied references, 2026-09-04)**
- **D-23: Layout follows Exiled Exchange 2's price-check panel** —
  `reference/ee2-panel-layout.png`. Measured from the screenshot: the panel occupies
  **x 794→1254 on a 1920×1080 screen — 460 px wide (24% of width), full height**, docked as a
  vertical column against the game. Content fills only the top portion; the rest of the stripe
  stays empty background rather than the window shrinking to fit. Structure to copy: a slim header
  bar with a centered title and a close control; content grouped into bordered boxes with slightly
  contrasting fills; small-caps section labels; zebra-striped list rows.
  **Take the structure, not the colors** — EE2's palette is a cool Tailwind slate (`#2D3748`
  ground, `#1A202C` boxes, `#EDF2F7` text) and is explicitly not what we want.
- **D-24: Palette and styling follow PoE2's own Options menu** —
  `reference/poe2-options-palette.png`. The author's goal: *"I want the app to look somewhat like
  a game feature extension."* Values sampled directly from that screenshot:

  | Role | Hex | Where it was sampled |
  |------|-----|----------------------|
  | Row ground (base) | `#0D0D0D` | settings row background |
  | Row ground (alternate) | `#121212` | adjacent zebra row |
  | Panel ground | `#141414` | panel body between sections |
  | Input / dropdown fill | `#090909` | Language dropdown — inputs recess *darker* than rows |
  | Header plate | `#3B3326` | the ornate "Options" title plate |
  | Label text (warm tan) | `#E3CD97` | "Language", "Auto-equip" row labels |
  | Section header (cream) | `#F1E4C2` | "LANGUAGE", "GUIDANCE", "GENERAL" |
  | Value text | `#FEFEFE` | dropdown values |
  | Control glyph (check) | `#C4C4A8` | checked toggle |
  | Button text | `#DEDEDE` | SAVE / DEFAULTS / CLOSE |
  | Tab active | `#B3AF93` | "GAME" tab |
  | Tab inactive / disabled | `#88856E` | "GRAPHICS", "UI" tabs |
  | Warm accent / destructive | `#A36C3C` | the red-bronze "USE DEFAULT" button |

  Typographic character: **small-caps serif throughout**, labels left / controls right, section
  headers brighter and larger than row labels, near-black grounds with warm tan-to-cream text.
  Toggles in PoE2 are **circular**, not square — worth matching, and it is what Phase 14's
  per-condition enable checkboxes should look like.
  `#A36C3C` is the accent to reserve for Phase 14's Reset, since the game uses it for the
  equivalent "use default" action.
- **D-25: The Dracula palette stays in the filter output, not in the GUI chrome.** This reverses
  Claude's earlier discretionary choice. The two palettes now have clean, separate jobs: Dracula
  colors the *items the filter paints in the game world* (pinned in `config.json` `styles`), while
  the *overlay chrome* is PoE-native so the tool reads as part of the game. Do not "harmonize"
  them — the split is intentional.

### Claude's Discretion
- Which screen edge the stripe docks to (EE2's reference sits left of the inventory panel).
- Exact stripe width — D-23's 24% is the reference, not a mandate.
- Layout of the read-only rule list within the PoE2 idiom.
- Whether to reproduce the ornate gold filigree of the header plate or just its flat bronze fill —
  a flat plate is expected to read closer to the game than a bad imitation of its artwork.
- The mechanism for show-without-activate (native eframe/winit capability vs. dropping to the raw
  `HWND`) — a research question, not a design one. **Resolved by this research: raw HWND, see
  Pattern 1 below.**
- Tray-icon crate selection. **Resolved by this research: `tray-icon` 0.24.2, see Standard Stack.**

### Deferred Ideas (OUT OF SCOPE)
- **Click-outside-dismiss as its own mechanism** — cut in favor of hide-on-focus-loss (D-07).
  Revisit only if D-06 feels wrong in play; it would need a low-level global mouse hook, which
  D-22 currently rules out.
- **Configurable hotkey** — hardcoded `Shift+Z` this phase (D-09). Becomes worth doing at the same
  time as any settings file.
- **Position / width persistence** — needs a settings file that does not exist yet (D-15).
- **PoE2 process and exclusive-fullscreen detection** — documented, not detected (D-21).
- **Editing, Reset and Apply in the stripe** — held at the phase boundary; this is Phase 14
  (EDIT-01..09), and the read-only list is deliberately the shape it grows into.
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|--------------------|
| SHELL-01 | An always-on-top window renders above Path of Exile 2 while the game runs in windowed or windowed-fullscreen mode. | `ViewportBuilder::with_always_on_top()`/`with_window_level(WindowLevel::AlwaysOnTop)` (native, no raw calls needed) + D-12's sub-screen-sizing to dodge egui #4458 — see Architecture Patterns and Common Pitfalls |
| SHELL-02 | A global hotkey toggles overlay visibility while the game holds focus, so it can be summoned without alt-tabbing. | `global-hotkey` 0.8.0 thread-affinity + polling pattern (Pattern 3), combined with the raw-HWND show/hide mechanism (Pattern 1) that is the load-bearing finding of this research — native `ViewportCommand::Visible` alone is **not** sufficient after the first toggle (see Summary) |
| SHELL-03 | The overlay window is opaque, non-passthrough, and never sized to the full screen — the configuration that avoids the documented egui/eframe Windows overlay defects. | Confirmed as a `ViewportBuilder` configuration question only (no `with_transparent`, no `with_mouse_passthrough`, sub-screen `with_inner_size`) — already the direction `ARCHITECTURE.md`/`PITFALLS.md` established; this research adds no new risk here beyond what was already known |
</phase_requirements>

## Summary

The single most consequential finding in this research overturns the framing of research question 1
as originally posed. `ViewportBuilder::with_active(false)` **does** suppress activation — but reading
winit 0.30.13's Windows backend directly (`window_state.rs`, `apply_diff`) shows this only holds for
the *very first* time the window's `VISIBLE` flag transitions to true. winit tracks a
`MARKER_ACTIVATE` bit that starts equal to `attributes.active`; the first time `VISIBLE` is applied
with that bit unset, winit calls `ShowWindow(hwnd, SW_SHOWNOACTIVATE)` **and immediately flips the
bit to true**. Every subsequent `ViewportCommand::Visible(true)` — exactly what a hotkey-toggled
overlay sends on the second, third, and every later summon — takes the `else` branch and calls
`ShowWindow(hwnd, SW_SHOW)`, which **does** activate. D-05 ("the hotkey reveals the stripe without
activating it") is only satisfiable on first launch through egui's native API; every subsequent
toggle needs the raw-HWND fallback the CONTEXT document already anticipated as a live possibility.
The fix is not exotic: cache the `HWND` once via `raw-window-handle` (eframe 0.36.1 re-exports
0.6.2) and drive **both** the show and the hide transitions with direct `ShowWindow` calls,
bypassing `ViewportCommand::Visible` entirely after window creation, so there is one source of
truth for visibility and no risk of winit's internal flag cache drifting from reality.

The second load-bearing finding closes research question 3: eframe was **not** always safe to hide.
[emilk/egui#5229](https://github.com/emilk/egui/issues/5229) documents that on Windows, a window set
to `Visible(false)` could never be shown again, because eframe only processed queued viewport
commands during `RedrawRequested`, and Windows stops delivering that event to invisible windows.
[PR #7905](https://github.com/emilk/egui/pull/7905) fixed this for the **0.34.0** release (well
before our pinned 0.36.1): invisible windows now get a throttled ~10 Hz heartbeat
(`WaitUntil` instead of `Poll`, redraw requests coalesced to ≥100 ms apart) specifically so queued
commands keep draining. This is good news for our hotkey-polling design: even a fully hidden window
still calls `update()` roughly every 100 ms, which is more than enough to notice a
`GlobalHotKeyEvent` and reveal itself. Nothing about our raw-HWND bypass changes this, because the
throttle is keyed off `window.is_visible()` (a live `IsWindowVisible` call), not winit's cached flag.

Tray icon and hotkey integration both turn out to be simpler than the open questions worried: both
`global-hotkey` 0.8.0 and `tray-icon` 0.24.2 create their own hidden hook windows and rely entirely
on **whatever Win32 message loop already runs on the thread they were created on** — they do not
spawn their own loop and do not need `winit`'s `with_msg_hook`. Since eframe's `run_native` pumps a
single `GetMessage`/`DispatchMessage` loop on the main thread, and message loops are per-thread (not
per-window) on Win32, creating the `GlobalHotKeyManager` and the `TrayIcon` on that same main thread
— before or during `eframe::run_native`'s callback — is sufficient. Both expose a global
`XxxEvent::receiver()` crossbeam channel that the egui `update()` closure polls with `try_recv()`
each frame; this is the officially documented integration pattern for both crates and is shown
verbatim in `tray-icon`'s own `examples/egui.rs`.

**Primary recommendation:** build the window shell natively via `ViewportBuilder` (SHELL-01/03 need
nothing exotic), but implement visibility toggling with a cached raw `HWND` and direct
`ShowWindow(SW_SHOWNOACTIVATE)`/`ShowWindow(SW_HIDE)` calls rather than `ViewportCommand::Visible`
(SHELL-02's actual hard part); poll `global-hotkey`/`tray-icon` event receivers inside `update()`
on the main thread with no extra event-loop plumbing; keep the visibility state machine, theme
definition, and `dynamic.json` view-model derivation as pure, unit-tested library functions per
`PITFALLS.md` §8.

## Architectural Responsibility Map

This phase has no browser/server/CDN tiers — it is a native Windows desktop process. The table below
substitutes the analogous native-app tiers so the planner can sanity-check where each capability's
logic belongs.

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| Window creation, always-on-top, decorations | Windowing shell (eframe/winit) | OS/Win32 | `ViewportBuilder` covers this natively; no raw calls needed |
| Show-without-activate on repeat toggles | OS/Win32 (raw HWND) | Windowing shell | winit's own API only handles the first reveal (see Summary); every later reveal needs a direct `ShowWindow(SW_SHOWNOACTIVATE)` call |
| Hide-on-focus-loss decision (the state machine) | Application/Library (`overlay::visibility`) | Presentation (egui `update()` feeds it the focus flag each frame) | **Corrected during plan verification.** This row originally said the state machine "belongs beside the UI code, not in the library", contradicting this document's own "Recommended Project Structure" and "Verification story" sections and `PITFALLS.md` §8. The transition function is pure — `fn advance(state, focused, toggle) -> state`, no egui types in its signature — so it lives in the library with `#[test]` coverage. Plan 13-03 follows the corrected placement. |
| Global hotkey registration & polling | Windowing shell (main thread) | — | Must be created on the same thread as the winit event loop (Win32 message-loop affinity) |
| Tray icon & menu | Windowing shell (main thread) | — | Same thread constraint as the hotkey manager |
| Reading `dynamic.json` into a display list | Application/Library (`ferrisfilter` crate) | Presentation | The read-only view-model derivation is a pure function over `Config`/`Rule`; keep it in the library so it is unit-testable per `PITFALLS.md` §8 |
| Theming (`Visuals`, custom painting) | Presentation (egui) | — | No library involvement; pure UI styling |
| `dynamic.json` → filter compile | Application/Library (`compile.rs`/`dynamic.rs`, Phase 12) | — | Already built; Phase 13 only *reads*, does not compile or write |

## Standard Stack

### Core

| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| `eframe` | 0.36.1 (2026-08-07) `[VERIFIED: crates.io]` | Window/app shell | Already locked in `STACK.md`; re-confirmed current as of today, no newer 0.36.x or 0.37 exists |
| `egui` | 0.36.1 (2026-08-07) `[VERIFIED: crates.io]` | Widgets/immediate-mode UI | Same release train, must share eframe's exact minor (unchanged guidance from `STACK.md`) |
| `global-hotkey` | 0.8.0 (2026-05-01) `[VERIFIED: crates.io]` | System-wide hotkey | Already locked; re-confirmed current, no 0.9 exists |
| `tray-icon` | 0.24.2 (2026-07-27) `[VERIFIED: crates.io + GitHub tags]` | System tray icon + menu | **New selection this phase** (Claude's discretion per CONTEXT). Same publisher (`tauri-apps`) as `global-hotkey`, so both share dependency conventions (`windows-sys`, `crossbeam-channel`) and the exact `TrayIconEvent::receiver()` / `GlobalHotKeyEvent::receiver()` polling idiom |
| `image` | 0.25.10 (2026-03-10) `[VERIFIED: crates.io]` | Decode the embedded Ferris PNG into RGBA for `tray_icon::Icon::from_rgba` | This is the crate `tray-icon`'s own `examples/egui.rs` uses for icon loading — not a guess, it's the documented pattern |
| `windows-sys` | 0.61.2 (latest) `[VERIFIED: crates.io]` | Direct `ShowWindow`/`SW_SHOWNOACTIVATE`/`SW_HIDE` calls for the show-without-activate fallback | Needed because winit's own `Visible` command cannot be trusted after the first reveal (see Summary). `windows-sys` (not the higher-level `windows` crate) matches what `global-hotkey`/`tray-icon` already pull in transitively, avoiding a second binding style in the dependency graph |

**Version pin discipline (unchanged from `STACK.md`):** `eframe`/`egui` must share an exact minor
(`"0.36"`). This phase adds no reason to relax that.

**Installation:**
```bash
cargo add eframe@0.36 egui@0.36 global-hotkey@0.8 tray-icon@0.24 image@0.25 --no-default-features --features image/png
cargo add windows-sys@0.61 --features Win32_Foundation,Win32_UI_WindowsAndMessaging
```
(`image` only needs the `png` decoder for a single embedded icon asset — `--no-default-features`
avoids pulling in every image codec `image` supports by default, a meaningful compile-time saving.)

### Supporting

| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| `raw-window-handle` | 0.6.2, transitive via `eframe` — **do not add directly** | Supplies `RawWindowHandle::Win32(Win32WindowHandle { hwnd: NonZeroIsize, .. })` from `frame.window_handle()` | eframe already depends on and re-exports the matching version; adding it directly risks a version-mismatch trap if it drifts (the exact hazard `winit`/`eframe`/`raw-window-handle` version skew the research prompt called out) |
| `keyboard-types` | 0.7, transitive via `global-hotkey` | Parses `Shift+Z` into a `HotKey` (`Modifiers::SHIFT`, `Code::KeyZ`) | Already pulled in; no direct dependency needed |

### Alternatives Considered

| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| `tray-icon` | Raw `Shell_NotifyIconW` via `windows-sys` directly | More control, zero extra dependency weight, but reimplements icon lifecycle + menu handling that `tray-icon` already gets right cross-input-device (keyboard activation of tray menu, DPI, etc.); not worth it for one tray icon |
| `windows-sys` for the HWND fallback | `windows` crate (higher-level, `windows::Win32::...`) | `windows` is nicer to write against but is a **separate** binding generation from `windows-sys`; since `global-hotkey`/`tray-icon` already pull `windows-sys` transitively, adding `windows` instead means two full sets of Win32 bindings compiling. `windows-sys` is the leaner, ecosystem-consistent choice for the ~3 raw calls this phase needs |

**Version verification performed:**
```bash
cargo search eframe        # eframe = "0.36.1"
cargo search egui          # egui = "0.36.1"
cargo search global-hotkey # global-hotkey = "0.8.0"
cargo search tray-icon     # tray-icon = "0.24.2"
cargo search image         # image = "0.25.10"
cargo search windows-sys   # windows-sys = "0.61.2"
```
All six ran clean against the live crates.io index today; no name typos, no cross-ecosystem
confusion risk (all are long-established Rust-only crate names).

## Package Legitimacy Audit

`slopcheck` could not be installed in this research environment (`pip` is not present on this
machine's `PATH`). Per the graceful-degradation protocol this would normally force every package to
`[ASSUMED]`; however, `eframe`/`egui`/`global-hotkey` are **not new** — they were already locked in
the milestone's `STACK.md` (2026-09-03) and are carried forward, not newly proposed here. The two
genuinely new-to-this-repo packages this phase adds (`tray-icon`, and its `image`/`windows-sys`
supporting deps) were manually audited against crates.io metadata in lieu of `slopcheck`:

| Package | Registry | Age | Downloads (all-time) | Source Repo | slopcheck | Disposition |
|---------|----------|-----|-----------------------|--------------|-----------|-------------|
| `eframe` | crates.io | since Jan 2021 | 17.6M+ | github.com/emilk/egui | N/A (tool unavailable) | Approved — locked in prior research, unchanged |
| `egui` | crates.io | since 2021 | very high (149k+ downloads on 0.36.1 alone) | github.com/emilk/egui | N/A | Approved — locked in prior research, unchanged |
| `global-hotkey` | crates.io | since Jan 2023 | 4.96M | github.com/tauri-apps/global-hotkey | N/A | Approved — locked in prior research, unchanged |
| `tray-icon` | crates.io | since Dec 2022 | 26.9M | github.com/tauri-apps/tray-icon | N/A | Approved (manual audit) — official Tauri org repo, MIT/Apache-2.0, 66 published versions, active release cadence |
| `image` | crates.io | since Nov 2014 | 181M | github.com/image-rs/image | N/A | Approved (manual audit) — the de facto standard Rust image crate, `image-rs` org |
| `windows-sys` | crates.io | Microsoft-published | very high | github.com/microsoft/windows-rs | N/A | Approved (manual audit) — official Microsoft binding crate |

**Packages removed due to slopcheck `[SLOP]` verdict:** none — slopcheck did not run.
**Packages flagged as suspicious `[SUS]`:** none found via manual audit.

*`slopcheck` was unavailable at research time for all six packages above.* Manual verification
(publisher identity, age, download volume, matching source repository) was substituted and found no
red flags for any of them, but per protocol the planner should still gate the two genuinely new
additions (`tray-icon`, `windows-sys` as a direct dependency) behind a lightweight
`checkpoint:human-verify` before the first `cargo add` lands in a commit, since no automated
slop-detection tool actually ran.

## Architecture Patterns

### System Architecture Diagram

```
                     ┌─────────────────────────────┐
                     │   Windows message loop      │   ← single GetMessage/DispatchMessage
                     │   (main thread, owned by     │     pump; ALL hidden-window hook
                     │   eframe::run_native)        │     procedures ride this thread
                     └───────────────┬─────────────┘
                                     │ dispatches WM_HOTKEY, WM_ACTIVATE,
                                     │ tray WM_APP messages, WM_PAINT, ...
              ┌──────────────────────┼───────────────────────┐
              ▼                      ▼                       ▼
    ┌───────────────────┐  ┌──────────────────┐   ┌────────────────────┐
    │ GlobalHotKeyManager│  │  Stripe HWND      │   │ TrayIcon (Shell_   │
    │  hidden hook window│  │  (eframe/winit)   │   │ NotifyIcon window) │
    │  RegisterHotKey     │  │  ViewportBuilder: │   │  right-click menu  │
    │  Shift+Z             │  │  always_on_top,   │   │  left-click toggle │
    └─────────┬───────────┘  │  no decorations,  │   └──────────┬─────────┘
              │              │  taskbar=false     │              │
              │ crossbeam    └────────┬───────────┘              │ crossbeam
              │ channel               │                          │ channel
              ▼                       ▼                          ▼
    ┌──────────────────────────────────────────────────────────────────┐
    │  egui update() closure — runs every frame (throttled ~10Hz while  │
    │  the stripe is hidden, per PR #7905's WaitUntil heartbeat)        │
    │                                                                    │
    │  1. GlobalHotKeyEvent::receiver().try_recv() → toggle requested?  │
    │  2. TrayIconEvent::receiver().try_recv()      → toggle/quit?      │
    │  3. visibility state machine (Hidden/Shown/Focused) advances      │
    │  4. if reveal:  raw ShowWindow(hwnd, SW_SHOWNOACTIVATE)            │
    │     if hide:    raw ShowWindow(hwnd, SW_HIDE)                     │
    │  5. ctx.input(|i| i.viewport().focused) checked for auto-hide      │
    │  6. read-only rule list rendered from an already-loaded            │
    │     `Vec<Rule>` snapshot (see below)                               │
    └───────────────────────────────┬────────────────────────────────┘
                                     │ one-shot read at startup
                                     ▼
                     ┌───────────────────────────────┐
                     │ ferrisfilter::dynamic /        │
                     │ ferrisfilter::compile (library) │
                     │  ensure_store → load Config     │
                     │  (NOT compile_store — no write) │
                     └───────────────────────────────┘
```

### Recommended Project Structure

```
src/
├── lib.rs                 # add `pub mod overlay;` (or keep GUI code entirely in the bin — see note)
├── bin/
│   ├── ff.rs               # NEW: GUI entry point (was the CLI shim; console body moves out)
│   └── ff-cli.rs           # NEW: the former ff.rs console body, unchanged behavior
├── main.rs                 # unchanged — `ferrisfilter` console binary
└── overlay/                # NEW, in the library per PITFALLS §8 ("logic in the library")
    ├── mod.rs
    ├── visibility.rs        # the Hidden/Shown/Focused state machine — PURE, unit-testable
    ├── theme.rs              # fn poe2_visuals() -> egui::Visuals — PURE, unit-testable
    ├── view_model.rs          # fn rule_rows(&Config) -> Vec<RuleRow> — PURE, unit-testable
    └── widgets.rs               # circular checkbox / header plate paint helpers — thin, not unit-tested (needs a Painter)
```

**Note on `overlay/` living in the library vs. the `ff` binary:** `PITFALLS.md` §8 says "if a bug can
only be found by clicking, it is in the wrong layer." The visibility state machine, the theme
definition, and the `dynamic.json` → display-row mapping are all pure data transformations with no
`egui::Context` dependency — they belong in the library so `cargo test` covers them without a
window. The parts that *must* touch `egui::Painter`/`egui::Ui` (custom circular checkboxes, the
header plate, the actual `eframe::App::update` loop) cannot usefully be unit-tested without
`egui_kittest` (see Verification Story below) and can live in the `ff` binary crate directly, or in
an `overlay::widgets` module in the library if the planner prefers one crate root. Either placement
is defensible; keeping the *pure* pieces out of the binary is the part that matters.

### Pattern 1: Show without activating (repeat toggles)

**What:** Cache the raw `HWND` once, then drive show/hide directly via `windows-sys::ShowWindow`,
bypassing `ViewportCommand::Visible` for both directions so there is one source of truth.

**Why not just `with_active(false)` + `ViewportCommand::Visible(true)`:** confirmed by reading
`winit` 0.30.13's Windows backend (the exact version `eframe` 0.36.1 pins) —
`crates/winit/src/platform_impl/windows/window_state.rs`, `apply_diff`:

```rust
// winit 0.30.13, window_state.rs — approximate reconstruction from the source read during research
if new.contains(WindowFlags::VISIBLE) {
    let flag = if !self.contains(WindowFlags::MARKER_ACTIVATE) {
        self.set(WindowFlags::MARKER_ACTIVATE, true);   // ← latches after first show
        SW_SHOWNOACTIVATE
    } else {
        SW_SHOW                                          // ← every later show
    };
    unsafe { ShowWindow(window, flag); }
}
```

`window_flags.set(WindowFlags::MARKER_ACTIVATE, attributes.active)` at creation means
`with_active(false)` only buys the *first* `SW_SHOWNOACTIVATE`. The second, third, and every later
`ViewportCommand::Visible(true)` — which is exactly what a hotkey toggle sends — activates the
window and steals focus from the game. This is a hard blocker for D-05 as stated if the native API
is trusted for the whole feature, not just the first launch.

**Code sketch** (Windows-only; matches D-01's Windows-only scope):

```rust
// overlay/mod.rs (or directly in the ff binary)
use raw_window_handle::{HasWindowHandle, RawWindowHandle};
use windows_sys::Win32::Foundation::HWND;
use windows_sys::Win32::UI::WindowsAndMessaging::{ShowWindow, SW_HIDE, SW_SHOWNOACTIVATE};

/// Cached once `update()` first runs with a live window handle.
struct OverlayHandle(HWND);

fn hwnd_of(frame: &eframe::Frame) -> Option<OverlayHandle> {
    match frame.window_handle().ok()?.as_raw() {
        RawWindowHandle::Win32(h) => Some(OverlayHandle(h.hwnd.get() as HWND)),
        _ => None,
    }
}

fn reveal(hwnd: &OverlayHandle) {
    unsafe { ShowWindow(hwnd.0, SW_SHOWNOACTIVATE) };
}

fn conceal(hwnd: &OverlayHandle) {
    unsafe { ShowWindow(hwnd.0, SW_HIDE) };
}
```

`HWND` in `windows-sys` 0.60+ is `*mut core::ffi::c_void` (an opaque pointer typedef), and
`Win32WindowHandle::hwnd` (from `raw-window-handle` 0.6.2) is `std::num::NonZeroIsize` — the
`.get() as HWND` cast above is the standard conversion, but **the exact wrapper shape should be
re-checked against whichever `windows-sys` minor is actually pinned** at implementation time; this
is the one place in this research where the type-level plumbing (not the approach) is worth a
five-minute sanity compile before writing the surrounding logic.

**Why this doesn't fight D-06 (click activates normally):** `SW_SHOWNOACTIVATE` only affects the one
`ShowWindow` call that makes the window visible — it does **not** set any persistent extended style
like `WS_EX_NOACTIVATE`. Once visible-but-unfocused, the window is fully normal from Windows' input
routing perspective: a click inside it triggers the OS's default `WM_MOUSEACTIVATE` →
`MA_ACTIVATE` behavior exactly as any other window, satisfying D-06 with **no extra code** — the
raw-HWND fallback for D-05 does not require also building a WS_EX_NOACTIVATE toggle for D-06.

### Pattern 2: Hide-on-focus-loss state machine (avoids the "never-focused → immediate hide" hazard)

**What:** `ctx.input(|i| i.viewport().focused)` (an `Option<bool>` on `ViewportInfo`, populated from
`WindowEvent::Focused` on the winit side) reports true only once the OS actually activates the
window (e.g. via the click described above). A naive `if !focused { hide() }` fires on the very
first frame after `reveal()`, because a window shown via `SW_SHOWNOACTIVATE` has never been focused
— `focused` reads `Some(false)` (or `None` before the first paint) immediately.

**State machine:**

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum OverlayState {
    Hidden,
    /// Shown non-activated; waiting for either a click-driven focus event or another toggle.
    ShownUnfocused,
    /// The window has been observed focused at least once since the last reveal —
    /// NOW losing focus means "the user clicked away," so hide.
    ShownFocused,
}

fn advance(state: OverlayState, focused_this_frame: bool, toggle_requested: bool) -> OverlayState {
    use OverlayState::*;
    match (state, focused_this_frame, toggle_requested) {
        (_, _, true) if state == Hidden => ShownUnfocused,        // hotkey/tray reveals it
        (_, _, true)                    => Hidden,                 // hotkey/tray hides it again
        (ShownUnfocused, true, false)   => ShownFocused,           // user clicked in
        (ShownFocused, false, false)    => Hidden,                 // focus lost → auto-hide (D-06)
        (s, _, false)                    => s,                      // no-op frame
    }
}
```

This is pure, has no `egui`/`winit` types in its signature, and is exactly the kind of function
`PITFALLS.md` §8 wants unit-tested headlessly (see Verification Story).

### Pattern 3: Hotkey + tray polling inside `update()`

**What:** Both `global-hotkey` and `tray-icon` publish events on a global `crossbeam_channel`
receiver reachable via `XxxEvent::receiver()` — no `EventLoopProxy` wiring is required for a
correctness bar this phase; it only matters for *latency*, and the ~100 ms hidden-window heartbeat
(Pattern 1's neighbor, PR #7905) already bounds that acceptably for a manually-pressed hotkey.

```rust
impl eframe::App for OverlayApp {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        if let Ok(event) = global_hotkey::GlobalHotKeyEvent::receiver().try_recv() {
            if event.id == self.toggle_hotkey.id()
                && event.state == global_hotkey::HotKeyState::Pressed
            {
                self.toggle_requested = true;
            }
        }
        if let Ok(event) = tray_icon::TrayIconEvent::receiver().try_recv() {
            // left-click toggles, same as the hotkey (D-17)
        }
        if let Ok(event) = tray_icon::menu::MenuEvent::receiver().try_recv() {
            // Show / Quit menu items (D-17)
        }

        // ... advance the OverlayState state machine, reveal()/conceal() as needed ...

        // Keep the loop ticking while hidden — belt-and-suspenders on top of the
        // built-in ~10Hz heartbeat; cheap, and makes the polling interval explicit
        // rather than implicit in eframe's internals.
        ctx.request_repaint_after(std::time::Duration::from_millis(50));
    }
}
```

**Thread affinity — the one hard constraint:** `GlobalHotKeyManager::new()` and
`TrayIconBuilder::build()` must both be called on the thread that runs the Win32 message loop —
i.e., the same thread `eframe::run_native` drives (the main thread, by default, on Windows). Build
both **before** calling `eframe::run_native`, or inside the `NativeOptions` app-creation closure —
either works since both run on the main thread pre-loop-start. Do not spawn either on a background
thread (a documented `tray-icon` constraint, quoted in its own crate docs: *"An event loop must be
running on the thread... it doesn't need to be the main thread but you have to create the tray icon
on the same thread as the event loop"*).

### Pattern 4: Tray icon from an embedded PNG

```rust
// examples/egui.rs from tray-icon 0.24.2 — the crate's own documented pattern
fn load_icon() -> tray_icon::Icon {
    let bytes = include_bytes!("../assets/ferris.png");
    let image = image::load_from_memory(bytes)
        .expect("embedded icon must decode")
        .into_rgba8();
    let (width, height) = image.dimensions();
    tray_icon::Icon::from_rgba(image.into_raw(), width, height)
        .expect("valid icon dimensions")
}
```

### Anti-Patterns to Avoid

- **Mixing `ViewportCommand::Visible` with raw `ShowWindow` calls on the same window.** Pick one
  mechanism per direction and stick to it (Pattern 1 recommends raw calls for *both* directions, to
  avoid winit's cached `WindowFlags` drifting from the real OS state — a drifted cache can cause a
  later `ViewportCommand::Visible(false)` to no-op because winit's diff sees "no change needed").
- **Setting `WS_EX_NOACTIVATE` as a persistent extended style.** It would solve D-05 but silently
  break D-06 (clicks would no longer activate the window either) — not needed per Pattern 1's
  finding that `SW_SHOWNOACTIVATE` is sufficient and scoped to only the reveal moment.
- **Creating the `GlobalHotKeyManager` or `TrayIcon` on a spawned thread.** Both require the Win32
  message loop of their *own* creating thread; a background thread with no message loop means
  `RegisterHotKey`'s `WM_HOTKEY` and the tray's menu/click messages are never delivered.
- **Sizing the window to the screen's exact dimensions.** Reaffirming `PITFALLS.md` #4458 — pick a
  height deliberately short of `available_monitors()[n].size()`.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| System-wide hotkey registration | Raw `RegisterHotKey`/`UnregisterHotKey` + your own hidden window + `WM_HOTKEY` dispatch | `global-hotkey` (already locked) | It already does exactly this, cross-platform, with a channel-based API — reimplementing buys nothing this phase needs |
| Tray icon lifecycle + menu | Raw `Shell_NotifyIconW` + a hand-rolled popup menu via `TrackPopupMenu` | `tray-icon` | DPI handling, icon refresh on explorer.exe restart, and keyboard-accessible menu activation are all handled; not worth reimplementing for one icon |
| PNG decoding for the tray icon | A hand-rolled PNG parser or `include_bytes!` of raw pre-decoded RGBA | `image` crate (`load_from_memory` + `into_rgba8`) | `tray-icon`'s own example already depends on it; one well-audited codec beats a bespoke decoder for an asset that never changes at runtime |
| Circular toggle switches | A custom `egui::Widget` built from scratch with no reference | Adapt `egui_demo_lib`'s `toggle_switch.rs` (official example, same repo/version as our pinned egui) — same four-step pattern (size → `allocate_exact_size` → interact → `ui.painter()`), just a circle instead of a pill | egui ships no circular checkbox, but the *pattern* for hand-painting an interactive widget is an official, version-matched reference — don't invent the widget-construction idiom from scratch |

**Key insight:** every "don't hand-roll" item above already has a crate-level or example-level
reference pinned to the *exact* versions this phase uses — there is no reason to freehand any of
these mechanisms.

## Common Pitfalls

### Pitfall 1: Trusting `with_active(false)` for the whole feature, not just first launch

**What goes wrong:** The window steals focus from the game on the second summon (works fine the
first time, so it looks correct in a quick manual check and then fails in play).
**Why it happens:** winit's `MARKER_ACTIVATE` flag latches to `true` after the very first
`ShowWindow` call — see Pattern 1 above, sourced directly from winit 0.30.13.
**How to avoid:** Use the raw-HWND `ShowWindow(SW_SHOWNOACTIVATE)` approach for every reveal, not
just to "unblock" a bug found later.
**Warning signs:** A manual test that only summons the overlay once and calls it done will not catch
this — the test must summon, dismiss, and summon again while the game holds focus.

### Pitfall 2: The "never-focused → immediate hide" bug

**What goes wrong:** The stripe appears for exactly one frame and vanishes.
**Why it happens:** `ctx.input(|i| i.viewport().focused)` reads `false` on the very first frame
after a non-activated reveal (see Pattern 2) — a naive `if !focused { hide() }` fires immediately.
**How to avoid:** Gate the auto-hide behind having observed `focused == true` at least once since
the last reveal (the `ShownUnfocused` → `ShownFocused` transition in Pattern 2).
**Warning signs:** The window flashes and disappears; looks like a crash or a race, not a logic bug,
when first observed.

### Pitfall 3: Non-optional `eframe`/`egui`/`wgpu` inflating every build, including `cargo test`

**What goes wrong:** `cargo test` (which only needs the library + `ferrisfilter`/`seed_prices`
binaries) becomes noticeably slower once eframe/egui/wgpu are plain `[dependencies]`, because Cargo
compiles the crate's full dependency graph regardless of which binary target is requested — D-01
made this an explicit, accepted tradeoff (no feature flag, no workspace split), but the planner
should not be surprised by a colder `cargo test` after this phase lands.
**Why it happens:** `wgpu` (an eframe transitive dependency) is one of the larger dependency trees in
the Rust ecosystem to compile from a cold cache.
**How to avoid:** Nothing to *avoid* — this is an accepted consequence of D-01, not a defect. Flag it
so the planner can budget CI/local iteration time expectations correctly, and so a slow `cargo test`
after this phase isn't mistaken for a regression to chase.
**Warning signs:** A clean-checkout `cargo test` taking meaningfully longer than it did before Phase
13; this is expected, not a bug.

### Pitfall 4: `windows-sys` version duplication in the dependency graph

**What goes wrong:** Nothing user-visible, but the build compiles two separate copies of
`windows-sys` (one pinned to `"0.59"` via `global-hotkey`, one to `">=0.60,<=0.61"` via `tray-icon`
and our own direct dependency), since these are semver-incompatible pre-1.0 ranges Cargo cannot
unify.
**Why it happens:** `windows-sys` treats every 0.x minor as a breaking change; `global-hotkey` 0.8.0
and `tray-icon` 0.24.2 pinned different windows at different times.
**How to avoid:** Nothing actionable — this is normal for the Rust Windows-bindings ecosystem and
does not cause a compile error, just modest extra compile time. Do not "fix" it by trying to force a
single version via `[patch]`; that risks breaking whichever crate expected its pinned range.
**Warning signs:** `cargo tree -d` showing `windows-sys` twice — expected, not a bug to chase.

### Pitfall 5: `windows_subsystem = "windows"` silently swallowing test/debug output

**What goes wrong:** If the attribute is placed unconditionally, a developer running the GUI binary
manually during debugging gets no `eprintln!`/panic-message visibility (no console is attached).
**Why it happens:** `#![windows_subsystem = "windows"]` at the crate root of the `ff` binary
suppresses Windows' automatic console allocation for that process, full stop — there's no
"debug-only" default.
**How to avoid:** The `#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]` idiom
(confirmed as a standard, widely-used pattern in the Rust GUI ecosystem — corroborated by community
discussion, not itself in the egui/eframe docs) keeps a console attached for `cargo run` (debug
profile) while suppressing it for release builds. Recommended.
**Warning signs:** A release-mode `ff.exe` that appears to do nothing when something goes wrong,
because the panic message went to a console window nobody can see.

### Pitfall 6: egui's `ab_glyph` rasterizer has no OpenType feature support (no true small caps)

**What goes wrong:** Registering a font with an `smcp` (small-caps) OpenType feature and expecting
egui to render lowercase text as small capitals — it will just render the font's normal lowercase
glyphs; the feature is silently ignored.
**Why it happens:** egui/epaint uses `ab_glyph` for rasterization, which does raw glyph-index
lookups from Unicode codepoints — it has no shaping engine (no HarfBuzz-equivalent), so OpenType
features (`smcp`, `c2sc`, ligatures, etc.) are invisible to it. (Confirmed: egui's own
`cosmic-text`/Parley integration attempts, tracked in
[emilk/egui#3378](https://github.com/emilk/egui/issues/3378) and
[PR #5784](https://github.com/emilk/egui/pull/5784), are unmerged explorations, not shipped
behavior as of 0.36.1.)
**How to avoid:** This confirms CONTEXT's own fallback plan is correct and necessary, not merely
acceptable: use an all-caps-only display face (see Font section) and fake the size hierarchy PoE2's
UI uses (bigger/brighter section headers, smaller row labels) via `egui::text::LayoutJob` with
per-run `TextFormat.font_id` sizes, rather than attempting true small caps.
**Warning signs:** None at compile time — this only shows up visually, so budget a look-at-the-
actual-render verification step, not just "it compiled."

## Code Examples

### Font registration (small-caps-adjacent, all-caps display face)

**Recommended font: Cinzel** (Google Fonts, SIL Open Font License 1.1 — explicitly redistributable,
commercial use permitted, no attribution file required beyond keeping the license text with the
font if redistributed, which `assets/` already does for `uwu.mp3`'s equivalent convention).
Cinzel is itself an **all-caps-only** display face (modeled on first-century Roman inscription
letterforms, the same design lineage as Trajan) — it has no distinct lowercase glyphs, which
sidesteps Pitfall 6 entirely: there is no "wrong" lowercase rendering to worry about, because the
font simply does not have one. `[CITED: fonts.google.com/specimen/Cinzel — OFL 1.1]`

```rust
// egui::FontDefinitions registration — standard pattern, confirmed API shape as of 0.36.1
let mut fonts = egui::FontDefinitions::default();
fonts.font_data.insert(
    "cinzel".to_owned(),
    egui::FontData::from_static(include_bytes!("../assets/Cinzel-Regular.ttf")).into(),
);
fonts
    .families
    .entry(egui::FontFamily::Name("cinzel".into()))
    .or_default()
    .insert(0, "cinzel".to_owned());
ctx.set_fonts(fonts);
```

Fake the size-hierarchy small-caps look with `LayoutJob` (section headers full-size, row labels one
step smaller — `TextFormat.extra_letter_spacing` is a real field on 0.36.1, confirmed via docs.rs,
useful for the "reduced letter-spacing" fallback CONTEXT flagged as acceptable):

```rust
let mut job = egui::text::LayoutJob::default();
job.append(
    "LANGUAGE",
    0.0,
    egui::TextFormat {
        font_id: egui::FontId::new(16.0, egui::FontFamily::Name("cinzel".into())),
        color: egui::Color32::from_rgb(0xF1, 0xE4, 0xC2), // section header cream, D-24
        extra_letter_spacing: 1.5,
        ..Default::default()
    },
);
```

### Visuals mapping (D-24 palette → `egui::Visuals` fields)

```rust
// overlay/theme.rs — a PURE function, unit-testable without a window (see Verification Story)
pub fn poe2_visuals() -> egui::Visuals {
    let mut v = egui::Visuals::dark();
    v.panel_fill = egui::Color32::from_rgb(0x14, 0x14, 0x14);      // Panel ground #141414
    v.extreme_bg_color = egui::Color32::from_rgb(0x09, 0x09, 0x09); // Input/dropdown fill #090909
    v.window_fill = egui::Color32::from_rgb(0x0D, 0x0D, 0x0D);      // Row ground (base) #0D0D0D
    v.widgets.inactive.bg_fill = egui::Color32::from_rgb(0x0D, 0x0D, 0x0D);
    v.widgets.inactive.fg_stroke.color = egui::Color32::from_rgb(0xE3, 0xCD, 0x97); // label warm tan
    v.widgets.active.fg_stroke.color = egui::Color32::from_rgb(0xC4, 0xC4, 0xA8);   // checked glyph
    v.hyperlink_color = egui::Color32::from_rgb(0xA3, 0x6C, 0x3C); // warm accent, reserved for Phase 14 Reset
    v
}
```

The **zebra row** effect (D-23 layout reference) and the **circular toggle** / **header plate**
(D-24 palette reference) are *not* reachable through `Visuals` alone — `Visuals` covers flat
per-state widget colors and window/panel grounds, but has no "alternate row color" or "circular
indicator shape" concept. Those need direct `egui::Painter` calls: alternate row fills via
`ui.painter().rect_filled(row_rect, 0.0, alt_color)` painted manually per row index (egui's built-in
`Visuals::striped` flag exists for `Grid`/`Table` but is a simpler on/off, not a designed color
pair — for full control over the two zebra tones, paint manually), and the circular toggle via the
`toggle_switch.rs` pattern cited under Don't Hand-Roll, substituting a circle for the pill shape.

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|---------------|--------|
| Hidden eframe windows never process queued `ViewportCommand`s (Windows) | Invisible windows get a throttled ~10 Hz heartbeat that drains the command queue | eframe/egui **0.34.0** (PR #7905, merged 2026-03-24) | Directly unblocks the hide/show toggle this phase needs; our pin (0.36.1) is well past this fix |
| `egui`'s text renderer has experimented with `cosmic-text`/Parley for shaping | Still `ab_glyph`-based as of 0.36.1; no shipped OpenType feature support | Ongoing, unresolved as of research date | Confirms no automatic small-caps path exists — must fake it (Pitfall 6) |

**Deprecated/outdated:** Nothing else in the previously-committed `STACK.md`/`PITFALLS.md` research
(dated 2026-09-03) is stale as of today — versions re-checked above are unchanged or bumped by a
patch release only (`eframe`/`egui` 0.36.0 → 0.36.1, released two days after the milestone's
research, no breaking changes expected at a patch bump).

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|----------------|
| A1 | The exact `windows-sys` `HWND` representation (`*mut c_void` newtype vs. a tuple struct) and the precise `.hwnd.get() as HWND` cast syntax, for whichever `windows-sys` 0.60/0.61 patch ends up pinned | Pattern 1 code sketch | A five-minute compile-fix, not a design risk — the *approach* (raw ShowWindow bypassing ViewportCommand) is HIGH confidence; only the exact cast syntax is unverified against the literal pinned patch version |
| A2 | `#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]` is safe and standard, and does not affect `cargo test`/other binaries | Pitfall 5 | Low — this is extremely well-established Rust/Windows behavior (subsystem is a per-binary-crate attribute), corroborated by community discussion during this research, but not pinned to an official Rust reference doc |
| A3 | Cinzel's OFL 1.1 license is compatible with bundling the `.ttf` directly in `assets/` and shipping it in the compiled binary via `include_bytes!` | Font section | Low — OFL is specifically designed to permit bundling in software; this is the license's primary use case. Re-confirm the license file text before committing the font, as a final check, not because the license type is in doubt |
| A4 | No `slopcheck` run against any of the six packages (tool unavailable in this environment) | Package Legitimacy Audit | Manual crates.io audit substituted; low risk given the extremely high download counts and matching official-org repos for every package, but the planner should still add a lightweight human-verify checkpoint per protocol |

## Open Questions (RESOLVED — routed to manual verification during planning)

> Both questions below are inherently answerable only by running the thing. Rather than leaving
> them open, planning routed each to a named manual checkpoint:
> **Q1** → Plan 13-05 Task 2, live-PoE2 checklist step 1 (`SW_SHOWNOACTIVATE` against PoE2's
> windowed-fullscreen rendering). **Q2** → Plan 13-03 Task 3, which cites this question directly
> and checks that the stripe still resizes and repaints correctly after several raw hide/reveal
> cycles. Neither blocks implementation.

1. **Does `SW_SHOWNOACTIVATE` behave identically when the target process (PoE2) is a DirectX
   fullscreen-borderless game vs. a plain windowed app?**
   - What we know: The Win32 API contract for `SW_SHOWNOACTIVATE` doesn't distinguish by what the
     currently-foreground window is doing internally; it's a documented, stable API since Windows
     3.1.
   - What's unclear: Whether PoE2's own input-handling (which may re-grab raw input focus on its own
     cadence in windowed-fullscreen) could visually "flash" activate/deactivate in a way that isn't
     visible from a synthetic test.
   - Recommendation: This is exactly the kind of thing D-21 already flagged as "the user will hit
     once and then know" — treat the roadmap's "verified against a running PoE2" success criterion
     (Phase 13 success criterion #1) as the actual gate, not a synthetic Windows-only test.

2. **Does the raw-HWND bypass (Pattern 1) interact badly with eframe's own internal event-loop
   `ControlFlow` decisions (`Poll` vs `WaitUntil`) if winit's cached `VISIBLE` flag never matches
   reality?**
   - What we know: `window.is_visible()` (used by PR #7905's throttle logic) calls `IsWindowVisible`
     directly, not the cached flag — so the throttle heartbeat should stay correct regardless of who
     called `ShowWindow`.
   - What's unclear: Whether any *other* winit-internal logic (resize handling, IME positioning,
     `WindowEvent::Focused` dispatch) reads the cached flag instead of live OS state anywhere else in
     the frame-processing path — this would require a full winit source audit beyond what this
     research budget covered.
   - Recommendation: The planner should treat this as a task-level verification item (does the
     stripe still resize/repaint correctly after several raw-hide/raw-reveal cycles?), not a blocker
     to starting implementation.

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|--------------|-----------|---------|----------|
| Rust toolchain (`rustc`/`cargo`) | Everything | ✓ | 1.90.0-nightly | — |
| Windows 11 | The whole phase (Windows-only per constraints) | ✓ | 10.0.26200 | — (no fallback needed; out of scope by design) |
| A running Path of Exile 2 process (windowed/windowed-fullscreen) | Manual verification of SHELL-01/02 (Roadmap success criteria) | Not verified in this research session — requires the author to launch the game | — | None — this is explicitly a human-in-the-loop verification step, not something research can substitute for |
| Internet access for `cargo add`/crates.io | Adding the new dependencies | ✓ (used throughout this research session) | — | — |

**Missing dependencies with no fallback:** none block *starting* the phase; the live-PoE2
verification step is inherently manual and already scoped that way by the roadmap.

## Security Domain

`security_enforcement` is absent from `.planning/config.json`, so it is treated as enabled per
protocol — but this phase has essentially no attack surface in the traditional web-app sense: no
network listeners, no authentication, no user-supplied untrusted input beyond a local read of
`dynamic.json` (already a trusted, locally-authored file per Phase 12's model). Most ASVS categories
are not applicable to a single-user, offline, read-only-this-phase desktop overlay.

### Applicable ASVS Categories

| ASVS Category | Applies | Standard Control |
|----------------|---------|-------------------|
| V2 Authentication | No | No accounts, no auth surface |
| V3 Session Management | No | No sessions |
| V4 Access Control | No | Single-user local process |
| V5 Input Validation | Marginal — `dynamic.json` is read (not written) this phase | Reuses Phase 12's existing `serde`/`deny_unknown_fields` validated `Config` model — no new parsing surface introduced |
| V6 Cryptography | No | No secrets, no network calls this phase |

### Known Threat Patterns for this stack

| Pattern | STRIDE | Standard Mitigation |
|---------|--------|-----------------------|
| A malformed/hand-edited `dynamic.json` causing a panic when the overlay reads it at startup | Denial of Service (local) | Already mitigated by Phase 12's `serde_path_to_error`-backed load path (`DynamicError`/`CompileError`) — the overlay should surface a load failure as an in-tray/on-window message, not crash the process (worth an explicit task: what does the stripe show if `dynamic.json` fails to parse?) |
| A malicious binary named `ff.exe` elsewhere on `PATH` being launched instead of the real one | Spoofing | Out of scope for this phase — a general Windows software-distribution concern, not specific to the overlay mechanism |

## Sources

### Primary (HIGH confidence — read directly against the pinned version)
- winit 0.30.13 source, `crates/winit/src/platform_impl/windows/window_state.rs` (`apply_diff`,
  `WindowFlags::MARKER_ACTIVATE`/`VISIBLE`) and `window.rs` (`set_visible`, `is_visible`,
  `focus_window`) — fetched at the exact tag eframe 0.36.1 pins
- [emilk/egui#5229](https://github.com/emilk/egui/issues/5229) — the "can't re-show a hidden window"
  bug, Windows-specific
- [emilk/egui PR #7905](https://github.com/emilk/egui/pull/7905) — the fix, merged for 0.34.0, with
  the throttled-heartbeat mechanism description
- `eframe` 0.36.1 `Frame` API (docs.rs) — `winit_window()`, `HasWindowHandle`/`HasDisplayHandle` via
  `raw-window-handle` 0.6.2
- `egui` 0.36.1 `ViewportBuilder`/`ViewportCommand` API (docs.rs)
- `egui` 0.36.1 `Visuals`/`WidgetVisuals`/`TextFormat` API (docs.rs)
- `global-hotkey` 0.8.0 and `tray-icon` 0.24.2 `Cargo.toml` (github.com/tauri-apps, exact tags
  `global-hotkey-v0.8.0` / `tray-icon-v0.24.2`) — dependency versions (`windows-sys`, no
  `raw-window-handle`, no `winit` runtime dependency)
- `tray-icon` 0.24.2 `examples/egui.rs` (github.com/tauri-apps/tray-icon) — the icon-loading and
  event-polling pattern quoted above
- `egui_demo_lib` 0.36.1 `toggle_switch.rs` (github.com/emilk/egui) — the official custom-widget
  painting pattern

### Secondary (MEDIUM confidence)
- crates.io registry metadata (download counts, publish dates, repository links) for `eframe`,
  `egui`, `global-hotkey`, `tray-icon`, `image`, `windows-sys` — fetched live today
- Community discussion corroborating `#![cfg_attr(not(debug_assertions), windows_subsystem =
  "windows")]` as a standard idiom (Pitfall 5) — consistent with well-known Rust/Windows semantics
  but not sourced from an official reference doc in this session
- fonts.google.com Cinzel specimen page — OFL 1.1 licensing, all-caps-only design lineage

### Tertiary (LOW confidence)
- None retained — items that could not be corroborated beyond a single WebSearch snippet were either
  dropped or explicitly logged under Assumptions/Open Questions instead of stated as fact.

## Metadata

**Confidence breakdown:**
- Standard stack (versions/compatibility): HIGH — every version cross-checked against crates.io and
  the actual pinned-tag `Cargo.toml` for transitive dependency conflicts
- Show-without-activate / hide-on-focus-loss mechanism: HIGH — verified against winit source at the
  exact pinned version, not just documentation or hearsay
- Tray/hotkey thread-affinity integration: HIGH — quoted directly from the crates' own docs and
  official examples
- Theming reach (Visuals vs. custom painting): MEDIUM — API surface is HIGH confidence
  (docs.rs-verified field lists), but the *visual result* against the actual PoE2 reference
  screenshots can only be confirmed by rendering it
- Font/small-caps: MEDIUM-HIGH — the "no OpenType shaping" claim is corroborated by an open,
  unmerged egui issue/PR pair, which is decent but not a changelog-level confirmation
- "Behavior against a live, running PoE2": explicitly NOT verifiable in this research session — see
  Open Questions #1 and Environment Availability

**Research date:** 2026-09-04
**Valid until:** ~30 days for the version pins (egui/eframe/global-hotkey/tray-icon all move at a
roughly monthly cadence); the winit-source-level mechanism findings (Patterns 1–2) are stable
architecture knowledge and should hold well past that, unless a future eframe/egui minor changes how
`ViewportCommand`/`Frame::window_handle` work.

---

## Risks for the planner

1. **The Pattern 1 raw-HWND mechanism is the single highest-risk piece of new code in this phase.**
   It is the one place this research recommends bypassing the high-level API entirely in favor of
   direct Win32 calls. Budget a dedicated task for it with its own acceptance criterion ("summon,
   dismiss, and summon again while a foreground window holds keyboard focus — the second summon
   must not steal focus"), not a sub-step folded into "build the window."
2. **The window-shell layer (eframe App, `update()`, raw-HWND calls, tray/hotkey wiring) is
   inherently the least testable code in this codebase** (confirmed by `PITFALLS.md` §8, unchanged
   by this research). The mitigations in "Recommended Project Structure" (pull the visibility state
   machine, theme function, and view-model derivation into pure library functions) shrink but do not
   eliminate this — the actual win32-calling glue code has no automated test story beyond manual
   in-game verification, matching the roadmap's own manual success criteria for this phase.
3. **Compile-time regression** (Pitfall 3) is a one-time cost the whole team should expect, not
   chase as a bug, once `eframe`/`egui`/`wgpu` become non-optional dependencies of the single crate.
4. **Visual fidelity to the two reference screenshots** (D-23/D-24) cannot be verified by any
   automated means — `egui_kittest` (see Verification Story below) can assert on `Visuals` field
   values and layout structure, but "does this look like a game feature extension" is inherently a
   look-at-it judgment call, same as the roadmap's own phrasing implies.
5. **Font licensing and asset provenance.** Downloading and bundling Cinzel is low-risk (OFL 1.1),
   but it is still a new binary asset landing in `assets/` — worth a small, explicit task (download,
   verify license file, `include_bytes!`) rather than an incidental line in a larger task, so the
   license text travels with the font file the way `CLAUDE.md`'s conventions already expect for
   `uwu.mp3`.

## Verification story (research question 9)

`egui_kittest` (crates.io, version `0.36.1` — same release train as `egui`, confirmed current) is
the answer to "does egui offer a headless test harness": it drives an `egui::Context` and can render
to a headless framebuffer for snapshot-style assertions, without opening a real OS window. It is a
reasonable choice **if** the planner wants pixel/snapshot-level UI tests, but given `PITFALLS.md`
§8's guidance and this phase's read-only scope, the higher-leverage, lower-cost investment is
unit-testing the pure functions directly:

| What | Testable headlessly? | How |
|------|------------------------|-----|
| `overlay::visibility::advance()` (Pattern 2's state machine) | Yes, trivially | Plain `#[test]` — no egui/window types in the signature |
| `overlay::theme::poe2_visuals()` | Yes, trivially | Assert specific `Color32` field values match the D-24 hex table |
| `overlay::view_model::rule_rows(&Config)` (the read-only display mapping from `dynamic.json`'s
  loaded `Config`) | Yes | Plain `#[test]` against a fixture `Config`/`Rule` list — reuses the
  existing `Rule`/`When` types from `src/models/config.rs`, no new parsing needed |
| The raw-HWND show/hide calls, hotkey/tray event wiring, actual egui `update()` loop | No — needs a
  real window and, for the hotkey specifically, a real running process to receive `WM_HOTKEY` | Manual,
  in-game verification per the roadmap's own success criteria (this phase's success criteria are
  explicitly manual, as noted in the phase definition) |
| Circular checkbox / header plate paint code | Partially — `egui_kittest` *could* snapshot-test
  these, but given the phase's read-only scope and that Phase 14 will touch these same widgets
  again, deferring `egui_kittest` adoption to Phase 14 (when there's mutation logic worth snapshot-
  guarding) is a reasonable scope call, not a gap | — |

Recommend the planner require unit tests for the three "Yes" rows above as a normal task deliverable
(consistent with `DYN-08`'s precedent of "reachable headlessly, without the overlay"), and treat
everything in the "No" row as the explicit, accepted manual-verification surface the roadmap already
expects for this phase.

## Recommended task shape

Given the roadmap's own estimate ("1-2 plans"), and the risk concentration identified above, the
natural split is:

**Plan A — Cargo/binary restructuring + window shell that behaves correctly, no styling.**
1. `Cargo.toml`/binary rename (`ff` → GUI, `src/bin/ff.rs` → `src/bin/ff-cli.rs`, new dependencies,
   `windows_subsystem` on the GUI binary only) — mechanical, low risk, but touches build
   configuration every other task depends on; do it first and alone.
2. Minimal `eframe::App` that opens an always-on-top, undecorated, sub-screen-sized, opaque window
   (`ViewportBuilder` per D-11/D-12/D-13/D-14's OS-decoration-off decision) with no content yet —
   proves SHELL-01 and SHELL-03's static properties.
3. Hotkey registration + tray icon/menu, wired to the Pattern 3 polling loop, no visibility logic
   yet beyond "toggle a bool" — proves the thread-affinity and event-delivery mechanics in isolation
   from the harder focus-management problem.
4. The Pattern 1 + Pattern 2 mechanism: raw-HWND show/hide, the visibility state machine (as a
   library-level unit-tested function per the Verification Story), wired into the `update()` loop —
   this is where SHELL-02's "without alt-tabbing" and D-05/D-06's focus semantics actually get
   proven, and deserves its own acceptance criterion distinct from "the window opens."

**Plan B — Read-only content + theming.**
5. `overlay::view_model::rule_rows` (pure, unit-tested) reading `dynamic.json` via the Phase 12
   `ensure_store`/`load_inputs` path (read-only — no `compile_store`, no `write_filter` calls this
   phase) and a plain list rendering inside the stripe (D-19).
6. Font registration (Cinzel) + `overlay::theme::poe2_visuals()` (pure, unit-tested) + custom-painted
   header plate and circular status indicators if used in the read-only view — this is the highest
   iteration-count, most subjective task (visual fidelity to two reference screenshots), and
   benefits from being isolated from the mechanism work in Plan A so it can be revised without
   re-touching hotkey/focus logic.

If the roadmap's "1-2 plans" ends up as a single plan, Plan A's four items are the ones that gate
correctness (SHELL-01/02/03 and the phase's own "no PoE2 detection, no input injection" boundary,
D-21/D-22); Plan B's two items are comparatively low-risk and could be folded into the same plan's
later waves without materially changing the risk profile.
