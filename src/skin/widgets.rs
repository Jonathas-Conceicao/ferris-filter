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

// ---------------------------------------------------------------------------------------------
// Header, close button, tabs

/// The header's height for a window `width` points wide: the header art's aspect ratio.
pub fn header_height(skin: &Skin, width: f32) -> f32 {
    let [w, h] = skin.title_bar.size();
    width * h as f32 / w.max(1) as f32
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
    let plate_y = rect.min.y + rect.height() * 0.717;
    match art {
        Some(tex) => slice::paint_image(&painter, tex, rect, Color32::WHITE),
        None => {
            painter.rect_filled(rect, 0.0, pal::INPUT_FILL);
            let plate = Rect::from_center_size(
                pos2(rect.center().x, plate_y),
                vec2(rect.width() * 0.4, rect.height() * 0.3),
            );
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
        let resp = ui.interact(hit, ui.id().with(("ff_tab", i)), Sense::click());
        let is_sel = *selected == i;
        let label = labels[i];
        resp.widget_info(|| WidgetInfo::selected(WidgetType::SelectableLabel, true, is_sel, label));
        if resp.clicked() && !is_sel {
            *selected = i;
            changed = true;
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
                .layout_no_wrap(format(*v), regular(16.0), pal::VALUE)
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
        regular(16.0),
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
    let width = if width > 0.0 {
        width
    } else {
        ui.available_width()
    };
    let id = ui.make_persistent_id(id_salt);
    let (rect, mut resp) = ui.allocate_exact_size(vec2(width, 28.0), Sense::click());
    let current = options.get(*selected).copied().unwrap_or("");
    resp.widget_info(|| WidgetInfo::labeled(WidgetType::ComboBox, true, current));
    let open = egui::Popup::is_id_open(ui.ctx(), id);
    let hot = resp.hovered() || open;
    let painter = ui.painter();
    input_box(painter, rect, hot);
    painter.with_clip_rect(rect.shrink(2.0)).text(
        pos2(rect.min.x + 10.0, rect.center().y),
        Align2::LEFT_CENTER,
        current,
        regular(16.0),
        pal::VALUE,
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

    let mut picked = None;
    egui::Popup::from_toggle_button_response(&resp)
        .id(id)
        .width(width)
        .gap(2.0)
        .frame(egui::Frame::NONE.inner_margin(6))
        .show(|ui| {
            let bg = ui.painter().add(Shape::Noop);
            ui.spacing_mut().item_spacing.y = 0.0;
            for (i, opt) in options.iter().enumerate() {
                let (r, o) = ui.allocate_exact_size(vec2(width - 12.0, 26.0), Sense::click());
                o.widget_info(|| {
                    WidgetInfo::selected(WidgetType::SelectableLabel, true, i == *selected, *opt)
                });
                if o.hovered() {
                    ui.painter().rect_filled(r, 0.0, pal::ROW_HOVER);
                }
                let color = if i == *selected {
                    pal::HEADING
                } else if o.hovered() {
                    pal::VALUE
                } else {
                    pal::LABEL
                };
                ui.painter().text(
                    pos2(r.min.x + 8.0, r.center().y),
                    Align2::LEFT_CENTER,
                    *opt,
                    regular(16.0),
                    color,
                );
                if o.clicked() {
                    picked = Some(i);
                }
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
    if let Some(i) = picked
        && i != *selected
    {
        *selected = i;
        resp.mark_changed();
    }
    resp
}

/// The game's generic button (3-slice art with hover / pressed states) with a Fontin SmallCaps
/// caption. At least `min_width` wide.
pub fn ff_button(ui: &mut Ui, skin: &Skin, text: &str, min_width: f32) -> Response {
    let height = 34.0;
    let galley = ui
        .painter()
        .layout_no_wrap(text.to_owned(), small_caps(17.0), pal::BUTTON_TEXT);
    let width = (galley.size().x + 44.0).max(min_width);
    let (rect, resp) = ui.allocate_exact_size(vec2(width, height), Sense::click());
    resp.widget_info(|| WidgetInfo::labeled(WidgetType::Button, true, text));
    let state = if resp.is_pointer_button_down_on() {
        2
    } else if resp.hovered() || resp.has_focus() {
        1
    } else {
        0
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
        Color32::WHITE,
    );
    let offset = if state == 2 { 1.0 } else { 0.0 };
    let pos = rect.center() - galley.size() / 2.0 + vec2(0.0, offset);
    painter.galley(pos, galley, pal::BUTTON_TEXT);
    resp
}

/// A small gold triangle button (the dropdown arrow art, flipped for "up"), for reordering.
/// Disabled arrows are dimmed and never report clicks.
pub fn ff_arrow(ui: &mut Ui, skin: &Skin, up: bool, enabled: bool) -> Response {
    let size = 16.0;
    let sense = if enabled {
        Sense::click()
    } else {
        Sense::hover()
    };
    let (rect, resp) = ui.allocate_exact_size(Vec2::splat(size), sense);
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

/// A single-line search / text field in the recessed input box style.
pub fn ff_text_field(ui: &mut Ui, text: &mut String, hint: &str, width: f32) -> Response {
    let width = if width > 0.0 {
        width
    } else {
        ui.available_width()
    };
    let (rect, _) = ui.allocate_exact_size(vec2(width, 30.0), Sense::hover());
    let bg = ui.painter().add(Shape::Noop);
    let edit = egui::TextEdit::singleline(text)
        .frame(egui::Frame::NONE)
        .hint_text(egui::RichText::new(hint).color(pal::DISABLED))
        .font(regular(16.0))
        .text_color(pal::VALUE)
        .vertical_align(Align::Center);
    let resp = ui.put(rect.shrink2(vec2(10.0, 3.0)), edit);
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
    let resp = ui.interact(rect, ui.id().with("ff_status"), Sense::hover());
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

    let view_h = out.inner_rect.height();
    let content_h = out.content_size.y;
    let track = Rect::from_min_max(pos2(outer.max.x - SCROLLBAR_WIDTH, outer.min.y), outer.max);
    let painter = ui.painter();
    let line = Rect::from_center_size(track.center(), vec2(5.0, track.height()));
    slice::paint_image(painter, &skin.scroll_track, line, Color32::from_gray(160));
    if content_h <= view_h + 0.5 {
        return out.inner;
    }
    let max_offset = content_h - view_h;
    let thumb_h = (view_h / content_h * track.height()).clamp(28.0, track.height());
    let travel = (track.height() - thumb_h).max(1.0);
    let offset = out.state.offset.y.clamp(0.0, max_offset);
    let thumb_top = track.min.y + offset / max_offset * travel;
    let thumb = Rect::from_min_size(pos2(track.center().x - 6.0, thumb_top), vec2(12.0, thumb_h));

    let bar = ui.interact(track, out.id.with("ff_scrollbar"), Sense::click_and_drag());
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
        let mut state = out.state;
        state.offset.y = o.clamp(0.0, max_offset);
        state.store(ui.ctx(), out.id);
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
    out.inner
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
        style.spacing.item_spacing = vec2(8.0, 2.0);
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use egui::{Event, Modifiers, PointerButton, Pos2, RawInput};

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

    #[test]
    fn frame_layout_keeps_content_inside_the_gold_line() {
        let outer = Rect::from_min_size(pos2(0.0, 100.0), vec2(655.0, 900.0));
        let l = frame_layout(outer, [1310, 1582]);
        assert_eq!(l.scale, 0.5);
        assert_eq!(l.content.min, pos2(52.0, 175.0));
        assert_eq!(l.content.max, pos2(605.0, 900.0));
        assert!(l.button_bar.min.y > l.content.max.y);
        assert!(l.button_bar.max.y < outer.max.y);
    }
}
