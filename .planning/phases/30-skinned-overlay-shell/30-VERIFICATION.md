---
phase: 30-skinned-overlay-shell
verified: 2026-10-16T00:00:00Z
status: human_needed
score: 8/8 must-haves verified (automated); runtime and visual checks remain
overrides_applied: 0
re_verification:
  previous_status: none
gaps: []
warnings:
  - "RESOLVED after verification: the RULES custom-sound caption 'The file must be in the game folder.' is now a tooltip on the File field (src/overlay/body.rs:2814), matching CONFIG (config_view.rs:1136)."
  - "apply::filter_state_message (src/overlay/apply.rs:260) is dead code outside its own tests: the old status-block text it builds ('Economy section found.' etc.) no longer reaches the UI. Info only."
human_verification:
  - test: "See the numbered checklist sections A-G in the report body"
    expected: "Each item behaves as described"
    why_human: "Visual appearance, Win32 docking, DPI, multi-monitor, global hotkey, tray, single-instance and in-game keystroke reload cannot be exercised by grep or unit tests"
---

# Phase 30: Skinned Overlay Shell Verification Report

**Phase Goal:** Launching `ff` opens the new game-styled window instead of the stripe, with the existing RULES and CONFIG content working inside it and every v0.5 overlay behaviour (hotkey, tray, single instance, in-game reload) intact.
**Verified:** 2026-10-16
**Status:** human_needed
**Re-verification:** No (initial verification). Review fixes (86a84a1..2a5d6fa) were verified in their current state.

## Goal Achievement

### Observable Truths (ROADMAP / REQUIREMENTS contract)

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| SHELL-01 | `ff` opens the skinned 660pt window docked left, full height, borderless, with Ferris behind content | VERIFIED (visual pending) | `src/bin/ff.rs` calls `overlay::launch()` (`default-run = "ff"`). `native_options()` in `src/overlay/mod.rs:117`: always_on_top, decorations off, resizable off, taskbar off, inner width `shell::WIDTH` (660). `app.rs` `ui()` paints `w::ff_background`, `ff_frame`, `ff_header`, tabs, content, bottom bar (lines 2330-2492). `shots/rules.png` shows the window at 660 px width, full height, Ferris faint behind rows. |
| SHELL-02 | Docks on the game's monitor (primary without game), correct at 100-200% | VERIFIED (runtime pending) | `win::target_monitor` (`MonitorFromWindow(game, NEAREST)` else primary), `monitor_work_area_px` (`GetMonitorInfoW.rcWork`), `monitor_scale` (`GetDpiForMonitor`), `dock_left` converts via `skin::window::dock_rect_px` and `SetWindowPos` twice (win.rs:206-279). Called on first handle, on every Reveal (`app.rs:2203`) and on scale change (`app.rs:2060`). Pure unit tests cover 100/150/200%, negative origin, left taskbar, clamp (`skin/window.rs` tests). |
| SHELL-03 | Title plate with "FerrisFilter" in a swappable art slot; close hides | VERIFIED | `shell::header_art` is the single switch (`shell.rs:~395`); `ff_header(..., "FerrisFilter")` at `app.rs:2345`; close sets `Visibility::Hidden` and calls `win::conceal`, never `ViewportCommand::Close` (the only Close send site is the quit path, `app.rs:2603`). Tooltip "Hide (Ctrl+Shift+Z)". |
| SHELL-04 | Tabs RULES / CONFIG / ECONOMY / ABOUT, ` *` marker kept | VERIFIED | `ShellTab::ALL`, `shell::tab_markers` (marker on a tab when the OTHER editing tab is dirty; Economy/About never marked), `ff_tabs_ex` with `apply::tab_label` and `OTHER_TAB_DIRTY_TIP`. Screenshots show the four stone tabs. |
| SHELL-05 | Bottom bar Revert / Reload / Apply + one status line (cut with ellipsis, hover full text, coloured by kind) | VERIFIED | `bottom_bar` (`app.rs:~2000-2044`) draws `ff_button_ex` x3 and `ff_status_ex`. `shell::status_for` is the executable priority table; `event_from` and `apply_success_kind` carry kinds (Ok/Warning/Error/Info). `shots/status-truncated.png` shows a 290-char text cut on one line. The 96px outcome area is gone (`grep "outcome area" CLAUDE.md` = 0 per summary; no such widget in `app.rs`). |
| SHELL-06 | Hotkey, tray + Quit, single instance, in-game reload work as in v0.5 | VERIFIED (runtime pending) | `git diff f34cb86` shows `instance.rs`, `hotkey.rs`, `chat.rs`, `visibility.rs` untouched; `tray.rs` only gained a debug-only `inert()` constructor. `launch()` still runs `instance::acquire`, `hotkey::register_toggle`, `tray::build` before `run_native`. Hotkey gate `win::foreground_is_game_or_overlay` (`app.rs:2081`), second-launch `take_show_request` (`app.rs:2179`), visibility state machine drives hide-on-game-focus (`app.rs:2195`). Tests green (instance, hotkey, chat, visibility suites). |
| SHELL-07 | Every confirmation uses the skinned popup | VERIFIED | `skin/popup.rs::ff_modal` (dim, game box, game buttons, Escape and backdrop click answer the LAST button). Seven `Popup` variants dispatched in `app.rs:1390-1535` (Quit, ChangedOnDisk, SwitchFilter, DeleteRule, DeleteStyle, RemoveEconomy, DiscardConfig), one per frame in priority order via `shell::active_popup`. No inline confirm frame remains (inventory A19, grep of `confirm_` shows only popup flags). Screenshots `popup-quit/changed/delete-rule.png` viewed. |
| SHELL-08 | UI reads less as text: captions to tooltips, flags to icons, messages to the status line | VERIFIED (residual caption fixed after verification) | 22 caption checks below match code. Flags are painted icons (`skin/icons.rs`, `rule_list.rs::flags_for`). Messages are routed through `status_for`. The one residual standing caption (RULES sound editor) was converted to a tooltip after verification (`body.rs:2814`). |

