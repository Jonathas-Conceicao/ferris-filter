# Phase 13: Overlay Shell - Context

**Gathered:** 2026-09-04
**Status:** Ready for planning

<domain>
## Phase Boundary

A vertical stripe window renders above a running Path of Exile 2, is summoned and dismissed by a
global hotkey without alt-tabbing, and lives in the system tray between uses. It displays the
dynamic rules **read-only** — proving the Phase 11/12 wiring reaches the GUI — and contains no
control that mutates anything.

**In scope:** window creation and always-on-top behavior over the game; global hotkey registration
and toggle; show-without-stealing-focus; click-to-activate then hide-on-focus-loss; system tray
icon and menu; binary/dependency restructuring so the GUI is the default build; a read-only list of
the rules in `dynamic.json`.

**Out of scope (Phase 14):** every editing control — add, delete, reorder, enable/disable, style
selector, class picker, rarity, defences, sockets, quality, corrupted — plus the Reset and Apply
buttons. The author asked for these during discussion; they were held at the phase boundary so that
a Phase 13 failure means *the stripe is broken*, not *the editor is broken*.

</domain>

<decisions>
## Implementation Decisions

### Binary layout and build shape

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

### Summon and dismiss

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

### Window shape

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

### System tray

- **D-16: A system tray icon shows the overlay is running.** The process lives in the tray; the
  stripe is a view it shows and hides. Requested by the author so there is always a visible sign
  the tool is alive even when the stripe is hidden.
- **D-17: Right-click menu with Show and Quit; left-click toggles the stripe** the same way the
  hotkey does. Quit is the only exit (see D-14).
- **D-18: The icon is Ferris (the Rust mascot)** — "anything for now". It belongs in `assets/`,
  alongside the existing `uwu.mp3`.

### Phase 13 payload

- **D-19: The stripe shows the rules from `dynamic.json` in ordinal order, read-only.** No status
  chrome — the author explicitly ruled out store path, price source, destination folder and any
  pricing information. The rule list is the content; everything else is noise.
- **D-20: All mutation, plus Reset and Apply, stays in Phase 14.** The editing controls land in the
  space the read-only list already occupies.

### Game awareness and conduct

- **D-21: No PoE2 process or window detection this phase.** The windowed / windowed-fullscreen
  requirement is documented for the user, not detected. Detection is a nice-to-have that would pull
  in window enumeration for a failure the author will hit once and then know.
- **D-22: No memory reads, no input injection, no global input hooks.** Reaffirmed — the tool
  writes a text file and nothing else. Per `PITFALLS.md` §3 the author has already closed the ToS
  question: no disclaimers in the product.

### Visual design (author-supplied references, 2026-09-04)

The author initially left style to Claude's discretion, then supplied two screenshots that
supersede that. Both are committed under `reference/` in this phase directory.

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
  `HWND`) — a research question, not a design one.
- Tray-icon crate selection.

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Milestone research (written 2026-09-03, directly about this phase)
- `.planning/research/PITFALLS.md` — **the most important ref for this phase.** The six egui/eframe
  Windows overlay defects and which configuration dodges them; the windowed-mode requirement; the
  ToS question already closed by the author.
- `.planning/research/STACK.md` — `eframe`/`egui` 0.36.1 and `global-hotkey` 0.8.0 with rationale,
  plus what was considered and rejected (Electron, `egui_overlay`, raw Win32, Tauri) so those are
  not re-litigated. Note the pin discipline: `egui` and `eframe` must share an exact minor.
- `.planning/research/ARCHITECTURE.md` — the two-binaries-over-one-library shape.
  **Superseded in part by D-01:** its workspace-split proposal assumed the CLI stayed primary.

### Phase definition
- `.planning/ROADMAP.md` § "Phase 13: Overlay Shell" — goal and the five success criteria.
- `.planning/REQUIREMENTS.md` § "Overlay Shell (SHELL)" — SHELL-01, SHELL-02, SHELL-03.
- `.planning/ROADMAP.md` § "Phase 14: Rule Editor + Apply" — read to know what this phase must
  *not* build, and what the read-only list has to grow into.

### What the GUI sits on top of
- `.planning/phases/12-dynamic-rule-store/12-VERIFICATION.md` § "Integration Check" — records that
  `dynamic::ensure_store`, `compile_store` and `write_filter` are public, print nothing, and take
  every path as a parameter, specifically so the overlay can be their second caller.
- `src/dynamic.rs` — the store lifecycle the stripe reads from.
- `src/compile.rs` — `load_inputs` / `compile_as`; the load path the read-only list needs.
- `src/models/config.rs` — `Rule` / `When`; every condition field is already `Option<…>`.
- `CLAUDE.md` § "Tier & color system (Dracula palette)" — the palette the *filter output* uses
  (**not** the GUI chrome — see D-25), and the game-folder path.

### Visual references (author-supplied 2026-09-04, committed in this phase directory)
- `.planning/phases/13-overlay-shell/reference/ee2-panel-layout.png` — Exiled Exchange 2's
  price-check panel. **The layout reference:** stripe proportions, header bar, boxed sections,
  zebra rows. Its colors are explicitly not adopted (D-23).
- `.planning/phases/13-overlay-shell/reference/poe2-options-palette.png` — PoE2's own Options
  menu. **The palette and styling reference** (D-24). Also shows the exact screen the player must
  visit after Apply — Options → Game → Guidance → **Item Filter**, a dropdown currently reading
  `universal`, with refresh and folder buttons beside it. Phase 14's post-Apply instruction can
  name that path precisely, and the player will be switching that dropdown to `dynamic`.

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- **`src/dynamic.rs`** — `ensure_store`, `reset_store`, `compile_store`, `write_filter`,
  `default_destination`. Built in Phase 12 as callable functions that print nothing precisely so a
  GUI could be their caller. Phase 13 needs only the load path; Phase 14 uses the rest.
