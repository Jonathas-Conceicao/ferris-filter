//! The style editor's lines, shared by the CONFIG style editor and a Custom rule's STYLE section.
//!
//! An adapter, not a model: the caller turns what it edits into a plain [`StyleView`] (CONFIG from
//! a `styles.json` [`Style`], RULES from the rule's action lines), [`show`] draws the lines in a
//! fixed order (Visibility, Label preview + Text / Border / Background, Font size, then the
//! Minimap, Beam, Alert sound and Drop sound groups) and returns what the user changed as
//! [`StyleIntent`]s, and the caller maps each intent onto its own edit (a `StyleEdit`, or the
//! minimal line edits of a rule). Nothing here mutates anything; an idle frame returns no intent.
//!
//! A group is drawn on one tinted band (one zebra index for all its lines). Its first line is the
//! label and an on/off check: off folds the group to that line and writes nothing; on offers,
//! first, the `/` tile for the filter's cancel value (`MinimapIcon -1`, `PlayEffect None`,
//! `PlayAlertSound None`; picking it greys the dependent pickers), then the values. The alert
//! sound dropdown lists Alert 1-16 and the sound files of the loaded filter's folder
//! ([`super::sound_files`]); there is no free-text file field.

use egui::{RichText, vec2};

use crate::models::config::{
    Beam, Cancelable, DropSound, FilterColor, Minimap, MinimapShape, Sound, Style, StyleAction,
    StyleColour,
};
use crate::skin::editor as ed;
use crate::skin::fonts::regular;
use crate::skin::minimap;
use crate::skin::palette as pal;

use super::config_state::ColourSlot;
use super::widgets;

/// The Visibility line. `Minimal` (a foreign rule header) lights neither segment.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Vis {
    Show,
    Hide,
    Minimal,
}

/// A colour slot's value: unmultiplied RGBA and whether the line was written with 4 tokens.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rgba {
    pub rgba: [u8; 4],
    pub had_alpha: bool,
}

/// What the lines show. Plain data, no egui.
#[derive(Debug, Clone, PartialEq)]
pub struct StyleView {
    pub visibility: Vis,
    pub text: Option<Rgba>,
    pub border: Option<Rgba>,
    pub background: Option<Rgba>,
    pub font: Option<i64>,
    /// `None` = off (no line), `Cancel` = `MinimapIcon -1`.
    pub minimap: Option<Cancelable<Minimap>>,
    /// Under `/`: the icon a loaded `MinimapIcon -1 <colour> <shape>` line names, which the
    /// pickers show and leaving `/` brings back (after one remembered this session).
    pub minimap_hint: Option<Minimap>,
    /// `None` = off (no line), `Cancel` = `PlayEffect None`.
    pub beam: Option<Cancelable<Beam>>,
    /// `None` = off (no line), `Cancel` = `PlayAlertSound None`. A set sound's `vol` is the shown
    /// volume: the written one, or the game default when none is written.
    pub sound: Option<Cancelable<Sound>>,
    /// Whether the volume is written (a rule line may omit it).
    pub sound_volume_written: bool,
    /// `None` = off (no line), else `DisableDropSound` / `EnableDropSound`.
    pub drop_sound: Option<DropSound>,
}

impl StyleView {
    /// A `styles.json` style: colours carry alpha, every volume written.
    pub fn from_style(style: &Style) -> StyleView {
        let rgba = |c: StyleColour| Rgba {
            rgba: c.rgba(),
            had_alpha: true,
        };
        StyleView {
            visibility: match style.action {
                StyleAction::Show => Vis::Show,
                StyleAction::Hide => Vis::Hide,
            },
            text: style.text.map(rgba),
            border: style.border.map(rgba),
            background: style.background.map(rgba),
            font: style.font.map(i64::from),
            minimap: style.minimap.clone(),
            minimap_hint: None,
            beam: style.beam.clone(),
            sound: style.sound.clone(),
            sound_volume_written: true,
            drop_sound: style.drop_sound(),
        }
    }

    fn colour(&self, slot: ColourSlot) -> Option<Rgba> {
        match slot {
            ColourSlot::Text => self.text,
            ColourSlot::Border => self.border,
            ColourSlot::Background => self.background,
        }
    }
}

/// One change the user made on a line.
#[derive(Debug, Clone, PartialEq)]
pub enum StyleIntent {
    Visibility(Vis),
    Colour {
        slot: ColourSlot,
        /// `None` clears the slot.
        value: Option<[u8; 4]>,
        /// The line's form (4 tokens) when the picker opened (CR-02).
        opened_had_alpha: bool,
    },
    Font(Option<i64>),
    Minimap(Option<Cancelable<Minimap>>),
    Beam(Option<Cancelable<Beam>>),
    Sound(Option<Cancelable<Sound>>),
    DropSound(Option<DropSound>),
}

