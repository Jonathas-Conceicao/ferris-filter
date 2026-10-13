//! The CONFIG tab: a thin egui view over [`ConfigState`].
//!
//! Layer rule (same as `rule_list.rs`): all model mutations are returned as [`ConfigAction`]s and
//! applied by the app after the frame; this module never mutates [`ConfigState`] and never touches
//! the filesystem (the filter list and the style-use counts arrive through [`ConfigViewCtx`]).
//! The style editor deliberately reuses the Phase 20 action widgets in `body.rs`
//! (`colour_combo`, `slider_pair`, `SIZE_CHOICES`) so the vocabulary is learnt once.

use std::collections::{BTreeSet, HashMap};
use std::path::Path;

use egui::{Align, Layout, RichText, vec2};

use crate::models::config::{
    Beam, FilterColor, Minimap, MinimapShape, Sound, Style, StyleAction, StyleNameError, StyleSet,
};
use crate::settings;

use super::body;
use super::config_state::{
    APPLY_BLOCKED_TOOLTIP, ColourSlot, ConfigState, OUT_OF_DATE_MESSAGE, OutsideEdit, StyleEdit,
    config_warning, delete_confirm_text, dirty_status, outside_edit_message,
};
use super::theme;
use super::widgets;

const WHOLE_NUMBER: &str = "Enter a whole number";
const NOT_SET: &str = "Not set (game default)";
const BUILTIN_TIP: &str = "Built-in styles can't be deleted.";
const RENAME_TIP: &str = "Names can't be changed. Add a new style and delete this one.";
const UNIT_TIP: &str = "Set in tiers.json; not editable here.";
/// Label of the FILTER section toggle that sends `/reloaditemfilter` after a RULES Apply.
pub const RELOAD_TOGGLE_LABEL: &str = "Reload filter in game after Apply";
/// Caveat shown under the toggle: the game reloads whichever filter is selected in game.
pub const RELOAD_TOGGLE_CAPTION: &str = "Sends /reloaditemfilter to the game after a RULES Apply. It reloads the filter selected in game, so select this filter there too. Close the chat box before pressing Apply.";
const APPLY_NOTHING_TIP: &str = "No unapplied config changes.";
const APPLY_LOAD_ERROR_TIP: &str =
    "styles.json or tiers.json failed to load. Revert to reload them.";

/// Everything the CONFIG view asks the app to do. Applied after the frame.
#[derive(Debug, Clone, PartialEq)]
pub enum ConfigAction {
    SelectFilter(String),
    CreateFilter(String),
    ConfirmSwitch,
    CancelSwitch,
    SetReloadAfterApply(bool),
    AddStyle(String),
    EditStyle { name: String, edit: StyleEdit },
    DeleteStyle(String),
    TierText { tier: String, text: String },
    Apply,
    Revert,
    Reload,
    Overwrite,
}

/// View state that is not part of the model. No egui types, so it is unit-testable.
#[derive(Debug, Clone, Default)]
pub struct ConfigUi {
    /// The one expanded style row.
    pub expanded: Option<String>,
    /// Salt of every style row's id scope; bumped when the expanded row changes so per-row egui
    /// state (drafts, combo popups) never leaks to another row.
    pub epoch: u64,
    /// Open `+ ADD STYLE` name draft.
    pub style_draft: Option<String>,
    /// Open `New empty filter...` name draft.
    pub filter_draft: Option<String>,
    /// Style whose inline delete confirm is showing.
    pub delete_armed: Option<String>,
    pub revert_armed: bool,
    /// A just-added style to scroll into view.
    pub scroll_to: Option<String>,
    /// `Keep editing` was pressed on the outside-edit guard.
    pub outside_dismissed: bool,
    /// A name draft was just opened; its text field takes focus once.
    pub focus_draft: bool,
}

impl ConfigUi {
    /// Expand `name`, or collapse it when it is already the expanded row. Any change drops an
    /// armed delete confirm and orphans per-row egui state.
    pub fn set_expanded(&mut self, name: &str) {
        if self.expanded.as_deref() == Some(name) {
            self.expanded = None;
        } else {
            self.expanded = Some(name.to_string());
        }
        self.delete_armed = None;
        self.epoch = self.epoch.wrapping_add(1);
    }

    pub fn arm_delete(&mut self, name: &str) {
        self.delete_armed = Some(name.to_string());
    }

    /// Called by the app on a tab change: every inline confirm disarms.
    pub fn disarm_all(&mut self) {
        self.delete_armed = None;
        self.revert_armed = false;
        self.outside_dismissed = false;
    }
}

/// What the view needs from the app each frame.
pub struct ConfigViewCtx<'a> {
    /// `*.filter` names in the game folder; `None` when the folder cannot be read.
    pub filters: Option<&'a [String]>,
    pub active_filter: &'a str,
    pub game_dir: Option<&'a Path>,
    /// A filter the user picked while rules have unapplied edits.
    pub pending_switch: Option<&'a str>,
    /// Style name -> number of loaded rules using it.
    pub rule_users: &'a HashMap<String, usize>,
    /// Styles `config.json` itself references.
    pub config_refs: &'a BTreeSet<String>,
    pub outside: OutsideEdit,
    pub out_of_date: bool,
    /// Current value of the reload-after-Apply setting.
    pub reload_after_apply: bool,
}

// ---------------------------------------------------------------------------------------------
// Pure helpers
// ---------------------------------------------------------------------------------------------

/// Why `draft` cannot be added as a style name right now (mirrors `StyleSet::insert_new`).
pub fn style_name_hint(state: &ConfigState, draft: &str) -> Option<String> {
    let name = draft.trim();
    if name.is_empty() {
        return Some(StyleNameError::Empty.to_string());
    }
    if name.chars().any(char::is_control) {
        return Some(StyleNameError::Invalid.to_string());
    }
    if state.style(name).is_some() {
        return Some(StyleNameError::Duplicate.to_string());
    }
    if let Some(req) = StyleSet::REQUIRED
        .iter()
        .find(|r| r.eq_ignore_ascii_case(name))
    {
        return Some(
            StyleNameError::ShadowsRequired {
                required: (*req).to_string(),
            }
            .to_string(),
        );
    }
    None
}

