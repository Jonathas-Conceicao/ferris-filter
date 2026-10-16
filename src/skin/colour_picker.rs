//! The shared, game-styled colour picker for style slots and rule colour actions. It replaces
//! egui's built-in picker: a saturation/brightness square, a hue bar, Hex and R/G/B fields, an
//! alpha slider only where the target supports alpha, the nine Dracula quick picks and a None
//! tile only where the slot can be empty.

use egui::{
    Align, Color32, Event, EventFilter, Key, Layout, Pos2, Rect, Response, Sense, Shape, Stroke,
    StrokeKind, Ui, WidgetInfo, WidgetType, pos2, vec2,
};

use super::Skin;
use super::editor::ff_swatch;
use super::fonts::regular;
use super::minimap::DRACULA;
use super::palette as pal;
use super::popup::paint_box_behind_pad;
use super::widgets::{ff_button, ff_slider, ff_text_field_id};

/// Width of the picker's content; the popup is this plus 2 x 16 of padding.
const CONTENT_W: f32 = 268.0;
/// How far the box art sits out from the picker content (inside the popup's 16pt margin).
const FRAME_PAD: f32 = 14.0;
const SQUARE_H: f32 = 150.0;
const BAR_H: f32 = 16.0;
const TILE: f32 = 24.0;
const TILE_GAP: f32 = 4.0;

/// What a picker slot supports.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PickerMode {
    /// Show the alpha slider (rule colour actions; styles are opaque).
    pub alpha: bool,
    /// Show the None tile (a style slot can be empty; a rule colour line is removed instead).
    pub allow_none: bool,
}

/// What happened in the picker body this frame.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct PickerResult {
    /// The user changed the colour (the working `Hsva` holds the new value).
    pub changed: bool,
    /// The user picked the None tile.
    pub none: bool,
    /// The user pressed Use colour.
    pub done: bool,
    /// The user clicked a quick pick, including the one already selected, or committed a Hex /
    /// R / G / B value with Enter, including the current one (an explicit choice of that colour,
    /// which matters for an empty slot whose seed is still showing).
    pub picked: bool,
}

/// An edit the popup asks its caller to apply.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PickerEdit {
    /// Set the colour (unmultiplied RGBA; alpha 255 when the mode has no alpha).
    Set([u8; 4]),
    /// Clear the slot.
    Clear,
}

#[cfg(test)]
thread_local! {
    static PROBE: std::cell::RefCell<Vec<(String, Rect)>> = const { std::cell::RefCell::new(Vec::new()) };
}

/// Test hook: records where a named piece was drawn.
#[cfg(test)]
fn probe(tag: &str, rect: Rect) {
    PROBE.with(|p| p.borrow_mut().push((tag.to_owned(), rect)));
}

/// Test hook for other modules: forgets the recorded probes.
#[cfg(test)]
pub(crate) fn probes_clear() {
    PROBE.with(|p| p.borrow_mut().clear());
}

/// Test hook for other modules: where the named piece was last drawn.
#[cfg(test)]
pub(crate) fn probe_rect(tag: &str) -> Option<Rect> {
    PROBE.with(|p| {
        p.borrow()
            .iter()
            .rev()
            .find(|(t, _)| t == tag)
            .map(|(_, r)| *r)
    })
}

#[cfg(not(test))]
fn probe(_tag: &str, _rect: Rect) {}

/// Unmultiplied HSVA of a filter colour. Going through `Hsva::from_rgba_unmultiplied` (not a
/// premultiplied `Color32`) keeps the RGB of a fully transparent colour such as
/// `SetBackgroundColor 40 42 54 0` (WR-04).
pub fn hsva_of(rgba: [u8; 4]) -> egui::ecolor::Hsva {
    use egui::ecolor::linear_f32_from_gamma_u8 as lin;
    egui::ecolor::Hsva::from_rgba_unmultiplied(
        lin(rgba[0]),
        lin(rgba[1]),
        lin(rgba[2]),
        f32::from(rgba[3]) / 255.0,
    )
}

/// Parses `#RRGGBB`, `RRGGBB`, `#RGB` or `RGB` (surrounding spaces ignored). Anything else, such
/// as non-hex digits or a length other than 3 or 6, is `None`.
pub fn parse_hex(text: &str) -> Option<[u8; 3]> {
    let t = text.trim();
    let t = t.strip_prefix('#').unwrap_or(t);
    if !t.chars().all(|c| c.is_ascii_hexdigit()) {
        return None;
    }
    let nibble = |c: char| c.to_digit(16).map(|d| d as u8);
    let digits: Vec<u8> = t.chars().map(nibble).collect::<Option<_>>()?;
    match digits.len() {
        3 => Some([digits[0] * 17, digits[1] * 17, digits[2] * 17]),
        6 => Some([
            digits[0] * 16 + digits[1],
            digits[2] * 16 + digits[3],
            digits[4] * 16 + digits[5],
        ]),
        _ => None,
    }
}

