//! Keeps a connection to the League client alive: discover → connect → stream events,
//! and start over whenever the client exits.

use std::sync::Arc;
use std::time::Duration;

use tauri::{AppHandle, Manager};
use tokio::sync::OnceCell;

use crate::clients::lcu::discovery::{self, Credentials, Discovery};
use crate::clients::lcu::events::UriRouter;
use crate::clients::lcu::http::LcuHttp;
use crate::clients::lcu::models::{LcuEventType, RegionLocale, Summoner};
use crate::clients::lcu::ws::LcuSocket;
use crate::clients::sgp::http::SgpClient;
use crate::clients::sgp::servers;
use crate::error::Result;
use crate::services::{auto_accept, draft, ongoing_game, overlay_window, panel_window};
use crate::state::gameflow::PHASE_NONE;
use crate::state::session::LcuSession;
use crate::state::{AppState, ClientInfo, ConnectionStatus};

const SCAN_INTERVAL: Duration = Duration::from_secs(2);
/// How long to wait for a freshly started client to answer API calls.
const READY_ATTEMPTS: u32 = 30;
const UNREADABLE_HINT: &str =
    "检测到英雄联盟客户端，但无法读取连接信息，需要以管理员身份运行 Lumina。";

const CURRENT_SUMMONER: &str = "/lol-summoner/v1/current-summoner";
const GAMEFLOW_PHASE: &str = "/lol-gameflow/v1/gameflow-phase";
const PLATFORM_ID: &str = "/lol-platform-config/v1/namespaces/LoginDataPacket/platformId";
const REGION_LOCALE: &str = "/riotclient/region-locale";

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
            Ok(Discovery::Unreadable) => state.mark_needs_admin(UNREADABLE_HINT),
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
            sgp_server: None,
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
    let locale: Result<RegionLocale> = http.get(REGION_LOCALE).await;
    let region = locale.map(|l| l.region).unwrap_or_default();
    let sgp = connect_sgp(&region, platform_id.as_deref());
    let sgp_server = sgp.as_ref().map(|c| c.server().server.name.clone());

    let session = LcuSession {
        http,
        sgp,
        game_data: OnceCell::new(),
    };
    state.set_session(Some(Arc::new(session)));

    state.update_lcu(|s| {
        s.status = ConnectionStatus::Connected;
        s.summoner = Some(summoner);
        s.last_error = None;
        if let Some(client) = &mut s.client {
            client.platform_id = platform_id;
            client.sgp_server = sgp_server;
        }
    });
    let phase = phase.unwrap_or_else(|_| PHASE_NONE.to_owned());
    state.set_gameflow_phase(phase.clone());
    auto_accept::on_phase(app, &phase);
    draft::on_phase(app, &phase);
    overlay_window::on_phase(app, &phase);
    ongoing_game::load_initial(app, &phase).await;
    log::info!("connected to LCU on port {}", creds.port);

    let router = event_router(app.clone());
    socket.run(|event| router.dispatch(&event)).await
}

/// SGP is optional: without a configured server everything falls back to LCU.
fn connect_sgp(region: &str, platform_id: Option<&str>) -> Option<SgpClient> {
    let Some(server) = platform_id.and_then(|p| servers::resolve(region, p)) else {
        log::warn!("no SGP server for region={region} platform={platform_id:?}, LCU only");
        return None;
    };
    log::info!("using SGP server {}", server.id);
    match SgpClient::new(server) {
        Ok(client) => Some(client),
        Err(err) => {
            log::warn!("failed to create SGP client: {err}");
            None
        }
    }
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
    let champ_select_app = app.clone();
    let gameflow_app = app.clone();
    let ready_check_app = app.clone();
    let lobby_app = app.clone();
    UriRouter::default()
        .on(GAMEFLOW_PHASE, move |event| {
            let phase = match event.event_type {
                LcuEventType::Delete => None,
                _ => event.data.as_str(),
            };
            let phase = phase.unwrap_or(PHASE_NONE).to_owned();
            ongoing_game::on_phase(&phase_app, &phase);
            auto_accept::on_phase(&phase_app, &phase);
            panel_window::on_phase(&phase_app, &phase);
            phase_app.state::<AppState>().set_gameflow_phase(phase.clone());
            // After the phase is stored: the overlays check it on every tick.
            draft::on_phase(&phase_app, &phase);
            overlay_window::on_phase(&phase_app, &phase);
        })
        .on(ongoing_game::CHAMP_SELECT_SESSION, move |event| {
            ongoing_game::on_champ_select(&champ_select_app, event);
            draft::on_champ_select(&champ_select_app, event);
        })
        .on(ongoing_game::GAMEFLOW_SESSION, move |event| {
            ongoing_game::on_gameflow_session(&gameflow_app, event);
        })
        .on(auto_accept::READY_CHECK, move |event| {
            auto_accept::on_ready_check(&ready_check_app, event);
        })
        .on(ongoing_game::LOBBY, move |event| {
            ongoing_game::on_lobby(&lobby_app, event);
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
