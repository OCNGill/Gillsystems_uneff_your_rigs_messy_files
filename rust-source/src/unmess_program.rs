//! # App Core Module — Scan Orchestration
//!
//! This module contains [`UnmessSecretFunctions`] — the local program core.
//! It owns scan orchestration, duplicate indexing, export, and remediation.

use anyhow::{anyhow, bail, Context, Result};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::sync::{mpsc, RwLock};
use tracing::{error, info, warn};
use uuid::Uuid;

use crate::config::Config;
use crate::database::{Database, FileRow};
use crate::file_scanner::{FileScanner, ScanProgress, ScanStatus};
use crate::gui::{DriveInfo, GuiMessage};
use crate::platform;
use crate::remediation::{RemediationEngine, RemediationResult};
use crate::service::GrpcService;

const NODE_ID_SETTING_KEY: &str = "local_node_id";

fn app_version() -> &'static str {
    option_env!("APP_VERSION").unwrap_or(env!("CARGO_PKG_VERSION"))
}

#[derive(Debug, Clone)]
pub struct ScanState {
    pub scan_id: String,
    pub status: ScanStatus,
    pub files_processed: u64,
    pub bytes_processed: u64,
    pub duplicates_found: u64,
    pub current_path: String,
}

pub struct UnmessSecretFunctions {
    config: Arc<Config>,
    database: Arc<Database>,
    scanner: Arc<FileScanner>,
    remediation: Arc<RemediationEngine>,
    gui_tx: Option<mpsc::UnboundedSender<GuiMessage>>,
    scan_state: Arc<RwLock<Option<ScanState>>>,
    cancel_flag: Arc<AtomicBool>,
    node_id: String,
}

impl UnmessSecretFunctions {
    pub async fn new(
        config: Arc<Config>,
        gui_tx: Option<mpsc::UnboundedSender<GuiMessage>>,
        _progress_tx: Option<mpsc::Sender<String>>,
    ) -> Result<Self> {
        info!("Initializing Unmess app core");

        let database = Arc::new(
            Database::new(&config.database).context("Failed to initialize database")?,
        );
        let scanner = Arc::new(FileScanner::new(Arc::new(config.scanning.clone())));
        let remediation = Arc::new(RemediationEngine::new(
            resolve_quarantine_path(&config),
            config.remediation.grace_period_hours,
            config.remediation.verify_before_delete,
        ));

        let node_id = match database.get_setting(NODE_ID_SETTING_KEY)? {
            Some(existing) if !existing.trim().is_empty() => existing,
            _ => {
                let generated = Uuid::new_v4().to_string();
                database.set_setting(NODE_ID_SETTING_KEY, &generated)?;
                generated
            }
        };

        let hostname = hostname::get()
            .map(|h| h.to_string_lossy().to_string())
            .unwrap_or_else(|_| "unknown".to_string());
        let platform_name = std::env::consts::OS.to_string();
        let now = unix_timestamp();

        database.upsert_node(
            &node_id,
            &hostname,
            "127.0.0.1",
            &platform_name,
            app_version(),
            now,
        )?;

        info!(
            "Unmess app core initialized — node_id: {}, hostname: {}, version: {}",
            node_id, hostname, app_version()
        );

        Ok(Self {
            config,
            database,
            scanner,
            remediation,
            gui_tx,
            scan_state: Arc::new(RwLock::new(None)),
            cancel_flag: Arc::new(AtomicBool::new(false)),
            node_id,
        })
    }

    pub async fn run_service(&self) -> Result<()> {
        info!("Unmess service starting on port {}", self.config.grpc_port);

        let grpc = GrpcService::new(self.config.grpc_port);
        let grpc_handle = tokio::spawn(async move {
            if let Err(e) = grpc.start().await {
                error!("gRPC service failed: {}", e);
            }
        });

        tokio::signal::ctrl_c().await?;
        info!("Unmess service shutting down gracefully");
        grpc_handle.abort();
        Ok(())
    }