/// `#RRGGBB`, upper case.
pub fn hex_of(rgb: [u8; 3]) -> String {
    format!("#{:02X}{:02X}{:02X}", rgb[0], rgb[1], rgb[2])
}

/// A decimal channel value 0-255 (surrounding spaces ignored); anything else is `None`.
pub fn parse_channel(text: &str) -> Option<u8> {
    let t = text.trim();
    if t.is_empty() || !t.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }
    t.parse::<u8>().ok()
}

/// Saturation (x) and brightness (y, top is bright) at `pos` inside the square `rect`, clamped.
pub fn sv_at(pos: Pos2, rect: Rect) -> (f32, f32) {
    let s = ((pos.x - rect.min.x) / rect.width().max(1.0)).clamp(0.0, 1.0);
    let v = 1.0 - ((pos.y - rect.min.y) / rect.height().max(1.0)).clamp(0.0, 1.0);
    (s, v)
}

/// Hue (0..=1) at horizontal position `x` along the bar `rect`, clamped.
pub fn hue_at(x: f32, rect: Rect) -> f32 {
    ((x - rect.min.x) / rect.width().max(1.0)).clamp(0.0, 1.0)
}

// ---------------------------------------------------------------------------------------------
// Gamma-space HSV helpers
//
// The picker works in the HSV a colour designer expects (computed on the gamma-encoded RGB the
// filter stores), not in the linear HSV of `Hsva`, whose brightness axis looks lopsided. The
// working `Hsva` stays the interface (it keeps RGB at zero alpha, WR-04); the gamma triple of the
// last edit is remembered next to it so the hue survives greys and black.

fn rgb_f32(rgb: [u8; 3]) -> [f32; 3] {
    [
        f32::from(rgb[0]) / 255.0,
        f32::from(rgb[1]) / 255.0,
        f32::from(rgb[2]) / 255.0,
    ]
}

fn hsv_of_rgb(rgb: [u8; 3]) -> [f32; 3] {
    let (h, s, v) = egui::ecolor::hsv_from_rgb(rgb_f32(rgb));
    [h, s, v]
}

fn rgb_of_hsv(hsv: [f32; 3]) -> [u8; 3] {
    let c = egui::ecolor::rgb_from_hsv((hsv[0], hsv[1], hsv[2]));
    let q = |x: f32| (x * 255.0).round().clamp(0.0, 255.0) as u8;
    [q(c[0]), q(c[1]), q(c[2])]
}

/// The gamma HSV to show for `rgb`: the remembered one while it still stands for `rgb`.
fn gamma_state(ctx: &egui::Context, id: egui::Id, rgb: [u8; 3]) -> [f32; 3] {
    match ctx.data(|d| d.get_temp::<([f32; 3], [u8; 3])>(id.with("hsv"))) {
        Some((hsv, at)) if at == rgb => hsv,
        _ => hsv_of_rgb(rgb),
    }
}

// ---------------------------------------------------------------------------------------------
// Painting

fn paint_square(painter: &egui::Painter, rect: Rect, hue: f32) {
    let (cols, rows) = (24_usize, 12_usize);
    let mut mesh = egui::Mesh::default();
    for r in 0..=rows {
        for c in 0..=cols {
            let s = c as f32 / cols as f32;
            let v = 1.0 - r as f32 / rows as f32;
            let [cr, cg, cb] = rgb_of_hsv([hue, s, v]);
            mesh.colored_vertex(
                pos2(
                    rect.min.x + s * rect.width(),
                    rect.min.y + (1.0 - v) * rect.height(),
                ),
                Color32::from_rgb(cr, cg, cb),
            );
        }
    }
    let stride = (cols + 1) as u32;
    for r in 0..rows as u32 {
        for c in 0..cols as u32 {
            let i = r * stride + c;
            mesh.add_triangle(i, i + 1, i + stride);
            mesh.add_triangle(i + 1, i + stride + 1, i + stride);
        }
    }
    painter.add(Shape::mesh(mesh));
    painter.rect_stroke(
        rect,
        0.0,
        Stroke::new(1.0, pal::INPUT_BORDER),
        StrokeKind::Inside,
    );
}

fn paint_hue_bar(painter: &egui::Painter, rect: Rect) {
    let mut mesh = egui::Mesh::default();
    for k in 0..=6_u32 {
        let t = k as f32 / 6.0;
        let [r, g, b] = rgb_of_hsv([t, 1.0, 1.0]);
        let col = Color32::from_rgb(r, g, b);
        let x = rect.min.x + t * rect.width();
        mesh.colored_vertex(pos2(x, rect.min.y), col);
        mesh.colored_vertex(pos2(x, rect.max.y), col);
    }
    for k in 0..6_u32 {
        let i = k * 2;
        mesh.add_triangle(i, i + 1, i + 2);
        mesh.add_triangle(i + 1, i + 3, i + 2);
    }
    painter.add(Shape::mesh(mesh));
    painter.rect_stroke(
        rect,
        0.0,
        Stroke::new(1.0, pal::INPUT_BORDER),
        StrokeKind::Inside,
    );
}

