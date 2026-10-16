//! Fixture-level proofs for the structural edits: byte preservation outside the touched units and
//! self-inverse behaviour on the FilterBlade fixture and the migrated v0.4 dynamic filter.

use ferrisfilter::filter::{BlockId, BlockKeyword, Document, parse, section};
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

fn first_diff(a: &[u8], b: &[u8]) -> Option<usize> {
    if a == b {
        return None;
    }
    Some(
        a.iter()
            .zip(b.iter())
            .position(|(x, y)| x != y)
            .unwrap_or(a.len().min(b.len())),
    )
}

fn assert_bytes_eq(label: &str, got: &[u8], want: &[u8]) {
    if let Some(off) = first_diff(got, want) {
        panic!(
            "{label}: bytes differ (got {} bytes, want {} bytes), first difference at offset {off}\n  got:  {}\n  want: {}",
            got.len(),
            want.len(),
            window(got, off),
            window(want, off)
        );
    }
}

fn fixture() -> Vec<u8> {
    fs::read(manifest().join("tests/fixtures/parser/filterblade.filter")).unwrap()
}

/// Ids of about 20 evenly spaced blocks, deterministic.
fn picks(d: &Document) -> Vec<BlockId> {
    let all: Vec<BlockId> = d.blocks().map(|b| b.id()).collect();
    let step = (all.len() / 20).max(1);
    all.iter().step_by(step).copied().take(20).collect()
}

fn pos(d: &Document, id: BlockId) -> usize {
    d.blocks().position(|b| b.id() == id).unwrap()
}

#[test]
fn move_then_inverse_is_identity() {
    let bytes = fixture();
    let mut d = parse(&bytes).unwrap();
    let ids = picks(&d);
    assert!(ids.len() >= 15);
    let all: Vec<BlockId> = d.blocks().map(|b| b.id()).collect();
    for id in ids {
        let up = pos(&d, id) > 0;
        d.move_block(id, up).unwrap();
        assert_ne!(d.to_bytes(), bytes, "a move changed nothing");
        d.move_block(id, !up).unwrap();
        assert_bytes_eq("move/inverse", &d.to_bytes(), &bytes);
        assert_eq!(d.blocks().map(|b| b.id()).collect::<Vec<_>>(), all);
    }
}

#[test]
fn duplicate_then_remove_is_identity() {
    let bytes = fixture();
    let mut d = parse(&bytes).unwrap();
    for id in picks(&d) {
        let copy = d.duplicate_block(id).unwrap();
        assert_ne!(copy, id);
        d.remove_block(copy).unwrap();
        assert_bytes_eq("duplicate/remove", &d.to_bytes(), &bytes);
    }
}

#[test]
fn keyword_change_and_back_is_identity() {
    let bytes = fixture();
    let mut d = parse(&bytes).unwrap();
    let mut changed = 0;
    for id in picks(&d) {
        let orig = d.block(id).unwrap().keyword();
        let other = if orig == BlockKeyword::Hide {
            BlockKeyword::Show
        } else {
            BlockKeyword::Hide
        };
        if d.set_block_keyword(id, other).is_ok() {
            assert_eq!(d.block(id).unwrap().keyword(), other);
            d.set_block_keyword(id, orig).unwrap();
            changed += 1;
        }
        assert_bytes_eq("keyword/back", &d.to_bytes(), &bytes);
    }
    assert!(changed >= 15, "only {changed} keyword edits applied");
}

#[test]
fn continue_toggle_and_back_is_identity() {
    let bytes = fixture();
    let mut d = parse(&bytes).unwrap();
    let mut toggled = 0;
    for id in picks(&d) {
        let had = d.block(id).unwrap().has_continue();
        if d.set_continue(id, !had).is_ok() {
            assert_eq!(d.block(id).unwrap().has_continue(), !had);
            d.set_continue(id, had).unwrap();
            toggled += 1;
        }
        // Removing then re-adding a pre-existing Continue may reposition it; adding first is exact.
        if had {
            d = parse(&bytes).unwrap();
        } else {
            assert_bytes_eq("continue/back", &d.to_bytes(), &bytes);
        }
    }
    assert!(toggled >= 10, "only {toggled} Continue toggles applied");
}

