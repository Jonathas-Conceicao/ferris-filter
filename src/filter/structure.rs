//! Whole-block structural edits: append, remove, duplicate and move a block, switch its
//! `Show`/`Hide`/`Minimal` keyword and toggle a standalone `Continue` line.
//!
//! Same contract as [`edit`](super::edit): validate first, mutate after, only the touched units
//! change, and an `Err` leaves the [`Document`] untouched. Unlike the in-place edits, these change
//! the number or order of blocks, so ids cannot be remapped by position. Each operation states
//! the id of every block in the new order (existing ids verbatim, `None` for a fresh block) and the
//! text each block's span must have afterwards; [`Document::commit_structural`] rebuilds the index
//! and refuses the edit if the rebuilt blocks disagree.
//!
//! A block's *unit* is its span (leading comments plus owned lines) and the blank-only lines
//! directly after it. Moves swap units and leave whatever lies between them where it is, which
//! makes a move its own inverse.

use std::ops::Range;

use super::Document;
use super::block::{Block, BlockId, BlockKeyword, Item};
use super::diag::Severity;
use super::disabled;
use super::edit::EditError;
use super::keywords::Kind;
use super::lines::{Eol, Line};
use super::parse;
use super::section;
use super::tokenize::tokenize;

struct Info {
    id: BlockId,
    header: usize,
    span: Range<usize>,
    unit: Range<usize>,
}

fn is_blank(l: &Line) -> bool {
    l.text().trim().is_empty()
}

fn is_marker(l: &Line) -> bool {
    matches!(
        l.ff_marker().map(|(k, _)| k),
        Some(section::BEGIN_KEY | section::END_KEY)
    )
}

impl Document {
    /// The terminator of the last terminated line, `Lf` when there is none.
    fn last_eol(&self) -> Eol {
        self.lines
            .iter()
            .rev()
            .map(|l| l.eol())
            .find(|e| *e != Eol::None)
            .unwrap_or(Eol::Lf)
    }

    fn infos(&self) -> Vec<Info> {
        let n = self.lines.len();
        self.blocks()
            .map(|b: &Block| {
                let span = b.span();
                let mut end = span.end;
                while end < n && is_blank(&self.lines[end]) {
                    end += 1;
                }
                Info {
                    id: b.id(),
                    header: b.header_line(),
                    unit: span.start..end,
                    span,
                }
            })
            .collect()
    }

    fn span_texts(&self, span: Range<usize>) -> Vec<String> {
        self.lines[span]
            .iter()
            .map(|l| l.text().to_string())
            .collect()
    }

    /// Rebuild the index over `new_lines` and commit it. `ids` names every block of the result in
    /// order (`None` = a fresh id); `expected` is the span text each block must have. Nothing is
    /// mutated on `Err`.
    fn commit_structural(
        &mut self,
        new_lines: Vec<Line>,
        ids: Vec<Option<BlockId>>,
        expected: Vec<Vec<String>>,
    ) -> Result<Vec<BlockId>, EditError> {
        let mut scratch = 0u64;
        let (mut items, diags) = parse::build(&new_lines, &mut scratch);
        let n_blocks = items.iter().filter(|i| matches!(i, Item::Block(_))).count();
        if n_blocks != ids.len() || ids.len() != expected.len() {
            return Err(EditError::NotOneBlock);
        }
        let mut next = self.next_id;
        let mut out = Vec::with_capacity(ids.len());
        let mut idx = 0usize;
        for it in items.iter_mut() {
            if let Item::Block(b) = it {
                let span = b.span();
                let same = new_lines[span]
                    .iter()
                    .map(|l| l.text())
                    .eq(expected[idx].iter().map(String::as_str));
                if !same {
                    return Err(EditError::WouldReattributeLines);
                }
                let id = match ids[idx] {
                    Some(id) => id,
                    None => {
                        let id = BlockId(next);
                        next += 1;
                        id
                    }
                };
                b.id = id;
                out.push(id);
                idx += 1;
            }
        }
        self.next_id = next;
        self.lines = new_lines;
        self.items = items;
        self.diagnostics = diags;
        Ok(out)
    }

    fn blank_line(&self, near: usize) -> Line {
        Line::new(String::new(), self.fresh_eol(near))
    }

