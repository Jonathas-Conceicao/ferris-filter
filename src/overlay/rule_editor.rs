//! The expanded rule body — EDIT-03 through EDIT-08's editing vocabulary made visible: style,
//! classes, rarity, defences, sockets, quality, corrupted, unidentified tier. Every control
//! here writes through
//! [`super::edit`]'s pure mutation functions rather than assigning a `When` field inline (D-18,
//! roadmap success criterion 8) — this file only decides *what to draw* and *which `edit::`
//! function to call in response to a click*, never *what a `When` field's value should become*.
//!
//! **Layout convention for the whole file:** every control is a vertical stack — a
//! theme-coloured label line, then the control beneath it — never a side-by-side label/control
//! pair. The expanded row lives inside the stripe's fixed 460px width and must never resize
//! (D-01), so `ui.add_space` separates controls instead of `ui.columns`.
//!
//! **Every option list is config-sourced.** Styles come from [`edit::available_styles`], class
//! names from [`edit::class_groups`] — never a hardcoded literal in this file
//! (`PITFALLS.md` §10: hand-typed class names already caused a real in-game defect in v0.2).
//!
//! ---
//!
//! EDIT-08 condition inventory — every condition this editor exposes has exactly one documented
//! way to be switched off, and switching one off changes nothing about the others:
//!
//! - **style** — always present (a rule must have a style); not a condition, no off switch.
//! - **classes** — off by deselecting every class in every group; [`edit::set_class`] already
//!   collapses the last deselection to `None` (its own doc comment explains why), so no separate
//!   "classes off" toggle is needed here.
//! - **rarity** — one `circular_toggle` ([`render_rarity`]).
//! - **defences** — one `circular_toggle` for the *group* ([`render_defence`]); the three
//!   checkboxes underneath never gain their own separate on/off state, only a ticked/unticked
//!   value while the group is on.
//! - **sockets** — one `circular_toggle` ([`render_sockets`]).
//! - **quality** — one `circular_toggle` ([`render_quality`]).
//! - **corrupted** — one `circular_toggle` ([`render_corrupted`]), deliberately reusing the same
//!   enable-toggle vocabulary as every other condition instead of a bespoke tri-state widget
//!   (RESEARCH Pattern 2) — do not "improve" this into a three-way cycling control.
//! - **unidentified tier** — one `circular_toggle` ([`render_unidentified_tier`]). Added
//!   2026-09-07 after live use found the condition displayed in the row summary but uneditable;
//!   EDIT-07 never named it, so the omission came from the requirement rather than execution.
//!
//! Not exposed here: `baseType` (no config list to source exact base names from, and no dynamic
//! rule uses it) and `areaLevel` (deliberately outside the dynamic vocabulary, DYN-05).
//!
//! This inventory is auditable by reading this file, not by clicking through the accordion.

use crate::models::config::{Config, Rarity, RarityOp, Rule};

use super::edit::{self, RuleId};
use super::theme;
use super::widgets;

/// Renders one expanded rule's condition editors. Returns `true` if any control changed the
/// rule this frame, for the caller's autosave dirty-tracking (D-02).
///
/// `id` salts every `ComboBox`/`CollapsingHeader` this function (or a function it calls)
/// creates, together with a per-control discriminator, so two different rows' controls never
/// share an egui id — plain value widgets (checkboxes, `selectable_value`, `DragValue`) don't
/// need their own salt, since they already render inside this row's uniquely-salted
/// `CollapsingHeader` scope (the accordion in `app.rs`).
pub fn render_rule_body(ui: &mut egui::Ui, rule: &mut Rule, config: &Config, id: RuleId) -> bool {
    let mut changed = false;

    changed |= render_style(ui, rule, config, id);
    ui.add_space(8.0);
    changed |= render_classes(ui, rule, config, id);
    ui.add_space(8.0);
    changed |= render_rarity(ui, rule, id);
    ui.add_space(8.0);
    changed |= render_defence(ui, rule, id);
    ui.add_space(8.0);
    changed |= render_sockets(ui, rule, id);
    ui.add_space(8.0);
    changed |= render_quality(ui, rule, id);
    ui.add_space(8.0);
    changed |= render_corrupted(ui, rule, id);
    ui.add_space(8.0);
    changed |= render_unidentified_tier(ui, rule, id);

    changed
}

