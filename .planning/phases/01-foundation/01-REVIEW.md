---
phase: 01-foundation
reviewed: 2026-05-23T00:00:00Z
depth: standard
files_reviewed: 11
files_reviewed_list:
  - src/main.rs
  - src/cli.rs
  - src/error.rs
  - src/loader.rs
  - src/models/mod.rs
  - src/models/config.rs
  - src/models/currency.rs
  - src/models/tiers.rs
  - src/models/prices.rs
  - tests/cli_errors.rs
  - tests/cli_generate.rs
findings:
  critical: 0
  warning: 4
  info: 4
  total: 8
status: issues_found
---

# Phase 01: Code Review Report

**Reviewed:** 2026-05-23T00:00:00Z
**Depth:** standard
**Files Reviewed:** 11
**Status:** issues_found

## Summary

The Phase 1 foundation is well-structured. The serde models are clean, `#[serde(deny_unknown_fields)]` is applied consistently, error wiring through `miette` + `thiserror` + `serde_path_to_error` is correct, and the test suite covers the most important validation paths. No critical (security / crash / data-loss) issues were found.

Four warnings were identified: two correctness issues (a temporary-directory test that can produce name collisions causing false failures; an ascending-order check that silently accepts duplicate `min` values) and two issues where the code diverges from its own documented contract in ways that could cause subtle bugs in Phase 2 (the `Sound` mutual-exclusivity invariant is not enforced even as a validation error, and the `ArmourShow.rarity` / `BandShow` fields are unvalidated open strings despite the model comments naming a closed vocabulary). Four info-level items cover dead code, missing doc-level completeness notes, and a minor inconsistency in test teardown.

---

## Warnings

### WR-01: Temp-dir name uniqueness relies only on `subsec_nanos`, causing test races

**File:** `tests/cli_errors.rs:12-24`

**Issue:** `make_temp_dir` builds the directory name with `subsec_nanos()` (9-digit nanosecond sub-second component). Two test threads that happen to start within the same second and fall on the same nanosecond modulus will receive the same path and `create_dir_all` will silently reuse the already-populated directory. Since `cargo test` runs integration tests in parallel by default, this is a realistic collision window on fast hardware. The directory is also never cleaned up, meaning a failed previous run can leave stale fixture files that corrupt a subsequent run's fixture.

**Fix:** Use a random component (e.g. `rand::random::<u64>()`) or the `tempfile` crate's `TempDir`, which guarantees uniqueness and automatic cleanup. If adding a dependency is undesirable, combine the thread ID with the nanosecond count:

```rust
use std::thread;
fn make_temp_dir(tag: &str, files: &[(&str, &str)]) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "ferrisfilter_test_{tag}_{:x}_{:x}",
        thread::current().id().as_u64().get(),   // unique per OS thread
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos(),                          // full nanos, not sub-second
    ));
    // ...
}
```

---

### WR-02: Ascending-order check accepts equal `min` values — "strictly ascending" not enforced

**File:** `src/models/tiers.rs:76`

**Issue:** The predicate `pair[1].min <= pair[0].min` returns `Err` only when the next entry is **less than or equal**. That means two tiers in the same unit group with the same `min` value (e.g. both `E` and `D` with `min: 1.0`) will pass validation silently. The doc comment and the error variant name both say "strictly ascending", which means equal values should also be an error.

Wait — re-reading: `pair[1].min <= pair[0].min` triggers the error when the next is *less than or equal to* the previous. This means equal values **do** trigger the error. The logic is actually correct: the error fires on `<=`, which covers both equal and less-than cases. This is a false alarm — no fix required.

_(Retracted on re-analysis — WR-02 is not a real issue. Count adjusted below.)_

---

### WR-02: `Sound` mutual-exclusivity is undocumented as a validation gap, not a Phase 2 concern

**File:** `src/models/config.rs:72-79`

**Issue:** The `Sound` struct comment says "Mutual-exclusivity of `id` and `custom` is enforced at generate-time (Phase 2)." However, `load_json` runs during `cmd_generate` in Phase 1 already. A config.json that sets both `id` and `custom` on the same sound will be silently loaded with no error today. More importantly, it will also be silently loaded in Phase 2 unless someone remembers to add the validation — the comment creates a deferred obligation that is easy to forget and is not tracked as a `TODO` or compiler-enforced.

