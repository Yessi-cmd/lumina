//! Per-player analysis for the game panel: win/loss, averages, Akari Score and tags.
//!
//! Ported from League Akari's player-card tags and the habit analysis added in
//! Akari-Yessi (champion practice, suspicious boosting, flash key), then extended:
//! stats are computed on games from the current queue when there are enough of them,
//! damage and farming are judged against a per-position baseline, and every tag carries
//! its evidence and sample size.

use std::collections::HashMap;

use serde::Serialize;

use super::match_history::{
    DataSource, GameMetrics, GameResult, GameSummary, MatchHistoryPage, MatchHistoryService,
};
use super::ongoing_game::PANEL_HISTORY_COUNT;
use crate::error::Result;
use crate::state::session::LcuSession;

/// Every threshold in one place; League Akari's defaults unless noted.
mod limits {
    /// Fewer games than this in the current queue falls back to all queues.
    pub const QUEUE_SAMPLE_MIN: usize = 5;
    /// Tags backed by fewer games are marked low confidence.
    pub const CONFIDENT_SAMPLE: usize = 8;
    pub const STREAK_MIN: usize = 3;
    pub const HIGH_WIN_RATE_MIN_GAMES: usize = 16;
    pub const HIGH_WIN_RATE: f64 = 0.85;
    /// Akari Score for "优秀" and "非凡".
    pub const GOOD_SCORE: f64 = 6.5;
    pub const GOOD_MIN_GAMES: usize = 5;
    pub const ELITE_SCORE: f64 = 8.0;
    pub const ELITE_MIN_GAMES: usize = 8;
    pub const PRACTICE_MIN_SAMPLE: usize = 8;
    pub const PRACTICE_MAX_GAMES: usize = 2;
    /// Lumina: mastery points below this back up 练英雄, at or above the next one the
    /// player is an old hand on the champion who just has not played it lately.
    pub const PRACTICE_MAX_POINTS: i64 = 20_000;
    pub const VETERAN_POINTS: i64 = 100_000;
    /// Lumina: a champion used in this share of recent Rift games is a signature pick.
    pub const SIGNATURE_MIN_GAMES: usize = 5;
    pub const SIGNATURE_SHARE: f64 = 0.4;
    /// Lumina: League Akari shows any non-zero average; one per game is notable.
    pub const SOLO_KILLS_MIN: f64 = 1.0;
    pub const KILL_DAMAGE_HIGH: f64 = 1.35;
    pub const KILL_DAMAGE_LOW: f64 = 0.65;
    pub const MISSING_PINGS_MIN: f64 = 3.0;
    /// Lumina: flash on the less used key this often before it counts as a habit.
    pub const FLASH_MINORITY_MIN: usize = 2;
    /// Lumina: damage share relative to the position baseline.
    pub const HIGH_DAMAGE_FACTOR: f64 = 1.3;
    pub const LOW_DAMAGE_FACTOR: f64 = 0.65;
    pub const GOOD_CS_PER_MINUTE: f64 = 8.0;
    pub const POOR_CS_PER_MINUTE: f64 = 5.0;
    pub const METRIC_MIN_GAMES: usize = 3;
    /// Ranked form: a game this many ranked games back weighs half as much as the latest.
    pub const FORM_HALF_LIFE: f64 = 6.0;
    /// Ranked form: weight of a game played off the player's current position.
    pub const OFF_ROLE_WEIGHT: f64 = 0.3;
    /// Deaths per 10 minutes of an average ranked player.
    pub const DEATHS_PER_10: f64 = 2.0;
}

const RANKED_QUEUES: [i64; 2] = [420, 440];
/// Summoner's Rift queues, where champion choice and farming are deliberate.
const CLASSIC_QUEUES: [i64; 6] = [400, 420, 430, 440, 490, 700];
const ENTERTAINMENT_QUEUES: [i64; 13] = [
    450, 900, 1020, 1300, 1400, 1700, 1710, 1750, 1810, 1820, 1830, 1840, 1900,
];
const ENTERTAINMENT_MODES: [&str; 7] = [
    "ARAM",
    "CHERRY",
    "ONEFORALL",
    "NEXUSBLITZ",
    "STRAWBERRY",
    "ULTBOOK",
    "URF",
];
const FLASH: i64 = 4;
const AKARI_NOTE: &str = "综合 KDA、胜率、伤害、承伤、治疗、补刀、经济、参团与视野。";
const PRACTICE_NOTE: &str = "拿不到英雄成就点，分不清是在练还是以前常玩、最近没碰。";
const BOOSTING_NOTE: &str = "这只是基于近期表现和操作习惯的提示，不代表换人或代练的事实。";