**Score:** 8/8 verified at the code level. Nothing FAILED. Visual, DPI, multi-monitor and keystroke behaviours are UNVERIFIABLE here and listed for the author.

### Required Artifacts

| Artifact | Status | Details |
|----------|--------|---------|
| `src/overlay/shell.rs` (1319 lines) | VERIFIED | tabs, markers, `status_for`, `bar_state`, `OverwriteBlock`, `popup_spec`, Economy and About tabs; used by `app.rs`. |
| `src/overlay/win.rs` dock functions | VERIFIED | monitor-aware, DPI-aware; wired from `app.rs`. The old primary-only `work_area()` remains but is no longer the dock path. |
| `src/skin/popup.rs`, `src/skin/icons.rs`, `src/skin/window.rs` | VERIFIED | substantive, unit-tested, wired. |
| `src/overlay/screenshot.rs` | VERIFIED | debug-only (`#[cfg(debug_assertions)]`) screenshot mode; release launch path has no argument parsing. |
| `shots/*.png` (8) | VERIFIED | viewed rules, config, popup-quit; consistent with the claims. |
| `CLAUDE.md`, `CHANGELOG.md` | VERIFIED per summary | not re-audited line by line. |

### Key Link Verification

| From | To | Via | Status |
|------|----|-----|--------|
| `bin/ff.rs` | `overlay::launch` | direct call | WIRED |
| `launch` | `OverlayApp::new` + skin install | `run_native` closure | WIRED |
| `ui()` | `win::dock_left` | first handle, every Reveal, scale change | WIRED |
| Status line click | RULES tab + expanded rule | `jump_to` sets `self.tab = Rules` | WIRED |
| Bar Reload (CONFIG tab, refusal pending) | `ConfigAction::Reload` | `bar_reload_target` | WIRED (CR-01 fix) |
| Popup choice | v0.5 handlers | `dispatch_popup` (Apply & quit uses `quit_step`, `quit_after_apply`) | WIRED |
| Apply slot while file changed | Overwrite -> reopens popup | `bar_state.apply.overwrite` -> `changed_popup = true` | WIRED |

### Data-Flow Trace (Level 4)

| Artifact | Data | Source | Status |
|----------|------|--------|--------|
| Status line | `StatusInputs` | real `apply_message`, `config_message`, economy state, `list_status`, grammar scan, filter state | FLOWING |
| Economy tab | league, last sync | `crate::sync::OVERLAY_LEAGUE`, `prices.json` mtime via `refresh_last_sync` | FLOWING |
| Rows' flag icons | `RuleRow` fields (errors, warnings, style, group_broken, moved_across_continue) | parsed filter | FLOWING (`rules.png` shows each icon) |