pub(crate) const FONT_TIP: &str = "Font size override; off uses the game default";
/// Hover text of the Font size check and its dimmed slider while the override is off.
pub(crate) const FONT_OFF_TIP: &str = "Off: the game's default size. Tick to set a size.";
/// Hover text of the Font size check while the override is on.
pub(crate) const FONT_ON_TIP: &str = "On: this size. Untick for the game's default size.";
/// The value text of the dimmed Font size slider while the override is off.
pub(crate) const FONT_OFF_VALUE: &str = "Default";
/// Hover texts of the group checks (on, off).
pub(crate) const MINIMAP_ON_TIP: &str = "On: this icon. Untick to write no minimap line.";
pub(crate) const MINIMAP_OFF_TIP: &str = "Off: no minimap line. Tick to set an icon.";
pub(crate) const BEAM_ON_TIP: &str = "On: this beam. Untick to write no beam line.";
pub(crate) const BEAM_OFF_TIP: &str = "Off: no beam line. Tick to set a beam.";
pub(crate) const SOUND_ON_TIP: &str = "On: this alert sound. Untick to write no alert sound line.";
pub(crate) const SOUND_OFF_TIP: &str = "Off: no alert sound line. Tick to set a sound.";
pub(crate) const DROP_ON_TIP: &str =
    "On: Silent or Normal drop sound. Untick to write no drop sound line.";
pub(crate) const DROP_OFF_TIP: &str =
    "Off: no drop sound line (the game default). Tick to set Silent or Normal.";
/// Hover texts of the `/` tiles (the filter's cancel values).
pub(crate) const MINIMAP_CANCEL_TIP: &str = "None: hide the icon (MinimapIcon -1)";
pub(crate) const BEAM_CANCEL_TIP: &str = "None: no beam (PlayEffect None)";
pub(crate) const SOUND_CANCEL_TIP: &str = "None: no alert sound (PlayAlertSound None)";
pub(crate) const POSITIONAL_TIP: &str =
    "Play the alert from the item's position (PlayAlertSoundPositional)";
pub(crate) const POSITIONAL_FILE_TIP: &str = "Sound files have no positional form";
/// The Minimap size segments, in `MinimapIcon` size order (0 = Large).
pub(crate) const MINIMAP_SIZES: [&str; 3] = ["Large", "Medium", "Small"];
/// The header of the sound files section of the Alert sound dropdown.
pub(crate) const FOLDER_HEADER: &str = "Filter folder";
/// The font size a tick brings back when none is remembered.
const FONT_DEFAULT: i64 = 32;

// Layout probe for tests: records where a named element was drawn. A no-op outside tests.
#[cfg(test)]
thread_local! {
    static PROBE: std::cell::RefCell<Vec<(&'static str, egui::Rect)>> =
        const { std::cell::RefCell::new(Vec::new()) };
}

#[cfg(test)]
pub(crate) fn probe(tag: &'static str, rect: egui::Rect) {
    PROBE.with(|p| p.borrow_mut().push((tag, rect)));
}

#[cfg(not(test))]
pub(crate) fn probe(_tag: &'static str, _rect: egui::Rect) {}

/// Every probe recorded on this thread since the last drain.
#[cfg(test)]
pub(crate) fn drain_probes() -> Vec<(&'static str, egui::Rect)> {
    PROBE.with(|p| std::mem::take(&mut *p.borrow_mut()))
}

/// One editor line: zebra-striped, label in the 128pt column, control after it. Each line is its
/// own id scope so widgets of different lines never share an auto id.
fn ff_field_row_line(
    ui: &mut egui::Ui,
    n: &mut usize,
    label: &str,
    tip: Option<&str>,
    add: impl FnOnce(&mut egui::Ui),
) {
    let i = *n;
    *n += 1;
    ui.push_id(("ed-line", i), |ui| {
        crate::skin::widgets::ff_field_row(ui, i, label, tip, add);
    });
}

/// The value a field had before it was switched to None, kept under the caller's `memo` id (the
/// caller decides how long it lives) so switching back restores it.
fn remember<T: Clone + Send + Sync + 'static>(
    ui: &egui::Ui,
    memo: egui::Id,
    field: &'static str,
    value: &T,
) {
    ui.data_mut(|d| d.insert_temp(memo.with(field), value.clone()));
}

fn recall<T: Clone + Send + Sync + 'static>(
    ui: &egui::Ui,
    memo: egui::Id,
    field: &'static str,
) -> Option<T> {
    ui.data(|d| d.get_temp(memo.with(field)))
}

/// How the view's item-label preview looks. `with_font` carries its font size; the CONFIG table
/// previews leave it off so every row is the same height.
pub fn label_look(view: &StyleView, with_font: bool) -> ed::LabelLook {
    let colour = |c: Rgba| {
        let [r, g, b, a] = c.rgba;
        egui::Color32::from_rgba_unmultiplied(r, g, b, a)
    };
    ed::LabelLook {
        text: view.text.map(colour),
        border: view.border.map(colour),
        background: view.background.map(colour),
        font_size: if with_font {
            view.font.map(|f| f as f32)
        } else {
            None
        },
        hidden: view.visibility == Vis::Hide,
    }
}

