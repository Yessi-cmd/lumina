use serde::Serialize;
use tauri::State;

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
