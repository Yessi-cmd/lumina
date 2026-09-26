//! Brings the main window forward when champ select starts and while the game loads.
//!
//! Windows does not let a background app take focus, and the game window only appears a
//! few seconds after GameStart, so focusing is not enough. Instead the window is made
//! topmost: briefly for champ select, and for the whole loading screen until the Live
//! Client API reports the match clock running (or five minutes pass).

use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use tauri::{AppHandle, Manager, WebviewWindow};

use crate::clients::live::LiveClient;
use crate::state::AppState;

const MAIN_WINDOW: &str = "main";
const POLL_INTERVAL: Duration = Duration::from_secs(2);
const MAX_PIN: Duration = Duration::from_secs(5 * 60);
/// Set while a loading-screen pin is active, so a second GameStart does not stack another.
static PINNED: AtomicBool = AtomicBool::new(false);

pub fn on_phase(app: &AppHandle, phase: &str) {
    if !app.state::<AppState>().settings().auto_show_panel {
        return;
    }
    let Some(window) = app.get_webview_window(MAIN_WINDOW) else {
        return;
    };
    // The champ-select overlays sit beside the client and hide whenever the client is
    // not in front, so raising the main window then would hide them.
    let overlays = app.state::<AppState>().settings().champ_select_overlay;
    match phase {
        "ChampSelect" if !overlays => raise(&window),
        "GameStart" => pin_during_loading(app, window),
        _ => {}
    }
}

/// Topmost on and off again lifts the window above the client without keeping it there.
fn raise(window: &WebviewWindow) {
    show(window);
    set_topmost(window, true);
    set_topmost(window, false);
    let _ = window.set_focus();
    log::info!("showing game panel for champ select");
}

fn pin_during_loading(app: &AppHandle, window: WebviewWindow) {
    if PINNED.swap(true, Ordering::SeqCst) {
        return;
    }
    show(&window);
    set_topmost(&window, true);
    log::info!("pinning game panel over the loading screen");
    tauri::async_runtime::spawn(unpin_when_loaded(app.clone(), window));
}

async fn unpin_when_loaded(app: AppHandle, window: WebviewWindow) {
    let started = Instant::now();
    let live = LiveClient::new().ok();
    let reason = loop {
        tokio::time::sleep(POLL_INTERVAL).await;
        let phase = app.state::<AppState>().lcu_snapshot().gameflow_phase;
        if !matches!(phase.as_str(), "GameStart" | "InProgress") {
            break "left the game";
        }
        if started.elapsed() > MAX_PIN {
            break "timed out";
        }
        if let Some(live) = &live {
            if matches!(live.game_time().await, Ok(t) if t > 0.0) {
                break "match started";
            }
        }
    };
    set_topmost(&window, false);
    PINNED.store(false, Ordering::SeqCst);
    let secs = started.elapsed().as_secs();
    log::info!("unpinned game panel after {secs}s: {reason}");
}

fn show(window: &WebviewWindow) {
    if let Err(err) = window.unminimize().and_then(|()| window.show()) {
        log::warn!("failed to show the window: {err}");
    }
}

fn set_topmost(window: &WebviewWindow, topmost: bool) {
    if let Err(err) = window.set_always_on_top(topmost) {
        log::warn!("failed to set always-on-top to {topmost}: {err}");
    }
}
