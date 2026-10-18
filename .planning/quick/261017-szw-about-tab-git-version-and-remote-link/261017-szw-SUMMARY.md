---
quick_id: 261017-szw
status: complete
date: 2026-10-17
---

# Quick 261017-szw: About tab version from git, link from origin — Summary

- `build.rs` (new) runs `git describe --tags --match v* --dirty` and `git remote get-url origin`
  and bakes `FF_VERSION`, `FF_GIT_DESCRIBE`, `FF_REPO_URL`, `FF_REPO_LINK_TEXT` into the build.
  No new crate: none reports the remote URL, and the git CLI covers both. Falls back to the
  `Cargo.toml` version / `repository` (added, the moved `ferris-filter` URL) without git.
- `src/build_info.rs` (new, std only, shared with `build.rs` via `#[path]`): describe parsing
  (`v0.6` -> `0.6`; past the tag or dirty -> `0.6-dev`; tags with `-` handled) and remote -> web
  URL (https / scp ssh / ssh://, credentials and ports dropped, local paths refused). 6 tests.
- ABOUT: `Version <FF_VERSION>`, the describe on hover of a `-dev` version, link from `origin`
  (hidden when there is no web URL). `skin::REPO_URL` / `REPO_LINK_TEXT` now come from the build.
- Fixed `config_sections_no_longer_draw_tiers`, which failed on a CRLF checkout.
- On this checkout: `0.6-dev` (describe `v0.6-1-ged66edb-dirty`), link
  `github.com/Jonathas-Conceicao/poe2-myfilter` (origin not yet re-pointed). Screenshot:
  `shots/about.png`.
- Checks: all tests pass, fmt clean, clippy 121 (baseline).
