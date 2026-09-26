//! A teammate's most played champions in recent solo/duo games, for the champ-select
//! overlay: what they are likely to pick and how it went.

use std::collections::HashMap;

use serde::Serialize;

use super::match_history::{GameResult, GameSummary, MatchHistoryService};
use crate::error::Result;
use crate::state::session::LcuSession;

const GAMES: u32 = 50;
const SOLO_DUO: i64 = 420;
const CHAMPIONS_SHOWN: usize = 5;

#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RankedChampions {
    /// Wins and losses only; remakes are left out.
    pub games: usize,
    pub wins: usize,
    pub avg_kills: f64,
    pub avg_deaths: f64,
    pub avg_assists: f64,
    /// Most played first.
    pub champions: Vec<ChampionLine>,
}

#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ChampionLine {
    pub champion_id: i64,
    pub games: usize,
    pub wins: usize,
    pub avg_kills: f64,
    pub avg_deaths: f64,
    pub avg_assists: f64,
}

pub async fn load(
    history: &MatchHistoryService,
    session: &LcuSession,
    puuid: &str,
) -> Result<RankedChampions> {
    let request = history.get_queue(session, puuid, 0, GAMES, Some(SOLO_DUO));
    Ok(summarize(&request.await?.games))
}

#[derive(Default)]
struct Totals {
    games: usize,
    wins: usize,
    kills: i64,
    deaths: i64,
    assists: i64,
}

impl Totals {
    fn add(&mut self, game: &GameSummary) {
        self.games += 1;
        self.wins += usize::from(game.result == GameResult::Win);
        self.kills += game.kills;
        self.deaths += game.deaths;
        self.assists += game.assists;
    }

    fn average(&self, total: i64) -> f64 {
        if self.games == 0 {
            0.0
        } else {
            total as f64 / self.games as f64
        }
    }
}

fn summarize(games: &[GameSummary]) -> RankedChampions {
    let mut all = Totals::default();
    let mut by_champion: HashMap<i64, Totals> = HashMap::new();
    for game in games {
        if !matches!(game.result, GameResult::Win | GameResult::Loss) {
            continue;
        }
        all.add(game);
        by_champion.entry(game.champion_id).or_default().add(game);
    }

    let mut champions = Vec::new();
    for (champion_id, t) in by_champion {
        champions.push(ChampionLine {
            champion_id,
            games: t.games,
            wins: t.wins,
            avg_kills: t.average(t.kills),
            avg_deaths: t.average(t.deaths),
            avg_assists: t.average(t.assists),
        });
    }
    champions.sort_by(|a, b| {
        let by_games = b.games.cmp(&a.games).then(b.wins.cmp(&a.wins));
        by_games.then(a.champion_id.cmp(&b.champion_id))
    });
    champions.truncate(CHAMPIONS_SHOWN);

    RankedChampions {
        games: all.games,
        wins: all.wins,
        avg_kills: all.average(all.kills),
        avg_deaths: all.average(all.deaths),
        avg_assists: all.average(all.assists),
        champions,
    }
}
