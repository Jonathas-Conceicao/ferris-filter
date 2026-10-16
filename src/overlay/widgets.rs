//! Custom-painted chrome for the PoE2-idiom stripe: the header plate, zebra-striped rows, the
//! circular enable/disable toggle, and the up/down reorder arrows.
//!
//! `egui::Visuals` alone cannot express a header plate (flat bronze fill, title, and a
//! hand-painted close control) or a two-tone alternating row background — `Visuals::striped`
//! is a simple on/off, not the designed two-color pair D-24 calls for (RESEARCH § "Code
//! Examples"). Both need direct `egui::Painter` calls, which is what this module is for. It is
//! the only place in the crate that paints the header's `✕` — as two crossed line segments, not
//! a Unicode glyph, since neither egui's default font nor the all-caps Cinzel face contains
//! U+2715 (the tofu box 13-03 found and deferred here).
//!
//! [`circular_toggle`] and [`arrow_button`] (plan 14-02) follow that same lesson: PoE2's own
//! toggle is a circle, not egui's square `Checkbox`, and Cinzel almost certainly has no ▲/▼
//! glyphs either (`PITFALLS.md`-adjacent RESEARCH Pitfall 4) — both are painted with
//! `egui::Painter` primitives over the plain `ui.interact(..., Sense::click())` hit-testing
//! `header_plate`'s `✕` already established, rather than a custom `egui::Widget` impl or a text
//! glyph that might render as tofu.

use super::text_entry::parse_base_type_entry;
use super::theme;
use crate::skin::colour_picker::{PickerEdit, PickerMode, colour_popup};
use crate::skin::editor::{ff_colour_card_ex, ff_swatch};
use crate::skin::fonts::regular;
use crate::skin::palette as pal;

/// What [`header_plate`] reports back to the caller.
pub struct HeaderResponse {
    /// `true` exactly on the frame the close control (`✕`) was clicked.
    pub close_clicked: bool,
}

/// Paints a full-width header plate filled with [`theme::HEADER_PLATE`]: `title` on the left in
/// the Cinzel family, and a hand-painted `✕` on the right. D-14: the caller is responsible for
/// what a click does (hide the stripe) — this function only reports the click.
pub fn header_plate(ui: &mut egui::Ui, title: &str) -> HeaderResponse {
    let height = 40.0;
    let width = ui.available_width();
    let (rect, _response) = ui.allocate_exact_size(egui::vec2(width, height), egui::Sense::hover());

    let painter = ui.painter();
    painter.rect_filled(rect, 0.0, theme::HEADER_PLATE);

    painter.text(
        egui::pos2(rect.left() + 14.0, rect.center().y),
        egui::Align2::LEFT_CENTER,
        title,
        egui::FontId::new(17.0, egui::FontFamily::Name(theme::DISPLAY_FONT.into())),
        theme::SECTION_CREAM,
    );

    // The close control's hit region: a square on the plate's right edge.
    let close_size = 22.0;
    let close_rect = egui::Rect::from_center_size(
        egui::pos2(rect.right() - 20.0, rect.center().y),
        egui::vec2(close_size, close_size),
    );
    let close_response = ui.interact(
        close_rect,
        ui.id().with("header-close"),
        egui::Sense::click(),
    );

    let stroke_color = if close_response.hovered() {
        theme::SECTION_CREAM
    } else {
        theme::LABEL_TAN
    };
    let stroke = egui::Stroke::new(2.0, stroke_color);
    let pad = 6.0;
    painter.line_segment(
        [
            close_rect.left_top() + egui::vec2(pad, pad),
            close_rect.right_bottom() - egui::vec2(pad, pad),
        ],
        stroke,
    );
    painter.line_segment(
        [
            close_rect.right_top() + egui::vec2(-pad, pad),
            close_rect.left_bottom() + egui::vec2(pad, -pad),
        ],
        stroke,
    );

    HeaderResponse {
        close_clicked: close_response.clicked(),
    }
}

/// Fills a row-height rect with the base or alternate zebra tone by `index` parity (D-23), then
/// renders `add`'s content inside it.
pub fn zebra_row(ui: &mut egui::Ui, index: usize, add: impl FnOnce(&mut egui::Ui)) {
    let color = if index % 2 == 0 {
        theme::ROW_BASE
    } else {
        theme::ROW_ALT
    };

    egui::Frame::new()
        .fill(color)
        .inner_margin(egui::Margin::symmetric(8, 4))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            add(ui);
        });
}

/// A PoE2-idiom circular toggle (D-24: "toggles are circular, not square").
///
/// Built on the exact same `allocate_exact_size` -> `ui.interact` -> `ui.painter()` sequence as
/// [`header_plate`]'s `✕`, swapping the paint step for a stroked circle (always) plus a filled
/// inner dot (only when `checked`) instead of crossed line segments. No custom `egui::Widget`
/// impl and no bespoke hit-testing — `ui.interact`'s `Sense::click()` already gives correct
/// click/hover/focus semantics, exactly as it does for the header's close control.
///
/// Mutates `*checked` in place and calls [`egui::Response::mark_changed`] on a click, the same
/// "widget function over a `&mut` reference" shape `egui::Checkbox`'s own `Widget` impl uses
/// internally — callers branch on `.changed()` exactly as they would for `ui.checkbox(...)`.
pub fn circular_toggle(ui: &mut egui::Ui, checked: &mut bool) -> egui::Response {
    let skin = crate::skin::of(ui.ctx());
    crate::skin::widgets::ff_check(ui, &skin, checked, "Enabled")
}

/// A modern on/off slide switch: a fixed 32x16 pill whose knob sits right when on, left when off.
///
/// Same allocate -> interact -> paint shape as [`circular_toggle`]: flips `*on` and calls
/// [`egui::Response::mark_changed`] on a click, so callers branch on `.changed()`.
pub fn slide_switch(ui: &mut egui::Ui, on: &mut bool) -> egui::Response {
    let size = egui::vec2(32.0, 16.0);
    let auto_id = ui.next_auto_id();
    let (rect, _) = ui.allocate_exact_size(size, egui::Sense::hover());
    let mut response = ui.interact(rect.expand(3.0), auto_id.with("hit"), egui::Sense::click());
    let state = *on;
    response.widget_info(|| {
        egui::WidgetInfo::selected(egui::WidgetType::Checkbox, true, state, "Switch")
    });

    if response.clicked() {
        *on = !*on;
        response.mark_changed();
    }

    let t = ui.ctx().animate_bool_responsive(auto_id.with("anim"), *on);
    // The "on" track is gold (UI-SPEC: track INPUT_FILL, on GOLD), not `WARM_ACCENT`, which is
    // now the near-black button fill and would make on and off look the same (review WR-06).
    let track = if *on {
        crate::skin::palette::GOLD
    } else {
        theme::INPUT_FILL
    };
    let edge = if response.hovered() {
        theme::SECTION_CREAM
    } else {
        theme::CONTROL_GLYPH
    };
    let painter = ui.painter();
    painter.rect(
        rect,
        size.y / 2.0,
        track,
        egui::Stroke::new(1.0, edge),
        egui::StrokeKind::Inside,
    );
    let r = size.y / 2.0 - 3.0;
    let x = egui::lerp((rect.left() + r + 3.0)..=(rect.right() - r - 3.0), t);
    painter.circle_filled(egui::pos2(x, rect.center().y), r, edge);

    response
}

/// Which way [`arrow_button`] points.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArrowDirection {
    Up,
    Down,
}

/// A hand-painted reorder arrow (D-05), used for the rule stripe's up/down move controls.
///
/// Painted as a filled triangle via [`egui::Shape::convex_polygon`], **not** a text glyph —
/// Cinzel is an all-caps display face and egui's rasterizer has no OpenType feature support, so
/// `▲`/`▼` are as likely to render as tofu boxes as U+2715 did for `header_plate`'s `✕` before
/// that control was hand-painted (RESEARCH Pitfall 4).
///
/// When `enabled` is `false` the control is allocated with `Sense::hover()` instead of
/// `Sense::click()`, so its `Response` structurally cannot report `clicked()` — callers pass
/// `enabled = false` for the first row's up arrow / the last row's down arrow rather than hiding
/// the control, keeping the row's control layout stable across rows.
pub fn arrow_button(ui: &mut egui::Ui, direction: ArrowDirection, enabled: bool) -> egui::Response {
    let skin = crate::skin::of(ui.ctx());
    crate::skin::widgets::ff_arrow(ui, &skin, direction == ArrowDirection::Up, enabled)
}