/// Draws the style lines for `view` and returns what the user changed. `memo` keys the
/// remembered None-values and the custom-file draft; `n` is the running zebra index.
pub fn show(
    ui: &mut egui::Ui,
    view: &StyleView,
    memo: egui::Id,
    n: &mut usize,
) -> Vec<StyleIntent> {
    let skin = crate::skin::of(ui.ctx());
    let mut out = Vec::new();
    ui.spacing_mut().item_spacing = vec2(0.0, 0.0);

    ff_field_row_line(ui, n, "Visibility", None, |ui| {
        let mut sel = match view.visibility {
            Vis::Show => 0,
            Vis::Hide => 1,
            Vis::Minimal => 2,
        };
        if view.visibility == Vis::Minimal {
            ui.spacing_mut().item_spacing.x = 8.0;
            ui.label(
                RichText::new("Minimal")
                    .font(regular(15.0))
                    .color(pal::MUTED),
            );
        }
        let r = ed::ff_segmented(ui, &skin, &mut sel, &["Show", "Hide"], 128.0);
        probe("ed-visibility", r.rect);
        if r.changed() {
            let next = if sel == 0 { Vis::Show } else { Vis::Hide };
            if next != view.visibility {
                out.push(StyleIntent::Visibility(next));
            }
        }
    });

    ff_field_row_line(ui, n, "Label", None, |ui| {
        let (rect, _) =
            ui.allocate_exact_size(vec2(ui.available_width(), 44.0), egui::Sense::hover());
        ed::ff_item_label(ui.painter(), rect, "Item Name", label_look(view, true));
    });

    ff_field_row_line(ui, n, "", None, |ui| {
        ui.spacing_mut().item_spacing.x = 8.0;
        let width = ((ui.available_width() - 16.0) / 3.0)
            .floor()
            .clamp(60.0, 110.0);
        for (tag, label, slot, seed) in [
            ("text", "Text", ColourSlot::Text, [248, 248, 242]),
            ("border", "Border", ColourSlot::Border, [248, 248, 242]),
            (
                "background",
                "Background",
                ColourSlot::Background,
                [40, 42, 54],
            ),
        ] {
            let current = view.colour(slot);
            let intent = {
                let cur = current.map(|c| c.rgba);
                let had = current.is_some_and(|c| c.had_alpha);
                let seed = [seed[0], seed[1], seed[2], 255];
                match widgets::colour_slot_rgba(ui, tag, label, cur, had, seed, width) {
                    widgets::SlotRgba::Unchanged => continue,
                    widgets::SlotRgba::Set(c, opened) => StyleIntent::Colour {
                        slot,
                        value: Some(c),
                        opened_had_alpha: opened,
                    },
                    widgets::SlotRgba::Cleared => StyleIntent::Colour {
                        slot,
                        value: None,
                        opened_had_alpha: had,
                    },
                }
            };
            out.push(intent);
        }
    });

    font_line(ui, &skin, view, memo, n, &mut out);
    minimap_group(ui, &skin, view, memo, n, &mut out);
    beam_group(ui, &skin, view, memo, n, &mut out);
    sound_group(ui, &skin, view, memo, n, &mut out);
    drop_group(ui, &skin, view, memo, n, &mut out);
    out
}

fn font_line(
    ui: &mut egui::Ui,
    skin: &crate::skin::Skin,
    view: &StyleView,
    memo: egui::Id,
    n: &mut usize,
    out: &mut Vec<StyleIntent>,
) {
    ff_field_row_line(ui, n, "Font size", Some(FONT_TIP), |ui| {
        ui.spacing_mut().item_spacing.x = 8.0;
        let mut on = view.font.is_some();
        let check = crate::skin::widgets::ff_check(ui, skin, &mut on, "Font size");
        probe("ed-font-check", check.rect);
        let check = check.on_hover_text(if view.font.is_some() {
            FONT_ON_TIP
        } else {
            FONT_OFF_TIP
        });
        if check.changed() {
            // WR-03: turning it back on restores the size it had before it was turned off.
            let next = if on {
                Some(recall::<i64>(ui, memo, "font").unwrap_or(FONT_DEFAULT))
            } else {
                if let Some(f) = view.font {
                    remember(ui, memo, "font", &f);
                }
                None
            };
            out.push(StyleIntent::Font(next));
        }
        if let Some(font) = view.font {
            let mut v = font as f64;
            // The slider takes the rest of the row after the check.
            let rest = (ui.max_rect().right() - ui.next_widget_position().x).max(120.0);
            let r = ui
                .allocate_ui(vec2(rest, 28.0), |ui| {
                    crate::skin::widgets::ff_slider(
                        ui,
                        skin,
                        &mut v,
                        1.0..=45.0,
                        1.0,
                        "Font size",
                        |v| format!("{v:.0}"),
                    )
                })
                .inner;
            probe("ed-font-slider", r.rect);
            if r.changed() {
                out.push(StyleIntent::Font(Some(v.round() as i64)));
            }
            if let Some(msg) = super::body::font_hint(font) {
                r.on_hover_text(msg);
            }
        } else {
            // Off keeps the line's shape: the slider stays, dimmed and inert, reading "Default"
            // at the size a tick would bring back, so the line never shows a lone circle.
            let mut v = recall::<i64>(ui, memo, "font").unwrap_or(FONT_DEFAULT) as f64;
            let rest = (ui.max_rect().right() - ui.next_widget_position().x).max(120.0);
            let r = ui
                .allocate_ui(vec2(rest, 28.0), |ui| {
                    ui.add_enabled_ui(false, |ui| {
                        crate::skin::widgets::ff_slider(
                            ui,
                            skin,
                            &mut v,
                            1.0..=45.0,
                            1.0,
                            "Font size (game default)",
                            |_| FONT_OFF_VALUE.to_string(),
                        )
                    })
                    .inner
                })
                .inner;
            probe("ed-font-slider-off", r.rect);
            r.on_disabled_hover_text(FONT_OFF_TIP);
        }
    });
}

