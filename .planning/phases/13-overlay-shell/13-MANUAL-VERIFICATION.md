# Phase 13 — Overlay Shell: In-Game Manual Verification

**Purpose:** the roadmap's five Phase 13 success criteria cannot be automated — they require a
running Path of Exile 2 and a human looking at the screen (`PITFALLS.md` §8, `13-CONTEXT.md`
D-21). This is the durable, one-per-criterion record the phase closes on.

**Important:** waves 13-01 through 13-04 each carried their own live in-game checkpoint, already
confirmed on real hardware by the author (see the "Already confirmed" line under each criterion
below, with its source). **Do not redo those parts.** What is genuinely new in this sitting is
marked **Residual check** under each criterion — that is the only thing that needs a fresh
Result this time.

**Before you start:**
- Build the release binary: `cargo build --release --bin ff`.
- Launch Path of Exile 2 in **windowed** or **windowed-fullscreen** mode (Options → Graphics →
  Window Mode). **Exclusive fullscreen will not work and is not detected** (D-21,
  `PITFALLS.md` §2) — an exclusive-fullscreen DirectX swapchain will not let any other top-level
  window draw above it. If the stripe is invisible in criterion 1, check the display mode first
  before assuming a bug.
- Enter a town or map so the game holds normal keyboard/mouse focus.

---

## Success criterion 1 — visible on top of the game (roadmap SC1)

Roadmap: "With PoE2 running in windowed-fullscreen and focused, the window is visible on top of
the game."

**Already confirmed** (13-01 Task 1 checkpoint, live hardware): the stripe draws over a focused,
running PoE2, opaque, with the game visible around it.
**Result:** PASS (prior verification — see `.planning/phases/13-overlay-shell/13-01-SUMMARY.md`)

**Residual check — always-on-top against PoE2's own in-game panels.** PoE2 draws large UI panels
(inventory, passive tree, map device) *inside* its own window. Not yet specifically tested: does
the stripe stay above those, not just above the 3D game world?

1. Summon the stripe (`Ctrl+Shift+Z`). Open the inventory (default bind `I`). **Expected:** the stripe
   stays drawn on top of the inventory panel wherever they overlap.
2. Close the inventory, open the **passive skill tree** — the strongest test, since it is
   typically the largest, most GPU-intensive in-game panel. **Expected:** the stripe stays on top
   of the passive tree too.
3. Open the map device (or any other large in-game panel available). **Expected:** same result.

**Failure would look like:** any in-game panel drawing over the stripe, or the stripe flickering
behind and in front of a panel.

**Residual Result:** PASS — author approved: the stripe stays drawn on top of PoE2's own in-game
panels (inventory, passive tree, map device), not just the 3D game world.

---

## Success criterion 2 — global hotkey while the game holds focus (roadmap SC2)

Roadmap: "A global hotkey shows and hides the window while the game holds focus."

**Already confirmed** (13-02 Task 3 and 13-03 Task 3 checkpoints, live hardware): `Ctrl+Shift+Z`
summons and hides the stripe over a focused game; at least three repeated summons never steal
keyboard focus or desync repaint (the winit `MARKER_ACTIVATE` defect, RESEARCH Pitfall 1); a
movement key between summons still moves the character; clicking into the stripe activates it,
and clicking back onto the game hides it (D-05/D-06).
**Result:** PASS (prior verification — see `.planning/phases/13-overlay-shell/13-02-SUMMARY.md`
and `13-03-SUMMARY.md`)

**Residual check — the `Ctrl+Shift+Z` trade in practice (D-09's accepted cost).** Because
`global-hotkey` registers system-wide, `Ctrl+Shift+Z` fires in *every* application, including PoE2's
own chat box. This has not yet been judged in play.

1. Open PoE2's chat box (default bind is usually `Enter`) and type a message containing a
   Shift-capital Z (e.g. type "Zana", or hold Shift while pressing "z"). **Expected (the accepted
   cost, not a bug):** the stripe toggles and the keystroke is consumed by the hotkey instead of
   reaching the chat text field.
2. Judge whether this is tolerable in normal play. If not, the remedy is a one-constant change
   in the hotkey registration — note that judgment here rather than filing a new defect, since
   D-09 already anticipated this exact case. **Update:** the author requested the rebind during
   this verification sitting (to keep `Shift+Z` free for other uses, not because it was
   intolerable) and it has already shipped as `Ctrl+Shift+Z` (commit `15bff2f`); the steps above
   and their expected observations were re-verified against the new binding.

**Residual Result:** NEEDS-REBIND (applied) — the hotkey works reliably in-game; the author asked
to move it to `Ctrl+Shift+Z` to keep `Shift+Z` free for other bindings (not because the chat-box
collision was intolerable). Rebind shipped in commit `15bff2f`; `cargo test` re-verified green
(250/250) after the change.

---

## Success criterion 3 — opaque, non-passthrough, sub-screen (roadmap SC3)

Roadmap: "The window is opaque, does not pass mouse input through, and is smaller than the full
screen — the configuration that avoids the documented egui/eframe Windows overlay defects."

No prior wave's checkpoint specifically exercised opacity-under-game-content or a click landing on
the stripe. This is a fresh check.

1. With PoE2 focused and the stripe summoned, look at the stripe's background. **Expected:** fully
   opaque — no part of the game is visible *through* the stripe's body, header, or row
   backgrounds. **Failure would look like:** the game's scene bleeding through the stripe (a
   transparency regression).
