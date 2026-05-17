mod commands;
mod db;
mod indexer;
mod models;

use commands::AppState;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_shell::init())
        .manage(AppState::new())
        .invoke_handler(tauri::generate_handler![
            commands::scan_library,
            commands::get_index_progress,
            commands::query_media,
            commands::get_media_detail,
            commands::get_people,
            commands::get_albums,
            commands::get_stats,
            commands::get_duplicates,
            commands::open_in_explorer,
            commands::delete_file,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
