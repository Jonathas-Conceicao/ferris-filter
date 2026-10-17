//! The Apply outcome, the footer's button decisions and the unapplied-changes indicator.
//!
//! Pure -- no GUI-framework types of any kind anywhere in this file (`PITFALLS.md` §8: logic
//! belongs in the library, presentation at the edge). `app.rs` is the only caller: it drives
//! [`crate::dynamic::apply_edited`] (the guarded writer the CLI shares) and turns
//! the `Result` into an [`ApplyOutcome`], then updates the [`ApplyState`] held across frames.
//!
//! **Why this state exists at all:** the filter file is the store, but PoE2 never re-reads it on its
//! own, so "did my change reach the game" has no feedback of its own (`PITFALLS.md` §1). The
//! overlay can now ask the running game to reload (RELOAD-01), but the file itself is never
//! re-read automatically. This
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
/// `Result` of [`crate::dynamic::apply_edited`]; this type itself performs no call.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ApplyOutcome {
    /// Apply succeeded; carries the filter's path. Bytes identical to the file on disk are not
    /// rewritten, but still count as applied: the reload is sent and the status reads the same.
    Written(PathBuf),
    /// Apply failed; a single displayable string (a `DynamicError`'s `Display`), never a raw
    /// `Debug` dump and never a panic.
    Failed(String),
}

/// The message rendered in the stripe after a successful Apply: names the written file and
/// states how to make the game read it. The exact substring `Options > Game > Item Filter >
/// Refresh` is asserted verbatim below — PoE2 never re-reads a filter on its own
/// (`PITFALLS.md` §1), so a tool that writes silently reads as broken.
pub fn success_message(written: &Path) -> String {
    format!(
        "Wrote {}. Reload it in-game: {RELOAD_INSTRUCTION}",
        written.display()
    )
}

/// The message when the game is not running: the game reads the filter at start, so there is no
/// reload text and no reason.
pub fn written_message(written: &Path) -> String {
    format!("Wrote {}.", written.display())
}

/// Reason shown when no reload was sent because no game window was found.
pub const REASON_GAME_NOT_RUNNING: &str = "game not running";
/// Reason shown when no reload was sent because the user turned the setting off.
pub const REASON_SETTING_OFF: &str = "reload setting is off";
/// Reason shown when no reload was sent because another one is still in flight.
pub const REASON_BUSY: &str = "a reload is already in progress";
/// Reason shown when no reload was sent because the stripe's window handle is not available, so
/// the clipboard could not be used safely.
pub const REASON_NO_WINDOW: &str = "the overlay window is not ready";

/// Reason shown when sending the reload to the game failed.
pub fn reason_send_failed(cause: &str) -> String {
    format!("could not reload in game: {cause}")
}

/// The message after a reload was sent. The wording "Reloaded in game" is the author's choice
/// (review WR-01), but what it actually certifies is narrower: every key of the send sequence
/// was delivered to the game window. Nothing outside the game can confirm the game ran the
/// command: an elevated game silently drops input from a non-elevated ff (UIPI), keys can be lost
/// to timing, text in an already-open chat box is sent instead (chat.rs, CR-02), and
/// `/reloaditemfilter` reloads the filter selected in game, which is not necessarily the file
/// just written. The CONFIG tooltip and CLAUDE.md say so, and point to the manual Refresh.
pub fn reloaded_message(written: &Path) -> String {
    format!("Wrote {}. Reloaded in game", written.display())
}

/// The interim message while the reload worker is still typing into the game.
pub fn reloading_message(written: &Path) -> String {
    format!("Wrote {}. Reloading in game...", written.display())
}

/// The success message plus the reason no reload was sent; keeps the manual Refresh reminder.
pub fn not_reloaded_message(written: &Path, reason: &str) -> String {
    format!("{} ({reason})", success_message(written))
}

/// The final message once the reload worker has reported.
pub fn reload_result_message(written: &Path, result: &Result<(), String>) -> String {
    match result {
        Ok(()) => reloaded_message(written),
        Err(cause) => not_reloaded_message(written, &reason_send_failed(cause)),
    }
}

