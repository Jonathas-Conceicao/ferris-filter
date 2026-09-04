---
phase: 13-overlay-shell
plan: 01
subsystem: ui
tags: [eframe, egui, winit, rust, gui, overlay, windows]

# Dependency graph
requires:
  - phase: 12-dynamic-rule-store
    provides: "dynamic::ensure_store / compile_store / write_filter — public, silent, path-parameterized functions the overlay will call as its second caller"
provides:
  - "ff (GUI binary) and ff-cli (short-name console CLI) as separate [[bin]] targets over the shared ferrisfilter library; ferrisfilter (long name) unchanged"
  - "eframe/egui 0.36, global-hotkey 0.8, tray-icon 0.24, image 0.25, windows-sys 0.61 (cfg(windows)) as pinned, human-approved dependencies"
  - "src/overlay/{mod,app,layout}.rs — overlay::launch() opens an opaque, undecorated, always-on-top vertical stripe docked to the right screen edge, provably never screen-sized"
affects: [13-02, 13-03, 13-04, 13-05, 14-rule-editor]

# Tech tracking
tech-stack:
  added: [eframe 0.36.1, egui 0.36.1, global-hotkey 0.8.0, tray-icon 0.24.2, image 0.25.10 (png only), windows-sys 0.61.2 (Win32_Foundation, Win32_UI_WindowsAndMessaging, cfg(windows) only)]
  patterns:
    - "Two thin binaries over one library, third binary added: ff (GUI, Windows subsystem in release), ff-cli (console, short name), ferrisfilter (console, long name) all call into the same lib"
    - "Pure geometry function (layout::dock_rect) kept fully unit-testable, separate from the eframe/winit call site that consumes it — logic in the library/pure-fn layer, egui as thin view (PITFALLS.md §8)"
    - "First-frame-only side effect latched via a bool field (OverlayApp.docked), not re-derived every frame"

key-files:
  created:
    - src/bin/ff-cli.rs
    - src/overlay/mod.rs
    - src/overlay/app.rs
    - src/overlay/layout.rs
  modified:
    - Cargo.toml
    - src/lib.rs
    - src/bin/ff.rs
    - CLAUDE.md

key-decisions:
  - "eframe 0.36.1's App trait uses fn ui(&mut self, ui: &mut egui::Ui, frame: &mut Frame), not fn update(&mut self, ctx: &egui::Context, frame: &mut Frame) as the plan's code sketch assumed — corrected in place (Rule 1), confirmed against the installed crate source, not guessed"
  - "EDGE_MARGIN pinned at 12.0 px (Claude's discretion per plan) — clearance kept on the right/top/bottom edges and the floor under which dock_rect's strictly-less-than invariant is enforced"
  - "Rustc toolchain (nightly, dated June 2025 / rustc 1.90) was too old for eframe/egui 0.36.1's rustc 1.95 MSRV; updated via `rustup update nightly` to 1.100.0-nightly before the first cargo build could succeed — a blocking-issue auto-fix (Rule 3), not a dependency version change"

requirements-completed: [SHELL-01, SHELL-03]

