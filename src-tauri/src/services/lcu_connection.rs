//! Keeps a connection to the League client alive: discover → connect → stream events,
//! and start over whenever the client exits.

use std::time::Duration;

use tauri::{AppHandle, Manager};

use crate::clients::lcu::discovery::{self, Credentials, Discovery};
use crate::clients::lcu::events::UriRouter;
use crate::clients::lcu::http::LcuHttp;
use crate::clients::lcu::models::{LcuEventType, Summoner};
use crate::clients::lcu::ws::LcuSocket;
use crate::error::Result;
use crate::state::gameflow::PHASE_NONE;
use crate::state::{AppState, ClientInfo, ConnectionStatus};

const SCAN_INTERVAL: Duration = Duration::from_secs(2);
/// How long to wait for a freshly started client to answer API calls.
const READY_ATTEMPTS: u32 = 30;
const UNREADABLE_HINT: &str =
    "检测到英雄联盟客户端，但无法读取连接信息。请尝试以管理员身份运行 Lumina。";

const CURRENT_SUMMONER: &str = "/lol-summoner/v1/current-summoner";
const GAMEFLOW_PHASE: &str = "/lol-gameflow/v1/gameflow-phase";
const PLATFORM_ID: &str = "/lol-platform-config/v1/namespaces/LoginDataPacket/platformId";

pub fn spawn(app: AppHandle) {
    tauri::async_runtime::spawn(supervise(app));
}

async fn supervise(app: AppHandle) {
    loop {
        let scan = tauri::async_runtime::spawn_blocking(discovery::discover).await;
        let state = app.state::<AppState>();
        match scan {
            Ok(Discovery::Found(creds)) => {
                log::info!("found League client: {creds:?}");
                if let Err(err) = run_session(&app, &creds).await {
                    log::warn!("LCU session ended: {err}");
                    state.reset_lcu(Some(err.to_string()));
                } else {
                    state.reset_lcu(None);
                }
            }
            Ok(Discovery::Unreadable) => state.reset_lcu(Some(UNREADABLE_HINT.to_owned())),
            Ok(Discovery::NotRunning) => state.reset_lcu(None),
            Err(err) => log::error!("process scan failed: {err}"),
        }
        tokio::time::sleep(SCAN_INTERVAL).await;
    }
}

/// One connection to one client process. Returns when the WebSocket closes.
async fn run_session(app: &AppHandle, creds: &Credentials) -> Result<()> {
    let state = app.state::<AppState>();
    state.update_lcu(|s| {
        s.status = ConnectionStatus::Connecting;
        s.client = Some(ClientInfo {
            pid: creds.pid,
            port: creds.port,
            platform_id: creds.platform_id.clone(),
            source: creds.source,
        });
    });

    let http = LcuHttp::new(creds)?;
    let summoner = wait_until_ready(&state, &http).await?;

    // Subscribe before reading the initial phase so no transition is missed in between.
    let socket = LcuSocket::connect(creds).await?;
    let phase = http.get::<String>(GAMEFLOW_PHASE).await;
    let platform_id = match &creds.platform_id {
        Some(id) => Some(id.clone()),
        None => http.get::<String>(PLATFORM_ID).await.ok(),
    };

    state.update_lcu(|s| {
        s.status = ConnectionStatus::Connected;
        s.summoner = Some(summoner);
        s.last_error = None;
        if let Some(client) = &mut s.client {
            client.platform_id = platform_id;
        }
    });
    state.set_gameflow_phase(phase.unwrap_or_else(|_| PHASE_NONE.to_owned()));
    log::info!("connected to LCU on port {}", creds.port);

    let router = event_router(app.clone());
    socket.run(|event| router.dispatch(&event)).await
}

/// The UX process accepts connections a little before its plugins are ready.
async fn wait_until_ready(state: &AppState, http: &LcuHttp) -> Result<Summoner> {
    let mut attempt = 1;
    loop {
        match http.get::<Summoner>(CURRENT_SUMMONER).await {
            Ok(summoner) => return Ok(summoner),
            Err(err) if attempt < READY_ATTEMPTS => {
                log::debug!("LCU not ready (attempt {attempt}): {err}");
                state.update_lcu(|s| s.last_error = Some(err.to_string()));
                attempt += 1;
                tokio::time::sleep(SCAN_INTERVAL).await;
            }
            Err(err) => return Err(err),
        }
    }
}

fn event_router(app: AppHandle) -> UriRouter {
    let phase_app = app.clone();
    UriRouter::default()
        .on(GAMEFLOW_PHASE, move |event| {
            let phase = match event.event_type {
                LcuEventType::Delete => None,
                _ => event.data.as_str(),
            };
            let phase = phase.unwrap_or(PHASE_NONE).to_owned();
            phase_app.state::<AppState>().set_gameflow_phase(phase);
        })
        .on(CURRENT_SUMMONER, move |event| {
            if event.event_type == LcuEventType::Delete {
                return;
            }
            match serde_json::from_value::<Summoner>(event.data.clone()) {
                Ok(summoner) => {
                    let state = app.state::<AppState>();
                    state.update_lcu(|s| s.summoner = Some(summoner));
                }
                Err(err) => log::warn!("unexpected current-summoner payload: {err}"),
            }
        })
}
