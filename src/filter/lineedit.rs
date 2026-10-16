//! Single-line body edits: patch one line's operator or values, insert or remove body lines.
//!
//! The primitive every condition and action control writes through. Only the touched line's bytes
//! change (PARSE-04): the operator and value byte ranges come from the tokenizer, every other byte
//! of the line (indent, spacing, trailing comment) is kept verbatim, and every other line keeps its
//! own terminator. Each op builds a candidate for the block's owned lines and commits it through
//! [`Document::commit_owned`], so an `Err` leaves the document untouched and an edit that would
//! introduce a new grammar problem is refused.

use super::Document;
use super::block::{BlockId, DisabledState};
use super::disabled;
use super::edit::EditError;
use super::lines::{Eol, Line};
use super::tokenize::{Operator, TokenizedLine, tokenize};

/// One edit to a single body line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LinePatch {
    /// Replace, insert (`Some`) or delete (`None`) the operator.
    Operator(Option<Operator>),
    /// Replace the value span (or insert values after the operator or keyword).
    Values(String),
    /// Operator change then values change, in one validated commit.
    Both(Option<Operator>, String),
    /// Raw edit of everything after the line's indent (and after a disabled block's `#`).
    Whole(String),
    /// Replace the keyword, operator and values (a keyword change such as `PlayAlertSound` to
    /// `CustomAlertSound`), keeping the indent and everything after the last value: its spacing
    /// and a trailing `# comment`. Validated like a value edit.
    Content(String),
}

fn is_ws(b: u8) -> bool {
    b == b' ' || b == b'\t'
}

fn tok(inner: &str) -> Result<TokenizedLine, EditError> {
    let t = tokenize(inner).map_err(|_| EditError::NotABodyLine)?;
    if t.keyword.is_none() {
        return Err(EditError::NotABodyLine);
    }
    Ok(t)
}

fn apply_operator(inner: &str, op: Option<Operator>) -> Result<String, EditError> {
    let t = tok(inner)?;
    let mut out = inner.to_string();
    match (op, t.operator) {
        (Some(op), Some((r, _))) => out.replace_range(r, op.as_str()),
        (Some(op), None) => match t.values.first() {
            Some(v) => {
                let glued = v.start == 0 || !is_ws(inner.as_bytes()[v.start - 1]);
                let ins = if glued {
                    format!(" {} ", op.as_str())
                } else {
                    format!("{} ", op.as_str())
                };
                out.insert_str(v.start, &ins);
            }
            None => {
                let at = t.keyword.map_or(inner.len(), |k| k.end);
                out.insert_str(at, &format!(" {}", op.as_str()));
            }
        },
        (None, Some((r, _))) => {
            let b = inner.as_bytes();
            let mut end = r.end;
            while end < b.len() && is_ws(b[end]) {
                end += 1;
            }
            let before_ws = r.start > 0 && is_ws(b[r.start - 1]);
            let glue = !before_ws && end < b.len();
            out.replace_range(r.start..end, if glue { " " } else { "" });
        }
        (None, None) => {}
    }
    Ok(out)
}

fn apply_values(inner: &str, text: &str) -> Result<String, EditError> {
    let t = tok(inner)?;
    let mut out = inner.to_string();
    match (t.values.first(), t.values.last()) {
        (Some(f), Some(l)) => out.replace_range(f.start..l.end, text),
        _ => {
            let at = t
                .operator
                .map(|(r, _)| r.end)
                .or(t.keyword.map(|k| k.end))
                .unwrap_or(inner.len());
            out.insert_str(at, &format!(" {text}"));
        }
    }
    Ok(out)
}

fn apply_content(inner: &str, text: &str) -> Result<String, EditError> {
    let t = tok(inner)?;
    let start = t.keyword.as_ref().map_or(0, |k| k.start);
    let end = t
        .values
        .last()
        .map(|v| v.end)
        .or(t.operator.as_ref().map(|(r, _)| r.end))
        .or(t.keyword.as_ref().map(|k| k.end))
        .unwrap_or(inner.len());
    let mut out = inner.to_string();
    out.replace_range(start..end, text);
    Ok(out)
}

