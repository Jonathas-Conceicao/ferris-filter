---
phase: 04-sync
fixed_at: 2026-05-25T00:00:00Z
review_path: .planning/phases/04-sync/04-REVIEW.md
iteration: 1
findings_in_scope: 5
fixed: 5
skipped: 0
status: all_fixed
---

# Phase 04: Code Review Fix Report

**Fixed at:** 2026-05-25T00:00:00Z
**Source review:** .planning/phases/04-sync/04-REVIEW.md
**Iteration:** 1

**Summary:**
- Findings in scope: 5 (WR-01 through WR-05; fix_scope = critical_warning)
- Fixed: 5
- Skipped: 0
- Verification: `cargo build` clean (only the harmless shared-`main.rs` informational
  note remains; the `async_fn_in_trait` warning is gone) and `cargo test` green at
  113 tests (92 + 4 + 8 + 8 + 1) after every fix.

## Fixed Issues

### WR-01: Divide-by-zero in chaos-primary conversion yields `inf`/`NaN` prices

**Files modified:** `src/sync/prices.rs`
**Commit:** c1d683d
**Applied fix:** In the `"chaos"` primary branch of `compute_prices`, replaced
`divine_rate.map(|r| primary_value / r)` and the exalt equivalent with
`.and_then(|r| (r != 0.0).then_some(primary_value / r))`. A zero rate now yields
`None` instead of `inf`/`NaN` (which serde_json silently serializes as `null`,
dropping the price). The `"divine"` branch only multiplies and was already safe.

### WR-02: Anchor validation accepts a zero/null Divine `div`

**Files modified:** `src/sync/prices.rs`
**Commit:** 5443f16
**Applied fix:** In `run_sync_prices`'s `validate()` closure, added a hard failure when
the Divine Orb `div` anchor is zero/missing (`if div_val == 0.0 { return Err(...) }`),
matching the existing Chaos guard. Divine is the cross-rate anchor, so a zero/missing
value indicates a malformed rate table; aborting now genuinely preserves a
previously-good `prices.json` (the comment about "aborting to preserve" is now
accurate). `exalt_val` remains non-fatal if null. Existing happy-path tests still pass
because Divine resolves to `div = 1.0` under `primary == "divine"`.

### WR-03: Identical `if`/`else` branches — dead `base_type` logic

**Files modified:** `src/sync/prices.rs`
**Commit:** de0452e
**Applied fix:** Collapsed the no-op `if base_type_is_name { name.clone() } else {
name.clone() }` into a single `let base_type = name.clone();` (with `let _ =
base_type_is_name;` to keep the binding referenced) and added a clear comment that
uniques (`base_type_is_name == false`, where the poe.ninja display name is NOT the
filter `BaseType`) are not yet supported and must resolve the true base type from the
metadata mapping when added. This is the minimal safe correction the orchestrator
authorized — the seed contains no uniques (`seeded_prices_no_uniques`), so the latent
unique-category bug is documented rather than force-fixed with a larger behavioral
change.

### WR-04: `sync metadata` refresh loop is a no-op — message overstates behavior

**Files modified:** `src/sync/metadata.rs`
**Commit:** 9157c64
**Applied fix:** Applied review option (b). Removed the dead per-row `map` closure whose
both branches were empty comments and which returned `ot` unchanged. Replaced the
`HashMap<raw,normalized>` accumulation with a `HashSet<normalized>` of RePoE classes
and a validation pass that counts confirmed committed class names and emits a
`Warning:` for any committed class not present in RePoE (real class-name-drift
signal). The committed `overview_types` are now explicitly passed through unchanged
(RePoE cannot re-key poe.ninja types). The output message changed from the misleading
"mapping.json refreshed (...)" to "validated mapping.json against RePoE (... N/M class
names confirmed ...) — rewrote mapping.json (content unchanged aside from
re-serialization)", which accurately describes what the command does. Removed the now-
unused `OverviewType` import. Rationale for not implementing full re-derivation: RePoE
exposes item classes, not poe.ninja `type` keys, so it cannot author the curated
`type→category` rows; honest validation + drift warning is the correct scope here, and
matches the orchestrator's instruction to make the message accurate rather than force a
large behavioral change.

### WR-05: `async_fn_in_trait` lib warning

**Files modified:** `src/sync/client.rs`
**Commit:** 9fd991e
**Applied fix:** Desugared `async fn get_json(...)` in the public `HttpClient` trait to
an explicit `fn get_json(...) -> impl std::future::Future<Output = miette::Result<
String>>`, with a comment explaining the single-`block_on` usage means no `Send` bound
is needed. The `ReqwestClient` impl (and the test `FakeClient` impls) keep their
ergonomic `async fn` bodies, which is allowed in impl position and does not trigger the
lint. `cargo build` now emits no `async_fn_in_trait` warning; only the pre-existing,
harmless informational note that `src/main.rs` is shared by the `ferrisfilter` and `ff`
bin targets remains (explicitly called out as non-warning noise in the review).

## Skipped Issues

None — all five in-scope findings were fixed.

## Notes

- Info findings IN-01 through IN-06 were out of scope (`fix_scope = critical_warning`)
  and were not addressed.
- All fixes verified per the 3-tier strategy: re-read of the edited region (Tier 1) plus
  `cargo build` and `cargo test` after each fix (Tier 2, project-appropriate). The
  working tree is clean with no uncommitted changes.

---

_Fixed: 2026-05-25T00:00:00Z_
_Fixer: Claude (gsd-code-fixer)_
_Iteration: 1_
