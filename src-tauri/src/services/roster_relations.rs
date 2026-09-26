//! Relations between the players of the current game, read from their recent histories:
//! premade groups (repeatedly on the same team) and players the user has met before.

use std::collections::{HashMap, HashSet, VecDeque};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::Serialize;

use super::match_history::{GameResult, GameSummary, MatchHistoryPage};
use super::player_profile::{PlayerTag, Tone};
use crate::state::ongoing::Roster;

/// Shared same-team games before two players count as premade. League Akari uses 5 over
/// longer histories; 3 in 20 recent games is already unlikely to happen by chance.
const PREMADE_MIN_SHARED: usize = 3;
const GROUP_NAMES: [&str; 5] = ["A", "B", "C", "D", "E"];
const DAY_MS: i64 = 24 * 60 * 60 * 1000;

#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RosterRelations {
    pub premades: Vec<PremadeGroup>,
    /// Relation tags per puuid; the frontend merges them with profile tags.
    pub tags: HashMap<String, Vec<PlayerTag>>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PremadeGroup {
    pub name: String,
    pub members: Vec<String>,
    pub allies: bool,
    /// Fewest shared games between two linked members.
    pub shared_games: usize,
}

/// A game seen in someone's history; `owner` is whose page it came from.
struct Seen<'a> {
    owner: &'a str,
    game: &'a GameSummary,
}

pub fn analyze(
    roster: &Roster,
    self_puuid: &str,
    pages: &[MatchHistoryPage],
    now_ms: i64,
) -> RosterRelations {
    let mut by_id: HashMap<i64, Seen> = HashMap::new();
    for page in pages {
        for game in &page.games {
            if game.game_id == roster.game_id || game.participants.is_empty() {
                continue;
            }
            let seen = Seen {
                owner: &page.puuid,
                game,
            };
            by_id.entry(game.game_id).or_insert(seen);
        }
    }
    let games: Vec<Seen> = by_id.into_values().collect();

    let mut relations = RosterRelations::default();
    for group in premade_groups(roster, &games) {
        let n = group.members.len();
        let others = n - 1;
        let (name, shared) = (&group.name, group.shared_games);
        let label = if n == 2 {
            format!("双排 {name}")
        } else {
            format!("{n}黑 {name}")
        };
        let detail = format!(
            "与同队另外 {others} 名玩家近期至少 {shared} 场同为队友，推断为预组队（小队 {name}）。"
        );
        let tone = if group.allies {
            Tone::Neutral
        } else {
            Tone::Warning
        };
        for member in &group.members {
            let tag = relation_tag("premade", label.clone(), tone, detail.clone(), 85);
            relations.tags.entry(member.clone()).or_default().push(tag);
        }
        relations.premades.push(group);
    }

    for (puuid, met) in encounters(roster, self_puuid, &games) {
        let label = format!("遇见过 {}", met.count);
        let tag = relation_tag("met", label, Tone::Neutral, met.detail(now_ms), 60);
        relations.tags.entry(puuid).or_default().push(tag);
    }
    relations
}

fn relation_tag(
    id: &'static str,
    label: String,
    tone: Tone,
    detail: String,
    priority: u8,
) -> PlayerTag {
    PlayerTag {
        id,
        label,
        tone,
        detail,
        low_confidence: false,
        priority,
    }
}

/// Links players of the same current side who were teammates in at least
/// `PREMADE_MIN_SHARED` recent games, then groups linked players.
fn premade_groups(roster: &Roster, games: &[Seen]) -> Vec<PremadeGroup> {
    let mut side: HashMap<&str, bool> = HashMap::new();
    for p in &roster.allies {
        side.insert(&p.puuid, true);
    }
    for p in &roster.enemies {
        side.insert(&p.puuid, false);
    }

    let mut pairs: HashMap<(&str, &str), usize> = HashMap::new();
    for seen in games {
        let mut teams: HashMap<i64, Vec<&str>> = HashMap::new();
        for p in &seen.game.participants {
            if side.contains_key(p.puuid.as_str()) {
                teams.entry(p.team_id).or_default().push(&p.puuid);
            }
        }
        for members in teams.values() {
            for (i, a) in members.iter().enumerate() {
                for b in &members[i + 1..] {
                    *pairs.entry(ordered(a, b)).or_default() += 1;
                }
            }
        }
    }

    let mut links: HashMap<&str, Vec<(&str, usize)>> = HashMap::new();
    for ((a, b), shared) in pairs {
        if shared >= PREMADE_MIN_SHARED && side[a] == side[b] {
            links.entry(a).or_default().push((b, shared));
            links.entry(b).or_default().push((a, shared));
        }
    }

    // Walk in roster order so group names are stable: allies first.
    let order = roster.allies.iter().chain(&roster.enemies);
    let mut visited: HashSet<&str> = HashSet::new();
    let mut groups = Vec::new();
    for start in order.map(|p| p.puuid.as_str()) {
        if visited.contains(start) || !links.contains_key(start) {
            continue;
        }
        let (members, shared_games) = component(start, &links, &mut visited);
        let name = GROUP_NAMES.get(groups.len()).copied().unwrap_or("?");
        groups.push(PremadeGroup {
            name: name.to_owned(),
            members,
            allies: side[start],
            shared_games,
        });
    }
    groups
}