### Behavioral Spot-Checks and Automated Gates

| Check | Result | Status |
|-------|--------|--------|
| `cargo test` | lib 1142 passed, 0 failed; all integration suites ok (`artifact_guard`, `generate_snapshot` included) | PASS |
| `cargo fmt --all --check` | clean | PASS |
| `git diff --exit-code f34cb86 -- tests/fixtures` | no diff | PASS |
| clippy (`-p ferrisfilter --all-targets`, `^warning` count) | 126 (baseline 130, expected <= 126) | PASS |
| Debt markers (TBD/FIXME/XXX) in files changed since f34cb86 | none | PASS |
| Probes | none declared by this phase | N/A |
| Launching `ff`, keystrokes, game folder | deliberately not done (instruction) | SKIPPED |

### Requirements Coverage

PLAN frontmatter `requirements`: 30-01 [SHELL-02, 05, 07, 08]; 30-02 [SHELL-01..06]; 30-03 [SHELL-07, 06]; 30-04 [SHELL-08]; 30-05 [SHELL-01..08]. Union = SHELL-01..08. REQUIREMENTS.md maps exactly SHELL-01..08 to Phase 30 (lines 26-50, 174-181). No orphaned IDs, none missing.

| Requirement | Status | Evidence |
|-------------|--------|----------|
| SHELL-01 | SATISFIED (visual pending) | truth table |
| SHELL-02 | SATISFIED (runtime pending) | truth table |
| SHELL-03 | SATISFIED | truth table |
| SHELL-04 | SATISFIED | truth table |
| SHELL-05 | SATISFIED | truth table |
| SHELL-06 | SATISFIED (runtime pending) | truth table |
| SHELL-07 | SATISFIED | truth table |
| SHELL-08 | SATISFIED | truth table |

### Anti-Patterns Found

| File | Line | Pattern | Severity |
|------|------|---------|----------|
| `src/overlay/body.rs` | 2817 | Standing caption `caption(ui, "The file must be in the game folder.")` in the RULES custom-sound editor | Fixed after verification: now a tooltip on the File field (body.rs:2814) |
| `src/overlay/apply.rs` | 260 | `filter_state_message` unused outside tests | Info |
| `src/overlay/win.rs` | 175 | `work_area()` (primary only) kept; no longer used for docking | Info |

### Inventory corrections (SUMMARY vs code)

I checked each item of the 30-05 `30-LESS-TEXTY-INVENTORY` against the code. All matched except these, which are corrected in the checklist below:

1. **A5**: the SUMMARY implies the "file must be in the game folder" caption is gone everywhere. It is a tooltip only on CONFIG; the RULES editor still prints it (`body.rs:2817`). (Fixed after verification: RULES now uses the tooltip too.)
2. **C1/C7 "shows until the next edit"**: event messages are also cleared when the window is hidden (`clear_events_on_conceal`, `app.rs:1303`), except the interim reload line. Also any edit or Revert on either tab clears them (`clear_events_on_edit`, WR-01 fix).
3. **C5**: the Rules-tab Revert message `Reverted to the last applied filter.` is a `list_status`, so it renders as Info (muted), not green. A CONFIG message that is not a problem is also Info.
4. **A14/A15**: `apply::filter_state_message` still exists but is dead; the UI text is `shell::ABSENT_STATUS`, `MIGRATED_STATUS`, `DAMAGED_MARKERS_MESSAGE`.

## Human Verification Required

Run `ff` from a temp copy (`FERRISFILTER_HOME` and `USERPROFILE` set, per the CLAUDE.md development warning) so your live `styles.json`, `tiers.json` and game-folder filter are untouched. `ff --screenshot x.png [--tab rules|config|economy|about] [--popup <name>] [--status "<text>"]` (debug build only, temp home) reproduces many of these without the game.

### A. Standing captions that became tooltips or short headings