/// Why `draft` cannot be created as a new filter file right now.
pub fn filter_name_hint(list: &[String], draft: &str) -> Option<String> {
    if draft.trim().is_empty() {
        return Some(settings::NameProblem::Empty.to_string());
    }
    let name = settings::new_filter_name(draft);
    if let Err(problem) = settings::valid_filter_name(&name) {
        return Some(problem.to_string());
    }
    if settings::contains_name(list, &name) {
        return Some("That file already exists".to_string());
    }
    None
}

/// Built-ins render no `Delete style` at all.
pub fn delete_visible(state: &ConfigState, name: &str) -> bool {
    !state.badges(name).builtin
}

/// The text of an armed delete confirm: the shared body, plus the `config.json` warning when
/// `generate` would fail without the style.
pub fn delete_prompt(name: &str, ctx: &ConfigViewCtx) -> String {
    let users = ctx.rule_users.get(name).copied().unwrap_or(0);
    let mut text = delete_confirm_text(name, users);
    if ctx.config_refs.contains(name) {
        text.push('\n');
        text.push_str(&config_warning(name));
    }
    text
}

/// `Discard N unapplied config changes?`
pub fn revert_prompt(n: usize) -> String {
    if n == 1 {
        "Discard 1 unapplied config change?".to_string()
    } else {
        format!("Discard {n} unapplied config changes?")
    }
}

/// The footer's buttons as (label, enabled), left to right. The footer renders from this.
pub fn footer_buttons(
    state: &ConfigState,
    cfg: &ConfigUi,
    outside: OutsideEdit,
) -> Vec<(&'static str, bool)> {
    if outside != OutsideEdit::None && !cfg.outside_dismissed {
        return vec![
            ("Reload from disk", true),
            ("Overwrite", true),
            ("Keep editing", true),
        ];
    }
    if cfg.revert_armed && state.dirty_count() > 0 {
        return vec![("Discard", true), ("Keep editing", true)];
    }
    vec![
        ("Apply", state.can_apply()),
        ("Revert", state.dirty_count() > 0),
    ]
}

// ---------------------------------------------------------------------------------------------
// Small widgets
// ---------------------------------------------------------------------------------------------

fn helper(ui: &mut egui::Ui, text: &str) {
    ui.add(
        egui::Label::new(
            RichText::new(text)
                .size(theme::ROW_SUMMARY_SIZE)
                .color(theme::TAB_ACTIVE),
        )
        .wrap(),
    );
}

fn small_button(text: &str) -> egui::Button<'static> {
    egui::Button::new(RichText::new(text.to_string()).size(theme::ROW_SUMMARY_SIZE))
}

fn accent_button(text: &str) -> egui::Button<'static> {
    egui::Button::new(RichText::new(text.to_string()).size(theme::ROW_SUMMARY_SIZE))
        .fill(theme::WARM_ACCENT)
}

fn segmented(ui: &mut egui::Ui, options: &[&str], current: usize) -> Option<usize> {
    let mut picked = None;
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 4.0;
        for (i, label) in options.iter().enumerate() {
            let mut selected = i == current;
            if widgets::toggle_chip(ui, label, &mut selected).changed() && i != current {
                picked = Some(i);
            }
        }
    });
    picked
}

// ---------------------------------------------------------------------------------------------
// Sections
// ---------------------------------------------------------------------------------------------

/// FILTER, STYLES and TIERS inside their own vertical scroll area. The footer is drawn by the
/// app (pinned, before this call) so Apply stays on screen however long the lists grow.
pub fn show_sections(
    ui: &mut egui::Ui,
    state: &ConfigState,
    cfg: &mut ConfigUi,
    ctx: &ConfigViewCtx,
) -> Vec<ConfigAction> {
    let mut actions = Vec::new();
    egui::ScrollArea::vertical()
        .id_salt("ff-config-scroll")
        .auto_shrink([false, false])
        .show(ui, |ui| {
            ui.label(theme::section_header("FILTER"));
            show_filter(ui, cfg, ctx, &mut actions);
            ui.add_space(16.0);
            show_styles(ui, state, cfg, ctx, &mut actions);
            ui.add_space(16.0);
            ui.label(theme::section_header("TIERS"));
            show_tiers(ui, state, &mut actions);
        });
    actions
}

fn show_filter(
    ui: &mut egui::Ui,
    cfg: &mut ConfigUi,
    ctx: &ConfigViewCtx,
    actions: &mut Vec<ConfigAction>,
) {
    egui::Frame::new()
        .fill(theme::ROW_BASE)
        .inner_margin(egui::Margin::same(8))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.spacing_mut().item_spacing.y = 4.0;
            body::caption(ui, "GAME FOLDER FILTER");

            let list: &[String] = ctx.filters.unwrap_or(&[]);
            let shown = if ctx.filters.is_some() {
                ctx.active_filter.to_string()
            } else {
                "(game folder not found)".to_string()
            };
            let mut open_draft = false;
            let combo = ui
                .add_enabled_ui(ctx.filters.is_some(), |ui| {
                    egui::ComboBox::from_id_salt("cfg-filter")
                        .selected_text(shown)
                        .width(ui.available_width() - 8.0)
                        .show_ui(ui, |ui| {
                            for name in list {
                                let is_active = name == ctx.active_filter;
                                if ui.selectable_label(is_active, name).clicked() && !is_active {
                                    actions.push(ConfigAction::SelectFilter(name.clone()));
                                }
                            }
                            ui.separator();
                            if ui.selectable_label(false, "New empty filter\u{2026}").clicked() {
                                open_draft = true;
                            }
                        })
                })
                .inner;
            if let Some(dir) = ctx.game_dir {
                combo
                    .response
                    .on_hover_text(dir.join(ctx.active_filter).display().to_string());
            }
            if open_draft && cfg.filter_draft.is_none() {
                cfg.filter_draft = Some(String::new());
                cfg.focus_draft = true;
            }

            if let Some(draft) = cfg.filter_draft.clone() {
                filter_draft_row(ui, cfg, list, draft, actions);
            }

            if let Some(target) = ctx.pending_switch {
                egui::Frame::new()
                    .fill(theme::ROW_ALT)
                    .inner_margin(egui::Margin::same(8))
                    .show(ui, |ui| {
                        ui.set_width(ui.available_width());
                        ui.add(
                            egui::Label::new(
                                RichText::new(
                                    "You have unapplied rule changes. Switching filters will discard them.",
                                )
                                .size(theme::ROW_SUMMARY_SIZE)
                                .color(theme::VALUE_TEXT),
                            )
                            .wrap(),
                        )
                        .on_hover_text(format!("Switching to {target}"));
                        ui.horizontal(|ui| {
                            if ui.add(accent_button("Switch and discard")).clicked() {
                                actions.push(ConfigAction::ConfirmSwitch);
                            }
                            if ui.add(small_button("Stay")).clicked() {
                                actions.push(ConfigAction::CancelSwitch);
                            }
                        });
                    });
            } else {
                helper(ui, "Saved on this computer. Default: dynamic.filter.");
            }

            ui.add_space(4.0);
            let mut on = ctx.reload_after_apply;
            ui.horizontal(|ui| {
                let toggle = widgets::circular_toggle(ui, &mut on)
                    .on_hover_text(RELOAD_TOGGLE_CAPTION);
                if toggle.changed() {
                    actions.push(ConfigAction::SetReloadAfterApply(on));
                }
                ui.label(
                    RichText::new(RELOAD_TOGGLE_LABEL)
                        .size(theme::ROW_SUMMARY_SIZE)
                        .color(theme::VALUE_TEXT),
                );
            });
            helper(ui, RELOAD_TOGGLE_CAPTION);
        });
}

