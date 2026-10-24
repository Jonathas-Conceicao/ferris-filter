# Feature Landscape: v0.7.2 Small UI Improvements

**Domain:** egui Windows overlay editor for PoE2 loot filters (subsequent milestone)
**Researched:** 2026-10-23
**Overall confidence:** MEDIUM. Behaviour norms come from established UI conventions (OS/web/game-app drag lists, hotkey capture in tools such as PowerToys, Discord, OBS, Exiled Exchange 2, Awakened PoE Trade; theme/style extraction tools) and the author's backlog text. No external sources were fetched this session, so the comparable-app claims are training-knowledge (LOW-MEDIUM) and the "dependencies on existing FerrisFilter" claims come from CLAUDE.md and ROADMAP 999.19-999.27 (HIGH).

**New Rust dependencies:** none are required by any feature below. Flagged items: (a) hotkey capture needs an egui-key to hotkey-code mapping table (hand-written, or whatever the existing `hotkey.rs` crate already provides; check what `TOGGLE_CODE` is typed as before planning); (b) throttle timestamps use `std::time::SystemTime` + the existing JSON handling. Drag-and-drop uses egui's own `Sense::drag`, `Ui::painter`, layer ids (`egui::Order::Tooltip`/`Foreground`); do NOT adopt `egui_dnd` or similar.

---

## 1. Style colour alpha

### Table stakes
| Feature | Why expected | Complexity | Notes / dependency |
|---|---|---|---|
| Alpha slider in the style colour picker (Text/Border/Background cards), with a numeric field | Rules' picker already has an Alpha slider; styles lacking it is an inconsistency users will notice | Low-Med | Shared colour picker already supports alpha for rules ("no alpha for styles" is a flag today); reuse it |
| Byte-identical round trip for alpha-less styles | Author's explicit constraint; `artifact_guard` + seed fixtures | Med | Model colour as `[r,g,b]` with optional alpha: absent alpha emits `R G B` exactly as now; store alpha only when != 255 (or when explicitly set). Decide: does alpha 255 typed explicitly write 4 values? Follow the rules' convention: "3-token stays 3-token while alpha is 255" |
| Alpha through every emitter (generate, economy section, styled-rule rewrite) | One action emitter is already shared | Low-Med | The CLAUDE.md claim "one action emitter" makes this a single change point; verify the emit path in generate, `apply`, and the overlay rewrite really share it |
| Preview card/label preview shows alpha | Users judge the result visually | Low | Draw swatch over a checkerboard or the game bg so translucency is visible; the item-label preview already exists |
| styles.json parse: accept 3 or 4 values/field; reject out of range | Existing validation style (load-time named errors) | Low | Choose the on-disk JSON shape up front (e.g. optional `"alpha"` per colour, or 4-element array) so old files load unchanged |

### Differentiators
| Feature | Value | Complexity | Notes |
|---|---|---|---|
| Checkerboard behind swatch | Clarity | Low | Pure paint |

### Anti-features
| Anti-feature | Why avoid | Instead |
|---|---|---|
| Forcing alpha 255 into existing files on save | Breaks byte-identity and the "tag-only migration is not an edit" principle | Write alpha only when set |
| Treating alpha 0 as "off" | Alpha 0 is a legal invisible colour; None tile is already the "off" | Keep None tile for removal |

Edge cases: Auto Detect must read `R G B A` with alpha and compare it exactly (255 vs absent 3-token are the same style, since the game treats missing alpha as 255 for text/border; background default is 240 per CLAUDE.md, so absent background alpha is NOT equal to explicit 255; compare against the effective value and decide how it is stored). This is the main trap; resolve in the alpha phase because Auto Detect depends on it.

---

## 2. Auto Detect styles

Comparable tools: theme/palette extractors (design-token extraction, "create style from selection" in Figma/Word/Photoshop "match existing style", linters that dedupe repeated CSS declarations). Common contract: exact-match first, group identical remainders, name generically, show a summary, make it undoable.

