# Phase 19 — UI Review

**Audited:** 2026-10-12
**Baseline:** 19-UI-SPEC.md (approved design contract)
**Screenshots:** not captured. This is a native egui overlay with no dev server, and launching it would touch the real game folder. Findings are code-only. Anything that needs real pixels is marked `needs_human_review`, and 19-HUMAN-UAT.md tests 1-3 are still pending.

---

## Pillar Scores

| Pillar | Score | Key Finding |
|--------|-------|-------------|
| 1. Copywriting | 3/4 | Contract strings are almost all verbatim. Gaps are an invented `grammar warning` flag and a status-line order that differs from the spec. |
| 2. Visuals | 3/4 | Row anatomy and virtualization are correct. Flags can starve the label, and the NAME toggle sits alone above its field. |
| 3. Color | 3/4 | Accent use is mostly on the reserved list. `WARM_ACCENT` at 13px is about 4.4:1 and so under the 4.5:1 AA threshold. |
| 4. Typography | 3/4 | No new sizes. Cinzel `row_label` is used for body-role text, and `.small()` 11px is used on the `Remove`, `Duplicate` and `Delete` buttons. |
| 5. Spacing | 3/4 | Mostly 4/8/10/16. Several `add_space(6.0)` calls are off the declared scale. |
| 6. Experience Design | 3/4 | All states are covered and mutations are deferred. Weaknesses are a footer that can grow without bound, a small click target for expand, and a stale delete-confirm. |

**Overall: 18/24**

No BLOCKERs found by code audit. All findings are WARNING.

---

## Top Priority Fixes

1. **Flags can squeeze the row label to nothing** (rule_list.rs:263-283) — A rule with `Continue` + `style wins` + `moved across Continue` uses about 200px of the 460px stripe. After the 16px arrows and the toggle, little is left for the label. The label is the only thing that identifies a rule. Fix: give the label a minimum width (about 140px) and let flags truncate or collapse to short glyph badges once space runs out. Cost: medium.
2. **Footer panel has no height cap and stacks up to 6 message lines** (app.rs:613-779) — This stack is the economy footer, indicator, buttons, sync status, apply message, list status and the grammar-gate line. `CHANGED_ON_DISK_MESSAGE` is about 190 chars and wraps to 3-4 lines. At small window heights the list is squeezed to a few rows. Fix: put the messages in a capped `ScrollArea`, or merge `list_status` and `apply_message` into a single "last outcome" slot. Cost: medium.
3. **Only the label text is the expand hit target** (rule_list.rs:302-314) — `Label.sense(click)` covers the text extent only. Short labels such as `3  Show · matches everything` leave dead space, and a disabled row dims further. Fix: allocate the whole left-rect response with `Sense::click()` and keep the label as paint. Cost: cheap.
4. **WARM_ACCENT flags at 13px on near-black fall below AA** (rule_list.rs:97,119,133; theme.rs:48) — #A36C3C on #0D0D0D is about 4.4:1, and the destructive confirm text is the same colour. The spec reserves this colour for exactly these error cues, so legibility matters most here. Fix: lighten the accent for text use only (about #B87A45), or bold or underline the flag. Each flag already has a text twin, so monochrome is fine. Cost: cheap.
5. **NAME toggle is orphaned above its field** (rule_list.rs:344-368) — The toggle is a bare 16px circle on its own line with no tooltip, and the text field sits below it. The spec calls for toggle + field, as in v0.4. Fix: wrap both in `ui.horizontal`, and add a tooltip (`Name this rule`). Cost: cheap.
6. **Delete-confirm persists across collapse** (rule_list.rs:449,466 and app.rs:220) — `confirm_delete` is keyed by id and not cleared when the row collapses or another row expands. Re-opening that row later shows `Delete this rule?` already armed. Fix: clear `confirm_delete` when `expanded` changes (rule_list.rs:312). Cost: cheap.

---

## Detailed Findings

### Pillar 1: Copywriting (3/4)

Verbatim matches to the contract:
- Search hint (rule_list.rs:479).
- Empty and no-match headings and bodies (app.rs:850-858).
- Delete and economy-remove confirm copy (rule_list.rs:450-458, 529-539, 559).
- Economy footer strings with correct singular/plural (rule_list.rs:503-506).
- `Reverted to the last applied filter.` (apply.rs:124).
- The Apply-blocked tooltip (apply.rs:127).
- The `Style "S" decides…` note (rule_list.rs:414).
- `moved across Continue` and its tooltip (rule_list.rs:132-135).
- Derived labels (`Rings +2 more`, `BaseType ×N`, `matches everything`; rules.rs:211-236).

