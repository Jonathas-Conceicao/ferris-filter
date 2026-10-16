---
phase: 32-rule-dividers-economy-banner
plan: 05
subsystem: overlay-rules-ui
tags: [rust, egui, overlay, sections]

requires: [32-02, 32-04]
provides:
  - "RULES tab section headers (user + locked Economy), economy rows, + Add section, Section above, WR-02 marker"
  - "Per-filter collapse persistence in ff-settings.json (never dirties the filter)"
affects: [32-06]

key-files:
  modified:
    - src/overlay/rule_list.rs
    - src/overlay/app.rs
    - src/overlay/body.rs
  created:
    - .planning/phases/32-rule-dividers-economy-banner/shots/05-sections.png
    - .planning/phases/32-rule-dividers-economy-banner/shots/05-economy-wr02.png

key-decisions:
  - "Chevron is its own click target (same toggle); during a search a header click is a no-op because the display ignores saved state"
  - "Expanded bottom row sets item spacing to 8 so the three buttons have the spec's 8pt gaps (it was 24pt)"
  - "A file with only dividers lists the headers and then draws the No rules yet heading under them"

requirements-completed: [SECT-01, SECT-02, SECT-03, SECT-04, SECT-05, RULES-03]

duration: ~75min
completed: 2026-10-16
---

# Phase 32 Plan 05: Sections in the RULES tab Summary

**The RULES list now draws `# ff:section` banners as 36pt headers with collapse, inline rename, up/down and remove, a locked collapsed-by-default Economy header with read-only rows, a third toolbar button, a `Section above` button and the WR-02 marker.**

## Accomplishments
- `rule_list.rs`: `ToolbarClick`, `SECTION_HEADER_HEIGHT`, `draw_section_header` (icon-only cluster with tooltips, no captions), `draw_economy_row`, `section_count_text` / `section_accessible_name`, `Section above` (128) beside Duplicate / Delete, WR-02 amber marker outside the flag fold, `show_list` over `ListEntry` with `build_offsets_mixed`, `ListFrame.toggled`. Footer now only for an absent or damaged economy section (`Add economy section`, 180).
- `app.rs`: `working.entries` with collapse lookup, `set_section_collapsed` saved immediately through `settings::save` (warning status on failure), rename carries the collapsed flag (`copy_section_state`), add-section / section-above follow-ups (clear search, scroll, focus + select name box), footer slot released when the economy section is present.
- 14 new headless tests in `rule_list.rs` (toggle, cluster actions, rename Enter / Escape, economy header lock and x gate, toolbar disable, Section above, search under collapsed header, WR-02) plus one in `app.rs`.

## Task Commits
1. Headers, economy rows, toolbar, Section above, WR-02 - `613159f`
2. App wiring, collapse persistence, shots - `d8128bf`

## Gates
- `cargo test` green (1238 lib tests plus integration); `cargo fmt --all --check` clean; clippy `^warning` 126 (baseline); `tests/fixtures` unchanged.

## Verification shots (viewed)
- `shots/05-sections.png` (sample filter, Endgame saved as collapsed): toolbar field + `+ ADD RULE` + `+ ADD SECTION` fit at 535pt; two headerless rules on top; `LEVELING` expanded (chevron down, rule line, `2 rules`, pencil, up, down, x) with its two rules; `ENDGAME` collapsed (chevron right, count still `2 rules`); `ECONOMY` last, collapsed, `60 rules` with padlock, no x; no locked-blocks footer; rule rows alternate ignoring headers.
- `shots/05-economy-wr02.png`: rule 1 (no conditions, followed by more rules) shows the amber triangle beside its arrows; a very long section name is cut with an ellipsis before the count with no overlap and no rule line; Economy expanded shows the padlock plus the small x and three muted read-only rows.
- Rename mode and search cannot be screenshotted (no CLI switch); both are covered by headless tests.

## Deviations from Plan
**1. `+ Add section` is wider than 140pt.** `ff_button_ex` pads the caption by 44pt, so the button is about 151pt and the search field about 241pt at 535pt (spec arithmetic: 255). Skin-wide button padding left unchanged; the field hint was already cut.

**2. Expanded bottom row gap.** The old row had 24pt between Duplicate and Delete (item spacing plus an explicit space); set to the spec's 8pt for the three buttons.

**3. `Section above` does not set `pending_scroll` in the widget;** the app follow-up does, after the action succeeds.

## Known Stubs
None.

## Threat Flags
None. T-32-16 (rename goes only through `RenameSection`, sanitised in the model), T-32-17 (existing `settings::save`, filter and dirty flag untouched), T-32-18 (economy x gated by app root and no changed-on-disk, routes to the existing popup), T-32-19 (screenshots only under `%TEMP%\ff-shot-32`) all hold.

## Self-Check: PASSED
- Commits 613159f and d8128bf exist; both PNGs present.
