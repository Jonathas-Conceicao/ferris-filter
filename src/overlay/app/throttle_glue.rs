//! Glue between the pure request throttle (`crate::throttle`) and the overlay's three network
//! actions (Sync economy, the league list, Check for updates).
//!
//! A click inside the hour is a *fake* refresh: the same worker seam runs a closure that returns
//! the last real result, so the status line shows the same text and nothing is sent. Only a real
//! success is recorded (never at click time, never for a fake), so a failure never starts the
//! hour and a fake never extends it. `ff-state.json` is written best effort and silently.

use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use crate::overlay::leagues::ListState;
use crate::overlay::update_job::UpdateProgress;
use crate::sync::models::EconomyLeague;
use crate::update::CheckResult;

use super::economy;

/// A real sync in flight: the league as requested and the name the sync returned.
struct PendingSync {
    requested: String,
    returned: Arc<Mutex<Option<String>>>,
}

/// Which running requests are real (and so are recorded when they succeed).
#[derive(Default)]
pub(super) struct PendingReal {
    sync: Option<PendingSync>,
    leagues: bool,
    update: bool,
}

impl super::OverlayApp {
    /// Sync economy for `league`: a fake replay when the log allows it, else the real job.
    pub(super) fn start_sync_gated(&mut self, root: PathBuf, league: String, now: u64) {
        let name = league.clone();
        self.start_sync_gated_with(league, now, move || {
            crate::sync::sync_economy_blocking(&root, &name).map_err(|e| e.to_string())
        });
    }

    pub(super) fn start_sync_gated_with<F>(&mut self, league: String, now: u64, real: F)
    where
        F: FnOnce() -> Result<String, String> + Send + 'static,
    {
        if self.economy.is_running() {
            return;
        }
        // The gate must see prices.json as it is now (the CLI may have rewritten or removed it
        // since the cached info was read). One small file read per click, never per frame.
        self.refresh_economy_info();
        if let Some(name) = self.request_log.sync_fake(
            &league,
            self.economy_info.league.as_deref(),
            self.economy_info.live,
            now,
        ) {
            // Drops a stale pending real sync (its outcome was cleared unseen) so it can never
            // be recorded later.
            self.pending_real.sync = None;
            self.economy.start(move || Ok(name));
            return;
        }
        let returned = Arc::new(Mutex::new(None));
        self.pending_real.sync = Some(PendingSync {
            requested: league,
            returned: Arc::clone(&returned),
        });
        self.economy.start(move || {
            let result = real();
            if let Ok(name) = &result
                && let Ok(mut slot) = returned.lock()
            {
                *slot = Some(name.clone());
            }
            result
        });
    }

    /// The league list: the cached list inside the hour, else the real fetch.
    pub(super) fn start_leagues_gated(&mut self, now: u64) {
        self.start_leagues_gated_with(now, super::fetch_leagues);
    }

    pub(super) fn start_leagues_gated_with<F>(&mut self, now: u64, real: F)
    where
        F: FnOnce() -> Result<Vec<EconomyLeague>, String> + Send + 'static,
    {
        if self.leagues.is_fetching() {
            return;
        }
        if let Some(list) = self.request_log.cached_leagues(now) {
            self.leagues.start(move || Ok(list));
        } else {
            self.pending_real.leagues = true;
            self.leagues.start(real);
        }
    }

    /// Check for updates: an "up to date" answer is replayed inside the hour, else `real` runs.
    pub(super) fn start_update_gated_with<F>(&mut self, now: u64, real: F) -> bool
    where
        F: FnOnce(UpdateProgress) -> CheckResult + Send + 'static,
    {
        if let Some(result) = self
            .request_log
            .cached_update(env!("CARGO_PKG_VERSION"), now)
        {
            self.pending_real.update = false;
            return self.update.start(move |_| result);
        }
        let started = self.update.start(real);
        if started {
            self.pending_real.update = true;
        }
        started
    }

