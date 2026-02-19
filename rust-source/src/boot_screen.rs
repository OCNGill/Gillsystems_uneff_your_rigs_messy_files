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
use std::sync::{Arc, Mutex};
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
            let image_bytes = include_bytes!("../assets/Gillsystems_background.png");
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

// ── Matrix green palette for the boot launcher ────────────────────────────────
const BOOT_GREEN:      egui::Color32 = egui::Color32::from_rgb(0,   255,  65);
const BOOT_GREEN_DIM:  egui::Color32 = egui::Color32::from_rgb(0,   200,  45);
const BOOT_GREEN_GLOW: egui::Color32 = egui::Color32::from_rgb(57,  255,  20);
const BOOT_GREEN_DARK: egui::Color32 = egui::Color32::from_rgb(0,    80,  15);
const BOOT_BG:         egui::Color32 = egui::Color32::from_rgb(5,   10,   5);

/// Real launcher window — three big Matrix-green mode buttons.
/// Closes itself the moment the user clicks one; returns the selected BootMode.
struct BootLauncher {
    chosen: Arc<Mutex<Option<BootMode>>>,
    background_texture:      Option<egui::TextureHandle>,
    background_texture_size: Option<[usize; 2]>,
    footer_texture:          Option<egui::TextureHandle>,
    footer_texture_size:     Option<[usize; 2]>,
}

impl BootLauncher {
    fn new(chosen: Arc<Mutex<Option<BootMode>>>) -> Self {
        Self {
            chosen,
            background_texture: None,
            background_texture_size: None,
            footer_texture: None,
            footer_texture_size: None,
        }
    }
    fn select(&self, ctx: &egui::Context, mode: BootMode) {
        *self.chosen.lock().unwrap() = Some(mode);
        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
    }

    #[cfg(windows)]
    fn minimize_window_native(&self) {
        unsafe {
            let hwnd = winapi::um::winuser::GetForegroundWindow();
            if !hwnd.is_null() {
                winapi::um::winuser::ShowWindow(hwnd, winapi::um::winuser::SW_MINIMIZE);
            }
        }
    }

    #[cfg(not(windows))]
    fn minimize_window_native(&self) {}
}

impl eframe::App for BootLauncher {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // ── Matrix Green style ────────────────────────────────────────────────
        {
            let mut style = (*ctx.style()).clone();
            style.visuals.panel_fill  = egui::Color32::BLACK;
            style.visuals.window_fill = egui::Color32::BLACK;
            style.visuals.override_text_color = Some(BOOT_GREEN);
            style.visuals.widgets.noninteractive.bg_fill      = egui::Color32::BLACK;
            style.visuals.widgets.noninteractive.weak_bg_fill = egui::Color32::BLACK;
            style.visuals.widgets.inactive.bg_fill      = egui::Color32::BLACK;
            style.visuals.widgets.inactive.weak_bg_fill = egui::Color32::BLACK;  // CRITICAL
            style.visuals.widgets.inactive.fg_stroke =
                egui::Stroke::new(1.0, BOOT_GREEN_DIM);
            style.visuals.widgets.inactive.bg_stroke =
                egui::Stroke::new(0.8, BOOT_GREEN_DARK);
            style.visuals.widgets.inactive.rounding = egui::Rounding::same(2.0);
            style.visuals.widgets.hovered.bg_fill =
                egui::Color32::from_rgba_unmultiplied(0, 60, 15, 220);
            style.visuals.widgets.hovered.weak_bg_fill =
                egui::Color32::from_rgba_unmultiplied(0, 60, 15, 220);
            style.visuals.widgets.hovered.fg_stroke =
                egui::Stroke::new(1.5, BOOT_GREEN);
            style.visuals.widgets.hovered.bg_stroke =
                egui::Stroke::new(1.5, BOOT_GREEN);
            style.visuals.widgets.hovered.rounding = egui::Rounding::same(2.0);
            style.visuals.widgets.active.bg_fill =
                egui::Color32::from_rgba_unmultiplied(0, 100, 25, 240);
            style.visuals.widgets.active.weak_bg_fill =
                egui::Color32::from_rgba_unmultiplied(0, 100, 25, 240);
            style.visuals.widgets.active.fg_stroke =
                egui::Stroke::new(2.0, BOOT_GREEN_GLOW);
            style.visuals.widgets.active.rounding = egui::Rounding::same(2.0);
            style.visuals.window_rounding = egui::Rounding::same(4.0);
            ctx.set_style(style);
        }

