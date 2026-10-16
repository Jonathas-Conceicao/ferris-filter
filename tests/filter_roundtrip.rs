//! Byte-identity contract for `ferrisfilter::filter` (PARSE-03): `to_bytes(parse(x)) == x`.
//!
//! Fixtures are read with `fs::read` and never normalized (no CRLF->LF), so the test sees exactly
//! the bytes the checkout produced.

use ferrisfilter::filter::{BlockKeyword, LoadError, parse};
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
        .filter(|l| {
            l.keyword
                .is_some_and(|k| k.name.eq_ignore_ascii_case("continue"))
        })
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
        assert!(
            doc.meta_get(b.id(), "name").is_none(),
            "D-07: foreign comments are not a name"
        );
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
    for (src, dst) in [
        ("tiers-default.json", "tiers.json"),
        ("styles-default.json", "styles.json"),
    ] {
        fs::copy(manifest().join(src), dir.join(dst)).unwrap();
    }
    fs::copy(
        manifest().join("tests/fixtures/seed-prices.json"),
        dir.join("prices.json"),
    )
    .unwrap();
    let (body, _) = dynamic::economy_body(
        &compile::InputPaths::relative_to(dir),
        compile::PriceSourcePolicy::Auto,
    )
    .unwrap();
    let text = dynamic::apply_text("", dynamic::ApplyOp::AddEconomy(&body)).unwrap();
    let doc = check_clean("generated dynamic", text.as_bytes());
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
        let label = path.display().to_string();
        check_clean(&label, &bytes);
        let (live, refused, dis, bad) = toggle_sweep(&label, &bytes);
        eprintln!(
            "{label}: {live} live toggled ({refused} refused), {dis} disabled toggled ({bad} not enableable)"
        );
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
    doc.blocks()
        .filter(|b| b.is_enabled())
        .map(|b| b.id())
        .collect()
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
    assert_bytes_eq(
        "edit region",
        &out[start..start + region.len()],
        region.as_bytes(),
    );
    assert_bytes_eq(
        "edit suffix",
        &out[start + region.len()..],
        &orig[old_end..],
    );
}

#[test]
fn edit_disable_enable_roundtrip_filterblade() {
    let orig = fixture();
    let mut doc = parse(&orig).unwrap();
    let ids = enabled_ids(&doc);
    assert_eq!(ids.len(), 460);
    for id in ids {
        doc.set_enabled(id, false)
            .unwrap_or_else(|e| panic!("disable {id:?}: {e}"));
        assert!(!doc.block(id).unwrap().is_enabled());
        doc.set_enabled(id, true)
            .unwrap_or_else(|e| panic!("re-enable {id:?}: {e}"));
    }
    assert_bytes_eq("disable/enable", &doc.to_bytes(), &orig);
}

#[test]
fn edit_disable_enable_roundtrip_seed_universal() {
    // Space-indented, generator-written blocks: every one must disable and enable back exactly.
    let orig = fs::read(manifest().join("tests/fixtures/seed-universal.filter")).unwrap();
    let mut doc = parse(&orig).unwrap();
    let ids = enabled_ids(&doc);
    assert!(!ids.is_empty());
    for id in ids {
        doc.set_enabled(id, false)
            .unwrap_or_else(|e| panic!("disable {id:?}: {e}"));
        doc.set_enabled(id, true)
            .unwrap_or_else(|e| panic!("re-enable {id:?}: {e}"));
    }
    assert_bytes_eq("seed disable/enable", &doc.to_bytes(), &orig);
}

/// A disabled block's line as re-disabling writes it: `# ` + the line past its first `#` and one
/// optional space (a bare `#` when that is empty). The identity on the current `# ` form.
fn respaced(text: &str) -> String {
    let p = text.find('#').expect("disabled line without #");
    let rest = &text[p + 1..];
    let live = format!("{}{}", &text[..p], rest.strip_prefix(' ').unwrap_or(rest));
    if live.is_empty() {
        "#".to_string()
    } else {
        format!("# {live}")
    }
}