fn filter_draft_row(
    ui: &mut egui::Ui,
    cfg: &mut ConfigUi,
    list: &[String],
    mut draft: String,
    actions: &mut Vec<ConfigAction>,
) {
    let hint = filter_name_hint(list, &draft);
    let mut commit = false;
    let mut discard = false;
    let edit = ui.add(
        egui::TextEdit::singleline(&mut draft)
            .id_salt("cfg-filter-draft")
            .hint_text("my-filter.filter")
            .desired_width(ui.available_width() - 8.0),
    );
    if std::mem::take(&mut cfg.focus_draft) {
        edit.request_focus();
    }
    if edit.lost_focus() {
        if ui.input(|i| i.key_pressed(egui::Key::Enter)) {
            commit = hint.is_none();
        } else if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
            discard = true;
        }
    }
    if let Some(msg) = &hint
        && !draft.trim().is_empty()
    {
        body::hint(ui, msg);
    } else if let Some(msg) = &hint {
        helper(ui, msg);
    }
    ui.horizontal(|ui| {
        if ui
            .add_enabled(hint.is_none(), small_button("Create filter"))
            .clicked()
        {
            commit = true;
        }
        if ui.add(small_button("Discard new filter")).clicked() {
            discard = true;
        }
    });
    if commit && hint.is_none() {
        actions.push(ConfigAction::CreateFilter(settings::new_filter_name(
            &draft,
        )));
        // The draft stays until the handler accepts the name, so a rejected name is not lost.
        cfg.filter_draft = Some(draft);
    } else if discard {
        cfg.filter_draft = None;
    } else {
        cfg.filter_draft = Some(draft);
    }
}

fn show_tiers(ui: &mut egui::Ui, state: &ConfigState, actions: &mut Vec<ConfigAction>) {
    helper(
        ui,
        "Minimum price for each tier. Items worth at least this much get the tier.",
    );
    ui.horizontal(|ui| {
        // Same 8px inset as the zebra rows below, so captions sit over their columns.
        ui.add_space(8.0);
        ui.add_sized(vec2(24.0, 14.0), egui::Label::new(caption_text("TIER")));
        ui.add_sized(
            vec2(112.0, 14.0),
            egui::Label::new(caption_text("MIN VALUE")),
        );
        ui.add_sized(vec2(64.0, 14.0), egui::Label::new(caption_text("UNIT")));
    });
    for (i, row) in state.tier_rows().iter().enumerate() {
        let mut stroke_rect = None;
        widgets::zebra_row(ui, i, |ui| {
            ui.horizontal(|ui| {
                ui.set_min_height(24.0);
                ui.add_sized(
                    vec2(24.0, 24.0),
                    egui::Label::new(RichText::new(&row.tier).size(14.0).color(theme::VALUE_TEXT)),
                );
                let id = ui.make_persistent_id(("cfg-tier", &row.tier));
                let mut buf = row.buffer.clone();
                let edit = ui.add(
                    egui::TextEdit::singleline(&mut buf)
                        .id(id)
                        .desired_width(112.0),
                );
                if edit.changed() {
                    actions.push(ConfigAction::TierText {
                        tier: row.tier.clone(),
                        text: buf,
                    });
                }
                ui.add_sized(
                    vec2(64.0, 24.0),
                    egui::Label::new(
                        RichText::new(row.unit)
                            .size(theme::ROW_SUMMARY_SIZE)
                            .color(theme::DISABLED_TEXT),
                    )
                    .sense(egui::Sense::hover()),
                )
                .on_hover_text(UNIT_TIP);
            });
            if let Some(err) = &row.error {
                ui.add(
                    egui::Label::new(
                        RichText::new(err)
                            .size(theme::ROW_SUMMARY_SIZE)
                            .color(theme::WARM_ACCENT_TEXT),
                    )
                    .wrap(),
                );
            }
            if row.edge {
                stroke_rect = Some(ui.min_rect());
            }
        });
        if let Some(r) = stroke_rect {
            let bar = egui::Rect::from_min_max(
                egui::pos2(r.left() - 8.0, r.top() - 4.0),
                egui::pos2(r.left() - 4.0, r.bottom() + 4.0),
            );
            ui.painter().rect_filled(bar, 0.0, theme::WARM_ACCENT);
        }
    }
    ui.add_space(4.0);
    helper(
        ui,
        "Only your live tiers.json is edited. The built-in early-league fallback is never changed.",
    );
}

fn caption_text(text: &str) -> RichText {
    RichText::new(text).small().color(theme::LABEL_TAN)
}