/// What the player is doing right now, as far as the roster knows.
#[derive(Debug, Clone, Default)]
pub struct ProfileContext {
    /// 0 when not picked yet.
    pub champion_id: i64,
    /// 0 when unknown.
    pub queue_id: i64,
    pub position: String,
    /// Mastery points on `champion_id`; `None` when unknown.
    pub champion_points: Option<i64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum SampleScope {
    Ranked,
    SameQueue,
    All,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Tone {
    Positive,
    Negative,
    Warning,
    Neutral,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlayerTag {
    pub id: &'static str,
    pub label: String,
    pub tone: Tone,
    /// Evidence shown on hover.
    pub detail: String,
    pub low_confidence: bool,
    /// Higher shows first; the card only has room for a few.
    pub priority: u8,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlayerProfile {
    pub puuid: String,
    pub source: DataSource,
    pub scope: SampleScope,
    /// Games in the sample that count towards win rate (no remakes).
    pub sample_games: usize,
    pub wins: usize,
    pub win_rate: f64,
    pub avg_kills: f64,
    pub avg_deaths: f64,
    pub avg_assists: f64,
    pub avg_kda: f64,
    /// Team-relative averages; `None` when the history came from LCU.
    pub team: Option<TeamAverages>,
    pub akari_score: Option<AkariScore>,
    /// Most recent first, across all queues.
    pub recent: Vec<GameResult>,
    pub top_champions: Vec<ChampionStat>,
    /// Position used for baselines: the current assignment, else the most played one.
    pub position: String,
    pub tags: Vec<PlayerTag>,
    /// Input to the power index; `None` without solo/duo or flex games.
    #[serde(skip)]
    pub form: Option<RankedForm>,
}

/// Solo/duo and flex games only, recent games and games on the current position
/// weighted most. Normal, ARAM and bot games say little about how someone plays ranked.
#[derive(Debug, Clone)]
pub struct RankedForm {
    pub games: usize,
    pub off_role_games: usize,
    /// Sum of game weights, and of the weights of the games won.
    pub weight: f64,
    pub wins: f64,
    /// -1..1 against the average player on the same position; `None` without SGP data.
    pub performance: Option<f64>,
    /// Games whose position is known, how many were on the most played position, and how
    /// many on `role`, the position in this game (empty when unknown).
    pub positioned_games: usize,
    pub main_position: String,
    pub main_games: usize,
    pub role: String,
    pub role_games: usize,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TeamAverages {
    pub games: usize,
    pub damage_share: f64,
    pub damage_taken_share: f64,
    pub gold_share: f64,
    pub kill_participation: f64,
    pub cs_per_minute: f64,
    pub vision_per_minute: f64,
    /// Champion damage per gold earned.
    pub damage_per_gold: f64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AkariScore {
    pub total: f64,
    pub max: f64,
    pub games: usize,
    pub outstanding: bool,
    pub extraordinary: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ChampionStat {
    pub champion_id: i64,
    pub games: usize,
    pub wins: usize,
}

/// Profile from the same cached page the game panel prefetches.
pub async fn load(
    history: &MatchHistoryService,
    session: &LcuSession,
    puuid: &str,
    ctx: &ProfileContext,
) -> Result<PlayerProfile> {
    let page = history.get(session, puuid, 0, PANEL_HISTORY_COUNT).await?;
    Ok(build(&page, ctx))
}

pub fn build(page: &MatchHistoryPage, ctx: &ProfileContext) -> PlayerProfile {
    let all: Vec<&GameSummary> = page.games.iter().collect();
    let counted: Vec<&GameSummary> = all.iter().copied().filter(|g| counts(g)).collect();
    let (scope, sample) = pick_sample(&counted, ctx.queue_id);

    let wins = sample
        .iter()
        .filter(|g| g.result == GameResult::Win)
        .count();
    let win_rate = ratio(wins, sample.len());
    let avg_kda = avg_kda_of(&sample);
    let with_metrics = metric_games(&sample);
    let position = if ctx.position.is_empty() {
        main_position(&sample)
    } else {
        ctx.position.clone()
    };
    let team = team_averages(&with_metrics);
    let akari_score = akari_score(&with_metrics, win_rate, avg_kda);

    let facts = Facts {
        ctx,
        all: &all,
        counted: &counted,
        sample: &sample,
        scope,
        win_rate,
        position: &position,
        team: team.as_ref(),
        akari_score: akari_score.as_ref(),
    };
    let mut tags = facts.tags();
    tags.sort_by_key(|t| std::cmp::Reverse(t.priority));

    PlayerProfile {
        puuid: page.puuid.clone(),
        source: page.source,
        scope,
        sample_games: sample.len(),
        wins,
        win_rate,
        avg_kills: mean(sample.iter().map(|g| g.kills as f64)),
        avg_deaths: mean(sample.iter().map(|g| g.deaths as f64)),
        avg_assists: mean(sample.iter().map(|g| g.assists as f64)),
        avg_kda,
        team,
        akari_score,
        recent: all.iter().take(10).map(|g| g.result).collect(),
        top_champions: top_champions(&counted),
        position,
        tags,
        form: ranked_form(&counted, &ctx.position),
    }
}

/// Inputs shared by the tag rules.
struct Facts<'a> {
    ctx: &'a ProfileContext,
    /// Every game, newest first.
    all: &'a [&'a GameSummary],
    /// Wins and losses only, newest first.
    counted: &'a [&'a GameSummary],
    /// `counted` narrowed to the current queue when possible.
    sample: &'a [&'a GameSummary],
    scope: SampleScope,
    win_rate: f64,
    position: &'a str,
    team: Option<&'a TeamAverages>,
    akari_score: Option<&'a AkariScore>,
}

impl Facts<'_> {
    fn tags(&self) -> Vec<PlayerTag> {
        let rules = [
            self.suspicious_boosting(),
            self.streak(),
            self.akari_score_tag(),
            self.high_win_rate(),
            self.champion_familiarity(),
            self.flash_key(),
            self.solo_kills(),
            self.damage_output(),
            self.farming(),
            self.kill_damage_efficiency(),
            self.missing_pings(),
        ];
        rules.into_iter().flatten().collect()
    }

    fn scope_text(&self) -> String {
        let n = self.sample.len();
        match self.scope {
            SampleScope::Ranked => format!("最近 {n} 场排位"),
            SampleScope::SameQueue => format!("最近 {n} 场同模式对局"),
            SampleScope::All => format!("最近 {n} 场对局"),
        }
    }

    /// Position baselines and farming only make sense on Summoner's Rift.
    fn on_rift(&self) -> bool {
        let queue = self.ctx.queue_id;
        queue == 0 || CLASSIC_QUEUES.contains(&queue)
    }

    fn streak(&self) -> Option<PlayerTag> {
        let first = self.counted.first()?.result;
        let same = self.counted.iter().take_while(|g| g.result == first);
        let len = same.count();
        if len < limits::STREAK_MIN {
            return None;
        }
        let (label, tone, word) = match first {
            GameResult::Win => (format!("{len} 连胜"), Tone::Positive, "连胜"),
            _ => (format!("{len} 连败"), Tone::Negative, "连败"),
        };
        Some(tag(
            "streak",
            label,
            tone,
            format!("截至最近一局，已经 {len} {word}（不计重开）。"),
            false,
            80,
        ))
    }

    fn high_win_rate(&self) -> Option<PlayerTag> {
        let n = self.sample.len();
        if n < limits::HIGH_WIN_RATE_MIN_GAMES || self.win_rate < limits::HIGH_WIN_RATE {
            return None;
        }
        let scope = self.scope_text();
        let rate = pct(self.win_rate);
        Some(tag(
            "high-win-rate",
            "高胜率".to_owned(),
            Tone::Positive,
            format!("{scope}胜率 {rate}%。"),
            false,
            70,
        ))
    }

    fn akari_score_tag(&self) -> Option<PlayerTag> {
        let score = self.akari_score?;
        let label = if score.extraordinary {
            "非凡"
        } else if score.outstanding {
            "优秀"
        } else {
            return None;
        };
        let (total, max, games) = (score.total, score.max, score.games);
        Some(tag(
            "akari-score",
            label.to_owned(),
            Tone::Positive,
            format!("Akari Score {total:.1} / {max:.0}，基于 {games} 场完整数据。{AKARI_NOTE}"),
            false,
            75,
        ))
    }

    /// Akari-Yessi's "练英雄", plus the opposite case: a one-trick on their main.
    fn champion_familiarity(&self) -> Option<PlayerTag> {
        let champion = self.ctx.champion_id;
        if champion <= 0 || !self.on_rift() {
            return None;
        }
        let (mut n, mut k, mut wins) = (0, 0, 0);
        for game in self.counted {
            if !CLASSIC_QUEUES.contains(&game.queue_id) {
                continue;
            }
            n += 1;
            if game.champion_id == champion {
                k += 1;
                if game.result == GameResult::Win {
                    wins += 1;
                }
            }
        }

        if n >= limits::PRACTICE_MIN_SAMPLE && k <= limits::PRACTICE_MAX_GAMES {
            return self.rarely_played(n, k);
        }
        if k >= limits::SIGNATURE_MIN_GAMES && ratio(k, n) >= limits::SIGNATURE_SHARE {
            let losses = k - wins;
            return Some(tag(
                "signature-champion",
                "绝活".to_owned(),
                Tone::Positive,
                format!("最近 {n} 场召唤师峡谷对局中 {k} 场用当前英雄，{wins} 胜 {losses} 负。"),
                false,
                65,
            ));
        }
        None
    }

    /// Few recent games on the champion: still practising, or an old hand back after a
    /// break? Mastery points decide; without them the tag only reports the recent games.
    fn rarely_played(&self, n: usize, k: usize) -> Option<PlayerTag> {
        let recent = format!("最近 {n} 场召唤师峡谷对局中，当前英雄只用过 {k} 场");
        let Some(points) = self.ctx.champion_points else {
            let detail = format!("{recent}。{PRACTICE_NOTE}");
            let label = "近期少玩".to_owned();
            let rare = tag("champion-rare", label, Tone::Warning, detail, true, 55);
            return Some(rare);
        };
        if points >= limits::VETERAN_POINTS {
            let label = format!("熟练 {}万", points / 10_000);
            let detail = format!("{recent}，但英雄成就点 {points}，以前常玩，只是最近少碰。");
            let veteran = tag("champion-veteran", label, Tone::Positive, detail, false, 60);
            return Some(veteran);
        }
        if points < limits::PRACTICE_MAX_POINTS {
            let detail = format!("{recent}，英雄成就点只有 {points}，很可能还在练。");
            let label = "练英雄".to_owned();
            let practice = tag("champion-practice", label, Tone::Warning, detail, false, 65);
            return Some(practice);
        }
        None
    }

    fn flash_key(&self) -> Option<PlayerTag> {
        let (d, f) = flash_keys(self.all);
        if d.min(f) < limits::FLASH_MINORITY_MIN {
            return None;
        }
        Some(tag(
            "flash-key",
            "闪现异位".to_owned(),
            Tone::Warning,
            format!("最近的对局里闪现放在 D 键 {d} 次、F 键 {f} 次，按键习惯不固定。"),
            d + f < limits::CONFIDENT_SAMPLE,
            55,
        ))
    }

    /// Akari-Yessi's "近期异常": a run of ranked games far above the player's usual level.
    fn suspicious_boosting(&self) -> Option<PlayerTag> {
        if !RANKED_QUEUES.contains(&self.ctx.queue_id) {
            return None;
        }
        let (d, f) = flash_keys(self.all);
        let signal = boosting_signal(self.all, d > 0 && f > 0)?;

        let n = signal.ranked_games;
        let wr = pct(signal.ranked_win_rate);
        let kda = signal.ranked_avg_kda;
        let mut detail = format!("最近连续 {n} 场排位胜率 {wr}%，平均 KDA {kda:.1}。");
        if signal.performance_spike {
            detail.push_str("近期表现达到异常高位，或相较更早的排位明显跃升。");
        }
        if signal.flash_inconsistent {
            detail.push_str("同一批样本中闪现同时出现在 D、F 两个键位。");
        }
        if signal.preceding_entertainment > 0 {
            let count = signal.preceding_entertainment;
            detail.push_str(&format!(
                "更早的样本中有 {count} 场娱乐模式（仅作辅助信号）。"
            ));
        }
        detail.push_str(BOOSTING_NOTE);
        Some(tag(
            "suspicious-boosting",
            "近期异常".to_owned(),
            Tone::Warning,
            detail,
            false,
            90,
        ))
    }

    fn solo_kills(&self) -> Option<PlayerTag> {
        let games = metric_games(self.sample);
        if games.len() < limits::METRIC_MIN_GAMES {
            return None;
        }
        let avg = mean(games.iter().map(|(_, m)| m.solo_kills));
        if avg < limits::SOLO_KILLS_MIN {
            return None;
        }
        let scope = self.scope_text();
        Some(tag(
            "solo-kills",
            format!("单杀 {avg:.1}"),
            Tone::Positive,
            format!("{scope}场均单杀 {avg:.2} 次。"),
            games.len() < limits::CONFIDENT_SAMPLE,
            50,
        ))
    }

    /// Damage share against what the position usually deals, so supports are not "low damage".
    fn damage_output(&self) -> Option<PlayerTag> {
        let team = self.team?;
        if team.games < limits::METRIC_MIN_GAMES || !self.on_rift() {
            return None;
        }
        let baseline = damage_share_baseline(self.position);
        let share = team.damage_share;
        let (id, label, tone) = if share >= baseline * limits::HIGH_DAMAGE_FACTOR {
            ("high-damage", "高输出", Tone::Positive)
        } else if share <= baseline * limits::LOW_DAMAGE_FACTOR {
            ("low-damage", "低输出", Tone::Negative)
        } else {
            return None;
        };
        let scope = self.scope_text();
        let role = position_name(self.position);
        let (share, base) = (pct(share), pct(baseline));
        Some(tag(
            id,
            label.to_owned(),
            tone,
            format!("{scope}平均伤害占比 {share}%，{role}常见水平约 {base}%。"),
            team.games < limits::CONFIDENT_SAMPLE,
            45,
        ))
    }

    fn farming(&self) -> Option<PlayerTag> {
        let team = self.team?;
        let lane = matches!(self.position, "TOP" | "MIDDLE" | "BOTTOM");
        if !lane || !self.on_rift() || team.games < limits::QUEUE_SAMPLE_MIN {
            return None;
        }
        let cs = team.cs_per_minute;
        let (id, label, tone) = if cs >= limits::GOOD_CS_PER_MINUTE {
            ("good-farming", "补刀稳", Tone::Positive)
        } else if cs <= limits::POOR_CS_PER_MINUTE {
            ("poor-farming", "补刀差", Tone::Negative)
        } else {
            return None;
        };
        let scope = self.scope_text();
        Some(tag(
            id,
            label.to_owned(),
            tone,
            format!("{scope}分均补刀 {cs:.1}。"),
            team.games < limits::CONFIDENT_SAMPLE,
            40,
        ))
    }

    /// League Akari's "K头 / 打工": share of team kills versus share of team damage.
    fn kill_damage_efficiency(&self) -> Option<PlayerTag> {
        let games = metric_games(self.sample);
        if games.len() < limits::METRIC_MIN_GAMES {
            return None;
        }
        let avg = mean(games.iter().map(|(_, m)| kill_damage_ratio(m)));
        let (id, label, detail) = if avg > limits::KILL_DAMAGE_HIGH {
            ("kill-stealer", "K头", "用较少的伤害拿到了较多击杀")
        } else if avg < limits::KILL_DAMAGE_LOW {
            ("damage-worker", "打工", "打出了较多伤害但击杀较少")
        } else {
            return None;
        };
        let scope = self.scope_text();
        Some(tag(
            id,
            label.to_owned(),
            Tone::Neutral,
            format!("{scope}击杀占比 ÷ 伤害占比平均为 {avg:.2}，{detail}。"),
            games.len() < limits::CONFIDENT_SAMPLE,
            35,
        ))
    }

    fn missing_pings(&self) -> Option<PlayerTag> {
        let games = metric_games(self.sample);
        if games.len() < limits::METRIC_MIN_GAMES {
            return None;
        }
        let avg = mean(games.iter().map(|(_, m)| m.enemy_missing_pings as f64));
        if avg < limits::MISSING_PINGS_MIN {
            return None;
        }
        let scope = self.scope_text();
        Some(tag(
            "missing-pings",
            format!("问号 {avg:.1}"),
            Tone::Warning,
            format!("{scope}平均每局发送敌方消失信号（问号）{avg:.1} 次。"),
            games.len() < limits::CONFIDENT_SAMPLE,
            30,
        ))
    }
}

fn tag(
    id: &'static str,
    label: String,
    tone: Tone,
    detail: String,
    low_confidence: bool,
    priority: u8,
) -> PlayerTag {
    PlayerTag {
        id,
        label,
        tone,
        detail,
        low_confidence,
        priority,
    }
}

fn counts(game: &GameSummary) -> bool {
    matches!(game.result, GameResult::Win | GameResult::Loss)
}

fn same_queue_family(a: i64, b: i64) -> bool {
    a == b || (RANKED_QUEUES.contains(&a) && RANKED_QUEUES.contains(&b))
}

fn pick_sample<'a>(
    counted: &[&'a GameSummary],
    queue_id: i64,
) -> (SampleScope, Vec<&'a GameSummary>) {
    if queue_id > 0 {
        let mut same = Vec::new();
        for game in counted {
            if same_queue_family(game.queue_id, queue_id) {
                same.push(*game);
            }
        }
        if same.len() >= limits::QUEUE_SAMPLE_MIN {
            let scope = if RANKED_QUEUES.contains(&queue_id) {
                SampleScope::Ranked
            } else {
                SampleScope::SameQueue
            };
            return (scope, same);
        }
    }
    (SampleScope::All, counted.to_vec())
}

fn metric_games<'a>(games: &[&'a GameSummary]) -> Vec<(&'a GameSummary, &'a GameMetrics)> {
    let mut out = Vec::new();
    for game in games {
        if let Some(metrics) = &game.metrics {
            out.push((*game, metrics));
        }
    }
    out
}

