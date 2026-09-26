//! Enemy picks in champ select: which lane each opponent is likely to play and which
//! champions counter them. Opponents' identities are hidden during champ select on the
//! Chinese servers, but their locked champions and their bans are not.
//!
//! A lane is inferred from three signals: how often the picked champion plays each lane
//! (lolalytics), the champions the player banned (players usually ban threats to their
//! own lane), and the fact that the five opponents play five different lanes. Counters
//! come from high-rank matchup data and only count beyond the margin of error first.

use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, PoisonError};

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, Manager};

use super::champion_assist::{Matchup, Verdict, POSITIONS};
use crate::clients::lcu::models::{LcuEvent, LcuEventType};
use crate::state::session::LcuSession;
use crate::state::AppState;

pub const DRAFT_EVENT: &str = "overlay://draft";
const CHAMP_SELECT_SESSION: &str = "/lol-champ-select/v1/session";

mod limits {
    /// Added to every lane share, so a champion never rules a lane out completely.
    pub const LANE_SMOOTHING: f64 = 0.02;
    /// How strongly banning a lane's champion points to the banner's own lane.
    pub const BAN_WEIGHT: f64 = 1.5;
    /// A lane the client assigned outweighs every other signal.
    pub const ASSIGNED_OTHER_LANES: f64 = 0.001;
    /// Counter picks shown per opponent.
    pub const COUNTERS: usize = 4;
}

/// Latest analysis, for overlays that open after it was emitted.
static LATEST: Mutex<Option<Draft>> = Mutex::new(None);
/// Champ-select state the latest analysis started from; unchanged events are skipped.
static SIGNATURE: Mutex<String> = Mutex::new(String::new());
/// Bumped per analysis, so a slow one never overwrites a newer result.
static GENERATION: AtomicU64 = AtomicU64::new(0);

