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
//! - **name** — its own `circular_toggle` ([`render_name`]); off calls [`edit::clear_name`]
//!   (D-12). Not a condition — a name labels the whole rule — but shares the toggle vocabulary
//!   and sits first in the body (D-16).
//! - **classes** — its own `circular_toggle` ([`render_classes`]); off calls
//!   [`edit::clear_class`] and hides the four `CollapsingHeader` groups entirely rather than
//!   greying them out (D-06/D-07/D-08). Deselecting every class individually still collapses
//!   `When.class` to `None` by the other route ([`edit::set_class`]'s own doc comment explains
//!   why) — the toggle is a second way to reach the same state, not a replacement for it.
//! - **base types** — its own `circular_toggle` ([`render_base_types`]); off calls
//!   [`edit::clear_base_type`] and hides the list and add field entirely (D-16, sits directly
//!   after classes).
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
//! Excluded from this editor entirely: `areaLevel` (deliberately outside the dynamic vocabulary,
//! DYN-05).
//!
//! Three of the toggles above — **name**, **classes** and **base types** — are backed by
//! [`panel_open`]'s ephemeral, id-keyed state rather than derived from `Option::is_some()` every
//! frame the way every other toggle in this list is: they have no sensible non-empty value to
//! write on enable (D-09 forbids a pre-selected class, D-11 forbids an auto-filled name, and
//! there is no default base type), so a derived toggle would read back `false` on the very next
//! repaint and the panel would flicker shut before the player could act (RESEARCH Pattern 1 /
//! Pitfall 1). Do not "simplify" these three into the uniform `Option::is_some()` pattern the
//! rest of this list uses — that is the exact regression this note exists to prevent.
//!
//! This inventory is auditable by reading this file, not by clicking through the accordion.

use crate::models::config::{Config, Rarity, RarityOp, Rule};

use super::edit::{self, RuleId};
use super::theme;
use super::widgets;

/// Ephemeral, presentation-only "is this optional-value panel open" state, keyed by an `egui::Id`
/// the caller has already salted with the rule's [`RuleId`] plus a control discriminator (e.g.
/// `(id, "name-open")`).
///
/// Deliberately **not** derived from `Option::is_some()` every frame the way every other
/// condition's toggle is (`rarity`, `sockets`, `quality`, `corrupted`, `unidentified_tier`,
/// `defence`) — [`render_name`], `render_classes`'s off switch and `render_base_types`'s off
/// switch write *nothing* to the model when switched on (D-09 forbids auto-selecting a class,
/// D-11 forbids auto-filling a name, and there is no sensible default base type), so a toggle
/// that re-derived its checked-state from the model each frame would read back `false` on the
/// very next repaint and the panel would flicker shut before the player could click or type
/// anything (RESEARCH Pattern 1 / Pitfall 1).
///
/// Deliberately **not** stored in [`super::edit`] or `WorkingRules` either: this state carries no
/// stake in the emitted filter (`PITFALLS.md` §8's accepted "no test story" surface for
/// presentation-only state), and giving it a home in the pure layer would mean inventing a
/// second persisted per-rule identity scheme alongside `RuleId` — exactly what D-13 and
/// `PITFALLS.md` §9 warn against. Because the id is salted with `RuleId`, this state is
/// automatically discarded whenever Revert mints fresh ids for a freshly reloaded rule set.
fn panel_open(ui: &mut egui::Ui, salt_id: egui::Id, seed: bool) -> bool {
    ui.data_mut(|d| *d.get_temp_mut_or(salt_id, seed))
}

/// Writes the ephemeral panel-open state described on [`panel_open`]. Called once, by the click
/// handler that flips the toggle — never by the render loop re-deriving a value every frame.
fn set_panel_open(ui: &mut egui::Ui, salt_id: egui::Id, open: bool) {
    ui.data_mut(|d| d.insert_temp(salt_id, open));
}

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

    changed |= render_name(ui, rule, id);
    ui.add_space(8.0);
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

