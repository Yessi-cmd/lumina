//! Everything the game panel derives from the roster as a whole: premade groups and
//! "met before" (`roster_relations`), early-game habits from timelines (`timeline`), and
//! the 上等马 / 下等马 comparison with advice (`matchup`).

use std::collections::HashMap;
use std::sync::Arc;

use serde::Serialize;

use super::match_history::{DataSource, GameResult, GameSummary, MatchHistoryPage};
use super::matchup::{self, Advice, LaneMatchup, PlayerPower};
use super::ongoing_game::PANEL_HISTORY_COUNT;
use super::player_profile::{self, PlayerProfile, PlayerTag, ProfileContext};
use super::roster_relations::{self, PremadeGroup};
use super::timeline::{EarlyGame, EarlyStats, GameDigest};
use crate::error::Result;
use crate::state::ongoing::Roster;
use crate::state::session::LcuSession;
use crate::state::AppState;

/// Recent Rift games per player whose timelines are read.
const TIMELINE_GAMES: usize = 6;
/// Summoner's Rift queues; other modes have no lanes or junglers.
const RIFT_QUEUES: [i64; 6] = [400, 420, 430, 440, 490, 700];

#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RosterInsights {
    pub premades: Vec<PremadeGroup>,
    /// Roster-wide tags per puuid; the frontend merges them with profile tags.
    pub tags: HashMap<String, Vec<PlayerTag>>,
    pub powers: HashMap<String, PlayerPower>,
    pub lanes: Vec<LaneMatchup>,
    pub advice: Vec<Advice>,
}

pub async fn load(state: &AppState) -> Result<RosterInsights> {
    let Some(roster) = state.roster() else {
        return Ok(RosterInsights::default());
    };
    let session = state.session()?;
    let summoner = state.lcu_snapshot().summoner;
    let self_puuid = summoner.map(|s| s.puuid).unwrap_or_default();

    let mut puuids: Vec<String> = roster.puuids().map(str::to_owned).collect();
    if !self_puuid.is_empty() && !puuids.contains(&self_puuid) {
        puuids.push(self_puuid.clone());
    }
    let history = &state.match_history;
    let requests = puuids
        .iter()
        .map(|puuid| history.get(&session, puuid, 0, PANEL_HISTORY_COUNT));
    let results = futures_util::future::join_all(requests).await;
    let pages: Vec<MatchHistoryPage> = results.into_iter().flatten().collect();

    let now = roster_relations::now_ms();
    let relations = roster_relations::analyze(&roster, &self_puuid, &pages, now);
    let profiles = profiles(&roster, &pages);
    let early = early_stats(state, &session, &roster, &pages).await;
    let matchup = matchup::analyze(&roster, &profiles, &early);

    let from_sgp = pages.iter().filter(|p| p.source == DataSource::Sgp);
    let sgp_pages = from_sgp.count();
    log::info!(
        "insights: {} histories ({sgp_pages} from SGP), {} rated, {} lanes, {} advice",
        pages.len(),
        matchup.powers.len(),
        matchup.lanes.len(),
        matchup.advice.len()
    );

    let mut tags = relations.tags;
    for (puuid, extra) in matchup.tags {
        tags.entry(puuid).or_default().extend(extra);
    }
    Ok(RosterInsights {
        premades: relations.premades,
        tags,
        powers: matchup.powers,
        lanes: matchup.lanes,
        advice: matchup.advice,
    })
}

/// Profiles judged in the context of this game, as the player cards show them.
fn profiles(roster: &Roster, pages: &[MatchHistoryPage]) -> HashMap<String, PlayerProfile> {
    let mut out = HashMap::new();
    for player in roster.allies.iter().chain(&roster.enemies) {
        let Some(page) = pages.iter().find(|p| p.puuid == player.puuid) else {
            continue;
        };
        let ctx = ProfileContext {
            champion_id: player.champion_id,
            queue_id: roster.queue_id,
            position: player.position.clone(),
        };
        out.insert(player.puuid.clone(), player_profile::build(page, &ctx));
    }
    out
}

/// Reads the timelines of each player's recent Rift games (shared games once) and
/// averages their early game. Without SGP there are no timelines and no early stats.
async fn early_stats(
    state: &AppState,
    session: &LcuSession,
    roster: &Roster,
    pages: &[MatchHistoryPage],
) -> HashMap<String, EarlyStats> {
    if session.sgp.is_none() {
        return HashMap::new();
    }
    let mut wanted: HashMap<String, Vec<&GameSummary>> = HashMap::new();
    let mut unique: HashMap<i64, &GameSummary> = HashMap::new();
    for puuid in roster.puuids() {
        let Some(page) = pages.iter().find(|p| p.puuid == puuid) else {
            continue;
        };
        let rift = page.games.iter().filter(|g| on_rift(g));
        let games: Vec<&GameSummary> = rift.take(TIMELINE_GAMES).collect();
        for game in &games {
            unique.entry(game.game_id).or_insert(*game);
        }
        wanted.insert(puuid.to_owned(), games);
    }

    let timelines = &state.timelines;
    let requests = unique.values().map(|game| async move {
        let digest = timelines.digest(session, game).await;
        (game.game_id, digest)
    });
    let mut digests: HashMap<i64, Arc<GameDigest>> = HashMap::new();
    let mut failures = Vec::new();
    for (game_id, result) in futures_util::future::join_all(requests).await {
        match result {
            Ok(digest) => {
                digests.insert(game_id, digest);
            }
            Err(err) => failures.push(format!("{game_id}: {err}")),
        }
    }
    let loaded = digests.len();
    match failures.first() {
        Some(first) => {
            let failed = failures.len();
            log::warn!("timelines: {loaded} loaded, {failed} unavailable, first: {first}");
        }
        None => log::info!("timelines: {loaded} loaded"),
    }

    let mut out = HashMap::new();
    for (puuid, games) in wanted {
        let mut early: Vec<&EarlyGame> = Vec::new();
        for game in games {
            if let Some(e) = digests.get(&game.game_id).and_then(|d| d.get(&puuid)) {
                early.push(e);
            }
        }
        out.insert(puuid, EarlyStats::from_games(early.into_iter()));
    }
    out
}

/// Finished Rift games that list their participants (SGP pages only).
fn on_rift(game: &GameSummary) -> bool {
    let finished = matches!(game.result, GameResult::Win | GameResult::Loss);
    finished && RIFT_QUEUES.contains(&game.queue_id) && !game.participants.is_empty()
}