        // ── Load Gillsystems header once (plain logo — no QR codes) ──────────
        if self.background_texture.is_none() {
            let bytes = include_bytes!("../../assets/Gill Systems Logo.png");
            if let Ok(img) = image::load_from_memory(bytes) {
                let rgba = img.to_rgba8();
                let (w, h) = rgba.dimensions();
                let pixels = rgba.into_raw();
                let ci = egui::ColorImage::from_rgba_unmultiplied(
                    [w as usize, h as usize], &pixels,
                );
                self.background_texture_size = Some([w as usize, h as usize]);
                self.background_texture = Some(ctx.load_texture(
                    "boot_header", ci, egui::TextureOptions::LINEAR,
                ));
            }
        }

        // ── Load footer image once ───────────────────────────────────────────
        if self.footer_texture.is_none() {
            let bytes = include_bytes!("../../assets/Gillsystems_logo_with_donation_qrcodes.png");
            if let Ok(img) = image::load_from_memory(bytes) {
                let rgba = img.to_rgba8();
                let (w, h) = rgba.dimensions();
                let pixels = rgba.into_raw();
                let ci = egui::ColorImage::from_rgba_unmultiplied(
                    [w as usize, h as usize], &pixels,
                );
                self.footer_texture_size = Some([w as usize, h as usize]);
                self.footer_texture = Some(ctx.load_texture(
                    "boot_footer", ci, egui::TextureOptions::LINEAR,
                ));
            }
        }

        // ── Paint black background ────────────────────────────────────────────
        let screen = ctx
            .input(|i| i.viewport().inner_rect)
            .unwrap_or_else(|| ctx.screen_rect());
        let painter = ctx.layer_painter(egui::LayerId::background());
        painter.rect_filled(screen, egui::Rounding::ZERO, BOOT_BG);

        // No global border — avoids windowed-mode edge artifact

