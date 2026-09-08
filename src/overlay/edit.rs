//! Pure mutation layer for the rule editor (D-18, roadmap success criterion 8).
//!
//! This module owns every mutation the editor performs: add, delete, move, enable/disable a
//! rule, and every per-condition set/clear function (EDIT-03..08). It must never contain any
//! `egui`/`eframe`/`winit`/`windows_sys` type — `PITFALLS.md` §8 states plainly that GUI code has
//! no test story, so any logic that can only be exercised by clicking belongs in the wrong layer.
//! Everything here is a plain function over `Rule`/`When`/`Config`, covered by `#[cfg(test)]`
//! with no window ever opened.
//!
//! Identity is a [`RuleId`], not an array index, because a reorder changes every index below the
//! moved rule (`PITFALLS.md` §9, D-19). A player who has a rule expanded in the accordion and
//! then clicks an up/down arrow must keep looking at the *same* rule afterward — an index-keyed
//! "currently expanded" state would silently point at whatever rule now occupies that slot.

use crate::models::config::{
    Config, NumericCondition, Rarity, RarityComparator, RarityOp, Rule, Styles, UnidentifiedTier,
    When,
};

// ---------------------------------------------------------------------------
// The RuleId working model (Task 1)
// ---------------------------------------------------------------------------

/// Session-only identity for a rule in the working model.
///
/// Never serialized — `dynamic.json`'s only identity is array position (unchanged from Phase
/// 12). The inner `u64` is private; nothing outside this module may construct, read, or forge a
/// `RuleId`, so the only way to get one is [`WorkingRules::from_rules`] or [`WorkingRules::add`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct RuleId(u64);

/// The editor's in-memory working model: stable identity paired with the rule data.
///
/// Array order here IS precedence order, exactly as `Vec<Rule>` always was — `RuleId` adds an
/// identity layer on top, it does not change the ordering semantics. [`WorkingRules::to_rules`]
/// strips that identity back to the plain `Vec<Rule>` `dynamic.json` expects.
#[derive(Debug, Clone)]
pub struct WorkingRules {
    next_id: u64,
    rows: Vec<(RuleId, Rule)>,
}

impl WorkingRules {
    /// Builds the working model from a freshly loaded `Vec<Rule>`, assigning each rule a fresh
    /// `RuleId` in array order.
    pub fn from_rules(rules: Vec<Rule>) -> Self {
        let mut next_id = 0u64;
        let rows = rules
            .into_iter()
            .map(|rule| {
                let id = RuleId(next_id);
                next_id += 1;
                (id, rule)
            })
            .collect();
        Self { next_id, rows }
    }

    /// Strips identity back to the plain `Vec<Rule>` `dynamic.json` expects, preserving order.
    pub fn to_rules(&self) -> Vec<Rule> {
        self.rows.iter().map(|(_, r)| r.clone()).collect()
    }

    /// The working rows in precedence order.
    pub fn rows(&self) -> &[(RuleId, Rule)] {
        &self.rows
    }

    /// The array index of `id`, or `None` if it is not present (e.g. it was just deleted).
    pub fn position(&self, id: RuleId) -> Option<usize> {
        self.rows.iter().position(|(rid, _)| *rid == id)
    }

    /// A read-only reference to the rule identified by `id`, or `None`.
    pub fn rule(&self, id: RuleId) -> Option<&Rule> {
        self.rows.iter().find(|(rid, _)| *rid == id).map(|(_, r)| r)
    }

    /// A mutable reference to the rule identified by `id`, or `None`.
    ///
    /// This is how every per-condition set/clear function in this module composes with the
    /// working model — callers do `edit::set_style(wr.rule_mut(id)?, "S")`, never touch `rows`
    /// directly.
    pub fn rule_mut(&mut self, id: RuleId) -> Option<&mut Rule> {
        self.rows.iter_mut().find(|(rid, _)| *rid == id).map(|(_, r)| r)
    }

    /// D-06/D-07: appends a new rule at the end — `{"style": "unchanged", "when": {}}`, enabled.
    /// Safe by construction: nothing an incomplete rule does can reach the game until the player
    /// deliberately Applies *and* reloads in-game.
    pub fn add(&mut self) -> RuleId {
        let id = RuleId(self.next_id);
        self.next_id += 1;
        self.rows.push((
            id,
            Rule {
                enabled: None,
                name: None,
                style: "unchanged".to_string(),
                when: Some(When::default()),
            },
        ));
        id
    }

    /// Removes the targeted rule. Returns `true` if a rule with `id` was found and removed;
    /// every surviving rule keeps its own id — nothing is renumbered.
    pub fn delete(&mut self, id: RuleId) -> bool {
        let before = self.rows.len();
        self.rows.retain(|(rid, _)| *rid != id);
        self.rows.len() != before
    }

    /// Swaps `id` one position earlier. No-op (returns `false`) if `id` is not found or already
    /// first — callers use the return value to decide whether to disable the "move up" button,
    /// not to guess from index 0.
    pub fn move_up(&mut self, id: RuleId) -> bool {
        let Some(pos) = self.position(id) else {
            return false;
        };
        if pos == 0 {
            return false;
        }
        self.rows.swap(pos, pos - 1);
        true
    }

    /// Swaps `id` one position later. No-op (returns `false`) if `id` is not found or already
    /// last.
    pub fn move_down(&mut self, id: RuleId) -> bool {
        let Some(pos) = self.position(id) else {
            return false;
        };
        if pos + 1 >= self.rows.len() {
            return false;
        }
        self.rows.swap(pos, pos + 1);
        true
    }

    /// EDIT-02: toggles `Rule.enabled`. `None`/`Some(true)` -> `Some(false)` -> `None`, so
    /// re-enabling never writes a redundant `"enabled": true` key (RESEARCH Pitfall 5) — both
    /// `None` and `Some(true)` mean enabled, and the toggle always returns to the minimal `None`
    /// form rather than treating them as distinct states.
    ///
    /// A stale/unknown `id` is silently a no-op — this mirrors every other lookup in this module
    /// (T-14-02: no panics on a bad id).
    pub fn toggle_enabled(&mut self, id: RuleId) {
        if let Some(rule) = self.rule_mut(id) {
            rule.enabled = match rule.enabled {
                Some(false) => None,
                _ => Some(false),
            };
        }
    }

