//! Who the player queues with and how those games go: the teammates met most often, and
//! the record with any of them against the record without.
//!
//! Only SGP pages list every player of a game, so games from the LCU fallback are not
//! counted. Arena has no plain teams and remakes decide nothing; both are left out.

use std::collections::{HashMap, HashSet};

use futures_util::future::join_all;
use serde::Serialize;

use super::career::{self, Range};
use super::match_history::{GameResult, GameSummary, MatchHistoryService};
use super::summoner;
use crate::error::Result;
use crate::state::session::LcuSession;

const ARENA: i64 = 1700;
/// Fewer games together say nothing about a pairing.
const MIN_GAMES: usize = 3;
const SHOWN: usize = 10;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Record {
    pub games: usize,
    pub wins: usize,
}

impl Record {
    fn add(&mut self, result: GameResult) {
        self.games += 1;
        self.wins += usize::from(result == GameResult::Win);
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Teammate {
    pub puuid: String,
    /// Empty when the name could not be looked up.
    pub name: String,
    pub games: usize,
    pub wins: usize,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Teammates {
    /// Games that list their players.
    pub games: usize,
    /// Most games together first.
    pub regulars: Vec<Teammate>,
    /// Games with at least one of the regulars.
    pub with_regulars: Record,
    /// Games with none of them.
    pub without_regulars: Record,
}

pub async fn load(
    history: &MatchHistoryService,
    session: &LcuSession,
    puuid: &str,
    queue: Option<i64>,
    range: Range,
) -> Result<Teammates> {
    let (games, _) = career::games(history, session, puuid, queue, range).await?;
    let mut found = analyze(&games, puuid);
    let lookups = found
        .regulars
        .iter()
        .map(|t| summoner::by_puuid(session, &t.puuid));
    let names = join_all(lookups).await;
    for (teammate, name) in found.regulars.iter_mut().zip(names) {
        if let Ok(summoner) = name {
            teammate.name = display_name(&summoner.game_name, &summoner.tag_line);
        }
    }
    Ok(found)
}

fn display_name(game_name: &str, tag_line: &str) -> String {
    if tag_line.is_empty() {
        return game_name.to_owned();
    }
    format!("{game_name}#{tag_line}")
}

/// Games with a result that list their players.
fn usable(game: &GameSummary) -> bool {
    let decided = matches!(game.result, GameResult::Win | GameResult::Loss);
    decided && game.queue_id != ARENA && !game.participants.is_empty()
}

/// The puuids of `me`'s teammates in `game`.
fn teammates_in<'a>(game: &'a GameSummary, me: &'a str) -> impl Iterator<Item = &'a str> + 'a {
    let team = game.team_id;
    let others = game.participants.iter();
    others
        .filter(move |p| p.team_id == team && p.puuid != me)
        .map(|p| p.puuid.as_str())
}

fn analyze(games: &[GameSummary], me: &str) -> Teammates {
    let listed: Vec<&GameSummary> = games.iter().filter(|g| usable(g)).collect();

    let mut together: HashMap<&str, Record> = HashMap::new();
    for &game in &listed {
        for other in teammates_in(game, me) {
            together.entry(other).or_default().add(game.result);
        }
    }
    let mut regulars: Vec<(&str, Record)> = together
        .into_iter()
        .filter(|(_, record)| record.games >= MIN_GAMES)
        .collect();
    regulars.sort_by(|a, b| {
        let by_games = b.1.games.cmp(&a.1.games);
        by_games.then(b.1.wins.cmp(&a.1.wins)).then(a.0.cmp(b.0))
    });
    regulars.truncate(SHOWN);

    let known: HashSet<&str> = regulars.iter().map(|(puuid, _)| *puuid).collect();
    let mut with_regulars = Record::default();
    let mut without_regulars = Record::default();
    for &game in &listed {
        if teammates_in(game, me).any(|p| known.contains(p)) {
            with_regulars.add(game.result);
        } else {
            without_regulars.add(game.result);
        }
    }

    let mut list = Vec::new();
    for (puuid, record) in regulars {
        list.push(Teammate {
            puuid: puuid.to_owned(),
            name: String::new(),
            games: record.games,
            wins: record.wins,
        });
    }
    Teammates {
        games: listed.len(),
        regulars: list,
        with_regulars,
        without_regulars,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::match_history::GameParticipant;

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

    /// `me` and `team` play on team 100 against `enemies` on team 200.
    fn game(result: GameResult, team: &[&str], enemies: &[&str]) -> GameSummary {
        let mut participants = on_team(team, 100);
        participants.extend(on_team(enemies, 200));
        GameSummary {
            game_id: 1,
            queue_id: 420,
            game_mode: "CLASSIC".to_owned(),
            created_at: 0,
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
            comparison: None,
            badge: None,
            augments: Vec::new(),
        }
    }

    const WIN: GameResult = GameResult::Win;
    const LOSS: GameResult = GameResult::Loss;

    #[test]
    fn counts_teammates_but_not_enemies_or_oneself() {
        let games = [
            game(WIN, &["me", "a", "b"], &["e"]),
            game(WIN, &["me", "a"], &["e"]),
            game(LOSS, &["me", "a"], &["e"]),
            game(WIN, &["me", "a"], &["e", "b"]),
        ];
        let found = analyze(&games, "me");
        assert_eq!(found.games, 4);
        assert_eq!(found.regulars.len(), 1);
        assert_eq!(found.regulars[0].puuid, "a");
        assert_eq!(found.regulars[0].games, 4);
        assert_eq!(found.regulars[0].wins, 3);
    }

    #[test]
    fn remakes_arena_and_unlisted_games_are_left_out() {
        let mut arena = game(WIN, &["me", "a"], &["e"]);
        arena.queue_id = ARENA;
        let mut unlisted = game(WIN, &["me", "a"], &["e"]);
        unlisted.participants.clear();
        let games = [
            game(GameResult::Remake, &["me", "a"], &["e"]),
            arena,
            unlisted,
            game(WIN, &["me", "a"], &["e"]),
        ];
        let found = analyze(&games, "me");
        assert_eq!(found.games, 1);
        assert!(found.regulars.is_empty());
    }

    #[test]
    fn records_with_and_without_the_regulars() {
        let games = [
            game(WIN, &["me", "a"], &["e"]),
            game(WIN, &["me", "a"], &["e"]),
            game(LOSS, &["me", "a"], &["e"]),
            game(LOSS, &["me", "x"], &["e"]),
            game(LOSS, &["me", "y"], &["e"]),
            game(WIN, &["me", "z"], &["e"]),
        ];
        let found = analyze(&games, "me");
        assert_eq!(found.with_regulars, Record { games: 3, wins: 2 });
        assert_eq!(found.without_regulars, Record { games: 3, wins: 1 });
    }

    #[test]
    fn most_games_together_come_first() {
        let mut games = Vec::new();
        for _ in 0..3 {
            games.push(game(WIN, &["me", "a"], &["e"]));
        }
        for _ in 0..5 {
            games.push(game(LOSS, &["me", "b"], &["e"]));
        }
        let found = analyze(&games, "me");
        assert_eq!(found.regulars[0].puuid, "b");
        assert_eq!(found.regulars[1].puuid, "a");
    }

    #[test]
    fn names_join_game_name_and_tag() {
        assert_eq!(display_name("小明", "12345"), "小明#12345");
        assert_eq!(display_name("小明", ""), "小明");
    }
}
