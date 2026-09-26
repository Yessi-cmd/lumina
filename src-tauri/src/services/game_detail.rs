//! Every player of one finished game, for the expandable rows of the match history:
//! per-team objectives and bans, full per-player stats and runes (SGP first, LCU as
//! fallback), and item/skill orders read from the SGP timeline. Finished games never
//! change, so everything is cached.

use std::collections::{HashMap, VecDeque};
use std::sync::{Mutex, PoisonError};

use serde::Serialize;

use super::match_history::{entitlements_token, DataSource};
use crate::clients::lcu::models::{LcuGame, LcuParticipantStats, LcuTeam};
use crate::clients::sgp::models::{SgpGameJson, SgpParticipant, SgpTeam, SgpTimeline};
use crate::error::{AppError, Result};
use crate::state::session::LcuSession;

const CACHE_CAPACITY: usize = 100;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GameDetail {
    pub game_id: i64,
    pub queue_id: i64,
    pub game_mode: String,
    pub created_at: i64,
    /// Seconds.
    pub duration: i64,
    pub source: DataSource,
    /// Blue side (100) first.
    pub teams: Vec<TeamDetail>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TeamDetail {
    pub team_id: i64,
    pub win: bool,
    pub kills: i64,
    pub gold: i64,
    pub bans: Vec<i64>,
    pub objectives: Objectives,
    pub players: Vec<PlayerLine>,
}

#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Objectives {
    pub baron: i64,
    pub dragon: i64,
    pub herald: i64,
    pub grubs: i64,
    pub atakhan: i64,
    pub tower: i64,
    pub inhibitor: i64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlayerLine {
    pub puuid: String,
    pub game_name: String,
    pub tag_line: String,
    pub champion_id: i64,
    pub champ_level: i64,
    pub kills: i64,
    pub deaths: i64,
    pub assists: i64,
    pub spells: [i64; 2],
    pub items: [i64; 7],
    pub runes: GameRunes,
    pub cs: i64,
    pub gold: i64,
    pub damage_to_champions: i64,
    pub physical_damage: i64,
    pub magic_damage: i64,
    pub true_damage: i64,
    pub damage_taken: i64,
    pub damage_mitigated: i64,
    pub healing: i64,
    pub shielding: i64,
    pub building_damage: i64,
    pub vision_score: i64,
    pub wards_placed: i64,
    pub wards_killed: i64,
    pub control_wards: i64,
    /// Seconds of crowd control applied to enemies.
    pub cc_seconds: i64,
    /// Double, triple, quadra and penta kills.
    pub multi_kills: [i64; 4],
    pub first_blood: bool,
    pub position: String,
}

#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GameRunes {
    pub primary_style: i64,
    pub sub_style: i64,
    /// Four primary runes (keystone first), then two secondary runes.
    pub perks: Vec<i64>,
    /// Offense, flex and defense stat shards.
    pub shards: Vec<i64>,
}

/// One player's purchases and skill order, from the timeline.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlayerBuild {
    pub puuid: String,
    pub items: Vec<ItemPurchase>,
    /// Skill slot per level: 1 = Q, 2 = W, 3 = E, 4 = R.
    pub skills: Vec<i64>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ItemPurchase {
    pub item_id: i64,
    /// Seconds since the start of the game.
    pub at: i64,
}

type Cache<T> = Mutex<(HashMap<i64, T>, VecDeque<i64>)>;

#[derive(Default)]
pub struct GameDetailService {
    details: Cache<GameDetail>,
    builds: Cache<Vec<PlayerBuild>>,
}

impl GameDetailService {
    pub async fn get(&self, session: &LcuSession, game_id: i64) -> Result<GameDetail> {
        if let Some(detail) = cached(&self.details, game_id) {
            return Ok(detail);
        }
        let detail = fetch(session, game_id).await?;
        store(&self.details, game_id, detail.clone());
        Ok(detail)
    }

    /// Needs SGP: LCU does not serve timelines for other players' games reliably.
    pub async fn builds(&self, session: &LcuSession, game_id: i64) -> Result<Vec<PlayerBuild>> {
        if let Some(builds) = cached(&self.builds, game_id) {
            return Ok(builds);
        }
        let Some(sgp) = &session.sgp else {
            return Err(AppError::Message("出装顺序需要 SGP 数据".to_owned()));
        };
        let token = entitlements_token(session).await?;
        let details = sgp.game_details(&token, game_id).await?;
        let builds = builds_from(&details.json);
        store(&self.builds, game_id, builds.clone());
        Ok(builds)
    }
}

