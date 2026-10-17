//! DPI-correct docking for a skinned window.
//!
//! Windows reports the work area (screen minus taskbar) in **physical** pixels, while egui's
//! `ViewportCommand::OuterPosition` / `InnerSize` take **logical** points. The `ff` overlay's
//! dock mixes the two (fine at 100% scaling, wrong at 125%+); this module converts with the
//! window's `native_pixels_per_point`.
//!
//! The overlay docks through `overlay::win::dock_left`, which finds the monitor the game is on
//! (the primary monitor when it is not running), reads that monitor's work area and DPI, and
//! places the window with `SetWindowPos` in physical pixels using [`dock_rect_px`] below.

use egui::{Rect, pos2, vec2};

/// Where a window `width` points wide should go to fill the left edge of `work_area_px`
/// (physical pixels) at full height, in logical points.
pub fn left_dock_rect(work_area_px: Rect, pixels_per_point: f32, width: f32) -> Rect {
    let ppp = if pixels_per_point > 0.0 {
        pixels_per_point
    } else {
        1.0
    };
    let min = pos2(work_area_px.min.x / ppp, work_area_px.min.y / ppp);
    let height = work_area_px.height() / ppp;
    let width = width.min(work_area_px.width() / ppp);
    Rect::from_min_size(min, vec2(width, height))
}

/// A physical-pixel window rectangle.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DockPx {
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
}

/// Flush-left, full-height dock for a window `width_pt` points wide at `scale` (DPI / 96) on
/// the monitor whose physical-pixel work area is `work_area_px`. A non-positive scale counts as
/// 1.0; the width never exceeds the work area.
pub fn dock_rect_px(work_area_px: Rect, scale: f32, width_pt: f32) -> DockPx {
    let scale = if scale > 0.0 { scale } else { 1.0 };
    let work_w = work_area_px.width().round() as i32;
    let w = ((width_pt * scale).round() as i32).min(work_w);
    DockPx {
        x: work_area_px.min.x.round() as i32,
        y: work_area_px.min.y.round() as i32,
        w,
        h: work_area_px.height().round() as i32,
    }
}

/// The primary monitor's work area in physical pixels, or `None` when Windows will not say (or
/// on other platforms).
#[cfg(windows)]
pub fn primary_work_area_px() -> Option<Rect> {
    use windows_sys::Win32::Foundation::POINT;
    use windows_sys::Win32::Graphics::Gdi::{
        GetMonitorInfoW, MONITOR_DEFAULTTOPRIMARY, MONITORINFO, MonitorFromPoint,
    };
    // SAFETY: plain value arguments; the primary default always yields a monitor.
    let m = unsafe { MonitorFromPoint(POINT { x: 0, y: 0 }, MONITOR_DEFAULTTOPRIMARY) };
    if m.is_null() {
        return None;
    }
    // SAFETY: a zeroed MONITORINFO is valid plain data; `cbSize` is set as the API requires.
    let mut info: MONITORINFO = unsafe { std::mem::zeroed() };
    info.cbSize = std::mem::size_of::<MONITORINFO>() as u32;
    // SAFETY: `info` is a live local of the documented type; the result is checked first.
    if unsafe { GetMonitorInfoW(m, &mut info) } == 0 {
        return None;
    }
    let r = info.rcWork;
    let area = Rect::from_min_max(
        pos2(r.left as f32, r.top as f32),
        pos2(r.right as f32, r.bottom as f32),
    );
    (area.width() > 1.0 && area.height() > 1.0).then_some(area)
}

/// Non-Windows stub.
#[cfg(not(windows))]
pub fn primary_work_area_px() -> Option<Rect> {
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn docks_left_full_height_in_logical_points() {
        // 1920x1080 at 150% with a 72px (physical) taskbar at the bottom.
        let work = Rect::from_min_max(pos2(0.0, 0.0), pos2(1920.0, 1008.0));
        let r = left_dock_rect(work, 1.5, 660.0);
        assert_eq!(r.min, pos2(0.0, 0.0));
        assert_eq!(r.width(), 660.0);
        assert_eq!(r.height(), 672.0);
    }

    #[test]
    fn respects_a_taskbar_on_the_left_and_never_exceeds_the_screen() {
        let work = Rect::from_min_max(pos2(96.0, 0.0), pos2(800.0, 600.0));
        let r = left_dock_rect(work, 2.0, 660.0);
        assert_eq!(r.min, pos2(48.0, 0.0));
        assert_eq!(r.width(), 352.0);
        assert_eq!(r.height(), 300.0);
    }

    fn wa(x0: f32, y0: f32, x1: f32, y1: f32) -> Rect {
        Rect::from_min_max(pos2(x0, y0), pos2(x1, y1))
    }

    #[test]
    fn dock_px_primary_100() {
        let d = dock_rect_px(wa(0.0, 0.0, 1920.0, 1040.0), 1.0, 660.0);
        assert_eq!(
            d,
            DockPx {
                x: 0,
                y: 0,
                w: 660,
                h: 1040
            }
        );
    }

    #[test]
    fn dock_px_second_monitor_150() {
        let d = dock_rect_px(wa(1920.0, 0.0, 3840.0, 1032.0), 1.5, 660.0);
        assert_eq!(
            d,
            DockPx {
                x: 1920,
                y: 0,
                w: 990,
                h: 1032
            }
        );
    }

    #[test]
    fn dock_px_negative_origin_200() {
        let d = dock_rect_px(wa(-2560.0, 0.0, 0.0, 1400.0), 2.0, 660.0);
        assert_eq!(
            d,
            DockPx {
                x: -2560,
                y: 0,
                w: 1320,
                h: 1400
            }
        );
    }

    #[test]
    fn dock_px_clamps_to_narrow_work_area() {
        let d = dock_rect_px(wa(0.0, 0.0, 500.0, 600.0), 1.0, 660.0);
        assert_eq!(d.w, 500);
    }

    #[test]
    fn dock_px_left_taskbar() {
        let d = dock_rect_px(wa(48.0, 0.0, 1920.0, 1080.0), 1.0, 660.0);
        assert_eq!(d.x, 48);
    }

    #[test]
    fn dock_px_bad_scale_is_one() {
        let d = dock_rect_px(wa(0.0, 0.0, 1920.0, 1080.0), 0.0, 660.0);
        assert_eq!(d.w, 660);
    }
}
