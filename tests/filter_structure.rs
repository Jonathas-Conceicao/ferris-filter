//! Fixture-level proofs for the structural edits: byte preservation outside the touched units and
//! self-inverse behaviour on the FilterBlade fixture and the migrated v0.4 dynamic filter.

use ferrisfilter::filter::{BlockId, BlockKeyword, Document, RuleStep, parse, section};
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
        d.lines()[s.begin..s.end]
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

const RULE: &str = section::RULE_LINE;

#[test]
fn divider_insert_then_remove_is_identity() {
    let bytes = fixture();
    let mut d = parse(&bytes).unwrap();
    let ids: Vec<BlockId> = d.blocks().map(|b| b.id()).collect();
    for id in picks(&d) {
        let k = d.insert_section_above(id, "Probe").unwrap();
        assert_eq!(d.blocks().map(|b| b.id()).collect::<Vec<_>>(), ids);
        d.remove_section(k).unwrap();
        assert_bytes_eq("insert/remove above", d.serialize().as_bytes(), &bytes);
    }
    let k = d.insert_section_at_top("Top").unwrap();
    assert_eq!(k, 0);
    d.remove_section(k).unwrap();
    assert_bytes_eq("insert/remove top", d.serialize().as_bytes(), &bytes);
}

#[test]
fn divider_rename_touches_only_its_line() {
    let src = format!("Show\n\n{RULE}\n# ff:section Old\n{RULE}\n\nHide\n");
    let mut d = parse(src.as_bytes()).unwrap();
    d.rename_section(0, "New name").unwrap();
    let want = src.replace("Old", "New name");
    assert_bytes_eq("rename", d.serialize().as_bytes(), want.as_bytes());
}

#[test]
fn add_rule_lands_above_a_leading_divider() {
    let src = format!("{RULE}\n# ff:section A\n{RULE}\n\nShow\n\tRarity Rare\n");
    let mut d = parse(src.as_bytes()).unwrap();
    d.insert_rule_at_top(vec!["Show".to_string()]).unwrap();
    let want = format!("Show\n\n{src}");
    assert_bytes_eq(
        "add above leading divider",
        d.serialize().as_bytes(),
        want.as_bytes(),
    );

    let src = format!("Show\n\n{RULE}\n# ff:section A\n{RULE}\n\nHide\n");
    let mut d = parse(src.as_bytes()).unwrap();
    d.insert_rule_at_top(vec!["Show".to_string()]).unwrap();
    let want = format!("Show\n\n{src}");
    assert_bytes_eq(
        "add above first rule",
        d.serialize().as_bytes(),
        want.as_bytes(),
    );
}

fn banner(name: &str) -> String {
    format!("{RULE}\n# ff:section {name}\n{RULE}\n")
}

#[test]
fn divider_move_is_its_own_inverse() {
    for eol in ["\n", "\r\n"] {
        let src = format!(
            "Show\n\tRarity Rare\n\n{}\n# stray note\nHide\n\tRarity Normal\n\nShow\n\tClass \"Rings\"\n",
            banner("Mid")
        )
        .replace('\n', eol);
        let mut d = parse(src.as_bytes()).unwrap();
        let ids: Vec<BlockId> = d.blocks().map(|b| b.id()).collect();
        assert!(d.section_can_move(0, false));
        assert!(d.section_can_move(0, true));
        d.move_section(0, false).unwrap();
        assert_eq!(d.blocks().map(|b| b.id()).collect::<Vec<_>>(), ids);
        let down = d.serialize();
        assert_ne!(down, src);
        let at = down.find("# ff:section").unwrap();
        assert!(
            down[..at].contains("Hide"),
            "banner is now below the second rule"
        );
        assert!(
            down[at..].contains("Class"),
            "third rule is below the banner"
        );
        d.move_section(0, true).unwrap();
        assert_bytes_eq(
            "move down then up",
            d.serialize().as_bytes(),
            src.as_bytes(),
        );
    }
}

#[test]
fn divider_move_keeps_unterminated_last_line() {
    // The banner is followed directly by the unterminated rule (a trailing blank line cannot
    // carry "no terminator", so a blank last line is not exercised here).
    let src = format!("Show\n\n{}Hide", banner("A"));
    let mut d = parse(src.as_bytes()).unwrap();
    d.move_section(0, false).unwrap();
    d.move_section(0, true).unwrap();
    assert_bytes_eq("unterminated", d.serialize().as_bytes(), src.as_bytes());
}

