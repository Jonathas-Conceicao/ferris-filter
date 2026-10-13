//! RELOAD-01 / Phase 23: after a successful RULES Apply, send `/reloaditemfilter` to a running
//! PoE2. That includes an Apply whose bytes equal the file on disk (not rewritten, but still
//! applied) and an OVERWRITE after a changed-on-disk refusal.
//!
//! One Apply click runs one send sequence: it is never repeated, scheduled, retried or queued
//! (CONTEXT "ToS posture"). The mechanism is copied from Exiled Exchange 2 / Awakened PoE Trade
//! (`main/src/shortcuts/text-box.ts` and `HostClipboard.ts`): put the command on the clipboard,
//! Enter, paste, Enter, then restore the clipboard about 120 ms later. As in EE2 for text starting
//! with `/` (its `AUTO_CLEAR` list), there is no select-all before the paste: a typed `/` command
//! needs no cleared prefix, and a synthetic Ctrl+A was caught as a global hotkey by another tool
//! the author runs. The trailing "restore last chat" keys are not copied.
//! Escape is never sent (it can open the game menu).
//!
//! **Known limitation (review CR-02):** nothing outside the game can tell whether its chat box is
//! already open, and EE2 / Awakened PoE Trade share this: they also press Enter first. If the chat
//! box was already open with text in it when Apply was pressed, the first Enter sends that text as
//! a chat message, the chat box closes, the select-all and paste go to the game world, the final
//! Enter opens an empty chat box, and `/reloaditemfilter` is not sent (the status still says
//! "Reloaded in game", because the keys were delivered). No key is pressed before the first Enter on purpose: with chat closed, an
//! Escape, Backspace or Ctrl+A would reach the game's own key bindings. The user is told to close
//! the chat box before pressing Apply (CONFIG caption, CLAUDE.md, manual check 7).
//!
//! This file is pure (no egui, no Win32): the production sink lives in plan 23-03 and calls only
//! the safe wrappers in `win.rs`. Tests drive everything through fake sinks.

use std::sync::mpsc::{Receiver, TryRecvError};
use std::time::Duration;

use super::apply::{self, ApplyOutcome};
use super::win;

/// The chat command sent to the game. A compile-time constant: no setting or UI value can reach
/// the clipboard text.
pub const RELOAD_COMMAND: &str = "/reloaditemfilter";

/// Time for the game window to settle after being focused.
const FOCUS_SETTLE_MS: u64 = 60;
/// Time for the chat box to open after Enter.
const CHAT_OPEN_MS: u64 = 60;
/// Time for the pasted text to land before the final Enter.
const PASTE_SETTLE_MS: u64 = 40;
/// Delay before the clipboard is restored (EE2's `RESTORE_AFTER = 120`). These four waits are
/// tuned in manual testing (RESEARCH assumption A8).
const RESTORE_AFTER_MS: u64 = 120;

/// The only keys the reload sequence ever presses. There is deliberately no Escape.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Key {
    Enter,
    Ctrl,
    V,
}

impl Key {
    /// Windows virtual-key code.
    pub fn vk(self) -> u16 {
        match self {
            Key::Enter => 0x0D,
            Key::Ctrl => 0x11,
            Key::V => 0x56,
        }
    }
}

/// One step of the reload plan.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Step {
    /// Bring the game window to the foreground.
    FocusGame,
    /// Fail unless the game is the foreground window.
    VerifyForeground,
    /// Fail if the user is holding a modifier key, polling briefly so a key being released as the
    /// click lands is not a failure. Runs once, before focus moves.
    VerifyModifiersUp,
    /// Fail at once, without waiting, if the user is holding a modifier key. Runs right before
    /// each key group, after the game has focus (review iteration 2, WR-01).
    VerifyModifiersStillUp,
    /// Remember the current clipboard text.
    SaveClipboard,
    /// Put this text on the clipboard.
    SetClipboard(String),
    /// Press and release one key.
    Tap(Key),
    /// Press `.0`, tap `.1`, release `.0`.
    Chord(Key, Key),
    /// Sleep this many milliseconds.
    Wait(u64),
    /// Put the saved clipboard back.
    RestoreClipboard,
}

/// Executes steps against the real system or a test double.
pub trait InputSink {
    /// Run one step; an `Err` aborts the plan.
    fn run(&mut self, step: &Step) -> Result<(), String>;
}

