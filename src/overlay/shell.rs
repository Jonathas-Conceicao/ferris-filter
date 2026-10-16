//! The skinned shell's decisions: which tab is which, which tabs carry the ` *` marker, which
//! bottom-bar buttons are live and what they say, and the one status line.
//!
//! Everything here is a pure function over plain values (PITFALLS section 8): `app.rs` draws what
//! these return and performs the calls. The status line shows exactly one message, so
//! [`status_for`] is the priority table from the Phase 30 UI contract made executable.

use egui::TextureHandle;

use crate::filter::BlockId;
use crate::skin::Skin;
use crate::skin::fonts;
use crate::skin::palette as pal;
use crate::skin::widgets::{self as w, StatusKind};

use super::apply::{self, ApplyState, FooterActions};
use super::config_state;
pub(crate) use super::config_view::{APPLY_LOAD_ERROR_TIP, APPLY_NOTHING_TIP};

/// Window width in points (the same as `ff-proto`).
pub(crate) const WIDTH: f32 = 660.0;

/// The four stone tabs, left to right.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ShellTab {
    Rules,
    Config,
    Economy,
    About,
}

impl ShellTab {
    pub(crate) const ALL: [ShellTab; 4] = [
        ShellTab::Rules,
        ShellTab::Config,
        ShellTab::Economy,
        ShellTab::About,
    ];

    /// The caption (Fontin SmallCaps renders it as small caps).
    pub(crate) fn label(self) -> &'static str {
        match self {
            ShellTab::Rules => "Rules",
            ShellTab::Config => "Config",
            ShellTab::Economy => "Economy",
            ShellTab::About => "About",
        }
    }

    pub(crate) fn index(self) -> usize {
        match self {
            ShellTab::Rules => 0,
            ShellTab::Config => 1,
            ShellTab::Economy => 2,
            ShellTab::About => 3,
        }
    }

    pub(crate) fn from_index(i: usize) -> Option<ShellTab> {
        Self::ALL.get(i).copied()
    }

    /// Parses a command-line tab name (`rules|config|economy|about`).
    #[cfg_attr(not(debug_assertions), allow(dead_code))]
    pub(crate) fn from_arg(s: &str) -> Option<ShellTab> {
        match s.to_ascii_lowercase().as_str() {
            "rules" => Some(ShellTab::Rules),
            "config" => Some(ShellTab::Config),
            "economy" => Some(ShellTab::Economy),
            "about" => Some(ShellTab::About),
            _ => None,
        }
    }
}

/// Which tabs show the ` *` marker: a tab is marked when the OTHER editing tab holds unapplied
/// edits. Economy and About are never marked.
pub(crate) fn tab_markers(active: ShellTab, rules_dirty: bool, config_dirty: bool) -> [bool; 4] {
    [
        active != ShellTab::Rules && rules_dirty,
        active != ShellTab::Config && config_dirty,
        false,
        false,
    ]
}

// ---------------------------------------------------------------------------------------------
// Strings
// ---------------------------------------------------------------------------------------------

/// Status for a v0.4 filter.
pub(crate) const MIGRATED_STATUS: &str =
    "v0.4 filter: converted on the next Apply (a .ffbak backup is kept).";

/// Status for a filter file that does not exist yet.
pub(crate) const ABSENT_STATUS: &str =
    "The filter does not exist yet. Apply creates it once it has rules.";

/// Status while the economy sync runs.
pub(crate) const SYNC_RUNNING: &str = "Fetching economy prices\u{2026}";

/// Tooltip on the dimmed Reload button.
pub(crate) const NOTHING_TO_RELOAD: &str = "Nothing to reload.";

/// Status for a filter that could not be loaded.
pub(crate) fn load_failed_status(msg: &str) -> String {
    format!("The filter could not be loaded: {msg}. Fix the file and press Reload.")
}

// ---------------------------------------------------------------------------------------------
// Status line
// ---------------------------------------------------------------------------------------------

/// The one message the status line shows.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct StatusLine {
    pub text: String,
    pub kind: StatusKind,
    /// The rule a click on the line jumps to.
    pub jump: Option<BlockId>,
}