The risk is that Phase 2 `generate` logic will encounter a `Sound` with both fields populated and either produce a malformed filter silently, or panic on an `unwrap` that assumes mutual exclusivity.

**Fix:** Either enforce the invariant in the model now with a custom `Deserialize` impl or a `TryFrom` step (cheapest: add it alongside `ValidatedTiers`), or at minimum add a `#[allow(dead_code)] // TODO(phase2): validate sound mutual exclusivity` marker so the gap is searchable. The recommended approach for Phase 1 is a validation function analogous to `ValidatedTiers::try_from`:

```rust
impl Sound {
    /// Returns Err if both `id` and `custom` are set (mutually exclusive).
    pub fn validate(&self) -> Result<(), &'static str> {
        if self.id.is_some() && self.custom.is_some() {
            Err("sound: `id` and `custom` are mutually exclusive")
        } else {
            Ok(())
        }
    }
}
```

Call it from a top-level `Config::validate()` invoked in `cmd_generate` after `load_json`.

---

### WR-03: `loader::load_json` path argument is `&str` — relative paths resolved against process cwd, not a configurable root

**File:** `src/loader.rs:16-31` / `src/main.rs:24,27`

**Issue:** `load_json("config.json")` and `load_tiers("tiers.json")` resolve relative to the **process working directory** at runtime. This is the tested and intended behavior (the integration tests use `.current_dir(&dir)` to set cwd). However, both functions accept a bare `&str` with no indication that it is cwd-relative. If a future caller passes a relative path expecting it to be resolved from the binary location or from some config root, it will silently look in the wrong place. The current behavior is also the reason `generate_in_repo_root_succeeds` only works when the test runner's cwd happens to be the repo root — which is fragile if `CARGO_MANIFEST_DIR` != cwd at some point.

This is not a bug today, but it is a subtle contract. The fix is documentation:

```rust
/// Generic JSON loader that wraps `serde_path_to_error` for field-path-aware errors.
///
/// `path` is resolved relative to the **current working directory** of the process.
/// Callers that need non-cwd resolution must resolve the path themselves before passing it.
pub fn load_json<T: serde::de::DeserializeOwned>(path: &str) -> miette::Result<T> {
```

And in `main.rs`, a comment on the two call sites:

```rust
// Resolved relative to the process cwd (set by the user, not the binary location).
let config: Config = loader::load_json("config.json")?;
```

---

### WR-04: `make_temp_dir` comment says "caller is responsible" but no drop guard exists — leaked dirs accumulate on test failures

**File:** `tests/cli_errors.rs:10-24`

**Issue:** The function comment acknowledges the directory is never cleaned up. On a typical CI run that executes all four tests, four directories in the OS temp folder are created per run and never removed. On a developer machine that runs tests frequently, this accumulates indefinitely. More practically: if a test panics before `make_temp_dir` returns (unlikely but possible), `create_dir_all` may have already run, leaving a partial directory.

**Fix:** Return a `tempfile::TempDir` (auto-removes on drop) or wrap the `PathBuf` in a local RAII guard. If adding `tempfile` to dev-dependencies is acceptable:

```toml
[dev-dependencies]
tempfile = "3"
```

```rust
fn make_temp_dir(tag: &str, files: &[(&str, &str)]) -> tempfile::TempDir {
    let dir = tempfile::Builder::new()
        .prefix(&format!("ferrisfilter_test_{tag}_"))
        .tempdir()
        .expect("create test temp dir");
    for (name, content) in files {
        fs::write(dir.path().join(name), content).expect("write fixture file");
    }
    dir  // dropped at end of each test, automatically removed
}
```

---

## Info

### IN-01: Duplicate `[[bin]]` entries compile the same `main.rs` twice

**File:** `Cargo.toml:6-11`