/// EDIT-11/D-10/D-11/D-12/D-16: a rule's optional name, rendered first in the body — a name
/// labels the whole rule and is not a condition, so it sits above `style` rather than beside the
/// other toggles (D-16).
///
/// Backed by [`panel_open`] rather than `rule.name.is_some()` (RESEARCH Pattern 1): D-11 forbids
/// auto-filling a name on enable, so a toggle that re-derived its checked-state from the model
/// every frame would read back `false` the very next repaint and the field would flicker shut
/// before the player could type anything.
fn render_name(ui: &mut egui::Ui, rule: &mut Rule, id: RuleId) -> bool {
    let mut changed = false;
    ui.label(egui::RichText::new("NAME").color(theme::LABEL_TAN).small());

    let open_id = egui::Id::new((id, "name-open"));
    let mut enabled = panel_open(ui, open_id, rule.name.is_some());

    ui.horizontal(|ui| {
        if widgets::circular_toggle(ui, &mut enabled).changed() {
            set_panel_open(ui, open_id, enabled);
            if !enabled {
                // D-12: unticking discards the stored name, matching every other off switch's
                // clear_* = None behavior rather than secretly retaining hidden text.
                edit::clear_name(rule);
            }
            // Ticking ON writes nothing else here (D-11) -- the field stays empty until typed.
            changed = true;
        }
        ui.label("has a name");
    });

    if enabled {
        // A per-rule text buffer held in the same ephemeral store, seeded once from the rule's
        // current name. Must be temp-stored rather than re-derived from `rule.name` every frame:
        // `edit::set_name` trims on commit, so re-deriving the buffer from the (not-yet-committed)
        // model each frame would delete a trailing space the instant the player typed one
        // mid-name.
        let buf_id = egui::Id::new((id, "name-buf"));
        let mut buf = ui.data_mut(|d| {
            d.get_temp_mut_or_insert_with(buf_id, || rule.name.clone().unwrap_or_default())
                .clone()
        });

        let response = ui.add(
            egui::TextEdit::singleline(&mut buf)
                .id_salt((id, "name-edit"))
                .hint_text("unnamed"),
        );
        if response.changed() {
            ui.data_mut(|d| d.insert_temp(buf_id, buf.clone()));
        }
        // Commit on losing focus for any reason -- on Enter, and also on clicking away, because a
        // name typed and then clicked away from should not be silently discarded (this
        // deliberately differs from the base-type add field, where a half-typed entry must NOT be
        // committed by a stray click).
        if response.lost_focus() {
            edit::set_name(rule, &buf);
            changed = true;
            if ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                // Enter specifically -- keep the keyboard in the field so a run of edits doesn't
                // need the mouse.
                response.request_focus();
            }
        }
    }

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

