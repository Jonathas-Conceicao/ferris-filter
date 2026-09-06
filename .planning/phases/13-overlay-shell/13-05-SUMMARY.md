---
phase: 13-overlay-shell
plan: 05
subsystem: ui
tags: [manual-verification, global-hotkey, egui, eframe, windows-overlay, docs]

# Dependency graph
requires:
  - phase: 13-01
    provides: "the always-on-top, opaque, sub-screen stripe window and the ff/ff-cli binary split"
  - phase: 13-02
    provides: "the Ferris tray icon, Show/Quit menu, and global hotkey registration/delivery"
  - phase: 13-03
    provides: "the visibility state machine and raw-HWND SW_SHOWNOACTIVATE/SW_HIDE reveal/hide"
  - phase: 13-04
    provides: "the themed, read-only dynamic.json rule list rendered inside the stripe"
provides:
  - "13-MANUAL-VERIFICATION.md — the committed, durable record of all five Phase 13 roadmap success criteria verified live against a running Path of Exile 2, all PASS"
  - "CLAUDE.md and CHANGELOG.md documentation of the overlay's windowed-mode requirement, the ff/ff-cli split, and the toggle hotkey"
  - "The toggle hotkey rebound from Shift+Z to Ctrl+Shift+Z (src/overlay/hotkey.rs), an author-requested change made during this verification sitting"
affects: [14-rule-editor]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "In-game manual verification checklists are committed files with per-criterion Result lines, not chat transcripts — the phase's sign-off is a durable, greppable record (T-13-20)"
    - "A checkpoint:human-verify plan can surface a live, author-requested config change (the hotkey rebind) mid-verification; that change ships as its own commit outside the checklist file, and the checklist is corrected afterward to match what shipped rather than left describing stale behavior"

key-files:
  created:
    - .planning/phases/13-overlay-shell/13-MANUAL-VERIFICATION.md
  modified:
    - CLAUDE.md
    - CHANGELOG.md
    - src/overlay/hotkey.rs
    - src/overlay/app.rs
    - src/overlay/mod.rs
    - .planning/phases/13-overlay-shell/13-CONTEXT.md

key-decisions:
  - "Toggle hotkey rebound from Shift+Z to Ctrl+Shift+Z at the author's request during live verification, to keep the bare Shift+Z combination free for other future bindings — not because the PoE2 chat-box collision was judged intolerable. Shipped in commit 15bff2f, prior to this plan's own commits, with cargo test re-verified at 250/250 afterward."
  - "The game-dimming request (author: the overlay should darken the game slightly, not just show a mild shadow around the window) is recorded as an observation, not implemented. Dimming the game behind the stripe would require a second, full-screen translucent window, which conflicts directly with this phase's SHELL-03 / D-12 / D-13 constraints (opaque, non-passthrough, never sized to the full screen — chosen specifically to avoid egui #4458 and #2537). Left for a later phase to resolve against that constraint."

requirements-completed: [SHELL-01, SHELL-02, SHELL-03]

# Metrics
duration: ~16min agent-visible (plus live in-game play time by the author between commits, not separately measured)
completed: 2026-09-06
---

# Phase 13 Plan 05: Overlay Shell — In-Game Manual Verification Summary

**All five Phase 13 roadmap success criteria verified live against a running Path of Exile 2 by the author — every criterion PASS, plus an author-requested hotkey rebind to `Ctrl+Shift+Z` made and shipped mid-verification.**

## Performance

- **Duration:** ~16 min of agent-visible work spanning three commits (`c348c49` → `15bff2f` → `660b9e2`); the elapsed wall-clock time also includes the author actively playing Path of Exile 2 to exercise each checklist step, which is not separately measured.
- **Tasks:** 2 (1 auto, 1 checkpoint:human-verify)
- **Files modified:** 7 across the plan (checklist, CLAUDE.md, CHANGELOG.md, plus the out-of-band hotkey rebind touching `src/overlay/hotkey.rs`, `src/overlay/app.rs`, `src/overlay/mod.rs`, `13-CONTEXT.md`)

## Accomplishments