    /// Records finished real successes and saves `ff-state.json`. Call once per frame, after the
    /// economy, league and update polls.
    pub(super) fn record_after_polls(&mut self, now: u64) {
        let mut changed = false;

        if self.pending_real.sync.is_some() {
            match self.economy.state() {
                economy::SyncState::Running => {}
                economy::SyncState::Finished(outcome) => {
                    let failed = outcome.is_failure();
                    if let Some(p) = self.pending_real.sync.take()
                        && !failed
                    {
                        let returned = p
                            .returned
                            .lock()
                            .ok()
                            .and_then(|s| s.clone())
                            .unwrap_or_else(|| p.requested.clone());
                        self.request_log.record_sync(&p.requested, &returned, now);
                        changed = true;
                    }
                }
                economy::SyncState::Idle => self.pending_real.sync = None,
            }
        }

        if self.pending_real.leagues && !self.leagues.is_fetching() {
            self.pending_real.leagues = false;
            if let ListState::Loaded(list) = self.leagues.state() {
                self.request_log.record_leagues(list, now);
                changed = true;
            }
        }

        if let Some(result) = self.update.take_done()
            && self.pending_real.update
        {
            self.pending_real.update = false;
            changed |= self
                .request_log
                .record_update(&result, env!("CARGO_PKG_VERSION"), now);
        }

        if changed {
            self.request_log.prune(now);
            if !self.is_shot()
                && let Ok(root) = &self.root
            {
                // Best effort and silent: a missing or unwritable file only means no throttle.
                let _ = crate::throttle::save(root, &self.request_log);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::OverlayApp;
    use super::*;
    use crate::overlay::economy::{SyncState, success_message};
    use crate::throttle::{self, RequestLog};
    use std::time::{Duration, Instant};

    const LEAGUE: &str = "Rise of the Abyssal";

    fn league(name: &str) -> EconomyLeague {
        EconomyLeague {
            name: name.into(),
            url: name.to_lowercase(),
            hardcore: false,
            indexed: true,
        }
    }

    /// An app on a temp home, optionally with a pre-written ff-state.json.
    fn app_with(log: Option<&RequestLog>) -> (OverlayApp, tempfile::TempDir, egui::Context) {
        use crate::overlay::hotkey::HotkeyStatus;
        use crate::overlay::tray::TrayHandles;
        let ctx = egui::Context::default();
        crate::skin::fonts::install(&ctx);
        let skin = crate::skin::install(&ctx);
        let home = tempfile::tempdir().unwrap();
        if let Some(log) = log {
            throttle::save(home.path(), log).unwrap();
        }
        let app = OverlayApp::build(
            TrayHandles::inert(),
            None,
            HotkeyStatus::Failed("test".to_string()),
            None,
            skin,
            Ok(home.path().to_path_buf()),
            Some(home.path().join("game")),
        );
        (app, home, ctx)
    }

    /// Writes a usable `prices.json` (the seed prices) synced for `league` into `home`.
    fn write_prices(home: &std::path::Path, league: &str) {
        let seed = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/seed-prices.json");
        let mut text: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(seed).unwrap()).unwrap();
        text["league"] = serde_json::json!(league);
        std::fs::write(home.join("prices.json"), text.to_string()).unwrap();
    }

    fn fresh_sync_log(requested: &str, returned: &str) -> RequestLog {
        let mut log = RequestLog::default();
        log.record_sync(requested, returned, 1000);
        log
    }

    /// Polls everything until no job is in flight, then records at `now`.
    fn pump(app: &mut OverlayApp, now: u64) {
        let deadline = Instant::now() + Duration::from_secs(5);
        loop {
            app.economy.poll();
            app.leagues.poll();
            let _ = app.update.poll(0);
            app.record_after_polls(now);
            let busy = app.economy.is_running()
                || app.leagues.is_fetching()
                || matches!(
                    app.update.state(),
                    crate::overlay::update_job::UpdateState::Checking
                );
            if !busy {
                return;
            }
            assert!(Instant::now() < deadline, "jobs never settled");
            std::thread::sleep(Duration::from_millis(1));
        }
    }

    fn sync_message(app: &OverlayApp) -> Option<String> {
        match app.economy.state() {
            SyncState::Finished(o) => Some(o.message().to_string()),
            _ => None,
        }
    }

    fn no_network<T>() -> Result<T, String> {
        panic!("a real request would have gone out");
    }

    #[test]
    fn fake_sync_replays_the_message_and_leaves_prices_alone() {
        let (mut app, home, _ctx) = app_with(Some(&fresh_sync_log(LEAGUE, LEAGUE)));
        let prices = home.path().join("prices.json");
        write_prices(home.path(), LEAGUE);
        let before = std::fs::metadata(&prices).unwrap().modified().unwrap();
        app.start_sync_gated_with(LEAGUE.into(), 1010, no_network);
        assert!(app.pending_real.sync.is_none());
        pump(&mut app, 1010);
        assert_eq!(sync_message(&app), Some(success_message(LEAGUE)));
        assert_eq!(
            std::fs::metadata(&prices).unwrap().modified().unwrap(),
            before
        );
        assert_eq!(app.request_log.sync[&LEAGUE.to_lowercase()].at, 1000);
    }

    #[test]
    fn real_sync_records_the_returned_spelling_then_fakes_it() {
        let (mut app, home, _ctx) = app_with(None);
        app.start_sync_gated_with(
            "rise of the abyssal".into(),
            2000,
            || Ok(LEAGUE.to_string()),
        );
        assert!(app.pending_real.sync.is_some());
        pump(&mut app, 2000);
        assert_eq!(app.request_log.sync["rise of the abyssal"].name, LEAGUE);
        assert!(app.pending_real.sync.is_none());
        assert!(
            throttle::load(home.path())
                .sync
                .contains_key("rise of the abyssal")
        );

        // The real sync would have written prices.json for the league it returned.
        write_prices(home.path(), LEAGUE);
        app.economy.clear_message();
        app.start_sync_gated_with("rise of the abyssal".into(), 2100, no_network);
        pump(&mut app, 2100);
        assert_eq!(sync_message(&app), Some(success_message(LEAGUE)));
        assert_eq!(app.request_log.sync["rise of the abyssal"].at, 2000);
    }

    #[test]
    fn stale_pending_real_sync_is_never_recorded_after_a_fake() {
        let mut log = fresh_sync_log("League B", "League B");
        log.sync.clear();
        log.record_sync("League B", "League B", 1000);
        let (mut app, home, _ctx) = app_with(Some(&log));
        write_prices(home.path(), "League B");
        // A real sync for A finishes, but its outcome is cleared before a poll recorded it.
        app.pending_real.sync = Some(PendingSync {
            requested: "League A".into(),
            returned: Arc::new(Mutex::new(Some("League A".into()))),
        });
        app.economy.start(|| Ok("League A".to_string()));
        let deadline = Instant::now() + Duration::from_secs(5);
        while !app.economy.poll() {
            assert!(Instant::now() < deadline);
            std::thread::sleep(Duration::from_millis(1));
        }
        app.economy.clear_message();
        app.start_sync_gated_with("League B".into(), 1010, no_network);
        assert!(app.pending_real.sync.is_none());
        pump(&mut app, 1010);
        assert!(!app.request_log.sync.contains_key("league a"));
        assert_eq!(app.request_log.sync["league b"].at, 1000);
    }

    #[test]
    fn real_path_when_other_league_not_live_or_expired() {
        let log = fresh_sync_log(LEAGUE, LEAGUE);
        for (league_arg, held, now) in [
            ("Other", Some("Other"), 1010),
            (LEAGUE, None, 1010),
            (LEAGUE, Some("Standard"), 1010),
            (LEAGUE, Some(LEAGUE), 1000 + 3600),
        ] {
            let (mut app, home, _ctx) = app_with(Some(&log));
            if let Some(held) = held {
                write_prices(home.path(), held);
            }
            app.start_sync_gated_with(league_arg.into(), now, || Err("offline".into()));
            assert!(app.pending_real.sync.is_some(), "{league_arg} {now}");
            pump(&mut app, now);
        }
    }

    #[test]
    fn gate_rereads_prices_json_at_click_time() {
        // The cached info says LEAGUE, but prices.json changed on disk since (CLI sync for
        // another league, or deleted): the click must go out for real.
        for replace in [Some("League B"), None] {
            let (mut app, home, _ctx) = app_with(Some(&fresh_sync_log(LEAGUE, LEAGUE)));
            write_prices(home.path(), LEAGUE);
            app.refresh_economy_info();
            assert_eq!(app.economy_info.league.as_deref(), Some(LEAGUE));
            match replace {
                Some(other) => write_prices(home.path(), other),
                None => std::fs::remove_file(home.path().join("prices.json")).unwrap(),
            }
            app.start_sync_gated_with(LEAGUE.into(), 1010, || Err("offline".into()));
            assert!(app.pending_real.sync.is_some(), "{replace:?}");
            pump(&mut app, 1010);
        }
    }

    #[test]
    fn failed_sync_records_nothing() {
        let (mut app, home, _ctx) = app_with(None);
        app.start_sync_gated_with(LEAGUE.into(), 100, || Err("boom".into()));
        pump(&mut app, 100);
        assert!(matches!(
            app.economy.state(),
            SyncState::Finished(o) if o.is_failure()
        ));
        assert!(app.request_log.sync.is_empty());
        assert!(app.pending_real.sync.is_none());
        assert!(!home.path().join("ff-state.json").exists());
    }

    #[test]
    fn cached_league_list_is_served_after_loading_the_state_file() {
        let mut log = RequestLog::default();
        log.record_leagues(&[league("A"), league("B")], 1000);
        let (mut app, _home, _ctx) = app_with(Some(&log));
        app.start_leagues_gated_with(1010, no_network);
        assert!(!app.pending_real.leagues);
        pump(&mut app, 1010);
        assert!(matches!(app.leagues.state(), ListState::Loaded(l) if l.len() == 2));
        assert_eq!(app.request_log.leagues_at, Some(1000));
    }

    #[test]
    fn real_league_fetch_is_recorded_and_failures_are_not() {
        let (mut app, home, _ctx) = app_with(None);
        app.start_leagues_gated_with(500, || Ok(vec![league("A")]));
        assert!(app.pending_real.leagues);
        pump(&mut app, 500);
        assert_eq!(app.request_log.leagues_at, Some(500));
        assert_eq!(throttle::load(home.path()).leagues.len(), 1);

        let (mut app, home, _ctx) = app_with(None);
        app.start_leagues_gated_with(500, || Err("offline".into()));
        pump(&mut app, 500);
        assert!(matches!(app.leagues.state(), ListState::Failed(_)));
        assert_eq!(app.request_log.leagues_at, None);
        assert!(!home.path().join("ff-state.json").exists());

        // Expired cache goes out again.
        let mut log = RequestLog::default();
        log.record_leagues(&[league("A")], 1000);
        let (mut app, _home, _ctx) = app_with(Some(&log));
        app.start_leagues_gated_with(1000 + 3600, || Ok(vec![league("A"), league("B")]));
        assert!(app.pending_real.leagues);
        pump(&mut app, 1000 + 3600);
        assert_eq!(app.request_log.leagues.len(), 2);
    }

    fn up_to_date() -> CheckResult {
        CheckResult::UpToDate {
            version: env!("CARGO_PKG_VERSION").to_string(),
            released: Some(1_700_000_000),
        }
    }

    #[test]
    fn update_fake_replays_the_message_and_is_not_recorded_again() {
        let mut log = RequestLog::default();
        log.record_update(&up_to_date(), env!("CARGO_PKG_VERSION"), 1000);
        let (mut app, home, _ctx) = app_with(Some(&log));
        assert!(app.start_update_gated_with(1010, |_| panic!("a real check went out")));
        assert!(!app.pending_real.update);
        let deadline = Instant::now() + Duration::from_secs(5);
        let text = loop {
            if let Some(m) = app.update.poll(0) {
                break m;
            }
            assert!(Instant::now() < deadline);
            std::thread::sleep(Duration::from_millis(1));
        };
        assert_eq!(text, crate::update::message_for(&up_to_date(), 0));
        app.record_after_polls(1010);
        assert_eq!(app.request_log.update_at, Some(1000));
        // The drained fake leaves the file as it was written.
        assert_eq!(throttle::load(home.path()).update_at, Some(1000));
    }

    #[test]
    fn real_up_to_date_is_recorded_ready_and_failed_are_not() {
        let (mut app, home, _ctx) = app_with(None);
        assert!(app.start_update_gated_with(300, |_| up_to_date()));
        assert!(app.pending_real.update);
        pump(&mut app, 300);
        assert_eq!(app.request_log.update_at, Some(300));
        assert!(throttle::load(home.path()).update_version.is_some());

        for result in [
            CheckResult::Failed(crate::update::Failure::Offline),
            CheckResult::Ready(crate::update::ReadyUpdate {
                version: "9.9.9".into(),
                published_unix: None,
                installer: PathBuf::from("x.exe"),
                sha256: String::new(),
            }),
        ] {
            let (mut app, home, _ctx) = app_with(None);
            assert!(app.start_update_gated_with(300, move |_| result));
            pump(&mut app, 300);
            assert_eq!(app.request_log.update_at, None);
            assert!(!home.path().join("ff-state.json").exists());
        }
    }

    #[test]
    fn missing_root_or_garbage_file_means_no_throttle() {
        let home = tempfile::tempdir().unwrap();
        std::fs::write(home.path().join("ff-state.json"), "{ nope").unwrap();
        assert_eq!(throttle::load(home.path()), RequestLog::default());
        let (app, _home, _ctx) = app_with(None);
        assert_eq!(app.request_log, RequestLog::default());
    }
}
