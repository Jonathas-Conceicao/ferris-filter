//! The style editor's lines, shared by the CONFIG style editor and a Custom rule's STYLE section.
//!
//! An adapter, not a model: the caller turns what it edits into a plain [`StyleView`] (CONFIG from
//! a `styles.json` [`Style`], RULES from the rule's action lines), [`show`] draws the lines in a
//! fixed order (Visibility, Label preview + Text / Border / Background, Font size, Minimap, Beam,
//! Sound + Volume, Drop sound) and returns what the user changed as [`StyleIntent`]s, and the
//! caller maps each intent onto its own edit (a `StyleEdit`, or the minimal line edits of a rule).
//! Nothing here mutates anything; an idle frame returns no intent.

use egui::{RichText, vec2};

use crate::models::config::{Beam, FilterColor, Minimap, MinimapShape, Sound, Style, StyleAction};
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
    pub minimap: Option<Minimap>,
    pub beam: Option<Beam>,
    /// `vol` is the shown volume: the written one, or the game default when none is written.
    pub sound: Option<Sound>,
    /// Whether the volume is written (a rule line may omit it).
    pub sound_volume_written: bool,
    pub disable_drop_sound: bool,
    /// Colour cards with an Alpha slider (rules) or opaque ones (styles store RGB).
    pub alpha: bool,
}

impl StyleView {
    /// A `styles.json` style: opaque colours, every volume written.
    pub fn from_style(style: &Style) -> StyleView {
        let rgba = |c: [u8; 3]| Rgba {
            rgba: [c[0], c[1], c[2], 255],
            had_alpha: false,
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
            beam: style.beam.clone(),
            sound: style.sound.clone(),
            sound_volume_written: true,
            disable_drop_sound: style.disable_drop_sound,
            alpha: false,
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
    Minimap(Option<Minimap>),
    Beam(Option<Beam>),
    Sound(Option<Sound>),
    DisableDropSound(bool),
}

pub(crate) const FONT_TIP: &str = "Font size override; off uses the game default";
/// Hover text of the Font size check and its dimmed slider while the override is off.
pub(crate) const FONT_OFF_TIP: &str = "Off: the game's default size. Tick to set a size.";
/// Hover text of the Font size check while the override is on.
pub(crate) const FONT_ON_TIP: &str = "On: this size. Untick for the game's default size.";
/// The value text of the dimmed Font size slider while the override is off.
pub(crate) const FONT_OFF_VALUE: &str = "Default";
pub(crate) const DROP_TIP: &str = "Silence the default drop sound";
pub(crate) const FILE_TIP: &str = "The file must be in the game folder.";
pub(crate) const BAD_FILE_TIP: &str = "File names cannot contain quotes or control characters";
const INVALID_RED: egui::Color32 = egui::Color32::from_rgb(0xFF, 0x55, 0x55);
pub(crate) const SOUND_OPTIONS: [&str; 18] = [
    "None",
    "Alert 1",
    "Alert 2",
    "Alert 3",
    "Alert 4",
    "Alert 5",
    "Alert 6",
    "Alert 7",
    "Alert 8",
    "Alert 9",
    "Alert 10",
    "Alert 11",
    "Alert 12",
    "Alert 13",
    "Alert 14",
    "Alert 15",
    "Alert 16",
    "Custom file",
];
/// Index of `Custom file` in [`SOUND_OPTIONS`].
pub(crate) const SOUND_CUSTOM: usize = 17;
pub(crate) const SOUND_FILE_DEFAULT: &str = "uwu.mp3";
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
            let intent = if view.alpha {
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
            } else {
                let cur = current.map(|c| [c.rgba[0], c.rgba[1], c.rgba[2]]);
                match widgets::colour_slot(ui, tag, label, cur, seed, width) {
                    widgets::SlotResult::Unchanged => continue,
                    widgets::SlotResult::Set(c) => StyleIntent::Colour {
                        slot,
                        value: Some([c[0], c[1], c[2], 255]),
                        opened_had_alpha: false,
                    },
                    widgets::SlotResult::Cleared => StyleIntent::Colour {
                        slot,
                        value: None,
                        opened_had_alpha: false,
                    },
                }
            };
            out.push(intent);
        }
    });

    font_line(ui, &skin, view, memo, n, &mut out);
    minimap_lines(ui, &skin, view, memo, n, &mut out);
    beam_line(ui, view, memo, n, &mut out);
    sound_lines(ui, &skin, view, memo, n, &mut out);

    ff_field_row_line(ui, n, "Drop sound", Some(DROP_TIP), |ui| {
        let mut on = view.disable_drop_sound;
        let r = crate::skin::widgets::ff_check(ui, &skin, &mut on, "Drop sound");
        probe("ed-drop-check", r.rect);
        if r.changed() {
            out.push(StyleIntent::DisableDropSound(on));
        }
    });
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

