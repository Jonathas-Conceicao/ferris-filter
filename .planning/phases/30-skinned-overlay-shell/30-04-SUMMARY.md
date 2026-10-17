---
phase: 30-skinned-overlay-shell
plan: 04
subsystem: ui
tags: [egui, overlay, icons, tooltips, less-texty]
requires: [30-01, 30-02, 30-03]
provides:
  - Collapsed-row flags as painted icons (Flag/FlagKind) with tooltips, errors nearest the arrows
  - Standing CONFIG / rule-editor / RULES-empty-state captions moved into tooltips
  - Styles table Visibility column no longer overlaps Name
affects: [30-05]
tech-stack:
  added: []
  patterns: [flag list built in paint order, badge painted last, tooltip-on-control instead of caption]
key-files:
  created: []
  modified: [src/overlay/rule_list.rs, src/overlay/config_view.rs, src/overlay/body.rs, src/overlay/app.rs, src/overlay/apply.rs]
decisions:
  - "The styled flag's gold diamond is a vector shape (convex polygon), not the slider_thumb texture, so show_list needs no Skin parameter"
  - "flags_for returns flags already in paint order (right to left); the +N badge is painted last (far left) and folds only the least important flags"
  - "GROUP_BROKEN_LONG const removed (no remaining user; the flag tooltip is the shortened text). CANNOT_ENABLE_LONG kept as the tooltip of the expanded-body Can't enable line"
metrics:
  tasks: 2
  completed: 2026-10-16
---

# Phase 30 Plan 04: Less-texty pass Summary

Row flags are now small painted icons with tooltips, standing explanatory captions are tooltips on the control they explain, the RULES empty states are a heading only, and the CONFIG Styles table header no longer overlaps. Flag conditions, the grammar-error gate on APPLY and all long texts are unchanged (the texts are kept verbatim and reachable on hover).

## Commits
- 95d411c feat(30-04): row flags become painted icons with tooltips
- 108c4b9 feat(30-04): captions become tooltips, short empty states, APPLY tooltip wording

## Less-texty changes (this plan)

All "old text" below was standing text; "new" says where it can now be found. Source of truth for 30-05's review checklist.

### Captions that became tooltips

| # | File | Old standing text | Where it is now |
|---|------|-------------------|-----------------|
| 1 | config_view.rs (FILTER) | "Saved on this computer. Default: dynamic.filter." | Second line of the tooltip on the game-folder filter dropdown (first line = full filter path; with no game folder it is the only line) |
| 2 | config_view.rs (FILTER) | `RELOAD_TOGGLE_CAPTION` paragraph under the toggle ("Sends /reloaditemfilter ... instead of the command.") | Tooltip on the "Reload filter in game after Apply" toggle AND on its label: caption, blank line, then `RELOAD_TOGGLE_TIP` (both consts unchanged) |
| 3 | config_view.rs (new-filter draft) | helper line with the empty-draft hint (name the new filter file) | Tooltip on the draft text field, shown only while the draft is empty. A hint for a non-empty invalid draft stays inline (`body::hint`) |
| 3b | config_view.rs (new-style draft) | helper line with the style-name hint (always shown) | Same rule applied: tooltip on the "Style name" field while empty; non-empty invalid draft keeps the inline `body::hint` |
| 4 | config_view.rs (Styles, empty) | "No styles loaded. Revert to reload styles.json." | Inline `No styles loaded` (warning colour); "Revert to reload styles.json." is its tooltip |
| 5 | config_view.rs (custom sound) | caption "The file must be in the game folder." under the File field | Tooltip on the custom-sound File text field |
| 6 | app.rs (RULES, empty filter) | paragraph "Press + ADD RULE ... saved with the filter when you press APPLY." | Heading `No rules yet` only; the paragraph is the `+ Add rule` tooltip (`ADD_RULE_TIP`, already there from 30-02) |
| 7 | app.rs (RULES, no match) | "Check the spelling, or press Clear to see every rule." | Removed; heading `No rules match "<query>"` only |
| 8 | app.rs (RULES, load failure) | the error message paragraph plus "Fix the file and press Reload." | Heading `The filter could not be loaded.` only; the detail is in the status line (priority 4, from 30-02). `centered_notice` lost its body parameter |
| 9 | body.rs (expanded rule, Conditions) | "No conditions: this rule matches every item." | Short label `No conditions`; the sentence is its tooltip (`NO_CONDITIONS` unchanged) |
| 10 | body.rs (expanded rule, Actions) | "No actions: matching items keep the game's look." | Short label `No actions`; the sentence is its tooltip (`NO_ACTIONS` unchanged) |
| 11 | rule_list.rs (expanded rule, disabled and invalid) | standing `CANNOT_ENABLE_LONG` paragraph | Short `Can't enable` line (warning colour); the long text is its tooltip |
| 12 | apply.rs | APPLY blocked tooltip: `Fix or disable the rules marked "grammar error" first.` | `Fix or disable the rules marked with the red ! first.` (names what the user now sees) |

Left as inline text on purpose (UI-SPEC "Retained"): field-validation messages, `NOT_SET`, short field captions, and the two tier-section helper lines (`Minimum price for each tier...` and `Only your live tiers.json is edited...`), which the plan did not list.

