//! Classification and grouping: turns raw lines into [`Item`]s plus line-tied diagnostics.
//!
//! This layer is read-only over the stored lines (D-01): it never writes to them, so
//! serialization stays the raw lines. Grammar problems become diagnostics, never errors (D-10,
//! D-12). Comments are ordinary comments; only GGG's grammar is interpreted (D-04).

use super::block::{Block, BlockId, BlockKeyword, BodyLine, DisabledState, Item};
use super::diag::{FilterDiagnostic, Severity};
use super::disabled;
use super::keywords::{lookup, Keyword, Kind};
use super::lines::Line;
use super::tokenize::{tokenize, validate_shape, TokenError, TokenizedLine};

pub(crate) enum Class {
    Blank,
    Comment,
    Kw {
        t: TokenizedLine,
        kw: Option<&'static Keyword>,
    },
    Bad(TokenError),
}

pub(crate) fn is_comment_line(text: &str) -> bool {
    text.trim_start_matches([' ', '\t']).starts_with('#')
}

pub(crate) fn classify(text: &str) -> Class {
    match tokenize(text) {
        Err(e) => Class::Bad(e),
        Ok(t) => match &t.keyword {
            Some(r) => {
                let kw = lookup(&text[r.clone()]);
                Class::Kw { t, kw }
            }
            None if t.comment.is_some() => Class::Comment,
            None => Class::Blank,
        },
    }
}

pub(crate) fn block_keyword(kw: &Keyword) -> BlockKeyword {
    if kw.name.eq_ignore_ascii_case("hide") {
        BlockKeyword::Hide
    } else if kw.name.eq_ignore_ascii_case("minimal") {
        BlockKeyword::Minimal
    } else {
        BlockKeyword::Show
    }
}

fn diag(line0: usize, severity: Severity, message: String) -> FilterDiagnostic {
    FilterDiagnostic {
        line: line0 + 1,
        severity,
        message,
    }
}

fn flush(items: &mut Vec<Item>, from: usize, to: usize) {
    if from < to {
        items.push(Item::Trivia(from..to));
    }
}

/// Start of the maximal run of comment lines directly above `header`, not reaching below `floor`.
fn lead_start(ls: &[Line], floor: usize, header: usize) -> usize {
    let mut j = header;
    while j > floor && is_comment_line(ls[j - 1].text()) {
        j -= 1;
    }
    j
}

pub(crate) fn build(ls: &[Line], next_id: &mut u64) -> (Vec<Item>, Vec<FilterDiagnostic>) {
    let n = ls.len();
    let mut items: Vec<Item> = Vec::new();
    let mut diags: Vec<FilterDiagnostic> = Vec::new();
    let mut pending = 0usize;
    let mut i = 0usize;

    while i < n {
        let text = ls[i].text();

        // Disabled (commented-out) block?
        if is_comment_line(text) {
            if let Some(d) = disabled::detect(ls, i) {
                let start = lead_start(ls, pending, i);
                flush(&mut items, pending, start);
                if let DisabledState::Invalid(reasons) = &d.state {
                    for r in reasons {
                        diags.push(FilterDiagnostic {
                            line: r.line,
                            severity: Severity::Info,
                            message: format!("disabled block cannot be enabled: {}", r.message),
                        });
                    }
                }
                let id = BlockId(*next_id);
                *next_id += 1;
                items.push(Item::Block(Block {
                    id,
                    keyword: d.keyword,
                    disabled: Some(d.state),
                    lead_start: start,
                    header: i,
                    end: d.end,
                    body: d.body,
                }));
                pending = d.end;
                i = d.end;
                continue;
            }
            i += 1;
            continue;
        }

        match classify(text) {
            Class::Blank | Class::Comment => {
                i += 1;
            }
            Class::Bad(e) => {
                flush(&mut items, pending, i);
                diags.push(diag(i, Severity::Error, e.message));
                items.push(Item::Opaque(i));
                i += 1;
                pending = i;
            }
            Class::Kw { t, kw } => match kw {
                None => {
                    flush(&mut items, pending, i);
                    let name = &text[t.keyword.clone().unwrap_or(0..0)];
                    diags.push(diag(
                        i,
                        Severity::Warning,
                        format!("unknown keyword `{name}` kept as-is"),
                    ));
                    items.push(Item::Opaque(i));
                    i += 1;
                    pending = i;
                }
                Some(k) if k.kind == Kind::BlockHeader => {
                    let start = lead_start(ls, pending, i);
                    flush(&mut items, pending, start);
                    if t.operator.is_some() || !t.values.is_empty() {
                        diags.push(diag(
                            i,
                            Severity::Warning,
                            "trailing tokens after block header kept as-is".to_string(),
                        ));
                    }
                    let (body, end) = scan_live_body(ls, i, &mut diags);
                    let id = BlockId(*next_id);
                    *next_id += 1;
                    items.push(Item::Block(Block {
                        id,
                        keyword: block_keyword(k),
                        disabled: None,
                        lead_start: start,
                        header: i,
                        end,
                        body,
                    }));
                    pending = end;
                    i = end;
                }
                Some(k) if k.kind == Kind::Directive => {
                    flush(&mut items, pending, i);
                    match validate_shape(k, text, &t) {
                        Ok(()) => {
                            let first = t.values[0].clone();
                            let raw = &text[first.clone()];
                            let path =
                                if raw.len() >= 2 && raw.starts_with('"') && raw.ends_with('"') {
                                    first.start + 1..first.end - 1
                                } else {
                                    first
                                };
                            let optional = t.values.len() == 2;
                            items.push(Item::Import {
                                line: i,
                                path,
                                optional,
                            });
                        }
                        Err(m) => {
                            diags.push(diag(i, Severity::Error, m));
                            items.push(Item::Opaque(i));
                        }
                    }
                    i += 1;
                    pending = i;
                }
                Some(k) => {
                    flush(&mut items, pending, i);
                    diags.push(diag(
                        i,
                        Severity::Warning,
                        format!("`{}` outside any block kept as-is", k.name),
                    ));
                    items.push(Item::Opaque(i));
                    i += 1;
                    pending = i;
                }
            },
        }
    }
    flush(&mut items, pending, n);
    (items, diags)
}

