//! User settings, persisted as JSON in the app config directory.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager};

use crate::error::{AppError, Result};

const FILE_NAME: &str = "settings.json";
pub const MAX_ACCEPT_DELAY_SECS: u32 = 10;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Settings {
    pub auto_accept: bool,
    /// Seconds to wait before accepting, leaving time to decline by hand.
    pub auto_accept_delay_secs: u32,
    /// Bring the window forward when champ select starts and when the game loads.
    pub auto_show_panel: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            auto_accept: false,
            auto_accept_delay_secs: 2,
            auto_show_panel: true,
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
        assert_eq!(s.auto_accept_delay_secs, MAX_ACCEPT_DELAY_SECS);
    }
}