fn component<'a>(
    start: &'a str,
    links: &HashMap<&'a str, Vec<(&'a str, usize)>>,
    visited: &mut HashSet<&'a str>,
) -> (Vec<String>, usize) {
    let mut members = Vec::new();
    let mut weakest = usize::MAX;
    let mut queue = VecDeque::from([start]);
    visited.insert(start);
    while let Some(puuid) = queue.pop_front() {
        members.push(puuid.to_owned());
        for &(next, shared) in links.get(puuid).into_iter().flatten() {
            weakest = weakest.min(shared);
            if visited.insert(next) {
                queue.push_back(next);
            }
        }
    }
    (members, weakest)
}

fn ordered<'a>(a: &'a str, b: &'a str) -> (&'a str, &'a str) {
    if a <= b {
        (a, b)
    } else {
        (b, a)
    }
}

struct Met {
    count: usize,
    teammate: usize,
    last_at: i64,
    last_teammate: bool,
    /// The user's result in that last game, when it can be told.
    last_result: Option<GameResult>,
}

impl Met {
    fn detail(&self, now_ms: i64) -> String {
        let (count, teammate) = (self.count, self.teammate);
        let opponent = count - teammate;
        let days = (now_ms - self.last_at).max(0) / DAY_MS;
        let when = if days == 0 {
            "今天".to_owned()
        } else {
            format!("{days} 天前")
        };
        let relation = if self.last_teammate {
            "队友"
        } else {
            "对手"
        };
        let outcome = match self.last_result {
            Some(GameResult::Win) => "，你赢了",
            Some(GameResult::Loss) => "，你输了",
            _ => "",
        };
        let counts = format!("近期同局 {count} 次（队友 {teammate}、对手 {opponent}）。");
        format!("{counts}上次在{when}，是{relation}{outcome}。")
    }
}

/// Everyone in the roster who shares a recent game with the user.
fn encounters(roster: &Roster, self_puuid: &str, games: &[Seen]) -> HashMap<String, Met> {
    let others: HashSet<&str> = roster.puuids().filter(|p| *p != self_puuid).collect();
    let mut met: HashMap<String, Met> = HashMap::new();
    if self_puuid.is_empty() {
        return met;
    }
    for seen in games {
        let participants = &seen.game.participants;
        let Some(me) = participants.iter().find(|p| p.puuid == self_puuid) else {
            continue;
        };
        for p in participants {
            if !others.contains(p.puuid.as_str()) {
                continue;
            }
            let teammate = p.team_id == me.team_id;
            let at = seen.game.created_at;
            let entry = met.entry(p.puuid.clone()).or_insert(Met {
                count: 0,
                teammate: 0,
                last_at: i64::MIN,
                last_teammate: teammate,
                last_result: None,
            });
            entry.count += 1;
            entry.teammate += usize::from(teammate);
            if at > entry.last_at {
                entry.last_at = at;
                entry.last_teammate = teammate;
                entry.last_result = self_result(seen, self_puuid, &p.puuid, teammate);
            }
        }
    }
    met
}

/// Each page states its owner's result; derive the user's from whichever page this was.
fn self_result(seen: &Seen, self_puuid: &str, other: &str, teammate: bool) -> Option<GameResult> {
    let result = seen.game.result;
    if seen.owner == self_puuid || (seen.owner == other && teammate) {
        return Some(result);
    }
    if seen.owner == other {
        return match result {
            GameResult::Win => Some(GameResult::Loss),
            GameResult::Loss => Some(GameResult::Win),
            _ => None,
        };
    }
    None
}

