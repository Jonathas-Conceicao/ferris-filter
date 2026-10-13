use crate::error::JsonLoadError;
use crate::models::config::{Config, Style, StyleError, StyleSet};
use crate::models::prices::PricesFile;
use crate::models::tiers::{TierEntry, TierValidationError, ValidatedTiers};
use miette::{Diagnostic, SourceOffset};
use thiserror::Error;

/// Everything that can go wrong while reading and validating an input file.
///
/// This is a typed error rather than a `miette::Report` so that callers other than the CLI —
/// notably the overlay front end — can `match` on the failure and decide how to present it
/// (CORE-04). The CLI converts it to a `miette::Error` at its own boundary.
///
/// Every variant that wraps an existing diagnostic does so with `#[error(transparent)]` +
/// `#[diagnostic(transparent)]`, so miette's rendering — including `JsonLoadError`'s source
/// span and label — is preserved byte-for-byte. `tests/cli_errors.rs` asserts on that rendered
/// output and must keep passing unmodified.
#[derive(Debug, Error, Diagnostic)]
pub enum LoadError {
    /// The file could not be read at all (missing, permissions, etc.).
    ///
    /// The message deliberately starts with ``Cannot read`` — `tests/cli_errors.rs`
    /// (`missing_config_json_exits_nonzero_with_message`) asserts on that exact prefix.
    #[error("Cannot read `{path}`: {source}")]
    #[diagnostic(code(ferrisfilter::io::read_error))]
    Read {
        path: String,
        #[source]
        source: std::io::Error,
    },

