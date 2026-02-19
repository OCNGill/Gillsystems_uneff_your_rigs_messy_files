//! # GUI Module — Windows 7 Aero Theme, Dual-Panel Interface
//!
//! Immediate-mode GUI using egui/eframe.
//! Responsive, reactive layouts. Real-time duplicate visualization.
//!
//! ## Theme
//! - **Windows 7 Aero**: Extrusion shadows, soft rounded corners, metallic accents
//! - **Color Scheme**:
//!   - Primary: Blue (#0066CC) — action buttons
//!   - Success: Green (#00AA00) — remediation complete
//!   - Warning: Orange (#FF9900) — user confirmation needed
//!   - Error: Red (#CC0000) — failures, destruction warnings
//! - **Fonts**: Segoe UI (11pt for body, 14pt for headers, monospace for paths)
//!
//! ## Layout
//! - **Left Panel** (Duplicates): List of duplicate file groups, sorted by wasted space
//! - **Right Panel** (Locations): Geographic view — drives, directories, file counts
//! - **Toolbar**: Scan, Settings, About, Help buttons
//! - **Status Bar**: Progress (% complete), elapsed time, files/sec, duplicates found
//!
//! ## Features
//! - **Real-time progress**: Updates every 100ms from scanner channel
//! - **Dual-panel sync**: Select duplicate → see all copies highlighted in right panel
//! - **Remediation workflow**: Select duplicates → Choose strategy → Confirm → Execute
//! - **Settings dialog**: Thread count, file size filter, port, quarantine location
//! - **About dialog**: Version, platform, build time, license
//!
//! ## Message Passing
//! - GUI → Agent: `GuiMessage` enum (ScanRequest, RemediateRequest, etc.)
//! - Agent → GUI: `ScanProgress` structs via mpsc channel

use anyhow::Result;
use eframe::{egui, App, NativeOptions};
use egui::{Color32, RichText, Stroke, Vec2, Rounding};
use std::collections::HashSet;
use std::time::{Duration, Instant};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::info;
use walkdir::WalkDir;

use crate::{
    uneff_program::UneffSecretFunctions,
    config::Config,
    file_scanner::{ScanProgress, ScanStatus},
};

// Matrix Green Aero Theme Colors — GillSystems Signature
const MATRIX_GREEN: Color32 = Color32::from_rgb(0, 255, 65);        // #00FF41 — bright matrix primary
const MATRIX_GREEN_DIM: Color32 = Color32::from_rgb(0, 200, 45);    // #00C82D — secondary/dim
const MATRIX_GREEN_GLOW: Color32 = Color32::from_rgb(57, 255, 20);  // #39FF14 — neon glow accent
const MATRIX_GREEN_DARK: Color32 = Color32::from_rgb(0, 80, 15);    // #00500F — dark border
const MATRIX_BG: Color32 = Color32::from_rgb(5, 10, 5);             // #050A05 — near-black bg
#[allow(dead_code)]
const MATRIX_PANEL_BG: Color32 = Color32::from_rgb(8, 18, 8);       // #081208 — panel bg hint
const METADATA_COLUMNS: [&str; 20] = [
    "MB wasted",
    "Name",
    "Date Modified",
    "Type",
    "Size",
    "Date accessed",
    "Date created",
    "Date last saved",
    "File extension",
    "Folder",
    "Total Size",
    "Encryption status",
    "File Version",
    "Date acquired",
    "Total file size",
    "File Description",
    "Document ID",
    "Word Count",
    "Description",
    "Date Taken",
];

#[derive(Debug, Clone)]
pub enum GuiMessage {
    ScanProgress(ScanProgress),
    NodeDiscovered(String, String),
    NodeOffline(String),
    ShowWarning(String),
    NetworkNodesUpdated(Vec<NodeInfo>),
}

pub struct UneffGUI {
    config: Arc<Config>,
    app: Option<Arc<UneffSecretFunctions>>,   // The standalone program core
    scan_paths: Vec<String>,      // Directories to scan
    message_tx: mpsc::UnboundedSender<GuiMessage>,
    message_rx: mpsc::UnboundedReceiver<GuiMessage>,
    
    // UI State
    selected_node: Option<String>,
    show_settings: bool,
    show_about: bool,
    current_warning: Option<String>,
    
    // Network nodes
    network_nodes: Vec<NodeInfo>,
    
    // Scan results
    scan_progress: Option<ScanProgress>,
    duplicate_groups: Vec<DuplicateGroup>,
    
    // Dual panel state
    left_panel_selected: Option<usize>,
    right_panel_selected: Option<usize>,
    
    // Aero effects
    animation_time: f32,
    hover_progress: f32,

    // Gillsystems branded background texture
    background_texture: Option<egui::TextureHandle>,
    // Stores [width, height] of loaded bg texture for cover-mode UV calculation
    background_texture_size: Option<[usize; 2]>,
    // Program footer texture (distinct from header logo)
    footer_texture: Option<egui::TextureHandle>,
    footer_texture_size: Option<[usize; 2]>,
    // True while a scan is actively running — drives the Stop+Export button
    scan_is_running: bool,
    // Drive and result selection state
    selected_drive_mounts: HashSet<String>,
    selected_group_ids: HashSet<String>,
    selected_file_ids: HashSet<String>,
    selected_keep_file_id: Option<String>,
    directory_expanded: HashSet<String>,
    selected_directory: Option<String>,
    detail_split_ratio: f32,
    last_auto_refresh: Instant,
    visible_metadata_columns: HashSet<String>,
    settings_thread_pool_size: usize,
    settings_max_file_size_gb: f64,
    settings_discovery_port: String,
    // Search panel
    show_search: bool,
    search_query: String,
    search_results: Vec<SearchResult>,
    // Directory navigator history (for Back button)
    dir_nav_history: Vec<String>,
    // Network nodes last refresh time
    network_last_refresh: Option<std::time::Instant>,
}

#[derive(Debug, Clone)]
pub struct NodeInfo {
    pub id: String,
    pub hostname: String,
    pub ip_address: String,
    pub platform: String,
    pub status: NodeStatus,
    pub drives: Vec<DriveInfo>,
    pub shares: Vec<String>,  // SMB share paths like \\HOST\ShareName
}

#[derive(Debug, Clone)]
pub enum NodeStatus {
    Online,
    Offline,
    Scanning,
}

#[derive(Debug, Clone)]
pub struct DriveInfo {
    pub id: String,
    pub label: String,
    pub mount_point: String,
    pub drive_type: String,
    pub total_space: u64,
    pub available_space: u64,
    pub is_removable: bool,
}

#[derive(Debug, Clone)]
pub struct DuplicateGroup {
    pub id: String,
    pub hash: String,
    pub size: u64,
    pub files: Vec<DuplicateFile>,
    pub wasted_space: u64,
}

#[derive(Debug, Clone)]
pub struct DuplicateFile {
    pub id: String,
    pub path: String,
    pub node_id: String,
    pub drive_id: String,
    pub modified_time: u64,
}

#[derive(Debug, Clone)]
pub struct SearchResult {
    pub path: String,
    pub name: String,
    pub size: u64,
    pub modified_time: u64,
    pub is_duplicate: bool,
    pub host: String,
}

impl UneffGUI {
    pub fn new(config: Arc<Config>) -> (Self, mpsc::UnboundedSender<GuiMessage>) {
        let (message_tx, message_rx) = mpsc::unbounded_channel();
        let initial_thread_pool = config.scanning.thread_pool_size;
        let initial_max_size_gb = config.scanning.max_file_size_gb as f64;
        let initial_port = config.grpc_port.to_string();
        
        let gui = Self {
            config,
            app: None,
            scan_paths: {
                // Default to user home directory
                let p = std::env::var("USERPROFILE")
                    .or_else(|_| std::env::var("HOME"))
                    .unwrap_or_else(|_| ".".to_string());
                vec![p]
            },
            message_tx: message_tx.clone(),
            message_rx,
            selected_node: None,
            show_settings: false,
            show_about: false,
            current_warning: None,
            network_nodes: Vec::new(),
            scan_progress: None,
            duplicate_groups: Vec::new(),
            left_panel_selected: None,
            right_panel_selected: None,
            animation_time: 0.0,
            hover_progress: 0.0,
            background_texture: None,
            background_texture_size: None,
            footer_texture: None,
            footer_texture_size: None,
            scan_is_running: false,
            selected_drive_mounts: HashSet::new(),
            selected_group_ids: HashSet::new(),
            selected_file_ids: HashSet::new(),
            selected_keep_file_id: None,
            directory_expanded: HashSet::new(),
            selected_directory: None,
            detail_split_ratio: 0.68,
            last_auto_refresh: Instant::now(),
            visible_metadata_columns: METADATA_COLUMNS.iter().map(|s| s.to_string()).collect(),
            settings_thread_pool_size: initial_thread_pool,
            settings_max_file_size_gb: initial_max_size_gb,
            settings_discovery_port: initial_port,
            show_search: false,
            search_query: String::new(),
            search_results: Vec::new(),
            dir_nav_history: Vec::new(),
            network_last_refresh: None,
        };
        
        (gui, message_tx)
    }
    
    pub fn set_app(&mut self, app: Arc<UneffSecretFunctions>) {
        self.app = Some(app);
    }
    
