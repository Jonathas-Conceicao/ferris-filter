//! The ABOUT tab: name, description, the orange Ferris, version, repository link, a divider, then
//! the disclaimer and credits pinned to the bottom of the page.

use egui::{Align, Color32, Layout, Rect, Sense, UiBuilder, pos2, vec2};

use crate::skin::Skin;
use crate::skin::fonts;
use crate::skin::palette as pal;
use crate::skin::widgets as w;

/// What the update button looks like, as plain data so the page stays stateless.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UpdateView {
    pub caption: String,
    pub enabled: bool,
    /// A verified installer is waiting: a click means install, not check.
    pub ready: bool,
    pub tooltip: &'static str,
}

/// What a click on the update button asks for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AboutAction {
    Check,
    InstallAndQuit,
}

/// The one-line description under the app name.
pub const APP_DESCRIPTION: &str = "Loot filter editor for Path of Exile 2";
/// The non-affiliation disclaimer.
pub const DISCLAIMER: &str =
    "This product isn't affiliated with or endorsed by Grinding Gear Games in any way.";
/// Who owns the art and fonts.
pub const OWNERSHIP: &str = "The game art and fonts belong to their owners. FerrisFilter is a free third-party tool for Path of Exile 2 players.";
/// Third-party credits, one line each.
pub const CREDITS: [&str; 3] = [
    "Game art and fonts: Grinding Gear Games",
    "Fontin typeface: exljbris",
    "Ferris the crab: Karen Rustad Tölva, public domain",
];

/// `Version 0.6` for a build of the release tag, `Version 0.6-dev` past it or with tracked edits
/// (from `git describe`, set by `build.rs`; the `Cargo.toml` version when built without git).
pub fn version_text() -> String {
    format!("Version {}", env!("FF_VERSION"))
}

/// The exact `git describe` of the build (`v0.6-3-gabc1234-dirty`), shown on hover of a `-dev`
/// version; empty when built without git or a tag.
pub const GIT_DESCRIBE: &str = env!("FF_GIT_DESCRIBE");

fn text(s: &str, size: f32, colour: Color32) -> egui::RichText {
    egui::RichText::new(s)
        .font(fonts::regular(size))
        .color(colour)
}

fn footer_label(ui: &mut egui::Ui, s: &str, size: f32, colour: Color32) {
    ui.add(
        egui::Label::new(text(s, size, colour))
            .halign(Align::Center)
            .wrap(),
    );
}

/// A horizontal gold rule over `rect`, transparent at both ends and strongest at the centre.
fn divider_mesh(rect: Rect) -> egui::Shape {
    let gold = pal::GOLD.gamma_multiply(0.7);
    let clear = Color32::TRANSPARENT;
    let mut mesh = egui::Mesh::default();
    let xs = [rect.min.x, rect.center().x, rect.max.x];
    let cols = [clear, gold, clear];
    for (x, c) in xs.into_iter().zip(cols) {
        mesh.colored_vertex(pos2(x, rect.min.y), c);
        mesh.colored_vertex(pos2(x, rect.max.y), c);
    }
    for i in 0..2u32 {
        let b = i * 2;
        mesh.add_triangle(b, b + 1, b + 2);
        mesh.add_triangle(b + 1, b + 3, b + 2);
    }
    egui::Shape::mesh(mesh)
}

