use anyhow::Result;
use eframe::{egui, App, NativeOptions};
use egui::{Color32, RichText, Stroke, Vec2, Rounding};
use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::info;

use crate::{
    agent::UneffAgent,
    config::Config,
    file_scanner::{ScanProgress, ScanStatus},
};

// Windows 7 Aero Theme Colors
const AERO_BLUE: Color32 = Color32::from_rgb(0, 102, 204);
const AERO_GLASS: Color32 = Color32::from_rgba_premultiplied(255, 255, 255, 180);
const AERO_HIGHLIGHT: Color32 = Color32::from_rgb(51, 153, 255);
const AERO_DARK: Color32 = Color32::from_rgb(51, 51, 51);

#[derive(Debug, Clone)]
pub enum GuiMessage {
    ScanProgress(ScanProgress),
    NodeDiscovered(String, String),
    NodeOffline(String),
    ShowWarning(String),
}

pub struct UneffGUI {
    config: Arc<Config>,
    agent: Option<UneffAgent>,
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
}

#[derive(Debug, Clone)]
pub struct NodeInfo {
    pub id: String,
    pub hostname: String,
    pub ip_address: String,
    pub platform: String,
    pub status: NodeStatus,
    pub drives: Vec<DriveInfo>,
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

impl UneffGUI {
    pub fn new(config: Arc<Config>) -> (Self, mpsc::UnboundedSender<GuiMessage>) {
        let (message_tx, message_rx) = mpsc::unbounded_channel();
        
        let gui = Self {
            config,
            agent: None,
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
        };
        
        (gui, message_tx)
    }
    
    pub fn set_agent(&mut self, agent: UneffAgent) {
        self.agent = Some(agent);
    }
    