    /// Owned line texts of a block, plus the header's byte offset of the grammar text (past the
    /// first `#` for a disabled block) and its tokens. Compact headers are refused.
    fn header_parts(
        &self,
        id: BlockId,
    ) -> Result<(Vec<String>, usize, std::ops::Range<usize>), EditError> {
        let b = self.block(id).ok_or(EditError::UnknownBlock)?;
        let owned = b.owned();
        let texts: Vec<String> = self.span_texts(owned);
        let text = &texts[0];
        let off = if b.is_enabled() {
            0
        } else {
            let inner = disabled::uncomment_once(text).ok_or(EditError::CompactHeader)?;
            text.len() - inner.len()
        };
        let t = tokenize(&text[off..]).map_err(|_| EditError::CompactHeader)?;
        if t.operator.is_some() || !t.values.is_empty() {
            return Err(EditError::CompactHeader);
        }
        let kw = t.keyword.ok_or(EditError::CompactHeader)?;
        Ok((texts, off, kw))
    }

    /// Switch the block header between `Show`, `Hide` and `Minimal`, touching only the keyword.
    pub fn set_block_keyword(&mut self, id: BlockId, kw: BlockKeyword) -> Result<(), EditError> {
        let (texts, off, range) = self.header_parts(id)?;
        let b = self.block(id).ok_or(EditError::UnknownBlock)?;
        if b.keyword() == kw {
            return Ok(());
        }
        let owned = b.owned();
        let word = match kw {
            BlockKeyword::Show => "Show",
            BlockKeyword::Hide => "Hide",
            BlockKeyword::Minimal => "Minimal",
        };
        let mut head = texts[0].clone();
        head.replace_range(off + range.start..off + range.end, word);
        let mut cand: Vec<Line> = self.lines[owned.clone()].to_vec();
        cand[0] = Line::new(head, self.lines[owned.start].eol());
        self.commit_owned(id, cand, true, false)
    }

    /// Add or remove the standalone `Continue` line of a block.
    pub fn set_continue(&mut self, id: BlockId, on: bool) -> Result<(), EditError> {
        self.header_parts(id)?;
        let b = self.block(id).ok_or(EditError::UnknownBlock)?;
        if b.has_continue() == on {
            return Ok(());
        }
        if on {
            let last = b.owned().end - 1;
            self.splice_impl(id, &[], last, &["Continue".to_string()], true)
        } else {
            let header = b.header_line();
            let doomed: Vec<usize> = b
                .body()
                .iter()
                .filter(|bl| {
                    bl.keyword.is_some_and(|k| {
                        k.kind == Kind::Modifier && k.name.eq_ignore_ascii_case("continue")
                    })
                })
                .map(|bl| bl.line)
                .collect();
            self.splice_impl(id, &doomed, header, &[], true)
        }
    }

    /// Append a new rule block before the economy section (or at the end of the file).
    pub fn append_rule(&mut self, lines: Vec<String>) -> Result<BlockId, EditError> {
        if lines.iter().any(|s| s.contains(['\n', '\r'])) {
            return Err(EditError::EmbeddedNewline);
        }
        let n = lines.len();
        let cand: Vec<Line> = lines
            .iter()
            .map(|t| Line::new(t.clone(), Eol::Lf))
            .collect();
        let mut scratch = 0u64;
        let (items, diags) = parse::build(&cand, &mut scratch);
        let one = items.len() == 1
            && matches!(&items[0], Item::Block(c) if c.lead_start == 0 && c.end == n);
        if !one {
            return Err(EditError::NotOneBlock);
        }
        if let Item::Block(c) = &items[0] {
            if c.is_enabled() {
                let errs: Vec<_> = diags
                    .iter()
                    .filter(|d| matches!(d.severity, Severity::Error | Severity::Warning))
                    .cloned()
                    .collect();
                if !errs.is_empty() {
                    return Err(EditError::InvalidBlock { diagnostics: errs });
                }
            }
        }

        let sec = section::find(self).map_err(|e| EditError::EconomyDamaged(e.to_string()))?;
        let total = self.lines.len();
        let ins = match sec {
            Some(s) => {
                let mut i = s.begin;
                while i > 0 && is_blank(&self.lines[i - 1]) {
                    i -= 1;
                }
                i
            }
            None => total,
        };
        let eol = if ins > 0 {
            self.fresh_eol(ins - 1)
        } else {
            self.last_eol()
        };
        let mut new: Vec<Line> = self.lines[..ins].to_vec();
        if let Some(last) = new.last_mut() {
            if last.eol() == Eol::None {
                *last = Line::new(last.text().to_string(), eol);
            }
        }
        if new.last().is_some_and(|l| !is_blank(l)) {
            new.push(Line::new(String::new(), eol));
        }
        for t in &lines {
            new.push(Line::new(t.clone(), eol));
        }
        if sec.is_some_and(|s| ins == s.begin) {
            new.push(Line::new(String::new(), eol));
        }
        new.extend_from_slice(&self.lines[ins..]);

        let infos = self.infos();
        let pos = infos.iter().filter(|i| i.header < ins).count();
        let mut ids: Vec<Option<BlockId>> = infos.iter().map(|i| Some(i.id)).collect();
        ids.insert(pos, None);
        let mut expected: Vec<Vec<String>> = infos
            .iter()
            .map(|i| self.span_texts(i.span.clone()))
            .collect();
        expected.insert(pos, lines);
        let out = self.commit_structural(new, ids, expected)?;
        Ok(out[pos])
    }

