# Phase 23: Auto-Reload Filter in Game on Apply - Research

**Researched:** 2026-10-13
**Domain:** Windows (windows-sys 0.61) synthetic keyboard input + clipboard to a DirectX game's chat box, from an egui overlay
**Confidence:** MEDIUM (mechanism HIGH from reference-overlay source; the `/reloaditemfilter` command itself is only weakly sourced, see Open Questions)

<user_constraints>
## User Constraints (from 23-CONTEXT.md)

### Locked Decisions
- **Trigger:** send only after a RULES-tab Apply whose outcome is `ApplyOutcome::Written`. Never on CONFIG Apply, `Unchanged`, or `Failed`.
- Only when a Path of Exile 2 window exists. Reuse the title-based detection in `src/overlay/win.rs` (title contains "path of exile", case-insensitive). Needs a "find the game window" form (game is usually NOT foreground at Apply time). Keep the title rule ("Exiled Exchange 2" must not match). Game not running -> nothing sent; the existing Refresh reminder stays.
- **Mechanism:** do what established PoE overlays do (EE2 / APT; cite source files). Expected sequence: foreground the game, Enter to open chat, clear partial text, enter `/reloaditemfilter`, Enter to send, leave focus with the game. Text entry = clipboard paste with save/restore OR `SendInput` `KEYEVENTF_UNICODE`; research picks one and justifies. Use `windows-sys` (add features as needed); no new crate unless strongly needed. If `SetForegroundWindow` fails -> report "not sent", never type into the wrong window. Verify foreground is really the game before sending keys. Must not block the egui frame for long (worker thread or bounded sequence).
- **ToS posture:** one user click -> one chat command. Never repeated, scheduled, retried in a loop, or sent without that Apply click.
- **Setting:** CONFIG-tab "Reload filter in game after Apply", persisted in `ff-settings.json`, default ON (absent key = on), `extra` map round-trip preserved. Persist like the filter selection (saved on change, no CONFIG Apply) unless it conflicts with CONF-04's Apply/Revert model. Place in the FILTER section.
- **Status:** on send, Apply message says "Reloaded in game" alongside the written path. When not sent: keep `apply::RELOAD_INSTRUCTION` ("Options > Game > Item Filter > Refresh") and add the reason (game not running, setting off, send failed + short cause).
- **Caveat to document:** `/reloaditemfilter` reloads the filter the game currently has selected; surface in UI (tooltip/caption) and CLAUDE.md/CHANGELOG. Do not switch the game's filter.
- **Docs:** CLAUDE.md overlay notes drop the "no auto-reload" claims (CLI `apply` row still needs manual Refresh). CHANGELOG `[Unreleased]` entry.
- **Testing:** never send real keystrokes in tests, never launch `ff`. Put sending behind a trait/seam (`InputSink` receiving abstract steps); unit-test step sequence, trigger decision (Written/Unchanged/Failed x setting x game present), status text, settings default/round-trip. Build with `CARGO_TARGET_DIR=C:/Users/jonat/AppData/Local/Temp/ff-target`. Fixtures stay byte-identical. A manual verification script (like Phase 22's) covers the real send.

### Claude's Discretion
Window lookup method, worker thread vs inline, delays between steps, exact clear-input keys, module name (e.g. `src/overlay/chat.rs`), exact wording of "not sent" reasons.

### Deferred Ideas (OUT OF SCOPE)
- Reload from `ff-cli apply`.
- Detecting which filter the game has selected / warning on mismatch.
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| RELOAD-01 | After a RULES-tab Apply that writes the filter, the overlay sends `/reloaditemfilter` to a running PoE2 | Sections "Reference overlay behaviour", "Recommended mechanism", "Architecture Patterns" (seam + trigger decision), "Settings" |
</phase_requirements>

## Summary

Both reference overlays (Exiled Exchange 2 and Awakened PoE Trade) send chat text identically: write the text to the system clipboard, tap `Enter` (open chat), optionally `Ctrl+A` (select existing input), tap `Ctrl+V`, tap `Enter` (send). The clipboard's previous *text* is saved first and restored 120 ms later. Key events are real `SendInput` calls carrying both `wVk` and a `wScan` from `MapVirtualKeyW` (libuiohook). Neither does anything clever to focus the game: EE2 calls plain `SetForegroundWindow(target_hwnd)` (via electron-overlay-window). So clipboard paste is the precedent that is known to work in PoE's chat box; `KEYEVENTF_UNICODE` (VK_PACKET) in PoE has no evidence either way, and the one thing it would buy (no clipboard clobber) is cheap to get another way (save/restore with a sequence-number guard).

`SetForegroundWindow` from the overlay should succeed because the user just clicked our window (we are the foreground process at that instant). It can still fail (Windows returns FALSE); the design verifies `GetForegroundWindow() == game_hwnd` before every key batch and aborts with "not sent" otherwise. The one real blocker with no workaround is UIPI: if the game runs elevated and `ff` does not, `SendInput` is silently dropped; EE2 specially detects and tells the user this. We cannot detect it from the SendInput return value, but the foreground verify + a documented caveat covers it.

The `/reloaditemfilter` command is the weak link: it is asserted by the author and by one low-quality third-party page; I found no GGG patch note, wiki page, or reference-overlay source mentioning it. Treat the first manual-verification step as "type `/reloaditemfilter` by hand in PoE2 and confirm the chat shows a reload/OK line". Keep the command string a single `const` so it is a one-line change if it turns out to differ.

**Primary recommendation:** New `src/overlay/chat.rs` that builds a pure `Vec<Step>` (`FocusGame`, `VerifyForeground`, `SetClipboard`, `Tap`, `Chord`, `Wait`, `RestoreClipboard`), executed on a short-lived worker thread by a `WinSink: InputSink`; Win32 helpers (`find_game_window`, `focus`, `send_key`, clipboard) live in `win.rs`. Clipboard paste, `Enter, Ctrl+A, Ctrl+V, Enter`, 120 ms clipboard restore, abort on any failed verification.

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| Trigger decision (Written x setting x game present) | `overlay/app.rs` `run_apply` | `overlay/chat.rs` pure fn | Only place the outcome is resolved; decision kept pure so it is testable |
| Step sequence | `overlay/chat.rs` (pure) | -- | No Win32; unit-testable |
| Raw Win32 (find window, focus, SendInput, clipboard) | `overlay/win.rs` | -- | Project rule: the only raw-Win32 module; SAFETY comments + `cfg(not(windows))` stand-ins |
| Worker thread / result hand-back | `overlay/app.rs` | `chat.rs` | Same pattern as SYNC ECONOMY worker (mpsc + `request_repaint`) |
| Setting persistence | `src/settings.rs` | `app.rs` handler | Save-on-change like `switch_to` |
| Toggle UI | `overlay/config_view.rs` `show_filter` | -- | FILTER section |
| Status text | `overlay/apply.rs` | -- | Holds `success_message`, `RELOAD_INSTRUCTION` |

## Reference overlay behaviour (exact, quoted from source)

Fetched 2026-10-13 via raw.githubusercontent.com (files saved in scratchpad; quoted below).

**Exiled Exchange 2 `main/src/shortcuts/text-box.ts`** [VERIFIED: https://raw.githubusercontent.com/Kvan7/Exiled-Exchange-2/master/main/src/shortcuts/text-box.ts]
```ts
const AUTO_CLEAR = ["#","%","@","$","&","/"];  // Global, Party, Whisper, Trade, Guild, Command
export function typeInChat(text, send, clipboard) {
  clipboard.restoreShortly((clipboard) => {
    const modifiers = process.platform === "darwin" ? [Key.Meta] : [Key.Ctrl];
    ...
    } else {                       // the common branch
      clipboard.writeText(text);
      uIOhook.keyTap(Key.Enter);
      if (!AUTO_CLEAR.includes(text[0])) {
        uIOhook.keyTap(Key.A, modifiers);   // select-all existing input
      }
    }
    uIOhook.keyTap(Key.V, modifiers);       // paste
    if (send) {
      uIOhook.keyTap(Key.Enter);            // send
      // restore the last chat
      uIOhook.keyTap(Key.Enter); uIOhook.keyTap(Key.ArrowUp); uIOhook.keyTap(Key.ArrowUp); uIOhook.keyTap(Key.Escape);
    }
  });
}
```
Awakened PoE Trade's `main/src/shortcuts/text-box.ts` is the same code (the repo's `AUTO_CLEAR` + `keyTap(Key.Enter)` + conditional `Ctrl+A` + `Ctrl+V` + `Enter`) [VERIFIED: https://raw.githubusercontent.com/SnosMe/awakened-poe-trade/master/main/src/shortcuts/text-box.ts, lines 7/35/36/40/43]. Notes:
- Messages beginning with a channel/command character (including `/`) **skip Ctrl+A**: the game treats a leading `/` typed/pasted into the box as starting a new command. For our `/reloaditemfilter` EE2 would therefore send `Enter, Ctrl+V, Enter`. CONTEXT asks us to clear partial text; `Ctrl+A` first is harmless on an empty box and makes the paste replace partial text or a pre-filled `@whisper ` prefix. [ASSUMED] that Ctrl+A+paste yields a clean `/reloaditemfilter` in PoE2; verify manually (matrix row "chat already open with partial text"). If Ctrl+A misbehaves, fall back to EE2's exact `Enter, Ctrl+V, Enter`.
- The "restore the last chat" tail (Enter, Up, Up, Escape) is for a user-convenience feature (re-open previous message); **do not copy it** -- extra keystrokes, and unnecessary for a command.

**Clipboard save/restore** `main/src/shortcuts/HostClipboard.ts` [VERIFIED: https://raw.githubusercontent.com/Kvan7/Exiled-Exchange-2/master/main/src/shortcuts/HostClipboard.ts]: `const RESTORE_AFTER = 120;` `restoreShortly(cb)`: refuses to run at all if a previous restore is pending ("This throttling helps against disconnects from 'Too many actions'"), `saved = clipboard.readText(); cb(); setTimeout(() => clipboard.writeText(saved), 120)`. Text-only save (images/files are lost). No delays between keyTaps.

**Key injection** libuiohook `src/windows/post_event.c` [VERIFIED: https://raw.githubusercontent.com/kwhat/libuiohook/1.3/src/windows/post_event.c]: `INPUT_KEYBOARD`, `ki.wVk = ...; ki.wScan = MapVirtualKeyW(wVk, MAPVK_VK_TO_VSC_EX);` extended-key flag when the high byte of the scan code is set; `KEYEVENTF_SCANCODE` is *not* set (it is commented out); one `SendInput(1, ...)` per event; key tap = down then up. => Send **both wVk and wScan**, no `KEYEVENTF_SCANCODE`, no `KEYEVENTF_UNICODE`.

**Focusing the game** `electron-overlay-window` `src/lib/windows.c` [VERIFIED: https://raw.githubusercontent.com/SnosMe/electron-overlay-window/master/src/lib/windows.c]: `void ow_focus_target() { SetForegroundWindow(target_info.hwnd); }` -- nothing else (no `AttachThreadInput`, no fake Alt press). EE2 `OverlayWindow.ts assertGameActive` calls `OverlayController.focusTarget()` [VERIFIED: https://raw.githubusercontent.com/Kvan7/Exiled-Exchange-2/master/main/src/windowing/OverlayWindow.ts]. That works there because EE2's overlay is the foreground window when the user presses its hotkey. Same situation as ours (user just clicked the stripe).

**Finding the game window:** EE2 attaches **by title** (`OverlayController.attachByTitle(window, title)`; title is a user option, default PoE2 window title) using `GetWindowTextW`-based enumeration -- not by window class [VERIFIED: OverlayWindow.ts `updateOpts(... windowTitle ...)` + windows.c `get_title`]. A web search found no confirmation of a PoE2 window class (`POEWindowClass` is PoE1 folklore) [LOW]; **do not hard-code a class**, match by title as the project already does. EE2 also handles "game running as administrator": shows an error that EE2 needs admin too (OverlayWindow.ts `handleOverlayAttached`).

## Standard Stack

### Core
| Library | Version | Purpose | Why |
|---------|---------|---------|-----|
| windows-sys | 0.61.2 (already a dep; 0.61.2 present in `~/.cargo/registry`) | all Win32 calls | Locked by CONTEXT; already used by `win.rs` |

No new crates. [VERIFIED: ~/.cargo/registry/src/*/windows-sys-0.61.2]

**Cargo.toml change** (`[target.'cfg(windows)'.dependencies]`):
```toml
windows-sys = { version = "0.61", features = [
  "Win32_Foundation", "Win32_Security", "Win32_System_Threading",
  "Win32_UI_WindowsAndMessaging",
  "Win32_UI_Input_KeyboardAndMouse",   # SendInput, INPUT, KEYBDINPUT, VK_*, KEYEVENTF_*, MapVirtualKeyW, GetAsyncKeyState
  "Win32_System_DataExchange",         # OpenClipboard/GetClipboardData/SetClipboardData/EmptyClipboard/CloseClipboard/GetClipboardSequenceNumber
  "Win32_System_Memory",               # GlobalAlloc/GlobalLock/GlobalUnlock/GlobalSize/GMEM_MOVEABLE
  "Win32_System_Ole",                  # CF_UNICODETEXT (const lives in System/Ole/mod.rs)
] }
```
Verified in the 0.61.2 source: feature names exist in its Cargo.toml (`Win32_System_DataExchange`, `Win32_System_Memory`, `Win32_UI_Input_KeyboardAndMouse`); `CF_UNICODETEXT: u16 = 13` is in `System/Ole/mod.rs` (needs `Win32_System_Ole`; alternatively define `const CF_UNICODETEXT: u32 = 13;` locally and skip that feature -- planner's choice, the local const is simpler and safe since the value is a stable ABI constant). `GlobalFree` is in `Win32_Foundation`. `windows-sys` wires `Win32_System_Ole -> Win32_System_Com`... if the feature pulls extra, prefer the local const.

**Exact function / type paths** (all in 0.61.2, `windows_link::link!` style, call inside `unsafe`):
- `windows_sys::Win32::UI::Input::KeyboardAndMouse::{SendInput, INPUT, INPUT_0, KEYBDINPUT, INPUT_KEYBOARD, KEYEVENTF_KEYUP, KEYEVENTF_EXTENDEDKEY, VK_RETURN (13), VK_CONTROL (17), VK_A (65), VK_V (86), VK_SHIFT, VK_MENU, GetAsyncKeyState, MapVirtualKeyW}`. `INPUT { r#type: u32, Anonymous: INPUT_0 }`, union field `ki: KEYBDINPUT { wVk: u16, wScan: u16, dwFlags: u32, time: u32, dwExtraInfo: usize }`. `SendInput(cinputs: u32, pinputs: *const INPUT, cbsize: i32) -> u32`; pass `size_of::<INPUT>() as i32`.
- `windows_sys::Win32::UI::WindowsAndMessaging::{SetForegroundWindow, GetForegroundWindow, EnumWindows (WNDENUMPROC = Option<unsafe extern "system" fn(HWND, LPARAM) -> BOOL>), GetWindowTextW, GetWindowTextLengthW, IsWindowVisible, IsIconic, IsWindow, ShowWindow, SW_RESTORE (9), GetWindowThreadProcessId, BringWindowToTop, AllowSetForegroundWindow}`.
- `windows_sys::Win32::System::DataExchange::{OpenClipboard, CloseClipboard, EmptyClipboard, GetClipboardData, SetClipboardData, GetClipboardSequenceNumber}`; `...::System::Memory::{GlobalAlloc, GlobalLock, GlobalUnlock, GlobalSize, GMEM_MOVEABLE}`; `...::Foundation::GlobalFree`.
- `FindWindowW` is **not** needed (title substring needs `EnumWindows`).

## Package Legitimacy Audit

No new packages. `windows-sys` is already in `Cargo.toml`/`Cargo.lock` (official Microsoft crate); only feature flags change, all verified present in the local 0.61.2 source. slopcheck not run (nothing new to install).

**Packages removed / flagged:** none.

## Architecture Patterns

### System Architecture Diagram

```
 RULES Apply click (egui frame)
        |
        v
 run_apply -> apply_working -> ApplyOutcome
        |
        +-- Failed / Unchanged --> existing message (no send)   [CONFIG Apply never reaches here]
        |
        +-- Written(path)
              |
              v
   chat::decide(outcome, setting_on, game_present) -> Decision
        | Skip(reason)  -> message = success_message + reason   (Refresh reminder kept)
        | Send(hwnd)
              v
   chat::reload_steps() -> Vec<Step>   (pure)
              |
              v  spawn worker thread (moved: hwnd, steps, ctx clone)
   execute(&mut dyn InputSink, steps)  --- Step::FocusGame -> SetForegroundWindow (+ restore if iconic)
        |                              --- Step::VerifyForeground -> GetForegroundWindow()==hwnd else ABORT
        |                              --- Step::SaveClipboard / SetClipboard / RestoreClipboard
        |                              --- Step::Tap(Enter) / Step::Chord(Ctrl, A|V) / Step::Wait(ms)
        v
   mpsc::Sender<ReloadResult{Sent | NotSent(cause)}> + ctx.request_repaint()
              v
   next frame: app drains result -> apply_message = "Wrote <path>. Reloaded in game" | "... <RELOAD_INSTRUCTION> (<cause>)"
```
Message must be updatable after the worker finishes (the frame must not block). Show an interim "Wrote <path>. Reloading in game..." then replace.

### Recommended Project Structure
```
src/overlay/chat.rs   # Step, Key, InputSink trait, reload_steps(), execute(), decide(), message builders, ReloadResult; pure + tests
src/overlay/win.rs    # + find_game_window(own: Option<&OverlayWindow>) -> Option<GameWindow>, WinSink (impl InputSink), cfg(not(windows)) stand-ins
src/overlay/apply.rs  # + reloaded_message / not_sent reasons (keeps RELOAD_INSTRUCTION verbatim)
src/settings.rs       # + reload_after_apply: Option<bool> + accessor
```

### Pattern 1: Pure steps + sink (the testable seam)
```rust
pub const RELOAD_COMMAND: &str = "/reloaditemfilter";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Key { Enter, Ctrl, A, V }

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Step {
    FocusGame,            // SetForegroundWindow (+ SW_RESTORE if minimized), then caller waits
    VerifyForeground,     // abort the whole sequence unless the game is foreground
    VerifyModifiersUp,    // abort if Ctrl/Shift/Alt currently physically held (see Pitfalls)
    SaveClipboard,
    SetClipboard(String),
    Tap(Key),
    Chord(Key, Key),      // modifier, key: down mod, down key, up key, up mod
    Wait(u64),            // ms
    RestoreClipboard,     // only if clipboard sequence number is still ours
}

pub trait InputSink { fn run(&mut self, step: &Step) -> Result<(), String>; }

pub fn reload_steps() -> Vec<Step> {
    use Step::*;
    vec![
        FocusGame, Wait(60), VerifyForeground, VerifyModifiersUp,
        SaveClipboard, SetClipboard(RELOAD_COMMAND.into()),
        Tap(Key::Enter), Wait(60),            // chat opens
        VerifyForeground,                     // re-check right before the paste/send
        Chord(Key::Ctrl, Key::A), Chord(Key::Ctrl, Key::V), Wait(40),
        Tap(Key::Enter),
        Wait(120), RestoreClipboard,          // EE2 RESTORE_AFTER = 120
    ]
}

pub fn execute(sink: &mut dyn InputSink, steps: &[Step]) -> Result<(), String> {
    for s in steps { sink.run(s)?; }   // first Err aborts: nothing further is typed
    Ok(())
}
```
`RestoreClipboard` must run even after an abort mid-sequence (do it in the worker's cleanup, not via the `?` early exit): after `execute` returns Err, call `sink.run(&Step::RestoreClipboard)` once if `SaveClipboard` ran. Tests: a `RecordingSink` collects steps; a `FailingSink` fails at step N and asserts no `Tap(Enter)` follows a failed `VerifyForeground`.

### Pattern 2: Trigger decision (pure)
```rust
pub enum Decision { Send, Skip(SkipReason) }
pub enum SkipReason { SettingOff, GameNotRunning }
pub fn decide(outcome: &ApplyOutcome, setting_on: bool, game_present: bool) -> Option<Decision>
// None for Unchanged/Failed (no statement about reload at all); Some(Send) only for Written && on && present.
```
Truth table to test: 3 outcomes x 2 x 2 = 12 cases; only (Written,true,true) sends; (Written,false,_) -> SettingOff (checked before game lookup, so no window enumeration when off); (Written,true,false) -> GameNotRunning.

### Pattern 3: Win32 sink
- `find_game_window`: `EnumWindows` with an `extern "system"` callback, `lparam` = `*mut Vec<HWND>`/context struct; for each `IsWindowVisible` (minimised windows are still "visible" in this sense; OK) and non-empty title containing `"path of exile"` (lowercased, same rule as `foreground_is_game_or_overlay` -- factor a shared `fn is_game_title(&str) -> bool` and unit-test it with "Path of Exile 2" yes, "Exiled Exchange 2" no, "path of exile" yes), skip our own overlay HWND (our stripe's title may contain... check; exclude by handle) and skip windows owned by our process (`GetWindowThreadProcessId` == `GetCurrentProcessId`). Return first match.
- Known weakness inherited from the existing rule: a browser tab titled "Path of Exile 2 ..." also matches. Mitigations (pick at least the first): (a) require the foreground window to be that same HWND after focusing -- a browser would be focused and the verify passes, so this alone does NOT save you; (b) therefore also prefer windows whose title **starts with** "path of exile" (browsers append " - Google Chrome", and tab titles of GGG pages are "Path of Exile ..." hmm: they would still start with it) [ASSUMED risk is low]; (c) optional exe-name check via `OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION)` + `QueryFullProcessImageNameW` (in `Win32_System_Threading`, already enabled) requiring `PathOfExile*` -- exe names for the Steam/standalone PoE2 clients are [ASSUMED] `PathOfExile.exe` / `PathOfExileSteam.exe` (CLAUDE.md/win.rs mention `PathOfExileSteam`). Recommendation: implement (b)+(c) as "soft" (if the exe name can be read and does not start with `pathofexile` -> not the game; if it cannot be read, fall back to title only). Add as a manual-verification row. Planner may defer (c) if the cost is high; document the browser-tab risk.
- Key send: build 2 `INPUT`s (down, up) per key; `wScan = MapVirtualKeyW(vk, MAPVK_VK_TO_VSC)` (const `MAPVK_VK_TO_VSC = 0`; libuiohook uses `_EX`, either is fine for these keys, none is extended: Enter on the main keyboard, Ctrl-left, A, V). Use VK_CONTROL (generic) as EE2/uiohook does for Ctrl. Send each tap as a single `SendInput` call with the whole array so it is atomic w.r.t. other input; check the returned count equals the array length, else `Err("SendInput blocked")`.
- Chord = [mod down, key down, key up, mod up] in one SendInput call.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Key events | Per-char typing loops | 4-5 `SendInput` calls (Enter, Ctrl+A, Ctrl+V, Enter) | Fewer events = less "too many actions" risk, layout-independent |
| Waiting for the game | Polling loops | 2-3 fixed `Wait`s (bounded, ~350 ms total) | Bounded; no retry loops (ToS) |
| Unicode text path | `KEYEVENTF_UNICODE` typing | Clipboard paste | Reference-overlay precedent; see below |

## Text entry decision: clipboard paste

- **Pick: clipboard paste** (`CF_UNICODETEXT`). Justification: (1) identical to what EE2/APT do in PoE chat for years [VERIFIED above]; (2) VK_PACKET (`KEYEVENTF_UNICODE`) delivers `WM_CHAR` via a synthesized packet; whether PoE's DirectX/custom input handling (it reads raw keys for hotkeys) accepts it is **unverified** -- no source found [LOW]; (3) layout concerns: `/reloaditemfilter` is pure ASCII, but `/` is Shift+7 on some layouts, so typing it as virtual keys would be layout-fragile; the paste path only depends on Ctrl+V / Ctrl+A / Enter which exist on every layout. Keyboard layouts that remap A/V physically (Dvorak/AZERTY): `VK_A`/`VK_V` are virtual keys resolved by the layout, with `wScan` from `MapVirtualKeyW` for the *current* layout, so the OS-reported key is consistent; PoE binds Ctrl+A/Ctrl+V in the text box by virtual key as it does for EE2 users. [ASSUMED]
- Cost: clobbers the clipboard for ~200 ms. Mitigation (better than EE2): `GetClipboardSequenceNumber()` after our `SetClipboardData`; at restore time only write the saved text back **if the sequence number is still ours** (the user/other app did not copy something in the interim). Saved content = `CF_UNICODETEXT` only; if the clipboard held non-text (image/files) we cannot restore it -- document, and in that case still restore nothing (do not empty). Acceptable per EE2 precedent.
- Clipboard API discipline: `OpenClipboard(0)` can fail if another app holds it -> retry a few times (<= 5 x 10 ms, bounded) then `Err("clipboard busy")` (not-sent, no keystrokes). Set: `GlobalAlloc(GMEM_MOVEABLE, (len+1)*2)`, `GlobalLock`, copy UTF-16 + NUL, `GlobalUnlock`, `EmptyClipboard`, `SetClipboardData(CF_UNICODETEXT, h)`; on success the system owns `h` (do NOT free); on failure `GlobalFree(h)`. Always `CloseClipboard`.

## Focus rules (SetForegroundWindow)

[CITED: learn.microsoft.com/windows/win32/api/winuser/nf-winuser-setforegroundwindow -- from training knowledge, not re-fetched this session; treat as MEDIUM] The call succeeds if the calling process is the foreground process (or was started by it, or received the last input event, or the foreground is being debugged/lock timeout expired, etc.). Our stripe was clicked by the user, so our process received the last input event and is foreground -> allowed. Failure returns `FALSE`; the foreground may instead just flash the taskbar button. Therefore:
1. `if IsIconic(hwnd) { ShowWindow(hwnd, SW_RESTORE) }` (game minimized / alt-tabbed windowed). Note `ShowWindow` return is "was visible", not success.
2. `SetForegroundWindow(hwnd)`; treat FALSE as `Err("could not focus the game")`.
3. `Wait(60)`, then **verify** `GetForegroundWindow() == hwnd` (this is the real gate; the BOOL alone is not trusted). Do not use `AttachThreadInput` or a fake Alt tap tricks: EE2 does not, and failing closed ("not sent") is what CONTEXT requires.
4. Because the overlay is `always_on_top`, check it does not re-steal focus: the stripe is created non-activating (Phase 13) and always-on-top only affects z-order, not activation. After focusing the game the stripe stays visible above it, which is intended. Do not call any egui `ViewportCommand::Focus` in the same frame.

## Common Pitfalls

### Pitfall 1: Modifier keys held at send time
**What goes wrong:** user clicks Apply with a physical Ctrl/Shift/Alt held (or Ctrl+Shift+Z hotkey just released): injected Enter becomes Ctrl+Enter, Ctrl+V becomes Ctrl+Shift+V, etc. (EE2 sets `modifiers` explicitly but does not check held keys; its trigger is itself a hotkey.)
**How to avoid:** `Step::VerifyModifiersUp` -> `GetAsyncKeyState(VK_CONTROL/VK_SHIFT/VK_MENU) & 0x8000 != 0` -> brief bounded wait (<= 300 ms in 25 ms slices) then abort with "a modifier key was held". Mouse-click Apply normally has none held. Never inject "key up" for keys the user is physically holding.

### Pitfall 2: Chat already open
Pressing Enter with chat open and text typed *sends* that text (or closes the box if empty). EE2 has the same exposure (their `Enter` first). A text in the box is sent to chat as a message, which cannot be detected from outside. This is why the sequence is not "Enter first, then clear" for an already-open box; there is no reliable detect. Accept and document in the manual matrix ("chat already open with partial text" -> observe), and in the UI caveat caption ("Close the chat box before pressing Apply"). Mitigation option: send `Escape` first (closes an open chat box; harmless in-world, but in PoE2 Escape with no UI opens the game menu!) -- **do not add Escape**. [ASSUMED about Escape opening the menu; avoiding it is the safe choice.]

### Pitfall 3: UIPI / elevated game
`SendInput` into a higher-integrity window is silently dropped (documented: no error). Detect cheaply: after the sequence nothing observable. Document in the setting caption/manual doc ("run ff and the game with the same privileges"), mirroring EE2's admin message. Foreground verify still passes, so this failure is "silent no-op" -> acceptable: the Refresh reminder remains in the message when `Sent`? CONTEXT says "Reloaded in game" on send; keep the reminder out. Accept.

### Pitfall 4: Exclusive fullscreen / game minimized
Exclusive fullscreen: overlay can't be seen, so Apply can't be clicked anyway (CLAUDE.md overlay note). Minimized game: handled by `SW_RESTORE` step; if restore+focus fails -> "not sent".

### Pitfall 5: Blocking the egui frame
Total sequence ~350-500 ms of sleeps. Run on `std::thread::spawn`, return result by `mpsc`; app drains in `update` (same pattern as SYNC ECONOMY). Guard against a second Apply while one is in flight (`reload_in_flight: bool`) -- at most one send per click, never queued/retried (ToS).

### Pitfall 6: `settings_with_filter` drops new fields
`app.rs:570 settings_with_filter` builds `Settings { filter, extra }` from scratch: adding a field to `Settings` is a compile error there (good), but must copy the new field, otherwise switching filters silently resets the toggle. Add a test.

### Pitfall 7: Unchanged Apply after a manual edit
`Unchanged` never sends (decision locked). Fine; consistent with "file not rewritten".

### Pitfall 8: Wrong-filter caveat
`/reloaditemfilter` reloads the game's *selected* filter. If `ff-settings.json` filter != game's selection, "Reloaded in game" is misleading. Message wording: "Reloaded in game (reloads the filter selected in game)". Surface the caveat in the setting's hover text/caption.

## Settings: persistence and toggle

[VERIFIED: src/settings.rs, src/overlay/app.rs, src/overlay/config_view.rs read this session]
- `Settings { filter: Option<String>, #[serde(flatten)] extra }` currently. Add `#[serde(default, skip_serializing_if = "Option::is_none")] pub reload_after_apply: Option<bool>` and `pub fn reload_after_apply(&self) -> bool { self.reload_after_apply.unwrap_or(true) }`. Absent key = ON; `skip_serializing_if` keeps a never-touched file byte-identical; the flatten `extra` still round-trips unknown keys (serde handles a named field + flatten fine; add a round-trip test with an unknown key plus `"reloadAfterApply": false` -- decide casing: existing key is `filter`; existing file uses default snake/lowercase, so use `#[serde(rename = "reloadAfterApply")]` only if other keys in the project are camelCase; otherwise `reload_after_apply`. Planner: check `tests` in settings.rs to match, keep one convention. RESOLVED: snake_case `reload_after_apply`, absent = on (plan 23-01).)
- Save-on-change mirrors `switch_to`: new `ConfigAction::SetReloadAfterApply(bool)`; handler builds `Settings { reload_after_apply: Some(v), ..self.settings.clone() }`, calls `settings::save(root, &next)`, assigns `self.settings = next`, surfaces a save error into `config_message` as `switch_to` does. It must **not** touch `config_changed_pending` / `compute_config_pending` (that tracks CONFIG draft edits for CONFIG Apply; a view-only preference is not part of the Apply/Revert model, consistent with the filter selection being saved on change). Add the `settings_with_filter` copy (Pitfall 6).
- UI: in `show_filter` (config_view.rs ~L276-352), after the combo / pending-switch block and before the final `helper(ui, "Saved on this computer...")`, add `ui.checkbox(&mut on, "Reload filter in game after Apply")` (or the project's circular toggle if config_view has one -- reuse the existing toggle style used for the rule editor), push `ConfigAction::SetReloadAfterApply(on)` when changed, and `.on_hover_text(...)`/`helper(...)` caption: "Sends /reloaditemfilter to the game. It reloads the filter selected in game, so pick the same filter there. Close the chat box first." `ConfigViewCtx` needs a new field `reload_after_apply: bool` (struct is built in app.rs and tests; update both).

## Status text (apply.rs)
- Sent: `format!("Wrote {}. Reloaded in game", path.display())` (CONTEXT wording; keep the `extras` suffix as `run_apply` does).
- Not sent: existing `success_message(&path)` (contains `RELOAD_INSTRUCTION` verbatim; existing tests assert it) + ` (<reason>)` where reason in { "game not running", "auto-reload is off", "could not reload: <cause>" }. Hold wording constants in `apply.rs`/`chat.rs`; unit-test each string.
- Interim while in flight: `"Wrote <path>. Reloading in game..."`, replaced by the worker result next frame.

## Validation Architecture

### Test Framework
| Property | Value |
|----------|-------|
| Framework | Rust built-in `#[test]` (cargo test), plus insta/assert_cmd already present |
| Config file | none |
| Quick run command | `CARGO_TARGET_DIR=C:/Users/jonat/AppData/Local/Temp/ff-target cargo test --lib overlay::chat` |
| Full suite command | `CARGO_TARGET_DIR=C:/Users/jonat/AppData/Local/Temp/ff-target cargo test` |

### Phase Requirements -> Test Map
| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| RELOAD-01 | `reload_steps()` exact sequence (focus, verify, clipboard, Enter, Ctrl+A, Ctrl+V, Enter, restore) | unit | `cargo test --lib overlay::chat::tests::steps` | Wave 0 |
| RELOAD-01 | `decide()` 12-case truth table | unit | `... chat::tests::decide` | Wave 0 |
| RELOAD-01 | Failed verify aborts before any Enter; clipboard restore still runs | unit (fake sink) | `... chat::tests::abort` | Wave 0 |
| RELOAD-01 | Status strings (sent / each skip reason / failure cause; Refresh substring kept) | unit | `... apply::tests` | Wave 0 (extend) |
| RELOAD-01 | Settings default ON, `false` round trip, unknown key survives, `settings_with_filter` keeps toggle | unit | `cargo test --lib settings` | extend |
| RELOAD-01 | `is_game_title`: "Path of Exile 2" yes, "Exiled Exchange 2" no, case-insensitive | unit | `... win::tests` (pure fn outside `cfg(windows)` mod) | Wave 0 |
| RELOAD-01 | Real send in game | manual | 23-MANUAL-VERIFICATION.md matrix (CONTEXT specifics) | Wave 0 |

Sampling: per task `cargo test --lib overlay::`; per wave full suite; `artifact_guard`/`generate_snapshot` must stay green (no fixture changes). Never call `WinSink` from a test.

### Wave 0 Gaps
- [ ] `src/overlay/chat.rs` with tests; [ ] manual verification doc; [ ] Cargo features; no framework install.

## Security Domain

| ASVS Category | Applies | Control |
|---------------|---------|---------|
| V5 Input validation | yes (low) | The injected string is a compile-time const, never user data; no path from settings/UI into the text |
| V4 Access control | n/a | -- |
| V6 Cryptography | no | -- |

| Threat | STRIDE | Mitigation |
|--------|--------|------------|
| Keystrokes land in the wrong app (e.g. password field) | Tampering / Info disclosure | `VerifyForeground` before clipboard-set and again before paste/send; abort on mismatch; text is a const, but Ctrl+A/Enter in another app can still do harm, hence two verifies |
| Clipboard clobber/leak of user's clipboard | Info disclosure | Restore saved text only if sequence number unchanged; saved text held in memory only, dropped after restore, never logged |
| Repeated/automated sending (ToS) | -- | one send per Apply click; `reload_in_flight` guard; no retries/timers |
| Unsafe code | -- | each `unsafe` block carries a SAFETY comment; all in `win.rs` per project rule |

**ToS posture:** GGG's macro policy: one server-side action per key press. One Apply click -> one chat command (Enter/paste/Enter is a single user-initiated action, same class as EE2's `/hideout` and whisper hotkeys). EE2/APT ship exactly this and are in wide use. [ASSUMED: no GGG policy re-read this session; consistent with CONTEXT]

## State of the Art / Evidence for the command

| Claim | Evidence | Confidence |
|-------|----------|------------|
| `/reloaditemfilter` reloads the currently selected filter | Single third-party page (vhpg.com `?p=65156`, "PoE Reload Your Item Filter /reloaditemfilter"; marketing site; now 404 on fetch); the project author states it exists | LOW |
| `/itemfilter <name>` selects a filter | PoETiS README (https://github.com/Essyer/PoETiS) -- PoE1-era tool | LOW (out of scope) |
| Official in-game route is Options > Game > Item Filter > Refresh | esports.gg guide, poewiki | HIGH |

No GGG patch note or wiki confirms the command in PoE2; searches for "reloaditemfilter" returned nothing official. Mitigation: manual-verify step zero; single const; the "not sent" path leaves the Refresh reminder so nothing regresses if the command is invalid (the worst case is a "unknown command" chat line).

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | `/reloaditemfilter` exists in PoE2 and reloads the selected filter | Evidence | Feature is a no-op; user sees chat error. Verify manually first |
| A2 | Ctrl+A then Ctrl+V cleanly replaces chat input starting with a `/` | Reference behaviour | Garbled command; fall back to EE2's Enter, Ctrl+V, Enter |
| A3 | Standalone/Steam PoE2 exe names start with `PathOfExile` | Pattern 3 | Optional exe check wrongly rejects the game -> make the check soft (title fallback) |
| A4 | Escape in PoE2 with no UI open opens the game menu | Pitfall 2 | Minor; we avoid Escape regardless |
| A5 | SetForegroundWindow succeeds for the process that just received the click (MS docs from training) | Focus rules | Falls to "not sent"; fail-closed |
| A6 | PoE accepts VK_PACKET/KEYEVENTF_UNICODE | Text entry | Irrelevant: we chose clipboard |
| A7 | Chat commands from a user-click overlay are ToS-acceptable | Security | Per CONTEXT decision; ships in EE2/APT |
| A8 | Delays 60/60/40/120 ms are enough for the game to react | Pattern 1 | Tune in manual test; constants in one place |

## Open Questions (RESOLVED)

1. **Does `/reloaditemfilter` work in PoE2?** Known: author asserts; weak third-party mention. Unclear: official status. Recommendation: first manual step is typing it by hand; planner should make verification doc step 0 and a `human-verify` checkpoint after implementation. RESOLVED: verified at execution by manual step 0 in 23-MANUAL-VERIFICATION.md (plan 23-04); the command is one const in chat.rs.
2. **Window-title false positives (browser tab titled "Path of Exile 2...").** Recommendation: starts_with + soft exe-name check, or accept and document. RESOLVED: title contains "path of exile" AND process image name starts with "pathofexile"; an unqueryable process is not the game (plan 23-02).
3. **Does PoE2 chat keep a pre-filled prefix after opening with Enter (e.g. last whisper target)?** Recommendation: Ctrl+A handles it; verify in the "partial text" row. RESOLVED: Ctrl+A kept before Ctrl+V, never Escape; manual check in 23-04, EE2 sequence as fallback.
4. **Settings key casing.** Check existing settings tests; keep one convention.

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| Rust toolchain (edition 2024) | build | yes (project builds) | -- | -- |
| windows-sys 0.61.2 features | Win32 calls | yes (local registry) | 0.61.2 | -- |
| Path of Exile 2 | manual verification only | n/a for tests | -- | Human UAT |

## Project Constraints (from CLAUDE.md)
- Update `CHANGELOG.md [Unreleased]`; update CLAUDE.md overlay notes (remove "never re-reads a filter" auto-reload claims; keep "PoE2 does not hot-reload"; CLI `apply` row keeps manual Refresh).
- Win32 only in `win.rs` (+ `cfg(not(windows))` stand-ins), SAFETY comments.
- Generated artifacts (`universal.filter`, etc.) and fixtures untouched; `artifact_guard` stays byte-identical.
- Build with `CARGO_TARGET_DIR=C:/Users/jonat/AppData/Local/Temp/ff-target`; never launch `ff` or send real keys in tests.
- Overlay in windowed/windowed-fullscreen only.

## Sources

### Primary (HIGH)
- https://raw.githubusercontent.com/Kvan7/Exiled-Exchange-2/master/main/src/shortcuts/text-box.ts, `.../shortcuts/HostClipboard.ts`, `.../windowing/OverlayWindow.ts`, `.../windowing/GameWindow.ts`
- https://raw.githubusercontent.com/SnosMe/awakened-poe-trade/master/main/src/shortcuts/text-box.ts
- https://raw.githubusercontent.com/SnosMe/electron-overlay-window/master/src/lib/windows.c and `src/index.ts`
- https://raw.githubusercontent.com/kwhat/libuiohook/1.3/src/windows/post_event.c
- Local `~/.cargo/registry/src/*/windows-sys-0.61.2` (features, signatures, constants)
- Repo: src/overlay/win.rs, app.rs (`run_apply` L780, `switch_to` L743, `settings_with_filter` L570), config_view.rs (`show_filter` L276, `ConfigAction` L38, `ConfigViewCtx` L102), apply.rs, settings.rs

### Secondary / Tertiary (LOW)
- https://www.vhpg.com/?p=65156 (command mention; unfetchable now), https://github.com/Essyer/PoETiS, https://www.poewiki.net/wiki/Loot_filter (UI reload only)

## Metadata
**Confidence:** Standard stack HIGH; mechanism HIGH (precedent) / focus rules MEDIUM (docs from training); command existence LOW; pitfalls MEDIUM.
**Research date:** 2026-10-13. **Valid until:** 2026-11-13 (PoE2 patches may change chat behaviour).
