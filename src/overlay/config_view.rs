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

use super::apply;
use super::body;
use super::config_state::{
    APPLY_BLOCKED_TOOLTIP, ColourSlot, ConfigState, StyleEdit, config_warning, delete_confirm_text,
};
use super::theme;
use super::widgets::{self, ArrowDirection};

const WHOLE_NUMBER: &str = "Enter a whole number";
const NOT_SET: &str = "Not set (game default)";
const SWITCH_TIP: &str = "On: shown. Off: hidden (Alt reveals).";
/// Fixed widths of the styles table columns (Visibility | Name | Preview), shared by the header
/// and every row so they stay aligned across the Built-in and Custom groups.
const COL_VISIBILITY: f32 = 48.0;
const COL_NAME: f32 = 104.0;
const COL_PREVIEW: f32 = 120.0;
const RENAME_TIP: &str = "Names can't be changed. Add a new style and delete this one.";
const UNIT_TIP: &str = "Set in tiers.json; not editable here.";
/// Label of the FILTER section toggle that sends `/reloaditemfilter` after an Apply.
pub const RELOAD_TOGGLE_LABEL: &str = "Reload filter in game after Apply";
/// Caveats shown under the toggle: the game reloads whichever filter is selected in game, and text
/// left in an open chat box is sent first (nothing outside the game can see whether chat is open).
pub const RELOAD_TOGGLE_CAPTION: &str = "Sends /reloaditemfilter to the game after Apply. It reloads the filter selected in game, so select this filter there too. Close the chat box before pressing Apply: text left in an open chat box is sent as a message instead of the command.";
/// The toggle's hover text: what "Reloaded in game" does and does not certify (review WR-01).
pub const RELOAD_TOGGLE_TIP: &str = "\"Reloaded in game\" means the command was sent to the game window; ff cannot see whether the game ran it. A game running as administrator ignores the keys unless ff runs as administrator too. If the game did not reload, use Options > Game > Item Filter > Refresh.";
const APPLY_NOTHING_TIP: &str = "Nothing to apply.";
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
    EditStyle {
        name: String,
        edit: StyleEdit,
    },
    DeleteStyle(String),
    /// Move a custom style one place up or down among the customs.
    MoveStyle {
        name: String,
        up: bool,
    },
    TierText {
        tier: String,
        text: String,
    },
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
    /// Styles, tiers or the filter changed on disk since they were loaded (the shared refusal).
    pub changed_on_disk: bool,
    /// Whether the combined APPLY is live.
    pub apply_enabled: bool,
    /// Enabled rules have grammar errors (explains a disabled APPLY).
    pub rules_blocking: bool,
    /// The filter's economy markers are damaged (explains a disabled APPLY).
    pub filter_damaged: bool,
    /// The one Apply state line both tabs show.
    pub indicator: &'a str,
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

