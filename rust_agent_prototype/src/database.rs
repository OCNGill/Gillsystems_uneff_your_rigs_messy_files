// Gillsystems_uneff_your_rigs_messy_files — Database Module
// Created by: Master Dev 3 (Data Pipeline Engineer)
// Philosophy: Radical Transparency — every hash, every path, every byte visible to you.
//
// Local SQLite database with WAL mode for maximum write performance.
// No cloud. No phone-home. Your data stays on your machine.

use anyhow::{Context, Result};
use rusqlite::{Connection, params};
use std::path::Path;
use tracing::{info, error};

use crate::config::DatabaseConfig;

/// Local SQLite database — sovereign data storage.
/// Peer-to-peer sync between nodes, but every node owns its own copy.
pub struct Database {
    conn: Connection,
}

impl Database {
    /// Open or create the local database.
    /// WAL mode enabled by default for concurrent read/write performance.
    pub fn new(config: &DatabaseConfig) -> Result<Self> {
        let conn = Connection::open(&config.path)
            .context("Failed to open SQLite database")?;

        // Enable WAL mode for performance — no brakes
        if config.wal_mode {
            conn.execute_batch("PRAGMA journal_mode=WAL;")?;
        }

        // Set cache size
        conn.execute_batch(&format!(
            "PRAGMA cache_size=-{};",
            config.cache_size_mb * 1024 // Convert MB to KB (negative = KB)
        ))?;

        let db = Self { conn };
        db.initialize_schema()?;

        info!("Database initialized at: {}", config.path);
        Ok(db)
    }

    /// Create all tables and indexes if they don't exist.
    /// Schema matches architecture_design.md specification exactly.
    fn initialize_schema(&self) -> Result<()> {
        self.conn.execute_batch(
            "
            CREATE TABLE IF NOT EXISTS nodes (
                id TEXT PRIMARY KEY,
                hostname TEXT NOT NULL,
                ip_address TEXT NOT NULL,
                platform TEXT NOT NULL,
                version TEXT NOT NULL,
                last_seen INTEGER NOT NULL,
                status TEXT DEFAULT 'offline',
                total_drives INTEGER DEFAULT 0,
                total_space INTEGER DEFAULT 0,
                available_space INTEGER DEFAULT 0,
                capabilities TEXT,
                created_at INTEGER DEFAULT (strftime('%s', 'now'))
            );

            CREATE TABLE IF NOT EXISTS drives (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                node_id TEXT NOT NULL,
                drive_letter TEXT,
                mount_point TEXT,
                drive_type TEXT NOT NULL,
                filesystem_type TEXT,
                total_space INTEGER NOT NULL,
                available_space INTEGER NOT NULL,
                is_removable BOOLEAN DEFAULT FALSE,
                is_network BOOLEAN DEFAULT FALSE,
                label TEXT,
                serial_number TEXT,
                last_scanned INTEGER,
                FOREIGN KEY (node_id) REFERENCES nodes(id)
            );

            CREATE TABLE IF NOT EXISTS scans (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                node_id TEXT NOT NULL,
                initiated_by TEXT NOT NULL,
                started_at INTEGER NOT NULL,
                completed_at INTEGER,
                status TEXT DEFAULT 'running',
                files_scanned INTEGER DEFAULT 0,
                bytes_scanned INTEGER DEFAULT 0,
                error_count INTEGER DEFAULT 0,
                config TEXT,
                FOREIGN KEY (node_id) REFERENCES nodes(id)
            );

            CREATE TABLE IF NOT EXISTS files (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                node_id TEXT NOT NULL,
                drive_id INTEGER NOT NULL,
                file_path TEXT NOT NULL,
                file_name TEXT NOT NULL,
                size_bytes INTEGER NOT NULL,
                modified_time INTEGER NOT NULL,
                permissions TEXT,
                xxhash64 TEXT,
                sha256_hash TEXT NOT NULL,
                is_deleted BOOLEAN DEFAULT FALSE,
                scan_id INTEGER NOT NULL,
                discovered_at INTEGER DEFAULT (strftime('%s', 'now')),
                FOREIGN KEY (node_id) REFERENCES nodes(id),
                FOREIGN KEY (drive_id) REFERENCES drives(id),
                FOREIGN KEY (scan_id) REFERENCES scans(id)
            );

            CREATE TABLE IF NOT EXISTS duplicate_groups (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                sha256_hash TEXT NOT NULL UNIQUE,
                size_bytes INTEGER NOT NULL,
                file_count INTEGER NOT NULL,
                total_wasted_bytes INTEGER NOT NULL,
                first_seen_at INTEGER DEFAULT (strftime('%s', 'now')),
                last_updated_at INTEGER DEFAULT (strftime('%s', 'now'))
            );

            CREATE TABLE IF NOT EXISTS duplicate_files (
                group_id INTEGER NOT NULL,
                file_id INTEGER NOT NULL,
                is_primary BOOLEAN DEFAULT FALSE,
                remediation_status TEXT DEFAULT 'none',
                remediation_at INTEGER,
                FOREIGN KEY (group_id) REFERENCES duplicate_groups(id),
                FOREIGN KEY (file_id) REFERENCES files(id),
                PRIMARY KEY (group_id, file_id)
            );

            CREATE TABLE IF NOT EXISTS remediation_actions (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                group_id INTEGER NOT NULL,
                action_type TEXT NOT NULL,
                file_ids TEXT NOT NULL,
                initiated_by_node TEXT NOT NULL,
                executed_by_node TEXT NOT NULL,
                initiated_at INTEGER NOT NULL,
                completed_at INTEGER,
                status TEXT DEFAULT 'pending',
                space_recovered INTEGER,
                error_message TEXT,
                FOREIGN KEY (group_id) REFERENCES duplicate_groups(id)
            );

            CREATE TABLE IF NOT EXISTS audit_log (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                action TEXT NOT NULL,
                resource_type TEXT NOT NULL,
                resource_id TEXT NOT NULL,
                details TEXT,
                node_id TEXT,
                timestamp INTEGER DEFAULT (strftime('%s', 'now'))
            );

            -- Performance indexes
            CREATE INDEX IF NOT EXISTS idx_files_sha256 ON files(sha256_hash);
            CREATE INDEX IF NOT EXISTS idx_files_size ON files(size_bytes);
            CREATE INDEX IF NOT EXISTS idx_files_scan ON files(scan_id);
            CREATE INDEX IF NOT EXISTS idx_files_node_path ON files(node_id, file_path);
            CREATE INDEX IF NOT EXISTS idx_duplicate_groups_hash ON duplicate_groups(sha256_hash);
            CREATE INDEX IF NOT EXISTS idx_duplicate_groups_wasted ON duplicate_groups(total_wasted_bytes DESC);
            CREATE INDEX IF NOT EXISTS idx_duplicate_files_group ON duplicate_files(group_id);
            CREATE INDEX IF NOT EXISTS idx_audit_timestamp ON audit_log(timestamp DESC);
            "
        )?;

        info!("Database schema initialized — all tables and indexes ready");
        Ok(())
    }
}