fn doc_bytes_with(doc: &Document, map: impl Fn(usize, &str) -> String) -> Vec<u8> {
    let mut out = Vec::new();
    if doc.has_bom() {
        out.extend_from_slice(&[0xEF, 0xBB, 0xBF]);
    }
    for (i, l) in doc.lines().iter().enumerate() {
        out.extend_from_slice(map(i, l.text()).as_bytes());
        out.extend_from_slice(l.eol().as_str().as_bytes());
    }
    out
}

fn enableable_disabled(doc: &Document) -> Vec<(BlockId, std::ops::Range<usize>)> {
    doc.blocks()
        .filter(|b| !b.is_enabled() && b.can_enable())
        .map(|b| (b.id(), b.owned()))
        .collect()
}

/// Toggle every block of `bytes` and back: each live block disables (as `# ` + line) and enables
/// to the original bytes; each enableable disabled block enables and disables to its `# ` form
/// ([`respaced`], the original bytes when already in that form), which round-trips exactly and
/// enables to the same live text as the original. Returns `(live toggled, live refused, disabled
/// toggled, disabled not enableable)`.
fn toggle_sweep(label: &str, bytes: &[u8]) -> (usize, usize, usize, usize) {
    let orig = parse(bytes).unwrap_or_else(|e| panic!("{label}: {e}"));
    let mut doc = orig.clone();
    let (mut live, mut refused) = (0, 0);
    for id in enabled_ids(&orig) {
        match doc.set_enabled(id, false) {
            Ok(()) => {
                let b = doc.block(id).unwrap();
                for l in b.owned() {
                    let t = doc.lines()[l].text();
                    assert!(t == "#" || t.starts_with("# "), "{label}: wrote {t:?}");
                }
                doc.set_enabled(id, true)
                    .unwrap_or_else(|e| panic!("{label}: re-enable {id:?}: {e}"));
                live += 1;
            }
            Err(_) => refused += 1,
        }
    }
    assert_bytes_eq(&format!("{label}: disable/enable"), &doc.to_bytes(), bytes);

    let dis = enableable_disabled(&orig);
    let not_enableable = orig.blocks().filter(|b| !b.is_enabled()).count() - dis.len();
    let want = doc_bytes_with(&orig, |i, t| {
        if dis.iter().any(|(_, r)| r.contains(&i)) {
            respaced(t)
        } else {
            t.to_string()
        }
    });
    let mut doc = orig.clone();
    let mut enabled_all = orig.clone();
    for (id, _) in &dis {
        doc.set_enabled(*id, true)
            .unwrap_or_else(|e| panic!("{label}: enable {id:?}: {e}"));
        doc.set_enabled(*id, false)
            .unwrap_or_else(|e| panic!("{label}: re-disable {id:?}: {e}"));
        enabled_all.set_enabled(*id, true).unwrap();
    }
    assert_bytes_eq(&format!("{label}: enable/disable"), &doc.to_bytes(), &want);

    // The `# ` form is a fixed point and enables to the same live text.
    let spaced = parse(&want).unwrap();
    let dis2 = enableable_disabled(&spaced);
    assert_eq!(dis2.len(), dis.len(), "{label}: enableable count changed");
    let mut again = spaced.clone();
    let mut spaced_enabled = spaced.clone();
    for (id, _) in &dis2 {
        again.set_enabled(*id, true).unwrap();
        again.set_enabled(*id, false).unwrap();
        spaced_enabled.set_enabled(*id, true).unwrap();
    }
    assert_bytes_eq(
        &format!("{label}: spaced round trip"),
        &again.to_bytes(),
        &want,
    );
    assert_bytes_eq(
        &format!("{label}: spaced enables the same"),
        &spaced_enabled.to_bytes(),
        &enabled_all.to_bytes(),
    );
    (live, refused, dis.len(), not_enableable)
}

#[test]
fn edit_enable_disable_roundtrip_filterblade() {
    let orig = fixture();
    let (live, refused, dis, bad) = toggle_sweep("filterblade", &orig);
    assert_eq!((live, refused), (460, 0));
    assert!(dis > 0);
    assert_eq!(bad, 0);
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
        vec![
            header,
            "\tClass \"Bows\"".to_string(),
            "\tRarity Rare".to_string(),
        ],
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
    want.extend_from_slice(b"# ff:name Exceptional 3 socks\r\n");
    want.extend_from_slice(&orig[at..]);
    assert_bytes_eq("set_meta", &doc.to_bytes(), &want);
    assert_eq!(doc.meta_get(id, "name"), Some("Exceptional 3 socks"));
}