/// Collect the body of the live block whose header is line `h`. Returns the body and the
/// exclusive end (one past the last keyword line, or past a directly following run of
/// commented-out body lines).
fn scan_live_body(
    ls: &[Line],
    h: usize,
    diags: &mut Vec<FilterDiagnostic>,
) -> (Vec<BodyLine>, usize) {
    let n = ls.len();
    let mut body = Vec::new();
    let mut end = h + 1;
    let mut k = h + 1;
    while k < n {
        let text = ls[k].text();
        if is_comment_line(text) {
            if disabled::is_header(text) {
                break;
            }
            k += 1;
            continue;
        }
        match classify(text) {
            Class::Blank | Class::Comment => {}
            Class::Bad(e) => {
                diags.push(diag(k, Severity::Error, e.message));
                body.push(BodyLine {
                    line: k,
                    keyword: None,
                    tokens: None,
                });
                end = k + 1;
            }
            Class::Kw { t, kw } => match kw {
                None => {
                    let name = &text[t.keyword.clone().unwrap_or(0..0)];
                    diags.push(diag(
                        k,
                        Severity::Warning,
                        format!("unknown keyword `{name}` kept as-is"),
                    ));
                    body.push(BodyLine {
                        line: k,
                        keyword: None,
                        tokens: Some(t),
                    });
                    end = k + 1;
                }
                Some(kw) if matches!(kw.kind, Kind::BlockHeader | Kind::Directive) => break,
                Some(kw) => {
                    if let Err(m) = validate_shape(kw, text, &t) {
                        diags.push(diag(k, Severity::Error, m));
                    }
                    body.push(BodyLine {
                        line: k,
                        keyword: Some(kw),
                        tokens: Some(t),
                    });
                    end = k + 1;
                }
            },
        }
        k += 1;
    }
    // A commented-out body line (`#\tSetFontSize 40`) directly after the last keyword line belongs
    // to this block, not to the next block's leading comments (WR-03). `disabled::detect` applies
    // the same rule one `#` deeper, so disabling and enabling keep the same extent.
    while end < n && disabled::is_commented_body(ls[end].text(), end) {
        end += 1;
    }
    (body, end)
}

