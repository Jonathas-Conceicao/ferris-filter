# Phase 13: Overlay Shell - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-09-04
**Phase:** 13-Overlay Shell
**Areas discussed:** Crate & binary shape, Summon & dismiss behavior, Window shape & persistence,
Phase-13 payload, Game awareness

**Format note:** presented in prose rather than option menus, per the recorded preference that open
design questions be asked as prose because the author's context often reframes the question. The
tables below reconstruct the alternatives that were laid out.

---

## Crate & binary shape

| Option | Description | Selected |
|--------|-------------|----------|
| Cargo workspace | Second crate depending on `ferrisfilter`; cleanest separation, most churn | |
| Optional dependency + feature flag | No restructuring; `--features` to remember; CLI stays lean | |
| Plain dependency, GUI is the default build | `cargo build` produces the GUI; CLI binaries link egui too | ✓ |

**User's choice:** "gui should be default build, that's gonna be the common usage of the tool"
**Notes:** This inverted the assumption in `ARCHITECTURE.md`, which sketched a workspace split on
the premise that the CLI stayed the primary front end. Once the GUI is the default, the workspace
split buys much less, so the single crate with a third `[[bin]]` was kept.

### Binary naming (follow-up)

| Option | Description | Selected |
|--------|-------------|----------|
| New `ffoverlay` binary alongside `ff` | GUI gets a new name; CLI names unchanged | |
| `ff` becomes the GUI; CLI moves to `ff-cli` | The short name is the common usage | ✓ |

**User's choice:** "Have the ff be the new GUI binary and ff-cli the old usage style"
**Notes:** Verified during discussion that all 25 integration-test call sites use
`cargo_bin("ferrisfilter")` and none reference `ff`, so the rename touches zero tests.
`default-run = "ff"` already pointed at that name and needs no change.

---

## Summon & dismiss behavior

| Option | Description | Selected |
|--------|-------------|----------|
| Show and take focus | Instant to use; steals keyboard from the game, risks stray keystrokes | |
| Show without focus; cursor-leave dismisses | No hook needed, focus never touched; feels like a hover-menu | |
| Show without focus; click-inside activates, focus-loss dismisses | Standard OS click-away; one focus handoff after you interact | ✓ |
| Show without focus; global mouse hook watches outside clicks | Matches the literal request; needs system-wide input monitoring | rejected |

**User's choice:** "key makes the overlay visible but we only take input if cursor moves over to the
screen we will render, any click outside of it should also turn-off the visibility" — then, once the
cost was laid out: "cut the 'click outside dismisses' if it adds too much complexity and we can
reevaluate it later, 'let a click inside activate the window normally, then dismiss on focus-loss'
sounds perfect, this is actually what I've tried to request before, but just go with a simple
implementation"

**Notes:** The global mouse hook was flagged as contradicting the project's no-input-monitoring
stance and was dropped. A consequence was raised and accepted: because focus follows the click, the
stripe *can* receive typed text, which removes a would-be mouse-only constraint on Phase 14's
numeric controls ("Yeah, keyboard follows the overlay focus").

### Hotkey (follow-up)

| Option | Description | Selected |
|--------|-------------|----------|
| `Alt+F` | Suggested default; unlikely to collide | |
| `Shift+Z` | Author's pick | ✓ |

**User's choice:** "let's have shift + Z for now as our toggle keybind"
**Notes:** Flagged that `global-hotkey` registers system-wide, so `Shift+Z` is swallowed in every
application including PoE2's chat box. Accepted as a "for now" choice; it is a one-constant change.

---

## Window shape & persistence

| Option | Description | Selected |
|--------|-------------|----------|
| Decorated panel, ~500×700, position remembered | Title bar gives drag and close for free | |
| Full-height narrow stripe, minimum width | All UI built inside the stripe | ✓ |

**User's choice:** "do something that takes over vertical space and only use the required horizontal
space, like a stripe, and inside said stripe you can build any functionality for configuring the
loot filter"

**Notes:** Borderless follows from the shape — a title bar would eat the vertical space the stripe
exists to preserve — which in turn makes the tray menu the only way to quit. Position and width
persistence dropped for this phase ("keep everything simple"); there is no settings file yet.
Edge, width and visual style were explicitly left to Claude: "I have no preference on style for
now, go with what you think fits the design well."

### System tray (author addition)

Not on the original list — raised by the author:

**User's request:** "be sure to have a system-tray icon (we can use anything for now, go for a basic
ferris (rust mascot)) so we know the overlay is running"
**Notes:** This settled an unasked question — the process lives in the tray and the stripe is a view
it shows and hides. Right-click Show/Quit plus left-click toggle were proposed and accepted.

---

## Phase-13 payload

| Option | Description | Selected |
|--------|-------------|----------|
| Blank panel | Proves only that the window and hotkey work | |
| Read-only status view | Store path, rule count, destination, price source | rejected |
| Read-only rule list | The rules from `dynamic.json` in ordinal order, nothing else | ✓ |
| Full editor + Reset/Apply | Everything the author asked for — but this is Phase 14 | deferred |

**User's choice:** "For now focus to adding to the GUI only the configurable options, so it shows
all rules and a way to add new rules, the options to edit them and the buttons to reset/apply, it
doesn't need any pricing information or details for now, store path nor anything like that"

**Notes:** This is Phase 14's entire content (EDIT-01..09), so it was flagged against the roadmap
boundary rather than silently absorbed. The author's intent — *don't clutter the GUI with status
chrome; its content is the rule editor* — was honored by making the Phase 13 payload the **rule
list, read-only**: no paths, no price source, no destination folder. The editing controls and the
Reset/Apply buttons land in Phase 14, in the space the list already occupies. The alternative
(collapsing Phases 13 and 14 into one) was offered and not taken; keeping the split means a failed
test against the running game points at the stripe rather than at the editor.

---

## Game awareness

| Option | Description | Selected |
|--------|-------------|----------|
| Detect PoE2 / exclusive fullscreen | Prevents a confusing silent failure; needs window enumeration | |
| Document the windowed-mode requirement | No new surface area; the user hits it once | ✓ |

**User's choice:** Not contested — recommendation stood.
**Notes:** Partly overtaken by events: show-without-activate likely needs the raw `HWND` anyway, so
the "avoid a Windows API dependency" argument for skipping detection no longer holds on its own.
The remaining argument — new surface area for a one-time failure — does.

---

## Claude's Discretion

- Visual design of the stripe → the Dracula palette already specified in `CLAUDE.md`, so the GUI
  and the filter it writes read as one tool.
- Which screen edge the stripe docks to, and its width.
- Layout of the read-only rule list.
- Mechanism for show-without-activate (native eframe/winit vs. raw `HWND`) — a research question.
- Tray-icon crate selection.

## Deferred Ideas

- Click-outside-dismiss as its own mechanism (would need a global mouse hook).
- Configurable hotkey — hardcoded `Shift+Z` for now.
- Position / width persistence — needs a settings file that does not exist yet.
- PoE2 process and exclusive-fullscreen detection.
- The full rule editor with Reset and Apply — Phase 14.
