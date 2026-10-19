//! The skinned controls. Every control paints the game's art where the game has art for it and
//! falls back to the [`palette`](super::palette) for the rest (row grounds, input boxes).
//!
//! Interactive controls follow the overlay's `allocate -> interact -> paint` shape, take the
//! value by `&mut`, call `Response::mark_changed` when it changes, and report themselves to
//! AccessKit through `Response::widget_info`.

use std::hash::Hash;
use std::ops::RangeInclusive;

use egui::{
    Align, Align2, Color32, CornerRadius, EventFilter, InnerResponse, Key, Layout, Painter, Rect,
    Response, Sense, Shape, Stroke, StrokeKind, TextureHandle, Ui, UiBuilder, Vec2, WidgetInfo,
    WidgetType, pos2, vec2,
};

use super::Skin;
use super::fonts::{regular, small_caps};
use super::palette as pal;
use super::slice::{self, Margins};

// ---------------------------------------------------------------------------------------------
// Background and frame

/// Paints the window ground and the static background drawing (Ferris), fixed to `window` so
/// content scrolling over it never moves it. The drawing is 2x art, shown at half size and
/// scaled down further if the window is narrower.
pub fn ff_background(painter: &Painter, skin: &Skin, window: Rect) {
    painter.rect_filled(window, 0.0, pal::GROUND);
    let [w, h] = skin.ferris_bg.size();
    let mut size = vec2(w as f32 / 2.0, h as f32 / 2.0);
    if size.x > window.width() {
        size *= window.width() / size.x;
    }
    let rect = Rect::from_center_size(window.center(), size);
    slice::paint_image(painter, &skin.ferris_bg, rect, Color32::WHITE);
}

/// The Options page frame's slice margins, in texels of the 4K art (1310x1582): the stone
/// border, the thin gold inner line, and the recessed button bar at the bottom.
pub const FRAME_MARGINS: Margins = Margins::new(104.0, 150.0, 100.0, 200.0);

/// Where things go inside a frame drawn by [`ff_frame`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FrameLayout {
    /// The whole frame.
    pub outer: Rect,
    /// Inside the thin gold line: the scrollable content goes here.
    pub content: Rect,
    /// The recessed bar at the bottom (the game's SAVE / DEFAULTS / CLOSE row).
    pub button_bar: Rect,
    /// Points per texel the frame is drawn at.
    pub scale: f32,
}

/// The layout of a frame filling `outer`, for a frame texture `tex_size` texels big. The frame
/// is scaled to `outer`'s width; its height stretches.
pub fn frame_layout(outer: Rect, tex_size: [usize; 2]) -> FrameLayout {
    let s = outer.width() / tex_size[0].max(1) as f32;
    let m = FRAME_MARGINS;
    let content = Rect::from_min_max(
        pos2(outer.min.x + m.left * s, outer.min.y + m.top * s),
        pos2(outer.max.x - m.right * s, outer.max.y - m.bottom * s),
    );
    let button_bar = Rect::from_min_max(
        pos2(outer.min.x + 94.0 * s, outer.max.y - 160.0 * s),
        pos2(outer.max.x - 92.0 * s, outer.max.y - 52.0 * s),
    );
    FrameLayout {
        outer,
        content,
        button_bar,
        scale: s,
    }
}

/// Paints the game's Options page frame into `outer` and returns its layout. The middle is not
/// painted (it is transparent in the art), so whatever is under it -- the window ground and the
/// background drawing -- shows through.
pub fn ff_frame(painter: &Painter, skin: &Skin, outer: Rect) -> FrameLayout {
    let layout = frame_layout(outer, skin.pages_frame.size());
    slice::paint_nine(
        painter,
        &skin.pages_frame,
        outer,
        FRAME_MARGINS,
        layout.scale,
        Color32::WHITE,
        false,
    );
    layout
}

/// The rows of the page frame's top border (in texels of the 4K art) that hold the tabs: from
/// just below the outer gold line (rows 41-47) to the top of the light stone strip (row 103),
/// the dark band where the game's Options window seats its tabs.
pub const TAB_STRIP_TEXELS: (f32, f32) = (48.0, 103.0);

/// Where [`ff_tabs`] goes on a frame: the dark band of its top border, as wide as the content.
/// Each stone is drawn at this full height, its bottom resting on the light stone strip and its
/// top clear of the gold line and of the header art above.
pub fn tab_strip(frame: &FrameLayout) -> Rect {
    let (top, bottom) = TAB_STRIP_TEXELS;
    Rect::from_min_max(
        pos2(frame.content.min.x, frame.outer.min.y + top * frame.scale),
        pos2(
            frame.content.max.x,
            frame.outer.min.y + bottom * frame.scale,
        ),
    )
}

// ---------------------------------------------------------------------------------------------
// Header, close button, tabs

/// The header's height for a window `width` points wide: the header art's aspect ratio.
pub fn header_height(skin: &Skin, width: f32) -> f32 {
    let [w, h] = skin.title_bar.size();
    width * h as f32 / w.max(1) as f32
}

/// The gold title plate inside a header drawn into `rect`: centred, on the gold bar at ~72% of
/// the art's height. The overlay hangs the filter-path tooltip on it.
pub fn header_plate_rect(rect: Rect) -> Rect {
    Rect::from_center_size(
        pos2(rect.center().x, rect.min.y + rect.height() * 0.717),
        vec2(rect.width() * 0.4, rect.height() * 0.3),
    )
}

/// Paints the header block into `rect` and returns the close button's response.
///
/// `art` is the header picture: the prototype passes the game's Options title bar; `None`
/// paints a plain placeholder plate (the slot reserved for FerrisFilter's own header art). The
/// title sits on the plate (the gold bar at ~72% of the art's height) in Fontin SmallCaps; the
/// round close button sits at the right on the same line.
pub fn ff_header(
    ui: &mut Ui,
    skin: &Skin,
    rect: Rect,
    art: Option<&TextureHandle>,
    title: &str,
) -> Response {
    let painter = ui.painter_at(rect);
    let plate_y = header_plate_rect(rect).center().y;
    match art {
        Some(tex) => slice::paint_image(&painter, tex, rect, Color32::WHITE),
        None => {
            painter.rect_filled(rect, 0.0, pal::INPUT_FILL);
            let plate = header_plate_rect(rect);
            painter.rect(
                plate,
                2.0,
                pal::GOLD,
                Stroke::new(1.0, pal::HEADING),
                StrokeKind::Inside,
            );
        }
    }
    let font = small_caps((rect.height() * 0.215).clamp(14.0, 30.0));
    // A soft light halo under the dark letters, like the engraved "Options".
    painter.text(
        pos2(rect.center().x + 0.5, plate_y + 1.5),
        Align2::CENTER_CENTER,
        title,
        font.clone(),
        Color32::from_rgba_unmultiplied(255, 236, 190, 70),
    );
    painter.text(
        pos2(rect.center().x, plate_y + 1.0),
        Align2::CENTER_CENTER,
        title,
        font,
        pal::ON_GOLD,
    );
    let size = (rect.height() * 0.3).clamp(20.0, 34.0);
    ff_close(ui, skin, pos2(rect.max.x - size * 0.9, plate_y - 2.0), size)
}

/// The game's round red close button, `size` points across, centred on `center`.
pub fn ff_close(ui: &mut Ui, skin: &Skin, center: egui::Pos2, size: f32) -> Response {
    let rect = Rect::from_center_size(center, Vec2::splat(size));
    let resp = ui.interact(rect, ui.id().with("ff_close"), Sense::click());
    resp.widget_info(|| WidgetInfo::labeled(WidgetType::Button, true, "Close"));
    let state = if resp.is_pointer_button_down_on() {
        2
    } else if resp.hovered() {
        1
    } else {
        0
    };
    slice::paint_image(ui.painter(), &skin.close[state], rect, Color32::WHITE);
    resp
}

/// Where one tab of a [`tab_layout`] goes.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TabGeom {
    /// The whole stone, slanted tail included.
    pub rect: Rect,
    /// Where the caption is centred: the middle of the stone's flat top, not of `rect` (the
    /// tail on the right is not label space).
    pub label_x: f32,
}

/// Lays out a row of game tabs `h` points high, centred on `center_x`, for captions
/// `label_widths` points wide. `left` / `right` are the cap textures' sizes in texels.
///
/// Like the game's Options tabs, each stone is left cap + flat middle (caption plus padding) +
/// right cap, whose long slanted tail runs under the next stone's rounded left edge; tabs
/// overlap by a fixed amount, so the gaps between captions only depend on caption widths. The
/// caption is centred on the flat top: from the middle of the left curve to just past the start
/// of the tail.
pub fn tab_layout(
    center_x: f32,
    top: f32,
    h: f32,
    label_widths: &[f32],
    left: [usize; 2],
    right: [usize; 2],
) -> Vec<TabGeom> {
    let lw = left[0] as f32 * h / left[1].max(1) as f32;
    let rw = right[0] as f32 * h / right[1].max(1) as f32;
    let pad = h * 0.4;
    let overlap = rw * TAB_OVERLAP;
    let mids: Vec<f32> = label_widths
        .iter()
        .map(|w| (w + 2.0 * pad).max(h * 1.6))
        .collect();
    let total: f32 = mids.iter().map(|m| lw + m + rw).sum::<f32>()
        - overlap * label_widths.len().saturating_sub(1) as f32;
    let mut x = center_x - total / 2.0;
    mids.iter()
        .map(|m| {
            let rect = Rect::from_min_size(pos2(x, top), vec2(lw + m + rw, h));
            let body_l = x + lw * 0.5;
            let body_r = x + lw + m + rw * 0.2;
            x += lw + m + rw - overlap;
            TabGeom {
                rect,
                label_x: (body_l + body_r) / 2.0,
            }
        })
        .collect()
}

/// How much of the right cap's width runs under the next tab.
pub const TAB_OVERLAP: f32 = 0.3;

/// A row of game tabs centred in `rect` (see [`tab_layout`]). The selected tab is drawn last
/// (on top of its neighbours' tails) at full brightness with pale gold text; the others are
/// dimmed. Returns the union of the tab responses, marked changed when the selection changed.
pub fn ff_tabs(
    ui: &mut Ui,
    skin: &Skin,
    rect: Rect,
    labels: &[&str],
    selected: &mut usize,
) -> Response {
    ff_tabs_ex(ui, skin, rect, labels, selected, &[])
}

