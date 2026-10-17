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
use super::rules::EconomyView;

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

/// Which tabs show the ` *` marker: a tab is marked when it holds unapplied edits, the selected
/// tab included. RULES owns rule edits, CONFIG owns style edits, ECONOMY owns tier edits and a
/// staged economy add/remove (its button and Revert live there); About is never marked.
/// `rules_dirty` must not include the staged economy op.
pub(crate) fn tab_markers(
    rules_dirty: bool,
    styles_dirty: bool,
    tiers_dirty: bool,
    economy_staged: bool,
) -> [bool; 4] {
    [
        rules_dirty,
        styles_dirty,
        tiers_dirty || economy_staged,
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
    /// A hover explanation for a short standing message (the unapplied indicator).
    pub tip: Option<&'static str>,
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
        tip: None,
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
            tip: None,
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
            tip: None,
        };
    }
    if inputs.migrated {
        return plain(MIGRATED_STATUS.to_string(), StatusKind::Info);
    }
    if inputs.absent {
        return plain(ABSENT_STATUS.to_string(), StatusKind::Info);
    }
    StatusLine {
        tip: apply::indicator_tip(inputs.indicator.0),
        ..plain(inputs.indicator.0.to_string(), inputs.indicator.1)
    }
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
    /// Style edits (CONFIG tab).
    pub styles_dirty: bool,
    /// Tier cutoff edits (ECONOMY tab).
    pub tiers_dirty: bool,
    /// An economy add/remove is staged (ECONOMY tab).
    pub economy_staged: bool,
    pub apply_enabled: bool,
    pub tier_errors: bool,
    pub load_error: bool,
    pub rules_blocking: bool,
    pub damaged: bool,
    /// A loaded config that fails validation (tier errors, invalid styles.json/tiers.json).
    pub config_invalid: bool,
}

/// Overwrite's tooltip when it is unavailable because of the config.
pub(crate) const OVERWRITE_CONFIG_INVALID_TIP: &str =
    "styles.json or tiers.json is invalid. Fix it first.";
/// Overwrite's tooltip when nothing can be written.
pub(crate) const OVERWRITE_NOTHING_TIP: &str = "Nothing to overwrite.";

/// Why Overwrite is unavailable during a changed-on-disk refusal, for the bar slot's tooltip and
/// a short line in the popup (review WR-02, iteration 2). The order follows the Apply slot's
/// reason chain.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum OverwriteBlock {
    TierErrors,
    ConfigInvalid,
    RulesBlocking,
    Damaged,
    Nothing,
}

impl OverwriteBlock {
    pub(crate) fn from_inputs(config: ConfigBarInputs) -> Self {
        if config.tier_errors {
            Self::TierErrors
        } else if config.config_invalid {
            Self::ConfigInvalid
        } else if config.rules_blocking {
            Self::RulesBlocking
        } else if config.damaged {
            Self::Damaged
        } else {
            Self::Nothing
        }
    }

