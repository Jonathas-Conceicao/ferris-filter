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

/// The v0.4 slider look: a visible warm rail, circle handle, `CONTROL_GLYPH` trailing fill. The
/// value is never clamped to `min..=max` (a hand-written out-of-range value must survive), and the
/// numeric readout is left to [`number_field`]. Callers write only when `.changed()`.
pub fn themed_slider(
    ui: &mut egui::Ui,
    value: &mut i64,
    min: i64,
    max: i64,
    log: bool,
) -> egui::Response {
    ui.scope(|ui| {
        let width = (ui.available_width() - 96.0).clamp(120.0, 200.0);
        let spacing = ui.spacing_mut();
        spacing.slider_width = width;
        spacing.slider_rail_height = 16.0;

        let visuals = ui.visuals_mut();
        visuals.handle_shape = egui::style::HandleShape::Circle;
        visuals.widgets.inactive.bg_fill = theme::HEADER_PLATE;
        visuals.widgets.hovered.bg_fill = theme::HEADER_PLATE;
        visuals.widgets.active.bg_fill = theme::HEADER_PLATE;
        visuals.selection.bg_fill = theme::CONTROL_GLYPH;
        for state in [
            &mut visuals.widgets.inactive,
            &mut visuals.widgets.hovered,
            &mut visuals.widgets.active,
        ] {
            state.fg_stroke = egui::Stroke::new(1.0, theme::SECTION_CREAM);
        }

        ui.add(
            egui::Slider::new(value, min..=max)
                .integer()
                .show_value(false)
                .clamping(egui::SliderClamping::Never)
                .logarithmic(log)
                .trailing_fill(true),
        )
    })
    .inner
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
/// is not focused. The caller shows the `Enter a whole number` hint when `invalid`.
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

    let mut response = ui.add(
        egui::TextEdit::singleline(&mut draft)
            .id(id)
            .desired_width(width),
    );
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

/// Unmultiplied HSVA of a filter colour. Going through `Hsva::from_rgba_unmultiplied` (not a
/// premultiplied `Color32`) keeps the RGB of a fully transparent colour such as
/// `SetBackgroundColor 40 42 54 0` (WR-04).
fn hsva_of(rgba: [u8; 4]) -> egui::ecolor::Hsva {
    use egui::ecolor::linear_f32_from_gamma_u8 as lin;
    egui::ecolor::Hsva::from_rgba_unmultiplied(
        lin(rgba[0]),
        lin(rgba[1]),
        lin(rgba[2]),
        f32::from(rgba[3]) / 255.0,
    )
}

/// An RGBA colour swatch that opens egui's picker (blend alpha only). Returns `Some(new)` only when
/// the user changed it and the unmultiplied result differs from `rgba`, so merely showing a
/// translucent colour never rewrites its line. The picker edits unmultiplied HSVA, so raising the
/// alpha of a transparent colour keeps its RGB.
pub fn colour_swatch(ui: &mut egui::Ui, rgba: [u8; 4]) -> Option<[u8; 4]> {
    let mut colour = hsva_of(rgba);
    let caption = rgba_caption(rgba);
    let mut result = None;
    ui.horizontal(|ui| {
        let response = egui::color_picker::color_edit_button_hsva(
            ui,
            &mut colour,
            egui::color_picker::Alpha::OnlyBlend,
        );
        if response.changed() {
            let now = colour.to_srgba_unmultiplied();
            if now != rgba {
                result = Some(now);
            }
        }
        response.on_hover_text(&caption);
        ui.label(
            egui::RichText::new(&caption)
                .size(theme::ROW_SUMMARY_SIZE)
                .color(theme::LABEL_TAN),
        );
    });
    result
}

/// `R {r} G {g} B {b}` -- the opaque swatch caption and tooltip.
pub fn rgb_caption(rgb: [u8; 3]) -> String {
    format!("R {} G {} B {}", rgb[0], rgb[1], rgb[2])
}

/// An opaque RGB swatch that opens egui's picker without an alpha channel. `Style` stores RGB
/// and the emitter always writes alpha 255, so a picker with alpha would show a translucency the
/// filter never emits. Returns `Some(new)` only when the picked RGB differs from `rgb`.
pub fn colour_swatch_rgb(ui: &mut egui::Ui, rgb: [u8; 3]) -> Option<[u8; 3]> {
    let mut colour = hsva_of([rgb[0], rgb[1], rgb[2], 255]);
    let caption = rgb_caption(rgb);
    let mut result = None;
    ui.horizontal(|ui| {
        let response = egui::color_picker::color_edit_button_hsva(
            ui,
            &mut colour,
            egui::color_picker::Alpha::Opaque,
        );
        if response.changed() {
            let now = colour.to_srgba_unmultiplied();
            let now = [now[0], now[1], now[2]];
            if now != rgb {
                result = Some(now);
            }
        }
        response.on_hover_text(&caption);
        ui.label(
            egui::RichText::new(&caption)
                .size(theme::ROW_SUMMARY_SIZE)
                .color(theme::LABEL_TAN),
        );
    });
    result
}

fn rgb_or_unset(rgb: Option<[u8; 3]>) -> String {
    rgb.map_or_else(|| "Not set (game default)".to_string(), rgb_caption)
}

/// The sample-text size of a preview chip for a style font size: clamped to what fits the chip.
fn chip_font_size(font: Option<u8>) -> f32 {
    font.map_or(14.0, |f| f32::from(f).clamp(12.0, 18.0))
}

/// A 120x24 preview of how a style draws an item label: background over a checkerboard, a 2px
/// border and the sample text `Item Name`. Hide styles are drawn at half strength with a strike
/// line and say so in the tooltip.
pub fn style_preview_chip(
    ui: &mut egui::Ui,
    name: &str,
    style: &crate::models::config::Style,
) -> egui::Response {
    use crate::models::config::StyleAction;

    let (rect, response) = ui.allocate_exact_size(egui::vec2(120.0, 24.0), egui::Sense::hover());
    let hidden = style.action == StyleAction::Hide;
    let fade = |c: egui::Color32| if hidden { c.gamma_multiply(0.5) } else { c };
    let rgb = |c: [u8; 3]| egui::Color32::from_rgb(c[0], c[1], c[2]);

    let painter = ui.painter_at(rect);
    // 6px checkerboard so any translucency would be visible.
    for cy in 0..4 {
        for cx in 0..20 {
            let shade = if (cx + cy) % 2 == 0 {
                egui::Color32::from_gray(0x2A)
            } else {
                egui::Color32::from_gray(0x1A)
            };
            let cell = egui::Rect::from_min_size(
                rect.min + egui::vec2(cx as f32 * 6.0, cy as f32 * 6.0),
                egui::vec2(6.0, 6.0),
            );
            painter.rect_filled(cell, 0.0, shade);
        }
    }
    if let Some(bg) = style.background {
        painter.rect_filled(rect, 4.0, rgb(bg));
    }
    if let Some(border) = style.border {
        painter.rect_stroke(
            rect.shrink(1.0),
            4.0,
            egui::Stroke::new(2.0, fade(rgb(border))),
            egui::StrokeKind::Inside,
        );
    }
    let text_colour = style.text.map_or(theme::VALUE_TEXT, rgb);
    painter.text(
        rect.center(),
        egui::Align2::CENTER_CENTER,
        "Item Name",
        egui::FontId::proportional(chip_font_size(style.font)),
        fade(text_colour),
    );
    if hidden {
        painter.line_segment(
            [rect.left_bottom(), rect.right_top()],
            egui::Stroke::new(1.0, theme::DISABLED_TEXT),
        );
    }

    let label = format!("Preview of style {name}");
    response.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Label, true, &label));
    let mut tip = String::new();
    if hidden {
        tip.push_str("Hidden in game; Alt reveals it.\n");
    }
    tip.push_str(&format!(
        "Text: {}\nBorder: {}\nBackground: {}",
        rgb_or_unset(style.text),
        rgb_or_unset(style.border),
        rgb_or_unset(style.background)
    ));
    response.on_hover_text(tip)
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
    max_height: f32,
) -> ChipListOutcome {
    let id = salted_id(ui, id_salt);
    let mut outcome = ChipListOutcome::default();

    let count = items.len();
    ui.label(
        egui::RichText::new(format!("{count} item{}", if count == 1 { "" } else { "s" }))
            .size(theme::ROW_SUMMARY_SIZE)
            .color(theme::LABEL_TAN),
    );

    egui::ScrollArea::vertical()
        .id_salt(id.with("scroll"))
        .max_height(max_height)
        .auto_shrink([false, true])
        .show(ui, |ui| {
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
                                        .size(theme::ROW_SUMMARY_SIZE)
                                        .color(theme::VALUE_TEXT),
                                );
                                let label = format!("Remove {item}");
                                if remove_x_button_with_id(ui, id.with(("x", index)), &label)
                                    .clicked()
                                {
                                    outcome.removed = Some(index);
                                }
                            });
                        });
                }
            });
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
    const ROW_H: f32 = 26.0;
    let id = salted_id(ui, id_salt);
    let mut outcome = ChipListOutcome::default();

    egui::ScrollArea::vertical()
        .id_salt(id.with("scroll"))
        .max_height(max_rows as f32 * ROW_H)
        .auto_shrink([false, true])
        .show(ui, |ui| {
            for (index, item) in items.iter().enumerate() {
                ui.horizontal(|ui| {
                    ui.set_height(ROW_H);
                    let label = format!("Remove {item}");
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if remove_x_button_with_id(ui, id.with(("x", index)), &label).clicked() {
                            outcome.removed = Some(index);
                        }
                        ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                            ui.add(
                                egui::Label::new(
                                    egui::RichText::new(item)
                                        .size(theme::ROW_SUMMARY_SIZE)
                                        .color(theme::VALUE_TEXT),
                                )
                                .truncate(),
                            )
                            .on_hover_text(item);
                        });
                    });
                });
            }
        });

    let (added, rejected) = add_field(ui, id, hint);
    outcome.added = added;
    outcome.rejected = rejected;
    outcome
}

