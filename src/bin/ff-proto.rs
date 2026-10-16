//! `ff-proto` -- a look-only prototype of FerrisFilter in Path of Exile 2's own UI style.
//!
//! Shows the shared skin (`ferrisfilter::skin`) on fake data so the author can judge the look
//! before the UI-redo milestone: a 660-point window docked to the left edge of the primary
//! monitor's work area at full height, the game's Options header / tabs / frame / controls, and
//! a static Ferris drawing behind the scrolling content. Nothing is read or written; the
//! buttons only set the status line, and style edits live in memory.
//!
//! ```text
//! cargo run --bin ff-proto
//! cargo run --bin ff-proto -- --tab config --expand-style kite --screenshot shot.png
//! ```
//!
//! - `--screenshot <png>` renders a few frames, saves one (physical pixels) and exits.
//! - `--tab rules|config|economy|about` picks the starting tab.
//! - `--expand-style <name>` opens that style's editor (CONFIG tab) and scrolls to it.
//! - `--status ok|warning|error` starts with the long status message REVERT / APPLY / RELOAD
//!   would set (to check the one-line truncation in a screenshot).
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::path::PathBuf;

use egui::{Align, Color32, Layout, Rect, Sense, UiBuilder, pos2, vec2};
use ferrisfilter::skin::editor::{self as ed, LabelLook};
use ferrisfilter::skin::minimap::{self, COLOURS, SHAPES, SWATCHES};
use ferrisfilter::skin::widgets::StatusKind;
use ferrisfilter::skin::{self, Skin, fonts, palette as pal, widgets as w};

/// The window's fixed width, in logical points.
const WIDTH: f32 = 660.0;
const TABS: [&str; 4] = ["Rules", "Config", "Economy", "About"];
/// The repository, shown on the ABOUT tab.
const REPO_URL: &str = "https://github.com/Jonathas-Conceicao/poe2-myfilter";

/// The long messages the bottom-bar buttons set, to exercise the one-line status truncation.
const MSG_OK: &str = "Reverted to the last applied filter: 3 rule edits and 1 style edit were \
                      discarded (prototype: nothing was read).";
const MSG_WARNING: &str = "Applied with warnings: 2 rules use the unknown style \
                           \"leaguestart-gold\" and were left as written, and the in-game reload \
                           was skipped because Path of Exile 2 was not the foreground window -- \
                           use Options > Game > Item Filter > Refresh (prototype: nothing was \
                           written).";
const MSG_ERROR: &str = "Could not reload dynamic.filter: the economy section's end marker is \
                         missing (expected after line 1412), so nothing was read. Restore the \
                         marker or remove the section, then press RELOAD again (prototype).";

struct Args {
    screenshot: Option<PathBuf>,
    tab: usize,
    expand_style: Option<String>,
    status: Option<StatusKind>,
}

fn parse_args() -> Args {
    let mut args = Args {
        screenshot: None,
        tab: 0,
        expand_style: None,
        status: None,
    };
    let mut it = std::env::args().skip(1);
    while let Some(a) = it.next() {
        match a.as_str() {
            "--screenshot" => args.screenshot = it.next().map(PathBuf::from),
            "--tab" => {
                args.tab = match it.next().as_deref() {
                    Some("config") => 1,
                    Some("economy") => 2,
                    Some("about") => 3,
                    _ => 0,
                }
            }
            "--expand-style" => args.expand_style = it.next(),
            "--status" => {
                args.status = match it.next().as_deref() {
                    Some("ok") => Some(StatusKind::Ok),
                    Some("warning") => Some(StatusKind::Warning),
                    Some("error") => Some(StatusKind::Error),
                    _ => None,
                }
            }
            other => eprintln!("ff-proto: ignoring unknown argument {other:?}"),
        }
    }
    args
}

fn main() -> eframe::Result<()> {
    let args = parse_args();
    let viewport = egui::ViewportBuilder::default()
        .with_title("FerrisFilter prototype")
        .with_decorations(false)
        .with_resizable(false)
        .with_inner_size([WIDTH, 900.0])
        // Shown on the first frame, once it has been docked, so it never flashes elsewhere.
        .with_visible(false);
    let options = eframe::NativeOptions {
        viewport,
        ..Default::default()
    };
    eframe::run_native(
        "ff-proto",
        options,
        Box::new(move |cc| {
            let skin = skin::install(&cc.egui_ctx);
            Ok(Box::new(Proto::new(skin, args)))
        }),
    )
}

/// One fake rule row.
struct Rule {
    name: &'static str,
    action: &'static str,
    summary: String,
    enabled: bool,
    cont: bool,
}

/// One style, as `styles.json` holds it (fake: loaded from `styles-default.json` at compile
/// time, edited in memory only).
#[derive(Clone)]
struct Style {
    name: String,
    builtin: bool,
    show: bool,
    /// Text, border, background; `None` = keep the game's.
    colors: [Option<Color32>; 3],
    /// What a colour card goes back to when switched from "none" to a colour.
    remembered: [Color32; 3],
    font: Option<f64>,
    /// `MinimapIcon` size (0 = Large); `None` = no icon. Colour and shape are kept while off.
    mm_size: Option<usize>,
    mm_colour: usize,
    mm_shape: usize,
    /// `PlayEffect` colour; `None` = no beam.
    beam: Option<usize>,
    beam_temp: bool,
    /// Index into [`Proto::sound_options`]: 0 none, 1-16 alert sounds, 17 the custom file.
    sound: usize,
    volume: f64,
}

impl Style {
    fn look(&self) -> LabelLook {
        LabelLook {
            text: self.colors[0],
            border: self.colors[1],
            background: self.colors[2],
            font_size: self.font.map(|f| f as f32),
            hidden: !self.show,
        }
    }
}

/// A message in the bottom bar.
struct Status {
    text: String,
    kind: StatusKind,
}

