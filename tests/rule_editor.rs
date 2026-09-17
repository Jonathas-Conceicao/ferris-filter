//! Library-level integration coverage for the rule editor's mutate -> save -> compile pipeline
//! (Phase 14, Plan 01).
//!
//! Unlike `tests/dynamic_filter.rs`, this file never shells out to the `ferrisfilter` binary —
//! it drives `ferrisfilter::overlay::{store, edit}` and `ferrisfilter::dynamic::compile_store`
//! directly, exactly the sequence `app.rs`'s autosave + Apply will eventually call. Every case
//! runs in a fresh temp dir seeded from committed files (never the repo root, whose
//! `prices.json` is git-ignored and reflects whatever the last `sync` fetched — that mistake
//! cost time twice during v0.3, per `tests/dynamic_filter.rs`'s own header).

use ferrisfilter::compile::InputPaths;
use ferrisfilter::dynamic::{self, StoreState};
use ferrisfilter::overlay::{edit, store};
use std::fs;
use std::path::{Path, PathBuf};

/// Normalize CRLF to LF. The generator emits LF-only; the working tree is CRLF on Windows.
fn normalize(s: &str) -> String {
    s.replace("\r\n", "\n")
}

fn manifest() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
}

/// A fresh temp dir containing the inputs a dynamic compile needs. Fixtures are copied from the
/// repo by `CARGO_MANIFEST_DIR` — no test in this file reads or writes the repo root itself.
fn workspace(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "ferrisfilter_ruleeditor_{tag}_{:x}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos()
    ));
    fs::create_dir_all(&dir).expect("create temp dir");
    for f in ["dynamic_base.json", "tiers.json", "config.json"] {
        fs::copy(manifest().join(f), dir.join(f)).unwrap_or_else(|e| panic!("copy {f}: {e}"));
    }
    fs::copy(
        manifest().join("tests/fixtures/seed-prices.json"),
        dir.join("prices.json"),
    )
    .expect("copy seed prices");
    dir
}

/// Only the policy blocks — everything before the first economy section banner. Mirrors
/// `tests/dynamic_filter.rs`'s helper of the same name so an economy section that happens to
/// mention a flask base type cannot mask a failure in the policy half.
fn policy_section(filter: &str) -> String {
    match filter.find("#============") {
        Some(i) => filter[..i].to_string(),
        None => filter.to_string(),
    }
}

/// Seeds `dynamic.json` from `dynamic_base.json` in `dir` and loads it as a `Config`.
fn seed_and_load(dir: &Path) -> ferrisfilter::models::config::Config {
    let base = dir.join(dynamic::BASE_FILE);
    let store_path = dir.join(dynamic::STORE_FILE);
    let view = store::load(&base, &store_path).expect("seed + load the store");
    assert_eq!(view.seeded, true, "a fresh workspace must seed from the template");
    view.config
}

fn store_path(dir: &Path) -> PathBuf {
    dir.join(dynamic::STORE_FILE)
}

fn compile_policy(dir: &Path) -> String {
    let compiled = dynamic::compile_store(&InputPaths::relative_to(dir), &store_path(dir))
        .expect("compile_store");
    policy_section(&normalize(&compiled.text))
}

// ---------------------------------------------------------------------------
// Roadmap success criterion 2 — the milestone's motivating example:
// "through the overlay the user can hide all flask drops and have that overwrite a
// dynamic.filter." dynamic_base.json ships with no flask/charm rule at all (its ten rules cover
// armour, weapon and jewellery classes only), so this test builds one through the edit API,
// compiles, asserts the block is present, toggles it off, recompiles, and asserts it is gone —
// against compiled filter text, not struct state, per the plan's explicit hazard warning.
// ---------------------------------------------------------------------------