/// The exact reload plan. A foreground check precedes the clipboard write and every key group, so
/// no key can land in another application. The modifier check runs at two points:
///
/// - once first, before any side effect (polling briefly): it reads global key state and does not
///   need the game focused, so a modifier held at click time aborts while the stripe still has
///   focus and can show why (review WR-03);
/// - again, without waiting, right before each key group (the first Enter, the Ctrl+V paste,
///   the final Enter). The game has just come to the front by then, and holding Alt to show
///   ground labels is a reflex. A modifier pressed in between would turn Enter into Alt+Enter and
///   the paste into Ctrl+Alt+V, and the chord's Ctrl key-up would release a Ctrl the
///   user holds (review iteration 2, WR-01). A held modifier there aborts with the same reason.
pub fn reload_steps() -> Vec<Step> {
    vec![
        Step::VerifyModifiersUp,
        Step::FocusGame,
        Step::Wait(FOCUS_SETTLE_MS),
        Step::VerifyForeground,
        Step::SaveClipboard,
        Step::SetClipboard(RELOAD_COMMAND.to_string()),
        Step::VerifyForeground,
        Step::VerifyModifiersStillUp,
        Step::Tap(Key::Enter),
        Step::Wait(CHAT_OPEN_MS),
        Step::VerifyForeground,
        Step::VerifyModifiersStillUp,
        Step::Chord(Key::Ctrl, Key::V),
        Step::Wait(PASTE_SETTLE_MS),
        Step::VerifyForeground,
        Step::VerifyModifiersStillUp,
        Step::Tap(Key::Enter),
        Step::Wait(RESTORE_AFTER_MS),
        Step::RestoreClipboard,
    ]
}

/// Run `steps` through `sink`, stopping at the first failed step. If the clipboard was saved and
/// not yet restored, it is restored exactly once before the error is returned. A failure of the
/// final restore never turns a sent command into an error.
pub fn execute(sink: &mut dyn InputSink, steps: &[Step]) -> Result<(), String> {
    let mut saved = false;
    let mut restored = false;
    for step in steps {
        if *step == Step::RestoreClipboard {
            let _ = sink.run(step);
            restored = true;
            continue;
        }
        match sink.run(step) {
            Ok(()) => {
                if *step == Step::SaveClipboard {
                    saved = true;
                }
            }
            Err(e) => {
                if saved && !restored {
                    let _ = sink.run(&Step::RestoreClipboard);
                }
                return Err(e);
            }
        }
    }
    Ok(())
}

/// Why no reload was sent.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SkipReason {
    SettingOff,
    Busy,
    GameNotRunning,
    /// The stripe's own window handle is not cached, so there is no clipboard owner.
    NoWindow,
}

impl SkipReason {
    /// The ASCII reason shown in the status line.
    pub fn text(self) -> &'static str {
        match self {
            SkipReason::SettingOff => apply::REASON_SETTING_OFF,
            SkipReason::Busy => apply::REASON_BUSY,
            SkipReason::GameNotRunning => apply::REASON_GAME_NOT_RUNNING,
            SkipReason::NoWindow => apply::REASON_NO_WINDOW,
        }
    }
}

/// The trigger decision for a finished Apply.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Decision<G> {
    /// Send the reload to this game window.
    Send(G),
    /// Do not send, for this reason.
    Skip(SkipReason),
}

/// Decide whether an Apply outcome triggers a reload. Every `Written` can, identical bytes
/// included; `Failed` produces no decision at all. `find_game` runs only when everything else
/// says send.
pub fn decide<G>(
    outcome: &ApplyOutcome,
    setting_on: bool,
    in_flight: bool,
    find_game: impl FnOnce() -> Option<G>,
) -> Option<Decision<G>> {
    if !matches!(outcome, ApplyOutcome::Written(_)) {
        return None;
    }
    if !setting_on {
        return Some(Decision::Skip(SkipReason::SettingOff));
    }
    if in_flight {
        return Some(Decision::Skip(SkipReason::Busy));
    }
    Some(match find_game() {
        Some(g) => Decision::Send(g),
        None => Decision::Skip(SkipReason::GameNotRunning),
    })
}