#[test]
fn both_meta_spellings_read_and_untouched_files_roundtrip() {
    let src = "# notes\r\n#ff:name Legacy\r\n#ff:style S\r\nShow # a\r\n\tRarity Rare\r\n\r\n# ff:name Current\r\n#Hide\r\n#\tClass \"Bows\"\r\n\r\n####\r\n#ff:section Mine\r\n####\r\nShow # mine\r\n\r\n####\r\n#ff:economy-begin\r\n####\r\n\r\nShow # eco\r\n";
    let doc = parse(src.as_bytes()).unwrap();
    assert_eq!(doc.to_bytes(), src.as_bytes());
    let blocks: Vec<_> = doc.blocks().cloned().collect();
    assert_eq!(blocks.len(), 4);
    assert_eq!(ferrisfilter::filter::section::banners(&doc).len(), 2);
    assert_eq!(doc.meta_get(blocks[0].id(), "name"), Some("Legacy"));
    assert_eq!(doc.meta_get(blocks[0].id(), "style"), Some("S"));
    assert_eq!(doc.meta_get(blocks[1].id(), "name"), Some("Current"));
    assert!(!blocks[1].is_enabled());
    assert!(ferrisfilter::filter::section::find(&doc).unwrap().is_some());
    // Rewriting a legacy setting writes the current prefix in place; other bytes stay.
    let mut doc = doc;
    doc.set_meta(blocks[0].id(), "name", "New").unwrap();
    let out = String::from_utf8(doc.to_bytes()).unwrap();
    assert_eq!(out, src.replace("#ff:name Legacy", "# ff:name New"));
}

#[test]
fn edit_no_final_newline() {
    let mut doc = parse(b"Hide\nShow\n\tRarity Rare").unwrap();
    let id = doc.blocks().nth(1).unwrap().id();
    doc.replace_block_lines(
        id,
        vec![
            "Show".into(),
            "\tRarity Unique".into(),
            "\tSockets 3".into(),
        ],
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
        "\"",
        "#",
        "=>",
        "Show \"",
        "#Show\n#\t",
        "\r\r\n",
        "\u{feff}\u{feff}",
        "Show\n\t\"",
        "#Show\n#\tRarity \"",
    ] {
        probe(g.as_bytes());
    }
}

// ---- EDIT-01/EDIT-02: single-line body edits (plan 20-01) ----

use ferrisfilter::filter::LinePatch;
use ferrisfilter::filter::keywords::Kind;

/// `(block, doc line, value span text, operator)` for every condition/action line with a value in
/// an enabled or valid-disabled block.
fn patchable_lines(
    doc: &Document,
) -> Vec<(
    BlockId,
    usize,
    String,
    Option<ferrisfilter::filter::tokenize::Operator>,
    Kind,
)> {
    let mut out = Vec::new();
    for b in doc.blocks() {
        if !b.is_enabled() && !b.can_enable() {
            continue;
        }
        for bl in b.body() {
            let (Some(kw), Some(t)) = (bl.keyword, bl.tokens.as_ref()) else {
                continue;
            };
            if !matches!(kw.kind, Kind::Condition | Kind::Action) || t.values.is_empty() {
                continue;
            }
            let text = doc.lines()[bl.line].text();
            let off = if b.is_enabled() {
                0
            } else {
                text.find('#').map_or(0, |i| i + 1)
            };
            let first = t.values.first().unwrap().start;
            let last = t.values.last().unwrap().end;
            out.push((
                b.id(),
                bl.line,
                text[off + first..off + last].to_string(),
                t.operator.as_ref().map(|(_, op)| *op),
                kw.kind,
            ));
        }
    }
    out
}

