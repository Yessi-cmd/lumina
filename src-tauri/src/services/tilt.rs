//! Warns before another ranked game when the last ones were all losses.
//!
//! The losses come from the rank history, so only ranked games count. The check runs
//! when a new point is recorded and when the player starts queueing.

use std::sync::{Mutex, PoisonError};

use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager};

use super::push::{self, Kind};
use super::rank_history::{Outcome, RankPoint};
use super::roster_relations::now_ms;
use crate::state::AppState;

pub const TILT_EVENT: &str = "tilt://warning";
/// A longer pause ends a losing run: yesterday's losses are no reason to nag today.
const RUN_GAP_MS: i64 = 3 * 60 * 60 * 1000;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TiltWarning {
    /// Ranked losses in a row.
    pub losses: usize,
    /// LP lost over them, zero or negative.
    pub lp: i64,
}

#[derive(Default)]
pub struct Tilt {
    warning: Mutex<Option<TiltWarning>>,
}

impl Tilt {
    pub fn current(&self) -> Option<TiltWarning> {
        let guard = self.warning.lock().unwrap_or_else(PoisonError::into_inner);
        guard.clone()
    }

    /// Returns whether the warning changed.
    fn set(&self, warning: Option<TiltWarning>) -> bool {
        let mut current = self.warning.lock().unwrap_or_else(PoisonError::into_inner);
        if *current == warning {
            return false;
        }
        *current = warning;
        true
    }
}

/// The losing run the record ends with, if it is still going on at `now`.
fn losing_run(points: &[RankPoint], now: i64) -> Option<TiltWarning> {
    let mut results: Vec<&RankPoint> = points.iter().filter(|p| p.outcome.is_some()).collect();
    results.sort_by_key(|p| p.at);

    let mut run = TiltWarning { losses: 0, lp: 0 };
    let mut newer = now;
    for point in results.into_iter().rev() {
        if point.outcome != Some(Outcome::Loss) || newer - point.at > RUN_GAP_MS {
            break;
        }
        run.losses += 1;
        run.lp += point.delta.unwrap_or(0);
        newer = point.at;
    }
    (run.losses > 0).then_some(run)
}

pub fn on_phase(app: &AppHandle, phase: &str) {
    if phase == "Matchmaking" {
        check(app, false);
    }
}

/// Recomputes the warning, tells the frontend when it changed and, with `phone`, pushes
/// a new warning to the phone.
pub fn check(app: &AppHandle, phone: bool) {
    let state = app.state::<AppState>();
    let threshold = state.settings().tilt_streak as usize;
    let puuid = state.lcu_snapshot().summoner.map(|s| s.puuid);
    let history = state.ranks.history(puuid.as_deref());
    let now = now_ms();
    let run = history.and_then(|h| losing_run(&h.points, now));
    let warning = run.filter(|w| threshold > 0 && w.losses >= threshold);
    if !state.tilt.set(warning.clone()) {
        return;
    }
    emit(app, warning.as_ref());
    let Some(warning) = warning.filter(|_| phone) else {
        return;
    };
    let body = format!("已连败 {} 把（{} 点）", warning.losses, warning.lp);
    push::notify(app, Kind::Tilt, "先歇一歇", &body);
}

pub fn dismiss(app: &AppHandle) {
    if app.state::<AppState>().tilt.set(None) {
        emit(app, None);
    }
}

fn emit(app: &AppHandle, warning: Option<&TiltWarning>) {
    if let Err(err) = app.emit(TILT_EVENT, warning) {
        log::warn!("failed to emit {TILT_EVENT}: {err}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const HOUR: i64 = 60 * 60 * 1000;

    fn point(at: i64, outcome: Option<Outcome>, delta: i64) -> RankPoint {
        RankPoint {
            at,
            outcome,
            delta: Some(delta),
            ..RankPoint::default()
        }
    }

    fn loss(at: i64, delta: i64) -> RankPoint {
        point(at, Some(Outcome::Loss), delta)
    }

    fn win(at: i64, delta: i64) -> RankPoint {
        point(at, Some(Outcome::Win), delta)
    }

    #[test]
    fn counts_the_losses_since_the_last_win() {
        let points = [win(HOUR, 20), loss(2 * HOUR, -17), loss(3 * HOUR, -19)];
        let run = losing_run(&points, 3 * HOUR + 1).unwrap();
        assert_eq!(run, TiltWarning { losses: 2, lp: -36 });
    }

    #[test]
    fn a_win_last_means_no_run() {
        let points = [loss(HOUR, -17), win(2 * HOUR, 20)];
        assert_eq!(losing_run(&points, 2 * HOUR), None);
    }

    #[test]
    fn a_long_pause_ends_the_run() {
        let points = [loss(HOUR, -17), loss(6 * HOUR, -19)];
        let run = losing_run(&points, 6 * HOUR).unwrap();
        assert_eq!(run.losses, 1);
        // Nothing new for hours: the run is over.
        assert_eq!(losing_run(&points, 12 * HOUR), None);
    }

    #[test]
    fn points_without_a_result_do_not_break_the_run() {
        let points = [
            loss(HOUR, -17),
            point(2 * HOUR, None, -3),
            loss(3 * HOUR, -19),
        ];
        assert_eq!(losing_run(&points, 3 * HOUR).unwrap().losses, 2);
    }

    #[test]
    fn unordered_points_are_sorted_first() {
        let points = [loss(3 * HOUR, -19), win(HOUR, 20), loss(2 * HOUR, -17)];
        assert_eq!(losing_run(&points, 3 * HOUR).unwrap().losses, 2);
    }
}