#[derive(Debug, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
struct DraftSession {
    game_id: i64,
    is_spectating: bool,
    my_team: Vec<Member>,
    their_team: Vec<Member>,
    actions: Vec<Vec<Action>>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
struct Member {
    cell_id: i64,
    champion_id: i64,
    /// `top` / `jungle` / `middle` / `bottom` / `utility`; empty when not shown.
    assigned_position: String,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
struct Action {
    actor_cell_id: i64,
    champion_id: i64,
    #[serde(rename = "type")]
    kind: String,
    completed: bool,
    is_ally_action: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Draft {
    pub game_id: i64,
    pub enemies: Vec<EnemySlot>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EnemySlot {
    /// Pick order within the enemy team, from 1.
    pub floor: usize,
    /// 0 until locked in.
    pub champion_id: i64,
    /// Champions this player banned.
    pub bans: Vec<i64>,
    /// Every lane with its probability, most likely first.
    pub lanes: Vec<LaneGuess>,
    pub advice: Advice,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LaneGuess {
    pub position: String,
    pub probability: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum Advice {
    /// Nothing locked in yet.
    Waiting,
    Loading,
    /// Every teammate has locked in, so nobody can answer this pick any more.
    NoPicksLeft,
    /// The teammate in that lane has locked in; how their matchup looks, in their favour.
    AllyLocked {
        ally_champion_id: i64,
        win_rate: Option<f64>,
        games: i64,
    },
    Counters {
        /// Pick order of the teammate in that lane, when positions are assigned.
        ally_floor: Option<usize>,
        picks: Vec<CounterPick>,
    },
    Unavailable {
        reason: String,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CounterPick {
    pub champion_id: i64,
    /// The counter's win rate against the enemy champion, percent.
    pub win_rate: f64,
    pub games: i64,
    /// Beyond the 95% margin of error.
    pub significant: bool,
}

/// Share of each champion's games in each of `POSITIONS`.
pub type LaneShares = HashMap<i64, [f64; 5]>;

/// Everything the counter advice needs besides the matchup data.
struct Board {
    draft: Draft,
    allies: Vec<AllySeat>,
    /// Banned or picked by anyone.
    unavailable: HashSet<i64>,
    ally_picks_left: usize,
}

struct AllySeat {
    floor: usize,
    /// One of `POSITIONS`, or empty.
    position: String,
    champion_id: i64,
    locked: bool,
}

pub fn on_champ_select(app: &AppHandle, event: &LcuEvent) {
    if event.event_type == LcuEventType::Delete || !enabled(app) {
        return;
    }
    match serde_json::from_value::<DraftSession>(event.data.clone()) {
        Ok(session) => update(app, session),
        Err(err) => log::debug!("unexpected champ select payload for the draft: {err}"),
    }
}

pub fn on_phase(app: &AppHandle, phase: &str) {
    if phase == "ChampSelect" && enabled(app) {
        tauri::async_runtime::spawn(load(app.clone()));
    } else {
        clear(app);
    }
}

pub fn latest() -> Option<Draft> {
    let latest = LATEST.lock().unwrap_or_else(PoisonError::into_inner);
    latest.clone()
}

fn enabled(app: &AppHandle) -> bool {
    app.state::<AppState>().settings().champ_select_overlay
}

/// Catches up when champ select was already running.
async fn load(app: AppHandle) {
    let Ok(session) = app.state::<AppState>().session() else {
        return;
    };
    let request = session.http.get::<DraftSession>(CHAMP_SELECT_SESSION);
    match request.await {
        Ok(draft) => update(&app, draft),
        Err(err) => log::debug!("no champ select session for the draft: {err}"),
    }
}

fn update(app: &AppHandle, session: DraftSession) {
    if session.is_spectating {
        return;
    }
    let teams = (&session.my_team, &session.their_team);
    let signature = format!("{} {teams:?} {:?}", session.game_id, session.actions);
    {
        let mut last = SIGNATURE.lock().unwrap_or_else(PoisonError::into_inner);
        if *last == signature {
            return;
        }
        *last = signature;
    }
    let generation = GENERATION.fetch_add(1, Ordering::SeqCst) + 1;
    tauri::async_runtime::spawn(analyze(app.clone(), session, generation));
}

fn clear(app: &AppHandle) {
    let generation = GENERATION.fetch_add(1, Ordering::SeqCst) + 1;
    let mut signature = SIGNATURE.lock().unwrap_or_else(PoisonError::into_inner);
    signature.clear();
    drop(signature);
    if latest().is_some() {
        publish(app, generation, None);
    }
}

async fn analyze(app: AppHandle, session: DraftSession, generation: u64) {
    let state = app.state::<AppState>();
    let Ok(lcu) = state.session() else {
        return;
    };
    let settings = state.settings();
    let assist = &state.champion_assist;
    let shares = match assist.lane_shares(&settings.stats_tier).await {
        Ok(shares) => shares,
        Err(err) => {
            log::warn!("lane shares unavailable, lanes from bans only: {err}");
            LaneShares::new()
        }
    };
    let board = board(&session, &shares);
    publish(&app, generation, Some(board.draft.clone()));

    let mut draft = board.draft.clone();
    for enemy in &mut draft.enemies {
        if enemy.advice == Advice::Loading {
            let tier = &settings.matchup_tier;
            let advice = advise(&state, &lcu, &board, tier, enemy).await;
            enemy.advice = advice;
        }
    }
    publish(&app, generation, Some(draft));
}

fn publish(app: &AppHandle, generation: u64, draft: Option<Draft>) {
    if GENERATION.load(Ordering::SeqCst) != generation {
        return;
    }
    *LATEST.lock().unwrap_or_else(PoisonError::into_inner) = draft.clone();
    if let Err(err) = app.emit(DRAFT_EVENT, &draft) {
        log::warn!("failed to emit {DRAFT_EVENT}: {err}");
    }
}

fn board(session: &DraftSession, shares: &LaneShares) -> Board {
    let their_cells: HashSet<i64> = session.their_team.iter().map(|m| m.cell_id).collect();
    let mut bans_by: HashMap<i64, Vec<i64>> = HashMap::new();
    let mut unavailable = HashSet::new();
    let mut locked_cells = HashSet::new();
    let mut ally_picks_left = 0;
    for action in session.actions.iter().flatten() {
        let done = action.completed && action.champion_id > 0;
        match action.kind.as_str() {
            "ban" if done => {
                unavailable.insert(action.champion_id);
                if their_cells.contains(&action.actor_cell_id) {
                    let bans = bans_by.entry(action.actor_cell_id).or_default();
                    bans.push(action.champion_id);
                }
            }
            "pick" if done => {
                locked_cells.insert(action.actor_cell_id);
            }
            "pick" if action.is_ally_action && !action.completed => ally_picks_left += 1,
            _ => {}
        }
    }
    for member in session.my_team.iter().chain(&session.their_team) {
        if member.champion_id > 0 {
            unavailable.insert(member.champion_id);
        }
    }

    let mut allies = Vec::new();
    for (i, member) in by_cell(&session.my_team).into_iter().enumerate() {
        allies.push(AllySeat {
            floor: i + 1,
            position: position_of(&member.assigned_position).to_owned(),
            champion_id: member.champion_id,
            locked: locked_cells.contains(&member.cell_id),
        });
    }

    let enemies = by_cell(&session.their_team);
    let mut weights = Vec::new();
    for member in &enemies {
        let bans = bans_by.get(&member.cell_id).map_or(&[][..], Vec::as_slice);
        weights.push(lane_weights(member, bans, shares));
    }
    let marginals = lane_marginals(&weights);
    let mut slots = Vec::new();
    for (i, (member, probabilities)) in enemies.iter().zip(marginals).enumerate() {
        let advice = if member.champion_id > 0 {
            Advice::Loading
        } else {
            Advice::Waiting
        };
        slots.push(EnemySlot {
            floor: i + 1,
            champion_id: member.champion_id,
            bans: bans_by.remove(&member.cell_id).unwrap_or_default(),
            lanes: ranked_lanes(probabilities),
            advice,
        });
    }

    Board {
        draft: Draft {
            game_id: session.game_id,
            enemies: slots,
        },
        allies,
        unavailable,
        ally_picks_left,
    }
}

fn by_cell(team: &[Member]) -> Vec<&Member> {
    let mut members: Vec<&Member> = team.iter().collect();
    members.sort_by_key(|m| m.cell_id);
    members
}

async fn advise(
    state: &AppState,
    lcu: &LcuSession,
    board: &Board,
    tier: &str,
    enemy: &EnemySlot,
) -> Advice {
    let Some(lane) = enemy.lanes.first() else {
        return Advice::Unavailable {
            reason: "无法判断分路".to_owned(),
        };
    };
    let position = lane.position.as_str();
    let assist = &state.champion_assist;
    let request = assist.matchup_rows(lcu, enemy.champion_id, position, tier);
    let rows = match request.await {
        Ok(rows) => rows,
        Err(err) => {
            log::debug!("no matchups for champion {}: {err}", enemy.champion_id);
            return Advice::Unavailable {
                reason: "暂无对位数据".to_owned(),
            };
        }
    };
    let ally = board.allies.iter().find(|a| a.position == position);
    if let Some(ally) = ally.filter(|a| a.locked && a.champion_id > 0) {
        let row = rows.iter().find(|m| m.champion_id == ally.champion_id);
        return Advice::AllyLocked {
            ally_champion_id: ally.champion_id,
            win_rate: row.map(|m| 100.0 - m.win_rate),
            games: row.map_or(0, |m| m.games),
        };
    }
    if board.ally_picks_left == 0 {
        return Advice::NoPicksLeft;
    }
    Advice::Counters {
        ally_floor: ally.map(|a| a.floor),
        picks: counter_picks(&rows, &board.unavailable),
    }
}

/// `rows` are the enemy champion's matchups: a row it loses is a counter. Counters
/// beyond the margin of error come first, then the enemy's other losing matchups.
fn counter_picks(rows: &[Matchup], unavailable: &HashSet<i64>) -> Vec<CounterPick> {
    let mut candidates = Vec::new();
    for row in rows {
        let enough = row.verdict != Verdict::TooFewGames;
        if enough && row.advantage < 0.0 && !unavailable.contains(&row.champion_id) {
            candidates.push(row);
        }
    }
    candidates.sort_by(|a, b| {
        let clear = |m: &Matchup| m.verdict != Verdict::Countered;
        clear(a)
            .cmp(&clear(b))
            .then(a.advantage.total_cmp(&b.advantage))
    });
    let mut picks = Vec::new();
    for row in candidates.into_iter().take(limits::COUNTERS) {
        picks.push(CounterPick {
            champion_id: row.champion_id,
            win_rate: 100.0 - row.win_rate,
            games: row.games,
            significant: row.verdict == Verdict::Countered,
        });
    }
    picks
}

/// Unnormalized likelihood of each lane for one opponent on their own.
fn lane_weights(member: &Member, bans: &[i64], shares: &LaneShares) -> [f64; 5] {
    let assigned = position_of(&member.assigned_position);
    if let Some(i) = POSITIONS.iter().position(|p| *p == assigned) {
        let mut weights = [limits::ASSIGNED_OTHER_LANES; 5];
        weights[i] = 1.0;
        return weights;
    }
    let mut weights = [1.0; 5];
    if let Some(share) = shares.get(&member.champion_id) {
        for (w, s) in weights.iter_mut().zip(share) {
            *w *= s + limits::LANE_SMOOTHING;
        }
    }
    for ban in bans {
        if let Some(share) = shares.get(ban) {
            for (w, s) in weights.iter_mut().zip(share) {
                *w *= 1.0 + limits::BAN_WEIGHT * s;
            }
        }
    }
    weights
}

/// Probability of each opponent playing each lane, given that no two share a lane:
/// every assignment of distinct lanes is weighed by the product of its likelihoods.
fn lane_marginals(weights: &[[f64; 5]]) -> Vec<[f64; 5]> {
    let mut search = Assignments {
        weights,
        marginals: vec![[0.0; 5]; weights.len()],
        total: 0.0,
    };
    let depth = weights.len().min(5);
    search.visit(&mut Vec::with_capacity(depth), 1.0, depth);
    if search.total > 0.0 {
        for row in &mut search.marginals {
            for p in row.iter_mut() {
                *p /= search.total;
            }
        }
    }
    search.marginals
}

struct Assignments<'a> {
    weights: &'a [[f64; 5]],
    marginals: Vec<[f64; 5]>,
    total: f64,
}

impl Assignments<'_> {
    fn visit(&mut self, lanes: &mut Vec<usize>, weight: f64, depth: usize) {
        let member = lanes.len();
        if member == depth {
            self.total += weight;
            for (m, lane) in lanes.iter().enumerate() {
                self.marginals[m][*lane] += weight;
            }
            return;
        }
        for lane in 0..5 {
            if lanes.contains(&lane) {
                continue;
            }
            lanes.push(lane);
            let next = weight * self.weights[member][lane];
            self.visit(lanes, next, depth);
            lanes.pop();
        }
    }
}

fn ranked_lanes(probabilities: [f64; 5]) -> Vec<LaneGuess> {
    let mut lanes = Vec::new();
    for (position, probability) in POSITIONS.iter().zip(probabilities) {
        lanes.push(LaneGuess {
            position: (*position).to_owned(),
            probability,
        });
    }
    lanes.sort_by(|a, b| b.probability.total_cmp(&a.probability));
    lanes
}

/// LCU `assignedPosition` to one of `POSITIONS`; empty when unknown.
fn position_of(assigned: &str) -> &'static str {
    let upper = assigned.to_ascii_uppercase();
    let found = POSITIONS.iter().find(|p| **p == upper);
    found.copied().unwrap_or("")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn member(cell_id: i64, champion_id: i64) -> Member {
        Member {
            cell_id,
            champion_id,
            assigned_position: String::new(),
        }
    }

    fn action(actor: i64, champion: i64, kind: &str, completed: bool, ally: bool) -> Action {
        Action {
            actor_cell_id: actor,
            champion_id: champion,
            kind: kind.to_owned(),
            completed,
            is_ally_action: ally,
        }
    }

    /// Champion `n` plays lane `n % 5` almost exclusively.
    fn shares() -> LaneShares {
        let mut shares = LaneShares::new();
        for champion in 1..=20 {
            let mut row = [0.02; 5];
            row[champion as usize % 5] = 0.92;
            shares.insert(champion, row);
        }
        shares
    }

    #[test]
    fn distinct_lanes_resolve_a_flex_pick() {
        // Cell 5 locked a champion that plays mid or top equally; cell 6 is a clear mid.
        let mut shares = shares();
        shares.insert(99, [0.5, 0.0, 0.5, 0.0, 0.0]);
        let session = DraftSession {
            game_id: 1,
            their_team: vec![member(5, 99), member(6, 2)],
            ..DraftSession::default()
        };
        let board = board(&session, &shares);
        let flex = &board.draft.enemies[0];
        assert_eq!(flex.lanes[0].position, "TOP");
        assert!(flex.lanes[0].probability > 0.7);
    }

    #[test]
    fn a_ban_points_to_the_banners_lane() {
        let session = DraftSession {
            their_team: vec![member(5, 0), member(6, 0)],
            actions: vec![vec![action(5, 4, "ban", true, false)]],
            ..DraftSession::default()
        };
        let board = board(&session, &shares());
        let banner = &board.draft.enemies[0];
        assert_eq!(banner.bans, vec![4]);
        assert_eq!(banner.lanes[0].position, "UTILITY");
        assert_eq!(banner.advice, Advice::Waiting);
        assert!(board.unavailable.contains(&4));
    }

    #[test]
    fn counts_teammate_picks_still_to_come() {
        let session = DraftSession {
            my_team: vec![member(0, 1), member(1, 0)],
            their_team: vec![member(5, 3)],
            actions: vec![vec![
                action(0, 1, "pick", true, true),
                action(5, 3, "pick", true, false),
                action(1, 0, "pick", false, true),
            ]],
            ..DraftSession::default()
        };
        let board = board(&session, &shares());
        assert_eq!(board.ally_picks_left, 1);
        assert!(board.allies[0].locked && !board.allies[1].locked);
        assert_eq!(board.draft.enemies[0].advice, Advice::Loading);
    }

    fn matchup(champion_id: i64, advantage: f64, verdict: Verdict) -> Matchup {
        Matchup {
            champion_id,
            win_rate: 50.0 + advantage,
            games: 1000,
            advantage,
            margin: 2.0,
            verdict,
            usual_position: "MIDDLE".to_owned(),
        }
    }

    #[test]
    fn clear_counters_come_first_and_unavailable_ones_are_skipped() {
        let rows = vec![
            matchup(1, -1.5, Verdict::Even),
            matchup(2, -4.0, Verdict::Countered),
            matchup(3, -6.0, Verdict::Countered),
            matchup(4, 3.0, Verdict::Counters),
            matchup(5, -9.0, Verdict::TooFewGames),
        ];
        let unavailable = HashSet::from([3]);
        let picks = counter_picks(&rows, &unavailable);
        let ids: Vec<i64> = picks.iter().map(|p| p.champion_id).collect();
        assert_eq!(ids, vec![2, 1]);
        assert!(picks[0].significant && !picks[1].significant);
        assert!((picks[0].win_rate - 54.0).abs() < 1e-9);
    }
}
