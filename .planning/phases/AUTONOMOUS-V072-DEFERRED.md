# v0.7.2 autonomous run — items deferred to the end

Collected during `/gsd-autonomous --from 43 --to 50` (started 2026-10-23). The author asked for every
human review and input to be stacked here and presented at the end, before Phase 50's author review
(REV-05). Each phase agent appends its items under its own heading.

## Deferred manual checks

1. Phase 46: press Sync economy twice within an hour for the same league. Expect the same status message both times and Last sync unchanged after the second click.
2. Phase 46: sync or open the Economy tab, quit ff, relaunch within the hour and open the Economy tab. Expect the league dropdown populated at once.
3. Phase 46: on an up-to-date build press Check for updates twice within an hour, then restart ff and press it again. Expect the same `Up to date (...)` text with the same release date each time.
4. Phase 46: with the network off press Sync economy (expect the failure message), reconnect and press it again. Expect the second click to go out and succeed.
5. Phase 46 (optional): run `ff-cli sync` through a proxy. Expect it to work and the request to carry `FerrisFilter-CLI/<ver>` as user agent.

## Art substitutes for review (Phase 48 / 47)

## Open gaps and judgement calls

- Phase 46: ff-state.json also persists the last 'up to date' version/release date and the app version (research A1, Claude's discretion) so the message replays byte-identically after a restart.
- Phase 46: a fake refresh flashes the Syncing/Checking state for one frame (accepted).
- Phase 46: `cargo clippy --locked --all-targets -- -D warnings` already reports 100+ errors in untouched files on the current floating nightly (pre-existing, e.g. single_element_loop); none come from Phase 46 files. Not fixed (out of scope).
