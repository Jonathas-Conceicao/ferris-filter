//! Style-editor pieces in the game skin: a segmented switch (Show / Hide, minimap size pills),
//! colour swatches and the Text / Border / Background cards, the item-label preview, minimap
//! icons and the framed cards that group them.
//!
//! The swatches and cards only show a colour (or "none") and report clicks; the picker they open
//! lives in `skin::colour_picker`.

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
    } else if color.is_none() {
        // The dark "none" tile needs a visible frame: a black one vanishes on the darker zebra
        // band, which made the `/` tile of one group look boxless next to another's.
        (pal::INPUT_BORDER, 1.0)
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
    ff_colour_card_ex(ui, label, color, width, false)
}

/// [`ff_colour_card`] that draws the 2pt `pal::HEADING` ring (as a selected swatch does) while its
/// colour picker is `open`.
pub fn ff_colour_card_ex(
    ui: &mut Ui,
    label: &str,
    color: Option<Color32>,
    width: f32,
    open: bool,
) -> Response {
    let width = if width > 0.0 {
        width
    } else {
        ui.available_width()
    };
    let (rect, resp) = ui.allocate_exact_size(vec2(width, 34.0), Sense::click());
    resp.widget_info(|| WidgetInfo::labeled(WidgetType::Button, true, label));
    let painter = ui.painter();
    let (text, text_color, none) = match color {
        Some(c) => {
            painter.rect_filled(rect, CornerRadius::same(1), c);
            painter.rect_filled(
                Rect::from_min_max(rect.min, pos2(rect.max.x, rect.center().y)),
                CornerRadius::same(1),
                Color32::from_white_alpha(14),
            );
            (label.to_owned(), on_color(c), false)
        }
        None => {
            painter.rect_filled(rect, CornerRadius::same(1), pal::INPUT_FILL);
            (format!("{label} (none)"), pal::DISABLED, true)
        }
    };
    let stroke = if open {
        Stroke::new(2.0, pal::HEADING)
    } else if resp.hovered() {
        Stroke::new(1.0, pal::INPUT_BORDER_HOT)
    } else {
        Stroke::new(1.0, pal::INPUT_BORDER)
    };
    painter.rect_stroke(rect, CornerRadius::same(1), stroke, StrokeKind::Inside);
    // 10pt before the caption (as drawn) and 4pt kept clear of the right edge.
    let room = rect.width() - 14.0;
    let shown = card_caption(ui, &text, label, none, room);
    let cut = shown != text;
    let mut job = egui::text::LayoutJob::simple_singleline(shown, small_caps(15.0), text_color);
    job.wrap = egui::text::TextWrapping {
        max_width: room.max(1.0),
        max_rows: 1,
        break_anywhere: true,
        overflow_character: Some('…'),
    };
    let galley = ui.fonts_mut(|f| f.layout_job(job));
    let cut = cut || galley.elided;
    let painter = ui.painter();
    painter.with_clip_rect(rect.shrink(2.0)).galley(
        pos2(
            rect.min.x + 10.0,
            rect.center().y + 1.0 - galley.size().y / 2.0,
        ),
        galley,
        text_color,
    );
    // A shortened caption shows its full text on hover.
    if cut { resp.on_hover_text(text) } else { resp }
}

