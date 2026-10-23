//! The CONFIG tab: a thin egui view over [`ConfigState`].
//!
//! Layer rule (same as `rule_list.rs`): all model mutations are returned as [`ConfigAction`]s and
//! applied by the app after the frame; this module never mutates [`ConfigState`] and never touches
//! the filesystem (the filter list and the style-use counts arrive through [`ConfigViewCtx`]).
//! The style editor's lines are the shared `style_lines` component (also drawn for a Custom
//! rule's STYLE section); this view maps their intents onto `StyleEdit`s.

use std::collections::{BTreeSet, HashMap};
use std::path::Path;

use egui::{Align, Layout, RichText, vec2};

use crate::models::config::{
    Cancelable, Style, StyleAction, StyleColour, StyleNameError, StyleSet,
};
use crate::settings;

use super::body;
use super::config_state::{
    ConfigState, RevertScope, StyleEdit, config_warning, delete_confirm_text,
};
use super::style_lines::{self, StyleIntent, StyleView, Vis, probe};
use super::widgets;
use crate::skin::editor as ed;
use crate::skin::fonts::{regular, small_caps};
use crate::skin::minimap;
use crate::skin::palette as pal;

/// Width of the Browse button.
const BROWSE_WIDTH: f32 = 120.0;
const SWITCH_TIP: &str = "On: shown. Off: hidden (Alt reveals).";
const ORDER_TIP: &str = "Built-in styles keep their order";
const RENAME_TIP: &str = "Built-in names are fixed. Rename a custom style in its table row.";
const INVALID_RED: egui::Color32 = egui::Color32::from_rgb(0xFF, 0x55, 0x55);
const UNIT_TIP: &str = "Set in tiers.json; not editable here.";
/// Label of the FILTER section toggle that sends `/reloaditemfilter` after an Apply.
pub const RELOAD_TOGGLE_LABEL: &str = "Reload filter in game after Apply";
/// Caveats shown under the toggle: the game reloads whichever filter is selected in game, and text
/// left in an open chat box is sent first (nothing outside the game can see whether chat is open).
pub const RELOAD_TOGGLE_CAPTION: &str = "Sends /reloaditemfilter to the game after Apply. It reloads the filter selected in game, so select this filter there too. Close the chat box before pressing Apply: text left in an open chat box is sent as a message instead of the command.";
/// The toggle's hover text: what "Reloaded in game" does and does not certify (review WR-01).
pub const RELOAD_TOGGLE_TIP: &str = "\"Reloaded in game\" means the command was sent to the game window; ff cannot see whether the game ran it. A game running as administrator ignores the keys unless ff runs as administrator too. If the game did not reload, use Options > Game > Item Filter > Refresh.";
pub(crate) const APPLY_NOTHING_TIP: &str = "Nothing to apply.";
pub(crate) const APPLY_LOAD_ERROR_TIP: &str =
    "styles.json or tiers.json failed to load. Revert to reload them.";

/// Everything the CONFIG view asks the app to do. Applied after the frame.
#[derive(Debug, Clone, PartialEq)]
pub enum ConfigAction {
    SelectFilter(String),
    CreateFilter(String),
    ConfirmSwitch,
    /// Open the native folder dialog to pick the game folder (the app starts it where the frame is at hand).
    BrowseFolder,
    /// Re-list the game folder's `.filter` files and the sound files: no write, no selection change.
    RefreshLists,
    CancelSwitch,
    SetReloadAfterApply(bool),
    /// A strictness level was clicked: a pending edit of the tier styles' visibility.
    SetStrictness(crate::overlay::strictness::Strictness),
    AddStyle(String),
    EditStyle {
        name: String,
        edit: StyleEdit,
    },
    DeleteStyle(String),
    /// Rename a custom style (committed from the Name cell's text field).
    RenameStyle {
        from: String,
        to: String,
    },
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
    /// Revert one tab's CONFIG edits: styles from CONFIG, tiers from ECONOMY.
    RevertScoped(RevertScope),
    /// Restore the built-in styles to their defaults (customs kept); a pending edit.
    ResetStyles,
    /// Restore the tier cutoffs to their defaults; a pending edit.
    ResetTiers,
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
    /// Salt of the remembered None-values (font, minimap, beam, sound): bumped only when the
    /// styles are replaced (Revert, Reload, Apply), so folding a row keeps them.
    pub memo_epoch: u64,
    /// Open `+ ADD STYLE` name draft.
    pub style_draft: Option<String>,
    /// Open `New empty filter...` name draft.
    pub filter_draft: Option<String>,
    /// Style whose inline delete confirm is showing.
    pub delete_armed: Option<String>,
    pub revert_armed: bool,
    /// What the armed Discard config changes? popup discards.
    pub revert_scope: RevertScope,
    /// Which Reset to defaults? popup is armed (styles on CONFIG, tiers on ECONOMY).
    pub reset_armed: Option<RevertScope>,
    /// A just-added style to scroll into view.
    pub scroll_to: Option<String>,
    /// A name draft was just opened; its text field takes focus once.
    pub focus_draft: bool,
    /// In-progress rename text per custom style id, kept only while it differs from the name.
    pub rename_drafts: HashMap<String, String>,
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

    /// Expand `name` (never a toggle: a row that is already expanded stays expanded). Drops an
    /// armed delete confirm and orphans per-row egui state, as [`Self::set_expanded`] does.
    pub fn expand(&mut self, name: &str) {
        self.expanded = Some(name.to_string());
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
        self.revert_scope = RevertScope::Styles;
        self.reset_armed = None;
    }

    /// Called when the styles are replaced (CONFIG Revert, reload, Apply, an outside edit
    /// followed): every confirm disarms and rename drafts go, so a name field never keeps showing
    /// stale text instead of the real name (WR-05).
    pub fn reset_for_new_styles(&mut self) {
        self.disarm_all();
        self.rename_drafts.clear();
        // Open pickers sit under ids salted with the epoch, the remembered None-values under
        // the memo epoch.
        self.epoch = self.epoch.wrapping_add(1);
        self.memo_epoch = self.memo_epoch.wrapping_add(1);
    }
}

/// What the view needs from the app each frame.
pub struct ConfigViewCtx<'a> {
    /// `*.filter` names in the game folder; `None` when the folder cannot be read.
    pub filters: Option<&'a [String]>,
    pub active_filter: &'a str,
    /// `active_filter` is saved as the user's selection (not just the filter the overlay opened
    /// because none is selected).
    pub active_filter_selected: bool,
    pub game_dir: Option<&'a Path>,
    /// The game folder is not on disk (refresh and the filter dropdown are disabled).
    pub game_dir_missing: bool,
    /// `FERRISFILTER_GAME_DIR` pins the folder (Browse is disabled).
    pub game_dir_pinned: bool,
    /// The default game folder, Browse's tooltip.
    pub default_game_dir: Option<&'a Path>,
    /// The native folder dialog is open (Browse is disabled).
    pub folder_dialog_open: bool,
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
    /// The shell skin for section headings (`None` in headless tests).
    pub skin: Option<&'a crate::skin::Skin>,
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

/// Why `draft` cannot be committed as the new name of custom style `from` right now.
pub fn rename_hint(state: &ConfigState, from: &str, draft: &str) -> Option<String> {
    state
        .pending_styles()
        .clone()
        .rename_custom(from, draft.trim())
        .err()
        .map(|e| e.to_string())
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

// ---------------------------------------------------------------------------------------------
// Small widgets
// ---------------------------------------------------------------------------------------------

/// A section heading: the skin's gold rule heading, or a plain heading label when no skin is
/// available (headless tests).
fn section_title(ui: &mut egui::Ui, ctx: &ConfigViewCtx, title: &str) -> egui::Response {
    match ctx.skin {
        Some(skin) => crate::skin::widgets::ff_section(ui, skin, title),
        None => ui.label(RichText::new(title).size(21.0)),
    }
}

fn small_button(text: &str) -> egui::Button<'static> {
    egui::Button::new(RichText::new(text.to_string()).size(crate::skin::fonts::SUMMARY_SIZE))
}

// ---------------------------------------------------------------------------------------------
// Sections
// ---------------------------------------------------------------------------------------------

/// FILTER and STYLES. The app wraps this in the shell's game-art scroll area; Apply,
/// Revert and Reload live in the shared bottom bar.
pub fn show_sections(
    ui: &mut egui::Ui,
    state: &ConfigState,
    cfg: &mut ConfigUi,
    ctx: &ConfigViewCtx,
) -> Vec<ConfigAction> {
    let mut actions = Vec::new();
    section_title(ui, ctx, "Filter");
    show_filter(ui, cfg, ctx, &mut actions);
    ui.add_space(16.0);
    show_styles(ui, state, cfg, ctx, &mut actions);
    actions
}

