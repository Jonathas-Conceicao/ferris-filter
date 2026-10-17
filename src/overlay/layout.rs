//! Pure window-docking geometry — no eframe/window state, just numbers in and a
//! rect out, so it is fully unit-testable without a running event loop.
//!
//! `dock_rect` guarantees, as an invariant rather than a coincidence, that the
//! returned rect never equals or exceeds the monitor size in either dimension
//! (D-12, egui #4458: `always_on_top` renders incorrectly when the window size
//! equals the screen size).

/// The overlay's nominal width in points, measured from the EE2 reference
/// screenshot (D-23: 460 px of a 1920×1080 screen).
pub const STRIPE_WIDTH: f32 = 460.0;

/// Clearance kept between the window and the screen edges it doesn't dock
/// against (right, top, bottom), and the minimum slack subtracted from the
/// docked edge so the rect can never equal the monitor size outright.
pub const EDGE_MARGIN: f32 = 12.0;

/// Returns the overlay's outer rect, docked against the right screen edge with
/// `EDGE_MARGIN` clearance on the right, top and bottom.
///
/// Clamps rather than trusting its inputs: for any `monitor` with both
/// dimensions greater than zero, the returned rect's width is strictly less
/// than `monitor.x` and its height is strictly less than `monitor.y`, and the
/// whole rect stays inside `[0, monitor]` — a tiny monitor or an oversized
/// requested `width` still yields a sane, sub-screen rect rather than
/// producing a rect equal to (or larger than) the monitor.
/// Docks inside an arbitrary usable rect — the desktop **work area**, which excludes the
/// taskbar — rather than assuming the usable region starts at the origin and spans the whole
/// screen.
///
/// [`dock_rect`] is this with the work area assumed to be the full monitor starting at 0,0, which
/// is what winit's `monitor_size` describes. That assumption put the bottom of the window, and
/// the APPLY row with it, behind the taskbar (author, screenshot poe2_print008, 2026-09-15).
pub fn dock_rect_in(work_area: egui::Rect, width: f32) -> egui::Rect {
    let local = dock_rect(work_area.size(), width);
    local.translate(work_area.min.to_vec2())
}

pub fn dock_rect(monitor: egui::Vec2, width: f32) -> egui::Rect {
    // Guard against a degenerate monitor size (zero or negative) so every
    // computation below has a size to work with.
    let monitor_x = monitor.x.max(2.0);
    let monitor_y = monitor.y.max(2.0);

    // The margin itself must never be large enough to consume the whole
    // monitor on a very small screen, so cap it relative to the monitor size
    // before using it.
    let margin = EDGE_MARGIN.clamp(0.0, (monitor_x.min(monitor_y) / 2.0 - 1.0).max(0.0));

    // Width: the smaller of the requested width and what fits with margin
    // clearance on both docked sides, then a final hard clamp so the result
    // is unconditionally strictly less than monitor_x regardless of margin.
    let width_with_margin = (monitor_x - 2.0 * margin).max(1.0);
    let w = width.max(1.0).min(width_with_margin).min(monitor_x - 1.0);

    // Height: full available height minus top/bottom margin, same final
    // hard clamp for the strictly-less-than invariant.
    let height_with_margin = (monitor_y - 2.0 * margin).max(1.0);
    let h = height_with_margin.min(monitor_y - 1.0);

    let x_min = monitor_x - margin - w;
    let y_min = margin;

    egui::Rect::from_min_size(egui::pos2(x_min, y_min), egui::vec2(w, h))
}

#[cfg(test)]
mod work_area_tests {
    use super::*;

    #[test]
    fn docking_into_a_work_area_keeps_the_stripe_clear_of_the_taskbar() {
        // 1080p with a 48px taskbar along the bottom: the work area is shorter than the screen
        // and the window must end inside it, not at the screen's own bottom edge.
        let screen = egui::vec2(1920.0, 1080.0);
        let work = egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1920.0, 1032.0));

        let docked = dock_rect_in(work, STRIPE_WIDTH);
        assert!(
            docked.max.y <= work.max.y,
            "stripe bottom {} runs past the work area {}",
            docked.max.y,
            work.max.y
        );

        // And the old whole-screen assumption is what put it under the taskbar — without this,
        // the assertion above could pass for a reason unrelated to the fix.
        let naive = dock_rect(screen, STRIPE_WIDTH);
        assert!(
            naive.max.y > work.max.y,
            "expected the screen-sized dock to overrun the work area, but it did not"
        );
    }

    #[test]
    fn a_taskbar_on_the_left_shifts_the_stripe_rather_than_only_shrinking_it() {
        // The work area need not start at the origin; a left or top appbar offsets it.
        let work = egui::Rect::from_min_max(egui::pos2(80.0, 0.0), egui::pos2(1920.0, 1080.0));
        let docked = dock_rect_in(work, STRIPE_WIDTH);
        assert!(
            docked.min.x >= work.min.x,
            "stripe starts left of the work area"
        );
        assert!(
            docked.max.x <= work.max.x,
            "stripe runs past the work area's right edge"
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stripe_is_never_screen_sized_on_a_1920x1080_monitor() {
        let monitor = egui::Vec2::new(1920.0, 1080.0);
        let rect = dock_rect(monitor, STRIPE_WIDTH);

        assert!(
            rect.width() < monitor.x,
            "width must be strictly under the monitor width"
        );
        assert!(
            rect.height() < monitor.y,
            "height must be strictly under the monitor height"
        );
        assert_eq!(
            rect.width(),
            STRIPE_WIDTH,
            "460px fits comfortably, no clamping expected"
        );
        assert_eq!(
            rect.right(),
            monitor.x - EDGE_MARGIN,
            "flush to the right edge minus margin"
        );
    }

    #[test]
    fn small_monitor_still_yields_a_rect_strictly_inside_it() {
        let monitor = egui::Vec2::new(400.0, 300.0);
        let rect = dock_rect(monitor, STRIPE_WIDTH);
        let monitor_rect = egui::Rect::from_min_size(egui::Pos2::ZERO, monitor);

        assert!(
            monitor_rect.contains_rect(rect),
            "rect must stay fully inside the monitor"
        );
        assert!(rect.width() < monitor.x);
        assert!(rect.height() < monitor.y);
    }

    #[test]
    fn a_requested_width_larger_than_the_monitor_is_clamped() {
        let monitor = egui::Vec2::new(400.0, 300.0);
        let rect = dock_rect(monitor, 5000.0);

        assert!(rect.width() < monitor.x);
        assert!(rect.width() > 0.0);
    }
}
