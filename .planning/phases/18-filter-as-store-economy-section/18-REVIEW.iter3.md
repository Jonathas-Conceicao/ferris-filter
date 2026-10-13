---
phase: 18-filter-as-store-economy-section
reviewed: 2026-10-13T01:37:30Z
depth: standard
iteration: 2
files_reviewed: 19
files_reviewed_list:
  - src/paths.rs
  - src/safe_write.rs
  - src/filter/section.rs
  - src/filter/parse.rs
  - src/filter/mod.rs
  - src/dynamic.rs
  - src/compile.rs
  - src/emitter.rs
  - src/cli.rs
  - src/run.rs
  - src/lib.rs
  - src/overlay/app.rs
  - src/overlay/apply.rs
  - src/overlay/mod.rs
  - tests/dynamic_filter.rs
  - tests/cli_errors.rs
  - .gitignore
  - CHANGELOG.md
  - CLAUDE.md
findings:
  critical: 0
  warning: 1
  info: 11
  total: 12
status: issues_found
---

# Phase 18: Code Review Report (iteration 2)

**Reviewed:** 2026-10-13T01:37:30Z
**Depth:** standard (phase diff `c9889a5..HEAD`; fix commits `f800e6f..f6a14b3` checked in detail)
**Files Reviewed:** 19
**Status:** issues_found

## Summary

This is a re-review after the iteration-1 fixes (CR-01, WR-01..WR-08, plus IN-01). I checked each
fix against its finding, then looked for regressions the fixes introduced and for defects the
first pass missed.

`cargo test` passes: 515 lib unit tests plus every integration binary, run with
`CARGO_TARGET_DIR` set to the temp target. `cargo clippy --all-targets` adds one new warning in a
phase file (IN-04). Every other clippy warning in these files is pre-existing style lint.

**Fix verification. All nine fixes are correct.**

- **CR-01:** `create_backup_with` writes the backup to a temp file, syncs it, then
  `persist_noclobber`s it. A failure partway drops and deletes the temp file, a lost race keeps
  the other writer's backup, and a non-file path at the backup name is an error. The injected
  half-write test proves no partial `.ffbak` and no stray temp file remain, and that the next
  write still makes a full backup.
- **WR-01:** `sync_all` now runs before persist. I confirmed in tempfile 3.27 that `persist` and
  `persist_noclobber` clear `FILE_ATTRIBUTE_TEMPORARY` before `MoveFileExW`, so the published file
  is a normal file.
- **WR-08:** The read-only check runs before the retry check, so a read-only target is never
  retried or reported as `Busy`. The OS error codes are now `#[cfg(windows)]`, and the retry tests
  are gated to match.
- **WR-02:** `footer_actions` offers RELOAD for no root, `Damaged`, `LoadFailed` or
  `needs_reload`, and `reload()` re-resolves the app root. The tests cover both the reload states
  and the clean states that hide it.
- **WR-03:** `lead_start` stops at an economy marker, and `block_meta`/`meta_get` read only
  `leading()`, so a marker can no longer become a block's metadata.
  - I traced the two other ways a marker could be absorbed. The live trailing run and
    `disabled::detect` both treat `#ff:economy-*` as prose, because its inner text is not a known
    keyword and has no body indentation. So `find`'s `owned()` check is still the complete
    "inside a block" test.
  - `filter_roundtrip` still passes.
- **WR-04:** `styles.json` is now the only root marker. The test proves that an unrelated
  `config.json` above the exe is passed over.
- **WR-05:** `under_root` joins relative `--tiers`/`--styles` onto the app root. The new
  integration test runs from `game/`, so it would have failed before the fix.
- **WR-06:** `.gitignore` again ignores `/dynamic.json`, and also ignores `*.ffbak`.
- **WR-07:** The default path is restored and the stale "store" wording is gone from the
  `styles.json` row and the CHANGELOG.

