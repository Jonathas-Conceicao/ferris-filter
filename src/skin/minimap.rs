//! The item-filter minimap icons and colours, as the game draws them.
//!
//! `assets/poe2/minimap/icons.png` (written by `ff-art` from the game's minimap sheet) is a grid
//! of 64-pixel cells: one row per [`COLOURS`] entry, one column per [`SHAPES`] entry, each cell the
//! game's Large icon for that shape in that colour. Smaller sizes are the same art drawn smaller.

use egui::{Color32, Rect, pos2};

/// The 11 colours a filter may give `MinimapIcon` and `PlayEffect`, in the atlas's row order.
pub const COLOURS: [&str; 11] = [
    "Red", "Green", "Blue", "Brown", "White", "Yellow", "Cyan", "Grey", "Orange", "Pink", "Purple",
];

/// The 12 `MinimapIcon` shapes, in the atlas's column order.
pub const SHAPES: [&str; 12] = [
    "Circle",
    "Diamond",
    "Hexagon",
    "Square",
    "Star",
    "Triangle",
    "Cross",
    "Moon",
    "Raindrop",
    "Kite",
    "Pentagon",
    "UpsideDownHouse",
];

/// The `MinimapIcon` sizes: 0 is the largest.
pub const SIZES: [&str; 3] = ["Large", "Medium", "Small"];

/// A swatch colour for each [`COLOURS`] entry, sampled from the middle of the game's icons so a
/// swatch matches the icon drawn next to it.
pub const SWATCHES: [Color32; 11] = [
    Color32::from_rgb(0xE0, 0x3C, 0x4A), // Red
    Color32::from_rgb(0x5C, 0xB8, 0x1C), // Green
    Color32::from_rgb(0x2E, 0x78, 0xD0), // Blue
    Color32::from_rgb(0x9A, 0x44, 0x18), // Brown
    Color32::from_rgb(0xD8, 0xDC, 0xDC), // White
    Color32::from_rgb(0xD8, 0xA8, 0x3C), // Yellow
    Color32::from_rgb(0x2C, 0xC8, 0xD0), // Cyan
    Color32::from_rgb(0x58, 0x58, 0x5C), // Grey
    Color32::from_rgb(0xF0, 0x84, 0x10), // Orange
    Color32::from_rgb(0xE8, 0x8C, 0xD8), // Pink
    Color32::from_rgb(0x9C, 0x2C, 0xD8), // Purple
];

/// The on-screen size, in points, of an icon of `size` (0 = Large) drawn at `large` points.
pub fn icon_points(size: usize, large: f32) -> f32 {
    match size {
        0 => large,
        1 => large * 0.78,
        _ => large * 0.58,
    }
}

/// UV rect of the icon for `shape` in `colour` (indices into [`SHAPES`] / [`COLOURS`]; out of
/// range indices are clamped), inset by half a texel of a `tex_size` atlas.
pub fn icon_uv(shape: usize, colour: usize, tex_size: [usize; 2]) -> Rect {
    let (cols, rows) = (SHAPES.len() as f32, COLOURS.len() as f32);
    let (c, r) = (
        shape.min(SHAPES.len() - 1) as f32,
        colour.min(COLOURS.len() - 1) as f32,
    );
    let (hx, hy) = (
        0.5 / tex_size[0].max(1) as f32,
        0.5 / tex_size[1].max(1) as f32,
    );
    Rect::from_min_max(
        pos2(c / cols + hx, r / rows + hy),
        pos2((c + 1.0) / cols - hx, (r + 1.0) / rows - hy),
    )
}

/// Index of `name` in `list`, ignoring case (filter keywords are case-insensitive in practice).
pub fn index_of(list: &[&str], name: &str) -> Option<usize> {
    list.iter().position(|n| n.eq_ignore_ascii_case(name))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn uv_addresses_the_right_cell() {
        let uv = icon_uv(0, 0, [768, 704]);
        assert!(uv.min.x > 0.0 && uv.max.x < 1.0 / 12.0 + 1e-6);
        assert!(uv.min.y > 0.0 && uv.max.y < 1.0 / 11.0 + 1e-6);
        let last = icon_uv(11, 10, [768, 704]);
        assert!(last.max.x < 1.0 && last.min.x > 11.0 / 12.0);
        assert!(last.max.y < 1.0 && last.min.y > 10.0 / 11.0);
        // Out of range clamps to the last cell instead of sampling outside the atlas.
        assert_eq!(icon_uv(99, 99, [768, 704]), last);
    }

    #[test]
    fn names_resolve_case_insensitively() {
        assert_eq!(index_of(&SHAPES, "kite"), Some(9));
        assert_eq!(index_of(&COLOURS, "PURPLE"), Some(10));
        assert_eq!(index_of(&SHAPES, "Hexagram"), None);
    }
}