# Metrics
duration: ~45min (Tasks 2-3, this continuation session; Task 1's checkpoint ran in a separate prior session)
completed: 2026-09-04
---

# Phase 13 Plan 01: Overlay Shell — Binary Split, GUI Dependencies, Stripe Viewport Summary

**`ff` is now the GUI overlay binary — `cargo build`'s default output opens an opaque, undecorated, always-on-top vertical stripe docked to the right screen edge, provably never screen-sized, while the console CLI keeps working unchanged under `ferrisfilter` and the new `ff-cli` short name.**

## Performance

- **Tasks:** 3 (1 checkpoint + 2 auto)
- **Files modified:** 8 (4 created, 4 modified) across Tasks 2-3
- **Commits:** 2 task commits (Task 1 was a verification-only checkpoint, no code change)

## Accomplishments

- Split the binary layout per D-01/D-02/D-03/D-04: `ff` retargeted to the GUI, `ff-cli` created to carry the old console shim body, `ferrisfilter` untouched, no binary or package named `overlay`.
- Added the six-crate GUI dependency set, approved at Task 1's human package-legitimacy checkpoint (audit table below), with `windows-sys` correctly scoped to `[target.'cfg(windows)'.dependencies]` so the library still compiles on non-Windows hosts.
- Built `src/overlay/{mod,app,layout}.rs`: a pure, unit-tested `dock_rect()` geometry function and an `eframe::App` that docks itself to the right screen edge on the first frame a monitor size is known, then latches so it never re-runs.
- Verified live on the actual 1920×1080 dev machine: the stripe opens at `Left=1448 Top=12 Right=1908 Bottom=1068` (460×1056), 12px clearance on the three undocked edges — both dimensions strictly under the screen size, matching the unit-tested invariant exactly.
- Zero fixture/snapshot drift: `artifact_guard` and `generate_snapshot` both green with `git status --porcelain tests/fixtures tests/snapshots` empty at every commit.

## Task 1: Package Legitimacy Gate (checkpoint — cleared by the author)

Task 1 was a `checkpoint:human-verify` (`gate="blocking-human"`) that paused before any `cargo add` landed, because no automated `slopcheck` tool was available at research time (RESEARCH Assumptions Log A4). The orchestrator verified all six crates against the live crates.io API and the author approved:

| Crate | Pinned | Current stable | Repository | Downloads | First published |
|---|---|---|---|---|---|
| `eframe` | 0.36.1 | 0.36.1 ✓ | github.com/emilk/egui ✓ | 17.6M | 2021-01-04 |
| `egui` | 0.36.1 | 0.36.1 ✓ | github.com/emilk/egui ✓ | 22.9M | 2020-05-30 |
| `global-hotkey` | 0.8.0 | 0.8.0 ✓ | github.com/tauri-apps/global-hotkey ✓ | 5.0M | 2023-01-16 |
| `tray-icon` | 0.24.2 | 0.24.2 ✓ | github.com/tauri-apps/tray-icon ✓ | 27.0M | 2022-12-01 |
| `image` | 0.25.x | 0.25.10 ✓ | github.com/image-rs/image ✓ | 181M | 2014-11-20 |
| `windows-sys` | 0.61.x | 0.61.2 ✓ | github.com/microsoft/windows-rs ✓ | 1.5B | 2021-10-18 |

Every repository owner matched the expected project; every pin was current stable; no typosquat or thin-download signature. This closed RESEARCH Assumptions Log item A4. No commits were made for Task 1 — it is a verification-only gate. Documented here per the continuation prompt's requirement to record the checkpoint outcome in this SUMMARY.

## Task Commits

1. **Task 1: Package legitimacy gate** — checkpoint, no commit (cleared by author in prior session)
2. **Task 2: Split the binaries and add the GUI dependency set** — `ae7a0de` (feat)
3. **Task 3: Configure the stripe viewport and the never-full-screen geometry** — `aa68569` (feat)

_No separate plan-metadata commit is included in this count; the final `docs(13-01): complete...` commit follows this SUMMARY._

## Files Created/Modified

- `Cargo.toml` — new `[[bin]] ff-cli`, six new dependencies (`eframe`, `egui`, `global-hotkey`, `tray-icon`, `image`, `windows-sys` under `cfg(windows)`)
- `src/lib.rs` — `pub mod overlay;`
- `src/bin/ff.rs` — rewritten as the GUI entry: `#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]` + `overlay::launch()`
- `src/bin/ff-cli.rs` — new; carries the console shim body `ff.rs` used to hold
- `src/overlay/mod.rs` — `launch()`: builds the `ViewportBuilder` (always-on-top, undecorated, 460×1000 initial size, non-resizable, no taskbar entry, not activated on reveal) and calls `eframe::run_native`
- `src/overlay/app.rs` — `OverlayApp`: `eframe::App::ui()` renders a placeholder `CentralPanel` label; docks itself once via `layout::dock_rect` on the first frame with a known monitor size
- `src/overlay/layout.rs` — pure `dock_rect(monitor, width) -> egui::Rect`, `STRIPE_WIDTH = 460.0`, `EDGE_MARGIN = 12.0`, plus 3 unit tests
- `CLAUDE.md` — repository-layout table documents `ff`, `ff-cli`, `ferrisfilter`

## Decisions Made

- **`eframe::App` trait signature correction.** The plan's code sketch specified `fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame)`, but eframe 0.36.1's actual `App` trait requires `fn ui(&mut self, ui: &mut egui::Ui, frame: &mut Frame)` (confirmed against the installed crate source at `eframe-0.36.1/src/epi.rs`). `CentralPanel::show` in this version also takes `&mut egui::Ui`, not `&egui::Context`. Fixed in place — Rule 1 (bug/incorrect API assumption), verified by a successful `cargo build`.
- **`EDGE_MARGIN = 12.0`** — Claude's discretion per the plan. Applied uniformly as clearance on the right/top/bottom edges and, more importantly, as the basis for `dock_rect`'s hard clamp guaranteeing the returned rect's width/height are unconditionally strictly less than the monitor's, even for a tiny monitor or an oversized requested width.
- **Toolchain update.** The installed `nightly` rustc (1.90.0-nightly, dated June 2025) predates eframe/egui 0.36.1's rustc 1.95 MSRV and failed to compile the dependency tree. Ran `rustup update nightly`, landing on `1.100.0-nightly` (2026-09-03), which builds cleanly. This is a local toolchain fix, not a dependency downgrade — the approved 0.36.1 pins are untouched.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Corrected `eframe::App` trait method from the plan's `update()` sketch to the real `ui()` signature**
- **Found during:** Task 2, first `cargo build` after creating `src/overlay/app.rs`
- **Issue:** `error[E0407]: method 'update' is not a member of trait 'eframe::App'` — eframe 0.36.1 uses `fn ui(&mut self, ui: &mut egui::Ui, frame: &mut Frame)`, and `CentralPanel::show` takes `&mut egui::Ui`, not `&egui::Context`
- **Fix:** Rewrote `OverlayApp::ui()` to match the real trait signature; `egui::CentralPanel::default().show(ui, |ui| ...)` passes the outer `ui` directly
- **Files modified:** `src/overlay/app.rs`
- **Verification:** `cargo build` succeeds; `cargo test` green (169 lib tests, up from 166)
- **Committed in:** `ae7a0de` (Task 2 commit)

**2. [Rule 3 - Blocking] Updated the local `nightly` Rust toolchain**
- **Found during:** Task 2, first `cargo build` after adding the six new dependencies
- **Issue:** `error: rustc 1.90.0-nightly is not supported by ... eframe@0.36.1 requires rustc 1.95` (and 7 other transitive crates with the same floor) — the installed nightly toolchain was dated June 2025, before those crates' MSRV existed
- **Fix:** `rustup update nightly` — landed on `1.100.0-nightly (a69a63265 2026-09-03)`, which satisfies every crate's MSRV
- **Files modified:** none (toolchain-only; no `rust-toolchain.toml` exists in this repo to pin a version)
- **Verification:** `cargo build` and `cargo test` both succeed post-update
- **Committed in:** n/a (no repo file changed; documented here for traceability)

**3. [Rule 1 - Bug] Rephrased a doc comment that literally contained the forbidden `with_transparent`/`with_mouse_passthrough`/`with_fullscreen` substrings**
- **Found during:** Task 3, self-verification of the acceptance criterion `grep -n 'with_transparent\|with_mouse_passthrough\|with_fullscreen' src/overlay/` must print nothing
- **Issue:** The explanatory comment above the `ViewportBuilder` construction named the omitted method calls verbatim (`with_transparent(true)`, etc.) to explain what was deliberately *not* called — which itself matched the grep the acceptance criterion runs
- **Fix:** Reworded the comment to describe the omitted behaviors (transparency, mouse click-through, exclusive fullscreen) without repeating the exact method-name substrings
- **Files modified:** `src/overlay/mod.rs`
- **Verification:** `grep -n 'with_transparent\|with_mouse_passthrough\|with_fullscreen' src/overlay/*` now prints nothing; `cargo build` still succeeds
- **Committed in:** `aa68569` (Task 3 commit)

---

**Total deviations:** 3 auto-fixed (1 bug — trait signature, 1 blocking — toolchain, 1 bug — self-defeating grep-matching comment)
**Impact on plan:** All three were necessary to reach a green build/test state; none changed the approved dependency pins, the window behavior, or the geometry invariants the plan specified. No scope creep.

## Issues Encountered

None beyond the three auto-fixes documented above.

## User Setup Required

None — no external service configuration required. (Note for the reader: the local Rust toolchain needed a `rustup update nightly` to build this plan's dependencies; anyone else building this repo on an older nightly will hit the same MSRV error and need the same update. No repo-level pin exists yet to catch this automatically — worth a `rust-toolchain.toml` in a future plan if this recurs.)

## Next Phase Readiness

- `ff` opens; `overlay::launch()`, `OverlayApp`, and `layout::dock_rect` are the concrete hooks plans 13-02 (hotkey summon/dismiss) and 13-03 (show-without-activate, tray icon) build on.
- The stripe currently shows only a placeholder label — the read-only rule list is plan 13-04's payload, per D-19/D-20.
- `cargo tree -d` will show `windows-sys` duplicated (0.59 via `global-hotkey`, up to 0.61 via `tray-icon`/our direct dep) — expected per RESEARCH Pitfall 4, not a regression to chase.
- No blockers for 13-02.