/// Draws the ABOUT page into the available rect. Not scrollable; no sections or rows.
pub fn about_tab(ui: &mut egui::Ui, skin: &Skin, view: &UpdateView) -> Option<AboutAction> {
    let mut action = None;
    let area = ui.available_rect_before_wrap();

    let mut col = ui.new_child(
        UiBuilder::new()
            .max_rect(area)
            .layout(Layout::top_down(Align::Center)),
    );
    col.spacing_mut().item_spacing.y = 0.0;
    col.add_space(16.0);
    col.label(
        egui::RichText::new("FerrisFilter")
            .font(fonts::small_caps(21.0))
            .color(pal::HEADING),
    );
    col.add_space(4.0);
    col.label(text(APP_DESCRIPTION, 15.0, pal::LABEL));
    col.add_space(16.0);
    let [tw, th] = skin.ferris_about.size();
    let (rect, resp) =
        col.allocate_exact_size(vec2(112.0, 112.0 * th as f32 / tw as f32), Sense::hover());
    crate::skin::slice::paint_image(col.painter(), &skin.ferris_about, rect, Color32::WHITE);
    resp.widget_info(|| {
        egui::WidgetInfo::labeled(egui::WidgetType::Label, true, "Ferris the crab")
    });
    col.add_space(8.0);
    let version = col.label(text(&version_text(), 14.0, pal::MUTED));
    if env!("FF_VERSION").ends_with("-dev") && !GIT_DESCRIBE.is_empty() {
        version.on_hover_text(format!("Unreleased build ({GIT_DESCRIBE})"));
    }
    col.add_space(8.0);
    let button = w::ff_button_ex(&mut col, skin, &view.caption, 240.0, view.enabled, false);
    let button = if view.enabled {
        button.on_hover_text(view.tooltip)
    } else {
        button.on_disabled_hover_text(view.tooltip)
    };
    if view.enabled && button.clicked() {
        action = Some(if view.ready {
            AboutAction::InstallAndQuit
        } else {
            AboutAction::Check
        });
    }
    col.add_space(8.0);
    if !crate::skin::REPO_URL.is_empty() {
        col.hyperlink_to(
            text(crate::skin::REPO_LINK_TEXT, 15.0, pal::GOLD),
            crate::skin::REPO_URL,
        )
        .on_hover_text("Open the repository in your browser.");
    }
    col.add_space(16.0);
    let (strip, _) = col.allocate_exact_size(vec2(area.width(), 3.0), Sense::hover());
    // The game's title underline art fades out to one side only, so the divider is a symmetric
    // gold rule instead: brightest in the middle, fading to nothing at both ends.
    col.painter().add(divider_mesh(Rect::from_center_size(
        strip.center(),
        vec2(strip.width() * 0.8, 1.0),
    )));

    // Footer: a 480pt column centred in the page, stacked upward from 16pt above the bottom, so
    // it stays pinned however tall the window is.
    let width = 480.0_f32.min(area.width());
    let foot = Rect::from_min_max(
        pos2(area.center().x - width / 2.0, area.min.y),
        pos2(
            area.center().x + width / 2.0,
            (area.max.y - 16.0).max(area.min.y),
        ),
    );
    let mut fui = ui.new_child(
        UiBuilder::new()
            .max_rect(foot)
            .layout(Layout::bottom_up(Align::Center)),
    );
    fui.spacing_mut().item_spacing.y = 4.0;
    for line in CREDITS.iter().rev() {
        footer_label(&mut fui, line, 14.0, pal::MUTED);
    }
    fui.add_space(12.0);
    footer_label(&mut fui, OWNERSHIP, 15.0, pal::MUTED);
    footer_label(&mut fui, DISCLAIMER, 15.0, pal::LABEL);
    action
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::overlay::update_job::UpdateState;
    use crate::update::ReadyUpdate;

    #[test]
    fn copy_matches_the_spec() {
        assert_eq!(APP_DESCRIPTION, "Loot filter editor for Path of Exile 2");
        assert_eq!(
            DISCLAIMER,
            "This product isn't affiliated with or endorsed by Grinding Gear Games in any way."
        );
        assert_eq!(
            OWNERSHIP,
            "The game art and fonts belong to their owners. FerrisFilter is a free third-party tool for Path of Exile 2 players."
        );
        assert_eq!(CREDITS[0], "Game art and fonts: Grinding Gear Games");
        assert_eq!(CREDITS[1], "Fontin typeface: exljbris");
        assert_eq!(
            CREDITS[2],
            "Ferris the crab: Karen Rustad Tölva, public domain"
        );
    }

    #[test]
    fn version_text_names_the_build_version() {
        let v = version_text();
        assert!(v.starts_with("Version "));
        assert!(v.ends_with(env!("FF_VERSION")));
    }

    fn states() -> Vec<UpdateState> {
        vec![
            UpdateState::Idle,
            UpdateState::Checking,
            UpdateState::Downloading {
                version: "0.8.0".into(),
            },
            UpdateState::Ready(ReadyUpdate {
                version: "0.8.0".into(),
                published_unix: None,
                installer: std::path::PathBuf::from("x.exe"),
                sha256: String::new(),
            }),
        ]
    }

    fn view_of(state: &UpdateState) -> UpdateView {
        UpdateView {
            caption: state.caption(),
            enabled: state.enabled(),
            ready: matches!(state, UpdateState::Ready(_)),
            tooltip: state.tooltip(),
        }
    }

    #[test]
    fn button_copy_matches_the_spec() {
        let views: Vec<UpdateView> = states().iter().map(view_of).collect();
        let captions: Vec<&str> = views.iter().map(|v| v.caption.as_str()).collect();
        assert_eq!(
            captions,
            [
                "Check for updates",
                "Checking…",
                "Downloading…",
                "Install 0.8.0 and quit"
            ]
        );
        let tips: Vec<&str> = views.iter().map(|v| v.tooltip).collect();
        assert_eq!(
            tips,
            [
                "Look for a newer version on GitHub.",
                "Asking GitHub.",
                "Downloading and checking the installer.",
                "Closes FerrisFilter and starts the installer."
            ]
        );
        let enabled: Vec<bool> = views.iter().map(|v| v.enabled).collect();
        assert_eq!(enabled, [true, false, false, true]);
    }

    fn frame(
        ctx: &egui::Context,
        skin: &Skin,
        view: &UpdateView,
        click: Option<egui::Pos2>,
    ) -> Option<AboutAction> {
        let area = Rect::from_min_size(pos2(0.0, 0.0), vec2(535.0, 760.0));
        let mut input = egui::RawInput {
            screen_rect: Some(area),
            ..Default::default()
        };
        if let Some(p) = click {
            input.events.push(egui::Event::PointerMoved(p));
            for pressed in [true, false] {
                input.events.push(egui::Event::PointerButton {
                    pos: p,
                    button: egui::PointerButton::Primary,
                    pressed,
                    modifiers: Default::default(),
                });
            }
        }
        let mut out = None;
        ctx.run_ui(input, |ui| {
            let mut child = ui.new_child(UiBuilder::new().max_rect(area));
            out = about_tab(&mut child, skin, view);
        })
        .drop_without_applying_deltas();
        out
    }

    #[test]
    fn click_returns_the_matching_action_and_disabled_returns_none() {
        let expected = [
            Some(AboutAction::Check),
            None,
            None,
            Some(AboutAction::InstallAndQuit),
        ];
        for (state, expect) in states().iter().zip(expected) {
            let ctx = egui::Context::default();
            let skin = crate::skin::install(&ctx);
            let view = view_of(state);
            assert_eq!(frame(&ctx, &skin, &view, None), None);
            // The button is centred in the 535pt page; scan down the centre for it.
            let mut got = None;
            for y in (120..420).step_by(3) {
                if let Some(a) = frame(&ctx, &skin, &view, Some(pos2(267.0, y as f32))) {
                    got = Some(a);
                    break;
                }
            }
            assert_eq!(got, expect, "{state:?}");
        }
    }

    fn render(w: f32, h: f32, view: &UpdateView) {
        let ctx = egui::Context::default();
        let skin = crate::skin::install(&ctx);
        let input = egui::RawInput {
            screen_rect: Some(Rect::from_min_size(pos2(0.0, 0.0), vec2(w, h))),
            ..Default::default()
        };
        ctx.run_ui(input, |ui| {
            let area = Rect::from_min_size(pos2(0.0, 0.0), vec2(w, h));
            let mut child = ui.new_child(UiBuilder::new().max_rect(area));
            about_tab(&mut child, &skin, view);
        })
        .drop_without_applying_deltas();
    }

    #[test]
    fn about_renders_in_a_full_page() {
        for s in states() {
            render(535.0, 760.0, &view_of(&s));
        }
    }

    #[test]
    fn about_renders_in_a_cramped_page() {
        for s in states() {
            render(535.0, 300.0, &view_of(&s));
        }
    }
}