fn show_styles(
    ui: &mut egui::Ui,
    state: &ConfigState,
    cfg: &mut ConfigUi,
    ctx: &ConfigViewCtx,
    actions: &mut Vec<ConfigAction>,
) {
    ui.horizontal(|ui| {
        ui.label(theme::section_header("STYLES"));
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            if ui.add(small_button("+ ADD STYLE")).clicked() && cfg.style_draft.is_none() {
                cfg.style_draft = Some(String::new());
                cfg.focus_draft = true;
            }
        });
    });
    helper(
        ui,
        "Named looks that rules can share. Changes reach rules on the RULES tab's Apply.",
    );

    if let Some(draft) = cfg.style_draft.clone() {
        style_draft_row(ui, state, cfg, draft, actions);
    }

    let names = state.style_names();
    if names.is_empty() {
        helper(ui, "No styles loaded. Revert to reload styles.json.");
        return;
    }
    let epoch = cfg.epoch;
    for (i, name) in names.iter().enumerate() {
        let Some(style) = state.style(name) else {
            continue;
        };
        ui.push_id(("style", name.as_str(), epoch), |ui| {
            widgets::zebra_row(ui, i, |ui| {
                let is_expanded = cfg.expanded.as_deref() == Some(name.as_str());
                let badges = state.badges(name);
                let header = ui.horizontal(|ui| {
                    ui.set_min_height(24.0);
                    ui.add_sized(
                        vec2(96.0, 20.0),
                        egui::Label::new(RichText::new(name).size(14.0).color(theme::VALUE_TEXT))
                            .truncate()
                            .sense(egui::Sense::hover()),
                    )
                    .on_hover_text(name);
                    widgets::style_preview_chip(ui, name, style);
                    ui.spacing_mut().item_spacing.x = 4.0;
                    if badges.hide {
                        widgets::badge(ui, "Hide", "Hidden in game; Alt reveals it.");
                    }
                    if badges.builtin {
                        widgets::badge(ui, "built-in", BUILTIN_TIP);
                    }
                    if badges.new {
                        widgets::badge(ui, "new", "Added this session; not applied yet.");
                    }
                    if badges.edited {
                        widgets::badge(ui, "edited", "Changed since the last Apply.");
                    }
                });
                let row = ui
                    .interact(
                        header.response.rect,
                        ui.id().with(("style-row", name.as_str())),
                        egui::Sense::click(),
                    )
                    .on_hover_cursor(egui::CursorIcon::PointingHand);
                row.widget_info(|| {
                    egui::WidgetInfo::selected(
                        egui::WidgetType::CollapsingHeader,
                        true,
                        is_expanded,
                        name.as_str(),
                    )
                });
                if cfg.scroll_to.as_deref() == Some(name.as_str()) {
                    row.scroll_to_me(Some(Align::Center));
                    cfg.scroll_to = None;
                }
                if row.clicked() {
                    cfg.set_expanded(name);
                    egui::Popup::close_all(ui.ctx());
                }
                if is_expanded {
                    ui.add_space(4.0);
                    egui::Frame::new()
                        .fill(theme::ROW_BASE)
                        .inner_margin(egui::Margin::same(8))
                        .show(ui, |ui| {
                            ui.set_width(ui.available_width());
                            style_editor(ui, state, cfg, ctx, name, style, actions);
                        });
                }
            });
        });
    }
}

fn style_draft_row(
    ui: &mut egui::Ui,
    state: &ConfigState,
    cfg: &mut ConfigUi,
    mut draft: String,
    actions: &mut Vec<ConfigAction>,
) {
    let hint = style_name_hint(state, &draft);
    let mut commit = false;
    let mut discard = false;
    egui::Frame::new()
        .fill(theme::ROW_BASE)
        .inner_margin(egui::Margin::same(8))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.spacing_mut().item_spacing.y = 4.0;
            let edit = ui.add(
                egui::TextEdit::singleline(&mut draft)
                    .id_salt("cfg-style-draft")
                    .hint_text("Style name")
                    .desired_width(ui.available_width() - 8.0),
            );
            if std::mem::take(&mut cfg.focus_draft) {
                edit.request_focus();
            }
            if edit.lost_focus() {
                if ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                    commit = hint.is_none();
                } else if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                    discard = true;
                }
            }
            if let Some(msg) = &hint {
                helper(ui, msg);
            }
            ui.horizontal(|ui| {
                if ui
                    .add_enabled(hint.is_none(), small_button("Add style"))
                    .clicked()
                {
                    commit = true;
                }
                if ui.add(small_button("Discard new style")).clicked() {
                    discard = true;
                }
            });
        });
    if commit && hint.is_none() {
        let name = draft.trim().to_string();
        actions.push(ConfigAction::AddStyle(name.clone()));
        cfg.expanded = Some(name.clone());
        cfg.scroll_to = Some(name);
        cfg.style_draft = None;
    } else if discard {
        cfg.style_draft = None;
    } else {
        cfg.style_draft = Some(draft);
    }
}

fn edit(actions: &mut Vec<ConfigAction>, name: &str, edit: StyleEdit) {
    actions.push(ConfigAction::EditStyle {
        name: name.to_string(),
        edit,
    });
}

