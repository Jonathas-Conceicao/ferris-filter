# Technology Stack — v0.7.2 additions

**Project:** FerrisFilter (v0.7.2 Small UI Improvements)
**Researched:** 2026-10-23
**Overall confidence:** HIGH (every API below read from the crate sources in `~/.cargo/registry/src`, versions from `Cargo.lock`)

## Verdict

**No new Rust dependencies are needed.** All 8 features are buildable with the current
`Cargo.toml`. The author's belief holds. Nothing goes in the "NEW DEPENDENCY" section (it is
empty, see the end). Do not add `keyboard-types` (global-hotkey re-exports `Code` and
`Modifiers`), `serde_with`, `chrono`/`time`, `egui_dnd`, `egui_extras`, `rdev`, or `device_query`.

Locked versions that matter: egui/eframe 0.36.1, global-hotkey 0.8.0, keyboard-types 0.7.0
(transitive), winit 0.30.13 (transitive). Note `~/.cargo/registry` also holds 0.36.2 sources;
Cargo.lock pins 0.36.1, which is what was read for the API names below. If `cargo update` is
ever run, re-check nothing in the dnd/scroll APIs changed (a patch bump should not).

## Per-feature: how to build it with what exists

### 1. Style colour alpha — serde + existing picker (no new dep)
- `src/skin/colour_picker.rs` already has an `alpha: bool` mode (used by rule colour actions;
  styles pass `false` today) and returns unmultiplied RGBA. Feature = pass `alpha: true` for
  style slots and carry the 4th byte through `Style` (`src/models/config.rs:243`).
- Persistence: add an optional alpha next to each colour, written **only when not 255**
  (`#[serde(default, skip_serializing_if = ...)]`), so existing `styles.json` files load and
  re-serialise byte-identically (the project already guarantees this for other new fields).
  Emitter: write `R G B A` with the alpha instead of the hard-coded 255, and keep the existing
  3-token behaviour for frozen seeds (`tests/fixtures/seed-styles.json` must still produce
  `seed-universal.filter` byte-for-byte; `artifact_guard`).
- Decide the JSON shape in the phase (extra `*Alpha` fields vs 4-element array); both are plain
  serde. Prefer extra optional fields, they cannot break old readers/round-trips.
- Confidence: HIGH.

### 2. Auto Detect styles — pure Rust over the existing parser (no new dep)
- Reuse the lossless filter parser/`style_lines.rs` "last line of each keyword family wins"
  binding to build a `Style` per rule; compare with `==` (derive `PartialEq` on `Style`, ensure
  `Option`/alpha/sound fields are all included), group with `std::collections::HashMap` or the
  existing `indexmap` for deterministic "Custom Style N" numbering by first appearance.
- Custom style ids: reuse the existing random-id generator (6-8 lowercase alnum); no `rand`
  needed (it already exists for custom styles/groups).
- Confidence: HIGH (no external API involved).

### 3. Drag-and-drop reordering — egui 0.36.1 primitives, but do NOT use `dnd_drag_source`
Verified APIs in egui 0.36.1: `Ui::dnd_drag_source(id, payload, add_contents)`
(`ui.rs:2641`), `Ui::dnd_drop_zone` (`ui.rs:2693`), `Response::dnd_set_drag_payload` /
`dnd_hover_payload` / `dnd_release_payload` (`response.rs:498/515/531`),
`DragAndDrop::{payload, take_payload, clear_payload, has_payload_of_type}`
(`drag_and_drop.rs`), `Context::is_being_dragged`, `Context::set_dragged_id`,
`Context::pointer_interact_pos`, `Context::transform_layer_shapes`,
`Context::layer_painter`, `Ui::scroll_with_delta`, `Ui::scroll_to_rect`,
`ScrollArea::vertical_scroll_offset` / `scroll_source(ScrollSource{..})`.

Recommendation (opinionated):
- **Drag source:** put a `Sense::drag()` interact on the handle rect only
  (`ui.interact(handle_rect, id, Sense::drag())`), draw the three handle PNGs
  (normal/hover/pressed) from `response.hovered()` / `response.is_pointer_button_down_on()` /
  `dragged()`. Use `response.drag_started()` to record `DragState { from: usize, grab_offset }`
  in app state (own struct, not egui memory) and `ctx.set_cursor_icon(Grabbing)`.
