---
phase: 04-sync
reviewed: 2026-05-25T00:00:00Z
depth: standard
files_reviewed: 13
files_reviewed_list:
  - Cargo.toml
  - mapping.json
  - src/cli.rs
  - src/lib.rs
  - src/main.rs
  - src/sync/client.rs
  - src/sync/league.rs
  - src/sync/mapping.rs
  - src/sync/metadata.rs
  - src/sync/mod.rs
  - src/sync/models.rs
  - src/sync/prices.rs
  - tests/sync_prices.rs
findings:
  critical: 0
  warning: 5
  info: 6
  total: 11
status: issues_found
---

# Phase 4: Code Review Report

**Reviewed:** 2026-05-25T00:00:00Z
**Depth:** standard
**Files Reviewed:** 13
**Status:** issues_found

## Summary

The phase 04 `sync` command is generally well-structured. The injectable `HttpClient`
seam keeps tests network-free, HTTP errors are surfaced (not panicked), `error_for_status()`
is checked, and the atomic-write failure-safety guarantee is implemented correctly via
`NamedTempFile::persist` in the same directory — the integration test
`failure_safety_byte_equal_on_network_error` proves an existing `prices.json` survives a
failed sync byte-for-byte. serde models are lenient where they must be (`#[serde(default)]`
on collection fields), and the strict `deny_unknown_fields` on the *output* model is
appropriate.

No critical (security/data-loss) issues found. The atomic-write contract holds. The
findings below are correctness edge-cases in the currency conversion math, dead/no-op
code, and the requested build warning.

Key concerns: a potential divide-by-zero producing `inf`/`NaN` in chaos-primary
conversion (WR-01), a no-op metadata refresh loop that makes `sync metadata` misleadingly
claim to "refresh" data it never changes (WR-04), and the `async_fn_in_trait` lib warning
(WR-05, the one `cargo build` emits).

## Warnings

### WR-01: Divide-by-zero in chaos-primary conversion yields `inf`/`NaN` prices

**File:** `src/sync/prices.rs:126-133`
**Issue:** When `core.primary == "chaos"`, the conversion divides by the divine and exalted
rates: `divine_rate.map(|r| primary_value / r)` and `exalt_rate.map(|r| primary_value / r)`.
If poe.ninja returns a `0.0` rate (plausible for an off-season / thin market where a rate key
is present but zero), the result is `inf` (or `NaN` if `primary_value` is also 0.0). That
poisoned value is written into `prices.json` as `div`/`exalt`, and `inf`/`NaN` do not
round-trip through JSON cleanly (serde_json serializes them as `null`, silently dropping the
price). The `divine` branch (line 118) is safe because it only multiplies, so this is unique
to the `chaos` primary path.
**Fix:** Guard the divisor before dividing:
```rust
let div = divine_rate.and_then(|r| (r != 0.0).then_some(primary_value / r));
let exalt = exalt_rate.and_then(|r| (r != 0.0).then_some(primary_value / r));
```
Optionally also reject non-finite results in the `divine`/unknown branches with
`.filter(|v| v.is_finite())` for defense in depth.

### WR-02: Anchor validation accepts a zero/null Divine `div`, weakening the "valid rates" guarantee

**File:** `src/sync/prices.rs:331-341`
**Issue:** The `validate()` closure only hard-fails when `Chaos Orb.chaos` is present but zero.
`div_val` (Divine) and `exalt_val` (Exalted) are computed and then explicitly discarded
(`let _ = (div_val, exalt_val); // non-fatal if null`). Because Divine Orb is the cross-rate
anchor (`primary == "divine"` makes its `div = 1.0`), a run where Divine ends up with
`div == None` or `0.0` indicates the rate table was malformed — yet the write proceeds and
overwrites a previously-good `prices.json` with a degraded one. The comment "aborting write
to preserve existing prices.json" overstates the protection actually provided.
**Fix:** Treat a missing/zero Divine anchor as fatal too, matching the Chaos guard:
```rust
if div_val == 0.0 {
    return Err(miette::miette!("Divine Orb div anchor is zero/missing — aborting write"));
}
```

### WR-03: Identical `if`/`else` branches — dead `base_type` logic

**File:** `src/sync/prices.rs:163-168`
**Issue:** Both arms of the `if base_type_is_name { ... } else { ... }` produce
`name.clone()`, so `base_type_is_name` has no effect on the result. For uniques
(`base_type_is_name == false`), the comment claims "use name as baseType (best available)",
but for a unique the poe.ninja `name` is the unique's *display name* (e.g. "Voidforge"),
not its filter `BaseType` (e.g. "Infernal Sword"). Emitting the display name as `baseType`
will produce a filter `BaseType ==` that never matches the dropped item. This is a latent
correctness bug for the uniques category, currently masked because the seed contains no
uniques (`seeded_prices_no_uniques` test).
**Fix:** Either collapse the dead branch to a single `let base_type = name.clone();` and
document that uniques are not yet supported, or resolve the true base type for uniques
(e.g. from the metadata mapping) before relying on `base_type_is_name`. At minimum, make
the distinction real rather than silently identical.

### WR-04: `sync metadata` refresh loop is a no-op — output message overstates behavior