Deviations:
- WARNING: an extra `grammar warning` flag (rule_list.rs:123-129) is not in the spec's closed flag set (Continue, style wins, grammar error, can't enable). It is plausible, but the spec said "only those that apply" and fixed the order. Update the spec or remove the flag.
- WARNING: the spec puts the status block (root, path, filter state) above the `FILTER` section header. The code draws `FILTER` first, then the status lines, then the search box (app.rs:787-844). The status text now sits between the heading and its own search box.
- WARNING: the `CONDITIONS` body still says `Editing conditions arrives in a later update.` That is per spec, but it is user-visible placeholder copy and should be tracked for Phase 20.
- Minor: the button order is `+ ADD RULE`, `RELOAD`, `SYNC ECONOMY`, `REVERT`, `APPLY` (app.rs:646-702). The spec lists `+ ADD RULE`, `REVERT`, `APPLY`, then the others. Because Apply is last it reads fine, but the sync-before-revert order was not decided in the spec.
- Minor: `Remove` and `Add economy section` are sentence-case in-row actions (correct per the casing rule).

### Pillar 2: Visuals (3/4)

- Passes: the row anatomy `[toggle] [label] [flags] [up][down]` is implemented as specified (rule_list.rs:234-294). Zebra fills alternate by visible index. Arrows are painted shapes, not glyphs, which avoids the Cinzel tofu problem. The accordion body is a PANEL_GROUND frame, so it reads as nested. Virtualization draws only the viewport range, and there is a test for 511 rules.
- WARNING: the label is squeezed. `left_rect` is whatever width remains after the flags (rule_list.rs:273-278), and there is no minimum. See Fix 1. `needs_human_review`: check a rule with 3 flags on a 460px stripe.
- WARNING: the expand target is only the label's text extent (rule_list.rs:302-314). See Fix 3.
- WARNING: the NAME toggle is stacked above the field with no label or tooltip (rule_list.rs:346-368). See Fix 5.
- WARNING: toggle and arrows are 16x16 (widgets.rs:115,159) inside a row about 24px high. That is small for a mouse target in a game overlay and has no accessible name; the painted controls report no `widget_info`.
- Minor: the expanded row header colours the label SECTION_CREAM, but nothing else (such as a left bar) distinguishes the open row from its neighbours. Use a stroke or accent bar if it reads weakly.
- Minor: disabled arrows are painted DISABLED_TEXT, but the disabled ACTION segment keeps HEADER_PLATE fill for the selected option (rule_list.rs:388-392). That is intended, but the only difference between a locked selected and a locked unselected option is the fill.

### Pillar 3: Color (3/4)

- Tokens come only from `theme.rs`. A grep of the audited files finds no hex literals or `Color32::from_rgb` outside theme.rs. Fills use the 60/30/10 split of PANEL_GROUND and ROW_BASE/ALT, then HEADER_PLATE and tan text, with the accent kept small.
- `WARM_ACCENT` is used in these places:
  - flags: can't enable, grammar error, moved across Continue (rule_list.rs:97,119,133)
  - delete and remove confirms (rule_list.rs:452,533)
  - `CANNOT_ENABLE_LONG` and error lines in the body (rule_list.rs:434,440)
  - economy damaged (rule_list.rs:579)
  - apply failures and the grammar-gate line (app.rs:724,744,767)
  - root error and damaged state (app.rs:808,817)
  All of these map to the spec's four reserved categories.