/// A row of `count` equal tiles of at most 24pt that fits `reserved` less than the free width.
fn tile_size(ui: &egui::Ui, count: f32, reserved: f32) -> f32 {
    ((ui.available_width() - 4.0 * (count - 1.0) - reserved - 2.0) / count)
        .floor()
        .clamp(14.0, 24.0)
}

/// One line of a group: every line of a group shares the group's zebra index `g`, so the group
/// reads as one tinted band; each line is still its own id scope.
fn group_line(
    ui: &mut egui::Ui,
    g: usize,
    tag: &'static str,
    k: usize,
    label: &str,
    add: impl FnOnce(&mut egui::Ui),
) {
    ui.push_id(("ed-group", tag, k), |ui| {
        crate::skin::widgets::ff_field_row(ui, g, label, None, add);
    });
}

/// A group's first line: its label and the on/off round check (`probe` tag), then `extra` on the
/// same line. Returns the check's new state when it was clicked.
#[allow(clippy::too_many_arguments)]
fn group_header(
    ui: &mut egui::Ui,
    skin: &crate::skin::Skin,
    g: usize,
    tag: &'static str,
    label: &str,
    on: bool,
    tips: (&str, &str),
    extra: impl FnOnce(&mut egui::Ui),
) -> Option<bool> {
    let mut toggled = None;
    group_line(ui, g, tag, 0, label, |ui| {
        ui.spacing_mut().item_spacing.x = 8.0;
        let mut now = on;
        let check = crate::skin::widgets::ff_check(ui, skin, &mut now, label);
        probe(tag, check.rect);
        let check = check.on_hover_text(if on { tips.0 } else { tips.1 });
        if check.changed() {
            toggled = Some(now);
        }
        extra(ui);
    });
    toggled
}

/// The `/` tile: the filter's cancel value (`probe` tag).
fn cancel_tile(
    ui: &mut egui::Ui,
    tag: &'static str,
    selected: bool,
    width: f32,
    tip: &str,
) -> egui::Response {
    let r = ed::ff_swatch(ui, None, selected, vec2(width, 24.0), "None").on_hover_text(tip);
    probe(tag, r.rect);
    r
}

/// The minimap icon a tick brings back when none is remembered.
fn default_minimap() -> Minimap {
    Minimap {
        size: 0,
        color: FilterColor::White,
        shape: MinimapShape::Circle,
    }
}

fn default_beam() -> Beam {
    Beam {
        color: FilterColor::White,
        temp: false,
    }
}

fn default_sound() -> Sound {
    Sound {
        id: Some(1),
        custom: None,
        vol: 100,
        positional: false,
    }
}

/// The on/off check of a cancelable group: unticking remembers the value (and its last set value)
/// and writes nothing; ticking brings the remembered value back, else `default`.
fn toggle_cancelable<T: Clone + Send + Sync + 'static>(
    ui: &egui::Ui,
    memo: egui::Id,
    keys: (&'static str, &'static str),
    current: Option<&Cancelable<T>>,
    now_on: bool,
    default: T,
) -> Option<Cancelable<T>> {
    if now_on {
        Some(recall::<Cancelable<T>>(ui, memo, keys.0).unwrap_or(Cancelable::Set(default)))
    } else {
        if let Some(c) = current {
            remember(ui, memo, keys.0, c);
            if let Some(v) = c.value() {
                remember(ui, memo, keys.1, v);
            }
        }
        None
    }
}

