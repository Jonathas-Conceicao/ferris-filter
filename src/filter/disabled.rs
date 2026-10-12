//! Disabled (commented-out) block detection and validity classification (D-08, D-09).
//!
//! A disabled block is a `#Show`/`#Hide`/`#Minimal` header followed by `#`-prefixed grammar lines.
//! Disabling adds one `#` per line and enabling removes exactly one, so the inner text is the line
//! after its FIRST `#`, kept verbatim. Prose such as `# Show items above 80` has values after the
//! keyword and stays an ordinary comment. The inner lines go through the same tokenizer and shape
//! validator as live lines; failures make the block `Invalid` (kept byte-for-byte), never an error.

use super::block::{BlockKeyword, BodyLine, DisabledState};
use super::diag::{FilterDiagnostic, Severity};
use super::keywords::{lookup, Kind};
use super::lines::Line;
use super::parse::block_keyword;
use super::tokenize::{tokenize, validate_shape};

pub(crate) struct Disabled {
    pub keyword: BlockKeyword,
    pub body: Vec<BodyLine>,
    /// Exclusive end: one past the last keyword line (or the header when there is no body).
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
        match tokenize(inner) {
            Ok(t) => {
                let Some(r) = &t.keyword else {
                    k += 1;
                    continue;
                };
                match lookup(&inner[r.clone()]) {
                    // Unknown first word: prose. Enabling it would produce a live
                    // unknown-keyword line, so inside the owned range it blocks enabling.
                    None => pending.push(reason(format!(
                        "`{}` is not filter grammar",
                        &inner[r.clone()]
                    ))),
                    Some(kw) if kw.kind == Kind::BlockHeader => {
                        if is_header(text) {
                            break;
                        }
                    }
                    Some(kw) if kw.kind == Kind::Directive => {}
                    Some(kw) => {
                        reasons.append(&mut pending);
                        if let Err(m) = validate_shape(kw, inner, &t) {
                            reasons.push(reason(m));
                        }
                        body.push(BodyLine {
                            line: k,
                            keyword: Some(kw),
                            tokens: Some(t),
                        });
                        end = k + 1;
                    }
                }
            }
            Err(e) => {
                // A malformed line only counts when it starts with a known grammar word.
                let word: String = inner
                    .trim_start_matches([' ', '\t'])
                    .chars()
                    .take_while(|c| c.is_ascii_alphabetic())
                    .collect();
                if let Some(kw) = lookup(&word) {
                    if matches!(kw.kind, Kind::Condition | Kind::Action | Kind::Modifier) {
                        reasons.append(&mut pending);
                        reasons.push(reason(e.message));
                        body.push(BodyLine {
                            line: k,
                            keyword: Some(kw),
                            tokens: None,
                        });
                        end = k + 1;
                    }
                }
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
