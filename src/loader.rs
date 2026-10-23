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

    /// Neither the live file nor its committed `-default.json` exists (DEF-03). Keeps the
    /// ``Cannot read`` shape of [`LoadError::Read`] and names both files.
    #[error("Cannot read `{path}` or `{default}`: neither file exists")]
    #[diagnostic(
        code(ferrisfilter::io::read_error),
        help("Restore {default}, or create {path}.")
    )]
    NoSource { path: String, default: String },

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
    load_json_str(&content, path)
}

/// [`load_json`] over text already in memory. `name` labels the source in errors (a full path for
/// a file, `styles-default.json (built in)` for an embedded copy).
///
/// # Errors
///
/// `LoadError::Parse` if the JSON is malformed or a field fails type validation.
pub fn load_json_str<T: serde::de::DeserializeOwned>(
    content: &str,
    name: &str,
) -> Result<T, LoadError> {
    let de = &mut serde_json::Deserializer::from_str(content);
    serde_path_to_error::deserialize(de).map_err(|e| {
        let inner = e.inner();
        let offset = SourceOffset::from_location(content, inner.line(), inner.column());
        JsonLoadError {
            file: name.to_string(),
            field_path: e.path().to_string(),
            cause: inner.to_string(),
            src: miette::NamedSource::new(name, content.to_string()),
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
/// - Existence check fails (permission denied, a sharing violation) → `LoadError::Read` on
///   that path, never "absent": an unreadable live file must not silently fall back to the
///   built-in prices.
///
/// Uses `std::path::Path::try_exists()` to distinguish absence from malformation, rather than
/// inspecting the error kind from a failed `fs::read_to_string`, because the underlying
/// `io::Error` is wrapped and downcasting is fragile.
pub fn load_prices_optional(path: &str) -> Result<Option<PricesFile>, LoadError> {
    match std::path::Path::new(path).try_exists() {
        Ok(false) => Ok(None),
        Ok(true) => load_json(path).map(Some),
        Err(source) => Err(LoadError::Read {
            path: path.to_string(),
            source,
        }),
    }
}

/// Parse a `prices.json` held in memory (the embedded fallback has no "absent" case).
///
/// # Errors
///
/// `LoadError::Parse` as [`load_json_str`].
pub fn load_prices_str(content: &str, name: &str) -> Result<PricesFile, LoadError> {
    load_json_str(content, name)
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

/// [`load_tiers`] over text already in memory, with the same validation.
///
/// # Errors
///
/// `LoadError::Parse` / `LoadError::Tiers` as [`load_tiers`].
pub fn load_tiers_str(content: &str, name: &str) -> Result<ValidatedTiers, LoadError> {
    let entries: Vec<TierEntry> = load_json_str(content, name)?;
    Ok(ValidatedTiers::try_from(entries)?)
}

/// Load a policy config (`config.json`) and attach the styles from
/// `styles_path` (loaded and validated via [`load_styles`]).
///
/// A leftover pre-Phase-17 inline `styles` section (OD-1) is handled by content:
/// - every inline entry equals the same-named entry in the styles file (the common case: a
///   store seeded from the old template and never restyled) — the section is dropped, since
///   nothing in it would be lost. The dropped section is simply not carried forward on the next
///   save of that config.
/// - any entry is missing from, differs from, or does not parse as a style for the styles file —
///   a load error naming those styles, because silently ignoring them would hide an edit that no
///   longer does anything. The one exception is a `null` entry for a name the styles file does
///   not define (pre-Phase-17 saves wrote `"craftable": null`): it carries nothing to lose.
///
/// # Errors
///
/// `LoadError::Read`/`Parse` for either file, `LoadError::Styles` for an invalid styles file or a
/// differing inline section.
pub fn load_config(path: &str, styles_path: &str) -> Result<Config, LoadError> {
    load_config_with_live(path, styles_path, styles_path)
}

/// [`load_config`] reading the styles from `styles_path` (possibly the committed
/// `-default.json`), while a differing inline section's advice names `live_styles_path`: the file
/// the user should move their styles into. The default file is never the place for local edits.
///
/// # Errors
///
/// As [`load_config`].
pub fn load_config_with_live(
    path: &str,
    styles_path: &str,
    live_styles_path: &str,
) -> Result<Config, LoadError> {
    let config: Config = load_json(path)?;
    let styles = load_styles(styles_path)?;
    attach_styles(config, styles, path, styles_path, live_styles_path)
}

/// [`load_config_with_live`] over config text already in memory; the caller loads `styles` from
/// its own source first. `name` / `styles_name` label the two sources in errors.
///
/// # Errors
///
/// `LoadError::Parse` for the config text, `LoadError::Styles` for a differing inline section.
pub fn load_config_str_with_live(
    content: &str,
    name: &str,
    styles: StyleSet,
    styles_name: &str,
    live_styles_path: &str,
) -> Result<Config, LoadError> {
    let config: Config = load_json_str(content, name)?;
    attach_styles(config, styles, name, styles_name, live_styles_path)
}

/// Attach loaded `styles` to a parsed `config`, applying the inline-`styles` rule of
/// [`load_config`].
///
/// # Errors
///
/// `LoadError::Styles` when an inline section differs from `styles`.
pub fn attach_styles(
    mut config: Config,
    styles: StyleSet,
    name: &str,
    styles_name: &str,
    live_styles_path: &str,
) -> Result<Config, LoadError> {
    if let Some(legacy) = config.legacy_styles.take() {
        check_legacy_styles(&legacy, &styles, name, styles_name, live_styles_path)?;
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
    live_styles_path: &str,
) -> Result<(), StyleError> {
    let reject = |differing| StyleError::InlineStylesSection {
        path: path.to_string(),
        styles_path: styles_path.to_string(),
        live_styles_path: live_styles_path.to_string(),
        differing,
    };
    let Some(entries) = legacy.as_object() else {
        return Err(reject(Vec::new()));
    };
    let differing: Vec<String> = entries
        .iter()
        .filter(|(name, value)| {
            if value.is_null() {
                // Old saves wrote `"craftable": null` for an unset optional style: nothing to
                // lose, unless the styles file defines the name (then the inline copy disagrees).
                return styles.get(name).is_some();
            }
            match serde_json::from_value::<Style>((*value).clone()) {
                Ok(mut inline) => {
                    // The id is identity metadata the pre-id inline copy never had.
                    inline.id = styles.id_of(name).map(str::to_string);
                    Some(&inline) != styles.get(name)
                }
                // Unparseable user content is never silently discarded.
                Err(_) => true,
            }
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
    let content = std::fs::read_to_string(path).map_err(|e| LoadError::Read {
        path: path.to_string(),
        source: e,
    })?;
    load_styles_str(&content, path)
}

/// [`load_styles`] over text already in memory, with the same id assignment and validation.
///
/// # Errors
///
/// `LoadError::Parse` / `LoadError::Styles` as [`load_styles`].
pub fn load_styles_str(content: &str, name: &str) -> Result<StyleSet, LoadError> {
    let mut set: StyleSet = load_json_str(content, name)?;
    set.assign_missing_ids();
    set.validate()?;
    Ok(set)
}

/// Render a `StyleSet` as `styles.json`: one compact line per style, in file order, so a save of
/// the tracked file diffs per style instead of exploding into pretty-printed noise. Pure; the
/// caller writes the bytes.
pub fn render_styles(set: &StyleSet) -> String {
    let last = set.len().saturating_sub(1);
    let mut out = String::from(
        "{
",
    );
    for (i, (name, style)) in set.iter().enumerate() {
        out.push_str("  ");
        out.push_str(&serde_json::to_string(name).expect("string serialization is infallible"));
        out.push_str(": ");
        out.push_str(
            &serde_json::to_string(style)
                .expect("Style serialization is infallible (no non-string map keys)"),
        );
        if i != last {
            out.push(',');
        }
        out.push('\n');
    }
    out.push_str(
        "}
",
    );
    out
}

/// Render `tiers.json` entries in the given order. Never sorts: `ValidatedTiers` checks ascending
/// order within a unit group in file order, so display order (S..F) would fail validation.
pub fn render_tiers(entries: &[TierEntry]) -> String {
    let last = entries.len().saturating_sub(1);
    let mut out = String::from(
        "[
",
    );
    for (i, e) in entries.iter().enumerate() {
        out.push_str("  ");
        out.push_str(&serde_json::to_string(e).expect("TierEntry serialization is infallible"));
        if i != last {
            out.push(',');
        }
        out.push('\n');
    }
    out.push_str(
        "]
",
    );
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    const TIERS: &str = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/tiers-default.json"));

    fn repo(file: &str) -> String {
        format!("{}/{file}", env!("CARGO_MANIFEST_DIR"))
    }

    fn text(file: &str) -> String {
        std::fs::read_to_string(repo(file)).unwrap()
    }
    #[test]
    fn style_colour_bad_length_names_style_path() {
        let src = r#"{"S":{"text":[1,2],"border":null,"background":null,"font":null,"minimap":null,"beam":null,"sound":null,"action":"Show","disableDropSound":false}}"#;
        let e = load_styles_str(src, "x").unwrap_err().to_string();
        assert!(e.contains("S.text") && e.contains("3 or 4 values"), "{e}");
    }


    #[test]
    fn styles_str_equals_file_loader() {
        let from_text = load_styles_str(&text("tests/fixtures/seed-styles.json"), "x").unwrap();
        let from_file = load_styles(&repo("tests/fixtures/seed-styles.json")).unwrap();
        assert_eq!(format!("{from_text:?}"), format!("{from_file:?}"));
    }

    #[test]
    fn tiers_str_equals_file_loader() {
        for f in ["tiers-default.json", "tiers-fallback.json"] {
            let from_text = load_tiers_str(&text(f), "x").unwrap();
            let from_file = load_tiers(&repo(f)).unwrap();
            assert_eq!(format!("{from_text:?}"), format!("{from_file:?}"), "{f}");
        }
    }

    #[test]
    fn prices_str_equals_file_loader() {
        let from_text = load_prices_str(&text("prices-fallback.json"), "x").unwrap();
        let from_file: PricesFile = load_json(&repo("prices-fallback.json")).unwrap();
        assert_eq!(format!("{from_text:?}"), format!("{from_file:?}"));
    }

    #[test]
    fn styles_str_parse_error_names_the_label() {
        let label = "styles-default.json (built in)";
        let err = load_styles_str("{", label).unwrap_err();
        match &err {
            LoadError::Parse(e) => assert_eq!(e.file, label),
            other => panic!("expected Parse, got {other:?}"),
        }
        assert!(err.to_string().contains(label));
    }

    #[test]
    fn config_str_equals_file_loader() {
        let styles_path = repo("tests/fixtures/seed-styles.json");
        let styles = load_styles(&styles_path).unwrap();
        let from_text = load_config_str_with_live(
            &text("config.json"),
            "config.json (built in)",
            styles,
            &styles_path,
            &styles_path,
        )
        .unwrap();
        let from_file =
            load_config_with_live(&repo("config.json"), &styles_path, &styles_path).unwrap();
        // Compared as JSON values: Debug output of the config's HashMaps is unordered.
        assert_eq!(
            serde_json::to_value(&from_text).unwrap(),
            serde_json::to_value(&from_file).unwrap()
        );
    }

    #[test]
    fn render_styles_shape_and_round_trip() {
        let set = StyleSet::committed_for_tests();
        let text = render_styles(&set);
        assert!(
            text.starts_with(
                "{
"
            ) && text.ends_with(
                "}
"
            )
        );
        assert_eq!(text.lines().count(), set.len() + 2);
        let inner: Vec<&str> = text.lines().skip(1).take(set.len()).collect();
        for (i, line) in inner.iter().enumerate() {
            assert!(line.starts_with("  \""), "{line}");
            assert_eq!(line.ends_with(','), i + 1 != inner.len(), "{line}");
        }
        let back: StyleSet = serde_json::from_str(&text).unwrap();
        assert_eq!(back, set);
        back.validate().unwrap();
        assert_eq!(render_styles(&back), text);
        // The new optional fields are written only when set: existing styles stay byte-identical.
        assert!(!text.contains("positional") && !text.contains("enableDropSound"));
        let s_line = text.lines().find(|l| l.starts_with("  \"S\":")).unwrap();
        assert!(
            s_line.contains(r#""sound":{"custom":"uwu.mp3","vol":300}"#),
            "{s_line}"
        );
        let u_line = text
            .lines()
            .find(|l| l.starts_with("  \"unchanged\":"))
            .unwrap();
        assert!(u_line.contains(r#""text":null"#), "{u_line}");
    }

    #[test]
    fn render_styles_escapes_names() {
        let mut set = StyleSet::committed_for_tests();
        let name = "we\"ird\\name";
        set.insert_new(name, Style::blank_show()).unwrap();
        let back: StyleSet = serde_json::from_str(&render_styles(&set)).unwrap();
        assert!(back.contains(name));
    }

    #[test]
    fn render_tiers_preserves_file_order_and_validates() {
        let entries: Vec<TierEntry> = serde_json::from_str(TIERS).unwrap();
        let text = render_tiers(&entries);
        let back: Vec<TierEntry> = serde_json::from_str(&text).unwrap();
        assert_eq!(back.len(), entries.len());
        for (a, b) in entries.iter().zip(&back) {
            assert_eq!((&a.tier, a.min, &a.unit), (&b.tier, b.min, &b.unit));
        }
        assert_eq!(back[0].tier, "F");
        ValidatedTiers::try_from(back).unwrap();
    }

    const STYLES: &str = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/seed-styles.json"
    ));

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

    /// WR-04: `"styles": null` is rejected like any other inline section that is not a
    /// repeat of styles.json.
    #[test]
    fn load_config_rejects_a_null_inline_styles_section() {
        let dir = tempfile::tempdir().unwrap();
        let styles_p = dir.path().join("styles.json");
        std::fs::write(&styles_p, STYLES).unwrap();
        let mut v: serde_json::Value = serde_json::from_str(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/config.json"
        )))
        .unwrap();
        v["styles"] = serde_json::Value::Null;
        let cfg_p = dir.path().join("config.json");
        std::fs::write(&cfg_p, v.to_string()).unwrap();
        let err = load_config(cfg_p.to_str().unwrap(), styles_p.to_str().unwrap()).unwrap_err();
        assert!(
            matches!(
                err,
                LoadError::Styles(StyleError::InlineStylesSection { .. })
            ),
            "{err}"
        );
    }

    /// Load the committed `config.json` with `styles` replaced by `inline`.
    fn load_with_inline_styles(inline: serde_json::Value) -> Result<Config, LoadError> {
        let dir = tempfile::tempdir().unwrap();
        let styles_p = dir.path().join("styles.json");
        std::fs::write(&styles_p, STYLES).unwrap();
        let mut v: serde_json::Value = serde_json::from_str(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/config.json"
        )))
        .unwrap();
        v["styles"] = inline;
        let cfg_p = dir.path().join("config.json");
        std::fs::write(&cfg_p, v.to_string()).unwrap();
        load_config(cfg_p.to_str().unwrap(), styles_p.to_str().unwrap())
    }

    fn rejected_names(err: LoadError) -> Vec<String> {
        match err {
            LoadError::Styles(StyleError::InlineStylesSection { differing, .. }) => differing,
            other => panic!("expected InlineStylesSection, got {other}"),
        }
    }

    /// Iteration-2 WR-01: an inline entry that does not parse as a style is rejected by name,
    /// not treated as equal to an absent style and dropped.
    #[test]
    fn load_config_rejects_an_unparseable_inline_style() {
        let err = load_with_inline_styles(serde_json::json!({ "Bogus": 7 })).unwrap_err();
        assert_eq!(rejected_names(err), vec!["Bogus".to_string()]);
    }

    #[test]
    fn load_config_rejects_an_inline_style_with_a_misspelled_field() {
        let err = load_with_inline_styles(serde_json::json!({
            "craftable": { "fnt": 30, "action": "Show" }
        }))
        .unwrap_err();
        assert_eq!(rejected_names(err), vec!["craftable".to_string()]);
    }

    /// Pre-Phase-17 saves wrote `"craftable": null`; that carries nothing and still loads.
    #[test]
    fn load_config_accepts_a_null_inline_entry_for_an_undefined_style() {
        let cfg = load_with_inline_styles(serde_json::json!({ "craftable": null })).unwrap();
        assert_eq!(cfg.styles.len(), 12);
    }

    /// A `null` entry for a style the styles file defines disagrees with it.
    #[test]
    fn load_config_rejects_a_null_inline_entry_for_a_defined_style() {
        let err = load_with_inline_styles(serde_json::json!({ "S": null })).unwrap_err();
        assert_eq!(rejected_names(err), vec!["S".to_string()]);
    }

    #[test]
    fn load_prices_optional_absent_is_none_and_unstatable_is_read_error() {
        let dir = tempfile::tempdir().unwrap();
        let absent = dir.path().join("prices.json");
        assert!(
            load_prices_optional(absent.to_str().unwrap())
                .unwrap()
                .is_none()
        );
        // A path the OS cannot stat is an error naming it, not "absent".
        let err = load_prices_optional("bad\0name/prices.json").unwrap_err();
        assert!(matches!(err, LoadError::Read { .. }), "{err}");
        assert!(err.to_string().starts_with("Cannot read `"), "{err}");
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

    /// WR-03: a bad field inside a cancelable minimap object is still reported with the style's
    /// name, the field and the file.
    #[test]
    fn load_styles_reports_an_unknown_field_inside_a_minimap() {
        let bad = STYLES.replacen(
            r#""shape": "Circle" }"#,
            r#""shape": "Circle", "zzbad": 1 }"#,
            1,
        );
        assert_ne!(bad, STYLES, "splice must apply");
        let err = load_text(&bad).unwrap_err();
        assert!(matches!(err, LoadError::Parse(_)));
        let msg = err.to_string();
        assert!(msg.contains("S.minimap"), "{msg}");
        assert!(msg.contains("unknown field `zzbad`"), "{msg}");
        assert!(msg.contains("styles.json"), "{msg}");
    }

    #[test]
    fn load_styles_missing_required_style_is_styles_error() {
        let mut v: serde_json::Value = serde_json::from_str(STYLES).unwrap();
        v.as_object_mut().unwrap().remove("E");
        let err = load_text(&v.to_string()).unwrap_err();
        assert!(matches!(
            err,
            LoadError::Styles(StyleError::MissingRequired(_))
        ));
        assert!(err.to_string().contains('E'));
    }

    /// `include_str!` returns the file's on-disk bytes, which are CRLF on a `core.autocrlf=true`
    /// checkout, so the splice keys on the first `{` only and the test runs against both endings.
    #[test]
    fn load_styles_rejects_duplicate_name() {
        let lf = STYLES.replace("\r\n", "\n");
        for text in [lf.clone(), lf.replace('\n', "\r\n")] {
            let dup = text.replacen(
                '{',
                "{ \"S\": { \"text\": null, \"border\": null, \"background\": null, \"font\": null, \"minimap\": null, \"beam\": null, \"sound\": null, \"action\": \"Show\", \"disableDropSound\": false },",
                1,
            );
            assert_ne!(dup, text, "splice must apply");
            let err = load_text(&dup).unwrap_err();
            assert!(err.to_string().contains("duplicate style name"), "{err}");
        }
    }

    #[test]
    fn render_styles_writes_id_first() {
        let set = StyleSet::committed_for_tests();
        let text = render_styles(&set);
        let s_line = text.lines().find(|l| l.starts_with("  \"S\":")).unwrap();
        assert!(s_line.contains(r#": {"id":"builtin:S","#), "{s_line}");
        for line in text.lines().skip(1).take(set.len()) {
            assert!(line.contains(r#": {"id":""#), "{line}");
        }
    }

    #[test]
    fn load_styles_assigns_ids_deterministically() {
        let dir = std::env::temp_dir().join("ff-26-01-load-ids");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("styles.json");
        let mut set = StyleSet::committed_for_tests();
        let names: Vec<String> = set.names().map(str::to_string).collect();
        for n in &names {
            set.get_mut(n).unwrap().id = None;
        }
        std::fs::write(&path, render_styles(&set)).unwrap();
        let a = load_styles(path.to_str().unwrap()).unwrap();
        let b = load_styles(path.to_str().unwrap()).unwrap();
        assert_eq!(a, b);
        assert_eq!(a.id_of("S"), Some("builtin:S"));
        assert!(a.id_of("kite").is_some());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
