//! The Economy tab's league list and price-source model.
//!
//! **Framework-free by contract** (same as `overlay::economy`): no egui type appears here. The
//! league list is fetched on a worker thread, only on the first visit to the Economy tab in a
//! session and on refresh, never at start-up, in tests or in screenshot mode (the caller enforces
//! that). One pure function, [`view`], decides the whole league UI so the view only draws it.

// The view and app wiring that consume this model land in the next plans.
#![allow(dead_code)]

use std::sync::mpsc::{self, Receiver, TryRecvError};

use crate::compile::PriceSourcePolicy;
use crate::sync::league::default_league;
use crate::sync::models::EconomyLeague;

pub(crate) const TIP_LEAGUE: &str = "The league Sync economy prices against.";
pub(crate) const TIP_LOADING: &str = "Loading the league list.";
pub(crate) const TIP_FAILED: &str =
    "The league list could not be loaded. Press refresh to try again.";
pub(crate) const TIP_PICK: &str = "Pick a league.";
pub(crate) const TIP_PICK_FIRST: &str = "Pick a league first.";
pub(crate) const TIP_PRESS_REFRESH: &str = "Press refresh to load the league list.";
pub(crate) const TIP_FALLBACK: &str = "Not used with Fallback prices.";
pub(crate) const TIP_REFRESH: &str = "Reload the league list";
pub(crate) const CAPTION_LOADING: &str = "Loading…";
pub(crate) const CAPTION_NO_LEAGUE: &str = "(no league)";
pub(crate) const WORKER_STOPPED: &str = "the league worker stopped unexpectedly";

/// Caption of a saved league that is no longer listed.
pub(crate) fn ended_caption(name: &str) -> String {
    format!("{name} (ended)")
}

/// Tooltip of the ended caption.
pub(crate) fn ended_tip(name: &str) -> String {
    format!("{name} is no longer on poe.ninja.")
}

/// Status line shown when the saved league has ended.
pub(crate) fn ended_status(name: &str) -> String {
    format!("{name} is no longer on poe.ninja; pick a league")
}

/// Where the league list fetch stands.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum ListState {
    NotStarted,
    Loading,
    Loaded(Vec<EconomyLeague>),
    Failed(String),
}

type Fetched = Result<Vec<EconomyLeague>, String>;

/// Owns the league list state and the worker's result channel.
pub(crate) struct LeagueList {
    state: ListState,
    rx: Option<Receiver<Fetched>>,
}

impl Default for LeagueList {
    fn default() -> Self {
        Self {
            state: ListState::NotStarted,
            rx: None,
        }
    }
}

impl LeagueList {
    /// Runs `work` on a worker thread. A no-op while a fetch is running. A previously loaded
    /// list stays visible during the refresh (OD-2).
    pub(crate) fn start<F>(&mut self, work: F)
    where
        F: FnOnce() -> Fetched + Send + 'static,
    {
        if self.rx.is_some() {
            return;
        }
        let (tx, rx) = mpsc::channel();
        std::thread::spawn(move || {
            let _ = tx.send(work());
        });
        self.rx = Some(rx);
        if !matches!(self.state, ListState::Loaded(_)) {
            self.state = ListState::Loading;
        }
    }

    /// Call once per frame. Returns true when the state changed.
    pub(crate) fn poll(&mut self) -> bool {
        let Some(rx) = &self.rx else {
            return false;
        };
        let next = match rx.try_recv() {
            Ok(Ok(list)) => ListState::Loaded(list),
            Ok(Err(e)) => ListState::Failed(e),
            Err(TryRecvError::Empty) => return false,
            Err(TryRecvError::Disconnected) => ListState::Failed(WORKER_STOPPED.to_string()),
        };
        self.rx = None;
        self.state = next;
        true
    }

    pub(crate) fn state(&self) -> &ListState {
        &self.state
    }

    pub(crate) fn is_fetching(&self) -> bool {
        self.rx.is_some()
    }

    /// Sets the state directly (screenshot mode and tests); drops any receiver.
    pub(crate) fn inject(&mut self, state: ListState) {
        self.state = state;
        self.rx = None;
    }
}

/// How the league caption is coloured; the view maps it to a palette entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CaptionKind {
    Value,
    Disabled,
    Warn,
}

/// Everything [`view`] decides from.
pub(crate) struct ViewInputs<'a> {
    pub saved: Option<&'a str>,
    pub state: &'a ListState,
    pub fetching: bool,
    pub prices_league: Option<&'a str>,
    pub fallback: bool,
    pub syncing: bool,
}

