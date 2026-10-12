//! Minimal block-scoped edit API (PARSE-04): enable/disable, replace a block's lines, set or
//! remove a `#ff:` key.
//!
//! Every operation validates first and mutates after: the candidate line vector is rebuilt and
//! checked before it replaces the document's lines, so an `Err` leaves the [`Document`] untouched.
//! Only the edited block's lines change; all other bytes stay as they were. After a successful
//! edit the whole index is rebuilt over the new lines and every block keeps its [`BlockId`] by
//! position. The rebuilt index is checked against the old one: no block is added or removed, and
//! every other item (block, comment run, import, opaque line) keeps its line range, shifted only
//! by the edit's line-count change, and its classification. An edit that would re-attribute lines
//! between other blocks is refused with [`EditError::WouldReattributeLines`].

use thiserror::Error;

use super::block::{BlockId, BlockKeyword, DisabledState, Item};
use super::diag::{FilterDiagnostic, Severity};
use super::disabled;
use super::lines::{Eol, Line};
use super::parse;
use super::Document;

#[derive(Debug, Error)]
pub enum EditError {
    #[error("no block with that id")]
    UnknownBlock,
    #[error("the disabled block cannot be enabled ({} problem(s))", reasons.len())]
    CannotEnable { reasons: Vec<FilterDiagnostic> },
    #[error("the replacement would produce an invalid enabled block ({} problem(s))", diagnostics.len())]
    InvalidBlock { diagnostics: Vec<FilterDiagnostic> },
    /// The commented-out form would not read back as this same block, so it could not be enabled
    /// again unchanged (e.g. a compact header with trailing tokens such as `Show Continue`, or a
    /// valid block directly followed by a comment line and then a commented condition, which in
    /// disabled form reads as a rule with a cut-off condition).
    #[error("the block cannot be disabled as-is: its commented-out form would not read back as the same block; fix the block first")]
    CannotDisable,
    #[error("the edit would not leave exactly one block in place")]
    NotOneBlock,
    #[error("replacement lines must not contain line breaks")]
    EmbeddedNewline,
    #[error("invalid metadata: {0}")]
    InvalidMeta(String),
    /// The edit would change which lines belong to some other block or comment run (e.g. a
    /// commented condition moving from one block to its neighbour), so it was not applied.
    #[error("the edit would change which lines belong to the surrounding blocks; nothing was changed")]
    WouldReattributeLines,
}

enum Fail {
    /// The edited block itself, or the block count, came out wrong.
    Structure,
    /// Some other item changed its range or classification.
    Neighbours,
    Errors(Vec<FilterDiagnostic>),
}

/// Range and classification of one top-level item, comparable across an edit.
#[derive(PartialEq, Eq)]
enum Sig {
    Trivia(usize, usize),
    Import(usize),
    Opaque(usize),
    Block {
        lead: usize,
        header: usize,
        end: usize,
        keyword: BlockKeyword,
        enabled: bool,
        can_enable: bool,
        body: usize,
    },
}

fn sig(it: &Item, shift: isize) -> Sig {
    let m = |i: usize| i.saturating_add_signed(shift);
    match it {
        Item::Trivia(r) => Sig::Trivia(m(r.start), m(r.end)),
        Item::Import { line, .. } => Sig::Import(m(*line)),
        Item::Opaque(line) => Sig::Opaque(m(*line)),
        Item::Block(b) => Sig::Block {
            lead: m(b.lead_start),
            header: m(b.header),
            end: m(b.end),
            keyword: b.keyword,
            enabled: b.is_enabled(),
            can_enable: b.can_enable(),
            body: b.body.len(),
        },
    }
}

struct Expect {
    header: usize,
    owned_len: usize,
    enabled: Option<bool>,
    reject_errors: bool,
    /// The edited block must come out enableable: disabling a clean live block must never leave a
    /// disabled rule that cannot be turned back on.
    enableable: bool,
}

impl Document {
    fn block_pos(&self, id: BlockId) -> Result<usize, EditError> {
        self.blocks()
            .position(|b| b.id() == id)
            .ok_or(EditError::UnknownBlock)
    }

    /// Terminator for lines we create: the header's, or the nearest real one above when the header
    /// is an unterminated final line.
    fn fresh_eol(&self, header: usize) -> Eol {
        self.lines[..=header]
            .iter()
            .rev()
            .map(|l| l.eol())
            .find(|e| *e != Eol::None)
            .unwrap_or(Eol::Lf)
    }