/// A 16px painted X inside a 24x24 click target, with a tooltip and accessible name `label`
/// (for example "Remove this condition"). Painted, not a glyph: Cinzel has no coverage.
pub fn remove_x_button(ui: &mut egui::Ui, label: &str) -> egui::Response {
    crate::skin::widgets::ff_x_button(ui, label)
}

/// Like [`remove_x_button`] but scoped under an explicit widget id (used by lists that need to
/// look a specific control up again).
fn remove_x_button_with_id(ui: &mut egui::Ui, id: egui::Id, label: &str) -> egui::Response {
    let response = ui
        .push_id(id, |ui| crate::skin::widgets::ff_x_button(ui, label))
        .inner;
    #[cfg(test)]
    X_RECTS.with(|m| m.borrow_mut().insert(id, response.rect));
    response
}

// Test support: where each explicit-id remove button was last laid out.
#[cfg(test)]
thread_local! {
    static X_RECTS: std::cell::RefCell<std::collections::HashMap<egui::Id, egui::Rect>> =
        std::cell::RefCell::new(std::collections::HashMap::new());
}

/// A 24px-high selectable chip (rarity-style toggle). Flips `*selected` and marks the response
/// changed on click; callers write only on `.changed()`.
pub fn toggle_chip(ui: &mut egui::Ui, label: &str, selected: &mut bool) -> egui::Response {
    crate::skin::editor::ff_chip(ui, label, selected)
}

/// A small outlined text badge (for example `from style`) with a hover tooltip. Not clickable.
pub fn badge(ui: &mut egui::Ui, text: &str, tip: &str) -> egui::Response {
    let inner = egui::Frame::new()
        .stroke(egui::Stroke::new(1.0, theme::LABEL_TAN))
        .inner_margin(egui::Margin::symmetric(4, 1))
        .show(ui, |ui| {
            ui.add(
                egui::Label::new(
                    egui::RichText::new(text)
                        .size(theme::ROW_SUMMARY_SIZE)
                        .color(theme::LABEL_TAN),
                )
                .selectable(false),
            )
        });
    inner.response.on_hover_text(tip)
}

/// The exact-number field's minimum width: room for three digits at regular 15 (the edit sits
/// 10 pt inside each side of the box).
pub const NUMBER_FIELD_W: f32 = 44.0;
/// Gap between a slider and its exact-number field.
const SLIDER_GAP: f32 = 8.0;
/// Extra width per character beyond three (a regular-15 digit is about 8 pt wide).
const NUMBER_CHAR_W: f32 = 9.0;

/// Width of an exact-number field that shows `chars` characters in full: [`NUMBER_FIELD_W`] for up
/// to three, growing by [`NUMBER_CHAR_W`] per extra character (WR-03: `BaseArmour 1500`,
/// `StackSize 5000`).
pub fn number_field_width(chars: usize) -> f32 {
    NUMBER_FIELD_W + NUMBER_CHAR_W * chars.saturating_sub(3) as f32
}

/// Characters needed to print the widest of `values`.
fn widest_chars(values: &[i64]) -> usize {
    values
        .iter()
        .map(|v| v.to_string().len())
        .max()
        .unwrap_or(0)
}

/// The skin slider for a whole number. The value is never clamped on display (a hand-written
/// out-of-range value must survive); it is written back only when the user moves the slider, and
/// the numeric readout is left to [`number_field`]. Log ranges (`log`) slide over `ln(1 + v - min)`.
/// The slider leaves the exact-number field (wide enough for the range ends and the current value,
/// see [`number_field_width`]) plus a gap free at the right end of the available width.
/// Callers write only when `.changed()`.
pub fn themed_slider(
    ui: &mut egui::Ui,
    value: &mut i64,
    min: i64,
    max: i64,
    log: bool,
) -> egui::Response {
    let skin = crate::skin::of(ui.ctx());
    let reserve = number_field_width(widest_chars(&[min, max, *value])) + SLIDER_GAP;
    let width = (ui.available_width() - reserve).max(120.0);
    let shown = (*value).clamp(min, max.max(min));
    let (mut x, range, step) = if log {
        (
            ((1 + shown - min) as f64).ln(),
            0.0..=((1 + max - min).max(1) as f64).ln(),
            0.0,
        )
    } else {
        (shown as f64, min as f64..=max as f64, 1.0)
    };
    let mut resp = ui
        .allocate_ui(egui::vec2(width, 24.0), |ui| {
            crate::skin::widgets::ff_slider(ui, &skin, &mut x, range, step, "Value", |_| {
                String::new()
            })
        })
        .inner;
    if resp.changed() {
        let new = if log {
            min + (x.exp() - 1.0).round() as i64
        } else {
            x.round() as i64
        }
        .clamp(min, max.max(min));
        if new == *value {
            resp.flags.remove(egui::response::Flags::CHANGED);
        } else {
            *value = new;
        }
    }
    // A log slider has no even step in slider space (step 0 above), so Left/Right step the whole
    // number by one here instead (WR-04).
    if log && resp.has_focus() {
        let delta = ui.input(|i| {
            i64::from(i.key_pressed(egui::Key::ArrowRight))
                - i64::from(i.key_pressed(egui::Key::ArrowLeft))
        });
        // A key only steps toward or within the range: a hand-written value outside it is never
        // pulled to the far end by a key pointing away from it.
        let hi = max.max(min);
        let blocked = (delta > 0 && *value >= hi) || (delta < 0 && *value <= min);
        if delta != 0 && !blocked {
            let new = ((*value).clamp(min, hi) + delta).clamp(min, hi);
            if new != *value {
                *value = new;
                resp.mark_changed();
            }
        }
    }
    resp
}

/// A persistent widget id from any `Hash` salt (egui's own salt bound also demands `Debug`).
fn salted_id(ui: &egui::Ui, salt: impl std::hash::Hash) -> egui::Id {
    use std::hash::Hasher;
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    salt.hash(&mut hasher);
    ui.make_persistent_id(hasher.finish())
}

/// What [`number_field`] reports: the TextEdit response (its `changed()` is set only when a new
/// value was committed) and whether the current draft fails to parse.
pub struct NumberField {
    pub response: egui::Response,
    pub invalid: bool,
}

/// An exact whole-number entry. The text lives in a draft kept in egui temp memory; it commits to
/// `*value` only when it parses as `i64` and differs, and resyncs from `*value` whenever the field
/// is not focused. The caller shows the `Enter a whole number` hint when `invalid`. The field is
/// `width` wide, or wider when the value or draft needs more than three characters.
pub fn number_field(
    ui: &mut egui::Ui,
    id_salt: impl std::hash::Hash,
    value: &mut i64,
    width: f32,
) -> NumberField {
    let id = salted_id(ui, id_salt);
    let focused = ui.memory(|m| m.has_focus(id));
    let mut draft: String = ui
        .data_mut(|d| d.get_temp::<String>(id))
        .unwrap_or_default();
    if !focused || ui.data(|d| d.get_temp::<String>(id)).is_none() {
        draft = value.to_string();
    }

    let chars = widest_chars(&[*value]).max(draft.chars().count());
    let width = width.max(number_field_width(chars));
    let mut response = crate::skin::widgets::ff_text_field_id(ui, id, &mut draft, "", width);
    let mut invalid = false;
    if response.changed() {
        match draft.trim().parse::<i64>() {
            Ok(n) if n != *value => {
                *value = n;
            }
            Ok(_) => {
                response.flags.remove(egui::response::Flags::CHANGED);
            }
            Err(_) => {
                invalid = true;
                response.flags.remove(egui::response::Flags::CHANGED);
            }
        }
    } else if focused && draft.trim().parse::<i64>().is_err() {
        invalid = true;
    }
    ui.data_mut(|d| d.insert_temp(id, draft));
    NumberField { response, invalid }
}

