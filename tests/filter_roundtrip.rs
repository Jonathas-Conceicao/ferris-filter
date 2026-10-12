//! Byte-identity contract for `ferrisfilter::filter` (PARSE-03): `to_bytes(parse(x)) == x`.
//!
//! Fixtures are read with `fs::read` and never normalized (no CRLF->LF), so the test sees exactly
//! the bytes the checkout produced.

use ferrisfilter::filter::{parse, BlockKeyword, LoadError};
use std::fs;
use std::path::Path;

fn manifest() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
}

fn window(b: &[u8], at: usize) -> String {
    let s = at.min(b.len());
    let end = (at + 40).min(b.len());
    b[s..end].escape_ascii().to_string()
}

fn assert_bytes_eq(label: &str, got: &[u8], want: &[u8]) {
    if got == want {
        return;
    }
    let off = got
        .iter()
        .zip(want.iter())
        .position(|(a, b)| a != b)
        .unwrap_or(got.len().min(want.len()));
    panic!(
        "{label}: bytes differ (got {} bytes, want {} bytes), first difference at offset {off}\n  got:  {}\n  want: {}",
        got.len(),
        want.len(),
        window(got, off),
        window(want, off)
    );
}

fn check(label: &str, bytes: &[u8]) {
    let doc = parse(bytes).unwrap_or_else(|e| panic!("{label}: {e}"));
    assert_bytes_eq(label, &doc.to_bytes(), bytes);
}

#[test]
fn roundtrip_filterblade_fixture() {
    let bytes = fs::read(manifest().join("tests/fixtures/parser/filterblade.filter")).unwrap();
    assert_eq!(bytes.len(), 258094);
    check("filterblade", &bytes);
}

#[test]
fn roundtrip_seed_universal() {
    let bytes = fs::read(manifest().join("tests/fixtures/seed-universal.filter")).unwrap();
    check("seed-universal", &bytes);
}

#[test]
fn roundtrip_synthetic_cases() {
    let cases: &[(&str, &[u8])] = &[
        ("lf", b"Show\n\tClass \"Bows\"\n"),
        ("crlf", b"Show\r\n\tClass \"Bows\"\r\n"),
        ("mixed", b"Show\r\n\tRarity Rare\nHide\r\n"),
        ("bom+crlf", b"\xEF\xBB\xBFShow\r\n\tRarity Rare\r\n"),
        ("no final newline", b"Show\n\tRarity Rare"),
        ("only bom", b"\xEF\xBB\xBF"),
        ("empty", b""),
        ("tabs+trailing ws", b"Show  \n\t\tRarity \t Rare \t \n"),
        ("hash in quotes", b"Show\n\tBaseType == \"a # b\" # c\n"),
        ("operator no space", b"Show\n\tHasExplicitMod >=1 \"x\"\n"),
        ("lone cr", b"Show\n\tRar\rity Rare\n"),
        ("blank lines only", b"\n\r\n\n\n"),
    ];
    for (label, bytes) in cases {
        check(label, bytes);
    }
}

#[test]
fn roundtrip_invalid_utf8_is_load_error() {
    assert!(matches!(
        parse(b"Show\n\xFF\xFE"),
        Err(LoadError::InvalidUtf8 { valid_up_to: 5 })
    ));
}

// ---- Corpus gate (PARSE-01): zero Error diagnostics + byte identity on real filters ----

use ferrisfilter::filter::{Item, Severity};

fn check_clean(label: &str, bytes: &[u8]) -> ferrisfilter::filter::Document {
    let doc = parse(bytes).unwrap_or_else(|e| panic!("{label}: {e}"));
    assert_bytes_eq(label, &doc.to_bytes(), bytes);
    let errors: Vec<_> = doc
        .diagnostics()
        .iter()
        .filter(|d| d.severity == Severity::Error)
        .collect();
    if !errors.is_empty() {
        let first: Vec<String> = errors.iter().take(20).map(|d| d.to_string()).collect();
        panic!(
            "{label}: {} Error diagnostics, first {}:\n{}",
            errors.len(),
            first.len(),
            first.join("\n")
        );
    }
    doc
}

