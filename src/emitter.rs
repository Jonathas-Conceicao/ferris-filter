/// Filter emitter — pure function: `generate(&Config, &PricedData) -> String`.
///
/// All block emission uses a single canonical action order (D-03) via `emit_style_actions`.
/// No HashMap iteration; all collection traversal is over Vecs or named struct fields (GEN-01).
/// Output is LF-only (`writeln!` produces `\n` on all platforms — never `\r\n`).
use std::fmt::Write;

use crate::models::config::{Config, FilterColor, MinimapShape, Style};
use crate::pricer::{CategoryBlock, PricedData, UniqueBlock};

// ---------------------------------------------------------------------------
// Public entry point
// ---------------------------------------------------------------------------

/// Generate the complete filter text from `config` and `priced`.
///
/// Returns a LF-only `String` ending with exactly one trailing newline.
/// Pure function — no I/O, no side effects.
pub fn generate(config: &Config, priced: &PricedData) -> String {
    let mut out = String::with_capacity(32_000);
    emit_header(&mut out);
    emit_recycle(&mut out, config);
    emit_gear_bands(&mut out, config);
    emit_flask_charm_bands(&mut out, config);
    emit_currency_sections(&mut out, config, &priced.category_blocks);
    emit_unique_sections(&mut out, config, &priced.unique_blocks);
    debug_assert!(out.ends_with('\n'), "generate() output must end with exactly one newline");
    out
}

// ---------------------------------------------------------------------------
// FilterColor / MinimapShape display helpers
// ---------------------------------------------------------------------------

impl FilterColor {
    pub fn as_filter_str(&self) -> &'static str {
        match self {
            FilterColor::Red => "Red",
            FilterColor::Green => "Green",
            FilterColor::Blue => "Blue",
            FilterColor::Brown => "Brown",
            FilterColor::White => "White",
            FilterColor::Yellow => "Yellow",
            FilterColor::Cyan => "Cyan",
            FilterColor::Grey => "Grey",
            FilterColor::Orange => "Orange",
            FilterColor::Pink => "Pink",
            FilterColor::Purple => "Purple",
        }
    }
}

impl MinimapShape {
    pub fn as_filter_str(&self) -> &'static str {
        match self {
            MinimapShape::Circle => "Circle",
            MinimapShape::Diamond => "Diamond",
            MinimapShape::Hexagon => "Hexagon",
            MinimapShape::Square => "Square",
            MinimapShape::Star => "Star",
            MinimapShape::Triangle => "Triangle",
            MinimapShape::Cross => "Cross",
            MinimapShape::Moon => "Moon",
            MinimapShape::Raindrop => "Raindrop",
            MinimapShape::Kite => "Kite",
            MinimapShape::Pentagon => "Pentagon",
            MinimapShape::UpsideDownHouse => "UpsideDownHouse",
        }
    }
}

// ---------------------------------------------------------------------------
// Canonical action emitter (D-03)
//
// Order (every block goes through this; None/false fields are skipped):
//   1. PlayEffect {color} [Temp]        — if beam present
//   2. MinimapIcon {size} {color} {shape} — if minimap present
//   3. SetTextColor R G B 255           — if text present
//   4. SetBackgroundColor R G B 255     — if background present
//   5. SetBorderColor R G B 255         — if border present
//   6. SetFontSize N                    — if font present
//   7. CustomAlertSound / PlayAlertSound — if sound present
//   8. DisableDropSound                 — if disable_drop_sound == true
// ---------------------------------------------------------------------------