impl Status {
    fn new(kind: StatusKind, text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            kind,
        }
    }
}

struct Proto {
    skin: Skin,
    args: Args,
    tab: usize,
    docked: bool,
    frames: u32,
    shot_requested: bool,
    status: Status,

    search: String,
    rules: Vec<Rule>,
    expanded: Option<usize>,
    op: usize,
    item_level: f64,
    rarity: usize,
    corrupted: bool,
    identified: bool,
    style: usize,

    filter: usize,
    reload_after_apply: bool,
    styles: Vec<Style>,
    style_open: Option<usize>,
    confirm_delete: bool,
    added: usize,
    sound_options: Vec<String>,
    tiers: Vec<(&'static str, f64)>,
}

impl Proto {
    fn new(skin: Skin, args: Args) -> Self {
        let tab = args.tab;
        let styles = fake_styles();
        let style_open = args
            .expand_style
            .as_ref()
            .and_then(|n| styles.iter().position(|s| s.name.eq_ignore_ascii_case(n)));
        let status = match args.status {
            Some(StatusKind::Ok) => Status::new(StatusKind::Ok, MSG_OK),
            Some(StatusKind::Warning) => Status::new(StatusKind::Warning, MSG_WARNING),
            Some(StatusKind::Error) => Status::new(StatusKind::Error, MSG_ERROR),
            _ => Status::new(StatusKind::Info, "Prototype: fake data, nothing is saved."),
        };
        let mut sound_options = vec!["None".to_owned()];
        sound_options.extend((1..=16).map(|i| format!("Alert sound {i}")));
        sound_options.push("Custom: uwu.mp3".to_owned());
        Self {
            skin,
            tab,
            args,
            docked: false,
            frames: 0,
            shot_requested: false,
            status,
            search: String::new(),
            rules: fake_rules(),
            expanded: Some(2),
            op: 2,
            item_level: 82.0,
            rarity: 2,
            corrupted: false,
            identified: true,
            style: 3,
            filter: 0,
            reload_after_apply: true,
            styles,
            style_open,
            confirm_delete: false,
            added: 0,
            sound_options,
            tiers: vec![
                ("S", 50.0),
                ("A", 10.0),
                ("B", 3.0),
                ("C", 1.0),
                ("D", 0.5),
                ("E", 0.2),
                ("F", 0.05),
            ],
        }
    }

    /// Docks the window to the left edge, full work-area height, converting the work area from
    /// physical pixels with the window's own scale factor; then shows it.
    fn dock(&mut self, ctx: &egui::Context) {
        let (ppp, monitor) = ctx.input(|i| {
            let v = i.viewport();
            (v.native_pixels_per_point, v.monitor_size)
        });
        let Some(ppp) = ppp else {
            return; // not known yet; retried next frame
        };
        let rect = match skin::window::primary_work_area_px() {
            Some(work) => skin::window::left_dock_rect(work, ppp, WIDTH),
            None => match monitor {
                Some(m) => Rect::from_min_size(pos2(0.0, 0.0), vec2(WIDTH, m.y)),
                None => return,
            },
        };
        ctx.send_viewport_cmd(egui::ViewportCommand::OuterPosition(rect.min));
        ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(rect.size()));
        ctx.send_viewport_cmd(egui::ViewportCommand::Visible(true));
        self.docked = true;
    }

    fn screenshot(&mut self, ui: &mut egui::Ui) {
        let Some(path) = self.args.screenshot.clone() else {
            return;
        };
        ui.ctx().request_repaint();
        let image = ui.ctx().input(|i| {
            i.raw.events.iter().find_map(|e| match e {
                egui::Event::Screenshot { image, .. } => Some(image.clone()),
                _ => None,
            })
        });
        if let Some(image) = image {
            let [w, h] = image.size;
            let mut buf = Vec::with_capacity(w * h * 4);
            for c in &image.pixels {
                buf.extend_from_slice(&c.to_srgba_unmultiplied());
            }
            match image::RgbaImage::from_raw(w as u32, h as u32, buf).map(|img| img.save(&path)) {
                Some(Ok(())) => eprintln!("ff-proto: saved {}", path.display()),
                Some(Err(e)) => eprintln!("ff-proto: could not save {}: {e}", path.display()),
                None => eprintln!("ff-proto: screenshot had an unexpected size"),
            }
            ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
        } else if self.docked && self.frames >= 20 && !self.shot_requested {
            ui.ctx()
                .send_viewport_cmd(egui::ViewportCommand::Screenshot(Default::default()));
            self.shot_requested = true;
        } else if self.frames > 600 {
            eprintln!("ff-proto: no screenshot arrived; exiting");
            ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
        }
    }
}

impl eframe::App for Proto {
    fn logic(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if !self.docked {
            self.dock(ctx);
        }
    }

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.frames += 1;
        if !self.docked {
            self.dock(ui.ctx());
        }
        self.screenshot(ui);

        let win = ui.max_rect();
        let skin = self.skin.clone();
        let painter = ui.painter().clone();

        // 1. Static background: never moves with the content.
        w::ff_background(&painter, &skin, win);

        // 2. Frame below the header; the header overlaps the frame's top strip.
        let header_h = w::header_height(&skin, win.width());
        let frame_outer = Rect::from_min_max(pos2(win.min.x, win.min.y + header_h - 14.0), win.max);
        let frame = w::ff_frame(&painter, &skin, frame_outer);

        // 3. Header art, title and close button.
        let header = Rect::from_min_size(win.min, vec2(win.width(), header_h));
        if w::ff_header(ui, &skin, header, Some(&skin.title_bar), "FerrisFilter").clicked() {
            ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
        }

        // 4. Tabs, sitting on the frame's top edge.
        let tab_h = 26.0;
        let tabs = Rect::from_min_max(
            pos2(frame.content.min.x, frame.outer.min.y + 13.0),
            pos2(frame.content.max.x, frame.outer.min.y + 13.0 + tab_h),
        );
        w::ff_tabs(ui, &skin, tabs, &TABS, &mut self.tab);

