//! The Apply outcome, the footer's button decisions and the unapplied-changes indicator.
//!
//! Pure -- no GUI-framework types of any kind anywhere in this file (`PITFALLS.md` §8: logic
//! belongs in the library, presentation at the edge). `app.rs` is the only caller: it drives
//! [`crate::dynamic::apply_file`] directly (the same function the CLI's `apply` uses) and turns
//! the `Result` into an [`ApplyOutcome`], then updates the [`ApplyState`] held across frames.
//!
//! **Why this state exists at all:** the filter file is the store, but PoE2 never re-reads it on its
//! own, so "did my change reach the game" has no feedback of its own (`PITFALLS.md` §1). This
//! state is deliberately transient and in-memory only -- it describes the current overlay
//! session's belief about the file, which is why [`ApplyState::NeverApplied`] reads identically to
//! [`ApplyState::Unapplied`] below.

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

/// Whether the filter on disk matches what the last Apply wrote, as far as this overlay session
/// knows.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ApplyState {
    /// No Apply has happened this session. Whatever the file holds was left by an earlier session
    /// or another program, so it is not presented as "up to date". [`indicator`] renders this
    /// identically to [`ApplyState::Unapplied`].
    NeverApplied,
    /// The in-memory view may differ from the file on disk.
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
/// `Result` of [`crate::dynamic::apply_file`]; this type itself performs no call.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ApplyOutcome {
    /// The file was rewritten; carries its path.
    Written(PathBuf),
    /// The new text equals what is on disk; nothing was touched.
    Unchanged(PathBuf),
    /// Apply failed; a single displayable string (a `DynamicError`'s `Display`), never a raw
    /// `Debug` dump and never a panic.
    Failed(String),
}

/// The message rendered in the stripe after a successful Apply: names the written file and
/// states how to make the game read it. The exact substring `Options > Game > Item Filter >
/// Refresh` is asserted verbatim below — PoE2 never re-reads a filter on its own
/// (`PITFALLS.md` §1), so a tool that writes silently reads as broken.
pub fn success_message(written: &Path) -> String {
    format!("Wrote {}. Reload it in-game: {RELOAD_INSTRUCTION}", written.display())
}

/// The message rendered after an Apply that found nothing to change.
pub fn unchanged_message(path: &Path) -> String {
    format!("{} is already up to date; nothing was written", path.display())
}

/// True only for the guarded-write refusal "the filter changed on disk since it was loaded" --
/// the one failure whose recovery is RELOAD.
pub fn is_changed_on_disk(err: &crate::dynamic::DynamicError) -> bool {
    matches!(
        err,
        crate::dynamic::DynamicError::Write(crate::safe_write::WriteError::ChangedOnDisk { .. })
    )
}

/// What the footer knows about the filter file.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FilterState {
    /// The file could not be loaded (unreadable, not UTF-8, damaged v0.4 shape).
    LoadFailed,
    /// The file does not exist yet.
    Absent,
    /// The file exists and has no economy section.
    NoSection,
    /// The economy section markers are damaged.
    Damaged,
    /// The file has an economy section.
    HasSection,
}

/// Which footer buttons are live. Computed once per frame by [`footer_actions`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct FooterActions {
    pub sync: bool,
    pub apply: bool,
    pub add_economy: bool,
    pub reload: bool,
}

/// The footer's button rules. A missing app root disables everything that touches files; after a
/// changed-on-disk refusal only RELOAD (and SYNC) remain; ADD ECONOMY SECTION is the only way a
/// section is ever created -- APPLY never does it implicitly.
pub fn footer_actions(root_ok: bool, state: FilterState, needs_reload: bool) -> FooterActions {
    FooterActions {
        sync: root_ok,
        reload: needs_reload,
        apply: root_ok && state == FilterState::HasSection && !needs_reload,
        add_economy: root_ok
            && matches!(state, FilterState::Absent | FilterState::NoSection)
            && !needs_reload,
    }
}