fn emit_style_actions(out: &mut String, style: &Style) {
    // 1. Beam (PlayEffect)
    if let Some(beam) = &style.beam {
        let temp = if beam.temp { " Temp" } else { "" };
        writeln!(out, "  PlayEffect {}{}", beam.color.as_filter_str(), temp).unwrap();
    }
    // 2. Minimap icon
    if let Some(mm) = &style.minimap {
        writeln!(
            out,
            "  MinimapIcon {} {} {}",
            mm.size,
            mm.color.as_filter_str(),
            mm.shape.as_filter_str()
        )
        .unwrap();
    }
    // 3. Text color
    if let Some(rgb) = style.text {
        writeln!(out, "  SetTextColor {} {} {} 255", rgb[0], rgb[1], rgb[2]).unwrap();
    }
    // 4. Background color
    if let Some(rgb) = style.background {
        writeln!(out, "  SetBackgroundColor {} {} {} 255", rgb[0], rgb[1], rgb[2]).unwrap();
    }
    // 5. Border color
    if let Some(rgb) = style.border {
        writeln!(out, "  SetBorderColor {} {} {} 255", rgb[0], rgb[1], rgb[2]).unwrap();
    }
    // 6. Font size
    if let Some(font) = style.font {
        writeln!(out, "  SetFontSize {}", font).unwrap();
    }
    // 7. Sound
    if let Some(sound) = &style.sound {
        if let Some(custom) = &sound.custom {
            writeln!(out, "  CustomAlertSound \"{}\" {}", custom, sound.vol).unwrap();
        } else if let Some(id) = sound.id {
            writeln!(out, "  PlayAlertSound {} {}", id, sound.vol).unwrap();
        }
    }
    // 8. DisableDropSound
    if style.disable_drop_sound {
        writeln!(out, "  DisableDropSound").unwrap();
    }
}

// ---------------------------------------------------------------------------
// Style-by-tier resolver
// ---------------------------------------------------------------------------

/// Resolve the `Style` for a tier letter from the config's `Styles`.
///
/// Panics with a descriptive message on unknown tier letters (invariant: pricer only
/// produces S/A/B/C/D/E/F).
fn style_for_tier<'a>(styles: &'a crate::models::config::Styles, tier: &str) -> &'a Style {
    match tier {
        "S" => &styles.s,
        "A" => &styles.a,
        "B" => &styles.b,
        "C" => &styles.c,
        "D" => &styles.d,
        "E" => &styles.e,
        "F" => &styles.f,
        other => panic!("Unknown tier '{}' in priced data", other),
    }
}

// ---------------------------------------------------------------------------
// Section 1: File header comment
// ---------------------------------------------------------------------------

fn emit_header(out: &mut String) {
    writeln!(out, "# universal.filter").unwrap();
    writeln!(out, "# GENERATED FROM config.json — edit the model, not this file (then regenerate).").unwrap();
    writeln!(out, "#").unwrap();
    writeln!(out, "# Block tags link each rule back to the model:").unwrap();
    writeln!(out, "#   [alwaysShow.K]          -> a level-independent highlight").unwrap();
    writeln!(out, "#   [areaLevelBands \"R\".K]  -> band R, show-key K").unwrap();
    writeln!(out, "#   [styles.X]              -> the style entry applied").unwrap();
    writeln!(out, "#").unwrap();
    writeln!(out, "# Philosophy: HIDE what we don't want, STYLE the few highlights, let everything").unwrap();
    writeln!(out, "# else fall through to the game default (uniques, jewels, gems, jewellery, quest").unwrap();
    writeln!(out, "# items, waystones, trials, unidentified — no rules touch them).").unwrap();
    writeln!(out, "# Currency base lists are curated data kept outside the model (carried over here).").unwrap();
    writeln!(out).unwrap();
}

// ---------------------------------------------------------------------------
// Section 2: Recycle block
// ---------------------------------------------------------------------------

fn emit_recycle(out: &mut String, config: &Config) {
    writeln!(out, "#============================================================").unwrap();
    writeln!(out, "# alwaysShow.recycle  ->  styles.Recycle").unwrap();
    writeln!(out, "#============================================================").unwrap();
    writeln!(out).unwrap();
    writeln!(
        out,
        "Show # recycle: sockets + quality salvage  [alwaysShow.recycle -> styles.Recycle]"
    )
    .unwrap();
    writeln!(out, "  Sockets >= 1").unwrap();
    writeln!(out, "  Quality >= 1").unwrap();
    emit_style_actions(out, &config.styles.recycle);
    writeln!(out).unwrap();
}

// ---------------------------------------------------------------------------
// Section 3: Gear AreaLevel bands (8 blocks: 7 Show + 1 Hide)
// ---------------------------------------------------------------------------

