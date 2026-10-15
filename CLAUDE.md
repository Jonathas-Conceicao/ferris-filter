# CLAUDE.md — FerrisFilter

This repo is **FerrisFilter**: a Rust CLI that compiles a human-editable config into a
Path of Exile 2 loot filter (`generate`) and fetches item prices from poe.ninja (`sync`).

- **Project plan & workflow:** managed with GSD in `.planning/` — see `.planning/PROJECT.md`
  (context), `.planning/ROADMAP.md` (4 phases), `.planning/REQUIREMENTS.md`, and
  `.planning/STATE.md`. Git uses a **milestone branching** workflow (see PROJECT.md → Git Workflow).
- **Changelog:** `CHANGELOG.md` (repo root) tracks notable changes. Keep `[Unreleased]` current
  as work lands, and **always update `CHANGELOG.md` when a milestone is completed** (e.g. via
  `/gsd-complete-milestone`): move the `[Unreleased]` entries into a new dated version section
  with a one-line milestone summary.
- **Domain spec (below):** the filter grammar, the Dracula style-tier system, and the design
  philosophy. `config.json` + `styles.json` are the executable spec the `generate` command compiles. The
  **byte-for-byte acceptance baseline** is now the committed fixture
  `tests/fixtures/seed-universal.filter` (generated from `config.json` + `styles.json` + the seed
  `tests/fixtures/seed-prices.json`); the `artifact_guard` test enforces it. The repo-root
  `universal.filter` / `prices.json` are **generated, git-ignored, and not committed**.
  Everything from "Repository layout" down is that reference — keep it accurate.

## Repository layout