fn team_averages(games: &[(&GameSummary, &GameMetrics)]) -> Option<TeamAverages> {
    if games.is_empty() {
        return None;
    }
    let avg = |f: fn(&GameMetrics) -> f64| mean(games.iter().map(|(_, m)| f(m)));
    let per_game = |f: fn(&GameSummary) -> f64| mean(games.iter().map(|(g, _)| f(g)));
    Some(TeamAverages {
        games: games.len(),
        damage_share: avg(|m| m.damage_share),
        damage_taken_share: avg(|m| m.damage_taken_share),
        gold_share: avg(|m| m.gold_share),
        kill_participation: avg(|m| m.kill_participation),
        cs_per_minute: per_game(|g| g.cs as f64 / minutes(g)),
        vision_per_minute: per_game(|g| g.vision_score as f64 / minutes(g)),
        damage_per_gold: per_game(|g| g.damage_to_champions as f64 / g.gold.max(1) as f64),
    })
}

fn akari_score(
    games: &[(&GameSummary, &GameMetrics)],
    win_rate: f64,
    avg_kda: f64,
) -> Option<AkariScore> {
    if games.len() < limits::METRIC_MIN_GAMES {
        return None;
    }
    let per_game = mean(games.iter().map(|(g, m)| akari::per_game(g, m)));
    let total = akari::kda(avg_kda) + akari::win_rate(win_rate) + per_game;
    let n = games.len();
    let outstanding = total >= limits::GOOD_SCORE && n >= limits::GOOD_MIN_GAMES;
    let extraordinary = total >= limits::ELITE_SCORE && n >= limits::ELITE_MIN_GAMES;
    Some(AkariScore {
        total,
        max: akari::MAX,
        games: n,
        outstanding,
        extraordinary,
    })
}

