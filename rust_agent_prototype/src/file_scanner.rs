//! # File Scanner Module — Parallel Filesystem Walk + Progressive Hashing
//!
//! Philosophy: Full speed — all CPU cores, no throttling, no artificial limits.
//!
//! ## Pipeline: Discover → Hash (xxHash64) → Collect → Report
//! - Files discovered via WalkBuilder (respects .gitignore, custom patterns)
//! - Each file gets hashed on discovery (xxHash64 streaming)
//! - Results collected into ScannedFile vector
//! - Progress reported to GUI via mpsc channel
//! - Database insertion handled by caller for separation of concerns
//!
//! ## Parallelism
//! - Thread pool sized to `num_cpus::get().min(8)` threads
//! - Independent file hashing per thread — zero contention
//! - Cancellation token checked periodically during walk
//!
//! ## Phases
//! - **Discovery**: Initial filesystem walk, size collection
//! - **Hashing**: Progressive xxHash64 computation
//! - **Complete**: All files hashed, ready for duplicate detection


use anyhow::Result;
use ignore::WalkBuilder;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::SystemTime;
use tokio::sync::mpsc;
use tracing::{debug, info, warn};

use crate::config::ScanningConfig;
use crate::hashing::HashEngine;

/// Metadata gathered during file discovery (before hashing).
#[derive(Debug, Clone)]
pub struct FileInfo {
    pub path: PathBuf,
    pub size: u64,
    pub modified_time: SystemTime,
    pub is_directory: bool,
    pub is_symlink: bool,
    pub symlink_target: Option<PathBuf>,
}

/// A fully scanned file — discovery metadata + hash results.
#[derive(Debug, Clone)]
pub struct ScannedFile {
    pub info: FileInfo,
    pub xxhash64: Option<String>,
    pub sha256_hash: Option<String>,
}

/// The scanning pipeline — discovers files, hashes them, reports progress.
/// Database writes are NOT done here (separation of concerns — agent.rs handles that).
pub struct FileScanner {
    config: Arc<ScanningConfig>,
    hash_engine: Arc<HashEngine>,
}

impl FileScanner {
    pub fn new(config: Arc<ScanningConfig>) -> Self {
        Self {
            hash_engine: Arc::new(HashEngine::new(config.clone())),
            config,
        }
    }

    /// Get a reference to the hash engine (for standalone hash operations).
    pub fn hash_engine(&self) -> &Arc<HashEngine> {
        &self.hash_engine
    }

    /// Scan paths and stream progress. Returns all discovered+hashed files.
    ///
    /// Pipeline:
    ///   1. Walk all paths → discover FileInfo
    ///   2. Hash each file (xxHash64 always, SHA-256 always for now)
    ///   3. Collect into Vec<ScannedFile>
    ///   4. Report progress via mpsc channel
    ///
    /// The cancel_flag can be set to true externally to abort the scan.
    pub async fn scan_paths(
        &self,
        paths: &[PathBuf],
        progress_sender: mpsc::Sender<ScanProgress>,
        cancel_flag: Arc<AtomicBool>,
    ) -> Result<Vec<ScannedFile>> {
        info!("Starting scan of {} paths", paths.len());

        // Phase 1: Discover all files
        let _ = progress_sender.send(ScanProgress {
            status: ScanStatus::Scanning,
            phase: ScanPhase::Discovery,
            ..Default::default()
        }).await;

        let discovered = self.discover_files(paths, &progress_sender, &cancel_flag).await?;

        if cancel_flag.load(Ordering::Relaxed) {
            let _ = progress_sender.send(ScanProgress {
                status: ScanStatus::Cancelled,
                files_found: discovered.len() as u64,
                ..Default::default()
            }).await;
            return Ok(Vec::new());
        }

        info!("Discovery complete: {} files found", discovered.len());

        // Phase 2: Hash all discovered files in batches
        let _ = progress_sender.send(ScanProgress {
            status: ScanStatus::Scanning,
            phase: ScanPhase::Hashing,
            files_found: discovered.len() as u64,
            ..Default::default()
        }).await;

        let scanned = self.hash_discovered_files(
            &discovered, &progress_sender, &cancel_flag,
        ).await?;

        if cancel_flag.load(Ordering::Relaxed) {
            let _ = progress_sender.send(ScanProgress {
                status: ScanStatus::Cancelled,
                files_found: discovered.len() as u64,
                files_processed: scanned.len() as u64,
                ..Default::default()
            }).await;
            return Ok(scanned);
        }

        // Phase 3: Complete
        let total_bytes: u64 = scanned.iter().map(|f| f.info.size).sum();
        let _ = progress_sender.send(ScanProgress {
            status: ScanStatus::Completed,
            phase: ScanPhase::Complete,
            files_found: discovered.len() as u64,
            files_processed: scanned.len() as u64,
            bytes_processed: total_bytes,
            ..Default::default()
        }).await;

        info!(
            "Scan completed: {} files discovered, {} hashed, {} bytes",
            discovered.len(), scanned.len(), total_bytes
        );

        Ok(scanned)
    }

