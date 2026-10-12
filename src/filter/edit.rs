//! Minimal block-scoped edit API (PARSE-04): enable/disable, replace a block's lines, set a
//! `#ff:` key.
//!
//! Every operation validates first and mutates after: the candidate line vector is rebuilt and
//! checked before it replaces the document's lines, so an `Err` leaves the [`Document`] untouched.
//! Only the edited block's lines change; all other bytes stay as they were. After a successful
//! edit the whole index is rebuilt over the new lines and every block keeps its [`BlockId`] by
//! position (an edit never adds or removes a block, which is checked).

use thiserror::Error;

use super::block::{BlockId, DisabledState, Item};
use super::diag::{FilterDiagnostic, Severity};
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
    #[error("the edit would not leave exactly one block in place")]
    NotOneBlock,
    #[error("replacement lines must not contain line breaks")]
    EmbeddedNewline,
    #[error("invalid metadata: {0}")]
    InvalidMeta(String),
}

enum Fail {
    Structure,
    Errors(Vec<FilterDiagnostic>),
}

struct Expect {
    header: usize,
    owned_len: usize,
    enabled: Option<bool>,
    reject_errors: bool,
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
                    ));
                }
                idx += 1;
            }
        }
        if idx != old_ids.len() {
            return Err(Fail::Structure);
        }
        let Some((header, owned_len, enabled, lead, end)) = span else {
            return Err(Fail::Structure);
        };
        if header != ex.header || owned_len != ex.owned_len {
            return Err(Fail::Structure);
        }
        if ex.enabled.is_some_and(|e| e != enabled) {
            return Err(Fail::Structure);
        }
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
        let ex = Expect {
            header: b.header_line(),
            owned_len: owned.len(),
            enabled: Some(enabled),
            reject_errors: enabled,
        };
        let mut new_lines = self.lines.clone();
        for i in owned {
            let l = &self.lines[i];
            let text = if enabled {
                match l.text().find('#') {
                    Some(p) => {
                        let mut s = l.text().to_string();
                        s.remove(p);
                        s
                    }
                    None => l.text().to_string(),
                }
            } else {
                format!("#{}", l.text())
            };
            new_lines[i] = Line::new(text, l.eol());
        }
        self.commit(new_lines, pos, ex).map_err(|f| match f {
            Fail::Structure => EditError::NotOneBlock,
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
                let errs: Vec<FilterDiagnostic> = diags
                    .iter()
                    .filter(|d| d.severity == Severity::Error)
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
        };
        self.commit(lines, pos, ex).map_err(|f| match f {
            Fail::Structure => EditError::NotOneBlock,
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
        };
        self.commit(lines, pos, ex).map_err(|_| EditError::NotOneBlock)
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
    fn enable_with_interior_prose_refused() {
        // CR-01: neither a parsed nor a replaced disabled block may enable prose as grammar.
        let src = "#Show\n#\tRarity Rare\n# note here\n#\tClass \"Bows\"\n";
        let mut d = p(src);
        let id = ids(&d)[0];
        assert!(matches!(
            d.set_enabled(id, true),
            Err(EditError::CannotEnable { .. })
        ));
        assert_eq!(s(&d), src);

        let mut d = p("Show\n\tRarity Rare\n");
        let id = ids(&d)[0];
        d.replace_block_lines(
            id,
            vec!["#Show".into(), "# hello world".into(), "#\tRarity Rare".into()],
        )
        .unwrap();
        let before = s(&d);
        assert!(!d.block(id).unwrap().can_enable());
        assert!(d.set_enabled(id, true).is_err());
        assert_eq!(s(&d), before);
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
