# Architecture Patterns

**Domain:** Lossless PoE2 loot-filter editor integrated into the existing FerrisFilter (Rust, eframe overlay)
**Milestone:** v0.5 Generic Filter Editor & Initial App Configuration
**Researched:** 2026-10-12
**Confidence:** HIGH on integration points (read from the code); MEDIUM on the parser shape (design recommendation, not yet validated against a real FilterBlade/NeverSink file; the first phase must do that).

## Recommendation in One Paragraph

Add **one new library layer, `src/filter/`** (a line-preserving concrete syntax tree, "CST"), that sits **beside** the `Config`/`Rule`/`When` model, not instead of it. `config.json -> universal.filter` keeps running through `Config` + `emit_rules` untouched, so the byte-for-byte `seed-universal.filter` fixture and `artifact_guard` cannot be affected. The dynamic path stops using `Config` entirely: its store becomes a parsed `FilterDoc`, edited through an adapter that exposes the **existing `When`/`Rule` typed editor** for the blocks it can model exactly, and shows the rest read-only. Economy sections live in a **sentinel-delimited region** that Apply splices byte-wise. Styles become a shared `StyleSet` (ordered name -> `Style` map) loaded from `styles.json` for the dynamic path and from `config.json`'s inline `styles` for the static path.

## Current Architecture (grounding)

| Piece | Today | Fact that constrains v0.5 |
|-------|-------|---------------------------|
| `models/config.rs` `Styles` | Closed struct, 11 fixed + optional `exceptional`/`craftable`, `deny_unknown_fields` | FUTURE note at ~l.110; `Config.styles` is `Styles` |
| `emitter.rs` | `generate_as(config, priced, identity)` = header + `emit_rules` + `emit_currency_sections` + `emit_unique_sections` | The two economy emitters read **only** `config.styles` (via `style_for_tier`), nothing else from `Config`. They are cheaply decoupled. `resolve_style` is a match (l.99) |
| `compile.rs` | `load_inputs(InputPaths, policy)` loads **Config** + prices + tiers + fallback pair, runs pricer -> `CompileInputs{config, priced, price_source}`; `compile_as` pure | `config` is baked into `CompileInputs`; the dynamic path currently smuggles `dynamic.json` in through `InputPaths::with_config` |
| `dynamic.rs` | `BASE_FILE`/`STORE_FILE`, `ensure_store`/`reset_store`, `compile_store`, atomic `write_filter` into the game dir | Whole seeding/reset half dies; `write_filter` (temp file in dest dir + persist) is reusable as-is |
| `overlay/store.rs` | `load`/`save` of `Config` as JSON, `default_paths()` = **`current_dir()`** | The launch-from-CWD bug lives here (and in `app.rs` via `store_path.parent()` -> `InputPaths::relative_to`, and `sync_economy_blocking(&base_dir)`) |
| `overlay/edit.rs` | `WorkingRules` = `Vec<(RuleId, Rule)>` with session ids; typed setters on `When` (`set_class`, `set_rarity`, ...); `available_styles(&Styles)` | ~1400 lines and most of the 368 tests are on `When` mutators; **keep the mutators**, change only what they sit on |
| `overlay/app.rs` Apply | `to_rules` -> `store::save` (terminal on failure, BLOCKER-1 guard) -> `compile_store` reads **disk** -> `write_filter` | The "save first, then compile from disk" ordering exists because compile reads the file. With the filter as store this collapses (see Apply semantics) |

## Recommended Architecture