fn cached<T: Clone>(cache: &Cache<T>, game_id: i64) -> Option<T> {
    let cache = cache.lock().unwrap_or_else(PoisonError::into_inner);
    cache.0.get(&game_id).cloned()
}

fn store<T>(cache: &Cache<T>, game_id: i64, value: T) {
    let mut cache = cache.lock().unwrap_or_else(PoisonError::into_inner);
    let (values, order) = &mut *cache;
    if values.insert(game_id, value).is_none() {
        order.push_back(game_id);
    }
    while order.len() > CACHE_CAPACITY {
        if let Some(oldest) = order.pop_front() {
            values.remove(&oldest);
        }
    }
}

async fn fetch(session: &LcuSession, game_id: i64) -> Result<GameDetail> {
    if let Some(sgp) = &session.sgp {
        let token = entitlements_token(session).await?;
        match sgp.game_summary(&token, game_id).await {
            Ok(game) => return Ok(from_sgp(game.json)),
            Err(err) => log::warn!("SGP game {game_id} failed, falling back to LCU: {err}"),
        }
    }
    let path = format!("/lol-match-history/v1/games/{game_id}");
    let game: LcuGame = session.http.get(&path).await?;
    Ok(from_lcu(game))
}

fn from_sgp(game: SgpGameJson) -> GameDetail {
    let mut players: Vec<(i64, bool, PlayerLine)> = Vec::new();
    for p in &game.participants {
        players.push((p.team_id, p.win, sgp_line(p)));
    }
    let mut teams = group_teams(players);
    for team in &mut teams {
        if let Some(info) = game.teams.iter().find(|t| t.team_id == team.team_id) {
            apply_sgp_team(team, info);
        }
    }
    GameDetail {
        game_id: game.game_id,
        queue_id: game.queue_id,
        game_mode: game.game_mode,
        created_at: game.game_creation,
        duration: game.game_duration,
        source: DataSource::Sgp,
        teams,
    }
}

fn apply_sgp_team(team: &mut TeamDetail, info: &SgpTeam) {
    team.bans = info.bans.iter().map(|b| b.champion_id).collect();
    let o = &info.objectives;
    team.objectives = Objectives {
        baron: o.baron.kills,
        dragon: o.dragon.kills,
        herald: o.rift_herald.kills,
        grubs: o.horde.kills,
        atakhan: o.atakhan.kills,
        tower: o.tower.kills,
        inhibitor: o.inhibitor.kills,
    };
}

fn sgp_line(p: &SgpParticipant) -> PlayerLine {
    let mut perks = Vec::new();
    for style in &p.perks.styles {
        perks.extend(style.selections.iter().map(|s| s.perk));
    }
    let style = |i: usize| p.perks.styles.get(i).map_or(0, |s| s.style);
    let shards = &p.perks.stat_perks;
    PlayerLine {
        puuid: p.puuid.clone(),
        game_name: p.riot_id_game_name.clone(),
        tag_line: p.riot_id_tagline.clone(),
        champion_id: p.champion_id,
        champ_level: p.champ_level,
        kills: p.kills,
        deaths: p.deaths,
        assists: p.assists,
        spells: [p.spell1_id, p.spell2_id],
        items: [
            p.item0, p.item1, p.item2, p.item3, p.item4, p.item5, p.item6,
        ],
        runes: GameRunes {
            primary_style: style(0),
            sub_style: style(1),
            perks,
            shards: vec![shards.offense, shards.flex, shards.defense],
        },
        cs: p.total_minions_killed + p.neutral_minions_killed,
        gold: p.gold_earned,
        damage_to_champions: p.total_damage_dealt_to_champions,
        physical_damage: p.physical_damage_dealt_to_champions,
        magic_damage: p.magic_damage_dealt_to_champions,
        true_damage: p.true_damage_dealt_to_champions,
        damage_taken: p.total_damage_taken,
        damage_mitigated: p.damage_self_mitigated,
        healing: p.total_heals_on_teammates,
        shielding: p.total_damage_shielded_on_teammates,
        building_damage: p.damage_dealt_to_buildings,
        vision_score: p.vision_score,
        wards_placed: p.wards_placed,
        wards_killed: p.wards_killed,
        control_wards: p.vision_wards_bought_in_game,
        cc_seconds: p.time_c_cing_others,
        multi_kills: [
            p.double_kills,
            p.triple_kills,
            p.quadra_kills,
            p.penta_kills,
        ],
        first_blood: p.first_blood_kill,
        position: p.team_position.clone(),
    }
}