/// The footer's buttons as (label, enabled), left to right. The footer renders from this. After a
/// changed-on-disk refusal only RELOAD and OVERWRITE remain, the same two as the RULES footer.
pub fn footer_buttons(
    state: &ConfigState,
    cfg: &ConfigUi,
    changed_on_disk: bool,
    apply_enabled: bool,
) -> Vec<(&'static str, bool)> {
    if changed_on_disk {
        return vec![("RELOAD", true), ("OVERWRITE", true)];
    }
    if cfg.revert_armed && state.is_dirty() {
        return vec![("Discard", true), ("Cancel", true)];
    }
    vec![("Apply", apply_enabled), ("Revert", state.is_dirty())]
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
                let toggle =
                    widgets::circular_toggle(ui, &mut on).on_hover_text(RELOAD_TOGGLE_TIP);
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

// Layout probe for tests: records where a named element was drawn. A no-op outside tests.
#[cfg(test)]
thread_local! {
    static PROBE: std::cell::RefCell<Vec<(&'static str, egui::Rect)>> =
        const { std::cell::RefCell::new(Vec::new()) };
}

#[cfg(test)]
fn probe(tag: &'static str, rect: egui::Rect) {
    PROBE.with(|p| p.borrow_mut().push((tag, rect)));
}

#[cfg(not(test))]
fn probe(_tag: &'static str, _rect: egui::Rect) {}

/// One fixed-width table cell. Header and rows both go through here so their columns line up.
fn cell<R>(
    ui: &mut egui::Ui,
    salt: &'static str,
    width: f32,
    add: impl FnOnce(&mut egui::Ui) -> R,
) -> (egui::Rect, R) {
    let (rect, _) = ui.allocate_exact_size(vec2(width, 24.0), egui::Sense::hover());
    let mut child = ui.new_child(
        egui::UiBuilder::new()
            .id_salt(salt)
            .max_rect(rect)
            .layout(Layout::left_to_right(Align::Center)),
    );
    let out = add(&mut child);
    (rect, out)
}

/// The Visibility | Name | Preview header, drawn once above the groups.
fn styles_table_header(ui: &mut egui::Ui) {
    egui::Frame::new()
        .inner_margin(egui::Margin::symmetric(8, 0))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 4.0;
                let head = |ui: &mut egui::Ui, salt: &'static str, width: f32, text: &str| {
                    let (rect, _) = cell(ui, salt, width, |ui| {
                        ui.label(caption_text(text));
                    });
                    rect
                };
                probe("head-vis", head(ui, "h-vis", COL_VISIBILITY, "Visibility"));
                probe("head-name", head(ui, "h-name", COL_NAME, "Name"));
                probe(
                    "head-preview",
                    head(ui, "h-preview", COL_PREVIEW, "Preview"),
                );
            });
        });
}

fn show_styles(
    ui: &mut egui::Ui,
    state: &ConfigState,
    cfg: &mut ConfigUi,
    ctx: &ConfigViewCtx,
    actions: &mut Vec<ConfigAction>,
) {
    ui.label(theme::section_header("STYLES"));

    let (builtin, custom) = style_groups(state);
    if builtin.is_empty() && custom.is_empty() {
        helper(ui, "No styles loaded. Revert to reload styles.json.");
        return;
    }
    styles_table_header(ui);
    if !builtin.is_empty() {
        ui.label(caption_text("Built-in"));
        for (i, name) in builtin.iter().enumerate() {
            style_row(ui, state, cfg, ctx, i, name, None, actions);
        }
        ui.add_space(4.0);
        ui.separator();
        ui.add_space(4.0);
    }
    // Always drawn, so Add style is reachable before the first custom style exists.
    ui.horizontal(|ui| {
        let heading = ui.label(caption_text("Custom"));
        probe("custom-heading", heading.rect);
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            let add = ui.add(small_button("+ ADD STYLE"));
            probe("add-style", add.rect);
            if add.clicked() && cfg.style_draft.is_none() {
                cfg.style_draft = Some(String::new());
                cfg.focus_draft = true;
            }
        });
    });
    if let Some(draft) = cfg.style_draft.clone() {
        style_draft_row(ui, state, cfg, draft, actions);
    }
    let last = custom.len().saturating_sub(1);
    for (i, name) in custom.iter().enumerate() {
        style_row(
            ui,
            state,
            cfg,
            ctx,
            i,
            name,
            Some((i > 0, i < last)),
            actions,
        );
    }
}

/// The style names split into (built-in, custom), each keeping file order.
pub fn style_groups(state: &ConfigState) -> (Vec<String>, Vec<String>) {
    state
        .style_names()
        .into_iter()
        .partition(|n| state.badges(n).builtin)
}

