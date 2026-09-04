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
    use windows_sys::Win32::Foundation::HWND;
    use windows_sys::Win32::UI::WindowsAndMessaging::{ShowWindow, SW_HIDE, SW_SHOWNOACTIVATE};

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
}

pub use imp::{conceal, from_frame, reveal, OverlayWindow};
