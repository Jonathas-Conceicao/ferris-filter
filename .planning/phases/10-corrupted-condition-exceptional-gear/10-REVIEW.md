---
phase: 10-corrupted-condition-exceptional-gear
reviewed: 2026-06-24T00:00:00Z
depth: standard
files_reviewed: 3
files_reviewed_list:
  - src/models/config.rs
  - src/emitter.rs
  - config.json
findings:
  critical: 0
  warning: 1
  info: 2
  total: 3
status: issues_found
---

# Phase 10: Code Review Report

**Reviewed:** 2026-06-24
**Depth:** standard
**Files Reviewed:** 3
**Status:** issues_found

## Summary

Phase 10 implements two independent deliverables: the `corrupted: Option<bool>` condition on `When`
(EXC-01) and the `exceptional` named style with three gear rules in `config.json` (EXC-02/03/04).
The core implementation is correct on all counts — corrupted emits with no operator (`Corrupted False`),
resolve_style is exhaustive, the exceptional style matches the EXC-04 spec exactly (pink 255 121 198,
Dracula bg, Pink Cross minimap, no sound, no font), the three rules are ordered before recycle with the
right class lists, correct thresholds, and `corrupted: false`. OR-logic is correctly split into three
separate blocks. Class spellings are all valid per CLAUDE.md. No scope creep detected.

One warning-level bug exists: the two untracked SSF config files (`ssf_icewolf_config.json`,
`ssf_mercenary_config.json`) now fail to parse because `exceptional` is a required field in `Styles`
and neither file defines it. Since the `--config` flag was added to support exactly these files, this
is a real runtime breakage. Two info-level doc-comment staleness issues are also present.

## Warnings

### WR-01: SSF config files break on parse due to required `exceptional` field

**File:** `ssf_icewolf_config.json` (untracked), `ssf_mercenary_config.json` (untracked)
**Issue:** `Styles.exceptional` is a required (non-Optional, no `#[serde(default)]`) struct field
added in this phase. Both SSF configs pre-date Phase 10 and omit the `"exceptional"` key entirely.
`ferrisfilter generate --config ssf_icewolf_config.json` (and the mercenary variant) now fails at
deserialization with a serde "missing field `exceptional`" error before any generation logic runs.
These files are not committed but they are functional artifacts that the `--config` flag was designed
to use.

**Fix:** Add the `exceptional` style entry to both files. Because the SSF configs do not use the
exceptional style in any rule (confirmed: zero rules reference it), the definition can be a
copy-paste of the one from `config.json` with no behavioral change:

```json
"exceptional": {
  "text": [255,121,198], "border": [255,121,198], "background": [40,42,54],
  "font": null, "minimap": { "size": 2, "color": "Pink", "shape": "Cross" },
  "beam": { "color": "Pink", "temp": false }, "sound": null,
  "action": "Show", "disableDropSound": false
}
```

Alternatively, if every future style addition would require updating all custom config files,
consider making `exceptional` optional via `#[serde(default)]` with a parallel `Option<Style>`
declaration (matching the pattern used for `craftable`) and guarding the `resolve_style` arm the
same way. This would let older config files continue to load without modification, at the cost of
producing an `UnknownStyle` error only when a rule actually references `"exceptional"`.

## Info

### IN-01: `Rule` doc comment style vocabulary is stale

**File:** `src/models/config.rs:350-351`
**Issue:** The doc comment on the `Rule` struct lists valid style values as
`"S"`, `"A"`, `"B"`, `"C"`, `"D"`, `"E"`, `"F"`, `"unchanged"`, `"hidden"`, `"Recycle"` —
omitting `"unidentified"` (added earlier) and `"exceptional"` (added this phase). The live
list of valid values is enforced by `resolve_style` in `emitter.rs`, not by this comment, so
there is no runtime effect, but any reader checking what styles are valid will get a misleading
incomplete list.

**Fix:** Update both the struct-level doc comment (line 350) and the inline field comment
(line 360) to read:

```
Valid values: `"S"`, `"A"`, `"B"`, `"C"`, `"D"`, `"E"`, `"F"`,
`"unchanged"`, `"hidden"`, `"Recycle"`, `"unidentified"`, `"exceptional"`,
`"craftable"` (requires the optional `styles.craftable` field to be defined).
```

### IN-02: `resolve_style` doc comment omits `exceptional` from the closed-set list

**File:** `src/emitter.rs:58-59`
**Issue:** The doc comment for `resolve_style` says it returns `Err` for any name not in
`S/A/B/C/D/E/F/Recycle/unchanged/hidden/unidentified/craftable` — but `exceptional` is now
also a valid arm in the match and should be listed. No runtime impact; the match itself is
correct and exhaustive.

**Fix:** Change line 59 to:

```
/// S/A/B/C/D/E/F/Recycle/unchanged/hidden/unidentified/exceptional/craftable.
```

---

_Reviewed: 2026-06-24_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