/// Everything the priority table looks at.
#[derive(Debug, Clone)]
pub(crate) struct StatusInputs {
    pub root_error: Option<String>,
    pub event: Option<(String, StatusKind)>,
    pub grammar_gate: Option<(String, BlockId)>,
    pub filter_error: Option<String>,
    pub syncing: bool,
    pub unknown: Option<(String, BlockId)>,
    pub migrated: bool,
    pub absent: bool,
    pub indicator: (&'static str, StatusKind),
}

/// The status line for these inputs: the highest-priority row of the contract wins.
pub(crate) fn status_for(inputs: StatusInputs) -> StatusLine {
    let plain = |text: String, kind| StatusLine {
        text,
        kind,
        jump: None,
    };
    if let Some(text) = inputs.root_error {
        return plain(text, StatusKind::Error);
    }
    if let Some((text, kind)) = inputs.event {
        return plain(text, kind);
    }
    if let Some((text, id)) = inputs.grammar_gate {
        return StatusLine {
            text,
            kind: StatusKind::Error,
            jump: Some(id),
        };
    }
    if let Some(text) = inputs.filter_error {
        return plain(text, StatusKind::Error);
    }
    if inputs.syncing {
        return plain(SYNC_RUNNING.to_string(), StatusKind::Info);
    }
    if let Some((text, id)) = inputs.unknown {
        return StatusLine {
            text,
            kind: StatusKind::Warning,
            jump: Some(id),
        };
    }
    if inputs.migrated {
        return plain(MIGRATED_STATUS.to_string(), StatusKind::Info);
    }
    if inputs.absent {
        return plain(ABSENT_STATUS.to_string(), StatusKind::Info);
    }
    plain(inputs.indicator.0.to_string(), inputs.indicator.1)
}

/// The colour class of the default indicator line.
pub(crate) fn indicator_kind(state: ApplyState) -> StatusKind {
    match state {
        ApplyState::NeverApplied | ApplyState::Unapplied => StatusKind::Warning,
        ApplyState::Applied => StatusKind::Info,
    }
}

/// The colour class of a successful Apply's message: Ok, or Warning when it carries the in-game
/// Refresh instruction (the reload did not happen). A failed Apply is always
/// [`StatusKind::Error`]; the kind is decided when the message is produced, never inferred later
/// from the apply state (review CR-02).
pub(crate) fn apply_success_kind(text: &str) -> StatusKind {
    if text.contains(apply::RELOAD_INSTRUCTION) {
        StatusKind::Warning
    } else {
        StatusKind::Ok
    }
}

/// The event message, if any: the Apply outcome wins over a CONFIG message, then a finished sync,
/// then the list status.
///
/// The Apply message carries the kind it was produced with (see [`apply_success_kind`]). A CONFIG
/// message is `(text, is_problem)`. A sync outcome is `(text, is_failure)`.
pub(crate) fn event_from(
    apply_message: Option<(String, StatusKind)>,
    config_message: Option<(String, bool)>,
    sync: Option<(String, bool)>,
    list_status: Option<String>,
) -> Option<(String, StatusKind)> {
    if let Some(message) = apply_message {
        return Some(message);
    }
    if let Some((text, problem)) = config_message {
        let kind = if problem {
            StatusKind::Error
        } else {
            StatusKind::Info
        };
        return Some((text, kind));
    }
    if let Some((text, failure)) = sync {
        let kind = if failure {
            StatusKind::Error
        } else {
            StatusKind::Ok
        };
        return Some((text, kind));
    }
    list_status.map(|text| (text, StatusKind::Info))
}

// ---------------------------------------------------------------------------------------------
// Bottom bar
// ---------------------------------------------------------------------------------------------

/// What the CONFIG tab adds to the bar decision.
#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct ConfigBarInputs {
    pub dirty: bool,
    pub apply_enabled: bool,
    pub tier_errors: bool,
    pub load_error: bool,
    pub rules_blocking: bool,
    pub damaged: bool,
}

/// The right-most button: Apply, or Overwrite while a file changed on disk.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ApplySlot {
    pub label: &'static str,
    pub enabled: bool,
    pub danger: bool,
    pub tip: Option<&'static str>,
    /// True when the slot is the Overwrite button.
    pub overwrite: bool,
}

/// The three bottom-bar buttons.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct BarState {
    pub revert: bool,
    pub reload: bool,
    pub reload_tip: &'static str,
    pub apply: ApplySlot,
}

/// The bottom bar for this tab. Rules mirrors `footer_actions_combined`; Config's Revert follows
/// its dirty state and its Apply follows `config_apply_enabled`; Economy and About have no Revert
/// and use the combined Apply decision.
pub(crate) fn bar_state(
    tab: ShellTab,
    actions: FooterActions,
    config: ConfigBarInputs,
    needs_reload: bool,
) -> BarState {
    let reload_tip = if actions.reload {
        apply::RELOAD_TOOLTIP
    } else {
        NOTHING_TO_RELOAD
    };
    let revert = match tab {
        ShellTab::Rules => actions.revert,
        ShellTab::Config => config.dirty,
        ShellTab::Economy | ShellTab::About => false,
    };
    let apply = if needs_reload {
        ApplySlot {
            label: "Overwrite",
            enabled: actions.overwrite,
            danger: true,
            tip: Some(apply::OVERWRITE_TOOLTIP),
            overwrite: true,
        }
    } else {
        let enabled = match tab {
            ShellTab::Config => config.apply_enabled,
            _ => actions.apply,
        };
        let tip = if enabled {
            None
        } else if config.tier_errors {
            Some(config_state::APPLY_BLOCKED_TOOLTIP)
        } else if config.load_error {
            Some(APPLY_LOAD_ERROR_TIP)
        } else if config.rules_blocking {
            Some(apply::APPLY_BLOCKED_TOOLTIP)
        } else if config.damaged {
            Some(apply::DAMAGED_MARKERS_MESSAGE)
        } else {
            Some(APPLY_NOTHING_TIP)
        };
        ApplySlot {
            label: "Apply",
            enabled,
            danger: false,
            tip,
            overwrite: false,
        }
    };
    BarState {
        revert,
        reload: actions.reload,
        reload_tip,
        apply,
    }
}

