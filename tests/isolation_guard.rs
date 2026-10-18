//! Source-scan guard for test isolation.
//!
//! Integration tests link the non-test library, so the `cfg(test)` panic in the folder resolver
//! does not protect them: a spawned `apply`, `sync` or `ff` that is not pinned would resolve the
//! real `%APPDATA%\FerrisFilter` and could overwrite the developer's live files. Every spawn that
//! resolves the FerrisFilter folder must therefore pin `FERRISFILTER_HOME`, and the game folder is
//! pinned by `FERRISFILTER_GAME_DIR` (`dirs::document_dir` ignores `USERPROFILE`), both to a temp
//! dir.
//!
//! This file deliberately does not set `[env] FERRISFILTER_HOME` in `.cargo/config.toml`: that
//! would hide a missing pin and defeat the first-run tests.

use std::fs;
use std::path::{Path, PathBuf};

const ROOT: &str = env!("CARGO_MANIFEST_DIR");

fn rs_files(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(rd) = fs::read_dir(dir) else { return };
    for e in rd.flatten() {
        let p = e.path();
        if p.is_dir() {
            rs_files(&p, out);
        } else if p.extension().is_some_and(|x| x == "rs") {
            out.push(p);
        }
    }
}

fn files_under(sub: &str) -> Vec<PathBuf> {
    let mut v = Vec::new();
    rs_files(&Path::new(ROOT).join(sub), &mut v);
    v.sort();
    v
}

fn rel(p: &Path) -> String {
    p.strip_prefix(ROOT)
        .unwrap_or(p)
        .to_string_lossy()
        .replace('\\', "/")
}

fn is_self(p: &Path) -> bool {
    rel(p) == "tests/isolation_guard.rs"
}

fn line_of(text: &str, byte: usize) -> usize {
    text[..byte].matches('\n').count() + 1
}

#[test]
fn spawns_that_resolve_the_folder_set_home() {
    let spawn = concat!("cargo_", "bin(");
    let mut bad = Vec::new();
    for f in files_under("tests") {
        if is_self(&f) {
            continue;
        }
        let text = fs::read_to_string(&f).unwrap();
        let mut from = 0;
        while let Some(i) = text[from..].find(spawn) {
            let start = from + i;
            let after = start + spawn.len();
            let end = [".assert(", ".output(", ".ok(", ";"]
                .iter()
                .filter_map(|m| text[after..].find(m).map(|j| after + j))
                .min()
                .unwrap_or(text.len());
            let chain = &text[start..end];
            let is_ff = chain.starts_with(&format!("{spawn}\"ff\")"));
            let resolves = is_ff
                || chain.contains("\"apply\"")
                || chain.contains("\"sync\"")
                || (chain.contains("\"install\"") && !chain.contains("--dest"));
            if resolves
                && !(chain.contains(".env(\"FERRISFILTER_HOME\"")
                    && chain.contains(".env(\"FERRISFILTER_GAME_DIR\""))
            {
                bad.push(format!("{}:{}", rel(&f), line_of(&text, start)));
            }
            from = end.max(after);
        }
    }
    assert!(
        bad.is_empty(),
        "spawn resolves the FerrisFilter folder without .env(\"FERRISFILTER_HOME\") and \
         .env(\"FERRISFILTER_GAME_DIR\"): {bad:?}"
    );
}

#[test]
fn dirs_only_in_locations() {
    let needle = concat!("dirs", "::");
    let mut bad = Vec::new();
    for f in files_under("src") {
        if rel(&f) == "src/locations.rs" {
            continue;
        }
        let text = fs::read_to_string(&f).unwrap();
        for (n, line) in text.lines().enumerate() {
            let code = line.trim_start();
            if code.starts_with("//") {
                continue;
            }
            if line.contains(needle) {
                bad.push(format!("{}:{}", rel(&f), n + 1));
            }
        }
    }
    assert!(
        bad.is_empty(),
        "OS folder lookup outside src/locations.rs: {bad:?}"
    );
}

#[test]
fn no_env_mutation() {
    let needles = [concat!("set_", "var("), concat!("remove_", "var(")];
    let mut bad = Vec::new();
    for f in files_under("src").into_iter().chain(files_under("tests")) {
        if is_self(&f) {
            continue;
        }
        let text = fs::read_to_string(&f).unwrap();
        for n in needles {
            if let Some(i) = text.find(n) {
                bad.push(format!("{}:{}", rel(&f), line_of(&text, i)));
            }
        }
    }
    assert!(bad.is_empty(), "process environment mutated: {bad:?}");
}

#[test]
fn from_env_call_sites() {
    let needle = concat!("locations::", "from_env");
    let allowed = [
        "src/locations.rs",
        "src/run.rs",
        "src/sync/mod.rs",
        "src/overlay/app.rs",
    ];
    let mut bad = Vec::new();
    for f in files_under("src").into_iter().chain(files_under("tests")) {
        if is_self(&f) || allowed.contains(&rel(&f).as_str()) {
            continue;
        }
        let text = fs::read_to_string(&f).unwrap();
        if let Some(i) = text.find(needle) {
            bad.push(format!("{}:{}", rel(&f), line_of(&text, i)));
        }
    }
    assert!(
        bad.is_empty(),
        "folder resolver called from an unexpected place: {bad:?}"
    );
}

#[test]
fn no_old_resolver() {
    let needles = [concat!("paths::", "app_root"), concat!("crate::", "paths")];
    let mut bad = Vec::new();
    for f in files_under("src").into_iter().chain(files_under("tests")) {
        if is_self(&f) {
            continue;
        }
        let text = fs::read_to_string(&f).unwrap();
        for n in needles {
            if let Some(i) = text.find(n) {
                bad.push(format!("{}:{}", rel(&f), line_of(&text, i)));
            }
        }
    }
    assert!(bad.is_empty(), "old walk-up resolver referenced: {bad:?}");
}