/// The decided league UI.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct LeagueView {
    pub options: Vec<String>,
    pub selected: usize,
    pub caption: CaptionKind,
    pub dropdown_enabled: bool,
    pub dropdown_tip: String,
    pub refresh_enabled: bool,
    pub refresh_tip: String,
    pub sync_league: Option<String>,
    pub sync_block_tip: Option<&'static str>,
    pub ended: Option<String>,
    /// The real league name behind each option; None for synthetic entries.
    pub picks: Vec<Option<String>>,
}

impl LeagueView {
    /// The real league behind option `index`, None for synthetic or out-of-range entries.
    pub(crate) fn pick_name(&self, index: usize) -> Option<&str> {
        self.picks.get(index).and_then(|p| p.as_deref())
    }
}

fn single(name: &str, caption: CaptionKind) -> LeagueView {
    LeagueView {
        options: vec![name.to_string()],
        selected: 0,
        caption,
        dropdown_enabled: true,
        dropdown_tip: TIP_LEAGUE.to_string(),
        refresh_enabled: true,
        refresh_tip: TIP_REFRESH.to_string(),
        sync_league: None,
        sync_block_tip: None,
        ended: None,
        picks: vec![None],
    }
}

/// The whole league UI, per the UI-SPEC state table.
pub(crate) fn view(i: &ViewInputs) -> LeagueView {
    let mut v = match i.state {
        ListState::Loaded(list) => loaded_view(i.saved, list),
        ListState::NotStarted | ListState::Loading => match i.saved {
            Some(x) => {
                let mut v = single(x, CaptionKind::Value);
                v.picks = vec![Some(x.to_string())];
                v.dropdown_enabled = false;
                v.refresh_enabled = false;
                v.dropdown_tip = TIP_LOADING.to_string();
                v.refresh_tip = TIP_LOADING.to_string();
                v.sync_league = Some(x.to_string());
                v
            }
            None => {
                let mut v = single(CAPTION_LOADING, CaptionKind::Disabled);
                v.dropdown_enabled = false;
                v.refresh_enabled = false;
                v.dropdown_tip = TIP_LOADING.to_string();
                v.refresh_tip = TIP_LOADING.to_string();
                v.sync_block_tip = Some(TIP_LOADING);
                v
            }
        },
        ListState::Failed(_) => match i.saved.or(i.prices_league) {
            Some(x) => {
                let mut v = single(x, CaptionKind::Value);
                v.picks = vec![Some(x.to_string())];
                v.dropdown_tip = TIP_FAILED.to_string();
                v.sync_league = Some(x.to_string());
                v
            }
            None => {
                let mut v = single(CAPTION_NO_LEAGUE, CaptionKind::Disabled);
                v.dropdown_enabled = false;
                v.dropdown_tip = TIP_FAILED.to_string();
                v.sync_block_tip = Some(TIP_PRESS_REFRESH);
                v
            }
        },
    };

    if i.fetching && v.refresh_enabled {
        v.refresh_enabled = false;
        v.refresh_tip = TIP_LOADING.to_string();
    }
    if i.syncing {
        v.dropdown_enabled = false;
        v.refresh_enabled = false;
    }
    if i.fallback {
        v.caption = CaptionKind::Disabled;
        v.dropdown_enabled = false;
        v.refresh_enabled = false;
        v.dropdown_tip = TIP_FALLBACK.to_string();
        v.refresh_tip = TIP_FALLBACK.to_string();
        v.sync_league = None;
        v.sync_block_tip = Some(TIP_FALLBACK);
        v.ended = None;
    }
    v
}

fn loaded_view(saved: Option<&str>, list: &[EconomyLeague]) -> LeagueView {
    let names: Vec<String> = list.iter().map(|l| l.name.clone()).collect();
    let mut v = LeagueView {
        options: names.clone(),
        selected: 0,
        caption: CaptionKind::Value,
        dropdown_enabled: true,
        dropdown_tip: TIP_LEAGUE.to_string(),
        refresh_enabled: true,
        refresh_tip: TIP_REFRESH.to_string(),
        sync_league: None,
        sync_block_tip: None,
        ended: None,
        picks: names.iter().cloned().map(Some).collect(),
    };
    let current = match saved {
        Some(s) => {
            if let Some(idx) = names.iter().position(|n| n == s) {
                Some((idx, s.to_string()))
            } else {
                v.options.insert(0, ended_caption(s));
                v.picks.insert(0, None);
                v.caption = CaptionKind::Warn;
                v.dropdown_tip = ended_tip(s);
                v.sync_block_tip = Some(TIP_PICK_FIRST);
                v.ended = Some(s.to_string());
                return v;
            }
        }
        None => default_league(list).and_then(|d| {
            names
                .iter()
                .position(|n| *n == d.name)
                .map(|idx| (idx, d.name.clone()))
        }),
    };
    match current {
        Some((idx, name)) => {
            v.selected = idx;
            v.sync_league = Some(name);
        }
        None => {
            v.options.insert(0, CAPTION_NO_LEAGUE.to_string());
            v.picks.insert(0, None);
            v.caption = CaptionKind::Disabled;
            v.dropdown_tip = TIP_PICK.to_string();
            v.sync_block_tip = Some(TIP_PICK_FIRST);
        }
    }
    v
}

