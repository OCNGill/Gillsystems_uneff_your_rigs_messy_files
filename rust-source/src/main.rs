#![allow(dead_code)]
#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]
//! # Gillsystems_uneff_your_rigs_messy_files — Un-eff your rigs!
//!
//! **Version**: 0.4.0 (Documentation Phase)
//! **Philosophy**: Systems Should Serve Humans — Power to the People!
//!
//! A cross-platform agent for finding and eliminating duplicate files intelligently.
//! 
//! ## Entry Points
//! - **GUI Mode** (default): `--gui-only` or no arguments
//! - **Service Mode**: `--service` — start gRPC peer listening for cluster commands
//! - **Headless**: Internally, the agent can be used programmatically
//!
//! ## Features
//! - **Fast duplicate detection**: xxHash64 (pre-filter) → SHA-256 (verify)
//! - **Platform-optimized remediation**: ZFS clone, NTFS hard link, ext4 reflink
//! - **Full audit trail**: Every operation logged with SHA-256 verification
//! - **Peer-to-peer**: No central authority, every node is sovereign
//! - **Windows 7 Aero UI**: Responsive, real-time duplicate visualization
//!
//! ## Architecture (10 modules)
//! - [`agent`]: Orchestration (scanning, remediation, dedup pipeline)
//! - [`database`]: SQLite storage (nodes, drives, files, scans, duplicates, audit log)
//! - [`file_scanner`]: Parallel filesystem walk + progressive hashing
//! - [`hashing`]: Two-stage (xxHash64 + SHA-256) fingerprinting
//! - [`remediation`]: Intelligent dedup (ZFS clone, hard link, quarantine, delete)
//! - [`platform`]: Cross-platform (ZFS, NTFS, ext4, XFS, FAT32, APFS)
//! - [`config`]: TOML-based runtime configuration
//! - [`service`]: gRPC peer-to-peer API
//! - [`gui`]: egui-based Windows 7 Aero theme UI
//!
//! ## Building
//! ```bash
//! cargo build --release  # 4.68 MB standalone binary
//! cargo test             # Run 5 comprehensive test suites
//! cargo clippy           # Check for lint warnings
//! ```
//!
//! ## Usage
//! ```bash
//! # GUI mode (default)
//! ./uneff-your-rigs
//!
//! # Service mode (cluster peer)
//! ./uneff-your-rigs --service
//!
//! # GUI only (no service listening)
//! ./uneff-your-rigs --gui-only
//! ```
//!
//! ## Philosophy
//! > Systems Should Serve Humans, Not The Reverse
//! 
//! - **Radical Transparency**: Every byte, every hash, every decision visible
//! - **User Empowerment**: Honest warnings, never silent deletions
//! - **Full Speed**: All CPU cores, no throttling, no artificial limits
//! - **Peer-to-Peer**: No cloud, no phone-home, no central authority
//!
//! ## Safety
//! - **Fully Reversible**: All operations reversible except explicit delete
//! - **SHA-256 Verification**: Atomic verification after each operation
//! - **Quarantine First**: Safe staging area before any destructive operation
//! - **Audit Logging**: Every action recorded with timestamps and node IDs
//!
//! For detailed documentation of each module, see module doc-comments.
//! For platform-specific details, see [`platform`] module.
//! For remediation strategies, see [`remediation`] module.

use anyhow::Result;
use clap::{Arg, Command};
use std::sync::Arc;
use tracing::{error, info};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

mod uneff_program;
mod boot_screen;
mod config;
mod database;
mod file_scanner;
mod gui;
mod hashing;
mod platform;
mod remediation;
mod service;
mod smb_server;

use uneff_program::UneffSecretFunctions;
use boot_screen::{BootMode, BootScreen};
use config::Config;
use gui::run_gui;
use smb_server::SMBServer;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "gillsystems-uneff-your-rigs-messy-files=info,tower_http=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    // Check if running with CLI arguments
    let args: Vec<String> = std::env::args().collect();
    
    // If explicit CLI argument, skip boot screen
    if args.len() > 1 {
        return run_cli_mode().await;
    }

    // Otherwise, show boot screen
    info!("Starting Gillsystems_uneff_your_rigs_messy_files v0.4.0 — Deliver Phase");
    
    // Do not force elevation here — launch immediately and let user continue.
    if !BootScreen::check_permissions().unwrap_or(false) {
        info!("Running without elevation. Some protected paths may be inaccessible.");
    }

    // Run the boot launcher — user picks Full GUI, Silent, or SMB
    let boot_mode = match boot_screen::run_boot_screen() {
        Ok(mode) => mode,
        Err(e) => {
            error!("Boot screen failed: {} — defaulting to GUI", e);
            BootMode::LaunchGUI
        }
    };

    // Load configuration
    let config = Arc::new(Config::load("config.toml")?);
    config.validate()?;

    match boot_mode {
        BootMode::LaunchGUI => {
            info!("Launching GUI mode");
            run_gui(config)
        }
        BootMode::LaunchService => {
            info!("Launching service mode");
            run_service_mode(config).await
        }
        BootMode::SetupSMB => {
            info!("Launching SMB setup");
            run_smb_setup().await
        }
        BootMode::ShowBootScreen => {
            // This shouldn't be reached
            error!("Boot screen selection incomplete");
            Err(anyhow::anyhow!("No mode selected"))
        }
    }
}

