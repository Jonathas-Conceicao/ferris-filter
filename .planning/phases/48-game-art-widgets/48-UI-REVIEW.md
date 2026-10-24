# Phase 48 — UI Review

**Audited:** 2026-10-23
**Baseline:** 48-UI-SPEC.md (approved design contract) plus the game reference crops `.planning/research/ui-refs/social-*.png`
**Screenshots:** captured by the debug `--screenshot` mode (no dev server or Playwright; egui app). Reviewed: `rules-headers.png`, `rules-search.png`, `config-refresh.png`, `economy-refresh.png`, `zoom-search.png` in `C:\Users\jonat\AppData\Local\Temp\claude\ffshot48\`. No new captures were taken. Not covered by any screenshot: the economy header (collapsed plate), the `+ condition` picker search, hover/pressed art, and DPI above 100%.

---

## Pillar Scores

| Pillar | Score | Key Finding |
|--------|-------|-------------|
| 1. Copywriting | 3/4 | `Clear search` copy matches the spec, but the RULES placeholder now truncates to `Search 20 rules, classes, ...` and drops "base types" |
| 2. Visuals | 3/4 | Plate, bar, magnifier and X match the game; right-cluster controls fight the bar art, and the refresh plate is taller than its dropdown |
| 3. Color | 3/4 | No new constants, art painted untinted; field and bar frames are much duller than the game's bronze and the up/down arrows lose contrast on the bar |
| 4. Typography | 3/4 | Reuses 15pt Fontin; no italic placeholder (declared); title offset leaves the title visibly off-centre against the game's centred titles |
| 5. Spacing | 3/4 | Geometry is spec-exact and unit-tested, but the 8/10/32pt search insets squeeze the placeholder, and the refresh button overshoots the row |
| 6. Experience Design | 3/4 | Click, focus and hover states are covered and the focus bug review items are fixed; picker X and economy header have no automated or visual check |

**Overall: 18/24**

No pillar scores 1, so there is no BLOCKER. Everything below is a WARNING.

---

## Top 3 Priority Fixes

1. **Right-cluster controls sit on the right cap's ornament and the up/down arrows are nearly invisible** (WARNING). In `rules-search.png` and `zoom-search.png` the dimmed arrows, the pencil and the remove X overlap the bar's swirled end and read as noise. Users cannot tell which arrow is live. Fix: keep the cluster inside the flat middle of the bar (end it at about `bar.max.x - 72 + 8`), or draw the cluster with a brighter rest tint on the bar. Spec judgement call 3 accepted "mild overlap", but the screenshots show more than mild.
2. **RULES search placeholder is cut mid-phrase** (WARNING). The field is about 240pt wide and the reserved 32pt on each side leaves room for `Search 20 rules, classes, ...`. The part that tells users base types are searchable is lost, even when the field is empty and the X is not shown. Fix: shorten the hint (`Search 20 rules…`, keeping the rest in a tooltip), or reserve the right 32pt only while the X is visible. The spec chose "always reserve" for no text reflow, but that rule costs text only when the field is empty, where nothing can reflow.
3. **Refresh button grows to about 3px taller than its dropdown on each side** (WARNING). `config-refresh.png` and `economy-refresh.png` show the plate centred correctly (ART-04 offset is fixed), but it stands taller than the neighbouring combo, and the glyph ring looks heavy beside it. The spec called this judgement call 5. Fix: either size the plate to the dropdown height (about 26pt), or raise the dropdown to `ROW_HEIGHT`. Leave this for the author's call, since the spec approved it.

Further fixes (minor, no priority order):
4. Economy header (collapsed plate, padlock) was never captured. Take `--tab rules` with the economy section visible or `--expand-economy` and confirm the dot-over-chevron plate and padlock position.
5. The picker search X (inside a popup) has no automated test and no screenshot (`--picker rule` or the `+ condition` popup). Capture it.
6. Re-capture at 150% and 200% (`pixels_per_point`) to confirm no bar seams. Only unit tests at ppp 1.0, 1.25, 1.5, 2.0 cover this.

---

## Detailed Findings

### Pillar 1: Copywriting (3/4)
- `Clear search` is both tooltip and accessible name (`src/skin/widgets.rs`, `on_hover_text("Clear search")` and `WidgetInfo::labeled(..., "Clear search")`). Matches the contract.
- Toggle plate accessible names are `Expand section` / `Collapse section` (REVIEW-FIX WR-02), named for the action. Good.
- Empty-state and status copy unchanged, as specified. No new generic labels.
- Placeholder truncation (fix 2 above): `rules-headers.png` shows `Search 20 rules, classes, ...`, the contract says placeholders are "unchanged". The text is unchanged in source but not in what the user reads. This is the pillar's only real defect.
- The status line `Apply refused: 1 enabled rule has a...` is the deliberate fixture's grammar error, not a phase issue.

### Pillar 2: Visuals (3/4)
- Header: plate at left, bronze stone bar, left-aligned title, `n rules` count, right controls (`rules-headers.png`, "Leveling"). Clear focal hierarchy: plate, then bar, then title. Chevron-up-over-dot reads as the expanded state, matching the game's `social-sections.png`.
- Bar is contiguous, with no seams at 1x. The game has a 3px gap between bars (dropped, judgement call 4), so stacked headers (`rules-search.png`, Leveling above Endgame) touch the rule row above and the next header; the bars lack the separation seen in the reference.
- The title sits at the bar's left rather than centred as in the game. Declared in CONTEXT (left-aligned), so not a defect, but the 60pt offset leaves a visible empty stretch left of "Leveling".
- Right cluster over the cap swirls: see fix 1. The rule rows' arrows (`rules-search.png`) are dim too, but that is Phase 49 scope.
- Search field: magnifier at left, round X at right (`zoom-search.png`). The glyph is small and low-contrast at rest, compared with the game's bright magnifier; the field border is a thin grey line against the game's bronze frame. Spec-conformant (`INPUT_BORDER`), but less game-like than the reference.
- X art is centred, crisp and not occluded. Matches the reference (`social-search-fields.png`).
- Refresh buttons: centred on the dropdown's row on both tabs (ART-04 satisfied), but taller than the dropdown (fix 3).
- Icon-only controls all have names: X (`Clear search`), plate (Expand/Collapse section). The magnifier is decorative by design and verified not to be a Tab stop (`search_glyph_is_not_a_tab_stop`).

### Pillar 3: Color (3/4)
- No new palette constants. New art is painted with `Color32::WHITE` (untinted), which matches the spec. The only literals in the diff are geometry values; no hardcoded RGB was added.
- Accent use is within contract: `pal::HEADING` title hover, `INPUT_BORDER_HOT` focus ring, plate hover art. The bar hover wash is `pal::ROW_HOVER`.
- The stone bar and plate bring in much more bronze than the previous flat `PANEL` headers, so the RULES list now has a dominant warm band per section; the 60/30/10 split still holds (rows remain dark ground).
- Contrast problem: dimmed up/down arrows (disabled tint) over the textured bar (fix 1). Nothing else fails.

### Pillar 4: Typography (3/4)
- Only 15pt Fontin Regular (field text and placeholder) and the existing section title style are used. No new sizes or weights.
- No italic placeholder (Fontin has none), approximated by `pal::DISABLED`. In `rules-headers.png` the placeholder is plain grey, close enough, but it does not look as soft as the game's italic (`social-search-fields.png`).
- Text-start shift to 32pt leaves the typed text aligned with the glyph as intended (`rules-search.png`, "ring").

### Pillar 5: Spacing (3/4)
- Spec geometry is implemented as pure functions with tests: plate 27x32 at (+4, +2), bar starting 4pt after the plate, title at `bar.min.x + 60`, glyph 16x16 at +10, edit insets 32/32, X disc 18 with a 24 hit target centred 17pt from the right.
- Search toolbar spacing unchanged (`+ Add rule`, `+ Add section`, `FIELD_GAP`).
- Cost of the 32/32 insets on a narrow field: see fix 2.
- Refresh button fills the full row height (`ROW_HEIGHT`) and is thereby taller than its neighbour (fix 3).
- Contiguous bars (no 3px game gap) accepted by the spec.

### Pillar 6: Experience Design (3/4)
- Header: whole bar toggles; plate has normal/hover/pressed states; keyboard focus ring on the bar; plate is `Sense::CLICK` (non-focusable), so the bar stays the single Tab stop (REVIEW-FIX WR-02).
- Search: the X shows only with text, clears and keeps focus (the test now focuses the edit first, REVIEW-FIX WR-03). Magnifier click focuses the field. X is not a Tab stop, Escape is unchanged. Hover shows the pointing hand.
- Not verified: the picker X click leaving the popup open (documented as a manual check in the 48-03 summary and in the verification file's human items); hover and pressed visuals; economy header; DPI above 1x. The phase still has no screenshot or harness evidence for any of these.
- State coverage of existing flows (empty list, search matches, collapse during search) is unchanged. In `rules-search.png` matched sections show `1 of 4 rules` with the plate in expanded state; that matches the displayed-state rule.
- No registry or shadcn audit (Tool: none).

---

## Files Audited
- `.planning/phases/48-game-art-widgets/48-UI-SPEC.md`, `48-CONTEXT.md`, `48-01..04-PLAN.md`, `48-01..04-SUMMARY.md`, `48-REVIEW-FIX.md`, `48-VERIFICATION.md`
- `src/overlay/rule_list.rs`, `src/overlay/body.rs`, `src/skin/widgets.rs`, `src/skin/slice.rs`, `src/skin/textures.rs` (diff against 9a56894, constants and a11y strings)
- Screenshots: `rules-headers.png`, `rules-search.png`, `config-refresh.png`, `economy-refresh.png`, `zoom-search.png`
- References: `.planning/research/ui-refs/social-sections.png`, `social-search-fields.png`