/// `games` newest first; games that are not ranked or were remade are skipped.
/// `position` is the one in the current game, empty when unknown.
pub fn ranked_form(games: &[&GameSummary], position: &str) -> Option<RankedForm> {
    let mut ranked = Vec::new();
    for game in games {
        if counts(game) && is_ranked(game) {
            ranked.push(*game);
        }
    }
    if ranked.is_empty() {
        return None;
    }
    let main = main_position(&ranked);
    let on = |p: &str| ranked.iter().filter(|g| g.position == p).count();
    let mut form = RankedForm {
        games: ranked.len(),
        off_role_games: 0,
        weight: 0.0,
        wins: 0.0,
        performance: None,
        positioned_games: ranked.len() - on(""),
        main_games: on(&main),
        role_games: on(position),
        main_position: main,
        role: position.to_owned(),
    };
    // Off-role means away from this game's position, else from the usual one.
    let role = match position {
        "" => form.main_position.clone(),
        _ => position.to_owned(),
    };
    let (mut rated, mut performance) = (0.0, 0.0);
    for (i, game) in ranked.iter().enumerate() {
        let mut weight = 0.5_f64.powf(i as f64 / limits::FORM_HALF_LIFE);
        let off_role = !role.is_empty() && !game.position.is_empty() && game.position != role;
        if off_role {
            form.off_role_games += 1;
            weight *= limits::OFF_ROLE_WEIGHT;
        }
        form.weight += weight;
        if game.result == GameResult::Win {
            form.wins += weight;
        }
        if let Some(m) = &game.metrics {
            rated += weight;
            performance += weight * game_performance(game, m);
        }
    }
    if rated > 0.0 {
        form.performance = Some(performance / rated);
    }
    Some(form)
}

