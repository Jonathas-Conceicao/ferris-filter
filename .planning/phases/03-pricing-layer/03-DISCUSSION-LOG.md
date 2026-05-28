# Phase 3: Pricing Layer - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-05-24
**Phase:** 3-Pricing Layer
**Areas discussed:** No-prices.json fallback (GEN-05), prices.json schema & migration, Uniques (capability vs highlighting), Output-preservation seeding, + the anchor-orb re-tier collision

**Origin:** User flagged during the Phase 2 checkpoint that value data should live in prices.json
(sync's target), not a separate currency.json — pulling the price-driven tiering forward and
dropping currency.json as a generate input. This matches PROJECT.md's stated intent.

---

## No-prices.json fallback (GEN-05)

| Option | Description | Selected |
|--------|-------------|----------|
| Fall back to currency.json tiers | prices.json primary; absent → curated currency.json tiers + warning (fully functional offline) | |
| No currency blocks + warning | currency.json purely reference/seed; absent → only config-driven blocks | ✓ |

**User's choice:** No currency blocks + warning (currency.json never read by generate; matches roadmap criterion 3's literal wording and the "drop currency.json" intent)

---

## prices.json schema & migration

### Item schema
| Option | Description | Selected |
|--------|-------------|----------|
| name + baseType + class + category + prices | Full shape; supports uniques (baseType ≠ name) | ✓ |
| Lighter: name + class + category + prices | Uses name as BaseType; breaks for uniques | |

**User's choice:** name + baseType + class + category + prices

### Block grouping
| Option | Description | Selected |
|--------|-------------|----------|
| By category, first-appearance order | Group by category field; emit in item-array order; tiers S→F within | ✓ |
| Explicit category-order list | Separate top-level order array | |

**User's choice:** By category, first-appearance order

---

## Uniques: capability vs highlighting now

### Approach
| Option | Description | Selected |
|--------|-------------|----------|
| Build capability, seed empty | TIER-03 logic + collision now; committed prices.json has no uniques (output unchanged); fixture-tested | ✓ |
| Scope real highlighting in now | Add unique blocks now → re-baseline | |

**User's choice:** Build capability, seed empty

### Unique block shape (forward spec)
| Option | Description | Selected |
|--------|-------------|----------|
| Rarity==Unique + BaseType, by tier | Block matches Rarity==Unique + BaseType, styled by price tier; untracked fall through | ✓ |
| Decide later | Lock shape when uniques are added | |

**User's choice:** Rarity==Unique + BaseType, by tier

---

## Output-preservation seeding

### Seed values
| Option | Description | Selected |
|--------|-------------|----------|
| Tier lower-bound value | F→0.1c … S→5div; deterministic, in-band, obvious placeholder | ✓ |
| Tier midpoint value | Middle of band; more price-like but arbitrary, S unbounded | |

**User's choice:** Tier lower-bound value

### Snapshot gate
| Option | Description | Selected |
|--------|-------------|----------|
| Yes — snapshot is the hard gate | Reworked generate must reproduce Phase 2 output byte-for-byte | ✓ (amended) |
| Allow a small re-baseline | Permit minor intentional changes | |

**User's choice:** Yes — hard gate … then AMENDED by the anchor-retier finding below.

---

## Anchor-orb re-tier collision (surfaced during discussion)

**Finding (Claude):** The lower-bound seeding can reproduce every curated tier EXCEPT the three
reference orbs, whose values are pinned (cross-rate anchors) and don't match their curated tiers
under the cutoffs: Chaos 1c → E (curated C); Exalt ~4.5c → D (curated C); Divine 220c=1div → A
(curated S, but S=5div so Divine is structurally A). Byte-identity is impossible for these.

| Option | Description | Selected |
|--------|-------------|----------|
| Accept a small re-baseline | Let the 3 orbs re-tier (Chaos→E, Exalt→D, Divine→A); new snapshot + review; everything else identical | ✓ |
| Tier-pin / override mechanism | Per-item tier override to preserve exact output | |
| Re-tune tiers.json cutoffs | Bend cutoffs to fit curated tiers | |

**User's choice:** Accept a small re-baseline (the feature working as intended; the "hard gate"
answer above is thus limited to: no changes BEYOND the anchor re-tiering and its direct consequences).

---

## Claude's Discretion
- Cross-rate math details; TIER-04 error wording; unique block placement; emitter refactor structure;
  whether the seeded prices.json is generated via a one-time migration script or hand-authored.

## Deferred Ideas
- poe.ninja sync populating prices.json → Phase 4.
- Emitting unique highlight blocks in committed output → when sync provides priced uniques.
- Tier-pin/override → considered, not adopted.
- sync → class/category mapping → Phase 4 discussion.
