---
phase: 13-overlay-shell
phase_name: Overlay Shell
verified: 2026-09-06T00:00:00Z
status: passed
scores:
  success_criteria: 5/5
  requirements: 3/3
  plans_executed: 5/5
gaps: []
---

# Phase 13: Overlay Shell Verification Report

**Phase Goal:** A window renders above a running Path of Exile 2 and can be summoned and dismissed
by a global hotkey without alt-tabbing — the delivery surface, with no editing in it yet.
**Verified:** 2026-09-06
**Status:** passed
**Re-verification:** No — initial verification

## Goal Achievement

This phase holds up under an adversarial read. Every claim in the SUMMARYs that could be checked
against the actual codebase was checked directly — source files, `cargo test`, `git log`/`git
diff` across the full phase commit range — not taken on the summaries' word. Nothing found
contradicts what was reported. The one genuinely interesting finding (the winit
`MARKER_ACTIVATE` show-without-activate defect) is implemented exactly as the research prescribed,
the scope fence against Phase 14 held with zero leakage, and the three pure/library modules really
are pure. The out-of-band `--league`/`--tiers` work and the mid-phase hotkey rebind are both
clean, documented, and do not touch the acceptance bar.

### Observable Truths

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | With PoE2 running in windowed-fullscreen and focused, the window is visible on top of the game (roadmap SC1) | ✓ VERIFIED | `src/overlay/mod.rs` builds an `always_on_top`, undecorated, non-taskbar viewport; `13-MANUAL-VERIFICATION.md` SC1 residual check: author-approved PASS that the stripe also stays above PoE2's own in-game panels (inventory, passive tree, map device), not just the 3D world — a stronger test than the roadmap literally asked for |
| 2 | A global hotkey shows and hides the window while the game holds focus (roadmap SC2) | ✓ VERIFIED | `src/overlay/hotkey.rs` registers `Ctrl+Shift+Z` system-wide; `src/overlay/win.rs` drives both directions via raw `ShowWindow(SW_SHOWNOACTIVATE)`/`ShowWindow(SW_HIDE)` on a cached HWND, never `ViewportCommand::Visible` (grep confirms zero occurrences in `src/`); `13-MANUAL-VERIFICATION.md` SC2 + `13-03-SUMMARY.md` record repeated summons (not a single one) with focus never stolen — the exact case that would expose the `MARKER_ACTIVATE` defect if it were still present |
| 3 | The window is opaque, non-passthrough, and smaller than the full screen (roadmap SC3) | ✓ VERIFIED | `src/overlay/mod.rs`'s `ViewportBuilder` never calls `with_transparent`/`with_mouse_passthrough`/`with_fullscreen` (grep confirms); `src/overlay/layout.rs::dock_rect` has a hard-clamped, unit-tested invariant that width/height are strictly less than the monitor size; `13-MANUAL-VERIFICATION.md` SC3 author-verified click-into-stripe does not reach the game and the stripe reads visibly narrower/shorter than the screen |
| 4 | Closing the overlay leaves the game unaffected; no game-memory reads or input injection (roadmap SC4) | ✓ VERIFIED | Code review: the only OS-facing calls outside the crate's own window are `ShowWindow`/`SetWindowPos`-family calls on the overlay's own cached HWND (`win.rs`) and one `RegisterHotKey` (`hotkey.rs`) — no process enumeration, no `ReadProcessMemory`, no `SendInput`; `13-MANUAL-VERIFICATION.md` SC4 author-verified a full play session plus tray Quit left the game unaffected |
| 5 | The overlay binary is not named `overlay` (roadmap SC5) | ✓ VERIFIED | `Cargo.toml` `[[bin]]` table: `ferrisfilter`, `ff`, `seed_prices`, `ff-cli` — no `overlay` target; `13-MANUAL-VERIFICATION.md` SC5 confirms the built exe list matches |

**Score:** 5/5 truths verified

### Milestone Acceptance Bar

The v0.4 acceptance bar (`config.json → universal.filter` byte-identical, zero fixture
re-baselining) was re-checked across the **whole phase commit range**, not just the current
working tree:

```
git diff --stat ae7a0de~1..e1e0c36 -- tests/fixtures tests/snapshots   → empty
git log --oneline 9b675ea..e1e0c36 -- tests/fixtures tests/snapshots  → empty
```