- [ ] A1 Config > Filter dropdown. Old: standing line "Saved on this computer. Default: dynamic.filter." Now: tooltip on the dropdown, line 1 the full filter path, line 2 that sentence (no game folder: only the sentence). Code: `config_view.rs:360-368`.
- [ ] A2 Config > Filter, **Reload filter in game after Apply** toggle. Old: paragraph under it. Now: tooltip on the toggle (and label) = paragraph + blank line + short tip (`config_view.rs:~380`).
- [ ] A3 Config > Filter, new-filter name field. Old: helper line. Now: tooltip on the empty field; a non-empty invalid name still shows an inline hint.
- [ ] A3b Config > Styles, **+ Add style** name field. Same treatment as A3.
- [ ] A4 Config > Styles with no styles loaded. Old: "No styles loaded. Revert to reload styles.json." Now: amber inline `No styles loaded`, hover for "Revert to reload styles.json." (`config_view.rs:637`).
- [ ] A5 Config > custom style with custom sound, **File** field. Old: caption "The file must be in the game folder." Now: tooltip on the field (`config_view.rs:1136`). Also in RULES: a rule's custom-sound **File** field now carries the same tooltip instead of the caption (`body.rs:2814`).
- [ ] A6 Rules, empty filter. Old: heading plus "Press + ADD RULE to write your first one..." Now: heading `No rules yet` only (`app.rs:1936`); hover **+ Add rule** for the sentence.
- [ ] A7 Rules, search with no hits. Old: heading plus "Check the spelling, or press Clear...". Now: heading `No rules match "<query>"` only; the field's Clear is the way out.
- [ ] A8 Rules, unloadable filter. Old: heading, error paragraph, "Fix the file and press RELOAD." Now: heading `The filter could not be loaded.` (`app.rs:1955`); detail in the red status line `The filter could not be loaded: <msg>. Fix the file and press Reload.` (`shell.rs:107`).
- [ ] A9 Expanded rule with no conditions. Old: "No conditions: this rule matches every item." Now: muted `No conditions` (`body.rs:449`), the sentence on hover.
- [ ] A10 Expanded rule with no actions. Old: "No actions: matching items keep the game's look." Now: muted `No actions` (`body.rs:2537`), sentence on hover.
- [ ] A11 Disabled rule with invalid grammar, expanded. Old: long "can't enable" paragraph. Now: amber `Can't enable`, long text on hover; the enable toggle carries the same tooltip (`rule_list.rs:610`).
- [ ] A12 Disabled Apply with a grammar error. Old tooltip: `Fix or disable the rules marked "grammar error" first.` Now: `Fix or disable the rules marked with the red ! first.` (`apply.rs:202`).
- [ ] A13 Filter path. Old: line in the status block. Now: hover the gold "FerrisFilter" plate; tooltip = full path only (`app.rs:2347-2354`).
- [ ] A14 "Economy section found." / "The filter has no economy section." Old: status-block line. Now: dropped; the economy footer row (`+ N locked rule blocks (economy)` / `No economy section`) says it.
- [ ] A15 v0.4 notice, absent-filter notice, damaged-markers line. Old: status-block paragraphs. Now: status line only (C12, C13, C10).
- [ ] A16 Apply indicator ("Filter out of date..." / "Filter matches saved rules"). Old: line above the buttons. Now: the default status line (C14).
- [ ] A17 Rule count. Old: "N rules (shown of N)" label and a "FILTER" header. Now: search hint `Search N rules, classes, base types…` (`rule_list.rs:815`).
- [ ] A18 Economy remove. Old: inline "Remove the economy section? Yes, remove / Keep". Now: the Remove economy popup carrying "Your own rules are kept. Add the section back any time." (D6).
- [ ] A19 Delete rule, Delete style, Revert on Config, Switch filter. Old: inline confirm frames. Now: popups D4, D5, D7, D2. No inline confirm should remain anywhere.
- [ ] A20 Outcome area. Old: 96px scrolling block with several lines and jump links. Now: one status line (section C): one message at a time, full text on hover only when cut.
- [ ] A21 Kept on purpose (should still be inline): field-validation hints, `Not set (game default)`, short field captions, the two tier helper lines (`Minimum price for each tier...`, `Only your live tiers.json is edited...`).

### B. Flags that became icons (collapsed Rules rows)

Painted right to left starting next to the reorder arrows; each icon is an 18pt hover target (`rule_list.rs::flags_for`, `skin/icons.rs`).