/// One game against the average player on the same position. Each part stays within ±1,
/// so one stat-padded game cannot carry the average, and kills and assists count through
/// kill participation rather than KDA.
fn game_performance(game: &GameSummary, m: &GameMetrics) -> f64 {
    let unit = |v: f64| v.clamp(-1.0, 1.0);
    let deaths_per_10 = game.deaths as f64 * 10.0 / minutes(game);
    let deaths = unit((limits::DEATHS_PER_10 - deaths_per_10) / 1.2);
    let participation = unit((m.kill_participation - 0.5) / 0.25);
    let damage_ratio = m.damage_share / damage_share_baseline(&game.position);
    let damage = unit((damage_ratio - 1.0) / 0.5);
    let gold_ratio = m.gold_share / gold_share_baseline(&game.position);
    let gold = unit((gold_ratio - 1.0) / 0.25);
    (deaths + participation + damage + gold) / 4.0
}

/// League Akari's scoring model (`analysis/player/scoring.ts`), weights unchanged.
mod akari {
    use super::{minutes, GameMetrics, GameSummary};

    pub const MAX: f64 = 17.0;

    fn linear(value: f64, min: f64, max: f64, score: f64) -> f64 {
        (value.clamp(min, max) - min) / (max - min) * score
    }

    pub fn kda(kda: f64) -> f64 {
        ((kda - 2.0).max(0.0).sqrt() * 3.0 / 7.0).clamp(0.0, 1.0)
    }

