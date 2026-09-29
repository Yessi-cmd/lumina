//! Small conveniences for the League client: chat status, the career background, and
//! restarting a stuck client window.

use reqwest::Method;
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::error::{AppError, Result};
use crate::state::session::LcuSession;
use crate::state::AppState;

const CHAT_ME: &str = "/lol-chat/v1/me";
const PROFILE: &str = "/lol-summoner/v1/current-summoner/summoner-profile";
const RESTART_UX: &str = "/riotclient/kill-and-restart-ux";
/// Online, away and invisible.
const AVAILABILITIES: [&str; 3] = ["chat", "away", "offline"];
const MAX_STATUS_CHARS: usize = 100;

/// `/lol-chat/v1/me`, trimmed to what the tools show.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct ChatStatus {
    /// `chat` (online), `away`, `dnd` (in game), `offline` (invisible) or `mobile`.
    pub availability: String,
    pub status_message: String,
}

pub async fn chat_status(session: &LcuSession) -> Result<ChatStatus> {
    session.http.get(CHAT_ME).await
}

pub async fn set_availability(session: &LcuSession, availability: &str) -> Result<()> {
    if !AVAILABILITIES.contains(&availability) {
        return Err(AppError::Message(format!("不支持的状态: {availability}")));
    }
    let body = json!({ "availability": availability });
    let http = &session.http;
    http.send_json(Method::PUT, CHAT_ME, Some(&body)).await
}

pub async fn set_status_message(session: &LcuSession, message: &str) -> Result<()> {
    if message.chars().count() > MAX_STATUS_CHARS {
        return Err(AppError::Message(format!("签名最多 {MAX_STATUS_CHARS} 个字")));
    }
    let body = json!({ "statusMessage": message });
    let http = &session.http;
    http.send_json(Method::PUT, CHAT_ME, Some(&body)).await
}

/// One skin of a champion, from `/lol-champions/v1/inventories/.../skins`.
#[derive(Debug, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
struct LcuSkin {
    id: i64,
    name: String,
    tile_path: String,
    ownership: Ownership,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
struct Ownership {
    owned: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Skin {
    pub id: i64,
    pub name: String,
    pub owned: bool,
    /// LCU asset path of the skin's tile; empty when the client gives none.
    pub tile_path: String,
}

pub async fn champion_skins(
    session: &LcuSession,
    summoner_id: u64,
    champion_id: i64,
) -> Result<Vec<Skin>> {
    let inventory = format!("/lol-champions/v1/inventories/{summoner_id}");
    let path = format!("{inventory}/champions/{champion_id}/skins");
    let found: Vec<LcuSkin> = session.http.get(&path).await?;
    let mut skins = Vec::new();
    for skin in found.into_iter().filter(|s| s.id > 0) {
        skins.push(Skin {
            id: skin.id,
            name: skin.name,
            owned: skin.ownership.owned,
            tile_path: skin.tile_path,
        });
    }
    Ok(skins)
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
struct SummonerProfile {
    background_skin_id: i64,
}

/// The skin id of the current career background; 0 when there is none.
pub async fn profile_background(session: &LcuSession) -> Result<i64> {
    let profile: SummonerProfile = session.http.get(PROFILE).await?;
    Ok(profile.background_skin_id)
}

pub async fn set_profile_background(session: &LcuSession, skin_id: i64) -> Result<()> {
    let body = json!({ "key": "backgroundSkinId", "value": skin_id });
    let http = &session.http;
    http.send_json(Method::POST, PROFILE, Some(&body)).await
}

/// Restarts the client's window process. The game keeps running, and the connection
/// to the client comes back by itself.
pub async fn restart_client(session: &LcuSession) -> Result<()> {
    session.http.post_empty(RESTART_UX).await
}

/// The id skins are listed under.
pub fn own_summoner_id(state: &AppState) -> Result<u64> {
    let summoner = state.lcu_snapshot().summoner;
    let id = summoner.map(|s| s.summoner_id);
    id.ok_or(AppError::NotConnected)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_skins_and_ownership_from_the_client_payload() {
        let json = r#"[
            {"id": 1000, "name": "Garen", "tilePath": "/a.jpg", "ownership": {"owned": true}},
            {"id": 1001, "name": "Sanguine", "ownership": {"owned": false}},
            {"id": 0, "name": "none"}
        ]"#;
        let skins: Vec<LcuSkin> = serde_json::from_str(json).unwrap();
        assert_eq!(skins.len(), 3);
        assert!(skins[0].ownership.owned);
        assert_eq!(skins[0].tile_path, "/a.jpg");
        assert!(!skins[1].ownership.owned);
        assert!(skins[1].tile_path.is_empty());
    }

    #[test]
    fn reads_the_chat_status_and_ignores_the_rest() {
        let json = r#"{"availability": "offline", "statusMessage": "hi", "puuid": "x"}"#;
        let status: ChatStatus = serde_json::from_str(json).unwrap();
        assert_eq!(status.availability, "offline");
        assert_eq!(status.status_message, "hi");
    }
}
