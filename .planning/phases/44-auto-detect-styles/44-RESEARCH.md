# Phase 44: Auto Detect Styles - Research

**Researched:** 2026-10-23
**Domain:** Rust/egui overlay, in-memory pending edits across the CONFIG model (`ConfigState`) and the RULES model (`WorkingFilter`). No new dependencies, no network.
**Confidence:** HIGH (everything below was read in this repo this session; the few items I could not confirm by reading are marked and listed in Open Questions)

## User Constraints (from CONTEXT.md)

### Locked Decisions
- Every rule **outside the economy section** is read: its action lines plus Show / Hide are parsed into the full `Style` struct and compared on **every parameter**. A font size off by 3 is a different style.
- Matching order: built-ins first with their **current pending values**, then existing custom styles, then new styles.
- A bare `Show` (no action lines) matches `unchanged`. A bare `Hide` matches no built-in (`hidden` also sets font 21 and DisableDropSound); it becomes a custom Hide style like any other distinct style.
- Equality is on **in-game values**: 3-value `SetBackgroundColor` = alpha **240**, 3-value text/border = **255**; missing volume = game default (`PlayAlertSound` 50, `CustomAlertSound` 100).
- Apply rewrites a tagged rule from its style as it does today (may normalise form). Accepted.
- Stays Custom: repeated lines of one action family, `CustomAlertSoundOptional`, `...IfAlertSound` keywords, out-of-range values, anything the style lines cannot show exactly (reuse the "leftover" classification).
- **Trailing comments on action lines do not block a match**; dropped when Apply rewrites the rule. No warning. Leave a code comment where they are dropped saying it is intentional.
- Scope: disabled rules included; a `# ff:group` counts once (whole group tagged); already-tagged rules skipped; commented-out blocks that are not valid grammar (`can't enable`) skipped; economy rules never touched.
- Pending-edit model: never writes to disk; new styles = pending CONFIG edits, tags = pending RULES edits; shown unapplied (tab ` *`, indicator, Quit popup); Apply writes via the normal all-or-none Apply. Revert and Reload keep their usual per-tab behaviour (reverting CONFIG afterwards leaves rules with the existing `unknown style` flag; Apply drops the tag).
- UI: button on the **Custom** heading next to **+ Add style**. New styles `Custom Style N` (smallest free N, case-insensitive against all pending names), normal random custom id, appended after existing customs in order of first appearance in the file. Status: `Auto Detect: 3 styles, 41 rules tagged`; nothing: `Auto Detect: nothing new`; second click with nothing new changes nothing.

