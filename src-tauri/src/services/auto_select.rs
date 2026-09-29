//! Bans and picks for the player during champ select, from presets per position.
//!
//! When the player's own ban or pick comes on the clock and nothing is hovered yet, the
//! first champion of the position's preset that the client allows is hovered, and locked
//! in after a short delay unless the player chose something else meanwhile. The `ANY`
//! preset backs up every position, and bans never take what the player or a teammate
//! is hovering or has declared.

use std::collections::HashSet;
use std::sync::{Mutex, PoisonError};
use std::time::Duration;

use reqwest::Method;
use serde::Deserialize;
use serde_json::json;
use tauri::{AppHandle, Manager};

use crate::clients::lcu::models::{LcuEvent, LcuEventType};
use crate::config::AutoSelect;
use crate::error::Result;
use crate::state::AppState;

const SESSION: &str = "/lol-champ-select/v1/session";
const ACTIONS: &str = "/lol-champ-select/v1/session/actions";
const BANNABLE: &str = "/lol-champ-select/v1/bannable-champion-ids";
const PICKABLE: &str = "/lol-champ-select/v1/pickable-champion-ids";
const ANY: &str = "ANY";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Kind {
    Ban,
    Pick,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
struct Session {
    /// Missing or negative when spectating.
    local_player_cell_id: Option<i64>,
    my_team: Vec<Member>,
    actions: Vec<Vec<Action>>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
struct Member {
    cell_id: i64,
    champion_id: i64,
    champion_pick_intent: i64,
    /// `top` / `jungle` / `middle` / `bottom` / `utility`; empty when not shown.
    assigned_position: String,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
struct Action {
    id: i64,
    actor_cell_id: i64,
    champion_id: i64,
    #[serde(rename = "type")]
    kind: String,
    completed: bool,
    is_in_progress: bool,
}

/// The player's own ban or pick that is on the clock.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Turn {
    action_id: i64,
    kind: Kind,
    /// `TOP` / `JUNGLE` / `MIDDLE` / `BOTTOM` / `UTILITY`; empty when unknown.
    position: String,
    /// Champions a ban must leave alone.
    avoid: HashSet<i64>,
}

impl Session {
    /// Only an action nothing is hovered on yet: a champion the player has already
    /// chosen is left alone.
    fn my_turn(&self) -> Option<Turn> {
        let me = self.local_player_cell_id.filter(|cell| *cell >= 0)?;
        let mine = self.my_team.iter().find(|m| m.cell_id == me)?;
        let on_the_clock = |a: &&Action| {
            a.actor_cell_id == me && a.is_in_progress && !a.completed && a.champion_id == 0
        };
        let action = self.actions.iter().flatten().find(on_the_clock)?;
        let kind = match action.kind.as_str() {
            "ban" => Kind::Ban,
            "pick" => Kind::Pick,
            _ => return None,
        };
        let mut avoid = HashSet::new();
        if kind == Kind::Ban {
            for member in &self.my_team {
                avoid.extend([member.champion_id, member.champion_pick_intent]);
            }
            avoid.remove(&0);
        }
        Some(Turn {
            action_id: action.id,
            kind,
            position: mine.assigned_position.to_uppercase(),
            avoid,
        })
    }

    /// Whether the action is still open with `champion` hovered on it.
    fn is_hovering(&self, action_id: i64, champion: i64) -> bool {
        let mut actions = self.actions.iter().flatten();
        actions.any(|a| {
            let open = a.is_in_progress && !a.completed;
            a.id == action_id && open && a.champion_id == champion
        })
    }
}

/// The position's list first, then the shared one.
fn candidates(config: &AutoSelect, position: &str, kind: Kind) -> Vec<i64> {
    let mut out = Vec::new();
    for key in [position, ANY] {
        let Some(preset) = config.presets.get(key) else {
            continue;
        };
        out.extend(match kind {
            Kind::Ban => &preset.bans,
            Kind::Pick => &preset.picks,
        });
    }
    out
}

/// The first candidate the client allows and that is not to be avoided.
fn choose(candidates: &[i64], available: &[i64], avoid: &HashSet<i64>) -> Option<i64> {
    let allowed = |id: &i64| available.contains(id) && !avoid.contains(id);
    candidates.iter().copied().find(allowed)
}

/// Action ids already handled in the current champ select.
#[derive(Default)]
pub struct AutoSelectState {
    handled: Mutex<HashSet<i64>>,
}

impl AutoSelectState {
    /// True the first time an action is seen, so one turn is handled once.
    fn claim(&self, action_id: i64) -> bool {
        let mut handled = self.handled.lock().unwrap_or_else(PoisonError::into_inner);
        handled.insert(action_id)
    }

    fn clear(&self) {
        let mut handled = self.handled.lock().unwrap_or_else(PoisonError::into_inner);
        handled.clear();
    }
}

pub fn on_phase(app: &AppHandle, phase: &str) {
    if phase != "ChampSelect" {
        app.state::<AppState>().auto_select.clear();
    }
}

pub fn on_champ_select(app: &AppHandle, event: &LcuEvent) {
    if event.event_type == LcuEventType::Delete {
        return;
    }
    let state = app.state::<AppState>();
    let config = state.settings().auto_select;
    if !config.ban && !config.pick {
        return;
    }
    let Ok(session) = serde_json::from_value::<Session>(event.data.clone()) else {
        return;
    };
    let Some(turn) = session.my_turn() else {
        return;
    };
    let wanted = match turn.kind {
        Kind::Ban => config.ban,
        Kind::Pick => config.pick,
    };
    if wanted && state.auto_select.claim(turn.action_id) {
        tauri::async_runtime::spawn(run(app.clone(), turn, config));
    }
}

async fn run(app: AppHandle, turn: Turn, config: AutoSelect) {
    if let Err(err) = select(&app, &turn, &config).await {
        log::warn!("auto {:?} failed: {err}", turn.kind);
    }
}

async fn select(app: &AppHandle, turn: &Turn, config: &AutoSelect) -> Result<()> {
    let state = app.state::<AppState>();
    let session = state.session()?;
    let list = match turn.kind {
        Kind::Ban => BANNABLE,
        Kind::Pick => PICKABLE,
    };
    let available: Vec<i64> = session.http.get(list).await?;
    let wanted = candidates(config, &turn.position, turn.kind);
    let Some(champion) = choose(&wanted, &available, &turn.avoid) else {
        log::info!(
            "auto {:?}: no champion of the preset is available",
            turn.kind
        );
        return Ok(());
    };

    let action = format!("{ACTIONS}/{}", turn.action_id);
    let hover = json!({ "championId": champion });
    session
        .http
        .send_json(Method::PATCH, &action, Some(&hover))
        .await?;
    tokio::time::sleep(Duration::from_secs(u64::from(config.delay_secs))).await;

    // The player may have chosen something else, or the turn may be over.
    let now: Session = session.http.get(SESSION).await?;
    if !now.is_hovering(turn.action_id, champion) {
        log::info!("auto {:?}: left alone, the player chose", turn.kind);
        return Ok(());
    }
    let complete = format!("{action}/complete");
    session.http.post_empty(&complete).await?;
    log::info!("auto {:?}: locked champion {champion}", turn.kind);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Preset;

    fn session(json: &str) -> Session {
        serde_json::from_str(json).unwrap()
    }

    const BAN_TURN: &str = r#"{
        "localPlayerCellId": 2,
        "myTeam": [
            {"cellId": 1, "championId": 10, "championPickIntent": 0, "assignedPosition": "top"},
            {"cellId": 2, "championId": 0, "championPickIntent": 55, "assignedPosition": "middle"},
            {"cellId": 3, "championId": 0, "championPickIntent": 77, "assignedPosition": "jungle"}
        ],
        "actions": [[
            {"id": 4, "actorCellId": 1, "championId": 0, "type": "ban", "completed": false, "isInProgress": false},
            {"id": 5, "actorCellId": 2, "championId": 0, "type": "ban", "completed": false, "isInProgress": true}
        ]]
    }"#;

    const CELL: &str = "\"localPlayerCellId\": 2";
    const OPEN: &str = "\"id\": 5, \"actorCellId\": 2, \"championId\": 0";
    const HOVERED: &str = "\"id\": 5, \"actorCellId\": 2, \"championId\": 99";

    #[test]
    fn finds_the_players_own_ban_on_the_clock() {
        let turn = session(BAN_TURN).my_turn().unwrap();
        assert_eq!(turn.action_id, 5);
        assert_eq!(turn.kind, Kind::Ban);
        assert_eq!(turn.position, "MIDDLE");
    }

    #[test]
    fn bans_avoid_what_the_team_hovers_or_declares() {
        let turn = session(BAN_TURN).my_turn().unwrap();
        let expected: HashSet<i64> = [10, 55, 77].into();
        assert_eq!(turn.avoid, expected);
    }

    #[test]
    fn picks_have_nothing_to_avoid() {
        let json = BAN_TURN.replace("\"ban\"", "\"pick\"");
        let turn = session(&json).my_turn().unwrap();
        assert_eq!(turn.kind, Kind::Pick);
        assert!(turn.avoid.is_empty());
    }

    #[test]
    fn a_hovered_champion_or_someone_elses_turn_is_left_alone() {
        let hovered = BAN_TURN.replace(OPEN, HOVERED);
        assert!(session(&hovered).my_turn().is_none());
        let elsewhere = BAN_TURN.replace(CELL, "\"localPlayerCellId\": 3");
        assert!(session(&elsewhere).my_turn().is_none());
        let spectating = BAN_TURN.replace(CELL, "\"localPlayerCellId\": -1");
        assert!(session(&spectating).my_turn().is_none());
    }

    #[test]
    fn recognises_the_hover_it_made() {
        let hovered = BAN_TURN.replace(OPEN, HOVERED);
        assert!(session(&hovered).is_hovering(5, 99));
        assert!(!session(&hovered).is_hovering(5, 98));
        assert!(!session(&hovered).is_hovering(4, 99));
    }

    fn config() -> AutoSelect {
        let mut config = AutoSelect::default();
        let mid = Preset {
            picks: vec![1, 2],
            bans: vec![10, 11],
        };
        let any = Preset {
            picks: vec![3],
            bans: vec![12],
        };
        config.presets.insert("MIDDLE".to_owned(), mid);
        config.presets.insert("ANY".to_owned(), any);
        config
    }

    #[test]
    fn candidates_follow_the_position_then_the_shared_list() {
        let config = config();
        assert_eq!(candidates(&config, "MIDDLE", Kind::Ban), vec![10, 11, 12]);
        assert_eq!(candidates(&config, "MIDDLE", Kind::Pick), vec![1, 2, 3]);
        assert_eq!(candidates(&config, "TOP", Kind::Ban), vec![12]);
        assert_eq!(candidates(&config, "", Kind::Pick), vec![3]);
    }

    #[test]
    fn chooses_the_first_allowed_champion() {
        let avoid: HashSet<i64> = [11].into();
        assert_eq!(choose(&[10, 11, 12], &[11, 12], &avoid), Some(12));
        assert_eq!(choose(&[10, 11], &[12], &avoid), None);
        assert_eq!(choose(&[], &[12], &avoid), None);
    }

    #[test]
    fn a_turn_is_handled_once_per_champ_select() {
        let state = AutoSelectState::default();
        assert!(state.claim(5));
        assert!(!state.claim(5));
        state.clear();
        assert!(state.claim(5));
    }
}
