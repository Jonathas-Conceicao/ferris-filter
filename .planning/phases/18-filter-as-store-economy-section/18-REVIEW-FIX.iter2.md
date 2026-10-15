---
phase: 18-filter-as-store-economy-section
fixed_at: 2026-10-12T00:00:00Z
review_path: .planning/phases/18-filter-as-store-economy-section/18-REVIEW.md
iteration: 1
findings_in_scope: 9
fixed: 9
skipped: 0
status: all_fixed
---

# Phase 18: Code Review Fix Report

**Fixed at:** 2026-10-12
**Source review:** .planning/phases/18-filter-as-store-economy-section/18-REVIEW.md
**Iteration:** 1

**Summary:**
- Findings in scope: 9 (CR-01, WR-01..WR-08; fix scope critical_warning)
- Fixed: 9
- Skipped: 0
- Also done at the orchestrator's request: IN-01 (`*.ffbak` git-ignored), folded into the WR-06 commit.

Verification: full `cargo test` passes (515 lib unit tests plus every integration binary), and
`cargo build --bins` is clean (both run with `CARGO_TARGET_DIR=C:/Users/jonat/AppData/Local/Temp/ff-target`).
`tests/fixtures/` and `tests/snapshots/` are byte-identical to `37ba59e`.

## Fixed Issues

### CR-01: A failed backup write leaves a partial `.ffbak` that permanently blocks the real backup

**Files modified:** `src/safe_write.rs`
**Commit:** f800e6f
**Applied fix:** New `create_backup_with` helper. If `.ffbak` is already a file, it is kept. If the path exists but is not a file, that is an error. Otherwise the original bytes are written to a temp file in the same directory, synced with `sync_all`, and published with `persist_noclobber`. If the write fails partway, the temp file is dropped and deleted, so no truncated `.ffbak` can take the final name. A lost `AlreadyExists` race keeps the other writer's backup. The byte writer can be injected, and the new test `failed_backup_write_leaves_no_partial_ffbak` uses that to fail a write halfway. It then asserts that neither a `.ffbak` nor a temp file is left behind, and that the next write produces a complete backup.

### WR-01: The temp file is never fsynced before the rename

**Files modified:** `src/safe_write.rs`
**Commit:** 7383722
**Applied fix:** `temp.flush()` is replaced with `temp.as_file().sync_all()` before persist, so the data is on disk before the rename publishes it.

### WR-08: A read-only target is retried and reported as "in use"; error codes are Windows-only

**Files modified:** `src/safe_write.rs`
**Commit:** 0fcfb3d
**Applied fix:** New `WriteError::ReadOnly` variant ("the file is read-only; clear its read-only attribute and apply again"). It is returned straight away, with no retry, when a persist fails and the target is read-only. `is_retriable` only recognises codes 5/32/33 under `#[cfg(windows)]` and returns `false` on other platforms. The two retry tests that use those codes are now `#[cfg(windows)]`. New test: `read_only_target_is_reported_as_read_only_not_busy`.

### WR-02: The overlay has no recovery path for a damaged or unloadable filter

**Files modified:** `src/overlay/apply.rs`, `src/overlay/app.rs`
**Commit:** f5e424b
**Applied fix:** `footer_actions` now offers RELOAD in more cases: `needs_reload || !root_ok || state is Damaged | LoadFailed`. Going beyond the review, a missing app root also gets RELOAD, because `reload()` now calls `paths::app_root()` again before re-reading the filter, which clears that dead end too. Tests updated to match: `footer_without_a_root_disables_every_file_action_but_offers_reload` and `footer_damaged_and_load_failed_offer_reload_only`. New test `footer_clean_states_hide_reload` confirms RELOAD stays hidden in clean states. The module doc for app.rs is updated as well.
**Status:** fixed: requires human verification. Logic change in the overlay's button rules, worth a live check in the overlay (damaged markers, then RELOAD, after fixing them by hand).

### WR-03: An economy marker that sits directly against a block becomes that block's `#ff:` metadata

**Files modified:** `src/filter/parse.rs`, `src/filter/section.rs`
**Commit:** b8bed35
**Applied fix:** I used the review's alternative fix. `lead_start` (parse.rs) now stops when it reaches an `#ff:economy-begin` / `#ff:economy-end` line, so a marker can never be part of a block's leading comments or metadata. A marker placed right against a header is still accepted; it stays trivia and needs no new error. Other `#ff:` lines (such as `#ff:name`) still attach to the block as before. New test: `a_marker_directly_above_a_block_is_not_that_blocks_metadata`. The `filter_roundtrip` suite (byte-identical round trip) still passes.
**Status:** fixed: requires human verification. This changes parser grouping, so please confirm it is acceptable for the Phase 19 rule list.

### WR-04: The root marker `config.json` is a generic file name

**Files modified:** `src/paths.rs`, `CLAUDE.md`
**Commit:** cc9c753
**Applied fix:** `is_root` now accepts `styles.json` only. 18-CONTEXT allowed "styles.json (or config.json)". I dropped `config.json` for two reasons: it is a common file name, and apply/the overlay never read it. The reasoning is written in the doc comment on `is_root`. The NotFound message and the module docs no longer mention config.json. The test that used to accept config.json is replaced by `config_json_alone_does_not_stop_the_walk_up`: an unrelated config.json above the exe is passed over, and the cwd that holds styles.json wins. The CLAUDE.md apply row now says only `styles.json` marks the root.

### WR-05: `apply --tiers` / `--styles` resolve against the cwd while every default input resolves against the app root

**Files modified:** `src/run.rs`, `src/cli.rs`, `tests/dynamic_filter.rs`, `CLAUDE.md`, `CHANGELOG.md`
**Commit:** e0a4e9d
**Applied fix:** New helper `under_root(root, p)`: absolute paths are used as given, relative ones are joined onto the app root. It is applied to both overrides. `--filter` still resolves against the working directory, and the docs say so. The CLI help for `--tiers` and `--styles` now states the rule. New integration test `apply_relative_overrides_resolve_against_the_app_root_not_the_cwd` runs from the `game` subfolder, so the cwd is not the root.

### WR-06: `dynamic.json` was removed from `.gitignore` while the user's per-user copy still sits in the repo

**Files modified:** `.gitignore`, `CLAUDE.md`, `CHANGELOG.md`
**Commit:** 9f05aa3
**Applied fix:** `/dynamic.json` is back in `.gitignore`, with a comment saying it is the obsolete v0.4 per-user store and is ignored only so a leftover copy is never committed. `*.ffbak` is ignored too (IN-01). CLAUDE.md (the `dynamic.filter` row) and the CHANGELOG `[Unreleased]` BREAKING entry now say a leftover `dynamic.json` is unused, safe to delete, and still git-ignored. They also note that `*.ffbak` is ignored. Checked with `git check-ignore`.

### WR-07: Doc errors in CLAUDE.md and CHANGELOG.md

**Files modified:** `CLAUDE.md`, `CHANGELOG.md`
**Commit:** f6a14b3
**Applied fix:**
- Restored the default path in the apply row: `%USERPROFILE%\OneDrive\Documentos\My Games\Path of Exile 2\dynamic.filter`.
- Reworded the `styles.json` row to "read by `generate` (beside the inputs) and by `apply`/the overlay (from the app root)". Removed the "saves the store" clause; a matching inline section is now described as dropped in memory.
- The Phase 17 CHANGELOG `styles` entry now notes that its `dynamic.json`/overlay-save/`--reset` wording is superseded by the filter-as-store change.

---

_Fixed: 2026-10-12_
_Fixer: Claude (gsd-code-fixer)_
_Iteration: 1_
