//! The PoE2 Options-menu palette (D-24) and display font for the overlay chrome.
//!
//! **D-25: this stays separate from the Dracula palette generated filter output uses** (pinned
//! in `config.json` `styles`). Do not "harmonize" the two — they have different jobs. Every hex
//! value below is doc-quoted against exactly where D-24 sampled it from the reference
//! screenshot (`reference/poe2-options-palette.png`), and asserted against the literal hex in
//! `<behavior>`'s tests so a future restyle cannot silently drift from the reference.
//!
//! [`poe2_visuals`] and the constants are pure — no window is needed to test them
//! (`PITFALLS.md` §8). [`install_fonts`], [`section_header`], and [`row_label`] are the thin
//! `egui`-typed sliver of this module; they are exercised by Task 3's manual look-at-it
//! verification, not by unit tests, since font rasterization only shows up visually
//! (`PITFALLS.md` Pitfall 6's own warning sign note).

use egui::text::LayoutJob;
use egui::{Color32, FontFamily, FontId, TextFormat};

/// Row ground (base) — settings row background. Sampled from the reference screenshot (D-24).
pub const ROW_BASE: Color32 = Color32::from_rgb(0x0D, 0x0D, 0x0D);
/// Row ground (alternate) — the adjacent zebra row. Sampled from the reference screenshot (D-24).
pub const ROW_ALT: Color32 = Color32::from_rgb(0x12, 0x12, 0x12);
/// Panel ground — panel body between sections. Sampled from the reference screenshot (D-24).
pub const PANEL_GROUND: Color32 = Color32::from_rgb(0x14, 0x14, 0x14);
/// Input / dropdown fill — the Language dropdown; recesses darker than a row. D-24.
pub const INPUT_FILL: Color32 = Color32::from_rgb(0x09, 0x09, 0x09);
/// Header plate — the ornate "Options" title plate. D-24.
pub const HEADER_PLATE: Color32 = Color32::from_rgb(0x3B, 0x33, 0x26);
/// Label text (warm tan) — "Language", "Auto-equip" row labels. D-24.
pub const LABEL_TAN: Color32 = Color32::from_rgb(0xE3, 0xCD, 0x97);
/// Section header (cream) — "LANGUAGE", "GUIDANCE", "GENERAL". D-24.
pub const SECTION_CREAM: Color32 = Color32::from_rgb(0xF1, 0xE4, 0xC2);
/// Value text — dropdown values. D-24.
pub const VALUE_TEXT: Color32 = Color32::from_rgb(0xFE, 0xFE, 0xFE);
/// Control glyph (check) — the checked-toggle glyph. D-24.
pub const CONTROL_GLYPH: Color32 = Color32::from_rgb(0xC4, 0xC4, 0xA8);
/// Button text — SAVE / DEFAULTS / CLOSE. D-24.
pub const BUTTON_TEXT: Color32 = Color32::from_rgb(0xDE, 0xDE, 0xDE);
/// Tab active — the "GAME" tab. D-24.
pub const TAB_ACTIVE: Color32 = Color32::from_rgb(0xB3, 0xAF, 0x93);
/// Tab inactive / disabled — "GRAPHICS", "UI" tabs. D-24. Doubles as the color a disabled rule
/// row renders in (Task 3), since both are "present but not active" states.
pub const DISABLED_TEXT: Color32 = Color32::from_rgb(0x88, 0x85, 0x6E);
/// Warm accent / destructive — the red-bronze "USE DEFAULT" button. D-24.
///
/// **Reserved for Phase 14's Reset control**, matching the game's own "use default" action —
/// nothing in this phase spends it (enforced by an acceptance-criteria grep restricted to
/// `theme.rs` itself).
pub const WARM_ACCENT: Color32 = Color32::from_rgb(0xA3, 0x6C, 0x3C);

/// The Cinzel display face's family name, as registered by [`install_fonts`].
pub const DISPLAY_FONT: &str = "cinzel";

/// Builds the overlay's `egui::Visuals`: starts from `Visuals::dark()` (near-black grounds
/// already point the right direction) and layers the D-24 palette on top, plus muted widget
/// strokes so nothing renders in egui's default blue (RESEARCH § "Code Examples" — "Visuals
/// mapping").
pub fn poe2_visuals() -> egui::Visuals {
    let mut v = egui::Visuals::dark();

    v.panel_fill = PANEL_GROUND;
    v.extreme_bg_color = INPUT_FILL;
    v.window_fill = ROW_BASE;

    v.widgets.noninteractive.bg_fill = ROW_BASE;
    v.widgets.noninteractive.fg_stroke.color = LABEL_TAN;
    v.widgets.inactive.bg_fill = ROW_BASE;
    v.widgets.inactive.fg_stroke.color = LABEL_TAN;
    v.widgets.hovered.fg_stroke.color = SECTION_CREAM;
    v.widgets.active.fg_stroke.color = CONTROL_GLYPH;

    v.hyperlink_color = WARM_ACCENT;

    v
}

