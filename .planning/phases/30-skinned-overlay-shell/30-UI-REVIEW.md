# Phase 30 — UI Review

**Audited:** 2026-10-16
**Baseline:** 30-UI-SPEC.md (approved). The interim mixed look of the legacy RULES/CONFIG widgets is accepted by the author until Phases 31/33, so it is not scored as a defect.
**Screenshots:** Not captured live. This is a Rust egui app, so there is no dev server or Playwright. The 8 committed shots from the debug `--screenshot` mode were viewed: `rules`, `config`, `economy`, `about`, `popup-quit`, `popup-changed`, `popup-delete-rule` and `status-truncated`. Hover tooltips, keyboard behaviour, DPI and multi-monitor docking are not visible in stills. They are marked needs_human_review and covered by 30-HUMAN-UAT.md.

---

## Pillar Scores

| Pillar | Score | Key Finding |
|--------|-------|-------------|
| 1. Copywriting | 3/4 | Mixed-case copy and heading-only empty states match the contract. The grammar-gate status is cut before it names the rule, and ABOUT shows a stale-looking version. |
| 2. Visuals | 3/4 | The chrome is a strong focal hierarchy and the icons read. The flag icons are small, and "A CONTINUE" reads as one phrase. |
| 3. Color | 3/4 | The palette is faithful and the accent is restrained. Some raw white is hardcoded, and the unremapped legacy grey widgets break the stone look. |
| 4. Typography | 3/4 | Fontin SmallCaps/Regular are used consistently on the 14/15/17/21 scale. The Config Name column is mis-aligned, and the About disclaimer orphans a word. |
| 5. Spacing | 2/4 | The search field and "+ Add rule" touch with no gap, and the contract's 8pt search-row gap is not met. Column alignment and the status budget are also off. |
| 6. Experience Design | 3/4 | Seven modals, a single status line with a priority order, and disabled-state reasons are covered. A truncated status hides the actionable part, and the Overwrite slot is not proven in the shot. |

**Overall: 17/24**

---

## Top 3 Priority Fixes

1. **The status line is cut before the useful part of the message (WARNING).**
   - In `rules.png`, `config.png`, `economy.png` and `about.png` the error reads `Apply refused: 1 enabled rule has a…`, so the rule number is lost.
   - Roughly 250pt of the roughly 600pt bar is left for text because the three buttons take over 300pt. They render about 95pt wide against the spec's 84 minimum.
   - The user has to hover a tooltip to learn what to fix, and the click-to-jump target is invisible.
   - Fix: shorten the standing strings (for example `Rule 2 has a grammar error` plus the jump), or let the status wrap to two lines inside the 34pt bar. Alternatively, size the buttons to their minimum of 84 and make `ff_status` prefer the tail ("first: rule N") over the head when it truncates.
2. **The search row has no gap between the field and "+ Add rule" (WARNING).**
   - In `rules.png` the field's right edge at about x=467 abuts the button's left edge.
   - UI-SPEC specifies field width = available - 132 with a 124pt button, which leaves an 8pt gap.
   - The result looks like one fused control.
   - Fix: re-check the `shell.rs` / `rule_list.rs` search-row width arithmetic against the spacing scale (sm = 8) and apply item spacing.
3. **Flag-icon legibility and crowding (WARNING).**
   - In row 7 of `rules.png` the styled flag reads `◆ A CONTINUE`. The style name `A` and the gold `CONTINUE` word share the same gold and muted weight and sit about 4pt apart, so they read as a phrase.
   - The broken-link icon (rule 6) looks like two joined rings (`oo`), and the padlock is tiny. Both are acknowledged as small in 30-05-SUMMARY.
   - Fix: add an 8pt gap before the Continue word, or put the styled chip left of the label as a pill. Redraw the broken-link icon with a visible gap and slash. Raise the icon stroke or size to 16pt and keep the 18pt hit rect.
4. **The Config Name column is mis-aligned (WARNING).** The "Name" header sits left-aligned at x=154 but the style names (S, A, B…) are centred around x=205, and the Visibility/Preview columns are left-aligned. Left-align the names, or centre the header. Also check the custom "Recycle" text field against the same column.
5. **Popup width is 580pt, not 520pt (WARNING).** `popup.rs:60` deviates from the contract. It was a logged choice in 30-03 so the Quit row fits. Update UI-SPEC to 580, or shorten the captions. Separately, the Changed-on-disk body is one run-on paragraph rather than two sentences on two lines.
6. **ABOUT version and disclaimer.**
   - `Version 0.5.0` is shown on the v0.6 milestone branch. Confirm Cargo.toml is meant to say 0.5.0.
   - The two-line disclaimer wraps to four lines and leaves "way." orphaned. Widen the text rect or hyphenate the copy.

