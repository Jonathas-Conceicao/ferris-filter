# Phase 1: Foundation - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-05-23
**Phase:** 1-Foundation
**Areas discussed:** CLI shell & error reporting, JSON modeling depth & strictness, New file schemas (tiers.json & prices.json), Crate layout & config rename

---

## CLI shell & error reporting

### clap API style
| Option | Description | Selected |
|--------|-------------|----------|
| Derive macros | Declarative `#[derive(Parser)]` structs/enums; idiomatic, compact | ✓ |
| Builder API | Programmatic `Command::new().arg()`; verbose, runtime-dynamic | |

**User's choice:** Derive macros

### Error reporting strategy
| Option | Description | Selected |
|--------|-------------|----------|
| miette + thiserror | Typed errors rendered by miette with JSON span + help line; best UX | ✓ |
| serde_path_to_error + thiserror | Dotted field path printed plainly; lighter | |
| anyhow only | Simplest; field-path quality depends on serde's own message | |

**User's choice:** miette + thiserror

### Binary invocation name
| Option | Description | Selected |
|--------|-------------|----------|
| ferrisfilter | Lowercase binary matching success-criteria text | |
| ferrisfilter + ff alias | Also ship a short `ff` alias | ✓ |

**User's choice:** ferrisfilter + ff alias

---

## JSON modeling depth & strictness

### Semantic parse depth
| Option | Description | Selected |
|--------|-------------|----------|
| Hybrid: enum closed sets, defer band DSL | Enum colors/shapes/action/armourType keys; band rarity rules stay strings | (Claude's discretion) |
| Full typed enums now | Also parse band rarity-rule DSL into enums in Phase 1 | |
| Structural only | Plain strings/structs, no enum validation | |

**User's choice:** "claude can decide on it, keep in mind that structure, labels and so on on config.json can be changed to fit the design better, no design until this milestone ends is set in stone."
**Notes:** Delegated parse-depth to Claude (leaning hybrid) AND granted milestone-wide latitude to redesign config.json schema/labels (captured as D-14).

### Unknown/extra keys
| Option | Description | Selected |
|--------|-------------|----------|
| Strict (deny_unknown_fields) | Reject unrecognized keys; typos error loudly | ✓ |
| Lenient | serde default; ignore unknown keys | |

**User's choice:** Strict (deny_unknown_fields)

### Round-trip meaning
| Option | Description | Selected |
|--------|-------------|----------|
| Semantic round-trip | deserialize→serialize→deserialize stable; not byte-identical | ✓ |
| Byte-identical JSON | Re-serialize reproduces source exactly | |

**User's choice:** Semantic round-trip

---

## New file schemas: tiers.json & prices.json

### Tier cutoff units
| Option | Description | Selected |
|--------|-------------|----------|
| Unit-tagged (chaos or div) | Per-entry unit; div cutoffs normalized to chaos at generate-time | ✓ |
| Chaos-only numbers | All cutoffs raw chaos; div boundaries pre-converted, static | |

**User's choice:** Unit-tagged (chaos or div)

### tiers.json structure
| Option | Description | Selected |
|--------|-------------|----------|
| Ordered list of lower-bounds | Array [{tier, min, unit}] ascending; validate complete + increasing | ✓ |
| Map tier → cutoff | Object keyed by tier letter; ordering imposed by code | |

**User's choice:** Ordered list of lower-bounds

### prices.json shape
| Option | Description | Selected |
|--------|-------------|----------|
| Wrapper + ordered entries | {league, fetchedAt, items:[{name, baseType?, exalt, chaos, div}]} | ✓ |
| Flat array, metadata inline | Top-level array with header entry for metadata | |

**User's choice:** Wrapper + ordered entries

---

## Crate layout & config rename

### Crate structure
| Option | Description | Selected |
|--------|-------------|----------|
| lib + thin bin | Core logic in lib; main.rs dispatches; clean test imports | |
| Single binary crate | Everything under src/main.rs + modules | ✓ |

**User's choice:** Single binary crate

### Rename filter-model.json → config.json now?
| Option | Description | Selected |
|--------|-------------|----------|
| Rename now in Phase 1 | git mv as part of Foundation; models target config.json | ✓ |
| Defer rename to Phase 2 | Keep filter-model.json until the generator consumes it | |

**User's choice:** Rename now in Phase 1

### Currency base-lists home
| Option | Description | Selected |
|--------|-------------|----------|
| Defer to Phase 2 | Phase 1 doesn't model currency lists | |
| Model now as a tracked input | Introduce a currency base-list input, model in Phase 1 | ✓ (variant) |

**User's choice:** "Create the file now with data on universal.filter" — extract the curated currency base-lists out of universal.filter into a new tracked data file now, populated with the existing data, and model it in Phase 1.

---

## Claude's Discretion

- Hybrid parse-depth field-by-field specifics (which fields become enums vs strings).
- Currency base-list file name and exact schema.
- Test-harness mechanics under a single-binary crate.
- Edition/MSRV, dependency versions, module naming, exit-code values.

## Deferred Ideas

- Filter-generation logic → Phase 2.
- Div-cutoff chaos normalization → Phase 3.
- Replacing curated currency lists with price-driven tiering → Phase 3.
- Real poe.ninja fetching + league flags for `sync` → Phase 4.
