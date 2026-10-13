//! Recognition of FerrisFilter's own metadata comments (`# ff:<key> <value>`).
//!
//! The writer form is exactly `# ff:<key> <value>` ([`FF_PREFIX`]), one setting per line in a
//! comment run directly above a block. Reading is a little more lenient: the pre-v0.5 spelling
//! `#ff:` ([`LEGACY_FF_PREFIX`]) is still recognized, leading spaces/tabs are accepted, and the
//! value view has trailing whitespace trimmed. Reading never touches stored text; Apply rewrites
//! legacy-spelled lines to the current prefix ([`normalize_ff_prefix`]).

use super::lines::split;

/// The prefix FerrisFilter writes on every metadata and economy-marker line.
pub const FF_PREFIX: &str = "# ff:";
/// The pre-v0.5 spelling (no space after `#`). Still read; rewritten to [`FF_PREFIX`] on Apply.
pub const LEGACY_FF_PREFIX: &str = "#ff:";

fn is_ws(c: char) -> bool {
    c == ' ' || c == '\t'
}

/// A metadata line in the writer form: `# ff:<key>`, plus ` <value>` when given.
pub fn ff_line(key: &str, value: Option<&str>) -> String {
    match value {
        Some(v) => format!("{FF_PREFIX}{key} {v}"),
        None => format!("{FF_PREFIX}{key}"),
    }
}

/// Split off the metadata prefix (either spelling): `(indent, is_legacy, rest after "ff:")`.
fn strip_prefix(text: &str) -> Option<(&str, bool, &str)> {
    let rest = text.trim_start_matches(is_ws);
    let indent = &text[..text.len() - rest.len()];
    if let Some(r) = rest.strip_prefix(FF_PREFIX) {
        Some((indent, false, r))
    } else {
        rest.strip_prefix(LEGACY_FF_PREFIX)
            .map(|r| (indent, true, r))
    }
}

/// Recognize a `# ff:` (or legacy `#ff:`) line. Returns `(key, value)`; `value` is `None` when
/// the line is only a key.
pub(crate) fn parse_ff_line(text: &str) -> Option<(&str, Option<&str>)> {
    let (_, _, rest) = strip_prefix(text)?;
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

/// The current-prefix form of a legacy-spelled (`#ff:`) metadata line, indent kept; `None` for
/// any other line, including one already in the current form.
pub(crate) fn normalize_ff_line(text: &str) -> Option<String> {
    parse_ff_line(text)?;
    let (indent, legacy, rest) = strip_prefix(text)?;
    legacy.then(|| format!("{indent}{FF_PREFIX}{rest}"))
}

/// Rewrite every legacy-spelled (`#ff:`) metadata/marker line to the current [`FF_PREFIX`]. Every
/// other byte (BOM, line endings, foreign comments) is kept exactly.
pub fn normalize_ff_prefix(text: &str) -> String {
    let (bom, lines) = split(text);
    let mut out = String::with_capacity(text.len() + 16);
    if bom {
        out.push('\u{feff}');
    }
    for l in &lines {
        match normalize_ff_line(l.text()) {
            Some(t) => out.push_str(&t),
            None => out.push_str(l.text()),
        }
        out.push_str(l.eol().as_str());
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn key_value() {
        for pre in [FF_PREFIX, LEGACY_FF_PREFIX] {
            assert_eq!(
                parse_ff_line(&format!("{pre}name Exceptional 3 socks")),
                Some(("name", Some("Exceptional 3 socks")))
            );
            assert_eq!(
                parse_ff_line(&format!("{pre}style S  \t")),
                Some(("style", Some("S")))
            );
        }
    }

    #[test]
    fn key_only_marker() {
        for pre in [FF_PREFIX, LEGACY_FF_PREFIX] {
            assert_eq!(
                parse_ff_line(&format!("{pre}economy-begin")),
                Some(("economy-begin", None))
            );
            assert_eq!(
                parse_ff_line(&format!("{pre}economy-end   ")),
                Some(("economy-end", None))
            );
        }
    }

    #[test]
    fn leading_whitespace_accepted() {
        assert_eq!(parse_ff_line("  \t#ff:name x"), Some(("name", Some("x"))));
        assert_eq!(parse_ff_line("  \t# ff:name x"), Some(("name", Some("x"))));
    }

    #[test]
    fn non_markers() {
        assert_eq!(parse_ff_line("#  ff:name x"), None);
        assert_eq!(parse_ff_line("#\tff:name x"), None);
        assert_eq!(parse_ff_line("# # ff:name x"), None);
        assert_eq!(parse_ff_line("# #ff:name x"), None);
        assert_eq!(parse_ff_line("# ff: name x"), None);
        assert_eq!(parse_ff_line("#ff:"), None);
        assert_eq!(parse_ff_line("# ff:"), None);
        assert_eq!(parse_ff_line("#ff:na!me x"), None);
        assert_eq!(parse_ff_line("# ff:na!me x"), None);
        assert_eq!(parse_ff_line("Show"), None);
        assert_eq!(parse_ff_line("#ffx:name"), None);
        assert_eq!(parse_ff_line("# ffx:name"), None);
    }

    #[test]
    fn ff_line_writes_the_spaced_prefix() {
        assert_eq!(ff_line("name", Some("x y")), "# ff:name x y");
        assert_eq!(ff_line("economy-end", None), "# ff:economy-end");
    }

    #[test]
    fn normalize_line() {
        assert_eq!(
            normalize_ff_line("#ff:name x").as_deref(),
            Some("# ff:name x")
        );
        assert_eq!(
            normalize_ff_line("\t#ff:economy-begin").as_deref(),
            Some("\t# ff:economy-begin")
        );
        assert_eq!(normalize_ff_line("# ff:name x"), None);
        assert_eq!(normalize_ff_line("#ff:"), None);
        assert_eq!(normalize_ff_line("#ff:na!me"), None);
        assert_eq!(normalize_ff_line("# note"), None);
    }

    #[test]
    fn normalize_text_touches_only_legacy_meta_lines() {
        let src = "\u{feff}# note\r\n#ff:name a\r\n#Show\n#\tRarity Rare\n#ff:\n# ff:style S\n#ff:economy-end";
        assert_eq!(
            normalize_ff_prefix(src),
            "\u{feff}# note\r\n# ff:name a\r\n#Show\n#\tRarity Rare\n#ff:\n# ff:style S\n# ff:economy-end"
        );
        let clean = "# a\r\n#Show\n#\tRarity Rare\n# ff:name x\nShow # #ff:name y\n";
        assert_eq!(normalize_ff_prefix(clean), clean);
        assert_eq!(normalize_ff_prefix(""), "");
    }
}
