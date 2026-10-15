---
phase: 18-filter-as-store-economy-section
reviewed: 2026-10-12T23:00:00Z
depth: standard
iteration: 3
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
  warning: 0
  info: 8
  total: 8
status: clean
---

# Phase 18: Code Review Report (iteration 3)

**Reviewed:** 2026-10-12T23:00:00Z
**Depth:** standard (phase diff `c9889a5..HEAD`; fix commits `7ca5008`, `84272de`, `798ece6` checked in detail)
**Files Reviewed:** 19
**Status:** clean (no Critical or Warning findings; Info items remain)

## Summary

This re-review follows the iteration-2 fixes: WR-01, plus IN-01 and IN-04, which the orchestrator
added to the scope. I checked each fix against its finding and looked for regressions. The three
commits touch only `CLAUDE.md` and the `app.rs` test module, so they leave little room for a
regression.

`cargo test` passes: 515 lib unit tests plus every integration binary, run with
`CARGO_TARGET_DIR` set to the temp target. `cargo clippy --all-targets` reports no new warnings in
the phase files. The `unused import` warning in `app.rs` is gone. The other warnings are style lints
(`result_large_err` in `compile.rs`/`dynamic.rs`, `collapsible_if` in `paths.rs`, and two lints in
`emitter.rs`). None of them affects behaviour.

**Fix verification. All three fixes are correct.**

- **WR-01 (7ca5008):** The CLAUDE.md RELOAD clause now says RELOAD re-resolves the FerrisFilter
  folder and re-reads the file. It lists every state that offers it: a changed-on-disk refusal, no
  folder found, a failed load, and damaged markers. I traced this against the code:
  - `apply::footer_actions` (`src/overlay/apply.rs:120-125`) sets `reload` when
    `!root_ok || matches!(state, Damaged | LoadFailed) || needs_reload`.
  - `OverlayApp::reload` (`src/overlay/app.rs:227-233`) calls `paths::app_root()` again before it
    reloads the filter.

  The doc and the code agree. The one cosmetic leftover is that the edited paragraph line was not
  rewrapped (about 170 columns). That does not affect the rendered output.
- **IN-01 (84272de):** The `apply` row now reads `[--add-economy \| --remove-economy]`, so the GFM
  table cell no longer splits.
- **IN-04 (798ece6):** `use super::*` is removed from the `app.rs` test module. The remaining test
  uses only `egui`, and the test binary compiles and passes without the import.

The fix commits introduced no regressions. The eight Info items below are iteration-2 Info items
that were outside the fixer's scope. I checked that each one is still present in the code at the
cited lines. None of them affects correctness, so none blocks shipping.

## Info

### IN-01: A backup made by a call that then fails is never reported, and the v0.4 notice can promise a backup that is not made (carried over: iteration-2 IN-02)

**File:** `src/safe_write.rs:210-266`; `src/overlay/apply.rs:148-153`; `src/overlay/app.rs:549-554`; `CLAUDE.md:59-60`
**Issue:**

- **Unreported backup.** Step 4 creates `.ffbak` before the target write. If the persist then
  fails (`ReadOnly`, `Busy`, `Unwritable`), the call returns `Err` and the new backup is never
  reported. The next successful write finds the existing `.ffbak` and returns `backup: None`.
