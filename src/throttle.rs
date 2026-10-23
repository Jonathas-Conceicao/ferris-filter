//! Request throttle: the overlay sends each kind of network request at most once an hour.
//!
//! Pure and clock-injected: every decision takes `now` (unix seconds) so tests use literal
//! seconds and never the real clock. The state lives in `ff-state.json` in the FerrisFilter
//! folder (separate from `ff-settings.json`, which holds preferences). A missing, unreadable or
//! wrongly typed file loads as an empty log, which means no throttle.
//!
//! The update check also persists the answered version and release date plus the running app
//! version, so the "Up to date (<ver>, released <date>)" message can be replayed identically
//! after a restart; a replay is offered only for the same app version that recorded it.
//!
//! The CLI never uses this module.

use std::collections::BTreeMap;
use std::io::{self, Write};
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::sync::models::EconomyLeague;
use crate::update::CheckResult;

/// One hour, from the last successful request of a kind.
pub const WINDOW_SECS: u64 = 3600;
/// The state file's name inside the FerrisFilter folder.
pub const FILE_NAME: &str = "ff-state.json";

const MAX_LEAGUES: usize = 200;

/// True when `last` is a recorded time no later than `now` and less than an hour old.
/// A time in the future counts as expired.
pub fn is_fresh(last: Option<u64>, now: u64) -> bool {
    match last {
        Some(t) => t <= now && now - t < WINDOW_SECS,
        None => false,
    }
}

/// A successful sync: when, and the league name the real sync returned.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SyncEntry {
    pub at: u64,
    pub name: String,
}

/// Last-success times and cached results, persisted as `ff-state.json`.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct RequestLog {
    /// Per requested league (lower-cased).
    pub sync: BTreeMap<String, SyncEntry>,
    pub leagues_at: Option<u64>,
    pub leagues: Vec<EconomyLeague>,
    pub update_at: Option<u64>,
    pub update_app_version: Option<String>,
    pub update_version: Option<String>,
    pub update_released: Option<i64>,
}

impl RequestLog {
    /// `Some(returned league name)` when a sync for `league` may be faked: it succeeded within
    /// the hour, `prices.json` is live and holds that league (either spelling).
    pub fn sync_fake(
        &self,
        league: &str,
        prices_league: Option<&str>,
        prices_live: bool,
        now: u64,
    ) -> Option<String> {
        let entry = self.sync.get(&league.to_lowercase())?;
        if !is_fresh(Some(entry.at), now) || !prices_live {
            return None;
        }
        let held = prices_league?;
        if held.eq_ignore_ascii_case(league) || held.eq_ignore_ascii_case(&entry.name) {
            Some(entry.name.clone())
        } else {
            None
        }
    }

    pub fn record_sync(&mut self, requested: &str, returned: &str, now: u64) {
        let name = if returned.is_empty() {
            requested
        } else {
            returned
        };
        self.sync.insert(
            requested.to_lowercase(),
            SyncEntry {
                at: now,
                name: name.to_string(),
            },
        );
    }

    pub fn cached_leagues(&self, now: u64) -> Option<Vec<EconomyLeague>> {
        if is_fresh(self.leagues_at, now) && !self.leagues.is_empty() {
            Some(self.leagues.clone())
        } else {
            None
        }
    }

    pub fn record_leagues(&mut self, list: &[EconomyLeague], now: u64) {
        if list.is_empty() {
            return;
        }
        self.leagues = list.iter().take(MAX_LEAGUES).cloned().collect();
        self.leagues_at = Some(now);
    }

    pub fn cached_update(&self, app_version: &str, now: u64) -> Option<CheckResult> {
        if !is_fresh(self.update_at, now) || self.update_app_version.as_deref() != Some(app_version)
        {
            return None;
        }
        let version = self.update_version.clone()?;
        Some(CheckResult::UpToDate {
            version,
            released: self.update_released,
        })
    }

    /// Records only an `UpToDate` result; returns whether it was recorded.
    pub fn record_update(&mut self, result: &CheckResult, app_version: &str, now: u64) -> bool {
        let CheckResult::UpToDate { version, released } = result else {
            return false;
        };
        self.update_at = Some(now);
        self.update_app_version = Some(app_version.to_string());
        self.update_version = Some(version.clone());
        self.update_released = *released;
        true
    }

    /// Drop sync entries that are no longer fresh.
    pub fn prune(&mut self, now: u64) {
        self.sync.retain(|_, e| is_fresh(Some(e.at), now));
    }
}

/// Load the log; any read or parse problem is an empty log.
pub fn load(root: &Path) -> RequestLog {
    std::fs::read_to_string(root.join(FILE_NAME))
        .ok()
        .and_then(|t| serde_json::from_str(&t).ok())
        .unwrap_or_default()
}

/// Persist the log atomically (same-directory temp file, then replace).
pub fn save(root: &Path, log: &RequestLog) -> io::Result<()> {
    let mut bytes = serde_json::to_vec_pretty(log).map_err(io::Error::other)?;
    bytes.push(b'\n');
    std::fs::create_dir_all(root)?;
    let mut t = tempfile::NamedTempFile::new_in(root)?;
    t.write_all(&bytes)?;
    t.as_file().sync_all()?;
    t.persist(root.join(FILE_NAME)).map_err(|e| e.error)?;
    Ok(())
}

