//! Champ-select overlays beside the League client, as in Akari-Yessi: teammates' recent
//! solo/duo champions on the left, counter picks against the enemy picks on the right.
//!
//! The overlays are separate undecorated, transparent, non-focusable windows that follow
//! the client window every tick. They show only while the client is in the foreground
//! and not minimized, so they never float over other programs, and clicking them never
//! takes keyboard focus away from the client (champ-select chat keeps working).

use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use tauri::{
    AppHandle, Manager, PhysicalPosition, PhysicalSize, WebviewUrl, WebviewWindow,
    WebviewWindowBuilder,
};

use super::client_window::{self, ClientWindow, Rect};
use crate::state::AppState;

const TICK: Duration = Duration::from_millis(250);
/// Client height the overlay layouts are designed for; they scale with the client.
const DESIGN_HEIGHT: f64 = 720.0;
/// Smaller clients than this are being resized or are not the champ-select window.
const MIN_CLIENT_HEIGHT: i32 = 300;
/// Set while a follow loop runs, so phase events do not start a second one.
static FOLLOWING: AtomicBool = AtomicBool::new(false);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Side {
    Left,
    Right,
}

struct Overlay {
    /// Also tells the frontend which overlay to render.
    label: &'static str,
    side: Side,
    design_width: f64,
}

const OVERLAYS: [Overlay; 2] = [
    Overlay {
        label: "overlay-allies",
        side: Side::Left,
        design_width: 220.0,
    },
    Overlay {
        label: "overlay-enemies",
        side: Side::Right,
        design_width: 240.0,
    },
];

pub fn on_phase(app: &AppHandle, phase: &str) {
    if phase != "ChampSelect" || !enabled(app) {
        hide_all(app);
        return;
    }
    if FOLLOWING.swap(true, Ordering::SeqCst) {
        return;
    }
    tauri::async_runtime::spawn(follow_client(app.clone()));
}

fn enabled(app: &AppHandle) -> bool {
    app.state::<AppState>().settings().champ_select_overlay
}

fn in_champ_select(app: &AppHandle) -> bool {
    let phase = app.state::<AppState>().lcu_snapshot().gameflow_phase;
    phase == "ChampSelect" && enabled(app)
}

async fn follow_client(app: AppHandle) {
    let mut windows = Vec::new();
    for overlay in &OVERLAYS {
        match window(&app, overlay) {
            Ok(window) => windows.push((overlay, window)),
            Err(err) => log::warn!("failed to create {}: {err}", overlay.label),
        }
    }
    log::info!("champ-select overlays following the client");
    let mut was_visible = None;
    while in_champ_select(&app) {
        let client = client_window::find().filter(usable);
        if was_visible != Some(client.is_some()) {
            was_visible = Some(client.is_some());
            let state = if client.is_some() { "shown" } else { "hidden" };
            log::debug!("champ-select overlays {state}");
        }
        for (overlay, window) in &windows {
            match &client {
                Some(client) => place(overlay, window, client),
                None => hide(window),
            }
        }
        tokio::time::sleep(TICK).await;
    }
    for (_, window) in &windows {
        hide(window);
    }
    FOLLOWING.store(false, Ordering::SeqCst);
}

fn usable(client: &ClientWindow) -> bool {
    let big_enough = client.bounds.height() >= MIN_CLIENT_HEIGHT;
    client.foreground && !client.minimized && big_enough
}

fn window(app: &AppHandle, overlay: &Overlay) -> tauri::Result<WebviewWindow> {
    if let Some(window) = app.get_webview_window(overlay.label) {
        return Ok(window);
    }
    let url = WebviewUrl::App("index.html".into());
    WebviewWindowBuilder::new(app, overlay.label, url)
        .title("Lumina")
        .decorations(false)
        .transparent(true)
        .shadow(false)
        .skip_taskbar(true)
        .always_on_top(true)
        .resizable(false)
        .focusable(false)
        .focused(false)
        .visible(false)
        .build()
}

fn place(overlay: &Overlay, window: &WebviewWindow, client: &ClientWindow) {
    let (x, y, width, height) = bounds(overlay, &client.bounds, &client.work_area);
    let position = PhysicalPosition::new(x, y);
    if window.outer_position().ok() != Some(position) {
        let _ = window.set_position(position);
    }
    let size = PhysicalSize::new(width, height);
    if window.inner_size().ok() != Some(size) {
        let _ = window.set_size(size);
    }
    if !window.is_visible().unwrap_or(false) {
        let _ = window.show();
    }
}

/// Outside the client on its side, clamped to the monitor so it overlaps the client when
/// the client touches the screen edge. Same height as the client.
fn bounds(overlay: &Overlay, client: &Rect, work: &Rect) -> (i32, i32, u32, u32) {
    let scale = f64::from(client.height()) / DESIGN_HEIGHT;
    let width = (overlay.design_width * scale).round() as i32;
    let x = match overlay.side {
        Side::Left => (client.left - width).max(work.left),
        Side::Right => client.right.min(work.right - width),
    };
    let height = client.height().max(0) as u32;
    (x, client.top, width.max(0) as u32, height)
}

fn hide(window: &WebviewWindow) {
    if window.is_visible().unwrap_or(false) {
        let _ = window.hide();
    }
}

fn hide_all(app: &AppHandle) {
    for overlay in &OVERLAYS {
        if let Some(window) = app.get_webview_window(overlay.label) {
            hide(&window);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rect(left: i32, top: i32, right: i32, bottom: i32) -> Rect {
        Rect {
            left,
            top,
            right,
            bottom,
        }
    }

    #[test]
    fn overlays_sit_outside_the_client_and_scale_with_it() {
        let work = rect(0, 0, 2560, 1400);
        let client = rect(600, 200, 1880, 920);
        assert_eq!(bounds(&OVERLAYS[0], &client, &work), (380, 200, 220, 720));
        assert_eq!(bounds(&OVERLAYS[1], &client, &work), (1880, 200, 240, 720));

        let big = rect(600, 0, 2040, 1080);
        assert_eq!(bounds(&OVERLAYS[0], &big, &work).2, 330);
    }

    #[test]
    fn overlays_stay_on_screen() {
        let work = rect(0, 0, 1920, 1040);
        let client = rect(0, 0, 1280, 720);
        assert_eq!(bounds(&OVERLAYS[0], &client, &work).0, 0);
        let right = rect(640, 0, 1920, 720);
        assert_eq!(bounds(&OVERLAYS[1], &right, &work).0, 1680);
    }
}