/// The stripe's status text for the filter file.
pub fn filter_state_message(state: FilterState, migrated: bool) -> String {
    let mut text = match state {
        FilterState::Absent => {
            "The filter does not exist yet. ADD ECONOMY SECTION creates it.".to_string()
        }
        FilterState::NoSection => {
            "The filter has no economy section. ADD ECONOMY SECTION adds one at the end.".to_string()
        }
        FilterState::Damaged => {
            "The economy section markers are damaged. Fix them by hand, then RELOAD.".to_string()
        }
        FilterState::HasSection => "Economy section found.".to_string(),
        FilterState::LoadFailed => "The filter could not be loaded.".to_string(),
    };
    if migrated {
        text.push_str(
            " This is a v0.4 dynamic.filter. It will be converted on the next APPLY \
             (a .ffbak backup of the original is kept).",
        );
    }
    text
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
    fn a_failed_apply_never_advances_the_state_to_applied() {
        // Pairs with the guard: the outcome is only half of it. The indicator must keep saying
        // the filter is out of date, from every prior state.
        for state in [ApplyState::NeverApplied, ApplyState::Unapplied, ApplyState::Applied] {
            assert_eq!(
                state.on_apply_failure(),
                state,
                "a failure must leave {state:?} exactly as it was"
            );
        }
        assert_eq!(
            indicator(ApplyState::NeverApplied.on_apply_failure()),
            indicator(ApplyState::NeverApplied),
            "the player must still be told the filter does not match their rules"
        );
    }

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

    #[test]
    fn unchanged_message_says_nothing_was_written_and_omits_reload() {
        let m = unchanged_message(Path::new("x.filter"));
        assert!(m.contains("x.filter") && m.contains("nothing was written"));
        assert!(!m.contains("Refresh"));
    }

    #[test]
    fn changed_on_disk_is_recognised_only_for_that_refusal() {
        use crate::dynamic::DynamicError;
        let yes = DynamicError::Write(crate::safe_write::WriteError::ChangedOnDisk {
            path: "p".into(),
        });
        assert!(is_changed_on_disk(&yes));
        assert!(!is_changed_on_disk(&DynamicError::EconomyUnavailable));
    }

    #[test]
    fn footer_without_a_root_disables_every_file_action() {
        for st in [
            FilterState::Absent,
            FilterState::NoSection,
            FilterState::HasSection,
            FilterState::Damaged,
            FilterState::LoadFailed,
        ] {
            assert_eq!(footer_actions(false, st, false), FooterActions::default());
        }
    }

    #[test]
    fn footer_absent_and_no_section_offer_add_economy_not_apply() {
        for st in [FilterState::Absent, FilterState::NoSection] {
            let a = footer_actions(true, st, false);
            assert!(a.add_economy && !a.apply && a.sync && !a.reload);
        }
    }

    #[test]
    fn footer_has_section_offers_apply_only() {
        let a = footer_actions(true, FilterState::HasSection, false);
        assert!(a.apply && !a.add_economy && a.sync);
    }

    #[test]
    fn footer_damaged_and_load_failed_offer_neither() {
        for st in [FilterState::Damaged, FilterState::LoadFailed] {
            let a = footer_actions(true, st, false);
            assert!(!a.apply && !a.add_economy && a.sync);
        }
    }

    #[test]
    fn footer_reload_first_after_a_changed_on_disk_refusal() {
        let a = footer_actions(true, FilterState::HasSection, true);
        assert!(a.reload && !a.apply && !a.add_economy);
    }

    #[test]
    fn state_messages_name_the_actions_and_the_backup() {
        assert!(filter_state_message(FilterState::Absent, false).contains("ADD ECONOMY SECTION"));
        assert!(filter_state_message(FilterState::HasSection, true).contains(".ffbak"));
        assert!(!filter_state_message(FilterState::HasSection, false).contains(".ffbak"));
    }
}
