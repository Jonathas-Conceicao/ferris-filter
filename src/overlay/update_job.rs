//! The About tab's update button model: worker thread, button states and install-on-quit intent.
//!
//! **Framework-free by contract** — no UI type appears in any signature here. What the button
//! says, whether it is enabled, what a finished check reports and what happens to the downloaded
//! installer at shutdown are all decided here and unit-tested without a window.
//!
//! This is a third copy of the `EconomySync` / `LeagueList` worker pattern on purpose (no shared
//! generic worker); it differs only in carrying a stage message (`Downloading`) before `Done`.
//!
//! **Installer lifecycle.** A verified installer is handed to the spawn only when the player
//! clicked Install (which re-checks the file and sets the intent) and the process is then shutting
//! down. Any other shutdown deletes the work folder: a partial download while Downloading, the
//! verified but unused installer while Ready.

use std::path::{Path, PathBuf};
use std::sync::mpsc::{Receiver, Sender, TryRecvError};
use std::sync::{Arc, Mutex};

use crate::update::{
    CheckResult, Failure, InstallerCheck, ReadyUpdate, UpdateKind, WORK_PREFIX,
    downloading_message, message_for, verify_installer,
};

/// What the button is doing.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum UpdateState {
    #[default]
    Idle,
    Checking,
    Downloading {
        version: String,
    },
    Ready(ReadyUpdate),
}

impl UpdateState {
    /// The button caption.
    pub fn caption(&self) -> String {
        match self {
            Self::Idle => "Check for updates".to_string(),
            Self::Checking => "Checking…".to_string(),
            Self::Downloading { .. } => "Downloading…".to_string(),
            Self::Ready(r) => format!("Install {} and quit", r.version),
        }
    }

    /// Whether the button can be clicked.
    pub fn enabled(&self) -> bool {
        matches!(self, Self::Idle | Self::Ready(_))
    }

    /// The button tooltip.
    pub fn tooltip(&self) -> &'static str {
        match self {
            Self::Idle => "Look for a newer version on GitHub.",
            Self::Checking => "Asking GitHub.",
            Self::Downloading { .. } => "Downloading and checking the installer.",
            Self::Ready(_) => "Closes FerrisFilter and starts the installer.",
        }
    }
}

/// A message from the worker to the UI thread.
#[derive(Debug)]
pub enum UpdateMsg {
    Downloading { version: String },
    Done(CheckResult),
}

/// The worker's handle for reporting progress and publishing its work folder.
#[derive(Debug, Clone)]
pub struct UpdateProgress {
    tx: Sender<UpdateMsg>,
    work_dir: Arc<Mutex<Option<PathBuf>>>,
}

impl UpdateProgress {
    /// Report that the installer for `version` is being downloaded.
    pub fn downloading(&self, version: &str) {
        // A send failure means the app shut down; dropping the message is correct.
        let _ = self.tx.send(UpdateMsg::Downloading {
            version: version.to_string(),
        });
    }

    /// The slot where the worker publishes its work folder, so shutdown can delete a partial one.
    pub fn work_dir(&self) -> &Mutex<Option<PathBuf>> {
        &self.work_dir
    }
}

/// The outcome of an Install click.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InstallClick {
    NotReady,
    Proceed,
    Failed((String, UpdateKind)),
}

/// Owns the running check, its channel and the install intent.
#[derive(Debug)]
pub struct UpdateJob {
    state: UpdateState,
    rx: Option<Receiver<UpdateMsg>>,
    work_dir: Arc<Mutex<Option<PathBuf>>>,
    install_on_quit: bool,
}

impl Default for UpdateJob {
    fn default() -> Self {
        Self::new()
    }
}

impl UpdateJob {
    pub fn new() -> Self {
        Self {
            state: UpdateState::Idle,
            rx: None,
            work_dir: Arc::new(Mutex::new(None)),
            install_on_quit: false,
        }
    }

    pub fn state(&self) -> &UpdateState {
        &self.state
    }

    pub fn caption(&self) -> String {
        self.state.caption()
    }

    pub fn enabled(&self) -> bool {
        self.state.enabled()
    }

