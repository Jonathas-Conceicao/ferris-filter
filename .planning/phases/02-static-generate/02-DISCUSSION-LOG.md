# Phase 2: Static Generate - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-05-24
**Phase:** 2-Static Generate
**Areas discussed:** Output fidelity (exact vs re-baseline), Acceptance test mechanism, config.json schema refactor (D-14), In-game equivalence verification

---

## Output fidelity: exact vs re-baseline

Context surfaced during scouting: the hand-built universal.filter uses inconsistent intra-block
action ordering across styles (Currency S and Vault Keys S share one order; the Recycle block
uses a different order), so no single canonical order reproduces the current file byte-for-byte.

### Fidelity approach
| Option | Description | Selected |
|--------|-------------|----------|
| Re-baseline (canonical order) | One canonical action order, regenerate + lock as new golden master | ✓ |
| Exact byte-match of today's file | Encode per-style action order to match current file exactly | |
| Re-baseline but minimize churn | Canonical order chosen to match the majority of blocks | |

**User's choice:** Re-baseline (canonical order)

### Diff guardrail
| Option | Description | Selected |
|--------|-------------|----------|
| Intra-block action reorder only | Only action reordering may change; show normalized diff before locking | |
| Allow minor cleanup too | Also permit spacing/comment-header cleanups as long as behavior is unchanged | ✓ |

**User's choice:** Allow minor cleanup too (bar = behavioral equivalence)

---

## Acceptance test mechanism

### Test oracle
| Option | Description | Selected |
|--------|-------------|----------|
| insta snapshot + artifact guard | insta .snap as lock + guard test that committed universal.filter == generator output | ✓ |
| Assert == committed universal.filter only | Single source of truth, relies on review discipline | |
| insta snapshot only | .snap lock; artifact could drift | |

**User's choice:** insta snapshot + artifact guard

### SHA-256
| Option | Description | Selected |
|--------|-------------|----------|
| Equality is enough | Snapshot/string equality proves byte-identity; no explicit SHA | ✓ |
| Add explicit SHA-256 check | Hash + compare in addition to snapshot | |

**User's choice:** Equality is enough

### Default output
| Option | Description | Selected |
|--------|-------------|----------|
| Yes — repo-root universal.filter | Default output = repo-root universal.filter (live, install.bat-copied) | ✓ |
| Write elsewhere by default | Default to build dir; root file updated intentionally | |

**User's choice:** Yes — repo-root universal.filter

---

## config.json schema refactor (D-14 latitude)

### Band rules
| Option | Description | Selected |
|--------|-------------|----------|
| Refactor to typed rules | Closed enum (Any/RareOnly/MagicPlusRare/MagicOnly/QualityOnly), validated on load | ✓ |
| Parse strings as-is | Keep freeform strings; parse at generate-time | |

**User's choice:** Refactor to typed rules

### Styles model
| Option | Description | Selected |
|--------|-------------|----------|
| Typed struct of named styles | S/A/B/C/D/E/F/Recycle/unchanged/hidden as struct fields; deterministic, complete-by-construction | ✓ |
| Ordered map (IndexMap/BTreeMap) | Keyed map with deterministic iteration | |
| Keep HashMap, lookup-only | No change; guarantee determinism by lookup discipline | |

**User's choice:** Typed struct of named styles

---

## In-game equivalence verification

### Equivalence check
| Option | Description | Selected |
|--------|-------------|----------|
| Automated semantic-equivalence check | Normalize old & new filters, assert block-by-block equivalence | |
| Manual in-game smoke test | Human-UAT: user loads regenerated filter in PoE2 and confirms behavior | ✓ |
| Both | Automated gate + optional manual spot-check | |

**User's choice:** Manual in-game smoke test (authoritative gate; no automated comparator built)

### Original reference
| Option | Description | Selected |
|--------|-------------|----------|
| Git history is enough | Equivalence check / audit reads original from prior git revision | ✓ |
| Keep a reference copy | Commit original as tests/reference/universal.filter.orig | |

**User's choice:** Git history is enough

### Additional requirement (free-text)
**User:** "once execution for this phase is done, have a diff ready for the config and the filter
output for human review."
**Captured as:** D-14 — at end of Phase 2 execution, produce human-reviewable diffs for both
config.json (old → refactored) and universal.filter (old → regenerated) for the user to review,
alongside the in-game smoke test.

---

## Claude's Discretion
- Canonical action order specifics; enum variant names + JSON spelling of refactored rules; emitter
  code structure; derivation of block/section order; header comment block reproduction vs cleanup.

## Deferred Ideas
- Price-driven tiering (currency + uniques, prices.json) → Phase 3.
- sync / poe.ninja networking → Phase 4.
- Automated semantic-equivalence comparator → not built (manual in-game test chosen instead).
