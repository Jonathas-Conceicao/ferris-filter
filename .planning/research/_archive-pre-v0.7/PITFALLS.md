# Domain Pitfalls (v0.5)

**Domain:** Lossless filter parse/edit, filter-as-rule-store, config-driven styles, overlay config tab, single-instance guard, added to an existing deterministic Rust compiler plus egui overlay
**Researched:** 2026-10-12
**Scope:** Only pitfalls new to v0.5. The v0.4 pitfalls (egui transparency defects, no hot-reload, windowed mode, atomic writes, ordinal identity, hardcoded class lists, fixture regeneration from the repo root) are in `_archive-pre-v0.5/PITFALLS.md` and are not repeated. Exceptions are called out where v0.5 re-triggers them.
**Confidence:** HIGH on facts measured from the real sample file. MEDIUM on Win32 mutex behavior (well documented, not run on this machine). LOW on NeverSink/FilterBlade grammar edge cases not present in the one sample.

## Facts measured from the sample (`filterblade.filter`, 258,094 bytes)

These shape every parser pitfall below:

- Pure CRLF: 5312 `\r` and 5312 `\n`, and the file ends with `\r\n`. A LF-only writer changes every line.
- Tab indentation: 3768 lines contain a tab (conditions and actions are tab-indented). Preserve the exact indent string per line, never re-indent.
- 460 `Show`/`Hide` header lines, including commented-out ones.
- Commented-out blocks look like `#Show  # # %D9 $type->chancing $tier->chances !apex_stier`, with two spaces and a doubled `#`. Their bodies are also commented (`#<tab>Continue`). Some blocks are live and some are commented in the same section.
- `Continue` is used on its own line in live blocks (lines 604, 637, 652, ...).
- 2 lines have trailing whitespace. Lines such as `#   [2905] Tier 3 uniques $` contain `$` and `[...]` inside comments.
- `BaseType == "Gold"` and `StackSize >= 5000` use spaced operators. No `Import` was found in this sample, but other FilterBlade/NeverSink outputs can have one.
- The header has a long `#====` banner and a `# VERSION:` line. Nothing in it is semantically meaningful, so it must pass through untouched.

## Critical Pitfalls

### 1. Rebuilding text from an AST instead of preserving source spans
**What goes wrong:** The parser produces `Block { conditions, actions }` and the writer re-emits them. Indentation, spacing, comments, blank lines, line endings and key casing are lost on the first save, even when the user changed nothing.
**Why it happens:** The existing `emit` path is generator-shaped. It writes canonical text from a model, so reusing it for editing is the path of least resistance.
**Consequences:** The "byte-identical round trip" requirement fails. A 258 KB FilterBlade file gets a giant diff, and the user can no longer diff against upstream.
**Prevention:** Use a lossless concrete syntax tree. Every line keeps its raw text (leading whitespace, keyword token, value text, trailing comment, original EOL). A line is re-serialized only when its parsed fields are mutated. Unmutated lines are emitted verbatim, and unmutated blocks are emitted verbatim including their inter-block blank lines. Treat the parser as "tokenize with trivia" and the model as a view over it.
**Detection:** A property test: `serialize(parse(bytes)) == bytes` over the sample, over every file in `reference-old-filters/`, over `tests/fixtures/seed-universal.filter`, and over generated mutations (CRLF/LF, no trailing newline, BOM). Run it as a hard gate before any edit code is written.
**Phase:** Parser phase. Must be first, with the round-trip test as its exit criterion.

### 2. Line-ending, BOM and final-newline normalization
**What goes wrong:** Rust `str::lines()` strips `\r\n` and `\n` alike. The writer then emits `\n`, or emits the repo's own `\n`-or-`\r\n` convention. A BOM may be dropped, or a missing final newline may be added.
**Prevention:** Parse over `&[u8]` or `&str` with an explicit splitter that keeps the terminator as part of each line. Store the EOL per line, because mixed endings occur in hand-edited files. Store the BOM flag and the "has final newline" flag. Newly inserted lines inherit the EOL of their neighbor, so the file stays uniformly CRLF. Never use `std::fs::read_to_string` followed by `write`, which is fine for UTF-8 but is easy to combine with `lines()`/`join("\n")`. Check that Windows git `autocrlf` does not rewrite committed fixtures: add `*.filter -text` (or `binary`) in `.gitattributes`, otherwise a CRLF fixture silently becomes LF on checkout and the byte-identical test passes locally but fails elsewhere. Also check the existing `seed-universal.filter` against this rule.
**Detection:** Round-trip test on a file with CRLF, a file with LF, a mixed file and a file without a trailing newline. A `git diff --stat` after a no-op save must show zero changes.
**Phase:** Parser phase.