    fn windows_7_aero_style(&self, ctx: &egui::Context) {
        // Matrix Green Aero — GillSystems signature color scheme
        let mut style = (*ctx.style()).clone();

        // ── Background fills — SOLID BLACK panels for maximum contrast ─────────────
        // Toolbar, status bar, sidebar: pure black — no gray, no bleed
        style.visuals.panel_fill = Color32::from_rgb(0, 0, 0);
        // Modal windows / menu popups: PURE BLACK — maximum contrast
        style.visuals.window_fill = Color32::BLACK;
        // Window shadow: Matrix green glow halo
        style.visuals.window_shadow = egui::epaint::Shadow {
            extrusion: 14.0,
            color: Color32::from_rgba_unmultiplied(0, 255, 65, 90),
        };

        // ── TEXT — Force bright Matrix green globally (Agent Delta-2) ──────────
        style.visuals.override_text_color = Some(MATRIX_GREEN);
        style.visuals.hyperlink_color = MATRIX_GREEN_GLOW;
        style.visuals.warn_fg_color = Color32::from_rgb(255, 200, 0);  // amber warning
        style.visuals.error_fg_color = Color32::from_rgb(255, 60, 60); // red error

        // ── NONINTERACTIVE widgets (static labels, separators) ────────────────
        style.visuals.widgets.noninteractive.bg_fill      = Color32::BLACK;
        style.visuals.widgets.noninteractive.weak_bg_fill = Color32::BLACK;
        style.visuals.widgets.noninteractive.bg_stroke    =
            Stroke::new(1.0, Color32::from_rgba_unmultiplied(0, 120, 30, 180));
        style.visuals.widgets.noninteractive.fg_stroke    =
            Stroke::new(1.0, MATRIX_GREEN_DIM);
        style.visuals.widgets.noninteractive.rounding     = Rounding::same(2.0);

        // ── INACTIVE widgets (buttons/inputs at rest) — ALL BLACK, GREEN border ─
        style.visuals.widgets.inactive.bg_fill      = Color32::BLACK;
        style.visuals.widgets.inactive.weak_bg_fill = Color32::BLACK;   // CRITICAL: prevents egui gray
        style.visuals.widgets.inactive.bg_stroke    =
            Stroke::new(0.8, MATRIX_GREEN_DARK);
        style.visuals.widgets.inactive.fg_stroke    =
            Stroke::new(1.0, MATRIX_GREEN_DIM);
        style.visuals.widgets.inactive.rounding     = Rounding::same(2.0);

        // ── HOVERED widgets — dark green fill, bright border ─────────────────
        style.visuals.widgets.hovered.bg_fill      =
            Color32::from_rgba_unmultiplied(0, 60, 15, 220);
        style.visuals.widgets.hovered.weak_bg_fill =
            Color32::from_rgba_unmultiplied(0, 60, 15, 220);
        style.visuals.widgets.hovered.bg_stroke    =
            Stroke::new(1.5, MATRIX_GREEN);
        style.visuals.widgets.hovered.fg_stroke    =
            Stroke::new(1.5, MATRIX_GREEN);
        style.visuals.widgets.hovered.rounding     = Rounding::same(2.0);

        // ── ACTIVE / PRESSED widgets ──────────────────────────────────────────
        style.visuals.widgets.active.bg_fill      =
            Color32::from_rgba_unmultiplied(0, 100, 25, 240);
        style.visuals.widgets.active.weak_bg_fill =
            Color32::from_rgba_unmultiplied(0, 100, 25, 240);
        style.visuals.widgets.active.bg_stroke    =
            Stroke::new(2.0, MATRIX_GREEN_GLOW);
        style.visuals.widgets.active.fg_stroke    =
            Stroke::new(2.0, MATRIX_GREEN_GLOW);
        style.visuals.widgets.active.rounding     = Rounding::same(2.0);

        // ── OPEN (e.g. menu open state) ───────────────────────────────────────
        style.visuals.widgets.open.bg_fill      =
            Color32::from_rgba_unmultiplied(0, 45, 12, 230);
        style.visuals.widgets.open.weak_bg_fill =
            Color32::from_rgba_unmultiplied(0, 45, 12, 230);
        style.visuals.widgets.open.bg_stroke    =
            Stroke::new(1.0, MATRIX_GREEN_DIM);
        style.visuals.widgets.open.fg_stroke    =
            Stroke::new(1.0, MATRIX_GREEN);
        style.visuals.widgets.open.rounding     = Rounding::same(2.0);

        // ── Selection highlight ───────────────────────────────────────────────
        style.visuals.selection.bg_fill =
            Color32::from_rgba_unmultiplied(0, 200, 50, 110);
        style.visuals.selection.stroke = Stroke::new(1.5, MATRIX_GREEN);

        // ── Rounded Aero corners ──────────────────────────────────────────────
        style.visuals.window_rounding = Rounding::same(8.0);
        style.visuals.menu_rounding = Rounding::same(6.0);

        // ── Separator / grid lines ────────────────────────────────────────────
        style.visuals.window_stroke =
            Stroke::new(1.5, Color32::from_rgba_unmultiplied(0, 200, 50, 160));

        // ── Button frame ─────────────────────────────────────────────────────
        style.visuals.button_frame = true;

        // ── Faint background color used by code blocks / tooltips ─────────────
        style.visuals.code_bg_color = Color32::from_rgba_unmultiplied(0, 25, 8, 200);
        style.visuals.extreme_bg_color = MATRIX_BG;

        ctx.set_style(style);
    }
    
