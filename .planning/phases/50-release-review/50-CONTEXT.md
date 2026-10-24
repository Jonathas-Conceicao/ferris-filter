# Phase 50: Release Review - Context

**Gathered:** 2026-10-23
**Status:** Ready for planning
**Mode:** Standard Release Review (PROJECT.md → Milestone Close-out); no discussion needed

<domain>
## Phase Boundary

Close out v0.7.2 per PROJECT.md "Milestone Close-out": code cleanup, README screenshots, version,
changelog draft, then the author's review. Requirements REV-01..05.

</domain>

<decisions>
## Implementation Decisions

### Steps (PROJECT.md, in order)
1. Code cleanup: remove duplication added in the milestone, dead code and scaffolding; strip every
   mention of in-milestone bugs (UAT / review ids, "fixes the regression from Phase N") from code,
   doc comments, test names, README.md and CLAUDE.md. No behaviour change; full test suite green
   before and after.
2. README screenshots: re-take any `docs/screenshots/` image whose screen changed (style editor with
   alpha, Styles section with Auto Detect, Config Overlay section, section headers, search fields,
   drag handles), with the debug `--screenshot` mode under a temp home, keeping names and framing.
3. Version: root `Cargo.toml` `version = "0.7.2"`, `Cargo.lock` refreshed (`ff-art` keeps its own).
4. Changelog draft: a `## [0.7.2] - <date>` section from the net changes since v0.7.1 (Added /
   Changed / Removed / Fixed, user-facing and short, wrapped at 70 chars), folding in `[Unreleased]`.
   Fixed lists only bugs that existed in a released version (e.g. the Config refresh button offset
   existed in 0.7.x). Draft for the author, not final.
5. Author review (last step, the phase's UAT): the author reviews and rewrites `README.md` and the
   drafted `CHANGELOG.md` section; agents change them only as directed. In the autonomous run this
   step is **stacked for the end** with the other deferred reviews.

### Docs to update
- CLAUDE.md must describe the new behaviour accurately (styles.json colour format, Auto Detect,
  economy names, `ff-state.json` (without advertising the delete-to-bypass trick), CLI user agent,
  hotkey setting and tray tooltip, section header / search art, drag reorder replacing arrows).
- `assets/README.md` / README credits for any new extracted art and the author's custom art.

### Claude's Discretion
- Which screenshots changed.

</decisions>

<code_context>
## Existing Code Insights

- Screenshot mode: `cargo run --bin ff -- --screenshot ...` (see CLAUDE.md `ff` row), temp home only.
- Release check: `installer/release-check.sh v0.7.2 <notes>` must pass (tag = `v` + Cargo version,
  non-empty CHANGELOG section).

</code_context>

<specifics>
## Specific Ideas

- README / CHANGELOG are human-written by the author: keep agent edits accurate and light, 70-char
  lines.

</specifics>

<deferred>
## Deferred Ideas

None.

</deferred>
