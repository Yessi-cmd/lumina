use serde::{Deserialize, Serialize};
use serde_json::Value;

/// `/lol-summoner/v1/current-summoner`, trimmed to what the UI needs.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Summoner {
    #[serde(default)]
    pub puuid: String,
    #[serde(default)]
    pub summoner_id: u64,
    #[serde(default)]
    pub game_name: String,
    #[serde(default)]
    pub tag_line: String,
    #[serde(default)]
    pub display_name: String,
    #[serde(default)]
    pub summoner_level: u32,
    #[serde(default)]
    pub profile_icon_id: u32,
}

/// Payload of a WAMP `[8, "OnJsonApiEvent", {...}]` message.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LcuEvent {
    pub uri: String,
    pub event_type: LcuEventType,
    #[serde(default)]
    pub data: Value,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
pub enum LcuEventType {
    Create,
    Update,
    Delete,
}
