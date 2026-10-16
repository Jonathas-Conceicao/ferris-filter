//! Style-editor pieces in the game skin: a segmented switch (Show / Hide, minimap size pills),
//! colour swatches and the Text / Border / Background cards, the item-label preview, minimap
//! icons and the framed cards that group them.
//!
//! There is deliberately no colour picker here yet: the swatches and cards only show a colour
//! (or "none") and report clicks.

use egui::{
    Align2, Color32, CornerRadius, InnerResponse, Painter, Pos2, Rect, Response, Sense, Stroke,
    StrokeKind, Ui, Vec2, WidgetInfo, WidgetType, pos2, vec2,
};

use super::Skin;
use super::fonts::{regular, small_caps};
use super::minimap;
use super::palette as pal;
use super::slice;

/// Height of a segmented switch.
pub const SEGMENT_HEIGHT: f32 = 30.0;

/// A segmented switch on the game's button art: one segment per option, the selected one lit
/// (hover art, pale gold caption). `width` of 0 fills the available width. Marks the response
/// changed when the selection changes.
pub fn ff_segmented(
    ui: &mut Ui,
    skin: &Skin,
    selected: &mut usize,
    options: &[&str],
    width: f32,
) -> Response {
    let width = if width > 0.0 {
        width
    } else {
        ui.available_width()
    };
    let (rect, mut resp) = ui.allocate_exact_size(vec2(width, SEGMENT_HEIGHT), Sense::hover());
    let n = options.len().max(1);
    let seg_w = rect.width() / n as f32;
    let painter = ui.painter().clone();
    let ppp = painter.pixels_per_point();
    let paint_art = |p: &Painter, state: usize, tint: Color32| {
        let art = &skin.button[state];
        slice::paint_three(
            p,
            [&art.left, &art.middle, &art.right],
            slice::three_h_rects(rect, art.left.size(), art.right.size(), ppp),
            tint,
        );
    };
    paint_art(&painter, 0, Color32::from_gray(170));
    let mut changed = false;
    for (i, opt) in options.iter().enumerate() {
        let seg = Rect::from_min_size(
            pos2(rect.min.x + seg_w * i as f32, rect.min.y),
            vec2(seg_w, rect.height()),
        );
        let r = ui.interact(seg, resp.id.with(i), Sense::click());
        let is_sel = *selected == i;
        r.widget_info(|| WidgetInfo::selected(WidgetType::RadioButton, true, is_sel, *opt));
        if r.clicked() && !is_sel {
            *selected = i;
            changed = true;
        }
        let is_sel = *selected == i;
        if is_sel {
            paint_art(&painter.with_clip_rect(seg), 1, Color32::WHITE);
        }
        if i > 0 {
            painter.line_segment(
                [
                    pos2(seg.min.x, seg.min.y + 7.0),
                    pos2(seg.min.x, seg.max.y - 7.0),
                ],
                Stroke::new(1.0, Color32::from_black_alpha(160)),
            );
        }
        let color = if is_sel {
            pal::HEADING
        } else if r.hovered() {
            pal::LABEL
        } else {
            pal::DISABLED
        };
        painter.text(
            seg.center() + vec2(0.0, 1.0),
            Align2::CENTER_CENTER,
            *opt,
            small_caps(16.0),
            color,
        );
        resp |= r;
    }
    if changed {
        resp.mark_changed();
    }
    resp
}

/// Paints the "no colour" mark: a dark box crossed by a thin red diagonal.
fn paint_none(painter: &Painter, rect: Rect) {
    painter.rect_filled(rect, CornerRadius::same(1), pal::INPUT_FILL);
    let r = rect.shrink(3.0);
    painter.line_segment(
        [r.left_bottom(), r.right_top()],
        Stroke::new(1.5, Color32::from_rgb(0xA0, 0x3A, 0x30)),
    );
}

/// A colour swatch `size` big: the colour, or the "none" mark for `None`. A selected swatch
/// gets a pale gold ring; a hovered one a warm border. `label` is for assistive tech.
pub fn ff_swatch(
    ui: &mut Ui,
    color: Option<Color32>,
    selected: bool,
    size: Vec2,
    label: &str,
) -> Response {
    let (rect, resp) = ui.allocate_exact_size(size, Sense::click());
    resp.widget_info(|| WidgetInfo::selected(WidgetType::RadioButton, true, selected, label));
    let painter = ui.painter();
    let inner = rect.shrink(2.0);
    match color {
        Some(c) => {
            painter.rect_filled(inner, CornerRadius::same(1), c);
            // A soft top highlight and bottom shade, so flat colours read as game "gems".
            let top = Rect::from_min_max(inner.min, pos2(inner.max.x, inner.center().y));
            painter.rect_filled(top, CornerRadius::same(1), Color32::from_white_alpha(18));
        }
        None => paint_none(painter, inner),
    }
    let (stroke, width) = if selected {
        (pal::HEADING, 2.0)
    } else if resp.hovered() {
        (pal::INPUT_BORDER_HOT, 1.0)
    } else {
        (Color32::from_black_alpha(200), 1.0)
    };
    painter.rect_stroke(
        rect.shrink(if selected { 0.0 } else { 1.5 }),
        CornerRadius::same(2),
        Stroke::new(width, stroke),
        StrokeKind::Inside,
    );
    if selected {
        painter.rect_stroke(
            inner,
            CornerRadius::same(1),
            Stroke::new(1.0, Color32::from_black_alpha(220)),
            StrokeKind::Inside,
        );
    }
    resp
}