    pub fn win_rate(win_rate: f64) -> f64 {
        linear(win_rate, 0.5, 1.0, 1.0)
    }

    /// Damage, damage taken, healing, farming, gold, participation and vision.
    pub fn per_game(game: &GameSummary, m: &GameMetrics) -> f64 {
        let n = m.team_size as f64;
        let expected = |share: f64| if m.team_size > 1 { share * n } else { 0.0 };
        let healing_full = if m.team_size == 1 { 1.0 } else { 1.4 };
        let cs_per_minute = game.cs as f64 / minutes(game);

        linear(expected(m.damage_share), 1.0, 2.0, 3.0)
            + linear(expected(m.damage_taken_share), 1.0, 2.0, 2.0)
            + linear(m.heal_ratio, 0.2, healing_full, 2.0)
            + linear(cs_per_minute, 5.0, 10.0, 2.0)
            + linear(expected(m.gold_share), 1.0, 1.5, 2.0)
            + linear(m.kill_participation, 0.3, 1.0, 2.0)
            + linear(expected(m.vision_share), 1.0, 2.0, 2.0)
    }
}

struct BoostingSignal {
    ranked_games: usize,
    ranked_win_rate: f64,
    ranked_avg_kda: f64,
    performance_spike: bool,
    flash_inconsistent: bool,
    preceding_entertainment: usize,
}

/// Port of `getSuspiciousBoostingSignalFromGames` from Akari-Yessi, thresholds unchanged.
fn boosting_signal(games: &[&GameSummary], flash_inconsistent: bool) -> Option<BoostingSignal> {
    let recent: Vec<&GameSummary> = games
        .iter()
        .take(5)
        .copied()
        .take_while(|g| is_ranked(g))
        .collect();
    if recent.len() < 3 {
        return None;
    }

    let older = &games[recent.len()..games.len().min(recent.len() + 8)];
    let preceding_entertainment = older.iter().filter(|g| is_entertainment(g)).count();
    let older_ranked: Vec<&GameSummary> = older.iter().copied().filter(|g| is_ranked(g)).collect();

    let ranked_win_rate = win_rate_of(&recent);
    let ranked_avg_kda = avg_kda_of(&recent);
    let absolute = ranked_win_rate >= 0.8 && ranked_avg_kda >= 5.0;
    let relative = older_ranked.len() >= 3
        && ranked_avg_kda >= 4.0
        && ranked_avg_kda >= avg_kda_of(&older_ranked) * 1.8
        && ranked_win_rate >= win_rate_of(&older_ranked) + 0.3;
    let performance_spike = absolute || relative;
    let backed_by_flash =
        flash_inconsistent && ranked_win_rate >= 2.0 / 3.0 && ranked_avg_kda >= 4.5;
    if !performance_spike && !backed_by_flash {
        return None;
    }

    Some(BoostingSignal {
        ranked_games: recent.len(),
        ranked_win_rate,
        ranked_avg_kda,
        performance_spike,
        flash_inconsistent,
        preceding_entertainment,
    })
}

