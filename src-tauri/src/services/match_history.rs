//! Match history for any player on the current server: SGP first, LCU as fallback.
//! Requests are capped at 5 in flight and pages are cached for 5 minutes, so the
//! champ-select panel (M4) can ask for 5–10 players at once.

use std::collections::HashMap;
use std::sync::{Mutex, PoisonError};
use std::time::{Duration, Instant};

use serde::Serialize;
use tokio::sync::Semaphore;

use crate::clients::lcu::models::{EntitlementsToken, LcuGame, LcuMatchHistory};
use crate::clients::sgp::http::SgpClient;
use crate::clients::sgp::models::{SgpGameJson, SgpParticipant};
use crate::error::Result;
use crate::state::session::LcuSession;

const MAX_CONCURRENT: usize = 5;
const CACHE_TTL: Duration = Duration::from_secs(5 * 60);
const CACHE_CAPACITY: usize = 256;
const MAX_PAGE_SIZE: u32 = 50;
const ENTITLEMENTS_TOKEN: &str = "/entitlements/v1/token";
const SMITE: i64 = 11;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum DataSource {
    Sgp,
    Lcu,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum GameResult {
    Win,
    Loss,
    Remake,
    Abort,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MatchHistoryPage {
    pub puuid: String,
    pub start: u32,
    pub count: u32,
    pub source: DataSource,
    /// Why SGP was not used when the page came from LCU.
    pub sgp_error: Option<String>,
    pub games: Vec<GameSummary>,
}

/// One game from the queried player's point of view.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GameSummary {
    pub game_id: i64,
    pub queue_id: i64,
    pub game_mode: String,
    /// Unix milliseconds.
    pub created_at: i64,
    /// Seconds.
    pub duration: i64,
    pub result: GameResult,
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
    /// `TOP`/`JUNGLE`/...; empty when the source does not say.
    pub position: String,
    pub team_id: i64,
    pub vision_score: i64,
    /// Team-relative figures. Only SGP lists every participant, so LCU pages have none.
    pub metrics: Option<GameMetrics>,
    /// Everyone in the game (SGP only), for premade and "met before" detection.
    pub participants: Vec<GameParticipant>,
}

/// Shares are fractions of the player's team total (0.25 = 25%).
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GameMetrics {
    pub team_size: usize,
    pub damage_share: f64,
    pub damage_taken_share: f64,
    pub gold_share: f64,
    pub cs_share: f64,
    pub vision_share: f64,
    pub kill_share: f64,
    pub kill_participation: f64,
    /// Own healing relative to the team's average damage taken (League Akari's definition).
    pub heal_ratio: f64,
    pub solo_kills: f64,
    pub enemy_missing_pings: i64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GameParticipant {
    pub puuid: String,
    pub team_id: i64,
    /// `TOP`/`JUNGLE`/...; empty outside Summoner's Rift.
    pub position: String,
    /// Plays jungle (by position, or by carrying Smite).
    pub jungler: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct PageRequest {
    puuid: String,
    start: u32,
    count: u32,
}

impl PageRequest {
    fn page(
        &self,
        source: DataSource,
        sgp_error: Option<String>,
        games: Vec<GameSummary>,
    ) -> MatchHistoryPage {
        MatchHistoryPage {
            puuid: self.puuid.clone(),
            start: self.start,
            count: self.count,
            source,
            sgp_error,
            games,
        }
    }
}

pub struct MatchHistoryService {
    limiter: Semaphore,
    cache: Mutex<HashMap<PageRequest, (Instant, MatchHistoryPage)>>,
}

impl Default for MatchHistoryService {
    fn default() -> Self {
        Self {
            limiter: Semaphore::new(MAX_CONCURRENT),
            cache: Mutex::default(),
        }
    }
}

impl MatchHistoryService {
    pub async fn get(
        &self,
        session: &LcuSession,
        puuid: &str,
        start: u32,
        count: u32,
    ) -> Result<MatchHistoryPage> {
        let req = PageRequest {
            puuid: puuid.to_owned(),
            start,
            count: count.clamp(1, MAX_PAGE_SIZE),
        };
        if let Some(page) = self.cached(&req) {
            return Ok(page);
        }

        let permit = self.limiter.acquire().await;
        let _permit = permit.expect("semaphore is never closed");
        // Another caller may have fetched the same page while we waited.
        if let Some(page) = self.cached(&req) {
            return Ok(page);
        }

        let page = fetch(session, &req).await?;
        self.store(req, page.clone());
        Ok(page)
    }

    fn cached(&self, req: &PageRequest) -> Option<MatchHistoryPage> {
        let cache = self.cache.lock().unwrap_or_else(PoisonError::into_inner);
        let (at, page) = cache.get(req)?;
        (at.elapsed() < CACHE_TTL).then(|| page.clone())
    }

    fn store(&self, req: PageRequest, page: MatchHistoryPage) {
        let mut cache = self.cache.lock().unwrap_or_else(PoisonError::into_inner);
        cache.retain(|_, (at, _)| at.elapsed() < CACHE_TTL);
        if cache.len() >= CACHE_CAPACITY {
            let oldest = cache
                .iter()
                .min_by_key(|(_, (at, _))| *at)
                .map(|(key, _)| key.clone());
            if let Some(oldest) = oldest {
                cache.remove(&oldest);
            }
        }
        cache.insert(req, (Instant::now(), page));
    }
}

async fn fetch(session: &LcuSession, req: &PageRequest) -> Result<MatchHistoryPage> {
    let mut sgp_error = None;
    if let Some(sgp) = &session.sgp {
        match fetch_sgp(session, sgp, req).await {
            Ok(games) => return Ok(req.page(DataSource::Sgp, None, games)),
            Err(err) => {
                log::warn!("SGP match history failed, falling back to LCU: {err}");
                sgp_error = Some(err.to_string());
            }
        }
    }
    let games = fetch_lcu(session, req).await?;
    Ok(req.page(DataSource::Lcu, sgp_error, games))
}

/// Current entitlements access token, which SGP match history accepts.
pub async fn entitlements_token(session: &LcuSession) -> Result<String> {
    let token: EntitlementsToken = session.http.get(ENTITLEMENTS_TOKEN).await?;
    Ok(token.access_token)
}

async fn fetch_sgp(
    session: &LcuSession,
    sgp: &SgpClient,
    req: &PageRequest,
) -> Result<Vec<GameSummary>> {
    let token = entitlements_token(session).await?;
    let history = sgp
        .match_history(&token, &req.puuid, req.start, req.count)
        .await?;

    let mut games = Vec::new();
    for game in history.games {
        games.extend(sgp_summary(game.json, &req.puuid));
    }
    Ok(games)
}

async fn fetch_lcu(session: &LcuSession, req: &PageRequest) -> Result<Vec<GameSummary>> {
    let (puuid, start) = (&req.puuid, req.start);
    let end = start + req.count - 1;
    let query = format!("begIndex={start}&endIndex={end}");
    let path = format!("/lol-match-history/v1/products/lol/{puuid}/matches?{query}");
    let history: LcuMatchHistory = session.http.get(&path).await?;

    let mut games = Vec::new();
    for game in history.games.games {
        games.extend(lcu_summary(game, puuid));
    }
    games.sort_by_key(|g| std::cmp::Reverse(g.created_at));
    Ok(games)
}

fn sgp_summary(game: SgpGameJson, puuid: &str) -> Option<GameSummary> {
    let all = &game.participants;
    let p = all.iter().find(|p| p.puuid == puuid)?;
    let result = game_result(
        &game.end_of_game_result,
        p.win,
        p.game_ended_in_early_surrender,
        p.team_early_surrendered,
    );
    // Arena groups players into subteams, which team shares do not describe.
    let metrics = if game.game_mode == "CHERRY" {
        None
    } else {
        Some(sgp_metrics(all, p))
    };
    let participants = all
        .iter()
        .map(|p| GameParticipant {
            puuid: p.puuid.clone(),
            team_id: p.team_id,
            position: p.team_position.clone(),
            jungler: p.team_position == "JUNGLE" || [p.spell1_id, p.spell2_id].contains(&SMITE),
        })
        .collect();
    Some(GameSummary {
        game_id: game.game_id,
        queue_id: game.queue_id,
        game_mode: game.game_mode.clone(),
        created_at: game.game_creation,
        duration: game.game_duration,
        result,
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
        position: p.team_position.clone(),
        team_id: p.team_id,
        vision_score: p.vision_score,
        metrics,
        participants,
    })
}

fn sgp_metrics(all: &[SgpParticipant], me: &SgpParticipant) -> GameMetrics {
    let team: Vec<&SgpParticipant> = all.iter().filter(|p| p.team_id == me.team_id).collect();
    let total = |f: fn(&SgpParticipant) -> i64| team_total(&team, f);
    let cs = |p: &SgpParticipant| p.total_minions_killed + p.neutral_minions_killed;

    let team_kills = total(|p| p.kills);
    let team_taken = total(|p| p.total_damage_taken);
    let avg_taken = team_taken as f64 / team.len().max(1) as f64;
    GameMetrics {
        team_size: team.len(),
        damage_share: ratio(
            me.total_damage_dealt_to_champions,
            total(|p| p.total_damage_dealt_to_champions),
        ),
        damage_taken_share: ratio(me.total_damage_taken, team_taken),
        gold_share: ratio(me.gold_earned, total(|p| p.gold_earned)),
        cs_share: ratio(cs(me), total(cs)),
        vision_share: ratio(me.vision_score, total(|p| p.vision_score)),
        kill_share: ratio(me.kills, team_kills),
        kill_participation: ratio(me.kills + me.assists, team_kills),
        heal_ratio: me.total_heal as f64 / avg_taken.max(1.0),
        solo_kills: me.challenges.solo_kills,
        enemy_missing_pings: me.enemy_missing_pings,
    }
}

fn team_total(team: &[&SgpParticipant], f: fn(&SgpParticipant) -> i64) -> i64 {
    team.iter().copied().map(f).sum()
}

fn ratio(part: i64, total: i64) -> f64 {
    if total <= 0 {
        0.0
    } else {
        part as f64 / total as f64
    }
}

/// LCU lists only the queried player's participant in each game.
fn lcu_summary(game: LcuGame, puuid: &str) -> Option<GameSummary> {
    let mut id = None;
    for identity in &game.participant_identities {
        if identity.player.puuid == puuid {
            id = Some(identity.participant_id);
        }
    }
    let mut participants = game.participants;
    let index = match id {
        Some(id) => participants.iter().position(|p| p.participant_id == id)?,
        None if participants.len() == 1 => 0,
        None => return None,
    };
    let p = participants.swap_remove(index);
    let s = p.stats;

    let result = game_result(
        &game.end_of_game_result,
        s.win,
        s.game_ended_in_early_surrender,
        s.team_early_surrendered,
    );
    Some(GameSummary {
        game_id: game.game_id,
        queue_id: game.queue_id,
        game_mode: game.game_mode,
        created_at: game.game_creation,
        duration: game.game_duration,
        result,
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
        position: String::new(),
        team_id: p.team_id,
        vision_score: s.vision_score,
        metrics: None,
        participants: Vec::new(),
    })
}

/// Same rules as League Akari's `computeWinResult`.
fn game_result(
    end_of_game_result: &str,
    win: bool,
    remake: bool,
    team_surrendered_early: bool,
) -> GameResult {
    if end_of_game_result.starts_with("Abort_") {
        GameResult::Abort
    } else if remake {
        GameResult::Remake
    } else if win && !team_surrendered_early {
        GameResult::Win
    } else {
        GameResult::Loss
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::clients::sgp::models::SgpMatchHistory;

    #[test]
    fn game_result_rules() {
        let cases = [
            ("Abort_Unexpected", true, false, false, GameResult::Abort),
            ("GameComplete", false, true, false, GameResult::Remake),
            ("GameComplete", true, false, true, GameResult::Loss),
            ("GameComplete", true, false, false, GameResult::Win),
            ("GameComplete", false, false, false, GameResult::Loss),
        ];
        for (eog, win, remake, early, expected) in cases {
            assert_eq!(game_result(eog, win, remake, early), expected);
        }
    }

    #[test]
    fn converts_sgp_game_for_the_queried_player() {
        let json = r#"{"games":[{"metadata":{},"json":{
            "gameId":7,"queueId":420,"gameMode":"CLASSIC","gameCreation":1000,
            "gameDuration":1800,"endOfGameResult":"GameComplete","participants":[
              {"puuid":"other","championId":1,"win":false},
              {"puuid":"me","championId":103,"kills":5,"deaths":2,"assists":9,"win":true,
               "spell1Id":4,"spell2Id":14,"item0":3157,"totalMinionsKilled":150,
               "neutralMinionsKilled":12,"teamPosition":"MIDDLE"}]}}]}"#;
        let history: SgpMatchHistory = serde_json::from_str(json).unwrap();
        let game = history.games.into_iter().next().unwrap().json;
        let s = sgp_summary(game, "me").unwrap();
        assert_eq!((s.game_id, s.champion_id), (7, 103));
        assert_eq!((s.kills, s.cs), (5, 162));
        assert_eq!(s.spells, [4, 14]);
        assert_eq!(s.items[0], 3157);
        assert_eq!(s.result, GameResult::Win);
        assert_eq!(s.position, "MIDDLE");
    }

    #[test]
    fn converts_lcu_game() {
        let json = r#"{"games":{"games":[{
            "gameId":8,"queueId":450,"gameMode":"ARAM","gameCreation":2000,
            "gameDuration":900,"participantIdentities":[
              {"participantId":1,"player":{"puuid":"me"}}],
            "participants":[{"participantId":1,"championId":22,"spell1Id":32,
              "spell2Id":4,"stats":{"kills":1,"deaths":8,"win":false,
              "gameEndedInEarlySurrender":true}}]}]}}"#;
        let history: LcuMatchHistory = serde_json::from_str(json).unwrap();
        let game = history.games.games.into_iter().next().unwrap();
        let s = lcu_summary(game, "me").unwrap();
        assert_eq!((s.game_id, s.champion_id, s.deaths), (8, 22, 8));
        assert_eq!(s.result, GameResult::Remake);
    }
}