    pub fn tooltip(&self) -> &'static str {
        self.state.tooltip()
    }

    /// Start a check on a worker thread. Only from Idle; otherwise a no-op returning false (a
    /// Ready update is sticky and a running check is never doubled).
    pub fn start<F>(&mut self, work: F) -> bool
    where
        F: FnOnce(UpdateProgress) -> CheckResult + Send + 'static,
    {
        if !matches!(self.state, UpdateState::Idle) {
            return false;
        }
        if let Ok(mut slot) = self.work_dir.lock() {
            *slot = None;
        }
        let (tx, rx) = std::sync::mpsc::channel();
        let progress = UpdateProgress {
            tx: tx.clone(),
            work_dir: Arc::clone(&self.work_dir),
        };
        std::thread::spawn(move || {
            // A send failure means the app shut down mid-check; the result has nowhere to go and
            // dropping it is correct.
            let _ = tx.send(UpdateMsg::Done(work(progress)));
        });
        self.rx = Some(rx);
        self.state = UpdateState::Checking;
        true
    }

    /// Collect worker messages; call once per frame. Returns the last status text that arrived.
    pub fn poll(&mut self, utc_offset_minutes: i32) -> Option<(String, UpdateKind)> {
        let mut text = None;
        loop {
            let Some(rx) = self.rx.as_ref() else {
                return text;
            };
            match rx.try_recv() {
                Ok(UpdateMsg::Downloading { version }) => {
                    text = Some(downloading_message(&version));
                    self.state = UpdateState::Downloading { version };
                }
                Ok(UpdateMsg::Done(result)) => {
                    text = Some(message_for(&result, utc_offset_minutes));
                    match result {
                        CheckResult::Ready(r) => self.state = UpdateState::Ready(r),
                        _ => {
                            self.state = UpdateState::Idle;
                            if let Ok(mut slot) = self.work_dir.lock() {
                                *slot = None;
                            }
                        }
                    }
                    self.rx = None;
                    return text;
                }
                Err(TryRecvError::Empty) => return text,
                Err(TryRecvError::Disconnected) => {
                    // The worker died before reporting (a panic): never leave the button stuck.
                    self.rx = None;
                    self.state = UpdateState::Idle;
                    self.abandon();
                    return Some(message_for(
                        &CheckResult::Failed(Failure::BadResponse),
                        utc_offset_minutes,
                    ));
                }
            }
        }
    }

    /// Set the state without a worker (screenshot mode and tests).
    #[cfg_attr(not(any(test, debug_assertions)), allow(dead_code))]
    pub fn inject(&mut self, state: UpdateState) {
        self.state = state;
        self.rx = None;
    }

    /// The Install click: re-check the installer and set the install-on-quit intent.
    /// Never spawns anything; the spawn happens after the event loop ends.
    pub fn begin_install(&mut self) -> InstallClick {
        let UpdateState::Ready(r) = &self.state else {
            return InstallClick::NotReady;
        };
        let failure = match verify_installer(&r.installer, &r.sha256) {
            Ok(()) => {
                self.install_on_quit = true;
                return InstallClick::Proceed;
            }
            Err(InstallerCheck::Gone) => Failure::InstallerGone {
                version: r.version.clone(),
            },
            Err(InstallerCheck::Mismatch) => Failure::Mismatch {
                version: r.version.clone(),
            },
        };
        self.abandon();
        self.state = UpdateState::Idle;
        self.install_on_quit = false;
        InstallClick::Failed(message_for(&CheckResult::Failed(failure), 0))
    }

    /// Clear the install intent (Cancel, or a refused Apply). Ready stays, the installer is kept.
    pub fn cancel_install(&mut self) {
        self.install_on_quit = false;
    }

    #[cfg(test)]
    pub fn install_intended(&self) -> bool {
        self.install_on_quit
    }

    /// Delete the work folder, best effort. Only ever removes a folder whose final component
    /// starts with [`WORK_PREFIX`].
    fn abandon(&mut self) {
        let mut candidates: Vec<PathBuf> = Vec::new();
        if let Ok(mut slot) = self.work_dir.lock()
            && let Some(p) = slot.take()
        {
            candidates.push(p);
        }
        if let UpdateState::Ready(r) = &self.state
            && let Some(parent) = r.installer.parent()
        {
            candidates.push(parent.to_path_buf());
        }
        for dir in candidates {
            if is_work_folder(&dir) {
                let _ = std::fs::remove_dir_all(&dir);
            }
        }
    }

    /// Called once when the app shuts down. Returns the installer to launch only when the player
    /// clicked Install; otherwise deletes the work folder and returns None.
    pub fn shutdown(&mut self) -> Option<PathBuf> {
        if self.install_on_quit
            && let UpdateState::Ready(r) = &self.state
        {
            // The installer runs from its folder; the stale sweep removes it later.
            return Some(r.installer.clone());
        }
        self.abandon();
        None
    }
}

