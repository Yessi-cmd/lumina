//! Who is in the current game. Party members come from the lobby, teammates from champ
//! select; opponents are hidden there on the Chinese servers (and in ranked elsewhere),
//! so they appear only once `/lol-gameflow/v1/session` lists both teams at GameStart.

use tauri::{AppHandle, Manager};

use crate::clients::lcu::models::{
    ChampSelectMember, ChampSelectSession, GameflowPlayer, GameflowSession, LcuEvent, LcuEventType,
    LobbySession,
};
use crate::state::ongoing::{Roster, RosterPlayer, RosterStage};
use crate::state::AppState;

pub const CHAMP_SELECT_SESSION: &str = "/lol-champ-select/v1/session";
pub const GAMEFLOW_SESSION: &str = "/lol-gameflow/v1/session";
pub const LOBBY: &str = "/lol-lobby/v2/lobby";
/// Same page the game panel requests, so prefetching fills its cache entry.
pub const PANEL_HISTORY_COUNT: u32 = 20;
const EMPTY_PUUID: &str = "00000000-0000-0000-0000-000000000000";
/// Phases in which `/lol-gameflow/v1/session` carries the full player list.
const IN_GAME_PHASES: [&str; 4] = ["GameStart", "InProgress", "Reconnect", "WaitingForStats"];
/// Phases in which the party lobby is the best roster there is.
const LOBBY_PHASES: [&str; 3] = ["Lobby", "Matchmaking", "ReadyCheck"];

pub fn on_champ_select(app: &AppHandle, event: &LcuEvent) {
    if event.event_type == LcuEventType::Delete {
        return;
    }
    match serde_json::from_value::<ChampSelectSession>(event.data.clone()) {
        Ok(session) => apply_champ_select(app, session),
        Err(err) => log::warn!("unexpected champ select payload: {err}"),
    }
}

pub fn on_gameflow_session(app: &AppHandle, event: &LcuEvent) {
    if event.event_type == LcuEventType::Delete {
        return;
    }
    match serde_json::from_value::<GameflowSession>(event.data.clone()) {
        Ok(session) => apply_gameflow(app, session),
        Err(err) => log::warn!("unexpected gameflow session payload: {err}"),
    }
}

pub fn on_phase(app: &AppHandle, phase: &str) {
    if phase == "None" {
        apply(app, None);
    } else if LOBBY_PHASES.contains(&phase) {
        tauri::async_runtime::spawn(load_lobby(app.clone()));
    }
}

pub fn on_lobby(app: &AppHandle, event: &LcuEvent) {
    if event.event_type == LcuEventType::Delete {
        let current = app.state::<AppState>().roster();
        if current.is_some_and(|r| r.stage == RosterStage::Lobby) {
            apply(app, None);
        }
        return;
    }
    match serde_json::from_value::<LobbySession>(event.data.clone()) {
        Ok(lobby) => apply_lobby(app, lobby),
        Err(err) => log::warn!("unexpected lobby payload: {err}"),
    }
}

async fn load_lobby(app: AppHandle) {
    let Ok(session) = app.state::<AppState>().session() else {
        return;
    };
    match session.http.get::<LobbySession>(LOBBY).await {
        Ok(lobby) => apply_lobby(&app, lobby),
        Err(err) => log::debug!("no lobby: {err}"),
    }
}

/// Lobby events also arrive during champ select and games; only use them before.
fn apply_lobby(app: &AppHandle, lobby: LobbySession) {
    let snapshot = app.state::<AppState>().lcu_snapshot();
    if !LOBBY_PHASES.contains(&snapshot.gameflow_phase.as_str()) {
        return;
    }
    let self_puuid = snapshot.summoner.map(|s| s.puuid).unwrap_or_default();
    apply(app, Some(from_lobby(lobby, &self_puuid)));
}

/// Catches up when Lumina connects in the middle of champ select or a game.
pub async fn load_initial(app: &AppHandle, phase: &str) {
    let state = app.state::<AppState>();
    let Ok(session) = state.session() else {
        return;
    };
    if LOBBY_PHASES.contains(&phase) {
        match session.http.get(LOBBY).await {
            Ok(lobby) => apply_lobby(app, lobby),
            Err(err) => log::debug!("no lobby: {err}"),
        }
    }
    if phase == "ChampSelect" {
        match session.http.get(CHAMP_SELECT_SESSION).await {
            Ok(cs) => apply_champ_select(app, cs),
            Err(err) => log::warn!("failed to load champ select session: {err}"),
        }
    }
    // Also gives champ select its queue id.
    if phase == "ChampSelect" || IN_GAME_PHASES.contains(&phase) {
        match session.http.get(GAMEFLOW_SESSION).await {
            Ok(gs) => apply_gameflow(app, gs),
            Err(err) => log::warn!("failed to load gameflow session: {err}"),
        }
    }
}

