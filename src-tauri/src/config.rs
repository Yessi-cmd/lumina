//! User settings, persisted as JSON in the app config directory.

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager};

use crate::error::{AppError, Result};

const FILE_NAME: &str = "settings.json";
pub const MAX_ACCEPT_DELAY_SECS: u32 = 10;
/// lolalytics rank filters offered in settings.
pub const STATS_TIERS: [&str; 4] = ["all", "platinum_plus", "emerald_plus", "diamond_plus"];
const DEFAULT_STATS_TIER: &str = "emerald_plus";
/// Matchups are judged on high-rank games, where both sides play their champions well.
pub const MATCHUP_TIERS: [&str; 3] = ["emerald_plus", "diamond_plus", "master_plus"];
const DEFAULT_MATCHUP_TIER: &str = "diamond_plus";
pub const MAX_TILT_STREAK: u32 = 10;
/// Honor categories of the client: cool, shot caller, friendly.
pub const HONOR_CATEGORIES: [&str; 3] = ["COOL", "SHOTCALLER", "HEART"];
const DEFAULT_HONOR_CATEGORY: &str = "HEART";
/// Positions with an auto-select preset; `ANY` backs up every position.
pub const PRESET_POSITIONS: [&str; 6] = ["TOP", "JUNGLE", "MIDDLE", "BOTTOM", "UTILITY", "ANY"];
const MAX_PRESET_LEN: usize = 10;
const MAX_SELECT_DELAY_SECS: u32 = 10;
/// Phone push channels: Bark (iOS) and Server酱 (WeChat).
pub const PUSH_PROVIDERS: [&str; 3] = ["off", "bark", "serverchan"];

/// Champions to ban and pick for one position, best first.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Preset {
    pub picks: Vec<i64>,
    pub bans: Vec<i64>,
}

/// Automatic ban and pick during champ select, see `services/auto_select.rs`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct AutoSelect {
    pub ban: bool,
    pub pick: bool,
    /// Seconds between hovering a champion and locking it in.
    pub delay_secs: u32,
    /// Keyed by `PRESET_POSITIONS`.
    pub presets: HashMap<String, Preset>,
}

impl Default for AutoSelect {
    fn default() -> Self {
        Self {
            ban: false,
            pick: false,
            delay_secs: 1,
            presets: HashMap::new(),
        }
    }
}

impl AutoSelect {
    fn normalized(mut self) -> Self {
        self.delay_secs = self.delay_secs.min(MAX_SELECT_DELAY_SECS);
        let known = |position: &String| PRESET_POSITIONS.contains(&position.as_str());
        self.presets.retain(|position, _| known(position));
        for preset in self.presets.values_mut() {
            clean_ids(&mut preset.picks);
            clean_ids(&mut preset.bans);
        }
        self
    }
}

/// Real champion ids only, each once, in order.
fn clean_ids(ids: &mut Vec<i64>) {
    let mut seen = HashSet::new();
    ids.retain(|id| *id > 0 && seen.insert(*id));
    ids.truncate(MAX_PRESET_LEN);
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Settings {
    pub auto_accept: bool,
    /// Seconds to wait before accepting, leaving time to decline by hand.
    pub auto_accept_delay_secs: u32,
    /// Bring the window forward when champ select starts and when the game loads.
    pub auto_show_panel: bool,
    /// Rank filter for champion tiers and builds, one of `STATS_TIERS`.
    pub stats_tier: String,
    /// Rank filter for matchups (counters), one of `MATCHUP_TIERS`.
    pub matchup_tier: String,
    /// Overlays beside the client during champ select: teammates' champions and counters
    /// to the enemy picks.
    pub champ_select_overlay: bool,
    /// Warn after this many ranked losses in a row; 0 turns it off.
    pub tilt_streak: u32,
    /// Honor a random teammate when the vote screen appears.
    pub auto_honor: bool,
    /// One of `HONOR_CATEGORIES`.
    pub honor_category: String,
    /// One of `PUSH_PROVIDERS`.
    pub push_provider: String,
    /// Bark device key (or self-hosted address) or Server酱 SendKey.
    pub push_key: String,
    pub push_match_found: bool,
    pub push_champ_select: bool,
    pub push_tilt: bool,
    pub auto_select: AutoSelect,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            auto_accept: false,
            auto_accept_delay_secs: 2,
            auto_show_panel: true,
            stats_tier: DEFAULT_STATS_TIER.to_owned(),
            matchup_tier: DEFAULT_MATCHUP_TIER.to_owned(),
            champ_select_overlay: true,
            tilt_streak: 3,
            auto_honor: false,
            honor_category: DEFAULT_HONOR_CATEGORY.to_owned(),
            push_provider: "off".to_owned(),
            push_key: String::new(),
            push_match_found: true,
            push_champ_select: true,
            push_tilt: true,
            auto_select: AutoSelect::default(),
        }
    }
}

