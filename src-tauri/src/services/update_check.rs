//! Tells the user when GitHub has a newer release. Only checks and links to the
//! release page: installing stays manual, which also suits the portable build.

use std::time::Duration;

use serde::{Deserialize, Serialize};

use crate::error::{AppError, Result};

const LATEST_RELEASE: &str = "https://api.github.com/repos/Yessi-cmd/lumina/releases/latest";
/// Only pages under this prefix may be opened from the frontend.
const RELEASES_PAGE: &str = "https://github.com/Yessi-cmd/lumina/releases";
const REQUEST_TIMEOUT: Duration = Duration::from_secs(10);
const USER_AGENT: &str = concat!("Lumina/", env!("CARGO_PKG_VERSION"));

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateInfo {
    pub current: String,
    /// Newest published release, without the leading `v`.
    pub latest: String,
    pub available: bool,
    pub url: String,
    /// Release notes as written on GitHub (Markdown).
    pub notes: String,
}

#[derive(Debug, Deserialize)]
struct GithubRelease {
    tag_name: String,
    html_url: String,
    #[serde(default)]
    body: String,
}

pub async fn check() -> Result<UpdateInfo> {
    let client = reqwest::Client::builder()
        .timeout(REQUEST_TIMEOUT)
        .user_agent(USER_AGENT)
        .build()?;
    let response = client
        .get(LATEST_RELEASE)
        .header("Accept", "application/vnd.github+json")
        .send()
        .await?
        .error_for_status()?;
    let release: GithubRelease = response.json().await?;

    let current = env!("CARGO_PKG_VERSION");
    let latest = release.tag_name.trim_start_matches('v').to_owned();
    Ok(UpdateInfo {
        current: current.to_owned(),
        available: newer(&latest, current),
        latest,
        url: release.html_url,
        notes: release.body,
    })
}

/// Opens a Lumina release page in the default browser.
pub fn open_release(url: &str) -> Result<()> {
    if !url.starts_with(RELEASES_PAGE) {
        return Err(AppError::Message(format!("不允许打开的链接: {url}")));
    }
    // Explorer hands the URL to the browser and exits at once, so its status is moot.
    std::process::Command::new("explorer").arg(url).status()?;
    Ok(())
}

/// Numeric `major.minor.patch` comparison; anything unparsable is never newer.
fn newer(candidate: &str, current: &str) -> bool {
    match (parse(candidate), parse(current)) {
        (Some(a), Some(b)) => a > b,
        _ => false,
    }
}

fn parse(version: &str) -> Option<(u64, u64, u64)> {
    // Pre-release and build suffixes (`-beta.1`, `+abc`) are ignored.
    let core = version.split(['-', '+']).next()?;
    let mut parts = core.split('.').map(|p| p.parse::<u64>().ok());
    let major = parts.next()??;
    let minor = parts.next().unwrap_or(Some(0))?;
    let patch = parts.next().unwrap_or(Some(0))?;
    Some((major, minor, patch))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compares_versions_numerically() {
        assert!(newer("0.3.0", "0.2.2"));
        assert!(newer("0.10.0", "0.9.9"));
        assert!(newer("1.0", "0.9.9"));
        assert!(!newer("0.2.2", "0.2.2"));
        assert!(!newer("0.2.1", "0.2.2"));
        assert!(!newer("latest", "0.2.2"));
    }
}