/// True only for the guarded-write refusal "the filter changed on disk since it was loaded" --
/// the one failure recovered by RELOAD or OVERWRITE.
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
    pub revert: bool,
    pub add_rule: bool,
    pub reload: bool,
    /// After a changed-on-disk refusal: write the in-memory rules over the outside change.
    pub overwrite: bool,
}

/// Refusal copy shown after a changed-on-disk refusal, naming every file that changed; RELOAD and
/// OVERWRITE recover, and their tooltips carry the explanation.
pub fn changed_on_disk_message(names: &[String]) -> String {
    format!("Changed on disk since it was loaded: {}.", names.join(", "))
}

/// The start of the Apply message when `styles.json` / `tiers.json` were written ("" when neither
/// was); the filter part of the message follows it.
pub fn config_saved_prefix(styles_written: bool, tiers_written: bool) -> String {
    match (styles_written, tiers_written) {
        (false, false) => String::new(),
        (true, false) => "Saved styles.json. ".to_string(),
        (false, true) => "Saved tiers.json. ".to_string(),
        (true, true) => "Saved styles.json and tiers.json. ".to_string(),
    }
}

/// The Apply message when there was no filter to write: just what the config part did.
pub fn config_only_message(styles_written: bool, tiers_written: bool) -> String {
    let prefix = config_saved_prefix(styles_written, tiers_written);
    if prefix.is_empty() {
        "Config already up to date.".to_string()
    } else {
        prefix.trim_end().to_string()
    }
}

/// Tooltip on RELOAD.
pub const RELOAD_TOOLTIP: &str = "Discard your edits and load the file from disk.";

/// Tooltip on OVERWRITE.
pub const OVERWRITE_TOOLTIP: &str = "Replace the file with your edits.";

/// Status after REVERT.
pub const REVERTED_MESSAGE: &str = "Reverted to the last applied filter.";

/// Tooltip on the disabled APPLY button while an enabled rule has a grammar error.
pub const APPLY_BLOCKED_TOOLTIP: &str = "Fix or disable the rules marked with the red ! first.";

/// The status shown when Apply is refused by the grammar gate.
pub fn invalid_blocks_message(count: usize, first_rule: usize, first_line: usize) -> String {
    let noun = if count == 1 {
        "1 enabled rule has a grammar error".to_string()
    } else {
        format!("{count} enabled rules have grammar errors")
    };
    format!(
        "Apply refused: {noun} (first: rule {first_rule}, line {first_line}). Fix them or turn them off."
    )
}

/// The status after + Add rule: the new rule is the first block of the file.
pub const RULE_ADDED_MESSAGE: &str = "Rule added at the top of the file. Press Apply to save it.";

/// The footer's button rules. A missing app root disables everything that touches files; after a
/// changed-on-disk refusal only RELOAD, OVERWRITE (gated like APPLY) and SYNC remain.
///
/// APPLY no longer needs an economy section (OD-2): a rules-only filter is written as-is. A
/// filter that does not exist yet can be created once there is something to write (`dirty`).
/// `blocking` is the number of enabled-rule grammar errors, which disable APPLY.
///
/// RELOAD is also offered whenever the overlay is stuck in a state only a re-read can clear: no
/// app root, damaged markers, or a filter that failed to load.
pub fn footer_actions(
    root_ok: bool,
    state: FilterState,
    needs_reload: bool,
    dirty: bool,
    blocking: usize,
) -> FooterActions {
    let editable = !matches!(state, FilterState::LoadFailed | FilterState::Damaged);
    let writable = match state {
        FilterState::HasSection | FilterState::NoSection => true,
        FilterState::Absent => dirty,
        FilterState::Damaged | FilterState::LoadFailed => false,
    };
    FooterActions {
        sync: root_ok,
        reload: needs_reload
            || !root_ok
            || matches!(state, FilterState::Damaged | FilterState::LoadFailed),
        apply: root_ok && writable && !needs_reload && blocking == 0,
        overwrite: root_ok && writable && needs_reload && blocking == 0,
        revert: dirty && !needs_reload && state != FilterState::LoadFailed,
        add_rule: !needs_reload && editable,
    }
}