fn check_text(text: &str) -> Result<(), EditError> {
    if text.contains(['\n', '\r']) {
        return Err(EditError::EmbeddedNewline);
    }
    if text.trim().is_empty() {
        return Err(EditError::EmptyValue);
    }
    Ok(())
}

impl Document {
    /// Byte offset of a line's grammar text: past the `#` (and one optional space) of a disabled
    /// block's line.
    fn grammar_offset(&self, enabled: bool, line: usize) -> usize {
        if enabled {
            return 0;
        }
        let text = self.lines[line].text();
        disabled::uncomment_once(text).map_or(0, |i| text.len() - i.len())
    }

    /// Patch one body line of a block. See [`LinePatch`].
    pub fn patch_line(
        &mut self,
        id: BlockId,
        line: usize,
        patch: LinePatch,
    ) -> Result<(), EditError> {
        match &patch {
            LinePatch::Values(t)
            | LinePatch::Both(_, t)
            | LinePatch::Whole(t)
            | LinePatch::Content(t) => check_text(t)?,
            LinePatch::Operator(_) => {}
        }
        let b = self.block(id).ok_or(EditError::UnknownBlock)?;
        if matches!(b.disabled_state(), Some(DisabledState::Invalid(_))) {
            return Err(EditError::CannotEditInvalidDisabled);
        }
        if !b.body().iter().any(|bl| bl.line == line) {
            return Err(EditError::NotABodyLine);
        }
        let enabled = b.is_enabled();
        let owned = b.owned();
        let off = self.grammar_offset(enabled, line);
        let text = self.lines[line].text();
        let (head, inner) = text.split_at(off);
        let new_inner = match &patch {
            LinePatch::Operator(op) => apply_operator(inner, *op)?,
            LinePatch::Values(v) => apply_values(inner, v)?,
            LinePatch::Both(op, v) => apply_values(&apply_operator(inner, *op)?, v)?,
            LinePatch::Whole(v) => {
                let ws = inner.bytes().take_while(|c| is_ws(*c)).count();
                format!("{}{v}", &inner[..ws])
            }
            LinePatch::Content(v) => apply_content(inner, v)?,
        };
        let mut cand: Vec<Line> = self.lines[owned.clone()].to_vec();
        cand[line - owned.start] = Line::new(format!("{head}{new_inner}"), self.lines[line].eol());
        let strict = !matches!(patch, LinePatch::Whole(_));
        self.commit_owned(id, cand, strict, !enabled)
    }

    /// Insert one body line (`text` is the bare content, e.g. `ItemLevel >= 65`) after doc line
    /// `after`. Returns the new line's doc index.
    pub fn insert_body_line(
        &mut self,
        id: BlockId,
        after: usize,
        text: &str,
    ) -> Result<usize, EditError> {
        self.splice_body(id, &[], after, &[text.to_string()])?;
        Ok(after + 1)
    }

    /// Remove one body line.
    pub fn remove_body_line(&mut self, id: BlockId, line: usize) -> Result<(), EditError> {
        let header = self.block(id).ok_or(EditError::UnknownBlock)?.header_line();
        self.splice_body(id, &[line], header, &[])
    }

    /// Remove the given body lines and insert `texts` after doc line `after` (an index in the
    /// pre-edit document) in one validated commit.
    pub fn splice_body(
        &mut self,
        id: BlockId,
        remove: &[usize],
        after: usize,
        texts: &[String],
    ) -> Result<(), EditError> {
        self.splice_impl(id, remove, after, texts, false)
    }

