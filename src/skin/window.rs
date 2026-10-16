//! DPI-correct docking for a skinned window.
//!
//! Windows reports the work area (screen minus taskbar) in **physical** pixels, while egui's
//! `ViewportCommand::OuterPosition` / `InnerSize` take **logical** points. The `ff` overlay's
//! dock mixes the two (fine at 100% scaling, wrong at 125%+); this module converts with the
//! window's `native_pixels_per_point`.
//!
//! The work area is the primary monitor's (`SPI_GETWORKAREA`): a new window opens on the
//! primary monitor, so that is the monitor "under the window" on the first frame, and it needs
//! no Win32 feature beyond the ones the crate already enables.

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

/// The primary monitor's work area in physical pixels, or `None` when Windows will not say (or
/// on other platforms).
#[cfg(windows)]
pub fn primary_work_area_px() -> Option<Rect> {
    use windows_sys::Win32::Foundation::RECT;
    use windows_sys::Win32::UI::WindowsAndMessaging::{SPI_GETWORKAREA, SystemParametersInfoW};
    let mut rect = RECT {
        left: 0,
        top: 0,
        right: 0,
        bottom: 0,
    };
    // SAFETY: SPI_GETWORKAREA writes one RECT through the pointer, which points at a live local
    // of exactly that type; the return value is checked before the RECT is read.
    let ok =
        unsafe { SystemParametersInfoW(SPI_GETWORKAREA, 0, (&mut rect as *mut RECT).cast(), 0) };
    if ok == 0 {
        return None;
    }
    let area = Rect::from_min_max(
        pos2(rect.left as f32, rect.top as f32),
        pos2(rect.right as f32, rect.bottom as f32),
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
}