    /// `true` unless the rule is `enabled: Some(false)` — absent or `Some(true)` both mean
    /// enabled, mirroring the emitter's own reading of the field (`emit_rules`'s skip check). A
    /// stale/unknown `id` reads as `false` rather than panicking.
    pub fn is_enabled(&self, id: RuleId) -> bool {
        self.rule(id).is_some_and(|r| r.enabled != Some(false))
    }
}

// ---------------------------------------------------------------------------
// Per-condition set/clear functions and config-sourced enumerations (Task 2)
// ---------------------------------------------------------------------------

/// Returns a mutable reference to `rule.when`, creating an empty (matches-everything) `When` if
/// absent. Every condition setter below goes through this so a rule that starts with `when: None`
/// (a catch-all) gains a `when` only when the player actually sets a condition on it.
pub fn when_mut(rule: &mut Rule) -> &mut When {
    rule.when.get_or_insert_with(When::default)
}

/// EDIT-03: sets the rule's style. The caller is expected to pick `style` from
/// [`available_styles`] so an unresolvable name can never reach the store.
pub fn set_style(rule: &mut Rule, style: &str) {
    rule.style = style.to_string();
}

/// EDIT-03: the style names selectable from the picker, in the fixed display order the game's
/// own tier system uses (S..F, then the named styles). Reads directly from the loaded `Styles`
/// struct — `exceptional`/`craftable` only appear if the config actually defines them, so this
/// can never offer a style name that would fail `emitter::resolve_style`'s generate-time check
/// (T-14-03).
pub fn available_styles(styles: &Styles) -> Vec<&'static str> {
    let mut names = vec![
        "S", "A", "B", "C", "D", "E", "F", "Recycle", "unchanged", "hidden", "kite",
    ];
    if styles.exceptional.is_some() {
        names.push("exceptional");
    }
    if styles.craftable.is_some() {
        names.push("craftable");
    }
    names
}

/// EDIT-04: the class groups the picker offers (D-10), borrowed from the config —
/// `PITFALLS.md` §10: class lists must never be hardcoded in the presentation layer.
///
/// Jewellery was added 2026-09-07 after live use: `config.json`'s rules already matched Rings,
/// Amulets and Belts, but no list existed to source them from, so they could not be selected in
/// the editor at all.
///
/// Empty groups are omitted rather than rendered as a bare header. A `dynamic.json` seeded
/// before `jewelleryClasses` existed parses with an empty list (the field is
/// `#[serde(default)]`), and an empty collapsible section would read as a bug.
pub fn class_groups(config: &Config) -> Vec<(&'static str, &[String])> {
    [
        ("WEAPONS", config.weapon_classes.as_slice()),
        ("ARMOUR", config.armour_classes.as_slice()),
        ("JEWELLERY", config.jewellery_classes.as_slice()),
        ("FLASKS & CHARMS", config.flask_charm_classes.as_slice()),
    ]
    .into_iter()
    .filter(|(_, classes)| !classes.is_empty())
    .collect()
}

/// EDIT-04: `true` if `class` is currently selected on `when`.
pub fn is_class_selected(when: &When, class: &str) -> bool {
    when.class
        .as_ref()
        .is_some_and(|classes| classes.iter().any(|c| c == class))
}

/// EDIT-04/EDIT-08: selects or deselects `class` on `when`.
///
/// Selection order is the player's click order — `add` appends, `retain` on deselect — never
/// sorted or re-derived from the config's own class order.
///
/// Deselecting the last remaining class collapses the field to `None` rather than `Some(vec![])`
/// (roadmap acceptance criterion): `src/emitter.rs`'s class-line emission (`emit_rules`, ~line
/// 362) writes a bare `Class "..." "..."` line straight from the vec with no emptiness guard, so
/// an empty-but-present vec would emit a malformed, empty `Class` line that fails to load
/// in-game — the exact defect class the invalid `Class "Tablet"` rows removed in v0.2 belonged
/// to. `None` is the only representation of "no class condition" the emitter handles correctly.
pub fn set_class(when: &mut When, class: &str, selected: bool) {
    if selected {
        let classes = when.class.get_or_insert_with(Vec::new);
        if !classes.iter().any(|c| c == class) {
            classes.push(class.to_string());
        }
    } else if let Some(classes) = when.class.as_mut() {
        classes.retain(|c| c != class);
        if classes.is_empty() {
            when.class = None;
        }
    }
}

/// EDIT-05: sets the rarity comparator. `RarityComparator`'s own `Serialize` impl already emits
/// the compact string form (`">=Magic"`, bare `"Rare"` for `Eq`) — this function only needs to
/// set the typed value.
pub fn set_rarity(when: &mut When, op: RarityOp, rarity: Rarity) {
    when.rarity = Some(RarityComparator { op, rarity });
}

/// EDIT-05/EDIT-08: clears the rarity condition entirely.
pub fn clear_rarity(when: &mut When) {
    when.rarity = None;
}

/// EDIT-06: one of the three defence attributes a rule's `when` can condition on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Defence {
    Armour,
    Evasion,
    EnergyShield,
}

/// EDIT-06: sets a single defence checkbox. `ticked` -> `min: Some(1)` (emits `> 0`); `!ticked`
/// -> `eq: Some(0)` (emits the literal `0`). Mirrors `emitter::emit_numeric`'s own semantics
/// exactly, so the picker cannot produce a combination the emitter interprets differently than
/// the picker intended.
///
/// This writes only its own attribute — a rule that arrives partially specified from
/// `dynamic_base.json` (e.g. rule 6's `baseEvasion: {"eq": 0}` alone) is never rewritten just by
/// being displayed; only [`enable_defence_group`] writes all three explicitly.
pub fn set_defence(when: &mut When, defence: Defence, ticked: bool) {
    let nc = Some(if ticked {
        NumericCondition { eq: None, min: Some(1), max: None }
    } else {
        NumericCondition { eq: Some(0), min: None, max: None }
    });
    match defence {
        Defence::Armour => when.base_armour = nc,
        Defence::Evasion => when.base_evasion = nc,
        Defence::EnergyShield => when.base_energy_shield = nc,
    }
}

