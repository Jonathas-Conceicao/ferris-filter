//! Disabled (commented-out) block detection and validity classification (D-08, D-09).
//!
//! A disabled block is a `#Show`/`#Hide`/`#Minimal` header followed by `#`-prefixed grammar lines.
//! Disabling adds one `#` per line and enabling removes exactly one, so the inner text is the line
//! after its FIRST `#`, kept verbatim. Prose such as `# Show items above 80` has values after the
//! keyword and stays an ordinary comment. The inner lines go through the same tokenizer and shape
//! validator as live lines; failures make the block `Invalid` (kept byte-for-byte), never an error.

use std::borrow::Cow;

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
    /// Not grammar, or not laid out like this block's body: an ordinary comment that ends the
    /// block (it belongs to whatever follows).
    Prose,
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
        Inner::Prose
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
                        Inner::Prose
                    }
                }
                // Enabled, an `Import` would end the block early.
                Some(kw) if kw.kind == Kind::Directive => {
                    Inner::Prose
                }
                Some(kw) => {
                    let problem = validate_shape(kw, inner, &t).err();
                    body_or_prose(inner, k, est, Some(kw), Some(t), problem)
                }
                // Body-indented (`#\tFutureKw 3`): a disabled unknown-keyword line, owned like the
                // live line it came from (WR-02) but never enableable.
                None => {
                    let problem = Some(format!("unknown keyword `{word}`"));
                    body_or_prose(inner, k, est, None, Some(t), problem)
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
                    Inner::Prose
                }
                _ => body_or_prose(inner, k, est, kw, None, Some(e.message)),
            }
        }
    }
}

/// `text` with exactly one `#` removed (the first one), as `set_enabled(true)` writes it.
pub(crate) fn enabled_text(text: &str) -> Cow<'_, str> {
    match text.find('#') {
        Some(0) => Cow::Borrowed(&text[1..]),
        Some(p) => {
            let mut s = text.to_string();
            s.remove(p);
            Cow::Owned(s)
        }
        None => Cow::Borrowed(text),
    }
}

/// Detect the disabled block whose header is line `h`.
///
/// The extent mirrors the live rules of `parse::scan_live_body` one `#` deeper:
/// - a disabled body line (`Inner::Body`) always extends the block, with any neutral lines before
///   it (they are the disabled form of the live block's interior blank and comment lines);
/// - a doubly commented-out body line (`##\tKw ...`) extends it only within the trailing run after
///   the last body line, which a blank (`#`) line or a heading/prose comment closes, just like the
///   live trailing run;
/// - the first prose line, header, or uncommented line ends the scan (WR-04). That prose and
///   anything after it is the next block's leading run or standalone comment text, never part of
///   this block, so enabling can never turn prose into a live line (CR-01).
pub(crate) fn detect(ls: &[Line], h: usize) -> Option<Disabled> {
    let keyword = detect_header(ls[h].text())?;
    let mut body = Vec::new();
    let mut reasons: Vec<FilterDiagnostic> = Vec::new();
    // Established body indentation: lead of the last body line's inner text (see `indent_ok`).
    let mut est: Option<&str> = None;
    // Whether the trailing run after the last body line is still open for `##` body lines.
    let mut run_open = true;
    let mut end = h + 1;
    for (k, l) in ls.iter().enumerate().skip(h + 1) {
        let text = l.text();
        let Some(inner) = uncomment_once(text) else {
            break;
        };
        match classify_inner(text, inner, k, est) {
            Inner::Header | Inner::Prose => break,
            Inner::Neutral => match uncomment_once(inner) {
                // `##...`: the disabled form of a live comment line.
                Some(inner2) => match classify_inner(inner, inner2, k, est) {
                    Inner::Body { .. } => {
                        if run_open {
                            end = k + 1;
                        }
                    }
                    Inner::Neutral => {}
                    Inner::Header | Inner::Prose => run_open = false,
                },
                // `#` alone: the disabled form of a live blank line.
                None => run_open = false,
            },
            Inner::Body { line, problem } => {
                if let Some(message) = problem {
                    reasons.push(FilterDiagnostic {
                        line: k + 1,
                        severity: Severity::Warning,
                        message,
                    });
                }
                body.push(line);
                est = Some(lead_ws(inner));
                run_open = true;
                end = k + 1;
            }
        }
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
    fn prose_ends_disabled_block() {
        // CR-01 / iteration-2 WR-04: prose after a disabled body line ends the block, exactly as
        // it ends the live trailing run, so it can never be enabled into a live line. What follows
        // it is standalone comment text (or the next block's leading run).
        let src = "#Show\n#\tRarity Rare\n# note here\n#\tClass \"Bows\"\n";
        let mut d = p(src);
        let b = d.blocks().next().unwrap();
        assert_eq!(b.owned(), 0..2);
        assert!(b.can_enable());
        let id = b.id();
        d.set_enabled(id, true).unwrap();
        assert_eq!(
            d.serialize(),
            "Show\n\tRarity Rare\n# note here\n#\tClass \"Bows\"\n"
        );
        assert_eq!(d.block(id).unwrap().owned(), 0..2);
        assert!(d.diagnostics().is_empty());
    }

    #[test]
    fn can_enable_agrees_with_enable() {
        // WR-01 (iteration 1) and iteration-2 WR-04: can_enable() is true exactly when
        // set_enabled(true) succeeds, and a refused enable leaves the bytes untouched.
        for (src, owned, enableable) in [
            ("#Show\n#\tRarity Rare\n# $tag\n#\tClass \"Bows\"\n", 0..2, true),
            ("#Show\n#\tRarity Rare\n#\t123\n#\tClass \"Bows\"\n", 0..4, false),
            ("#Show\n#\tRarity Rare\n#Import \"x\"\n#\tClass \"Bows\"\n", 0..2, true),
            ("#Show\n#\tRarity Rare\n#Show Continue\n#\tClass \"Bows\"\n", 0..2, true),
            ("#Show\n#\tRarity Rare\n# note\n#\tClass \"Bows\"\nHide\n", 0..2, true),
            // Enabled, the stray condition below the blank line would join the block.
            ("#Show\n#\tRarity Rare\n\n\tClass \"Bows\"\n", 0..2, false),
            // A blank (`#`) line closes the doubly commented trailing run, as it does live.
            ("#Show\n#\tRarity Rare\n#\n##\tSetFontSize 40\nHide\n", 0..2, true),
            ("#Show\n#\tRarity Rare\n## note\n##\tSetFontSize 40\nHide\n", 0..2, true),
            ("#Show\n#\tRarity Rare\n###\n##\tSetFontSize 40\nHide\n", 0..4, true),
        ] {
            let mut d = p(src);
            let id = d.blocks().next().unwrap().id();
            let b = d.block(id).unwrap();
            assert_eq!(b.owned(), owned, "{src:?}");
            assert_eq!(b.can_enable(), enableable, "{src:?}");
            let r = d.set_enabled(id, true);
            assert_eq!(r.is_ok(), enableable, "{src:?}: {r:?}");
            if enableable {
                assert_eq!(d.block(id).unwrap().owned(), owned, "{src:?}");
            } else {
                assert_eq!(d.serialize(), src);
            }
        }
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
