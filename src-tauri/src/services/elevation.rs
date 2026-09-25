//! The Tencent client runs elevated, so its command line is only readable from an
//! elevated process. Like League Akari, offer to restart Lumina as administrator.

use std::process::Command;

use tauri::AppHandle;

use crate::error::{AppError, Result};

#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

/// Starts an elevated copy of Lumina through UAC, then exits this one.
pub async fn relaunch_as_admin(app: AppHandle) -> Result<()> {
    let exe = std::env::current_exe()?;
    let path = exe.to_string_lossy().replace('\'', "''");
    let script = format!("Start-Process -FilePath '{path}' -Verb RunAs");

    let mut cmd = Command::new("powershell");
    cmd.args(["-NoProfile", "-NonInteractive", "-Command"]);
    cmd.arg(script);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }

    let task = tauri::async_runtime::spawn_blocking(move || cmd.status());
    let Ok(status) = task.await else {
        return Err(AppError::Message("无法启动提权进程".to_owned()));
    };
    if !status?.success() {
        return Err(AppError::Message("管理员授权已取消".to_owned()));
    }

    app.exit(0);
    Ok(())
}
