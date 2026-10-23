# Phase 47: Customizable Hotkey - Research

**Researched:** 2026-10-23
**Domain:** Rust egui/eframe 0.36 overlay: global hotkey re-registration (global-hotkey 0.8, Win32 `RegisterHotKey`), egui key capture, settings persistence, all-or-none Apply integration, skin art
**Confidence:** HIGH (every API claim checked against the crate sources in `~/.cargo/registry` or the codebase; the keybind art was dumped and inspected from the game install)

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

#### Layout (author)
- A new **Overlay** section on the CONFIG tab, under Filter, with one row: **Show / hide overlay**.
- Styled after the game's Options → Input keybind rows (reference
  `.planning/research/ui-refs/options-input-tab.png`, crop `options-keybind-rows.png`): the action
  name on the left in SmallCaps, and on the right a **wide button** holding the chord (dark plate,
  ornate gold corners, faint gold glow along the bottom, chord centred in white). Extract the
  keybind-button art from the Options menu with `ff-art`; fall back to the existing button art if it
  cannot be found (report which, for the author's review).
- Beside it the round game-art reset button (the refresh-style icon button) with tooltip
  `Reset to Ctrl+Shift+Z`.
- Compensate the art's transparent margins when positioning (see Phase 48 / ART-04).

#### Capture (author)
- Clicking the button makes it read `Press a key…`; held modifiers show live (`Ctrl+Shift+…`);
  Escape or a click elsewhere cancels.
- At least one modifier is required, except bare F1-F24. Reject modifier-only chords and the Windows
  key.
- While capturing, the live hotkey must not fire (pause / ignore it).

#### Saving (author — overrides the "save at once" proposal)
- A captured chord is a **pending CONFIG edit**: CONFIG tab ` *`, the unapplied indicator, Revert
  discards it, and the Quit popup counts it. **Apply** saves it to `ff-settings.json` and re-registers
  it at once (no restart).
- If registration fails at Apply (another app owns the chord), the old binding stays registered, the
  status line says why, and the new chord is not saved (decide how this fits the all-or-none Apply:
  validate / try registration before writing, so a refused chord refuses or skips cleanly).
- Reset restores `Ctrl+Shift+Z` (also a pending edit, applied on Apply).
- An invalid or unusable saved chord at start-up falls back to `Ctrl+Shift+Z` with a warning.

#### Tray and other mentions (author)
- The tray icon's mouse-over tooltip must show the **live (applied)** chord, or stop naming the chord
  at all — never a stale `Ctrl+Shift+Z`. Same for any other UI text or messages naming the hotkey.
- Keep the foreground-only rule (acts only while the game or the overlay is the foreground window).
- Debug builds keep their separate instance name.

### Claude's Discretion
- Key mapping (egui `Key` → global-hotkey `Code`, a hand-written table with round-trip tests),
  the `ff-settings.json` key name, registration ordering (register new, then unregister old).

### Deferred Ideas (OUT OF SCOPE)
None — discussion stayed within phase scope.

### Approved UI contract
`47-UI-SPEC.md` (status approved 2026-10-23) is binding for layout, copy, states, settings key
(`hotkey`), screenshot flags (`--hotkey <chord>`, `--capture-hotkey`) and the Apply ordering
(validate → register → write; refused registration refuses the whole Apply). This research does not
re-open it; it supplies the integration facts and flags two places where the spec's mechanics need a
correction (the border-art `clear`, see Pitfall 1; Copy/Cut/Paste events, see Pitfall 4).
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| KEY-01 | CONFIG **Show / hide overlay** row in the game's keybind style; clicking captures the next chord | Art found and inspected (§Keybind art); `ff_row` + new `ff_keybind_button` (§Pattern 6); capture state machine and event swallowing (§Pattern 3, §Pattern 4); egui key facts (§egui key input facts) |
| KEY-02 | Pending CONFIG edit (tab ` *`, indicator, Revert, Quit popup); Apply saves to `ff-settings.json` and re-registers without restart; foreground rule kept | `ConfigState::hotkey_pending` folds into every existing dirty path (§Pattern 2); settings file as a 4th all-or-none target in `apply_all` (§Pattern 5); registration on the UI thread inside `ui()` (§Thread affinity); poll unchanged except the live id (§Pattern 1) |
| KEY-03 | Modifier required (bare F1-F24 allowed); chord owned by another app refused, old binding kept, status says why | Chord validation in pure `chord.rs` (§Pattern 3); register-new-before-unregister-old with `AlreadyRegistered` mapping (§global-hotkey facts); refusal between `prepare` and `commit` writes nothing (§Pattern 5) |
| KEY-04 | Reset restores Ctrl+Shift+Z; invalid saved chord falls back with warning; tray tooltip / other mentions show the live chord | `hotkey::startup` fallback chain (§Pattern 1); `TrayHandles::set_hotkey_state` via `TrayIcon::set_tooltip` + kept `Menu` clone (§Tray); mention inventory (§Runtime State Inventory) |
</phase_requirements>

## Project Constraints (from CLAUDE.md)

- **No new dependencies** (CONTEXT / orchestrator). Everything below uses crates already in `Cargo.lock`.
- **Layer rule** (`src/overlay/mod.rs` header): pure logic in the library, egui a thin view. Chord parsing/formatting, the key table, the capture state machine and the registration orchestration must be pure and unit-tested; `app.rs` only wires them.
- **Tests never register a real OS hotkey** and never resolve the real FerrisFilter folder (`tests/isolation_guard.rs`; lib tests pin homes). Screenshot mode skips hotkey and tray and never applies.
- **Overlay UI not texty**: explanations are tooltips; no standing captions; no new/edited tags.
- **CLAUDE.md is the domain reference and must stay accurate**: the `ff` row (screenshot flags), the `ff-settings.json` row (`hotkey` key), the overlay note ("Press **`Ctrl+Shift+Z`** (system-wide, hardcoded)"), the tray Quit wording and the Config tab note (new Overlay section) all change in this phase.
- **CHANGELOG**: agents add no entries during a milestone (Release Review drafts them). README / `installer/portable-readme.txt` are human-written; keep any edit accurate and light (wrap prose at 70 chars).
- **Assets**: `assets/poe2/` is regenerated by `ff-art` from `manifest.toml`; never hand-edit the PNGs. `ff-art` is a non-default workspace member (nightly); run with `cargo ff-art <cmd>`; set `CARGO_TARGET_DIR` outside the repo if `ff.exe` is running.
- **Git**: milestone branching; this worktree is on `gsd/phase-47-customizable-hotkey`. Phase 44 edits `show_styles` in parallel (keep Overlay changes in a new `show_overlay` fn); Phase 48 (ART-04) will change `ff_reload_button` margins (do not add local offsets for the reset button).

## Summary

The hotkey today is a hardcoded constant pair (`TOGGLE_MODIFIERS` / `TOGGLE_CODE`) registered once in `overlay::launch()` before `eframe::run_native`, with the manager **dropped on failure**, released at tray Quit by unregistering the **constant** chord, and named literally in five places (tray tooltip/menu, header close tooltip, second-launch message box, code comments, CLAUDE.md/README). Making it configurable is mostly plumbing: a pure `Chord` type with its own text format, a registrar trait seam over `GlobalHotKeyManager`, a live-chord runtime held by the app, and one new pending field in `ConfigState` so every existing dirty/Revert/Quit path picks it up for free.

The Apply integration fits the existing validate-then-write pipeline cleanly: `apply_all::prepare` already does every refusal before any byte is written, and `commit` writes through `safe_write::write_all_or_none_before` with in-memory rollback. Add `ff-settings.json` as a fourth `Target` (only when a chord edit is pending), refuse in `prepare` if the settings file exists but is unusable, and call a registration hook between `prepare` and `commit`. On commit failure, unregister the new chord; on success, unregister the old one, update tray/header, and adopt. Registration happens inside `App::ui` on the main thread, which is the thread that created the hotkey manager's hidden window — the same thread tray Quit already unregisters from today.

Key gotchas found in the sources: Windows refuses re-registering keystrokes already registered (so "new == live" must be a no-op), egui-winit turns Ctrl+C/X/V, Ctrl+Insert, Shift+Insert and Shift+Delete into `Event::Copy/Cut/Paste` with **no `Event::Key`**, shifted punctuation arrives as different logical keys (`Plus`, `Exclamationmark`, …) so non-letters must use `physical_key`, egui 0.36 emits modifier keys (`ShiftLeft`…`SuperRight`) as `Event::Key`, and the game's keybind border texture has an **irregular, gradient white interior** that a rectangular `clear` cannot remove.

**Primary recommendation:** Build a pure `src/overlay/chord.rs` (format/parse/egui-key table/validation) and a `HotkeyRegistrar` trait in `hotkey.rs`; store the pending chord in `ConfigState` (`Option<Chord>`, None = no edit); make `ff-settings.json` a fourth all-or-none target in `apply_all` with registration between `prepare` and `commit`; extend `ff-art` with a small "key out the white interior" manifest option for the border art.

## Architectural Responsibility Map

Desktop app; the "tiers" are the app's layers.

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| Chord text format, parse, validate, egui-Key→Code table | Pure library (`src/overlay/chord.rs`) | — | Layer rule; round-trip tests without egui context or OS |
| Capture state machine (start/cancel/commit/reject captions) | Pure library (`chord.rs` or `hotkey_capture.rs`) | App (`app.rs` feeds it drained events) | Testable with synthetic `egui::Event`s |
| OS registration / unregistration / swap / startup fallback | `hotkey.rs` behind `HotkeyRegistrar` trait | App holds the runtime | Test seam: fake registrar; real one wraps `GlobalHotKeyManager` |
| Pending chord edit (dirty, revert, counts) | `ConfigState` (`config_state.rs`) | App (sets it with the live chord) | Reuses every existing CONFIG dirty path |
| Persisting `hotkey` | `apply_all` (settings target) + `settings.rs` (field, salvage, validation warning) | — | All-or-none with styles/tiers/filter |
| Tray tooltip / failure menu line | `tray.rs` | App calls after Apply | Must run on the main thread |
| Keybind button widget | `src/skin/widgets.rs` + `textures.rs` | `config_view.rs::show_overlay` | Skin owns painting; view owns layout/actions |
| Art | `tools/ff-art` + `assets/poe2/manifest.toml` | — | Generated assets only |

## Standard Stack

No new libraries. Versions verified in `Cargo.lock` [VERIFIED: Cargo.lock grep].

### Core (already in tree)
| Library | Version | Purpose in this phase |
|---------|---------|----------------------|
| global-hotkey | 0.8.0 | `GlobalHotKeyManager::register/unregister`, `HotKey::new(Some(Modifiers), Code)`, `HotKey::id()`, `Error::AlreadyRegistered` |
| keyboard-types | 0.7.0 | `Code` / `Modifiers` (re-exported as `global_hotkey::hotkey::{Code, Modifiers}`) |
| egui / eframe / egui-winit | 0.36.1 | `Event::Key { key, physical_key, modifiers, .. }`, `InputState::keys_down`, `Context::input_mut`, `Memory::surrender_focus` |
| tray-icon | 0.24.2 | `TrayIcon::set_tooltip(Option<S>)` |
| muda | 0.19.3 | `Menu` (Clone, Rc inside), `Menu::append/remove`, `MenuItem::set_text` |
| windows-sys | 0.61 (feature `Win32_UI_Input_KeyboardAndMouse` already on) | `GetAsyncKeyState(VK_LWIN/VK_RWIN/VK_INSERT)` for Windows-key detection and Copy disambiguation |
| serde_json / tempfile | in tree | settings serialization (same as `save_keys`) |

**Installation:** none.

## Package Legitimacy Audit

No external packages are installed by this phase. slopcheck not run (nothing to check).

**Packages removed due to slopcheck [SLOP] verdict:** none
**Packages flagged as suspicious [SUS]:** none

## global-hotkey 0.8 facts (Windows)

All [VERIFIED: ~/.cargo/registry/src/.../global-hotkey-0.8.0/src].

- `HotKey::new(mods, key)` maps `META` to `SUPER`; `id = (mods.bits() << 16) | key as u32`. A different chord always has a different id, so register-new-then-unregister-old never collides on id.
- `register` sets `MOD_NOREPEAT | MOD_SHIFT/ALT/CONTROL`, `MOD_WIN` for `SUPER|META`, maps the `Code` to a VK via `key_to_vk`, and calls `RegisterHotKey(hwnd, id, mods, vk)`. `ERROR_HOTKEY_ALREADY_REGISTERED` → `Error::AlreadyRegistered(HotKey)`; other failures → `Error::OsError`; a `Code` with no VK → `Error::FailedToRegister("Unknown VKCode for …")`.
- `key_to_vk` covers: KeyA-Z, Digit0-9, Equal, Comma, Minus, Period, Semicolon, Slash, Backquote, BracketLeft, Backslash, BracketRight, Quote, Backspace, Tab, Space, Enter, CapsLock, Escape, PageUp/Down, End, Home, Arrow*, PrintScreen, Insert, Delete, **F1-F24**, NumLock, Numpad*, ScrollLock, media keys, Pause. **Not** `IntlBackslash`.
- `unregister` → `UnregisterHotKey(hwnd, id)`; failure → `FailedToUnRegister`.
- The manager owns a hidden window created in `new()` on the calling thread; `Drop` only `DestroyWindow`s.
- `FromStr`/`Display` exist but produce `shift+control+KeyZ`-style text (lowercase modifiers, `Code` Debug names). **Do not use them for the settings text**; UI-SPEC requires FerrisFilter's own `Ctrl+Shift+Z` format.
- Event delivery: `WM_HOTKEY` → `GlobalHotKeyEvent { id, Pressed }`, then a spawned thread polls `GetAsyncKeyState` and sends `Released`. The existing poll already filters `Pressed` + our id.

Win32 `RegisterHotKey` [CITED: learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-registerhotkey]:
- "This function fails if you try to associate a hot key with a window created by another thread." → registration must run on the main thread (it does, see Thread affinity).
- "Typically, RegisterHotKey also fails if the keystrokes specified for the hot key have already been registered for another hot key." → re-registering the live chord fails; treat "new == live" as no edit / no-op.
- "The F12 key is reserved for use by the debugger at all times, so it should not be registered as a hot key." → bare/modified F12 may misbehave; the normal refusal path covers a failure (see Pitfall 9).
- `MOD_WIN`: "Keyboard shortcuts that involve the WINDOWS key are reserved for use by the operating system." → matches the "reject Windows key" decision.
- App ids should be 0x0000-0xBFFF; global-hotkey's ids exceed that and it works today with `Ctrl+Shift+Z` [VERIFIED: current code runs]. Not this phase's problem; noted only.

## egui key input facts (0.36.1, egui-winit 0.36.1)

All [VERIFIED: egui-0.36.1/src/data/key.rs, data/input/event.rs, egui-winit-0.36.1/src/lib.rs].

- `Event::Key { key, physical_key: Option<Key>, pressed, repeat, modifiers }`. `key` = logical key **or** physical fallback (`logical_key.or(physical_key)`); `physical_key` = `key_from_key_code(KeyCode)` (position, ignores layout).
- `egui::Key` has `F1`…`F35` (F13-F24 are mapped from both `NamedKey` and `KeyCode`), letters, `Num0-9`, arrows, Escape/Tab/Backspace/Enter/Space/Insert/Delete/Home/End/PageUp/PageDown, punctuation (`Comma, Period, Slash, Semicolon, Quote, OpenBracket, CloseBracket, Minus, Equals, Backtick, Backslash`, plus shifted `Plus, Colon, Pipe, Questionmark, Exclamationmark, OpenCurlyBracket, CloseCurlyBracket`), and **new in 0.36: `ShiftLeft/Right, ControlLeft/Right, AltLeft/Right, SuperLeft/Right, IntlBackslash`** emitted as physical `Event::Key`s.
- `egui::Modifiers { alt, ctrl, shift, mac_cmd, command }`. On Windows `mac_cmd` is always false and there is **no Windows-key flag**; `ModifiersChanged` sets only alt/ctrl/shift/command.
- `InputState::keys_down: HashSet<Key>` is updated in `begin_pass` from `Event::Key` (so it contains `SuperLeft`/`SuperRight` while held — their logical `NamedKey::Super` maps to `None`, so `key` falls back to the physical variant).
- **Copy/Cut/Paste interception** (egui-winit `on_keyboard_input`): when pressed, `is_cut_command` (command+X, Shift+Delete on Windows), `is_copy_command` (command+C, Ctrl+Insert), `is_paste_command` (command+V, Shift+Insert) push `Event::Cut/Copy/Paste` and **return without pushing `Event::Key`**. `command == ctrl` on Windows, so Ctrl+Shift+C etc. also become `Copy`.
- Logical keys follow Shift: Shift+`=` → `Plus`, Shift+`1` → `Exclamationmark`, Shift+`/` → `Questionmark`, Shift+`;` → `Colon`, Shift+`\` → `Pipe`, Shift+`[` → `OpenCurlyBracket`. Shift+`7` on US → "&" (no egui Key) → falls back to physical `Num7`. `physical_key` is stable (`Equals`, `Num1`, …).
- Numpad: physical `Numpad0-9` → `Num0-9`, `NumpadEnter` → `Enter`, `NumpadSubtract` → `Minus`, `NumpadDivide` → `Slash`, `NumpadAdd` → `Plus`. egui exposes no key location.
- Letters: `Key::from_name` accepts upper and lower case, so Shift+z is `Key::Z`. On AltGr (Ctrl+Alt) layouts the logical key may be a symbol (→ `None`), then the physical letter is used.
- APIs: `Context::input_mut(|i| …)` (context.rs:1002), `InputState::pointer.any_pressed()` / `press_origin()`, `Memory::surrender_focus(id)` / `request_focus(id)`, `viewport().focused`.

## Thread affinity (registration at Apply)

- `launch()` registers before `run_native` on the main thread [VERIFIED: src/overlay/mod.rs:82-87]. eframe 0.36 native runs `App::ui` on the same (winit event-loop) thread [VERIFIED: tray Quit already calls `hotkey::release_toggle` → `UnregisterHotKey` from `begin_shutdown`, which runs inside `ui()`, and works]. `run_apply` is called synchronously from `ui()` (bar / `handle_config_actions`), so registering/unregistering in Apply is on the right thread. **Never** move registration to a worker.
- `TrayIcon::set_tooltip` / muda menu edits have the same main-thread rule (tray.rs module docs) and are also called from `ui()`.

## Architecture Patterns

### System Architecture Diagram

```
 start-up (main thread, launch())
   resolve root ──► settings::load(root) ──► saved `hotkey` (validated in settings.rs; invalid → None + warning)
        │                                              │
        ▼                                              ▼
   GlobalHotKeyManager::new()  ──►  hotkey::startup(registrar, saved) ──► HotkeyRuntime{registrar, live: Option<Chord>, status, warning}
                                         │ saved taken? try Ctrl+Shift+Z; both taken → Failed(last tried)
                                         ▼
                                   tray::build(tooltip(live/status), failure line?)
                                         ▼
                                   OverlayApp::new(tray, runtime, …)  (warning → list_status)

 every frame (OverlayApp::ui)
   ┌─ capture active? ── drain Key/Text/Copy/Cut/Paste events from ctx.input_mut ──► capture::step ──► Commit(chord) │ Reject(caption) │ Cancel │ Continue
   │                                                                                       │ Commit
   │                                                                                       ▼
   │                                                               ConfigState::set_hotkey(chord, live)  (equal to live → clears edit)
   ├─ GlobalHotKeyEvent (Pressed, id == live id)
   │      capturing → capture commits the live chord (no toggle)
   │      else foreground game/overlay → toggle_requested
   └─ CONFIG tab: show_overlay (keybind button, reset) → ConfigAction::{StartCapture, ResetHotkey}

 Apply (run_apply → apply_everything)
   apply_all::prepare(root, config{… hotkey_pending}, filter, overwrite)
        │ refusals (invalid, grammar, changed on disk, ff-settings.json unusable) → nothing written
        ▼
   register hook: registrar.register(new)  ── Err(AlreadyRegistered) ──► refuse: "<new> is in use by another app. <old> still works; nothing was applied."
        │ Ok
        ▼
   apply_all::commit: write_all_or_none(styles?, tiers?, filter?, ff-settings.json) ── Err ──► registrar.unregister(new); old stays
        │ Ok
        ▼
   registrar.unregister(old); runtime.live = new; settings.hotkey = new; tray.set_hotkey_state; outcome "+ Show / hide is now <new>."
```

### Recommended file layout
```
src/overlay/chord.rs        # NEW, pure: Chord {ctrl, shift, alt, key: ChordKey}; format/parse; egui Key→ChordKey; ChordKey→Code; validate; DEFAULT
src/overlay/hotkey.rs       # registrar trait + real impl, HotkeyRuntime/startup/swap/release, tooltip text helpers
src/overlay/config_state.rs # + hotkey_pending: Option<Chord>
src/overlay/config_view.rs  # + show_overlay (new fn only), ConfigAction::{StartHotkeyCapture, CancelHotkeyCapture, ResetHotkey}, ctx fields
src/overlay/apply_all.rs    # + settings target (prepare/commit), Committed.hotkey
src/overlay/app.rs          # wiring: runtime field, capture field, event draining, poll, apply hook, header tooltip, shutdown
src/overlay/tray.rs         # + kept Menu/status item clones, set_hotkey_state
src/overlay/instance.rs     # message stops naming the chord
src/overlay/mod.rs          # launch(): settings read + hotkey::startup before tray::build
src/overlay/screenshot.rs   # --hotkey, --capture-hotkey
src/settings.rs             # + hotkey: Option<String>, salvage + validation warning
src/skin/textures.rs        # + keybind_border, keybind_plate
src/skin/widgets.rs         # + ff_keybind_button
tools/ff-art/src/main.rs    # + white-key option (see Pitfall 1)
assets/poe2/manifest.toml   # + two [[image]] entries
```

### Pattern 1: Startup and the hotkey runtime
**What:** Keep the manager alive whenever `GlobalHotKeyManager::new()` succeeds, even if registration fails (today it is dropped on failure, which would make a later Apply impossible). Track the live chord separately from the status.

```rust
// hotkey.rs (sketch)
pub trait HotkeyRegistrar {
    fn register(&mut self, chord: Chord) -> Result<(), RegisterError>;
    fn unregister(&mut self, chord: Chord);
}
pub enum RegisterError { InUse, Other(String) }      // map global_hotkey::Error::AlreadyRegistered → InUse

impl HotkeyRegistrar for GlobalHotKeyManager { /* HotKey::new(Some(chord.modifiers()), chord.code()) */ }

pub struct HotkeyRuntime {
    registrar: Option<Box<dyn HotkeyRegistrar>>,   // None: manager could not be created (or screenshot/test)
    live: Option<Chord>,                            // None: nothing registered
    pub status: HotkeyStatus,                       // Registered{id} | Failed(text) — id = HotKey id of `live`
    pub shown: Chord,                               // what the UI calls the applied chord (live, else the last tried)
}

pub fn startup(reg: Option<Box<dyn HotkeyRegistrar>>, saved: Option<Chord>) -> (HotkeyRuntime, Option<String /*warning*/>)
// saved None → try DEFAULT.
// saved Some(c) → try c; InUse/Other → try DEFAULT; Ok → warning "`hotkey` <c> is in use by another app, so Ctrl+Shift+Z is used".
// both fail → status Failed(reason), naming the chord tried last (DEFAULT).
pub fn release(rt: &mut HotkeyRuntime)   // unregister live (not the constant!), then drop registrar
```
The poll in `ui()` stays as is but compares `event.id` with the **live** id. `HotkeyStatus::Registered { id }` can stay; recompute `id` on swap.

**Settings validity warning** (`hotkey` unparsable / no modifier and not F-key / Windows key / wrong type) is produced in `settings::load_parts` like `game_folder` (drop to `None` + warning text from the UI-SPEC copy table), so it composes with the other key warnings. The "in use" warning comes from `startup` and is passed into the app; when both a settings warning and a hotkey warning exist, join them into `list_status` (e.g. `"{a} {b}"`).

### Pattern 2: Pending chord in ConfigState (`Option<Chord>`, None = no edit)
**What:** Store `hotkey_pending: Option<Chord>` in `ConfigState`. Not a baseline compare: `ConfigState::load` then needs no knowledge of the live chord, and a reload of ConfigState (root change, changed-on-disk Reload) simply drops the edit like other config edits.

- `set_hotkey(chord, live: Option<Chord>)`: `self.hotkey_pending = (Some(chord) != live).then_some(chord)` (UI-SPEC rule 8: capturing the applied chord clears the edit).
- `hotkey_dirty()`, include `+ usize::from(hotkey_dirty)` in `dirty_count()` and `scope_dirty_count(RevertScope::Styles)`; clear it in `revert()` and `revert_scope(Styles)`. **Do not** fold it into `styles_dirty()` / `styles_changed()` (those drive economy re-pricing at app.rs:2839 and `follow_disk`).
- `prepare_write` is unaffected (writes no config file for a hotkey-only edit; `styles_changed`/`tiers_changed` stay false). `can_apply()` becomes true via `dirty_count`.
- App sites to touch: the Config tab marker and Config bar Revert use the local `styles_dirty` (app.rs:3742 → `c.styles_dirty() || c.hotkey_dirty()`; pass that to `shell::tab_markers` and `ConfigBarInputs`). Every `config_dirty` (`c.is_dirty()`) site — unapplied indicator, `combined_dirty`, Apply enable, Quit popup (`applicable_dirty`, `quit_tier_edits_only`), Revert popup count (app.rs:2366) — picks it up automatically.
- Reset = `set_hotkey(DEFAULT, live)`.
- After a successful Apply the pending is cleared by `adopt` (or explicitly by the app after the swap); the app gets the new chord from `Committed`.

### Pattern 3: Pure chord module
```rust
pub struct Chord { pub ctrl: bool, pub shift: bool, pub alt: bool, pub key: ChordKey }
pub enum ChordKey { Letter(u8 /*b'A'..=b'Z'*/), Digit(u8), F(u8 /*1..=24*/), Space, Tab, Enter, Backspace,
                    Insert, Delete, Home, End, PageUp, PageDown, Up, Down, Left, Right,
                    Comma, Period, Slash, Semicolon, Quote, BracketLeft, BracketRight, Minus, Equal, Backquote, Backslash }
pub const DEFAULT: Chord = /* Ctrl+Shift+Z */;
impl Chord {
    pub fn text(&self) -> String;                 // "Ctrl+Shift+Alt+<key>" in that order (UI-SPEC copy table)
    pub fn parse(s: &str) -> Result<Chord, ()>;   // own format; case-insensitive modifiers/named keys; reject Win/Super/Meta/Cmd tokens, duplicates, unknown keys, Escape
    pub fn is_bindable(&self) -> bool;            // any modifier, or key is F1..F24
    pub fn modifiers(&self) -> Modifiers;         // CONTROL|SHIFT|ALT (never SUPER)
    pub fn code(&self) -> Code;                   // table below
}
pub fn key_from_event(key: egui::Key, physical: Option<egui::Key>) -> Option<ChordKey>;
```
**Key resolution rule:** if `key` is a letter A-Z use it (layout-correct, matches VK_A-Z semantics of `RegisterHotKey`); otherwise use `physical_key` (stable under Shift), else `key`. Escape and the eight modifier variants are never `ChordKey` (handled by the state machine).

**egui → ChordKey → Code table** (round-trip test every row: `parse(text(c)) == c`, `code()` has a VK in global-hotkey's `key_to_vk` list):

| egui Key (physical) | ChordKey text | `Code` |
|---|---|---|
| A..Z | `A`..`Z` | `KeyA`..`KeyZ` |
| Num0..Num9 | `0`..`9` | `Digit0`..`Digit9` |
| F1..F24 | `F1`..`F24` | `F1`..`F24` |
| Space, Tab, Enter, Backspace | `Space`, `Tab`, `Enter`, `Backspace` | same |
| Insert, Delete, Home, End, PageUp, PageDown | same | same |
| ArrowUp/Down/Left/Right | `Up`, `Down`, `Left`, `Right` | `ArrowUp`… |
| Comma, Period, Slash, Semicolon, Quote | `,` `.` `/` `;` `'` | `Comma`, `Period`, `Slash`, `Semicolon`, `Quote` |
| OpenBracket, CloseBracket | `[` `]` | `BracketLeft`, `BracketRight` |
| Minus, Equals, Backtick, Backslash | `-` `=` `` ` `` `\` | `Minus`, `Equal`, `Backquote`, `Backslash` |
| F25-F35, IntlBackslash, Plus/Colon/Pipe/…(no physical), Copy/Cut/Paste keys, BrowserBack | — | rejected: `Key not supported` |

Note `+` is never a key token (the `=` key is `=`), so splitting on `+` is unambiguous; parse `Ctrl+Shift+=` correctly by splitting into at most N tokens where the last is the key.

### Pattern 4: Capture state machine and event ownership
- State on the app (not `ConfigUi`), because events must be drained at the **top of `ui()`**, before any widget, popup or shortcut reads them: `capture: Option<CaptureState { rejected: Option<RejectReason> }>`.
- Each frame while capturing:
  ```rust
  let events = ui.ctx().input_mut(|i| {
      let (mine, rest): (Vec<_>, Vec<_>) = std::mem::take(&mut i.events).into_iter()
          .partition(|e| matches!(e, egui::Event::Key{..} | egui::Event::Text(_) | egui::Event::Copy
                                      | egui::Event::Cut | egui::Event::Paste(_) | egui::Event::Ime(_)));
      i.events = rest; mine
  });
  let held = ui.ctx().input(|i| i.modifiers);            // live caption Ctrl/Shift/Alt
  let win  = ui.ctx().input(|i| i.keys_down.contains(&Key::SuperLeft) || i.keys_down.contains(&Key::SuperRight))
             || win::windows_key_held();                 // GetAsyncKeyState(VK_LWIN|VK_RWIN), new helper beside modifiers_held()
  match capture::step(state, &events, held, win) { … }
  ```
  `capture::step` is pure: ignores `pressed == false`, `repeat`, and the modifier variants; Escape → Cancel; first other press → resolve `ChordKey` → Commit if bindable and no Windows key, else stay with a reject reason.
- **Copy/Cut/Paste** (no `Event::Key` exists for these chords): map with the event's modifiers known from `held` — `Copy` + ctrl → `C` unless `GetAsyncKeyState(VK_INSERT)` is down (then `Insert`); `Cut` + ctrl → `X`, + shift only → `Delete`; `Paste` + ctrl → `V`, + shift only → `Insert`. Keep it in the pure step with an `insert_down: bool` input.
- Also call `ctx.memory_mut(|m| m.surrender_focus(focused))` when capture starts so no text field holds focus (a field would otherwise not see events anyway once drained, but cursor blink/IME state stays clean).
- Cancel conditions (UI-SPEC): pointer press outside the button (`i.pointer.any_pressed()` with `press_origin` outside the button rect remembered from last frame), second click on the button, tab switch, window hidden (any `Visibility` transition to Hidden), `viewport().focused == Some(false)`, any popup opening.
- Hotkey poll while capturing: an event with the live id commits the live chord (no toggle) — UI-SPEC rule 7.
- Request a repaint while capturing is not needed: keyboard events and `ModifiersChanged` already repaint.

### Pattern 5: Apply integration (all-or-none)
- `apply_all::prepare`: if `config.hotkey_pending()` is `Some(c)`, build a `SettingsPart { path: settings::path(root), guard: Option<Vec<u8>>, new: Vec<u8>, chord: c }`:
  - read the file now; `NotFound` → `guard None`, `new = to_string_pretty(&{"hotkey": c.text()}) + "\n"`;
  - exists and parses as a JSON object → insert `"hotkey"` (keeps every other key, even wrongly typed ones — same rule as `save_keys`), `guard Some(bytes)`;
  - unreadable or not an object → `ApplyAllError::Invalid("ff-settings.json could not be read, so the hotkey cannot be saved; nothing was applied. Fix or delete the file, then Reload.")`.
  Do **not** add it to the `ChangedOnDisk` check (no load baseline exists for the settings file; the guard is the bytes read in `prepare`, which only trips if something writes in the milliseconds before commit, then surfaces as a `Write` error).
- `Prepared` gets `settings: Option<SettingsPart>` and a `pub fn hotkey(&self) -> Option<Chord>`.
- `commit_with`: `create_dir_all(root)` when the settings part exists (the folder may not exist yet), push it as the **last** `Target` (styles, tiers, filter, settings), and report `Committed.hotkey_saved: Option<Chord>`. On success clear `config.hotkey_pending`.
- `apply_everything` (app.rs:624) gains a hook param, called after `prepare` returns `Ok` and only when `prepared.hotkey().is_some()`:
  ```rust
  register: &mut dyn FnMut(Chord) -> Result<(), String /* finished refusal sentence */>,
  ```
  After `commit` fails, `run_apply` unregisters the new chord. After success: unregister old, `runtime.live = Some(new)`, recompute `status`, `self.settings.hotkey = Some(new.text())` (in-memory, so later whole-file `settings::save` calls keep it), `tray.set_hotkey_state(...)`, append ` Show / hide is now <new>.` to `extras`.
- Refusal text when the registrar says `InUse`: `"<new> is in use by another app. <old> still works; nothing was applied."` (no live old chord: `"<new> is in use by another app; nothing was applied."`). `Other(e)` from the OS: use the same sentence shape with the reason (planner's wording; keep "nothing was applied").
- No registrar (manager creation failed / screenshot / tests without a fake): refuse with an `Other` reason rather than silently saving a chord that is not registered.
- The test rig's `fn apply` (app.rs:4721) and existing tests pass a no-op hook.

### Pattern 6: Keybind button widget
Follow `ff_button_ex` / `ff_reload_button` structure (allocate, `widget_info`, state 0/1/2, tint, paint, caption galley). Paint `keybind_plate` then `keybind_border` with `slice::paint_nine(painter, tex, rect, Margins::new(m,m,m,m), 30.0 / tex_h, tint, draw_center)`. Both trimmed textures are 60x60 [VERIFIED: ff-art dump + info, opaque bbox [0,0,60,60] in a 68x68 image]; the border's corner ornament clusters extend about 20 px (visual estimate from an 8x zoom, MEDIUM — measure when writing margins). The plate is horizontally uniform with a bright band at the bottom and a radial glow above it, so stretching horizontally reproduces the reference's gold bottom line.

### Anti-Patterns to Avoid
- **Unregistering the old chord before the new one is confirmed** — loses the binding to another app if the new one fails.
- **Re-registering the live chord** (e.g. Apply with pending == live, or Reset when already default) — `RegisterHotKey` fails for already-registered keystrokes; treat as no edit.
- **Using `HotKey::from_str` / `Display` for the settings text** — different format (`shift+control+KeyZ`).
- **Reading the pending chord from `settings.hotkey`** — `settings` holds the saved value; pending lives in `ConfigState`, live in the runtime.
- **Folding the hotkey into `styles_dirty()`** — triggers economy re-pricing previews.
- **Handling capture keys inside `config_view`** — popups and shortcuts elsewhere would already have consumed Escape/keys; drain at the top of `ui()`.
- **Registering from a worker thread** — `RegisterHotKey` fails for a window created by another thread.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Multi-file atomic write with rollback | a separate `settings::save` after `commit` | a 4th `Target` in `safe_write::write_all_or_none_before` | existing guarded, staged, rolled-back writer; keeps "a refusal writes nothing" |
| Per-key settings merge | new JSON merge code | the `save_keys` approach (parse Map, insert key, pretty print + `\n`) — factor a shared `render_with_key(text, key, value)` helper | preserves unknown and wrongly typed keys exactly as `save_keys` does |
| Nine-slice painting | custom mesh math | `slice::paint_nine` | pixel snapping, ppp handling already tested |
| Modifier-held query | new Win32 code paths | extend `win.rs` beside `modifiers_held()` (`GetAsyncKeyState`) | same feature flag, same `#[cfg(windows)]` / stub pattern |
| Hotkey OS calls | raw `RegisterHotKey` | `global_hotkey::GlobalHotKeyManager` | already owns the hidden window and the event channel |

## Runtime State Inventory

Not a rename phase, but the hardcoded chord is "renamed" to a variable, so every place naming it was inventoried.

| Category | Items Found | Action Required |
|----------|-------------|------------------|
| Stored data | `ff-settings.json` gains `hotkey` (absent = Ctrl+Shift+Z). No existing data to migrate. | code only |
| Live service config | none — verified: no external service knows the chord | none |
| OS-registered state | the `RegisterHotKey` registration itself (process lifetime; released at Quit by `release_toggle`, which unregisters the **constant** today) | code: release the live chord; swap at Apply |
| Secrets/env vars | none — `FERRISFILTER_HOME` / `_GAME_DIR` unaffected | none |
| Build artifacts | `assets/poe2/options/keybind_border.png`, `keybind_plate.png` to generate (embedded via `include_bytes!`, so a rebuild picks them up) | regenerate with `cargo ff-art`; commit PNGs |

Chord mentions to change [VERIFIED: grep `Ctrl+Shift+Z`]:
- `src/overlay/mod.rs:94,97` tray tooltip/menu text → from runtime; `:76` comment.
- `src/overlay/app.rs:3811` header close tooltip `Hide (Ctrl+Shift+Z)` → `Hide (<live>)`; comments at `:93,96,107,1546,2706`.
- `src/overlay/instance.rs:154` second-launch message → `… Use its tray icon or its show / hide hotkey instead.`
- `src/overlay/hotkey.rs` doc comments and the `toggle_combo_is_shift_z` test (replace with `DEFAULT` tests).
- `CLAUDE.md:59` overlay note and the other rows listed in Project Constraints.
- `README.md:62`, `installer/portable-readme.txt:10` (human-written; light accurate edit such as "`Ctrl+Shift+Z` (changeable on the Config tab)" — flag for the author's Release Review if not edited).
- Unchanged: the `Ctrl+Shift+Z` default text in reset tooltip / fallback warnings (correct by design).

## Keybind art (KEY-01)

[VERIFIED: `ff-art list` + `ff-art dump` against the local Steam install, 2026-10-23]
- `art/textures/interface/2d/2dart/uiimages/ingame/options menu/optionsbuttonborder.dds` and `…/optionsbutton.dds` exist, 68x68 each with an 8 px transparent right/bottom gutter (opaque 60x60 → `trim = true`). No 4K sibling was listed (`list` of the `4k/…` path returned nothing), so omit `prefer_4k`.
- **The fallback to the generic button art is not needed** (report for the author).
- `optionsbuttonborder`: dark gold frame with four ornate corner clusters and a **white, soft-edged, non-rectangular interior** (bulges between the corner clusters; grey gradient fringe). See Pitfall 1.
- `optionsbutton`: greyscale plate, darker top half, bright band along the bottom with a radial glow — tint at draw time as the UI-SPEC says.
- A built `ff-art.exe` exists in the main checkout's `target/release` (used read-only for this check); a change to `ff-art` needs a rebuild (`cargo ff-art …`, nightly, builds `poe_data_tools`; slow first time).

## Common Pitfalls

### Pitfall 1: The border's white interior cannot be removed with a rectangular `clear`
**What goes wrong:** UI-SPEC prescribes `clear = [[x, y, w, h]]` for the "opaque white interior". The interior is irregular (narrow near the corner clusters, wide between them) with an anti-aliased grey fringe, so any rectangle either leaves white/grey fringe in the nine-slice edge pieces or erases parts of the gold corners.
**Why it happens:** the game likely uses the white area as a mask/lit region when compositing the plate; FerrisFilter draws with plain alpha blending.
**How to avoid:** add one small option to `ff-art`'s `ImageEntry`, e.g. `key_white = [x, y]`: flood-fill from the seed through connected pixels whose `min(r,g,b)` is above a threshold (start ~96), and apply colour-to-alpha against white on those pixels (`a' = a * (1 - min(r,g,b)/255)`, colour un-premultiplied toward black). Corner ornaments are separated from the interior by the dark frame line, so the flood does not reach their highlights. ~30 lines, no new dependency (`image` is already used). Verify with `ff-art zoom` and a debug screenshot against `options-keybind-rows.png`. If it still fringes, fall back to `draw_center = false` plus several `clear` rects approximating the shape and accept a slightly darker interior edge — record the outcome for the author.
**Warning signs:** a pale halo just inside the gold line in the screenshot.

### Pitfall 2: The manager is dropped on registration failure
**What goes wrong:** `register_toggle` returns `(None, Failed)` when the chord is taken, so the app has no manager and a later Apply can never register anything.
**How to avoid:** keep the manager whenever `GlobalHotKeyManager::new()` succeeded; `live = None` describes "nothing registered".

### Pitfall 3: Same keystrokes registered twice fail
**What goes wrong:** Apply with pending == live, or a chord equal to live after Reset, calls `register` on keystrokes this process already owns → `AlreadyRegistered` → a bogus "in use" refusal.
**How to avoid:** `set_hotkey` clears the edit when equal to live; the swap function returns early when `new == live`.

### Pitfall 4: Ctrl+C/X/V, Ctrl+Insert, Shift+Insert, Shift+Delete never arrive as `Event::Key`
**What goes wrong:** the capture waits forever (or commits on a later key) for these chords.
**How to avoid:** handle `Event::Copy/Cut/Paste` in the capture step (mapping in Pattern 4). Drain them too, so a focused field does not paste.

### Pitfall 5: Shifted punctuation is a different logical key
**What goes wrong:** Shift+`=` arrives as `Key::Plus`, Shift+`1` as `Exclamationmark`; a logical-key table would reject or mis-label them.
**How to avoid:** non-letters resolve through `physical_key` (Pattern 3 rule). Letters prefer the logical key so the chord matches the VK the user's layout labels.

### Pitfall 6: Modifier keys are `Event::Key`s in egui 0.36
**What goes wrong:** pressing Shift first "commits" a chord whose key is `ShiftLeft` (not in the table → `Key not supported` shown on every modifier press).
**How to avoid:** the capture step ignores `ShiftLeft/Right, ControlLeft/Right, AltLeft/Right` and treats `SuperLeft/Right` only as the Windows-key flag.

### Pitfall 7: No Windows-key flag in `egui::Modifiers` on Windows
**How to avoid:** detect via `keys_down` (`SuperLeft/Right`) plus `GetAsyncKeyState(VK_LWIN|VK_RWIN)`. Shell-owned Win+key combos (Win+E, Win+L) never reach the app at all — no handling needed. Answer for UI-SPEC's open point: egui *does* report the Windows key on Windows, as `Key::SuperLeft/Right` key events, not as a modifier.

### Pitfall 8: Numpad and AltGr
**What goes wrong:** numpad digits come in as `Num0-9` (no location), are saved as `0`-`9` and registered as `VK_0`-`VK_9`, so the numpad key itself will not trigger the hotkey. On European layouts AltGr is Ctrl+Alt, so a `Ctrl+Alt+<key>` chord swallows a typed character system-wide.
**How to avoid:** accept as known limitations (document in a code comment; optional tooltip text is out of scope per "not texty"). LOW impact.

### Pitfall 9: F12 is reserved by Windows for debuggers
**What goes wrong:** Microsoft says F12 "should not be registered as a hot key"; behaviour may differ by machine.
**How to avoid:** keep UI-SPEC's "F1-F24 allowed"; if registration fails the normal refusal path reports it. Do not special-case unless verification shows misbehaviour.

### Pitfall 10: Whole-file `settings::save` writes the in-memory `Settings`
**What goes wrong:** filter switch / reload toggle use `settings::save(root, &next)` (whole struct). If `settings.hotkey` held the pending chord, it would be saved without Apply; if it held the fallback after an "in use" start-up, the user's saved choice would be overwritten.
**How to avoid:** `settings.hotkey` = the saved value only. Start-up "in use" fallback keeps the saved string in memory (it applies again next launch). An *invalid* saved value is dropped to `None` at load (same precedent as `game_folder`), so a later whole save removes it.

### Pitfall 11: Settings salvage path
**What goes wrong:** a wrongly typed `hotkey` (e.g. `42`) fails typed deserialization; `load_parts` then salvages each known key from a `Map`. Without an explicit `hotkey` arm the value silently rides along in `extra` with no warning.
**How to avoid:** add a `hotkey` arm (string → validate; anything else → warning `` `hotkey` is not a usable key combination, so Ctrl+Shift+Z is used ``, value dropped) and a typed-path check after successful deserialization (invalid text → `None` + same warning). Add the key to the `Settings` struct with `skip_serializing_if = "Option::is_none"`.

### Pitfall 12: RULES Reload re-reads settings but must not re-register
**What goes wrong:** `reload()` replaces `self.settings` from disk; a hand-edited `hotkey` would then disagree with the registration.
**How to avoid:** Reload never touches the runtime (decision: only Apply and start-up register). The button keeps showing the live chord. Optional: if the reloaded file's chord differs from live, do nothing (it applies at next launch).

### Pitfall 13: Parallel phases
Phase 44 edits `show_styles` (keep `show_overlay` separate; only add its call in `show_sections`). Phase 48 changes `ff_reload_button` margins (do not offset the reset button locally). Merge conflicts in `config_view.rs` / `ConfigViewCtx` test constructors (config_view.rs:1341) are likely; keep diffs small.

## Code Examples

### Tray: live tooltip and the failure line
```rust
// tray.rs — keep clones (muda Menu/MenuItem are Rc-backed Clone) [VERIFIED: muda-0.19.3 #[derive(Clone)]]
pub struct TrayHandles { tray_icon: Option<TrayIcon>, menu: Option<Menu>, status_item: Option<MenuItem>, show_id: MenuId, quit_id: MenuId }

pub fn set_hotkey_state(&mut self, tooltip: &str, failure: Option<&str>) {
    if let Some(icon) = &self.tray_icon { let _ = icon.set_tooltip(Some(tooltip)); }   // tray-icon 0.24.2 lib.rs:405
    match (failure, &self.status_item, &self.menu) {
        (Some(text), Some(item), _) => item.set_text(text),
        (Some(text), None, Some(menu)) => { let item = MenuItem::new(text, false, None); let _ = menu.append(&item); self.status_item = Some(item); }
        (None, Some(item), Some(menu)) => { let _ = menu.remove(item); self.status_item = None; }
        _ => {}
    }
}
```
Tooltip texts: `FerrisFilter — <chord> to toggle` / `FerrisFilter — <chord> unavailable: <reason>` (put both formatters in `hotkey.rs` so `launch()` and Apply share them and tests cover them).

### Screenshot flags
- `--hotkey <chord>`: in `new_screenshot`, before `Self::build`, write the raw value into the temp home's `ff-settings.json` (the guard already proved the home is under Temp), so the normal load path produces the fallback warning for invalid values. Build the runtime with no registrar: `live = None`, `shown = parsed-or-default`, status `Failed("screenshot mode")`.
- `--capture-hotkey`: set `app.tab = ShellTab::Config` and `app.capture = Some(CaptureState::default())`.
- Add both to the `screenshot.rs` module doc line, the parser match, `ShotArgs`, and the parse tests; document in the CLAUDE.md `ff` row.

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| egui modifier keys only in `Modifiers` | egui 0.36 also emits `ShiftLeft…SuperRight`, `IntlBackslash` as `Event::Key` | egui 0.36 | capture must ignore them; enables Windows-key detection |
| hardcoded `Ctrl+Shift+Z` (Phase 13, D-09) | configurable via CONFIG, saved `hotkey` | this phase | tray/header/instance texts become dynamic |

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | Border corner clusters extend ~20 px (from an 8x zoom by eye) | Pattern 6 | wrong nine-slice margins; measure before coding |
| A2 | The white-key flood fill will not bleed into corner ornaments (dark frame line separates them) | Pitfall 1 | gold highlights partly transparent; fallback described |
| A3 | `GetAsyncKeyState(VK_INSERT)` reliably distinguishes Ctrl+Insert from Ctrl+C at the time the Copy event is processed (key still down) | Pattern 4 | Ctrl+Insert captured as Ctrl+C; rare chord |
| A4 | F12 registers on typical machines without a debugger | Pitfall 9 | refusal message instead; harmless |

## Open Questions

1. **Exact `key_white` threshold and seed for the border art**
   - Known: interior shape and fringe from the dump. Unclear: best threshold for a clean edge.
   - Recommendation: implement, run `cargo ff-art`, compare a debug screenshot (`--tab config`) with `options-keybind-rows.png`, record the chosen values and final tints in the summary.
2. **README / portable-readme wording**
   - Human-written files; recommend a minimal accurate edit (chord "by default, changeable on the Config tab") or leave a note for the Release Review. Planner decides; do not add a CHANGELOG entry.

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| Rust nightly toolchain | build/tests | ✓ | cargo/rustc 1.100.0-nightly (2026-09-03) | — |
| PoE2 install (Steam) | `ff-art` extraction | ✓ | `C:\Program Files (x86)\Steam\steamapps\common\Path of Exile 2` | committed PNGs |
| `ff-art` | art regeneration | ✓ (prebuilt in main checkout `target/release`; rebuild needed after the manifest-option change) | — | — |
| Python | — | ✗ | — | not needed (use `ff-art info/zoom`) |

**Missing dependencies with no fallback:** none.

## Security Domain

| ASVS Category | Applies | Standard Control |
|---------------|---------|-----------------|
| V2 Authentication | no | — |
| V3 Session Management | no | — |
| V4 Access Control | no | — |
| V5 Input Validation | yes | hand-edited `ff-settings.json` `hotkey` parsed by the strict own parser; invalid → default + warning; never passed to `HotKey::from_str` |
| V6 Cryptography | no | — |

| Pattern | STRIDE | Mitigation |
|---------|--------|------------|
| Corrupt/hostile settings value (huge string, odd tokens) | Tampering / DoS | strict token parser, reject unknown tokens, no allocation beyond the string |
| Overwriting a hand-edited unusable settings file | Tampering (data loss) | refuse Apply when the file exists but is unusable (never rewrite it), as UI-SPEC says |
| Global chord stealing common keys system-wide | Denial of service (usability) | modifier required (F-keys excepted), foreground-only toggle unchanged, Windows key refused |

## Sources

### Primary (HIGH confidence)
- `~/.cargo/registry/src/index.crates.io-*/global-hotkey-0.8.0/src/{hotkey.rs,error.rs,platform_impl/windows/mod.rs}` — id formula, register/unregister, errors, VK table
- `~/.cargo/registry/src/index.crates.io-*/egui-0.36.1/src/data/{key.rs,input/event.rs,input/modifiers.rs}`, `context.rs`, `memory/mod.rs`, `input_state/mod.rs`
- `~/.cargo/registry/src/index.crates.io-*/egui-winit-0.36.1/src/lib.rs` — key translation, Copy/Cut/Paste interception, modifiers
- `~/.cargo/registry/src/index.crates.io-*/tray-icon-0.24.2/src/lib.rs`, `muda-0.19.3/src/{menu.rs,items/normal.rs}`
- Codebase: `src/overlay/{hotkey,mod,tray,app,apply_all,config_state,config_view,screenshot,instance,win}.rs`, `src/settings.rs`, `src/safe_write.rs`, `src/skin/{widgets,textures,slice}.rs`, `assets/poe2/manifest.toml`, `tools/ff-art/src/main.rs`
- Game files via `ff-art list/dump/info/zoom` (local install, read-only)
- https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-registerhotkey — thread rule, duplicate keystrokes, F12, MOD_WIN

### Secondary / Tertiary
- none

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — no new deps; versions from Cargo.lock; APIs read from sources
- Architecture: HIGH — every integration point read in the codebase; one design choice (pending in ConfigState) recorded with rationale
- Pitfalls: HIGH for API pitfalls (source-verified); MEDIUM for art processing (visual inspection)

**Research date:** 2026-10-23
**Valid until:** 2026-11-22 (pinned crate versions; re-check only if egui/global-hotkey are bumped)