| Path | Purpose |
|------|---------|
| `config.json` | **Source of truth.** Declarative policy model the filter is compiled from. Top-level sections: `build` (weapon/armour classes + defence-type taper), `weaponClasses`/`armourClasses`/`flaskCharmClasses` (complete class lists), `classGroups` (documentation aliases), the ordered `rules` array (the policy — first-match-wins), and `alwaysShow` (currency, recycle). Edit `rules` (and `styles.json` for how a style looks), then regenerate. A `styles` section here is rejected at load unless every entry in it equals the same-named `styles.json` entry (then it is dropped). |
| `styles.json` | **Committed single source of named styles** (repo root): a map of style name to style object (text/border/background colours, font, minimap, beam, sound, `action` Show/Hide, `disableDropSound`), each carrying an `"id"` written first: `builtin:<name>` for the nine built-ins, a random 6-8 character lowercase alphanumeric id for custom styles (+ ADD STYLE). A file without ids still loads (ids are assigned at load, deterministically from a hash of the name so they are stable across loads, and persisted by the next overlay Apply without showing as an edit); duplicate ids, a wrong built-in id or a custom id starting with `builtin:` are load errors. The generator and `config.json` ignore ids and keep using names. File order is the overlay picker order. `S`, `A`, `B`, `C`, `D`, `E`, `F`, `unchanged` and `hidden` are required; every other style is optional, and adding one needs no code change. Read by `generate` (beside the inputs) and by `apply`/the overlay (from the app root); `--styles <path>` on `generate` and `apply` overrides it. A config that still has an inline `styles` section loads only if every inline entry equals the same-named entry here (an untouched pre-v0.5 seed): the section is then dropped in memory (the file itself is not rewritten). Otherwise it is rejected with a message naming the file and the differing styles; move those here and delete the section. The overlay's Apply (either tab) rewrites this file when the CONFIG tab has edits (one style per line, file order kept, which includes a custom-style reorder); `tiers.json` is rewritten the same way (entry order kept) — and the `-fallback` pair is never touched. |
| `universal.filter` (repo root) | **Generated** by `ferrisfilter generate` and **git-ignored** — regenerate on demand, do not commit or hand-edit. Each policy block is headed `Show # <rule name>` (from the `name` field in `config.json`). |
| `prices.json` (repo root) | **Generated** by `ferrisfilter sync` from poe.ninja and **git-ignored** — your live economy data, regenerated per sync, never committed. `sync --league <name\|slug>` targets a specific poe.ninja league (useful during a league transition, when the automatic pick resolves to the outgoing league). |
| `ff-settings.json` (app root) | **Per-user overlay settings, git-ignored.** Written by the overlay's CONFIG tab; holds the selected game-folder filter name (a bare `*.filter` file name, validated before it is ever joined onto the game folder), default `dynamic.filter` when absent or invalid. Unknown keys are preserved on save. Never committed. |
| `dynamic.filter` | **The store.** Since Phase 18 the filter file itself is the store — there is no `dynamic.json` (a leftover repo-root copy from v0.4 is obsolete and safe to delete; `/dynamic.json` stays git-ignored only so it is never committed). It lives in the game folder and is edited **in place**; general rules in it are the user's and are never touched. The **economy section** at the end of the file is fenced by two marker lines, quoted exactly: `# ff:economy-begin Generated by FerrisFilter from prices.json, tiers.json and styles.json. Do not edit; Apply regenerates everything up to economy-end.` and `# ff:economy-end End of the FerrisFilter economy section.`. Apply regenerates only what lies between them (from `prices.json`, `tiers.json` and `styles.json`) and leaves everything outside untouched; damaged markers stop apply. Every FerrisFilter metadata line (`# ff:name`, `# ff:style`, the two markers) is written with a space after `#`; the pre-v0.5 spelling `#ff:` is still read (either spelling pairs with the other, and one of each is a duplicate marker), and every Apply rewrites a remaining `#ff:` line to `# ff:`, leaving all other bytes alone. Disabled blocks are written the same way: disabling puts `# ` in front of every line (`# Show`, `# \tRarity Rare`; an empty line becomes a bare `#`), and enabling removes the `#` plus one space after it when present, otherwise just the `#` — so the older `#Show` form (earlier FerrisFilter versions, foreign filters) still enables, and disable→enable is byte-exact. Old `#Show` blocks are **not** normalized on Apply (they cannot be told apart from a foreign filter's own disabled blocks); a block takes the `# ` form the next time it is disabled. The first write keeps `<name>.filter.ffbak` next to the file — created once, never overwritten (delete it to take a fresh one); `*.ffbak` is git-ignored. Apply refuses if the file changed on disk since it was loaded (the overlay then offers **RELOAD** and **OVERWRITE**). A v0.4 file is converted once, on its first apply: markers added, `Show # <name>` names kept as `# ff:name`, the stale `# GENERATED FROM dynamic.json` header line left as it is. **Git-ignored** at the repo root. PoE2 does not hot-reload — after `ff-cli apply`, reload via Options → Game → Item Filter → Refresh; the overlay's Apply can send `/reloaditemfilter` instead (see the overlay notes). |
| `tests/fixtures/seed-prices.json`, `tests/fixtures/seed-universal.filter` | **Committed test fixtures** — the frozen seed prices + its generated filter. These are the test oracle (`include_str!` unit tests + `generate_snapshot`/`artifact_guard`). Regenerate the `.filter` fixture only after a deliberate generator change. |
| `prices-fallback.json` + `tiers-fallback.json` (repo root) | **Committed early-league fallback pair**, always used *together* (their values are calibrated to each other). `generate` loads them instead of `prices.json`/`tiers.json` when `prices.json` is absent/empty (with a warning) or when `--early-game` is passed. Tier membership comes from a poe2filter.com export; cutoffs are **exalt-denominated** (`unit: "exalt"`) so Divine isn't pinned to the A floor. Every base — orbs included — is priced in **exalt at its tier floor** (Divine 20→S, Chaos 2.5→A, Exalt 1→B). No stored rates: `pricer::derive_rates` reads the three anchor orbs (items[0..2]) and expresses them relative to the Chaos Orb (`exalt_in_chaos = value(Exalt)/value(Chaos)` etc.), so quoting the orbs in exalt is enough. The same code on live chaos-quoted data (Chaos = 1c) reproduces the old numbers. The three anchors must share one unit. |
| other `*.filter` (repo root) | Active, version-controlled filters we author/maintain. |
| `reference-old-filters/` | Older/stale filters copied from the game's config folder. **Git-ignored**, kept only as reference. Do not edit these as if they were live; treat them as read-only history. |
| `ferrisfilter apply` | `ff-cli apply [--filter <path>] [--add-economy \| --remove-economy] [--tiers <path>] [--styles <path>]`. Refreshes the economy section of the filter at `--filter` (default `%USERPROFILE%\OneDrive\Documentos\My Games\Path of Exile 2\dynamic.filter`). Plain apply only refreshes an existing section and **never adds one**; `--add-economy` creates it at the end of the file, `--remove-economy` deletes it. The old reset, base, store and dest flags no longer exist. **App root:** apply and the overlay find `styles.json` / `tiers.json` / `prices.json` via `FERRISFILTER_HOME`, else by walking up from the exe's folder to the one holding `styles.json` (only `styles.json` marks it — a stray `config.json` is not a marker), else the working directory, else an error. A relative `--tiers`/`--styles` given to apply also resolves against the app root (`--filter` stays working-directory relative). `generate`, `install` and `sync` stay working-directory relative, so `sync` run elsewhere writes `prices.json` there while apply reads the app root's. |
| `ferrisfilter install` | Subcommand that copies `universal.filter` into the in-game config folder (replaces the old `install.bat`). Defaults to `%USERPROFILE%\OneDrive\Documentos\My Games\Path of Exile 2`; pass `--dest <dir>` to override and `--input <file>` for a non-default filter. Copy-only — run `generate` first. |
| `ff` (binary, v0.4) | **The GUI overlay** — `cargo build`'s default binary (`default-run = "ff"`). Opens the always-on-top stripe window over the game; Windows GUI subsystem in release builds (no console flash), debug builds keep a console for `eprintln!`/panic visibility. Entry point is `ferrisfilter::overlay::launch()`. Since Phase 18 it drives the game-folder `dynamic.filter` as a rule list over its parsed blocks (add, delete, duplicate, reorder, enable/disable, Show/Hide + Continue, naming; Apply / Revert / Reload / Sync economy) and a CONFIG tab (filter choice, `styles.json`, `tiers.json`) — see the overlay notes below. |
| `ff-cli` (binary, v0.4) | The short-name console CLI — identical behavior to `ferrisfilter`, just a shorter name to type. Holds the console shim body `ff` used to have before it became the GUI binary. |
| `ferrisfilter` (binary) | The long-name console CLI, unchanged: `generate` / `sync` / `install` / `apply`. |
| `CLAUDE.md` | This file — the filter-syntax reference so we rarely need to re-fetch the docs. |

