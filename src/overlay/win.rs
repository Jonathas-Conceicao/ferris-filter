//! The overlay's home for raw Win32 (`windows_sys`) — see the threat register entry T-13-09.
//! Originally two `ShowWindow` functions; it now also holds the RELOAD-01 (Phase 23) game
//! lookup, focus, `SendInput` and clipboard wrappers. Together with `instance.rs` it is still the
//! only overlay code allowed raw Win32, and every `unsafe` block carries a `// SAFETY:` comment.
//! Note that the reload deliberately moves focus to the game, unlike the D-05 reveal, because
//! that is where the user wants to be after Apply (CONTEXT "Established Patterns").
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

/// Lowercase title fragment that marks the game window (CONTEXT "Trigger"). "Exiled Exchange 2",
/// the trade overlay the author runs alongside, does not contain it.
pub const GAME_TITLE_NEEDLE: &str = "path of exile";

/// Lowercase image-name prefix of the game executable: covers `PathOfExile.exe`,
/// `PathOfExile_x64Steam.exe` and `PathOfExileSteam.exe`.
pub const GAME_EXE_PREFIX: &str = "pathofexile";

/// Whether a window title looks like the game's. Title alone cannot exclude a browser tab
/// ("Path of Exile 2 - Google Chrome"); the reload lookup adds [`is_game_exe`].
pub fn is_game_title(title: &str) -> bool {
    title.to_lowercase().contains(GAME_TITLE_NEEDLE)
}

/// Whether an executable path names the game: the file name (text after the last `\` or `/`),
/// not any folder, starts with [`GAME_EXE_PREFIX`].
pub fn is_game_exe(image_path: &str) -> bool {
    let name = image_path.rsplit(['\\', '/']).next().unwrap_or(image_path);
    name.to_lowercase().starts_with(GAME_EXE_PREFIX)
}

/// The reload target rule (orchestrator resolution for the browser-tab false match): title AND
/// process image name. A process that could not be queried (`None`) is not the game.
pub fn is_game_window(title: &str, image_path: Option<&str>) -> bool {
    is_game_title(title) && image_path.is_some_and(is_game_exe)
}

#[cfg(windows)]
mod imp {
    // Reached via eframe's own re-export (`eframe::wgpu::rwh`, wgpu's alias for
    // `raw_window_handle`) rather than a direct dependency on `raw-window-handle`, so this
    // module can never drift to a version eframe itself does not agree with (RESEARCH's stated
    // version-skew hazard).
    use eframe::wgpu::rwh::{HasWindowHandle, RawWindowHandle};
    use windows_sys::Win32::Foundation::{CloseHandle, HWND, LPARAM, RECT};
    use windows_sys::Win32::System::Threading::{
        GetCurrentProcessId, OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION,
        QueryFullProcessImageNameW,
    };
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        EnumWindows, GetForegroundWindow, GetWindowTextW, GetWindowThreadProcessId, IsIconic,
        IsWindow, IsWindowVisible, SPI_GETWORKAREA, SW_HIDE, SW_RESTORE, SW_SHOWNOACTIVATE,
        SetForegroundWindow, ShowWindow, SystemParametersInfoW,
    };
    use windows_sys::core::BOOL;

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
        super::is_game_title(&title)
    }

    /// The running game's window. Wraps an opaque `HWND`.
    pub struct GameWindow(HWND);

    // SAFETY: same argument as `OverlayWindow`: an opaque handle value moved to the reload worker
    // thread and only ever passed back to user32; no bytes behind it are dereferenced.
    unsafe impl Send for GameWindow {}

    struct FindCtx {
        own_pid: u32,
        found: HWND,
    }

    /// The executable path of process `pid`, or `None` when it cannot be queried.
    fn process_image_path(pid: u32) -> Option<String> {
        // SAFETY: `OpenProcess` with the limited-query right takes plain values and returns a
        // handle or null; null is handled below and a non-null handle is always closed.
        let handle = unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid) };
        if handle.is_null() {
            return None;
        }
        let mut buf = [0u16; 1024];
        let mut len = buf.len() as u32;
        // SAFETY: `QueryFullProcessImageNameW` writes at most `len` units into `buf` and updates
        // `len` to the count written; both are live locals with the true capacity.
        let ok = unsafe { QueryFullProcessImageNameW(handle, 0, buf.as_mut_ptr(), &mut len) };
        // SAFETY: `handle` was returned non-null by `OpenProcess` above and is closed once.
        unsafe {
            CloseHandle(handle);
        }
        if ok == 0 {
            return None;
        }
        Some(String::from_utf16_lossy(
            &buf[..(len as usize).min(buf.len())],
        ))
    }

    unsafe extern "system" fn find_callback(hwnd: HWND, lparam: LPARAM) -> BOOL {
        // SAFETY: `lparam` is the pointer to the `FindCtx` that `find_game_window` keeps alive on
        // its stack for the whole `EnumWindows` call, and nothing else aliases it.
        let ctx = unsafe { &mut *(lparam as *mut FindCtx) };
        // SAFETY: `IsWindowVisible` takes only a handle handed to us by `EnumWindows`.
        if unsafe { IsWindowVisible(hwnd) } == 0 {
            return 1;
        }
        let mut buf = [0u16; 256];
        // SAFETY: writes at most `buf.len()` units into a live local of that capacity.
        let len = unsafe { GetWindowTextW(hwnd, buf.as_mut_ptr(), buf.len() as i32) };
        if len <= 0 {
            return 1;
        }
        let title = String::from_utf16_lossy(&buf[..len as usize]);
        if !super::is_game_title(&title) {
            return 1;
        }
        let mut pid = 0u32;
        // SAFETY: writes one `u32` through a pointer to a live local.
        unsafe {
            GetWindowThreadProcessId(hwnd, &mut pid);
        }
        if pid == 0 || pid == ctx.own_pid {
            return 1;
        }
        let image = process_image_path(pid);
        if super::is_game_window(&title, image.as_deref()) {
            ctx.found = hwnd;
            return 0;
        }
        1
    }

    /// Finds the game window whether or not it is foreground: visible top-level window whose
    /// title contains "path of exile" and whose process image name starts with "pathofexile".
    /// ff's own windows are skipped and an unqueryable process is not the game.
    pub fn find_game_window() -> Option<GameWindow> {
        // SAFETY: `GetCurrentProcessId` takes no arguments.
        let own_pid = unsafe { GetCurrentProcessId() };
        let mut ctx = FindCtx {
            own_pid,
            found: std::ptr::null_mut(),
        };
        // SAFETY: the callback only dereferences `lparam` as the `FindCtx` passed here, which
        // outlives the call. `EnumWindows` returns 0 after an early stop; that is not an error.
        unsafe {
            EnumWindows(Some(find_callback), &mut ctx as *mut FindCtx as LPARAM);
        }
        (!ctx.found.is_null()).then_some(GameWindow(ctx.found))
    }

    /// Brings the game to the front, restoring it first if minimized. Fails closed: no
    /// `AttachThreadInput` or fake-key tricks.
    pub fn focus_game(game: &GameWindow) -> Result<(), String> {
        // SAFETY: `IsWindow` accepts any handle value and reports validity.
        if unsafe { IsWindow(game.0) } == 0 {
            return Err("the game window closed".into());
        }
        // SAFETY: `IsIconic` on a handle just checked; its result is a plain flag.
        if unsafe { IsIconic(game.0) } != 0 {
            // SAFETY: restoring a window we have a handle to; the return value is the previous
            // visibility, not success, so it is ignored.
            unsafe {
                ShowWindow(game.0, SW_RESTORE);
            }
        }
        // SAFETY: `SetForegroundWindow` takes the handle only; failure is reported by 0.
        if unsafe { SetForegroundWindow(game.0) } == 0 {
            return Err("Windows refused to bring the game to the front".into());
        }
        Ok(())
    }

    /// Whether the game window is the foreground window right now.
    pub fn game_is_foreground(game: &GameWindow) -> bool {
        // SAFETY: `GetForegroundWindow` takes no arguments and returns a handle or null.
        unsafe { GetForegroundWindow() == game.0 }
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

    /// Non-Windows stand-in for the game's window handle.
    pub struct GameWindow;

    /// No game lookup off Windows.
    pub fn find_game_window() -> Option<GameWindow> {
        None
    }

    /// Focusing is not supported off Windows.
    pub fn focus_game(_game: &GameWindow) -> Result<(), String> {
        Err("not supported on this platform".into())
    }

    /// Never foreground off Windows.
    pub fn game_is_foreground(_game: &GameWindow) -> bool {
        false
    }
}