/// [`ff_tabs`] with an optional hover tooltip per tab (`tips[i]`) and Left / Right arrow
/// selection while a tab has keyboard focus.
pub fn ff_tabs_ex(
    ui: &mut Ui,
    skin: &Skin,
    rect: Rect,
    labels: &[&str],
    selected: &mut usize,
    tips: &[Option<&str>],
) -> Response {
    let painter = ui.painter().clone();
    let h = rect.height();
    let font = small_caps((h * 0.58).clamp(12.0, 22.0));
    let widths: Vec<f32> = labels
        .iter()
        .map(|l| {
            painter
                .layout_no_wrap((*l).to_owned(), font.clone(), Color32::WHITE)
                .size()
                .x
        })
        .collect();
    let art = &skin.tab;
    let geoms = tab_layout(
        rect.center().x,
        rect.min.y,
        h,
        &widths,
        art.left.size(),
        art.right.size(),
    );
    // Interact first, on the flat bodies only (they never overlap, so a neighbour's tail never
    // steals a click), then paint with the selected stone last.
    let mut resps: Vec<Option<Response>> = vec![None; labels.len()];
    let mut changed = false;
    for (i, g) in geoms.iter().enumerate() {
        let hit = Rect::from_min_max(
            pos2(g.rect.min.x + h * 0.3, g.rect.min.y),
            pos2(
                g.rect.max.x
                    - art.right.size()[0] as f32 * h / art.right.size()[1].max(1) as f32 * 0.6,
                g.rect.max.y,
            ),
        );
        let mut resp = ui.interact(hit, ui.id().with(("ff_tab", i)), Sense::click());
        if let Some(Some(tip)) = tips.get(i) {
            resp = resp.on_hover_text(*tip);
        }
        let is_sel = *selected == i;
        let label = labels[i];
        resp.widget_info(|| WidgetInfo::selected(WidgetType::SelectableLabel, true, is_sel, label));
        if resp.clicked() && !is_sel {
            *selected = i;
            changed = true;
        }
        if resp.has_focus() {
            ui.memory_mut(|m| {
                m.set_focus_lock_filter(
                    resp.id,
                    EventFilter {
                        horizontal_arrows: true,
                        vertical_arrows: false,
                        tab: false,
                        escape: false,
                    },
                )
            });
            let (left, right) = ui.input(|inp| {
                (
                    inp.key_pressed(Key::ArrowLeft),
                    inp.key_pressed(Key::ArrowRight),
                )
            });
            let next = if left && i > 0 {
                Some(i - 1)
            } else if right && i + 1 < labels.len() {
                Some(i + 1)
            } else {
                None
            };
            if let Some(n) = next {
                *selected = n;
                changed = true;
                ui.memory_mut(|m| m.request_focus(ui.id().with(("ff_tab", n))));
            }
        }
        resps[i] = Some(resp);
    }
    let order: Vec<usize> = (0..labels.len())
        .filter(|i| *i != *selected)
        .chain((*selected < labels.len()).then_some(*selected))
        .collect();
    for i in order {
        let g = geoms[i];
        let resp = resps[i].as_ref().expect("every tab has a response");
        let is_sel = *selected == i;
        let tint = if is_sel {
            Color32::WHITE
        } else if resp.hovered() {
            Color32::from_gray(200)
        } else {
            Color32::from_gray(150)
        };
        let rects = slice::three_h_rects(
            g.rect,
            art.left.size(),
            art.right.size(),
            painter.pixels_per_point(),
        );
        slice::paint_three(&painter, [&art.left, &art.middle, &art.right], rects, tint);
        let color = if is_sel {
            pal::HEADING
        } else if resp.hovered() {
            pal::LABEL
        } else {
            pal::MUTED
        };
        // Cream letters with a dark drop shadow, as the game draws its tab captions.
        let center = pos2(g.label_x, g.rect.center().y + h * 0.06);
        painter.text(
            center + vec2(1.0, 1.0),
            Align2::CENTER_CENTER,
            labels[i],
            font.clone(),
            Color32::from_black_alpha(220),
        );
        painter.text(
            center,
            Align2::CENTER_CENTER,
            labels[i],
            font.clone(),
            color,
        );
    }
    let mut resp = resps
        .into_iter()
        .flatten()
        .reduce(|a, b| a | b)
        .unwrap_or_else(|| ui.interact(rect, ui.id().with("ff_tabs"), Sense::hover()));
    if changed {
        resp.mark_changed();
    }
    resp
}

// ---------------------------------------------------------------------------------------------
// Sections and rows

/// A section heading ("Display Settings"): Fontin SmallCaps in pale gold over a faint gold rule.
pub fn ff_section(ui: &mut Ui, skin: &Skin, title: &str) -> Response {
    ui.add_space(8.0);
    let galley = ui
        .painter()
        .layout_no_wrap(title.to_owned(), small_caps(21.0), pal::HEADING);
    let (rect, resp) = ui.allocate_exact_size(
        vec2(ui.available_width(), galley.size().y + 12.0),
        Sense::hover(),
    );
    resp.widget_info(|| WidgetInfo::labeled(WidgetType::Label, true, title));
    let painter = ui.painter();
    painter.galley(
        pos2(rect.min.x + 6.0, rect.min.y + 2.0),
        galley,
        pal::HEADING,
    );
    let rule = Rect::from_min_size(pos2(rect.min.x, rect.max.y - 5.0), vec2(rect.width(), 3.0));
    slice::paint_image(
        painter,
        &skin.title_underline,
        rule,
        Color32::from_white_alpha(150),
    );
    ui.add_space(2.0);
    resp
}

/// Height of a settings row.
pub const ROW_HEIGHT: f32 = 34.0;
/// Width of the control column on the right of a settings row.
pub const CONTROL_WIDTH: f32 = 252.0;
/// Inner horizontal padding of a row.
pub const ROW_PAD: f32 = 10.0;

/// Paints a row ground: alternating subtle stripes, warmer while hovered.
pub fn row_background(ui: &Ui, index: usize, rect: Rect) {
    let color = if ui.rect_contains_pointer(rect) {
        pal::ROW_HOVER
    } else if index.is_multiple_of(2) {
        pal::ROW_EVEN
    } else {
        pal::ROW_ODD
    };
    ui.painter().rect_filled(rect, 0.0, color);
}

/// A settings row: `label` on the left in Fontin SmallCaps, the control on the right in a
/// fixed-width column ([`CONTROL_WIDTH`]), vertically centred, on an alternating ground.
pub fn ff_row<R>(
    ui: &mut Ui,
    index: usize,
    label: &str,
    add_control: impl FnOnce(&mut Ui) -> R,
) -> InnerResponse<R> {
    let (rect, resp) =
        ui.allocate_exact_size(vec2(ui.available_width(), ROW_HEIGHT), Sense::hover());
    row_background(ui, index, rect);
    let galley = ui
        .painter()
        .layout_no_wrap(label.to_owned(), small_caps(17.0), pal::LABEL);
    let label_end = rect.min.x + ROW_PAD + galley.size().x;
    // The control column starts at the usual place, or after a label too long for it (a lone
    // checkbox then sits right after its label instead of on top of it).
    let control_min = (rect.max.x - ROW_PAD - CONTROL_WIDTH)
        .max(label_end + 12.0)
        .min(rect.max.x - ROW_PAD - 24.0);
    ui.painter()
        .with_clip_rect(Rect::from_min_max(
            rect.min,
            pos2(control_min - 4.0, rect.max.y),
        ))
        .galley(
            pos2(
                rect.min.x + ROW_PAD,
                rect.center().y - galley.size().y / 2.0,
            ),
            galley,
            pal::LABEL,
        );
    let control = Rect::from_min_max(
        pos2(control_min, rect.min.y),
        pos2(rect.max.x - ROW_PAD, rect.max.y),
    );
    let mut child = ui.new_child(
        UiBuilder::new()
            .max_rect(control)
            .layout(Layout::left_to_right(Align::Center)),
    );
    let inner = add_control(&mut child);
    InnerResponse::new(inner, resp)
}

/// Width of the label column of [`ff_field_row`].
pub const LABEL_COL_W: f32 = 128.0;
/// Gap between the label column and the control of [`ff_field_row`].
pub const LABEL_GAP: f32 = 16.0;

/// An editor line: `label` in a fixed 128pt column, the control starting 16pt after it and
/// filling the rest of the row. Unlike [`ff_row`] the row is as tall as its control needs
/// (at least [`ROW_HEIGHT`]), and a control that does not fit wraps. The label is centred on
/// the first 34pt line; `tip` is its tooltip.
pub fn ff_field_row<R>(
    ui: &mut Ui,
    index: usize,
    label: &str,
    tip: Option<&str>,
    add_control: impl FnOnce(&mut Ui) -> R,
) -> InnerResponse<R> {
    let full = ui.available_rect_before_wrap();
    let left = full.min.x;
    let right = ui.max_rect().max.x.min(full.max.x);
    let control_min_x = left + ROW_PAD + LABEL_COL_W + LABEL_GAP;
    let ground = ui.painter().add(Shape::Noop);
    let control = Rect::from_min_max(
        pos2(control_min_x, full.min.y),
        pos2(
            (right - ROW_PAD).max(control_min_x + 24.0),
            full.min.y + ROW_HEIGHT,
        ),
    );
    let mut child = ui.new_child(
        UiBuilder::new()
            .max_rect(control)
            .layout(Layout::left_to_right(Align::Center).with_main_wrap(true)),
    );
    let inner = add_control(&mut child);
    let used = child.min_rect().height();
    let height = if used > ROW_HEIGHT {
        used + 4.0
    } else {
        ROW_HEIGHT
    };
    let rect = Rect::from_min_size(pos2(left, full.min.y), vec2(right - left, height));
    let resp = ui.allocate_rect(rect, Sense::hover());
    let color = if ui.rect_contains_pointer(rect) {
        pal::ROW_HOVER
    } else if index.is_multiple_of(2) {
        pal::ROW_EVEN
    } else {
        pal::ROW_ODD
    };
    ui.painter()
        .set(ground, Shape::rect_filled(rect, 0.0, color));
    let galley = ui.painter().layout_job(field_label_job(label));
    let elided = galley.elided;
    let label_pos = pos2(
        left + ROW_PAD,
        full.min.y + ROW_HEIGHT / 2.0 - galley.size().y / 2.0,
    );
    let label_clip = Rect::from_min_max(rect.min, pos2(control_min_x - 4.0, rect.max.y));
    ui.painter()
        .with_clip_rect(label_clip)
        .galley(label_pos, galley.clone(), pal::LABEL);
    // A cut label shows its full text on hover (above the tip, when there is one).
    let hover = match (elided, tip) {
        (true, Some(tip)) => Some(format!("{label}\n{tip}")),
        (true, None) => Some(label.to_owned()),
        (false, Some(tip)) => Some(tip.to_owned()),
        (false, None) => None,
    };
    if let Some(hover) = hover {
        let label_rect = Rect::from_min_size(label_pos, galley.size()).intersect(label_clip);
        let id = ui.id().with(("ff-field-label", label));
        ui.interact(label_rect, id, Sense::hover())
            .on_hover_text(hover);
    }
    InnerResponse::new(inner, resp)
}

