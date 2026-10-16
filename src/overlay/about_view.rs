//! The ABOUT tab: name, description, the orange Ferris, version, repository link, a divider, then
//! the disclaimer and credits pinned to the bottom of the page.

use egui::{Align, Color32, Layout, Rect, Sense, UiBuilder, pos2, vec2};

use crate::skin::Skin;
use crate::skin::fonts;
use crate::skin::palette as pal;

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

/// `Version x.y.z` for the running build.
pub fn version_text() -> String {
    format!("Version {}", env!("CARGO_PKG_VERSION"))
}

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
pub fn about_tab(ui: &mut egui::Ui, skin: &Skin) {
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
    col.label(text(&version_text(), 14.0, pal::MUTED));
    col.add_space(4.0);
    col.hyperlink_to(
        text(crate::skin::REPO_LINK_TEXT, 15.0, pal::GOLD),
        crate::skin::REPO_URL,
    )
    .on_hover_text("Open the repository in your browser.");
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
}

#[cfg(test)]
mod tests {
    use super::*;

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
    fn version_text_names_the_crate_version() {
        let v = version_text();
        assert!(v.starts_with("Version "));
        assert!(v.contains(env!("CARGO_PKG_VERSION")));
    }

    fn render(w: f32, h: f32) {
        let ctx = egui::Context::default();
        let skin = crate::skin::install(&ctx);
        let input = egui::RawInput {
            screen_rect: Some(Rect::from_min_size(pos2(0.0, 0.0), vec2(w, h))),
            ..Default::default()
        };
        ctx.run_ui(input, |ui| {
            let area = Rect::from_min_size(pos2(0.0, 0.0), vec2(w, h));
            let mut child = ui.new_child(UiBuilder::new().max_rect(area));
            about_tab(&mut child, &skin);
        })
        .drop_without_applying_deltas();
    }

    #[test]
    fn about_renders_in_a_full_page() {
        render(535.0, 760.0);
    }

    #[test]
    fn about_renders_in_a_cramped_page() {
        render(535.0, 300.0);
    }
}