/// The caption a colour card shows in `room` points: `text` when it fits, else (for a "none"
/// card) the bare `label`, which the dim colour already marks as none. Whatever is returned is
/// still cut with "…" by the caller if it does not fit either.
fn card_caption(ui: &Ui, text: &str, label: &str, none: bool, room: f32) -> String {
    let width = |t: &str| {
        ui.fonts_mut(|f| {
            f.layout_no_wrap(t.to_owned(), small_caps(15.0), Color32::WHITE)
                .size()
                .x
        })
    };
    if !none || width(text) <= room {
        text.to_owned()
    } else {
        label.to_owned()
    }
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

/// A multi-select pill: 28pt high, SmallCaps 17, 12pt side padding. Off is a recessed input box;
/// on has a gold border and a faint gold fill. Clicking flips `selected`.
pub fn ff_chip(ui: &mut Ui, label: &str, selected: &mut bool) -> Response {
    let galley = ui
        .painter()
        .layout_no_wrap(label.to_owned(), small_caps(17.0), pal::LABEL);
    let size = vec2(galley.size().x + 24.0, 28.0);
    let (rect, mut resp) = ui.allocate_exact_size(size, Sense::click());
    resp.widget_info(|| WidgetInfo::selected(WidgetType::SelectableLabel, true, *selected, label));
    if resp.clicked() {
        *selected = !*selected;
        resp.mark_changed();
    }
    let (fill, border, text) = if *selected {
        (
            Color32::from_rgba_unmultiplied(pal::GOLD.r(), pal::GOLD.g(), pal::GOLD.b(), 60),
            pal::GOLD,
            pal::HEADING,
        )
    } else {
        (
            pal::INPUT_FILL,
            if resp.hovered() {
                pal::INPUT_BORDER_HOT
            } else {
                pal::INPUT_BORDER
            },
            pal::LABEL,
        )
    };
    let painter = ui.painter();
    painter.rect(
        rect,
        CornerRadius::same(1),
        fill,
        Stroke::new(1.0, border),
        StrokeKind::Inside,
    );
    painter.galley(rect.center() - galley.size() / 2.0, galley, text);
    resp
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

    /// The painted stroke colours and widths of the card rectangle.
    fn card_strokes(open: bool) -> Vec<(f32, Color32)> {
        let ctx = egui::Context::default();
        super::super::fonts::install(&ctx);
        let full = ctx.run_ui(egui::RawInput::default(), |ui| {
            ff_colour_card_ex(ui, "Text", Some(Color32::from_rgb(1, 2, 3)), 110.0, open);
        });
        let mut out = Vec::new();
        fn walk(s: &egui::Shape, out: &mut Vec<(f32, Color32)>) {
            match s {
                egui::Shape::Rect(r) => out.push((r.stroke.width, r.stroke.color)),
                egui::Shape::Vec(v) => v.iter().for_each(|s| walk(s, out)),
                _ => {}
            }
        }
        for c in &full.shapes {
            walk(&c.shape, &mut out);
        }
        full.drop_without_applying_deltas();
        out
    }

    /// The painted stroke colours of one unselected swatch.
    fn swatch_strokes(color: Option<Color32>) -> Vec<Color32> {
        let ctx = egui::Context::default();
        let full = ctx.run_ui(egui::RawInput::default(), |ui| {
            ff_swatch(ui, color, false, vec2(24.0, 24.0), "x");
        });
        let mut out = Vec::new();
        for c in &full.shapes {
            if let egui::Shape::Rect(r) = &c.shape {
                out.push(r.stroke.color);
            }
        }
        full.drop_without_applying_deltas();
        out
    }

    #[test]
    fn an_unselected_none_tile_has_a_visible_frame() {
        // The `/` tile of every group (Minimap, Beam, Alert sound) looks the same on either band.
        assert!(swatch_strokes(None).contains(&pal::INPUT_BORDER));
        assert!(!swatch_strokes(Some(Color32::RED)).contains(&pal::INPUT_BORDER));
    }

    #[test]
    fn a_none_card_caption_drops_none_when_it_does_not_fit() {
        let ctx = egui::Context::default();
        super::super::fonts::install(&ctx);
        let mut got = Vec::new();
        ctx.run_ui(egui::RawInput::default(), |ui| {
            for (none, room) in [(true, 400.0), (true, 90.0), (false, 90.0)] {
                got.push(card_caption(
                    ui,
                    "Background (none)",
                    "Background",
                    none,
                    room,
                ));
            }
        })
        .drop_without_applying_deltas();
        assert_eq!(
            got,
            ["Background (none)", "Background", "Background (none)"]
        );
    }

    #[test]
    fn an_open_colour_card_shows_the_heading_ring() {
        assert!(card_strokes(true).contains(&(2.0, pal::HEADING)));
        assert!(!card_strokes(false).contains(&(2.0, pal::HEADING)));
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