- **Why not `dnd_drag_source`:** it paints the body into a Tooltip layer and moves it with
  `transform_layer_shapes`, which fits non-virtualized lists. Here the list is virtualized
  (`src/overlay/virtual_list.rs`: `build_offsets`, `visible_range`), so the dragged row can
  scroll out of the visible range and stop being built. egui tolerates that (the widget being
  dragged "is now in the air and not registered", `interaction.rs:124-131`), but any logic that
  lives inside the row's closure stops running. Keep the drag state and the drop logic outside
  the row closure, keyed by the row's stable id/index, and finish the drag from the
  `ctx.input(|i| i.pointer.any_released())` check once per frame at the list level.
- **Shadow copy:** paint the row (or a cheap re-draw: plate + name + handle) with
  `ctx.layer_painter(LayerId::new(Order::Tooltip, Id::new("ff-drag-shadow")))` at
  `pointer_interact_pos() - grab_offset`, with reduced opacity (`Color32::gamma_multiply` /
  `Painter::set_opacity`). Tooltip order draws over everything and is not interactive.
- **Drop marker:** compute the target index from the pointer y against the existing `offsets`
  vector (the same one `visible_range` uses: `partition_point` over offsets, snap to the nearest
  row boundary); paint a line/bar at `offsets[target]` with `ui.painter()`. No hit-testing of
  rows needed, so it works for rows that are not currently built. Skip no-op targets
  (`target == from || target == from + 1`).
- **Wheel scroll while dragging:** the `ScrollArea` already consumes wheel events; while a drag
  is active the wheel still reaches it unless the area is told otherwise. Verify in a spike; if
  the pointer capture blocks it, read `ctx.input(|i| i.smooth_scroll_delta)` and feed it to
  `ui.scroll_with_delta(...)` yourself.
- **Edge auto-scroll:** inside the scroll area's `ui`, when `pointer_pos.y` is within ~32pt of
  the viewport top/bottom, call `ui.scroll_with_delta(vec2(0.0, ±speed * dt))` with
  `dt = ctx.input(|i| i.stable_dt)` and `ctx.request_repaint()`. Speed scales with proximity to
  the edge. Since the project drives offsets itself (`area.vertical_scroll_offset(o)` at
  `src/skin/widgets.rs:1578`), the alternative is to mutate that stored offset directly; pick
  whichever the existing helper makes simpler.
- **Section/economy constraints:** rule drag moves one "unit" (a whole `# ff:group` run), must
  not cross the economy section, and the reorder itself should call the existing move
  operations (same model path as the arrows) so Continue-flags, unapplied state and Revert
  behave identically. No new model code beyond "move rule from i to j".
- **Custom art:** `assets/custom/icon_reoder_{normal,hover,pressed}.png` through the existing
  `include_bytes!` + `skin::textures` path (LINEAR + mipmaps, pixel-snapped). Note the
  filenames contain the typo `reoder`; the files are untracked in git right now, add them.
- Confidence: HIGH on API names (read from source), MEDIUM on the wheel-during-drag detail
  (needs an in-app spike).

### 4. Social-panel section headers — ff-art (dev tool, already a workspace member)
- Art extraction uses the existing `tools/ff-art` manifest (`assets/poe2/manifest.toml`): add
  entries for the toggle plate / up-down glyph / stone bar; `[[atlas]]` packing exists if the
  glyphs live in a grid sheet. `ff-art dump`/`sheet`/`zoom` find the source DDS. Needs the game
  installed and the pinned nightly; output PNGs are committed, so the shipped `ff` build needs
  nothing new. Paint with the existing nine/three-slice helpers in `src/skin/slice.rs`.
- No new dependency in either crate (`image-extras` DDS, `resvg` already present in ff-art).
- Confidence: HIGH for tooling, LOW on which game file holds the Social panel art (discover
  with `ff-art list`).

### 5. Clear (X) button and magnifier inside search fields — painting only
- egui `TextEdit` plus painting into the field rect: reserve right/left margin
  (`TextEdit::margin`), paint the magnifier with the existing `skin::icons` pattern (either a
  small PNG via `include_bytes!` or two painter primitives: `circle_stroke` + `line_segment`),
  and an X via the existing `ff_x_button`. Show X only when the text is non-empty; click clears
  and `request_focus()`. Both are wrappers around the existing `ff_text_field_*`.
- Confidence: HIGH.

