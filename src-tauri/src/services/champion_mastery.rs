//! Champion mastery points count every game ever played on a champion, so they tell an
//! old main who took a break apart from someone trying the champion for the first time;
//! a page of recent games cannot.

use serde::{Deserialize, Serialize};

use crate::state::session::LcuSession;

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Mastery {
    pub champion_id: i64,
    #[serde(default)]
    pub champion_level: i64,
    pub champion_points: i64,
}

/// Every champion the player has mastery on, most points first; `None` when the client
/// does not say.
pub async fn all(session: &LcuSession, puuid: &str) -> Option<Vec<Mastery>> {
    let path = format!("/lol-champion-mastery/v1/{puuid}/champion-mastery");
    match session.http.get::<Vec<Mastery>>(&path).await {
        Ok(mut masteries) => {
            masteries.sort_by_key(|m| std::cmp::Reverse(m.champion_points));
            Some(masteries)
        }
        Err(err) => {
            log::debug!("champion mastery unavailable: {err}");
            None
        }
    }
}

/// Points on one champion; 0 when never played, `None` when the client does not say.
pub async fn points(session: &LcuSession, puuid: &str, champion_id: i64) -> Option<i64> {
    let masteries = all(session, puuid).await?;
    let found = masteries.iter().find(|m| m.champion_id == champion_id);
    Some(found.map_or(0, |m| m.champion_points))
}
