//! Fixture-scale behaviour and EDIT-07 responsiveness of the overlay's pure working model.
//! The timing budgets are deliberately loose: they catch accidental quadratic behaviour, not
//! machine speed.

use std::time::{Duration, Instant};

use ferrisfilter::filter::BlockKeyword;
use ferrisfilter::overlay::rules::{Dir, RowAction, WorkingFilter};

fn fixture() -> String {
    let bytes = std::fs::read(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/parser/filterblade.filter"
    ))
    .expect("fixture readable");
    String::from_utf8(bytes).expect("fixture is utf-8")
}

#[test]
fn fixture_loads_every_block_as_a_row() {
    let wf = WorkingFilter::from_text(&fixture());
    let enabled = wf.rows().iter().filter(|r| r.enabled).count();
    let disabled = wf.rows().iter().filter(|r| !r.enabled).count();
    // Counts are derived, not pinned: a fixture refresh must not break this test.
    assert!(wf.rows().len() > 100, "fixture should be a realistic size");
    assert_eq!(enabled + disabled, wf.rows().len());
    assert!(enabled > 0 && disabled > 0);
    assert_eq!(
        *wf.economy(),
        ferrisfilter::overlay::rules::EconomyView::Absent
    );
    assert!(wf.blocking().is_empty());
    for r in wf.rows() {
        assert!(!r.label.contains("$type"), "{}", r.label);
        assert!(!r.label.contains("%D"), "{}", r.label);
        assert!(!r.label.contains('!'), "{}", r.label);
    }
    let mut wf = wf;
    assert!(!wf.matches("ring").is_empty());
    assert!(!wf.is_dirty());
}

#[test]
fn fixture_edits_stay_fast_and_revert_restores_bytes() {
    let text = fixture();
    let t0 = Instant::now();
    let mut wf = WorkingFilter::from_text(&text);
    assert!(
        t0.elapsed() < Duration::from_secs(30),
        "load {:?}",
        t0.elapsed()
    );

    let t = Instant::now();
    let hits = wf.matches("ring");
    assert!(!hits.is_empty());
    assert!(
        t.elapsed() < Duration::from_secs(2),
        "search {:?}",
        t.elapsed()
    );

    let t = Instant::now();
    let mut done = 0u32;
    for i in 0..100usize {
        let rows = wf.rows();
        let r = &rows[(i * 7) % rows.len()];
        let id = r.id;
        let action = match i % 6 {
            0 => RowAction::SetEnabled(id, !r.enabled),
            1 => RowAction::Move(id, if i % 2 == 0 { Dir::Up } else { Dir::Down }),
            2 => RowAction::SetKeyword(id, BlockKeyword::Hide),
            3 => RowAction::SetContinue(id, !r.has_continue),
            4 => {
                let o = wf.apply_action(RowAction::Duplicate(id));
                if let Some(copy) = o.focus {
                    wf.apply_action(RowAction::Delete(copy));
                }
                done += 1;
                continue;
            }
            _ => RowAction::SetName(id, format!("rule {i}")),
        };
        wf.apply_action(action);
        done += 1;
    }
    let avg = t.elapsed() / done;
    assert!(avg < Duration::from_secs(1), "avg {avg:?}");
    assert!(wf.is_dirty());

    wf.revert();
    assert_eq!(wf.text(), text);
    assert!(!wf.is_dirty());
}