fn minimap_group(
    ui: &mut egui::Ui,
    skin: &crate::skin::Skin,
    view: &StyleView,
    memo: egui::Id,
    n: &mut usize,
    out: &mut Vec<StyleIntent>,
) {
    let g = *n;
    *n += 1;
    let on = view.minimap.is_some();
    if let Some(now) = group_header(
        ui,
        skin,
        g,
        "ed-mm-check",
        "Minimap",
        on,
        (MINIMAP_ON_TIP, MINIMAP_OFF_TIP),
        |_| {},
    ) {
        let next = toggle_cancelable(
            ui,
            memo,
            ("minimap", "minimap-set"),
            view.minimap.as_ref(),
            now,
            default_minimap(),
        );
        // Unticking `MinimapIcon -1 <colour> <shape>` keeps that icon for leaving `/` later: the
        // tick brings back a bare `-1`, which names none (review WR-03).
        if !now && let Some(hint) = &view.minimap_hint {
            remember(ui, memo, "minimap-set", hint);
        }
        out.push(StyleIntent::Minimap(next));
    }
    let Some(current) = &view.minimap else {
        return;
    };
    let cancel = current.is_cancel();
    // What the pickers show: the value, or under `/` the one a pick brings back.
    let shown = current
        .value()
        .cloned()
        .or_else(|| view.minimap_hint.clone())
        .or_else(|| recall(ui, memo, "minimap-set"))
        .unwrap_or_else(default_minimap);
    let colour = minimap::index_of(&minimap::COLOURS, shown.color.as_str()).unwrap_or(0);
    let shape = minimap::index_of(&minimap::SHAPES, shown.shape.as_str()).unwrap_or(0);
    let mut tile = 24.0;
    group_line(ui, g, "minimap", 1, "", |ui| {
        ui.spacing_mut().item_spacing.x = 4.0;
        // 13 slots: the shapes line below indents one tile and shares this size.
        tile = tile_size(ui, 13.0, 0.0);
        if cancel_tile(ui, "ed-mm-cancel", cancel, tile, MINIMAP_CANCEL_TIP).clicked() && !cancel {
            remember(ui, memo, "minimap-set", &shown);
            out.push(StyleIntent::Minimap(Some(Cancelable::Cancel)));
        }
        for (i, label) in minimap::COLOURS.iter().enumerate() {
            let lit = !cancel && i == colour;
            let r = ed::ff_swatch(ui, Some(minimap::SWATCHES[i]), lit, vec2(tile, 24.0), label)
                .on_hover_text(*label);
            probe("ed-mm-colour", r.rect);
            if r.clicked()
                && !lit
                && let Some(color) = FilterColor::from_name(label)
            {
                let mut next = shown.clone();
                next.color = color;
                out.push(StyleIntent::Minimap(Some(Cancelable::Set(next))));
            }
        }
    });
    let indent = tile + 4.0;
    group_line(ui, g, "minimap", 2, "", |ui| {
        ui.spacing_mut().item_spacing.x = 4.0;
        ui.add_space(indent);
        {
            if cancel {
                ui.disable();
            }
            for (i, label) in minimap::SHAPES.iter().enumerate() {
                let lit = !cancel && i == shape;
                let r = ed::ff_icon_button(ui, skin, i, colour, lit, tile);
                probe("ed-mm-shape", r.rect);
                if r.clicked()
                    && !lit
                    && let Some(s) = MinimapShape::from_name(label)
                {
                    let mut next = shown.clone();
                    next.shape = s;
                    out.push(StyleIntent::Minimap(Some(Cancelable::Set(next))));
                }
            }
        }
    });
    group_line(ui, g, "minimap", 3, "", |ui| {
        ui.add_space(indent);
        let width = 288.0f32.min(ui.available_width());
        {
            if cancel {
                ui.disable();
            }
            let mut sel = if cancel {
                usize::MAX
            } else {
                usize::from(shown.size)
            };
            let r = ed::ff_segmented(ui, skin, &mut sel, &MINIMAP_SIZES, width);
            probe("ed-mm-size", r.rect);
            if r.changed()
                && let Ok(size) = u8::try_from(sel)
            {
                let mut next = shown.clone();
                next.size = size;
                out.push(StyleIntent::Minimap(Some(Cancelable::Set(next))));
            }
        }
    });
}

fn beam_group(
    ui: &mut egui::Ui,
    skin: &crate::skin::Skin,
    view: &StyleView,
    memo: egui::Id,
    n: &mut usize,
    out: &mut Vec<StyleIntent>,
) {
    let g = *n;
    *n += 1;
    if let Some(now) = group_header(
        ui,
        skin,
        g,
        "ed-beam-check",
        "Beam",
        view.beam.is_some(),
        (BEAM_ON_TIP, BEAM_OFF_TIP),
        |_| {},
    ) {
        let next = toggle_cancelable(
            ui,
            memo,
            ("beam", "beam-set"),
            view.beam.as_ref(),
            now,
            default_beam(),
        );
        out.push(StyleIntent::Beam(next));
    }
    let Some(current) = &view.beam else {
        return;
    };
    let cancel = current.is_cancel();
    let shown = current
        .value()
        .cloned()
        .or_else(|| recall(ui, memo, "beam-set"))
        .unwrap_or_else(default_beam);
    let colour = minimap::index_of(&minimap::COLOURS, shown.color.as_str());
    group_line(ui, g, "beam", 1, "", |ui| {
        ui.spacing_mut().item_spacing.x = 4.0;
        // Room for the Temp chip after the 12 tiles.
        let tile = tile_size(ui, 12.0, 62.0);
        if cancel_tile(ui, "ed-beam-cancel", cancel, tile, BEAM_CANCEL_TIP).clicked() && !cancel {
            remember(ui, memo, "beam-set", &shown);
            out.push(StyleIntent::Beam(Some(Cancelable::Cancel)));
        }
        for (i, label) in minimap::COLOURS.iter().enumerate() {
            let lit = !cancel && colour == Some(i);
            let r = ed::ff_swatch(ui, Some(minimap::SWATCHES[i]), lit, vec2(tile, 24.0), label)
                .on_hover_text(*label);
            probe("ed-beam-colour", r.rect);
            if r.clicked()
                && !lit
                && let Some(color) = FilterColor::from_name(label)
            {
                let mut next = shown.clone();
                next.color = color;
                out.push(StyleIntent::Beam(Some(Cancelable::Set(next))));
            }
        }
        let mut temp = !cancel && shown.temp;
        ui.add_enabled_ui(!cancel, |ui| {
            let r = ed::ff_chip(ui, "Temp", &mut temp);
            probe("ed-beam-temp", r.rect);
            if r.changed() {
                let mut next = shown.clone();
                next.temp = temp;
                out.push(StyleIntent::Beam(Some(Cancelable::Set(next))));
            }
        });
    });
}