/// The label of an [`ff_field_row`]: small caps 17 on one line, cut with "…" so it never runs
/// past the [`LABEL_COL_W`] column (`SetBackgroundColor` reads `SET BACKGROUND…`, never a bare
/// `SET BACKGROUND C`).
pub fn field_label_job(label: &str) -> egui::text::LayoutJob {
    let mut job = egui::text::LayoutJob::single_section(
        label.replace(['\n', '\r'], " "),
        egui::TextFormat::simple(small_caps(17.0), pal::LABEL),
    );
    job.wrap = egui::text::TextWrapping {
        max_width: LABEL_COL_W + LABEL_GAP - 4.0,
        max_rows: 1,
        break_anywhere: true,
        overflow_character: Some('…'),
    };
    job
}

// ---------------------------------------------------------------------------------------------
// Check, slider, dropdown, button, small arrows, search box

/// The game's round checkbox: an empty ring, or a ring with a gold tick. Flips `checked` on a
/// click. `label` is what assistive tech reads.
pub fn ff_check(ui: &mut Ui, skin: &Skin, checked: &mut bool, label: &str) -> Response {
    let size = 22.0;
    let (rect, mut resp) = ui.allocate_exact_size(Vec2::splat(size), Sense::click());
    let state = *checked;
    resp.widget_info(|| WidgetInfo::selected(WidgetType::Checkbox, true, state, label));
    if resp.clicked() {
        *checked = !*checked;
        resp.mark_changed();
    }
    let tex = if *checked {
        &skin.check_on
    } else {
        &skin.check_off
    };
    let [w, h] = tex.size();
    let draw = Rect::from_min_size(rect.min, vec2(size, size * h as f32 / w.max(1) as f32));
    let tint = if resp.hovered() || resp.has_focus() {
        Color32::WHITE
    } else {
        Color32::from_gray(222)
    };
    slice::paint_image(ui.painter(), tex, draw, tint);
    resp
}

/// Maps a pointer x inside `track` (with `inset` points of dead zone at each end) to a value in
/// `range`, rounded to `step`.
pub fn slider_value_at(
    x: f32,
    track: Rect,
    inset: f32,
    range: &RangeInclusive<f64>,
    step: f64,
) -> f64 {
    let usable = (track.width() - 2.0 * inset).max(1.0);
    let t = ((x - track.min.x - inset) / usable).clamp(0.0, 1.0) as f64;
    let v = range.start() + t * (range.end() - range.start());
    snap_value(v, range, step)
}

/// Rounds `v` to `step` (relative to the range start) and clamps it into `range`.
pub fn snap_value(v: f64, range: &RangeInclusive<f64>, step: f64) -> f64 {
    let v = if step > 0.0 {
        range.start() + ((v - range.start()) / step).round() * step
    } else {
        v
    };
    v.clamp(*range.start(), *range.end())
}

/// The game's slider: dark pill track, gold fill up to the diamond thumb, value on the right.
///
/// Drag or click to set; when focused (it takes focus on click), Left/Right step by `step`,
/// Home/End jump to the ends. `format` renders the value text; `label` is for assistive tech.
pub fn ff_slider(
    ui: &mut Ui,
    skin: &Skin,
    value: &mut f64,
    range: RangeInclusive<f64>,
    step: f64,
    label: &str,
    format: impl Fn(f64) -> String,
) -> Response {
    let width = ui.available_width().max(120.0);
    // Room for the widest value text the range can produce, so it never overlaps the track.
    let value_w = [*range.start(), *range.end(), *value]
        .iter()
        .map(|v| {
            ui.painter()
                .layout_no_wrap(format(*v), regular(15.0), pal::VALUE)
                .size()
                .x
        })
        .fold(0.0_f32, f32::max)
        + 12.0;
    let (rect, mut resp) = ui.allocate_exact_size(vec2(width, 24.0), Sense::click_and_drag());
    let track = Rect::from_min_max(
        pos2(rect.min.x + 4.0, rect.center().y - 6.0),
        pos2(rect.max.x - value_w, rect.center().y + 6.0),
    );
    let inset = 6.0;
    let before = *value;

    if resp.clicked() || resp.drag_started() {
        resp.request_focus();
    }
    if (resp.dragged() || resp.clicked())
        && let Some(p) = resp.interact_pointer_pos()
    {
        *value = slider_value_at(p.x, track, inset, &range, step);
    }
    if resp.has_focus() {
        ui.memory_mut(|m| {
            m.set_focus_lock_filter(
                resp.id,
                EventFilter {
                    horizontal_arrows: true,
                    vertical_arrows: false,
                    tab: false,
                    escape: false,
                },
            )
        });
        let (left, right, home, end) = ui.input(|i| {
            (
                i.key_pressed(Key::ArrowLeft),
                i.key_pressed(Key::ArrowRight),
                i.key_pressed(Key::Home),
                i.key_pressed(Key::End),
            )
        });
        if left {
            *value = snap_value(*value - step, &range, step);
        }
        if right {
            *value = snap_value(*value + step, &range, step);
        }
        if home {
            *value = *range.start();
        }
        if end {
            *value = *range.end();
        }
    }
    if *value != before {
        resp.mark_changed();
    }
    let v = *value;
    resp.widget_info(|| WidgetInfo::slider(true, v, label));

    let painter = ui.painter();
    let ppp = painter.pixels_per_point();
    let span = (range.end() - range.start()).max(f64::EPSILON);
    let t = ((v - range.start()) / span).clamp(0.0, 1.0) as f32;
    let thumb_x = track.min.x + inset + t * (track.width() - 2.0 * inset);
    let tr = &skin.slider_track;
    slice::paint_three(
        painter,
        [&tr.left, &tr.middle, &tr.right],
        slice::three_h_rects(track, tr.left.size(), tr.right.size(), ppp),
        Color32::WHITE,
    );
    let fill_rect = Rect::from_min_max(track.min, pos2(thumb_x + 2.0, track.max.y));
    if fill_rect.width() > 2.0 {
        let f = &skin.slider_fill;
        slice::paint_three(
            painter,
            [&f.left, &f.middle, &f.right],
            slice::three_h_rects(fill_rect, f.left.size(), f.right.size(), ppp),
            Color32::from_gray(210),
        );
    }
    let hot = resp.hovered() || resp.dragged() || resp.has_focus();
    let thumb = Rect::from_center_size(pos2(thumb_x, track.center().y), Vec2::splat(20.0));
    slice::paint_image(
        painter,
        &skin.slider_thumb,
        thumb,
        if hot {
            Color32::WHITE
        } else {
            Color32::from_gray(225)
        },
    );
    painter.text(
        pos2(rect.max.x - 2.0, rect.center().y),
        Align2::RIGHT_CENTER,
        format(v),
        regular(15.0),
        pal::VALUE,
    );
    resp
}

/// Paints the recessed input box used by dropdowns and the search field.
fn input_box(painter: &Painter, rect: Rect, hot: bool) {
    painter.rect(
        rect,
        CornerRadius::same(1),
        pal::INPUT_FILL,
        Stroke::new(
            1.0,
            if hot {
                pal::INPUT_BORDER_HOT
            } else {
                pal::INPUT_BORDER
            },
        ),
        StrokeKind::Inside,
    );
}

/// The game's dropdown: a dark box with the value on the left and the gold triangle on the
/// right; clicking opens a list in a gold-edged popup. `width` of 0 fills the available width.
pub fn ff_dropdown(
    ui: &mut Ui,
    skin: &Skin,
    id_salt: impl Hash + std::fmt::Debug,
    selected: &mut usize,
    options: &[&str],
    width: f32,
) -> Response {
    ff_dropdown_ex(ui, skin, id_salt, selected, options, width, None)
}

/// [`ff_dropdown`] whose closed caption can be painted in `caption_color` (e.g. `pal::WARN`
/// for an unknown style) instead of the usual value colour.
pub fn ff_dropdown_ex(
    ui: &mut Ui,
    skin: &Skin,
    id_salt: impl Hash + std::fmt::Debug,
    selected: &mut usize,
    options: &[&str],
    width: f32,
    caption_color: Option<Color32>,
) -> Response {
    let current = options.get(*selected).copied().unwrap_or("");
    let mut picked = None;
    let sel = *selected;
    let (mut resp, _) = dropdown_shell(
        ui,
        skin,
        id_salt,
        width,
        current,
        caption_color,
        options.len(),
        |ui, row_w, _| {
            for (i, opt) in options.iter().enumerate() {
                dropdown_row(ui, row_w, i, opt, sel, &mut picked);
            }
        },
    );
    if let Some(i) = picked
        && i != *selected
    {
        *selected = i;
        resp.mark_changed();
    }
    resp
}

