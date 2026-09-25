//! Application state owned by the backend. The frontend only receives snapshots.

pub mod gameflow;
pub mod ongoing;
pub mod session;

use std::sync::{Arc, Mutex, PoisonError, RwLock};

use serde::Serialize;
use tauri::{AppHandle, Emitter};

use crate::clients::lcu::discovery::CredentialSource;
use crate::clients::lcu::models::Summoner;
use crate::error::{AppError, Result};
use crate::services::match_history::MatchHistoryService;
use crate::services::timeline::TimelineService;
use ongoing::Roster;
use session::LcuSession;

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
    /// Display name of the SGP server, e.g. 艾欧尼亚; `None` means LCU-only.
    pub sgp_server: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LcuSnapshot {
    pub status: ConnectionStatus,
    pub client: Option<ClientInfo>,
    pub summoner: Option<Summoner>,
    pub gameflow_phase: String,
    pub last_error: Option<String>,
    /// A client is running but only an elevated Lumina can read its credentials.
    pub needs_admin: bool,
}

impl Default for LcuSnapshot {
    fn default() -> Self {
        Self {
            status: ConnectionStatus::Disconnected,
            client: None,
            summoner: None,
            gameflow_phase: gameflow::PHASE_NONE.to_owned(),
            last_error: None,
            needs_admin: false,
        }
    }
}

pub struct AppState {
    app: AppHandle,
    lcu: Mutex<LcuSnapshot>,
    session: RwLock<Option<Arc<LcuSession>>>,
    roster: Mutex<Option<Roster>>,
    pub match_history: MatchHistoryService,
    pub timelines: TimelineService,
}

impl AppState {
    pub fn new(app: AppHandle) -> Self {
        Self {
            app,
            lcu: Mutex::default(),
            session: RwLock::default(),
            roster: Mutex::default(),
            match_history: MatchHistoryService::default(),
            timelines: TimelineService::default(),
        }
    }

    /// The live client connection, for commands that talk to LCU or SGP.
    pub fn session(&self) -> Result<Arc<LcuSession>> {
        let guard = self.session.read().unwrap_or_else(PoisonError::into_inner);
        guard.clone().ok_or(AppError::NotConnected)
    }

    pub fn set_session(&self, session: Option<Arc<LcuSession>>) {
        let mut guard = self.session.write().unwrap_or_else(PoisonError::into_inner);
        *guard = session;
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
        self.set_session(None);
        self.set_roster(None);
        self.update_lcu(|s| {
            *s = LcuSnapshot {
                last_error,
                ..LcuSnapshot::default()
            };
        });
    }

    pub fn mark_needs_admin(&self, hint: &str) {
        self.set_session(None);
        self.set_roster(None);
        self.update_lcu(|s| {
            *s = LcuSnapshot {
                last_error: Some(hint.to_owned()),
                needs_admin: true,
                ..LcuSnapshot::default()
            };
        });
    }
}
