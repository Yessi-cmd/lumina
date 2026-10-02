//! Tells the user when GitHub has a newer release and, for installed copies, updates
//! in one click: download the NSIS installer, check it, run it passively and quit.
//! Portable copies only get the link to the release page.

use std::fmt::Write;
use std::path::{Path, PathBuf};
use std::time::Duration;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tauri::{AppHandle, Emitter};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

mod staging;

use crate::error::{AppError, Result};

const LATEST_RELEASE: &str = "https://api.github.com/repos/Yessi-cmd/lumina/releases/latest";
/// Only pages under this prefix may be opened from the frontend.
const RELEASES_PAGE: &str = "https://github.com/Yessi-cmd/lumina/releases";
/// Installers are only downloaded from this repository's release assets.
const DOWNLOADS: &str = "https://github.com/Yessi-cmd/lumina/releases/download/";
const INSTALLER_SUFFIX: &str = "-setup.exe";
pub const PROGRESS_EVENT: &str = "update://progress";
const REQUEST_TIMEOUT: Duration = Duration::from_secs(10);
/// Downloads have no overall limit, only a stall limit.
const READ_TIMEOUT: Duration = Duration::from_secs(30);
const MAX_INSTALLER_BYTES: u64 = 256 * 1024 * 1024;
const USER_AGENT: &str = concat!("Lumina/", env!("CARGO_PKG_VERSION"));

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateInfo {
    pub current: String,
    /// Newest published release, without the leading `v`.
    pub latest: String,
    pub available: bool,
    /// This copy was installed and the release has an installer, so one-click works.
    pub installable: bool,
    pub url: String,
    /// Release notes as written on GitHub (Markdown).
    pub notes: String,
}

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Progress {
    pub downloaded: u64,
    pub total: u64,
}

#[derive(Debug, Deserialize)]
struct GithubRelease {
    tag_name: String,
    html_url: String,
    #[serde(default)]
    body: String,
    #[serde(default)]
    assets: Vec<GithubAsset>,
}

#[derive(Debug, Deserialize)]
struct GithubAsset {
    name: String,
    browser_download_url: String,
    size: u64,
    /// `sha256:<hex>`; older releases may lack it.
    #[serde(default)]
    digest: Option<String>,
}

impl GithubRelease {
    fn version(&self) -> &str {
        self.tag_name.trim_start_matches('v')
    }

    fn installer(&self) -> Option<&GithubAsset> {
        self.assets
            .iter()
            .find(|a| a.name.ends_with(INSTALLER_SUFFIX))
    }
}

pub async fn check() -> Result<UpdateInfo> {
    let release = latest_release().await?;
    let current = env!("CARGO_PKG_VERSION");
    let available = newer(release.version(), current);
    Ok(UpdateInfo {
        current: current.to_owned(),
        latest: release.version().to_owned(),
        available,
        installable: available
            && installed()
            && release
                .installer()
                .is_some_and(|a| expected_digest(a).is_ok()),
        url: release.html_url,
        notes: release.body,
    })
}

