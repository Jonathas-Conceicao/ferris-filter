//! Read-only block model laid over the raw lines of a [`Document`](super::Document).
//!
//! Nothing here owns text. Every block, trivia run and body line is a line index or range into
//! `Document.lines`, so serialization stays the raw lines (PARSE-03). Block identity is a stable
//! counter-assigned [`BlockId`], never an index.

use std::ops::Range;

use super::diag::FilterDiagnostic;
use super::keywords::Keyword;
use super::tokenize::TokenizedLine;

/// Stable block identity, assigned from a document counter (mirrors `overlay::edit::RuleId`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct BlockId(pub(crate) u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlockKeyword {
    Show,
    Hide,
    Minimal,
}

/// Classification of a commented-out block.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DisabledState {
    /// Every inner line parses cleanly; the block can be enabled.
    Valid,
    /// Kept byte-for-byte; each reason is tied to a 1-based line.
    Invalid(Vec<FilterDiagnostic>),
}

/// One body line of a block.
///
/// For a disabled block the `tokens` ranges are relative to the text after the line's first `#`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BodyLine {
    /// 0-based index into the document's lines.
    pub line: usize,
    /// `None` = keyword not in the table (or unparsable): the line is opaque.
    pub keyword: Option<&'static Keyword>,
    pub tokens: Option<TokenizedLine>,
}

#[derive(Debug, Clone)]
pub struct Block {
    pub(crate) id: BlockId,
    pub(crate) keyword: BlockKeyword,
    pub(crate) disabled: Option<DisabledState>,
    pub(crate) lead_start: usize,
    pub(crate) header: usize,
    pub(crate) end: usize,
    pub(crate) body: Vec<BodyLine>,
}

impl Block {
    pub fn id(&self) -> BlockId {
        self.id
    }
    pub fn keyword(&self) -> BlockKeyword {
        self.keyword
    }
    pub fn is_enabled(&self) -> bool {
        self.disabled.is_none()
    }
    pub fn disabled_state(&self) -> Option<&DisabledState> {
        self.disabled.as_ref()
    }
    /// True only for a disabled block whose inner lines parse cleanly.
    pub fn can_enable(&self) -> bool {
        matches!(self.disabled, Some(DisabledState::Valid))
    }
    /// Attached comment lines directly above the header (includes `#ff:` lines).
    pub fn leading(&self) -> Range<usize> {
        self.lead_start..self.header
    }
    pub fn header_line(&self) -> usize {
        self.header
    }
    /// Header through the last body line (exclusive end).
    pub fn owned(&self) -> Range<usize> {
        self.header..self.end
    }
    /// Leading comments plus owned lines.
    pub fn span(&self) -> Range<usize> {
        self.lead_start..self.end
    }
    pub fn body(&self) -> &[BodyLine] {
        &self.body
    }
    pub fn has_continue(&self) -> bool {
        self.body.iter().any(|b| {
            b.keyword
                .is_some_and(|k| k.name.eq_ignore_ascii_case("continue"))
        })
    }
}

/// Top-level partition of the document: every line index belongs to exactly one item, in order.
#[derive(Debug, Clone)]
pub enum Item {
    /// Standalone comment/blank lines.
    Trivia(Range<usize>),
    Block(Block),
    /// An `Import` directive. `path` is a byte range into the line text (quotes excluded); the
    /// file is never opened.
    Import {
        line: usize,
        path: Range<usize>,
        optional: bool,
    },
    /// A line outside any block that is not usable grammar, kept as-is.
    Opaque(usize),
}