```
                       static path (UNCHANGED, byte-for-byte guarded)
 config.json ---> Config{styles: StyleSet, rules} --emit_rules--+
 (styles inline)                                                +--> universal.filter
 prices/tiers --> pricer --> PricedData --emit_economy(&StyleSet)-+
                                                  ^
                       dynamic path (NEW)         | same economy emitters, now taking &StyleSet
 <game>/dynamic.filter --parse--> FilterDoc (CST) |
        ^                          |  +-- Block / Trivia / Region(Economy)
        |                          v
        |                  overlay: WorkingDoc ---typed adapter---> When/Rule editor (existing)
        |                          |
 styles.json --> StyleSet ---------+--> owned-action re-render
 tiers.json  --> ValidatedTiers ---+--> pricer
        |                          v
        +------ Apply: render(doc) with Economy region spliced -> atomic write
```

### New and modified components

| Component | New / Modified | Responsibility | Talks to |
|-----------|---------------|----------------|----------|
| `src/filter/` (`cst.rs`, `parse.rs`, `render.rs`, `grammar.rs`) | **NEW** | Lossless line-preserving parse + render of any `.filter`; typed view of known conditions/actions; sentinel region handling | `models::config` (`When`, via adapter), `models::styles` |
| `src/filter/adapter.rs` | **NEW** | Block <-> `Rule`/`When` bridge: "can this block be modelled exactly?", `lift`, and `apply_when_diff` (patch condition lines in place) | `filter::cst`, `models::config` |
| `src/filter/region.rs` | **NEW** | Find/replace/insert the `FERRISFILTER:ECONOMY` region | `filter::cst`, `emitter` economy emitters |
| `src/models/styles.rs` | **NEW** | `StyleSet` (ordered name->`Style`), `get`, `validate(rules)`, required-tier check; `styles.json` load/save | `models::config` (re-exports `Style`), `loader` |
| `src/models/config.rs` | **MODIFIED** | `Config.styles: Styles` -> `StyleSet`; delete the closed struct + FUTURE note | everything that names `Styles` |
| `src/emitter.rs` | **MODIFIED** | `resolve_style` becomes `styles.get(name)`; `style_for_tier` ditto; economy emitters take `&StyleSet`; economy emission exposed as `emit_economy(&StyleSet, &PricedData) -> String` | `StyleSet`, `pricer` |
| `src/compile.rs` | **MODIFIED** | Split `load_inputs` into config-free pieces: `load_economy(paths, policy) -> Economy{priced, price_source}`; keep `load_inputs` as `load_economy` + `Config` load (static path signature unchanged) | `pricer`, `loader` |
| `src/dynamic.rs` | **MODIFIED (shrinks)** | Delete `BASE_FILE`/`STORE_FILE`/`ensure_store`/`reset_store`/`copy_template`; keep `write_filter` and `default_destination`; add `refresh_economy(doc_text, styles, economy)` | `filter::region`, `emitter` |
| `src/overlay/store.rs` | **MODIFIED (rewritten)** | Load/save `FilterDoc` text, `styles.json`, `tiers.json`; resolve the app root; no `Config` JSON | `filter`, `models` |
| `src/overlay/edit.rs` | **MODIFIED** | `WorkingRules` -> `WorkingDoc` of `Entry` rows; `When` mutators untouched | `filter::adapter` |
| `src/overlay/rule_editor.rs`, `view_model.rs`, `app.rs` | **MODIFIED** | Row kinds (editable / partially editable / locked / economy), tab switcher, new Apply | `edit.rs`, new `config_tab.rs` |
| `src/overlay/config_tab.rs` | **NEW** | Styles + tier-cutoff editors with their own working copies, dirty/Revert, Apply | `models::styles`, `models::tiers` |
| `src/overlay/single_instance.rs` | **NEW** | Named mutex + wake signal | `win.rs` pattern, `visibility.rs` toggle path |
| `src/run.rs` / `cli.rs` | **MODIFIED** | `apply` operates on a filter file; `--reset`/`--base` removed | `dynamic`, `filter` |

## Design Question 1: CST that sits beside `When`/`Rule`

**Decision: a line-preserving CST, separate from `Config`. Do not extend `When` to carry arbitrary conditions.**

