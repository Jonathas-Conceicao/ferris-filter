// Injectable HTTP client seam for poe.ninja API calls.
// The trait allows tests to swap in a fake returning recorded fixture strings
// without making any live network calls.

pub trait HttpClient {
    async fn get_json(&self, url: &str, query: &[(&str, &str)]) -> miette::Result<String>;
}

pub struct ReqwestClient {
    inner: reqwest::Client,
}

impl ReqwestClient {
    pub fn new() -> miette::Result<Self> {
        // Install the ring crypto provider for rustls (required when using rustls-no-provider).
        // Safe to call multiple times — the provider is set once globally.
        let _ = rustls::crypto::ring::default_provider().install_default();

        let inner = reqwest::Client::builder()
            .user_agent("FerrisFilter/0.1 (github.com/Jonathas-Conceicao/poe2_myfilter; contact: jonathasaoc@gmail.com)")
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
