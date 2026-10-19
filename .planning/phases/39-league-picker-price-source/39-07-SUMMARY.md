---
phase: 39-league-picker-price-source
plan: 07
subsystem: docs
tags: [claude-md, overlay, economy, league-picker, price-source]
requires: [39-05]
provides:
  - CLAUDE.md describing the shipped league picker, price source, settings keys, status priority and screenshot flags
affects: [CLAUDE.md]
key-files:
  modified: [CLAUDE.md]
decisions:
  - "Screenshot flags documented as specified in 39-06-PLAN.md (--leagues, --league, --price-source); orchestrator to reconcile if 39-06 shipped other names"
metrics:
  tasks: 1
  files: 1
completed: 2026-10-18
---

# Phase 39 Plan 07: CLAUDE.md documentation Summary

CLAUDE.md now documents the Economy tab league picker and Price source switch and no longer mentions the pinned `sync::OVERLAY_LEAGUE`.

## What changed (CLAUDE.md only)

- "Economy sync from the overlay": pinned-league text replaced by the picker behaviour (`economyLeagues` list, worker fetch on first Economy-tab open and on refresh, never at start-up, default rule ignoring `indexed`, pick saved only on user pick, offline fallback to saved league / last priced league, `(ended)` state greys Sync and is never switched silently). CLI keeps automatic resolution and `--league`, and reads neither overlay key.
- Tabs paragraph: Economy contents rewritten (Price source, League, Last sync, Priced items), Fallback greying with the shipped tooltips, tier errors not blocking under Fallback, pending source switch marking the Economy tab and the indicator, not undone by Revert. Economy ` *` clause extended.
- `ff-settings.json` row: `league` and `price_source` keys.
- Status line priority: ended-league entry after "sync running".
- `ff` row: `--leagues`, `--league`, `--price-source` flags with a short explanation.
- Fallback-pair row: `ff-cli apply` ignores `price_source`; `generate --early-game` still forces the pair.

Verified against src: the strings `Not used with Fallback prices.`, `The fallback tiers are fixed.`, `<name> (ended)` and `<name> is no longer on poe.ninja; pick a league` exist in `src/overlay/`.

## Deviations from Plan

None. The `--leagues`/`--league`/`--price-source` names were not verified in src because 39-06 was running concurrently.

## For the Release Review

The v0.7 CHANGELOG/README should mention: league picker on the Economy tab (replaces the pinned league, so no per-league constant update), Live/Fallback Price source switch, ended-league warning.

## Commits

- 1406640: docs(39-07): document league picker and price source in CLAUDE.md

## Self-Check: PASSED
