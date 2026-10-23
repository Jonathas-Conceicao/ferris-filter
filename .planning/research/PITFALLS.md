# Domain Pitfalls

**Domain:** Adding v0.7.2 features (style alpha, Auto Detect styles, drag-and-drop reorder, Social-panel headers, search icons, custom hotkey, economy tier price range, request throttling) to FerrisFilter (Rust, egui/eframe 0.36 overlay, lossless filter editing)
**Researched:** 2026-10-23
**Overall confidence:** MEDIUM. The invariants come from CLAUDE.md (HIGH). A few code facts were checked in `src/` (hotkey.rs, emitter.rs, app.rs). egui drag-and-drop and Windows hotkey behaviour are from training knowledge plus code reading and were not re-verified against current docs (MEDIUM/LOW, flagged per item).

**New dependencies:** none of the prevention steps below need one. Items that look like they might are flagged `DEP?` for the author's review.

Phase labels used below (the roadmap should map them): **P-ALPHA** (style alpha), **P-AUTO** (Auto Detect), **P-DND** (drag and drop), **P-UI** (section headers, search icons), **P-HOTKEY**, **P-ECON** (tier price range), **P-THROTTLE**.

---

## Critical Pitfalls

### Pitfall 1: Alpha support changes bytes of existing `styles.json`, fixtures or generated filters
**What goes wrong:** Colours are `[r,g,b]`-style today, and the filter always emits alpha 255 (CLAUDE.md: "styles store RGB and the filter always emits alpha 255"). Adding alpha tends to (a) change the serde shape of the colour field so old files re-serialise differently (`"#ff79c6"` becomes `"#ff79c6ff"`, or a 3-array becomes a 4-array), (b) make the emitter write `SetTextColor R G B 255` where `generate` today writes the 3-value form (or the reverse), and (c) change the economy tier blocks, which then diverge from `tests/fixtures/seed-universal.filter` and break `artifact_guard`.
**Why it happens:** One shared colour type is used by `styles.json`, `generate`, the economy emitter and the styled-rule rewrite ("all emit through the one action emitter"). Any change to that type's `Serialize` or `Display` hits all four.
**Consequences:** `artifact_guard` fails, `styles.json` gets rewritten on the first Apply for every user (a spurious edit, a changed-on-disk warning), and every styled rule is rewritten on Apply, which makes it look like unapplied changes.
**Prevention:**
- Make alpha `Option<u8>` (or "255 means absent") with `skip_serializing_if`, so it is written only when not 255 (mirror the existing "new fields are written only when set" rule for `positional` / `enableDropSound`).
- Decide once: alpha 255 and absent are the same value and normalise on load, not on write. Keep the filter emitter at exactly today's output when alpha is 255, including the background `240` default, so `SetBackgroundColor 40 42 54 255` (Dracula) and the bronze `175 96 37 240` stay byte-identical.
- Write the byte-identical round-trip test first: load `styles-default.json` and `tests/fixtures/seed-styles.json`, re-serialise, assert equality. Run `artifact_guard` before touching any UI.
- Note the bronze unique background is `240`, not 255. If alpha is exposed for tier styles only, the unique block must not inherit a style alpha by accident.
- A Hide style's colours are emitted too. Check that alpha is emitted there with the same rule.
**Detection:** `artifact_guard` diff; `styles.json` showing as edited right after opening the overlay; "Updated N styled rules" with no edits.
**Phase:** P-ALPHA (first, since P-AUTO's equality depends on the final colour type).

### Pitfall 2: Rules with a 3-value colour vs. a style with alpha 255 are "equal" for Auto Detect but not byte-equal on rewrite
**What goes wrong:** A rule line `SetTextColor 255 121 198` (3-value, alpha 255 implied) and `... 255` (4-value) are the same colour. Auto Detect matches both to style `S`, tags the rule `# ff:style builtin:S S`. The next Apply then rewrites the rule's action lines from the style. That is the intended contract of a styled rule, but the bytes change (line order, 3- to 4-value, dropped trailing comments, volume form, an extra `EnableDropSound`).
**Why it happens:** CLAUDE.md says a styled rule's actions are "a hand edit ... overwritten on the next Apply", and "A 3-value colour stays 3-value while alpha is 255" only applies to the Custom editor. Detection and rewrite are two separate steps with different notions of equality.
**Consequences:** The user presses Auto Detect expecting a relabel, and Apply silently reorders lines, drops `# comment`s and changes unrelated lines. If the detected style round-trips differently in game (see Pitfall 3), the rule looks different in game.
**Prevention:**
- Define the detection's equality as "the styled rewrite of this style produces the same effective actions", and implement it by actually running the emitter for the candidate style, parsing the effective actions (last line of each family), and comparing. Do not hand-write a second comparator.
- Decide and document whether a rule whose bytes would change on rewrite counts as a match. Recommended: match on effective actions (that is the point of the feature), but surface "N rules will be rewritten" in the status line, and keep the `.ffbak` guarantee as the safety net.
- Preserve trailing comments? The styled rewrite does not today. Detect rules with a trailing comment on an action line and either skip them or say so. Never silently drop user text.
- Add a test: detect, apply, reload, detect again must produce zero changes (idempotent), and the filter's effective actions per rule are unchanged.
**Detection:** diff of filter text before/after Auto Detect + Apply on a NeverSink-sized sample.
**Phase:** P-AUTO.

### Pitfall 3: Equality traps in the Rule-actions to Style mapping
**What goes wrong:** Each of these makes two rules look the same when the game treats them differently, or looks different when the game treats them the same.
| Trap | Concrete risk | Prevention |
|------|---------------|------------|
| Font size absent vs default | Style `C` has no font override; a rule with `SetFontSize 32` (the game default, per player experience) is not "no override". | Treat absent and an explicit value as different. Never infer a default. |
| Alpha 255 vs absent | 3-value and 4-value-255 colour are equal (Pitfall 2). Background default alpha is 240 in game, so `SetBackgroundColor r g b` (no alpha) is 240, not 255. | Compare effective alpha: background absent-alpha = 240, text/border = 255. Verify the background default against the game and CLAUDE.md before coding (`default alpha 240`). |
| `PlayAlertSound` vs `PlayAlertSoundPositional` | Different keyword, different game behaviour. Style `positional: true` exists only for built-in alerts and is refused with `custom`. | Keyword is part of the equality. |
| Volume omitted vs explicit | `PlayAlertSound 6` (volume 50 default) vs `PlayAlertSound 6 300`. | Normalise to the game default for comparison only (50 for alert, 100 for custom), but treat as a different style when values differ. |
| Cancel values | `MinimapIcon -1`, `PlayEffect None`, `PlayAlertSound None` are `"None"` in styles, versus "line absent". A rule with the cancel value is not equal to a style with null, and the reverse. | Map them explicitly to the `"None"` variant, not to null. `MinimapIcon -1 <colour> <shape>` and `PlayAlertSound None <volume>` are shown by the editor as "nearest honest state" and are inexpressible exactly: leave those rules Custom. |
| Sound file case / extension | `CustomAlertSound "Uwu.mp3"` vs `"uwu.mp3"` (Windows file names are case-insensitive, the filter's matching may not be). | Compare byte-exact; do not case-fold. A near-miss becomes a new custom style, which is safe. |
| `CustomAlertSoundOptional` | The overlay never writes it. | Inexpressible, leave Custom. |
| `DisableDropSound` / `EnableDropSound` / `...IfAlertSound` | Style has both fields, and `IfAlertSound` keywords are not expressible. | Presence of an unexpressible line, or a family with a non-last duplicate line, makes the rule inexpressible. Check the "earlier lines of a family" rows. |
| Duplicate lines in a family | The last line wins in game. | Compare effective (last) values, but if an earlier line is a different kind of value (e.g. earlier `PlayEffect Red`), the rewrite would delete it. Visible change, flag it. |
| Show/Hide plus `Continue` | `Continue` is a rule property, not a style property, and is last in the body. | Exclude from the comparison and keep it on rewrite. Verify the styled rewrite preserves it. |
| Hide with visuals | `hidden` has `Hide` plus font 21. A `Hide` rule with no actions is not `hidden` unless font 21 and DisableDropSound match. | Compare the whole Style struct including `action`. |
| `Minimal` header | Offered nowhere. | Inexpressible, leave Custom. |
**Consequence of wrong equality:** a rule is tagged with a style that looks different in game (silent regression), or ten hundred near-duplicate "Custom Style N" entries are created.
**Phase:** P-AUTO. Put these in a table-driven unit test, one row per trap.

### Pitfall 4: "Custom Style N" naming and id collisions
**What goes wrong:** `styles.json` rejects duplicate names, names differing by case from a required one, and duplicate ids; custom ids are random 6-8 chars and must not start with `builtin:`. A naive `format!("Custom Style {}", n)` using a count collides with an existing style (user renamed one to "Custom Style 3", or deleted style 2 and the count is now stale), or collides within one Auto Detect run (two new styles minted before either is inserted).
**Prevention:**
- Pick N as the smallest positive integer such that the name is free (case-insensitive) against all current styles, including pending edits and the styles minted earlier in the same run.
- Mint ids with the existing generator (the one used by + ADD STYLE) against the pending set, never a second generator.
- Two rules with identical expressible actions should share one new style (dedupe on the Style struct within the run). Otherwise an NeverSink sample produces a style per rule.
- `generate` / `config.json` reference styles by name. A new name cannot break them, but a name must not shadow `Recycle`, `kite`, `exceptional`.
- Auto Detect must run against the pending (unapplied) style set, per the brief ("pending values"). A built-in the user is mid-editing matches by its pending values. The tag it writes uses the id, which is stable.
- Cap and warn: a huge filter may produce hundreds of styles. Decide a limit, or show the count before creating.
**Detection:** `styles.json` load error after Apply ("duplicate name"); a second Auto Detect creating "Custom Style 1" again.
**Phase:** P-AUTO.

### Pitfall 5: Auto Detect's state model vs Revert / Reload / Quit indicator / strictness row
**What goes wrong:**
- It changes two things at once: new styles (Config state) and tags in rules (Rules state). If the new styles live in Config edits and the tags in Rules edits, **Revert on Rules** (discards rules edits) leaves orphan custom styles, and **Revert on Config** removes styles that tagged rules still reference (rules then become `unknown style`).
- The Rules tab "style picker and tag checks follow the unapplied CONFIG style set." Revert on the Config tab drops styles; the reference check must then flag the rules, not crash.
- Reload re-reads the filter and discards rule edits but not config edits (and the reverse for a changed `styles.json`). Same orphan problem.
- Both tabs show ` *`. The Quit popup and "Unapplied changes" indicator must count Auto Detect as pending even if no style text changed (only tags changed).
- Tag-only migration deliberately does **not** mark the filter unapplied (old `# ff:style <name>` to id form). Auto Detect tagging must not reuse that silent path.
- The strictness row derives the level from the S-F visibility. A new custom style does not touch built-ins, so it must not change the derived level. But if Auto Detect "matches" a built-in with pending edits, it is the pending values that count, and Reset to defaults can then make those tagged rules look different. That is expected, but it needs a test.
- **Reset to defaults** keeps custom styles after the built-ins (one pending edit). Detected customs survive it. Fine, but the rules tagged to a built-in will be rewritten with reset values on Apply. Intended, document in a tooltip only (the UI is "not texty").
**Prevention:** Make Auto Detect one atomic "pending edit" whose undo is a single step across both models, or, simpler and recommended, make Revert on either tab (always behind the confirm popup) revert both when an Auto Detect result is pending. Alternative: show Auto Detect only when the filter is applied and Config has no edits, and disable it otherwise. Pick one and test the matrix {Revert Rules, Revert Config, Reload, Reload after file changed, Quit} x {before, after Auto Detect}.
**Also:** Apply must validate all-or-none. If writing `styles.json` succeeds and the filter write fails, the filter references ids that exist, which is fine. The reverse order would leave unknown styles. The current order (styles, tiers, then filter) is correct; do not change it.
**Detection:** unknown-style flags after Revert; a ` *` that does not go away; Quit popup not appearing.
**Phase:** P-AUTO; the status-line / indicator wiring should be part of the same phase.

### Pitfall 6: Disabled rules, groups and tagged rules in Auto Detect
**What goes wrong:** The brief includes disabled rules but groups "once" and skips tagged ones. Pitfalls: (a) a `# ff:group` run is detected per block, and the blocks drift apart (the group's blocks differ only in their defence lines, so the group is one rule; apply the tag to all copies via the existing group edit path, not per block); (b) a disabled commented-out block that is not valid grammar (`can't enable`) has no parsed actions: skip it; (c) economy-section blocks are read-only and the model refuses actions on them, so Auto Detect must iterate user rules only; (d) tagging a disabled rule changes its bytes while disabled (`# ` prefixes): use the existing block edit functions, never string surgery; (e) a rule with `unknown style` tag counts as tagged (skipped).
**Prevention:** Drive everything through the existing rule edit API that keeps bytes untouched except the edited lines, plus the same "styled rule" action writer used by the preset picker. Test on a NeverSink-size sample for time (detection must be O(rules x styles)) and for disabled and grouped rules.
**Phase:** P-AUTO.

### Pitfall 7: Drag-and-drop in a virtualized egui list: dragged row leaves the visible range
**What goes wrong:** The list is virtualized (rows are only built when in the viewport, around ~500 blocks). The row being dragged is no longer built once it scrolls out of view (wheel or edge auto-scroll), so its drag interaction ends, `drag_stopped` never fires, or the payload is dropped, leaving the app in a stuck "dragging" state, or the drop lands on the wrong row.
**Why it happens:** egui interaction state is keyed by `Id` and by the widget's presence in the previous frame. A widget that is not rendered cannot report `dragged()`/`drag_stopped()`.
**Prevention:**
- Own the drag state in the list state, not in widgets: `drag: Option<{source_index/rule id, grab_offset, start_pos}>`. Start it from the handle's `drag_started()`, but end it from the **pointer** (`ctx.input(|i| i.pointer.any_released())` / `!i.pointer.primary_down()`), not from the row's response.
- Identify the source by stable key (rule index plus a generation, or the block id), not by a `Ui` response, and recompute the drop slot from the pointer's y in list coordinates each frame (using the same row-height / cumulative-height table the virtualization uses), not by hit-testing widgets (widgets for far rows do not exist).
- Rows have two heights (collapsed two-line vs expanded editor, and section header rows at 36pt). The slot computation needs the real cumulative heights. Collapse the expanded row (or refuse to drag an expanded row) at drag start so heights are stable.
- Escape cancels the drag and releases it (also when the window loses focus or is hidden via hotkey/tray mid-drag, or a popup opens, e.g. the File-changed-on-disk popup). Reset on `Event::WindowFocused(false)`.
- Draw the shadow and drop marker on a `Foreground`/`Order::Tooltip` layer via `ui.painter().with_layer_id` so it is not clipped by the scroll area's clip rect, and the shadow is not part of the layout.
- Pixel snapping: the skin paints with pixel-snapped nine-slice at fractional DPI (100-200%). Snap the shadow position or it will shimmer.
**Detection:** drag, then wheel far; release outside the window; release over the economy rows; row "sticks" to the cursor after release.
**Phase:** P-DND. Needs a spike: build the drag state machine as pure logic (slot from y, auto-scroll velocity) with unit tests first, then wire egui. Confidence on egui specifics: LOW-MEDIUM, verify with an egui 0.36 `Context::drag_and_drop` / `DragAndDrop` check (may already be adequate; **do not add `egui_dnd`, that is a new dependency (`DEP?`)**).

### Pitfall 8: Drag-and-drop: invalid drop targets and structure invariants
**What goes wrong:** Reordering is not a flat list. The model has rules, `# ff:group` runs (one rule, moves as one unit), section banners (headers move past rules one at a time, the divider below the economy section cannot move down), the locked economy section (always last, read-only, rows muted), rules above the first banner, collapsed sections (hidden rows), and "moved across Continue" flags (existing behaviour that drag must keep). Typical bugs: dropping a rule into or below the economy section (corrupts the file shape, economy "begin-only, always last"); dropping a rule between the blocks of a group (breaks the group; the checks make it show as `group broken`); dropping into a collapsed section (the rule vanishes); dragging a section header drags its rules (arrows moved only the divider); off-by-one when the source index is above the target (remove-then-insert shifts the index); search active (arrows were disabled while searching, the visible list is filtered, so slots do not map to file positions).
**Prevention:**
- Express a drop as a single call into the same model function the arrows use (`move rule from i to slot s`), so every existing invariant, test and the "moved across Continue" flag are reused. Do not write a second mover.
- Compute slots over "rule-level" items, treat a group as one item, and forbid slots inside the economy section and between group members.
- Disable drag while searching (same as the arrows were), and say so in the handle's tooltip.
- Define what dropping on a collapsed section header does (recommended: insert as the section's first rule and leave it collapsed, or refuse, but pick one).
- Custom styles reorder: the same reorder must keep built-ins above customs ("a custom style never moves above the built-ins"), and the order is the `styles.json` order saved by Apply and undone by Revert.
- Reordering rule rows must be one undo step (Revert only, there is no other undo) and must mark the tab ` *` and the unapplied indicator.
**Phase:** P-DND.

### Pitfall 9: Removing the arrows removes the only keyboard path
**What goes wrong:** The reorder arrows were the only way to reorder without a mouse (and a precise way when the list is long). Drag-only makes long-distance moves painful (500 rules, auto-scroll speed too high or too low) and removes accessibility. Auto-scroll tuned on a 60 Hz test machine overshoots on 144 Hz because it is per-frame, not per-second.
**Prevention:**
- Make auto-scroll velocity time-based (`dt`), proportional to the distance into the edge zone, and clamp. Test at high and low frame rates (egui only repaints on input unless `request_repaint` is called while dragging, so auto-scroll stalls when the pointer is held still at the edge: request repaint every frame while a drag is active).
- Keep a keyboard route: either keep the arrows in a hidden form (focus-only, or Alt+Up / Alt+Down on a focused row), or accept the loss deliberately. If the author wants the arrows gone (backlog 999.26), record the decision. Flag for author review rather than deciding silently.
- Hit area of the handle: the custom-art handle must be at least the previous arrows' target size, and it needs a hover and pressed art state (the untracked `assets/custom/icon_reoder_*.png` files suggest three states: normal, hover, pressed). Cursor icon `Grab`/`Grabbing`.
- Clicking the handle must not expand/collapse the row, and a drag must not toggle the round check or select the row (click vs drag threshold; egui's `Sense::click_and_drag` decides click on release only if no drag).
**Phase:** P-DND.

### Pitfall 10: Custom hotkey registration: capture, conflicts and re-registration
**What goes wrong (verified in `src/overlay/hotkey.rs`):** the current code registers once at start-up with constants `TOGGLE_MODIFIERS`/`TOGGLE_CODE`, keeps the manager for process lifetime, and `release_toggle` unregisters the same constants. Making it runtime-configurable breaks assumptions in several places (`app.rs`, `instance.rs`, `mod.rs` also reference the constants or the `Ctrl+Shift+Z` text).
- **Unregister the wrong key:** `release_toggle` and any unregister must use the currently registered `HotKey`, not the constant, or the old key stays registered (and a relaunch after Quit cannot take it, the exact WR-01 bug).
- **Register-new-before-unregister-old:** if the new key fails (taken by another app), the user must keep the old one. Register new first, then unregister old only on success, and save to `ff-settings.json` only on success. Order matters; an unregister-first design leaves the user with no hotkey.
- **Same key re-registered:** re-registering the already-registered chord fails with an "already registered" error on the same manager. Treat "new == current" as a no-op.
- **Thread affinity:** the module doc says `register_toggle` must run on the thread with the Win32 message loop (the hotkey window). The capture UI runs inside `update`, which is on the main thread, so it is OK, but any move of registration to a worker or the settings save path would break it (hotkey events silently never arrive).
- **Hotkey firing while capturing:** while the user is recording a chord, pressing the *current* hotkey (e.g. the default Ctrl+Shift+Z) toggles/hides the window mid-capture. Unregister the current hotkey for the duration of capture (and re-register on cancel / failure), or ignore toggle events while capturing.
- **Capturing from egui key events:** egui's `Key` set differs from `global_hotkey::hotkey::Code` (physical codes). Capture physical keys (`KeyboardEvent` with `physical_key`) or map explicitly; otherwise AZERTY/QWERTZ users register a different key than they pressed. Numpad keys, Fn row beyond F12, media keys have no egui `Key`. (LOW confidence on what egui 0.36 exposes: check `egui::Event::Key { physical_key }`.)
- **Modifier-only chords and bare keys:** reject a modifier-only chord (register fails or blocks that modifier everywhere). A bare key with no modifier (e.g. `Z`) was already shown to hurt (the Phase 13 note: plain `Shift+Z` swallowed typing everywhere, including the game chat). Require at least one modifier (Ctrl/Alt/Shift/Win) plus a non-modifier key, and warn or refuse Shift-only or bare letter/digit/space chords. `Win+...` chords are mostly reserved by Windows (registration fails).
- **Conflicts with the game or its tooling:** PoE2 uses many keys; the overlay's hotkey only acts when the game or overlay is foreground, but registration is system-wide, so the key is lost everywhere. The text should say so in the tooltip (not a wall of text).
- **Foreground-only gating (existing):** the handler checks the foreground window, so it still has to work after re-registration: the event `id` changes with the chord (`hotkey.id()` is derived from the chord). The event loop compares against a stored `Registered { id }`, which must be updated on re-registration, otherwise the new key fires but is ignored.
- **Debug vs release instance:** both can run side by side and conflict on the same chord (existing tray-tooltip note). A saved custom hotkey is shared through `ff-settings.json` if both use the same `FERRISFILTER_HOME`, so debug and release will fight. Not a bug, but the status must say which failed. Tests and `--screenshot` mode must not register a hotkey at all, and must not read or write the real `ff-settings.json` (isolation_guard).
- **Settings robustness:** invalid saved value (bad token, modifier-only) must fall back to the default with a warning (follow the existing "unusable key → warning, default" pattern), never to "no hotkey".
- **Strings that mention the chord:** the tray tooltip, About text, README, `CLAUDE.md` ("hardcoded `Ctrl+Shift+Z`"), failure messages (`HotkeyStatus::Failed`), and `instance.rs` text all hardcode it. Derive from the configured value.
- **Persistence format:** store a stable, human-readable string (`"Ctrl+Shift+Z"`), parsed with `HotKey::from_str` if 0.8 provides it (verify; if not, a small parser is needed, which is not a new dependency but is new code with tests, `DEP?` no). Unknown keys in `ff-settings.json` are preserved on save (existing rule).
- **Tray Quit ordering:** the release order is hotkey, tray icon, instance name. Re-registration must not leave a registered hotkey after Quit begins (disable capture and re-registration once quitting).
**Phase:** P-HOTKEY.

### Pitfall 11: Throttling clock problems
**What goes wrong:** "Once per hour per kind, persisted across restarts" uses wall-clock timestamps in `ff-settings.json` (or a sibling file). The existing code already uses `SystemTime` for display (`app.rs:3881`) and sync's `fetched_at` (`sync/prices.rs`).
- **Clock goes backwards / timestamp in the future:** `now - last` underflows (`duration_since` returns `Err`), which many implementations treat as 0 elapsed forever → throttled until the clock passes the future time (days, or "forever" with a bad year). Treat a future timestamp as "unknown", allow the request, and overwrite it.
- **Clock jumps forward** (DST is not an issue for UTC epoch seconds, but a manual change or a VM resume is): a burst of allowed requests. Acceptable for an hourly limit, but never panic on arithmetic.
- **Use persisted epoch seconds for cross-restart, `Instant` only inside a session.** `Instant` does not survive restarts and may not advance during sleep on Windows, so do not use it for the persisted window.
- **Per-kind and per-league keys:** the limiter key must include what the request is about. Leagues list (`economyLeagues`), prices per league (and per category, since sync makes many calls), update check (GitHub `releases/latest`). If the key is only `prices`, switching league within the hour is blocked or, worse, serves the previous league's data as fresh. If it is per league, an arbitrary league string becomes a key in the settings file: sanitise and cap the number of keys, and drop stale entries.
- **Failed requests:** do not start the window on failure (network down, 5xx, HTTP 429 should be a *longer* backoff, not a retry). Only a successful response *and* a successful write of `prices.json` starts the window. A success whose parse fails should not count either. Record on success only, and write the timestamp after the data is persisted, never before (a crash between leaves "throttled with no data").
- **Race of two clicks / two instances:** a double click starts two requests (the existing `is_running()` guard handles one instance). Check-and-start must be atomic with the running flag. Two overlay instances can't coexist (single-instance), but the CLI can run beside the overlay, and the CLI is unthrottled by design, so the CLI must **not** update the overlay's timestamps, or it must on purpose; decide. Recommended: CLI neither reads nor writes the throttle state ("The CLI keeps automatic resolution and reads neither overlay key", extend that sentence).
- **Fake refresh lying about freshness:** the "fake refresh" (UI behaves as if refreshed when throttled) must not update "Last sync" or `fetched_at`/mtime, must not claim "synced just now", and must not hide that data is up to 1 hour old. Show the real last-fetch time, and the status line should say it used cached data (one short message). If fake refresh re-runs the Apply pricing from the on-disk `prices.json`, say so honestly. Also: user trust is damaged if "Sync economy" appears to succeed without changing prices; the tooltip should state the cooldown remaining.
- **Per-kind scope in tests:** tests must inject a clock (a `Clock` trait or a `now` parameter, like `sweep_stale(tmp_root, now)` already does) and a temp home. Never call `SystemTime::now()` inside the limiter. No sleeping tests.
- **First-run and ended-league paths:** league list fetch happens "the first time Economy is opened in a session". Throttling it means a fresh start within the hour uses the persisted list: persist the list itself (or the per-league state needs the league list to decide "ended"), or the dropdown is empty after a restart. Decide whether the league list is cached to disk. Without it, the throttle returns nothing. (Confidence MEDIUM: depends on how it is implemented; check `overlay/economy.rs`.)
- **Check for updates:** the About button is the only trigger and a user-explicit action; throttling it feels broken ("button does nothing"). Throttle only automated or repeated calls, or show the cached result with a clear message.
- **User-Agent:** two distinct strings (overlay vs CLI) must still name the public repo; `sync::client::USER_AGENT` has a test (`user_agent_names_the_public_repo`) and `update.rs` reuses the constant. Changing the constant for one path must not change the update check's UA unintentionally. Add a test for each.
- **Settings file failure:** an unreadable or invalid `ff-settings.json` means reload off and `dynamic.filter`. The throttle state must not be stored in a way that a settings parse failure silently disables the throttle (fail closed would block all requests forever; fail open could hammer). Recommended: fail open, but with the in-session `Instant` guard still active. Prefer a **separate** small state file so a settings error cannot affect it and unknown-key preservation is not stressed (needs author decision; the folder-listing says the FerrisFilter folder contents are documented, so a new file needs CLAUDE.md update).
**Phase:** P-THROTTLE.

### Pitfall 12: Economy name change breaks matchers, search, snapshots and the "ff:name" contract
**What goes wrong:** Economy block names are `# ff:name <T> tier <Group>` (`emitter.rs:554`, `:632`), and CLAUDE.md pins examples (`S tier Currency`, `A tier Uniques`). The tier price range (e.g. `S tier Currency (20-150 div)`) is a new, **price-dependent** name. Consequences:
- **Tests and fixtures:** `emitter.rs:1275/1278` assert exact names, `overlay/rules.rs:2125/2140` parse `A tier Uniques`, `app.rs:6809` checks `.contains(" tier ")`. `tests/fixtures/seed-universal.filter` is the byte-for-byte oracle for `generate`, and it carries `# ff:name` only in the "economy form" (CLAUDE.md: `generate` output has neither `ff:name` nor `ff:style`, but verify). If the range is added in the shared emitter, the `generate` output can change and break `artifact_guard`. Add the range only in the economy (apply) path, or confirm `generate` names are unaffected. Check `Show # <rule name>` in generate output separately.
- **Name changes on every sync:** the economy section is regenerated on every Apply from `prices.json`; prices change per sync, so the names change on every Apply and the economy section always "differs", which has two risks: (1) the "Apply whose result equals the file on disk is not rewritten" optimisation never applies; (2) the "changed on disk" guard and `.ffbak` are unaffected, but an Apply produces a diff for every sync, which is expected. (3) "Updated N styled rules" counters. Check that nothing treats a changed economy name as a user edit (the economy rows are read-only, so low risk).
- **Search** matches `# ff:name` and class/base values, "Search finds economy rules (by name too)". A name containing digits and ranges changes what the search matches (searching "20" now hits dozens of economy rules). Acceptable, but verify search does not break on the parentheses or `-` characters (no regex), and the unnamed/`tier` detection used for the display name.
- **Parsing:** anything that splits names on `" tier "` (to extract tier or group) will now see a suffix. Grep for `tier` string handling before changing; at minimum `app.rs:6809`, `rules.rs`. The group name is `category.name` and can contain parentheses itself (check categories with parentheses or digits, e.g. "Soul Cores", names with `-`), so the range suffix must be unambiguous, e.g. appended after a fixed delimiter, and the group portion not parsed back.
- **Units:** the tier cutoffs are chaos-denominated in live data and exalt-denominated in the fallback pair (`unit: "exalt"`); `derive_rates` expresses everything relative to Chaos. The range shown must state the unit and convert correctly, or the same tier shows `2.5` under fallback and `1.7` under live with no unit. Divine is "pinned to the A floor" under the old unit. Ranges for the top tier (no upper bound) and the lowest tier (F, lower bound 0 / hidden) need explicit text ("20+").
- **Floating-point formatting:** `0.30000000000000004`-style output, locale decimal commas, and trailing zero changes create name churn. Use a fixed rounding and integer formatting where possible; make the formatter a pure function with tests.
- **Price-tiered unique blocks share names across tiers** and "deduped per base to the highest tier". The range per group must be computed from the **cutoffs** (tiers), not from the items' actual prices in the block, or the name changes per item set. Decide which: cutoffs are stable (changes only on tier edit), item prices are volatile. Recommend cutoffs.
- **The tier editor changes cutoffs live (pending edit)**; the read-only economy rows are "re-priced when they change", so names must re-render from pending tiers, not stale ones.
- **`# ff:name` is one line:** the name must not contain a newline (strip), and is user-visible in game-folder files, so keep it short.
- **Existing user files:** the economy section is regenerated, so old names are replaced on the first Apply. A v0.4 conversion keeps `Show # <name>` names as `# ff:name`. No migration needed, but the first Apply will show a large economy diff, which is not a bug.
**Phase:** P-ECON (after alpha; independent otherwise). Release notes are written by the author, not agents.

---

## Moderate Pitfalls

### Pitfall 13: Section headers restyled but virtualized heights and hit areas change
**What goes wrong:** Section headers are 36pt rows today. A Social-panel look (different height, nine-slice art, new chevron/pencil/arrow/x hit areas) changes row height; the virtualization height table, scroll-to-rule ("jumping to a rule opens its section"), and drag slot math (Pitfall 8) all rely on it. Art taken from the game goes in `assets/poe2/` via `ff-art` (manifest entries), not hand-edited (CLAUDE.md); hand-made art goes in `assets/custom/`. Texture memory and LINEAR + mipmaps: nine-slice art stretched at 200% DPI can bleed edges (the skin uses pixel-snapped slicing, reuse it).
**Prevention:** Keep the row height a single constant used by both painting and the virtualization table. Use `ff-art` manifest for new game art, committed with the manifest, including GGG credit (README Credits). Screenshot mode (`--screenshot`) is the visual regression check; use it at 100% and 200%. Keep the economy locked header distinct (padlock, muted).
**Phase:** P-UI.

### Pitfall 14: Search field clear-X and magnifier
**What goes wrong:** The clear X takes focus away from the text edit (the field loses focus, the next typed key is lost) or does not clear when the field is focused and the Escape handler also clears the search; the magnifier overlaps the hint text (the hint carries the rule count, `Search N rules, classes, base types…`) and the text cursor start; clearing while a search is active must restore the arrows/drag (disabled while searching) and re-expand section behaviour ("Search shows matching rules inside collapsed sections"). The `+ Add rule` also clears the search. The X must be hidden when empty, must not be hit by a click that was meant for the field, and the text-field width must account for the icons (the existing `ff_text_field_id` / `_ex` helpers).
**Prevention:** Reuse `ff_text_field_ex`, draw icons inside its margins, `request_focus` after clear, unit-test "clear resets filter state and leaves focus". Same widget on the Economy/Config/class-picker/condition picker searches if they share it (backlog 999.23-999.25 mention more fields): keep one helper.
**Phase:** P-UI.

### Pitfall 15: Style editor alpha UI
**What goes wrong:** The colour picker "has no alpha for styles" and style colour cards have a None tile. Adding an Alpha slider to the style picker (shared by RULES colour actions, which already have alpha) risks regressions in the rule-action picker semantics: "a 3-token colour line stays 3-token while alpha is 255", "a colour added to an empty slot is written with 4 values", and "opening without a change edits nothing". The preview swatch must show alpha over a checkerboard or the tinted bands; the label preview uses background alpha. `Escape` restores the opening colour, including alpha. Hex field with 8 digits vs 6: decide, and make 6-digit input keep the current alpha rather than reset to 255. The nine Dracula quick picks are opaque; picking one should keep or reset alpha, define it.
**Prevention:** One picker, a `with_alpha: bool` flag already implied by the two call sites; add the style call site with the flag on, and keep a golden test for the Rules call site. Opening and closing with no change must leave `styles.json` untouched (byte-identical, Pitfall 1).
**Phase:** P-ALPHA.

### Pitfall 16: Auto Detect and alpha interplay on the styled rewrite
**What goes wrong:** If a style has alpha < 255 and a rule is tagged, the Apply rewrite writes a 4-value colour; a rule previously 3-value gets a 4-value line (acceptable, but is a byte change). If Auto Detect runs before alpha lands, the mapping logic has no alpha model and must be reworked.
**Prevention:** Land P-ALPHA first and make the detection read the final colour type. Detection must compare alpha for text, border and background individually.
**Phase:** ordering note for the roadmap.

### Pitfall 17: Test isolation regressions
**What goes wrong:** New features read the clock, the settings file, the hotkey manager and the network. Tests that touch the throttle, hotkey setting, or settings write to the real `%APPDATA%\FerrisFilter` unless `FERRISFILTER_HOME` is pinned (`tests/isolation_guard.rs` enforces on spawns; lib tests "never resolve the real folder"). The fake-refresh and throttle code must go through the injected seams (`UpdateHttp` style) so tests never hit the network.
**Prevention:** Extend the guard's pattern to any new state file; take `home: &Path`, `now: SystemTime` parameters. No test registers a real global hotkey (CI runner has no desktop session; `GlobalHotKeyManager::new` may fail), assert via a pure parse/validate function.
**Phase:** every phase; verify at P-THROTTLE and P-HOTKEY.

### Pitfall 18: Release hygiene
**What goes wrong:** Milestone rules: a bug introduced and fixed within the milestone is never mentioned outside `.planning/`; agents do not write changelog entries during a milestone; the Release Review phase does cleanup, screenshots and version. `CLAUDE.md` rows ("hardcoded `Ctrl+Shift+Z`", "always emits alpha 255", "no alpha", "arrows", "moved across Continue") become inaccurate; each phase should list the CLAUDE.md sentences it invalidates, since "Everything from Repository layout down is that reference — keep it accurate".
**Phase:** each phase's last plan, plus Release Review.

---

## Minor Pitfalls

### Pitfall 19: Custom hotkey docs and tooltips repeating the chord
Strings in the tray tooltip, README, and `instance.rs` that name `Ctrl+Shift+Z`; make a single `display()` function. (see Pitfall 10).

### Pitfall 20: Drag handle art states and DPI
Three art states exist (normal/hover/pressed) as untracked files; they must be committed with their `.ase` and loaded through the skin's `include_bytes!`, LINEAR + mipmaps; handle art at fractional DPI must be pixel snapped.

### Pitfall 21: Throttle state growing without bound
Per-league keys accumulate over seasons: prune keys older than the window plus a margin on write.

### Pitfall 22: Auto Detect run time and UI freeze
Detection over several hundred blocks, with emitter calls per candidate style per rule, can be slow in a debug build; precompute the per-style expected action set once, not per rule.

---

## Phase-Specific Warnings

| Phase Topic | Likely Pitfall | Mitigation |
|-------------|---------------|------------|
| P-ALPHA (do first) | Shared colour type changes `styles.json`/fixture bytes (1); picker regression (15) | Optional alpha, skip when 255; byte-identical round-trip + `artifact_guard` tests first |
| P-AUTO | Rewrite changes bytes (2); equality traps (3); name/id collisions (4); Revert/Reload/Quit orphaning (5); groups/disabled/economy (6) | Detect via the emitter on effective actions; table-driven trap tests; smallest free N + shared id generator; one atomic pending edit across Rules + Config; use existing edit APIs |
| P-DND | Source row virtualized away (7); invalid drop targets (8); no keyboard path, auto-scroll timing (9) | Pointer-owned drag state, slot from y and a height table; reuse the arrow mover; time-based auto-scroll with repaint; author decision on keyboard fallback |
| P-UI | Header height vs virtualization (13); search icons focus (14) | One height constant; shared text field helper; screenshot checks at 100% and 200% |
| P-HOTKEY | Wrong unregister, register-order, id mismatch, capture firing the old hotkey, modifier-only, physical key mapping (10) | Register new then unregister old; stored id updated; pause hotkey during capture; require modifier + key; physical keys |
| P-ECON | Name parsing, `artifact_guard`, units, volatile names (12) | Cutoff-based range via pure formatter; economy path only; grep `tier` string uses; unit-aware text |
| P-THROTTLE | Future timestamps, per-league keys, failure counting, fake refresh honesty, CLI separation, UA (11) | Injected clock; record on success after persisting; tolerant arithmetic; honest status; separate state file (author decision) |

## Suggested ordering implied by dependencies

P-ALPHA before P-AUTO (equality needs the final colour type). P-UI before or with P-DND (row heights, header art). P-DND is independent of the rest. P-ECON and P-THROTTLE are independent; P-THROTTLE touches the Economy tab's sync and league fetch, so land it after P-ECON to avoid rework in the same code. P-HOTKEY is independent and the smallest blast radius; it needs a manual in-game check.

## Items needing author review / flagged

- `DEP?` Drag and drop: do not add a DnD crate; egui's own pointer and `DragAndDrop` support should suffice (verify on 0.36).
- `DEP?` Hotkey chord string parsing: confirm `global-hotkey` 0.8 `HotKey::from_str` exists; otherwise write a small parser (no new crate).
- `DEP?` Throttle: epoch seconds via `std::time`; no crate needed (no `chrono`).
- Decision: keep a keyboard reorder path after removing the arrows (Pitfall 9).
- Decision: where throttle state lives (`ff-settings.json` key vs a separate file) and whether the league list is cached on disk (Pitfall 11).
- Decision: whether Auto Detect may rewrite rules whose bytes would change (comments, 3-value colour) (Pitfall 2).

## Sources

- `C:\Users\jonat\Repositories\poe2_myfilter\CLAUDE.md` (HIGH; invariants)
- `src/overlay/hotkey.rs` (HIGH; constants, register/release, thread-affinity note)
- `src/emitter.rs` lines 122, 554, 632, 1275-1278 (HIGH; economy `# ff:name` format and pinned tests)
- `src/overlay/app.rs` lines ~3881, 6809; `src/sync/client.rs`, `src/update.rs` (HIGH; clock use, UA constant, `.contains(" tier ")`)
- `src/overlay/rules.rs` lines ~2125-2140 (HIGH; name parsing test)
- egui drag/auto-scroll, `global-hotkey` 0.8 behaviours: training knowledge, not re-verified (LOW-MEDIUM; verify during phase research)
