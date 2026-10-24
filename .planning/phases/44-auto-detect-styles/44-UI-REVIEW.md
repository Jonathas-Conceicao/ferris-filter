# Phase 44 - UI Review

**Audited:** 2026-10-23
**Baseline:** 44-UI-SPEC.md (approved)
**Screenshots:** partial. Debug screenshots `config-custom.png` and `config-status.png` were read. Both cut the Custom heading off below the fold, so the button itself was never seen rendered. Layout is judged from the code and the two layout tests.

---

## Pillar Scores

| Pillar | Score | Key Finding |
|--------|-------|-------------|
| 1. Copywriting | 3/4 | Status copy matches the spec but is truncated at the 660pt window width |
| 2. Visuals | 3/4 | Button reuses the skin and is placed correctly, but no screenshot shows it |
| 3. Color | 4/4 | No new colour, no accent; `StatusKind::Ok` / `Info` as specified |
| 4. Typography | 4/4 | `ff_button_ex` and `ff_status` only; nothing new |
| 5. Spacing | 3/4 | Width is 139.5pt, not the contracted 112pt; the row has not been confirmed visually |
| 6. Experience Design | 3/4 | Idempotent and revertible, but new rows are not surfaced and a damaged-markers no-op is silent |

**Overall: 20/24**

---

## Top 3 Priority Fixes

1. **The success status is cut off in the bottom bar** (WARNING) - `config-status.png` shows `Auto Detect: 3 styles, 41 rules tagg...`. The user cannot see "tagged" without hovering, and counts such as `Auto Detect: 12 styles, 130 rules tagged` get cut earlier. Shorten the copy (for example `Auto Detect: 3 styles, 41 tagged`) or make the bottom-bar status slot wider. The spec's own example string does not fit next to Revert / Reload / Apply.
2. **The result is invisible from where the user clicked** (WARNING) - The Custom heading sits below the fold in the default Config view, and the new `Custom Style N` rows are collapsed with no scroll-to and no highlight (spec judgement call 6). A user who clicks the button sees only a truncated status line and no sign of what changed. Scroll the Custom heading into view after a click that adds styles, or add the new-style count to the Custom heading. This is a small change that keeps the "not texty" rule.
3. **The button was never seen rendered** (WARNING) - The Custom heading is below the fold in both screenshots, and the summary has deferred the check. Take a screenshot with the Custom heading in view (the existing `--expand-style` or scroll flags may help) and confirm that `[Auto Detect 139.5pt] [+ Add style 124pt]` fits on one line at 660pt, with the 8pt gap and the right edge aligned to the Order column.

Minor:
- Button width is 139.5pt, against 112pt in the spec. The 112 passed to `ff_button_ex` is only a minimum. Either amend the spec to the real width, or shorten the label. Together with Add style, the row takes 271.5pt plus the heading. That fits, but it is not the spec's number.
- The handler runs `clear_events_on_edit()` before checking the damaged-markers guard (`app.rs:3062-3064`). If the button is somehow clicked in that state, the previous status message is wiped and nothing replaces it. The button is disabled in that state, so this is unreachable from the UI today and only matters for robustness.
- The disabled tooltip has a third variant, `Economy markers damaged.`, which the spec does not list (it lists only `No filter loaded.`). It is sensible and matches the existing pattern, so it is a spec amendment rather than a defect.

---

## Detailed Findings

### Pillar 1: Copywriting (3/4)
- Button label `Auto Detect` and the enabled tooltip match the spec verbatim (`config_view.rs:860,866`). `No filter loaded.` matches.
- `status_text` and `NOTHING_NEW` match the contract, including singular forms (`style_detect.rs:93-97`, tests at lines 303-306).
- `config-status.png`: the one-line status renders as `Auto Detect: 3 styles, 41 rules tagg...`. The copy is correct but unreadable in full at this width; the full text appears on hover only.
- `Auto Detect: 0 styles, 12 rules tagged` reads slightly oddly, but it is specified.

### Pillar 2: Visuals (3/4)
- The button uses the same widget and the same heading row as + Add style, so the focal hierarchy is unchanged. It is a secondary stone button next to the primary one, as the spec intends.
- The button is left of + Add style inside the `right_to_left` layout, added after it (`config_view.rs:849-877`), which is the specified order.
- A `auto-detect` probe is registered, and layout tests cover placement and the disabled state.
- No visual evidence of the button exists (see priority fix 3). This is why the score is not 4.

### Pillar 3: Color (4/4)
- No new palette constants, no hardcoded colours, no accent. Status uses `StatusKind::Ok` for a change and `Info` for a no-op, matching spec judgement call 5.

### Pillar 4: Typography (4/4)
- Nothing new. The Fontin caption comes from `ff_button_ex` and the status font from `ff_status`.

### Pillar 5: Spacing (3/4)
- The 8pt gap comes from the Ui's item spacing, which is the contract's value (the test asserts it).
- Width deviation: the label needs 139.5pt, against the spec's 112pt. The row still fits in the ~660pt window, but the contract number is wrong in both the spec and the implementation.
- The row is not confirmed visually.

### Pillar 6: Experience Design (3/4)
Covered:
- Disabled with a tooltip when no filter is loaded or the markers are damaged.
- Not blocked by Fallback, drafts or pending edits, as specified.
- One batch of pending edits, nothing written to disk.
- A second click is a no-op with an Info status and no unapplied state.
- Revert Rules, Revert Config, bottom-bar Reload, the Quit confirm and Apply (ids and group-wide tags written, a second Apply a no-op) all have tests (`app.rs:6743-6869`).
- No destructive action, so no confirmation is needed.

Gaps:
- New rows are collapsed and not highlighted, and the heading is below the fold (priority fix 2).
- After a Config-only Revert, tags remain as amber `unknown style` flags. This is accepted in the spec, but it is a confusing result for a user and is worth a manual UAT look.
- There is no loading or error state, which is fine because the action is in-memory only.

---

## Files Audited
- `.planning/phases/44-auto-detect-styles/44-UI-SPEC.md`, `44-03-SUMMARY.md`
- `src/overlay/config_view.rs` (lines 825-895, 2099-2145)
- `src/overlay/app.rs` (lines 3061-3089, 3319, 6675-6870)
- `src/overlay/style_detect.rs` (status copy)
- `C:/Users/jonat/AppData/Local/Temp/ff44/config-custom.png`, `config-status.png`
