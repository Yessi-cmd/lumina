use serde::Serialize;
use tauri::{AppHandle, State};

use crate::clients::lcu::models::Summoner;
use crate::error::Result;
use crate::services;
use crate::services::game_data::GameData;
use crate::services::match_history::MatchHistoryPage;
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
    services::summoner::by_riot_id(&state.session()?, &riot_id).await
}

#[tauri::command]
pub async fn summoner_by_puuid(state: State<'_, AppState>, puuid: String) -> Result<Summoner> {
    services::summoner::by_puuid(&state.session()?, &puuid).await
}

#[tauri::command]
pub async fn match_history(
    state: State<'_, AppState>,
    puuid: String,
    start: u32,
    count: u32,
) -> Result<MatchHistoryPage> {
    let session = state.session()?;
    state.match_history.get(&session, &puuid, start, count).await
}

#[tauri::command]
pub async fn game_data(state: State<'_, AppState>) -> Result<GameData> {
    services::game_data::get(&state.session()?).await
}