### 6. User-customizable global hotkey — global-hotkey 0.8.0 + egui key events
Verified in `global-hotkey-0.8.0`:
- `HotKey::new(Option<Modifiers>, Code)`; `id = (mods.bits() << 16) | key as u32`. **The id
  changes with the combo**, so `HotkeyStatus::Registered { id }` and the poll in
  `src/overlay/app.rs:3501` (matches `event.id == *id`) must be updated on every re-register.
- `impl FromStr for HotKey`, `Display`/`into_string()` (`"shift+control+alt+super+<Code>"`,
  e.g. `control+shift+KeyZ`), plus optional `serde` feature (not enabled; do not enable, a
  plain `String` field is enough and parse errors become a warning like the other keys).
  Parser accepts `"Ctrl+Shift+Z"`, `"KeyZ"`, `"Z"`, `"F5"`, `"Digit1"`, etc.
- `GlobalHotKeyManager::register/unregister(hotkey)`; Windows impl calls
  `RegisterHotKey(..., MOD_NOREPEAT | mods, vk)` / `UnregisterHotKey` on the manager's hidden
  window, so **runtime re-registration is supported**, on the same thread that created the
  manager (the existing note in `hotkey.rs`). The overlay's update loop runs on the
  eframe/winit thread that built it, so call it from there. `ERROR_HOTKEY_ALREADY_REGISTERED`
  surfaces as an `Err`, which maps to the existing `HotkeyStatus::Failed(text)`.
- Re-register algorithm (safe order, keeps the old combo on failure): `register(new)`; on
  `Ok` then `unregister(old)` and swap the id; on `Err` leave the old one and show the error.
  If new == old, no-op. Never `register` the manager twice (do not call
  `GlobalHotKeyManager::new()` again at runtime; keep the one in `hotkey_manager`).
- **Capture in the Config tab:** no new crate. While the "press a key" control is armed,
  read `ctx.input(|i| i.events.clone())` for `egui::Event::Key { key, physical_key, pressed:
  true, repeat: false, modifiers, .. }` and take `modifiers` (egui `Modifiers`: `ctrl`, `shift`,
  `alt`, `mac_cmd`/`command`) from the event. Ignore events whose key is only a modifier (egui
  has no modifier `Key`, so lone modifiers produce no `Key` event, which is what you want).
  `Escape` cancels; `Backspace`/`Delete` could reset to default `Ctrl+Shift+Z`.
  Use `physical_key.unwrap_or(key)`: `RegisterHotKey` takes a virtual key derived from `Code`, and
  `physical_key` is layout-independent (documented as "not recommended" for text shortcuts but
  appropriate for a hotkey that is stored as a physical `Code`). Be aware global-hotkey maps
  `Code` to a VK (so on AZERTY a `KeyA` physical stores the VK of that position); keep the
  hint text honest ("key position").
- **egui `Key` -> `global_hotkey::hotkey::Code` mapping:** there is **no built-in conversion**
  (egui `Key` is its own enum; `Event::Key.physical_key` is also an egui `Key`). Write one
  small `match` (~80 lines) in `src/overlay/hotkey.rs`: `Key::A..Z -> Code::KeyA..KeyZ`,
  `Num0..9 -> Digit0..9`, `F1..F20 -> F1..F20`, arrows, `Space`, `Tab`, `Enter`, `Insert`,
  `Delete`, `Home`, `End`, `PageUp`, `PageDown`, `Backspace`, punctuation (`Minus`, `Equals`,
  `Comma`, `Period`, `Slash`, `Backslash`, `Semicolon`, `Quote`, `Backtick`, `OpenBracket`,
  `CloseBracket`). Return `Option<Code>` and say "that key is not supported" otherwise. Cheaper
  trick, also verified: `Key::name()` strings often match `parse_key` tokens
  (`"A"`, `"F1"`, `"Insert"`), but names such as `Down`/`Left`/`PageUp`/`Minus` do not match
  global-hotkey's expectations (`ArrowDown`, `PageUp`, `Minus`...), so the explicit `match` is
  more reliable and unit-testable. Put a table test over all supported keys that round-trips
  `HotKey::new(mods, code).to_string().parse::<HotKey>() == Ok(hotkey)`.
- **Validation rules (suggest):** require at least one of Ctrl/Alt/Shift/Super unless the key
  is an F-key (a bare letter would be swallowed system-wide, which is exactly why the author
  moved off plain `Shift+Z`; see the comment in `hotkey.rs`). Reject keys the game or the app
  needs? Not enforceable; just warn.