**File:** `src/sync/metadata.rs:88-110`
**Issue:** The `map(|ot| { ... ot })` closure computes `repoe_normalized` and branches on
whether RePoE confirms each class, but **both branches do nothing** (the bodies are empty
comments) and the closure returns `ot` unchanged. So the "refreshed" mapping is byte-identical
to the committed one except for re-serialization. The command then prints
`"mapping.json refreshed (... base items, ... uniques)"`, implying the RePoE data was
incorporated when it was not. `base_items`/`uniques` are fetched, validated, and discarded.
This is misleading and means a class-name drift in RePoE will never actually update
`mapping.json`.
**Fix:** Either (a) implement the intended re-derivation (update `ot.class` when RePoE
provides a more canonical normalized spelling, or emit a warning for unconfirmed classes), or
(b) if a no-op pass-through is intentional for now, remove the dead `repoe_classes`
computation and the empty if/else, and change the message to accurately describe what
happened (e.g. "validated mapping.json against N RePoE base items").

### WR-05: `async_fn_in_trait` lib warning (the warning `cargo build` emits)

**File:** `src/sync/client.rs:6`
**Issue:** `cargo build` emits exactly one lib warning:
`warning: use of \`async fn\` in public traits is discouraged as auto trait bounds cannot
be specified` on `async fn get_json(...)` in the public `HttpClient` trait. Because the
returned `Future` has no `Send` bound, the trait cannot be used across threads in a
multithreaded executor context without further work — acceptable here since the futures are
driven on a single `block_on`, but the warning will keep firing and obscure future warnings.
(Note: the build also prints an *informational* note that `src/main.rs` is shared by the
`ferrisfilter` and `ff` bin targets — that is not a compiler warning and is harmless, but it
adds noise.)
**Fix:** Desugar to an explicit future to silence it and make the bound explicit:
```rust
pub trait HttpClient {
    fn get_json(
        &self,
        url: &str,
        query: &[(&str, &str)],
    ) -> impl std::future::Future<Output = miette::Result<String>>;
}
```
Or, if the warning is accepted as intentional, add `#[allow(async_fn_in_trait)]` on the
trait with a comment explaining the single-threaded usage, so it stops masking new warnings.

## Info

### IN-01: `secs == 0` epoch fallback produces a misleading timestamp

**File:** `src/sync/prices.rs:357-364`
**Issue:** If `SystemTime::now().duration_since(UNIX_EPOCH)` errors (clock before epoch), the
fallback `fetched_at` is the string `"0"`, which is not a valid ISO-8601 timestamp and
contrasts with the `format_utc_timestamp` output used on the success path.
**Fix:** Fall back to a valid sentinel like `"1970-01-01T00:00:00Z"` (call
`format_utc_timestamp(0)`) so downstream consumers always parse a well-formed timestamp.

### IN-02: `format_utc_timestamp` ignores leap seconds and has no upper-bound test

**File:** `src/sync/prices.rs:385-435`
**Issue:** The hand-rolled date math is correct for normal Gregorian dates (leap-year rule is
right), but there are no unit tests for it and it silently assumes UTC == TAI (ignores leap
seconds, which is fine for a filename timestamp). Given it is non-trivial arithmetic, it is
worth a couple of pinned assertions (e.g. a known epoch second → expected string) to prevent
regressions.
**Fix:** Add a test: `assert_eq!(format_utc_timestamp(0), "1970-01-01T00:00:00Z");` and one
post-2020 value computed independently.

### IN-03: `items.get(0)` triggers clippy `get_first` lint

**File:** `src/sync/prices.rs:327`
**Issue:** `items.get(0)` is flagged by clippy (`clippy::get_first`); prefer `items.first()`.
Minor, but `cargo clippy` will warn.
**Fix:** `let exalt = items.first().filter(|i| i.name == "Exalted Orb");`

### IN-04: Unused `call_count` machinery beyond a trivial assert

**File:** `src/sync/metadata.rs:159,182-183,292`
**Issue:** `FakeClient::call_count` is incremented under a `Mutex` but only asserted as
`count >= 1` in `both_urls_are_fetched`, which the test comment itself admits is weak ("we
may only see 1 call"). The mutex/counter adds complexity for an assertion that does not verify
the stated intent (both URLs fetched).
**Fix:** Either record the set of fetched URLs and assert both appear, or drop the counter and
simplify the fake. Test-only, low priority.

### IN-05: `normalize_item_class` fallback splits acronyms/digits oddly

**File:** `src/sync/mapping.rs:116-125`
**Issue:** The fallback inserts a space before *every* interior uppercase char, so an input
like `"ESArmour"` becomes `"E S Armour"` and `"Tier2Map"` keeps the digit attached
oddly. This only matters for RePoE classes not in the override table; today all known PoE2
classes are covered by the table, so it is latent. Worth noting because WR-04's intended
re-derivation would route real RePoE values through this path.
**Fix:** When implementing WR-04, prefer extending the override table over trusting the
heuristic, and/or only split on a lower→upper transition (`prev.is_lowercase() && c.is_uppercase()`).

### IN-06: `seed_prices` bin and `src/models/prices.rs` are in scope but not in the review file list

**File:** `Cargo.toml:19-21`
**Issue:** `Cargo.toml` declares a third bin target `seed_prices` (`src/bin/seed_prices.rs`)
that participates in phase 04 (seeding the committed `prices.json`) but was not included in
the review file list, and `src/models/prices.rs` (the `PricesFile`/`PriceItem` schema that
`write_prices_atomic` depends on) was likewise out of the explicit list. Both were read for
context here and look sound (`deny_unknown_fields` on output is correct), but the
`seed_prices.rs` body was not reviewed.
**Fix:** Consider adding `src/bin/seed_prices.rs` to a follow-up review pass if it is part of
the phase 04 deliverable.

---

_Reviewed: 2026-05-25T00:00:00Z_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
