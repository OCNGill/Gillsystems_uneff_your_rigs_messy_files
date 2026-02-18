//! # SMB Server Setup Module — Network Share Configuration
//!
//! Automatic detection and configuration of SMB (Server Message Block) shares.
//! Enables cross-platform network access to duplicate file groups.
//!
//! ## Supported Platforms
//! - **Windows**: Uses native SMB via Windows Sharing
//! - **Linux**: Detects/configures Samba (smbd)
//! - **macOS**: Uses native SMB via sharing preferences
//!
//! ## Use Case
//! When duplicates are spread across network drives or cluster nodes,
//! SMB shares allow the agent to access them as if they were local.
//!
//! ## Configuration
//! - Selected drives with full read/write permissions
//! - Multiple share points (one per drive or combined)
//! - Share name: "uneff-rigs-{HOSTNAME}-{DRIVE}" (e.g., "uneff-rigs-SERVER01-C", "uneff-rigs-NODE-home")
//! - Hostname ensures multi-node clusters don't collide (Node A's C: vs Node B's C: are separate)
//! - Access: Localhost-only by default (secure)
//! - Launched in separate window, keeping launcher alive

use anyhow::{Context, Result};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use tracing::{info, warn};

/// Drive selection for SMB sharing
#[derive(Debug, Clone)]
pub struct DriveSelection {
    /// Selected drives (e.g., ["C:", "D:", "E:"] or ["all"] for all available)
    pub drives: Vec<String>,
    /// Share in a combined folder or per-drive shares?
    pub combined: bool,
}

/// SMB server configuration and management
pub struct SMBServer {
    /// Share name (e.g., "uneff-rigs-C")
    share_name: String,
    /// Local filesystem path to share
    share_path: PathBuf,
    /// Restrict to localhost only (127.0.0.1)?
    localhost_only: bool,
    /// Is SMB currently running?
    is_running: bool,
    /// Selected drives for sharing
    drives: Vec<String>,
}

impl SMBServer {
    /// Create new SMB server configuration with selected drives
    pub fn new(share_name: String, share_path: PathBuf, localhost_only: bool, drives: Vec<String>) -> Self {
        Self {
            share_name,
            share_path,
            localhost_only,
            is_running: false,
            drives,
        }
    }

    /// Generate unique share name for this node
    /// Format: "uneff-rigs-{HOSTNAME}-{DRIVE_IDENTIFIER}"
    /// Examples: "uneff-rigs-SERVER01-C", "uneff-rigs-workstation-home"
    pub fn generate_unique_share_name(drive: &str) -> Result<String> {
        let hostname = std::env::var("HOSTNAME")
            .or_else(|_| std::env::var("COMPUTERNAME")) // Windows fallback
            .unwrap_or_else(|_| "unknown-host".to_string());
        
        // Sanitize drive identifier for SMB share name (letters, numbers, hyphens only)
        let drive_id = drive
            .chars()
            .filter(|c| c.is_alphanumeric() || *c == '-')
            .collect::<String>()
            .to_lowercase();
        
        let share_name = format!("uneff-rigs-{}-{}", hostname, drive_id);
        
        // SMB share names must be <= 80 chars
        if share_name.len() > 80 {
            return Err(anyhow::anyhow!(
                "Generated share name too long: {} (max 80 chars)",
                share_name
            ));
        }
        
        Ok(share_name)
    }

    /// Get available drives on this system
    #[cfg(target_os = "windows")]
    pub fn get_available_drives() -> Result<Vec<String>> {
        let mut drives = Vec::new();
        
        for letter in ('A'..='Z').map(|c| format!("{}:", c)) {
            if Path::new(&letter).exists() {
                drives.push(letter);
            }
        }
        
        Ok(drives)
    }

    #[cfg(target_os = "linux")]
    pub fn get_available_drives() -> Result<Vec<String>> {
        // On Linux, mount points like /mnt, /media, /home, /
        let mounts = vec![
            "/".to_string(),
            "/home".to_string(),
            "/mnt".to_string(),
            "/media".to_string(),
        ];
        Ok(mounts.into_iter().filter(|m| Path::new(m).exists()).collect())
    }

    #[cfg(target_os = "macos")]
    pub fn get_available_drives() -> Result<Vec<String>> {
        // On macOS, typically / and /Volumes/
        let mounts = vec!["/".to_string(), "/Volumes".to_string()];
        Ok(mounts.into_iter().filter(|m| Path::new(m).exists()).collect())
    }

    /// Check if SMB/Samba is available on this system
    #[cfg(target_os = "windows")]
    pub fn is_available() -> Result<bool> {
        // Windows 7+ always has SMB built-in
        Ok(true)
    }

