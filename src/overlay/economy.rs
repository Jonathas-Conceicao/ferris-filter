//! The overlay's economy-sync affordance: refresh `prices.json` without leaving the game.
//!
//! **Framework-free by contract** — no `egui` type appears in any signature here, the same
//! discipline `overlay::apply` holds (`PITFALLS.md` §8). What a sync *reports*
//! and what state it leaves behind is decided here and unit-tested without a window; `app.rs` owns
//! the button and the status line.
//!
//! **Why a worker thread.** A price sync is dozens of HTTP round trips against poe.ninja. Running
//! it on the UI thread would freeze the stripe — and the stripe is drawn over a live game, so a
//! frozen overlay is worse than a slow one. [`EconomySync::start`] hands the work to a thread and
//! the frame loop collects the result through a channel, in the same poll-once-per-frame place the
//! tray menu is already read.
//!
//! **What a sync does not do.** It writes `prices.json` and stops. `dynamic.filter` — the file the
//! game reads — changes only on **Apply**, exactly as before. That separation is what makes the
//! button safe to press mid-map: nothing the game is reading changes underneath it, and the new
//! economy lands on the next Apply the player chooses to make (author, 2026-09-10).

use std::sync::mpsc::{Receiver, TryRecvError};

/// What the stripe should be showing about the economy data.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum SyncState {
    /// No sync has run this session.
    #[default]
    Idle,
    /// A worker thread is fetching. The button is inert while this holds.
    Running,
    /// The last sync finished. Carries the line to show the player.
    Finished(SyncOutcome),
}

/// The result of one completed sync, in the words the stripe will use.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SyncOutcome {
    /// Prices were refreshed and written.
    Ok(String),
    /// The sync failed. The message is the player's only account of why — the release binary runs
    /// under the Windows GUI subsystem with no console (D-17), so anything not said here is said
    /// nowhere.
    Failed(String),
}

impl SyncOutcome {
    /// The line to render.
    pub fn message(&self) -> &str {
        match self {
            Self::Ok(m) | Self::Failed(m) => m,
        }
    }

    /// Whether this outcome is a failure, for the caller's colour choice. Callers must not decide
    /// this by string-matching the message.
    pub fn is_failure(&self) -> bool {
        matches!(self, Self::Failed(_))
    }
}

/// What the stripe says after prices are refreshed.
///
/// It names Apply explicitly. A player who has just synced has every reason to think the game now
/// sees new prices, and it does not — the same reasoning that makes Apply repeat the in-game reload
/// path (D-16). Telling them here is cheaper than letting them discover it from a drop that did not
/// change colour.
pub fn success_message(league: &str) -> String {
    if league.is_empty() {
        return "Economy prices refreshed. Press APPLY to compile them into the filter."
            .to_string();
    }
    format!("Economy prices refreshed from {league}. Press APPLY to compile them into the filter.")
}

/// What the stripe says when a sync fails.
pub fn failure_message(err: &str) -> String {
    format!("Economy sync failed: {err}")
}

/// Owns the running sync and its result channel.
///
/// Held by the app across frames. [`Self::poll`] must be called once per frame; nothing else moves
/// the state machine forward.
#[derive(Debug, Default)]
pub struct EconomySync {
    state: SyncState,
    rx: Option<Receiver<Result<String, String>>>,
}

impl EconomySync {
    pub fn new() -> Self {
        Self::default()
    }

    /// The current state, for rendering.
    pub fn state(&self) -> &SyncState {
        &self.state
    }

    /// Whether a sync is in flight — the button must be inert while it is, or a player can queue
    /// several syncs writing the same file.
    pub fn is_running(&self) -> bool {
        matches!(self.state, SyncState::Running)
    }

