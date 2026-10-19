---
phase: 38-built-in-styles-strictness-reset
verified: 2026-10-18T00:00:00Z
status: human_needed
score: 7/7 must-haves verified
overrides_applied: 0
human_verification:
  - test: "Open the live overlay Config tab (temp FERRISFILTER_HOME) and look at the strictness row"
    expected: "Seven buttons fit on one line in the 660pt window, stacked captions readable, selected level highlighted (Flat Broke on a fresh install), Custom dim and not clickable"
    why_human: "Visual look and skin fit cannot be verified by grep or unit tests"
  - test: "Hover a minimap shape tile in the style editor, rule chips and pickers"
    expected: "Exactly one tooltip appears"
    why_human: "Unit test covers the style editor shape tile only; other draw sites need a live look"
  - test: "Click Reset to defaults on Styles and Tiers headings in a live overlay"
    expected: "Skinned popup with red Reset / Keep; reset is one pending edit, Revert undoes it, Apply writes it"
    why_human: "Popup visuals and interactive flow"
---

# Phase 38: Built-in Styles, Strictness & Reset Verification Report

**Phase Goal:** Shipped built-ins are the reworked set, one button row on Config sets strictness, and styles and tiers can be reset to defaults.
**Status:** human_needed (all automated checks pass)
**Re-verification:** No

## Observable Truths (ROADMAP success criteria)

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | Fresh install shows only reworked built-ins S-F, unchanged, hidden; no custom sound file; no custom styles | VERIFIED | `styles-default.json` has exactly 9 entries; each line is identical to the author snapshot `.planning/research/author-styles-2026-10-17.json` (checked per key); sounds are built-in ids only (6, 2) or null; no `custom` field |
| 2 | Existing user's `styles.json` kept; ids stay `builtin:S`..`builtin:F` | VERIFIED | ids `builtin:*` in defaults; loader unchanged (custom styles preserved by reset test `reset_styles_restores_builtins_and_keeps_customs`); old styles frozen as `tests/fixtures/seed-styles.json`, repointed in `cli_generate.rs`/`cli_errors.rs`; `cli_generate` 20/20 pass (includes byte-for-byte fixture/artifact guard) |
| 3 | Config shows `Custom \| Flat Broke \| Scrounging \| Getting By \| Comfy \| Loaded \| Div Farmer`; click hides nothing/F/E-F/D-F/C-F/B-F as one pending edit, Revert undoes, Apply writes | VERIFIED | `src/overlay/strictness.rs` (levels, `hidden()`), `ConfigState::set_strictness` (by id via `name_of_id`), `strictness_row` in `config_view.rs:742` wired to `ConfigAction::SetStrictness`; app tests `strictness_click_is_a_pending_style_edit_that_revert_undoes` and `strictness_apply_writes_styles_json_into_the_temp_home_only` pass |
| 4 | Highlight derived from S-F visibility; other patterns show Custom; Custom inert | VERIFIED | `strictness_of` derives from Show/Hide of the 7 ids, else Custom; `Level.clickable = l != Custom`; `set_strictness(Custom)` returns false; tests `shipped_defaults_read_flat_broke`, `other_patterns_are_custom` pass |
| 5 | Non-tier built-ins/customs untouched; tooltip says rules using tier styles are affected | VERIFIED | `set_strictness` only touches TIER_LETTERS; test `strictness_touches_only_tier_styles_and_reverts`; `AFFECTS_NOTE` appended to every level tooltip |
| 6 | Reset to defaults on Styles (Config) and Tiers (Economy) ask first, restore as one pending edit, customs kept, Revert undoes | VERIFIED | Button + `reset_armed` in `config_view.rs`, `shell.rs:712` (tiers); popups "Reset styles/tiers to defaults?" in `shell.rs:938-943`; `reset_styles`/`reset_tiers`/`*_at_defaults` in `config_state.rs`; test `reset_popups_apply_one_pending_edit_and_keep_does_nothing` passes |
| 7 | Minimap icon tooltip shown once | VERIFIED (code + test) | `style_lines.rs` test `a_minimap_shape_tile_shows_one_tooltip` counts rendered tooltip text == 1 and passes; live check of other draw sites left to human |

**Score:** 7/7

## Requirements Coverage

| Requirement | Status | Evidence |
|-------------|--------|----------|
| BSTY-01 | SATISFIED | defaults equal snapshot built-ins (CONTEXT supersedes wording: no shipped customs) |
| BSTY-02 | SATISFIED | no custom sound file in any shipped style |
| BSTY-03 | SATISFIED | ids kept; user file not touched |
| STRICT-01..04 | SATISFIED | see truths 3-5 |
| RESET-01, RESET-02 | SATISFIED | see truth 6 |
| FIX-01 | SATISFIED | see truth 7 |

All 10 IDs are marked Complete in REQUIREMENTS.md and have implementation evidence; no orphaned requirements.

## CONTEXT.md decisions honored

Defaults = built-ins only in snapshot order; `config.json` and `generate` untouched; frozen fixture `tests/fixtures/seed-styles.json`; CLAUDE.md updated (lines 30, 31, 209, 214 document no shipped customs, the `generate` unknown-style consequence, strictness row and resets); fresh install reads Flat Broke; reset keeps customs after built-ins.

## Behavioral Spot-Checks

Run with temp `FERRISFILTER_HOME` / `FERRISFILTER_GAME_DIR`:

| Check | Result |
|-------|--------|
| `cargo test --lib -- strictness reset minimap tooltip` | 41 passed, 0 failed |
| `cargo test --test cli_generate` | 20 passed, 0 failed |

Plan 06 reported full suite 1658 passed (not re-run).

## Anti-Patterns

None found in the phase files (no TBD/FIXME/XXX checked in strictness.rs, reset code). Pre-existing clippy errors out of scope.

## Gaps

None.

## Human Verification Required

1. Strictness row look in a live overlay (one line at 660pt, highlight, dim Custom).
2. Minimap tooltip appears once at every draw site (style editor, rule chips, pickers).
3. Reset popups and Revert/Apply flow visually in the live overlay.

---

_Verified: 2026-10-18_
_Verifier: Claude (gsd-verifier)_