### Claude's Discretion
- Where the detection logic lives (pure module wrapping `rule_style_view` / `style_leftovers`) and how the batch is applied to the CONFIG and RULES models (one batch: add pending styles, re-sync the rules model's style set, tag rules; not per-rule `ApplyStyle`).

### Deferred Ideas (OUT OF SCOPE)
- Auto Detect for the economy section's custom-styled blocks.

## Phase Requirements

<phase_requirements>
| ID | Description | Research Support |
|----|-------------|------------------|
| DETECT-01 | Auto Detect button reads every non-economy rule, bare Show = `unchanged` | `rows()` excludes economy blocks and group followers (rules.rs:975, dispatch 1552); `rule_style_view` + new `to_style` |
| DETECT-02 | Match existing style on every parameter, built-ins (pending values) first | `StyleSet::iter` + `is_builtin`; equality with id excluded (see Equality table) |
| DETECT-03 | Remaining distinct styles become `Custom Style N`, rules tagged | `StyleSet::insert_new` assigns the fresh id (config.rs:793) |
| DETECT-04 | Inexpressible rules stay Custom; trailing comment does not block | `style_leftovers` (body.rs:2859); tokenizer keeps comment out of `values` (tokenize.rs:59) |
| DETECT-05 | Disabled included, group once, tagged skipped, invalid commented skipped | `RuleRow.cannot_enable`, `RuleRow.style`, `groups.members` |
| DETECT-06 | Pending in-memory edits only, per-tab Revert/Reload | batch via `ConfigState` + `WorkingFilter`; matrix below |
| DETECT-07 | Status line counts; idempotent second click | `apply_message: Option<(String, StatusKind)>` carries Ok and Info |
</phase_requirements>

## Summary

Everything needed exists; the missing pieces are small. (1) `StyleView -> Style` conversion, (2) a pure planner that classifies rules and assigns names, (3) one `ConfigState` method to append styles in one go, (4) one `WorkingFilter` method that tags rules in one batch (set the `# ff:style` meta on every block of a group, one `rebuild()`), (5) a `ConfigAction::AutoDetect` handler in `app.rs`, (6) the button and a probe in `config_view.rs`, (7) the code comment where Apply drops trailing comments.

The two non-obvious facts that shape the design: **`RowAction::ApplyStyle` rewrites the rule's bytes immediately** (rules.rs:1816-1850, `styled::rewrite_block`) and clones the whole document per call, so it must not be used per rule (O(rules x document), and it would normalise forms at click time). A batch that only sets the tag meta is enough, because Apply already rewrites every enabled tagged rule from its style (`styled::rewrite_styled_with`, styled.rs:161) and the unapplied state is derived from `doc.serialize() != applied_text` (rules.rs:990). And **the rules model only sees new styles after `sync_working_styles`** (app.rs:326); the per-frame sync runs at the top of the next frame (app.rs:3727), so the handler must call it explicitly between adding the styles and tagging.

**Primary recommendation:** pure `src/overlay/style_detect.rs` (plan = new styles + tags), applied by a `ConfigAction::AutoDetect` handler in this order: build inputs from `WorkingFilter` rows, plan against `config.pending_styles()`, if the plan is empty post the Info message and touch nothing; else `ConfigState::add_detected_styles`, `sync_working_styles`, `WorkingFilter::tag_styles`, post the Ok message.

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| Parse a rule's actions into a `Style`, equality, naming, planning | Pure model (`style_detect.rs`) | `style_lines.rs` (`to_style`) | No egui, unit-testable table-driven |
| Add pending styles | CONFIG model (`config_state.rs`) | `models::config::StyleSet` | Owns pending styles, id generation |
| Tag rules (group-aware, atomic, one rebuild) | RULES model (`rules.rs`) | `filter` document meta | Owns the document and groups |
| Orchestration, status message, sync | `app.rs` handler | — | Only place that holds both models and the messages |
| Button, tooltip, probe | `config_view.rs` | `skin::widgets::ff_button_ex` | Existing Custom heading row |
| Write to disk | Existing Apply (`apply_all.rs`, `styled.rs`) | — | Unchanged by this phase |

## Standard Stack

No new crates (project constraint). Use what is there: `egui` (existing), `indexmap` via `StyleSet`, existing `crate::filter` document API, `tempfile` for the test rigs (already used in `app.rs` tests). Package Legitimacy Audit: not applicable, no packages installed. [VERIFIED: codebase]

## Architecture Patterns

### Data flow

```
click "Auto Detect" (config_view.rs) -> ConfigAction::AutoDetect
  -> app.rs handle_config_actions:
       clear_events_on_edit()
       inputs  <- WorkingFilter rows (style None, not cannot_enable, no errors) + body_view(id) + keyword
       plan    <- style_detect::plan(&inputs, config.pending_styles())
       plan empty? -> apply_message = (Info "Auto Detect: nothing new"), STOP (no model touched)
       ConfigState::add_detected_styles(plan.new_styles)   // insert_new => fresh ids
       sync_working_styles(config, working)                // rules model learns the new set
       n <- WorkingFilter::tag_styles(plan.tags)           // meta on all group members, one rebuild
       apply_message = (Ok "Auto Detect: S styles, N rules tagged")
  next frame: combined_dirty -> apply_state Unapplied (app.rs:3748); tab markers from
  styles_dirty (Config *) and rules_edited (Rules *)
```

### Recommended structure
```
src/overlay/style_detect.rs   # pure: classify, equality, naming, plan, status_text
src/overlay/style_lines.rs    # + StyleView::to_style(&self) -> Option<Style>
src/overlay/config_state.rs   # + add_detected_styles(&mut self, Vec<(String, Style)>) -> usize
src/overlay/rules.rs          # + detect_inputs(&self) and tag_styles(&mut self, &[(BlockId, String)]) -> usize
src/overlay/config_view.rs    # + ConfigAction::AutoDetect, button, ConfigViewCtx.can_auto_detect, probe "auto-detect"
src/overlay/app.rs            # + handler, ctx field, add to the clear-events matches! list (app.rs:3026)
src/styled.rs                 # + code comment at the splice_body call in rewrite_block
src/overlay/mod.rs            # pub mod style_detect;
```

### Pattern 1: StyleView to Style (the missing step)
`rule_style_view(entries, keyword)` (body.rs:2646) returns `(StyleView, StyleBinding)`; the last line of each family is bound. Exactness is `style_leftovers(entries, &binding).is_empty()`: it is non-empty if any Action entry is an earlier line of a family, is outside every family (`DisableDropSoundIfAlertSound`, `...IfAlertSound`), or is a bound line in `binding.inexact` (out-of-range font via `FONT_SIZE_RANGE`, volume outside `VOLUME_RANGE`, `MinimapIcon -1 <colour> <shape>`, `PlayAlertSound None <vol>`, `PlayAlertSoundPositional None`, named sound, `CustomAlertSoundOptional`, `... False` drop line, minimap size outside 0..=2, unknown colour/shape names). [VERIFIED: body.rs:2646-2867]

`to_style` (put next to `from_style`, style_lines.rs:72, the inverse of it):
- `visibility`: `Show` -> `StyleAction::Show`, `Hide` -> `Hide`, `Minimal` -> return `None` (cannot be expressed; rule stays Custom).
- `text`, `border`: `StyleColour::from_array(rgba)`; the decoder already defaults a 3-token line to alpha 255 (grammar.rs:680-687). [VERIFIED]
- `background`: if `!had_alpha` use alpha **240** (game default); else the written alpha. This is the one place the decoder's default (255) is wrong for equality. Do NOT use `Rgba.rgba` blindly for background.
- `font`: `u8::try_from`; the view already clamps and leftovers rejects out-of-range, so exact rules are in 1..=45.
- `minimap`, `beam`, `sound`: clone (`Cancelable::Set/Cancel`). The view fills the default volume (`sound_shown`: 50 for ids, 100 for files) so `Sound.vol` is already the in-game value; `positional` already reflects `PlayAlertSoundPositional`. [VERIFIED: body.rs:2771-2817]
- drop sound: `style.set_drop_sound(view.drop_sound)`.
- `id: None`.
- Guard: a custom sound file name containing `"` fails `StyleSet::validate` (config.rs:691), which would block Apply. A quoted token cannot normally contain `"`, but add the same check in the planner and treat it as inexpressible rather than create an unusable style.

### Pattern 2: Equality (excluding id)
`Style` derives `PartialEq` including `id: Option<String>` (config.rs:300-323), so `==` is wrong for detection. Compare after clearing the id on both (`let mut a = a.clone(); a.id = None;`) or field by field. All other fields participate: `text/border/background` (`StyleColour`, 4 channels, Phase 43), `font`, `minimap`, `beam`, `sound` (`id`, `custom`, `vol`, `positional`), `action`, `disable_drop_sound`, `enable_drop_sound`. [VERIFIED]

Match order: iterate built-ins (`StyleSet::is_builtin(name)`) first, then the non-built-ins, from `config.pending_styles()` (the user's unsaved edits included). Do not rely on set order for "built-ins first" (a hand-edited file can interleave). First hit wins, so two identical customs resolve to the earlier one deterministically. New styles are deduped among themselves with a linear scan (no `Hash` on `Style`).

Equality table (use as the table-driven unit test; each row is a rule body -> expected):

| Rule body | Result |
|-----------|--------|
| bare `Show` | matches `unchanged` (if its pending value is still blank Show; otherwise a new blank-Show custom, accepted) |
| bare `Hide` | new custom Hide (not `hidden`) |
| `SetFontSize 21` + `DisableDropSound` + `Hide` | matches `hidden` |
| `SetBackgroundColor 40 42 54` vs `... 54 240` | same style (240); vs `... 54 255` a different style |
| `SetTextColor 1 2 3` vs `1 2 3 255` | same style |
| `PlayAlertSound 6` vs `PlayAlertSound 6 50` | same; `6 300` different |
| `CustomAlertSound "a.mp3"` vs `"a.mp3" 100` | same |
| `PlayAlertSound 6` vs `PlayAlertSoundPositional 6` | different (`positional`) |
| `PlayAlertSound None` vs no sound line | different (`Cancel` vs `None`) |
| `MinimapIcon -1` vs no minimap line | different |
| `DisableDropSound` vs `DisableDropSound True` | same; `DisableDropSound False` is inexact -> Custom |
| `SetFontSize 20` vs `23` | different (font off by 3) |
| two `SetFontSize` lines | Custom (earlier line is a leftover) |
| `CustomAlertSoundOptional "x.mp3"`, `EnableDropSoundIfAlertSound`, `SetFontSize 60`, `PlayAlertSound 6 400`, `MinimapIcon 3 ...` | Custom |
| `SetFontSize 20 # note` | matches like the plain line |
| `Minimal` header | Custom |

### Pattern 3: Planning (pure)
Input per candidate rule: `(BlockId, Vec<BodyEntry>, BlockKeyword)`. Candidate filter (do it in `WorkingFilter::detect_inputs`, so the planner is egui- and doc-free):
`row.style == StyleState::None` (this skips Known, Unknown and Unchecked tags: "already tagged" means any `# ff:style` tag; this also keeps a second click a no-op after a CONFIG revert has left Unknown tags), `!row.cannot_enable`, `row.errors.is_empty()` (an enabled rule with a grammar error cannot be styled reliably and would be rewritten away; known-action keywords with bad tokens are `Opaque` entries that `style_leftovers` does not see, yet `rewrite_block` selects action lines by keyword only, styled.rs:108-113), and `body_editable(id)`. `rows()` already excludes economy blocks and group followers, so a `# ff:group` appears once and a `group_broken` block is an ordinary row.

Algorithm: for each candidate in file order -> `rule_style_view`; skip if leftovers non-empty or `to_style` is `None`; look for an existing match (built-ins, then customs, then styles already planned in this run); else add a new planned style named `Custom Style N`. N is the smallest positive integer such that no pending name **or already planned name** equals `Custom Style N` case-insensitively. `plan` returns `{ new_styles: Vec<(String, Style)>, tags: Vec<(BlockId, String)> }` where the tag value is the style **name**; ids do not exist until insertion. New styles are appended in order of first appearance, which falls out of the loop order.

### Pattern 4: Applying the batch
- `ConfigState::add_detected_styles(&mut self, Vec<(String, Style)>)`: for each, `self.styles.insert_new(&name, style)` (fresh id per call, unique against the growing set, config.rs:793, 904). Names cannot collide or shadow required names by construction, but handle `Err` by skipping (and not tagging that name) rather than unwrap. No other `ConfigState` bookkeeping is needed: dirtiness is `PartialEq` against `styles_applied` (config_state.rs:602), so `Config *` appears at once.
- `sync_working_styles(config, working)` before tagging: `set_styles_over` -> `set_styles` -> `retag_renamed_legacy` + `rebuild()` (rules.rs:1017, 1059). Without it `set.tag_for(name)` is `None` and rows would show `unknown style`.
- `WorkingFilter::tag_styles(&mut self, tags: &[(BlockId, String)]) -> usize`: for each tag, resolve `self.styles.tag_for(name)` (`"<id> <name>"`), re-check the row is still eligible, then `doc.set_meta(member, "style", &tag)` for **every** member of `self.groups.members(id)` (group blocks must stay identical, including `# ff:style`, or the group breaks and shows `group broken`; rules.rs:1596 area and CLAUDE.md group rules). Do it on a doc snapshot per rule and restore on error (mirror the `dispatch` pattern, rules.rs:1578-1621). Do not call `styled::rewrite_block` here (Apply does it; disabled blocks are skipped by Apply until enabled, styled.rs:197). Call `self.rebuild()` once at the end. Count rules, not blocks. `set_meta` already works on disabled blocks (ApplyStyle uses it for them, rules.rs:1836).
- Do not set `apply_state` by hand: the frame derives `Unapplied` from `combined_dirty` (app.rs:3744-3750).

### Status line
`ff_status` is fed by `shell::event_from(update, update_behind, apply_message, config_message, sync, list_status)` (shell.rs:249). `config_message` is `(String, bool problem)` and can only be Info or Error, so use **`apply_message: Option<(String, StatusKind)>`** for both: `StatusKind::Ok` for the count message, `StatusKind::Info` for nothing new (precedence: apply_message wins over config_message, so it displays). Call `clear_events_on_edit()` first (app.rs:2282; it clears `apply_message`, `config_message`, `list_status`, `update_message`), either by adding `ConfigAction::AutoDetect` to the `matches!` list at app.rs:3026 (but note that list clears before the plan is known, so for the no-op click the clear is harmless but it does wipe an earlier message; acceptable) or by calling it inside the handler. Copy via a pure `status_text(new_styles, rules_tagged)`: `Auto Detect: {n} style(s), {m} rule(s) tagged`, singular `1 style`, `1 rule tagged`; `0 styles, 12 rules tagged` is valid. (Per UI-SPEC, nothing is shown for skipped rules.) [VERIFIED: shell.rs:249-294, app.rs:2282]

### Button (config_view.rs)
In the existing `ui.horizontal` with `group_heading(ui, "Custom")` (config_view.rs:840-851), inside the `right_to_left` closure, after `+ Add style` add `ui.add_space(8.0)` and `ff_button_ex(ui, &skin, "Auto Detect", 112.0, ctx.can_auto_detect, false)`; attach the tooltip with both `on_hover_text` and `on_disabled_hover_text` (precedent: the Reset button, config_view.rs:807-814, because `ff_button_ex` greys inside an enabled Ui). `probe("auto-detect", r.rect)`. Click -> `actions.push(ConfigAction::AutoDetect)`. Add `can_auto_detect: bool` to `ConfigViewCtx` (config_view.rs:171; constructed at app.rs:3272 and in config_view tests at 1325): `self.working.as_ref().is_some_and(|w| w.style_set().is_some())` (a loaded filter and a usable style set; `config` is already `Ok` for this view to render). Existing tests count `add-style` probes (config_view.rs:2052, 2215); extend those layout tests for the new probe and the 8pt gap, not replace them. Not blocked by an open draft, Fallback, or other edits (UI-SPEC).

### Trailing-comment code comment
Trailing comments never reach `StyleView` (tokenizer keeps them in `Tokens.comment`, not `values`; tokenize.rs:59) so they cannot block a match. They are lost where `rewrite_block` replaces action lines: `doc.splice_body(id, &action_lines, after, &targets)` (styled.rs:140). Note `rewrite_block` returns `Ok(false)` before that when the normalised lines already match (styled.rs:122), so a comment survives in that case; it disappears only when the rule is actually rewritten. Put the author-requested comment immediately above the `splice_body` call (and a sentence in the `rewrite_block` doc): dropping trailing comments on action lines is intentional; if it changes, Auto Detect (DETECT-04) relies on them not blocking a match. Do not add a UI warning.

### Anti-Patterns to Avoid
- **Per-rule `RowAction::ApplyStyle`:** clones the document and rewrites bytes per rule; contradicts "tags are pending", and normalises at click time.
- **`Style ==` for matching:** includes `id`.
- **Taking `Rgba.rgba` for the background:** gives 255 for a 3-token line, should be 240.
- **Tagging only the group leader:** breaks the `# ff:group` run.
- **Creating the styles but forgetting `sync_working_styles`:** every tagged rule would flash `unknown style`.
- **Posting the Ok message when the plan is empty:** second click must change nothing (no model call, no sync).

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Parse action lines / exactness | A second parser | `rule_style_view` + `style_leftovers` | The Custom editor's definition of "can't show exactly"; same one the author asked to reuse |
| Fresh custom id | Own random id | `StyleSet::insert_new` (`new_custom_id`) | Unique, 6-8 lowercase alnum, never `builtin:` |
| Tag text | `format!("{id} {name}")` | `StyleSet::tag_for` | Canonical form Apply expects (`resolve_tag` canonical check) |
| Group membership | Scan for `# ff:group` | `self.groups.members(id)` | Already validated runs |
| Unapplied state | A new dirty flag | Existing derived dirtiness (`rules_edited`, `styles_dirty`, `combined_dirty`) | Quit popup and tab markers follow for free |
| Rewriting from the style | Own line writer | `styled::rewrite_styled_with` at Apply | Unchanged pipeline |

## Revert / Reload matrix (pitfall 3)

Behaviour derived from the code; the tests should assert it, and any cell that disagrees with the code at test time is a finding, not a test to bend.

| Action after Auto Detect | Config | Rules | Notes |
|--------------------------|--------|-------|-------|
| Revert Rules (`revert_rules`, rules.rs:2056) | new styles stay pending (`Config *`), unused | tags gone, `Rules *` gone | doc re-parsed from `applied_text` |
| Revert Config (`ConfigAction::Revert*`, app.rs:3121-3152) | styles gone | tagged rules flip to `StyleState::Unknown` (flag, never blocking), still `Rules *` | next frame `sync_working_styles` re-judges; `retag_renamed_legacy` only moves tags whose id survives, so it does not resurrect them. Apply then runs `drop_unknown_style_tags` (tag dropped, actions kept) |
| Reload (filter) | per `reload_changed` / `config_reload_plan` (app.rs:2993, 3173) | unapplied edits discarded | assert actual behaviour per path |
| Quit with only tags pending | — | `rules_edited` true -> popup shows | and with only new styles pending (`styles_dirty`) |
| Apply | writes `styles.json` (new styles with ids) + filter (tag lines, rules rewritten from styles) all-or-none | | unchanged pipeline |

## Common Pitfalls

### Pitfall 1: Background alpha default
**What goes wrong:** `SetBackgroundColor 40 42 54` becomes alpha 255 and never matches a style that Apply would write back as 240.
**How to avoid:** `to_style` maps `!had_alpha` background to 240; test both forms in the equality table.

### Pitfall 2: Volume defaults and sound kinds
**What goes wrong:** `PlayAlertSound 6` vs `... 6 50` split into two styles; positional treated as equal.
**How to avoid:** use the view's filled `vol` (already 50/100) and compare `positional`; table rows above.

### Pitfall 3: Group tag on the leader only
**What goes wrong:** follower blocks lack `# ff:style`, group fails the identical-blocks check and becomes N rules flagged `group broken`.
**How to avoid:** set meta on `groups.members(id)`; test that a 3-block group is still one row, one tag.

### Pitfall 4: Idempotence and stale Unknown tags
**What goes wrong:** treating `Unknown` tags as untagged would re-tag after a CONFIG revert, or a second click creates styles again.
**How to avoid:** skip any rule with a tag (`StyleState != None`); empty plan returns before touching either model; test a double click for byte-identical `text()` and unchanged `style_names()`.

### Pitfall 5: Compact header cannot be rewritten
**What goes wrong:** `rewrite_block` refuses some headers (e.g. `Hide Continue`, styled.rs:34-37); at Apply the rule is listed in `StyleReport.failed` and keeps its actions while still carrying the tag.
**How to avoid:** accepted existing behaviour (pre-existing for any tagged rule). Optionally the planner may skip such rows; see Open Questions.

### Pitfall 6: Bare `Show` flood
Plain `Show` rules (foreign filters have many) will all be tagged `unchanged`, which locks their actions behind `style_lock` ("Set by style"). This is the decided behaviour; mention in the tooltip only if the author wants. If the pending `unchanged` was edited, a bare `Show` spawns a blank custom style instead.

### Pitfall 7: Existing layout tests
`config_view.rs` tests assert the heading-row geometry (probes at 2052, 2215). Adding a button changes them; update, do not delete.

## Code Examples

### `to_style` skeleton (style_lines.rs)
```rust
// Source: inverse of StyleView::from_style (style_lines.rs:72); background default per CLAUDE.md
impl StyleView {
    pub fn to_style(&self) -> Option<Style> {
        let action = match self.visibility {
            Vis::Show => StyleAction::Show,
            Vis::Hide => StyleAction::Hide,
            Vis::Minimal => return None,
        };
        let col = |c: Rgba| StyleColour::from_array(c.rgba);
        let mut s = Style::blank_show();
        s.action = action;
        s.text = self.text.map(col);
        s.border = self.border.map(col);
        s.background = self.background.map(|c| {
            let [r, g, b, a] = c.rgba;
            StyleColour::new(r, g, b, if c.had_alpha { a } else { 240 })
        });
        s.font = self.font.and_then(|f| u8::try_from(f).ok());
        s.minimap = self.minimap.clone();
        s.beam = self.beam.clone();
        s.sound = self.sound.clone();
        s.set_drop_sound(self.drop_sound);
        Some(s)
    }
}
```

### Equality ignoring id
```rust
fn same_style(a: &Style, b: &Style) -> bool {
    let (mut a, mut b) = (a.clone(), b.clone());
    a.id = None;
    b.id = None;
    a == b
}
```

### Smallest free name
```rust
fn next_name(taken: &[String]) -> String {
    (1..).map(|n| format!("Custom Style {n}"))
        .find(|c| !taken.iter().any(|t| t.eq_ignore_ascii_case(c)))
        .unwrap()
}
```
(`taken` = all pending names plus names planned so far.)

## State of the Art
Not applicable (internal feature). Phase 43 made `StyleColour([u8;4])` the colour type (3-value files read as alpha 255, written with 4 values); equality is over all four channels. [VERIFIED: 43-01-SUMMARY.md, config.rs:238-263]

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | `set_meta(id, "style", ..)` succeeds on a disabled block exactly as ApplyStyle relies on (rules.rs:1836 calls it for disabled ones) | Pattern 4 | A disabled-rule test would show it; fall back to skipping disabled rules is NOT allowed by DETECT-05, so fix `tag_styles` instead |
| A2 | `rewrite_block` failing on compact headers (`Hide Continue`) is the only Apply-time failure class for tagged rules | Pitfall 5 | An untagged-looking rule keeps the tag but not the style; surfaced by existing `StyleReport.failed` |
| A3 | Reload path cells in the matrix (exact reset of config vs rules) | Matrix | Test asserts actual behaviour; may reveal a surprising but pre-existing behaviour |

## Open Questions

1. **Skip rules whose header `rewrite_block` would refuse?**
   - Known: refusal only happens at Apply, is reported, and is non-blocking (styled.rs:34-37, 200-210).
   - Unclear: whether the author wants tagging of such rules at all.
   - Recommendation: do not pre-check (cost: a document clone per rule); note it in the plan as accepted.
2. **Should an `Unknown`-tag rule be re-detectable?**
   - Recommendation: no (CONTEXT says tagged rules are skipped; keeps idempotence after CONFIG revert).
3. **Message slot:** `apply_message` is semantically the Apply outcome. Recommendation: still use it (only slot with Ok and Info kinds); if the planner prefers, widen `config_message` instead, which touches `shell::event_from` and ~20 call sites.

## Environment Availability

Step 2.6: SKIPPED (pure code change; no external tools). Build/test with `CARGO_TARGET_DIR=C:\Users\jonat\AppData\Local\Temp\ff44\t` (ff.exe may lock `target/`).

## Validation Architecture

### Test Framework
| Property | Value |
|----------|-------|
| Framework | built-in `cargo test` (nightly via `rust-toolchain.toml`), lib tests + integration |
| Config file | `Cargo.toml`, `rust-toolchain.toml` |
| Quick run command | `CARGO_TARGET_DIR=C:/Users/jonat/AppData/Local/Temp/ff44/t cargo test --lib style_detect` |
| Full suite command | `CARGO_TARGET_DIR=C:/Users/jonat/AppData/Local/Temp/ff44/t cargo test --locked` |

### Phase Requirements -> Test Map
| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| DETECT-01/02 | bare Show = unchanged; built-in (pending values) match; id ignored; order builtin > custom > new | unit table | `cargo test --lib style_detect` | Wave 0 |
| DETECT-02 | equality table rows above (alpha 240/255, volumes, positional, cancel values, drop forms) | unit table | same | Wave 0 |
| DETECT-03 | `Custom Style N` smallest free N, case-insensitive; first-appearance order; fresh unique ids; 3 distinct -> 3 styles; font 20 vs 23 -> 2 | unit + `config_state` test | `cargo test --lib add_detected_styles` | Wave 0 |
| DETECT-04 | leftovers / Minimal / out-of-range stay untagged; trailing comment still matches; `rewrite_block` comment-drop behaviour pinned | unit + `styled.rs` test | `cargo test --lib trailing` | Wave 0 |
| DETECT-05 | disabled tagged; 3-block group -> one row, all members tagged, still one group; economy blocks untouched; tagged and `cannot_enable` skipped | `rules.rs` tests on `tag_styles` | `cargo test --lib tag_styles` | Wave 0 |
| DETECT-06 | file bytes and `styles.json` unchanged after click; Config * and Rules * set; per-tab Revert/Reload/Quit matrix before and after; Apply writes ids + tags, rules rewritten, second Apply no-op | `app.rs` rig tests (use `rig(..)`, `sync_working_styles`, `apply_everything`) | `cargo test --lib auto_detect` | Wave 0 |
| DETECT-07 | `status_text` plural/singular (`1 style, 1 rule tagged`, `0 styles, 12 rules tagged`); Info on empty plan; double click leaves `text()`/`style_names()` identical | unit + rig | `cargo test --lib auto_detect_status` | Wave 0 |
| UI | `auto-detect` probe left of `add-style`, 112pt wide, 8pt gap, disabled with no filter | `config_view.rs` probe tests (precedent 2052, 2215) | `cargo test --lib config_view` | Wave 0 |

### Sampling Rate
- **Per task commit:** `cargo test --lib style_detect` (and the touched module)
- **Per wave merge:** `cargo test --locked`
- **Phase gate:** full suite green incl. `artifact_guard`, `isolation_guard` (tests must pin `FERRISFILTER_HOME` / `FERRISFILTER_GAME_DIR`; lib tests use the existing temp rigs and never resolve the real folder)

### Wave 0 Gaps
- [ ] `src/overlay/style_detect.rs` with its table-driven `#[cfg(test)]` module
- [ ] test helper building `Vec<BodyEntry>` from a text body (use `WorkingFilter::from_text` + `body_view`, as `body.rs` tests do)
- [ ] no framework install needed

## Security Domain

No network, no new input surface, no file writes beyond the existing Apply. Relevant controls: V5 input validation only in the sense that generated style names/sound file names must satisfy `StyleSet::validate` (no `"`/control chars; name rules from `check_new_name`), otherwise Apply would refuse. No ASVS auth/session/crypto categories apply. [VERIFIED: config.rs:691, 802]

## Sources

### Primary (HIGH confidence, read this session)
- `src/overlay/body.rs` 2530-2867 (`StyleBinding`, `rule_style_view`, `style_leftovers`, volumes, drop state)
- `src/overlay/style_lines.rs` 1-120 (`StyleView`, `Rgba`, `from_style`)
- `src/models/config.rs` 200-360, 780-1000 (`Style`, `Sound`, `StyleColour`, `insert_new`, `new_custom_id`, `validate`)
- `src/overlay/rules.rs` 60-110, 295-348, 600-700, 960-1160, 1430-1500, 1541-1623, 1780-1860 (rows, `set_styles_over`, `dispatch`, `ApplyStyle`, groups)
- `src/styled.rs` 1-250 (`rewrite_block`, `rewrite_styled_with`, `drop_unknown_style_tags`)
- `src/overlay/app.rs` 324-337, 2279-2287, 3024-3237, 3257-3305, 3724-3750, rig at 4700-4730
- `src/overlay/config_view.rs` 20-205, 770-880, 1120-1180; `src/overlay/shell.rs` 249-294
- `src/emitter.rs` 221-278; `src/overlay/grammar.rs` 676-688
- `.planning/research/SUMMARY.md` (pitfalls 2, 3), `43-01-SUMMARY.md`, CONTEXT, UI-SPEC, CLAUDE.md

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH (no new dependencies)
- Architecture: HIGH (every integration point read; one new batch method per model)
- Pitfalls: HIGH for equality and groups; MEDIUM for the Reload matrix cells (A3) pending a test run

**Research date:** 2026-10-23
**Valid until:** until `rules.rs` / `app.rs` / `config_view.rs` change materially (line numbers drift; re-grep before editing)

## Project Constraints (from CLAUDE.md)
- No changelog / README edits during a milestone; a bug introduced and fixed inside the milestone is never mentioned outside `.planning/`.
- Overlay UI is not texty: tooltip only for explanation (matches UI-SPEC).
- Tests pin `FERRISFILTER_HOME` / `FERRISFILTER_GAME_DIR` on every spawn (`tests/isolation_guard.rs`); lib tests never resolve the real folder; only `src/locations.rs` asks the OS.
- `styles.json` ids: custom ids are random 6-8 lowercase alphanumerics, never `builtin:`-prefixed; ids are never touched by equality.
- No new dependencies; do not build in `target/` while `ff.exe` runs.
- Apply is the single all-or-none write; Revert is per tab; `Unknown` style tags are flagged but never block, and Apply drops them.
- Docs wrap at 70 chars only for README.md / CHANGELOG.md (not touched here).