- **While capturing, unregister the live hotkey** (or at least ignore its events), otherwise
  pressing the current combo toggles the window instead of being captured (RegisterHotKey
  swallows the chord before egui sees it). Re-register the old one if capture is cancelled.
- **Persistence:** new `ff-settings.json` key (e.g. `toggle_hotkey: "control+shift+KeyZ"`),
  `Option<String>` in `Settings` with the same `skip_serializing_if` pattern, saved with a new
  `save_keys(&["toggle_hotkey"])` wrapper like `save_league`. Absent or unparsable reads the
  default with a warning (follow the existing "wrong type reads the default with a warning"
  convention). Unknown keys are preserved by the flatten `extra` map already.
- **Tray/tooltip text** that mentions `Ctrl+Shift+Z` must read from the same value; `hotkey.rs`
  constants `TOGGLE_MODIFIERS`/`TOGGLE_CODE` become the default only. `release_toggle` must
  unregister the *current* combo, not the constant.
- **Screenshot mode and tests** use no hotkey; unit-test the key mapping, settings round trip
  and the register-order logic behind a small trait or a pure function (do not call the OS in
  tests; the isolation guard applies).
- Confidence: HIGH on global-hotkey APIs, MEDIUM on layout behaviour of `physical_key`
  (verify on a non-QWERTY layout or accept QWERTY-position semantics).

### 7. Tier price range in economy names — pure pricer logic
- `pricer.rs` already has the tier cutoffs (`tiers.json`, exalt/chaos/divine units, derived
  rates). Format the lower/upper bound of a tier with the existing value formatter; no crate.
  Names are written as `# ff:name <T> tier <Group>` today; the suffix changes the name text, so
  check the parser/`styled.rs` places that match names by prefix (`S tier Currency`) and the
  committed snapshot tests (`generate_snapshot`, `artifact_guard` only if `generate` shares the
  code path; the seed fixture must not change unless intended).
- Confidence: HIGH.

### 8. Network throttle — std + serde, and a distinct User-Agent
- Persist timestamps as **unix seconds (`u64`)** from `std::time::SystemTime::now()
  .duration_since(UNIX_EPOCH)` (same pattern as `src/sync/prices.rs:611`). No `chrono`/`time`.
  Do not use `Instant` (not persistable across restarts). Treat a stored time in the future
  (clock moved back) as expired, never as "blocked for years": `now.saturating_sub(last) >= 3600`
  is wrong for future stamps (it returns 0), so write `last > now || now - last >= 3600`.
- Where to store: a new key in `ff-settings.json` (e.g. `net_last_request` map of key to
  seconds, keys `sync:<league slug>`, `leagues`, `update`) with its own `save_keys` wrapper,
  OR a separate `net-state.json` in the FerrisFilter folder. Prefer `ff-settings.json`: one
  atomic-write path exists. Caveat: a damaged `ff-settings.json` already means reload-off and a
  warning; throttle state in a file that cannot be saved should fail **open for display but
  closed for requests**? Recommend: if the throttle state cannot be read or saved, still allow
  the request (an offline-safe, honest choice) but keep an in-memory timestamp so a session
  cannot spam.
- `prices.json` already holds `fetched_at` (sync time, `sync/prices.rs`); do not derive the
  throttle from it, because a failed request must also throttle, and the CLI writes it too.
  The throttle is overlay-only; `ff-cli sync` and `generate` ignore it.
- UI behaviour: a throttled button should not just go grey silently. Show the remaining
  minutes on hover and in the status line (existing `StatusKind::Info`), and offer no bypass
  unless the author wants one (an explicit decision to take in the phase).
- League list: throttle the network call but still serve the last list from memory; today the
  list is fetched once per session, so persist the *time*, and consider also persisting the
  list names to the settings so a restart inside the hour has something to show
  (small `Vec<String>`; otherwise the dropdown is empty until the hour passes).
