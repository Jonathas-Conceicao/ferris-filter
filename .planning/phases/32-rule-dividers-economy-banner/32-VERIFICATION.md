---
phase: 32-rule-dividers-economy-banner
verified: 2026-10-16T00:00:00Z
status: human_needed
score: 5/5 must-haves verified
overrides_applied: 0
human_verification:
  - test: "SECT-01: load a filter with # ff:section banners (temp home, sample filter)"
    expected: "Each banner is a header with the right rule count; a foreign #==== comment stays a comment; rules above the first banner have no header; a filter starting with a banner has no headerless rules"
    why_human: "Live overlay rendering"
  - test: "+ Add section, Section above, rename (pencil)"
    expected: "+ Add section inserts at top, scrolls to it, focuses the name box; Section above puts the banner above the rule (above the whole run for a grouped rule); Enter commits, Escape cancels; spaces trimmed, empty becomes Section, pasted newlines stripped. The Section above row is captured in shots/section-above-row.png (added by the orchestrator)"
    why_human: "Interaction"
  - test: "Move / remove a section; Apply / Revert / Reload for add, rename, move, remove"
    expected: "Divider steps past one rule per click (a group counts as one); arrows dim when nothing is left to step over; x removes only the banner; Apply writes, Revert undoes, Reload re-reads"
    why_human: "Interactive flow"
  - test: "SECT-03 persistence"
    expected: "Collapse/expand survives an overlay restart (collapsed_sections in ff-settings.json), never shows the unapplied marker or changes the filter file; renaming a collapsed section keeps it collapsed"
    why_human: "Needs a real restart"
  - test: "SECT-04 Economy section"
    expected: "Economy is the last header, collapsed by default, closed padlock, '{n} rules', muted read-only rows, no arrows; expanded shows the small x opening Remove economy popup; afterwards the Add economy section footer button appears and re-adds it"
    why_human: "Visual and interaction"
  - test: "ff-cli apply --add-economy / --remove-economy; old markers"
    expected: "--add-economy appends the three-line banner at EOF; --remove-economy removes banner and blocks but keeps a following user section banner; plain apply never adds one; a lone old one-line ff:economy-end / economy-begin file is reported damaged and stops apply"
    why_human: "End-to-end CLI run against a real file (unit tests cover the logic)"
  - test: "SECT-05 search and reorder"
    expected: "Search finds a rule inside a collapsed section; header expands for display with 'm of n rules'; clearing restores collapse; rule arrows hop a divider; + Add rule on a file starting with a banner inserts above the banner"
    why_human: "Interactive behaviour"
  - test: "Round trip"
    expected: "Open an untouched banner filter, Apply, file byte-identical"
    why_human: "Live Apply on a real file"
  - test: "Less-texty UI changes for author review"
    expected: "WR-02 amber triangle on a condition-less enabled non-Continue rule followed by more rules (not on the last, Continue or disabled rule); closed padlock on Economy header and on can't-enable rules; chevron and pencil icons; Section above button; interim small x on expanded Economy header"
    why_human: "Visual design judgement; the pointer-hover artefact in shots is a screenshot-mode quirk"
---

# Phase 32: Rule Dividers & Economy Banner Verification Report

**Phase Goal:** Named, collapsible rule sections from a three-line `# ff:section <name>` banner; the economy section as a begin-only banner shown as the locked last divider; collapse state in ff-settings.json; CLI add/remove economy; + Add rule outside every section.
**Status:** human_needed (all automated checks pass; live-overlay behaviour needs the author)
**Re-verification:** No, initial verification

## Observable Truths

| # | Truth (ROADMAP SC) | Status | Evidence |
|---|---|---|---|
| 1 | Three-line banner loads as named section; rules above first have no header; foreign banners are comments | VERIFIED | `src/filter/section.rs` (strict banner detection, tests at lines ~492-516 for foreign `#====`, short rules, lone line); `src/filter/divider.rs` edits with byte-exact tests |
| 2 | Add/rename/move/delete divider, deleting keeps rules; Apply-only, Revert undoes, bytes outside unchanged | VERIFIED (interaction: human) | `divider.rs` tests for insert/rename/remove/CRLF preservation; `rules.rs` section actions; lib suite 1246 passed |
| 3 | Collapse/expand persisted per filter in ff-settings.json; search finds inside collapsed; reorder/groups work across sections | VERIFIED (restart: human) | `settings.rs` `collapsed_sections` per-filter map with reserved economy key; tests `search_opens_collapsed_sections_and_counts_matches`, `collapsed_sections_hide_their_rules...` |
| 4 | Economy as locked last divider replacing the footer; CLI add/remove banner, begin-only, old markers not recognised; Apply regenerates | VERIFIED | `section.rs` `BEGIN_LINE` = required text, damaged-marker errors for old one-line form (lines 69, 73); `locked rule blocks` string absent from src; add/remove economy wired in cli.rs / run.rs / dynamic.rs; interim overlay entry (footer button + x) accepted per orchestrator note |
| 5 | generate output and fixtures unchanged | VERIFIED | `git diff 473449f -- tests/fixtures` empty; orchestrator gates (artifact_guard/generate_snapshot inside 1357 passing) |
| RULES-03 | + Add rule lands outside every section | VERIFIED | `structure.rs::insert_rule_at_top`; tests `add_rule_inserts_at_top_of_file`, `add_rule_above_a_group_keeps_the_group_one_rule` |

**Score:** 5/5 truths verified

### Spot checks
- `CARGO_TARGET_DIR=.../ff-target cargo test --lib`: 1246 passed, 0 failed (run by verifier).
- Orchestrator gates: full cargo test 1357/0, fmt clean, clippy 126 (baseline), release test build OK.

### Requirements Coverage

| Requirement | Source Plans | Status |
|---|---|---|
| SECT-01 | 32-01, 32-05, 32-06 | SATISFIED |
| SECT-02 | 32-03, 32-04, 32-05, 32-06 | SATISFIED |
| SECT-03 | 32-02, 32-04, 32-05, 32-06 | SATISFIED |
| SECT-04 | 32-01, 32-04, 32-05, 32-06 | SATISFIED |
| SECT-05 | 32-03, 32-04, 32-05, 32-06 | SATISFIED |
| RULES-03 (extra, plans 03/05) | 32-03, 32-05 | SATISFIED |

No orphaned requirements: all five SECT IDs appear in plan frontmatter and in REQUIREMENTS.md (Phase 32, Complete).

### Anti-Patterns
None blocking found in the files checked; review (32-REVIEW.md iteration 2) reports no blockers.

### Gaps Summary
No gaps. Remaining items are interactive/visual checks, listed in the human_verification section (the 32-06 checklist consolidated, plus the UI changes for author review).

---

_Verified: 2026-10-16_
_Verifier: Claude (gsd-verifier)_
