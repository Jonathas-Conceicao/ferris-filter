/// Filter emitter — pure function: `generate(&Config, &PricedData) -> Result<String, GenerateError>`.
///
/// All block emission uses a single canonical action order (D-03) via `emit_style_actions`.
/// No HashMap iteration; all collection traversal is over Vecs or named struct fields (GEN-01).
/// Output is LF-only (`writeln!` produces `\n` on all platforms — never `\r\n`).
use std::fmt::Write;

use miette::Diagnostic;
use thiserror::Error;

use crate::models::config::{
    Config, FilterColor, MinimapShape, RarityOp, Style, StyleAction, StyleSet,
};
use crate::pricer::{CategoryBlock, PricedData, UniqueBlock};

// ---------------------------------------------------------------------------
// Error type
// ---------------------------------------------------------------------------

#[derive(Error, Debug, Diagnostic)]
pub enum GenerateError {
    #[error("rule {rule_index} (\"{rule_name}\") references unknown style \"{style}\"; \
             defined styles: {available}")]
    #[diagnostic(code(ferrisfilter::generate::unknown_style))]
    UnknownStyle {
        rule_index: usize,
        rule_name: String,
        style: String,
        /// The names defined in the loaded StyleSet, comma-separated, in file order.
        available: String,
    },
}

// ---------------------------------------------------------------------------
// Public entry point
// ---------------------------------------------------------------------------

/// Which filter is being produced, and from what — the two identifying lines of the header.
///
/// Exists so the dynamic path can say `# dynamic.filter` / `# GENERATED FROM dynamic.json`
/// without changing a byte of the universal path's output. `Default` is exactly the pair the
/// header hardcoded before v0.4.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FilterIdentity<'a> {
    /// Filename this filter is conventionally written as, named on the header's first line.
    pub output_name: &'a str,
    /// Source file it was compiled from, named on the header's second line.
    pub source_name: &'a str,
}

impl Default for FilterIdentity<'_> {
    fn default() -> Self {
        FilterIdentity {
            output_name: "universal.filter",
            source_name: "config.json",
        }
    }
}

/// Generate the universal filter — `config.json` → `universal.filter`.
///
/// A thin delegation to [`generate_as`] with the default identity. Kept as its own function with
/// an unchanged signature so the universal path is not merely *believed* unaffected by the
/// identity parameter: it runs the same code it always did, and `generate_matches_generate_as_default`
/// asserts the two agree.
pub fn generate(config: &Config, priced: &PricedData) -> Result<String, GenerateError> {
    generate_as(config, priced, &FilterIdentity::default())
}

/// Generate the complete filter text from `config` and `priced`, naming the filter and its
/// source in the header.
///
/// Returns a LF-only `String` ending with exactly one trailing newline.
/// Pure function — no I/O, no side effects.
/// Returns `Err(GenerateError::UnknownStyle)` if any *enabled* rule references an unknown style
/// name; disabled rules are skipped before their style is resolved.
pub fn generate_as(
    config: &Config,
    priced: &PricedData,
    identity: &FilterIdentity,
) -> Result<String, GenerateError> {
    let mut out = String::with_capacity(32_000);
    emit_header(&mut out, identity);
    emit_rules(&mut out, config)?;
    emit_currency_sections(&mut out, config, &priced.category_blocks);
    emit_unique_sections(&mut out, config, &priced.unique_blocks);
    debug_assert!(out.ends_with('\n'), "generate() output must end with exactly one newline");
    Ok(out)
}

// ---------------------------------------------------------------------------
// Style resolver
// ---------------------------------------------------------------------------

/// Resolve a style name to the matching `&Style` in the `StyleSet`.
///
/// Returns `Err(GenerateError::UnknownStyle)` (listing the defined names) for any name the set
/// does not contain.
fn resolve_style<'a>(
    styles: &'a StyleSet,
    style: &str,
    idx: usize,
    name: &str,
) -> Result<&'a Style, GenerateError> {
    styles.get(style).ok_or_else(|| GenerateError::UnknownStyle {
        rule_index: idx,
        rule_name: name.to_string(),
        style: style.to_string(),
        available: styles.names().collect::<Vec<_>>().join(", "),
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

/// Resolve the `Style` for a tier letter from the config's `StyleSet`.
///
/// Panics with a descriptive message on unknown tier letters (invariant: pricer only produces
/// S/A/B/C/D/E/F, and `StyleSet::validate` guarantees those are defined).
fn style_for_tier<'a>(styles: &'a StyleSet, tier: &str) -> &'a Style {
    styles
        .get(tier)
        .unwrap_or_else(|| panic!("Unknown tier '{}' in priced data", tier))
}

