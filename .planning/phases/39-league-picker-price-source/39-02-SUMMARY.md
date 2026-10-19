---
phase: 39-league-picker-price-source
plan: 02
subsystem: overlay
tags: [price-source, apply, tiers]
requires: [39-01]
provides:
  - FilterInput.policy, apply_everything(.., policy, ..), staged_economy_body(.., policy)
  - ConfigState::set_tiers_in_use / tiers_in_use
  - App::sync_tiers_in_use
affects: [39-05]
key-files:
  modified: [src/overlay/apply_all.rs, src/overlay/app.rs, src/overlay/config_state.rs]
key-decisions:
  - "OD-3: Fallback sets tiers_in_use=false; has_tier_errors/tiers_error return nothing, tier_rows keeps row errors"
requirements-completed: [LEAGUE-04, LEAGUE-05]
completed: 2026-10-18
---

# Phase 39 Plan 02: Price source through Apply and the tiers-in-use flag Summary

Apply and the read-only economy preview now price with the saved Live/Fallback source, and a greyed Tiers editor cannot block Apply under Fallback.

## What changed

- `FilterInput.policy` replaces both hard-coded `Auto` uses in `apply_all::prepare`.
- `apply_everything` and `staged_economy_body` take a `PriceSourcePolicy`; `run_apply`, `stage_add_economy` and `refresh_staged_economy` pass `settings.price_policy()`. `staged_econ_inputs` is now `(styles, tiers, policy)`, so a policy-only change re-prices the preview. `PriceSourcePolicy` already derived `Copy + PartialEq`; compile.rs untouched.
- `ConfigState.tiers_in_use` (default true). While false, `has_tier_errors()` and `tiers_error()` report nothing, so `can_apply` and `validate` pass. `tier_rows()` still carries row errors, `tiers_dirty()` is unaffected, and `prepare_write` writes the last valid `self.tiers` (tiers.json bytes unchanged, tested).
- `App::sync_tiers_in_use` runs at the start of `run_apply`, at the end of `reload`, and after the config-only reload and `reload_changed_files`. The per-frame call before the Apply gate is left to plan 05.

## Tests

`cargo test --lib -- overlay::` : 814 passed, 0 failed (run with FERRISFILTER_HOME / FERRISFILTER_GAME_DIR pointing at empty temp dirs). New: Auto vs ForceFallback apply (different filter text, `used_fallback`), Auto without prices.json, tier gate on/off/on, general-error variant, App-level gate via `sync_tiers_in_use`, preview key re-pricing on policy change. rustfmt applied; `cargo clippy --all-targets` reports nothing in the three touched files.

## Deviations from Plan

- Both tasks went into one commit (e450f25): the `app.rs` edits of the two tasks interleave (`run_apply` uses both the policy and `sync_tiers_in_use`), so a clean split was not practical.
- Unrelated to the plan: `ConfigState::adopt` and `revert` call `resync_buffers`, which rebuilds tier text from the last valid values. After an Apply under Fallback, an ignored bad tier text is therefore reset to the last valid value (that value is what was written). Existing behaviour, kept.

## Decisions

- Live with an unusable prices.json still falls back with the reason (unchanged `Auto` path); Fallback shows the existing " Used the fallback prices." extra on Apply.

## For the Release Review

- With price source Fallback, Apply writes the economy section from the built-in fallback pair even when a live `prices.json` exists, and tier cutoff errors no longer block Apply (they return on Live).

## Known Stubs

None. The Fallback toggle UI and per-frame sync arrive in plans 04/05.

## Self-Check: PASSED

Commit e450f25 exists; all three modified files present.
