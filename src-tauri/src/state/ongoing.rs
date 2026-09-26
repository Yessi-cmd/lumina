//! Players of the current game, pushed to the frontend as `ongoing://roster`.

use std::collections::HashSet;
use std::sync::PoisonError;

use serde::Serialize;
use tauri::Emitter;

use super::AppState;

pub const ROSTER_EVENT: &str = "ongoing://roster";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum RosterStage {
    /// In a party before queueing: only the party members are known.
    Lobby,
    ChampSelect,
    InGame,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Roster {
    pub stage: RosterStage,
    pub game_id: i64,
    /// 0 when unknown (champ select does not say).
    pub queue_id: i64,
    pub allies: Vec<RosterPlayer>,
    /// Hidden teammates whose obfuscated PUUID is absent or cannot be resolved.
    pub anonymous_allies: Vec<AnonymousPlayer>,
    pub enemies: Vec<RosterPlayer>,
    /// Opponents whose identity cannot be resolved from the client payload.
    pub hidden_enemies: usize,
    /// Champions the opponents have locked, even when their identities are hidden.
    pub enemy_champions: Vec<i64>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RosterPlayer {
    pub puuid: String,
    /// Locked or hovered champion; 0 when none yet.
    pub champion_id: i64,
    /// `TOP`/`JUNGLE`/`MIDDLE`/`BOTTOM`/`UTILITY`; empty outside role queues.
    pub position: String,
    pub is_self: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AnonymousPlayer {
    /// Locked or hovered champion; 0 when none yet.
    pub champion_id: i64,
    pub position: String,
}

impl Roster {
    pub fn puuids(&self) -> impl Iterator<Item = &str> {
        let players = self.allies.iter().chain(&self.enemies);
        players.map(|p| p.puuid.as_str())
    }
}

impl AppState {
    pub fn roster(&self) -> Option<Roster> {
        let guard = self.roster.lock().unwrap_or_else(PoisonError::into_inner);
        guard.clone()
    }

    /// Replaces the roster and returns the puuids that were not in the previous one.
    pub fn set_roster(&self, roster: Option<Roster>) -> Vec<String> {
        let fresh = {
            let mut guard = self.roster.lock().unwrap_or_else(PoisonError::into_inner);
            if *guard == roster {
                return Vec::new();
            }
            let fresh = new_puuids(guard.as_ref(), roster.as_ref());
            guard.clone_from(&roster);
            fresh
        };
        if let Err(err) = self.app.emit(ROSTER_EVENT, &roster) {
            log::warn!("failed to emit {ROSTER_EVENT}: {err}");
        }
        fresh
    }
}

fn new_puuids(old: Option<&Roster>, new: Option<&Roster>) -> Vec<String> {
    let known: HashSet<&str> = old.into_iter().flat_map(|r| r.puuids()).collect();
    let current = new.into_iter().flat_map(|r| r.puuids());
    let fresh = current.filter(|p| !known.contains(p));
    fresh.map(str::to_owned).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_new_puuids() {
        let player = |puuid: &str| RosterPlayer {
            puuid: puuid.to_owned(),
            champion_id: 0,
            position: String::new(),
            is_self: false,
        };
        let roster = |allies: Vec<RosterPlayer>| Roster {
            stage: RosterStage::ChampSelect,
            game_id: 1,
            queue_id: 0,
            allies,
            anonymous_allies: Vec::new(),
            enemies: Vec::new(),
            hidden_enemies: 0,
            enemy_champions: Vec::new(),
        };
        let state_old = roster(vec![player("a")]);
        let state_new = roster(vec![player("a"), player("b")]);
        let fresh = new_puuids(Some(&state_old), Some(&state_new));
        assert_eq!(fresh, vec!["b".to_owned()]);
    }
}
