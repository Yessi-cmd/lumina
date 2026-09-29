//! User settings, persisted as JSON in the app config directory.

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
/// Phone push channels: Bark (iOS) and Server酱 (WeChat).
pub const PUSH_PROVIDERS: [&str; 3] = ["off", "bark", "serverchan"];

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
}
