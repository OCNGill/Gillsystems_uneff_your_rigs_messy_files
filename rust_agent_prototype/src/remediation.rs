// Gillsystems_uneff_your_rigs_messy_files — Remediation Module
// Philosophy: User Empowerment — honest warnings, never silent deletions.
//
// Five remediation strategies (filesystem-aware):
//   1. Quarantine — move to safe directory for review
//   2. ZFS Block Cloning — zero-copy dedup via reflink/FICLONE (PRIMARY target)
//   3. Hard Link — NTFS/ext4/XFS/APFS hard links for dedup
//   4. Move — relocate files to a different path
//   5. Delete — permanent removal (with verification option)
//
// Filesystem priority: ZFS first → NTFS second → ext4/XFS → FAT32 fallback
// Every action is logged to the audit trail. No silent operations.

use anyhow::{Context, Result};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};
use tracing::{info, warn, error};

use crate::hashing::HashEngine;

/// Result of a remediation operation.
#[derive(Debug, Clone)]
pub struct RemediationResult {
    pub action: String,
    pub file_path: String,
    pub source_path: Option<String>,
    pub strategy: String,
    pub fs_type: String,
    pub space_recovered: u64,
    pub success: bool,
    pub error: Option<String>,
}

/// Remediation engine — handles all file operations on duplicate groups.
/// Funny but honest warnings. Never blocking. Power to the people.
pub struct RemediationEngine {
    quarantine_path: PathBuf,
    grace_period_hours: u32,
    verify_before_delete: bool,
}

impl RemediationEngine {
    pub fn new(quarantine_path: PathBuf, grace_period_hours: u32, verify_before_delete: bool) -> Self {
        // Ensure quarantine directory exists
        std::fs::create_dir_all(&quarantine_path).ok();
        Self {
            quarantine_path,
            grace_period_hours,
            verify_before_delete,
        }
    }

    /// Get the quarantine path.
    pub fn quarantine_path(&self) -> &Path {
        &self.quarantine_path
    }

    /// Quarantine a file — move to safe directory for later review.
    /// Risk Level: 🟢 Low — files are preserved, just relocated.
    /// Preserves original filename with timestamp prefix to avoid collisions.
    pub fn quarantine(&self, file_path: &Path) -> Result<RemediationResult> {
        info!("Quarantining: {}", file_path.display());

        let file_name = file_path.file_name()
            .unwrap_or_default()
            .to_string_lossy();

        // Timestamp prefix to avoid collisions
        let ts = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let dest = self.quarantine_path.join(format!("{}_{}", ts, file_name));

        // Get file size before moving
        let _size = std::fs::metadata(file_path)
            .map(|m| m.len())
            .unwrap_or(0);

        // Move the file
        match std::fs::rename(file_path, &dest) {
            Ok(_) => {
                info!("Quarantined: {} → {}", file_path.display(), dest.display());
                Ok(RemediationResult {
                    action: "quarantine".to_string(),
                    file_path: file_path.to_string_lossy().to_string(),
                    source_path: Some(dest.to_string_lossy().to_string()),
                    strategy: "move".to_string(),
                    fs_type: String::new(),
                    space_recovered: 0, // Not deleted, just moved
                    success: true,
                    error: None,
                })
            }
            Err(e) => {
                // Cross-device move — fall back to copy+delete
                warn!("Rename failed (cross-device?), trying copy+delete: {}", e);
                std::fs::copy(file_path, &dest)
                    .context("Failed to copy to quarantine")?;
                std::fs::remove_file(file_path)
                    .context("Failed to remove original after copy")?;

                info!("Quarantined (copy+delete): {} → {}", file_path.display(), dest.display());
                Ok(RemediationResult {
                    action: "quarantine".to_string(),
                    file_path: file_path.to_string_lossy().to_string(),
                    source_path: Some(dest.to_string_lossy().to_string()),
                    strategy: "copy-delete".to_string(),
                    fs_type: String::new(),
                    space_recovered: 0,
                    success: true,
                    error: None,
                })
            }
        }
    }

    /// Restore a quarantined file to its original location.
    pub fn restore_from_quarantine(&self, quarantine_name: &str, original_path: &Path) -> Result<()> {
        let quarantine_file = self.quarantine_path.join(quarantine_name);
        if !quarantine_file.exists() {
            anyhow::bail!("Quarantine file not found: {}", quarantine_file.display());
        }

        // Ensure parent directory exists
        if let Some(parent) = original_path.parent() {
            std::fs::create_dir_all(parent).ok();
        }

        std::fs::rename(&quarantine_file, original_path)
            .or_else(|_| {
                std::fs::copy(&quarantine_file, original_path)?;
                std::fs::remove_file(&quarantine_file)?;
                Ok::<(), std::io::Error>(())
            })
            .context("Failed to restore from quarantine")?;

        info!("Restored: {} → {}", quarantine_name, original_path.display());
        Ok(())
    }