    /// Remove the block's unit (span plus the blank lines directly after it).
    pub fn remove_block(&mut self, id: BlockId) -> Result<(), EditError> {
        let infos = self.infos();
        let pos = infos
            .iter()
            .position(|i| i.id == id)
            .ok_or(EditError::UnknownBlock)?;
        let unit = infos[pos].unit.clone();
        let mut new = self.lines[..unit.start].to_vec();
        new.extend_from_slice(&self.lines[unit.end..]);
        let mut ids: Vec<Option<BlockId>> = infos.iter().map(|i| Some(i.id)).collect();
        ids.remove(pos);
        let mut expected: Vec<Vec<String>> = infos
            .iter()
            .map(|i| self.span_texts(i.span.clone()))
            .collect();
        expected.remove(pos);
        self.commit_structural(new, ids, expected).map(|_| ())
    }

    /// Insert a verbatim copy of the block directly after it. The copy gets a fresh id.
    pub fn duplicate_block(&mut self, id: BlockId) -> Result<BlockId, EditError> {
        let infos = self.infos();
        let pos = infos
            .iter()
            .position(|i| i.id == id)
            .ok_or(EditError::UnknownBlock)?;
        let (span, unit) = (infos[pos].span.clone(), infos[pos].unit.clone());
        let more = unit.end < self.lines.len();
        let eol = self.last_eol();
        let mut new = self.lines[..unit.end].to_vec();
        if let Some(last) = new.last_mut() {
            if last.eol() == Eol::None {
                *last = Line::new(last.text().to_string(), eol);
            }
        }
        if unit.end == span.end {
            new.push(self.blank_line(span.end - 1));
        }
        new.extend_from_slice(&self.lines[span.clone()]);
        if more {
            new.push(self.blank_line(span.end - 1));
        }
        new.extend_from_slice(&self.lines[unit.end..]);

        let mut ids: Vec<Option<BlockId>> = infos.iter().map(|i| Some(i.id)).collect();
        ids.insert(pos + 1, None);
        let mut expected: Vec<Vec<String>> = infos
            .iter()
            .map(|i| self.span_texts(i.span.clone()))
            .collect();
        expected.insert(pos + 1, self.span_texts(span));
        let out = self.commit_structural(new, ids, expected)?;
        Ok(out[pos + 1])
    }

    /// The block this one would swap with when moved, or `None` when there is none, when either is
    /// an economy block, or when an economy marker lies between the two.
    pub fn move_neighbour(&self, id: BlockId, up: bool) -> Result<Option<BlockId>, EditError> {
        let infos = self.infos();
        let pos = infos
            .iter()
            .position(|i| i.id == id)
            .ok_or(EditError::UnknownBlock)?;
        let sec = section::find(self).map_err(|e| EditError::EconomyDamaged(e.to_string()))?;
        let np = if up {
            pos.checked_sub(1)
        } else {
            (pos + 1 < infos.len()).then_some(pos + 1)
        };
        let Some(np) = np else {
            return Ok(None);
        };
        let in_economy = |h: usize| sec.is_some_and(|s| h > s.begin && h < s.end);
        if in_economy(infos[pos].header) || in_economy(infos[np].header) {
            return Ok(None);
        }
        let (a, b) = if np < pos {
            (&infos[np], &infos[pos])
        } else {
            (&infos[pos], &infos[np])
        };
        if (a.span.end..b.span.start).any(|i| is_marker(&self.lines[i])) {
            return Ok(None);
        }
        Ok(Some(infos[np].id))
    }