#[test]
fn divider_steps_over_a_group_run() {
    let group = "# ff:group abc123 1/2\nShow\n\tBaseArmour > 0\n\n# ff:group abc123 2/2\nShow\n\tBaseArmour > 0\n";
    let src = format!("{}\n{group}\nHide\n", banner("S"));
    let mut d = parse(src.as_bytes()).unwrap();
    let ids: Vec<BlockId> = d.blocks().map(|b| b.id()).collect();
    assert_eq!(
        ferrisfilter::filter::group::analyze(&d)
            .members(ids[0])
            .len(),
        2
    );
    d.move_section(0, false).unwrap();
    let out = d.serialize();
    assert!(out.starts_with(&format!("{group}\n{RULE}\n# ff:section S")));
    let again = parse(out.as_bytes()).unwrap();
    let nid: Vec<BlockId> = again.blocks().map(|b| b.id()).collect();
    let g = ferrisfilter::filter::group::analyze(&again);
    assert_eq!(g.members(nid[0]).len(), 2, "run still valid");
    assert!(!g.is_broken(nid[0]) && !g.is_broken(nid[1]));
    d.move_section(0, true).unwrap();
    assert_bytes_eq(
        "group move inverse",
        d.serialize().as_bytes(),
        src.as_bytes(),
    );
}

#[test]
fn rule_hops_a_divider_into_the_next_section() {
    // X sits directly above the divider; hopping it makes it the first rule of that section.
    let src = format!(
        "Show\n\tRarity Rare\n\n{}\nHide\n\tRarity Normal\n",
        banner("Next")
    );
    let mut d = parse(src.as_bytes()).unwrap();
    let ids: Vec<BlockId> = d.blocks().map(|b| b.id()).collect();
    assert_eq!(
        d.rule_step(ids[0], false).unwrap(),
        RuleStep::CrossBanner(0)
    );
    assert_eq!(d.rule_step(ids[1], true).unwrap(), RuleStep::CrossBanner(0));
    d.move_section(0, true).unwrap();
    assert_eq!(
        d.blocks().map(|b| b.id()).collect::<Vec<_>>(),
        ids,
        "rule order unchanged"
    );
    let want = format!(
        "{}\nShow\n\tRarity Rare\n\nHide\n\tRarity Normal\n",
        banner("Next")
    );
    assert_bytes_eq("hop", d.serialize().as_bytes(), want.as_bytes());
    // Now the first rule is the first of the section; nothing is above it but the divider.
    assert_eq!(d.rule_step(ids[0], true).unwrap(), RuleStep::CrossBanner(0));
    assert_eq!(d.rule_step(ids[0], false).unwrap(), RuleStep::Swap(ids[1]));
}

#[test]
fn rule_step_blocks_at_the_edges() {
    let d = parse(b"Show\n\nHide\n").unwrap();
    let ids: Vec<BlockId> = d.blocks().map(|b| b.id()).collect();
    assert_eq!(d.rule_step(ids[0], true).unwrap(), RuleStep::Blocked);
    assert_eq!(d.rule_step(ids[0], false).unwrap(), RuleStep::Swap(ids[1]));
    assert_eq!(d.rule_step(ids[1], false).unwrap(), RuleStep::Blocked);
}

#[test]
fn nothing_crosses_the_economy_banner() {
    let src = format!(
        "Show\n\n{}\nHide\n\n{RULE}\n{}\n{RULE}\n\nShow\n\tClass \"Rings\"\n",
        banner("User"),
        section::BEGIN_LINE
    );
    let mut d = parse(src.as_bytes()).unwrap();
    let ids: Vec<BlockId> = d.blocks().map(|b| b.id()).collect();
    // ordinals: 0 = User, 1 = Economy
    assert!(!d.section_can_move(1, true));
    assert!(!d.section_can_move(1, false));
    assert!(
        d.section_can_move(0, false),
        "Hide is a user rule below the divider"
    );
    assert!(matches!(
        d.move_section(1, true),
        Err(ferrisfilter::filter::EditError::SectionLocked)
    ));
    assert!(matches!(
        d.move_section(1, false),
        Err(ferrisfilter::filter::EditError::SectionLocked)
    ));
    assert!(matches!(
        d.rename_section(1, "x"),
        Err(ferrisfilter::filter::EditError::SectionLocked)
    ));
    assert!(matches!(
        d.remove_section(1),
        Err(ferrisfilter::filter::EditError::SectionLocked)
    ));
    // The last user rule cannot step down over the economy banner; economy blocks are fixed.
    assert_eq!(d.rule_step(ids[1], false).unwrap(), RuleStep::Blocked);
    assert_eq!(d.rule_step(ids[2], true).unwrap(), RuleStep::Blocked);
    assert_eq!(d.rule_step(ids[2], false).unwrap(), RuleStep::Blocked);
    // The user divider may pass Hide, but then nothing further down is a user rule.
    d.move_section(0, false).unwrap();
    assert!(!d.section_can_move(0, false));
    assert_eq!(
        d.rule_step(ids[1], false).unwrap(),
        RuleStep::CrossBanner(0)
    );
    d.move_section(0, true).unwrap();
    assert_bytes_eq(
        "economy untouched",
        d.serialize().as_bytes(),
        src.as_bytes(),
    );
}