> **Overlay (v0.4, `ff`):** Path of Exile 2 must be running in **windowed** or
> **windowed-fullscreen** mode — exclusive fullscreen prevents any external window from drawing
> above it, and `ff` does not detect this, so a blank/missing stripe means checking the game's
> display mode first (see `.planning/phases/13-overlay-shell/13-MANUAL-VERIFICATION.md`). Press
> **`Ctrl+Shift+Z`** (system-wide, hardcoded) to summon or hide the stripe without alt-tabbing —
> **only while Path of Exile 2 or the stripe itself is the foreground window**, so the binding stays
> free for other applications; the
> stripe's own `✕` only **hides** it, and the system tray icon's **Quit** is the only way to exit
> the process. Only one `ff` runs at a time (`src/overlay/instance.rs`): launching it again
> (including by double-clicking the exe) brings up the running stripe, even when it is hidden to
> the tray, and the new process exits; if the running one does not answer within about 2 seconds a
> message box says "FerrisFilter is already running but not responding". A launch made while the
> running one is still starting up first waits up to 10 seconds for its first frame before that
> 2-second window starts, and several launches at once are each answered. Tray **Quit** gives up the
> hotkey, then the tray icon, then the single-instance name at once, so relaunching right after Quit starts a fresh stripe instead of
> asking the closing one (and a launch whose running instance quits or crashes while it is asking
> runs as the new instance rather than showing a message). If the single-instance
> check itself fails, `ff` does not start (a message box names the Windows error) rather than risk
> a second stripe. Debug builds use a
> separate instance name (`-overlay-debug`), so a `cargo run` build can run beside a release `ff`
> (their hotkeys then conflict, which the tray tooltip already reports).
>
> **The rule list and editor (Phases 19-20):** the overlay treats the game-folder `dynamic.filter` as its store (see
> the `dynamic.filter` row above); there is no `dynamic.json`. Every block of the loaded filter is a
> row in a flat, virtualized list (responsive on the ~500-block NeverSink/FilterBlade sample), with a
> search box matching the `# ff:name` and Class/BaseType values. A rule's name comes
> only from its `# ff:name` line; foreign header text such as NeverSink's `$type->` tags is kept on disk
> but never shown. An unnamed rule's header reads just `<index>  Show` (or `Hide`); the collapsed
> summary line gives the context. Rows expand in
> place (one at a time) with NAME, CONDITIONS, then STYLE, then Duplicate / Delete (inline confirm).
> NAME is a plain text box (no toggle): empty means no `# ff:name` line, clearing it removes the line,
> typing adds or updates it. STYLE is one picker of **Custom** plus every `styles.json` preset. For
> **Custom** only, Show / Hide sits on its own line below the picker, followed by the action rows
> with **+ style**; a preset hides both, decides Show/Hide itself and writes `# ff:style <id> <name>`.
> Switching a preset back to Custom keeps the current actions and drops the tag. Only Show and Hide
> are offered: a foreign `Minimal` block is shown as Minimal (Custom only) and left as is unless
> changed. **Continue to later rules** is its own line, last in the body. No caption
> text; an unknown style is flagged on the picker with a tooltip. Every condition and action is editable with structured
> controls: a grouped, searchable **+ condition** / **+ style** picker, an operator dropdown with
> plain-English labels (an existing operator is never normalized), sliders paired with exact numbers,
> a class picker whose groups come from `config.json`, quoted lists, a colour
> picker with alpha, and minimap / beam / sound controls. An edit rewrites only the edited value's
> lines. A list condition added with **+ condition** is an unwritten row until its first entry.
> Quoted lists (BaseType, mod names) are a vertical list, one entry per line with its own remove, a
> type + Enter / **+ ADD** field that splits a pasted quoted list, in a scroll area of about 7 rows.
> Class is the grouped picker only (classes outside `config.json`'s groups are kept and listed on one
> compact line). Booleans are a compact **Yes | No** control (no operator; remove the condition to
> match either; an existing operator such as `!=` is kept). Condition remove buttons are right-aligned.
> **Sockets** is a plain numeric condition (operator plus a 0-6 slider); the overlay never edits socket
> colours, but a hand-written `Sockets >= 5GGG` still loads and keeps its letters when the count is
> changed.
> **Defences** is one condition (one card per rule) with Armour / Evasion / Energy Shield toggles, each
> with its own operator and slider; `BaseDefencePercentile` is gone from the keyword table (a line using
> it loads as a raw row; Ward is not supported yet). Whole-number conditions (and each defence) offer a
> **Between** operator: one card with two sliders writing `X >= min` and `X <= max`, clamped so min never
> exceeds max; a loaded one-lower/one-upper pair shows as one card with its bytes kept until edited
> (a loaded impossible pair is flagged and left as written), while other duplicates (two `>=`) stay
> separate cards. In the **+ condition** picker, sections start collapsed (a search opens matches), a
> condition already on the rule is greyed (clicking it focuses the existing card instead of adding
> another), and a pending condition shows its operator dropdown before the first value.
> Lines with an unknown keyword show as raw rows (Enter commits; an edit that would add a grammar
> Error shows an inline hint and changes nothing). PoE1-only conditions (e.g. `LinkedSockets`,
> `SocketGroup`, `HasInfluence`, `MapTier`, `Exceptional`) are not in the keyword table or picker; a filter using
> them still loads byte-for-byte, shows them as raw rows, and Apply is not blocked (warnings never
> block), and an invalid commented-out block is shown
> read-only. Arrows reorder
> (disabled while searching; a move across a Continue rule is flagged `moved across Continue`). The
> circular toggle comments a block out (`# ` before each line) or back in; commented-out blocks load as disabled rules, and a
> commented block that is not valid grammar is flagged `can't enable`. Enabled rules with grammar
> errors are flagged and **APPLY is refused** until they are fixed or disabled (warnings are flagged
> but do not block). **+ ADD RULE** appends an empty `Show` rule before the economy section. The
> economy blocks are never listed: a footer row shows `+ N locked rule blocks (economy)` with Remove,
> or **Add economy section** when absent. All edits stay in memory until **APPLY** writes them (a
> filter without an economy section can be applied); **REVERT** re-reads the last applied text,
> **RELOAD** re-reads the file and discards unapplied edits (also offered when the folder was not
> found, the filter could not be loaded, or APPLY was refused because the file changed on disk), and
> **SYNC ECONOMY** is described below. **APPLY** is one action shared with the CONFIG tab (see the
> CONFIG note). When it is refused because a file changed on disk the refusal names the file(s) and
> both tabs offer **RELOAD** and **OVERWRITE** (details in the CONFIG note); OVERWRITE
> counts as an Apply, in-game reload included (the normal guarded write: same atomic write,
> retry and one-time `.ffbak`). An Apply
> whose result equals the file on disk is not rewritten but still counts as applied: same status,
> same in-game reload. Opening the overlay never writes: a v0.4 filter is announced as
> "will be converted on the next APPLY" and a `.ffbak` backup is kept by that first write. Picking a preset in the STYLE section writes
> `# ff:style <id> <name>` (the id links the rule to the style; the name is informational and refreshed on Apply; an old `# ff:style <name>` tag still resolves by name and is rewritten to the new form on the next Apply; that tag-only migration does not mark the filter unapplied or trigger the Quit confirm) and sets the rule's actions from `styles.json`. On every Apply, styled rules are
> rewritten from the current `styles.json` (re-read at Apply, and watched while the overlay runs: a
> change on disk marks the filter as having unapplied changes), and the outcome says
> `Updated N styled rules.`. A preset's actions are not shown in the UI; a hand edit
> of a styled rule's actions in the file is overwritten on the next Apply. Switching to **Custom**
> keeps the current actions as the rule's own. A tag naming a style that
> does not exist is flagged `unknown style`, reported in the outcome area, and never rewritten or
> blocking. A disabled styled rule is rewritten once it is enabled. A collapsed rule shows a one-line condition summary under its header (hidden when expanded). Collapsed rows show `styled:
> <name>` (truncated to 12 characters) and fold flags beyond a 120px budget into a `+N` badge, so the
> rule label always keeps at least 140px. All footer messages (Apply outcome, list status, sync
> status, the grammar-gate and unknown-style lines) share one outcome area capped at 96px that
> scrolls, so APPLY, REVERT and RELOAD stay on screen. A successful Apply on either tab (or OVERWRITE) sends
> `/reloaditemfilter` to a running Path of Exile 2 when the CONFIG setting **Reload filter in game
> after Apply** is on (the default), and the outcome reads `Reloaded in game` -- meaning every key was
> sent to the game window; ff cannot see whether the game ran the command (an elevated game silently
> drops the keys, see the caveats below), so if the game did not reload, Refresh by hand. The result
> lands after the game has taken focus (which hides the stripe) and is shown on the next reveal. A
> refused Apply sends nothing. With the game not running the outcome is just `Wrote <path>.` (the game reads the filter when it starts). For the other reasons the outcome keeps the **Options → Game → Item
> Filter → Refresh** reminder plus the reason (`reload setting is off`, `a reload
> is already in progress`, `the overlay window is not ready`, `could not reload in game: <cause>`).