    /// The file was read but is malformed or fails type validation. Carries the dotted field
    /// path and the highlighted source span.
    #[error(transparent)]
    #[diagnostic(transparent)]
    Parse(#[from] JsonLoadError),

    /// `tiers.json` parsed but failed completeness/ordering validation.
    #[error(transparent)]
    #[diagnostic(transparent)]
    Tiers(#[from] TierValidationError),

    /// `styles.json` parsed but failed validation (missing required style, bad custom sound).
    #[error(transparent)]
    #[diagnostic(transparent)]
    Styles(#[from] StyleError),
}

/// Generic JSON loader that wraps `serde_path_to_error` for field-path-aware errors.
///
/// On success returns the deserialized value. On failure returns a `LoadError`:
/// `Read` if the file could not be read, `Parse` (carrying a `JsonLoadError` diagnostic with
/// the dotted field path, byte offset and raw source text) if it could not be deserialized.
///
/// # Errors
///
/// - If the file cannot be read, returns `LoadError::Read`.
/// - If the JSON is malformed or a field fails type validation, returns `LoadError::Parse`
///   with the source span highlighted.
pub fn load_json<T: serde::de::DeserializeOwned>(path: &str) -> Result<T, LoadError> {
    let content = std::fs::read_to_string(path).map_err(|e| LoadError::Read {
        path: path.to_string(),
        source: e,
    })?;
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

/// Load `prices.json` if present, returning `Ok(None)` if the file does not exist.
///
/// **Absence vs. malformation:**
/// - File absent → returns `Ok(None)`. This enables GEN-05: `generate` can warn and skip
///   the currency/category blocks rather than aborting with an I/O error.
/// - File present but malformed → returns `Err(...)` from `load_json` (a `LoadError::Parse`
///   with the dotted field path). The caller must abort; a corrupted prices.json is not
///   a recoverable situation.
///
/// Uses `std::path::Path::exists()` to distinguish absence from malformation, rather than
/// inspecting the error kind from a failed `fs::read_to_string`, because the underlying
/// `io::Error` is wrapped and downcasting is fragile.
pub fn load_prices_optional(path: &str) -> Result<Option<PricesFile>, LoadError> {
    if !std::path::Path::new(path).exists() {
        return Ok(None);
    }
    load_json(path).map(Some)
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
/// - If the file cannot be read or parsed, returns `LoadError::Read` / `LoadError::Parse`.
/// - If tier validation fails, returns `LoadError::Tiers`.
pub fn load_tiers(path: &str) -> Result<ValidatedTiers, LoadError> {
    let entries: Vec<TierEntry> = load_json(path)?;
    Ok(ValidatedTiers::try_from(entries)?)
}

/// Load a policy config (`config.json` / `dynamic.json`) and attach the styles from
/// `styles_path` (loaded and validated via [`load_styles`]).
///
/// A leftover pre-Phase-17 inline `styles` section (OD-1) is handled by content:
/// - every inline entry equals the same-named entry in the styles file (the common case: a
///   store seeded from the old template and never restyled) — the section is dropped, since
///   nothing in it would be lost. `overlay::store::save` never writes `styles`, so the next
///   Apply rewrites the store without it.
/// - any entry is missing from, or differs from, the styles file — a load error naming those
///   styles, because silently ignoring them would hide an edit that no longer does anything.
///
/// # Errors
///
/// `LoadError::Read`/`Parse` for either file, `LoadError::Styles` for an invalid styles file or a
/// differing inline section.
pub fn load_config(path: &str, styles_path: &str) -> Result<Config, LoadError> {
    let mut config: Config = load_json(path)?;
    let styles = load_styles(styles_path)?;
    if let Some(legacy) = config.legacy_styles.take() {
        check_legacy_styles(&legacy, &styles, path, styles_path)?;
    }
    config.styles = styles;
    Ok(config)
}

/// Accept an inline `styles` section only if every entry equals the loaded styles file's.
fn check_legacy_styles(
    legacy: &serde_json::Value,
    styles: &StyleSet,
    path: &str,
    styles_path: &str,
) -> Result<(), StyleError> {
    let reject = |differing| StyleError::InlineStylesSection {
        path: path.to_string(),
        styles_path: styles_path.to_string(),
        differing,
    };
    let Some(entries) = legacy.as_object() else {
        return Err(reject(Vec::new()));
    };
    let differing: Vec<String> = entries
        .iter()
        .filter(|(name, value)| {
            let inline = serde_json::from_value::<Style>((*value).clone()).ok();
            inline.as_ref() != styles.get(name)
        })
        .map(|(name, _)| name.clone())
        .collect();
    if differing.is_empty() {
        Ok(())
    } else {
        Err(reject(differing))
    }
}

/// Load and validate `styles.json`.
///
/// The file's top level is the name -> style map, so a parse error's dotted path starts at the
/// style name (e.g. `S.font`). `validate()` always runs here so a bad file is reported at load,
/// naming the file; `emitter::generate_as` re-checks it as a backstop for library callers.
///
/// # Errors
///
/// - Unreadable file or bad JSON/duplicate name: `LoadError::Read` / `LoadError::Parse`.
/// - Failed validation: `LoadError::Styles`.
pub fn load_styles(path: &str) -> Result<StyleSet, LoadError> {
    let set: StyleSet = load_json(path)?;
    set.validate()?;
    Ok(set)
}

#[cfg(test)]
mod tests {
    use super::*;

    const STYLES: &str = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/styles.json"));

    fn load_text(text: &str) -> Result<StyleSet, LoadError> {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("styles.json");
        std::fs::write(&p, text).unwrap();
        load_styles(p.to_str().unwrap())
    }

    #[test]
    fn load_styles_reads_committed_file() {
        assert_eq!(load_text(STYLES).unwrap().len(), 12);
    }

    #[test]
    fn load_styles_missing_path_is_read_error() {
        let err = load_styles("definitely/not/here/styles.json").unwrap_err();
        assert!(matches!(err, LoadError::Read { .. }));
        assert!(err.to_string().starts_with("Cannot read `"), "{err}");
    }

    #[test]
    fn load_styles_bad_type_reports_path_from_style_name() {
        let bad = STYLES.replacen("\"font\": 37", "\"font\": \"BROKEN\"", 1);
        let err = load_text(&bad).unwrap_err();
        assert!(matches!(err, LoadError::Parse(_)));
        let msg = err.to_string();
        assert!(msg.contains("S.font"), "{msg}");
        assert!(msg.contains("expected u8"), "{msg}");
    }

    #[test]
    fn load_styles_missing_required_style_is_styles_error() {
        let mut v: serde_json::Value = serde_json::from_str(STYLES).unwrap();
        v.as_object_mut().unwrap().remove("E");
        let err = load_text(&v.to_string()).unwrap_err();
        assert!(matches!(err, LoadError::Styles(StyleError::MissingRequired(_))));
        assert!(err.to_string().contains('E'));
    }

    #[test]
    fn load_styles_rejects_duplicate_name() {
        let dup = STYLES.replacen("{
", "{
  \"S\": { \"text\": null, \"border\": null, \"background\": null, \"font\": null, \"minimap\": null, \"beam\": null, \"sound\": null, \"action\": \"Show\", \"disableDropSound\": false },
", 1);
        let err = load_text(&dup).unwrap_err();
        assert!(err.to_string().contains("duplicate style name"), "{err}");
    }
}
