use serde::Serialize;

use crate::clients::lcu::models::Summoner;
use crate::error::{AppError, Result};
use crate::state::session::LcuSession;

const ALIASES: &str = "/lol-summoner/v1/summoners/aliases";

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Alias<'a> {
    game_name: &'a str,
    tag_line: &'a str,
}

/// Looks up `名字#标签` on the current server.
pub async fn by_riot_id(session: &LcuSession, riot_id: &str) -> Result<Summoner> {
    let Some((name, tag)) = parse_riot_id(riot_id) else {
        return Err(AppError::Message("请输入完整的 Riot ID，例如 名字#12345".to_owned()));
    };
    let alias = Alias {
        game_name: name,
        tag_line: tag,
    };
    let found: Vec<Option<Summoner>> = session.http.post(ALIASES, &[alias]).await?;
    let mut found = found.into_iter().flatten();
    let Some(summoner) = found.find(|s| !s.puuid.is_empty()) else {
        return Err(AppError::Message(format!("找不到召唤师 {name}#{tag}")));
    };
    Ok(summoner)
}

pub async fn by_puuid(session: &LcuSession, puuid: &str) -> Result<Summoner> {
    let path = format!("/lol-summoner/v2/summoners/puuid/{puuid}");
    session.http.get(&path).await
}

fn parse_riot_id(riot_id: &str) -> Option<(&str, &str)> {
    let (name, tag) = riot_id.rsplit_once('#')?;
    let (name, tag) = (name.trim(), tag.trim());
    if name.is_empty() || tag.is_empty() {
        return None;
    }
    Some((name, tag))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_riot_id() {
        assert_eq!(parse_riot_id(" 小明 # 12345 "), Some(("小明", "12345")));
        assert_eq!(parse_riot_id("a#b#c"), Some(("a#b", "c")));
        assert_eq!(parse_riot_id("小明"), None);
        assert_eq!(parse_riot_id("小明#"), None);
    }
}
