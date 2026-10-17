# Phase 34 — UI Review

**Audited:** 2026-10-16
**Baseline:** 34-UI-SPEC.md (extends 33/32/30-UI-SPEC)
**Screenshots:** no dev server (Rust egui app). Committed shots viewed: `economy-fallback.png`, `economy-synced.png`, `economy-invalid-tier.png`, `remove-economy-popup.png`, `rules-economy-divider.png`, `config-no-tiers.png`, `about.png` (re-rendered after 2475c48). No new shots rendered. Six shots predate the WR-01..03 fixes, so the malformed-prices display and the Revert/marker scoping are not visible in them. Code spot-checked: `config_view.rs::show_tiers_section`, `about_view.rs`, `shell.rs` (economy rows), `rule_list.rs` (`ECONOMY_TIP`). Hover, DPI, link click, real Apply/sync are needs_human_review (34-HUMAN-UAT / 34-VERIFICATION list them).

---

## Pillar Scores

| Pillar | Score | Key Finding |
|--------|-------|-------------|
| 1. Copywriting | 3/4 | Spec copy is verbatim on ECONOMY, ABOUT and the popup. The RULES divider reads `60 rules`, not the contracted `+ N locked rule blocks (economy)`. Invalid-tier shot shows no readable reason, and Apply is greyed. |
| 2. Visuals | 3/4 | Clear focal button on ECONOMY and the exact ABOUT order. The orange Ferris sits above a second, pale Ferris that fills the empty middle. The tier fields do not line up with the value column above them. |
| 3. Color | 3/4 | Palette-only, no new colours. Gold is on headings, the link and the Fallback warning, and red only on Remove. Fallback is amber, which is close to the gold link and labels. |
| 4. Typography | 3/4 | Only 14/15/17/21 and two faces. Tier letters render in Regular while the labels above them are small caps. |
| 5. Spacing | 3/4 | 16pt gaps and 34pt rows hold. `60 locked blocks` nearly touches the Remove button, the tier column sits left of the value column, and the ABOUT middle is a large void. |
| 6. Experience Design | 3/4 | Staged Add/Remove, scoped Revert/markers, danger popup, malformed-file reason on hover. Evidence is code and tests only, and the human checklist has 15 pending items. |

**Overall: 18/24** (Phase 33: 18/24)

---

## Top Priority Fixes

1. **WARNING — Invalid tier cutoff gives no visible reason.** `economy-invalid-tier.png` shows only a thin red outline on `abc`, a greyed Apply and a status that still says `Unapplied changes`. The cause (outline hover) and the Apply block (`APPLY_BLOCKED_TOOLTIP`) are hover-only. A user sees a dead Apply button. Fix: when any tier is invalid, make the status line read an error such as `Tier cutoffs are not valid` (the status slot already has an error colour and ellipsis), and keep the tooltips.
2. **WARNING — Tier fields do not line up with the rows above.** In all ECONOMY shots the Economy values start at x≈320, while the tier inputs start at x≈216 with a `chaos` unit after them. Two different control columns sit in one scroll area, so the page reads as two stacked widgets. Also the tier letters are Regular, not small caps like every other label. Fix: align the tier field to the same value column (`LABEL_COL_W` and control column) and use the SmallCaps label face for the tier letter.
3. **WARNING — `Filter section` row is crowded.** `economy-synced.png`: `60 locked blocks` ends at x≈323 and the Remove button starts at x≈331, a gap of about 8pt, and the MUTED text looks brighter than the labels' dimmer tone. Fix: give the count a right margin of 16pt, or move the count into the button's tooltip, which the spec already provides.
4. **WARNING — Two Ferris drawings on ABOUT, and the empty middle is the largest element.** `about.png`: the 112pt orange Ferris is repeated 6x larger as the pale background crab directly below the divider, and the divider itself is barely visible (2475c48 made it symmetric but still faint). The focal point competes with its own ghost. Fix: either suppress the pale background on ABOUT or lift the divider alpha (spec: `from_white_alpha(110)` over a dark ground). Decide in the not-texty UAT item.
5. **WARNING — RULES Economy divider summary differs from the contract.** `rules-economy-divider.png` reads `Economy ... 60 rules [lock]`, where the spec keeps `+ N locked rule blocks (economy)`. The tooltip text with `Add or remove it in the ECONOMY tab.` is present in code (`rule_list.rs:388`) but is hover-only; nothing visible tells a user where the removed Add and Sync buttons went. Fix: accept `60 rules` and amend the spec, or restore the contracted wording.
6. **WARNING — `Fallback prices` is amber, close to the gold of labels and the Sync button.** `economy-fallback.png`: the colour `WARN` is barely distinguishable from `GOLD`, so the one warning on the page does not stand out. `Priced items 0` beside it is correct. Fix: keep the colour but add the tooltip hint visibly (e.g. `Fallback prices (sync for live)`) or use the error-leaning end of the palette.
7. **Minor — ABOUT shows `Version 0.5.0` on the v0.6 branch** (`CARGO_PKG_VERSION`). Correct by construction, but bump the Cargo version at milestone close. Disclaimer line 1 wraps leaving `in any way.` orphaned; widening the footer to about 520pt gives one line.

