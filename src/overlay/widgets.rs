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
        .inner_margin(egui::Margin::symmetric(10, 6))
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
    let size = 16.0;
    // Layout stays 16x16; the click target is padded to 24x24 (UI-SPEC carry-over).
    let auto_id = ui.next_auto_id();
    let (rect, _) = ui.allocate_exact_size(egui::vec2(size, size), egui::Sense::hover());
    let mut response = ui.interact(rect.expand(4.0), auto_id.with("hit"), egui::Sense::click());
    let state = *checked;
    response.widget_info(|| {
        egui::WidgetInfo::selected(egui::WidgetType::Checkbox, true, state, "Enabled")
    });

    if response.clicked() {
        *checked = !*checked;
        response.mark_changed();
    }

    let stroke_color = if response.hovered() {
        theme::SECTION_CREAM
    } else {
        theme::CONTROL_GLYPH
    };

    let painter = ui.painter();
    let center = rect.center();
    let radius = size / 2.0 - 1.0;
    painter.circle_stroke(center, radius, egui::Stroke::new(1.5, stroke_color));
    if *checked {
        painter.circle_filled(center, radius - 3.0, theme::CONTROL_GLYPH);
    }

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
    let size = 16.0;
    let auto_id = ui.next_auto_id();
    let (rect, hover) = ui.allocate_exact_size(egui::vec2(size, size), egui::Sense::hover());
    // Padded 24x24 click target; disabled arrows keep hover-only sensing.
    let response = if enabled {
        ui.interact(rect.expand(4.0), auto_id.with("hit"), egui::Sense::click())
    } else {
        hover
    };
    let label = match direction {
        ArrowDirection::Up => "Move up",
        ArrowDirection::Down => "Move down",
    };
    response.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, enabled, label));

    let color = if !enabled {
        theme::DISABLED_TEXT
    } else if response.hovered() {
        theme::SECTION_CREAM
    } else {
        theme::CONTROL_GLYPH
    };

    let pad = 4.0;
    let left = rect.left() + pad;
    let right = rect.right() - pad;
    let top = rect.top() + pad;
    let bottom = rect.bottom() - pad;
    let mid_x = rect.center().x;

    let points = match direction {
        ArrowDirection::Up => vec![
            egui::pos2(mid_x, top),
            egui::pos2(right, bottom),
            egui::pos2(left, bottom),
        ],
        ArrowDirection::Down => vec![
            egui::pos2(left, top),
            egui::pos2(right, top),
            egui::pos2(mid_x, bottom),
        ],
    };

    ui.painter().add(egui::Shape::convex_polygon(
        points,
        color,
        egui::Stroke::NONE,
    ));

    response
}

/// A 16px painted X inside a 24x24 click target, with a tooltip and accessible name `label`
/// (for example "Remove this condition"). Painted, not a glyph: Cinzel has no coverage.
pub fn remove_x_button(ui: &mut egui::Ui, label: &str) -> egui::Response {
    let id = ui.next_auto_id().with("hit");
    remove_x_button_with_id(ui, id, label)
}

/// Like [`remove_x_button`] but with an explicit widget id (used by lists that need to look a
/// specific control up again).
fn remove_x_button_with_id(ui: &mut egui::Ui, id: egui::Id, label: &str) -> egui::Response {
    let (rect, _) = ui.allocate_exact_size(egui::vec2(24.0, 24.0), egui::Sense::hover());
    let response = ui.interact(rect, id, egui::Sense::click());
    paint_x(ui, rect, response.hovered());
    response.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, true, label));
    response.on_hover_text(label)
}

fn paint_x(ui: &egui::Ui, rect: egui::Rect, hovered: bool) {
    let color = if hovered {
        theme::SECTION_CREAM
    } else {
        theme::LABEL_TAN
    };
    let stroke = egui::Stroke::new(1.5, color);
    let half = 4.0; // 8px cross inside the 16px paint box
    let c = rect.center();
    let painter = ui.painter();
    painter.line_segment(
        [c + egui::vec2(-half, -half), c + egui::vec2(half, half)],
        stroke,
    );
    painter.line_segment(
        [c + egui::vec2(half, -half), c + egui::vec2(-half, half)],
        stroke,
    );
}

/// A 24px-high selectable chip (rarity-style toggle). Flips `*selected` and marks the response
/// changed on click; callers write only on `.changed()`.
pub fn toggle_chip(ui: &mut egui::Ui, label: &str, selected: &mut bool) -> egui::Response {
    let text_color = if *selected {
        theme::SECTION_CREAM
    } else {
        theme::LABEL_TAN
    };
    let galley = ui.painter().layout_no_wrap(
        label.to_owned(),
        egui::FontId::proportional(theme::ROW_SUMMARY_SIZE),
        text_color,
    );
    let size = egui::vec2(galley.size().x + 16.0, 24.0);
    let (rect, mut response) = ui.allocate_exact_size(size, egui::Sense::click());
    if response.clicked() {
        *selected = !*selected;
        response.mark_changed();
    }
    let fill = if *selected {
        theme::HEADER_PLATE
    } else {
        theme::INPUT_FILL
    };
    let stroke_color = if response.hovered() {
        theme::SECTION_CREAM
    } else {
        theme::CONTROL_GLYPH
    };
    let painter = ui.painter();
    painter.rect(
        rect,
        2.0,
        fill,
        egui::Stroke::new(1.0, stroke_color),
        egui::StrokeKind::Inside,
    );
    painter.galley(
        egui::pos2(rect.left() + 8.0, rect.center().y - galley.size().y / 2.0),
        galley,
        text_color,
    );
    let state = *selected;
    response
        .widget_info(|| egui::WidgetInfo::selected(egui::WidgetType::Button, true, state, label));
    response
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
        ctx.set_fonts(egui::FontDefinitions::empty()); // no font atlas needed for this smoke test
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
    fn circular_toggle_target_is_padded_but_layout_stays_16px() {
        let ctx = headless_ctx();
        let mut checked = false;
        let mut rect = egui::Rect::NOTHING;
        ctx.run_ui(RawInput::default(), |ui| {
            let before = ui.cursor().min;
            let r = circular_toggle(ui, &mut checked);
            let after = ui.min_rect();
            assert!(after.width() <= 16.0 + 0.01, "layout width must stay 16");
            rect = egui::Rect::from_min_size(before, egui::vec2(16.0, 16.0));
            drop(r);
        })
        .drop_without_applying_deltas();
        // 3px outside the painted circle edge flips it.
        let pos = rect.right_center() + egui::vec2(3.0, 0.0);
        ctx.run_ui(click_at(pos), |ui| {
            circular_toggle(ui, &mut checked);
        })
        .drop_without_applying_deltas();
        assert!(checked, "click inside the padded target must flip");
        // Well outside (past the padded target and egui interact radius) does not.
        let pos = rect.right_center() + egui::vec2(14.0, 0.0);
        ctx.run_ui(click_at(pos), |ui| {
            circular_toggle(ui, &mut checked);
        })
        .drop_without_applying_deltas();
        assert!(checked, "click outside the padded target must not flip");
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
}
