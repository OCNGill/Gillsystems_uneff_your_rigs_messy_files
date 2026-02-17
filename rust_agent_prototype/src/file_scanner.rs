use anyhow::{Context, Result};
use ignore::WalkBuilder;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::SystemTime;
use tokio::sync::mpsc;
use tracing::{debug, error, info, warn};
use walkdir::WalkDir;

use crate::config::ScanningConfig;
use crate::hashing::HashEngine;

#[derive(Debug, Clone)]
pub struct FileInfo {
    pub path: PathBuf,
    pub size: u64,
    pub modified_time: SystemTime,
    pub is_directory: bool,
    pub is_symlink: bool,
    pub symlink_target: Option<PathBuf>,
}

#[derive(Debug, Clone)]
pub struct ScannedFile {
    pub info: FileInfo,
    pub xxhash64: Option<String>,
    pub sha256_hash: Option<String>,
}

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
    
    pub async fn scan_paths(
        &self,
        paths: &[PathBuf],
        progress_sender: mpsc::Sender<ScanProgress>,
    ) -> Result<()> {
        info!("Starting scan of {} paths", paths.len());
        
        let (file_sender, mut file_receiver) = mpsc::channel::<FileInfo>(1000);
        let (hash_sender, mut hash_receiver) = mpsc::channel::<FileInfo>(1000);
        
        // File discovery task
        let discovery_config = self.config.clone();
        let discovery_sender = file_sender.clone();
        let discovery_task = tokio::spawn(async move {
            if let Err(e) = Self::discover_files(paths, discovery_config, discovery_sender).await {
                error!("File discovery failed: {}", e);
            }
        });
        
        // Hashing task pool
        let hash_config = self.config.clone();
        let hash_engine = self.hash_engine.clone();
        let hash_progress_sender = progress_sender.clone();
        let hash_task = tokio::spawn(async move {
            if let Err(e) = Self::hash_files(hash_engine, hash_config, hash_sender, hash_progress_sender).await {
                error!("File hashing failed: {}", e);
            }
        });
        
        // Progress reporting task
        let mut files_processed = 0u64;
        let mut bytes_processed = 0u64;
        let mut last_progress = SystemTime::now();
        
        while let Some(file_info) = file_receiver.recv().await {
            files_processed += 1;
            bytes_processed += file_info.size;
            
            // Send file to hashing pipeline
            if let Err(e) = hash_sender.send(file_info.clone()).await {
                warn!("Failed to send file to hasher: {}", e);
            }
            
            // Report progress periodically
            if last_progress.elapsed().unwrap_or_default().as_millis() 
                >= self.config.progress_report_interval_ms as u128 {
                
                let progress = ScanProgress {
                    files_processed,
                    bytes_processed,
                    current_path: file_info.path.to_string_lossy().to_string(),
                    ..Default::default()
                };
                
                if let Err(e) = progress_sender.send(progress).await {
                    debug!("Progress send failed: {}", e);
                    break;
                }
                
                last_progress = SystemTime::now();
            }
        }
        
        // Wait for all tasks to complete
        drop(hash_sender); // Close hash sender to signal completion
        
        let _ = tokio::try_join!(discovery_task, hash_task);
        
        // Send final progress
        let final_progress = ScanProgress {
            files_processed,
            bytes_processed,
            status: ScanStatus::Completed,
            ..Default::default()
        };
        
        let _ = progress_sender.send(final_progress).await;
        
        info!("Scan completed: {} files, {} bytes processed", files_processed, bytes_processed);
        Ok(())
    }
    
    async fn discover_files(
        paths: &[PathBuf],
        config: Arc<ScanningConfig>,
        sender: mpsc::Sender<FileInfo>,
    ) -> Result<()> {
        for path in paths {
            info!("Discovering files in: {}", path.display());
            
            let mut walk_builder = WalkBuilder::new(path);
            
            // Configure ignore patterns
            for pattern in &config.default_exclude_patterns {
                walk_builder.add_ignore(pattern);
            }
            
            walk_builder
                .max_depth(Some(10)) // Reasonable default depth
                .follow_links(false)
                .same_file_system(true);
            
            let walker = walk_builder.build();
            
            for result in walker {
                match result {
                    Ok(entry) => {
                        let metadata = match entry.metadata() {
                            Ok(meta) => meta,
                            Err(e) => {
                                warn!("Failed to read metadata for {}: {}", entry.path().display(), e);
                                continue;
                            }
                        };
                        
                        let file_size = metadata.len();
                        
                        // Skip files that are too large
                        if file_size > config.max_file_size_gb * 1024 * 1024 * 1024 {
                            debug!("Skipping large file: {} ({} bytes)", entry.path().display(), file_size);
                            continue;
                        }
                        
                        let file_info = FileInfo {
                            path: entry.path().to_path_buf(),
                            size: file_size,
                            modified_time: metadata.modified().unwrap_or_else(|_| SystemTime::now()),
                            is_directory: metadata.is_dir(),
                            is_symlink: metadata.is_symlink(),
                            symlink_target: if metadata.is_symlink() {
                                std::fs::read_link(entry.path()).ok()
                            } else {
                                None
                            },
                        };
                        
                        if let Err(e) = sender.send(file_info).await {
                            error!("Failed to send file info: {}", e);
                            break;
                        }
                    }
                    Err(e) => {
                        warn!("Error during file walk: {}", e);
                    }
                }
            }
        }
        
        Ok(())
    }
    
    async fn hash_files(
        hash_engine: Arc<HashEngine>,
        config: Arc<ScanningConfig>,
        mut receiver: mpsc::Receiver<FileInfo>,
        progress_sender: mpsc::Sender<ScanProgress>,
    ) -> Result<()> {
        let mut batch = Vec::with_capacity(config.hash_batch_size);
        
        while let Some(file_info) = receiver.recv().await {
            batch.push(file_info);
            
            if batch.len() >= config.hash_batch_size {
                Self::process_hash_batch(&hash_engine, &batch, &progress_sender).await?;
                batch.clear();
            }
        }
        
        // Process remaining files
        if !batch.is_empty() {
            Self::process_hash_batch(&hash_engine, &batch, &progress_sender).await?;
        }
        
        Ok(())
    }
    
    async fn process_hash_batch(
        hash_engine: &HashEngine,
        batch: &[FileInfo],
        progress_sender: &mpsc::Sender<ScanProgress>,
    ) -> Result<()> {
        let hashed_files = hash_engine.hash_files(batch).await?;
        
        for scanned_file in hashed_files {
            // Here we would typically send the scanned file to the database
            // For now, we'll just update progress
            debug!("Hashed file: {:?}", scanned_file.info.path);
        }
        
        // Update progress
        let progress = ScanProgress {
            files_processed: batch.len() as u64,
            ..Default::default()
        };
        
        if let Err(e) = progress_sender.try_send(progress) {
            debug!("Failed to send hash progress: {}", e);
        }
        
        Ok(())
    }
}

#[derive(Debug, Clone, Default)]
pub struct ScanProgress {
    pub files_processed: u64,
    pub bytes_processed: u64,
    pub files_found: u64,
    pub duplicates_found: u64,
    pub current_path: String,
    pub status: ScanStatus,
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
    fn default() -> Self {
        Self::Scanning
    }
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
        
        let (progress_sender, _progress_receiver) = mpsc::channel(100);
        
        scanner
            .scan_paths(&[dir.path().to_path_buf()], progress_sender)
            .await?;
        
        Ok(())
    }
}