    /// Discover all files across the given paths.
    /// Respects config: max_file_size_gb, default_exclude_patterns.
    /// Reports progress periodically.
    async fn discover_files(
        &self,
        paths: &[PathBuf],
        progress_sender: &mpsc::Sender<ScanProgress>,
        cancel_flag: &AtomicBool,
    ) -> Result<Vec<FileInfo>> {
        let mut discovered = Vec::new();
        let mut last_report = SystemTime::now();

        for path in paths {
            if cancel_flag.load(Ordering::Relaxed) { break; }
            info!("Discovering files in: {}", path.display());

            let mut walk_builder = WalkBuilder::new(path);

            // Apply exclude patterns from config
            for pattern in &self.config.default_exclude_patterns {
                walk_builder.add_ignore(pattern);
            }

            walk_builder
                .follow_links(false)
                .same_file_system(true)
                .threads(num_cpus::get().min(8)); // Parallel walk

            let walker = walk_builder.build();

            for result in walker {
                if cancel_flag.load(Ordering::Relaxed) { break; }

                match result {
                    Ok(entry) => {
                        let metadata = match entry.metadata() {
                            Ok(m) => m,
                            Err(e) => {
                                warn!("Metadata error {}: {}", entry.path().display(), e);
                                continue;
                            }
                        };

                        // Skip directories — we only hash files
                        if metadata.is_dir() { continue; }

                        let file_size = metadata.len();

                        // Skip empty files (can't be duplicates meaningfully)
                        if file_size == 0 { continue; }

                        // Skip files beyond max size
                        let max_bytes = self.config.max_file_size_gb * 1024 * 1024 * 1024;
                        if file_size > max_bytes {
                            debug!(
                                "Skipping oversized: {} ({} bytes)",
                                entry.path().display(), file_size
                            );
                            continue;
                        }

                        let file_info = FileInfo {
                            path: entry.path().to_path_buf(),
                            size: file_size,
                            modified_time: metadata
                                .modified()
                                .unwrap_or_else(|_| SystemTime::now()),
                            is_directory: false,
                            is_symlink: metadata.is_symlink(),
                            symlink_target: if metadata.is_symlink() {
                                std::fs::read_link(entry.path()).ok()
                            } else {
                                None
                            },
                        };

                        discovered.push(file_info);

                        // Report progress periodically
                        let elapsed = last_report.elapsed().unwrap_or_default().as_millis();
                        if elapsed >= self.config.progress_report_interval_ms as u128 {
                            let _ = progress_sender.try_send(ScanProgress {
                                status: ScanStatus::Scanning,
                                phase: ScanPhase::Discovery,
                                files_found: discovered.len() as u64,
                                current_path: entry.path().to_string_lossy().to_string(),
                                ..Default::default()
                            });
                            last_report = SystemTime::now();
                        }
                    }
                    Err(e) => {
                        warn!("Walk error: {}", e);
                    }
                }
            }
        }

        Ok(discovered)
    }