### Table stakes
| Feature | Why expected | Complexity | Notes / dependency |
|---|---|---|---|
| Exact equality on all parameters, no fuzzy merging | Author decision | Med | Needs a `parse rule actions -> Style` function, the inverse of the action emitter; round-trip tests (emit then parse equals) are the oracle |
| Match order: built-ins (current, incl. edits) and existing custom first, then new "Custom Style N" for each distinct remainder | Author decision | Med | Ambiguity: two styles equal to each other (a custom clone of a built-in): pick deterministic priority (built-in first, then file order) |
| Bare `Show` = `unchanged`; bare `Hide`? | Visibility is part of the style (`action`) | Low | Decide: Hide with no actions is not `hidden` (which has font 21 + DisableDropSound); it becomes a new custom Hide style or stays unmatched. Needs explicit rule; list in plan |
| Skip: already tagged, economy section, repeated family lines, unsupported keywords | Author decision | Med | The overlay already detects "line the editor cannot show exactly" and "earlier lines of a family"; reuse those predicates as the "inexpressible" test |
| `# ff:group` counted once, disabled rules included | Author decision | Low | Existing group model is one rule |
| In-memory only, one pending edit, Revert/Reload behave per tab | Author decision | Med | Touches both tabs: style additions (Config edits, ` *` on Config) AND rule tags (Rules edits). One button, two undo domains: define what Revert on each tab does (Rules revert drops tags; Config revert drops new styles, leaving tags pointing at ids that no longer exist = "unknown style"). Strong recommendation: Revert from either tab must not leave dangling tags; consider making Auto Detect a single combined pending edit, or auto-handling. Highest-risk design point |
| Result summary in the status line | Users must know what happened | Low | e.g. `Auto Detect: tagged 41 rules, 3 new styles.` and a no-op message (`Nothing to detect.`). Matches the one-status-line design |
| Idempotent | Running twice changes nothing | Low | Follows from skipping already-tagged rules |
| New style id is random custom id; names `Custom Style N` unique against existing names (incl. case-insensitive clash with required names) | styles.json load rules | Low | N continues from max existing |

### Differentiators
| Feature | Value | Complexity | Notes |
|---|---|---|---|
| Confirmation popup with counts before applying the in-memory edit | Safety for a bulk edit | Low-Med | Popups are a fixed ten; adding one is acceptable but not needed since edits are in memory and revertible; skip unless Revert split is awkward |
| Ordering new styles by first use in the file | Predictable | Low | Free if iterating rules top-to-bottom |

### Anti-features
| Anti-feature | Why avoid | Instead |
|---|---|---|
| Fuzzy/near match, "snap to nearest style" | Author said exact | none |
| Auto-naming from content ("Pink Star") | Names would churn and collide | `Custom Style N` |
| Auto-running on load | Opening must never write or edit | Button only |
| Re-tagging already-tagged rules | Overwrites deliberate choices | Skip |
| Touching economy rules | Locked/read-only | Skip |

Dependencies: alpha (section 1) must land first; action parser; custom-style add path; tag write (`# ff:style <id> <name>`); style report counts.
Edge cases: colour written 3-token vs 4-token equal; `MinimapIcon -1`/`PlayEffect None`/`PlayAlertSound None` map to the `"None"` style value, not "off"; `CustomAlertSound` file vs Alert N; positional; font size outside 1-45; trailing comments on action lines (kept? the tag-only rewrite on Apply will rewrite those actions from the style, dropping comments: warn or accept; since styled rules are rewritten on Apply, a rule whose actions are equal gets rewritten byte-differently only if formatting differs; pick: Apply rewrite from the style is the existing behaviour, so accepted); a rule whose `Continue` line exists (Continue is separate from style); conditions are irrelevant.

---

## 3. Drag-and-drop reordering (rules + custom styles)