/// The status line for damaged economy markers; also the disabled CONFIG Apply tooltip.
pub const DAMAGED_MARKERS_MESSAGE: &str =
    "The economy section markers are damaged. Fix them by hand, then Reload.";

/// The stripe's status text for the filter file.
pub fn filter_state_message(state: FilterState, migrated: bool) -> String {
    let mut text = match state {
        FilterState::Absent => "The filter does not exist yet. Apply creates it once it has rules \
             or an economy section."
            .to_string(),
        FilterState::NoSection => "The filter has no economy section.".to_string(),
        FilterState::Damaged => DAMAGED_MARKERS_MESSAGE.to_string(),
        FilterState::HasSection => "Economy section found.".to_string(),
        FilterState::LoadFailed => "The filter could not be loaded.".to_string(),
    };
    if migrated {
        text.push_str(
            " This is a v0.4 dynamic.filter. It will be converted on the next Apply \
             (a .ffbak backup of the original is kept).",
        );
    }
    text
}

/// D-03's short status text. `NeverApplied` and `Unapplied` read identically — both mean "the
/// written filter does not match these rules" — and `Applied` reads differently in wording, not
/// just colour, so the signal survives a colourblind or monochrome render (a colour-only signal
/// is not an indicator).
/// Short enough to fit the status line at 660pt; [`indicator_tip`] carries the explanation.
pub fn indicator(state: ApplyState) -> &'static str {
    match state {
        ApplyState::NeverApplied | ApplyState::Unapplied => UNAPPLIED_INDICATOR,
        ApplyState::Applied => "Filter matches saved rules",
    }
}

/// The unapplied indicator's text.
pub const UNAPPLIED_INDICATOR: &str = "Unapplied changes";

/// Hover text of the unapplied indicator: what the short message means and what to do.
pub const UNAPPLIED_INDICATOR_TIP: &str = "The filter on disk does not match these edits yet. \
     Press Apply to write it; it is then reloaded in game (or use Options > Game > Item Filter > \
     Refresh).";

/// The hover explanation for an indicator text, if it has one.
pub fn indicator_tip(text: &str) -> Option<&'static str> {
    (text == UNAPPLIED_INDICATOR).then_some(UNAPPLIED_INDICATOR_TIP)
}

/// Status after REMOVE STYLE: the rule keeps its actions.
pub const STYLE_REMOVED_STATUS: &str = "Style removed. The actions are now yours to edit.";

/// Appended to the Apply outcome when styled rules were rewritten (leading space; empty for 0).
pub fn styled_rules_message(n: usize) -> String {
    match n {
        0 => String::new(),
        1 => " Updated 1 styled rule.".to_string(),
        n => format!(" Updated {n} styled rules."),
    }
}

/// Footer line for rules whose `# ff:style` names a style that does not exist.
pub fn unknown_styles_message(count: usize, first_rule: usize) -> String {
    if count == 1 {
        format!("1 rule uses an unknown style (first: rule {first_rule}).")
    } else {
        format!("{count} rules use unknown styles (first: rule {first_rule}).")
    }
}

/// Apply-message tail for rules whose `# ff:style` tag was removed because the style is gone.
pub fn dropped_styles_message(count: usize, first_rule: usize) -> String {
    if count == 1 {
        format!(
            "Removed the style link from 1 rule whose style no longer exists (first: rule {first_rule})."
        )
    } else {
        format!(
            "Removed the style link from {count} rules whose styles no longer exist (first: rule {first_rule})."
        )
    }
}

/// Apply-message tail for styled rules whose rewrite was refused; they keep their current actions.
pub fn failed_styles_message(count: usize, first_rule: usize, err: &str) -> String {
    if count == 1 {
        format!("Could not update the style of rule {first_rule} ({err}); it keeps its actions.")
    } else {
        format!(
            "Could not update the style of {count} rules (first: rule {first_rule}: {err}); they keep their actions."
        )
    }
}

