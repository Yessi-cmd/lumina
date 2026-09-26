//! Champ select assistant: which champions are strong in a position, how to build one, and
//! writing the chosen runes and summoner spells into the client.

use std::collections::{HashMap, HashSet};
use std::sync::{Mutex, PoisonError};
use std::time::{Duration, Instant};

use reqwest::Method;
use serde::{Deserialize, Serialize};
use serde_json::json;

use super::game_data;
use crate::clients::lcu::models::{ChampSelectSession, LcuOwnedChampion, LcuPerkPage};
use crate::clients::lolalytics::{BuildFull, BuildSet, LolalyticsClient, TierList};
use crate::error::{AppError, Result};
use crate::state::session::LcuSession;

const CACHE_TTL: Duration = Duration::from_secs(6 * 60 * 60);
const OWNED: &str = "/lol-champions/v1/owned-champions-minimal";
const PERK_PAGES: &str = "/lol-perks/v1/pages";
const CURRENT_PERK_PAGE: &str = "/lol-perks/v1/currentpage";
const CHAMP_SELECT: &str = "/lol-champ-select/v1/session";
const MY_SELECTION: &str = "/lol-champ-select/v1/session/my-selection";
/// Rune pages Lumina created start with this, so the next apply can replace them.
const PAGE_PREFIX: &str = "Lumina";
const FLASH: i64 = 4;
const TIER_LABELS: [&str; 15] = [
    "S+", "S", "S-", "A+", "A", "A-", "B+", "B", "B-", "C+", "C", "C-", "D+", "D", "D-",
];

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TierEntry {
    pub champion_id: i64,
    pub rank: i64,
    pub tier: String,
    /// Percentages, e.g. 52.3.
    pub win_rate: f64,
    pub pick_rate: f64,
    pub ban_rate: f64,
    pub games: i64,
    /// Owned or free this week; `false` when ownership could not be read.
    pub owned: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ChampionBuild {
    pub champion_id: i64,
    pub position: String,
    pub tier: String,
    pub rank: i64,
    pub rank_total: i64,
    pub win_rate: f64,
    pub pick_rate: f64,
    pub ban_rate: f64,
    pub games: i64,
    pub strong_against: Vec<i64>,
    pub weak_against: Vec<i64>,
    /// "最常用" then "最高胜率".
    pub variants: Vec<BuildVariant>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BuildVariant {
    pub label: String,
    pub rune_win_rate: f64,
    pub rune_games: i64,
    pub runes: RunePage,
    pub spells: Vec<i64>,
    /// e.g. `E > Q > W`.
    pub skill_priority: String,
    /// First levels, e.g. `Q E W Q Q R`.
    pub skill_order: String,
    pub start_items: Vec<i64>,
    pub core_items: Vec<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RunePage {
    pub primary_style: i64,
    pub sub_style: i64,
    /// Four primary runes, two secondary runes, three stat shards.
    pub perks: Vec<i64>,
}

/// lolalytics responses cached in memory for a few hours.
pub struct ChampionAssist {
    client: Option<LolalyticsClient>,
    tier_lists: Mutex<HashMap<String, (Instant, Vec<TierEntry>)>>,
    builds: Mutex<HashMap<String, (Instant, ChampionBuild)>>,
}

impl Default for ChampionAssist {
    fn default() -> Self {
        let client = LolalyticsClient::new();
        if let Err(err) = &client {
            log::warn!("lolalytics client unavailable: {err}");
        }
        Self {
            client: client.ok(),
            tier_lists: Mutex::default(),
            builds: Mutex::default(),
        }
    }
}

impl ChampionAssist {
    /// Champions ranked for one position (`TOP`, `JUNGLE`, `MIDDLE`, `BOTTOM`, `UTILITY`).
    pub async fn tier_list(
        &self,
        session: &LcuSession,
        position: &str,
        tier: &str,
    ) -> Result<Vec<TierEntry>> {
        let lane = lane(position)?;
        let key = format!("{lane}:{tier}");
        let mut entries = match cached(&self.tier_lists, &key) {
            Some(entries) => entries,
            None => {
                let list = self.client()?.tier_list(lane, tier).await?;
                let entries = ranked_entries(list);
                store(&self.tier_lists, key, entries.clone());
                entries
            }
        };

        let owned = owned_champions(session).await;
        for entry in &mut entries {
            entry.owned = owned.contains(&entry.champion_id);
        }
        Ok(entries)
    }

    pub async fn build(
        &self,
        session: &LcuSession,
        champion_id: i64,
        position: &str,
        tier: &str,
    ) -> Result<ChampionBuild> {
        let lane = lane(position)?;
        let key = format!("{champion_id}:{lane}:{tier}");
        if let Some(build) = cached(&self.builds, &key) {
            return Ok(build);
        }
        let data = game_data::get(session).await?;
        let Some(champion) = data.champions.get(&champion_id) else {
            return Err(AppError::Message(format!("未知英雄 {champion_id}")));
        };
        let alias = lolalytics_alias(&champion.alias);
        let full = self.client()?.build(&alias, lane, tier).await?;
        let build = to_build(champion_id, position, full);
        store(&self.builds, key, build.clone());
        Ok(build)
    }

    fn client(&self) -> Result<&LolalyticsClient> {
        let client = self.client.as_ref();
        client.ok_or_else(|| AppError::Message("lolalytics 客户端不可用".to_owned()))
    }
}

/// Replaces Lumina's previous rune page (or the current one when pages are full), then
/// sets the summoner spells while keeping Flash on the key the player uses.
pub async fn apply(session: &LcuSession, title: &str, variant: &BuildVariant) -> Result<()> {
    apply_runes(session, title, &variant.runes).await?;
    if let &[first, second] = variant.spells.as_slice() {
        apply_spells(session, first, second).await?;
    }
    Ok(())
}

async fn apply_runes(session: &LcuSession, title: &str, page: &RunePage) -> Result<()> {
    let http = &session.http;
    let pages: Vec<LcuPerkPage> = http.get(PERK_PAGES).await?;
    for old in pages.iter().filter(|p| p.name.starts_with(PAGE_PREFIX)) {
        if old.is_deletable {
            let path = format!("{PERK_PAGES}/{}", old.id);
            http.send_json::<()>(Method::DELETE, &path, None).await?;
        }
    }

    let body = json!({
        "name": format!("{PAGE_PREFIX} {title}"),
        "primaryStyleId": page.primary_style,
        "subStyleId": page.sub_style,
        "selectedPerkIds": page.perks,
        "current": true,
    });
    let created = http.send_json(Method::POST, PERK_PAGES, Some(&body)).await;
    if let Err(err) = created {
        // Usually the page limit: overwrite the current page instead, if it may be edited.
        log::info!("creating a rune page failed ({err}); overwriting the current page");
        let current: LcuPerkPage = http.get(CURRENT_PERK_PAGE).await?;
        if !current.is_editable {
            return Err(AppError::Message(
                "符文页已满，且当前符文页不可编辑".to_owned(),
            ));
        }
        let path = format!("{PERK_PAGES}/{}", current.id);
        http.send_json(Method::PUT, &path, Some(&body)).await?;
    }
    log::info!("applied rune page {title}");
    Ok(())
}

async fn apply_spells(session: &LcuSession, first: i64, second: i64) -> Result<()> {
    let http = &session.http;
    let champ_select: ChampSelectSession = http.get(CHAMP_SELECT).await?;
    let me = champ_select.my_team.iter();
    let me = me.find(|m| m.cell_id == champ_select.local_player_cell_id);
    let flash_on_f = me.is_some_and(|m| m.spell2_id == FLASH);
    let (d, f) = order_spells(first, second, flash_on_f);
    let body = json!({ "spell1Id": d, "spell2Id": f });
    http.send_json(Method::PATCH, MY_SELECTION, Some(&body))
        .await
}

/// Keeps Flash on F for players who already have it there.
fn order_spells(first: i64, second: i64, flash_on_f: bool) -> (i64, i64) {
    if flash_on_f && first == FLASH {
        (second, first)
    } else if !flash_on_f && second == FLASH {
        (second, first)
    } else {
        (first, second)
    }
}

async fn owned_champions(session: &LcuSession) -> HashSet<i64> {
    match session.http.get::<Vec<LcuOwnedChampion>>(OWNED).await {
        Ok(owned) => owned.into_iter().map(|c| c.id).collect(),
        Err(err) => {
            log::warn!("failed to read owned champions: {err}");
            HashSet::new()
        }
    }
}

/// Champions ranked in this lane, best first; ownership is filled in per request.
fn ranked_entries(list: TierList) -> Vec<TierEntry> {
    let mut entries = Vec::new();
    for (id, row) in list.cid {
        let Ok(champion_id) = id.parse::<i64>() else {
            continue;
        };
        if row.rank <= 0 {
            continue;
        }
        entries.push(TierEntry {
            champion_id,
            rank: row.rank,
            tier: tier_label(row.tier),
            win_rate: row.wr,
            pick_rate: row.pr,
            ban_rate: row.br,
            games: row.games,
            owned: false,
        });
    }
    entries.sort_by_key(|e| e.rank);
    entries
}

fn to_build(champion_id: i64, position: &str, full: BuildFull) -> ChampionBuild {
    let header = full.header;
    let variants = vec![
        variant("最常用", full.summary.pick),
        variant("最高胜率", full.summary.win),
    ];
    ChampionBuild {
        champion_id,
        position: position.to_owned(),
        tier: header.tier,
        rank: header.rank,
        rank_total: header.rank_total,
        win_rate: header.wr,
        pick_rate: header.pr,
        ban_rate: header.br,
        games: header.n,
        strong_against: header.counters.strong,
        weak_against: header.counters.weak,
        variants,
    }
}

fn variant(label: &str, set: BuildSet) -> BuildVariant {
    let runes = set.runes.set;
    let mut perks = runes.pri.clone();
    perks.extend(&runes.sec);
    perks.extend(&runes.shards);
    let primary_style = runes.pri.first().map_or(0, |&p| rune_tree(p));
    let sub_style = runes.sec.first().map_or(0, |&p| rune_tree(p));
    BuildVariant {
        label: label.to_owned(),
        rune_win_rate: set.runes.wr,
        rune_games: set.runes.n,
        runes: RunePage {
            primary_style,
            sub_style,
            perks,
        },
        spells: set.sums.ids,
        skill_priority: skill_priority(&set.skillpriority.id),
        skill_order: skill_order(&set.skillorder.id.to_string()),
        start_items: set.items.start.set,
        core_items: set.items.core.set,
    }
}

/// Rune tree of a rune id: 80xx and the 91xx Precision minors, 81xx, 82xx, 83xx, 84xx.
fn rune_tree(perk: i64) -> i64 {
    match perk {
        9100..=9199 => 8000,
        8000..=8499 => perk / 100 * 100,
        _ => 0,
    }
}

fn skill_priority(id: &str) -> String {
    let keys: Vec<String> = id.chars().map(String::from).collect();
    keys.join(" > ")
}

/// `312334313141122` → `E Q W E E R`: the first six levels.
fn skill_order(digits: &str) -> String {
    let keys: Vec<&str> = digits
        .chars()
        .take(6)
        .filter_map(|d| match d {
            '1' => Some("Q"),
            '2' => Some("W"),
            '3' => Some("E"),
            '4' => Some("R"),
            _ => None,
        })
        .collect();
    keys.join(" ")
}

fn lane(position: &str) -> Result<&'static str> {
    match position {
        "TOP" => Ok("top"),
        "JUNGLE" => Ok("jungle"),
        "MIDDLE" => Ok("middle"),
        "BOTTOM" => Ok("bottom"),
        "UTILITY" => Ok("support"),
        _ => Err(AppError::Message(format!("未知位置 {position}"))),
    }
}

/// lolalytics URLs use the lowercase English name, except where it differs from the alias.
fn lolalytics_alias(alias: &str) -> String {
    match alias {
        "MonkeyKing" => "wukong".to_owned(),
        other => other.to_lowercase(),
    }
}

fn tier_label(tier: i64) -> String {
    let index = usize::try_from(tier - 1).ok();
    let label = index.and_then(|i| TIER_LABELS.get(i));
    label.copied().unwrap_or("-").to_owned()
}

fn cached<T: Clone>(cache: &Mutex<HashMap<String, (Instant, T)>>, key: &str) -> Option<T> {
    let cache = cache.lock().unwrap_or_else(PoisonError::into_inner);
    let (at, value) = cache.get(key)?;
    (at.elapsed() < CACHE_TTL).then(|| value.clone())
}

fn store<T>(cache: &Mutex<HashMap<String, (Instant, T)>>, key: String, value: T) {
    let mut cache = cache.lock().unwrap_or_else(PoisonError::into_inner);
    cache.retain(|_, (at, _)| at.elapsed() < CACHE_TTL);
    cache.insert(key, (Instant::now(), value));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_runes_to_trees() {
        assert_eq!(rune_tree(8010), 8000);
        assert_eq!(rune_tree(9111), 8000);
        assert_eq!(rune_tree(8234), 8200);
        assert_eq!(rune_tree(8444), 8400);
        assert_eq!(rune_tree(5008), 0);
    }

    #[test]
    fn formats_skills_and_tiers() {
        assert_eq!(skill_order("312334313141122"), "E Q W E E R");
        assert_eq!(skill_priority("EQW"), "E > Q > W");
        assert_eq!(tier_label(1), "S+");
        assert_eq!(tier_label(5), "A");
        assert_eq!(tier_label(0), "-");
    }

    #[test]
    fn keeps_flash_on_the_players_key() {
        assert_eq!(order_spells(4, 14, true), (14, 4));
        assert_eq!(order_spells(4, 14, false), (4, 14));
        assert_eq!(order_spells(14, 4, false), (4, 14));
        assert_eq!(order_spells(11, 12, true), (11, 12));
    }

    #[test]
    fn aliases_for_lolalytics() {
        assert_eq!(lolalytics_alias("MonkeyKing"), "wukong");
        assert_eq!(lolalytics_alias("Garen"), "garen");
    }
}
