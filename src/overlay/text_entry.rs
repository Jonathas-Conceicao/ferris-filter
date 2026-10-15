//! Text-entry helpers shared by the rule editor (Phase 15 base-type entry, reused by Phase 20).
//!
//! Pure string handling, no egui and no model types, so it stays unit-testable without a window
//! (`PITFALLS.md` §8).

/// Parses one commit of the base-type add field.
///
/// A bare typed name (no `"` anywhere in the input) yields exactly one entry, trimmed. Input
/// containing a double quote is treated as the `BaseType == "A" "B"` form the author copies out
/// of hand-written filters: every quoted span becomes its own entry, in the order it appears,
/// with any text between spans (whitespace, or the `BaseType ==` prefix itself if pasted whole)
/// ignored, and an empty span dropped. Blank or whitespace-only input yields an empty vec.
///
/// This is the whole of it -- trimming and quote-stripping is string hygiene on input, not
/// validation. There is no list of legitimate base names to check against anywhere in the
/// codebase: a typo is stored exactly as typed, and PoE2 itself is the only oracle that warns
/// about it, at filter load time.
pub fn parse_base_type_entry(raw: &str) -> Vec<String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Vec::new();
    }
    if trimmed.contains('"') {
        let mut out = Vec::new();
        let mut chars = trimmed.char_indices().peekable();
        while let Some((start, c)) = chars.next() {
            if c == '"' {
                if let Some(end) = trimmed[start + 1..].find('"') {
                    let inner = trimmed[start + 1..start + 1 + end].trim();
                    if !inner.is_empty() {
                        out.push(inner.to_string());
                    }
                    // Skip the iterator past the closing quote so it isn't reconsidered as the
                    // opening quote of the next span.
                    while let Some((i, _)) = chars.peek() {
                        if *i > start + 1 + end {
                            break;
                        }
                        chars.next();
                    }
                }
            }
        }
        out
    } else {
        vec![trimmed.to_string()]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_base_type_entry_splits_a_pasted_quoted_list() {
        let parsed = parse_base_type_entry(
            "\"Heavy Belt\" \"Utility Belt\" \"Armoured Cap\" \"Sacred Focus\"",
        );
        assert_eq!(
            parsed,
            vec![
                "Heavy Belt".to_string(),
                "Utility Belt".to_string(),
                "Armoured Cap".to_string(),
                "Sacred Focus".to_string(),
            ]
        );
    }

    #[test]
    fn parse_base_type_entry_strips_surrounding_quotes_from_a_single_name() {
        assert_eq!(
            parse_base_type_entry("\"Heavy Belt\""),
            vec!["Heavy Belt".to_string()]
        );
    }

    #[test]
    fn parse_base_type_entry_on_blank_input_yields_nothing() {
        assert_eq!(parse_base_type_entry(""), Vec::<String>::new());
        assert_eq!(parse_base_type_entry("   "), Vec::<String>::new());
    }

    #[test]
    fn parse_base_type_entry_keeps_an_unknown_name_verbatim() {
        // D-05: a typo is stored, not rejected -- there is no base-name list in the
        // codebase to check against.
        assert_eq!(
            parse_base_type_entry("Sacerd Focus"),
            vec!["Sacerd Focus".to_string()]
        );
    }
}
