//! # Boot Screen Module — Launcher UI with Permission Elevation
//!
//! The first screen users see when launching the application.
//! Provides three main options:
//! 1. **GUI Mode**: Full Windows 7 Aero interface for duplicate detection
//! 2. **Service Mode**: Background gRPC peer listening for cluster commands
//! 3. **SMB Server Setup**: Configure network share for cross-system access
//!
//! This module also handles permission elevation:
//! - Windows: Requests admin rights via UAC
//! - Linux: Checks for sudo/root and re-executes if needed
//! - macOS: Prompts for Full Disk Access (if not using GUI)
//!
//! ## Philosophy
//! Permission gatekeeping ENDS AT STARTUP.
//! Once inside, assume full admin. No interruptions.

use anyhow::{Context, Result};
use eframe::egui;
use std::path::PathBuf;
use tracing::{info, error, warn};

/// Boot screen mode selection
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BootMode {
    /// Show the boot/launcher screen
    ShowBootScreen,
    /// Launch full GUI with duplicate detection
    LaunchGUI,
    /// Start as background service
    LaunchService,
    /// Configure SMB server
    SetupSMB,
}

/// Boot screen state and UI
pub struct BootScreen {
    /// Current selected mode (None = no selection yet)
    selected_mode: Option<BootMode>,
    /// Status message (e.g., "Checking permissions...", "Admin rights granted")
    status_message: String,
    /// Is permission check in progress?
    checking_permissions: bool,
    /// Did permission check succeed?
    has_permissions: bool,
    /// SMB setup options
    smb_config: SMBConfig,
    /// Gillsystems branded background texture
    background_texture: Option<egui::TextureHandle>,
}

/// SMB server configuration options
#[derive(Debug, Clone)]
pub struct SMBConfig {
    /// Enable SMB server on this node?
    pub enable_smb: bool,
    /// Share name (default: "uneff-rigs")
    pub share_name: String,
    /// Share path (default: current user's temp directory)
    pub share_path: String,
    /// Restrict to localhost only?
    pub localhost_only: bool,
    /// Available drives on system
    pub available_drives: Vec<String>,
    /// Selected drives for sharing
    pub selected_drives: Vec<String>,
    /// Use all drives?
    pub select_all_drives: bool,
    /// Share in combined folder or per-drive?
    pub combined_share: bool,
    /// Currently showing drive selection UI?
    pub showing_drive_selection: bool,
}

impl Default for SMBConfig {
    fn default() -> Self {
        Self {
            enable_smb: false,
            share_name: "uneff-rigs".to_string(),
            share_path: std::env::temp_dir().to_string_lossy().to_string(),
            localhost_only: true,
            available_drives: Vec::new(),
            selected_drives: Vec::new(),
            select_all_drives: true,
            combined_share: false,
            showing_drive_selection: false,
        }
    }
}

impl Default for BootScreen {
    fn default() -> Self {
        Self {
            selected_mode: None,
            status_message: "Checking permissions...".to_string(),
            checking_permissions: true,
            has_permissions: false,
            smb_config: SMBConfig::default(),
            background_texture: None,
        }
    }
}

impl BootScreen {
    /// Create a new boot screen instance
    pub fn new() -> Self {
        Self::default()
    }

    /// Check if current process has required elevated privileges
    ///
    /// # Returns
    /// - `Ok(true)`: Has admin/sudo/root access
    /// - `Ok(false)`: No elevated access (need re-execution)
    /// - `Err`: Permission check failed
    #[cfg(target_os = "windows")]
    pub fn check_permissions() -> Result<bool> {
        // Check if running with admin rights on Windows
        // Try to access HKLM registry — only works with admin
        match winreg::RegKey::predef(winreg::enums::HKEY_LOCAL_MACHINE).open_subkey("SOFTWARE") {
            Ok(_) => {
                info!("✅ Admin rights verified (HKLM access successful)");
                Ok(true)
            }
            Err(_) => {
                info!("❌ No admin rights (HKLM access denied)");
                Ok(false)
            }
        }
    }

    #[cfg(target_os = "linux")]
    pub fn check_permissions() -> Result<bool> {
        // Check if running as root (uid 0)
        let uid = unsafe { libc::getuid() };
        Ok(uid == 0)
    }

