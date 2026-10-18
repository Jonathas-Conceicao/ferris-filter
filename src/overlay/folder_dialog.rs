//! The native "pick the game folder" dialog (`rfd`), run so the egui frame loop never blocks.
//!
//! Read-only: the dialog only lets the user choose a folder; nothing here writes, and the picked
//! path is handed on exactly as the dialog produced it (never canonicalized).
//!
//! The dialog is built on the UI thread, parented to the overlay window (`set_parent`), and its
//! `pick_folder` runs on a worker thread that sends the result back over a channel; the UI only
//! `try_recv`s it ([`FolderPick::poll`]). Cancel, a closed channel and a worker panic all read as
//! "no pick", and only one dialog can be open at a time.
//!
//! Z-order contingency: an owned window is expected above its topmost owner (Win32 owned-window
//! rule). If the in-game check finds the dialog behind the overlay, the fallback is to send
//! `ViewportCommand::WindowLevel(WindowLevel::Normal)` when the pick starts and
//! `WindowLevel::AlwaysOnTop` when [`FolderPick::poll`] finishes (wired by the app only if needed).

// Not used by the app yet; remove this allow when the Config tab starts the dialog.
#![allow(dead_code)]

use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::time::{Duration, Instant};

/// Title of the native dialog.
pub(crate) const DIALOG_TITLE: &str = "Select your Path of Exile 2 folder";

/// How long the overlay stays visible after the dialog closes, unless it is focused sooner.
pub(crate) const CLOSE_GRACE: Duration = Duration::from_millis(500);

/// A folder pick running on a worker thread.
#[derive(Default)]
pub(crate) struct FolderPick {
    rx: Option<Receiver<Option<PathBuf>>>,
    closed_at: Option<Instant>,
}

impl FolderPick {
    /// A dialog is open (its result has not been taken yet).
    pub(crate) fn is_open(&self) -> bool {
        self.rx.is_some()
    }

    /// Runs `pick` on a worker thread, then `repaint`. Returns `false` (and spawns nothing) when a
    /// dialog is already open.
    pub(crate) fn start_with(
        &mut self,
        pick: impl FnOnce() -> Option<PathBuf> + Send + 'static,
        repaint: impl FnOnce() + Send + 'static,
    ) -> bool {
        if self.is_open() {
            return false;
        }
        let (tx, rx) = mpsc::channel();
        self.rx = Some(rx);
        self.closed_at = None;
        std::thread::spawn(move || {
            let _ = tx.send(pick());
            repaint();
        });
        true
    }

    /// Opens the native dialog owned by the overlay window, at `current` or its nearest existing
    /// parent. Returns `false` when a dialog is already open.
    #[cfg(windows)]
    pub(crate) fn start(
        &mut self,
        frame: &eframe::Frame,
        current: Option<&Path>,
        ctx: &egui::Context,
    ) -> bool {
        if self.is_open() {
            return false;
        }
        let mut dialog = rfd::FileDialog::new().set_title(DIALOG_TITLE);
        if let Some(dir) = current.and_then(existing_ancestor) {
            dialog = dialog.set_directory(dir);
        }
        dialog = dialog.set_parent(frame);
        let ctx = ctx.clone();
        self.start_with(move || dialog.pick_folder(), move || ctx.request_repaint())
    }

    /// Not available off Windows.
    #[cfg(not(windows))]
    pub(crate) fn start(
        &mut self,
        _frame: &eframe::Frame,
        _current: Option<&Path>,
        _ctx: &egui::Context,
    ) -> bool {
        false
    }

    /// The picked folder once the dialog closed; `None` while it is open, after a cancel, or when
    /// the worker is gone.
    pub(crate) fn poll(&mut self) -> Option<PathBuf> {
        let rx = self.rx.as_ref()?;
        let result = match rx.try_recv() {
            Ok(picked) => picked,
            Err(TryRecvError::Empty) => return None,
            Err(TryRecvError::Disconnected) => None,
        };
        self.rx = None;
        self.closed_at = Some(Instant::now());
        result
    }

    /// Time since the dialog last closed, `None` before any dialog or once the overlay regained
    /// focus ([`note_focused`](Self::note_focused)).
    pub(crate) fn since_close(&self) -> Option<Duration> {
        self.closed_at.map(|t| t.elapsed())
    }

    /// The overlay is focused again: the close grace ends early.
    pub(crate) fn note_focused(&mut self) {
        self.closed_at = None;
    }
}

/// `path` itself when it is a folder, else its nearest existing parent folder; `None` when no
/// ancestor exists (the dialog then opens at the OS default).
pub(crate) fn existing_ancestor(path: &Path) -> Option<&Path> {
    path.ancestors().find(|p| p.is_dir())
}