fn minimap_lines(
    ui: &mut egui::Ui,
    skin: &crate::skin::Skin,
    view: &StyleView,
    memo: egui::Id,
    n: &mut usize,
    out: &mut Vec<StyleIntent>,
) {
    ff_field_row_line(ui, n, "Minimap", None, |ui| {
        let mut sel = view.minimap.as_ref().map_or(0, |m| usize::from(m.size) + 1);
        let width = 288.0f32.min(ui.available_width());
        let r = ed::ff_segmented(
            ui,
            skin,
            &mut sel,
            &["None", "Large", "Medium", "Small"],
            width,
        );
        probe("ed-minimap", r.rect);
        if r.changed() {
            let next = if sel == 0 {
                if let Some(m) = &view.minimap {
                    remember(ui, memo, "minimap", m);
                }
                None
            } else {
                let mut m = view
                    .minimap
                    .clone()
                    .or_else(|| recall(ui, memo, "minimap"))
                    .unwrap_or(Minimap {
                        size: 0,
                        color: FilterColor::White,
                        shape: MinimapShape::Circle,
                    });
                m.size = u8::try_from(sel - 1).unwrap_or(0);
                Some(m)
            };
            out.push(StyleIntent::Minimap(next));
        }
    });
    let Some(m) = &view.minimap else {
        return;
    };
    let colour = minimap::index_of(&minimap::COLOURS, m.color.as_str()).unwrap_or(0);
    let shape = minimap::index_of(&minimap::SHAPES, m.shape.as_str()).unwrap_or(0);
    ff_field_row_line(ui, n, "", None, |ui| {
        ui.spacing_mut().item_spacing.x = 4.0;
        let tile = tile_size(ui, 11.0, 0.0);
        for (i, label) in minimap::COLOURS.iter().enumerate() {
            let r = ed::ff_swatch(
                ui,
                Some(minimap::SWATCHES[i]),
                i == colour,
                vec2(tile, 24.0),
                label,
            )
            .on_hover_text(*label);
            probe("ed-mm-colour", r.rect);
            if r.clicked()
                && i != colour
                && let Some(color) = FilterColor::from_name(label)
            {
                let mut next = m.clone();
                next.color = color;
                out.push(StyleIntent::Minimap(Some(next)));
            }
        }
    });
    ff_field_row_line(ui, n, "", None, |ui| {
        ui.spacing_mut().item_spacing.x = 4.0;
        let tile = tile_size(ui, 12.0, 0.0);
        for (i, label) in minimap::SHAPES.iter().enumerate() {
            let r = ed::ff_icon_button(ui, skin, i, colour, i == shape, tile).on_hover_text(*label);
            probe("ed-mm-shape", r.rect);
            if r.clicked()
                && i != shape
                && let Some(s) = MinimapShape::from_name(label)
            {
                let mut next = m.clone();
                next.shape = s;
                out.push(StyleIntent::Minimap(Some(next)));
            }
        }
    });
}

