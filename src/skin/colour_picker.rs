//! The shared, game-styled colour picker for style slots and rule colour actions. It replaces
//! egui's built-in picker: a saturation/brightness square, a hue bar, Hex and R/G/B fields, an
//! alpha slider only where the target supports alpha, the nine Dracula quick picks and a None
//! tile only where the slot can be empty.

use egui::{Pos2, Rect};

/// Unmultiplied HSVA of a filter colour. Going through `Hsva::from_rgba_unmultiplied` (not a
/// premultiplied `Color32`) keeps the RGB of a fully transparent colour such as
/// `SetBackgroundColor 40 42 54 0` (WR-04).
pub fn hsva_of(rgba: [u8; 4]) -> egui::ecolor::Hsva {
    use egui::ecolor::linear_f32_from_gamma_u8 as lin;
    egui::ecolor::Hsva::from_rgba_unmultiplied(
        lin(rgba[0]),
        lin(rgba[1]),
        lin(rgba[2]),
        f32::from(rgba[3]) / 255.0,
    )
}

/// Parses `#RRGGBB`, `RRGGBB`, `#RGB` or `RGB` (surrounding spaces ignored). Anything else, such
/// as non-hex digits or a length other than 3 or 6, is `None`.
pub fn parse_hex(text: &str) -> Option<[u8; 3]> {
    let t = text.trim();
    let t = t.strip_prefix('#').unwrap_or(t);
    if !t.chars().all(|c| c.is_ascii_hexdigit()) {
        return None;
    }
    let nibble = |c: char| c.to_digit(16).map(|d| d as u8);
    let digits: Vec<u8> = t.chars().map(nibble).collect::<Option<_>>()?;
    match digits.len() {
        3 => Some([digits[0] * 17, digits[1] * 17, digits[2] * 17]),
        6 => Some([
            digits[0] * 16 + digits[1],
            digits[2] * 16 + digits[3],
            digits[4] * 16 + digits[5],
        ]),
        _ => None,
    }
}

/// `#RRGGBB`, upper case.
pub fn hex_of(rgb: [u8; 3]) -> String {
    format!("#{:02X}{:02X}{:02X}", rgb[0], rgb[1], rgb[2])
}

/// A decimal channel value 0-255 (surrounding spaces ignored); anything else is `None`.
pub fn parse_channel(text: &str) -> Option<u8> {
    let t = text.trim();
    if t.is_empty() || !t.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }
    t.parse::<u8>().ok()
}

/// Saturation (x) and brightness (y, top is bright) at `pos` inside the square `rect`, clamped.
pub fn sv_at(pos: Pos2, rect: Rect) -> (f32, f32) {
    let s = ((pos.x - rect.min.x) / rect.width().max(1.0)).clamp(0.0, 1.0);
    let v = 1.0 - ((pos.y - rect.min.y) / rect.height().max(1.0)).clamp(0.0, 1.0);
    (s, v)
}

/// Hue (0..=1) at horizontal position `x` along the bar `rect`, clamped.
pub fn hue_at(x: f32, rect: Rect) -> f32 {
    ((x - rect.min.x) / rect.width().max(1.0)).clamp(0.0, 1.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use egui::{pos2, vec2};

    #[test]
    fn hex_parses_and_formats() {
        assert_eq!(parse_hex("#FF79C6"), Some([255, 121, 198]));
        assert_eq!(parse_hex("ff79c6"), Some([255, 121, 198]));
        assert_eq!(parse_hex("#F0A"), Some([255, 0, 170]));
        assert_eq!(parse_hex(" #ff79c6 "), Some([255, 121, 198]));
        assert_eq!(parse_hex("#GG0000"), None);
        assert_eq!(parse_hex("#12345"), None);
        assert_eq!(parse_hex(""), None);
        assert_eq!(parse_hex("#"), None);
        assert_eq!(hex_of([255, 121, 198]), "#FF79C6");
        for c in [[0, 0, 0], [255, 255, 255], [40, 42, 54], [1, 2, 3]] {
            assert_eq!(parse_hex(&hex_of(c)), Some(c));
        }
    }

    #[test]
    fn channels_are_strict() {
        assert_eq!(parse_channel("0"), Some(0));
        assert_eq!(parse_channel("255"), Some(255));
        assert_eq!(parse_channel(" 12 "), Some(12));
        assert_eq!(parse_channel("256"), None);
        assert_eq!(parse_channel("-1"), None);
        assert_eq!(parse_channel("+1"), None);
        assert_eq!(parse_channel("x"), None);
        assert_eq!(parse_channel(""), None);
    }

    #[test]
    fn hsva_keeps_rgb_at_zero_alpha() {
        assert_eq!(
            hsva_of([40, 42, 54, 0]).to_srgba_unmultiplied(),
            [40, 42, 54, 0]
        );
        assert_eq!(
            hsva_of([0, 0, 0, 255]).to_srgba_unmultiplied(),
            [0, 0, 0, 255]
        );
        assert_eq!(
            hsva_of([255, 121, 198, 128]).to_srgba_unmultiplied(),
            [255, 121, 198, 128]
        );
    }

    #[test]
    fn square_and_bar_map_and_clamp() {
        let r = Rect::from_min_size(pos2(10.0, 20.0), vec2(268.0, 150.0));
        assert_eq!(sv_at(r.min, r), (0.0, 1.0));
        assert_eq!(sv_at(r.max, r), (1.0, 0.0));
        assert_eq!(sv_at(pos2(-50.0, 999.0), r), (0.0, 0.0));
        assert_eq!(sv_at(pos2(999.0, -50.0), r), (1.0, 1.0));
        let bar = Rect::from_min_size(pos2(10.0, 0.0), vec2(268.0, 16.0));
        assert_eq!(hue_at(10.0, bar), 0.0);
        assert_eq!(hue_at(278.0, bar), 1.0);
        assert_eq!(hue_at(-5.0, bar), 0.0);
        assert_eq!(hue_at(900.0, bar), 1.0);
    }
}