    /// Swap the block with its neighbour in place. Ids follow the blocks.
    pub fn move_block(&mut self, id: BlockId, up: bool) -> Result<(), EditError> {
        let nb = self.move_neighbour(id, up)?.ok_or(EditError::NoNeighbour)?;
        let infos = self.infos();
        let p1 = infos
            .iter()
            .position(|i| i.id == id)
            .ok_or(EditError::UnknownBlock)?;
        let p2 = infos
            .iter()
            .position(|i| i.id == nb)
            .ok_or(EditError::UnknownBlock)?;
        let (pa, pb) = (p1.min(p2), p1.max(p2));
        let a_region = infos[pa].span.start..infos[pa].unit.end;
        let strays = infos[pa].unit.end..infos[pb].span.start;
        let b_region = infos[pb].span.start..infos[pb].unit.end;

        let eol = self.last_eol();
        let mut old = self.lines.clone();
        let last = old.len() - 1;
        let was_unterminated = old[last].eol() == Eol::None;
        if was_unterminated {
            old[last] = Line::new(old[last].text().to_string(), eol);
        }
        let mut new = old[..a_region.start].to_vec();
        new.extend_from_slice(&old[b_region.clone()]);
        new.extend_from_slice(&old[strays]);
        new.extend_from_slice(&old[a_region.clone()]);
        new.extend_from_slice(&old[b_region.end..]);
        if was_unterminated {
            if let Some(l) = new.last_mut() {
                if !l.text().is_empty() {
                    *l = Line::new(l.text().to_string(), Eol::None);
                }
            }
        }

        let mut ids: Vec<Option<BlockId>> = infos.iter().map(|i| Some(i.id)).collect();
        ids.swap(pa, pb);
        let mut expected: Vec<Vec<String>> = infos
            .iter()
            .map(|i| self.span_texts(i.span.clone()))
            .collect();
        expected.swap(pa, pb);
        self.commit_structural(new, ids, expected).map(|_| ())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::filter::parse as parse_doc;

    const BEGIN: &str = section::BEGIN_LINE;
    const END: &str = section::END_LINE;

    fn p(s: &str) -> Document {
        parse_doc(s.as_bytes()).unwrap()
    }
    fn ids(d: &Document) -> Vec<BlockId> {
        d.blocks().map(|b| b.id()).collect()
    }
    fn s(d: &Document) -> String {
        d.serialize()
    }
    fn v(a: &[&str]) -> Vec<String> {
        a.iter().map(|x| x.to_string()).collect()
    }

    #[test]
    fn append_basic_and_fresh_id() {
        let mut d = p("Show\n    Class \"Rings\"\n");
        let before = ids(&d);
        let id = d.append_rule(v(&["Show"])).unwrap();
        assert_eq!(s(&d), "Show\n    Class \"Rings\"\n\nShow\n");
        assert!(!before.contains(&id));
        assert_eq!(ids(&d), vec![before[0], id]);
    }

    #[test]
    fn append_before_economy_section() {
        let src = format!("Show # A\n  Rarity Rare\n\n{BEGIN}\n\nHide # e\n\n{END}\n");
        let mut d = p(&src);
        let econ_before: Vec<String> = d.lines()[3..]
            .iter()
            .map(|l| l.text().to_string())
            .collect();
        d.append_rule(v(&["Show # N"])).unwrap();
        assert_eq!(
            s(&d),
            format!("Show # A\n  Rarity Rare\n\nShow # N\n\n{BEGIN}\n\nHide # e\n\n{END}\n")
        );
        assert!(section::find(&d).unwrap().is_some());
        let n = d.lines().len();
        let econ_after: Vec<String> = d.lines()[n - econ_before.len()..]
            .iter()
            .map(|l| l.text().to_string())
            .collect();
        assert_eq!(econ_before, econ_after);
        // No blank run above the begin marker: one is created.
        let src = format!("Show # A\n{BEGIN}\n\nHide\n\n{END}\n");
        let mut d = p(&src);
        d.append_rule(v(&["Show # N"])).unwrap();
        assert_eq!(
            s(&d),
            format!("Show # A\n\nShow # N\n\n{BEGIN}\n\nHide\n\n{END}\n")
        );
    }

    #[test]
    fn append_after_trailing_comment_and_edge_inputs() {
        let mut d = p("Show\n# note\n");
        d.append_rule(v(&["Show"])).unwrap();
        assert_eq!(s(&d), "Show\n# note\n\nShow\n");
        assert_eq!(d.blocks().nth(1).unwrap().leading().len(), 0);

        let mut d = p("");
        d.append_rule(v(&["Show"])).unwrap();
        assert_eq!(s(&d), "Show\n");

        let mut d = p("Show\r\nHide");
        d.append_rule(v(&["Show"])).unwrap();
        assert_eq!(s(&d), "Show\r\nHide\r\n\r\nShow\r\n");
    }

    #[test]
    fn append_rejects_bad_input_and_damaged_economy() {
        let src = "Show\n";
        let mut d = p(src);
        assert!(matches!(
            d.append_rule(v(&["Show\n"])),
            Err(EditError::EmbeddedNewline)
        ));
        assert!(matches!(
            d.append_rule(v(&["Show", "Hide"])),
            Err(EditError::NotOneBlock)
        ));
        assert!(matches!(
            d.append_rule(v(&["Show", "  Rarity =< Rare"])),
            Err(EditError::InvalidBlock { .. })
        ));
        assert_eq!(s(&d), src);
        let src = format!("Show\n{END}\n");
        let mut d = p(&src);
        assert!(matches!(
            d.append_rule(v(&["Show"])),
            Err(EditError::EconomyDamaged(_))
        ));
        assert_eq!(s(&d), src);
    }

    #[test]
    fn remove_takes_following_blanks_only() {
        let mut d = p("Show # A\n\nHide # B\n  Class \"Bows\"\n\n\nShow # C\n");
        let all = ids(&d);
        d.remove_block(all[1]).unwrap();
        assert_eq!(s(&d), "Show # A\n\nShow # C\n");
        assert_eq!(ids(&d), vec![all[0], all[2]]);
        let before = s(&d);
        assert!(matches!(
            d.remove_block(all[1]),
            Err(EditError::UnknownBlock)
        ));
        assert_eq!(s(&d), before);
    }

    #[test]
    fn duplicate_is_verbatim_with_fresh_id() {
        let mut d = p("# lead\n#ff:name a\nShow # A\n  Rarity Rare\n\nHide # B\n");
        let all = ids(&d);
        let copy = d.duplicate_block(all[0]).unwrap();
        assert_eq!(
            s(&d),
            "# lead\n#ff:name a\nShow # A\n  Rarity Rare\n\n# lead\n#ff:name a\nShow # A\n  Rarity Rare\n\nHide # B\n"
        );
        assert_eq!(ids(&d), vec![all[0], copy, all[1]]);
        assert!(!all.contains(&copy));
        assert_eq!(d.meta_get(copy, "name"), Some("a"));
        d.remove_block(copy).unwrap();
        assert_eq!(
            s(&d),
            "# lead\n#ff:name a\nShow # A\n  Rarity Rare\n\nHide # B\n"
        );
    }

    #[test]
    fn duplicate_disabled_and_without_trailing_blank() {
        let mut d = p("#Show\n#  Rarity Rare\n\nHide\n");
        let all = ids(&d);
        let copy = d.duplicate_block(all[0]).unwrap();
        assert!(!d.block(copy).unwrap().is_enabled());
        assert!(d.block(all[0]).unwrap().is_enabled() == false);
        let mut d = p("Show # A\nHide # B\n");
        let all = ids(&d);
        d.duplicate_block(all[0]).unwrap();
        assert_eq!(s(&d), "Show # A\n\nShow # A\n\nHide # B\n");
        let mut d = p("Show # A");
        let a = ids(&d)[0];
        d.duplicate_block(a).unwrap();
        assert_eq!(s(&d), "Show # A\n\nShow # A");
    }

    #[test]
    fn move_swaps_units_and_ids_follow() {
        let src = "#ff:name a\nShow\n  Rarity Rare\n\n#ff:name b\nHide\n  Class \"Bows\"\n\n";
        let mut d = p(src);
        let all = ids(&d);
        d.move_block(all[0], false).unwrap();
        assert_eq!(
            s(&d),
            "#ff:name b\nHide\n  Class \"Bows\"\n\n#ff:name a\nShow\n  Rarity Rare\n\n"
        );
        assert_eq!(ids(&d), vec![all[1], all[0]]);
        assert_eq!(d.meta_get(all[0], "name"), Some("a"));
        assert_eq!(d.meta_get(all[1], "name"), Some("b"));
        d.move_block(all[0], true).unwrap();
        assert_eq!(s(&d), src);
        assert_eq!(ids(&d), all);
    }

    #[test]
    fn move_leaves_stray_lines_in_place() {
        let src = "Show # A\n\n# stray\n\nHide # B\n";
        let mut d = p(src);
        let all = ids(&d);
        d.move_block(all[1], true).unwrap();
        assert_eq!(
            s(&d),
            "Hide # B\n# stray\n\nShow # A\n\n"
                .replace("Hide # B\n# stray\n\nShow # A\n\n", &s(&d))
        );
        d.move_block(all[1], false).unwrap();
        assert_eq!(s(&d), src);
    }

    #[test]
    fn move_never_crosses_economy() {
        let src = format!("Show # R1\n\n{BEGIN}\n\nHide # E\n\n{END}\nShow # R2\n");
        let mut d = p(&src);
        let all = ids(&d);
        assert_eq!(d.move_neighbour(all[0], false).unwrap(), None);
        assert_eq!(d.move_neighbour(all[1], true).unwrap(), None);
        assert_eq!(d.move_neighbour(all[1], false).unwrap(), None);
        assert_eq!(d.move_neighbour(all[2], true).unwrap(), None);
        for (i, up) in [(0, false), (1, true), (1, false), (2, true)] {
            assert!(matches!(
                d.move_block(all[i], up),
                Err(EditError::NoNeighbour)
            ));
        }
        assert_eq!(s(&d), src);
        // An economy section with no blocks still separates its neighbours.
        let src = format!("Show # R1\n\n{BEGIN}\n\n{END}\nShow # R2\n");
        let d = p(&src);
        let all = ids(&d);
        assert_eq!(d.move_neighbour(all[0], false).unwrap(), None);
        assert_eq!(d.move_neighbour(all[1], true).unwrap(), None);
        // Rules on one side still swap.
        let src = format!("Show # R1\n\nShow # R2\n\n{BEGIN}\n\n{END}\n");
        let mut d = p(&src);
        let all = ids(&d);
        d.move_block(all[1], true).unwrap();
        assert_eq!(
            s(&d),
            format!("Show # R2\n\nShow # R1\n\n{BEGIN}\n\n{END}\n")
        );
        assert!(section::find(&d).unwrap().is_some());
    }

    #[test]
    fn move_unterminated_final_block() {
        let src = "Show # A\nHide # B";
        let mut d = p(src);
        let all = ids(&d);
        d.move_block(all[1], true).unwrap();
        assert_eq!(s(&d), "Hide # B\nShow # A");
        d.move_block(all[1], false).unwrap();
        assert_eq!(s(&d), src);
        // A trailing blank line cannot be unterminated: it takes the document terminator.
        let src = "Show # A\n\nHide # B";
        let mut d = p(src);
        let all = ids(&d);
        d.move_block(all[1], true).unwrap();
        assert_eq!(s(&d), "Hide # B\nShow # A\n\n");
        // The "no final newline" fact is not recoverable from a blank last line, so the
        // inverse move ends terminated.
        d.move_block(all[1], false).unwrap();
        assert_eq!(
            s(&d),
            format!(
                "{src}
"
            )
        );
    }

    #[test]
    fn crlf_is_preserved() {
        let src = "Show # A\r\n\r\nHide # B\r\n";
        let mut d = p(src);
        let all = ids(&d);
        d.duplicate_block(all[0]).unwrap();
        d.append_rule(v(&["Minimal"])).unwrap();
        d.move_block(all[1], false).unwrap();
        d.remove_block(all[0]).unwrap();
        let out = s(&d);
        assert_eq!(out.matches('\n').count(), out.matches("\r\n").count());
    }

    #[test]
    fn errors_leave_document_unchanged() {
        let src = "Show # A\n\nHide # B\n";
        let mut d = p(src);
        let all = ids(&d);
        assert!(matches!(
            d.move_block(all[0], true),
            Err(EditError::NoNeighbour)
        ));
        assert!(matches!(
            d.move_block(all[1], false),
            Err(EditError::NoNeighbour)
        ));
        let bogus = BlockId(9999);
        assert!(matches!(
            d.move_block(bogus, true),
            Err(EditError::UnknownBlock)
        ));
        assert!(matches!(
            d.duplicate_block(bogus),
            Err(EditError::UnknownBlock)
        ));
        assert!(matches!(
            d.move_neighbour(bogus, true),
            Err(EditError::UnknownBlock)
        ));
        assert_eq!(s(&d), src);
        assert_eq!(ids(&d), all);
    }

    #[test]
    fn keyword_rewrites_only_the_token() {
        let src = "  Show # %D4 $type->currency !slug
  Class \"Bows\"
";
        let mut d = p(src);
        let id = ids(&d)[0];
        d.set_block_keyword(id, BlockKeyword::Hide).unwrap();
        assert_eq!(s(&d), src.replace("Show", "Hide"));
        assert_eq!(d.block(id).unwrap().keyword(), BlockKeyword::Hide);
        d.set_block_keyword(id, BlockKeyword::Hide).unwrap();
        d.set_block_keyword(id, BlockKeyword::Show).unwrap();
        assert_eq!(s(&d), src);
    }

    #[test]
    fn keyword_on_disabled_block_keeps_it_disabled() {
        let src = "#Show # c
#  Rarity Rare
";
        let mut d = p(src);
        let id = ids(&d)[0];
        d.set_block_keyword(id, BlockKeyword::Minimal).unwrap();
        assert_eq!(
            s(&d),
            "#Minimal # c
#  Rarity Rare
"
        );
        let b = d.block(id).unwrap();
        assert!(!b.is_enabled() && b.can_enable());
    }

    #[test]
    fn compact_header_is_refused() {
        let src = "Show Continue
  Rarity Rare
";
        let mut d = p(src);
        let id = ids(&d)[0];
        assert!(matches!(
            d.set_block_keyword(id, BlockKeyword::Hide),
            Err(EditError::CompactHeader)
        ));
        assert!(matches!(
            d.set_continue(id, true),
            Err(EditError::CompactHeader)
        ));
        assert_eq!(s(&d), src);
    }

    #[test]
    fn continue_uses_body_indent_and_removes_all() {
        let mut d = p("Show
	Rarity Rare

Hide
");
        let id = ids(&d)[0];
        d.set_continue(id, true).unwrap();
        assert_eq!(
            s(&d),
            "Show
	Rarity Rare
	Continue

Hide
"
        );
        d.set_continue(id, true).unwrap();
        d.set_continue(id, false).unwrap();
        assert_eq!(
            s(&d),
            "Show
	Rarity Rare

Hide
"
        );
        let hide = ids(&d)[1];
        d.set_continue(hide, true).unwrap();
        assert_eq!(
            s(&d),
            "Show
	Rarity Rare

Hide
  Continue
"
        );
        let mut d = p("Show
  Continue
  Rarity Rare
  Continue
");
        let id = ids(&d)[0];
        d.set_continue(id, false).unwrap();
        assert_eq!(
            s(&d),
            "Show
  Rarity Rare
"
        );
    }

    #[test]
    fn continue_on_disabled_block() {
        let mut d = p("#Show
#	Rarity Rare
");
        let id = ids(&d)[0];
        d.set_continue(id, true).unwrap();
        assert_eq!(
            s(&d),
            "#Show
#	Rarity Rare
#	Continue
"
        );
        assert!(d.block(id).unwrap().has_continue());
        d.set_continue(id, false).unwrap();
        assert_eq!(
            s(&d),
            "#Show
#	Rarity Rare
"
        );
    }
}
