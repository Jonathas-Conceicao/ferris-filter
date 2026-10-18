//! Bakes the ABOUT tab's version and repository link into the binary from git: the version from
//! `git describe` (the release tag, `-dev` past it or with tracked edits), the link from the
//! `origin` remote. Without git, a tag or a web remote it falls back to the `Cargo.toml` version
//! and `repository`.

use std::path::Path;
use std::process::Command;

#[path = "src/build_info.rs"]
mod build_info;

fn git(args: &[&str]) -> Option<String> {
    let out = Command::new("git").args(args).output().ok()?;
    if !out.status.success() {
        return None;
    }
    let s = String::from_utf8(out.stdout).ok()?.trim().to_string();
    (!s.is_empty()).then_some(s)
}

fn rerun_if_exists(path: &Path) {
    // A missing path would make cargo rerun this script on every build.
    if path.exists() {
        println!("cargo:rerun-if-changed={}", path.display());
    }
}

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=src");
    println!("cargo:rerun-if-changed=Cargo.toml");
    if let Some(git_dir) = git(&["rev-parse", "--git-dir"]) {
        let git_dir = Path::new(&git_dir);
        let common = git(&["rev-parse", "--git-common-dir"]);
        let common = common.as_deref().map(Path::new).unwrap_or(git_dir);
        rerun_if_exists(&git_dir.join("HEAD"));
        rerun_if_exists(&git_dir.join("index"));
        rerun_if_exists(&common.join("refs/tags"));
        rerun_if_exists(&common.join("packed-refs"));
        rerun_if_exists(&common.join("config"));
        if let Some(branch) = git(&["symbolic-ref", "-q", "HEAD"]) {
            rerun_if_exists(&common.join(branch));
        }
    }

    let describe = git(&["describe", "--tags", "--match", "v*", "--dirty"]).unwrap_or_default();
    let version = build_info::version_from_describe(&describe)
        .unwrap_or_else(|| std::env::var("CARGO_PKG_VERSION").unwrap_or_default());
    let repo = git(&["remote", "get-url", "origin"])
        .and_then(|r| build_info::web_url(&r))
        .or_else(|| {
            std::env::var("CARGO_PKG_REPOSITORY")
                .ok()
                .and_then(|r| build_info::web_url(&r))
        })
        .unwrap_or_default();

    println!("cargo:rustc-env=FF_VERSION={version}");
    println!("cargo:rustc-env=FF_GIT_DESCRIBE={describe}");
    println!(
        "cargo:rustc-env=FF_REPO_LINK_TEXT={}",
        build_info::link_text(&repo)
    );
    println!("cargo:rustc-env=FF_REPO_URL={repo}");
}