Comparable: Trello/Notion/VS Code explorer/browser tab strips/Windows Settings lists/ OBS source list, egui_dnd-style lists. Expected: handle press starts drag after a small movement threshold, ghost follows cursor, source dimmed, insertion line, auto-scroll near edges, Escape cancels, drop outside cancels, cursor changes (grab/grabbing).

### Table stakes
| Feature | Why expected | Complexity | Notes |
|---|---|---|---|
| Handle with normal/hover/pressed art; grab cursor icons (`CursorIcon::Grab/Grabbing`) | Affordance | Low-Med | Author draws art; placeholder first (as backlog says); keep the handle's hit size >= arrow button size |
| Press threshold before drag starts (egui `drag_started`), no accidental drags on click | Standard | Low | egui handles |
| Greyed source row, translucent shadow, drop marker line | Author spec | Med | Shadow = paint the row's layout into a foreground layer at pointer position; with fixed two-line rows (known height) it is simple; snapshot the row before drag |
| Wheel scroll + edge auto-scroll, speed ramps with proximity | Author spec | Med | egui ScrollArea: set `scroll_with_delta` / `scroll_to_rect`-style offset changes manually while dragging; needs `request_repaint` each frame; virtualized list means compute drop slot from row heights, not from drawn widgets |
| Escape cancels; release outside list cancels | Author spec | Low | Existing Escape handling in text fields must not collide; consume Escape while dragging |
| One undoable edit | Author spec | Med | Single model move op, same path as arrow move today |
| Group = one unit; `moved across Continue` flag; section banners (drop past banner enters that section); economy locked; disabled while searching; customs never above built-ins | Carried constraints | Med-High | Valid drop slots computed from the model, not pixels: slot list = between rule units, after section banners; invalid slots simply not offered (marker hidden, release = cancel). Reuse the arrows' move-validity code by expressing a drop as "move unit to index" |
| Expanded row: collapse it on drag start (or drag by its header) | Avoids huge ghost, variable heights break slot math | Low-Med | Recommend collapse on start; restore expansion on the dropped row? decide; one-row-expanded invariant already exists |
| Drop marker with section awareness | Clarity at banners | Med | Banner rows are not drop "rows"; slot directly under a banner = first in that section; slot above the economy header is the last allowed |
| Dragging while window hides/focus lost cancels | Robustness | Low | Window hide on hotkey/game-foreground change must cancel, or the drag state sticks |

### Differentiators
| Feature | Value | Complexity | Notes |
|---|---|---|---|
| Keyboard reordering (Alt+Up/Down on a focused row) | Open point in backlog: arrows are going away so accessibility regresses | Med | Recommend deferring, but record as follow-up; losing arrows removes the only non-pointer way |
| Section headers draggable (move section with its rules) | Natural expectation once everything drags | High | Backlog open point; today's section arrows move only the divider. Recommend NOT in v0.7.2: different semantics (divider-only vs block move) need a decision; keep section arrows for now OR define divider-only drag. Flag as scope question |
| Drop-target auto-expand of collapsed sections on hover | Long lists | Med | Defer |
| Animated row gaps | Polish | Med | Skip |

### Anti-features
| Anti-feature | Why avoid | Instead |
|---|---|---|
| Whole-row drag (press anywhere) | Conflicts with expand-on-click, text fields, sliders | Handle only |
| Live reorder during drag (rows shuffle under cursor) | Fights virtualization and group/Continue validity | Marker + drop on release |
| Multi-select drag | Out of scope | Single unit |
| New dnd crate | Author reviews deps | egui built-ins |

Edge cases: ~500 rule lists (slot lookup by cumulative heights/binary search, not iterating drawn rows); dropping at the same position = no edit (no pending marker); dropping into the same slot after the unit; pointer over the scrollbar; drag started while Apply in flight (disable); grouped-rule broken-flag rows are separate units; disabled/commented rules draggable; collapsed section's rules (drop onto a collapsed section header = ambiguous: disallow or treat as "top of section"); DPI (handle hit target in points); custom style table has expand editors inline: same collapse rule.

