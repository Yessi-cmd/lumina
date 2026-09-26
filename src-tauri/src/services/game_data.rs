//! Static lookup tables (champions, items, spells, queues) in the client's language.

use std::collections::HashMap;

use serde::Serialize;

use crate::clients::lcu::http::LcuHttp;
use crate::clients::lcu::models::{LcuChampion, LcuIconAsset, LcuPerkStyles, LcuQueue};
use crate::error::Result;
use crate::state::session::LcuSession;

const CHAMPIONS: &str = "/lol-game-data/assets/v1/champion-summary.json";
const ITEMS: &str = "/lol-game-data/assets/v1/items.json";
const SPELLS: &str = "/lol-game-data/assets/v1/summoner-spells.json";
const QUEUES: &str = "/lol-game-queues/v1/queues";
const PERKS: &str = "/lol-game-data/assets/v1/perks.json";
const PERK_STYLES: &str = "/lol-game-data/assets/v1/perkstyles.json";

/// Icon values are LCU asset paths, served to the webview through `lcu-asset://`.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GameData {
    pub champions: HashMap<i64, Champion>,
    pub item_icons: HashMap<i64, String>,
    pub spell_icons: HashMap<i64, String>,
    pub queue_names: HashMap<i64, String>,
    /// Runes, rune trees (8000, 8100, ...) and stat shards.
    pub perk_icons: HashMap<i64, String>,
    pub perk_names: HashMap<i64, String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Champion {
    pub name: String,
    pub icon: String,
    /// English id such as `Garen`, used by third-party statistics sites.
    pub alias: String,
}

/// Loaded once per client session.
pub async fn get(session: &LcuSession) -> Result<GameData> {
    let cell = &session.game_data;
    let data = cell.get_or_try_init(|| fetch(&session.http)).await?;
    Ok(data.clone())
}

async fn fetch(http: &LcuHttp) -> Result<GameData> {
    let (champions, items, spells, queues) = tokio::try_join!(
        http.get::<Vec<LcuChampion>>(CHAMPIONS),
        http.get::<Vec<LcuIconAsset>>(ITEMS),
        http.get::<Vec<LcuIconAsset>>(SPELLS),
        http.get::<Vec<LcuQueue>>(QUEUES),
    )?;
    let (perks, styles) = tokio::try_join!(
        http.get::<Vec<LcuIconAsset>>(PERKS),
        http.get::<LcuPerkStyles>(PERK_STYLES),
    )?;
    let mut perk_icons = HashMap::new();
    let mut perk_names = HashMap::new();
    for perk in perks.into_iter().chain(styles.styles) {
        perk_icons.insert(perk.id, perk.icon_path);
        perk_names.insert(perk.id, perk.name);
    }

    let mut champion_map = HashMap::new();
    for c in champions.into_iter().filter(|c| c.id > 0) {
        let champion = Champion {
            name: c.name,
            icon: c.square_portrait_path,
            alias: c.alias,
        };
        champion_map.insert(c.id, champion);
    }

    let mut queue_names = HashMap::new();
    for q in queues {
        let name = if q.description.is_empty() {
            q.name
        } else {
            q.description
        };
        queue_names.insert(q.id, name);
    }

    Ok(GameData {
        champions: champion_map,
        item_icons: icons(items),
        spell_icons: icons(spells),
        queue_names,
        perk_icons,
        perk_names,
    })
}

fn icons(assets: Vec<LcuIconAsset>) -> HashMap<i64, String> {
    assets.into_iter().map(|a| (a.id, a.icon_path)).collect()
}
