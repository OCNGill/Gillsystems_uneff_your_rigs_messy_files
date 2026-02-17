// Gillsystems_uneff_your_rigs_messy_files — Platform Integration Module
// Created by: Master Dev 4 (Network & Platform Engineer)
// Philosophy: Power to the People — full admin, full speed, every platform.
//
// Cross-platform service registration and drive enumeration.
// Windows (Service + Run Key), Linux (systemd), macOS (LaunchAgent).
// Admin/sudo REQUIRED — no permission gatekeeping.

use anyhow::Result;
use tracing::info;

// Platform-specific submodules
#[cfg(unix)]
pub mod unix;

#[cfg(windows)]
pub mod windows;

/// Cross-platform module — compile-time platform selection.
/// Each platform gets its own service registration and drive enumeration.

#[cfg(unix)]
pub mod unix {
    use super::*;

    /// Register as a systemd user service (Linux) or LaunchAgent (macOS).
    /// Requires root/sudo — no half measures.
    pub fn register_service() -> Result<()> {
        info!("Registering Unix service — full admin assumed");
        // TODO: DEVELOP phase — generate systemd unit file
        // TODO: DEVELOP phase — generate macOS LaunchAgent plist
        // TODO: DEVELOP phase — enable and start service
        Ok(())
    }

    /// Enumerate all mounted filesystems on Unix.
    /// Scans /proc/mounts (Linux) or diskutil (macOS).
    pub fn get_all_mounts() -> Result<Vec<String>> {
        // TODO: DEVELOP phase — read /proc/mounts or run diskutil
        Ok(Vec::new())
    }
}

#[cfg(windows)]
pub mod windows {
    use super::*;

    /// Register as a Windows Service + Run Key for auto-start.
    /// Auto-elevation via UAC — no prompts, no friction.
    pub fn register_service() -> Result<()> {
        info!("Registering Windows service — full admin assumed");
        // TODO: DEVELOP phase — register Windows Service via windows-service crate
        // TODO: DEVELOP phase — set HKCU Run key for GUI auto-start
        Ok(())
    }

    /// Enumerate all drives on Windows (C:, D:, USB, network shares).
    /// Uses Win32 GetLogicalDriveStrings + GetDriveType.
    pub fn get_all_drives() -> Result<Vec<String>> {
        // TODO: DEVELOP phase — Win32 API drive enumeration
        Ok(Vec::new())
    }
}
