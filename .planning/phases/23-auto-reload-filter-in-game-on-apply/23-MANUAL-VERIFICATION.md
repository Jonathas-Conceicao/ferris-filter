# Phase 23 - Auto-Reload Filter in Game on Apply: Manual Verification

**Purpose:** the checks headless tests cannot see: the real keystroke send, game focus, chat-box
behaviour and clipboard handling. Requirement RELOAD-01 stays "Complete (UAT pending)" until this
script has been run.

**Before you start (do not skip):**
- Use a release build: `CARGO_TARGET_DIR=C:/Users/jonat/AppData/Local/Temp/ff-target cargo build --release --bin ff`,
  then launch `ff.exe` from that target dir's `release` folder.
- Path of Exile 2 in windowed or windowed-fullscreen mode (see `13-MANUAL-VERIFICATION.md`).
- The overlay's selected filter (CONFIG -> FILTER) is the same filter selected in Options -> Game ->
  Item Filter.
- `ff` and the game run at the same privilege level (both normal, or both elevated).
- Development warning (CLAUDE.md): a CONFIG Apply from a repo-root `ff` rewrites the committed
  `styles.json` / `tiers.json`. Do not press CONFIG Apply in check 10 unless `FERRISFILTER_HOME`
  points at a temp copy.

Record Result as PASS / FAIL with a note for every step.

---

0. **Command exists.** In game, open chat and type `/reloaditemfilter` by hand.
   **Expected:** the game reloads the filter (a chat/system line or a visible restyle). If the game
   reports an unknown command, stop: the feature cannot work; report the exact message (the command
   is the single const `RELOAD_COMMAND` in `src/overlay/chat.rs`). **Result:**

1. **Game running, setting on.** Make a visible rule edit (for example a style on a common base),
   then RULES Apply.
   **Expected:** status first "Reloading in game..."; the game takes focus and the stripe hides (the
   usual click-away rule); the chat shows no stray text; the edit is visible in game without
   opening Options; exactly one command is sent. Press Ctrl+Shift+Z: the stripe shows
   "Wrote <path>. Reloaded in game" (the result is kept for the next reveal). Note that this status
   means the keys were sent, not that the game confirmed the reload. **Result:**

2. **Clipboard restored.** Copy some text elsewhere, repeat check 1, then paste into Notepad.
   **Expected:** the original text, not `/reloaditemfilter`. **Result:**

3. **Clipboard with an image (optional).** Copy an image, then Apply.
   **Expected:** reload works; the clipboard is now empty (documented trade-off). **Result:**

4. **Game not running.** Close PoE2, Apply with an edit.
   **Expected:** the Refresh reminder plus "(game not running)"; nothing typed anywhere. **Result:**

5. **Browser tab.** With PoE2 closed, open a browser tab whose title contains "Path of Exile 2"
   (for example poe2db.tw), then Apply.
   **Expected:** "(game not running)"; nothing typed into the browser. **Result:**

6. **Setting off.** CONFIG -> FILTER, switch **Reload filter in game after Apply** off; Apply with an
   edit.
   **Expected:** the Refresh reminder plus "(reload setting is off)"; `ff-settings.json` holds
   `"reload_after_apply": false`; after relaunching ff the toggle is still off. Switch it back on.
   **Result:**

7. **Chat already open with partial text (known limitation).** Use a private channel or a party of
   one, since the text is really sent. In game press Enter, type `hello` (do not send), click the
   stripe, Apply.
   **Expected (documented, not a pass/fail on the code):** the first Enter sends `hello` as a chat
   message, the chat box closes, the final Enter leaves an empty chat box open, and
   `/reloaditemfilter` is not sent; the status still says "Reloaded in game" (the keys were
   delivered). Nothing outside the game can see whether chat is open, so this is the reason for the
   "Close the chat box before pressing Apply" caption; no key can help because anything sent runs after the
   first Enter. Record anything else (for example the command arriving garbled). Also record
   whether Ctrl+V does anything in the game world with chat closed. **Result:**