/// Apply refusal when styled rules depend on styles that cannot be read (`styles.json`, or
/// `styles-default.json` when it is absent; `err` names the file).
pub fn styles_unavailable_message(err: &str) -> String {
    format!("The styles could not be read, so styled rules cannot be updated: {err}")
}

/// Whether anything is waiting for the next Apply: rule edits (or a staged economy op), a style
/// rewrite the written filter does not have yet, or CONFIG edits. Both tabs show this one state.
pub fn combined_dirty(rules_dirty: bool, style_pending: bool, config_dirty: bool) -> bool {
    rules_dirty || style_pending || config_dirty
}

/// The user's answer in the tray-Quit confirm.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QuitChoice {
    ApplyAndQuit,
    QuitWithoutApplying,
    Cancel,
}

/// Something that can change what the tray Quit does.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QuitEvent {
    TrayQuit,
    Choice(QuitChoice),
    ApplyFinished { ok: bool },
}

/// What the app does next about quitting.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QuitStep {
    /// Run the shutdown sequence now.
    QuitNow,
    /// Show the stripe with the confirm.
    OpenConfirm,
    /// Confirm already open: nothing to do.
    KeepConfirm,
    /// Close the confirm, stay open.
    CloseConfirm,
    /// Run the combined Apply.
    RunApply,
    /// Apply succeeded: quit once any in-game reload worker has finished.
    QuitWhenIdle,
}

/// Pure decision for the tray Quit: pending edits x confirm state x event -> next step.
pub fn quit_step(dirty: bool, confirm_open: bool, event: QuitEvent) -> QuitStep {
    match event {
        QuitEvent::TrayQuit if !dirty => QuitStep::QuitNow,
        QuitEvent::TrayQuit if confirm_open => QuitStep::KeepConfirm,
        QuitEvent::TrayQuit => QuitStep::OpenConfirm,
        QuitEvent::Choice(QuitChoice::Cancel) => QuitStep::CloseConfirm,
        QuitEvent::Choice(QuitChoice::QuitWithoutApplying) => QuitStep::QuitNow,
        QuitEvent::Choice(QuitChoice::ApplyAndQuit) => QuitStep::RunApply,
        QuitEvent::ApplyFinished { ok: true } => QuitStep::QuitWhenIdle,
        QuitEvent::ApplyFinished { ok: false } => QuitStep::CloseConfirm,
    }
}

/// True when the step leaves the confirm open, so a hidden stripe must be revealed to show it.
/// A second tray Quit counts too: the stripe may have been hidden while the confirm was up.
pub fn quit_step_shows_confirm(step: QuitStep) -> bool {
    matches!(step, QuitStep::OpenConfirm | QuitStep::KeepConfirm)
}

pub const QUIT_CONFIRM_TITLE: &str = "Unapplied changes";
pub const QUIT_APPLY_LABEL: &str = "Apply & quit";
pub const QUIT_DISCARD_LABEL: &str = "Quit without applying";
pub const QUIT_CANCEL_LABEL: &str = "Cancel";
pub const QUIT_DISCARD_TIP: &str = "Unapplied edits are lost";

/// The RULES footer: APPLY follows the combined state (CONFIG edits alone can make an absent
/// filter writable), REVERT follows only the rule edits it would discard.
pub fn footer_actions_combined(
    root_ok: bool,
    state: FilterState,
    needs_reload: bool,
    rules_dirty: bool,
    config_dirty: bool,
    blocking: usize,
) -> FooterActions {
    let mut actions = footer_actions(
        root_ok,
        state,
        needs_reload,
        rules_dirty || config_dirty,
        blocking,
    );
    actions.revert = footer_actions(root_ok, state, needs_reload, rules_dirty, blocking).revert;
    actions
}