// ---------------------------------------------------------------------------------------------
// Header art, Economy and About
// ---------------------------------------------------------------------------------------------

/// The header picture: the ONE switch for FerrisFilter's own header art. `None` paints the
/// placeholder plate; today it is the game's Options title bar.
pub(crate) fn header_art(skin: &Skin) -> Option<&TextureHandle> {
    Some(&skin.title_bar)
}

/// "Last sync" as local `YYYY-MM-DD HH:MM` from the `prices.json` modified time (Unix seconds)
/// and the local UTC offset in minutes; `Never` when there is no such file.
pub(crate) fn last_sync_text(mtime_unix_secs: Option<u64>, utc_offset_minutes: i32) -> String {
    let Some(secs) = mtime_unix_secs else {
        return "Never".to_string();
    };
    let local = (secs as i64 + i64::from(utc_offset_minutes) * 60).max(0) as u64;
    let (year, month, day) = crate::sync::prices::days_to_ymd(local / 86_400);
    let rem = local % 86_400;
    format!(
        "{year:04}-{month:02}-{day:02} {:02}:{:02}",
        rem / 3600,
        (rem % 3600) / 60
    )
}

/// The ECONOMY tab: league, last sync and the one Sync economy button. Returns true when the
/// button was clicked. While `syncing` it reads `Syncing…` and is disabled.
pub(crate) fn economy_tab(
    ui: &mut egui::Ui,
    skin: &Skin,
    league: &str,
    last_sync: &str,
    syncing: bool,
    enabled: bool,
) -> bool {
    let mut clicked = false;
    w::ff_scroll_area(ui, skin, "economy", |ui| {
        w::ff_section(ui, skin, "Economy");
        for (i, (label, value)) in [("League", league), ("Last sync", last_sync)]
            .into_iter()
            .enumerate()
        {
            w::ff_row(ui, i, label, |ui| {
                ui.label(
                    egui::RichText::new(value)
                        .font(fonts::regular(15.0))
                        .color(pal::VALUE),
                );
            });
        }
        ui.add_space(14.0);
        ui.with_layout(egui::Layout::top_down(egui::Align::Center), |ui| {
            let label = if syncing {
                "Syncing…"
            } else {
                "Sync economy"
            };
            if w::ff_button_ex(ui, skin, label, 180.0, enabled && !syncing, false).clicked() {
                clicked = true;
            }
        });
    });
    clicked
}

/// The ABOUT tab: identity, credits and the not-affiliated disclaimer pinned to the bottom.
pub(crate) fn about_tab(ui: &mut egui::Ui, skin: &Skin) {
    use egui::{Align, Layout, Rect, UiBuilder, pos2, vec2};
    let area = ui.available_rect_before_wrap();
    let foot_h = 80.0;
    let body = Rect::from_min_max(
        area.min,
        pos2(area.max.x, (area.max.y - foot_h).max(area.min.y)),
    );
    let mut bui = ui.new_child(
        UiBuilder::new()
            .max_rect(body)
            .layout(Layout::top_down(Align::Min)),
    );
    w::ff_scroll_area(&mut bui, skin, "about", |ui| {
        ui.add_space(24.0);
        ui.vertical_centered(|ui| {
            ui.spacing_mut().item_spacing.y = 6.0;
            ui.label(
                egui::RichText::new("FerrisFilter")
                    .font(fonts::small_caps(21.0))
                    .color(pal::HEADING),
            );
            ui.label(
                egui::RichText::new(format!("Version {}", env!("CARGO_PKG_VERSION")))
                    .font(fonts::regular(15.0))
                    .color(pal::MUTED),
            );
            ui.label(
                egui::RichText::new("A loot-filter editor and overlay for Path of Exile 2")
                    .font(fonts::regular(15.0))
                    .color(pal::LABEL),
            );
        });
        ui.add_space(24.0);
        w::ff_section(ui, skin, "Credits");
        let credits = [
            ("Game UI art and font files", "© Grinding Gear Games"),
            ("Fontin typeface", "exljbris"),
            ("Ferris the crab", "Karen Rustad Tölva, public domain"),
        ];
        for (i, (what, who)) in credits.iter().enumerate() {
            w::ff_row(ui, i, what, |ui| {
                ui.label(
                    egui::RichText::new(*who)
                        .font(fonts::regular(15.0))
                        .color(pal::VALUE),
                );
            });
        }
    });
    let foot = Rect::from_min_max(pos2(area.min.x, area.max.y - foot_h), area.max);
    crate::skin::slice::paint_image(
        ui.painter(),
        &skin.title_underline,
        Rect::from_center_size(
            pos2(foot.center().x, foot.min.y),
            vec2(foot.width() * 0.8, 3.0),
        ),
        egui::Color32::from_white_alpha(110),
    );
    let mut fui = ui.new_child(
        UiBuilder::new()
            .max_rect(foot.shrink2(vec2(6.0, 12.0)))
            .layout(Layout::top_down(Align::Center)),
    );
    fui.spacing_mut().item_spacing.y = 4.0;
    for (text, color) in [
        (
            "This product isn't affiliated with or endorsed by Grinding Gear Games in any way.",
            pal::LABEL,
        ),
        (
            "Game art and fonts belong to their owners. FerrisFilter is a free third-party              tool for Path of Exile 2 players.",
            pal::MUTED,
        ),
    ] {
        fui.add(
            egui::Label::new(
                egui::RichText::new(text)
                    .font(fonts::regular(15.0))
                    .color(color),
            )
            .halign(Align::Center)
            .wrap(),
        );
    }
}

