//! A Path of Exile 2 look for egui, built from the game's own Options-window art.
//!
//! This module is the shared skin of the `ff` overlay: palette, fonts, textures, painting and the
//! `ff_*` widgets.
//!
//! - [`palette`]: PoE2 golds, browns and near-blacks.
//! - [`fonts`]: Fontin SmallCaps (headings, labels -- the face the game uses) and Fontin Regular
//!   (body).
//! - [`Skin`]: every texture, embedded with `include_bytes!` and uploaded once
//!   (LINEAR filtering with mipmaps, so the 4K art scales down cleanly).
//! - [`slice`]: pixel-snapped nine-slice / three-slice painting with a half-texel UV inset.
//! - [`widgets`]: the controls (`ff_frame`, `ff_header`, `ff_tabs`, `ff_section`, `ff_row`,
//!   `ff_check`, `ff_slider`, `ff_dropdown`, `ff_button`, `ff_close`, `ff_scroll_area`,
//!   `ff_status`).
//! - [`editor`]: the style-editor pieces (segmented switch, colour swatches and cards, item-label
//!   preview, minimap icons, cards).
//! - [`colour_picker`]: the skinned colour picker popup shared by the style table and the rules.
//! - [`popup`]: the skinned modal (dimmed window, game box, game buttons).
//! - [`icons`]: painted flag icons for rule rows.
//! - [`minimap`]: the item-filter minimap shapes / colours and their atlas lookup.
//! - [`window`]: DPI-correct docking geometry.
//!
//! The PNGs come from `tools/ff-art` (see `assets/poe2/manifest.toml`); the art belongs to
//! Grinding Gear Games (see README.md).

pub mod colour_picker;
pub mod editor;
pub mod fonts;
pub mod icons;
pub mod minimap;
pub mod palette;
pub mod popup;
pub mod slice;
pub mod textures;
pub mod widgets;
pub mod window;

pub use textures::Skin;

/// The FerrisFilter repository, linked from the ABOUT tab.
pub const REPO_URL: &str = "https://github.com/Jonathas-Conceicao/poe2-myfilter";
/// The link text shown for [`REPO_URL`].
pub const REPO_LINK_TEXT: &str = "github.com/Jonathas-Conceicao/poe2-myfilter";

/// One-call setup: installs the fonts and the skin's style on `ctx` and loads the textures.
/// Call once, before the first frame paints, and keep the returned [`Skin`].
pub fn install(ctx: &egui::Context) -> Skin {
    fonts::install(ctx);
    widgets::apply_style(ctx);
    let skin = Skin::load(ctx);
    remember(ctx, &skin);
    skin
}

/// Stores a clone of `skin` in the context so any widget can fetch it with [`of`].
pub fn remember(ctx: &egui::Context, skin: &Skin) {
    ctx.data_mut(|d| d.insert_temp(egui::Id::new("ff-skin"), skin.clone()));
}

/// The skin installed on `ctx`. The overlay wrappers use this so their signatures need no
/// `&Skin`. When none was installed (headless tests that only installed fonts) it is loaded
/// once, remembered, and returned, so later calls reuse the same textures.
pub fn of(ctx: &egui::Context) -> Skin {
    if let Some(s) = ctx.data(|d| d.get_temp::<Skin>(egui::Id::new("ff-skin"))) {
        return s;
    }
    #[cfg(test)]
    let skin = textures::load_light(ctx);
    #[cfg(not(test))]
    let skin = Skin::load(ctx);
    remember(ctx, &skin);
    skin
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn repo_link_text_is_the_url_without_scheme() {
        assert!(REPO_URL.ends_with(REPO_LINK_TEXT));
    }

    #[test]
    fn skin_of_returns_the_installed_skin() {
        let ctx = egui::Context::default();
        let s = install(&ctx);
        assert_eq!(of(&ctx).check_on.id(), s.check_on.id());
    }

    #[test]
    fn skin_of_loads_once_when_absent() {
        let ctx = egui::Context::default();
        fonts::install(&ctx);
        let a = of(&ctx);
        let b = of(&ctx);
        assert_eq!(a.check_on.id(), b.check_on.id());
        assert_eq!(a.title_bar.id(), b.title_bar.id());
    }
}
