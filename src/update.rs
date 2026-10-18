//! The update check behind the About tab's Check for updates button (UPD-01..03).
//!
//! Framework-free: no egui and no overlay types. Everything here is a pure, table-tested function
//! (version rule, date text, checksum parsing, file hash, release JSON, response classification,
//! and the one function that turns an outcome into its status message) plus the `UpdateHttp` seam
//! the network code plugs into. The check runs only on a click, never at startup.

use std::io::Read;
use std::path::{Path, PathBuf};
use std::time::Duration;

use semver::Version;
use serde::Deserialize;

/// GitHub owner of the project's releases.
pub const OWNER: &str = "Jonathas-Conceicao";
/// GitHub repository of the project's releases.
pub const REPO: &str = "ferris-filter";
/// The one URL the check asks; no environment variable or CLI switch overrides it.
pub const LATEST_URL: &str =
    "https://api.github.com/repos/Jonathas-Conceicao/ferris-filter/releases/latest";
/// Name of the checksum asset attached to every release.
pub const SUMS_NAME: &str = "SHA256SUMS.txt";
/// Prefix of the per-download work folder name.
pub const WORK_PREFIX: &str = "FerrisFilter-update-";
/// Largest accepted release JSON.
pub const MAX_API_BYTES: u64 = 1 << 20;
/// Largest accepted checksum file.
pub const MAX_SUMS_BYTES: u64 = 64 << 10;
/// Largest accepted installer download.
pub const MAX_INSTALLER_BYTES: u64 = 200 << 20;
/// A downloaded installer older than this is considered stale.
pub const STALE_AFTER: Duration = Duration::from_secs(24 * 60 * 60);

// ---------------------------------------------------------------------------------------------
// Version rule
// ---------------------------------------------------------------------------------------------

/// The running version reduced to major.minor.patch (pre-release and build metadata dropped).
pub fn current_base(pkg_version: &str) -> Option<Version> {
    let v = Version::parse(pkg_version).ok()?;
    Some(Version::new(v.major, v.minor, v.patch))
}

/// How a release tag compares with the running build.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Compare {
    Newer(Version),
    Same,
    Older,
}

/// Compare a release tag (one leading `v` allowed) with the running package version.
/// A pre-release tag, or a pre-release / draft release, counts as `Older`.
#[allow(clippy::result_unit_err)]
pub fn compare_release(
    tag: &str,
    prerelease: bool,
    draft: bool,
    current: &str,
) -> Result<Compare, ()> {
    let raw = tag.strip_prefix('v').unwrap_or(tag);
    let parsed = Version::parse(raw).map_err(|_| ())?;
    let base = current_base(current).ok_or(())?;
    if prerelease || draft || !parsed.pre.is_empty() {
        return Ok(Compare::Older);
    }
    let tag_base = Version::new(parsed.major, parsed.minor, parsed.patch);
    Ok(match tag_base.cmp(&base) {
        std::cmp::Ordering::Greater => Compare::Newer(tag_base),
        std::cmp::Ordering::Equal => Compare::Same,
        std::cmp::Ordering::Less => Compare::Older,
    })
}

/// The installer file name for a validated version.
pub fn installer_name(v: &Version) -> String {
    format!("FerrisFilter-{v}-setup.exe")
}

// ---------------------------------------------------------------------------------------------
// Dates
// ---------------------------------------------------------------------------------------------

/// Days since 1970-01-01 for a civil date (proleptic Gregorian).
pub fn days_from_civil(y: i64, m: u32, d: u32) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = y - era * 400;
    let mp = i64::from((m + 9) % 12);
    let doy = (153 * mp + 2) / 5 + i64::from(d) - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

/// Parse `YYYY-MM-DDTHH:MM:SSZ` (and nothing else) to unix seconds.
pub fn parse_published(s: &str) -> Option<i64> {
    let b = s.as_bytes();
    if b.len() != 20 {
        return None;
    }
    for (i, c) in b.iter().enumerate() {
        let ok = match i {
            4 | 7 => *c == b'-',
            10 => *c == b'T',
            13 | 16 => *c == b':',
            19 => *c == b'Z',
            _ => c.is_ascii_digit(),
        };
        if !ok {
            return None;
        }
    }
    let num = |a: usize, z: usize| -> i64 { s[a..z].parse::<i64>().unwrap_or(-1) };
    let (y, mo, d) = (num(0, 4), num(5, 7), num(8, 10));
    let (h, mi, se) = (num(11, 13), num(14, 16), num(17, 19));
    if !(1..=12).contains(&mo) || !(1..=31).contains(&d) || h >= 24 || mi >= 60 || se >= 60 {
        return None;
    }
    Some(days_from_civil(y, mo as u32, d as u32) * 86_400 + h * 3600 + mi * 60 + se)
}

