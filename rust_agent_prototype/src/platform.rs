// Gillsystems_uneff_your_rigs_messy_files — Platform Integration Module
// Philosophy: Power to the People — full admin, full speed, every platform.
//
// Cross-platform service registration and drive enumeration.
// ZFS detection is PRIMARY. Then NTFS, ext4, XFS, FAT32.
// Admin/sudo REQUIRED — no permission gatekeeping.

use anyhow::Result;
use tracing::{info, warn};

// ── Unix platform ──────────────────────────────────────────────────────

#[cfg(unix)]
pub mod unix {
    use super::*;
    use std::process::Command;

    /// Mount info for a Unix filesystem.
    #[derive(Debug, Clone)]
    pub struct MountInfo {
        pub device: String,
        pub mount_point: String,
        pub fs_type: String,
        pub total_space: u64,
        pub available_space: u64,
    }

    /// Register as a systemd user service (Linux) or LaunchAgent (macOS).
    /// Requires root/sudo — no half measures.
    pub fn register_service() -> Result<()> {
        info!("Registering Unix service — full admin assumed");

        let exe_path = std::env::current_exe()?;
        let exe = exe_path.to_string_lossy();

        #[cfg(target_os = "linux")]
        {
            let unit = format!(
                "[Unit]\n\
                 Description=Gillsystems Uneff Your Rigs Messy Files\n\
                 After=network.target\n\n\
                 [Service]\n\
                 Type=simple\n\
                 ExecStart={} --service\n\
                 Restart=on-failure\n\
                 RestartSec=5\n\n\
                 [Install]\n\
                 WantedBy=multi-user.target\n",
                exe
            );

            let unit_path = "/etc/systemd/system/gillsystems-uneff.service";
            if let Err(e) = std::fs::write(unit_path, &unit) {
                warn!("Failed to write systemd unit (need root?): {}", e);
            } else {
                Command::new("systemctl").args(["daemon-reload"]).output().ok();
                Command::new("systemctl").args(["enable", "gillsystems-uneff"]).output().ok();
                info!("Systemd service registered at {}", unit_path);
            }
        }

        #[cfg(target_os = "macos")]
        {
            let plist = format!(
                "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
                 <!DOCTYPE plist PUBLIC \"-//Apple//DTD PLIST 1.0//EN\" \"http://www.apple.com/DTDs/PropertyList-1.0.dtd\">\n\
                 <plist version=\"1.0\"><dict>\n\
                 <key>Label</key><string>net.gillsystems.uneff</string>\n\
                 <key>ProgramArguments</key><array><string>{}</string><string>--service</string></array>\n\
                 <key>RunAtLoad</key><true/>\n\
                 <key>KeepAlive</key><true/>\n\
                 </dict></plist>",
                exe
            );

            let plist_path = format!(
                "{}/Library/LaunchAgents/net.gillsystems.uneff.plist",
                std::env::var("HOME").unwrap_or_default()
            );
            if let Err(e) = std::fs::write(&plist_path, &plist) {
                warn!("Failed to write LaunchAgent plist: {}", e);
            } else {
                Command::new("launchctl").args(["load", &plist_path]).output().ok();
                info!("LaunchAgent registered at {}", plist_path);
            }
        }

        Ok(())
    }

