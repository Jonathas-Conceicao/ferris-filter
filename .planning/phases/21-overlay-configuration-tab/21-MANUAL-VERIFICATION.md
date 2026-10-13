# Phase 21 — Overlay Configuration Tab: Manual Verification

**Purpose:** the checks headless tests cannot see (tab strip at real stripe width, colour picker,
preview chips, an outside edit in a real text editor, the game-folder dropdown, a relaunch, an
in-game reload). Requirements STORE-02, CONF-01, CONF-02, CONF-03 and CONF-04 stay "Complete (UAT
pending)" until this script has been run.

**Before you start (do not skip):**
- CONFIG Apply REWRITES `styles.json` and `tiers.json`. Never run it against the repo checkout
  (`artifact_guard` reads the committed copies). Make a scratch app root:
  copy `styles.json`, `tiers.json`, `config.json`, `prices.json` (or `prices-fallback.json` and
  `tiers-fallback.json`) into `C:\Temp\ff-home`, then
  `set FERRISFILTER_HOME=C:\Temp\ff-home`.
- Point the game folder at a copy too: `set USERPROFILE=C:\Temp\poe2-copy-home` with a few `*.filter`
  files (including a `dynamic.filter`) at `OneDrive\Documentos\My Games\Path of Exile 2\` inside it.
  Keep an untouched backup of each.
- Launch from the same shell: `cargo run --bin ff`. Path of Exile 2 in windowed or
  windowed-fullscreen mode (see `13-MANUAL-VERIFICATION.md`).
- Use a `dynamic.filter` with a few rules, at least two carrying `#ff:style kite`.

Record Result as PASS / FAIL with a note for every step.

---

1. **Tab strip.** Look under the header plate. Click CONFIG, then RULES; focus a tab and use the
   Left/Right arrow keys.
   **Expected:** two equal segments; the active one has a cream underline; switching changes the
   body only. **Result:**

2. **Cross-tab markers.** On RULES edit a rule (do not Apply), switch to CONFIG; then add a style on
   CONFIG, switch back.
   **Expected:** CONFIG shows `RULES *` while RULES has unapplied edits, and RULES shows `CONFIG *`
   while CONFIG does; hovering the marker says `Unapplied changes on this tab`; nothing is applied or
   discarded by switching. **Result:**

3. **Add a style.** STYLES > `+ ADD STYLE`; type `Mine` and watch the name hint; press add.
   **Expected:** names that clash or are invalid show a hint and cannot be added; the new style
   appears expanded with a `new` badge and is scrolled into view. **Result:**

4. **Edit every field.** In the new style change action Show/Hide, text/border/background colours
   (opaque picker, no alpha slider), font size, sound None/Alert/Custom, drop sound, minimap,
   beam.
   **Expected:** the preview chip follows each change; a Hide style shows the hidden chip; the row
   shows an `edited`/`new` badge. **Result:**

5. **Built-ins cannot be deleted.** Expand `S`, `unchanged` and `hidden`.
   **Expected:** no Delete control on any built-in. **Result:**

6. **Delete a style in use.** Expand `kite` (or another style rules use) and press Delete.
   **Expected:** an inline confirm shows `N rules use this style...` and, when `config.json` uses
   it, the line that `ferrisfilter generate` will fail until it is added back. **Result:**

7. **Tier validation.** In TIERS enter `abc` or a negative cutoff.
   **Expected:** the row shows an inline error, Apply is disabled and its tooltip says
   `Fix the tier errors first.`; fixing the value re-enables it. **Result:**

8. **Config Apply writes only two files.** Note the folder listing and modification times of
   `C:\Temp\ff-home` and the game folder, then press Apply on CONFIG.
   **Expected:** only `styles.json` and `tiers.json` change (compare the listings;
   `prices-fallback.json` and `tiers-fallback.json` and every game-folder file are untouched); the
   message reads `Applied config: N styles, 7 tiers.` followed by the out-of-date sentence.
   **Result:**

9. **Out-of-date line.** After step 8 look at both tabs.
   **Expected:** both show `Filter out of date: config changed. Press Apply on the RULES tab to
   regenerate it.`; the RULES style picker already lists the new style. **Result:**

10. **RULES Apply clears it, and deleted styles detach.** Having deleted `kite` in step 6 and applied,
    press APPLY on the RULES tab.
    **Expected:** the out-of-date line disappears from both tabs; rules that used the deleted style
    lose their `#ff:style` tag and keep their actions (open the file to confirm). **Result:**

11. **Revert.** Make two edits on CONFIG, press Revert.
    **Expected:** an inline confirm shows the edit count; Discard restores the on-disk state and
    says `Reverted config changes.`; Cancel keeps the edits. **Result:**

12. **Outside edit.** Open `tiers.json` in a text editor, change a cutoff, save. Switch to RULES and
    back to CONFIG (or press Apply with a pending edit).
    **Expected:** a warning names the file that changed and offers **Reload from disk**,
    **Overwrite** and **Keep editing**; Reload shows the editor's value, Overwrite replaces the file
    with the overlay's state, Keep editing dismisses the warning; nothing is overwritten
    silently. **Result:**

13. **Filter dropdown.** Open FILTER.
    **Expected:** it lists the `*.filter` files in the game-folder copy and marks the active one; RULES
    shows the active file's path. **Result:**

14. **Create a filter.** Choose `New empty filter...`, type `test`, add it.
    **Expected:** the name becomes `test.filter`; the RULES tab shows the empty `test.filter` with
    "does not exist yet"; the file appears in the folder only after the first RULES Apply; a name
    that already exists or contains a path is refused with a hint. **Result:**

15. **Switch guard.** On RULES edit a rule (do not Apply), then pick another filter on CONFIG.
    **Expected:** the dropdown keeps the current name and asks `Switch and discard` / `Stay`; Stay
    cancels, Switch and discard loads the other filter without writing anything. **Result:**

16. **Remembered filter.** Quit from the tray (Quit), relaunch `ff` with the same environment.
    **Expected:** the last chosen filter is loaded and selected; `ff-settings.json` exists in
    `C:\Temp\ff-home` with the filter name and is not tracked by git. **Result:**

17. **In game.** Apply on RULES, then in Path of Exile 2 use Options > Game > Item Filter >
    **Refresh** and select the filter you applied.
    **Expected:** items drop with the new styles. **Result:**

---

**After the run:** if every step passed, change the five requirement rows in
`.planning/REQUIREMENTS.md` from `Complete (UAT pending)` to `Complete`. Record any FAIL as a gap
for `/gsd-plan-phase 21 --gaps`.
