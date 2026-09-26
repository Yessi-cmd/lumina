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
    /// Hover text, as plain text.
    pub items: HashMap<i64, Described>,
    pub spells: HashMap<i64, Described>,
    pub perk_descriptions: HashMap<i64, String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Described {
    pub name: String,
    pub description: String,
    /// Items only; 0 otherwise.
    pub price: i64,
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
    let mut perk_descriptions = HashMap::new();
    for perk in perks.into_iter().chain(styles.styles) {
        if !perk.short_desc.is_empty() {
            perk_descriptions.insert(perk.id, plain_text(&perk.short_desc));
        }
        perk_icons.insert(perk.id, perk.icon_path);
        perk_names.insert(perk.id, perk.name);
    }
    let item_info = described(&items);
    let spell_info = described(&spells);

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
        item_icons: icons(&items),
        spell_icons: icons(&spells),
        queue_names,
        perk_icons,
        perk_names,
        items: item_info,
        spells: spell_info,
        perk_descriptions,
    })
}

fn described(assets: &[LcuIconAsset]) -> HashMap<i64, Described> {
    let mut out = HashMap::new();
    for asset in assets {
        let entry = Described {
            name: asset.name.clone(),
            description: plain_text(&asset.description),
            price: asset.price_total,
        };
        out.insert(asset.id, entry);
    }
    out
}

/// The client's rich text (`<mainText><stats>...</stats><br>...`) as plain lines.
fn plain_text(markup: &str) -> String {
    let mut out = String::new();
    let mut rest = markup;
    while let Some(start) = rest.find('<') {
        out.push_str(&rest[..start]);
        let Some(end) = rest[start..].find('>') else {
            rest = "";
            break;
        };
        let tag = &rest[start + 1..start + end];
        if tag.starts_with("br") || tag.starts_with("/li") || tag == "li" {
            out.push('\n');
        }
        rest = &rest[start + end + 1..];
    }
    out.push_str(rest);
    let text = out.replace("&nbsp;", " ");
    let lines: Vec<&str> = text.lines().map(str::trim).collect();
    let mut joined = lines.join("\n");
    while joined.contains("\n\n\n") {
        joined = joined.replace("\n\n\n", "\n\n");
    }
    joined.trim().to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strips_client_markup() {
        let markup = "<mainText><stats><attention>40</attention> 攻击力</stats><br><br>\
                      <passive>重伤</passive>：造成伤害</mainText>";
        assert_eq!(plain_text(markup), "40 攻击力\n\n重伤：造成伤害");
    }
}

fn icons(assets: &[LcuIconAsset]) -> HashMap<i64, String> {
    let icons = assets.iter().map(|a| (a.id, a.icon_path.clone()));
    icons.collect()
}