fn is_ranked(game: &GameSummary) -> bool {
    RANKED_QUEUES.contains(&game.queue_id)
}

fn is_entertainment(game: &GameSummary) -> bool {
    ENTERTAINMENT_QUEUES.contains(&game.queue_id)
        || ENTERTAINMENT_MODES.contains(&game.game_mode.as_str())
}

fn win_rate_of(games: &[&GameSummary]) -> f64 {
    let wins = games.iter().filter(|g| g.result == GameResult::Win).count();
    ratio(wins, games.len())
}

fn avg_kda_of(games: &[&GameSummary]) -> f64 {
    mean(games.iter().map(|g| kda(g)))
}

fn flash_keys(games: &[&GameSummary]) -> (usize, usize) {
    let d = games.iter().filter(|g| g.spells[0] == FLASH).count();
    let f = games.iter().filter(|g| g.spells[1] == FLASH).count();
    (d, f)
}

fn kill_damage_ratio(m: &GameMetrics) -> f64 {
    if m.damage_share <= 0.0 {
        1.0
    } else {
        m.kill_share / m.damage_share
    }
}

fn main_position(games: &[&GameSummary]) -> String {
    let mut counts: HashMap<&str, usize> = HashMap::new();
    for game in games {
        if !game.position.is_empty() {
            *counts.entry(game.position.as_str()).or_default() += 1;
        }
    }
    let best = counts.into_iter().max_by_key(|(_, n)| *n);
    best.map(|(p, _)| p.to_owned()).unwrap_or_default()
}

/// Typical share of team champion damage per position in solo queue.
fn damage_share_baseline(position: &str) -> f64 {
    match position {
        "UTILITY" => 0.10,
        "JUNGLE" => 0.17,
        "TOP" => 0.22,
        "MIDDLE" | "BOTTOM" => 0.26,
        _ => 0.20,
    }
}

/// Typical share of team gold per position in solo queue.
fn gold_share_baseline(position: &str) -> f64 {
    match position {
        "UTILITY" => 0.13,
        "JUNGLE" => 0.19,
        "TOP" => 0.21,
        "MIDDLE" => 0.22,
        "BOTTOM" => 0.24,
        _ => 0.20,
    }
}

fn position_name(position: &str) -> &'static str {
    match position {
        "TOP" => "上单",
        "JUNGLE" => "打野",
        "MIDDLE" => "中单",
        "BOTTOM" => "下路",
        "UTILITY" => "辅助",
        _ => "全位置",
    }
}

fn top_champions(counted: &[&GameSummary]) -> Vec<ChampionStat> {
    let mut by_champion: HashMap<i64, (usize, usize)> = HashMap::new();
    for game in counted {
        let entry = by_champion.entry(game.champion_id).or_insert((0, 0));
        entry.0 += 1;
        if game.result == GameResult::Win {
            entry.1 += 1;
        }
    }
    let mut stats = Vec::new();
    for (champion_id, (games, wins)) in by_champion {
        stats.push(ChampionStat {
            champion_id,
            games,
            wins,
        });
    }
    stats.sort_by(|a, b| b.games.cmp(&a.games).then(b.wins.cmp(&a.wins)));
    stats.truncate(3);
    stats
}

fn minutes(game: &GameSummary) -> f64 {
    (game.duration as f64 / 60.0).max(1.0)
}

fn kda(game: &GameSummary) -> f64 {
    (game.kills + game.assists) as f64 / game.deaths.max(1) as f64
}

fn mean(values: impl Iterator<Item = f64>) -> f64 {
    let (sum, n) = values.fold((0.0, 0usize), |(sum, n), v| (sum + v, n + 1));
    if n == 0 {
        0.0
    } else {
        sum / n as f64
    }
}

fn ratio(part: usize, total: usize) -> f64 {
    if total == 0 {
        0.0
    } else {
        part as f64 / total as f64
    }
}