/// Whether the effective price source is the fallback pair.
pub(crate) fn effective_fallback(policy: PriceSourcePolicy, prices_live: bool) -> bool {
    policy == PriceSourcePolicy::ForceFallback || !prices_live
}

/// Whether switching the price source leaves the economy section out of date (OD-1: the cheap
/// effective-source comparison, not an output diff; assumption A2: the start-up policy matches
/// how the file was last written).
pub(crate) fn source_pending(
    applied: PriceSourcePolicy,
    now: PriceSourcePolicy,
    prices_live: bool,
    has_economy_section: bool,
) -> bool {
    has_economy_section
        && effective_fallback(applied, prices_live) != effective_fallback(now, prices_live)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sync::models::IndexState;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::Arc;
    use std::time::{Duration, Instant};

    const FIXTURE: &str = include_str!("../../tests/fixtures/index-state-2026-10-18.json");

    fn leagues() -> Vec<EconomyLeague> {
        serde_json::from_str::<IndexState>(FIXTURE)
            .unwrap()
            .economy_leagues
    }

    fn settle(list: &mut LeagueList) {
        let deadline = Instant::now() + Duration::from_secs(5);
        while !list.poll() {
            assert!(Instant::now() < deadline, "worker did not report");
            std::thread::sleep(Duration::from_millis(1));
        }
    }

    // ---- worker ----

    #[test]
    fn a_new_list_is_not_started() {
        let l = LeagueList::default();
        assert_eq!(l.state(), &ListState::NotStarted);
        assert!(!l.is_fetching());
    }

    #[test]
    fn start_loads_the_list() {
        let mut l = LeagueList::default();
        let expect = leagues();
        let sent = expect.clone();
        l.start(move || Ok(sent));
        assert_eq!(l.state(), &ListState::Loading);
        assert!(l.is_fetching());
        settle(&mut l);
        assert_eq!(l.state(), &ListState::Loaded(expect));
        assert!(!l.is_fetching());
    }

    #[test]
    fn a_failing_fetch_is_failed() {
        let mut l = LeagueList::default();
        l.start(|| Err("offline".into()));
        settle(&mut l);
        assert_eq!(l.state(), &ListState::Failed("offline".into()));
    }

    #[test]
    fn a_refresh_keeps_the_last_good_list_visible() {
        let mut l = LeagueList::default();
        let first = leagues();
        l.inject(ListState::Loaded(first.clone()));
        let second = vec![first[0].clone()];
        let sent = second.clone();
        l.start(move || {
            std::thread::sleep(Duration::from_millis(30));
            Ok(sent)
        });
        assert_eq!(l.state(), &ListState::Loaded(first));
        assert!(l.is_fetching());
        settle(&mut l);
        assert_eq!(l.state(), &ListState::Loaded(second));
    }

    #[test]
    fn a_failed_refresh_is_failed() {
        let mut l = LeagueList::default();
        l.inject(ListState::Loaded(leagues()));
        l.start(|| Err("down".into()));
        settle(&mut l);
        assert!(matches!(l.state(), ListState::Failed(_)));
    }

    #[test]
    fn start_while_fetching_is_a_no_op() {
        let mut l = LeagueList::default();
        l.start(|| {
            std::thread::sleep(Duration::from_millis(30));
            Ok(vec![])
        });
        let ran = Arc::new(AtomicBool::new(false));
        let r = ran.clone();
        l.start(move || {
            r.store(true, Ordering::SeqCst);
            Ok(vec![])
        });
        settle(&mut l);
        std::thread::sleep(Duration::from_millis(20));
        assert!(!ran.load(Ordering::SeqCst));
    }

    #[test]
    fn a_panicking_worker_is_failed() {
        let mut l = LeagueList::default();
        l.start(|| panic!("boom"));
        settle(&mut l);
        assert_eq!(l.state(), &ListState::Failed(WORKER_STOPPED.into()));
    }

    #[test]
    fn inject_sets_state_and_drops_the_receiver() {
        let mut l = LeagueList::default();
        l.start(|| {
            std::thread::sleep(Duration::from_millis(30));
            Ok(vec![])
        });
        l.inject(ListState::Failed("x".into()));
        assert_eq!(l.state(), &ListState::Failed("x".into()));
        assert!(!l.is_fetching());
    }

    // ---- view ----

    fn inputs<'a>(saved: Option<&'a str>, state: &'a ListState) -> ViewInputs<'a> {
        ViewInputs {
            saved,
            state,
            fetching: false,
            prices_league: None,
            fallback: false,
            syncing: false,
        }
    }

    fn names(v: &LeagueView) -> Vec<&str> {
        v.options.iter().map(String::as_str).collect()
    }

    #[test]
    fn loaded_with_nothing_saved_picks_the_default() {
        let st = ListState::Loaded(leagues());
        let v = view(&inputs(None, &st));
        assert_eq!(v.options.len(), 6);
        assert_eq!(v.selected, 0);
        assert_eq!(v.options[0], "Forbidden Rites");
        assert_eq!(v.caption, CaptionKind::Value);
        assert!(v.dropdown_enabled && v.refresh_enabled);
        assert_eq!(v.dropdown_tip, TIP_LEAGUE);
        assert_eq!(v.refresh_tip, TIP_REFRESH);
        assert_eq!(v.sync_league.as_deref(), Some("Forbidden Rites"));
        assert_eq!(v.sync_block_tip, None);
        assert_eq!(v.ended, None);
    }

    #[test]
    fn loaded_with_a_saved_league_selects_it() {
        let st = ListState::Loaded(leagues());
        let v = view(&inputs(Some("Runes of Aldur"), &st));
        assert_eq!(v.selected, 1);
        assert_eq!(v.sync_league.as_deref(), Some("Runes of Aldur"));
    }

    #[test]
    fn a_saved_league_missing_from_the_list_is_ended() {
        let st = ListState::Loaded(leagues());
        let v = view(&inputs(Some("Fate of the Vaal"), &st));
        assert_eq!(v.options.len(), 7);
        assert_eq!(v.options[0], "Fate of the Vaal (ended)");
        assert_eq!(v.options[1], "Forbidden Rites");
        assert_eq!(v.selected, 0);
        assert_eq!(v.caption, CaptionKind::Warn);
        assert!(v.dropdown_enabled);
        assert_eq!(
            v.dropdown_tip,
            "Fate of the Vaal is no longer on poe.ninja."
        );
        assert_eq!(v.sync_league, None);
        assert_eq!(v.sync_block_tip, Some("Pick a league first."));
        assert_eq!(v.ended.as_deref(), Some("Fate of the Vaal"));
        assert_eq!(v.pick_name(0), None);
        assert_eq!(v.pick_name(1), Some("Forbidden Rites"));
    }

    #[test]
    fn the_saved_name_match_is_case_sensitive() {
        let st = ListState::Loaded(leagues());
        let v = view(&inputs(Some("forbidden rites"), &st));
        assert_eq!(v.ended.as_deref(), Some("forbidden rites"));
    }

    #[test]
    fn loaded_without_a_default_shows_no_league() {
        let all = leagues();
        let st = ListState::Loaded(all[4..].to_vec());
        let v = view(&inputs(None, &st));
        assert_eq!(names(&v), ["(no league)", "Standard", "Hardcore"]);
        assert_eq!(v.caption, CaptionKind::Disabled);
        assert!(v.dropdown_enabled);
        assert_eq!(v.dropdown_tip, "Pick a league.");
        assert_eq!(v.sync_league, None);
        assert_eq!(v.sync_block_tip, Some("Pick a league first."));
        assert_eq!(v.pick_name(0), None);
    }

    #[test]
    fn pick_name_maps_real_entries() {
        let st = ListState::Loaded(leagues());
        let v = view(&inputs(None, &st));
        assert_eq!(v.pick_name(2), Some("HC Forbidden Rites"));
        assert_eq!(v.pick_name(99), None);
    }

    #[test]
    fn loading_and_not_started_with_a_saved_league() {
        for st in [ListState::Loading, ListState::NotStarted] {
            let v = view(&inputs(Some("X"), &st));
            assert_eq!(names(&v), ["X"]);
            assert_eq!(v.caption, CaptionKind::Value);
            assert!(!v.dropdown_enabled && !v.refresh_enabled);
            assert_eq!(v.dropdown_tip, TIP_LOADING);
            assert_eq!(v.refresh_tip, TIP_LOADING);
            assert_eq!(v.sync_league.as_deref(), Some("X"));
            assert_eq!(v.sync_block_tip, None);
            assert_eq!(v.ended, None);
        }
    }

    #[test]
    fn loading_with_nothing_saved() {
        let v = view(&inputs(None, &ListState::Loading));
        assert_eq!(names(&v), ["Loading…"]);
        assert_eq!(v.caption, CaptionKind::Disabled);
        assert!(!v.dropdown_enabled && !v.refresh_enabled);
        assert_eq!(v.sync_league, None);
        assert_eq!(v.sync_block_tip, Some(TIP_LOADING));
        assert_eq!(v.pick_name(0), None);
    }

    #[test]
    fn failed_with_a_saved_league_never_ends_it() {
        let st = ListState::Failed("e".into());
        let v = view(&inputs(Some("X"), &st));
        assert_eq!(names(&v), ["X"]);
        assert_eq!(v.caption, CaptionKind::Value);
        assert!(v.dropdown_enabled && v.refresh_enabled);
        assert_eq!(v.dropdown_tip, TIP_FAILED);
        assert_eq!(v.sync_league.as_deref(), Some("X"));
        assert_eq!(v.ended, None);
    }

    #[test]
    fn failed_falls_back_to_the_prices_league() {
        let st = ListState::Failed("e".into());
        let mut i = inputs(None, &st);
        i.prices_league = Some("Runes of Aldur");
        let v = view(&i);
        assert_eq!(names(&v), ["Runes of Aldur"]);
        assert!(v.dropdown_enabled);
        assert_eq!(v.sync_league.as_deref(), Some("Runes of Aldur"));
    }

    #[test]
    fn failed_with_nothing_known_blocks_sync() {
        let st = ListState::Failed("e".into());
        let v = view(&inputs(None, &st));
        assert_eq!(names(&v), ["(no league)"]);
        assert_eq!(v.caption, CaptionKind::Disabled);
        assert!(!v.dropdown_enabled && v.refresh_enabled);
        assert_eq!(v.sync_league, None);
        assert_eq!(
            v.sync_block_tip,
            Some("Press refresh to load the league list.")
        );
    }

    #[test]
    fn fallback_blocks_everything_in_every_state() {
        let states = [
            ListState::NotStarted,
            ListState::Loading,
            ListState::Loaded(leagues()),
            ListState::Failed("e".into()),
        ];
        for st in &states {
            for saved in [None, Some("X"), Some("Forbidden Rites")] {
                let plain = view(&inputs(saved, st));
                let mut i = inputs(saved, st);
                i.fallback = true;
                let v = view(&i);
                assert_eq!(v.options, plain.options);
                assert_eq!(v.selected, plain.selected);
                assert_eq!(v.caption, CaptionKind::Disabled);
                assert!(!v.dropdown_enabled && !v.refresh_enabled);
                assert_eq!(v.dropdown_tip, TIP_FALLBACK);
                assert_eq!(v.refresh_tip, TIP_FALLBACK);
                assert_eq!(v.sync_block_tip, Some(TIP_FALLBACK));
                assert_eq!(v.sync_league, None);
                assert_eq!(v.ended, None);
            }
        }
    }

    #[test]
    fn syncing_disables_the_controls_but_keeps_the_league() {
        let st = ListState::Loaded(leagues());
        let mut i = inputs(None, &st);
        i.syncing = true;
        let v = view(&i);
        assert!(!v.dropdown_enabled && !v.refresh_enabled);
        assert_eq!(v.sync_league.as_deref(), Some("Forbidden Rites"));
    }

    #[test]
    fn a_refresh_disables_only_the_refresh_button() {
        let st = ListState::Loaded(leagues());
        let mut i = inputs(None, &st);
        i.fetching = true;
        let v = view(&i);
        assert!(!v.refresh_enabled);
        assert_eq!(v.refresh_tip, TIP_LOADING);
        assert!(v.dropdown_enabled);
        assert_eq!(v.options.len(), 6);
    }

    // ---- price source ----

    #[test]
    fn effective_fallback_table() {
        use PriceSourcePolicy::{Auto, ForceFallback};
        assert!(!effective_fallback(Auto, true));
        assert!(effective_fallback(Auto, false));
        assert!(effective_fallback(ForceFallback, true));
    }

    #[test]
    fn source_pending_table() {
        use PriceSourcePolicy::{Auto, ForceFallback};
        assert!(source_pending(Auto, ForceFallback, true, true));
        assert!(!source_pending(Auto, ForceFallback, true, false));
        assert!(!source_pending(Auto, ForceFallback, false, true));
        assert!(!source_pending(ForceFallback, ForceFallback, true, true));
        assert!(source_pending(ForceFallback, Auto, true, true));
    }
}