/// EDIT-03/D-09: a style dropdown whose options are exactly the styles the loaded config
/// defines ([`edit::available_styles`]) — never a literal style name in this file, so an
/// unresolvable style can never reach the store through this control.
fn render_style(ui: &mut egui::Ui, rule: &mut Rule, config: &Config, id: RuleId) -> bool {
    let mut changed = false;
    ui.label(egui::RichText::new("STYLE").color(theme::LABEL_TAN).small());

    let styles = edit::available_styles(&config.styles);
    let mut current = rule.style.clone();
    egui::ComboBox::from_id_salt((id, "style"))
        .selected_text(current.clone())
        .show_ui(ui, |ui| {
            for name in &styles {
                ui.selectable_value(&mut current, (*name).to_string(), *name);
            }
        });
    if current != rule.style {
        edit::set_style(rule, &current);
        changed = true;
    }

    changed
}

/// EDIT-04/D-10: a grouped, collapsible class picker sourced from the config's own class lists
/// ([`edit::class_groups`]) — one `CollapsingHeader` per group (WEAPONS / ARMOUR / JEWELLERY /
/// FLASKS & CHARMS), collapsed by default so two dozen checkboxes never swamp the stripe at once.
/// Empty groups are omitted by `class_groups` rather than rendered as a bare header. The selected count is
/// shown in each group's header text so a collapsed group still tells the player something.
///
/// EDIT-08 for classes: there is no separate "classes off" toggle; deselecting the last class
/// already collapses `When.class` to `None` ([`edit::set_class`]'s own doc comment explains
/// why) — the emptied state IS "classes off".
fn render_classes(ui: &mut egui::Ui, rule: &mut Rule, config: &Config, id: RuleId) -> bool {
    let mut changed = false;
    ui.label(egui::RichText::new("CLASSES").color(theme::LABEL_TAN).small());

    for (group_label, classes) in edit::class_groups(config) {
        let selected_count = classes
            .iter()
            .filter(|class| {
                rule.when
                    .as_ref()
                    .is_some_and(|when| edit::is_class_selected(when, class))
            })
            .count();

        egui::CollapsingHeader::new(format!("{group_label} ({selected_count})"))
            .id_salt((id, group_label))
            .default_open(false)
            .show(ui, |ui| {
                for class in classes {
                    let mut selected = rule
                        .when
                        .as_ref()
                        .is_some_and(|when| edit::is_class_selected(when, class));
                    if ui.checkbox(&mut selected, class.as_str()).changed() {
                        let when = edit::when_mut(rule);
                        edit::set_class(when, class, selected);
                        changed = true;
                    }
                }
            });
    }

    changed
}

/// EDIT-05/D-11: rarity as a comparator plus a value, never typed free text. Assembling the
/// compact comparator-plus-rarity string form is `RarityComparator`'s own `Serialize` impl's
/// job — this function only ever passes a typed [`RarityOp`]/[`Rarity`] pair to
/// [`edit::set_rarity`], never assembles a comparator string itself.
fn render_rarity(ui: &mut egui::Ui, rule: &mut Rule, _id: RuleId) -> bool {
    let mut changed = false;
    ui.label(egui::RichText::new("RARITY").color(theme::LABEL_TAN).small());

    let mut enabled = rule.when.as_ref().and_then(|when| when.rarity).is_some();
    ui.horizontal(|ui| {
        if widgets::circular_toggle(ui, &mut enabled).changed() {
            if enabled {
                // Seeds a sensible default (>= Magic) through edit::set_rarity on enable.
                edit::set_rarity(edit::when_mut(rule), RarityOp::Gte, Rarity::Magic);
            } else if let Some(when) = rule.when.as_mut() {
                edit::clear_rarity(when);
            }
            changed = true;
        }
        ui.label("has rarity");
    });

    if enabled {
        if let Some(current) = rule.when.as_ref().and_then(|when| when.rarity) {
            let mut op = current.op;
            let mut value = current.rarity;
            let mut value_changed = false;

            ui.horizontal(|ui| {
                value_changed |= ui.selectable_value(&mut op, RarityOp::Eq, "Exactly").changed();
                value_changed |=
                    ui.selectable_value(&mut op, RarityOp::Gte, "At Least").changed();
                value_changed |= ui.selectable_value(&mut op, RarityOp::Lte, "At Most").changed();
            });
            ui.horizontal(|ui| {
                value_changed |=
                    ui.selectable_value(&mut value, Rarity::Normal, "Normal").changed();
                value_changed |=
                    ui.selectable_value(&mut value, Rarity::Magic, "Magic").changed();
                value_changed |= ui.selectable_value(&mut value, Rarity::Rare, "Rare").changed();
                value_changed |=
                    ui.selectable_value(&mut value, Rarity::Unique, "Unique").changed();
            });

            if value_changed {
                edit::set_rarity(edit::when_mut(rule), op, value);
                changed = true;
            }
        }
    }

    changed
}

