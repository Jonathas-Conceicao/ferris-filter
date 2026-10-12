# Technology Stack (v0.5 additions)

**Project:** FerrisFilter v0.5 Generic Filter Editor & Initial App Configuration
**Researched:** 2026-10-12
**Scope:** Only NEW capabilities. Existing deps (clap, serde, eframe/egui 0.36, tray-icon, global-hotkey, tempfile, windows-sys 0.61) are unchanged.

## Headline Recommendation

**Add zero new crates.** The only change is widening `windows-sys` feature flags. The parser is hand-written std Rust, the single-instance guard uses the already-present `windows-sys`, and the config tab uses egui's built-in color picker.

## Recommended Stack

### 1. Lossless filter parser: hand-written, line-oriented, std only

| Technology | Version | Purpose | Why |
|------------|---------|---------|-----|
| Hand-written lexer/parser (new module, e.g. `src/filter/`) | n/a | Parse any .filter into a lossless line-based document model | See rationale below |

**Why hand-written rather than winnow 1.0.4 / logos 0.16.1 / nom / pest / chumsky:**
- The PoE filter grammar is **line-oriented** (one directive per line, keyword + operator + whitespace-separated tokens, `#` comments to EOL, quoted strings). There is no nesting and no expression grammar. A parser-combinator framework buys almost nothing.
- The hard requirement is **byte-identical round-trip** (CRLF, odd spacing like `#Show  # $type->...`, commented-out blocks, trailing whitespace, mixed indentation, missing final newline). Combinator libs and pest discard whitespace/trivia by default, and you would fight them to keep it. A hand-written model keeps the raw text trivially.
- Confidence: MEDIUM-HIGH (reasoned from the grammar plus the stated byte-identity requirement; crate versions verified via `cargo search`, winnow 1.0.4, logos 0.16.1).