    /// Rebuild the index over `new_lines`, check the edited block at position `pos`, then commit.
    fn commit(&mut self, new_lines: Vec<Line>, pos: usize, ex: Expect) -> Result<(), Fail> {
        let old_ids: Vec<BlockId> = self.blocks().map(|b| b.id()).collect();
        let mut scratch = 0u64;
        let (mut items, diags) = parse::build(&new_lines, &mut scratch);
        let mut idx = 0usize;
        let mut span = None;
        for it in items.iter_mut() {
            if let Item::Block(b) = it {
                let Some(id) = old_ids.get(idx) else {
                    return Err(Fail::Structure);
                };
                b.id = *id;
                if idx == pos {
                    span = Some((
                        b.header,
                        b.end - b.header,
                        b.is_enabled(),
                        b.lead_start,
                        b.end,
                        b.can_enable(),
                    ));
                }
                idx += 1;
            }
        }
        if idx != old_ids.len() {
            return Err(Fail::Structure);
        }
        let Some((header, owned_len, enabled, lead, end, can_enable)) = span else {
            return Err(Fail::Structure);
        };
        if header != ex.header || owned_len != ex.owned_len {
            return Err(Fail::Structure);
        }
        if ex.enableable && !can_enable {
            return Err(Fail::Structure);
        }
        if ex.enabled.is_some_and(|e| e != enabled) {
            return Err(Fail::Structure);
        }
        self.check_neighbours(&items, pos, new_lines.len())?;
        if ex.reject_errors && enabled {
            // Warnings count too: an unknown keyword or trailing header tokens in a block the
            // tool just enabled is still a block the game refuses (D-11).
            let errs: Vec<FilterDiagnostic> = diags
                .iter()
                .filter(|d| {
                    matches!(d.severity, Severity::Error | Severity::Warning)
                        && d.line > lead
                        && d.line <= end
                })
                .cloned()
                .collect();
            if !errs.is_empty() {
                return Err(Fail::Errors(errs));
            }
        }
        self.lines = new_lines;
        self.items = items;
        self.diagnostics = diags;
        Ok(())
    }

    /// Every item other than the edited block (position `pos` among blocks) must keep its range,
    /// shifted by the line-count change when it lies below the edit, and its classification. The
    /// edited block must keep its leading-run start. Edits only touch lines inside the edited
    /// block's span, so anything else is a re-attribution the caller did not ask for.
    fn check_neighbours(&self, items: &[Item], pos: usize, new_len: usize) -> Result<(), Fail> {
        if items.len() != self.items.len() {
            return Err(Fail::Neighbours);
        }
        let delta = new_len as isize - self.lines.len() as isize;
        let mut blocks_seen = 0usize;
        let mut after = false;
        for (old, new) in self.items.iter().zip(items) {
            if let (Item::Block(o), Item::Block(n)) = (old, new) {
                if blocks_seen == pos {
                    blocks_seen += 1;
                    after = true;
                    if o.lead_start != n.lead_start {
                        return Err(Fail::Neighbours);
                    }
                    continue;
                }
                blocks_seen += 1;
            }
            if sig(old, if after { delta } else { 0 }) != sig(new, 0) {
                return Err(Fail::Neighbours);
            }
        }
        Ok(())
    }

    /// Enable or disable a block by adding/removing exactly one `#` per owned line (D-08).
    pub fn set_enabled(&mut self, id: BlockId, enabled: bool) -> Result<(), EditError> {
        let pos = self.block_pos(id)?;
        let b = self.block(id).ok_or(EditError::UnknownBlock)?;
        if b.is_enabled() == enabled {
            return Ok(());
        }
        if enabled && !b.can_enable() {
            let reasons = match b.disabled_state() {
                Some(DisabledState::Invalid(r)) => r.clone(),
                _ => Vec::new(),
            };
            return Err(EditError::CannotEnable { reasons });
        }
        let owned = b.owned();
        // A live block with no Error/Warning diagnostic in its span is valid grammar, so its
        // disabled form must read back as an enableable rule. Otherwise disabling would strand it
        // (iteration-3 CR-01: e.g. a comment line followed by a commented condition right below
        // the block, which in disabled form reads as a rule with a dropped condition).
        let clean = !enabled
            && !self.diagnostics.iter().any(|d| {
                matches!(d.severity, Severity::Error | Severity::Warning)
                    && d.line > owned.start
                    && d.line <= owned.end
            });
        let ex = Expect {
            header: b.header_line(),
            owned_len: owned.len(),
            enabled: Some(enabled),
            reject_errors: enabled,
            enableable: clean,
        };
        let mut new_lines = self.lines.clone();
        for i in owned {
            let l = &self.lines[i];
            let text = if enabled {
                disabled::enabled_text(l.text()).into_owned()
            } else {
                format!("#{}", l.text())
            };
            new_lines[i] = Line::new(text, l.eol());
        }
        self.commit(new_lines, pos, ex).map_err(|f| match f {
            Fail::Structure if !enabled => EditError::CannotDisable,
            Fail::Structure => EditError::NotOneBlock,
            Fail::Neighbours => EditError::WouldReattributeLines,
            Fail::Errors(reasons) => EditError::CannotEnable { reasons },
        })
    }

