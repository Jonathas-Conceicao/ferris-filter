/// Filter emitter — pure function: `generate(&Config, &PricedData) -> Result<String, GenerateError>`.
///
/// All block emission uses a single canonical action order (D-03) via `emit_style_actions`.
/// No HashMap iteration; all collection traversal is over Vecs or named struct fields (GEN-01).
/// Output is LF-only (`writeln!` produces `\n` on all platforms — never `\r\n`).
use std::fmt::Write;

use miette::Diagnostic;
use thiserror::Error;

use crate::models::config::{
    Config, FilterColor, MinimapShape, RarityOp, Style, StyleAction, Styles,
};
use crate::pricer::{CategoryBlock, PricedData, UniqueBlock};

// ---------------------------------------------------------------------------
// Error type
// ---------------------------------------------------------------------------

#[derive(Error, Debug, Diagnostic)]
pub enum GenerateError {
    #[error("rule {rule_index} (\"{rule_name}\") references unknown style \"{style}\"; \
             valid styles: S, A, B, C, D, E, F, Recycle, unchanged, hidden, unidentified, craftable \
             (craftable must be defined in the config's styles object)")]
    #[diagnostic(code(ferrisfilter::generate::unknown_style))]
    UnknownStyle {
        rule_index: usize,
        rule_name: String,
        style: String,
    },
}

// ---------------------------------------------------------------------------
// Public entry point
// ---------------------------------------------------------------------------

/// Generate the complete filter text from `config` and `priced`.
///
/// Returns a LF-only `String` ending with exactly one trailing newline.
/// Pure function — no I/O, no side effects.
/// Returns `Err(GenerateError::UnknownStyle)` if any rule references an unknown style name.
pub fn generate(config: &Config, priced: &PricedData) -> Result<String, GenerateError> {
    let mut out = String::with_capacity(32_000);
    emit_header(&mut out);
    emit_rules(&mut out, config)?;
    emit_currency_sections(&mut out, config, &priced.category_blocks);
    emit_unique_sections(&mut out, config, &priced.unique_blocks);
    debug_assert!(out.ends_with('\n'), "generate() output must end with exactly one newline");
    Ok(out)
}

// ---------------------------------------------------------------------------
// Style resolver
// ---------------------------------------------------------------------------

/// Resolve a style name to the matching `&Style` in `Styles`.
///
/// Returns `Err(GenerateError::UnknownStyle)` for any name not in the closed set
/// S/A/B/C/D/E/F/Recycle/unchanged/hidden/unidentified/craftable.
///
/// FUTURE (style flexibility): this match hardcodes the style vocabulary, so adding a style
/// means editing this arm and the `Styles` struct. To let any config define and use new
/// styles, back `Styles` with a name→`Style` map and resolve by lookup here instead of a
/// match. See the matching FUTURE note on `models::config::Styles`.
fn resolve_style<'a>(
    styles: &'a Styles,
    style: &str,
    idx: usize,
    name: &str,
) -> Result<&'a Style, GenerateError> {
    Ok(match style {
        "S" => &styles.s,
        "A" => &styles.a,
        "B" => &styles.b,
        "C" => &styles.c,
        "D" => &styles.d,
        "E" => &styles.e,
        "F" => &styles.f,
        "Recycle" => &styles.recycle,
        "unchanged" => &styles.unchanged,
        "hidden" => &styles.hidden,
        "unidentified" => &styles.unidentified,
        // Optional style — present only when the config defines it. A rule referencing
        // "craftable" against a config that omits it is treated as an unknown style.
        "craftable" if styles.craftable.is_some() => styles.craftable.as_ref().unwrap(),
        other => {
            return Err(GenerateError::UnknownStyle {
                rule_index: idx,
                rule_name: name.to_string(),
                style: other.to_string(),
            })
        }
    })
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
// Style-by-tier resolver (used by pricer sections)
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
// Numeric condition helper
// ---------------------------------------------------------------------------