/// One styles-table row. `reorder` is `Some((can_move_up, can_move_down))` for custom styles and
/// `None` for built-ins, which draw no arrows.
#[allow(clippy::too_many_arguments)]
fn style_row(
    ui: &mut egui::Ui,
    state: &ConfigState,
    cfg: &mut ConfigUi,
    ctx: &ConfigViewCtx,
    i: usize,
    name: &str,
    reorder: Option<(bool, bool)>,
    actions: &mut Vec<ConfigAction>,
) {
    let Some(style) = state.style(name) else {
        return;
    };
    let epoch = cfg.epoch;
    ui.push_id(("style", name, epoch), |ui| {
        widgets::zebra_row(ui, i, |ui| {
            let is_expanded = cfg.expanded.as_deref() == Some(name);
            let mut name_left = None;
            let mut arrows_left = None;
            let header = ui.horizontal(|ui| {
                ui.set_min_height(24.0);
                ui.spacing_mut().item_spacing.x = 4.0;
                let (vis, _) = cell(ui, "vis", COL_VISIBILITY, |ui| {
                    let mut on = style.action == StyleAction::Show;
                    let resp = widgets::slide_switch(ui, &mut on).on_hover_text(SWITCH_TIP);
                    if resp.changed() {
                        let action = if on {
                            StyleAction::Show
                        } else {
                            StyleAction::Hide
                        };
                        edit(actions, name, StyleEdit::Action(action));
                    }
                });
                probe("row-vis", vis);
                let (label, _) = cell(ui, "name", COL_NAME, |ui| {
                    ui.add_sized(
                        vec2(COL_NAME, 20.0),
                        egui::Label::new(RichText::new(name).size(14.0).color(theme::VALUE_TEXT))
                            .truncate()
                            .sense(egui::Sense::hover()),
                    )
                    .on_hover_text(name);
                });
                probe("row-name", label);
                name_left = Some(label.left());
                let (chip, _) = cell(ui, "preview", COL_PREVIEW, |ui| {
                    widgets::style_preview_chip(ui, name, style);
                });
                probe("row-preview", chip);
                if let Some((can_up, can_down)) = reorder {
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        let down = widgets::arrow_button(ui, ArrowDirection::Down, can_down);
                        let up = widgets::arrow_button(ui, ArrowDirection::Up, can_up);
                        probe("row-up", up.rect);
                        probe("row-down", down.rect);
                        arrows_left = Some(up.rect.left() - 4.0);
                        if down.clicked() {
                            actions.push(ConfigAction::MoveStyle {
                                name: name.to_string(),
                                up: false,
                            });
                        }
                        if up.clicked() {
                            actions.push(ConfigAction::MoveStyle {
                                name: name.to_string(),
                                up: true,
                            });
                        }
                    });
                }
            });
            // The row's click target covers the name and preview columns only, so it never
            // overlaps the switch or the arrows.
            let mut row_rect = header.response.rect;
            if let Some(left) = name_left {
                row_rect.min.x = left;
            }
            if let Some(left) = arrows_left {
                row_rect.max.x = (left - 2.0).max(row_rect.min.x);
            }
            let row = ui
                .interact(
                    row_rect,
                    ui.id().with(("style-row", name)),
                    egui::Sense::click(),
                )
                .on_hover_cursor(egui::CursorIcon::PointingHand);
            row.widget_info(|| {
                egui::WidgetInfo::selected(
                    egui::WidgetType::CollapsingHeader,
                    true,
                    is_expanded,
                    name,
                )
            });
            if cfg.scroll_to.as_deref() == Some(name) {
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

/// The pinned button row. The app draws the outcome area above it.
pub fn show_footer(
    ui: &mut egui::Ui,
    state: &ConfigState,
    cfg: &mut ConfigUi,
    ctx: &ConfigViewCtx,
) -> Vec<ConfigAction> {
    let mut actions = Vec::new();
    // A revert confirm cannot outlive its edits.
    if !state.is_dirty() {
        cfg.revert_armed = false;
    }

    if cfg.revert_armed && state.is_dirty() && !ctx.changed_on_disk {
        ui.label(
            RichText::new(revert_prompt(state.dirty_count()))
                .size(theme::ROW_SUMMARY_SIZE)
                .color(theme::VALUE_TEXT),
        );
    }
    // The same state line as the RULES footer: one Apply state for both tabs.
    ui.label(
        RichText::new(ctx.indicator)
            .size(theme::ROW_SUMMARY_SIZE)
            .color(theme::TAB_ACTIVE),
    );

    let buttons = footer_buttons(state, cfg, ctx.changed_on_disk, ctx.apply_enabled);
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 8.0;
        for (label, enabled) in buttons {
            let accent = matches!(label, "OVERWRITE" | "Discard");
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
                } else if ctx.rules_blocking {
                    apply::APPLY_BLOCKED_TOOLTIP
                } else if ctx.filter_damaged {
                    apply::DAMAGED_MARKERS_MESSAGE
                } else {
                    APPLY_NOTHING_TIP
                };
                response = response.on_disabled_hover_text(tip);
            }
            match label {
                "RELOAD" => response = response.on_hover_text(apply::RELOAD_TOOLTIP),
                "OVERWRITE" => response = response.on_hover_text(apply::OVERWRITE_TOOLTIP),
                _ => {}
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
                "RELOAD" => actions.push(ConfigAction::Reload),
                "OVERWRITE" => actions.push(ConfigAction::Overwrite),
                "Cancel" => cfg.revert_armed = false,
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
            changed_on_disk: false,
            apply_enabled: true,
            rules_blocking: false,
            filter_damaged: false,
            indicator: "",
            reload_after_apply: true,
        }
    }

    #[test]
    fn reload_toggle_text_is_fixed_ascii_and_renders_quietly() {
        assert_eq!(RELOAD_TOGGLE_LABEL, "Reload filter in game after Apply");
        assert!(RELOAD_TOGGLE_CAPTION.contains("/reloaditemfilter"));
        assert!(RELOAD_TOGGLE_CAPTION.contains("selected in game"));
        assert!(RELOAD_TOGGLE_CAPTION.contains("Close the chat box"));
        assert!(RELOAD_TOGGLE_TIP.contains("command was sent"));
        assert!(RELOAD_TOGGLE_TIP.contains("administrator"));
        assert!(RELOAD_TOGGLE_TIP.contains(super::super::apply::RELOAD_INSTRUCTION));
        assert!(RELOAD_TOGGLE_TIP.is_ascii());
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
    fn styles_split_builtin_first_then_custom() {
        let (_d, state) = loaded();
        let (builtin, custom) = style_groups(&state);
        assert!(!builtin.is_empty());
        assert!(builtin.iter().all(|n| state.badges(n).builtin));
        assert!(custom.iter().all(|n| !state.badges(n).builtin));
        assert_eq!(builtin.len() + custom.len(), state.style_names().len());
    }

    #[test]
    fn style_switch_click_emits_hide() {
        let (_d, state) = loaded();
        let users = HashMap::new();
        let refs = BTreeSet::new();
        let (builtin, _) = style_groups(&state);
        let first = builtin[0].clone();
        assert_eq!(state.style(&first).unwrap().action, StyleAction::Show);
        let mut found = 0;
        let mut first_hit = false;
        for y in (0..140).step_by(3) {
            for x in (8..56).step_by(4) {
                let (ctx, base) = headless();
                let pos = egui::pos2(x as f32, y as f32);
                let mut input = base;
                input.events = vec![
                    egui::Event::PointerMoved(pos),
                    egui::Event::PointerButton {
                        pos,
                        button: egui::PointerButton::Primary,
                        pressed: true,
                        modifiers: Default::default(),
                    },
                    egui::Event::PointerButton {
                        pos,
                        button: egui::PointerButton::Primary,
                        pressed: false,
                        modifiers: Default::default(),
                    },
                ];
                let mut cfg = ConfigUi::default();
                let vctx = view_ctx(None, &users, &refs);
                let mut actions = Vec::new();
                // Frame 1 settles layout and hover; frame 2 delivers the click.
                ctx.run_ui(
                    egui::RawInput {
                        screen_rect: input.screen_rect,
                        ..Default::default()
                    },
                    |ui| {
                        show_styles(ui, &state, &mut cfg, &vctx, &mut actions);
                    },
                )
                .drop_without_applying_deltas();
                ctx.run_ui(input, |ui| {
                    show_styles(ui, &state, &mut cfg, &vctx, &mut actions);
                })
                .drop_without_applying_deltas();
                let hits: Vec<_> = actions
                    .iter()
                    .filter(|a| matches!(a, ConfigAction::EditStyle { .. }))
                    .collect();
                if let Some(ConfigAction::EditStyle { name, edit }) = hits.first() {
                    assert_eq!(hits.len(), 1);
                    assert!(builtin.contains(name));
                    if name == &first {
                        first_hit = true;
                    }
                    assert_eq!(*edit, StyleEdit::Action(StyleAction::Hide));
                    assert_eq!(cfg.expanded, None, "switch click must not expand the row");
                    found += 1;
                }
            }
        }
        assert!(found > 0 && first_hit, "no click reached the first switch");
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
    fn footer_buttons_follow_the_shared_state() {
        let (_d, mut state) = loaded();
        let mut cfg = ConfigUi::default();
        // Apply follows the combined state the app computes; Revert follows the config edits.
        assert_eq!(
            footer_buttons(&state, &cfg, false, true),
            vec![("Apply", true), ("Revert", false)]
        );
        assert_eq!(
            footer_buttons(&state, &cfg, false, false),
            vec![("Apply", false), ("Revert", false)]
        );
        state.set_tier_text("S", "oops");
        assert_eq!(
            footer_buttons(&state, &cfg, false, false),
            vec![("Apply", false), ("Revert", true)]
        );
        assert!(state.has_tier_errors());

        // After a changed-on-disk refusal: the same two buttons as the RULES footer, no guard.
        let changed = footer_buttons(&state, &cfg, true, true);
        assert_eq!(changed, vec![("RELOAD", true), ("OVERWRITE", true)]);
        assert!(changed.iter().all(|(l, _)| *l != "Cancel"));

        // The Revert confirm still works.
        cfg.revert_armed = true;
        assert_eq!(
            footer_buttons(&state, &cfg, false, true),
            vec![("Discard", true), ("Cancel", true)]
        );
        assert_eq!(revert_prompt(3), "Discard 3 unapplied config changes?");
    }

    fn drain_probes() -> Vec<(&'static str, egui::Rect)> {
        PROBE.with(|p| std::mem::take(&mut *p.borrow_mut()))
    }

    fn shape_texts(shape: &egui::Shape, out: &mut Vec<String>) {
        match shape {
            egui::Shape::Text(t) => out.push(t.galley.text().to_string()),
            egui::Shape::Vec(v) => v.iter().for_each(|s| shape_texts(s, out)),
            _ => {}
        }
    }

    fn tall_input() -> egui::RawInput {
        egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                vec2(460.0, 1400.0),
            )),
            ..Default::default()
        }
    }

    #[test]
    fn no_status_line_or_text_badges() {
        let (_d, mut state) = loaded();
        state.add_style("Gold").unwrap();
        assert!(state.edit_style("S", StyleEdit::Font(Some(40))));
        let users = HashMap::new();
        let refs = BTreeSet::new();
        let (ctx, _) = headless();
        let mut cfg = ConfigUi::default();
        let vctx = view_ctx(None, &users, &refs);
        let out = ctx.run_ui(tall_input(), |ui| {
            show_sections(ui, &state, &mut cfg, &vctx);
        });
        let mut texts = Vec::new();
        for clipped in &out.shapes {
            shape_texts(&clipped.shape, &mut texts);
        }
        out.drop_without_applying_deltas();
        assert!(!texts.is_empty(), "nothing was drawn");
        for t in &texts {
            assert!(!t.contains("unapplied change"), "{t}");
            assert!(t != "new" && t != "edited", "{t}");
        }
    }

    #[test]
    fn custom_heading_always_shows_add_button() {
        let (_d, mut state) = loaded();
        for name in style_groups(&state).1 {
            assert!(state.delete_style(&name));
        }
        assert!(style_groups(&state).1.is_empty(), "no custom styles left");
        let users = HashMap::new();
        let refs = BTreeSet::new();
        let (ctx, _) = headless();
        let mut cfg = ConfigUi::default();
        let vctx = view_ctx(None, &users, &refs);
        let mut actions = Vec::new();
        drain_probes();
        ctx.run_ui(tall_input(), |ui| {
            show_styles(ui, &state, &mut cfg, &vctx, &mut actions);
        })
        .drop_without_applying_deltas();
        let probes = drain_probes();
        let heading: Vec<_> = probes
            .iter()
            .filter(|(t, _)| *t == "custom-heading")
            .collect();
        let add: Vec<_> = probes.iter().filter(|(t, _)| *t == "add-style").collect();
        assert_eq!(heading.len(), 1, "the Custom heading is always drawn");
        assert_eq!(
            add.len(),
            1,
            "exactly one add button, on the Custom heading"
        );
        let (h, a) = (heading[0].1, add[0].1);
        assert!(
            a.top() < h.bottom() && h.top() < a.bottom(),
            "button {a:?} is not on the heading row {h:?}"
        );
        assert!(
            a.left() > h.right(),
            "button sits to the right of the heading"
        );
    }

    #[test]
    fn styles_table_header_and_columns() {
        let (_d, mut state) = loaded();
        state.add_style("k1").unwrap();
        let users = HashMap::new();
        let refs = BTreeSet::new();
        let (ctx, _) = headless();
        let mut cfg = ConfigUi::default();
        let vctx = view_ctx(None, &users, &refs);
        let mut actions = Vec::new();
        drain_probes();
        ctx.run_ui(tall_input(), |ui| {
            show_styles(ui, &state, &mut cfg, &vctx, &mut actions);
        })
        .drop_without_applying_deltas();
        let probes = drain_probes();
        let of = |tag: &str| -> Vec<egui::Rect> {
            probes
                .iter()
                .filter(|(t, _)| *t == tag)
                .map(|(_, r)| *r)
                .collect()
        };
        let (hv, hn, hp) = (of("head-vis"), of("head-name"), of("head-preview"));
        assert_eq!((hv.len(), hn.len(), hp.len()), (1, 1, 1));
        assert!(hv[0].left() < hn[0].left() && hn[0].left() < hp[0].left());
        let total = state.style_names().len();
        for (tag, head) in [
            ("row-vis", hv[0]),
            ("row-name", hn[0]),
            ("row-preview", hp[0]),
        ] {
            let rows = of(tag);
            assert_eq!(rows.len(), total, "{tag}");
            for r in rows {
                assert!(
                    (r.left() - head.left()).abs() <= 1.0,
                    "{tag} {r:?} vs header {head:?}"
                );
            }
        }
    }

    /// Clicks `target` on a fresh frame after one settling frame.
    fn click_styles(
        ctx: &egui::Context,
        state: &ConfigState,
        cfg: &mut ConfigUi,
        target: egui::Pos2,
    ) -> Vec<ConfigAction> {
        let users = HashMap::new();
        let refs = BTreeSet::new();
        let vctx = view_ctx(None, &users, &refs);
        let mut actions = Vec::new();
        let button = |pressed| egui::Event::PointerButton {
            pos: target,
            button: egui::PointerButton::Primary,
            pressed,
            modifiers: Default::default(),
        };
        ctx.run_ui(tall_input(), |ui| {
            show_styles(ui, state, cfg, &vctx, &mut actions);
        })
        .drop_without_applying_deltas();
        let mut input = tall_input();
        input.events = vec![
            egui::Event::PointerMoved(target),
            button(true),
            button(false),
        ];
        ctx.run_ui(input, |ui| {
            show_styles(ui, state, cfg, &vctx, &mut actions);
        })
        .drop_without_applying_deltas();
        actions
    }

    fn moves(actions: &[ConfigAction]) -> Vec<&ConfigAction> {
        actions
            .iter()
            .filter(|a| matches!(a, ConfigAction::MoveStyle { .. }))
            .collect()
    }

    #[test]
    fn custom_arrows_reorder_and_builtins_have_none() {
        let (_d, mut state) = loaded();
        for n in ["k1", "k2", "k3"] {
            state.add_style(n).unwrap();
        }
        let (builtin, custom) = style_groups(&state);
        assert_eq!(custom.last().map(String::as_str), Some("k3"));
        let users = HashMap::new();
        let refs = BTreeSet::new();
        let (ctx, _) = headless();
        let mut cfg = ConfigUi::default();
        let vctx = view_ctx(None, &users, &refs);
        let mut actions = Vec::new();
        drain_probes();
        ctx.run_ui(tall_input(), |ui| {
            show_styles(ui, &state, &mut cfg, &vctx, &mut actions);
        })
        .drop_without_applying_deltas();
        let probes = drain_probes();
        let rects = |tag: &str| -> Vec<egui::Rect> {
            probes
                .iter()
                .filter(|(t, _)| *t == tag)
                .map(|(_, r)| *r)
                .collect()
        };
        let (ups, downs) = (rects("row-up"), rects("row-down"));
        assert_eq!(ups.len(), custom.len(), "arrows only on custom rows");
        assert_eq!(downs.len(), custom.len());

        // The ends are disabled: a click on them emits nothing.
        let (ctx, _) = headless();
        let mut cfg = ConfigUi::default();
        let first_up = click_styles(&ctx, &state, &mut cfg, ups[0].center());
        assert!(moves(&first_up).is_empty(), "the first Up is disabled");
        let (ctx, _) = headless();
        let mut cfg = ConfigUi::default();
        let last_down = click_styles(&ctx, &state, &mut cfg, downs[downs.len() - 1].center());
        assert!(moves(&last_down).is_empty(), "the last Down is disabled");

        // k3's Up reorders.
        let (ctx, _) = headless();
        let mut cfg = ConfigUi::default();
        let clicked = click_styles(&ctx, &state, &mut cfg, ups[ups.len() - 1].center());
        assert_eq!(
            moves(&clicked),
            [&ConfigAction::MoveStyle {
                name: "k3".to_string(),
                up: true
            }]
        );
        assert!(state.move_style("k3", true));
        let (builtin_after, custom_after) = style_groups(&state);
        assert_eq!(builtin_after, builtin, "built-ins are unchanged");
        let tail: Vec<_> = custom_after
            .iter()
            .rev()
            .take(3)
            .rev()
            .map(String::as_str)
            .collect();
        assert_eq!(tail, ["k1", "k3", "k2"]);
        assert!(!state.move_style("S", true), "built-ins never move");
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
        // A few frames: a bottom panel sizes itself from the previous frame's content.
        for _ in 0..4 {
            let out = ctx.run_ui(input.clone(), |ui| {
                egui::CentralPanel::default().show(ui, |ui| {
                    egui::Panel::bottom(egui::Id::new("cfg-test-footer")).show(ui, |ui| {
                        show_footer(ui, &state, &mut cfg, &vctx);
                        footer_rect = ui.min_rect();
                    });
                    show_sections(ui, &state, &mut cfg, &vctx);
                });
            });
            out.drop_without_applying_deltas();
        }
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
