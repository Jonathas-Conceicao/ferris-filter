# Phase 23: Auto-Reload Filter in Game on Apply - Context

**Gathered:** 2026-10-13
**Status:** Ready for planning
**Mode:** Decisions made on the author's behalf (autonomous); the author gives feedback after use

<domain>
## Phase Boundary

After a RULES-tab Apply that writes the filter, the overlay sends the chat command
`/reloaditemfilter` to a running Path of Exile 2, so the user no longer has to open Options → Game
→ Item Filter → Refresh by hand. Requirement: RELOAD-01. Added to v0.5 after the milestone audit,
at the author's request: "/reloaditemfilter is a command that reloads the lootfilter... add a phase
for adding this on save; you'll need to research how the overlay can send chat commands, but
that's something most PoE2 overlays do."

In scope: the send itself, the trigger, the CONFIG setting, the post-Apply status text, docs.
Out of scope: switching which filter the game has selected, sending any other chat command,
reloading from the CLI (`ff-cli apply` stays as it is).

</domain>

<decisions>
## Implementation Decisions

### Trigger
- Send only after a **RULES-tab Apply whose outcome is `ApplyOutcome::Written`** (the file was
  actually rewritten). Never on CONFIG Apply (it does not write the filter), never on
  `ApplyOutcome::Unchanged` ("already up to date"), never on `ApplyOutcome::Failed` (refused or
  errored Apply).
- Only when a Path of Exile 2 window exists. Reuse the existing title-based detection in
  `src/overlay/win.rs` (`foreground_is_game_or_overlay` matches a window title containing
  "path of exile", case-insensitive; the hotkey gate in `src/overlay/hotkey.rs`/`app.rs` uses it).
  This phase needs a "find the game window" form of the same match (the game is usually *not* the
  foreground window at Apply time, because the user just clicked the stripe), e.g. enumerate
  top-level windows or `FindWindowW` on the game's window class, keeping the title rule
  ("Exiled Exchange 2" must not match). Game not running → nothing is sent; the existing Refresh
  reminder stays.