    /// Start a sync on a worker thread.
    ///
    /// `work` is the blocking job — in production `sync::sync_economy_blocking`, in tests a closure
    /// that returns immediately. Taking it as a parameter is what lets the whole state machine be
    /// tested with no network and no window.
    ///
    /// Calling this while a sync is already running does nothing, so a double-click cannot start
    /// two writers against `prices.json`.
    pub fn start<F>(&mut self, work: F)
    where
        F: FnOnce() -> Result<String, String> + Send + 'static,
    {
        if self.is_running() {
            return;
        }
        let (tx, rx) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            // A send failure means the app shut down mid-sync; the result has nowhere to go and
            // dropping it is correct.
            let _ = tx.send(work());
        });
        self.rx = Some(rx);
        self.state = SyncState::Running;
    }

    /// Drop a finished sync's message, returning to [`SyncState::Idle`].
    ///
    /// Called when the stripe is concealed: the message answers a question the player asked
    /// before hiding it, and on the next reveal it would read as a report about whatever they are
    /// about to do. A sync still **in flight** is deliberately untouched — hiding the overlay does
    /// not cancel the worker, and the button must stay inert until it reports.
    pub fn clear_message(&mut self) {
        if matches!(self.state, SyncState::Finished(_)) {
            self.state = SyncState::Idle;
        }
    }

    /// Collect a finished sync, if one has landed. Call once per frame.
    ///
    /// Returns `true` when this call moved the state, so the caller can request a repaint rather
    /// than polling at whatever rate the game happens to be redrawing at.
    pub fn poll(&mut self) -> bool {
        let Some(rx) = self.rx.as_ref() else {
            return false;
        };
        match rx.try_recv() {
            Ok(Ok(league)) => {
                self.state = SyncState::Finished(SyncOutcome::Ok(success_message(&league)));
                self.rx = None;
                true
            }
            Ok(Err(e)) => {
                self.state = SyncState::Finished(SyncOutcome::Failed(failure_message(&e)));
                self.rx = None;
                true
            }
            Err(TryRecvError::Empty) => false,
            Err(TryRecvError::Disconnected) => {
                // The worker died without sending — a panic in the sync. Report it rather than
                // leaving the button inert forever, which is what happens if this arm is missing.
                self.state = SyncState::Finished(SyncOutcome::Failed(failure_message(
                    "the sync worker stopped unexpectedly",
                )));
                self.rx = None;
                true
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn settle(sync: &mut EconomySync) {
        // A wall-clock deadline, not a yield count: a worker that panics has to unwind before its
        // sender drops, which takes far longer than a few thousand yields and made this flaky.
        // The loop still exits the instant a result lands, so the common case costs ~nothing.
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
        while std::time::Instant::now() < deadline {
            if sync.poll() {
                return;
            }
            std::thread::sleep(std::time::Duration::from_millis(1));
        }
        panic!("sync never reported a result within 5s");
    }

    #[test]
    fn a_fresh_sync_has_not_run_yet() {
        let sync = EconomySync::new();
        assert_eq!(*sync.state(), SyncState::Idle);
        assert!(!sync.is_running());
    }

    #[test]
    fn a_started_sync_is_running_until_it_reports() {
        let mut sync = EconomySync::new();
        sync.start(|| Ok("Forbidden Rites".to_string()));
        assert!(
            sync.is_running(),
            "the button must be inert while a sync is in flight"
        );
        settle(&mut sync);
        assert!(!sync.is_running());
    }

    #[test]
    fn a_successful_sync_tells_the_player_to_press_apply() {
        // The whole point of the separation: prices land in prices.json, and the game sees them
        // only after Apply. A success message that does not say so invites the player to assume
        // the filter already changed.
        let mut sync = EconomySync::new();
        sync.start(|| Ok("Forbidden Rites".to_string()));
        settle(&mut sync);
        let SyncState::Finished(outcome) = sync.state() else {
            panic!("expected a finished sync, got {:?}", sync.state());
        };
        assert!(!outcome.is_failure());
        assert!(
            outcome.message().to_ascii_uppercase().contains("APPLY"),
            "success must name Apply; got: {}",
            outcome.message()
        );
    }

    #[test]
    fn a_successful_sync_names_the_league_it_priced_against() {
        // Automatic resolution picks the first non-hardcore *indexed* league, and poe.ninja has
        // had that flag wrong about a live league before -- during the Forbidden Rites launch it
        // flagged the outgoing league instead. A filter priced for last league's economy looks
        // entirely normal, so the league has to be stated rather than assumed (author, 2026-09-10).
        let mut sync = EconomySync::new();
        sync.start(|| Ok("Forbidden Rites".to_string()));
        settle(&mut sync);
        let SyncState::Finished(outcome) = sync.state() else {
            panic!("expected a finished sync");
        };
        assert!(
            outcome.message().contains("Forbidden Rites"),
            "the league must appear in the success line; got: {}",
            outcome.message()
        );
    }

    #[test]
    fn hiding_the_stripe_clears_a_finished_message_but_not_a_running_sync() {
        // Clearing: the message answers a question asked before the stripe was hidden, and would
        // otherwise reappear looking like a report about whatever the player does next.
        let mut sync = EconomySync::new();
        sync.start(|| Ok("Forbidden Rites".to_string()));
        settle(&mut sync);
        sync.clear_message();
        assert_eq!(*sync.state(), SyncState::Idle);

        // Not cancelling: hiding the overlay does not stop the worker, so a sync still in flight
        // must stay Running or the button would go live against a thread still writing the file.
        let mut running = EconomySync::new();
        let (tx, rx) = std::sync::mpsc::channel::<()>();
        running.start(move || {
            let _ = rx.recv();
            Ok(String::new())
        });
        running.clear_message();
        assert!(
            running.is_running(),
            "an in-flight sync must survive the stripe being hidden"
        );
        let _ = tx.send(());
        settle(&mut running);
    }

    #[test]
    fn a_failed_sync_surfaces_the_reason_in_the_stripe() {
        // D-17: the release binary has no console, so an error not shown here is shown nowhere.
        let mut sync = EconomySync::new();
        sync.start(|| Err("no such league 'Forbidden Rites'".to_string()));
        settle(&mut sync);
        let SyncState::Finished(outcome) = sync.state() else {
            panic!("expected a finished sync");
        };
        assert!(outcome.is_failure());
        assert!(
            outcome.message().contains("Forbidden Rites"),
            "the underlying reason must survive into the message; got: {}",
            outcome.message()
        );
    }

    #[test]
    fn a_worker_that_panics_does_not_wedge_the_button_forever() {
        let mut sync = EconomySync::new();
        sync.start(|| panic!("boom"));
        settle(&mut sync);
        assert!(
            !sync.is_running(),
            "a dead worker must not leave the sync stuck as Running"
        );
        let SyncState::Finished(outcome) = sync.state() else {
            panic!("expected a finished sync");
        };
        assert!(outcome.is_failure());
    }

    #[test]
    fn starting_twice_does_not_launch_a_second_writer() {
        // Two threads writing prices.json is a corrupted file, so a double-click must be inert
        // rather than merely unlikely to matter.
        let mut sync = EconomySync::new();
        let (block_tx, block_rx) = std::sync::mpsc::channel::<()>();
        sync.start(move || {
            let _ = block_rx.recv();
            Ok(String::new())
        });
        assert!(sync.is_running());

        let second_ran = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        let flag = std::sync::Arc::clone(&second_ran);
        sync.start(move || {
            flag.store(true, std::sync::atomic::Ordering::SeqCst);
            Ok(String::new())
        });
        assert!(
            !second_ran.load(std::sync::atomic::Ordering::SeqCst),
            "a second sync must not launch while the first is in flight"
        );
        assert!(sync.is_running(), "still the first sync");

        let _ = block_tx.send(());
        settle(&mut sync);
        assert!(!sync.is_running());
    }
}
