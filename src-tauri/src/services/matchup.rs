//! 田忌赛马: who is strong and who is weak in this game, and where to spend resources.
//!
//! Every identified player gets a power index (0–100, 50 = average) from solo/duo and flex
//! games only, recent games and games on the current position weighted most: win rate
//! shrunk towards 50% for small samples, performance against the average player on the
//! same position, and gold against the lane opponent at 10 minutes. Players are then
//! ranked within their own team (上等马 / 下等马), opponents are called out as 硬骨头 /
//! 软柿子, same-position players are compared lane by lane, and early deaths to the enemy
//! jungler mark players as easy or hard to gank.

use std::collections::HashMap;

use serde::Serialize;

use super::player_profile::{PlayerProfile, PlayerTag, RankedForm, Tone};
use super::timeline::EarlyStats;
use crate::state::ongoing::{Roster, RosterPlayer};

/// Every threshold in one place.
mod limits {
    /// Fewer solo/duo and flex games and the player gets no power index.
    pub const POWER_MIN_GAMES: usize = 5;
    /// Pseudo-games at 50% mixed into the win rate, so 3-0 is not treated as 100%.
    pub const WIN_RATE_PRIOR_GAMES: f64 = 4.0;
    pub const WIN_RATE_WEIGHT: f64 = 50.0;
    /// Power per unit of ranked performance (-1..1), and the most it can add or remove.
    pub const PERFORMANCE_WEIGHT: f64 = 24.0;
    pub const PERFORMANCE_CAP: f64 = 12.0;
    /// Gold at 10 minutes per power point, and the most it can add or remove.
    pub const LANE_GOLD_PER_POINT: f64 = 40.0;
    pub const LANE_CAP: f64 = 12.0;
    pub const LANE_MIN_GAMES: usize = 3;
    /// Distance from the team median before someone is called 上等马 / 下等马.
    pub const TIER_GAP: f64 = 6.0;
    pub const TIER_MIN_PLAYERS: usize = 3;
    /// Power gap between lane opponents that counts as an advantage.
    pub const LANE_EDGE: f64 = 10.0;
    /// League Akari: more than 2 early deaths to the jungler is very easy, 1.5 is easy.
    pub const VERY_EASY_GANK: f64 = 2.0;
    pub const EASY_GANK: f64 = 1.5;
    /// Lumina: below this the player rarely dies to early ganks.
    pub const HARD_GANK: f64 = 0.5;
    pub const GANK_MIN_GAMES: usize = 4;
    pub const STRONG_LANE_GOLD: f64 = 350.0;
    pub const LANE_TAG_MIN_GAMES: usize = 4;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Tier {
    Top,
    Bottom,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlayerPower {
    pub power: f64,
    pub tier: Option<Tier>,
    pub ally: bool,
    /// How the index was built, for the hover text.
    pub breakdown: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LaneMatchup {
    pub position: String,
    pub ally: String,
    pub enemy: String,
    pub ally_power: f64,
    pub enemy_power: f64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Advice {
    pub tone: Tone,
    /// Player the advice is about, for the champion icon.
    pub puuid: String,
    pub title: String,
    pub detail: String,
}

#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Matchup {
    pub powers: HashMap<String, PlayerPower>,
    pub lanes: Vec<LaneMatchup>,
    pub advice: Vec<Advice>,
    #[serde(skip)]
    pub tags: HashMap<String, Vec<PlayerTag>>,
}

/// One roster player with everything known about them.
struct Seat<'a> {
    player: &'a RosterPlayer,
    ally: bool,
    position: String,
    power: Option<f64>,
    early: Option<&'a EarlyStats>,
}

pub fn analyze(
    roster: &Roster,
    profiles: &HashMap<String, PlayerProfile>,
    early: &HashMap<String, EarlyStats>,
) -> Matchup {
    let mut matchup = Matchup::default();
    let mut seats = Vec::new();
    let sides = [(&roster.allies, true), (&roster.enemies, false)];
    for (players, ally) in sides {
        for player in players.iter() {
            let profile = profiles.get(&player.puuid);
            let stats = early.get(&player.puuid);
            let position = match profile {
                Some(p) if player.position.is_empty() => p.position.clone(),
                _ => player.position.clone(),
            };
            let power = profile.and_then(|p| power_index(p, stats));
            if let (Some(p), Some(power)) = (profile, power) {
                let breakdown = breakdown(p, stats, power);
                let entry = PlayerPower {
                    power,
                    tier: None,
                    ally,
                    breakdown,
                };
                matchup.powers.insert(player.puuid.clone(), entry);
            }
            seats.push(Seat {
                player,
                ally,
                position,
                power,
                early: stats,
            });
        }
    }

    for seat in &seats {
        for tag in gank_and_lane_tags(seat) {
            matchup.add_tag(&seat.player.puuid, tag);
        }
    }
    let enemies_known = !roster.enemies.is_empty();
    for ally in [true, false] {
        let side: Vec<&Seat> = seats.iter().filter(|s| s.ally == ally).collect();
        matchup.rank_side(&side, enemies_known);
    }
    matchup.compare_lanes(&seats);
    matchup
}

impl Matchup {
    fn add_tag(&mut self, puuid: &str, tag: PlayerTag) {
        self.tags.entry(puuid.to_owned()).or_default().push(tag);
    }

    /// 上等马 / 下等马 within one team; for opponents they read as 硬骨头 / 软柿子.
    fn rank_side(&mut self, side: &[&Seat], enemies_known: bool) {
        let mut rated: Vec<(&Seat, f64)> = Vec::new();
        for seat in side {
            if let Some(power) = seat.power {
                rated.push((seat, power));
            }
        }
        if rated.len() < limits::TIER_MIN_PLAYERS {
            return;
        }
        rated.sort_by(|a, b| b.1.total_cmp(&a.1));
        let median = rated[rated.len() / 2].1;
        let (top, top_power) = rated[0];
        let (bottom, bottom_power) = rated[rated.len() - 1];

        if top_power - median >= limits::TIER_GAP {
            self.set_tier(top, Tier::Top, median);
        }
        if median - bottom_power >= limits::TIER_GAP {
            self.set_tier(bottom, Tier::Bottom, median);
        }

        let ally = side[0].ally;
        if ally && !enemies_known && median - bottom_power >= limits::TIER_GAP {
            let detail = format!(
                "战力 {bottom_power:.0}，队内最低（中位 {median:.0}）。前期多给视野和支援。"
            );
            self.advise(Tone::Warning, &bottom.player.puuid, "需要照顾", detail);
        }
    }

    fn set_tier(&mut self, seat: &Seat, tier: Tier, median: f64) {
        let Some(power) = seat.power else {
            return;
        };
        if let Some(entry) = self.powers.get_mut(&seat.player.puuid) {
            entry.tier = Some(tier);
        }
        let place = match tier {
            Tier::Top => "最高",
            Tier::Bottom => "最低",
        };
        let detail = format!("战力 {power:.0}，队内{place}（队内中位 {median:.0}）。");
        let (id, label, tone) = match (seat.ally, tier) {
            (true, Tier::Top) => ("top-horse", "上等马", Tone::Positive),
            (true, Tier::Bottom) => ("bottom-horse", "下等马", Tone::Warning),
            (false, Tier::Top) => ("hard-nut", "硬骨头", Tone::Warning),
            (false, Tier::Bottom) => ("soft-target", "软柿子", Tone::Positive),
        };
        let t = tag(id, label, tone, detail.clone(), 88);
        self.add_tag(&seat.player.puuid, t);

        if seat.ally {
            return;
        }
        match tier {
            Tier::Bottom => {
                let gank = seat.early.map_or(0.0, |e| e.avg_deaths_to_jungler);
                let mut detail = detail;
                if gank >= limits::EASY_GANK {
                    detail.push_str(&format!("15 分钟前场均被打野参与击杀 {gank:.1} 次。"));
                }
                detail.push_str("打野和中单可以优先针对这一路。");
                self.advise(Tone::Positive, &seat.player.puuid, "敌方软柿子", detail);
            }
            Tier::Top => {
                let detail = format!("{detail}这一路以稳为主，别把资源送进去。");
                self.advise(Tone::Warning, &seat.player.puuid, "敌方硬骨头", detail);
            }
        }
    }

    /// Same-position players on opposite teams, compared by power.
    fn compare_lanes(&mut self, seats: &[Seat]) {
        let mut best: Option<(f64, usize)> = None;
        let mut worst: Option<(f64, usize)> = None;
        for ally in seats.iter().filter(|s| s.ally && !s.position.is_empty()) {
            let mut same = seats
                .iter()
                .filter(|s| !s.ally && s.position == ally.position);
            let (Some(enemy), None) = (same.next(), same.next()) else {
                continue;
            };
            let (Some(a), Some(e)) = (ally.power, enemy.power) else {
                continue;
            };
            let index = self.lanes.len();
            self.lanes.push(LaneMatchup {
                position: ally.position.clone(),
                ally: ally.player.puuid.clone(),
                enemy: enemy.player.puuid.clone(),
                ally_power: a,
                enemy_power: e,
            });

            let edge = a - e;
            let role = position_name(&ally.position);
            let detail = format!("{role}战力 {a:.0} 对 {e:.0}。");
            if edge >= limits::LANE_EDGE {
                let t = tag("lane-edge", "对位优势", Tone::Positive, detail, 70);
                self.add_tag(&ally.player.puuid, t);
            } else if edge <= -limits::LANE_EDGE {
                let t = tag("lane-deficit", "对位劣势", Tone::Warning, detail, 70);
                self.add_tag(&ally.player.puuid, t);
            }
            if best.is_none_or(|(b, _)| edge > b) {
                best = Some((edge, index));
            }
            if worst.is_none_or(|(w, _)| edge < w) {
                worst = Some((edge, index));
            }
        }

        if let Some((edge, i)) = best.filter(|(edge, _)| *edge >= limits::LANE_EDGE) {
            let lane = self.lanes[i].clone();
            let (a, e) = (lane.ally_power, lane.enemy_power);
            let detail = format!("我方 {a:.0} 对 {e:.0}（+{edge:.0}），可围绕这一路扩大优势。");
            let title = format!("优势路 · {}", position_name(&lane.position));
            self.advise(Tone::Positive, &lane.ally, &title, detail);
        }
        if let Some((edge, i)) = worst.filter(|(edge, _)| *edge <= -limits::LANE_EDGE) {
            let lane = self.lanes[i].clone();
            let (a, e) = (lane.ally_power, lane.enemy_power);
            let detail = format!("我方 {a:.0} 对 {e:.0}（{edge:.0}），注意支援和视野。");
            let title = format!("劣势路 · {}", position_name(&lane.position));
            self.advise(Tone::Warning, &lane.ally, &title, detail);
        }
    }

    fn advise(&mut self, tone: Tone, puuid: &str, title: &str, detail: String) {
        self.advice.push(Advice {
            tone,
            puuid: puuid.to_owned(),
            title: title.to_owned(),
            detail,
        });
    }
}

fn power_index(profile: &PlayerProfile, early: Option<&EarlyStats>) -> Option<f64> {
    let form = profile.form.as_ref()?;
    if form.games < limits::POWER_MIN_GAMES {
        return None;
    }
    let (win, performance, lane) = power_parts(form, early);
    Some((50.0 + win + performance + lane).clamp(0.0, 100.0))
}

/// Contributions of ranked win rate, ranked performance and laning to the power index.
fn power_parts(form: &RankedForm, early: Option<&EarlyStats>) -> (f64, f64, f64) {
    let prior = limits::WIN_RATE_PRIOR_GAMES;
    let shrunk = (form.wins + prior / 2.0) / (form.weight + prior);
    let win = (shrunk - 0.5) * limits::WIN_RATE_WEIGHT;

    let cap = limits::PERFORMANCE_CAP;
    let raw = form.performance.unwrap_or(0.0) * limits::PERFORMANCE_WEIGHT;
    let performance = raw.clamp(-cap, cap);

    let mut lane = 0.0;
    if let Some(e) = lane_sample(early) {
        let raw = e.avg_gold_diff_10 / limits::LANE_GOLD_PER_POINT;
        lane = raw.clamp(-limits::LANE_CAP, limits::LANE_CAP);
    }
    (win, performance, lane)
}

fn lane_sample(early: Option<&EarlyStats>) -> Option<&EarlyStats> {
    early.filter(|e| e.lane_games >= limits::LANE_MIN_GAMES)
}

fn breakdown(profile: &PlayerProfile, early: Option<&EarlyStats>, power: f64) -> String {
    let Some(form) = &profile.form else {
        return String::new();
    };
    let (win, performance, lane) = power_parts(form, early);
    let rate = (form.wins / form.weight * 100.0).round();
    let n = form.games;
    let head = format!("战力 {power:.0} = 50 + 胜率 {win:+.0}");
    let mut text = format!("{head}（单双排/灵活 {n} 场，近期加权 {rate}%）");
    if form.performance.is_some() {
        text.push_str(&format!(" + 表现 {performance:+.0}"));
    }
    if let Some(e) = lane_sample(early) {
        let gold = e.avg_gold_diff_10;
        text.push_str(&format!(" + 对线 {lane:+.0}（10 分钟经济差 {gold:+.0}）"));
    }
    if form.off_role_games > 0 {
        let off = form.off_role_games;
        text.push_str(&format!("。{off} 场不在本位置，权重降低"));
    }
    text
}

/// 好抓 / 难抓 from early deaths to the jungler; 对线强 / 对线弱 from gold at 10 minutes.
/// The same fact is good news about an opponent and a warning about a teammate.
fn gank_and_lane_tags(seat: &Seat) -> Vec<PlayerTag> {
    let Some(early) = seat.early else {
        return Vec::new();
    };
    let (good, bad) = if seat.ally {
        (Tone::Positive, Tone::Warning)
    } else {
        (Tone::Warning, Tone::Positive)
    };
    let mut tags = Vec::new();

    if early.gank_games >= limits::GANK_MIN_GAMES {
        let (n, avg) = (early.gank_games, early.avg_deaths_to_jungler);
        let detail = format!("近 {n} 场，前 15 分钟平均被敌方打野参与击杀 {avg:.1} 次。");
        let rule = if avg > limits::VERY_EASY_GANK {
            Some(("very-easy-gank", "非常好抓", bad))
        } else if avg >= limits::EASY_GANK {
            Some(("easy-gank", "好抓", bad))
        } else if avg < limits::HARD_GANK {
            Some(("hard-gank", "难抓", good))
        } else {
            None
        };
        if let Some((id, label, tone)) = rule {
            tags.push(tag(id, label, tone, detail, 78));
        }
    }

    if early.lane_games >= limits::LANE_TAG_MIN_GAMES {
        let n = early.lane_games;
        let gold = early.avg_gold_diff_10;
        let cs = early.avg_cs_diff_10;
        let detail = format!("近 {n} 场，10 分钟时与对位经济差 {gold:+.0}、补刀差 {cs:+.1}。");
        if gold >= limits::STRONG_LANE_GOLD {
            tags.push(tag("strong-lane", "对线强", good, detail, 72));
        } else if gold <= -limits::STRONG_LANE_GOLD {
            tags.push(tag("weak-lane", "对线弱", bad, detail, 72));
        }
    }
    tags
}

fn tag(id: &'static str, label: &str, tone: Tone, detail: String, priority: u8) -> PlayerTag {
    PlayerTag {
        id,
        label: label.to_owned(),
        tone,
        detail,
        low_confidence: false,
        priority,
    }
}

fn position_name(position: &str) -> &'static str {
    match position {
        "TOP" => "上单",
        "JUNGLE" => "打野",
        "MIDDLE" => "中单",
        "BOTTOM" => "下路",
        "UTILITY" => "辅助",
        _ => "",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::match_history::DataSource;
    use crate::services::player_profile::SampleScope;
    use crate::state::ongoing::RosterStage;

    fn profile(wins: usize, games: usize) -> PlayerProfile {
        PlayerProfile {
            puuid: String::new(),
            source: DataSource::Sgp,
            scope: SampleScope::Ranked,
            sample_games: games,
            wins,
            win_rate: wins as f64 / games as f64,
            avg_kills: 5.0,
            avg_deaths: 5.0,
            avg_assists: 5.0,
            avg_kda: 2.0,
            team: None,
            akari_score: None,
            recent: Vec::new(),
            top_champions: Vec::new(),
            position: String::new(),
            tags: Vec::new(),
            form: Some(RankedForm {
                games,
                off_role_games: 0,
                weight: games as f64,
                wins: wins as f64,
                performance: None,
            }),
        }
    }

    fn seat(puuid: &str, position: &str) -> RosterPlayer {
        RosterPlayer {
            puuid: puuid.to_owned(),
            champion_id: 1,
            position: position.to_owned(),
            is_self: false,
        }
    }

    const LANES: [&str; 5] = ["TOP", "JUNGLE", "MIDDLE", "BOTTOM", "UTILITY"];

    fn game() -> (Roster, HashMap<String, PlayerProfile>) {
        let allies = LANES.map(|l| seat(&format!("a-{l}"), l)).to_vec();
        let enemies = LANES.map(|l| seat(&format!("e-{l}"), l)).to_vec();
        let mut profiles = HashMap::new();
        for l in LANES {
            profiles.insert(format!("a-{l}"), profile(10, 20));
            profiles.insert(format!("e-{l}"), profile(10, 20));
        }
        profiles.insert("a-MIDDLE".to_owned(), profile(17, 20));
        profiles.insert("e-BOTTOM".to_owned(), profile(4, 20));
        let roster = Roster {
            stage: RosterStage::InGame,
            game_id: 1,
            queue_id: 420,
            allies,
            anonymous_allies: Vec::new(),
            enemies,
            hidden_enemies: 0,
            enemy_champions: Vec::new(),
        };
        (roster, profiles)
    }

    fn labels(m: &Matchup, puuid: &str) -> Vec<String> {
        let tags = m.tags.get(puuid).map(Vec::as_slice).unwrap_or_default();
        tags.iter().map(|t| t.label.clone()).collect()
    }

    fn tone_of(m: &Matchup, puuid: &str, id: &str) -> Tone {
        let tags = &m.tags[puuid];
        tags.iter().find(|t| t.id == id).unwrap().tone
    }

    #[test]
    fn ranks_horses_and_compares_lanes() {
        let (roster, profiles) = game();
        let m = analyze(&roster, &profiles, &HashMap::new());

        assert_eq!(m.powers["a-MIDDLE"].tier, Some(Tier::Top));
        assert!(labels(&m, "a-MIDDLE").contains(&"上等马".to_owned()));
        assert!(labels(&m, "a-MIDDLE").contains(&"对位优势".to_owned()));
        assert!(labels(&m, "e-BOTTOM").contains(&"软柿子".to_owned()));
        assert!(labels(&m, "a-BOTTOM").contains(&"对位优势".to_owned()));
        assert_eq!(m.lanes.len(), 5);

        let titles: Vec<&str> = m.advice.iter().map(|a| a.title.as_str()).collect();
        assert!(titles.contains(&"敌方软柿子"));
        assert!(titles.iter().any(|t| t.starts_with("优势路")));
    }

    #[test]
    fn small_samples_get_no_power() {
        let (roster, mut profiles) = game();
        profiles.insert("e-TOP".to_owned(), profile(3, 3));
        let m = analyze(&roster, &profiles, &HashMap::new());
        assert!(!m.powers.contains_key("e-TOP"));
    }

    #[test]
    fn gank_tags_read_differently_per_side() {
        let (roster, profiles) = game();
        let easy = EarlyStats {
            gank_games: 6,
            avg_deaths_to_jungler: 1.8,
            ..EarlyStats::default()
        };
        let early = HashMap::from([
            ("a-TOP".to_owned(), easy.clone()),
            ("e-TOP".to_owned(), easy),
        ]);
        let m = analyze(&roster, &profiles, &early);
        assert_eq!(tone_of(&m, "a-TOP", "easy-gank"), Tone::Warning);
        assert_eq!(tone_of(&m, "e-TOP", "easy-gank"), Tone::Positive);
    }
}