/// `R {r} G {g} B {b} A {a}` — the swatch caption and tooltip.
pub fn rgba_caption(rgba: [u8; 4]) -> String {
    format!("R {} G {} B {} A {}", rgba[0], rgba[1], rgba[2], rgba[3])
}

/// A 34x22 colour chip that opens the skinned picker below it. The chip carries no caption (the
/// `caption` is its tooltip) and shows the 2pt ring while its popup is open. `had_alpha` is the
/// line's current form (4 tokens); the form it had when the picker opened is remembered for as
/// long as the picker stays open. Returns the picker's edit, if any, with that opening form.
fn colour_chip(
    ui: &mut egui::Ui,
    rgba: [u8; 4],
    had_alpha: bool,
    caption: &str,
    mode: PickerMode,
) -> Option<([u8; 4], bool)> {
    let colour = egui::Color32::from_rgba_unmultiplied(rgba[0], rgba[1], rgba[2], rgba[3]);
    let probe_id = egui::Id::new(("ff-colour-chip-open", ui.next_auto_id()));
    let open = ui.data(|d| d.get_temp::<bool>(probe_id)).unwrap_or(false);
    let chip =
        ff_swatch(ui, Some(colour), open, egui::vec2(34.0, 22.0), "Colour").on_hover_text(caption);
    let popup_id = egui::Popup::default_response_id(&chip);
    // CR-02: live edits can change the line's form (an alpha edit turns `1 2 3` into
    // `1 2 3 128`), so the form is captured while the picker is closed and kept while it is open.
    let form_id = popup_id.with("had-alpha");
    if !egui::Popup::is_id_open(ui.ctx(), popup_id) {
        ui.data_mut(|d| d.insert_temp(form_id, had_alpha));
    }
    let out = colour_popup(ui, &chip, "rule", Some(rgba), rgba, mode);
    let opened_had_alpha = ui
        .data(|d| d.get_temp::<bool>(form_id))
        .unwrap_or(had_alpha);
    let now_open = egui::Popup::is_id_open(ui.ctx(), popup_id);
    ui.data_mut(|d| d.insert_temp(probe_id, now_open));
    match out {
        Some(PickerEdit::Set(c)) if c != rgba => Some((c, opened_had_alpha)),
        _ => None,
    }
}

/// An RGBA colour chip that opens the skinned picker with an Alpha slider. `had_alpha` is whether
/// the line currently has 4 tokens. Returns `Some((new, had_alpha_at_open))` only when the user
/// changed it and the unmultiplied result differs from `rgba`, so merely showing (or opening) a
/// translucent colour never rewrites its line. Encoding the edit with the form the line had when
/// the picker opened makes Escape restore a 3-token line byte for byte.
pub fn colour_swatch(ui: &mut egui::Ui, rgba: [u8; 4], had_alpha: bool) -> Option<([u8; 4], bool)> {
    let mode = PickerMode {
        alpha: true,
        allow_none: false,
    };
    colour_chip(ui, rgba, had_alpha, &rgba_caption(rgba), mode)
}

/// What a style colour slot did this frame.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SlotResult {
    Unchanged,
    Set([u8; 3]),
    Cleared,
}

/// One of the style editor's Text / Border / Background cards over the opaque picker (no Alpha,
/// with a None tile). `current` is the slot (`None` = unset, shown as `<label> (none)`); `seed` is
/// what an unset slot opens on. Opening or closing without a change returns `Unchanged`.
pub fn colour_slot(
    ui: &mut egui::Ui,
    tag: &str,
    label: &str,
    current: Option<[u8; 3]>,
    seed: [u8; 3],
    width: f32,
) -> SlotResult {
    let open_id = ui.id().with(("ff-colour-slot-open", tag));
    let open = ui.data(|d| d.get_temp::<bool>(open_id)).unwrap_or(false);
    let card = ff_colour_card_ex(
        ui,
        label,
        current.map(|c| egui::Color32::from_rgb(c[0], c[1], c[2])),
        width,
        open,
    );
    let name = format!("{label} colour");
    card.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, true, &name));
    let mode = PickerMode {
        alpha: false,
        allow_none: true,
    };
    let out = colour_popup(
        ui,
        &card,
        tag,
        current.map(|c| [c[0], c[1], c[2], 255]),
        [seed[0], seed[1], seed[2], 255],
        mode,
    );
    let now_open = egui::Popup::is_id_open(ui.ctx(), egui::Popup::default_response_id(&card));
    ui.data_mut(|d| d.insert_temp(open_id, now_open));
    match out {
        Some(PickerEdit::Set(c)) => SlotResult::Set([c[0], c[1], c[2]]),
        Some(PickerEdit::Clear) => SlotResult::Cleared,
        None => SlotResult::Unchanged,
    }
}

/// `R {r} G {g} B {b}` -- the opaque swatch caption and tooltip.
pub fn rgb_caption(rgb: [u8; 3]) -> String {
    format!("R {} G {} B {}", rgb[0], rgb[1], rgb[2])
}

/// An opaque RGB chip that opens the picker without an alpha channel. `Style` stores RGB and the
/// emitter always writes alpha 255, so a picker with alpha would show a translucency the filter
/// never emits. Returns `Some(new)` only when the picked RGB differs from `rgb`.
pub fn colour_swatch_rgb(ui: &mut egui::Ui, rgb: [u8; 3]) -> Option<[u8; 3]> {
    let mode = PickerMode {
        alpha: false,
        allow_none: false,
    };
    colour_chip(
        ui,
        [rgb[0], rgb[1], rgb[2], 255],
        false,
        &rgb_caption(rgb),
        mode,
    )
    .map(|(c, _)| [c[0], c[1], c[2]])
    .filter(|c| *c != rgb)
}

/// What one frame of [`chip_list`] asks the caller to do.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct ChipListOutcome {
    /// Index of the chip whose x was clicked.
    pub removed: Option<usize>,
    /// New entries committed from the add field (quoted pastes already split).
    pub added: Vec<String>,
    /// A visible refusal message, when an entry was not added.
    pub rejected: Option<String>,
}

const CHIP_REJECT_MESSAGE: &str = "Values cannot contain quotes or control characters";

/// True when every `"` in `draft` belongs to a closed quoted span and nothing but whitespace sits
/// outside those spans, i.e. it is either quote-free or a clean quoted paste
/// (`"Heavy Belt" "Utility Belt"`) that `parse_base_type_entry` splits without dropping text.
fn quoted_paste_is_clean(draft: &str) -> bool {
    if !draft.contains('"') {
        return true;
    }
    let mut inside = false;
    for c in draft.chars() {
        match c {
            '"' => inside = !inside,
            c if !inside && !c.is_whitespace() => return false,
            _ => {}
        }
    }
    !inside
}

/// A wrapped list of removable chips with an `N items` counter, a scroll cap and an add field.
///
/// Restores the v0.4 list-entry model: the caller owns the data; this reports one removal index
/// and any added entries, never mutating during iteration. The add draft lives in egui temp
/// memory under the widget id and is cleared after a commit.
pub fn chip_list(
    ui: &mut egui::Ui,
    id_salt: impl std::hash::Hash,
    items: &[String],
    hint: &str,
    _max_height: f32,
) -> ChipListOutcome {
    let id = salted_id(ui, id_salt);
    let mut outcome = ChipListOutcome::default();

    let count = items.len();
    ui.label(
        egui::RichText::new(format!("{count} item{}", if count == 1 { "" } else { "s" }))
            .size(theme::ROW_SUMMARY_SIZE)
            .color(theme::LABEL_TAN),
    );

    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing = egui::vec2(4.0, 4.0);
        for (index, item) in items.iter().enumerate() {
            egui::Frame::new()
                .fill(theme::INPUT_FILL)
                .stroke(egui::Stroke::new(1.0, theme::CONTROL_GLYPH))
                .inner_margin(egui::Margin {
                    left: 4,
                    right: 0,
                    top: 0,
                    bottom: 0,
                })
                .show(ui, |ui| {
                    ui.spacing_mut().item_spacing.x = 4.0;
                    ui.horizontal(|ui| {
                        ui.label(
                            egui::RichText::new(item)
                                .font(regular(15.0))
                                .color(pal::VALUE),
                        );
                        let label = format!("Remove {item}");
                        if remove_x_button_with_id(ui, id.with(("x", index)), &label).clicked() {
                            outcome.removed = Some(index);
                        }
                    });
                });
        }
    });

    let (added, rejected) = add_field(ui, id, hint);
    outcome.added = added;
    outcome.rejected = rejected;
    outcome
}

