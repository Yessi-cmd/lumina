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

/// `/entitlements/v1/token`; the access token authenticates SGP match history.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EntitlementsToken {
    pub access_token: String,
}

/// `/riotclient/region-locale`; `region` is `TENCENT` on the Chinese servers.
#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub struct RegionLocale {
    pub region: String,
}

/// `/lol-match-history/v1/products/lol/{puuid}/matches`
#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub struct LcuMatchHistory {
    pub games: LcuGames,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub struct LcuGames {
    pub games: Vec<LcuGame>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct LcuGame {
    pub game_id: i64,
    pub queue_id: i64,
    pub game_mode: String,
    pub game_creation: i64,
    pub game_duration: i64,
    pub end_of_game_result: String,
    pub participants: Vec<LcuParticipant>,
    pub participant_identities: Vec<LcuParticipantIdentity>,
    pub teams: Vec<LcuTeam>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct LcuTeam {
    pub team_id: i64,
    pub bans: Vec<LcuBan>,
    pub baron_kills: i64,
    pub dragon_kills: i64,
    pub rift_herald_kills: i64,
    pub horde_kills: i64,
    pub tower_kills: i64,
    pub inhibitor_kills: i64,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct LcuBan {
    pub champion_id: i64,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct LcuParticipant {
    pub participant_id: i64,
    pub team_id: i64,
    pub champion_id: i64,
    pub spell1_id: i64,
    pub spell2_id: i64,
    pub stats: LcuParticipantStats,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct LcuParticipantStats {
    pub champ_level: i64,
    pub kills: i64,
    pub deaths: i64,
    pub assists: i64,
    pub win: bool,
    pub game_ended_in_early_surrender: bool,
    pub team_early_surrendered: bool,
    pub item0: i64,
    pub item1: i64,
    pub item2: i64,
    pub item3: i64,
    pub item4: i64,
    pub item5: i64,
    pub item6: i64,
    pub total_minions_killed: i64,
    pub neutral_minions_killed: i64,
    pub gold_earned: i64,
    pub total_damage_dealt_to_champions: i64,
    pub total_damage_taken: i64,
    pub vision_score: i64,
    pub perk_primary_style: i64,
    pub perk_sub_style: i64,
    pub perk0: i64,
    pub perk1: i64,
    pub perk2: i64,
    pub perk3: i64,
    pub perk4: i64,
    pub perk5: i64,
    pub stat_perk0: i64,
    pub stat_perk1: i64,
    pub stat_perk2: i64,
    pub largest_multi_kill: i64,
    pub double_kills: i64,
    pub triple_kills: i64,
    pub quadra_kills: i64,
    pub penta_kills: i64,
    pub first_blood_kill: bool,
    pub physical_damage_dealt_to_champions: i64,
    pub magic_damage_dealt_to_champions: i64,
    pub true_damage_dealt_to_champions: i64,
    pub damage_self_mitigated: i64,
    pub total_heal: i64,
    pub damage_dealt_to_turrets: i64,
    pub wards_placed: i64,
    pub wards_killed: i64,
    pub vision_wards_bought_in_game: i64,
    pub time_c_cing_others: i64,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct LcuParticipantIdentity {
    pub participant_id: i64,
    pub player: LcuPlayer,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct LcuPlayer {
    pub puuid: String,
    pub game_name: String,
    pub tag_line: String,
}

/// `/lol-game-data/assets/v1/champion-summary.json`
#[derive(Debug, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct LcuChampion {
    pub id: i64,
    pub name: String,
    /// English id such as `Garen` or `MonkeyKing`.
    pub alias: String,
    pub square_portrait_path: String,
}

/// Entries of `items.json` and `summoner-spells.json`.
#[derive(Debug, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct LcuIconAsset {
    pub id: i64,
    pub name: String,
    pub icon_path: String,
}

/// `/lol-game-data/assets/v1/perkstyles.json`
#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub struct LcuPerkStyles {
    pub styles: Vec<LcuIconAsset>,
}

/// `/lol-perks/v1/pages` entries.
#[derive(Debug, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct LcuPerkPage {
    pub id: i64,
    pub name: String,
    pub is_deletable: bool,
    pub is_editable: bool,
}

/// `/lol-champions/v1/owned-champions-minimal` entries.
#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub struct LcuOwnedChampion {
    pub id: i64,
}

/// `/lol-game-queues/v1/queues`
#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub struct LcuQueue {
    pub id: i64,
    pub name: String,
    pub description: String,
}

/// `/lol-champ-select/v1/session`
#[derive(Debug, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct ChampSelectSession {
    pub game_id: i64,
    pub local_player_cell_id: i64,
    pub my_team: Vec<ChampSelectMember>,
    pub their_team: Vec<ChampSelectMember>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct ChampSelectMember {
    pub cell_id: i64,
    pub puuid: String,
    pub champion_id: i64,
    pub champion_pick_intent: i64,
    pub assigned_position: String,
    pub spell1_id: i64,
    pub spell2_id: i64,
    /// `HIDDEN` for anonymized players (every opponent on the Chinese servers).
    pub name_visibility_type: String,
}

/// `/lol-gameflow/v1/session`
#[derive(Debug, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct GameflowSession {
    pub phase: String,
    pub game_data: GameflowGameData,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct GameflowGameData {
    pub game_id: i64,
    pub queue: GameflowQueue,
    pub team_one: Vec<GameflowPlayer>,
    pub team_two: Vec<GameflowPlayer>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub struct GameflowQueue {
    pub id: i64,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct GameflowPlayer {
    pub puuid: String,
    pub champion_id: i64,
    pub selected_position: String,
}

/// `/lol-lobby/v2/lobby`
#[derive(Debug, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct LobbySession {
    pub game_config: LobbyGameConfig,
    pub members: Vec<LobbyMember>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct LobbyGameConfig {
    pub queue_id: i64,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct LobbyMember {
    pub puuid: String,
    /// `TOP`/`JUNGLE`/...; `UNSELECTED` or `FILL` when none.
    pub first_position_preference: String,
}