/// EDIT-06: `true` only for `min == Some(1)` — i.e. the checkbox reads as ticked only for the
/// "strictly positive" encoding, not for an `eq: Some(1)` a hand-edited file could contain.
pub fn defence_ticked(when: &When, defence: Defence) -> bool {
    let nc = match defence {
        Defence::Armour => &when.base_armour,
        Defence::Evasion => &when.base_evasion,
        Defence::EnergyShield => &when.base_energy_shield,
    };
    nc.as_ref().is_some_and(|nc| nc.min == Some(1))
}

/// EDIT-06: `true` iff at least one of the three defence attributes is `Some` — so
/// `dynamic_base.json`'s evasion-only rule (rule 6, `baseEvasion: {"eq": 0}` alone) reads as an
/// enabled defence group without being rewritten on load.
pub fn defence_group_enabled(when: &When) -> bool {
    when.base_armour.is_some() || when.base_evasion.is_some() || when.base_energy_shield.is_some()
}

/// EDIT-06: turns the defence group on, writing all three attributes explicitly as
/// `eq: Some(0)`. This is what makes roadmap criterion 5's `BaseEvasion 0` line appear for an
/// attribute the player never ticked — ticking armour and energy shield and leaving evasion
/// unticked, starting from a rule with no defence condition at all, must still emit
/// `BaseEvasion 0` once the group itself is on.
pub fn enable_defence_group(when: &mut When) {
    let off = || Some(NumericCondition { eq: Some(0), min: None, max: None });
    if when.base_armour.is_none() {
        when.base_armour = off();
    }
    if when.base_evasion.is_none() {
        when.base_evasion = off();
    }
    if when.base_energy_shield.is_none() {
        when.base_energy_shield = off();
    }
}

/// EDIT-06/EDIT-08: turns the defence group off — clears all three to `None`, so the compiled
/// block contains no `BaseArmour`/`BaseEvasion`/`BaseEnergyShield` line at all.
pub fn clear_defence_group(when: &mut When) {
    when.base_armour = None;
    when.base_evasion = None;
    when.base_energy_shield = None;
}

/// EDIT-07: sets the sockets minimum.
pub fn set_sockets_min(when: &mut When, min: i64) {
    when.sockets = Some(NumericCondition { eq: None, min: Some(min), max: None });
}

/// EDIT-07/EDIT-08: clears the sockets condition entirely.
pub fn clear_sockets(when: &mut When) {
    when.sockets = None;
}

/// EDIT-07: sets the quality minimum.
pub fn set_quality_min(when: &mut When, min: i64) {
    when.quality = Some(NumericCondition { eq: None, min: Some(min), max: None });
}

/// EDIT-07/EDIT-08: clears the quality condition entirely.
pub fn clear_quality(when: &mut When) {
    when.quality = None;
}

/// The bounds of PoE2's unidentified-tier scale. It is a small enumerable range, not an
/// open-ended count like sockets or quality, which is why the editor offers it as a five-position
/// slider rather than a drag-value — every valid tier is visible at once and an out-of-range
/// value cannot be entered.
///
/// Kept here rather than in the presentation layer so the bound is stated once, beside the
/// setter it constrains (`PITFALLS.md` §10's principle applied to a numeric range).
pub const UNIDENTIFIED_TIER_MIN: u32 = 1;
pub const UNIDENTIFIED_TIER_MAX: u32 = 5;
/// What enabling the condition starts at — the threshold `dynamic_base.json`'s rules 7 and 8
/// already use.
pub const UNIDENTIFIED_TIER_DEFAULT: u32 = 4;

/// Sets the unidentified-item-tier minimum, emitting `UnidentifiedItemTier >= min`.
///
/// Added 2026-09-07 after live use: `dynamic_base.json`'s rules 7 and 8 both carry this
/// condition and the collapsed summary displayed it, but no control existed to change it.
/// EDIT-07 enumerated only sockets, quality and corrupted, so the gap was inherited from the
/// requirement rather than introduced in execution.
///
/// `min` is `u32` in the model (unlike sockets/quality, which are `i64`) — a negative tier is
/// meaningless, so the caller must clamp rather than cast.
pub fn set_unidentified_tier_min(when: &mut When, min: u32) {
    when.unidentified_tier = Some(UnidentifiedTier { min });
}

/// EDIT-08: clears the unidentified-tier condition entirely, emitting nothing.
pub fn clear_unidentified_tier(when: &mut When) {
    when.unidentified_tier = None;
}

/// EDIT-07/EDIT-08: sets (or, via `None`, clears) the corrupted condition. `Some(false)` emits
/// `Corrupted False`; `Some(true)` emits `Corrupted True`; `None` omits the condition entirely.
/// `corrupted` is not a bespoke tri-state widget — it uses the same enable-toggle + value
/// architecture as every other condition (RESEARCH Pattern 2), so its "unset" state is exactly
/// the same mechanism (`Option::None`) as every other condition's "off" state.
pub fn set_corrupted(when: &mut When, value: Option<bool>) {
    when.corrupted = value;
}

// ---------------------------------------------------------------------------
// Base-type list and rule name (Phase 15, EDIT-10/EDIT-11)
// ---------------------------------------------------------------------------