#[test]
fn a_move_changes_nothing_outside_the_two_units() {
    let bytes = fixture();
    let mut d = parse(&bytes).unwrap();
    let (id, lo, hi) = {
        let blocks: Vec<_> = d.blocks().collect();
        let k = blocks.len() / 2;
        let (a, b) = (blocks[k], blocks[k + 1]);
        let lines = d.lines();
        let mut end_line = b.span().end;
        while end_line < lines.len() && lines[end_line].text().trim().is_empty() {
            end_line += 1;
        }
        let offset = |line: usize| -> usize {
            lines[..line]
                .iter()
                .map(|l| l.text().len() + l.eol().as_str().len())
                .sum()
        };
        (a.id(), offset(a.span().start), offset(end_line))
    };
    d.move_block(id, false).unwrap();
    let got = d.to_bytes();
    assert_eq!(got.len(), bytes.len());
    assert_bytes_eq("prefix", &got[..lo], &bytes[..lo]);
    assert_bytes_eq("suffix", &got[hi..], &bytes[hi..]);
    assert_ne!(&got[lo..hi], &bytes[lo..hi]);
}

#[test]
fn enable_disable_works_on_moved_and_duplicated_blocks() {
    let bytes = fixture();
    let mut d = parse(&bytes).unwrap();
    let candidate = d
        .blocks()
        .skip(5)
        .map(|b| b.id())
        .find(|&id| {
            let mut t = parse(&bytes).unwrap();
            t.set_enabled(id, false).is_ok() && t.set_enabled(id, true).is_ok()
        })
        .unwrap();
    d.move_block(candidate, true).unwrap();
    d.set_enabled(candidate, false).unwrap();
    assert!(!d.block(candidate).unwrap().is_enabled());
    d.set_enabled(candidate, true).unwrap();
    let copy = d.duplicate_block(candidate).unwrap();
    d.set_enabled(copy, false).unwrap();
    assert!(d.block(copy).unwrap().can_enable());
    d.set_enabled(copy, true).unwrap();
    assert!(d.block(copy).unwrap().is_enabled());
}

#[test]
fn insert_top_leaves_existing_bytes_alone() {
    let bytes = fixture();
    let mut d = parse(&bytes).unwrap();
    let id = d
        .insert_rule_at_top(vec!["Show # N".into(), "  Rarity Rare".into()])
        .unwrap();
    assert_eq!(d.blocks().next().unwrap().id(), id);
    let inserted = d.to_bytes();
    // Everything before the new block (a free-standing preamble) and everything after it is
    // untouched.
    let grown = inserted.len() - bytes.len();
    let at = inserted.windows(8).position(|w| w == b"Show # N").unwrap();
    assert_bytes_eq("untouched prefix", &inserted[..at], &bytes[..at]);
    assert_bytes_eq("untouched suffix", &inserted[at + grown..], &bytes[at..]);
    d.remove_block(id).unwrap();
    assert_bytes_eq("after remove", &d.to_bytes(), &bytes);
}

#[test]
fn v04_dynamic_economy_is_never_touched() {
    let text = fs::read_to_string(manifest().join("tests/fixtures/v04-dynamic.filter")).unwrap();
    let migrated = section::migrate_v04(&text).unwrap().unwrap();
    let mut d = parse(migrated.as_bytes()).unwrap();
    let span = section::find(&d).unwrap().unwrap();
    let econ = |d: &Document| -> Vec<String> {
        let s = section::find(d).unwrap().unwrap();
        d.lines()[s.begin..=s.end]
            .iter()
            .map(|l| l.text().to_string())
            .collect()
    };
    let before = econ(&d);
    let rules: Vec<BlockId> = d
        .blocks()
        .filter(|b| b.header_line() < span.begin)
        .map(|b| b.id())
        .collect();
    let last = *rules.last().unwrap();
    assert_eq!(d.move_neighbour(last, false).unwrap(), None);
    assert!(d.move_block(last, false).is_err());
    let first_economy = d
        .blocks()
        .find(|b| b.header_line() > span.begin)
        .unwrap()
        .id();
    assert_eq!(d.move_neighbour(first_economy, true).unwrap(), None);

    let new = d.insert_rule_at_top(vec!["Show # Added".into()]).unwrap();
    assert!(section::find(&d).unwrap().is_some());
    assert_eq!(econ(&d), before);
    let s2 = section::find(&d).unwrap().unwrap();
    assert!(d.block(new).unwrap().header_line() < s2.begin);
    // The new rule is the first one: it cannot move up, and the economy stays untouched.
    assert_eq!(d.blocks().next().unwrap().id(), new);
    assert_eq!(d.move_neighbour(new, true).unwrap(), None);
    assert_eq!(econ(&d), before);
    d.remove_block(new).unwrap();
    assert_eq!(econ(&d), before);
}