/// Champ select does not name the queue; keep the one the gameflow session reported.
fn apply_champ_select(app: &AppHandle, session: ChampSelectSession) {
    let mut roster = from_champ_select(session);
    let current = app.state::<AppState>().roster();
    let same_game = current.as_ref().filter(|c| c.game_id == roster.game_id);
    // Queueing from a party already told us the queue.
    let party = current.as_ref().filter(|c| c.stage == RosterStage::Lobby);
    if let Some(lobby) = party {
        roster.queue_id = lobby.queue_id;
    }
    match same_game {
        Some(current) => roster.queue_id = current.queue_id,
        // First event of this champ select: ask the gameflow session for the queue.
        None => {
            tauri::async_runtime::spawn(load_queue(app.clone()));
        }
    }
    apply(app, Some(roster));
}

async fn load_queue(app: AppHandle) {
    let Ok(session) = app.state::<AppState>().session() else {
        return;
    };
    match session.http.get::<GameflowSession>(GAMEFLOW_SESSION).await {
        Ok(gs) => apply_gameflow(&app, gs),
        Err(err) => log::debug!("failed to load gameflow session: {err}"),
    }
}

fn apply_gameflow(app: &AppHandle, session: GameflowSession) {
    if session.phase == "ChampSelect" {
        set_champ_select_queue(app, session.game_data.queue.id);
        return;
    }
    if !IN_GAME_PHASES.contains(&session.phase.as_str()) {
        return;
    }
    let snapshot = app.state::<AppState>().lcu_snapshot();
    let self_puuid = snapshot.summoner.map(|s| s.puuid).unwrap_or_default();
    if let Some(roster) = from_gameflow(session, &self_puuid) {
        apply(app, Some(roster));
    }
}

fn set_champ_select_queue(app: &AppHandle, queue_id: i64) {
    let Some(mut roster) = app.state::<AppState>().roster() else {
        return;
    };
    if roster.stage == RosterStage::ChampSelect && roster.queue_id != queue_id {
        roster.queue_id = queue_id;
        apply(app, Some(roster));
    }
}

fn apply(app: &AppHandle, roster: Option<Roster>) {
    let summary = roster.as_ref().map(describe);
    let fresh = app.state::<AppState>().set_roster(roster);
    if fresh.is_empty() {
        return;
    }
    if let Some(summary) = summary {
        log::info!("roster {summary}, {} new players", fresh.len());
    }
    let app = app.clone();
    tauri::async_runtime::spawn(prefetch(app, fresh));
}

fn describe(roster: &Roster) -> String {
    let (id, queue) = (roster.game_id, roster.queue_id);
    let (allies, enemies) = (roster.allies.len(), roster.enemies.len());
    let hidden = roster.hidden_enemies;
    let stage = roster.stage;
    format!(
        "{stage:?} game {id} queue {queue}: {allies} allies, {enemies} enemies, {hidden} hidden"
    )
}

/// Warms the match-history cache so each player card fills as soon as it asks.
async fn prefetch(app: AppHandle, puuids: Vec<String>) {
    let state = app.state::<AppState>();
    let Ok(session) = state.session() else {
        return;
    };
    let history = &state.match_history;
    let requests = puuids
        .iter()
        .map(|puuid| history.get(&session, puuid, 0, PANEL_HISTORY_COUNT));
    for result in futures_util::future::join_all(requests).await {
        if let Err(err) = result {
            log::debug!("match history prefetch failed: {err}");
        }
    }
}

fn from_lobby(lobby: LobbySession, self_puuid: &str) -> Roster {
    let mut allies = Vec::new();
    for member in lobby.members {
        if !is_known(&member.puuid) {
            continue;
        }
        let preference = member.first_position_preference.to_uppercase();
        let position = match preference.as_str() {
            "TOP" | "JUNGLE" | "MIDDLE" | "BOTTOM" | "UTILITY" => preference,
            _ => String::new(),
        };
        allies.push(RosterPlayer {
            is_self: member.puuid == self_puuid,
            puuid: member.puuid,
            champion_id: 0,
            position,
        });
    }
    Roster {
        stage: RosterStage::Lobby,
        game_id: 0,
        queue_id: lobby.game_config.queue_id,
        allies,
        enemies: Vec::new(),
        hidden_enemies: 0,
        enemy_champions: Vec::new(),
    }
}

fn from_champ_select(session: ChampSelectSession) -> Roster {
    let local = session.local_player_cell_id;
    let mut allies = Vec::new();
    for member in session.my_team {
        allies.extend(champ_select_player(member, local));
    }
    let mut enemies = Vec::new();
    let mut hidden_enemies = 0;
    let mut enemy_champions = Vec::new();
    for member in session.their_team {
        if member.champion_id > 0 {
            enemy_champions.push(member.champion_id);
        }
        match champ_select_player(member, local) {
            Some(player) => enemies.push(player),
            None => hidden_enemies += 1,
        }
    }
    Roster {
        stage: RosterStage::ChampSelect,
        game_id: session.game_id,
        queue_id: 0,
        allies,
        enemies,
        hidden_enemies,
        enemy_champions,
    }
}

