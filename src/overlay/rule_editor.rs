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
    // The caller renders this body *inside* a scope that blanks button chrome — `app.rs` sets
    // `widgets.inactive.weak_bg_fill = TRANSPARENT` and `bg_stroke = NONE` so the row header's own
    // frame stays invisible except on hover (the affordance that says the whole line is
    // clickable). Everything drawn in here inherits that, which is why the base-type Remove and
    // + ADD controls shipped as bare text while the row's own Delete — rendered outside that
    // scope — looked like a button (author, screenshot poe2_print006, 2026-09-09).
    //
    // Restoring the palette here rather than in `app.rs` keeps the fix next to the widgets that
    // need it: any control added to this body in future gets normal chrome without the author of
    // that control having to know about a scope two files away.
    ui.scope(|ui| {
        restore_body_chrome(ui.visuals_mut());
        render_rule_body_inner(ui, rule, config, id)
    })
    .inner
}

/// Puts back the button chrome the row header's own scope blanks, so controls in a rule body
/// render as buttons rather than as bare text.
///
/// Split out from [`render_rule_body`] so it can be asserted without a window: the defect it
/// fixes is invisible to every existing test, all of which check that widgets *exist* rather than
/// that they are drawn with a frame.
fn restore_body_chrome(v: &mut egui::Visuals) {
    let restored = theme::poe2_visuals();
    v.widgets.inactive.weak_bg_fill = restored.widgets.inactive.weak_bg_fill;
    v.widgets.inactive.bg_stroke = restored.widgets.inactive.bg_stroke;
    v.widgets.hovered.weak_bg_fill = restored.widgets.hovered.weak_bg_fill;
    v.widgets.hovered.bg_stroke = restored.widgets.hovered.bg_stroke;
    v.widgets.active.weak_bg_fill = restored.widgets.active.weak_bg_fill;
    v.widgets.active.bg_stroke = restored.widgets.active.bg_stroke;
    // The header's full-width framing is the header's business, not the body's — the class group
    // headers in here are plain disclosure triangles.
    v.collapsing_header_frame = restored.collapsing_header_frame;
}

