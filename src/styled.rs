//! Styled-rule rewrite (STYLE-03 / EDIT-06).
//!
//! A tag is `# ff:style <id> <name>` (UAT3-02): the id is the identity, the name a readable hint.
//! A legacy tag that is just a style name still resolves by name; `rewrite_styled_with(.., true)`
//! migrates such tags (and stale names after a rename) to the canonical form.
//!
//! A rule tagged `# ff:style <name>` for a known style gets that style's action lines and its
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
use std::collections::HashMap;

use crate::filter::group;
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
    /// `(1-based rule index, style name)` for rules whose `# ff:style` tag was removed because the
    /// style no longer exists. [`rewrite_styled`] never fills it; [`drop_unknown_style_tags`] does.
    pub dropped: Vec<(usize, String)>,
    /// Rules whose `# ff:style` tag line was rewritten to the canonical `<id> <name>` form
    /// (only [`rewrite_styled_with`] with `retag = true` fills it).
    pub retagged: usize,
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

/// 1-based index among non-economy rules (same numbering as the overlay's rule rows: a grouped
/// rule counts once).
pub fn rule_index_of(doc: &Document, id: BlockId) -> Option<usize> {
    rule_indices(doc).get(&id).copied()
}

/// [`rule_index_of`] for every non-economy block at once; a later block of a grouped rule maps to
/// its rule's index. Passes that edit the document compute it first, so a group that is briefly
/// out of step mid-pass never shifts the numbers (review WR-04).
fn rule_indices(doc: &Document) -> HashMap<BlockId, usize> {
    let span = economy_span(doc);
    let groups = group::analyze(doc);
    let mut out = HashMap::new();
    let mut index = 0usize;
    for b in doc.blocks() {
        if in_economy(span, b.header_line()) {
            continue;
        }
        if !groups.is_follower(b.id()) {
            index += 1;
        }
        // Blocks come in file order, so a follower's leader is already numbered: `index`.
        out.insert(b.id(), index);
    }
    out
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

/// Rewrite every enabled, non-economy rule whose `# ff:style` names a known style.
///
/// A rule whose rewrite is refused is left byte-identical (`rewrite_block` is all-or-nothing)
/// and listed in [`StyleReport::failed`]; the pass goes on with the other rules.
pub fn rewrite_styled(doc: &mut Document, styles: &StyleSet) -> StyleReport {
    rewrite_styled_with(doc, styles, false)
}

/// [`rewrite_styled`], optionally migrating tags. With `retag`, every non-economy rule (enabled
/// or disabled) whose tag resolves but is not canonical gets its tag line rewritten to
/// `<id> <name>`; a canonical tag line is never touched. Actions are rewritten for enabled rules
/// only, as before.
pub fn rewrite_styled_with(doc: &mut Document, styles: &StyleSet, retag: bool) -> StyleReport {
    let span = economy_span(doc);
    let ids: Vec<(BlockId, bool, usize)> = doc
        .blocks()
        .map(|b| (b.id(), b.is_enabled(), b.header_line()))
        .collect();
    // Every block of a grouped rule is rewritten (they stay identical), but the rule is reported
    // once, by its first block.
    let followers = group::analyze(doc).followers();
    let indices = rule_indices(doc);
    let mut report = StyleReport::default();
    for (id, enabled, header) in ids {
        if in_economy(span, header) {
            continue;
        }
        let follower = followers.contains(&id);
        let Some(tag) = doc.meta_get(id, "style").map(str::to_string) else {
            continue;
        };
        let Some(m) = styles.resolve_tag(&tag) else {
            if enabled
                && !follower
                && let Some(&i) = indices.get(&id)
            {
                report.unknown.push((i, tag));
            }
            continue;
        };
        if retag
            && !m.canonical
            && let Some(canon) = styles.tag_for(m.name)
            && doc.set_meta(id, "style", &canon).is_ok()
            && !follower
        {
            report.retagged += 1;
        }
        if !enabled {
            continue;
        }
        match rewrite_block(doc, id, m.style) {
            Ok(true) if !follower => report.rewritten += 1,
            Ok(_) => {}
            Err(e) => {
                // A failed later block of a group is reported under its rule, once.
                if let Some(&i) = indices.get(&id)
                    && !report.failed.iter().any(|(f, _)| *f == i)
                {
                    report.failed.push((i, e.to_string()));
                }
            }
        }
    }
    report
}

/// Remove the `# ff:style` tag from every non-economy rule (enabled or disabled) whose style is
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
    let followers = group::analyze(doc).followers();
    let indices = rule_indices(doc);
    let mut dropped = Vec::new();
    for (id, _) in ids {
        let Some(name) = doc.meta_get(id, "style").map(str::to_string) else {
            continue;
        };
        if styles.resolve_tag(&name).is_some() {
            continue;
        }
        if followers.contains(&id) {
            // A later block of a grouped rule: drop its tag too, report the rule once.
            let _ = doc.remove_meta(id, "style");
            continue;
        }
        let Some(&index) = indices.get(&id) else {
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
        crate::loader::load_styles(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/fixtures/seed-styles.json"
        ))
        .unwrap()
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

    /// The cancel values, a positional alert and EnableDropSound pass the grammar gate and a
    /// second run changes nothing.
    #[test]
    fn cancel_values_and_enable_drop_sound_rewrite_idempotently() {
        use crate::models::config::{Cancelable, Sound};
        let mut set = styles();
        let s = set.get_mut("S").unwrap();
        s.minimap = Some(Cancelable::Cancel);
        s.beam = Some(Cancelable::Cancel);
        s.sound = Some(Cancelable::Cancel);
        s.disable_drop_sound = false;
        s.enable_drop_sound = true;
        set.validate().unwrap();
        let src = "#ff:style S\nShow\n  Class \"Rings\"\n  SetFontSize 20\n";
        let (once, rep) = run_with(src, &set, false);
        assert_eq!(rep.rewritten, 1);
        for line in [
            "PlayEffect None",
            "MinimapIcon -1",
            "PlayAlertSound None",
            "EnableDropSound",
        ] {
            assert!(once.contains(&format!("  {line}\n")), "{line} in {once}");
        }
        let (twice, rep) = run_with(&once, &set, false);
        assert_eq!(once, twice);
        assert_eq!(rep.rewritten, 0);
        let s = set.get_mut("S").unwrap();
        s.sound = Some(Cancelable::Set(Sound {
            id: Some(6),
            custom: None,
            vol: 200,
            positional: true,
        }));
        let (once, _) = run_with(src, &set, false);
        assert!(
            once.contains("  PlayAlertSoundPositional 6 200\n"),
            "{once}"
        );
        let (twice, rep) = run_with(&once, &set, false);
        assert_eq!(once, twice);
        assert_eq!(rep.rewritten, 0);
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
        bad.sound
            .as_mut()
            .and_then(crate::models::config::Cancelable::value_mut)
            .unwrap()
            .custom = Some("a\nb.mp3".into());
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
            "{}\n{}\n{}\n#ff:style S\nHide\n  SetFontSize 20\n",
            section::RULE_LINE,
            section::BEGIN_LINE,
            section::RULE_LINE
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
        assert!(out.contains("ff:style S\n"));
        assert!(out.contains("ff:name Rings"));
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
        assert!(!out.contains("ff:style"));
        assert!(out.contains("#Show\n#  Class \"Rings\"\n"));
    }

    #[test]
    fn drop_skips_economy_blocks() {
        let src = format!(
            "{}\n{}\n{}\n#ff:style Gone\nShow\n  SetFontSize 20\n",
            section::RULE_LINE,
            section::BEGIN_LINE,
            section::RULE_LINE
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

    fn run_with(src: &str, set: &StyleSet, retag: bool) -> (String, StyleReport) {
        let mut doc = parse(src.as_bytes()).unwrap();
        let rep = rewrite_styled_with(&mut doc, set, retag);
        (doc.serialize(), rep)
    }

    #[test]
    fn retag_migrates_legacy_tag_and_actions() {
        let src = "#ff:style S\nHide\n  Class \"Rings\"\n  SetFontSize 20\n  Continue\n";
        let (out, rep) = run_with(src, &styles(), true);
        let expect =
            format!("# ff:style builtin:S S\nShow\n  Class \"Rings\"\n{S_LINES}  Continue\n");
        assert_eq!(out, expect);
        assert_eq!(rep.retagged, 1);
        assert_eq!(rep.rewritten, 1);
    }

    #[test]
    fn no_retag_leaves_tag_lines_alone() {
        let src = "#ff:style S\nHide\n  Class \"Rings\"\n";
        let (out, rep) = run_with(src, &styles(), false);
        assert!(out.starts_with("#ff:style S\nShow\n"));
        assert_eq!(rep.retagged, 0);
        let (via_plain, _) = run(src);
        assert_eq!(out, via_plain);
    }

    #[test]
    fn canonical_tag_line_is_untouched() {
        let src = format!("#ff:style builtin:S S\nShow\n  Class \"Rings\"\n{S_LINES}");
        let (out, rep) = run_with(&src, &styles(), true);
        assert_eq!(out, src);
        assert_eq!(rep.retagged, 0);
        assert_eq!(rep.rewritten, 0);
    }

    #[test]
    fn disabled_rule_legacy_tag_is_retagged() {
        let src = "#ff:style hidden\n#Hide\n#  Class \"Rings\"\n";
        let mut doc = parse(src.as_bytes()).unwrap();
        let id = doc.blocks().next().unwrap().id();
        assert!(!doc.block(id).unwrap().is_enabled());
        let rep = rewrite_styled_with(&mut doc, &styles(), true);
        assert_eq!(rep.retagged, 1);
        assert_eq!(doc.meta_get(id, "style"), Some("builtin:hidden hidden"));
    }

    #[test]
    fn unknown_tag_reported_and_untouched_even_when_retagging() {
        let src = "#ff:style zzzzzz kite\nHide\n  SetFontSize 20\n";
        let (out, rep) = run_with(src, &styles(), true);
        assert_eq!(out, src);
        assert_eq!(rep.unknown, vec![(1, "zzzzzz kite".to_string())]);
    }

    #[test]
    fn drop_unknown_keeps_legacy_name_and_id_tags() {
        let src = "#ff:style S\nShow\n\n#ff:style builtin:A A\nShow\n\n#ff:style Nope\nShow\n";
        let mut doc = parse(src.as_bytes()).unwrap();
        let dropped = drop_unknown_style_tags(&mut doc, &styles());
        assert_eq!(dropped, vec![(3, "Nope".to_string())]);
        let out = doc.serialize();
        assert!(out.contains("#ff:style S\n") && out.contains("#ff:style builtin:A A\n"));
        assert!(!out.contains("Nope"));
    }

    #[test]
    fn renamed_style_tag_follows_the_id() {
        let mut set = styles();
        let kite_id = set.id_of("kite").unwrap().to_string();
        set.rename_custom("kite", "locator").unwrap();
        let src = format!("#ff:style {kite_id} kite\nShow\n  Class \"Rings\"\n");
        let (out, rep) = run_with(&src, &set, true);
        assert!(
            out.starts_with(&format!("# ff:style {kite_id} locator\n")),
            "{out}"
        );
        assert_eq!(rep.retagged, 1);
        assert!(rep.unknown.is_empty());
    }
}
