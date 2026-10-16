//! Small painted flag icons for rule rows: vector shapes, no textures, tinted by the caller.
//!
//! [`paint_flag_icon`] draws into any rect (16pt glyph, 1.75pt strokes); [`flag_icon`] is the
//! 18x18 hover-only widget with an accessible name and a tooltip.

use egui::{
    Align2, Color32, Painter, Pos2, Rect, Response, Sense, Shape, Stroke, Ui, Vec2, WidgetInfo,
    WidgetType, pos2, vec2,
};

use super::fonts::regular;
use super::palette as pal;

/// Side of the square a [`flag_icon`] occupies.
pub const FLAG_ICON_SIZE: f32 = 18.0;

/// Stroke width of every glyph.
const STROKE: f32 = 1.75;

/// The flags a rule row can carry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FlagIcon {
    /// Locked rule (economy).
    Padlock,
    /// The rule names a style that does not exist.
    UnknownStyle,
    /// A grammar error (blocks Apply).
    GrammarError,
    /// A grammar warning (does not block).
    GrammarWarning,
    /// A defence group whose blocks no longer match.
    GroupBroken,
    /// The rule was moved across a Continue rule.
    MovedAcrossContinue,
    /// Section collapsed (click to expand).
    ChevronRight,
    /// Section expanded (click to collapse).
    ChevronDown,
    /// Rename a section.
    Rename,
}

/// The two chain links of the group-broken glyph: separated by a visible gap.
pub fn group_broken_links(rect: Rect) -> (Rect, Rect) {
    let c = rect.center();
    let r = rect.width().min(rect.height()).min(16.0) / 2.0;
    // Wide, short links (a chain seen from the side), not tall boxes: tall ones with a slash
    // between them read as "0/0".
    let size = vec2(r * 0.95, r * 0.8);
    (
        Rect::from_center_size(pos2(c.x - r * 0.6, c.y), size),
        Rect::from_center_size(pos2(c.x + r * 0.6, c.y), size),
    )
}

/// Paints `icon` centred in `rect` in `colour`.
pub fn paint_flag_icon(painter: &Painter, rect: Rect, icon: FlagIcon, colour: Color32) {
    let c = rect.center();
    let r = rect.width().min(rect.height()).min(16.0) / 2.0;
    let stroke = Stroke::new(STROKE, colour);
    match icon {
        FlagIcon::Padlock => {
            let body = Rect::from_min_max(
                pos2(c.x - r * 0.75, c.y - r * 0.05),
                pos2(c.x + r * 0.75, c.y + r),
            );
            painter.rect_filled(
                body,
                1.0,
                Color32::from_rgba_unmultiplied(pal::WARN.r(), pal::WARN.g(), pal::WARN.b(), 90),
            );
            painter.rect_stroke(body, 1.0, stroke, egui::StrokeKind::Inside);
            let centre = pos2(c.x, c.y - r * 0.05);
            let sr = r * 0.5;
            let mut pts: Vec<Pos2> = (0..=6)
                .map(|i| {
                    // Stops short of the half circle: the right leg is left open.
                    let a = std::f32::consts::PI * 0.8 * (i as f32 / 6.0);
                    pos2(centre.x - a.cos() * sr, centre.y - a.sin() * sr)
                })
                .collect();
            pts.insert(0, pos2(centre.x - sr, centre.y));
            painter.add(Shape::line(pts, stroke));
        }
        FlagIcon::UnknownStyle => {
            painter.circle_stroke(c, r * 0.9, stroke);
            painter.text(c, Align2::CENTER_CENTER, "?", regular(13.0), colour);
        }
        FlagIcon::GrammarError => {
            painter.circle_filled(c, r * 0.9, colour);
            painter.text(c, Align2::CENTER_CENTER, "!", regular(13.0), pal::HEADING);
        }
        FlagIcon::GrammarWarning => {
            let tri = vec![
                pos2(c.x, c.y - r * 0.9),
                pos2(c.x + r, c.y + r * 0.8),
                pos2(c.x - r, c.y + r * 0.8),
            ];
            painter.add(Shape::closed_line(tri, stroke));
            painter.text(
                pos2(c.x, c.y + r * 0.3),
                Align2::CENTER_CENTER,
                "!",
                regular(12.0),
                colour,
            );
        }
        FlagIcon::GroupBroken => {
            let (a, b) = group_broken_links(rect);
            let rad = (r * 0.45) as u8;
            painter.rect_stroke(a, rad, stroke, egui::StrokeKind::Inside);
            painter.rect_stroke(b, rad, stroke, egui::StrokeKind::Inside);
            // The break: two short sparks in the gap between the links.
            for dy in [-1.0_f32, 1.0] {
                painter.line_segment(
                    [
                        pos2(c.x, c.y + dy * r * 0.25),
                        pos2(c.x, c.y + dy * r * 0.7),
                    ],
                    Stroke::new(1.25, colour),
                );
            }
        }
        FlagIcon::MovedAcrossContinue => {
            let up = pos2(c.x - r * 0.45, c.y);
            let down = pos2(c.x + r * 0.45, c.y);
            for (x, dir) in [(up.x, -1.0_f32), (down.x, 1.0)] {
                let tip = pos2(x, c.y + dir * r * 0.9);
                let tail = pos2(x, c.y - dir * r * 0.9);
                painter.line_segment([tail, tip], stroke);
                painter.line_segment([tip, pos2(x - r * 0.4, tip.y - dir * r * 0.5)], stroke);
                painter.line_segment([tip, pos2(x + r * 0.4, tip.y - dir * r * 0.5)], stroke);
            }
            painter.line_segment(
                [pos2(c.x - r, c.y), pos2(c.x + r, c.y)],
                Stroke::new(1.0, colour.gamma_multiply(0.6)),
            );
        }
        FlagIcon::ChevronRight => {
            let k = r * 0.6;
            let pts = vec![
                pos2(c.x - k * 0.5, c.y - k),
                pos2(c.x + k * 0.5, c.y),
                pos2(c.x - k * 0.5, c.y + k),
            ];
            painter.add(Shape::line(pts, stroke));
        }
        FlagIcon::ChevronDown => {
            let k = r * 0.6;
            let pts = vec![
                pos2(c.x - k, c.y - k * 0.5),
                pos2(c.x, c.y + k * 0.5),
                pos2(c.x + k, c.y - k * 0.5),
            ];
            painter.add(Shape::line(pts, stroke));
        }
        FlagIcon::Rename => {
            // Pencil: a diagonal body from the lower left to the upper right, tip at the lower left.
            let tip = pos2(c.x - r * 0.8, c.y + r * 0.8);
            let top = pos2(c.x + r * 0.75, c.y - r * 0.75);
            let d = (top - tip).normalized();
            let n = vec2(-d.y, d.x) * r * 0.22;
            let neck = tip + d * r * 0.45;
            painter.add(Shape::closed_line(vec![tip, neck + n, neck - n], stroke));
            painter.line_segment([neck + n, top + n], stroke);
            painter.line_segment([neck - n, top - n], stroke);
            painter.line_segment([top + n, top - n], stroke);
        }
    }
}

