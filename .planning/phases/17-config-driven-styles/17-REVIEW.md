---
phase: 17-config-driven-styles
reviewed: 2026-10-12T00:00:00Z
depth: standard
iteration: 3
files_reviewed: 23
files_reviewed_list:
  - src/models/config.rs
  - src/loader.rs
  - src/emitter.rs
  - src/compile.rs
  - src/cli.rs
  - src/run.rs
  - src/dynamic.rs
  - src/overlay/edit.rs
  - src/overlay/rule_editor.rs
  - src/overlay/store.rs
  - src/overlay/view_model.rs
  - styles.json
  - config.json
  - dynamic_base.json
  - Cargo.toml
  - CHANGELOG.md
  - CLAUDE.md
  - tests/cli_errors.rs
  - tests/cli_generate.rs
  - tests/dynamic_filter.rs
  - tests/filter_roundtrip.rs
  - tests/rule_editor.rs
  - tests/sync_prices.rs
findings:
  critical: 0
  warning: 0
  info: 7
  total: 7
status: clean
---

# Phase 17: Code Review Report (iteration 3)

**Reviewed:** 2026-10-12
**Depth:** standard
**Files Reviewed:** 23
**Status:** clean (no critical or warning findings; info items remain)

## Summary

This pass re-reviewed the Phase 17 diff (`e116acc..HEAD`), focusing on the iteration-2 fix
commits d4501f7, 1e58e76 and 2cb655c (`git diff 504f7e9 HEAD` touches only `src/loader.rs` and
`CHANGELOG.md`).

**Verification of the iteration-2 fixes:**

- **WR-01 (fixed):** `check_legacy_styles` (`src/loader.rs:157-170`) now handles three cases:
  - A JSON `null` is accepted only when `styles.json` does not define that name.
  - An entry that fails to deserialize is always counted as differing.
  - A parsed entry must equal the `styles.json` entry.

  The old `None == None` path is gone. Four new unit tests cover these cases: `{"Bogus": 7}`, a
  misspelled `craftable.fnt`, `{"craftable": null}` (accepted) and `{"S": null}` (rejected). The
  pre-Phase-17 `"craftable": null` save still loads, so WR-03 from iteration 1 still holds.
- **WR-02 (fixed):** `load_styles_rejects_duplicate_name` normalizes the file to LF and then runs
  the splice against both the LF and CRLF variants. The splice keys on the first `{`, and
  `assert_ne!` guards against a splice that changes nothing. The test no longer depends on the
  checkout's line endings.
- **IN-02 (fixed):** The CHANGELOG `[Unreleased]` entry now lists the font, minimap size, sound
  id/volume range checks and the exactly-one-sound-source rule. This matches `check_ranges`.

`cargo test` passes in full (462 lib tests plus every integration suite).
`cargo clippy --all-targets` reports no warnings in any file this phase changed.

No new critical or warning issues were found. One new info item (IN-08) is about how the error
message words the new reject path. Five info items carry over from earlier iterations (IN-01,
IN-03 to IN-06). IN-07 (the stale store doc) also carries over: the module doc still says
`load_json` and "autosave".

## Info

### IN-01: Empty `sound.custom` passes validation (carried)

**File:** `src/models/config.rs:265-271`
**Issue:** `"custom": ""` passes both the character check and the exactly-one-source check. The
filter then gets `CustomAlertSound "" <vol>`, which is only reported when the game loads the
filter.
**Fix:** Reject an empty or whitespace-only `custom` value with `InvalidCustomSound`.

### IN-03: `StyleSet::from_pairs` is dead public API that silently keeps the last duplicate (carried)

**File:** `src/models/config.rs:237`
**Issue:** The function has no callers. A duplicate name is last-wins, which contradicts OD-3, and
the function does not run `validate()`.
**Fix:** Delete it, or gate it with `#[cfg(test)]`.

### IN-04: The unknown-style check lives in two places with two error types (carried)

**File:** `src/models/config.rs` (`check_rules`), `src/emitter.rs:115` (`resolve_style`)
**Issue:** The two paths format their lists and handle disabled rules independently, so they can
drift apart.
**Fix:** Share one formatter and one lookup, or treat an emitter lookup miss as an internal error.

### IN-05: CLI `apply` and the overlay resolve `styles.json` from different directories (carried)

**File:** `src/run.rs:146-153` (`InputPaths::conventional()`) vs `src/overlay/store.rs:74-75`
(`relative_to(store_dir)`)
**Issue:** `apply --store other/dynamic.json` uses `./styles.json`, while the overlay uses
`other/styles.json`. Which file is used also decides what the legacy inline section is compared
against, so the same store can load in one entry point and be rejected in the other.
**Fix:** Have `cmd_apply` resolve styles relative to the store's directory, or document the split.

### IN-06: Non-required style names may collide by case, and an empty name is accepted (carried)

**File:** `src/models/config.rs:254-263`
**Issue:** `kite` and `Kite` can coexist and show up as near-identical picker entries. `""` renders
as a blank picker row.
**Fix:** Reject case-insensitive duplicates across all names, and reject empty names.

### IN-07: The `overlay::store` module doc is stale (carried)

**File:** `src/overlay/store.rs:3-9`
**Issue:** The doc still says `load` uses `loader::load_json`; it now uses `load_config` with a
styles path. It also says `save` runs "once per frame from the editor's autosave", but Apply has
been the only writer since Phase 14.
**Fix:** Rewrite the module doc to describe `load_config(store, styles)` and Apply as the only
writer.

### IN-08: The inline-section error says "differ" for entries that do not parse, and drops the parse cause (new)

**File:** `src/models/config.rs:156-160, 170-176`; `src/loader.rs:165-168`
**Issue:** Since d4501f7, `differing` also includes entries that fail to deserialize (`{"Bogus": 7}`,
a misspelled `fnt`) and a `null` for a defined style. The message still reads "These inline styles
are missing from, or differ from, styles.json", and the variant's doc says the same. The `serde`
error (for example ``unknown field `fnt` ``) is discarded with `Err(_) => true`. So a user who
mistyped a field is told the style "differs" and given no reason. They will only see the actual
typo after moving the entry into `styles.json` and reloading. The style is not lost, because the
load is still rejected, so this is a diagnostics issue only.
**Fix:** Widen the wording to "missing from, differ from, or are not valid styles for
{styles_path}" and update the variant doc. Optionally, carry the first parse error as
`name: cause`, for example by collecting `format!("{name} ({e})")` in the `Err(e)` arm.

---

_Reviewed: 2026-10-12_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