async fn run_cli_mode() -> Result<()> {
    // Parse command line arguments
    let matches = Command::new("gillsystems-uneff-your-rigs-messy-files")
        .version("0.4.0")
        .about("Gillsystems_uneff_your_rigs_messy_files - Cross-platform duplicate file management - Power to the people!")
        .arg(
            Arg::new("config")
                .short('c')
                .long("config")
                .value_name("FILE")
                .help("Configuration file path")
                .default_value("config.toml"),
        )
        .arg(
            Arg::new("service-mode")
                .short('s')
                .long("service")
                .help("Run as background service only")
                .action(clap::ArgAction::SetTrue),
        )
        .arg(
            Arg::new("gui-only")
                .short('g')
                .long("gui-only")
                .help("Run GUI only (connect to existing service)")
                .action(clap::ArgAction::SetTrue),
        )
        .get_matches();

    // Load configuration
    let config_path = matches.get_one::<String>("config").unwrap();
    let config = Arc::new(Config::load(config_path)?);
    config.validate()?;

    // Handle different run modes
    let service_mode = matches.get_flag("service-mode");
    let gui_only = matches.get_flag("gui-only");

    match (service_mode, gui_only) {
        (true, false) => {
            // Service mode only
            info!("Starting Gillsystems_uneff_your_rigs_messy_files in service mode");
            run_service_mode(config).await
        }
        (false, true) => {
            // GUI mode only
            info!("Starting Gillsystems_uneff_your_rigs_messy_files GUI only");
            run_gui(config)
        }
        (false, false) => {
            // Full mode: service + GUI
            info!("Starting Gillsystems_uneff_your_rigs_messy_files with GUI and service");
            run_full_mode(config).await
        }
        (true, true) => {
            error!("Cannot specify both --service and --gui-only");
            Err(anyhow::anyhow!("Conflicting arguments: --service and --gui-only"))
        }
    }
}

async fn run_service_mode(config: Arc<Config>) -> Result<()> {
    let app = UneffSecretFunctions::new(config, None, None).await?;

    // Register service
    #[cfg(unix)]
    {
        platform::unix::register_service()?;
    }
    
    #[cfg(windows)]
    {
        platform::windows::register_service()?;
    }
    
    // Run service
    app.run_service().await?;

    Ok(())
}

async fn run_full_mode(config: Arc<Config>) -> Result<()> {
    // Start background gRPC service
    let svc_config = config.clone();
    let svc_handle = tokio::spawn(async move {
        if let Ok(core) = UneffSecretFunctions::new(svc_config, None, None).await {
            if let Err(e) = core.run_service().await {
                error!("Background service failed: {}", e);
            }
        }
    });

    // Give service time to start
    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

    // Start GUI (creates its own app core internally)
    if let Err(e) = run_gui(config) {
        error!("GUI failed: {}", e);
    }

    svc_handle.abort();

    Ok(())
}
async fn run_smb_setup() -> Result<()> {
    info!("Starting SMB server setup");
    
    // Check if SMB is available
    if !SMBServer::is_available()? {
        error!("SMB not available on this system");
        println!("To enable SMB server:");
        
        #[cfg(target_os = "linux")]
        println!("  sudo apt-get install samba samba-common");
        
        #[cfg(target_os = "macos")]
        println!("  SMB is built-in on macOS 10.5+");
        
        #[cfg(target_os = "windows")]
        println!("  SMB is built-in on Windows 7+");
        
        return Err(anyhow::anyhow!("SMB not available"));
    }
    
    // Create SMB server instance with all available drives
    let share_path = std::env::temp_dir();
    let available_drives = SMBServer::get_available_drives()?;
    
    // Generate unique share name for first drive as example
    let share_name = if let Some(first_drive) = available_drives.first() {
        SMBServer::generate_unique_share_name(first_drive)?
    } else {
        "uneff-rigs-unknown".to_string()
    };
    
    let mut server = SMBServer::new(
        share_name.clone(),
        share_path.clone(),
        true, // localhost only
        available_drives, // share all available drives
    );
    
    // Check if already shared
    if SMBServer::is_path_shared(&share_path)? {
        println!("✅ Path already shared via SMB");
        println!("   Connection: {}", server.get_connection_string());
        return Ok(());
    }
    
    // Start SMB server
    server.start()?;
    
    println!("✅ SMB server started successfully!");
    println!("   Share Name: uneff-rigs");
    println!("   Path: {}", share_path.display());
    println!("   Connection: {}", server.get_connection_string());
    println!("   Access: Localhost only (secure)");
    
    info!("SMB setup complete");
    Ok(())
}