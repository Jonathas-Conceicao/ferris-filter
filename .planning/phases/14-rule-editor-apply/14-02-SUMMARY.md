---
phase: 14-rule-editor-apply
plan: 02
subsystem: overlay
tags: [rust, egui, immediate-mode-gui, filter-editor]

# Dependency graph
requires:
  - phase: 14-rule-editor-apply
    plan: 01
    provides: overlay::edit::WorkingRules (RuleId-keyed add/delete/move_up/move_down/toggle_enabled), overlay::store::save (atomic write)
provides:
  - "The editable accordion: OverlayApp now owns a Result<WorkingRules, String> plus Option<Config>, expansion keyed by RuleId, a confirm_delete: Option<RuleId>, and a dirty flag"
  - "widgets::circular_toggle and widgets::arrow_button — PoE2-idiom painted controls built on the same allocate_exact_size -> interact -> paint primitive as header_plate's close glyph, no custom egui::Widget impl"
  - "Per-frame coalesced autosave: exactly one store::save call site, gated on dirty, guaranteed (by program order, not a test) to run before the deferred Quit -> ViewportCommand::Close send"
affects: [14-03, 14-04]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Deferred-action rendering: row controls only record a RowAction enum value during the render pass (an immutable walk of WorkingRules::rows()); the actual mutation happens once, after the loop, avoiding any conflict between iterating and mutating the same WorkingRules"
    - "CollapsingHeader::id_salt(RuleId).open(Some(bool)) driven entirely by the app's own Option<RuleId>, never egui's internal per-id memory — selection survives reordering by construction (D-19)"
    - "Painted-not-typed controls: circular_toggle and arrow_button both paint over ui.interact(..., Sense::click()) rather than a custom Widget impl or a text glyph, following header_plate's established Phase-13 shape"

key-files:
  created: []
  modified:
    - src/overlay/widgets.rs
    - src/overlay/app.rs

key-decisions:
  - "Row summaries are recomputed from the live WorkingRules every frame via view_model::rule_rows (config.rules synced from working.to_rules() first), rather than caching/duplicating the summary logic in app.rs — cheap at this file's scale (a handful of rules) and guarantees the header text can never drift from the working model"
  - "Quit is only *requested* during tray-menu polling; the actual ViewportCommand::Close send is deferred to after the CentralPanel body renders and the autosave runs, so a save issued earlier in the same ui() call is guaranteed (by eframe's synchronous single-threaded call structure) to complete before Close is even dispatched — RESEARCH Risk 4 / T-14-06, asserted by a line-order shell check in the plan's own verify block since no unit test can catch this class of ordering regression"
  - "arrow_button's disabled state uses Sense::hover() instead of Sense::click(), so a disabled control structurally cannot report clicked() — the row keeps the arrow visible-but-inert at the list ends instead of hiding it, so the row's control layout never jumps between rows"
  - "The four widget click-simulation tests construct a headless egui::Context via run_ui with a synthetic two-event PointerButton press+release, discovering the widget's rect in an unclicked first frame and reusing that exact rect for the click in a second frame — deterministic because layout depends only on prior state, which is identical between the two frames"

requirements-completed: [EDIT-01, EDIT-02]

# Metrics
duration: ~70min
completed: 2026-09-07
---

# Phase 14 Plan 02: The Accordion, Reorder/Add/Delete/Toggle, and Autosave Summary

**Turns the Phase 13 read-only rule stripe into a fully editable accordion — expand-in-place rows keyed by `RuleId`, painted circular toggle and reorder arrows, an inline delete confirm, an `+ ADD RULE` button, and a per-frame coalesced autosave that is provably ordered ahead of Quit.**

## Performance

- **Duration:** ~70 min
- **Tasks:** 3 completed
- **Files modified:** 2

## Accomplishments

- `widgets::circular_toggle` and `widgets::arrow_button` (`ArrowDirection::{Up,Down}`) — both painted, not typed, both built on the exact `allocate_exact_size -> ui.interact -> ui.painter()` sequence `header_plate`'s `✕` already established, no custom `egui::Widget` impl, no bespoke hit-testing. Covered by four headless `egui::Context::run_ui` click-simulation tests (a genuinely new test technique for this codebase — two-frame rect-discovery-then-click, not just a "does it panic" smoke test).
- `OverlayApp` now owns the editable model: `rules: Result<WorkingRules, String>`, `config: Option<Config>`, `store_path`, `expanded: Option<RuleId>`, `confirm_delete: Option<RuleId>`, `dirty: bool`, `last_save_error: Option<String>`. Every row renders through `CollapsingHeader::id_salt(id).open(Some(is_open))`, so expansion state is the app's own `Option<RuleId>`, never a second egui-internal copy that could drift (RESEARCH Pattern 3).
- Reorder, enable/disable, delete-with-confirm, and add are all wired: up/down `arrow_button`s call `WorkingRules::move_up`/`move_down` (disabled-not-hidden at the list ends); `circular_toggle` calls `toggle_enabled`; a delete control sets `confirm_delete`, and while pending that row's controls become `Delete? [Yes] [Cancel]` inline (D-08, no modal); `+ ADD RULE` calls `WorkingRules::add` and immediately expands the new rule.
- Autosave is one `store::save` call site per frame, gated on `dirty`. Quit is now only *requested* during tray-menu polling — the real `ViewportCommand::Close` send moved to after the body renders and the autosave runs, closing the "lose the last edit on Quit" gap RESEARCH Risk 4 named.
- Roadmap success criterion 1 (list/add/delete/reorder, reordering changes precedence) and criterion 2 (disabling a rule removes its filter block) are both now reachable from the stripe itself, not just from `overlay::edit`'s library-level tests (14-01) — this plan is the click-level proof surface for what 14-01 proved at the library level.