---

## 4. Social-panel section headers (999.23)

### Table stakes
Toggle plate with two glyph states (game convention: chevron-up over dot = expanded; reversed from the usual), bar texture three-slice, title left-aligned `Name` with `{n} rules`, existing pencil/arrows/x/padlock on the right, whole bar clickable. Complexity Med (art lookup via `ff-art list` is the risk; fallback is painting). Dependencies: `draw_section_header` in `rule_list.rs`, economy header variant (padlock, muted), arrows which 999.26 may remove, `ff-art` manifest + committed assets. Keep 36pt height so the virtualized row-height math is unchanged.
Differentiators: hover/press states on the plate (Low). Anti-feature: centring the title like the game (conflicts with controls). Edge: long names truncate with ellipsis before controls; search-expanded display (`m of n rules`) keeps working; hit-testing the plate vs the bar must both toggle, but pencil/x must not.
Coordination: section header controls change with 999.26 (arrows removed/handle added); sequence them in the same or adjacent phases to avoid redrawing twice.

## 5. Search field clear X and magnifier (999.24/25)

### Table stakes
| Feature | Complexity | Notes |
|---|---|---|
| X appears only when text non-empty; click clears and keeps focus; Esc still clears (RULES toolbar) | Low | Needs the clear to act as a normal text change so the filtered list recomputes; applies to RULES search and the + condition picker search |
| Magnifier as leading decorative glyph, placeholder shifted right | Low-Med | Custom text field widget must reserve left/right padding so text never runs under icons; `ff_text_field_id/_ex` is the single place |
| Hit area >= ~18pt even though the disc is small | Low | |
| Glyph extraction via `ff-art` if it exists, else paint | Low-Med | Same art pipeline risk as 4 |
Differentiators: italic grey placeholder per the game style (Low). Anti-features: adding the clear X to non-search fields (name fields, hex fields) — leave off; magnifier as a clickable button.
Edge: the X must not steal the focus loss/commit logic of the field; picker search clear must not close the picker; Escape precedence (clear text first, then close).

## 6. Customizable hotkey (999.19)

Comparable: PowerToys shortcut control, OBS hotkeys, Discord keybinds, EE2/APT overlay hotkey settings, AutoHotkey GUIs. Expected: click the control (it enters "press shortcut" state), press a combination, it shows live and commits on key-up of a non-modifier; Esc cancels; a clear/reset-to-default button; conflicts reported.

### Table stakes
| Feature | Complexity | Notes |
|---|---|---|
| Capture control on Config tab: click -> "Press new shortcut...", records modifiers + one key, commits on non-modifier key press, Esc cancels | Med | egui gives key events + modifiers; the hotkey itself is a global OS registration, so capture is egui-side, registration is a separate step. Beware the overlay's own hotkey firing while capturing: unregister or ignore the toggle while the capture control is active (otherwise pressing the current combo hides the window) |
| Require at least one modifier (Ctrl/Alt/Shift) plus a key; reject bare keys, modifier-only, and reserved combos (Win-key combos, Ctrl+Alt+Del) | Low-Med | Bare keys would make game typing impossible (hotkey acts while the game is foreground). Show an inline message for rejection |
| Re-register at once, no restart; on failure keep the old one and report | Med | Register new before dropping old (or restore old on failure); the tray tooltip already reports conflicts, extend it with the current combo text |
| Persist in `ff-settings.json`; invalid/unusable -> default `Ctrl+Shift+Z` + warning | Low | Follows the existing key/warning pattern: "a wrong type reads the default with a warning". Define a stable text format (`Ctrl+Shift+Z`) and a parse test |
| Reset to default button | Low | |
| Display: human-readable combo text; shown in the tray tooltip and About/docs strings | Low | CLAUDE.md prose and any UI text mentioning `Ctrl+Shift+Z` becomes dynamic |
| Keep foreground-only behaviour | Low | Existing; unchanged |
| Layout independence | Med | Capture a physical key/virtual-key vs a character: non-US layouts matter. Pick what the existing registration uses (virtual key via a `Code`); egui `Key` is layout-resolved, so a mapping from egui::Key to the registration code is needed (the "possible hand-written table, no new dependency" item). Verify during planning |
Differentiators: allow two-modifier-only? no. Clear-to-disable the hotkey (tray-only use): Low-Med but a footgun, recommend NOT offering. Showing "in use by another app" on pick: registration failure is the only detection available (Windows gives no owner info).
Anti-features: multiple hotkeys; per-action hotkeys (Apply/Revert); chord sequences; accepting the Win key; hidden settings-file-only config.
Edge cases: modifiers order normalisation; numpad/extended keys; keys egui cannot report (media keys, some OEM keys) just are not capturable; PoE2's own bindings (the game consumes Ctrl+key combos only if registered globally first? global registration wins, so picking a key the game uses breaks it in-game: warn in tooltip); `ff` and game elevation unrelated; debug-vs-release instance both registering (existing note); the single-instance second-launch path unaffected; capture while window loses focus cancels.