/// Downloads the newest installer, starts it and quits so it can replace the files.
/// The installer restarts Lumina when it is done.
pub async fn install(app: &AppHandle) -> Result<()> {
    if !installed() {
        return Err(AppError::Message("免安装版请到发布页下载新版本".to_owned()));
    }
    let release = latest_release().await?;
    if !newer(release.version(), env!("CARGO_PKG_VERSION")) {
        return Err(AppError::Message("已是最新版本".to_owned()));
    }
    let asset = release
        .installer()
        .ok_or_else(|| AppError::Message("这个版本没有安装包".to_owned()))?;
    if !asset.browser_download_url.starts_with(DOWNLOADS) {
        let url = &asset.browser_download_url;
        return Err(AppError::Message(format!("不允许的下载地址: {url}")));
    }
    let expected = expected_digest(asset)?;
    if asset.size == 0 || asset.size > MAX_INSTALLER_BYTES {
        return Err(AppError::Message("安装包大小不合法".to_owned()));
    }
    let staging = staging::Staging::new()?;
    let path = staging.path();
    download(app, asset, &path).await?;
    // The directory denies unelevated writes. Keep a read-only file handle that
    // denies writes/deletion from disk verification through CreateProcess.
    let mut verified = tokio::fs::File::from_std(staging.open_verified()?);
    verify_file(&mut verified, asset.size, expected).await?;

    log::info!("starting installer {}", path.display());
    // Passive: progress only, no questions; /R starts Lumina again afterwards.
    std::process::Command::new(&path)
        .args(["/P", "/R"])
        .spawn()?;
    // NSIS may still need its on-disk executable while it starts up.
    staging.keep();
    app.exit(0);
    Ok(())
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

async fn latest_release() -> Result<GithubRelease> {
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
    Ok(response.json().await?)
}

async fn download(app: &AppHandle, asset: &GithubAsset, path: &Path) -> Result<()> {
    let client = reqwest::Client::builder()
        .connect_timeout(REQUEST_TIMEOUT)
        .read_timeout(READ_TIMEOUT)
        .user_agent(USER_AGENT)
        .build()?;
    let mut response = client
        .get(&asset.browser_download_url)
        .send()
        .await?
        .error_for_status()?;

    let total = asset.size;
    let mut file = tokio::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .await?;
    let mut downloaded = 0u64;
    while let Some(chunk) = response.chunk().await? {
        if chunk.len() as u64 > total.saturating_sub(downloaded) {
            return Err(AppError::Message("安装包超过声明大小".to_owned()));
        }
        file.write_all(&chunk).await?;
        downloaded += chunk.len() as u64;
        let _ = app.emit(PROGRESS_EVENT, Progress { downloaded, total });
    }
    file.flush().await?;

    if downloaded != total {
        let message = format!("安装包不完整（{downloaded}/{total} 字节）");
        return Err(AppError::Message(message));
    }
    Ok(())
}

fn expected_digest(asset: &GithubAsset) -> Result<&str> {
    asset
        .digest
        .as_deref()
        .and_then(|value| value.strip_prefix("sha256:"))
        .filter(|value| value.len() == 64 && value.bytes().all(|c| c.is_ascii_hexdigit()))
        .ok_or_else(|| AppError::Message("安装包缺少有效校验信息，请到发布页下载".to_owned()))
}

async fn verify_file(file: &mut tokio::fs::File, size: u64, expected: &str) -> Result<()> {
    let mut hasher = Sha256::new();
    let mut buffer = [0u8; 64 * 1024];
    let mut read = 0u64;
    loop {
        let count = file.read(&mut buffer).await?;
        if count == 0 {
            break;
        }
        read += count as u64;
        if read > size {
            return Err(AppError::Message("安装包大小校验失败".to_owned()));
        }
        hasher.update(&buffer[..count]);
    }
    if read != size || !hex(&hasher.finalize()).eq_ignore_ascii_case(expected) {
        return Err(AppError::Message("安装包校验失败，请重试".to_owned()));
    }
    Ok(())
}

/// The NSIS installer puts `uninstall.exe` beside the app; a portable copy has none.
fn installed() -> bool {
    let exe = std::env::current_exe().ok();
    let dir: Option<PathBuf> = exe.and_then(|e| e.parent().map(Path::to_path_buf));
    dir.is_some_and(|d| d.join("uninstall.exe").is_file())
}

fn hex(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        let _ = write!(out, "{b:02x}");
    }
    out
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
    fn requires_a_complete_sha256_digest() {
        let mut asset = GithubAsset {
            name: "installer-setup.exe".to_owned(),
            browser_download_url: String::new(),
            size: 1,
            digest: None,
        };
        for digest in [None, Some("sha256:ab"), Some("sha512:abcd")] {
            asset.digest = digest.map(str::to_owned);
            assert!(expected_digest(&asset).is_err());
        }
        asset.digest = Some(format!("sha256:{}", "g".repeat(64)));
        assert!(expected_digest(&asset).is_err());
        asset.digest = Some(format!("sha256:{}", "a".repeat(64)));
        assert!(expected_digest(&asset).is_ok());
    }

    #[test]
    fn compares_versions_numerically() {
        assert!(newer("0.3.0", "0.2.2"));
        assert!(newer("0.10.0", "0.9.9"));
        assert!(newer("1.0", "0.9.9"));
        assert!(!newer("0.2.2", "0.2.2"));
        assert!(!newer("0.2.1", "0.2.2"));
        assert!(!newer("latest", "0.2.2"));
    }

    #[test]
    fn finds_the_installer_asset() {
        let json = r#"{"tag_name":"v0.4.0","html_url":"u","assets":[
            {"name":"Lumina_0.4.0_x64_portable.zip","browser_download_url":"a","size":1},
            {"name":"Lumina_0.4.0_x64-setup.exe","browser_download_url":"b","size":2,
             "digest":"sha256:ab"}]}"#;
        let release: GithubRelease = serde_json::from_str(json).unwrap();
        assert_eq!(release.version(), "0.4.0");
        assert_eq!(release.installer().unwrap().browser_download_url, "b");
        assert_eq!(hex(&[0x0a, 0xff]), "0aff");
    }
}