7b. **Chat pre-fill.** With chat closed, Apply after having last chatted in a party or guild
   channel (the game may pre-fill a channel prefix when chat opens).
   **Expected:** the typed `/reloaditemfilter` overrides the pre-filled prefix and the command is sent as is (there is no Ctrl+A since the fix for another tool's Ctrl+A hotkey). **Result:**

8. **Identical Apply.** Apply twice with no edit in between.
   **Expected:** the second still counts as applied: the same success status as a write
   (`Reloaded in game`), the command is sent once, and the file is not rewritten (its modified time
   does not change).
   **Result:**

9. **Refused Apply.** Create a grammar error in an enabled rule (or edit the file on disk so Apply is
   refused).
   **Expected:** a refusal message, nothing sent, the game is not focused. For the
   changed-on-disk case the footer reads "Filter changed on disk since it was loaded." with RELOAD and
   OVERWRITE; press OVERWRITE: the file now holds the overlay's rules, the warning clears and the
   command is sent once, like any Apply. **Result:**

10. **CONFIG Apply.** Change a tier cutoff and press CONFIG Apply (only with `FERRISFILTER_HOME` at a
    temp copy).
    **Expected:** nothing is sent to the game. **Result:**

11. **Different filter selected in game (the caveat).** Select another filter in Options -> Game ->
    Item Filter, then Apply an edit in the overlay.
    **Expected:** "Reloaded in game" shows, but the edit does NOT appear in game (the game reloaded
    its selected filter). This confirms the documented caveat. Restore the selection. **Result:**

12. **Modifier held.** Hold Shift while clicking Apply and keep holding for a second.
    **Expected:** the Refresh reminder plus "could not reload in game: a modifier key (Ctrl, Shift or
    Alt) was held down"; nothing typed; the game is not focused, so the stripe stays open and shows
    the reason at once (the modifier check runs before the focus change). **Result:**

12b. **Modifier pressed after the game takes focus.** Click Apply with no key held, then press and
    hold Alt (or Ctrl) the moment the game comes to the front, and keep holding for a second.
    **Expected:** no Alt+Enter (the game does not toggle fullscreen), no Ctrl+Alt+V,
    and a Ctrl you are holding is not released by the overlay. Depending on timing, either the
    reload completes ("Reloaded in game") because the keys went out before the press, or it stops
    before the next key group; the next Ctrl+Shift+Z then shows the Refresh reminder plus "could not
    reload in game: a modifier key (Ctrl, Shift or Alt) was held down", and the clipboard is
    restored. An immediate modifier check runs before the first Enter, before Ctrl+V and
    before the final Enter (review iteration 2, WR-01). **Result:**

13. **Game minimized.** Minimize PoE2 (windowed), Apply with an edit.
    **Expected:** the game is restored and reloaded, or the Refresh reminder with a "could not reload
    in game" cause; never keystrokes into another window. **Result:**

14. **Hotkey gate unchanged.** **Expected:** Ctrl+Shift+Z still toggles the stripe only while PoE2 or
    the stripe is the foreground window. **Result:**

15. **Timing (A8).** If any step drops keys (chat opens but no command arrives), note it; the delays
    are the four consts in `src/overlay/chat.rs` (`FOCUS_SETTLE_MS`, `CHAT_OPEN_MS`,
    `PASTE_SETTLE_MS`, `RESTORE_AFTER_MS`). **Expected:** no dropped keys. **Result:**

16. **Elevated game (optional, documented limitation).** Run PoE2 as administrator and `ff` normally,
    then Apply with an edit.
    **Expected:** Windows silently drops the keys (UIPI), so nothing changes in game, while the
    status may still read "Reloaded in game" (ff cannot detect this; the CONFIG tooltip and CLAUDE.md
    say so). Record what the status says. **Result:**