### 3. Comments mis-split at `#` (quoted strings, commented-out blocks)
**What goes wrong:** Splitting a line at the first `#` breaks `BaseType == "Foo #1"`. Conversely, treating `#Show  # ...` as plain prose loses the fact that it is a disabled block that the user may want to enable.
**Prevention:** Write a small tokenizer that tracks quote state, so `#` only starts a comment outside quotes. Classify full-line comments into three kinds: prose, a commented block header (`#\s*(Show|Hide|Minimal)` followed by commented conditions and actions), and a section banner. Commented blocks stay opaque and verbatim by default. Do not auto-parse them into rules in v0.5 unless the roadmap explicitly wants "enable disabled block". If that is wanted, enabling means stripping exactly one leading `#` from each line of the contiguous run, which is reversible only if the original spacing is retained. Trailing comments on a value line (`SetFontSize 37 # big`) must be kept as trivia, and the leading spaces before `#` kept too.
**Detection:** Tests with `"a # b"`, a trailing comment after a quoted list, `#Show` with no space, `# Show` with a space, and the real line 449.
**Phase:** Parser phase.

### 4. Keyword case, operator spacing and quoting normalization on edit
**What goes wrong:** PoE filters are case-insensitive for keywords (`show`, `SHOW`) and accept operators with or without spaces (`Rarity>=Magic`, `Class "Bows"` with no operator). An editor that edits a field and re-serializes it as `Rarity >= Magic` rewrites the user's style on every touched line. Unquoted values (`Rarity Rare`, `Corrupted True`) get quoted, or the reverse.
**Prevention:** When mutating, change only the value token's byte range and keep the keyword, operator, and separators verbatim. Newly added lines use the neighbor-line style (the same indent string, with the keyword in the generator's canonical case). Quote a value only if it contains spaces or the neighboring list was quoted. For list conditions (`BaseType`, `Class`), add or remove one quoted element without reflowing the list.
**Detection:** A test that edits one value on a line like `\tRarity>=magic # x` and asserts that the diff is limited to that value's span.
**Phase:** Edit-model phase.

### 5. Semantic mismatch: PoE1 docs vs PoE2 grammar
**What goes wrong:** The parser hard-rejects or mis-types a condition or action it does not know (newer PoE2 keywords, `SetFontSize` limits, `Continue`, `Import`, `PlayAlertSoundPositional`, `CustomAlertSoundOptional`, `MinimapIcon -1`, PoE1-only `LinkedSockets`, `HasInfluence`). The official docs page describes PoE1, so a closed enum of keywords drifts.
**Prevention:** The syntax layer must be keyword-agnostic. An unknown keyword is still a valid line, stored as raw text with a generic `(keyword, rest)` shape. Give only the conditions/actions the editor UI understands a typed view (the existing `when` vocabulary plus actions that styles use). Everything else round-trips as opaque. Never fail the whole file on an unknown keyword: warn and continue. Do not validate values (the game itself warns on load, per the existing base-type decision).
**Detection:** Round-trip a fixture with a made-up keyword, and a real file with every keyword used in the sample.
**Phase:** Parser phase, then edit-model phase for the typed view.

