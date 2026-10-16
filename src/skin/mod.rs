//! A Path of Exile 2 look for egui, built from the game's own Options-window art.
//!
//! This module is the shared skin for the UI redo: the `ff-proto` binary shows every piece of
//! it, and the `ff` overlay can adopt it later. The overlay does **not** use it yet; its look
//! still comes from `overlay::theme`.
//!
//! - [`palette`]: PoE2 golds, browns and near-blacks.
//! - [`fonts`]: Fontin SmallCaps (headings, labels -- the face the game uses) and Fontin Regular
//!   (body), with Cinzel kept registered because `ff` uses it.
//! - [`Skin`]: every texture, embedded with `include_bytes!` and uploaded once
//!   (LINEAR filtering with mipmaps, so the 4K art scales down cleanly).
//! - [`slice`]: pixel-snapped nine-slice / three-slice painting with a half-texel UV inset.
//! - [`widgets`]: the controls (`ff_frame`, `ff_header`, `ff_tabs`, `ff_section`, `ff_row`,
//!   `ff_check`, `ff_slider`, `ff_dropdown`, `ff_button`, `ff_close`, `ff_scroll_area`,
//!   `ff_status`).
//! - [`editor`]: the style-editor pieces (segmented switch, colour swatches and cards, item-label
//!   preview, minimap icons, cards).
//! - [`minimap`]: the item-filter minimap shapes / colours and their atlas lookup.
//! - [`window`]: DPI-correct docking geometry.
//!
//! The PNGs come from `tools/ff-art` (see `assets/poe2/manifest.toml`); the art belongs to
//! Grinding Gear Games (see README.md).

pub mod editor;
pub mod fonts;
pub mod minimap;
pub mod palette;
pub mod slice;
pub mod textures;
pub mod widgets;
pub mod window;

pub use textures::Skin;

/// One-call setup: installs the fonts and the skin's style on `ctx` and loads the textures.
/// Call once, before the first frame paints, and keep the returned [`Skin`].
pub fn install(ctx: &egui::Context) -> Skin {
    fonts::install(ctx);
    widgets::apply_style(ctx);
    Skin::load(ctx)
}