    #[cfg(target_os = "macos")]
    pub fn check_permissions() -> Result<bool> {
        // Check if running as root on macOS
        let uid = unsafe { libc::getuid() };
        Ok(uid == 0)
    }

    /// Request elevated permissions (admin/sudo)
    ///
    /// # Platform-Specific Behavior
    /// - **Windows**: Uses `runas` to re-execute with UAC elevation
    /// - **Linux**: Re-executes with `sudo`
    /// - **macOS**: Re-executes with `sudo` (Full Disk Access still manual)
    #[cfg(target_os = "windows")]
    pub fn request_elevation() -> Result<()> {
        use std::process::Command;

        let exe_path = std::env::current_exe()?;
        let exe_path_str = exe_path.to_string_lossy();

        // Re-execute with runas for UAC elevation
        let status = Command::new("cmd")
            .args(&["/C", "runas", "/user:Administrator", &format!("\"{}\"", exe_path_str)])
            .spawn()?
            .wait()
            .context("Failed to execute elevated process")?;

        if status.success() {
            info!("Process elevated to admin successfully");
            Ok(())
        } else {
            Err(anyhow::anyhow!("User denied UAC prompt or elevation failed"))
        }
    }

    #[cfg(target_os = "linux")]
    pub fn request_elevation() -> Result<()> {
        use std::process::Command;

        let exe_path = std::env::current_exe()?;
        let exe_path_str = exe_path.to_string_lossy();

        // Re-execute with sudo
        let status = Command::new("sudo")
            .arg(exe_path_str.as_ref())
            .spawn()?
            .wait()
            .context("Failed to execute sudo")?;

        if status.success() {
            info!("Process elevated to root successfully");
            Ok(())
        } else {
            Err(anyhow::anyhow!("Sudo elevation failed or user denied password prompt"))
        }
    }

    #[cfg(target_os = "macos")]
    pub fn request_elevation() -> Result<()> {
        use std::process::Command;

        let exe_path = std::env::current_exe()?;
        let exe_path_str = exe_path.to_string_lossy();

        // Re-execute with sudo
        let status = Command::new("sudo")
            .arg(exe_path_str.as_ref())
            .spawn()?
            .wait()
            .context("Failed to execute sudo")?;

        if status.success() {
            info!("Process elevated to root successfully");
            info!("Note: For full filesystem access, grant Full Disk Access in System Preferences → Security & Privacy");
            Ok(())
        } else {
            Err(anyhow::anyhow!("Sudo elevation failed or user denied password prompt"))
        }
    }

    /// Render the boot screen UI
    ///
    /// Returns `Some(BootMode)` if user selected an option, `None` otherwise
    pub fn show(&mut self, ctx: &egui::Context) -> Option<BootMode> {
        // Load Gillsystems branded background once
        if self.background_texture.is_none() {
            let image_bytes = include_bytes!("../assets/gillsystems_bg.png");
            if let Ok(img) = image::load_from_memory(image_bytes) {
                let rgba = img.to_rgba8();
                let (w, h) = rgba.dimensions();
                let pixels = rgba.into_raw();
                let color_image = egui::ColorImage::from_rgba_unmultiplied(
                    [w as usize, h as usize],
                    &pixels,
                );
                self.background_texture = Some(ctx.load_texture(
                    "boot_bg",
                    color_image,
                    egui::TextureOptions::LINEAR,
                ));
            }
        }

        // Paint background behind all boot screen content
        let screen_rect = ctx.screen_rect();
        let painter = ctx.layer_painter(egui::LayerId::background());
        if let Some(tex) = &self.background_texture {
            painter.image(
                tex.id(),
                screen_rect,
                egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                egui::Color32::WHITE,
            );
        }

        // If checking permissions, show status screen
        if self.checking_permissions {
            return self.show_permission_check_screen(ctx);
        }

        // If no permissions, show permission error
        if !self.has_permissions {
            return self.show_permission_error_screen(ctx);
        }

        // Otherwise show mode selection
        self.show_mode_selection(ctx)
    }

