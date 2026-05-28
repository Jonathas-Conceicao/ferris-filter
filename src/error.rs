use miette::{Diagnostic, NamedSource, SourceOffset};
use thiserror::Error;

#[derive(Error, Debug, Diagnostic)]
#[error("Failed to load `{file}`: {field_path}: {cause}")]
#[diagnostic(code(ferrisfilter::config::parse_error))]
pub struct JsonLoadError {
    pub file: String,
    pub field_path: String,
    pub cause: String,
    #[source_code]
    pub src: NamedSource<String>,
    #[label("error here")]
    pub location: SourceOffset,
}