    pub async fn start_scan(&self, paths: Vec<PathBuf>) -> Result<String> {
        let normalized_paths = normalize_scan_paths(paths);
        if normalized_paths.is_empty() {
            bail!("No valid scan paths were provided");
        }

        let scan_id = Uuid::new_v4().to_string();
        let now = unix_timestamp();
        self.cancel_flag.store(false, Ordering::Relaxed);
        self.database.create_scan(&scan_id, &self.node_id, "user", now)?;

        {
            let mut state = self.scan_state.write().await;
            *state = Some(ScanState {
                scan_id: scan_id.clone(),
                status: ScanStatus::Scanning,
                files_processed: 0,
                bytes_processed: 0,
                duplicates_found: 0,
                current_path: String::new(),
            });
        }

        info!(
            "Scan {} started — {} normalized roots queued",
            scan_id,
            normalized_paths.len()
        );

        let (progress_tx, mut progress_rx) = mpsc::channel::<ScanProgress>(100);
        let scanner = self.scanner.clone();
        let cancel = self.cancel_flag.clone();
        let scan_paths = normalized_paths.clone();

        let scanner_handle = tokio::spawn(async move {
            scanner.scan_paths(&scan_paths, progress_tx, cancel).await
        });

        let gui_tx = self.gui_tx.clone();
        let scan_state = self.scan_state.clone();
        let progress_handle = tokio::spawn(async move {
            while let Some(progress) = progress_rx.recv().await {
                {
                    let mut state = scan_state.write().await;
                    if let Some(ref mut s) = *state {
                        s.files_processed = progress.files_processed;
                        s.bytes_processed = progress.bytes_processed;
                        s.duplicates_found = progress.duplicates_found;
                        s.current_path = progress.current_path.clone();
                        s.status = progress.status.clone();
                    }
                }

                if let Some(ref tx) = gui_tx {
                    let _ = tx.send(GuiMessage::ScanProgress(progress));
                }
            }
        });

        let scanned_files = scanner_handle
            .await
            .map_err(|e| anyhow!("Scanner task panic: {}", e))?
            .map_err(|e| anyhow!("Scanner failed: {}", e))?;
        let _ = progress_handle.await;

        if !scanned_files.is_empty() {
            let file_rows: Vec<FileRow> = scanned_files
                .iter()
                .map(|scanned_file| {
                    let modified = scanned_file
                        .info
                        .modified_time
                        .duration_since(UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_secs() as i64;

                    let file_name = scanned_file
                        .info
                        .path
                        .file_name()
                        .map(|name| name.to_string_lossy().to_string())
                        .unwrap_or_default();

                    FileRow {
                        id: None,
                        node_id: self.node_id.clone(),
                        scan_id: scan_id.clone(),
                        file_path: scanned_file.info.path.to_string_lossy().to_string(),
                        file_name,
                        size_bytes: scanned_file.info.size as i64,
                        modified_time: modified,
                        xxhash64: scanned_file.xxhash64.clone(),
                        sha256_hash: scanned_file.sha256_hash.clone(),
                        remediation_status: "none".to_string(),
                    }
                })
                .collect();

            info!("Persisting {} scanned files", file_rows.len());
            self.database.insert_files_batch(&file_rows)?;
        }

        let duplicates_found = self.rebuild_duplicates_for_scan(&scan_id).unwrap_or_else(|error| {
            warn!("Duplicate rebuild failed for scan {}: {}", scan_id, error);
            0
        });

        let final_status = if self.cancel_flag.load(Ordering::Relaxed) {
            ScanStatus::Cancelled
        } else {
            ScanStatus::Completed
        };
        let final_status_name = match final_status {
            ScanStatus::Cancelled => "cancelled",
            _ => "completed",
        };

        {
            let mut state = self.scan_state.write().await;
            if let Some(ref mut s) = *state {
                s.duplicates_found = duplicates_found;
                s.status = final_status.clone();
            }
        }

        let completed_at = unix_timestamp();
        self.database.finish_scan(
            &scan_id,
            final_status_name,
            completed_at,
            scanned_files.len() as i64,
            scanned_files.iter().map(|file| file.info.size as i64).sum(),
            0,
        )?;

        info!(
            "Scan {} finished — {} files, {} duplicate groups, status {}",
            scan_id,
            scanned_files.len(),
            duplicates_found,
            final_status_name
        );

        Ok(scan_id)
    }

    pub async fn stop_scan(&self) -> Result<()> {
        self.cancel_flag.store(true, Ordering::Relaxed);
        let mut state = self.scan_state.write().await;
        if let Some(ref mut s) = *state {
            s.status = ScanStatus::Cancelled;
            info!("Scan {} cancellation requested", s.scan_id);
        }
        Ok(())
    }

    pub async fn get_scan_state(&self) -> Option<ScanState> {
        self.scan_state.read().await.clone()
    }

    pub fn get_local_drives(&self) -> Result<Vec<DriveInfo>> {
        let mut drives = Vec::new();

        #[cfg(windows)]
        {
            if let Ok(win_drives) = crate::platform::windows::get_all_drives() {
                for (index, drive) in win_drives.iter().enumerate() {
                    drives.push(DriveInfo {
                        id: format!("drive_{}", index),
                        label: drive.label.clone(),
                        mount_point: drive.mount_point.clone(),
                        drive_type: drive.drive_type.clone(),
                        total_space: drive.total_space,
                        available_space: drive.available_space,
                        is_removable: drive.is_removable,
                    });
                }
            }
        }

        #[cfg(unix)]
        {
            if let Ok(unix_mounts) = crate::platform::unix::get_all_mounts() {
                for (index, mount) in unix_mounts.iter().enumerate() {
                    drives.push(DriveInfo {
                        id: format!("mount_{}", index),
                        label: mount.device.clone(),
                        mount_point: mount.mount_point.clone(),
                        drive_type: mount.fs_type.clone(),
                        total_space: mount.total_space,
                        available_space: mount.available_space,
                        is_removable: false,
                    });
                }
            }
        }

        Ok(drives)
    }

    pub fn node_id(&self) -> &str {
        &self.node_id
    }

    pub fn database(&self) -> &Arc<Database> {
        &self.database
    }

    pub fn remediation(&self) -> &Arc<RemediationEngine> {
        &self.remediation
    }

    pub fn latest_scan_id(&self) -> Result<Option<String>> {
        self.database.get_latest_completed_scan_id()
    }

    pub fn quarantine_file(&self, group_id: Option<i64>, file_id: i64) -> Result<String> {
        let file = self.load_file(file_id)?;
        let result = self
            .remediation
            .quarantine(Path::new(&file.file_path))
            .with_context(|| format!("Failed to quarantine {}", file.file_path))?;
        self.finish_file_remediation(group_id, &file, result, "quarantined", true)
    }

    pub fn delete_file(&self, group_id: Option<i64>, file_id: i64) -> Result<String> {
        let file = self.load_file(file_id)?;
        let result = self
            .remediation
            .delete(Path::new(&file.file_path), file.sha256_hash.as_deref())
            .with_context(|| format!("Failed to delete {}", file.file_path))?;
        self.finish_file_remediation(group_id, &file, result, "deleted", true)
    }

    pub fn dedup_file(&self, group_id: i64, primary_file_id: i64, duplicate_file_id: i64) -> Result<String> {
        if primary_file_id == duplicate_file_id {
            bail!("Primary and duplicate file selections must be different");
        }

        let primary = self.load_file(primary_file_id)?;
        let duplicate = self.load_file(duplicate_file_id)?;
        if primary.scan_id != duplicate.scan_id {
            bail!("Cannot deduplicate files from different scans");
        }

        let fs_type = platform::detect_fs_type(&duplicate.file_path);
        let result = self
            .remediation
            .dedup_or_hard_link(
                Path::new(&primary.file_path),
                Path::new(&duplicate.file_path),
                &fs_type,
            )
            .with_context(|| {
                format!(
                    "Failed to deduplicate {} against {}",
                    duplicate.file_path, primary.file_path
                )
            })?;

        self.finish_file_remediation(Some(group_id), &duplicate, result, "deduplicated", false)
    }

    pub fn export_scan_log(&self, scan_id: Option<&str>) -> Result<()> {
        use chrono::Local;

        let target_scan_id = match scan_id {
            Some(id) => id.to_string(),
            None => self
                .database
                .get_latest_completed_scan_id()?
                .context("No completed scan is available to export")?,
        };

        let now = Local::now();
        let timestamp = now.format("%Y%m%d_%H%M%S").to_string();
        let display_time = now.format("%Y-%m-%d %H:%M:%S").to_string();
        let log_dir = PathBuf::from("scan_logs");
        std::fs::create_dir_all(&log_dir)?;

        let groups = self.database.get_duplicate_groups_for_scan(&target_scan_id)?;
        let total_wasted = self.database.get_total_wasted_space_for_scan(&target_scan_id)?;
        let total_files_in_groups: i64 = groups.iter().map(|group| group.file_count).sum();

        let mut group_details = Vec::with_capacity(groups.len());
        for group in &groups {
            let files = self.database.get_duplicate_files_for_group(group.id)?;
            group_details.push((group.clone(), files));
        }

        let mut markdown = String::new();
        markdown.push_str(&format!("# Unmess Scan Report — {}\n\n", display_time));
        markdown.push_str("## Summary\n\n| Metric | Value |\n|---|---|\n");
        markdown.push_str(&format!("| Scan ID | `{}` |\n", target_scan_id));
        markdown.push_str(&format!("| Duplicate Groups | {} |\n", groups.len()));
        markdown.push_str(&format!("| Files in Groups | {} |\n", total_files_in_groups));
        markdown.push_str(&format!(
            "| Total Wasted Space | {:.2} GB |\n",
            total_wasted as f64 / 1_073_741_824.0
        ));
        markdown.push_str(&format!("| Node ID | `{}` |\n\n", self.node_id));
        markdown.push_str("## Duplicate Groups\n");
        for (index, (group, files)) in group_details.iter().enumerate() {
            let wasted_mb = group.total_wasted_bytes / 1_048_576;
            markdown.push_str(&format!("\n### Group {} — {} MB wasted\n", index + 1, wasted_mb));
            markdown.push_str(&format!("- **Hash**: `{}`\n", group.sha256_hash));
            markdown.push_str(&format!("- **Size per file**: {} bytes\n", group.size_bytes));
            markdown.push_str("- **Files**:\n");
            for file in files {
                markdown.push_str(&format!("  - `{}`\n", file.file_path));
            }
        }
        let markdown_path = log_dir.join(format!("scan_{}.md", timestamp));
        std::fs::write(&markdown_path, markdown)?;

        let json_groups: Vec<serde_json::Value> = group_details
            .iter()
            .map(|(group, files)| {
                serde_json::json!({
                    "group_id": group.id,
                    "scan_id": group.scan_id,
                    "sha256": group.sha256_hash,
                    "size_bytes": group.size_bytes,
                    "file_count": group.file_count,
                    "wasted_bytes": group.total_wasted_bytes,
                    "files": files.iter().map(|file| serde_json::json!({
                        "path": file.file_path,
                        "modified_time": file.modified_time,
                    })).collect::<Vec<_>>(),
                })
            })
            .collect();

        let report = serde_json::json!({
            "scan_date": display_time,
            "scan_id": target_scan_id,
            "node_id": self.node_id,
            "duplicate_groups": groups.len(),
            "total_files_in_groups": total_files_in_groups,
            "total_wasted_bytes": total_wasted,
            "groups": json_groups,
        });
        let json_path = log_dir.join(format!("scan_{}.json", timestamp));
        std::fs::write(&json_path, serde_json::to_string_pretty(&report)?)?;

        info!(
            "Scan log exported for {}: {} and {}",
            target_scan_id,
            markdown_path.display(),
            json_path.display()
        );
        Ok(())
    }

    fn rebuild_duplicates_for_scan(&self, scan_id: &str) -> Result<u64> {
        self.database.clear_duplicate_results_for_scan(scan_id)?;
        let matches = self.database.find_sha256_matches_for_scan(scan_id, 2)?;
        let mut groups_found = 0u64;

        for (sha256, size, _count) in matches {
            let files = self.database.get_files_by_hash_for_scan(scan_id, &sha256)?;
            let file_ids: Vec<i64> = files.iter().filter_map(|file| file.id).collect();
            if file_ids.len() < 2 {
                continue;
            }

            let primary_file_id = file_ids[0];
            let group_id = self
                .database
                .upsert_duplicate_group(scan_id, &sha256, size, file_ids.len() as i64)?;
            self.database
                .replace_duplicate_group_files(group_id, &file_ids, primary_file_id)?;
            groups_found += 1;
        }

        if groups_found > 0 {
            let wasted = self.database.get_total_wasted_space_for_scan(scan_id)?;
            info!(
                "Duplicate rebuild complete for scan {} — {} groups, {} bytes wasted",
                scan_id, groups_found, wasted
            );
        } else {
            info!("No duplicate groups found for scan {}", scan_id);
        }

        Ok(groups_found)
    }

    fn finish_file_remediation(
        &self,
        group_id: Option<i64>,
        file: &FileRow,
        result: RemediationResult,
        final_status: &str,
        mark_deleted: bool,
    ) -> Result<String> {
        if !result.success {
            bail!(result.error.unwrap_or_else(|| "Remediation failed".to_string()));
        }

        let file_id = file.id.context("Missing file ID for remediation target")?;
        self.database
            .set_file_remediation_status(file_id, final_status, mark_deleted)?;

        let fs_type = if result.fs_type.is_empty() {
            platform::detect_fs_type(&file.file_path)
        } else {
            result.fs_type.clone()
        };
        self.database.log_remediation(
            group_id,
            &result.action,
            &file.file_path,
            result.source_path.as_deref(),
            &self.node_id,
            result.space_recovered as i64,
            &fs_type,
            &result.strategy,
        )?;
        self.database.log_audit(
            &format!("remediation:{}", result.action),
            "file",
            &file_id.to_string(),
            &format!("{} via {}", final_status, result.strategy),
            &self.node_id,
        )?;
        let _ = self.rebuild_duplicates_for_scan(&file.scan_id);

        let message = match result.action.as_str() {
            "quarantine" => format!(
                "Quarantined {}{}",
                file.file_path,
                result
                    .source_path
                    .as_deref()
                    .map(|path| format!(" to {}", path))
                    .unwrap_or_default()
            ),
            "delete" => format!("Deleted {}", file.file_path),
            "dedup" => format!(
                "Deduplicated {} against {}",
                file.file_path,
                result.source_path.as_deref().unwrap_or("the selected primary file")
            ),
            _ => format!("Completed {} for {}", result.action, file.file_path),
        };

        Ok(message)
    }

    fn load_file(&self, file_id: i64) -> Result<FileRow> {
        self.database
            .get_file_by_id(file_id)?
            .with_context(|| format!("File {} was not found in the database", file_id))
    }
}

fn resolve_quarantine_path(config: &Config) -> PathBuf {
    let configured = PathBuf::from(&config.remediation.quarantine_path);
    if configured.is_absolute() {
        return configured;
    }

    let database_path = PathBuf::from(&config.database.path);
    let base_dir = database_path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from("."));
    base_dir.join(configured)
}

fn normalize_scan_paths(paths: Vec<PathBuf>) -> Vec<PathBuf> {
    let mut canonical_paths: Vec<PathBuf> = paths
        .into_iter()
        .filter(|path| !path.as_os_str().is_empty())
        .map(|path| std::fs::canonicalize(&path).unwrap_or(path))
        .collect();
    canonical_paths.sort_by_key(|path| path.components().count());

    let mut normalized = Vec::new();
    for candidate in canonical_paths {
        if normalized.iter().any(|existing| candidate == *existing || candidate.starts_with(existing)) {
            continue;
        }
        normalized.push(candidate);
    }
    normalized
}

fn unix_timestamp() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64
}