- [ ] B1 Grammar error: red filled circle with `!`. Tooltip: one `Line N: message` per diagnostic. Try `Sockets >= abc`. Never folded into `+N`.
- [ ] B2 Grammar warning: amber outline triangle with `!`. Tooltip: `Line N: message`. Try `LinkedSockets >= 4`.
- [ ] B3 Unknown style: amber ring with `?`. Tooltip: "No style with this name exists. This rule keeps its current actions, and its style link is removed on the next Apply." Try `# ff:style nosuch` above `Show`; the status line also reads `1 rule uses an unknown style (first: rule N).` and clicking it jumps to the rule.
- [ ] B4 Group broken: amber broken-link glyph (two open ovals with a gap; reads small at 14pt per the 30-05 observation). Tooltip: "Group broken: these blocks were one rule (# ff:group) but no longer match, so each is shown on its own. Left as written until you edit it." Try a lone `# ff:group abc123 1/2` block.
- [ ] B5 Moved across Continue: amber up/down arrows with a bar. Tooltip: "This move changes which rules still apply after a Continue. Check the order." Reproduce by moving a rule across a `Continue` rule (cannot be rendered by the screenshot mode).
- [ ] B6 Continue: stays the gold small-caps word `Continue`. Tooltip: "Matching goes on to the rules below".
- [ ] B7 Styled: gold diamond + style name in muted text, cut at 10 characters plus `…` (was 12, no diamond). Tooltip: `Style "<full name>" sets this rule's actions on Apply`.
- [ ] B8 Can't enable: amber padlock alone on a disabled rule with invalid grammar (the only flag shown for it). Tooltip: "Can't enable: these lines are not valid filter grammar. You can still move or delete it."
- [ ] B9 Order right to left: error, warning, unknown style, group broken, moved across Continue, Continue, styled. Overflow folds the least important into a muted `+N` badge (hover lists `name: tip`); the label keeps at least 140px.
- [ ] B10 Accessible names: each icon keeps the old flag word as its accessible name (UI inspector; not visible).

### C. Where each former outcome-area, status-block or footer message now appears

All appear in the one status line left of Revert / Reload / Apply. Colours: Info muted, Ok green, Warning amber, Error red. Priority (`shell::status_for`): root error, event message, grammar gate, load failure or damaged markers, sync running, unknown styles, v0.4 notice, absent filter, indicator.

- [ ] C1 Apply success: `Wrote <path>.` plus `Reloaded in game` when the game runs and the setting is on. Green; amber when the text carries the Options > Game > Item Filter > Refresh reminder. Stays until the next edit, Revert, or hiding the window (correction).
- [ ] C2 Written but not reloaded (setting off, reload in progress, window not ready, game not running, `could not reload in game: <cause>`): amber, with the Refresh reminder and the reason.
- [ ] C3 Apply refused, write failed or a file changed on disk: red, names the file(s); a changed file also opens the File changed on disk popup (D1).
- [ ] C4 Overwrite outcome: same texts as Apply, in-game reload included.
- [ ] C5 Reload outcome (re-read from disk): status message from the reload. Rules-tab Revert message `Reverted to the last applied filter.` is Info (muted), not green (correction).
- [ ] C6 Sync economy: `Fetching economy prices…` (Info) while the button reads `Syncing…` and is disabled; then `Economy prices refreshed from <league>. Press Apply to compile them into the filter.` (green) or `Economy sync failed: <err>` (red). Shown on every tab.
- [ ] C7 List status, e.g. `Rule added at position N (hidden by the current search)`, and `Style removed. The actions are now yours to edit.`: Info, replaced by the next edit.
- [ ] C8 Grammar gate: `Apply refused: 1 enabled rule has a grammar error (first: rule N, line M). Fix them or turn them off.` Red; clicking it switches to RULES, expands and scrolls to the rule (clearing a hiding search); hand cursor while a target exists; Apply disabled with the A12 tooltip.
- [ ] C9 Unknown-style summary: amber, click jumps to the rule.
- [ ] C10 Damaged economy markers: `The economy section markers are damaged. Fix them by hand, then Reload.` Red; footer row label red with the same tooltip; Apply disabled with that tooltip.
- [ ] C11 Load failure: `The filter could not be loaded: <msg>. Fix the file and press Reload.` Red.
- [ ] C12 v0.4 filter: `v0.4 filter: converted on the next Apply (a .ffbak backup is kept).` Info.
- [ ] C13 Absent filter: `The filter does not exist yet. Apply creates it once it has rules.` Info.
- [ ] C14 Default indicator: `Filter out of date — press Apply, then reload in-game` amber when unapplied or never applied; `Filter matches saved rules` Info when applied. Same on every tab.
- [ ] C15 Missing app root: red, highest priority.
- [ ] C16 Config message (e.g. `Could not save ff-settings.json: <err>` with a read-only settings file): red on every tab.
- [ ] C17 Long text: cut with `…` on one line, full text on hover only then (max width 420), same colour. Reproduce with `ff --screenshot x.png --status "<300 chars>"` and compare `shots/status-truncated.png`.
- [ ] C18 Button tooltips: Reload "Discard your edits and load the file from disk." (disabled: "Nothing to reload."), Overwrite "Replace the file with your edits.", Apply disabled reasons per `shell::bar_state` (tier errors, load error, red `!` rules, damaged markers, nothing to apply).