**Recommended design (the key to losslessness):**
1. Split the input into lines with a splitter that **keeps each line's terminator** (`\r\n`, `\n`, or none). Never use `str::lines()`, which discards them.
2. Every line becomes a node holding `raw: String` (the exact original bytes, terminator included) plus an optional parsed view (`Option<Directive>`).
3. Serialization = concatenate `raw` for unmodified nodes. Only an **edited** node regenerates its text, and it should reuse the original indentation, the original terminator, and the original trailing comment where possible. Unedited content therefore cannot drift.
4. Anything unrecognized (unknown keyword, malformed operator, commented-out blocks, `Import`, stray text) is stored as an opaque raw node. This gives "unknown content preserved" for free.
5. A commented-out block (`#Show ...`) is just a comment line. Do **not** try to parse it. Optionally expose "disabled rule" later as a view over comment lines.
6. Group lines into blocks: a block starts at a `Show`/`Hide`/`Minimal`/`Continue`-bearing header line and runs to the next header. Interstitial comments and blank lines belong to the preceding block's trailing trivia, or to a file-level preamble.
7. Treat the file as bytes-or-UTF-8 with care. Use `std::fs::read` plus `String::from_utf8`. A BOM must be preserved (keep it as part of the first line's raw text). On a non-UTF-8 file, fail loudly rather than lossy-convert.
8. **Byte-identity test:** property/fixture test `serialize(parse(x)) == x` over the real 258KB `filterblade.filter` (copy a trimmed or full fixture into `tests/fixtures/`), plus synthetic edge cases (CRLF, LF, mixed, no trailing newline, BOM, tabs).

**Token-level helper:** a ~50-line tokenizer for a directive line (split on whitespace while honoring `"quoted strings"`, strip a trailing `#` comment outside quotes) is enough. Keep the comment's exact leading whitespace so it can be re-emitted.

### 2. Windows single-instance: raw `windows-sys` (already a dependency)

| Technology | Version | Purpose | Why |
|------------|---------|---------|-----|
| windows-sys | 0.61 (current 0.61.2, already in Cargo.toml) | Named mutex for the guard plus named event for the "show yourself" signal | No new dependency, small FFI surface, and it avoids the `single-instance` crate's limitations |

**Feature flags to add** to the existing `windows-sys` line: `Win32_System_Threading` (CreateMutexW, CreateEventW, SetEvent, OpenEventW, WaitForSingleObject) and `Win32_Security` (SECURITY_ATTRIBUTES parameter type, passed as null). `Win32_Foundation` (GetLastError, ERROR_ALREADY_EXISTS, HANDLE) is already enabled. Verify exact module paths at compile time. Windows-sys moves items between features occasionally, so treat the feature list as MEDIUM confidence and let `cargo check` confirm.

**Mechanism:**
- Startup in `ff`: `CreateMutexW(null, FALSE, L"Local\\FerrisFilter.ff.single-instance")`. If `GetLastError() == ERROR_ALREADY_EXISTS`, we are the second instance. Use the `Local\` namespace (per session) so other users or sessions are unaffected.
- Signal channel: a named **auto-reset event** `Local\FerrisFilter.ff.show`. The first instance creates it and spawns a small watcher thread looping `WaitForSingleObject(event, INFINITE)`. On wake it sends the show command into the **same channel the global-hotkey and tray handlers already use** (e.g. `ctx.request_repaint()` plus `ViewportCommand::Visible(true)`/Focus), so integration reuses the existing "summon stripe" path. The second instance calls `OpenEventW` + `SetEvent`, then exits 0.
- Keep the handles in an RAII wrapper held for the process lifetime (`CloseHandle` on drop). The mutex must live in `main`, not a temporary.
- Race: if the second instance signals before the first has created the event, create the event in the first instance **before** (or atomically with) the mutex acquisition, or have the second instance use `CreateEventW` too (creating by the same name is idempotent). Using `CreateEventW` on both sides removes the race entirely.

**Why not the alternatives:**
| Alternative | Why not |
|-------------|---------|
| `single-instance` 0.3.3 | Only gives the "is another running" boolean (named mutex). No signalling, so you would still need your own IPC. Little value over 10 lines of FFI. |
| `interprocess` 2.4.5 (named pipe) | Real IPC is overkill for a one-bit "show" message. Pulls in more deps. Consider only if you later need to pass payloads. |
| `windows` 0.62.2 crate | Heavier and duplicates `windows-sys`, which is already depended on. |
| `FindWindow` + `PostMessage` | Fragile with a hidden/tool window and winit-owned windows, and needs a custom message hook in winit. The event approach is independent of the UI toolkit. |
| `fs2`/lock file | Does not signal the existing instance, and stale-lock handling is annoying. |

Confidence: HIGH on the technique (standard Win32 pattern), MEDIUM on exact feature-flag names.

### 3. Styles/tiers editing in egui: built-in only

| Technology | Version | Purpose | Why |
|------------|---------|---------|-----|
| egui `color_edit_button_srgba` / `ui.color_edit_button_srgba_premultiplied` / `egui::color_picker` | 0.36 (existing) | RGBA pickers for text/border/background | Built in, no crate needed |
| `egui::DragValue`, `ComboBox`, `Checkbox` | 0.36 | Font size, sounds, beam/minimap enums, tier cutoffs | Built in |

- Use `Color32` (u8 sRGB). Style colors in the model are `R G B A` u8, so the conversion is lossless. Avoid the `f32` `Rgba` or HSVA variants for storage, to prevent rounding drift on RGB values like `255 121 198`.
- Persist via serde in `styles.json` and `tiers.json` as already planned, with `tempfile` atomic writes (existing). Reuse the Phase 14 in-memory edit / Apply / Revert pattern.
- Config-driven styles: use `BTreeMap<String, Style>` (or `indexmap::IndexMap` **only if** you need user-defined ordering in the picker). Default recommendation is `BTreeMap` for no new dep, with an explicit `order: Vec<String>` field if the tier ladder (S..F) needs a stable display order. Validate at load (unknown style referenced by a rule is a hard error, as it is today).
- Avoid `egui_extras` or third-party color-picker crates. Not needed.

## What NOT to Add

| Crate | Reason |
|-------|--------|
| winnow / nom / pest / chumsky / logos | Grammar is line-based and trivia preservation is the core need. They add friction for byte-exactness. |
| rowan / lossless-syntax-tree crates | Designed for nested CSTs. Overkill for flat lines; also heavy API. |
| single-instance, interprocess, windows (crate) | See above. |
| indexmap | Only if style ordering genuinely requires it. Prefer BTreeMap plus an order list. |
| encoding_rs / chardet | PoE filters are UTF-8. Fail loudly on non-UTF-8 instead. |
| proptest | Optional dev-dep. Fixture plus hand-written edge cases suffice. Add only if you want fuzzed round-trips. |

## Installation

```toml
# Cargo.toml -- only change
[target.'cfg(windows)'.dependencies]
windows-sys = { version = "0.61", features = [
  "Win32_Foundation",
  "Win32_UI_WindowsAndMessaging",
  "Win32_System_Threading",   # CreateMutexW, CreateEventW, SetEvent, WaitForSingleObject
  "Win32_Security",           # SECURITY_ATTRIBUTES type used by the Create* signatures
] }
```

## Integration Notes

- Single-instance check belongs at the very top of `ff`'s `main` (before `overlay::launch()` builds the eframe window), gated with `#[cfg(windows)]`. On other platforms it is a no-op.
- The watcher thread needs an `egui::Context` clone, which is available once eframe's creation callback runs. Build the thread there, and pass it the same show-stripe sender the hotkey uses.
- The hotkey foreground gating (only when PoE2 or the stripe is foreground) is independent; the second-launch signal should show the stripe unconditionally.
- Parser module is pure std and belongs in the lib crate, so `assert_cmd`/`insta` tests can cover it without the GUI.

## Sources

- `cargo search` against the crates.io index on 2026-10-12: single-instance 0.3.3, windows-sys 0.61.2, windows 0.62.2, interprocess 2.4.5, winnow 1.0.4, logos 0.16.1 (HIGH, versions)
- Local registry confirms egui 0.36.1, windows-sys 0.61.2 already resolved (HIGH)
- Win32 named mutex / event semantics, from training knowledge of the Win32 API (MEDIUM; not re-fetched, feature-flag names to be confirmed by `cargo check`)
- Parser-design rationale: derived from the stated requirements and the filter grammar in CLAUDE.md (MEDIUM; design judgment, not external source)