/// One entry of [`ff_dropdown_items`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DropItem<'a> {
    /// A pickable option.
    Option(&'a str),
    /// A muted section header; never picked.
    Header(&'a str),
    /// A pickable option painted in the warning colour with the amber warning flag.
    Warn(&'a str),
}

impl DropItem<'_> {
    pub fn label(&self) -> &str {
        match self {
            DropItem::Option(s) | DropItem::Header(s) | DropItem::Warn(s) => s,
        }
    }
}

/// What [`ff_dropdown_items`] reports for one frame.
pub struct DropPick {
    pub response: Response,
    /// The item clicked this frame (headers never are), even when it is the selected one.
    pub picked: Option<usize>,
    /// The list opened this frame (a closed-to-open click).
    pub opened: bool,
}

/// [`ff_dropdown`] over sectioned items: options, muted headers and warning options. The closed
/// box shows `items[selected]` (or nothing), in `caption_color` when given; every click on an
/// option is reported, so the caller decides what picking the selected one means.
pub fn ff_dropdown_items(
    ui: &mut Ui,
    skin: &Skin,
    id_salt: impl Hash + std::fmt::Debug,
    selected: Option<usize>,
    items: &[DropItem],
    width: f32,
    caption_color: Option<Color32>,
) -> DropPick {
    let current = selected.and_then(|i| items.get(i));
    let caption = current.map_or("", |c| c.label());
    let caption_color =
        caption_color.or_else(|| matches!(current, Some(DropItem::Warn(_))).then_some(pal::WARN));
    let mut picked = None;
    let sel = selected.unwrap_or(usize::MAX);
    let (response, opened) = dropdown_shell(
        ui,
        skin,
        id_salt,
        width,
        caption,
        caption_color,
        items.len(),
        |ui, row_w, first| {
            for (i, item) in items.iter().enumerate() {
                // The selected row is scrolled into view when the list opens.
                let reveal = first && i == sel;
                match item {
                    DropItem::Option(opt) => {
                        dropdown_row_ex(ui, row_w, i, opt, sel, &mut picked, false, reveal)
                    }
                    DropItem::Warn(opt) => {
                        dropdown_row_ex(ui, row_w, i, opt, sel, &mut picked, true, reveal)
                    }
                    DropItem::Header(text) => {
                        let (r, _) =
                            ui.allocate_exact_size(vec2(row_w, DROPDOWN_ROW_H), Sense::hover());
                        ui.painter().text(
                            pos2(r.min.x + 6.0, r.center().y + 2.0),
                            Align2::LEFT_CENTER,
                            *text,
                            small_caps(14.0),
                            pal::MUTED,
                        );
                        ui.painter().line_segment(
                            [
                                pos2(r.min.x + 4.0, r.max.y - 3.0),
                                pos2(r.max.x - 4.0, r.max.y - 3.0),
                            ],
                            Stroke::new(1.0, Color32::from_black_alpha(160)),
                        );
                    }
                }
            }
        },
    );
    DropPick {
        response,
        picked,
        opened,
    }
}

/// The closed box and the popup list shared by the dropdowns; `rows` draws the `row_count`
/// list rows at a given row width. Returns the box's response and whether the list opened.
#[allow(clippy::too_many_arguments)]
fn dropdown_shell(
    ui: &mut Ui,
    skin: &Skin,
    id_salt: impl Hash + std::fmt::Debug,
    width: f32,
    caption: &str,
    caption_color: Option<Color32>,
    row_count: usize,
    mut rows: impl FnMut(&mut Ui, f32, bool),
) -> (Response, bool) {
    let width = if width > 0.0 {
        width
    } else {
        ui.available_width()
    };
    let id = ui.make_persistent_id(id_salt);
    let was_open = egui::Popup::is_id_open(ui.ctx(), id);
    let (rect, resp) = ui.allocate_exact_size(vec2(width, 28.0), Sense::click());
    resp.widget_info(|| WidgetInfo::labeled(WidgetType::ComboBox, true, caption));
    let open = egui::Popup::is_id_open(ui.ctx(), id);
    let hot = resp.hovered() || open;
    let painter = ui.painter();
    input_box(painter, rect, hot);
    painter.with_clip_rect(rect.shrink(2.0)).text(
        pos2(rect.min.x + 10.0, rect.center().y),
        Align2::LEFT_CENTER,
        caption,
        regular(15.0),
        caption_color.unwrap_or(pal::VALUE),
    );
    let arrow = Rect::from_center_size(
        pos2(rect.max.x - 15.0, rect.center().y + 1.0),
        Vec2::splat(18.0),
    );
    slice::paint_image(
        painter,
        &skin.combo_arrow[usize::from(hot)],
        arrow,
        Color32::WHITE,
    );

    // A long list scrolls inside a popup that fits the screen above or below the box (WR-05).
    let screen = ui.ctx().content_rect();
    let room = (screen.max.y - rect.max.y).max(rect.min.y - screen.min.y) - 2.0 - 12.0 - 4.0;
    let max_h = (DROPDOWN_MAX_ROWS * DROPDOWN_ROW_H)
        .min(room)
        .max(3.0 * DROPDOWN_ROW_H);
    let scroll = row_count as f32 * DROPDOWN_ROW_H > max_h;

    egui::Popup::from_toggle_button_response(&resp)
        .id(id)
        .width(width)
        .gap(2.0)
        .frame(egui::Frame::NONE.inner_margin(6))
        .show(|ui| {
            let bg = ui.painter().add(Shape::Noop);
            // True on the first frame the list is shown (every pass of it).
            let shown_id = id.with("shown");
            let frame = ui.ctx().cumulative_frame_nr();
            let (state, first) =
                reveal_state(ui.ctx().data(|d| d.get_temp::<(u64, u64)>(shown_id)), frame);
            ui.ctx().data_mut(|d| d.insert_temp(shown_id, state));
            ui.spacing_mut().item_spacing.y = 0.0;
            if scroll {
                ui.allocate_ui(vec2(width - 12.0, max_h), |ui| {
                    ui.spacing_mut().item_spacing.y = 0.0;
                    ff_scroll_area(ui, skin, (id, "list"), |ui| {
                        ui.spacing_mut().item_spacing.y = 0.0;
                        let row_w = ui.available_width();
                        rows(ui, row_w, first);
                    });
                });
            } else {
                rows(ui, width - 12.0, first);
            }
            let frame_rect = ui.min_rect().expand(6.0);
            let mut mesh = egui::Mesh::with_texture(skin.box_9slice.id());
            for p in slice::nine_slice(
                frame_rect,
                skin.box_9slice.size(),
                Margins::new(8.0, 8.0, 8.0, 8.0),
                1.0,
                ui.painter().pixels_per_point(),
            ) {
                mesh.add_rect_with_uv(p.dest, p.uv, Color32::WHITE);
            }
            ui.painter().set(bg, Shape::mesh(mesh));
        });
    let open_now = egui::Popup::is_id_open(ui.ctx(), id);
    if !open_now {
        ui.ctx()
            .data_mut(|d| d.remove::<(u64, u64)>(id.with("shown")));
    }
    let opened = !was_open && open_now;
    (resp, opened)
}

/// The dropdown list's reveal state: `(opened_at, last_shown)` frame numbers. Drawing the list on
/// `frame` returns the new state and whether this is the list's first frame (all its passes),
/// when the selected row is scrolled into view. A list not drawn on the previous frame starts
/// again, so a state left over from a list that closed while the widget was not drawn (a folded
/// group, a switched tab) cannot suppress the next reveal (IN-03).
fn reveal_state(prev: Option<(u64, u64)>, frame: u64) -> ((u64, u64), bool) {
    let opened_at = match prev {
        Some((at, last)) if last == frame || last + 1 == frame => at,
        _ => frame,
    };
    ((opened_at, frame), opened_at == frame)
}

/// Debug-only: asks the dropdown drawn under `tag` (e.g. the style editor's `sound`) to open
/// itself on its next draw (screenshot support). Compiled out of release builds.
#[cfg(debug_assertions)]
pub fn request_dropdown_open(ctx: &egui::Context, tag: &str) {
    ctx.data_mut(|d| d.insert_temp(egui::Id::new("ff-dropdown-shot"), tag.to_owned()));
}

/// Takes a [`request_dropdown_open`] for `tag`: true once, for the first dropdown that asks.
#[cfg(debug_assertions)]
pub fn take_dropdown_request(ctx: &egui::Context, tag: &str) -> bool {
    let key = egui::Id::new("ff-dropdown-shot");
    if ctx.data(|d| d.get_temp::<String>(key)).as_deref() == Some(tag) {
        ctx.data_mut(|d| d.remove::<String>(key));
        true
    } else {
        false
    }
}

/// Height of one option row in [`ff_dropdown_ex`]'s popup.
const DROPDOWN_ROW_H: f32 = 26.0;
/// Rows shown before [`ff_dropdown_ex`]'s popup scrolls.
const DROPDOWN_MAX_ROWS: f32 = 12.0;

/// One option row of [`ff_dropdown_ex`]'s popup; sets `picked` when clicked.
fn dropdown_row(
    ui: &mut Ui,
    row_w: f32,
    i: usize,
    opt: &str,
    selected: usize,
    picked: &mut Option<usize>,
) {
    dropdown_row_ex(ui, row_w, i, opt, selected, picked, false, false);
}

/// [`dropdown_row`], optionally as a warning row (caption in the warning colour, the amber
/// warning flag at its right end) and scrolled into view (`reveal`).
#[allow(clippy::too_many_arguments)]
fn dropdown_row_ex(
    ui: &mut Ui,
    row_w: f32,
    i: usize,
    opt: &str,
    selected: usize,
    picked: &mut Option<usize>,
    warn: bool,
    reveal: bool,
) {
    let (r, o) = ui.allocate_exact_size(vec2(row_w, DROPDOWN_ROW_H), Sense::click());
    if reveal {
        o.scroll_to_me(Some(egui::Align::Center));
    }
    o.widget_info(|| WidgetInfo::selected(WidgetType::SelectableLabel, true, i == selected, opt));
    if o.hovered() {
        ui.painter().rect_filled(r, 0.0, pal::ROW_HOVER);
    }
    let color = if warn {
        pal::WARN
    } else if i == selected {
        pal::HEADING
    } else if o.hovered() {
        pal::VALUE
    } else {
        pal::LABEL
    };
    ui.painter().text(
        pos2(r.min.x + 8.0, r.center().y),
        Align2::LEFT_CENTER,
        opt,
        regular(15.0),
        color,
    );
    if warn {
        super::icons::paint_flag_icon(
            ui.painter(),
            Rect::from_center_size(pos2(r.max.x - 14.0, r.center().y), Vec2::splat(16.0)),
            super::icons::FlagIcon::GrammarWarning,
            pal::WARN,
        );
    }
    if o.clicked() {
        *picked = Some(i);
    }
}

/// The game's generic button (3-slice art with hover / pressed states) with a Fontin SmallCaps
/// caption. At least `min_width` wide.
pub fn ff_button(ui: &mut Ui, skin: &Skin, text: &str, min_width: f32) -> Response {
    ff_button_ex(ui, skin, text, min_width, true, false)
}

/// [`ff_button`] that can be disabled (dimmed art and caption, never clicked, no hover state)
/// and can be marked dangerous (red caption).
pub fn ff_button_ex(
    ui: &mut Ui,
    skin: &Skin,
    text: &str,
    min_width: f32,
    enabled: bool,
    danger: bool,
) -> Response {
    // A button in a disabled `Ui` (a read-only economy rule) is drawn disabled too.
    let enabled = enabled && ui.is_enabled();
    let height = 34.0;
    let caption = if !enabled {
        pal::DISABLED
    } else if danger {
        pal::ERROR
    } else {
        pal::BUTTON_TEXT
    };
    let galley = ui
        .painter()
        .layout_no_wrap(text.to_owned(), small_caps(17.0), caption);
    let width = (galley.size().x + 44.0).max(min_width);
    let sense = if enabled {
        Sense::click()
    } else {
        Sense::hover()
    };
    let (rect, resp) = ui.allocate_exact_size(vec2(width, height), sense);
    resp.widget_info(|| WidgetInfo::labeled(WidgetType::Button, enabled, text));
    let state = if !enabled {
        0
    } else if resp.is_pointer_button_down_on() {
        2
    } else if resp.hovered() || resp.has_focus() {
        1
    } else {
        0
    };
    let tint = if enabled {
        Color32::WHITE
    } else {
        Color32::from_white_alpha(90)
    };
    let art = &skin.button[state];
    let painter = ui.painter();
    slice::paint_three(
        painter,
        [&art.left, &art.middle, &art.right],
        slice::three_h_rects(
            rect,
            art.left.size(),
            art.right.size(),
            painter.pixels_per_point(),
        ),
        tint,
    );
    let offset = if state == 2 { 1.0 } else { 0.0 };
    let pos = rect.center() - galley.size() / 2.0 + vec2(0.0, offset);
    painter.galley(pos, galley, caption);
    resp
}

/// The opaque part of the icon-button plate texture: its top-left 84 of 92 pixels (the rest is
/// transparent margin).
const PLATE_VISIBLE: f32 = 84.0 / 92.0;
/// The ring's height as a fraction of the visible plate, measured on the in-game button.
const RING_OF_PLATE: f32 = 0.78;
/// The ring's share of the glyph texture: 40 of its 56 pixels.
const RING_OF_GLYPH: f32 = 40.0 / 56.0;
/// The ring's centre in the mirrored glyph texture, as a fraction of its size (the ring sits off
/// centre, at pixel 24.5, 23.5 before mirroring).
const RING_CENTRE: Vec2 = Vec2::new((56.0 - 24.5) / 56.0, 23.5 / 56.0);
/// The game draws the mirrored glyph turned a little anticlockwise (degrees).
const RING_TURN_DEG: f32 = -28.0;
/// The glyph's resting colour: the hover art multiplied down to the cooler cream it has in game.
const GLYPH_REST_TINT: Color32 = Color32::from_rgb(191, 214, 253);

/// The game's reload button: the square icon button plate ([`Skin::icon_button`], normal, hover or
/// focus, pressed) with the round-arrow glyph ([`Skin::reload`]) on it, mirrored left to right and
/// turned as in game. At rest the glyph is tinted to the game's cream; hovered or pressed it shows
/// its own warmer colour. [`ROW_HEIGHT`] points on a side; `label` is the accessible name.
/// Disabled: dimmed resting art, never clicked, no hover state.
pub fn ff_reload_button(ui: &mut Ui, skin: &Skin, label: &str, enabled: bool) -> Response {
    let enabled = enabled && ui.is_enabled();
    let sense = if enabled {
        Sense::click()
    } else {
        Sense::hover()
    };
    let (rect, resp) = ui.allocate_exact_size(vec2(ROW_HEIGHT, ROW_HEIGHT), sense);
    resp.widget_info(|| WidgetInfo::labeled(WidgetType::Button, enabled, label));
    let state = if !enabled {
        0
    } else if resp.is_pointer_button_down_on() {
        2
    } else if resp.hovered() || resp.has_focus() {
        1
    } else {
        0
    };
    let tint = if enabled {
        Color32::WHITE
    } else {
        Color32::from_white_alpha(90)
    };
    slice::paint_image(ui.painter(), &skin.icon_button[state], rect, tint);

    let glyph_tint = match (enabled, state) {
        (false, _) => GLYPH_REST_TINT.gamma_multiply(90.0 / 255.0),
        (true, 0) => GLYPH_REST_TINT,
        (true, _) => Color32::WHITE,
    };
    let plate = ROW_HEIGHT * PLATE_VISIBLE;
    let side = plate * RING_OF_PLATE / RING_OF_GLYPH;
    let pressed = if state == 2 { 1.0 } else { 0.0 };
    // The game sits the ring 1pt below the plate's centre.
    let ring_centre = rect.min + Vec2::splat(plate / 2.0) + vec2(0.0, 1.0 + pressed);
    let dest = Rect::from_min_size(ring_centre - RING_CENTRE * side, Vec2::splat(side));
    let uv = slice::full_uv(skin.reload.size());
    // Swapping the u edges mirrors the glyph left to right.
    let mirrored = Rect::from_min_max(pos2(uv.max.x, uv.min.y), pos2(uv.min.x, uv.max.y));
    egui::Image::new(&skin.reload)
        .uv(mirrored)
        .rotate(RING_TURN_DEG.to_radians(), RING_CENTRE)
        .tint(glyph_tint)
        .paint_at(ui, dest);
    resp
}

/// Padding of [`ff_arrow`]'s click target around its 16pt art. Vertically 3, not 4, so in a
/// collapsed rule row (arrows at y 18..34) the target stays clear of the summary line's centre
/// (y 38), where a click expands the row.
pub const ARROW_HIT_PAD: Vec2 = Vec2::new(4.0, 3.0);

/// A small gold triangle button (the dropdown arrow art, flipped for "up"), for reordering.
/// Disabled arrows are dimmed and never report clicks. The art takes 16x16 of the layout, but an
/// enabled arrow senses clicks on a padded 24x22 target around it ([`ARROW_HIT_PAD`], WR-08), so
/// a slightly-off click does not fall through to the row behind.
pub fn ff_arrow(ui: &mut Ui, skin: &Skin, up: bool, enabled: bool) -> Response {
    let size = 16.0;
    let auto_id = ui.next_auto_id();
    let (rect, hover) = ui.allocate_exact_size(Vec2::splat(size), Sense::hover());
    let resp = if enabled {
        ui.interact(
            rect.expand2(ARROW_HIT_PAD),
            auto_id.with("hit"),
            Sense::click(),
        )
    } else {
        hover
    };
    let label = if up { "Move up" } else { "Move down" };
    resp.widget_info(|| WidgetInfo::labeled(WidgetType::Button, enabled, label));
    let tex = &skin.combo_arrow[usize::from(enabled && resp.hovered())];
    let full = slice::full_uv(tex.size());
    let uv = if up {
        Rect::from_min_max(pos2(full.min.x, full.max.y), pos2(full.max.x, full.min.y))
    } else {
        full
    };
    let tint = if enabled {
        Color32::WHITE
    } else {
        Color32::from_white_alpha(70)
    };
    ui.painter().image(
        tex.id(),
        slice::snap_rect(rect, ui.painter().pixels_per_point()),
        uv,
        tint,
    );
    resp
}

/// A 24x24 remove button: a thin painted X, muted until hovered. `label` is the accessible
/// name and the tooltip.
pub fn ff_x_button(ui: &mut Ui, label: &str) -> Response {
    let (rect, resp) = ui.allocate_exact_size(Vec2::splat(24.0), Sense::click());
    resp.widget_info(|| WidgetInfo::labeled(WidgetType::Button, true, label));
    let color = if resp.hovered() || resp.has_focus() {
        pal::HEADING
    } else {
        pal::MUTED
    };
    let c = rect.center();
    let h = 5.0;
    let stroke = Stroke::new(1.75, color);
    let painter = ui.painter();
    painter.line_segment([pos2(c.x - h, c.y - h), pos2(c.x + h, c.y + h)], stroke);
    painter.line_segment([pos2(c.x - h, c.y + h), pos2(c.x + h, c.y - h)], stroke);
    resp.on_hover_text(label)
}

/// A single-line search / text field in the recessed input box style.
pub fn ff_text_field(ui: &mut Ui, text: &mut String, hint: &str, width: f32) -> Response {
    ff_text_field_ex(ui, None, text, hint, width, regular(15.0))
}

/// [`ff_text_field`] with an explicit TextEdit id, so callers can focus it by id.
pub fn ff_text_field_id(
    ui: &mut Ui,
    id: egui::Id,
    text: &mut String,
    hint: &str,
    width: f32,
) -> Response {
    ff_text_field_ex(ui, Some(id), text, hint, width, regular(15.0))
}

/// The gap left after a text field's frame before the next widget on the same row.
pub const FIELD_GAP: f32 = 10.0;

/// The single text-field implementation: optional explicit `id`, and the `font` (e.g. monospace
/// 14 for raw filter rows).
pub fn ff_text_field_ex(
    ui: &mut Ui,
    id: Option<egui::Id>,
    text: &mut String,
    hint: &str,
    width: f32,
    font: egui::FontId,
) -> Response {
    let width = if width > 0.0 {
        width
    } else {
        ui.available_width()
    };
    let (rect, _) = ui.allocate_exact_size(vec2(width, 30.0), Sense::hover());
    let bg = ui.painter().add(Shape::Noop);
    let mut edit = egui::TextEdit::singleline(text)
        .frame(egui::Frame::NONE)
        .hint_text(egui::RichText::new(hint).color(pal::DISABLED))
        .font(font)
        .text_color(pal::VALUE)
        .vertical_align(Align::Center);
    if let Some(id) = id {
        edit = edit.id(id);
    }
    let resp = ui.put(rect.shrink2(vec2(10.0, 3.0)), edit);
    // `put` leaves the cursor at the inner edit's right edge, 10pt inside the frame, which made
    // the next widget on a row touch (or overlap) it. Put the cursor back past the whole frame and
    // keep FIELD_GAP before whatever follows, in a horizontal layout and when room is left.
    ui.advance_cursor_after_rect(rect);
    if ui.layout().main_dir().is_horizontal() {
        let extra = FIELD_GAP - ui.spacing().item_spacing.x;
        if extra > 0.0 && ui.available_width() > FIELD_GAP {
            ui.add_space(extra);
        }
    }
    let hot = resp.hovered() || resp.has_focus();
    let shapes = vec![
        Shape::rect_filled(rect, CornerRadius::same(1), pal::INPUT_FILL),
        Shape::rect_stroke(
            rect,
            CornerRadius::same(1),
            Stroke::new(
                1.0,
                if hot {
                    pal::INPUT_BORDER_HOT
                } else {
                    pal::INPUT_BORDER
                },
            ),
            StrokeKind::Inside,
        ),
    ];
    ui.painter().set(bg, Shape::Vec(shapes));
    resp
}

// ---------------------------------------------------------------------------------------------
// Status line

/// What a status message reports; decides its colour.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StatusKind {
    /// Neutral information.
    Info,
    /// Something succeeded.
    Ok,
    /// It worked, but something needs a look.
    Warning,
    /// Nothing happened because of an error.
    Error,
}

