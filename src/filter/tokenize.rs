//! Quote-aware single-line tokenizer and lenient shape validator.
//!
//! Operates on borrowed text and returns byte ranges into the line. It never rebuilds, trims or
//! normalizes stored text; operators are recorded verbatim. Foreign comment tags (`$type->`, `%D4`,
//! `!slug`) are only comment spans and are never interpreted.

use std::ops::Range;

use super::keywords::{Keyword, Ops, ValueShape, COLOURS, RARITIES, SHAPES};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Operator {
    Eq,
    EqEq,
    Bang,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
}

impl Operator {
    pub fn as_str(self) -> &'static str {
        match self {
            Operator::Eq => "=",
            Operator::EqEq => "==",
            Operator::Bang => "!",
            Operator::Ne => "!=",
            Operator::Lt => "<",
            Operator::Le => "<=",
            Operator::Gt => ">",
            Operator::Ge => ">=",
        }
    }

    fn from_run(run: &str) -> Option<Operator> {
        Some(match run {
            "=" => Operator::Eq,
            "==" => Operator::EqEq,
            "!" => Operator::Bang,
            "!=" => Operator::Ne,
            "<" => Operator::Lt,
            "<=" => Operator::Le,
            ">" => Operator::Gt,
            ">=" => Operator::Ge,
            _ => return None,
        })
    }
}

/// All ranges are byte offsets into the tokenized line; quoted values include their quotes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TokenizedLine {
    pub indent: Range<usize>,
    pub keyword: Option<Range<usize>>,
    pub operator: Option<(Range<usize>, Operator)>,
    pub values: Vec<Range<usize>>,
    pub comment: Option<Range<usize>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TokenError {
    /// 1-based byte column.
    pub col: usize,
    pub message: String,
}

fn is_ws(b: u8) -> bool {
    b == b' ' || b == b'\t'
}

fn is_op_char(b: u8) -> bool {
    matches!(b, b'=' | b'<' | b'>' | b'!')
}

/// Tokenize one line (without its terminator). Delimiters are all ASCII, so every slice boundary
/// falls on a char boundary.
pub fn tokenize(text: &str) -> Result<TokenizedLine, TokenError> {
    let b = text.as_bytes();
    let n = b.len();
    let mut i = 0;
    while i < n && is_ws(b[i]) {
        i += 1;
    }
    let mut t = TokenizedLine {
        indent: 0..i,
        keyword: None,
        operator: None,
        values: Vec::new(),
        comment: None,
    };
    if i >= n {
        return Ok(t);
    }
    if b[i] == b'#' {
        t.comment = Some(i..n);
        return Ok(t);
    }
    let ks = i;
    while i < n && b[i].is_ascii_alphabetic() {
        i += 1;
    }
    if i == ks {
        return Err(TokenError {
            col: ks + 1,
            message: "expected keyword".to_string(),
        });
    }
    t.keyword = Some(ks..i);

    // An operator may be glued to the keyword or to the first value.
    let mut j = i;
    while j < n && is_ws(b[j]) {
        j += 1;
    }
    let mut end_prev = i;
    if j < n && is_op_char(b[j]) {
        let mut k = j;
        while k < n && is_op_char(b[k]) {
            k += 1;
        }
        let run = &text[j..k];
        match Operator::from_run(run) {
            Some(op) => t.operator = Some((j..k, op)),
            None => {
                return Err(TokenError {
                    col: j + 1,
                    message: format!("unknown operator `{run}`"),
                })
            }
        }
        i = k;
        end_prev = k;
    }
    loop {
        while i < n && is_ws(b[i]) {
            i += 1;
        }
        if i >= n {
            break;
        }
        match b[i] {
            b'#' => {
                // The comment span includes the whitespace run before the `#`.
                t.comment = Some(end_prev..n);
                break;
            }
            b'"' => {
                let qs = i;
                i += 1;
                while i < n && b[i] != b'"' {
                    i += 1;
                }
                if i >= n {
                    return Err(TokenError {
                        col: qs + 1,
                        message: "unterminated quote".to_string(),
                    });
                }
                i += 1;
                t.values.push(qs..i);
            }
            _ => {
                let vs = i;
                while i < n && !is_ws(b[i]) && b[i] != b'#' && b[i] != b'"' {
                    i += 1;
                }
                t.values.push(vs..i);
            }
        }
        end_prev = i;
    }
    Ok(t)
}

fn unquote(v: &str) -> &str {
    v.strip_prefix('"')
        .and_then(|s| s.strip_suffix('"'))
        .unwrap_or(v)
}

fn is_int(v: &str) -> bool {
    v.parse::<i64>().is_ok()
}

fn in_list(v: &str, list: &[&str]) -> bool {
    list.iter().any(|w| w.eq_ignore_ascii_case(unquote(v)))
}

fn is_socket_word(v: &str) -> bool {
    let rest = v.trim_start_matches(|c: char| c.is_ascii_digit());
    !v.is_empty() && rest.chars().all(|c| "RGBADWrgbadw".contains(c))
}