    fn windows_7_aero_style(&self, ctx: &egui::Context) {
        // Windows 7 Aero color scheme — applied to the REAL context
        let mut style = (*ctx.style()).clone();
        
        // Glass effect backgrounds
        style.visuals.panel_fill = AERO_GLASS;
        style.visuals.window_fill = AERO_GLASS;
        style.visuals.window_shadow = egui::epaint::Shadow {
            extrusion: 8.0,
            color: Color32::from_rgba_premultiplied(0, 0, 0, 100),
        };
        
        // Aero-style buttons
        style.visuals.button_frame = true;
        style.visuals.widgets.hovered.bg_fill = AERO_HIGHLIGHT;
        style.visuals.widgets.active.bg_fill = AERO_BLUE;
        style.visuals.widgets.inactive.bg_fill = Color32::from_rgba_premultiplied(255, 255, 255, 120);
        
        // Rounded corners for Aero look
        style.visuals.window_rounding = Rounding::same(6.0);
        style.visuals.widgets.noninteractive.rounding = Rounding::same(4.0);
        style.visuals.widgets.inactive.rounding = Rounding::same(4.0);
        style.visuals.widgets.hovered.rounding = Rounding::same(4.0);
        style.visuals.widgets.active.rounding = Rounding::same(4.0);
        
        // Aero-style selection
        style.visuals.selection.bg_fill = AERO_BLUE;
        style.visuals.selection.stroke = Stroke::new(1.0, AERO_DARK);
        
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
                    open::that("https://github.com/OCNGill/Gillsystems_uneff_your_rigs_messy_files/tree/main/docs").ok();
                }
                ui.separator();
                if ui.button("About").clicked() {
                    self.show_about = true;
                }
            });
            
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                // Quick action buttons
                if ui.button("🔍 Scan").clicked() {
                    self.start_new_scan();
                }
                if ui.button("🗑️ Delete Selected").clicked() {
                    self.show_delete_warning();
                }
                if ui.button("⚙️ Settings").clicked() {
                    self.show_settings = true;
                }
            });
        });
        
        ui.separator();
    }
    
    fn draw_left_sidebar(&mut self, ui: &mut egui::Ui) {
        ui.heading("Network Devices");
        
        // Network nodes section
        egui::ScrollArea::vertical()
            .id_source("network_nodes")
            .show(ui, |ui| {
                for node in &self.network_nodes.clone() {
                    let node_id = node.id.clone();
                    let is_selected = self.selected_node.as_ref() == Some(&node_id);
                    
                    let status_color = match node.status {
                        NodeStatus::Online => Color32::GREEN,
                        NodeStatus::Offline => Color32::RED,
                        NodeStatus::Scanning => Color32::YELLOW,
                    };
                    
                    let response = ui.horizontal(|ui| {
                        ui.colored_label(status_color, "●");
                        if ui.selectable_label(is_selected, &node.hostname).clicked() {
                            self.selected_node = Some(node_id);
                        }
                    }).response;
                    
                    // Show node details on hover
                    response.on_hover_ui(|ui| {
                        ui.label(&format!("IP: {}", node.ip_address));
                        ui.label(&format!("Platform: {}", node.platform));
                        ui.label(&format!("Drives: {}", node.drives.len()));
                    });
                }
            });
        
        ui.separator();
        
        // Local drives section
        ui.heading("Local Drives");
        if let Some(ref agent) = self.agent {
            if let Ok(drives) = agent.get_local_drives() {
                for drive in drives {
                    ui.horizontal(|ui| {
                        ui.label(&drive.label);
                        ui.label(&format!("({})", drive.mount_point));
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.label(format!("{:.1} GB free", drive.available_space as f64 / 1024.0 / 1024.0 / 1024.0));
                        });
                    });
                }
            }
        }
    }
    
    fn draw_dual_panel(&mut self, ui: &mut egui::Ui) {
        let _panel_width = ui.available_width() / 2.0 - 5.0;
        
        ui.horizontal(|ui| {
            // Left panel - Duplicate groups
            ui.vertical(|ui| {
                ui.heading("Duplicate Files");
                ui.separator();
                
                egui::ScrollArea::vertical()
                    .id_source("left_panel")
                    .show(ui, |ui| {
                        for (i, group) in self.duplicate_groups.iter().enumerate() {
                            let _is_selected = self.left_panel_selected == Some(i);
                            
                            let response = ui.horizontal(|ui| {
                                ui.checkbox(&mut false, ""); // TODO: Implement selection
                                ui.label(format!("📁 {} files", group.files.len()));
                                ui.label(format!("{} MB", group.size / 1024 / 1024));
                                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                    ui.label(format!("{} MB wasted", group.wasted_space / 1024 / 1024));
                                });
                            }).response;
                            
                            if response.clicked() {
                                self.left_panel_selected = Some(i);
                                self.right_panel_selected = None;
                            }
                            
                            // Show file list on hover
                            response.on_hover_ui(|ui| {
                                ui.heading("Files in this group:");
                                for file in &group.files {
                                    ui.label(&file.path);
                                }
                            });
                        }
                    });
            });
            
            ui.separator();
            
            // Right panel - File locations
            ui.vertical(|ui| {
                ui.heading("File Locations");
                ui.separator();
                
                if let Some(selected_idx) = self.left_panel_selected {
                    if let Some(group) = self.duplicate_groups.get(selected_idx).cloned() {
                        let right_sel = self.right_panel_selected;
                        let mut delete_id: Option<String> = None;
                        let mut open_path: Option<String> = None;
                        let mut new_right_sel: Option<usize> = None;

                        egui::ScrollArea::vertical()
                            .id_source("right_panel")
                            .show(ui, |ui| {
                                for (i, file) in group.files.iter().enumerate() {
                                    let _is_selected = right_sel == Some(i);
                                    
                                    let response = ui.horizontal(|ui| {
                                        ui.checkbox(&mut false, ""); // TODO: Implement selection
                                        ui.label(&file.path);
                                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                            if ui.button("🗑️").clicked() {
                                                delete_id = Some(file.id.clone());
                                            }
                                            if ui.button("📁").clicked() {
                                                open_path = Some(file.path.clone());
                                            }
                                        });
                                    }).response;
                                    
                                    if response.clicked() {
                                        new_right_sel = Some(i);
                                    }
                                    
                                    // Show metadata on hover
                                    response.on_hover_ui(|ui| {
                                        ui.label(&format!("Node: {}", file.node_id));
                                        ui.label(&format!("Drive: {}", file.drive_id));
                                        ui.label(&format!("Modified: {}", file.modified_time));
                                        ui.label(&format!("Size: {} bytes", group.size));
                                    });
                                }
                            });

                        // Apply deferred actions now that the closure is done
                        if let Some(id) = delete_id {
                            self.delete_file(&id);
                        }
                        if let Some(path) = open_path {
                            self.open_file_location(&path);
                        }
                        if let Some(sel) = new_right_sel {
                            self.right_panel_selected = Some(sel);
                        }
                    }
                } else {
                    ui.centered_and_justified(|ui| {
                        ui.label("Select a duplicate group to view file locations");
                    });
                }
            });
        });
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
    
    fn draw_settings_dialog(&mut self, ctx: &egui::Context) {
        let mut show = self.show_settings;
        if show {
            let mut trigger_reset_warning = false;
            egui::Window::new("Settings")
                .open(&mut show)
                .resizable(true)
                .default_size(Vec2::new(600.0, 400.0))
                .show(ctx, |ui| {
                    ui.heading("Application Settings");
                    ui.separator();
                    
                    ui.horizontal(|ui| {
                        ui.label("Scan Threads:");
                        ui.add(egui::Slider::new(&mut 0, 1..=16).text("threads"));
                    });
                    
                    ui.horizontal(|ui| {
                        ui.label("Max File Size (GB):");
                        ui.add(egui::Slider::new(&mut 0.0, 0.1..=100.0).text("GB"));
                    });
                    
                    ui.separator();
                    
                    ui.heading("Network Settings");
                    ui.horizontal(|ui| {
                        ui.label("Discovery Port:");
                        ui.add(egui::TextEdit::singleline(&mut "50051".to_string()));
                    });
                    
                    ui.separator();
                    
                    ui.heading("Danger Zone");
                    ui.colored_label(Color32::RED, "⚠️ These settings can cause data loss!");
                    
                    if ui.button("Reset All Data").clicked() {
                        trigger_reset_warning = true;
                    }
                });
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
                        ui.label("Version 0.3.0");
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
        info!("Starting new scan");
        if let Some(ref _agent) = self.agent {
            // TODO: Implement scan start
        }
    }
    
    fn discover_network_nodes(&mut self) {
        info!("Discovering network nodes");
        // TODO: Implement network discovery
    }
    
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
            std::process::Command::new("explorer")
                .args(["/select,", path])
                .spawn()
                .ok();
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
        info!("Refreshing view");
        // TODO: Implement refresh
    }
    
    fn show_warning(&mut self, message: String) {
        self.current_warning = Some(message);
    }

    fn process_messages(&mut self) {
        while let Ok(message) = self.message_rx.try_recv() {
            match message {
                GuiMessage::ScanProgress(progress) => {
                    self.scan_progress = Some(progress);
                }
                GuiMessage::NodeDiscovered(id, hostname) => {
                    self.network_nodes.push(NodeInfo {
                        id,
                        hostname,
                        ip_address: String::new(),
                        platform: String::new(),
                        status: NodeStatus::Online,
                        drives: Vec::new(),
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
            }
        }
    }
}

impl App for UneffGUI {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Apply Windows 7 Aero styling to the real context
        self.windows_7_aero_style(ctx);
        
        // Process background messages
        self.process_messages();
        
        // Main layout
        egui::TopBottomPanel::top("top_bar").show(ctx, |ui| {
            self.draw_top_bar(ui);
        });
        
        egui::TopBottomPanel::bottom("status_bar").show(ctx, |ui| {
            self.draw_status_bar(ui);
        });
        
        egui::SidePanel::left("left_sidebar")
            .default_width(250.0)
            .show(ctx, |ui| {
                self.draw_left_sidebar(ui);
            });
        
        egui::CentralPanel::default().show(ctx, |ui| {
            self.draw_dual_panel(ui);
        });
        
        // Modal dialogs
        self.draw_warning_dialog(ctx);
        self.draw_settings_dialog(ctx);
        self.draw_about_dialog(ctx);
    }
}

pub fn run_gui(config: Arc<Config>) -> Result<()> {
    let options = NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1200.0, 800.0])
            .with_resizable(true)
            .with_decorations(true)
            .with_title("Gillsystems_uneff_your_rigs_messy_files"),
        ..Default::default()
    };
    
    let (gui, _message_tx) = UneffGUI::new(config);
    
    eframe::run_native(
        "Gillsystems_uneff_your_rigs_messy_files",
        options,
        Box::new(|_cc| Box::new(gui) as Box<dyn eframe::App>),
    ).map_err(|e| anyhow::anyhow!("GUI error: {}", e))?;
    
    Ok(())
}
