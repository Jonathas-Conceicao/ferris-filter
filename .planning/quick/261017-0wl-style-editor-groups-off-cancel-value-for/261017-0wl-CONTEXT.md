# Quick Task 261017-0wl: style editor groups (off / cancel / value) - Context

**Gathered:** 2026-10-17
**Status:** Ready for planning (design approved by the author; decisions are LOCKED)

<domain>
## Task Boundary

Style editor groups: off / cancel / value for minimap, beam, alert sound, drop sound; folder sound
picker; positional toggle. The CONFIG style editor and a Custom rule's STYLE editor must remain the
SAME UI with the SAME options, from the shared `src/overlay/style_lines.rs` (made shared by quick
task 261016-qi7 — read its SUMMARY). No duplication.

</domain>

<decisions>
## Approved layout

```
 STYLE        [ Custom            v]
 VISIBILITY   [Show][ Hide ]
 LABEL        |        [ ITEM NAME ]        |   <- preview: no icon/beam when off or /
              [ Text ][ Border ][ Background ]
 FONT SIZE    (x) ==========O======   40       (same on/off pattern, one line, no cancel value)
+================================================================+
| MINIMAP  (x)                                                   |  <- one tinted band per group
|          [ / ] # # # # # # # # # # #         (11 colours)      |
|                12 shapes                                       |
|                [Large][Medium][Small]                          |
+================================================================+
+================================================================+
| BEAM     (x)                                                   |
|          [ / ] # # # # # # # # # # #     [Temp]                |
+================================================================+
+================================================================+
| ALERT SOUND (x)                                                |
|          [ / ] [ Alert 6            v]                         |
|                VOLUME  ==========O====  200                    |
|                POSITIONAL ( )                                  |
+================================================================+
+================================================================+
| DROP SOUND  (x)   [ Silent ][ Normal ]                         |
+================================================================+
              ( ) CONTINUE TO LATER RULES          <- rules only
```

## Rules (locked)

1. Each of Minimap / Beam / Alert sound / Drop sound is a multi-line group drawn on one shared
   tinted band. Its first line is the label + an on/off round check. Off = the group folds to that
   one line and writes NOTHING (inherit / game default). On = expands. Turning off removes the
   line(s); the last values are remembered to restore on re-enable (existing memory mechanism in
   style_lines.rs). Font size keeps the same on/off pattern on one line, no cancel value.
2. When on, Minimap / Beam / Alert sound offer a `/` tile first = the cancel value:
   `MinimapIcon -1`, `PlayEffect None`, `PlayAlertSound None`. With `/` picked, the other pickers
   stay visible but greyed. Then the values: Minimap 11 colours + 12 shapes + Large/Medium/Small
   (the old None size pill goes away); Beam colours + Temp; Alert sound dropdown + Volume +
   **Positional** toggle (writes `PlayAlertSoundPositional` instead of `PlayAlertSound`; greyed when
   a file is picked — there is no positional form for custom files). Drop sound when on is a
   `[Silent][Normal]` switch: Silent -> `DisableDropSound`, Normal -> `EnableDropSound`.
3. Alert sound dropdown: `Alert 1 ... Alert 16`, then a "Filter folder" section listing sound files
   found in the game filter folder (the folder the loaded filter is in; read-only listing, refreshed
   sensibly, never written). Extensions: PoE supports .mp3 and .wav for CustomAlertSound (verify
   if anything in the repo says otherwise; .ogg only if evidence). Picking one writes
   `CustomAlertSound "<file>" <vol>`. NO free-text file field any more, and never write
   `CustomAlertSoundOptional`. A rule/style referencing a file that is not in the folder keeps its
   line and shows in the picker as that name with the amber warning flag (tooltip explains).
   Loaded `CustomAlertSoundOptional` / other unrepresentable lines stay kept as rows as today.
4. Preview: when Minimap is off or `/`, the label preview, the CONFIG table Icon column and the
   collapsed rule's style chip show NO icon (the `/` appears only in the picker). Same for the beam
   in the preview if it shows one.
5. Styles: the `styles.json` style model in src must store these states: minimap / beam /
   alert-sound cancel values, positional alert sound, drop sound Enable as well as Disable. Extend
   the model/serialisation compatibly (existing files load unchanged; existing styles serialise
   byte-identically), and make `generate` and the economy-section emitter emit the new values when a
   style uses them. `generate` output and `tests/fixtures` must stay byte-identical (no shipped style
   uses them). Keep the overlay's untouched-rule lossless guarantee and "edit rewrites only its line".
6. Regression test: a Custom rule with a single alert-sound line shows no extra leftover action row
   (an earlier screenshot showed a stale duplicate "Play Alert Sound" row; ensure it cannot come
   back).

## Hard rules for every agent

- The user is running their own `cargo run --bin ff` (debug). Use
  `CARGO_TARGET_DIR=C:/Users/jonat/AppData/Local/Temp/ff-target` for ALL cargo commands. Never kill
  or launch their processes; no keystrokes.
- Stay on branch `milestone/v0.6-new-poe2-based-tool-ui`, never push. Commit with the Bash tool
  using a heredoc; every commit message ends with exactly:
  ```
  Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
  Claude-Session: https://claude.ai/code/session_016M9qsaR7etMBDh279zy4Qx
  ```
- Never write to `C:\Users\jonat\OneDrive\Documentos\My Games\Path of Exile 2` or the Steam install.
- `tests/fixtures` byte-identical, `cargo test` green, `cargo fmt --check` clean, no new clippy
  warnings beyond the baseline count (122; count with
  `cargo clippy --all-targets 2>&1 | grep -c '^warning'` style — re-verify the baseline first).
- Update CLAUDE.md (overlay notes rule-editor STYLE paragraph, the `styles.json` row, Named styles
  section) and CHANGELOG `[Unreleased]`.
- Stage explicit paths only. Never touch untracked/ignored user files: styles.json, tiers.json,
  prices.json, ff-settings.json, dynamic.json, mapping.json, scratch_*, ssf_*, foo.filter,
  temptest_gen.filter, *-leaguestart.filter, the `.gitkeep` files under `.planning/phases/24, 26,
  27, 28`. No Python. Keep committed files LF.

</decisions>

<canonical_refs>
- `.planning/quick/261016-qi7-custom-rule-styling-uses-the-style-edito/261016-qi7-SUMMARY.md`
- CLAUDE.md (overlay notes, `styles.json` row, Named styles table)
- https://www.pathofexile.com/item-filter/about (MinimapIcon -1, PlayEffect None, PlayAlertSound None)
</canonical_refs>