/// Whether the CONFIG tab's APPLY is live: the config is valid, nothing changed on disk, something
/// is pending on either tab, and (when a filter will be written too) no enabled rule has a grammar
/// error and the economy markers are not damaged.
pub fn config_apply_enabled(
    root_ok: bool,
    config_valid: bool,
    state: FilterState,
    changed_on_disk: bool,
    dirty: bool,
    blocking: usize,
) -> bool {
    let has_filter = state != FilterState::LoadFailed;
    root_ok
        && config_valid
        && !changed_on_disk
        && dirty
        && (!has_filter || (blocking == 0 && state != FilterState::Damaged))
}

/// Whether OVERWRITE is live after a changed-on-disk refusal, for the bottom bar and the popup
/// alike (review WR-03): the filter's own decision (`filter_overwrite`, i.e.
/// `FooterActions::overwrite`), or -- for a refusal that only the config files can resolve, such as
/// a filter that failed to load or an absent, clean one -- the CONFIG Apply decision with the
/// changed-on-disk guard lifted, so a config-only refusal stays overwritable.
///
/// `config_blocks` is a loaded config that fails validation (tier errors, an invalid
/// styles.json/tiers.json): every Apply validates it, so Overwrite is off then, as v0.5's CONFIG
/// OVERWRITE was gated on validity like its Apply (review WR-02, iteration 2).
#[allow(clippy::too_many_arguments)]
pub fn overwrite_enabled(
    filter_overwrite: bool,
    root_ok: bool,
    config_valid: bool,
    config_blocks: bool,
    state: FilterState,
    needs_reload: bool,
    dirty: bool,
    blocking: usize,
) -> bool {
    needs_reload
        && !config_blocks
        && (filter_overwrite
            || config_apply_enabled(root_ok, config_valid, state, false, dirty, blocking))
}

/// The indicator after one tab's REVERT: still out of date when the other tab holds edits.
pub fn state_after_revert(applied_this_session: bool, other_dirty: bool) -> ApplyState {
    if other_dirty {
        ApplyState::Unapplied
    } else if applied_this_session {
        ApplyState::Applied
    } else {
        ApplyState::NeverApplied
    }
}

/// A tab's label, with ` *` when that tab holds unapplied edits.
pub fn tab_label(name: &str, dirty: bool) -> String {
    if dirty {
        format!("{name} *")
    } else {
        name.to_string()
    }
}