/// A vertical list: one row per entry (value on the left, its own remove X flush right) in a
/// scroll area capped at `max_rows` rows, then the shared add field. No counter line.
pub fn string_list(
    ui: &mut egui::Ui,
    id_salt: impl std::hash::Hash,
    items: &[String],
    hint: &str,
    max_rows: usize,
) -> ChipListOutcome {
    const ROW_H: f32 = 28.0;
    let id = salted_id(ui, id_salt);
    let mut outcome = ChipListOutcome::default();
    let skin = crate::skin::of(ui.ctx());

    if !items.is_empty() {
        let height = (items.len() as f32 * ROW_H).min(max_rows as f32 * ROW_H);
        ui.allocate_ui(egui::vec2(ui.available_width(), height), |ui| {
            ui.spacing_mut().item_spacing.y = 0.0;
            crate::skin::widgets::ff_scroll_area(ui, &skin, id.with("scroll"), |ui| {
                for (index, item) in items.iter().enumerate() {
                    let (rect, row) = ui.allocate_exact_size(
                        egui::vec2(ui.available_width(), ROW_H),
                        egui::Sense::hover(),
                    );
                    let x_rect = egui::Rect::from_center_size(
                        egui::pos2(rect.max.x - 14.0, rect.center().y),
                        egui::vec2(24.0, 24.0),
                    );
                    let text_w = (x_rect.min.x - rect.min.x - 12.0).max(20.0);
                    let mut job = egui::text::LayoutJob::simple(
                        item.clone(),
                        regular(15.0),
                        pal::VALUE,
                        text_w,
                    );
                    job.wrap.max_rows = 1;
                    job.wrap.break_anywhere = true;
                    let galley = ui.painter().layout_job(job);
                    ui.painter().galley(
                        egui::pos2(rect.min.x + 8.0, rect.center().y - galley.size().y / 2.0),
                        galley,
                        pal::VALUE,
                    );
                    row.on_hover_text(item);
                    let label = format!("Remove {item}");
                    ui.scope_builder(egui::UiBuilder::new().max_rect(x_rect), |ui| {
                        if remove_x_button_with_id(ui, id.with(("x", index)), &label).clicked() {
                            outcome.removed = Some(index);
                        }
                    });
                }
            });
        });
    }

    let (added, rejected) = add_field(ui, id, hint);
    outcome.added = added;
    outcome.rejected = rejected;
    outcome
}

/// The shared add field: draft in temp memory under `id`, Enter / + ADD commits, a quoted paste
/// is split, stray quotes and control characters are refused visibly with the draft kept.
fn add_field(ui: &mut egui::Ui, id: egui::Id, hint: &str) -> (Vec<String>, Option<String>) {
    let skin = crate::skin::of(ui.ctx());
    let field_id = id.with("field");
    let mut added = Vec::new();
    let mut rejected = None;
    let mut draft: String = ui
        .data_mut(|d| d.get_temp::<String>(id.with("draft")))
        .unwrap_or_default();
    let mut commit = false;
    let mut enter = false;
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 8.0;
        let width = (ui.available_width() - 84.0 - 8.0).max(60.0);
        let response =
            crate::skin::widgets::ff_text_field_id(ui, field_id, &mut draft, hint, width);
        enter = response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
        if crate::skin::widgets::ff_button(ui, &skin, "+ Add", 84.0).clicked() {
            commit = true;
        }
    });
    if enter || commit {
        // A quote that is not part of a clean quoted paste (`Foo"Bar`, an unclosed `"Foo`) would
        // otherwise vanish: refuse it visibly and keep the draft so it can be fixed (WR-06).
        let mut refused = !quoted_paste_is_clean(&draft);
        if !refused {
            for entry in parse_base_type_entry(&draft) {
                if entry.chars().any(|c| c == '"' || c.is_control()) {
                    refused = true;
                } else {
                    added.push(entry);
                }
            }
        }
        if refused {
            rejected = Some(CHIP_REJECT_MESSAGE.to_owned());
        } else {
            draft.clear();
        }
    }
    ui.data_mut(|d| d.insert_temp(id.with("draft"), draft));
    if enter {
        ui.memory_mut(|m| m.request_focus(field_id));
    }
    (added, rejected)
}

#[cfg(test)]
mod tests {
    use super::*;
    use egui::{Event, Modifiers, PointerButton, Pos2, RawInput, Rect};

