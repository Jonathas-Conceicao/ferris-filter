---
phase: 50-release-review
subsystem: release
tags: [release-review, cleanup, screenshots, version, changelog]
requirements: [REV-01, REV-02, REV-03, REV-04]
key-files:
  modified:
    - src/overlay/rules.rs
    - src/overlay/app.rs
    - src/overlay/config_state.rs
    - src/overlay/config_view.rs
    - src/overlay/drag_reorder.rs
    - src/filter/structure.rs
    - src/filter/divider.rs
    - src/models/config.rs
    - src/skin/colour_picker.rs
    - src/skin/editor.rs
    - src/skin/icons.rs
    - src/skin/widgets.rs
    - tests/filter_structure.rs
    - tests/overlay_rules.rs
    - docs/screenshots/*.png
    - Cargo.toml
    - Cargo.lock
    - CHANGELOG.md
    - README.md
    - CLAUDE.md
    - assets/README.md
    - installer/portable-readme.txt
metrics:
  completed: 2026-10-23
---

# Phase 50: Release Review (steps 1-4) Summary

Steps 1-4 of the v0.7.2 close-out on `milestone/v0.7.2-small-ui-improvements`. Step 5 (REV-05,
the author's review of README.md and the CHANGELOG draft) is stacked in
`.planning/phases/AUTONOMOUS-V072-DEFERRED.md` under "Phase 50". Nothing pushed, merged or tagged.

## 1. Code cleanup (REV-01)

- Stripped review / planning ids added this milestone from code comments and docs (WR-xx, D-xx,
  "Phase 44-03" style section markers, "49-01 spike", "research Pitfall N"), plus the old `WR-02`
  in CLAUDE.md's Sections note. Ids from earlier milestones left as they were.
- Removed the arrow reorder model, dead since drag reorder: `RowAction::Move`, `RuleRow`
  `can_move_up/down` and the layout pass that filled them, `Document::move_block`, `swap_runs`,
  `move_neighbour`, `rule_step` and `RuleStep`, `ConfigAction::MoveStyle`,
  `ConfigState::move_style`, `StyleSet::move_custom`. Section-header arrows are untouched.
  Arrow-only tests deleted; the others ported to `MoveTo` / `move_run_to` / `move_custom_to`.
- Removed other helpers left without a caller: chevron flag icons, `ff_text_field`,
  `ff_colour_card` / `ff_colour_card_ex`, `PickerMode::alpha` (always true).
- The headless egui drag experiments in `drag_reorder.rs` are kept as behaviour tests (module
  `egui_behaviour`, test names without spike ids).
- `move_then_inverse_is_identity` (fixture-level) became
  `move_then_back_keeps_every_line_and_the_order`: drag moves carry leading comments, so a byte
  round trip is not a property of `move_run_to` on the FilterBlade fixture.
- Clippy: two collapsible `if`s in milestone code fixed; 126 warnings remain, all on lines that
  predate the milestone. `cargo fmt --check` clean.
- Tests: green before (lib 1931) and after (lib 1921, filter_structure 17); no behaviour change.

## 2. README screenshots (REV-02)

All six `docs/screenshots/*.png` re-taken (every screen changed), 660x1032, same names and
framing, with the 42-02 recipe under `%TEMP%\ff50\shots` (temp home, game folder and profile;
RUSTUP_HOME / CARGO_HOME pinned to the real ones so rustup does not reinstall into the temp
profile). The real cursor showed as a row hover, so it was parked off the window during renders
and restored. Extra non-README renders for the author: `extra-config-top.png`,
`extra-style-picker.png` in `%TEMP%\ff50\shots\new`.

## 3. Version (REV-03)

Root `Cargo.toml` 0.7.2, `Cargo.lock` refreshed with `cargo update -p ferrisfilter --offline`;
`ff-art` stays 0.1.0.

## 4. Changelog draft (REV-04)

`## [0.7.2] - 2026-10-23` with a one-line summary, Added / Changed / Removed / Fixed, the
4-value `styles.json` compatibility note, and the `[0.7.2]` link (Unreleased now compares from
v0.7.2). `bash installer/release-check.sh v0.7.2 <notes>` passes (version=0.7.2,
prerelease=false).

## Docs

- README: hotkey bullet says another key can be picked in the Config tab's Overlay section;
  same note in `installer/portable-readme.txt`.
- `assets/README.md`: the `poe2/` row lists the new extracted art (Social-panel toggle plates
  and stone bar, clear button, magnifier, keybind button).
- CLAUDE.md: the assets row names the keybind button art and the `key_white` manifest option;
  checked against 50-CONTEXT "Docs to update" (alpha colour format, Auto Detect, economy names,
  `ff-state.json`, CLI user agent, hotkey and tray tooltip, header / search art, drag reorder).

## Commits

- 569a778 chore(50): strip review and planning ids from code comments and docs
- 7bc182a refactor(50): remove the arrow reorder model and other dead helpers
- e79afb7 style(50): collapse two nested ifs clippy flags in milestone code
- 12c480d docs(50): re-take README screenshots for v0.7.2
- 7eff6a5 chore(50): version 0.7.2
- 84a15bb docs(50): draft the 0.7.2 changelog section
- d3d2f57 docs(50): mention the configurable hotkey and list the new game art