#[test]
fn corpus_filterblade() {
    let bytes = fs::read(manifest().join("tests/fixtures/parser/filterblade.filter")).unwrap();
    let doc = check_clean("filterblade", &bytes);
    let enabled = |k: BlockKeyword| {
        doc.blocks()
            .filter(|b| b.is_enabled() && b.keyword() == k)
            .count()
    };
    assert_eq!(enabled(BlockKeyword::Show), 447);
    assert_eq!(enabled(BlockKeyword::Hide), 13);
    assert_eq!(enabled(BlockKeyword::Minimal), 0);
    let continue_lines = doc
        .blocks()
        .filter(|b| b.is_enabled())
        .flat_map(|b| b.body().iter())
        .filter(|l| l.keyword.is_some_and(|k| k.name.eq_ignore_ascii_case("continue")))
        .count();
    assert_eq!(continue_lines, 40);
    assert!(!doc.items().iter().any(|i| matches!(i, Item::Import { .. })));

    // Expected disabled-block count, computed independently from the raw text.
    let text = std::str::from_utf8(&bytes).unwrap();
    let expected = text
        .split('\n')
        .map(|l| l.trim_end_matches('\r'))
        .filter(|l| {
            ["#Show", "#Hide", "#Minimal"].iter().any(|p| {
                l.strip_prefix(p).is_some_and(|rest| {
                    let r = rest.trim_start_matches([' ', '\t']);
                    r.is_empty() || r.starts_with('#')
                })
            })
        })
        .count();
    assert!(expected > 0);
    let disabled = doc.blocks().filter(|b| !b.is_enabled()).count();
    assert_eq!(disabled, expected);

    for b in doc.blocks() {
        assert!(doc.meta_get(b.id(), "name").is_none(), "D-07: foreign comments are not a name");
    }
}

#[test]
fn corpus_seed_universal() {
    let bytes = fs::read(manifest().join("tests/fixtures/seed-universal.filter")).unwrap();
    let doc = check_clean("seed-universal", &bytes);
    assert!(doc.blocks().count() > 0);
}

#[test]
fn corpus_generated_dynamic() {
    use ferrisfilter::{compile, dynamic};
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    for f in ["dynamic_base.json", "tiers.json", "config.json"] {
        fs::copy(manifest().join(f), dir.join(f)).unwrap();
    }
    fs::copy(
        manifest().join("tests/fixtures/seed-prices.json"),
        dir.join("prices.json"),
    )
    .unwrap();
    let store = dir.join(dynamic::STORE_FILE);
    dynamic::ensure_store(&dir.join(dynamic::BASE_FILE), &store).unwrap();
    let compiled = dynamic::compile_store(&compile::InputPaths::relative_to(dir), &store).unwrap();
    let doc = check_clean("generated dynamic", compiled.text.as_bytes());
    assert!(doc.blocks().count() > 0);
}

#[test]
fn corpus_reference_old_filters() {
    let dir = manifest().join("reference-old-filters");
    if !dir.is_dir() {
        eprintln!("skipping: reference-old-filters/ not present");
        return;
    }
    let mut checked = 0;
    for entry in fs::read_dir(&dir).unwrap() {
        let path = entry.unwrap().path();
        if path.extension().and_then(|e| e.to_str()) != Some("filter") {
            continue;
        }
        let bytes = fs::read(&path).unwrap();
        check_clean(&path.display().to_string(), &bytes);
        checked += 1;
    }
    eprintln!("checked {checked} reference-old-filters/*.filter files");
    assert!(checked >= 1);
}

#[test]
fn corpus_unknown_keyword_kept_opaque() {
    let src = b"Show\n\tFutureKeyword 3\n\tRarity Rare\n";
    let doc = check_clean("unknown keyword", src);
    let diags = doc.diagnostics();
    assert_eq!(diags.len(), 1);
    assert_eq!(diags[0].severity, Severity::Warning);
    assert_eq!(doc.blocks().count(), 1);
}
