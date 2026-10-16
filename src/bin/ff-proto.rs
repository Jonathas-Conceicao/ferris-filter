//! `ff-proto` -- a look-only prototype of FerrisFilter in Path of Exile 2's own UI style.
//!
//! Shows the shared skin (`ferrisfilter::skin`) on fake data so the author can judge the look
//! before the UI-redo milestone: a 660-point window docked to the left edge of the primary
//! monitor's work area at full height, the game's Options header / tabs / frame / controls, and
//! a static Ferris drawing behind the scrolling content. Nothing is read or written; the
//! buttons only set the status line.
//!
//! ```text
//! cargo run --bin ff-proto
//! cargo run --bin ff-proto -- --tab config --screenshot shot.png
//! ```
//!
//! `--screenshot <png>` renders a few frames, saves one (physical pixels) and exits.
//! `--tab rules|config|economy` picks the starting tab.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::path::PathBuf;

use egui::{Align, Align2, Color32, Layout, Rect, Sense, UiBuilder, pos2, vec2};
use ferrisfilter::skin::{self, Skin, fonts, palette as pal, widgets as w};

/// The window's fixed width, in logical points.
const WIDTH: f32 = 660.0;
const TABS: [&str; 3] = ["Rules", "Config", "Economy"];

struct Args {
    screenshot: Option<PathBuf>,
    tab: usize,
}

fn parse_args() -> Args {
    let mut args = Args {
        screenshot: None,
        tab: 0,
    };
    let mut it = std::env::args().skip(1);
    while let Some(a) = it.next() {
        match a.as_str() {
            "--screenshot" => args.screenshot = it.next().map(PathBuf::from),
            "--tab" => {
                args.tab = match it.next().as_deref() {
                    Some("config") => 1,
                    Some("economy") => 2,
                    _ => 0,
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

/// One fake style row.
struct StyleRow {
    name: &'static str,
    visible: bool,
    text: Color32,
    border: Color32,
    bg: Color32,
}

struct Proto {
    skin: Skin,
    args: Args,
    tab: usize,
    docked: bool,
    frames: u32,
    shot_requested: bool,
    status: String,

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
    styles: Vec<StyleRow>,
    tiers: Vec<(&'static str, f64)>,
}

impl Proto {
    fn new(skin: Skin, args: Args) -> Self {
        let tab = args.tab;
        Self {
            skin,
            tab,
            args,
            docked: false,
            frames: 0,
            shot_requested: false,
            status: "Prototype: fake data, nothing is saved.".into(),
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
            styles: fake_styles(),
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
            _ => self.economy_tab(&mut cui, &skin),
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
            self.status = "APPLY pressed (prototype: nothing was written).".into();
        }
        if w::ff_button(&mut bui, skin, "Reload", 84.0).clicked() {
            self.status = "RELOAD pressed (prototype: nothing was read).".into();
        }
        if w::ff_button(&mut bui, skin, "Revert", 84.0).clicked() {
            self.status = "REVERT pressed (prototype: nothing to revert).".into();
        }
        let status_rect =
            Rect::from_min_max(inner.min, pos2(bui.min_rect().min.x - 8.0, inner.max.y));
        ui.painter().with_clip_rect(status_rect).text(
            pos2(status_rect.min.x, status_rect.center().y),
            Align2::LEFT_CENTER,
            &self.status,
            fonts::regular(13.0),
            pal::MUTED,
        );
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
                self.status = "+ ADD RULE pressed (prototype).".into();
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
            self.status = format!("Moved rule {} down (prototype).", i + 1);
        }
        if w::ff_arrow(&mut right, skin, true, i > 0).clicked() {
            self.status = format!("Moved rule {} up (prototype).", i + 1);
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
            Align2::LEFT_CENTER,
            rule.name,
            fonts::small_caps(16.0),
            fade(pal::LABEL),
        );
        p.text(
            pos2(text_left, rect.min.y + 34.0),
            Align2::LEFT_CENTER,
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
                self.status = "Operator changed (prototype).".into();
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
        w::ff_row(&mut body, 1, "Style", |ui| {
            w::ff_dropdown(
                ui,
                skin,
                "style",
                &mut self.style,
                &[
                    "Custom", "S", "A", "B", "C", "D", "E", "F", "Recycle", "kite", "hidden",
                ],
                0.0,
            );
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
            for (i, s) in self.styles.iter_mut().enumerate() {
                let (rect, _) = ui
                    .allocate_exact_size(vec2(ui.available_width(), w::ROW_HEIGHT), Sense::hover());
                w::row_background(ui, i, rect);
                let mut row = ui.new_child(
                    UiBuilder::new()
                        .max_rect(rect.shrink2(vec2(w::ROW_PAD, 0.0)))
                        .layout(Layout::left_to_right(Align::Center)),
                );
                w::ff_check(&mut row, skin, &mut s.visible, "Visible");
                row.add_space(12.0);
                let color = if s.visible { pal::LABEL } else { pal::DISABLED };
                row.label(
                    egui::RichText::new(s.name)
                        .font(fonts::small_caps(17.0))
                        .color(color),
                );
                // Colour preview chip on the right.
                let chip = Rect::from_min_size(
                    pos2(rect.max.x - w::ROW_PAD - 96.0, rect.center().y - 11.0),
                    vec2(96.0, 22.0),
                );
                let p = ui.painter();
                p.rect(
                    chip,
                    1.0,
                    s.bg,
                    egui::Stroke::new(1.5, s.border),
                    egui::StrokeKind::Inside,
                );
                p.text(
                    chip.center(),
                    Align2::CENTER_CENTER,
                    "Item",
                    fonts::regular(14.0),
                    s.text,
                );
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
                    self.status = "SYNC ECONOMY pressed (prototype: no network).".into();
                }
            });
        });
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

fn fake_styles() -> Vec<StyleRow> {
    let bg = Color32::from_rgb(40, 42, 54);
    let style = |name, rgb: (u8, u8, u8), visible| StyleRow {
        name,
        visible,
        text: Color32::from_rgb(rgb.0, rgb.1, rgb.2),
        border: Color32::from_rgb(rgb.0, rgb.1, rgb.2),
        bg,
    };
    vec![
        style("S", (255, 121, 198), true),
        style("A", (189, 147, 249), true),
        style("B", (255, 85, 85), true),
        style("C", (241, 250, 140), true),
        style("D", (139, 233, 253), true),
        style("E", (248, 248, 242), true),
        style("F", (248, 248, 242), false),
        style("Recycle", (80, 250, 123), true),
        style("kite", (255, 184, 108), true),
        style("exceptional", (255, 121, 198), true),
        StyleRow {
            name: "unchanged",
            visible: true,
            text: Color32::from_rgb(200, 200, 200),
            border: Color32::from_gray(70),
            bg: Color32::from_rgb(20, 20, 20),
        },
        StyleRow {
            name: "hidden",
            visible: false,
            text: Color32::from_gray(150),
            border: Color32::from_gray(50),
            bg: Color32::from_rgb(12, 12, 12),
        },
    ]
}