#[test]
fn disabling_one_flask_rule_removes_the_flask_block_from_the_compiled_filter() {
    let dir = workspace("flask");
    let config = seed_and_load(&dir);

    // Build the working model from the seeded store, add a new rule covering every flask/charm
    // class, styled `hidden` (D-07's noop default is `unchanged`; this test deliberately picks a
    // visible-effect style so the "present" half of the assertion is meaningful).
    let mut wr = edit::WorkingRules::from_rules(config.rules.clone().unwrap_or_default());
    let flask_id = wr.add();
    {
        let rule = wr.rule_mut(flask_id).expect("just-added rule must exist");
        edit::set_style(rule, "hidden");
        for class in config.flask_charm_classes.clone() {
            edit::set_class(edit::when_mut(rule), &class, true);
        }
    }

    let mut config = config;
    config.rules = Some(wr.to_rules());
    store::save(&config, &store_path(&dir)).expect("save the store with the new flask rule");

    let before = compile_policy(&dir);
    assert!(before.contains("\"Life Flasks\" \"Mana Flasks\" \"Charms\""), "expected the flask/charm class line in the compiled filter, got:\n{before}");

    // Disable the same rule (by id — D-19) and recompile.
    wr.toggle_enabled(flask_id);
    config.rules = Some(wr.to_rules());
    store::save(&config, &store_path(&dir)).expect("save the store with the flask rule disabled");

    let after = compile_policy(&dir);
    assert!(!after.contains("\"Life Flasks\" \"Mana Flasks\" \"Charms\""), "disabling the rule must remove its block from the compiled filter, got:\n{after}");
}

// ---------------------------------------------------------------------------
// Roadmap success criterion 1 — two overlapping rules compile in list order, and reordering
// changes which style the first matching block carries.
// ---------------------------------------------------------------------------

#[test]
fn move_up_changes_which_style_wins_for_overlapping_rules() {
    let dir = workspace("reorder");
    let base = dir.join(dynamic::BASE_FILE);
    let store_path = store_path(&dir);
    let state = dynamic::ensure_store(&base, &store_path).expect("seed store");
    assert_eq!(state, StoreState::Seeded);

    // Two rules that both match "Boots", styled differently, so which one wins is entirely a
    // function of array order (first-match-wins).
    let rule_e = ferrisfilter::models::config::Rule {
        enabled: None,
        name: None,
        style: "E".to_string(),
        when: Some(ferrisfilter::models::config::When {
            class: Some(vec!["Boots".to_string()]),
            ..Default::default()
        }),
    };
    let rule_f = ferrisfilter::models::config::Rule {
        enabled: None,
        name: None,
        style: "F".to_string(),
        when: Some(ferrisfilter::models::config::When {
            class: Some(vec!["Boots".to_string()]),
            ..Default::default()
        }),
    };

    let mut wr = edit::WorkingRules::from_rules(vec![rule_e, rule_f]);
    let e_id = wr.rows()[0].0;
    let f_id = wr.rows()[1].0;

    let mut config = store::load(&base, &store_path).expect("load store").config;
    config.rules = Some(wr.to_rules());
    store::save(&config, &store_path).expect("save two-rule store");

    let first = compile_policy(&dir);
    let e_pos = first.find("Show # rule 0").expect("rule 0 block");
    let f_pos = first.find("Show # rule 1").expect("rule 1 block");
    assert!(e_pos < f_pos, "authored order (E then F) should hold before any reorder");

    // Move the second rule (F) up past E -- the caller re-resolves both rules by id, per D-19,
    // never by the index that just changed.
    assert!(wr.move_up(f_id));
    assert_eq!(wr.position(f_id), Some(0));
    assert_eq!(wr.position(e_id), Some(1));
    assert_eq!(wr.rule(f_id).unwrap().style, "F", "id still resolves to the F-styled rule");
    assert_eq!(wr.rule(e_id).unwrap().style, "E", "id still resolves to the E-styled rule");

    config.rules = Some(wr.to_rules());
    store::save(&config, &store_path).expect("save reordered store");

    let second = compile_policy(&dir);
    let f_pos2 = second.find("Show # rule 0").expect("rule 0 block after reorder");
    let e_pos2 = second.find("Show # rule 1").expect("rule 1 block after reorder");
    assert!(
        f_pos2 < e_pos2,
        "moving F up must swap the compiled block order -- array order is the precedence"
    );
    // And the block content itself carries F's style marker, not E's -- e.g. font 25 is F-only,
    // font 29 is E-only (CLAUDE.md's Named styles table).
    let f_block_start = &second[f_pos2..];
    let f_block = &f_block_start[..f_block_start.find("Show # rule 1").unwrap_or(f_block_start.len())];
    assert!(
        f_block.contains("SetFontSize 25"),
        "the first matching block after the reorder must be F's, got:\n{f_block}"
    );
}
