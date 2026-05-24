use crate::error::JsonLoadError;
use crate::models::tiers::{TierEntry, ValidatedTiers};
use miette::SourceOffset;

/// Generic JSON loader that wraps `serde_path_to_error` for field-path-aware errors.
///
/// On success returns the deserialized value. On failure returns a `JsonLoadError`
/// diagnostic that includes the dotted field path, the byte offset, and the raw source
/// text — suitable for miette's graphical renderer.
///
/// # Errors
///
/// - If the file cannot be read, returns a plain `miette::miette!` error.
/// - If the JSON is malformed or a field fails type validation, returns `JsonLoadError`
///   with the source span highlighted.
pub fn load_json<T: serde::de::DeserializeOwned>(path: &str) -> miette::Result<T> {
    let content = std::fs::read_to_string(path)
        .map_err(|e| miette::miette!("Cannot read `{path}`: {e}"))?;
    let de = &mut serde_json::Deserializer::from_str(&content);
    serde_path_to_error::deserialize(de).map_err(|e| {
        let inner = e.inner();
        let offset = SourceOffset::from_location(&content, inner.line(), inner.column());
        JsonLoadError {
            file: path.to_string(),
            field_path: e.path().to_string(),
            cause: inner.to_string(),
            src: miette::NamedSource::new(path, content),
            location: offset,
        }
        .into()
    })
}

/// Load and validate `tiers.json`.
///
/// Deserializes the file into `Vec<TierEntry>` via the generic `load_json` helper,
/// then applies `ValidatedTiers::try_from` which enforces:
/// 1. All 7 tiers S–F are present.
/// 2. Within each unit group, `min` values are strictly ascending.
///
/// # Errors
///
/// - If the file cannot be read or parsed, returns a `JsonLoadError` (field-path aware).
/// - If tier validation fails, returns the `TierValidationError` as a `miette::Error`.
pub fn load_tiers(path: &str) -> miette::Result<ValidatedTiers> {
    let entries: Vec<TierEntry> = load_json(path)?;
    ValidatedTiers::try_from(entries).map_err(miette::Error::from)
}