/// EDIT-06/D-12: a group toggle plus three checkboxes reaches all seven PoE2 armour
/// combinations (armour/evasion/energy shield, each present or absent) without asking the
/// player to choose among seven named options. Ticked stores a strictly-positive minimum
/// (emitting a "greater than zero" filter condition); unticked stores an explicit exact-zero
/// condition — that asymmetry is the single least-guessable thing in this editor, so the row
/// states it directly rather than leaving it to be inferred. The actual encoding is
/// [`edit::set_defence`]'s job, not this file's.
fn render_defence(ui: &mut egui::Ui, rule: &mut Rule, _id: RuleId) -> bool {
    let mut changed = false;
    ui.label(egui::RichText::new("DEFENCES").color(theme::LABEL_TAN).small());
    ui.label(
        egui::RichText::new("Ticked = has this defence. Unticked = has none of it.")
            .color(theme::DISABLED_TEXT)
            .small(),
    );

    let mut group_on = rule.when.as_ref().is_some_and(edit::defence_group_enabled);
    ui.horizontal(|ui| {
        if widgets::circular_toggle(ui, &mut group_on).changed() {
            if group_on {
                edit::enable_defence_group(edit::when_mut(rule));
            } else if let Some(when) = rule.when.as_mut() {
                edit::clear_defence_group(when);
            }
            changed = true;
        }
        ui.label("Enable defence conditions");
    });

    // The three checkboxes grey out (not disappear) with the group -- the player can still see
    // the control exists even while it contributes nothing to the emitted block.
    ui.add_enabled_ui(group_on, |ui| {
        for (label, defence) in [
            ("Armour", edit::Defence::Armour),
            ("Evasion", edit::Defence::Evasion),
            ("Energy Shield", edit::Defence::EnergyShield),
        ] {
            let mut ticked = rule
                .when
                .as_ref()
                .is_some_and(|when| edit::defence_ticked(when, defence));
            if ui.checkbox(&mut ticked, label).changed() {
                edit::set_defence(edit::when_mut(rule), defence, ticked);
                changed = true;
            }
        }
    });

    changed
}

/// EDIT-07/D-13: sockets as a minimum-value field with its own enable toggle. `DragValue`
/// already implements click-to-type entry (RESEARCH "Don't Hand-Roll") — no hand-rolled
/// text-entry-plus-parse-on-blur widget is built here.
fn render_sockets(ui: &mut egui::Ui, rule: &mut Rule, _id: RuleId) -> bool {
    let mut changed = false;
    ui.label(egui::RichText::new("SOCKETS").color(theme::LABEL_TAN).small());

    let mut enabled = rule.when.as_ref().is_some_and(|when| when.sockets.is_some());
    ui.horizontal(|ui| {
        if widgets::circular_toggle(ui, &mut enabled).changed() {
            if enabled {
                edit::set_sockets_min(edit::when_mut(rule), 1);
            } else if let Some(when) = rule.when.as_mut() {
                edit::clear_sockets(when);
            }
            changed = true;
        }
        ui.label("min sockets");

        if enabled {
            let mut min = rule
                .when
                .as_ref()
                .and_then(|when| when.sockets.as_ref())
                .and_then(|nc| nc.min)
                .unwrap_or(1);
            if ui
                .add(egui::DragValue::new(&mut min).range(0..=6).speed(0.2))
                .changed()
            {
                edit::set_sockets_min(edit::when_mut(rule), min);
                changed = true;
            }
        }
    });

    changed
}

/// EDIT-07/D-13: quality as a minimum-value field with its own enable toggle — mirrors
/// [`render_sockets`] exactly, over the `quality` field instead of `sockets`.
fn render_quality(ui: &mut egui::Ui, rule: &mut Rule, _id: RuleId) -> bool {
    let mut changed = false;
    ui.label(egui::RichText::new("QUALITY").color(theme::LABEL_TAN).small());

    let mut enabled = rule.when.as_ref().is_some_and(|when| when.quality.is_some());
    ui.horizontal(|ui| {
        if widgets::circular_toggle(ui, &mut enabled).changed() {
            if enabled {
                edit::set_quality_min(edit::when_mut(rule), 1);
            } else if let Some(when) = rule.when.as_mut() {
                edit::clear_quality(when);
            }
            changed = true;
        }
        ui.label("min quality");

        if enabled {
            let mut min = rule
                .when
                .as_ref()
                .and_then(|when| when.quality.as_ref())
                .and_then(|nc| nc.min)
                .unwrap_or(1);
            if ui
                .add(egui::DragValue::new(&mut min).range(0..=20).speed(0.2))
                .changed()
            {
                edit::set_quality_min(edit::when_mut(rule), min);
                changed = true;
            }
        }
    });

    changed
}

