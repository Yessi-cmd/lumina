//! Application state owned by the backend. The frontend only receives snapshots.

pub mod gameflow;

use std::sync::{Mutex, PoisonError};

use serde::Serialize;
use tauri::{AppHandle, Emitter};

use crate::clients::lcu::discovery::CredentialSource;
use crate::clients::lcu::models::Summoner;

pub const LCU_SNAPSHOT_EVENT: &str = "lcu://snapshot";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum ConnectionStatus {
    #[default]
    Disconnected,
    Connecting,
    Connected,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ClientInfo {
    pub pid: u32,
    pub port: u16,
    pub platform_id: Option<String>,
    pub source: CredentialSource,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LcuSnapshot {
    pub status: ConnectionStatus,
    pub client: Option<ClientInfo>,
    pub summoner: Option<Summoner>,
    pub gameflow_phase: String,
    pub last_error: Option<String>,
}

impl Default for LcuSnapshot {
    fn default() -> Self {
        Self {
            status: ConnectionStatus::Disconnected,
            client: None,
            summoner: None,
            gameflow_phase: gameflow::PHASE_NONE.to_owned(),
            last_error: None,
        }
    }
}

pub struct AppState {
    app: AppHandle,
    lcu: Mutex<LcuSnapshot>,
}

impl AppState {
    pub fn new(app: AppHandle) -> Self {
        Self {
            app,
            lcu: Mutex::default(),
        }
    }

    pub fn lcu_snapshot(&self) -> LcuSnapshot {
        let guard = self.lcu.lock().unwrap_or_else(PoisonError::into_inner);
        guard.clone()
    }

    /// Mutates the LCU snapshot and pushes it to the frontend if anything changed.
    pub fn update_lcu(&self, f: impl FnOnce(&mut LcuSnapshot)) {
        let snapshot = {
            let mut guard = self.lcu.lock().unwrap_or_else(PoisonError::into_inner);
            let before = guard.clone();
            f(&mut guard);
            if *guard == before {
                return;
            }
            guard.clone()
        };
        if let Err(err) = self.app.emit(LCU_SNAPSHOT_EVENT, &snapshot) {
            log::warn!("failed to emit {LCU_SNAPSHOT_EVENT}: {err}");
        }
    }

    /// Back to `Disconnected`, keeping only an optional error for the UI.
    pub fn reset_lcu(&self, last_error: Option<String>) {
        self.update_lcu(|s| {
            *s = LcuSnapshot {
                last_error,
                ..LcuSnapshot::default()
            };
        });
    }
}