fn style_editor(
    ui: &mut egui::Ui,
    state: &ConfigState,
    cfg: &mut ConfigUi,
    ctx: &ConfigViewCtx,
    name: &str,
    style: &Style,
    actions: &mut Vec<ConfigAction>,
) {
    ui.spacing_mut().item_spacing.y = 8.0;
    let salt = |field: &'static str| ("cfg-style", name.to_string(), field);

    // Name (read-only).
    ui.vertical(|ui| {
        body::caption(ui, "NAME");
        ui.add(
            egui::Label::new(RichText::new(name).size(14.0).color(theme::VALUE_TEXT))
                .sense(egui::Sense::hover()),
        )
        .on_hover_text(RENAME_TIP);
    });

    // Action.
    ui.vertical(|ui| {
        body::caption(ui, "ACTION");
        let current = usize::from(style.action == StyleAction::Hide);
        if let Some(pick) = segmented(ui, &["Show", "Hide"], current) {
            let action = if pick == 0 {
                StyleAction::Show
            } else {
                StyleAction::Hide
            };
            edit(actions, name, StyleEdit::Action(action));
        }
    });

    // Colours.
    for (label, slot, current, seed) in [
        ("TEXT", ColourSlot::Text, style.text, [248, 248, 242]),
        ("BORDER", ColourSlot::Border, style.border, [248, 248, 242]),
        (
            "BACKGROUND",
            ColourSlot::Background,
            style.background,
            [40, 42, 54],
        ),
    ] {
        ui.horizontal(|ui| {
            body::caption(ui, label);
            let mut on = current.is_some();
            if widgets::circular_toggle(ui, &mut on).changed() {
                edit(
                    actions,
                    name,
                    StyleEdit::Colour {
                        slot,
                        value: on.then_some(seed),
                    },
                );
            }
            match current {
                Some(rgb) => {
                    if let Some(new) = widgets::colour_swatch_rgb(ui, rgb) {
                        edit(
                            actions,
                            name,
                            StyleEdit::Colour {
                                slot,
                                value: Some(new),
                            },
                        );
                    }
                }
                None => body::caption(ui, NOT_SET),
            }
        });
    }

    // Font size.
    ui.vertical(|ui| {
        body::caption(ui, "FONT SIZE");
        ui.horizontal(|ui| {
            let mut on = style.font.is_some();
            if widgets::circular_toggle(ui, &mut on).changed() {
                edit(actions, name, StyleEdit::Font(on.then_some(32)));
            }
            if let Some(font) = style.font {
                let mut invalid = false;
                let (new, _) =
                    body::slider_pair(ui, salt("font"), i64::from(font), (1, 45), &mut invalid);
                if let Some(v) = new {
                    edit(actions, name, StyleEdit::Font(Some(v)));
                }
                if invalid {
                    body::hint(ui, WHOLE_NUMBER);
                }
            } else {
                body::caption(ui, NOT_SET);
            }
        });
        if let Some(font) = style.font
            && let Some(msg) = body::font_hint(i64::from(font))
        {
            helper(ui, msg);
        }
    });

    sound_editor(ui, name, style, actions);

    // Drop sound.
    ui.horizontal(|ui| {
        let mut on = style.disable_drop_sound;
        if widgets::circular_toggle(ui, &mut on).changed() {
            edit(actions, name, StyleEdit::DisableDropSound(on));
        }
        ui.label(
            RichText::new("Silence the default drop sound")
                .size(theme::ROW_SUMMARY_SIZE)
                .color(theme::LABEL_TAN),
        );
    });

    minimap_editor(ui, name, style, actions);
    beam_editor(ui, name, style, actions);

    if let Some(err) = state.styles_error() {
        ui.add(
            egui::Label::new(
                RichText::new(err)
                    .size(theme::ROW_SUMMARY_SIZE)
                    .color(theme::WARM_ACCENT_TEXT),
            )
            .wrap(),
        );
    }

    // Delete.
    if delete_visible(state, name) {
        if cfg.delete_armed.as_deref() == Some(name) {
            egui::Frame::new()
                .fill(theme::ROW_ALT)
                .inner_margin(egui::Margin::same(8))
                .show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    ui.add(
                        egui::Label::new(
                            RichText::new(delete_prompt(name, ctx))
                                .size(theme::ROW_SUMMARY_SIZE)
                                .color(theme::VALUE_TEXT),
                        )
                        .wrap(),
                    );
                    ui.horizontal(|ui| {
                        if ui.add(accent_button("Delete style")).clicked() {
                            actions.push(ConfigAction::DeleteStyle(name.to_string()));
                            cfg.delete_armed = None;
                        }
                        if ui.add(small_button("Keep style")).clicked() {
                            cfg.delete_armed = None;
                        }
                    });
                });
        } else {
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if ui.add(small_button("Delete style")).clicked() {
                    cfg.arm_delete(name);
                }
            });
        }
    }
}

fn sound_editor(ui: &mut egui::Ui, name: &str, style: &Style, actions: &mut Vec<ConfigAction>) {
    let salt = |field: &'static str| ("cfg-style", name.to_string(), field);
    ui.vertical(|ui| {
        body::caption(ui, "SOUND");
        let mode = match &style.sound {
            None => 0,
            Some(s) if s.custom.is_some() => 2,
            Some(_) => 1,
        };
        if let Some(pick) = segmented(ui, &["None", "Alert", "Custom"], mode)
            && pick != mode
        {
            let next = match pick {
                0 => None,
                1 => Some(Sound {
                    id: Some(1),
                    custom: None,
                    vol: 100,
                }),
                _ => Some(Sound {
                    id: None,
                    custom: Some("uwu.mp3".to_string()),
                    vol: 100,
                }),
            };
            edit(actions, name, StyleEdit::Sound(next));
        }
        let Some(sound) = &style.sound else {
            return;
        };
        let mut invalid = false;
        if let Some(file) = &sound.custom {
            let mut buf = file.clone();
            ui.horizontal(|ui| {
                body::caption(ui, "File");
                let resp = ui.add(
                    egui::TextEdit::singleline(&mut buf)
                        .id_salt(salt("file"))
                        .hint_text("uwu.mp3")
                        .desired_width(160.0),
                );
                if resp.changed() {
                    edit(
                        actions,
                        name,
                        StyleEdit::Sound(Some(Sound {
                            id: None,
                            custom: Some(buf.clone()),
                            vol: sound.vol,
                        })),
                    );
                }
            });
            body::caption(ui, "The file must be in the game folder.");
        } else {
            let id = sound.id.unwrap_or(1);
            ui.horizontal(|ui| {
                body::caption(ui, "ID");
                let (new, _) =
                    body::slider_pair(ui, salt("sid"), i64::from(id), (1, 16), &mut invalid);
                // Values that do not fit the stored type are dropped, never wrapped.
                if let Some(v) = new.and_then(|v| u8::try_from(v).ok()) {
                    edit(
                        actions,
                        name,
                        StyleEdit::Sound(Some(Sound {
                            id: Some(v),
                            custom: None,
                            vol: sound.vol,
                        })),
                    );
                }
            });
        }
        ui.horizontal(|ui| {
            body::caption(ui, "Volume");
            let (new, _) = body::slider_pair(
                ui,
                salt("svol"),
                i64::from(sound.vol),
                (0, 300),
                &mut invalid,
            );
            if let Some(v) = new.and_then(|v| u16::try_from(v).ok()) {
                edit(
                    actions,
                    name,
                    StyleEdit::Sound(Some(Sound {
                        id: sound.id,
                        custom: sound.custom.clone(),
                        vol: v,
                    })),
                );
            }
        });
        if invalid {
            body::hint(ui, WHOLE_NUMBER);
        }
    });
}