/// Lenient arity/shape check. Never validates numeric ranges or names (the game warns on load).
pub fn validate_shape(kw: &Keyword, line: &str, t: &TokenizedLine) -> Result<(), String> {
    if kw.ops == Ops::None {
        if let Some((_, op)) = &t.operator {
            return Err(format!(
                "`{}` does not take an operator (found `{}`)",
                kw.name,
                op.as_str()
            ));
        }
    }
    let vals: Vec<&str> = t.values.iter().map(|r| &line[r.clone()]).collect();
    let n = vals.len();
    let need = |ok: bool, what: &str| -> Result<(), String> {
        if ok {
            Ok(())
        } else {
            Err(format!("`{}` expects {what}", kw.name))
        }
    };
    match kw.value {
        ValueShape::NoValue => need(n == 0, "no values"),
        ValueShape::Bool => need(
            n == 1
                && (vals[0].eq_ignore_ascii_case("true") || vals[0].eq_ignore_ascii_case("false")),
            "True or False",
        ),
        ValueShape::Int => need(
            n >= 1 && vals.iter().all(|v| is_int(v)),
            "one or more integers",
        ),
        ValueShape::QuotedList => need(n >= 1, "one or more values"),
        ValueShape::CountThenNames => {
            need(n >= 1, "a count and names")?;
            need(
                !(is_int(vals[0]) && n < 2),
                "at least one name after the count",
            )
        }
        ValueShape::Enum(list) => need(
            n >= 1 && vals.iter().all(|v| in_list(v, list)),
            &format!("values from {list:?}"),
        ),
        ValueShape::Rarity => need(
            n >= 1 && vals.iter().all(|v| in_list(v, RARITIES)),
            "Normal, Magic, Rare or Unique",
        ),
        ValueShape::Sockets => need(
            n >= 1 && vals.iter().all(|v| is_socket_word(v)),
            "socket counts or colours like 5GGG",
        ),
        ValueShape::BoolOrName => need(n == 1, "one value"),
        ValueShape::Rgba => need(
            (n == 3 || n == 4) && vals.iter().all(|v| is_int(v)),
            "3 or 4 integers",
        ),
        ValueShape::SoundId => need(
            (n == 1 || n == 2)
                && (is_int(vals[0]) || vals[0].eq_ignore_ascii_case("none"))
                && (n == 1 || is_int(vals[1])),
            "an id (or None) and optional volume",
        ),
        ValueShape::CustomSound => need(
            (n == 1 || n == 2) && (n == 1 || is_int(vals[1])),
            "a file and optional volume",
        ),
        ValueShape::MinimapIcon => {
            let ok = if n >= 1 && vals[0] == "-1" {
                n <= 3
                    && (n < 2 || in_list(vals[1], COLOURS))
                    && (n < 3 || in_list(vals[2], SHAPES))
            } else {
                n == 3 && is_int(vals[0]) && in_list(vals[1], COLOURS) && in_list(vals[2], SHAPES)
            };
            need(ok, "size, colour and shape (or -1)")
        }
        ValueShape::PlayEffect => {
            let ok = n >= 1
                && if vals[0].eq_ignore_ascii_case("none") {
                    n == 1
                } else {
                    in_list(vals[0], COLOURS)
                        && (n == 1 || (n == 2 && vals[1].eq_ignore_ascii_case("temp")))
                };
            need(ok, "None, or a colour with optional Temp")
        }
        ValueShape::FontSize => need(n == 1 && is_int(vals[0]), "one integer"),
        ValueShape::ImportPath => need(
            (n == 1 || n == 2)
                && vals[0].starts_with('"')
                && (n == 1 || vals[1].eq_ignore_ascii_case("optional")),
            "a quoted path and optional `Optional`",
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::filter::keywords::lookup;

    fn s<'a>(line: &'a str, r: &Range<usize>) -> &'a str {
        &line[r.clone()]
    }
    fn vals<'a>(line: &'a str, t: &TokenizedLine) -> Vec<&'a str> {
        t.values.iter().map(|r| s(line, r)).collect()
    }

    #[test]
    fn tok_spaced_operator() {
        let l = "\tRarity >= Rare";
        let t = tokenize(l).unwrap();
        assert_eq!(s(l, &t.indent), "\t");
        assert_eq!(s(l, t.keyword.as_ref().unwrap()), "Rarity");
        assert_eq!(t.operator.as_ref().unwrap().1, Operator::Ge);
        assert_eq!(vals(l, &t), ["Rare"]);
    }

    #[test]
    fn tok_glued_operators() {
        let l = "Rarity>=Magic";
        let t = tokenize(l).unwrap();
        assert_eq!(t.operator.as_ref().unwrap().1, Operator::Ge);
        assert_eq!(vals(l, &t), ["Magic"]);
        let l = "HasExplicitMod >=1 \"Foo\" \"Bar\"";
        let t = tokenize(l).unwrap();
        assert_eq!(t.operator.as_ref().unwrap().1, Operator::Ge);
        assert_eq!(vals(l, &t), ["1", "\"Foo\"", "\"Bar\""]);
    }

    #[test]
    fn tok_all_eight_operators() {
        for (sym, op) in [
            ("=", Operator::Eq),
            ("==", Operator::EqEq),
            ("!", Operator::Bang),
            ("!=", Operator::Ne),
            ("<", Operator::Lt),
            ("<=", Operator::Le),
            (">", Operator::Gt),
            (">=", Operator::Ge),
        ] {
            let l = format!("Quality {sym} 1");
            let t = tokenize(&l).unwrap();
            let (r, o) = t.operator.unwrap();
            assert_eq!(o, op);
            assert_eq!(&l[r], sym);
            assert_eq!(o.as_str(), sym);
        }
    }

    #[test]
    fn tok_hash_in_quotes_is_not_comment() {
        let l = "BaseType == \"a # b\" # real";
        let t = tokenize(l).unwrap();
        assert_eq!(vals(l, &t), ["\"a # b\""]);
        assert_eq!(s(l, t.comment.as_ref().unwrap()), " # real");
    }

    #[test]
    fn tok_no_operator_and_multi_value() {
        let l = "Class \"Bows\"";
        let t = tokenize(l).unwrap();
        assert!(t.operator.is_none());
        assert_eq!(vals(l, &t), ["\"Bows\""]);
        let l = "Rarity Normal Magic Rare";
        let t = tokenize(l).unwrap();
        assert!(t.operator.is_none());
        assert_eq!(t.values.len(), 3);
    }

    #[test]
    fn tok_bad_operators() {
        for l in ["Rarity =< Rare", "Rarity => Rare"] {
            let e = tokenize(l).unwrap_err();
            assert!(e.message.contains("unknown operator"), "{}", e.message);
        }
    }

    #[test]
    fn tok_unterminated_quote() {
        let e = tokenize("BaseType == \"unterminated").unwrap_err();
        assert_eq!(e.message, "unterminated quote");
    }

    #[test]
    fn tok_blank_and_comment_lines() {
        for l in ["# just a comment", "", "   "] {
            let t = tokenize(l).unwrap();
            assert!(t.keyword.is_none(), "{l:?}");
        }
        let t = tokenize("# just a comment").unwrap();
        assert!(t.comment.is_some());
    }

    #[test]
    fn tok_foreign_tags_are_comment_only() {
        let l = "Show # $type->currency %D4";
        let t = tokenize(l).unwrap();
        assert_eq!(s(l, t.keyword.as_ref().unwrap()), "Show");
        assert!(t.values.is_empty());
        assert_eq!(s(l, t.comment.as_ref().unwrap()), " # $type->currency %D4");
    }

    #[test]
    fn tok_expected_keyword() {
        assert!(tokenize("\t123").is_err());
    }

    #[test]
    fn tok_never_panics_on_multibyte() {
        for base in [
            "Base\u{e9}Type == \"a\u{2014}b\"",
            "Show # \u{1F600} x",
            "\u{e9}\u{2014}\u{1F600}",
        ] {
            let mut positions: Vec<usize> = base.char_indices().map(|(p, _)| p).collect();
            positions.push(base.len());
            for pos in positions {
                for ins in ["\"", "#", "="] {
                    let mut x = String::from(&base[..pos]);
                    x.push_str(ins);
                    x.push_str(&base[pos..]);
                    let _ = tokenize(&x);
                }
            }
        }
    }

    fn check(line: &str) -> Result<(), String> {
        let t = tokenize(line).unwrap();
        let k = t.keyword.clone().unwrap();
        let kw = lookup(&line[k]).unwrap();
        validate_shape(kw, line, &t)
    }

    #[test]
    fn shape_accepts_real_world_forms() {
        for l in [
            "SetTextColor 1 2 3",
            "SetTextColor 1 2 3 4",
            "MinimapIcon 0 Red Circle",
            "MinimapIcon -1",
            "PlayEffect None",
            "PlayEffect Red Temp",
            "PlayAlertSound 6 300",
            "CustomAlertSound \"uwu.mp3\" 300",
            "Sockets > 4",
            "Sockets 5GGG",
            "SocketGroup RGB",
            "Corrupted True",
            "Continue",
            "DisableDropSound",
            "Import \"x.filter\" Optional",
            "Class \"Bows\"",
            "BaseType \"Heavy Belt\"",
            "Rarity Normal Magic Rare",
            "HasExplicitMod >=1 \"x\"",
            "SetFontSize 37",
        ] {
            assert_eq!(check(l), Ok(()), "{l}");
        }
    }

    #[test]
    fn shape_rejects_bad_forms() {
        for l in [
            "SetTextColor red",
            "MinimapIcon 0 Mauve Circle",
            "Corrupted Maybe",
            "AreaLevel abc",
            "SetFontSize >= 3",
        ] {
            assert!(check(l).is_err(), "{l}");
        }
    }
}