// ---------------------------------------------------------------------------------------------
// Popups (SHELL-07)
// ---------------------------------------------------------------------------------------------

/// The seven confirmations and choices the overlay asks, each shown as one skinned modal.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Popup {
    Quit,
    ChangedOnDisk,
    SwitchFilter,
    DeleteRule,
    DeleteStyle,
    RemoveEconomy,
    DiscardConfig,
}

/// Which popups want to be open. Only one is shown at a time.
#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct PopupFlags {
    pub quit: bool,
    pub changed: bool,
    pub switch: bool,
    pub delete_rule: bool,
    pub delete_style: bool,
    pub remove_economy: bool,
    pub discard_config: bool,
}

/// The popup to show: Quit beats Changed-on-disk, which beats the rest (in the order of
/// [`Popup`]'s declaration after those two).
pub(crate) fn active_popup(f: PopupFlags) -> Option<Popup> {
    [
        (f.quit, Popup::Quit),
        (f.changed, Popup::ChangedOnDisk),
        (f.switch, Popup::SwitchFilter),
        (f.delete_rule, Popup::DeleteRule),
        (f.delete_style, Popup::DeleteStyle),
        (f.remove_economy, Popup::RemoveEconomy),
        (f.discard_config, Popup::DiscardConfig),
    ]
    .into_iter()
    .find_map(|(on, p)| on.then_some(p))
}

pub(crate) const KEEP_EDITING: &str = "Keep editing";
pub(crate) const SWITCH_AND_DISCARD: &str = "Switch and discard";
pub(crate) const STAY: &str = "Stay";
pub(crate) const DELETE: &str = "Delete";
pub(crate) const KEEP: &str = "Keep";
pub(crate) const DELETE_STYLE: &str = "Delete style";
pub(crate) const KEEP_STYLE: &str = "Keep style";
pub(crate) const REMOVE: &str = "Remove";
pub(crate) const DISCARD: &str = "Discard";
pub(crate) const RELOAD: &str = "Reload";
pub(crate) const OVERWRITE: &str = "Overwrite";

/// One popup button: caption, red caption, tooltip.
pub(crate) type SpecButton = (&'static str, bool, Option<&'static str>);

/// Everything a popup says. The last button is the safe dismiss (Escape, backdrop click).
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct PopupSpec {
    pub title: String,
    pub body: String,
    pub buttons: Vec<SpecButton>,
}

/// The facts a popup's text is made from.
#[derive(Debug, Clone)]
pub(crate) enum PopupData<'a> {
    ChangedOnDisk {
        files: &'a [String],
        /// Whether Overwrite is live; the same decision as the bottom bar's slot (WR-03). When it
        /// is not, the popup offers no Overwrite button.
        overwrite: bool,
    },
    SwitchFilter {
        target: &'a str,
    },
    Quit,
    DeleteRule {
        /// 1-based position of the rule in the list.
        index: usize,
        label: &'a str,
        /// Blocks the rule is written as (more than one for a grouped rule).
        blocks: usize,
    },
    DeleteStyle {
        prompt: String,
    },
    RemoveEconomy,
    DiscardConfig {
        count: usize,
    },
}

/// The changed-on-disk popup's choice as the full-set index (0 Reload, 1 Overwrite, 2 Keep
/// editing), from the index of the button clicked when Overwrite may have been left out.
pub(crate) fn changed_on_disk_choice(overwrite: bool, clicked: usize) -> usize {
    if overwrite || clicked == 0 {
        clicked
    } else {
        clicked + 1
    }
}

