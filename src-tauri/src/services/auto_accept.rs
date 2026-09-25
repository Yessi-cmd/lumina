//! Accepts the "match found" popup after the configured delay, unless the user answers it
//! first, cancels, turns the feature off, or the ready check ends.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, PoisonError};
use std::time::Duration;

use serde::Serialize;
use serde_json::Value;
use tauri::{AppHandle, Emitter, Manager};

use super::roster_relations::now_ms;
use crate::clients::lcu::models::LcuEvent;
use crate::state::AppState;

pub const READY_CHECK: &str = "/lol-matchmaking/v1/ready-check";
const ACCEPT: &str = "/lol-matchmaking/v1/ready-check/accept";
pub const AUTO_ACCEPT_EVENT: &str = "auto-accept://state";

/// An accept waiting for its delay to pass.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Pending {
    #[serde(skip)]
    id: u64,
    /// Unix milliseconds at which the match will be accepted.
    pub accept_at: i64,
}

#[derive(Default)]
pub struct AutoAccept {
    next_id: AtomicU64,
    pending: Mutex<Option<Pending>>,
}

impl AutoAccept {
    pub fn pending(&self) -> Option<Pending> {
        *self.pending.lock().unwrap_or_else(PoisonError::into_inner)
    }

    fn start(&self, accept_at: i64) -> u64 {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
        let mut pending = self.pending.lock().unwrap_or_else(PoisonError::into_inner);
        *pending = Some(Pending { id, accept_at });
        id
    }

    /// Claims the pending accept if it is still the one scheduled as `id`.
    fn claim(&self, id: u64) -> bool {
        let mut pending = self.pending.lock().unwrap_or_else(PoisonError::into_inner);
        if pending.is_some_and(|p| p.id == id) {
            *pending = None;
            return true;
        }
        false
    }

    /// Returns whether anything was pending.
    pub fn clear(&self) -> bool {
        let mut pending = self.pending.lock().unwrap_or_else(PoisonError::into_inner);
        pending.take().is_some()
    }
}

pub fn on_phase(app: &AppHandle, phase: &str) {
    if phase != "ReadyCheck" {
        cancel(app);
        return;
    }
    let state = app.state::<AppState>();
    let settings = state.settings();
    if settings.auto_accept && state.auto_accept.pending().is_none() {
        schedule(app, settings.auto_accept_delay_secs);
    }
}

/// The user answered the popup themselves.
pub fn on_ready_check(app: &AppHandle, event: &LcuEvent) {
    let response = event.data.get("playerResponse").and_then(Value::as_str);
    if matches!(response, Some("Accepted" | "Declined")) {
        cancel(app);
    }
}

pub fn cancel(app: &AppHandle) {
    if app.state::<AppState>().auto_accept.clear() {
        log::info!("auto accept cancelled");
        emit(app);
    }
}

fn schedule(app: &AppHandle, delay_secs: u32) {
    let accept_at = now_ms() + i64::from(delay_secs) * 1000;
    let id = app.state::<AppState>().auto_accept.start(accept_at);
    log::info!("ready check: accepting in {delay_secs}s");
    emit(app);

    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(Duration::from_secs(u64::from(delay_secs))).await;
        accept(&app, id).await;
    });
}

async fn accept(app: &AppHandle, id: u64) {
    let state = app.state::<AppState>();
    if !state.auto_accept.claim(id) {
        return;
    }
    emit(app);
    let Ok(session) = state.session() else {
        return;
    };
    match session.http.post_empty(ACCEPT).await {
        Ok(()) => log::info!("ready check accepted"),
        Err(err) => log::warn!("failed to accept ready check: {err}"),
    }
}

fn emit(app: &AppHandle) {
    let pending = app.state::<AppState>().auto_accept.pending();
    if let Err(err) = app.emit(AUTO_ACCEPT_EVENT, pending) {
        log::warn!("failed to emit {AUTO_ACCEPT_EVENT}: {err}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_the_latest_schedule_can_accept() {
        let auto = AutoAccept::default();
        let first = auto.start(1);
        let second = auto.start(2);
        assert!(!auto.claim(first));
        assert!(auto.claim(second));
        assert!(auto.pending().is_none());
    }

    #[test]
    fn cleared_schedule_does_not_accept() {
        let auto = AutoAccept::default();
        let id = auto.start(1);
        assert!(auto.clear());
        assert!(!auto.claim(id));
        assert!(!auto.clear());
    }
}
