//! Rank history. The client only reports the current rank, so every finished ranked game
//! is recorded here with its LP change; the rank page draws its trend from these points.
//!
//! The rank is read when the client connects and when a game ends (a few times, because
//! the client updates it a moment late). A point is added whenever a queue's standing
//! differs from its last point. History is kept per account in `rank_history.json`.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, PoisonError};
use std::time::Duration;

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, Manager};

use super::match_history::{GameResult, GameSummary};
use super::roster_relations::now_ms;
use crate::clients::lcu::models::Summoner;
use crate::error::{AppError, Result};
use crate::state::session::LcuSession;
use crate::state::AppState;

pub const RANK_EVENT: &str = "rank://updated";
const FILE_NAME: &str = "rank_history.json";
const CURRENT_RANKED_STATS: &str = "/lol-ranked/v1/current-ranked-stats";
/// LCU queue key, the name the frontend uses, and the matchmaking queue id.
const QUEUES: [(&str, &str, i64); 2] = [
    ("RANKED_SOLO_5x5", "solo", 420),
    ("RANKED_FLEX_SR", "flex", 440),
];
/// Oldest points beyond this are dropped, per account.
const MAX_POINTS: usize = 4000;
/// The client updates the rank a few seconds after the game ends.
const SETTLE_ATTEMPTS: u32 = 6;
const SETTLE_INTERVAL: Duration = Duration::from_secs(5);
/// Match history can lag behind the rank as well.
const GAME_ATTEMPTS: u32 = 3;
const GAME_RETRY: Duration = Duration::from_secs(4);
const GAME_LOOKBACK: u32 = 3;

const TIERS: [&str; 7] = [
    "IRON", "BRONZE", "SILVER", "GOLD", "PLATINUM", "EMERALD", "DIAMOND",
];
const APEX_TIERS: [&str; 3] = ["MASTER", "GRANDMASTER", "CHALLENGER"];
/// Master starts where Diamond I ends (7 tiers of 4 divisions of 100 LP).
const APEX_BASE: i64 = 2800;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Outcome {
    Win,
    Loss,
}

/// A queue's standing after a change, with what is known about the game behind it.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct RankPoint {
    /// Unix milliseconds at which the change was noticed.
    pub at: i64,
    /// `solo` or `flex`.
    pub queue: String,
    pub tier: String,
    /// Empty for Master and above.
    pub division: String,
    pub lp: i64,
    pub wins: i64,
    pub losses: i64,
    /// Change of the rank score (LP, counted across divisions) since the previous point
    /// of the queue; `None` for the first point and after a season reset.
    pub delta: Option<i64>,
    /// Only when exactly one game was played since the previous point.
    pub outcome: Option<Outcome>,
    pub game_id: Option<i64>,
    pub champion_id: Option<i64>,
    /// Kills, deaths, assists.
    pub kda: Option<[i64; 3]>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
struct Account {
    name: String,
    points: Vec<RankPoint>,
}

#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
struct Store {
    /// The account seen last, shown when the client is closed.
    last_puuid: Option<String>,
    accounts: HashMap<String, Account>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RankHistory {
    pub puuid: String,
    pub name: String,
    pub points: Vec<RankPoint>,
}

/// `/lol-ranked/v1/current-ranked-stats`
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
}

/// One queue's rank as the client reports it.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Standing {
    queue: &'static str,
    tier: String,
    division: String,
    lp: i64,
    wins: i64,
    losses: i64,
}

/// A point that was just added.
#[derive(Debug, Clone, Copy)]
struct Fresh {
    queue: &'static str,
    at: i64,
    /// `at` of the point before it, so the game must have started later.
    since: i64,
    outcome: Option<Outcome>,
}

pub struct RankTracker {
    store: Mutex<Store>,
    path: Option<PathBuf>,
}

impl RankTracker {
    pub fn load(app: &AppHandle) -> Self {
        let path = app.path().app_data_dir().ok();
        let path = path.map(|dir| dir.join(FILE_NAME));
        let store = path.as_deref().map(read).unwrap_or_default();
        Self {
            store: Mutex::new(store),
            path,
        }
    }

    /// History of `puuid`, or of the account seen last.
    pub fn history(&self, puuid: Option<&str>) -> Option<RankHistory> {
        let store = self.store.lock().unwrap_or_else(PoisonError::into_inner);
        let puuid = puuid.or(store.last_puuid.as_deref())?;
        let account = store.accounts.get(puuid)?;
        Some(RankHistory {
            puuid: puuid.to_owned(),
            name: account.name.clone(),
            points: account.points.clone(),
        })
    }