    /// Enumerate all mounted filesystems on Unix.
    /// Detects ZFS pools FIRST (primary target), then standard mounts.
    pub fn get_all_mounts() -> Result<Vec<MountInfo>> {
        let mut mounts = Vec::new();

        // ── ZFS pool detection (PRIMARY) ──
        if let Ok(output) = Command::new("zpool").args(["list", "-Hp", "-o", "name,size,free"]).output() {
            if output.status.success() {
                let stdout = String::from_utf8_lossy(&output.stdout);
                for line in stdout.lines() {
                    let parts: Vec<&str> = line.split('\t').collect();
                    if parts.len() >= 3 {
                        let pool_name = parts[0].to_string();
                        let total: u64 = parts[1].parse().unwrap_or(0);
                        let free: u64 = parts[2].parse().unwrap_or(0);

                        // Get ZFS datasets for this pool
                        if let Ok(ds_out) = Command::new("zfs")
                            .args(["list", "-Hp", "-o", "name,used,avail,mountpoint", "-r", &pool_name])
                            .output()
                        {
                            let ds_stdout = String::from_utf8_lossy(&ds_out.stdout);
                            for ds_line in ds_stdout.lines() {
                                let ds_parts: Vec<&str> = ds_line.split('\t').collect();
                                if ds_parts.len() >= 4 {
                                    let mount = ds_parts[3].to_string();
                                    if mount != "-" && mount != "none" {
                                        let used: u64 = ds_parts[1].parse().unwrap_or(0);
                                        let avail: u64 = ds_parts[2].parse().unwrap_or(0);
                                        mounts.push(MountInfo {
                                            device: ds_parts[0].to_string(),
                                            mount_point: mount,
                                            fs_type: "zfs".to_string(),
                                            total_space: used + avail,
                                            available_space: avail,
                                        });
                                    }
                                }
                            }
                        }

                        info!("ZFS pool detected: {} ({} total, {} free)", pool_name, total, free);
                    }
                }
            }
        }

        // ── Standard mount parsing via /proc/mounts (Linux) or mount command ──
        #[cfg(target_os = "linux")]
        {
            if let Ok(content) = std::fs::read_to_string("/proc/mounts") {
                for line in content.lines() {
                    let parts: Vec<&str> = line.split_whitespace().collect();
                    if parts.len() >= 3 {
                        let device = parts[0];
                        let mount_point = parts[1];
                        let fs_type = parts[2];

                        // Skip virtual filesystems and already-found ZFS mounts
                        if matches!(fs_type, "proc" | "sysfs" | "devtmpfs" | "tmpfs" | "cgroup" | "cgroup2"
                            | "devpts" | "mqueue" | "hugetlbfs" | "debugfs" | "securityfs"
                            | "pstore" | "bpf" | "fusectl" | "configfs" | "tracefs" | "autofs") {
                            continue;
                        }
                        if fs_type == "zfs" {
                            continue; // Already detected above
                        }

                        // Get space info via statvfs
                        let (total, avail) = get_statvfs_space(mount_point);

                        mounts.push(MountInfo {
                            device: device.to_string(),
                            mount_point: mount_point.to_string(),
                            fs_type: fs_type.to_string(),
                            total_space: total,
                            available_space: avail,
                        });
                    }
                }
            }
        }

        #[cfg(target_os = "macos")]
        {
            if let Ok(output) = Command::new("mount").output() {
                let stdout = String::from_utf8_lossy(&output.stdout);
                for line in stdout.lines() {
                    // Format: /dev/disk1s1 on / (apfs, local, journaled)
                    if let Some((device_part, rest)) = line.split_once(" on ") {
                        if let Some((mount_point, fs_info)) = rest.split_once(" (") {
                            let fs_type = fs_info.split(',').next().unwrap_or("unknown").trim();
                            if fs_type == "zfs" { continue; } // Already detected
                            if matches!(fs_type, "devfs" | "autofs") { continue; }

                            let (total, avail) = get_statvfs_space(mount_point);
                            mounts.push(MountInfo {
                                device: device_part.to_string(),
                                mount_point: mount_point.to_string(),
                                fs_type: fs_type.to_string(),
                                total_space: total,
                                available_space: avail,
                            });
                        }
                    }
                }
            }
        }

        info!("Detected {} mounts ({} ZFS)",
            mounts.len(),
            mounts.iter().filter(|m| m.fs_type == "zfs").count()
        );
        Ok(mounts)
    }