    /// A one-frame click-and-release at `pos`, on a large-enough screen that the widget under
    /// test (allocated near the origin, per `run_ui`'s background-layer `Ui`) is never clipped.
    fn click_at(pos: Pos2) -> RawInput {
        RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, egui::vec2(400.0, 400.0))),
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

    fn headless_ctx() -> egui::Context {
        let ctx = egui::Context::default();
        crate::skin::fonts::install(&ctx);
        ctx
    }

    #[test]
    fn circular_toggle_flips_and_reports_changed_on_click() {
        let ctx = headless_ctx();
        let mut checked = false;

        // Frame 1: lay out the control with no pointer input, to learn where it lands.
        let mut rect = egui::Rect::NOTHING;
        ctx.run_ui(RawInput::default(), |ui| {
            rect = circular_toggle(ui, &mut checked).rect;
        })
        .drop_without_applying_deltas();
        assert!(!checked, "must start unchecked");

        // Frame 2: click at the widget's own center. Layout is deterministic (same starting
        // state, same call), so the rect computed in frame 1 is exactly where frame 2 paints it.
        let mut changed = false;
        ctx.run_ui(click_at(rect.center()), |ui| {
            changed = circular_toggle(ui, &mut checked).changed();
        })
        .drop_without_applying_deltas();

        assert!(checked, "a click must flip checked to true");
        assert!(changed, "a click must report Response::changed()");
    }

    #[test]
    fn slide_switch_flips_on_click() {
        let ctx = headless_ctx();
        let mut on = false;
        let mut rect = egui::Rect::NOTHING;
        ctx.run_ui(RawInput::default(), |ui| {
            rect = slide_switch(ui, &mut on).rect;
        })
        .drop_without_applying_deltas();
        assert!((rect.width() - 32.0).abs() < 7.0 && !on);

        let mut changed = false;
        ctx.run_ui(click_at(rect.center()), |ui| {
            changed = slide_switch(ui, &mut on).changed();
        })
        .drop_without_applying_deltas();
        assert!(on && changed);

        ctx.run_ui(click_at(rect.center()), |ui| {
            changed = slide_switch(ui, &mut on).changed();
        })
        .drop_without_applying_deltas();
        assert!(!on && changed);
    }

    #[test]
    fn slide_switch_idle_frame_changes_nothing() {
        let ctx = headless_ctx();
        let mut on = true;
        ctx.run_ui(RawInput::default(), |ui| {
            assert!(!slide_switch(ui, &mut on).changed());
        })
        .drop_without_applying_deltas();
        assert!(on);
    }

    #[test]
    fn circular_toggle_does_not_flip_without_a_click() {
        let ctx = headless_ctx();
        let mut checked = false;

        ctx.run_ui(RawInput::default(), |ui| {
            let response = circular_toggle(ui, &mut checked);
            assert!(!response.changed());
        })
        .drop_without_applying_deltas();
        assert!(!checked);
    }

    #[test]
    fn enabled_arrow_button_reports_clicked_on_click() {
        let ctx = headless_ctx();

        let mut rect = egui::Rect::NOTHING;
        ctx.run_ui(RawInput::default(), |ui| {
            rect = arrow_button(ui, ArrowDirection::Up, true).rect;
        })
        .drop_without_applying_deltas();

        let mut clicked = false;
        ctx.run_ui(click_at(rect.center()), |ui| {
            clicked = arrow_button(ui, ArrowDirection::Up, true).clicked();
        })
        .drop_without_applying_deltas();

        assert!(
            clicked,
            "an enabled arrow button must report clicked() on click"
        );
    }

    /// WR-08: the 16pt art keeps a padded 24x22 click target, so a click 3pt outside the art
    /// still counts.
    #[test]
    fn enabled_arrow_button_has_a_padded_click_target() {
        let ctx = headless_ctx();
        let mut rect = egui::Rect::NOTHING;
        ctx.run_ui(RawInput::default(), |ui| {
            ui.add_space(10.0);
            ui.horizontal(|ui| {
                ui.add_space(10.0);
                rect = arrow_button(ui, ArrowDirection::Up, true).rect;
            });
        })
        .drop_without_applying_deltas();
        assert_eq!(rect.size(), egui::vec2(24.0, 22.0));
        let art = rect.shrink2(crate::skin::widgets::ARROW_HIT_PAD);
        assert_eq!(art.size(), egui::vec2(16.0, 16.0));
        let mut clicked = false;
        ctx.run_ui(click_at(art.left_top() - egui::vec2(3.0, 2.5)), |ui| {
            ui.add_space(10.0);
            ui.horizontal(|ui| {
                ui.add_space(10.0);
                clicked = arrow_button(ui, ArrowDirection::Up, true).clicked();
            });
        })
        .drop_without_applying_deltas();
        assert!(clicked, "a click just outside the art must still count");
    }

    #[test]
    fn disabled_arrow_button_never_reports_clicked() {
        let ctx = headless_ctx();

        let mut rect = egui::Rect::NOTHING;
        ctx.run_ui(RawInput::default(), |ui| {
            rect = arrow_button(ui, ArrowDirection::Down, false).rect;
        })
        .drop_without_applying_deltas();

        let mut clicked = false;
        ctx.run_ui(click_at(rect.center()), |ui| {
            clicked = arrow_button(ui, ArrowDirection::Down, false).clicked();
        })
        .drop_without_applying_deltas();

        assert!(
            !clicked,
            "a disabled arrow button must never report clicked()"
        );
    }

    #[test]
    fn circular_toggle_is_a_22_square_check_that_flips_on_click() {
        let ctx = headless_ctx();
        let mut checked = false;
        let mut rect = egui::Rect::NOTHING;
        ctx.run_ui(RawInput::default(), |ui| {
            rect = circular_toggle(ui, &mut checked).rect;
        })
        .drop_without_applying_deltas();
        assert_eq!(rect.size(), egui::vec2(22.0, 22.0));
        ctx.run_ui(click_at(rect.center()), |ui| {
            circular_toggle(ui, &mut checked);
        })
        .drop_without_applying_deltas();
        assert!(checked, "click on the check must flip");
        let pos = rect.right_center() + egui::vec2(14.0, 0.0);
        ctx.run_ui(click_at(pos), |ui| {
            circular_toggle(ui, &mut checked);
        })
        .drop_without_applying_deltas();
        assert!(checked, "click outside the check must not flip");
    }

    #[test]
    fn remove_x_button_is_24px_and_clicks_only_on_click() {
        let ctx = headless_ctx();
        let mut rect = egui::Rect::NOTHING;
        ctx.run_ui(RawInput::default(), |ui| {
            let r = remove_x_button(ui, "Remove this condition");
            assert!(!r.clicked());
            rect = r.rect;
        })
        .drop_without_applying_deltas();
        assert_eq!(rect.size(), egui::vec2(24.0, 24.0));
        let mut clicked = false;
        ctx.run_ui(click_at(rect.center()), |ui| {
            clicked = remove_x_button(ui, "Remove this condition").clicked();
        })
        .drop_without_applying_deltas();
        assert!(clicked);
    }

    #[test]
    fn toggle_chip_flips_on_click_and_is_idle_otherwise() {
        let ctx = headless_ctx();
        let mut selected = false;
        let mut rect = egui::Rect::NOTHING;
        ctx.run_ui(RawInput::default(), |ui| {
            let r = toggle_chip(ui, "Normal", &mut selected);
            assert!(!r.changed());
            rect = r.rect;
        })
        .drop_without_applying_deltas();
        assert!(!selected);
        let mut changed = false;
        ctx.run_ui(click_at(rect.center()), |ui| {
            changed = toggle_chip(ui, "Normal", &mut selected).changed();
        })
        .drop_without_applying_deltas();
        assert!(selected && changed);
    }

    #[test]
    fn badge_never_reports_clicked() {
        let ctx = headless_ctx();
        let mut rect = egui::Rect::NOTHING;
        ctx.run_ui(RawInput::default(), |ui| {
            rect = badge(ui, "from style", "tip").rect;
        })
        .drop_without_applying_deltas();
        let mut clicked = true;
        ctx.run_ui(click_at(rect.center()), |ui| {
            clicked = badge(ui, "from style", "tip").clicked();
        })
        .drop_without_applying_deltas();
        assert!(!clicked);
    }

    /// Frame input carrying typed text (and nothing else).
    fn text_input(text: &str) -> RawInput {
        RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, egui::vec2(400.0, 400.0))),
            events: vec![Event::Text(text.to_owned())],
            ..Default::default()
        }
    }

    fn key_input(key: egui::Key) -> RawInput {
        RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, egui::vec2(400.0, 400.0))),
            events: vec![Event::Key {
                key,
                physical_key: Some(key),
                pressed: true,
                repeat: false,
                modifiers: Modifiers::default(),
            }],
            ..Default::default()
        }
    }

    #[test]
    fn themed_slider_never_clamps_on_display() {
        let ctx = headless_ctx();
        let mut a = 70_i64;
        let mut b = 400_i64;
        for _ in 0..2 {
            ctx.run_ui(RawInput::default(), |ui| {
                assert!(!themed_slider(ui, &mut a, 1, 100, false).changed());
                assert!(!themed_slider(ui, &mut b, 0, 30, false).changed());
            })
            .drop_without_applying_deltas();
        }
        assert_eq!((a, b), (70, 400));
    }

    #[test]
    fn themed_slider_log_maps_midpoint() {
        let ctx = headless_ctx();
        let mut v = 1_i64;
        let mut rect = egui::Rect::NOTHING;
        ctx.run_ui(
            RawInput {
                screen_rect: Some(Rect::from_min_size(Pos2::ZERO, egui::vec2(400.0, 400.0))),
                ..Default::default()
            },
            |ui| {
                rect = themed_slider(ui, &mut v, 1, 5000, true).rect;
            },
        )
        .drop_without_applying_deltas();
        // Track runs from rect.min + 4 to rect.max - 12 (empty value text keeps 12 pt).
        let mid = egui::pos2(
            (rect.min.x + 4.0 + rect.max.x - 12.0) / 2.0,
            rect.center().y,
        );
        let mut changed = false;
        ctx.run_ui(click_at(mid), |ui| {
            changed = themed_slider(ui, &mut v, 1, 5000, true).changed();
        })
        .drop_without_applying_deltas();
        assert!(changed);
        assert!(
            (50..=100).contains(&v),
            "log midpoint should be near sqrt(5000), got {v}"
        );
    }

    #[test]
    fn themed_slider_log_steps_by_one_on_arrow_keys() {
        let ctx = headless_ctx();
        let mut v = 1_i64;
        let mut rect = egui::Rect::NOTHING;
        ctx.run_ui(key_input(egui::Key::Home), |ui| {
            rect = themed_slider(ui, &mut v, 1, 5000, true).rect;
        })
        .drop_without_applying_deltas();
        // Click focuses the slider (and sets a value), then the arrows step by one.
        ctx.run_ui(click_at(rect.center()), |ui| {
            themed_slider(ui, &mut v, 1, 5000, true);
        })
        .drop_without_applying_deltas();
        let start = v;
        let mut changed = false;
        ctx.run_ui(key_input(egui::Key::ArrowRight), |ui| {
            changed = themed_slider(ui, &mut v, 1, 5000, true).changed();
        })
        .drop_without_applying_deltas();
        assert!(changed);
        assert_eq!(v, start + 1);
        ctx.run_ui(key_input(egui::Key::ArrowLeft), |ui| {
            changed = themed_slider(ui, &mut v, 1, 5000, true).changed();
        })
        .drop_without_applying_deltas();
        assert!(changed);
        assert_eq!(v, start);
    }

    #[test]
    fn themed_slider_log_arrow_keeps_out_of_range_value() {
        let ctx = headless_ctx();
        for (start, key) in [
            (1500_i64, egui::Key::ArrowRight),
            (-5, egui::Key::ArrowLeft),
        ] {
            let mut v = start;
            let mut id = egui::Id::NULL;
            ctx.run_ui(RawInput::default(), |ui| {
                id = themed_slider(ui, &mut v, 0, 1000, true).id;
            })
            .drop_without_applying_deltas();
            ctx.memory_mut(|m| m.request_focus(id));
            ctx.run_ui(RawInput::default(), |ui| {
                themed_slider(ui, &mut v, 0, 1000, true);
            })
            .drop_without_applying_deltas();
            assert!(ctx.memory(|m| m.has_focus(id)), "the slider has focus");
            let mut changed = true;
            ctx.run_ui(key_input(key), |ui| {
                changed = themed_slider(ui, &mut v, 0, 1000, true).changed();
            })
            .drop_without_applying_deltas();
            assert!(!changed, "{key:?} on {start} must not edit");
            assert_eq!(v, start);
        }
    }

    #[test]
    fn number_field_is_44_wide_and_keeps_focus_id() {
        let ctx = headless_ctx();
        let mut value = 5_i64;
        let mut rect = egui::Rect::NOTHING;
        let mut id = egui::Id::NULL;
        ctx.run_ui(RawInput::default(), |ui| {
            id = salted_id(ui, "n");
            rect = number_field(ui, "n", &mut value, NUMBER_FIELD_W)
                .response
                .rect;
        })
        .drop_without_applying_deltas();
        // The edit sits 10 pt inside each side of the 44 pt box.
        assert_eq!(rect.width() + 20.0, 44.0);
        ctx.run_ui(click_at(rect.center()), |ui| {
            number_field(ui, "n", &mut value, NUMBER_FIELD_W);
        })
        .drop_without_applying_deltas();
        assert!(
            ctx.memory(|m| m.has_focus(id)),
            "the field keeps its salted id for focus"
        );
    }

    #[test]
    fn number_field_grows_for_four_digits_and_the_slider_leaves_room() {
        assert_eq!(number_field_width(3), NUMBER_FIELD_W);
        assert!(
            number_field_width(4) >= 52.0,
            "four digits need ~32 pt plus 20"
        );
        assert!(number_field_width(5) > number_field_width(4));
        let ctx = headless_ctx();
        let (mut slider, mut field) = (egui::Rect::NOTHING, egui::Rect::NOTHING);
        let mut v = 1500_i64;
        ctx.run_ui(RawInput::default(), |ui| {
            ui.set_max_width(360.0);
            ui.horizontal(|ui| {
                slider = themed_slider(ui, &mut v, 0, 1000, true).rect;
                field = number_field(ui, "n", &mut v, NUMBER_FIELD_W).response.rect;
            });
        })
        .drop_without_applying_deltas();
        assert_eq!(field.width() + 20.0, number_field_width(4));
        assert!(field.min.x >= slider.max.x, "{slider:?} {field:?}");
        assert!(field.max.x <= 360.0 + 1.0, "{field:?}");
    }

    /// Focus the number field (click), then type `text`; returns (value, invalid, changed-on-type).
    fn type_into_number_field(start: i64, text: &str) -> (i64, bool, bool) {
        let ctx = headless_ctx();
        let mut value = start;
        let mut rect = egui::Rect::NOTHING;
        ctx.run_ui(RawInput::default(), |ui| {
            let f = number_field(ui, "n", &mut value, NUMBER_FIELD_W);
            assert!(!f.response.changed() && !f.invalid, "idle frame is quiet");
            rect = f.response.rect;
        })
        .drop_without_applying_deltas();
        ctx.run_ui(click_at(rect.center()), |ui| {
            number_field(ui, "n", &mut value, NUMBER_FIELD_W);
        })
        .drop_without_applying_deltas();
        ctx.run_ui(RawInput::default(), |ui| {
            number_field(ui, "n", &mut value, NUMBER_FIELD_W);
        })
        .drop_without_applying_deltas();
        let (mut invalid, mut changed) = (false, false);
        ctx.run_ui(text_input(text), |ui| {
            let f = number_field(ui, "n", &mut value, NUMBER_FIELD_W);
            invalid = f.invalid;
            changed = f.response.changed();
        })
        .drop_without_applying_deltas();
        (value, invalid, changed)
    }

    #[test]
    fn number_field_rejects_garbage_without_writing() {
        let (value, invalid, changed) = type_into_number_field(5, "abc");
        assert_eq!(value, 5);
        assert!(invalid);
        assert!(!changed);
    }

    #[test]
    fn number_field_commits_a_new_number() {
        let (value, invalid, changed) = type_into_number_field(5, "7");
        assert!(
            value == 57 || value == 75,
            "typed digit joins the draft, got {value}"
        );
        assert!(!invalid);
        assert!(changed);
    }

    #[test]
    fn number_field_same_number_is_not_a_change() {
        let (value, invalid, changed) = type_into_number_field(0, "0");
        assert_eq!(value, 0);
        assert!(!invalid);
        assert!(!changed);
    }

    #[test]
    fn colour_swatch_idle_frames_return_none_even_when_translucent() {
        let ctx = headless_ctx();
        for rgba in [[40, 42, 54, 255], [255, 121, 198, 128], [40, 42, 54, 0]] {
            for _ in 0..2 {
                ctx.run_ui(RawInput::default(), |ui| {
                    assert_eq!(colour_swatch(ui, rgba, rgba[3] != 255), None);
                })
                .drop_without_applying_deltas();
            }
        }
    }

    /// WR-04: the picker's colour model keeps RGB at any alpha, including 0, so a transparent
    /// colour round-trips exactly and raising its alpha keeps its RGB.
    #[test]
    fn colour_model_keeps_rgb_at_zero_and_low_alpha() {
        for rgb in [
            [40u8, 42, 54],
            [255, 121, 198],
            [0, 0, 0],
            [255, 255, 255],
            [1, 2, 3],
        ] {
            for a in [0u8, 1, 2, 7, 128, 254, 255] {
                let rgba = [rgb[0], rgb[1], rgb[2], a];
                assert_eq!(
                    crate::skin::colour_picker::hsva_of(rgba).to_srgba_unmultiplied(),
                    rgba,
                    "{rgba:?}"
                );
            }
        }
        let mut h = crate::skin::colour_picker::hsva_of([40, 42, 54, 0]);
        h.a = 1.0;
        assert_eq!(h.to_srgba_unmultiplied(), [40, 42, 54, 255]);
    }

    #[test]
    fn rgba_caption_formats_all_four_channels() {
        assert_eq!(
            rgba_caption([255, 121, 198, 128]),
            "R 255 G 121 B 198 A 128"
        );
    }

    fn chip_items() -> Vec<String> {
        ["Heavy Belt", "Utility Belt", "Sapphire Ring"]
            .map(String::from)
            .to_vec()
    }

    #[test]
    fn chip_list_idle_frame_reports_nothing() {
        let ctx = headless_ctx();
        let items = chip_items();
        ctx.run_ui(RawInput::default(), |ui| {
            let out = chip_list(ui, "chips", &items, "Add a base type", 160.0);
            assert_eq!(out, ChipListOutcome::default());
        })
        .drop_without_applying_deltas();
    }

    /// WR-07: the add field shows the caller's hint, not a fixed one.
    #[test]
    fn add_field_shows_the_callers_hint() {
        let ctx = headless_ctx();
        let items = chip_items();
        let hint = "Type a whole number, press Enter";
        let mut full = ctx.run_ui(RawInput::default(), |ui| {
            chip_list(ui, "chips", &items, hint, 160.0);
            string_list(ui, "list", &items, "Type a mod, press Enter", 7);
        });
        full.textures_delta.clear();
        let texts: Vec<String> = full
            .shapes
            .iter()
            .filter_map(|c| match &c.shape {
                egui::epaint::Shape::Text(t) => Some(t.galley.text().to_string()),
                _ => None,
            })
            .collect();
        assert!(texts.iter().any(|t| t == hint), "{texts:?}");
        assert!(texts.iter().any(|t| t == "Type a mod, press Enter"));
        assert!(!texts.iter().any(|t| t == "Type a name, press Enter"));
    }

    #[test]
    fn chip_list_x_click_reports_that_index() {
        let ctx = headless_ctx();
        let items = chip_items();
        let mut x_id = egui::Id::NULL;
        ctx.run_ui(RawInput::default(), |ui| {
            x_id = salted_id(ui, "chips").with(("x", 1usize));
            chip_list(ui, "chips", &items, "hint", 160.0);
        })
        .drop_without_applying_deltas();
        let rect = X_RECTS.with(|m| m.borrow().get(&x_id).copied().unwrap_or(Rect::NOTHING));
        assert_eq!(rect.size(), egui::vec2(24.0, 24.0));
        let mut out = ChipListOutcome::default();
        ctx.run_ui(click_at(rect.center()), |ui| {
            out = chip_list(ui, "chips", &items, "hint", 160.0);
        })
        .drop_without_applying_deltas();
        assert_eq!(out.removed, Some(1));
    }

    /// Preload a draft, focus the field, press Enter; returns the outcome of the Enter frame.
    fn commit_with_enter(draft: &str) -> (ChipListOutcome, String) {
        let ctx = headless_ctx();
        let items = chip_items();
        let mut ids = (egui::Id::NULL, egui::Id::NULL);
        ctx.run_ui(RawInput::default(), |ui| {
            let id = salted_id(ui, "chips");
            ids = (id.with("draft"), id.with("field"));
            ui.data_mut(|d| d.insert_temp(ids.0, draft.to_owned()));
            ui.memory_mut(|m| m.request_focus(ids.1));
            chip_list(ui, "chips", &items, "hint", 160.0);
        })
        .drop_without_applying_deltas();
        ctx.run_ui(RawInput::default(), |ui| {
            chip_list(ui, "chips", &items, "hint", 160.0);
        })
        .drop_without_applying_deltas();
        let mut out = ChipListOutcome::default();
        ctx.run_ui(key_input(egui::Key::Enter), |ui| {
            out = chip_list(ui, "chips", &items, "hint", 160.0);
        })
        .drop_without_applying_deltas();
        let left = ctx
            .data_mut(|d| d.get_temp::<String>(ids.0))
            .unwrap_or_default();
        (out, left)
    }

    #[test]
    fn chip_list_enter_splits_a_quoted_paste_and_clears_the_draft() {
        let (out, left) = commit_with_enter("\"Heavy Belt\" \"Utility Belt\"");
        assert_eq!(out.added, ["Heavy Belt", "Utility Belt"]);
        assert!(left.is_empty());
    }

    #[test]
    fn chip_list_blank_draft_adds_nothing() {
        let (out, _) = commit_with_enter("   ");
        assert!(out.added.is_empty());
    }

    #[test]
    fn chip_list_control_characters_are_refused() {
        let (out, _) = commit_with_enter("Bad\u{7}Name");
        assert!(out.added.is_empty());
        assert_eq!(out.rejected.as_deref(), Some(CHIP_REJECT_MESSAGE));
    }

    /// WR-06: a stray quote is refused with the message and the draft is kept, instead of the
    /// entry vanishing silently.
    #[test]
    fn chip_list_stray_quotes_are_refused_and_the_draft_kept() {
        for draft in [
            "Foo\"Bar",
            "\"Heavy Belt",
            "\"A\" B\"C\"",
            "x \"Heavy Belt\"",
        ] {
            let (out, left) = commit_with_enter(draft);
            assert!(out.added.is_empty(), "{draft}: {:?}", out.added);
            assert_eq!(
                out.rejected.as_deref(),
                Some(CHIP_REJECT_MESSAGE),
                "{draft}"
            );
            assert_eq!(left, draft);
        }
        assert!(quoted_paste_is_clean("Heavy Belt"));
        assert!(quoted_paste_is_clean(" \"Heavy Belt\"  \"Utility Belt\" "));
    }

    #[test]
    fn chip_list_add_button_commits_a_plain_entry() {
        let ctx = headless_ctx();
        let items = chip_items();
        let mut ids = (egui::Id::NULL, egui::Id::NULL);
        ctx.run_ui(RawInput::default(), |ui| {
            let id = salted_id(ui, "chips");
            ids = (id.with("draft"), id.with("field"));
            ui.data_mut(|d| d.insert_temp(ids.0, "Sapphire Ring".to_owned()));
            chip_list(ui, "chips", &items, "hint", 160.0);
        })
        .drop_without_applying_deltas();
        let field = ctx
            .read_response(ids.1)
            .map(|r| r.rect)
            .unwrap_or(Rect::NOTHING);
        let pos = field.right_center() + egui::vec2(20.0, 0.0);
        let mut out = ChipListOutcome::default();
        ctx.run_ui(click_at(pos), |ui| {
            out = chip_list(ui, "chips", &items, "hint", 160.0);
        })
        .drop_without_applying_deltas();
        assert_eq!(out.added, ["Sapphire Ring"]);
    }

    #[test]
    fn string_list_one_row_per_entry_and_remove() {
        let ctx = headless_ctx();
        let items = chip_items();
        let mut x_id = egui::Id::NULL;
        ctx.run_ui(RawInput::default(), |ui| {
            x_id = salted_id(ui, "list").with(("x", 1usize));
            string_list(ui, "list", &items, "hint", 7);
        })
        .drop_without_applying_deltas();
        let rect = X_RECTS.with(|m| m.borrow().get(&x_id).copied().unwrap_or(Rect::NOTHING));
        assert_eq!(rect.size(), egui::vec2(24.0, 24.0));
        let mut out = ChipListOutcome::default();
        ctx.run_ui(click_at(rect.center()), |ui| {
            out = string_list(ui, "list", &items, "hint", 7);
        })
        .drop_without_applying_deltas();
        assert_eq!(out.removed, Some(1));
        assert!(out.added.is_empty());
    }

    fn string_list_commit(draft: &str) -> (ChipListOutcome, String) {
        let ctx = headless_ctx();
        let items = chip_items();
        let mut ids = (egui::Id::NULL, egui::Id::NULL);
        ctx.run_ui(RawInput::default(), |ui| {
            let id = salted_id(ui, "list");
            ids = (id.with("draft"), id.with("field"));
            ui.data_mut(|d| d.insert_temp(ids.0, draft.to_owned()));
            ui.memory_mut(|m| m.request_focus(ids.1));
            string_list(ui, "list", &items, "hint", 7);
        })
        .drop_without_applying_deltas();
        ctx.run_ui(RawInput::default(), |ui| {
            string_list(ui, "list", &items, "hint", 7);
        })
        .drop_without_applying_deltas();
        let mut out = ChipListOutcome::default();
        ctx.run_ui(key_input(egui::Key::Enter), |ui| {
            out = string_list(ui, "list", &items, "hint", 7);
        })
        .drop_without_applying_deltas();
        let left = ctx
            .data_mut(|d| d.get_temp::<String>(ids.0))
            .unwrap_or_default();
        (out, left)
    }

    #[test]
    fn string_list_quoted_paste_splits() {
        let (out, left) = string_list_commit("\"Heavy Belt\" \"Utility Belt\"");
        assert_eq!(out.added, ["Heavy Belt", "Utility Belt"]);
        assert!(left.is_empty());
    }

    #[test]
    fn string_list_rejects_a_stray_quote() {
        let (out, left) = string_list_commit("Foo\"Bar");
        assert!(out.added.is_empty());
        assert!(out.rejected.is_some());
        assert_eq!(left, "Foo\"Bar");
    }

    #[test]
    fn string_list_idle_is_quiet() {
        let ctx = headless_ctx();
        let items = chip_items();
        ctx.run_ui(RawInput::default(), |ui| {
            assert_eq!(
                string_list(ui, "list", &items, "hint", 7),
                ChipListOutcome::default()
            );
        })
        .drop_without_applying_deltas();
    }
    #[test]
    fn chip_list_counter_pluralises() {
        for (n, expected) in [(14usize, "14 items"), (1, "1 item")] {
            let ctx = headless_ctx();
            let items: Vec<String> = (0..n).map(|i| format!("Base {i}")).collect();
            let mut found = false;
            let out = ctx.run_ui(RawInput::default(), |ui| {
                chip_list(ui, "chips", &items, "hint", 160.0);
            });
            for clipped in &out.shapes {
                if let egui::epaint::Shape::Text(t) = &clipped.shape {
                    if t.galley.text() == expected {
                        found = true;
                    }
                }
            }
            out.drop_without_applying_deltas();
            assert!(found, "caption {expected:?} not painted");
        }
    }

    #[test]
    fn rgb_caption_formats_three_channels() {
        assert_eq!(rgb_caption([255, 121, 198]), "R 255 G 121 B 198");
    }

    // ---- skinned colour picker wiring ----

    fn click_events(pos: Pos2) -> Vec<Event> {
        let button = |pressed| Event::PointerButton {
            pos,
            button: PointerButton::Primary,
            pressed,
            modifiers: Modifiers::default(),
        };
        vec![Event::PointerMoved(pos), button(true), button(false)]
    }

    fn picker_input(events: Vec<Event>) -> RawInput {
        RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, egui::vec2(660.0, 900.0))),
            events,
            ..Default::default()
        }
    }

    /// One frame of `colour_swatch`; returns its result and every painted text.
    fn swatch_frame(
        ctx: &egui::Context,
        events: Vec<Event>,
        rgba: [u8; 4],
    ) -> (Option<([u8; 4], bool)>, Vec<String>) {
        crate::skin::colour_picker::probes_clear();
        let mut out = None;
        let full = ctx.run_ui(picker_input(events), |ui| {
            out = colour_swatch(ui, rgba, rgba[3] != 255);
        });
        let mut texts = Vec::new();
        for clipped in &full.shapes {
            collect_text(&clipped.shape, &mut texts);
        }
        full.drop_without_applying_deltas();
        (out, texts)
    }

    fn collect_text(shape: &egui::Shape, out: &mut Vec<String>) {
        match shape {
            egui::Shape::Text(t) => out.push(t.galley.text().to_owned()),
            egui::Shape::Vec(v) => v.iter().for_each(|s| collect_text(s, out)),
            _ => {}
        }
    }

    #[test]
    fn colour_swatch_paints_no_caption() {
        let ctx = headless_ctx();
        let rgba = [255, 121, 198, 255];
        for _ in 0..2 {
            let (_, texts) = swatch_frame(&ctx, vec![], rgba);
            assert!(
                texts.iter().all(|t| !t.contains("R 255")),
                "caption painted: {texts:?}"
            );
        }
    }

    #[test]
    fn opening_the_rule_picker_emits_nothing_and_a_quick_pick_keeps_alpha() {
        for rgba in [[40, 42, 54, 255], [1, 2, 3, 128]] {
            let ctx = headless_ctx();
            swatch_frame(&ctx, vec![], rgba);
            crate::skin::colour_picker::request_open(&ctx, "rule");
            for _ in 0..3 {
                let (out, _) = swatch_frame(&ctx, vec![], rgba);
                assert_eq!(out, None, "an open, untouched picker emits nothing");
            }
            let pink = crate::skin::colour_picker::probe_rect("quick:Pink").expect("open");
            assert!(crate::skin::colour_picker::probe_rect("alpha").is_some());
            assert!(crate::skin::colour_picker::probe_rect("none").is_none());
            let (out, _) = swatch_frame(&ctx, click_events(pink.center()), rgba);
            assert_eq!(out, Some(([255, 121, 198, rgba[3]], rgba[3] != 255)));
        }
    }

    /// CR-02: an alpha edit turns a 3-token line into 4 tokens; Escape must report the form the
    /// line had when the picker opened, not its current one.
    #[test]
    fn escape_after_an_alpha_edit_reports_the_opening_form() {
        let ctx = headless_ctx();
        let start = [1, 2, 3, 255];
        swatch_frame(&ctx, vec![], start);
        crate::skin::colour_picker::request_open(&ctx, "rule");
        for _ in 0..3 {
            swatch_frame(&ctx, vec![], start);
        }
        let alpha = crate::skin::colour_picker::probe_rect("alpha").expect("open");
        let (out, _) = swatch_frame(&ctx, click_events(alpha.left_center()), start);
        let Some((edited, form)) = out else {
            panic!("the alpha click emitted nothing");
        };
        assert_ne!(edited[3], 255, "{edited:?}");
        assert!(!form, "a 3-token line opened the picker");
        // The caller wrote 4 tokens; the line now has alpha. Escape restores the opening value.
        let escape = vec![Event::Key {
            key: egui::Key::Escape,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: Modifiers::default(),
        }];
        let (out, _) = swatch_frame(&ctx, escape, edited);
        assert_eq!(out, Some((start, false)));
    }

    #[test]
    fn colour_swatch_rgb_has_no_alpha_slider() {
        let ctx = headless_ctx();
        let frame = |ctx: &egui::Context, events: Vec<Event>| {
            crate::skin::colour_picker::probes_clear();
            let mut out = None;
            ctx.run_ui(picker_input(events), |ui| {
                out = colour_swatch_rgb(ui, [1, 2, 3]);
            })
            .drop_without_applying_deltas();
            out
        };
        frame(&ctx, vec![]);
        crate::skin::colour_picker::request_open(&ctx, "rule");
        for _ in 0..3 {
            assert_eq!(frame(&ctx, vec![]), None);
        }
        assert!(crate::skin::colour_picker::probe_rect("square").is_some());
        assert!(crate::skin::colour_picker::probe_rect("alpha").is_none());
        let pink = crate::skin::colour_picker::probe_rect("quick:Pink").unwrap();
        assert_eq!(
            frame(&ctx, click_events(pink.center())),
            Some([255, 121, 198])
        );
    }

    fn slot_frame(ctx: &egui::Context, events: Vec<Event>, current: Option<[u8; 3]>) -> SlotResult {
        crate::skin::colour_picker::probes_clear();
        let mut out = SlotResult::Unchanged;
        ctx.run_ui(picker_input(events), |ui| {
            out = colour_slot(ui, "text", "Text", current, [248, 248, 242], 110.0);
        })
        .drop_without_applying_deltas();
        out
    }

    #[test]
    fn colour_slot_idle_and_open_emit_nothing_set_or_unset() {
        for current in [Some([1, 2, 3]), None] {
            let ctx = headless_ctx();
            assert_eq!(slot_frame(&ctx, vec![], current), SlotResult::Unchanged);
            crate::skin::colour_picker::request_open(&ctx, "text");
            for _ in 0..3 {
                assert_eq!(slot_frame(&ctx, vec![], current), SlotResult::Unchanged);
            }
            assert!(crate::skin::colour_picker::probe_rect("square").is_some());
            assert!(crate::skin::colour_picker::probe_rect("alpha").is_none());
            assert!(crate::skin::colour_picker::probe_rect("none").is_some());
        }
    }

    #[test]
    fn colour_slot_quick_pick_sets_and_none_tile_clears() {
        let ctx = headless_ctx();
        slot_frame(&ctx, vec![], None);
        crate::skin::colour_picker::request_open(&ctx, "text");
        for _ in 0..3 {
            slot_frame(&ctx, vec![], None);
        }
        let pink = crate::skin::colour_picker::probe_rect("quick:Pink").unwrap();
        assert_eq!(
            slot_frame(&ctx, click_events(pink.center()), None),
            SlotResult::Set([255, 121, 198])
        );

        let ctx = headless_ctx();
        let cur = Some([1, 2, 3]);
        slot_frame(&ctx, vec![], cur);
        crate::skin::colour_picker::request_open(&ctx, "text");
        for _ in 0..3 {
            slot_frame(&ctx, vec![], cur);
        }
        let none = crate::skin::colour_picker::probe_rect("none").unwrap();
        assert_eq!(
            slot_frame(&ctx, click_events(none.center()), cur),
            SlotResult::Cleared
        );
    }
}
