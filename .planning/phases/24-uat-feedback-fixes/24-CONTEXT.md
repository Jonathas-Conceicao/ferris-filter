# Phase 24: UAT Feedback Fixes - Context

**Gathered:** 2026-10-13
**Status:** Ready for planning
**Mode:** Locked decisions from the author's hands-on UAT of v0.5 (list approved by the author);
run autonomously. No UI-SPEC: the decisions below are the spec.

<domain>
## Phase Boundary

Thirteen overlay fixes from the author's UAT of milestone v0.5 (requirements UAT-01..UAT-13).
The author dislikes "texty" UI: keep text minimal and prefer tooltips. Backlog 999.11 covers a later
full UI review, so do not redesign anything beyond this list.

Out of scope: generator changes (`generate`/`artifact_guard`/`generate_snapshot` fixtures stay
byte-identical), a broader UI review, changing the other reload-skip messages.

</domain>

<decisions>
## Implementation Decisions (all locked)

### UAT-01: no message when the game is not running
- After a RULES Apply where the in-game reload was skipped because **the game is not running**,
  show **no reload message at all** (no Refresh reminder, no reason). The game reads the filter when
  it starts. `SkipReason::GameNotRunning` (src/overlay/chat.rs) → nothing appended to the Apply
  status.
- The other skip reasons (setting off, send failed, modifier held, focus failed) keep their current
  message.

### UAT-02: CONFIG → Styles in two groups
- Built-in styles first, then the user's custom styles, each group under its own heading with a
  proper divider between them. Remove the per-row "built-in" badge (`config_view.rs`, `BUILTIN_TIP`
  badge). Only custom styles have a remove control.
- "Built-in" means whatever `config_state`/`styled` already treats as built-in (do not invent a new
  definition).

### UAT-03: Show/Hide as an on/off slide switch per style
- Each style's Show/Hide becomes a modern on/off slide switch (on = Show, off = Hide). Purpose: hide
  a whole group of styled rules (e.g. low economy tiers) at once. Takes effect on the next RULES
  Apply through the existing style rewrite (`# ff:style` tags) + economy regeneration. No new
  mechanism. A short tooltip may explain; no caption text.

### UAT-04: drop Minimal from the editor
- The style config action control and the per-rule action control (body.rs segmented
  Show/Hide/Minimal selector) offer **Show and Hide only**.
- A `Minimal` block in a foreign filter is still parsed, still displayed as Minimal, and left
  untouched (no rewrite unless the user changes it). Nothing in the UI lets the user set Minimal.
  `Minimal` stays in the keyword table (it is a real block header).
- If a style JSON somehow carries `Minimal`, keep loading it (don't fail); the UI just never offers
  it.

### UAT-05: condition ✕ at the right edge
- The ✕ that removes a condition is right-aligned in its row so the ✕ buttons line up across rows
  and across rules.

### UAT-06: vertical list for quoted string lists
- BaseType and other quoted-list conditions return to the v0.4 vertical list: one entry per line with
  its own Remove, plus the type + Enter / **+ ADD** field and quoted-paste splitting
  (`text_entry::parse_base_type_entry`). Replaces the current variable-width chips.
- The list sits in an inner `ScrollArea` with a capped height (about 6–8 rows) so a long list does
  not take over the stripe.
- Edits touch only that line (PARSE-04); the list writes back quoted values.

### UAT-07: Class condition without the chip list
- Remove the chip list of selected classes. Keep only the grouped class picker with its
  sub-selections (groups come from config.json; never hardcoded in the GUI). Unknown (non-catalogue)
  classes already selected must still be preserved on write-back; if they are no longer visible,
  they must not be silently dropped (Claude's discretion on how to surface them minimally, e.g. a
  compact line or tooltip).

### UAT-08: styled rule caption removed
- Remove the caption text under the style picker that explains where the style comes from. Keep the
  unknown-style warning behaviour functional (a tooltip is fine).

### UAT-09: remove PoE1-only conditions
- The official docs page (pathofexile.com/item-filter/about, fetched 2026-10-13) does **not** mark
  any condition PoE1-only (it only marks six as "PoE2-only": AlwaysShow, HasVaalUniqueMod,
  IsVaalUnique, TwiceCorrupted, UnidentifiedItemTier, WaystoneTier). So the list is decided by
  mechanic: a condition is removed when the mechanic it filters on does not exist in PoE2.
- **Final removal set (25):** LinkedSockets, SocketGroup, ShaperItem, ElderItem, SynthesisedItem,
  HasInfluence, ElderMap, ShapedMap, BlightedMap, UberBlightedMap, MapTier, Scourged, Replica,
  HasSearingExarchImplicit, HasEaterOfWorldsImplicit, EnchantmentPassiveNode,
  EnchantmentPassiveNum, MemoryStrands (author-approved) plus AlternateQuality, GemQualityType,
  TransfiguredGem (PoE1 gem-quality/transfigured-gem mechanics), ArchnemesisMod (PoE1 Archnemesis
  league), HasCruciblePassiveTree (PoE1 Crucible), ZanaMemory (PoE1 memories), BaseWard (Ward is a
  PoE1 defence).
- **Kept (not clearly PoE1-only):** Exceptional, Foulborn, Imbued, MirageMap, Vestigial,
  CorruptedMods, BaseDefencePercentile, HasEnchantment, AnyEnchantment, HasImplicitMod,
  FracturedItem, and all six PoE2-only ones.
- Remove them from `src/filter/keywords.rs` `KEYWORDS` (also drop the now-unused `INFLUENCES` and
  `GEM_QUALITY` word lists), from grammar.rs groups/defaults, and update the keyword-table tests
  (`documented_vocabulary_present` count, etc.).
- A foreign filter line using a removed keyword must still load losslessly (byte-identical round
  trip) and show as the existing unknown-keyword raw row. The parser already emits an
  "unknown keyword `X` kept as-is" **warning** (parse.rs) and keeps the line opaque.
- **Apply must keep working** for filters containing such lines: verify how the grammar gate
  (app.rs grammar_gate / grammar validation) treats unknown-keyword lines; they must not count as
  errors that block Apply. Add a test: a block with an enabled `LinkedSockets 6` line round-trips
  and does not block Apply. Report what was found/done.

### UAT-10: slider defaults inside slider ranges
- GemLevel defaults to 65 but its slider is 1–21 (grammar.rs ~319/331). Fix it (e.g. default 20).
- Review every numeric condition and action so each default lies inside its slider range, and add a
  test that iterates the whole keyword table asserting `min <= default <= max` for every numeric
  keyword that has a slider.

### UAT-11: style section reworked and moved to the end of the rule body
- The style section moves to the **end** of the expanded rule body.
- One picker listing **Custom** plus every preset style from styles.json, with a Show/Hide control
  beside it (Show/Hide only; see UAT-04).
- **Custom** → the action rows (colours, font, sounds, minimap, beam, etc.) are shown and edited
  right there, under the picker.
- **A preset** → the action rows are hidden; the rule uses that style (`# ff:style` tag; rewritten
  on Apply as today).
- Switching preset → Custom keeps the currently rendered actions as the starting point and drops the
  tag. Switching Custom → preset sets the tag and applies the preset's actions (as today's style
  apply does).

