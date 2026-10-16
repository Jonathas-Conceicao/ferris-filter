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

pub(super) struct Info {
    pub(super) id: BlockId,
    pub(super) header: usize,
    pub(super) span: Range<usize>,
    pub(super) unit: Range<usize>,
}

pub(super) fn is_blank(l: &Line) -> bool {
    l.text().trim().is_empty()
}

impl Document {
    /// The terminator of the last terminated line, `Lf` when there is none.
    pub(super) fn last_eol(&self) -> Eol {
        self.lines
            .iter()
            .rev()
            .map(|l| l.eol())
            .find(|e| *e != Eol::None)
            .unwrap_or(Eol::Lf)
    }

    pub(super) fn infos(&self) -> Vec<Info> {
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

    pub(super) fn span_texts(&self, span: Range<usize>) -> Vec<String> {
        self.lines[span]
            .iter()
            .map(|l| l.text().to_string())
            .collect()
    }

    /// Rebuild the index over `new_lines` and commit it. `ids` names every block of the result in
    /// order (`None` = a fresh id); `expected` is the span text each block must have. Nothing is
    /// mutated on `Err`.
    pub(super) fn commit_structural(
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

    /// Insert a new rule block as the first block of the file, above the first block's leading
    /// comments, outside every section.
    pub fn insert_rule_at_top(&mut self, lines: Vec<String>) -> Result<BlockId, EditError> {
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
        let ins = self.top_insertion_line(sec);
        let eol = if ins > 0 {
            self.fresh_eol(ins - 1)
        } else {
            match self.lines.first().map(|l| l.eol()) {
                Some(e) if e != Eol::None => e,
                _ => self.last_eol(),
            }
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
        if self.lines.get(ins).is_some_and(|l| !is_blank(l)) {
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

    /// The one insertion point for new rules: the first line of the first block outside the
    /// economy section, including that block's leading comments and `# ff:` meta lines, or the
    /// first section banner when that comes first, so a new rule always lands outside every
    /// section. With neither it is just above the economy section's leading blank lines, or the
    /// end of the file.
    pub(super) fn top_insertion_line(&self, sec: Option<section::SectionSpan>) -> usize {
        let limit = sec.map_or(usize::MAX, |s| s.begin);
        let first_block = self
            .infos()
            .iter()
            .find(|i| i.span.start < limit)
            .map(|i| i.span.start);
        let first_banner = section::banners(self)
            .iter()
            .find(|b| b.kind == section::BannerKind::Section && b.start < limit)
            .map(|b| b.start);
        if let Some(at) = first_block.into_iter().chain(first_banner).min() {
            return at;
        }
        match sec {
            Some(s) => {
                let mut i = s.begin;
                while i > 0 && is_blank(&self.lines[i - 1]) {
                    i -= 1;
                }
                i
            }
            None => self.lines.len(),
        }
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
        let banners = section::banner_starts(&self.lines);
        if (a.span.end..b.span.start).any(|i| banners.iter().any(|&s| i >= s && i < s + 3)) {
            return Ok(None);
        }
        Ok(Some(infos[np].id))
    }

    /// Swap the block with its neighbour in place. Ids follow the blocks.
    pub fn move_block(&mut self, id: BlockId, up: bool) -> Result<(), EditError> {
        let nb = self.move_neighbour(id, up)?.ok_or(EditError::NoNeighbour)?;
        if up {
            self.swap_runs(&[nb], &[id])
        } else {
            self.swap_runs(&[id], &[nb])
        }
    }

    /// Remove the blocks `old` (which must be the blocks directly after `anchor`, in order) and
    /// insert one block per entry of `spans` (span texts, no terminators) after `anchor`, each
    /// preceded by one blank line. The new blocks reuse the ids of `old` by position (fresh ids
    /// for the rest) and their ids are returned. Lines between `anchor` and the end of the last
    /// old block are replaced; everything else keeps its bytes.
    pub fn replace_blocks_after(
        &mut self,
        anchor: BlockId,
        old: &[BlockId],
        spans: Vec<Vec<String>>,
    ) -> Result<Vec<BlockId>, EditError> {
        if spans.iter().flatten().any(|s| s.contains(['\n', '\r'])) {
            return Err(EditError::EmbeddedNewline);
        }
        let infos = self.infos();
        let pos = infos
            .iter()
            .position(|i| i.id == anchor)
            .ok_or(EditError::UnknownBlock)?;
        for (k, o) in old.iter().enumerate() {
            if infos.get(pos + 1 + k).map(|i| i.id) != Some(*o) {
                return Err(EditError::NotOneBlock);
            }
        }
        if old.is_empty() && spans.is_empty() {
            return Ok(Vec::new());
        }
        let a_end = infos[pos].span.end;
        let region_end = match old.len() {
            0 => a_end,
            n => infos[pos + n].span.end,
        };
        let eol = self.fresh_eol(a_end - 1);
        let mut new = self.lines[..a_end].to_vec();
        if let Some(last) = new.last_mut()
            && last.eol() == Eol::None
        {
            *last = Line::new(last.text().to_string(), eol);
        }
        for span in &spans {
            new.push(Line::new(String::new(), eol));
            for t in span {
                new.push(Line::new(t.clone(), eol));
            }
        }
        let rest = &self.lines[region_end..];
        if rest.first().is_some_and(|l| !is_blank(l)) {
            new.push(Line::new(String::new(), eol));
        }
        if rest.is_empty()
            && self.lines.last().is_some_and(|l| l.eol() == Eol::None)
            && let Some(l) = new.last_mut()
            && !l.text().is_empty()
        {
            *l = Line::new(l.text().to_string(), Eol::None);
        }
        new.extend_from_slice(rest);

        let mut ids: Vec<Option<BlockId>> = infos.iter().map(|i| Some(i.id)).collect();
        let mut expected: Vec<Vec<String>> = infos
            .iter()
            .map(|i| self.span_texts(i.span.clone()))
            .collect();
        let range = pos + 1..pos + 1 + old.len();
        let n_new = spans.len();
        ids.splice(range.clone(), (0..n_new).map(|k| old.get(k).copied()));
        expected.splice(range, spans);
        let out = self.commit_structural(new, ids, expected)?;
        Ok(out[pos + 1..pos + 1 + n_new].to_vec())
    }

    /// Swap two adjacent runs of consecutive blocks: `a` directly followed by `b` (each in file
    /// order) becomes `b` then `a`. A run moves with the blank lines after each of its blocks;
    /// stray lines between the runs stay where they are. Ids follow the blocks. The caller checks
    /// that the swap is allowed (see [`Document::move_neighbour`]).
    pub fn swap_runs(&mut self, a: &[BlockId], b: &[BlockId]) -> Result<(), EditError> {
        let infos = self.infos();
        let (Some(a0), false) = (a.first(), b.is_empty()) else {
            return Err(EditError::NoNeighbour);
        };
        let pa = infos
            .iter()
            .position(|i| i.id == *a0)
            .ok_or(EditError::UnknownBlock)?;
        let pb = pa + a.len();
        let consecutive = a
            .iter()
            .chain(b)
            .enumerate()
            .all(|(k, id)| infos.get(pa + k).map(|i| i.id) == Some(*id));
        if !consecutive {
            return Err(EditError::NoNeighbour);
        }
        let (pa_last, pb_last) = (pb - 1, pb + b.len() - 1);
        let a_region = infos[pa].span.start..infos[pa_last].unit.end;
        let strays = infos[pa_last].unit.end..infos[pb].span.start;
        let b_region = infos[pb].span.start..infos[pb_last].unit.end;

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
        ids[pa..=pb_last].rotate_left(a.len());
        let mut expected: Vec<Vec<String>> = infos
            .iter()
            .map(|i| self.span_texts(i.span.clone()))
            .collect();
        expected[pa..=pb_last].rotate_left(a.len());
        self.commit_structural(new, ids, expected).map(|_| ())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::filter::parse as parse_doc;

    /// The economy banner (three lines, no trailing newline); the section runs to the end of the
    /// file, so there is no end marker.
    const BEGIN: &str = concat!(
        "################################################################################\n",
        "# ff:economy-begin Economy (generated by FerrisFilter, do not edit)\n",
        "################################################################################"
    );
    const END: &str = "";

    #[test]
    fn begin_const_is_the_real_banner() {
        assert_eq!(
            BEGIN,
            format!(
                "{}\n{}\n{}",
                section::RULE_LINE,
                section::BEGIN_LINE,
                section::RULE_LINE
            )
        );
    }

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
    fn insert_top_basic_and_fresh_id() {
        let mut d = p("Show
    Class \"Rings\"
");
        let before = ids(&d);
        let id = d.insert_rule_at_top(v(&["Show"])).unwrap();
        assert_eq!(
            s(&d),
            "Show

Show
    Class \"Rings\"
"
        );
        assert!(!before.contains(&id));
        assert_eq!(ids(&d), vec![id, before[0]]);
    }

    #[test]
    fn insert_top_keeps_first_block_meta() {
        let mut d = p("# ff:name X
Show
");
        let old = ids(&d)[0];
        d.insert_rule_at_top(v(&["Show"])).unwrap();
        assert_eq!(
            s(&d),
            "Show

# ff:name X
Show
"
        );
        assert_eq!(d.meta_get(old, "name"), Some("X"));
    }

    #[test]
    fn insert_top_after_free_preamble() {
        let mut d = p("# header

Show
");
        d.insert_rule_at_top(v(&["Show"])).unwrap();
        assert_eq!(
            s(&d),
            "# header

Show

Show
"
        );
    }

    #[test]
    fn insert_top_above_disabled_block() {
        let mut d = p("# Show\n# \tRarity Rare\n");
        d.insert_rule_at_top(v(&["Show"])).unwrap();
        assert_eq!(s(&d), "Show\n\n# Show\n# \tRarity Rare\n");
    }

    #[test]
    fn insert_top_with_economy_after_rules() {
        let tail = format!(
            "Show # A
  Rarity Rare

{BEGIN}

Hide # e

{END}
"
        );
        let mut d = p(&tail);
        d.insert_rule_at_top(v(&["Show # N"])).unwrap();
        let out = s(&d);
        assert_eq!(
            out,
            format!(
                "Show # N

{tail}"
            )
        );
        assert!(out.ends_with(&tail));
        assert!(section::find(&d).unwrap().is_some());
    }

    #[test]
    fn insert_top_economy_only_lands_above_begin() {
        let src = format!(
            "{BEGIN}

Hide

{END}
"
        );
        let mut d = p(&src);
        d.insert_rule_at_top(v(&["Show # N"])).unwrap();
        assert_eq!(
            s(&d),
            format!(
                "Show # N

{src}"
            )
        );
        let src = format!(
            "# note
{BEGIN}

Hide

{END}
"
        );
        let mut d = p(&src);
        d.insert_rule_at_top(v(&["Show # N"])).unwrap();
        assert_eq!(
            s(&d),
            format!(
                "# note

Show # N

{BEGIN}

Hide

{END}
"
            )
        );
        let sec = section::find(&d).unwrap().unwrap();
        assert!(d.blocks().next().unwrap().header_line() < sec.begin);
    }

    #[test]
    fn insert_top_keeps_group_run_intact() {
        let run = "# ff:group abc 1/2
Show
  BaseArmour 0

# ff:group abc 2/2
Show
  BaseEvasion 0
";
        let mut d = p(run);
        let old = ids(&d);
        let id = d.insert_rule_at_top(v(&["Show"])).unwrap();
        assert_eq!(
            s(&d),
            format!(
                "Show

{run}"
            )
        );
        assert_eq!(ids(&d), vec![id, old[0], old[1]]);
    }

    #[test]
    fn insert_top_no_blocks_appends() {
        let mut d = p("");
        d.insert_rule_at_top(v(&["Show"])).unwrap();
        assert_eq!(
            s(&d),
            "Show
"
        );
        let mut d = p("# note
");
        d.insert_rule_at_top(v(&["Show"])).unwrap();
        assert_eq!(
            s(&d),
            "# note

Show
"
        );
        let src = format!(
            "# note

{BEGIN}

{END}
"
        );
        let mut d = p(&src);
        d.insert_rule_at_top(v(&["Show"])).unwrap();
        assert_eq!(
            s(&d),
            format!(
                "# note

Show

{BEGIN}

{END}
"
            )
        );
    }

    #[test]
    fn insert_top_crlf() {
        // Escaped literals on purpose: a real line break in the source would be LF-only.
        let mut d = p("Hide\r\n");
        d.insert_rule_at_top(v(&["Show"])).unwrap();
        assert_eq!(s(&d), "Show\r\n\r\nHide\r\n");
        let mut d = p("Show\r\nHide");
        d.insert_rule_at_top(v(&["Show"])).unwrap();
        assert_eq!(s(&d), "Show\r\n\r\nShow\r\nHide");
        // The line ending of the first line decides, even in a mixed file.
        let mut d = p("Hide\r\nShow\n");
        d.insert_rule_at_top(v(&["Show"])).unwrap();
        assert_eq!(s(&d), "Show\r\n\r\nHide\r\nShow\n");
    }

    #[test]
    fn insert_top_rejects_bad_input_and_damaged_economy() {
        let src = "Show
";
        let mut d = p(src);
        assert!(matches!(
            d.insert_rule_at_top(v(&["Show
"])),
            Err(EditError::EmbeddedNewline)
        ));
        assert!(matches!(
            d.insert_rule_at_top(v(&["Show", "Hide"])),
            Err(EditError::NotOneBlock)
        ));
        assert!(matches!(
            d.insert_rule_at_top(v(&["Show", "  Rarity =< Rare"])),
            Err(EditError::InvalidBlock { .. })
        ));
        assert_eq!(s(&d), src);
        let src = format!(
            "Show
{}
",
            section::BEGIN_LINE
        );
        let mut d = p(&src);
        assert!(matches!(
            d.insert_rule_at_top(v(&["Show"])),
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
        d.insert_rule_at_top(v(&["Minimal"])).unwrap();
        let all = ids(&d);
        d.duplicate_block(all[1]).unwrap();
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

    #[test]
    fn swap_runs_moves_a_run_as_a_unit() {
        let src = "Show # A\n\nShow # B1\n\nShow # B2\n\n# stray\n\nShow # C\n";
        let mut d = p(src);
        let i = ids(&d);
        d.swap_runs(&[i[0]], &[i[1], i[2]]).unwrap();
        assert_eq!(
            s(&d),
            "Show # B1\n\nShow # B2\n\nShow # A\n\n# stray\n\nShow # C\n"
        );
        assert_eq!(ids(&d), vec![i[1], i[2], i[0], i[3]]);
        // Its own inverse.
        d.swap_runs(&[i[1], i[2]], &[i[0]]).unwrap();
        assert_eq!(s(&d), src);
        // Runs must be adjacent and in order.
        assert!(d.swap_runs(&[i[0]], &[i[2]]).is_err());
        assert!(d.swap_runs(&[i[1]], &[i[0]]).is_err());
    }

    #[test]
    fn replace_blocks_after_inserts_replaces_and_removes() {
        let src = "Show # A\n\nShow # B\n\nShow # C\n";
        let mut d = p(src);
        let i = ids(&d);
        let new = d
            .replace_blocks_after(i[0], &[], vec![v(&["# ff:x 1", "Show # N"])])
            .unwrap();
        assert_eq!(
            s(&d),
            "Show # A\n\n# ff:x 1\nShow # N\n\nShow # B\n\nShow # C\n"
        );
        assert!(!i.contains(&new[0]));
        // Replace N and B with one block that keeps N's id.
        let out = d
            .replace_blocks_after(i[0], &[new[0], i[1]], vec![v(&["Hide"])])
            .unwrap();
        assert_eq!(out, vec![new[0]]);
        assert_eq!(s(&d), "Show # A\n\nHide\n\nShow # C\n");
        // Remove it again; the rest keeps its bytes.
        d.replace_blocks_after(i[0], &[new[0]], Vec::new()).unwrap();
        assert_eq!(s(&d), "Show # A\n\nShow # C\n");
        // `old` must follow the anchor directly.
        assert!(d.replace_blocks_after(i[2], &[i[0]], Vec::new()).is_err());
    }

    #[test]
    fn replace_blocks_after_keeps_crlf_and_a_missing_final_newline() {
        let mut d = p("Show # A\r\n  Rarity Rare");
        let i = ids(&d);
        d.replace_blocks_after(i[0], &[], vec![v(&["Show # N"])])
            .unwrap();
        assert_eq!(s(&d), "Show # A\r\n  Rarity Rare\r\n\r\nShow # N");
    }
}
