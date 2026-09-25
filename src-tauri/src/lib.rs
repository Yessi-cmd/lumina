mod asset_proxy;
mod clients;
mod commands;
mod config;
mod error;
mod logging;
mod services;
mod state;

use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    logging::record_panics();

    tauri::Builder::default()
        .plugin(logging::plugin())
        .register_asynchronous_uri_scheme_protocol(asset_proxy::SCHEME, asset_proxy::handle)
        .setup(|app| {
            let version = env!("CARGO_PKG_VERSION");
            log::info!("Lumina {version} starting on {}", std::env::consts::OS);
            app.manage(state::AppState::new(app.handle().clone()));
            services::lcu_connection::spawn(app.handle().clone());
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::app_info,
            commands::lcu_snapshot,
            commands::relaunch_as_admin,
            commands::lookup_summoner,
            commands::summoner_by_puuid,
            commands::match_history,
            commands::game_data,
            commands::ongoing_roster,
            commands::player_profile,
            commands::roster_insights,
            commands::settings,
            commands::save_settings,
            commands::auto_accept_state,
            commands::cancel_auto_accept,
            commands::log_dir,
            commands::open_log_dir,
            commands::log_frontend
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
