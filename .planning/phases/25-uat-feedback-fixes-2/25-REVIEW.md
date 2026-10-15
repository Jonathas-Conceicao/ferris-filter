---
phase: 25-uat-feedback-fixes-2
reviewed: 2026-10-14T00:00:00Z
depth: standard
files_reviewed: 15
files_reviewed_list:
  - src/overlay/apply_all.rs
  - src/overlay/app.rs
  - src/overlay/config_state.rs
  - src/overlay/config_view.rs
  - src/overlay/apply.rs
  - src/safe_write.rs
  - src/dynamic.rs
  - src/compile.rs
  - src/models/config.rs
  - src/overlay/body.rs
  - src/overlay/rule_list.rs
  - src/overlay/rules.rs
  - src/overlay/grammar.rs
  - src/filter/keywords.rs
  - src/overlay/mod.rs
findings:
  critical: 1
  warning: 3
  info: 5
  total: 9
status: issues_found
---

# Phase 25: Code Review Report

**Reviewed:** 2026-10-14
**Depth:** standard
**Files Reviewed:** 15
**Status:** issues_found

## Summary

Scope: the phase-25 diff (`03b7ccf..HEAD`) in the 15 listed files, with callers traced into
`styled.rs`, `widgets.rs` and `dynamic::prepare_edited`. `cargo test --lib`: 861 passed, 0 failed.

What checks out:
- **Validate-then-write.** `apply_all::prepare` does every check that can refuse (config validity,
  styled text, economy pricing, grammar gate, markers on Refresh/Add/Remove, changed-on-disk) before
  any write. Baselines move only in `commit_with` after `write_all_or_none` returns `Ok`.
- **Three-target rollback.** `write_all_or_none_with` restores the files it already replaced, in
  reverse order, from the in-memory loaded bytes. A `None` baseline is restored by deleting the file.
  Tests cover this at both levels (`three_targets_third_persist_fails_restores_first_two`,
  `third_rename_failure_rolls_back_all`).
- **Rendering with pending config.** `text_with_style_set(c.pending_styles())` and
  `economy_body_with(.., pending_styles, pending_tiers)` are used. When live prices are missing,
  the fallback pair still ignores pending tiers, as documented.
- **In-game reload only after success.** `chat::decide` runs only on `AllOutcome::Applied` with a
  filter path. Config-only applies never reload.
- **PARSE-04.** A clean config keeps its loaded bytes (`prepare_write` returns the baseline bytes
  when nothing changed). The sockets slider uses `SliderClamping::Never` and edits only the count,
  passing colour letters through unchanged. The name box only touches the `# ff:name` line, and
  `SetName` trims. `move_custom` swaps two customs and leaves every built-in where it is.
- **Exceptional** is removed from `KEYWORDS`/grammar groups. It stays a raw row and the gate ignores it.

The main defect: when CONFIG has **no edits**, the config files still join the changed-on-disk
guard and the write. A hand edit of `styles.json`/`tiers.json` therefore blocks every Apply, and
OVERWRITE silently writes the stale in-memory copy back over it.

## Narrative Findings (AI reviewer)

## Critical Issues

### CR-01: A clean CONFIG still guards and writes styles.json/tiers.json, so an outside edit blocks every Apply and OVERWRITE reverts it with no backup

**File:** `src/overlay/apply_all.rs:226-256` (with `src/overlay/config_state.rs:404-426`, `src/overlay/app.rs:868-881`)
**Issue:** `prepare` always builds a `ConfigPart` when `ConfigState` loaded, whether or not it has
edits. It compares both files with the load baselines and adds both as write targets:
- `prepare_write` on a clean state returns `styles_new = styles_bytes` / `tiers_new = tiers_bytes`
  (the bytes read at startup).
- If the author hand-edits `tiers.json` or `styles.json` while the overlay runs (`tiers.json` is
  modified in the current working tree), the next Apply is refused with
  `Changed on disk: tiers.json`. This happens even for a RULES-only edit, even though the author
  never touched CONFIG. Before this phase, `poll_styles` picked up an outside `styles.json` edit and
  the next RULES Apply used it. `poll_styles` still loads the new styles into the RULES view, so the
  rules tab shows the hand edit as live while Apply refuses it.
