//! The skin's colours: PoE2 golds and browns over near-black, sampled from the game's Options
//! window (screenshot `poe2_print009`), plus the overlay theme's values that carry over.

use egui::Color32;

/// Window ground behind everything (and behind the Ferris drawing).
pub const GROUND: Color32 = Color32::from_rgb(0x0A, 0x0A, 0x09);
/// The framed content area's ground (the gear-free middle of the Options page frame).
pub const PANEL: Color32 = Color32::from_rgb(0x0F, 0x0E, 0x0D);

/// Section headings ("Display Settings"): pale gold.
pub const HEADING: Color32 = Color32::from_rgb(0xE6, 0xD3, 0xA6);
/// Row labels ("Renderer", "Mode"): warm tan.
pub const LABEL: Color32 = Color32::from_rgb(0xD8, 0xC1, 0x8E);
/// Values inside controls (dropdown text, slider value).
pub const VALUE: Color32 = Color32::from_rgb(0xEC, 0xEA, 0xE4);
/// Secondary text (rule summaries, captions).
pub const MUTED: Color32 = Color32::from_rgb(0xA9, 0xA2, 0x8C);
/// Disabled text and inactive tabs.
pub const DISABLED: Color32 = Color32::from_rgb(0x88, 0x85, 0x6E);
/// Bright gold accent (hover, focus rings, the Continue marker).
pub const GOLD: Color32 = Color32::from_rgb(0xC8, 0xA2, 0x5A);
/// Dark brown for text sitting on the gold title plate and on the light selected tab.
pub const ON_GOLD: Color32 = Color32::from_rgb(0x3A, 0x28, 0x10);
/// Button captions.
pub const BUTTON_TEXT: Color32 = Color32::from_rgb(0xE2, 0xDA, 0xC6);

/// Status text: a success ("Wrote dynamic.filter.").
pub const OK: Color32 = Color32::from_rgb(0x9C, 0xC0, 0x78);
/// Status text: a warning (applied, but something needs a look).
pub const WARN: Color32 = Color32::from_rgb(0xE8, 0xB0, 0x52);
/// Status text: an error (nothing was written).
pub const ERROR: Color32 = Color32::from_rgb(0xE0, 0x62, 0x52);

/// Alternating row grounds: translucent, so the static background shows faintly through.
pub const ROW_EVEN: Color32 = Color32::from_rgba_premultiplied(0, 0, 0, 96);
/// See [`ROW_EVEN`].
pub const ROW_ODD: Color32 = Color32::from_rgba_premultiplied(7, 7, 6, 7);
/// Row ground while hovered.
pub const ROW_HOVER: Color32 = Color32::from_rgba_premultiplied(18, 15, 9, 24);

/// Input / dropdown box fill (recessed, darker than a row).
pub const INPUT_FILL: Color32 = Color32::from_rgb(0x07, 0x07, 0x07);
/// Input / dropdown box border.
pub const INPUT_BORDER: Color32 = Color32::from_rgb(0x3E, 0x3B, 0x34);
/// Input / dropdown box border while hovered or open.
pub const INPUT_BORDER_HOT: Color32 = Color32::from_rgb(0x7A, 0x68, 0x44);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn text_colours_are_brighter_than_the_grounds() {
        // A cheap legibility floor: every text colour must sit far above the panel ground.
        let lum = |c: Color32| c.r() as u32 + c.g() as u32 + c.b() as u32;
        for c in [HEADING, LABEL, VALUE, MUTED, BUTTON_TEXT, OK, WARN, ERROR] {
            assert!(lum(c) > lum(PANEL) + 300, "{c:?} is too dark to read");
        }
    }
}