One fix introduced a regression. The WR-02 change widened when RELOAD appears, but the overlay
paragraph in `CLAUDE.md` still describes the old behaviour (WR-01 below). The project instructions
require that this section stay accurate. The other findings are Info items: six iteration-1 Info
items the fixer did not address (all still present), plus a few small new ones.

No test can reach the real game folder. Every spawn of `apply` sets `FERRISFILTER_HOME` and
`USERPROFILE` to a temp dir, including the new override test and `apply_obsolete_flags_are_rejected`.

## Warnings

### WR-01: The CLAUDE.md overlay note still says RELOAD appears only after a changed-on-disk refusal

**File:** `CLAUDE.md:57-58`
**Issue:** The WR-02 fix (commit f5e424b) changed `footer_actions` in four ways:

- RELOAD now also appears when no app root was found, when the markers are damaged (`Damaged`),
  and when the filter could not be loaded (`LoadFailed`).
- `reload()` now re-resolves the app root.

The `app.rs` module doc was updated, but the CLAUDE.md overlay paragraph still says:

> **RELOAD** (re-reads the file after an APPLY was refused because it changed on disk)

That no longer matches the code. A reader following CLAUDE.md would conclude that a filter with
damaged markers or a failed load can only be recovered by quitting from the tray, which is the
dead end WR-02 removed. CLAUDE.md requires this reference section to stay accurate.

**Fix:** Reword the RELOAD clause, for example:
```markdown
**RELOAD** (re-resolves the FerrisFilter folder and re-reads the file; offered after an APPLY was
refused because the file changed on disk, and whenever the folder was not found, the filter could
not be loaded, or its economy markers are damaged)
```

## Info

### IN-01: The CLAUDE.md `apply` row has an unescaped `|` inside a table cell

**File:** `CLAUDE.md:34`
**Issue:** `[--add-economy | --remove-economy]` sits inside a GFM table row. GFM splits cells on
`|` even inside a code span, so the row gets an extra cell and its excess cells are dropped. In
any rendered view (GitHub, IDE preview), everything after `--add-economy` disappears: the default
path, the flag semantics and the app-root paragraph. The `prices.json` row already escapes its pipe
correctly (`<name\|slug>`).
**Fix:** Write `[--add-economy \| --remove-economy]`.

### IN-02: A backup made by a call that then fails is never reported, and the v0.4 notice can promise a backup that is not made

**File:** `src/safe_write.rs:210-266`; `src/overlay/apply.rs:148-153`; `src/overlay/app.rs:549-554`
**Issue:**

- **Unreported backup.** Step 4 creates `.ffbak` before the target write. If the persist then
  fails (`ReadOnly`, `Busy`, `Unwritable`), the call returns `Err` and the new backup is never
  reported. The next successful write sees the existing `.ffbak` and reports `backup: None`, so
  the "Kept the original as ..." line never appears for that file. The new
  `read_only_target_is_reported_as_read_only_not_busy` test exercises exactly this path: it
  creates `x.filter.ffbak` and does not assert anything about it.
- **v0.4 notice.** The notice says "a .ffbak backup of the original is kept" unconditionally. If a
  stale `dynamic.filter.ffbak` already exists, the v0.4 original is not backed up at all, because
  the backup is never overwritten.

**Fix:** Do the read-only pre-check before the backup step, so the most likely persist failure
leaves nothing behind. Make the migration notice conditional on `!backup_path(path).exists()`, or
say "unless a .ffbak already exists".

### IN-03: The read-only classification can mislabel unrelated errors off Windows

**File:** `src/safe_write.rs:241-246`
**Issue:** On Unix, `rename(2)` ignores the target file's mode, so a 0444 target never causes the
persist failure. But any persist error (`EXDEV`, `ENOSPC`, `EACCES` on the directory) with a 0444
target is still reported as "the file is read-only; clear its read-only attribute". This is low
impact because the project targets Windows.
**Fix:** Gate the `is_read_only` branch behind `#[cfg(windows)]`, or only take it when
`raw_os_error() == Some(5)`.

### IN-04: Unused `use super::*` in the `app.rs` test module

