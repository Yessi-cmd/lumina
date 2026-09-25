//! Every player of one finished game, for the expandable rows of the match history.
//! SGP first, LCU as fallback; finished games never change, so results are cached.

use std::collections::{HashMap, VecDeque};
use std::sync::{Mutex, PoisonError};

use serde::Serialize;

use super::match_history::{entitlements_token, DataSource};
use crate::clients::lcu::models::LcuGame;
use crate::clients::sgp::models::{SgpGameJson, SgpParticipant};
use crate::error::Result;
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
    pub players: Vec<PlayerLine>,
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
    pub cs: i64,
    pub gold: i64,
    pub damage_to_champions: i64,
    pub damage_taken: i64,
    pub vision_score: i64,
    pub position: String,
}

#[derive(Default)]
pub struct GameDetailService {
    cache: Mutex<(HashMap<i64, GameDetail>, VecDeque<i64>)>,
}

impl GameDetailService {
    pub async fn get(&self, session: &LcuSession, game_id: i64) -> Result<GameDetail> {
        if let Some(detail) = self.cached(game_id) {
            return Ok(detail);
        }
        let detail = fetch(session, game_id).await?;
        self.store(detail.clone());
        Ok(detail)
    }

    fn cached(&self, game_id: i64) -> Option<GameDetail> {
        let cache = self.cache.lock().unwrap_or_else(PoisonError::into_inner);
        cache.0.get(&game_id).cloned()
    }

    fn store(&self, detail: GameDetail) {
        let mut cache = self.cache.lock().unwrap_or_else(PoisonError::into_inner);
        let (details, order) = &mut *cache;
        let id = detail.game_id;
        if details.insert(id, detail).is_none() {
            order.push_back(id);
        }
        while order.len() > CACHE_CAPACITY {
            if let Some(oldest) = order.pop_front() {
                details.remove(&oldest);
            }
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
    GameDetail {
        game_id: game.game_id,
        queue_id: game.queue_id,
        game_mode: game.game_mode,
        created_at: game.game_creation,
        duration: game.game_duration,
        source: DataSource::Sgp,
        teams: group_teams(players),
    }
}

fn sgp_line(p: &SgpParticipant) -> PlayerLine {
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
        cs: p.total_minions_killed + p.neutral_minions_killed,
        gold: p.gold_earned,
        damage_to_champions: p.total_damage_dealt_to_champions,
        damage_taken: p.total_damage_taken,
        vision_score: p.vision_score,
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
        let s = &p.stats;
        let player = identities.get(&p.participant_id);
        let line = PlayerLine {
            puuid: player.map(|x| x.puuid.clone()).unwrap_or_default(),
            game_name: player.map(|x| x.game_name.clone()).unwrap_or_default(),
            tag_line: player.map(|x| x.tag_line.clone()).unwrap_or_default(),
            champion_id: p.champion_id,
            champ_level: s.champ_level,
            kills: s.kills,
            deaths: s.deaths,
            assists: s.assists,
            spells: [p.spell1_id, p.spell2_id],
            items: [
                s.item0, s.item1, s.item2, s.item3, s.item4, s.item5, s.item6,
            ],
            cs: s.total_minions_killed + s.neutral_minions_killed,
            gold: s.gold_earned,
            damage_to_champions: s.total_damage_dealt_to_champions,
            damage_taken: s.total_damage_taken,
            vision_score: s.vision_score,
            position: String::new(),
        };
        players.push((p.team_id, s.win, line));
    }
    GameDetail {
        game_id: game.game_id,
        queue_id: game.queue_id,
        game_mode: game.game_mode.clone(),
        created_at: game.game_creation,
        duration: game.game_duration,
        source: DataSource::Lcu,
        teams: group_teams(players),
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::clients::sgp::models::SgpGame;

    #[test]
    fn groups_sgp_players_by_team() {
        let json = r#"{"json":{"gameId":5,"queueId":420,"gameDuration":1500,"participants":[
            {"puuid":"r1","teamId":200,"win":false,"kills":2,"goldEarned":900},
            {"puuid":"b1","teamId":100,"win":true,"kills":7,"goldEarned":1000,
             "riotIdGameName":"小明","riotIdTagline":"123"},
            {"puuid":"b2","teamId":100,"win":true,"kills":3,"goldEarned":500}]}}"#;
        let game: SgpGame = serde_json::from_str(json).unwrap();
        let detail = from_sgp(game.json);
        assert_eq!(detail.teams.len(), 2);
        let blue = &detail.teams[0];
        assert_eq!((blue.team_id, blue.win), (100, true));
        assert_eq!((blue.kills, blue.gold), (10, 1500));
        assert_eq!(blue.players[0].game_name, "小明");
        assert_eq!(detail.teams[1].players[0].puuid, "r1");
    }
}