pub fn now_ms() -> i64 {
    let since = SystemTime::now().duration_since(UNIX_EPOCH);
    let elapsed = since.unwrap_or_default();
    i64::try_from(elapsed.as_millis()).unwrap_or(i64::MAX)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::match_history::{DataSource, GameParticipant};
    use crate::state::ongoing::{RosterPlayer, RosterStage};

    fn player(puuid: &str) -> RosterPlayer {
        RosterPlayer {
            puuid: puuid.to_owned(),
            champion_id: 0,
            position: String::new(),
            is_self: puuid == "me",
        }
    }

    fn roster() -> Roster {
        Roster {
            stage: RosterStage::InGame,
            game_id: 999,
            queue_id: 420,
            allies: ["me", "a1", "a2"].map(player).to_vec(),
            enemies: ["e1", "e2"].map(player).to_vec(),
            hidden_enemies: 0,
            enemy_champions: Vec::new(),
        }
    }

    /// A game where `blue` played on team 100 and `red` on team 200.
    fn game(id: i64, result: GameResult, blue: &[&str], red: &[&str]) -> GameSummary {
        let mut participants = on_team(blue, 100);
        participants.extend(on_team(red, 200));
        GameSummary {
            game_id: id,
            queue_id: 420,
            game_mode: "CLASSIC".to_owned(),
            created_at: id * DAY_MS,
            duration: 1800,
            result,
            champion_id: 1,
            champ_level: 18,
            kills: 1,
            deaths: 1,
            assists: 1,
            spells: [4, 14],
            items: [0; 7],
            cs: 200,
            gold: 10000,
            damage_to_champions: 15000,
            position: String::new(),
            team_id: 100,
            vision_score: 10,
            keystone: 0,
            sub_style: 0,
            largest_multi_kill: 0,
            metrics: None,
            participants,
        }
    }

    fn on_team(puuids: &[&str], team_id: i64) -> Vec<GameParticipant> {
        let mut out = Vec::new();
        for puuid in puuids {
            out.push(GameParticipant {
                puuid: (*puuid).to_owned(),
                team_id,
                champion_id: 0,
                position: String::new(),
                jungler: false,
            });
        }
        out
    }

    fn page(puuid: &str, games: Vec<GameSummary>) -> MatchHistoryPage {
        MatchHistoryPage {
            puuid: puuid.to_owned(),
            start: 0,
            count: 20,
            source: DataSource::Sgp,
            sgp_error: None,
            games,
        }
    }

    #[test]
    fn groups_players_who_keep_queueing_together() {
        let duo: Vec<GameSummary> = (1..=3)
            .map(|id| game(id, GameResult::Win, &["e1", "e2"], &["x"]))
            .collect();
        let pages = [page("e1", duo)];
        let relations = analyze(&roster(), "me", &pages, 10 * DAY_MS);
        assert_eq!(relations.premades.len(), 1);
        let group = &relations.premades[0];
        assert!(!group.allies);
        assert_eq!(group.shared_games, 3);
        assert_eq!(relations.tags["e1"][0].label, "双排 A");
    }

    #[test]
    fn ignores_the_current_game_and_chance_meetings() {
        let games = vec![
            game(999, GameResult::Win, &["a1", "a2"], &[]),
            game(1, GameResult::Win, &["a1", "a2"], &[]),
            game(2, GameResult::Win, &["a1", "a2"], &[]),
        ];
        let relations = analyze(&roster(), "me", &[page("a1", games)], 10 * DAY_MS);
        assert!(relations.premades.is_empty());
    }

    #[test]
    fn reports_met_players_with_the_users_result() {
        let games = vec![
            game(1, GameResult::Win, &["me"], &["e1"]),
            game(4, GameResult::Win, &["e1"], &["me"]),
        ];
        let relations = analyze(&roster(), "me", &[page("e1", games)], 6 * DAY_MS);
        let met = &relations.tags["e1"][0];
        assert_eq!(met.label, "遇见过 2");
        assert!(met.detail.contains("2 天前"));
        assert!(met.detail.contains("对手，你输了"));
    }
}