(`ae7a0de` is Phase 13's first commit, `e1e0c36` its last; `9b675ea` is Phase 12's final commit —
the wider range was also checked and is equally clean.) `git status --porcelain tests/fixtures
tests/snapshots` is empty right now as well. This confirms the bar held for the entire phase, not
merely that no one left a dirty tree behind.

### The Scope Fence Against Phase 14

Phase 13 is contractually read-only against the dynamic store, and every EDIT-01..09 behavior
must be absent. Verified directly:

```
grep -rn "compile_store\|write_filter\|reset_store" src/overlay/   → no matches
grep -rn "TextEdit\|checkbox\|ComboBox\|DragValue\|drag_value" src/overlay/  → no matches
```

`src/overlay/store.rs` calls only `dynamic::ensure_store` (seed-only, proven never to overwrite an
existing store by its own unit test `load_never_overwrites_an_existing_store_even_when_it_fails_to_parse`)
and `loader::load_json`. The header's `✕` (a hand-painted crossed-line stroke pair in
`widgets.rs`, not an egui `Button`) is the sole interactive element in the stripe, and it only
sets `Visibility::Hidden` — it never touches `dynamic.json`. No add/delete/reorder/enable-disable/
style-picker/class-picker/rarity/defence/sockets/quality/corrupted control exists anywhere in
`src/overlay/`. The fence held with zero leakage.

### Layering (PITFALLS §8)

- **`src/overlay/visibility.rs`** — confirmed by reading: no `egui`/`eframe`/`winit`/`windows_sys`
  import or type appears anywhere in the file. Pure `enum Visibility` / `enum Effect` and two
  functions (`advance`, `effect_for`) over plain booleans. 12 `#[test]`s, including the
  load-bearing 20-consecutive-unfocused-frame guard against the never-focused hazard.
- **`src/overlay/view_model.rs`** — confirmed by reading: imports only
  `crate::models::config::{...}`, no GUI-framework type in any signature. `rule_rows(&Config) ->
  Vec<RuleRow>` is a pure mapping. 11 `#[test]`s, including one against the real
  `dynamic_base.json`.
- **`src/overlay/theme.rs`** — uses `egui::Color32`/`egui::Visuals`/`egui::text::LayoutJob` as
  plain data types (there is no way to describe an egui palette without egui's color type), but
  requires no live window, no `eframe::Frame`, and no event loop to construct or assert against —
  its 3 `#[test]`s run headlessly and pass. This is a materially different claim than
  "framework-free" in the same sense as `visibility.rs`/`view_model.rs` (which import zero GUI
  crate symbols at all), and the SUMMARY's own module doc is honest about that distinction
  ("no window is needed to test them," not "no egui types"). Not a gap — theme configuration
  necessarily speaks in the target framework's data vocabulary — but worth naming plainly rather
  than rubber-stamping the phrase "framework-free" across all three modules uncritically.
- **Win32 confinement** — `grep -rln "windows_sys" src/` returns only `src/overlay/win.rs`. Every
  `unsafe` block in the overlay lives in that one file's two public functions (`reveal`,
  `conceal`), each a single `ShowWindow` call, exactly as claimed.

### The Focus Mechanism (Research Finding Implementation)

Verified directly against `src/overlay/win.rs` and `src/overlay/app.rs`:

- `grep -rn "ViewportCommand::Visible" src/` → **zero matches**. The mechanism the RESEARCH
  document identified as necessary (bypass `ViewportCommand::Visible` because winit's
  `MARKER_ACTIVATE` flag latches true after the first reveal, silently activating on every later
  toggle) is exactly what shipped: `win::reveal`/`win::conceal` call raw
  `ShowWindow(SW_SHOWNOACTIVATE)`/`ShowWindow(SW_HIDE)` on a cached HWND, and `app.rs`'s frame loop
  never calls `send_viewport_cmd(ViewportCommand::Visible(...))` anywhere.
- The verification evidence explicitly covers **repeat summons, not just one**. Both
  `13-03-SUMMARY.md` ("Keyboard focus stays with Notepad across every summon (steps 3 and 4 — the
  load-bearing case; a single-summon test would have passed even with the winit `MARKER_ACTIVATE`
  defect present)") and `13-MANUAL-VERIFICATION.md` SC2 record multiple cycles. This is the
  correct test shape — a single-summon check would have passed even with the defect present, since
  `with_active(false)` alone is sufficient for the first reveal — and the author's checklist was
  written with that awareness rather than stopping at the easy case.