fn champ_select_player(member: ChampSelectMember, local_cell: i64) -> Option<RosterPlayer> {
    if member.name_visibility_type == "HIDDEN" || !is_known(&member.puuid) {
        return None;
    }
    let champion_id = if member.champion_id > 0 {
        member.champion_id
    } else {
        member.champion_pick_intent
    };
    Some(RosterPlayer {
        puuid: member.puuid,
        champion_id,
        position: member.assigned_position.to_uppercase(),
        is_self: member.cell_id == local_cell,
    })
}

/// `None` when the local player is in neither team (e.g. spectating).
fn from_gameflow(session: GameflowSession, self_puuid: &str) -> Option<Roster> {
    let data = session.game_data;
    let in_team_one = data.team_one.iter().any(|p| p.puuid == self_puuid);
    let in_team_two = data.team_two.iter().any(|p| p.puuid == self_puuid);
    let (mine, theirs) = match (in_team_one, in_team_two) {
        (true, _) => (data.team_one, data.team_two),
        (_, true) => (data.team_two, data.team_one),
        _ => return None,
    };

    let allies = mine
        .into_iter()
        .filter_map(|p| gameflow_player(p, self_puuid))
        .collect();
    let mut enemies = Vec::new();
    let mut hidden_enemies = 0;
    let mut enemy_champions = Vec::new();
    for player in theirs {
        if player.champion_id > 0 {
            enemy_champions.push(player.champion_id);
        }
        match gameflow_player(player, self_puuid) {
            Some(player) => enemies.push(player),
            None => hidden_enemies += 1,
        }
    }
    Some(Roster {
        stage: RosterStage::InGame,
        game_id: data.game_id,
        queue_id: data.queue.id,
        allies,
        enemies,
        hidden_enemies,
        enemy_champions,
    })
}

fn gameflow_player(player: GameflowPlayer, self_puuid: &str) -> Option<RosterPlayer> {
    if !is_known(&player.puuid) {
        return None;
    }
    let is_self = player.puuid == self_puuid;
    Some(RosterPlayer {
        puuid: player.puuid,
        champion_id: player.champion_id,
        position: player.selected_position.to_uppercase(),
        is_self,
    })
}

fn is_known(puuid: &str) -> bool {
    !puuid.is_empty() && puuid != EMPTY_PUUID
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn champ_select_keeps_visible_allies_and_counts_hidden_enemies() {
        let json = r#"{"gameId":1,"localPlayerCellId":2,
            "myTeam":[
              {"cellId":1,"puuid":"a","championId":0,"championPickIntent":64,
               "assignedPosition":"jungle","nameVisibilityType":"VISIBLE"},
              {"cellId":2,"puuid":"me","championId":103,"nameVisibilityType":"VISIBLE"}],
            "theirTeam":[
              {"cellId":5,"puuid":"","nameVisibilityType":"HIDDEN"},
              {"cellId":6,"puuid":"","nameVisibilityType":"HIDDEN"}]}"#;
        let session: ChampSelectSession = serde_json::from_str(json).unwrap();
        let roster = from_champ_select(session);
        assert_eq!(roster.allies.len(), 2);
        assert_eq!(roster.allies[0].champion_id, 64);
        assert_eq!(roster.allies[0].position, "JUNGLE");
        assert!(roster.allies[1].is_self);
        assert!(roster.enemies.is_empty());
        assert_eq!(roster.hidden_enemies, 2);
    }

    #[test]
    fn gameflow_puts_local_player_on_the_ally_side() {
        let json = r#"{"phase":"GameStart","gameData":{"gameId":9,"queue":{"id":420},
            "teamOne":[{"puuid":"x","championId":1}],
            "teamTwo":[{"puuid":"me","championId":2},{"puuid":"y","championId":3}]}}"#;
        let session: GameflowSession = serde_json::from_str(json).unwrap();
        let roster = from_gameflow(session, "me").unwrap();
        assert_eq!(roster.stage, RosterStage::InGame);
        assert_eq!(roster.queue_id, 420);
        assert_eq!(roster.allies.len(), 2);
        assert_eq!(roster.enemies[0].puuid, "x");
    }

    #[test]
    fn gameflow_without_local_player_is_none() {
        let json = r#"{"phase":"InProgress","gameData":{"teamOne":[{"puuid":"x"}]}}"#;
        let session: GameflowSession = serde_json::from_str(json).unwrap();
        assert!(from_gameflow(session, "me").is_none());
    }
}
