---
phase: 44-auto-detect-styles
reviewed: 2026-10-23T00:00:00Z
depth: standard
files_reviewed: 8
files_reviewed_list:
  - src/overlay/style_detect.rs
  - src/overlay/style_lines.rs
  - src/overlay/rules.rs
  - src/overlay/config_state.rs
  - src/overlay/config_view.rs
  - src/overlay/app.rs
  - src/overlay/mod.rs
  - src/styled.rs
findings:
  critical: 0
  warning: 3
  info: 4
  total: 7
status: issues_found
---

# Phase 44: Code Review Report

**Reviewed:** 2026-10-23
**Depth:** standard
**Files Reviewed:** 8
**Status:** issues_found

## Narrative Findings (AI reviewer)

## Summary

Reviewed `git diff 988e165..HEAD` for the eight files against 44-CONTEXT.md.

What holds up when traced through the code:
- **Equality.** `grammar::decode` sets alpha to 255 for a 3-token colour. `to_style` then changes only the background to 240. `rule_style_view` fills in the default volumes (alert 50, custom 100). Cancel values, the positional sound and both drop sounds stay distinct. Out-of-range fonts, volumes, sound ids and minimap sizes are inexact, so `style_leftovers` skips those rules. A quote or control character in a custom sound name is refused, matching `StyleSet::validate`.
- **Group tagging.** Every member is tagged and the group stays unbroken.
- **Normal economy exclusion.** `rows()` never holds economy blocks.
- **Idempotence on a second click.** Tagged rules, including Unknown ones, are skipped.
- **Revert.** Revert keeps its per-tab behaviour.

No blockers were found. The real gaps:
- Economy blocks are not excluded when the section markers are damaged.
- The Reload "finding" recorded in the summary comes from a test of a code path users cannot reach. The real bottom-bar Reload is never tested.
- The second-Apply idempotence test does not check the second Apply's outcome.

## Warnings

### WR-01: Economy blocks get Auto Detect styles and tags when the economy markers are damaged

**File:** `src/overlay/rules.rs:1428-1442` (`detect_inputs` / `detect_eligible`), `src/overlay/rules.rs:765-770` (`rebuild`), `src/overlay/app.rs:3317` (`can_auto_detect`)
**Issue:** `rebuild` decides which blocks are economy blocks from `section::find(&self.doc)`. When the markers are damaged, `find` returns `Err`, `span` is `None`, and `in_economy` is false for every block. Every generated economy block then becomes an ordinary user row in `rows()`. `detect_eligible` only checks the style tag, `cannot_enable`, errors and `body_editable`, and `can_auto_detect` ignores `filter_damaged`. As a result, Auto Detect plans the untagged price-tiered unique blocks (bronze background, `# ff:name ... Uniques`, no `# ff:style`):
- It adds a `Custom Style N` for each distinct one.
- It tags the unique blocks themselves.

This breaks the binding decision "economy rules are never touched". Apply is refused while the markers are damaged, but the damage does not stay contained. The user repairs the banner and presses Reload, which discards rule edits and keeps CONFIG edits. The phantom economy styles then remain pending and are written to `styles.json` by the next Apply.
**Fix:** Refuse Auto Detect when the section is damaged, both in the model and in the button:
```rust
// rules.rs
pub fn detect_inputs(&self) -> Vec<(BlockId, Vec<BodyEntry>, BlockKeyword)> {
    if self.damaged().is_some() {   // whatever accessor exposes rebuild's `damaged`
        return Vec::new();
    }
    ...
}
// app.rs
can_auto_detect: self.working.is_some() && self.config.is_ok() && !footer.filter_damaged,
```
Add a test: a filter with a lone `# ff:economy-begin` line above a unique block must give an empty plan.

### WR-02: The Reload test covers a path users cannot reach, and the recorded "finding" about Reload is wrong

**File:** `src/overlay/app.rs` (test `auto_detect_reload_keeps_rule_edits_and_reloads_config_only`); `.planning/phases/AUTONOMOUS-V072-DEFERRED.md:32`; 44-03-SUMMARY "Reload finding"
**Issue:** The test sends `ConfigAction::Reload` while `changed_on_disk` is empty, which takes the `ConfigReload::ConfigOnly` branch. In the UI, `ConfigAction::Reload` is only sent from the File changed on disk popup (`app.rs:2545`), and `bar_reload_target` sends it only when `changed` is not empty. The bottom-bar Reload a user actually presses is `BarReload::Shared`, which leads to `reload_pressed()` and then `reload()`. That path re-reads the filter, which discards the Auto Detect tags, and keeps CONFIG edits, so the new `Custom Style N` styles survive.

