use miette::Diagnostic;
use serde::{Deserialize, Serialize};
use thiserror::Error;

const ALL_TIERS: &[&str] = &["S", "A", "B", "C", "D", "E", "F"];

/// The price unit for a tier lower-bound entry.
#[derive(Debug, Deserialize, Serialize, PartialEq, Clone)]
#[serde(rename_all = "lowercase")]
pub enum Unit {
    Chaos,
    Div,
    /// Exalt-denominated cutoff. Used by `tiers-fallback.json` so the early-league fallback
    /// can tier currency relative to Exalted Orb (like poe2filter.com) instead of Divine —
    /// this frees Divine from being pinned to the A floor. See `prices-fallback.json`.
    Exalt,
}

impl Unit {
    /// The lowercase serde spelling, as shown read-only in the CONFIG tab.
    pub fn as_str(&self) -> &'static str {
        match self {
            Unit::Chaos => "chaos",
            Unit::Div => "div",
            Unit::Exalt => "exalt",
        }
    }
}

/// A single lower-bound entry in `tiers.json`.
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct TierEntry {
    pub tier: String,
    pub min: f64,
    pub unit: Unit,
}

/// A validated, complete set of tier entries.
///
/// Use `ValidatedTiers::try_from(vec)` to construct; the constructor enforces:
/// 1. All 7 tiers S–F are present.
/// 2. Within each unit group, `min` values are strictly ascending.
#[derive(Debug)]
pub struct ValidatedTiers(pub Vec<TierEntry>);

/// Validation error for `tiers.json`.
#[derive(Debug, Error, Diagnostic)]
pub enum TierValidationError {
    /// One or more required tiers are absent from the file.
    #[error("tiers.json missing required tier(s): {}", .0.join(", "))]
    #[diagnostic(code(ferrisfilter::tiers::missing))]
    MissingTiers(Vec<String>),

    /// Two adjacent entries in the same unit group are not strictly ascending.
    #[error(
        "tiers.json bounds not strictly ascending: tier {prev} min={prev_min} must be < tier {curr} min={curr_min} (same unit: {unit:?})"
    )]
    #[diagnostic(code(ferrisfilter::tiers::order))]
    NotAscending {
        prev: String,
        prev_min: f64,
        curr: String,
        curr_min: f64,
        unit: Unit,
    },
}

impl TryFrom<Vec<TierEntry>> for ValidatedTiers {
    type Error = TierValidationError;