impl StatusKind {
    /// The palette colour for this kind.
    pub fn color(self) -> Color32 {
        match self {
            Self::Info => pal::MUTED,
            Self::Ok => pal::OK,
            Self::Warning => pal::WARN,
            Self::Error => pal::ERROR,
        }
    }
}

/// The layout of a status message: one row of Fontin Regular in the kind's colour, line
/// breaks flattened to spaces, cut with "…" past `width`.
pub fn status_job(text: &str, kind: StatusKind, width: f32) -> egui::text::LayoutJob {
    let mut job = egui::text::LayoutJob::single_section(
        text.replace(['\n', '\r'], " "),
        egui::TextFormat::simple(regular(15.0), kind.color()),
    );
    job.wrap = egui::text::TextWrapping {
        max_width: width.max(0.0),
        max_rows: 1,
        break_anywhere: true,
        overflow_character: Some('…'),
    };
    job
}

/// A one-line status message in `rect`: coloured by `kind`, cut with "…" when it does not
/// fit, and shown whole in a tooltip while hovered (only when it was cut).
pub fn ff_status(ui: &mut Ui, rect: Rect, text: &str, kind: StatusKind) -> Response {
    ff_status_ex(ui, rect, text, kind, false)
}

/// [`ff_status`] that can be clickable (pointing-hand cursor, `clicked()` reports the click),
/// for a status line that jumps to the thing it talks about.
pub fn ff_status_ex(
    ui: &mut Ui,
    rect: Rect,
    text: &str,
    kind: StatusKind,
    clickable: bool,
) -> Response {
    let sense = if clickable {
        Sense::click()
    } else {
        Sense::hover()
    };
    let mut resp = ui.interact(rect, ui.id().with("ff_status"), sense);
    if clickable {
        resp = resp.on_hover_cursor(egui::CursorIcon::PointingHand);
    }
    resp.widget_info(|| WidgetInfo::labeled(WidgetType::Label, true, text));
    let galley = ui
        .painter()
        .layout_job(status_job(text, kind, rect.width()));
    let elided = galley.elided;
    ui.painter().with_clip_rect(rect).galley(
        pos2(rect.min.x, rect.center().y - galley.size().y / 2.0),
        galley,
        kind.color(),
    );
    if elided {
        let full = text.to_owned();
        resp.on_hover_ui(|ui| {
            ui.set_max_width(420.0);
            ui.label(
                egui::RichText::new(full)
                    .font(regular(15.0))
                    .color(kind.color()),
            );
        })
    } else {
        resp
    }
}

