---
phase: 18-filter-as-store-economy-section
reviewed: 2026-10-12T00:00:00Z
depth: standard
files_reviewed: 25
files_reviewed_list:
  - src/paths.rs
  - src/safe_write.rs
  - src/filter/section.rs
  - src/filter/mod.rs
  - src/dynamic.rs
  - src/compile.rs
  - src/emitter.rs
  - src/cli.rs
  - src/run.rs
  - src/lib.rs
  - src/loader.rs
  - src/models/config.rs
  - src/overlay/app.rs
  - src/overlay/apply.rs
  - src/overlay/mod.rs
  - src/overlay/edit.rs
  - src/overlay/rule_editor.rs
  - src/overlay/view_model.rs
  - tests/dynamic_filter.rs
  - tests/cli_errors.rs
  - tests/filter_roundtrip.rs
  - .gitignore
  - .gitattributes
  - CHANGELOG.md
  - CLAUDE.md
findings:
  critical: 1
  warning: 8
  info: 8
  total: 17
status: issues_found
---

# Phase 18: Code Review Report

**Reviewed:** 2026-10-12
**Depth:** standard (diff `c9889a5..HEAD`)
**Files Reviewed:** 25
**Status:** issues_found

## Summary

Phase 18 makes the game-folder `.filter` the store. It adds a marker-fenced economy section
(`filter::section`), a guarded, backed-up, atomic writer (`safe_write`), an app-root resolver
(`paths`), a new CLI `apply`, and an overlay footer built on them.

The core text operations hold up. `refresh` touches only the lines between the markers, keeps
the BOM, follows the begin marker's EOL, and is idempotent. Damaged markers are a hard stop, and
v0.4 migration happens in memory only. The full suite passes: 511 unit tests plus all integration
binaries (run with `CARGO_TARGET_DIR` set to the temp target).

No test can reach the real game folder. Every spawn of `apply` sets `USERPROFILE` and
`FERRISFILTER_HOME` to a temp dir, `install::default_dest` reads `USERPROFILE` directly, and the
overlay unit tests never build `OverlayApp`.

The defects are concentrated in four places:

- **The one-time `.ffbak` guarantee.** A backup that fails partway leaves a truncated `.ffbak`,
  and that file then silently blocks every future backup.
- **Durability of the "atomic" replace.** The temp file is never synced before the rename.
- **Overlay dead end.** A damaged or unloadable filter offers no RELOAD.
- **Doc accuracy.** The new `apply` row in CLAUDE.md lost its path separators, and several entries
  still describe the JSON store.

## Critical Issues

### CR-01: A failed backup write leaves a partial `.ffbak` that permanently blocks the real backup

**File:** `src/safe_write.rs:151-171`

**Issue:** The backup is opened with `create_new(true)`, then filled with `write_all` +
`sync_all`. If either fails after the open (disk full, OneDrive I/O error, AV interference), the
code returns `WriteError::Backup` but leaves the truncated or empty `<name>.ffbak` on disk.

On the user's next Apply, `create_new` hits `AlreadyExists && bp.is_file()` and takes the
silent `{}` arm. The original filter is then overwritten with no valid backup, and the outcome
reports `backup: None`, so the user is not told. The `.ffbak` is "created once, never
overwritten" by design, so the corrupt copy is the only backup there will ever be.

This matters most for the one-time v0.4 migration. That `.ffbak` is the only copy of the
pre-conversion file.

**Fix:** Build the backup through a temp file and publish it without clobbering, so a partial
backup can never take the final name:
```rust
if let Some(original) = loaded {
    let bp = backup_path(path);
    if !bp.exists() {
        let mut t = tempfile::NamedTempFile::new_in(dest_dir)
            .map_err(|e| WriteError::Backup { path: bp.display().to_string(), source: e })?;
        t.write_all(original)
            .and_then(|_| t.as_file().sync_all())
            .map_err(|e| WriteError::Backup { path: bp.display().to_string(), source: e })?;
        match t.persist_noclobber(&bp) {
            Ok(_) => backup = Some(bp),
            Err(e) if e.error.kind() == io::ErrorKind::AlreadyExists && bp.is_file() => {}
            Err(e) => return Err(WriteError::Backup { path: bp.display().to_string(), source: e.error }),
        }
    } else if !bp.is_file() {
        return Err(WriteError::Backup { /* "exists and is not a file" */ .. });
    }
}
```
Add a test that injects a write failure after the open and asserts that no `.ffbak` remains.

