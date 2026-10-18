//! Turns git output into the version and repository link the ABOUT tab shows. `build.rs` includes
//! this file (std only, no crate imports) and bakes the results into the binary as the
//! `FF_VERSION`, `FF_GIT_DESCRIBE`, `FF_REPO_URL` and `FF_REPO_LINK_TEXT` environment variables.

/// The version for `git describe --tags --match v* --dirty` output: the tag without its leading
/// `v` (`v0.6` -> `0.6`), plus `-dev` when the build is past the tag (`v0.6-3-gabc1234`) or the
/// tracked tree has edits (`-dirty`). `None` for empty output.
pub fn version_from_describe(describe: &str) -> Option<String> {
    let describe = describe.trim();
    let (rest, dirty) = match describe.strip_suffix("-dirty") {
        Some(r) => (r, true),
        None => (describe, false),
    };
    // `<tag>-<n>-g<hash>`; a tag may itself contain `-` (`v0.7-rc1`), so split from the right.
    let mut parts = rest.rsplitn(3, '-');
    let (hash, count, head) = (parts.next(), parts.next(), parts.next());
    let (tag, ahead) = match (hash, count, head) {
        (Some(h), Some(n), Some(tag))
            if h.len() > 1
                && h.starts_with('g')
                && h[1..].chars().all(|c| c.is_ascii_hexdigit())
                && !n.is_empty()
                && n.chars().all(|c| c.is_ascii_digit()) =>
        {
            (tag, true)
        }
        _ => (rest, false),
    };
    let tag = tag.strip_prefix('v').unwrap_or(tag);
    if tag.is_empty() {
        return None;
    }
    Some(if dirty || ahead {
        format!("{tag}-dev")
    } else {
        tag.to_string()
    })
}

/// The https web page for a git remote URL: `https://host/owner/repo` from an https, scp-style
/// (`git@host:owner/repo.git`) or `ssh://` remote, without credentials, a port, `.git` or a
/// trailing `/`. `None` for anything else (a local path, `file://`), so a build never links to
/// something that is not a web page.
pub fn web_url(remote: &str) -> Option<String> {
    let remote = remote.trim();
    let (host_and_path, ssh) = if let Some(r) = remote.strip_prefix("https://") {
        (r.to_string(), false)
    } else if let Some(r) = remote.strip_prefix("http://") {
        (r.to_string(), false)
    } else if let Some(r) = remote.strip_prefix("ssh://") {
        (r.to_string(), true)
    } else if !remote.contains("://") && remote.contains('@') && remote.contains(':') {
        // scp-style `user@host:owner/repo`; the first `:` separates host and path.
        let (host, path) = remote.split_once(':')?;
        (format!("{host}/{path}"), true)
    } else {
        return None;
    };
    let (authority, path) = host_and_path.split_once('/')?;
    // Drop `user[:token]@` (never bake credentials into the binary) and an ssh port.
    let host = authority.rsplit('@').next()?;
    let host = if ssh { host.split(':').next()? } else { host };
    let path = path.trim_end_matches('/');
    let path = path
        .strip_suffix(".git")
        .unwrap_or(path)
        .trim_end_matches('/');
    if host.is_empty() || path.is_empty() || host.contains('\\') || path.contains('\\') {
        return None;
    }
    Some(format!("https://{host}/{path}"))
}

/// The link text for a web URL: the URL without its scheme.
pub fn link_text(url: &str) -> &str {
    url.strip_prefix("https://")
        .or_else(|| url.strip_prefix("http://"))
        .unwrap_or(url)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_on_the_tag_is_the_tag() {
        assert_eq!(version_from_describe("v0.6\n").as_deref(), Some("0.6"));
        assert_eq!(
            version_from_describe("v0.7-rc1").as_deref(),
            Some("0.7-rc1")
        );
    }

    #[test]
    fn version_past_the_tag_or_dirty_is_dev() {
        assert_eq!(
            version_from_describe("v0.6-1-ged66edb").as_deref(),
            Some("0.6-dev")
        );
        assert_eq!(
            version_from_describe("v0.6-dirty").as_deref(),
            Some("0.6-dev")
        );
        assert_eq!(
            version_from_describe("v0.6-12-g0123abc-dirty").as_deref(),
            Some("0.6-dev")
        );
        assert_eq!(
            version_from_describe("v0.7-rc1-2-gabc1234").as_deref(),
            Some("0.7-rc1-dev")
        );
    }

    #[test]
    fn empty_describe_has_no_version() {
        assert_eq!(version_from_describe(""), None);
        assert_eq!(version_from_describe("v"), None);
    }

    #[test]
    fn remote_urls_become_web_urls() {
        let want = Some("https://github.com/Jonathas-Conceicao/ferris-filter".to_string());
        for remote in [
            "https://github.com/Jonathas-Conceicao/ferris-filter.git",
            "https://github.com/Jonathas-Conceicao/ferris-filter",
            "https://github.com/Jonathas-Conceicao/ferris-filter/",
            "https://user:ghp_secret@github.com/Jonathas-Conceicao/ferris-filter.git",
            "git@github.com:Jonathas-Conceicao/ferris-filter.git",
            "ssh://git@github.com/Jonathas-Conceicao/ferris-filter.git",
            "ssh://git@github.com:22/Jonathas-Conceicao/ferris-filter.git",
            "http://github.com/Jonathas-Conceicao/ferris-filter.git\n",
        ] {
            assert_eq!(web_url(remote), want, "{remote}");
        }
    }

    #[test]
    fn local_remotes_have_no_web_url() {
        for remote in [
            "",
            "C:\\repos\\ferris-filter",
            "../ferris-filter",
            "/srv/git/ferris-filter.git",
            "file:///srv/git/ferris-filter.git",
            "https://github.com",
        ] {
            assert_eq!(web_url(remote), None, "{remote}");
        }
    }

    #[test]
    fn link_text_drops_the_scheme() {
        assert_eq!(link_text("https://github.com/a/b"), "github.com/a/b");
    }
}