#[cfg(test)]
pub(crate) fn assert_partition(items: &[Item], len: usize) {
    let mut next = 0usize;
    for it in items {
        let r = match it {
            Item::Trivia(r) => r.clone(),
            Item::Block(b) => b.span(),
            Item::Import { line, .. } | Item::Opaque(line) => *line..*line + 1,
        };
        assert_eq!(r.start, next, "gap or overlap before {it:?}");
        assert!(r.end > r.start, "empty item {it:?}");
        next = r.end;
    }
    assert_eq!(next, len, "items do not cover all lines");
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::filter::{parse, Document};

    fn p(s: &str) -> Document {
        parse(s.as_bytes()).unwrap()
    }
    fn blocks(d: &Document) -> Vec<&Block> {
        d.blocks().collect()
    }
    fn count(d: &Document, sev: Severity) -> usize {
        d.diagnostics().iter().filter(|x| x.severity == sev).count()
    }

    #[test]
    fn simple_block() {
        let d = p("Show\n\tRarity Rare\n");
        let b = blocks(&d);
        assert_eq!(b.len(), 1);
        assert_eq!(b[0].keyword(), BlockKeyword::Show);
        assert!(b[0].is_enabled());
        assert_eq!(b[0].header_line(), 0);
        assert_eq!(b[0].owned(), 0..2);
        assert!(d.diagnostics().is_empty());
    }

    #[test]
    fn leading_comments_attach() {
        let d = p("# a\n# b\nShow\n\tRarity Rare\n");
        assert_eq!(blocks(&d)[0].leading(), 0..2);
        assert_eq!(blocks(&d)[0].span(), 0..4);
    }

    #[test]
    fn blank_line_breaks_attachment() {
        let d = p("# a\n\nShow\n");
        assert!(matches!(d.items()[0], Item::Trivia(ref r) if *r == (0..2)));
        assert!(blocks(&d)[0].leading().is_empty());
    }

    #[test]
    fn trivia_between_blocks() {
        let d = p("Show\n\tRarity Rare\n\n# next\nHide\n");
        let b = blocks(&d);
        assert_eq!(b[0].owned(), 0..2);
        assert!(matches!(d.items()[1], Item::Trivia(ref r) if *r == (2..3)));
        assert_eq!(b[1].leading(), 3..4);
        assert_eq!(b[1].keyword(), BlockKeyword::Hide);
    }

    #[test]
    fn interior_comments_belong_to_block() {
        let d = p("Show\n\tRarity Rare\n# mid\n\n\tClass \"Bows\"\n");
        assert_eq!(blocks(&d)[0].owned(), 0..5);
        assert_eq!(blocks(&d)[0].body().len(), 2);
    }

    #[test]
    fn trailing_commented_condition_stays_with_its_block() {
        // WR-03 (1): `#\tSetFontSize 40` belongs to Show, not to Hide's leading comments.
        let d = p("Show\n\tRarity Rare\n#\tSetFontSize 40\nHide\n");
        let b = blocks(&d);
        assert_eq!(b[0].owned(), 0..3);
        assert_eq!(b[1].span(), 3..4);
        assert_eq!(b[0].body().len(), 1);
        // Prose and a blank line still end the run.
        let d = p("Show\n\tRarity Rare\n# about Hide\nHide\n");
        assert_eq!(blocks(&d)[0].owned(), 0..2);
        assert_eq!(blocks(&d)[1].leading(), 2..3);
        let d = p("Show\n\tRarity Rare\n\n#\tSetFontSize 40\nHide\n");
        assert_eq!(blocks(&d)[0].owned(), 0..2);
        assert_eq!(blocks(&d)[1].leading(), 3..4);
    }

    #[test]
    fn disabled_block_does_not_absorb_prose_with_grammar_word() {
        // WR-03 (2): `# Class weapons below` is Hide's leading comment, not a disabled condition.
        let d = p("#Show\n#\tRarity Rare\n# Class weapons below\nHide\n");
        let b = blocks(&d);
        assert_eq!(b[0].owned(), 0..2);
        assert!(b[0].can_enable());
        assert_eq!(b[1].leading(), 2..3);
        // Indented like a body, the same words are grammar.
        let d = p("#Show\n#\tRarity Rare\n#\tClass Rings Amulets\n");
        assert_eq!(blocks(&d)[0].owned(), 0..3);
        assert!(blocks(&d)[0].can_enable());
    }

    #[test]
    fn continue_is_body_modifier() {
        let d = p("Show\n\tContinue\n");
        let b = blocks(&d)[0];
        assert!(b.has_continue());
        assert_eq!(b.body().len(), 1);
        assert_eq!(b.body()[0].keyword.unwrap().kind, Kind::Modifier);
    }

    #[test]
    fn unknown_keyword_is_warning() {
        let d = p("Show\n\tFutureKeyword 3\n\tRarity Rare\n");
        assert_eq!(d.diagnostics().len(), 1);
        assert_eq!(d.diagnostics()[0].line, 2);
        assert_eq!(d.diagnostics()[0].severity, Severity::Warning);
        assert!(d.diagnostics()[0]
            .message
            .contains("unknown keyword `FutureKeyword`"));
        assert_eq!(blocks(&d)[0].body().len(), 2);
        assert!(blocks(&d)[0].is_enabled());
        assert_eq!(count(&d, Severity::Error), 0);
    }

    #[test]
    fn bad_operator_is_error_and_bytes_kept() {
        let src = "Show\n\tRarity =< Rare\n";
        let d = p(src);
        assert_eq!(d.diagnostics().len(), 1);
        assert_eq!(d.diagnostics()[0].severity, Severity::Error);
        assert_eq!(d.diagnostics()[0].line, 2);
        assert!(d.diagnostics()[0].message.contains("unknown operator"));
        assert_eq!(blocks(&d).len(), 1);
        assert_eq!(d.to_bytes(), src.as_bytes());
    }

    #[test]
    fn import_item() {
        let d = p("Import \"base.filter\" Optional\n");
        match &d.items()[0] {
            Item::Import {
                line,
                path,
                optional,
            } => {
                assert_eq!(*line, 0);
                assert_eq!(&d.line_text(0)[path.clone()], "base.filter");
                assert!(*optional);
            }
            other => panic!("{other:?}"),
        }
        assert!(d.diagnostics().is_empty());
    }

    #[test]
    fn condition_before_header_is_opaque() {
        let d = p("\tRarity Rare\nShow\n");
        assert!(matches!(d.items()[0], Item::Opaque(0)));
        assert_eq!(d.diagnostics()[0].severity, Severity::Warning);
        assert_eq!(d.diagnostics()[0].line, 1);
    }

    #[test]
    fn compact_header_warns() {
        let d = p("Hide ItemLevel < 85 Continue\n");
        assert_eq!(blocks(&d).len(), 1);
        assert_eq!(d.diagnostics().len(), 1);
        assert_eq!(d.diagnostics()[0].severity, Severity::Warning);
        assert_eq!(d.diagnostics()[0].line, 1);
    }

    #[test]
    fn case_insensitive() {
        let d = p("show\n\trarity>=magic\n");
        assert_eq!(blocks(&d).len(), 1);
        assert!(d.diagnostics().is_empty());
    }

    #[test]
    fn ff_metadata() {
        let d = p("#ff:name Exceptional 3 socks\n#ff:style S\nShow # $type->x\n");
        let id = blocks(&d)[0].id();
        assert_eq!(d.meta_get(id, "name"), Some("Exceptional 3 socks"));
        assert_eq!(d.meta_get(id, "style"), Some("S"));
        assert_eq!(d.block_meta(id).len(), 2);
        let d = p("Show # $type->x\n");
        let id = blocks(&d)[0].id();
        assert_eq!(d.meta_get(id, "name"), None);
    }

    #[test]
    fn ff_marker_is_trivia() {
        let d = p("#ff:economy-begin\n\nShow\n");
        assert!(matches!(d.items()[0], Item::Trivia(_)));
        assert_eq!(d.line_at(0).ff_marker(), Some(("economy-begin", None)));
    }

    #[test]
    fn items_partition_all_lines() {
        let src = "# banner\n\n#ff:name x\nShow\n\tRarity Rare\n\n\tFoo 1\n# t\n\nImport \"a\"\n\tRarity Rare\n#Show\n#\tClass \"Bows\"\n# end\nHide\r\n";
        let d = p(src);
        assert_partition(d.items(), d.line_count());
        assert_eq!(d.to_bytes(), src.as_bytes());
    }

    #[test]
    fn ids_are_unique_and_increasing() {
        let d = p("Show\nHide\nMinimal\nShow\n");
        let ids: Vec<u64> = d.blocks().map(|b| b.id().0).collect();
        assert_eq!(ids, vec![0, 1, 2, 3]);
        assert_eq!(d.block(blocks(&d)[2].id()).unwrap().id().0, 2);
    }
}