/// A two-tone grey checkerboard with `rgba` blended over it, so alpha reads at a glance.
fn paint_checker_preview(painter: &egui::Painter, rect: Rect, rgba: [u8; 4]) {
    let cell = 6.0;
    let (light, dark) = (Color32::from_gray(200), Color32::from_gray(120));
    let painter = painter.with_clip_rect(rect);
    let (cols, rows) = (
        (rect.width() / cell).ceil() as i32,
        (rect.height() / cell).ceil() as i32,
    );
    for r in 0..rows {
        for c in 0..cols {
            let col = if (r + c) % 2 == 0 { light } else { dark };
            let min = pos2(rect.min.x + c as f32 * cell, rect.min.y + r as f32 * cell);
            painter.rect_filled(Rect::from_min_size(min, vec2(cell, cell)), 0.0, col);
        }
    }
    painter.rect_filled(
        rect,
        0.0,
        Color32::from_rgba_unmultiplied(rgba[0], rgba[1], rgba[2], rgba[3]),
    );
    painter.rect_stroke(
        rect,
        0.0,
        Stroke::new(1.0, pal::INPUT_BORDER),
        StrokeKind::Inside,
    );
}

// ---------------------------------------------------------------------------------------------
// The picker body

/// A text field whose text is kept in memory while it has focus (so typing is not rewritten from
/// the colour every frame) and re-derived from the colour otherwise. Returns the typed text on the
/// frame the field commits (Enter or focus loss).
fn value_field(
    ui: &mut Ui,
    id: egui::Id,
    derived: String,
    width: f32,
    name: &str,
    tag: &str,
) -> Option<String> {
    let text_id = id.with("txt");
    let focused = ui.memory(|m| m.has_focus(id));
    let mut text = if focused {
        ui.data(|d| d.get_temp::<String>(text_id))
            .unwrap_or(derived)
    } else {
        derived
    };
    let resp = ff_text_field_id(ui, id, &mut text, "", width);
    probe(tag, resp.rect);
    resp.widget_info(|| WidgetInfo::labeled(WidgetType::TextEdit, true, name));
    let resp = resp.on_hover_text(name);
    if resp.has_focus() {
        ui.data_mut(|d| d.insert_temp(text_id, text.clone()));
    } else {
        ui.data_mut(|d| d.remove::<String>(text_id));
    }
    resp.lost_focus().then_some(text)
}