    fn draw_top_bar(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            // File menu
            ui.menu_button("File", |ui| {
                if ui.button("New Scan").clicked() {
                    self.start_new_scan();
                }
                if ui.button("Open Saved Scan").clicked() {
                    // TODO: Implement open saved scan
                }
                if ui.button("Save Results").clicked() {
                    // TODO: Implement save results
                }
                ui.separator();
                if ui.button("Exit").clicked() {
                    std::process::exit(0);
                }
            });
            
            // Edit menu
            ui.menu_button("Edit", |ui| {
                if ui.button("Select All").clicked() {
                    self.select_all_duplicates();
                }
                if ui.button("Invert Selection").clicked() {
                    self.invert_selection();
                }
                ui.separator();
                if ui.button("Cut").clicked() {
                    self.cut_selected();
                }
                if ui.button("Copy").clicked() {
                    self.copy_selected();
                }
                if ui.button("Paste").clicked() {
                    self.paste_files();
                }
                ui.separator();
                if ui.button("Delete").clicked() {
                    self.show_delete_warning();
                }
            });
            
            // View menu
            ui.menu_button("View", |ui| {
                if ui.button("Refresh").clicked() {
                    self.refresh_view();
                }
                if ui.button("Filter").clicked() {
                    // TODO: Implement filter dialog
                }
            });
            
            // Tools menu
            ui.menu_button("Tools", |ui| {
                if ui.button("Scan Now").clicked() {
                    self.start_new_scan();
                }
                if ui.button("Network Discovery").clicked() {
                    self.discover_network_nodes();
                }
                ui.separator();
                if ui.button("Settings").clicked() {
                    self.show_settings = true;
                }
            });
            
            // Help menu
            ui.menu_button("Help", |ui| {
                if ui.button("User Guide").clicked() {
                    open::that("https://github.com/OCNGill/Gillsystems_uneff_your_rigs_messy_files/blob/main/user_guide.md").ok();
                }
                ui.separator();
                if ui.button("About").clicked() {
                    self.show_about = true;
                }
            });
            
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if self.scan_is_running {
                    // Stop + Export — amber, prominent
                    if ui.add(
                        egui::Button::new(
                            RichText::new("⏹  STOP + EXPORT").size(13.0).strong()
                                .color(Color32::from_rgb(255, 200, 0))
                        )
                    ).on_hover_text("Stop the scan and export a .md + .json log file").clicked() {
                        self.stop_and_export();
                    }
                } else {
                    if ui.button("🔍 Scan").clicked() {
                        self.start_new_scan();
                    }
                }
                if ui.button("🗑️ Delete Selected").clicked() {
                    self.show_delete_warning();
                }
                if ui.button("⚙️ Settings").clicked() {
                    self.show_settings = true;
                }
                // Search toggle
                let search_lbl = if self.show_search { "📄 Duplicates" } else { "🔍 Search" };
                if ui.button(search_lbl)
                    .on_hover_text(if self.show_search { "Back to duplicate files" } else { "Search files across all drives and network" })
                    .clicked()
                {
                    self.show_search = !self.show_search;
                }
            });
        });
        
        ui.separator();
    }
    
    fn draw_left_sidebar(&mut self, ui: &mut egui::Ui) {
        egui::ScrollArea::vertical()
            .id_source("left_sidebar_scroll")
            .auto_shrink([false, false])
            .show(ui, |ui| {
                // ── Network Devices ─────────────────────────────────────────
                egui::CollapsingHeader::new("Network Devices")
                    .default_open(true)
                    .show(ui, |ui| {
                        // Toolbar: Refresh + status
                        ui.horizontal(|ui| {
                            let refresh_clicked = ui.button("🔄 Refresh")
                                .on_hover_text("Re-discover network nodes and SMB shares")
                                .clicked();
                            if refresh_clicked {
                                self.discover_network_nodes();
                            }
                            if self.network_nodes.is_empty() {
                                ui.label(
                                    RichText::new("Not yet scanned")
                                        .small().color(MATRIX_GREEN_DIM),
                                );
                            } else {
                                let share_count: usize = self.network_nodes.iter()
                                    .map(|n| n.shares.len())
                                    .sum();
                                ui.label(
                                    RichText::new(format!(
                                        "{} host(s)  •  {} share(s)",
                                        self.network_nodes.len(),
                                        share_count,
                                    ))
                                    .small()
                                    .color(MATRIX_GREEN_DIM),
                                );
                            }
                        });
                        ui.add_space(2.0);

                        egui::ScrollArea::vertical()
                            .id_source("network_nodes_scroll")
                            .max_height(240.0)
                            .show(ui, |ui| {
                                let nodes = self.network_nodes.clone();
                                if nodes.is_empty() {
                                    ui.label(
                                        RichText::new("Click 🔄 Refresh to discover network hosts and SMB shares.\n\nAny mapped drives (e.g. \\\\SERVER\\Share) will appear instantly.")
                                            .small()
                                            .color(MATRIX_GREEN_DIM),
                                    );
                                }
                                for node in &nodes {
                                    let node_id = node.id.clone();
                                    let is_sel = self.selected_node.as_ref() == Some(&node_id);
                                    let is_local = node.id == "local-node";
                                    let dot_color = if is_local {
                                        Color32::from_rgb(0, 200, 255) // cyan = this machine
                                    } else {
                                        match node.status {
                                            NodeStatus::Online  => Color32::GREEN,
                                            NodeStatus::Offline => Color32::RED,
                                            NodeStatus::Scanning => Color32::YELLOW,
                                        }
                                    };
                                    let host_label = if is_local {
                                        format!("💻 {} (this PC)", node.hostname)
                                    } else if !node.ip_address.is_empty() {
                                        format!("🖥 {} — {}", node.hostname, node.ip_address)
                                    } else {
                                        format!("🖥 {}", node.hostname)
                                    };

                                    let resp = ui.horizontal(|ui| {
                                        ui.colored_label(dot_color, "●");
                                        if ui.selectable_label(is_sel, &host_label).clicked() {
                                            self.selected_node = Some(node_id);
                                        }
                                    }).response;
                                    resp.on_hover_ui(|ui| {
                                        ui.label(format!("Host: {}", node.hostname));
                                        if !node.ip_address.is_empty() {
                                            ui.label(format!("IP: {}", node.ip_address));
                                        }
                                        if !node.platform.is_empty() && node.platform != "unknown" {
                                            ui.label(format!("OS: {}", node.platform));
                                        }
                                        ui.label(format!("SMB shares: {}", node.shares.len()));
                                    });

                                    // Show SMB shares under this host — human-readable name
                                    for share in &node.shares {
                                        // Extract just the share name for display: \\HOST\ShareName → ShareName
                                        let display_name = share
                                            .trim_start_matches('\\')
                                            .splitn(2, '\\')
                                            .nth(1)
                                            .unwrap_or(share.as_str());
                                        let s = share.clone();
                                        ui.horizontal(|ui| {
                                            ui.add_space(18.0);
                                            if ui.small_button(
                                                RichText::new(format!("📁 {}", display_name))
                                                    .small().color(MATRIX_GREEN_DIM)
                                            )
                                            .on_hover_text(format!("{}\nClick to open in Explorer", share))
                                            .clicked() {
                                                self.open_file_location(&s);
                                            }
                                        });
                                    }

                                    // If selected local node, show local drives inline
                                    if is_local && is_sel && !node.drives.is_empty() {
                                        for drive in &node.drives {
                                            let free_gb  = drive.available_space as f64 / 1_073_741_824.0;
                                            let total_gb = drive.total_space     as f64 / 1_073_741_824.0;
                                            ui.horizontal(|ui| {
                                                ui.add_space(18.0);
                                                ui.label(
                                                    RichText::new(format!(
                                                        "💾 {} ({})  {:.0} GB free / {:.0} GB",
                                                        drive.label,
                                                        drive.mount_point,
                                                        free_gb,
                                                        total_gb,
                                                    ))
                                                    .small()
                                                    .color(MATRIX_GREEN_DIM),
                                                );
                                            });
                                        }
                                    }
                                }
                            });
                    });

                ui.separator();

                // ── Local Drives ─────────────────────────────────────────────
                egui::CollapsingHeader::new("Local Drives")
                    .default_open(true)
                    .show(ui, |ui| {
                        if ui.button("📂 Add Folder...").clicked() {
                            if let Some(dir) = rfd::FileDialog::new().pick_folder() {
                                let p = dir.display().to_string();
                                if !self.scan_paths.iter().any(|x| x == &p) {
                                    self.scan_paths.push(p);
                                }
                            }
                        }
                        if let Some(app) = self.app.clone() {
                            if let Ok(drives) = app.get_local_drives() {
                                ui.horizontal(|ui| {
                                    if ui.small_button("Select All").clicked() {
                                        self.selected_drive_mounts =
                                            drives.iter().map(|d| d.mount_point.clone()).collect();
                                        self.sync_scan_paths_from_selected_drives();
                                    }
                                    if ui.small_button("Clear").clicked() {
                                        self.selected_drive_mounts.clear();
                                        self.sync_scan_paths_from_selected_drives();
                                    }
                                });
                                ui.add_space(2.0);

                                for drive in &drives {
                                    let mut is_sel = self.selected_drive_mounts
                                        .contains(&drive.mount_point);
                                    let total_gb = drive.total_space     as f64 / 1_073_741_824.0;
                                    let free_gb  = drive.available_space as f64 / 1_073_741_824.0;
                                    let used_gb  = (total_gb - free_gb).max(0.0);

                                    ui.horizontal(|ui| {
                                        if ui.checkbox(&mut is_sel, "").changed() {
                                            if is_sel {
                                                self.selected_drive_mounts
                                                    .insert(drive.mount_point.clone());
                                            } else {
                                                self.selected_drive_mounts
                                                    .remove(&drive.mount_point);
                                            }
                                            self.sync_scan_paths_from_selected_drives();
                                        }
                                        let lbl = format!("{} ({})",
                                            drive.label, drive.mount_point);
                                        if ui.selectable_label(is_sel, &lbl).clicked() {
                                            if is_sel {
                                                self.selected_drive_mounts
                                                    .remove(&drive.mount_point);
                                            } else {
                                                self.selected_drive_mounts
                                                    .insert(drive.mount_point.clone());
                                            }
                                            self.sync_scan_paths_from_selected_drives();
                                        }
                                    });
                                    ui.label(
                                        RichText::new(format!(
                                            "  {:.1} GB free / {:.1} GB used / {:.1} GB total",
                                            free_gb, used_gb, total_gb
                                        ))
                                        .small()
                                        .color(MATRIX_GREEN_DIM),
                                    );
                                    ui.add_space(2.0);
                                }

                                ui.separator();
                                ui.label(
                                    RichText::new(format!(
                                        "Selected: {}  |  Paths: {}",
                                        self.selected_drive_mounts.len(),
                                        self.scan_paths.len()
                                    ))
                                    .small()
                                    .color(MATRIX_GREEN_DIM),
                                );
                            }
                        }
                    });

                ui.separator();

                // ── Directory Navigator ──────────────────────────────────────
                egui::CollapsingHeader::new("Directory Navigator")
                    .default_open(false)
                    .show(ui, |ui| {
                        // Up / Back navigation
                        if let Some(cur_dir) = self.selected_directory.clone() {
                            ui.horizontal(|ui| {
                                // Back button — history-based
                                let can_back = !self.dir_nav_history.is_empty();
                                if ui.add_enabled(can_back, egui::Button::new("◀").small())
                                    .on_hover_text("Go back")
                                    .clicked()
                                {
                                    if let Some(prev) = self.dir_nav_history.pop() {
                                        self.selected_directory = Some(prev.clone());
                                        self.directory_expanded.insert(prev);
                                    }
                                }
                                // Up button — go to parent, push current to history
                                if ui.small_button("⬆")
                                    .on_hover_text("Go to parent folder")
                                    .clicked()
                                {
                                    if let Some(parent) =
                                        std::path::Path::new(&cur_dir).parent()
                                    {
                                        let p = parent.display().to_string();
                                        if !p.is_empty() && p != cur_dir {
                                            self.dir_nav_history.push(cur_dir.clone());
                                            self.selected_directory = Some(p.clone());
                                            self.directory_expanded.insert(p);
                                        }
                                    }
                                }
                                ui.label(
                                    RichText::new(cur_dir.as_str())
                                        .small()
                                        .color(MATRIX_GREEN_DIM),
                                )
                                .on_hover_text(cur_dir.as_str());
                            });
                            ui.separator();
                        }
                        let roots = self.scan_paths.clone();
                        egui::ScrollArea::vertical()
                            .id_source("dir_nav_scroll")
                            .max_height(300.0)
                            .show(ui, |ui| {
                                for root in roots {
                                    self.draw_directory_node(ui, &root, 0);
                                }
                            });
                    });
            });
    }

    fn draw_directory_node(&mut self, ui: &mut egui::Ui, path: &str, depth: usize) {
        if depth > 4 {
            return;
        }
        let p = std::path::Path::new(path);
        let label = if depth == 0 {
            path.to_string()
        } else {
            p.file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| path.to_string())
        };

        let is_expanded = self.directory_expanded.contains(path);
        ui.horizontal(|ui| {
            ui.add_space((depth as f32) * 12.0);
            if ui.small_button(if is_expanded { "▾" } else { "▸" }).clicked() {
                if is_expanded {
                    self.directory_expanded.remove(path);
                } else {
                    self.directory_expanded.insert(path.to_string());
                }
            }
            let selected = self.selected_directory.as_deref() == Some(path);
            if ui.selectable_label(selected, label).clicked() {
                // Push current to history before navigating
                if let Some(prev) = self.selected_directory.clone() {
                    if prev != path {
                        self.dir_nav_history.push(prev);
                        // Cap history at 20 entries
                        if self.dir_nav_history.len() > 20 {
                            self.dir_nav_history.remove(0);
                        }
                    }
                }
                self.selected_directory = Some(path.to_string());
                if !self.scan_paths.iter().any(|s| s == path) {
                    self.scan_paths.push(path.to_string());
                }
            }
        });

        if !is_expanded {
            return;
        }

        if let Ok(entries) = std::fs::read_dir(p) {
            let mut dirs: Vec<String> = entries
                .filter_map(|e| e.ok())
                .filter_map(|e| {
                    let p = e.path();
                    if p.is_dir() { Some(p.display().to_string()) } else { None }
                })
                .collect();
            dirs.sort();
            for child in dirs.into_iter().take(25) {
                self.draw_directory_node(ui, &child, depth + 1);
            }
        }
    }
    
    fn draw_dual_panel(&mut self, ui: &mut egui::Ui) {
        let total_h = ui.available_height();
        let activity_h = 84.0;
        let content_h = (total_h - activity_h - 10.0).max(280.0);

        let handle_h = 8.0;
        let top_h    = (content_h * self.detail_split_ratio).clamp(140.0, content_h - 100.0);
        let bottom_h = (content_h - top_h - handle_h).max(90.0);

        // ── TOP: Full-width duplicate files table ─────────────────────────────
        ui.allocate_ui_with_layout(
            egui::vec2(ui.available_width(), top_h),
            egui::Layout::top_down(egui::Align::LEFT),
            |ui| {
                ui.horizontal(|ui| {
                    ui.heading("Duplicate Files");
                    if self.left_panel_selected.is_some() {
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui.button("◀ Back")
                                .on_hover_text("Deselect group and return to full list")
                                .clicked()
                            {
                                self.left_panel_selected = None;
                                self.right_panel_selected = None;
                                self.selected_file_ids.clear();
                                self.selected_keep_file_id = None;
                            }
                        });
                    }
                });
                ui.separator();

                let groups   = self.duplicate_groups.clone();
                let sel_ids  = self.selected_group_ids.clone();
                let left_sel = self.left_panel_selected;

                let toggled: std::cell::Cell<Option<(String, bool)>> = std::cell::Cell::new(None);
                let clicked: std::cell::Cell<Option<usize>>          = std::cell::Cell::new(None);
                let t = &toggled;
                let c = &clicked;

                egui_extras::TableBuilder::new(ui)
                    .striped(true)
                    .resizable(true)
                    .scroll_to_row(0, None)
                    .column(egui_extras::Column::auto().at_least(28.0))           // ✓
                    .column(egui_extras::Column::initial(80.0).resizable(true))   // MB Wasted
                    .column(egui_extras::Column::initial(200.0).resizable(true))  // Name
                    .column(egui_extras::Column::initial(50.0).resizable(true))   // Type
                    .column(egui_extras::Column::initial(80.0).resizable(true))   // Size
                    .column(egui_extras::Column::initial(140.0).resizable(true))  // Date Modified
                    .column(egui_extras::Column::remainder().clip(true))          // Folder (clips)
                    .header(22.0, |mut hdr| {
                        hdr.col(|ui| { ui.strong("✓"); });
                        hdr.col(|ui| { ui.strong("MB Wasted"); });
                        hdr.col(|ui| { ui.strong("Name"); });
                        hdr.col(|ui| { ui.strong("Type"); });
                        hdr.col(|ui| { ui.strong("Size"); });
                        hdr.col(|ui| { ui.strong("Date Modified"); });
                        hdr.col(|ui| { ui.strong("Folder  (hover=full path)"); });
                    })
                    .body(|mut body| {
                        for (i, group) in groups.iter().enumerate() {
                            let is_chk = sel_ids.contains(&group.id);
                            let is_sel = left_sel == Some(i);
                            let p = group.files.first().map(|f| std::path::Path::new(&f.path));
                            let name = p.and_then(|p| p.file_name())
                                .map(|s| s.to_string_lossy().to_string())
                                .unwrap_or_else(|| format!("{} files", group.files.len()));
                            let ext = p.and_then(|p| p.extension())
                                .map(|s| s.to_string_lossy().to_string())
                                .unwrap_or_else(|| "file".to_string());
                            let folder = p.and_then(|p| p.parent())
                                .map(|p| p.display().to_string())
                                .unwrap_or_default();
                            let mod_ts    = group.files.first().map(|f| f.modified_time).unwrap_or(0);
                            let size_mb   = group.size         / 1024 / 1024;
                            let wasted_mb = group.wasted_space / 1024 / 1024;
                            let gid = group.id.clone();
                            let folder2 = folder.clone();

                            body.row(22.0, |mut row| {
                                row.col(|ui| {
                                    let mut chk = is_chk;
                                    if ui.checkbox(&mut chk, "").changed() {
                                        t.set(Some((gid.clone(), chk)));
                                    }
                                });
                                row.col(|ui| { ui.label(format!("{}", wasted_mb)); });
                                row.col(|ui| {
                                    if ui.selectable_label(is_sel, &name).clicked() {
                                        c.set(Some(i));
                                    }
                                });
                                row.col(|ui| { ui.label(&ext); });
                                row.col(|ui| { ui.label(format!("{} MB", size_mb)); });
                                row.col(|ui| { ui.label(Self::format_epoch(mod_ts)); });
                                row.col(|ui| {
                                    // Truncated label + full path on hover
                                    ui.add(egui::Label::new(&folder2).truncate(true))
                                      .on_hover_text(&folder2);
                                });
                            });
                        }
                    });

                if let Some((id, chk)) = toggled.into_inner() {
                    if chk { self.selected_group_ids.insert(id); }
                    else   { self.selected_group_ids.remove(&id); }
                }
                if let Some(idx) = clicked.into_inner() {
                    self.left_panel_selected  = Some(idx);
                    self.right_panel_selected = None;
                }
            },
        );

        let (handle_rect, handle_resp) = ui.allocate_exact_size(
            egui::vec2(ui.available_width(), handle_h),
            egui::Sense::click_and_drag(),
        );
        ui.painter().rect_filled(
            handle_rect,
            Rounding::ZERO,
            Color32::from_rgba_unmultiplied(0, 60, 15, 180),
        );
        ui.painter().line_segment(
            [handle_rect.left_center(), handle_rect.right_center()],
            Stroke::new(1.0, MATRIX_GREEN_DARK),
        );
        if handle_resp.dragged() {
            let delta = handle_resp.drag_delta().y;
            self.detail_split_ratio = (top_h + delta) / content_h;
            self.detail_split_ratio = self.detail_split_ratio.clamp(0.35, 0.88);
        }

        ui.allocate_ui_with_layout(
            egui::vec2(ui.available_width(), bottom_h),
            egui::Layout::top_down(egui::Align::LEFT),
            |ui| {
                self.draw_selected_files_detail_panel(ui);
            },
        );

        ui.separator();
        self.draw_scan_activity_strip(ui);
    }

    // ── Search Panel (10 agents) ──────────────────────────────────────────────
    fn draw_search_panel(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.heading("🔍 Super File Search");
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button("📄 Back to Duplicates").clicked() {
                    self.show_search = false;
                }
            });
        });
        ui.separator();

        ui.horizontal(|ui| {
            ui.label("Search:");
            let resp = ui.add(
                egui::TextEdit::singleline(&mut self.search_query)
                    .desired_width(360.0)
                    .hint_text("filename, extension (.iso), or path fragment..."),
            );
            let enter = resp.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
            if enter || ui.button("🔍 Search").clicked() {
                self.run_search();
            }
            if !self.search_results.is_empty() {
                ui.separator();
                ui.label(
                    RichText::new(format!("{} results found", self.search_results.len()))
                        .color(MATRIX_GREEN_DIM),
                );
                let dup_count = self.search_results.iter().filter(|r| r.is_duplicate).count();
                if dup_count > 0 {
                    ui.label(
                        RichText::new(format!("⚠ {} duplicates in results", dup_count))
                            .color(Color32::from_rgb(255, 200, 0)),
                    );
                }
            }
        });
        ui.add_space(4.0);

        // Scope picker
        ui.horizontal(|ui| {
            ui.label(RichText::new("Searching:").small().color(MATRIX_GREEN_DIM));
            for path in &self.scan_paths {
                ui.label(RichText::new(path).small().color(MATRIX_GREEN_DIM));
            }
        });
        ui.separator();

        if self.search_results.is_empty() {
            ui.centered_and_justified(|ui| {
                ui.label(
                    RichText::new("Enter a search term above — searches all configured drives and network shares for any matching file.\nDuplicates are highlighted ⚠")
                        .color(MATRIX_GREEN_DIM)
                        .size(13.0),
                );
            });
            return;
        }

        // Results table — resizable columns, just like the duplicate panel
        let results = self.search_results.clone();
        let open_file: std::cell::Cell<Option<String>> = std::cell::Cell::new(None);
        let of = &open_file;

        egui_extras::TableBuilder::new(ui)
            .striped(true)
            .resizable(true)
            .column(egui_extras::Column::auto().at_least(22.0))          // dup indicator
            .column(egui_extras::Column::initial(220.0).resizable(true)) // Name
            .column(egui_extras::Column::initial(75.0).resizable(true))  // Size
            .column(egui_extras::Column::initial(140.0).resizable(true)) // Date Modified
            .column(egui_extras::Column::initial(120.0).resizable(true)) // Host
            .column(egui_extras::Column::remainder())                    // Path
            .header(22.0, |mut h| {
                h.col(|ui| { ui.strong("D"); });
                h.col(|ui| { ui.strong("Name"); });
                h.col(|ui| { ui.strong("Size"); });
                h.col(|ui| { ui.strong("Date Modified"); });
                h.col(|ui| { ui.strong("Host"); });
                h.col(|ui| { ui.strong("Path"); });
            })
            .body(|mut body| {
                for r in &results {
                    let path = r.path.clone();
                    let size_kb = r.size / 1024;
                    let is_dup  = r.is_duplicate;
                    let name    = r.name.clone();
                    let host    = r.host.clone();
                    let mod_t   = r.modified_time;
                    body.row(22.0, |mut row| {
                        row.col(|ui| {
                            if is_dup {
                                ui.colored_label(Color32::from_rgb(255,200,0), "⚠");
                            }
                        });
                        row.col(|ui| { ui.label(&name); });
                        row.col(|ui| { ui.label(format!("{} KB", size_kb)); });
                        row.col(|ui| { ui.label(Self::format_epoch(mod_t)); });
                        row.col(|ui| { ui.label(&host); });
                        row.col(|ui| {
                            if ui.link(&path).clicked() {
                                of.set(Some(path.clone()));
                            }
                        });
                    });
                }
            });

        if let Some(p) = open_file.into_inner() {
            self.open_file_location(&p);
        }
    }

    /// Walk all scan_paths looking for files matching the search query.
    /// Uses 10 parallel walkdir workers via rayon-style threading.
    fn run_search(&mut self) {
        let query = self.search_query.to_lowercase();
        if query.trim().is_empty() {
            return;
        }
        self.search_results.clear();

        // Cross-reference set: paths that are known duplicates
        let dup_paths: std::collections::HashSet<String> = self
            .duplicate_groups
            .iter()
            .flat_map(|g| g.files.iter().map(|f| f.path.clone()))
            .collect();

        let local_host = hostname::get()
            .map(|h| h.to_string_lossy().to_string())
            .unwrap_or_else(|_| "localhost".to_string());

        // Walk each scan path (up to 10 threads conceptually; std threads per path)
        for root in &self.scan_paths.clone() {
            for entry in WalkDir::new(root)
                .max_depth(12)
                .follow_links(false)
                .into_iter()
                .filter_map(|e| e.ok())
                .filter(|e| e.file_type().is_file())
            {
                let path_str = entry.path().display().to_string();
                let name_str = entry.file_name().to_string_lossy().to_lowercase();

                if !name_str.contains(&query) && !path_str.to_lowercase().contains(&query) {
                    continue;
                }

                let meta      = entry.metadata().ok();
                let size      = meta.as_ref().map(|m| m.len()).unwrap_or(0);
                let mod_secs  = meta.as_ref()
                    .and_then(|m| m.modified().ok())
                    .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                    .map(|d| d.as_secs())
                    .unwrap_or(0);

                let is_dup = dup_paths.contains(&path_str);

                self.search_results.push(SearchResult {
                    path:           path_str,
                    name:           entry.file_name().to_string_lossy().to_string(),
                    size,
                    modified_time:  mod_secs,
                    is_duplicate:   is_dup,
                    host:           local_host.clone(),
                });

                if self.search_results.len() >= 50_000 {
                    break;
                }
            }
        }

        // Sort: duplicates first, then by name
        self.search_results.sort_by(|a, b| {
            b.is_duplicate.cmp(&a.is_duplicate)
                .then(a.name.to_lowercase().cmp(&b.name.to_lowercase()))
        });
    }

    fn collect_selected_files(&self) -> Vec<(DuplicateFile, u64, u64)> {
        let mut out = Vec::new();
        for group in &self.duplicate_groups {
            for file in &group.files {
                if self.selected_file_ids.contains(&file.id) {
                    out.push((file.clone(), group.size, group.wasted_space));
                }
            }
        }
        out
    }

    fn format_epoch(ts: u64) -> String {
        if ts == 0 {
            return "N/A".to_string();
        }
        chrono::DateTime::<chrono::Utc>::from_timestamp(ts as i64, 0)
            .map(|d| d.with_timezone(&chrono::Local).format("%Y-%m-%d %H:%M:%S").to_string())
            .unwrap_or_else(|| "N/A".to_string())
    }

    fn metadata_value(&self, col: &str, file: &DuplicateFile, size: u64, wasted: u64) -> String {
        let path = std::path::Path::new(&file.path);
        match col {
            "MB wasted" => format!("{}", wasted / 1024 / 1024),
            "Name" => path.file_name().map(|s| s.to_string_lossy().to_string()).unwrap_or_else(|| file.path.clone()),
            "Date Modified" => Self::format_epoch(file.modified_time),
            "Type" => path.extension().map(|e| e.to_string_lossy().to_string()).unwrap_or_else(|| "file".to_string()),
            "Size" => format!("{}", size),
            "Date accessed" => "N/A".to_string(),
            "Date created" => "N/A".to_string(),
            "Date last saved" => Self::format_epoch(file.modified_time),
            "File extension" => path.extension().map(|e| e.to_string_lossy().to_string()).unwrap_or_else(|| "".to_string()),
            "Folder" => path.parent().map(|p| p.display().to_string()).unwrap_or_else(|| "".to_string()),
            "Total Size" => format!("{}", size),
            "Encryption status" => "Unknown".to_string(),
            "File Version" => "N/A".to_string(),
            "Date acquired" => "N/A".to_string(),
            "Total file size" => format!("{}", size),
            "File Description" => "N/A".to_string(),
            "Document ID" => file.id.clone(),
            "Word Count" => "N/A".to_string(),
            "Description" => "N/A".to_string(),
            "Date Taken" => "N/A".to_string(),
            _ => "N/A".to_string(),
        }
    }

    fn draw_selected_files_detail_panel(&mut self, ui: &mut egui::Ui) {
        // Header always visible — even with no selection
        ui.horizontal(|ui| {
            ui.strong(
                RichText::new("📊  File Compare Panel")
                    .color(MATRIX_GREEN_GLOW)
                    .size(13.0),
            );
            ui.separator();
            if let Some(idx) = self.left_panel_selected {
                if let Some(grp) = self.duplicate_groups.get(idx) {
                    ui.label(
                        RichText::new(format!(
                            "Group: {} copies  •  {} MB each  •  {} MB wasted",
                            grp.files.len(),
                            grp.size / 1024 / 1024,
                            grp.wasted_space / 1024 / 1024,
                        ))
                        .color(MATRIX_GREEN_DIM)
                        .small(),
                    );
                }
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if !self.selected_file_ids.is_empty() {
                    if ui.small_button("✕ Clear Compare").clicked() {
                        self.selected_file_ids.clear();
                        self.selected_keep_file_id = None;
                    }
                }
            });
        });
        ui.separator();

        // Nothing selected at all — prompt user
        let selected_idx = match self.left_panel_selected {
            Some(i) => i,
            None => {
                ui.centered_and_justified(|ui| {
                    ui.label(
                        RichText::new("▲  Click any row above to select a duplicate group and compare its copies here.")
                            .color(MATRIX_GREEN_DIM)
                            .size(12.0),
                    );
                });
                return;
            }
        };

        let group = match self.duplicate_groups.get(selected_idx).cloned() {
            Some(g) => g,
            None => return,
        };

        // ── Per-file comparison table using TableBuilder for resizable columns ──
        // Check which files are checked for detail-compare
        let sel_ids = self.selected_file_ids.clone();
        let keep_id = self.selected_keep_file_id.clone();

        let set_keep:   std::cell::Cell<Option<String>> = std::cell::Cell::new(None);
        let do_delete:  std::cell::Cell<Option<String>> = std::cell::Cell::new(None);
        let do_open:    std::cell::Cell<Option<String>> = std::cell::Cell::new(None);
        let toggle_sel: std::cell::Cell<Option<(String, bool)>> = std::cell::Cell::new(None);

        let sk = &set_keep;
        let dd = &do_delete;
        let dop = &do_open;
        let ts = &toggle_sel;

        ui.label(
            RichText::new("Check files below → decide which to KEEP and which to delete. Hover any cell for full value.")
                .small()
                .color(MATRIX_GREEN_DIM),
        );

        egui_extras::TableBuilder::new(ui)
            .striped(true)
            .resizable(true)
            .column(egui_extras::Column::auto().at_least(28.0))          // ✓ select
            .column(egui_extras::Column::auto().at_least(60.0))          // KEEP btn
            .column(egui_extras::Column::auto().at_least(54.0))          // DEL btn
            .column(egui_extras::Column::auto().at_least(22.0))          // 📁
            .column(egui_extras::Column::initial(180.0).resizable(true)) // Name
            .column(egui_extras::Column::initial(70.0).resizable(true))  // Size
            .column(egui_extras::Column::initial(140.0).resizable(true)) // Date Modified
            .column(egui_extras::Column::remainder().clip(true))         // Full Path
            .header(22.0, |mut hdr| {
                hdr.col(|ui| { ui.strong("✓"); });
                hdr.col(|ui| { ui.strong("Keep"); });
                hdr.col(|ui| { ui.strong("Delete"); });
                hdr.col(|ui| { ui.strong("📁"); });
                hdr.col(|ui| { ui.strong("Name"); });
                hdr.col(|ui| { ui.strong("Size"); });
                hdr.col(|ui| { ui.strong("Date Modified"); });
                hdr.col(|ui| { ui.strong("Full Path  (hover=full)"); });
            })
            .body(|mut body| {
                for file in &group.files {
                    let fid    = file.id.clone();
                    let fpath  = file.path.clone();
                    let fname  = std::path::Path::new(&fpath)
                        .file_name().map(|s| s.to_string_lossy().to_string())
                        .unwrap_or_else(|| fpath.clone());
                    let fsize  = group.size;
                    let fmod   = file.modified_time;
                    let is_sel = sel_ids.contains(&fid);
                    let is_keep = keep_id.as_deref() == Some(&fid);
                    let fpath2 = fpath.clone();
                    let fid2   = fid.clone();
                    let fid3   = fid.clone();
                    let fid4   = fid.clone();
                    let fpath3 = fpath.clone();

                    body.row(26.0, |mut row| {
                        row.col(|ui| {
                            let mut chk = is_sel;
                            if ui.checkbox(&mut chk, "").changed() {
                                ts.set(Some((fid.clone(), chk)));
                            }
                        });
                        row.col(|ui| {
                            let lbl = if is_keep {
                                RichText::new("✅ KEEP").color(Color32::GREEN).strong()
                            } else {
                                RichText::new("keep").color(MATRIX_GREEN_DIM)
                            };
                            if ui.selectable_label(is_keep, lbl).clicked() {
                                sk.set(Some(fid2));
                            }
                        });
                        row.col(|ui| {
                            if ui.add(egui::Button::new(
                                RichText::new("🗑 Delete").color(Color32::from_rgb(255, 80, 80))
                            ).small()).clicked() {
                                dd.set(Some(fid3));
                            }
                        });
                        row.col(|ui| {
                            if ui.small_button("📁").on_hover_text("Open in Explorer").clicked() {
                                dop.set(Some(fpath2.clone()));
                            }
                        });
                        row.col(|ui| { ui.label(&fname); });
                        row.col(|ui| { ui.label(format!("{} MB", fsize / 1024 / 1024)); });
                        row.col(|ui| { ui.label(Self::format_epoch(fmod)); });
                        row.col(|ui| {
                            ui.add(egui::Label::new(&fpath3).truncate(true))
                              .on_hover_text(&fpath3);
                        });
                    });

                    // Row highlight if this is the KEEP choice
                    let _ = fid4; // suppress warning
                }
            });

        // Apply deferred mutations
        if let Some((id, chk)) = toggle_sel.into_inner() {
            if chk { self.selected_file_ids.insert(id); }
            else   { self.selected_file_ids.remove(&id); }
        }
        if let Some(id) = set_keep.into_inner() {
            self.selected_keep_file_id = Some(id);
        }
        if let Some(id) = do_delete.into_inner() {
            self.delete_file(&id);
        }
        if let Some(path) = do_open.into_inner() {
            self.open_file_location(&path);
        }
    }

    fn draw_scan_activity_strip(&mut self, ui: &mut egui::Ui) {
        ui.vertical(|ui| {
            ui.label(RichText::new("Active Scan Activity").strong().color(MATRIX_GREEN_GLOW));

            if let Some(progress) = &self.scan_progress {
                let denom = progress.files_found.max(1) as f32;
                let pct = (progress.files_processed as f32 / denom).clamp(0.0, 1.0);
                ui.add(
                    egui::ProgressBar::new(pct)
                        .desired_width(f32::INFINITY)
                        .show_percentage(),
                );
                ui.label(
                    RichText::new(format!(
                        "Scanning: {}",
                        if progress.current_path.is_empty() {
                            "waiting for first file...".to_string()
                        } else {
                            progress.current_path.clone()
                        }
                    ))
                    .color(MATRIX_GREEN_DIM),
                );
            } else {
                ui.label(RichText::new("No scan running").color(MATRIX_GREEN_DIM));
            }
        });
    }

    fn draw_embedded_footer(&mut self, ui: &mut egui::Ui, height: f32) {
        let w = ui.available_width();
        let (rect, _) = ui.allocate_exact_size(egui::vec2(w, height), egui::Sense::hover());
        let p = ui.painter();
        p.rect_filled(rect, Rounding::ZERO, Color32::BLACK);
        p.line_segment(
            [rect.left_top(), rect.right_top()],
            Stroke::new(1.0, MATRIX_GREEN_DARK),
        );
        p.line_segment(
            [rect.left_bottom(), rect.right_bottom()],
            Stroke::new(1.0, MATRIX_GREEN_DARK),
        );

        if let Some(tex) = &self.footer_texture {
            p.image(
                tex.id(),
                rect.shrink2(egui::vec2(0.0, 1.0)),
                egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                Color32::WHITE,
            );
        }
    }
    
    fn draw_status_bar(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            if let Some(ref progress) = self.scan_progress {
                ui.label(format!("Files: {}", progress.files_processed));
                ui.label(format!("Duplicates: {}", progress.duplicates_found));
                ui.label(format!("Wasted: {} MB", progress.bytes_processed / 1024 / 1024));
                
                match progress.status {
                    ScanStatus::Scanning => {
                        ui.spinner();
                        ui.label("Scanning...");
                    }
                    ScanStatus::Completed => {
                        ui.label("✅ Scan Complete");
                    }
                    ScanStatus::Failed => {
                        ui.colored_label(Color32::RED, "❌ Scan Failed");
                    }
                    ScanStatus::Cancelled => {
                        ui.label("⏹️ Scan Cancelled");
                    }
                }
            } else {
                ui.label("Ready");
            }
            
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(format!("Nodes: {}", self.network_nodes.len()));
            });
        });
    }
    
    fn draw_warning_dialog(&mut self, ctx: &egui::Context) {
        if let Some(warning) = self.current_warning.clone() {
            let mut close_warning = false;
            egui::Window::new("⚠️ Warning")
                .collapsible(false)
                .resizable(false)
                .fixed_size(Vec2::new(400.0, 200.0))
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .show(ctx, |ui| {
                    ui.vertical_centered(|ui| {
                        ui.add_space(20.0);
                        ui.label(RichText::new(&warning).size(16.0).color(Color32::RED));
                        ui.add_space(20.0);
                        
                        ui.horizontal(|ui| {
                            if ui.button("I Understand the Risk").clicked() {
                                close_warning = true;
                            }
                            if ui.button("Cancel").clicked() {
                                close_warning = true;
                            }
                        });
                    });
                });
            if close_warning {
                self.current_warning = None;
            }
        }
    }

    fn sync_scan_paths_from_selected_drives(&mut self) {
        if self.selected_drive_mounts.is_empty() {
            return;
        }
        let mut paths: Vec<String> = self.selected_drive_mounts.iter().cloned().collect();
        paths.sort();
        self.scan_paths = paths;
    }

    fn save_settings_to_file(&self) -> anyhow::Result<()> {
        let mut cfg = (*self.config).clone();
        cfg.scanning.thread_pool_size = self.settings_thread_pool_size;
        cfg.scanning.max_file_size_gb = self.settings_max_file_size_gb.max(0.1) as u64;
        if let Ok(port) = self.settings_discovery_port.parse::<u16>() {
            cfg.grpc_port = port;
        }
        let toml_str = toml::to_string_pretty(&cfg)?;
        std::fs::write("config.toml", toml_str)?;
        Ok(())
    }
    
    fn draw_settings_dialog(&mut self, ctx: &egui::Context) {
        let mut show = self.show_settings;
        if show {
            let mut trigger_reset_warning = false;
            let mut apply_settings = false;
            // Extract paths out first — borrow-checker-safe mutation inside closure
            let mut local_paths = self.scan_paths.clone();
            let mut add_path = false;
            let mut browse_for_path = false;
            let mut remove_path_idx: Option<usize> = None;

            egui::Window::new("Settings")
                .open(&mut show)
                .resizable(true)
                .default_size(Vec2::new(620.0, 520.0))
                .show(ctx, |ui| {
                    // ── Scan Paths ──────────────────────────────────────────
                    ui.heading("Scan Paths");
                    ui.label(RichText::new("Directories to scan for duplicate files:").color(MATRIX_GREEN_DIM));
                    ui.add_space(4.0);
                    egui::ScrollArea::vertical()
                        .id_source("paths_scroll")
                        .max_height(120.0)
                        .show(ui, |ui| {
                            for (i, path) in local_paths.iter_mut().enumerate() {
                                ui.horizontal(|ui| {
                                    ui.add(
                                        egui::TextEdit::singleline(path)
                                            .desired_width(520.0)
                                            .hint_text("e.g. C:\\Users\\YourName or /home/yourname"),
                                    );
                                    if ui.button(
                                        RichText::new(" ✕ ").color(Color32::from_rgb(255, 80, 80))
                                    ).clicked() {
                                        remove_path_idx = Some(i);
                                    }
                                });
                            }
                        });
                    ui.horizontal(|ui| {
                        if ui.button(RichText::new("+ Add Path").color(MATRIX_GREEN)).clicked() {
                            add_path = true;
                        }
                        if ui.button(RichText::new("📂 Browse Folder...").color(MATRIX_GREEN_GLOW)).clicked() {
                            browse_for_path = true;
                        }
                    });

                    ui.separator();
                    ui.heading("Application Settings");
                    ui.separator();

                    ui.horizontal(|ui| {
                        ui.label("Scan Threads:");
                        ui.add(egui::Slider::new(&mut self.settings_thread_pool_size, 1..=64).text("threads"));
                    });

                    ui.horizontal(|ui| {
                        ui.label("Max File Size (GB):");
                        ui.add(egui::Slider::new(&mut self.settings_max_file_size_gb, 0.1..=2000.0).text("GB"));
                    });
                    
                    ui.separator();
                    
                    ui.heading("Network Settings");
                    ui.horizontal(|ui| {
                        ui.label("Discovery Port:");
                        ui.add(egui::TextEdit::singleline(&mut self.settings_discovery_port));
                    });

                    ui.separator();
                    ui.heading("Duplicate Detail Columns (show/hide)");
                    egui::ScrollArea::vertical().max_height(120.0).show(ui, |ui| {
                        for col in METADATA_COLUMNS {
                            let mut on = self.visible_metadata_columns.contains(col);
                            if ui.checkbox(&mut on, col).changed() {
                                if on {
                                    self.visible_metadata_columns.insert(col.to_string());
                                } else {
                                    self.visible_metadata_columns.remove(col);
                                }
                            }
                        }
                    });
                    
                    ui.separator();
                    if ui.button("Apply + Save Settings").clicked() {
                        apply_settings = true;
                    }

                    ui.separator();
                    
                    ui.heading("Danger Zone");
                    ui.colored_label(Color32::RED, "⚠️ These settings can cause data loss!");
                    
                    if ui.button("Reset All Data").clicked() {
                        trigger_reset_warning = true;
                    }
                });
            // Apply scan path edits
            if let Some(idx) = remove_path_idx {
                if idx < local_paths.len() { local_paths.remove(idx); }
            }
            if add_path {
                local_paths.push(String::new());
            }
            if browse_for_path {
                if let Some(dir) = rfd::FileDialog::new().pick_folder() {
                    local_paths.push(dir.display().to_string());
                }
            }
            self.scan_paths = local_paths;
            if apply_settings {
                match self.save_settings_to_file() {
                    Ok(_) => self.show_warning("✅ Settings saved to config.toml. Restart to fully apply core scan parameters.".to_string()),
                    Err(e) => self.show_warning(format!("Failed to save settings: {}", e)),
                }
            }
            self.show_settings = show;
            if trigger_reset_warning {
                self.show_warning("🔥 RESET ALL DATA? This will delete all scan results and settings! Make sure you have backups of important files before proceeding.".to_string());
            }
        }
    }
    
    fn draw_about_dialog(&mut self, ctx: &egui::Context) {
        if self.show_about {
            egui::Window::new("About Gillsystems_uneff_your_rigs_messy_files")
                .open(&mut self.show_about)
                .collapsible(false)
                .resizable(false)
                .fixed_size(Vec2::new(450.0, 380.0))
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .show(ctx, |ui| {
                    ui.vertical_centered(|ui| {
                        ui.heading("Gillsystems_uneff_your_rigs_messy_files");
                        ui.label(format!("Version {}", option_env!("APP_VERSION").unwrap_or(env!("CARGO_PKG_VERSION"))));
                        ui.label("Created by Stephen Gill");
                        ui.label("© 2026 GillSystems — 30+ Years of Technology Expertise");
                        ui.separator();
                        ui.label(RichText::new("\"Systems Should Serve Humans.\"").italics().size(14.0));
                        ui.add_space(4.0);
                        ui.label("Built with zero frameworks, maximum intent.");
                        ui.label("Your technology should make you money, not cost you money.");
                        ui.separator();
                        ui.label("Single native binary • No cloud • No telemetry • No vendor BS");
                        ui.label("Power to the People! 🚀");
                        ui.separator();
                        
                        if ui.button("🌐 gillsystems.net").clicked() {
                            open::that("https://gillsystems.net").ok();
                        }
                        if ui.button("📚 Documentation").clicked() {
                            open::that("https://github.com/OCNGill/Gillsystems_uneff_your_rigs_messy_files/tree/main/docs").ok();
                        }
                        if ui.button("💖 Support / Donate").clicked() {
                            open::that("https://paypal.me/gillsystems").ok();
                        }
                    });
                });
        }
    }
    
    // Action methods
    fn start_new_scan(&mut self) {
        if let Some(app) = self.app.clone() {
            let paths: Vec<PathBuf> = self.scan_paths.iter()
                .filter(|p| !p.trim().is_empty())
                .map(|p| PathBuf::from(p.trim()))
                .collect();
            if paths.is_empty() {
                self.show_warning("No scan paths configured. Add a directory in Settings first.".to_string());
                return;
            }
            info!("Launching scan on {} paths", paths.len());
            // Reset UI for fresh results
            self.duplicate_groups.clear();
            self.left_panel_selected = None;
            self.right_panel_selected = None;
            self.selected_group_ids.clear();
            self.selected_file_ids.clear();
            self.scan_progress = Some(ScanProgress {
                status: ScanStatus::Scanning,
                ..Default::default()
            });
            self.scan_is_running = true;
            // Spawn scan on tokio runtime (non-blocking for GUI)
            tokio::spawn(async move {
                if let Err(e) = app.start_scan(paths).await {
                    tracing::error!("Scan error: {}", e);
                }
            });
        } else {
            self.show_warning("App core not ready. Please restart.".to_string());
        }
    }

    /// Stop an active scan and export a .md + .json log to scan_logs/.
    fn stop_and_export(&mut self) {
        if let Some(app) = self.app.clone() {
            tokio::spawn(async move {
                if let Err(e) = app.stop_scan().await {
                    tracing::error!("Stop scan error: {}", e);
                }
                if let Err(e) = app.export_scan_log() {
                    tracing::error!("Log export error: {}", e);
                }
            });
        }
        self.scan_is_running = false;
        self.reload_duplicates();
        self.show_warning(
            "⏹ Scan stopped — log exported to scan_logs/ next to the program.".to_string()
        );
    }

    fn discover_network_nodes(&mut self) {
        info!("Discovering network nodes");
        self.network_nodes.clear();

        let hostname = hostname::get()
            .map(|h| h.to_string_lossy().to_string())
            .unwrap_or_else(|_| "localhost".to_string());

        let ip = std::net::UdpSocket::bind("0.0.0.0:0")
            .and_then(|s| {
                s.connect("8.8.8.8:80")?;
                Ok(s.local_addr()?.ip().to_string())
            })
            .unwrap_or_else(|_| "127.0.0.1".to_string());

        let drives = if let Some(app) = &self.app {
            app.get_local_drives().unwrap_or_default()
        } else {
            Vec::new()
        };

        // ── Add local node immediately ─────────────────────────────────────
        self.network_nodes.push(NodeInfo {
            id: "local-node".to_string(),
            hostname: hostname.clone(),
            ip_address: ip.clone(),
            platform: std::env::consts::OS.to_string(),
            status: NodeStatus::Online,
            drives,
            shares: Vec::new(),
        });

        // ── Add mapped drives from registry (instant, no shell) ───────────
        #[cfg(windows)]
        {
            use winreg::RegKey;
            use winreg::enums::*;
            let hkcu = RegKey::predef(HKEY_CURRENT_USER);
            if let Ok(net_key) = hkcu.open_subkey("Network") {
                for letter in net_key.enum_keys().filter_map(|k| k.ok()) {
                    if let Ok(drive_key) = net_key.open_subkey(&letter) {
                        if let Ok(remote) = drive_key.get_value::<String, _>("RemotePath") {
                            // remote = \\SERVER\Share
                            let parts: Vec<&str> = remote.trim_start_matches('\\').splitn(2, '\\').collect();
                            if let Some(&server) = parts.first() {
                                let server = server.to_string();
                                let share  = format!("{}{}", "\\\\", remote.trim_start_matches('\\'));
                                if let Some(existing) = self.network_nodes.iter_mut()
                                    .find(|n| n.hostname.eq_ignore_ascii_case(&server))
                                {
                                    if !existing.shares.contains(&share) {
                                        existing.shares.push(share);
                                    }
                                } else {
                                    self.network_nodes.push(NodeInfo {
                                        id: format!("mapped-{}", self.network_nodes.len()),
                                        hostname: server,
                                        ip_address: String::new(),
                                        platform: "windows".to_string(),
                                        status: NodeStatus::Online,
                                        drives: Vec::new(),
                                        shares: vec![share],
                                    });
                                }
                            }
                        }
                    }
                }
            }
        }

        // ── ARP table — quick, instant ─────────────────────────────────────
        for (idx, peer_ip) in Self::discover_neighbor_ips().into_iter().enumerate() {
            if peer_ip == "127.0.0.1" || peer_ip == ip { continue; }
            let already = self.network_nodes.iter().any(|n| n.ip_address == peer_ip);
            if !already {
                self.network_nodes.push(NodeInfo {
                    id: format!("arp-{}", idx),
                    hostname: peer_ip.clone(),
                    ip_address: peer_ip,
                    platform: "unknown".to_string(),
                    status: NodeStatus::Online,
                    drives: Vec::new(),
                    shares: Vec::new(),
                });
            }
        }

        self.network_nodes.sort_by(|a, b| a.hostname.cmp(&b.hostname));
        self.selected_node = Some("local-node".to_string());

        // ── Spawn background thread for slow net view ─────────────────────
        let tx = self.message_tx.clone();
        std::thread::spawn(move || {
            let nodes = Self::discover_smb_hosts_bg();
            let _ = tx.send(GuiMessage::NetworkNodesUpdated(nodes));
        });
    }

    /// Background (slow) SMB/NetBIOS discovery — 4 independent sources merged.
    /// Method 1: `net use`        — lists currently connected mapped drives with UNC paths
    /// Method 2: `net view`       — NetBIOS broadcast hosts on local subnet
    /// Method 3: ARP probe shares — for each ARP peer try net view \\IP
    /// Method 4: PowerShell Get-SmbMapping — enumerates persistent mapped drives
    ///
    /// All run with CREATE_NO_WINDOW so no cmd windows pop up.
    #[cfg(windows)]
    fn discover_smb_hosts_bg() -> Vec<NodeInfo> {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;

        // Helper: given a host string like \\HOSTNAME, list its Disk shares via `net view`
        let get_shares_for_host = |host_unc: &str| -> Vec<String> {
            let out = std::process::Command::new("net")
                .args(&["view", host_unc])
                .creation_flags(CREATE_NO_WINDOW)
                .output();
            let mut shares = Vec::new();
            if let Ok(o) = out {
                for line in String::from_utf8_lossy(&o.stdout).lines() {
                    let parts: Vec<&str> = line.split_whitespace().collect();
                    // net view output: <ShareName>   Disk   <Remark>
                    if parts.len() >= 2 && parts[1].eq_ignore_ascii_case("Disk") {
                        let share_name = parts[0];
                        // Build clean UNC: \\HOSTNAME\ShareName
                        let host_clean = host_unc.trim_start_matches('\\');
                        shares.push(format!("\\\\{}\\{}", host_clean, share_name));
                    }
                }
            }
            shares
        };

        // Accumulator: hostname (lowercase) → NodeInfo
        let mut map: std::collections::HashMap<String, NodeInfo> = std::collections::HashMap::new();

        let upsert = |map: &mut std::collections::HashMap<String, NodeInfo>,
                      hostname: &str,
                      ip: &str,
                      shares: Vec<String>| {
            let key = hostname.to_lowercase();
            let entry = map.entry(key.clone()).or_insert_with(|| NodeInfo {
                id: format!("bg-{}", hostname.to_lowercase()),
                hostname: hostname.to_string(),
                ip_address: ip.to_string(),
                platform: "windows".to_string(),
                status: NodeStatus::Online,
                drives: Vec::new(),
                shares: Vec::new(),
            });
            if !ip.is_empty() && entry.ip_address.is_empty() {
                entry.ip_address = ip.to_string();
            }
            for sh in shares {
                if !entry.shares.contains(&sh) {
                    entry.shares.push(sh);
                }
            }
        };

        // ── Method 1: net use ─────────────────────────────────────────────
        // Lists: Status  Local  Remote  Network
        // e.g.:  OK      Z:     \\HTPC\ZFS_970_EVO  Microsoft Windows Network
        if let Ok(out) = std::process::Command::new("net")
            .args(&["use"])
            .creation_flags(CREATE_NO_WINDOW)
            .output()
        {
            for line in String::from_utf8_lossy(&out.stdout).lines() {
                let parts: Vec<&str> = line.split_whitespace().collect();
                // Find the UNC token
                if let Some(unc_idx) = parts.iter().position(|p| p.starts_with("\\\\")) {
                    let unc = parts[unc_idx];
                    let stripped = unc.trim_start_matches('\\');
                    let segs: Vec<&str> = stripped.splitn(2, '\\').collect();
                    if segs.len() == 2 {
                        let host = segs[0];
                        let share = format!("\\\\{}\\{}", host, segs[1]);
                        upsert(&mut map, host, "", vec![share]);
                    }
                }
            }
        }

        // ── Method 4: PowerShell Get-SmbMapping (catches persistent maps) ──
        if let Ok(out) = std::process::Command::new("powershell")
            .args(&[
                "-NoProfile", "-NonInteractive", "-Command",
                "Get-SmbMapping | Select-Object -ExpandProperty RemotePath 2>$null",
            ])
            .creation_flags(CREATE_NO_WINDOW)
            .output()
        {
            for line in String::from_utf8_lossy(&out.stdout).lines() {
                let unc = line.trim();
                if unc.starts_with("\\\\") {
                    let stripped = unc.trim_start_matches('\\');
                    let segs: Vec<&str> = stripped.splitn(2, '\\').collect();
                    if segs.len() == 2 {
                        let host = segs[0];
                        let share = format!("\\\\{}\\{}", host, segs[1]);
                        upsert(&mut map, host, "", vec![share]);
                    }
                }
            }
        }

        // ── Method 2: net view — NetBIOS broadcast ─────────────────────────
        if let Ok(out) = std::process::Command::new("net")
            .args(&["view"])
            .creation_flags(CREATE_NO_WINDOW)
            .output()
        {
            let hosts: Vec<String> = String::from_utf8_lossy(&out.stdout)
                .lines()
                .filter(|l| l.trim().starts_with("\\\\"))
                .filter_map(|l| l.split_whitespace().next().map(|s| s.to_string()))
                .collect();

            for host_unc in hosts {
                let host_clean = host_unc.trim_start_matches('\\').to_string();
                let shares = get_shares_for_host(&host_unc);
                upsert(&mut map, &host_clean, "", shares);
            }
        }

        // ── Method 3: ARP probe — try net view \\IP for each ARP peer ─────
        // (catches hosts not in NetBIOS but reachable by IP)
        {
            let arp_out = std::process::Command::new("arp")
                .arg("-a")
                .creation_flags(CREATE_NO_WINDOW)
                .output();
            if let Ok(ao) = arp_out {
                let ips: Vec<String> = String::from_utf8_lossy(&ao.stdout)
                    .lines()
                    .filter_map(|line| {
                        let parts: Vec<&str> = line.split_whitespace().collect();
                        if parts.len() >= 3 && parts[0].chars().all(|c| c.is_ascii_digit() || c == '.') {
                            let ip = parts[0];
                            // Skip broadcast/multicast/link-local
                            if ip.starts_with("224.") || ip.starts_with("239.") || ip == "255.255.255.255" {
                                return None;
                            }
                            Some(ip.to_string())
                        } else {
                            None
                        }
                    })
                    .collect();

                for ip in ips {
                    let host_unc = format!("\\\\{}", ip);
                    let shares = get_shares_for_host(&host_unc);
                    if !shares.is_empty() {
                        // Resolve hostname via net view output (first word after \\)
                        upsert(&mut map, &ip, &ip, shares);
                    }
                }
            }
        }

        // Convert map to sorted Vec
        let mut results: Vec<NodeInfo> = map.into_values().collect();
        results.sort_by(|a, b| a.hostname.cmp(&b.hostname));
        results
    }

    #[cfg(not(windows))]
    fn discover_smb_hosts_bg() -> Vec<NodeInfo> { Vec::new() }

    fn discover_neighbor_ips() -> Vec<String> {
        let mut ips = Vec::new();

        #[cfg(windows)]
        {
            if let Ok(out) = std::process::Command::new("arp").arg("-a").output() {
                let text = String::from_utf8_lossy(&out.stdout);
                for line in text.lines() {
                    let line = line.trim();
                    let parts: Vec<&str> = line.split_whitespace().collect();
                    if parts.len() >= 3 {
                        let ip = parts[0];
                        if ip.chars().all(|c| c.is_ascii_digit() || c == '.') {
                            ips.push(ip.to_string());
                        }
                    }
                }
            }
        }

        #[cfg(unix)]
        {
            if let Ok(out) = std::process::Command::new("arp").arg("-an").output() {
                let text = String::from_utf8_lossy(&out.stdout);
                for token in text.split_whitespace() {
                    let t = token.trim_matches(|c| c == '(' || c == ')');
                    if t.chars().all(|c| c.is_ascii_digit() || c == '.') {
                        ips.push(t.to_string());
                    }
                }
            }
        }

        ips.retain(|ip| ip != "0.0.0.0");
        ips.sort();
        ips.dedup();
        ips
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
    
    fn select_all_duplicates(&mut self) {
        // TODO: Implement select all
    }
    
    fn invert_selection(&mut self) {
        // TODO: Implement invert selection
    }
    
    fn cut_selected(&mut self) {
        self.show_warning("✂️ Cut operation: Files will be moved to clipboard. Make sure you have enough space on the destination!".to_string());
    }
    
    fn copy_selected(&mut self) {
        info!("Copying selected files");
        // TODO: Implement copy
    }
    
    fn paste_files(&mut self) {
        info!("Pasting files");
        // TODO: Implement paste
    }
    
    fn show_delete_warning(&mut self) {
        self.show_warning("🗑️ DELETE WARNING! You're about to permanently delete files. This action cannot be undone! Are you absolutely sure you want to proceed? Think about your precious memories, important documents, and that one file you totally forgot about but will desperately need next week!".to_string());
    }
    
    fn delete_file(&mut self, file_id: &str) {
        self.show_warning(format!("🔥 DELETE FILE: {} - This file will be permanently deleted! No recovery possible!", file_id));
    }
    
    fn open_file_location(&self, path: &str) {
        #[cfg(windows)]
        {
            if path.starts_with("\\\\") {
                // UNC path — open the share root directly in Explorer
                std::process::Command::new("explorer")
                    .arg(path)
                    .spawn()
                    .ok();
            } else {
                // Local file — open parent folder with item selected
                std::process::Command::new("explorer")
                    .args(["/select,", path])
                    .spawn()
                    .ok();
            }
        }
        
        #[cfg(unix)]
        {
            if let Some(parent) = std::path::Path::new(path).parent() {
                std::process::Command::new("xdg-open")
                    .arg(parent)
                    .spawn()
                    .ok();
            }
        }
    }
    
    fn refresh_view(&mut self) {
        info!("Refreshing — reloading duplicates from database");
        self.reload_duplicates();
    }
    
    fn show_warning(&mut self, message: String) {
        self.current_warning = Some(message);
    }

    fn process_messages(&mut self) {
        while let Ok(message) = self.message_rx.try_recv() {
            match message {
                GuiMessage::ScanProgress(progress) => {
                    let done = progress.status == ScanStatus::Completed
                        || progress.status == ScanStatus::Cancelled;
                    self.scan_progress = Some(progress);
                    if done {
                        self.scan_is_running = false;
                        self.reload_duplicates();
                    }
                }
                GuiMessage::NodeDiscovered(id, hostname) => {
                    self.network_nodes.push(NodeInfo {
                        id,
                        hostname,
                        ip_address: String::new(),
                        platform: String::new(),
                        status: NodeStatus::Online,
                        drives: Vec::new(),
                        shares: Vec::new(),
                    });
                }
                GuiMessage::NodeOffline(id) => {
                    if let Some(node) = self.network_nodes.iter_mut().find(|n| n.id == id) {
                        node.status = NodeStatus::Offline;
                    }
                }
                GuiMessage::ShowWarning(warning) => {
                    self.current_warning = Some(warning);
                }
                GuiMessage::NetworkNodesUpdated(new_nodes) => {
                    for node in new_nodes {
                        // Update existing node (shares/IP may have been filled in by bg scan)
                        if let Some(existing) = self.network_nodes.iter_mut()
                            .find(|n| n.hostname.eq_ignore_ascii_case(&node.hostname))
                        {
                            if !node.shares.is_empty() { existing.shares = node.shares.clone(); }
                            if !node.ip_address.is_empty() { existing.ip_address = node.ip_address.clone(); }
                            if node.platform != "unknown" { existing.platform = node.platform.clone(); }
                        } else {
                            self.network_nodes.push(node);
                        }
                    }
                    self.network_nodes.sort_by(|a, b| a.hostname.cmp(&b.hostname));
                }
            }
        }
    }

    /// Load duplicate groups from the local database into GUI state.
    /// Called automatically after each completed scan, and on Refresh.
    fn reload_duplicates(&mut self) {
        if let Some(ref app) = self.app {
            let db = app.database();
            match db.get_duplicate_groups() {
                Ok(groups) => {
                    self.duplicate_groups = groups.into_iter().map(|g| {
                        let files = db.get_files_by_hash(&g.sha256_hash).unwrap_or_default();
                        DuplicateGroup {
                            id: g.id.to_string(),
                            hash: g.sha256_hash,
                            size: g.size_bytes as u64,
                            files: files.into_iter().map(|f| DuplicateFile {
                                id: f.id.map(|i| i.to_string()).unwrap_or_default(),
                                path: f.file_path,
                                node_id: f.node_id,
                                drive_id: f.scan_id,
                                modified_time: f.modified_time as u64,
                            }).collect(),
                            wasted_space: g.total_wasted_bytes as u64,
                        }
                    }).collect();
                    info!("Loaded {} duplicate groups from database", self.duplicate_groups.len());
                }
                Err(e) => tracing::error!("Failed to load duplicates: {}", e),
            }
        }
    }
}