fn pct(value: f64) -> String {
    format!("{:.0}", value * 100.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn game(queue_id: i64, result: GameResult, k: i64, d: i64, a: i64) -> GameSummary {
        GameSummary {
            game_id: 0,
            queue_id,
            game_mode: "CLASSIC".to_owned(),
            created_at: 0,
            duration: 1800,
            result,
            champion_id: 1,
            champ_level: 18,
            kills: k,
            deaths: d,
            assists: a,
            spells: [4, 14],
            items: [0; 7],
            cs: 240,
            gold: 12000,
            damage_to_champions: 20000,
            position: "MIDDLE".to_owned(),
            team_id: 100,
            vision_score: 20,
            keystone: 0,
            sub_style: 0,
            largest_multi_kill: 0,
            metrics: None,
            participants: Vec::new(),
            comparison: None,
        }
    }

    fn many(n: usize, queue_id: i64, result: GameResult, kda: [i64; 3]) -> Vec<GameSummary> {
        let [k, d, a] = kda;
        (0..n).map(|_| game(queue_id, result, k, d, a)).collect()
    }

    fn page(games: Vec<GameSummary>) -> MatchHistoryPage {
        MatchHistoryPage {
            puuid: "p".to_owned(),
            start: 0,
            count: 20,
            source: DataSource::Sgp,
            sgp_error: None,
            games,
        }
    }

    fn has(profile: &PlayerProfile, id: &str) -> bool {
        profile.tags.iter().any(|t| t.id == id)
    }

    #[test]
    fn detects_losing_streak_ignoring_remakes() {
        let mut games = many(1, 420, GameResult::Remake, [0, 0, 0]);
        games.extend(many(3, 420, GameResult::Loss, [1, 5, 1]));
        games.extend(many(1, 420, GameResult::Win, [5, 1, 5]));
        let profile = build(&page(games), &ProfileContext::default());
        let streak = profile.tags.iter().find(|t| t.id == "streak").unwrap();
        assert_eq!(streak.label, "3 连败");
        assert_eq!(streak.tone, Tone::Negative);
    }

    #[test]
    fn prefers_current_queue_sample() {
        let mut games = many(6, 450, GameResult::Win, [9, 1, 9]);
        games.extend(many(5, 420, GameResult::Loss, [1, 5, 1]));
        let ctx = ProfileContext {
            queue_id: 440,
            ..ProfileContext::default()
        };
        let profile = build(&page(games), &ctx);
        assert_eq!(profile.scope, SampleScope::Ranked);
        assert_eq!(profile.sample_games, 5);
        assert_eq!(profile.wins, 0);
    }

    #[test]
    fn ranked_form_counts_positions() {
        let mut games = many(6, 420, GameResult::Win, [5, 2, 5]);
        games[0].position = "JUNGLE".to_owned();
        games[1].position = String::new();
        let refs: Vec<&GameSummary> = games.iter().collect();
        let form = ranked_form(&refs, "JUNGLE").unwrap();
        assert_eq!(form.positioned_games, 5);
        assert_eq!(form.main_position, "MIDDLE");
        assert_eq!((form.main_games, form.role_games), (4, 1));
    }

    #[test]
    fn ranked_form_skips_other_queues_and_discounts_off_role_games() {
        let mut games = many(5, 450, GameResult::Loss, [0, 10, 0]);
        games.extend(many(3, 420, GameResult::Loss, [0, 10, 0]));
        games.extend(many(3, 440, GameResult::Win, [5, 2, 5]));
        for game in &mut games[5..8] {
            game.position = "UTILITY".to_owned();
        }
        let ctx = ProfileContext {
            position: "MIDDLE".to_owned(),
            ..ProfileContext::default()
        };
        let form = build(&page(games), &ctx).form.unwrap();
        assert_eq!(form.games, 6);
        assert_eq!(form.off_role_games, 3);
        assert!(form.wins / form.weight > 0.6);
    }

    #[test]
    fn champion_practice_and_signature() {
        let games = many(10, 420, GameResult::Win, [5, 2, 5]);
        let rarely = |points: Option<i64>| ProfileContext {
            champion_id: 99,
            champion_points: points,
            ..ProfileContext::default()
        };
        let profile = build(&page(games.clone()), &rarely(Some(5_000)));
        assert!(has(&profile, "champion-practice"));
        let profile = build(&page(games.clone()), &rarely(Some(300_000)));
        assert!(has(&profile, "champion-veteran"));
        assert!(!has(&profile, "champion-practice"));
        let profile = build(&page(games.clone()), &rarely(None));
        assert!(has(&profile, "champion-rare"));
        let main = ProfileContext {
            champion_id: 1,
            ..ProfileContext::default()
        };
        let profile = build(&page(games), &main);
        assert!(has(&profile, "signature-champion"));
    }

    #[test]
    fn flags_flash_on_both_keys() {
        let ctx = ProfileContext::default();
        let mut games = many(4, 420, GameResult::Win, [1, 1, 1]);
        games[0].spells = [14, 4];
        assert!(!has(&build(&page(games.clone()), &ctx), "flash-key"));
        games[1].spells = [14, 4];
        assert!(has(&build(&page(games), &ctx), "flash-key"));
    }

    #[test]
    fn boosting_needs_a_ranked_run_far_above_older_games() {
        let mut games = many(4, 420, GameResult::Win, [12, 1, 8]);
        games.extend(many(5, 420, GameResult::Loss, [2, 7, 3]));
        let ranked = ProfileContext {
            queue_id: 420,
            ..ProfileContext::default()
        };
        let profile = build(&page(games.clone()), &ranked);
        assert!(has(&profile, "suspicious-boosting"));
        let profile = build(&page(games), &ProfileContext::default());
        assert!(!has(&profile, "suspicious-boosting"));
    }

    #[test]
    fn akari_score_is_bounded() {
        let m = GameMetrics {
            team_size: 5,
            damage_share: 1.0,
            damage_taken_share: 1.0,
            gold_share: 1.0,
            cs_share: 1.0,
            vision_share: 1.0,
            kill_share: 1.0,
            kill_participation: 1.0,
            heal_ratio: 10.0,
            solo_kills: 5.0,
            enemy_missing_pings: 0,
        };
        let mut g = game(420, GameResult::Win, 30, 0, 30);
        g.cs = 600;
        g.metrics = Some(m);
        let games = vec![g.clone(), g.clone(), g];
        let profile = build(&page(games), &ProfileContext::default());
        let score = profile.akari_score.unwrap();
        assert!((score.total - akari::MAX).abs() < 1e-9);
    }
}
