---
status: diagnosed
phase: 14-rule-editor-apply
source: 14-01-SUMMARY.md, 14-02-SUMMARY.md, 14-03-SUMMARY.md, 14-04-SUMMARY.md
started: 2026-09-17T00:00:00Z
updated: 2026-09-17T00:00:00Z
---

## Scope note

Phase 14's twelve original manual checks are **already approved** (author, 2026-09-08/09 — see
`14-MANUAL-VERIFICATION.md`, which records "Nothing outstanding. All twelve checks across both
passes are approved."). **They are not re-asked here.**

What this session tests is the surface that changed *after* those checks: the live-use fix stream
that landed on Phase 14's own controls between 2026-09-10 and 2026-09-16, plus the two cold-start
risks the v0.4 milestone audit raised. Commits in scope:

| Commit | What it changed on a Phase 14 surface |
|---|---|
| `174957f` | APPLY moved into a bottom panel so a long rule list can't push it off-screen |
| `1c85e32` | League pinned; footer unstacked; stale messages cleared on hide; `+ ADD RULE` moved inside the scroll area |
| `4623cf6` | Stripe docks inside the work area (not under the taskbar); hotkey gated to the game window |
| `6ba59af` | `Jewels` added to the JEWELLERY class-picker group |
| `10ffe65` | `seed_prices` + `currency.json` retired (no user-facing surface; included for completeness) |

Automated UI verification does not apply: `ff` is a native egui window, not a browser, so no
Playwright/Puppeteer path exists. Every check below needs a human at a running game.

**Before starting:** `cargo build --release --bin ff` (already done, 2026-09-17), and PoE2 running
in **windowed** or **windowed-fullscreen** — exclusive fullscreen shows no overlay and is not
detected (`PITFALLS.md` §2).

## Current Test

[testing complete]

## Tests

### 1. Cold start from a shortcut
expected: Launched by double-clicking `target\release\ff.exe` (not from the repo root), the stripe opens and lists the rules normally.
result: issue
reported: "Opening from the exmplorer with a double click on the ff.exe on release dir gave me an error, when the ff window opened it showed: \"Cannot read `C:\Users\jonat\Repositories\poe2_myfilter\target\release\dynamic_base.json`: The system cannot find the file specified. (os error 2)\""
severity: blocker
note: >-
  Confirms the milestone audit's WARNING-4 exactly, and the reported path is the proof: the store
  resolved `dynamic_base.json` against the EXE's directory (`target\release\`), which is the
  process CWD when launched from Explorer. One thing that went right -- the failure is a readable
  diagnostic naming the exact path, not a crash or a silent blank stripe.

### 2. APPLY row clear of the taskbar
expected: With the stripe open, the whole footer is visible above the Windows taskbar -- the status line, REVERT, SYNC ECONOMY and APPLY are all fully readable and clickable, with a small gap below APPLY rather than the row being clipped at the screen edge. (Fix for screenshot poe2_print008; the stripe now asks Windows for the work area instead of assuming the full 1080p height.)
result: pass

### 3. Footer no longer collides with + ADD RULE
expected: Scroll the rule list to the very bottom. `+ ADD RULE` sits at the end of the list and scrolls with it; no text overlaps the footer, including right after pressing SYNC ECONOMY when the economy message is showing. (Fix for screenshot poe2_print007.)
result: pass

### 4. Hotkey only fires in the game
expected: With PoE2 focused, Ctrl+Shift+Z summons and hides the stripe as before. Then alt-tab to a browser or editor and press Ctrl+Shift+Z -- nothing should happen, the stripe stays as it was. Focus the stripe itself and press it -- it should still dismiss.
result: pass

### 5. SYNC ECONOMY names the right league
expected: Press SYNC ECONOMY. The stripe stays responsive (the sync runs on a worker thread), and the success line names **Forbidden Rites** -- not Runes of Aldur. Your `prices.json` currently holds Runes of Aldur data, so this is the first real test of the pinned league.
result: pass
note: >-
  Author confirmed 2026-09-17: verified in an earlier session. prices.json holds league "Forbidden Rites", 1357 items, fetched 2026-09-16T03:24:24Z -- contents corroborate the pin. The file was not rewritten during this UAT round (mtime unchanged), consistent with the button having been exercised earlier rather than re-pressed here.