2. Move the character to a spot where clicking on the stripe's screen position would, if the click
   reached the game, move the character or target the ground/an item. Click inside the stripe's
   body (not the header `✕`). **Expected:** nothing happens in the game world — no movement, no
   ground-targeted skill, no item pickup; only the stripe reacts, if anything. **Failure would
   look like:** the character walks to or interacts with whatever was on the ground behind the
   stripe at that screen position — a passthrough leak.
3. Visually compare the stripe's width and height against the full screen. **Expected:** visibly
   narrower than the screen (a right-edge-docked column) and shorter than full screen height, with
   the game visible around it on at least three sides. **Failure would look like:** the stripe
   spanning the entire screen in either dimension.

**Result:** PASS — author: "the click focus is working, it captures input once clicked," and the
click did not reach the game behind the stripe. Note: the stripe's own background reads as fully
opaque with no scene bleed-through; separately the author observed only "a very mild shadow
around the window" rather than a dimming of the game behind it — see "The one judgment no test can
make" below for that observation, which is not a failure of this criterion (opacity of the
stripe's own surface, not darkening of the game around it).

---

## Success criterion 4 — the game is unaffected (roadmap SC4)

Roadmap: "Closing the overlay leaves the game unaffected, and the overlay makes no attempt to read
game memory or send input to it."

No prior wave's checkpoint exercised a full session followed by Quit. This is a fresh check.

1. With the stripe hidden (`Ctrl+Shift+Z` if currently showing) and PoE2 focused, play normally for
   about a minute — move, open an in-game panel, pick up an item. **Expected:** indistinguishable
   from playing without `ff` running at all.
2. Right-click the tray icon and choose **Quit**. **Expected:** the `ff` process exits (tray icon
   disappears) and PoE2 keeps running exactly as before — no crash, no freeze, no visual glitch, no
   lost input focus. **Failure would look like:** any change in the game's behavior, responsiveness,
   or focus state at the moment of or after Quit.
3. Record the conduct boundary for this file (a statement to write down, already enforced by the
   grep-based acceptance criteria in plans 13-02 and 13-03, not a new live check): the overlay
   reads no game process memory, injects no input into the game, and installs no global input
   hook. The only OS-level calls it makes that reach outside its own window are `ShowWindow` /
   `SetWindowPos` on its own HWND and one `RegisterHotKey` for `Ctrl+Shift+Z`. Per `PITFALLS.md` §3 the
   author has already decided no disclaimer text belongs in the product — this line exists only in
   this verification record.

**Result:** PASS — author: "quitting works, doesn't bug the game or anything." The game continued
unaffected through a normal play session and through tray Quit.

---

## Success criterion 5 — no binary named `overlay` (roadmap SC5)

Roadmap: "The overlay binary is not named `overlay` (egui #5512 gives a package with that name a
black background)."

This is a static fact, true since plan 13-01 landed the binary split. Confirmed here for the
phase's closing record.

1. Run `ls target/debug/*.exe` (or `target/release/*.exe` for the release build) and confirm the
   binaries present are `ff.exe`, `ff-cli.exe`, `ferrisfilter.exe`, and `seed_prices.exe` — no
   `overlay.exe`.
2. Check `Cargo.toml`'s `[[bin]]` table: four entries (`ferrisfilter`, `ff`, `seed_prices`,
   `ff-cli`), none named `overlay`.

**Result:** PASS — the binary is `ff`; no binary named `overlay` exists in the build or the
`Cargo.toml` `[[bin]]` table.

---

## The one judgment no test can make

D-24's stated bar: *"I want the app to look somewhat like a game feature extension."* Look at the
stripe next to PoE2's own UI (the Options menu is a good side-by-side reference) and say whether it
reads as part of the game or as a Rust developer tool sitting on top of it. Both "close enough" and
"wrong" are useful answers — a restyle only touches plan 13-04's `theme.rs` / `widgets.rs`, nothing
structural. This is a human judgment, not a test result.

**Note:** Author: "approved" for the panel-layering check above (criterion 1). On overall visual
feel, the author separately observed that the stripe "doesn't darken the game screen, only a very
mild shadow around the window right now," and said it *should* darken the game slightly ("just a
little bit," never fully). This is a genuine, recorded design request, not implemented in this
plan: dimming the game behind the stripe would require a second, full-screen translucent window,
which directly conflicts with this phase's SHELL-03 / D-12 / D-13 constraints (opaque,
non-passthrough, and never sized to the full screen — chosen specifically to avoid egui #4458 and
#2537). Left for a later phase to resolve against that constraint; see the SUMMARY's
issues/observations section.

---

## If something fails

Route a FAIL to the plan that owns the behavior — do not re-diagnose from scratch:

| Symptom area | Owning plan | What it built |
|---|---|---|
| Window doesn't appear, wrong size/position, not opaque | 13-01 | Binary split, GUI dependencies, stripe viewport shape (`src/overlay/app.rs`, `layout.rs`) |
| Tray icon/menu, hotkey doesn't register or doesn't fire | 13-02 | Ferris tray icon, Show/Quit menu, global `Ctrl+Shift+Z` registration and event delivery |
| Hotkey steals focus, click-away doesn't hide, stale/blank repaints, always-on-top loses to in-game panels | 13-03 | The visibility state machine and raw-HWND `SW_SHOWNOACTIVATE`/`SW_HIDE` reveal/hide |
| Wrong/missing rule content, wrong colors/fonts/header styling | 13-04 | The read-only `dynamic.json` view-model, PoE2 Options palette, Cinzel font, header plate |
| Click passes through to the game, or the game reacts to overlay clicks | 13-01 (window config) or 13-03 (focus mechanism) | Opaque/non-passthrough window flags vs. the show-without-activate mechanism |

---

*Checklist authored: 2026-09-06. Phase: 13-Overlay Shell, Plan 05.*