/// EDIT-04/D-06/D-07/D-08/D-09/D-10: a grouped, collapsible class picker sourced from the
/// config's own class lists ([`edit::class_groups`]) — one `CollapsingHeader` per group
/// (WEAPONS / ARMOUR / JEWELLERY / FLASKS & CHARMS), collapsed by default so two dozen
/// checkboxes never swamp the stripe at once. Empty groups are omitted by `class_groups` rather
/// than rendered as a bare header. The selected count is shown in each group's header text so a
/// collapsed group still tells the player something.
///
/// EDIT-08 for classes: gated behind its own `circular_toggle`, backed by [`panel_open`] rather
/// than `when.class.is_some()` (RESEARCH Pattern 1) — D-09 forbids pre-selecting a class on
/// enable, so a derived toggle would flicker shut the frame after being switched on. While the
/// toggle is off, the four `CollapsingHeader` groups are not rendered at all — absent, not
/// greyed out and not collapsed-but-present (D-07), unlike [`render_defence`]'s group toggle,
/// which uses `add_enabled_ui` to grey its three checkboxes out: three checkboxes are worth
/// keeping visible while off, two dozen class checkboxes are not. Switching the toggle off
/// discards the class list via [`edit::clear_class`] (D-08) — the same end state
/// [`edit::set_class`] already reaches when the last class is deselected one at a time, reached
/// here in one step.
fn render_classes(ui: &mut egui::Ui, rule: &mut Rule, config: &Config, id: RuleId) -> bool {
    let mut changed = false;
    ui.label(egui::RichText::new("CLASSES").color(theme::LABEL_TAN).small());

    let open_id = egui::Id::new((id, "classes-open"));
    let mut enabled = panel_open(
        ui,
        open_id,
        rule.when.as_ref().is_some_and(|when| when.class.is_some()),
    );

    ui.horizontal(|ui| {
        if widgets::circular_toggle(ui, &mut enabled).changed() {
            set_panel_open(ui, open_id, enabled);
            if !enabled {
                if let Some(when) = rule.when.as_mut() {
                    edit::clear_class(when);
                }
            }
            // Ticking ON writes nothing else here (D-09) -- no class is pre-selected.
            changed = true;
        }
        ui.label("match classes");
    });

    if enabled {
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

    // Captured before the scope below, which narrows `available_width` as it lays out.
    let ui_width = ui.available_width();

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
        //
        // The visuals below are overridden in a scope rather than globally. egui paints a
        // slider's rail with `widgets.inactive.bg_fill`, which in this theme is the recessed
        // input fill (#090909) sitting on a #0D0D0D row — invisible by construction, so the
        // control read as a stray checkbox beside a number rather than a bar (reported from
        // live use 2026-09-07, screenshot poe2_print003). Scoping keeps every other widget on
        // the shared theme.
        let slider_changed = ui
            .scope(|ui| {
                let spacing = ui.spacing_mut();
                // Bounded, not "fill the row". The first cut used available-width minus a
                // margin, which ran the rail past the stripe's right edge and pushed the handle
                // and the value box out of sight entirely (screenshot poe2_print004). A five-
                // position scale needs only enough room to separate its stops, and the value
                // box needs room to sit beside it inside 460px.
                spacing.slider_width = (ui_width - 96.0).clamp(120.0, 200.0);
                spacing.slider_rail_height = 6.0;

                let visuals = ui.visuals_mut();
                visuals.handle_shape = egui::style::HandleShape::Circle;
                // The groove: warm and clearly lighter than the row behind it.
                visuals.widgets.inactive.bg_fill = theme::HEADER_PLATE;
                visuals.widgets.hovered.bg_fill = theme::HEADER_PLATE;
                visuals.widgets.active.bg_fill = theme::HEADER_PLATE;
                // The filled portion and the handle, in the same tone the checkmarks use so an
                // active value reads consistently across controls. WARM_ACCENT is deliberately
                // not used here — it is reserved for Reset (D-24).
                visuals.selection.bg_fill = theme::CONTROL_GLYPH;
                for state in [
                    &mut visuals.widgets.inactive,
                    &mut visuals.widgets.hovered,
                    &mut visuals.widgets.active,
                ] {
                    state.fg_stroke = egui::Stroke::new(1.0, theme::SECTION_CREAM);
                }

                ui.add(
                    egui::Slider::new(
                        &mut min,
                        edit::UNIDENTIFIED_TIER_MIN..=edit::UNIDENTIFIED_TIER_MAX,
                    )
                    .integer()
                    .trailing_fill(true),
                )
                .changed()
            })
            .inner;

        if slider_changed {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::config::When;
    use egui::{Event, Modifiers, PointerButton, Pos2, RawInput, Rect};

    /// A one-frame click-and-release at `pos`, mirroring `widgets.rs`'s own test harness exactly
    /// (no second harness is invented for this file's controls).
    fn click_at(pos: Pos2) -> RawInput {
        RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, egui::vec2(400.0, 400.0))),
            events: vec![
                Event::PointerMoved(pos),
                Event::PointerButton {
                    pos,
                    button: PointerButton::Primary,
                    pressed: true,
                    modifiers: Modifiers::default(),
                },
                Event::PointerButton {
                    pos,
                    button: PointerButton::Primary,
                    pressed: false,
                    modifiers: Modifiers::default(),
                },
            ],
            ..Default::default()
        }
    }

    /// A no-input frame sharing `click_at`'s own 400x400 screen rect, so a control's layout is
    /// identical across a click-free frame and a click frame -- without this, `RawInput::default`
    /// (no `screen_rect` at all) can lay a row out differently than `click_at`'s input does.
    fn blank_input() -> RawInput {
        RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, egui::vec2(400.0, 400.0))),
            ..Default::default()
        }
    }

    fn headless_ctx() -> egui::Context {
        let ctx = egui::Context::default();
        ctx.set_fonts(egui::FontDefinitions::empty());
        ctx
    }

    fn unnamed_rule() -> Rule {
        Rule { enabled: None, name: None, style: "unchanged".to_string(), when: None }
    }

    /// The same fixture `edit.rs`'s own tests load a real `Config` from — a full, valid class
    /// list to drive `render_classes` against, rather than a hand-built stand-in that could drift
    /// from what the config-sourced picker actually offers (`PITFALLS.md` §10).
    fn base_config() -> Config {
        let json = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/dynamic_base.json"));
        serde_json::from_str(json).expect("dynamic_base.json must parse as a Config")
    }

    // -- panel_open / set_panel_open (the shared primitive) --------------------------------

    #[test]
    fn a_panel_opened_by_a_click_is_still_open_on_the_next_frame_with_a_contrary_seed() {
        let ctx = headless_ctx();
        let id = egui::Id::new("test-panel-open");

        ctx.run_ui(RawInput::default(), |ui| {
            set_panel_open(ui, id, true);
        })
        .drop_without_applying_deltas();

        let mut open = false;
        ctx.run_ui(RawInput::default(), |ui| {
            open = panel_open(ui, id, false);
        })
        .drop_without_applying_deltas();

        assert!(
            open,
            "panel_open must return the previously-written state, not the contrary seed passed \
             on this call"
        );
    }

    #[test]
    fn an_untouched_panel_falls_back_to_its_model_derived_seed() {
        let ctx = headless_ctx();

        let mut open_true = false;
        ctx.run_ui(RawInput::default(), |ui| {
            open_true = panel_open(ui, egui::Id::new("fresh-a"), true);
        })
        .drop_without_applying_deltas();
        assert!(open_true, "a fresh id with a true seed must read true");

        let mut open_false = true;
        ctx.run_ui(RawInput::default(), |ui| {
            open_false = panel_open(ui, egui::Id::new("fresh-b"), false);
        })
        .drop_without_applying_deltas();
        assert!(!open_false, "a fresh id with a false seed must read false");
    }

    // -- render_name -------------------------------------------------------------------------

    #[test]
    fn render_name_never_auto_fills_a_name() {
        let ctx = headless_ctx();
        let mut wr = edit::WorkingRules::from_rules(vec![unnamed_rule()]);
        let id = wr.rows()[0].0;

        for _ in 0..2 {
            ctx.run_ui(RawInput::default(), |ui| {
                let rule = wr.rule_mut(id).unwrap();
                let changed = render_name(ui, rule, id);
                assert!(!changed, "render_name must not report a change with no input");
            })
            .drop_without_applying_deltas();
        }

        assert_eq!(wr.rule(id).unwrap().name, None);
    }

    #[test]
    fn the_name_field_survives_the_frame_after_its_toggle_was_clicked() {
        let ctx = headless_ctx();
        let mut wr = edit::WorkingRules::from_rules(vec![unnamed_rule()]);
        let id = wr.rows()[0].0;

        // Frame 1: no input, discover the toggle's own rect.
        ctx.run_ui(blank_input(), |ui| {
            let rule = wr.rule_mut(id).unwrap();
            render_name(ui, rule, id);
        })
        .drop_without_applying_deltas();
        let rects = ctx.interactive_rects_last_pass();
        assert_eq!(
            rects.len(),
            1,
            "a closed name panel renders exactly one interactive widget: its toggle"
        );
        let toggle_rect = rects[0];

        // Frame 2: click the toggle, switching the panel on.
        ctx.run_ui(click_at(toggle_rect.center()), |ui| {
            let rule = wr.rule_mut(id).unwrap();
            render_name(ui, rule, id);
        })
        .drop_without_applying_deltas();

        // Frame 3: no input at all -- the repaint that would expose the flicker.
        ctx.run_ui(blank_input(), |ui| {
            let rule = wr.rule_mut(id).unwrap();
            render_name(ui, rule, id);
        })
        .drop_without_applying_deltas();

        let rects_after = ctx.interactive_rects_last_pass();
        assert!(
            rects_after.len() > 1,
            "the name field must still be open the frame after its toggle was clicked, even \
             though nothing has been typed (RESEARCH Pitfall 1)"
        );
        assert_eq!(
            wr.rule(id).unwrap().name,
            None,
            "opening the panel must not auto-fill a name (D-11)"
        );
    }

    // -- render_classes ----------------------------------------------------------------------

    #[test]
    fn the_class_groups_survive_the_frame_after_the_toggle_was_clicked() {
        let ctx = headless_ctx();
        let config = base_config();
        let mut wr = edit::WorkingRules::from_rules(vec![Rule {
            enabled: None,
            name: None,
            style: "unchanged".to_string(),
            when: Some(When::default()),
        }]);
        let id = wr.rows()[0].0;

        // Frame 1: no input -- with nothing selected, the toggle is the only click-sensing
        // widget; the four `CollapsingHeader`s do not exist yet.
        ctx.run_ui(blank_input(), |ui| {
            let rule = wr.rule_mut(id).unwrap();
            render_classes(ui, rule, &config, id);
        })
        .drop_without_applying_deltas();
        let rects = ctx.interactive_rects_last_pass();
        assert_eq!(
            rects.len(),
            1,
            "a closed class panel renders exactly one interactive widget: its toggle"
        );
        let toggle_rect = rects[0];

        // Frame 2: click the toggle, switching the panel on.
        ctx.run_ui(click_at(toggle_rect.center()), |ui| {
            let rule = wr.rule_mut(id).unwrap();
            render_classes(ui, rule, &config, id);
        })
        .drop_without_applying_deltas();

        // Frame 3: no input at all -- the repaint that would expose the flicker.
        ctx.run_ui(blank_input(), |ui| {
            let rule = wr.rule_mut(id).unwrap();
            render_classes(ui, rule, &config, id);
        })
        .drop_without_applying_deltas();

        let rects_after = ctx.interactive_rects_last_pass();
        assert!(
            rects_after.len() > 1,
            "the class groups must still be visible the frame after the toggle was clicked, \
             even though nothing has been selected (RESEARCH Pitfall 1)"
        );
        assert_eq!(
            wr.rule(id).unwrap().when.as_ref().unwrap().class,
            None,
            "opening the panel must not pre-select a class (D-09)"
        );
    }
}