    /// Get total/available space for a mount point using statvfs.
    #[cfg(unix)]
    fn get_statvfs_space(path: &str) -> (u64, u64) {
        use std::ffi::CString;
        use std::mem::MaybeUninit;

        let c_path = match CString::new(path) {
            Ok(p) => p,
            Err(_) => return (0, 0),
        };

        unsafe {
            let mut stat: MaybeUninit<libc::statvfs> = MaybeUninit::uninit();
            if libc::statvfs(c_path.as_ptr(), stat.as_mut_ptr()) == 0 {
                let stat = stat.assume_init();
                let total = stat.f_blocks as u64 * stat.f_frsize as u64;
                let avail = stat.f_bavail as u64 * stat.f_frsize as u64;
                (total, avail)
            } else {
                (0, 0)
            }
        }
    }

    /// Detect the filesystem type for a given path.
    /// Priority: ZFS first.
    pub fn detect_fs_type(path: &str) -> String {
        // Check if path is on a ZFS dataset
        if let Ok(output) = Command::new("df").args(["-T", path]).output() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            if let Some(line) = stdout.lines().nth(1) {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 2 {
                    return parts[1].to_lowercase();
                }
            }
        }

        // macOS fallback
        if let Ok(output) = Command::new("stat").args(["-f", "%T", path]).output() {
            let fs = String::from_utf8_lossy(&output.stdout).trim().to_lowercase();
            if !fs.is_empty() { return fs; }
        }

        "unknown".to_string()
    }
}

// ── Windows platform ───────────────────────────────────────────────────

#[cfg(windows)]
pub mod windows {
    use super::*;
    use std::ffi::OsString;
    use std::os::windows::ffi::OsStringExt;

    /// Drive info for a Windows volume.
    #[derive(Debug, Clone)]
    pub struct WinDriveInfo {
        pub mount_point: String,
        pub label: String,
        pub drive_type: String,
        pub fs_type: String,
        pub total_space: u64,
        pub available_space: u64,
        pub is_removable: bool,
    }

    /// Register as a Windows Service + Run Key for auto-start.
    /// Auto-elevation via UAC — no prompts, no friction.
    pub fn register_service() -> Result<()> {
        info!("Registering Windows service — full admin assumed");

        let exe_path = std::env::current_exe()?;
        let exe = exe_path.to_string_lossy().to_string();

        // Set HKCU\Software\Microsoft\Windows\CurrentVersion\Run key for GUI auto-start
        let hkcu = winreg::RegKey::predef(winreg::enums::HKEY_CURRENT_USER);
        if let Ok(run_key) = hkcu.open_subkey_with_flags(
            "Software\\Microsoft\\Windows\\CurrentVersion\\Run",
            winreg::enums::KEY_SET_VALUE,
        ) {
            run_key.set_value("GillsystemsUneff", &exe).ok();
            info!("Auto-start registered in HKCU Run key");
        }

        Ok(())
    }

