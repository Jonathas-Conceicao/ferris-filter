---
phase: 06-emitter-rewrite-migration-parity
reviewed: 2026-06-05T00:00:00Z
depth: deep
files_reviewed: 4
files_reviewed_list:
  - src/emitter.rs
  - src/run.rs
  - config.json
  - tests/cli_generate.rs
findings:
  critical: 0
  warning: 3
  info: 4
  total: 7
status: issues_found
---

# Phase 6: Code Review Report

**Reviewed:** 2026-06-05
**Depth:** deep
**Files Reviewed:** 4
**Status:** issues_found

## Summary

The Phase 6 emitter rewrite is mechanically correct. `cargo build` and `cargo test` both
pass cleanly (124 unit tests + 11 integration tests). The `emit_rules` function compiles every
`When` field to the right filter syntax for every condition type. `GenerateError` is properly
propagated through `run.rs`. The 13-rule `config.json` migration reproduces all 13 policy
blocks with correct conditions, and the `artifact_guard` / `parity_policy_blocks_match_oracle`
tests confirm byte-level and semantic parity respectively.

Three warnings and four info items follow. None are blockers. The most significant finding
is a logical gap in the `NumericCondition` emitter that could silently produce malformed
filter output if a future config author sets both `eq` and `min` on the same condition; the
second is a cosmetic inconsistency in how unique-block background color is positioned relative
to the canonical action order; and the third is a missing CLI-level integration test for the
`GenerateError::UnknownStyle` path.

---

## Warnings

### WR-01: `emit_numeric` emits two condition lines when both `eq` and `min` are set

**File:** `src/emitter.rs:233`
**Issue:** `emit_numeric` contains three independent `if let Some(...)` branches — `eq`, then
`min`, then `max` — with no mutual-exclusion guard. If a rule in `config.json` specifies both
`"eq": 0` and `"min": 1` on the same `NumericCondition`, the emitter will emit two condition
lines for the same attribute (e.g., `BaseEvasion 0` followed by `BaseEvasion >= 1`). That
would produce a block whose conditions can never all be true simultaneously (0 AND >= 1),
silently dropping items the author intended to match.

No current `config.json` rule triggers this, but the type system has no guard, and the doc
comment on `NumericCondition` describes `eq`, `min`, and `max` as independent optional
fields with no mention of mutual exclusivity. Anyone adding a new rule with copy-paste of
an existing object could accidentally set both.

**Fix:** Add an early validation in `emit_numeric` (or at load time in a `validate_rules`
function called from `generate`) that returns an error when more than one of `eq`/`min`/`max`
is set:

```rust
fn emit_numeric(out: &mut String, attr: &str, nc: &crate::models::config::NumericCondition) {
    let set_count = nc.eq.is_some() as u8 + nc.min.is_some() as u8 + nc.max.is_some() as u8;
    debug_assert!(
        !(nc.eq.is_some() && nc.min.is_some()),
        "NumericCondition for {attr}: 'eq' and 'min' are mutually exclusive"
    );
    // existing branches follow...
}
```

Alternatively, document the mutual exclusivity constraint explicitly in the doc comment for
`NumericCondition` so authors are warned at the schema level.

---

### WR-02: Unique-block `SetBackgroundColor` is emitted after all other actions, violating the canonical D-03 order

**File:** `src/emitter.rs:447`
**Issue:** The canonical action order (documented at line 131–139) places `SetBackgroundColor`
at position 4 (after `SetTextColor`, before `SetBorderColor`). For unique blocks, the style's
`background` field is set to `None` before calling `emit_style_actions`, then a hardcoded
`SetBackgroundColor 175 96 37 240` is written after the entire `emit_style_actions` call
returns — placing it at position 8+, after `SetBorderColor`, font, and sound.

For example, a tier-A unique block currently emits:
```
PlayEffect Purple
MinimapIcon 1 Purple Circle
SetTextColor 189 147 249 255
SetBorderColor 189 147 249 255
SetFontSize 37
CustomAlertSound "uwu.mp3" 300
SetBackgroundColor 175 96 37 240   ← out of canonical order
```

PoE2's filter parser accepts action lines in any order, so this has no functional effect on
the live game. It does, however, break the invariant stated by the D-03 comment block and
will look inconsistent to any human reading the generated filter.

**Fix:** Construct a modified `Sound` parameter or add a `background_override` argument to
`emit_style_actions`, or emit the background inline as position 4 by inserting it before
the border:

```rust
// In emit_style_actions or as a post-pass:
// After step 3 (text), emit background override if provided.
writeln!(out, "  SetBackgroundColor 175 96 37 240").unwrap(); // step 4 override
// Then step 5 (border) continues normally.
```

The simplest approach is to keep `unique_style.background` as `None` (so the Dracula bg is
skipped) and insert the bronze background write between the text-color and border-color writes
inside `emit_unique_sections`, rather than appending it at the end.

---

### WR-03: No CLI-level integration test for `GenerateError::UnknownStyle`

**File:** `tests/cli_generate.rs`, `tests/cli_errors.rs`
**Issue:** `GenerateError::UnknownStyle` is the only new failure mode introduced by
`emit_rules` that was not present before the rewrite. It is covered by `resolve_style`'s
logic and the unit-level compile test in `src/models/config.rs`, but there is no
integration test that:
1. Puts a `config.json` containing a rule with `"style": "InvalidTier"` in a temp dir.
2. Runs `ferrisfilter generate`.
3. Asserts exit code != 0 and that stderr contains the style name and "unknown style".