        // 5. Content, clipped to the area inside the gold line.
        let content = frame.content.shrink2(vec2(10.0, 8.0));
        let mut cui = ui.new_child(
            UiBuilder::new()
                .max_rect(content)
                .layout(Layout::top_down(Align::Min)),
        );
        cui.set_clip_rect(frame.content);
        match self.tab {
            0 => self.rules_tab(&mut cui, &skin),
            1 => self.config_tab(&mut cui, &skin),
            2 => self.economy_tab(&mut cui, &skin),
            _ => self.about_tab(&mut cui, &skin),
        }

        // 6. Bottom bar: status on the left, REVERT / RELOAD / APPLY on the right.
        self.bottom_bar(ui, &skin, frame.button_bar);
    }
}

impl Proto {
    fn bottom_bar(&mut self, ui: &mut egui::Ui, skin: &Skin, bar: Rect) {
        let inner = bar.shrink2(vec2(10.0, 0.0));
        let mut bui = ui.new_child(
            UiBuilder::new()
                .max_rect(inner)
                .layout(Layout::right_to_left(Align::Center)),
        );
        bui.spacing_mut().item_spacing.x = 6.0;
        if w::ff_button(&mut bui, skin, "Apply", 84.0).clicked() {
            self.status = Status::new(StatusKind::Warning, MSG_WARNING);
        }
        if w::ff_button(&mut bui, skin, "Reload", 84.0).clicked() {
            self.status = Status::new(StatusKind::Error, MSG_ERROR);
        }
        if w::ff_button(&mut bui, skin, "Revert", 84.0).clicked() {
            self.status = Status::new(StatusKind::Ok, MSG_OK);
        }
        let status_rect =
            Rect::from_min_max(inner.min, pos2(bui.min_rect().min.x - 10.0, inner.max.y));
        w::ff_status(ui, status_rect, &self.status.text, self.status.kind);
    }

    fn info(&mut self, text: impl Into<String>) {
        self.status = Status::new(StatusKind::Info, text);
    }

