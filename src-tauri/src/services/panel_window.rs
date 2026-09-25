//! Brings the main window forward when champ select starts and when the game loads, so
//! the game panel is in view without alt-tabbing. The frontend switches to the panel page.

use tauri::{AppHandle, Manager};

use crate::state::AppState;

const MAIN_WINDOW: &str = "main";

pub fn on_phase(app: &AppHandle, phase: &str) {
    let enabled = app.state::<AppState>().settings().auto_show_panel;
    if !enabled || !matches!(phase, "ChampSelect" | "GameStart") {
        return;
    }
    let Some(window) = app.get_webview_window(MAIN_WINDOW) else {
        return;
    };
    let shown = window
        .unminimize()
        .and_then(|()| window.show())
        .and_then(|()| window.set_focus());
    match shown {
        Ok(()) => log::info!("showing game panel for {phase}"),
        Err(err) => log::warn!("failed to bring the window forward: {err}"),
    }
}