---

## Detailed Findings

### Pillar 1: Copywriting (3/4)

**Matches the contract**
- Button captions are mixed-case source strings rendered in small caps: Revert, Reload, Apply, Sync economy, + Add rule, Add economy section.
- Search hint `Search 89 rules, classes, base types…` carries the rule count as specified.
- Popup titles and buttons match the inventory: "Unapplied changes" with Apply & quit / Quit without applying / Cancel, "File changed on disk" with Reload / Overwrite / Keep editing, and "Delete this rule?" with Delete / Keep. Each ends in a safe dismiss.
- The delete body `Rule 1: Chaos orbs shout` matches the contract.
- ABOUT copy matches exactly, including the credits.

**Deviations**
- WARNING: the status copy `Apply refused: 1 enabled rule has a…` is cut at the point where the actionable part ("first: rule N, line M") begins (see fix 1). The full text lives only in a hover tooltip.
- WARNING: the changed-on-disk body renders as `dynamic.filter changed since ff loaded it. Reload discards your edits. Overwrite replaces the file with them.` as one paragraph, where the spec gives two lines.
- WARNING: ABOUT `Version 0.5.0` on a v0.6 branch. This is not a UI-SPEC defect, but it looks stale to the user.
- Minor: the Economy placeholder shows an em-dash-free but very sparse screen. This is per spec ("No other text").

### Pillar 2: Visuals (3/4)

- Strong focal hierarchy: the gold title plate, the bright selected tab and the rightmost Apply button. Tabs overlap, with the selected stone drawn brightest (Rules in `rules.png`, About in `about.png`).
- Ferris sits faintly behind rows and is fixed to the window, as specified. It shows through the rows without hurting legibility.
- The flag icons are present and distinguishable: red `!` circle, amber triangle, amber `?` ring, padlock, gold diamond. Each has an 18pt hit rect and a tooltip. `icons.rs:118-120` sets a `widget_info` label and `on_hover_text`, so the icon-only buttons are accessible.
- WARNING: the broken-link icon at 14pt is not self-explanatory, and the padlock is very small (see fix 3).
- WARNING: `◆ A CONTINUE` is ambiguous (see fix 3).
- WARNING: the Config page's flat grey ComboBox (`DYNAMIC.FILTER`) and the grey `+ ADD STYLE` button are visually heavier than the stone-skinned buttons around them. This is the accepted interim legacy look, so it is not scored down.
- Modal dim is clear, the box is centred, and the title has an underline rule. The Overwrite caption is red as danger.
- needs_human_review: in `popup-changed.png` the bottom-bar Apply slot reads "Overwrite" in a dimmed state under the modal. Confirm in the running overlay that it is the red danger caption and is clickable after Keep editing.

### Pillar 3: Color (3/4)

- The palette follows the contract. Near-black ground, stone art, gold only on the selected tab, headings and their rule, the Continue word, the styled diamond and the ring-check. Red appears only on destructive popup buttons, the error status and the grammar-error icon. Status colours were seen in red (`Apply refused`) and amber (`Wrote C:/…` in `status-truncated.png`).
- Hardcoded colours are limited to `Color32::WHITE` for the `!` glyph and icon test paint (`icons.rs:65`), `from_white_alpha(110)` (`shell.rs:518`, disabled tint), `from_white_alpha(150)` (`popup.rs:130`, title rule) and `from_black_alpha(160)` (`popup.rs:37`, backdrop). These are intentional and spec-aligned.
- WARNING: the one hardcoded white `!` on the grammar-error icon should come from a palette constant for consistency.
- WARNING: accent weight is slightly diluted on the RULES list. Gold appears on every enabled-ring, every Continue word and every styled diamond, so one rule can show 3+ gold cues. That is arguably beyond "accent only on declared elements" in a busy list.
- The 60/30/10 split is plausible in the screenshots (dark ground, stone, gold), but a real distribution count was not possible without live paint.