// ---------------------------------------------------------------------------
// Section 1: File header comment
// ---------------------------------------------------------------------------

fn emit_header(out: &mut String, identity: &FilterIdentity) {
    writeln!(out, "# {}", identity.output_name).unwrap();
    writeln!(
        out,
        "# GENERATED FROM {} — edit the model, not this file (then regenerate).",
        identity.source_name
    )
    .unwrap();
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
        // A disabled rule emits nothing at all — no block, not even a commented one.
        // `continue` here rather than filtering the iterator, deliberately: `enumerate()` must
        // keep walking the whole array so labels stay tied to a rule's index in the source file.
        // Filtering first would renumber every later rule, so disabling rule 2 would silently
        // relabel rule 3's block as `rule 2` and break the trace from a filter block back to its
        // position in the store.
        //
        // Skipping before `resolve_style` also means a disabled rule referencing an unknown style
        // is inert rather than a hard error — a rule the author switched off should not be able to
        // fail generation.
        if rule.enabled == Some(false) {
            continue;
        }

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
        // rarity, quality, sockets, corrupted, baseType, unidentifiedTier
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

            // 8b. Corrupted (boolean — no operator)
            if let Some(c) = when.corrupted {
                writeln!(out, "  Corrupted {}", if c { "True" } else { "False" }).unwrap();
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
            // Per-category background override (Phase 9): mirror the priced-unique bronze
            // pattern — clone the tier style, null its background so emit_style_actions
            // skips the Dracula SetBackgroundColor line, then append the single override.
            match category.background_override {
                Some([r, g, b]) => {
                    let mut bg_style = style.clone();
                    bg_style.background = None;
                    emit_style_actions(out, &bg_style);
                    writeln!(out, "  SetBackgroundColor {} {} {} 255", r, g, b).unwrap();
                }
                None => emit_style_actions(out, style),
            }
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
        let mut config = config;
        config.styles = StyleSet::committed_for_tests();

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
        // Count updated in 10-02 Task 2: Phase 10 added 3 exceptional Show blocks (74 + 3 = 77).
        // Prior baseline: 08-02 added Verisium (2), Uncut Gems (4), Lineage Support Gems (2)
        // blocks (+8 total); tiers.json cutoff changes removed some mid-range tiers (net: 74).
        assert_eq!(show_count, 77, "expected 77 Show blocks, got {}", show_count);
        assert_eq!(hide_count, 3, "expected 3 Hide blocks, got {}", hide_count);
    }

    /// Guard test (EXC-02/03/04, T-10-04): asserts all three exceptional blocks are present,
    /// carry the correct conditions and pink style, and appear BEFORE the recycle block.
    #[test]
    fn exceptional_blocks_present_and_precede_recycle() {
        let (config, priced) = load_inputs();
        let output = generate(&config, &priced).expect("generate failed");

        // All three exceptional block headers must be present
        assert!(
            output.contains("Show # exceptional sockets 2h/body"),
            "missing 'Show # exceptional sockets 2h/body' block"
        );
        assert!(
            output.contains("Show # exceptional sockets 1h"),
            "missing 'Show # exceptional sockets 1h' block"
        );
        assert!(
            output.contains("Show # exceptional quality"),
            "missing 'Show # exceptional quality' block"
        );

        // Each block must carry Corrupted False
        // Extract each block's text by locating the header and scanning until the next blank line.
        let extract_block = |header: &str| -> String {
            let start = output.find(header).expect(&format!("block not found: {header}"));
            let slice = &output[start..];
            let end = slice.find("\n\n").unwrap_or(slice.len());
            slice[..end].to_string()
        };

        let block_2h = extract_block("Show # exceptional sockets 2h/body");
        let block_1h = extract_block("Show # exceptional sockets 1h");
        let block_quality = extract_block("Show # exceptional quality");

        // Sockets thresholds
        assert!(block_2h.contains("Sockets >= 3"), "2h/body block must have Sockets >= 3");
        assert!(block_1h.contains("Sockets >= 2"), "1h block must have Sockets >= 2");
        assert!(block_quality.contains("Quality >= 24"), "quality block must have Quality >= 24");

        // Corrupted False on all three
        assert!(block_2h.contains("Corrupted False"), "2h/body block must have Corrupted False");
        assert!(block_1h.contains("Corrupted False"), "1h block must have Corrupted False");
        assert!(block_quality.contains("Corrupted False"), "quality block must have Corrupted False");

        // Pink exceptional style on all three
        for (label, block) in [
            ("2h/body", &block_2h),
            ("1h", &block_1h),
            ("quality", &block_quality),
        ] {
            assert!(
                block.contains("SetTextColor 255 121 198 255"),
                "exceptional {label} block must have pink SetTextColor"
            );
            assert!(
                block.contains("SetBorderColor 255 121 198 255"),
                "exceptional {label} block must have pink SetBorderColor"
            );
            assert!(
                block.contains("SetBackgroundColor 40 42 54 255"),
                "exceptional {label} block must have Dracula background"
            );
            assert!(
                block.contains("PlayEffect Pink"),
                "exceptional {label} block must have PlayEffect Pink"
            );
            assert!(
                block.contains("MinimapIcon 2 Pink Cross"),
                "exceptional {label} block must have MinimapIcon 2 Pink Cross"
            );
        }

        // Ordering: all exceptional blocks appear BEFORE the recycle block (T-10-04)
        let recycle_pos = output.find("Show # recycle").expect("recycle block not found");
        let exc_2h_pos = output.find("Show # exceptional sockets 2h/body").unwrap();
        let exc_1h_pos = output.find("Show # exceptional sockets 1h").unwrap();
        let exc_quality_pos = output.find("Show # exceptional quality").unwrap();

        assert!(
            exc_2h_pos < recycle_pos,
            "exceptional sockets 2h/body block must precede the recycle block"
        );
        assert!(
            exc_1h_pos < recycle_pos,
            "exceptional sockets 1h block must precede the recycle block"
        );
        assert!(
            exc_quality_pos < recycle_pos,
            "exceptional quality block must precede the recycle block"
        );
    }

    #[test]
    fn generate_rejects_unknown_style_name() {
        // RULE-03 / Phase-5 deferral: a rule referencing a style name outside the
        // name absent from the StyleSet must fail generate-time with GenerateError::UnknownStyle
        // naming the offending style.
        let (mut config, priced) = load_inputs();
        config
            .rules
            .get_or_insert_with(Vec::new)
            .push(crate::models::config::Rule {
                enabled: None,
                name: Some("bad rule".to_string()),
                style: "NotATier".to_string(),
                when: None,
            });
        match generate(&config, &priced) {
            Err(e @ GenerateError::UnknownStyle { .. }) => {
                let GenerateError::UnknownStyle { style, .. } = &e;
                assert_eq!(style, "NotATier", "error must name the offending style");
                let msg = e.to_string();
                assert!(msg.contains("NotATier"), "{msg}");
                assert!(msg.contains("exceptional"), "list comes from the set: {msg}");
            }
            Ok(_) => panic!("expected generate to reject the unknown style 'NotATier'"),
        }
    }

    fn bare_style(action: crate::models::config::StyleAction) -> Style {
        Style {
            text: None,
            border: None,
            background: None,
            font: None,
            minimap: None,
            beam: None,
            sound: None,
            action,
            disable_drop_sound: false,
        }
    }

    fn named_rule(name: &str, style: &str) -> crate::models::config::Rule {
        crate::models::config::Rule {
            enabled: None,
            name: Some(name.to_string()),
            style: style.to_string(),
            when: None,
        }
    }

    /// Block text from the header line `header` up to the next blank line.
    fn block_after<'a>(output: &'a str, header: &str) -> &'a str {
        let start = output.find(header).unwrap_or_else(|| panic!("missing {header:?}"));
        let rest = &output[start..];
        rest.split("\n\n").next().unwrap()
    }

    /// STYLE-01 (D-07): a style added to the set and used by a rule emits with no code change.
    #[test]
    fn a_new_style_in_the_set_emits_with_no_code_change() {
        use crate::models::config::StyleAction;
        let (mut config, priced) = load_inputs();
        let mut st = bare_style(StyleAction::Show);
        st.text = Some([1, 2, 3]);
        st.font = Some(30);
        config.styles.insert("craftable", st);
        config.rules.get_or_insert_with(Vec::new).push(named_rule("brand new style", "craftable"));
        let output = generate(&config, &priced).expect("generate failed");
        let block = block_after(&output, "Show # brand new style");
        assert!(block.contains("SetTextColor 1 2 3 255"), "{block}");
        assert!(block.contains("SetFontSize 30"), "{block}");
    }

    /// D-08: the style's action decides Show/Hide.
    #[test]
    fn a_hide_style_decides_the_block_header() {
        let (mut config, priced) = load_inputs();
        config.rules.get_or_insert_with(Vec::new).push(named_rule("hidden by style", "hidden"));
        let output = generate(&config, &priced).expect("generate failed");
        assert!(output.contains("\nHide # hidden by style\n"), "missing Hide header");
    }

    /// D-09: a Hide style keeps every visual line under its Hide header.
    #[test]
    fn a_hide_style_carries_full_visuals() {
        use crate::models::config::{Beam, Minimap, Sound, StyleAction};
        let (mut config, priced) = load_inputs();
        let mut st = bare_style(StyleAction::Hide);
        st.text = Some([10, 20, 30]);
        st.border = Some([40, 50, 60]);
        st.background = Some([70, 80, 90]);
        st.font = Some(33);
        st.sound = Some(Sound { id: Some(3), custom: None, vol: 100 });
        st.minimap = Some(Minimap { size: 1, color: FilterColor::Red, shape: MinimapShape::Star });
        st.beam = Some(Beam { color: FilterColor::Red, temp: false });
        st.disable_drop_sound = true;
        config.styles.insert("loudhide", st);
        config.rules.get_or_insert_with(Vec::new).push(named_rule("loud hide", "loudhide"));
        let output = generate(&config, &priced).expect("generate failed");
        let block = block_after(&output, "Hide # loud hide");
        for line in [
            "SetTextColor 10 20 30 255",
            "SetBorderColor 40 50 60 255",
            "SetBackgroundColor 70 80 90 255",
            "SetFontSize 33",
            "PlayAlertSound 3 100",
            "MinimapIcon 1 Red Star",
            "PlayEffect Red",
            "DisableDropSound",
        ] {
            assert!(block.contains(line), "missing {line:?} in:\n{block}");
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
        let mut config: Config = serde_json::from_str(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/config.json"
        )))
        .expect("config.json parse failed");
        config.styles = StyleSet::committed_for_tests();

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

    // -----------------------------------------------------------------------
    // FilterIdentity — the header's two identifying lines (12-CONTEXT D-10)
    // -----------------------------------------------------------------------

    /// The standing guard that parameterizing the header did not move the universal path.
    /// `generate` delegates to `generate_as` with the default identity, so these must agree
    /// byte-for-byte — if they ever diverge, `seed-universal.filter` is about to move.
    #[test]
    fn generate_matches_generate_as_default() {
        let (config, priced) = load_inputs();
        let via_generate = generate(&config, &priced).expect("generate failed");
        let via_generate_as =
            generate_as(&config, &priced, &FilterIdentity::default()).expect("generate_as failed");
        assert_eq!(
            via_generate, via_generate_as,
            "generate() and generate_as(default) must produce identical output"
        );
    }

    #[test]
    fn default_identity_is_the_historical_pair() {
        let d = FilterIdentity::default();
        assert_eq!(d.output_name, "universal.filter");
        assert_eq!(d.source_name, "config.json");
    }

    #[test]
    fn generate_as_names_the_filter_and_its_source() {
        let (config, priced) = load_inputs();
        let output = generate_as(
            &config,
            &priced,
            &FilterIdentity {
                output_name: "dynamic.filter",
                source_name: "dynamic.json",
            },
        )
        .expect("generate_as failed");

        let mut lines = output.lines();
        assert_eq!(lines.next(), Some("# dynamic.filter"));
        assert_eq!(
            lines.next(),
            Some("# GENERATED FROM dynamic.json — edit the model, not this file (then regenerate).")
        );
    }

    /// Only the first two lines differ; the legend and philosophy note are untouched (D-11).
    #[test]
    fn custom_identity_changes_only_the_first_two_header_lines() {
        let (config, priced) = load_inputs();
        let default_out = generate(&config, &priced).expect("generate failed");
        let custom_out = generate_as(
            &config,
            &priced,
            &FilterIdentity {
                output_name: "dynamic.filter",
                source_name: "dynamic.json",
            },
        )
        .expect("generate_as failed");

        let default_rest: Vec<&str> = default_out.lines().skip(2).collect();
        let custom_rest: Vec<&str> = custom_out.lines().skip(2).collect();
        assert_eq!(
            default_rest, custom_rest,
            "everything after the first two header lines must be identical"
        );
    }

    // -----------------------------------------------------------------------
    // Disabled-rule suppression (DYN-01) — a rule stays in the store but emits nothing
    // -----------------------------------------------------------------------

    /// Push a rule onto the config and return the index it landed at.
    fn push_rule(config: &mut Config, enabled: Option<bool>, style: &str) -> usize {
        use crate::models::config::{Rule, When};
        let rules = config.rules.get_or_insert_with(Vec::new);
        rules.push(Rule {
            enabled,
            name: None,
            style: style.to_string(),
            when: Some(When {
                sockets: Some(crate::models::config::NumericCondition { min: Some(3), max: None, eq: None }),
                ..Default::default()
            }),
        });
        rules.len() - 1
    }

    #[test]
    fn disabled_rule_emits_no_block_at_all() {
        let (mut config, priced) = load_inputs();
        let idx = push_rule(&mut config, Some(false), "E");
        let output = generate(&config, &priced).expect("generate failed");
        assert!(
            !output.contains(&format!("# rule {}", idx)),
            "a disabled rule must emit no block, but `# rule {}` appeared:\n{}",
            idx,
            output
        );
    }

    #[test]
    fn enabled_true_and_absent_both_emit() {
        for enabled in [Some(true), None] {
            let (mut config, priced) = load_inputs();
            let idx = push_rule(&mut config, enabled, "E");
            let output = generate(&config, &priced).expect("generate failed");
            assert!(
                output.contains(&format!("# rule {}", idx)),
                "enabled={:?} must emit a block, but `# rule {}` was absent",
                enabled,
                idx
            );
        }
    }

    /// The reason `emit_rules` uses `continue` inside the loop rather than filtering the
    /// iterator: labels must stay tied to a rule's index in the source file, so disabling one
    /// rule does not silently relabel the rules after it.
    #[test]
    fn disabling_a_rule_does_not_renumber_later_rules() {
        let (mut config, priced) = load_inputs();
        let first = push_rule(&mut config, None, "E");
        let middle = push_rule(&mut config, Some(false), "E");
        let last = push_rule(&mut config, None, "E");

        let output = generate(&config, &priced).expect("generate failed");
        assert!(output.contains(&format!("# rule {}", first)), "first rule missing");
        assert!(
            !output.contains(&format!("# rule {}", middle)),
            "disabled middle rule must not emit"
        );
        assert!(
            output.contains(&format!("# rule {}", last)),
            "the rule after a disabled one must keep its own index ({}), not shift down",
            last
        );
    }

    /// A rule the author switched off must not be able to fail generation.
    #[test]
    fn disabled_rule_with_unknown_style_is_inert() {
        let (mut config, priced) = load_inputs();
        push_rule(&mut config, Some(false), "no-such-style");
        generate(&config, &priced)
            .expect("a disabled rule referencing an unknown style must not error");
    }

    // -----------------------------------------------------------------------
    // Corrupted condition emit tests (EXC-01)
    // -----------------------------------------------------------------------

    #[test]
    fn emit_rules_emits_corrupted_false() {
        use crate::models::config::{Rule, When};
        let (mut config, priced) = load_inputs();
        config.rules.get_or_insert_with(Vec::new).push(Rule {
            enabled: None,
            name: Some("test-corrupted-false".to_string()),
            style: "E".to_string(),
            when: Some(When {
                corrupted: Some(false),
                ..Default::default()
            }),
        });
        let output = generate(&config, &priced).expect("generate failed");
        assert!(
            output.contains("  Corrupted False"),
            "expected '  Corrupted False' in output, got:\n{}",
            output
        );
        assert!(
            !output.contains("Corrupted =="),
            "Corrupted must NOT use == operator, got:\n{}",
            output
        );
    }

    #[test]
    fn emit_rules_emits_corrupted_true() {
        use crate::models::config::{Rule, When};
        let (mut config, priced) = load_inputs();
        config.rules.get_or_insert_with(Vec::new).push(Rule {
            enabled: None,
            name: Some("test-corrupted-true".to_string()),
            style: "E".to_string(),
            when: Some(When {
                corrupted: Some(true),
                ..Default::default()
            }),
        });
        let output = generate(&config, &priced).expect("generate failed");
        assert!(
            output.contains("  Corrupted True"),
            "expected '  Corrupted True' in output, got:\n{}",
            output
        );
    }

    #[test]
    fn emit_rules_omits_corrupted_when_none() {
        use crate::models::config::{Rule, When, NumericCondition};
        let (mut config, priced) = load_inputs();
        config.rules.get_or_insert_with(Vec::new).push(Rule {
            enabled: None,
            name: Some("test-no-corrupted".to_string()),
            style: "E".to_string(),
            when: Some(When {
                sockets: Some(NumericCondition { eq: None, min: Some(1), max: None }),
                corrupted: None,
                ..Default::default()
            }),
        });
        let output = generate(&config, &priced).expect("generate failed");
        // Find the block for our test rule
        let block = output
            .split("\n\n")
            .find(|b| b.contains("test-no-corrupted"))
            .expect("block for test-no-corrupted not found");
        assert!(
            !block.contains("Corrupted"),
            "no Corrupted line should be emitted when corrupted is None:\n{}",
            block
        );
    }

    #[test]
    fn gem_blocks_have_single_per_category_background() {
        let (config, priced) = load_inputs();
        let output = generate(&config, &priced).expect("generate failed");
        // Walk blocks: a block starts at a "Show"/"Hide" line and ends at a blank line.
        for block in output.split("\n\n") {
            let header = block.lines().next().unwrap_or("");
            let bg_count = block.lines().filter(|l| l.trim_start().starts_with("SetBackgroundColor")).count();
            if header.contains("# Uncut Gems") {
                assert!(block.contains("SetBackgroundColor 80 70 55 255"), "uncut gem block must carry 80 70 55 bg:\n{block}");
                assert!(!block.contains("SetBackgroundColor 40 42 54 255"), "uncut gem block must not keep the Dracula bg:\n{block}");
                assert_eq!(bg_count, 1, "uncut gem block must have exactly one SetBackgroundColor:\n{block}");
            }
            if header.contains("# Lineage Support Gems") {
                assert!(block.contains("SetBackgroundColor 10 150 150 255"), "lineage gem block must carry 10 150 150 bg:\n{block}");
                assert!(!block.contains("SetBackgroundColor 40 42 54 255"), "lineage gem block must not keep the Dracula bg:\n{block}");
                assert_eq!(bg_count, 1, "lineage gem block must have exactly one SetBackgroundColor:\n{block}");
            }
            if header.contains("# Verisium") {
                assert!(block.contains("SetBackgroundColor 40 42 54 255"), "Verisium must keep the Dracula bg:\n{block}");
                assert!(!block.contains("SetBackgroundColor 80 70 55"), "Verisium must not get the uncut override:\n{block}");
                assert!(!block.contains("SetBackgroundColor 10 150 150"), "Verisium must not get the lineage override:\n{block}");
            }
        }
    }
}