/// The local `YYYY-MM-DD` date of a unix time, given the UTC offset in minutes.
pub fn release_date_text(unix: i64, utc_offset_minutes: i32) -> String {
    let local = (unix + i64::from(utc_offset_minutes) * 60).max(0) as u64;
    let (year, month, day) = crate::sync::prices::days_to_ymd(local / 86_400);
    format!("{year:04}-{month:02}-{day:02}")
}

// ---------------------------------------------------------------------------------------------
// Checksums
// ---------------------------------------------------------------------------------------------

/// The checksum file has no usable line for the requested name.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SumsError {
    Invalid,
}

/// Find the SHA-256 (lowercase hex) of `file_name` in a `sha256sum`-style text.
pub fn parse_sums(text: &str, file_name: &str) -> Result<String, SumsError> {
    let mut found: Option<String> = None;
    for line in text.lines() {
        let line = line.strip_suffix('\r').unwrap_or(line);
        let Some((hash, rest)) = line.split_once(' ') else {
            continue;
        };
        let rest = rest.trim_start_matches(' ');
        let name = rest.strip_prefix('*').unwrap_or(rest);
        if name != file_name {
            continue;
        }
        if hash.len() != 64 || !hash.bytes().all(|c| c.is_ascii_hexdigit()) {
            return Err(SumsError::Invalid);
        }
        let hash = hash.to_ascii_lowercase();
        match &found {
            Some(prev) if *prev != hash => return Err(SumsError::Invalid),
            _ => found = Some(hash),
        }
    }
    found.ok_or(SumsError::Invalid)
}

// ---------------------------------------------------------------------------------------------
// File hash
// ---------------------------------------------------------------------------------------------

/// Lowercase hex of a byte slice.
pub fn hex_lower(bytes: &[u8]) -> String {
    use std::fmt::Write;
    let mut out = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        let _ = write!(out, "{b:02x}");
    }
    out
}

/// SHA-256 of a file, streamed in 64 KiB reads.
pub fn sha256_file(path: &Path) -> std::io::Result<String> {
    let mut f = std::fs::File::open(path)?;
    let mut ctx = ring::digest::Context::new(&ring::digest::SHA256);
    let mut buf = vec![0u8; 64 * 1024];
    loop {
        let n = f.read(&mut buf)?;
        if n == 0 {
            break;
        }
        ctx.update(&buf[..n]);
    }
    Ok(hex_lower(ctx.finish().as_ref()))
}

/// Why a downloaded installer failed its last check before launch.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InstallerCheck {
    Gone,
    Mismatch,
}

/// Re-check an installer file against its expected lowercase hex digest.
pub fn verify_installer(path: &Path, expected_hex: &str) -> Result<(), InstallerCheck> {
    if !std::fs::metadata(path).is_ok_and(|m| m.is_file()) {
        return Err(InstallerCheck::Gone);
    }
    let actual = sha256_file(path).map_err(|_| InstallerCheck::Gone)?;
    if actual.eq_ignore_ascii_case(expected_hex) {
        Ok(())
    } else {
        Err(InstallerCheck::Mismatch)
    }
}

// ---------------------------------------------------------------------------------------------
// Release JSON
// ---------------------------------------------------------------------------------------------

/// The parts of a GitHub release this check reads.
#[derive(Deserialize, Debug, Clone)]
pub struct Release {
    pub tag_name: String,
    #[serde(default)]
    pub draft: bool,
    #[serde(default)]
    pub prerelease: bool,
    #[serde(default)]
    pub published_at: Option<String>,
    #[serde(default)]
    pub assets: Vec<Asset>,
}

/// One attached file of a release.
#[derive(Deserialize, Debug, Clone)]
pub struct Asset {
    pub name: String,
    pub browser_download_url: String,
    #[serde(default)]
    pub state: Option<String>,
    #[serde(default)]
    pub size: u64,
}

/// The fully uploaded asset with exactly this name.
pub fn find_asset<'a>(release: &'a Release, name: &str) -> Option<&'a Asset> {
    release
        .assets
        .iter()
        .find(|a| a.name == name && a.state.as_deref() == Some("uploaded"))
}