---

## Detailed Findings

### Pillar 1: Copywriting (3/4)
- Verbatim matches in shots: section names `Economy`, `Economy section`, `Tiers`; labels `League`, `Last sync`, `Priced items`, `Price source`, `Filter section`; values `Never`, `Just now`, `Live prices`, `Fallback prices`; buttons `Sync economy`, `Add economy section`, `Remove economy section` (red); popup `Remove economy section?` / `Your own rules are kept. Add the section back any time.` / `Remove` (red), `Keep`. ABOUT: name, description, `Version 0.5.0`, link, both disclaimer sentences and three credit lines match the contract word for word.
- No standing paragraphs on ECONOMY, matching the not-texty memory. `shell.rs:678` writes `{n} locked blocks`, per spec.
- Gaps: fix 1 (invalid tier has no visible text), fix 5 (divider wording). `config-no-tiers.png` correctly shows Filter and Styles only; the old TIERS heading is gone.
- Status slot reads `Unapplied changes` everywhere (2fa9875 is resolved: short, no cut). In the ECONOMY and ABOUT shots it shows even though nothing was edited in `economy-synced.png`; this is the screenshot staging, not evidence of a bug, but the standing text does not say what is unapplied.

### Pillar 2: Visuals (3/4)
- Focal point works: `Sync economy` is the only framed button centred above the fold, with the four info rows zebra-striped above it (`economy-*.png`). Remove has the red caption, Add has none, so a destructive action is distinguishable at a glance.
- ABOUT order matches the contract exactly (name, description, Ferris, version, link, divider, void, disclaimer, credits). The Ferris is crisp at 100%. Credits are visibly dimmer and smaller than the disclaimer, as intended.
- Deficits: fixes 2 and 4. The pale Ferris also shows behind the Economy section and Tiers rows (`economy-fallback.png`, behind `Filter section`), which slightly reduces legibility of the row label and the count text. The spec scoped no background change, and the UAT lists this as a not-texty acceptance item.
- Icon-only controls: none added. Tab strip, close button and arrows are unchanged.
- Phase 33 shot-carryover: `config-no-tiers.png` still shows the legacy flat grey Filter dropdown (`DYNAMIC.FILTER`) against the skinned Styles table, and `UNCHANGED` previewing in saturated yellow on black (Phase 33 fix 7, unchanged). The first custom row is cut by the viewport, which is just scroll.

