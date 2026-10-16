---
phase: 30-skinned-overlay-shell
reviewed: 2026-10-16T00:00:00Z
depth: standard
files_reviewed: 19
files_reviewed_list:
  - Cargo.toml
  - src/overlay/app.rs
  - src/overlay/apply.rs
  - src/overlay/body.rs
  - src/overlay/config_view.rs
  - src/overlay/economy.rs
  - src/overlay/mod.rs
  - src/overlay/rule_list.rs
  - src/overlay/screenshot.rs
  - src/overlay/shell.rs
  - src/overlay/theme.rs
  - src/overlay/tray.rs
  - src/overlay/win.rs
  - src/skin/icons.rs
  - src/skin/mod.rs
  - src/skin/popup.rs
  - src/skin/widgets.rs
  - src/skin/window.rs
  - src/sync/prices.rs
findings:
  critical: 2
  warning: 8
  info: 5
  total: 15
status: issues_found
---

# Phase 30: Code Review Report

**Reviewed:** 2026-10-16
**Depth:** standard (focused on `git diff f34cb86..HEAD -- src Cargo.toml`)
**Files Reviewed:** 19
**Status:** issues_found

## Summary

The shell's pure decision layer (`shell.rs`: `status_for`, `bar_state`, `active_popup`,
`popup_spec`) is well tested. Several v0.5 paths still behave as before: the hotkey gate, the tray
Quit order, the single-instance poll, the conceal/reload-result interplay and the Quit-confirm state
machine. The Win32 monitor/DPI code is sound: the unsafe blocks use checked returns and
correctly sized structs, and `MonitorFromWindow` handles minimised and stale handles. The screenshot
mode is compiled out of release builds.

The defects are in how `app.rs` wires the new single bottom bar and single status line into the old
per-tab logic:

- The shared **Reload** button on the CONFIG tab silently throws away CONFIG edits in states where
  it is only enabled because the *filter* is broken. It also never re-reads that filter.
- The status line's colour comes from `apply_state`, not from the Apply outcome. A failed
  Apply can therefore show in the success colour, which breaks D-17.
- With only one line left, stale event messages (CONFIG messages in particular, which are never
  cleared on hide) hide the grammar gate, the load error and the out-of-date indicator.

## Critical Issues

### CR-01: CONFIG-tab Reload discards unapplied CONFIG edits without confirmation and never reloads the filter it was enabled for

**File:** `src/overlay/app.rs:2374-2378` (dispatch), `src/overlay/app.rs:1666-1680` (`ConfigAction::Reload`), `src/overlay/shell.rs:273-277,319` (`bar_state`)
**Issue:** `bar_state` enables Reload on every tab from the RULES-derived `actions.reload`. That is
`needs_reload || !root_ok || Damaged || LoadFailed`. On the CONFIG tab the click becomes
`ConfigAction::Reload`. With no changed-on-disk refusal (`changed_on_disk` empty),
`config_reload_plan` returns `ConfigOnly`, which runs `self.config = ConfigState::load(root)`.