    fn try_from(entries: Vec<TierEntry>) -> Result<Self, Self::Error> {
        // 1. All 7 tiers must be present.
        let present: std::collections::HashSet<&str> =
            entries.iter().map(|e| e.tier.as_str()).collect();
        let missing: Vec<String> = ALL_TIERS
            .iter()
            .filter(|t| !present.contains(*t))
            .map(|s| s.to_string())
            .collect();
        if !missing.is_empty() {
            return Err(TierValidationError::MissingTiers(missing));
        }

        // 2. Within each unit group, min values must be strictly ascending.
        // NOTE: Cross-unit comparison is intentionally skipped (Pitfall 6 — units are
        // incommensurable in Phase 1; normalization is Phase 3's job).
        for unit in [Unit::Chaos, Unit::Div, Unit::Exalt] {
            let group: Vec<&TierEntry> = entries.iter().filter(|e| e.unit == unit).collect();
            for pair in group.windows(2) {
                if pair[1].min <= pair[0].min {
                    return Err(TierValidationError::NotAscending {
                        prev: pair[0].tier.clone(),
                        prev_min: pair[0].min,
                        curr: pair[1].tier.clone(),
                        curr_min: pair[1].min,
                        unit: unit.clone(),
                    });
                }
            }
        }

        Ok(ValidatedTiers(entries))
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn unit_as_str_matches_serde() {
        use super::Unit;
        for u in [Unit::Chaos, Unit::Div, Unit::Exalt] {
            assert_eq!(
                serde_json::to_string(&u).unwrap(),
                format!("\"{}\"", u.as_str())
            );
        }
    }

    use super::*;

    // ──────────────────────────────────────────────────────────────
    // Helpers
    // ──────────────────────────────────────────────────────────────

    fn all_seven() -> Vec<TierEntry> {
        vec![
            TierEntry {
                tier: "A".into(),
                min: 1.0,
                unit: Unit::Div,
            },
            TierEntry {
                tier: "S".into(),
                min: 5.0,
                unit: Unit::Div,
            },
            TierEntry {
                tier: "F".into(),
                min: 0.1,
                unit: Unit::Chaos,
            },
            TierEntry {
                tier: "E".into(),
                min: 0.5,
                unit: Unit::Chaos,
            },
            TierEntry {
                tier: "D".into(),
                min: 2.0,
                unit: Unit::Chaos,
            },
            TierEntry {
                tier: "C".into(),
                min: 10.0,
                unit: Unit::Chaos,
            },
            TierEntry {
                tier: "B".into(),
                min: 30.0,
                unit: Unit::Chaos,
            },
        ]
    }

    // ──────────────────────────────────────────────────────────────
    // Fixture round-trip
    // ──────────────────────────────────────────────────────────────

    #[test]
    fn load_real_tiers_json_parses_and_validates() {
        let src = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/tiers.json"));
        let entries: Vec<TierEntry> = serde_json::from_str(src).expect("parse tiers.json");
        let validated = ValidatedTiers::try_from(entries).expect("validate tiers.json");
        assert_eq!(validated.0.len(), 7);
    }

    #[test]
    fn load_real_tiers_fallback_json_parses_and_validates() {
        let src = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/tiers-fallback.json"));
        let entries: Vec<TierEntry> = serde_json::from_str(src).expect("parse tiers-fallback.json");
        let validated = ValidatedTiers::try_from(entries).expect("validate tiers-fallback.json");
        assert_eq!(validated.0.len(), 7);
        assert!(
            validated.0.iter().all(|e| e.unit == Unit::Exalt),
            "tiers-fallback.json should be entirely exalt-denominated"
        );
    }

    #[test]
    fn round_trip_tier_entry() {
        let json = r#"{"tier":"S","min":5.0,"unit":"div"}"#;
        let entry: TierEntry = serde_json::from_str(json).unwrap();
        let back = serde_json::to_string(&entry).unwrap();
        let entry2: TierEntry = serde_json::from_str(&back).unwrap();
        assert_eq!(entry.tier, entry2.tier);
        assert_eq!(entry.min, entry2.min);
        assert_eq!(entry.unit, Unit::Div);
    }

    // ──────────────────────────────────────────────────────────────
    // ValidatedTiers::try_from validation
    // ──────────────────────────────────────────────────────────────

    #[test]
    fn all_seven_tiers_ok() {
        assert!(ValidatedTiers::try_from(all_seven()).is_ok());
    }

    #[test]
    fn missing_tier_b_returns_err() {
        let entries: Vec<TierEntry> = all_seven().into_iter().filter(|e| e.tier != "B").collect();
        match ValidatedTiers::try_from(entries) {
            Err(TierValidationError::MissingTiers(missing)) => {
                assert!(missing.contains(&"B".to_string()));
            }
            other => panic!("expected MissingTiers, got {other:?}"),
        }
    }

    #[test]
    fn non_ascending_within_chaos_unit_returns_err() {
        // E min=0.1 then D min=0.05 — not ascending within chaos group
        let mut entries = all_seven();
        // Replace E and D entries with non-ascending values
        for e in &mut entries {
            if e.tier == "E" {
                e.min = 0.1;
            }
            if e.tier == "D" {
                e.min = 0.05;
            }
        }
        let result = ValidatedTiers::try_from(entries);
        assert!(
            matches!(result, Err(TierValidationError::NotAscending { .. })),
            "expected NotAscending, got {result:?}"
        );
    }

    #[test]
    fn cross_unit_transition_5_chaos_then_1_div_is_ok() {
        // A cross-unit transition (5.0 chaos → 1.0 div) must NOT trigger NotAscending
        // because units are incommensurable in Phase 1.
        let entries = vec![
            TierEntry {
                tier: "F".into(),
                min: 0.1,
                unit: Unit::Chaos,
            },
            TierEntry {
                tier: "E".into(),
                min: 0.5,
                unit: Unit::Chaos,
            },
            TierEntry {
                tier: "D".into(),
                min: 2.0,
                unit: Unit::Chaos,
            },
            TierEntry {
                tier: "C".into(),
                min: 5.0,
                unit: Unit::Chaos,
            },
            TierEntry {
                tier: "B".into(),
                min: 1.0,
                unit: Unit::Div,
            },
            TierEntry {
                tier: "A".into(),
                min: 2.0,
                unit: Unit::Div,
            },
            TierEntry {
                tier: "S".into(),
                min: 5.0,
                unit: Unit::Div,
            },
        ];
        assert!(ValidatedTiers::try_from(entries).is_ok());
    }
}