## Warnings

### WR-01: The temp file is never fsynced before the rename, so the "atomic" replace is not crash-safe

**File:** `src/safe_write.rs:180-191`

**Issue:** `temp.write_all(..).and_then(|_| temp.flush())` only flushes the userspace buffer;
`flush` on a `File` is a no-op. `persist` (`MoveFileExW(MOVEFILE_REPLACE_EXISTING)`, without
`MOVEFILE_WRITE_THROUGH`) can make the rename durable before the data is. A power loss or crash
in that window can leave `dynamic.filter` empty or partial.

The guarantee this module exists to provide is "either old bytes or new bytes". The backup path
already calls `sync_all()`, so the target write is the inconsistent one.

**Fix:**
```rust
temp.write_all(new_bytes)
    .and_then(|_| temp.as_file().sync_all())
    .map_err(|e| WriteError::Unwritable { path: shown(), source: e })?;
```

### WR-02: The overlay has no recovery path for a damaged or unloadable filter

**File:** `src/overlay/apply.rs:116-125, 136-138`; `src/overlay/app.rs:377, 417`

**Issue:** `footer_actions` enables RELOAD only when `needs_reload` is set, and only a
`ChangedOnDisk` refusal sets it.

- **Damaged markers.** The status line tells the user to fix the markers by hand.
  `filter_state_message(Damaged)` even says "Fix them by hand, then RELOAD". But no RELOAD button
  is shown, so the stripe keeps the stale parse forever.
- **`LoadFailed`.** Covers a file locked at launch, not UTF-8, or `USERPROFILE` missing. The same
  dead end applies.
- **Unresolved app root.** Root resolution runs once in `new()` and is never retried.

In every case the only recovery is tray → Quit and relaunch. The test
`footer_damaged_and_load_failed_offer_neither` locks this behaviour in.

**Fix:** Make RELOAD available whenever the state is not clean:
```rust
reload: needs_reload || matches!(state, FilterState::Damaged | FilterState::LoadFailed),
```
Also re-resolve `paths::app_root()` inside `reload()`.

### WR-03: An economy marker that sits directly against a block becomes that block's `#ff:` metadata

**File:** `src/filter/section.rs:101-108` (with `src/filter/parse.rs:70-76`)

**Issue:** `find` rejects markers only inside a block's `owned()` range. `lead_start` attaches
every comment line directly above a header to that block, `#ff:` lines included. So
`#ff:economy-end\nShow # Z` is accepted (the `remove_keeps_text_after_section` test uses exactly
this shape), and the end marker becomes metadata of block Z: `block_meta(Z)` returns
`("economy-end", "")`.

Any span-based edit of Z will carry the marker with it: delete, move, or the Phase 19 rule list
reordering rules after the section. That silently damages the section, or moves the fence into
the general rules.

**Fix:** In `find`, also reject a marker that falls in any block's `leading()` range. Use a
dedicated error such as "economy marker is attached to the block on line N; put a blank line
between them". Alternatively, make `lead_start` stop at economy marker keys so they are always
trivia.

### WR-04: The root marker `config.json` is a generic file name, so exe walk-up can lock onto an unrelated folder

**File:** `src/paths.rs:413-416, 434-438`

**Issue:** The walk-up runs before the cwd fallback and stops at the first ancestor holding
either `styles.json` or `config.json`. `config.json` is a very common name. If the exe sits
anywhere under a folder that holds some unrelated `config.json`, the resolver returns that
folder. It never falls back to a cwd that really is the FerrisFilter folder, and Apply then fails
with "cannot read styles.json".

`apply` and the overlay never read `config.json`. They need `styles.json` and `tiers.json`.

