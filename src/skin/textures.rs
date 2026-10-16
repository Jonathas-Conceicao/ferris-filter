//! Every texture the skin paints with, embedded in the binary and uploaded once.
//!
//! The PNGs are written by `tools/ff-art` from the game files listed in
//! `assets/poe2/manifest.toml`. Most are the game's 4K variants; they are uploaded with
//! LINEAR filtering and mipmaps so they stay smooth when drawn at half size on a 1080p screen.

use egui::{ColorImage, Context, TextureFilter, TextureHandle, TextureOptions};

/// Upload options for every skin texture: bilinear, clamped, with linear mipmaps.
pub const OPTIONS: TextureOptions = TextureOptions {
    mipmap_mode: Some(TextureFilter::Linear),
    ..TextureOptions::LINEAR
};

/// Decodes an embedded PNG into an egui image. Panics on a corrupt embedded asset, which is a
/// build-time mistake (the bytes are compiled in), never a user-input condition.
pub fn decode_png(name: &str, bytes: &[u8]) -> ColorImage {
    let img = image::load_from_memory_with_format(bytes, image::ImageFormat::Png)
        .unwrap_or_else(|e| panic!("embedded skin texture {name} is not a valid PNG: {e}"))
        .to_rgba8();
    let size = [img.width() as usize, img.height() as usize];
    ColorImage::from_rgba_unmultiplied(size, img.as_raw())
}

fn load(ctx: &Context, name: &str, bytes: &[u8]) -> TextureHandle {
    ctx.load_texture(format!("skin/{name}"), decode_png(name, bytes), OPTIONS)
}

macro_rules! tex {
    ($ctx:expr, $path:literal) => {
        load($ctx, $path, include_bytes!(concat!("../../assets/", $path)))
    };
}

/// Left cap, stretched middle, right cap.
#[derive(Clone)]
pub struct ThreeSlice {
    pub left: TextureHandle,
    pub middle: TextureHandle,
    pub right: TextureHandle,
}

/// The skin's textures. Cheap to clone (handles are reference counted).
#[derive(Clone)]
pub struct Skin {
    /// The Options window's ornate header (angels, hooded figure, gold title plate).
    pub title_bar: TextureHandle,
    /// The Options page: stone frame, thin gold inner border, recessed button bar. Its middle
    /// (the game's gear drawing) is transparent.
    pub pages_frame: TextureHandle,
    /// A tab (stone shape; the selected one is drawn brighter).
    pub tab: ThreeSlice,
    /// Slider track.
    pub slider_track: ThreeSlice,
    /// Slider gold fill (left of the thumb).
    pub slider_fill: ThreeSlice,
    /// Slider diamond thumb.
    pub slider_thumb: TextureHandle,
    /// Gold-edged dark square for popups (nine-sliced).
    pub box_9slice: TextureHandle,
    /// Scrollbar track (a thin vertical line).
    pub scroll_track: TextureHandle,
    /// Scrollbar thumb: top cap (`left`), stretched centre, bottom cap (`right`).
    pub scroll_thumb: ThreeSlice,
    /// Thin gold rule under section titles.
    pub title_underline: TextureHandle,
    /// Round checkbox, checked (gold tick).
    pub check_on: TextureHandle,
    /// Round checkbox, empty ring.
    pub check_off: TextureHandle,
    /// Round red close button: normal, hover, pressed.
    pub close: [TextureHandle; 3],
    /// Gold dropdown triangle: normal, hover.
    pub combo_arrow: [TextureHandle; 2],
    /// Generic button: normal, hover, pressed.
    pub button: [ThreeSlice; 3],
    /// The static background drawing (Ferris as pale line art, 2x resolution).
    pub ferris_bg: TextureHandle,
    /// The item-filter minimap icons: one row per colour, one column per shape (see
    /// [`super::minimap`]).
    pub minimap_icons: TextureHandle,
}

