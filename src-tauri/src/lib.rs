pub mod commands;
pub mod db;
pub mod lastfm;
pub mod local;
pub mod model;
pub mod radio;
pub mod spotify;
pub mod stream;

use commands::AppState;
use db::Db;
use std::sync::Arc;
use tauri::Manager;

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let db_dir = app
                .path()
                .app_data_dir()
                .expect("failed to resolve app data dir");
            std::fs::create_dir_all(&db_dir).ok();
            let db_path = db_dir.join("musicadena.db");
            let db = Arc::new(
                Db::open(db_path.to_str().unwrap_or("musicadena.db"))
                    .expect("failed to open database"),
            );
            let http = reqwest::Client::builder()
                .user_agent("Musicadena/0.1.0")
                .build()
                .expect("failed to build http client");
            let state = AppState::new(db, http);
            app.manage(state);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::search_all,
            commands::resolve_stream,
            commands::get_library,
            commands::scan_library,
            commands::get_playlists,
            commands::get_playlist,
            commands::create_playlist,
            commands::add_to_playlist,
            commands::remove_from_playlist,
            commands::delete_playlist,
            commands::get_playlist_tracks,
            commands::get_history,
            commands::clear_history,
            commands::radio_suggestions,
            commands::get_settings,
            commands::set_settings,
            commands::spotify_auth_url,
            commands::spotify_callback,
            commands::spotify_status,
            commands::record_playback,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
