use std::collections::HashMap;
use std::sync::LazyLock;

use serde::Deserialize;

const SERVERS_JSON: &str = include_str!("../../../resources/servers.json");

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SgpServer {
    pub match_history: String,
    pub name: String,
    pub region_path_param: Option<String>,
}

#[derive(Deserialize)]
struct ServersFile {
    servers: HashMap<String, SgpServer>,
}

static SERVERS: LazyLock<HashMap<String, SgpServer>> = LazyLock::new(|| {
    let file: Result<ServersFile, _> = serde_json::from_str(SERVERS_JSON);
    file.expect("servers.json is valid").servers
});

#[derive(Debug, Clone)]
pub struct ResolvedServer {
    pub id: String,
    /// Region segment of game ids in SGP paths, e.g. `HN1` in `HN1_11313055562`.
    pub path_region: String,
    pub server: SgpServer,
}

/// Maps the client's region and platform to a configured SGP server.
pub fn resolve(region: &str, platform_id: &str) -> Option<ResolvedServer> {
    let platform = platform_id.to_uppercase();
    let tencent = format!("TENCENT_{platform}");
    let ids = [server_id(region, &platform), tencent];
    let id = ids.into_iter().find(|id| SERVERS.contains_key(id))?;
    let server = SERVERS[&id].clone();
    let path_region = match &server.region_path_param {
        Some(param) => param.clone(),
        None if id.starts_with("TENCENT_") => platform,
        None => id.clone(),
    };
    Some(ResolvedServer {
        id,
        path_region,
        server,
    })
}

/// Same normalization as League Akari's `getSgpServerId`.
fn server_id(region: &str, platform: &str) -> String {
    let region = region.to_uppercase();
    if region == "TENCENT" {
        return format!("TENCENT_{platform}");
    }
    let id = match region.as_str() {
        "NA" => "NA1",
        "BR" => "BR1",
        "TR" => "TR1",
        "LAN" => "LA1",
        "LAS" => "LA2",
        "OCE" => "OC1",
        "EUW1" => "EUW",
        "JP1" => "JP",
        other => other,
    };
    id.to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolves_tencent_server() {
        let s = resolve("TENCENT", "hn1").unwrap();
        assert_eq!(s.id, "TENCENT_HN1");
        assert_eq!(s.path_region, "HN1");
        assert_eq!(s.server.name, "艾欧尼亚");
    }

    #[test]
    fn falls_back_to_tencent_platform_when_region_is_unknown() {
        assert_eq!(resolve("", "HN10").unwrap().id, "TENCENT_HN10");
    }

    #[test]
    fn normalizes_riot_regions() {
        assert_eq!(resolve("EUW1", "EUW1").unwrap().path_region, "EUW1");
        assert_eq!(resolve("NA", "NA1").unwrap().id, "NA1");
    }

    #[test]
    fn unknown_server_is_none() {
        assert!(resolve("MARS", "MARS1").is_none());
    }
}
