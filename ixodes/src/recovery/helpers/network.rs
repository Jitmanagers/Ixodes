use crate::recovery::task::RecoveryError;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio::process::Command;

#[derive(Serialize, Deserialize)]
pub struct NetworkTrafficStat {
    pub name: String,
    pub description: String,
    pub status: String,
    pub mac_address: String,
    pub link_speed: String,
    pub received_bytes: u64,
    pub sent_bytes: u64,
    pub received_packets: u64,
    pub sent_packets: u64,
}

pub async fn gather_network_traffic() -> Result<Vec<NetworkTrafficStat>, RecoveryError> {
    let script = r#"
        Get-NetAdapter | ForEach-Object {
            $stat = Get-NetAdapterStatistics -Name $_.Name
            [PSCustomObject]@{
                Name = $_.Name
                Description = $_.InterfaceDescription
                Status = $_.Status.ToString()
                MacAddress = $_.MacAddress
                LinkSpeed = $_.LinkSpeed
                ReceivedBytes = $stat.ReceivedBytes
                SentBytes = $stat.SentBytes
                ReceivedPackets = $stat.ReceivedPackets
                SentPackets = $stat.SentPackets
            }
        } | ConvertTo-Json
    "#;
    
    let value = capture_powershell_json(script).await?;
    Ok(parse_network_stats(value))
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

fn parse_network_stats(value: Value) -> Vec<NetworkTrafficStat> {
    let mut adapters = Vec::new();
    match value {
        Value::Array(items) => {
            for item in items {
                if let Ok(stat) = serde_json::from_value::<NetworkTrafficStat>(item) {
                    adapters.push(stat);
                }
            }
        }
        Value::Object(_) => {
            if let Ok(stat) = serde_json::from_value::<NetworkTrafficStat>(value) {
                adapters.push(stat);
            }
        }
        _ => {}
    }
    adapters
}
