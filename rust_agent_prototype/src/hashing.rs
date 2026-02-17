// Gillsystems_uneff_your_rigs_messy_files — Hashing Engine Module
// Philosophy: Built with zero frameworks, maximum intent.
//
// Two-stage hashing pipeline:
//   Stage 1: xxHash64 — extremely fast non-cryptographic hash for pre-filtering
//   Stage 2: SHA-256 — cryptographic hash for collision-resistant verification
//
// Only files that match on size AND xxHash64 get promoted to SHA-256.
// Streaming for large files (>1GB) to avoid memory pressure.
// Full speed — all CPU cores, no throttling, no artificial limits.

use anyhow::Result;
use sha2::{Sha256, Digest};
use std::io::Read;
use std::path::Path;
use std::sync::Arc;
use tracing::debug;

use crate::config::ScanningConfig;
use crate::file_scanner::{FileInfo, ScannedFile};

/// Buffer size for streaming hashes (64KB for optimal I/O throughput).
const HASH_BUFFER_SIZE: usize = 64 * 1024;

/// Files larger than this use streaming xxHash (don't load into memory).
const STREAMING_THRESHOLD: u64 = 256 * 1024 * 1024; // 256MB

/// The hash engine that powers duplicate detection.
/// Full speed — all CPU cores, no throttling, no artificial limits.
pub struct HashEngine {
    config: Arc<ScanningConfig>,
}

impl HashEngine {
    pub fn new(config: Arc<ScanningConfig>) -> Self {
        Self { config }
    }

    /// Hash a batch of files — xxHash64 always, SHA-256 always.
    /// In the full pipeline, SHA-256 would be deferred until xxHash matches are found.
    pub async fn hash_files(&self, files: &[FileInfo]) -> Result<Vec<ScannedFile>> {
        let mut results = Vec::with_capacity(files.len());

        for file_info in files {
            if file_info.is_directory || file_info.size == 0 {
                continue;
            }

            // Skip files beyond configured max size
            let max_bytes = self.config.max_file_size_gb * 1024 * 1024 * 1024;
            if file_info.size > max_bytes {
                debug!("Skipping oversized file: {} ({} bytes)", file_info.path.display(), file_info.size);
                continue;
            }

            let xxhash = self.compute_xxhash64(&file_info.path, file_info.size);
            let sha256 = self.compute_sha256(&file_info.path);

            results.push(ScannedFile {
                info: file_info.clone(),
                xxhash64: xxhash.ok(),
                sha256_hash: sha256.ok(),
            });
        }

        Ok(results)
    }

    /// Compute only xxHash64 (Stage 1 — fast pre-filter).
    /// Use this for the initial pass; only promote to SHA-256 on match.
    pub fn compute_xxhash64_only(&self, path: &Path, size: u64) -> Result<String> {
        self.compute_xxhash64(path, size)
    }

    /// Compute only SHA-256 (Stage 2 — verification).
    /// Only call this on files that already matched on size + xxHash64.
    pub fn compute_sha256_only(&self, path: &Path) -> Result<String> {
        self.compute_sha256(path)
    }

    /// Compute xxHash64 — blazing fast, non-cryptographic.
    /// Streams for files above STREAMING_THRESHOLD to avoid memory pressure.
    fn compute_xxhash64(&self, path: &Path, size: u64) -> Result<String> {
        if size <= STREAMING_THRESHOLD {
            // Small file: read entirely into memory (fastest path)
            let data = std::fs::read(path)?;
            let hash = xxhash_rust::xxh3::xxh3_64(&data);
            Ok(format!("{:016x}", hash))
        } else {
            // Large file: stream in chunks
            let mut file = std::fs::File::open(path)?;
            let mut hasher = xxhash_rust::xxh3::Xxh3Default::new();
            let mut buffer = vec![0u8; HASH_BUFFER_SIZE];

            loop {
                let bytes_read = file.read(&mut buffer)?;
                if bytes_read == 0 { break; }
                hasher.update(&buffer[..bytes_read]);
            }

            let hash = hasher.digest();
            Ok(format!("{:016x}", hash))
        }
    }

    /// Compute SHA-256 — cryptographic, collision-resistant.
    /// Always streams regardless of file size (SHA-256 is stateful).
    fn compute_sha256(&self, path: &Path) -> Result<String> {
        let mut file = std::fs::File::open(path)?;
        let mut hasher = Sha256::new();
        let mut buffer = [0u8; HASH_BUFFER_SIZE];

        loop {
            let bytes_read = file.read(&mut buffer)?;
            if bytes_read == 0 { break; }
            hasher.update(&buffer[..bytes_read]);
        }

        let result = hasher.finalize();
        Ok(format!("{:x}", result))
    }

    /// Verify two files are byte-for-byte identical.
    /// Used before destructive operations (delete, hard link).
    pub fn verify_identical(path_a: &Path, path_b: &Path) -> Result<bool> {
        let mut file_a = std::fs::File::open(path_a)?;
        let mut file_b = std::fs::File::open(path_b)?;

        let meta_a = file_a.metadata()?;
        let meta_b = file_b.metadata()?;

        if meta_a.len() != meta_b.len() {
            return Ok(false);
        }

        let mut buf_a = [0u8; HASH_BUFFER_SIZE];
        let mut buf_b = [0u8; HASH_BUFFER_SIZE];

        loop {
            let n_a = file_a.read(&mut buf_a)?;
            let n_b = file_b.read(&mut buf_b)?;

            if n_a != n_b { return Ok(false); }
            if n_a == 0 { break; }
            if buf_a[..n_a] != buf_b[..n_b] { return Ok(false); }
        }

        Ok(true)
    }
}
