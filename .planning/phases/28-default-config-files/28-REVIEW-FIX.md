---
phase: 28-default-config-files
fixed_at: 2026-10-15T09:04:14Z
review_path: .planning/phases/28-default-config-files/28-REVIEW.md
iteration: 1
findings_in_scope: 5
fixed: 5
skipped: 0
status: all_fixed
---

# Phase 28: Code Review Fix Report

**Fixed at:** 2026-10-15T09:04:14Z
**Source review:** .planning/phases/28-default-config-files/28-REVIEW.md
**Iteration:** 1

**Summary:**
- Findings in scope: 5 (WR-01..WR-04, plus IN-01 as a trivial doc fix)
- Fixed: 5
- Skipped: 0
- Out of scope (Info, not attempted): IN-02, IN-03, IN-04, IN-05

Verification: `cargo fmt --check` clean; full `cargo test` green (1142 tests: 1046 lib + 96
integration); `tests/fixtures/*` unchanged.

## Fixed Issues

### WR-04: `read_source` uses `Path::exists()`, so an unreadable live file silently falls back to the defaults

**Files modified:** `src/compile.rs`
**Commit:** 877ccf8
**Applied fix:** `read_source` now uses `try_exists()` on both the live and the default path. A
failed check becomes `LoadError::Read` naming that file, and the defaults are not used. New test
`read_source_failed_existence_check_is_an_error_not_a_fallback` triggers a failed check with a NUL
byte in the file name, which fails on every platform.

### WR-01: Content and guard baselines come from separate reads

**Files modified:** `src/overlay/config_state.rs`
**Commit:** c405bdb
**Applied fix:** New helper `read_effective(live, resolve)` reads the live file once:
- **Live file present:** that one read is both the content baseline and the guard baseline.
- **Live file absent:** the source is resolved (normally the default) and read, and the guard
  baseline is `None`.
- **Live file created in between:** the resolver now returns the live path, and the single read
  again feeds both baselines.

`ConfigState::load` and both branches of `follow_disk` use the helper. New test
`read_effective_takes_both_baselines_from_one_read` covers all three cases. It simulates the
creation race inside the resolver closure.
**Status:** fixed: requires human verification (it changes guard-baseline logic).

### WR-02: `ids_unpersisted` from a default source makes any Apply create a live `styles.json`

**Files modified:** `src/overlay/config_state.rs`
**Commit:** d1649da
**Applied fix:** Ids now count as unpersisted only when the live file was the source:
- `assign_missing_ids() && styles_live.is_some()` in `load`;
- the same condition in `follow_disk`;
- `follow_disk` also clears the flag whenever the source falls back to the default.

New tests:
- `idless_default_source_is_not_persisted_without_an_edit`: an id-less default does not trigger a
  write; an id-less live file still does; deleting that file clears the flag; a real edit still
  writes.
- `committed_styles_default_carries_every_id`: guards the invariant the old test relied on.

**Status:** fixed: requires human verification (state-flag logic).

### WR-03: Legacy inline-`styles` error tells the user to edit `styles-default.json`

**Files modified:** `src/loader.rs`, `src/models/config.rs`, `src/compile.rs`, `src/overlay/app.rs`
**Commit:** 72c3c01
**Applied fix:**
- New `loader::load_config_with_live(path, styles_path, live_styles_path)`. `load_config`
  delegates to it with the same path twice, so existing callers are unchanged.
- `StyleError::InlineStylesSection` gained a `live_styles_path` field.
- When the source is the default, the message now reads "...named styles now live in styles.json...
  Move any styles you changed or added into styles.json (create it from a copy of
  styles-default.json), then delete the `styles` section...".
- With a live file, the message stays byte-identical.
- `load_inputs` and the overlay's `load_classes_at` pass the live path.

New test: `inline_styles_advice_names_the_live_file_when_reading_the_default`. It runs against a
defaults-only root, asserts the advice never names the default, and checks that no live file is
created.

### IN-01: Stale doc comment on `with_tiers` still calls `tiers.json` committed

**Files modified:** `src/compile.rs`
**Commit:** e669262
**Applied fix:** The comment now reads "the live `tiers.json` (the test oracle reads
`tiers-default.json`)".

---

_Fixed: 2026-10-15T09:04:14Z_
_Fixer: Claude (gsd-code-fixer)_
_Iteration: 1_