/// One row of the Alert sound dropdown.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum SoundItem {
    Alert(u8),
    /// The "Filter folder" section header.
    Header,
    /// A sound file found in the filter folder.
    File(String),
    /// The current file, not found in the filter folder.
    Missing(String),
    /// The current file when the filter folder could not be listed: nothing is known about it,
    /// so it is not flagged.
    Unlisted(String),
}

impl SoundItem {
    fn label(&self) -> String {
        match self {
            SoundItem::Alert(k) => format!("Alert {k}"),
            SoundItem::Header => FOLDER_HEADER.to_string(),
            SoundItem::File(f) | SoundItem::Missing(f) | SoundItem::Unlisted(f) => f.clone(),
        }
    }
}

/// The dropdown rows (Alert 1-16, the Filter folder header, the folder's `files`, and the
/// `current` file when it is not among them) and the row `current` is. `listed` is whether the
/// folder was read: a file absent from an unlisted folder is [`SoundItem::Unlisted`], not
/// [`SoundItem::Missing`].
pub(crate) fn sound_menu(
    files: &[String],
    listed: bool,
    current: &Sound,
) -> (Vec<SoundItem>, usize) {
    let mut items: Vec<SoundItem> = (1..=16).map(SoundItem::Alert).collect();
    items.push(SoundItem::Header);
    items.extend(files.iter().cloned().map(SoundItem::File));
    let selected = match &current.custom {
        Some(f) => match files.iter().position(|x| x.eq_ignore_ascii_case(f)) {
            Some(i) => 17 + i,
            None => {
                items.push(if listed {
                    SoundItem::Missing(f.clone())
                } else {
                    SoundItem::Unlisted(f.clone())
                });
                items.len() - 1
            }
        },
        None => usize::from(current.id.unwrap_or(1).clamp(1, 16)) - 1,
    };
    (items, selected)
}

/// The sound picking `item` writes, keeping the volume and positional toggle of `base` (a file
/// has no positional form). `None` for the header.
pub(crate) fn sound_for_item(item: &SoundItem, base: &Sound) -> Option<Sound> {
    match item {
        SoundItem::Alert(k) => Some(Sound {
            id: Some(*k),
            custom: None,
            vol: base.vol,
            positional: base.custom.is_none() && base.positional,
        }),
        SoundItem::File(f) | SoundItem::Missing(f) | SoundItem::Unlisted(f) => Some(Sound {
            id: None,
            custom: Some(f.clone()),
            vol: base.vol,
            positional: false,
        }),
        SoundItem::Header => None,
    }
}

/// A custom sound file name that is safe to write into `CustomAlertSound "<name>"`.
pub(crate) fn safe_file_name(name: &str) -> bool {
    !name.chars().any(|c| c == '"' || c.is_control())
}

/// An inline caption in the control column (Volume, Positional).
fn inline_label(ui: &mut egui::Ui, text: &str) {
    ui.label(
        RichText::new(text.to_uppercase())
            .font(crate::skin::fonts::small_caps(15.0))
            .color(pal::LABEL),
    );
}