> **The CONFIG tab (Phase 21):** the stripe has **RULES | CONFIG** tabs under the header. Switching tabs
> never applies or discards anything; a tab label shows ` *` when the other tab has unapplied edits.
> CONFIG has three sections. **FILTER** picks which `*.filter` in the game folder the RULES tab edits
> (or creates a new empty one, written by the first Apply); the choice applies at once, is saved
> in `ff-settings.json` and is restored on the next launch (default `dynamic.filter`). FILTER also holds
> the **Reload filter in game after Apply** toggle (saved on change). Picking another
> filter while the RULES tab has unapplied edits asks first (`Switch and discard` / `Stay`). **STYLES**
> is one table per group, each with its own header: **Built-in** (`Visibility | Name | Preview`; switch and
> actions editable; no rename, move or delete), then the **Custom** heading, which carries the
> **+ ADD STYLE** button and is shown even with no custom styles, with an extra **Order** column holding
> the arrows. A custom style's name is an inline text field: renaming keeps its rules linked by id (rules
> still carrying a legacy name-only tag for it are moved to the new name in memory too) and is one
> revertible edit. Caveat: `config.json` references styles by name, so renaming a custom style it uses
> makes `generate` fail with the unknown-style error until `config.json` is updated. Custom styles can be
> deleted and reordered with up/down arrows (order = `styles.json` order, saved by Apply, undone by
> Revert; a custom style never moves above the built-ins). There is no status line and no new/edited
> tag. Each style has an on/off
> slide switch (on = Show, off = Hide) that reaches the filter on the next Apply (styled rules
> and the economy section are regenerated), which hides a whole group of styled rules at once (the
> colour picker is opaque because styles store RGB and the filter always emits alpha 255). Deleting a
> style shows how many rules use it, and warns when `config.json` itself references it
> (`generate` would then fail). **TIERS** edits the `tiers.json` cutoffs with inline validation.
> There is **one APPLY**, the same on either tab. It validates everything first (tier cutoffs, rule
> grammar, economy markers and pricing, style rewrites, and the changed-on-disk guard of the filter and of any config file
> with CONFIG edits; an unedited `styles.json`/`tiers.json` just follows the disk and is never
> written), then writes `styles.json`, `tiers.json` and the filter (rendered with the pending styles and
> tiers) all or none, then reloads in game under the existing rules. With no filter loaded it writes
> the config files only. A refusal writes nothing. When a file changed on disk the refusal names it
> and both tabs offer **RELOAD** (re-reads the changed files; a changed filter discards rule edits, a
> changed `styles.json`/`tiers.json` discards config edits) and **OVERWRITE** (writes everything over
> them); nothing is overwritten silently. **Revert** stays per tab (RULES discards rule edits, CONFIG
> discards config edits), and both tabs show the same unapplied state and the same Apply message.
> The RULES style picker and tag checks follow the unapplied CONFIG style set. Tray **Quit** with anything
> unapplied asks **Apply & quit** (quits only after a successful Apply, and waits for an in-game reload to
> finish) / **Quit without applying** / **Cancel**; with nothing pending it exits at once.
> After Apply the RULES tab sees the new styles at once. Rules whose
> `# ff:style` names a deleted style lose that tag on that Apply and keep their actions.
>
> **Development warning:** running `ff` from the repo resolves the repo root as the app root, so Apply
> (from either tab, once CONFIG has edits) would rewrite the committed `styles.json` / `tiers.json` that `artifact_guard` reads. For manual
> testing set `FERRISFILTER_HOME` (and `USERPROFILE`) to a temp copy first.
>
> **In-game reload after Apply (Phase 23, RELOAD-01):** the mechanism is: focus the game, Enter,
> paste `/reloaditemfilter` from the clipboard (Ctrl+V), Enter -- the same clipboard-paste method
> Exiled Exchange 2 and Awakened PoE Trade use for `/` commands. There is deliberately **no Ctrl+A**
> (EE2 skips select-all for `/` commands too): a synthetic Ctrl+A fired another tool's global
> hotkey. Ctrl+V and Enter are the only chords/keys sent. One Apply click runs one send sequence, never repeated
> or retried. The game window is found by title ("path of exile", so "Exiled Exchange 2" does not count)
> AND by a process image name starting with `PathOfExile`, so a browser tab titled "Path of Exile 2"
> is not the game. Keys are sent only after checking the game is the foreground window; otherwise
> nothing is sent and the reason is shown. Caveats: `/reloaditemfilter` reloads the filter **selected
> in game**, so if the overlay edits a different filter than the one picked in Options → Game → Item
> Filter, the reload will not show the edit (the overlay does not switch the game's filter). Close the
> chat box before pressing Apply: nothing outside the game can tell whether chat is open (EE2 and
> Awakened PoE Trade share this), so if it was already open with text, the first Enter sends that
> text as a chat message and `/reloaditemfilter` is not sent. No key is pressed before that Enter
> on purpose (with chat closed it would reach the game's own bindings). Run `ff` and the
> game at the same privilege level (Windows silently drops input into an elevated game, undetectably,
> so the status still reads `Reloaded in game`). The clipboard's
> text is restored about 120 ms later unless something else was copied meanwhile; non-text clipboard
> content (an image, copied files) is replaced. The setting is the `reload_after_apply` key in
> `ff-settings.json`, saved as soon as it is toggled; absent means on, and a value that is not
> `true`/`false` means **off** with a warning naming the key (an unreadable opt-out fails closed).
> The same holds for the whole file: an `ff-settings.json` that exists but cannot be used (invalid
> JSON such as a trailing comma, or unreadable) means `dynamic.filter` **and** reload off, with a
> warning that says both; only a missing file means the default (on).
> `ff-cli apply` does not send
> the command.

> **Economy sync from the overlay (v0.4):** a **SYNC ECONOMY** button beside Apply refreshes
> `prices.json` in the app root (see the App root note in the `ferrisfilter apply` row), on a worker thread so the stripe never freezes on network
> I/O. It writes prices and nothing else — `dynamic.filter` still changes only on **Apply**, so the
> button is safe to press mid-map and the new economy lands on the next Apply you choose to make.
> The GUI exposes no `--standard`/`--hc`/`--league`/`--tiers` switches: it syncs with the tiers file
> already in place, and **the league is pinned** in `sync::OVERLAY_LEAGUE` — currently
> `"Forbidden Rites"`. **Update that constant at the start of each league.** It is pinned because
> poe.ninja's `indexed` flag does not reliably identify the current league: it was observed
> flipping between Forbidden Rites and Runes of Aldur within six hours on 2026-09-10/11, and a sync
> caught on the wrong side writes last league's economy into `prices.json` while the resulting
> filter looks completely normal. The success line names the league it priced against, so a stale
> pin is visible rather than silent, and a pin that no longer matches any league fails loudly with
> the list of available ones. The CLI keeps automatic resolution and its `--league` override.
>
> **Design philosophy (current):** Hide what we don't want, style the few highlights, and let
> everything else fall through to the game default. Uniques, jewels, gems, quest items,
> waystones, and trials have **no rules** — they show natively. Jewellery, unidentified
> high-tier items, and equipment are covered by explicit rules in the `config.json` `rules`
> array. `config.json` is the authoritative model — the reconciliation of the old
> hardcoded-band description with the actual code is complete as of v0.2.

Source folder for filters in-game (where they must be copied to be usable):
`C:\Users\jonat\OneDrive\Documentos\My Games\Path of Exile 2`

External references (fetch only when something here is insufficient):
- Filter syntax: https://www.pathofexile.com/item-filter/about
- Exact item names / base values: https://poe2db.tw/us/

> **Caveat:** The official `item-filter/about` page documents the PoE1 filter language.
> PoE2 uses the *same grammar* but a different set of valid values (classes, base types,
> tiers) and omits some PoE1 concepts (see "PoE2 differences" below). When in doubt about
> an exact base name, class, or numeric range, verify against poe2db.tw or the live game.

---

## Filter grammar

A filter is an ordered list of **blocks**. Each item that drops is tested against blocks
**top to bottom**; the **first matching block wins** (unless `Continue` is used). A block is
a header (`Show` / `Hide` / etc.) followed by indented **conditions** and **actions**.

```
Show # Human description (optional-slug)
  Rarity == Unique
  Class == "Body Armours"
  SetTextColor 53 30 14 255
  MinimapIcon 1 Brown Kite
  PlayEffect Brown
```

### Block types
- `Show` — display matching items.
- `Hide` — conceal matching items.
- `Minimal` — minimal label (Ruthless filters only; rarely relevant for PoE2).
- `Continue` — placed inside a block so matching does **not** stop here; subsequent blocks
  may also apply (lets you layer styling rules).
- `Import "file.filter" [Optional]` — include another filter file.

### Comments
- `#` starts a comment to end of line. A `#` after a block header is a common convention for
  naming the block (e.g. `Show # Excellent Uniques (uniques-excellent)`).

### Operators
- `=` equal · `==` exact match · `!=` / `!` not equal
- `<` `<=` `>` `>=` for numeric/ordinal conditions
- For named/list conditions (e.g. `BaseType`, `Class`), listing multiple quoted values means
  **match any**. `==` enforces *exact* full-name match; without `==` it matches as a
  substring/prefix (e.g. `Class "Bows"` also matches `"Bow"`-containing class names). Prefer
  `==` with full names to avoid accidental matches.

---

## Conditions (common / PoE2-relevant)

**Numeric / ordinal**
- `ItemLevel` — item level.
- `DropLevel` — base item's drop level.
- `AreaLevel` — level of the area where it dropped.
- `Quality` — 0–20+ (corruption can exceed).
- `Rarity` — `Normal` < `Magic` < `Rare` < `Unique` (supports `<`/`>` ordering and `==`).
- `Sockets` — socket count (e.g. `Sockets >= 3`). PoE2 sockets hold runes/soul cores.
- `Height` / `Width` — item's inventory dimensions.
- `StackSize` — for stackable currency.
- `WaystoneTier` — PoE2 map (Waystone) tier.
- `BaseArmour` / `BaseEvasion` / `BaseEnergyShield` — base defence values.
- `UnidentifiedItemTier` — confirmed PoE2 (author-tested, v0.2): highlights high-tier
  unidentified items (e.g. `UnidentifiedItemTier >= 4`). Used by the unidentified rules, which
  wear the `kite` style. Note the condition and the style are independent: the condition is what
  matches, `kite` is only how a match is drawn.
- `GemLevel` etc. — verify PoE2 applicability before relying on them. PoE1-only conditions are
  not in FerrisFilter's keyword table (kept as raw lines when met).

**Boolean (value `True`/`False`)**
- `Corrupted`, `Mirrored`, `Identified`, `FracturedItem`, `HasImplicitMod`, `AnyEnchantment`
- `IsVaalUnique`, `HasVaalUniqueMod` (seen in the user's PoE2 uniques rules)

**Named / string**
- `Class "..."` — item class (PoE2 classes include: `Bows`, `Crossbows`, `Quarterstaves`,
  `Spears`, `Wands`, `Sceptres`, `Staves`, `Foci`, `One Hand Maces`, `Two Hand Maces`,
  `Quivers`, `Talismans`, `Body Armours`, `Helmets`, `Gloves`, `Boots`, `Shields`,
  `Bucklers`, `Rings`, `Amulets`, `Belts`, `Stackable Currency`, ...). Verify exact spelling.
- `BaseType "..."` — exact base item name (e.g. `"Sapphire Ring"`, `"Gold"`). The most
  precise lever; pull exact names from poe2db.tw.
- `Rarity` — see above.
- `HasExplicitMod`, `HasEnchantment`, `HasInfluence` — verify PoE2 support before use.

---

## Actions (styling & alerts)

**Colors** — `R G B [A]`, each `0–255`:
- `SetTextColor R G B A`
- `SetBorderColor R G B A`
- `SetBackgroundColor R G B A` (default alpha 240)

**Text size**
- `SetFontSize N` — `1–45` (in-game usable range ~18–45; the user's filters use up to 37).

**Sounds**
- `PlayAlertSound Id Volume` — `Id` 1–16, `Volume` 0–300 (default 50).
- `PlayAlertSoundPositional Id Volume` — positional variant.
- `CustomAlertSound "file" [Volume]` — custom sound file (e.g. the bundled `uwu.mp3`),
  volume 0–300 (default 100). `CustomAlertSoundOptional` to tolerate a missing file.
- `DisableDropSound` / `EnableDropSound` — toggle the default drop sound.

**Map / world**
- `MinimapIcon Size Colour Shape`
  - `Size`: `0` (largest) – `2` (smallest).
  - `Colour`: Red, Green, Blue, Brown, White, Yellow, Cyan, Grey, Orange, Pink, Purple.
  - `Shape`: Circle, Diamond, Hexagon, Square, Star, Triangle, Cross, Moon, Raindrop, Kite,
    Pentagon, UpsideDownHouse.
- `PlayEffect Colour [Temp]` — colored beam on the ground. Same colour set as above.
  `Temp` shows only briefly when the item drops.

---

## PoE2 differences to remember
- **No linked sockets / colors-as-links.** PoE2 has no `LinkedSockets`/`SocketGroup` link
  mechanic; `Sockets N` is just a count. Don't author link-based rules.
- **Waystones, not maps.** Use `WaystoneTier` for endgame map tiering.
- **Different base/class names.** Many PoE1 base types don't exist; always confirm via
  poe2db.tw before adding a `BaseType`.
- Existing filters in this repo were generated/edited with **poe2filter.com** and use a
  `Show # Description (slug)` naming convention — keep that style for consistency.

## Tier & color system (Dracula palette)

Color signals **value/desirability**. Text + border use the tier color; background is the
basic Dracula bg `40 42 54 255`. Beams/sounds only on the most valuable. These tier colors
are stable across leagues — only *which items* sit in each tier changes per season.

> **Native rarity color is preserved on uncurated equipment.** Tier *colors* apply only to
> **curated** drops — specific named bases (`BaseType ==` lists), currency, gems, and the
> Recycle/unique rules. Generic equipment/jewellery/flask/charm blocks that match by
> `Class` + `Rarity` alone must **not** repaint the item: omit `SetTextColor` /
> `SetBorderColor` / `SetBackgroundColor` so the game's native rarity color (Normal white,
> Magic blue, Rare yellow) shows through. For those blocks, **emphasis is conveyed by font
> size only** — bigger = more relevant, font 21 = de-emphasized.

| Tier | Meaning | Color | RGB (text+border) | Alert |
|---|---|---|---|---|
| **S** | top | Pink | `255 121 198 255` | `CustomAlertSound "uwu.mp3" 300` + beam + minimap |
| **A** | excellent | Purple | `189 147 249 255` | `CustomAlertSound "uwu.mp3" 300` + beam + minimap |
| **B** | high | Red | `255 85 85 255` | `PlayAlertSound` + beam + minimap |
| **C** | notable | Yellow | `241 250 140 255` | minimap icon |
| **D** | minor | Cyan | `139 233 253 255` | — |
| **E** | baseline | White | `248 248 242 255` | — |
| **F** | unwanted | *native rarity color* | (no color override) | **`Hide` (Alt-reveals it) — styled small font 21 + `DisableDropSound`** |
| **Recycle** ★ | salvage value | Green | `80 250 123 255` | green minimap icon + beam; overrides tier |

Full Dracula RGB reference: bg `40 42 54` · fg/white `248 248 242` · cyan `139 233 253` ·
green `80 250 123` · orange `255 184 108` · pink `255 121 198` · purple `189 147 249` ·
red `255 85 85` · yellow `241 250 140`. (Source: dracula/dracula-theme.)

**Rules:**
- **Uniques are highlighted by price tier, never blanket-restyled.** `sync` fetches per-unique
  prices from poe.ninja's `stash/current/item/overview` endpoint and records each with its
  filter **base type** (e.g. `Voidforge → Infernal Sword`). `generate` groups those bases into
  `Show # Unique <tier>` blocks (`Rarity == Unique` + `BaseType ==`, deduped per base to the
  highest tier). Price-tiered unique blocks get the tier's text/border/beam **plus a distinct
  bronze background `SetBackgroundColor 175 96 37 240`** so they read as "valuable unique"
  regardless of tier color. Uniques whose base is *not* priced get no rule — they fall through
  to native game style. There is still **no blanket restyle of all `Rarity == Unique`**; only
  curated/priced bases are touched. (Caveat: a `BaseType` match highlights every unique on that
  base, since PoE2 filters cannot target a unique by name.)
- **Unwanted items are hidden, not deleted.** The `hidden` style uses `Hide` — off the screen
  by default but **revealable by holding Alt** in-game. Hidden blocks keep no color overrides
  (native rarity color), font 21, and `DisableDropSound`, so an Alt-reveal renders them
  de-emphasized, not in the game default. The `hidden` style currently applies to equipment
  (gear fallback rule), jewellery (rings/amulets/belts fallback rule), and the flask/charm
  fallback rule.

**Flasks & charms** have three show rules (native rarity color throughout): quality `≥1` always
show (highest priority); `AreaLevel ≤25` show all rarities; `AreaLevel ≤65` show `Magic` or
better (`>=Magic`). Deep endgame (AreaLevel ≥78) surfaces Normal-rarity charms. Everything else
falls to the `hidden` flask/charm fallback rule.

**Jewels** (`Class "Jewels"`) have **no rule in `config.json`** — they fall through to the game
default (GEN-04), which is what the emitter's `generate_no_unique_or_jewels_blocks` test holds.
The older "always shown, font 32, cyan diamond (`jewels-all`)" description came from the
hand-authored poe2filter.com filters in `reference-old-filters/` and has not described
`universal.filter` since v0.2. The class *is* offered in the overlay's class picker (below), so a
dynamic rule can target it — that is the editor, not the generated baseline.
- **Recycle** = items with **both `Sockets >= 1` and `Quality >= 1`**. Shown green regardless
  of zone level; the Recycle rule is first in the rules array so it wins before any hide rule.

## Declarative rule model

`universal.filter` is generated by compiling the `rules` array in `config.json`. This is the
authoritative description of how the filter is produced (v0.2+). There is no separate
leveling/endgame file — a single generated `universal.filter` handles all area levels because
the rules themselves carry `areaLevel` conditions.

### Rules array

`rules` is an **ordered array** of rule objects; the first matching rule wins
(first-match-wins, identical to how the filter engine evaluates blocks). Each rule compiles to
exactly one `Show` or `Hide` block — the disposition is determined by the `action` field of the
referenced style. A rule with no `when` is a catch-all that matches everything.

```json
{ "name": "band <=5: all armour any rarity", "style": "unchanged",
  "when": { "areaLevel": { "max": 5 }, "class": ["Body Armours", "Helmets", ...] } }
```

### `when` condition vocabulary

All fields are optional. All present conditions must match (AND semantics within a rule).

| Field | JSON type | Emitted filter condition |
|---|---|---|
| `class` | `["ClassName", ...]` | `Class "ClassName" "ClassName" ...` (bare, match-any) |
| `rarity` | compact string | `Rarity == Rare` / `Rarity >= Magic` / `Rarity <= Rare` (bare value = `==`) |
| `areaLevel` | `{ "min": N, "max": N }` | `AreaLevel >= N` and/or `AreaLevel <= N` |
| `baseArmour` | `{ "eq": N }` / `{ "min": N }` | `BaseArmour N` / `BaseArmour >= N` |
| `baseEvasion` | `{ "eq": N }` / `{ "min": N }` | `BaseEvasion N` / `BaseEvasion >= N` |
| `baseEnergyShield` | `{ "eq": N }` / `{ "min": N }` | `BaseEnergyShield N` / `BaseEnergyShield >= N` |
| `quality` | `{ "min": N }` | `Quality >= N` |
| `sockets` | `{ "min": N }` | `Sockets >= N` |
| `baseType` | `["Base Name", ...]` | `BaseType == "Base Name" "Base Name" ...` |
| `unidentifiedTier` | `{ "min": N }` | `UnidentifiedItemTier >= N` |

Note: `min: 1` on a defence attribute emits `> 0` (e.g. `BaseEnergyShield > 0`), matching the
oracle filter syntax. The `max` field on `NumericCondition` emits `<= max` (available for
future use).

> Confirmed working in-game (author-tested, v0.2): PoE2 honors
> `BaseArmour`/`BaseEvasion`/`BaseEnergyShield` as base-defence filters and
> `UnidentifiedItemTier` as a threshold condition. (The recycle `Sockets>=1 & Quality>=1`
> threshold remains a tuning knob, not a correctness question.)

### Named styles

`styles.json` defines all named styles; the overlay's style picker lists them in file order.
`S`, `A`, `B`, `C`, `D`, `E`, `F`, `unchanged` and `hidden` are required; any other style is
optional. An unknown style in an enabled rule, a missing required style, a duplicate name, a name
differing only by case from a required one, or a bad field (unknown colour, unsafe custom sound
name) is a load-time error naming the problem. A style's `action` decides Show/Hide for the rule
using it, and Hide styles may set any visual (shown on Alt-reveal). Current styles:

| Name | Action | What it does |
|---|---|---|
| `S` | Show | Pink text/border, Dracula bg, font 37, Pink beam, Pink circle minimap, custom sound uwu.mp3 |
| `A` | Show | Purple text/border, Dracula bg, font 37, Purple beam, Purple circle minimap, custom sound uwu.mp3 |
| `B` | Show | Red text/border, Dracula bg, font 37, Red beam, Red circle minimap, alert sound |
| `C` | Show | Yellow text/border, Dracula bg, no font override, Yellow beam, Yellow circle minimap, alert sound |
| `D` | Show | Cyan text/border, Dracula bg, no font override, White beam, White circle minimap |
| `E` | Show | White text/border, Dracula bg, font 29 |
| `F` | Show | White text/border, Dracula bg, font 25 (de-emphasized; used as fallback before a `hidden` rule) |
| `Recycle` | Show | Green text/border, Dracula bg, Green beam, Green cross minimap |
| `unchanged` | Show | No color/font/minimap/beam overrides — native rarity color and game defaults |
| `hidden` | Hide | No color overrides, font 21, DisableDropSound — Alt-reveals as de-emphasized |
| `kite` | Show | No color overrides (native rarity), Orange beam, Orange Kite minimap, alert sound. A **locator** style, not a value tier — it says "go look over there" without claiming worth, which is why its orange sits outside the Dracula ladder. Renamed from `unidentified` on 2026-09-07: the old name described the rules that happened to use it rather than what it does, so seeing it offered for an unrelated rule in the overlay picker read as nonsense. |
| `exceptional` | Show | Pink text/border, Dracula bg, no font override, Pink beam, Pink Cross minimap, no sound — the exceptional-gear highlight, kept separate from `S` so it can be retuned independently |

### `classGroups`

`classGroups` is a documentation-only alias map (e.g. `"buildWeapons": ["Staves","Wands","Foci"]`).
It is **not consumed by the compiler** — rules inline their full class lists in `when.class`.
Use `classGroups` as a reference when authoring new rules to keep class lists consistent.

### Fallthrough

Any item not matched by a rule is left to the game default (uniques, jewels, gems, quest
items, waystones, trials, etc.). See the `"fallthrough"` field in `config.json` for the note.

## Authoring conventions for this repo
- New filters derive from the curated item lists in `reference-old-filters/end_endgame.filter`
  (membership reshuffled per league), restyled into the tier system above.
- **Ignore the trailing config block** that poe2filter.com appended to the old filters
  (`### Filter Configuration` / `# [[Config Start]]` … commented JSON). Do not carry it into
  new filters — `universal.filter` already has it stripped.
- Order blocks **most specific → most general**; tier F is the catch-all.
- Keep block comments descriptive: `Show # What it matches (kebab-slug)`.
- The `uwu.mp3` custom sound lives in `assets/` (version-controlled). For the filter to play
  it in-game, the mp3 must also be in the game folder (it currently is).
- After editing a `.filter`, copy it into the game's config folder (path above) to take effect.
  We will not auto-copy unless asked.
