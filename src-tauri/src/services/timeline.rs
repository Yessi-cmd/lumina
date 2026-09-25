//! Early-game digests from SGP timelines: deaths to the enemy jungler before 15 minutes
//! (League Akari's "easy to gank") and gold / CS against the lane opponent at 10 minutes.

use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, Mutex, PoisonError};

use tokio::sync::Semaphore;

use super::match_history::{entitlements_token, GameParticipant, GameSummary};
use crate::clients::sgp::models::{SgpFrame, SgpParticipantFrame, SgpTimeline};
use crate::error::{AppError, Result};
use crate::state::session::LcuSession;

const MAX_CONCURRENT: usize = 4;
const CACHE_CAPACITY: usize = 400;
/// League Akari's `EARLY_JUNGLE_INVOLVEMENT_LIMIT_MS`.
const EARLY_MS: i64 = 15 * 60 * 1000;
const LANE_CHECK_MS: i64 = 10 * 60 * 1000;

/// One player's early game in one match.
#[derive(Debug, Clone, Default)]
pub struct EarlyGame {
    /// `None` for the jungler, or when the enemy jungler is unknown.
    pub deaths_to_jungler: Option<i64>,
    /// Own gold minus the lane opponent's at 10 minutes; `None` without a clear opponent.
    pub gold_diff_10: Option<i64>,
    pub cs_diff_10: Option<i64>,
}

/// Early game of everyone in one match, keyed by puuid.
pub type GameDigest = HashMap<String, EarlyGame>;

/// A player's early game averaged over their recent Rift games.
#[derive(Debug, Clone, Default)]
pub struct EarlyStats {
    pub gank_games: usize,
    pub avg_deaths_to_jungler: f64,
    pub lane_games: usize,
    pub avg_gold_diff_10: f64,
    pub avg_cs_diff_10: f64,
}

impl EarlyStats {
    pub fn from_games<'a>(games: impl Iterator<Item = &'a EarlyGame>) -> Self {
        let (mut deaths, mut gold, mut cs) = (Vec::new(), Vec::new(), Vec::new());
        for game in games {
            deaths.extend(game.deaths_to_jungler);
            gold.extend(game.gold_diff_10);
            cs.extend(game.cs_diff_10);
        }
        Self {
            gank_games: deaths.len(),
            avg_deaths_to_jungler: mean(&deaths),
            lane_games: gold.len(),
            avg_gold_diff_10: mean(&gold),
            avg_cs_diff_10: mean(&cs),
        }
    }
}

pub struct TimelineService {
    limiter: Semaphore,
    cache: Mutex<Cache>,
}

#[derive(Default)]
struct Cache {
    digests: HashMap<i64, Arc<GameDigest>>,
    order: VecDeque<i64>,
}

impl Default for TimelineService {
    fn default() -> Self {
        Self {
            limiter: Semaphore::new(MAX_CONCURRENT),
            cache: Mutex::default(),
        }
    }
}

impl TimelineService {
    /// Timelines never change once a game is over, so digests are cached for the session.
    pub async fn digest(
        &self,
        session: &LcuSession,
        game: &GameSummary,
    ) -> Result<Arc<GameDigest>> {
        if let Some(digest) = self.cached(game.game_id) {
            return Ok(digest);
        }
        let Some(sgp) = &session.sgp else {
            return Err(AppError::Message("没有可用的 SGP 服务器".to_owned()));
        };

        let permit = self.limiter.acquire().await;
        let _permit = permit.expect("semaphore is never closed");
        if let Some(digest) = self.cached(game.game_id) {
            return Ok(digest);
        }
        let token = entitlements_token(session).await?;
        let details = sgp.game_details(&token, game.game_id).await?;
        let digest = Arc::new(digest(&details.json, &game.participants));
        self.store(game.game_id, digest.clone());
        Ok(digest)
    }

    fn cached(&self, game_id: i64) -> Option<Arc<GameDigest>> {
        let cache = self.cache.lock().unwrap_or_else(PoisonError::into_inner);
        cache.digests.get(&game_id).cloned()
    }

    fn store(&self, game_id: i64, digest: Arc<GameDigest>) {
        let mut cache = self.cache.lock().unwrap_or_else(PoisonError::into_inner);
        if cache.digests.insert(game_id, digest).is_none() {
            cache.order.push_back(game_id);
        }
        while cache.order.len() > CACHE_CAPACITY {
            if let Some(oldest) = cache.order.pop_front() {
                cache.digests.remove(&oldest);
            }
        }
    }
}

