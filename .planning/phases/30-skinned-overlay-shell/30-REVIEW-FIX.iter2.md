---
phase: 30-skinned-overlay-shell
fixed_at: 2026-10-16T00:00:00Z
review_path: .planning/phases/30-skinned-overlay-shell/30-REVIEW.md
iteration: 1
findings_in_scope: 10
fixed: 10
skipped: 0
status: all_fixed
---

# Phase 30: Code Review Fix Report

**Fixed at:** 2026-10-16
**Source review:** .planning/phases/30-skinned-overlay-shell/30-REVIEW.md
**Iteration:** 1

**Summary:**
- Findings in scope: 10 (2 critical, 8 warning; Info out of scope)
- Fixed: 10
- Skipped: 0

Verification after all fixes: `cargo test` green (lib 1139 passed, all integration suites ok),
`cargo fmt --all --check` clean, clippy count 126 (unchanged), `tests/fixtures` untouched,
`cargo check --release --bins` clean.

## Fixed Issues

### CR-01: CONFIG-tab Reload discards unapplied CONFIG edits without confirmation and never reloads the filter it was enabled for

**Files modified:** `src/overlay/app.rs`
**Commit:** 86a84a1
**Applied fix:** New pure `bar_reload_target(tab, changed_on_disk)`: a bar Reload on CONFIG goes
to `ConfigAction::Reload` only after a changed-on-disk refusal (v0.5 semantics); otherwise it runs
the shared RULES reload (`reload_pressed` -> `reload`), which re-reads the filter and keeps CONFIG
edits unless the root moved or the config had failed to load. Inline "Reload from disk" is
unchanged. Test: `bar_reload_on_config_reloads_the_filter_unless_a_refusal_is_pending`.
Status: fixed: requires human verification (logic change in the reload routing).

### CR-02: An Apply failure can show in the success (Ok) colour, and a success can show as Error

**Files modified:** `src/overlay/app.rs`, `src/overlay/shell.rs`
**Commit:** e90f6c0
**Applied fix:** `apply_message` is now `Option<(String, StatusKind)>`; a failure is stored as
`Error`, every success via `applied_message()` / `shell::apply_success_kind` (Ok, or Warning when the
text carries the Refresh instruction). `event_from` passes the stored kind through and no longer
takes `apply_ok_state`. The conceal and landed-reload paths keep the kind. Test:
`apply_failure_after_a_success_is_an_error` (real refused Apply with state `Applied`).
Status: fixed: requires human verification (status colour logic).

### WR-01: Stale event messages hide the grammar gate, the load error and the out-of-date indicator

**Files modified:** `src/overlay/app.rs`
**Commit:** db42ebd
**Applied fix:** Conceal now goes through `clear_events_on_conceal`, which also clears
`config_message` and `list_status` (the reload interim-line exception is kept). A rule edit in
`run_row_actions` clears `config_message`. Test: `conceal_and_rule_edits_clear_stale_event_messages`.

### WR-02: Clicking the status-line jump on CONFIG, ECONOMY or ABOUT does nothing visible

**Files modified:** `src/overlay/app.rs`
**Commit:** 470ce79
**Applied fix:** Taking the jump sets `self.tab = ShellTab::Rules` before expanding/scrolling.

### WR-03: The Overwrite slot is gated on RULES writability, while the popup's Overwrite is not gated at all

**Files modified:** `src/overlay/apply.rs`, `src/overlay/shell.rs`, `src/overlay/app.rs`
**Commit:** 131b68b
**Applied fix:** New `apply::overwrite_enabled` = needs_reload && (filter Overwrite ||
`config_apply_enabled` with the changed-on-disk guard lifted), so a config-only refusal (filter
failed to load, or absent and clean) stays overwritable. `OverlayApp::overwrite_decision` computes it
for the bar slot and again right before the popup is drawn (so a refusal in the same frame is seen).
`PopupData::ChangedOnDisk` gained `overwrite`; when false the popup omits Overwrite (and its body
sentence) and `shell::changed_on_disk_choice` maps the clicked index back. Tests:
`overwrite_covers_config_only_refusals`, `changed_on_disk_popup_drops_a_disabled_overwrite`.
Status: fixed: requires human verification (enable logic).

### WR-04: CONFIG Revert stays enabled during a changed-on-disk refusal

**Files modified:** `src/overlay/shell.rs`
**Commit:** 12a0d75
**Applied fix:** `ShellTab::Config => config.dirty && !needs_reload`. Test:
`config_revert_is_off_during_a_refusal`.

### WR-05: The ABOUT footer string has an embedded run of spaces

**Files modified:** `src/overlay/shell.rs`
**Commit:** 10dfb46
**Applied fix:** Restored the `\` line continuation.

### WR-06: `theme::WARM_ACCENT` was remapped to near-black, so the switch track and the tier edge bar disappear

**Files modified:** `src/overlay/widgets.rs`, `src/overlay/config_view.rs`
**Commit:** 5888173
**Applied fix:** Kept `WARM_ACCENT = pal::INPUT_FILL` (the UI-SPEC button-fill mapping, pinned by a
theme test). `slide_switch` now paints its on-track `pal::GOLD` (as UI-SPEC section 4 specifies), and
the tier-error edge bar uses `theme::WARM_ACCENT_TEXT` (`pal::WARN`, the error text's colour).

### WR-07: The screenshot-mode safety guard trusts `TEMP`/`TMP`, and `stage_filter` can then overwrite a real `dynamic.filter`

**Files modified:** `src/overlay/screenshot.rs`, `CLAUDE.md`
**Commit:** 78ba579
**Applied fix:** `paths_under_temp` now also refuses a temp folder with no `Temp`/`tmp` path
component (`temp_looks_real`), so `TEMP=C:\Users\me` or `C:\` cannot make the real game folder pass.
`stage_filter` stages under the filter name the temp home's `ff-settings.json` selects, and through
`stage_into` never overwrites an existing file in a game folder it did not create (it marks folders
it creates with `.ff-screenshot-staged`). CLAUDE.md's screenshot-mode description updated. Tests:
`a_temp_folder_without_a_temp_component_is_refused`,
`staging_never_overwrites_a_file_in_a_folder_it_did_not_create`. Note: a temp game folder left by an
earlier run (no marker) with an existing filter now refuses; delete that temp folder to re-stage.

### WR-08: Debug builds of `ff` refuse to start on any unrecognised argument

**Files modified:** `src/overlay/screenshot.rs`
**Commit:** be95c96
**Applied fix:** `parse` returns `Ok(None)` (normal launch) unless `--screenshot` is present; flags
are validated only in screenshot mode. Tests updated.

---

_Fixed: 2026-10-16_
_Fixer: Claude (gsd-code-fixer)_
_Iteration: 1_
