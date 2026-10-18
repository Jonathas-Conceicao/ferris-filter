---
quick_id: 261017-szw
slug: about-tab-git-version-and-remote-link
date: 2026-10-17
---

# Quick 261017-szw: About tab version from git, link from origin

**Goal:** the ABOUT tab's version is the current release tag (`0.6`), or the tag plus `-dev` when
the build is past the tag or the tracked tree is dirty; the repository link is the `origin`
remote's web URL.

## Tasks

1. `src/build_info.rs` (std-only, unit-tested): `version_from_describe` parses
   `git describe --tags --match v* --dirty` output; `web_url` turns an origin URL
   (https / scp-style ssh / ssh://) into an https web URL, dropping credentials and `.git`,
   refusing local paths. `link_text` strips the scheme.
2. `build.rs`: runs git, emits `FF_VERSION`, `FF_GIT_DESCRIBE`, `FF_REPO_URL`,
   `FF_REPO_LINK_TEXT`; falls back to `CARGO_PKG_VERSION` / `CARGO_PKG_REPOSITORY` without git.
   No new crate: no git-info crate reports the remote URL, and the git CLI covers both.
   rerun-if-changed on HEAD, the current branch ref, refs/tags, packed-refs, config, index, src,
   Cargo.toml.
3. `Cargo.toml` `repository` = the moved repo (fallback); `skin::REPO_URL` / `REPO_LINK_TEXT`
   and `about_view::version_text` read the env vars; describe on hover of the version.
4. CLAUDE.md About note, CHANGELOG `[Unreleased]` line. Tests, clippy, fmt.