Picture a user whose filter has damaged economy markers. They are editing styles on CONFIG, fix the
markers by hand, and press Reload, as the status line tells them to ("Fix them by hand, then
Reload."). This happens:
1. Every unapplied style and tier edit is lost, with no Discard confirmation. The CONFIG Revert
   path asks first through the DiscardConfig popup.
2. The damaged or failed filter is not re-read, so the status line keeps the same error.

v0.5's CONFIG footer offered RELOAD only after a changed-on-disk refusal, so this is a new
data-loss path.
**Fix:** On the CONFIG tab, send a Reload that has no changed-on-disk refusal to the filter reload,
which keeps CONFIG edits unless the root moved:
```rust
if clicks.reload {
    match self.tab {
        ShellTab::Config if !self.changed_on_disk.is_empty() || self.config.is_err() => {
            config_actions.push(ConfigAction::Reload)
        }
        _ => self.reload_pressed(),
    }
}
```
The inline "Reload from disk" in `show_load_error` can keep using `ConfigAction::Reload`. Add an app
test: config edits plus a Damaged filter, then a bar Reload on CONFIG, must keep `config.is_dirty()`
and re-read the filter.

### CR-02: An Apply failure can show in the success (Ok) colour, and a success can show as Error

**File:** `src/overlay/app.rs:2311-2317`, `src/overlay/shell.rs:193-209` (`event_from`)
**Issue:** `event_from` takes the message's kind from
`apply_ok_state = apply_state == Applied && changed_on_disk.is_empty()`, not from whether the Apply
succeeded. `on_apply_failure` leaves the state as it was (D-17). `footer_actions` keeps Apply
enabled with nothing dirty (`HasSection | NoSection => writable`).

So after a successful Apply (state `Applied`), a second Apply with no edits can fail: an I/O
error, a OneDrive or antivirus lock, or an unreadable `styles.json` refusal. That failure keeps
`Applied`, and its error text is drawn in `StatusKind::Ok`. It is drawn as `Warning` if the text
happens to contain `RELOAD_INSTRUCTION`. This is exactly what D-17 forbids ("a failure must never be
presented as though it had applied"). On a one-line status that is truncated with "…", the colour is
the main signal.

The opposite also happens. A success is followed by a `styles.json` poll that sets
`style_rewrite_pending`, which flips the state to `Unapplied` the next frame. The success message
then turns red (Error).
**Fix:** Store the kind together with the message when it is produced, and stop inferring it later:
```rust
// field
apply_message: Option<(String, StatusKind)>,
// in run_apply
AllOutcome::Failed(message) => { self.apply_message = Some((message, StatusKind::Error)); ... }
// success paths: StatusKind::Ok, or Warning when the text carries RELOAD_INSTRUCTION
```
Then `event_from` passes the stored kind through. Add a test where Apply fails while the state is
`Applied` and assert the kind is `Error`.

## Warnings

### WR-01: Stale event messages hide the grammar gate, the load error and the out-of-date indicator

**File:** `src/overlay/app.rs:2096-2103`, `src/overlay/app.rs:1243-1260`, `src/overlay/app.rs:1653`, `src/overlay/app.rs:1007-1010`; `src/overlay/shell.rs:147-149`
**Issue:** Priority row 2 (event) beats rows 3-9. The UI-SPEC says an event "is cleared by the
next edit", but:
- `config_message` (for example "Reverted config changes." or "Could not save ff-settings.json")
  is not cleared by rule edits (`run_row_actions` clears only `apply_message`/`list_status`), and
  not on conceal (only `apply_message` and the sync message are). It stays across hide and reveal
  indefinitely. Meanwhile new grammar errors (row 3), and the "Filter out of date" indicator
  (row 9) that is the tab's main state signal, never appear.
- `list_status` set at startup by the `ff-settings.json` warning or by "Class groups unavailable
  (config.json)" stays until a row edit. When the filter failed to load there are no rows, so it
  never clears. The load-failure reason (row 4) is then hidden behind an Info line until Reload.
**Fix:** Clear `config_message` and `list_status` in the conceal branch, as done for
`apply_message`. Clear `config_message` in `run_row_actions` when `changed`. Alternatively, let a
standing Error (rows 3-4) outrank a non-error event:
`if event.kind != Error && grammar_gate.is_some() { ... }`.

### WR-02: Clicking the status-line jump on CONFIG, ECONOMY or ABOUT does nothing visible and leaves stale expand/scroll state

**File:** `src/overlay/app.rs:2360-2362`, `src/overlay/app.rs:2419-2425`
**Issue:** The grammar-gate and unknown-style lines are clickable on every tab (pointing hand).
The jump only sets `list.expanded`, `list.pending_scroll` and maybe clears `query`. It does not
switch to RULES, so nothing visible happens. The user's search query is also cleared off-screen,
and the next time they open RULES the list scrolls and expands unexpectedly.
**Fix:** Set `self.tab = ShellTab::Rules` (or `picked_tab = Some(ShellTab::Rules)`) when a jump is
taken, or make the line non-clickable (`jump: None`) when `self.tab != ShellTab::Rules`.

### WR-03: The Overwrite slot is gated on RULES writability, while the popup's Overwrite is not gated at all

**File:** `src/overlay/shell.rs:283-290`, `src/overlay/shell.rs:576-587`, `src/overlay/app.rs:1371-1381`
**Issue:**
- The bar's Overwrite is `enabled: actions.overwrite`, which needs `writable && blocking == 0`
  for the filter. Say the filter failed to load (or is Absent and clean) and only
  `styles.json`/`tiers.json` changed on disk. The user picks "Keep editing", the bar's Overwrite is
  then dead, and the popup cannot be reopened. Only Reload, which discards CONFIG edits, remains.
  v0.5's CONFIG footer always offered OVERWRITE after a refusal.
- In the other direction, the popup always offers an enabled Overwrite, even when grammar errors
  (`blocking > 0`) keep the bar's Overwrite disabled. The popup and the bar disagree.
**Fix:** Pass the computed `enabled` into `PopupSpec` (or skip the Overwrite button when it is
disabled). Derive the Overwrite enable from the combined config+filter decision, so a config-only
refusal stays overwritable, the same way `config_apply_enabled` treats `!has_filter`.

### WR-04: CONFIG Revert stays enabled during a changed-on-disk refusal

**File:** `src/overlay/shell.rs:278-282`
**Issue:** RULES Revert is disabled while `needs_reload` (`footer_actions`: `!needs_reload`). CONFIG
Revert is `config.dirty` with no such guard. v0.5 showed only RELOAD/OVERWRITE on CONFIG after a
refusal, and the removed `footer_buttons` test pinned that. A Revert here leaves the refusal pending
against a now-clean config, so the Overwrite slot still writes the filter over the outside change.
**Fix:** `ShellTab::Config => config.dirty && !needs_reload,` plus a test.

### WR-05: The ABOUT footer string has an embedded run of spaces

**File:** `src/overlay/shell.rs:464`
**Issue:** The literal is `"... a free third-party              tool for Path of Exile 2 players."`.
It contains 14 spaces, left over from a lost `\` line continuation, and is rendered as-is in the
disclaimer.
**Fix:** `"Game art and fonts belong to their owners. FerrisFilter is a free third-party \
           tool for Path of Exile 2 players."`, or put it on one line.

### WR-06: `theme::WARM_ACCENT` was remapped to near-black, so the style on/off switch track and the tier edge bar disappear

**File:** `src/overlay/theme.rs:50`, used at `src/overlay/widgets.rs:168-172` and `src/overlay/config_view.rs:524`
**Issue:** `WARM_ACCENT` is now `pal::INPUT_FILL` (`#070707`). `slide_switch` paints the "on"
track with it and the "off" track with `theme::INPUT_FILL` (`#090909`), so the two are
indistinguishable. This is the CONFIG Show/Hide switch, which hides whole groups of styled rules,
and only the knob position now tells the states apart. The tier "edge" marker bar at
`config_view.rs:524` is painted near-black on a near-black ground, so it is invisible.
**Fix:** Map `WARM_ACCENT` to a visible accent (for example `pal::GOLD` or a dimmed `pal::WARN`),
or give `slide_switch` an explicit on-track colour.

### WR-07: The screenshot-mode safety guard trusts `TEMP`/`TMP`, and `stage_filter` can then overwrite a real `dynamic.filter`

**File:** `src/overlay/screenshot.rs:128-160`
**Issue:** "Under temp" is decided by `std::env::temp_dir()`, which on Windows comes from
`TMP`/`TEMP`, the same kind of env var the guard is meant to distrust. Suppose
`TEMP=C:\Users\jonat` (or `C:\`) and `USERPROFILE` is unchanged. The real game folder then passes
the guard, and `stage_filter` runs `std::fs::copy(src, <real game dir>/dynamic.filter)`, a silent
overwrite of the user's store with no `.ffbak`. It is debug-only and needs a misconfiguration, but
the module doc promises the mode "can never read or write the real game folder".

Also, `stage_filter` always writes `dynamic.filter`, while the app loads
`settings.filter_name()` from `ff-settings.json` in the temp home. If that file names another
filter, the staged file is never shown.
**Fix:** Also refuse when the resolved game folder equals `dynamic::default_destination()` computed
from the *real* profile (`dirs::home_dir`/`SHGetKnownFolderPath`), or require the temp home to
contain a marker file created by the screenshot script. In `stage_filter`, refuse to overwrite an
existing file outside a directory the run created. Stage under the configured filter name.

### WR-08: Debug builds of `ff` refuse to start on any unrecognised argument

**File:** `src/overlay/mod.rs:40-50`, `src/overlay/screenshot.rs:67`
**Issue:** `screenshot::parse` returns `Err` for any argument that is not a screenshot flag, and
`launch` then prints and returns without starting the overlay. A release `ff` ignores arguments.
So a debug build started with an extra argument (a shortcut, a launcher, `cargo run -- foo`) silently
does nothing. That diverges from release behaviour and hides real-launch issues during development.
**Fix:** Only treat the arguments as screenshot arguments when `--screenshot` is present. Otherwise
return `Ok(None)` and fall through to the normal launch.

## Info

### IN-01: Dead or duplicated code left by the migration

**File:** `src/overlay/win.rs:175,682` (`work_area`, the SPI_GETWORKAREA primary-only path, now unused outside a test); `src/overlay/layout.rs:11-18` (`STRIPE_WIDTH`, `dock_rect*` right-edge docking, unused); `src/overlay/config_view.rs:152-160` (`ConfigViewCtx::{changed_on_disk, apply_enabled, rules_blocking, filter_damaged, indicator}` are written but no longer read); `src/overlay/config_view.rs:229` (`delete_prompt` is test-only; `app.rs:1316-1329` reimplements it); `src/skin/window.rs:55` (`primary_work_area_px` duplicates `win::monitor_work_area_px`).
**Fix:** Remove them, or route `popup_spec_for(DeleteStyle)` through `config_view::delete_prompt`
so the two cannot drift apart.

### IN-02: Stale documentation

**File:** `src/overlay/app.rs:3` ("docking the stripe to the right screen edge"; it now docks left through `win::dock_left`); `src/skin/icons.rs:22` (`Padlock` is documented as "Locked rule (economy)" but marks `can't enable`).
**Fix:** Update the comments.

### IN-03: "Last sync" applies today's UTC offset to an old timestamp; magic number in the time-zone branch

**File:** `src/overlay/shell.rs:337-349`, `src/overlay/win.rs:279-296`
**Issue:** A `prices.json` written before a DST change displays one hour off.
`id == 2` should be the named `TIME_ZONE_ID_DAYLIGHT` constant.
**Fix:** Use `SystemTimeToTzSpecificLocalTime` / `FileTimeToLocalFileTime` on the mtime. Import the
constant.

### IN-04: The Delete-rule popup falls back to "Rule 0: " when its row vanished

**File:** `src/overlay/app.rs:1309-1313`
**Issue:** If `confirm_delete` names an id that no longer exists, the popup still opens with an
empty label.
**Fix:** Clear `list.confirm_delete` when `working.row(id)` is `None`, rather than showing the
popup.

### IN-05: A test hook writes into egui temp memory every frame in production

**File:** `src/skin/popup.rs:174-176`
**Issue:** `data_mut(insert_temp(... "ff_modal_rects"))` runs on every frame of every modal, only
for tests.
**Fix:** Gate it with `#[cfg(test)]`.

---

_Reviewed: 2026-10-16_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