impl Skin {
    /// Decodes and uploads every texture. Call once (e.g. in the eframe creation closure) and
    /// keep the result; it is not cached anywhere else.
    pub fn load(ctx: &Context) -> Self {
        let three = |l: TextureHandle, m: TextureHandle, r: TextureHandle| ThreeSlice {
            left: l,
            middle: m,
            right: r,
        };
        Self {
            title_bar: tex!(ctx, "poe2/options/titlebar.png"),
            pages_frame: tex!(ctx, "poe2/options/pages_frame.png"),
            tab: three(
                tex!(ctx, "poe2/options/tab_left.png"),
                tex!(ctx, "poe2/options/tab_center.png"),
                tex!(ctx, "poe2/options/tab_right.png"),
            ),
            slider_track: three(
                tex!(ctx, "poe2/options/slider_bg_left.png"),
                tex!(ctx, "poe2/options/slider_bg_middle.png"),
                tex!(ctx, "poe2/options/slider_bg_right.png"),
            ),
            slider_fill: three(
                tex!(ctx, "poe2/options/slider_fill_left.png"),
                tex!(ctx, "poe2/options/slider_fill_middle.png"),
                tex!(ctx, "poe2/options/slider_fill_right.png"),
            ),
            slider_thumb: tex!(ctx, "poe2/options/slider_thumb.png"),
            box_9slice: tex!(ctx, "poe2/options/box_9slice.png"),
            scroll_track: tex!(ctx, "poe2/options/scroll_track.png"),
            scroll_thumb: three(
                tex!(ctx, "poe2/options/scroll_thumb_top.png"),
                tex!(ctx, "poe2/options/scroll_thumb_center.png"),
                tex!(ctx, "poe2/options/scroll_thumb_bottom.png"),
            ),
            title_underline: tex!(ctx, "poe2/options/title_underline.png"),
            check_on: tex!(ctx, "poe2/common/check_on.png"),
            check_off: tex!(ctx, "poe2/common/check_off.png"),
            close: [
                tex!(ctx, "poe2/common/close_normal.png"),
                tex!(ctx, "poe2/common/close_hover.png"),
                tex!(ctx, "poe2/common/close_down.png"),
            ],
            combo_arrow: [
                tex!(ctx, "poe2/common/combo_arrow.png"),
                tex!(ctx, "poe2/common/combo_arrow_hover.png"),
            ],
            button: [
                three(
                    tex!(ctx, "poe2/common/button_normal_left.png"),
                    tex!(ctx, "poe2/common/button_normal_middle.png"),
                    tex!(ctx, "poe2/common/button_normal_right.png"),
                ),
                three(
                    tex!(ctx, "poe2/common/button_hover_left.png"),
                    tex!(ctx, "poe2/common/button_hover_middle.png"),
                    tex!(ctx, "poe2/common/button_hover_right.png"),
                ),
                three(
                    tex!(ctx, "poe2/common/button_pressed_left.png"),
                    tex!(ctx, "poe2/common/button_pressed_middle.png"),
                    tex!(ctx, "poe2/common/button_pressed_right.png"),
                ),
            ],
            ferris_bg: tex!(ctx, "ui/ferris-bg.png"),
            minimap_icons: tex!(ctx, "poe2/minimap/icons.png"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_embedded_texture_decodes() {
        // Loading into a headless context decodes every PNG; a bad asset panics here rather than
        // on the author's screen.
        let ctx = Context::default();
        let skin = Skin::load(&ctx);
        assert_eq!(skin.pages_frame.size(), [1310, 1582]);
        assert!(skin.title_bar.size()[0] > skin.title_bar.size()[1]);
        // 12 shapes x 11 colours of 64px cells.
        assert_eq!(skin.minimap_icons.size(), [12 * 64, 11 * 64]);
    }

    #[test]
    fn textures_are_mipmapped_and_linear() {
        assert_eq!(OPTIONS.mipmap_mode, Some(TextureFilter::Linear));
        assert_eq!(OPTIONS.magnification, TextureFilter::Linear);
        assert_eq!(OPTIONS.minification, TextureFilter::Linear);
    }
}