/// A text colour readable on `bg`: near-black on light colours, cream on dark ones.
pub fn on_color(bg: Color32) -> Color32 {
    let l = 0.299 * bg.r() as f32 + 0.587 * bg.g() as f32 + 0.114 * bg.b() as f32;
    if l > 150.0 {
        Color32::from_rgb(0x1A, 0x16, 0x10)
    } else {
        pal::VALUE
    }
}

/// One of the Text / Border / Background cards: a box filled with the colour, its name on it
/// ("Text", or "Text (none)" on the dark "none" ground). `width` of 0 fills the width.
pub fn ff_colour_card(ui: &mut Ui, label: &str, color: Option<Color32>, width: f32) -> Response {
    let width = if width > 0.0 {
        width
    } else {
        ui.available_width()
    };
    let (rect, resp) = ui.allocate_exact_size(vec2(width, 34.0), Sense::click());
    resp.widget_info(|| WidgetInfo::labeled(WidgetType::Button, true, label));
    let painter = ui.painter();
    let (text, text_color) = match color {
        Some(c) => {
            painter.rect_filled(rect, CornerRadius::same(1), c);
            painter.rect_filled(
                Rect::from_min_max(rect.min, pos2(rect.max.x, rect.center().y)),
                CornerRadius::same(1),
                Color32::from_white_alpha(14),
            );
            (label.to_owned(), on_color(c))
        }
        None => {
            painter.rect_filled(rect, CornerRadius::same(1), pal::INPUT_FILL);
            (format!("{label} (none)"), pal::DISABLED)
        }
    };
    painter.rect_stroke(
        rect,
        CornerRadius::same(1),
        Stroke::new(
            1.0,
            if resp.hovered() {
                pal::INPUT_BORDER_HOT
            } else {
                pal::INPUT_BORDER
            },
        ),
        StrokeKind::Inside,
    );
    painter.with_clip_rect(rect.shrink(2.0)).text(
        pos2(rect.min.x + 10.0, rect.center().y + 1.0),
        Align2::LEFT_CENTER,
        text,
        small_caps(15.0),
        text_color,
    );
    resp
}

/// What the item-label preview draws.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LabelLook {
    /// `SetTextColor`; `None` keeps the game's colour (a rare item's yellow).
    pub text: Option<Color32>,
    /// `SetBorderColor`; `None` draws no border.
    pub border: Option<Color32>,
    /// `SetBackgroundColor`; `None` keeps the game's translucent black.
    pub background: Option<Color32>,
    /// `SetFontSize` (1-45); `None` is the game default (32).
    pub font_size: Option<f32>,
    /// A Hide style: drawn dimmed, as an Alt-revealed label.
    pub hidden: bool,
}

/// The game's own label colours when a filter sets none.
const DEFAULT_TEXT: Color32 = Color32::from_rgb(0xFF, 0xFF, 0x77);
const DEFAULT_BG: Color32 = Color32::from_rgba_premultiplied(0, 0, 0, 220);
/// The game's default `SetFontSize`.
pub const DEFAULT_FONT_SIZE: f32 = 32.0;

/// Points per filter font-size unit in the preview: a 32 label is 16 points, a 45 one ~23.
pub const PREVIEW_SCALE: f32 = 0.5;

/// Paints an item-label preview of `look` centred in `rect` on a dark "ground" strip, with the
/// item name `text` in Fontin SmallCaps (the game's label face).
pub fn ff_item_label(painter: &Painter, rect: Rect, text: &str, look: LabelLook) {
    // The ground: a dark, slightly warm strip, like the floor under a dropped item.
    painter.rect_filled(
        rect,
        CornerRadius::same(2),
        Color32::from_rgb(0x14, 0x12, 0x0E),
    );
    painter.rect_stroke(
        rect,
        CornerRadius::same(2),
        Stroke::new(1.0, Color32::from_black_alpha(200)),
        StrokeKind::Inside,
    );
    let size = look.font_size.unwrap_or(DEFAULT_FONT_SIZE).clamp(1.0, 45.0) * PREVIEW_SCALE;
    let fade = if look.hidden { 0.45 } else { 1.0 };
    let text_c = look.text.unwrap_or(DEFAULT_TEXT).gamma_multiply(fade);
    let galley = painter.layout_no_wrap(text.to_owned(), small_caps(size), text_c);
    let pad = vec2(size * 0.55, size * 0.18);
    let label = Rect::from_center_size(rect.center(), galley.size() + 2.0 * pad);
    let label = slice::snap_rect(label, painter.pixels_per_point());
    painter.rect_filled(
        label,
        CornerRadius::ZERO,
        look.background.unwrap_or(DEFAULT_BG).gamma_multiply(fade),
    );
    if let Some(b) = look.border {
        painter.rect_stroke(
            label,
            CornerRadius::ZERO,
            Stroke::new(1.5, b.gamma_multiply(fade)),
            StrokeKind::Inside,
        );
    }
    painter.galley(
        label.center() - galley.size() / 2.0 + vec2(0.0, 1.0),
        galley,
        text_c,
    );
}

