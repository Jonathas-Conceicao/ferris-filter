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

/// The id of segment `i` of a segmented control whose whole-control response is `resp`. Not
/// `resp.id.with(i)`: egui names a widget's tooltip area `widget_id.with(0)`, so segment 0 would
/// share its id with the control's own tooltip.
fn segment_id(resp: &Response, i: usize) -> egui::Id {
    resp.id.with(("segment", i))
}

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
        let r = ui.interact(seg, segment_id(&resp, i), Sense::click());
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

/// Height of the one-line levels switch (two stacked caption lines fit).
pub const LEVELS_HEIGHT: f32 = 40.0;

/// One segment of [`ff_levels`]: its caption lines (two for a stacked two-word caption), its
/// tooltip, and whether a click on it selects it (a display-only segment senses hover only).
pub struct Level<'a> {
    pub lines: &'a [&'a str],
    pub tip: &'a str,
    pub clickable: bool,
}

/// Segment widths for a row `total` wide whose captions need `naturals` (widest line, in points):
/// natural width plus 16pt padding plus an equal share of the slack; when the row is too narrow
/// the padding drops to 8pt, and if even that does not fit the widths shrink proportionally.
/// The widths always sum to `total`.
pub fn levels_layout(total: f32, naturals: &[f32]) -> Vec<f32> {
    let n = naturals.len();
    if n == 0 {
        return Vec::new();
    }
    let sum: f32 = naturals.iter().sum();
    for pad in [16.0, 8.0] {
        let slack = total - sum - pad * n as f32;
        if slack >= 0.0 {
            return naturals
                .iter()
                .map(|w| w + pad + slack / n as f32)
                .collect();
        }
    }
    let need = sum + 8.0 * n as f32;
    naturals
        .iter()
        .map(|w| (w + 8.0) * total.max(0.0) / need.max(f32::EPSILON))
        .collect()
}

/// A one-line, multi-segment switch in the look of [`ff_segmented`] (same art, dividers and
/// captions), with content-proportional segments and two-line captions. Returns the combined
/// response and the index of a clickable, not-yet-selected segment that was clicked.
pub fn ff_levels(
    ui: &mut Ui,
    skin: &Skin,
    selected: usize,
    levels: &[Level],
) -> (Response, Option<usize>) {
    let width = ui.available_width();
    let (rect, mut resp) = ui.allocate_exact_size(vec2(width, LEVELS_HEIGHT), Sense::hover());
    let naturals: Vec<f32> = levels
        .iter()
        .map(|l| {
            l.lines
                .iter()
                .map(|t| {
                    ui.painter()
                        .layout_no_wrap((*t).to_owned(), small_caps(14.0), pal::LABEL)
                        .size()
                        .x
                })
                .fold(0.0, f32::max)
        })
        .collect();
    let widths = levels_layout(rect.width(), &naturals);
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
    let mut clicked = None;
    let mut x = rect.min.x;
    for (i, level) in levels.iter().enumerate() {
        let seg = Rect::from_min_size(pos2(x, rect.min.y), vec2(widths[i], rect.height()));
        x += widths[i];
        let sense = if level.clickable {
            Sense::click()
        } else {
            Sense::hover()
        };
        let r = ui.interact(seg, segment_id(&resp, i), sense);
        let is_sel = selected == i;
        let name = level.lines.join(" ");
        r.widget_info(|| {
            WidgetInfo::selected(WidgetType::RadioButton, level.clickable, is_sel, &name)
        });
        if level.clickable && !is_sel && r.clicked() {
            clicked = Some(i);
        }
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
        } else if level.clickable && r.hovered() {
            pal::LABEL
        } else {
            pal::DISABLED
        };
        let mut size = 14.0;
        for s in [14.0, 13.0, 12.0] {
            size = s;
            let widest = level
                .lines
                .iter()
                .map(|t| {
                    painter
                        .layout_no_wrap((*t).to_owned(), small_caps(s), color)
                        .size()
                        .x
                })
                .fold(0.0, f32::max);
            if widest <= seg.width() - 8.0 {
                break;
            }
        }
        let count = level.lines.len().max(1) as f32;
        for (k, line) in level.lines.iter().enumerate() {
            let dy = (k as f32 - (count - 1.0) / 2.0) * 14.0 + 1.0;
            painter.text(
                seg.center() + vec2(0.0, dy),
                Align2::CENTER_CENTER,
                *line,
                small_caps(size),
                color,
            );
        }
        resp |= r.on_hover_text(level.tip);
    }
    (resp, clicked)
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

