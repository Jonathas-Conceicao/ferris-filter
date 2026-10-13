//! Styled-rule rewrite (STYLE-03 / EDIT-06).
//!
//! A rule tagged `#ff:style <name>` for a known style gets that style's action lines and its
//! Show/Hide keyword whenever [`rewrite_styled`] runs (on Apply). Pure and library-level: no IO,
//! no UI, so a later style editor can reuse it. It lives outside `filter` because `filter` must
//! not depend on `models::config`.
//!
//! Rules: disabled blocks and economy-section blocks are never touched; an unknown style name is
//! reported by `rewrite_styled` (its rule keeps its actions) and its tag is removed by
//! [`drop_unknown_style_tags`], which callers run after `rewrite_styled` on Apply (CONTEXT
//! "Deleting"); a rule that already matches its style is not edited,
//! which keeps the pass idempotent.

use crate::emitter::style_action_lines;
use crate::filter::keywords::Kind;
use crate::filter::section::{self, SectionSpan};
use crate::filter::tokenize::tokenize;
use crate::filter::{BlockId, BlockKeyword, Document, EditError};
use crate::models::config::{Style, StyleAction, StyleSet};

/// Outcome of one [`rewrite_styled`] pass.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct StyleReport {
    /// Rules whose bytes were changed.
    pub rewritten: usize,
    /// `(1-based rule index among non-economy blocks, style name)` for unknown style tags.
    pub unknown: Vec<(usize, String)>,
    /// `(1-based rule index, error text)` for known-style rules that could not be rewritten
    /// (e.g. a compact header such as `Hide Continue`). Such a rule keeps its current actions,
    /// like an unknown style, instead of blocking the whole pass.
    pub failed: Vec<(usize, String)>,
    /// `(1-based rule index, style name)` for rules whose `#ff:style` tag was removed because the
    /// style no longer exists. [`rewrite_styled`] never fills it; [`drop_unknown_style_tags`] does.
    pub dropped: Vec<(usize, String)>,
}

/// False when `name` is empty, padded with whitespace or holds a control character: such a name
/// would be trimmed by the tag reader and read back as a different (unknown) style.
pub fn style_name_ok(name: &str) -> bool {
    !name.is_empty() && name == name.trim() && !name.chars().any(char::is_control)
}

fn economy_span(doc: &Document) -> Option<SectionSpan> {
    section::find(doc).ok().flatten()
}

fn in_economy(span: Option<SectionSpan>, header: usize) -> bool {
    span.is_some_and(|s| header > s.begin && header < s.end)
}

/// 1-based index among non-economy blocks (same numbering as the overlay's rule rows).
pub fn rule_index_of(doc: &Document, id: BlockId) -> Option<usize> {
    let span = economy_span(doc);
    let mut index = 0usize;
    for b in doc.blocks() {
        if in_economy(span, b.header_line()) {
            continue;
        }
        index += 1;
        if b.id() == id {
            return Some(index);
        }
    }
    None
}

/// `(lowercased keyword, value tokens joined by one space)` for an action line, else `None`.
fn normalize(text: &str) -> Option<String> {
    let t = tokenize(text).ok()?;
    let kw = t.keyword.clone()?;
    let mut out = text[kw].to_ascii_lowercase();
    for v in &t.values {
        out.push(' ');
        out.push_str(&text[v.clone()]);
    }
    Some(out)
}

