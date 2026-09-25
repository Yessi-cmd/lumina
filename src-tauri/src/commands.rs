use serde::Serialize;
use tauri::{AppHandle, State};

use crate::error::Result;
use crate::services;
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