#[test]
fn noop_value_patch_sweep_filterblade() {
    let orig = fixture();
    let mut doc = parse(&orig).unwrap();
    let lines = patchable_lines(&doc);
    assert!(lines.len() > 1000, "only {} lines swept", lines.len());
    for (id, line, vals, _, _) in &lines {
        doc.patch_line(*id, *line, LinePatch::Values(vals.clone()))
            .unwrap_or_else(|e| panic!("line {}: {e}", line + 1));
    }
    assert_bytes_eq("value sweep", &doc.to_bytes(), &orig);
}

#[test]
fn noop_operator_patch_sweep_filterblade() {
    let orig = fixture();
    let mut doc = parse(&orig).unwrap();
    let lines = patchable_lines(&doc);
    let mut n = 0usize;
    for (id, line, _, op, kind) in &lines {
        if *kind != Kind::Condition || op.is_none() {
            continue;
        }
        doc.patch_line(*id, *line, LinePatch::Operator(*op))
            .unwrap_or_else(|e| panic!("line {}: {e}", line + 1));
        n += 1;
    }
    assert!(n > 100, "only {n} operator lines swept");
    assert_bytes_eq("operator sweep", &doc.to_bytes(), &orig);
}

#[test]
fn values_patch_keeps_bare_class_operator() {
    let src = "Show\n\tClass \"Rings\"\n\tBaseType == \"Ruby Ring\"\n";
    let mut doc = parse(src.as_bytes()).unwrap();
    let id = doc.blocks().next().unwrap().id();
    doc.patch_line(id, 1, LinePatch::Values("\"Rings\" \"Amulets\"".into()))
        .unwrap();
    let out = doc.serialize();
    let changed: Vec<_> = out
        .lines()
        .zip(src.lines())
        .filter(|(a, b)| a != b)
        .collect();
    assert_eq!(changed.len(), 1);
    assert_eq!(changed[0].0, "\tClass \"Rings\" \"Amulets\"");
    assert!(!changed[0].0.contains("=="));
}

#[test]
fn insert_remove_roundtrip_crlf() {
    let orig = fixture();
    let mut doc = parse(&orig).unwrap();
    let ids: Vec<BlockId> = enabled_ids(&doc).into_iter().take(50).collect();
    for id in ids {
        let b = doc.block(id).unwrap();
        let after = b.owned().end - 1;
        let new = doc.insert_body_line(id, after, "Quality >= 1").unwrap();
        assert_eq!(new, after + 1);
        doc.remove_body_line(id, new).unwrap();
    }
    assert_bytes_eq("insert/remove", &doc.to_bytes(), &orig);
}

#[test]
fn banners_round_trip_byte_for_byte() {
    use ferrisfilter::filter::section::{self, BannerKind};
    let rule = section::RULE_LINE;
    let lf = format!(
        "# top\nShow # a\n  Rarity Rare\n\n{rule}\n# ff:section One\n{rule}\nShow # b\n\n#====\n# [[0100]] Foreign\n#====\nHide # c\n\n{rule}\n#ff:section Two\n{rule}\n\n{rule}\n{}\n{rule}\n\nShow # eco\n",
        section::BEGIN_LINE
    );
    for src in [
        lf.clone(),
        lf.replace('\n', "\r\n"),
        format!("\u{feff}{lf}"),
        format!("\u{feff}{}", lf.replace('\n', "\r\n")),
    ] {
        let doc = parse(src.as_bytes()).unwrap();
        assert_eq!(doc.to_bytes(), src.as_bytes());
        let b = section::banners(&doc);
        assert_eq!(b.len(), 3);
        assert_eq!(b[0].name, "One");
        assert_eq!(b[1].name, "Two");
        assert_eq!(b[2].kind, BannerKind::Economy);
    }
}

#[test]
fn foreign_banner_is_not_a_section() {
    let src = "#====\n# [[0100]] Title\n#====\nShow\n  Rarity Rare\n\n#######\n# [[0200]] Other\n#######\nHide\n";
    let doc = parse(src.as_bytes()).unwrap();
    assert!(ferrisfilter::filter::section::banners(&doc).is_empty());
    assert_eq!(doc.to_bytes(), src.as_bytes());
    // A bare tag without both rule lines is a plain comment too.
    let src = "# ff:section Lonely\nShow\n";
    let doc = parse(src.as_bytes()).unwrap();
    assert!(ferrisfilter::filter::section::banners(&doc).is_empty());
}