fn emit_gear_bands(out: &mut String, config: &Config) {
    writeln!(out, "#============================================================").unwrap();
    writeln!(out, "# areaLevelBands — gear").unwrap();
    writeln!(out, "# Shown gear = styles.unchanged (Show, no restyle, protects it from the").unwrap();
    writeln!(out, "# trailing Hide). The Hide fallback = styles.hidden. Uniques excluded from").unwrap();
    writeln!(out, "# the fallback so they fall through.").unwrap();
    writeln!(out, "#============================================================").unwrap();
    writeln!(out).unwrap();

    // Build class lists as space-separated quoted strings
    let armour_classes_str = config
        .armour_classes
        .iter()
        .map(|s| format!("\"{}\"", s))
        .collect::<Vec<_>>()
        .join(" ");

    let build_armour_str = config
        .build
        .armour
        .iter()
        .map(|s| format!("\"{}\"", s))
        .collect::<Vec<_>>()
        .join(" ");

    let build_weapons_str = config
        .build
        .weapons
        .iter()
        .map(|s| format!("\"{}\"", s))
        .collect::<Vec<_>>()
        .join(" ");

    // otherGear = weapon_classes + armour_classes combined
    let other_gear_classes: Vec<String> = config
        .weapon_classes
        .iter()
        .chain(config.armour_classes.iter())
        .map(|s| format!("\"{}\"", s))
        .collect();
    let other_gear_str = other_gear_classes.join(" ");

    // --- Block 1: band <=5 all armour any rarity ---
    writeln!(out, "# [areaLevelBands \"1-5\".allArmour] all armour, any rarity").unwrap();
    writeln!(out, "# (build weapons come from the weapon rules below; rares of any class from the <=45 otherGear rule)").unwrap();
    writeln!(
        out,
        "Show # band <=5: all armour any rarity  [areaLevelBands \"1-5\".allArmour -> styles.unchanged]"
    )
    .unwrap();
    writeln!(out, "  AreaLevel <= 5").unwrap();
    writeln!(out, "  Class {}", armour_classes_str).unwrap();
    emit_style_actions(out, &config.styles.unchanged);
    writeln!(out).unwrap();

    // --- Block 2: band 6-25 build weapons any rarity ---
    writeln!(out, "# [areaLevelBands \"6-25\".weapons] build weapons, any rarity").unwrap();
    writeln!(
        out,
        "Show # band 6-25: build weapons any rarity  [build.weapons -> styles.unchanged]"
    )
    .unwrap();
    writeln!(out, "  AreaLevel <= 25").unwrap();
    writeln!(out, "  Class {}", build_weapons_str).unwrap();
    emit_style_actions(out, &config.styles.unchanged);
    writeln!(out).unwrap();

    // --- Block 3: bands 26+ build weapons Rare only ---
    writeln!(out, "# [areaLevelBands 26+ .weapons \"Rare only\"] build weapons, Rare, any level").unwrap();
    writeln!(
        out,
        "Show # bands 26+: build weapons Rare only  [build.weapons -> styles.unchanged]"
    )
    .unwrap();
    writeln!(out, "  Class {}", build_weapons_str).unwrap();
    writeln!(out, "  Rarity == Rare").unwrap();
    emit_style_actions(out, &config.styles.unchanged);
    writeln!(out).unwrap();

    // --- Block 4: band 6-25 build armour anyNonEvasion Magic+Rare ---
    let any_non_evasion = config
        .build
        .armour_types
        .get("anyNonEvasion")
        .expect("config.build.armourTypes must contain 'anyNonEvasion'");
    writeln!(out, "# [areaLevelBands \"6-25\".armour anyNonEvasion, Magic+Rare]").unwrap();
    writeln!(
        out,
        "Show # band 6-25: build armour (non-evasion)  [build.armour/anyNonEvasion -> styles.unchanged]"
    )
    .unwrap();
    writeln!(out, "  AreaLevel <= 25").unwrap();
    writeln!(out, "  Class {}", build_armour_str).unwrap();
    for cond in any_non_evasion {
        writeln!(out, "  {}", cond).unwrap();
    }
    writeln!(out, "  Rarity >= Magic").unwrap();
    emit_style_actions(out, &config.styles.unchanged);
    writeln!(out).unwrap();

    // --- Block 5: band 26-45 build armour esHybrid Magic+Rare ---
    let es_hybrid = config
        .build
        .armour_types
        .get("esHybrid")
        .expect("config.build.armourTypes must contain 'esHybrid'");
    writeln!(out, "# [areaLevelBands \"26-45\".armour esHybrid, Magic+Rare]").unwrap();
    writeln!(
        out,
        "Show # band 26-45: build armour (ES / hybrid)  [build.armour/esHybrid -> styles.unchanged]"
    )
    .unwrap();
    writeln!(out, "  AreaLevel >= 26").unwrap();
    writeln!(out, "  AreaLevel <= 45").unwrap();
    writeln!(out, "  Class {}", build_armour_str).unwrap();
    for cond in es_hybrid {
        writeln!(out, "  {}", cond).unwrap();
    }
    writeln!(out, "  Rarity >= Magic").unwrap();
    emit_style_actions(out, &config.styles.unchanged);
    writeln!(out).unwrap();

    // --- Block 6: bands 46+ build armour pureES Magic+Rare ---
    let pure_es = config
        .build
        .armour_types
        .get("pureES")
        .expect("config.build.armourTypes must contain 'pureES'");
    writeln!(out, "# [areaLevelBands \"46-65\"+\"66+\".armour pureES, Magic+Rare]").unwrap();
    writeln!(
        out,
        "Show # bands 46+: build armour (pure ES)  [build.armour/pureES -> styles.unchanged]"
    )
    .unwrap();
    writeln!(out, "  AreaLevel >= 46").unwrap();
    writeln!(out, "  Class {}", build_armour_str).unwrap();
    for cond in pure_es {
        writeln!(out, "  {}", cond).unwrap();
    }
    writeln!(out, "  Rarity >= Magic").unwrap();
    emit_style_actions(out, &config.styles.unchanged);
    writeln!(out).unwrap();

    // --- Block 7: bands <=45 any other Rare gear ---
    writeln!(out, "# [areaLevelBands \"6-25\"+\"26-45\".otherGear \"Rare\"] any other equipment, Rare, <=45").unwrap();
    writeln!(
        out,
        "Show # bands <=45: any other Rare gear  [areaLevelBands.otherGear -> styles.unchanged]"
    )
    .unwrap();
    writeln!(out, "  AreaLevel <= 45").unwrap();
    writeln!(out, "  Class {}", other_gear_str).unwrap();
    writeln!(out, "  Rarity == Rare").unwrap();
    emit_style_actions(out, &config.styles.unchanged);
    writeln!(out).unwrap();

    // --- Block 8: gear fallback Hide ---
    writeln!(out, "# [fallthrough guard] hide remaining equipment (uniques excluded -> fall through)").unwrap();
    writeln!(
        out,
        "Hide # gear fallback: hide unmatched equipment  [styles.hidden]"
    )
    .unwrap();
    writeln!(out, "  Class {}", other_gear_str).unwrap();
    writeln!(out, "  Rarity <= Rare").unwrap();
    emit_style_actions(out, &config.styles.hidden);
    writeln!(out).unwrap();
}

