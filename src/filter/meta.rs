//! Recognition of FerrisFilter's own metadata comments (`#ff:<key> <value>`).
//!
//! The writer form (used by later phases) is exactly `#ff:<key> <value>`, one setting per line in
//! a comment run directly above a block. Reading is a little more lenient: leading spaces/tabs
//! before `#ff:` are accepted, and the value view has trailing whitespace trimmed. Stored text is
//! never touched.

fn is_ws(c: char) -> bool {
    c == ' ' || c == '\t'
}

/// Recognize a `#ff:` line. Returns `(key, value)`; `value` is `None` when the line is only a key.
pub(crate) fn parse_ff_line(text: &str) -> Option<(&str, Option<&str>)> {
    let rest = text.trim_start_matches(is_ws).strip_prefix("#ff:")?;
    let key_len = rest
        .bytes()
        .take_while(|b| b.is_ascii_alphanumeric() || *b == b'_' || *b == b'-')
        .count();
    if key_len == 0 {
        return None;
    }
    let (key, after) = rest.split_at(key_len);
    if after.is_empty() {
        return Some((key, None));
    }
    if !after.starts_with(is_ws) {
        return None;
    }
    let value = after.trim_start_matches(is_ws).trim_end_matches(is_ws);
    Some((key, if value.is_empty() { None } else { Some(value) }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn key_value() {
        assert_eq!(
            parse_ff_line("#ff:name Exceptional 3 socks"),
            Some(("name", Some("Exceptional 3 socks")))
        );
        assert_eq!(parse_ff_line("#ff:style S  \t"), Some(("style", Some("S"))));
    }

    #[test]
    fn key_only_marker() {
        assert_eq!(
            parse_ff_line("#ff:economy-begin"),
            Some(("economy-begin", None))
        );
        assert_eq!(
            parse_ff_line("#ff:economy-end   "),
            Some(("economy-end", None))
        );
    }

    #[test]
    fn leading_whitespace_accepted() {
        assert_eq!(parse_ff_line("  \t#ff:name x"), Some(("name", Some("x"))));
    }

    #[test]
    fn non_markers() {
        assert_eq!(parse_ff_line("# ff:name x"), None);
        assert_eq!(parse_ff_line("#ff:"), None);
        assert_eq!(parse_ff_line("#ff:na!me x"), None);
        assert_eq!(parse_ff_line("Show"), None);
        assert_eq!(parse_ff_line("#ffx:name"), None);
    }
}