    fn observe(&self, summoner: &Summoner, standings: &[Standing], at: i64) -> Vec<Fresh> {
        let mut store = self.store.lock().unwrap_or_else(PoisonError::into_inner);
        let account = store.accounts.entry(summoner.puuid.clone()).or_default();
        account.name = account_name(summoner);
        let fresh: Vec<Fresh> = standings
            .iter()
            .filter_map(|standing| account.observe(standing, at))
            .collect();
        account.trim();
        let switched = store.last_puuid.as_deref() != Some(summoner.puuid.as_str());
        store.last_puuid = Some(summoner.puuid.clone());
        if !fresh.is_empty() || switched {
            self.save(&store);
        }
        fresh
    }

    /// Adds the game behind a point.
    fn attach(&self, puuid: &str, queue: &str, at: i64, game: &GameSummary) {
        let mut store = self.store.lock().unwrap_or_else(PoisonError::into_inner);
        let Some(account) = store.accounts.get_mut(puuid) else {
            return;
        };
        let found = account
            .points
            .iter_mut()
            .find(|p| p.at == at && p.queue == queue);
        let Some(point) = found else {
            return;
        };
        point.game_id = Some(game.game_id);
        point.champion_id = Some(game.champion_id);
        point.kda = Some([game.kills, game.deaths, game.assists]);
        self.save(&store);
    }

    fn save(&self, store: &Store) {
        let Some(path) = &self.path else {
            return;
        };
        if let Err(err) = write(path, store) {
            log::warn!("failed to save {}: {err}", path.display());
        }
    }
}

impl Account {
    /// Adds a point when `standing` differs from the last point of its queue.
    fn observe(&mut self, standing: &Standing, at: i64) -> Option<Fresh> {
        let now = score(&standing.tier, &standing.division, standing.lp)?;
        let mut point = RankPoint {
            at,
            queue: standing.queue.to_owned(),
            tier: standing.tier.clone(),
            division: standing.division.clone(),
            lp: standing.lp,
            wins: standing.wins,
            losses: standing.losses,
            ..RankPoint::default()
        };
        let mut since = 0;
        if let Some(last) = self.last(standing.queue).cloned() {
            let unchanged = last.tier == standing.tier
                && last.division == standing.division
                && last.lp == standing.lp
                && last.wins == standing.wins
                && last.losses == standing.losses;
            if unchanged {
                return None;
            }
            // Win and loss counts start over with a new season.
            let new_season = standing.wins < last.wins || standing.losses < last.losses;
            if !new_season {
                let before = score(&last.tier, &last.division, last.lp)?;
                point.delta = Some(now - before);
                point.outcome = outcome(standing.wins - last.wins, standing.losses - last.losses);
                since = last.at;
            }
        }
        let fresh = Fresh {
            queue: standing.queue,
            at,
            since,
            outcome: point.outcome,
        };
        self.points.push(point);
        Some(fresh)
    }

    fn last(&self, queue: &str) -> Option<&RankPoint> {
        self.points.iter().rev().find(|p| p.queue == queue)
    }

    fn trim(&mut self) {
        let excess = self.points.len().saturating_sub(MAX_POINTS);
        self.points.drain(..excess);
    }
}

/// Rank as one number, so a promotion counts as the LP it is worth: Iron IV 0 LP is 0, and
/// every division is 100 further. Master and above have no divisions and share one scale.
fn score(tier: &str, division: &str, lp: i64) -> Option<i64> {
    if APEX_TIERS.contains(&tier) {
        return Some(APEX_BASE + lp);
    }
    let tier_index = TIERS.iter().position(|t| *t == tier)?;
    let division_index = match division {
        "IV" => 0,
        "III" => 1,
        "II" => 2,
        "I" => 3,
        _ => return None,
    };
    Some(tier_index as i64 * 400 + division_index * 100 + lp)
}

fn outcome(wins: i64, losses: i64) -> Option<Outcome> {
    match (wins, losses) {
        (1, 0) => Some(Outcome::Win),
        (0, 1) => Some(Outcome::Loss),
        _ => None,
    }
}

fn standings(stats: &LcuRankedStats) -> Vec<Standing> {
    let mut out = Vec::new();
    for (key, queue, _) in QUEUES {
        let Some(q) = stats.queue_map.get(key) else {
            continue;
        };
        // Unranked queues report an empty tier or `NONE`, which has no score.
        let tier = q.tier.to_ascii_uppercase();
        let division = q.division.to_ascii_uppercase();
        if score(&tier, &division, q.league_points).is_none() {
            continue;
        }
        out.push(Standing {
            queue,
            tier,
            division,
            lp: q.league_points,
            wins: q.wins,
            losses: q.losses,
        });
    }
    out
}