/// Pair a `Send` decision with the clipboard owner window. Without one the send is skipped
/// ([`SkipReason::NoWindow`]): an ownerless `EmptyClipboard` makes the following clipboard write
/// fail, which would lose the user's clipboard (review WR-02). Every other decision passes
/// through unchanged.
pub fn with_owner<G, O>(
    decision: Option<Decision<G>>,
    owner: Option<O>,
) -> Option<Decision<(G, O)>> {
    match decision? {
        Decision::Send(game) => Some(match owner {
            Some(o) => Decision::Send((game, o)),
            None => Decision::Skip(SkipReason::NoWindow),
        }),
        Decision::Skip(reason) => Some(Decision::Skip(reason)),
    }
}

/// The failure reason for a held modifier, from either modifier check.
pub const MODIFIER_HELD: &str = "a modifier key (Ctrl, Shift or Alt) was held down";

/// Reported when the worker dies without sending a result.
pub const WORKER_STOPPED: &str = "the reload worker stopped unexpectedly";

/// Owns the single in-flight reload worker and its result channel.
#[derive(Debug, Default)]
pub struct ReloadSend {
    rx: Option<Receiver<Result<(), String>>>,
}

impl ReloadSend {
    /// Start `work` on a worker thread, then call `notify` once the result is ready. Returns
    /// `false` without running anything if a reload is already in flight: that guard is the
    /// "one click, one command" rule, and nothing is ever queued.
    pub fn start<W, N>(&mut self, work: W, notify: N) -> bool
    where
        W: FnOnce() -> Result<(), String> + Send + 'static,
        N: FnOnce() + Send + 'static,
    {
        if self.is_running() {
            return false;
        }
        let (tx, rx) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            // A send failure means the app shut down; dropping the result is correct.
            let _ = tx.send(work());
            notify();
        });
        self.rx = Some(rx);
        true
    }

    /// Whether a worker has not reported yet.
    pub fn is_running(&self) -> bool {
        self.rx.is_some()
    }

    /// Collect the result if it has landed. A worker that died without reporting yields
    /// `Err(WORKER_STOPPED)` instead of staying in flight forever.
    pub fn poll(&mut self) -> Option<Result<(), String>> {
        let rx = self.rx.as_ref()?;
        match rx.try_recv() {
            Ok(r) => {
                self.rx = None;
                Some(r)
            }
            Err(TryRecvError::Empty) => None,
            Err(TryRecvError::Disconnected) => {
                self.rx = None;
                Some(Err(WORKER_STOPPED.to_string()))
            }
        }
    }
}

/// How many times the modifier check polls, and how far apart.
const MODIFIER_POLLS: u32 = 12;
const MODIFIER_POLL_MS: u64 = 25;

/// The clipboard sequence number the restore guard compares against after a `SetClipboard`
/// step: ours on success. A failed write that had already emptied the clipboard still counts as
/// ours (the sequence right after the failure), so the saved text is put back instead of the
/// user's clipboard being lost (review WR-02). A write that failed before touching anything
/// leaves the clipboard alone (`None`).
fn ours_after_write(
    result: &Result<u32, win::ClipboardWriteError>,
    now: impl FnOnce() -> u32,
) -> Option<u32> {
    match result {
        Ok(seq) => Some(*seq),
        Err(e) if e.emptied => Some(now()),
        Err(_) => None,
    }
}

/// The production [`InputSink`] over the safe wrappers in `win.rs`. Never constructed in tests.
pub struct WinSink {
    game: win::GameWindow,
    owner: win::ClipboardOwner,
    /// The clipboard text before our write; `Some(None)` means it held no text.
    saved: Option<Option<String>>,
    /// The clipboard sequence number right after our write.
    ours: Option<u32>,
}

impl WinSink {
    /// `owner` is the stripe's own window; there is no ownerless fallback (an ownerless
    /// `EmptyClipboard` makes the following `SetClipboardData` fail), so the caller skips the
    /// send when no window is cached.
    pub fn new(game: win::GameWindow, owner: win::ClipboardOwner) -> Self {
        WinSink {
            game,
            owner,
            saved: None,
            ours: None,
        }
    }
}