    /// Filesystem-aware dedup — optimized for ZFS first, NTFS second, FAT32 fallback.
    /// Priority: ZFS block cloning > NTFS hard link > ext4/XFS hard link > FAT32 copy-delete.
    /// Risk Level: 🟡 Medium — depends on filesystem capabilities.
    ///
    /// source = the file to KEEP (primary)
    /// duplicate = the file to REPLACE with a link
    pub fn dedup_or_hard_link(&self, source: &Path, duplicate: &Path, fs_type: &str) -> Result<RemediationResult> {
        let size = std::fs::metadata(duplicate)
            .map(|m| m.len())
            .unwrap_or(0);

        let result = match fs_type {
            "zfs" => {
                info!("ZFS block cloning: {} → {} (zero-copy, COW-aware)", duplicate.display(), source.display());
                self.zfs_reflink(source, duplicate)
                    .map(|_| ("zfs_reflink".to_string(), size))
            }
            "ntfs" => {
                info!("NTFS hard link: {} → {}", duplicate.display(), source.display());
                self.create_hard_link(source, duplicate)
                    .map(|_| ("ntfs_hardlink".to_string(), size))
            }
            "ext4" | "xfs" | "apfs" | "btrfs" => {
                info!("POSIX hard link: {} → {} ({})", duplicate.display(), source.display(), fs_type);
                // Try reflink first for btrfs/APFS, fall back to hard link
                if fs_type == "btrfs" || fs_type == "apfs" {
                    match self.posix_reflink(source, duplicate) {
                        Ok(_) => Ok(("reflink".to_string(), size)),
                        Err(_) => {
                            info!("Reflink failed on {}, falling back to hard link", fs_type);
                            self.create_hard_link(source, duplicate)
                                .map(|_| ("hardlink".to_string(), size))
                        }
                    }
                } else {
                    self.create_hard_link(source, duplicate)
                        .map(|_| ("hardlink".to_string(), size))
                }
            }
            "fat32" | "exfat" | "vfat" => {
                warn!("FAT32/exFAT: No hard link support — no dedup possible for {}", duplicate.display());
                // On FAT32 we can't dedup. We can only delete or quarantine.
                anyhow::bail!(
                    "FAT32/exFAT does not support hard links or reflinks. Use quarantine or delete instead."
                );
            }
            _ => {
                info!("Unknown filesystem '{}' — attempting POSIX hard link as fallback", fs_type);
                self.create_hard_link(source, duplicate)
                    .map(|_| ("hardlink_fallback".to_string(), size))
            }
        };

        match result {
            Ok((strategy, recovered)) => Ok(RemediationResult {
                action: "dedup".to_string(),
                file_path: duplicate.to_string_lossy().to_string(),
                source_path: Some(source.to_string_lossy().to_string()),
                strategy,
                fs_type: fs_type.to_string(),
                space_recovered: recovered,
                success: true,
                error: None,
            }),
            Err(e) => Ok(RemediationResult {
                action: "dedup".to_string(),
                file_path: duplicate.to_string_lossy().to_string(),
                source_path: Some(source.to_string_lossy().to_string()),
                strategy: "failed".to_string(),
                fs_type: fs_type.to_string(),
                space_recovered: 0,
                success: false,
                error: Some(e.to_string()),
            }),
        }
    }

    /// Create a hard link: delete the duplicate, create a hard link in its place.
    fn create_hard_link(&self, source: &Path, duplicate: &Path) -> Result<()> {
        // Safety: verify both files exist and have same content
        if !source.exists() {
            anyhow::bail!("Source file not found: {}", source.display());
        }
        if !duplicate.exists() {
            anyhow::bail!("Duplicate file not found: {}", duplicate.display());
        }

        // Verify files are identical before destructive operation
        if self.verify_before_delete {
            if !HashEngine::verify_identical(source, duplicate)? {
                anyhow::bail!(
                    "Files are NOT identical despite matching hash! Refusing to link: {} ↔ {}",
                    source.display(), duplicate.display()
                );
            }
        }

        // Delete the duplicate
        std::fs::remove_file(duplicate)
            .context("Failed to remove duplicate before hard link")?;

        // Create hard link
        #[cfg(unix)]
        {
            std::fs::hard_link(source, duplicate)
                .context("Failed to create POSIX hard link")?;
        }

        #[cfg(windows)]
        {
            std::fs::hard_link(source, duplicate)
                .context("Failed to create hard link")?;
        }

        info!("Hard link created: {} → {}", duplicate.display(), source.display());
        Ok(())
    }