    /// Hash all discovered files in batches.
    /// Stage 1: xxHash64 (always).
    /// Stage 2: SHA-256 (always, for now — future optimization: only on xxHash match).
    async fn hash_discovered_files(
        &self,
        files: &[FileInfo],
        progress_sender: &mpsc::Sender<ScanProgress>,
        cancel_flag: &AtomicBool,
    ) -> Result<Vec<ScannedFile>> {
        let total = files.len();
        let batch_size = self.config.hash_batch_size.max(1);
        let mut results = Vec::with_capacity(total);
        let mut last_report = SystemTime::now();

        for chunk in files.chunks(batch_size) {
            if cancel_flag.load(Ordering::Relaxed) { break; }

            let hashed = self.hash_engine.hash_files(chunk).await?;
            results.extend(hashed);

            // Report progress
            let elapsed = last_report.elapsed().unwrap_or_default().as_millis();
            if elapsed >= self.config.progress_report_interval_ms as u128 {
                let bytes_so_far: u64 = results.iter().map(|f| f.info.size).sum();
                let _ = progress_sender.try_send(ScanProgress {
                    status: ScanStatus::Scanning,
                    phase: ScanPhase::Hashing,
                    files_found: total as u64,
                    files_processed: results.len() as u64,
                    bytes_processed: bytes_so_far,
                    current_path: chunk.last()
                        .map(|f| f.path.to_string_lossy().to_string())
                        .unwrap_or_default(),
                    ..Default::default()
                });
                last_report = SystemTime::now();
            }
        }

        Ok(results)
    }
}

// ── Progress Types ─────────────────────────────────────────────────────

#[derive(Debug, Clone, Default)]
pub struct ScanProgress {
    pub files_processed: u64,
    pub bytes_processed: u64,
    pub files_found: u64,
    pub duplicates_found: u64,
    pub current_path: String,
    pub status: ScanStatus,
    pub phase: ScanPhase,
    pub error_message: String,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ScanStatus {
    Scanning,
    Completed,
    Failed,
    Cancelled,
}

impl Default for ScanStatus {
    fn default() -> Self { Self::Scanning }
}

#[derive(Debug, Clone, PartialEq)]
pub enum ScanPhase {
    Discovery,
    Hashing,
    DuplicateDetection,
    Complete,
}

impl Default for ScanPhase {
    fn default() -> Self { Self::Discovery }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    use tokio::sync::mpsc;

    #[tokio::test]
    async fn test_file_scanner_basic() -> Result<()> {
        let dir = tempdir()?;
        let test_file = dir.path().join("test.txt");
        std::fs::write(&test_file, "Hello, World!")?;

        let config = Arc::new(ScanningConfig::default());
        let scanner = FileScanner::new(config);
        let cancel = Arc::new(AtomicBool::new(false));
        let (progress_sender, _progress_receiver) = mpsc::channel(100);

        let results = scanner
            .scan_paths(&[dir.path().to_path_buf()], progress_sender, cancel)
            .await?;

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].info.size, 13); // "Hello, World!" = 13 bytes
        assert!(results[0].xxhash64.is_some());
        assert!(results[0].sha256_hash.is_some());

        Ok(())
    }

    #[tokio::test]
    async fn test_cancellation() -> Result<()> {
        let dir = tempdir()?;
        for i in 0..100 {
            std::fs::write(dir.path().join(format!("file_{}.txt", i)), format!("content {}", i))?;
        }

        let config = Arc::new(ScanningConfig::default());
        let scanner = FileScanner::new(config);
        let cancel = Arc::new(AtomicBool::new(true)); // Pre-cancelled
        let (progress_sender, _) = mpsc::channel(100);

        let results = scanner
            .scan_paths(&[dir.path().to_path_buf()], progress_sender, cancel)
            .await?;

        assert!(results.is_empty());
        Ok(())
    }
}