// ---------------------------------------------------------------------------
// Section 4: Flask/charm AreaLevel bands (4 blocks: 3 Show + 1 Hide)
// ---------------------------------------------------------------------------

fn emit_flask_charm_bands(out: &mut String, config: &Config) {
    writeln!(out, "#============================================================").unwrap();
    writeln!(out, "# areaLevelBands — flasks & charms  [flaskCharmClasses]").unwrap();
    writeln!(out, "#============================================================").unwrap();
    writeln!(out).unwrap();

    let flask_classes_str = config
        .flask_charm_classes
        .iter()
        .map(|s| format!("\"{}\"", s))
        .collect::<Vec<_>>()
        .join(" ");

    // --- Block 1: quality always shown ---
    writeln!(out, "# quality always shown (this is the \">65 Quality only\" tier)").unwrap();
    writeln!(
        out,
        "Show # flasks/charms: quality always  [areaLevelBands.flasksCharms -> styles.unchanged]"
    )
    .unwrap();
    writeln!(out, "  Quality >= 1").unwrap();
    writeln!(out, "  Class {}", flask_classes_str).unwrap();
    emit_style_actions(out, &config.styles.unchanged);
    writeln!(out).unwrap();

    // --- Block 2: bands <=25 all flasks/charms ---
    writeln!(out, "# [areaLevelBands \"1-5\"+\"6-25\".flasksCharms \"any rarity\"]").unwrap();
    writeln!(out, "Show # bands <=25: all flasks/charms  [styles.unchanged]").unwrap();
    writeln!(out, "  AreaLevel <= 25").unwrap();
    writeln!(out, "  Class {}", flask_classes_str).unwrap();
    emit_style_actions(out, &config.styles.unchanged);
    writeln!(out).unwrap();

    // --- Block 3: bands 26-65 magic flasks/charms ---
    writeln!(out, "# [areaLevelBands \"26-45\"+\"46-65\".flasksCharms \"Magic\"]").unwrap();
    writeln!(out, "Show # bands 26-65: magic flasks/charms  [styles.unchanged]").unwrap();
    writeln!(out, "  AreaLevel <= 65").unwrap();
    writeln!(out, "  Class {}", flask_classes_str).unwrap();
    writeln!(out, "  Rarity == Magic").unwrap();
    emit_style_actions(out, &config.styles.unchanged);
    writeln!(out).unwrap();

    // --- Block 4: flasks/charms fallback Hide ---
    writeln!(out, "# remaining flasks/charms hidden").unwrap();
    writeln!(out, "Hide # flasks/charms fallback  [styles.hidden]").unwrap();
    writeln!(out, "  Class {}", flask_classes_str).unwrap();
    emit_style_actions(out, &config.styles.hidden);
    writeln!(out).unwrap();
}