- Pressing OVERWRITE (tooltip: "Replace the file with your edits") sets the guard to the current
  disk bytes. The target's `new` is then the stale startup bytes, `loaded != new`, and
  `write_all_or_none` writes the old content over the hand edit. The filter is also rendered with
  those stale pending styles/tiers. Config files get no `.ffbak`, so the hand edit is unrecoverable.
  The user had no CONFIG edits that could justify replacing the file.

**Fix:** A config file with no pending change must not take part in the write. It should follow the
disk instead:
```rust
// apply_all::prepare, step 3: only files whose value changed are targets/guarded.
let styles_target = write.styles_changed();
let tiers_target  = write.tiers_changed();
if styles_target && styles_now.as_deref() != Some(styles_base) { changed.push(DiskFile::Styles); }
if tiers_target  && tiers_now.as_deref()  != Some(tiers_base)  { changed.push(DiskFile::Tiers); }
// ...and in commit_with push a Target only for *_target files.
```
In `run_apply` (and in `poll_styles`), when `!config.is_dirty()` and either file's bytes differ
from `baselines()`, re-run `ConfigState::load(root)` before rendering. That way a clean config
always renders and adopts what is on disk. Add a test: clean config, outside `tiers.json` edit,
RULES-only Apply succeeds and leaves `tiers.json` byte-identical, and OVERWRITE never rewrites it.

## Warnings

### WR-01: CONFIG Apply writes a filter whose economy markers are damaged, bypassing the RULES gate and the locked "damaged markers" refusal

**File:** `src/overlay/apply.rs:370-380`, `src/overlay/apply_all.rs:173-215`
**Issue:** `config_apply_enabled` treats any state other than `LoadFailed` as "has a filter",
including `FilterState::Damaged`. For a damaged filter, `WorkingFilter::economy_plan()` returns
`Keep` (`rules.rs:619-627`), and `prepare_edited(.., EconomyOp::Keep)` never inspects the markers.
So CONFIG APPLY (enabled when `blocking == 0`) rewrites the damaged filter: style rewrites with the
pending set go in, and the economy section is **not** re-priced with the new tiers. The result is
then reported as a success with an in-game reload. The RULES footer correctly refuses this
(`footer_actions`: `Damaged => writable = false`), and 25-CONTEXT lists "damaged economy markers"
as a refusal for the combined Apply.
**Fix:** Refuse in `prepare` when the loaded filter's section is damaged, whatever the plan:
```rust
if let Err(e) = input.loaded.has_section() {
    return Err(ApplyAllError::Invalid(e.to_string()));
}
```
Also make `config_apply_enabled` take `state == Damaged` as disabled when a filter part would be
written, with a tooltip. Add a test: CONFIG edit + damaged-marker filter leaves all three files
untouched.

### WR-02: The one-time `.ffbak` is created before the all-or-none guard, so "Nothing was applied" can still leave a new file behind

**File:** `src/overlay/apply_all.rs:296-304`
**Issue:** `commit_with` calls `safe_write::ensure_backup` before `write(&targets)`. If
`write_all_or_none` then refuses (`NothingWritten`: read-only or locked `styles.json`/`tiers.json`,
a missing destination, or a commit-time `ChangedOnDisk`), the message says "Nothing was applied"
but a `.ffbak` was created in the game folder. That uses up the one-time backup slot.
- With a commit-time race (the filter changed between `prepare` and `commit`), the backup holds the
  stale load bytes, not what is on disk.
- A later OVERWRITE of a real outside edit then finds the backup already present and does not
  preserve that outside edit. The old `write_guarded` checked the guard before backing up (step 2
  before step 4).
- `any_refusal_writes_nothing` only exercises `prepare` refusals, so the tests miss this.

**Fix:** Run the read-only guard first and back up just before the persist phase. Either:
- add a `before_persist` hook to `write_all_or_none_with` that runs after stage (c) and preflight
  (d), or
- re-read the filter in `commit_with` and only call `ensure_backup` when the current bytes equal
  `f.guard`.

Extend the refusal test to a commit-time `NothingWritten` (read-only `tiers.json`) and assert there
is no `.ffbak`.

