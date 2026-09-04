//! Custom-painted chrome for the PoE2-idiom stripe: the header plate and zebra-striped rows.
//!
//! `egui::Visuals` alone cannot express a header plate (flat bronze fill, title, and a
//! hand-painted close control) or a two-tone alternating row background — `Visuals::striped`
//! is a simple on/off, not the designed two-color pair D-24 calls for (RESEARCH § "Code
//! Examples"). Both need direct `egui::Painter` calls, which is what this module is for. It is
//! the only place in the crate that paints the header's `✕` — as two crossed line segments, not
//! a Unicode glyph, since neither egui's default font nor the all-caps Cinzel face contains
//! U+2715 (the tofu box 13-03 found and deferred here).

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
    let (rect, _response) =
        ui.allocate_exact_size(egui::vec2(width, height), egui::Sense::hover());

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
    let close_response = ui.interact(close_rect, ui.id().with("header-close"), egui::Sense::click());

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