    fn rules_tab(&mut self, ui: &mut egui::Ui, skin: &Skin) {
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 8.0;
            let field_w = ui.available_width() - 132.0;
            w::ff_text_field(
                ui,
                &mut self.search,
                "Search rules, classes, base types...",
                field_w,
            );
            if w::ff_button(ui, skin, "+ Add rule", 124.0).clicked() {
                self.info("+ ADD RULE pressed (prototype).");
            }
        });
        ui.add_space(6.0);
        let needle = self.search.to_lowercase();
        w::ff_scroll_area(ui, skin, "rules", |ui| {
            let sections: [(&str, std::ops::Range<usize>); 3] = [
                ("Leveling", 0..14),
                ("Endgame", 14..32),
                ("Cleanup", 32..40),
            ];
            for (title, range) in sections {
                w::ff_section(ui, skin, title);
                for i in range {
                    let matches = needle.is_empty()
                        || self.rules[i].name.to_lowercase().contains(&needle)
                        || self.rules[i].summary.to_lowercase().contains(&needle);
                    if matches {
                        self.rule_row(ui, skin, i);
                    }
                }
            }
            ui.add_space(8.0);
        });
    }

    fn rule_row(&mut self, ui: &mut egui::Ui, skin: &Skin, i: usize) {
        let height = 48.0;
        let (rect, resp) =
            ui.allocate_exact_size(vec2(ui.available_width(), height), Sense::click());
        w::row_background(ui, i, rect);
        let expanded = self.expanded == Some(i);
        if expanded {
            ui.painter().rect_stroke(
                rect,
                0.0,
                egui::Stroke::new(1.0, pal::INPUT_BORDER_HOT),
                egui::StrokeKind::Inside,
            );
        }

        let mut row = ui.new_child(
            UiBuilder::new()
                .max_rect(rect.shrink2(vec2(w::ROW_PAD, 0.0)))
                .layout(Layout::left_to_right(Align::Center)),
        );
        let rule = &mut self.rules[i];
        w::ff_check(&mut row, skin, &mut rule.enabled, "Enabled");
        row.add_space(10.0);
        let text_left = row.cursor().min.x;

        // Right side: Continue marker and the reorder arrows.
        let mut right = row.new_child(
            UiBuilder::new()
                .max_rect(rect.shrink2(vec2(w::ROW_PAD, 0.0)))
                .layout(Layout::right_to_left(Align::Center)),
        );
        right.spacing_mut().item_spacing.x = 4.0;
        if w::ff_arrow(&mut right, skin, false, i + 1 < 40).clicked() {
            self.info(format!("Moved rule {} down (prototype).", i + 1));
        }
        if w::ff_arrow(&mut right, skin, true, i > 0).clicked() {
            self.info(format!("Moved rule {} up (prototype).", i + 1));
        }
        let rule = &self.rules[i];
        if rule.cont {
            right.add_space(6.0);
            right.label(
                egui::RichText::new("Continue")
                    .font(fonts::small_caps(14.0))
                    .color(pal::GOLD),
            );
        }
        let text_right = right.min_rect().min.x - 8.0;

        let dim = if rule.enabled { 1.0 } else { 0.55 };
        let fade = |c: Color32| c.gamma_multiply(dim);
        let p = ui.painter().with_clip_rect(Rect::from_min_max(
            pos2(text_left, rect.min.y),
            pos2(text_right, rect.max.y),
        ));
        let action_color = if rule.action == "Hide" {
            pal::DISABLED
        } else {
            pal::HEADING
        };
        let l1 = rect.min.y + 15.0;
        let idx = p.layout_no_wrap(format!("{:>2}", i + 1), fonts::regular(15.0), pal::MUTED);
        let idx_w = idx.size().x;
        p.galley(
            pos2(text_left, l1 - idx.size().y / 2.0),
            idx,
            fade(pal::MUTED),
        );
        let act = p.layout_no_wrap(rule.action.into(), fonts::small_caps(16.0), action_color);
        let act_w = act.size().x;
        let x = text_left + idx_w + 10.0;
        p.galley(
            pos2(x, l1 - act.size().y / 2.0 - 1.0),
            act,
            fade(action_color),
        );
        p.text(
            pos2(x + act_w + 10.0, l1),
            egui::Align2::LEFT_CENTER,
            rule.name,
            fonts::small_caps(16.0),
            fade(pal::LABEL),
        );
        p.text(
            pos2(text_left, rect.min.y + 34.0),
            egui::Align2::LEFT_CENTER,
            &rule.summary,
            fonts::regular(14.0),
            fade(pal::MUTED),
        );

        if resp.clicked() {
            self.expanded = if expanded { None } else { Some(i) };
        }
        if expanded {
            self.rule_body(ui, skin);
        }
    }

    fn rule_body(&mut self, ui: &mut egui::Ui, skin: &Skin) {
        let mut body = ui.new_child(
            UiBuilder::new()
                .max_rect(ui.available_rect_before_wrap().shrink2(vec2(18.0, 0.0)))
                .layout(Layout::top_down(Align::Min)),
        );
        body.add_space(4.0);
        w::ff_row(&mut body, 1, "Item Level", |ui| {
            ui.spacing_mut().item_spacing.x = 8.0;
            if w::ff_dropdown(
                ui,
                skin,
                "op",
                &mut self.op,
                &["=", "<", ">=", "<=", ">"],
                58.0,
            )
            .changed()
            {
                self.status = Status::new(StatusKind::Info, "Operator changed (prototype).");
            }
            w::ff_slider(
                ui,
                skin,
                &mut self.item_level,
                1.0..=100.0,
                1.0,
                "Item Level",
                |v| format!("{v:.0}"),
            );
        });
        w::ff_row(&mut body, 0, "Rarity", |ui| {
            w::ff_dropdown(
                ui,
                skin,
                "rarity",
                &mut self.rarity,
                &["Normal", "Magic", "Rare", "Unique"],
                0.0,
            );
        });
        w::ff_row(&mut body, 1, "Corrupted", |ui| {
            w::ff_check(ui, skin, &mut self.corrupted, "Corrupted");
        });
        w::ff_row(&mut body, 0, "Identified", |ui| {
            w::ff_check(ui, skin, &mut self.identified, "Identified");
        });
        let names: Vec<&str> = std::iter::once("Custom")
            .chain(self.styles.iter().map(|s| s.name.as_str()))
            .collect();
        w::ff_row(&mut body, 1, "Style", |ui| {
            w::ff_dropdown(ui, skin, "style", &mut self.style, &names, 0.0);
        });
        body.add_space(6.0);
        let used = body.min_rect();
        ui.allocate_rect(
            Rect::from_min_max(ui.cursor().min, pos2(ui.max_rect().max.x, used.max.y)),
            Sense::hover(),
        );
    }

    fn config_tab(&mut self, ui: &mut egui::Ui, skin: &Skin) {
        w::ff_scroll_area(ui, skin, "config", |ui| {
            w::ff_section(ui, skin, "Filter");
            w::ff_row(ui, 0, "Filter", |ui| {
                w::ff_dropdown(
                    ui,
                    skin,
                    "filter",
                    &mut self.filter,
                    &[
                        "dynamic.filter",
                        "leaguestart.filter",
                        "ssf_icewolf.filter",
                        "New filter...",
                    ],
                    0.0,
                );
            });
            w::ff_row(ui, 1, "Reload filter in game after Apply", |ui| {
                w::ff_check(
                    ui,
                    skin,
                    &mut self.reload_after_apply,
                    "Reload filter in game after Apply",
                );
            });

            w::ff_section(ui, skin, "Styles");
            self.style_group_heading(ui, skin, "Built-in", false);
            for i in 0..self.styles.len() {
                if self.styles[i].builtin {
                    self.style_row(ui, skin, i);
                }
            }
            ui.add_space(6.0);
            self.style_group_heading(ui, skin, "Custom", true);
            for i in 0..self.styles.len() {
                if i < self.styles.len() && !self.styles[i].builtin {
                    self.style_row(ui, skin, i);
                }
            }

            w::ff_section(ui, skin, "Tiers");
            for (i, (name, value)) in self.tiers.iter_mut().enumerate() {
                w::ff_row(ui, i, &format!("Tier {name}"), |ui| {
                    w::ff_slider(ui, skin, value, 0.0..=100.0, 0.05, name, |v| {
                        format!("{v:.2} ex")
                    });
                });
            }
            ui.add_space(8.0);
        });
    }

    /// "Built-in" / "Custom" above a style group; the Custom one carries "+ Add style".
    fn style_group_heading(&mut self, ui: &mut egui::Ui, skin: &Skin, title: &str, add: bool) {
        ui.add_space(4.0);
        ui.horizontal(|ui| {
            ui.set_min_height(34.0);
            ui.add_space(6.0);
            ui.label(
                egui::RichText::new(title)
                    .font(fonts::small_caps(16.0))
                    .color(pal::GOLD),
            );
            if add {
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if w::ff_button(ui, skin, "+ Add style", 130.0).clicked() {
                        self.add_style();
                    }
                });
            }
        });
        ui.add_space(2.0);
    }

    fn add_style(&mut self) {
        self.added += 1;
        let name = format!("new style {}", self.added);
        let white = Color32::from_rgb(248, 248, 242);
        let dracula = Color32::from_rgb(40, 42, 54);
        self.styles.push(Style {
            name: name.clone(),
            builtin: false,
            show: true,
            colors: [Some(white), Some(white), Some(dracula)],
            remembered: [white, white, dracula],
            font: None,
            mm_size: None,
            mm_colour: 4,
            mm_shape: 0,
            beam: None,
            beam_temp: false,
            sound: 0,
            volume: 100.0,
        });
        self.style_open = Some(self.styles.len() - 1);
        self.confirm_delete = false;
        self.info(format!(
            "Added style \"{name}\" (prototype: in memory only)."
        ));
    }

    /// One style: a collapsed row (show switch, name, minimap icon, label chip, and for custom
    /// styles the reorder arrows); clicking it opens the editor below it.
    fn style_row(&mut self, ui: &mut egui::Ui, skin: &Skin, i: usize) {
        let (rect, resp) = ui.allocate_exact_size(vec2(ui.available_width(), 40.0), Sense::click());
        w::row_background(ui, i, rect);
        let open = self.style_open == Some(i);
        if open {
            ui.painter().rect_stroke(
                rect,
                0.0,
                egui::Stroke::new(1.0, pal::INPUT_BORDER_HOT),
                egui::StrokeKind::Inside,
            );
            if self.frames < 12 && self.args.expand_style.is_some() {
                ui.scroll_to_rect(rect, Some(Align::Min));
            }
        }
        let mut row = ui.new_child(
            UiBuilder::new()
                .max_rect(rect.shrink2(vec2(w::ROW_PAD, 0.0)))
                .layout(Layout::left_to_right(Align::Center)),
        );
        let s = &mut self.styles[i];
        let label = format!("Show {}", s.name);
        w::ff_check(&mut row, skin, &mut s.show, &label);
        row.add_space(12.0);
        let name_left = row.cursor().min.x;

        let mut right = row.new_child(
            UiBuilder::new()
                .max_rect(rect.shrink2(vec2(w::ROW_PAD, 0.0)))
                .layout(Layout::right_to_left(Align::Center)),
        );
        right.spacing_mut().item_spacing.x = 4.0;
        let custom_range = self.first_custom()..self.styles.len();
        if !self.styles[i].builtin {
            let down = i + 1 < custom_range.end;
            let up = i > custom_range.start;
            if w::ff_arrow(&mut right, skin, false, down).clicked() {
                self.move_style(i, i + 1);
            }
            if w::ff_arrow(&mut right, skin, true, up).clicked() {
                self.move_style(i, i - 1);
            }
            right.add_space(8.0);
        }
        let Some(s) = self.styles.get(i) else {
            return;
        };
        // The label chip, then the minimap icon left of it.
        let (chip, _) = right.allocate_exact_size(vec2(130.0, 30.0), Sense::hover());
        let mut look = s.look();
        look.font_size = Some(26.0);
        ed::ff_item_label(ui.painter(), chip, "Item", look);
        right.add_space(6.0);
        let (icon, _) = right.allocate_exact_size(vec2(28.0, 28.0), Sense::hover());
        if let Some(size) = s.mm_size {
            ed::paint_minimap_icon(
                ui.painter(),
                skin,
                icon.center(),
                s.mm_shape,
                s.mm_colour,
                minimap::icon_points(size, 28.0),
                Color32::WHITE,
            );
        }
        let name_right = right.min_rect().min.x - 8.0;
        let color = if s.show { pal::LABEL } else { pal::DISABLED };
        ui.painter()
            .with_clip_rect(Rect::from_min_max(
                pos2(name_left, rect.min.y),
                pos2(name_right, rect.max.y),
            ))
            .text(
                pos2(name_left, rect.center().y),
                egui::Align2::LEFT_CENTER,
                &s.name,
                fonts::small_caps(17.0),
                color,
            );
        if !s.show {
            ui.painter().text(
                pos2(name_right, rect.center().y),
                egui::Align2::RIGHT_CENTER,
                "Hide",
                fonts::small_caps(14.0),
                pal::DISABLED,
            );
        }

        if resp.clicked() {
            self.style_open = if open { None } else { Some(i) };
            self.confirm_delete = false;
        }
        if self.style_open == Some(i) && open {
            self.style_editor(ui, skin, i);
        }
    }

    fn first_custom(&self) -> usize {
        self.styles
            .iter()
            .position(|s| !s.builtin)
            .unwrap_or(self.styles.len())
    }

    fn move_style(&mut self, from: usize, to: usize) {
        self.styles.swap(from, to);
        self.style_open = match self.style_open {
            Some(o) if o == from => Some(to),
            Some(o) if o == to => Some(from),
            other => other,
        };
        self.info(format!(
            "Moved style \"{}\" (prototype: in memory only).",
            self.styles[to].name
        ));
    }

    /// The style editor, laid out like Scalpel's tier editor in the game's look.
    fn style_editor(&mut self, ui: &mut egui::Ui, skin: &Skin, i: usize) {
        let mut deleted = false;
        egui::Frame::NONE
            .fill(Color32::from_black_alpha(80))
            .inner_margin(egui::Margin {
                left: 12,
                right: 10,
                top: 8,
                bottom: 10,
            })
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.spacing_mut().item_spacing.y = 8.0;
                if !self.styles[i].builtin {
                    deleted = self.custom_header(ui, skin, i);
                    if deleted {
                        return;
                    }
                }
                let sound_options: Vec<&str> =
                    self.sound_options.iter().map(String::as_str).collect();
                let s = &mut self.styles[i];

                ed::ff_card(ui, "Visibility", |ui| {
                    let mut sel = usize::from(!s.show);
                    if ed::ff_segmented(ui, skin, &mut sel, &["Show", "Hide"], 0.0).changed() {
                        s.show = sel == 0;
                    }
                });

                ed::ff_card(ui, "Label", |ui| {
                    let (strip, _) =
                        ui.allocate_exact_size(vec2(ui.available_width(), 64.0), Sense::hover());
                    ed::ff_item_label(ui.painter(), strip, "Razor Quarterstaff", s.look());
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = 6.0;
                        let card_w = (ui.available_width() - 12.0) / 3.0;
                        for (k, name) in ["Text", "Border", "Background"].iter().enumerate() {
                            if ed::ff_colour_card(ui, name, s.colors[k], card_w).clicked() {
                                s.colors[k] = match s.colors[k] {
                                    Some(c) => {
                                        s.remembered[k] = c;
                                        None
                                    }
                                    None => Some(s.remembered[k]),
                                };
                            }
                        }
                    });
                    ui.horizontal(|ui| {
                        ui.set_min_height(28.0);
                        ui.spacing_mut().item_spacing.x = 8.0;
                        let mut custom = s.font.is_some();
                        if w::ff_check(ui, skin, &mut custom, "Font size").changed() {
                            s.font = custom.then_some(32.0);
                        }
                        ed::caption(ui, "Font size");
                        match &mut s.font {
                            Some(f) => {
                                w::ff_slider(ui, skin, f, 18.0..=45.0, 1.0, "Font size", |v| {
                                    format!("{v:.0}")
                                });
                            }
                            None => {
                                ui.label(
                                    egui::RichText::new("game default")
                                        .font(fonts::regular(15.0))
                                        .color(pal::DISABLED),
                                );
                            }
                        }
                    });
                });

                ed::ff_card(ui, "Minimap icon", |ui| {
                    ui.horizontal(|ui| {
                        let (icon, _) = ui.allocate_exact_size(vec2(44.0, 40.0), Sense::hover());
                        match s.mm_size {
                            Some(size) => ed::paint_minimap_icon(
                                ui.painter(),
                                skin,
                                icon.center(),
                                s.mm_shape,
                                s.mm_colour,
                                minimap::icon_points(size, 40.0),
                                Color32::WHITE,
                            ),
                            None => {
                                ui.painter().text(
                                    icon.center(),
                                    egui::Align2::CENTER_CENTER,
                                    "--",
                                    fonts::regular(15.0),
                                    pal::DISABLED,
                                );
                            }
                        }
                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            let mut sel = s.mm_size.map_or(0, |v| v + 1);
                            if ed::ff_segmented(
                                ui,
                                skin,
                                &mut sel,
                                &["None", "Large", "Medium", "Small"],
                                300.0,
                            )
                            .changed()
                            {
                                s.mm_size = sel.checked_sub(1);
                            }
                        });
                    });
                    let on = s.mm_size.is_some();
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = 4.0;
                        let n = COLOURS.len() as f32;
                        let sw = ((ui.available_width() - 4.0 * (n - 1.0)) / n).floor();
                        for (c, name) in COLOURS.iter().enumerate() {
                            let sel = on && s.mm_colour == c;
                            if ed::ff_swatch(ui, Some(SWATCHES[c]), sel, vec2(sw, 24.0), name)
                                .on_hover_text(*name)
                                .clicked()
                            {
                                s.mm_colour = c;
                                s.mm_size.get_or_insert(1);
                            }
                        }
                    });
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = 4.0;
                        let n = SHAPES.len() as f32;
                        let tile = ((ui.available_width() - 4.0 * (n - 1.0)) / n).floor();
                        for shape in 0..SHAPES.len() {
                            let sel = on && s.mm_shape == shape;
                            if ed::ff_icon_button(ui, skin, shape, s.mm_colour, sel, tile).clicked()
                            {
                                s.mm_shape = shape;
                                s.mm_size.get_or_insert(1);
                            }
                        }
                    });
                });

                ed::ff_card(ui, "Beam", |ui| {
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = 4.0;
                        let temp_w = 74.0;
                        let n = COLOURS.len() as f32 + 1.0;
                        let sw = ((ui.available_width() - temp_w - 4.0 * n) / n).floor();
                        if ed::ff_swatch(ui, None, s.beam.is_none(), vec2(sw, 24.0), "No beam")
                            .on_hover_text("None")
                            .clicked()
                        {
                            s.beam = None;
                        }
                        for (c, name) in COLOURS.iter().enumerate() {
                            if ed::ff_swatch(
                                ui,
                                Some(SWATCHES[c]),
                                s.beam == Some(c),
                                vec2(sw, 24.0),
                                name,
                            )
                            .on_hover_text(*name)
                            .clicked()
                            {
                                s.beam = Some(c);
                            }
                        }
                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            ed::caption(ui, "Temp");
                            w::ff_check(ui, skin, &mut s.beam_temp, "Temporary beam");
                        });
                    });
                });

                ed::ff_card(ui, "Sound", |ui| {
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = 10.0;
                        w::ff_dropdown(ui, skin, ("sound", i), &mut s.sound, &sound_options, 190.0);
                        ed::caption(ui, "Volume");
                        w::ff_slider(ui, skin, &mut s.volume, 0.0..=300.0, 5.0, "Volume", |v| {
                            format!("{v:.0}")
                        });
                    });
                });
            });
        if deleted {
            self.style_open = None;
        }
    }

    /// A custom style's name field and Delete (with an inline confirm). Returns true when the
    /// style was deleted.
    fn custom_header(&mut self, ui: &mut egui::Ui, skin: &Skin, i: usize) -> bool {
        let mut delete = false;
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 8.0;
            ed::caption(ui, "Name");
            let field_w = ui.available_width() - if self.confirm_delete { 250.0 } else { 120.0 };
            w::ff_text_field(ui, &mut self.styles[i].name, "Style name", field_w);
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if self.confirm_delete {
                    if w::ff_button(ui, skin, "Cancel", 90.0).clicked() {
                        self.confirm_delete = false;
                    }
                    if w::ff_button(ui, skin, "Delete", 90.0).clicked() {
                        delete = true;
                    }
                    ed::caption(ui, "Sure?");
                } else if w::ff_button(ui, skin, "Delete", 100.0).clicked() {
                    self.confirm_delete = true;
                }
            });
        });
        if delete {
            let name = self.styles.remove(i).name;
            self.confirm_delete = false;
            self.info(format!(
                "Deleted style \"{name}\" (prototype: in memory only)."
            ));
        }
        delete
    }

    fn economy_tab(&mut self, ui: &mut egui::Ui, skin: &Skin) {
        w::ff_scroll_area(ui, skin, "economy", |ui| {
            w::ff_section(ui, skin, "Economy");
            let rows = [
                ("League", "Forbidden Rites"),
                ("Last sync", "2026-10-15 21:04"),
                ("Priced items", "1,284"),
            ];
            for (i, (label, value)) in rows.iter().enumerate() {
                w::ff_row(ui, i, label, |ui| {
                    ui.label(
                        egui::RichText::new(*value)
                            .font(fonts::regular(16.0))
                            .color(pal::VALUE),
                    );
                });
            }
            ui.add_space(14.0);
            ui.with_layout(Layout::top_down(Align::Center), |ui| {
                if w::ff_button(ui, skin, "Sync economy", 180.0).clicked() {
                    self.info("SYNC ECONOMY pressed (prototype: no network).");
                }
            });
        });
    }

    fn about_tab(&mut self, ui: &mut egui::Ui, skin: &Skin) {
        let area = ui.available_rect_before_wrap();
        ui.add_space(26.0);
        ui.vertical_centered(|ui| {
            ui.spacing_mut().item_spacing.y = 6.0;
            ui.label(
                egui::RichText::new("FerrisFilter")
                    .font(fonts::small_caps(36.0))
                    .color(pal::HEADING),
            );
            ui.label(
                egui::RichText::new(format!("Version {}", env!("CARGO_PKG_VERSION")))
                    .font(fonts::regular(15.0))
                    .color(pal::MUTED),
            );
            ui.add_space(4.0);
            ui.label(
                egui::RichText::new("A loot-filter editor and overlay for Path of Exile 2")
                    .font(fonts::regular(17.0))
                    .color(pal::LABEL),
            );
            ui.hyperlink_to(
                egui::RichText::new("github.com/Jonathas-Conceicao/poe2-myfilter")
                    .font(fonts::regular(16.0))
                    .color(pal::GOLD),
                REPO_URL,
            );
        });
        ui.add_space(10.0);
        w::ff_section(ui, skin, "Credits");
        let credits = [
            ("Game UI art and font files", "\u{a9} Grinding Gear Games"),
            ("Fontin typeface", "exljbris"),
            ("Ferris the crab", "Karen Rustad T\u{f6}lva, public domain"),
        ];
        for (i, (what, who)) in credits.iter().enumerate() {
            w::ff_row(ui, i, what, |ui| {
                ui.label(
                    egui::RichText::new(*who)
                        .font(fonts::regular(16.0))
                        .color(pal::VALUE),
                );
            });
        }

        // The disclaimer, pinned to the bottom of the page.
        let foot = Rect::from_min_max(pos2(area.min.x, area.max.y - 80.0), area.max);
        let painter = ui.painter();
        ferrisfilter::skin::slice::paint_image(
            painter,
            &skin.title_underline,
            Rect::from_center_size(
                pos2(foot.center().x, foot.min.y),
                vec2(foot.width() * 0.8, 3.0),
            ),
            Color32::from_white_alpha(110),
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
                "Game art and fonts belong to their owners. FerrisFilter is a free third-party \
                 tool for Path of Exile 2 players.",
                pal::MUTED,
            ),
        ] {
            fui.add(
                egui::Label::new(
                    egui::RichText::new(text)
                        .font(fonts::regular(13.5))
                        .color(color),
                )
                .halign(Align::Center)
                .wrap(),
            );
        }
    }
}