/// Draws the picker body (square, hue bar, fields, alpha, quick picks, None, Use colour) at
/// content width 268 and edits `hsva` in place. `id` keys the picker's memory.
pub fn ff_colour_picker(
    ui: &mut Ui,
    skin: &Skin,
    id: egui::Id,
    hsva: &mut egui::ecolor::Hsva,
    mode: PickerMode,
) -> PickerResult {
    let ctx = ui.ctx().clone();
    let mut res = PickerResult::default();
    let rgba0 = hsva.to_srgba_unmultiplied();
    let mut rgb = [rgba0[0], rgba0[1], rgba0[2]];
    let mut alpha = rgba0[3];
    let mut hsv = gamma_state(&ctx, id, rgb);
    let mut changed = false;
    let shift = ui.input(|i| i.modifiers.shift);

    // Saturation / brightness square.
    let (square, sresp) =
        ui.allocate_exact_size(vec2(CONTENT_W, SQUARE_H), Sense::click_and_drag());
    probe("square", square);
    paint_square(ui.painter(), square, hsv[0]);
    if sresp.clicked() || sresp.drag_started() {
        sresp.request_focus();
    }
    if (sresp.dragged() || sresp.clicked() || sresp.drag_started())
        && let Some(p) = sresp.interact_pointer_pos()
    {
        let (s, v) = sv_at(p, square);
        if (s, v) != (hsv[1], hsv[2]) {
            hsv[1] = s;
            hsv[2] = v;
            rgb = rgb_of_hsv(hsv);
            changed = true;
        }
    }
    if sresp.has_focus() {
        ui.memory_mut(|m| {
            m.set_focus_lock_filter(
                sresp.id,
                EventFilter {
                    horizontal_arrows: true,
                    vertical_arrows: true,
                    tab: false,
                    escape: false,
                },
            )
        });
        let step = if shift { 0.10 } else { 0.01 };
        let (l, r, u, d) = ui.input(|i| {
            (
                i.key_pressed(Key::ArrowLeft),
                i.key_pressed(Key::ArrowRight),
                i.key_pressed(Key::ArrowUp),
                i.key_pressed(Key::ArrowDown),
            )
        });
        let (ds, dv) = (
            f32::from(i8::from(r) - i8::from(l)) * step,
            f32::from(i8::from(u) - i8::from(d)) * step,
        );
        if ds != 0.0 || dv != 0.0 {
            hsv[1] = (hsv[1] + ds).clamp(0.0, 1.0);
            hsv[2] = (hsv[2] + dv).clamp(0.0, 1.0);
            rgb = rgb_of_hsv(hsv);
            changed = true;
        }
    }
    sresp
        .widget_info(|| WidgetInfo::labeled(WidgetType::Slider, true, "Saturation and brightness"));
    let cursor = pos2(
        square.min.x + hsv[1] * square.width(),
        square.min.y + (1.0 - hsv[2]) * square.height(),
    );
    ui.painter()
        .circle_stroke(cursor, 5.0, Stroke::new(3.5, Color32::BLACK));
    ui.painter()
        .circle_stroke(cursor, 5.0, Stroke::new(1.5, Color32::WHITE));

    // Hue bar.
    ui.add_space(8.0);
    let (bar, hresp) = ui.allocate_exact_size(vec2(CONTENT_W, BAR_H), Sense::click_and_drag());
    probe("hue", bar);
    paint_hue_bar(ui.painter(), bar);
    if hresp.clicked() || hresp.drag_started() {
        hresp.request_focus();
    }
    if (hresp.dragged() || hresp.clicked() || hresp.drag_started())
        && let Some(p) = hresp.interact_pointer_pos()
    {
        let h = hue_at(p.x, bar);
        if h != hsv[0] {
            hsv[0] = h;
            rgb = rgb_of_hsv(hsv);
            changed = true;
        }
    }
    if hresp.has_focus() {
        ui.memory_mut(|m| {
            m.set_focus_lock_filter(
                hresp.id,
                EventFilter {
                    horizontal_arrows: true,
                    vertical_arrows: false,
                    tab: false,
                    escape: false,
                },
            )
        });
        let step = if shift { 10.0 / 360.0 } else { 1.0 / 360.0 };
        let (l, r) = ui.input(|i| {
            (
                i.key_pressed(Key::ArrowLeft),
                i.key_pressed(Key::ArrowRight),
            )
        });
        let dh = f32::from(i8::from(r) - i8::from(l)) * step;
        if dh != 0.0 {
            hsv[0] = (hsv[0] + dh).clamp(0.0, 1.0);
            rgb = rgb_of_hsv(hsv);
            changed = true;
        }
    }
    hresp.widget_info(|| WidgetInfo::labeled(WidgetType::Slider, true, "Hue"));
    let mx = bar.min.x + hsv[0] * bar.width();
    let marker = Rect::from_center_size(pos2(mx, bar.center().y), vec2(4.0, 22.0));
    ui.painter().rect_filled(marker, 0.0, Color32::WHITE);
    ui.painter().rect_stroke(
        marker,
        0.0,
        Stroke::new(1.0, Color32::BLACK),
        StrokeKind::Outside,
    );

    // Hex and R / G / B (one line: 88 + 3 x (label + 38) + gaps = 268 of 268).
    ui.add_space(12.0);
    let mut typed: Option<[u8; 3]> = None;
    // A value committed with Enter is an explicit choice even when it equals the working colour
    // (an empty slot's seed), like a click on the selected quick pick.
    let mut entered = false;
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 3.0;
        if let Some(t) = value_field(ui, id.with("hex"), hex_of(rgb), 88.0, "Hex", "hex")
            && let Some(v) = parse_hex(&t)
        {
            entered |= ui.input(|i| i.key_pressed(egui::Key::Enter));
            if v != rgb {
                typed = Some(v);
            }
        }
        for (k, (label, name)) in [("R", "Red"), ("G", "Green"), ("B", "Blue")]
            .into_iter()
            .enumerate()
        {
            ui.add_space(5.0);
            ui.label(
                egui::RichText::new(label)
                    .font(regular(14.0))
                    .color(pal::MUTED),
            );
            if let Some(t) = value_field(
                ui,
                id.with(("ch", k)),
                rgb[k].to_string(),
                38.0,
                name,
                &name.to_lowercase(),
            ) && let Some(v) = parse_channel(&t)
            {
                entered |= ui.input(|i| i.key_pressed(egui::Key::Enter));
                if v != rgb[k] {
                    let mut next = typed.unwrap_or(rgb);
                    next[k] = v;
                    typed = Some(next);
                }
            }
        }
    });
    if let Some(v) = typed {
        rgb = v;
        hsv = hsv_of_rgb(rgb);
        changed = true;
    }

    // Alpha.
    if mode.alpha {
        ui.add_space(8.0);
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 8.0;
            let (rect, _) = ui.allocate_exact_size(vec2(TILE, TILE), Sense::hover());
            paint_checker_preview(ui.painter(), rect, [rgb[0], rgb[1], rgb[2], alpha]);
            let mut a = f64::from(alpha);
            let r = ff_slider(ui, skin, &mut a, 0.0..=255.0, 1.0, "Alpha", |v| {
                format!("{v:.0}")
            });
            probe("alpha", r.rect);
            if r.changed() {
                alpha = a.round().clamp(0.0, 255.0) as u8;
                changed = true;
            }
        });
    }

    // Quick picks and, where the slot can be empty, the None tile: one row. The tiles shrink
    // just enough (24 -> 23pt) for the tenth tile to fit the content width.
    ui.add_space(12.0);
    let tiles = DRACULA.len() + usize::from(mode.allow_none);
    let tile = TILE.min(((CONTENT_W - (tiles - 1) as f32 * TILE_GAP) / tiles as f32).floor());
    let mut none_tile = |ui: &mut Ui| {
        let r = ff_swatch(ui, None, false, vec2(tile, tile), "No colour")
            .on_hover_text("No colour (game default)");
        probe("none", r.rect);
        if r.clicked() {
            res.none = true;
        }
    };
    let mut picked = false;
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = TILE_GAP;
        for (name, c) in DRACULA {
            let selected = rgb == [c.r(), c.g(), c.b()];
            let r = ff_swatch(ui, Some(c), selected, vec2(tile, tile), name).on_hover_text(name);
            probe(&format!("quick:{name}"), r.rect);
            if r.clicked() {
                picked = true;
                if !selected {
                    rgb = [c.r(), c.g(), c.b()];
                    hsv = hsv_of_rgb(rgb);
                    changed = true;
                }
            }
        }
        if mode.allow_none {
            none_tile(ui);
        }
    });
    res.picked = picked || entered;

    // Footer.
    ui.add_space(12.0);
    ui.with_layout(Layout::right_to_left(Align::Min), |ui| {
        let r = ff_button(ui, skin, "Use colour", 112.0);
        probe("use", r.rect);
        if r.clicked() {
            res.done = true;
        }
    });

    if changed {
        *hsva = hsva_of([rgb[0], rgb[1], rgb[2], alpha]);
        ctx.data_mut(|d| d.insert_temp(id.with("hsv"), (hsv, rgb)));
        res.changed = true;
    }
    res
}