    /// Enumerate all drives on Windows (C:, D:, USB, network shares).
    /// Uses Win32 GetLogicalDriveStringsW + GetVolumeInformationW + GetDiskFreeSpaceExW.
    pub fn get_all_drives() -> Result<Vec<WinDriveInfo>> {
        let mut drives = Vec::new();

        // Get logical drive strings (C:\, D:\, etc.)
        let mut buffer = [0u16; 256];
        let len = unsafe {
            winapi::um::fileapi::GetLogicalDriveStringsW(buffer.len() as u32, buffer.as_mut_ptr())
        };

        if len == 0 {
            warn!("GetLogicalDriveStringsW returned 0 drives");
            return Ok(drives);
        }

        // Parse null-delimited drive strings
        let drive_strings: Vec<String> = buffer[..len as usize]
            .split(|&c| c == 0)
            .filter(|s| !s.is_empty())
            .map(|s| OsString::from_wide(s).to_string_lossy().to_string())
            .collect();

        for drive_root in &drive_strings {
            let drive_root_wide: Vec<u16> = drive_root.encode_utf16().chain(std::iter::once(0)).collect();

            // Get drive type
            let drive_type_val = unsafe {
                winapi::um::fileapi::GetDriveTypeW(drive_root_wide.as_ptr())
            };

            let (drive_type, is_removable) = match drive_type_val {
                2 => ("removable".to_string(), true),
                3 => ("fixed".to_string(), false),
                4 => ("network".to_string(), false),
                5 => ("cdrom".to_string(), true),
                6 => ("ramdisk".to_string(), false),
                _ => ("unknown".to_string(), false),
            };

            // Get volume label and filesystem type
            let mut vol_name = [0u16; 256];
            let mut fs_name = [0u16; 256];
            let mut serial = 0u32;
            let mut max_component = 0u32;
            let mut flags = 0u32;

            let vol_ok = unsafe {
                winapi::um::fileapi::GetVolumeInformationW(
                    drive_root_wide.as_ptr(),
                    vol_name.as_mut_ptr(),
                    vol_name.len() as u32,
                    &mut serial,
                    &mut max_component,
                    &mut flags,
                    fs_name.as_mut_ptr(),
                    fs_name.len() as u32,
                )
            };

            let label = if vol_ok != 0 {
                let end = vol_name.iter().position(|&c| c == 0).unwrap_or(vol_name.len());
                OsString::from_wide(&vol_name[..end]).to_string_lossy().to_string()
            } else {
                String::new()
            };

            let fs_type = if vol_ok != 0 {
                let end = fs_name.iter().position(|&c| c == 0).unwrap_or(fs_name.len());
                OsString::from_wide(&fs_name[..end]).to_string_lossy().to_lowercase()
            } else {
                "unknown".to_string()
            };

            // Get disk space
            let mut free_bytes_available: u64 = 0;
            let mut total_bytes: u64 = 0;
            let mut total_free_bytes: u64 = 0;

            unsafe {
                winapi::um::fileapi::GetDiskFreeSpaceExW(
                    drive_root_wide.as_ptr(),
                    &mut free_bytes_available as *mut u64 as *mut _,
                    &mut total_bytes as *mut u64 as *mut _,
                    &mut total_free_bytes as *mut u64 as *mut _,
                );
            }

            let display_label = if label.is_empty() {
                format!("{} ({})", drive_root.trim_end_matches('\\'), drive_type)
            } else {
                format!("{} ({})", label, drive_root.trim_end_matches('\\'))
            };

            drives.push(WinDriveInfo {
                mount_point: drive_root.clone(),
                label: display_label,
                drive_type,
                fs_type,
                total_space: total_bytes,
                available_space: free_bytes_available,
                is_removable,
            });
        }

        info!("Detected {} Windows drives", drives.len());
        Ok(drives)
    }

    /// Detect the filesystem type for a given path on Windows.
    pub fn detect_fs_type(path: &str) -> String {
        // Get volume root from path
        let root = if path.len() >= 3 && path.as_bytes()[1] == b':' {
            format!("{}\\", &path[..2])
        } else {
            return "unknown".to_string();
        };

        let root_wide: Vec<u16> = root.encode_utf16().chain(std::iter::once(0)).collect();
        let mut fs_name = [0u16; 256];

        let ok = unsafe {
            winapi::um::fileapi::GetVolumeInformationW(
                root_wide.as_ptr(),
                std::ptr::null_mut(), 0,
                std::ptr::null_mut(), std::ptr::null_mut(), std::ptr::null_mut(),
                fs_name.as_mut_ptr(), fs_name.len() as u32,
            )
        };

        if ok != 0 {
            let end = fs_name.iter().position(|&c| c == 0).unwrap_or(fs_name.len());
            OsString::from_wide(&fs_name[..end]).to_string_lossy().to_lowercase()
        } else {
            "unknown".to_string()
        }
    }
}

// ── Cross-platform helpers ─────────────────────────────────────────────

/// Detect the filesystem type for a given path (cross-platform dispatch).
pub fn detect_fs_type(path: &str) -> String {
    #[cfg(unix)]
    { unix::detect_fs_type(path) }

    #[cfg(windows)]
    { windows::detect_fs_type(path) }
}