/// Tooltip of the ` *` marker on a tab label.
pub const OTHER_TAB_DIRTY_TIP: &str = "Unapplied changes on this tab";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quit_step_table() {
        use QuitChoice::*;
        use QuitEvent::*;
        use QuitStep::*;
        let cases = [
            (false, false, TrayQuit, QuitNow),
            (false, true, TrayQuit, QuitNow),
            (true, false, TrayQuit, OpenConfirm),
            (true, true, TrayQuit, KeepConfirm),
            (true, true, Choice(Cancel), CloseConfirm),
            (true, true, Choice(QuitWithoutApplying), QuitNow),
            (true, true, Choice(ApplyAndQuit), RunApply),
            (true, false, ApplyFinished { ok: true }, QuitWhenIdle),
            (true, false, ApplyFinished { ok: false }, CloseConfirm),
        ];
        for (dirty, open, ev, want) in cases {
            assert_eq!(quit_step(dirty, open, ev), want, "{dirty} {open} {ev:?}");
        }
    }

    #[test]
    fn a_second_tray_quit_reveals_the_confirm_too() {
        assert!(quit_step_shows_confirm(quit_step(
            true,
            false,
            QuitEvent::TrayQuit
        )));
        assert!(quit_step_shows_confirm(quit_step(
            true,
            true,
            QuitEvent::TrayQuit
        )));
        for s in [
            QuitStep::QuitNow,
            QuitStep::CloseConfirm,
            QuitStep::RunApply,
            QuitStep::QuitWhenIdle,
        ] {
            assert!(!quit_step_shows_confirm(s), "{s:?}");
        }
    }

    #[test]
    fn combined_dirty_marks_unapplied() {
        assert!(!combined_dirty(false, false, false));
        for (r, s, c) in [
            (true, false, false),
            (false, true, false),
            (false, false, true),
        ] {
            assert!(combined_dirty(r, s, c));
        }
        // RULES Apply with only CONFIG edits: live even for a filter that does not exist yet.
        let a = footer_actions_combined(true, FilterState::Absent, false, false, true, 0);
        assert!(a.apply && !a.revert, "revert follows the rule edits only");
        assert!(!footer_actions_combined(true, FilterState::Absent, false, false, false, 0).apply);
        // CONFIG Apply with only RULES edits.
        let st = FilterState::HasSection;
        assert!(config_apply_enabled(true, true, st, false, true, 0));
        assert!(
            !config_apply_enabled(true, true, st, false, false, 0),
            "nothing pending"
        );
        assert!(
            !config_apply_enabled(true, false, st, false, true, 0),
            "invalid config"
        );
        assert!(
            !config_apply_enabled(true, true, st, true, true, 0),
            "changed on disk"
        );
        assert!(
            !config_apply_enabled(true, true, st, false, true, 2),
            "grammar errors block"
        );
        assert!(
            !config_apply_enabled(true, true, FilterState::Damaged, false, true, 0),
            "damaged economy markers block"
        );
        // Without a filter part the config alone applies.
        assert!(config_apply_enabled(
            true,
            true,
            FilterState::LoadFailed,
            false,
            true,
            2
        ));
    }

    #[test]
    fn revert_state_depends_on_the_other_tab() {
        assert_eq!(state_after_revert(true, true), ApplyState::Unapplied);
        assert_eq!(state_after_revert(false, true), ApplyState::Unapplied);
        assert_eq!(state_after_revert(true, false), ApplyState::Applied);
        assert_eq!(state_after_revert(false, false), ApplyState::NeverApplied);
    }

    #[test]
    fn tab_labels_mark_a_dirty_tab() {
        assert_eq!(tab_label("RULES", true), "RULES *");
        assert_eq!(tab_label("CONFIG", false), "CONFIG");
    }

    #[test]
    fn a_failed_apply_never_advances_the_state_to_applied() {
        // Pairs with the guard: the outcome is only half of it. The indicator must keep saying
        // the filter is out of date, from every prior state.
        for state in [
            ApplyState::NeverApplied,
            ApplyState::Unapplied,
            ApplyState::Applied,
        ] {
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
    fn written_message_has_no_reload_text() {
        let p = Path::new("C:/g/dynamic.filter");
        let m = written_message(p);
        assert_eq!(m, format!("Wrote {}.", p.display()));
        assert!(!m.contains("Refresh") && !m.contains("Reload"), "{m}");
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
        assert_eq!(
            indicator(ApplyState::NeverApplied),
            indicator(ApplyState::Unapplied)
        );
    }

    #[test]
    fn indicator_is_short_and_explained_on_hover() {
        // The status slot left of Revert / Reload / Apply holds ~30 characters at 660pt.
        for state in [ApplyState::Unapplied, ApplyState::Applied] {
            assert!(
                indicator(state).chars().count() <= 26,
                "{}",
                indicator(state)
            );
        }
        assert_eq!(
            indicator_tip(indicator(ApplyState::Unapplied)),
            Some(UNAPPLIED_INDICATOR_TIP)
        );
        assert_eq!(indicator_tip(indicator(ApplyState::Applied)), None);
    }

    #[test]
    fn indicator_differs_once_applied() {
        assert_ne!(
            indicator(ApplyState::Unapplied),
            indicator(ApplyState::Applied)
        );
        assert_ne!(
            indicator(ApplyState::NeverApplied),
            indicator(ApplyState::Applied)
        );
    }

    #[test]
    fn on_apply_success_moves_any_starting_state_to_applied() {
        for start in [
            ApplyState::NeverApplied,
            ApplyState::Unapplied,
            ApplyState::Applied,
        ] {
            assert_eq!(start.on_apply_success(), ApplyState::Applied);
        }
    }

    #[test]
    fn on_apply_failure_leaves_every_starting_state_unchanged() {
        for start in [
            ApplyState::NeverApplied,
            ApplyState::Unapplied,
            ApplyState::Applied,
        ] {
            assert_eq!(start.on_apply_failure(), start);
        }
    }

    #[test]
    fn changed_on_disk_is_recognised_only_for_that_refusal() {
        use crate::dynamic::DynamicError;
        let yes =
            DynamicError::Write(crate::safe_write::WriteError::ChangedOnDisk { path: "p".into() });
        assert!(is_changed_on_disk(&yes));
        assert!(!is_changed_on_disk(&DynamicError::EconomyUnavailable));
    }

    #[test]
    fn footer_without_a_root_disables_every_file_action_but_offers_reload() {
        for st in [
            FilterState::Absent,
            FilterState::NoSection,
            FilterState::HasSection,
            FilterState::Damaged,
            FilterState::LoadFailed,
        ] {
            let a = footer_actions(false, st, false, true, 0);
            assert!(!a.sync && !a.apply && a.reload, "{st:?}");
        }
    }

    #[test]
    fn apply_is_live_with_or_without_an_economy_section() {
        for st in [FilterState::HasSection, FilterState::NoSection] {
            assert!(footer_actions(true, st, false, false, 0).apply, "{st:?}");
            assert!(!footer_actions(true, st, false, false, 1).apply, "{st:?}");
        }
    }

    #[test]
    fn apply_on_an_absent_filter_needs_something_to_write() {
        assert!(!footer_actions(true, FilterState::Absent, false, false, 0).apply);
        assert!(footer_actions(true, FilterState::Absent, false, true, 0).apply);
    }

    #[test]
    fn footer_damaged_and_load_failed_offer_reload_only() {
        for st in [FilterState::Damaged, FilterState::LoadFailed] {
            let a = footer_actions(true, st, false, true, 0);
            assert!(!a.apply && !a.add_rule && a.sync && a.reload, "{st:?}");
        }
        assert!(!footer_actions(true, FilterState::LoadFailed, false, true, 0).revert);
        assert!(footer_actions(true, FilterState::Damaged, false, true, 0).revert);
    }

    #[test]
    fn footer_clean_states_hide_reload() {
        for st in [
            FilterState::Absent,
            FilterState::NoSection,
            FilterState::HasSection,
        ] {
            assert!(!footer_actions(true, st, false, false, 0).reload, "{st:?}");
        }
    }

    #[test]
    fn footer_reload_or_overwrite_after_a_changed_on_disk_refusal() {
        let a = footer_actions(true, FilterState::HasSection, true, true, 0);
        assert!(a.sync && a.reload && a.overwrite && !a.apply && !a.revert && !a.add_rule);
        // OVERWRITE is gated like APPLY.
        assert!(!footer_actions(true, FilterState::HasSection, true, true, 1).overwrite);
        assert!(!footer_actions(false, FilterState::HasSection, true, true, 0).overwrite);
        assert!(!footer_actions(true, FilterState::Damaged, true, true, 0).overwrite);
        // Never offered outside a changed-on-disk refusal.
        assert!(!footer_actions(true, FilterState::HasSection, false, true, 0).overwrite);
    }

    #[test]
    fn overwrite_covers_config_only_refusals() {
        use FilterState::*;
        // Arguments: filter_overwrite, root_ok, config_valid, config_blocks, state,
        // needs_reload, dirty, blocking.
        // The filter's own Overwrite stands.
        assert!(overwrite_enabled(
            true, true, true, false, HasSection, true, false, 0
        ));
        // A failed or absent-and-clean filter: a config edit is still overwritable.
        for st in [LoadFailed, Absent] {
            assert!(
                overwrite_enabled(false, true, true, false, st, true, true, 0),
                "{st:?}"
            );
            assert!(
                !overwrite_enabled(false, true, true, false, st, true, false, 0),
                "{st:?}"
            );
            assert!(
                !overwrite_enabled(false, true, false, true, st, true, true, 0),
                "{st:?}"
            );
        }
        // Grammar errors or damaged markers still keep a filter write from happening.
        assert!(!overwrite_enabled(
            false, true, true, false, HasSection, true, true, 1
        ));
        assert!(!overwrite_enabled(
            false, true, true, false, Damaged, true, true, 0
        ));
        // An invalid config (tier errors) blocks even the filter's own Overwrite (WR-02).
        assert!(!overwrite_enabled(
            true, true, false, true, HasSection, true, true, 0
        ));
        // A config that failed to load is not validated, so it does not block.
        assert!(overwrite_enabled(
            true, true, false, false, HasSection, true, true, 0
        ));
        // Never outside a refusal.
        assert!(!overwrite_enabled(
            true, true, true, false, HasSection, false, true, 0
        ));
    }

    #[test]
    fn revert_follows_dirty() {
        assert!(!footer_actions(true, FilterState::HasSection, false, false, 0).revert);
        assert!(footer_actions(true, FilterState::HasSection, false, true, 0).revert);
    }

    #[test]
    fn state_messages_name_the_actions_and_the_backup() {
        assert!(!filter_state_message(FilterState::Absent, false).contains("ADD ECONOMY SECTION"));
        assert!(
            !filter_state_message(FilterState::NoSection, false).contains("ADD ECONOMY SECTION")
        );
        assert!(filter_state_message(FilterState::HasSection, true).contains(".ffbak"));
        assert!(!filter_state_message(FilterState::HasSection, false).contains(".ffbak"));
    }

    #[test]
    fn refusal_and_status_copy() {
        assert_eq!(
            invalid_blocks_message(2, 12, 3),
            "Apply refused: 2 enabled rules have grammar errors (first: rule 12, line 3). Fix them or turn them off."
        );
        assert!(invalid_blocks_message(1, 1, 1).contains("1 enabled rule has a grammar error"));
        assert_eq!(REVERTED_MESSAGE, "Reverted to the last applied filter.");
        assert_eq!(
            changed_on_disk_message(&["styles.json".to_string(), "dynamic.filter".to_string()]),
            "Changed on disk since it was loaded: styles.json, dynamic.filter."
        );
        assert_eq!(config_saved_prefix(false, false), "");
        assert_eq!(
            config_saved_prefix(true, true),
            "Saved styles.json and tiers.json. "
        );
        assert_eq!(config_only_message(false, true), "Saved tiers.json.");
        assert_eq!(
            config_only_message(false, false),
            "Config already up to date."
        );
        assert!(RELOAD_TOOLTIP.contains("Discard") && OVERWRITE_TOOLTIP.contains("Replace"));
        assert!(APPLY_BLOCKED_TOOLTIP.contains("red !"));
        assert_eq!(
            RULE_ADDED_MESSAGE,
            "Rule added at the top of the file. Press Apply to save it."
        );
    }

    #[test]
    fn style_messages() {
        assert_eq!(styled_rules_message(0), "");
        assert_eq!(styled_rules_message(1), " Updated 1 styled rule.");
        assert_eq!(styled_rules_message(3), " Updated 3 styled rules.");
        assert_eq!(
            unknown_styles_message(1, 12),
            "1 rule uses an unknown style (first: rule 12)."
        );
        assert_eq!(
            unknown_styles_message(2, 5),
            "2 rules use unknown styles (first: rule 5)."
        );
        assert_eq!(
            dropped_styles_message(1, 3),
            "Removed the style link from 1 rule whose style no longer exists (first: rule 3)."
        );
        assert_eq!(
            dropped_styles_message(2, 5),
            "Removed the style link from 2 rules whose styles no longer exist (first: rule 5)."
        );
        assert!(styles_unavailable_message("x").ends_with(": x"));
        assert_eq!(
            failed_styles_message(1, 4, "bad header"),
            "Could not update the style of rule 4 (bad header); it keeps its actions."
        );
        assert!(failed_styles_message(2, 3, "e").contains("2 rules (first: rule 3: e)"));
    }
}
