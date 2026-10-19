---
phase: 37-game-folder-resolver-picker
fixed_at: 2026-10-18T00:00:00Z
review_path: .planning/phases/37-game-folder-resolver-picker/37-REVIEW.md
iteration: 2
findings_in_scope: 2
fixed: 2
skipped: 0
status: all_fixed
---

# Phase 37: Code Review Fix Report

**Fixed at:** 2026-10-18
**Source review:** .planning/phases/37-game-folder-resolver-picker/37-REVIEW.md (re-review, its iteration 3)
**Iteration:** 2

**Summary:**
- Findings in scope: 2 (2 warnings; Info IN-01..IN-05 excluded by scope)
- Fixed: 2
- Skipped: 0

Full suite after both fixes: `cargo test --no-fail-fast` green (lib 1502 passed, every
integration file passed). It ran with `FERRISFILTER_HOME` / `FERRISFILTER_GAME_DIR` under
`%TEMP%\ff-fix37i2\{home,game}`, and both folders were still empty afterwards. rustfmt is clean on
the touched files. Clippy reports nothing new in the changed lines.

## Fixed Issues

### WR-01: Any settings warning, not only a filter fallback, suppresses the fresh-install filter pick

**Files modified:** `src/settings.rs`, `src/overlay/app.rs`, `CLAUDE.md`
**Commit:** 9e21743
**Applied fix:** Added `settings::load_detailed(root) -> Loaded { settings, warning, filter_fell_back }`.
`load` is now a thin wrapper around it, so its output is unchanged. `filter_fell_back` is true only
in three cases:
- the whole file is unusable;
- the `filter` key fails the salvage path;
- the stored name is invalid.

A bad `game_folder`, `reload_after_apply` or `collapsed_sections` alone leaves it false. `build`
and `reload` now check `!filter_fell_back` instead of `settings_warning.is_none()`. CLAUDE.md now
says that a warning about another key leaves the pick unchanged.

Tests:
- `settings::filter_fell_back_only_when_the_filter_name_did`: a 12-case table, which also checks
  that `load` equals `load_detailed`.
- `overlay::app::a_warning_about_another_key_still_selects_the_folders_filter`: the three cited
  keys, each with only `x.filter` in the folder, open `x.filter` at build and after Reload.

The two iteration-1 tests still pass. In them, an unusable file or an invalid name still opens
`dynamic.filter`.
**Status:** fixed: requires human verification (logic change in a condition).

### WR-02: The unsaved in-memory pick is saved by unrelated preference writes

**Files modified:** `src/settings.rs`, `src/overlay/app.rs`, `src/overlay/config_view.rs`, `CLAUDE.md`
**Commit:** 348db27
**Applied fix:** I took the reviewer's first option, which matches the documented intent and the
WR-04 promise:
- **New field.** `Settings::auto_filter` is marked `#[serde(skip)]`. `filter_name()` reads
  `filter`, then `auto_filter`, then `dynamic.filter`.
- **Where the pick goes.** `choose_unsaved_filter` now writes to `auto_filter`. Nothing serializes
  that field: not `save`, not the `NotFound` branch of `save_section_state`, and not the reload
  toggle's `save`.
- **Explicit picks.** `settings_with_filter` and `settings_with_folder` set `filter` and clear
  `auto_filter`, so `switch_to` and `apply_folder` save exactly as before.
- **Reload.** Reload still re-applies the pick in memory.
- **CLI.** It reads only the serialized `filter`, so it is unaffected.

**Decision I made (needs your review):** without a change to the dropdown, there was no way to
make the automatic pick the saved selection:
- the dropdown ignored a click on the active entry;
- a re-pick of the same folder returns before saving.

So `ConfigViewCtx` has a new `active_filter_selected` field. A click on the active entry is now
sent when that entry is unselected. `select_filter`'s `Noop` branch then saves the open filter
through the new `remember_open_filter`, which does not reload anything. A selected active entry
still ignores clicks, as before.

CLAUDE.md now says the automatic pick is not saved by another preference save, and that picking it
in the Filter list, or picking a folder, saves it.

Tests:
- `saving_the_reload_preference_does_not_select_the_open_filter`: the saved file has no `filter`
  key, also after Reload.
- `a_fold_that_creates_the_settings_file_does_not_select_the_open_filter`
- `picking_the_open_unselected_filter_saves_it`
- `picking_another_filter_or_a_folder_saves_the_selection`: a filter choice and a folder pick both
  still save.
- `settings::the_automatic_filter_is_read_but_never_saved`

`reload_keeps_the_open_unsaved_filter_among_several` now seeds `auto_filter` and asserts that
`filter` stays `None`.
**Status:** fixed: requires human verification (state-handling change plus the dropdown decision
above).

## Iteration 1 (previous run, already committed)

| Finding | Commit(s) | Summary |
|---|---|---|
| CR-01 Reload drops the fresh-install filter choice | 12d2fd4 | `choose_unsaved_filter` used by `build` and `reload` |
| WR-01 warnings say `dynamic.filter` while another filter is open | 4439152 | pick only after a clean load (narrowed by iteration 2 WR-01) |
| WR-02 refresh greyed while the folder is missing | 08829a6 | refresh live whenever a folder is known; same-folder re-pick re-lists |
| WR-03 CLAUDE.md `$1` corruption | 46d0d43 | `ff-settings.json` row and related lines restored and aligned |
| WR-04 `ff-cli apply` ignores the picked filter | c7f4ceb | `saved_choice_in` reads folder and filter; `default_filter_path` takes the name |
| WR-05 CLI silent fallback on an unusable root or key | b974d9c, dc1c80c | `saved_choice_from` errors on an unresolved, unpinned root; warnings go to stderr |

## Out of scope (Info, not attempted)

IN-01 to IN-05 of the re-review were not attempted, because `fix_scope` is `critical_warning`.

---

_Fixed: 2026-10-18_
_Fixer: Claude (gsd-code-fixer)_
_Iteration: 2_