So the behaviour the summary and deferred note describe never happens in the app: "Config Reload only re-reads styles; the tags then stay … as `unknown style`". The behaviour that does happen is untested. The test's own assertion is also weak: `d.names().len() <= n` would pass even if Reload did nothing to the styles but drop some.
**Fix:** Replace or add a test that goes through the real path, and correct the deferred note and summary:
```rust
d.click();
d.app.reload_pressed();                 // bottom-bar Reload, nothing changed on disk
assert!(!d.rules_dirty());               // tags discarded with the rule edits
assert_eq!(d.app.working.as_ref().unwrap().text(), d.text);
assert_eq!(d.names().len(), n + 3);     // CONFIG edits kept (usual per-tab behaviour)
```

### WR-03: The second-Apply idempotence test and the naming test cannot fail

**File:** `src/overlay/app.rs` (test `auto_detect_apply_writes_styles_with_ids_and_tags_once`, `let _ = r2;`); `src/overlay/style_detect.rs:233-256` (`names_take_the_smallest_free_number_in_order_of_appearance`)
**Issue:**
1. The test discards the second `apply_everything` result (`let _ = r2;`). It then only checks that the filter bytes did not change. A second Apply that is refused also leaves the bytes unchanged: for example a changed-on-disk refusal, a grammar gate, or a style-rewrite failure on a compact header. So "second Apply is a no-op" is not actually checked.
2. The naming test computes its expected value with `next_name` itself (`let first = next_name(&taken)`), which makes it circular. It swallows the `insert_new("custom style 1", …)` result with `.unwrap_or(())`, with a comment admitting the author did not know whether the insert succeeds. Then it only asserts `names[1] != "Custom Style 3"`. A regression that skips N=2 or reuses a taken case-variant would still pass.
**Fix:**
```rust
assert!(matches!(r2.outcome, AllOutcome::Applied { .. }), "{:?}", r2.outcome);
```
and in the naming test:
```rust
set.insert_new("custom style 1", Style::blank_show()).unwrap();
...
assert_eq!(names, vec!["Custom Style 2", "Custom Style 4"]);
```

## Info

### IN-01: Rules with grammar errors are skipped silently, a gap in the CONTEXT scope

**File:** `src/overlay/rules.rs:1437-1442`
**Issue:** CONTEXT lists exactly what to skip: tagged, `can't enable`, economy. It also says "Every rule outside the economy section is read". `detect_eligible` additionally drops enabled rules with any grammar error, including errors in conditions only. Their action lines may be perfectly exact. Neither the status line nor the count says those rules were left out. Since the error blocks Apply anyway, the impact is small: after fixing it, a second click tags the rule.
**Fix:** Either accept this and record it in CONTEXT/CLAUDE.md, or skip only rules whose errors fall on action lines.

### IN-02: Auto Detect reads a 3-token background as alpha 240, but the Custom rule editor shows it as 255

**File:** `src/overlay/style_lines.rs:107-113` vs `src/overlay/body.rs` (`rule_style_view` → `grammar::decode` sets `rgba[3] = 255`)
**Issue:** CONTEXT requires the in-game value, 240, and `to_style` follows it. The rule's own Custom editor and its label preview still show the same `SetBackgroundColor 40 42 54` at alpha 255. After Auto Detect, the new style's editor shows alpha 240 for a rule that the RULES editor showed at 255 a moment earlier.
**Fix:** Make the Custom rule view treat a 3-token background as 240 for display and preview, or note the difference. Out of scope for this phase, but the two views now disagree.

### IN-03: A partial tagging failure leaves orphan pending styles and still reports Ok

**File:** `src/overlay/app.rs:3061-3086`, `src/overlay/rules.rs:1450-1478`
**Issue:** Styles are added before tagging. If `tag_styles` skips entries, the styles they would have used stay pending with no rules. Causes include a `set_meta` error rolled back by the snapshot, or `tag_for` returning `None`. The status still says `Auto Detect: N styles, M rules tagged` at `StatusKind::Ok`, even when M is 0. In practice this is unlikely, since `next_name` cannot produce a name that `insert_new` refuses, and loaded styles always have ids.
**Fix:** After tagging, delete any added style that no tagged rule uses. Report Info `NOTHING_NEW` when the tag count is 0.

### IN-04: Tagging couples ordinary user rules to the strictness control and to visibility rewrites

**File:** `src/overlay/style_detect.rs:53-90`
**Issue:** A user rule that visually equals a built-in from S to F gets tagged with that built-in, as decided. From then on, the Economy Visibility strictness control flips that user rule between Show and Hide. If the rule has a compact header such as `Hide Continue`, the keyword switch then fails at Apply (`StyleReport.failed`). The rule keeps the tag but no longer follows the style. Both behaviours follow from the decisions, but neither is stated in the tooltip or the CLAUDE.md note.
**Fix:** Mention it in the Auto Detect CLAUDE.md note, or skip compact-header rows in `detect_inputs` (research Pitfall 5).

---

_Reviewed: 2026-10-23_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