fn fake_rules() -> Vec<Rule> {
    let names = [
        (
            "Show",
            "Recycle: sockets + quality",
            "Sockets >= 1; Quality >= 1",
            false,
        ),
        (
            "Show",
            "Currency S",
            "Class: Stackable Currency; BaseType: 4 items",
            false,
        ),
        (
            "Show",
            "Rare weapons, early",
            "ItemLevel >= 82; Rarity Rare; Class: Staves, Wands, Foci",
            true,
        ),
        (
            "Show",
            "Flasks with quality",
            "Class: 4 items; Quality >= 1",
            false,
        ),
        (
            "Show",
            "Flasks, act 1-3",
            "Class: 4 items; AreaLevel <= 25",
            false,
        ),
        (
            "Show",
            "Magic flasks",
            "Class: 4 items; Rarity >= Magic; AreaLevel <= 65",
            false,
        ),
        (
            "Show",
            "Unidentified T5",
            "UnidentifiedItemTier >= 5; Rarity Rare",
            true,
        ),
        (
            "Show",
            "Armour, any rarity",
            "AreaLevel <= 5; Class: 6 items",
            false,
        ),
        (
            "Show",
            "ES bases",
            "BaseEnergyShield > 0; Class: 6 items",
            false,
        ),
        (
            "Show",
            "Evasion bases",
            "BaseEvasion > 0; BaseArmour 0",
            false,
        ),
        (
            "Hide",
            "Normal armour",
            "Rarity Normal; AreaLevel >= 14",
            false,
        ),
        (
            "Show",
            "Rings and amulets",
            "Class: Rings, Amulets, Belts; Rarity Rare",
            false,
        ),
        ("Show", "Waystones", "Class: Waystones", false),
        (
            "Hide",
            "Leveling clutter",
            "Rarity <= Magic; AreaLevel >= 30",
            false,
        ),
        (
            "Show",
            "Uniques S",
            "Rarity Unique; BaseType: 12 items",
            false,
        ),
        (
            "Show",
            "Uniques A",
            "Rarity Unique; BaseType: 31 items",
            false,
        ),
        (
            "Show",
            "Uniques B",
            "Rarity Unique; BaseType: 57 items",
            false,
        ),
        (
            "Show",
            "Exceptional bases",
            "Quality >= 21; Rarity <= Rare",
            true,
        ),
        (
            "Show",
            "Six-socket gear",
            "Sockets >= 3; Rarity Rare",
            false,
        ),
        (
            "Show",
            "Rare jewellery T5",
            "UnidentifiedItemTier >= 5; Class: 3 items",
            false,
        ),
        (
            "Show",
            "Rare body armours",
            "ItemLevel >= 81; Class: Body Armours",
            false,
        ),
        (
            "Show",
            "Corrupted rares",
            "Corrupted True; Rarity Rare",
            false,
        ),
        (
            "Show",
            "Enchanted gear",
            "AnyEnchantment True; Rarity Rare",
            true,
        ),
        ("Show", "Fractured items", "FracturedItem True", false),
        ("Show", "Charms", "Class: Charms; Rarity >= Magic", false),
        ("Show", "Relics", "Class: Relics", false),
        ("Show", "Tablets", "Class: Tablets; Rarity >= Magic", false),
        ("Show", "Omens", "BaseType: 14 items", false),
        (
            "Show",
            "Breach splinters",
            "BaseType: Breach Splinter; StackSize >= 10",
            false,
        ),
        (
            "Show",
            "Expedition logbooks",
            "BaseType: Expedition Logbook",
            false,
        ),
        ("Show", "Socketables", "Class: Socketable", false),
        ("Show", "Gold", "BaseType: Gold; StackSize >= 500", false),
        (
            "Hide",
            "Small gold piles",
            "BaseType: Gold; StackSize < 100",
            false,
        ),
        (
            "Hide",
            "Normal weapons",
            "Rarity Normal; Class: 13 items",
            false,
        ),
        (
            "Hide",
            "Magic weapons",
            "Rarity Magic; AreaLevel >= 65",
            false,
        ),
        (
            "Hide",
            "Low-tier rares",
            "UnidentifiedItemTier <= 3; AreaLevel >= 70",
            false,
        ),
        (
            "Hide",
            "Scrolls of wisdom",
            "BaseType: Scroll of Wisdom",
            false,
        ),
        ("Hide", "Flask fallback", "Class: 4 items", false),
        (
            "Hide",
            "Jewellery fallback",
            "Class: Rings, Amulets, Belts",
            false,
        ),
        ("Hide", "Gear fallback", "Class: 21 items", false),
    ];
    names
        .iter()
        .enumerate()
        .map(|(i, (action, name, summary, cont))| Rule {
            name,
            action,
            summary: (*summary).to_string(),
            enabled: i % 7 != 5,
            cont: *cont,
        })
        .collect()
}

