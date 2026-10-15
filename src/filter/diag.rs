//! Diagnostic data and the one load failure.

use miette::Diagnostic as MietteDiagnostic;
use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    Error,
    Warning,
    Info,
}

/// A problem found while parsing, tied to a 1-based line. Plain data; the library never prints it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FilterDiagnostic {
    pub line: usize,
    pub severity: Severity,
    pub message: String,
}

impl std::fmt::Display for FilterDiagnostic {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "line {}: {}", self.line, self.message)
    }
}

/// The only way loading a filter can fail.
#[derive(Debug, Error, MietteDiagnostic)]
pub enum LoadError {
    #[error("filter is not valid UTF-8 (first invalid byte at offset {valid_up_to})")]
    #[diagnostic(code(ferrisfilter::filter::invalid_utf8))]
    InvalidUtf8 { valid_up_to: usize },
}