## Task Commits

Each task was committed atomically:

1. **Task 1: The painted controls — circular toggle and reorder arrows** - `7ec486b` (feat)
2. **Task 2: The accordion — expansion keyed by RuleId, scrolled into view** - `60fcffa` (feat)
3. **Task 3: Reorder, add, enable/disable, inline delete confirm — and the autosave** - `d8d7cf5` (feat)

**Plan metadata:** committed with this SUMMARY (see final commit below)

## Files Created/Modified

- `src/overlay/widgets.rs` — added `circular_toggle`, `ArrowDirection`, `arrow_button`, and a `#[cfg(test)]` module with four headless click-simulation tests; reworded the module header to name both new controls
- `src/overlay/app.rs` — replaced the read-only `rows: Result<Vec<RuleRow>, String>` with the editable `rules`/`config`/`store_path`/`expanded`/`confirm_delete`/`dirty`/`last_save_error` fields; the accordion render loop; the `RowAction` deferred-mutation enum and its apply step; the coalesced autosave; the deferred Quit send

## Decisions Made

See `key-decisions` in the frontmatter — the deferred-action rendering pattern, summary-recomputed-every-frame, disabled-not-hidden arrows, and the Quit-deferral ordering are the load-bearing ones. All four were anticipated by RESEARCH (Patterns 1/3/5, Risk 4) rather than discovered ad hoc during implementation.

## Deviations from Plan

**1. [Rule 1 — wording, not logic] Reworded two prose comments so the plan's own line-order verification check would find the correct occurrence**
- **Found during:** Task 3, running the plan's `<verify>` shell check
- **Issue:** The plan's automated ordering check does `grep -n 'ViewportCommand::Close' src/overlay/app.rs | head -1` and compares that line number against `store::save`'s. My first draft's explanatory comments (near the tray-poll block and near the save call) both contained the literal string `` `ViewportCommand::Close` `` in prose, appearing earlier in the file than the actual `send_viewport_cmd` call — so `head -1` picked up a comment, not the code, and the ordering check failed even though the real code was already correctly ordered.
- **Fix:** Reworded both comments to describe "the deferred close-the-viewport send" / "the close command" instead of the literal API path, so the only occurrence of the exact string `ViewportCommand::Close` in the file is the actual call site.
- **Files modified:** `src/overlay/app.rs`
- **Committed in:** `d8d7cf5` (part of the Task 3 commit — caught and fixed before commit, never shipped broken)

None of the above changed behavior or design — both are wording-only fixes to satisfy the plan's own grep-based verification.

## Known, Pre-Existing Verification Discrepancy (not introduced by this plan)

The plan's top-level `<verification>` block asserts `grep -rnE 'compile_store|write_filter|reset_store' src/overlay/` prints nothing. This is true for both files this plan modified (`app.rs`, `widgets.rs`) but **not** true for `src/overlay/store.rs`, whose module-header prose (from plan 14-01, commit `3f44241`, already reviewed and merged) mentions `dynamic::compile_store`/`write_filter` by name while explaining that they remain Apply-only entry points called from `app.rs`, not from `store.rs`. This is a doc-comment mention, not a call — `store.rs` contains no actual invocation of either function — and `store.rs` is outside this plan's `files_modified` (`src/overlay/widgets.rs`, `src/overlay/app.rs` only), so it was left as-is per the scope-boundary rule (pre-existing conditions in files this plan does not touch are out of scope). Flagging here rather than silently letting the grep "pass" by coincidence.

## Known Stubs

- `render_rule_body` in `src/overlay/app.rs` renders only a placeholder label naming the rule's style (`"(\"{style}\" rule — condition editors arrive in plan 14-03)"`). This is the plan's explicit, deliberate scope fence — the objective states "leaving it near-empty here is deliberate, so the accordion, the identity handling and the save path can be judged on their own." Plan 14-03 replaces this function's body with the style/class/rarity/defence/sockets/quality/corrupted editors. Not a defect; tracked here per the summary protocol.

## Issues Encountered

None beyond the wording-only fix documented above under Deviations.

## User Setup Required

None — no external service configuration required.

## Next Phase Readiness

- Plan 14-03 can implement `render_rule_body`'s real contents directly against `overlay::edit`'s per-condition set/clear functions (already built and tested in 14-01) and the `Rule`/`When` references already flowing into `render_rule_body(ui, rule)` — no further plumbing is a prerequisite. Note `render_rule_body` currently takes `&Rule` (read-only); 14-03 will need to change this call site to pass a mutable reference (or thread a `RuleId` through so its own controls can call `working.rule_mut(id)`), since condition edits need write access.
- `self.dirty`/`self.last_save_error`/the autosave call site are all ready for 14-03's condition editors to set `dirty = true` on a real change — no autosave-side changes anticipated.
- Plan 14-04 (Apply) can add its own control below the existing `+ ADD RULE` button / status area; `self.config` already holds the full `Config` autosave writes from, ready for `dynamic::compile_store`/`write_filter` to consume unchanged.
- No blockers or concerns.

---
*Phase: 14-rule-editor-apply*
*Completed: 2026-09-07*

## Self-Check: PASSED

All created/modified files (`src/overlay/widgets.rs`, `src/overlay/app.rs`, this SUMMARY.md)
confirmed present on disk. All three task commit hashes (`7ec486b`, `60fcffa`, `d8d7cf5`)
confirmed present in `git log`.