- **`src/lib.rs`** — a real `[lib]` target already exists and exports every module. The GUI binary
  is a consumer of the library exactly as `ff`/`ferrisfilter` are.
- **`assets/uwu.mp3`** — establishes `assets/` as the home for binary assets; the Ferris icon
  follows the same convention.
- **`config.json` `styles` + class lists** — pinned this milestone, and the source Phase 14's
  pickers must read from (never hardcode class names — `PITFALLS.md` §10).

### Established Patterns
- **Logic in the library, presentation at the edge.** `run.rs` presents; the library returns values
  (`PriceSource`, `StoreState`). The egui layer must follow the same rule — `PITFALLS.md` §8: if a
  bug can only be found by clicking, it is in the wrong layer.
- **Two thin binaries over one library** (`src/main.rs`, `src/bin/ff.rs` are 4 lines each). The GUI
  binary should be similarly thin over a GUI module in the library.
- **Byte-for-byte oracle.** `artifact_guard` + `generate_snapshot` must stay green with zero
  re-baselining. Adding a GUI must not touch them.

### Integration Points
- `Cargo.toml` — new `[[bin]] ff-cli`, `ff` retargeted to the GUI source, new dependencies.
  `default-run = "ff"` is already correct.
- `src/bin/ff.rs` — currently the 4-line console shim; becomes the GUI entry (its console body
  moves to `src/bin/ff-cli.rs`).
- `CLAUDE.md` — the repository-layout table needs the new binaries and the windowed-mode
  requirement.

</code_context>

<specifics>
## Specific Ideas

- **"Like a stripe"** — the author's own word for the window shape: takes over vertical space, uses
  only the horizontal space it requires, and any filter-configuration functionality gets built
  inside it. This is the single most shape-defining statement in the discussion.
- **Exiled Exchange 2** was cited during milestone scoping as the reference overlay for how such a
  tool should feel — while noting that its transparent click-through shape is *not* ours. The
  author later supplied a screenshot of it (D-23) to pin the layout concretely.
- **"I want the app to look somewhat like a game feature extension"** — the sentence behind D-24.
  The overlay should not look like a Rust developer tool that happens to sit over PoE2; it should
  look like a screen the game shipped with. That is the bar for the visual work.
- The player's post-Apply destination is visible in the reference screenshot: Options → Game →
  Guidance → Item Filter, currently set to `universal`.

</specifics>

<deferred>
## Deferred Ideas

- **Click-outside-dismiss as its own mechanism** — cut in favor of hide-on-focus-loss (D-07).
  Revisit only if D-06 feels wrong in play; it would need a low-level global mouse hook, which
  D-22 currently rules out.
- **Configurable hotkey** — hardcoded `Shift+Z` this phase (D-09). Becomes worth doing at the same
  time as any settings file.
- **Position / width persistence** — needs a settings file that does not exist yet (D-15).
- **PoE2 process and exclusive-fullscreen detection** — documented, not detected (D-21).
- **Editing, Reset and Apply in the stripe** — requested during this discussion, held at the phase
  boundary. This is Phase 14 (EDIT-01..09), and the read-only list is deliberately the shape it
  grows into.

### Carried from earlier phases (not this phase's work)
- `Cargo.toml` still reads `version = "0.1.0"` while git tags read v0.3.
- Untracked `ssf_*` presets and scratch files await a tracked-vs-ignored decision.

</deferred>

<open_questions_for_research>
## For the Researcher

Design is settled; these are mechanism questions the planner needs answered:

1. **Show without activating.** Can `eframe`/`egui` 0.36 make a viewport visible without giving it
   focus, or does this require the raw `HWND` (`SW_SHOWNOACTIVATE` / `SWP_NOACTIVATE`, or
   `WS_EX_NOACTIVATE`) via the `windows` crate? D-05 depends on the answer.
2. **Hide on focus loss.** Which egui/winit event reports it, and does it fire correctly for an
   always-on-top borderless window that was shown without activation?
3. **`global-hotkey` 0.8 alongside eframe's winit event loop** on Windows — same thread, or its own?
4. **Tray icon crate** — `tray-icon` is by the same authors as `global-hotkey`; confirm the version
   and whether it needs its own event loop or shares winit's.
5. **Windows GUI subsystem** — `#![windows_subsystem = "windows"]` on the binary, and whether it
   interacts with `cargo test` / `assert_cmd` for the other binaries.
6. Confirm `eframe`/`egui`/`global-hotkey` versions are still current, and pin exact minors.
7. **Font.** D-24 calls for small-caps serif. egui ships sans-serif defaults (Ubuntu/Hack), so this
   needs a custom font registered via `egui::FontDefinitions` — find a freely-redistributable
   serif close to PoE's (Fontin/Cinzel family shape) whose license permits bundling in
   `assets/`, and confirm whether egui can synthesize small-caps or whether the glyphs must come
   from the face itself. If no good option exists, an all-caps fallback at reduced letter-spacing
   is acceptable and closer to the game than a default sans.
8. **Theming reach.** How much of the PoE2 look is achievable through `egui::Style`/`Visuals`
   alone (grounds, strokes, rounding, text colors) versus needing custom painting — particularly
   the circular toggles and the header plate.

</open_questions_for_research>

---

*Phase: 13-Overlay Shell*
*Context gathered: 2026-09-04*
