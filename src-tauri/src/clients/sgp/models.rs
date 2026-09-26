//! Subset of `match-history-query` SUMMARY payloads (match-v5 shaped).
//! Every field defaults so schema drift degrades to zeros instead of failing a page.

use std::collections::HashMap;

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
    pub teams: Vec<SgpTeam>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct SgpTeam {
    pub team_id: i64,
    pub win: bool,
    pub bans: Vec<SgpBan>,
    pub objectives: SgpObjectives,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct SgpBan {
    pub champion_id: i64,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct SgpObjectives {
    pub baron: SgpObjective,
    pub dragon: SgpObjective,
    pub rift_herald: SgpObjective,
    /// Void grubs.
    pub horde: SgpObjective,
    pub atakhan: SgpObjective,
    pub tower: SgpObjective,
    pub inhibitor: SgpObjective,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub struct SgpObjective {
    pub kills: i64,
}

/// match-v5 `perks`: two rune trees and three stat shards.
#[derive(Debug, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct SgpPerks {
    pub stat_perks: SgpStatPerks,
    /// Primary tree first, then the secondary tree.
    pub styles: Vec<SgpPerkStyle>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub struct SgpStatPerks {
    pub offense: i64,
    pub flex: i64,
    pub defense: i64,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub struct SgpPerkStyle {
    pub style: i64,
    pub selections: Vec<SgpPerkSelection>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub struct SgpPerkSelection {
    pub perk: i64,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct SgpParticipant {
    pub puuid: String,
    pub riot_id_game_name: String,
    pub riot_id_tagline: String,
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
    pub perks: SgpPerks,
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
    pub total_heals_on_teammates: i64,
    pub total_damage_shielded_on_teammates: i64,
    pub damage_dealt_to_buildings: i64,
    pub wards_placed: i64,
    pub wards_killed: i64,
    pub vision_wards_bought_in_game: i64,
    pub time_c_cing_others: i64,
}

/// match-v5 `challenges`; values may be fractional, so they are read as floats.
#[derive(Debug, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct SgpChallenges {
    pub solo_kills: f64,
}

/// `match-history-query` DETAILS: the match-v5 timeline, trimmed to what lane analysis needs.
#[derive(Debug, Deserialize)]
pub struct SgpGameDetails {
    pub json: SgpTimeline,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub struct SgpTimeline {
    pub frames: Vec<SgpFrame>,
    pub participants: Vec<SgpTimelineParticipant>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct SgpTimelineParticipant {
    pub participant_id: i64,
    pub puuid: String,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct SgpFrame {
    /// Milliseconds since game start.
    pub timestamp: i64,
    pub events: Vec<SgpEvent>,
    /// Keyed by participant id as a string.
    pub participant_frames: HashMap<String, SgpParticipantFrame>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct SgpEvent {
    #[serde(rename = "type")]
    pub kind: String,
    pub timestamp: i64,
    pub killer_id: i64,
    pub victim_id: i64,
    pub assisting_participant_ids: Vec<i64>,
    /// Buyer, seller or levelling player of item and skill events.
    pub participant_id: i64,
    pub item_id: i64,
    /// Item restored by an `ITEM_UNDO`.
    pub before_id: i64,
    /// 1 = Q, 2 = W, 3 = E, 4 = R.
    pub skill_slot: i64,
    /// `NORMAL`, or `EVOLVE` for evolutions that do not cost a point.
    pub level_up_type: String,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct SgpParticipantFrame {
    pub total_gold: i64,
    pub xp: i64,
    pub minions_killed: i64,
    pub jungle_minions_killed: i64,
}
