//! Phone notifications, for players who queue and walk away from the PC: a match was
//! found, champ select started, or the tilt warning. Providers are Bark (iOS) and
//! Server酱 (WeChat); messages go straight from this machine to the provider.

use std::time::Duration;

use serde_json::json;
use tauri::{AppHandle, Manager};

use crate::config::Settings;
use crate::error::{AppError, Result};
use crate::state::AppState;

const TIMEOUT: Duration = Duration::from_secs(10);
const USER_AGENT: &str = concat!("Lumina/", env!("CARGO_PKG_VERSION"));
const BARK_SERVER: &str = "https://api.day.app";
const SERVERCHAN: &str = "https://sctapi.ftqq.com";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    MatchFound,
    ChampSelect,
    Tilt,
}

impl Kind {
    fn enabled(self, settings: &Settings) -> bool {
        match self {
            Self::MatchFound => settings.push_match_found,
            Self::ChampSelect => settings.push_champ_select,
            Self::Tilt => settings.push_tilt,
        }
    }
}

fn configured(settings: &Settings) -> bool {
    settings.push_provider != "off" && !settings.push_key.is_empty()
}

pub fn on_phase(app: &AppHandle, phase: &str) {
    match phase {
        "ReadyCheck" => {
            let auto = app.state::<AppState>().settings().auto_accept;
            let body = if auto {
                "已开启自动接受，请留意客户端是否成功进入英雄选择"
            } else {
                "请尽快回到电脑接受对局"
            };
            notify(app, Kind::MatchFound, "找到对局", body);
        }
        "ChampSelect" => notify(app, Kind::ChampSelect, "英雄选择开始", "该选英雄了"),
        _ => {}
    }
}

/// Sends in the background, if a provider is set up and this kind of message is on.
pub fn notify(app: &AppHandle, kind: Kind, title: &str, body: &str) {
    let settings = app.state::<AppState>().settings();
    if !configured(&settings) || !kind.enabled(&settings) {
        return;
    }
    let (title, body) = (title.to_owned(), body.to_owned());
    tauri::async_runtime::spawn(async move {
        match send(&settings, &title, &body).await {
            Ok(()) => log::info!("push sent: {title}"),
            Err(err) => log::warn!("push failed: {err}"),
        }
    });
}

/// Sends one message through the saved settings, so the user can see it works.
pub async fn test(settings: &Settings) -> Result<()> {
    if !configured(settings) {
        return Err(AppError::Message("还没有选择推送渠道或填写密钥".to_owned()));
    }
    send(settings, "Lumina 测试", "推送已连通").await
}

async fn send(settings: &Settings, title: &str, body: &str) -> Result<()> {
    let client = reqwest::Client::builder()
        .timeout(TIMEOUT)
        .user_agent(USER_AGENT)
        .build()?;
    let key = settings.push_key.trim();
    let request = match settings.push_provider.as_str() {
        "bark" => client.get(bark_url(key, title, body)?),
        "serverchan" => {
            let url = format!("{SERVERCHAN}/{}.send", checked_key(key)?);
            let payload = json!({ "title": title, "desp": body });
            client.post(url).json(&payload)
        }
        _ => return Err(AppError::Message("没有选择推送渠道".to_owned())),
    };
    request.send().await?.error_for_status()?;
    Ok(())
}

/// Server酱 keys go into the URL path, so anything but letters, digits and `-_` is a typo.
fn checked_key(key: &str) -> Result<&str> {
    let allowed = |c: char| c.is_ascii_alphanumeric() || c == '-' || c == '_';
    if !key.is_empty() && key.chars().all(allowed) {
        return Ok(key);
    }
    Err(AppError::Message("推送密钥格式不对".to_owned()))
}

/// Bark takes `/{key}/{title}/{body}`. The key may also be the full address of a
/// self-hosted server, e.g. `https://bark.example.com/abc`.
fn bark_url(key: &str, title: &str, body: &str) -> Result<reqwest::Url> {
    let base = if key.starts_with("http://") || key.starts_with("https://") {
        key.to_owned()
    } else {
        format!("{BARK_SERVER}/{}", checked_key(key)?)
    };
    let mut url = reqwest::Url::parse(&base).map_err(|_| bad_address())?;
    push_segments(&mut url, &[title, body])?;
    Ok(url)
}

/// Appends path segments, percent-encoding whatever they contain.
fn push_segments(url: &mut reqwest::Url, parts: &[&str]) -> Result<()> {
    let mut segments = url.path_segments_mut().map_err(|()| bad_address())?;
    segments.pop_if_empty().extend(parts);
    Ok(())
}

fn bad_address() -> AppError {
    AppError::Message("推送地址不对".to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bark_message_is_percent_encoded_into_the_path() {
        let url = bark_url("abc123", "找到 对局", "a/b").unwrap();
        assert_eq!(url.host_str(), Some("api.day.app"));
        let segments: Vec<&str> = url.path_segments().unwrap().collect();
        assert_eq!(segments.len(), 3);
        assert_eq!(segments[0], "abc123");
        assert!(!segments[1].contains(' '));
        assert!(!segments[2].contains('/'));
    }

    #[test]
    fn bark_accepts_a_self_hosted_address() {
        let url = bark_url("https://push.example.com/abc/", "t", "b").unwrap();
        assert_eq!(url.host_str(), Some("push.example.com"));
        assert_eq!(url.path(), "/abc/t/b");
    }

    #[test]
    fn keys_with_odd_characters_are_rejected() {
        assert!(checked_key("SCT123abc_-").is_ok());
        assert!(checked_key("").is_err());
        assert!(checked_key("abc/def").is_err());
        assert!(checked_key("abc def").is_err());
        assert!(bark_url("a b", "t", "b").is_err());
    }

    #[test]
    fn nothing_is_sent_without_a_provider_and_key() {
        let mut settings = Settings::default();
        assert!(!configured(&settings));
        settings.push_provider = "bark".to_owned();
        assert!(!configured(&settings));
        settings.push_key = "abc".to_owned();
        assert!(configured(&settings));
    }
}