**File:** `src/overlay/app.rs:580`
**Issue:** After the JSON-store tests were removed, the remaining test uses only `egui`.
`cargo clippy --all-targets` reports `unused import: super::*`.
**Fix:** Delete the import.

### IN-05: The CLI prints "converting ..." before a write that can still fail, and prints the reload hint on a no-op (carried over: iteration-1 IN-02)

**File:** `src/run.rs:210-215, 240`
**Issue:** The conversion line is printed before `apply_file`. "Reload it in-game" is printed even
when the outcome is `Unchanged`, although the overlay's `unchanged_message` deliberately omits it.
**Fix:** Print the conversion line from `report.migrated` after success, and print the reload hint
only for `Written`.

### IN-06: The `FilterNotFound` help text is wrong for `--remove-economy` (carried over: iteration-1 IN-03)

**File:** `src/dynamic.rs:57-60, 223-225`
**Issue:** `apply --remove-economy` on a missing file says "Use --add-economy to create it with an
economy section."
**Fix:** Use a neutral help text, or a separate variant for removal.

### IN-07: Stale comments mention the JSON-store identity, which has no non-test caller (carried over: iteration-1 IN-04)

**File:** `src/compile.rs:320`; `src/emitter.rs:53`
**Issue:** The comments still say "The dynamic path passes ... `source_name: "dynamic.json"`" and
"Exists so the dynamic path can say `# GENERATED FROM dynamic.json`". No production path does
either any more.
**Fix:** Update or remove the comments. Consider removing `compile_as` until Phase 19 needs it.

### IN-08: The resolver tests depend on no `styles.json` sitting above `%TEMP%` (carried over: iteration-1 IN-05)

**File:** `src/paths.rs:181-201`
**Issue:** `falls_back_to_cwd_with_marker` and `cwd_without_marker_is_not_found_and_lists_attempts`
pass a `tempdir()` as `exe_dir`. WR-04 narrowed the marker to `styles.json`, so this is less
likely to fail, but a stray `C:\Users\<u>\styles.json` would still break both tests.
**Fix:** Nest `exe_dir` inside a tempdir and assert that no ancestor holds `styles.json` first, or
restructure the tests so they do not need a real ancestor chain.

### IN-09: `remove` is not the inverse of `add` for an unterminated last line (carried over: iteration-1 IN-06)

**File:** `src/filter/section.rs:193-197, 446`
**Issue:** `add` gives an unterminated last line an EOL, and `remove` keeps it, so the round trip
changes one byte. `refresh_idempotent_all_shapes` still builds `nofinal` with a no-op
`format!("{}{}{}", .., "", "")`.
**Fix:** Document and assert the one-byte difference, or restore `Eol::None`. Simplify the
`nofinal` expression.

### IN-10: `LoadedFilter::load` parses the file only to validate UTF-8, then validates it again (carried over: iteration-1 IN-07)

**File:** `src/dynamic.rs:130-132`
**Issue:** The `crate::filter::parse(&bytes)` result is discarded, and `String::from_utf8(bytes.clone())`
repeats the same check.
**Fix:** Drop the `parse` call.

### IN-11: Replace-by-rename breaks a symlinked or hard-linked filter, and integration-test workspaces are never cleaned up (carried over: iteration-1 IN-08, plus a test-hygiene note)

**File:** `src/safe_write.rs:219-266`; `tests/dynamic_filter.rs:27-46`
**Issue:**

- **Linked filters.** The atomic replace swaps the directory entry, so a symlinked or hard-linked
  `dynamic.filter` loses its link after the first Apply. This is undocumented.
- **Test workspaces.** `workspace()` creates `%TEMP%\ferrisfilter_dyn_<tag>_<nanos>` directories
  with `fs::create_dir_all` and never removes them. Every run of the suite leaves about 20
  directories behind.

**Fix:** Document the link behaviour, or `fs::canonicalize` a symlink target before choosing the
temp directory. Use `tempfile::TempDir`, which is removed on drop, for the test workspaces.

---

_Reviewed: 2026-10-13T01:37:30Z_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
