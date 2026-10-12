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
use super::keywords::{lookup, Keyword, Kind, ValueShape};
use super::lines::Line;
use super::meta::parse_ff_line;
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

/// The one rule (shared by the live trailing run and disabled-block detection) deciding whether a
/// commented line that is not a header reads as a disabled body line of the current block.
///
/// `est` is the established body indentation: the leading whitespace of the block's previous body
/// line (live text, or the text after the `#` for a disabled block), `None` before the first one.
/// - A **clean** grammar line (a condition, action or `Continue` that fully validates) is a body
///   line at any indentation (tabs, spaces, mixed or none), exactly as the live scanner takes a
///   live body line at any indentation. Indentation is never a reason to cut a valid rule short
///   (iteration-3 CR-01 / WR-01).
/// - A line with a problem (bad values, unknown keyword, untokenizable) is a body line only when
///   the block already has a body and the line is indented, exactly like that body. This is the
///   disabled form of a broken live line (`#\tFutureKw 3` under `#\tRarity Rare`).
/// - Directly under a header (no body yet), a line with a problem is a body line only when it is
///   body-indented (tab or two spaces) and starts with a known condition/action word: the broken
///   first condition `#\tRarity =< Rare` is owned, while `#  Note: x` or `#\tSee notes below`
///   after a header-only catch-all is the next block's heading (iteration-3 WR-02). `detect`
///   still refuses to enable a disabled header whose next line is a body-indented line it did not
///   take (`#\tFutureKw 3`), since enabled the bare header would be a catch-all.
///
/// So `# Corrupted items` and `#  Note: x` under a tab-indented body are prose, while the disabled
/// form of every clean live body line is a body line.
fn indent_ok(inner: &str, clean: bool, known: bool, est: Option<&str>) -> bool {
    if clean {
        return true;
    }
    let lead = lead_ws(inner);
    match est {
        Some(e) => lead == e && !lead.is_empty(),
        None => known && body_indented(inner),
    }
}

/// Body-style indentation after the `#` (tab or at least two spaces).
fn body_indented(inner: &str) -> bool {
    inner.starts_with('\t') || inner.starts_with("  ")
}

/// A list-valued line (`Class`, `BaseType`, ...) with several values of which some are unquoted,
/// and no body-style indentation after the `#`, reads as prose even though the grammar accepts it:
/// `# Class weapons below`. It is treated as a line with a problem, so it never counts as a clean
/// condition: it cannot extend a block at an unusual indentation, and enabling never turns it into
/// a live condition.
fn prose_shaped(kw: &Keyword, inner: &str, t: &TokenizedLine) -> bool {
    kw.value == ValueShape::QuotedList
        && !body_indented(inner)
        && t.values.len() >= 2
        && t.values.iter().any(|v| !inner[v.clone()].starts_with('"'))
}

/// Whether a clean commented grammar line (at any indentation) follows line `from` within the run
/// of commented lines that a header, a `#ff:` line or a non-comment line ends. Used after prose
/// ends a disabled block: such a line reads as part of the rule, and enabling the block would
/// leave it behind as a comment, producing a live rule broader than its text (iteration-3 CR-01).
/// Returns the 0-based line of the first one.
fn clean_grammar_ahead(ls: &[Line], from: usize) -> Option<usize> {
    for (j, l) in ls.iter().enumerate().skip(from) {
        let text = l.text();
        let inner = uncomment_once(text)?;
        if parse_ff_line(text).is_some() {
            return None;
        }
        match classify_inner(text, inner, j, None) {
            Inner::Header => return None,
            Inner::Body { problem: None, .. } => return Some(j),
            _ => {}
        }
    }
    None
}

