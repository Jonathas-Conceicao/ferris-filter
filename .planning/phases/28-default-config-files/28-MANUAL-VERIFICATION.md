# Phase 28 - Default Config Files: Manual Verification

**Purpose:** The author's hands-on check of the default/live config split; DEF-01..05 stay "UAT pending" until this is run.

**Before you start:**
- Build: `CARGO_TARGET_DIR=C:/Users/jonat/AppData/Local/Temp/ff-target cargo build --release --bin ff --bin ff-cli`
- Make a temp app root holding `config.json`, `styles-default.json`, `tiers-default.json`, `prices.json` (no `styles.json` / `tiers.json`); point `FERRISFILTER_HOME` and `USERPROFILE` at temp folders.
- PoE2 in windowed mode (optional; only for the in-game reload).

1. **UAT5-01** - open `ff`, look at CONFIG Styles and Tiers - Expected: the default styles and tiers are shown; no `styles.json` / `tiers.json` appears in the app root - Result: assumed pass (not tested, accepted by the author 2026-10-15)
2. **UAT5-02** - edit a style colour in CONFIG and Apply - Expected: `styles.json` is created (`tiers.json` only if a tier was edited); `styles-default.json` / `tiers-default.json` are unchanged - Result: assumed pass (not tested, accepted by the author 2026-10-15)
3. **UAT5-03** - with `ff` running, delete the live `styles.json` - Expected: within a few seconds the stripe falls back to the default styles - Result: assumed pass (not tested, accepted by the author 2026-10-15)
4. **UAT5-04** - make a CONFIG edit, then create `styles.json` by hand in the app root, Apply - Expected: Apply is refused naming `styles.json`, with RELOAD and OVERWRITE offered - Result: assumed pass (not tested, accepted by the author 2026-10-15)
5. **UAT5-05** - `ff-cli generate` in the app root with and without the live files; then `--styles missing.json` - Expected: both runs succeed (live files used when present, defaults otherwise); the missing `--styles` path fails without falling back - Result: assumed pass (not tested, accepted by the author 2026-10-15)
6. **UAT5-06** - `ff-cli apply --filter <temp filter> --add-economy` with and without the live files - Expected: both succeed and read the live files when present, the defaults otherwise - Result: assumed pass (not tested, accepted by the author 2026-10-15)
7. **UAT5-07** - run `ff` from the repo (no `FERRISFILTER_HOME`), Apply a CONFIG edit, then `git status` (this writes your own live config) - Expected: no `styles.json` / `tiers.json` change is listed - Result: assumed pass (not tested, accepted by the author 2026-10-15)

Results recorded at the v0.5 wrap-up (2026-10-15): the author accepted UAT5-01..07 as assumed passes without running them. DEF-03..05 are Complete.