/// `c` (premultiplied) laid over the opaque `ground`.
pub fn composite_over(c: Color32, ground: Color32) -> Color32 {
    let k = 255 - c.a() as u32;
    let ch = |top: u8, under: u8| (top as u32 + (under as u32 * k + 127) / 255).min(255) as u8;
    Color32::from_rgb(ch(c.r(), ground.r()), ch(c.g(), ground.g()), ch(c.b(), ground.b()))
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
    ff_colour_card_tip(ui, label, color, width, open, None)
}

/// [`ff_colour_card_ex`] with an optional tooltip (the `R G B A` of a translucent colour). A
/// translucent colour is painted over the card ground, not the panel behind it.
pub fn ff_colour_card_tip(
    ui: &mut Ui,
    label: &str,
    color: Option<Color32>,
    width: f32,
    open: bool,
    tip: Option<&str>,
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
            painter.rect_filled(rect, CornerRadius::same(1), pal::INPUT_FILL);
            painter.rect_filled(rect, CornerRadius::same(1), c);
            painter.rect_filled(
                Rect::from_min_max(rect.min, pos2(rect.max.x, rect.center().y)),
                CornerRadius::same(1),
                Color32::from_white_alpha(14),
            );
            (label.to_owned(), on_color(composite_over(c, pal::INPUT_FILL)), false)
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
    // A shortened caption shows its full text on hover; a tip replaces it (joined when both).
    let hover = match (tip, cut) {
        (Some(t), true) => Some(format!("{text}: {t}")),
        (Some(t), false) => Some(t.to_owned()),
        (None, true) => Some(text),
        (None, false) => None,
    };
    match hover {
        // Also in a disabled Ui (a read-only economy rule).
        Some(h) => resp.on_hover_text(&h).on_disabled_hover_text(h),
        None => resp,
    }
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
    fn translucent_card_caption_follows_the_composited_colour() {
        let faint = Color32::from_rgba_unmultiplied(255, 255, 255, 20);
        assert_eq!(
            on_color(composite_over(faint, pal::INPUT_FILL)),
            pal::VALUE
        );
        assert_ne!(
            on_color(composite_over(Color32::WHITE, pal::INPUT_FILL)),
            pal::VALUE
        );
        // An opaque colour composites to itself.
        let c = Color32::from_rgb(10, 200, 30);
        assert_eq!(composite_over(c, pal::INPUT_FILL), c);
    }

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

    const LEVEL_CAPTIONS: [&[&str]; 7] = [
        &["Custom"],
        &["Flat", "Broke"],
        &["Scrounging"],
        &["Getting", "By"],
        &["Comfy"],
        &["Loaded"],
        &["Div", "Farmer"],
    ];

    fn levels_of(clickable_first: bool) -> Vec<Level<'static>> {
        LEVEL_CAPTIONS
            .iter()
            .enumerate()
            .map(|(i, l)| Level {
                lines: l,
                tip: "tip",
                clickable: i > 0 || clickable_first,
            })
            .collect()
    }

    fn natural_widths(ctx: &egui::Context) -> Vec<f32> {
        let mut out = Vec::new();
        ctx.run_ui(egui::RawInput::default(), |ui| {
            for l in LEVEL_CAPTIONS {
                out.push(
                    l.iter()
                        .map(|t| {
                            ui.painter()
                                .layout_no_wrap((*t).to_owned(), small_caps(14.0), pal::LABEL)
                                .size()
                                .x
                        })
                        .fold(0.0, f32::max),
                );
            }
        })
        .drop_without_applying_deltas();
        out
    }

    #[test]
    fn levels_layout_sums_to_the_total_and_keeps_padding() {
        let ctx = egui::Context::default();
        super::super::fonts::install(&ctx);
        let nat = natural_widths(&ctx);
        for total in [517.0, 660.0] {
            let w = levels_layout(total, &nat);
            assert!((w.iter().sum::<f32>() - total).abs() < 0.5);
            for (w, n) in w.iter().zip(&nat) {
                assert!(*w >= n + 16.0 - 0.01, "{w} < {n} + 16");
            }
        }
        let a = levels_layout(517.0, &nat);
        let b = levels_layout(660.0, &nat);
        let extra = b[0] - a[0];
        for i in 0..7 {
            assert!((b[i] - a[i] - extra).abs() < 0.01, "slack shared equally");
        }
        let w = levels_layout(460.0, &nat);
        assert!((w.iter().sum::<f32>() - 460.0).abs() < 0.5);
        for (w, n) in w.iter().zip(&nat) {
            assert!(*w >= n + 8.0 - 0.01, "{w} < {n} + 8");
        }
        assert!(levels_layout(100.0, &nat).iter().all(|w| *w >= 0.0));
    }

    #[test]
    fn levels_row_is_one_40pt_row_and_every_caption_fits() {
        let ctx = egui::Context::default();
        super::super::fonts::install(&ctx);
        let skin = Skin::load(&ctx);
        let nat = natural_widths(&ctx);
        let widths = levels_layout(517.0, &nat);
        for (w, n) in widths.iter().zip(&nat) {
            assert!(n <= &(w - 8.0), "caption {n} does not fit {w}");
        }
        let levels = levels_of(false);
        let mut rect = Rect::NOTHING;
        let mut before = 0.0;
        let mut after = 0.0;
        ctx.run_ui(egui::RawInput::default(), |ui| {
            ui.set_max_width(517.0);
            before = ui.cursor().min.y;
            rect = ff_levels(ui, &skin, 0, &levels).0.rect;
            after = ui.cursor().min.y;
        })
        .drop_without_applying_deltas();
        assert_eq!(rect.height(), LEVELS_HEIGHT);
        assert!((rect.width() - 517.0).abs() < 0.5);
        assert!(after - before >= LEVELS_HEIGHT && after - before < LEVELS_HEIGHT + 10.0);
    }

    /// Selected `sel`, click at fraction `at` across the row; the reported click.
    fn levels_click(first_clickable: bool, sel: usize, seg: usize) -> Option<usize> {
        use egui::{Event, Modifiers, PointerButton, RawInput};
        let ctx = egui::Context::default();
        super::super::fonts::install(&ctx);
        let skin = Skin::load(&ctx);
        let nat = natural_widths(&ctx);
        let levels = levels_of(first_clickable);
        let mut rect = Rect::NOTHING;
        ctx.run_ui(RawInput::default(), |ui| {
            ui.set_max_width(517.0);
            rect = ff_levels(ui, &skin, sel, &levels).0.rect;
        })
        .drop_without_applying_deltas();
        let widths = levels_layout(rect.width(), &nat);
        let left = rect.min.x + widths[..seg].iter().sum::<f32>();
        let pos = pos2(left + widths[seg] / 2.0, rect.center().y);
        let btn = |pressed| Event::PointerButton {
            pos,
            button: PointerButton::Primary,
            pressed,
            modifiers: Modifiers::default(),
        };
        let click = RawInput {
            events: vec![Event::PointerMoved(pos), btn(true), btn(false)],
            ..Default::default()
        };
        let mut got = None;
        ctx.run_ui(click, |ui| {
            ui.set_max_width(517.0);
            got = ff_levels(ui, &skin, sel, &levels).1;
        })
        .drop_without_applying_deltas();
        got
    }

    #[test]
    fn levels_report_clicks_only_on_clickable_unselected_segments() {
        assert_eq!(levels_click(false, 0, 3), Some(3));
        assert_eq!(levels_click(false, 3, 3), None);
        assert_eq!(levels_click(false, 3, 0), None);
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

    /// Hovers the first segment of `draw`'s control for several frames with tooltips shown at
    /// once. egui's debug build asserts (panics) when a widget id is reused on another layer, which
    /// is what a segment sharing its id with the control's tooltip area does.
    fn hover_first_segment(draw: impl Fn(&mut Ui, &Skin) -> Rect) {
        use egui::{Event, RawInput};
        let ctx = egui::Context::default();
        super::super::fonts::install(&ctx);
        ctx.global_style_mut(|s| s.interaction.tooltip_delay = 0.0);
        let skin = Skin::load(&ctx);
        let mut rect = Rect::NOTHING;
        ctx.run_ui(RawInput::default(), |ui| rect = draw(ui, &skin))
            .drop_without_applying_deltas();
        let pos = pos2(rect.min.x + 10.0, rect.center().y);
        for frame in 0..6 {
            let input = RawInput {
                time: Some(frame as f64 * 0.2),
                events: if frame == 0 {
                    vec![Event::PointerMoved(pos)]
                } else {
                    vec![]
                },
                ..Default::default()
            };
            ctx.run_ui(input, |ui| {
                draw(ui, &skin);
            })
            .drop_without_applying_deltas();
        }
    }

    #[test]
    fn segmented_switch_tooltip_does_not_clash_with_its_first_segment() {
        hover_first_segment(|ui, skin| {
            let mut sel = 0;
            ff_segmented(ui, skin, &mut sel, &["Live", "Fallback"], 200.0)
                .on_hover_text("Price source")
                .rect
        });
    }

    #[test]
    fn levels_row_tooltip_does_not_clash_with_its_first_segment() {
        let levels = levels_of(false);
        hover_first_segment(|ui, skin| {
            ui.set_max_width(517.0);
            ff_levels(ui, skin, 1, &levels).0.on_hover_text("Strictness").rect
        });
    }
}