fn minimap_editor(ui: &mut egui::Ui, name: &str, style: &Style, actions: &mut Vec<ConfigAction>) {
    let salt = |field: &'static str| ("cfg-style", name.to_string(), field);
    ui.vertical(|ui| {
        body::caption(ui, "MINIMAP ICON");
        ui.horizontal(|ui| {
            let mut on = style.minimap.is_some();
            if widgets::circular_toggle(ui, &mut on).changed() {
                let seed = on.then_some(Minimap {
                    size: 2,
                    color: FilterColor::White,
                    shape: MinimapShape::Circle,
                });
                edit(actions, name, StyleEdit::Minimap(seed));
            }
            if style.minimap.is_none() {
                body::caption(ui, NOT_SET);
            }
        });
        let Some(m) = &style.minimap else {
            return;
        };
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 8.0;
            let size_label = body::SIZE_CHOICES
                .iter()
                .find(|(_, v)| *v == i64::from(m.size))
                .map_or_else(|| m.size.to_string(), |(l, _)| (*l).to_string());
            egui::ComboBox::from_id_salt(salt("msize"))
                .selected_text(size_label)
                .width(88.0)
                .show_ui(ui, |ui| {
                    for (label, v) in body::SIZE_CHOICES.iter().filter(|(_, v)| *v >= 0) {
                        let selected = *v == i64::from(m.size);
                        if ui.selectable_label(selected, *label).clicked()
                            && !selected
                            && let Ok(size) = u8::try_from(*v)
                        {
                            let mut next = m.clone();
                            next.size = size;
                            edit(actions, name, StyleEdit::Minimap(Some(next)));
                        }
                    }
                });
            if let Some(c) = body::colour_combo(ui, salt("mcol"), m.color.as_str(), false)
                && let Some(color) = FilterColor::from_name(&c)
            {
                let mut next = m.clone();
                next.color = color;
                edit(actions, name, StyleEdit::Minimap(Some(next)));
            }
            egui::ComboBox::from_id_salt(salt("mshape"))
                .selected_text(m.shape.as_str())
                .width(104.0)
                .show_ui(ui, |ui| {
                    for shape in MinimapShape::ALL {
                        let selected = shape == m.shape;
                        if ui.selectable_label(selected, shape.as_str()).clicked() && !selected {
                            let mut next = m.clone();
                            next.shape = shape;
                            edit(actions, name, StyleEdit::Minimap(Some(next)));
                        }
                    }
                });
        });
    });
}

fn beam_editor(ui: &mut egui::Ui, name: &str, style: &Style, actions: &mut Vec<ConfigAction>) {
    let salt = |field: &'static str| ("cfg-style", name.to_string(), field);
    ui.vertical(|ui| {
        body::caption(ui, "BEAM");
        ui.horizontal(|ui| {
            let mut on = style.beam.is_some();
            if widgets::circular_toggle(ui, &mut on).changed() {
                let seed = on.then_some(Beam {
                    color: FilterColor::White,
                    temp: false,
                });
                edit(actions, name, StyleEdit::Beam(seed));
            }
            match &style.beam {
                None => body::caption(ui, NOT_SET),
                Some(b) => {
                    if let Some(c) = body::colour_combo(ui, salt("bcol"), b.color.as_str(), false)
                        && let Some(color) = FilterColor::from_name(&c)
                    {
                        let mut next = b.clone();
                        next.color = color;
                        edit(actions, name, StyleEdit::Beam(Some(next)));
                    }
                    let mut temp = b.temp;
                    if widgets::circular_toggle(ui, &mut temp).changed() {
                        let mut next = b.clone();
                        next.temp = temp;
                        edit(actions, name, StyleEdit::Beam(Some(next)));
                    }
                    body::caption(ui, "Temporary");
                }
            }
        });
    });
}

// ---------------------------------------------------------------------------------------------
// Status, footer, load error
// ---------------------------------------------------------------------------------------------

/// The status block above the scroll area: dirty count, plus the out-of-date notice.
pub fn show_status(ui: &mut egui::Ui, state: &ConfigState, ctx: &ConfigViewCtx) {
    ui.label(
        RichText::new(dirty_status(state.dirty_count()))
            .size(theme::ROW_SUMMARY_SIZE)
            .color(theme::TAB_ACTIVE),
    );
    if ctx.out_of_date {
        ui.add(
            egui::Label::new(
                RichText::new(OUT_OF_DATE_MESSAGE)
                    .size(14.0)
                    .color(theme::VALUE_TEXT),
            )
            .wrap(),
        );
    }
}

/// The pinned button row. The app draws the outcome area above it.
pub fn show_footer(
    ui: &mut egui::Ui,
    state: &ConfigState,
    cfg: &mut ConfigUi,
    ctx: &ConfigViewCtx,
) -> Vec<ConfigAction> {
    let mut actions = Vec::new();
    let guard = ctx.outside != OutsideEdit::None && !cfg.outside_dismissed;
    // A revert confirm cannot outlive its edits.
    if state.dirty_count() == 0 {
        cfg.revert_armed = false;
    }

    let revert_prompt_open = !guard && cfg.revert_armed && state.dirty_count() > 0;
    if guard {
        ui.add(
            egui::Label::new(
                RichText::new(outside_edit_message(ctx.outside))
                    .size(theme::ROW_SUMMARY_SIZE)
                    .color(theme::WARM_ACCENT_TEXT),
            )
            .wrap(),
        );
    } else if revert_prompt_open {
        ui.label(
            RichText::new(revert_prompt(state.dirty_count()))
                .size(theme::ROW_SUMMARY_SIZE)
                .color(theme::VALUE_TEXT),
        );
    }

    let buttons = footer_buttons(state, cfg, ctx.outside);
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 8.0;
        for (label, enabled) in buttons {
            let accent = matches!(label, "Overwrite" | "Discard");
            let text = RichText::new(label);
            let mut button = egui::Button::new(text).min_size(vec2(0.0, 28.0));
            if accent {
                button = button.fill(theme::WARM_ACCENT);
            }
            let mut response = ui.add_enabled(enabled, button);
            if label == "Apply" && !enabled {
                let tip = if state.has_tier_errors() {
                    APPLY_BLOCKED_TOOLTIP
                } else if state.styles_error().is_some() || state.tiers_error().is_some() {
                    APPLY_LOAD_ERROR_TIP
                } else {
                    APPLY_NOTHING_TIP
                };
                response = response.on_disabled_hover_text(tip);
            }
            if !response.clicked() {
                continue;
            }
            match label {
                "Apply" => actions.push(ConfigAction::Apply),
                "Revert" => cfg.revert_armed = true,
                "Discard" => {
                    cfg.revert_armed = false;
                    actions.push(ConfigAction::Revert);
                }
                "Reload from disk" => actions.push(ConfigAction::Reload),
                "Overwrite" => actions.push(ConfigAction::Overwrite),
                "Keep editing" => {
                    cfg.revert_armed = false;
                    cfg.outside_dismissed = true;
                }
                _ => {}
            }
        }
    });
    actions
}