    /// Replace the block's owned lines (header through last body line); leading comments stay.
    pub fn replace_block_lines(
        &mut self,
        id: BlockId,
        new_lines: Vec<String>,
    ) -> Result<(), EditError> {
        if new_lines.iter().any(|s| s.contains(['\n', '\r'])) {
            return Err(EditError::EmbeddedNewline);
        }
        let pos = self.block_pos(id)?;
        let b = self.block(id).ok_or(EditError::UnknownBlock)?;
        let owned = b.owned();
        let header = b.header_line();
        let n = new_lines.len();

        let eol = self.fresh_eol(header);
        let last_unterminated =
            owned.end == self.lines.len() && self.lines[owned.end - 1].eol() == Eol::None;
        let cand: Vec<Line> = new_lines
            .into_iter()
            .enumerate()
            .map(|(i, t)| {
                let e = if i + 1 == n && last_unterminated {
                    Eol::None
                } else {
                    eol
                };
                Line::new(t, e)
            })
            .collect();

        // Isolated classification: exactly one block covering every candidate line.
        let mut scratch = 0u64;
        let (items, diags) = parse::build(&cand, &mut scratch);
        let one = items.len() == 1
            && matches!(&items[0], Item::Block(c) if c.header == 0 && c.end == n);
        if !one {
            return Err(EditError::NotOneBlock);
        }
        if let Item::Block(c) = &items[0] {
            if c.is_enabled() {
                // Unknown keywords and trailing header tokens are Warnings when loaded from a
                // file, but an enabled block the tool writes must be fully valid (D-11).
                let errs: Vec<FilterDiagnostic> = diags
                    .iter()
                    .filter(|d| matches!(d.severity, Severity::Error | Severity::Warning))
                    .cloned()
                    .collect();
                if !errs.is_empty() {
                    return Err(EditError::InvalidBlock { diagnostics: errs });
                }
            }
        }

        let mut lines = self.lines[..owned.start].to_vec();
        lines.extend(cand);
        lines.extend_from_slice(&self.lines[owned.end..]);
        let ex = Expect {
            header,
            owned_len: n,
            enabled: None,
            reject_errors: false,
            enableable: false,
        };
        self.commit(lines, pos, ex).map_err(|f| match f {
            Fail::Structure => EditError::NotOneBlock,
            Fail::Neighbours => EditError::WouldReattributeLines,
            Fail::Errors(diagnostics) => EditError::InvalidBlock { diagnostics },
        })
    }

    /// Set a `#ff:<key> <value>` setting: rewrite an existing line, or insert one directly above
    /// the header (D-05).
    pub fn set_meta(&mut self, id: BlockId, key: &str, value: &str) -> Result<(), EditError> {
        if key.is_empty()
            || !key
                .bytes()
                .all(|c| c.is_ascii_alphanumeric() || c == b'_' || c == b'-')
        {
            return Err(EditError::InvalidMeta(format!("bad key `{key}`")));
        }
        if value.contains(['\n', '\r']) {
            return Err(EditError::InvalidMeta("value contains a line break".into()));
        }
        // The reader trims surrounding spaces/tabs, so store exactly what reads back.
        let value = value.trim_matches([' ', '\t']);
        if value.is_empty() {
            return Err(EditError::InvalidMeta(
                "empty value; use remove_meta to clear a key".into(),
            ));
        }
        let pos = self.block_pos(id)?;
        let b = self.block(id).ok_or(EditError::UnknownBlock)?;
        let header = b.header_line();
        let owned_len = b.owned().len();
        let existing = b
            .leading()
            .find(|&i| self.lines[i].ff_marker().is_some_and(|(k, _)| k == key));
        let text = format!("#ff:{key} {value}");
        let mut lines = self.lines.clone();
        let new_header = match existing {
            Some(i) => {
                lines[i] = Line::new(text, self.lines[i].eol());
                header
            }
            None => {
                lines.insert(header, Line::new(text, self.fresh_eol(header)));
                header + 1
            }
        };
        let ex = Expect {
            header: new_header,
            owned_len,
            enabled: None,
            reject_errors: false,
            enableable: false,
        };
        self.commit(lines, pos, ex)
            .map_err(|_| EditError::WouldReattributeLines)
    }

