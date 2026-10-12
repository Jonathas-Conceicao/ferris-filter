//! Disabled (commented-out) block detection and validity classification (D-08, D-09).
//!
//! A disabled block is a `#Show`/`#Hide`/`#Minimal` header followed by `#`-prefixed grammar lines.
//! Disabling adds one `#` per line and enabling removes exactly one, so the inner text is the line
//! after its FIRST `#`, kept verbatim. Prose such as `# Show items above 80` has values after the
//! keyword and stays an ordinary comment. The inner lines go through the same tokenizer and shape
//! validator as live lines; failures make the block `Invalid` (kept byte-for-byte), never an error.

use super::block::{BlockKeyword, BodyLine, DisabledState};
use super::diag::{FilterDiagnostic, Severity};
use super::keywords::{lookup, Keyword, Kind};
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
    /// Not grammar, or not laid out like this block's body: an ordinary comment.
    Prose(String),
}

/// Leading spaces/tabs of `s`.
pub(crate) fn lead_ws(s: &str) -> &str {
    &s[..s.len() - s.trim_start_matches([' ', '\t']).len()]
}

/// Body-style indentation after the `#` (tab or at least two spaces), as `disable` produces from an
/// indented live line. Prose comments conventionally use `# ` with a single space.
fn body_indented(inner: &str) -> bool {
    inner.starts_with('\t') || inner.starts_with("  ")
}

/// The one rule (shared by the live trailing run and disabled-block detection) deciding whether a
/// commented line that is not a header reads as a disabled body line of the current block.
///
/// `est` is the established body indentation: the leading whitespace of the block's previous body
/// line (live text, or the text after the `#` for a disabled block), `None` before the first one.
/// - Once established, the line must be indented exactly like it. A line with a problem (bad
///   values, unknown keyword, untokenizable) must additionally be indented at all: an unindented
///   `#Note here` under an unindented body is prose.
/// - Before the first body line, a clean grammar line counts at any indentation; a line with a
///   problem only when body-indented (tab or two spaces).
///
/// So `# Corrupted items` under a tab-indented body, `#  Note: x` under a tab-indented body, and
/// `# Quality gear below` directly under a header are all prose, while the disabled form of any
/// live body line laid out like its neighbours is a body line.
fn indent_ok(inner: &str, clean: bool, est: Option<&str>) -> bool {
    let lead = lead_ws(inner);
    match est {
        Some(e) => lead == e && (clean || !lead.is_empty()),
        None => clean || body_indented(inner),
    }
}

fn body_or_prose(
    inner: &str,
    k: usize,
    est: Option<&str>,
    keyword: Option<&'static Keyword>,
    tokens: Option<TokenizedLine>,
    problem: Option<String>,
    prose: impl FnOnce() -> String,
) -> Inner {
    if indent_ok(inner, problem.is_none(), est) {
        Inner::Body {
            problem,
            line: BodyLine {
                line: k,
                keyword,
                tokens,
            },
        }
    } else {
        Inner::Prose(prose())
    }
}