Why not replace/extend `When`: `When` is a *normalizing* model (it merges `Class "A"` and `Class == "A"`, drops comments, forces canonical order). Normalization is exactly what byte-identity forbids. It also cannot represent operators on `Class`/`BaseType`, repeated conditions, `Continue`, `Import`, unknown keys. It stays what it is: the policy model for the static path and the *typed lens* the overlay edits through.

**Shape (the filter grammar is line-oriented, so the CST is too):**

```rust
pub struct FilterDoc { pub items: Vec<Item>, /* BOM, dominant EOL, trailing-newline flag */ }
pub enum Item {
    Trivia(RawLines),                 // blank lines, comments, Import, anything outside a block
    Block(Block),
    Region(Region),                   // sentinel-delimited, e.g. Economy
}
pub struct Block { pub header: Line, pub lines: Vec<Line>, pub trailing_trivia: ... }
pub struct Line { pub raw: String /* incl. leading ws AND its original EOL */, pub kind: LineKind }
pub enum LineKind { Condition(Cond), Action(Act), Continue, Comment, Blank, Unknown }
```

Rules that make byte-identity structural rather than aspirational:

1. **`raw` is the source of truth; parsed fields are a cache.** Render = concatenate `raw`. An edit rewrites `raw` of *only* the touched line(s). A parser bug can then lose understanding, never bytes.
2. **Keep each line's own EOL** (CRLF/LF mixed files survive), BOM, tabs vs spaces, missing final newline.
3. **Block ownership of trivia:** comments/blank lines between blocks stay in `Trivia` items between blocks; comments inside a block stay in that block's `lines`. Moving a block (up/down arrows) moves its inner lines; the leading comment lines directly above a block (no blank gap) travel with it. State this convention once and test it.
4. **Unknown keyword = `Unknown` line, never an error.** The parser is total. `Import`, `Minimal`, `Continue`, `SetBorderColor`, unseen PoE2 conditions all parse (typed where known, `Unknown` otherwise) and render verbatim.
5. **Mandatory test: parse -> render == input** for every `.filter` in `reference-old-filters/`, `tests/fixtures/`, `dynamic.filter`, plus the user's FilterBlade/NeverSink file, plus proptest-style mutation (random CRLF/BOM/no-EOL). This is the milestone's acceptance gate and should be the first thing built.

**Typed grammar tables (`grammar.rs`)**: one place listing known condition keys (with value kind: numeric-ordinal, bool, string-list, rarity) and action keys. PoE2-valid values are assembled by research, but because unknown content is preserved, an incomplete table degrades gracefully to read-only display, not data loss. This de-risks the "official docs describe PoE1" issue.

## Design Question 2: How the overlay editor edits blocks it can model, and locks the rest

The editor is built around `When` + style name. Bridge it with an **exactness test**, not heuristics:

> A block is **Editable** iff `lift(block) -> Rule` succeeds and re-rendering the lifted `When` yields exactly the block's condition lines in canonical form (same keys, same operators, same values, no unknown conditions).

Three tiers (the UI already needs a row-kind concept; `view_model::RuleRow` gains `kind`):

| Kind | Criteria | UI | Edit write path |
|------|----------|----|-----------------|
| **Editable** | Exact lift; actions are FerrisFilter-owned (tagged) | Full existing accordion | Re-render owned lines |
| **Partial** | Some condition lines lift (e.g. `Class`, `Rarity`, `Sockets`), others are opaque (unknown condition, `Class` without `==`, `HasInfluence`, ...) | Typed controls for liftable fields; opaque lines listed as read-only chips; style shown as "custom actions" | `apply_when_diff` patches only changed condition lines; opaque lines and their positions untouched |
| **Locked** | Header not Show/Hide, `Import`, `Continue`-bearing, region content, or unparseable | Collapsed row with raw text view; enable/move/delete only | none (move/delete are whole-block ops) |

Key mechanics:

