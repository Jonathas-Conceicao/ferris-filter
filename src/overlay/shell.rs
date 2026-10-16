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

/// The event message, if any: the Apply outcome wins over a CONFIG message, then a finished sync,
/// then the list status.
///
/// `apply_ok_state` is true when the apply state is `Applied` and no file changed on disk; the
/// message is then Ok, or Warning when it carries the in-game reload instruction. Otherwise it is
/// an Error. A CONFIG message is `(text, is_problem)`. A sync outcome is `(text, is_failure)`.
pub(crate) fn event_from(
    apply_message: Option<String>,
    apply_ok_state: bool,
    config_message: Option<(String, bool)>,
    sync: Option<(String, bool)>,
    list_status: Option<String>,
) -> Option<(String, StatusKind)> {
    if let Some(text) = apply_message {
        let kind = if !apply_ok_state {
            StatusKind::Error
        } else if text.contains(apply::RELOAD_INSTRUCTION) {
            StatusKind::Warning
        } else {
            StatusKind::Ok
        };
        return Some((text, kind));
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
            Some("a".into()),
            true,
            Some(("c".into(), true)),
            Some(("s".into(), true)),
            Some("l".into()),
        );
        assert_eq!(e, Some(("a".into(), StatusKind::Ok)));
        let e = event_from(Some("a".into()), false, None, None, None);
        assert_eq!(e, Some(("a".into(), StatusKind::Error)));
        let text = format!("Wrote x. {}", apply::RELOAD_INSTRUCTION);
        let e = event_from(Some(text.clone()), true, None, None, None);
        assert_eq!(e, Some((text, StatusKind::Warning)));
        let e = event_from(
            None,
            true,
            Some(("c".into(), true)),
            Some(("s".into(), true)),
            None,
        );
        assert_eq!(e, Some(("c".into(), StatusKind::Error)));
        let e = event_from(None, true, Some(("c".into(), false)), None, None);
        assert_eq!(e, Some(("c".into(), StatusKind::Info)));
        let e = event_from(None, true, None, Some(("s".into(), true)), Some("l".into()));
        assert_eq!(e, Some(("s".into(), StatusKind::Error)));
        let e = event_from(
            None,
            true,
            None,
            Some(("s".into(), false)),
            Some("l".into()),
        );
        assert_eq!(e, Some(("s".into(), StatusKind::Ok)));
        let e = event_from(None, true, None, None, Some("l".into()));
        assert_eq!(e, Some(("l".into(), StatusKind::Info)));
        assert_eq!(event_from(None, true, None, None, None), None);
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
}