### D. Popups

One at a time. The last (safe) button answers Escape and a click on the dim backdrop. The popup stays open if the window is hidden and summoned again. Priority when stacked: Quit, File changed on disk, then the others (`shell::active_popup`). Reproduce with `--popup <name>`.

- [ ] D1 `changed`: title "File changed on disk"; body names the file(s) and explains Reload and Overwrite. Buttons: Reload, Overwrite (red), Keep editing (the last, so Escape = Keep editing). When Overwrite is unavailable the Overwrite button is absent and one sentence explains why (see G3). After Keep editing the Apply slot reads **Overwrite** (red) and reopens the popup. Trigger: edit the filter on disk, then Apply.
- [ ] D2 `switch`: "Switch filter?", body "Unapplied rule changes will be discarded. Switching to <file>." Buttons: Switch and discard (red), Stay. Escape = Stay. Trigger: Config > Filter, pick another filter with unapplied rule edits.
- [ ] D3 `quit`: "Unapplied changes", body "Apply them before quitting?" Buttons: Apply & quit, Quit without applying (red, tooltip "Unapplied edits are lost"), Cancel. Escape = Cancel. Apply & quit quits only after a successful Apply and waits for an in-game reload; with nothing pending Quit exits at once.
- [ ] D4 `delete-rule`: "Delete this rule?", body `Rule N: <name or Show/Hide>` plus `Deletes all N blocks of this rule.` for a group. Buttons: Delete (red), Keep. Trigger: expand a rule, Delete.
- [ ] D5 `delete-style`: "Delete style?", body the rule count and the `config.json` warning when referenced. Buttons: Delete style (red), Keep style.
- [ ] D6 `remove-economy`: "Remove economy section?", body "Your own rules are kept. Add the section back any time." Buttons: Remove (red), Keep.
- [ ] D7 `discard-config`: "Discard config changes?", body `Discard N unapplied config changes?` Buttons: Discard (red), Keep editing.
- [ ] D8 Stacking: with a quit request and a changed-on-disk refusal both pending, Quit shows first, then File changed on disk.
- [ ] D9 Keyboard: Escape = last button (no action); Tab cycles buttons; Enter/Space activates the focused one; no button focused on open; backdrop click = last button.
- [ ] D10 Ctrl+Shift+Z while a popup is open, summon again: the same popup is still there.

### E. Docking, DPI and monitors

- [ ] E1 Game on the primary monitor at 100%: window 660pt wide, flush left, full work-area height (above the taskbar), borderless, not movable, not resizable, always on top.
- [ ] E2 Repeat at 125%, 150%, 200% (sign out/in if needed): still 660 logical points (grows in pixels), full height above the taskbar, no gap or overlap with the screen edge. 125% was the old failure.
- [ ] E3 Game (windowed or windowed-fullscreen) on a secondary monitor: the window docks on that monitor's left edge at that monitor's own scale.
- [ ] E4 Game not running: docks on the primary monitor.
- [ ] E5 Re-dock on summon: hide the window, move the game to the other monitor, press Ctrl+Shift+Z (game in front): it appears on the game's monitor. Repeat via a tray icon click and via launching `ff` a second time.
- [ ] E6 Move the game between monitors of different scale, then summon: correctly sized (no mixed physical/logical sizes); a scale change while shown re-docks once.
- [ ] E7 Taskbar on the left: window starts right of it. Taskbar on top: window starts below it and ends above any bottom bar.
- [ ] E8 Close button (round red, top right) hides, never quits; tooltip "Hide (Ctrl+Shift+Z)". Only tray Quit exits.
- [ ] E9 Window cannot be dragged or resized; it hides when the game takes focus (v0.5 behaviour).
- [ ] E10 Content fits at about 535pt with no horizontal scrolling (expanded rule with Class, Rarity and Defences cards; the Custom styles table).

