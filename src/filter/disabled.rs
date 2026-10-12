//! Disabled (commented-out) block detection and validity classification (D-08, D-09).
//!
//! A disabled block is a `#Show`/`#Hide`/`#Minimal` header followed by `#`-prefixed grammar lines.
//! Disabling adds one `#` per line and enabling removes exactly one, so the inner text is the line
//! after its FIRST `#`, kept verbatim. Prose such as `# Show items above 80` has values after the
//! keyword and stays an ordinary comment. The inner lines go through the same tokenizer and shape
//! validator as live lines; failures make the block `Invalid` (kept byte-for-byte), never an error.

use super::block::{BlockKeyword, BodyLine, DisabledState};
use super::diag::{FilterDiagnostic, Severity};
use super::keywords::{lookup, Keyword, Kind, ValueShape};
use super::lines::Line;
use super::parse::block_keyword;
use super::tokenize::{tokenize, validate_shape, TokenizedLine};

pub(crate) struct Disabled {
    pub keyword: BlockKeyword,
    pub body: Vec<BodyLine>,
    /// Exclusive end: one past the last body line (or the header when there is no body), including
    /// a directly following run of doubly commented-out body lines.
    pub end: usize,
    pub state: DisabledState,
}

/// The text after the first `#`, when the line's first non-whitespace character is `#`.
pub(crate) fn uncomment_once(text: &str) -> Option<&str> {
    text.trim_start_matches([' ', '\t']).strip_prefix('#')
}

/// If `text` is a commented-out block header (`#Show`, `#Hide`, `#Minimal`, optionally followed by
/// a comment), its keyword.
pub(crate) fn detect_header(text: &str) -> Option<BlockKeyword> {
    let inner = uncomment_once(text)?;
    let t = tokenize(inner).ok()?;
    let r = t.keyword.as_ref()?;
    let kw = lookup(&inner[r.clone()])?;
    if kw.kind == Kind::BlockHeader && t.operator.is_none() && t.values.is_empty() {
        Some(block_keyword(kw))
    } else {
        None
    }
}

pub(crate) fn is_header(text: &str) -> bool {
    detect_header(text).is_some()
}

/// How one disabled line's inner text (after its first `#`) would behave if enabled.
pub(crate) enum Inner {
    /// Blank or a pure comment (`##...`): neutral, owned only when inside the range.
    Neutral,
    /// A commented-out block header: ends the block.
    Header,
    /// A grammar line that extends the block. `problem` makes the block un-enableable.
    Body {
        line: BodyLine,
        problem: Option<String>,
    },
    /// Not grammar. Owned only when a later body line pulls it inside the range, and then it
    /// blocks enabling.
    Prose(String),
}

/// Body-style indentation after the `#` (tab or at least two spaces), as `disable` produces from an
/// indented live line. Prose comments conventionally use `# ` with a single space.
fn body_indented(inner: &str) -> bool {
    inner.starts_with('\t') || inner.starts_with("  ")
}

/// A `QuotedList` line with several values of which some are unquoted, and no body-style
/// indentation after the `#`, reads as prose: `# Class weapons below`.
fn prose_shaped(kw: &Keyword, inner: &str, t: &TokenizedLine) -> bool {
    kw.value == ValueShape::QuotedList
        && !body_indented(inner)
        && t.values.len() >= 2
        && t.values.iter().any(|v| !inner[v.clone()].starts_with('"'))
}

/// Classify the inner text of the `#`-prefixed line `text` at 0-based line `k`.
pub(crate) fn classify_inner(text: &str, inner: &str, k: usize) -> Inner {
    match tokenize(inner) {
        Ok(t) => {
            let Some(r) = t.keyword.clone() else {
                return Inner::Neutral;
            };
            let word = &inner[r];
            match lookup(word) {
                // Body-indented (`#\tFutureKw 3`): a disabled unknown-keyword line, owned like the
                // live line it came from (WR-02) but never enableable.
                None if body_indented(inner) => Inner::Body {
                    problem: Some(format!("unknown keyword `{word}`")),
                    line: BodyLine {
                        line: k,
                        keyword: None,
                        tokens: Some(t),
                    },
                },
                // Enabling would produce a live unknown-keyword line.
                None => Inner::Prose(format!("`{word}` is not filter grammar")),
                Some(kw) if kw.kind == Kind::BlockHeader => {
                    if is_header(text) {
                        Inner::Header
                    } else {
                        // A header word with trailing text: enabled, it would start a new block.
                        Inner::Prose(format!("`{word}` would start a new block"))
                    }
                }
                // Enabled, an `Import` would end the block early.
                Some(kw) if kw.kind == Kind::Directive => {
                    Inner::Prose(format!("`{}` cannot appear inside a block", kw.name))
                }
                Some(kw) if prose_shaped(kw, inner, &t) => {
                    Inner::Prose(format!("`{word}` line reads as prose, not filter grammar"))
                }
                Some(kw) => Inner::Body {
                    problem: validate_shape(kw, inner, &t).err(),
                    line: BodyLine {
                        line: k,
                        keyword: Some(kw),
                        tokens: Some(t),
                    },
                },
            }
        }
        Err(e) => {
            // A malformed line extends the block when it starts with a known grammar word, or
            // when it is body-indented (`#\t123`, the disabled form of a bad live body line).
            // Otherwise (`# $tag`) it is prose.
            let word: String = inner
                .trim_start_matches([' ', '\t'])
                .chars()
                .take_while(|c| c.is_ascii_alphabetic())
                .collect();
            match lookup(&word) {
                Some(kw) if matches!(kw.kind, Kind::Condition | Kind::Action | Kind::Modifier) => {
                    Inner::Body {
                        problem: Some(e.message),
                        line: BodyLine {
                            line: k,
                            keyword: Some(kw),
                            tokens: None,
                        },
                    }
                }
                None if body_indented(inner) => Inner::Body {
                    problem: Some(e.message),
                    line: BodyLine {
                        line: k,
                        keyword: None,
                        tokens: None,
                    },
                },
                _ => Inner::Prose(format!("not filter grammar: {}", e.message)),
            }
        }
    }
}