- **v0.4 notice.** The overlay notices (and now the CLAUDE.md paragraph, "a `.ffbak` backup is kept
  by that first write") promise a backup unconditionally. A stale `dynamic.filter.ffbak` means the
  v0.4 original is not backed up, because an existing backup is never overwritten.

**Fix:** Do the `is_read_only(path)` pre-check before step 4. Make the migration notice conditional
on `!backup_path(path).exists()`, or add "unless a .ffbak already exists".

### IN-02: The read-only classification can mislabel unrelated errors off Windows (carried over: iteration-2 IN-03)

**File:** `src/safe_write.rs:125-127, 241-246`
**Issue:** On Unix a 0444 target never causes a `rename` failure. But any persist error with a
read-only target is still reported as `ReadOnly`. Impact is low because the project targets
Windows.
**Fix:** Gate the branch with `#[cfg(windows)]`, or take it only when `raw_os_error() == Some(5)`.

### IN-03: The CLI prints "converting ..." before a write that can still fail, and prints the reload hint on a no-op (carried over: iteration-2 IN-05)

**File:** `src/run.rs:210-215, 240`
**Issue:** The conversion line is printed before `apply_file`, so it appears even when the write is
refused. "Reload it in-game" is printed even for `WriteOutcome::Unchanged`, although the overlay's
`unchanged_message` deliberately leaves it out.
**Fix:** Print the conversion line after success (from `report.migrated`), and print the reload hint
only for `Written`.

### IN-04: The `FilterNotFound` help text is wrong for `--remove-economy` (carried over: iteration-2 IN-06)

**File:** `src/dynamic.rs:57-60, 223-225`
**Issue:** `apply --remove-economy` on a missing file suggests "Use --add-economy to create it with
an economy section."
**Fix:** Use neutral help text, or add a separate variant for removal.

### IN-05: Stale comments describe a JSON-store identity that has no non-test caller (carried over: iteration-2 IN-07)

**File:** `src/compile.rs:320`; `src/emitter.rs:53`
**Issue:** The comments still say the dynamic path passes `source_name: "dynamic.json"` and emits
`# GENERATED FROM dynamic.json`. No production path does either.
**Fix:** Update or remove the comments. Consider dropping `compile_as` until Phase 19 needs it.

### IN-06: The resolver tests depend on no `styles.json` sitting above `%TEMP%` (carried over: iteration-2 IN-08)

**File:** `src/paths.rs:181-201`
**Issue:** `falls_back_to_cwd_with_marker` and `cwd_without_marker_is_not_found_and_lists_attempts`
walk the real ancestor chain of a `tempdir()`. A stray `styles.json` in `C:\Users\<u>\` or above
would break both tests.
**Fix:** Assert that no ancestor holds `styles.json` first (skip with a message if one does), or
give `resolve_from` an injectable stop point.

### IN-07: `remove` is not the inverse of `add` for an unterminated last line, and the test has a no-op `format!` (carried over: iteration-2 IN-09)

**File:** `src/filter/section.rs:193-197, 446`
**Issue:** `add` gives an unterminated last line an EOL, and `remove` keeps that EOL, so the round
trip changes one byte. `refresh_idempotent_all_shapes` still builds `nofinal` with
`format!("{}{}{}", &r1[..r1.len() - eol.len()], "", "")`.
**Fix:** Document and assert the one-byte difference, or restore `Eol::None` on remove. Replace the
`format!` with `r1[..r1.len() - eol.len()].to_string()`.

### IN-08: Redundant UTF-8 validation in `LoadedFilter::load`; links are broken by replace-by-rename; test workspaces leak (carried over: iteration-2 IN-10 + IN-11)

**File:** `src/dynamic.rs:130-132`; `src/safe_write.rs:219-266`; `tests/dynamic_filter.rs:26-46`
**Issue:**

- **Double validation.** `crate::filter::parse(&bytes)` runs only to validate UTF-8, its result is
  discarded, and `String::from_utf8(bytes.clone())` repeats the same check.
- **Linked filters.** The atomic rename replaces the directory entry, so a symlinked or hard-linked
  `dynamic.filter` loses its link after the first Apply. This behaviour is undocumented.
- **Test workspaces.** `workspace()` creates `%TEMP%\ferrisfilter_dyn_<tag>_<nanos>` and never
  removes it.

**Fix:** Drop the `parse` call and use `String::from_utf8(bytes)` directly. Document the link
behaviour, or canonicalize a symlink target before choosing the temp directory. Use
`tempfile::TempDir` for the test workspaces.

---

_Reviewed: 2026-10-12T23:00:00Z_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