fn queue_id(queue: &str) -> i64 {
    let found = QUEUES.iter().find(|(_, name, _)| *name == queue);
    found.map_or(0, |(_, _, id)| *id)
}

fn account_name(summoner: &Summoner) -> String {
    if summoner.game_name.is_empty() {
        return summoner.display_name.clone();
    }
    format!("{}#{}", summoner.game_name, summoner.tag_line)
}

fn read(path: &Path) -> Store {
    let Ok(text) = std::fs::read_to_string(path) else {
        return Store::default();
    };
    match serde_json::from_str(&text) {
        Ok(store) => store,
        Err(err) => {
            // Keep the unreadable file: the next save would otherwise overwrite it.
            log::warn!("ignoring invalid {}: {err}", path.display());
            let _ = std::fs::rename(path, path.with_extension("json.bad"));
            Store::default()
        }
    }
}

fn write(path: &Path, store: &Store) -> Result<()> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let json = serde_json::to_vec(store);
    let json = json.map_err(|err| AppError::Message(err.to_string()))?;
    // Written beside the file first, so a crash cannot leave half a history behind.
    let temp = path.with_extension("json.tmp");
    std::fs::write(&temp, json)?;
    std::fs::rename(&temp, path)?;
    Ok(())
}

/// The client just connected: take the rank as the first point.
pub fn on_connected(app: &AppHandle) {
    tauri::async_runtime::spawn(settle(app.clone(), 1));
}

pub fn on_phase(app: &AppHandle, phase: &str) {
    let attempts = match phase {
        "EndOfGame" => SETTLE_ATTEMPTS,
        // A game the client never reported as ended, e.g. after a reconnect.
        "Lobby" | "None" => 1,
        _ => return,
    };
    tauri::async_runtime::spawn(settle(app.clone(), attempts));
}

async fn settle(app: AppHandle, attempts: u32) {
    for attempt in 0..attempts {
        if attempt > 0 {
            tokio::time::sleep(SETTLE_INTERVAL).await;
        }
        match record(&app).await {
            Ok(true) => return,
            Ok(false) => {}
            Err(err) => {
                log::debug!("rank history not updated: {err}");
                return;
            }
        }
    }
}

/// Reads the current rank; returns whether a point was added.
async fn record(app: &AppHandle) -> Result<bool> {
    let state = app.state::<AppState>();
    let session = state.session()?;
    let Some(summoner) = state.lcu_snapshot().summoner else {
        return Ok(false);
    };
    let stats: LcuRankedStats = session.http.get(CURRENT_RANKED_STATS).await?;
    let found = standings(&stats);
    let ranks = &state.ranks;
    let fresh = ranks.observe(&summoner, &found, now_ms());
    if fresh.is_empty() {
        return Ok(false);
    }
    notify(app);
    for point in fresh.iter().filter(|f| f.outcome.is_some()) {
        let game = latest_game(&state, &session, &summoner.puuid, point).await;
        if let Some(game) = game {
            ranks.attach(&summoner.puuid, point.queue, point.at, &game);
            notify(app);
        }
    }
    Ok(true)
}

/// The game that moved the rank: the newest one of its queue since the previous point.
async fn latest_game(
    state: &AppState,
    session: &LcuSession,
    puuid: &str,
    fresh: &Fresh,
) -> Option<GameSummary> {
    let queue = Some(queue_id(fresh.queue));
    for attempt in 0..GAME_ATTEMPTS {
        if attempt > 0 {
            tokio::time::sleep(GAME_RETRY).await;
        }
        let history = &state.match_history;
        let page = history
            .get_fresh(session, puuid, 0, GAME_LOOKBACK, queue)
            .await
            .ok()?;
        let found = page
            .games
            .into_iter()
            .find(|g| g.created_at > fresh.since && result_matches(g.result, fresh.outcome));
        if found.is_some() {
            return found;
        }
    }
    None
}

fn result_matches(result: GameResult, outcome: Option<Outcome>) -> bool {
    matches!(
        (result, outcome),
        (GameResult::Win, Some(Outcome::Win)) | (GameResult::Loss, Some(Outcome::Loss))
    )
}