## 7. Price range in economy rule names (999.27)

### Table stakes
| Feature | Complexity | Notes |
|---|---|---|
| `E tier Essences < 2 chaos`, top tier `S tier Currency >= 30 chaos` (`>=` ASCII is safer for filter files and fonts; Fontin may lack the glyph; recommend ASCII `>=`) | Low-Med | Emitter name format at `src/emitter.rs`; tiers' unit from tiers.json (chaos live, exalt fallback) |
| Short number format (0.5, 2, 12; no trailing zeros; thousands?) | Low | Define rounding for floats like 0.333, thousands separators for 1000+ (recommend plain digits) |
| Tier gaps/ordering: upper bound = next tier's min; tiers not in numeric order or equal mins | Low | Sort by min; equal mins degrade gracefully (no range or `< x` of next distinct) |
| Name change must not break stuff keyed to name | **Med-High** | CRITICAL: `# ff:name <T> tier <Group>` is the economy block identity (CLAUDE.md: every generated block carries it; `generate` fixtures `seed-universal.filter` compare bytes). Check what matches/relinks on that name (economy regeneration, search, `ff:group`, existing filters containing old names, expanded-row state keyed by name). Decide whether `generate` (CLI, frozen fixture) also changes or only the economy-section emitter does; if shared, fixture regeneration is a deliberate generator change; if the CLI must stay byte-identical, add the suffix only for the overlay/apply path |
| Old filters with old names: Apply regenerates the whole economy section, so rename is automatic | Low | Verify no "unapplied changes" noise: a name-only diff appears as a changed filter on the first Apply after upgrade; acceptable and expected |
Differentiators: unit conversion hint (exalt vs chaos) — skip. Re-pricing when tiers edited: names must update in the pending-economy read-only list ("re-priced when they change") — comes free if the emitter is used.
Anti-features: showing price per item; lower-bound for non-top tiers (author chose upper bound); localised formats.
Edge: price-tiered unique blocks (`A tier Uniques < 30 chaos`); Fallback pair is exalt-denominated; Divine as unit never used; collapsed row truncates the longer name (hover shows full; tooltip exists).

## 8. Network request throttling (999.20)

Comparable: update checkers (Chrome/Steam/VS Code check intervals), poe.ninja client etiquette (cache, respect limits), apps that serve cached results on "refresh" within a cooldown (Exiled Exchange 2 caches price data).