fn from_lcu(game: LcuGame) -> GameDetail {
    let mut identities = HashMap::new();
    for identity in &game.participant_identities {
        identities.insert(identity.participant_id, &identity.player);
    }
    let mut players: Vec<(i64, bool, PlayerLine)> = Vec::new();
    for p in &game.participants {
        let player = identities.get(&p.participant_id);
        let mut line = lcu_line(&p.stats);
        line.puuid = player.map(|x| x.puuid.clone()).unwrap_or_default();
        line.game_name = player.map(|x| x.game_name.clone()).unwrap_or_default();
        line.tag_line = player.map(|x| x.tag_line.clone()).unwrap_or_default();
        line.champion_id = p.champion_id;
        line.spells = [p.spell1_id, p.spell2_id];
        players.push((p.team_id, p.stats.win, line));
    }
    let mut teams = group_teams(players);
    for team in &mut teams {
        if let Some(info) = game.teams.iter().find(|t| t.team_id == team.team_id) {
            apply_lcu_team(team, info);
        }
    }
    GameDetail {
        game_id: game.game_id,
        queue_id: game.queue_id,
        game_mode: game.game_mode.clone(),
        created_at: game.game_creation,
        duration: game.game_duration,
        source: DataSource::Lcu,
        teams,
    }
}

fn apply_lcu_team(team: &mut TeamDetail, info: &LcuTeam) {
    team.bans = info.bans.iter().map(|b| b.champion_id).collect();
    team.objectives = Objectives {
        baron: info.baron_kills,
        dragon: info.dragon_kills,
        herald: info.rift_herald_kills,
        grubs: info.horde_kills,
        atakhan: 0,
        tower: info.tower_kills,
        inhibitor: info.inhibitor_kills,
    };
}

/// Stats of one LCU participant; identity, champion and spells are filled in by the caller.
fn lcu_line(s: &LcuParticipantStats) -> PlayerLine {
    let perks = vec![s.perk0, s.perk1, s.perk2, s.perk3, s.perk4, s.perk5];
    PlayerLine {
        puuid: String::new(),
        game_name: String::new(),
        tag_line: String::new(),
        champion_id: 0,
        champ_level: s.champ_level,
        kills: s.kills,
        deaths: s.deaths,
        assists: s.assists,
        spells: [0, 0],
        items: [
            s.item0, s.item1, s.item2, s.item3, s.item4, s.item5, s.item6,
        ],
        runes: GameRunes {
            primary_style: s.perk_primary_style,
            sub_style: s.perk_sub_style,
            perks,
            shards: vec![s.stat_perk0, s.stat_perk1, s.stat_perk2],
        },
        cs: s.total_minions_killed + s.neutral_minions_killed,
        gold: s.gold_earned,
        damage_to_champions: s.total_damage_dealt_to_champions,
        physical_damage: s.physical_damage_dealt_to_champions,
        magic_damage: s.magic_damage_dealt_to_champions,
        true_damage: s.true_damage_dealt_to_champions,
        damage_taken: s.total_damage_taken,
        damage_mitigated: s.damage_self_mitigated,
        healing: s.total_heal,
        shielding: 0,
        building_damage: s.damage_dealt_to_turrets,
        vision_score: s.vision_score,
        wards_placed: s.wards_placed,
        wards_killed: s.wards_killed,
        control_wards: s.vision_wards_bought_in_game,
        cc_seconds: s.time_c_cing_others,
        multi_kills: [
            s.double_kills,
            s.triple_kills,
            s.quadra_kills,
            s.penta_kills,
        ],
        first_blood: s.first_blood_kill,
        position: String::new(),
    }
}

fn group_teams(players: Vec<(i64, bool, PlayerLine)>) -> Vec<TeamDetail> {
    let mut teams: Vec<TeamDetail> = Vec::new();
    for (team_id, win, line) in players {
        let index = match teams.iter().position(|t| t.team_id == team_id) {
            Some(index) => index,
            None => {
                teams.push(TeamDetail {
                    team_id,
                    win,
                    kills: 0,
                    gold: 0,
                    bans: Vec::new(),
                    objectives: Objectives::default(),
                    players: Vec::new(),
                });
                teams.len() - 1
            }
        };
        let team = &mut teams[index];
        team.kills += line.kills;
        team.gold += line.gold;
        team.players.push(line);
    }
    teams.sort_by_key(|t| t.team_id);
    teams
}