fn sound_group(
    ui: &mut egui::Ui,
    skin: &crate::skin::Skin,
    view: &StyleView,
    memo: egui::Id,
    n: &mut usize,
    out: &mut Vec<StyleIntent>,
) {
    let g = *n;
    *n += 1;
    if let Some(now) = group_header(
        ui,
        skin,
        g,
        "ed-sound-check",
        "Alert sound",
        view.sound.is_some(),
        (SOUND_ON_TIP, SOUND_OFF_TIP),
        |_| {},
    ) {
        let next = toggle_cancelable(
            ui,
            memo,
            ("sound", "sound-set"),
            view.sound.as_ref(),
            now,
            default_sound(),
        );
        out.push(StyleIntent::Sound(next));
    }
    let Some(current) = &view.sound else {
        return;
    };
    let cancel = current.is_cancel();
    let shown = current
        .value()
        .cloned()
        .or_else(|| recall(ui, memo, "sound-set"))
        .unwrap_or_else(default_sound);
    let tile = 24.0;
    group_line(ui, g, "sound", 1, "", |ui| {
        ui.spacing_mut().item_spacing.x = 4.0;
        if cancel_tile(ui, "ed-sound-cancel", cancel, tile, SOUND_CANCEL_TIP).clicked() && !cancel {
            remember(ui, memo, "sound-set", &shown);
            out.push(StyleIntent::Sound(Some(Cancelable::Cancel)));
        }
        #[cfg(debug_assertions)]
        if crate::skin::widgets::take_dropdown_request(ui.ctx(), "sound") {
            super::sound_files::refresh(ui.ctx());
            egui::Popup::open_id(ui.ctx(), ui.make_persistent_id("sound"));
        }
        let folder = super::sound_files::get(ui.ctx());
        let (items, selected) = sound_menu(&folder.files, folder.listed, &shown);
        let labels: Vec<String> = items.iter().map(SoundItem::label).collect();
        let drop_items: Vec<crate::skin::widgets::DropItem> = items
            .iter()
            .zip(&labels)
            .map(|(item, label)| match item {
                SoundItem::Header => crate::skin::widgets::DropItem::Header(label),
                SoundItem::Missing(_) => crate::skin::widgets::DropItem::Warn(label),
                _ => crate::skin::widgets::DropItem::Option(label),
            })
            .collect();
        let pick = crate::skin::widgets::ff_dropdown_items(
            ui,
            skin,
            "sound",
            Some(selected),
            &drop_items,
            180.0,
            cancel.then_some(pal::MUTED),
        );
        probe("ed-sound", pick.response.rect);
        if pick.opened {
            super::sound_files::refresh(ui.ctx());
        }
        if let Some(i) = pick.picked
            && (cancel || i != selected)
            && let Some(next) = items.get(i).and_then(|item| sound_for_item(item, &shown))
        {
            out.push(StyleIntent::Sound(Some(Cancelable::Set(next))));
        }
        if !cancel && let Some(SoundItem::Missing(file)) = items.get(selected) {
            ui.add_space(4.0);
            let dir = folder
                .dir
                .as_ref()
                .map_or_else(|| "(unknown)".to_string(), |d| d.display().to_string());
            let r = crate::skin::icons::flag_icon(
                ui,
                crate::skin::icons::FlagIcon::GrammarWarning,
                pal::WARN,
                "sound file missing",
                &format!(
                    "{file} is not in the filter folder {dir}; the game plays no sound until it is there"
                ),
            );
            probe("ed-sound-missing", r.rect);
        }
        if !cancel && let Some(SoundItem::Unlisted(file)) = items.get(selected) {
            pick.response.clone().on_hover_text(format!(
                "The filter folder could not be listed, so whether {file} is there is unknown"
            ));
        }
    });
    let indent = tile + 4.0;
    group_line(ui, g, "sound", 2, "", |ui| {
        ui.spacing_mut().item_spacing.x = 8.0;
        ui.add_space(indent - 8.0);
        {
            if cancel {
                ui.disable();
            }
            inline_label(ui, "Volume");
            let mut v = f64::from(shown.vol);
            let rest = (ui.max_rect().right() - ui.next_widget_position().x).max(120.0);
            let r = ui
                .allocate_ui(vec2(rest, 28.0), |ui| {
                    crate::skin::widgets::ff_slider(
                        ui,
                        skin,
                        &mut v,
                        0.0..=300.0,
                        1.0,
                        "Volume",
                        |v| format!("{v:.0}"),
                    )
                })
                .inner;
            probe("ed-volume", r.rect);
            if r.changed()
                && let Ok(vol) = u16::try_from(v.round() as i64)
            {
                out.push(StyleIntent::Sound(Some(Cancelable::Set(Sound {
                    vol,
                    ..shown.clone()
                }))));
            }
        }
    });
    group_line(ui, g, "sound", 3, "", |ui| {
        ui.spacing_mut().item_spacing.x = 8.0;
        ui.add_space(indent - 8.0);
        let file = shown.custom.is_some();
        {
            if cancel || file {
                ui.disable();
            }
            inline_label(ui, "Positional");
            let mut on = !cancel && !file && shown.positional;
            let r = crate::skin::widgets::ff_check(ui, skin, &mut on, "Positional");
            probe("ed-sound-positional", r.rect);
            let r = if file {
                r.on_disabled_hover_text(POSITIONAL_FILE_TIP)
            } else {
                r.on_hover_text(POSITIONAL_TIP)
            };
            if r.changed() {
                out.push(StyleIntent::Sound(Some(Cancelable::Set(Sound {
                    positional: on,
                    ..shown.clone()
                }))));
            }
        }
    });
}