/// Whether the overlay must stay visible. While the dialog is open the overlay must not
/// auto-hide: `visibility::advance` hides on the first unfocused frame and hiding the owner hides
/// its owned dialog. A short grace after it closes covers focus returning to the overlay a frame
/// or more after the dialog is gone; without it the overlay would hide right after a pick or
/// cancel.
pub(crate) fn keep_visible(
    focused: bool,
    dialog_open: bool,
    since_close: Option<Duration>,
) -> bool {
    focused || dialog_open || since_close.is_some_and(|d| d < CLOSE_GRACE)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::mpsc::channel;

    fn wait(p: &mut FolderPick) -> Option<PathBuf> {
        for _ in 0..400 {
            if let Some(v) = p.poll() {
                return Some(v);
            }
            if !p.is_open() {
                return None;
            }
            std::thread::sleep(Duration::from_millis(5));
        }
        panic!("worker never finished");
    }

    #[test]
    fn default_is_closed() {
        let mut p = FolderPick::default();
        assert!(!p.is_open());
        assert_eq!(p.poll(), None);
        assert_eq!(p.since_close(), None);
    }

    #[test]
    fn pick_is_returned_then_closed() {
        let mut p = FolderPick::default();
        let (go_tx, go_rx) = channel::<()>();
        assert!(p.start_with(
            move || {
                go_rx.recv().unwrap();
                Some(PathBuf::from("C:/x"))
            },
            || {}
        ));
        assert!(p.is_open());
        assert_eq!(p.poll(), None);
        assert!(p.is_open());
        go_tx.send(()).unwrap();
        assert_eq!(wait(&mut p), Some(PathBuf::from("C:/x")));
        assert!(!p.is_open());
    }

    #[test]
    fn picked_path_is_verbatim() {
        let mut p = FolderPick::default();
        let raw = PathBuf::from("C:/a/./b//c/..");
        let sent = raw.clone();
        p.start_with(move || Some(sent), || {});
        assert_eq!(wait(&mut p), Some(raw));
    }

    #[test]
    fn cancel_reads_as_no_pick() {
        let mut p = FolderPick::default();
        p.start_with(|| None, || {});
        assert_eq!(wait(&mut p), None);
        assert!(!p.is_open());
        assert!(p.since_close().is_some());
    }

    #[test]
    fn worker_panic_reads_as_cancel() {
        let mut p = FolderPick::default();
        p.start_with(|| panic!("boom"), || {});
        assert_eq!(wait(&mut p), None);
        assert!(!p.is_open());
    }

    #[test]
    fn second_start_while_open_is_refused() {
        let mut p = FolderPick::default();
        let (go_tx, go_rx) = channel::<()>();
        assert!(p.start_with(
            move || {
                go_rx.recv().ok();
                None
            },
            || {}
        ));
        assert!(!p.start_with(|| Some(PathBuf::from("other")), || {}));
        go_tx.send(()).unwrap();
        assert_eq!(wait(&mut p), None);
    }

    #[test]
    fn repaint_runs_after_the_pick() {
        let mut p = FolderPick::default();
        let (tx, rx) = channel::<()>();
        p.start_with(|| None, move || tx.send(()).unwrap());
        rx.recv_timeout(Duration::from_secs(2)).unwrap();
    }

    #[test]
    fn existing_ancestor_cases() {
        let tmp = std::env::temp_dir().join(format!("ff-fd-{}", std::process::id()));
        std::fs::create_dir_all(&tmp).unwrap();
        assert_eq!(existing_ancestor(&tmp), Some(tmp.as_path()));
        let deep = tmp.join("a").join("b").join("c");
        assert_eq!(existing_ancestor(&deep), Some(tmp.as_path()));
        std::fs::remove_dir_all(&tmp).unwrap();
        assert_eq!(existing_ancestor(Path::new("Z:/no/such/dir/at/all")), None);
    }

    #[test]
    fn keep_visible_rules() {
        assert!(keep_visible(false, true, None));
        assert!(!keep_visible(false, false, None));
        assert!(keep_visible(true, false, None));
    }

    #[test]
    fn keep_visible_grace() {
        assert!(keep_visible(false, false, Some(Duration::from_millis(100))));
        assert!(!keep_visible(false, false, Some(CLOSE_GRACE)));
    }

    #[test]
    fn note_focused_clears_grace() {
        let mut p = FolderPick::default();
        p.start_with(|| None, || {});
        wait(&mut p);
        assert!(p.since_close().is_some());
        p.note_focused();
        assert_eq!(p.since_close(), None);
    }
}
