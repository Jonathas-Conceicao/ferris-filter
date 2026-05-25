# Phase 3 Re-Baseline Diff Artifact

**Plan:** 03-04  
**Date:** 2026-05-24  
**Phase 2 baseline commit:** `89547c8` (feat(02-02): wire cmd_generate + regenerate universal.filter)  
**Phase 3 commit:** in progress (see plan 03-04)

---

## Summary

The Phase 3 generator is price-driven (`prices.json` + `tiers.json`). The committed `prices.json`
uses realistic values for the three anchor orbs (Chaos=1c, Exalt=4.5c, Divine=220c), which
re-tiers them relative to their Phase 2 curated positions.

**Exactly 5 block edits in the `# --- Currency ---` section.** All other categories and all
gear/flask/recycle/header sections are **byte-identical** to Phase 2.

### The 5 Expected Currency Block Changes

| Block | Change |
|-------|--------|
| **Currency S** | `"Divine Orb"` removed (re-tiered from S → A; 220c = A lower bound exactly) |
| **Currency A** | `"Divine Orb"` prepended (moved from S, appears first in A tier group) |
| **Currency C** | `"Chaos Orb"` and `"Exalted Orb"` removed (Chaos → E at 1c; Exalt → D at 4.5c) |
| **Currency D** | `"Exalted Orb"` prepended (moved from C, appears first in D tier group) |
| **Currency E** | `"Chaos Orb"` prepended (moved from C, appears first in E tier group) |

Currency B and Currency F: **unchanged**.

### What Does NOT Change

The following 14 categories are entirely unchanged (verified via diff):
Abyssal Eyes, Ancient Bones, Breach, Delirium, Essences, Expedition, Idols, Incubators,
Map Fragments & Pinnacle Keys, Omens, Runes, Soul Cores, Tablets, Vault Keys.

All gear AreaLevel bands, flask/charm bands, recycle block, and the filter header/footer are
also completely unchanged.

**Block counts:** Phase 2 = 83 Show + 2 Hide. Phase 3 = **83 Show + 2 Hide** (unchanged).

**No `Rarity == Unique` blocks:** verified (zero occurrences).

---

## git diff 89547c8 -- universal.filter

```diff
diff --git a/universal.filter b/universal.filter
index 22fd767..abf7c72 100644
--- a/universal.filter
+++ b/universal.filter
@@ -121,7 +121,7 @@ Hide # flasks/charms fallback  [styles.hidden]
 # --- Currency ---
 Show # Currency S
   Class == "Stackable Currency"
-  BaseType == "Perfect Chaos Orb" "Mirror of Kalandra" "Hinekora's Lock" "Perfect Exalted Orb" "Divine Orb" "Fracturing Orb" "Albino Rhoa Feather" "Vaal Cultivation Orb"
+  BaseType == "Perfect Chaos Orb" "Mirror of Kalandra" "Hinekora's Lock" "Perfect Exalted Orb" "Fracturing Orb" "Albino Rhoa Feather" "Vaal Cultivation Orb"
   PlayEffect Pink
   MinimapIcon 1 Pink Circle
   SetTextColor 255 121 198 255
@@ -132,7 +132,7 @@ Show # Currency S
 
 Show # Currency A
   Class == "Stackable Currency"
-  BaseType == "Perfect Jeweller's Orb" "Core Destabiliser" "Vaal Infuser" "Architect's Orb" "Ancient Infuser" "Crystallised Corruption"
+  BaseType == "Divine Orb" "Perfect Jeweller's Orb" "Core Destabiliser" "Vaal Infuser" "Architect's Orb" "Ancient Infuser" "Crystallised Corruption"
   PlayEffect Purple
   MinimapIcon 1 Purple Circle
   SetTextColor 189 147 249 255
@@ -154,7 +154,7 @@ Show # Currency B
 
 Show # Currency C
   Class == "Stackable Currency"
-  BaseType == "Chaos Orb" "Orb of Chance" "Exalted Orb" "Greater Exalted Orb" "Perfect Orb of Augmentation"
+  BaseType == "Orb of Chance" "Greater Exalted Orb" "Perfect Orb of Augmentation"
   PlayEffect Yellow
   MinimapIcon 2 Yellow Circle
   SetTextColor 241 250 140 255
@@ -164,7 +164,7 @@ Show # Currency C
 
 Show # Currency D
   Class == "Stackable Currency"
-  BaseType == "Arcanist's Etcher" "Armourer's Scrap" "Orb of Alchemy" "Perfect Orb of Transmutation" "Greater Regal Orb" "Glassblower's Bauble" "Gemcutter's Prism" "Chance Shard" "Vaal Orb"
+  BaseType == "Exalted Orb" "Arcanist's Etcher" "Armourer's Scrap" "Orb of Alchemy" "Perfect Orb of Transmutation" "Greater Regal Orb" "Glassblower's Bauble" "Gemcutter's Prism" "Chance Shard" "Vaal Orb"
   PlayEffect White
   MinimapIcon 2 White Circle
   SetTextColor 139 233 253 255
@@ -173,7 +173,7 @@ Show # Currency D
 
 Show # Currency E
   Class == "Stackable Currency"
-  BaseType == "Blacksmith's Whetstone" "Lesser Jeweller's Orb" "Greater Jeweller's Orb" "Artificer's Orb"
+  BaseType == "Chaos Orb" "Blacksmith's Whetstone" "Lesser Jeweller's Orb" "Greater Jeweller's Orb" "Artificer's Orb"
   SetTextColor 248 248 242 255
   SetBackgroundColor 40 42 54 255
   SetBorderColor 248 248 242 255
```

---

## Guardrail Verification

_(Added in Task 2 — see below)_
