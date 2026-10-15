# Phase 25: UAT Feedback Fixes 2 - Context

**Gathered:** 2026-10-14
**Status:** Ready for planning
**Mode:** Locked decisions from the author's second hands-on UAT round of v0.5 (list approved by the
author); run autonomously, no questions. No UI-SPEC: the decisions below are the spec.

<domain>
## Phase Boundary

Ten overlay changes (requirements UAT2-01..UAT2-10): rule-body layout tweaks, rule names without a
toggle, Sockets as a plain slider, `Exceptional` out of the keyword table, a tidier CONFIG styles
table with custom-style reordering, and **one all-or-nothing Apply** that saves and applies CONFIG
and RULES edits together.

The author dislikes "texty" UI: keep text minimal, tooltips over captions. Backlog 999.11 covers a
later full UI review; do not redesign beyond this list.

Out of scope: generator changes (`generate`/`artifact_guard`/`generate_snapshot` fixtures stay
byte-identical), the CLI, style rename, a broader UI review.

</domain>

<decisions>
## Implementation Decisions (all locked)

### Rule editor

**UAT2-01: Continue on its own last line.** "Continue to later rules" moves out of the style
section's row to its own line after the style section: the last thing in the rule body (before the
Duplicate/Delete footer controls, which are not "body content"). Same toggle behaviour as today.

**UAT2-02: "+ style".** In the Custom style, the button that adds action lines (colours, font,
sounds, minimap, beam) reads **+ style** instead of **+ action** (body.rs ~1746). The picker it opens
is unchanged. Update any test or doc that names the old label.