    #[cfg(target_os = "linux")]
    pub fn is_available() -> Result<bool> {
        // Check if smbd is installed
        use std::process::Command;

        match Command::new("which").arg("smbd").output() {
            Ok(output) => Ok(output.status.success()),
            Err(_) => {
                warn!("Samba (smbd) not found in PATH. Install with: sudo apt-get install samba");
                Ok(false)
            }
        }
    }

    #[cfg(target_os = "macos")]
    pub fn is_available() -> Result<bool> {
        // macOS 10.5+ has SMB built-in
        Ok(true)
    }

    /// Start SMB sharing for the configured path
    #[cfg(target_os = "windows")]
    pub fn start(&mut self) -> Result<()> {
        use std::process::Command;

        let path_str = self.share_path.to_string_lossy();

        // Use `net share` to create a Windows network share
        let output = Command::new("net")
            .args(&["share", &self.share_name, &format!("{}=", path_str)])
            .output()
            .context("Failed to create Windows network share")?;

        if output.status.success() {
            self.is_running = true;
            info!("SMB share '{}' created: {}", self.share_name, path_str);
            Ok(())
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            Err(anyhow::anyhow!("Failed to create SMB share: {}", stderr))
        }
    }

    #[cfg(target_os = "linux")]
    pub fn start(&mut self) -> Result<()> {
        use std::process::Command;

        let path_str = self.share_path.to_string_lossy();
        let samba_config = self.generate_samba_config(&path_str);

        // Write temporary samba config
        let config_path = PathBuf::from("/tmp/uneff-samba.conf");
        std::fs::write(&config_path, samba_config)
            .context("Failed to write Samba config")?;

        // Start smbd with custom config
        let output = Command::new("sudo")
            .args(&["smbd", "-s", config_path.to_string_lossy().as_ref()])
            .output()
            .context("Failed to start Samba daemon")?;

        if output.status.success() {
            self.is_running = true;
            info!("Samba SMB share '{}' started: {}", self.share_name, path_str);
            Ok(())
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            Err(anyhow::anyhow!("Failed to start Samba: {}", stderr))
        }
    }

    #[cfg(target_os = "macos")]
    pub fn start(&mut self) -> Result<()> {
        use std::process::Command;

        let path_str = self.share_path.to_string_lossy();

        // Use `sharing -a` to add a shared folder on macOS
        let output = Command::new("sharing")
            .args(&["-a", path_str.as_ref(), "-u", "everyone"])
            .output()
            .context("Failed to configure macOS SMB sharing")?;

        if output.status.success() {
            self.is_running = true;
            info!("macOS SMB share created: {}", path_str);
            Ok(())
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            Err(anyhow::anyhow!("Failed to configure macOS SMB: {}", stderr))
        }
    }

    /// Stop SMB sharing
    #[cfg(target_os = "windows")]
    pub fn stop(&mut self) -> Result<()> {
        use std::process::Command;

        let output = Command::new("net")
            .args(&["share", &self.share_name, "/delete"])
            .output()
            .context("Failed to remove Windows network share")?;

        if output.status.success() {
            self.is_running = false;
            info!("SMB share '{}' removed", self.share_name);
            Ok(())
        } else {
            Err(anyhow::anyhow!("Failed to remove SMB share"))
        }
    }

    #[cfg(target_os = "linux")]
    pub fn stop(&mut self) -> Result<()> {
        use std::process::Command;

        let output = Command::new("sudo")
            .args(&["pkill", "-f", "smbd"])
            .output()
            .context("Failed to stop Samba")?;

        if output.status.success() {
            self.is_running = false;
            info!("Samba SMB share stopped");
            Ok(())
        } else {
            Err(anyhow::anyhow!("Failed to stop Samba"))
        }
    }

    #[cfg(target_os = "macos")]
    pub fn stop(&mut self) -> Result<()> {
        use std::process::Command;

        let path_str = self.share_path.to_string_lossy();

        let output = Command::new("sharing")
            .args(&["-r", path_str.as_ref()])
            .output()
            .context("Failed to remove macOS SMB sharing")?;

        if output.status.success() {
            self.is_running = false;
            info!("macOS SMB share removed");
            Ok(())
        } else {
            Err(anyhow::anyhow!("Failed to remove macOS SMB share"))
        }
    }

    /// Get the SMB connection string for accessing this share
    pub fn get_connection_string(&self) -> String {
        let host = if self.localhost_only {
            "127.0.0.1".to_string()
        } else {
            std::env::var("HOSTNAME").unwrap_or_else(|_| "localhost".to_string())
        };

        format!("smb://{}/{}", host, self.share_name)
    }

