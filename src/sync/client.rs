// Injectable HTTP client seam for poe.ninja API calls.
// The trait allows tests to swap in a fake returning recorded fixture strings
// without making any live network calls.

pub trait HttpClient {
    // Desugared from `async fn` to an explicit `impl Future` to silence the
    // `async_fn_in_trait` lint (auto trait bounds like `Send` cannot be expressed on a
    // bare `async fn` in a trait). The futures here are driven on a single `block_on`,
    // so no `Send` bound is required.
    fn get_json(
        &self,
        url: &str,
        query: &[(&str, &str)],
    ) -> impl std::future::Future<Output = miette::Result<String>>;
}

/// The overlay's HTTP user agent (economy sync, league list, update check).
/// It names the public repo and the running version.
pub const USER_AGENT: &str = concat!(
    "FerrisFilter/",
    env!("CARGO_PKG_VERSION"),
    " (github.com/Jonathas-Conceicao/ferris-filter; contact: jonathasaoc@gmail.com)"
);

/// The CLI's (`ff-cli`, `ferrisfilter`) HTTP user agent: same repo and contact text as
/// [`USER_AGENT`], its own product name. The CLI is never throttled.
pub const CLI_USER_AGENT: &str = concat!(
    "FerrisFilter-CLI/",
    env!("CARGO_PKG_VERSION"),
    " (github.com/Jonathas-Conceicao/ferris-filter; contact: jonathasaoc@gmail.com)"
);

#[cfg(test)]
mod user_agent_tests {
    use super::{CLI_USER_AGENT, USER_AGENT};

    #[test]
    fn cli_user_agent_is_its_own() {
        assert!(CLI_USER_AGENT.starts_with(concat!("FerrisFilter-CLI/", env!("CARGO_PKG_VERSION"))));
        assert!(CLI_USER_AGENT.contains("github.com/Jonathas-Conceicao/ferris-filter"));
        assert!(!CLI_USER_AGENT.to_lowercase().contains("myfilter"));
        assert!(!USER_AGENT.starts_with("FerrisFilter-CLI"));
        let tail = |s: &str| s[s.find(" (").unwrap()..].to_string();
        assert_eq!(tail(USER_AGENT), tail(CLI_USER_AGENT));
    }

    #[test]
    fn user_agent_names_the_public_repo() {
        assert!(USER_AGENT.starts_with(concat!("FerrisFilter/", env!("CARGO_PKG_VERSION"))));
        assert!(USER_AGENT.contains("github.com/Jonathas-Conceicao/ferris-filter"));
        assert!(!USER_AGENT.to_lowercase().contains("myfilter"));
    }
}

pub struct ReqwestClient {
    inner: reqwest::Client,
}

impl ReqwestClient {
    pub fn new(user_agent: &'static str) -> miette::Result<Self> {
        // Install the ring crypto provider for rustls (required when using rustls-no-provider).
        // Safe to call multiple times — the provider is set once globally.
        let _ = rustls::crypto::ring::default_provider().install_default();

        let inner = reqwest::Client::builder()
            .user_agent(user_agent)
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .map_err(|e| miette::miette!("Cannot build HTTP client: {e}"))?;
        Ok(Self { inner })
    }
}

impl HttpClient for ReqwestClient {
    async fn get_json(&self, url: &str, query: &[(&str, &str)]) -> miette::Result<String> {
        self.inner
            .get(url)
            .query(query)
            .send()
            .await
            .map_err(|e| miette::miette!("request to {url} failed: {e}"))?
            .error_for_status()
            .map_err(|e| miette::miette!("{url} returned HTTP error: {e}"))?
            .text()
            .await
            .map_err(|e| miette::miette!("reading body from {url} failed: {e}"))
    }
}