### 6. Synced prices reach the filter on the next Apply
expected: After test 5 succeeds, press APPLY, then reload in-game (Options -> Game -> Item Filter -> Refresh). Currency and unique highlighting reflect the freshly synced economy. Nothing should have changed in the game folder between the sync and the Apply -- that separation is the point of the button.
result: pass

### 7. Messages clear after hiding the stripe
expected: With a "Wrote ..." message and/or an economy message on screen, hide the stripe (Ctrl+Shift+Z or the stripe's own X) and summon it again. Both messages are gone -- a fresh stripe, not last session's status text.
result: pass

### 8. Jewels in the class picker
expected: Expand any rule, switch on "match classes", open the **JEWELLERY** group. It lists Rings, Amulets, Belts and **Jewels**. Ticking Jewels and pressing APPLY emits a `Class` line containing "Jewels" in `dynamic.filter`. (Requires the ff restart from test 1 -- the running instance loaded `dynamic.json` before the field was added.)
result: pass

## Summary

total: 8
passed: 7
issues: 1
pending: 0
skipped: 0
blocked: 0

## Gaps

<!-- Pre-seeded from the v0.4 milestone audit: verified by code inspection, not by user report.
     Not presented as a test because reproducing it requires forcing a disk-write failure. -->

- truth: "A failed store save must not report Apply as successful, and must not advance the Revert baseline."
  status: failed
  reason: "Found by code inspection during the v0.4 milestone audit (BLOCKER-1), not by user report. src/overlay/app.rs:493-497 records a store::save error into last_save_error and falls through with no early return; compile_store then reads the stale dynamic.json, write_filter succeeds, and the Written branch (app.rs:525-531) sets the success message, flips apply_state to Applied, advances self.baseline to never-persisted edits, and clears self.dirty (removing REVERT). The red save-error line and the green success line render simultaneously."
  severity: blocker
  test: 0
  root_cause: "Missing early return on the store::save error branch in the APPLY handler (src/overlay/app.rs:493-497). The documented save-then-compile contract five lines above already implies it."
  artifacts:
    - path: "src/overlay/app.rs"
      issue: "APPLY handler: store::save failure is recorded but not acted on; execution continues into compile_store"
  missing:
    - "Treat a store::save failure as ApplyOutcome::Failed and return before compiling"
    - "A test that forces a save failure and asserts apply_state is not Applied and baseline is unchanged"
  debug_session: ""

- truth: "Launching ff the way a player would -- double-clicking the built exe -- opens a working stripe."
  status: failed
  reason: "User reported: Opening from the exmplorer with a double click on the ff.exe on release dir gave me an error, when the ff window opened it showed: \"Cannot read `C:\Users\jonat\Repositories\poe2_myfilter\target\release\dynamic_base.json`: The system cannot find the file specified. (os error 2)\""
  severity: blocker
  test: 1
  root_cause: >-
    src/overlay/store.rs:81-84 (`default_paths`) resolves the base and store paths against
    `std::env::current_dir()`. `ff` is a Windows GUI-subsystem binary, so launched from Explorer
    or a shortcut without "Start in", the CWD is the exe's own directory -- `target\release\` --
    which contains no `dynamic_base.json`, `tiers.json` or `mapping.json`. Phase 11's CORE-02 made
    the library path-agnostic precisely so a front end could resolve its own root; the overlay
    front end never took up that affordance and inherited the CLI's working-directory convention,
    which is correct for a console tool and wrong for a double-clickable GUI.
    Compounded by WARNING-1: nothing in src/overlay/ calls `dynamic::reset_store`, so there is no
    in-GUI recovery from a store that will not load.
  artifacts:
    - path: "src/overlay/store.rs"
      issue: "default_paths() derives every overlay path from the process working directory"
    - path: "src/overlay/app.rs"
      issue: "store-load failure renders an error in place of the rule list; APPLY stays clickable and there is no reset/locate action"
  missing:
    - "Resolve the store root from a stable anchor rather than the CWD -- exe-relative lookup, walking up for dynamic_base.json, or a remembered location"
    - "An in-GUI recovery path when the store will not load (locate folder, or reset from template)"
    - "A test that resolves paths with the CWD set somewhere without the template, asserting the resolver still finds the repo root"
  debug_session: ""
