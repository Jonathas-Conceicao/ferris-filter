# v0.7.2 autonomous run — items deferred to the end

Collected during `/gsd-autonomous --from 43 --to 50` (started 2026-10-23). The author asked for every
human review and input to be stacked here and presented at the end, before Phase 50's author review
(REV-05). Each phase agent appends its items under its own heading.

## Deferred manual checks

### Phase 45: Economy tier-range names

1. Open the RULES tab on a filter with an economy section, press Apply once: every economy row name now ends with its range (`S tier Currency >= 30 chaos` live, `... exalt` under Fallback); the first Apply after upgrading rewrites all old names once (expected per CONTEXT).
2. Longer labels such as `D tier Map Fragments & Pinnacle Keys < 8 chaos` are cut cleanly with an ellipsis on collapsed rows and the style chip / flags still fit.
3. On the Economy tab change a cutoff (e.g. E to 1): the staged / previewed economy rows on RULES rename (`D tier ... < 1 chaos`) before Apply.
4. Search on RULES finds economy rows by the new text (e.g. typing `< 2 chaos`).

## Art substitutes for review (Phase 48 / 47)

## Open gaps and judgement calls

- Phase 45: a non-bottom cutoff below 0.005 renders as `0` under the locked 2-decimal rule (no shipped file hits it); mixed-unit tier files label each tier with its upper neighbour's own unit (and `div` if a user uses it); two tiers with the same chaos-equivalent floor (only possible across units) make the lower one read `< X` with X equal to its own floor.
- Phase 45 (code review WR-01, author call): the locked "at most 2 decimals" rule can misstate a range when a cutoff has more decimals: 0.025 exalt prints `< 0.03 exalt` (an item at 0.027 is really in the tier above), a top floor of 1.999 prints `>= 2`, and a cutoff below 0.005 prints `< 0`. Options: print more digits only when a cutoff is not exact at 2 decimals, or have the Tiers editor reject cutoffs with more than 2 decimals. Formatter: `fmt_amount` in `src/pricer.rs`.
- Phase 45 (code review IN-01/IN-03): a `-0` cutoff is accepted and prints `>= -0` / `< -0`; a unique name such as `A tier Uniques < 30 chaos` describes the most valuable unique on each base, while the block highlights every unique on those bases.