    /// The disabled Overwrite button's tooltip.
    pub(crate) fn tip(self) -> &'static str {
        match self {
            Self::TierErrors => config_state::APPLY_BLOCKED_TOOLTIP,
            Self::ConfigInvalid => OVERWRITE_CONFIG_INVALID_TIP,
            Self::RulesBlocking => apply::APPLY_BLOCKED_TOOLTIP,
            Self::Damaged => apply::DAMAGED_MARKERS_MESSAGE,
            Self::Nothing => OVERWRITE_NOTHING_TIP,
        }
    }

    /// The popup's extra sentence: how to get Overwrite back, when there is a way.
    pub(crate) fn popup_clause(self) -> Option<&'static str> {
        match self {
            Self::TierErrors => Some("Fix the tier errors to overwrite instead."),
            Self::ConfigInvalid => Some("Fix styles.json or tiers.json to overwrite instead."),
            Self::RulesBlocking => Some("Fix or disable the flagged rules to overwrite instead."),
            Self::Damaged => Some("Overwrite is off while the economy markers are damaged."),
            Self::Nothing => None,
        }
    }
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
/// its style edits and Economy's follows tier edits or a staged economy add/remove; both use
/// `config_apply_enabled` for Apply. About has no Revert and uses the combined Apply decision.
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
        // As on RULES, no Revert during a changed-on-disk refusal: only Reload or Overwrite
        // resolve it (v0.5; review WR-04).
        ShellTab::Config => config.styles_dirty && !needs_reload,
        ShellTab::Economy => (config.tiers_dirty || config.economy_staged) && !needs_reload,
        ShellTab::About => false,
    };
    let apply = if needs_reload {
        ApplySlot {
            label: "Overwrite",
            enabled: actions.overwrite,
            danger: true,
            // A dead Overwrite says why (WR-02, iteration 2).
            tip: Some(if actions.overwrite {
                apply::OVERWRITE_TOOLTIP
            } else {
                OverwriteBlock::from_inputs(config).tip()
            }),
            overwrite: true,
        }
    } else {
        let enabled = match tab {
            ShellTab::Config | ShellTab::Economy => config.apply_enabled,
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
/// placeholder plate; today it is the custom header from `assets/custom/`.
pub(crate) fn header_art(skin: &Skin) -> Option<&TextureHandle> {
    Some(&skin.header)
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

/// "Last sync" as a relative age: `Never`, `Just now`, `N min ago`, `N h ago`, `N d ago`, and the
/// local `YYYY-MM-DD` date from 14 days on. A modified time in the future counts as `Just now`.
pub(crate) fn relative_sync_text(
    mtime_unix_secs: Option<u64>,
    now_unix_secs: u64,
    utc_offset_minutes: i32,
) -> String {
    let Some(mtime) = mtime_unix_secs else {
        return "Never".to_string();
    };
    let age = now_unix_secs.saturating_sub(mtime);
    if age < 60 {
        "Just now".to_string()
    } else if age < 3_600 {
        format!("{} min ago", age / 60)
    } else if age < 86_400 {
        format!("{} h ago", age / 3_600)
    } else if age < 14 * 86_400 {
        format!("{} d ago", age / 86_400)
    } else {
        let local = (mtime as i64 + i64::from(utc_offset_minutes) * 60).max(0) as u64;
        let (year, month, day) = crate::sync::prices::days_to_ymd(local / 86_400);
        format!("{year:04}-{month:02}-{day:02}")
    }
}

/// A count with comma thousands separators (`1,284`).
pub(crate) fn thousands(n: usize) -> String {
    let digits = n.to_string();
    let mut out = String::with_capacity(digits.len() + digits.len() / 3);
    for (i, c) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i).is_multiple_of(3) {
            out.push(',');
        }
        out.push(c);
    }
    out
}

/// Tooltip of the economy section button when it is unavailable.
pub(crate) const FIX_FILTER_FIRST: &str = "Fix the filter first.";

/// The Add / Remove economy section button of the ECONOMY tab.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct SectionButton {
    pub add: bool,
    pub enabled: bool,
    pub blocks: Option<usize>,
}

impl SectionButton {
    pub(crate) fn label(&self) -> &'static str {
        if self.add {
            "Add economy section"
        } else {
            "Remove economy section"
        }
    }

    pub(crate) fn danger(&self) -> bool {
        !self.add
    }

    pub(crate) fn tip(&self) -> Option<&'static str> {
        (!self.enabled).then_some(FIX_FILTER_FIRST)
    }
}

/// The section button for the working filter's economy view (None: no filter loaded).
pub(crate) fn section_button(
    view: Option<&EconomyView>,
    root_ok: bool,
    changed_on_disk: bool,
) -> SectionButton {
    let usable = root_ok && !changed_on_disk;
    match view {
        None | Some(EconomyView::Damaged(_)) => SectionButton {
            add: true,
            enabled: false,
            blocks: None,
        },
        Some(EconomyView::Absent) => SectionButton {
            add: true,
            enabled: usable,
            blocks: None,
        },
        Some(EconomyView::Present { blocks }) => SectionButton {
            add: false,
            enabled: usable,
            blocks: Some(*blocks),
        },
    }
}

/// What the clicks on the ECONOMY tab asked for; the app performs them.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub(crate) struct EconomyClicks {
    pub sync: bool,
    pub add: bool,
    pub remove: bool,
}