fn show_filter(
    ui: &mut egui::Ui,
    cfg: &mut ConfigUi,
    ctx: &ConfigViewCtx,
    actions: &mut Vec<ConfigAction>,
) {
    egui::Frame::new()
        .fill(pal::ROW_EVEN)
        .inner_margin(egui::Margin::same(8))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.spacing_mut().item_spacing.y = 4.0;
            let gap = crate::skin::widgets::FIELD_GAP;
            let row_h = crate::skin::widgets::ROW_HEIGHT;
            let missing = ctx.game_dir_missing;

            // Row 1: the game folder and its Browse button.
            crate::skin::widgets::ff_field_row(ui, 0, "Game folder", None, |ui| {
                ui.spacing_mut().item_spacing.x = gap;
                let path_w = (ui.available_width() - BROWSE_WIDTH - gap).max(24.0);
                let (text, color) = match ctx.game_dir {
                    Some(dir) => (
                        dir.display().to_string(),
                        if missing { pal::WARN } else { pal::VALUE },
                    ),
                    None => ("(no Documents folder)".to_string(), pal::WARN),
                };
                ui.allocate_ui(vec2(path_w, row_h), |ui| {
                    ui.set_width(path_w);
                    let label = ui.add(
                        egui::Label::new(
                            RichText::new(text.clone())
                                .size(crate::skin::fonts::SUMMARY_SIZE)
                                .color(color),
                        )
                        .truncate()
                        // The full path tooltip below is the only one: egui's own elided-text
                        // tooltip would show the same path a second time.
                        .show_tooltip_when_elided(false),
                    );
                    if ctx.game_dir.is_some() {
                        label.on_hover_text(text);
                    }
                });
                let enabled = !ctx.game_dir_pinned && !ctx.folder_dialog_open;
                let browse = match ctx.skin {
                    Some(skin) => crate::skin::widgets::ff_button_ex(
                        ui,
                        skin,
                        "Browse",
                        BROWSE_WIDTH,
                        enabled,
                        false,
                    ),
                    None => ui.add_enabled(enabled, small_button("Browse")),
                };
                let tip = if ctx.game_dir_pinned {
                    Some("Set by FERRISFILTER_GAME_DIR".to_string())
                } else {
                    ctx.default_game_dir
                        .map(|d| format!("Default: {}", d.display()))
                };
                let browse = match tip {
                    Some(t) => browse.on_hover_text(t.clone()).on_disabled_hover_text(t),
                    None => browse,
                };
                if enabled && browse.clicked() {
                    actions.push(ConfigAction::BrowseFolder);
                }
            });

            // Row 2: the filter dropdown with the refresh button beside it.
            let list: &[String] = ctx.filters.unwrap_or(&[]);
            let shown = if ctx.filters.is_some() {
                ctx.active_filter.to_string()
            } else {
                "(game folder not found)".to_string()
            };
            let mut open_draft = false;
            crate::skin::widgets::ff_field_row(ui, 1, "Filter", None, |ui| {
                ui.spacing_mut().item_spacing.x = gap;
                let combo_w = (ui.available_width() - row_h - gap).max(24.0);
                let enabled = ctx.filters.is_some() && !missing;
                let combo = ui
                    .add_enabled_ui(enabled, |ui| {
                        egui::ComboBox::from_id_salt("cfg-filter")
                            .selected_text(shown)
                            .width(combo_w)
                            .show_ui(ui, |ui| {
                                for name in list {
                                    let is_active = name == ctx.active_filter;
                                    // The open filter, when the overlay chose it on its own, can
                                    // still be picked: that saves it as the selection.
                                    if ui.selectable_label(is_active, name).clicked()
                                        && (!is_active || !ctx.active_filter_selected)
                                    {
                                        actions.push(ConfigAction::SelectFilter(name.clone()));
                                    }
                                }
                                ui.separator();
                                if ui
                                    .selectable_label(false, "New empty filter\u{2026}")
                                    .clicked()
                                {
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
                // Refresh only re-lists, so it stays live while the folder is missing: it is how
                // a folder that appeared later (the game's first launch) is picked up.
                let refresh_on = ctx.game_dir.is_some();
                let refresh = match ctx.skin {
                    Some(skin) => {
                        crate::skin::widgets::ff_reload_button(ui, skin, "Refresh", refresh_on)
                    }
                    None => ui.add_enabled(refresh_on, small_button("Refresh")),
                };
                let tip = "Refresh the file lists";
                let refresh = refresh.on_hover_text(tip).on_disabled_hover_text(tip);
                if refresh_on && refresh.clicked() {
                    actions.push(ConfigAction::RefreshLists);
                }
            });
            if open_draft && cfg.filter_draft.is_none() {
                cfg.filter_draft = Some(String::new());
                cfg.focus_draft = true;
            }

            if let Some(draft) = cfg.filter_draft.clone() {
                filter_draft_row(ui, cfg, list, draft, actions);
            }

            ui.add_space(4.0);
            let mut on = ctx.reload_after_apply;
            let reload_tip = format!("{RELOAD_TOGGLE_CAPTION}\n\n{RELOAD_TOGGLE_TIP}");
            ui.horizontal(|ui| {
                let toggle =
                    widgets::circular_toggle(ui, &mut on).on_hover_text(reload_tip.clone());
                if toggle.changed() {
                    actions.push(ConfigAction::SetReloadAfterApply(on));
                }
                ui.label(
                    RichText::new(RELOAD_TOGGLE_LABEL)
                        .size(crate::skin::fonts::SUMMARY_SIZE)
                        .color(pal::VALUE),
                )
                .on_hover_text(reload_tip);
            });
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
    let mut edit = ui.add(
        egui::TextEdit::singleline(&mut draft)
            .id_salt("cfg-filter-draft")
            .hint_text("my-filter.filter")
            .desired_width(ui.available_width() - 8.0),
    );
    if draft.trim().is_empty()
        && let Some(msg) = &hint
    {
        edit = edit.on_hover_text(msg.as_str());
    }
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

/// Width of a tier cutoff box: room for about 8 digits (cutoffs stay under ~6).
const TIER_FIELD_W: f32 = 88.0;
/// Gap between a tier cutoff box and its unit.
const TIER_UNIT_GAP: f32 = crate::skin::widgets::FIELD_GAP;

/// The tier cutoffs editor, drawn on the ECONOMY tab under its Tiers heading.
pub fn show_tiers_section(
    ui: &mut egui::Ui,
    skin: &crate::skin::Skin,
    state: &ConfigState,
) -> Vec<ConfigAction> {
    let _ = skin;
    let mut actions = Vec::new();
    let tip = "Minimum price for each tier. Items worth at least this much get the tier.";
    for (i, row) in state.tier_rows().iter().enumerate() {
        let inner = crate::skin::widgets::ff_field_row(ui, i, &row.tier, Some(tip), |ui| {
            // The box starts at the value column of the ECONOMY rows above (`ff_row` puts its
            // control `CONTROL_WIDTH` from the right edge), sized for about 8 digits, then a clear
            // gap before the unit.
            ui.spacing_mut().item_spacing.x = 0.0;
            let value_x = ui.max_rect().max.x - crate::skin::widgets::CONTROL_WIDTH;
            let lead = (value_x - ui.cursor().min.x).max(0.0);
            ui.add_space(lead);
            let field_x = ui.cursor().min.x;
            let id = ui.make_persistent_id(("cfg-tier", &row.tier));
            let mut buf = row.buffer.clone();
            let edit = crate::skin::widgets::ff_text_field_id(ui, id, &mut buf, "", TIER_FIELD_W);
            // The field leaves FIELD_GAP after its frame; this tops up to exactly the same gap
            // if the row spacing ever changes.
            let unit_x = field_x + TIER_FIELD_W + TIER_UNIT_GAP;
            ui.add_space((unit_x - ui.cursor().min.x).max(0.0));
            if edit.changed() {
                actions.push(ConfigAction::TierText {
                    tier: row.tier.clone(),
                    text: buf,
                });
            }
            if let Some(err) = &row.error {
                ui.painter().rect_stroke(
                    edit.rect,
                    2.0,
                    egui::Stroke::new(1.0, INVALID_RED),
                    egui::StrokeKind::Inside,
                );
                edit.on_hover_text(err);
            }
            ui.label(
                RichText::new(row.unit)
                    .font(crate::skin::fonts::regular(14.0))
                    .color(crate::skin::palette::MUTED),
            )
            .on_hover_text(format!(
                "{UNIT_TIP} Only your live tiers.json is edited; the built-in early-league fallback is never changed."
            ));
        });
        if row.edge {
            let r = inner.response.rect;
            let bar = egui::Rect::from_min_size(r.left_top(), vec2(4.0, r.height()));
            ui.painter().rect_filled(bar, 0.0, INVALID_RED);
        }
    }
    actions
}

/// Gap between two columns of the styles table.
const COL_GAP: f32 = 8.0;
/// Height of the table's column header strip.
const HEADER_HEIGHT: f32 = 24.0;

/// One column of the styles table: `x` from the row's left edge, and its width.
#[derive(Debug, Clone, Copy, PartialEq)]
struct Col {
    x: f32,
    w: f32,
}

impl Col {
    /// This column's rect inside `row`.
    fn rect(self, row: egui::Rect) -> egui::Rect {
        egui::Rect::from_min_size(
            egui::pos2(row.left() + self.x, row.top()),
            vec2(self.w, row.height()),
        )
    }
}

/// The five columns of the styles table (Visibility | Name | Icon | Preview | Order), shared by
/// the header and every row so they line up across the Built-in and Custom groups.
#[derive(Debug, Clone, Copy, PartialEq)]
struct TableColumns {
    vis: Col,
    name: Col,
    icon: Col,
    preview: Col,
    order: Col,
}

/// Lays the columns out across `width`: `ROW_PAD` on both sides, `COL_GAP` between columns, the
/// Name column flexible with a 110pt minimum (the Preview column gives way below that).
fn table_columns(width: f32) -> TableColumns {
    const VIS: f32 = 64.0;
    const ICON: f32 = 32.0;
    const PREVIEW: f32 = 160.0;
    const ORDER: f32 = 56.0;
    const NAME_MIN: f32 = 110.0;
    const PREVIEW_MIN: f32 = 100.0;
    let pad = crate::skin::widgets::ROW_PAD;
    let fixed = 2.0 * pad + VIS + ICON + ORDER + 4.0 * COL_GAP;
    let mut preview = PREVIEW;
    let mut name = width - fixed - preview;
    if name < NAME_MIN {
        preview = (preview - (NAME_MIN - name)).max(PREVIEW_MIN);
        name = NAME_MIN;
    }
    let vis = Col { x: pad, w: VIS };
    let name = Col {
        x: vis.x + vis.w + COL_GAP,
        w: name,
    };
    let icon = Col {
        x: name.x + name.w + COL_GAP,
        w: ICON,
    };
    let preview = Col {
        x: icon.x + icon.w + COL_GAP,
        w: preview,
    };
    let order = Col {
        x: preview.x + preview.w + COL_GAP,
        w: ORDER,
    };
    TableColumns {
        vis,
        name,
        icon,
        preview,
        order,
    }
}

/// One table cell: a child ui over `rect` with `layout`.
/// Where the text starts inside a custom style's name field; built-in names start there too.
const NAME_TEXT_INSET: f32 = 9.0;

fn cell_at<R>(
    ui: &mut egui::Ui,
    salt: &'static str,
    rect: egui::Rect,
    layout: Layout,
    add: impl FnOnce(&mut egui::Ui) -> R,
) -> R {
    let mut child = ui.new_child(
        egui::UiBuilder::new()
            .id_salt(salt)
            .max_rect(rect)
            .layout(layout),
    );
    add(&mut child)
}

/// The Visibility | Name | Icon | Preview | Order header of one group.
fn styles_table_header(ui: &mut egui::Ui) {
    let width = ui.available_width();
    let cols = table_columns(width);
    let (rect, _) = ui.allocate_exact_size(vec2(width, HEADER_HEIGHT), egui::Sense::hover());
    let painter = ui.painter();
    for (tag, text, col, right) in [
        ("head-vis", "Visibility", cols.vis, false),
        ("head-name", "Name", cols.name, false),
        ("head-icon", "Icon", cols.icon, false),
        ("head-preview", "Preview", cols.preview, false),
        // Right-aligned over the arrows, which hug the table's right edge.
        ("head-order", "Order", cols.order, true),
    ] {
        let cell = col.rect(rect);
        let galley = painter.layout_no_wrap(text.to_owned(), regular(14.0), pal::MUTED);
        let x = if right {
            cell.right() - galley.size().x
        } else {
            cell.left()
        };
        let y = rect.center().y - galley.size().y / 2.0;
        painter.galley(egui::pos2(x, y), galley, pal::MUTED);
        probe(tag, cell);
    }
    painter.hline(
        rect.x_range(),
        rect.bottom() - 0.5,
        egui::Stroke::new(1.0, pal::INPUT_BORDER),
    );
}

/// A group heading ("Built-in" / "Custom") in the skin's small caps.
fn group_heading(ui: &mut egui::Ui, title: &str) -> egui::Response {
    ui.label(
        RichText::new(title)
            .font(small_caps(15.0))
            .color(pal::LABEL),
    )
}

/// The one-line strictness switch; the selection is derived from the pending style set.
fn strictness_row(ui: &mut egui::Ui, state: &ConfigState, actions: &mut Vec<ConfigAction>) {
    use crate::overlay::strictness::Strictness;
    let tips: Vec<String> = Strictness::ALL.iter().map(|l| l.tooltip()).collect();
    let levels: Vec<crate::skin::editor::Level> = Strictness::ALL
        .iter()
        .zip(&tips)
        .map(|(l, tip)| crate::skin::editor::Level {
            lines: l.caption_lines(),
            tip,
            clickable: *l != Strictness::Custom,
        })
        .collect();
    let current = state.strictness();
    let selected = Strictness::ALL
        .iter()
        .position(|l| *l == current)
        .unwrap_or(0);
    let (resp, clicked) =
        crate::skin::editor::ff_levels(ui, &crate::skin::of(ui.ctx()), selected, &levels);
    probe("strictness-row", resp.rect);
    if let Some(i) = clicked {
        actions.push(ConfigAction::SetStrictness(Strictness::ALL[i]));
    }
}

fn show_styles(
    ui: &mut egui::Ui,
    state: &ConfigState,
    cfg: &mut ConfigUi,
    ctx: &ConfigViewCtx,
    actions: &mut Vec<ConfigAction>,
) {
    let heading = section_title(ui, ctx, "Styles");

    let (builtin, custom) = style_groups(state);
    if builtin.is_empty() && custom.is_empty() {
        ui.label(
            RichText::new("No styles loaded")
                .font(regular(15.0))
                .color(pal::WARN),
        )
        .on_hover_text("Revert to reload styles.json.");
        return;
    }
    let skin = crate::skin::of(ui.ctx());
    {
        let enabled = !state.styles_at_defaults();
        let mut right = ui.new_child(
            egui::UiBuilder::new()
                .max_rect(egui::Rect::from_min_max(
                    heading.rect.min,
                    egui::pos2(ui.max_rect().right() - 8.0, heading.rect.max.y),
                ))
                .layout(Layout::right_to_left(Align::Center)),
        );
        let mut r = crate::skin::widgets::ff_button_ex(
            &mut right,
            &skin,
            "Reset to defaults",
            150.0,
            enabled,
            false,
        );
        // `ff_button_ex` greys a disabled button inside an enabled Ui, so the response still
        // counts as enabled: attach the tip both ways.
        let tip = if enabled {
            "Restore the built-in styles. Your custom styles are kept."
        } else {
            "Already at the defaults."
        };
        r = r.on_hover_text(tip).on_disabled_hover_text(tip);
        probe("reset-styles", r.rect);
        if r.clicked() && enabled {
            cfg.reset_armed = Some(RevertScope::Styles);
        }
    }
    ui.add_space(4.0);
    let heading = group_heading(ui, "Economy Visibility").on_hover_text(
        "Which economy tiers show: hides the tier styles S to F that the FF economy section uses, \
         lowest first.",
    );
    probe("strictness-heading", heading.rect);
    strictness_row(ui, state, actions);
    ui.add_space(8.0);
    if !builtin.is_empty() {
        let heading = group_heading(ui, "Built-in");
        probe("builtin-heading", heading.rect);
        styles_table_header(ui);
        for (i, name) in builtin.iter().enumerate() {
            style_row(ui, state, cfg, ctx, i, name, None, actions);
        }
        ui.add_space(4.0);
        ui.separator();
        ui.add_space(4.0);
    }
    // Always drawn, so Add style is reachable before the first custom style exists.
    ui.horizontal(|ui| {
        let heading = group_heading(ui, "Custom");
        probe("custom-heading", heading.rect);
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            let add = crate::skin::widgets::ff_button(ui, &skin, "+ Add style", 124.0);
            probe("add-style", add.rect);
            if add.clicked() && cfg.style_draft.is_none() {
                cfg.style_draft = Some(String::new());
                cfg.focus_draft = true;
            }
        });
    });
    // With no custom styles there is nothing for a header to label.
    if !custom.is_empty() || cfg.style_draft.is_some() {
        styles_table_header(ui);
    }
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

/// The Name cell of a custom row: an inline text field committing on Enter or focus-out.
fn rename_field(
    ui: &mut egui::Ui,
    state: &ConfigState,
    cfg: &mut ConfigUi,
    ctx: &ConfigViewCtx,
    name: &str,
    width: f32,
    actions: &mut Vec<ConfigAction>,
) {
    let key = state.style_id(name).unwrap_or(name).to_string();
    let mut text = cfg
        .rename_drafts
        .get(&key)
        .cloned()
        .unwrap_or_else(|| name.to_string());
    let hint = rename_hint(state, name, &text);
    let resp = crate::skin::widgets::ff_text_field_id(
        ui,
        ui.id().with("rename"),
        &mut text,
        "Style name",
        width,
    );
    if hint.is_some() {
        ui.painter().rect_stroke(
            resp.rect,
            2.0,
            egui::Stroke::new(1.0, INVALID_RED),
            egui::StrokeKind::Outside,
        );
    }
    let tip = match &hint {
        Some(h) => Some(h.clone()),
        None => ctx.config_refs.contains(name).then(|| config_warning(name)),
    };
    let resp = match tip {
        Some(t) => resp.on_hover_text(t),
        None => resp,
    };
    let escaped = resp.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Escape));
    if escaped {
        text = name.to_string();
    } else if resp.lost_focus() && hint.is_none() && text.trim() != name {
        actions.push(ConfigAction::RenameStyle {
            from: name.to_string(),
            to: text.trim().to_string(),
        });
    }
    if text == name {
        cfg.rename_drafts.remove(&key);
    } else {
        cfg.rename_drafts.insert(key, text);
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
/// `None` for built-ins.
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
    let skin = crate::skin::of(ui.ctx());
    ui.push_id(("style", name, epoch), |ui| {
        let width = ui.available_width();
        let cols = table_columns(width);
        let is_expanded = cfg.expanded.as_deref() == Some(name);
        let (rect, _) = ui.allocate_exact_size(
            vec2(width, crate::skin::widgets::ROW_HEIGHT),
            egui::Sense::hover(),
        );
        crate::skin::widgets::row_background(ui, i, rect);

        // The row's click target covers the name (built-in) or icon (custom: the Name cell is a
        // text field) through the preview column, so it never overlaps the switch or the arrows.
        let first = if reorder.is_some() {
            cols.icon
        } else {
            cols.name
        };
        let click_rect = egui::Rect::from_min_max(
            egui::pos2(rect.left() + first.x, rect.top()),
            egui::pos2(
                (rect.left() + cols.order.x - 2.0).max(rect.left() + first.x),
                rect.bottom(),
            ),
        );
        let row = ui
            .interact(
                click_rect,
                ui.id().with(("style-row", name)),
                egui::Sense::click(),
            )
            .on_hover_cursor(egui::CursorIcon::PointingHand);
        row.widget_info(|| {
            egui::WidgetInfo::selected(egui::WidgetType::CollapsingHeader, true, is_expanded, name)
        });

        let left_to_right = Layout::left_to_right(Align::Center);
        let vis = cols.vis.rect(rect);
        cell_at(ui, "vis", vis, left_to_right, |ui| {
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

        let name_cell = cols.name.rect(rect);
        cell_at(ui, "name", name_cell, left_to_right, |ui| {
            if reorder.is_some() {
                rename_field(ui, state, cfg, ctx, name, name_cell.width(), actions);
            } else {
                // Left-aligned at the text inset of the custom rows' name fields, so the Name
                // column reads as one edge in both groups.
                ui.add_space(NAME_TEXT_INSET);
                ui.add(
                    egui::Label::new(RichText::new(name).font(small_caps(17.0)).color(pal::VALUE))
                        .truncate()
                        .show_tooltip_when_elided(false)
                        .sense(egui::Sense::hover()),
                )
                .on_hover_text(format!("{name}\n{RENAME_TIP}"));
            }
        });
        probe("row-name", name_cell);

        let icon = cols.icon.rect(rect);
        // Off and the cancel value (`MinimapIcon -1`) draw no icon.
        if let Some(m) = style.minimap.as_ref().and_then(Cancelable::value)
            && let Some(shape) = minimap::index_of(&minimap::SHAPES, m.shape.as_str())
            && let Some(colour) = minimap::index_of(&minimap::COLOURS, m.color.as_str())
        {
            ed::paint_minimap_icon(
                ui.painter(),
                &skin,
                icon.center(),
                shape,
                colour,
                minimap::icon_points(usize::from(m.size), 24.0),
                egui::Color32::WHITE,
            );
        }
        probe("row-icon", icon);

        let preview = cols.preview.rect(rect);
        let label = egui::Rect::from_center_size(preview.center(), vec2(preview.width(), 26.0));
        ed::ff_item_label(ui.painter(), label, "Item Name", label_look(style, false));
        probe("row-preview", preview);

        let order = cols.order.rect(rect);
        cell_at(
            ui,
            "order",
            order,
            Layout::right_to_left(Align::Center),
            |ui| {
                ui.spacing_mut().item_spacing.x = 4.0;
                // Built-ins draw both arrows greyed, at the same x as the custom rows' working
                // ones, so the column reads as one.
                let (can_up, can_down) = reorder.unwrap_or((false, false));
                let down = crate::skin::widgets::ff_arrow(ui, &skin, false, can_down);
                let up = crate::skin::widgets::ff_arrow(ui, &skin, true, can_up);
                probe("row-up", up.rect);
                probe("row-down", down.rect);
                let (down_clicked, up_clicked) = (down.clicked(), up.clicked());
                if reorder.is_none() {
                    down.on_hover_text(ORDER_TIP);
                    up.on_hover_text(ORDER_TIP);
                }
                if down_clicked {
                    actions.push(ConfigAction::MoveStyle {
                        name: name.to_string(),
                        up: false,
                    });
                }
                if up_clicked {
                    actions.push(ConfigAction::MoveStyle {
                        name: name.to_string(),
                        up: true,
                    });
                }
            },
        );
        probe("row-order", order);

        if cfg.scroll_to.as_deref() == Some(name) {
            row.scroll_to_me(Some(Align::Center));
            cfg.scroll_to = None;
        }
        if row.clicked() {
            cfg.set_expanded(name);
            egui::Popup::close_all(ui.ctx());
        }
        if is_expanded {
            let editor = ui.scope(|ui| {
                style_editor(ui, state, cfg, name, style, actions);
            });
            // A gold outline ties the row to the editor opened under it.
            ui.painter().rect_stroke(
                rect.union(editor.response.rect),
                0.0,
                egui::Stroke::new(1.0, pal::GOLD),
                egui::StrokeKind::Inside,
            );
        }
    });
}

/// How a style's item-label preview looks. `with_font` carries the style's font size; the table
/// previews leave it off so every row is the same height.
fn label_look(style: &Style, with_font: bool) -> ed::LabelLook {
    style_lines::label_look(&StyleView::from_style(style), with_font)
}

fn style_draft_row(
    ui: &mut egui::Ui,
    state: &ConfigState,
    cfg: &mut ConfigUi,
    mut draft: String,
    actions: &mut Vec<ConfigAction>,
) {
    use crate::skin::widgets::{ROW_HEIGHT, ROW_PAD, ff_button, ff_button_ex, ff_text_field_id};

    let hint = style_name_hint(state, &draft);
    let skin = crate::skin::of(ui.ctx());
    let mut commit = false;
    let mut discard = false;
    let (rect, _) =
        ui.allocate_exact_size(vec2(ui.available_width(), ROW_HEIGHT), egui::Sense::hover());
    crate::skin::widgets::row_background(ui, 0, rect);
    let mut row = ui.new_child(
        egui::UiBuilder::new()
            .id_salt("cfg-style-draft-row")
            .max_rect(rect.shrink2(vec2(ROW_PAD, 0.0)))
            .layout(Layout::right_to_left(Align::Center)),
    );
    if ff_button(&mut row, &skin, "Discard new style", 168.0).clicked() {
        discard = true;
    }
    if ff_button_ex(&mut row, &skin, "Add style", 112.0, hint.is_none(), false).clicked() {
        commit = true;
    }
    let width = (row.available_width() - 8.0).max(80.0);
    let id = row.id().with("cfg-style-draft");
    let mut edit = ff_text_field_id(&mut row, id, &mut draft, "Style name", width);
    if draft.trim().is_empty()
        && let Some(msg) = &hint
    {
        edit = edit.on_hover_text(msg.as_str());
    }
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
    }
    if commit && hint.is_none() {
        let name = draft.trim().to_string();
        // The AddStyle handler expands the new row (WR-01: expanding here too made its toggle
        // collapse it again).
        actions.push(ConfigAction::AddStyle(name.clone()));
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

/// The `StyleEdit` a shared style line's intent stands for (alpha kept).
fn style_edit_for(intent: StyleIntent) -> StyleEdit {
    match intent {
        StyleIntent::Visibility(Vis::Hide) => StyleEdit::Action(StyleAction::Hide),
        StyleIntent::Visibility(_) => StyleEdit::Action(StyleAction::Show),
        StyleIntent::Colour { slot, value, .. } => StyleEdit::Colour {
            slot,
            value: value.map(StyleColour::from_array),
        },
        StyleIntent::Font(f) => StyleEdit::Font(f),
        StyleIntent::Minimap(m) => StyleEdit::Minimap(m),
        StyleIntent::Beam(b) => StyleEdit::Beam(b),
        StyleIntent::Sound(s) => StyleEdit::Sound(s),
        StyleIntent::DropSound(d) => StyleEdit::DropSound(d),
    }
}

fn style_editor(
    ui: &mut egui::Ui,
    state: &ConfigState,
    cfg: &mut ConfigUi,
    name: &str,
    style: &Style,
    actions: &mut Vec<ConfigAction>,
) {
    let skin = crate::skin::of(ui.ctx());
    ui.spacing_mut().item_spacing = vec2(0.0, 0.0);
    let mut n = 0usize;
    // The remembered None-values outlive a fold (only Revert / Reload / Apply clear them).
    let memo = egui::Id::new(("cfg-style-last", name, cfg.memo_epoch));
    let view = StyleView::from_style(style);
    for intent in style_lines::show(ui, &view, memo, &mut n) {
        edit(actions, name, style_edit_for(intent));
    }

    if let Some(err) = state.styles_error() {
        ui.add(egui::Label::new(RichText::new(err).font(regular(15.0)).color(pal::WARN)).wrap());
    }

    if delete_visible(state, name) {
        ui.add_space(12.0);
        ui.horizontal(|ui| {
            ui.add_space(crate::skin::widgets::ROW_PAD);
            let r =
                crate::skin::widgets::ff_button_ex(ui, &skin, "Delete style", 128.0, true, true);
            probe("delete-style", r.rect);
            if r.clicked() {
                cfg.arm_delete(name);
            }
        });
    }
}

// ---------------------------------------------------------------------------------------------
// Status, footer, load error
// ---------------------------------------------------------------------------------------------

/// The load-failure view: the message and a way to retry.
pub fn show_load_error(ui: &mut egui::Ui, message: &str) -> Option<ConfigAction> {
    ui.add(
        egui::Label::new(
            RichText::new(message)
                .size(crate::skin::fonts::SUMMARY_SIZE)
                .color(pal::WARN),
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
    use crate::models::config::{Beam, FilterColor, Minimap, MinimapShape, Sound};
    use crate::overlay::style_lines::{
        FOLDER_HEADER, MINIMAP_SIZES, SoundItem, safe_file_name, sound_for_item, sound_menu,
    };
    use std::fs;

    #[test]
    fn new_styles_drop_rename_drafts_and_confirms() {
        let mut cfg = ConfigUi {
            delete_armed: Some("kite".to_string()),
            revert_armed: true,
            ..ConfigUi::default()
        };
        cfg.rename_drafts
            .insert("t5hw2k".to_string(), "Recycle".to_string());
        let epoch = cfg.epoch;
        cfg.reset_for_new_styles();
        assert_ne!(
            cfg.epoch, epoch,
            "open pickers and remembered values are orphaned"
        );
        assert!(cfg.rename_drafts.is_empty());
        assert!(cfg.delete_armed.is_none() && !cfg.revert_armed);
    }

    fn temp_root() -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        for (src, dst) in [
            ("tests/fixtures/seed-styles.json", "styles.json"),
            ("tiers-default.json", "tiers.json"),
        ] {
            fs::copy(
                Path::new(env!("CARGO_MANIFEST_DIR")).join(src),
                dir.path().join(dst),
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
        crate::skin::fonts::install(&ctx);
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
            active_filter_selected: true,
            game_dir: None,
            game_dir_missing: false,
            game_dir_pinned: false,
            default_game_dir: None,
            folder_dialog_open: false,
            rule_users: users,
            config_refs: refs,
            changed_on_disk: false,
            apply_enabled: true,
            rules_blocking: false,
            filter_damaged: false,
            indicator: "",
            reload_after_apply: true,
            skin: None,
        }
    }

    /// Every action produced by clicking, one point at a time, over the Filter section's two
    /// rows (headless, so the buttons are plain egui buttons).
    fn actions_over_rows(set: impl Fn(&mut ConfigViewCtx)) -> Vec<ConfigAction> {
        let (_d, state) = loaded();
        let users = HashMap::new();
        let refs = BTreeSet::new();
        let filters = vec!["dynamic.filter".to_string()];
        let default_dir = Path::new("C:/default/game");
        let mut found = Vec::new();
        for y in (30..170).step_by(6) {
            for x in (200..456).step_by(6) {
                let mut vctx = view_ctx(Some(filters.as_slice()), &users, &refs);
                vctx.game_dir = Some(Path::new("C:/game"));
                vctx.default_game_dir = Some(default_dir);
                set(&mut vctx);
                let (ctx, input) = headless();
                let mut cfg = ConfigUi::default();
                ctx.run_ui(input, |ui| {
                    show_sections(ui, &state, &mut cfg, &vctx);
                })
                .drop_without_applying_deltas();
                let pos = egui::pos2(x as f32, y as f32);
                let click = egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        vec2(460.0, 600.0),
                    )),
                    events: vec![
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
                    ],
                    ..Default::default()
                };
                ctx.run_ui(click, |ui| {
                    found.extend(show_sections(ui, &state, &mut cfg, &vctx));
                })
                .drop_without_applying_deltas();
            }
        }
        found
    }

    #[test]
    fn browse_and_refresh_push_their_actions() {
        let found = actions_over_rows(|_| {});
        assert!(found.contains(&ConfigAction::BrowseFolder));
        assert!(found.contains(&ConfigAction::RefreshLists));
    }

    #[test]
    fn browse_is_off_when_pinned_or_while_the_dialog_is_open() {
        for set in [
            (|c: &mut ConfigViewCtx| c.game_dir_pinned = true) as fn(&mut ConfigViewCtx),
            |c| c.folder_dialog_open = true,
        ] {
            let found = actions_over_rows(set);
            assert!(!found.contains(&ConfigAction::BrowseFolder));
            assert!(found.contains(&ConfigAction::RefreshLists));
        }
    }

    #[test]
    fn a_missing_folder_keeps_browse_and_refresh_live() {
        let found = actions_over_rows(|c| {
            c.game_dir_missing = true;
            c.filters = None;
        });
        assert!(found.contains(&ConfigAction::BrowseFolder));
        assert!(found.contains(&ConfigAction::RefreshLists));
    }

    #[test]
    fn refresh_is_off_without_a_game_folder() {
        let found = actions_over_rows(|c| {
            c.game_dir = None;
            c.filters = None;
        });
        assert!(!found.contains(&ConfigAction::RefreshLists));
    }

    #[test]
    fn the_removed_captions_stay_removed() {
        let src = include_str!("config_view.rs");
        let old_caption = ["GAME FOLDER", " FILTER"].concat();
        let old_default = ["Default: dynamic", ".filter"].concat();
        assert_eq!(src.matches(old_caption.as_str()).count(), 0);
        assert_eq!(src.matches(old_default.as_str()).count(), 0);
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
    fn open_drafts_render() {
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
        let vctx = view_ctx(Some(&filters), &users, &refs);
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
    fn tiers_section_draws_every_tier_and_an_unchanged_frame_acts_on_nothing() {
        let (_d, state) = loaded();
        assert_eq!(state.tier_rows().len(), 7);
        let (ctx, input) = headless();
        let skin = crate::skin::textures::load_light(&ctx);
        let mut actions = None;
        ctx.run_ui(input, |ui| {
            actions = Some(show_tiers_section(ui, &skin, &state));
        })
        .drop_without_applying_deltas();
        assert_eq!(actions, Some(vec![]));
    }

    #[test]
    fn tier_boxes_are_narrow_and_sit_at_the_value_column() {
        let (_d, state) = loaded();
        let (ctx, input) = headless();
        let skin = crate::skin::textures::load_light(&ctx);
        let mut area = egui::Rect::NOTHING;
        ctx.run_ui(input, |ui| {
            area = ui.max_rect();
            show_tiers_section(ui, &skin, &state);
        })
        .drop_without_applying_deltas();
        // Per row: the edit inside the field (10pt inner padding each side), then the unit label.
        let mut rects: Vec<egui::Rect> = ctx.viewport(|v| {
            v.prev_pass
                .widgets
                .layers()
                .flat_map(|(_, w)| w.iter())
                .filter(|w| w.sense.senses_click() && w.sense.senses_drag())
                .map(|w| w.rect)
                .collect()
        });
        // The edit and its unit share a row centre (the unit is as tall as the field frame).
        rects.sort_by(|a, b| {
            let key = |r: &egui::Rect| (r.center().y.round(), r.min.x);
            key(a).partial_cmp(&key(b)).unwrap()
        });
        assert_eq!(rects.len(), 14, "{rects:?}");
        let pad = crate::skin::widgets::ROW_PAD;
        let value_x = area.max.x - pad - crate::skin::widgets::CONTROL_WIDTH;
        for pair in rects.chunks(2) {
            let (field, unit) = (pair[0].expand2(vec2(10.0, 0.0)), pair[1]);
            assert!((field.width() - TIER_FIELD_W).abs() < 1.0, "{field:?}");
            assert!(
                (field.min.x - value_x).abs() < 1.0,
                "{field:?} vs {value_x}"
            );
            assert!(
                (unit.min.x - field.max.x - TIER_UNIT_GAP).abs() < 1.0,
                "{field:?} {unit:?}"
            );
            assert!(unit.max.x < area.max.x - pad, "the unit fits: {unit:?}");
        }
    }

    #[test]
    fn config_sections_no_longer_draw_tiers() {
        let src = include_str!("config_view.rs").replace("\r\n", "\n");
        let body = &src[src.find("pub fn show_sections(").unwrap()..];
        let body = &body[..body.find("\n}\n").unwrap()];
        assert!(!body.contains("show_tiers") && !body.contains("\"Tiers\""));
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

    /// The probe rects of one tag from a single rendering of the styles section.
    fn render_probes(state: &ConfigState, cfg: &mut ConfigUi, tag: &str) -> Vec<egui::Rect> {
        render_probes_with_folder(state, cfg, tag, None)
    }

    /// [`render_probes`] with the sound folder listing cached from `folder` first.
    fn render_probes_with_folder(
        state: &ConfigState,
        cfg: &mut ConfigUi,
        tag: &str,
        folder: Option<&std::path::Path>,
    ) -> Vec<egui::Rect> {
        let users = HashMap::new();
        let refs = BTreeSet::new();
        let vctx = view_ctx(None, &users, &refs);
        let (ctx, _) = headless();
        if let Some(dir) = folder {
            crate::overlay::sound_files::set_dir(&ctx, Some(dir.to_path_buf()));
        }
        let mut actions = Vec::new();
        drain_probes();
        ctx.run_ui(tall_input(), |ui| {
            show_styles(ui, state, cfg, &vctx, &mut actions);
        })
        .drop_without_applying_deltas();
        drain_probes()
            .into_iter()
            .filter(|(t, _)| *t == tag)
            .map(|(_, r)| r)
            .collect()
    }

    #[test]
    fn style_switch_click_emits_hide() {
        let (_d, state) = loaded();
        let (builtin, _) = style_groups(&state);
        let first = builtin[0].clone();
        assert_eq!(state.style(&first).unwrap().action, StyleAction::Show);
        let mut cfg = ConfigUi::default();
        let switches = render_probes(&state, &mut cfg, "row-vis");
        let (ctx, _) = headless();
        let actions = click_styles(
            &ctx,
            &state,
            &mut cfg,
            switches[0].left_center() + vec2(16.0, 0.0),
        );
        let edits: Vec<_> = actions
            .iter()
            .filter(|a| matches!(a, ConfigAction::EditStyle { .. }))
            .collect();
        assert_eq!(
            edits,
            [&ConfigAction::EditStyle {
                name: first,
                edit: StyleEdit::Action(StyleAction::Hide),
            }]
        );
        assert_eq!(cfg.expanded, None, "switch click must not expand the row");
    }

    #[test]
    fn label_look_reads_the_style() {
        let (_d, state) = loaded();
        let s = label_look(state.style("S").unwrap(), false);
        assert_eq!(s.text, Some(egui::Color32::from_rgb(255, 121, 198)));
        assert_eq!(s.border, Some(egui::Color32::from_rgb(255, 121, 198)));
        assert_eq!(s.background, Some(egui::Color32::from_rgb(40, 42, 54)));
        assert_eq!((s.font_size, s.hidden), (None, false));
        let with_font = label_look(state.style("S").unwrap(), true);
        assert_eq!(
            with_font.font_size,
            state.style("S").unwrap().font.map(f32::from)
        );
        assert!(with_font.font_size.is_some());
        let h = label_look(state.style("hidden").unwrap(), false);
        assert!(h.hidden);
        assert_eq!((h.text, h.border, h.background), (None, None, None));
    }

    fn mesh_rects(shape: &egui::Shape, out: &mut Vec<egui::Rect>) {
        match shape {
            egui::Shape::Mesh(m) => out.push(m.calc_bounds()),
            egui::Shape::Vec(v) => v.iter().for_each(|s| mesh_rects(s, out)),
            _ => {}
        }
    }

    #[test]
    fn a_style_with_a_minimap_paints_its_icon_in_the_icon_cell() {
        let (_d, state) = loaded();
        assert!(state.style("S").unwrap().minimap.is_some());
        assert!(state.style("unchanged").unwrap().minimap.is_none());
        let users = HashMap::new();
        let refs = BTreeSet::new();
        let vctx = view_ctx(None, &users, &refs);
        let (ctx, _) = headless();
        let mut cfg = ConfigUi::default();
        let mut actions = Vec::new();
        drain_probes();
        let out = ctx.run_ui(tall_input(), |ui| {
            show_styles(ui, &state, &mut cfg, &vctx, &mut actions);
        });
        let icons: Vec<_> = drain_probes()
            .into_iter()
            .filter(|(t, _)| *t == "row-icon")
            .map(|(_, r)| r)
            .collect();
        let mut meshes = Vec::new();
        for clipped in &out.shapes {
            mesh_rects(&clipped.shape, &mut meshes);
        }
        out.drop_without_applying_deltas();
        let painted_in = |cell: egui::Rect| {
            meshes
                .iter()
                .filter(|m| cell.expand(1.0).contains_rect(**m))
                .count()
        };
        let (builtin, _) = style_groups(&state);
        let row = |n: &str| builtin.iter().position(|x| x == n).unwrap();
        assert_eq!(painted_in(icons[row("S")]), 1);
        assert_eq!(painted_in(icons[row("unchanged")]), 0);
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

    fn drain_probes() -> Vec<(&'static str, egui::Rect)> {
        style_lines::drain_probes()
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

    /// Run `show_styles` once, then again with a click at the point `click` picks from the first
    /// frame's probes; returns those probes and the click frame's actions.
    fn styles_frames(
        state: &ConfigState,
        click: impl Fn(&[(&'static str, egui::Rect)]) -> Option<egui::Pos2>,
    ) -> (Vec<(&'static str, egui::Rect)>, Vec<ConfigAction>) {
        let users = HashMap::new();
        let refs = BTreeSet::new();
        let (ctx, _) = headless();
        let mut cfg = ConfigUi::default();
        let vctx = view_ctx(None, &users, &refs);
        let mut actions = Vec::new();
        drain_probes();
        ctx.run_ui(tall_input(), |ui| {
            show_styles(ui, state, &mut cfg, &vctx, &mut actions);
        })
        .drop_without_applying_deltas();
        let probes = drain_probes();
        actions.clear();
        if let Some(pos) = click(&probes) {
            let ev = |pressed| egui::Event::PointerButton {
                pos,
                button: egui::PointerButton::Primary,
                pressed,
                modifiers: Default::default(),
            };
            let mut input = tall_input();
            input.events = vec![egui::Event::PointerMoved(pos), ev(true), ev(false)];
            ctx.run_ui(input, |ui| {
                show_styles(ui, state, &mut cfg, &vctx, &mut actions);
            })
            .drop_without_applying_deltas();
        }
        (probes, actions)
    }

    /// Centre of segment `index` of the strictness row, from the widget's own layout.
    fn segment_centre(probes: &[(&'static str, egui::Rect)], index: usize) -> egui::Pos2 {
        let row = probes
            .iter()
            .find(|(t, _)| *t == "strictness-row")
            .unwrap()
            .1;
        let (ctx, _) = headless();
        let mut nat = Vec::new();
        ctx.run_ui(tall_input(), |ui| {
            for l in crate::overlay::strictness::Strictness::ALL {
                let widest = l
                    .caption_lines()
                    .iter()
                    .map(|t| {
                        ui.painter()
                            .layout_no_wrap((*t).to_owned(), small_caps(14.0), pal::LABEL)
                            .size()
                            .x
                    })
                    .fold(0.0, f32::max);
                nat.push(widest);
            }
        })
        .drop_without_applying_deltas();
        let widths = ed::levels_layout(row.width(), &nat);
        let x = row.min.x + widths[..index].iter().sum::<f32>() + widths[index] / 2.0;
        egui::pos2(x, row.center().y)
    }

    #[test]
    fn strictness_row_sits_above_the_built_in_group_and_selects_flat_broke() {
        use crate::overlay::strictness::Strictness;
        let (_d, state) = loaded();
        assert_eq!(state.strictness(), Strictness::FlatBroke);
        let (probes, _) = styles_frames(&state, |_| None);
        let rows: Vec<_> = probes
            .iter()
            .filter(|(t, _)| *t == "strictness-row")
            .collect();
        assert_eq!(rows.len(), 1);
        let heading = probes
            .iter()
            .find(|(t, _)| *t == "builtin-heading")
            .unwrap()
            .1;
        assert!(rows[0].1.bottom() <= heading.top());
    }

    #[test]
    fn clicking_a_level_emits_set_strictness_once() {
        use crate::overlay::strictness::Strictness;
        let (_d, state) = loaded();
        let idx = Strictness::ALL
            .iter()
            .position(|l| *l == Strictness::Comfy)
            .unwrap();
        let (_, actions) = styles_frames(&state, |p| Some(segment_centre(p, idx)));
        assert_eq!(
            actions,
            vec![ConfigAction::SetStrictness(Strictness::Comfy)]
        );
    }

    #[test]
    fn clicking_the_selected_level_or_custom_emits_nothing() {
        let (_d, state) = loaded();
        // Flat Broke is selected (index 1); Custom is index 0.
        for idx in [0, 1] {
            let (_, actions) = styles_frames(&state, |p| Some(segment_centre(p, idx)));
            assert!(actions.is_empty(), "{idx}: {actions:?}");
        }
    }

    #[test]
    fn a_manual_switch_flip_reads_as_custom() {
        use crate::overlay::strictness::Strictness;
        let (_d, mut state) = loaded();
        state.edit_style("E", StyleEdit::Action(StyleAction::Hide));
        assert_eq!(state.strictness(), Strictness::Custom);
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
            assert!(
                t != "Show" && t != "Hide",
                "no Show/Hide caption in a row: {t}"
            );
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
    fn table_columns_follow_the_spec() {
        let c = table_columns(517.0);
        assert_eq!((c.vis.x, c.vis.w), (10.0, 64.0));
        assert_eq!((c.name.x, c.name.w), (82.0, 153.0));
        assert_eq!((c.icon.x, c.icon.w), (243.0, 32.0));
        assert_eq!((c.preview.x, c.preview.w), (283.0, 160.0));
        assert_eq!((c.order.x, c.order.w), (451.0, 56.0));
        assert_eq!(c.order.x + c.order.w, 517.0 - 10.0);
        for width in [440.0, 460.0, 480.0, 517.0, 660.0] {
            let c = table_columns(width);
            assert!(c.name.w >= 110.0, "{width}: {c:?}");
            assert!(
                c.order.x + c.order.w <= width - 10.0 + 0.01,
                "{width}: {c:?}"
            );
            for pair in [
                (c.vis, c.name),
                (c.name, c.icon),
                (c.icon, c.preview),
                (c.preview, c.order),
            ] {
                assert!((pair.1.x - (pair.0.x + pair.0.w) - COL_GAP).abs() < 0.01);
            }
        }
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
        let total = state.style_names().len();
        let n_builtin = style_groups(&state).0.len();
        // One header per group with all five columns; each lines up with the same column of every
        // row below it (both groups).
        for col in ["vis", "name", "icon", "preview", "order"] {
            let heads = of(&format!("head-{col}"));
            let rows = of(&format!("row-{col}"));
            assert_eq!(heads.len(), 2, "head-{col}");
            assert_eq!(rows.len(), total, "row-{col}");
            assert!(
                (heads[0].left() - heads[1].left()).abs() <= 0.5,
                "head-{col} differs between groups"
            );
            for (k, r) in rows.iter().enumerate() {
                let head = if k < n_builtin { heads[0] } else { heads[1] };
                assert!(
                    (r.left() - head.left()).abs() <= 0.5
                        && (r.right() - head.right()).abs() <= 0.5,
                    "row-{col} {r:?} vs header {head:?}"
                );
            }
        }
        let h = |c: &str| of(&format!("head-{c}"))[0];
        assert!(h("vis").right() <= h("name").left());
        assert!(h("name").right() <= h("icon").left());
        assert!(h("icon").right() <= h("preview").left());
        assert!(h("preview").right() <= h("order").left());
        // Every row's arrows sit inside the Order cell, Up then Down, Down at its right edge.
        let (orders, ups, downs) = (of("row-order"), of("row-up"), of("row-down"));
        for ((u, d), k) in ups.iter().zip(&downs).zip(0..) {
            let o = orders[k];
            assert!(u.center().x < d.center().x);
            assert!(
                o.contains(u.center()) && o.contains(d.center()),
                "{u:?} {d:?} {o:?}"
            );
            assert!((d.center().x + 8.0 - o.right()).abs() <= 1.0, "{d:?} {o:?}");
        }
    }

    #[test]
    fn styles_layout_runs_heading_then_header_then_rows_per_group() {
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
        let ys = |tag: &str| -> Vec<f32> {
            probes
                .iter()
                .filter(|(t, _)| *t == tag)
                .map(|(_, r)| r.top())
                .collect()
        };
        let builtin_heading = ys("builtin-heading")[0];
        let heads = ys("head-vis");
        let rows = ys("row-vis");
        let custom_heading = ys("custom-heading")[0];
        let n_builtin = style_groups(&state).0.len();
        assert!(builtin_heading < heads[0]);
        assert!(heads[0] < rows[0]);
        assert!(rows[n_builtin - 1] < custom_heading);
        assert!(custom_heading < heads[1]);
        assert!(heads[1] < rows[n_builtin]);
    }

    #[test]
    fn with_no_custom_styles_the_custom_header_is_omitted() {
        let (_d, mut state) = loaded();
        for name in style_groups(&state).1 {
            assert!(state.delete_style(&name));
        }
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
        let count = |tag: &str| probes.iter().filter(|(t, _)| *t == tag).count();
        assert_eq!(count("custom-heading"), 1);
        assert_eq!(count("add-style"), 1);
        assert_eq!(count("head-vis"), 1, "only the built-in header");
        assert_eq!(count("head-order"), 1, "Built-in also has an Order column");
    }

    #[test]
    fn rename_hints_follow_the_add_rules() {
        let (_d, mut state) = loaded();
        state.add_style("k1").unwrap();
        state.add_style("k2").unwrap();
        assert_eq!(rename_hint(&state, "k1", "k1"), None);
        assert_eq!(rename_hint(&state, "k1", "locator"), None);
        assert_eq!(
            rename_hint(&state, "k1", "").as_deref(),
            Some("Enter a style name")
        );
        assert_eq!(
            rename_hint(&state, "k1", "k2").as_deref(),
            Some("A style with this name already exists")
        );
        assert_eq!(
            rename_hint(&state, "k1", "HIDDEN").as_deref(),
            Some("That name is reserved for the built-in style hidden")
        );
        assert!(rename_hint(&state, "k1", "a\tb").is_some());
    }

    /// Drives the first custom row's Name field: click into it, type, then press a key.
    fn type_into_first_custom(
        state: &ConfigState,
        cfg: &mut ConfigUi,
        typed: &str,
        finish: egui::Key,
    ) -> Vec<ConfigAction> {
        let users = HashMap::new();
        let refs = BTreeSet::new();
        let vctx = view_ctx(None, &users, &refs);
        let (ctx, _) = headless();
        let mut actions = Vec::new();
        drain_probes();
        ctx.run_ui(tall_input(), |ui| {
            show_styles(ui, state, cfg, &vctx, &mut actions);
        })
        .drop_without_applying_deltas();
        let n_builtin = style_groups(state).0.len();
        let names: Vec<_> = drain_probes()
            .into_iter()
            .filter(|(t, _)| *t == "row-name")
            .map(|(_, r)| r)
            .collect();
        // Click near the right end so the caret lands after the text.
        let target = egui::pos2(names[n_builtin].right() - 14.0, names[n_builtin].center().y);
        let button = |pressed| egui::Event::PointerButton {
            pos: target,
            button: egui::PointerButton::Primary,
            pressed,
            modifiers: Default::default(),
        };
        let mut frame = |events: Vec<egui::Event>, actions: &mut Vec<ConfigAction>| {
            let mut input = tall_input();
            input.events = events;
            ctx.run_ui(input, |ui| {
                show_styles(ui, state, cfg, &vctx, actions);
            })
            .drop_without_applying_deltas();
        };
        frame(vec![egui::Event::PointerMoved(target)], &mut actions);
        frame(vec![button(true), button(false)], &mut actions);
        frame(vec![egui::Event::Text(typed.to_string())], &mut actions);
        assert!(actions.is_empty(), "typing must not emit an action");
        frame(
            vec![egui::Event::Key {
                key: finish,
                physical_key: Some(finish),
                pressed: true,
                repeat: false,
                modifiers: Default::default(),
            }],
            &mut actions,
        );
        frame(vec![], &mut actions);
        actions
    }

    fn renames(actions: &[ConfigAction]) -> Vec<&ConfigAction> {
        actions
            .iter()
            .filter(|a| matches!(a, ConfigAction::RenameStyle { .. }))
            .collect()
    }

    #[test]
    fn enter_commits_a_valid_rename_and_the_draft_survives_typing() {
        let (_d, mut state) = loaded();
        state.add_style("k1").unwrap();
        let mut cfg = ConfigUi::default();
        let actions = type_into_first_custom(&state, &mut cfg, "x", egui::Key::Enter);
        let first = style_groups(&state).1[0].clone();
        // The test never applies the action, so a still-pending draft may be re-offered on the
        // next frame; the app applies it after the first, which makes the second a no-op.
        let expected = ConfigAction::RenameStyle {
            from: first.clone(),
            to: format!("{first}x"),
        };
        let got = renames(&actions);
        assert!(
            !got.is_empty() && got.iter().all(|a| **a == expected),
            "{got:?}"
        );
        assert_eq!(
            cfg.expanded, None,
            "the field click must not expand the row"
        );
        let id = state.style_id(&first).unwrap();
        assert_eq!(
            cfg.rename_drafts.get(id).map(String::as_str),
            Some(format!("{first}x").as_str()),
            "the draft is kept, keyed by id, until the app applies the rename"
        );
    }

    #[test]
    fn an_invalid_rename_is_not_emitted_and_the_draft_is_kept() {
        let (_d, mut state) = loaded();
        state.add_style("k1").unwrap();
        let first = style_groups(&state).1[0].clone();
        // Typing a control character makes the draft invalid.
        let mut cfg = ConfigUi::default();
        let actions = type_into_first_custom(&state, &mut cfg, "\u{1}", egui::Key::Enter);
        assert!(renames(&actions).is_empty());
        let _ = first;
    }

    #[test]
    fn escape_restores_the_name() {
        let (_d, mut state) = loaded();
        state.add_style("k1").unwrap();
        let mut cfg = ConfigUi::default();
        let actions = type_into_first_custom(&state, &mut cfg, "zz", egui::Key::Escape);
        assert!(renames(&actions).is_empty());
        assert!(cfg.rename_drafts.is_empty(), "{:?}", cfg.rename_drafts);
    }

    #[test]
    fn built_in_names_are_not_editable() {
        let (_d, state) = loaded();
        let users = HashMap::new();
        let refs = BTreeSet::new();
        let vctx = view_ctx(None, &users, &refs);
        let (ctx, _) = headless();
        let mut cfg = ConfigUi::default();
        let mut actions = Vec::new();
        drain_probes();
        ctx.run_ui(tall_input(), |ui| {
            show_styles(ui, &state, &mut cfg, &vctx, &mut actions);
        })
        .drop_without_applying_deltas();
        let first = drain_probes()
            .into_iter()
            .find(|(t, _)| *t == "row-name")
            .unwrap()
            .1;
        let target = first.center();
        let click = |pressed| egui::Event::PointerButton {
            pos: target,
            button: egui::PointerButton::Primary,
            pressed,
            modifiers: Default::default(),
        };
        for events in [
            vec![egui::Event::PointerMoved(target)],
            vec![click(true), click(false)],
            vec![egui::Event::Text("x".to_string())],
        ] {
            let mut input = tall_input();
            input.events = events;
            ctx.run_ui(input, |ui| {
                show_styles(ui, &state, &mut cfg, &vctx, &mut actions);
            })
            .drop_without_applying_deltas();
        }
        assert!(renames(&actions).is_empty());
        assert!(cfg.rename_drafts.is_empty());
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

    fn loaded_edited() -> (tempfile::TempDir, ConfigState) {
        let (d, mut state) = loaded();
        state.reset_styles();
        let (builtin, _) = style_groups(&state);
        assert!(state.edit_style(&builtin[0], StyleEdit::Action(StyleAction::Hide)));
        (d, state)
    }

    #[test]
    fn reset_styles_button_is_greyed_at_defaults_and_arms_when_edited() {
        let (_d, mut state) = loaded();
        state.reset_styles();
        assert!(state.styles_at_defaults());
        let mut cfg = ConfigUi::default();
        let at_defaults = render_probes(&state, &mut cfg, "reset-styles");
        assert_eq!(at_defaults.len(), 1);
        let (ctx, _) = headless();
        click_styles(&ctx, &state, &mut cfg, at_defaults[0].center());
        assert_eq!(cfg.reset_armed, None, "a disabled button must not arm");

        let (_d2, state) = loaded_edited();
        let mut cfg = ConfigUi::default();
        let rects = render_probes(&state, &mut cfg, "reset-styles");
        let (ctx, _) = headless();
        let actions = click_styles(&ctx, &state, &mut cfg, rects[0].center());
        assert_eq!(cfg.reset_armed, Some(RevertScope::Styles));
        assert!(
            !actions
                .iter()
                .any(|a| matches!(a, ConfigAction::ResetStyles)),
            "the click only arms the confirmation"
        );
        cfg.disarm_all();
        assert_eq!(cfg.reset_armed, None);
    }

    fn moves(actions: &[ConfigAction]) -> Vec<&ConfigAction> {
        actions
            .iter()
            .filter(|a| matches!(a, ConfigAction::MoveStyle { .. }))
            .collect()
    }

    #[test]
    fn custom_arrows_reorder_and_builtin_arrows_are_disabled() {
        let (_d, mut state) = loaded();
        for n in ["k1", "k2", "k3"] {
            state.add_style(n).unwrap();
        }
        let (builtin, custom) = style_groups(&state);
        assert_eq!(custom.last().map(String::as_str), Some("k3"));
        let mut cfg = ConfigUi::default();
        let ups = render_probes(&state, &mut cfg, "row-up");
        let downs = render_probes(&state, &mut cfg, "row-down");
        let n = builtin.len();
        assert_eq!(ups.len(), n + custom.len(), "every row draws both arrows");
        assert_eq!(downs.len(), n + custom.len());

        // Built-in arrows are disabled: a click emits nothing, and they sit at the same x as the
        // custom rows' working arrows.
        for target in [ups[0].center(), downs[0].center(), downs[n - 1].center()] {
            let (ctx, _) = headless();
            let mut cfg = ConfigUi::default();
            let clicked = click_styles(&ctx, &state, &mut cfg, target);
            assert!(moves(&clicked).is_empty(), "built-in arrow at {target:?}");
        }
        assert!((ups[0].center().x - ups[n].center().x).abs() <= 0.5);
        assert!((downs[0].center().x - downs[n].center().x).abs() <= 0.5);

        // The custom ends are disabled too.
        let (ctx, _) = headless();
        let mut cfg = ConfigUi::default();
        let first_up = click_styles(&ctx, &state, &mut cfg, ups[n].center());
        assert!(
            moves(&first_up).is_empty(),
            "the first custom Up is disabled"
        );
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

    // ---- style editor (Phase 33 Plan 04) ----

    fn expanded(name: &str) -> ConfigUi {
        ConfigUi {
            expanded: Some(name.to_string()),
            ..Default::default()
        }
    }

    fn edits_of(actions: &[ConfigAction]) -> Vec<StyleEdit> {
        actions
            .iter()
            .filter_map(|a| match a {
                ConfigAction::EditStyle { edit, .. } => Some(edit.clone()),
                _ => None,
            })
            .collect()
    }

    /// Clicks the centre of the `idx`-th probe `tag` (shifted by `dx`) of the expanded editor.
    fn click_probe(
        ctx: &egui::Context,
        state: &ConfigState,
        cfg: &mut ConfigUi,
        tag: &str,
        idx: usize,
        dx: f32,
    ) -> Vec<ConfigAction> {
        let rects = render_probes(state, cfg, tag);
        assert!(rects.len() > idx, "no probe {tag}[{idx}]");
        click_styles(ctx, state, cfg, rects[idx].center() + vec2(dx, 0.0))
    }

    #[test]
    fn idle_frames_emit_nothing_for_custom_styles_with_every_field() {
        let (_d, state) = loaded();
        let users = HashMap::new();
        let refs = BTreeSet::new();
        let (ctx, _) = headless();
        for name in ["kite", "exceptional", "Recycle", "hidden"] {
            let mut cfg = expanded(name);
            let vctx = view_ctx(None, &users, &refs);
            let mut actions = Vec::new();
            for _ in 0..2 {
                ctx.run_ui(tall_input(), |ui| {
                    show_styles(ui, &state, &mut cfg, &vctx, &mut actions);
                })
                .drop_without_applying_deltas();
            }
            assert!(actions.is_empty(), "{name}: {actions:?}");
        }
    }

    fn big_fills(shape: &egui::Shape, out: &mut Vec<egui::Rect>) {
        match shape {
            egui::Shape::Rect(r) if r.fill == pal::ROW_EVEN => out.push(r.rect),
            egui::Shape::Vec(v) => v.iter().for_each(|s| big_fills(s, out)),
            _ => {}
        }
    }

    #[test]
    fn expanded_editor_draws_no_frame_fill() {
        let (_d, state) = loaded();
        let users = HashMap::new();
        let refs = BTreeSet::new();
        let vctx = view_ctx(None, &users, &refs);
        let (ctx, _) = headless();
        let mut cfg = expanded("kite");
        let mut actions = Vec::new();
        let out = ctx.run_ui(tall_input(), |ui| {
            show_styles(ui, &state, &mut cfg, &vctx, &mut actions);
        });
        let mut fills = Vec::new();
        for clipped in &out.shapes {
            big_fills(&clipped.shape, &mut fills);
        }
        out.drop_without_applying_deltas();
        // The zebra grounds are single lines; a boxed editor would be one tall ROW_BASE rect.
        let tall = crate::skin::widgets::ROW_HEIGHT * 2.0;
        assert!(
            fills.iter().all(|r| r.height() <= tall),
            "boxed editor fill: {fills:?}"
        );
    }

    #[test]
    fn visibility_hide_emits_the_action_edit() {
        let (_d, state) = loaded();
        let (ctx, _) = headless();
        let mut cfg = expanded("S");
        let rects = render_probes(&state, &mut cfg, "ed-visibility");
        let r = rects[0];
        let acts = click_styles(
            &ctx,
            &state,
            &mut cfg,
            r.right_center() - vec2(r.width() / 4.0, 0.0),
        );
        assert_eq!(edits_of(&acts), [StyleEdit::Action(StyleAction::Hide)]);
    }

    #[test]
    fn font_check_toggles_between_none_and_32() {
        let (_d, state) = loaded();
        let (ctx, _) = headless();
        let mut cfg = expanded("S");
        let acts = click_probe(&ctx, &state, &mut cfg, "ed-font-check", 0, 0.0);
        assert_eq!(edits_of(&acts), [StyleEdit::Font(None)]);
        let mut cfg = expanded("kite");
        let acts = click_probe(&ctx, &state, &mut cfg, "ed-font-check", 0, 0.0);
        assert_eq!(edits_of(&acts), [StyleEdit::Font(Some(32))]);
    }

    /// Font size off keeps a dimmed, inert slider on the line instead of a lone check.
    #[test]
    fn font_off_shows_a_dimmed_slider_that_does_nothing() {
        let (_d, state) = loaded();
        let (ctx, _) = headless();
        let mut cfg = expanded("kite");
        assert_eq!(state.style("kite").unwrap().font, None);
        let off = render_probes(&state, &mut cfg, "ed-font-slider-off");
        assert_eq!(off.len(), 1, "the off line keeps its slider");
        assert!(off[0].width() >= 120.0, "{:?}", off[0]);
        let acts = click_probe(&ctx, &state, &mut cfg, "ed-font-slider-off", 0, 20.0);
        assert!(edits_of(&acts).is_empty(), "{acts:?}");
        let mut cfg = expanded("S");
        assert!(render_probes(&state, &mut cfg, "ed-font-slider-off").is_empty());
        assert_eq!(render_probes(&state, &mut cfg, "ed-font-slider").len(), 1);
    }

    /// WR-03: off then on again restores the style's own size, not 32.
    #[test]
    fn font_off_then_on_restores_the_last_size() {
        let (_d, mut state) = loaded();
        let (ctx, _) = headless();
        let mut cfg = expanded("S");
        assert_eq!(state.style("S").unwrap().font, Some(37));
        let acts = click_probe(&ctx, &state, &mut cfg, "ed-font-check", 0, 0.0);
        assert_eq!(edits_of(&acts), [StyleEdit::Font(None)]);
        assert!(state.edit_style("S", StyleEdit::Font(None)));
        let acts = click_probe(&ctx, &state, &mut cfg, "ed-font-check", 0, 0.0);
        assert_eq!(edits_of(&acts), [StyleEdit::Font(Some(37))]);
    }

    /// Review iteration 2, WR-02: folding the row and opening it again keeps the remembered
    /// size; only replacing the styles (Revert / Reload / Apply) forgets it.
    #[test]
    fn the_remembered_font_size_survives_a_fold() {
        let (_d, mut state) = loaded();
        let (ctx, _) = headless();
        let mut cfg = expanded("S");
        let acts = click_probe(&ctx, &state, &mut cfg, "ed-font-check", 0, 0.0);
        assert_eq!(edits_of(&acts), [StyleEdit::Font(None)]);
        assert!(state.edit_style("S", StyleEdit::Font(None)));
        cfg.set_expanded("S");
        cfg.set_expanded("S");
        assert_eq!(cfg.expanded.as_deref(), Some("S"));
        let acts = click_probe(&ctx, &state, &mut cfg, "ed-font-check", 0, 0.0);
        assert_eq!(edits_of(&acts), [StyleEdit::Font(Some(37))]);
        // Replacing the styles forgets it.
        assert!(state.edit_style("S", StyleEdit::Font(None)));
        cfg.reset_for_new_styles();
        let acts = click_probe(&ctx, &state, &mut cfg, "ed-font-check", 0, 0.0);
        assert_eq!(edits_of(&acts), [StyleEdit::Font(Some(32))]);
    }

    #[test]
    fn drop_sound_check_emits_the_negation() {
        let (_d, state) = loaded();
        let (ctx, _) = headless();
        let mut cfg = expanded("S");
        let acts = click_probe(&ctx, &state, &mut cfg, "ed-drop-check", 0, 0.0);
        assert_eq!(
            edits_of(&acts),
            [StyleEdit::DropSound(Some(
                crate::models::config::DropSound::Silent
            ))]
        );
        let mut cfg = expanded("hidden");
        let acts = click_probe(&ctx, &state, &mut cfg, "ed-drop-check", 0, 0.0);
        assert_eq!(edits_of(&acts), [StyleEdit::DropSound(None)]);
    }

    #[test]
    fn only_custom_styles_get_a_delete_line_that_arms_the_popup() {
        let (_d, state) = loaded();
        let mut cfg = expanded("S");
        assert!(render_probes(&state, &mut cfg, "delete-style").is_empty());
        let (ctx, _) = headless();
        let mut cfg = expanded("kite");
        let rects = render_probes(&state, &mut cfg, "delete-style");
        assert_eq!(rects.len(), 1);
        let acts = click_styles(&ctx, &state, &mut cfg, rects[0].center());
        assert!(edits_of(&acts).is_empty(), "{acts:?}");
        assert_eq!(cfg.delete_armed.as_deref(), Some("kite"));
    }

    #[test]
    fn editor_lines_use_the_field_labels() {
        let (_d, state) = loaded();
        let users = HashMap::new();
        let refs = BTreeSet::new();
        let vctx = view_ctx(None, &users, &refs);
        let (ctx, _) = headless();
        let mut cfg = expanded("S");
        let mut actions = Vec::new();
        let out = ctx.run_ui(tall_input(), |ui| {
            show_styles(ui, &state, &mut cfg, &vctx, &mut actions);
        });
        let mut texts = Vec::new();
        for clipped in &out.shapes {
            shape_texts(&clipped.shape, &mut texts);
        }
        out.drop_without_applying_deltas();
        for label in ["Visibility", "Label", "Font size", "Drop sound"] {
            assert!(
                texts.iter().any(|t| t == label),
                "missing {label}: {texts:?}"
            );
        }
        assert!(!texts.iter().any(|t| t == "NAME" || t.contains("Not set")));
    }

    fn mm(size: u8, color: FilterColor, shape: MinimapShape) -> StyleEdit {
        StyleEdit::Minimap(Some(Cancelable::Set(Minimap { size, color, shape })))
    }

    /// Click offset (from the segmented control's centre) onto segment `i` of `n`.
    fn segment_dx(rect: egui::Rect, i: usize, n: usize) -> f32 {
        rect.width() * ((i as f32 + 0.5) / n as f32 - 0.5)
    }

    fn colour_index(name: &str) -> usize {
        minimap::index_of(&minimap::COLOURS, name).unwrap()
    }

    #[test]
    fn minimap_off_is_one_line_and_ticking_seeds_white_circle() {
        let (_d, state) = loaded();
        let (ctx, _) = headless();
        let mut cfg = expanded("E");
        assert_eq!(render_probes(&state, &mut cfg, "ed-mm-check").len(), 1);
        for tag in ["ed-mm-cancel", "ed-mm-colour", "ed-mm-shape", "ed-mm-size"] {
            assert!(render_probes(&state, &mut cfg, tag).is_empty(), "{tag}");
        }
        let acts = click_probe(&ctx, &state, &mut cfg, "ed-mm-check", 0, 0.0);
        assert_eq!(
            edits_of(&acts),
            [mm(0, FilterColor::White, MinimapShape::Circle)]
        );
    }

    #[test]
    fn minimap_off_then_on_restores_the_last_values() {
        let (_d, mut state) = loaded();
        let (ctx, _) = headless();
        let mut cfg = expanded("kite");
        let acts = click_probe(&ctx, &state, &mut cfg, "ed-mm-check", 0, 0.0);
        assert_eq!(edits_of(&acts), [StyleEdit::Minimap(None)]);
        assert!(state.edit_style("kite", StyleEdit::Minimap(None)));
        assert!(render_probes(&state, &mut cfg, "ed-mm-colour").is_empty());
        let acts = click_probe(&ctx, &state, &mut cfg, "ed-mm-check", 0, 0.0);
        assert_eq!(
            edits_of(&acts),
            [mm(2, FilterColor::Orange, MinimapShape::Kite)]
        );
    }

    #[test]
    fn minimap_colour_and_shape_keep_the_other_fields() {
        let (_d, state) = loaded();
        let (ctx, _) = headless();
        let mut cfg = expanded("kite");
        let colours = render_probes(&state, &mut cfg, "ed-mm-colour");
        let shapes = render_probes(&state, &mut cfg, "ed-mm-shape");
        assert_eq!((colours.len(), shapes.len()), (11, 12));
        assert!(shapes.iter().all(|r| r.min.y == shapes[0].min.y), "one row");
        assert!(
            colours.iter().all(|r| r.min.y == colours[0].min.y),
            "one row"
        );
        let cancel = render_probes(&state, &mut cfg, "ed-mm-cancel");
        assert_eq!(cancel.len(), 1);
        assert_eq!(
            cancel[0].min.y, colours[0].min.y,
            "the / tile leads the colours"
        );
        assert!(cancel[0].max.x <= colours[0].min.x);
        let pink = colour_index("Pink");
        let acts = click_styles(&ctx, &state, &mut cfg, colours[pink].center());
        assert_eq!(
            edits_of(&acts),
            [mm(2, FilterColor::Pink, MinimapShape::Kite)]
        );
        let mut cfg = expanded("exceptional");
        let kite = minimap::index_of(&minimap::SHAPES, "Kite").unwrap();
        let shapes = render_probes(&state, &mut cfg, "ed-mm-shape");
        let acts = click_styles(&ctx, &state, &mut cfg, shapes[kite].center());
        assert_eq!(
            edits_of(&acts),
            [mm(2, FilterColor::Pink, MinimapShape::Kite)]
        );
    }

    #[test]
    fn minimap_size_is_large_medium_small() {
        let (_d, state) = loaded();
        let (ctx, _) = headless();
        let mut cfg = expanded("kite");
        assert_eq!(MINIMAP_SIZES, ["Large", "Medium", "Small"]);
        let r = render_probes(&state, &mut cfg, "ed-mm-size")[0];
        for (i, size) in [(0, 0u8), (1, 1)] {
            let acts = click_styles(
                &ctx,
                &state,
                &mut cfg,
                r.center() + vec2(segment_dx(r, i, 3), 0.0),
            );
            assert_eq!(
                edits_of(&acts),
                [mm(size, FilterColor::Orange, MinimapShape::Kite)]
            );
        }
        // Small is the current size: no edit.
        let acts = click_styles(
            &ctx,
            &state,
            &mut cfg,
            r.center() + vec2(segment_dx(r, 2, 3), 0.0),
        );
        assert!(edits_of(&acts).is_empty(), "{acts:?}");
    }

    #[test]
    fn minimap_cancel_greys_the_pickers_and_a_colour_leaves_it() {
        let (_d, mut state) = loaded();
        let (ctx, _) = headless();
        let mut cfg = expanded("kite");
        let acts = click_probe(&ctx, &state, &mut cfg, "ed-mm-cancel", 0, 0.0);
        assert_eq!(
            edits_of(&acts),
            [StyleEdit::Minimap(Some(Cancelable::Cancel))]
        );
        assert!(state.edit_style("kite", StyleEdit::Minimap(Some(Cancelable::Cancel))));
        // Shapes and sizes are drawn but inert.
        let kite = minimap::index_of(&minimap::SHAPES, "Kite").unwrap();
        let acts = click_probe(&ctx, &state, &mut cfg, "ed-mm-shape", kite + 1, 0.0);
        assert!(edits_of(&acts).is_empty(), "{acts:?}");
        let acts = click_probe(&ctx, &state, &mut cfg, "ed-mm-size", 0, 0.0);
        assert!(edits_of(&acts).is_empty(), "{acts:?}");
        // Clicking / again changes nothing; a colour restores the remembered icon with it.
        let acts = click_probe(&ctx, &state, &mut cfg, "ed-mm-cancel", 0, 0.0);
        assert!(edits_of(&acts).is_empty(), "{acts:?}");
        let pink = colour_index("Pink");
        let acts = click_probe(&ctx, &state, &mut cfg, "ed-mm-colour", pink, 0.0);
        assert_eq!(
            edits_of(&acts),
            [mm(2, FilterColor::Pink, MinimapShape::Kite)]
        );
    }

    #[test]
    fn beam_off_cancel_colour_and_temp() {
        let (_d, mut state) = loaded();
        let (ctx, _) = headless();
        let mut cfg = expanded("kite");
        let acts = click_probe(&ctx, &state, &mut cfg, "ed-beam-check", 0, 0.0);
        assert_eq!(edits_of(&acts), [StyleEdit::Beam(None)]);
        let acts = click_probe(&ctx, &state, &mut cfg, "ed-beam-cancel", 0, 0.0);
        assert_eq!(edits_of(&acts), [StyleEdit::Beam(Some(Cancelable::Cancel))]);
        let pink = colour_index("Pink");
        let acts = click_probe(&ctx, &state, &mut cfg, "ed-beam-colour", pink, 0.0);
        assert_eq!(
            edits_of(&acts),
            [StyleEdit::Beam(Some(Cancelable::Set(Beam {
                color: FilterColor::Pink,
                temp: false
            })))]
        );
        let acts = click_probe(&ctx, &state, &mut cfg, "ed-beam-temp", 0, 0.0);
        assert_eq!(
            edits_of(&acts),
            [StyleEdit::Beam(Some(Cancelable::Set(Beam {
                color: FilterColor::Orange,
                temp: true
            })))]
        );
        // Under / the Temp chip is inert.
        assert!(state.edit_style("kite", StyleEdit::Beam(Some(Cancelable::Cancel))));
        let acts = click_probe(&ctx, &state, &mut cfg, "ed-beam-temp", 0, 0.0);
        assert!(edits_of(&acts).is_empty(), "{acts:?}");
    }

    #[test]
    fn beam_off_is_one_line_and_ticking_seeds_white() {
        let (_d, state) = loaded();
        let (ctx, _) = headless();
        let mut cfg = expanded("E");
        for tag in ["ed-beam-cancel", "ed-beam-colour", "ed-beam-temp"] {
            assert!(render_probes(&state, &mut cfg, tag).is_empty(), "{tag}");
        }
        let acts = click_probe(&ctx, &state, &mut cfg, "ed-beam-check", 0, 0.0);
        assert_eq!(
            edits_of(&acts),
            [StyleEdit::Beam(Some(Cancelable::Set(Beam {
                color: FilterColor::White,
                temp: false
            })))]
        );
    }

    fn alert(id: u8, vol: u16, positional: bool) -> Sound {
        Sound {
            id: Some(id),
            custom: None,
            vol,
            positional,
        }
    }

    fn file(name: &str, vol: u16) -> Sound {
        Sound {
            id: None,
            custom: Some(name.to_string()),
            vol,
            positional: false,
        }
    }

    #[test]
    fn sound_menu_lists_alerts_then_the_folder_and_flags_a_missing_file() {
        let files = vec!["a.mp3".to_string(), "uwu.mp3".to_string()];
        let (items, sel) = sound_menu(&files, true, &alert(6, 200, false));
        assert_eq!(items.len(), 16 + 1 + 2);
        assert_eq!(items[0], SoundItem::Alert(1));
        assert_eq!(items[15], SoundItem::Alert(16));
        assert_eq!(items[16], SoundItem::Header);
        assert_eq!(items[18], SoundItem::File("uwu.mp3".to_string()));
        assert_eq!(sel, 5);
        let (_, sel) = sound_menu(&files, true, &file("UWU.mp3", 300));
        assert_eq!(sel, 18, "matched case-insensitively");
        let (items, sel) = sound_menu(&files, true, &file("gone.wav", 300));
        assert_eq!(items.len(), 20);
        assert_eq!(items[sel], SoundItem::Missing("gone.wav".to_string()));
        assert_eq!(FOLDER_HEADER, "Filter folder");
    }

    #[test]
    fn sound_menu_never_flags_a_file_when_the_folder_was_not_listed() {
        let (items, sel) = sound_menu(&[], false, &file("uwu.mp3", 300));
        assert_eq!(items.len(), 16 + 1 + 1);
        assert_eq!(items[sel], SoundItem::Unlisted("uwu.mp3".to_string()));
        assert!(!items.iter().any(|i| matches!(i, SoundItem::Missing(_))));
        assert_eq!(
            sound_for_item(&items[sel], &file("uwu.mp3", 300)),
            Some(file("uwu.mp3", 300))
        );
    }

    #[test]
    fn sound_picks_keep_the_volume_and_files_drop_positional() {
        let base = alert(2, 200, true);
        assert_eq!(
            sound_for_item(&SoundItem::Alert(3), &base),
            Some(alert(3, 200, true))
        );
        assert_eq!(
            sound_for_item(&SoundItem::File("b.wav".to_string()), &base),
            Some(file("b.wav", 200))
        );
        assert_eq!(
            sound_for_item(&SoundItem::Alert(4), &file("b.wav", 90)),
            Some(alert(4, 90, false))
        );
        assert_eq!(sound_for_item(&SoundItem::Header, &base), None);
    }

    #[test]
    fn custom_sound_file_names_are_checked() {
        assert!(safe_file_name("uwu.mp3"));
        assert!(!safe_file_name("a\"b.mp3"));
        assert!(!safe_file_name("a\nb.mp3"));
    }

    #[test]
    fn sound_group_lines_follow_the_sound() {
        let (_d, state) = loaded();
        let (ctx, _) = headless();
        let mut cfg = expanded("E");
        for tag in [
            "ed-sound",
            "ed-sound-cancel",
            "ed-volume",
            "ed-sound-positional",
        ] {
            assert!(render_probes(&state, &mut cfg, tag).is_empty(), "{tag}");
        }
        let acts = click_probe(&ctx, &state, &mut cfg, "ed-sound-check", 0, 0.0);
        assert_eq!(
            edits_of(&acts),
            [StyleEdit::Sound(Some(Cancelable::Set(alert(
                1, 100, false
            ))))]
        );
        let mut cfg = expanded("kite");
        for tag in [
            "ed-sound",
            "ed-sound-cancel",
            "ed-volume",
            "ed-sound-positional",
        ] {
            assert_eq!(render_probes(&state, &mut cfg, tag).len(), 1, "{tag}");
        }
        assert!(render_probes(&state, &mut cfg, "ed-sound-file").is_empty());
        assert!(render_probes(&state, &mut cfg, "ed-sound-missing").is_empty());
        let acts = click_probe(&ctx, &state, &mut cfg, "ed-sound-positional", 0, 0.0);
        assert_eq!(
            edits_of(&acts),
            [StyleEdit::Sound(Some(Cancelable::Set(alert(2, 200, true))))]
        );
        let acts = click_probe(&ctx, &state, &mut cfg, "ed-sound-cancel", 0, 0.0);
        assert_eq!(
            edits_of(&acts),
            [StyleEdit::Sound(Some(Cancelable::Cancel))]
        );
        // S plays a file. With no folder known nothing can be said about it: not flagged (WR-01).
        let mut cfg = expanded("S");
        assert!(render_probes(&state, &mut cfg, "ed-sound-missing").is_empty());
        let gone = tempfile::tempdir().unwrap();
        let unreadable = gone.path().join("nope");
        assert!(
            render_probes_with_folder(&state, &mut cfg, "ed-sound-missing", Some(&unreadable))
                .is_empty(),
            "an unreadable folder flags nothing"
        );
        // A listed (here empty) folder without the file: flagged, no Positional.
        let empty = tempfile::tempdir().unwrap();
        assert_eq!(
            render_probes_with_folder(&state, &mut cfg, "ed-sound-missing", Some(empty.path()))
                .len(),
            1
        );
        let acts = click_probe(&ctx, &state, &mut cfg, "ed-sound-positional", 0, 0.0);
        assert!(edits_of(&acts).is_empty(), "{acts:?}");
    }

    #[test]
    fn sound_cancel_greys_volume_and_positional() {
        let (_d, mut state) = loaded();
        let (ctx, _) = headless();
        let mut cfg = expanded("kite");
        assert!(state.edit_style("kite", StyleEdit::Sound(Some(Cancelable::Cancel))));
        let acts = click_probe(&ctx, &state, &mut cfg, "ed-sound-positional", 0, 0.0);
        assert!(edits_of(&acts).is_empty(), "{acts:?}");
        let acts = click_probe(&ctx, &state, &mut cfg, "ed-volume", 0, 40.0);
        assert!(edits_of(&acts).is_empty(), "{acts:?}");
    }

    #[test]
    fn drop_sound_on_is_a_silent_normal_switch() {
        let (_d, state) = loaded();
        let (ctx, _) = headless();
        let mut cfg = expanded("S");
        assert!(render_probes(&state, &mut cfg, "ed-drop").is_empty());
        let mut cfg = expanded("hidden");
        let r = render_probes(&state, &mut cfg, "ed-drop")[0];
        let acts = click_styles(
            &ctx,
            &state,
            &mut cfg,
            r.center() + vec2(segment_dx(r, 1, 2), 0.0),
        );
        assert_eq!(
            edits_of(&acts),
            [StyleEdit::DropSound(Some(
                crate::models::config::DropSound::Normal
            ))]
        );
        let check = render_probes(&state, &mut cfg, "ed-drop-check")[0];
        assert_eq!(check.center().y, r.center().y, "one line");
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

    #[test]
    fn styles_header_captions_fit_their_columns() {
        let ctx = egui::Context::default();
        crate::skin::fonts::install(&ctx);
        let cols = table_columns(460.0);
        let out = ctx.run_ui(egui::RawInput::default(), |ui| {
            for (text, col) in [
                ("Visibility", cols.vis),
                ("Name", cols.name),
                ("Icon", cols.icon),
                ("Preview", cols.preview),
                ("Order", cols.order),
            ] {
                let galley =
                    ui.painter()
                        .layout_no_wrap(text.to_owned(), regular(14.0), pal::MUTED);
                let w = galley.size().x;
                assert!(
                    w <= col.w,
                    "{text} is {w}pt wide, its column is {}pt",
                    col.w
                );
            }
        });
        out.drop_without_applying_deltas();
    }
}