- **`.planning/phases/13-overlay-shell/13-MANUAL-VERIFICATION.md`** — an ordered, one-sitting, five-section checklist (one per roadmap success criterion), each with numbered steps, an exact expected observation, and a filled-in `Result:` line. All five: **PASS**.
  - **SC1** (always-on-top, incl. PoE2's own panels — inventory, passive tree, map device): PASS, author approved.
  - **SC2** (global hotkey while the game holds focus): PASS. Residual check surfaced the author's decision to rebind the hotkey (see Decisions).
  - **SC3** (opaque, non-passthrough, sub-screen): PASS — "the click focus is working, it captures input once clicked," and the click did not reach the game behind the stripe.
  - **SC4** (game unaffected, including quitting mid-game): PASS — "quitting works, doesn't bug the game or anything."
  - **SC5** (no binary named `overlay`): PASS — the binary is `ff`.
- **CLAUDE.md** documents the overlay in the repository-layout table, the windowed/windowed-fullscreen requirement (exclusive fullscreen is undetected and will not work — D-21), and the toggle hotkey.
- **CHANGELOG.md**'s `[Unreleased]` section names the overlay shell work (the `ff` GUI binary, `ff-cli` console binary, tray icon, hotkey toggle, read-only rule list).
- **The toggle hotkey changed from `Shift+Z` to `Ctrl+Shift+Z`** (`src/overlay/hotkey.rs`'s `TOGGLE_MODIFIERS = Modifiers::CONTROL.union(Modifiers::SHIFT)` and its unit test), requested by the author live during this verification to free up the bare `Shift+Z` combination. `cargo test` was re-run and green (250/250) after the change. The checklist's binding references were corrected from `Shift+Z` to `Ctrl+Shift+Z` to match what actually shipped, rather than describing a binding that no longer exists.

## Task Commits

1. **Task 1: Write the in-game checklist and document the overlay** - `c348c49` (docs)
2. **Out-of-band, author-requested during Task 2's live verification: rebind the hotkey to `Ctrl+Shift+Z`** - `15bff2f` (feat)
3. **Task 2: Record the author's in-game verification results** - `660b9e2` (docs)

**Plan metadata:** committed as part of this SUMMARY's own commit.

## Files Created/Modified

- `.planning/phases/13-overlay-shell/13-MANUAL-VERIFICATION.md` - The five-criterion checklist, now carrying the author's filled-in PASS results and corrected `Ctrl+Shift+Z` binding references
- `CLAUDE.md` - Overlay repository-layout entry, windowed-mode requirement, toggle hotkey documentation
- `CHANGELOG.md` - `[Unreleased]` entries for the overlay shell work
- `src/overlay/hotkey.rs` - Toggle hotkey changed from `Shift+Z` to `Ctrl+Shift+Z`
- `src/overlay/app.rs`, `src/overlay/mod.rs` - Doc comments and tray tooltip strings updated to the new binding
- `.planning/phases/13-overlay-shell/13-CONTEXT.md` - D-09 annotated SUPERSEDED by the rebind

## Decisions Made

- **Hotkey rebind to `Ctrl+Shift+Z`.** During live verification the author reported the hotkey "works too, but let's have ctrl+shift+Z for now to have that bind free" — a request to reclaim the bare `Shift+Z` combination for other future use, not a report that the PoE2 chat-box collision (D-09's accepted trade) was intolerable in practice. Shipped immediately as commit `15bff2f`, ahead of this plan's own remaining commits, with the full suite re-verified green (250/250) afterward.
- **Game-dimming request not implemented.** The author observed the overlay "doesn't darken the game screen, only a very mild shadow around the window right now," and said it should darken the game "just a little bit," never fully. This is a genuine design ask but out of scope for this plan: dimming the game behind the stripe requires a second, full-screen translucent window, which directly conflicts with SHELL-03 / D-12 / D-13 (opaque, non-passthrough, never sized to the full screen — the configuration chosen specifically to dodge egui #4458 and #2537). Recorded as a note in the checklist and as an issue below for a later phase to pick up against that constraint, rather than implemented here.

## Deviations from Plan

### Auto-fixed Issues

None — the hotkey rebind was an explicit, author-directed change made live during the Task 2 checkpoint, not an auto-fix under Rules 1-3. It is documented here and in the checklist rather than silently folded into "as planned."

---

**Total deviations:** 0 auto-fixed. One author-directed, in-checkpoint change (the hotkey rebind), documented above and not treated as a deviation rule.
**Impact on plan:** None on scope — the rebind is a one-constant change plus its unit test and doc-string updates, exactly the remedy D-09 already anticipated for this exact case.

## Issues Encountered

- **Game-dimming is not implemented and is architecturally blocked by this phase's own constraints.** The author's request for the overlay to slightly darken the game behind the stripe cannot be satisfied by adjusting the stripe's own background alpha (already fully opaque, by design) — it needs a second, full-screen, click-through, translucent window layered behind the stripe. That directly reintroduces the shape of window this phase deliberately avoided (D-12/D-13, chasing away from egui/eframe Windows overlay defects #4458 and #2537). Whoever picks this up next should start from that constraint — a naive full-screen overlay window is very likely to reproduce the defects this phase was built to dodge — rather than rediscovering it from scratch. Recorded in `13-MANUAL-VERIFICATION.md`'s "The one judgment no test can make" section for permanence.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- SHELL-01, SHELL-02 and SHELL-03 are now verified against a real, running Path of Exile 2 session, not just against code review — Phase 13 is complete.
- Phase 14 (Rule Editor + Apply) can build directly on the stripe, tray, hotkey and themed read-only list this phase shipped; `Ctrl+Shift+Z` is the binding it should assume and document going forward.
- The game-dimming request is an open, recorded design question for whichever phase next touches the overlay's window configuration — it is not blocking Phase 14's editing-UI work, which does not need a second window.
- No other blockers.

---
*Phase: 13-overlay-shell*
*Completed: 2026-09-06*

## Self-Check: PASSED

- FOUND: .planning/phases/13-overlay-shell/13-MANUAL-VERIFICATION.md
- FOUND: .planning/phases/13-overlay-shell/13-05-SUMMARY.md
- FOUND: CLAUDE.md
- FOUND: CHANGELOG.md
- FOUND: src/overlay/hotkey.rs
- FOUND commit: c348c49 (docs(13-05): in-game verification checklist + windowed-mode docs)
- FOUND commit: 15bff2f (feat(13-02): change the toggle hotkey to Ctrl+Shift+Z)
- FOUND commit: 660b9e2 (docs(13-05): record author's in-game verification results)