- **`apply_when_diff(block, old: &When, new: &When)`**: the existing mutators (`set_class`, `set_sockets_min`, ...) keep operating on a `When` copy. After each mutation, diff per field: unchanged field -> line untouched; changed -> rewrite that line's `raw` (preserving its indentation and trailing comment); newly set -> insert after the last condition line; cleared -> delete the line. This reuses all existing `edit.rs` logic and tests and never regenerates untouched lines.
- **Operator-sensitive fields:** `Class "Bows"` (substring) vs `Class == "Bows"` (exact) differ semantically. The lift must record the operator; if `When` cannot carry it, the line is opaque (Partial) until the user explicitly edits it, at which point the editor writes `==` and says so. Do not silently normalize.
- **Style identity for tagged blocks.** A filter cannot say "this block is style S". Recommended: FerrisFilter-written blocks carry a tag in the header comment, e.g. `Show # <name> [ff:S]` (consistent with the repo's `Show # Description (slug)` convention). For tagged blocks the **action lines are owned**: re-rendered from `styles.json` on Apply, so editing a style in the config tab propagates. For untagged (foreign) blocks the actions are preserved verbatim and the style picker is replaced by "custom" with an explicit **Adopt style...** action (replaces the block's action lines with a style's, tags it). Do not try to reverse-match actions to styles automatically as the primary mechanism: a styles.json edit would silently orphan every block.
- **`enabled: false`** has no filter-grammar equivalent. Represent as the whole block commented out with a recognisable prefix (e.g. `#~ `), recognised **only** with that prefix; foreign commented-out blocks stay inert trivia. Unit-test that a disabled block re-enables to byte-identical text.
- **Row identity:** `RuleId` (session) survives unchanged; `WorkingRules` becomes `WorkingDoc { rows: Vec<(RuleId, Entry)> }` where `Entry = Block | Trivia | Region`. Trivia rows are not shown as rows (they travel with neighbours); Region shows as one locked "Economy section" row (counts, last refresh).
- **Index-as-identity ends.** Dynamic rules were identified by array index; now identify by `RuleId` in session and by position+header at load. No persistent IDs are needed because the file itself is the store.
- **Large files:** a FilterBlade filter has hundreds of blocks. The accordion must virtualize (egui `ScrollArea::show_rows`/collapsed rows cost ~0) and offer a text filter on header/name; flag as a real UI risk for the editor phase. Do not expand all by default.

## Design Question 3: Locating and refreshing economy sections

**Decision: explicit sentinel comments around one generated region, spliced by byte range.**

```
# >>> FERRISFILTER ECONOMY BEGIN (generated; do not edit between markers)
# source: prices.json (live)  league: Forbidden Rites  tiers: tiers.json
...existing emit_currency_sections + emit_unique_sections output...
# <<< FERRISFILTER ECONOMY END
```

- **Why not detect by content** (`Show # Currency S` headings): fragile against hand edits and indistinguishable from a FilterBlade filter's own currency blocks. Explicit markers cannot false-positive on foreign filters.
- **Refresh algorithm:** parse -> find `Region(Economy)` (BEGIN..END lines) -> replace its inner text with `emitter::emit_economy(styles, priced)` -> render. Everything outside is byte-identical by construction. Idempotent: refreshing with unchanged prices is a no-op diff (test it).
- **Placement matters** (first-match-wins): economy blocks must sit *above* any broad hide/fallback that would swallow currency, below nothing specific. For a foreign filter with no region, **do nothing implicitly.** Offer an explicit "Insert economy section" action; default position at the end of the file, movable like any row (the Region is a row). The status line must say "no economy section in this filter" rather than silently skipping.
- **Foreign filter already has currency blocks** (FilterBlade always does): inserting ours does not conflict mechanically (first match wins), but the user should be told. Surface as an informational note, not a blocker.
- **Marker damage:** BEGIN without END (or duplicates) -> treat the half-marker lines as trivia, report "economy region damaged", refuse to refresh until fixed. Never guess a range.
- **What the region replaces from the old model:** `emit_header` text (which says "GENERATED ... edit the model, not this file") must **not** be written into user-edited filters; the header only appears when the tool creates a new file from scratch. New-file creation (replacing `--reset`): write a minimal header + empty Region; the user adds blocks in the editor. Decide explicitly whether to ship a starter template (see Open Questions).

## Design Question 4: `styles.json` vs `config.json` styles

- **One type, two homes.** `StyleSet` (ordered name -> `Style`) is used for `Config.styles` (config.json, static path) *and* `styles.json` (dynamic path). `config.json` keeps its inline `styles` object, same JSON keys and values, so no change to the file the byte-for-byte fixture depends on. `styles.json` initial content = a copy of config.json's `styles` object (the repo ships it committed; the overlay edits it in place; decide git-tracked vs ignored in Open Questions).
- **Ordering:** `HashMap` reshuffles (already a known quirk for `classGroups`). Use `serde_json`'s `preserve_order` feature or `indexmap` so the style picker and the config tab show S, A, B... in authored order. Do not use `BTreeMap` (alphabetical breaks the S..F ladder).
- **Validation at load (`StyleSet::validate`)**:
  1. Static path: every *enabled* `rule.style` resolves (keep `GenerateError::UnknownStyle` semantics and message: disabled-with-unknown-style stays inert; existing test `disabled_rule_with_unknown_style_is_inert`).
  2. Pricer-required tiers `S..F` plus `Recycle`-like names the economy emitters use must exist (`style_for_tier` currently assumes them). Missing -> load-time error naming the missing tier, replacing a possible runtime panic/unwrap.
  3. Dynamic path: tagged blocks (`[ff:X]`) whose `X` is absent are **reported, not fatal** (row marked "unknown style X", actions left as-is). A foreign filter must still open.
  4. Removing `deny_unknown_fields` on the closed struct is a behaviour change: a typo'd key (`"SS"`) is now a *new style*, caught only if used. Mitigation: `validate` warns on styles unused by any rule/tier in the static config? (Keep light: a warning list, not an error.)
- **Static-path safety:** `resolve_style` -> `styles.get(name)` is a pure refactor; the emitted bytes are unchanged. Run `artifact_guard` + `generate_snapshot` at the end of that phase before anything else lands. Tests that construct `Styles { s: ..., }` literals need a `StyleSet::from_pairs` helper; this is the main mechanical cost.
- **Config tab edits styles.json**, never config.json. The static path's styles remain edited by hand in config.json (v0.5 non-goal to unify).

## Design Question 5: Apply semantics when the filter is the store

**Where is the store?** Recommend: **the game-folder `dynamic.filter` itself** (path resolved via the existing `dynamic::default_destination()` / `install::default_dest()`), with an explicit "open another filter..." later. One file, no repo/game copy divergence, and a user can point the tool at the FilterBlade file they already installed. If the user's filter is that file, Apply rewrites it in place, so:

- **Backup before first write** per session (`<name>.filter.ffbak` next to it, once per launch). A lossless parser reduces the risk, but the file is the user's only copy now and Apply overwrites in game dir (OneDrive-synced, which gives some version history; do not rely on it).
- **Apply = render + atomic write.** `render(WorkingDoc)` is pure; economy refresh happens first (`refresh_economy` if a region exists and prices are loadable), then `write_filter`-style temp-file + persist **to the same path it was loaded from**. The "save store then compile from disk" two-step (and BLOCKER-1 hazard) disappears: there is one artifact and one writer. Keep the spirit of the guard: if render or write fails, `baseline` is not advanced and `dirty` stays set (existing `ApplyState` transitions reused as-is).
- **Revert** = re-parse the last applied text held in memory (`baseline_text`), not re-read the disk, so an external edit is not silently pulled in.
- **External modification detection:** record mtime+len (or a hash) at load/Apply; if it changed when Apply is pressed, block with "file changed on disk since loaded: Reload or Overwrite". This replaces the old model's guarantee that only the tool touched `dynamic.json`. It is new and important now that the user may also edit in FilterBlade/another editor.
- **SYNC ECONOMY** still writes `prices.json` only and is independent; the new "Refresh economy" is just part of the next Apply (no separate button needed; show region staleness in the stripe the way unapplied state is shown today).
- **CLI `apply`** becomes: parse filter, refresh economy region, write. `--reset`/`--base`/`--store` go away (breaking, document in CHANGELOG). `--tiers` override stays (feeds the pricer). Both CLI and overlay call one `dynamic::apply_text(...)` function.

## Design Question 6: Store paths and the launch-from-CWD bug (fix now, it is free)

The store location changes anyway, and the bug exists in three places (`store::default_paths`, `InputPaths::relative_to(store_dir)` in Apply, `sync_economy_blocking(&base_dir)`). Introduce a single **`AppRoot`** resolved once in `launch()`:

1. `FERRISFILTER_HOME` env var if set;
2. else the exe's directory **if** it contains `styles.json` or `tiers.json`;
3. else walk up from the exe dir (covers `target/debug/ff.exe` -> repo root) looking for `styles.json`;
4. else CWD;
5. else fail with a visible stripe message ("cannot find styles.json; set FERRISFILTER_HOME"), not a panic.

`AppRoot` yields `InputPaths::relative_to(root)` plus `styles.json`/`tiers.json`; every caller takes it as a parameter (the library already forbids implicit CWD reads; only the overlay's `default_paths` violated that in spirit). Unit-test resolution with temp dirs. This resolves the v0.4 "open at close" blocker as a side effect and costs about a day inside the store rewrite phase.

## Config tab architecture (feature 4)

- A **tab bar** in the stripe: RULES | CONFIG. Separate state so the rule editor's `dirty` is independent: `ConfigTabState { styles: StyleSet working + baseline, tiers: Vec<TierEntry> working + baseline }`.
- **Same discipline as rules:** in-memory edits, Revert = baseline clone, Apply persists. Apply writes `styles.json` / `tiers.json` atomically (reuse the `store::save` temp-in-same-dir pattern) **and** marks the filter "stale" because tagged blocks and the economy region derive from them. Cleanest UX: config Apply writes the two files, then the rules tab shows "unapplied changes" until its own Apply re-renders owned actions + economy. One tab's Apply must not silently write the game filter; but state clearly in the UI that the filter needs Apply.
- **Validation before enabling Apply:** `ValidatedTiers::try_from(vec)` already enforces S-F present and strictly ascending within a unit group, so the tab shows its error text inline and disables Apply. Style edits validated by `StyleSet::validate`. Deleting a style used by a tagged block: block count shown, require confirm.
- **Style editor surface** is bigger than it looks: text/border/background RGB, font size, minimap (size, colour, shape), beam (colour, temp), sound (alert id/volume or custom file), `DisableDropSound`, Show/Hide action. Build from the `Style` struct fields; reuse `widgets.rs`/`theme.rs`. Add/rename/delete style = new capability; renaming must also rewrite `[ff:X]` tags (or forbid rename in v0.5; recommended: forbid, offer duplicate+delete).
- **tiers.json unit trap:** `Unit` (chaos vs exalt) cutoffs are calibrated to the rate anchors; the tab edits `min` and shows the unit read-only, otherwise users can produce a set that passes validation but mis-tiers (the Divine-pinned-to-A incident in `with_tiers` docs).
- Edits to `tiers.json` while `prices-fallback`/`tiers-fallback` exist: the config tab edits `tiers.json` only; the fallback pair stays committed and untouched (stated requirement).

## Single-instance guard (feature 5)

Independent of everything else; fits in `launch()` **before** the tray/hotkey are built (so a second process never registers the hotkey or a second tray icon).

- **Detect:** named mutex via `windows-sys` (`CreateMutexW` + `GetLastError == ERROR_ALREADY_EXISTS`); adds the `Win32_System_Threading` feature to the existing `windows-sys` entry. `windows-sys` is already a dependency and `win.rs` is the declared single home of `unsafe`; put this in a sibling `win.rs` function or `single_instance.rs` and keep the same "all unsafe lives in win" rule. Use a `Local\` (per-session) name.
- **Wake the existing instance:** second instance signals first. Simplest robust option: a named auto-reset **event** the first instance polls each frame (`WaitForSingleObject(…, 0)`) and feeds into the same `toggle_requested`/reveal path as the hotkey in `visibility::advance`. No sockets, no new crate. `Reveal` not `Toggle` semantics: launching must show, never hide (add a "force reveal" input or use a distinct event; do not reuse the toggle edge).
- The hidden window has no repaint pump; the existing hotkey receiver already forces periodic repaint, so the event poll belongs next to it.
- Second instance exits with code 0 after signalling. Under the Windows GUI subsystem there is no console; no output needed.
- Test: logic (state transition on "reveal request") is unit-testable via `visibility.rs`; the mutex itself needs only a smoke test using two handles in-process.

## Data Flow Changes

| Flow | v0.4 | v0.5 |
|------|------|------|
| Load (overlay) | `dynamic_base.json` -> seed `dynamic.json` -> `Config` -> `WorkingRules` | `AppRoot` -> read filter text (+BOM/EOL) -> `FilterDoc` -> `WorkingDoc`; `styles.json` -> `StyleSet`; `tiers.json` -> working tiers |
| Edit rule | mutate `Rule.when` | mutate `When` copy -> `apply_when_diff` patches lines |
| Apply | save JSON -> compile from disk -> write game file | refresh region + re-render owned blocks -> render -> atomic write to the source path |
| Sync economy | writes `prices.json` in store dir | unchanged but takes `AppRoot` |
| Static generate | `Config` -> `universal.filter` | unchanged output; `Styles` -> `StyleSet` internally |

## Anti-Patterns to Avoid

- **Extending `When` into a universal condition model.** It normalizes; losslessness needs raw lines. Two representations (CST + typed lens) is the point.
- **Parse -> model -> regenerate the whole file.** That is v0.4's design and is exactly what byte-identity forbids. Edit lines in place.
- **Detecting economy blocks by content/heading.** Use sentinels; refuse to guess when damaged.
- **Auto-adopting styles from foreign actions.** Tag explicitly; otherwise styles.json edits silently orphan blocks.
- **Normalizing `Class "x"` to `Class == "x"` on load.** Semantic change; only on explicit user edit.
- **Rewriting a foreign file on mere open or on a no-op Apply.** Apply of an unchanged doc must produce identical bytes (and can skip the write).
- **Touching `config.json`'s serde attributes** to share types. Share `StyleSet`, keep field names/values.
- **Letting the config tab write the game filter.** Tab Apply writes styles/tiers only.
- **A second `current_dir()` call anywhere.** `AppRoot` or a parameter, nothing else.

## Suggested Build Order (dependency-driven)

1. **Phase A: Lossless CST (`src/filter/`) + grammar table + round-trip corpus tests.** No UI, no behavioural change. Acceptance: parse->render byte-identical across all repo filters and the user's FilterBlade/NeverSink file. *Highest risk, everything else depends on it; start with real files to settle PoE2 grammar.*
2. **Phase B: `StyleSet` + config-driven styles (static path).** Pure refactor of `Styles`/`resolve_style`/`style_for_tier`, `StyleSet::validate`, `styles.json` load/save, ship `styles.json`. Gate: `artifact_guard` + `generate_snapshot` still byte-identical. Independent of A, can run in parallel with it.
3. **Phase C: Economy region + `emit_economy` split + `compile` decoupling from `Config`.** Needs A (region parsing) and B (`&StyleSet` in emitters). Includes `refresh_economy` idempotence tests and the damaged-marker behaviour.
4. **Phase D: Store rewrite: `AppRoot`, filter-as-store, backup, mtime guard, new `apply` (CLI + library), delete `dynamic.json`/`dynamic_base.json`/seed/reset.** Needs A, B, C. Fixes the CWD bug here. CLI `apply` goes headless-first, then the overlay uses it (v0.4's headless-before-GUI discipline paid off; repeat it).
5. **Phase E: Overlay editor over `WorkingDoc`:** adapter (`lift`/`apply_when_diff`), row kinds, locked/partial UI, tagged-style ownership, disable-by-comment, large-file virtualization. Needs D. Largest UI phase; split in two: (E1) load/display/reorder/delete/enable of all rows with Editable only, (E2) Partial + Adopt style.
6. **Phase F: Config tab (styles + tiers).** Needs B and D; independent of E2.
7. **Phase G: Single-instance guard.** Independent; can slot anywhere (cheap, good as a palate cleanser between big phases).

Parallelisable: A || B; F after B+D; G anywhere.

## Research Flags for Phases

| Phase | Flag |
|-------|------|
| A | **Needs deeper research:** actual PoE2 condition/action list; behaviour of `Import`, `Continue`, `Minimal` in PoE2; real FilterBlade/NeverSink quirks (BOM, CRLF, trailing config blocks, `#` inside quoted strings, quoted values containing spaces/escapes) |
| B | Standard refactor; risk is mechanical test churn |
| C | Placement semantics for first insertion in a foreign filter; moderate |
| D | OneDrive atomic-rename behaviour and file-lock/sync races in the game folder (existing `write_filter` already handles cross-volume); mtime guard design |
| E | UI scale (hundreds of blocks); egui performance; operator-aware lift edge cases |
| F | Style editor surface size; rename semantics |
| G | Standard; verify Win32 named-event approach with the hidden-window repaint pump |

## Open Questions (for the roadmap/requirements owner)

1. **Store file location:** game-folder `dynamic.filter` (recommended) vs a repo-root working copy that Apply copies out. Latter keeps a pristine copy but reintroduces divergence; it also decides whether "open another filter" is a v0.5 feature.
2. **`styles.json` tracked or ignored?** It is user-edited via the tab (per-user state, like `dynamic.json` was) but needs a shipped default; recommend committing a `styles.default.json`-style seed or embedding the defaults via `include_str!` and git-ignoring `styles.json`. The brief says "repo-root styles.json"; confirm.
3. **Starter content for a brand-new dynamic filter** now that `dynamic_base.json` is gone: empty filter + region, or an embedded template (`include_str!`) of today's base rules? Losing the baseline is "intentional" per the brief, but a first-run user with an empty filter is a UX cliff.
4. **Existing users' `dynamic.json`:** one-time import (compile it once to `dynamic.filter` text, tag blocks) or drop? Not required by the brief; a CLI-only one-shot is cheap if wanted.
5. **Disabled-rule encoding** (`#~` prefix) is a new on-disk convention that other tools will see as comments; acceptable?

## Sources

- Code read directly (HIGH): `src/models/config.rs`, `src/emitter.rs`, `src/compile.rs`, `src/dynamic.rs`, `src/overlay/{store,edit,app,win,visibility,mod}.rs`, `src/models/tiers.rs`, `Cargo.toml`, `CLAUDE.md`, `.planning/PROJECT.md`.
- Official filter grammar (PoE1-oriented; PoE2 list to be assembled in Phase A): https://www.pathofexile.com/item-filter/about (not re-fetched this pass; LOW for PoE2 specifics).
- The parser shape, sentinel region, tag convention, and tab/Apply semantics are design recommendations (MEDIUM), to be validated against real FilterBlade/NeverSink files in Phase A.
