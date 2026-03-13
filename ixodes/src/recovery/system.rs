use crate::recovery::{
    context::RecoveryContext,
    helpers::network::gather_network_traffic,
    output::write_json_artifact,
    registry::format_reg_value,
    task::{RecoveryArtifact, RecoveryCategory, RecoveryError, RecoveryTask},
};
use async_trait::async_trait;
use directories::BaseDirs;
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::{fs, process::Command, task};
use tracing::{debug, warn};
use winreg::{
    HKEY, RegKey,
    enums::{HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE},
};

pub fn system_tasks(_ctx: &RecoveryContext) -> Vec<Arc<dyn RecoveryTask>> {
    vec![
        Arc::new(SystemInfoTask),
        Arc::new(StartupProgramsTask),
        Arc::new(SoftwareInventoryTask),
        Arc::new(SystemUpdatesTask),
        Arc::new(NetworkTrafficTask),
    ]
}

struct SystemInfoTask;

#[async_trait]
impl RecoveryTask for SystemInfoTask {
    fn label(&self) -> String {
        "System Inventory".to_string()
    }

    fn category(&self) -> RecoveryCategory {
        RecoveryCategory::System
    }

    async fn run(&self, ctx: &RecoveryContext) -> Result<Vec<RecoveryArtifact>, RecoveryError> {
        let properties = match capture_powershell_json(
            r#"$os = Get-CimInstance Win32_OperatingSystem; $cs = Get-CimInstance Win32_ComputerSystem; $bios = Get-CimInstance Win32_BIOS; $cpu = Get-CimInstance Win32_Processor | Select-Object -First 1; [PSCustomObject]@{ "Host Name"=$cs.Name; "OS Name"=$os.Caption; "OS Version"=$os.Version; "Manufacturer"=$cs.Manufacturer; "Model"=$cs.Model; "Processor"=$cpu.Name; "BIOS Version"=$bios.Version; "Total Memory"=$([math]::Round($cs.TotalPhysicalMemory/1GB,2).ToString()+' GB'); "Install Date"=$os.InstallDate.ToString(); "Boot Time"=$os.LastBootUpTime.ToString(); "Time Zone"=(Get-TimeZone).DisplayName; "Domain"=$cs.Domain } | ConvertTo-Json"#,
        )
        .await
        {
            Ok(value) => parse_system_properties_json(value),
            Err(err) => {
                warn!(error = ?err, "PowerShell system properties query failed");
                Vec::new()
            }
        };

        let disk_stats = match capture_powershell_json(
            "Get-CimInstance Win32_LogicalDisk | Select-Object DeviceID,Size,FreeSpace | ConvertTo-Json",
        )
        .await
        {
            Ok(value) => parse_disk_stats_json(value),
            Err(err) => {
                warn!(error = ?err, "PowerShell disk query failed");
                Vec::new()
            }
        };

        let network_configuration = match capture_command_output("ipconfig", &["/all"]).await {
            Ok(output) => output,
            Err(err) => {
                warn!(error = ?err, "ipconfig command failed");
                String::new()
            }
        };

        let summary = SystemSnapshot {
            properties,
            disk_stats,
            network_configuration,
        };

        let artifact = write_json_artifact(
            ctx,
            self.category(),
            &self.label(),
            "system-inventory.json",
            &summary,
        )
        .await?;

        Ok(artifact.into_iter().collect())
    }
}

#[derive(Deserialize)]
struct RawDiskStats {
    #[serde(rename = "DeviceID")]
    device_id: String,
    #[serde(rename = "Size")]
    size: Option<u64>,
    #[serde(rename = "FreeSpace")]
    free_space: Option<u64>,
}

fn parse_disk_stats_json(value: Value) -> Vec<DiskStats> {
    let mut stats = Vec::new();
    let items = match value {
        Value::Array(arr) => arr,
        Value::Object(_) => vec![value],
        _ => return stats,
    };

    for item in items {
        if let Ok(raw) = serde_json::from_value::<RawDiskStats>(item) {
            stats.push(DiskStats {
                name: raw.device_id,
                size_bytes: raw.size,
                free_bytes: raw.free_space,
            });
        }
    }
    stats
}

#[derive(Serialize)]
struct SystemSnapshot {
    properties: Vec<SystemProperty>,
    disk_stats: Vec<DiskStats>,
    network_configuration: String,
}

#[derive(Serialize, Deserialize)]
struct SystemProperty {
    key: String,
    value: String,
}

#[derive(Serialize)]
struct DiskStats {
    name: String,
    size_bytes: Option<u64>,
    free_bytes: Option<u64>,
}

