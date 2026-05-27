---
status: partial
phase: 04-sync
source: [04-VERIFICATION.md]
started: 2026-05-25T00:00:00Z
updated: 2026-05-26T00:00:00Z
---

## Current Test

[testing complete]

## Tests

### 1. Live softcore sync run
expected: `ferrisfilter sync` (no flags) against live poe.ninja writes `prices.json` with Exalt/Chaos/Divine as the first three entries, a non-empty item list, the correct softcore trade league name, and an ISO-8601 `fetchedAt` timestamp.
result: blocked
blocked_by: third-party
reason: "The temporary softcore challenge league ('Fate of the Vaal') ended — poe.ninja's index-state now lists only Standard/Hardcore in economyLeagues, both indexed=false. The no-flag default requires an active indexed softcore trade league, so it fails safe with 'No active softcore trade league found' and leaves prices.json untouched (failure-safety held). The happy path cannot be demonstrated until a new league launches."

### 2. League flag targeting
expected: `ferrisfilter sync --hc` and `ferrisfilter sync --standard` each write `prices.json` with the correct league name resolved from the live poe.ninja index-state.
result: pass
note: "Fixed in d15e82e — --standard/--hc hoisted onto the `sync` command. `sync --standard` now parses (verified via `sync --help` listing both flags + regression test sync_help_shows_league_flags). Live `sync prices --standard` previously confirmed the resolution+write (318 items, league Standard)."

## Summary

total: 2
passed: 1
issues: 0
pending: 0
skipped: 0
blocked: 1

## Gaps

- truth: "`ferrisfilter sync --standard` (and `--hc`) resolves the league and writes prices.json"
  status: resolved
  reason: "User-facing CLI mismatch: --standard/--hc are defined on the `prices` subcommand (SyncSubcommand::Prices), but `sync` takes an Option<subcommand>, so `sync --standard` is rejected by clap as an unexpected argument. Flags only work as `sync prices --standard`. The intended/documented UX is `sync --standard`."
  severity: major
  test: 2
  artifacts: [src/cli.rs, src/sync/mod.rs]
  missing: ["--standard/--hc flags reachable from the bare `sync` invocation (e.g. hoist onto Sync or flatten the default Prices args)"]

## Notes (non-blocking observations)

- On the Standard league, `sync prices --standard` logged `Warning: failed to parse item overview for type 'UniqueSanctumRelics': missing field core` (non-fatal lenient-parse gap) and wrote 0 uniques — Standard's unique endpoints return a different/empty shape than an active league. Worth a lenient-parse guard but not blocking.
- Consider whether the no-flag `sync` should fall back to Standard (or print a hint to use `--standard`) when no challenge league is active, rather than only erroring.