/// Everything the ECONOMY tab shows.
pub(crate) struct EconomyTabInputs<'a> {
    pub league: &'a str,
    pub mtime_secs: Option<u64>,
    pub now_secs: u64,
    pub utc_offset_minutes: i32,
    pub priced_items: usize,
    pub live: bool,
    /// Why an existing prices.json cannot be used (shown instead of "No prices.json yet").
    pub problem: Option<&'a str>,
    pub syncing: bool,
    pub sync_enabled: bool,
    pub sync_disabled_tip: Option<&'a str>,
    pub section: SectionButton,
}

/// The ECONOMY tab: league, last sync, price source, the Sync economy button, the economy
/// section button and (through `tiers`) the tier cutoffs editor. Draws only; returns the clicks.
pub(crate) fn economy_tab(
    ui: &mut egui::Ui,
    skin: &Skin,
    inputs: &EconomyTabInputs,
    tiers: impl FnOnce(&mut egui::Ui),
) -> EconomyClicks {
    use egui::{Align, Layout, Sense, UiBuilder, vec2};
    let mut clicks = EconomyClicks::default();
    w::ff_scroll_area(ui, skin, "economy", |ui| {
        w::ff_section(ui, skin, "Economy");
        let value = |ui: &mut egui::Ui, text: &str, color: egui::Color32| {
            ui.label(
                egui::RichText::new(text)
                    .font(fonts::regular(15.0))
                    .color(color),
            )
        };
        w::ff_row(ui, 0, "League", |ui| {
            value(ui, inputs.league, pal::VALUE).on_hover_text("Pinned in this build.");
        });
        w::ff_row(ui, 1, "Last sync", |ui| {
            let r = value(
                ui,
                &relative_sync_text(
                    inputs.mtime_secs,
                    inputs.now_secs,
                    inputs.utc_offset_minutes,
                ),
                pal::VALUE,
            );
            if inputs.mtime_secs.is_some() {
                r.on_hover_text(last_sync_text(inputs.mtime_secs, inputs.utc_offset_minutes));
            }
        });
        w::ff_row(ui, 2, "Priced items", |ui| {
            value(ui, &thousands(inputs.priced_items), pal::VALUE);
        });
        w::ff_row(ui, 3, "Price source", |ui| {
            if inputs.live {
                value(ui, "Live prices", pal::VALUE)
                    .on_hover_text("prices.json from the last sync.");
            } else if let Some(problem) = inputs.problem {
                value(ui, "Fallback prices", pal::WARN).on_hover_text(format!(
                    "prices.json could not be used ({problem}) so the built-in early-league prices apply. Sync to replace it."
                ));
            } else {
                value(ui, "Fallback prices", pal::WARN).on_hover_text(
                    "No prices.json yet. Using the built-in early-league prices. Sync to get live ones.",
                );
            }
        });
        ui.add_space(16.0);
        ui.with_layout(Layout::top_down(Align::Center), |ui| {
            let label = if inputs.syncing {
                "Syncing…"
            } else {
                "Sync economy"
            };
            let enabled = inputs.sync_enabled && !inputs.syncing;
            let mut r = w::ff_button_ex(ui, skin, label, 180.0, enabled, false);
            if !enabled
                && !inputs.syncing
                && let Some(tip) = inputs.sync_disabled_tip
            {
                r = r.on_hover_text(tip);
            }
            if r.clicked() {
                clicks.sync = true;
            }
        });
        ui.add_space(16.0);
        w::ff_section(ui, skin, crate::overlay::rules::ECONOMY_DISPLAY_NAME);
        let (rect, _) =
            ui.allocate_exact_size(vec2(ui.available_width(), w::ROW_HEIGHT), Sense::hover());
        w::row_background(ui, 0, rect);
        let inner = rect.shrink2(vec2(w::ROW_PAD, 0.0));
        let mut left = ui.new_child(
            UiBuilder::new()
                .max_rect(inner)
                .layout(Layout::left_to_right(Align::Center)),
        );
        left.label(
            egui::RichText::new("Filter section")
                .font(fonts::small_caps(17.0))
                .color(pal::LABEL),
        )
        .on_hover_text(
            "The economy blocks FerrisFilter writes at the end of the filter. Added and removed by Apply.",
        );
        let mut right = ui.new_child(
            UiBuilder::new()
                .max_rect(inner)
                .layout(Layout::right_to_left(Align::Center)),
        );
        let section = inputs.section;
        let mut r = w::ff_button_ex(
            &mut right,
            skin,
            section.label(),
            188.0,
            section.enabled,
            section.danger(),
        );
        if let Some(tip) = section.tip() {
            r = r.on_hover_text(tip);
        }
        if r.clicked() {
            if section.add {
                clicks.add = true;
            } else {
                clicks.remove = true;
            }
        }
        if let Some(n) = section.blocks
            && !section.add
        {
            value(&mut right, &format!("{n} locked blocks"), pal::MUTED)
                .on_hover_text("Rules inside the section cannot be edited.");
        }
        ui.add_space(16.0);
        w::ff_section(ui, skin, "Tiers");
        tiers(ui);
    });
    clicks
}