**Fix:** Require `styles.json` as the marker. If `config.json` must stay as a hint, require it
together with `styles.json`:
```rust
fn is_root(dir: &Path) -> bool {
    crate::compile::InputPaths::relative_to(dir).styles.is_file()
}
```

### WR-05: `apply --tiers` / `--styles` resolve against the cwd while every default input resolves against the app root

**File:** `src/run.rs:163-172`

**Issue:** `InputPaths::relative_to(&root).with_tiers(t)` stores `t` as given, so a relative
path resolves against the process working directory. Phase 18 lets users run `ff-cli apply` from
anywhere, with inputs found via the app root. The documented recipe
`ff-cli apply --tiers tiers-fallback.json` (CLAUDE.md, CLI help) now fails with "cannot read
tiers-fallback.json" outside the repo, even though prices and styles resolve fine.

The `apply_styles_flag_reads_alternate_path` test hides this by setting `cwd == FERRISFILTER_HOME`.

**Fix:** Join relative override paths onto the root:
`inputs.with_tiers(if Path::new(t).is_relative() { root.join(t) } else { t.into() })`.
Alternatively, document the asymmetry explicitly in the help and CLAUDE.md, and add a test whose
cwd differs from the root.

### WR-06: `dynamic.json` was removed from `.gitignore` while the user's per-user copy still sits in the repo

**File:** `.gitignore:21-23`

**Issue:** The `/dynamic.json` ignore rule was deleted. The author's working tree still holds a
`dynamic.json` (git status shows `?? dynamic.json`), so a routine `git add -A` / `git add .`
would commit personal state that the project says is never committed. The file is now orphaned
(nothing reads it), but it is still personal data.

**Fix:** Keep `/dynamic.json` in `.gitignore`, with a comment that it is v0.4 leftover state.
Alternatively, tell the user in CHANGELOG/CLAUDE.md to delete it.

### WR-07: Doc errors in CLAUDE.md and CHANGELOG.md (wrong default path, stale "store" wording)

**File:** `CLAUDE.md:26, 35`; `CHANGELOG.md` (`[Unreleased]` → Changed, the Phase 17 `styles` entry)

**Issue:**

1. In the new `ferrisfilter apply` row, the default path is rendered as
   `%USERPROFILE%OneDriveDocumentosMy GamesPath of Exile 2dynamic.filter`. Every backslash was
   stripped, so the default destination is documented incorrectly.
2. The `styles.json` row still says the file is read "beside the store", and that "the overlay's
   next Apply saves the store without it". No JSON store exists any more, and Apply never writes a
   config.
3. The adjacent CHANGELOG entry still names `dynamic.json` as a store that "the overlay's next
   Apply" rewrites.

CLAUDE.md requires that this section be kept accurate.

**Fix:** Restore the path as
`%USERPROFILE%\OneDrive\Documentos\My Games\Path of Exile 2\dynamic.filter`. Reword the
`styles.json` row to "read by `generate` (beside the inputs) and by `apply`/the overlay (from the
app root)". Drop the "saves the store" clause, and note in the CHANGELOG entry that the store
wording is superseded by the Phase 18 breaking change.

### WR-08: A read-only or permission-denied target is retried and then reported as "in use"; the error codes are Windows-only

**File:** `src/safe_write.rs:102-104, 68-75`

**Issue:** `is_retriable` treats raw OS error 5 (`ERROR_ACCESS_DENIED`) as a sharing violation.
`MoveFileExW` also returns 5 for a read-only target or a missing write ACL. In those cases the
writer sleeps about 750 ms and then reports `Busy`: "is in use ... close anything holding it,
such as the game or OneDrive". That message is wrong and sends the user in the wrong direction.

On non-Windows targets, 5, 32 and 33 are `EIO`, `EPIPE` and `EDOM`, so an `EIO` would be retried
and also mislabelled as "in use".

**Fix:** Gate the codes behind `#[cfg(windows)]` (return `false` elsewhere). After the final
attempt with code 5, check `fs::metadata(path)?.permissions().readonly()`. If the target is
read-only, report `Unwritable` with a "file is read-only" hint instead of `Busy`.