fn emit_numeric(out: &mut String, attr: &str, nc: &crate::models::config::NumericCondition) {
    if let Some(v) = nc.eq {
        writeln!(out, "  {attr} {v}").unwrap();
    }
    if let Some(n) = nc.min {
        let is_defence = matches!(attr, "BaseArmour" | "BaseEvasion" | "BaseEnergyShield");
        if is_defence && n == 1 {
            writeln!(out, "  {attr} > 0").unwrap();
        } else {
            writeln!(out, "  {attr} >= {n}").unwrap();
        }
    }
    if let Some(n) = nc.max {
        writeln!(out, "  {attr} <= {n}").unwrap();
    }
}

// ---------------------------------------------------------------------------
// Section 2: Rule-driven policy blocks
// ---------------------------------------------------------------------------

/// Emit all policy blocks by iterating `config.rules` in authored order.
///
/// Each rule compiles to exactly one Show/Hide block. The style name is resolved
/// via `resolve_style`; an unknown name is a hard error (RULE-03 / T-06-01).
fn emit_rules(out: &mut String, config: &Config) -> Result<(), GenerateError> {
    for (idx, rule) in config
        .rules
        .as_deref()
        .unwrap_or(&[])
        .iter()
        .enumerate()
    {
        let derived_label = format!("rule {}", idx);
        let label: &str = rule.name.as_deref().unwrap_or(&derived_label);
        let style = resolve_style(&config.styles, &rule.style, idx, label)?;

        // Header line: Show or Hide based on style.action
        let block_type = match style.action {
            StyleAction::Show => "Show",
            StyleAction::Hide => "Hide",
        };
        writeln!(out, "{} # {}", block_type, label).unwrap();

        // Emit condition lines from rule.when in fixed deterministic order:
        // areaLevel(min then max), class, baseArmour, baseEvasion, baseEnergyShield,
        // rarity, quality, sockets, baseType, unidentifiedTier
        if let Some(when) = &rule.when {
            // 1. AreaLevel >= min
            if let Some(al) = &when.area_level {
                if let Some(n) = al.min {
                    writeln!(out, "  AreaLevel >= {}", n).unwrap();
                }
                if let Some(m) = al.max {
                    writeln!(out, "  AreaLevel <= {}", m).unwrap();
                }
            }

            // 2. Class (bare, no ==)
            if let Some(classes) = &when.class {
                let class_str = classes
                    .iter()
                    .map(|c| format!("\"{}\"", c))
                    .collect::<Vec<_>>()
                    .join(" ");
                writeln!(out, "  Class {}", class_str).unwrap();
            }

            // 3. BaseArmour
            if let Some(nc) = &when.base_armour {
                emit_numeric(out, "BaseArmour", nc);
            }

            // 4. BaseEvasion
            if let Some(nc) = &when.base_evasion {
                emit_numeric(out, "BaseEvasion", nc);
            }

            // 5. BaseEnergyShield
            if let Some(nc) = &when.base_energy_shield {
                emit_numeric(out, "BaseEnergyShield", nc);
            }

            // 6. Rarity
            if let Some(rc) = &when.rarity {
                let op_str = match rc.op {
                    RarityOp::Eq => "==",
                    RarityOp::Gte => ">=",
                    RarityOp::Lte => "<=",
                };
                let rarity_str = match rc.rarity {
                    crate::models::config::Rarity::Normal => "Normal",
                    crate::models::config::Rarity::Magic => "Magic",
                    crate::models::config::Rarity::Rare => "Rare",
                    crate::models::config::Rarity::Unique => "Unique",
                };
                writeln!(out, "  Rarity {} {}", op_str, rarity_str).unwrap();
            }

            // 7. Quality
            if let Some(nc) = &when.quality {
                emit_numeric(out, "Quality", nc);
            }

            // 8. Sockets
            if let Some(nc) = &when.sockets {
                emit_numeric(out, "Sockets", nc);
            }

            // 9. BaseType == (with ==)
            if let Some(base_types) = &when.base_type {
                let bt_str = base_types
                    .iter()
                    .map(|b| format!("\"{}\"", b))
                    .collect::<Vec<_>>()
                    .join(" ");
                writeln!(out, "  BaseType == {}", bt_str).unwrap();
            }

            // 10. UnidentifiedItemTier
            if let Some(ut) = &when.unidentified_tier {
                writeln!(out, "  UnidentifiedItemTier >= {}", ut.min).unwrap();
            }
        }

        emit_style_actions(out, style);
        writeln!(out).unwrap();
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Section 3: Currency / category tier blocks (from PricedData.category_blocks)
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
            // Multi-class support: use the typed `is_multi_class` signal from the pricer
            // (set for the Uncut Gems merge group) to emit the pre-formatted multi-class
            // string verbatim. Single-class values are wrapped in quotes normally.
            if tier_group.is_multi_class {
                writeln!(out, "  Class == {}", tier_group.class).unwrap();
            } else {
                writeln!(out, "  Class == \"{}\"", tier_group.class).unwrap();
            }
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
// Section 4: Unique blocks (from PricedData.unique_blocks)
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

        let prices_src = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/seed-prices.json"));
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
        let output = generate(&config, &priced).expect("generate failed");
        assert!(
            output.starts_with("# universal.filter"),
            "output must start with '# universal.filter'"
        );
        // When config.rules contains the recycle rule, the block comment includes its name.
        // If rules is populated (Task 3+), the recycle rule name must appear in the output.
        if config.rules.as_ref().map_or(false, |r| !r.is_empty()) {
            assert!(
                output.contains("recycle"),
                "output must contain the recycle rule name when rules are defined"
            );
        }
    }

    #[test]
    fn generate_lf_only() {
        let (config, priced) = load_inputs();
        let output = generate(&config, &priced).expect("generate failed");
        assert!(
            !output.contains("\r\n"),
            "generate() output must not contain CRLF sequences"
        );
    }

    #[test]
    fn generate_ends_with_single_newline() {
        let (config, priced) = load_inputs();
        let output = generate(&config, &priced).expect("generate failed");
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
        let output = generate(&config, &priced).expect("generate failed");
        let show_count = output.lines().filter(|l| l.starts_with("Show")).count();
        let hide_count = output.lines().filter(|l| l.starts_with("Hide")).count();
        // Count updated in 08-02 Task 3: seed-prices.json re-baseline added Verisium (2),
        // Uncut Gems (4), and Lineage Support Gems (2) blocks (+8 total); tiers.json
        // cutoff changes removed some mid-range tiers (net: 84 - 18 + 8 = 74).
        assert_eq!(show_count, 74, "expected 74 Show blocks, got {}", show_count);
        assert_eq!(hide_count, 3, "expected 3 Hide blocks, got {}", hide_count);
    }

    #[test]
    fn generate_rejects_unknown_style_name() {
        // RULE-03 / Phase-5 deferral: a rule referencing a style name outside the
        // closed Styles set must fail generate-time with GenerateError::UnknownStyle
        // naming the offending style.
        let (mut config, priced) = load_inputs();
        config
            .rules
            .get_or_insert_with(Vec::new)
            .push(crate::models::config::Rule {
                name: Some("bad rule".to_string()),
                style: "NotATier".to_string(),
                when: None,
            });
        match generate(&config, &priced) {
            Err(GenerateError::UnknownStyle { style, .. }) => {
                assert_eq!(style, "NotATier", "error must name the offending style");
            }
            Err(other) => panic!("expected UnknownStyle, got {:?}", other),
            Ok(_) => panic!("expected generate to reject the unknown style 'NotATier'"),
        }
    }

    #[test]
    fn generate_no_unique_or_jewels_blocks() {
        let (config, priced) = load_inputs();
        let output = generate(&config, &priced).expect("generate failed");
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

        let output = generate(&config, &priced).expect("generate failed");
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
        let first = generate(&config, &priced).expect("generate failed");
        let second = generate(&config, &priced).expect("generate failed");
        assert_eq!(first, second, "generate() must be deterministic");
    }
}