### UAT-12: compact Yes/No boolean conditions
- Boolean conditions (`ValueShape::Bool`: Corrupted, Identified, Mirrored, AnyEnchantment, …)
  render as a single compact `Corrupted  [Yes | No]` control. Yes writes `Corrupted True`, No writes
  `Corrupted False`. No operator dropdown, no on/off toggle; the user removes the condition with ✕
  to not care.
- Untouched lines keep their existing operator/text verbatim (PARSE-04): a line like
  `Corrupted == True` is not rewritten unless the user clicks the other value. Claude's discretion
  whether a click that changes the value rewrites to the canonical `Corrupted False` or keeps the
  operator; prefer the minimal edit that keeps any existing operator.
- A `!=`/`!` operator on a boolean line: display the effective value (e.g. `Corrupted != True` reads
  as No) or fall back to the raw row — Claude's discretion, but never lose the text.

### UAT-13: collapsed-row summary line
- Collapsed rules show a short summary line under the header again (like v0.4's), describing the
  conditions (rules.rs already builds a condition summary around line 300). It hides when the row is
  expanded.
- Keep row virtualization correct (`virtual_list.rs` row heights must account for the extra line)
  and performance acceptable at ~460 blocks (summary text computed/cached, not rebuilt per frame per
  block if that is costly).

### General
- Edits touch only the edited lines; never normalize untouched operators.
- Minimal text; tooltips over captions.

### Claude's Discretion
- Exact widget code for the slide switch (custom painted toggle in widgets.rs is fine), headings
  wording for style groups ("Built-in" / "Custom"), inner list height (6–8 rows), summary truncation
  length, boolean value rewrite form, plan split and wave layout.

</decisions>

<code_context>
## Existing Code Insights

- `src/filter/keywords.rs`: the single keyword table (80 entries); `lookup` returning `None` = unknown
  keyword kept opaque.
- `src/filter/parse.rs`: unknown keyword → warning diagnostic + `Item::Opaque`.
- `src/overlay/grammar.rs`: condition groups (asserts every group name is a known keyword — must be
  updated when keywords are removed), slider ranges (~line 319) and defaults (~line 331).
- `src/overlay/body.rs`: rule body editors — raw rows for unknown keywords (~225), class card
  (~1170 chip list), alert sounds, action selector Show/Hide/Minimal (~1552), style section with
  captions (`STYLE_UNKNOWN_EXPLAIN` ~2098).
- `src/overlay/rules.rs`: row model, `BlockKeyword` labels, condition summary builder (~300).
- `src/overlay/rule_list.rs` + `virtual_list.rs`: the virtualized list and row heights.
- `src/overlay/config_view.rs`: CONFIG Styles (built-in badge ~580), `config_state.rs` built-in test.
- `src/overlay/text_entry.rs`: `parse_base_type_entry` (quoted-paste splitting).
- `src/overlay/chat.rs` `SkipReason::GameNotRunning`; `src/overlay/apply.rs` reason strings and
  `RELOAD_INSTRUCTION`; `app.rs` `run_apply` composes the status.
- `src/overlay/widgets.rs`: shared widgets (badge, toggles) — home for a slide switch.

</code_context>

<specifics>
## Specific Ideas

- Tests: slider-default table test (UAT-10); removed-keyword round-trip + Apply-not-blocked test
  (UAT-09); boolean Yes/No write tests incl. untouched operator preservation (UAT-12); no message on
  GameNotRunning (UAT-01); style switch writes Hide/Show (UAT-03); preset→Custom keeps actions and
  drops tag (UAT-11); Minimal block untouched (UAT-04); summary row height accounted (UAT-13).
- Build/test with `CARGO_TARGET_DIR=C:/Users/jonat/AppData/Local/Temp/ff-target`. Never write to
  the real game folder, never send real keystrokes, don't launch or kill ff.
- Docs: CLAUDE.md overlay notes + CHANGELOG `[Unreleased]`; `24-MANUAL-VERIFICATION.md` (one line
  per item).

</specifics>

<deferred>
## Deferred Ideas

- Full UI review (backlog 999.11).
- Removing the other reload-skip messages (setting off, send failed, modifier held, focus failed).

</deferred>
