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

/// Solo/duo and flex: normal, ARAM and bot games say little about how someone plays
/// ranked, so the power index, laning and gank habits read only these.
const RANKED_QUEUES: [i64; 2] = [420, 440];
/// Ranked games per player, fetched on top of the shared page so players who mostly
/// play other modes still get a ranked sample.
const RANKED_GAMES: u32 = 20;
/// Recent ranked games per player whose timelines are read.
const TIMELINE_GAMES: usize = 8;

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
    let ranked = ranked_games(state, &session, &roster, &pages).await;
    let profiles = profiles(&roster, &pages, &ranked);
    let early = early_stats(state, &session, &ranked).await;
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

/// Each roster player's recent ranked games, newest first: the shared page plus one
/// page per ranked queue, which SGP filters on the server.
async fn ranked_games(
    state: &AppState,
    session: &LcuSession,
    roster: &Roster,
    pages: &[MatchHistoryPage],
) -> HashMap<String, Vec<GameSummary>> {
    let history = &state.match_history;
    let requests = roster.puuids().map(|puuid| async move {
        let mut games = Vec::new();
        if let Some(page) = pages.iter().find(|p| p.puuid == puuid) {
            games.extend(page.games.iter().cloned());
        }
        if session.sgp.is_some() {
            for queue in RANKED_QUEUES {
                let page = history.get_queue(session, puuid, 0, RANKED_GAMES, Some(queue));
                match page.await {
                    Ok(page) => games.extend(page.games),
                    Err(err) => log::debug!("ranked history for queue {queue} failed: {err}"),
                }
            }
        }
        games.retain(|g| RANKED_QUEUES.contains(&g.queue_id));
        games.sort_by_key(|g| std::cmp::Reverse(g.created_at));
        games.dedup_by_key(|g| g.game_id);
        games.truncate(RANKED_GAMES as usize);
        (puuid.to_owned(), games)
    });
    let results = futures_util::future::join_all(requests).await;
    results.into_iter().collect()
}

/// Profiles judged in the context of this game, with the power index's ranked form
/// taken from the ranked games.
fn profiles(
    roster: &Roster,
    pages: &[MatchHistoryPage],
    ranked: &HashMap<String, Vec<GameSummary>>,
) -> HashMap<String, PlayerProfile> {
    let mut out = HashMap::new();
    for player in roster.allies.iter().chain(&roster.enemies) {
        let Some(page) = pages.iter().find(|p| p.puuid == player.puuid) else {
            continue;
        };
        let ctx = ProfileContext {
            champion_id: player.champion_id,
            queue_id: roster.queue_id,
            position: player.position.clone(),
            // Only the power index reads these profiles, and it ignores the champion.
            champion_points: None,
        };
        let mut profile = player_profile::build(page, &ctx);
        if let Some(games) = ranked.get(&player.puuid) {
            let games: Vec<&GameSummary> = games.iter().collect();
            profile.form = player_profile::ranked_form(&games, &player.position);
        }
        out.insert(player.puuid.clone(), profile);
    }
    out
}

/// Reads the timelines of each player's recent ranked games (shared games once) and
/// averages their early game. Without SGP there are no timelines and no early stats.
async fn early_stats(
    state: &AppState,
    session: &LcuSession,
    ranked: &HashMap<String, Vec<GameSummary>>,
) -> HashMap<String, EarlyStats> {
    if session.sgp.is_none() {
        return HashMap::new();
    }
    let mut wanted: HashMap<String, Vec<&GameSummary>> = HashMap::new();
    let mut unique: HashMap<i64, &GameSummary> = HashMap::new();
    for (puuid, games) in ranked {
        let with_timeline = games.iter().filter(|g| has_timeline(g));
        let games: Vec<&GameSummary> = with_timeline.take(TIMELINE_GAMES).collect();
        for game in &games {
            unique.entry(game.game_id).or_insert(*game);
        }
        wanted.insert(puuid.clone(), games);
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

/// Finished ranked games that list their participants (SGP pages only).
fn has_timeline(game: &GameSummary) -> bool {
    let finished = matches!(game.result, GameResult::Win | GameResult::Loss);
    finished && RANKED_QUEUES.contains(&game.queue_id) && !game.participants.is_empty()
}