/// EDIT-10/D-14: parses one commit of the base-type add field.
///
/// A bare typed name (no `"` anywhere in the input) yields exactly one entry, trimmed. Input
/// containing a double quote is treated as the `BaseType == "A" "B"` form the author copies out
/// of hand-written filters: every quoted span becomes its own entry, in the order it appears,
/// with any text between spans (whitespace, or the `BaseType ==` prefix itself if pasted whole)
/// ignored, and an empty span dropped. Blank or whitespace-only input yields an empty vec.
///
/// This is the whole of it — trimming and quote-stripping is string hygiene on input, not the
/// validation D-05 forbids. There is no list of legitimate base names to check against here or
/// anywhere else in this module: a typo is stored exactly as typed, and PoE2 itself is the only
/// oracle that warns about it, at filter load time.
pub fn parse_base_type_entry(raw: &str) -> Vec<String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Vec::new();
    }
    if trimmed.contains('"') {
        let mut out = Vec::new();
        let mut chars = trimmed.char_indices().peekable();
        while let Some((start, c)) = chars.next() {
            if c == '"' {
                if let Some(end) = trimmed[start + 1..].find('"') {
                    let inner = trimmed[start + 1..start + 1 + end].trim();
                    if !inner.is_empty() {
                        out.push(inner.to_string());
                    }
                    // Skip the iterator past the closing quote so it isn't reconsidered as the
                    // opening quote of the next span.
                    while let Some((i, _)) = chars.peek() {
                        if *i > start + 1 + end {
                            break;
                        }
                        chars.next();
                    }
                }
            }
        }
        out
    } else {
        vec![trimmed.to_string()]
    }
}

/// EDIT-10: appends `name` to `when.base_type`, creating the list if absent, skipping an
/// empty-after-trim name and skipping an entry already present verbatim (the same dedup guard
/// [`set_class`] uses for classes). Accepts whatever text is given, unmodified beyond trimming —
/// D-05's accept-what-is-typed principle applies here exactly as it does to [`parse_base_type_entry`].
pub fn add_base_type(when: &mut When, name: &str) {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        return;
    }
    let list = when.base_type.get_or_insert_with(Vec::new);
    if !list.iter().any(|existing| existing == trimmed) {
        list.push(trimmed.to_string());
    }
}

/// EDIT-10/D-02: removes the entry at `index`, from any position in the list. A no-op if `index`
/// is out of range.
///
/// Collapses `when.base_type` to `None` when the removal empties the list, mirroring
/// [`set_class`]'s own empty-vec guard: `src/emitter.rs` writes the `BaseType ==` line straight
/// from the vec with no emptiness check, so `Some(vec![])` would emit a malformed, empty
/// `BaseType ==` line that fails to load in-game — the same defect class as the invalid
/// `Class "Tablet"` rows removed in v0.2.
pub fn remove_base_type(when: &mut When, index: usize) {
    if let Some(list) = when.base_type.as_mut() {
        if index < list.len() {
            list.remove(index);
        }
        if list.is_empty() {
            when.base_type = None;
        }
    }
}