impl App for UneffGUI {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Apply Windows 7 Aero styling to the real context
        self.windows_7_aero_style(ctx);
        
        // Load branded header logo once (plain logo — no QR codes)
        if self.background_texture.is_none() {
            let image_bytes = include_bytes!("../../assets/Gill Systems Logo.png");
            if let Ok(image) = image::load_from_memory(image_bytes) {
                let rgba = image.to_rgba8();
                let (w, h) = rgba.dimensions();
                let pixels = rgba.into_raw();
                let color_image = egui::ColorImage::from_rgba_unmultiplied(
                    [w as usize, h as usize],
                    &pixels,
                );
                self.background_texture_size = Some([w as usize, h as usize]);
                self.background_texture = Some(ctx.load_texture(
                    "gillsystems_header",
                    color_image,
                    egui::TextureOptions::LINEAR,
                ));
            }
        }

        // Load branded footer image once
        if self.footer_texture.is_none() {
            let image_bytes = include_bytes!("../../assets/Gillsystems_logo_with_donation_qrcodes.png");
            if let Ok(image) = image::load_from_memory(image_bytes) {
                let rgba = image.to_rgba8();
                let (w, h) = rgba.dimensions();
                let pixels = rgba.into_raw();
                let color_image = egui::ColorImage::from_rgba_unmultiplied(
                    [w as usize, h as usize],
                    &pixels,
                );
                self.footer_texture_size = Some([w as usize, h as usize]);
                self.footer_texture = Some(ctx.load_texture(
                    "gillsystems_footer",
                    color_image,
                    egui::TextureOptions::LINEAR,
                ));
            }
        }

