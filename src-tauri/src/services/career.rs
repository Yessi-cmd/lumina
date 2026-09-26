//! Career analysis of one summoner over recent games, this season or as far back as the
//! history goes: overall numbers, most played champions and positions, ranked tiers,
//! champion mastery, and a performance profile against the other players of the same
//! games. Matchmaking puts players of similar rank together, so those players are the
//! same-rank comparison; the lane opponent is the fairest one when positions are known.

use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

use super::champion_mastery::{self, Mastery};
use super::match_history::{average_rates, GameResult, GameSummary, MatchHistoryService, Rates};
use crate::error::Result;
use crate::state::session::LcuSession;

const PAGE: u32 = 50;
const RECENT_GAMES: u32 = 20;
/// Season and career stop here: enough for a profile, and a few seconds to load.
const MAX_GAMES: usize = 400;
const MAX_PAGES: u32 = 12;
const CHAMPIONS_SHOWN: usize = 10;
const MASTERY_SHOWN: usize = 10;
const TREND_GAMES: usize = 60;
const DAY_MS: i64 = 86_400_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Range {
    /// The last 20 games.
    Recent,
    /// Since January 1st: League seasons start with the year.
    Season,
    /// As far back as match history goes, up to `MAX_GAMES`.
    Career,
}

#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Career {
    /// Wins and losses only; remakes are left out.
    pub games: usize,
    pub wins: usize,
    pub avg_kills: f64,
    pub avg_deaths: f64,
    pub avg_assists: f64,
    pub avg_minutes: f64,
    /// Unix milliseconds of the oldest and newest game counted.
    pub first_game_at: i64,
    pub last_game_at: i64,
    /// More games exist than were read.
    pub truncated: bool,
    /// Games with per-player data from every participant (SGP).
    pub compared_games: usize,
    pub me: Rates,
    pub peers: Rates,
    /// Average lane opponent, when enough games have positions.
    pub opponents: Option<Rates>,
    /// `opponent` or `peers`: what the radar compares against.
    pub reference: String,
    pub radar: Vec<RadarAxis>,
    pub champions: Vec<CareerChampion>,
    pub positions: Vec<PositionLine>,
    pub ranked: Vec<RankedQueue>,
    pub mastery: Vec<Mastery>,
    /// Oldest first.
    pub trend: Vec<TrendPoint>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RadarAxis {
    pub key: &'static str,
    pub label: &'static str,
    /// 0–100; 50 matches the reference, 100 is twice as good.
    pub score: f64,
    pub me: f64,
    pub reference: f64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CareerChampion {
    pub champion_id: i64,
    pub games: usize,
    pub wins: usize,
    pub avg_kills: f64,
    pub avg_deaths: f64,
    pub avg_assists: f64,
    /// Damage to champions per minute.
    pub damage: f64,
    pub mastery_points: Option<i64>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PositionLine {
    pub position: String,
    pub games: usize,
    pub wins: usize,
}

#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RankedQueue {
    /// `solo` or `flex`.
    pub queue: String,
    /// Empty when unranked.
    pub tier: String,
    pub division: String,
    pub league_points: i64,
    pub wins: i64,
    pub losses: i64,
    pub highest_tier: String,
    pub highest_division: String,
    pub previous_tier: String,
    pub previous_division: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TrendPoint {
    pub result: GameResult,
    pub kda: f64,
    pub created_at: i64,
}

/// `/lol-ranked/v1/ranked-stats/{puuid}`
#[derive(Debug, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
struct LcuRankedStats {
    queue_map: HashMap<String, LcuRankedQueue>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
struct LcuRankedQueue {
    tier: String,
    division: String,
    league_points: i64,
    wins: i64,
    losses: i64,
    highest_tier: String,
    highest_division: String,
    previous_season_end_tier: String,
    previous_season_end_division: String,
}

pub async fn load(
    history: &MatchHistoryService,
    session: &LcuSession,
    puuid: &str,
    queue: Option<i64>,
    range: Range,
) -> Result<Career> {
    let fetched = games(history, session, puuid, queue, range);
    let (fetched, ranked, mastery) = tokio::join!(
        fetched,
        ranked(session, puuid),
        champion_mastery::all(session, puuid),
    );
    let (games, truncated) = fetched?;
    let mastery = mastery.unwrap_or_default();
    let mut career = summarize(&games, &mastery);
    career.truncated = truncated;
    career.ranked = ranked;
    career.mastery = mastery.into_iter().take(MASTERY_SHOWN).collect();
    Ok(career)
}

/// Newest first; the flag says whether the history goes on beyond what was read.
async fn games(
    history: &MatchHistoryService,
    session: &LcuSession,
    puuid: &str,
    queue: Option<i64>,
    range: Range,
) -> Result<(Vec<GameSummary>, bool)> {
    if range == Range::Recent {
        let request = history.get_queue(session, puuid, 0, RECENT_GAMES, queue);
        return Ok((request.await?.games, false));
    }
    let since = match range {
        Range::Season => year_start_ms(now_ms()),
        _ => 0,
    };
    let mut games = Vec::new();
    for page in 0..MAX_PAGES {
        let request = history.get_queue(session, puuid, page * PAGE, PAGE, queue);
        let found = request.await?.games;
        let done = found.is_empty() || found.iter().any(|g| g.created_at < since);
        games.extend(found.into_iter().filter(|g| g.created_at >= since));
        if done || games.len() >= MAX_GAMES {
            let truncated = !done;
            games.truncate(MAX_GAMES);
            return Ok((games, truncated));
        }
    }
    Ok((games, true))
}

async fn ranked(session: &LcuSession, puuid: &str) -> Vec<RankedQueue> {
    let path = format!("/lol-ranked/v1/ranked-stats/{puuid}");
    let stats = match session.http.get::<LcuRankedStats>(&path).await {
        Ok(stats) => stats,
        Err(err) => {
            log::debug!("ranked stats unavailable: {err}");
            return Vec::new();
        }
    };
    let mut out = Vec::new();
    for (key, queue) in [("RANKED_SOLO_5x5", "solo"), ("RANKED_FLEX_SR", "flex")] {
        let Some(q) = stats.queue_map.get(key) else {
            continue;
        };
        out.push(RankedQueue {
            queue: queue.to_owned(),
            tier: ranked_tier(&q.tier),
            division: q.division.clone(),
            league_points: q.league_points,
            wins: q.wins,
            losses: q.losses,
            highest_tier: ranked_tier(&q.highest_tier),
            highest_division: q.highest_division.clone(),
            previous_tier: ranked_tier(&q.previous_season_end_tier),
            previous_division: q.previous_season_end_division.clone(),
        });
    }
    out
}

/// `NONE` and empty both mean unranked.
fn ranked_tier(tier: &str) -> String {
    match tier {
        "NONE" => String::new(),
        other => other.to_owned(),
    }
}

#[derive(Default)]
struct Totals {
    games: usize,
    wins: usize,
    kills: i64,
    deaths: i64,
    assists: i64,
    damage: f64,
}

impl Totals {
    fn add(&mut self, game: &GameSummary) {
        self.games += 1;
        self.wins += usize::from(game.result == GameResult::Win);
        self.kills += game.kills;
        self.deaths += game.deaths;
        self.assists += game.assists;
        let minutes = (game.duration as f64 / 60.0).max(1.0);
        self.damage += game.damage_to_champions as f64 / minutes;
    }

    fn mean(&self, total: f64) -> f64 {
        if self.games == 0 {
            0.0
        } else {
            total / self.games as f64
        }
    }
}

fn summarize(games: &[GameSummary], mastery: &[Mastery]) -> Career {
    let counted: Vec<&GameSummary> = games.iter().filter(|g| counts(g)).collect();
    let mut all = Totals::default();
    let mut by_champion: HashMap<i64, Totals> = HashMap::new();
    let mut by_position: HashMap<String, Totals> = HashMap::new();
    let (mut mine, mut peers, mut opponents) = (Vec::new(), Vec::new(), Vec::new());
    let mut seconds = 0;
    for game in &counted {
        all.add(game);
        seconds += game.duration;
        by_champion.entry(game.champion_id).or_default().add(game);
        if !game.position.is_empty() {
            let position = by_position.entry(game.position.clone()).or_default();
            position.add(game);
        }
        if let Some(c) = &game.comparison {
            mine.push(c.me);
            peers.push(c.peers);
            opponents.extend(c.opponent);
        }
    }

    let me = average_rates(&mine);
    let peers = average_rates(&peers);
    // Lane opponents are the fairer yardstick, once most games have them.
    let opponents = (opponents.len() * 2 >= mine.len() && !opponents.is_empty())
        .then(|| average_rates(&opponents));
    let (reference, reference_rates) = match &opponents {
        Some(o) => ("opponent", *o),
        None => ("peers", peers),
    };

    Career {
        games: all.games,
        wins: all.wins,
        avg_kills: all.mean(all.kills as f64),
        avg_deaths: all.mean(all.deaths as f64),
        avg_assists: all.mean(all.assists as f64),
        avg_minutes: all.mean(seconds as f64 / 60.0),
        first_game_at: counted.last().map_or(0, |g| g.created_at),
        last_game_at: counted.first().map_or(0, |g| g.created_at),
        truncated: false,
        compared_games: mine.len(),
        me,
        peers,
        opponents,
        reference: reference.to_owned(),
        radar: if mine.is_empty() {
            Vec::new()
        } else {
            radar(&me, &reference_rates)
        },
        champions: champions(by_champion, mastery),
        positions: positions(by_position),
        ranked: Vec::new(),
        mastery: Vec::new(),
        trend: trend(&counted),
    }
}

fn counts(game: &GameSummary) -> bool {
    matches!(game.result, GameResult::Win | GameResult::Loss)
}

fn champions(by_champion: HashMap<i64, Totals>, mastery: &[Mastery]) -> Vec<CareerChampion> {
    let mut out = Vec::new();
    for (champion_id, t) in by_champion {
        let points = mastery.iter().find(|m| m.champion_id == champion_id);
        out.push(CareerChampion {
            champion_id,
            games: t.games,
            wins: t.wins,
            avg_kills: t.mean(t.kills as f64),
            avg_deaths: t.mean(t.deaths as f64),
            avg_assists: t.mean(t.assists as f64),
            damage: t.mean(t.damage),
            mastery_points: points.map(|m| m.champion_points),
        });
    }
    out.sort_by(|a, b| b.games.cmp(&a.games).then(b.wins.cmp(&a.wins)));
    out.truncate(CHAMPIONS_SHOWN);
    out
}

fn positions(by_position: HashMap<String, Totals>) -> Vec<PositionLine> {
    let mut out = Vec::new();
    for (position, t) in by_position {
        out.push(PositionLine {
            position,
            games: t.games,
            wins: t.wins,
        });
    }
    out.sort_by_key(|p| std::cmp::Reverse(p.games));
    out
}

fn trend(counted: &[&GameSummary]) -> Vec<TrendPoint> {
    let mut points = Vec::new();
    for game in counted.iter().take(TREND_GAMES).rev() {
        let deaths = game.deaths.max(1) as f64;
        points.push(TrendPoint {
            result: game.result,
            kda: (game.kills + game.assists) as f64 / deaths,
            created_at: game.created_at,
        });
    }
    points
}

/// Six dimensions, each scored so that matching the reference is 50.
fn radar(me: &Rates, reference: &Rates) -> Vec<RadarAxis> {
    let axes: [(&'static str, &'static str, fn(&Rates) -> f64); 5] = [
        ("damage", "输出", |r| r.damage),
        ("tanking", "承伤", |r| r.damage_taken),
        ("economy", "发育", |r| r.gold),
        ("teamfight", "参团", |r| r.kill_participation),
        ("vision", "视野", |r| r.vision),
    ];
    let mut out = Vec::new();
    for (key, label, value) in axes {
        let (mine, theirs) = (value(me), value(reference));
        out.push(RadarAxis {
            key,
            label,
            score: score(mine, theirs),
            me: mine,
            reference: theirs,
        });
    }
    // Fewer deaths is better, so the ratio is inverted.
    out.push(RadarAxis {
        key: "survival",
        label: "生存",
        score: score(reference.deaths, me.deaths.max(0.1)),
        me: me.deaths,
        reference: reference.deaths,
    });
    out
}

fn score(mine: f64, theirs: f64) -> f64 {
    if theirs <= 0.0 {
        return 50.0;
    }
    (50.0 * mine / theirs).clamp(0.0, 100.0)
}

fn now_ms() -> i64 {
    let since = SystemTime::now().duration_since(UNIX_EPOCH);
    since.map_or(0, |d| d.as_millis() as i64)
}

/// Unix milliseconds of January 1st (UTC) of the year `now_ms` falls in.
fn year_start_ms(now_ms: i64) -> i64 {
    let days = now_ms.div_euclid(DAY_MS);
    let mut year = 1970 + days / 366;
    while days_to_new_year(year + 1) <= days {
        year += 1;
    }
    days_to_new_year(year) * DAY_MS
}

/// Days from 1970-01-01 to January 1st of `year` (proleptic Gregorian).
fn days_to_new_year(year: i64) -> i64 {
    let y = year - 1;
    let era = y.div_euclid(400);
    let year_of_era = y - era * 400;
    // January is month 10 of the March-based year used by the algorithm.
    let day_of_year = 306;
    let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
    era * 146_097 + day_of_era - 719_468
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_the_start_of_the_year() {
        assert_eq!(days_to_new_year(1970), 0);
        assert_eq!(days_to_new_year(2000), 10_957);
        // 2026-09-26T12:00:00Z
        let now = 1_790_424_000_000;
        assert_eq!(year_start_ms(now), 1_767_225_600_000);
    }

    #[test]
    fn radar_is_fifty_when_equal_and_inverts_deaths() {
        let me = Rates {
            damage: 1000.0,
            deaths: 2.0,
            ..Rates::default()
        };
        let reference = Rates {
            damage: 500.0,
            deaths: 4.0,
            ..Rates::default()
        };
        let axes = radar(&me, &reference);
        let scores: Vec<f64> = axes.iter().map(|a| a.score).collect();
        for (score, expected) in scores.iter().zip([100.0, 50.0, 50.0, 50.0, 50.0, 100.0]) {
            assert!((score - expected).abs() < 1e-9, "{scores:?}");
        }
    }
}