    /// ZFS reflink via ioctl FICLONE (Linux) or copy_file_range.
    #[cfg(target_os = "linux")]
    fn zfs_reflink(&self, source: &Path, duplicate: &Path) -> Result<()> {
        use std::os::unix::io::AsRawFd;

        // Verify files are identical
        if self.verify_before_delete {
            if !HashEngine::verify_identical(source, duplicate)? {
                anyhow::bail!("Files are NOT identical! Refusing reflink.");
            }
        }

        // Delete duplicate, create new file, FICLONE from source
        std::fs::remove_file(duplicate)
            .context("Failed to remove duplicate before reflink")?;

        let src_file = std::fs::File::open(source)?;
        let dst_file = std::fs::File::create(duplicate)?;

        // FICLONE ioctl
        const FICLONE: libc::c_ulong = 0x40049409;
        let ret = unsafe {
            libc::ioctl(dst_file.as_raw_fd(), FICLONE, src_file.as_raw_fd())
        };

        if ret != 0 {
            // FICLONE failed — fall back to hard link
            warn!("FICLONE failed, falling back to hard link");
            std::fs::remove_file(duplicate).ok();
            std::fs::hard_link(source, duplicate)
                .context("Hard link fallback after FICLONE failure")?;
        }

        info!("ZFS reflink created: {} → {}", duplicate.display(), source.display());
        Ok(())
    }

    #[cfg(not(target_os = "linux"))]
    fn zfs_reflink(&self, source: &Path, duplicate: &Path) -> Result<()> {
        // ZFS reflink only supported on Linux; fall back to hard link
        warn!("ZFS reflink not available on this platform — falling back to hard link");
        self.create_hard_link(source, duplicate)
    }

    /// POSIX reflink for btrfs/APFS via copy_file_range or clonefile.
    #[cfg(target_os = "linux")]
    fn posix_reflink(&self, source: &Path, duplicate: &Path) -> Result<()> {
        // Same as ZFS reflink on Linux — uses FICLONE
        self.zfs_reflink(source, duplicate)
    }

    #[cfg(target_os = "macos")]
    fn posix_reflink(&self, source: &Path, duplicate: &Path) -> Result<()> {
        use std::ffi::CString;

        if self.verify_before_delete {
            if !HashEngine::verify_identical(source, duplicate)? {
                anyhow::bail!("Files are NOT identical! Refusing reflink.");
            }
        }

        std::fs::remove_file(duplicate)?;

        let src = CString::new(source.to_string_lossy().as_bytes())?;
        let dst = CString::new(duplicate.to_string_lossy().as_bytes())?;

        // macOS clonefile(2)
        let ret = unsafe {
            libc::clonefile(src.as_ptr(), dst.as_ptr(), 0)
        };

        if ret != 0 {
            warn!("clonefile failed, falling back to hard link");
            self.create_hard_link(source, duplicate)?;
        }

        Ok(())
    }

    #[cfg(windows)]
    fn posix_reflink(&self, _source: &Path, _duplicate: &Path) -> Result<()> {
        anyhow::bail!("Reflink not supported on Windows — use hard link instead")
    }

    /// Move a file to a new location.
    /// Risk Level: 🟡 Medium — file is relocated, not deleted.
    pub fn move_file(&self, source: &Path, destination: &Path) -> Result<RemediationResult> {
        info!("Moving: {} → {}", source.display(), destination.display());

        let _size = std::fs::metadata(source)
            .map(|m| m.len())
            .unwrap_or(0);

        // Ensure destination parent exists
        if let Some(parent) = destination.parent() {
            std::fs::create_dir_all(parent).ok();
        }

        match std::fs::rename(source, destination) {
            Ok(_) => {}
            Err(_) => {
                // Cross-device: copy then delete
                std::fs::copy(source, destination)
                    .context("Failed to copy for cross-device move")?;
                std::fs::remove_file(source)
                    .context("Failed to remove source after copy")?;
            }
        }

        info!("Moved: {} → {}", source.display(), destination.display());
        Ok(RemediationResult {
            action: "move".to_string(),
            file_path: source.to_string_lossy().to_string(),
            source_path: Some(destination.to_string_lossy().to_string()),
            strategy: "rename".to_string(),
            fs_type: String::new(),
            space_recovered: 0,
            success: true,
            error: None,
        })
    }