/// Unidentified item tier as a minimum-value field with its own enable toggle — mirrors
/// [`render_sockets`], over `unidentified_tier`.
///
/// Added 2026-09-07 after live use: `dynamic_base.json`'s rules 7 and 8 carry this condition
/// and the collapsed summary showed it, but nothing could change it. EDIT-07 enumerated only
/// sockets, quality and corrupted, so the gap came from the requirement, not from execution.
///
/// Uses a 1..=5 integer slider rather than the drag-value the other numeric conditions use:
/// PoE2's unidentified tier is a small, fully-enumerable scale, so every valid position is
/// visible at once and an out-of-range value cannot be typed or dragged in. Sockets and quality
/// keep their drag-values because their ranges are wider and less evenly meaningful.
///
/// Author's request, 2026-09-07. The first cut shipped a 0..=10 drag-value, which allowed tiers
/// the game does not have.
fn render_unidentified_tier(ui: &mut egui::Ui, rule: &mut Rule, _id: RuleId) -> bool {
    let mut changed = false;
    ui.label(
        egui::RichText::new("UNIDENTIFIED TIER")
            .color(theme::LABEL_TAN)
            .small(),
    );

    let mut enabled = rule
        .when
        .as_ref()
        .is_some_and(|when| when.unidentified_tier.is_some());
    ui.horizontal(|ui| {
        if widgets::circular_toggle(ui, &mut enabled).changed() {
            if enabled {
                edit::set_unidentified_tier_min(edit::when_mut(rule), edit::UNIDENTIFIED_TIER_DEFAULT);
            } else if let Some(when) = rule.when.as_mut() {
                edit::clear_unidentified_tier(when);
            }
            changed = true;
        }
        ui.label("min tier");
    });

    if enabled {
        let mut min = rule
            .when
            .as_ref()
            .and_then(|when| when.unidentified_tier.as_ref())
            .map(|ut| ut.min)
            .unwrap_or(edit::UNIDENTIFIED_TIER_DEFAULT)
            .clamp(edit::UNIDENTIFIED_TIER_MIN, edit::UNIDENTIFIED_TIER_MAX);
        // Full width on its own row: at 460px the slider needs the space to keep its five
        // positions distinguishable, which is the whole reason it is a slider and not a
        // drag-value.
        if ui
            .add(
                egui::Slider::new(
                    &mut min,
                    edit::UNIDENTIFIED_TIER_MIN..=edit::UNIDENTIFIED_TIER_MAX,
                )
                .integer(),
            )
            .changed()
        {
            edit::set_unidentified_tier_min(edit::when_mut(rule), min);
            changed = true;
        }
    }

    changed
}

/// EDIT-07/EDIT-08/D-13/D-14: corrupted deliberately reuses the same enable-toggle + value
/// vocabulary as every other condition (RESEARCH Pattern 2) instead of a bespoke tri-state
/// widget — `None` is "unset" (no condition at all, the same mechanism as every other
/// condition's "off" state), `Some(false)`/`Some(true)` is an ordinary two-way value. Do not
/// "improve" this into a three-way cycling control; that would make `corrupted` the only
/// condition in this editor with its own bespoke interaction pattern.
fn render_corrupted(ui: &mut egui::Ui, rule: &mut Rule, _id: RuleId) -> bool {
    let mut changed = false;
    ui.label(egui::RichText::new("CORRUPTED").color(theme::LABEL_TAN).small());

    let mut enabled = rule.when.as_ref().and_then(|when| when.corrupted).is_some();
    ui.horizontal(|ui| {
        if widgets::circular_toggle(ui, &mut enabled).changed() {
            if enabled {
                edit::set_corrupted(edit::when_mut(rule), Some(false));
            } else if let Some(when) = rule.when.as_mut() {
                edit::set_corrupted(when, None);
            }
            changed = true;
        }

        if enabled {
            let mut value =
                rule.when.as_ref().and_then(|when| when.corrupted).unwrap_or(false);
            let mut value_changed = false;
            value_changed |= ui.selectable_value(&mut value, false, "False").changed();
            value_changed |= ui.selectable_value(&mut value, true, "True").changed();
            if value_changed {
                edit::set_corrupted(edit::when_mut(rule), Some(value));
                changed = true;
            }
        } else {
            ui.label("unset");
        }
    });

    changed
}