/// Rewrite one block to `style`. `Ok(false)` when it already matches semantically.
pub fn rewrite_block(doc: &mut Document, id: BlockId, style: &Style) -> Result<bool, EditError> {
    let targets = style_action_lines(style);
    let want_kw = match style.action {
        StyleAction::Show => BlockKeyword::Show,
        StyleAction::Hide => BlockKeyword::Hide,
    };
    let block = doc.block(id).ok_or(EditError::UnknownBlock)?;
    let action_lines: Vec<usize> = block
        .body()
        .iter()
        .filter(|bl| bl.keyword.is_some_and(|k| k.kind == Kind::Action))
        .map(|bl| bl.line)
        .collect();

    let mut have: Vec<Option<String>> = action_lines
        .iter()
        .map(|&i| normalize(doc.lines()[i].text()))
        .collect();
    let mut want: Vec<Option<String>> = targets.iter().map(|t| normalize(t)).collect();
    have.sort();
    want.sort();
    if block.keyword() == want_kw && have == want {
        return Ok(false);
    }

    let header = block.header_line();
    let last_body = block.body().last().map(|bl| bl.line);
    let after = match action_lines.first() {
        Some(&first) => first - 1,
        None => last_body.unwrap_or(header),
    };
    // Keep the block all-or-nothing. A refused `splice_body` validates before it writes, so the
    // only mutation that can need undoing is the keyword switch. Undo it by restoring a snapshot,
    // not by writing the old keyword back: that would put the canonical `Hide` over a header
    // spelled `hide` (IN-05). The snapshot is taken only when the keyword actually changes.
    let snapshot = (block.keyword() != want_kw).then(|| doc.clone());
    if snapshot.is_some() {
        doc.set_block_keyword(id, want_kw)?;
    }
    if let Err(e) = doc.splice_body(id, &action_lines, after, &targets) {
        if let Some(snap) = snapshot {
            *doc = snap;
        }
        return Err(e);
    }
    Ok(true)
}

/// Rewrite every enabled, non-economy rule whose `#ff:style` names a known style.
///
/// A rule whose rewrite is refused is left byte-identical (`rewrite_block` is all-or-nothing)
/// and listed in [`StyleReport::failed`]; the pass goes on with the other rules.
pub fn rewrite_styled(doc: &mut Document, styles: &StyleSet) -> StyleReport {
    let span = economy_span(doc);
    let ids: Vec<(BlockId, bool, usize)> = doc
        .blocks()
        .map(|b| (b.id(), b.is_enabled(), b.header_line()))
        .collect();
    let mut report = StyleReport::default();
    for (id, enabled, header) in ids {
        if !enabled || in_economy(span, header) {
            continue;
        }
        let Some(name) = doc.meta_get(id, "style").map(str::to_string) else {
            continue;
        };
        match styles.get(&name) {
            Some(style) => match rewrite_block(doc, id, style) {
                Ok(true) => report.rewritten += 1,
                Ok(false) => {}
                Err(e) => {
                    if let Some(i) = rule_index_of(doc, id) {
                        report.failed.push((i, e.to_string()));
                    }
                }
            },
            None => {
                if let Some(i) = rule_index_of(doc, id) {
                    report.unknown.push((i, name));
                }
            }
        }
    }
    report
}