### Table stakes
| Feature | Complexity | Notes |
|---|---|---|
| Per-kind 1h throttle: Sync (per league), League list, Update check | Med | Key: `sync:<league slug>`, `leagues`, `update` |
| Failure does not start the hour | Low | Record timestamp only on success |
| Fake-refresh: same flow/outcome/status/animation as a real request, from last result | Med | For Sync: re-run the "sync finished" path from existing `prices.json` (no network); for leagues: reuse cached list; for update: reuse the last result (including a downloaded ready installer: do not re-download; if the download was removed on quit, throttled result must not claim "ready" without the file: re-verify or treat as expired) |
| Persist last-request times across restarts in `ff-settings.json` | Low-Med | Open point; recommend yes, otherwise restart defeats the throttle. Unknown keys are preserved on save, so a new key is safe; must follow "a write never touches an unusable file" rules. Clock changes: if stored time is in the future, treat as expired |
| Cached result needed after restart | Med | Sync: `prices.json` exists on disk (its timestamp/league = the cache); league list: needs persisted list or the fake refresh has nothing to show (decide: persist list in settings, or only throttle when in-memory/persisted cache exists, else go to network). Update check result across restart: only throttle if a cached "latest version" is stored |
| Never lie in the UI | Low | Status text stays identical to a real result (author wants identical), but Last sync time must not advance on a fake refresh |
| CLI not throttled, distinct User-Agent (e.g. `ferrisfilter-cli/<ver>` vs `ff-overlay/<ver>`) | Low | UA string set where HTTP clients are built; share one helper; test pinned |
| Tests never hit network/real settings | Low | Existing isolation guard (`FERRISFILTER_HOME`) applies; add a clock seam (like `UpdateHttp`) for deterministic hour tests |
Differentiators: a hover tooltip "Last synced 12 min ago" (exists partly: Last sync); manual force bypass (hidden Shift-click): tempting, but conflicts with the author's intent of fake refresh; do not add. Anti-features: background auto-refresh; throttling the CLI; throttling error retries; separate throttle for dropdown first-open fetch vs refresh (same kind, same hour).
Edge: league switch mid-hour (separate key per league); `--league` ended league; Fallback price source (Sync greyed, unaffected); sync running concurrently; the hour measured from request start or finish (use success time); settings file unusable (cannot persist: throttle in memory only, no warning spam).

---

## Feature dependencies

```
Style alpha -> Auto Detect (parser compares alpha; effective-default rules for background 240 / others 255)
Auto Detect -> existing action-family/inexpressible detection, custom style add, tag write
999.26 DnD  -> constraints from arrows (groups, Continue, sections, economy lock) ; touches 999.23 headers (arrows/handle on the right)
999.23 headers + 999.24/25 icons -> shared ff-art manifest/texture work (batch art extraction in one phase)
999.19 hotkey -> Config tab control + settings key + hotkey.rs re-register
999.27 names -> emitter shared with generate/fixtures; economy rule identity
999.20 throttle -> settings key + cached results (prices.json, leagues list, update result)
```

## MVP recommendation (ordering suggestion)

1. Style alpha (foundation, isolated, byte-identity tests)
2. Auto Detect (depends on 1; riskiest design: split Revert across tabs)
3. Economy name price ranges (small, but verify name-keyed behaviour and fixture impact)
4. Art/skin batch: section headers + search X + magnifier (one ff-art pass)
5. Drag-and-drop (largest UI engineering; after headers so handle art/layout is final; custom styles table too)
6. Hotkey customisation
7. Request throttling (independent; can go anywhere, low UI)

Defer: keyboard reordering, draggable section headers, drop-onto-collapsed-section, hotkey disable option.

## Research flags
- Auto Detect: deeper design pass on Revert semantics and Hide-bare/default-alpha equality.
- DnD: prototype slot math with virtualization + auto-scroll in egui 0.36 before planning (check the egui 0.36 drag API, not verified here).
- Hotkey: verify the registration crate/type in `hotkey.rs` and egui::Key coverage (possible dependency flag if mapping is impractical).
- Economy names: grep for consumers of `# ff:name` in economy blocks.

## Sources
- `CLAUDE.md`, `.planning/ROADMAP.md` (999.19-999.27), `.planning/research/ui-refs/` (read, HIGH for FerrisFilter behaviour)
- Comparable-app behaviour: general UI convention knowledge, not freshly verified (LOW-MEDIUM)
