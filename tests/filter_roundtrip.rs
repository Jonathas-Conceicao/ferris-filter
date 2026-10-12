//! Byte-identity contract for `ferrisfilter::filter` (PARSE-03): `to_bytes(parse(x)) == x`.
//!
//! Fixtures are read with `fs::read` and never normalized (no CRLF->LF), so the test sees exactly
//! the bytes the checkout produced.

use ferrisfilter::filter::{parse, LoadError};
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
