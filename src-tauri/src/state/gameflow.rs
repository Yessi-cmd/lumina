//! `/lol-gameflow/v1/gameflow-phase`:
//! `None → Lobby → Matchmaking → ReadyCheck → ChampSelect → InProgress → EndOfGame`

use serde::Serialize;
use tauri::Emitter;

use super::AppState;

pub const PHASE_NONE: &str = "None";
pub const GAMEFLOW_PHASE_EVENT: &str = "lcu://gameflow-phase";

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PhaseChange {
    pub phase: String,
    pub previous: String,
}

impl AppState {
    pub fn set_gameflow_phase(&self, phase: String) {
        let mut previous = None;
        self.update_lcu(|s| {
            if s.gameflow_phase != phase {
                previous = Some(std::mem::replace(&mut s.gameflow_phase, phase.clone()));
            }
        });
        if let Some(previous) = previous {
            log::info!("gameflow phase: {previous} -> {phase}");
            let change = PhaseChange { phase, previous };
            if let Err(err) = self.app.emit(GAMEFLOW_PHASE_EVENT, &change) {
                log::warn!("failed to emit {GAMEFLOW_PHASE_EVENT}: {err}");
            }
        }
    }
}