    /// Remove every `#ff:<key>` line attached to the block. Removing an absent key is a no-op.
    pub fn remove_meta(&mut self, id: BlockId, key: &str) -> Result<(), EditError> {
        let pos = self.block_pos(id)?;
        let b = self.block(id).ok_or(EditError::UnknownBlock)?;
        let header = b.header_line();
        let owned_len = b.owned().len();
        let doomed: Vec<usize> = b
            .leading()
            .filter(|&i| self.lines[i].ff_marker().is_some_and(|(k, _)| k == key))
            .collect();
        if doomed.is_empty() {
            return Ok(());
        }
        let lines: Vec<Line> = self
            .lines
            .iter()
            .enumerate()
            .filter(|(i, _)| !doomed.contains(i))
            .map(|(_, l)| l.clone())
            .collect();
        let ex = Expect {
            header: header - doomed.len(),
            owned_len,
            enabled: None,
            reject_errors: false,
            enableable: false,
        };
        self.commit(lines, pos, ex)
            .map_err(|_| EditError::WouldReattributeLines)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::filter::parse as parse_doc;

    fn p(s: &str) -> Document {
        parse_doc(s.as_bytes()).unwrap()
    }
    fn ids(d: &Document) -> Vec<BlockId> {
        d.blocks().map(|b| b.id()).collect()
    }
    fn s(d: &Document) -> String {
        d.serialize()
    }

    #[test]
    fn disable_then_enable_crlf() {
        let src = "Show\r\n\tRarity Rare\r\n";
        let mut d = p(src);
        let id = ids(&d)[0];
        d.set_enabled(id, false).unwrap();
        assert_eq!(s(&d), "#Show\r\n#\tRarity Rare\r\n");
        let b = d.block(id).unwrap();
        assert!(!b.is_enabled() && b.can_enable());
        d.set_enabled(id, true).unwrap();
        assert_eq!(s(&d), src);
    }

    #[test]
    fn roundtrip_with_interior_blank_and_comment() {
        let src = "Show\n\tRarity Rare\n\n# mid\n\tClass \"Bows\"\n";
        let mut d = p(src);
        let id = ids(&d)[0];
        d.set_enabled(id, false).unwrap();
        assert_eq!(s(&d), "#Show\n#\tRarity Rare\n#\n## mid\n#\tClass \"Bows\"\n");
        d.set_enabled(id, true).unwrap();
        assert_eq!(s(&d), src);
    }

    #[test]
    fn disable_enable_with_trailing_commented_condition() {
        // WR-03: the trailing commented-out line moves with its block in both states.
        let src = "Show\n\tRarity Rare\n#\tSetFontSize 40\nHide\n";
        let mut d = p(src);
        let id = ids(&d)[0];
        d.set_enabled(id, false).unwrap();
        assert_eq!(s(&d), "#Show\n#\tRarity Rare\n##\tSetFontSize 40\nHide\n");
        assert_eq!(d.block(id).unwrap().owned(), 0..3);
        d.set_enabled(id, true).unwrap();
        assert_eq!(s(&d), src);
    }

    #[test]
    fn disable_blocks_with_bad_or_unknown_last_line() {
        // WR-02: the remedy for a broken enabled block is disabling it, so these must work. The
        // disabled form is then Invalid (cannot be enabled back until fixed).
        for src in [
            "Show\n\tRarity Rare\n\tFutureKw 3\n",
            "Show\n\tRarity Rare\n\t123\n",
            "Show\n\tRarity Rare\n\tRarity =< Rare\n",
            "Show\n  Rarity Rare\n  FutureKw 3\nHide\n",
            "Show\n\tRarity Rare\n# Class weapons below\nHide\n",
            "Show\n\tRarity Rare\n#\tSetFontSize 40\nHide\n",
        ] {
            let mut d = p(src);
            let id = ids(&d)[0];
            d.set_enabled(id, false)
                .unwrap_or_else(|e| panic!("{src:?}: {e}"));
            assert!(!d.block(id).unwrap().is_enabled(), "{src:?}");
        }
        // Iteration-3 CR-01: prose then a commented condition is the next block's leading run.
        // Disabled, the text would read as a rule whose condition was cut off, which cannot be
        // enabled; a clean block is never disabled into that, so the disable is refused.
        let src = "Show\n\tRarity Rare\n# note\n#\tClass \"Bows\"\nHide\n";
        let mut d = p(src);
        let id = ids(&d)[0];
        assert!(matches!(d.set_enabled(id, false), Err(EditError::CannotDisable)));
        assert_eq!(s(&d), src);
        // Enabling the disabled unknown-keyword block back is refused, bytes untouched.
        let mut d = p("Show\n\tRarity Rare\n\tFutureKw 3\n");
        let id = ids(&d)[0];
        d.set_enabled(id, false).unwrap();
        assert!(!d.block(id).unwrap().can_enable());
        let before = s(&d);
        assert!(matches!(
            d.set_enabled(id, true),
            Err(EditError::CannotEnable { .. })
        ));
        assert_eq!(s(&d), before);
    }

    /// Every short document over a pool of tricky lines: for each block, `can_enable()` agrees
    /// with `set_enabled(true)`, a successful disable re-enables byte-identically when the disabled
    /// form is enableable, metadata edits never move another block's lines, and a refused edit
    /// leaves the bytes untouched.
    #[test]
    fn exhaustive_small_documents_agree() {
        // Tab-, space-, mixed- and un-indented lines (iteration-3 CR-01 / WR-01), prose that can
        // sit between conditions (`# note`, `# Corrupted items`, `#  Note: x`), and the header-only
        // heading shapes of iteration-3 WR-02.
        const POOL: [&str; 16] = [
            "\tRarity Rare",
            "#\tRarity Rare",
            "#\tSetFontSize 40",
            "##\tSetFontSize 40",
            "# note",
            "# Corrupted items",
            "#",
            "",
            "#\tFutureKw 3",
            "#ff:name x",
            "#  Note: x",
            "\tClass \"Bows\"",
            "    Class \"Bows\"",
            "#    Class \"Bows\"",
            "#Rarity Rare",
            "Rarity Rare",
        ];
        let mut docs = 0usize;
        for head in ["Show", "#Show"] {
            for code in 0..POOL.len().pow(4) {
                let mut lines = vec![head];
                let mut c = code;
                for _ in 0..4 {
                    lines.push(POOL[c % POOL.len()]);
                    c /= POOL.len();
                }
                lines.push("Hide");
                let src = lines.join("\n") + "\n";
                check_agreement(&src);
                docs += 1;
            }
        }
        assert_eq!(docs, 2 * 16usize.pow(4));
    }

    /// Clean commented grammar lines (any indentation) in the run of commented lines below the
    /// disabled header `h`, up to a header, a `#ff:` line or a non-comment line: the conditions and
    /// actions a reader takes the rule to contain.
    fn commented_conditions(d: &Document, h: usize) -> usize {
        let mut n = 0;
        for j in h + 1..d.line_count() {
            let text = d.line_text(j);
            let Some(inner) = disabled::uncomment_once(text) else {
                break;
            };
            if d.line_at(j).ff_marker().is_some() {
                break;
            }
            match disabled::classify_inner(text, inner, j, None) {
                disabled::Inner::Header => break,
                disabled::Inner::Body { problem: None, .. } => n += 1,
                _ => {}
            }
        }
        n
    }

    #[test]
    fn enable_never_drops_a_commented_condition() {
        // Iteration-3 CR-01: the review's inputs, enabled whole or refused.
        for src in [
            "#Show\n#\tRarity Rare\n#    Class \"Bows\"\n",
            "#Show\n#Rarity Rare\n#\tClass \"Bows\"\n",
            "#Hide\n#\tRarity Normal\n# only these bases\n#\tBaseType == \"X\"\n",
            "#Hide\n#\tRarity Normal\n# Corrupted items\n#\n#\tBaseType == \"X\"\nShow\n",
        ] {
            let mut d = p(src);
            let id = ids(&d)[0];
            let want = commented_conditions(&d, 0);
            match d.set_enabled(id, true) {
                Ok(()) => assert!(d.block(id).unwrap().body().len() >= want, "{src:?}"),
                Err(EditError::CannotEnable { .. }) => assert_eq!(s(&d), src),
                Err(e) => panic!("{src:?}: {e}"),
            }
        }
    }

    /// Line ranges of every block other than `skip`, as `(leading start, end)`.
    fn spans_except(d: &Document, skip: BlockId) -> Vec<(BlockId, usize, usize)> {
        d.blocks()
            .filter(|b| b.id() != skip)
            .map(|b| (b.id(), b.span().start, b.span().end))
            .collect()
    }

    fn check_agreement(src: &str) {
        let d0 = p(src);
        for id in ids(&d0) {
            // Metadata edits either keep every other block's lines in place or are refused
            // with the bytes untouched (iteration-2 WR-03).
            let mut d = d0.clone();
            let others = spans_except(&d, id);
            let old_end = d.block(id).unwrap().span().end;
            match d.set_meta(id, "name", "y") {
                Ok(()) => {
                    assert_eq!(d.meta_get(id, "name"), Some("y"), "{src:?}");
                    let shifted: Vec<_> = others
                        .iter()
                        .map(|&(o, a, z)| {
                            if a >= old_end { (o, a + 1, z + 1) } else { (o, a, z) }
                        })
                        .collect();
                    assert_eq!(spans_except(&d, id), shifted, "{src:?}");
                    let before = s(&d);
                    match d.remove_meta(id, "name") {
                        // Removing what set_meta inserted restores the original bytes.
                        Ok(()) if d0.meta_get(id, "name").is_none() => {
                            assert_eq!(s(&d), src, "set+remove meta changed bytes")
                        }
                        Ok(()) => {}
                        Err(EditError::WouldReattributeLines) => {
                            assert!(d0.meta_get(id, "name").is_some(), "{src:?}");
                            assert_eq!(s(&d), before, "refused remove_meta changed bytes");
                        }
                        Err(e) => panic!("{src:?}: remove_meta: {e}"),
                    }
                }
                Err(_) => assert_eq!(s(&d), src, "refused set_meta changed bytes"),
            }
            let mut d = d0.clone();
            let b = d.block(id).unwrap();
            if b.is_enabled() {
                let owned = b.owned();
                let clean = !d.diagnostics().iter().any(|x| {
                    matches!(x.severity, Severity::Error | Severity::Warning)
                        && x.line > owned.start
                        && x.line <= owned.end
                });
                match d.set_enabled(id, false) {
                    Ok(()) => {
                        // A clean block is never disabled into a rule that cannot be enabled back.
                        assert!(!clean || d.block(id).unwrap().can_enable(), "{src:?}");
                        if d.block(id).unwrap().can_enable() {
                            d.set_enabled(id, true)
                                .unwrap_or_else(|e| panic!("{src:?}: re-enable: {e}"));
                            assert_eq!(s(&d), src, "re-enable changed bytes");
                        }
                    }
                    Err(_) => assert_eq!(s(&d), src, "refused disable changed bytes"),
                }
            } else {
                let can = b.can_enable();
                let want = commented_conditions(&d, b.header_line());
                let r = d.set_enabled(id, true);
                assert_eq!(can, r.is_ok(), "{src:?}: can_enable={can} but {r:?}");
                match r {
                    Ok(()) => {
                        // Iteration-3 CR-01: enabling never writes a rule with fewer conditions
                        // than the commented run it came from.
                        let got = d.block(id).unwrap().body().len();
                        assert!(got >= want, "{src:?}: enabled {got} body lines, text has {want}");
                        d.set_enabled(id, false)
                            .unwrap_or_else(|e| panic!("{src:?}: re-disable: {e}"));
                        assert_eq!(s(&d), src, "re-disable changed bytes");
                    }
                    Err(_) => assert_eq!(s(&d), src, "refused enable changed bytes"),
                }
            }
        }
    }

    #[test]
    fn disable_compact_header_is_dedicated_error() {
        // `#Show Continue` is not a disabled header (it would swallow prose like
        // `# Show items above 80`), so the block cannot be disabled until its header is fixed.
        let src = "Show Continue\n\tRarity Rare\n";
        let mut d = p(src);
        let id = ids(&d)[0];
        assert!(matches!(d.set_enabled(id, false), Err(EditError::CannotDisable)));
        assert_eq!(s(&d), src);
    }

    #[test]
    fn disable_leaves_leading_comments() {
        let src = "# note\n#ff:name x\nShow\n\tRarity Rare\n";
        let mut d = p(src);
        let id = ids(&d)[0];
        d.set_enabled(id, false).unwrap();
        assert_eq!(s(&d), "# note\n#ff:name x\n#Show\n#\tRarity Rare\n");
        assert_eq!(d.meta_get(id, "name"), Some("x"));
    }

    #[test]
    fn enable_invalid_refused() {
        let src = "#Show\n#\tRarity =< Rare\n";
        let mut d = p(src);
        let id = ids(&d)[0];
        match d.set_enabled(id, true) {
            Err(EditError::CannotEnable { reasons }) => assert!(!reasons.is_empty()),
            other => panic!("{other:?}"),
        }
        assert_eq!(s(&d), src);
    }

    #[test]
    fn enable_never_turns_prose_into_grammar() {
        // CR-01 / iteration-2 WR-04: prose ends a disabled block, so enabling leaves it a comment.
        let src = "#Show\n#\tRarity Rare\n# note here\n#\tSetFontSize 40 extra\n";
        let mut d = p(src);
        let id = ids(&d)[0];
        d.set_enabled(id, true).unwrap();
        assert_eq!(s(&d), "Show\n\tRarity Rare\n# note here\n#\tSetFontSize 40 extra\n");
        assert!(d.diagnostics().is_empty());
        // Iteration-3 CR-01: a clean condition below the prose would be dropped, so refuse.
        let src = "#Show\n#\tRarity Rare\n# note here\n#\tClass \"Bows\"\n";
        let mut d = p(src);
        let id = ids(&d)[0];
        assert!(matches!(
            d.set_enabled(id, true),
            Err(EditError::CannotEnable { .. })
        ));
        assert_eq!(s(&d), src);

        // A replacement whose prose sits between body lines is not one block, so it is refused.
        let src = "Show\n\tRarity Rare\n";
        let mut d = p(src);
        let id = ids(&d)[0];
        assert!(matches!(
            d.replace_block_lines(
                id,
                vec!["#Show".into(), "# hello world".into(), "#\tRarity Rare".into()],
            ),
            Err(EditError::NotOneBlock)
        ));
        assert_eq!(s(&d), src);
    }

    #[test]
    fn noop_enable_state() {
        let src = "Show\n\tRarity Rare\n";
        let mut d = p(src);
        let id = ids(&d)[0];
        d.set_enabled(id, true).unwrap();
        assert_eq!(s(&d), src);
        d.set_enabled(id, false).unwrap();
        d.set_enabled(id, false).unwrap();
        assert_eq!(s(&d), "#Show\n#\tRarity Rare\n");
    }

    #[test]
    fn disabling_error_block_clears_error() {
        let mut d = p("Show\n\tRarity =< Rare\n");
        assert!(d.diagnostics().iter().any(|x| x.severity == Severity::Error));
        let id = ids(&d)[0];
        d.set_enabled(id, false).unwrap();
        assert!(!d.diagnostics().iter().any(|x| x.severity == Severity::Error));
        assert!(!d.block(id).unwrap().can_enable());
    }

    #[test]
    fn replace_crlf() {
        let mut d = p("Hide\r\n\r\nShow\r\n\tRarity Rare\r\n\r\nHide\r\n");
        let all = ids(&d);
        d.replace_block_lines(all[1], vec!["Show".into(), "\tRarity Unique".into()])
            .unwrap();
        assert_eq!(
            s(&d),
            "Hide\r\n\r\nShow\r\n\tRarity Unique\r\n\r\nHide\r\n"
        );
        d.replace_block_lines(
            all[1],
            vec!["Show".into(), "\tRarity Unique".into(), "\tClass \"Bows\"".into()],
        )
        .unwrap();
        assert_eq!(
            s(&d),
            "Hide\r\n\r\nShow\r\n\tRarity Unique\r\n\tClass \"Bows\"\r\n\r\nHide\r\n"
        );
        assert_eq!(ids(&d), all);
    }

    #[test]
    fn replace_invalid_refused() {
        let src = "Show\n\tRarity Rare\n";
        let mut d = p(src);
        let id = ids(&d)[0];
        match d.replace_block_lines(id, vec!["Show".into(), "\tRarity =< Rare".into()]) {
            Err(EditError::InvalidBlock { diagnostics }) => {
                assert_eq!(diagnostics[0].line, 2);
            }
            other => panic!("{other:?}"),
        }
        assert_eq!(s(&d), src);
    }

    #[test]
    fn replace_with_warning_level_problems_refused() {
        // WR-04: unknown keywords and compact headers are not valid enabled grammar.
        let src = "Show\n\tRarity Rare\n";
        let mut d = p(src);
        let id = ids(&d)[0];
        for cand in [
            vec!["Show".to_string(), "\tFooBar 1".to_string()],
            vec!["Show Rarity Rare".to_string()],
        ] {
            assert!(
                matches!(
                    d.replace_block_lines(id, cand.clone()),
                    Err(EditError::InvalidBlock { .. })
                ),
                "{cand:?}"
            );
        }
        assert_eq!(s(&d), src);
        // A disabled candidate may still carry such lines.
        d.replace_block_lines(id, vec!["#Show".into(), "#\tFooBar 1".into()])
            .unwrap();
        assert!(!d.block(id).unwrap().can_enable());
    }

    #[test]
    fn replace_not_one_block_or_newline() {
        let src = "Show\n\tRarity Rare\n";
        let mut d = p(src);
        let id = ids(&d)[0];
        assert!(matches!(
            d.replace_block_lines(id, vec!["Show".into(), "Hide".into()]),
            Err(EditError::NotOneBlock)
        ));
        assert!(matches!(
            d.replace_block_lines(id, vec![]),
            Err(EditError::NotOneBlock)
        ));
        assert!(matches!(
            d.replace_block_lines(id, vec!["# only".into()]),
            Err(EditError::NotOneBlock)
        ));
        assert!(matches!(
            d.replace_block_lines(id, vec!["Show\n".into()]),
            Err(EditError::EmbeddedNewline)
        ));
        assert!(matches!(
            d.replace_block_lines(id, vec!["Show\r".into()]),
            Err(EditError::EmbeddedNewline)
        ));
        assert_eq!(s(&d), src);
    }

    #[test]
    fn replace_last_block_no_final_newline() {
        let mut d = p("Hide\nShow\n\tRarity Rare");
        let id = ids(&d)[1];
        d.replace_block_lines(id, vec!["Show".into(), "\tRarity Unique".into(), "\tSockets 3".into()])
            .unwrap();
        assert_eq!(s(&d), "Hide\nShow\n\tRarity Unique\n\tSockets 3");
    }

    #[test]
    fn replace_unterminated_header_only() {
        let mut d = p("Hide\nShow");
        let id = ids(&d)[1];
        d.replace_block_lines(id, vec!["Show".into(), "\tRarity Rare".into()])
            .unwrap();
        assert_eq!(s(&d), "Hide\nShow\n\tRarity Rare");
    }

    #[test]
    fn replace_with_disabled_candidate_accepted() {
        let mut d = p("Show\n\tRarity Rare\n");
        let id = ids(&d)[0];
        d.replace_block_lines(id, vec!["#Show".into(), "#\tRarity =< Rare".into()])
            .unwrap();
        assert!(!d.block(id).unwrap().can_enable());
        assert!(!d.diagnostics().iter().any(|x| x.severity == Severity::Error));
    }

    #[test]
    fn meta_insert_and_rewrite() {
        let mut d = p("Hide\r\n\r\nShow\r\n\tRarity Rare\r\n");
        let id = ids(&d)[1];
        d.set_meta(id, "name", "Exceptional 3 socks").unwrap();
        assert_eq!(
            s(&d),
            "Hide\r\n\r\n#ff:name Exceptional 3 socks\r\nShow\r\n\tRarity Rare\r\n"
        );
        assert_eq!(d.meta_get(id, "name"), Some("Exceptional 3 socks"));
        d.set_meta(id, "name", "new").unwrap();
        assert_eq!(
            s(&d),
            "Hide\r\n\r\n#ff:name new\r\nShow\r\n\tRarity Rare\r\n"
        );
        assert_eq!(d.meta_get(id, "name"), Some("new"));
        d.set_meta(id, "style", "S").unwrap();
        assert_eq!(d.meta_get(id, "style"), Some("S"));
    }

    #[test]
    fn meta_rewrite_keeps_other_lines() {
        let src = "# a\n#ff:name old\n#ff:style S\nShow\n";
        let mut d = p(src);
        let id = ids(&d)[0];
        d.set_meta(id, "name", "x").unwrap();
        assert_eq!(s(&d), "# a\n#ff:name x\n#ff:style S\nShow\n");
    }

    #[test]
    fn meta_value_round_trips_and_remove() {
        // WR-05: what is written is what reads back; empty is refused; remove_meta clears.
        let mut d = p("# a\nShow\n");
        let id = ids(&d)[0];
        d.set_meta(id, "name", "  padded \t").unwrap();
        assert_eq!(s(&d), "# a\n#ff:name padded\nShow\n");
        assert_eq!(d.meta_get(id, "name"), Some("padded"));
        assert!(matches!(d.set_meta(id, "name", ""), Err(EditError::InvalidMeta(_))));
        assert!(matches!(d.set_meta(id, "name", " \t "), Err(EditError::InvalidMeta(_))));
        assert_eq!(s(&d), "# a\n#ff:name padded\nShow\n");
        d.set_meta(id, "style", "S").unwrap();
        d.remove_meta(id, "name").unwrap();
        assert_eq!(s(&d), "# a\n#ff:style S\nShow\n");
        assert_eq!(d.meta_get(id, "name"), None);
        d.remove_meta(id, "name").unwrap();
        assert_eq!(s(&d), "# a\n#ff:style S\nShow\n");
        // Duplicates are all removed, other blocks untouched, ids stable.
        let mut d = p("#ff:name a\n#ff:name b\nShow\n\n#ff:name c\nHide\n");
        let all = ids(&d);
        d.remove_meta(all[0], "name").unwrap();
        assert_eq!(s(&d), "Show\n\n#ff:name c\nHide\n");
        assert_eq!(ids(&d), all);
        assert_eq!(d.meta_get(all[1], "name"), Some("c"));
        assert!(matches!(
            d.remove_meta(BlockId(9999), "name"),
            Err(EditError::UnknownBlock)
        ));
    }

    #[test]
    fn edit_never_reattributes_neighbour_lines() {
        // Iteration-2 WR-03: removing the `#ff:` line would hand `#	SetFontSize 40` from
        // Hide's leading comments to Show's trailing run. Refused, nothing changes.
        let src = "Show
	Rarity Rare
#ff:name x
#	SetFontSize 40
Hide
";
        let mut d = p(src);
        let all = ids(&d);
        assert_eq!(d.block(all[0]).unwrap().owned(), 0..2);
        assert_eq!(d.block(all[1]).unwrap().leading(), 2..4);
        assert!(matches!(
            d.remove_meta(all[1], "name"),
            Err(EditError::WouldReattributeLines)
        ));
        assert_eq!(s(&d), src);
        assert_eq!(d.meta_get(all[1], "name"), Some("x"));
        // Rewriting the value in place is fine.
        d.set_meta(all[1], "name", "z").unwrap();
        assert_eq!(d.block(all[0]).unwrap().owned(), 0..2);
        assert_eq!(d.meta_get(all[1], "name"), Some("z"));
        // Without anything for the neighbour to take, removal works.
        let mut d = p("Show
	Rarity Rare
#ff:name x
# about Hide
Hide
");
        let all = ids(&d);
        d.remove_meta(all[1], "name").unwrap();
        assert_eq!(s(&d), "Show
	Rarity Rare
# about Hide
Hide
");
    }

    #[test]
    fn meta_invalid() {
        let mut d = p("Show\n");
        let id = ids(&d)[0];
        assert!(matches!(d.set_meta(id, "bad key", "x"), Err(EditError::InvalidMeta(_))));
        assert!(matches!(d.set_meta(id, "", "x"), Err(EditError::InvalidMeta(_))));
        assert!(matches!(d.set_meta(id, "k", "a\nb"), Err(EditError::InvalidMeta(_))));
        assert_eq!(s(&d), "Show\n");
    }

    #[test]
    fn other_ids_survive_and_unknown_id() {
        let src = "Show\n\tRarity Rare\nHide\n\tClass \"Bows\"\n#Show\n#\tSockets 3\n";
        let mut d = p(src);
        let all = ids(&d);
        let headers: Vec<String> = d
            .blocks()
            .map(|b| d.lines()[b.header_line()].text().to_string())
            .collect();
        d.set_meta(all[0], "name", "n").unwrap();
        d.replace_block_lines(all[1], vec!["Hide".into(), "\tClass \"Wands\"".into()])
            .unwrap();
        d.set_enabled(all[2], true).unwrap();
        assert_eq!(ids(&d), all);
        for (i, id) in all.iter().enumerate() {
            let b = d.block(*id).unwrap();
            let h = d.lines()[b.header_line()].text();
            if i == 2 {
                assert_eq!(h, "Show");
            } else {
                assert_eq!(h, headers[i]);
            }
        }
        let mut other = p("Show\n");
        assert!(matches!(
            other.set_enabled(all[2], true),
            Err(EditError::UnknownBlock) | Ok(())
        ));
        let bogus = BlockId(9999);
        assert!(matches!(other.set_enabled(bogus, true), Err(EditError::UnknownBlock)));
        assert!(matches!(
            other.replace_block_lines(bogus, vec!["Show".into()]),
            Err(EditError::UnknownBlock)
        ));
        assert!(matches!(other.set_meta(bogus, "a", "b"), Err(EditError::UnknownBlock)));
    }
}
