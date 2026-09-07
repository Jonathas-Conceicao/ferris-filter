//! The Apply outcome and the unapplied-changes indicator (D-03, D-15..D-17).
//!
//! Pure — no GUI-framework types of any kind anywhere in this file (D-18: `PITFALLS.md` §8's rule
//! that logic belongs in the library, presentation at the edge). `app.rs` is the only caller: it
//! drives the dynamic module's compile-and-write pair directly (D-15 — those functions stay
//! Phase 12's, unmodified, and this module never wraps or re-implements them), then turns
//! the `Result` into an [`ApplyOutcome`] and updates the [`ApplyState`] it holds across frames.
//!
//! **Why this state exists at all (D-02/D-03):** `dynamic.json` autosaves on every edit, but
//! `dynamic.filter` — the file the game actually reads — changes only when the player presses
//! Apply. Those two can diverge the moment the store autosaves and Apply has not run since. If
//! the UI does not say so, the player has no way to tell "did my edit reach the game" from
//! "nothing happened" — PoE2 gives no feedback of its own either way (`PITFALLS.md` §1). This
//! state is deliberately transient and in-memory only, never written to `dynamic.json` or
//! anywhere else — it describes the current overlay session's belief about two files on disk, not
//! a fact worth persisting past a restart (a fresh launch has no idea what the last session did
//! last, which is exactly why [`ApplyState::NeverApplied`] reads identically to
//! [`ApplyState::Unapplied`] below).

use std::path::{Path, PathBuf};

/// D-16's exact reload instruction. Its wording matters — this is the one string in this phase a
/// player standing in front of the game reads literally, so it is a named constant rather than
/// inlined at each use site.
/// Deliberately ASCII. The stripe renders this in the Cinzel display family, which has no
/// arrow glyph, so a → came out as a tofu box in the confirmation line (reported from live use
/// 2026-09-07, screenshot poe2_print005). `install_fonts` now gives that family a fallback
/// chain, but the fallback fonts are not guaranteed to carry U+2192 either — and this is the
/// one string whose job is to tell the player how to make their filter take effect, so it does
/// not get to depend on glyph coverage.
pub const RELOAD_INSTRUCTION: &str = "Options > Game > Item Filter > Refresh";

/// Whether the filter on disk (`dynamic.filter`) currently matches the saved rule store
/// (`dynamic.json`), as far as this overlay session knows.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ApplyState {
    /// No Apply has happened this session. Because D-02 lets the store and the filter move
    /// independently, whatever `dynamic.filter` holds right now is whatever the *last* session
    /// left — it must not be presented as "up to date" just because this session hasn't touched
    /// it yet. [`indicator`] deliberately renders this identically to [`ApplyState::Unapplied`].
    NeverApplied,
    /// An edit has landed in the store since the most recent successful Apply. The filter on disk
    /// does not match the saved rules.
    Unapplied,
    /// The most recent Apply attempt succeeded and no edit has landed since.
    Applied,
}

impl ApplyState {
    /// A successful Apply moves any prior state to `Applied` (D-03).
    pub fn on_apply_success(self) -> Self {
        ApplyState::Applied
    }

    /// A failed Apply leaves the previous state exactly as it was (D-17) — a failure must never
    /// be presented as though it had applied.
    pub fn on_apply_failure(self) -> Self {
        self
    }
}

/// What an Apply attempt produced, for the caller to render. `app.rs` builds this from the
/// `Result`s [`crate::dynamic::compile_store`] and [`crate::dynamic::write_filter`] return —
/// this type itself performs neither call (D-15's second-caller reuse stays in `app.rs`, where
/// the acceptance criteria can grep for exactly one call site each).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ApplyOutcome {
    /// Compiled and written successfully; carries the path [`crate::dynamic::write_filter`]
    /// reported.
    Written(PathBuf),
    /// Compilation or the write failed; carries a single displayable string naming the failing
    /// path (D-17) — a `DynamicError`'s `Display` text, or an equivalent rendered failure for a
    /// missing default destination (never a raw `Debug` dump, never a panic).
    Failed(String),
}

/// The message rendered in the stripe after a successful Apply: names the written file and
/// states how to make the game read it. The exact substring `Options > Game > Item Filter >
/// Refresh` is asserted verbatim below — PoE2 never re-reads a filter on its own
/// (`PITFALLS.md` §1), so a tool that writes silently reads as broken.
pub fn success_message(written: &Path) -> String {
    format!("Wrote {}. Reload it in-game: {RELOAD_INSTRUCTION}", written.display())
}

/// D-03's short status text. `NeverApplied` and `Unapplied` read identically — both mean "the
/// written filter does not match these rules" — and `Applied` reads differently in wording, not
/// just colour, so the signal survives a colourblind or monochrome render (a colour-only signal
/// is not an indicator).
pub fn indicator(state: ApplyState) -> &'static str {
    match state {
        ApplyState::NeverApplied | ApplyState::Unapplied => {
            "Filter out of date — press Apply, then reload in-game"
        }
        ApplyState::Applied => "Filter matches saved rules",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn success_message_names_the_written_path_and_contains_the_exact_reload_instruction() {
        let msg = success_message(Path::new(r"C:\Games\PoE2\dynamic.filter"));
        assert!(msg.contains("dynamic.filter"), "got: {msg}");
        assert!(
            msg.contains("Options > Game > Item Filter > Refresh"),
            "the reload instruction must appear verbatim, got: {msg}"
        );
    }

    #[test]
    fn indicator_reads_identically_for_never_applied_and_unapplied() {
        assert_eq!(indicator(ApplyState::NeverApplied), indicator(ApplyState::Unapplied));
    }

    #[test]
    fn indicator_differs_once_applied() {
        assert_ne!(indicator(ApplyState::Unapplied), indicator(ApplyState::Applied));
        assert_ne!(indicator(ApplyState::NeverApplied), indicator(ApplyState::Applied));
    }

    #[test]
    fn on_apply_success_moves_any_starting_state_to_applied() {
        for start in [ApplyState::NeverApplied, ApplyState::Unapplied, ApplyState::Applied] {
            assert_eq!(start.on_apply_success(), ApplyState::Applied);
        }
    }

    #[test]
    fn on_apply_failure_leaves_every_starting_state_unchanged() {
        for start in [ApplyState::NeverApplied, ApplyState::Unapplied, ApplyState::Applied] {
            assert_eq!(start.on_apply_failure(), start);
        }
    }
}