/// A hover-only 18x18 flag icon with an accessible `name` and a `tip` tooltip.
pub fn flag_icon(ui: &mut Ui, icon: FlagIcon, colour: Color32, name: &str, tip: &str) -> Response {
    let (rect, resp) = ui.allocate_exact_size(Vec2::splat(FLAG_ICON_SIZE), Sense::hover());
    resp.widget_info(|| WidgetInfo::labeled(WidgetType::Label, true, name));
    paint_flag_icon(ui.painter(), rect.shrink(1.0), icon, colour);
    resp.on_hover_text(tip)
}

#[cfg(test)]
mod tests {
    use super::*;

    const ALL: [FlagIcon; 9] = [
        FlagIcon::ChevronRight,
        FlagIcon::ChevronDown,
        FlagIcon::Rename,
        FlagIcon::Padlock,
        FlagIcon::UnknownStyle,
        FlagIcon::GrammarError,
        FlagIcon::GrammarWarning,
        FlagIcon::GroupBroken,
        FlagIcon::MovedAcrossContinue,
    ];

    fn shape_count(f: impl FnOnce(&mut Ui)) -> usize {
        let mut f = Some(f);
        let ctx = egui::Context::default();
        super::super::fonts::install(&ctx);
        let mut n = 0;
        let out = ctx.run_ui(egui::RawInput::default(), |ui| {
            if let Some(f) = f.take() {
                f(ui);
            }
        });
        n += out.shapes.len();
        out.drop_without_applying_deltas();
        n
    }

    #[test]
    fn every_icon_paints_something() {
        let base = shape_count(|_| {});
        for icon in ALL {
            let n = shape_count(|ui| {
                let rect = Rect::from_min_size(pos2(10.0, 10.0), Vec2::splat(14.0));
                paint_flag_icon(ui.painter(), rect, icon, Color32::WHITE);
            });
            assert!(n > base, "{icon:?} painted nothing");
        }
    }

    #[test]
    fn group_broken_links_have_a_gap() {
        let rect = Rect::from_min_size(pos2(10.0, 10.0), Vec2::splat(16.0));
        let (a, b) = group_broken_links(rect);
        assert!(!a.intersects(b));
        assert!(b.min.x - a.max.x >= 2.0, "gap {}", b.min.x - a.max.x);
    }

    #[test]
    fn flag_icon_is_18_square_and_accessible() {
        let ctx = egui::Context::default();
        super::super::fonts::install(&ctx);
        ctx.enable_accesskit();
        let mut size = Vec2::ZERO;
        let out = ctx.run_ui(egui::RawInput::default(), |ui| {
            size = flag_icon(ui, FlagIcon::Padlock, Color32::WHITE, "Locked", "tip")
                .rect
                .size();
        });
        assert_eq!(size, Vec2::splat(18.0));
        let update = out
            .platform_output
            .accesskit_update
            .as_ref()
            .expect("update");
        let found = update
            .nodes
            .iter()
            .any(|(_, n)| n.label() == Some("Locked") || n.value() == Some("Locked"));
        assert!(found, "no accessibility node labelled Locked");
        out.drop_without_applying_deltas();
    }
}