/// Title, body and buttons of a popup.
pub(crate) fn popup_spec(data: PopupData) -> PopupSpec {
    let (title, body, buttons): (&str, String, Vec<SpecButton>) = match data {
        PopupData::ChangedOnDisk { files, overwrite } => {
            let mut buttons = vec![(RELOAD, false, Some(apply::RELOAD_TOOLTIP))];
            let body = if overwrite {
                buttons.push((OVERWRITE, true, Some(apply::OVERWRITE_TOOLTIP)));
                format!(
                    "{} changed since ff loaded it. Reload discards your edits. Overwrite replaces the file with them.",
                    files.join(", ")
                )
            } else {
                format!(
                    "{} changed since ff loaded it. Reload discards your edits.",
                    files.join(", ")
                )
            };
            buttons.push((KEEP_EDITING, false, None));
            ("File changed on disk", body, buttons)
        }
        PopupData::SwitchFilter { target } => (
            "Switch filter?",
            format!("Unapplied rule changes will be discarded. Switching to {target}."),
            vec![(SWITCH_AND_DISCARD, true, None), (STAY, false, None)],
        ),
        PopupData::Quit => (
            apply::QUIT_CONFIRM_TITLE,
            "Apply them before quitting?".to_string(),
            vec![
                (apply::QUIT_APPLY_LABEL, false, None),
                (
                    apply::QUIT_DISCARD_LABEL,
                    true,
                    Some(apply::QUIT_DISCARD_TIP),
                ),
                (apply::QUIT_CANCEL_LABEL, false, None),
            ],
        ),
        PopupData::DeleteRule {
            index,
            label,
            blocks,
        } => {
            let mut body = format!("Rule {index}: {label}");
            if blocks > 1 {
                body.push_str(&format!("\nDeletes all {blocks} blocks of this rule."));
            }
            (
                "Delete this rule?",
                body,
                vec![(DELETE, true, None), (KEEP, false, None)],
            )
        }
        PopupData::DeleteStyle { prompt } => (
            "Delete style?",
            prompt,
            vec![(DELETE_STYLE, true, None), (KEEP_STYLE, false, None)],
        ),
        PopupData::RemoveEconomy => (
            "Remove economy section?",
            "Your own rules are kept. Add the section back any time.".to_string(),
            vec![(REMOVE, true, None), (KEEP, false, None)],
        ),
        PopupData::DiscardConfig { count } => (
            "Discard config changes?",
            super::config_view::revert_prompt(count),
            vec![(DISCARD, true, None), (KEEP_EDITING, false, None)],
        ),
    };
    PopupSpec {
        title: title.to_string(),
        body,
        buttons,
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn last_sync_text_formats_local_time() {
        assert_eq!(super::last_sync_text(Some(0), 0), "1970-01-01 00:00");
        assert_eq!(super::last_sync_text(None, 120), "Never");
        // 2023-11-14 22:13:20 UTC
        assert_eq!(
            super::last_sync_text(Some(1_700_000_000), 0),
            "2023-11-14 22:13"
        );
        assert_eq!(
            super::last_sync_text(Some(1_700_000_000), 120),
            "2023-11-15 00:13"
        );
        assert_eq!(
            super::last_sync_text(Some(1_700_000_000), -180),
            "2023-11-14 19:13"
        );
        // A negative offset before the epoch clamps instead of wrapping.
        assert_eq!(super::last_sync_text(Some(0), -60), "1970-01-01 00:00");
    }

    use super::*;

    fn id() -> BlockId {
        BlockId(1)
    }

    fn empty() -> StatusInputs {
        StatusInputs {
            root_error: None,
            event: None,
            grammar_gate: None,
            filter_error: None,
            syncing: false,
            unknown: None,
            migrated: false,
            absent: false,
            indicator: ("indicator", StatusKind::Info),
        }
    }

    fn full() -> StatusInputs {
        StatusInputs {
            root_error: None,
            event: Some(("event".into(), StatusKind::Ok)),
            grammar_gate: Some(("gate".into(), id())),
            filter_error: Some("load".into()),
            syncing: true,
            unknown: Some(("unknown".into(), id())),
            migrated: true,
            absent: true,
            indicator: ("indicator", StatusKind::Info),
        }
    }

    #[test]
    fn row1_root_error_beats_everything() {
        let mut i = full();
        i.root_error = Some("no root".into());
        let s = status_for(i);
        assert_eq!(s.text, "no root");
        assert_eq!(s.kind, StatusKind::Error);
        assert_eq!(s.jump, None);
    }

    #[test]
    fn row2_event_beats_standing_conditions() {
        let s = status_for(full());
        assert_eq!(s.text, "event");
        assert_eq!(s.kind, StatusKind::Ok);
        assert_eq!(s.jump, None);
    }

    #[test]
    fn row3_grammar_gate_jumps() {
        let mut i = full();
        i.event = None;
        let s = status_for(i);
        assert_eq!(s.text, "gate");
        assert_eq!(s.kind, StatusKind::Error);
        assert_eq!(s.jump, Some(id()));
    }

    #[test]
    fn row4_filter_error() {
        let mut i = full();
        i.event = None;
        i.grammar_gate = None;
        let s = status_for(i);
        assert_eq!(s.text, "load");
        assert_eq!(s.kind, StatusKind::Error);
        assert_eq!(s.jump, None);
    }

    #[test]
    fn row5_syncing() {
        let mut i = full();
        i.event = None;
        i.grammar_gate = None;
        i.filter_error = None;
        let s = status_for(i);
        assert_eq!(s.text, SYNC_RUNNING);
        assert_eq!(s.kind, StatusKind::Info);
    }

    #[test]
    fn row6_unknown_styles_warn_and_jump() {
        let mut i = full();
        i.event = None;
        i.grammar_gate = None;
        i.filter_error = None;
        i.syncing = false;
        let s = status_for(i);
        assert_eq!(s.text, "unknown");
        assert_eq!(s.kind, StatusKind::Warning);
        assert_eq!(s.jump, Some(id()));
    }

    #[test]
    fn row7_migrated() {
        let mut i = empty();
        i.migrated = true;
        i.absent = true;
        let s = status_for(i);
        assert_eq!(s.text, MIGRATED_STATUS);
        assert_eq!(s.kind, StatusKind::Info);
    }

    #[test]
    fn row8_absent() {
        let mut i = empty();
        i.absent = true;
        let s = status_for(i);
        assert_eq!(s.text, ABSENT_STATUS);
        assert_eq!(s.kind, StatusKind::Info);
    }

    #[test]
    fn row9_indicator_kinds() {
        for (state, kind) in [
            (ApplyState::NeverApplied, StatusKind::Warning),
            (ApplyState::Unapplied, StatusKind::Warning),
            (ApplyState::Applied, StatusKind::Info),
        ] {
            assert_eq!(indicator_kind(state), kind);
            let mut i = empty();
            i.indicator = (apply::indicator(state), indicator_kind(state));
            let s = status_for(i);
            assert_eq!(s.text, apply::indicator(state));
            assert_eq!(s.kind, kind);
        }
    }

    #[test]
    fn load_failed_text() {
        assert_eq!(
            load_failed_status("bad"),
            "The filter could not be loaded: bad. Fix the file and press Reload."
        );
    }

    #[test]
    fn event_precedence_and_kinds() {
        let e = event_from(
            Some(("a".into(), StatusKind::Ok)),
            Some(("c".into(), true)),
            Some(("s".into(), true)),
            Some("l".into()),
        );
        assert_eq!(e, Some(("a".into(), StatusKind::Ok)));
        // The stored kind passes through unchanged: a failure stays an Error whatever the
        // apply state is (CR-02).
        let e = event_from(Some(("a".into(), StatusKind::Error)), None, None, None);
        assert_eq!(e, Some(("a".into(), StatusKind::Error)));
        let text = format!("Wrote x. {}", apply::RELOAD_INSTRUCTION);
        assert_eq!(apply_success_kind(&text), StatusKind::Warning);
        assert_eq!(apply_success_kind("Wrote x."), StatusKind::Ok);
        let e = event_from(
            None,
            Some(("c".into(), true)),
            Some(("s".into(), true)),
            None,
        );
        assert_eq!(e, Some(("c".into(), StatusKind::Error)));
        let e = event_from(None, Some(("c".into(), false)), None, None);
        assert_eq!(e, Some(("c".into(), StatusKind::Info)));
        let e = event_from(None, None, Some(("s".into(), true)), Some("l".into()));
        assert_eq!(e, Some(("s".into(), StatusKind::Error)));
        let e = event_from(None, None, Some(("s".into(), false)), Some("l".into()));
        assert_eq!(e, Some(("s".into(), StatusKind::Ok)));
        let e = event_from(None, None, None, Some("l".into()));
        assert_eq!(e, Some(("l".into(), StatusKind::Info)));
        assert_eq!(event_from(None, None, None, None), None);
    }

    #[test]
    fn markers() {
        use ShellTab::*;
        assert_eq!(tab_markers(Rules, true, true), [false, true, false, false]);
        assert_eq!(tab_markers(Config, true, true), [true, false, false, false]);
        assert_eq!(tab_markers(Economy, true, true), [true, true, false, false]);
        assert_eq!(tab_markers(About, false, true), [false, true, false, false]);
        assert_eq!(
            tab_markers(Rules, true, false),
            [false, false, false, false]
        );
    }

    #[test]
    fn tab_helpers_round_trip() {
        for (i, tab) in ShellTab::ALL.into_iter().enumerate() {
            assert_eq!(tab.index(), i);
            assert_eq!(ShellTab::from_index(i), Some(tab));
            assert_eq!(ShellTab::from_arg(tab.label()), Some(tab));
        }
        assert_eq!(ShellTab::from_index(4), None);
        assert_eq!(ShellTab::from_arg("nope"), None);
    }

    fn actions(apply: bool, revert: bool, reload: bool, overwrite: bool) -> FooterActions {
        FooterActions {
            sync: true,
            apply,
            revert,
            add_rule: true,
            reload,
            overwrite,
        }
    }

    #[test]
    fn rules_bar_mirrors_footer_actions() {
        let b = bar_state(
            ShellTab::Rules,
            actions(true, true, false, false),
            ConfigBarInputs::default(),
            false,
        );
        assert!(b.revert && !b.reload && b.apply.enabled);
        assert_eq!(b.apply.label, "Apply");
        assert_eq!(b.apply.tip, None);
        assert_eq!(b.reload_tip, NOTHING_TO_RELOAD);
        let b = bar_state(
            ShellTab::Rules,
            actions(false, false, true, false),
            ConfigBarInputs::default(),
            false,
        );
        assert!(!b.revert && b.reload && !b.apply.enabled);
        assert_eq!(b.reload_tip, apply::RELOAD_TOOLTIP);
    }

    #[test]
    fn config_bar_uses_config_state() {
        let cfg = ConfigBarInputs {
            dirty: true,
            apply_enabled: true,
            ..Default::default()
        };
        let b = bar_state(
            ShellTab::Config,
            actions(false, false, false, false),
            cfg,
            false,
        );
        assert!(b.revert);
        assert!(b.apply.enabled);
    }

    #[test]
    fn economy_and_about_have_no_revert() {
        for tab in [ShellTab::Economy, ShellTab::About] {
            let b = bar_state(
                tab,
                actions(true, true, false, false),
                ConfigBarInputs {
                    dirty: true,
                    ..Default::default()
                },
                false,
            );
            assert!(!b.revert);
            assert!(b.apply.enabled);
        }
    }

    #[test]
    fn needs_reload_turns_apply_into_overwrite() {
        let b = bar_state(
            ShellTab::Rules,
            actions(false, false, true, true),
            ConfigBarInputs::default(),
            true,
        );
        assert_eq!(b.apply.label, "Overwrite");
        assert!(b.apply.danger && b.apply.overwrite && b.apply.enabled);
        assert_eq!(b.apply.tip, Some(apply::OVERWRITE_TOOLTIP));
        let b = bar_state(
            ShellTab::Rules,
            actions(false, false, true, false),
            ConfigBarInputs::default(),
            true,
        );
        assert!(!b.apply.enabled);
    }

    #[test]
    fn disabled_apply_tip_chain() {
        let a = actions(false, false, false, false);
        let tip = |cfg: ConfigBarInputs| bar_state(ShellTab::Config, a, cfg, false).apply.tip;
        let all = ConfigBarInputs {
            tier_errors: true,
            load_error: true,
            rules_blocking: true,
            damaged: true,
            ..Default::default()
        };
        assert_eq!(tip(all), Some(config_state::APPLY_BLOCKED_TOOLTIP));
        let all = ConfigBarInputs {
            tier_errors: false,
            ..all
        };
        assert_eq!(tip(all), Some(APPLY_LOAD_ERROR_TIP));
        let all = ConfigBarInputs {
            load_error: false,
            ..all
        };
        assert_eq!(tip(all), Some(apply::APPLY_BLOCKED_TOOLTIP));
        let all = ConfigBarInputs {
            rules_blocking: false,
            ..all
        };
        assert_eq!(tip(all), Some(apply::DAMAGED_MARKERS_MESSAGE));
        assert_eq!(tip(ConfigBarInputs::default()), Some(APPLY_NOTHING_TIP));
    }

    #[test]
    fn popup_stacking_order() {
        let all = PopupFlags {
            quit: true,
            changed: true,
            switch: true,
            delete_rule: true,
            delete_style: true,
            remove_economy: true,
            discard_config: true,
        };
        assert_eq!(active_popup(all), Some(Popup::Quit));
        let f = PopupFlags { quit: false, ..all };
        assert_eq!(active_popup(f), Some(Popup::ChangedOnDisk));
        let f = PopupFlags {
            changed: false,
            ..f
        };
        assert_eq!(active_popup(f), Some(Popup::SwitchFilter));
        let f = PopupFlags { switch: false, ..f };
        assert_eq!(active_popup(f), Some(Popup::DeleteRule));
        let f = PopupFlags {
            delete_rule: false,
            ..f
        };
        assert_eq!(active_popup(f), Some(Popup::DeleteStyle));
        let f = PopupFlags {
            delete_style: false,
            ..f
        };
        assert_eq!(active_popup(f), Some(Popup::RemoveEconomy));
        let f = PopupFlags {
            remove_economy: false,
            ..f
        };
        assert_eq!(active_popup(f), Some(Popup::DiscardConfig));
        assert_eq!(active_popup(PopupFlags::default()), None);
    }

    fn labels(s: &PopupSpec) -> Vec<&'static str> {
        s.buttons.iter().map(|b| b.0).collect()
    }

    #[test]
    fn changed_on_disk_popup_drops_a_disabled_overwrite() {
        let files = vec!["styles.json".to_string()];
        let s = popup_spec(PopupData::ChangedOnDisk {
            files: &files,
            overwrite: false,
        });
        assert_eq!(labels(&s), ["Reload", "Keep editing"]);
        assert!(!s.body.contains("Overwrite"), "{}", s.body);
        assert_eq!(changed_on_disk_choice(false, 0), 0);
        assert_eq!(changed_on_disk_choice(false, 1), 2);
        for i in 0..3 {
            assert_eq!(changed_on_disk_choice(true, i), i);
        }
    }

    #[test]
    fn popup_specs_match_the_contract() {
        let files = vec!["dynamic.filter".to_string(), "styles.json".to_string()];
        let s = popup_spec(PopupData::ChangedOnDisk {
            files: &files,
            overwrite: true,
        });
        assert_eq!(s.title, "File changed on disk");
        assert!(
            s.body
                .starts_with("dynamic.filter, styles.json changed since ff loaded it.")
        );
        assert!(
            s.body
                .contains("Reload discards your edits. Overwrite replaces the file with them.")
        );
        assert_eq!(labels(&s), ["Reload", "Overwrite", "Keep editing"]);
        assert_eq!(s.buttons[0].2, Some(apply::RELOAD_TOOLTIP));
        assert_eq!(
            s.buttons[1],
            ("Overwrite", true, Some(apply::OVERWRITE_TOOLTIP))
        );

        let s = popup_spec(PopupData::SwitchFilter { target: "a.filter" });
        assert_eq!(s.title, "Switch filter?");
        assert!(s.body.contains("Unapplied rule changes will be discarded."));
        assert!(s.body.contains("Switching to a.filter."));
        assert_eq!(labels(&s), ["Switch and discard", "Stay"]);
        assert!(s.buttons[0].1);

        let s = popup_spec(PopupData::Quit);
        assert_eq!(s.title, apply::QUIT_CONFIRM_TITLE);
        assert_eq!(s.body, "Apply them before quitting?");
        assert_eq!(
            labels(&s),
            [
                apply::QUIT_APPLY_LABEL,
                apply::QUIT_DISCARD_LABEL,
                apply::QUIT_CANCEL_LABEL
            ]
        );
        assert_eq!(
            s.buttons[1],
            (
                apply::QUIT_DISCARD_LABEL,
                true,
                Some(apply::QUIT_DISCARD_TIP)
            )
        );

        let one = popup_spec(PopupData::DeleteRule {
            index: 3,
            label: "Boots",
            blocks: 1,
        });
        assert_eq!(one.title, "Delete this rule?");
        assert_eq!(one.body, "Rule 3: Boots");
        assert_eq!(labels(&one), ["Delete", "Keep"]);
        let four = popup_spec(PopupData::DeleteRule {
            index: 3,
            label: "Boots",
            blocks: 4,
        });
        assert!(four.body.contains("Deletes all 4 blocks of this rule."));

        let s = popup_spec(PopupData::DeleteStyle { prompt: "p".into() });
        assert_eq!((s.title.as_str(), s.body.as_str()), ("Delete style?", "p"));
        assert_eq!(labels(&s), ["Delete style", "Keep style"]);

        let s = popup_spec(PopupData::RemoveEconomy);
        assert_eq!(s.title, "Remove economy section?");
        assert_eq!(
            s.body,
            "Your own rules are kept. Add the section back any time."
        );
        assert_eq!(labels(&s), ["Remove", "Keep"]);

        let s = popup_spec(PopupData::DiscardConfig { count: 2 });
        assert_eq!(s.title, "Discard config changes?");
        assert_eq!(s.body, super::super::config_view::revert_prompt(2));
        assert_eq!(labels(&s), ["Discard", "Keep editing"]);
    }

    #[test]
    fn the_last_button_is_the_safe_one_and_never_danger() {
        let files = vec!["x".to_string()];
        for data in [
            PopupData::ChangedOnDisk {
                files: &files,
                overwrite: true,
            },
            PopupData::ChangedOnDisk {
                files: &files,
                overwrite: false,
            },
            PopupData::SwitchFilter { target: "t" },
            PopupData::Quit,
            PopupData::DeleteRule {
                index: 1,
                label: "l",
                blocks: 1,
            },
            PopupData::DeleteStyle {
                prompt: String::new(),
            },
            PopupData::RemoveEconomy,
            PopupData::DiscardConfig { count: 1 },
        ] {
            let s = popup_spec(data);
            let last = s.buttons.last().unwrap();
            assert!(!last.1, "{}: safe button is not danger", s.title);
            assert!(s.buttons.len() >= 2);
        }
    }
}