### Flag to icon mapping (collapsed RULES rows)

Icons are painted vectors from `skin::icons` (14pt glyph in an 18x18 hover rect; accessible name = old flag word). Listed in paint order, right to left: the first one sits next to the reorder arrows.

| Order | Old flag text | Icon | Colour | Tooltip |
|-------|---------------|------|--------|---------|
| 1 | `grammar error` | red `!` (GrammarError) | error red | the diagnostics, one `Line N: message` per line (unchanged) |
| 2 | `grammar warning` | warning triangle (GrammarWarning) | warn | the diagnostics (unchanged) |
| 3 | `unknown style` | `?` (UnknownStyle) | warn | "No style with this name exists. This rule keeps its current actions, and its style link is removed on the next Apply." (unchanged) |
| 4 | `group broken` | broken link (GroupBroken) | warn | "Group broken: these blocks were one rule (# ff:group) but no longer match, so each is shown on its own. Left as written until you edit it." (shortened; the old long text is gone) |
| 5 | `moved across Continue` | move arrows (MovedAcrossContinue) | warn | "This move changes which rules still apply after a Continue. Check the order." (unchanged) |
| 6 | `Continue` | stays a word, small caps | gold | "Matching goes on to the rules below" (unchanged) |
| 7 | `styled: <name>` | gold diamond plus the style name, now 10 characters + ellipsis (was 12) | gold diamond, muted name | `Style "<full name>" sets this rule's actions on Apply` (unchanged) |
| alone | `can't enable` (disabled and cannot enable) | padlock (Padlock) | warn | "Can't enable: these lines are not valid filter grammar. You can still move or delete it." (shortened from the long text) |

Layout notes: `FLAG_BUDGET` 120 and `LABEL_MIN_W` 140 are unchanged; each icon counts 18pt, so up to five (six when the badge is not needed) fit. When flags do not fit, the `+N` badge (muted, tooltip = hidden flags `"<name>: <tip>"` joined by newlines) now sits at the far left of the flag group and folds the last (least important) flags, so a grammar error is never folded away.

### Removed outright
- The no-match and load-failure explanatory paragraphs (items 7, 8) are gone from the RULES pane (the load-failure detail remains in the status line).
- Flag words (`group broken`, `can't enable`, `unknown style`, `moved across Continue`, `grammar error`, `grammar warning`) no longer appear in collapsed rows (only as icon accessible names and in the `+N` tooltip).
- The `GROUP_BROKEN_LONG` const (its text lives only in the shortened tooltip).

## Deviations from Plan

**1. [Carried-over bug, requested] CONFIG Styles "Visibility" caption overlapped "Name".** Cause: the Visibility column was 48pt wide, but the caption in the skin font is wider. Fix: `COL_VISIBILITY` 48 to 80pt (header and rows share the constant, so alignment is kept); new test `styles_header_captions_fit_their_columns` asserts every header caption fits its column. Verified with a debug screenshot (`ff --screenshot ... --tab config`, temp home): Visibility, Name and Preview read separately in both the Built-in and Custom tables. Commit 108c4b9.

**2. [Rule 3 - Blocking] Overlay test contexts now install the real skin fonts.** The new `regular(15)` / `small_caps(17)` text in rows and bodies panics on the empty font set the tests used. `headless_ctx` in rule_list.rs, body.rs and app.rs now call `skin::fonts::install`. This shifted layout, so three pixel-scan tests in body.rs were re-aimed (class header y range 160..188 to 188..212; the operator-dropdown scan now tries every popup-opening click from y 164 until one offers "Not", with a wider item search). Side effect: the full `cargo test` run takes about 17 minutes in the debug profile (lib tests alone ~1000 s) because of per-context font-atlas builds; flagging it for a later test-speed pass.

**3. Styled diamond is a vector shape, not `skin.slider_thumb`.** The plan allowed the texture, but that would have required threading `&Skin` through `show_list` and its many callers; a 14pt tinted diamond is visually equivalent.

**4. Item 3 applied to the new-style draft as well** (the plan allowed "other `helper(ui, msg)` sites only if they are empty-draft hints").

## Known Stubs
None.

## Verification
- Full `cargo test` green (1131 unit tests plus integration suites); `tests/fixtures` untouched; `cargo fmt --all --check` clean.
- Clippy: below the baseline of 126 (lib 76 + lib-test-only 43 + 3 in integration tests).
- Acceptance greps: no `helper(ui, RELOAD_TOGGLE_CAPTION)`, no "Fix the file and press", `No rules yet` once, `marked with the red ! first` once, `RELOAD_TOGGLE_TIP` six times.
- Screenshots (temp home, not committed): config tab shows the fixed table header and the filter section with no caption lines; rules tab shows the heading-only empty state. Icon rows were not screenshotted (the sample home has no rules); they are covered by tests (kinds, order, truncation, tips, `fit_flags`) and left for 30-05's manual check.

## Threat Flags
None. Presentation only; no file, settings, network or keystroke code changed (T-30-15..17 as planned).

## Self-Check: PASSED