fn digest(timeline: &SgpTimeline, players: &[GameParticipant]) -> GameDigest {
    let mut ids: HashMap<&str, i64> = HashMap::new();
    for p in &timeline.participants {
        ids.insert(&p.puuid, p.participant_id);
    }
    let lane_frame = timeline.frames.iter().find(|f| f.timestamp >= LANE_CHECK_MS);

    let mut out = GameDigest::new();
    for player in players {
        let Some(&id) = ids.get(player.puuid.as_str()) else {
            continue;
        };
        let enemies = players.iter().filter(|p| p.team_id != player.team_id);
        let enemy_junglers: Vec<i64> = enemies
            .clone()
            .filter(|p| p.jungler)
            .filter_map(|p| ids.get(p.puuid.as_str()).copied())
            .collect();
        let deaths_to_jungler = if player.jungler || enemy_junglers.is_empty() {
            None
        } else {
            Some(early_deaths_to(timeline, id, &enemy_junglers))
        };

        let mut opponents = enemies.filter(|p| same_lane(p, player));
        let opponent = match (opponents.next(), opponents.next()) {
            (Some(only), None) => ids.get(only.puuid.as_str()).copied(),
            _ => None,
        };
        let diff = match (lane_frame, opponent) {
            (Some(frame), Some(opponent)) => lane_diff(frame, id, opponent),
            _ => None,
        };

        let early = EarlyGame {
            deaths_to_jungler,
            gold_diff_10: diff.map(|(gold, _)| gold),
            cs_diff_10: diff.map(|(_, cs)| cs),
        };
        out.insert(player.puuid.clone(), early);
    }
    out
}

/// Deaths before 15 minutes where one of `killers` got the kill or an assist.
fn early_deaths_to(timeline: &SgpTimeline, victim: i64, killers: &[i64]) -> i64 {
    let mut count = 0;
    for event in timeline.frames.iter().flat_map(|f| &f.events) {
        if event.kind != "CHAMPION_KILL" || event.victim_id != victim || event.timestamp > EARLY_MS {
            continue;
        }
        let assists = &event.assisting_participant_ids;
        let assisted = assists.iter().any(|a| killers.contains(a));
        if killers.contains(&event.killer_id) || assisted {
            count += 1;
        }
    }
    count
}

/// Gold and CS differences between two participants in one frame.
fn lane_diff(frame: &SgpFrame, me: i64, opponent: i64) -> Option<(i64, i64)> {
    let mine = frame.participant_frames.get(&me.to_string())?;
    let theirs = frame.participant_frames.get(&opponent.to_string())?;
    Some((mine.total_gold - theirs.total_gold, cs(mine) - cs(theirs)))
}

fn cs(frame: &SgpParticipantFrame) -> i64 {
    frame.minions_killed + frame.jungle_minions_killed
}

fn same_lane(a: &GameParticipant, b: &GameParticipant) -> bool {
    !a.position.is_empty() && a.position == b.position
}

fn mean(values: &[i64]) -> f64 {
    if values.is_empty() {
        0.0
    } else {
        values.iter().sum::<i64>() as f64 / values.len() as f64
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn participant(puuid: &str, team_id: i64, position: &str) -> GameParticipant {
        GameParticipant {
            puuid: puuid.to_owned(),
            team_id,
            position: position.to_owned(),
            jungler: position == "JUNGLE",
        }
    }

    #[test]
    fn counts_early_jungle_deaths_and_lane_gold() {
        let json = r#"{"participants":[
              {"participantId":1,"puuid":"mid"},{"participantId":2,"puuid":"jg"},
              {"participantId":6,"puuid":"emid"},{"participantId":7,"puuid":"ejg"}],
            "frames":[
              {"timestamp":300000,"events":[
                {"type":"CHAMPION_KILL","timestamp":300000,"killerId":6,"victimId":1,
                 "assistingParticipantIds":[7]},
                {"type":"CHAMPION_KILL","timestamp":400000,"killerId":6,"victimId":1}]},
              {"timestamp":600000,"participantFrames":{
                "1":{"totalGold":3500,"minionsKilled":80},
                "6":{"totalGold":4000,"minionsKilled":85}}},
              {"timestamp":1000000,"events":[
                {"type":"CHAMPION_KILL","timestamp":1000000,"killerId":7,"victimId":1}]}]}"#;
        let timeline: SgpTimeline = serde_json::from_str(json).unwrap();
        let players = [
            participant("mid", 100, "MIDDLE"),
            participant("jg", 100, "JUNGLE"),
            participant("emid", 200, "MIDDLE"),
            participant("ejg", 200, "JUNGLE"),
        ];
        let digest = digest(&timeline, &players);
        let mid = &digest["mid"];
        assert_eq!(mid.deaths_to_jungler, Some(1));
        assert_eq!(mid.gold_diff_10, Some(-500));
        assert_eq!(mid.cs_diff_10, Some(-5));
        assert_eq!(digest["jg"].deaths_to_jungler, None);
    }

    #[test]
    fn averages_early_stats() {
        let games = [
            EarlyGame {
                deaths_to_jungler: Some(2),
                gold_diff_10: Some(300),
                cs_diff_10: None,
            },
            EarlyGame {
                deaths_to_jungler: Some(1),
                gold_diff_10: None,
                cs_diff_10: None,
            },
        ];
        let stats = EarlyStats::from_games(games.iter());
        assert_eq!((stats.gank_games, stats.lane_games), (2, 1));
        assert!((stats.avg_deaths_to_jungler - 1.5).abs() < 1e-9);
        assert!((stats.avg_gold_diff_10 - 300.0).abs() < 1e-9);
    }
}