- WARNING: contrast. #A36C3C against #0D0D0D works out to about 4.4:1, which is under AA for 13px text. See Fix 4.
- WARNING: `CANNOT_ENABLE_LONG` is painted in the accent in the expanded body (rule_list.rs:434). The spec lists it as informational guidance. It is also an error cue, so accent is defensible, but it is a gap between the spec and the code (the spec's accent list item 2 names the flag text only).
- Minor: the `grammar warning` flag uses LABEL_TAN, the same colour as `Continue` and `style wins`, so a warning is not visually distinct from an informational flag. Its text twin is the only cue.

### Pillar 4: Typography (3/4)

Sizes in use: 11 (`.small()` captions and small buttons), 13 (ROW_SUMMARY_SIZE), 14 (egui body and `row_label`), 18 (Cinzel `section_header`). That is within the spec's 11/13/14/17-18. Weights: none are set, so it is Regular plus Cinzel.

- WARNING: the stale indicator uses `theme::row_label` (Cinzel 14, letter-spaced) (app.rs:641). The spec's Body role is egui default 14 Regular, and Cinzel is reserved for the header plate and section headers. This is inherited from v0.4, but the spec is explicit.
- WARNING: flags and the NAME/ACTION/CONDITIONS captions are all small text. In particular the all-caps `.small()` captions at 11px LABEL_TAN are the smallest text in the stripe.
- Minor: `section_header` is 18px and the spec says 17-18px. This is fine.
- Minor: centered_notice renders the empty-state heading in Cinzel 18 (app.rs:263). A long no-match query (`No rules match "<query>"`) is a single non-wrapping job and could overflow the 460px stripe. Cinzel is all caps, so the heading is wider than expected. `needs_human_review`.

### Pillar 5: Spacing (3/4)

- Declared scale: 4/8/16. Code constants: `add_space(4.0)`, `8.0`, `16.0` (app.rs:261), `10.0` (inherited, allowed by the spec exception), frame inner margins `Margin::symmetric(10, 8)` and `Margin::same(8)` (rule_list.rs:319,521), arrow/toggle 16px, row height `interact_size.y + 4` (rule_list.rs:75), and `item_spacing.y = 0` inside the list.
- WARNING: off-scale `add_space(6.0)` at app.rs:264, 802, 845 and rule_list.rs:294. Use 4 or 8.
- WARNING: the expanded body's inner margin of 10 px horizontal (rule_list.rs:319) matches the inherited 10px inset, but the collapsed row's inner shrink is 8px (rule_list.rs:232). The toggle's left edge therefore does not line up with the body's left edge (8 vs 10).
- Minor: the md token (16px) between the list and the footer is not used. The only gap is the 10px top margin of the footer frame, which is 0 (app.rs:619-624), so the list's last row sits directly against the economy band. `needs_human_review`.
- Minor: the flag separator is `item_spacing.x = 4.0` (rule_list.rs:263), as specified.

### Pillar 6: Experience Design (3/4)

State coverage:
- Empty list, no search matches, load error with recovery text, and the damaged/absent/present economy states are all handled (app.rs:847-880, rule_list.rs:565-581).
- Apply: busy/disabled by grammar gate with an on-disabled tooltip (app.rs:693-699); changed-on-disk forces RELOAD; the clickable gate line jumps to the offending rule and clears the query if the rule is hidden (app.rs:757-778, 887-893).
- Revert is enabled only when dirty; Add is staged; search disables reorder with a tooltip (rule_list.rs:250-253).
- All mutations are `RowAction`s applied after the loop. Rule identity is by `BlockId`, so state survives reorders and filtering, and `clear_ids` resets it on Revert, Apply and RELOAD.
- There is a headless test for virtualization at 511 rules.

Issues:
- WARNING: the footer has no height cap (app.rs:613-779); see Fix 2.
- WARNING: `confirm_delete` and `confirm_economy_remove` survive collapse or context change; see Fix 6. `confirm_economy_remove` also stays armed if the Present state changes underneath (for example after RELOAD, which does clear it, but not after staging Add elsewhere).
- WARNING: while a search is active the up/down arrows get the "Clear the search to reorder" tooltip on every row, including the first and last rows, whose arrows would be disabled anyway. The message is slightly misleading there (rule_list.rs:250-253).
- WARNING: the first-run bug risk: `list.expanded_h` starts at 180 and corrects by a repaint request (rule_list.rs:325-328). The first frame after expanding therefore uses a stale offset, so there may be a one-frame jump when scrolling to an expanded row near the bottom. `needs_human_review`.
- Minor: Esc is handled only when the search box has focus or just lost it (rule_list.rs:485-491), which matches the spec.
- Minor: the toggle tooltips (`Enabled` / `Disabled`) say the current state, not the action. This is per the spec, but it is ambiguous on hover.
- Registry audit: skipped (shadcn not initialized).

---

## Files Audited
- src/overlay/rule_list.rs
- src/overlay/app.rs (lines 125-280, 575-900, plus grep of the remainder)
- src/overlay/widgets.rs (circular_toggle, arrow_button)
- src/overlay/theme.rs
- src/overlay/rules.rs (derived labels, lines 195-260)
- src/overlay/apply.rs (message constants via grep)
- src/overlay/layout.rs (constants via grep)
- .planning/phases/19-overlay-rule-list-over-parsed-blocks/19-UI-SPEC.md
- .planning/phases/19-overlay-rule-list-over-parsed-blocks/19-HUMAN-UAT.md

## Post-audit fixes

Commit 96da66b fixed top issues #3 (whole-row click to expand), #5 (inline NAME toggle with tooltip) and #6 (stale delete confirm disarmed when the expanded row changes). Issues #1, #2, #4 and #7-#9 remain open for a later pass (scores above are pre-fix).
