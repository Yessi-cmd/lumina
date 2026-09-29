//! Votes for a random teammate when the honor screen appears, if the user turned that on.

use std::time::{Duration, SystemTime, UNIX_EPOCH};

use reqwest::Method;
use serde::Deserialize;
use serde_json::{json, Value};
use tauri::{AppHandle, Manager};

use crate::state::AppState;

const BALLOT: &str = "/lol-honor-v2/v1/ballot";
const HONOR: &str = "/lol-honor-v2/v1/honor-player";
const PHASE: &str = "PreEndOfGame";
/// The ballot shows up a moment after the phase starts.
const ATTEMPTS: u32 = 8;
const INTERVAL: Duration = Duration::from_secs(2);

/// `/lol-honor-v2/v1/ballot`
#[derive(Debug, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
struct Ballot {
    game_id: i64,
    eligible_allies: Vec<Candidate>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
struct Candidate {
    summoner_id: i64,
    puuid: String,
}

pub fn on_phase(app: &AppHandle, phase: &str) {
    if phase == PHASE && app.state::<AppState>().settings().auto_honor {
        tauri::async_runtime::spawn(vote(app.clone()));
    }
}

async fn vote(app: AppHandle) {
    for attempt in 0..ATTEMPTS {
        if attempt > 0 {
            tokio::time::sleep(INTERVAL).await;
        }
        let state = app.state::<AppState>();
        // The player may have voted by hand, or the screen may be gone.
        if state.lcu_snapshot().gameflow_phase != PHASE {
            return;
        }
        let Ok(session) = state.session() else {
            return;
        };
        let ballot: Ballot = match session.http.get(BALLOT).await {
            Ok(ballot) => ballot,
            Err(err) => {
                log::debug!("no honor ballot yet: {err}");
                continue;
            }
        };
        let Some(ally) = pick(&ballot.eligible_allies) else {
            continue;
        };
        let category = state.settings().honor_category;
        let body = vote_body(ballot.game_id, &category, ally);
        let sent = session.http.send_json(Method::POST, HONOR, Some(&body));
        match sent.await {
            Ok(()) => log::info!("honored a teammate ({category})"),
            Err(err) => log::warn!("failed to honor a teammate: {err}"),
        }
        return;
    }
}

/// Any teammate will do; the clock's nanoseconds are random enough for that.
fn pick(allies: &[Candidate]) -> Option<&Candidate> {
    if allies.is_empty() {
        return None;
    }
    let elapsed = SystemTime::now().duration_since(UNIX_EPOCH);
    let nanos = elapsed.map_or(0, |d| d.subsec_nanos()) as usize;
    allies.get(nanos % allies.len())
}

/// Newer clients identify players by puuid, older ones by summoner id.
fn vote_body(game_id: i64, category: &str, ally: &Candidate) -> Value {
    if ally.summoner_id > 0 {
        return json!({
            "gameId": game_id,
            "honorCategory": category,
            "summonerId": ally.summoner_id,
        });
    }
    json!({
        "gameId": game_id,
        "honorCategory": category,
        "puuid": ally.puuid,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ally(summoner_id: i64, puuid: &str) -> Candidate {
        Candidate {
            summoner_id,
            puuid: puuid.to_owned(),
        }
    }

    #[test]
    fn no_teammates_no_vote() {
        assert!(pick(&[]).is_none());
        let allies = [ally(1, "a"), ally(2, "b")];
        assert!(pick(&allies).is_some());
    }

    #[test]
    fn vote_names_the_teammate_the_way_the_client_knows_them() {
        let body = vote_body(7, "HEART", &ally(42, "p"));
        assert_eq!(body["summonerId"], 42);
        assert_eq!(body["honorCategory"], "HEART");
        assert_eq!(body["gameId"], 7);
        let body = vote_body(7, "COOL", &ally(0, "p"));
        assert_eq!(body["puuid"], "p");
        assert!(body.get("summonerId").is_none());
    }
}