    /// Permanently delete a file.
    /// Risk Level: 🔴 High — this is irreversible.
    /// If verify_before_delete is true, verifies hash matches before deletion.
    pub fn delete(&self, file_path: &Path, expected_sha256: Option<&str>) -> Result<RemediationResult> {
        warn!("DELETING: {} — this action is permanent!", file_path.display());

        let size = std::fs::metadata(file_path)
            .map(|m| m.len())
            .unwrap_or(0);

        // Verify hash before deletion if configured
        if self.verify_before_delete {
            if let Some(expected) = expected_sha256 {
                info!("Verification enabled — checking SHA-256 before delete");
                let config = std::sync::Arc::new(crate::config::ScanningConfig::default());
                let engine = HashEngine::new(config);
                match engine.compute_sha256_only(file_path) {
                    Ok(actual) if actual == expected => {
                        info!("SHA-256 verified: {} ✓", actual);
                    }
                    Ok(actual) => {
                        error!(
                            "SHA-256 MISMATCH! Expected {} but got {}. REFUSING to delete {}",
                            expected, actual, file_path.display()
                        );
                        return Ok(RemediationResult {
                            action: "delete".to_string(),
                            file_path: file_path.to_string_lossy().to_string(),
                            source_path: None,
                            strategy: "aborted_hash_mismatch".to_string(),
                            fs_type: String::new(),
                            space_recovered: 0,
                            success: false,
                            error: Some(format!("Hash mismatch: expected {} got {}", expected, actual)),
                        });
                    }
                    Err(e) => {
                        error!("Failed to verify hash: {}. REFUSING to delete.", e);
                        return Ok(RemediationResult {
                            action: "delete".to_string(),
                            file_path: file_path.to_string_lossy().to_string(),
                            source_path: None,
                            strategy: "aborted_verify_error".to_string(),
                            fs_type: String::new(),
                            space_recovered: 0,
                            success: false,
                            error: Some(format!("Verification failed: {}", e)),
                        });
                    }
                }
            }
        }

        // Perform the delete
        std::fs::remove_file(file_path)
            .context("Failed to delete file")?;

        info!("Deleted: {} ({} bytes recovered)", file_path.display(), size);
        Ok(RemediationResult {
            action: "delete".to_string(),
            file_path: file_path.to_string_lossy().to_string(),
            source_path: None,
            strategy: "permanent".to_string(),
            fs_type: String::new(),
            space_recovered: size,
            success: true,
            error: None,
        })
    }

    /// Clean quarantine — remove files older than grace period.
    pub fn clean_quarantine(&self) -> Result<u64> {
        let mut cleaned = 0u64;
        let grace_secs = (self.grace_period_hours as u64) * 3600;
        let now = SystemTime::now();

        if let Ok(entries) = std::fs::read_dir(&self.quarantine_path) {
            for entry in entries.flatten() {
                if let Ok(meta) = entry.metadata() {
                    if let Ok(modified) = meta.modified() {
                        if let Ok(age) = now.duration_since(modified) {
                            if age.as_secs() > grace_secs {
                                let path = entry.path();
                                let size = meta.len();
                                if std::fs::remove_file(&path).is_ok() {
                                    info!("Quarantine cleanup: removed {} ({} bytes, {} hours old)",
                                        path.display(), size, age.as_secs() / 3600);
                                    cleaned += size;
                                }
                            }
                        }
                    }
                }
            }
        }

        info!("Quarantine cleanup complete: {} bytes recovered", cleaned);
        Ok(cleaned)
    }

    /// Generate a funny but honest warning message for dangerous actions.
    /// The GillSystems way — we warn you, but we never stop you.
    pub fn warn_user(action: &str) -> String {
        match action {
            "delete_all" => "⚠️ WHOA THERE COWBOY! You're about to delete EVERYTHING! This is like nuking your digital life from orbit. I say do it, don't be a little...crazy.".to_string(),
            "format_drive" => "🔥 NOW the question is - which format is best? Formatting drives is permanent! Like, really permanent. Don't come crying to me. Think, McFly, think!".to_string(),
            "quarantine_all" => "📦 Moving ALL files to quarantine. They'll be safe there (sorta, lol), but your folders are about to look like a ghost town. Sure about this?".to_string(),
            _ => format!("⚠️ About to {}: This might be a bad idea. But hey, it's your system — Power to the People!", action),
        }
    }
}