### WR-03: CONFIG load-error "Reload from disk" discards RULES edits and does not reload the config after a filter refusal

**File:** `src/overlay/app.rs:1278-1290` (with `src/overlay/config_view.rs:1289-1301`, `src/overlay/app.rs:359-385`)
**Issue:** When `ConfigState` failed to load, the CONFIG tab shows only `show_load_error`, whose
button emits `ConfigAction::Reload`. If `changed_on_disk` is non-empty, that handler calls
`reload_changed()`, which reloads only the files in the list. Example sequence:
1. Config is `Err`.
2. The author edits rules, and the filter is changed outside.
3. RULES Apply is refused with `[Filter]`. The config is not guarded, since it is `None`.
4. The author fixes `styles.json` and presses "Reload from disk" on CONFIG.
5. The filter is reloaded and every RULES edit is discarded. The config is **not** reloaded, because
   `Styles` is not in the list, so it stays `Err`.

The button destroys the other tab's edits and does not do what it says.
**Fix:** In the `Reload` arm, always reload the config when `self.config.is_err()`, and only
re-read the files in `changed_on_disk` that belong to CONFIG:
```rust
ConfigAction::Reload => {
    let config_changed = self.changed_on_disk.iter()
        .any(|f| matches!(f, DiskFile::Styles | DiskFile::Tiers));
    if self.config.is_err() || self.changed_on_disk.is_empty() || !config_changed {
        self.config = /* ConfigState::load(root) */;
        self.changed_on_disk.retain(|f| *f == DiskFile::Filter);
    } else {
        self.reload_changed();
    }
}
```

## Info

### IN-01: Dead code left from the separate CONFIG apply

**File:** `src/overlay/config_state.rs:372-378, 400-426, 469-478`
**Issue:**
- `prepare_write_inner(force)` is only ever called with `force = false`. The `force` path existed
  for the removed config-only overwrite.
- `ConfigState::can_apply` and `ConfigWrite::styles_changed`/`tiers_changed` are used only by tests.
- The footer now uses `apply_enabled`, `validate` and `is_dirty`.

**Fix:** Inline `prepare_write_inner` into `prepare_write` and drop `force`. Remove `can_apply`,
or reuse the `ConfigWrite` accessors for the CR-01 fix (they are exactly the "file has a pending
change" signal it needs).

### IN-02: `ApplyAllError::message` capitalises "The filter" mid-list, and the branch is unused

**File:** `src/overlay/apply_all.rs:70-84`
**Issue:** The `ChangedOnDisk` branch produces "Changed on disk since it was loaded: tiers.json, The
filter." Production goes through `app::refusal`, which uses `DiskFile::label` instead, so this
branch only serves callers that do not exist yet.
**Fix:** Use "the filter", or route through `DiskFile::label` / `apply::changed_on_disk_message`
so there is one copy of the sentence.

### IN-03: Reload toggle caption still says "after a RULES Apply"

**File:** `src/overlay/config_view.rs:41`
**Issue:** The Apply is combined now: CONFIG APPLY also writes the filter and sends
`/reloaditemfilter`. The caption (and `chat.rs`'s module doc) still ties the reload to the RULES tab.
**Fix:** "Sends /reloaditemfilter to the game after Apply. ..."

### IN-04: Redundant indicator flip in the MoveStyle handler

**File:** `src/overlay/app.rs:1249-1259`
**Issue:** `MoveStyle` sets `apply_state = Unapplied` by hand. The per-frame `combined_dirty`
check already does this for every CONFIG edit, and the other style/tier actions do not repeat it.
The two paths are inconsistent.
**Fix:** Drop the manual flip and rely on `combined_dirty`.

### IN-05: Staged "Add economy" block count comes from the on-disk styles and tiers, not the pending ones

**File:** `src/overlay/app.rs:1097-1109`
**Issue:** `stage_add_economy` counts blocks with `dynamic::economy_body` (disk `styles.json` and
`tiers.json`). The Apply that follows renders with `economy_body_with(pending ..)`. With pending
tier edits, the locked count shown in the footer can differ from what is written.
**Fix:** When `self.config` is `Ok`, count with `economy_body_with(.., c.pending_styles(),
c.pending_tiers())`.

---

_Reviewed: 2026-10-14_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