/// EDIT-10/EDIT-08: clears the base-type condition entirely — the base-type condition's own off
/// switch (used by plan 15-02).
pub fn clear_base_type(when: &mut When) {
    when.base_type = None;
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rule(style: &str) -> Rule {
        Rule { enabled: None, name: None, style: style.to_string(), when: None }
    }

    /// A `RuleId` guaranteed not to resolve against `wr`: added then immediately deleted, so the
    /// id is real (not merely a same-numbered id from an unrelated `WorkingRules`, which — since
    /// every `WorkingRules` starts its own counter at 0 — would collide with a genuinely present
    /// id and make these boundary tests pass for the wrong reason).
    fn unknown_id(wr: &mut WorkingRules) -> RuleId {
        let id = wr.add();
        wr.delete(id);
        id
    }

    fn base_config() -> Config {
        let json =
            include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/dynamic_base.json"));
        serde_json::from_str(json).expect("dynamic_base.json must parse as a Config")
    }

    // -- WorkingRules: from_rules / to_rules --------------------------------

    #[test]
    fn from_rules_assigns_three_distinct_ids_in_array_order() {
        let wr = WorkingRules::from_rules(vec![rule("A"), rule("B"), rule("C")]);
        let ids: Vec<RuleId> = wr.rows().iter().map(|(id, _)| *id).collect();
        assert_eq!(ids.len(), 3);
        assert_ne!(ids[0], ids[1]);
        assert_ne!(ids[1], ids[2]);
        assert_ne!(ids[0], ids[2]);
        assert_eq!(wr.rows()[0].1.style, "A");
        assert_eq!(wr.rows()[1].1.style, "B");
        assert_eq!(wr.rows()[2].1.style, "C");
    }

    #[test]
    fn to_rules_strips_identity_and_preserves_order() {
        let wr = WorkingRules::from_rules(vec![rule("A"), rule("B")]);
        let plain = wr.to_rules();
        assert_eq!(plain, vec![rule("A"), rule("B")]);
    }

    // -- move_up / move_down and D-19 selection survival --------------------

    #[test]
    fn selection_survives_a_move_up_because_it_is_keyed_by_id_not_index() {
        let mut wr = WorkingRules::from_rules(vec![rule("A"), rule("B"), rule("C")]);
        let b_id = wr.rows()[1].0;

        assert!(wr.move_up(b_id));
        // B is now at index 0 -- the caller's "currently expanded" state, if it stored
        // `b_id` (not an index), still resolves to the same rule after the move.
        assert_eq!(wr.position(b_id), Some(0));
        assert_eq!(wr.rule(b_id).unwrap().style, "B");
    }

    #[test]
    fn selection_survives_a_different_rule_moving_past_it() {
        let mut wr = WorkingRules::from_rules(vec![rule("A"), rule("B"), rule("C")]);
        let a_id = wr.rows()[0].0;
        // Move C up past B and A entirely, capturing a_id first.
        let c_id = wr.rows()[2].0;
        assert!(wr.move_up(c_id));
        assert!(wr.move_up(c_id));
        assert_eq!(wr.position(c_id), Some(0));
        // a_id still resolves to the rule styled "A", now shifted one slot later by C
        // passing over it (order is now C, A, B).
        assert_eq!(wr.rule(a_id).unwrap().style, "A");
        assert_eq!(wr.position(a_id), Some(1));
    }

    #[test]
    fn move_up_on_the_first_rule_is_a_no_op() {
        let mut wr = WorkingRules::from_rules(vec![rule("A"), rule("B")]);
        let a_id = wr.rows()[0].0;
        assert!(!wr.move_up(a_id));
        assert_eq!(wr.rows()[0].1.style, "A");
    }

    #[test]
    fn move_down_on_the_last_rule_is_a_no_op() {
        let mut wr = WorkingRules::from_rules(vec![rule("A"), rule("B")]);
        let b_id = wr.rows()[1].0;
        assert!(!wr.move_down(b_id));
        assert_eq!(wr.rows()[1].1.style, "B");
    }

    #[test]
    fn move_up_and_move_down_on_an_unknown_id_return_false() {
        let mut wr = WorkingRules::from_rules(vec![rule("A"), rule("B")]);
        let stray = unknown_id(&mut wr);
        assert!(!wr.move_up(stray));
        assert!(!wr.move_down(stray));
    }

    // -- add / delete ---------------------------------------------------------

    #[test]
    fn add_appends_the_unchanged_noop_rule_at_the_end_enabled() {
        let mut wr = WorkingRules::from_rules(vec![rule("A")]);
        let new_id = wr.add();
        assert_eq!(wr.rows().len(), 2);
        let (id, added) = &wr.rows()[1];
        assert_eq!(*id, new_id);
        assert_eq!(added.style, "unchanged");
        assert_eq!(added.enabled, None, "D-07: new rules are enabled");
        assert_eq!(added.when, Some(When::default()));
    }

    #[test]
    fn delete_removes_only_the_targeted_rule_others_keep_their_id() {
        let mut wr = WorkingRules::from_rules(vec![rule("A"), rule("B"), rule("C")]);
        let b_id = wr.rows()[1].0;
        let c_id = wr.rows()[2].0;
        assert!(wr.delete(b_id));
        assert_eq!(wr.rows().len(), 2);
        assert_eq!(wr.position(c_id), Some(1), "C keeps its own id after B is removed");
        assert!(wr.rule(b_id).is_none());
    }

    #[test]
    fn delete_on_an_unknown_id_returns_false_and_changes_nothing() {
        let mut wr = WorkingRules::from_rules(vec![rule("A")]);
        let stray = unknown_id(&mut wr);
        assert!(!wr.delete(stray));
        assert_eq!(wr.rows().len(), 1);
    }

    // -- toggle_enabled / is_enabled -----------------------------------------

    #[test]
    fn toggle_enabled_cycles_none_to_false_to_none() {
        let mut wr = WorkingRules::from_rules(vec![rule("A")]);
        let id = wr.rows()[0].0;
        assert!(wr.is_enabled(id));
        assert_eq!(wr.rule(id).unwrap().enabled, None);

        wr.toggle_enabled(id);
        assert!(!wr.is_enabled(id));
        assert_eq!(wr.rule(id).unwrap().enabled, Some(false));

        wr.toggle_enabled(id);
        assert!(wr.is_enabled(id));
        assert_eq!(
            wr.rule(id).unwrap().enabled,
            None,
            "re-enabling must collapse back to None, never write a redundant Some(true)"
        );
    }

    #[test]
    fn toggle_enabled_on_a_rule_starting_explicitly_true_collapses_to_none() {
        let mut wr = WorkingRules::from_rules(vec![Rule {
            enabled: Some(true),
            name: None,
            style: "A".to_string(),
            when: None,
        }]);
        let id = wr.rows()[0].0;
        wr.toggle_enabled(id);
        assert_eq!(wr.rule(id).unwrap().enabled, Some(false));
        wr.toggle_enabled(id);
        assert_eq!(wr.rule(id).unwrap().enabled, None);
    }

    #[test]
    fn toggle_enabled_on_an_unknown_id_does_not_panic() {
        let mut wr = WorkingRules::from_rules(vec![rule("A")]);
        let stray = unknown_id(&mut wr);
        wr.toggle_enabled(stray); // must not panic
        assert!(wr.is_enabled(wr.rows()[0].0));
    }

    #[test]
    fn is_enabled_on_an_unknown_id_reads_false_not_a_panic() {
        let mut wr = WorkingRules::from_rules(vec![rule("A")]);
        let stray = unknown_id(&mut wr);
        assert!(!wr.is_enabled(stray));
    }

    // -- rule_mut composability ------------------------------------------------

    #[test]
    fn rule_mut_allows_composing_with_condition_setters() {
        let mut wr = WorkingRules::from_rules(vec![rule("unchanged")]);
        let id = wr.rows()[0].0;
        let rule = wr.rule_mut(id).expect("rule must exist");
        set_style(rule, "S");
        assert_eq!(wr.rule(id).unwrap().style, "S");
    }

    // -- available_styles (EDIT-03) -------------------------------------------

    #[test]
    fn dynamic_base_json_offers_exactly_twelve_styles() {
        let config = base_config();
        let names = available_styles(&config.styles);
        assert_eq!(names.len(), 12, "dynamic_base.json defines exceptional but not craftable");
        assert!(names.contains(&"exceptional"));
        assert!(!names.contains(&"craftable"));
    }

    #[test]
    fn a_styles_struct_with_neither_optional_style_offers_eleven() {
        let mut config = base_config();
        config.styles.exceptional = None;
        config.styles.craftable = None;
        let names = available_styles(&config.styles);
        assert_eq!(names.len(), 11);
        assert!(!names.contains(&"exceptional"));
        assert!(!names.contains(&"craftable"));
    }

    // -- class_groups (EDIT-04, PITFALLS.md §10) -------------------------------

    #[test]
    fn class_groups_are_borrowed_from_the_config_in_order() {
        let config = base_config();
        let groups = class_groups(&config);
        assert_eq!(groups.len(), 4);
        assert_eq!(groups[0], ("WEAPONS", config.weapon_classes.as_slice()));
        assert_eq!(groups[1], ("ARMOUR", config.armour_classes.as_slice()));
        assert_eq!(groups[2], ("JEWELLERY", config.jewellery_classes.as_slice()));
        assert_eq!(groups[3], ("FLASKS & CHARMS", config.flask_charm_classes.as_slice()));
        assert_eq!(groups[0].1.len(), 12);
        assert_eq!(groups[1].1.len(), 6);
        assert_eq!(groups[3].1.len(), 3);
    }

    #[test]
    fn jewellery_is_offered_and_comes_from_the_config_not_a_literal() {
        // Regression: `config.json`'s own rules matched Rings/Amulets/Belts, but no list
        // existed to source them from, so the picker could not offer them at all
        // (reported from live use 2026-09-07).
        let config = base_config();
        let groups = class_groups(&config);
        let jewellery = groups
            .iter()
            .find(|(name, _)| *name == "JEWELLERY")
            .expect("the picker must offer a jewellery group");
        assert_eq!(
            jewellery.1,
            ["Rings", "Amulets", "Belts"],
            "jewellery classes must come from config.jewelleryClasses"
        );
    }

    #[test]
    fn an_empty_group_is_omitted_rather_than_rendered_as_a_bare_header() {
        // A `dynamic.json` seeded before `jewelleryClasses` existed parses with an empty list
        // (the field is `#[serde(default)]`). An empty collapsible section would read as a bug.
        let mut config = base_config();
        config.jewellery_classes = Vec::new();
        let groups = class_groups(&config);
        assert_eq!(groups.len(), 3);
        assert!(
            !groups.iter().any(|(name, _)| *name == "JEWELLERY"),
            "an empty group must not be offered"
        );
    }

    // -- set_class / is_class_selected (EDIT-04, empty-vec guard) --------------

    #[test]
    fn set_class_on_an_empty_selection_yields_some_with_one_entry() {
        let mut when = When::default();
        set_class(&mut when, "Boots", true);
        assert_eq!(when.class, Some(vec!["Boots".to_string()]));
        assert!(is_class_selected(&when, "Boots"));
    }

    #[test]
    fn deselecting_the_last_remaining_class_yields_none_never_an_empty_vec() {
        let mut when = When::default();
        set_class(&mut when, "Boots", true);
        set_class(&mut when, "Boots", false);
        assert_eq!(when.class, None, "must collapse to None, never Some(vec![])");
        assert!(!is_class_selected(&when, "Boots"));
    }

    #[test]
    fn deselecting_one_of_several_classes_leaves_the_others_selected_in_click_order() {
        let mut when = When::default();
        set_class(&mut when, "Boots", true);
        set_class(&mut when, "Helmets", true);
        set_class(&mut when, "Gloves", true);
        set_class(&mut when, "Helmets", false);
        assert_eq!(
            when.class,
            Some(vec!["Boots".to_string(), "Gloves".to_string()]),
            "selection order is click order, not config order"
        );
    }

    #[test]
    fn selecting_an_already_selected_class_does_not_duplicate_it() {
        let mut when = When::default();
        set_class(&mut when, "Boots", true);
        set_class(&mut when, "Boots", true);
        assert_eq!(when.class, Some(vec!["Boots".to_string()]));
    }

    // -- rarity (EDIT-05) ---------------------------------------------------------

    #[test]
    fn set_rarity_gte_emits_the_compact_gte_string() {
        let mut when = When::default();
        set_rarity(&mut when, RarityOp::Gte, Rarity::Magic);
        let rule = Rule { enabled: None, name: None, style: "C".to_string(), when: Some(when) };
        let json = serde_json::to_string(&rule).unwrap();
        assert!(json.contains("\">=Magic\""), "got: {json}");
    }

    #[test]
    fn set_rarity_eq_emits_the_bare_form() {
        let mut when = When::default();
        set_rarity(&mut when, RarityOp::Eq, Rarity::Rare);
        let rule = Rule { enabled: None, name: None, style: "C".to_string(), when: Some(when) };
        let json = serde_json::to_string(&rule).unwrap();
        assert!(json.contains("\"Rare\""), "got: {json}");
        assert!(!json.contains("==Rare"), "Eq must serialize bare, not with ==, got: {json}");
    }

    #[test]
    fn clear_rarity_yields_none() {
        let mut when = When::default();
        set_rarity(&mut when, RarityOp::Eq, Rarity::Rare);
        clear_rarity(&mut when);
        assert_eq!(when.rarity, None);
    }

    // -- defence picker (EDIT-06, roadmap criterion 5) -----------------------------

    #[test]
    fn ticking_armour_and_energy_shield_leaving_evasion_off_matches_criterion_5_struct_state() {
        let mut when = When::default();
        set_defence(&mut when, Defence::Armour, true);
        set_defence(&mut when, Defence::EnergyShield, true);
        set_defence(&mut when, Defence::Evasion, false);

        assert_eq!(when.base_armour.as_ref().unwrap().min, Some(1));
        assert_eq!(when.base_energy_shield.as_ref().unwrap().min, Some(1));
        assert_eq!(when.base_evasion.as_ref().unwrap().eq, Some(0));
    }

    #[test]
    fn criterion_5_defence_combination_emits_exact_filter_lines() {
        let mut when = When::default();
        set_defence(&mut when, Defence::Armour, true);
        set_defence(&mut when, Defence::EnergyShield, true);
        set_defence(&mut when, Defence::Evasion, false);

        let mut config = base_config();
        config.rules = Some(vec![Rule {
            enabled: None,
            name: None,
            style: "unchanged".to_string(),
            when: Some(when),
        }]);

        let filter = crate::emitter::generate(&config, &crate::pricer::PricedData::empty()).expect("generate");
        assert!(filter.contains("BaseArmour > 0"), "got:\n{filter}");
        assert!(filter.contains("BaseEnergyShield > 0"), "got:\n{filter}");
        assert!(filter.contains("BaseEvasion 0"), "got:\n{filter}");
    }

    #[test]
    fn turning_the_defence_group_off_emits_no_defence_line_at_all() {
        let mut when = When::default();
        enable_defence_group(&mut when);
        assert!(defence_group_enabled(&when));
        clear_defence_group(&mut when);
        assert!(!defence_group_enabled(&when));

        let mut config = base_config();
        config.rules = Some(vec![Rule {
            enabled: None,
            name: None,
            style: "unchanged".to_string(),
            when: Some(when),
        }]);
        let filter = crate::emitter::generate(&config, &crate::pricer::PricedData::empty()).expect("generate");
        assert!(!filter.contains("BaseArmour"), "got:\n{filter}");
        assert!(!filter.contains("BaseEvasion"), "got:\n{filter}");
        assert!(!filter.contains("BaseEnergyShield"), "got:\n{filter}");
    }

    #[test]
    fn enable_defence_group_writes_all_three_attributes_explicitly() {
        let mut when = When::default();
        enable_defence_group(&mut when);
        assert_eq!(when.base_armour.as_ref().unwrap().eq, Some(0));
        assert_eq!(when.base_evasion.as_ref().unwrap().eq, Some(0));
        assert_eq!(when.base_energy_shield.as_ref().unwrap().eq, Some(0));
    }

    #[test]
    fn defence_group_enabled_reads_true_for_a_partially_specified_rule_without_rewriting_it() {
        // Mirrors dynamic_base.json rule 6: baseEvasion: {"eq": 0} alone.
        let when = When {
            base_evasion: Some(NumericCondition { eq: Some(0), min: None, max: None }),
            ..Default::default()
        };
        assert!(defence_group_enabled(&when));
        assert!(when.base_armour.is_none(), "must not be rewritten just by being displayed");
        assert!(when.base_energy_shield.is_none());
    }

    #[test]
    fn defence_ticked_is_true_only_for_the_min_one_encoding() {
        let mut when = When::default();
        set_defence(&mut when, Defence::Armour, true);
        assert!(defence_ticked(&when, Defence::Armour));
        set_defence(&mut when, Defence::Armour, false);
        assert!(!defence_ticked(&when, Defence::Armour));
    }

    // -- unidentified tier (gap found in live use 2026-09-07) ----------------------

    #[test]
    fn the_unidentified_tier_scale_matches_the_games_and_the_default_sits_inside_it() {
        // PoE2's unidentified tier runs 1..=5. The editor offers a five-position slider, so a
        // widened range here would silently let the player pick a tier the game never assigns —
        // a rule that matches nothing, with no error to explain why.
        assert_eq!(UNIDENTIFIED_TIER_MIN, 1);
        assert_eq!(UNIDENTIFIED_TIER_MAX, 5);
        assert!(
            (UNIDENTIFIED_TIER_MIN..=UNIDENTIFIED_TIER_MAX).contains(&UNIDENTIFIED_TIER_DEFAULT),
            "enabling the condition must start at a tier the slider can actually show"
        );
    }

    #[test]
    fn set_unidentified_tier_min_emits_the_threshold_line() {
        let mut when = When::default();
        set_unidentified_tier_min(&mut when, 4);

        let mut config = base_config();
        config.rules = Some(vec![Rule {
            enabled: None,
            name: None,
            style: "kite".to_string(),
            when: Some(when),
        }]);

        let filter =
            crate::emitter::generate(&config, &crate::pricer::PricedData::empty()).expect("generate");
        assert!(
            filter.contains("UnidentifiedItemTier >= 4"),
            "got:\n{filter}"
        );
    }

    #[test]
    fn clearing_the_unidentified_tier_emits_no_line_at_all() {
        let mut when = When::default();
        set_unidentified_tier_min(&mut when, 4);
        clear_unidentified_tier(&mut when);
        assert!(when.unidentified_tier.is_none());

        let mut config = base_config();
        config.rules = Some(vec![Rule {
            enabled: None,
            name: None,
            style: "kite".to_string(),
            when: Some(when),
        }]);

        let filter =
            crate::emitter::generate(&config, &crate::pricer::PricedData::empty()).expect("generate");
        assert!(
            !filter.contains("UnidentifiedItemTier"),
            "a disabled control must contribute nothing (EDIT-08); got:\n{filter}"
        );
    }

    #[test]
    fn the_base_rules_that_already_use_unidentified_tier_are_now_reachable() {
        // Regression: rules 7 and 8 of dynamic_base.json carry this condition and the collapsed
        // summary displayed it, but no control could change it. If this list ever empties, the
        // editor control has lost the rules it exists to serve.
        let config = base_config();
        let with_tier = config
            .rules
            .as_deref()
            .unwrap_or_default()
            .iter()
            .filter(|r| {
                r.when
                    .as_ref()
                    .is_some_and(|w| w.unidentified_tier.is_some())
            })
            .count();
        assert!(
            with_tier >= 2,
            "expected the base rule set to still use unidentifiedTier, found {with_tier}"
        );
    }

    // -- sockets / quality / corrupted (EDIT-07/EDIT-08) ---------------------------

    #[test]
    fn set_sockets_min_yields_the_expected_condition_and_emits_gte() {
        let mut when = When::default();
        set_sockets_min(&mut when, 3);
        assert_eq!(
            when.sockets,
            Some(NumericCondition { eq: None, min: Some(3), max: None })
        );

        let mut config = base_config();
        config.rules = Some(vec![Rule {
            enabled: None,
            name: None,
            style: "unchanged".to_string(),
            when: Some(when),
        }]);
        let filter = crate::emitter::generate(&config, &crate::pricer::PricedData::empty()).expect("generate");
        assert!(filter.contains("Sockets >= 3"), "got:\n{filter}");
    }

    #[test]
    fn clear_sockets_yields_none_and_emits_no_sockets_line() {
        let mut when = When::default();
        set_sockets_min(&mut when, 3);
        clear_sockets(&mut when);
        assert_eq!(when.sockets, None);

        let mut config = base_config();
        config.rules = Some(vec![Rule {
            enabled: None,
            name: None,
            style: "unchanged".to_string(),
            when: Some(when),
        }]);
        let filter = crate::emitter::generate(&config, &crate::pricer::PricedData::empty()).expect("generate");
        assert!(!filter.contains("Sockets"), "got:\n{filter}");
    }

    #[test]
    fn set_and_clear_quality_min_mirrors_sockets() {
        let mut when = When::default();
        set_quality_min(&mut when, 5);
        assert_eq!(
            when.quality,
            Some(NumericCondition { eq: None, min: Some(5), max: None })
        );
        clear_quality(&mut when);
        assert_eq!(when.quality, None);
    }

    #[test]
    fn set_corrupted_false_emits_corrupted_false() {
        let mut when = When::default();
        set_corrupted(&mut when, Some(false));

        let mut config = base_config();
        config.rules = Some(vec![Rule {
            enabled: None,
            name: None,
            style: "unchanged".to_string(),
            when: Some(when),
        }]);
        let filter = crate::emitter::generate(&config, &crate::pricer::PricedData::empty()).expect("generate");
        assert!(filter.contains("Corrupted False"), "got:\n{filter}");
    }

    #[test]
    fn set_corrupted_none_emits_no_corrupted_line() {
        let mut when = When::default();
        set_corrupted(&mut when, None);

        let mut config = base_config();
        config.rules = Some(vec![Rule {
            enabled: None,
            name: None,
            style: "unchanged".to_string(),
            when: Some(when),
        }]);
        let filter = crate::emitter::generate(&config, &crate::pricer::PricedData::empty()).expect("generate");
        assert!(!filter.contains("Corrupted"), "got:\n{filter}");
    }

    // -- base types (EDIT-10) ------------------------------------------------------

    #[test]
    fn parse_base_type_entry_splits_a_pasted_quoted_list() {
        let parsed = parse_base_type_entry(
            "\"Heavy Belt\" \"Utility Belt\" \"Armoured Cap\" \"Sacred Focus\"",
        );
        assert_eq!(
            parsed,
            vec![
                "Heavy Belt".to_string(),
                "Utility Belt".to_string(),
                "Armoured Cap".to_string(),
                "Sacred Focus".to_string(),
            ]
        );
    }

    #[test]
    fn parse_base_type_entry_strips_surrounding_quotes_from_a_single_name() {
        assert_eq!(
            parse_base_type_entry("\"Heavy Belt\""),
            vec!["Heavy Belt".to_string()]
        );
    }

    #[test]
    fn parse_base_type_entry_on_blank_input_yields_nothing() {
        assert_eq!(parse_base_type_entry(""), Vec::<String>::new());
        assert_eq!(parse_base_type_entry("   "), Vec::<String>::new());
    }

    #[test]
    fn parse_base_type_entry_keeps_an_unknown_name_verbatim() {
        // D-05: a typo is stored, not rejected -- there is no base-name list in this module or
        // anywhere else in the codebase to check against.
        assert_eq!(
            parse_base_type_entry("Sacerd Focus"),
            vec!["Sacerd Focus".to_string()]
        );
    }

    #[test]
    fn add_base_type_ignores_an_exact_duplicate() {
        let mut when = When::default();
        add_base_type(&mut when, "Heavy Belt");
        add_base_type(&mut when, "Heavy Belt");
        assert_eq!(when.base_type, Some(vec!["Heavy Belt".to_string()]));
    }

    #[test]
    fn removing_the_last_base_type_yields_none_never_an_empty_vec() {
        let mut when = When::default();
        add_base_type(&mut when, "Heavy Belt");
        remove_base_type(&mut when, 0);
        assert_eq!(
            when.base_type, None,
            "must collapse to None, never Some(vec![])"
        );
    }

    #[test]
    fn removing_from_the_middle_keeps_the_others_in_order() {
        let mut when = When::default();
        add_base_type(&mut when, "Heavy Belt");
        add_base_type(&mut when, "Utility Belt");
        add_base_type(&mut when, "Armoured Cap");
        remove_base_type(&mut when, 1);
        assert_eq!(
            when.base_type,
            Some(vec!["Heavy Belt".to_string(), "Armoured Cap".to_string()])
        );
    }

    #[test]
    fn remove_base_type_out_of_range_is_a_no_op() {
        let mut when = When::default();
        add_base_type(&mut when, "Heavy Belt");
        remove_base_type(&mut when, 5);
        assert_eq!(when.base_type, Some(vec!["Heavy Belt".to_string()]));
    }

    #[test]
    fn clear_base_type_yields_none() {
        let mut when = When::default();
        add_base_type(&mut when, "Heavy Belt");
        clear_base_type(&mut when);
        assert_eq!(when.base_type, None);
    }

    #[test]
    fn an_emptied_base_type_list_emits_no_base_type_line() {
        let mut when = When::default();
        add_base_type(&mut when, "Heavy Belt");
        remove_base_type(&mut when, 0);

        let mut config = base_config();
        config.rules = Some(vec![Rule {
            enabled: None,
            name: None,
            style: "unchanged".to_string(),
            when: Some(when),
        }]);
        let filter = crate::emitter::generate(&config, &crate::pricer::PricedData::empty())
            .expect("generate");
        assert!(!filter.contains("BaseType"), "got:\n{filter}");
    }

    // -- clearing one condition leaves the others untouched (roadmap criterion 6) --

    #[test]
    fn clearing_one_condition_leaves_the_others_untouched_in_the_emitted_block() {
        let mut when = When::default();
        set_class(&mut when, "Boots", true);
        set_sockets_min(&mut when, 2);
        set_corrupted(&mut when, Some(false));

        let mut config = base_config();
        config.rules = Some(vec![Rule {
            enabled: None,
            name: None,
            style: "unchanged".to_string(),
            when: Some(when.clone()),
        }]);
        let before = crate::emitter::generate(&config, &crate::pricer::PricedData::empty()).expect("generate");
        assert!(before.contains("Class \"Boots\""));
        assert!(before.contains("Sockets >= 2"));
        assert!(before.contains("Corrupted False"));

        // Clear only sockets.
        clear_sockets(&mut when);
        config.rules = Some(vec![Rule {
            enabled: None,
            name: None,
            style: "unchanged".to_string(),
            when: Some(when),
        }]);
        let after = crate::emitter::generate(&config, &crate::pricer::PricedData::empty()).expect("generate");

        assert!(after.contains("Class \"Boots\""), "the class line must be untouched");
        assert!(after.contains("Corrupted False"), "the corrupted line must be untouched");
        assert!(!after.contains("Sockets"), "only the sockets line should be gone");
    }
}
