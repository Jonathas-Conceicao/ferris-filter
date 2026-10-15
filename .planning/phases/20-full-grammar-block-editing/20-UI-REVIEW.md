---
phase: 20
slug: full-grammar-block-editing
audited: 2026-10-13
baseline: 20-UI-SPEC.md
screenshots: not captured (native egui overlay, no dev server)
overall: 18/24
---

# Phase 20 — UI Review

**Audited:** 2026-10-13
**Baseline:** 20-UI-SPEC.md (design contract)
**Screenshots:** not captured. Code-only static audit of `src/overlay/{body,rule_list,widgets,theme,app}.rs`. Nothing was built or run.

---

## Pillar Scores

| Pillar | Score | Key Finding |
|--------|-------|-------------|
| 1. Copywriting | 3/4 | Contract strings mostly verbatim. Deviations: status-block order, Duplicate/Delete button size. |
| 2. Visuals | 3/4 | Shared row anatomy, flag budget, 24px X/swatch targets present. Control heights are mixed. |
| 3. Color | 3/4 | `WARM_ACCENT_TEXT` applied; no raw `WARM_ACCENT` text use. Contrast figure needs a pixel check. |
| 4. Typography | 2/4 | 11px buttons remain although the spec reserves 11px for captions. |
| 5. Spacing | 2/4 | `add_space` is clean, but off-scale margins remain in widgets.rs. |
| 6. Experience Design | 3/4 | Capped outcome area, locked style rows, `+ action` hidden when styled. Body controls below 24px minimum. |

**Overall: 18/24.** No BLOCKERs that break task completion. All findings are WARNING.

---

## Phase 19 carry-over status

| Item | Status | Evidence |
|------|--------|----------|
| Row labels squeezed by flags/arrows | Fixed | `LABEL_MIN_W=140`, `FLAG_BUDGET=120`, `+N` overflow badge via `fit_flags` (rule_list.rs:135-143, 381) |
| Footer height cap | Fixed | `OUTCOME_MAX_HEIGHT=96`, single scroll area (app.rs:610-654, 1003-1010); tests assert cap and APPLY on screen |
| Error-red contrast at 13px | Fixed in code | `WARM_ACCENT_TEXT` #BF8548 (theme.rs:49) used for text (app.rs:1042, 1051) |
| Status lines below FILTER heading | NOT fixed | app.rs:1023 draws `FILTER`, then status block at 1038-1071, then search bar. Spec wants status above the heading. |
| Off-scale spacing | Partly fixed | `add_space` only 4/8/16. Remaining: `Margin::symmetric(10, 6)` (widgets.rs:97), `left: 6` and `item_spacing.x = 2.0` (widgets.rs:~536-543), badge `symmetric(4, 1)` (widgets.rs:308) |

---

## Top Priority Fixes

1. **Status block sits between FILTER heading and its search box** (app.rs:1021-1071). Heading and its search are separated by status text. Reorder two blocks. Cheap (a few lines).
2. **Duplicate and Delete use `ui.small_button` (11px)** (rule_list.rs:582, 585); economy `Remove` / `Add economy section` (rule_list.rs:669, 690) use `.small()` too, and the socket `Remove` (body.rs:1351) may. Spec: small buttons are 13px, 11px captions only. Replace with `Button::new(RichText::new(..).size(ROW_SUMMARY_SIZE))` as `+ condition` / `+ action` already do (body.rs:294, 1625). Cheap.
3. **Body interactive height not raised to 24px.** No `interact_size.y = 24` override found. ComboBox, DragValue, Slider and checkbox stay near 18px while custom X, toggle and swatch are 24px, so rows are uneven and below the spec minimum. Set `ui.spacing_mut().interact_size.y = 24.0` at the top of the body scope. Cheap, but check collapsed-row virtualization height is unaffected.
4. **Leftover off-scale margins** in widgets.rs (10/6, 6, 2). Snap to 8/4 and 4. Cheap (3-4 constants); shifts header plate and zebra rows by a pixel or two, so check visually.
5. **Needs human review:** whether the STYLE banner reads as the focal point, the real contrast of `WARM_ACCENT_TEXT` at 13px, and accessibility-name exposure at runtime. Painted widgets do call `widget_info` (widgets.rs:122, 180, 232, 300). Belongs in HUMAN-UAT, not a code fix.

---

## Detailed Findings

### Pillar 1: Copywriting (3/4)
- Unknown-style, from-style, `Remove style`, `Hide icon` and `(PoE 1)` strings are present. Delete and Duplicate keep Phase 19 copy.
- WARNING: status-block order deviates from the spec (Fix 1).

### Pillar 2: Visuals (3/4)
- Rows use the shared card anatomy; `LABEL_W=120` and `OP_W=104` match the spec.
- `+ action` is hidden when the rule has a known style (body.rs:1621-1630), as required.
- Picker popups capped at 320px (body.rs:440); chip lists and class picker capped at 160px.
- WARNING: mixed control heights (Fix 3).

### Pillar 3: Color (3/4)
- No hardcoded hex in the audited overlay files; palette comes from theme.rs.
- Raw `WARM_ACCENT` is not used for text; `WARM_ACCENT_TEXT` is used for the root error and damaged state.
- `needs_human_review`: the claimed 5.6:1 at 13px was not measured.

### Pillar 4: Typography (2/4)
- Sizes in use: 11 (captions via `.small()`, body.rs:157), 13, 14, 18. No new sizes. `row_label` has no remaining uses in the audited files; the stale indicator moved to body text.
- WARNING: 11px buttons violate the "11px for captions only" rule (Fix 2).

### Pillar 5: Spacing (2/4)
- `add_space` values: only 4.0, 8.0, 16.0 in body.rs, app.rs, rule_list.rs.
- WARNING: off-scale margins and spacing remain in widgets.rs (Fix 4).
- WARNING: the spec says off-scale values are retired in touched code, and this was only partly done.

### Pillar 6: Experience Design (3/4)
- Outcome lines are click-to-jump. Adding a row calls `scroll_to_me` and `request_focus` (body.rs:757-759, 1759-1761). Esc handling and deferred `RowAction`s are retained.
- WARNING: body controls below the 24px minimum (Fix 3).
- Registry audit: skipped (shadcn not initialized).

---

## Files Audited
- src/overlay/app.rs
- src/overlay/rule_list.rs
- src/overlay/body.rs
- src/overlay/widgets.rs
- src/overlay/theme.rs
- .planning/phases/20-full-grammar-block-editing/20-UI-SPEC.md
- .planning/phases/19-overlay-rule-list-over-parsed-blocks/19-UI-REVIEW.md

## Post-audit fixes

Applied after this audit (commit "fix(20): UI review quick wins"):

- Fix 1 (status order): status block moved above the `FILTER` heading. Done.
- Fix 2 (11px buttons): Duplicate, Delete, Keep, Clear, Remove and Add economy section now render at 13px (`ROW_SUMMARY_SIZE`). Done.
- Fix 4 (off-scale margins): header plate 10/6 -> 8/4, chip left 6 -> 4, chip item spacing 2 -> 4. Badge `symmetric(4, 1)` left as is. Done (needs a visual glance in UAT).
- Fix 3 (24px interact height in the body): deferred. `row_height` and the virtualized list's `expanded_h` cache derive from `interact_size.y`, so raising it is not a contained change; left for a later polish pass.
- Fix 5 (focal point / accessible names): runtime-only, covered by 20-HUMAN-UAT.md.