**UAT2-03: Show/Hide only for Custom.** The Show/Hide control sits on its own line **below** the
style dropdown and is drawn only when **Custom** is selected. With a preset selected the style
decides Show/Hide (the preset's `action` is applied by the existing style apply/rewrite) and the
control is hidden. A foreign `Minimal` block keeps its non-clickable Minimal label (UAT-04 rule)
where the control would be, only in Custom.

**UAT2-04: rule name is a plain text box.**
- Remove the name on/off circular toggle (rule_list.rs ~563, `name_open`) and the derived default
  name for unnamed rules. The row header for an unnamed rule shows the index and the Show/Hide
  keyword only (rules.rs ~444 `derived_pieces` is dropped from the label; it may be deleted if
  unused). A named rule keeps `<index>  <kw> - <name>`. The collapsed summary line (Phase 24
  UAT-13) gives the context.
- The search haystack must still find rules by class/base type (it already adds those separately).
- The body shows a single text box: empty = no name and **no `# ff:name` line**; clearing the box
  removes the line; typing adds/updates it. Keep the existing draft mechanism so clearing the box
  does not collapse/lose focus mid-typing (`name_drafts`). Edits touch only the `# ff:name` line.
- A hint text inside the empty box (e.g. "name") is fine; no caption.

**UAT2-05: Sockets is a plain numeric slider, 0-6.** Drop the "+ socket" button (body.rs ~1389) and
the socket-colour editing entirely. Sockets renders like any numeric condition (operator + slider
0-6, `grammar::slider_range("Sockets")` already `(0, 6)`).
- Lossless: keep `ValueShape::Sockets` in the parser so a foreign `Sockets >= 5GGG` line still
  parses without a grammar error and round-trips byte-identically. If a line carries colour
  letters, the slider edits only the count and the letters are kept verbatim (never added or
  normalized by the UI); an untouched line is never rewritten. The UI never offers colour letters.

**UAT2-06: remove `Exceptional`.** Remove it from `src/filter/keywords.rs` `KEYWORDS` and from
grammar.rs groups/defaults/display, same treatment as Phase 24's PoE1-only removals: a foreign
`Exceptional True` line still loads losslessly as an unknown-keyword raw row (parse warning, not
error) and does not block Apply. Update keyword-table tests (`documented_vocabulary_present`, the
`["MirageMap", "Exceptional", "Minimal"]` loop at keywords.rs ~229, counts). Add a test like the
Phase 24 `gate_ignores_removed_poe1_keyword` / `removed_poe1_keyword_is_a_raw_row` for Exceptional.
Leave unrelated "Exceptional" strings (models/config.rs EXC tests, `# ff:name Exceptional 3 socks`
fixtures) alone.

### CONFIG tab

**UAT2-07:** Remove the "Config: no unapplied changes" / "Config: N unapplied changes" status line
at the top of the CONFIG panel (config_view.rs ~1108, `dirty_status`; delete the helper if unused
and adjust tests), and the "new" badge on styles (config_view.rs ~611). The "edited" indication may
stay only if it is not a text tag either; prefer dropping text tags entirely in favour of the shared
Apply state.

**UAT2-08:** The **Add style** button sits on the **Custom** group heading line (right of the
heading), not at the bottom of the list. The Custom heading is shown even when there are no custom
styles yet, so Add style is always reachable.

**UAT2-09: styles table.**
- A header row with columns in this order: **Visibility | Name | Preview** (Visibility = the
  Phase 24 Show/Hide slide switch; Name; Preview = the existing style preview swatch/label).
  Column widths aligned across Built-in and Custom groups.
- Custom styles get **up/down reorder arrows** like rules (same arrow widgets as the rule list).
  Order = order in styles.json; a reorder is an in-memory CONFIG edit (dirty, Revert-able) that is
  persisted on Apply. Arrows are disabled at the ends of the Custom group; a custom style can never
  move above the built-ins.
- Built-in styles (whatever `StyleSet::is_builtin` says; do not invent a new definition): the
  visibility switch and editing their actions are allowed; they cannot be renamed, moved or deleted
  (no arrows, no delete, no rename field). Built-ins stay first, customs after the divider.
- Reordering must not change any untouched style's serialized bytes beyond the moved entries'
  position (styles.json write keeps existing formatting conventions).

### One Apply for everything (UAT2-10)

- **APPLY on either tab** saves and applies everything pending: CONFIG edits -> styles.json +
  tiers.json; RULES edits + regenerated economy + style rewrites -> the filter; then the in-game
  `/reloaditemfilter` under the existing Phase 23 rules (setting on, game running, stripe window
  owner, etc.; game-not-running is silent per UAT-01).
- **Validate everything first, then write.** Validation covers: invalid tier cutoffs / unparsable
  style edits (today's `ConfigApplyError::Invalid`), enabled rules with grammar errors
  (`DynamicError::InvalidBlocks`), damaged economy markers, economy pricing failure, unreadable
  styles for style rewrites, and the changed-on-disk guard of **all three files**. Any refusal ->
  nothing written, nothing reloaded, refusal shown.
- **Render with the new config.** The filter text is rendered from the in-memory (pending) styles
  and tiers, not the on-disk ones: style rewrites use the pending StyleSet, and the economy section
  is priced with the pending tier cutoffs. This needs an `economy_body` variant (or parameter) that
  takes in-memory tiers/styles instead of reading `InputPaths` files for those two.
- **Write with the all-or-none machinery.** `safe_write::write_all_or_none` already takes N
  targets (guard all, stage all, persist in order, roll back from in-memory loaded bytes). Use it
  for up to three targets: styles.json, tiers.json, the filter. Unchanged targets drop out (already
  handled). Rollback on a later rename failure is required and must be tested with three targets
  (the existing injectable `write_all_or_none_with` persist hook).
- **The filter's one-time `.ffbak`** (today made by `write_guarded` on the first write of a foreign
  filter) is preserved: create it before the all-or-none write if it does not exist yet (it is a
  copy of the loaded bytes, so it never needs rollback). Claude's discretion on exact placement.
- **No filter loaded** (no filter selected, load failed, absent with nothing to write): APPLY
  applies the CONFIG part alone (there is nothing to render); no reload. This keeps CONFIG usable
  without a filter. A load-failed filter is not a refusal of the config part.
- **Nothing pending on one side** is fine: config-only edits still re-render the filter (that is the
  point: styles/tiers changes reach the filter), rules-only edits write only the filter.
  Identical bytes are not rewritten but still count as applied (today's rule; reload still runs).
- **Changed on disk:** if any of the three files changed since load, the refusal names it and the
  overlay offers **OVERWRITE** and **RELOAD** on both tabs.
  - OVERWRITE applies to everything: rebase every changed file's guard baseline to its current disk
    bytes (on copies adopted only on success), then the same all-or-none write of the in-memory
    state. A failed overwrite leaves the old baselines and the refusal in place.
  - RELOAD reloads from disk: re-read every file that changed on disk, discarding the in-memory
    edits of the tab that owns it (filter -> RULES edits; styles.json/tiers.json -> CONFIG edits);
    the other tab's edits survive. (Claude's discretion: if CONFIG state can only reload styles and
    tiers together, reload both.)
- **Revert stays per tab:** RULES Revert discards rule edits (and staged economy ops) only; CONFIG
  Revert discards config edits only.
- **Stale indicators:** both tabs' Apply button/state reflect one combined "unapplied" state: dirty
  if RULES has edits **or** CONFIG has edits (or a staged economy op). After a successful Apply both
  clear. The "Filter out of date: config changed. Press Apply on the RULES tab..." notice
  (`config_state::OUT_OF_DATE_MESSAGE`, `config_changed_pending`, `out_of_date` in
  `ConfigViewCtx`) is removed: after a combined Apply it no longer applies.
- **One status message** for the Apply result, visible on whichever tab is shown (e.g. a shared
  apply message; the CONFIG footer and RULES footer show the same text). Keep the success text
  short: what was written, then reload outcome (existing messages).
- The CONFIG "outside edit" guard UI (`OutsideEdit`, config_outside) is folded into the shared
  changed-on-disk refusal so there is exactly one OVERWRITE/RELOAD flow.
- After a successful Apply the RULES working document picks up the applied styles (`set_styles`)
  and CONFIG's baselines move to the written bytes, exactly as the separate applies did.

### General
- Edits touch only the edited lines; never normalize untouched operators (PARSE-04).
- Minimal text; tooltips over captions.
- Tests must never write to the real game folder or the repo's committed `styles.json`/`tiers.json`
  (use temp roots); never send real keystrokes (use the existing fake sinks).

### Claude's Discretion
- Plan split and wave layout; exact widget code for the table header and arrows; placeholder hint
  in the name box; how the shared apply message is stored; whether `derived_pieces` is deleted;
  `.ffbak` placement; RELOAD granularity detail noted above.

</decisions>

<code_context>
## Existing Code Insights

- `src/safe_write.rs`: `write_all_or_none(&[Target])` (N targets, guard/stage/persist/rollback,
  `AllOrNoneError::{NothingWritten, RolledBack, RollbackFailed}`, `changed_on_disk_path`),
  injectable `write_all_or_none_with` for tests; `write_guarded` (single file, `.ffbak`).
- `src/overlay/config_state.rs`: `ConfigState::apply/apply_inner/overwrite` validate and write
  styles.json+tiers.json through `write_all_or_none`; `ConfigApplyError`, `OUT_OF_DATE_MESSAGE`,
  `dirty_status`, built-in flag from `StyleSet::is_builtin`.
- `src/overlay/app.rs`: `apply_working` (style rewrite via `working.text_with_styles()`, economy plan,
  `dynamic::economy_body(&InputPaths, ..)` reading tiers/prices from disk, `dynamic::apply_edited`),
  `overwrite_working`, `run_apply` (reload decision via `chat::decide`, `apply_state`,
  `config_changed_pending`), `config_apply`, `revert`, `reload`, `config_body` (`ConfigViewCtx`).
- `src/dynamic.rs`: `apply_edited`, `economy_body`, `LoadedFilter` (`rebase_to_disk`, guard bytes).
- `src/overlay/apply.rs`: `ApplyState`, messages (`CHANGED_ON_DISK_MESSAGE`, RELOAD/OVERWRITE tips,
  "Filter out of date" ~261).
- `src/overlay/body.rs`: `style_section` (~1730), "+ action" (~1746), "Continue to later rules"
  (~1771), DISPOSITION_CHOICES, Sockets editor and "+ socket" (~1314-1389).
- `src/overlay/rule_list.rs`: name toggle (~563, `name_open`, `name_drafts`), up/down arrows for rules.
- `src/overlay/rules.rs`: row label (~444) and `derived_pieces` (~354).
- `src/overlay/config_view.rs`: style groups (~557), `style_row` (~581), "new" badge (~611),
  Add style (~709), dirty status line (~1108).
- `src/filter/keywords.rs`: `Exceptional` (~87) and its tests; `src/overlay/grammar.rs` (~73).
- `src/styled.rs`: StyleSet, style rewrite.

</code_context>

<specifics>
## Specific Ideas

- Tests: Continue is the last body section; "+ style" label; Show/Hide hidden for presets, shown for
  Custom; empty name writes/removes no `# ff:name` line, typing adds it; unnamed header has no
  derived pieces; Sockets slider edits only the count and keeps colour letters; Exceptional raw row
  + Apply not blocked; no dirty status line; custom reorder persisted in styles.json order and
  built-ins immovable; combined Apply: (a) config-only edit re-renders the filter with the pending
  tiers/styles, (b) a refusal of any part writes none of the three files, (c) rollback with three
  targets when the third rename fails, (d) changed-on-disk on any file -> OVERWRITE writes all,
  (e) no-filter Apply writes config only, (f) per-tab Revert independence.
- Build/test with `CARGO_TARGET_DIR=C:/Users/jonat/AppData/Local/Temp/ff-target` (if parallel
  worktrees contend, use a private target dir, and `touch src/lib.rs` to force a rebuild).
- Docs: CLAUDE.md overlay notes + CHANGELOG `[Unreleased]`; `25-MANUAL-VERIFICATION.md` (one line
  per item).

</specifics>

<deferred>
## Deferred Ideas

- Full UI review (backlog 999.11).
- Style rename with tag rewriting.

</deferred>
