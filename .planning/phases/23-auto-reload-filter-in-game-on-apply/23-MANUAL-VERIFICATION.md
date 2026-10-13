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
   **Expected:** status first "Reloading in game...", then "Wrote <path>. Reloaded in game"; the game
   takes focus; the chat shows no stray text; the edit is visible in game without opening Options;
   exactly one command is sent. **Result:**

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

7. **Chat already open with partial text.** In game press Enter, type `hello` (do not send), click
   the stripe, Apply.
   **Expected:** observe and record what happens (assumption A2: Ctrl+A should replace `hello` with
   the command). If `hello` is sent to chat or the command arrives garbled, record it. Fallback: use
   Exiled Exchange 2's exact sequence, Enter, Ctrl+V, Enter (remove `Chord(Ctrl, A)` from
   `reload_steps()`); the CLAUDE.md "close the chat first" note covers the residual case.
   **Result:**

8. **Unchanged Apply.** Apply twice with no edit in between.
   **Expected:** the second shows "already up to date; nothing was written" and nothing is sent.
   **Result:**

9. **Refused Apply.** Create a grammar error in an enabled rule (or edit the file on disk so Apply is
   refused).
   **Expected:** a refusal message, nothing sent, the game is not focused. **Result:**

10. **CONFIG Apply.** Change a tier cutoff and press CONFIG Apply (only with `FERRISFILTER_HOME` at a
    temp copy).
    **Expected:** nothing is sent to the game. **Result:**

11. **Different filter selected in game (the caveat).** Select another filter in Options -> Game ->
    Item Filter, then Apply an edit in the overlay.
    **Expected:** "Reloaded in game" shows, but the edit does NOT appear in game (the game reloaded
    its selected filter). This confirms the documented caveat. Restore the selection. **Result:**

12. **Modifier held.** Hold Shift while clicking Apply and keep holding for a second.
    **Expected:** the Refresh reminder plus "could not reload in game: a modifier key (Ctrl, Shift or
    Alt) was held down"; nothing typed. **Result:**

13. **Game minimized.** Minimize PoE2 (windowed), Apply with an edit.
    **Expected:** the game is restored and reloaded, or the Refresh reminder with a "could not reload
    in game" cause; never keystrokes into another window. **Result:**

14. **Hotkey gate unchanged.** **Expected:** Ctrl+Shift+Z still toggles the stripe only while PoE2 or
    the stripe is the foreground window. **Result:**

15. **Timing (A8).** If any step drops keys (chat opens but no command arrives), note it; the delays
    are the four consts in `src/overlay/chat.rs` (`FOCUS_SETTLE_MS`, `CHAT_OPEN_MS`,
    `PASTE_SETTLE_MS`, `RESTORE_AFTER_MS`). **Expected:** no dropped keys. **Result:**