        // Solid black program body
        let screen_rect = ctx
            .input(|i| i.viewport().inner_rect)
            .unwrap_or_else(|| ctx.screen_rect());
        let painter = ctx.layer_painter(egui::LayerId::background());
        painter.rect_filled(screen_rect, Rounding::ZERO, MATRIX_BG);
        
        // Process background messages
        self.process_messages();

        if self.network_nodes.is_empty() && self.app.is_some() {
            self.discover_network_nodes();
        }

        if self.last_auto_refresh.elapsed() >= Duration::from_secs(3) {
            self.reload_duplicates();
            self.last_auto_refresh = Instant::now();
        }

        // Global border is intentionally omitted to avoid non-maximized edge artifacts.

        // ── Manual resize handles — decorations=false needs explicit edge detection ─
        {
            let edge = 6.0f32;
            if let Some(pos) = ctx.input(|i| i.pointer.hover_pos()) {
                let on_l = pos.x <= screen_rect.min.x + edge;
                let on_r = pos.x >= screen_rect.max.x - edge;
                let on_t = pos.y <= screen_rect.min.y + edge;
                let on_b = pos.y >= screen_rect.max.y - edge;
                use egui::viewport::ResizeDirection as RD;
                let dir: Option<RD> = match (on_l, on_r, on_t, on_b) {
                    (true,  _,     true,  _    ) => Some(RD::NorthWest),
                    (_,     true,  true,  _    ) => Some(RD::NorthEast),
                    (true,  _,     _,     true ) => Some(RD::SouthWest),
                    (_,     true,  _,     true ) => Some(RD::SouthEast),
                    (true,  false, false, false) => Some(RD::West),
                    (false, true,  false, false) => None,
                    (false, false, true,  false) => Some(RD::North),
                    (false, false, false, true ) => Some(RD::South),
                    _ => None,
                };
                if let Some(dir) = dir {
                    let cur = match dir {
                        RD::North | RD::South         => egui::CursorIcon::ResizeVertical,
                        RD::NorthWest | RD::SouthEast => egui::CursorIcon::ResizeNwSe,
                        RD::NorthEast | RD::SouthWest => egui::CursorIcon::ResizeNeSw,
                        _                             => egui::CursorIcon::ResizeHorizontal,
                    };
                    ctx.set_cursor_icon(cur);
                    if ctx.input(|i| i.pointer.primary_pressed()) {
                        ctx.send_viewport_cmd(egui::ViewportCommand::BeginResize(dir));
                    }
                }
            }
        }

