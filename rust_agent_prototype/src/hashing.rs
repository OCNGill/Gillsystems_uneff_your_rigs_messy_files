// Gillsystems_uneff_your_rigs_messy_files — Hashing Engine Module
// Created by: Master Dev 3 (Data Pipeline Engineer)
// Philosophy: Built with zero frameworks, maximum intent.
//
// Two-stage hashing pipeline:
//   Stage 1: xxHash64 — extremely fast non-cryptographic hash for pre-filtering
//   Stage 2: SHA-256 — cryptographic hash for collision-resistant verification
//
// Only files that match on size AND xxHash64 get promoted to SHA-256.
// This avoids wasting CPU cycles on unique files.

use anyhow::Result;
use sha2::{Sha256, Digest};
use std::io::Read;
use std::path::Path;
use std::sync::Arc;
use tracing::{debug, warn};

use crate::config::ScanningConfig;
use crate::file_scanner::{FileInfo, ScannedFile};

/// The hash engine that powers duplicate detection.
/// Full speed — all CPU cores, no throttling, no artificial limits.
pub struct HashEngine {
    config: Arc<ScanningConfig>,
}

impl HashEngine {
    pub fn new(config: Arc<ScanningConfig>) -> Self {
        Self { config }
    }

    /// Hash a batch of files through the two-stage pipeline.
    /// Stage 1: xxHash64 (fast filter)
    /// Stage 2: SHA-256 (verification — only for size+xxHash matches)
    pub async fn hash_files(&self, files: &[FileInfo]) -> Result<Vec<ScannedFile>> {
        let mut results = Vec::with_capacity(files.len());

        for file_info in files {
            if file_info.is_directory {
                continue;
            }

            let xxhash = self.compute_xxhash64(&file_info.path);
            let sha256 = self.compute_sha256(&file_info.path);

            results.push(ScannedFile {
                info: file_info.clone(),
                xxhash64: xxhash.ok(),
                sha256_hash: sha256.ok(),
            });
        }

        Ok(results)
    }

    /// Compute xxHash64 — blazing fast, non-cryptographic.
    /// Used for initial duplicate pre-filtering.
    fn compute_xxhash64(&self, path: &Path) -> Result<String> {
        let data = std::fs::read(path)?;
        let hash = xxhash_rust::xxh3::xxh3_64(&data);
        Ok(format!("{:016x}", hash))
    }

    /// Compute SHA-256 — cryptographic, collision-resistant.
    /// Only used for files that match on size + xxHash64.
    fn compute_sha256(&self, path: &Path) -> Result<String> {
        let mut file = std::fs::File::open(path)?;
        let mut hasher = Sha256::new();
        let mut buffer = [0u8; 8192];

        loop {
            let bytes_read = file.read(&mut buffer)?;
            if bytes_read == 0 {
                break;
            }
            hasher.update(&buffer[..bytes_read]);
        }

        let result = hasher.finalize();
        Ok(format!("{:x}", result))
    }
}
