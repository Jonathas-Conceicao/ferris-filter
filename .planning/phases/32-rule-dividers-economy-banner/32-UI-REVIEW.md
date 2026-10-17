# Phase 32 — UI Review

**Audited:** 2026-10-16
**Baseline:** 32-UI-SPEC.md (approved with FLAGs; extends 31-UI-SPEC / 30-UI-SPEC)
**Screenshots:** no dev server (Rust egui app). Viewed: `sections.png`, `economy-expanded.png`, `rename.png`, `search-collapsed.png`, `section-above-row.png`, `05-economy-wr02.png`. Code checked in `rule_list.rs`, `rules.rs`. Hover, keyboard, persistence across restart and DPI are needs_human_review (32-HUMAN-UAT.md is 0/9).

---

## Pillar Scores

| Pillar | Score | Key Finding |
|--------|-------|-------------|
| 1. Copywriting | 3/4 | Every spec string and tooltip is present and icon-first. The new statuses (`Section added…`, `Section removed…`) will be cut by the status line and are not visible in any shot. |
| 2. Visuals | 3/4 | Headers read as dividers: chevron, small-caps name, rule line, count, icon cluster, as in the mockup. The header ground is barely distinct from the rule rows, and the Economy rows are very dim. |
| 3. Color | 3/4 | No new colours. Gold stays off headers. The amber WR-02 triangle is the only new warm accent. One stale shot still shows an open padlock. |
| 4. Typography | 4/4 | Fontin small-caps 17 name, regular 15 muted count, no new sizes. |
| 5. Spacing | 3/4 | 36pt headers against 52pt rows give a clear rhythm, and the long name is cut before the count. `+ Add section` is ~151pt, not 140, so the search field is ~241pt, not 255. |
| 6. Experience Design | 3/4 | Collapse, rename, search expansion, locked Economy and the absent-economy entry point are all covered. Rename and Remove stay live during search, the Economy x leaks into the UI as an interim, and the bottom row is not evidenced in any shot. |

**Overall: 19/24** (Phase 31: 19/24)

---

## Top Priority Fixes

1. **WARNING — the `Section above` row has no visual evidence.** `section-above-row.png` does show `SECTION ABOVE / DUPLICATE / DELETE` below the expanded rule, with Delete red and rightmost. The row is therefore evidenced, but only on a one-rule, two-section file with an empty rule. The group case (banner goes above the whole run) and the rename-focus follow-up are headless-test only. Fix: add one shot of a grouped rule with `Section above` clicked, and confirm in UAT items 2 and 7.
2. **WARNING — new status strings are invisible.** The status slot reads `Filter out of date – press Apply, th…` in every shot, so `Section added at the top. Press Apply to save it.` and `Section removed. Its rules joined the section above.` (`rules.rs:238-239`, `1423`) are cut after about 40 characters. Both are event messages and show until the next edit, but they start with the less important part. Fix: shorten them (`Section added`, `Section removed`) or give the status line a tail-preferring cut. This is Phase 30 shell code, but this phase adds to the problem.
3. **WARNING — header and row grounds are too close.** In `sections.png` the PANEL header with 1pt rules is only slightly different from ROW_EVEN/ROW_ODD, so the `Leveling` and `Endgame` headers read mainly through the chevron and name. The 36 vs 52pt height helps. The collapsed Economy header looks nearly identical to a rule row apart from the padlock. Fix: slightly raise the top-rule contrast (`INPUT_BORDER` to `INPUT_BORDER_HOT` at low alpha) or add a faint `HEADING` underline; no new colour needed.
4. **WARNING — controls live where they do nothing useful.** During a search (`search-collapsed.png`) the section keeps the live pencil and x while the arrows dim. Removing a section while a filtered list is shown is surprising. Fix: dim rename and remove too, or keep them live and accept it, but decide deliberately (spec silent).
5. **WARNING — interim Economy x.** `economy-expanded.png` and `05-economy-wr02.png` show a plain x beside the padlock on the locked header. It is the spec's interim (Phase 34 deletes it) but contradicts "locked, collapse/expand only" and sits next to a closed padlock, where it reads as a per-section remove. Fix: tooltip is present (`Remove the economy section`); keep it, and track removal in Phase 34.
6. **WARNING — stale evidence.** `05-economy-wr02.png` still shows the old open padlock (`32-06` fixed it later), and `search-collapsed.png` / `economy-expanded.png` hover artefacts muddy the rows. Re-shoot `05-*` or delete it so the folder agrees with the code.

Minor: `+ Add section` width and field width differ from spec arithmetic (disclosed in 32-05 summary). The `Add economy section` footer button (`section-above-row.png`) is left-aligned at 180pt, as specified, but sits in a dead band below the list with a visible gap. The long name test cuts at `A very long section name that should b…` with the count intact and no overlap (pass).

---

## Detailed Findings

