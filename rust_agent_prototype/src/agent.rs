// Gillsystems_uneff_your_rigs_messy_files — Agent Core Module
// Created by: Master Dev 2 (Systems Core Engineer)
// Philosophy: Systems Should Serve Humans — Power to the People!
//
// This module contains the UneffAgent struct — the heart of the application.
// It owns the scanning pipeline, database, network service, and remediation engine.

use anyhow::Result;
use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::{info, error};

use crate::config::Config;
use crate::database::Database;
use crate::file_scanner::FileScanner;
use crate::gui::GuiMessage;
use crate::service::GrpcService;

/// The core agent that orchestrates all subsystems.
/// Single binary, single responsibility: un-eff your rigs.
pub struct UneffAgent {
    config: Arc<Config>,
    database: Database,
    scanner: FileScanner,
    grpc_service: Option<GrpcService>,
    gui_tx: Option<mpsc::UnboundedSender<GuiMessage>>,
}

impl UneffAgent {
    /// Create a new agent instance.
    /// Full admin assumed — no permission checks, no gatekeeping.
    pub async fn new(
        config: Arc<Config>,
        gui_tx: Option<mpsc::UnboundedSender<GuiMessage>>,
        _progress_tx: Option<mpsc::Sender<String>>,
    ) -> Result<Self> {
        info!("Initializing UneffAgent — Systems Should Serve Humans");

        let database = Database::new(&config.database)?;
        let scanner = FileScanner::new(Arc::new(config.scanning.clone()));

        Ok(Self {
            config,
            database,
            scanner,
            grpc_service: None,
            gui_tx,
        })
    }

    /// Run the agent as a background service.
    /// Full speed, no brakes, all CPU cores.
    pub async fn run_service(&self) -> Result<()> {
        info!("UneffAgent service starting on port {}", self.config.grpc_port);

        // TODO: DEVELOP phase — implement gRPC server startup
        // TODO: DEVELOP phase — implement peer discovery
        // TODO: DEVELOP phase — implement scheduled scanning

        // Keep service alive
        tokio::signal::ctrl_c().await?;
        info!("UneffAgent service shutting down gracefully");
        Ok(())
    }

    /// Get local drives for GUI sidebar display.
    pub fn get_local_drives(&self) -> Result<Vec<crate::gui::DriveInfo>> {
        // TODO: DEVELOP phase — enumerate drives via platform module
        Ok(Vec::new())
    }
}