// ---------------------------------------------------------------------------------------------
// Scrolling

/// Width reserved on the right of [`ff_scroll_area`] for the game-art scrollbar.
pub const SCROLLBAR_WIDTH: f32 = 14.0;

/// A vertical scroll area filling the rest of `ui`, with the game's scrollbar (thin track line,
/// gold capsule thumb) instead of egui's. The thumb drags, clicking the track pages, and the
/// mouse wheel scrolls as usual. Returns what `add_contents` returns.
pub fn ff_scroll_area<R>(
    ui: &mut Ui,
    skin: &Skin,
    id_salt: impl Hash + std::fmt::Debug,
    add_contents: impl FnOnce(&mut Ui) -> R,
) -> R {
    let outer = ui.available_rect_before_wrap();
    let content = Rect::from_min_max(
        outer.min,
        pos2(outer.max.x - SCROLLBAR_WIDTH - 4.0, outer.max.y),
    );
    let mut child = ui.new_child(
        UiBuilder::new()
            .max_rect(content)
            .layout(Layout::top_down(Align::Min)),
    );
    let out = egui::ScrollArea::vertical()
        .id_salt(id_salt)
        .auto_shrink([false, false])
        .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysHidden)
        .show(&mut child, add_contents);
    ui.allocate_rect(outer, Sense::hover());
    paint_game_scrollbar(
        ui,
        skin,
        outer,
        out.state,
        out.id,
        out.content_size.y,
        out.inner_rect.height(),
    );
    out.inner
}

/// [`ff_scroll_area`] for virtualised lists: `add_contents` also receives the visible viewport
/// (in content coordinates, so `viewport.min.y` is the scroll offset), and `offset` can set
/// the vertical scroll position (`None` keeps the stored one). The content is as wide as
/// `ff_scroll_area`'s, leaving room for the game scrollbar.
pub fn ff_scroll_area_viewport<R>(
    ui: &mut Ui,
    skin: &Skin,
    id_salt: impl Hash + std::fmt::Debug,
    offset: Option<f32>,
    add_contents: impl FnOnce(&mut Ui, Rect) -> R,
) -> R {
    let outer = ui.available_rect_before_wrap();
    let content = Rect::from_min_max(
        outer.min,
        pos2(outer.max.x - SCROLLBAR_WIDTH - 4.0, outer.max.y),
    );
    let mut child = ui.new_child(
        UiBuilder::new()
            .max_rect(content)
            .layout(Layout::top_down(Align::Min)),
    );
    let mut area = egui::ScrollArea::vertical()
        .id_salt(id_salt)
        .auto_shrink([false, false])
        .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysHidden);
    if let Some(o) = offset {
        area = area.vertical_scroll_offset(o);
    }
    let out = area.show_viewport(&mut child, add_contents);
    ui.allocate_rect(outer, Sense::hover());
    paint_game_scrollbar(
        ui,
        skin,
        outer,
        out.state,
        out.id,
        out.content_size.y,
        out.inner_rect.height(),
    );
    out.inner
}

/// Paints the game scrollbar (track line, gold thumb) in the right strip of `outer` and applies
/// thumb drags / track clicks to the stored scroll state.
fn paint_game_scrollbar(
    ui: &mut Ui,
    skin: &Skin,
    outer: Rect,
    state: egui::scroll_area::State,
    id: egui::Id,
    content_h: f32,
    view_h: f32,
) {
    let track = Rect::from_min_max(pos2(outer.max.x - SCROLLBAR_WIDTH, outer.min.y), outer.max);
    let painter = ui.painter();
    let line = Rect::from_center_size(track.center(), vec2(5.0, track.height()));
    slice::paint_image(painter, &skin.scroll_track, line, Color32::from_gray(160));
    if content_h <= view_h + 0.5 {
        return;
    }
    let max_offset = content_h - view_h;
    let thumb_h = (view_h / content_h * track.height()).clamp(28.0, track.height());
    let travel = (track.height() - thumb_h).max(1.0);
    let offset = state.offset.y.clamp(0.0, max_offset);
    let thumb_top = track.min.y + offset / max_offset * travel;
    let thumb = Rect::from_min_size(pos2(track.center().x - 6.0, thumb_top), vec2(12.0, thumb_h));

    let bar = ui.interact(track, id.with("ff_scrollbar"), Sense::click_and_drag());
    let mut new_offset = None;
    if bar.dragged() {
        new_offset = Some(offset + bar.drag_delta().y * max_offset / travel);
    } else if bar.clicked()
        && let Some(p) = bar.interact_pointer_pos()
        && !thumb.contains(p)
    {
        let page = if p.y < thumb.min.y { -view_h } else { view_h };
        new_offset = Some(offset + page);
    }
    if let Some(o) = new_offset {
        let mut state = state;
        state.offset.y = o.clamp(0.0, max_offset);
        state.store(ui.ctx(), id);
        ui.ctx().request_repaint();
    }
    let th = &skin.scroll_thumb;
    let tint = if bar.hovered() || bar.dragged() {
        Color32::WHITE
    } else {
        Color32::from_gray(225)
    };
    slice::paint_three(
        ui.painter(),
        [&th.left, &th.middle, &th.right],
        slice::three_v_rects(
            thumb,
            th.left.size(),
            th.right.size(),
            ui.painter().pixels_per_point(),
        ),
        tint,
    );
}