// ---------------------------------------------------------------------------
// Section 5: Currency / category tier blocks (from PricedData.category_blocks)
// ---------------------------------------------------------------------------

fn emit_currency_sections(out: &mut String, config: &Config, blocks: &[CategoryBlock]) {
    writeln!(out, "#============================================================").unwrap();
    writeln!(out, "# alwaysShow.currency — tiered S-F  [styles.S .. styles.F]").unwrap();
    writeln!(out, "# Each block's tier letter = the styles.<tier> applied. Base lists are").unwrap();
    writeln!(out, "# curated data carried over from the prior filter (not stored in the model).").unwrap();
    writeln!(out, "# Gems are intentionally omitted (fall through to game default).").unwrap();
    writeln!(out, "#============================================================").unwrap();
    writeln!(out).unwrap();

    for category in blocks {
        writeln!(out, "# --- {} ---", category.name).unwrap();

        for tier_group in &category.tiers {
            let label = derive_block_label(&category.name, &tier_group.tier, &tier_group.class);
            let style = style_for_tier(&config.styles, &tier_group.tier);

            // Build BaseType list: space-separated quoted base names
            let base_type_str = tier_group
                .items
                .iter()
                .map(|i| format!("\"{}\"", i.base_type))
                .collect::<Vec<_>>()
                .join(" ");

            writeln!(out, "Show # {}", label).unwrap();
            writeln!(out, "  Class == \"{}\"", tier_group.class).unwrap();
            writeln!(out, "  BaseType == {}", base_type_str).unwrap();
            emit_style_actions(out, style);
            writeln!(out).unwrap();
        }
    }

    // Remove the final trailing blank line — universal.filter ends immediately after the
    // last block's final action line (no trailing blank line per RESEARCH Pitfall 6).
    // The writeln!(out) above the loop's end adds one extra blank line after the last entry.
    // Trim it: pop the trailing '\n' from the blank line (but keep the block's own trailing '\n').
    // Strategy: after the loop, the output ends with "...\n\n"; trim the last '\n'.
    if out.ends_with("\n\n") {
        out.pop();
    }
}

// ---------------------------------------------------------------------------
// Section 6: Unique blocks (from PricedData.unique_blocks)
//
// Emits a dedicated section only when unique_blocks is non-empty.
// With the committed seed (zero uniques), this function emits nothing.
// ---------------------------------------------------------------------------