fn drop_group(
    ui: &mut egui::Ui,
    skin: &crate::skin::Skin,
    view: &StyleView,
    memo: egui::Id,
    n: &mut usize,
    out: &mut Vec<StyleIntent>,
) {
    let g = *n;
    *n += 1;
    let mut switched = None;
    let toggled = group_header(
        ui,
        skin,
        g,
        "ed-drop-check",
        "Drop sound",
        view.drop_sound.is_some(),
        (DROP_ON_TIP, DROP_OFF_TIP),
        |ui| {
            let Some(d) = view.drop_sound else {
                return;
            };
            let mut sel = match d {
                DropSound::Silent => 0,
                DropSound::Normal => 1,
            };
            let width = 192.0f32.min(ui.available_width());
            let r = ed::ff_segmented(ui, skin, &mut sel, &["Silent", "Normal"], width);
            probe("ed-drop", r.rect);
            if r.changed() {
                switched = Some(if sel == 0 {
                    DropSound::Silent
                } else {
                    DropSound::Normal
                });
            }
        },
    );
    if let Some(now) = toggled {
        let next = if now {
            Some(recall(ui, memo, "drop").unwrap_or(DropSound::Silent))
        } else {
            if let Some(d) = view.drop_sound {
                remember(ui, memo, "drop", &d);
            }
            None
        };
        out.push(StyleIntent::DropSound(next));
    } else if let Some(d) = switched {
        out.push(StyleIntent::DropSound(Some(d)));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::filter::BlockKeyword;
    use crate::overlay::rules::WorkingFilter;

    #[test]
    fn from_style_carries_the_style_alpha() {
        let mut s = Style::blank_show();
        s.text = Some(crate::models::config::StyleColour::new(10, 20, 30, 90));
        let v = StyleView::from_style(&s);
        assert_eq!(v.text.map(|c| c.rgba), Some([10, 20, 30, 90]));
    }

    fn every_group_style() -> Style {
        let mut s = Style::blank_show();
        s.text = Some(crate::models::config::StyleColour::opaque(1, 2, 3));
        s.font = Some(30);
        s.minimap = Some(Cancelable::Set(Minimap {
            size: 1,
            color: FilterColor::Pink,
            shape: MinimapShape::Star,
        }));
        s.beam = Some(Cancelable::Cancel);
        s.sound = Some(Cancelable::Set(Sound {
            id: Some(6),
            custom: None,
            vol: 200,
            positional: true,
        }));
        s.set_drop_sound(Some(DropSound::Normal));
        s
    }

    /// The probe tags `show` draws for `view`, in order, and the intents of an idle frame.
    fn drawn(view: &StyleView) -> (Vec<&'static str>, Vec<StyleIntent>) {
        let ctx = egui::Context::default();
        crate::skin::fonts::install(&ctx);
        let raw = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                vec2(460.0, 900.0),
            )),
            ..Default::default()
        };
        drain_probes();
        let mut intents = Vec::new();
        ctx.run_ui(raw, |ui| {
            let mut n = 0;
            intents = show(ui, view, egui::Id::new("t"), &mut n);
        })
        .drop_without_applying_deltas();
        let tags = drain_probes().into_iter().map(|(t, _)| t).collect();
        (tags, intents)
    }

    /// Resting on a minimap shape tile shows its name as a tooltip exactly once.
    #[test]
    fn a_minimap_shape_tile_shows_one_tooltip() {
        fn texts(shape: &egui::epaint::Shape, name: &str, n: &mut usize) {
            match shape {
                egui::epaint::Shape::Text(t) if t.galley.text() == name => *n += 1,
                egui::epaint::Shape::Vec(v) => v.iter().for_each(|s| texts(s, name, n)),
                _ => {}
            }
        }
        let view = StyleView::from_style(&every_group_style());
        let ctx = egui::Context::default();
        crate::skin::fonts::install(&ctx);
        ctx.global_style_mut(|s| s.interaction.tooltip_delay = 0.0);
        let screen = egui::Rect::from_min_size(egui::Pos2::ZERO, vec2(460.0, 900.0));
        let frame = |events: Vec<egui::Event>| {
            let raw = egui::RawInput {
                screen_rect: Some(screen),
                events,
                ..Default::default()
            };
            drain_probes();
            let out = ctx.run_ui(raw, |ui| {
                let mut n = 0;
                show(ui, &view, egui::Id::new("t"), &mut n);
            });
            let rect = drain_probes()
                .into_iter()
                .find(|(t, _)| *t == "ed-mm-shape")
                .map(|(_, r)| r);
            let mut count = 0;
            for c in &out.shapes {
                texts(&c.shape, minimap::SHAPES[0], &mut count);
            }
            out.drop_without_applying_deltas();
            (rect, count)
        };
        let (rect, _) = frame(vec![]);
        let pos = rect.expect("shape tile drawn").center();
        let mut count = 0;
        for i in 0..6 {
            let ev = if i == 0 {
                vec![egui::Event::PointerMoved(pos)]
            } else {
                vec![]
            };
            count = frame(ev).1;
        }
        assert_eq!(count, 1, "one tooltip with the shape name");
    }

    /// The CONFIG style editor and a Custom rule's STYLE section draw the same lines with the same
    /// options: the rule written from a style shows the same probe tags in the same order.
    #[test]
    fn config_and_rule_editors_draw_the_same_lines() {
        let style = every_group_style();
        let mut src = String::from("Show\n");
        for line in crate::emitter::style_action_lines(&style) {
            src.push_str(&format!("    {line}\n"));
        }
        let wf = WorkingFilter::from_text(&src);
        let entries = wf.body_view(wf.rows()[0].id);
        let (rule_view, _) = super::super::body::rule_style_view(&entries, BlockKeyword::Show);
        let (config_tags, config_idle) = drawn(&StyleView::from_style(&style));
        let (rule_tags, rule_idle) = drawn(&rule_view);
        assert!(config_idle.is_empty() && rule_idle.is_empty());
        assert_eq!(config_tags, rule_tags);
        for tag in [
            "ed-mm-check",
            "ed-mm-cancel",
            "ed-mm-size",
            "ed-beam-check",
            "ed-beam-cancel",
            "ed-sound-check",
            "ed-sound-cancel",
            "ed-sound",
            "ed-volume",
            "ed-sound-positional",
            "ed-drop-check",
            "ed-drop",
        ] {
            assert!(config_tags.contains(&tag), "{tag}: {config_tags:?}");
        }
        assert!(!config_tags.contains(&"ed-sound-file"));
    }

    /// Off folds a group to its one check line.
    #[test]
    fn off_groups_draw_only_their_check() {
        let (tags, idle) = drawn(&StyleView::from_style(&Style::blank_show()));
        assert!(idle.is_empty());
        for tag in [
            "ed-mm-check",
            "ed-beam-check",
            "ed-sound-check",
            "ed-drop-check",
        ] {
            assert_eq!(tags.iter().filter(|t| **t == tag).count(), 1, "{tag}");
        }
        for tag in [
            "ed-mm-cancel",
            "ed-mm-colour",
            "ed-beam-cancel",
            "ed-sound",
            "ed-volume",
            "ed-drop",
        ] {
            assert!(!tags.contains(&tag), "{tag}");
        }
    }
}