### 6. `Continue` semantics and block order get lost in the rule model
**What goes wrong:** The existing model says "one rule is one block, first match wins". A filter with `Continue` has multi-match layering. If the editor presents blocks as a first-match-wins list, the user reorders or deletes a block and silently changes the effective behavior of blocks that rely on `Continue` falling through to later ones. Inserting a new block at the "end" lands it after a catch-all, so it never fires.
**Prevention:** Model `Continue` as a first-class flag on a block, show it in the row, and warn when a reorder moves a `Continue` block across another block that overlaps it. Keep file order as the only ordering truth. New blocks insert before the economy section or catch-all, not at the end, and the insertion point should be an explicit anchor, not "append". The existing ordinal-identity pitfall (v0.4 #9) applies to a file with 460 blocks even more. Identify blocks by stable ID assigned at parse time (not by index), and re-resolve after moves.
**Detection:** A test that reorders around a `Continue` block and asserts a warning or an unchanged effective order. A test that inserts a rule into a file that ends with a catch-all.
**Phase:** Edit-model phase, UI phase for the warning.

### 7. Economy-section refresh clobbering user edits (the file is now the store)
**What goes wrong:** Today `dynamic.json` holds the rules and `dynamic.filter` is regenerated whole, so economy sections are rebuilt from `prices.json` on every Apply. When the `.filter` itself becomes the store, regenerating the economy sections must replace only those sections. A naive "recompile and overwrite" erases the user's hand edits, and a naive "leave the economy alone" leaves stale prices. If the economy region is not delimited, a rule the user placed between generated blocks gets deleted on the next sync.
**Prevention:** Fence the generated economy region with unambiguous marker comments (for example `# ==== FERRISFILTER:ECONOMY BEGIN ====` / `END`) and only ever rewrite the bytes between them. Everything outside the fence is the user's and is never touched by sync. Missing or duplicated markers is a hard error that stops the write, not a silent fallback to "regenerate everything". A foreign file (FilterBlade) has no fence: in that case, do not inject economy sections unless the user explicitly asks, and say so in the UI. The migration from `dynamic.json` needs a one-time, explicit import step that writes the first fenced file and keeps `dynamic.json` as a backup, not a delete.
**Detection:** A test: parse, edit a rule above and below the fence, run economy refresh, assert that bytes outside the fence are identical. A test with the fence removed.
**Phase:** Store-migration phase (dropping dynamic.json). Needs its own design decision before coding.

### 8. Breaking the static path's byte-for-byte guarantee
**What goes wrong:** Refactoring the emitter to take config-driven styles (replacing the hardcoded `Styles` struct), or to share code with the new editor, changes a space, a key order, or a `HashMap` iteration order and `artifact_guard` or the insta snapshot moves. The temptation is to re-baseline.
**Prevention:** The v0.4 rule still holds: a behavior-preserving refactor produces zero fixture changes. Specific traps for this milestone: (a) `styles.json` is a new file, so its serialization of `HashMap`s is nondeterministic, so use `BTreeMap` or an ordered `Vec` for anything that influences emission or is round-tripped; (b) a style that was a struct field becomes a map lookup, so the field order in the emitted block (text color, border, background, font, sound, beam, icon) must stay a fixed order in the emitter, not the JSON key order; (c) the default `styles.json` must be generated from the current hardcoded values and tested for equality with them before the struct is deleted; (d) test the old struct and the loaded styles side by side for one commit, then delete the struct. Never regenerate `tests/fixtures/seed-universal.filter` in the same commit as a refactor.
**Detection:** `artifact_guard` red. Run it after every commit in the styles phase.
**Phase:** Styles-config phase, before any UI.

### 9. Style rename or delete orphaning rules
**What goes wrong:** Rules reference styles by name (`"style": "S"`). Renaming `S` in `styles.json`, or deleting it, leaves rules that fail at generate time ("unknown style is a hard error"). In the filter-as-store world, a block has no style reference at all, because it is just inlined lines, so "this block uses style S" is only a matching heuristic.
**Prevention:** Decide the model explicitly. Option A: style names are immutable keys, and the UI offers add/edit/delete but not rename (simplest, recommended for v0.5). Option B: rename is an atomic operation that rewrites every referencing rule and fails if any referencing source is unwritable. For delete, block it while references exist and list the referencing rules. Keep `unchanged`, `hidden` and `kite` as reserved built-ins that the UI cannot delete, because compiled code (such as the recycle and unidentified rules) references them by name. In the filter-store world, record the style as a marker comment on the block (`Show # name`) or ignore it and treat applying a style as "overwrite these action lines", and say which. Validate the whole styles file on load, and refuse Apply with a clear message rather than writing a partial filter.
**Detection:** Tests: rename referenced style, delete referenced style, delete a built-in, duplicate name, name differing only by case.
**Phase:** Styles-config phase and config-tab phase.

### 10. Tier cutoff edits producing invalid tiers
**What goes wrong:** `ValidatedTiers` enforces ordering. A live editor lets the user type values one keystroke at a time, so intermediate states are invalid (an empty field, a cutoff below the next tier, a negative number, NaN, a unit mismatch). If the editor validates only at Apply, the user finds the failure late. If it validates on every keystroke and rejects, they cannot type through an invalid intermediate state. If the editor writes through the existing `serde` path, the output may drop or reorder fields and break `prices-fallback`/`tiers-fallback` pairing (those two are calibrated to each other).
**Prevention:** Hold the text buffer separately from the parsed value. Parse and validate on every change, show the error inline, and keep the last valid value as the one Apply would use. Disable Apply while any field is invalid, and keep Revert always enabled. Reuse `ValidatedTiers` as the single validator (never reimplement the ordering check in the GUI). Edit only the live `tiers.json`. Never write to the committed `tiers-fallback.json`. If the tiers unit (`chaos` vs `exalt`) must match the prices file, show the unit read-only. Write atomically (tempfile then rename), as in v0.4 #7.
**Detection:** Tests: tier ordering violations, empty, equal values, huge values, unit mismatch, and a round-trip of `tiers.json` with no edits that produces an identical file.
**Phase:** Config-tab phase.

### 11. Config tab Apply/Revert has two files and two failure modes
**What goes wrong:** Apply writes `styles.json` and `tiers.json` and then the filter. If the second write fails, the first has already changed, so the stripe shows "applied" while disk state is inconsistent. Revert restores in-memory edits to the "last applied" state, which is undefined if the app launched with files that were edited externally. A file edited on disk (by hand, by git) while the overlay is open is overwritten on Apply without a warning.
**Prevention:** Validate everything first, then write in order, with temp files for all of them, and rename only after all temps succeeded. Record file mtime or hash on load, and warn or refuse on Apply if the file changed underneath (the stripe already has a "stale" concept, so extend it). Revert re-reads from disk and does not use a cached snapshot. A single "dirty" state per document, and a single Apply that is all-or-nothing from the user's point of view. Keep all logic in the library (v0.4 #8).
**Detection:** Tests with a read-only second target and with an externally modified file.
**Phase:** Config-tab phase.

### 12. Single-instance guard: Local vs Global namespace, and abandoned mutexes
**What goes wrong:** `CreateMutexW(name)` with a bare name is session-local (`Local\`). That is usually correct for a per-user overlay. `Global\` requires privileges in some configurations, and a mutex shared across sessions is not wanted. The classic mistake is to treat `ERROR_ALREADY_EXISTS` as the only signal while ignoring `GetLastError` timing: you must read the error immediately after `CreateMutexW`, before any other API call overwrites it. Another trap: a crashed instance. Windows releases a named mutex automatically when the owning process dies, so the "stale lock file" problem of file-based guards does not apply. Do not use a lock file (a crash leaves it behind and permanently blocks launch). Keep the handle alive for the process lifetime (store it in a `static` or in the `main` frame): if it is dropped early (`let _ = ...` instead of `let _guard = ...`), the guard silently stops working.
**Prevention:** Use `CreateMutexW` through the `windows` crate (or `windows-sys`), a name that includes an app-specific GUID and is `Local\` scoped, hold the handle in a binding that lives for the whole of `main`, and read `GetLastError` right after the call. Do not call `WaitForSingleObject` (that would hang the second launch or return `WAIT_ABANDONED`). Debug and release builds use the same name unless you intentionally want to run both, in which case suffix the debug build, otherwise `cargo run` fails while a release build lives in the tray, which will confuse development.
**Detection:** Manually launch twice, kill the first with Task Manager, launch again (must succeed), and launch twice as different users or from an elevated shell.
**Phase:** Single-instance phase.

### 13. Second launch must signal the first, and the first may be hidden
**What goes wrong:** The user launches `ff` again because the stripe is hidden (it lives in the tray, hidden via the hotkey or the `✕`). The second process silently exits, and the user sees nothing happen and concludes the app is broken. Or the second process tries to `SetForegroundWindow` on a hidden window, which does nothing, and Windows' foreground-lock rules block focus stealing from a non-foreground process anyway.
**Prevention:** On detecting an existing instance, the second process sends a "show" request and then exits. Mechanisms, in order of simplicity: a registered window message (`RegisterWindowMessageW`) broadcast with `PostMessage(HWND_BROADCAST)`, a named event (`CreateEventW`, which the first instance's UI thread waits on), or a localhost pipe. The first instance reacts by showing the stripe through the same code path as the `Ctrl+Shift+Z` toggle, not by a new path. The egui event loop only runs when repainted, so a hidden window may not poll the event: wake it with `ctx.request_repaint()` from the watcher thread, or the show request will be delayed until the next input. The existing hotkey is gated on "PoE2 or stripe is foreground", so the second-launch show request must bypass that gate, because the user explicitly asked. Let the second instance call `AllowSetForegroundWindow` (it is the process that has the foreground rights, being the one just launched by the user) before signaling.
**Detection:** Manual: hide the stripe, launch again, and it must appear. Also with the stripe visible, with the stripe visible but behind the game, and with PoE2 in the foreground.
**Phase:** Single-instance phase.

### 14. Tray app lifetime and exit paths
**What goes wrong:** The tray `Quit` is the only way to exit (existing decision). If the guard's handle is tied to an object that the tray quit path does not drop, or the process exits via `std::process::exit` without cleanup, nothing breaks (Windows reclaims the mutex), but a hung process with a dead window still holds the mutex and blocks every future launch. The user has to find the process in Task Manager.
**Prevention:** Ensure Quit actually terminates the process, including worker threads (the SYNC ECONOMY worker, the watcher thread, the tray thread). Make the watcher thread a daemon that does not block exit. Surface a message on the second launch if the first does not respond to the show signal within a short time ("another ff is running but not responding") instead of silently exiting.
**Detection:** Quit during an in-flight sync, then relaunch.
**Phase:** Single-instance phase.

### 15. Working-directory-relative paths (known defect, re-triggered by every new file)
**What goes wrong:** `ff` cannot be launched by double-click because `styles.json`, `tiers.json`, `dynamic_base.json` and `prices.json` all resolve relative to the CWD. v0.5 adds `styles.json` and a user-chosen filter path, so there are more paths that break, and the single-instance work makes double-click launching (and launching from a shortcut or autostart) much more likely. With the guard, a mis-launched first instance with the wrong CWD would also hold the mutex and silently misdirect the second.
**Prevention:** Resolve all data paths from one function, anchored in a decided order: explicit CLI flag, then the exe's directory (`std::env::current_exe()`), then CWD, with the choice logged in the stripe or a log line. Never call `Path::new("styles.json")` in new code. Treat the filter file path as persisted user state (a small settings file next to the exe), not as something derived from CWD. Show the resolved absolute path of each file in the config tab, so a wrong file is visible. Decide this once, early, before the new files multiply the problem. A candidate for a prerequisite task in the first phase.
**Detection:** Launch the release exe by double-click and from a different directory. The overlay must find its files or show a clear error with the path it tried.
**Phase:** Foundation phase (before styles/config).

## Moderate Pitfalls

### 16. Edit-in-place of a 460-block file in an immediate-mode UI
**What goes wrong:** egui redraws the full list each frame. Walking 460 blocks, formatting each row header, and re-parsing on every frame makes the stripe sluggish, and re-parsing the file every frame also destroys the trivia/IDs the UI holds.
**Prevention:** Parse once, keep the tree in memory, and cache per-block summary strings, invalidated on mutation. Use `ScrollArea::show_rows` or collapse sections by banner so only visible rows are laid out. Add a search or filter for blocks, since the old "accordion of a dozen rules" will not scale to 460.
**Phase:** UI phase.

### 17. Generated-section blocks vs foreign blocks in one list
**What goes wrong:** The user sees economy-fenced blocks in the same editable list, edits one, and the edit disappears on the next sync (see #7).
**Prevention:** Render fenced blocks as read-only and visibly different (or collapse them to a single "economy: N blocks, refreshed by sync" row).
**Phase:** UI phase.

### 18. Re-parsing generated output with the editor's parser
**What goes wrong:** The generator's current output is the parser's most important regression sample, but its `Show # name` comments carry meaning (the rule name). The parser's treatment of a trailing `# comment` on a header as a "name" must be consistent with the emitter, or an unnamed rule gets `rule 3` shown as a name.
**Prevention:** Round-trip `seed-universal.filter` and a generated `dynamic.filter` through the new parser as tests. Read the header comment as a name only when it is not a derived `rule <index>` label.
**Phase:** Parser phase.

### 19. Reset/restore semantics after dropping `dynamic.json`
**What goes wrong:** `apply --reset` currently restores from `dynamic_base.json`. When the filter is the store, "reset" now means overwrite the user's own file. A blind reset destroys a hand-tuned or imported filter.
**Prevention:** Back up before reset (timestamped copy beside the file), ask for confirmation in the UI, and keep `dynamic_base.json` as the template for new files only. Keep a CLI deprecation path for `dynamic.json` for one release, and note it in `CHANGELOG.md`.
**Phase:** Store-migration phase.

### 20. Writing into the game folder (OneDrive path)
**What goes wrong:** The game folder is under OneDrive (`OneDrive\Documentos\...`). Atomic temp-then-rename across a synced folder can trigger sync conflicts, and a temp file in a different directory makes rename non-atomic across volumes. Editing a file the game or a sync client has open can fail with a sharing violation.
**Prevention:** Create the temp file in the same directory as the destination. Retry a rename briefly on `ERROR_SHARING_VIOLATION`. Surface a clear error rather than falling back to a non-atomic copy.
**Phase:** Store-migration phase.

## Minor Pitfalls

### 21. Trailing whitespace and `$` in the sample
Two lines in the sample have trailing whitespace, and comments contain `$` and `[...]`. Any regex-based parser written with `\s*$` and `trim()` will eat them. Use `trim_end_matches(['\r','\n'])` only, never `trim()`, on the stored raw line.

### 22. Duplicate `Show` banner sections and repeated headers
Comment banners such as `[2905] Tier 3 uniques` appear twice (lines 100 and 4051). Do not use banner text as a unique key. Use position-derived IDs.

### 23. Style name collisions with reserved names
`hidden`, `unchanged`, `kite` and the tier letters are referenced by compiled logic. Reject new styles that shadow them (case-insensitively), see #9.

### 24. Crate name and binary naming
Carried over from the v0.4 guard: do not name anything `overlay` in a way that triggers egui #5512 if a new crate or binary is added for the parser.

## Phase-Specific Warnings

| Phase Topic | Likely Pitfall | Mitigation |
|-------------|---------------|------------|
| Foundation: path resolution | CWD-relative paths multiply with each new file (#15) | One path-resolver function, exe-dir anchor, show resolved paths |
| Lossless parser | Rebuild-from-AST, EOL/BOM, `#` in quotes, unknown keywords (#1-#5, #18, #21) | Trivia-preserving tree, per-line EOL, round-trip property test as exit gate, `.gitattributes -text` |
| Edit model | Spacing/case normalization, `Continue`, stable block IDs (#4, #6) | Span-based mutation, `Continue` as a flag, ID not index |
| Store migration (drop dynamic.json) | Economy refresh clobbering edits, destructive reset, OneDrive writes (#7, #19, #20) | Fenced economy region, backups, same-dir temp file |
| Styles config | `artifact_guard` regression, rename/delete orphaning, HashMap order (#8, #9, #23) | Equality test vs hardcoded struct before deleting it, immutable names, ordered maps |
| Config tab | Invalid tiers mid-typing, two-file Apply, external edits (#10, #11) | Buffer vs value, `ValidatedTiers` as single validator, all-or-nothing writes, mtime check |
| Overlay UI scale | 460-block immediate-mode lists, generated blocks looking editable (#16, #17) | Cached summaries, virtualized rows, read-only economy row |
| Single instance | Mutex scope, dropped handle, signaling a hidden window, hotkey gate, hung first instance (#12-#14) | Local mutex held in `main`, registered message or named event plus `request_repaint`, bypass the foreground gate, timeout message |

## Research flags

- Parser phase: needs a design spike on the trivia model and on whether commented-out blocks are enabled-able in v0.5. The sample proves they exist in volume (`#Show  # # $type->...`).
- Store migration: needs a decision on the economy fence for foreign files (inject, or leave alone).
- Single instance: standard Win32 patterns, unlikely to need research beyond a manual test matrix.
- Not verified: NeverSink/FilterBlade variants other than the one sample (for example `Import` lines, `Minimal`, mixed EOL). Treat as LOW confidence and cover through the opaque-line fallback.

## Sources

- Measured directly from `C:\Users\jonat\OneDrive\Documentos\My Games\Path of Exile 2\filterblade.filter` (byte counts, CRLF/tab/comment samples), HIGH confidence.
- `C:\Users\jonat\Repositories\poe2_myfilter\CLAUDE.md` (overlay decisions, `ValidatedTiers`, fallback pair, known CWD defect), HIGH.
- `.planning/research/_archive-pre-v0.5/PITFALLS.md` (prior pitfalls not repeated).
- Win32 named-mutex semantics (`CreateMutexW`, `ERROR_ALREADY_EXISTS`, automatic release on process exit, `Local\` vs `Global\`) and foreground-lock rules (`AllowSetForegroundWindow`) from general Windows API knowledge, not re-verified against Microsoft docs in this session, MEDIUM.
- Filter grammar notes: https://www.pathofexile.com/item-filter/about (documents PoE1; PoE2 differences per CLAUDE.md), MEDIUM.