### Pillar 1: Copywriting (3/4)
- Present and matching spec: `ADD_SECTION_TIP` (`rule_list.rs:1370`), `ECONOMY_TIP` (`:394`), `Move section up/down`, `Rename section`, `Remove section (its rules stay)`, `Collapse`/`Expand`, `Inserts a section divider directly above this rule.` (`:1241`), `Appends the generated economy section…` (`:1389`), `Clear the search to reorder` (`:671`).
- Counts: `3 rules`, `1 rule`, `1 of 4 rules` (search) all verified in shots. Default section name and the headers show in the source case through small caps (`Leveling` renders `LEVELING`).
- Author's "not texty" preference is met: no captions, no standing paragraphs; the only words are the section name, counts and button labels.
- Gap: the two new status strings cannot be seen (fix 2). Disabled arrows give no tooltip at all outside a search (`rule_list.rs:678-691`), so a dimmed arrow is unexplained.

### Pillar 2: Visuals (3/4)
- Positive: the header anatomy is exactly the mockup (`sections.png`): chevron, name, line, count, pencil, up, down, x. The Economy header shows just the padlock, no pencil or arrows. The WR-02 amber triangle on rule 1 is a quiet, text-free cue (`sections.png`, `05-economy-wr02.png`). The padlock is a closed shackle in the latest shot. `rename.png` shows an inline field with selected text and the cluster still visible.
- Economy rows: muted, no check or arrows, name and summary aligned at the same x as normal rows, so columns line up. They are dim to the point of low contrast, but that is the intent.
- Deficits: see fix 3. The `Section above` button is the leftmost of three equal-weight buttons, with no icon, which costs scan speed; acceptable per spec.
- Focal point: unchanged from Phase 31.

### Pillar 3: Color (3/4)
- Only existing palette tokens appear. `Color32` hits in `rule_list.rs` are `PLACEHOLDER` (galley paint) and a `WHITE` for text measurement (`:905`, harmless). No hardcoded hex.
- Gold is not used on headers or chevrons; the section name uses `HEADING` per spec; the count is `MUTED`.
- The amber WR-02 triangle plus the amber warning-triangle language adds a second amber cue to rule 1 only. It is correct, but when it coexists with `Continue` and styled flags the row gets busy (not exercised in any shot).
- Stale open padlock in `05-economy-wr02.png`; the code and `sections.png` are fine.

### Pillar 4: Typography (4/4)
Only small-caps 17 (name, buttons, field text), regular 15 (count, summaries), 14 and 21 elsewhere. No bold. Long names cut with an ellipsis, not wrapped.

### Pillar 5: Spacing (3/4)
- 36pt header vs 52pt rows is clear. Cluster gaps 4pt, ROW_PAD 10, rule line begins 8pt after the name (`sections.png`).
- Deviation: `+ Add section` ~151pt (button caption padding of 44pt), so the field is ~241pt instead of 255pt. The hint is already cut (`Search 9 rules, classes, base typ…`), so the cut is slightly earlier than in Phase 31 (`rules, classes, base typ…`). Accepted in the summary but still a drift from the contract.
- Bottom row gaps are now 8pt (fixed in 32-05). Rule line is omitted where it would be under 16pt (long name, correct).
- Empty vertical band between the last section and the `Add economy section` button in `section-above-row.png`.

### Pillar 6: Experience Design (3/4)
- Covered: collapse and expand with chevron and header click, collapse state per filter without dirtying the filter (headless tests), rename Enter/Escape/trim, search auto-expands collapsed sections with `m of n rules` and hides empty sections (`search-collapsed.png` shows Leveling and Economy hidden), arrows dim at the ends (`leading-divider.png` per summary, `sections.png`), Economy collapsed by default, `Add economy section` only when absent (`economy-absent.png`), no popup for section remove (Revert undoes it).
- Gaps: rename and remove stay live while searching (fix 4); the interim Economy x (fix 5); `32-HUMAN-UAT.md` is 0/9 passed, so keyboard toggle (Enter/Space), restart persistence and `ff-cli apply` flows are code-and-test evidence only; header click during search is a no-op (documented decision, but the header looks clickable with the expanded chevron).
- No registry audit (no shadcn).

---

## Registry Safety
Not applicable (not a web project; `components.json` absent).

## Files Audited
- `src/overlay/rule_list.rs`, `src/overlay/rules.rs`, `src/overlay/app.rs` (greps), `src/skin/icons.rs`
- `.planning/phases/32-rule-dividers-economy-banner/32-UI-SPEC.md`, `32-CONTEXT.md`, `32-05-SUMMARY.md`, `32-06-SUMMARY.md`, `32-HUMAN-UAT.md`
- Shots: `sections.png`, `economy-expanded.png`, `rename.png`, `search-collapsed.png`, `section-above-row.png`, `05-economy-wr02.png`