/// Classify the inner text of the `#`-prefixed line `text` at 0-based line `k`, given the block's
/// established body indentation `est` (see [`indent_ok`]).
pub(crate) fn classify_inner(text: &str, inner: &str, k: usize, est: Option<&str>) -> Inner {
    match tokenize(inner) {
        Ok(t) => {
            let Some(r) = t.keyword.clone() else {
                return Inner::Neutral;
            };
            let word = &inner[r];
            match lookup(word) {
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
                Some(kw) => {
                    let problem = validate_shape(kw, inner, &t).err();
                    body_or_prose(inner, k, est, Some(kw), Some(t), problem, || {
                        format!("`{word}` line is not laid out like the block body")
                    })
                }
                // Body-indented (`#\tFutureKw 3`): a disabled unknown-keyword line, owned like the
                // live line it came from (WR-02) but never enableable.
                None => {
                    let problem = Some(format!("unknown keyword `{word}`"));
                    body_or_prose(inner, k, est, None, Some(t), problem, || {
                        format!("`{word}` is not filter grammar")
                    })
                }
            }
        }
        Err(e) => {
            // A malformed line (`#\t123`, the disabled form of a bad live body line) extends the
            // block only when laid out like a body line; otherwise (`# $tag`) it is prose.
            let word: String = inner
                .trim_start_matches([' ', '\t'])
                .chars()
                .take_while(|c| c.is_ascii_alphabetic())
                .collect();
            let kw = lookup(&word);
            match kw {
                Some(kw) if !matches!(kw.kind, Kind::Condition | Kind::Action | Kind::Modifier) => {
                    Inner::Prose(format!("not filter grammar: {}", e.message))
                }
                _ => {
                    let msg = e.message;
                    let prose = format!("not filter grammar: {msg}");
                    body_or_prose(inner, k, est, kw, None, Some(msg), || prose)
                }
            }
        }
    }
}

/// True when `text` is a commented-out body line of a block whose established body indentation is
/// `est`: one `#` away from a line that extends that block. A run of these after a live block's
/// last line belongs to that block (WR-03), so a commented-out condition moves with its rule.
pub(crate) fn is_commented_body(text: &str, k: usize, est: Option<&str>) -> bool {
    uncomment_once(text)
        .is_some_and(|inner| matches!(classify_inner(text, inner, k, est), Inner::Body { .. }))
}

pub(crate) fn detect(ls: &[Line], h: usize) -> Option<Disabled> {
    let keyword = detect_header(ls[h].text())?;
    let mut body = Vec::new();
    let mut reasons: Vec<FilterDiagnostic> = Vec::new();
    // Non-grammar lines seen after the current `end`. They only become reasons once a later
    // grammar line pulls them inside the owned range (enabling would turn them into live lines);
    // trailing prose stays an ordinary comment outside the block.
    let mut pending: Vec<FilterDiagnostic> = Vec::new();
    // Established body indentation: lead of the last body line's inner text (see `indent_ok`).
    let mut est: Option<&str> = None;
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
        match classify_inner(text, inner, k, est) {
            Inner::Header => break,
            Inner::Neutral => {
                // A commented-out body line directly after the last owned line (`##\tKw ...`)
                // mirrors the live rule in `parse::scan_live_body`, so disable/enable keep the
                // same extent.
                if k == end && is_commented_body(inner, k, est) {
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
                est = Some(lead_ws(inner));
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
    fn headings_with_grammar_words_do_not_invalidate_disabled_rules() {
        // Iteration-2 WR-01: a heading that starts with a grammar word (or is indented unlike the
        // body) is not a disabled condition, so a valid disabled rule stays enableable.
        for heading in [
            "# Corrupted items",
            "# Quality gear below",
            "# ItemLevel >= 80 only",
            "# Continue below",
            "# Sockets 3 is good",
            "#  Note: these are weapons",
            "#   [[0100]] Section",
        ] {
            let src = format!("#Show\n#\tRarity Rare\n{heading}\nShow\n");
            let mut d = p(&src);
            let b: Vec<_> = d.blocks().collect();
            assert_eq!(b[0].owned(), 0..2, "{heading:?}");
            assert!(b[0].can_enable(), "{heading:?}");
            assert_eq!(b[1].leading(), 2..3, "{heading:?}");
            let id = b[0].id();
            d.set_enabled(id, true)
                .unwrap_or_else(|e| panic!("{heading:?}: {e}"));
            assert_eq!(
                d.serialize(),
                format!("Show\n\tRarity Rare\n{heading}\nShow\n")
            );
        }
        // Directly under a header (no body yet), a line with a problem must be body-indented.
        let d = p("#Show\n# Corrupted items\nShow\n");
        assert_eq!(d.blocks().next().unwrap().owned(), 0..1);
        assert!(d.blocks().next().unwrap().can_enable());
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
