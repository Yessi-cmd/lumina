//! Champion mastery points count every game ever played on a champion, so they tell an
//! old main who took a break apart from someone trying the champion for the first time;
//! a page of recent games cannot.

use serde::Deserialize;

use crate::state::session::LcuSession;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Mastery {
    champion_id: i64,
    champion_points: i64,
}

/// Points on one champion; 0 when never played, `None` when the client does not say.
pub async fn points(session: &LcuSession, puuid: &str, champion_id: i64) -> Option<i64> {
    let path = format!("/lol-champion-mastery/v1/{puuid}/champion-mastery");
    match session.http.get::<Vec<Mastery>>(&path).await {
        Ok(masteries) => {
            let found = masteries.iter().find(|m| m.champion_id == champion_id);
            Some(found.map_or(0, |m| m.champion_points))
        }
        Err(err) => {
            log::debug!("champion mastery unavailable: {err}");
            None
        }
    }
}
