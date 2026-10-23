# Research Summary — v0.7.2 Small UI Improvements

**Synthesized from:** STACK.md, FEATURES.md, ARCHITECTURE.md, PITFALLS.md (2026-10-23)
**Overall confidence:** MEDIUM-HIGH (stack and integration points read from code and crate
sources; egui drag behaviour and hotkey re-registration not yet spiked)

## Executive summary

Eight small-to-medium features on a well-factored codebase, plus the Release Review. Existing
seams carry most of the work: one shared style emitter (`emitter::style_action_lines`), a
style-from-action-lines parser (`rule_style_view` / `style_leftovers` in `src/overlay/body.rs`),
an alpha-capable colour picker (Rules path), closure-taking network workers, a virtual list with a
cumulative offsets table, and key-by-key settings writes. New logic goes into pure, tested modules
(`style_detect`, `drag_reorder`, `throttle`, tier labels); views stay thin.

**No new Rust dependencies are needed.** Do not add `egui_dnd`, `keyboard-types`, `chrono`,
`rand`, `serde_with` or global-hotkey's `serde` feature. The egui `Key` → global-hotkey `Code`
table is hand-written (~80 lines).

The risks are byte identity and semantics, not UI: alpha must not change existing `styles.json`,
`generate` output or `artifact_guard`; Auto Detect equality has many traps; drag in a virtualized
list needs pointer-owned state and a new `move_run_to` (the arrow move is one step at a time);
hotkey capture must not trigger the current chord.

## Stack

- **Alpha:** optional alpha, written only when not 255.
- **Drag:** `Sense::drag()` on the handle only, drag state in list state, shadow on a Tooltip-order
  `layer_painter`, drop slot by `partition_point` over the offsets, auto-scroll via
  `scroll_with_delta` scaled by `stable_dt`. Not `dnd_drag_source` (virtualized list).
- **Hotkey:** `HotKey` `FromStr`/`Display`; `Code`/`Modifiers` re-exported by global-hotkey; the id
  is `(mods << 16) | key`, so it changes with the chord and the poll must use the current id;
  register/unregister at runtime on the manager's thread; capture from `Event::Key { physical_key }`.
- **Throttle:** unix seconds from `SystemTime`; a timestamp in the future counts as expired.
- **User-Agent:** `USER_AGENT` (`src/sync/client.rs`, also used by `src/update.rs`) becomes a
  parameter: one overlay value, one CLI value, both naming the public repo.
- **Social-panel art:** `ff-art` manifest entries; vector painting as plan B.

## Features (table stakes)

- **Alpha:** Alpha slider in the style colour picker, byte-identical round trip, alpha through
  every emitter, translucent previews.
- **Auto Detect:** exact equality; built-ins, then existing customs, then new `Custom Style N`
  (smallest free N, case-insensitive); idempotent; status-line summary.
- **Drag:** handle only, greyed source, shadow, drop marker, wheel and edge auto-scroll, Escape
  cancels; groups as one unit; never into or below the economy section; off while searching;
  customs stay below built-ins.
- **Headers / search:** bar + toggle plate at the existing 36pt; X only with text, keeps focus;
  magnifier reserves left padding.
- **Hotkey:** capture control, modifier required, register new then unregister old, reset to
  default, bad saved value → default with a warning.
- **Economy names:** range from the cutoffs (not item prices), unit shown.
- **Throttle:** per kind and per league, recorded on success only, fake refresh looks identical and
  does not advance Last sync.

**Deferred candidates:** keyboard reordering, draggable section headers, drop onto a collapsed
section, hotkey disable, throttle bypass, Auto Detect confirmation popup.

## Architecture

- **Alpha:** a colour type with alpha 255 normalised to absent; one emitter edit; CONFIG moves onto
  the Rules alpha picker path.
