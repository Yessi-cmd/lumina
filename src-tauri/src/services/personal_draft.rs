//! Personal BP recommendations. Familiarity is a hard priority; matchup statistics
//! can only reorder champions within that priority. No pick or lock is ever written.

use std::collections::{HashMap, HashSet};
use std::time::Duration;

use futures_util::{stream, StreamExt};
use serde::{Deserialize, Serialize};

use super::champion_assist::{Matchup, Verdict, POSITIONS};
use super::match_history::{DataSource, GameResult, GameSummary};
use crate::config::PoolPreference;
use crate::error::{AppError, Result};
use crate::state::AppState;

#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RecommendationReport {
    pub candidates: Vec<Candidate>,
    pub warnings: Vec<String>,
    pub sample_games: usize,
    pub queue_id: i64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Candidate {
    pub champion_id: i64,
    pub label: String,
    pub games: usize,
    pub wins: usize,
    pub reasons: Vec<String>,
    pub matchups: Vec<Matchup>,
    #[serde(skip)]
    familiar: bool,
    #[serde(skip)]
    score: f64,
    #[serde(skip)]
    fills: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
#[serde(default, rename_all = "camelCase")]
struct DraftContext {
    game_id: i64,
    local_player_cell_id: Option<i64>,
    my_team: Vec<Member>,
    their_team: Vec<Member>,
    actions: Vec<Vec<Action>>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
#[serde(default, rename_all = "camelCase")]
struct Member {
    cell_id: i64,
    champion_id: i64,
    champion_pick_intent: i64,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
#[serde(default, rename_all = "camelCase")]
struct Action {
    actor_cell_id: i64,
    champion_id: i64,
    #[serde(rename = "type")]
    kind: String,
    completed: bool,
}

impl DraftContext {
    fn enemies(&self) -> Vec<i64> {
        // Team championId may be only a hover. Use completed pick actions.
        let cells: HashSet<i64> = self.their_team.iter().map(|m| m.cell_id).collect();
        self.actions
            .iter()
            .flatten()
            .filter(|a| a.completed && a.kind == "pick" && cells.contains(&a.actor_cell_id))
            .map(|a| a.champion_id)
            .filter(|id| *id > 0)
            .collect()
    }

    fn allies(&self) -> Vec<i64> {
        self.my_team
            .iter()
            .filter(|m| Some(m.cell_id) != self.local_player_cell_id)
            .map(|m| m.champion_id)
            .filter(|id| *id > 0)
            .collect()
    }

    fn unavailable(&self) -> HashSet<i64> {
        let mut ids: HashSet<i64> = self
            .actions
            .iter()
            .flatten()
            .filter(|a| a.completed && (a.kind == "pick" || a.kind == "ban"))
            .map(|a| a.champion_id)
            .collect();
        // Respect teammates' intent as well as completed picks.
        for member in &self.my_team {
            if Some(member.cell_id) != self.local_player_cell_id {
                ids.extend([member.champion_id, member.champion_pick_intent]);
            }
        }
        ids
    }
}

pub async fn recommend(
    state: &AppState,
    position: &str,
    opponent_id: i64,
) -> Result<RecommendationReport> {
    if !POSITIONS.contains(&position) {
        return Err(AppError::Message("请选择有效分路".to_owned()));
    }
    let session = state.session()?;
    let snapshot = state.lcu_snapshot();
    let summoner = snapshot.summoner.ok_or(AppError::NotConnected)?;
    let mut report = RecommendationReport::default();
    if snapshot.gameflow_phase != "ChampSelect" {
        report
            .warnings
            .push("进入英雄选择后生成个人推荐".to_owned());
        return Ok(report);
    }
    let queue = state.roster().map_or(0, |r| r.queue_id);
    if queue == 0 {
        report
            .warnings
            .push("正在识别对局模式，请稍后刷新".to_owned());
        return Ok(report);
    }
    if ![400, 420, 430, 440].contains(&queue) {
        report
            .warnings
            .push("个人 BP 暂仅支持常规峡谷选人".to_owned());
        return Ok(report);
    }
    let settings = state.settings();
    let platform = snapshot
        .client
        .and_then(|c| c.platform_id)
        .unwrap_or_default();
    let account = format!("{platform}:{}", summoner.puuid);
    let entries = settings
        .champion_pools
        .get(&account)
        .and_then(|lanes| lanes.get(position))
        .cloned()
        .unwrap_or_default();
    let draft: DraftContext = session.http.get("/lol-champ-select/v1/session").await?;
    let Some(cell) = draft.local_player_cell_id.filter(|cell| *cell >= 0) else {
        return Err(AppError::Message("观战时不生成个人推荐".to_owned()));
    };
    if !draft.my_team.iter().any(|m| m.cell_id == cell) {
        return Err(AppError::Message("尚未获取到自己的选人席位".to_owned()));
    }
    if draft
        .actions
        .iter()
        .flatten()
        .any(|a| a.actor_cell_id == cell && a.kind == "pick" && a.completed)
    {
        report
            .warnings
            .push("你已锁定英雄，可以继续查看出装与对位".to_owned());
        return Ok(report);
    }
    let pickable: Vec<i64> = session
        .http
        .get("/lol-champ-select/v1/pickable-champion-ids")
        .await?;
    let unavailable = draft.unavailable();
    let available: HashSet<i64> = pickable
        .into_iter()
        .filter(|id| !unavailable.contains(id))
        .collect();
    report.queue_id = if queue == 440 { 440 } else { 420 };
    let history_request =
        state
            .match_history
            .get_queue(&session, &summoner.puuid, 0, 50, Some(report.queue_id));
    let tier_request = state
        .champion_assist
        .lane_entries(position, &settings.stats_tier);
    let (history, tiers) = tokio::join!(history_request, tier_request);
    let games = match history {
        Ok(page) => {
            if page.source == DataSource::Lcu {
                report
                    .warnings
                    .push("使用客户端回退战绩；缺少分路的对局不计入个人表现".to_owned());
            }
            page.games
        }
        Err(_) => {
            report
                .warnings
                .push("个人战绩暂不可用；仅使用手动英雄池和可用的全局数据".to_owned());
            Vec::new()
        }
    };
    let mut stats = HashMap::<i64, (usize, usize)>::new();
    for game in games
        .iter()
        .filter(|game| eligible(game, position, report.queue_id))
    {
        let row = stats.entry(game.champion_id).or_default();
        row.0 += 1;
        row.1 += usize::from(game.result == GameResult::Win);
        report.sample_games += 1;
    }
    let tiers = match tiers {
        Ok(rows) => rows,
        Err(_) => {
            report
                .warnings
                .push("全局分路榜暂不可用；保留个人英雄池推荐".to_owned());
            Vec::new()
        }
    };
    let mut ids: HashSet<i64> = stats.keys().copied().collect();
    ids.extend(entries.iter().map(|e| e.champion_id));
    // Cold start and practice alternatives must actually have lane data.
    ids.extend(
        tiers
            .iter()
            .filter(|t| available.contains(&t.champion_id))
            .take(5)
            .map(|t| t.champion_id),
    );
    let mut candidates: Vec<Candidate> = ids
        .into_iter()
        .filter(|id| available.contains(id))
        .filter_map(|id| {
            let preference = entries
                .iter()
                .find(|e| e.champion_id == id)
                .map(|e| e.preference);
            let (games, wins) = stats.get(&id).copied().unwrap_or_default();
            candidate(id, games, wins, preference)
        })
        .collect();
    for candidate in &mut candidates {
        if let Some(tier) = tiers
            .iter()
            .find(|t| t.champion_id == candidate.champion_id && t.games >= 200)
        {
            candidate.score += (tier.win_rate - 50.0).clamp(-3.0, 3.0);
        }
        add_composition(candidate, &draft.allies());
    }
    sort_candidates(&mut candidates);
    // Bound external requests while retaining a practice alternative.
    let practice = candidates.iter().find(|c| !c.familiar).cloned();
    candidates.truncate(8);
    if let Some(practice) = practice {
        if !candidates
            .iter()
            .any(|c| c.champion_id == practice.champion_id)
        {
            candidates.push(practice);
        }
    }
    let enemies = draft.enemies();
    let enemies = if opponent_id > 0 {
        vec![opponent_id]
    } else {
        enemies
    };
    if enemies.is_empty() {
        report
            .warnings
            .push("尚无已锁定的敌方英雄，当前优先按熟练度推荐".to_owned());
    } else {
        let results = stream::iter(candidates.into_iter().map(|mut candidate| {
            let session = &session;
            let enemies = &enemies;
            let tier = &settings.matchup_tier;
            async move {
                let request = state.champion_assist.matchup_rows(
                    session,
                    candidate.champion_id,
                    position,
                    tier,
                );
                match tokio::time::timeout(Duration::from_secs(6), request).await {
                    Ok(Ok(rows)) => {
                        add_matchups(&mut candidate, &rows, enemies, position, opponent_id > 0)
                    }
                    _ => candidate
                        .reasons
                        .push("对位数据暂不可用，未计入推荐".to_owned()),
                }
                candidate
            }
        }))
        .buffer_unordered(3)
        .collect::<Vec<_>>()
        .await;
        candidates = results;
    }
    report.candidates = select_candidates(candidates);
    if report.candidates.is_empty() {
        report.warnings.push(
            "没有可推荐的英雄：可能已被禁选、被队友预选或设为不推荐；可调整英雄池后重试".to_owned(),
        );
    }
    // Reject a result for a previous account, pool, or draft instead of showing stale BP.
    state.ensure_current_session(&session)?;
    let current: DraftContext = session.http.get("/lol-champ-select/v1/session").await?;
    state.ensure_current_session(&session)?;
    if current != draft
        || state.settings().champion_pools != settings.champion_pools
        || state.lcu_snapshot().gameflow_phase != "ChampSelect"
    {
        return Err(AppError::Message(
            "选人或英雄池已变化，正在等待刷新".to_owned(),
        ));
    }
    Ok(report)
}

fn eligible(game: &GameSummary, position: &str, queue: i64) -> bool {
    game.queue_id == queue
        && game.position == position
        && matches!(game.result, GameResult::Win | GameResult::Loss)
}

fn candidate(
    id: i64,
    games: usize,
    wins: usize,
    preference: Option<PoolPreference>,
) -> Option<Candidate> {
    if preference == Some(PoolPreference::Excluded) {
        return None;
    }
    let familiar = preference == Some(PoolPreference::Familiar)
        || (preference != Some(PoolPreference::Practice) && games >= 8);
    let mut reasons = vec![match preference {
        Some(PoolPreference::Familiar) => "你已标记为会玩，优先考虑".to_owned(),
        Some(PoolPreference::Practice) => "你已标记为想练，不作为熟练首选".to_owned(),
        _ if familiar => "近期同分路至少 8 场，作为熟悉候选".to_owned(),
        _ => "个人经验不足，适合作为练习备选".to_owned(),
    }];
    if games > 0 {
        reasons.push(format!(
            "近 50 场该排位队列中，同分路 {games} 场 {wins} 胜{}",
            if games < 8 { "，样本偏少" } else { "" }
        ));
    } else {
        reasons.push("近期没有可用的同分路样本，不代表你不会玩".to_owned());
    }
    // Shrink small-sample win rate toward 50%; never let it cross familiarity tiers.
    let score = games.min(20) as f64 + ((wins as f64 + 4.0) / (games as f64 + 8.0) - 0.5) * 16.0;
    Some(Candidate {
        champion_id: id,
        label: String::new(),
        games,
        wins,
        reasons,
        matchups: Vec::new(),
        familiar,
        score,
        fills: false,
    })
}

fn add_matchups(
    candidate: &mut Candidate,
    rows: &[Matchup],
    enemies: &[i64],
    position: &str,
    pinned: bool,
) {
    let matching: Vec<Matchup> = rows
        .iter()
        .filter(|m| enemies.contains(&m.champion_id) && (pinned || m.usual_position == position))
        .cloned()
        .collect();
    if matching.len() != 1 {
        candidate
            .reasons
            .push("敌方对位不确定；各对位单独展示，不计入排序".to_owned());
    } else {
        let matchup = &matching[0];
        if matches!(matchup.verdict, Verdict::Counters | Verdict::Countered) {
            candidate.score += matchup.advantage.clamp(-5.0, 5.0);
        }
        candidate.reasons.push(
            if pinned {
                "按你指定的对位比较"
            } else {
                "按敌方英雄常用分路推测对位，可手动指定"
            }
            .to_owned(),
        );
    }
    candidate.matchups = if matching.is_empty() {
        rows.iter()
            .filter(|m| enemies.contains(&m.champion_id))
            .cloned()
            .collect()
    } else {
        matching
    };
}

/// Conservative, hand-maintained examples, not an exhaustive team classifier.
/// Missing tags never mean a champion cannot perform that role.
fn roles(id: i64) -> [bool; 3] {
    [
        [12, 14, 31, 36, 54, 57, 78, 89, 98, 111, 113, 154, 201, 516].contains(&id),
        [12, 32, 54, 57, 59, 89, 111, 113, 154, 497, 516].contains(&id),
        [
            1, 4, 7, 8, 9, 13, 30, 34, 45, 50, 61, 63, 69, 85, 99, 101, 112, 134, 161, 268,
        ]
        .contains(&id),
    ]
}

fn add_composition(candidate: &mut Candidate, allies: &[i64]) {
    // Too early to diagnose a composition with fewer than three teammate hovers.
    if allies.len() < 3 {
        return;
    }
    let tags = roles(candidate.champion_id);
    for (index, label) in ["前排", "先手开团", "法术输出"].iter().enumerate() {
        if tags[index] && !allies.iter().any(|id| roles(*id)[index]) {
            candidate.fills = true;
            candidate.score += 2.0;
            candidate.reasons.push(format!(
                "可提供{label}；当前队友选择中未识别到同类标签（粗略参考）"
            ));
            break;
        }
    }
}

fn sort_candidates(candidates: &mut [Candidate]) {
    candidates.sort_by(|a, b| {
        b.familiar
            .cmp(&a.familiar)
            .then_with(|| b.score.total_cmp(&a.score))
            .then_with(|| a.champion_id.cmp(&b.champion_id))
    });
}

fn select_candidates(mut candidates: Vec<Candidate>) -> Vec<Candidate> {
    sort_candidates(&mut candidates);
    if candidates.is_empty() {
        return candidates;
    }
    let mut first = candidates.remove(0);
    first.label = if first.familiar {
        "熟练首选"
    } else {
        "练习候选"
    }
    .to_owned();
    let mut selected = vec![first];
    if let Some(index) = candidates.iter().position(|c| c.familiar && c.fills) {
        let mut fill = candidates.remove(index);
        fill.label = "阵容补位".to_owned();
        selected.push(fill);
    }
    for mut candidate in candidates {
        if selected.len() == 3 {
            break;
        }
        candidate.label = if candidate.familiar {
            "熟练备选"
        } else {
            "练习候选"
        }
        .to_owned();
        selected.push(candidate);
    }
    selected
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn familiarity_wins_over_small_sample_perfect_winrate_and_counter() {
        let familiar = candidate(1, 0, 0, Some(PoolPreference::Familiar)).unwrap();
        let mut newcomer = candidate(2, 2, 2, None).unwrap();
        newcomer.score += 20.0;
        let result = select_candidates(vec![newcomer, familiar]);
        assert_eq!(result[0].champion_id, 1);
        assert_eq!(result[0].label, "熟练首选");
        assert!(result[0].reasons.iter().any(|r| r.contains("没有可用")));
    }

    #[test]
    fn explicit_preferences_override_history() {
        assert!(candidate(1, 50, 40, Some(PoolPreference::Excluded)).is_none());
        assert!(
            !candidate(1, 50, 40, Some(PoolPreference::Practice))
                .unwrap()
                .familiar
        );
        assert!(
            candidate(1, 0, 0, Some(PoolPreference::Familiar))
                .unwrap()
                .familiar
        );
    }

    #[test]
    fn ambiguous_or_small_sample_matchups_do_not_affect_rank() {
        let mut choice = candidate(1, 10, 5, None).unwrap();
        let before = choice.score;
        let row = |id, verdict| Matchup {
            champion_id: id,
            win_rate: 60.0,
            games: 100,
            advantage: 12.0,
            margin: 15.0,
            verdict,
            usual_position: "MIDDLE".to_owned(),
        };
        let rows = vec![row(2, Verdict::Counters), row(3, Verdict::Counters)];
        add_matchups(&mut choice, &rows, &[2, 3], "MIDDLE", false);
        assert_eq!(choice.score, before);
        assert_eq!(choice.matchups.len(), 2);
        add_matchups(
            &mut choice,
            &[row(2, Verdict::TooFewGames)],
            &[2],
            "MIDDLE",
            true,
        );
        assert_eq!(choice.score, before);
        add_matchups(&mut choice, &rows, &[2], "MIDDLE", true);
        assert_eq!(choice.score, before + 5.0);
    }

    #[test]
    fn respects_completed_bans_picks_and_teammate_intent() {
        let draft: DraftContext = serde_json::from_value(serde_json::json!({
            "localPlayerCellId": 0,
            "myTeam": [{"cellId": 0, "championId": 1}, {"cellId": 1, "championPickIntent": 2}],
            "theirTeam": [{"cellId": 5, "championId": 3}],
            "actions": [[{"actorCellId": 5, "championId": 3, "type": "pick", "completed": true},
                {"championId": 4, "type": "ban", "completed": true},
                {"championId": 5, "type": "ban", "completed": false}]]
        }))
        .unwrap();
        assert_eq!(draft.enemies(), vec![3]);
        let unavailable = draft.unavailable();
        assert!(unavailable.contains(&2) && unavailable.contains(&3) && unavailable.contains(&4));
        assert!(!unavailable.contains(&1) && !unavailable.contains(&5));
    }

    #[test]
    fn caps_unique_choices_and_only_calls_familiar_fill_a_composition_pick() {
        let mut fill = candidate(54, 0, 0, Some(PoolPreference::Familiar)).unwrap();
        add_composition(&mut fill, &[22, 238, 64]);
        let strong = candidate(1, 20, 12, None).unwrap();
        let result = select_candidates(vec![
            strong,
            fill,
            candidate(2, 2, 1, None).unwrap(),
            candidate(3, 0, 0, None).unwrap(),
        ]);
        assert_eq!(result.len(), 3);
        assert_eq!(result[1].label, "阵容补位");
    }
}
