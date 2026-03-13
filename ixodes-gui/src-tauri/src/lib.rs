mod builder;
mod commands;
mod error;
mod models;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            commands::list_settings_files,
            commands::build_ixodes,
            commands::test_telegram_connection,
            commands::test_discord_connection
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