// ---------------------------------------------------------------------------------------------
// Popups (SHELL-07)
// ---------------------------------------------------------------------------------------------

/// The eight confirmations and choices the overlay asks, each shown as one skinned modal.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Popup {
    Quit,
    ChangedOnDisk,
    SwitchFilter,
    DeleteRule,
    DeleteStyle,
    RemoveEconomy,
    DiscardConfig,
    DiscardRules,
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
    pub discard_rules: bool,
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
        (f.discard_rules, Popup::DiscardRules),
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
        /// Why Overwrite is unavailable; read only when `overwrite` is false.
        block: OverwriteBlock,
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
        /// Unapplied edits in the reverted scope (styles on CONFIG, tiers on ECONOMY).
        count: usize,
        /// The ECONOMY Revert (tier cutoffs plus a staged economy add/remove).
        economy: bool,
        /// A staged economy add/remove goes too (ECONOMY only).
        staged: bool,
    },
    DiscardRules {
        /// The filter's file name; empty when unknown.
        filter: &'a str,
    },
}

/// The ECONOMY Discard popup's body: the tier edits and/or the staged section add/remove.
pub(crate) fn economy_revert_prompt(count: usize, staged: bool) -> String {
    let tiers = match count {
        0 => String::new(),
        1 => "1 unapplied tier change".to_string(),
        n => format!("{n} unapplied tier changes"),
    };
    match (tiers.is_empty(), staged) {
        (true, _) => "Discard the staged economy section change?".to_string(),
        (false, true) => format!("Discard {tiers} and the staged economy section change?"),
        (false, false) => format!("Discard {tiers}?"),
    }
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
        PopupData::ChangedOnDisk {
            files,
            overwrite,
            block,
        } => {
            let mut buttons = vec![(RELOAD, false, Some(apply::RELOAD_TOOLTIP))];
            let body = if overwrite {
                buttons.push((OVERWRITE, true, Some(apply::OVERWRITE_TOOLTIP)));
                format!(
                    "{} changed since ff loaded it. Reload discards your edits. Overwrite replaces the file with them.",
                    files.join(", ")
                )
            } else {
                let mut body = format!(
                    "{} changed since ff loaded it. Reload discards your edits.",
                    files.join(", ")
                );
                if let Some(clause) = block.popup_clause() {
                    body.push(' ');
                    body.push_str(clause);
                }
                body
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
        PopupData::DiscardConfig {
            count,
            economy,
            staged,
        } => {
            if economy {
                (
                    "Discard economy changes?",
                    economy_revert_prompt(count, staged),
                    vec![(DISCARD, true, None), (KEEP_EDITING, false, None)],
                )
            } else {
                (
                    "Discard config changes?",
                    super::config_view::revert_prompt(count),
                    vec![(DISCARD, true, None), (KEEP_EDITING, false, None)],
                )
            }
        }
        PopupData::DiscardRules { filter } => (
            "Discard rule changes?",
            if filter.is_empty() {
                "Unapplied rule changes will be discarded.".to_string()
            } else {
                format!("Unapplied rule changes to {filter} will be discarded.")
            },
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
    use super::*;

    fn draw(inputs: &EconomyTabInputs) -> (EconomyClicks, u32) {
        let ctx = egui::Context::default();
        crate::skin::fonts::install(&ctx);
        let skin = crate::skin::textures::load_light(&ctx);
        let mut ran = 0;
        let mut clicks = EconomyClicks::default();
        let raw = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(535.0, 600.0),
            )),
            ..Default::default()
        };
        ctx.run_ui(raw, |ui| {
            clicks = economy_tab(ui, &skin, inputs, |_| ran += 1);
        })
        .drop_without_applying_deltas();
        (clicks, ran)
    }

    fn inputs(section: SectionButton) -> EconomyTabInputs<'static> {
        EconomyTabInputs {
            league: "Test League",
            mtime_secs: None,
            now_secs: 1_700_000_000,
            utc_offset_minutes: 0,
            priced_items: 1284,
            live: false,
            problem: None,
            syncing: false,
            sync_enabled: true,
            sync_disabled_tip: None,
            section,
        }
    }

    const ADD: SectionButton = SectionButton {
        add: true,
        enabled: true,
        blocks: None,
    };

    #[test]
    fn economy_clicks_default_is_all_false() {
        assert_eq!(
            EconomyClicks::default(),
            EconomyClicks {
                sync: false,
                add: false,
                remove: false
            }
        );
    }

    #[test]
    fn economy_tab_renders_every_state_and_runs_tiers_once() {
        let mut live = inputs(ADD);
        live.mtime_secs = Some(1_699_999_000);
        live.live = true;
        let mut syncing = inputs(SectionButton {
            add: false,
            enabled: true,
            blocks: Some(7),
        });
        syncing.syncing = true;
        let mut disabled = inputs(SectionButton {
            add: true,
            enabled: false,
            blocks: None,
        });
        disabled.sync_enabled = false;
        disabled.sync_disabled_tip = Some("No app root");
        let mut broken = inputs(ADD);
        broken.priced_items = 0;
        broken.problem = Some("expected value at line 1");
        for i in [inputs(ADD), live, syncing, disabled, broken] {
            let (clicks, ran) = draw(&i);
            assert_eq!(ran, 1);
            assert_eq!(clicks, EconomyClicks::default());
        }
    }

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
            assert_eq!(s.tip.is_some(), state != ApplyState::Applied);
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
        assert_eq!(
            tab_markers(false, false, true, false),
            [false, false, true, false]
        );
        assert_eq!(
            tab_markers(true, true, true, false),
            [true, true, true, false]
        );
        assert_eq!(
            tab_markers(true, false, false, false),
            [true, false, false, false]
        );
        // About is never marked; a dirty selected tab is marked like any other.
        assert_eq!(
            tab_markers(false, true, false, false),
            [false, true, false, false]
        );
    }

    #[test]
    fn a_staged_economy_op_marks_economy_not_rules() {
        // Only a staged op: rules_dirty is false (the caller passes rule edits only).
        assert_eq!(
            tab_markers(false, false, false, true),
            [false, false, true, false]
        );
    }

    #[test]
    fn relative_sync_buckets() {
        let now = 10_000_000u64;
        assert_eq!(relative_sync_text(None, now, 0), "Never");
        assert_eq!(relative_sync_text(Some(now - 30), now, 0), "Just now");
        assert_eq!(relative_sync_text(Some(now + 500), now, 0), "Just now");
        assert_eq!(
            relative_sync_text(Some(now - 59 * 60), now, 0),
            "59 min ago"
        );
        assert_eq!(relative_sync_text(Some(now - 61 * 60), now, 0), "1 h ago");
        assert_eq!(
            relative_sync_text(Some(now - 23 * 3600), now, 0),
            "23 h ago"
        );
        assert_eq!(
            relative_sync_text(Some(now - 3 * 86_400), now, 0),
            "3 d ago"
        );
        assert_eq!(
            relative_sync_text(Some(now - 13 * 86_400), now, 0),
            "13 d ago"
        );
        // 2026-01-01 00:00 UTC is 1767225600; at +60 minutes it is still the 1st, at -60 the 31st.
        let mtime = 1_767_225_600u64;
        let late = mtime + 14 * 86_400;
        assert_eq!(relative_sync_text(Some(mtime), late, 0), "2026-01-01");
        assert_eq!(relative_sync_text(Some(mtime), late, -60), "2025-12-31");
    }

    #[test]
    fn thousands_groups() {
        assert_eq!(thousands(0), "0");
        assert_eq!(thousands(999), "999");
        assert_eq!(thousands(1000), "1,000");
        assert_eq!(thousands(1284), "1,284");
        assert_eq!(thousands(1_234_567), "1,234,567");
    }

    #[test]
    fn section_button_decisions() {
        let b = section_button(None, true, false);
        assert!(b.add && !b.enabled && b.blocks.is_none());
        assert_eq!(b.label(), "Add economy section");
        assert_eq!(b.tip(), Some("Fix the filter first."));
        let b = section_button(Some(&EconomyView::Absent), true, false);
        assert!(b.add && b.enabled && b.tip().is_none() && !b.danger());
        let b = section_button(Some(&EconomyView::Present { blocks: 12 }), true, false);
        assert!(!b.add && b.enabled && b.danger());
        assert_eq!(b.blocks, Some(12));
        assert_eq!(b.label(), "Remove economy section");
        let b = section_button(Some(&EconomyView::Damaged("x".into())), true, false);
        assert!(!b.enabled);
        for (root_ok, changed) in [(false, false), (true, true)] {
            let b = section_button(Some(&EconomyView::Absent), root_ok, changed);
            assert!(!b.enabled);
            assert_eq!(b.tip(), Some(FIX_FILTER_FIRST));
        }
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
            styles_dirty: true,
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
    fn about_has_no_revert() {
        let b = bar_state(
            ShellTab::About,
            actions(true, true, false, false),
            ConfigBarInputs {
                styles_dirty: true,
                tiers_dirty: true,
                ..Default::default()
            },
            false,
        );
        assert!(!b.revert);
        assert!(b.apply.enabled);
    }

    #[test]
    fn revert_is_scoped_per_tab() {
        let a = actions(false, false, false, false);
        let styles = ConfigBarInputs {
            styles_dirty: true,
            ..Default::default()
        };
        let tiers = ConfigBarInputs {
            tiers_dirty: true,
            ..Default::default()
        };
        let staged = ConfigBarInputs {
            economy_staged: true,
            ..Default::default()
        };
        assert!(bar_state(ShellTab::Config, a, styles, false).revert);
        assert!(!bar_state(ShellTab::Config, a, tiers, false).revert);
        assert!(!bar_state(ShellTab::Economy, a, styles, false).revert);
        assert!(bar_state(ShellTab::Economy, a, tiers, false).revert);
        assert!(bar_state(ShellTab::Economy, a, staged, false).revert);
        assert!(!bar_state(ShellTab::Economy, a, tiers, true).revert);
    }

    #[test]
    fn economy_apply_follows_config_gate() {
        let a = actions(true, false, false, false);
        let blocked = ConfigBarInputs {
            apply_enabled: false,
            tier_errors: true,
            ..Default::default()
        };
        let b = bar_state(ShellTab::Economy, a, blocked, false);
        assert!(!b.apply.enabled);
        assert_eq!(b.apply.tip, Some(config_state::APPLY_BLOCKED_TOOLTIP));
        let ok = ConfigBarInputs {
            apply_enabled: true,
            ..Default::default()
        };
        assert!(bar_state(ShellTab::Economy, a, ok, false).apply.enabled);
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
    fn config_revert_is_off_during_a_refusal() {
        let cfg = ConfigBarInputs {
            styles_dirty: true,
            ..ConfigBarInputs::default()
        };
        let a = actions(false, true, true, true);
        assert!(bar_state(ShellTab::Config, a, cfg, false).revert);
        assert!(!bar_state(ShellTab::Config, a, cfg, true).revert);
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
            discard_rules: true,
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
        let f = PopupFlags {
            discard_config: false,
            ..f
        };
        assert_eq!(active_popup(f), Some(Popup::DiscardRules));
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
            block: OverwriteBlock::Nothing,
        });
        assert_eq!(labels(&s), ["Reload", "Keep editing"]);
        assert!(!s.body.contains("Overwrite"), "{}", s.body);
        assert_eq!(changed_on_disk_choice(false, 0), 0);
        assert_eq!(changed_on_disk_choice(false, 1), 2);
        for i in 0..3 {
            assert_eq!(changed_on_disk_choice(true, i), i);
        }
    }

    /// WR-02 (iteration 2): a dead Overwrite says why, on the bar and in the popup.
    #[test]
    fn a_disabled_overwrite_explains_itself() {
        let dead = FooterActions {
            reload: true,
            ..FooterActions::default()
        };
        let tip = |cfg: ConfigBarInputs| bar_state(ShellTab::Rules, dead, cfg, true).apply.tip;
        let rules = ConfigBarInputs {
            rules_blocking: true,
            ..ConfigBarInputs::default()
        };
        assert_eq!(tip(rules), Some(apply::APPLY_BLOCKED_TOOLTIP));
        let tiers = ConfigBarInputs {
            tier_errors: true,
            config_invalid: true,
            rules_blocking: true,
            ..ConfigBarInputs::default()
        };
        assert_eq!(tip(tiers), Some(config_state::APPLY_BLOCKED_TOOLTIP));
        let damaged = ConfigBarInputs {
            damaged: true,
            ..ConfigBarInputs::default()
        };
        assert_eq!(tip(damaged), Some(apply::DAMAGED_MARKERS_MESSAGE));
        assert_eq!(tip(ConfigBarInputs::default()), Some(OVERWRITE_NOTHING_TIP));
        // A live Overwrite keeps its own tooltip.
        let live = FooterActions {
            overwrite: true,
            ..dead
        };
        assert_eq!(
            bar_state(ShellTab::Rules, live, rules, true).apply.tip,
            Some(apply::OVERWRITE_TOOLTIP)
        );

        let files = vec!["dynamic.filter".to_string()];
        let s = popup_spec(PopupData::ChangedOnDisk {
            files: &files,
            overwrite: false,
            block: OverwriteBlock::RulesBlocking,
        });
        assert!(
            s.body
                .ends_with("Fix or disable the flagged rules to overwrite instead."),
            "{}",
            s.body
        );
        assert_eq!(labels(&s), ["Reload", "Keep editing"]);
    }

    #[test]
    fn popup_specs_match_the_contract() {
        let files = vec!["dynamic.filter".to_string(), "styles.json".to_string()];
        let s = popup_spec(PopupData::ChangedOnDisk {
            files: &files,
            overwrite: true,
            block: OverwriteBlock::Nothing,
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

        let s = popup_spec(PopupData::DiscardConfig {
            count: 2,
            economy: false,
            staged: false,
        });
        assert_eq!(s.title, "Discard config changes?");
        assert_eq!(s.body, super::super::config_view::revert_prompt(2));
        assert_eq!(labels(&s), ["Discard", "Keep editing"]);

        let s = popup_spec(PopupData::DiscardConfig {
            count: 2,
            economy: true,
            staged: true,
        });
        assert_eq!(s.title, "Discard economy changes?");
        assert_eq!(
            s.body,
            "Discard 2 unapplied tier changes and the staged economy section change?"
        );
        assert_eq!(labels(&s), ["Discard", "Keep editing"]);
        let s = popup_spec(PopupData::DiscardConfig {
            count: 0,
            economy: true,
            staged: true,
        });
        assert_eq!(s.body, "Discard the staged economy section change?");
        let s = popup_spec(PopupData::DiscardConfig {
            count: 1,
            economy: true,
            staged: false,
        });
        assert_eq!(s.body, "Discard 1 unapplied tier change?");

        let s = popup_spec(PopupData::DiscardRules { filter: "a.filter" });
        assert_eq!(s.title, "Discard rule changes?");
        assert_eq!(
            s.body,
            "Unapplied rule changes to a.filter will be discarded."
        );
        assert_eq!(labels(&s), ["Discard", "Keep editing"]);
        assert!(s.buttons[0].1, "Discard is the destructive button");
        let s = popup_spec(PopupData::DiscardRules { filter: "" });
        assert_eq!(s.body, "Unapplied rule changes will be discarded.");
    }

    #[test]
    fn the_last_button_is_the_safe_one_and_never_danger() {
        let files = vec!["x".to_string()];
        for data in [
            PopupData::ChangedOnDisk {
                files: &files,
                overwrite: true,
                block: OverwriteBlock::Nothing,
            },
            PopupData::ChangedOnDisk {
                files: &files,
                overwrite: false,
                block: OverwriteBlock::Nothing,
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
            PopupData::DiscardConfig {
                count: 1,
                economy: false,
                staged: false,
            },
            PopupData::DiscardConfig {
                count: 0,
                economy: true,
                staged: true,
            },
            PopupData::DiscardRules { filter: "f" },
        ] {
            let s = popup_spec(data);
            let last = s.buttons.last().unwrap();
            assert!(!last.1, "{}: safe button is not danger", s.title);
            assert!(s.buttons.len() >= 2);
        }
    }
}
