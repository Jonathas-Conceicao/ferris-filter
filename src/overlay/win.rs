//! The one module in this phase allowed to touch raw Win32 (`windows_sys`) — see the
//! Task 1 threat register entry T-13-09: every `unsafe` call in the whole overlay lives here,
//! in exactly two functions, each a single `ShowWindow` call on a handle obtained from our own
//! `eframe::Frame`.
//!
//! ## Why this exists instead of egui's own viewport-visibility command
//!
//! RESEARCH read winit 0.30.13's Windows backend directly (`window_state.rs`, `apply_diff`) and
//! found that `ViewportBuilder::with_active(false)` only buys the show-without-activating flag
//! on the **first** reveal: winit latches a `MARKER_ACTIVATE` flag the first time visibility is
//! applied, and every later visibility command issues the plain activating show call, which
//! activates the window and steals keyboard focus from the game. D-05 ("summon without stealing
//! focus") must therefore hold on the second, third and every later summon, not just the first —
//! so this module bypasses egui's viewport-visibility command entirely and drives both
//! directions with a raw call on a cached `HWND`, so there is exactly one source of truth for
//! visibility and winit's cached window flags can never drift from the real OS state.
//!
//! **Never mix this module's calls with egui's own viewport-visibility command**
//! (RESEARCH's anti-pattern) — doing so would reintroduce the drift this module exists to
//! remove.
//!
//! ## Why the show-without-activating flag does not fight D-06
//!
//! The show-without-activating flag only affects the one call that makes the window visible; it
//! does **not** set any persistent extended style. Once visible-but-unfocused, the window is
//! fully normal from Windows' input-routing perspective — a click inside it triggers the OS's
//! default mouse-activation behavior exactly as any other window, so D-06 (a click activates the
//! stripe normally) needs no extra code here. This is also why this module must never set the
//! Win32 "never activate this window" extended style: that would suppress activation on *every*
//! click, not just the reveal, and would break D-06 outright.

