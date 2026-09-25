//! Subset of `match-history-query` SUMMARY payloads (match-v5 shaped).
//! Every field defaults so schema drift degrades to zeros instead of failing a page.

use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct SgpMatchHistory {
    #[serde(default)]
    pub games: Vec<SgpGame>,
}

#[derive(Debug, Deserialize)]
pub struct SgpGame {
    pub json: SgpGameJson,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct SgpGameJson {
    pub game_id: i64,
    pub queue_id: i64,
    pub game_mode: String,
    /// Unix milliseconds.
    pub game_creation: i64,
    /// Seconds.
    pub game_duration: i64,
    pub end_of_game_result: String,
    pub participants: Vec<SgpParticipant>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct SgpParticipant {
    pub puuid: String,
    pub team_id: i64,
    pub champion_id: i64,
    pub champ_level: i64,
    pub kills: i64,
    pub deaths: i64,
    pub assists: i64,
    pub win: bool,
    pub game_ended_in_early_surrender: bool,
    pub team_early_surrendered: bool,
    pub spell1_id: i64,
    pub spell2_id: i64,
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
    pub total_heal: i64,
    pub vision_score: i64,
    pub enemy_missing_pings: i64,
    pub team_position: String,
    pub challenges: SgpChallenges,
}

/// match-v5 `challenges`; values may be fractional, so they are read as floats.
#[derive(Debug, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct SgpChallenges {
    pub solo_kills: f64,
}