/// True when `text` is a commented-out body line: one `#` away from a line that extends a block.
/// A run of these directly after a block's last line belongs to that block (WR-03), so a
/// commented-out condition moves with its rule instead of becoming the next block's comment.
pub(crate) fn is_commented_body(text: &str, k: usize) -> bool {
    uncomment_once(text).is_some_and(|inner| matches!(classify_inner(text, inner, k), Inner::Body { .. }))
}

pub(crate) fn detect(ls: &[Line], h: usize) -> Option<Disabled> {
    let keyword = detect_header(ls[h].text())?;
    let mut body = Vec::new();
    let mut reasons: Vec<FilterDiagnostic> = Vec::new();
    // Non-grammar lines seen after the current `end`. They only become reasons once a later
    // grammar line pulls them inside the owned range (enabling would turn them into live lines);
    // trailing prose stays an ordinary comment outside the block.
    let mut pending: Vec<FilterDiagnostic> = Vec::new();
    let mut end = h + 1;
    let mut k = h + 1;
    while k < ls.len() {
        let text = ls[k].text();
        let Some(inner) = uncomment_once(text) else {
            break;
        };
        let reason = |message: String| FilterDiagnostic {
            line: k + 1,
            severity: Severity::Warning,
            message,
        };
        match classify_inner(text, inner, k) {
            Inner::Header => break,
            Inner::Neutral => {
                // A commented-out body line directly after the last owned line (`##\tKw ...`)
                // mirrors the live rule in `parse::scan_live_body`, so disable/enable keep the
                // same extent.
                if k == end && is_commented_body(inner, k) {
                    end = k + 1;
                }
            }
            Inner::Prose(m) => pending.push(reason(m)),
            Inner::Body { line, problem } => {
                reasons.append(&mut pending);
                if let Some(m) = problem {
                    reasons.push(reason(m));
                }
                body.push(line);
                end = k + 1;
            }
        }
        k += 1;
    }
    let state = if reasons.is_empty() {
        DisabledState::Valid
    } else {
        DisabledState::Invalid(reasons)
    };
    Some(Disabled {
        keyword,
        body,
        end,
        state,
    })
}

#[cfg(test)]
mod tests {
    use crate::filter::{parse, BlockKeyword, DisabledState, Document, Item, Severity};

    fn p(s: &str) -> Document {
        parse(s.as_bytes()).unwrap()
    }

    #[test]
    fn disabled_valid_with_continue() {
        let d = p("#Show  # # %D9 $type->x\n#\tRarity Rare\n#\tContinue\n");
        let b: Vec<_> = d.blocks().collect();
        assert_eq!(b.len(), 1);
        assert!(!b[0].is_enabled());
        assert!(b[0].can_enable());
        assert_eq!(b[0].keyword(), BlockKeyword::Show);
        assert_eq!(b[0].owned(), 0..3);
        assert!(b[0].has_continue());
    }

    #[test]
    fn quoted_hash_respected() {
        let d = p("#Show\n#\tBaseType == \"a # b\"\n");
        assert!(d.blocks().next().unwrap().can_enable());
    }

    #[test]
    fn invalid_reason_tied_to_line() {
        let d = p("#Show\n#\tRarity =< Rare\n");
        let b = d.blocks().next().unwrap();
        match b.disabled_state().unwrap() {
            DisabledState::Invalid(r) => {
                assert_eq!(r.len(), 1);
                assert_eq!(r[0].line, 2);
                assert!(r[0].message.contains("unknown operator"));
            }
            other => panic!("{other:?}"),
        }
        assert!(!b.can_enable());
        assert!(!d.diagnostics().is_empty());
        assert!(d
            .diagnostics()
            .iter()
            .all(|x| x.severity != Severity::Error));
    }