- **Auto Detect:** wraps `rule_style_view` / `style_leftovers` (needs `StyleView::to_style()`);
  applies as one batch (add pending styles → re-sync the rules model's style set → tag rules), not
  per-rule `ApplyStyle`, which rewrites bytes at once. Apply pipeline unchanged.
- **Drag:** `Document::move_run_to` in `src/filter/structure.rs` with a banner-aware insertion
  point, `RowAction::MoveTo` computing `moved across Continue` from every rule passed over, a pure
  `drag_reorder.rs` shared by the rule list and the style table.
- **Hotkey:** keep the manager alive after a failed registration; read the saved chord before the
  first registration in `launch()`; tray tooltip from the live value.
- **Economy names:** `tier_labels` from `pricer`, used only on the tagged (game-folder economy)
  path; `generate` output and fixtures untouched.
- **Throttle:** a pure `RequestLog` with injected `now`, wrapping three call sites in `app.rs`.

## Critical pitfalls

1. **Alpha changes bytes** — normalise 255 to absent; round-trip test first; keep `artifact_guard`
   green. The bronze unique background (`175 96 37 240`) stays outside styles.
2. **Equality traps** — background default alpha is 240 vs 255 for text/border; volume defaults
   (50 alert, 100 custom); cancel values are `"None"`, not null; `PlayAlertSound` ≠ Positional; a
   bare `Hide` is not `hidden`; inexpressible lines leave the rule Custom. Table-driven tests.
3. **Per-tab Revert** (author decision: kept) — Config Revert can leave rules tagged to a now-absent
   style; they show `unknown style` (flagged, never blocking) and Apply drops the tag. Test the
   {Revert Rules, Revert Config, Reload, Quit} × {before, after Auto Detect} matrix; the tab ` *`
   and the Quit popup must count a tags-only change.
4. **Virtualized drag** — list-level drag state ended from the pointer; slot from y vs offsets;
   collapse the expanded row on drag start; cancel on Escape and focus loss; repaint every frame.
5. **Hotkey** — pause the live hotkey while capturing; register new then unregister old, save only
   on success; update the stored id; never register a real hotkey in tests.
6. **Throttle** — record only after a successful write; fake a refresh only when matching data is
   on disk; the CLI neither reads nor writes throttle state.
7. **Economy names** — old names are pinned in tests (`emitter.rs`, `dynamic.rs`, `rules.rs`) and
   `app.rs` checks `.contains(" tier ")`; the first Apply after upgrade shows a one-time economy
   text diff (expected).

## Suggested phase order

1. Style alpha (foundation for Auto Detect)
2. Auto Detect styles
3. Economy tier-range names
4. Request throttling + CLI user agent
5. Customizable hotkey
6. Game-art widgets: section headers, search X and magnifier (one art session; before drag)
7. Drag-and-drop reorder (largest; builds on the redrawn rows and the handle art)
8. Release Review

Phases 3-5 are independent. Phases touching `config_view.rs` (1, 2, 5, 7) stay sequential.

## Research flags

- **Spike / research:** Phase 2 (bare Hide, default-alpha equality, trailing comments, Revert
  matrix); Phase 5 (register-then-unregister, tray tooltip update, `physical_key` on non-QWERTY);
  Phase 6 (`ff-art list` for the Social-panel art); Phase 7 (wheel + forced offset during a drag,
  shadow opacity, source row leaving view).
- **Standard patterns:** Phases 1, 3, 4, 8.
- ARCHITECTURE.md line numbers will drift; re-grep before editing.

## Decided by the author

No new dependencies; per-tab Revert/Reload with Auto Detect as ordinary pending edits; exact
equality on every parameter (bare `Show` = `unchanged`, built-in matches tagged as the built-in);
inexpressible rules stay Custom; disabled included, groups once, tagged skipped; alpha before Auto
Detect; throttle persists across restarts, CLI unthrottled with its own User-Agent; asset names in
`assets/custom/` kept exactly; 999.16 and 999.15 stay in the backlog.