fn render_rule_body_inner(ui: &mut egui::Ui, rule: &mut Rule, config: &Config, id: RuleId) -> bool {
    let mut changed = false;

    changed |= render_name(ui, rule, id);
    ui.add_space(8.0);
    changed |= render_style(ui, rule, config, id);
    ui.add_space(8.0);
    changed |= render_classes(ui, rule, config, id);
    ui.add_space(8.0);
    changed |= render_base_types(ui, rule, id);
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
    ui.label(
        egui::RichText::new("CLASSES")
            .color(theme::LABEL_TAN)
            .small(),
    );

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

/// EDIT-10/D-01/D-02/D-03/D-05/D-14/D-16: the base-type list, sitting directly after the class
/// control (D-16 — class and base type are the two "which item is this" conditions).
///
/// Gated behind its own `circular_toggle`, backed by [`panel_open`] the same way
/// [`render_name`]/[`render_classes`] are (RESEARCH Pattern 1): there is no sensible base type to
/// seed on enable, so a derived toggle would flicker shut exactly as it would for name/class.
/// Off hides the list and the add field entirely and calls [`edit::clear_base_type`]; on writes
/// only the open state and adds no default entry, keeping a freshly-opened panel empty (D-09's
/// spirit extended to base types).
///
/// While open, an **owned clone** of the rule's base-type list is taken once at the top and
/// rendered one row per entry — never a live borrow — so the remove button below can call
/// [`edit::remove_base_type`] without mutating the array mid-walk. A remove click records the
/// index in a single `Option<usize>`; the removal is applied once, after the loop ends (Pattern
/// 2, mirroring `app.rs`'s `RowAction` collect-then-apply discipline, scoped to this one
/// function). The remove control is a plain `Remove` text button, matching `app.rs`'s existing
/// `Delete` button, deliberately not a Unicode close/cross glyph — this codebase has already
/// shipped that exact tofu-box rendering defect twice (Phase 13's header close control, Phase
/// 14's reorder arrows).
///
/// The add field commits on Enter (the response losing focus in the same frame `Key::Enter` was
/// pressed) or the `+ ADD` button — and only those two, so clicking elsewhere never commits a
/// half-typed name (this deliberately differs from [`render_name`], where any loss of focus
/// commits). On commit the buffer is parsed by [`edit::parse_base_type_entry`] (D-14's quote
/// stripping and paste splitting) and each result is appended via [`edit::add_base_type`]; the
/// buffer is then cleared and refocused so several names can be typed in a row with only Enter
/// between them (D-03). This file decides nothing about what an entry's text becomes — that
/// hygiene lives entirely in `edit::` (D-18).
fn render_base_types(ui: &mut egui::Ui, rule: &mut Rule, id: RuleId) -> bool {
    let mut changed = false;
    ui.label(
        egui::RichText::new("BASE TYPES")
            .color(theme::LABEL_TAN)
            .small(),
    );

    let open_id = egui::Id::new((id, "base-type-open"));
    let mut enabled = panel_open(
        ui,
        open_id,
        rule.when
            .as_ref()
            .is_some_and(|when| when.base_type.is_some()),
    );

    ui.horizontal(|ui| {
        if widgets::circular_toggle(ui, &mut enabled).changed() {
            set_panel_open(ui, open_id, enabled);
            if !enabled {
                if let Some(when) = rule.when.as_mut() {
                    edit::clear_base_type(when);
                }
            }
            // Ticking ON writes nothing else here -- no default entry is added.
            changed = true;
        }
        ui.label("match base types");
    });

    if enabled {
        // Owned clone -- NOT a live borrow of `rule.when.base_type` -- so the remove button below
        // can freely call `edit::remove_base_type` without fighting the borrow checker or
        // mutating the array mid-walk (Pattern 2).
        let items: Vec<String> = rule
            .when
            .as_ref()
            .and_then(|when| when.base_type.clone())
            .unwrap_or_default();

        let mut remove_at: Option<usize> = None;
        for (i, name) in items.iter().enumerate() {
            // Right-to-left so the button hugs the stripe's right edge and the name takes what is
            // left, wrapping into it (author, 2026-09-09). Laying it out left-to-right put the
            // button first and left every base name indented behind a ragged column of buttons.
            //
            // **The size argument is load-bearing, not decoration.** `Ui::with_layout` builds its
            // child through `scope_builder`, which applies no size constraint, so the child
            // inherits the whole remaining rect — and `Align::Center` on a right-to-left layout is
            // a *vertical* alignment, which then centres one row inside an entire screenful. The
            // first cut used `with_layout` and produced exactly that: one base type per screen,
            // the panel running past the bottom of the stripe (author, screenshot poe2_print006).
            //
            // `allocate_ui_with_layout` with an explicit row height is what `Ui::horizontal` does
            // internally (`egui-0.36.1/src/ui.rs:2370`, `horizontal_with_main_wrap_dyn`) — this is
            // that recipe with the direction flipped. The height is an initial size, not a cap, so
            // a long name still wraps to a taller row.
            let row_size = egui::vec2(
                ui.available_size_before_wrap().x,
                ui.spacing().interact_size.y,
            );
            ui.allocate_ui_with_layout(
                row_size,
                egui::Layout::right_to_left(egui::Align::Center),
                |ui| {
                    // `small_button`, matching the row-level Delete in `app.rs` — both remove one
                    // thing from a list and should not look like different kinds of action.
                    if ui.small_button("Remove").clicked() {
                        remove_at = Some(i); // recorded, not applied -- Pattern 2
                    }
                    // The button is placed first so it claims the right edge; the name then reads
                    // from the LEFT edge of what remains. Without this inner flip the name is laid
                    // out right-to-left too and ends up hugging the button, leaving a ragged left
                    // margin — which is how it first shipped (author, 2026-09-09).
                    //
                    // Nesting is safe here where it was not before: the enclosing
                    // `allocate_ui_with_layout` has already bounded this row's height, so the
                    // unconstrained-child problem that produced poe2_print006 cannot recur.
                    ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                        ui.add(egui::Label::new(name).wrap());
                    });
                },
            );
        }
        if let Some(i) = remove_at {
            if let Some(when) = rule.when.as_mut() {
                edit::remove_base_type(when, i);
            }
            changed = true;
        }

        let buf_id = egui::Id::new((id, "base-type-buf"));
        let mut buf = ui.data_mut(|d| d.get_temp_mut_or_default::<String>(buf_id).clone());

        let mut text_response = None;
        let mut add_clicked = false;
        ui.horizontal(|ui| {
            let response = ui.add(
                egui::TextEdit::singleline(&mut buf)
                    .id_salt((id, "base-type-edit"))
                    .hint_text("Add base type\u{2026}"),
            );
            if response.changed() {
                ui.data_mut(|d| d.insert_temp(buf_id, buf.clone()));
            }
            // `button`, not `small_button`: every other action affordance in the stripe —
            // `+ ADD RULE`, `APPLY`, `REVERT` — is a full-size button, and this one committing an
            // entry is the same kind of act. `small_button` is reserved for the per-item removes
            // (author, 2026-09-09).
            add_clicked = ui.button("+ ADD").clicked();
            text_response = Some(response);
        });
        let response = text_response.expect("ui.horizontal always runs its closure");

        let commit_by_enter =
            response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
        if commit_by_enter || add_clicked {
            let parsed = edit::parse_base_type_entry(&buf);
            if !parsed.is_empty() {
                let when = edit::when_mut(rule);
                for name in parsed {
                    edit::add_base_type(when, &name);
                }
                changed = true;
            }
            ui.data_mut(|d| d.insert_temp(buf_id, String::new()));
            response.request_focus();
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
    ui.label(
        egui::RichText::new("RARITY")
            .color(theme::LABEL_TAN)
            .small(),
    );

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
                value_changed |= ui
                    .selectable_value(&mut op, RarityOp::Eq, "Exactly")
                    .changed();
                value_changed |= ui
                    .selectable_value(&mut op, RarityOp::Gte, "At Least")
                    .changed();
                value_changed |= ui
                    .selectable_value(&mut op, RarityOp::Lte, "At Most")
                    .changed();
            });
            ui.horizontal(|ui| {
                value_changed |= ui
                    .selectable_value(&mut value, Rarity::Normal, "Normal")
                    .changed();
                value_changed |= ui
                    .selectable_value(&mut value, Rarity::Magic, "Magic")
                    .changed();
                value_changed |= ui
                    .selectable_value(&mut value, Rarity::Rare, "Rare")
                    .changed();
                value_changed |= ui
                    .selectable_value(&mut value, Rarity::Unique, "Unique")
                    .changed();
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
    ui.label(
        egui::RichText::new("DEFENCES")
            .color(theme::LABEL_TAN)
            .small(),
    );
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
/// A full-width slider wearing the PoE2 palette, shared by every bounded numeric condition
/// (sockets, quality, unidentified tier).
///
/// The visuals are overridden in a `scope` rather than globally. egui paints a slider's rail with
/// `widgets.inactive.bg_fill`, which in this theme is the recessed input fill (#090909) sitting on
/// a #0D0D0D row — invisible by construction, so the control read as a stray checkbox beside a
/// number rather than a bar (reported from live use 2026-09-07, screenshot poe2_print003).
/// Scoping keeps every other widget on the shared theme.
///
/// Extracted 2026-09-09 when sockets and quality became sliders too. Written once here rather
/// than three times: a rail that is invisible in one control and not the others is exactly the
/// drift this repo has already paid for once.
///
/// `ui_width` must be captured by the caller *before* any layout narrows `available_width`.
fn themed_slider<T: egui::emath::Numeric>(
    ui: &mut egui::Ui,
    ui_width: f32,
    value: &mut T,
    range: std::ops::RangeInclusive<T>,
) -> bool {
    ui.scope(|ui| {
        let spacing = ui.spacing_mut();
        // Bounded, not "fill the row". The first cut used available-width minus a margin, which
        // ran the rail past the stripe's right edge and pushed the handle and the value box out
        // of sight entirely (screenshot poe2_print004).
        spacing.slider_width = (ui_width - 96.0).clamp(120.0, 200.0);
        spacing.slider_rail_height = 6.0;

        let visuals = ui.visuals_mut();
        visuals.handle_shape = egui::style::HandleShape::Circle;
        // The groove: warm and clearly lighter than the row behind it.
        visuals.widgets.inactive.bg_fill = theme::HEADER_PLATE;
        visuals.widgets.hovered.bg_fill = theme::HEADER_PLATE;
        visuals.widgets.active.bg_fill = theme::HEADER_PLATE;
        // The filled portion and the handle, in the same tone the checkmarks use so an active
        // value reads consistently across controls. WARM_ACCENT is deliberately not used here —
        // it is reserved for Reset (D-24).
        visuals.selection.bg_fill = theme::CONTROL_GLYPH;
        for state in [
            &mut visuals.widgets.inactive,
            &mut visuals.widgets.hovered,
            &mut visuals.widgets.active,
        ] {
            state.fg_stroke = egui::Stroke::new(1.0, theme::SECTION_CREAM);
        }

        ui.add(
            egui::Slider::new(value, range)
                .integer()
                .trailing_fill(true),
        )
        .changed()
    })
    .inner
}

fn render_sockets(ui: &mut egui::Ui, rule: &mut Rule, _id: RuleId) -> bool {
    let mut changed = false;
    ui.label(
        egui::RichText::new("SOCKETS")
            .color(theme::LABEL_TAN)
            .small(),
    );

    let mut enabled = rule
        .when
        .as_ref()
        .is_some_and(|when| when.sockets.is_some());
    ui.horizontal(|ui| {
        if widgets::circular_toggle(ui, &mut enabled).changed() {
            if enabled {
                edit::set_sockets_min(edit::when_mut(rule), edit::SOCKETS_DEFAULT);
            } else if let Some(when) = rule.when.as_mut() {
                edit::clear_sockets(when);
            }
            changed = true;
        }
        ui.label("min sockets");
    });

    // Captured before the scope below, which narrows `available_width` as it lays out.
    let ui_width = ui.available_width();

    if enabled {
        // A slider rather than a drag-value (author, 2026-09-09): six positions are few enough
        // to show at once, and a drag-value hides the range it accepts — you cannot see that 6
        // is the ceiling until you hit it. Same treatment as the unidentified-tier control.
        let mut min = rule
            .when
            .as_ref()
            .and_then(|when| when.sockets.as_ref())
            .and_then(|nc| nc.min)
            .unwrap_or(edit::SOCKETS_DEFAULT)
            .clamp(edit::SOCKETS_MIN, edit::SOCKETS_MAX);
        if themed_slider(
            ui,
            ui_width,
            &mut min,
            edit::SOCKETS_MIN..=edit::SOCKETS_MAX,
        ) {
            edit::set_sockets_min(edit::when_mut(rule), min);
            changed = true;
        }
    }

    changed
}

/// EDIT-07/D-13: quality as a minimum-value field with its own enable toggle — mirrors
/// [`render_sockets`] exactly, over the `quality` field instead of `sockets`.
fn render_quality(ui: &mut egui::Ui, rule: &mut Rule, _id: RuleId) -> bool {
    let mut changed = false;
    ui.label(
        egui::RichText::new("QUALITY")
            .color(theme::LABEL_TAN)
            .small(),
    );

    let mut enabled = rule
        .when
        .as_ref()
        .is_some_and(|when| when.quality.is_some());
    ui.horizontal(|ui| {
        if widgets::circular_toggle(ui, &mut enabled).changed() {
            if enabled {
                edit::set_quality_min(edit::when_mut(rule), edit::QUALITY_DEFAULT);
            } else if let Some(when) = rule.when.as_mut() {
                edit::clear_quality(when);
            }
            changed = true;
        }
        ui.label("min quality");
    });

    // Captured before the scope below, which narrows `available_width` as it lays out.
    let ui_width = ui.available_width();

    if enabled {
        // Ceiling is 30, not 20 — corrupted items exceed the normal quality cap, and a filter
        // that cannot express that cannot describe the items most worth calling out.
        let mut min = rule
            .when
            .as_ref()
            .and_then(|when| when.quality.as_ref())
            .and_then(|nc| nc.min)
            .unwrap_or(edit::QUALITY_DEFAULT)
            .clamp(edit::QUALITY_MIN, edit::QUALITY_MAX);
        if themed_slider(
            ui,
            ui_width,
            &mut min,
            edit::QUALITY_MIN..=edit::QUALITY_MAX,
        ) {
            edit::set_quality_min(edit::when_mut(rule), min);
            changed = true;
        }
    }

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
                edit::set_unidentified_tier_min(
                    edit::when_mut(rule),
                    edit::UNIDENTIFIED_TIER_DEFAULT,
                );
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
        // Full width on its own row via the shared [`themed_slider`], which carries the palette
        // fixes this control was the first to need.
        if themed_slider(
            ui,
            ui_width,
            &mut min,
            edit::UNIDENTIFIED_TIER_MIN..=edit::UNIDENTIFIED_TIER_MAX,
        ) {
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
    ui.label(
        egui::RichText::new("CORRUPTED")
            .color(theme::LABEL_TAN)
            .small(),
    );

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
            let mut value = rule
                .when
                .as_ref()
                .and_then(|when| when.corrupted)
                .unwrap_or(false);
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

    /// The defect from screenshot poe2_print006: every control in a rule body rendered as bare
    /// text, because `app.rs` draws the body inside a scope that blanks button chrome to keep the
    /// row header's own frame invisible.
    ///
    /// Reproduces the caller's blanking exactly, then asserts the body puts it back. Every other
    /// test in this file checks that widgets *exist*; none of them can see that a button has no
    /// frame, which is why this defect shipped through a green suite twice.
    #[test]
    fn a_rule_body_puts_back_the_button_chrome_the_row_header_blanks() {
        let mut blanked = theme::poe2_visuals();
        // Exactly what app.rs's header scope does.
        blanked.widgets.inactive.weak_bg_fill = egui::Color32::TRANSPARENT;
        blanked.widgets.inactive.bg_stroke = egui::Stroke::NONE;
        blanked.widgets.hovered.weak_bg_fill = theme::HEADER_PLATE;
        blanked.widgets.hovered.bg_stroke = egui::Stroke::NONE;
        blanked.widgets.active.weak_bg_fill = theme::HEADER_PLATE;
        blanked.widgets.active.bg_stroke = egui::Stroke::NONE;
        blanked.collapsing_header_frame = true;

        restore_body_chrome(&mut blanked);

        let expected = theme::poe2_visuals();
        assert_ne!(
            blanked.widgets.inactive.weak_bg_fill,
            egui::Color32::TRANSPARENT,
            "a button in a rule body must have a fill, or it reads as plain text"
        );
        assert_eq!(
            blanked.widgets.inactive.weak_bg_fill,
            expected.widgets.inactive.weak_bg_fill
        );
        assert_eq!(
            blanked.widgets.hovered.weak_bg_fill,
            expected.widgets.hovered.weak_bg_fill
        );
        assert!(
            !blanked.collapsing_header_frame,
            "the full-width header frame belongs to the row header, not to the class groups \
             inside the body"
        );
    }

    /// The same defect as
    /// [`a_rule_body_puts_back_the_button_chrome_the_row_header_blanks`], but asserted against
    /// what is actually *painted* rather than against the helper in isolation.
    ///
    /// The unit test above passes even if nobody calls `restore_body_chrome` — deleting the call
    /// from `render_rule_body` compiles clean and leaves it green, which is precisely the hole
    /// that let this ship. This test renders a real body under the caller's blanked visuals and
    /// requires a filled button rect in the output, so it fails if the call site goes away.
    #[test]
    fn a_button_in_a_rule_body_is_painted_with_a_fill_not_left_as_bare_text() {
        let mut when = When::default();
        edit::add_base_type(&mut when, "Heavy Belt");
        let mut wr = edit::WorkingRules::from_rules(vec![Rule {
            enabled: None,
            name: None,
            style: "unchanged".to_string(),
            when: Some(when),
        }]);
        let id = wr.rows()[0].0;
        let config = base_config();

        let ctx = headless_ctx();
        // Exactly what app.rs's row-header scope does before it renders the body.
        let mut blanked = theme::poe2_visuals();
        blanked.widgets.inactive.weak_bg_fill = egui::Color32::TRANSPARENT;
        blanked.widgets.inactive.bg_stroke = egui::Stroke::NONE;
        ctx.set_visuals(blanked);

        let output = ctx.run_ui(blank_input(), |ui| {
            let rule = wr.rule_mut(id).unwrap();
            render_rule_body(ui, rule, &config, id);
        });

        let wanted = theme::poe2_visuals().widgets.inactive.weak_bg_fill;
        let painted_button = output
            .shapes
            .iter()
            .any(|clipped| matches!(&clipped.shape, egui::Shape::Rect(r) if r.fill == wanted));
        output.drop_without_applying_deltas();

        assert!(
            painted_button,
            "no rect was painted with the theme's button fill ({wanted:?}) — every control in \
             the body is rendering as bare text, the poe2_print006 defect"
        );
    }

    fn sized_input(height: f32) -> RawInput {
        RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, egui::vec2(400.0, height))),
            ..Default::default()
        }
    }

    /// The base-type rows must size to their content, never to the space available.
    ///
    /// `Ui::with_layout` applies no size constraint to its child, so a `right_to_left` layout —
    /// whose `Align::Center` is a *vertical* alignment — centres a single row inside the entire
    /// remaining rect. Shipped that way once: one base type per screenful, the panel running past
    /// the bottom of the stripe (author, screenshot poe2_print006, 2026-09-09).
    ///
    /// Rendering the same three entries into a 400px-tall viewport and a 4000px-tall one must
    /// produce the same height. That equality is the invariant; a row that stretches fails it by
    /// roughly the difference between the two viewports.
    #[test]
    fn the_base_type_rows_size_to_their_content_not_to_the_space_available() {
        let mut when = When::default();
        for name in ["Heavy Belt", "Utility Belt", "Armoured Cap"] {
            edit::add_base_type(&mut when, name);
        }
        let rule = Rule {
            enabled: None,
            name: None,
            style: "unchanged".to_string(),
            when: Some(when),
        };

        let measure = |height: f32| -> f32 {
            let ctx = headless_ctx();
            let mut wr = edit::WorkingRules::from_rules(vec![rule.clone()]);
            let id = wr.rows()[0].0;
            let mut used = 0.0_f32;
            ctx.run_ui(sized_input(height), |ui| {
                let r = wr.rule_mut(id).unwrap();
                render_base_types(ui, r, id);
                used = ui.min_rect().height();
            })
            .drop_without_applying_deltas();
            used
        };

        let short = measure(400.0);
        let tall = measure(4000.0);

        assert!(
            (short - tall).abs() < 1.0,
            "base-type panel height tracks the viewport instead of its content: \
             {short} at 400px vs {tall} at 4000px"
        );
        assert!(
            short < 400.0,
            "three base types should not fill a 400px viewport; got {short}"
        );
    }

    fn unnamed_rule() -> Rule {
        Rule {
            enabled: None,
            name: None,
            style: "unchanged".to_string(),
            when: None,
        }
    }

    /// The same fixture `edit.rs`'s own tests load a real `Config` from — a full, valid class
    /// list to drive `render_classes` against, rather than a hand-built stand-in that could drift
    /// from what the config-sourced picker actually offers (`PITFALLS.md` §10).
    fn base_config() -> Config {
        let json = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/fixtures/v04-dynamic-base.json"
        ));
        let mut config: Config =
            serde_json::from_str(json).expect("dynamic_base.json must parse as a Config");
        config.styles = crate::models::config::StyleSet::committed_for_tests();
        config
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
                assert!(
                    !changed,
                    "render_name must not report a change with no input"
                );
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

    // -- render_base_types -------------------------------------------------------------------

    #[test]
    fn render_base_types_never_adds_an_entry_on_its_own() {
        let ctx = headless_ctx();
        let mut wr = edit::WorkingRules::from_rules(vec![Rule {
            enabled: None,
            name: None,
            style: "unchanged".to_string(),
            when: Some(When::default()),
        }]);
        let id = wr.rows()[0].0;

        for _ in 0..2 {
            ctx.run_ui(RawInput::default(), |ui| {
                let rule = wr.rule_mut(id).unwrap();
                render_base_types(ui, rule, id);
            })
            .drop_without_applying_deltas();
        }

        assert_eq!(wr.rule(id).unwrap().when.as_ref().unwrap().base_type, None);
    }

    #[test]
    fn the_base_type_list_survives_the_frame_after_the_toggle_was_clicked() {
        let ctx = headless_ctx();
        let mut wr = edit::WorkingRules::from_rules(vec![Rule {
            enabled: None,
            name: None,
            style: "unchanged".to_string(),
            when: Some(When::default()),
        }]);
        let id = wr.rows()[0].0;

        // Frame 1: no input -- neither the list nor the add field exists while the toggle is off.
        ctx.run_ui(blank_input(), |ui| {
            let rule = wr.rule_mut(id).unwrap();
            render_base_types(ui, rule, id);
        })
        .drop_without_applying_deltas();
        let rects = ctx.interactive_rects_last_pass();
        assert_eq!(
            rects.len(),
            1,
            "a closed base-type panel renders exactly one interactive widget: its toggle"
        );
        let toggle_rect = rects[0];

        // Frame 2: click the toggle, switching the panel on.
        ctx.run_ui(click_at(toggle_rect.center()), |ui| {
            let rule = wr.rule_mut(id).unwrap();
            render_base_types(ui, rule, id);
        })
        .drop_without_applying_deltas();

        // Frame 3: no input at all -- the repaint that would expose the flicker.
        ctx.run_ui(blank_input(), |ui| {
            let rule = wr.rule_mut(id).unwrap();
            render_base_types(ui, rule, id);
        })
        .drop_without_applying_deltas();

        let rects_after = ctx.interactive_rects_last_pass();
        assert!(
            rects_after.len() > 1,
            "the add field and its + ADD button must still be visible the frame after the \
             toggle was clicked, even though nothing has been typed (RESEARCH Pitfall 1)"
        );
        assert_eq!(
            wr.rule(id).unwrap().when.as_ref().unwrap().base_type,
            None,
            "opening the panel must not add a default base type (D-09's spirit extended here)"
        );
    }

    // -- render_rule_body ---------------------------------------------------------------------

    #[test]
    fn a_new_rule_opens_with_every_condition_untouched() {
        let ctx = headless_ctx();
        let config = base_config();
        let mut wr = edit::WorkingRules::from_rules(vec![Rule {
            enabled: None,
            name: None,
            style: "unchanged".to_string(),
            when: Some(When::default()),
        }]);
        let id = wr.rows()[0].0;

        for _ in 0..2 {
            ctx.run_ui(RawInput::default(), |ui| {
                let rule = wr.rule_mut(id).unwrap();
                let changed = render_rule_body(ui, rule, &config, id);
                assert!(
                    !changed,
                    "render_rule_body must not change the rule with no input"
                );
            })
            .drop_without_applying_deltas();
        }

        let rule = wr.rule(id).unwrap();
        assert_eq!(rule.name, None, "a new rule has nothing checked (D-09)");
        let when = rule.when.as_ref().unwrap();
        assert_eq!(when.class, None);
        assert_eq!(when.rarity, None);
        assert_eq!(when.area_level, None);
        assert_eq!(when.base_armour, None);
        assert_eq!(when.base_evasion, None);
        assert_eq!(when.base_energy_shield, None);
        assert_eq!(when.quality, None);
        assert_eq!(when.sockets, None);
        assert_eq!(when.corrupted, None);
        assert_eq!(when.base_type, None);
        assert_eq!(when.unidentified_tier, None);
    }
}
