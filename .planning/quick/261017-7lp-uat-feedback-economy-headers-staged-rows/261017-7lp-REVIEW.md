---
status: issues_found
depth: standard
range: 2305cca..41cecd6
critical: 0
warning: 3
info: 3
---
# Code Review: quick 261017-7lp

Risks checked and clear: generate()/seed fixture unchanged (`tags: false` in `generate_as`); styled
rewrite skips economy blocks; unknown-style report and `style_users` read user rows only; preview ids
start at `1<<48`, no collision with document ids.

## Warnings

### WR-01: `add_field` overflows its row by 2pt
`src/overlay/widgets.rs:748-756` sizes the field as `available - 84 - 8` (assumed 8pt spacing) but
`ff_text_field_ex` now leaves a 10pt gap (`src/skin/widgets.rs:1318-1327`). Fix: use the gap
constant; audit other `ff_text_field*` callers that subtract an assumed spacing.

### WR-02: staged-economy preview stale after Sync economy
`src/overlay/app.rs:2064-2082` `refresh_staged_economy` re-prices only on pending styles/tiers
change; a sync completing while Add is staged leaves old preview rows/count. Fix: invalidate
`staged_econ_inputs` when a sync finishes (or key on prices).

### WR-03: failed preview build looks like an empty section
`src/overlay/rules.rs:1301-1311`, `1593-1611`: `build_preview` returning `None` still stages
`Pending::Add { blocks: 0 }` silently. Fix: surface an error via `list_status` and do not stage.

## Info

### IN-01: stale comments
`src/overlay/rule_list.rs` (~1500) search-row comment describes removed behaviour;
`src/emitter.rs:57-62` doc comment over-long line.

### IN-02: `count_blocks` dead outside its test (`src/overlay/rules.rs:469`).

### IN-03: literal line breaks inside test string literals
`src/dynamic.rs:605-613`, `src/overlay/rules.rs:417-436`; use `\n`.

Unrated: preview ids restart on every re-price (an expanded preview row may point at another row
after a re-price); pricing failure caches inputs so no retry until inputs change.
