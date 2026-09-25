//! Log file for debugging reports: everything at Info, Lumina's own modules at Debug,
//! 5 MB per file with the five most recent files kept, local timestamps, and panics
//! recorded before the process aborts. Frontend errors arrive via `log_frontend`.

use std::path::PathBuf;

use tauri::plugin::TauriPlugin;
use tauri::{AppHandle, Manager, Wry};
use tauri_plugin_log::{RotationStrategy, Target, TargetKind, TimezoneStrategy};

use crate::error::{AppError, Result};

const FILE_NAME: &str = "lumina";
const MAX_FILE_BYTES: u128 = 5 * 1024 * 1024;
const KEEP_FILES: usize = 5;

pub fn plugin() -> TauriPlugin<Wry> {
    let file = TargetKind::LogDir {
        file_name: Some(FILE_NAME.to_owned()),
    };
    tauri_plugin_log::Builder::new()
        .clear_targets()
        .targets([Target::new(TargetKind::Stdout), Target::new(file)])
        .level(log::LevelFilter::Info)
        .level_for("lumina_lib", log::LevelFilter::Debug)
        .max_file_size(MAX_FILE_BYTES)
        .rotation_strategy(RotationStrategy::KeepSome(KEEP_FILES))
        .timezone_strategy(TimezoneStrategy::UseLocal)
        .build()
}

/// Release builds use `panic = "abort"`, so without this nothing records why the app died.
pub fn record_panics() {
    let default_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        log::error!("panic: {info}");
        default_hook(info);
    }));
}

pub fn dir(app: &AppHandle) -> Result<PathBuf> {
    let dir = app.path().app_log_dir();
    dir.map_err(|err| AppError::Message(format!("找不到日志目录: {err}")))
}

pub fn open_dir(app: &AppHandle) -> Result<()> {
    let dir = dir(app)?;
    std::fs::create_dir_all(&dir)?;
    // Explorer hands the window off and exits at once (with code 1), so its status is moot.
    std::process::Command::new("explorer").arg(&dir).status()?;
    Ok(())
}

/// Messages forwarded from the webview (uncaught errors, `console.warn/error`).
pub fn frontend(level: &str, message: &str) {
    match level {
        "error" => log::error!(target: "frontend", "{message}"),
        "warn" => log::warn!(target: "frontend", "{message}"),
        _ => log::info!(target: "frontend", "{message}"),
    }
}