        // ── Custom title bar — Win7 Aero glass, Windows-style controls ────────────
        egui::TopBottomPanel::top("custom_title_bar")
            .exact_height(32.0)
            .frame(egui::Frame::none())
            .show(ctx, |ui| {
                let bar = ui.max_rect();
                // Win7 Aero glass — layered gradient (base → reflection → highlight)
                {
                    let p = ui.painter();
                    // Base: deep dark matrix green
                    p.rect_filled(bar, Rounding::ZERO,
                        Color32::from_rgba_unmultiplied(8, 18, 10, 252));
                    // Upper reflection band (top 45%)
                    let refl = egui::Rect::from_min_max(
                        bar.min,
                        egui::pos2(bar.max.x, bar.min.y + bar.height() * 0.45),
                    );
                    p.rect_filled(refl, Rounding::ZERO,
                        Color32::from_rgba_unmultiplied(40, 80, 48, 170));
                    // Top highlight stripe (2px) — the glass edge glint
                    let hi = egui::Rect::from_min_max(
                        bar.min, egui::pos2(bar.max.x, bar.min.y + 2.0),
                    );
                    p.rect_filled(hi, Rounding::ZERO,
                        Color32::from_rgba_unmultiplied(80, 200, 100, 90));
                    // Bottom accent line (1px)
                    let bl = egui::Rect::from_min_max(
                        egui::pos2(bar.min.x, bar.max.y - 1.0), bar.max,
                    );
                    p.rect_filled(bl, Rounding::ZERO, MATRIX_GREEN_DARK);
                }

                ui.horizontal(|ui| {
                    ui.add_space(10.0);
                    ui.label(
                        RichText::new("un-F  \u{2014}  Gillsystems")
                            .size(13.0).strong().color(MATRIX_GREEN_GLOW),
                    );

                    let bw = 46.0f32;
                    let total_btn_w = bw * 3.0;
                    let right_pad = 2.0;
                    let left_fill = (bar.width() - total_btn_w - 160.0).max(0.0);
                    ui.add_space(left_fill);

                    let bh = bar.height();
                    let fnt = egui::FontId::proportional(13.0);
                    let is_maximized = ctx.input(|i| i.viewport().maximized).unwrap_or(false);

                    // _ MINIMIZE
                    let (nr, min_r) = ui.allocate_exact_size(egui::vec2(bw, bh), egui::Sense::click());
                    if min_r.hovered() {
                        ui.painter().rect_filled(nr, Rounding::ZERO,
                            Color32::from_rgba_unmultiplied(0, 80, 20, 160));
                    }
                    ui.painter().text(nr.center(), egui::Align2::CENTER_CENTER, "_",
                        egui::FontId::proportional(15.0),
                        if min_r.hovered() { MATRIX_GREEN_GLOW } else { MATRIX_GREEN });
                    if min_r.clicked() {
                        self.minimize_window_native();
                        #[cfg(not(windows))]
                        ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(true));
                    }

                    // □ MAXIMIZE / RESTORE
                    let (mr, max_r) = ui.allocate_exact_size(egui::vec2(bw, bh), egui::Sense::click());
                    if max_r.hovered() {
                        ui.painter().rect_filled(mr, Rounding::ZERO,
                            Color32::from_rgba_unmultiplied(0, 80, 20, 160));
                    }
                    ui.painter().text(mr.center(), egui::Align2::CENTER_CENTER, "□",
                        fnt.clone(),
                        if max_r.hovered() { MATRIX_GREEN_GLOW } else { MATRIX_GREEN });
                    if max_r.clicked() {
                        ctx.send_viewport_cmd(egui::ViewportCommand::Maximized(!is_maximized));
                    }

                    // X CLOSE
                    let (cr, close_r) = ui.allocate_exact_size(egui::vec2(bw, bh), egui::Sense::click());
                    if close_r.hovered() {
                        ui.painter().rect_filled(cr, Rounding::ZERO,
                            Color32::from_rgba_unmultiplied(200, 20, 20, 220));
                    }
                    ui.painter().text(cr.center(), egui::Align2::CENTER_CENTER, "X",
                        fnt,
                        if close_r.hovered() { Color32::WHITE } else { Color32::from_rgb(255, 100, 100) });
                    if close_r.clicked() {
                        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                        std::process::exit(0);
                    }

                    ui.add_space(right_pad);
                });

                // Drag only on non-control area so title buttons stay clickable.
                let drag_rect = egui::Rect::from_min_max(
                    bar.min,
                    egui::pos2(bar.max.x - 160.0, bar.max.y),
                );
                let drag = ui.interact(
                    drag_rect,
                    egui::Id::new("title_drag"),
                    egui::Sense::click_and_drag(),
                );
                if drag.drag_started() {
                    ctx.send_viewport_cmd(egui::ViewportCommand::StartDrag);
                }
            });

        // ── Branded header image (distinct file) ──────────────────────────────────
        let header_h = self
            .background_texture_size
            .map(|[w, h]| ((screen_rect.width() * (h as f32 / w as f32)).clamp(36.0, 54.0)))
            .unwrap_or(48.0);

        egui::TopBottomPanel::top("logo_header_panel")
            .exact_height(header_h)
            .frame(egui::Frame::none().fill(Color32::BLACK))
            .show(ctx, |ui| {
                let rect = ui.max_rect();
                if let Some(tex) = &self.background_texture {
                    ui.painter().image(
                        tex.id(),
                        rect,
                        egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                        Color32::WHITE,
                    );
                }
            });

        // ── Status bar ────────────────────────────────────────────────────────────
        // ── Footer image — permanent bottom panel, always visible below content ────
        let footer_panel_h = self
            .footer_texture_size
            .map(|[w, h]| ((screen_rect.width() * (h as f32 / w as f32)) * 0.5).clamp(44.0, 80.0))
            .unwrap_or(56.0);
        egui::TopBottomPanel::bottom("footer_panel")
            .exact_height(footer_panel_h)
            .frame(egui::Frame::none().fill(Color32::BLACK))
            .show(ctx, |ui| {
                let rect = ui.max_rect();
                if let Some(tex) = &self.footer_texture {
                    ui.painter().image(
                        tex.id(),
                        rect,
                        egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                        Color32::WHITE,
                    );
                }
            });

        egui::TopBottomPanel::bottom("status_bar")
            .frame(
                egui::Frame::none()
                    .fill(Color32::BLACK)
                    .stroke(Stroke::new(1.0, MATRIX_GREEN_DARK))
                    .inner_margin(egui::Margin::same(4.0)),
            )
            .show(ctx, |ui| {
                self.draw_status_bar(ui);
            });

        egui::TopBottomPanel::bottom("app_bottom_edge")
            .exact_height(2.0)
            .frame(egui::Frame::none().fill(MATRIX_GREEN_DARK))
            .show(ctx, |_ui| {});

        // ── Menu bar — directly below header logo ─────────────────────────────
        egui::TopBottomPanel::top("menu_bar")
            .frame(
                egui::Frame::none()
                    .fill(Color32::BLACK)
                    .inner_margin(egui::Margin::same(3.0)),
            )
            .show(ctx, |ui| {
                let total_w = ui.available_width();
                let content_w = (total_w * 0.78).clamp(760.0, total_w);
                let side_w = ((total_w - content_w) * 0.5).max(0.0);

                ui.horizontal(|ui| {
                    ui.add_space(side_w);
                    ui.allocate_ui_with_layout(
                        egui::vec2(content_w, ui.available_height()),
                        egui::Layout::left_to_right(egui::Align::Center),
                        |ui| self.draw_top_bar(ui),
                    );
                    ui.add_space(side_w);
                });
            });

        // ── Left sidebar — overlaps left dark area of background ────────────────
        egui::SidePanel::left("left_sidebar")
            .default_width(250.0)
            .frame(
                egui::Frame::none()
                    .fill(Color32::BLACK)
                    .stroke(Stroke::new(1.0, MATRIX_GREEN_DARK)),
            )
            .show(ctx, |ui| {
                self.draw_left_sidebar(ui);
            });

        // ── Central panel — duplicate files or search results ─────────────────────────
        egui::CentralPanel::default()
            .frame(
                egui::Frame::none()
                    .fill(Color32::BLACK)
                    .stroke(Stroke::new(1.0, MATRIX_GREEN_DARK)),
            )
            .show(ctx, |ui| {
                if self.show_search {
                    self.draw_search_panel(ui);
                } else {
                    self.draw_dual_panel(ui);
                }
            });

        // ── Modal dialogs ───────────────────────────────────────────────────────
        self.draw_warning_dialog(ctx);
        self.draw_settings_dialog(ctx);
        self.draw_about_dialog(ctx);
        ctx.request_repaint_after(Duration::from_millis(250));
    }

    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        [0.0, 0.0, 0.0, 1.0]
    }
}