/// Current unix seconds (0 when the clock is before the epoch). Production call sites only.
pub fn now_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::update::Failure;

    fn league(name: &str) -> EconomyLeague {
        EconomyLeague {
            name: name.into(),
            url: name.to_lowercase(),
            hardcore: false,
            indexed: true,
        }
    }

    #[test]
    fn freshness_boundaries() {
        assert!(is_fresh(Some(1000), 1000));
        assert!(is_fresh(Some(1000), 4599));
        assert!(!is_fresh(Some(1000), 4600));
        assert!(!is_fresh(Some(5000), 1000));
        assert!(!is_fresh(None, 1000));
    }

    #[test]
    fn sync_fake_rules() {
        let mut log = RequestLog::default();
        let l = "Rise of the Abyssal";
        log.record_sync(l, l, 100);
        assert_eq!(
            log.sync_fake(l, Some("rise of the abyssal"), true, 110),
            Some(l.into())
        );
        assert_eq!(log.sync_fake("Other", Some("Other"), true, 110), None);
        assert_eq!(log.sync_fake(l, Some(l), false, 110), None);
        assert_eq!(log.sync_fake(l, None, true, 110), None);
        assert_eq!(log.sync_fake(l, Some("Standard"), true, 110), None);
        assert_eq!(log.sync_fake(l, Some(l), true, 100 + 3600), None);
    }

    #[test]
    fn sync_spelling_and_key_case() {
        let mut log = RequestLog::default();
        log.record_sync("rise of the abyssal", "Rise of the Abyssal", 100);
        assert_eq!(
            log.sync_fake(
                "rise of the abyssal",
                Some("Rise of the Abyssal"),
                true,
                101
            ),
            Some("Rise of the Abyssal".into())
        );
        log.record_sync("Standard", "Standard", 100);
        assert!(
            log.sync_fake("standard", Some("Standard"), true, 101)
                .is_some()
        );
        assert_eq!(log.sync.len(), 2);
        log.record_sync("X", "", 100);
        assert_eq!(log.sync["x"].name, "X");
    }

    #[test]
    fn leagues_cache() {
        let mut log = RequestLog::default();
        log.record_leagues(&[], 100);
        assert_eq!(log.leagues_at, None);
        assert!(log.cached_leagues(100).is_none());
        log.record_leagues(&[league("A"), league("B")], 100);
        assert_eq!(log.cached_leagues(3699).unwrap().len(), 2);
        assert!(log.cached_leagues(3700).is_none());
    }

    #[test]
    fn leagues_capped() {
        let many: Vec<_> = (0..500).map(|i| league(&format!("L{i}"))).collect();
        let mut log = RequestLog::default();
        log.record_leagues(&many, 1);
        assert_eq!(log.leagues.len(), 200);
    }

    #[test]
    fn update_only_up_to_date() {
        let mut log = RequestLog::default();
        let ok = CheckResult::UpToDate {
            version: "0.7.2".into(),
            released: Some(1_700_000_000),
        };
        assert!(log.record_update(&ok, "0.7.2", 100));
        assert_eq!(log.cached_update("0.7.2", 105), Some(ok.clone()));
        assert_eq!(log.cached_update("0.7.3", 105), None);
        assert_eq!(log.cached_update("0.7.2", 3700), None);

        let mut fresh = RequestLog::default();
        assert!(!fresh.record_update(&CheckResult::Failed(Failure::BadResponse), "0.7.2", 100));
        assert_eq!(fresh, RequestLog::default());
    }

    #[test]
    fn save_load_round_trip() {
        let dir = tempfile::tempdir().unwrap();
        let mut log = RequestLog::default();
        log.record_sync("A", "A", 10);
        log.record_sync("B", "b", 20);
        log.record_leagues(&[league("A"), league("B")], 30);
        log.record_update(
            &CheckResult::UpToDate {
                version: "1.0.0".into(),
                released: None,
            },
            "1.0.0",
            40,
        );
        save(dir.path(), &log).unwrap();
        assert!(dir.path().join("ff-state.json").is_file());
        assert_eq!(load(dir.path()), log);
    }

    #[test]
    fn bad_files_load_default() {
        let dir = tempfile::tempdir().unwrap();
        assert_eq!(load(dir.path()), RequestLog::default());
        for text in ["not json", "{\"sync\": 5}", "", "{\"extra\": 1}"] {
            std::fs::write(dir.path().join(FILE_NAME), text).unwrap();
            assert_eq!(load(dir.path()), RequestLog::default(), "{text}");
        }
    }

    #[test]
    fn prune_drops_stale() {
        let mut log = RequestLog::default();
        log.record_sync("old", "old", 0);
        log.record_sync("new", "new", 5000);
        log.prune(5001);
        assert_eq!(log.sync.keys().collect::<Vec<_>>(), vec!["new"]);
    }
}
