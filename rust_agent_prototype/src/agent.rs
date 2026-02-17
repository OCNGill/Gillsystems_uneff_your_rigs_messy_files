// Gillsystems_uneff_your_rigs_messy_files — Agent Core Module
// Philosophy: Systems Should Serve Humans — Power to the People!
//
// This module contains the UneffAgent struct — the heart of the application.
// It owns the scanning pipeline, database, network service, and remediation engine.
// Single binary, single responsibility: un-eff your rigs.

use anyhow::{Context, Result};
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::sync::{mpsc, RwLock};
use tracing::{info, warn, error};
use uuid::Uuid;

use crate::config::Config;
use crate::database::{Database, FileRow};
use crate::file_scanner::{FileScanner, ScanProgress, ScanStatus};
use crate::gui::{GuiMessage, DriveInfo};
use crate::remediation::RemediationEngine;
use crate::service::GrpcService;

/// Scan state visible to GUI and service layer.
#[derive(Debug, Clone)]
pub struct ScanState {
    pub scan_id: String,
    pub status: ScanStatus,
    pub files_processed: u64,
    pub bytes_processed: u64,
    pub duplicates_found: u64,
    pub current_path: String,
}

/// The core agent that orchestrates all subsystems.
/// Single binary, single responsibility: un-eff your rigs.
pub struct UneffAgent {
    config: Arc<Config>,
    database: Arc<Database>,
    scanner: Arc<FileScanner>,
    remediation: Arc<RemediationEngine>,
    grpc_service: Option<GrpcService>,
    gui_tx: Option<mpsc::UnboundedSender<GuiMessage>>,
    scan_state: Arc<RwLock<Option<ScanState>>>,
    cancel_flag: Arc<AtomicBool>,
    node_id: String,
}

impl UneffAgent {
    /// Create a new agent instance.
    /// Full admin assumed — no permission checks, no gatekeeping.
    pub async fn new(
        config: Arc<Config>,
        gui_tx: Option<mpsc::UnboundedSender<GuiMessage>>,
        _progress_tx: Option<mpsc::Sender<String>>,
    ) -> Result<Self> {
        info!("Initializing UneffAgent — Systems Should Serve Humans");

        // Initialize database
        let database = Arc::new(
            Database::new(&config.database)
                .context("Failed to initialize database")?
        );

        // Initialize scanner
        let scanner = Arc::new(FileScanner::new(Arc::new(config.scanning.clone())));

        // Initialize remediation engine
        let quarantine_path = PathBuf::from(&config.database.path)
            .parent()
            .unwrap_or_else(|| std::path::Path::new("."))
            .join("quarantine");
        std::fs::create_dir_all(&quarantine_path).ok();

        let remediation = Arc::new(RemediationEngine::new(
            quarantine_path,
            72,   // 72-hour grace period
            true,  // verify before delete
        ));

        // Generate or load node ID
        let node_id = Uuid::new_v4().to_string();

        // Register this node in the database
        let hostname = hostname::get()
            .map(|h| h.to_string_lossy().to_string())
            .unwrap_or_else(|_| "unknown".to_string());
        let platform = std::env::consts::OS.to_string();
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64;

        database.upsert_node(&node_id, &hostname, "127.0.0.1", &platform, "0.3.0", now)?;

        info!("UneffAgent initialized — node_id: {}, hostname: {}", node_id, hostname);

        Ok(Self {
            config,
            database,
            scanner,
            remediation,
            grpc_service: None,
            gui_tx,
            scan_state: Arc::new(RwLock::new(None)),
            cancel_flag: Arc::new(AtomicBool::new(false)),
            node_id,
        })
    }

    /// Run the agent as a background service.
    /// Full speed, no brakes, all CPU cores.
    pub async fn run_service(&self) -> Result<()> {
        info!("UneffAgent service starting on port {}", self.config.grpc_port);

        // Start gRPC service
        let grpc = GrpcService::new(self.config.grpc_port);
        let grpc_handle = tokio::spawn(async move {
            if let Err(e) = grpc.start().await {
                error!("gRPC service failed: {}", e);
            }
        });

        // Keep service alive until shutdown signal
        tokio::signal::ctrl_c().await?;
        info!("UneffAgent service shutting down gracefully");

        grpc_handle.abort();
        Ok(())
    }