## Info

### IN-01: `*.ffbak` and stray `.tmp*` files are not git-ignored

**File:** `.gitignore`

**Issue:** `apply --filter ./x.filter` inside the repo leaves `x.filter.ffbak`, and an
interrupted write can leave `.tmpXXXX` files. Both would show up as untracked files.

**Fix:** Add `*.ffbak` (and optionally `/.tmp*`).

### IN-02: The CLI prints "converting ..." before the write that can still fail, and prints the reload hint on a no-op

**File:** `src/run.rs:198-203, 226`

**Issue:** The v0.4 conversion notice is printed before `apply_file`. If the write is then
refused (`ChangedOnDisk`, `Busy`), the output claims a conversion that never happened. Separately,
"Reload it in-game" is printed even when the outcome is `Unchanged`; the overlay's
`unchanged_message` deliberately omits it.

**Fix:** Print the conversion line from `report.migrated` after success, and print the reload
hint only for `Written`.

### IN-03: The `FilterNotFound` help text is wrong for `--remove-economy`

**File:** `src/dynamic.rs:57-60, 223-225`

**Issue:** `apply --remove-economy` on a missing file says "Use --add-economy to create it with an
economy section", which is the opposite of what the user asked for.

**Fix:** Use a separate variant (or a variant field) for removal, or a neutral help text.

### IN-04: Stale comments and a caller-less API left over from the JSON store

**File:** `src/compile.rs:318-325`; `src/emitter.rs:53`; `CLAUDE.md` (Jewels paragraph)

**Issue:**

- `compile_as` still says "The dynamic path passes `source_name: "dynamic.json"`", but
  `compile_as` and the dynamic `FilterIdentity` have no non-test caller any more.
- CLAUDE.md says Jewels "is offered in the overlay's class picker", but the picker is unreachable
  until Phase 19.

**Fix:** Update or remove these comments. Consider removing `compile_as` until Phase 19 needs it.

### IN-05: The resolver tests depend on the machine's temp-dir ancestors having no marker file

**File:** `src/paths.rs:546-565`

**Issue:** `falls_back_to_cwd_with_marker` and `cwd_without_marker_is_not_found_and_lists_attempts`
pass a `tempdir()` as `exe_dir`. They assume no ancestor of `%TEMP%` holds `config.json` or
`styles.json`. Given WR-04, a stray `C:\Users\<u>\config.json` makes them fail.

**Fix:** Pass `exe_dir: None`, or build `exe_dir` inside a tempdir and check for markers in
ancestors before asserting.

### IN-06: `remove` is not the inverse of `add` for an unterminated last line, and the tests skip that case

**File:** `src/filter/section.rs:193-197, 528-537, 446`

**Issue:** `add` appends an EOL to an unterminated last line, and `remove` keeps it, so the
round trip changes one byte. `remove_inverts_add` quietly skips those inputs. `refresh_idempotent_all_shapes`
also builds `nofinal` with a meaningless `format!("{}{}{}", .., "", "")`.

**Fix:** Either restore `Eol::None` on the line before the removed separator when it was
originally unterminated, or document the one-byte difference and assert it explicitly. Simplify
the `nofinal` expression.

### IN-07: `LoadedFilter::load` parses and copies the file twice just to validate UTF-8

**File:** `src/dynamic.rs:130-132`

**Issue:** `crate::filter::parse(&bytes)` is called only for its UTF-8 check and its result is
discarded. `String::from_utf8(bytes.clone())` then checks UTF-8 again on a copy.

**Fix:** Keep the `from_utf8(bytes.clone())` check, and drop the `parse` call or reuse its
`Document`.

### IN-08: Replace-by-rename breaks a symlinked or hard-linked `dynamic.filter`

**File:** `src/safe_write.rs:173-191`

**Issue:** The replace swaps the directory entry. A user who symlinked or hard-linked the game
file to a repo copy loses the link after the first Apply, and file attributes are not carried
over.

**Fix:** Document this, or resolve a symlink target with `fs::canonicalize` before choosing the
temp directory.

---

_Reviewed: 2026-10-12_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
