//! The pure show/hide decision for the stripe (D-05, D-06, D-07).
//!
//! This module has **no** `egui`, `eframe`, `winit` or `windows_sys` type in any signature —
//! per `PITFALLS.md` §8 ("if a bug can only be found by clicking, it is in the wrong layer"),
//! the decision of *whether* to show or hide is a value computed from two booleans, fully
//! testable without a window. `app.rs` is the thin view: it reads the OS focus flag, calls
//! [`advance`], asks [`effect_for`] what to do about the transition, and performs that one
//! effect via `win::reveal`/`win::conceal`. No branching logic belongs in `app.rs` itself.
//!
//! ## Why `ShownUnfocused` exists as a distinct state (Pitfall 2)
//!
//! RESEARCH found that a window revealed with `SW_SHOWNOACTIVATE` (the only way to satisfy
//! D-05 across repeat summons — see `win.rs`) has **never been focused**. Its OS-reported
//! `focused` flag reads `false` (or unset) from the instant it appears. A naive rule of
//! "hide whenever `focused == false`" would therefore hide the stripe on the very first frame
//! after every reveal — it would flash and vanish before the player could ever click into it.
//!
//! `ShownUnfocused` is the state that absorbs this: it tolerates any number of consecutive
//! unfocused frames without hiding, because it has not yet observed a focus gain to lose.
//! Only after the OS reports `focused == true` at least once (the user clicked in) does the
//! machine move to `ShownFocused`, at which point losing focus really does mean "the user
//! clicked away" (D-06) and triggers a hide.
//!
//! ## Why there is no click-outside-dismiss here (D-07)
//!
//! D-07 deliberately cut a global mouse hook that could watch clicks landing outside our own
//! window. Hiding here is driven exclusively by *our own* focus-loss signal (`ShownFocused` →
//! `Hidden`), which is how the OS already reports "some other window was activated" without any
//! system-wide input monitoring.

/// The three states a stripe can be in. Deliberately three, not two ([`Hidden`]/`Shown`) — the
/// distinction between "shown but never yet focused" and "shown and was focused" is what
/// prevents the never-focused hide hazard described in the module docs above.
///
/// [`Hidden`]: Visibility::Hidden
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Visibility {
    /// The stripe is not visible.
    Hidden,
    /// The stripe is visible but has not yet been observed focused since its last reveal.
    ShownUnfocused,
    /// The stripe is visible and has been focused at least once since its last reveal — a
    /// subsequent focus loss now means the user clicked away.
    ShownFocused,
}

/// The single side effect a caller must perform after a [`Visibility`] transition, as decided
/// by [`effect_for`]. There are only two: reveal the window, or conceal it. Every other
/// transition (including no transition at all) requires no OS call.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Effect {
    /// Call `win::reveal` — show the window without taking focus (D-05).
    Reveal,
    /// Call `win::conceal` — hide the window.
    Conceal,
}

/// Computes the next [`Visibility`] from the current state and this frame's inputs.
///
/// `focused_this_frame` is the OS-reported focus state for our own window this frame
/// (`ctx.input(|i| i.viewport().focused).unwrap_or(false)` at the call site — see `app.rs`).
/// `toggle_requested` is `true` exactly once, on the frame a hotkey press or a tray click (or a
/// tray "Show" while hidden) asks for a toggle.
///
/// A toggle always wins over a focus reading, and always flips to the opposite of "currently
/// visible": `Hidden` → `ShownUnfocused`, and either shown state → `Hidden`. This makes the
/// hotkey and tray behave identically whether the stripe is currently focused, unfocused, or
/// hidden (D-09/D-17: one shared toggle behavior regardless of source).
pub fn advance(state: Visibility, _focused_this_frame: bool, _toggle_requested: bool) -> Visibility {
    // RED: intentionally wrong (identity) so every non-trivial test below fails first.
    // Implemented properly in the immediately-following commit (TDD GREEN).
    state
}

/// Maps a `(previous, next)` transition to the one OS-facing effect the caller must perform, or
/// `None` if the transition needs no OS call. In particular, `ShownUnfocused -> ShownFocused` is
/// `None`: the OS already activated the window on its own (that is what produced the focus
/// event in the first place) — we are only *observing* that transition, not causing it.
pub fn effect_for(_previous: Visibility, _next: Visibility) -> Option<Effect> {
    // RED: intentionally wrong (always none) — implemented in the following GREEN commit.
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use Visibility::{Hidden, ShownFocused, ShownUnfocused};

    #[test]
    fn hotkey_reveals_from_hidden() {
        assert_eq!(advance(Hidden, false, true), ShownUnfocused);
    }

    #[test]
    fn never_focused_guard_survives_many_consecutive_unfocused_frames() {
        // The single most important test in this module (Pitfall 2): a window shown via
        // SW_SHOWNOACTIVATE reads focused=false from the instant it appears, and must not hide
        // itself before the user has ever had a chance to click into it.
        let mut state = ShownUnfocused;
        for _ in 0..20 {
            state = advance(state, false, false);
            assert_eq!(state, ShownUnfocused, "must not hide before ever being focused");
        }
    }

    #[test]
    fn click_in_moves_to_shown_focused() {
        assert_eq!(advance(ShownUnfocused, true, false), ShownFocused);
    }

    #[test]
    fn focus_loss_after_having_been_focused_hides() {
        assert_eq!(advance(ShownFocused, false, false), Hidden);
    }

    #[test]
    fn staying_focused_is_a_no_op() {
        assert_eq!(advance(ShownFocused, true, false), ShownFocused);
    }

    #[test]
    fn toggle_hides_from_shown_unfocused() {
        assert_eq!(advance(ShownUnfocused, false, true), Hidden);
        assert_eq!(advance(ShownUnfocused, true, true), Hidden);
    }

    #[test]
    fn toggle_hides_from_shown_focused() {
        assert_eq!(advance(ShownFocused, false, true), Hidden);
        assert_eq!(advance(ShownFocused, true, true), Hidden);
    }

    #[test]
    fn idle_while_hidden_never_reveals_from_a_stray_focus_reading() {
        assert_eq!(advance(Hidden, false, false), Hidden);
        // Even an errant `focused=true` reading (should not occur while genuinely hidden, but
        // the function must not use it to reveal) does nothing without a toggle.
        assert_eq!(advance(Hidden, true, false), Hidden);
    }

    #[test]
    fn effect_for_reveal_from_hidden() {
        assert_eq!(effect_for(Hidden, ShownUnfocused), Some(Effect::Reveal));
    }

    #[test]
    fn effect_for_conceal_on_hide() {
        assert_eq!(effect_for(ShownFocused, Hidden), Some(Effect::Conceal));
    }

    #[test]
    fn effect_for_focus_observation_is_not_our_effect() {
        // The OS already activated the window by the time this transition is observed; we must
        // not issue a redundant reveal call for it.
        assert_eq!(effect_for(ShownUnfocused, ShownFocused), None);
    }

    #[test]
    fn effect_for_no_transition_is_none_for_every_state() {
        assert_eq!(effect_for(Hidden, Hidden), None);
        assert_eq!(effect_for(ShownUnfocused, ShownUnfocused), None);
        assert_eq!(effect_for(ShownFocused, ShownFocused), None);
    }
}