- The `ShownUnfocused` third state in `visibility.rs` (as opposed to a plain
  `Hidden`/`Shown` boolean) is exactly the mechanism that prevents a `SW_SHOWNOACTIVATE`-revealed
  window (which reads `focused == false` from the instant it appears) from reading that as
  "clicked away" and re-hiding itself on its very next frame — verified by the
  `never_focused_guard_survives_many_consecutive_unfocused_frames` test, which runs the guard for
  20 consecutive unfocused frames.

### Decision Coverage (D-01..D-25)

Spot-checked the shipped code against `13-CONTEXT.md`'s decisions rather than trusting the
summaries' claims of compliance:

| Decision | Verified in code |
|---|---|
| D-01 GUI is the default build | `Cargo.toml`: `default-run = "ff"`, `eframe`/`egui` plain (non-optional, non-feature-gated) dependencies |
| D-02/D-03/D-04 `ff`/`ff-cli`/`ferrisfilter` split, Windows GUI subsystem, no `overlay` binary | `src/bin/ff.rs` (`#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]`), `src/bin/ff-cli.rs`, `Cargo.toml` `[[bin]]` table |
| D-05/D-06/D-07 show-without-activate, click activates normally, no global mouse hook | `win.rs` (`SW_SHOWNOACTIVATE` scoped to the reveal call only, no `WS_EX_NOACTIVATE` set); no mouse-hook crate or code anywhere in `src/overlay/` |
| D-09 (SUPERSEDED) `Shift+Z` → `Ctrl+Shift+Z` | `hotkey.rs`'s `TOGGLE_MODIFIERS = Modifiers::CONTROL.union(Modifiers::SHIFT)`; `13-CONTEXT.md`'s D-09 entry is explicitly annotated `SUPERSEDED 2026-09-06 → Ctrl+Shift+Z` with the rationale recorded, not silently overwritten |
| D-10 registration failure surfaced | `HotkeyStatus::Failed(String)` propagated into the tray tooltip and a disabled menu line (`mod.rs`, `tray.rs`) |
| D-12/D-13 never full-screen, opaque/non-passthrough/always-on-top | `layout::dock_rect`'s clamp invariant + unit tests; `mod.rs`'s `ViewportBuilder` omits transparency/passthrough/fullscreen calls |
| D-14 borderless + custom header, `✕` hides only, tray Quit is sole exit | `widgets::header_plate` (painted, not OS chrome); `app.rs`: `close_clicked` sets `Visibility::Hidden` only; `OverlayCommand::Quit` is the only `ViewportCommand::Close` call site (verified: it appears exactly once, in `app.rs`'s tray-menu match arm) |
| D-16/D-17/D-18 tray icon, Show/Quit menu, Ferris icon | `tray.rs` (`build`, `command_for_menu`), `assets/ferris.png` + `assets/ASSETS.md` provenance |
| D-19/D-20 read-only ordinal list, no status chrome, all mutation deferred | `view_model::rule_rows` (ordinal = array index, never filtered), `app.rs` renders only ordinal/style/summary — no store path, price source, or destination folder anywhere in the rendered UI |
| D-21/D-22 no process detection, no memory/input access | Documented in `CLAUDE.md`, not implemented; code review confirms no such calls exist |
| D-24/D-25 PoE2 palette in chrome, Dracula stays in filter output | `theme.rs` constants match the D-24 hex table exactly (asserted by its own unit tests); `config.json`'s Dracula `styles` block is untouched (fixture guard confirms byte-identical output) |

D-09's supersession is the one decision that changed mid-phase, and it is handled exactly as the
adversarial brief asked to check: the CONTEXT.md entry is edited in place with a `SUPERSEDED`
marker and a dated rationale, not silently replaced, and the code (`hotkey.rs`'s doc comment and
constant) matches what's recorded.

### Required Artifacts

| Artifact | Expected | Status | Details |
|---|---|---|---|
| `src/overlay/app.rs` | eframe App, frame loop wiring, no show/hide branching of its own | ✓ VERIFIED | 272 lines; delegates to `visibility`/`win`/`store`/`view_model`/`widgets`; contains only "advance, ask for effect, perform it" |
| `src/overlay/visibility.rs` | Pure 3-state machine | ✓ VERIFIED | Framework-free, 12 tests, all passing |
| `src/overlay/win.rs` | Sole `windows_sys` call site | ✓ VERIFIED | 99 lines, `cfg(windows)`/`cfg(not(windows))` split, two public fns |
| `src/overlay/hotkey.rs` | Global hotkey registration, status as a value | ✓ VERIFIED | `Ctrl+Shift+Z`, `HotkeyStatus` enum, no `unwrap`/`expect` on external state |
| `src/overlay/tray.rs` | Tray icon + Show/Quit menu | ✓ VERIFIED | `build`, `command_for_menu` (pure, unit-tested) |
| `src/overlay/layout.rs` | Pure docking geometry | ✓ VERIFIED | `dock_rect` invariant-tested against 1920×1080 and a degenerate small monitor |
| `src/overlay/theme.rs` | D-24 palette + Cinzel registration | ✓ VERIFIED | 13 named constants, each doc-quoted and hex-asserted |
| `src/overlay/widgets.rs` | Header plate + zebra row painting | ✓ VERIFIED | 90 lines, exactly the two functions claimed |
| `src/overlay/view_model.rs` | Pure rule-row mapping | ✓ VERIFIED | Ordinal-indexed, never filters disabled rules out |
| `src/overlay/store.rs` | Read-only seed-then-load path | ✓ VERIFIED | Calls only `ensure_store` + `load_json`; 5 tests including a never-overwrites-on-failure guarantee |
| `assets/ferris.png`, `assets/Cinzel-Regular.ttf`, `assets/ASSETS.md` | Licensed binary assets with provenance | ✓ VERIFIED | Present; `ASSETS.md` records license/source for both |
| `.planning/phases/13-overlay-shell/13-MANUAL-VERIFICATION.md` | Committed, durable, per-criterion record | ✓ VERIFIED | All 5 roadmap SCs recorded PASS with author quotes, not a chat transcript |
| `Cargo.toml` `[[bin]]` table | `ferrisfilter`, `ff`, `ff-cli`, `seed_prices`; no `overlay` | ✓ VERIFIED | Confirmed |
| `CLAUDE.md` | Documents `ff`/`ff-cli`/`ferrisfilter`, windowed-mode requirement, hotkey | ✓ VERIFIED | Repository-layout table and a dedicated callout block, both accurate as of `Ctrl+Shift+Z` |

### Key Link Verification

| From | To | Via | Status | Details |
|---|---|---|---|---|
| Global hotkey event | `OverlayApp` toggle | `GlobalHotKeyEvent::receiver().try_recv()` filtered to `HotKeyState::Pressed` and the registered id | ✓ WIRED | `app.rs` lines 133-139; live-verified exactly one toggle per keypress (no double-fire) |
| Tray click/menu event | `OverlayApp` toggle/Show/Quit | `TrayIconEvent::receiver()` / `MenuEvent::receiver()` polled per frame | ✓ WIRED | `app.rs` lines 143-177 |
| `visibility::advance`/`effect_for` | `win::reveal`/`win::conceal` | one call pair per frame in `app.rs`'s `ui()` | ✓ WIRED | No mixing with `ViewportCommand::Visible` (confirmed absent) |
| `dynamic.json` (via `dynamic::ensure_store` + `loader::load_json`) | rendered rule list | `store::load` → `view_model::rule_rows` → `app.rs`'s `ScrollArea` | ✓ WIRED, DATA FLOWS | `OverlayApp::new()` calls `store::load(&base, &store_path)` and maps the result through `view_model::rule_rows`; this is real data from the actual `dynamic.json`/`dynamic_base.json` on disk, not a static stub — confirmed by the `rule_rows_on_the_real_dynamic_base_json_has_dense_ordinals_and_nonempty_styles` test and by the author's own screenshot-verified manual check in 13-04 |
| Header `✕` click | `Visibility::Hidden` | `header.close_clicked` branch in `app.rs` | ✓ WIRED | Never calls `ViewportCommand::Close`; that call site is reserved for tray Quit only |

### Data-Flow Trace (Level 4)

| Artifact | Data Variable | Source | Produces Real Data | Status |
|---|---|---|---|---|
| `app.rs`'s rule list `ScrollArea` | `self.rows: Result<Vec<RuleRow>, String>` | `store::load(&base, &store_path)` → `dynamic::ensure_store` (Phase 12) → `loader::load_json` → `view_model::rule_rows` | Yes — reads the real `dynamic.json`/`dynamic_base.json` on disk, seeding from the template only if absent, never overwriting an existing store | ✓ FLOWING |

### Behavioral Spot-Checks

| Behavior | Command | Result | Status |
|---|---|---|---|
| Full suite passes, count matches the orchestrator's claimed 250 | `cargo test --quiet` | 212 lib + 38 integration/doctest = 250 passed, 0 failed | ✓ PASS |
| `overlay::` module tests pass in isolation | `cargo test --lib overlay::` | 37 passed, 0 failed | ✓ PASS |
| No `ViewportCommand::Visible` anywhere in the crate | `grep -rn "ViewportCommand::Visible" src/` | no matches | ✓ PASS |
| `windows_sys` confined to `win.rs` | `grep -rln "windows_sys" src/` | `src/overlay/win.rs` only | ✓ PASS |
| No store-mutation calls or editing widgets in the overlay | `grep -rn "compile_store\|write_filter\|reset_store" src/overlay/` and a widget-name grep | no matches | ✓ PASS |
| Zero fixture/snapshot drift across the full phase commit range | `git diff --stat ae7a0de~1..e1e0c36 -- tests/fixtures tests/snapshots` | empty | ✓ PASS |
| No unresolved debt markers in phase-modified files | `grep -rn -iE "TBD\|FIXME\|XXX\|TODO\|HACK\|PLACEHOLDER" src/overlay/ ...` | one match, a doc comment describing the intended behavior ("never a placeholder like `none` or `-`"), not a debt marker | ✓ PASS |

### Requirements Coverage

| Requirement | Source Plan | Description | Status | Evidence |
|---|---|---|---|---|
| SHELL-01 | 13-01, 13-04 | Always-on-top window renders above PoE2 in windowed/windowed-fullscreen mode | ✓ SATISFIED | Viewport config + author-verified in-game (SC1, including layering above PoE2's own panels) |
| SHELL-02 | 13-02, 13-03 | Global hotkey toggles visibility while the game holds focus | ✓ SATISFIED | `hotkey.rs` + `win.rs`/`visibility.rs` mechanism, author-verified across repeated summons |
| SHELL-03 | 13-01 | Opaque, non-passthrough, never full-screen | ✓ SATISFIED | `layout.rs` invariant + `mod.rs` viewport flags, author-verified (SC3) |

No orphaned requirements — REQUIREMENTS.md's traceability table lists exactly SHELL-01/02/03 for
Phase 13, all three claimed and all three satisfied.

### Anti-Patterns Found

None that constitute a blocker or warning. The single grep hit for "placeholder" in
`view_model.rs` is prose in a doc comment explaining that absent conditions never render a literal
placeholder string — the opposite of a stub. No `TBD`/`FIXME`/`XXX`/`HACK` markers, no
`return null`/empty-body handlers, no hardcoded-empty props feeding a render path.

### Out-of-Band Work (assessed, not scope violations)

- `31fde9b` (`sync --league`) and `732de39` (`generate`/`apply --tiers`) — both cleanly scoped,
  each with a clear commit message explaining the real-world trigger (a live league transition),
  and neither touches `src/overlay/` or the fixture guard. `--tiers` is explicitly implemented as
  an override flag rather than an edit to the committed `tiers.json`, specifically to avoid
  re-baselining `tests/cli_generate.rs`'s oracle — the right call given the acceptance bar.
- `15bff2f` (hotkey rebind) — `D-09` in `13-CONTEXT.md` is annotated `SUPERSEDED` with a dated
  rationale; the checklist (`13-MANUAL-VERIFICATION.md`) was corrected to reference the new
  binding rather than left describing a binding that no longer shipped; `cargo test` was
  re-verified green (250/250) after the change, which this verification independently confirmed.
- `e1e0c36` (game-dimming marked DECLINED) — correctly closed rather than left as an open thread;
  the author's own words ("we can drop the shadow for now, it is not really necessary") are
  recorded in `13-05-SUMMARY.md`'s Next Phase Readiness section, and Phase 14 is told explicitly
  it needs no second window for this.

### Human Verification Required

None. All five roadmap success criteria are manual-by-design (PITFALLS §8, D-21) and the author
has already performed them live against a running Path of Exile 2, recorded verbatim with
per-criterion `Result:` lines and direct quotes in `13-MANUAL-VERIFICATION.md`. Per the phase
facts provided for this verification, that record is treated as the evidence, not as a gap to
re-flag.

### Gaps Summary

No gaps. Every roadmap success criterion, every SHELL requirement, the milestone acceptance bar,
the scope fence against Phase 14, the layering discipline, and the specific research-driven focus
mechanism (raw `ShowWindow` bypassing `ViewportCommand::Visible`) all hold up under direct
inspection of the code, the test suite, and the git history — not just the summaries' narrative.
The one nuance worth flagging for the record (not a gap): `theme.rs` is "framework-free" in the
weaker sense of "testable without a window," not in the stronger sense `visibility.rs` and
`view_model.rs` achieve of "imports zero GUI-framework symbols" — this is inherent to configuring
an `egui::Visuals` and does not compromise `PITFALLS.md` §8's actual intent (no bug here can only
be found by clicking; all three modules' behavior is exercised by `#[test]`s that run headlessly).

## What Went Well / Was Simpler or Harder Than Planned

- **The `MARKER_ACTIVATE` research finding was correctly translated into shipped code**, and the
  verification evidence was written with the defect's shape in mind — testing repeated summons
  specifically because a single-summon check would not have caught it. This is a case where the
  research phase's depth (reading winit's actual source, not just its docs) paid off directly in
  the implementation and in how the manual checklist was designed.
- **The read-only/mutation boundary held cleanly** in both directions: `store.rs` calls only the
  seed-and-load half of Phase 12's lifecycle, and its own test suite proves the never-overwrite
  guarantee rather than asserting it in prose only.
- **Two real defects were found and fixed within-phase rather than deferred**: the U+2715 tofu-box
  close glyph (13-03 → fixed in 13-04 via hand-painted strokes) and the toolchain MSRV mismatch
  (13-01, `rustup update nightly`). Both are documented as deviations with root cause and fix
  location, not silently patched over.
- **One design ask (game-dimming) was correctly recognized as architecturally in tension with this
  phase's own defect-avoidance constraints** (D-12/D-13) rather than force-fit in, and was properly
  closed by the author rather than left as an ambiguous "todo."
- **The single-instance-guard gap** (a second `ff` launch opens a second stripe) was discovered
  during the phase, correctly judged out of scope for any SHELL requirement, and captured as a
  concrete backlog item (999.2) with a specific fix shape — not silently dropped.

## Deviations Carried Forward

- Toggle hotkey is `Ctrl+Shift+Z`, not the `Shift+Z` named in the original D-09 and in
  `ROADMAP.md`'s plan-list summary text — CLAUDE.md, the manual verification checklist, and the
  code all agree on `Ctrl+Shift+Z`. Phase 14 should assume and document `Ctrl+Shift+Z` going
  forward, as `13-05-SUMMARY.md` already states.
- No single-instance guard exists; launching `ff` twice opens two stripes and only the first
  instance's hotkey registers (the second's failure is surfaced correctly per D-10). Tracked as
  backlog Phase 999.2, not a Phase 13 or Phase 14 requirement.
- Commit `c6f9c03` is labeled `feat(13-02):` but is actually 13-03's Task 2 work — a cosmetic
  bookkeeping mislabel documented in `13-03-SUMMARY.md`'s Deviations section. No functional impact;
  noted here so future commit-based tooling isn't surprised by it.

## Lessons

- Writing the manual-verification checklist with explicit awareness of *why* a defect would be
  missed by a naive test (the single-summon vs. repeated-summon distinction) is a stronger pattern
  than a generic "click the thing and see" checklist — it turns a research finding into a concrete,
  falsifiable check rather than trusting the fix in the abstract.
- The "framework-free" claim needs a sharper definition when a module's entire purpose is
  configuring the framework's own data types (as with `theme.rs`'s `egui::Visuals`/`Color32`
  usage). The distinction that matters for `PITFALLS.md` §8 is "testable without a live
  window/event loop," not "imports zero symbols from the GUI crate" — worth stating that
  distinction explicitly in future phases that build theming modules, so a reviewer doesn't have
  to infer it from context.

---

*Verified: 2026-09-06*
*Verifier: Claude (gsd-verifier)*
