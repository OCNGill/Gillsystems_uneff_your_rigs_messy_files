// Gillsystems_uneff_your_rigs_messy_files — gRPC Service Module
// Created by: Master Dev 4 (Network & Platform Engineer)
// Philosophy: Peer-to-peer — no central authority, no central orchestrator.
//
// Implements the UneffAgent gRPC service defined in agent_service.proto.
// Uses tonic for high-performance gRPC with mTLS authentication.
// Every node is equal. Every node is sovereign.

use anyhow::Result;
use tonic::{Request, Response, Status};
use tracing::{info, error};

// The generated proto module will be created by build.rs / tonic-build
// pub mod proto {
//     tonic::include_proto!("gillsystems_uneff");
// }

/// The gRPC service implementation for peer-to-peer communication.
/// Handles: StartScan, GetSystemInfo, ExecuteRemediation, HealthCheck, GetMounts, StopScan.
pub struct GrpcService {
    port: u16,
}

impl GrpcService {
    pub fn new(port: u16) -> Self {
        Self { port }
    }

    /// Start the gRPC server with mTLS.
    /// Binds to all interfaces — full admin assumed.
    pub async fn start(&self) -> Result<()> {
        info!("Starting gRPC service on port {} — peer-to-peer, no central authority", self.port);

        // TODO: DEVELOP phase — tonic::transport::Server builder
        // TODO: DEVELOP phase — load TLS certs for mTLS
        // TODO: DEVELOP phase — register UneffAgentServer
        // TODO: DEVELOP phase — serve on 0.0.0.0:{port}

        Ok(())
    }
}

// TODO: DEVELOP phase — implement tonic::codegen traits for each RPC:
//   - StartScan: Stream scan progress back to caller
//   - GetSystemInfo: Return hostname, platform, drives, memory
//   - ExecuteRemediation: Quarantine/delete/hardlink/move
//   - HealthCheck: Return uptime, metrics, status
//   - GetMounts: Return all mounted filesystems
//   - StopScan: Cancel an in-progress scan