### Pillar 3: Color (3/4)
- Only `pal::` colours in the phase files. Grep of `about_view.rs` finds `Color32::TRANSPARENT` and `Color32::WHITE` (the tint for the Ferris art, spec-sanctioned) and nothing else. `INVALID_RED` is the single hardcoded colour in the tiers editor, kept by spec.
- Accent distribution matches the contract: gold on section headings, app name, GitHub link, tab text; red only on `Remove economy section` and the popup `Remove`. No gold fill on rows. The orange Ferris is art, flagged as such.
- Deficit: fix 6, WARN vs GOLD discrimination. The `abc` outline in `economy-invalid-tier.png` is the correct red but 1pt thin on a near-black field.

### Pillar 4: Typography (3/4)
- Sizes seen: 14 (version, credits, count, unit), 15 (values, link, disclaimer), 17 (row labels, buttons), 21 (headings, app name). No 36/16/13.5 carried over from the prototype. No bold; two faces only.
- Deficit: tier letters `S`..`F` are Regular, labels above them are SmallCaps (`economy-*.png`). The unit text `chaos` at 14 MUTED is very small next to a 112pt field.
- The ABOUT description is 15 Regular in the heading tone, while the spec says `LABEL`. It reads fine, but it is visibly brighter than the version line and close to the app name.

### Pillar 5: Spacing (3/4)
- 16pt gaps between Economy, Economy section and Tiers hold; rows are 34pt with the zebra restarting per section (`economy-*.png`). ABOUT top pad, 4/8/16 gaps and the footer pin at the bottom edge follow the spec; nothing clips at 660pt.
- Deficits: fix 3 (count nearly touching the Remove button) and fix 2 (two different control columns). The ABOUT void between the divider and the disclaimer is about 280pt, which is by design but heavy. Tiers end with about 70pt of empty page above the bottom bar on ECONOMY.
- 33 carry-over (Hex and `R` label touch) is not in the phase scope.

### Pillar 6: Experience Design (3/4)
- Covered: Add stages in memory and flips the label, Remove asks in a popup with Escape and backdrop as Keep (`remove-economy-popup.png` shows the dimmed window and red Remove), tab markers scoped (ECONOMY for tiers and a staged op, CONFIG for styles), Revert scoped with the Discard popup, tier validation gating Apply (`economy-invalid-tier.png`: Apply greyed, Revert enabled), sync off the UI thread, WR-01 (staged op survives a Keep editing), WR-02 (marker ownership) and WR-03 (malformed or empty `prices.json` shows the reason and `Never`) are in code with tests.
- Gaps: fix 1 (invalid tier cause is hover-only; a greyed Apply says nothing). `Syncing…` state, the `Fix the filter first.` disabled tooltip, the malformed-prices hover, and the exact-time hover were not shot. The hover/DPI/link/real-Apply/sync/in-game reload checks are all `[pending]` in 34-HUMAN-UAT.
- Fix(33) polish commits, as far as the shots can say:
  - 2fa9875 `Unapplied changes` indicator: resolved in every shot (short text, no cut).
  - 96d5861 label ellipsis: not evidenced (no long label on these tabs; ECONOMY labels fit).
  - 6ca7ba7 dimmed Font size `Default` slider: not evidenced (no style editor shot).
  - 2816f13 picker frame: not evidenced (no picker shot).
  - 2475c48 ABOUT divider: visible and symmetric but very faint.
- No registry audit (no shadcn).

---

## Registry Safety
Not applicable (not a web project; `components.json` absent; one first-party PNG, `assets/ui/ferris-about.png`, 9 KB).

## Files Audited
- `src/overlay/about_view.rs`, `src/overlay/config_view.rs` (`show_tiers_section`), `src/overlay/shell.rs` (economy rows), `src/overlay/rule_list.rs` (`ECONOMY_TIP`) via greps and targeted reads
- `.planning/phases/34-economy-about-tabs/34-UI-SPEC.md`, `34-VERIFICATION.md`, `34-REVIEW-FIX.md`, `33-UI-REVIEW.md` (format and carry-over)
- Shots: `economy-fallback.png`, `economy-synced.png`, `economy-invalid-tier.png`, `remove-economy-popup.png`, `rules-economy-divider.png`, `config-no-tiers.png`, `about.png`