    /// `lenient` skips the invalid-disabled refusal and the enableable expectation (the keyword and
    /// `Continue` toggles never had them).
    pub(super) fn splice_impl(
        &mut self,
        id: BlockId,
        remove: &[usize],
        after: usize,
        texts: &[String],
        lenient: bool,
    ) -> Result<(), EditError> {
        for t in texts {
            check_text(t)?;
        }
        let b = self.block(id).ok_or(EditError::UnknownBlock)?;
        if !lenient && matches!(b.disabled_state(), Some(DisabledState::Invalid(_))) {
            return Err(EditError::CannotEditInvalidDisabled);
        }
        let owned = b.owned();
        let header = b.header_line();
        let enabled = b.is_enabled();
        if !owned.contains(&after) || remove.contains(&after) {
            return Err(EditError::NotABodyLine);
        }
        if remove
            .iter()
            .any(|r| !b.body().iter().any(|bl| bl.line == *r))
        {
            return Err(EditError::NotABodyLine);
        }
        let (prefix, indent) = match b.body().first() {
            Some(bl) => {
                let text = self.lines[bl.line].text();
                let off = self.grammar_offset(enabled, bl.line);
                let ws = text[off..].bytes().take_while(|c| is_ws(*c)).count();
                (text[..off].to_string(), text[off..off + ws].to_string())
            }
            None => (
                if enabled {
                    ""
                } else {
                    disabled::DISABLE_PREFIX
                }
                .to_string(),
                "  ".to_string(),
            ),
        };

        let fresh = self.fresh_eol(header);
        let after_eol = match self.lines[after].eol() {
            Eol::None => fresh,
            e => e,
        };
        let last_unterminated =
            owned.end == self.lines.len() && self.lines[owned.end - 1].eol() == Eol::None;
        let mut cand: Vec<Line> = Vec::new();
        for i in owned.clone() {
            if remove.contains(&i) {
                continue;
            }
            cand.push(self.lines[i].clone());
            if i == after {
                for t in texts {
                    cand.push(Line::new(format!("{prefix}{indent}{t}"), after_eol));
                }
            }
        }
        // An interior line never stays unterminated; the document's last line keeps its absence of
        // a final newline.
        let n = cand.len();
        for (i, l) in cand.iter_mut().enumerate() {
            let want = if i + 1 == n && last_unterminated {
                Some(Eol::None)
            } else if l.eol() == Eol::None {
                Some(fresh)
            } else {
                None
            };
            if let Some(e) = want {
                if e != l.eol() {
                    *l = Line::new(l.text().to_string(), e);
                }
            }
        }
        self.commit_owned(id, cand, true, !enabled && !lenient)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::filter::{BlockKeyword, parse as parse_doc};

    fn p(s: &str) -> Document {
        parse_doc(s.as_bytes()).unwrap()
    }
    fn ids(d: &Document) -> Vec<BlockId> {
        d.blocks().map(|b| b.id()).collect()
    }
    fn s(d: &Document) -> String {
        d.serialize()
    }
    fn patch(src: &str, line: usize, lp: LinePatch) -> String {
        let mut d = p(src);
        let id = ids(&d)[0];
        d.patch_line(id, line, lp)
            .unwrap_or_else(|e| panic!("{src:?}: {e}"));
        s(&d)
    }

    #[test]
    fn values_keep_bare_operator_form() {
        assert_eq!(
            patch(
                "Show\n\tClass \"Rings\"\n",
                1,
                LinePatch::Values("\"Rings\" \"Amulets\"".into())
            ),
            "Show\n\tClass \"Rings\" \"Amulets\"\n"
        );
    }

    #[test]
    fn values_keep_trailing_comment_and_spacing() {
        assert_eq!(
            patch(
                "Show\n\tItemLevel >= 70 # keep\n",
                1,
                LinePatch::Values("75".into())
            ),
            "Show\n\tItemLevel >= 75 # keep\n"
        );
    }

    #[test]
    fn operator_replace_remove_insert() {
        assert_eq!(
            patch(
                "Show\n\tRarity>=Magic\n",
                1,
                LinePatch::Operator(Some(Operator::Le))
            ),
            "Show\n\tRarity<=Magic\n"
        );
        assert_eq!(
            patch("Show\n\tRarity>=Magic\n", 1, LinePatch::Operator(None)),
            "Show\n\tRarity Magic\n"
        );
        assert_eq!(
            patch("Show\n\tRarity >= Magic\n", 1, LinePatch::Operator(None)),
            "Show\n\tRarity Magic\n"
        );
        assert_eq!(
            patch(
                "Show\n\tItemLevel 70\n",
                1,
                LinePatch::Operator(Some(Operator::Ge))
            ),
            "Show\n\tItemLevel >= 70\n"
        );
    }

    #[test]
    fn both_switches_to_list_form() {
        assert_eq!(
            patch(
                "Show\n\tRarity >= Magic\n",
                1,
                LinePatch::Both(None, "Normal Magic Rare".into())
            ),
            "Show\n\tRarity Normal Magic Rare\n"
        );
    }

    #[test]
    fn empty_and_newline_values_refused() {
        let src = "Show\n\tItemLevel >= 70\n";
        let mut d = p(src);
        let id = ids(&d)[0];
        assert!(matches!(
            d.patch_line(id, 1, LinePatch::Values(String::new())),
            Err(EditError::EmptyValue)
        ));
        assert!(matches!(
            d.patch_line(id, 1, LinePatch::Values("7\n5".into())),
            Err(EditError::EmbeddedNewline)
        ));
        assert!(matches!(
            d.patch_line(id, 1, LinePatch::Values("7\r".into())),
            Err(EditError::EmbeddedNewline)
        ));
        assert_eq!(s(&d), src);
    }

    #[test]
    fn whole_on_opaque_lines() {
        let src = "Show\n\tFutureKw 3\n\tRarity Rare\n";
        assert_eq!(
            patch(src, 1, LinePatch::Whole("FutureKw 4".into())),
            "Show\n\tFutureKw 4\n\tRarity Rare\n"
        );
        assert_eq!(
            patch(src, 1, LinePatch::Whole("ItemLevel >= 70".into())),
            "Show\n\tItemLevel >= 70\n\tRarity Rare\n"
        );
        let mut d = p(src);
        let id = ids(&d)[0];
        assert!(matches!(
            d.patch_line(id, 1, LinePatch::Whole("Class".into())),
            Err(EditError::InvalidBlock { .. })
        ));
        assert!(matches!(
            d.patch_line(id, 1, LinePatch::Whole(String::new())),
            Err(EditError::EmptyValue)
        ));
        assert_eq!(s(&d), src);
    }

    #[test]
    fn content_replaces_keyword_and_values_and_keeps_the_comment() {
        assert_eq!(
            patch(
                "Show\n\tPlayAlertSound 3 200  # loud\n",
                1,
                LinePatch::Content("CustomAlertSound \"uwu.mp3\" 200".into())
            ),
            "Show\n\tCustomAlertSound \"uwu.mp3\" 200  # loud\n"
        );
        assert_eq!(
            patch(
                "#Show\n# \tEnableDropSound\n",
                1,
                LinePatch::Content("DisableDropSound".into())
            ),
            "#Show\n# \tDisableDropSound\n"
        );
        assert_eq!(
            patch(
                "Show\n    CustomAlertSound \"a.mp3\"\n",
                1,
                LinePatch::Content("PlayAlertSound 2".into())
            ),
            "Show\n    PlayAlertSound 2\n"
        );
    }

    #[test]
    fn whole_keeps_disabled_prefix_and_indent() {
        assert_eq!(
            patch(
                "#Show\n#\tItemLevel 60\n",
                1,
                LinePatch::Whole("ItemLevel  >=  70".into())
            ),
            "#Show\n#\tItemLevel  >=  70\n"
        );
    }

    #[test]
    fn edit_beside_a_pre_existing_warning() {
        let src = "Show\n\tFutureKw 3\n\tRarity Rare\n";
        assert_eq!(
            patch(src, 2, LinePatch::Values("Magic".into())),
            "Show\n\tFutureKw 3\n\tRarity Magic\n"
        );
    }

    #[test]
    fn header_and_foreign_lines_are_not_body_lines() {
        let src = "Show\n\tRarity Rare\n\nHide\n\tClass \"Bows\"\n";
        let mut d = p(src);
        let id = ids(&d)[0];
        for line in [0, 2, 3, 4] {
            assert!(
                matches!(
                    d.patch_line(id, line, LinePatch::Values("x".into())),
                    Err(EditError::NotABodyLine)
                ),
                "line {line}"
            );
        }
        assert_eq!(s(&d), src);
    }

    #[test]
    fn crlf_and_mixed_eols_survive() {
        let src = "Show\r\n\tRarity Rare\r\n\tClass \"Bows\"\r\n";
        assert_eq!(
            patch(src, 1, LinePatch::Values("Magic".into())),
            "Show\r\n\tRarity Magic\r\n\tClass \"Bows\"\r\n"
        );
        let mixed = "Show\r\n\tRarity Rare\n\tClass \"Bows\"\r\n";
        let mut d = p(mixed);
        let id = ids(&d)[0];
        d.patch_line(id, 2, LinePatch::Values("\"Wands\"".into()))
            .unwrap();
        assert_eq!(s(&d), "Show\r\n\tRarity Rare\n\tClass \"Wands\"\r\n");
        let new = d.insert_body_line(id, 1, "Quality >= 1").unwrap();
        assert_eq!(new, 2);
        assert_eq!(
            s(&d),
            "Show\r\n\tRarity Rare\n\tQuality >= 1\n\tClass \"Wands\"\r\n"
        );
        d.remove_body_line(id, 2).unwrap();
        assert_eq!(s(&d), "Show\r\n\tRarity Rare\n\tClass \"Wands\"\r\n");
    }

    #[test]
    fn valid_disabled_block_stays_enableable() {
        let mut d = p("#Show\n#\tItemLevel 70\n");
        let id = ids(&d)[0];
        d.patch_line(id, 1, LinePatch::Values("80".into())).unwrap();
        assert_eq!(s(&d), "#Show\n#\tItemLevel 80\n");
        assert!(d.block(id).unwrap().can_enable());
        // An edit that would leave it unenableable is refused.
        assert!(
            d.patch_line(id, 1, LinePatch::Whole("FutureKw 1".into()))
                .is_err()
        );
        assert_eq!(s(&d), "#Show\n#\tItemLevel 80\n");
    }

    #[test]
    fn invalid_disabled_block_refuses_every_op() {
        let src = "#Show\n#\tRarity =< Rare\n#\tClass \"Bows\"\n";
        let mut d = p(src);
        let id = ids(&d)[0];
        assert!(!d.block(id).unwrap().can_enable());
        assert!(matches!(
            d.patch_line(id, 2, LinePatch::Values("\"Wands\"".into())),
            Err(EditError::CannotEditInvalidDisabled)
        ));
        assert!(matches!(
            d.insert_body_line(id, 0, "Rarity Rare"),
            Err(EditError::CannotEditInvalidDisabled)
        ));
        assert!(matches!(
            d.remove_body_line(id, 2),
            Err(EditError::CannotEditInvalidDisabled)
        ));
        assert_eq!(s(&d), src);
    }

    #[test]
    fn insert_copies_indent() {
        let mut d = p("Show\n    Rarity Rare\n");
        let id = ids(&d)[0];
        assert_eq!(d.insert_body_line(id, 0, "Quality >= 1").unwrap(), 1);
        assert_eq!(s(&d), "Show\n    Quality >= 1\n    Rarity Rare\n");
        let mut d = p("Show\n");
        let id = ids(&d)[0];
        d.insert_body_line(id, 0, "Quality >= 1").unwrap();
        assert_eq!(s(&d), "Show\n  Quality >= 1\n");
        let mut d = p("#Show\n#\tRarity Rare\n");
        let id = ids(&d)[0];
        d.insert_body_line(id, 1, "Quality >= 1").unwrap();
        assert_eq!(s(&d), "#Show\n#\tRarity Rare\n#\tQuality >= 1\n");
        assert!(d.block(id).unwrap().can_enable());
        // The current `# ` form: the prefix is copied from the body, or `# ` under a bare header.
        let mut d = p("# Show\n# \tRarity Rare\n");
        let id = ids(&d)[0];
        d.insert_body_line(id, 1, "Quality >= 1").unwrap();
        assert_eq!(s(&d), "# Show\n# \tRarity Rare\n# \tQuality >= 1\n");
        let mut d = p("# Show\n");
        let id = ids(&d)[0];
        d.insert_body_line(id, 0, "Quality >= 1").unwrap();
        assert_eq!(s(&d), "# Show\n#   Quality >= 1\n");
        d.set_enabled(id, true).unwrap();
        assert_eq!(s(&d), "Show\n  Quality >= 1\n");
    }

    #[test]
    fn insert_and_remove_without_final_newline() {
        let mut d = p("Show\n\tRarity Rare");
        let id = ids(&d)[0];
        d.insert_body_line(id, 1, "Quality >= 1").unwrap();
        assert_eq!(s(&d), "Show\n\tRarity Rare\n\tQuality >= 1");
        d.remove_body_line(id, 2).unwrap();
        assert_eq!(s(&d), "Show\n\tRarity Rare");
    }

    #[test]
    fn remove_is_exact_and_neighbour_safe() {
        let src = "Show\n\tRarity Rare\n\tClass \"Bows\"\n\nHide\n\tQuality 1\n";
        let mut d = p(src);
        let all = ids(&d);
        d.remove_body_line(all[0], 1).unwrap();
        assert_eq!(s(&d), "Show\n\tClass \"Bows\"\n\nHide\n\tQuality 1\n");
        assert_eq!(ids(&d), all);
        // A line of another block, or the header, is not removable here.
        assert!(matches!(
            d.remove_body_line(all[0], 4),
            Err(EditError::NotABodyLine)
        ));
        assert!(matches!(
            d.remove_body_line(all[0], 0),
            Err(EditError::NotABodyLine)
        ));
        // A trailing commented condition that would be re-attributed to the neighbour.
        let src = "Show\n\tRarity Rare\n#\tSetFontSize 40\nHide\n";
        let mut d = p(src);
        let id = ids(&d)[0];
        let r = d.remove_body_line(id, 1);
        if r.is_err() {
            assert_eq!(s(&d), src);
        }
    }

    #[test]
    fn splice_is_atomic() {
        let mut d = p("Show\n\tRarity Rare\n\tSetFontSize 30\n\tPlayEffect Red\n");
        let id = ids(&d)[0];
        d.splice_body(
            id,
            &[2, 3],
            1,
            &[
                "SetTextColor 1 2 3".to_string(),
                "SetBorderColor 1 2 3".to_string(),
                "SetBackgroundColor 1 2 3".to_string(),
            ],
        )
        .unwrap();
        assert_eq!(
            s(&d),
            "Show\n\tRarity Rare\n\tSetTextColor 1 2 3\n\tSetBorderColor 1 2 3\n\tSetBackgroundColor 1 2 3\n"
        );
        let before = s(&d);
        assert!(
            d.splice_body(id, &[2], 1, &["FutureKw 3".to_string()])
                .is_err()
        );
        assert_eq!(s(&d), before);
    }

    #[test]
    fn ids_survive() {
        let mut d = p("Show\n\tRarity Rare\nHide\n\tClass \"Bows\"\n");
        let all = ids(&d);
        d.patch_line(all[1], 3, LinePatch::Values("\"Wands\"".into()))
            .unwrap();
        d.insert_body_line(all[0], 1, "Quality >= 1").unwrap();
        assert_eq!(ids(&d), all);
    }

    #[test]
    fn keyword_switch_keeps_each_eol() {
        let mut d = p("Show\r\n\tRarity Rare\n");
        let id = ids(&d)[0];
        d.set_block_keyword(id, BlockKeyword::Hide).unwrap();
        assert_eq!(s(&d), "Hide\r\n\tRarity Rare\n");
    }

    #[test]
    fn continue_toggle_restores_mixed_eols() {
        let src = "Show\r\n\tRarity Rare\n\tClass \"Bows\"\r\n";
        let mut d = p(src);
        let id = ids(&d)[0];
        d.set_continue(id, true).unwrap();
        assert_eq!(
            s(&d),
            "Show\r\n\tRarity Rare\n\tClass \"Bows\"\r\n\tContinue\r\n"
        );
        d.set_continue(id, false).unwrap();
        assert_eq!(s(&d), src);
    }
}