/// The shared add field: draft in temp memory under `id`, Enter / + ADD commits, a quoted paste
/// is split, stray quotes and control characters are refused visibly with the draft kept.
fn add_field(ui: &mut egui::Ui, id: egui::Id, hint: &str) -> (Vec<String>, Option<String>) {
    let field_id = id.with("field");
    let mut added = Vec::new();
    let mut rejected = None;
    let mut draft: String = ui
        .data_mut(|d| d.get_temp::<String>(id.with("draft")))
        .unwrap_or_default();
    let mut commit = false;
    let mut enter = false;
    ui.horizontal(|ui| {
        let width = (ui.available_width() - 64.0).max(60.0);
        let response = ui.add(
            egui::TextEdit::singleline(&mut draft)
                .id(field_id)
                .hint_text(hint)
                .desired_width(width),
        );
        enter = response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
        if ui.button("+ ADD").clicked() {
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
    fn themed_slider_idle_frame_changes_nothing_and_never_clamps() {
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

    /// Focus the number field (click), then type `text`; returns (value, invalid, changed-on-type).
    fn type_into_number_field(start: i64, text: &str) -> (i64, bool, bool) {
        let ctx = headless_ctx();
        let mut value = start;
        let mut rect = egui::Rect::NOTHING;
        ctx.run_ui(RawInput::default(), |ui| {
            let f = number_field(ui, "n", &mut value, 80.0);
            assert!(!f.response.changed() && !f.invalid, "idle frame is quiet");
            rect = f.response.rect;
        })
        .drop_without_applying_deltas();
        ctx.run_ui(click_at(rect.center()), |ui| {
            number_field(ui, "n", &mut value, 80.0);
        })
        .drop_without_applying_deltas();
        ctx.run_ui(RawInput::default(), |ui| {
            number_field(ui, "n", &mut value, 80.0);
        })
        .drop_without_applying_deltas();
        let (mut invalid, mut changed) = (false, false);
        ctx.run_ui(text_input(text), |ui| {
            let f = number_field(ui, "n", &mut value, 80.0);
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
                    assert_eq!(colour_swatch(ui, rgba), None);
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
                assert_eq!(hsva_of(rgba).to_srgba_unmultiplied(), rgba, "{rgba:?}");
            }
        }
        let mut h = hsva_of([40, 42, 54, 0]);
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

    #[test]
    fn chip_font_is_clamped_to_what_fits() {
        assert_eq!(chip_font_size(None), 14.0);
        assert_eq!(chip_font_size(Some(5)), 12.0);
        assert_eq!(chip_font_size(Some(37)), 18.0);
        assert_eq!(chip_font_size(Some(15)), 15.0);
    }

    #[test]
    fn the_preview_chip_is_120_by_24_for_show_and_hide() {
        use crate::models::config::{Style, StyleAction};
        let ctx = egui::Context::default();
        ctx.set_fonts(egui::FontDefinitions::empty());
        let mut sizes = Vec::new();
        let out = ctx.run_ui(
            RawInput {
                screen_rect: Some(Rect::from_min_size(Pos2::ZERO, egui::vec2(400.0, 400.0))),
                ..Default::default()
            },
            |ui| {
                let mut show = Style::blank_show();
                show.text = Some([255, 121, 198]);
                show.background = Some([40, 42, 54]);
                let mut hide = Style::blank_show();
                hide.action = StyleAction::Hide;
                for s in [&show, &hide] {
                    sizes.push(style_preview_chip(ui, "x", s).rect.size());
                }
            },
        );
        out.drop_without_applying_deltas();
        assert!(sizes.iter().all(|s| *s == egui::vec2(120.0, 24.0)));
    }
}
