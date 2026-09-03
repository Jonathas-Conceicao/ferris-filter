# Phase 12: Dynamic Rule Store — Discussion Log

**Date:** 2026-09-03
**Mode:** default (interactive, prose-first)
**Participants:** author + Claude

> Human reference only. Downstream agents read `12-CONTEXT.md`.

## Pre-discussion scout

Three findings shaped the questions before any were asked:

1. **`emitter::generate` needs a full `Config`**, and the economy sections use only `styles` plus
   the priced blocks. So a dynamic filter falls out of feeding it a Config whose rules are the
   dynamic ones — no new emitter path.
2. **`emit_rules` already has an ordinal fallback:** `rule.name.as_deref().unwrap_or(&format!("rule
   {idx}"))`. Ordinal identity (DYN-02) was nearly free.
3. **`install::default_dest()` already resolves the game folder**, solving DYN-07's write target.

## Areas discussed

### 1. What `dynamic.json` contains on first run

This had been asked once during milestone scoping and lost when the conversation was paused, so it
was re-raised.

**Author:** *"i've created dynamic_base.json copied from config.json with some hand alterations to
serve as a base for the dynamic config I want, it can also be used as a base for a 'reset' we can
have to start a new filter from 'scratch' without losing everything."*

**Verified immediately.** `dynamic_base.json` turned out to be a **full `Config`** — same ten
top-level keys as `config.json`, 10 rules instead of 20, **zero `areaLevel` conditions** (already
stripped by hand), full styles block. Running `generate --config dynamic_base.json` produced a
valid 69 Show / 1 Hide filter *with the economy sections included*.

**Impact (D-01, D-03):** this collapsed the phase's biggest assumed task. The research's
`{version, rules: [...]}` envelope sketch is superseded; the store is a full Config, and Phase 11's
`load_inputs`/`compile` already compiles it. DYN-06 was effectively already satisfied.

**Seeding/reset (D-12…D-14):** `dynamic_base.json` is the committed template, `dynamic.json` the
git-ignored working copy seeded from it; reset re-copies.

### 2. How a disabled rule persists

**Author:** *"yeah, add an enabled label, a disabled rule should still be in the json but not be
generated (or be generated as comment only) to the actual filter, and we should have a delete
option in case we actually want the rule gone."*

**Decision (D-07…D-09):** `Rule.enabled: Option<bool>`, `serde(default, skip_serializing_if)`,
absent = enabled — so `config.json` and every fixture round-trip unchanged. Disabled rules stay in
the JSON and are **omitted entirely** from the filter. The author left comment-vs-omit to Claude;
omitting won because the filter is machine-generated and never read by hand.

### 3. `dynamic.filter`'s header — a conflict raised

Claude flagged that `emit_header` hardcodes `# universal.filter` / `# GENERATED FROM config.json`
and is called by **both** paths, so changing the strings would move `seed-universal.filter` and
break `artifact_guard` — violating the milestone acceptance bar the author had set.

**Author:** *"Not sure I understand the conflict you've pointed out, but if the dynamic.filter file
is generated mentioning a 'config.json' and a 'universal.filter' in the comments that's also not a
big issue for now, update the hardcoded strings and tests, or make it parametrizable now if you
think it's best, it's not an important part we shouldn't dwell on it too long."*

**Decision (D-10):** parameterize. The author was willing to re-baseline and explicitly did not
want time spent here — but parameterizing costs the same ~2 lines and is the only option that
leaves the oracle untouched. Taken without further discussion, per the "don't dwell" steer. Body
text left alone (D-11).

### 4. Rule naming and a persistent index

**Author:** *"the json configs might also need to be cleared of the name field and have a
persistent index, although the json's own list ordering is fine by me if you think so. You can
remove the name field if that works better, I'm fine either way, we can break json compatibility
for this new design."*

**Decision (D-04…D-06):** array order is the identity; **no** stored index — it would be a second
source of truth that desyncs on reorder, and reordering is a first-class Phase 14 operation.
Strip `name` from `dynamic_base.json`'s rules.

**A defect this surfaced:** those rules carry `"name": ""` (empty string, key *present*), so the
`unwrap_or("rule {idx}")` fallback never fires and blocks currently emit `Show # ` with nothing
after them. Removing the key makes the existing fallback produce `Show # rule 4` — a traceable
pointer back to the index in `dynamic.json`. **No emitter change needed; it is a data fix.**
Confirmed safe: `config.json` has zero empty names, so `universal.filter` is unaffected.

**Compatibility break offered but not needed:** `Rule.name` is already `Option<String>` and
`enabled` defaults, so nothing breaks.

### 5. Headless entry point

**Author:** *"I'm fine with whatever here."*

**Decision (D-15…D-17):** a CLI subcommand rather than library-only, so the feature is usable and
scriptable the moment this phase lands — which is the reason Phase 12 precedes the overlay. Naming
and flags left to Claude.

## Claude's discretion (recorded)

- Subcommand name, flags, module placement.
- Where the seed/reset helper lives.
- Plan split.
- Whether `dynamic_base.json`'s `meta.name` changes from the currently misleading `"universal"`.

## Deferred ideas

Captured in `12-CONTEXT.md` `<deferred>`. None were author scope creep — reset-as-UI and header
body text were explicitly deferred by the author; the rest are carries from earlier phases.

## Process note

No `AskUserQuestion` menus, per `memory/prefers-discussion-over-option-votes.md`. Vindicated
again: the answer to question 1 was a *file the author had already written*, which no option list
would have anticipated and which removed the phase's largest assumed task.

---

*Phase: 12-dynamic-rule-store*
*Logged: 2026-09-03*