fn beam_line(
    ui: &mut egui::Ui,
    view: &StyleView,
    memo: egui::Id,
    n: &mut usize,
    out: &mut Vec<StyleIntent>,
) {
    ff_field_row_line(ui, n, "Beam", None, |ui| {
        ui.spacing_mut().item_spacing.x = 4.0;
        // Room for the Temp chip after the 12 tiles.
        let tile = tile_size(ui, 12.0, 62.0);
        let none = ed::ff_swatch(ui, None, view.beam.is_none(), vec2(tile, 24.0), "No beam")
            .on_hover_text("None");
        probe("ed-beam-none", none.rect);
        if none.clicked()
            && let Some(b) = &view.beam
        {
            remember(ui, memo, "beam", b);
            out.push(StyleIntent::Beam(None));
        }
        let current = view
            .beam
            .as_ref()
            .and_then(|b| minimap::index_of(&minimap::COLOURS, b.color.as_str()));
        for (i, label) in minimap::COLOURS.iter().enumerate() {
            let r = ed::ff_swatch(
                ui,
                Some(minimap::SWATCHES[i]),
                current == Some(i),
                vec2(tile, 24.0),
                label,
            )
            .on_hover_text(*label);
            probe("ed-beam-colour", r.rect);
            if r.clicked()
                && current != Some(i)
                && let Some(color) = FilterColor::from_name(label)
            {
                let mut next = view
                    .beam
                    .clone()
                    .or_else(|| recall(ui, memo, "beam"))
                    .unwrap_or(Beam {
                        color: FilterColor::White,
                        temp: false,
                    });
                next.color = color;
                out.push(StyleIntent::Beam(Some(next)));
            }
        }
        let mut temp = view.beam.as_ref().is_some_and(|b| b.temp);
        ui.add_enabled_ui(view.beam.is_some(), |ui| {
            let r = ed::ff_chip(ui, "Temp", &mut temp);
            probe("ed-beam-temp", r.rect);
            if r.changed()
                && let Some(b) = &view.beam
            {
                let mut next = b.clone();
                next.temp = temp;
                out.push(StyleIntent::Beam(Some(next)));
            }
        });
    });
}

/// The sound a dropdown choice stands for. `base` is the current sound, or the last one it had
/// before it was switched to None; it supplies the volume (default 100) and the file.
/// `last_file` is the custom file it had before it was switched away from Custom, used when
/// `base` has none.
pub(crate) fn sound_for_choice(
    choice: usize,
    base: Option<&Sound>,
    last_file: Option<&str>,
) -> Option<Sound> {
    let vol = base.map_or(100, |s| s.vol);
    match choice {
        0 => None,
        SOUND_CUSTOM => Some(Sound {
            id: None,
            custom: Some(
                base.and_then(|s| s.custom.clone())
                    .or_else(|| last_file.map(str::to_string))
                    .unwrap_or_else(|| SOUND_FILE_DEFAULT.to_string()),
            ),
            vol,
        }),
        k => Some(Sound {
            id: u8::try_from(k).ok(),
            custom: None,
            vol,
        }),
    }
}

/// The sound dropdown's switch-back memory: the sound before a switch to None, and the custom
/// file before a switch away from Custom (WR-02: through an Alert the file used to be lost).
#[derive(Debug, Default, PartialEq)]
pub(crate) struct SoundMemo {
    pub(crate) last: Option<Sound>,
    pub(crate) file: Option<String>,
}

/// What switching the sound dropdown to `choice` writes, given the `current` sound; updates
/// `memo` with what the switch replaces.
pub(crate) fn switch_sound(
    choice: usize,
    current: Option<&Sound>,
    memo: &mut SoundMemo,
) -> Option<Sound> {
    let base = current.cloned().or_else(|| memo.last.clone());
    if choice == 0
        && let Some(s) = current
    {
        memo.last = Some(s.clone());
    }
    if choice != SOUND_CUSTOM
        && let Some(file) = current.and_then(|s| s.custom.as_ref())
    {
        memo.file = Some(file.clone());
    }
    sound_for_choice(choice, base.as_ref(), memo.file.as_deref())
}