    #[test]
    fn prose_stays_comment() {
        for s in ["# Show items above 80\n", "# Showcase\n"] {
            let d = p(s);
            assert_eq!(d.blocks().count(), 0, "{s}");
            assert!(matches!(d.items()[0], Item::Trivia(_)));
        }
    }

    #[test]
    fn header_alone_is_valid_empty() {
        let d = p("#Show\n");
        let b = d.blocks().next().unwrap();
        assert!(b.can_enable());
        assert!(b.body().is_empty());
        assert_eq!(b.owned(), 0..1);
    }

    #[test]
    fn trailing_banner_not_owned() {
        let d = p("#Show\n#\tRarity Rare\n# a section banner\n");
        let b = d.blocks().next().unwrap();
        assert_eq!(b.owned(), 0..2);
        assert!(matches!(d.items()[1], Item::Trivia(ref r) if *r == (2..3)));
    }

    #[test]
    fn interior_prose_blocks_enabling() {
        // CR-01: prose between grammar lines would become a live unknown-keyword line.
        let d = p("#Show\n#\tRarity Rare\n# note here\n#\tClass \"Bows\"\n");
        let b = d.blocks().next().unwrap();
        assert_eq!(b.owned(), 0..4);
        assert!(!b.can_enable());
        match b.disabled_state().unwrap() {
            DisabledState::Invalid(r) => {
                assert_eq!(r.len(), 1);
                assert_eq!(r[0].line, 3);
                assert!(r[0].message.contains("`note` is not filter grammar"));
            }
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn can_enable_agrees_with_enable() {
        // WR-01: tag lines, bare numbers, commented directives and header words with trailing
        // text inside the owned range make the block un-enableable instead of silently passing.
        for src in [
            "#Show\n#\tRarity Rare\n# $tag\n#\tClass \"Bows\"\n",
            "#Show\n#\tRarity Rare\n#\t123\n#\tClass \"Bows\"\n",
            "#Show\n#\tRarity Rare\n#Import \"x\"\n#\tClass \"Bows\"\n",
            "#Show\n#\tRarity Rare\n#Show Continue\n#\tClass \"Bows\"\n",
        ] {
            let mut d = p(src);
            let id = d.blocks().next().unwrap().id();
            let b = d.block(id).unwrap();
            assert_eq!(b.owned(), 0..4, "{src:?}");
            assert!(!b.can_enable(), "{src:?}");
            assert!(d.set_enabled(id, true).is_err(), "{src:?}");
            assert_eq!(d.serialize(), src);
        }
        // Trailing tag lines stay ordinary comments.
        let d = p("#Show\n#\tRarity Rare\n# $tag\n");
        let b = d.blocks().next().unwrap();
        assert_eq!(b.owned(), 0..2);
        assert!(b.can_enable());
    }

    #[test]
    fn trailing_prose_keeps_block_valid() {
        let d = p("#Show\n#\tRarity Rare\n# note here\n");
        let b = d.blocks().next().unwrap();
        assert_eq!(b.owned(), 0..2);
        assert!(b.can_enable());
    }

    #[test]
    fn interior_hash_only_line_owned() {
        let d = p("#Show\n#\tRarity Rare\n#\n#\tBaseType \"x\"\n");
        let b = d.blocks().next().unwrap();
        assert_eq!(b.owned(), 0..4);
        assert_eq!(b.body().len(), 2);
    }

    #[test]
    fn adjacent_disabled_headers_split() {
        let d = p("#Show\n#\tRarity Rare\n#Hide\n#\tRarity Rare\n");
        let b: Vec<_> = d.blocks().collect();
        assert_eq!(b.len(), 2);
        assert_eq!(b[0].owned(), 0..2);
        assert_eq!(b[1].keyword(), BlockKeyword::Hide);
        assert_eq!(b[1].owned(), 2..4);
    }

    #[test]
    fn live_header_ends_disabled_block() {
        let d = p("#Show\n#\tRarity Rare\nShow\n\tRarity Rare\n");
        let b: Vec<_> = d.blocks().collect();
        assert_eq!(b.len(), 2);
        assert!(!b[0].is_enabled());
        assert_eq!(b[0].owned(), 0..2);
        assert!(b[1].is_enabled());
    }

    #[test]
    fn disabled_header_ends_live_block() {
        let d = p("Show\n\tRarity Rare\n#Hide\n#\tRarity Rare\n");
        let b: Vec<_> = d.blocks().collect();
        assert_eq!(b[0].owned(), 0..2);
        assert!(!b[1].is_enabled());
    }
}