// ---------------------------------------------------------------------------------------------
// The popup host

#[derive(Clone)]
struct OpenState {
    /// The value the slot had when the picker opened (what Escape restores).
    opening: Option<[u8; 4]>,
    /// The working colour.
    hsva: egui::ecolor::Hsva,
}

/// Debug-only: asks the `colour_popup` whose `tag` matches to open itself on its next frame
/// (screenshot support). Compiled out of release builds.
#[cfg(debug_assertions)]
pub fn request_open(ctx: &egui::Context, tag: &str) {
    ctx.data_mut(|d| d.insert_temp(egui::Id::new("ff-colour-picker-shot"), tag.to_owned()));
}

/// The picker as a popup anchored below `trigger` (left-aligned, flipped above near the bottom of
/// the screen), framed in the game's box art. It edits live: each change returns an edit at once;
/// Escape returns the value the picker opened with; a click outside, the trigger again or Use
/// colour keeps the current value. `current` is the slot's value (`None` = empty), `seed` what an
/// empty slot starts the picker from; on an empty slot Use colour or any quick pick (even the
/// selected one) sets the working colour. Opening, or closing without a change, returns `None`.
pub fn colour_popup(
    ui: &mut Ui,
    trigger: &Response,
    tag: &str,
    current: Option<[u8; 4]>,
    seed: [u8; 4],
    mode: PickerMode,
) -> Option<PickerEdit> {
    let ctx = ui.ctx().clone();
    let popup_id = egui::Popup::default_response_id(trigger);
    let state_id = popup_id.with("picker-state");
    let body_id = popup_id.with("body");
    #[cfg(debug_assertions)]
    {
        let shot = egui::Id::new("ff-colour-picker-shot");
        if ctx.data(|d| d.get_temp::<String>(shot)).as_deref() == Some(tag) {
            ctx.data_mut(|d| d.remove::<String>(shot));
            egui::Popup::close_all(&ctx);
            egui::Popup::open_id(&ctx, popup_id);
        }
    }
    #[cfg(not(debug_assertions))]
    let _ = tag;
    // One picker at a time: opening this one closes any other.
    if trigger.clicked() && !egui::Popup::is_id_open(&ctx, popup_id) {
        egui::Popup::close_all(&ctx);
    }
    let skin = crate::skin::of(&ctx);
    let mut out = None;
    egui::Popup::from_toggle_button_response(trigger)
        .id(popup_id)
        .width(CONTENT_W + 32.0)
        .gap(2.0)
        .align(egui::RectAlign::BOTTOM_START)
        .align_alternatives(&[egui::RectAlign::TOP_START])
        .frame(egui::Frame::NONE.inner_margin(16))
        .close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside)
        .show(|ui| {
            ui.set_width(CONTENT_W);
            let bg = ui.painter().add(Shape::Noop);
            ui.spacing_mut().item_spacing.y = 0.0;
            let mut st = ctx
                .data(|d| d.get_temp::<OpenState>(state_id))
                .unwrap_or_else(|| OpenState {
                    opening: current,
                    hsva: hsva_of(current.unwrap_or(seed)),
                });
            let res = ff_colour_picker(ui, &skin, body_id, &mut st.hsva, mode);
            ctx.data_mut(|d| d.insert_temp(state_id, st.clone()));
            // The box fills the popup (its 16pt margin less a 2pt rim), like the modals frame
            // their content, rather than hugging the controls.
            paint_box_behind_pad(ui, &skin, bg, FRAME_PAD);
            let escape = ui.input(|i| {
                i.events.iter().any(|e| {
                    matches!(
                        e,
                        Event::Key {
                            key: Key::Escape,
                            pressed: true,
                            ..
                        }
                    )
                })
            });
            if escape {
                if st.opening != current {
                    out = Some(match st.opening {
                        Some(c) => PickerEdit::Set(c),
                        None => PickerEdit::Clear,
                    });
                }
            } else if res.none {
                if current.is_some() {
                    out = Some(PickerEdit::Clear);
                }
            } else if res.changed || ((res.done || res.picked) && current.is_none()) {
                // An explicit choice (Use colour, a quick pick) on an empty slot commits the
                // working colour even when it is still the seed.
                let mut now = st.hsva.to_srgba_unmultiplied();
                if !mode.alpha {
                    now[3] = 255;
                }
                if Some(now) != current {
                    out = Some(PickerEdit::Set(now));
                }
            }
            if escape || res.none || res.done {
                egui::Popup::close_id(&ctx, popup_id);
            }
        });
    if !egui::Popup::is_id_open(&ctx, popup_id) {
        ctx.data_mut(|d| {
            d.remove::<OpenState>(state_id);
            d.remove::<([f32; 3], [u8; 3])>(body_id.with("hsv"));
        });
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use egui::{pos2, vec2};

    #[test]
    fn hex_parses_and_formats() {
        assert_eq!(parse_hex("#FF79C6"), Some([255, 121, 198]));
        assert_eq!(parse_hex("ff79c6"), Some([255, 121, 198]));
        assert_eq!(parse_hex("#F0A"), Some([255, 0, 170]));
        assert_eq!(parse_hex(" #ff79c6 "), Some([255, 121, 198]));
        assert_eq!(parse_hex("#GG0000"), None);
        assert_eq!(parse_hex("#12345"), None);
        assert_eq!(parse_hex(""), None);
        assert_eq!(parse_hex("#"), None);
        assert_eq!(hex_of([255, 121, 198]), "#FF79C6");
        for c in [[0, 0, 0], [255, 255, 255], [40, 42, 54], [1, 2, 3]] {
            assert_eq!(parse_hex(&hex_of(c)), Some(c));
        }
    }

    #[test]
    fn channels_are_strict() {
        assert_eq!(parse_channel("0"), Some(0));
        assert_eq!(parse_channel("255"), Some(255));
        assert_eq!(parse_channel(" 12 "), Some(12));
        assert_eq!(parse_channel("256"), None);
        assert_eq!(parse_channel("-1"), None);
        assert_eq!(parse_channel("+1"), None);
        assert_eq!(parse_channel("x"), None);
        assert_eq!(parse_channel(""), None);
    }

    #[test]
    fn hsva_keeps_rgb_at_zero_alpha() {
        assert_eq!(
            hsva_of([40, 42, 54, 0]).to_srgba_unmultiplied(),
            [40, 42, 54, 0]
        );
        assert_eq!(
            hsva_of([0, 0, 0, 255]).to_srgba_unmultiplied(),
            [0, 0, 0, 255]
        );
        assert_eq!(
            hsva_of([255, 121, 198, 128]).to_srgba_unmultiplied(),
            [255, 121, 198, 128]
        );
    }

    #[test]
    fn square_and_bar_map_and_clamp() {
        let r = Rect::from_min_size(pos2(10.0, 20.0), vec2(268.0, 150.0));
        assert_eq!(sv_at(r.min, r), (0.0, 1.0));
        assert_eq!(sv_at(r.max, r), (1.0, 0.0));
        assert_eq!(sv_at(pos2(-50.0, 999.0), r), (0.0, 0.0));
        assert_eq!(sv_at(pos2(999.0, -50.0), r), (1.0, 1.0));
        let bar = Rect::from_min_size(pos2(10.0, 0.0), vec2(268.0, 16.0));
        assert_eq!(hue_at(10.0, bar), 0.0);
        assert_eq!(hue_at(278.0, bar), 1.0);
        assert_eq!(hue_at(-5.0, bar), 0.0);
        assert_eq!(hue_at(900.0, bar), 1.0);
    }

    // ---- headless popup tests ----

    use egui::{Context, Event, Modifiers, PointerButton, RawInput};

    const OPEN: PickerMode = PickerMode {
        alpha: true,
        allow_none: true,
    };

    fn input(events: Vec<Event>) -> RawInput {
        RawInput {
            screen_rect: Some(Rect::from_min_size(pos2(0.0, 0.0), vec2(660.0, 900.0))),
            events,
            ..Default::default()
        }
    }

    fn click_at(pos: egui::Pos2) -> Vec<Event> {
        let button = |pressed| Event::PointerButton {
            pos,
            button: PointerButton::Primary,
            pressed,
            modifiers: Modifiers::default(),
        };
        vec![Event::PointerMoved(pos), button(true), button(false)]
    }

    fn escape() -> Vec<Event> {
        vec![Event::Key {
            key: Key::Escape,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: Modifiers::default(),
        }]
    }

    /// One frame of a trigger chip plus its popup; returns the edit and the probes of the frame.
    fn frame(
        ctx: &Context,
        events: Vec<Event>,
        current: Option<[u8; 4]>,
        mode: PickerMode,
    ) -> (Option<PickerEdit>, Vec<(String, Rect)>) {
        PROBE.with(|p| p.borrow_mut().clear());
        let mut out = None;
        ctx.run_ui(input(events), |ui| {
            let (rect, _) = ui.allocate_exact_size(vec2(34.0, 22.0), Sense::hover());
            let trigger = ui.interact(rect, egui::Id::new("trigger"), Sense::click());
            out = colour_popup(ui, &trigger, "t", current, [248, 248, 242, 255], mode);
        })
        .drop_without_applying_deltas();
        (out, PROBE.with(|p| p.borrow().clone()))
    }

    fn setup() -> Context {
        let ctx = Context::default();
        crate::skin::fonts::install(&ctx);
        ctx
    }

    fn rect_of(probes: &[(String, Rect)], tag: &str) -> Option<Rect> {
        probes.iter().rev().find(|(t, _)| t == tag).map(|(_, r)| *r)
    }

    /// Opens the picker and lays it out (the first open frame is a sizing pass).
    fn open(ctx: &Context, current: Option<[u8; 4]>, mode: PickerMode) -> Vec<(String, Rect)> {
        frame(ctx, vec![], current, mode);
        request_open(ctx, "t");
        let (out, _) = frame(ctx, vec![], current, mode);
        assert_eq!(out, None, "opening emits nothing");
        let (out, probes) = frame(ctx, vec![], current, mode);
        assert_eq!(out, None, "an open, untouched picker emits nothing");
        probes
    }

    #[test]
    fn closed_picker_is_idle_for_any_alpha() {
        let ctx = setup();
        for a in [255, 128, 0] {
            for _ in 0..2 {
                let (out, probes) = frame(&ctx, vec![], Some([40, 42, 54, a]), OPEN);
                assert_eq!(out, None);
                assert!(rect_of(&probes, "square").is_none(), "popup is closed");
            }
        }
    }

    #[test]
    fn opening_and_idling_emits_nothing_for_every_alpha() {
        for a in [255, 128, 0] {
            let ctx = setup();
            let probes = open(&ctx, Some([40, 42, 54, a]), OPEN);
            assert!(rect_of(&probes, "square").is_some(), "popup is open");
            for _ in 0..2 {
                let (out, _) = frame(&ctx, vec![], Some([40, 42, 54, a]), OPEN);
                assert_eq!(out, None);
            }
        }
    }

    #[test]
    fn a_quick_pick_keeps_the_current_alpha() {
        let ctx = setup();
        let cur = Some([1, 2, 3, 128]);
        let probes = open(&ctx, cur, OPEN);
        let pink = rect_of(&probes, "quick:Pink").unwrap();
        let (out, _) = frame(&ctx, click_at(pink.center()), cur, OPEN);
        assert_eq!(out, Some(PickerEdit::Set([255, 121, 198, 128])));
    }

    #[test]
    fn a_quick_pick_equal_to_the_current_colour_emits_nothing() {
        let ctx = setup();
        let cur = Some([255, 121, 198, 255]);
        let probes = open(&ctx, cur, OPEN);
        let pink = rect_of(&probes, "quick:Pink").unwrap();
        let (out, _) = frame(&ctx, click_at(pink.center()), cur, OPEN);
        assert_eq!(out, None);
    }

    #[test]
    fn the_none_tile_clears_and_closes() {
        let ctx = setup();
        let cur = Some([1, 2, 3, 255]);
        let probes = open(&ctx, cur, OPEN);
        let none = rect_of(&probes, "none").unwrap();
        let (out, _) = frame(&ctx, click_at(none.center()), cur, OPEN);
        assert_eq!(out, Some(PickerEdit::Clear));
        let (_, probes) = frame(&ctx, vec![], None, OPEN);
        assert!(rect_of(&probes, "square").is_none(), "closed after None");
    }

    #[test]
    fn escape_restores_the_opening_value() {
        let ctx = setup();
        let probes = open(&ctx, Some([1, 2, 3, 255]), OPEN);
        let pink = rect_of(&probes, "quick:Pink").unwrap();
        let (out, _) = frame(&ctx, click_at(pink.center()), Some([1, 2, 3, 255]), OPEN);
        assert_eq!(out, Some(PickerEdit::Set([255, 121, 198, 255])));
        let (out, _) = frame(&ctx, escape(), Some([255, 121, 198, 255]), OPEN);
        assert_eq!(out, Some(PickerEdit::Set([1, 2, 3, 255])));
    }

    #[test]
    fn escape_on_an_unset_slot_clears_it() {
        let ctx = setup();
        let probes = open(&ctx, None, OPEN);
        let pink = rect_of(&probes, "quick:Pink").unwrap();
        let (out, _) = frame(&ctx, click_at(pink.center()), None, OPEN);
        assert_eq!(out, Some(PickerEdit::Set([255, 121, 198, 255])));
        let (out, _) = frame(&ctx, escape(), Some([255, 121, 198, 255]), OPEN);
        assert_eq!(out, Some(PickerEdit::Clear));
    }

    #[test]
    fn escape_without_a_change_emits_nothing() {
        let ctx = setup();
        let cur = Some([1, 2, 3, 255]);
        open(&ctx, cur, OPEN);
        let (out, _) = frame(&ctx, escape(), cur, OPEN);
        assert_eq!(out, None);
    }

    #[test]
    fn alpha_and_none_follow_the_mode() {
        let ctx = setup();
        let mode = PickerMode {
            alpha: false,
            allow_none: false,
        };
        let probes = open(&ctx, Some([1, 2, 3, 255]), mode);
        assert!(rect_of(&probes, "alpha").is_none());
        assert!(rect_of(&probes, "none").is_none());
        assert!(rect_of(&probes, "square").is_some());
        let ctx = setup();
        let probes = open(&ctx, Some([1, 2, 3, 255]), OPEN);
        assert!(rect_of(&probes, "alpha").is_some());
        assert!(rect_of(&probes, "none").is_some());
    }

    #[test]
    fn the_square_sets_saturation_and_brightness() {
        let ctx = setup();
        let cur = Some([255, 0, 0, 255]);
        let probes = open(&ctx, cur, OPEN);
        let sq = rect_of(&probes, "square").unwrap();
        // Bottom-left corner area is black.
        let (out, _) = frame(
            &ctx,
            click_at(sq.left_bottom() + vec2(1.0, -1.0)),
            cur,
            OPEN,
        );
        let Some(PickerEdit::Set(c)) = out else {
            panic!("expected a Set, got {out:?}");
        };
        assert!(c[0] < 8 && c[1] < 8 && c[2] < 8, "{c:?}");
        assert_eq!(c[3], 255);
    }

    #[test]
    fn use_colour_closes_without_an_edit() {
        let ctx = setup();
        let cur = Some([1, 2, 3, 255]);
        let probes = open(&ctx, cur, OPEN);
        let useb = rect_of(&probes, "use").unwrap();
        let (out, _) = frame(&ctx, click_at(useb.center()), cur, OPEN);
        assert_eq!(out, None);
        let (_, probes) = frame(&ctx, vec![], cur, OPEN);
        assert!(rect_of(&probes, "square").is_none());
    }

    #[test]
    fn use_colour_on_an_unset_slot_sets_the_seed() {
        let ctx = setup();
        let probes = open(&ctx, None, OPEN);
        let useb = rect_of(&probes, "use").unwrap();
        let (out, _) = frame(&ctx, click_at(useb.center()), None, OPEN);
        assert_eq!(out, Some(PickerEdit::Set([248, 248, 242, 255])));
        let (_, probes) = frame(&ctx, vec![], Some([248, 248, 242, 255]), OPEN);
        assert!(
            rect_of(&probes, "square").is_none(),
            "closed after Use colour"
        );
    }

    /// Review iteration 2, WR-01: typing the seed's own hex on an empty slot and pressing Enter
    /// sets it.
    #[test]
    fn entering_the_seed_hex_on_an_unset_slot_sets_it() {
        let ctx = setup();
        let probes = open(&ctx, None, OPEN);
        let hex = rect_of(&probes, "hex").unwrap();
        let _ = frame(&ctx, click_at(hex.center()), None, OPEN);
        let (out, _) = frame(
            &ctx,
            vec![egui::Event::Key {
                key: egui::Key::Enter,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: egui::Modifiers::NONE,
            }],
            None,
            OPEN,
        );
        assert_eq!(out, Some(PickerEdit::Set([248, 248, 242, 255])));
    }

    #[test]
    fn the_selected_quick_pick_on_an_unset_slot_sets_it() {
        let ctx = setup();
        let probes = open(&ctx, None, OPEN);
        // The seed is Foreground, so that tile is drawn selected.
        let fg = rect_of(&probes, "quick:Foreground").unwrap();
        let (out, _) = frame(&ctx, click_at(fg.center()), None, OPEN);
        assert_eq!(out, Some(PickerEdit::Set([248, 248, 242, 255])));
        // Once set, clicking it again is a no-op.
        let (out, _) = frame(
            &ctx,
            click_at(fg.center()),
            Some([248, 248, 242, 255]),
            OPEN,
        );
        assert_eq!(out, None);
    }
}