    /// Start a scan on the given paths.
    /// Full pipeline: discover → hash → DB insert → duplicate detect → report.
    /// Returns the scan ID for tracking.
    pub async fn start_scan(&self, paths: Vec<PathBuf>) -> Result<String> {
        let scan_id = Uuid::new_v4().to_string();
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64;

        // Reset cancel flag
        self.cancel_flag.store(false, Ordering::Relaxed);

        // Create scan record
        self.database.create_scan(&scan_id, &self.node_id, "user", now)?;

        // Update scan state
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

        info!("Scan {} started — {} paths queued", scan_id, paths.len());

        // Progress channel
        let (progress_tx, mut progress_rx) = mpsc::channel::<ScanProgress>(100);

        // Spawn scanner — returns Vec<ScannedFile>
        let scanner = self.scanner.clone();
        let scan_paths = paths.clone();
        let cancel = self.cancel_flag.clone();

        let scanner_handle = tokio::spawn(async move {
            scanner.scan_paths(&scan_paths, progress_tx, cancel).await
        });

        // Forward progress to GUI + update internal state
        let gui_tx = self.gui_tx.clone();
        let scan_state = self.scan_state.clone();

        let progress_handle = tokio::spawn(async move {
            while let Some(progress) = progress_rx.recv().await {
                // Update internal state
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

                // Forward to GUI
                if let Some(ref tx) = gui_tx {
                    let _ = tx.send(GuiMessage::ScanProgress(progress));
                }
            }
        });

        // Wait for scanner to finish
        let scanned_files = scanner_handle.await
            .map_err(|e| anyhow::anyhow!("Scanner task panic: {}", e))?
            .map_err(|e| anyhow::anyhow!("Scanner failed: {}", e))?;

        // Wait for progress forwarding to complete
        let _ = progress_handle.await;

        // ── Phase 3: Insert scanned files into database (batch) ────────
        if !scanned_files.is_empty() {
            let file_rows: Vec<FileRow> = scanned_files.iter().map(|sf| {
                let modified = sf.info.modified_time
                    .duration_since(UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs() as i64;

                let file_name = sf.info.path
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_default();

                FileRow {
                    id: None,
                    node_id: self.node_id.clone(),
                    scan_id: scan_id.clone(),
                    file_path: sf.info.path.to_string_lossy().to_string(),
                    file_name,
                    size_bytes: sf.info.size as i64,
                    modified_time: modified,
                    xxhash64: sf.xxhash64.clone(),
                    sha256_hash: sf.sha256_hash.clone(),
                }
            }).collect();

            info!("Inserting {} files into database (batch)", file_rows.len());
            if let Err(e) = self.database.insert_files_batch(&file_rows) {
                error!("Failed to batch-insert files: {}", e);
            }
        }

        // ── Phase 4: Duplicate detection ───────────────────────────────
        let duplicates_found = self.detect_duplicates().await.unwrap_or(0);

        // Update final state
        {
            let mut state = self.scan_state.write().await;
            if let Some(ref mut s) = *state {
                s.duplicates_found = duplicates_found;
                s.status = ScanStatus::Completed;
            }
        }

        // Complete scan record in DB
        let completed_at = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64;

        self.database.complete_scan(
            &scan_id,
            completed_at,
            scanned_files.len() as i64,
            scanned_files.iter().map(|f| f.info.size as i64).sum(),
        )?;

        info!(
            "Scan {} complete — {} files, {} duplicate groups",
            scan_id, scanned_files.len(), duplicates_found
        );

        Ok(scan_id)
    }

    /// Detect duplicate files in the database.
    /// Pipeline: size match → SHA-256 match → upsert duplicate group.
    /// Returns the number of duplicate groups found.
    pub async fn detect_duplicates(&self) -> Result<u64> {
        info!("Running duplicate detection pipeline...");

        // Find all SHA-256 matches (files that share the same hash)
        let sha_matches = self.database.find_sha256_matches(2)?;

        let mut groups_found = 0u64;

        for (sha256, size, count) in &sha_matches {
            // Upsert the duplicate group
            if let Err(e) = self.database.upsert_duplicate_group(sha256, *size, *count) {
                warn!("Failed to upsert duplicate group {}: {}", sha256, e);
                continue;
            }
            groups_found += 1;
        }

        if groups_found > 0 {
            let wasted = self.database.get_total_wasted_space().unwrap_or(0);
            info!(
                "Duplicate detection complete: {} groups, {} bytes wasted",
                groups_found, wasted
            );
        } else {
            info!("No duplicates found");
        }

        Ok(groups_found)
    }

    /// Stop an in-progress scan.
    pub async fn stop_scan(&self) -> Result<()> {
        self.cancel_flag.store(true, Ordering::Relaxed);
        let mut state = self.scan_state.write().await;
        if let Some(ref mut s) = *state {
            s.status = ScanStatus::Cancelled;
            info!("Scan {} cancelled by user", s.scan_id);
        }
        Ok(())
    }

    /// Get local drives for GUI sidebar display.
    pub fn get_local_drives(&self) -> Result<Vec<DriveInfo>> {
        let mut drives = Vec::new();

        #[cfg(windows)]
        {
            if let Ok(win_drives) = crate::platform::windows::get_all_drives() {
                for (i, d) in win_drives.iter().enumerate() {
                    drives.push(DriveInfo {
                        id: format!("drive_{}", i),
                        label: d.label.clone(),
                        mount_point: d.mount_point.clone(),
                        drive_type: d.drive_type.clone(),
                        total_space: d.total_space,
                        available_space: d.available_space,
                        is_removable: d.is_removable,
                    });
                }
            }
        }

        #[cfg(unix)]
        {
            if let Ok(unix_mounts) = crate::platform::unix::get_all_mounts() {
                for (i, m) in unix_mounts.iter().enumerate() {
                    drives.push(DriveInfo {
                        id: format!("mount_{}", i),
                        label: m.device.clone(),
                        mount_point: m.mount_point.clone(),
                        drive_type: m.fs_type.clone(),
                        total_space: m.total_space,
                        available_space: m.available_space,
                        is_removable: false,
                    });
                }
            }
        }

        Ok(drives)
    }

    /// Get current scan state.
    pub async fn get_scan_state(&self) -> Option<ScanState> {
        self.scan_state.read().await.clone()
    }

    /// Get the node ID.
    pub fn node_id(&self) -> &str {
        &self.node_id
    }

    /// Get a reference to the database.
    pub fn database(&self) -> &Arc<Database> {
        &self.database
    }

    /// Get a reference to the remediation engine.
    pub fn remediation(&self) -> &Arc<RemediationEngine> {
        &self.remediation
    }
}