/// The load-failure view: the message and a way to retry.
pub fn show_load_error(ui: &mut egui::Ui, message: &str) -> Option<ConfigAction> {
    ui.add(
        egui::Label::new(
            RichText::new(message)
                .size(theme::ROW_SUMMARY_SIZE)
                .color(theme::WARM_ACCENT_TEXT),
        )
        .wrap(),
    );
    ui.add(small_button("Reload from disk"))
        .clicked()
        .then_some(ConfigAction::Reload)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn temp_root() -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        for f in ["styles.json", "tiers.json"] {
            fs::copy(
                Path::new(env!("CARGO_MANIFEST_DIR")).join(f),
                dir.path().join(f),
            )
            .unwrap();
        }
        dir
    }

    fn loaded() -> (tempfile::TempDir, ConfigState) {
        let dir = temp_root();
        let state = ConfigState::load(dir.path()).unwrap();
        (dir, state)
    }

    fn headless() -> (egui::Context, egui::RawInput) {
        let ctx = egui::Context::default();
        theme::install_fonts(&ctx);
        let input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                vec2(460.0, 600.0),
            )),
            ..Default::default()
        };
        (ctx, input)
    }

    fn view_ctx<'a>(
        filters: Option<&'a [String]>,
        users: &'a HashMap<String, usize>,
        refs: &'a BTreeSet<String>,
    ) -> ConfigViewCtx<'a> {
        ConfigViewCtx {
            filters,
            active_filter: "dynamic.filter",
            game_dir: None,
            pending_switch: None,
            rule_users: users,
            config_refs: refs,
            outside: OutsideEdit::None,
            out_of_date: false,
            reload_after_apply: true,
        }
    }

    #[test]
    fn reload_toggle_text_is_fixed_ascii_and_renders_quietly() {
        assert_eq!(RELOAD_TOGGLE_LABEL, "Reload filter in game after Apply");
        assert!(RELOAD_TOGGLE_CAPTION.contains("/reloaditemfilter"));
        assert!(RELOAD_TOGGLE_CAPTION.contains("selected in game"));
        assert!(RELOAD_TOGGLE_CAPTION.is_ascii() && RELOAD_TOGGLE_LABEL.is_ascii());
        let (_d, state) = loaded();
        let users = HashMap::new();
        let refs = BTreeSet::new();
        let filters = vec!["dynamic.filter".to_string()];
        for list in [Some(filters.as_slice()), None] {
            for on in [true, false] {
                let (ctx, input) = headless();
                let mut cfg = ConfigUi::default();
                let mut vctx = view_ctx(list, &users, &refs);
                vctx.reload_after_apply = on;
                let mut actions = None;
                let out = ctx.run_ui(input, |ui| {
                    actions = Some(show_sections(ui, &state, &mut cfg, &vctx));
                });
                out.drop_without_applying_deltas();
                assert_eq!(actions, Some(vec![]));
            }
        }
    }

    #[test]
    fn style_name_hints_follow_insert_new() {
        let (_d, state) = loaded();
        assert_eq!(
            style_name_hint(&state, "").as_deref(),
            Some("Enter a style name")
        );
        assert_eq!(
            style_name_hint(&state, "  ").as_deref(),
            Some("Enter a style name")
        );
        assert_eq!(
            style_name_hint(&state, "S").as_deref(),
            Some("A style with this name already exists")
        );
        assert_eq!(style_name_hint(&state, "Gold"), None);
        assert_eq!(
            style_name_hint(&state, "hIdDeN").as_deref(),
            Some("That name is reserved for the built-in style hidden")
        );
    }

    #[test]
    fn filter_name_hints() {
        let list = vec!["dynamic.filter".to_string()];
        assert_eq!(
            filter_name_hint(&list, "").as_deref(),
            Some("Enter a new file name")
        );
        assert_eq!(
            filter_name_hint(&list, "DYNAMIC").as_deref(),
            Some("That file already exists")
        );
        assert_eq!(
            filter_name_hint(&list, "a/b").as_deref(),
            Some("Enter a file name, not a folder path")
        );
        assert_eq!(filter_name_hint(&list, "ssf"), None);
    }

    #[test]
    fn expanding_another_row_disarms_the_delete_and_bumps_the_epoch() {
        let mut cfg = ConfigUi::default();
        cfg.set_expanded("S");
        cfg.arm_delete("S");
        let epoch = cfg.epoch;
        cfg.set_expanded("kite");
        assert_eq!(cfg.expanded.as_deref(), Some("kite"));
        assert_eq!(cfg.delete_armed, None);
        assert_ne!(cfg.epoch, epoch);
        cfg.set_expanded("kite");
        assert_eq!(cfg.expanded, None, "same name toggles it closed");
    }

    #[test]
    fn idle_frames_emit_no_actions_for_any_expanded_style() {
        let (_d, state) = loaded();
        let users = HashMap::new();
        let refs = BTreeSet::new();
        let filters = vec!["dynamic.filter".to_string(), "ssf.filter".to_string()];
        let (ctx, input) = headless();
        for expanded in [
            None,
            Some("S"),
            Some("unchanged"),
            Some("hidden"),
            Some("kite"),
        ] {
            let mut cfg = ConfigUi {
                expanded: expanded.map(str::to_string),
                ..Default::default()
            };
            let vctx = view_ctx(Some(&filters), &users, &refs);
            let mut actions = None;
            let out = ctx.run_ui(input.clone(), |ui| {
                actions = Some(show_sections(ui, &state, &mut cfg, &vctx));
            });
            out.drop_without_applying_deltas();
            assert_eq!(actions, Some(vec![]), "expanded {expanded:?}");
        }
    }

    #[test]
    fn a_missing_game_folder_renders_without_panicking() {
        let (_d, state) = loaded();
        let users = HashMap::new();
        let refs = BTreeSet::new();
        let (ctx, input) = headless();
        let mut cfg = ConfigUi::default();
        let vctx = view_ctx(None, &users, &refs);
        let out = ctx.run_ui(input, |ui| {
            show_sections(ui, &state, &mut cfg, &vctx);
        });
        out.drop_without_applying_deltas();
    }

    #[test]
    fn open_drafts_and_a_switch_guard_render() {
        let (_d, state) = loaded();
        let users = HashMap::new();
        let refs = BTreeSet::new();
        let filters = vec!["dynamic.filter".to_string()];
        let (ctx, input) = headless();
        let mut cfg = ConfigUi {
            style_draft: Some("Gold".to_string()),
            filter_draft: Some("ssf".to_string()),
            ..Default::default()
        };
        let mut vctx = view_ctx(Some(&filters), &users, &refs);
        vctx.pending_switch = Some("ssf.filter");
        let mut actions = None;
        let out = ctx.run_ui(input, |ui| {
            actions = Some(show_sections(ui, &state, &mut cfg, &vctx));
        });
        out.drop_without_applying_deltas();
        assert_eq!(actions, Some(vec![]));
        assert_eq!(cfg.style_draft.as_deref(), Some("Gold"));
        assert_eq!(cfg.filter_draft.as_deref(), Some("ssf"));
    }

    #[test]
    fn built_ins_have_no_delete_and_others_do() {
        let (_d, state) = loaded();
        for builtin in ["S", "A", "B", "C", "D", "E", "F", "unchanged", "hidden"] {
            assert!(!delete_visible(&state, builtin), "{builtin}");
        }
        assert!(delete_visible(&state, "kite"));
    }

    #[test]
    fn the_delete_prompt_adds_the_config_warning() {
        let users: HashMap<String, usize> = [("kite".to_string(), 2)].into();
        let refs: BTreeSet<String> = ["kite".to_string()].into();
        let ctx = view_ctx(None, &users, &refs);
        assert_eq!(
            delete_prompt("kite", &ctx),
            format!(
                "{}\n{}",
                delete_confirm_text("kite", 2),
                config_warning("kite")
            )
        );
        let no_refs = BTreeSet::new();
        let plain = view_ctx(None, &users, &no_refs);
        assert_eq!(
            delete_prompt("kite", &plain),
            delete_confirm_text("kite", 2)
        );
    }

    #[test]
    fn footer_buttons_follow_the_state() {
        let (_d, mut state) = loaded();
        let mut cfg = ConfigUi::default();
        assert_eq!(
            footer_buttons(&state, &cfg, OutsideEdit::None),
            vec![("Apply", false), ("Revert", false)]
        );

        state.set_tier_text("S", "oops");
        assert_eq!(
            footer_buttons(&state, &cfg, OutsideEdit::None),
            vec![("Apply", false), ("Revert", true)]
        );
        assert!(state.has_tier_errors());

        state.set_tier_text("S", "31");
        assert_eq!(
            footer_buttons(&state, &cfg, OutsideEdit::None)[0],
            ("Apply", true)
        );

        assert_eq!(
            footer_buttons(&state, &cfg, OutsideEdit::Tiers),
            vec![
                ("Reload from disk", true),
                ("Overwrite", true),
                ("Keep editing", true)
            ]
        );

        cfg.revert_armed = true;
        assert_eq!(
            footer_buttons(&state, &cfg, OutsideEdit::None),
            vec![("Discard", true), ("Keep editing", true)]
        );
        assert_eq!(revert_prompt(3), "Discard 3 unapplied config changes?");
    }

    #[test]
    fn status_lines_cover_dirty_and_out_of_date() {
        let (_d, state) = loaded();
        let users = HashMap::new();
        let refs = BTreeSet::new();
        let (ctx, input) = headless();
        let mut vctx = view_ctx(None, &users, &refs);
        vctx.out_of_date = true;
        let out = ctx.run_ui(input, |ui| show_status(ui, &state, &vctx));
        out.drop_without_applying_deltas();
        assert_eq!(
            dirty_status(state.dirty_count()),
            "Config: no unapplied changes"
        );
    }

    #[test]
    fn the_footer_apply_button_stays_inside_the_stripe() {
        let (_d, state) = loaded();
        let users = HashMap::new();
        let refs = BTreeSet::new();
        let (ctx, input) = headless();
        let mut cfg = ConfigUi::default();
        let vctx = view_ctx(None, &users, &refs);
        let screen = input.screen_rect.unwrap();
        let mut footer_rect = egui::Rect::NOTHING;
        let out = ctx.run_ui(input, |ui| {
            egui::CentralPanel::default().show(ui, |ui| {
                egui::Panel::bottom(egui::Id::new("cfg-test-footer")).show(ui, |ui| {
                    show_footer(ui, &state, &mut cfg, &vctx);
                    footer_rect = ui.min_rect();
                });
                show_sections(ui, &state, &mut cfg, &vctx);
            });
        });
        out.drop_without_applying_deltas();
        assert!(
            footer_rect.max.y <= screen.max.y,
            "footer {footer_rect:?} outside {screen:?}"
        );
        assert!(footer_rect.height() >= 28.0);
    }

    #[test]
    fn load_error_view_renders() {
        let (ctx, input) = headless();
        let mut action = None;
        let out = ctx.run_ui(input, |ui| {
            action = show_load_error(ui, "styles.json: boom");
        });
        out.drop_without_applying_deltas();
        assert_eq!(action, None);
    }
}
