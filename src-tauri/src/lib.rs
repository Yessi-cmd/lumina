mod asset_proxy;
mod clients;
mod commands;
mod error;
mod services;
mod state;

use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let log_plugin = tauri_plugin_log::Builder::new()
        .level(log::LevelFilter::Info)
        .build();

    tauri::Builder::default()
        .plugin(log_plugin)
        .register_asynchronous_uri_scheme_protocol(asset_proxy::SCHEME, asset_proxy::handle)
        .setup(|app| {
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
            commands::roster_insights
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
