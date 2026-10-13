---
phase: 17-config-driven-styles
reviewed: 2026-10-12T00:00:00Z
depth: standard
iteration: 2
files_reviewed: 21
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
  - tests/cli_errors.rs
  - tests/cli_generate.rs
  - tests/dynamic_filter.rs
  - tests/filter_roundtrip.rs
  - tests/rule_editor.rs
  - tests/sync_prices.rs
findings:
  critical: 0
  warning: 2
  info: 7
  total: 9
status: issues_found
---

# Phase 17: Code Review Report (iteration 2)

**Reviewed:** 2026-10-12
**Depth:** standard
**Files Reviewed:** 21
**Status:** issues_found

## Summary

This pass re-reviewed the Phase 17 diff (`e116acc..HEAD`) and focused on the four fix commits
(55500e5, e8983bd, 0b57ecb, 504f7e9).

**Verification of the iteration-1 fixes:**

- **WR-01 (fixed):** `generate_as` now calls `config.styles.validate()?` before it emits anything.
  `style_for_tier` returns `GenerateError::UnknownTier` instead of panicking, and both economy
  emitters propagate the `Result`. No production caller builds a partial `StyleSet`, so the new
  check rejects nothing that is valid.
- **WR-02 (fixed):** `check_ranges` covers font (1..=45), minimap size (0..=2), sound id (1..=16),
  sound volume (0..=300), and requires exactly one of `id`/`custom`. The committed `styles.json`
  passes all of these checks.
- **WR-03 (fixed, with a new gap; see WR-01 below):** I tested this end to end with the CLI.
  - A store seeded from the pre-Phase-17 `dynamic_base.json` (`git show e116acc:dynamic_base.json`)
    now loads, and `apply` writes the filter.
  - The same store with `"craftable": null` also loads. Every pre-17 overlay save wrote that key,
    because the old `Styles.craftable` had no `skip_serializing_if`.
  - A non-object section (`[]`) is rejected.
  - However, the content check treats any inline entry that fails to deserialize as equal to an
    absent style. That lets a user-authored entry be dropped silently.
- **WR-04 (fixed):** `deserialize_present_value` maps `"styles": null` to `Some(Value::Null)`, and
  `check_legacy_styles` rejects it because it is not an object.

The full `cargo test` suite passes on this checkout. The CRLF concern the fixer raised is real:
`load_styles_rejects_duplicate_name` passes here only because the working-tree `styles.json`
happens to be LF (see WR-02).

## Warnings

### WR-01: An unparseable inline style is treated as matching and silently dropped, then deleted from disk by the next Apply

**File:** `src/loader.rs:156-160`
**Issue:** `check_legacy_styles` compares
`serde_json::from_value::<Style>(value).ok()` with `styles.get(name)`. When an entry fails to
deserialize (wrong type, a misspelled field under `deny_unknown_fields`, an out-of-range `u8`
colour) and its name is not in `styles.json`, both sides are `None`. The entry is then counted as
"not differing", the whole section is dropped, and the load succeeds.

Reproduced with the CLI: a store containing `"styles": {"Bogus": 7}` applies successfully and
reports no error. The realistic case is a user following the error message, who hand-edits a
custom style such as `craftable` in the legacy section and makes a typo (`"fnt": 30`). On the next
load the style disappears without any diagnostic. Because `store::save` never writes `styles`, the
overlay's next Apply then removes it from `dynamic.json` permanently. This is the silent-ignore
outcome that OD-1 forbids, now combined with data loss.

The only `None == None` case that has to be accepted is the literal `null` that old saves wrote
for `craftable`.
**Fix:** Treat a JSON `null` as "absent". Count any other entry that fails to parse as differing:
```rust
.filter(|(name, value)| {
    if value.is_null() {
        // Old saves wrote `"craftable": null` for an unset optional style: nothing to lose,
        // unless styles.json defines the name (then the inline copy disagrees).
        return styles.get(name).is_some();
    }
    match serde_json::from_value::<Style>((*value).clone()) {
        Ok(inline) => Some(&inline) != styles.get(name),
        Err(_) => true, // unparseable user content is never silently discarded
    }
})
```
Add tests for `{"Bogus": 7}` and for a misspelled field (both should be rejected and named), and
for `{"craftable": null}` (should be accepted).

### WR-02: `load_styles_rejects_duplicate_name` fails on any fresh checkout of this repo

