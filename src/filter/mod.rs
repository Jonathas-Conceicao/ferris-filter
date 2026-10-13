//! Lossless PoE2 item-filter document model.
//!
//! Contract: the raw lines (text + their own terminator + a BOM flag) are the source of truth;
//! parsed values are a lookup layer built over them. Serializing an unedited [`Document`] returns
//! the input byte-for-byte. The library never prints and does no file IO; the only `Err` is
//! non-UTF-8 input ([`LoadError`]). Grammar problems become [`FilterDiagnostic`]s.

pub mod block;
pub mod diag;
mod disabled;
mod edit;
pub mod keywords;
mod lineedit;
pub mod lines;
pub mod meta;
mod parse;
pub mod section;
mod structure;
pub mod tokenize;

pub use block::{Block, BlockId, BlockKeyword, BodyLine, DisabledState, Item};
pub use diag::{FilterDiagnostic, LoadError, Severity};
pub use edit::EditError;
pub use lineedit::LinePatch;
pub use lines::{Eol, Line};

const BOM_BYTES: &[u8] = &[0xEF, 0xBB, 0xBF];

#[derive(Debug, Clone, Default)]
pub struct Document {
    bom: bool,
    lines: Vec<Line>,
    diagnostics: Vec<FilterDiagnostic>,
    items: Vec<Item>,
    next_id: u64,
}

/// Parse filter bytes. Fails only if the input is not valid UTF-8.
pub fn parse(input: &[u8]) -> Result<Document, LoadError> {
    let text = std::str::from_utf8(input).map_err(|e| LoadError::InvalidUtf8 {
        valid_up_to: e.valid_up_to(),
    })?;
    let (bom, lines) = lines::split(text);
    let mut next_id = 0u64;
    let (items, diagnostics) = parse::build(&lines, &mut next_id);
    Ok(Document {
        bom,
        lines,
        diagnostics,
        items,
        next_id,
    })
}

impl Document {
    pub fn has_bom(&self) -> bool {
        self.bom
    }
    pub fn lines(&self) -> &[Line] {
        &self.lines
    }
    pub fn diagnostics(&self) -> &[FilterDiagnostic] {
        &self.diagnostics
    }
    /// Top-level partition: every line index belongs to exactly one item, in order.
    pub fn items(&self) -> &[Item] {
        &self.items
    }
    pub fn blocks(&self) -> impl Iterator<Item = &Block> {
        self.items.iter().filter_map(|i| match i {
            Item::Block(b) => Some(b),
            _ => None,
        })
    }
    pub fn block(&self, id: BlockId) -> Option<&Block> {
        self.blocks().find(|b| b.id() == id)
    }
    /// `# ff:` settings attached to a block, in order, as `(key, value)` (missing value = "").
    pub fn block_meta(&self, id: BlockId) -> Vec<(&str, &str)> {
        let Some(b) = self.block(id) else {
            return Vec::new();
        };
        b.leading()
            .filter_map(|i| self.lines[i].ff_marker())
            .map(|(k, v)| (k, v.unwrap_or("")))
            .collect()
    }
    /// First `# ff:<key>` value attached to the block. Foreign header comments are never a name.
    pub fn meta_get(&self, id: BlockId, key: &str) -> Option<&str> {
        let b = self.block(id)?;
        b.leading()
            .filter_map(|i| self.lines[i].ff_marker())
            .find(|(k, _)| *k == key)
            .and_then(|(_, v)| v)
    }
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut out = Vec::new();
        if self.bom {
            out.extend_from_slice(BOM_BYTES);
        }
        for l in &self.lines {
            out.extend_from_slice(l.text().as_bytes());
            out.extend_from_slice(l.eol().as_str().as_bytes());
        }
        out
    }
    pub fn serialize(&self) -> String {
        String::from_utf8(self.to_bytes()).expect("lines are valid UTF-8 by construction")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rt(b: &[u8]) {
        assert_eq!(parse(b).unwrap().to_bytes(), b);
    }

    #[test]
    fn round_trips() {
        for b in [
            &b"a\nb\n"[..],
            b"a\r\nb\r\n",
            b"a\r\nb\nc",
            b"\xEF\xBB\xBFShow\n",
            b"",
            b"\xEF\xBB\xBF",
            b"a\rb\n",
            b"\tRarity  Rare   \n",
        ] {
            rt(b);
        }
    }

    #[test]
    fn bom_flag() {
        let d = parse(b"\xEF\xBB\xBF").unwrap();
        assert!(d.has_bom());
        assert!(d.lines.is_empty());
    }

    #[test]
    fn invalid_utf8_is_error() {
        assert!(matches!(
            parse(b"\xFF\xFE"),
            Err(LoadError::InvalidUtf8 { valid_up_to: 0 })
        ));
    }
}

#[cfg(test)]
impl Document {
    pub(crate) fn line_at(&self, i: usize) -> &Line {
        &self.lines[i]
    }
    pub(crate) fn line_text(&self, i: usize) -> &str {
        self.lines[i].text()
    }
    pub(crate) fn line_count(&self) -> usize {
        self.lines.len()
    }
}
