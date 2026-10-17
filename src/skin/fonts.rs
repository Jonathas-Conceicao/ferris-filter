//! Font setup: Fontin SmallCaps and Fontin Regular (shipped with PoE2, by exljbris / Jos
//! Buivenga).
//!
//! Fontin SmallCaps draws lowercase letters as small capitals, so mixed-case text such as
//! "Display Settings" renders the way the game's headings do -- no OpenType `smcp` support
//! needed from egui's rasterizer.

use egui::{FontFamily, FontId};

/// Family name of Fontin SmallCaps (headings, row labels, tabs, buttons).
pub const SMALL_CAPS: &str = "fontin-sc";
/// Family name of Fontin Regular (body text, values, summaries).
pub const REGULAR: &str = "fontin";
/// Size of summary text and small captions in the overlay (rule summaries, button captions).
pub const SUMMARY_SIZE: f32 = 15.0;

/// A Fontin SmallCaps font of `size` points.
pub fn small_caps(size: f32) -> FontId {
    FontId::new(size, FontFamily::Name(SMALL_CAPS.into()))
}

/// A Fontin Regular font of `size` points.
pub fn regular(size: f32) -> FontId {
    FontId::new(size, FontFamily::Name(REGULAR.into()))
}

/// Installs the fonts: the two named families above, and Fontin Regular as the default
/// proportional face. Every family keeps egui's default chain as a fallback, so symbols the
/// Fontin faces lack (arrows, `✕`) still render instead of tofu.
pub fn install(ctx: &egui::Context) {
    let mut fonts = egui::FontDefinitions::default();
    let data = [
        (
            SMALL_CAPS,
            &include_bytes!("../../assets/poe2/fonts/Fontin-SmallCaps.ttf")[..],
        ),
        (
            REGULAR,
            &include_bytes!("../../assets/poe2/fonts/Fontin-Regular.ttf")[..],
        ),
    ];
    let default_chain = fonts
        .families
        .get(&FontFamily::Proportional)
        .cloned()
        .unwrap_or_default();
    for (name, bytes) in data {
        fonts
            .font_data
            .insert(name.to_owned(), egui::FontData::from_static(bytes).into());
        let family = fonts
            .families
            .entry(FontFamily::Name(name.into()))
            .or_default();
        family.push(name.to_owned());
        family.extend(default_chain.iter().cloned());
    }
    fonts
        .families
        .entry(FontFamily::Proportional)
        .or_default()
        .insert(0, REGULAR.to_owned());
    ctx.set_fonts(fonts);
}