/// The skin's `egui::Visuals` and spacing: dark, gold selection, recessed inputs, and a thin
/// scrollbar for any plain egui scroll area (popups, text fields).
pub fn apply_style(ctx: &egui::Context) {
    ctx.global_style_mut(|style| {
        let v = &mut style.visuals;
        *v = egui::Visuals::dark();
        v.panel_fill = pal::GROUND;
        v.window_fill = pal::INPUT_FILL;
        v.extreme_bg_color = pal::INPUT_FILL;
        v.selection.bg_fill = Color32::from_rgba_unmultiplied(200, 162, 90, 70);
        v.selection.stroke = Stroke::new(1.0, pal::GOLD);
        v.text_cursor.stroke = Stroke::new(1.5, pal::HEADING);
        v.widgets.noninteractive.fg_stroke.color = pal::LABEL;
        v.widgets.inactive.fg_stroke.color = pal::LABEL;
        v.widgets.inactive.bg_fill = pal::INPUT_FILL;
        v.widgets.inactive.bg_stroke = Stroke::new(1.0, pal::INPUT_BORDER);
        v.widgets.hovered.fg_stroke.color = pal::HEADING;
        v.widgets.hovered.bg_stroke = Stroke::new(1.0, pal::INPUT_BORDER_HOT);
        v.widgets.active.fg_stroke.color = pal::HEADING;
        style.spacing.scroll = egui::style::ScrollStyle::thin();
        style.spacing.scroll.bar_width = 6.0;
        v.faint_bg_color = pal::INPUT_FILL;
        v.hyperlink_color = pal::GOLD;
        v.warn_fg_color = pal::WARN;
        v.error_fg_color = pal::ERROR;
        for w in [
            &mut v.widgets.noninteractive,
            &mut v.widgets.inactive,
            &mut v.widgets.hovered,
            &mut v.widgets.active,
            &mut v.widgets.open,
        ] {
            w.corner_radius = CornerRadius::same(1);
        }
        v.window_stroke = Stroke::new(1.0, pal::INPUT_BORDER_HOT);
        v.window_shadow = egui::Shadow::NONE;
        v.popup_shadow = egui::Shadow::NONE;
        style.text_styles = [
            (egui::TextStyle::Heading, small_caps(21.0)),
            (egui::TextStyle::Body, regular(15.0)),
            (egui::TextStyle::Button, small_caps(17.0)),
            (egui::TextStyle::Small, regular(15.0)),
            (egui::TextStyle::Monospace, egui::FontId::monospace(14.0)),
        ]
        .into();
        style.spacing.item_spacing = vec2(8.0, 4.0);
        style.spacing.button_padding = vec2(8.0, 4.0);
        style.spacing.interact_size.y = 28.0;
        style.spacing.tooltip_width = 420.0;
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use egui::{Event, Modifiers, PointerButton, Pos2, RawInput};

    /// IN-03: the selected row is revealed on the list's first frame (every pass of it), and
    /// again after the list was not drawn for a frame, even if the old state was never cleared.
    #[test]
    fn dropdown_reveal_restarts_after_a_frame_not_drawn() {
        let (s, first) = reveal_state(None, 10);
        assert!(first);
        let (s, first) = reveal_state(Some(s), 10);
        assert!(first, "a second pass of the opening frame still reveals");
        let (s, first) = reveal_state(Some(s), 11);
        assert!(!first);
        let (s, first) = reveal_state(Some(s), 12);
        assert!(!first);
        // Closed while not drawn (state left behind), reopened later.
        let (_, first) = reveal_state(Some(s), 40);
        assert!(first, "a stale state does not suppress the reveal");
    }

    fn click_at(pos: Pos2) -> RawInput {
        RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(600.0, 400.0))),
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

    fn setup() -> (egui::Context, Skin) {
        let ctx = egui::Context::default();
        super::super::fonts::install(&ctx);
        let skin = Skin::load(&ctx);
        (ctx, skin)
    }

    #[test]
    fn slider_maps_pointer_to_stepped_values() {
        let track = Rect::from_min_max(pos2(0.0, 0.0), pos2(112.0, 10.0));
        let range = 0.0..=100.0;
        assert_eq!(slider_value_at(6.0, track, 6.0, &range, 1.0), 0.0);
        assert_eq!(slider_value_at(106.0, track, 6.0, &range, 1.0), 100.0);
        assert_eq!(slider_value_at(56.0, track, 6.0, &range, 1.0), 50.0);
        assert_eq!(slider_value_at(-50.0, track, 6.0, &range, 1.0), 0.0);
        assert_eq!(slider_value_at(56.0, track, 6.0, &range, 25.0), 50.0);
        assert_eq!(snap_value(0.26, &(0.0..=1.0), 0.05), 0.25);
    }

    #[test]
    fn a_text_field_leaves_a_gap_before_the_next_widget_on_the_row() {
        let (ctx, skin) = setup();
        let (mut field, mut button) = (Rect::NOTHING, Rect::NOTHING);
        ctx.run_ui(RawInput::default(), |ui| {
            ui.horizontal(|ui| {
                let mut text = String::new();
                field = ff_text_field(ui, &mut text, "hint", 200.0).rect;
                button = ff_button(ui, &skin, "Add", 40.0).rect;
            });
        })
        .drop_without_applying_deltas();
        assert!(
            button.min.x - field.max.x >= FIELD_GAP - 0.5,
            "field {field:?} button {button:?}"
        );
    }

    #[test]
    fn check_flips_on_click() {
        let (ctx, skin) = setup();
        let mut on = false;
        let mut rect = Rect::NOTHING;
        ctx.run_ui(RawInput::default(), |ui| {
            rect = ff_check(ui, &skin, &mut on, "x").rect;
        })
        .drop_without_applying_deltas();
        let mut changed = false;
        ctx.run_ui(click_at(rect.center()), |ui| {
            changed = ff_check(ui, &skin, &mut on, "x").changed();
        })
        .drop_without_applying_deltas();
        assert!(on && changed);
    }

    #[test]
    fn slider_click_sets_value_and_idle_frames_change_nothing() {
        let (ctx, skin) = setup();
        let mut v = 10.0;
        let mut rect = Rect::NOTHING;
        ctx.run_ui(RawInput::default(), |ui| {
            let r = ff_slider(ui, &skin, &mut v, 0.0..=100.0, 1.0, "x", |v| format!("{v}"));
            assert!(!r.changed(), "an idle frame must not change the value");
            rect = r.rect;
        })
        .drop_without_applying_deltas();
        assert_eq!(v, 10.0);
        // Click a little left of the track's right end: the value goes near the top.
        let target = pos2(rect.max.x - 60.0, rect.center().y);
        ctx.run_ui(click_at(target), |ui| {
            ff_slider(ui, &skin, &mut v, 0.0..=100.0, 1.0, "x", |v| format!("{v}"));
        })
        .drop_without_applying_deltas();
        assert!(
            v > 85.0,
            "clicking near the right end must move the value there, got {v}"
        );
    }

    #[test]
    fn tabs_are_centred_evenly_spaced_and_captions_sit_on_the_flat_body() {
        // The game's caps: 26x37 left, 42x37 right; drawn 37 high, so 1 point per texel.
        let g = tab_layout(
            300.0,
            0.0,
            37.0,
            &[40.0, 60.0, 80.0, 40.0],
            [26, 37],
            [42, 37],
        );
        assert_eq!(g.len(), 4);
        let overlap = 42.0 * TAB_OVERLAP;
        for w in g.windows(2) {
            // Constant overlap: the next stone starts under the previous tail.
            assert!((w[0].rect.max.x - overlap - w[1].rect.min.x).abs() < 1e-3);
        }
        // The row is centred.
        let (l, r) = (g[0].rect.min.x, g[3].rect.max.x);
        assert!(((l + r) / 2.0 - 300.0).abs() < 1e-3);
        for t in &g {
            // Every caption sits left of the stone's centre (the tail is not label space) and
            // clear of the tail.
            assert!(t.label_x < t.rect.center().x);
            assert!(t.label_x > t.rect.min.x + 26.0);
        }
        // Equal captions get equal stones, so the spacing is even.
        assert!((g[0].rect.width() - g[3].rect.width()).abs() < 1e-3);
    }

    #[test]
    fn long_status_is_cut_to_one_row_and_short_status_is_whole() {
        let (ctx, _skin) = setup();
        ctx.run_ui(RawInput::default(), |ui| {
            let long = "word ".repeat(80);
            let g = ui
                .painter()
                .layout_job(status_job(&long, StatusKind::Warning, 200.0));
            assert!(g.elided, "a long message must be cut");
            assert_eq!(g.rows.len(), 1);
            assert!(g.size().x <= 200.5);
            let g = ui
                .painter()
                .layout_job(status_job("Wrote it.\nDone.", StatusKind::Ok, 400.0));
            assert!(!g.elided);
            assert_eq!(g.rows.len(), 1, "line breaks are flattened");
        })
        .drop_without_applying_deltas();
    }

    fn rect_of(ctx: &egui::Context, mut f: impl FnMut(&mut Ui) -> Rect) -> Rect {
        let mut r = Rect::NOTHING;
        ctx.run_ui(RawInput::default(), |ui| r = f(ui))
            .drop_without_applying_deltas();
        r
    }

    #[test]
    fn disabled_button_never_clicks_and_enabled_does() {
        let (ctx, skin) = setup();
        for enabled in [false, true] {
            let rect = rect_of(&ctx, |ui| {
                ff_button_ex(ui, &skin, "Go", 84.0, enabled, false).rect
            });
            let mut clicked = false;
            ctx.run_ui(click_at(rect.center()), |ui| {
                clicked = ff_button_ex(ui, &skin, "Go", 84.0, enabled, false).clicked();
            })
            .drop_without_applying_deltas();
            assert_eq!(clicked, enabled);
        }
    }

    #[test]
    fn tabs_with_tips_still_select_the_clicked_tab() {
        let (ctx, skin) = setup();
        let strip = Rect::from_min_size(pos2(0.0, 0.0), vec2(600.0, 36.0));
        let labels = ["Rules", "Config *", "Economy", "About"];
        let tips = [None, Some("Unapplied changes on this tab"), None, None];
        let mut sel = 0usize;
        ctx.run_ui(RawInput::default(), |ui| {
            ff_tabs_ex(ui, &skin, strip, &labels, &mut sel, &tips);
        })
        .drop_without_applying_deltas();
        let mut changed = false;
        ctx.run_ui(click_at(pos2(430.0, 18.0)), |ui| {
            changed = ff_tabs_ex(ui, &skin, strip, &labels, &mut sel, &tips).changed();
        })
        .drop_without_applying_deltas();
        assert!(changed && sel != 0, "selected {sel}");
    }

    #[test]
    fn clickable_status_clicks_and_plain_status_does_not() {
        let (ctx, _skin) = setup();
        let rect = Rect::from_min_size(pos2(10.0, 10.0), vec2(300.0, 24.0));
        for clickable in [true, false] {
            ctx.run_ui(RawInput::default(), |ui| {
                ff_status_ex(ui, rect, "hello", StatusKind::Info, clickable);
            })
            .drop_without_applying_deltas();
            let mut clicked = false;
            ctx.run_ui(click_at(rect.center()), |ui| {
                clicked = if clickable {
                    ff_status_ex(ui, rect, "hello", StatusKind::Info, true).clicked()
                } else {
                    ff_status(ui, rect, "hello", StatusKind::Info).clicked()
                };
            })
            .drop_without_applying_deltas();
            assert_eq!(clicked, clickable);
        }
    }

    #[test]
    fn header_plate_is_centred_on_the_gold_bar() {
        let rect = Rect::from_min_size(pos2(10.0, 20.0), vec2(600.0, 100.0));
        let p = header_plate_rect(rect);
        assert!((p.center().x - rect.center().x).abs() < 1e-3);
        assert!((p.center().y - (20.0 + 71.7)).abs() < 1e-3);
        assert!((p.width() - 240.0).abs() < 1e-3 && (p.height() - 30.0).abs() < 1e-3);
    }

    #[test]
    fn apply_style_sets_the_skin_text_sizes_and_spacing() {
        let (ctx, _skin) = setup();
        apply_style(&ctx);
        let st = ctx.global_style();
        let size = |t: &egui::TextStyle| st.text_styles[t].size;
        assert_eq!(size(&egui::TextStyle::Body), 15.0);
        assert_eq!(size(&egui::TextStyle::Button), 17.0);
        assert_eq!(size(&egui::TextStyle::Heading), 21.0);
        assert_eq!(size(&egui::TextStyle::Small), 15.0);
        assert_eq!(size(&egui::TextStyle::Monospace), 14.0);
        assert_eq!(
            st.text_styles[&egui::TextStyle::Button].family,
            egui::FontFamily::Name(super::super::fonts::SMALL_CAPS.into())
        );
        assert_eq!(st.spacing.item_spacing, vec2(8.0, 4.0));
        assert_eq!(st.spacing.interact_size.y, 28.0);
    }

    #[test]
    fn frame_layout_keeps_content_inside_the_gold_line() {
        let outer = Rect::from_min_size(pos2(0.0, 100.0), vec2(655.0, 900.0));
        let l = frame_layout(outer, [1310, 1582]);
        assert_eq!(l.scale, 0.5);
        assert_eq!(l.content.min, pos2(52.0, 175.0));
        assert_eq!(l.content.max, pos2(605.0, 900.0));
        assert!(l.button_bar.min.y > l.content.max.y);
        assert!(l.button_bar.max.y < outer.max.y);
        // The tab strip sits inside the frame's top border, above the content, never above the
        // frame (where the header art is).
        let t = tab_strip(&l);
        assert!(t.min.y > outer.min.y && t.max.y < l.content.min.y);
        assert_eq!((t.min.y, t.max.y), (124.0, 151.5));
        assert!(t.height() >= 26.0);
    }

    #[test]
    fn viewport_scroll_area_honours_offset_and_width() {
        let (ctx, skin) = setup();
        let mut seen = None;
        for _ in 0..2 {
            ctx.run_ui(RawInput::default(), |ui| {
                ui.set_max_size(vec2(300.0, 300.0));
                ff_scroll_area_viewport(ui, &skin, "t", Some(120.0), |ui, vp| {
                    seen = Some((vp, ui.available_width()));
                    ui.allocate_space(vec2(10.0, 2000.0));
                });
            })
            .drop_without_applying_deltas();
        }
        let (vp, w) = seen.unwrap();
        assert!((vp.min.y - 120.0).abs() < 0.5, "viewport {vp:?}");
        assert!(w <= 300.0 - SCROLLBAR_WIDTH - 4.0 + 0.5, "width {w}");
    }

    #[test]
    fn viewport_scroll_area_keeps_offset_without_override() {
        let (ctx, skin) = setup();
        let mut offsets = vec![];
        for i in 0..3 {
            ctx.run_ui(RawInput::default(), |ui| {
                ui.set_max_size(vec2(300.0, 300.0));
                let off = if i == 0 { Some(80.0) } else { None };
                ff_scroll_area_viewport(ui, &skin, "keep", off, |ui, vp| {
                    offsets.push(vp.min.y);
                    ui.allocate_space(vec2(10.0, 2000.0));
                });
            })
            .drop_without_applying_deltas();
        }
        assert!((offsets[2] - 80.0).abs() < 0.5, "offsets {offsets:?}");
    }

    #[test]
    fn field_row_grows_with_tall_controls() {
        let (ctx, _skin) = setup();
        let tall = rect_of(&ctx, |ui| {
            ff_field_row(ui, 0, "Name", None, |ui| {
                ui.allocate_space(vec2(40.0, 80.0));
            })
            .response
            .rect
        });
        assert!(tall.height() >= 80.0, "{tall:?}");
        let small = rect_of(&ctx, |ui| {
            ff_field_row(ui, 0, "Name", None, |ui| {
                ui.allocate_space(vec2(40.0, 20.0));
            })
            .response
            .rect
        });
        assert_eq!(small.height(), ROW_HEIGHT);
    }

    #[test]
    fn field_row_control_starts_after_label_column() {
        let (ctx, _skin) = setup();
        let mut start = 0.0;
        let mut left = 0.0;
        ctx.run_ui(RawInput::default(), |ui| {
            let r = ff_field_row(ui, 0, "Name", Some("tip"), |ui| {
                ui.allocate_exact_size(vec2(10.0, 10.0), Sense::hover()).0
            });
            start = r.inner.min.x;
            left = r.response.rect.min.x;
        })
        .drop_without_applying_deltas();
        assert!((start - (left + ROW_PAD + LABEL_COL_W + LABEL_GAP)).abs() < 0.5);
    }

    #[test]
    fn long_field_labels_are_cut_with_an_ellipsis() {
        let (ctx, _skin) = setup();
        let mut long = None;
        let mut short = None;
        ctx.run_ui(RawInput::default(), |ui| {
            long = Some(
                ui.painter()
                    .layout_job(field_label_job("SetBackgroundColor")),
            );
            short = Some(ui.painter().layout_job(field_label_job("Rarity")));
        })
        .drop_without_applying_deltas();
        let (long, short) = (long.unwrap(), short.unwrap());
        assert!(long.elided, "a long label is cut");
        let last = long
            .rows
            .last()
            .and_then(|r| r.glyphs.last())
            .map(|g| g.chr);
        assert_eq!(last, Some('…'));
        assert!(long.size().x <= LABEL_COL_W + LABEL_GAP - 4.0 + 0.5);
        assert!(!short.elided);
        assert_eq!(short.text(), "Rarity");
    }

    #[test]
    fn chip_toggles_and_is_28_high() {
        let (ctx, _skin) = setup();
        let mut sel = false;
        let rect = rect_of(&ctx, |ui| {
            super::super::editor::ff_chip(ui, "Full AR", &mut sel).rect
        });
        assert_eq!(rect.height(), 28.0);
        ctx.run_ui(click_at(rect.center()), |ui| {
            let r = super::super::editor::ff_chip(ui, "Full AR", &mut sel);
            assert!(r.changed());
        })
        .drop_without_applying_deltas();
        assert!(sel);
    }

    #[test]
    fn reload_button_is_row_height_square() {
        let (ctx, skin) = setup();
        let size = rect_of(&ctx, |ui| {
            ff_reload_button(ui, &skin, "Refresh", true).rect
        })
        .size();
        assert_eq!(size, Vec2::splat(ROW_HEIGHT));
    }

    #[test]
    fn reload_button_clicks_only_when_enabled() {
        let (ctx, skin) = setup();
        let rect = rect_of(&ctx, |ui| {
            ff_reload_button(ui, &skin, "Refresh", true).rect
        });
        let mut on = false;
        ctx.run_ui(click_at(rect.center()), |ui| {
            on = ff_reload_button(ui, &skin, "Refresh", true).clicked();
        })
        .drop_without_applying_deltas();
        assert!(on, "an enabled reload button reports the click");
        let (ctx, skin) = setup();
        let mut off = true;
        ctx.run_ui(click_at(rect.center()), |ui| {
            off = ff_reload_button(ui, &skin, "Refresh", false).clicked();
        })
        .drop_without_applying_deltas();
        assert!(!off, "a disabled reload button never reports a click");
    }

    #[test]
    fn reload_button_is_labelled_and_reports_enabled() {
        let (ctx, skin) = setup();
        ctx.enable_accesskit();
        let out = ctx.run_ui(RawInput::default(), |ui| {
            ff_reload_button(ui, &skin, "Refresh", false);
        });
        let update = out
            .platform_output
            .accesskit_update
            .as_ref()
            .expect("update");
        assert!(update.nodes.iter().any(|(_, n)| {
            (n.label() == Some("Refresh") || n.value() == Some("Refresh")) && n.is_disabled()
        }));
        out.drop_without_applying_deltas();
    }

    #[test]
    fn x_button_is_24_square_and_labelled() {
        let (ctx, _skin) = setup();
        ctx.enable_accesskit();
        let mut size = Vec2::ZERO;
        let out = ctx.run_ui(RawInput::default(), |ui| {
            size = ff_x_button(ui, "Remove condition").rect.size();
        });
        assert_eq!(size, Vec2::splat(24.0));
        let update = out
            .platform_output
            .accesskit_update
            .as_ref()
            .expect("update");
        assert!(update.nodes.iter().any(|(_, n)| {
            n.label() == Some("Remove condition") || n.value() == Some("Remove condition")
        }));
        out.drop_without_applying_deltas();
    }

    #[test]
    fn text_field_id_is_focusable_by_id() {
        let (ctx, _skin) = setup();
        let id = egui::Id::new("my-field");
        let mut text = String::new();
        let mut got = None;
        ctx.run_ui(RawInput::default(), |ui| {
            got = Some(ff_text_field_id(ui, id, &mut text, "hint", 200.0).id);
        })
        .drop_without_applying_deltas();
        assert_eq!(got, Some(id));
        ctx.memory_mut(|m| m.request_focus(id));
        ctx.run_ui(RawInput::default(), |ui| {
            ff_text_field_id(ui, id, &mut text, "hint", 200.0);
        })
        .drop_without_applying_deltas();
        assert!(ctx.memory(|m| m.has_focus(id)));
    }

    #[test]
    fn dropdown_ex_keeps_selection_semantics() {
        let (ctx, skin) = setup();
        let mut sel = 1usize;
        ctx.run_ui(RawInput::default(), |ui| {
            let r = ff_dropdown_ex(
                ui,
                &skin,
                "dd",
                &mut sel,
                &["a", "b"],
                120.0,
                Some(pal::WARN),
            );
            assert!(!r.changed());
            assert_eq!(r.rect.height(), 28.0);
        })
        .drop_without_applying_deltas();
        assert_eq!(sel, 1);
    }

    /// Opens a dropdown of `n` options in a 400x400 screen; returns the popup's area rect.
    fn open_dropdown_area(n: usize) -> Rect {
        let (ctx, skin) = setup();
        let labels: Vec<String> = (0..n).map(|i| format!("opt {i}")).collect();
        let opts: Vec<&str> = labels.iter().map(String::as_str).collect();
        let screen = Rect::from_min_size(Pos2::ZERO, vec2(400.0, 400.0));
        let mut sel = 0usize;
        let mut rect = Rect::NOTHING;
        let frame = |events: Vec<egui::Event>| RawInput {
            screen_rect: Some(screen),
            events,
            ..Default::default()
        };
        ctx.run_ui(frame(vec![]), |ui| {
            rect = ff_dropdown(ui, &skin, "dd", &mut sel, &opts, 120.0).rect;
        })
        .drop_without_applying_deltas();
        let c = rect.center();
        let click = vec![
            egui::Event::PointerMoved(c),
            egui::Event::PointerButton {
                pos: c,
                button: egui::PointerButton::Primary,
                pressed: true,
                modifiers: egui::Modifiers::default(),
            },
            egui::Event::PointerButton {
                pos: c,
                button: egui::PointerButton::Primary,
                pressed: false,
                modifiers: egui::Modifiers::default(),
            },
        ];
        for events in [click, vec![], vec![]] {
            ctx.run_ui(frame(events), |ui| {
                ff_dropdown(ui, &skin, "dd", &mut sel, &opts, 120.0);
            })
            .drop_without_applying_deltas();
        }
        ctx.memory(|m| {
            m.areas()
                .visible_layer_ids()
                .into_iter()
                .filter(|l| l.order == egui::Order::Foreground)
                .filter_map(|l| m.area_rect(l.id))
                .next()
        })
        .expect("the dropdown popup is open")
    }

    #[test]
    fn dropdown_long_list_scrolls_inside_the_screen() {
        let short = open_dropdown_area(4);
        assert!(short.height() < 4.0 * 26.0 + 20.0, "{short:?}");
        let long = open_dropdown_area(40);
        assert!(long.height() <= 12.0 * 26.0 + 20.0, "{long:?}");
        assert!(long.max.y <= 400.0 + 1.0, "{long:?}");
    }
}