**File:** `src/loader.rs:253-260`
**Issue:** The test splices a duplicate entry by replacing the first `"{\n"` in
`include_str!("styles.json")`. The Rust literal is always LF, because rustc normalizes CRLF in
source files. `include_str!`, however, returns the file's bytes exactly as they are on disk. This
repo has `* text=auto`, and the machine has `core.autocrlf=true`.

`git ls-files --eol` shows that every committed text file is checked out `w/crlf` (`config.json`,
`dynamic_base.json`, `src/loader.rs`, ...). The exception is `styles.json` (`w/lf`), and only
because it was created in this working tree and has never been checked out. On a fresh clone or
worktree, `styles.json` becomes CRLF. `replacen` then finds no match, `dup == STYLES`, the load
succeeds, and `unwrap_err()` panics.

This is a test-reliability defect: the duplicate-name rejection (OD-3) loses its only regression
test on exactly the checkouts CI or a new contributor would use. The failure does not predate
Phase 17 in any meaningful sense, because the test was added in this phase (4fd2695/a185514).
**Fix:** Make the splice independent of line endings. Build the input from a fixed literal so the
test does not depend on the file's bytes:
```rust
let dup = STYLES.replacen('{', "{ \"S\": { \"text\": null, \"border\": null, \"background\": null, \"font\": null, \"minimap\": null, \"beam\": null, \"sound\": null, \"action\": \"Show\", \"disableDropSound\": false },", 1);
assert_ne!(dup, STYLES, "splice must apply");
```
Splicing after the first `{` character is what `tests/cli_errors.rs` and `overlay::store` tests
already do. The `assert_ne!` turns any future no-op splice into a clear failure instead of a
misleading `unwrap_err` panic.

## Info

### IN-01: Empty `sound.custom` passes validation

**File:** `src/models/config.rs:265-271`
**Issue:** `"custom": ""` passes both the character check and the new exactly-one-source check, so
the filter gets `CustomAlertSound "" <vol>`. That line points at no file and is only reported when
the game loads the filter, which is the late failure WR-02 (iteration 1) was meant to prevent.
**Fix:** Reject an empty or whitespace-only `custom` value with `InvalidCustomSound`.

### IN-02: The CHANGELOG list of load-time validations predates the range checks

**File:** `CHANGELOG.md:17-20`
**Issue:** The "Validated at load" list does not mention the new font, minimap size, sound
id/volume range checks, or the exactly-one-sound-source rule from e8983bd. Without that, a user
whose custom `styles.json` starts failing has no changelog entry explaining why.
**Fix:** Add those checks to the `[Unreleased]` entry.

### IN-03: `StyleSet::from_pairs` is dead public API that silently keeps the last duplicate (carried from iteration 1)

**File:** `src/models/config.rs:237`
**Issue:** It has no callers. A duplicate name is last-wins, which contradicts OD-3, and it does not
run `validate()`.
**Fix:** Delete it, or gate it with `#[cfg(test)]`.

### IN-04: The unknown-style check lives in two places with two error types (carried)

**File:** `src/models/config.rs` (`check_rules`), `src/emitter.rs:115` (`resolve_style`)
**Issue:** The two paths format their lists and handle disabled rules independently, so they can
drift apart.
**Fix:** Share one formatter and one lookup, or treat an emitter lookup miss as an internal error.

### IN-05: CLI `apply` and the overlay resolve `styles.json` from different directories (carried)

**File:** `src/run.rs:146` (`InputPaths::conventional()`) vs `src/overlay/store.rs:74-75`
(`relative_to(store_dir)`)
**Issue:** `apply --store other/dynamic.json` uses `./styles.json`, while the overlay uses
`other/styles.json`. Since WR-03 this also decides which file the legacy inline section is compared
against, so the same store can load in one entry point and be rejected in the other.
**Fix:** Have `cmd_apply` resolve styles relative to the store's directory, or document the split.

### IN-06: Non-required style names may collide by case, and an empty name is accepted (carried)

**File:** `src/models/config.rs:254-263`
**Issue:** `kite` and `Kite` can coexist and show up as near-identical entries in the picker. `""`
renders as a blank picker row.
**Fix:** Reject case-insensitive duplicates across all names, and reject empty names.

### IN-07: The `overlay::store` module doc is stale (carried)

**File:** `src/overlay/store.rs:3-9`
**Issue:** The doc still says `load` uses `loader::load_json`; it now uses `load_config` with a
styles path. It also says `save` runs "once per frame from the editor's autosave", but Apply has
been the only writer since Phase 14.
**Fix:** Rewrite the module doc to describe `load_config(store, styles)` and Apply as the only
writer.

---

_Reviewed: 2026-10-12_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