/// The committed default styles, read at compile time (never from disk at run time, never
/// written).
const STYLES_DEFAULT: &str = include_str!("../../styles-default.json");

/// The fake style list: `styles-default.json` parsed in memory, in file order.
fn fake_styles() -> Vec<Style> {
    let map: serde_json::Map<String, serde_json::Value> =
        serde_json::from_str(STYLES_DEFAULT).expect("styles-default.json is valid JSON");
    // serde_json's map may sort its keys; recover the file order from the raw text.
    let mut names: Vec<&String> = map.keys().collect();
    names.sort_by_key(|n| {
        STYLES_DEFAULT
            .find(&format!("\"{n}\":"))
            .unwrap_or(usize::MAX)
    });
    names
        .into_iter()
        .map(|name| parse_style(name, &map[name.as_str()]))
        .collect()
}

fn parse_style(name: &str, v: &serde_json::Value) -> Style {
    let rgb = |key: &str| -> Option<Color32> {
        let a = v.get(key)?.as_array()?;
        let c = |i: usize| a.get(i).and_then(|x| x.as_u64()).unwrap_or(0).min(255) as u8;
        Some(Color32::from_rgb(c(0), c(1), c(2)))
    };
    let colors = [rgb("text"), rgb("border"), rgb("background")];
    let fallback = [
        Color32::from_rgb(248, 248, 242),
        Color32::from_rgb(248, 248, 242),
        Color32::from_rgb(40, 42, 54),
    ];
    let remembered = [0, 1, 2].map(|k| colors[k].unwrap_or(fallback[k]));
    let str_at = |obj: &str, key: &str| -> Option<String> {
        Some(v.get(obj)?.get(key)?.as_str()?.to_owned())
    };
    let mm = v.get("minimap").filter(|m| !m.is_null());
    let colour_index = |name: Option<String>| {
        name.and_then(|n| minimap::index_of(&COLOURS, &n))
            .unwrap_or(4)
    };
    let sound = v.get("sound").filter(|s| !s.is_null());
    Style {
        name: name.to_owned(),
        builtin: v
            .get("id")
            .and_then(|i| i.as_str())
            .is_some_and(|i| i.starts_with("builtin:")),
        show: v.get("action").and_then(|a| a.as_str()) != Some("Hide"),
        colors,
        remembered,
        font: v.get("font").and_then(|f| f.as_f64()),
        mm_size: mm
            .and_then(|m| m.get("size"))
            .and_then(|s| s.as_u64())
            .map(|s| s.min(2) as usize),
        mm_colour: colour_index(str_at("minimap", "color")),
        mm_shape: str_at("minimap", "shape")
            .and_then(|s| minimap::index_of(&SHAPES, &s))
            .unwrap_or(0),
        beam: str_at("beam", "color").and_then(|c| minimap::index_of(&COLOURS, &c)),
        beam_temp: v
            .get("beam")
            .and_then(|b| b.get("temp"))
            .and_then(|t| t.as_bool())
            .unwrap_or(false),
        sound: match sound {
            Some(s) if s.get("custom").is_some() => 17,
            Some(s) => s
                .get("id")
                .and_then(|i| i.as_u64())
                .map_or(0, |i| i.clamp(1, 16) as usize),
            None => 0,
        },
        volume: sound
            .and_then(|s| s.get("vol"))
            .and_then(|x| x.as_f64())
            .unwrap_or(100.0),
    }
}