/// Paints the game's minimap icon for `shape` in `colour` (indices into
/// [`minimap::SHAPES`] / [`minimap::COLOURS`]) as a square `size` points across.
pub fn paint_minimap_icon(
    painter: &Painter,
    skin: &Skin,
    center: Pos2,
    shape: usize,
    colour: usize,
    size: f32,
    tint: Color32,
) {
    let tex = &skin.minimap_icons;
    let rect = slice::snap_rect(
        Rect::from_center_size(center, Vec2::splat(size)),
        painter.pixels_per_point(),
    );
    painter.image(
        tex.id(),
        rect,
        minimap::icon_uv(shape, colour, tex.size()),
        tint,
    );
}

/// A square button showing a minimap icon (the shape grid): dark tile, the icon in `colour`,
/// a pale gold ring when `selected`.
pub fn ff_icon_button(
    ui: &mut Ui,
    skin: &Skin,
    shape: usize,
    colour: usize,
    selected: bool,
    size: f32,
) -> Response {
    let (rect, resp) = ui.allocate_exact_size(Vec2::splat(size), Sense::click());
    let name = minimap::SHAPES.get(shape).copied().unwrap_or("");
    resp.widget_info(|| WidgetInfo::selected(WidgetType::RadioButton, true, selected, name));
    let painter = ui.painter();
    let fill = if selected {
        Color32::from_rgba_unmultiplied(200, 162, 90, 40)
    } else if resp.hovered() {
        pal::ROW_HOVER
    } else {
        Color32::from_black_alpha(140)
    };
    painter.rect_filled(rect, CornerRadius::same(2), fill);
    painter.rect_stroke(
        rect,
        CornerRadius::same(2),
        Stroke::new(
            if selected { 1.5 } else { 1.0 },
            if selected {
                pal::HEADING
            } else if resp.hovered() {
                pal::INPUT_BORDER_HOT
            } else {
                pal::INPUT_BORDER
            },
        ),
        StrokeKind::Inside,
    );
    let tint = if selected || resp.hovered() {
        Color32::WHITE
    } else {
        Color32::from_gray(190)
    };
    paint_minimap_icon(
        painter,
        skin,
        rect.center(),
        shape,
        colour,
        size * 0.8,
        tint,
    );
    resp.on_hover_text(name)
}

/// A framed group inside the style editor: a dark translucent card with a thin border and a
/// small Fontin SmallCaps caption, holding `add_contents`.
pub fn ff_card<R>(
    ui: &mut Ui,
    title: &str,
    add_contents: impl FnOnce(&mut Ui) -> R,
) -> InnerResponse<R> {
    egui::Frame::NONE
        .fill(Color32::from_black_alpha(110))
        .stroke(Stroke::new(1.0, Color32::from_rgb(0x2C, 0x29, 0x23)))
        .corner_radius(CornerRadius::same(2))
        .inner_margin(egui::Margin::symmetric(10, 8))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.label(
                egui::RichText::new(title)
                    .font(small_caps(14.0))
                    .color(pal::GOLD),
            );
            ui.add_space(4.0);
            add_contents(ui)
        })
}

/// A small caption in body text (e.g. "Volume" next to a slider).
pub fn caption(ui: &mut Ui, text: &str) -> Response {
    ui.label(
        egui::RichText::new(text)
            .font(regular(15.0))
            .color(pal::MUTED),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn readable_text_on_light_and_dark_swatches() {
        assert_eq!(
            on_color(Color32::WHITE),
            Color32::from_rgb(0x1A, 0x16, 0x10)
        );
        assert_eq!(on_color(Color32::from_rgb(40, 42, 54)), pal::VALUE);
    }

    #[test]
    fn segmented_switch_selects_the_clicked_option() {
        use egui::{Event, Modifiers, PointerButton, RawInput};
        let ctx = egui::Context::default();
        super::super::fonts::install(&ctx);
        let skin = Skin::load(&ctx);
        let mut sel = 0;
        let mut rect = Rect::NOTHING;
        ctx.run_ui(RawInput::default(), |ui| {
            rect = ff_segmented(ui, &skin, &mut sel, &["Show", "Hide"], 200.0).rect;
        })
        .drop_without_applying_deltas();
        let pos = pos2(rect.max.x - 20.0, rect.center().y);
        let click = RawInput {
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
        };
        let mut changed = false;
        ctx.run_ui(click, |ui| {
            changed = ff_segmented(ui, &skin, &mut sel, &["Show", "Hide"], 200.0).changed();
        })
        .drop_without_applying_deltas();
        assert_eq!(sel, 1);
        assert!(changed);
    }
}
