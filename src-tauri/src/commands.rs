use serde::Serialize;
use tauri::{AppHandle, State};

use crate::clients::lcu::models::Summoner;
use crate::config::Settings;
use crate::error::Result;
use crate::logging;
use crate::services;
use crate::services::auto_accept::Pending;
use crate::services::champion_assist::{BuildVariant, ChampionBuild, MatchupReport, TierEntry};
use crate::services::game_data::GameData;
use crate::services::game_detail::{GameDetail, PlayerBuild};
use crate::services::match_history::MatchHistoryPage;
use crate::services::player_profile::{PlayerProfile, ProfileContext};
use crate::services::roster_insights::RosterInsights;
use crate::state::ongoing::Roster;
use crate::state::{AppState, LcuSnapshot};

#[derive(Serialize)]
pub struct AppInfo {
    name: &'static str,
    version: &'static str,
}

#[tauri::command]
pub fn app_info() -> AppInfo {
    AppInfo {
        name: env!("CARGO_PKG_NAME"),
        version: env!("CARGO_PKG_VERSION"),
    }
}

/// Current LCU state; later changes arrive as `lcu://snapshot` events.
#[tauri::command]
pub fn lcu_snapshot(state: State<'_, AppState>) -> LcuSnapshot {
    state.lcu_snapshot()
}

#[tauri::command]
pub async fn relaunch_as_admin(app: AppHandle) -> Result<()> {
    services::elevation::relaunch_as_admin(app).await
}

#[tauri::command]
pub async fn lookup_summoner(state: State<'_, AppState>, riot_id: String) -> Result<Summoner> {
    let session = state.session()?;
    services::summoner::by_riot_id(&session, &riot_id).await
}

#[tauri::command]
pub async fn summoner_by_puuid(state: State<'_, AppState>, puuid: String) -> Result<Summoner> {
    let session = state.session()?;
    services::summoner::by_puuid(&session, &puuid).await
}

#[tauri::command]
pub async fn match_history(
    state: State<'_, AppState>,
    puuid: String,
    start: u32,
    count: u32,
) -> Result<MatchHistoryPage> {
    let session = state.session()?;
    state
        .match_history
        .get(&session, &puuid, start, count)
        .await
}

#[tauri::command]
pub async fn game_data(state: State<'_, AppState>) -> Result<GameData> {
    let session = state.session()?;
    services::game_data::get(&session).await
}

/// Current game's players; later changes arrive as `ongoing://roster` events.
#[tauri::command]
pub fn ongoing_roster(state: State<'_, AppState>) -> Option<Roster> {
    state.roster()
}

/// Win/loss, averages and tags for one player, judged in the context of the current game.
#[tauri::command]
pub async fn player_profile(
    state: State<'_, AppState>,
    puuid: String,
    champion_id: i64,
    queue_id: i64,
    position: String,
) -> Result<PlayerProfile> {
    let session = state.session()?;
    let ctx = ProfileContext {
        champion_id,
        queue_id,
        position,
    };
    services::player_profile::load(&state.match_history, &session, &puuid, &ctx).await
}

/// Premades, "met before", early-game tags and the 上等马 / 下等马 comparison for the roster.
#[tauri::command]
pub async fn roster_insights(state: State<'_, AppState>) -> Result<RosterInsights> {
    services::roster_insights::load(&state).await
}

#[tauri::command]
pub fn settings(state: State<'_, AppState>) -> Settings {
    state.settings()
}

/// Turning auto accept off also cancels an accept that is already counting down.
#[tauri::command]
pub fn save_settings(
    app: AppHandle,
    state: State<'_, AppState>,
    settings: Settings,
) -> Result<Settings> {
    let saved = state.set_settings(settings)?;
    if !saved.auto_accept {
        services::auto_accept::cancel(&app);
    }
    Ok(saved)
}

/// Pending auto accept; later changes arrive as `auto-accept://state` events.
#[tauri::command]
pub fn auto_accept_state(state: State<'_, AppState>) -> Option<Pending> {
    state.auto_accept.pending()
}

#[tauri::command]
pub fn cancel_auto_accept(app: AppHandle) {
    services::auto_accept::cancel(&app);
}

#[tauri::command]
pub fn log_dir(app: AppHandle) -> Result<String> {
    Ok(logging::dir(&app)?.display().to_string())
}

#[tauri::command]
pub fn open_log_dir(app: AppHandle) -> Result<()> {
    logging::open_dir(&app)
}

/// Writes a frontend message (uncaught error, console warning) into the log file.
#[tauri::command]
pub fn log_frontend(level: String, message: String) {
    logging::frontend(&level, &message);
}

/// Every player of one finished game, for the expandable match history rows.
#[tauri::command]
pub async fn game_detail(state: State<'_, AppState>, game_id: i64) -> Result<GameDetail> {
    let session = state.session()?;
    state.game_details.get(&session, game_id).await
}

/// Champions ranked for a position, from lolalytics at the configured rank filter.
#[tauri::command]
pub async fn champion_tier_list(
    state: State<'_, AppState>,
    position: String,
) -> Result<Vec<TierEntry>> {
    let session = state.session()?;
    let tier = state.settings().stats_tier;
    let assist = &state.champion_assist;
    assist.tier_list(&session, &position, &tier).await
}

#[tauri::command]
pub async fn champion_build(
    state: State<'_, AppState>,
    champion_id: i64,
    position: String,
) -> Result<ChampionBuild> {
    let session = state.session()?;
    let tier = state.settings().stats_tier;
    let assist = &state.champion_assist;
    assist.build(&session, champion_id, &position, &tier).await
}

/// Writes a build's runes and summoner spells into the client.
#[tauri::command]
pub async fn apply_build(
    state: State<'_, AppState>,
    title: String,
    variant: BuildVariant,
) -> Result<()> {
    let session = state.session()?;
    services::champion_assist::apply(&session, &title, &variant).await
}

/// Same-lane matchups of a champion from high-rank games, including against the
/// opponents' locked picks.
#[tauri::command]
pub async fn champion_matchups(
    state: State<'_, AppState>,
    champion_id: i64,
    position: String,
    enemies: Vec<i64>,
) -> Result<MatchupReport> {
    let session = state.session()?;
    let tier = state.settings().matchup_tier;
    let assist = &state.champion_assist;
    assist
        .matchups(&session, champion_id, &position, &enemies, &tier)
        .await
}

/// Item purchase and skill order of every player in one game, from the SGP timeline.
#[tauri::command]
pub async fn game_builds(state: State<'_, AppState>, game_id: i64) -> Result<Vec<PlayerBuild>> {
    let session = state.session()?;
    state.game_details.builds(&session, game_id).await
}
