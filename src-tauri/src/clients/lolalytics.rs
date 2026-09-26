//! lolalytics.com champion statistics (Riot servers; the Chinese servers are not included).
//! These are the endpoints the site itself uses: undocumented and liable to change, so the
//! caller caches responses and requests them rarely.

use std::collections::HashMap;
use std::time::Duration;

use serde::de::DeserializeOwned;
use serde::Deserialize;
use serde_json::Value;

use crate::error::{AppError, Result};

const BASE_URL: &str = "https://a1.lolalytics.com/mega/";
/// The rolling window the site requests by default.
const PATCH: &str = "30";
/// Ranked solo/duo, also the best stand-in for flex and normal draft.
const QUEUE: &str = "420";
const REQUEST_TIMEOUT: Duration = Duration::from_secs(15);
const USER_AGENT: &str = concat!("Lumina/", env!("CARGO_PKG_VERSION"));

pub struct LolalyticsClient {
    client: reqwest::Client,
}

impl LolalyticsClient {
    pub fn new() -> Result<Self> {
        let client = reqwest::Client::builder()
            .timeout(REQUEST_TIMEOUT)
            .user_agent(USER_AGENT)
            .build()?;
        Ok(Self { client })
    }

    /// Every champion's standing in one lane.
    pub async fn tier_list(&self, lane: &str, tier: &str) -> Result<TierList> {
        let params = [("ep", "list"), ("lane", lane), ("tier", tier)];
        self.fetch(&params).await
    }

    /// Runes, spells, skills and items of one champion in one lane.
    /// `alias` is the lowercase English name, e.g. `garen`.
    pub async fn build(&self, alias: &str, lane: &str, tier: &str) -> Result<BuildFull> {
        let params = [
            ("ep", "build-full"),
            ("c", alias),
            ("lane", lane),
            ("tier", tier),
        ];
        self.fetch(&params).await
    }

    /// Same-lane matchups of one champion against every opponent.
    pub async fn counters(&self, alias: &str, lane: &str, tier: &str) -> Result<CounterList> {
        let params = [
            ("ep", "counter"),
            ("c", alias),
            ("lane", lane),
            ("vslane", lane),
            ("tier", tier),
        ];
        self.fetch(&params).await
    }

    async fn fetch<T: DeserializeOwned>(&self, params: &[(&str, &str)]) -> Result<T> {
        let common = [
            ("v", "1"),
            ("patch", PATCH),
            ("queue", QUEUE),
            ("region", "all"),
        ];
        let request = self.client.get(BASE_URL).query(&common).query(params);
        let resp = request.send().await?;
        let status = resp.status();
        if !status.is_success() {
            return Err(AppError::Message(format!("lolalytics 返回 {status}")));
        }
        // Unknown parameters come back as a PHP dump or `{"status":404}`, not an HTTP error.
        let text = resp.text().await?;
        let value: Value = serde_json::from_str(&text)
            .map_err(|_| AppError::Message("lolalytics 返回了无效数据".to_owned()))?;
        if value.get("status").is_some() {
            return Err(AppError::Message(
                "lolalytics 没有这个英雄的数据".to_owned(),
            ));
        }
        let parsed = serde_json::from_value(value);
        parsed.map_err(|err| AppError::Message(err.to_string()))
    }
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub struct TierList {
    /// Keyed by champion id as a string.
    pub cid: HashMap<String, TierRow>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub struct TierRow {
    /// 0 when the champion is rarely played in this lane.
    pub rank: i64,
    /// 1 = S+, 2 = S, 3 = S-, 4 = A+ ... 15 = D-; 0 when unranked.
    pub tier: i64,
    pub wr: f64,
    pub pr: f64,
    pub br: f64,
    pub games: i64,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub struct CounterList {
    pub counters: Vec<CounterRow>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct CounterRow {
    /// Opponent champion id.
    pub cid: i64,
    /// Win rate against this opponent, in percent.
    pub vs_wr: f64,
    pub n: i64,
    /// Win-rate points beyond what both champions' overall win rates predict.
    pub d2: f64,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub struct BuildFull {
    pub header: BuildHeader,
    pub summary: BuildSummary,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct BuildHeader {
    pub tier: String,
    pub wr: f64,
    pub pr: f64,
    pub br: f64,
    pub n: i64,
    pub rank: i64,
    pub rank_total: i64,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub struct BuildSummary {
    /// Most picked choices.
    pub pick: BuildSet,
    /// Highest win-rate choices.
    pub win: BuildSet,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub struct BuildSet {
    pub skillpriority: SkillPriority,
    pub skillorder: SkillOrder,
    pub sums: Spells,
    pub runes: Runes,
    pub items: Items,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub struct SkillPriority {
    /// e.g. `EQW`.
    pub id: String,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub struct SkillOrder {
    /// One digit per level: 1 = Q, 2 = W, 3 = E, 4 = R.
    pub id: Value,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub struct Spells {
    pub ids: Vec<i64>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub struct Runes {
    pub wr: f64,
    pub n: i64,
    pub set: RuneSet,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub struct RuneSet {
    /// Keystone first, then the three minor runes of the primary tree.
    pub pri: Vec<i64>,
    pub sec: Vec<i64>,
    #[serde(rename = "mod")]
    pub shards: Vec<i64>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub struct Items {
    pub start: ItemSet,
    pub core: ItemSet,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub struct ItemSet {
    pub set: Vec<i64>,
    pub wr: f64,
    pub n: i64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_build_full() {
        let json = r#"{"header":{"tier":"A","wr":53.02,"n":391879,"rank":22,"rankTotal":107,
            "counters":{"strong":[897],"weak":[10]}},
            "summary":{"pick":{"skillpriority":{"id":"EQW"},"skillorder":{"id":312334313141122},
            "sums":{"ids":[4,14]},"runes":{"wr":52.9,"n":277913,
            "set":{"pri":[8010,9111,9105,8299],"sec":[8234,8224],"mod":[5008,5008,5001]}},
            "items":{"start":{"set":[1054,2003]},"core":{"set":[3006,6631,3046],"wr":55.05}}}}}"#;
        let build: BuildFull = serde_json::from_str(json).unwrap();
        assert_eq!(build.header.tier, "A");
        let pick = &build.summary.pick;
        assert_eq!(pick.runes.set.shards, vec![5008, 5008, 5001]);
        assert_eq!(pick.items.core.set.len(), 3);
        assert_eq!(pick.skillorder.id.to_string(), "312334313141122");
    }
}
