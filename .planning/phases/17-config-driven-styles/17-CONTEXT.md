# Phase 17: Config-Driven Styles - Context

**Gathered:** 2026-10-12
**Status:** Ready for planning
**Mode:** Smart discuss (autonomous, all v0.5 phases discussed upfront)

<domain>
## Phase Boundary

Named styles become data. A committed repo-root `styles.json` defines them, they are resolved by
name, and they are validated at load. `generate` output stays byte-identical. Requirements:
STYLE-01, STYLE-02, STYLE-04. Style tags on filter blocks (STYLE-03) are Phase 20. Style editing UI
is Phase 21. Promoted from backlog 999.1. The `FUTURE` notes at `src/models/config.rs` (`Styles`)
and `src/emitter.rs` (`resolve_style`) mark the two code sites.

</domain>

<decisions>
## Implementation Decisions

### styles.json shape
- A JSON object mapping style name → style, using the **same per-style fields as today's `Style`**.
  File order is preserved through an ordered map (`indexmap`, a small, standard dependency,
  accepted). It is committed at the repo root. Moving it to a per-user config location is a future
  item; installation isn't a concern yet.
- Initial content is exactly today's `config.json` styles (S, A, B, C, D, E, F, Recycle,
  unchanged, hidden, kite, exceptional), so output is unchanged.

### One source of truth
- `config.json` **loses its `styles` section**. Both `generate` (static path) and the dynamic
  path read `styles.json`. `artifact_guard` and `generate_snapshot` prove `generate` output is
  byte-identical, with no fixture re-baselining.
- Before deleting the hardcoded `Styles` struct, assert that the loaded default `styles.json`
  equals the old hardcoded/deserialized values. Emission order of style actions stays fixed in the
  emitter and never depends on JSON key order.
- Untracked experimental configs (`ssf_*_config.json`, which have inline styles such as
  `craftable`) are not supported migration targets. A clear error, or an optional `--styles`
  path, is fine (planner's discretion).

### Required styles and validation
- **Required, undeletable:** S, A, B, C, D, E, F (the economy emitter needs them), plus
  `unchanged` and `hidden`. Everything else is optional and user-definable.
- A rule naming an unknown style, or an invalid `styles.json` (missing required style, bad colour,
  bad enum), is a clear load-time error naming the style or field (STYLE-02). Adding a new style
  requires no code change (STYLE-01).

### Show/Hide and styles
- A style keeps its **action** (Show/Hide). A block may have its own Show/Hide, but **when a
  style is applied, the style's action overrides it** (author decision).
- **Hide styles can carry full visuals.** A Hide-action style may set any action (colours, font,
  sounds, minimap, beam, drop sound), since hidden items are revealed with Alt and the author wants
  to style them. Today's `hidden` (font 21 + DisableDropSound) is one such style, and the model
  must not restrict Hide styles to a subset.

### Claude's Discretion
- Type name (`StyleSet` suggested), a `from_pairs`-style helper so the many test literals don't
  churn, error variants, and loader placement in `compile::`.

</decisions>

<code_context>
## Existing Code Insights

### Reusable Assets
- `Style`, `Minimap`, `Beam`, `Sound`, `FilterColor`, `MinimapShape`, `StyleAction` in
  `src/models/config.rs` already capture the fields.
- The `emit_style_actions` path and the clone-null-append override pattern (priced uniques' bronze
  bg, gem per-category bg) must keep working.

### Established Patterns
- New optional fields use `serde(default)` + `skip_serializing_if`.
- Fixture discipline: never regenerate `seed-universal.filter` in a refactor commit.

### Integration Points
- `Styles` is used by the emitter, the pricer's economy emitters, `compile::load_inputs`, and the
  overlay's style picker (`rule_editor.rs`, `view_model.rs`). The overlay picker should list styles
  from the map in file order.

</code_context>

<specifics>
## Specific Ideas

- PITFALLS.md §8, §9, §23 (static byte guarantee, style rename, ordering) apply directly.

</specifics>

<deferred>
## Deferred Ideas

- Style rename (future requirement).
- Per-user `styles.json` location (future requirement).

</deferred>
