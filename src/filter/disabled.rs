//! Disabled (commented-out) block detection.

use super::block::{BlockKeyword, BodyLine, DisabledState};
use super::lines::Line;

pub(crate) struct Disabled {
    pub keyword: BlockKeyword,
    pub body: Vec<BodyLine>,
    /// Exclusive end: one past the last keyword line (or the header when there is no body).
    pub end: usize,
    pub state: DisabledState,
}

pub(crate) fn is_header(_text: &str) -> bool {
    false
}

pub(crate) fn detect(_ls: &[Line], _h: usize) -> Option<Disabled> {
    None
}
