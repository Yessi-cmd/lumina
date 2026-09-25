use std::time::Duration;

use super::models::SgpMatchHistory;
use super::servers::ResolvedServer;
use crate::error::{AppError, Result};

const REQUEST_TIMEOUT: Duration = Duration::from_secs(10);
const USER_AGENT: &str = concat!("Lumina/", env!("CARGO_PKG_VERSION"));

pub struct SgpClient {
    client: reqwest::Client,
    server: ResolvedServer,
}

impl SgpClient {
    pub fn new(server: ResolvedServer) -> Result<Self> {
        let client = reqwest::Client::builder()
            .timeout(REQUEST_TIMEOUT)
            .user_agent(USER_AGENT)
            .build()?;
        Ok(Self { client, server })
    }

    pub fn server(&self) -> &ResolvedServer {
        &self.server
    }

    /// Newest first. Authenticated with the LCU entitlements access token.
    pub async fn match_history(
        &self,
        entitlements_token: &str,
        puuid: &str,
        start: u32,
        count: u32,
    ) -> Result<SgpMatchHistory> {
        let base = &self.server.server.match_history;
        let url = format!("{base}/match-history-query/v1/products/lol/player/{puuid}/SUMMARY");
        let resp = self
            .client
            .get(url)
            .bearer_auth(entitlements_token)
            .query(&[("startIndex", start), ("count", count)])
            .send()
            .await?;
        let status = resp.status();
        if !status.is_success() {
            return Err(AppError::SgpStatus(status.as_u16()));
        }
        Ok(resp.json().await?)
    }
}