fn parse_system_properties_json(value: Value) -> Vec<SystemProperty> {
    let mut props = Vec::new();
    if let Value::Object(map) = value {
        for (k, v) in map {
            props.push(SystemProperty {
                key: k,
                value: match v {
                    Value::String(s) => s,
                    Value::Number(n) => n.to_string(),
                    Value::Bool(b) => b.to_string(),
                    _ => v.to_string(),
                },
            });
        }
    }
    props
}

async fn capture_command_output(cmd: &str, args: &[&str]) -> Result<String, RecoveryError> {
    let output = Command::new(cmd).args(args).output().await?;
    if !output.status.success() {
        return Err(RecoveryError::Custom(format!(
            "command `{cmd}` failed with code {:?}",
            output.status.code()
        )));
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

struct StartupProgramsTask;

#[async_trait]
impl RecoveryTask for StartupProgramsTask {
    fn label(&self) -> String {
        "Startup Programs".to_string()
    }

    fn category(&self) -> RecoveryCategory {
        RecoveryCategory::System
    }

    async fn run(&self, ctx: &RecoveryContext) -> Result<Vec<RecoveryArtifact>, RecoveryError> {
        let registry_entries = collect_registry_entries().await?;
        let startup_directories = gather_startup_directories().await;

        let summary = StartupProgramsSummary {
            registry_entries,
            startup_directories,
        };

        let artifact = write_json_artifact(
            ctx,
            self.category(),
            &self.label(),
            "startup-programs.json",
            &summary,
        )
        .await?;

        Ok(artifact.into_iter().collect())
    }
}

#[derive(Serialize)]
struct StartupProgramsSummary {
    registry_entries: Vec<RegistryStartupEntry>,
    startup_directories: Vec<StartupDirectory>,
}

#[derive(Serialize)]
struct RegistryStartupEntry {
    root: String,
    key: String,
    name: String,
    value: String,
}

#[derive(Serialize)]
struct StartupDirectory {
    label: String,
    path: String,
    entries: Vec<String>,
}

impl StartupDirectory {
    async fn describe(label: &str, path: PathBuf) -> Self {
        let entries = list_directory_entries(&path).await;
        Self {
            label: label.to_string(),
            path: path.display().to_string(),
            entries,
        }
    }
}

static REGISTRY_PATHS: Lazy<Vec<String>> = Lazy::new(|| {
    vec![
        r"Software\Microsoft\Windows\CurrentVersion\Run".to_string(),
        r"Software\Microsoft\Windows\CurrentVersion\RunOnce".to_string(),
        r"Software\Microsoft\Windows\CurrentVersion\RunServices".to_string(),
        r"Software\Microsoft\Windows\CurrentVersion\Policies\Explorer\Run".to_string(),
        r"Software\WOW6432Node\Microsoft\Windows\CurrentVersion\Run".to_string(),
    ]
});

const REGISTRY_ROOTS: &[(HKEY, &str)] = &[
    (HKEY_CURRENT_USER, "HKEY_CURRENT_USER"),
    (HKEY_LOCAL_MACHINE, "HKEY_LOCAL_MACHINE"),
];

async fn collect_registry_entries() -> Result<Vec<RegistryStartupEntry>, RecoveryError> {
    let entries = task::spawn_blocking(|| collect_registry_entries_blocking())
        .await
        .map_err(|err| RecoveryError::Custom(format!("registry scan interrupted: {err}")))?;
    Ok(entries)
}

fn collect_registry_entries_blocking() -> Vec<RegistryStartupEntry> {
    let mut entries = Vec::new();
    for &(hkey, root_name) in REGISTRY_ROOTS {
        let root = RegKey::predef(hkey);

        for path in REGISTRY_PATHS.iter() {
            match root.open_subkey(path) {
                Ok(key) => {
                    for value_result in key.enum_values() {
                        if let Ok((name, value)) = value_result {
                            entries.push(RegistryStartupEntry {
                                root: root_name.to_string(),
                                key: path.to_string(),
                                name,
                                value: format_reg_value(&value),
                            });
                        }
                    }
                }
                Err(err) => {
                    debug!(root = root_name, path, error = ?err, "startup registry key unavailable");
                }
            }
        }
    }

    entries
}

async fn gather_startup_directories() -> Vec<StartupDirectory> {
    let mut directories = Vec::new();

    if let Some(base) = BaseDirs::new() {
        let user_path = base
            .data_dir()
            .join("Microsoft")
            .join("Windows")
            .join("Start Menu")
            .join("Programs")
            .join("Startup");
        directories.push(StartupDirectory::describe("User Startup", user_path).await);
    }

    if let Ok(program_data) = std::env::var("PROGRAMDATA") {
        let common_path = PathBuf::from(program_data)
            .join("Microsoft")
            .join("Windows")
            .join("Start Menu")
            .join("Programs")
            .join("Startup");
        directories.push(StartupDirectory::describe("Common Startup", common_path).await);
    }

    directories
}

async fn list_directory_entries(path: &Path) -> Vec<String> {
    let mut entries = Vec::new();
    match fs::read_dir(path).await {
        Ok(mut dir) => loop {
            match dir.next_entry().await {
                Ok(Some(entry)) => entries.push(entry.path().display().to_string()),
                Ok(None) => break,
                Err(err) => {
                    warn!(path = ?path, error = ?err, "failed to list startup folder");
                    break;
                }
            }
        },
        Err(err) => {
            warn!(path = ?path, error = ?err, "startup directory not readable");
        }
    }
    entries
}

struct SoftwareInventoryTask;

#[async_trait]
impl RecoveryTask for SoftwareInventoryTask {
    fn label(&self) -> String {
        "Installed Software".to_string()
    }

    fn category(&self) -> RecoveryCategory {
        RecoveryCategory::System
    }

    async fn run(&self, ctx: &RecoveryContext) -> Result<Vec<RecoveryArtifact>, RecoveryError> {
        let software = task::spawn_blocking(|| collect_installed_software())
            .await
            .map_err(|err| RecoveryError::Custom(format!("software scan interrupted: {err}")))?;

        let artifact = write_json_artifact(
            ctx,
            self.category(),
            &self.label(),
            "installed-software.json",
            &SoftwareInventorySummary { software },
        )
        .await?;

        Ok(artifact.into_iter().collect())
    }
}

#[derive(Serialize)]
struct SoftwareInventorySummary {
    software: Vec<SoftwareRecord>,
}

#[derive(Serialize)]
struct SoftwareRecord {
    name: String,
    version: Option<String>,
    publisher: Option<String>,
    install_date: Option<String>,
    install_location: Option<String>,
    source: String,
}

struct SystemUpdatesTask;

#[async_trait]
impl RecoveryTask for SystemUpdatesTask {
    fn label(&self) -> String {
        "System Updates".to_string()
    }

    fn category(&self) -> RecoveryCategory {
        RecoveryCategory::System
    }

    async fn run(&self, ctx: &RecoveryContext) -> Result<Vec<RecoveryArtifact>, RecoveryError> {
        let summary = match capture_powershell_json(
            "Get-HotFix | Select-Object Caption,Description,HotFixID,InstalledOn,CSName | ConvertTo-Json -Depth 2",
        )
        .await
        {
            Ok(value) => QuickFixSummary {
                updates: parse_quick_fix_json(value),
            },
            Err(err) => {
                warn!(error = ?err, "PowerShell Get-HotFix query failed");
                match capture_command_output("wmic", &["qfe", "list", "/format:list"]).await {
                    Ok(output) => QuickFixSummary {
                        updates: parse_quick_fix_output(&output),
                    },
                    Err(err) => {
                        warn!(error = ?err, "wmic qfe query failed");
                        QuickFixSummary {
                            updates: Vec::new(),
                        }
                    }
                }
            }
        };

        let artifact = write_json_artifact(
            ctx,
            self.category(),
            &self.label(),
            "system-updates.json",
            &summary,
        )
        .await?;

        Ok(artifact.into_iter().collect())
    }
}

#[derive(Serialize)]
struct QuickFixSummary {
    updates: Vec<QuickFixRecord>,
}

#[derive(Serialize)]
struct QuickFixRecord {
    caption: Option<String>,
    description: Option<String>,
    hotfix_id: Option<String>,
    installed_on: Option<String>,
    cs_name: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct RawQuickFixRecord {
    caption: Option<String>,
    description: Option<String>,
    #[serde(rename = "HotFixID")]
    hotfix_id: Option<String>,
    installed_on: Option<String>,
    #[serde(rename = "CSName")]
    cs_name: Option<String>,
}

struct NetworkTrafficTask;

#[async_trait]
impl RecoveryTask for NetworkTrafficTask {
    fn label(&self) -> String {
        "Network Traffic".to_string()
    }

    fn category(&self) -> RecoveryCategory {
        RecoveryCategory::System
    }

    async fn run(&self, ctx: &RecoveryContext) -> Result<Vec<RecoveryArtifact>, RecoveryError> {
        let adapters = gather_network_traffic().await.unwrap_or_default();

        let artifact = write_json_artifact(
            ctx,
            self.category(),
            &self.label(),
            "network-traffic.json",
            &NetworkTrafficSummary { adapters },
        )
        .await?;

        Ok(artifact.into_iter().collect())
    }
}

#[derive(Serialize)]
struct NetworkTrafficSummary {
    adapters: Vec<crate::recovery::helpers::network::NetworkTrafficStat>,
}

async fn capture_powershell_json(script: &str) -> Result<Value, RecoveryError> {
    let output = Command::new("powershell")
        .arg("-NoProfile")
        .arg("-Command")
        .arg(script)
        .output()
        .await?;

    if !output.status.success() {
        return Err(RecoveryError::Custom(format!(
            "PowerShell command failed with code {:?}",
            output.status.code()
        )));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    if stdout.trim().is_empty() {
        return Ok(Value::Null);
    }

    serde_json::from_str(stdout.trim())
        .map_err(|err| RecoveryError::Custom(format!("PowerShell JSON parse failed: {err}")))
}

fn parse_quick_fix_json(value: Value) -> Vec<QuickFixRecord> {
    let mut updates = Vec::new();
    match value {
        Value::Array(items) => {
            for item in items {
                if let Ok(raw) = serde_json::from_value::<RawQuickFixRecord>(item) {
                    updates.push(QuickFixRecord {
                        caption: raw.caption,
                        description: raw.description,
                        hotfix_id: raw.hotfix_id,
                        installed_on: raw.installed_on,
                        cs_name: raw.cs_name,
                    });
                }
            }
        }
        Value::Object(_) => {
            if let Ok(raw) = serde_json::from_value::<RawQuickFixRecord>(value) {
                updates.push(QuickFixRecord {
                    caption: raw.caption,
                    description: raw.description,
                    hotfix_id: raw.hotfix_id,
                    installed_on: raw.installed_on,
                    cs_name: raw.cs_name,
                });
            }
        }
        _ => {}
    }
    updates
}

fn parse_quick_fix_output(output: &str) -> Vec<QuickFixRecord> {
    let mut updates = Vec::new();
    let mut current = HashMap::new();

    for line in output.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            if !current.is_empty() {
                updates.push(record_from_map(&current));
                current.clear();
            }
            continue;
        }

        if let Some((key, value)) = trimmed.split_once('=') {
            current.insert(key.to_string(), value.to_string());
        }
    }

    if !current.is_empty() {
        updates.push(record_from_map(&current));
    }

    updates
}

fn record_from_map(map: &HashMap<String, String>) -> QuickFixRecord {
    QuickFixRecord {
        caption: map.get("Caption").cloned(),
        description: map.get("Description").cloned(),
        hotfix_id: map.get("HotFixID").cloned(),
        installed_on: map.get("InstalledOn").cloned(),
        cs_name: map.get("CSName").cloned(),
    }
}

fn collect_installed_software() -> Vec<SoftwareRecord> {
    const SOFTWARE_LOCATIONS: &[InstallLocation] = &[
        InstallLocation {
            root: HKEY_LOCAL_MACHINE,
            path: r"SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall",
        },
        InstallLocation {
            root: HKEY_LOCAL_MACHINE,
            path: r"SOFTWARE\WOW6432Node\Microsoft\Windows\CurrentVersion\Uninstall",
        },
        InstallLocation {
            root: HKEY_CURRENT_USER,
            path: r"SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall",
        },
    ];

    let mut records = Vec::new();

    for location in SOFTWARE_LOCATIONS {
        let root = RegKey::predef(location.root);
        if let Ok(key) = root.open_subkey(location.path) {
            for subkey in key.enum_keys().filter_map(Result::ok) {
                if let Ok(entry) = key.open_subkey(&subkey) {
                    if let Some(name) = read_string_value(&entry, "DisplayName") {
                        records.push(SoftwareRecord {
                            name,
                            version: read_string_value(&entry, "DisplayVersion"),
                            publisher: read_string_value(&entry, "Publisher"),
                            install_date: read_string_value(&entry, "InstallDate"),
                            install_location: read_string_value(&entry, "InstallLocation"),
                            source: format!(r"{}\{}", location.path, subkey),
                        });
                    }
                }
            }
        }
    }

    records
}

fn read_string_value(key: &RegKey, name: &str) -> Option<String> {
    key.get_raw_value(name).ok().map(|v| {
        let text = format_reg_value(&v);
        text.trim().trim_matches('"').to_string()
    })
}

struct InstallLocation {
    root: HKEY,
    path: &'static str,
}