        // ── Custom title bar — Win7 Aero glass, Windows-style controls ────────
        egui::TopBottomPanel::top("boot_title")
            .exact_height(32.0)
            .frame(egui::Frame::none())
            .show(ctx, |ui| {
                let bar = ui.max_rect();
                // Win7 Aero glass gradient layers
                {
                    let p = ui.painter();
                    p.rect_filled(bar, egui::Rounding::ZERO,
                        egui::Color32::from_rgba_unmultiplied(8, 18, 10, 252));
                    let refl = egui::Rect::from_min_max(
                        bar.min, egui::pos2(bar.max.x, bar.min.y + bar.height() * 0.45),
                    );
                    p.rect_filled(refl, egui::Rounding::ZERO,
                        egui::Color32::from_rgba_unmultiplied(40, 80, 48, 170));
                    let hi = egui::Rect::from_min_max(
                        bar.min, egui::pos2(bar.max.x, bar.min.y + 2.0),
                    );
                    p.rect_filled(hi, egui::Rounding::ZERO,
                        egui::Color32::from_rgba_unmultiplied(80, 200, 100, 90));
                    let bl = egui::Rect::from_min_max(
                        egui::pos2(bar.min.x, bar.max.y - 1.0), bar.max,
                    );
                    p.rect_filled(bl, egui::Rounding::ZERO, BOOT_GREEN_DARK);
                }
                ui.horizontal(|ui| {
                    ui.add_space(10.0);
                    ui.label(
                        egui::RichText::new("un-F  \u{2014}  Launcher")
                            .size(13.0).strong().color(BOOT_GREEN_GLOW),
                    );

                    let bw = 46.0f32;
                    let total_btn_w = bw * 3.0;
                    let left_fill = (bar.width() - total_btn_w - 150.0).max(0.0);
                    ui.add_space(left_fill);

                    let bh = bar.height();
                    let fnt = egui::FontId::proportional(13.0);
                    let is_maximized = ctx.input(|i| i.viewport().maximized).unwrap_or(false);

                    // _ MINIMIZE
                    let (nr, min_r) = ui.allocate_exact_size(egui::vec2(bw, bh), egui::Sense::click());
                    if min_r.hovered() {
                        ui.painter().rect_filled(nr, egui::Rounding::ZERO,
                            egui::Color32::from_rgba_unmultiplied(0, 80, 20, 160));
                    }
                    ui.painter().text(nr.center(), egui::Align2::CENTER_CENTER, "_",
                        egui::FontId::proportional(15.0),
                        if min_r.hovered() { BOOT_GREEN_GLOW } else { BOOT_GREEN });
                    if min_r.clicked() {
                        self.minimize_window_native();
                        #[cfg(not(windows))]
                        ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(true));
                    }

                    // □ MAXIMIZE
                    let (mr, max_r) = ui.allocate_exact_size(egui::vec2(bw, bh), egui::Sense::click());
                    if max_r.hovered() {
                        ui.painter().rect_filled(mr, egui::Rounding::ZERO,
                            egui::Color32::from_rgba_unmultiplied(0, 80, 20, 160));
                    }
                    ui.painter().text(mr.center(), egui::Align2::CENTER_CENTER, "□",
                        fnt.clone(),
                        if max_r.hovered() { BOOT_GREEN_GLOW } else { BOOT_GREEN });
                    if max_r.clicked() {
                        ctx.send_viewport_cmd(egui::ViewportCommand::Maximized(!is_maximized));
                    }

                    // X CLOSE
                    let (cr, close_r) = ui.allocate_exact_size(egui::vec2(bw, bh), egui::Sense::click());
                    if close_r.hovered() {
                        ui.painter().rect_filled(cr, egui::Rounding::ZERO,
                            egui::Color32::from_rgba_unmultiplied(200, 20, 20, 220));
                    }
                    ui.painter().text(cr.center(), egui::Align2::CENTER_CENTER, "X",
                        fnt,
                        if close_r.hovered() { egui::Color32::WHITE }
                        else { egui::Color32::from_rgb(255, 100, 100) });
                    if close_r.clicked() {
                        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                        std::process::exit(0);
                    }
                });
                // Drag — fixed: drag_started() not is_pointer_button_down_on()
                let drag_rect = egui::Rect::from_min_max(
                    bar.min,
                    egui::pos2(bar.max.x - 160.0, bar.max.y),
                );
                let drag = ui.interact(drag_rect, egui::Id::new("boot_drag"),
                    egui::Sense::click_and_drag());
                if drag.drag_started() {
                    ctx.send_viewport_cmd(egui::ViewportCommand::StartDrag);
                }
            });

        // ── Header image ──────────────────────────────────────────────────────
        let header_h = self
            .background_texture_size
            .map(|[w, h]| ((screen.width() * (h as f32 / w as f32)).clamp(36.0, 54.0)))
            .unwrap_or(48.0);

        egui::TopBottomPanel::top("boot_header_logo")
            .exact_height(header_h)
            .frame(egui::Frame::none().fill(egui::Color32::BLACK))
            .show(ctx, |ui| {
                let rect = ui.max_rect();
                if let Some(tex) = &self.background_texture {
                    ui.painter().image(
                        tex.id(),
                        rect,
                        egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                        egui::Color32::WHITE,
                    );
                }
            });

        // ── Footer image (smaller) ────────────────────────────────────────────
        let footer_h = self
            .footer_texture_size
            .map(|[w, h]| (((screen.width() * (h as f32 / w as f32)) * 0.5).clamp(44.0, 88.0)))
            .unwrap_or(56.0);

        egui::TopBottomPanel::bottom("boot_footer_logo")
            .exact_height(footer_h)
            .frame(
                egui::Frame::none()
                    .fill(egui::Color32::BLACK)
                    .stroke(egui::Stroke::new(1.0, BOOT_GREEN_DARK))
            )
            .show(ctx, |ui| {
                let rect = ui.max_rect();
                if let Some(tex) = &self.footer_texture {
                    ui.painter().image(
                        tex.id(),
                        rect,
                        egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                        egui::Color32::WHITE,
                    );
                }
            });

        // ── Central: three launch buttons ─────────────────────────────────────
        egui::CentralPanel::default()
            .frame(egui::Frame::none()
                .fill(egui::Color32::from_rgba_unmultiplied(0, 0, 0, 200)))
            .show(ctx, |ui| {
                ui.vertical_centered(|ui| {
                    ui.add_space(32.0);
                    ui.label(
                        egui::RichText::new("Choose Your Launch Mode")
                            .size(22.0).strong().color(BOOT_GREEN),
                    );
                    ui.add_space(6.0);
                    ui.label(
                        egui::RichText::new(format!(
                            "GillSystems  —  un-F Your Rigs  v{}",
                            option_env!("APP_VERSION").unwrap_or(env!("CARGO_PKG_VERSION"))
                        ))
                            .size(11.0).color(BOOT_GREEN_DIM),
                    );
                    ui.add_space(36.0);

                    // Full mode
                    if ui.add_sized(
                        [440.0, 66.0],
                        egui::Button::new(
                            egui::RichText::new("▶  FULL  —  GUI + Scanner + SMB")
                                .size(17.0).strong().color(BOOT_GREEN_GLOW)
                        ),
                    ).on_hover_text(
                        "Full interface: duplicate scanner, live results, optional SMB share"
                    ).clicked() {
                        self.select(ctx, BootMode::LaunchGUI);
                    }

                    ui.add_space(18.0);

                    // Silent mode
                    if ui.add_sized(
                        [440.0, 66.0],
                        egui::Button::new(
                            egui::RichText::new("⛔  SILENT  —  Scanner only, no GUI")
                                .size(17.0).color(BOOT_GREEN)
                        ),
                    ).on_hover_text(
                        "Headless background service — scans and logs without any window"
                    ).clicked() {
                        self.select(ctx, BootMode::LaunchService);
                    }

                    ui.add_space(18.0);

                    // SMB mode
                    if ui.add_sized(
                        [440.0, 66.0],
                        egui::Button::new(
                            egui::RichText::new("🌐  SMB  —  Network Share Setup")
                                .size(17.0).color(BOOT_GREEN)
                        ),
                    ).on_hover_text(
                        "Configure an SMB network share for cross-machine duplicate access"
                    ).clicked() {
                        self.select(ctx, BootMode::SetupSMB);
                    }

                    ui.add_space(28.0);
                    ui.label(
                        egui::RichText::new(
                            "\u{2713} GUI launches automatically if no selection is made"
                        ).size(10.0).color(BOOT_GREEN_DIM),
                    );
                });
            });
    }

    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        [0.0, 0.0, 0.0, 1.0]
    }
}

