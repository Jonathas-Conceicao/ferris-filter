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

// ---- PARSE-04: block-scoped edits (plan 16-04) ----

use ferrisfilter::filter::{BlockId, Document};

fn byte_offset(doc: &Document, line_idx: usize) -> usize {
    let bom = if doc.has_bom() { 3 } else { 0 };
    bom + doc.lines()[..line_idx]
        .iter()
        .map(|l| l.text().len() + l.eol().as_str().len())
        .sum::<usize>()
}

fn fixture() -> Vec<u8> {
    fs::read(manifest().join("tests/fixtures/parser/filterblade.filter")).unwrap()
}

fn enabled_ids(doc: &Document) -> Vec<BlockId> {
    doc.blocks().filter(|b| b.is_enabled()).map(|b| b.id()).collect()
}

#[test]
fn edit_replace_one_block_filterblade() {
    let orig = fixture();
    let mut doc = parse(&orig).unwrap();
    let id = enabled_ids(&doc)[99];
    let b = doc.block(id).unwrap();
    let owned = b.owned();
    let start = byte_offset(&doc, owned.start);
    let old_end = byte_offset(&doc, owned.end);
    let header = doc.lines()[b.header_line()].text().to_string();
    doc.replace_block_lines(id, vec![header.clone(), "\tRarity Unique".to_string()])
        .unwrap();
    let out = doc.to_bytes();
    let region = format!("{header}\r\n\tRarity Unique\r\n");
    assert_bytes_eq("edit prefix", &out[..start], &orig[..start]);
    assert_bytes_eq("edit region", &out[start..start + region.len()], region.as_bytes());
    assert_bytes_eq("edit suffix", &out[start + region.len()..], &orig[old_end..]);
}

#[test]
fn edit_disable_enable_roundtrip_filterblade() {
    let orig = fixture();
    let mut doc = parse(&orig).unwrap();
    let ids = enabled_ids(&doc);
    let mut done = 0;
    for id in ids.into_iter().step_by(50) {
        if doc.set_enabled(id, false).is_ok() {
            assert!(!doc.block(id).unwrap().is_enabled());
            doc.set_enabled(id, true).unwrap();
            done += 1;
        }
    }
    assert!(done >= 5, "only {done} blocks round-tripped");
    assert_bytes_eq("disable/enable", &doc.to_bytes(), &orig);
}

#[test]
fn edit_crlf_preserved() {
    let orig = fixture();
    let mut doc = parse(&orig).unwrap();
    let ids = enabled_ids(&doc);
    let header = doc.lines()[doc.block(ids[10]).unwrap().header_line()]
        .text()
        .to_string();
    doc.replace_block_lines(
        ids[10],
        vec![header, "\tClass \"Bows\"".to_string(), "\tRarity Rare".to_string()],
    )
    .unwrap();
    doc.set_meta(ids[20], "name", "x y").unwrap();
    let out = doc.to_bytes();
    for (i, &c) in out.iter().enumerate() {
        if c == b'\n' {
            assert!(i > 0 && out[i - 1] == b'\r', "lone LF at offset {i}");
        }
    }
}

#[test]
fn edit_set_meta_inserts_only_one_line() {
    let orig = fixture();
    let mut doc = parse(&orig).unwrap();
    let id = enabled_ids(&doc)[42];
    let at = byte_offset(&doc, doc.block(id).unwrap().header_line());
    doc.set_meta(id, "name", "Exceptional 3 socks").unwrap();
    let mut want = orig[..at].to_vec();
    want.extend_from_slice(b"#ff:name Exceptional 3 socks\r\n");
    want.extend_from_slice(&orig[at..]);
    assert_bytes_eq("set_meta", &doc.to_bytes(), &want);
    assert_eq!(doc.meta_get(id, "name"), Some("Exceptional 3 socks"));
}

#[test]
fn edit_no_final_newline() {
    let mut doc = parse(b"Hide\nShow\n\tRarity Rare").unwrap();
    let id = doc.blocks().nth(1).unwrap().id();
    doc.replace_block_lines(
        id,
        vec!["Show".into(), "\tRarity Unique".into(), "\tSockets 3".into()],
    )
    .unwrap();
    assert_bytes_eq(
        "no final newline",
        &doc.to_bytes(),
        b"Hide\nShow\n\tRarity Unique\n\tSockets 3",
    );
}

#[test]
fn edit_ids_stable_filterblade() {
    let orig = fixture();
    let mut doc = parse(&orig).unwrap();
    let heads = |d: &Document| -> Vec<(BlockId, String)> {
        d.blocks()
            .map(|b| (b.id(), d.lines()[b.header_line()].text().to_string()))
            .collect()
    };
    let before = heads(&doc);
    let target = enabled_ids(&doc)[7];
    doc.set_meta(target, "style", "S").unwrap();
    let header = before.iter().find(|(i, _)| *i == target).unwrap().1.clone();
    doc.replace_block_lines(target, vec![header, "\tRarity Rare".into()])
        .unwrap();
    assert_eq!(heads(&doc), before);
}

#[test]
fn parse_never_panics_on_prefixes() {
    fn probe(bytes: &[u8]) {
        if let Ok(doc) = parse(bytes) {
            assert_eq!(doc.to_bytes(), bytes);
        }
    }
    let fx = fixture();
    let mut n = 0;
    while n <= fx.len() {
        probe(&fx[..n]);
        n += 997;
    }
    let sample = "\u{feff}# c\r\n#ff:name x\r\nShow # t\r\n\tBaseType == \"a # b\"\r\n\tSetFontSize 30\r\n#Hide\r\n#\tRarity Rare\r\nImport \"x\" Optional\r\nHide\r\n\té=>\r\n";
    for n in 0..=sample.len() {
        if sample.is_char_boundary(n) {
            probe(&sample.as_bytes()[..n]);
        }
    }
    for g in [
        "\"", "#", "=>", "Show \"", "#Show\n#\t", "\r\r\n", "\u{feff}\u{feff}", "Show\n\t\"", "#Show\n#\tRarity \"",
    ] {
        probe(g.as_bytes());
    }
}
