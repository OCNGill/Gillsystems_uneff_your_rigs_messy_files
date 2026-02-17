use anyhow::Result;
use clap::{Arg, Command};
use std::env;
use std::sync::Arc;
use tracing::{error, info};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

mod agent;
mod config;
mod database;
mod file_scanner;
mod gui;
mod hashing;
mod platform;
mod remediation;
mod service;

use agent::UneffAgent;
use config::Config;
use gui::run_gui;

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

    // Parse command line arguments
    let matches = Command::new("gillsystems-uneff-your-rigs-messy-files")
        .version("0.1.0")
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
    let agent = UneffAgent::new(config, None, None).await?;
    
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
    agent.run_service().await?;
    
    Ok(())
}

async fn run_full_mode(config: Arc<Config>) -> Result<()> {
    // Start agent in background
    let agent_config = config.clone();
    let agent_handle = tokio::spawn(async move {
        if let Ok(agent) = UneffAgent::new(agent_config, None, None).await {
            if let Err(e) = agent.run_service().await {
                error!("Agent service failed: {}", e);
            }
        }
    });
    
    // Give agent time to start
    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
    
    // Start GUI
    if let Err(e) = run_gui(config) {
        error!("GUI failed: {}", e);
    }
    
    // Shutdown agent
    agent_handle.abort();
    
    Ok(())
}