/// Remove the `#ff:style` tag from every non-economy rule (enabled or disabled) whose style is
/// not in `styles`; the rule keeps its current actions (CONTEXT "Deleting"). Returns
/// `(1-based rule index, style name)` per removed tag. Callers run it after [`rewrite_styled`]
/// and only when a StyleSet actually loaded. A block whose tag cannot be removed is skipped.
pub fn drop_unknown_style_tags(doc: &mut Document, styles: &StyleSet) -> Vec<(usize, String)> {
    let span = economy_span(doc);
    let ids: Vec<(BlockId, usize)> = doc
        .blocks()
        .map(|b| (b.id(), b.header_line()))
        .filter(|&(_, header)| !in_economy(span, header))
        .collect();
    let mut dropped = Vec::new();
    for (id, _) in ids {
        let Some(name) = doc.meta_get(id, "style").map(str::to_string) else {
            continue;
        };
        if styles.get(&name).is_some() {
            continue;
        }
        let Some(index) = rule_index_of(doc, id) else {
            continue;
        };
        if doc.remove_meta(id, "style").is_ok() {
            dropped.push((index, name));
        }
    }
    dropped
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::filter::parse;

    fn styles() -> StyleSet {
        crate::loader::load_styles(concat!(env!("CARGO_MANIFEST_DIR"), "/styles.json")).unwrap()
    }

    fn run(src: &str) -> (String, StyleReport) {
        let mut doc = parse(src.as_bytes()).unwrap();
        let rep = rewrite_styled(&mut doc, &styles());
        (doc.serialize(), rep)
    }

    const S_LINES: &str = "  PlayEffect Pink\n  MinimapIcon 1 Pink Circle\n  SetTextColor 255 121 198 255\n  SetBackgroundColor 40 42 54 255\n  SetBorderColor 255 121 198 255\n  SetFontSize 37\n  CustomAlertSound \"uwu.mp3\" 300\n";

    #[test]
    fn rewrites_actions_and_keyword_keeping_conditions() {
        let src = "#ff:style S\nHide\n  Class \"Rings\"\n  SetFontSize 20\n  Continue\n";
        let (out, rep) = run(src);
        let expect = format!("#ff:style S\nShow\n  Class \"Rings\"\n{S_LINES}  Continue\n");
        assert_eq!(out, expect);
        assert_eq!(rep.rewritten, 1);
        assert!(rep.unknown.is_empty());
    }

    #[test]
    fn appends_when_no_actions() {
        let src = "#ff:style hidden\nShow\n  Class \"Rings\"\n";
        let (out, rep) = run(src);
        assert_eq!(
            out,
            "#ff:style hidden\nHide\n  Class \"Rings\"\n  SetFontSize 21\n  DisableDropSound\n"
        );
        assert_eq!(rep.rewritten, 1);
    }

    #[test]
    fn second_run_is_idempotent() {
        let src = "#ff:style S\nHide\n  Class \"Rings\"\n  SetFontSize 20\n";
        let (once, _) = run(src);
        let (twice, rep) = run(&once);
        assert_eq!(once, twice);
        assert_eq!(rep.rewritten, 0);
    }

    #[test]
    fn semantically_equal_block_is_not_rewritten() {
        let src = "#ff:style hidden\nHide\n\tDisableDropSound\n\tSetFontSize   21\n";
        let (out, rep) = run(src);
        assert_eq!(out, src);
        assert_eq!(rep.rewritten, 0);
    }

    #[test]
    fn unknown_style_is_reported_and_untouched() {
        let src = "Show\n  Class \"Bows\"\n\n#ff:style Nope\nHide\n  SetFontSize 20\n";
        let (out, rep) = run(src);
        assert_eq!(out, src);
        assert_eq!(rep.rewritten, 0);
        assert_eq!(rep.unknown, vec![(2, "Nope".to_string())]);
    }

    /// WR-02: one rule that cannot be rewritten is reported by index and left untouched, and the
    /// other styled rules are still rewritten.
    #[test]
    fn refused_rewrite_is_reported_and_does_not_block_others() {
        let src = "#ff:style S\nHide Continue\n  SetFontSize 20\n\n#ff:style hidden\nShow\n  Class \"Rings\"\n";
        let (out, rep) = run(src);
        assert_eq!(rep.rewritten, 1);
        assert_eq!(rep.failed.len(), 1, "{rep:?}");
        assert_eq!(rep.failed[0].0, 1);
        assert!(out.starts_with("#ff:style S\nHide Continue\n  SetFontSize 20\n\n"));
        assert!(out.ends_with(
            "#ff:style hidden\nHide\n  Class \"Rings\"\n  SetFontSize 21\n  DisableDropSound\n"
        ));
    }

    /// IN-05: a refused rewrite after the keyword switch restores the original bytes, so a
    /// lowercase `hide` header is not canonicalised to `Hide` and the pass still reports the rule
    /// instead of failing the whole Apply.
    #[test]
    fn refused_rewrite_keeps_a_lowercase_keyword() {
        // A sound path with an embedded newline makes `splice_body` refuse the action lines
        // after the `hide` -> `Show` switch has already succeeded.
        let mut bad = styles().get("S").unwrap().clone();
        bad.sound.as_mut().unwrap().custom = Some("a\nb.mp3".into());
        let src = "#ff:style S\nhide\n  SetFontSize 20\n";
        let mut doc = parse(src.as_bytes()).unwrap();
        let id = doc.blocks().next().unwrap().id();
        assert!(rewrite_block(&mut doc, id, &bad).is_err());
        assert_eq!(
            doc.serialize(),
            src,
            "a refused rewrite restores the exact bytes"
        );
    }

    #[test]
    fn disabled_block_is_untouched() {
        let src = "#ff:style S\n#Hide\n#  Class \"Rings\"\n#  SetFontSize 20\n";
        let (out, rep) = run(src);
        assert_eq!(out, src);
        assert_eq!(rep.rewritten, 0);
    }

    #[test]
    fn economy_block_is_untouched() {
        let src = format!(
            "{}\n#ff:style S\nHide\n  SetFontSize 20\n{}\n",
            section::BEGIN_LINE,
            section::END_LINE
        );
        let (out, rep) = run(&src);
        assert_eq!(out, src);
        assert_eq!(rep.rewritten, 0);
    }

    #[test]
    fn unknown_keyword_line_does_not_block_rewrite() {
        let src = "#ff:style hidden\nShow\n  Class \"Rings\"\n  FutureThing 3\n";
        let (out, rep) = run(src);
        assert!(out.contains("FutureThing 3"));
        assert!(out.starts_with("#ff:style hidden\nHide\n"));
        assert_eq!(rep.rewritten, 1);
    }

    #[test]
    fn mixed_eols_keep_non_action_lines() {
        let src = "#ff:style S\r\nHide\r\n  Class \"Rings\"\r\n  SetFontSize 20\n  Rarity Rare\n";
        let (out, rep) = run(src);
        assert_eq!(rep.rewritten, 1);
        assert!(out.starts_with("#ff:style S\r\nShow\r\n  Class \"Rings\"\r\n"));
        assert!(out.ends_with("  Rarity Rare\n"));
        assert!(!out.contains("SetFontSize 20"));
    }

    #[test]
    fn unchanged_style_removes_actions_and_shows() {
        let src =
            "#ff:style unchanged\nHide\n  SetFontSize 21\n  DisableDropSound\n  PlayEffect Red\n";
        let (out, rep) = run(src);
        assert_eq!(out, "#ff:style unchanged\nShow\n");
        assert_eq!(rep.rewritten, 1);
    }

    fn drop_run(src: &str) -> (String, Vec<(usize, String)>) {
        let mut doc = parse(src.as_bytes()).unwrap();
        let d = drop_unknown_style_tags(&mut doc, &styles());
        (doc.serialize(), d)
    }

    #[test]
    fn drop_removes_unknown_tag_keeps_actions_and_other_meta() {
        let src = "#ff:style S\nShow\n  Class \"A\"\n\n#ff:style Gone\n#ff:name Rings\nShow\n  Class \"Rings\"\n  SetFontSize 40\n";
        let (out, d) = drop_run(src);
        assert_eq!(d, vec![(2, "Gone".to_string())]);
        assert!(!out.contains("Gone"));
        assert!(out.contains("#ff:style S\n"));
        assert!(out.contains("#ff:name Rings"));
        assert!(out.contains("  Class \"Rings\"\n  SetFontSize 40\n"));
        let (again, d2) = drop_run(&out);
        assert!(d2.is_empty());
        assert_eq!(again, out);
    }

    #[test]
    fn drop_handles_disabled_block() {
        let src = "#ff:style Gone\n#Show\n#  Class \"Rings\"\n#  SetFontSize 20\n";
        let (out, d) = drop_run(src);
        assert_eq!(d, vec![(1, "Gone".to_string())]);
        assert!(!out.contains("#ff:style"));
        assert!(out.contains("#Show\n#  Class \"Rings\"\n"));
    }

    #[test]
    fn drop_skips_economy_blocks() {
        let src = format!(
            "{}\n#ff:style Gone\nShow\n  SetFontSize 20\n{}\n",
            section::BEGIN_LINE,
            section::END_LINE
        );
        let (out, d) = drop_run(&src);
        assert!(d.is_empty());
        assert_eq!(out, src);
        assert!(StyleReport::default().dropped.is_empty());
    }

    #[test]
    fn style_name_ok_rules() {
        assert!(style_name_ok("S"));
        for bad in ["", " S", "S ", "S\tX"] {
            assert!(!style_name_ok(bad), "{bad:?}");
        }
    }

    #[test]
    fn rule_index_counts_non_economy_blocks() {
        let doc = parse(b"Show\n  Class \"A\"\n\nHide\n  Class \"B\"\n").unwrap();
        let ids: Vec<_> = doc.blocks().map(|b| b.id()).collect();
        assert_eq!(rule_index_of(&doc, ids[0]), Some(1));
        assert_eq!(rule_index_of(&doc, ids[1]), Some(2));
    }
}