/// Purchases (with undos taken back) and skill level-ups per player, in game order.
fn builds_from(timeline: &SgpTimeline) -> Vec<PlayerBuild> {
    let mut builds: Vec<PlayerBuild> = Vec::new();
    let mut index: HashMap<i64, usize> = HashMap::new();
    for p in &timeline.participants {
        index.insert(p.participant_id, builds.len());
        builds.push(PlayerBuild {
            puuid: p.puuid.clone(),
            items: Vec::new(),
            skills: Vec::new(),
        });
    }

    for event in timeline.frames.iter().flat_map(|f| &f.events) {
        let Some(&i) = index.get(&event.participant_id) else {
            continue;
        };
        let build = &mut builds[i];
        match event.kind.as_str() {
            "ITEM_PURCHASED" => build.items.push(ItemPurchase {
                item_id: event.item_id,
                at: event.timestamp / 1000,
            }),
            "ITEM_UNDO" => {
                let items = &build.items;
                let undone = items.iter().rposition(|p| p.item_id == event.before_id);
                if let Some(pos) = undone {
                    build.items.remove(pos);
                }
            }
            "SKILL_LEVEL_UP" if event.level_up_type != "EVOLVE" => {
                build.skills.push(event.skill_slot);
            }
            _ => {}
        }
    }
    builds
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::clients::sgp::models::SgpGame;

    #[test]
    fn groups_sgp_players_with_team_info() {
        let json = r#"{"json":{"gameId":5,"queueId":420,"gameDuration":1500,
            "teams":[{"teamId":100,"win":true,"bans":[{"championId":157}],
              "objectives":{"dragon":{"kills":3},"horde":{"kills":6}}}],
            "participants":[
            {"puuid":"r1","teamId":200,"win":false,"kills":2,"goldEarned":900},
            {"puuid":"b1","teamId":100,"win":true,"kills":7,"goldEarned":1000,
             "riotIdGameName":"小明","riotIdTagline":"123","doubleKills":2,
             "perks":{"statPerks":{"offense":5008,"flex":5008,"defense":5001},
               "styles":[{"style":8000,"selections":[{"perk":8010},{"perk":9111}]},
                         {"style":8400,"selections":[{"perk":8444}]}]}},
            {"puuid":"b2","teamId":100,"win":true,"kills":3,"goldEarned":500}]}}"#;
        let game: SgpGame = serde_json::from_str(json).unwrap();
        let detail = from_sgp(game.json);
        assert_eq!(detail.teams.len(), 2);
        let blue = &detail.teams[0];
        assert_eq!((blue.team_id, blue.win), (100, true));
        assert_eq!((blue.kills, blue.gold), (10, 1500));
        assert_eq!(blue.bans, vec![157]);
        assert_eq!((blue.objectives.dragon, blue.objectives.grubs), (3, 6));
        let b1 = &blue.players[0];
        assert_eq!(b1.game_name, "小明");
        assert_eq!(b1.multi_kills[0], 2);
        assert_eq!((b1.runes.primary_style, b1.runes.sub_style), (8000, 8400));
        assert_eq!(b1.runes.perks, vec![8010, 9111, 8444]);
        assert_eq!(detail.teams[1].players[0].puuid, "r1");
    }

    #[test]
    fn reads_builds_and_takes_back_undos() {
        let json = r#"{"participants":[{"participantId":1,"puuid":"me"}],
            "frames":[{"timestamp":0,"events":[
              {"type":"ITEM_PURCHASED","timestamp":1000,"participantId":1,"itemId":1055},
              {"type":"ITEM_PURCHASED","timestamp":2000,"participantId":1,"itemId":2003},
              {"type":"ITEM_UNDO","timestamp":3000,"participantId":1,"beforeId":2003},
              {"type":"SKILL_LEVEL_UP","timestamp":4000,"participantId":1,"skillSlot":1,
               "levelUpType":"NORMAL"},
              {"type":"SKILL_LEVEL_UP","timestamp":5000,"participantId":1,"skillSlot":4,
               "levelUpType":"EVOLVE"}]}]}"#;
        let timeline: SgpTimeline = serde_json::from_str(json).unwrap();
        let builds = builds_from(&timeline);
        assert_eq!(builds[0].items.len(), 1);
        assert_eq!(builds[0].items[0].item_id, 1055);
        assert_eq!(builds[0].skills, vec![1]);
    }
}