### F. v0.5 behaviour checks (SHELL-06)

- [ ] F1 Ctrl+Shift+Z toggles only while Path of Exile 2 or ff is the foreground window; in a browser or editor the key does nothing and is not replayed later.
- [ ] F2 Tray: Show works; tray Quit with nothing unapplied exits at once; with unapplied edits (either tab) it opens the Quit popup: Apply & quit (quits only after a successful Apply, waits for the in-game reload), Quit without applying, Cancel.
- [ ] F3 Single instance: launching `ff` again (also while the first is hidden to tray, or still starting) shows the running window and the new process exits; after tray Quit an immediate relaunch starts a fresh window.
- [ ] F4 Apply with "Reload filter in game after Apply" on and the game running (same privilege level, chat closed): `/reloaditemfilter` is sent and the status reads `Reloaded in game`; with the game not running it reads `Wrote <path>.` With the setting off the status keeps the Options > Game > Item Filter > Refresh reminder.
- [ ] F5 Exclusive fullscreen still hides the window (unchanged requirement: windowed or windowed-fullscreen).
- [ ] F6 Tab ` *` markers: edit on RULES, look at CONFIG (marked), and the reverse; Economy and About never marked; switching tabs applies and discards nothing.

### G. Review-fix behaviours flagged "requires human verification"

- [ ] G1 CONFIG-tab Reload routing (CR-01, `app.rs` `bar_reload_target`): on CONFIG with a changed-on-disk refusal pending, the bar's Reload re-reads only the changed files (a changed `styles.json`/`tiers.json` discards CONFIG edits; a changed filter discards RULES edits). On CONFIG with no refusal, Reload re-reads the filter and keeps CONFIG edits.
- [ ] G2 Overwrite enable/disable (WR-03 and WR-02): during a refusal the Apply slot reads **Overwrite** (red). It is enabled only when Apply would be; the bar and the popup agree (the popup then has the Overwrite button; otherwise it has Reload and Keep editing only). A loaded `styles.json`/`tiers.json` that fails validation also disables it.
- [ ] G3 Disabled Overwrite reason (WR-02): the bar tooltip and one popup sentence name the reason, in this order: tier errors ("Fix the tier errors to overwrite instead."), invalid config ("styles.json or tiers.json is invalid. Fix it first." / "Fix styles.json or tiers.json to overwrite instead."), flagged rules ("Fix or disable the flagged rules to overwrite instead."), damaged markers ("Overwrite is off while the economy markers are damaged."), nothing to write ("Nothing to overwrite." and no popup sentence). Check the wording reads well in the skinned popup.
- [ ] G4 Apply status colours (CR-02): success green, success with the Refresh reminder amber, refusal and failure red; colour is fixed when the message is produced and does not change when the apply state later flips to out-of-date.
- [ ] G5 Stale messages (WR-01, both iterations): after any edit, Revert (either tab), economy add/remove staging, or hiding and re-summoning, no old `Wrote ...` / `Rule added ...` / config message hides the standing lines (grammar gate, load error, out-of-date indicator). Exception: an in-progress reload line survives a hide.
- [ ] G6 Status-line jump (WR-02 iter 1): from CONFIG or ECONOMY click the grammar-gate or unknown-style line: it switches to RULES and expands the rule.
- [ ] G7 CONFIG Revert (WR-04): disabled while a changed-on-disk refusal is pending; style switch track and tier-error bar are visibly coloured (WR-06); ABOUT disclaimer has no run of spaces (WR-05).

## Gaps Summary

No blocking gaps. All eight SHELL requirements are implemented, wired and exercised by a green suite (1142 lib tests, integration suites, fmt clean, fixtures untouched, clippy 126 against a ceiling of 126). The v0.5 process-level modules (single instance, hotkey, chat reload, visibility) are byte-identical to the pre-phase code, which is the strongest available evidence for SHELL-06 without running the game. Remaining work is human: the visual, DPI, multi-monitor and keystroke checks above. The one residual caption the verifier found (RULES sound editor) was converted to a tooltip afterwards (`body.rs:2814`).

---

_Verified: 2026-10-16_
_Verifier: Claude (gsd-verifier)_