impl InputSink for WinSink {
    fn run(&mut self, step: &Step) -> Result<(), String> {
        match step {
            Step::FocusGame => win::focus_game(&self.game),
            Step::VerifyForeground => {
                if win::game_is_foreground(&self.game) {
                    Ok(())
                } else {
                    Err("the game window did not take focus".to_string())
                }
            }
            Step::VerifyModifiersUp => {
                for i in 0..MODIFIER_POLLS {
                    if !win::modifiers_held() {
                        return Ok(());
                    }
                    if i + 1 < MODIFIER_POLLS {
                        std::thread::sleep(Duration::from_millis(MODIFIER_POLL_MS));
                    }
                }
                Err(MODIFIER_HELD.to_string())
            }
            Step::VerifyModifiersStillUp => {
                if win::modifiers_held() {
                    Err(MODIFIER_HELD.to_string())
                } else {
                    Ok(())
                }
            }
            Step::SaveClipboard => {
                self.saved = Some(win::read_clipboard_text(self.owner)?);
                Ok(())
            }
            Step::SetClipboard(text) => {
                let result = win::write_clipboard_text(self.owner, text);
                self.ours = ours_after_write(&result, win::clipboard_sequence);
                result.map(|_| ()).map_err(|e| e.message)
            }
            Step::Tap(k) => win::send_tap(k.vk()),
            Step::Chord(m, k) => win::send_chord(m.vk(), k.vk()),
            Step::Wait(ms) => {
                std::thread::sleep(Duration::from_millis(*ms));
                Ok(())
            }
            Step::RestoreClipboard => {
                let saved = self.saved.take();
                let ours = self.ours.take();
                // Only when nothing else wrote the clipboard since our write (or our failed write
                // that emptied it); otherwise leave whatever the user copied meanwhile alone. The
                // saved text is never logged.
                if ours == Some(win::clipboard_sequence()) {
                    match saved {
                        Some(Some(text)) => {
                            win::write_clipboard_text(self.owner, &text).map_err(|e| e.message)?;
                        }
                        Some(None) => win::clear_clipboard(self.owner)?,
                        None => {}
                    }
                }
                Ok(())
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

    #[test]
    fn plan_is_exact() {
        use Key::*;
        assert_eq!(
            reload_steps(),
            vec![
                Step::VerifyModifiersUp,
                Step::FocusGame,
                Step::Wait(60),
                Step::VerifyForeground,
                Step::SaveClipboard,
                Step::SetClipboard(RELOAD_COMMAND.to_string()),
                Step::VerifyForeground,
                Step::VerifyModifiersStillUp,
                Step::Tap(Enter),
                Step::Wait(60),
                Step::VerifyForeground,
                Step::VerifyModifiersStillUp,
                Step::Chord(Ctrl, V),
                Step::Wait(40),
                Step::VerifyForeground,
                Step::VerifyModifiersStillUp,
                Step::Tap(Enter),
                Step::Wait(120),
                Step::RestoreClipboard,
            ]
        );
    }

    #[test]
    fn plan_invariants() {
        let steps = reload_steps();
        assert_eq!(RELOAD_COMMAND, "/reloaditemfilter");
        assert_eq!(
            steps
                .iter()
                .filter(|s| **s == Step::Tap(Key::Enter))
                .count(),
            2
        );
        assert_eq!(
            steps.iter().filter(|s| matches!(s, Step::Tap(_))).count(),
            2
        );
        // The paste is the only chord: no select-all, which another tool may own as a global
        // hotkey (EE2 also skips it for `/` commands).
        let chords: Vec<usize> = steps
            .iter()
            .enumerate()
            .filter(|(_, s)| matches!(s, Step::Chord(..)))
            .map(|(i, _)| i)
            .collect();
        assert_eq!(chords.len(), 1);
        let v = chords[0];
        assert_eq!(steps[v], Step::Chord(Key::Ctrl, Key::V));
        // Review WR-03: the polling modifier check runs before any side effect, focus included,
        // and only there (polling once the game has focus would only delay the failure).
        assert_eq!(steps[0], Step::VerifyModifiersUp);
        assert_eq!(
            steps
                .iter()
                .filter(|s| **s == Step::VerifyModifiersUp)
                .count(),
            1
        );
        let first_verify = steps
            .iter()
            .position(|s| *s == Step::VerifyForeground)
            .unwrap();
        for (i, s) in steps.iter().enumerate() {
            if matches!(s, Step::Tap(_) | Step::Chord(..) | Step::SetClipboard(_)) {
                assert!(i > first_verify);
            }
            // Every key group (each tap, and the Ctrl+V paste) is
            // directly preceded, with no wait in between, by the immediate modifier check, and
            // that by a foreground check (review iteration 2, WR-01). SetClipboard is guarded by
            // the earlier foreground check and the first modifier check.
            if matches!(s, Step::Tap(_) | Step::Chord(..)) {
                assert_eq!(steps[i - 1], Step::VerifyModifiersStillUp, "step {i}");
                assert_eq!(steps[i - 2], Step::VerifyForeground, "step {i}");
            }
            // The immediate check is used only right before a key group.
            if *s == Step::VerifyModifiersStillUp {
                assert!(
                    matches!(steps[i + 1], Step::Tap(_) | Step::Chord(..)),
                    "step {i}"
                );
            }
        }
        assert_eq!(
            steps
                .iter()
                .filter(|s| **s == Step::VerifyModifiersStillUp)
                .count(),
            3
        );
        let total: u64 = steps
            .iter()
            .filter_map(|s| {
                if let Step::Wait(n) = s {
                    Some(*n)
                } else {
                    None
                }
            })
            .sum();
        assert!(total <= 500);
        assert_eq!(Key::Enter.vk(), 0x0D);
        assert_eq!(Key::Ctrl.vk(), 0x11);
        assert_eq!(Key::V.vk(), 0x56);
    }

    #[derive(Default)]
    struct RecordingSink {
        calls: Vec<Step>,
    }
    impl InputSink for RecordingSink {
        fn run(&mut self, step: &Step) -> Result<(), String> {
            self.calls.push(step.clone());
            Ok(())
        }
    }

    /// Fails the n-th call (0-based).
    struct FailingSink {
        fail_at: usize,
        calls: Vec<Step>,
    }
    impl InputSink for FailingSink {
        fn run(&mut self, step: &Step) -> Result<(), String> {
            let n = self.calls.len();
            self.calls.push(step.clone());
            if n == self.fail_at {
                Err("boom".to_string())
            } else {
                Ok(())
            }
        }
    }

    fn restores(calls: &[Step]) -> usize {
        calls
            .iter()
            .filter(|s| **s == Step::RestoreClipboard)
            .count()
    }

    fn sent_input(calls: &[Step]) -> bool {
        calls
            .iter()
            .any(|s| matches!(s, Step::Tap(_) | Step::Chord(..) | Step::SetClipboard(_)))
    }

    #[test]
    fn success_runs_every_step() {
        let mut sink = RecordingSink::default();
        assert_eq!(execute(&mut sink, &reload_steps()), Ok(()));
        assert_eq!(sink.calls, reload_steps());
    }

    #[test]
    fn modifier_failure_happens_before_focus_moves() {
        // A held modifier is the first step's failure: the game is never focused, so the stripe
        // keeps focus and shows the reason (review WR-03).
        let mut sink = FailingSink {
            fail_at: 0,
            calls: vec![],
        };
        assert_eq!(execute(&mut sink, &reload_steps()), Err("boom".into()));
        assert_eq!(sink.calls, vec![Step::VerifyModifiersUp]);
        assert!(!sink.calls.contains(&Step::FocusGame));
    }

    #[test]
    fn first_verify_failure_sends_nothing_and_never_saved() {
        let at = reload_steps()
            .iter()
            .position(|s| *s == Step::VerifyForeground)
            .unwrap();
        let mut sink = FailingSink {
            fail_at: at,
            calls: vec![],
        };
        assert_eq!(execute(&mut sink, &reload_steps()), Err("boom".into()));
        assert_eq!(sink.calls.last(), Some(&Step::VerifyForeground));
        assert!(!sent_input(&sink.calls));
        assert_eq!(restores(&sink.calls), 0);
    }

    #[test]
    fn second_verify_failure_restores_once_and_stops() {
        let mut sink = FailingSink {
            fail_at: 6,
            calls: vec![],
        };
        assert_eq!(execute(&mut sink, &reload_steps()), Err("boom".into()));
        assert!(
            !sink
                .calls
                .iter()
                .any(|s| matches!(s, Step::Tap(_) | Step::Chord(..)))
        );
        assert_eq!(restores(&sink.calls), 1);
        assert_eq!(sink.calls.last(), Some(&Step::RestoreClipboard));
    }

    #[test]
    fn first_tap_failure_restores_once() {
        let at = reload_steps()
            .iter()
            .position(|s| *s == Step::Tap(Key::Enter))
            .unwrap();
        let mut sink = FailingSink {
            fail_at: at,
            calls: vec![],
        };
        assert_eq!(execute(&mut sink, &reload_steps()), Err("boom".into()));
        assert_eq!(restores(&sink.calls), 1);
        assert_eq!(sink.calls.last(), Some(&Step::RestoreClipboard));
        assert!(!sink.calls.iter().any(|s| matches!(s, Step::Chord(..))));
    }

    #[test]
    fn a_modifier_pressed_mid_sequence_aborts_before_the_next_key_group() {
        // Review iteration 2, WR-01: a modifier pressed after the game took focus fails the
        // immediate check in front of the next key group. No further key goes out, and the
        // clipboard is restored exactly once.
        let steps = reload_steps();
        let checks: Vec<usize> = steps
            .iter()
            .enumerate()
            .filter(|(_, s)| **s == Step::VerifyModifiersStillUp)
            .map(|(i, _)| i)
            .collect();
        assert_eq!(checks.len(), 3);
        for at in checks {
            let mut sink = FailingSink {
                fail_at: at,
                calls: vec![],
            };
            assert_eq!(execute(&mut sink, &steps), Err("boom".into()));
            // Everything up to the check ran; the key group after it never did.
            assert_eq!(sink.calls[..=at], steps[..=at]);
            assert_eq!(sink.calls.len(), at + 2);
            assert_eq!(sink.calls.last(), Some(&Step::RestoreClipboard));
            assert_eq!(restores(&sink.calls), 1);
            let keys = |s: &[Step]| {
                s.iter()
                    .filter(|s| matches!(s, Step::Tap(_) | Step::Chord(..)))
                    .count()
            };
            assert_eq!(keys(&sink.calls), keys(&steps[..at]));
        }
    }

    #[test]
    fn unreadable_clipboard_aborts_before_anything_is_written() {
        // Review WR-05: a clipboard whose text cannot be read fails SaveClipboard, and the send
        // stops there: no write, no key, and no restore that could empty the clipboard.
        let at = reload_steps()
            .iter()
            .position(|s| *s == Step::SaveClipboard)
            .unwrap();
        let mut sink = FailingSink {
            fail_at: at,
            calls: vec![],
        };
        assert_eq!(execute(&mut sink, &reload_steps()), Err("boom".into()));
        assert_eq!(sink.calls.last(), Some(&Step::SaveClipboard));
        assert!(!sent_input(&sink.calls));
        assert_eq!(restores(&sink.calls), 0);
    }

    #[test]
    fn clipboard_write_failure_restores_once_and_sends_no_key() {
        let at = reload_steps()
            .iter()
            .position(|s| matches!(s, Step::SetClipboard(_)))
            .unwrap();
        let mut sink = FailingSink {
            fail_at: at,
            calls: vec![],
        };
        assert_eq!(execute(&mut sink, &reload_steps()), Err("boom".into()));
        assert_eq!(restores(&sink.calls), 1);
        assert_eq!(sink.calls.last(), Some(&Step::RestoreClipboard));
        assert!(
            !sink
                .calls
                .iter()
                .any(|s| matches!(s, Step::Tap(_) | Step::Chord(..)))
        );
    }

    #[test]
    fn a_write_that_emptied_the_clipboard_still_counts_as_ours() {
        use win::ClipboardWriteError as E;
        // Success: the sequence the write returned; `now` is not consulted.
        assert_eq!(
            ours_after_write(&Ok(7), || panic!("not read on success")),
            Some(7)
        );
        // Emptied, then failed: the restore must run, so the guard gets the current sequence
        // (review WR-02: the user's clipboard must never be lost).
        assert_eq!(
            ours_after_write(&Err(E::emptied("could not set the clipboard")), || 9),
            Some(9)
        );
        // Failed before touching anything (busy): leave the clipboard alone.
        assert_eq!(
            ours_after_write(&Err(E::untouched("the clipboard is busy")), || 9),
            None
        );
    }

    #[test]
    fn restore_failure_does_not_fail_the_send() {
        // Fail the last call, which is the final RestoreClipboard.
        let mut sink = FailingSink {
            fail_at: reload_steps().len() - 1,
            calls: vec![],
        };
        assert_eq!(execute(&mut sink, &reload_steps()), Ok(()));
        assert_eq!(restores(&sink.calls), 1);
    }

    #[test]
    fn decide_truth_table() {
        // `Written` also covers an Apply whose bytes matched the file (not rewritten): it
        // decides exactly like a real write.
        let outcomes = [
            ApplyOutcome::Written(PathBuf::from("x.filter")),
            ApplyOutcome::Failed("x".into()),
        ];
        for outcome in &outcomes {
            for setting_on in [false, true] {
                for in_flight in [false, true] {
                    for game in [false, true] {
                        let calls = AtomicUsize::new(0);
                        let got = decide(outcome, setting_on, in_flight, || {
                            calls.fetch_add(1, Ordering::SeqCst);
                            game.then_some(7u32)
                        });
                        let written = matches!(outcome, ApplyOutcome::Written(_));
                        let (want, want_calls) = if !written {
                            (None, 0)
                        } else if !setting_on {
                            (Some(Decision::Skip(SkipReason::SettingOff)), 0)
                        } else if in_flight {
                            (Some(Decision::Skip(SkipReason::Busy)), 0)
                        } else if !game {
                            (Some(Decision::Skip(SkipReason::GameNotRunning)), 1)
                        } else {
                            (Some(Decision::Send(7u32)), 1)
                        };
                        assert_eq!(got, want, "{outcome:?} {setting_on} {in_flight} {game}");
                        assert_eq!(calls.load(Ordering::SeqCst), want_calls);
                    }
                }
            }
        }
    }

    #[test]
    fn skip_reason_text() {
        assert_eq!(SkipReason::SettingOff.text(), apply::REASON_SETTING_OFF);
        assert_eq!(SkipReason::Busy.text(), apply::REASON_BUSY);
        assert_eq!(
            SkipReason::GameNotRunning.text(),
            apply::REASON_GAME_NOT_RUNNING
        );
        assert_eq!(SkipReason::NoWindow.text(), apply::REASON_NO_WINDOW);
        assert!(apply::REASON_NO_WINDOW.is_ascii());
    }

    #[test]
    fn a_send_without_a_clipboard_owner_is_skipped() {
        assert_eq!(
            with_owner(Some(Decision::Send(1u8)), Some('o')),
            Some(Decision::Send((1u8, 'o')))
        );
        // Review WR-02: never fall back to an ownerless clipboard.
        assert_eq!(
            with_owner(Some(Decision::Send(1u8)), None::<char>),
            Some(Decision::Skip(SkipReason::NoWindow))
        );
        for owner in [Some('o'), None] {
            assert_eq!(
                with_owner(Some(Decision::<u8>::Skip(SkipReason::SettingOff)), owner),
                Some(Decision::Skip(SkipReason::SettingOff))
            );
            assert_eq!(with_owner(None::<Decision<u8>>, owner), None);
        }
    }

    fn settle(r: &mut ReloadSend) -> Result<(), String> {
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
        while std::time::Instant::now() < deadline {
            if let Some(res) = r.poll() {
                return res;
            }
            std::thread::sleep(std::time::Duration::from_millis(1));
        }
        panic!("reload worker never reported within 5s");
    }

    #[test]
    fn worker_runs_once_and_notifies_once() {
        let mut r = ReloadSend::default();
        assert!(!r.is_running());
        assert_eq!(r.poll(), None);

        let (gate_tx, gate_rx) = std::sync::mpsc::channel::<()>();
        let notified = Arc::new(AtomicUsize::new(0));
        let n = notified.clone();
        assert!(r.start(
            move || {
                let _ = gate_rx.recv();
                Ok(())
            },
            move || {
                n.fetch_add(1, Ordering::SeqCst);
            }
        ));
        assert!(r.is_running());

        let ran = Arc::new(AtomicBool::new(false));
        let ran2 = ran.clone();
        assert!(!r.start(
            move || {
                ran2.store(true, Ordering::SeqCst);
                Ok(())
            },
            || {}
        ));

        gate_tx.send(()).unwrap();
        assert_eq!(settle(&mut r), Ok(()));
        assert!(!ran.load(Ordering::SeqCst));
        assert_eq!(r.poll(), None);
        assert!(!r.is_running());
        // notify runs right after the result is sent; give it a moment.
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
        while notified.load(Ordering::SeqCst) == 0 && std::time::Instant::now() < deadline {
            std::thread::sleep(std::time::Duration::from_millis(1));
        }
        assert_eq!(notified.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn worker_error_and_panic_are_reported() {
        let mut r = ReloadSend::default();
        assert!(r.start(|| Err("x".to_string()), || {}));
        assert_eq!(settle(&mut r), Err("x".to_string()));

        assert!(r.start(|| panic!("worker panic"), || {}));
        assert_eq!(settle(&mut r), Err(WORKER_STOPPED.to_string()));
        assert!(!r.is_running());
    }
}