fn emit_unique_sections(out: &mut String, config: &Config, blocks: &[UniqueBlock]) {
    // Per Open Questions resolution: omit the header entirely when unique_blocks is empty.
    if blocks.is_empty() {
        return;
    }

    // If currency sections already trimmed the trailing blank, we need a blank separator.
    // At this point out ends with exactly one '\n' (trimmed by emit_currency_sections).
    // Add a blank line before the unique section header.
    writeln!(out).unwrap();
    writeln!(out, "# --- Uniques (price-tiered) ---").unwrap();

    for block in blocks {
        let style = style_for_tier(&config.styles, &block.tier);

        let base_type_str = block
            .base_types
            .iter()
            .map(|b| format!("\"{}\"", b))
            .collect::<Vec<_>>()
            .join(" ");

        writeln!(out, "Show # Unique {}", block.tier).unwrap();
        writeln!(out, "  Rarity == Unique").unwrap();
        writeln!(out, "  BaseType == {}", base_type_str).unwrap();
        // Price-tiered uniques carry a distinct background (bronze, 175 96 37 240) on top
        // of their tier text/border/beam so they read as "valuable unique" regardless of
        // tier color. Suppress the tier's own (Dracula) background and emit this instead.
        let mut unique_style = style.clone();
        unique_style.background = None;
        emit_style_actions(out, &unique_style);
        writeln!(out, "  SetBackgroundColor 175 96 37 240").unwrap();
        writeln!(out).unwrap();
    }

    // Trim the final trailing blank line (same pattern as emit_currency_sections).
    if out.ends_with("\n\n") {
        out.pop();
    }
}