### Pillar 4: Typography (3/4)

- Only Fontin Regular and SmallCaps are used. The grepped sizes in `shell.rs` and `popup.rs` are all 15, 17 and 21, plus the art-derived header/tab sizes, which are allowed exceptions. The `icons.rs` `!` glyph is 13pt, an unlisted size, but it is icon-internal.
- The hierarchy reads well: section headings (Filter, Styles, Economy, Credits) in 21 SmallCaps, labels in 17 SmallCaps, and values and body in 15 Regular.
- WARNING: the Config Built-in table's header "Name" is left-aligned but the cells are centred (see fix 4). The "Built-in" / "Custom" sub-labels use a different, smaller treatment than the section headings.
- WARNING: the ABOUT disclaimer wraps to four lines and orphans "way."; the contract says two lines.
- WARNING: the collapsed-row muted summary line (`Class: 6 items; Sockets >= 3…`) and the header line are nearly the same size and tone, so the two-line row has a weak internal hierarchy. This is legacy and partly deferred to Phase 31.

### Pillar 5: Spacing (2/4)

- BLOCKER-adjacent defect, not blocking a task: the search field touches the "+ Add rule" button with no gap, which violates the `sm` 8pt token and the spec's width arithmetic (`rules.png`).
- WARNING: the status slot gets about 250pt of text width. The buttons are wider than the 84 minimum (about 95pt each with 8pt gaps), which pushes truncation earlier than the contract anticipated.
- WARNING: Config table columns are not aligned (Name header versus centred cells). The custom Order arrows are tiny and tight to the right edge.
- WARNING: in the Config tab the first custom row is cut at the content bottom with no scroll affordance beyond the thin scrollbar. This is expected clipping, so it is not penalised on its own.
- WARNING: `◆ A CONTINUE` spacing is about 4pt where 8pt is needed.
- Passing: the content fits in the roughly 535pt width with no horizontal scroll. Popup padding, the title rule and the button gaps look like the contract's 24/8. The economy footer is pinned at the bottom of the frame. The art-fixed exceptions are accepted per the spec's own checker flag.

### Pillar 6: Experience Design (3/4)

- Confirmations: seven skinned modals replace inline confirms, each with Escape and click-outside mapped to the safe last button (`popup.rs:5`, Escape test at about line 278). Each popup carries an `egui::WidgetInfo` label equal to its title (`popup.rs:100`).
- The status line has a defined priority table. Event messages replace each other and clear on the next edit.
- Disabled states with reasons: Reload and Apply are dimmed when nothing is pending (visible in `rules.png`). Reasons live in tooltips, so not provable from stills.
- The Sync economy button has a loading state ("Syncing…", disabled) per spec, though it is not visible in a still.
- Empty and error states exist as heading-only states. The load-failure and no-styles states are covered in code and in the UAT checklist.
- WARNING: the key recovery information for the grammar gate (which rule) is only in a hover tooltip. The click-to-jump affordance is invisible apart from a pointing-hand cursor.
- WARNING: the Revert button stays enabled and bright on the RULES screenshot while Apply is refused. This is plausibly correct (dirty rules), but a ghost state on other tabs, where Revert is bright on ECONOMY and ABOUT in `economy.png` and `about.png`, appears to contradict the spec ("ECONOMY/ABOUT: disabled"). Revert looks dimmer there, so this is needs_human_review.
- WARNING: with no clear Overwrite proof in the shot (see Pillar 2).
- needs_human_review: keyboard Tab/Enter, hide-while-modal-open, multi-monitor re-dock and DPI 100-200% are all untestable from stills. They are listed in 30-HUMAN-UAT.md, which is still pending.

---

## Registry Safety

Skipped. shadcn is not initialized (UI-SPEC `shadcn_initialized: false`), and no third-party registries are used.

---

## Files Audited

- `.planning/phases/30-skinned-overlay-shell/30-UI-SPEC.md`
- `.planning/phases/30-skinned-overlay-shell/30-CONTEXT.md`
- `.planning/phases/30-skinned-overlay-shell/30-05-SUMMARY.md`
- `.planning/phases/30-skinned-overlay-shell/shots/*.png` (all 8)
- `src/overlay/shell.rs` (targeted greps)
- `src/skin/popup.rs` (targeted greps)
- `src/skin/icons.rs` (targeted greps)
