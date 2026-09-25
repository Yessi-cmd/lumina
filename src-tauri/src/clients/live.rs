//! Live Client Data API, served by the game itself on port 2999 while a match runs.
//! Its certificate chains to the same Riot root as the LCU.

use std::time::Duration;

use serde::Deserialize;

use super::lcu::http::tls_connector;
use crate::error::{AppError, Result};

const GAME_STATS: &str = "https://127.0.0.1:2999/liveclientdata/gamestats";
const REQUEST_TIMEOUT: Duration = Duration::from_secs(2);

#[derive(Debug, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
struct GameStats {
    game_time: f64,
}

pub struct LiveClient {
    client: reqwest::Client,
}

impl LiveClient {
    pub fn new() -> Result<Self> {
        let client = reqwest::Client::builder()
            .use_preconfigured_tls(tls_connector()?)
            .timeout(REQUEST_TIMEOUT)
            .no_proxy()
            .build()?;
        Ok(Self { client })
    }

    /// Seconds on the match clock; fails while the loading screen is still up.
    pub async fn game_time(&self) -> Result<f64> {
        let resp = self.client.get(GAME_STATS).send().await?;
        let status = resp.status();
        if !status.is_success() {
            return Err(AppError::Message(format!("游戏内接口返回 {status}")));
        }
        let stats: GameStats = resp.json().await?;
        Ok(stats.game_time)
    }
}