/// True only for `https://github.com/<owner>/<repo>/releases/download/<tag>/<name>` (ASCII case
/// ignored); every other scheme, host, path or query is refused.
pub fn asset_url_is_trusted(url: &str, tag: &str, name: &str) -> bool {
    let expected = format!("https://github.com/{OWNER}/{REPO}/releases/download/{tag}/{name}");
    url.eq_ignore_ascii_case(&expected)
}

// ---------------------------------------------------------------------------------------------
// HTTP seam and classification
// ---------------------------------------------------------------------------------------------

/// A buffered GET reply.
#[derive(Debug, Clone)]
pub struct Reply {
    pub status: u16,
    pub ratelimit_remaining: Option<u64>,
    pub retry_after: Option<u64>,
    pub body: Vec<u8>,
}

/// A request that produced no usable reply.
#[derive(Debug)]
pub enum FetchError {
    Transport(String),
    TooLarge,
    Sink(std::io::Error),
}

/// The network seam; tests plug in a fake, the overlay plugs in the reqwest implementation.
pub trait UpdateHttp {
    /// GET `url` with an `Accept` header, buffering at most `max_bytes` of body.
    fn get(&self, url: &str, accept: &str, max_bytes: u64) -> Result<Reply, FetchError>;
    /// GET `url`, streaming the body into `sink`; returns the HTTP status.
    fn download(
        &self,
        url: &str,
        max_bytes: u64,
        sink: &mut dyn FnMut(&[u8]) -> std::io::Result<()>,
    ) -> Result<u16, FetchError>;
}

/// What the latest-release request came back as.
#[derive(Debug, Clone)]
pub enum LatestResponse {
    Release(Release),
    NoReleases,
    RateLimited,
    Offline,
    BadResponse,
}

/// Sort a latest-release reply into one of the outcomes. A plain 403 is not a rate limit.
pub fn classify_latest(r: Result<Reply, FetchError>) -> LatestResponse {
    let Ok(reply) = r else {
        return LatestResponse::Offline;
    };
    match reply.status {
        200 => match serde_json::from_slice::<Release>(&reply.body) {
            Ok(rel) => LatestResponse::Release(rel),
            Err(_) => LatestResponse::BadResponse,
        },
        404 => LatestResponse::NoReleases,
        429 => LatestResponse::RateLimited,
        403 if reply.ratelimit_remaining == Some(0) || reply.retry_after.is_some() => {
            LatestResponse::RateLimited
        }
        _ => LatestResponse::BadResponse,
    }
}

// ---------------------------------------------------------------------------------------------
// Outcomes and messages
// ---------------------------------------------------------------------------------------------

/// A verified installer waiting to be launched.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReadyUpdate {
    pub version: String,
    pub published_unix: Option<i64>,
    pub installer: PathBuf,
    pub sha256: String,
}

/// Why a check or download did not end well.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Failure {
    Offline,
    RateLimited,
    NoInstaller { version: String },
    NoSums { version: String },
    BadSums { version: String },
    Mismatch { version: String },
    DownloadFailed { version: String },
    InstallerGone { version: String },
    BadResponse,
}

/// The end of one check.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CheckResult {
    UpToDate {
        version: String,
        released: Option<i64>,
    },
    Ready(ReadyUpdate),
    Failed(Failure),
}

/// Framework-free mirror of the skin's status kinds; the overlay maps it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UpdateKind {
    Info,
    Ok,
    Warning,
    Error,
}

/// The status text and kind for an outcome.
pub fn message_for(result: &CheckResult, utc_offset_minutes: i32) -> (String, UpdateKind) {
    let date = |u: Option<i64>| u.map(|u| release_date_text(u, utc_offset_minutes));
    match result {
        CheckResult::UpToDate { version, released } => match date(*released) {
            Some(d) => (
                format!("Up to date ({version}, released {d})"),
                UpdateKind::Ok,
            ),
            None => (format!("Up to date ({version})"), UpdateKind::Ok),
        },
        CheckResult::Ready(r) => match date(r.published_unix) {
            Some(d) => (
                format!("{} available, released {d}", r.version),
                UpdateKind::Info,
            ),
            None => (format!("{} available", r.version), UpdateKind::Info),
        },
        CheckResult::Failed(f) => match f {
            Failure::Offline => (
                "Could not reach GitHub. Check your connection and try again.".to_string(),
                UpdateKind::Warning,
            ),
            Failure::RateLimited => (
                "GitHub is limiting requests. Try again in a few minutes.".to_string(),
                UpdateKind::Warning,
            ),
            Failure::NoInstaller { version } => (
                format!(
                    "Release {version} has no installer. Download it from the repository page."
                ),
                UpdateKind::Error,
            ),
            Failure::NoSums { version } => (
                format!("Release {version} has no checksum file, so it was not downloaded."),
                UpdateKind::Error,
            ),
            Failure::BadSums { version } => (
                format!("The checksum file for {version} is not valid, so it was not downloaded."),
                UpdateKind::Error,
            ),
            Failure::Mismatch { version } => (
                format!("The download of {version} did not match its checksum and was deleted."),
                UpdateKind::Error,
            ),
            Failure::DownloadFailed { version } => (
                format!("Could not download {version}. Try again."),
                UpdateKind::Error,
            ),
            Failure::InstallerGone { version } => (
                format!("The installer for {version} is no longer there. Check for updates again."),
                UpdateKind::Error,
            ),
            Failure::BadResponse => (
                "Could not read the latest version from GitHub.".to_string(),
                UpdateKind::Error,
            ),
        },
    }
}