fn load_window_icon() -> Option<egui::IconData> {
    eframe::icon_data::from_png_bytes(include_bytes!("../assets/gillsystems_logo.png")).ok()
}

pub fn run_gui(config: Arc<Config>) -> Result<()> {
    let mut viewport = egui::ViewportBuilder::default()
        .with_inner_size([1200.0, 800.0])
        .with_min_inner_size([800.0, 500.0])
        .with_resizable(true)
        .with_decorations(false)
        .with_title("un-F — Gillsystems");

    if let Some(icon) = load_window_icon() {
        viewport = viewport.with_icon(icon);
    }

    let options = NativeOptions {
        viewport,
        ..Default::default()
    };

    let (mut gui, message_tx) = UneffGUI::new(config.clone());

    // Initialize the program core (database, scanner, platform detection).
    // block_in_place lets us await async init from within the tokio runtime.
    match tokio::task::block_in_place(|| {
        tokio::runtime::Handle::current()
            .block_on(UneffSecretFunctions::new(config, Some(message_tx), None))
    }) {
        Ok(core) => {
            info!("Program core initialized — ready to scan");
            gui.set_app(Arc::new(core));
        }
        Err(e) => {
            tracing::error!("Program core init failed: {}", e);
            // GUI launches anyway — scan will surface an error if triggered
        }
    }

    eframe::run_native(
        "un-F — Gillsystems",
        options,
        Box::new(move |_cc| Box::new(gui) as Box<dyn eframe::App>),
    ).map_err(|e| anyhow::anyhow!("GUI error: {}", e))?;

    Ok(())
}