- **Distinct User-Agent:** `src/sync/client.rs:19` defines the single `USER_AGENT`, used at
  `client.rs:48` (poe.ninja `ReqwestClient`) and `update.rs:841` (GitHub). Make the UA a
  parameter: e.g. `pub const USER_AGENT_OVERLAY` and `USER_AGENT_CLI` built with `concat!`
  (`"FerrisFilter/<ver> (overlay; github.com/...; contact: ...)"` vs `"... (cli; ...)"`), and
  give `ReqwestClient::new(user_agent)` (or a builder) the choice; the overlay's sync worker,
  league fetch and update check pass the overlay one, `ferrisfilter`/`ff-cli` pass the CLI
  one. Update the existing `user_agent_tests` (it asserts prefix, repo name, no "myfilter") to
  cover both. This is reqwest's `ClientBuilder::user_agent`, already used.
  The GitHub update check from the CLI does not exist, so only `ff` uses the overlay UA there.
- Confidence: HIGH.

## Recommended stack (unchanged)

| Technology | Version (Cargo.lock) | Role in v0.7.2 |
|------------|----------------------|----------------|
| egui / eframe | 0.36.1 | DnD primitives, painter layers, key events, scroll control |
| global-hotkey | 0.8.0 | `HotKey`, `FromStr`/`Display`, register/unregister at runtime, `Code`/`Modifiers` re-exports |
| serde / serde_json | 1.0.228 / 1.0.154 | optional alpha fields, new `ff-settings.json` keys |
| std `SystemTime` | n/a | persisted throttle timestamps |
| reqwest | 0.13 | `user_agent` per client |
| indexmap | 2 | ordered "Custom Style N" grouping |
| ff-art (dev tool) | workspace | Social-panel art extraction |

## What NOT to add

| Do not add | Why |
|------------|-----|
| `egui_dnd` / `egui_extras` | Not needed; they assume non-virtualized lists, and the project has its own virtual list and skin. |
| `keyboard-types` as a direct dep | `global_hotkey::hotkey::{Code, Modifiers}` re-export it (`hotkey.rs:30`). |
| `rdev`, `device_query`, `inputbot` | Global capture is unnecessary: the chord is captured from egui key events while the overlay has focus. |
| `chrono` / `time` | Unix seconds via `SystemTime` is enough. |
| `rand` | Existing id generation is reused. |
| `serde_with` | Plain `Option` + `skip_serializing_if` is enough. |
| `global-hotkey` `serde` feature | A `String` field is simpler and keeps warnings consistent. |

## NEW DEPENDENCY — NEEDS AUTHOR REVIEW

**None.** Every feature is covered above. Nothing in this research required a new crate.
If the dnd spike (feature 3) shows that wheel scrolling cannot be made to work during a drag
with `scroll_with_delta`, the fix is still in egui (read `smooth_scroll_delta` and apply it);
no crate would be proposed without coming back to the author first.

## Risks and open questions for the phases

1. **DnD in a virtualized list** is the only feature with real technical uncertainty (wheel
   during drag, drag continuing when the source row leaves the visible range). Plan a short
   spike first. Verify with `cargo run --bin ff -- --screenshot` where possible; real pointer
   input needs a manual check (numbered list per the author's preference).
2. **Hotkey `Code` vs layout:** decide and document key-position semantics.
3. **Settings write path:** every new key needs its own `save_keys` wrapper so a fold, league
   pick or hotkey change never rewrites unrelated keys; add tests to `settings.rs` mirroring
   `save_league`.
4. **Alpha and the frozen oracle:** keep `seed-styles.json` untouched; alpha defaults must
   emit exactly the old text.

## Sources

- egui 0.36.1 source: `src/ui.rs` (dnd_drag_source/dnd_drop_zone, scroll_with_delta),
  `src/drag_and_drop.rs`, `src/response.rs`, `src/interaction.rs`, `src/containers/scroll_area.rs`,
  `src/data/input/event.rs` (`Event::Key` with `physical_key`), `src/data/key.rs` (`Key::name`)
  — HIGH (local registry sources).
- global-hotkey 0.8.0 source: `src/hotkey.rs` (`HotKey::new`, id, `FromStr`, `parse_key`),
  `src/lib.rs` (`GlobalHotKeyManager`), `src/platform_impl/windows/mod.rs` (`RegisterHotKey`,
  `UnregisterHotKey`, `MOD_NOREPEAT`) — HIGH.
- Repo: `Cargo.toml`, `Cargo.lock`, `src/overlay/hotkey.rs`, `src/overlay/app.rs` (hotkey poll),
  `src/settings.rs`, `src/sync/client.rs`, `src/update.rs`, `src/skin/colour_picker.rs`,
  `src/overlay/virtual_list.rs`, `tools/ff-art/Cargo.toml` — HIGH.