fn load_window_icon() -> Option<egui::IconData> {
    eframe::icon_data::from_png_bytes(include_bytes!("../assets/gillsystems_logo.png")).ok()
}

/// Run the boot launcher window synchronously and return the user's chosen BootMode.
/// If the window is closed without a selection, defaults to LaunchGUI.
/// This MUST be called from the main thread (eframe requirement).
pub fn run_boot_screen() -> Result<BootMode> {
    let chosen: Arc<Mutex<Option<BootMode>>> = Arc::new(Mutex::new(None));
    let chosen_inner = chosen.clone();

    let mut viewport = egui::ViewportBuilder::default()
        .with_inner_size([900.0, 560.0])
        .with_resizable(false)
        .with_decorations(false)
        .with_title("un-F — Launcher");

    if let Some(icon) = load_window_icon() {
        viewport = viewport.with_icon(icon);
    }

    let options = eframe::NativeOptions {
        viewport,
        ..Default::default()
    };

    eframe::run_native(
        "un-F — Launcher",
        options,
        Box::new(move |_cc| Box::new(BootLauncher::new(chosen_inner)) as Box<dyn eframe::App>),
    ).map_err(|e| anyhow::anyhow!("Boot screen error: {}", e))?;

    let mode = chosen.lock().unwrap().unwrap_or(BootMode::LaunchGUI);
    info!("Boot mode selected: {:?}", mode);
    Ok(mode)
}