impl Settings {
    /// Missing or unreadable files fall back to defaults.
    pub fn load(app: &AppHandle) -> Self {
        let Some(path) = path(app) else {
            return Self::default();
        };
        let Ok(text) = std::fs::read_to_string(&path) else {
            return Self::default();
        };
        match serde_json::from_str::<Self>(&text) {
            Ok(settings) => settings.normalized(),
            Err(err) => {
                log::warn!("ignoring invalid {}: {err}", path.display());
                Self::default()
            }
        }
    }

    pub fn save(&self, app: &AppHandle) -> Result<()> {
        let Some(path) = path(app) else {
            return Err(AppError::Message("找不到配置目录".to_owned()));
        };
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        let json = serde_json::to_vec_pretty(self);
        let json = json.map_err(|err| AppError::Message(err.to_string()))?;
        std::fs::write(path, json)?;
        Ok(())
    }

    pub fn normalized(mut self) -> Self {
        self.auto_accept_delay_secs = self.auto_accept_delay_secs.min(MAX_ACCEPT_DELAY_SECS);
        if !STATS_TIERS.contains(&self.stats_tier.as_str()) {
            self.stats_tier = DEFAULT_STATS_TIER.to_owned();
        }
        if !MATCHUP_TIERS.contains(&self.matchup_tier.as_str()) {
            self.matchup_tier = DEFAULT_MATCHUP_TIER.to_owned();
        }
        self.tilt_streak = self.tilt_streak.min(MAX_TILT_STREAK);
        if !HONOR_CATEGORIES.contains(&self.honor_category.as_str()) {
            self.honor_category = DEFAULT_HONOR_CATEGORY.to_owned();
        }
        if !PUSH_PROVIDERS.contains(&self.push_provider.as_str()) {
            self.push_provider = "off".to_owned();
        }
        self.push_key = self.push_key.trim().to_owned();
        self.auto_select = std::mem::take(&mut self.auto_select).normalized();
        self
    }
}

fn path(app: &AppHandle) -> Option<PathBuf> {
    let dir = app.path().app_config_dir().ok()?;
    Some(dir.join(FILE_NAME))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fills_missing_fields_and_clamps_delay() {
        let json = r#"{"autoAcceptDelaySecs":99}"#;
        let s: Settings = serde_json::from_str(json).unwrap();
        let s = s.normalized();
        assert!(!s.auto_accept);
        assert!(s.auto_show_panel);
        assert_eq!(s.stats_tier, DEFAULT_STATS_TIER);
        assert_eq!(s.auto_accept_delay_secs, MAX_ACCEPT_DELAY_SECS);
        assert_eq!(s.tilt_streak, 3);
        assert_eq!(s.push_provider, "off");
    }

    #[test]
    fn rejects_unknown_choices_and_trims_the_push_key() {
        let s = Settings {
            tilt_streak: 99,
            honor_category: "X".to_owned(),
            push_provider: "sms".to_owned(),
            push_key: "  abc \n".to_owned(),
            ..Settings::default()
        };
        let s = s.normalized();
        assert_eq!(s.tilt_streak, MAX_TILT_STREAK);
        assert_eq!(s.honor_category, DEFAULT_HONOR_CATEGORY);
        assert_eq!(s.push_provider, "off");
        assert_eq!(s.push_key, "abc");
    }

    #[test]
    fn cleans_auto_select_presets() {
        let mut auto_select = AutoSelect {
            delay_secs: 99,
            ..AutoSelect::default()
        };
        let messy = Preset {
            picks: vec![5, 0, 5, -3, 7],
            bans: (1..=20).collect(),
        };
        auto_select.presets.insert("MIDDLE".to_owned(), messy);
        let unknown = "MID".to_owned();
        auto_select.presets.insert(unknown, Preset::default());
        let s = Settings {
            auto_select,
            ..Settings::default()
        };
        let auto_select = s.normalized().auto_select;
        assert_eq!(auto_select.delay_secs, MAX_SELECT_DELAY_SECS);
        assert_eq!(auto_select.presets.len(), 1);
        let preset = &auto_select.presets["MIDDLE"];
        assert_eq!(preset.picks, vec![5, 7]);
        assert_eq!(preset.bans.len(), MAX_PRESET_LEN);
    }
}