/// Derive the block comment label from category, tier, and class.
///
/// The universal.filter uses these specific labels (verified line-by-line):
///  - "Expedition" category, tier B, class "Expedition Logbooks" -> "Expedition Logbooks B"
///  - "Map Fragments & Pinnacle Keys", tier S, class "Pinnacle Keys" -> "Pinnacle Keys S"
///  - "Map Fragments & Pinnacle Keys", tier A, class "Map Fragments"  -> "Map Fragments A"
///  - "Map Fragments & Pinnacle Keys", tier A, class "Pinnacle Keys"  -> "Pinnacle Keys A"
///  - "Map Fragments & Pinnacle Keys", tier C, class "Map Fragments"  -> "Map Fragments C"
///  - "Map Fragments & Pinnacle Keys", tier E, class "Trial Coins"    -> "Trial Coins E"
///  - "Map Fragments & Pinnacle Keys", tier E, class "Inscribed Ultimatum" -> "Inscribed Ultimatum E"
///  - "Omens" category, tier E, class "Stackable Currency"            -> "Omen Splinter E"
///  - "Tablets", tier B -> "Tablet B", tier C -> "Tablet C"
///  - All others: "{category_name} {tier}"
fn derive_block_label(category: &str, tier: &str, class: &str) -> String {
    match (category, tier, class) {
        // Expedition: logbooks use the class name as the label prefix
        ("Expedition", "B", "Expedition Logbooks") => format!("Expedition Logbooks {}", tier),
        // Map Fragments & Pinnacle Keys: label is the class name, not the category name
        ("Map Fragments & Pinnacle Keys", _, "Pinnacle Keys") => format!("Pinnacle Keys {}", tier),
        ("Map Fragments & Pinnacle Keys", _, "Map Fragments") => format!("Map Fragments {}", tier),
        ("Map Fragments & Pinnacle Keys", _, "Trial Coins") => format!("Trial Coins {}", tier),
        ("Map Fragments & Pinnacle Keys", _, "Inscribed Ultimatum") => {
            format!("Inscribed Ultimatum {}", tier)
        }
        // Omens: the Stackable Currency entry is "Omen Splinter E"
        ("Omens", "E", "Stackable Currency") => format!("Omen Splinter {}", tier),
        // Omens: the Omen class entries use "Omen {tier}" (not "Omens {tier}")
        ("Omens", _, "Omen") => format!("Omen {}", tier),
        // Tablets: singular "Tablet" not "Tablets"
        ("Tablets", _, _) => format!("Tablet {}", tier),
        // Default: "{category_name} {tier}"
        _ => format!("{} {}", category, tier),
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::config::Config;
    use crate::models::{prices::PricesFile, tiers::TierEntry};
    use crate::models::tiers::ValidatedTiers;

    fn load_inputs() -> (Config, PricedData) {
        let config: Config = serde_json::from_str(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/config.json"
        )))
        .expect("config.json parse failed");

        let prices_src = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/prices.json"));
        let prices: PricesFile = serde_json::from_str(prices_src).expect("prices.json parse failed");

        let tiers_src = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/tiers.json"));
        let tier_entries: Vec<TierEntry> =
            serde_json::from_str(tiers_src).expect("tiers.json parse failed");
        let tiers = ValidatedTiers::try_from(tier_entries).expect("tiers.json validation failed");

        let rates = crate::pricer::derive_rates(&prices.items).expect("derive_rates failed");
        let priced = crate::pricer::price_tiers(&prices, &tiers, &rates).expect("price_tiers failed");

        (config, priced)
    }

    #[test]
    fn smoke_test_generate_starts_with_header_and_contains_recycle_tag() {
        let (config, priced) = load_inputs();
        let output = generate(&config, &priced);
        assert!(
            output.starts_with("# universal.filter"),
            "output must start with '# universal.filter'"
        );
        assert!(
            output.contains("[alwaysShow.recycle -> styles.Recycle]"),
            "output must contain the recycle model-path tag"
        );
    }

    #[test]
    fn generate_lf_only() {
        let (config, priced) = load_inputs();
        let output = generate(&config, &priced);
        assert!(
            !output.contains("\r\n"),
            "generate() output must not contain CRLF sequences"
        );
    }

    #[test]
    fn generate_ends_with_single_newline() {
        let (config, priced) = load_inputs();
        let output = generate(&config, &priced);
        assert!(
            output.ends_with('\n'),
            "generate() output must end with a newline"
        );
        assert!(
            !output.ends_with("\n\n"),
            "generate() output must not end with a double newline"
        );
    }

    #[test]
    fn generate_block_counts() {
        let (config, priced) = load_inputs();
        let output = generate(&config, &priced);
        let show_count = output.lines().filter(|l| l.starts_with("Show")).count();
        let hide_count = output.lines().filter(|l| l.starts_with("Hide")).count();
        assert_eq!(show_count, 83, "expected 83 Show blocks, got {}", show_count);
        assert_eq!(hide_count, 2, "expected 2 Hide blocks, got {}", hide_count);
    }

    #[test]
    fn generate_no_unique_or_jewels_blocks() {
        let (config, priced) = load_inputs();
        let output = generate(&config, &priced);
        assert!(
            !output.contains("Rarity == Unique"),
            "GEN-04: no Rarity == Unique blocks"
        );
        assert!(
            !output.contains("Class \"Jewels\""),
            "GEN-04: no Class \"Jewels\" blocks"
        );
    }

    /// Positive counterpart: when the priced data DOES contain a unique block (as it
    /// now will once `sync` fetches the stash/item endpoint), the emitter must produce
    /// a `Rarity == Unique` block keyed on the unique's BASE TYPE.
    #[test]
    fn generate_emits_unique_block_when_present() {
        let config: Config = serde_json::from_str(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/config.json"
        )))
        .expect("config.json parse failed");

        let priced = PricedData {
            category_blocks: vec![],
            unique_blocks: vec![crate::pricer::UniqueBlock {
                tier: "A".to_string(),
                base_types: vec!["Silk Robe".to_string()],
            }],
        };

        let output = generate(&config, &priced);
        assert!(
            output.contains("Rarity == Unique"),
            "a priced unique must produce a Rarity == Unique block"
        );
        assert!(
            output.contains("BaseType == \"Silk Robe\""),
            "the unique block must match on the unique's base type"
        );
        assert!(
            output.contains("SetBackgroundColor 175 96 37 240"),
            "price-tiered uniques must carry the distinct bronze background"
        );
    }

    #[test]
    fn generate_is_deterministic() {
        let (config, priced) = load_inputs();
        let first = generate(&config, &priced);
        let second = generate(&config, &priced);
        assert_eq!(first, second, "generate() must be deterministic");
    }
}