/// A custom sound file name that is safe to write into `CustomAlertSound "<name>"`.
pub(crate) fn safe_file_name(name: &str) -> bool {
    !name.chars().any(|c| c == '"' || c.is_control())
}

fn sound_lines(
    ui: &mut egui::Ui,
    skin: &crate::skin::Skin,
    view: &StyleView,
    memo: egui::Id,
    n: &mut usize,
    out: &mut Vec<StyleIntent>,
) {
    ff_field_row_line(ui, n, "Sound", None, |ui| {
        ui.spacing_mut().item_spacing.x = 8.0;
        let mut sel = match &view.sound {
            None => 0,
            Some(s) if s.custom.is_some() => SOUND_CUSTOM,
            Some(s) => usize::from(s.id.unwrap_or(1)).clamp(1, 16),
        };
        let r =
            crate::skin::widgets::ff_dropdown(ui, skin, "sound", &mut sel, &SOUND_OPTIONS, 150.0);
        probe("ed-sound", r.rect);
        if r.changed() {
            let mut sm = SoundMemo {
                last: recall::<Sound>(ui, memo, "sound"),
                file: recall::<String>(ui, memo, "sound-file"),
            };
            let next = switch_sound(sel, view.sound.as_ref(), &mut sm);
            if let Some(s) = &sm.last {
                remember(ui, memo, "sound", s);
            }
            if let Some(f) = &sm.file {
                remember(ui, memo, "sound-file", f);
            }
            out.push(StyleIntent::Sound(next));
        }
        if let Some(sound) = &view.sound
            && let Some(file) = &sound.custom
        {
            // Keyed by the editor's memo id so two editors never share a draft.
            let draft_id = ui.id().with(("style-file-draft", memo));
            let had: Option<String> = ui.data(|d| d.get_temp(draft_id));
            let mut buf = had.clone().unwrap_or_else(|| file.clone());
            let field_id = ui.id().with("file");
            let resp = crate::skin::widgets::ff_text_field_id(
                ui,
                field_id,
                &mut buf,
                SOUND_FILE_DEFAULT,
                150.0,
            );
            probe("ed-sound-file", resp.rect);
            let bad = !safe_file_name(&buf);
            if resp.changed() {
                ui.data_mut(|d| d.insert_temp(draft_id, buf.clone()));
                if !bad && !buf.trim().is_empty() {
                    out.push(StyleIntent::Sound(Some(Sound {
                        id: None,
                        custom: Some(buf.trim().to_string()),
                        vol: sound.vol,
                    })));
                }
            } else if !resp.has_focus() && had.is_some() {
                ui.data_mut(|d| d.remove_temp::<String>(draft_id));
            }
            if bad {
                ui.painter().rect_stroke(
                    resp.rect,
                    2.0,
                    egui::Stroke::new(1.0, INVALID_RED),
                    egui::StrokeKind::Inside,
                );
                resp.on_hover_text(BAD_FILE_TIP);
            } else {
                resp.on_hover_text(FILE_TIP);
            }
        }
    });
    let Some(sound) = &view.sound else {
        return;
    };
    ff_field_row_line(ui, n, "Volume", None, |ui| {
        let mut v = f64::from(sound.vol);
        let r =
            crate::skin::widgets::ff_slider(ui, skin, &mut v, 0.0..=300.0, 1.0, "Volume", |v| {
                format!("{v:.0}")
            });
        probe("ed-volume", r.rect);
        if r.changed()
            && let Ok(vol) = u16::try_from(v.round() as i64)
        {
            out.push(StyleIntent::Sound(Some(Sound {
                id: sound.id,
                custom: sound.custom.clone(),
                vol,
            })));
        }
    });
}
