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

use crate::models::config::{Rule, When};

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
}