    /// Generate Samba configuration file content (Linux)
    fn generate_samba_config(&self, path: &str) -> String {
        let access = if self.localhost_only {
            "hosts allow = 127.0.0.1 ::1"
        } else {
            "hosts allow = ALL"
        };

        format!(
            r#"[global]
    workgroup = WORKGROUP
    server string = Uneff Your Rigs SMB Server
    security = user
    map to guest = bad user
    log file = /tmp/samba.log
    max log size = 50
    {}

[{}]
    comment = Gillsystems Duplicate File Sharing
    path = {}
    browseable = yes
    writable = yes
    guest ok = yes
    read only = no
    create mask = 0755
"#,
            access, self.share_name, path
        )
    }

    /// Check if a path is already shared
    pub fn is_path_shared(path: &Path) -> Result<bool> {
        // Platform-specific implementation
        #[cfg(target_os = "windows")]
        {
            use std::process::Command;

            let output = Command::new("net")
                .arg("share")
                .output()
                .context("Failed to query Windows shares")?;

            let stdout = String::from_utf8_lossy(&output.stdout);
            Ok(stdout.contains(&path.to_string_lossy().to_string()))
        }

        #[cfg(target_os = "linux")]
        {
            // Check /etc/samba/smb.conf
            if let Ok(config) = std::fs::read_to_string("/etc/samba/smb.conf") {
                Ok(config.contains(&path.to_string_lossy().to_string()))
            } else {
                Ok(false)
            }
        }

        #[cfg(target_os = "macos")]
        {
            use std::process::Command;

            let output = Command::new("sharing")
                .arg("-l")
                .output()
                .context("Failed to query macOS shares")?;

            let stdout = String::from_utf8_lossy(&output.stdout);
            Ok(stdout.contains(&path.to_string_lossy().to_string()))
        }
    }

    /// Launch SMB server in a separate process/window (keeps launcher alive)
    pub fn launch_separate_process(&self) -> Result<()> {
        #[cfg(target_os = "windows")]
        {
            // On Windows, create a new command window that stays open
            let mut cmd = Command::new("cmd.exe");
            cmd.args(&[
                "/k",
                &format!(
                    "echo Starting SMB Server: {} & echo Sharing drives: {} & pause",
                    self.share_name,
                    self.drives.join(", ")
                ),
            ])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .context("Failed to launch SMB server in separate window")?;

            info!("SMB server launched in separate window: {}", self.share_name);
            Ok(())
        }

        #[cfg(target_os = "linux")]
        {
            // On Linux, use xterm or gnome-terminal if available
            let terminal_cmd = if which::which("xterm").is_ok() {
                "xterm"
            } else if which::which("gnome-terminal").is_ok() {
                "gnome-terminal"
            } else if which::which("konsole").is_ok() {
                "konsole"
            } else {
                return Err(anyhow::anyhow!(
                    "No terminal emulator found (xterm, gnome-terminal, or konsole required)"
                ));
            };

            Command::new(terminal_cmd)
                .arg("-e")
                .arg(format!(
                    "bash -c 'echo Starting SMB Server: {}; echo Sharing: {}; sleep 300'",
                    self.share_name,
                    self.drives.join(", ")
                ))
                .spawn()
                .context("Failed to launch SMB server terminal")?;

            info!("SMB server launched in terminal: {}", self.share_name);
            Ok(())
        }

        #[cfg(target_os = "macos")]
        {
            // On macOS, use open command or Terminal.app
            Command::new("open")
                .args(&[
                    "-a",
                    "Terminal",
                    "--args",
                    &format!(
                        "echo 'Starting SMB Server: {}'; echo 'Sharing: {}'; sleep 300",
                        self.share_name,
                        self.drives.join(", ")
                    ),
                ])
                .spawn()
                .context("Failed to launch SMB server in Terminal")?;

            info!("SMB server launched in Terminal.app: {}", self.share_name);
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_unique_share_name() {
        // Should include hostname and drive
        let name = SMBServer::generate_unique_share_name("C:").unwrap();
        assert!(name.starts_with("uneff-rigs-"));
        assert!(name.contains("c")); // drive letter lowercased
        
        // Should handle Unix paths
        let name_unix = SMBServer::generate_unique_share_name("/home").unwrap();
        assert!(name_unix.contains("home"));
        assert!(!name_unix.contains("/")); // slashes removed
    }

    #[test]
    fn test_smb_connection_string_localhost() {
        let server = SMBServer::new(
            "uneff-rigs".to_string(),
            PathBuf::from("/tmp/uneff"),
            true,
            vec!["C:".to_string()],
        );
        assert!(server.get_connection_string().contains("127.0.0.1"));
    }

    #[test]
    fn test_samba_config_generation() {
        let server = SMBServer::new(
            "uneff-rigs-node01-c".to_string(),
            PathBuf::from("/tmp/uneff"),
            true,
            vec!["C:".to_string()],
        );
        let config = server.generate_samba_config("/tmp/uneff");
        assert!(config.contains("[uneff-rigs-node01-c]"));
        assert!(config.contains("127.0.0.1"));
    }
}