fn notify(app: &AppHandle) {
    if let Err(err) = app.emit(RANK_EVENT, ()) {
        log::warn!("failed to emit {RANK_EVENT}: {err}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn step(
        account: &mut Account,
        (tier, division, lp): (&str, &str, i64),
        (wins, losses): (i64, i64),
        at: i64,
    ) -> Option<Fresh> {
        let standing = Standing {
            queue: "solo",
            tier: tier.to_owned(),
            division: division.to_owned(),
            lp,
            wins,
            losses,
        };
        account.observe(&standing, at)
    }

    #[test]
    fn promotion_counts_the_lp_it_is_worth() {
        assert_eq!(score("EMERALD", "IV", 75), Some(2075));
        assert_eq!(score("EMERALD", "III", 0), Some(2100));
        assert_eq!(score("DIAMOND", "I", 99), Some(2799));
        assert_eq!(score("MASTER", "NA", 0), Some(2800));
        assert_eq!(score("GRANDMASTER", "NA", 150), Some(2950));
        assert_eq!(score("", "NA", 0), None);
        assert_eq!(score("GOLD", "NA", 0), None);
    }

    #[test]
    fn first_point_has_no_delta_and_the_next_one_does() {
        let mut account = Account::default();
        let first = step(&mut account, ("GOLD", "II", 40), (10, 8), 1).unwrap();
        assert_eq!(first.outcome, None);
        assert_eq!(account.points[0].delta, None);

        let next = step(&mut account, ("GOLD", "II", 62), (11, 8), 2).unwrap();
        assert_eq!(next.outcome, Some(Outcome::Win));
        assert_eq!(next.since, 1);
        assert_eq!(account.points[1].delta, Some(22));
    }

    #[test]
    fn promotion_and_demotion_give_real_deltas() {
        let mut account = Account::default();
        step(&mut account, ("GOLD", "II", 90), (10, 8), 1);
        step(&mut account, ("GOLD", "I", 5), (11, 8), 2);
        assert_eq!(account.points[1].delta, Some(15));
        step(&mut account, ("GOLD", "II", 80), (11, 9), 3);
        assert_eq!(account.points[2].delta, Some(-25));
        assert_eq!(account.points[2].outcome, Some(Outcome::Loss));
    }

    #[test]
    fn unchanged_standing_adds_nothing() {
        let mut account = Account::default();
        step(&mut account, ("GOLD", "II", 40), (10, 8), 1);
        assert!(step(&mut account, ("GOLD", "II", 40), (10, 8), 2).is_none());
        assert_eq!(account.points.len(), 1);
    }

    #[test]
    fn queues_are_tracked_apart() {
        let mut account = Account::default();
        step(&mut account, ("GOLD", "II", 40), (10, 8), 1);
        let flex = Standing {
            queue: "flex",
            tier: "SILVER".to_owned(),
            division: "I".to_owned(),
            lp: 10,
            wins: 3,
            losses: 3,
        };
        let fresh = account.observe(&flex, 2).unwrap();
        assert_eq!(fresh.outcome, None);
        assert_eq!(account.points[1].delta, None);
    }

    #[test]
    fn several_games_or_decay_have_a_delta_but_no_outcome() {
        let mut account = Account::default();
        step(&mut account, ("PLATINUM", "IV", 50), (10, 10), 1);
        step(&mut account, ("PLATINUM", "IV", 80), (12, 10), 2);
        assert_eq!(account.points[1].delta, Some(30));
        assert_eq!(account.points[1].outcome, None);
        step(&mut account, ("PLATINUM", "IV", 60), (12, 10), 3);
        assert_eq!(account.points[2].delta, Some(-20));
        assert_eq!(account.points[2].outcome, None);
    }

    #[test]
    fn new_season_starts_over() {
        let mut account = Account::default();
        step(&mut account, ("GOLD", "II", 40), (100, 90), 1);
        step(&mut account, ("SILVER", "II", 0), (1, 0), 2);
        assert_eq!(account.points[1].delta, None);
        assert_eq!(account.points[1].outcome, None);
    }

    #[test]
    fn old_points_are_dropped_first() {
        let mut account = Account::default();
        for at in 0..(MAX_POINTS as i64 + 5) {
            account.points.push(RankPoint {
                at,
                ..RankPoint::default()
            });
        }
        account.trim();
        assert_eq!(account.points.len(), MAX_POINTS);
        assert_eq!(account.points[0].at, 5);
    }

    #[test]
    fn unranked_queues_are_skipped() {
        let unranked = LcuRankedQueue {
            tier: "NONE".to_owned(),
            ..LcuRankedQueue::default()
        };
        let ranked = LcuRankedQueue {
            tier: "Gold".to_owned(),
            division: "iv".to_owned(),
            league_points: 12,
            ..LcuRankedQueue::default()
        };
        let mut queue_map = HashMap::new();
        queue_map.insert(String::from("RANKED_SOLO_5x5"), unranked);
        queue_map.insert(String::from("RANKED_FLEX_SR"), ranked);
        let found = standings(&LcuRankedStats { queue_map });
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].queue, "flex");
        assert_eq!(found[0].tier, "GOLD");
        assert_eq!(found[0].division, "IV");
    }
}