### Mechanism (expected shape; research confirms and the planner may refine)
- Do what established PoE overlays do. Research the chat-command path of **Exiled Exchange 2**
  (https://github.com/Kvan7/Exiled-Exchange-2) and **Awakened PoE Trade**
  (https://github.com/SnosMe/awakened-poe-trade), and cite the source files.
- Expected sequence: bring the game window to the foreground, Enter to open chat, clear any
  partial text already in the input (as those overlays do, e.g. select-all + delete/overwrite),
  enter `/reloaditemfilter`, Enter to send, and leave focus with the game.
- Text entry: clipboard paste with save/restore, or `SendInput` with `KEYEVENTF_UNICODE` typing.
  Research picks one and justifies it (reference-overlay precedent, robustness against keyboard
  layout, clipboard clobbering, game behaviour).
- Use `windows-sys` (add features such as `Win32_UI_Input_KeyboardAndMouse`,
  `Win32_System_DataExchange`, `Win32_System_Memory` as needed). No new crate unless research
  shows a strong need.
- Focus stealing: `SetForegroundWindow` from the overlay is normally allowed because the user just
  clicked the stripe (the overlay is the foreground process). Research the documented rules and
  what to do if it fails (then report "not sent" rather than typing into the wrong window).
- Before sending keys, verify the foreground window really is the game; if not, abort and report
  the failure. Keystrokes must never land in another application.
- The send must not block the egui frame for long; a short worker thread or a bounded sequence is
  fine. Planner decides.

### ToS posture
- One user click → one chat command. This is the same pattern accepted overlay tools use (they
  send chat commands such as `/hideout` or whispers on a user action). Never repeated, never
  scheduled, never retried in a loop, never sent without that Apply click.

### Setting
- A CONFIG-tab setting **"Reload filter in game after Apply"**, persisted in `ff-settings.json`,
  **default ON** (absent key = on). Settings keeps unknown keys round-tripping (`extra` map); the
  new field must keep that.
- Persist like the filter selection does (saved on change, no CONFIG Apply needed), unless the
  planner finds that conflicts with CONF-04's Apply/Revert model; then follow the existing CONFIG
  draft model. Place it in the FILTER section, following existing CONFIG styles.

### Status after Apply
- On a send: the Apply message says **"Reloaded in game"** (alongside the written path).
- When nothing was sent: keep the existing Refresh reminder (`apply::RELOAD_INSTRUCTION`,
  "Options > Game > Item Filter > Refresh") and add the reason: game not running, setting off, or
  the send failed (with a short cause).

### Caveat to document
- `/reloaditemfilter` reloads the filter the game currently has selected. If the overlay edits a
  different filter than the one selected in game, the reload will not show the edit. Surface this
  in the UI (setting tooltip or caption) and in CLAUDE.md/CHANGELOG. Do not try to switch the
  game's filter.

### Docs
- CLAUDE.md overlay notes: remove the "no auto-reload" claims (e.g. "PoE2 never re-reads a filter
  on its own, so after every successful Apply reload via Options → Game → Item Filter → Refresh").
  "PoE2 does not hot-reload" stays true, but now the overlay can send the command. The CLI `apply`
  row still needs the manual Refresh.
- CHANGELOG.md `[Unreleased]` entry.

### Testing
- Never send real keystrokes in tests and never launch `ff` or touch the user's game. Put input
  sending behind a trait/seam (e.g. an `InputSink` that receives a sequence of abstract steps) and
  unit-test: the step sequence, the trigger decision (Written / Unchanged / Failed × setting ×
  game present), the status text, and settings default/round-trip.
- Build/test with `CARGO_TARGET_DIR=C:/Users/jonat/AppData/Local/Temp/ff-target` (a running ff.exe
  may lock `target\debug`). Fixtures for `artifact_guard`/`generate_snapshot` stay byte-identical.
- A manual verification script (like 22's) covers the real send in game.

### Claude's Discretion
- Window lookup method, worker thread vs inline, delays between steps, exact clear-input keys,
  module name (e.g. `src/overlay/chat.rs`), and the exact wording of the "not sent" reasons.

</decisions>

<code_context>
## Existing Code Insights

### Reusable Assets
- `src/overlay/win.rs`: the only place for raw Win32; `foreground_is_game_or_overlay` (title
  match), `OverlayWindow` (cached stripe HWND). New unsafe Win32 calls belong here or in a sibling
  module with the same discipline (SAFETY comments, `cfg(not(windows))` stand-ins).
- `src/overlay/apply.rs`: `ApplyOutcome` (Written/Unchanged/Failed), `success_message`,
  `unchanged_message`, `RELOAD_INSTRUCTION` (tests assert the Refresh substring verbatim).
- `src/overlay/app.rs` `run_apply`: the single place a RULES Apply resolves its outcome; the hook
  point for the send.
- `src/settings.rs` `Settings` (`filter`, flattened `extra`), `load`/`save`; `switch_to` in
  `app.rs` shows the save-on-change pattern.
- `src/overlay/config_view.rs` `show_filter` (FILTER section), `body::caption`, theme styles.

### Established Patterns
- Phase 13: the stripe is shown without stealing focus. This phase deliberately moves focus to the
  game after Apply, which is where the user wants to be anyway.
- Phase 22: Win32 features added to `windows-sys` in Cargo.toml as needed; in-process tests only.

### Integration Points
- `run_apply` (send after `Written`), CONFIG FILTER section (toggle), `ff-settings.json`.

</code_context>

<specifics>
## Specific Ideas

- Manual test matrix: Apply with game running (windowed), with game not running, with setting
  off, with chat already open and partial text typed, Unchanged Apply, refused Apply, and with a
  different filter selected in game (the caveat).

</specifics>

<deferred>
## Deferred Ideas

- Reload from `ff-cli apply` (a console tool has no user-click context in the game).
- Detecting which filter the game has selected and warning on mismatch.

</deferred>