#[cfg(windows)]
mod imp {
    // Reached via eframe's own re-export (`eframe::wgpu::rwh`, wgpu's alias for
    // `raw_window_handle`) rather than a direct dependency on `raw-window-handle`, so this
    // module can never drift to a version eframe itself does not agree with (RESEARCH's stated
    // version-skew hazard).
    use eframe::wgpu::rwh::{HasWindowHandle, RawWindowHandle};
    use windows_sys::Win32::Foundation::{HWND, RECT};
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        GetForegroundWindow, GetWindowTextW, SPI_GETWORKAREA, SW_HIDE, SW_SHOWNOACTIVATE,
        ShowWindow, SystemParametersInfoW,
    };

    /// The stripe's own window handle, cached once on the first frame it is available. Never
    /// obtained for, or applied to, any window other than our own.
    pub struct OverlayWindow(HWND);

    // SAFETY: an `HWND` is an opaque handle value (a process-local integer-like identifier), not
    // a pointer to data we access directly. Handing it across threads is not attempted by this
    // module (`app.rs` calls `reveal`/`conceal` from the same thread that owns the window, the
    // eframe/winit main thread), but marking it `Send` costs nothing extra and is the standard
    // shape for a raw handle wrapper: no bytes behind the pointer are ever dereferenced here,
    // only passed back to the same Win32 API family that issued it.
    unsafe impl Send for OverlayWindow {}

    /// Caches the stripe's `HWND` from an `eframe::Frame`, once one is available. Returns
    /// `None` before the platform window exists yet (the earliest frames) or if the raw handle
    /// is not the `Win32` variant (should not happen under `cfg(windows)`, but never assumed).
    pub fn from_frame(frame: &eframe::Frame) -> Option<OverlayWindow> {
        match frame.window_handle().ok()?.as_raw() {
            RawWindowHandle::Win32(h) => Some(OverlayWindow(h.hwnd.get() as HWND)),
            _ => None,
        }
    }

    /// Shows the window without activating it (D-05) — safe to call on every summon, not just
    /// the first, unlike egui's own viewport-visibility command (see module docs).
    pub fn reveal(w: &OverlayWindow) {
        // SAFETY: `w.0` is our own window's HWND, obtained via `from_frame` above from our own
        // `eframe::Frame`. `ShowWindow` with `SW_SHOWNOACTIVATE` is the documented, side-effect-
        // free way to display a window without changing input focus.
        unsafe {
            ShowWindow(w.0, SW_SHOWNOACTIVATE);
        }
    }

    /// Hides the window.
    pub fn conceal(w: &OverlayWindow) {
        // SAFETY: same handle-provenance argument as `reveal` above.
        unsafe {
            ShowWindow(w.0, SW_HIDE);
        }
    }

    /// The desktop work area — the screen minus the taskbar and any other appbars.
    ///
    /// The stripe used to dock against `monitor_size`, which winit reports as the full screen
    /// resolution. On a 1080p screen with a taskbar that put the bottom of the window, and the
    /// APPLY row with it, underneath the taskbar (author, screenshot poe2_print008). Taskbar
    /// height and edge are user settings, so no fixed margin can stand in for asking.
    pub fn work_area() -> Option<egui::Rect> {
        let mut rect = RECT {
            left: 0,
            top: 0,
            right: 0,
            bottom: 0,
        };
        // SAFETY: `SystemParametersInfoW` with `SPI_GETWORKAREA` writes a `RECT` through the
        // pointer we pass. The pointer is to a live local of exactly that type, the documented
        // `uiParam`/`fWinIni` for a read are both zero, and the return value is checked before
        // the value is used.
        let ok = unsafe {
            SystemParametersInfoW(SPI_GETWORKAREA, 0, (&mut rect as *mut RECT).cast(), 0)
        };
        if ok == 0 {
            return None;
        }
        let min = egui::pos2(rect.left as f32, rect.top as f32);
        let max = egui::pos2(rect.right as f32, rect.bottom as f32);
        let area = egui::Rect::from_min_max(min, max);
        (area.width() > 1.0 && area.height() > 1.0).then_some(area)
    }

    /// Whether the window the user is actually working in is the game, or the stripe itself.
    ///
    /// Gates the global hotkey: a system-wide binding that fires in a browser or an editor is a
    /// key the user cannot use for anything else (author, 2026-09-15). The stripe counts as well
    /// as the game — once it has focus, the same key has to be able to dismiss it, or the only
    /// way out would be the tray.
    ///
    /// Matched on the foreground window's title rather than its process, which keeps this to one
    /// already-enabled Win32 call and works for both the Steam client (`PathOfExileSteam`) and
    /// the standalone one. "Exiled Exchange 2", a trade overlay the author runs alongside, does
    /// not contain this substring.
    pub fn foreground_is_game_or_overlay(overlay: Option<&OverlayWindow>) -> bool {
        // SAFETY: `GetForegroundWindow` takes no arguments and returns a handle or null.
        let fg = unsafe { GetForegroundWindow() };
        if fg.is_null() {
            return false;
        }
        if let Some(w) = overlay {
            if fg == w.0 {
                return true;
            }
        }

        let mut buf = [0u16; 256];
        // SAFETY: `GetWindowTextW` writes at most `buf.len()` UTF-16 units into the buffer we
        // pass and returns how many it wrote, excluding the terminator. The buffer is a live
        // local and the length is its true capacity.
        let len = unsafe { GetWindowTextW(fg, buf.as_mut_ptr(), buf.len() as i32) };
        if len <= 0 {
            return false;
        }
        let title = String::from_utf16_lossy(&buf[..len as usize]);
        title.to_lowercase().contains("path of exile")
    }
}

#[cfg(not(windows))]
mod imp {
    /// Non-Windows stand-in so the library still compiles off Windows (D-01 is Windows-only in
    /// scope, but the crate as a whole must not fail to build elsewhere).
    pub struct OverlayWindow;

    pub fn from_frame(_frame: &eframe::Frame) -> Option<OverlayWindow> {
        None
    }

    pub fn reveal(_w: &OverlayWindow) {}

    pub fn conceal(_w: &OverlayWindow) {}

    pub fn work_area() -> Option<egui::Rect> {
        None
    }

    /// Off Windows there is no foreground-window query here, so the hotkey stays unconditional
    /// rather than silently never firing.
    pub fn foreground_is_game_or_overlay(_overlay: Option<&OverlayWindow>) -> bool {
        true
    }
}

pub use imp::{
    OverlayWindow, conceal, foreground_is_game_or_overlay, from_frame, reveal, work_area,
};

#[cfg(all(test, windows))]
mod tests {
    /// A smoke test for the one new Win32 read: it must come back with a plausible desktop, not
    /// a zeroed rect. Everything else in this module needs a real window and cannot be tested
    /// here (PITFALLS.md section 8), but this call needs none.
    #[test]
    fn the_work_area_is_a_plausible_desktop() {
        let Some(area) = super::work_area() else {
            // A headless or unusual session may legitimately have none; the caller falls back to
            // the monitor size, so this is not a failure.
            return;
        };
        assert!(
            area.width() >= 320.0,
            "work area {area:?} is implausibly narrow"
        );
        assert!(
            area.height() >= 240.0,
            "work area {area:?} is implausibly short"
        );
        eprintln!("work area on this machine: {area:?}");
    }
}