    /// Show the permission checking screen
    fn show_permission_check_screen(&mut self, ctx: &egui::Context) -> Option<BootMode> {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.vertical_centered(|ui| {
                ui.add_space(100.0);

                ui.heading("🔐 Checking Permissions");
                ui.add_space(20.0);

                ui.label("Verifying elevated access rights...");
                ui.add_space(20.0);

                // Spinner animation (simple)
                ui.label("⏳ Please wait...");

                ui.add_space(40.0);

                // Check permissions
                match Self::check_permissions() {
                    Ok(true) => {
                        self.checking_permissions = false;
                        self.has_permissions = true;
                        self.status_message = "✅ Admin/Root access confirmed".to_string();
                        info!("Permission check successful: has admin/root");
                    }
                    Ok(false) => {
                        self.checking_permissions = false;
                        self.has_permissions = false;
                        self.status_message = "❌ Not running with admin/root privileges".to_string();
                        warn!("No elevated privileges detected. Will attempt re-execution.");
                    }
                    Err(e) => {
                        self.checking_permissions = false;
                        self.has_permissions = false;
                        self.status_message = format!("❌ Permission check failed: {}", e);
                        error!("Permission check error: {}", e);
                    }
                }
            });
        });

        None
    }

    /// Show permission error and elevation request
    fn show_permission_error_screen(&mut self, ctx: &egui::Context) -> Option<BootMode> {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.vertical_centered(|ui| {
                ui.add_space(80.0);

                ui.heading("⚠️ Elevation Required");
                ui.add_space(20.0);

                ui.label("This application requires administrator/root privileges to:");
                ui.label("• Scan all filesystems");
                ui.label("• Access protected directories");
                ui.label("• Perform remediation operations");

                ui.add_space(30.0);

                if ui.button("📤 Request Elevated Access").clicked() {
                    // Try to elevate
                    match Self::request_elevation() {
                        Ok(_) => {
                            info!("Elevation request successful");
                            // After elevation, app should restart — exit here
                            std::process::exit(0);
                        }
                        Err(e) => {
                            error!("Elevation failed: {}", e);
                            self.status_message = format!("Elevation failed: {}", e);
                        }
                    }
                }

                ui.add_space(10.0);

                if ui.button("❌ Cancel").clicked() {
                    std::process::exit(1);
                }

                ui.add_space(40.0);
                ui.label(&self.status_message);
            });
        });

        None
    }

    /// Show the main mode selection screen
    fn show_mode_selection(&mut self, ctx: &egui::Context) -> Option<BootMode> {
        let mut selected_mode = None;
        
        // Check if we should show drive selection for SMB
        if self.smb_config.showing_drive_selection {
            return self.show_smb_drive_selection(ctx);
        }
        
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.vertical_centered(|ui| {
                ui.add_space(30.0);

                ui.heading("🚀 Gillsystems — Un-eff Your Rigs");
                ui.label("v0.4.0 • Document Complete");

                ui.add_space(40.0);

                ui.label("What would you like to do?");

                ui.add_space(30.0);

                // GUI Mode Button
                if ui.button("🎮 Launch GUI\n\nFull duplicate detection with Windows 7 Aero interface")
                    .on_hover_text("Start the full GUI with duplicate detection")
                    .clicked() {
                    self.selected_mode = Some(BootMode::LaunchGUI);
                    selected_mode = Some(BootMode::LaunchGUI);
                }

                ui.add_space(15.0);

                // Service Mode Button
                if ui.button("⚙️ Start Service\n\nRun as background peer (gRPC on port 50051)")
                    .on_hover_text("Start as background service for cluster operations")
                    .clicked() {
                    self.selected_mode = Some(BootMode::LaunchService);
                    selected_mode = Some(BootMode::LaunchService);
                }

                ui.add_space(15.0);

                // SMB Setup Button
                if ui.button("🌐 Configure SMB\n\nSet up network share for cluster access")
                    .on_hover_text("Configure SMB network share for cross-node access")
                    .clicked() {
                    // Load available drives
                    if let Ok(drives) = crate::smb_server::SMBServer::get_available_drives() {
                        self.smb_config.available_drives = drives;
                        self.smb_config.selected_drives = self.smb_config.available_drives.clone();
                    }
                    self.smb_config.showing_drive_selection = true;
                }

                ui.add_space(40.0);

                ui.label("✅ Admin/Root access verified — Full power enabled!");
            });
        });

        selected_mode
    }

    /// SMB Drive Selection Screen
    fn show_smb_drive_selection(&mut self, ctx: &egui::Context) -> Option<BootMode> {
        let mut launch_smb = false;
        let mut go_back = false;
        
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.vertical_centered(|ui| {
                ui.add_space(20.0);

                ui.heading("🌐 Configure SMB Sharing");
                ui.label("Select drives to share for network access");

                ui.add_space(20.0);

                // Select All / Select None
                ui.horizontal(|ui| {
                    if ui.button("📌 Select All").on_hover_text("Share all available drives").clicked() {
                        self.smb_config.select_all_drives = true;
                        self.smb_config.selected_drives = self.smb_config.available_drives.clone();
                    }
                    
                    if ui.button("🚫 Select None").on_hover_text("Share no drives").clicked() {
                        self.smb_config.select_all_drives = false;
                        self.smb_config.selected_drives.clear();
                    }
                });

                ui.add_space(15.0);

                ui.label("Available Drives/Mounts:");

                // Drive checkboxes
                ui.indent("drives", |ui| {
                    for drive in &self.smb_config.available_drives {
                        let mut is_selected = self.smb_config.selected_drives.contains(drive);
                        let original_selected = is_selected;
                        
                        ui.checkbox(&mut is_selected, format!("  📂 {}", drive));
                        
                        if is_selected && !original_selected {
                            self.smb_config.selected_drives.push(drive.clone());
                        } else if !is_selected && original_selected {
                            self.smb_config.selected_drives.retain(|d| d != drive);
                        }
                    }
                });

                ui.add_space(15.0);

                ui.separator();
                ui.label(format!(
                    "Sharing: {} drive(s) — Each will have FULL read/write access",
                    self.smb_config.selected_drives.len()
                ));
                if !self.smb_config.selected_drives.is_empty() {
                    ui.label(format!("  {}", self.smb_config.selected_drives.join(", ")));
                }
                ui.separator();

                ui.add_space(15.0);

                ui.horizontal(|ui| {
                    if ui.button("✅ Launch SMB Server").on_hover_text("Start SMB in separate window with selected drives").clicked() {
                        if !self.smb_config.selected_drives.is_empty() {
                            launch_smb = true;
                        }
                    }

                    if ui.button("⬅️ Back").on_hover_text("Return to mode selection").clicked() {
                        go_back = true;
                    }
                });

                ui.add_space(20.0);

                ui.label("⚠️ Selected drives will be shared with FULL read/write permissions");
            });
        });

        if launch_smb {
            // Launch SMB in separate process with unique share names per drive
            // Each drive gets its own share: uneff-rigs-{HOSTNAME}-{DRIVE}
            for drive in &self.smb_config.selected_drives {
                // Generate unique share name for this drive on this node
                let share_name = match crate::smb_server::SMBServer::generate_unique_share_name(drive) {
                    Ok(name) => name,
                    Err(e) => {
                        error!("Failed to generate share name for {}: {}", drive, e);
                        continue;
                    }
                };
                
                let smb = crate::smb_server::SMBServer::new(
                    share_name.clone(),
                    PathBuf::from(&self.smb_config.share_path),
                    self.smb_config.localhost_only,
                    vec![drive.clone()],
                );
                
                if let Err(e) = smb.launch_separate_process() {
                    error!("Failed to launch SMB for drive {}: {}", drive, e);
                } else {
                    info!("SMB server launched for {}: {}", drive, share_name);
                }
            }
            
            // Keep boot screen open, reset to mode selection
            self.smb_config.showing_drive_selection = false;
        }

        if go_back {
            self.smb_config.showing_drive_selection = false;
        }

        None
    }
}

/// Standalone boot screen app (for testing)
pub struct BootScreenApp {
    boot_screen: BootScreen,
    selected_mode: Option<BootMode>,
}

impl BootScreenApp {
    pub fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        Self {
            boot_screen: BootScreen::new(),
            selected_mode: None,
        }
    }
}

impl eframe::App for BootScreenApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if let Some(mode) = self.boot_screen.show(ctx) {
            self.selected_mode = Some(mode);
            // In real usage, this would signal to main.rs to proceed with the selected mode
        }
    }
}

/// Run the boot screen as a standalone window
pub async fn show_boot_screen() -> Result<Option<BootMode>> {
    // For now, return the selected mode
    // In production, this would run the egui window and wait for user selection
    Ok(Some(BootMode::LaunchGUI))
}