pub use imp::{
    GameWindow, OverlayWindow, conceal, find_game_window, focus_game,
    foreground_is_game_or_overlay, from_frame, game_is_foreground, reveal, work_area,
};

#[cfg(test)]
mod predicate_tests {
    use super::*;

    const GAME: &str = r"C:\Games\Path of Exile 2\PathOfExile.exe";
    const CHROME: &str = r"C:\Program Files\Google\Chrome\Application\chrome.exe";

    #[test]
    fn titles() {
        assert!(is_game_title("Path of Exile 2"));
        assert!(is_game_title("path of exile"));
        assert!(is_game_title("PATH OF EXILE 2"));
        assert!(!is_game_title("Exiled Exchange 2"));
        assert!(!is_game_title(""));
        assert!(is_game_title("Path of Exile 2 - Google Chrome"));
    }

    #[test]
    fn exes() {
        assert!(is_game_exe(GAME));
        assert!(is_game_exe(
            r"D:\Steam\steamapps\common\Path of Exile 2\PathOfExileSteam.exe"
        ));
        assert!(is_game_exe("PathOfExile_x64Steam.exe"));
        assert!(is_game_exe("C:/x/pathofexile_x64.exe"));
        assert!(!is_game_exe(CHROME));
        assert!(!is_game_exe(
            r"C:\Users\u\AppData\Local\Programs\Exiled Exchange 2\Exiled Exchange 2.exe"
        ));
        assert!(!is_game_exe(r"C:\Path of Exile 2\notes.exe"));
        assert!(!is_game_exe(""));
    }

    #[test]
    fn windows() {
        assert!(is_game_window("Path of Exile 2", Some(GAME)));
        assert!(!is_game_window(
            "Path of Exile 2 - Google Chrome",
            Some(CHROME)
        ));
        assert!(!is_game_window("Path of Exile 2", None));
        assert!(!is_game_window("Exiled Exchange 2", Some(GAME)));
    }
}

#[cfg(all(test, windows))]
mod tests {
    /// Read-only enumeration: the result is not asserted, only that it does not panic.
    #[test]
    fn find_game_window_enumerates_without_panicking() {
        let _ = super::find_game_window();
    }

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