/// Status while the check runs.
pub fn checking_message() -> (String, UpdateKind) {
    ("Checking for updates…".to_string(), UpdateKind::Info)
}

/// Status while the installer downloads.
pub fn downloading_message(version: &str) -> (String, UpdateKind) {
    (format!("Downloading {version}…"), UpdateKind::Info)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn reply(status: u16, rem: Option<u64>, retry: Option<u64>, body: &str) -> Reply {
        Reply {
            status,
            ratelimit_remaining: rem,
            retry_after: retry,
            body: body.as_bytes().to_vec(),
        }
    }

    const JSON: &str = r#"{"tag_name":"v0.8.0","published_at":"2026-11-02T14:03:11Z","extra":{"x":1},
        "assets":[{"name":"FerrisFilter-0.8.0-setup.exe","browser_download_url":"https://github.com/x","state":"uploaded","size":5,"id":9},
                  {"name":"SHA256SUMS.txt","browser_download_url":"https://github.com/y","state":"starter"}]}"#;

    #[test]
    fn release_json_parses_and_find_asset() {
        let rel: Release = serde_json::from_str(JSON).unwrap();
        assert_eq!(rel.tag_name, "v0.8.0");
        assert_eq!(rel.assets.len(), 2);
        assert!(find_asset(&rel, "FerrisFilter-0.8.0-setup.exe").is_some());
        assert!(find_asset(&rel, "SHA256SUMS.txt").is_none());
        assert!(find_asset(&rel, "ferrisfilter-0.8.0-setup.exe").is_none());
    }

    #[test]
    fn trusted_urls() {
        let ok =
            "https://github.com/Jonathas-Conceicao/ferris-filter/releases/download/v0.8.0/a.exe";
        assert!(asset_url_is_trusted(ok, "v0.8.0", "a.exe"));
        assert!(asset_url_is_trusted(
            &ok.to_ascii_lowercase(),
            "v0.8.0",
            "a.exe"
        ));
        assert!(!asset_url_is_trusted(
            &ok.replace("https", "http"),
            "v0.8.0",
            "a.exe"
        ));
        assert!(!asset_url_is_trusted(
            &ok.replace("ferris-filter", "other"),
            "v0.8.0",
            "a.exe"
        ));
        assert!(!asset_url_is_trusted(ok, "v0.9.0", "a.exe"));
        assert!(!asset_url_is_trusted(ok, "v0.8.0", "b.exe"));
        assert!(!asset_url_is_trusted(
            &ok.replace("v0.8.0/", "v0.8.0/../"),
            "v0.8.0",
            "a.exe"
        ));
        assert!(!asset_url_is_trusted(
            &format!("{ok}?x=1"),
            "v0.8.0",
            "a.exe"
        ));
    }

    #[test]
    fn classify_latest_rows() {
        use LatestResponse as L;
        assert!(matches!(
            classify_latest(Err(FetchError::Transport("x".into()))),
            L::Offline
        ));
        assert!(matches!(
            classify_latest(Ok(reply(200, None, None, JSON))),
            L::Release(_)
        ));
        assert!(matches!(
            classify_latest(Ok(reply(200, None, None, "nope"))),
            L::BadResponse
        ));
        assert!(matches!(
            classify_latest(Ok(reply(404, None, None, ""))),
            L::NoReleases
        ));
        assert!(matches!(
            classify_latest(Ok(reply(429, None, None, ""))),
            L::RateLimited
        ));
        assert!(matches!(
            classify_latest(Ok(reply(403, Some(0), None, ""))),
            L::RateLimited
        ));
        assert!(matches!(
            classify_latest(Ok(reply(403, None, Some(60), ""))),
            L::RateLimited
        ));
        assert!(matches!(
            classify_latest(Ok(reply(403, Some(5), None, ""))),
            L::BadResponse
        ));
        assert!(matches!(
            classify_latest(Ok(reply(403, None, None, ""))),
            L::BadResponse
        ));
        assert!(matches!(
            classify_latest(Ok(reply(500, None, None, ""))),
            L::BadResponse
        ));
        assert!(matches!(
            classify_latest(Ok(reply(301, None, None, ""))),
            L::BadResponse
        ));
    }

    #[test]
    fn message_table() {
        use UpdateKind::*;
        let ver = || "0.8.0".to_string();
        let t = parse_published("2026-11-02T14:03:11Z");
        let rows: Vec<(CheckResult, &str, UpdateKind)> = vec![
            (
                CheckResult::UpToDate {
                    version: "0.7.0".into(),
                    released: parse_published("2026-10-18T12:00:00Z"),
                },
                "Up to date (0.7.0, released 2026-10-18)",
                Ok,
            ),
            (
                CheckResult::UpToDate {
                    version: "0.7.0".into(),
                    released: None,
                },
                "Up to date (0.7.0)",
                Ok,
            ),
            (
                CheckResult::Ready(ReadyUpdate {
                    version: ver(),
                    published_unix: t,
                    installer: PathBuf::new(),
                    sha256: String::new(),
                }),
                "0.8.0 available, released 2026-11-02",
                Info,
            ),
            (
                CheckResult::Ready(ReadyUpdate {
                    version: ver(),
                    published_unix: None,
                    installer: PathBuf::new(),
                    sha256: String::new(),
                }),
                "0.8.0 available",
                Info,
            ),
            (
                CheckResult::Failed(Failure::Offline),
                "Could not reach GitHub. Check your connection and try again.",
                Warning,
            ),
            (
                CheckResult::Failed(Failure::RateLimited),
                "GitHub is limiting requests. Try again in a few minutes.",
                Warning,
            ),
            (
                CheckResult::Failed(Failure::NoInstaller { version: ver() }),
                "Release 0.8.0 has no installer. Download it from the repository page.",
                Error,
            ),
            (
                CheckResult::Failed(Failure::NoSums { version: ver() }),
                "Release 0.8.0 has no checksum file, so it was not downloaded.",
                Error,
            ),
            (
                CheckResult::Failed(Failure::BadSums { version: ver() }),
                "The checksum file for 0.8.0 is not valid, so it was not downloaded.",
                Error,
            ),
            (
                CheckResult::Failed(Failure::Mismatch { version: ver() }),
                "The download of 0.8.0 did not match its checksum and was deleted.",
                Error,
            ),
            (
                CheckResult::Failed(Failure::DownloadFailed { version: ver() }),
                "Could not download 0.8.0. Try again.",
                Error,
            ),
            (
                CheckResult::Failed(Failure::InstallerGone { version: ver() }),
                "The installer for 0.8.0 is no longer there. Check for updates again.",
                Error,
            ),
            (
                CheckResult::Failed(Failure::BadResponse),
                "Could not read the latest version from GitHub.",
                Error,
            ),
        ];
        for (r, text, kind) in rows {
            assert_eq!(message_for(&r, 0), (text.to_string(), kind), "{r:?}");
        }
        assert_eq!(
            checking_message(),
            ("Checking for updates…".to_string(), Info)
        );
        assert_eq!(
            downloading_message("0.8.0"),
            ("Downloading 0.8.0…".to_string(), Info)
        );
    }

    fn v(s: &str) -> Version {
        Version::parse(s).unwrap()
    }

    #[test]
    fn compare_release_rows() {
        let c = compare_release;
        assert_eq!(
            c("v0.8.0", false, false, "0.7.0"),
            Ok(Compare::Newer(v("0.8.0")))
        );
        assert_eq!(c("v0.7.0", false, false, "0.7.0"), Ok(Compare::Same));
        assert_eq!(c("v0.6.1", false, false, "0.7.0"), Ok(Compare::Older));
        assert_eq!(c("v0.7.0", false, false, "0.7.0-dev"), Ok(Compare::Same));
        assert_eq!(
            c("0.8.0", false, false, "0.7.0"),
            Ok(Compare::Newer(v("0.8.0")))
        );
        assert_eq!(c("v0.8.0-rc.1", false, false, "0.7.0"), Ok(Compare::Older));
        assert_eq!(c("v0.8.0", true, false, "0.7.0"), Ok(Compare::Older));
        assert_eq!(c("v0.8.0", false, true, "0.7.0"), Ok(Compare::Older));
        assert_eq!(c("vbanana", false, false, "0.7.0"), Err(()));
        assert_eq!(c("v0.8", false, false, "0.7.0"), Err(()));
        assert_eq!(c("v0.8.0", false, false, "junk"), Err(()));
    }

    #[test]
    fn current_base_drops_pre_and_build() {
        assert_eq!(current_base("0.7.0-dev+abc"), Some(v("0.7.0")));
        assert_eq!(current_base("nope"), None);
    }

    #[test]
    fn installer_name_from_version() {
        assert_eq!(installer_name(&v("0.8.0")), "FerrisFilter-0.8.0-setup.exe");
    }

    #[test]
    fn parse_published_rows() {
        assert_eq!(parse_published("1970-01-01T00:00:00Z"), Some(0));
        assert_eq!(parse_published("2026-11-02T14:03:11Z"), Some(1_793_628_191));
        assert_eq!(parse_published("2026-11-02"), None);
        assert_eq!(parse_published("2026-11-02T14:03:11.5Z"), None);
        assert_eq!(parse_published(""), None);
        assert_eq!(parse_published("2026-13-02T14:03:11Z"), None);
        assert_eq!(parse_published("2026-11-02T24:03:11Z"), None);
    }

    #[test]
    fn release_date_text_applies_offset() {
        let t = parse_published("2026-11-02T23:30:00Z").unwrap();
        assert_eq!(release_date_text(t, 120), "2026-11-03");
        assert_eq!(release_date_text(t, 0), "2026-11-02");
        let u = parse_published("2026-11-02T01:00:00Z").unwrap();
        assert_eq!(release_date_text(u, -180), "2026-11-01");
        let leap = parse_published("2028-02-29T12:00:00Z").unwrap();
        assert_eq!(release_date_text(leap, 0), "2028-02-29");
        assert_eq!(release_date_text(0, -60), "1970-01-01");
    }

    const H1: &str = "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad";
    const H2: &str = "0000000000000000000000000000000000000000000000000000000000000001";

    #[test]
    fn parse_sums_rows() {
        let name = "FerrisFilter-0.8.0-setup.exe";
        let text = format!("{H1}  {name}\n{H2}  ff-cli.exe\n");
        assert_eq!(parse_sums(&text, name), Ok(H1.to_string()));
        assert_eq!(
            parse_sums(&format!("{H1} {name}\n"), name),
            Ok(H1.to_string())
        );
        assert_eq!(
            parse_sums(&format!("{H1} *{name}\n"), name),
            Ok(H1.to_string())
        );
        assert_eq!(
            parse_sums(&format!("{H1}  {name}\r\n{H2}  x\r\n"), name),
            Ok(H1.to_string())
        );
        assert_eq!(
            parse_sums(&format!("{}  {name}\n", H1.to_uppercase()), name),
            Ok(H1.to_string())
        );
        assert_eq!(
            parse_sums(&format!("{H1}  other\n"), name),
            Err(SumsError::Invalid)
        );
        assert_eq!(
            parse_sums(&format!("{}  {name}\n", &H1[..63]), name),
            Err(SumsError::Invalid)
        );
        assert_eq!(
            parse_sums(&format!("{}  {name}\n", "z".repeat(64)), name),
            Err(SumsError::Invalid)
        );
        assert_eq!(
            parse_sums(&format!("{H1}  {name}\n{H2}  {name}\n"), name),
            Err(SumsError::Invalid)
        );
        assert_eq!(
            parse_sums(&format!("{H1}  {name}\n{H1}  {name}\n"), name),
            Ok(H1.to_string())
        );
    }

    #[test]
    fn sha256_and_verify() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("f.bin");
        std::fs::write(&p, b"abc").unwrap();
        assert_eq!(sha256_file(&p).unwrap(), H1);
        assert_eq!(verify_installer(&p, H1), Ok(()));
        assert_eq!(verify_installer(&p, H2), Err(InstallerCheck::Mismatch));
        assert_eq!(
            verify_installer(&dir.path().join("missing"), H1),
            Err(InstallerCheck::Gone)
        );
        assert_eq!(verify_installer(dir.path(), H1), Err(InstallerCheck::Gone));
    }
}