The existing `cli_errors.rs` covers config parse failures and tiers validation but not the
generate-time style resolution failure. If `resolve_style` were accidentally broken to return
`Ok` for unknown names, no integration test would catch it.

**Fix:** Add a test to `tests/cli_errors.rs`:

```rust
#[test]
fn unknown_style_in_rule_exits_nonzero() {
    let bad_config = valid_config().replace(
        "\"style\": \"Recycle\"",
        "\"style\": \"InvalidTierXYZ\"",
    );
    let dir = make_temp_dir("unknown_style", &[
        ("config.json", &bad_config),
        ("tiers.json", &valid_tiers()),
        ("currency.json", &valid_currency()),
    ]);
    Command::cargo_bin("ferrisfilter")
        .unwrap()
        .current_dir(&dir)
        .arg("generate")
        .assert()
        .failure()
        .stderr(contains("InvalidTierXYZ"));
}
```

---

## Info

### IN-01: `classGroups` is loaded into a `HashMap` but never used by `emit_rules`

**File:** `src/models/config.rs:412`, `config.json:39`
**Issue:** `Config.class_groups` is deserialized as `Option<HashMap<String, Vec<String>>>`.
The field is populated in `config.json` with five named groups (`buildWeapons`, `buildArmour`,
`allArmour`, `allGear`, `flaskCharm`). However, `emit_rules` never consults it: rules in
`config.json` inline the class lists directly. The `classGroups` value is parsed, passes
`config_round_trips`, and is then silently ignored.

This is not a bug today, but the field name `classGroups` implies it should serve as an
alias table for `when.class` values. If that intent was deferred, a comment should document
the deferred status; if it is documentary-only metadata, it should be documented as such.

**Fix:** Either:
- Add a `/// Note: not yet resolved by emit_rules; rules must inline class lists directly.`
  comment on the `class_groups` field, OR
- Implement alias resolution in `emit_rules` (look up `when.class` values against
  `class_groups` before emitting the class line).

---

### IN-02: Condition order within `emit_rules` diverges from the pre-rewrite oracle

**File:** `src/emitter.rs:277`, `tests/fixtures/parity-baseline-earlygame.filter`
**Issue:** The pre-rewrite emitter produced defence conditions in the order
`BaseEnergyShield, BaseArmour, BaseEvasion` (reflecting the original hard-coded band
functions). The new `emit_rules` processes them in fixed struct-field order:
`BaseArmour (3), BaseEvasion (4), BaseEnergyShield (5)`. Concretely:

- **band 26-45 oracle:** `BaseEnergyShield > 0` then `BaseEvasion 0`
- **band 26-45 new:** `BaseEvasion 0` then `BaseEnergyShield > 0`
- **band 46+ oracle:** `BaseEnergyShield > 0`, `BaseArmour 0`, `BaseEvasion 0`
- **band 46+ new:** `BaseArmour 0`, `BaseEvasion 0`, `BaseEnergyShield > 0`

Similarly, the recycle block places `Quality >= 1` before `Sockets >= 1` (oracle had
`Sockets >= 1` first), and `flasks/charms: quality always` places `Class` before `Quality`
(oracle had `Quality` first).

PoE2 filter conditions within a block are AND-ed; order has no effect on matching. The
`parity_policy_blocks_match_oracle` test correctly uses sorted block comparison, so these
ordering differences are invisible to the test suite. The `artifact_guard` test uses the
re-baselined `seed-universal.filter`, so the new order is now the stable oracle.

No action is needed; this is documented for transparency. The ordering in the new emitter
(alphabetical by struct field declaration) is consistent and deterministic.

---

### IN-03: Rule `name` field comment mentions "model path tag" from the old emitter

**File:** `src/emitter.rs:213` (file header comment)
**Issue:** The generated file header still says:
```
# Block tags link each rule back to the model:
#   [alwaysShow.K]          -> a level-independent highlight
#   [areaLevelBands "R".K]  -> band R, show-key K
#   [styles.X]              -> the style entry applied
```
These `[areaLevelBands "R".K]` tags appeared in the pre-rewrite emitter (visible in
`parity-baseline-earlygame.filter`, lines 34–109) but are no longer emitted. The new
emitter uses plain rule names (`Show # recycle: sockets + quality salvage`), not
model-path tags. The header comment describes a format that no longer exists in the output.

**Fix:** Update `emit_header` to describe the actual new format:
```rust
writeln!(out, "# Block comments use rule names from config.json rules[].name.").unwrap();
writeln!(out, "# Each Show/Hide block corresponds to one entry in the rules array.").unwrap();
```

---

### IN-04: Clippy `uninlined_format_args` warnings (41 instances in `src/emitter.rs`)

**File:** `src/emitter.rs` (multiple lines)
**Issue:** `cargo clippy` reports 41 `clippy::uninlined_format_args` warnings in
`src/emitter.rs` and related files. Examples:
- Line 172: `writeln!(out, "  SetFontSize {}", font)` should be `writeln!(out, "  SetFontSize {font}")`
- Line 266: `format!("rule {}", idx)` should be `format!("rule {idx}")`

These are style-level warnings that do not affect correctness. They are fixable in bulk
with `cargo clippy --fix --lib`.

**Fix:** Run `cargo clippy --fix --lib -p ferrisfilter` to auto-apply all suggestions, then
review and commit the result.

---

_Reviewed: 2026-06-05_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: deep_