fn is_work_folder(dir: &Path) -> bool {
    dir.file_name()
        .and_then(|n| n.to_str())
        .is_some_and(|n| n.starts_with(WORK_PREFIX))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{Duration, Instant};

    fn settle(job: &mut UpdateJob) -> Option<(String, UpdateKind)> {
        let deadline = Instant::now() + Duration::from_secs(5);
        let mut last = None;
        while Instant::now() < deadline {
            if let Some(m) = job.poll(0) {
                last = Some(m);
            }
            if matches!(job.state(), UpdateState::Idle | UpdateState::Ready(_)) {
                return last;
            }
            std::thread::sleep(Duration::from_millis(1));
        }
        panic!("update worker did not settle");
    }

    /// A real installer file in a `FerrisFilter-update-test` folder under `root`.
    fn ready_in(root: &Path) -> ReadyUpdate {
        let dir = root.join(format!("{WORK_PREFIX}test"));
        std::fs::create_dir_all(&dir).unwrap();
        let installer = dir.join("FerrisFilter-0.8.0-setup.exe");
        std::fs::write(&installer, b"installer bytes").unwrap();
        let sha256 = crate::update::sha256_file(&installer).unwrap();
        ReadyUpdate {
            version: "0.8.0".to_string(),
            published_unix: Some(1_700_000_000),
            installer,
            sha256,
        }
    }

    fn ready_job(root: &Path) -> (UpdateJob, ReadyUpdate) {
        let r = ready_in(root);
        let mut job = UpdateJob::new();
        job.inject(UpdateState::Ready(r.clone()));
        (job, r)
    }

    #[test]
    fn fresh_job_is_idle_with_check_caption() {
        let job = UpdateJob::new();
        assert_eq!(job.state(), &UpdateState::Idle);
        assert_eq!(job.caption(), "Check for updates");
        assert!(job.enabled());
        assert_eq!(job.tooltip(), "Look for a newer version on GitHub.");
    }

    #[test]
    fn state_tables_match_the_spec() {
        let checking = UpdateState::Checking;
        assert_eq!(checking.caption(), "Checking…");
        assert!(!checking.enabled());
        assert_eq!(checking.tooltip(), "Asking GitHub.");
        let dl = UpdateState::Downloading {
            version: "0.8.0".into(),
        };
        assert_eq!(dl.caption(), "Downloading…");
        assert!(!dl.enabled());
        assert_eq!(dl.tooltip(), "Downloading and checking the installer.");
        let td = tempfile::tempdir().unwrap();
        let ready = UpdateState::Ready(ready_in(td.path()));
        assert_eq!(ready.caption(), "Install 0.8.0 and quit");
        assert!(ready.enabled());
        assert_eq!(
            ready.tooltip(),
            "Closes FerrisFilter and starts the installer."
        );
    }

    #[test]
    fn start_is_a_noop_unless_idle() {
        let td = tempfile::tempdir().unwrap();
        let (tx, rx) = std::sync::mpsc::channel::<()>();
        let mut job = UpdateJob::new();
        assert!(job.start(move |_| {
            let _ = rx.recv();
            CheckResult::UpToDate {
                version: "0.7.0".into(),
                released: None,
            }
        }));
        assert_eq!(job.state(), &UpdateState::Checking);
        let ran = Arc::new(Mutex::new(false));
        let ran2 = Arc::clone(&ran);
        assert!(!job.start(move |_| {
            *ran2.lock().unwrap() = true;
            CheckResult::Failed(Failure::Offline)
        }));
        tx.send(()).unwrap();
        settle(&mut job);
        assert!(!*ran.lock().unwrap());

        // Ready is sticky.
        let (mut job, _) = ready_job(td.path());
        assert!(!job.start(|_| CheckResult::Failed(Failure::Offline)));
        assert!(matches!(job.state(), UpdateState::Ready(_)));
    }

    #[test]
    fn downloading_then_ready_flow() {
        let td = tempfile::tempdir().unwrap();
        let r = ready_in(td.path());
        let (gate_tx, gate_rx) = std::sync::mpsc::channel::<()>();
        let (seen_tx, seen_rx) = std::sync::mpsc::channel::<()>();
        let r2 = r.clone();
        let mut job = UpdateJob::new();
        job.start(move |p| {
            p.downloading("0.8.0");
            let _ = seen_tx.send(());
            let _ = gate_rx.recv();
            CheckResult::Ready(r2)
        });
        seen_rx.recv_timeout(Duration::from_secs(5)).unwrap();
        let deadline = Instant::now() + Duration::from_secs(5);
        let first = loop {
            if let Some(m) = job.poll(0) {
                break m;
            }
            assert!(Instant::now() < deadline);
            std::thread::sleep(Duration::from_millis(1));
        };
        assert_eq!(first, ("Downloading 0.8.0…".to_string(), UpdateKind::Info));
        assert_eq!(
            job.state(),
            &UpdateState::Downloading {
                version: "0.8.0".into()
            }
        );
        gate_tx.send(()).unwrap();
        let last = settle(&mut job).unwrap();
        assert_eq!(last.1, UpdateKind::Info);
        assert!(last.0.starts_with("0.8.0 available, released "));
        assert_eq!(job.state(), &UpdateState::Ready(r));
    }

    #[test]
    fn both_messages_in_one_poll_end_ready_with_last_text() {
        let td = tempfile::tempdir().unwrap();
        let r = ready_in(td.path());
        let mut job = UpdateJob::new();
        let r2 = r.clone();
        job.start(move |p| {
            p.downloading("0.8.0");
            CheckResult::Ready(r2)
        });
        // Let the worker finish, then poll once.
        std::thread::sleep(Duration::from_millis(200));
        let m = job.poll(0).unwrap();
        assert!(m.0.starts_with("0.8.0 available"));
        assert_eq!(job.state(), &UpdateState::Ready(r));
    }

    #[test]
    fn up_to_date_and_failure_return_to_idle() {
        let mut job = UpdateJob::new();
        job.start(|_| CheckResult::UpToDate {
            version: "0.7.0".into(),
            released: None,
        });
        let m = settle(&mut job).unwrap();
        assert_eq!(job.state(), &UpdateState::Idle);
        assert_eq!(m.1, UpdateKind::Ok);
        assert!(m.0.starts_with("Up to date ("));

        job.start(|_| CheckResult::Failed(Failure::Offline));
        let m = settle(&mut job).unwrap();
        assert_eq!(job.state(), &UpdateState::Idle);
        assert_eq!(m.1, UpdateKind::Warning);
    }

    #[test]
    fn panicking_worker_returns_to_idle_with_generic_message() {
        let mut job = UpdateJob::new();
        job.start(|_| panic!("worker died"));
        let m = settle(&mut job).unwrap();
        assert_eq!(job.state(), &UpdateState::Idle);
        assert_eq!(
            m,
            (
                "Could not read the latest version from GitHub.".to_string(),
                UpdateKind::Error
            )
        );
    }

    #[test]
    fn inject_sets_state_and_poll_is_quiet() {
        let td = tempfile::tempdir().unwrap();
        let (mut job, _) = ready_job(td.path());
        assert!(matches!(job.state(), UpdateState::Ready(_)));
        assert_eq!(job.poll(0), None);
    }

    #[test]
    fn begin_install_when_not_ready() {
        let mut job = UpdateJob::new();
        assert_eq!(job.begin_install(), InstallClick::NotReady);
        assert!(!job.install_intended());
    }

    #[test]
    fn missing_installer_goes_idle_with_no_longer_there_message() {
        let td = tempfile::tempdir().unwrap();
        let (mut job, r) = ready_job(td.path());
        std::fs::remove_file(&r.installer).unwrap();
        assert_eq!(
            job.begin_install(),
            InstallClick::Failed((
                "The installer for 0.8.0 is no longer there. Check for updates again.".to_string(),
                UpdateKind::Error
            ))
        );
        assert_eq!(job.state(), &UpdateState::Idle);
        assert!(!job.install_intended());
    }

    #[test]
    fn changed_installer_bytes_fail_and_delete_folder() {
        let td = tempfile::tempdir().unwrap();
        let (mut job, r) = ready_job(td.path());
        std::fs::write(&r.installer, b"tampered").unwrap();
        let click = job.begin_install();
        let expected = message_for(
            &CheckResult::Failed(Failure::Mismatch {
                version: "0.8.0".into(),
            }),
            0,
        );
        assert_eq!(click, InstallClick::Failed(expected));
        assert_eq!(job.state(), &UpdateState::Idle);
        assert!(!r.installer.parent().unwrap().exists());
    }

    #[test]
    fn valid_install_click_sets_intent_and_cancel_keeps_ready() {
        let td = tempfile::tempdir().unwrap();
        let (mut job, r) = ready_job(td.path());
        assert_eq!(job.begin_install(), InstallClick::Proceed);
        assert!(job.install_intended());
        assert!(matches!(job.state(), UpdateState::Ready(_)));
        job.cancel_install();
        assert!(!job.install_intended());
        assert!(matches!(job.state(), UpdateState::Ready(_)));
        assert!(r.installer.exists());
    }

    #[test]
    fn shutdown_with_intent_returns_path_and_keeps_folder() {
        let td = tempfile::tempdir().unwrap();
        let (mut job, r) = ready_job(td.path());
        assert_eq!(job.begin_install(), InstallClick::Proceed);
        assert_eq!(job.shutdown(), Some(r.installer.clone()));
        assert!(r.installer.parent().unwrap().exists());
    }

    #[test]
    fn shutdown_without_intent_deletes_the_folder() {
        let td = tempfile::tempdir().unwrap();
        let (mut job, r) = ready_job(td.path());
        assert_eq!(job.shutdown(), None);
        assert!(!r.installer.parent().unwrap().exists());
    }

    #[test]
    fn shutdown_after_cancel_deletes_the_folder() {
        let td = tempfile::tempdir().unwrap();
        let (mut job, r) = ready_job(td.path());
        job.begin_install();
        job.cancel_install();
        assert_eq!(job.shutdown(), None);
        assert!(!r.installer.parent().unwrap().exists());
    }

    #[test]
    fn shutdown_while_downloading_deletes_the_partial_folder() {
        let td = tempfile::tempdir().unwrap();
        let partial = td.path().join(format!("{WORK_PREFIX}partial"));
        std::fs::create_dir_all(&partial).unwrap();
        std::fs::write(partial.join("part.exe"), b"half").unwrap();
        let (gate_tx, gate_rx) = std::sync::mpsc::channel::<()>();
        let (pub_tx, pub_rx) = std::sync::mpsc::channel::<()>();
        let p2 = partial.clone();
        let mut job = UpdateJob::new();
        job.start(move |p| {
            *p.work_dir().lock().unwrap() = Some(p2);
            p.downloading("0.8.0");
            let _ = pub_tx.send(());
            let _ = gate_rx.recv();
            CheckResult::Failed(Failure::Offline)
        });
        pub_rx.recv_timeout(Duration::from_secs(5)).unwrap();
        assert_eq!(job.shutdown(), None);
        assert!(!partial.exists());
        let _ = gate_tx.send(());
    }

    #[test]
    fn shutdown_while_idle_is_none() {
        let mut job = UpdateJob::new();
        assert_eq!(job.shutdown(), None);
    }

    #[test]
    fn abandon_never_deletes_a_folder_without_the_work_prefix() {
        let td = tempfile::tempdir().unwrap();
        let other = td.path().join("something-else");
        std::fs::create_dir_all(&other).unwrap();
        let mut job = UpdateJob::new();
        *job.work_dir.lock().unwrap() = Some(other.clone());
        assert_eq!(job.shutdown(), None);
        assert!(other.exists());
    }
}