/// Registers the Cinzel display face (D-24's small-caps-adjacent serif) under
/// `FontFamily::Name("cinzel")`. Call once from the eframe creation closure, before the first
/// frame paints. Cinzel is all-caps-only, which is exactly why no true small-caps rendering is
/// attempted here — `PITFALLS.md` Pitfall 6: egui's `ab_glyph` rasterizer has no OpenType
/// feature support, so `smcp` would be silently ignored on a face that *did* have lowercase
/// glyphs. Cinzel sidesteps the whole problem by not having any.
pub fn install_fonts(ctx: &egui::Context) {
    let mut fonts = egui::FontDefinitions::default();
    fonts.font_data.insert(
        DISPLAY_FONT.to_owned(),
        egui::FontData::from_static(include_bytes!("../../assets/Cinzel-Regular.ttf")).into(),
    );
    // Seed the display family with the default proportional chain BEFORE inserting Cinzel at
    // the front, so any glyph Cinzel lacks falls through instead of rendering as a tofu box.
    // `or_default()` creates an EMPTY family for a new name — the first cut relied on that and
    // left Cinzel with no fallback at all, which is why an arrow in the Apply confirmation came
    // out as a box (screenshot poe2_print005). Cinzel is an all-caps display face with a narrow
    // repertoire, so it will not be the last word on punctuation or symbols.
    let default_chain = fonts
        .families
        .get(&FontFamily::Proportional)
        .cloned()
        .unwrap_or_default();
    let family = fonts
        .families
        .entry(FontFamily::Name(DISPLAY_FONT.into()))
        .or_default();
    family.extend(default_chain);
    family.insert(0, DISPLAY_FONT.to_owned());
    ctx.set_fonts(fonts);
}

/// A section label (e.g. `"RULES"`) in the brighter, larger cream style D-24 uses for section
/// headers, with letter-spacing standing in for true small caps (Pitfall 6).
pub fn section_header(text: &str) -> LayoutJob {
    let mut job = LayoutJob::default();
    job.append(
        text,
        0.0,
        TextFormat {
            font_id: FontId::new(18.0, FontFamily::Name(DISPLAY_FONT.into())),
            color: SECTION_CREAM,
            extra_letter_spacing: 1.5,
            ..Default::default()
        },
    );
    job
}

/// A row label in the smaller, tan style D-24 uses for individual rule rows — one step smaller
/// than [`section_header`], establishing the size hierarchy the game's own UI uses in place of
/// true small caps.
pub fn row_label(text: &str) -> LayoutJob {
    let mut job = LayoutJob::default();
    job.append(
        text,
        0.0,
        TextFormat {
            font_id: FontId::new(14.0, FontFamily::Name(DISPLAY_FONT.into())),
            color: LABEL_TAN,
            extra_letter_spacing: 0.5,
            ..Default::default()
        },
    );
    job
}

/// The condition summary shown under a collapsed rule row.
///
/// Deliberately **not** the display font: a summary is mixed-case content
/// (`"Rings, Amulets, Belts · Unidentified Tier ≥ 4"`), and Cinzel is an all-caps display face,
/// so this stays in the default proportional family where lowercase is legible.
///
/// Sized one step under [`row_label`] rather than `.small()`, and in the muted-active tone
/// rather than [`DISABLED_TEXT`]. The first cut used both, which made the line that actually
/// identifies a rule the hardest thing in the stripe to read — and painted live content in the
/// palette's *disabled* colour (author, 2026-09-07).
pub fn row_summary(text: &str) -> egui::RichText {
    egui::RichText::new(text)
        .size(ROW_SUMMARY_SIZE)
        .color(TAB_ACTIVE)
}

/// Summary text size — below [`row_label`]'s 14.0 so the hierarchy still reads, but far above
/// egui's `.small()`.
pub const ROW_SUMMARY_SIZE: f32 = 13.0;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_row_summary_stays_readable_and_subordinate() {
        // It is the text that identifies a rule at a glance, so it must not drift back toward
        // `.small()`; it is also not a heading, so it must stay under `row_label`'s 14.0.
        assert!(ROW_SUMMARY_SIZE >= 12.0, "too small to scan");
        assert!(ROW_SUMMARY_SIZE < 14.0, "must stay below a row label");
    }

    #[test]
    fn palette_constants_match_the_d24_hex_table() {
        assert_eq!(PANEL_GROUND, Color32::from_rgb(0x14, 0x14, 0x14));
        assert_eq!(ROW_BASE, Color32::from_rgb(0x0D, 0x0D, 0x0D));
        assert_eq!(ROW_ALT, Color32::from_rgb(0x12, 0x12, 0x12));
        assert_eq!(INPUT_FILL, Color32::from_rgb(0x09, 0x09, 0x09));
        assert_eq!(HEADER_PLATE, Color32::from_rgb(0x3B, 0x33, 0x26));
        assert_eq!(LABEL_TAN, Color32::from_rgb(0xE3, 0xCD, 0x97));
        assert_eq!(SECTION_CREAM, Color32::from_rgb(0xF1, 0xE4, 0xC2));
        assert_eq!(VALUE_TEXT, Color32::from_rgb(0xFE, 0xFE, 0xFE));
        assert_eq!(WARM_ACCENT, Color32::from_rgb(0xA3, 0x6C, 0x3C));
        assert_eq!(DISABLED_TEXT, Color32::from_rgb(0x88, 0x85, 0x6E));
    }

    #[test]
    fn poe2_visuals_wires_the_palette_into_the_expected_fields() {
        let v = poe2_visuals();
        assert_eq!(v.panel_fill, PANEL_GROUND);
        assert_eq!(v.extreme_bg_color, INPUT_FILL);
        assert_eq!(v.window_fill, ROW_BASE);
        assert_eq!(v.widgets.inactive.fg_stroke.color, LABEL_TAN);
        assert_eq!(v.hyperlink_color, WARM_ACCENT);
    }

    #[test]
    fn poe2_visuals_is_dark_mode_derived() {
        assert!(
            poe2_visuals().dark_mode,
            "poe2_visuals must start from Visuals::dark()"
        );
    }
}