fn body_or_prose(
    inner: &str,
    k: usize,
    est: Option<&str>,
    keyword: Option<&'static Keyword>,
    tokens: Option<TokenizedLine>,
    problem: Option<String>,
) -> Inner {
    if indent_ok(inner, problem.is_none(), keyword.is_some(), est) {
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
                    let problem = validate_shape(kw, inner, &t).err().or_else(|| {
                        prose_shaped(kw, inner, &t)
                            .then(|| format!("`{word}` line reads as prose, not filter grammar"))
                    });
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
/// - the first prose line, `#ff:` line, header, or uncommented line ends the scan (WR-04). That prose and
///   anything after it is the next block's leading run or standalone comment text, never part of
///   this block, so enabling can never turn prose into a live line (CR-01).
/// - when prose ends the scan but a clean commented condition follows further down the same
///   commented run (before a header, `#ff:` line or uncommented line), the block keeps its extent
///   but is `Invalid`: enabling it would leave that condition behind and write a broader live rule
///   than the text (iteration-3 CR-01).
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
        // A `#ff:` line belongs to the header run of the block below it (D-05), never inside
        // another block.
        if parse_ff_line(text).is_some() {
            break;
        }
        match classify_inner(text, inner, k, est) {
            Inner::Header => break,
            Inner::Prose => {
                // Directly under the header, a body-indented line not taken as a condition
                // (`#\tFutureKw 3`) may still be the rule's broken first condition rather than a
                // heading: enabled, the header alone would be a catch-all (for `Hide`, hiding
                // everything), so refuse (WR-02).
                if body.is_empty() && body_indented(inner) {
                    reasons.push(FilterDiagnostic {
                        line: k + 1,
                        severity: Severity::Warning,
                        message: "the indented line below the header reads as a broken condition; enabled, the rule would match every item".into(),
                    });
                }
                // Prose ends the block, but a clean condition further down the same commented
                // run reads as part of this rule: enabling would drop it, so refuse (CR-01).
                if let Some(j) = clean_grammar_ahead(ls, k + 1) {
                    reasons.push(FilterDiagnostic {
                        line: j + 1,
                        severity: Severity::Warning,
                        message: format!(
                            "a commented condition below the comment on line {} would be left behind when enabled",
                            k + 1
                        ),
                    });
                }
                break;
            }
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
        // Iteration-2 WR-04: prose after a disabled body line ends the block, exactly as it ends
        // the live trailing run, so it can never be enabled into a live line.
        let src = "#Show\n#\tRarity Rare\n# note here\nHide\n";
        let mut d = p(src);
        let b = d.blocks().next().unwrap();
        assert_eq!(b.owned(), 0..2);
        assert!(b.can_enable());
        let id = b.id();
        d.set_enabled(id, true).unwrap();
        assert_eq!(d.serialize(), "Show\n\tRarity Rare\n# note here\nHide\n");
        assert_eq!(d.block(id).unwrap().owned(), 0..2);
        assert!(d.diagnostics().is_empty());
    }

    #[test]
    fn prose_between_conditions_refuses_enable() {
        // Iteration-3 CR-01: when a clean commented condition follows the prose that ended the
        // block, enabling would leave it behind and write a broader live rule than the text (for
        // `Hide`, hiding more loot). The block keeps its extent but is Invalid, and the enable is
        // refused with the bytes untouched.
        for (src, line) in [
            ("#Show\n#\tRarity Rare\n# note here\n#\tClass \"Bows\"\n", 4),
            ("#Hide\n#\tRarity Normal\n# only these bases\n#\tBaseType == \"X\"\n", 4),
            ("#Show\n#\tRarity Rare\n# note\n#\n#    Class \"Bows\"\nHide\n", 5),
            ("#Show\n#\tRarity Rare\n# $tag\n## x\n#Class \"Bows\"\n", 5),
        ] {
            let mut d = p(src);
            let b = d.blocks().next().unwrap();
            assert_eq!(b.owned(), 0..2, "{src:?}");
            match b.disabled_state().unwrap() {
                DisabledState::Invalid(r) => assert_eq!(r[0].line, line, "{src:?}"),
                other => panic!("{src:?}: {other:?}"),
            }
            assert!(!b.can_enable(), "{src:?}");
            let id = b.id();
            assert!(d.set_enabled(id, true).is_err(), "{src:?}");
            assert_eq!(d.serialize(), src);
        }
        // A `#ff:` line or a header ends the look-ahead: what follows belongs to the next block.
        for src in [
            "#Show\n#\tRarity Rare\n# note\n#ff:name x\n#\tClass \"Bows\"\nHide\n",
            "#Show\n#\tRarity Rare\n# note\n#Hide\n#\tClass \"Bows\"\n",
            "#Show\n#\tRarity Rare\n# note\n\n#\tClass \"Bows\"\n",
        ] {
            assert!(p(src).blocks().next().unwrap().can_enable(), "{src:?}");
        }
    }

    #[test]
    fn mixed_indentation_enables_whole_rule() {
        // Iteration-3 CR-01: a clean commented condition extends the block at any indentation, so
        // enabling writes every condition the user wrote.
        for (src, enabled) in [
            (
                "#Show\n#\tRarity Rare\n#    Class \"Bows\"\n",
                "Show\n\tRarity Rare\n    Class \"Bows\"\n",
            ),
            (
                "#Show\n#Rarity Rare\n#\tClass \"Bows\"\n",
                "Show\nRarity Rare\n\tClass \"Bows\"\n",
            ),
            (
                "#Show\n#  Rarity Rare\n#\tClass \"Bows\"\n#Sockets >= 2\nHide\n",
                "Show\n  Rarity Rare\n\tClass \"Bows\"\nSockets >= 2\nHide\n",
            ),
        ] {
            let mut d = p(src);
            let b = d.blocks().next().unwrap();
            let id = b.id();
            assert!(b.can_enable(), "{src:?}");
            let n = b.body().len();
            d.set_enabled(id, true)
                .unwrap_or_else(|e| panic!("{src:?}: {e}"));
            assert_eq!(d.serialize(), enabled);
            assert_eq!(d.block(id).unwrap().body().len(), n);
            assert!(d.diagnostics().is_empty(), "{src:?}");
            d.set_enabled(id, false)
                .unwrap_or_else(|e| panic!("{src:?}: {e}"));
            assert_eq!(d.serialize(), src);
        }
    }

    #[test]
    fn can_enable_agrees_with_enable() {
        // WR-01 (iteration 1) and iteration-2 WR-04: can_enable() is true exactly when
        // set_enabled(true) succeeds, and a refused enable leaves the bytes untouched.
        for (src, owned, enableable) in [
            // Iteration-3 CR-01: a clean condition below the prose that ended the block refuses.
            ("#Show\n#\tRarity Rare\n# $tag\n#\tClass \"Bows\"\n", 0..2, false),
            ("#Show\n#\tRarity Rare\n#\t123\n#\tClass \"Bows\"\n", 0..4, false),
            ("#Show\n#\tRarity Rare\n#Import \"x\"\n#\tClass \"Bows\"\n", 0..2, false),
            ("#Show\n#\tRarity Rare\n#Show Continue\n#\tClass \"Bows\"\n", 0..2, false),
            ("#Show\n#\tRarity Rare\n# note\n#\tClass \"Bows\"\nHide\n", 0..2, false),
            ("#Show\n#\tRarity Rare\n# $tag\nHide\n", 0..2, true),
            ("#Show\n#\tRarity Rare\n#Import \"x\"\nHide\n", 0..2, true),
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
        // Directly under a header (no body yet), a line with a problem is never a body line.
        let d = p("#Show\n# Corrupted items\nShow\n");
        assert_eq!(d.blocks().next().unwrap().owned(), 0..1);
        assert!(d.blocks().next().unwrap().can_enable());
    }

    #[test]
    fn header_only_disabled_block_leaves_heading_and_never_enables_as_catch_all() {
        // Iteration-3 WR-02: the heading below a disabled header-only block stays the next
        // block's leading comment. When that line is body-indented it may instead be the rule's
        // broken first condition, and enabled the bare header would match every item, so the
        // block is Invalid: never enabled as a catch-all.
        for (src, enableable) in [
            ("#Show\n#  Note: these are weapons\nHide\n", false),
            ("#Show\n#\tSee notes below\nHide\n", false),
            ("#Show # catch-all\n# Rarity is irrelevant here\nHide\n", true),
            ("#Hide\n#\tFutureKw 3\nShow\n", false),
            ("#Show\n# Note: weapons\nHide\n", true),
            ("#Show\n#Note: weapons\nHide\n", true),
        ] {
            let mut d = p(src);
            let b: Vec<_> = d.blocks().collect();
            assert_eq!(b[0].owned(), 0..1, "{src:?}");
            assert_eq!(b[1].leading(), 1..2, "{src:?}");
            assert_eq!(b[0].can_enable(), enableable, "{src:?}");
            let id = b[0].id();
            assert_eq!(d.set_enabled(id, true).is_ok(), enableable, "{src:?}");
            if !enableable {
                assert_eq!(d.serialize(), src);
            }
        }
    }

    #[test]
    fn ff_line_is_never_pulled_into_a_block() {
        // Iteration-2 WR-02: a `#ff:` line belongs to the header run of the block below it, even
        // when a commented body line follows it, in the disabled and the live case alike.
        for src in [
            "#Show\n#\tRarity Rare\n#ff:name x\n#\tSetFontSize 40\nHide\n",
            "Show\n\tRarity Rare\n#ff:name x\n#\tSetFontSize 40\nHide\n",
            "#Show\n#\tRarity Rare\n  #ff:name x\n#\tSetFontSize 40\nHide\n",
        ] {
            let d = p(src);
            let b: Vec<_> = d.blocks().collect();
            assert_eq!(b[0].owned(), 0..2, "{src:?}");
            assert!(b[0].is_enabled() || b[0].can_enable(), "{src:?}");
            assert_eq!(b[1].leading(), 2..4, "{src:?}");
            assert_eq!(d.meta_get(b[1].id(), "name"), Some("x"), "{src:?}");
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