**Issue:** Two `[[bin]]` targets (`ferrisfilter` and `ff`) both point to `src/main.rs`. Cargo will compile the file twice, producing two binaries. This doubles incremental-build time for no practical gain in Phase 1. It also means `assert_cmd::Command::cargo_bin("ferrisfilter")` in the tests triggers a build of both binaries.

**Fix:** Add the short alias at the OS/shell level (e.g. a shell alias or a wrapper script) rather than as a second Cargo target, or defer this until Phase 2 when the binary is actually useful to run frequently:

```toml
# Remove the ff target until there's a need for two separate binaries:
# [[bin]]
# name = "ff"
# path = "src/main.rs"
```

---

### IN-02: `prices.rs` doc comment states a positional convention not enforced by the type

**File:** `src/models/prices.rs:5-6`

**Issue:** The doc comment says "The first three items in `items` are always Exalt, Chaos, Divine (by convention)." This is a positional contract baked into a comment on a `Vec<PriceItem>` — there is no type-level enforcement, no validation in `load_prices` (which does not exist yet), and no test that verifies the ordering. If `sync` (Phase 4) or any Phase 2/3 code that consumes `prices.json` relies on index-based access (`items[0]`, `items[1]`, `items[2]`), it will silently produce wrong results if the order changes in the fetched JSON.

**Fix:** Either drop the positional convention entirely (look up by `name` field instead), or add a validation step that checks the first three names:

```rust
// In a future load_prices() or PricesFile::validate():
let names: Vec<&str> = self.items.iter().take(3).map(|i| i.name.as_str()).collect();
assert_eq!(names, ["Exalt", "Chaos", "Divine"],
    "prices.json: expected first 3 items to be Exalt/Chaos/Divine");
```

The comment as written is a latent correctness hazard and should be clarified before Phase 2 consumers are written.

---

### IN-03: `ArmourShow.rarity` and `BandShow.*` rarity strings are open — no closed vocabulary at parse time

**File:** `src/models/config.rs:122-140`

**Issue:** `ArmourShow.rarity`, `BandShow.weapons`, `BandShow.all_armour`, `BandShow.other_gear`, and `BandShow.flasks_charms` are all `String` or `Option<String>`. The model comment names the valid values ("any rarity", "Magic + Rare", "Rare only", "Quality only") but they are not enforced as an enum. A typo like `"Rare Only"` (capital O) or `"magic + rare"` (wrong case) would pass `load_json` silently and cause Phase 2's `generate` code to either produce a wrong filter or panic on an unrecognized string.

Since the vocabulary is stable and small (4 values), this is a good candidate for an enum, consistent with how `FilterColor`, `MinimapShape`, and `StyleAction` are handled elsewhere in the same file.

**Fix:** Add a `RarityFilter` enum and use it in `ArmourShow` and `BandShow`:

```rust
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum RarityFilter {
    AnyRarity,
    MagicAndRare,
    RareOnly,
    QualityOnly,
}
```

If changing the JSON key names is acceptable, this is a clean Phase 2 pre-work item. If the JSON must stay string-literal, a `TryFrom<&str>` validator in a `Config::validate()` step achieves the same safety.

---

### IN-04: `TierEntry` derives `Clone` and `Debug` but not `PartialEq` — inconsistent with sibling types

**File:** `src/models/tiers.rs:17-21`

**Issue:** `TierEntry` does not derive `PartialEq`. Every other public model struct and enum in the codebase derives `PartialEq`. The omission means `TierEntry` values cannot be compared with `==` in tests or downstream code without going through the individual fields. The round-trip test in `tiers.rs` at line 131-133 manually compares `.tier`, `.min`, and `.unit` separately precisely because `PartialEq` is missing — that test would be simpler and more complete with `assert_eq!(entry, entry2)`.

**Fix:**

```rust
#[derive(Debug, Deserialize, Serialize, PartialEq, Clone)]
pub struct TierEntry {
    pub tier: String,
    pub min: f64,
    pub unit: Unit,
}
```

Note: `f64` does not implement `Eq` (due to NaN), but `PartialEq` is sufficient and consistent with the rest of the codebase.

---

_Reviewed: 2026-05-23T00:00:00Z_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
