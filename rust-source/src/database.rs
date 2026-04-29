//! # Database Module — Sovereign Data Storage
//!
//! Radical Transparency: every hash, every path, every byte visible to you.
//!
//! Local SQLite database with WAL mode for maximum write performance.
//! No cloud. No phone-home. Your data stays on your machine.
//!
//! ## Tables
//! - `nodes`: Agent instances (node_id, hostname, ip, platform, version)
//! - `drives`: Storage devices per node (drive_letter, name, fs_type, total_size)
//! - `files`: Scanned files (path, size, modified_time, remediation_status)
//! - `scans`: Scan session history (start_time, end_time, files_found)
//! - `duplicate_groups`: Per-scan groups of identical files
//! - `duplicate_files`: Membership table for duplicate groups
//! - `remediation_actions`: Deduplication operations (method_used, freed_space, status)
//! - `audit_log`: All mutations (timestamp, action, affected_item)
//! - `settings`: Small persisted key-value settings (node_id, etc.)
//!
//! ## Features
//! - **Thread-safe**: All operations protected by Mutex
//! - **WAL mode**: Concurrent reads while writes complete
//! - **Batch operations**: Efficient bulk inserts for large file lists
//! - **Transaction support**: Atomic multi-table updates

use anyhow::{Context, Result};
use rusqlite::{params, Connection, OptionalExtension};
use std::path::Path;
use std::sync::Mutex;
use tracing::info;

use crate::config::DatabaseConfig;

/// Local SQLite database — sovereign data storage.
///
/// Thread-safe via Mutex. WAL mode for concurrent reads + writes.
/// All data persisted locally — no network calls, no telemetry.
pub struct Database {
    conn: Mutex<Connection>,
}

impl Database {
    /// Create or open a local SQLite database.
    pub fn new(config: &DatabaseConfig) -> Result<Self> {
        if let Some(parent) = Path::new(&config.path).parent() {
            if !parent.as_os_str().is_empty() {
                std::fs::create_dir_all(parent).ok();
            }
        }

        let conn = Connection::open(&config.path)
            .context("Failed to open SQLite database")?;

        if config.wal_mode {
            conn.execute_batch("PRAGMA journal_mode=WAL;")?;
        }

        conn.execute_batch(&format!(
            "PRAGMA cache_size=-{};
             PRAGMA synchronous=NORMAL;
             PRAGMA temp_store=MEMORY;
             PRAGMA mmap_size=268435456;",
            config.cache_size_mb * 1024
        ))?;

        let db = Self {
            conn: Mutex::new(conn),
        };
        db.initialize_schema()?;

        info!("Database initialized at: {}", config.path);
        Ok(db)
    }

    fn initialize_schema(&self) -> Result<()> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| anyhow::anyhow!("DB lock poisoned: {}", e))?;

        conn.execute_batch(
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
                id TEXT PRIMARY KEY,
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
                drive_id INTEGER,
                scan_id TEXT NOT NULL,
                file_path TEXT NOT NULL,
                file_name TEXT NOT NULL,
                size_bytes INTEGER NOT NULL,
                modified_time INTEGER NOT NULL,
                permissions TEXT,
                xxhash64 TEXT,
                sha256_hash TEXT,
                is_deleted BOOLEAN DEFAULT FALSE,
                remediation_status TEXT DEFAULT 'none',
                discovered_at INTEGER DEFAULT (strftime('%s', 'now')),
                FOREIGN KEY (node_id) REFERENCES nodes(id),
                FOREIGN KEY (scan_id) REFERENCES scans(id)
            );

            CREATE TABLE IF NOT EXISTS remediation_actions (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                group_id INTEGER,
                action_type TEXT NOT NULL,
                file_path TEXT NOT NULL,
                source_path TEXT,
                initiated_by_node TEXT NOT NULL,
                executed_at INTEGER DEFAULT (strftime('%s', 'now')),
                status TEXT DEFAULT 'pending',
                space_recovered INTEGER DEFAULT 0,
                fs_type TEXT,
                strategy TEXT,
                error_message TEXT
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

            CREATE TABLE IF NOT EXISTS settings (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL,
                updated_at INTEGER DEFAULT (strftime('%s', 'now'))
            );
            "
        )?;

        if !table_has_column(&conn, "files", "remediation_status")? {
            conn.execute(
                "ALTER TABLE files ADD COLUMN remediation_status TEXT DEFAULT 'none'",
                [],
            )?;
        }

        let needs_duplicate_schema_reset = !table_has_column(&conn, "duplicate_groups", "scan_id")?;
        if needs_duplicate_schema_reset {
            conn.execute_batch(
                "
                DROP TABLE IF EXISTS duplicate_files;
                DROP TABLE IF EXISTS duplicate_groups;
                ",
            )?;
        }

        conn.execute_batch(
            "
            CREATE TABLE IF NOT EXISTS duplicate_groups (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                scan_id TEXT NOT NULL,
                sha256_hash TEXT NOT NULL,
                size_bytes INTEGER NOT NULL,
                file_count INTEGER NOT NULL,
                total_wasted_bytes INTEGER NOT NULL,
                first_seen_at INTEGER DEFAULT (strftime('%s', 'now')),
                last_updated_at INTEGER DEFAULT (strftime('%s', 'now')),
                FOREIGN KEY (scan_id) REFERENCES scans(id),
                UNIQUE (scan_id, sha256_hash)
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

            CREATE UNIQUE INDEX IF NOT EXISTS idx_files_unique_per_scan_path
                ON files(node_id, scan_id, file_path);
            CREATE INDEX IF NOT EXISTS idx_files_sha256 ON files(sha256_hash);
            CREATE INDEX IF NOT EXISTS idx_files_xxhash ON files(xxhash64);
            CREATE INDEX IF NOT EXISTS idx_files_size ON files(size_bytes);
            CREATE INDEX IF NOT EXISTS idx_files_scan ON files(scan_id);
            CREATE INDEX IF NOT EXISTS idx_files_scan_status
                ON files(scan_id, remediation_status, is_deleted);
            CREATE INDEX IF NOT EXISTS idx_files_node_path ON files(node_id, file_path);
            CREATE INDEX IF NOT EXISTS idx_duplicate_groups_scan_wasted
                ON duplicate_groups(scan_id, total_wasted_bytes DESC);
            CREATE INDEX IF NOT EXISTS idx_duplicate_groups_scan_hash
                ON duplicate_groups(scan_id, sha256_hash);
            CREATE INDEX IF NOT EXISTS idx_duplicate_files_group ON duplicate_files(group_id);
            CREATE INDEX IF NOT EXISTS idx_audit_timestamp ON audit_log(timestamp DESC);
            CREATE INDEX IF NOT EXISTS idx_scans_completed_at ON scans(completed_at DESC, started_at DESC);
            "
        )?;

        info!("Database schema initialized — all tables and indexes ready");
        Ok(())
    }

    pub fn get_setting(&self, key: &str) -> Result<Option<String>> {
        let conn = self.conn.lock().map_err(|e| anyhow::anyhow!("DB lock: {}", e))?;
        let value = conn
            .query_row(
                "SELECT value FROM settings WHERE key = ?1",
                params![key],
                |row| row.get(0),
            )
            .optional()?;
        Ok(value)
    }

    pub fn set_setting(&self, key: &str, value: &str) -> Result<()> {
        let conn = self.conn.lock().map_err(|e| anyhow::anyhow!("DB lock: {}", e))?;
        conn.execute(
            "INSERT INTO settings (key, value, updated_at)
             VALUES (?1, ?2, strftime('%s','now'))
             ON CONFLICT(key) DO UPDATE SET value = excluded.value, updated_at = excluded.updated_at",
            params![key, value],
        )?;
        Ok(())
    }

    pub fn upsert_node(
        &self,
        id: &str,
        hostname: &str,
        ip: &str,
        platform: &str,
        version: &str,
        last_seen: i64,
    ) -> Result<()> {
        let conn = self.conn.lock().map_err(|e| anyhow::anyhow!("DB lock: {}", e))?;
        conn.execute(
            "INSERT INTO nodes (id, hostname, ip_address, platform, version, last_seen, status)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, 'online')
             ON CONFLICT(id) DO UPDATE SET
                hostname=?2, ip_address=?3, platform=?4, version=?5, last_seen=?6, status='online'",
            params![id, hostname, ip, platform, version, last_seen],
        )?;
        Ok(())
    }

    pub fn get_nodes(&self) -> Result<Vec<NodeRow>> {
        let conn = self.conn.lock().map_err(|e| anyhow::anyhow!("DB lock: {}", e))?;
        let mut stmt = conn.prepare(
            "SELECT id, hostname, ip_address, platform, version, last_seen, status FROM nodes",
        )?;
        let rows = stmt
            .query_map([], |row| {
                Ok(NodeRow {
                    id: row.get(0)?,
                    hostname: row.get(1)?,
                    ip_address: row.get(2)?,
                    platform: row.get(3)?,
                    version: row.get(4)?,
                    last_seen: row.get(5)?,
                    status: row.get(6)?,
                })
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        Ok(rows)
    }

    pub fn insert_drive(
        &self,
        node_id: &str,
        mount_point: &str,
        drive_type: &str,
        fs_type: &str,
        total: u64,
        available: u64,
        label: &str,
        removable: bool,
    ) -> Result<i64> {
        let conn = self.conn.lock().map_err(|e| anyhow::anyhow!("DB lock: {}", e))?;
        conn.execute(
            "INSERT INTO drives (node_id, mount_point, drive_type, filesystem_type, total_space, available_space, label, is_removable)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                node_id,
                mount_point,
                drive_type,
                fs_type,
                total as i64,
                available as i64,
                label,
                removable
            ],
        )?;
        Ok(conn.last_insert_rowid())
    }

    pub fn get_drives(&self, node_id: &str) -> Result<Vec<DriveRow>> {
        let conn = self.conn.lock().map_err(|e| anyhow::anyhow!("DB lock: {}", e))?;
        let mut stmt = conn.prepare(
            "SELECT id, mount_point, drive_type, filesystem_type, total_space, available_space, label, is_removable
             FROM drives WHERE node_id = ?1",
        )?;
        let rows = stmt
            .query_map(params![node_id], |row| {
                Ok(DriveRow {
                    id: row.get(0)?,
                    mount_point: row.get(1)?,
                    drive_type: row.get(2)?,
                    filesystem_type: row.get::<_, Option<String>>(3)?.unwrap_or_default(),
                    total_space: row.get(4)?,
                    available_space: row.get(5)?,
                    label: row.get::<_, Option<String>>(6)?.unwrap_or_default(),
                    is_removable: row.get(7)?,
                })
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        Ok(rows)
    }

    pub fn create_scan(&self, id: &str, node_id: &str, initiated_by: &str, started_at: i64) -> Result<()> {
        let conn = self.conn.lock().map_err(|e| anyhow::anyhow!("DB lock: {}", e))?;
        conn.execute(
            "INSERT INTO scans (id, node_id, initiated_by, started_at) VALUES (?1, ?2, ?3, ?4)",
            params![id, node_id, initiated_by, started_at],
        )?;
        Ok(())
    }

    pub fn complete_scan(&self, id: &str, completed_at: i64, files: i64, bytes: i64) -> Result<()> {
        self.finish_scan(id, "completed", completed_at, files, bytes, 0)
    }

    pub fn finish_scan(
        &self,
        id: &str,
        status: &str,
        completed_at: i64,
        files: i64,
        bytes: i64,
        error_count: i64,
    ) -> Result<()> {
        let conn = self.conn.lock().map_err(|e| anyhow::anyhow!("DB lock: {}", e))?;
        conn.execute(
            "UPDATE scans
             SET status=?2, completed_at=?3, files_scanned=?4, bytes_scanned=?5, error_count=?6
             WHERE id=?1",
            params![id, status, completed_at, files, bytes, error_count],
        )?;
        Ok(())
    }

    pub fn get_latest_completed_scan_id(&self) -> Result<Option<String>> {
        let conn = self.conn.lock().map_err(|e| anyhow::anyhow!("DB lock: {}", e))?;
        let scan_id = conn
            .query_row(
                "SELECT id
                 FROM scans
                 WHERE status IN ('completed', 'cancelled') OR completed_at IS NOT NULL
                 ORDER BY COALESCE(completed_at, started_at) DESC
                 LIMIT 1",
                [],
                |row| row.get(0),
            )
            .optional()?;
        Ok(scan_id)
    }

    pub fn insert_file(
        &self,
        node_id: &str,
        scan_id: &str,
        path: &str,
        name: &str,
        size: i64,
        modified: i64,
        xxhash: Option<&str>,
        sha256: Option<&str>,
    ) -> Result<i64> {
        let conn = self.conn.lock().map_err(|e| anyhow::anyhow!("DB lock: {}", e))?;
        conn.execute(
            "INSERT INTO files (node_id, scan_id, file_path, file_name, size_bytes, modified_time, xxhash64, sha256_hash, remediation_status)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, 'none')
             ON CONFLICT(node_id, scan_id, file_path) DO UPDATE SET
                file_name = excluded.file_name,
                size_bytes = excluded.size_bytes,
                modified_time = excluded.modified_time,
                xxhash64 = excluded.xxhash64,
                sha256_hash = excluded.sha256_hash,
                remediation_status = 'none',
                is_deleted = FALSE",
            params![node_id, scan_id, path, name, size, modified, xxhash, sha256],
        )?;
        let id = conn.query_row(
            "SELECT id FROM files WHERE node_id = ?1 AND scan_id = ?2 AND file_path = ?3",
            params![node_id, scan_id, path],
            |row| row.get(0),
        )?;
        Ok(id)
    }

    pub fn insert_files_batch(&self, files: &[FileRow]) -> Result<Vec<i64>> {
        let mut conn = self.conn.lock().map_err(|e| anyhow::anyhow!("DB lock: {}", e))?;
        let tx = conn.transaction()?;
        let mut ids = Vec::with_capacity(files.len());

        for file in files {
            tx.execute(
                "INSERT INTO files (node_id, scan_id, file_path, file_name, size_bytes, modified_time, xxhash64, sha256_hash, remediation_status)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, 'none')
                 ON CONFLICT(node_id, scan_id, file_path) DO UPDATE SET
                    file_name = excluded.file_name,
                    size_bytes = excluded.size_bytes,
                    modified_time = excluded.modified_time,
                    xxhash64 = excluded.xxhash64,
                    sha256_hash = excluded.sha256_hash,
                    remediation_status = 'none',
                    is_deleted = FALSE",
                params![
                    file.node_id,
                    file.scan_id,
                    file.file_path,
                    file.file_name,
                    file.size_bytes,
                    file.modified_time,
                    file.xxhash64,
                    file.sha256_hash,
                ],
            )?;

            let id = tx.query_row(
                "SELECT id FROM files WHERE node_id = ?1 AND scan_id = ?2 AND file_path = ?3",
                params![file.node_id, file.scan_id, file.file_path],
                |row| row.get(0),
            )?;
            ids.push(id);
        }

        tx.commit()?;
        Ok(ids)
    }

    pub fn get_file_by_id(&self, file_id: i64) -> Result<Option<FileRow>> {
        let conn = self.conn.lock().map_err(|e| anyhow::anyhow!("DB lock: {}", e))?;
        let file = conn
            .query_row(
                "SELECT id, node_id, scan_id, file_path, file_name, size_bytes, modified_time, xxhash64, sha256_hash, remediation_status
                 FROM files WHERE id = ?1",
                params![file_id],
                |row| {
                    Ok(FileRow {
                        id: Some(row.get(0)?),
                        node_id: row.get(1)?,
                        scan_id: row.get(2)?,
                        file_path: row.get(3)?,
                        file_name: row.get(4)?,
                        size_bytes: row.get(5)?,
                        modified_time: row.get(6)?,
                        xxhash64: row.get(7)?,
                        sha256_hash: row.get(8)?,
                        remediation_status: row.get::<_, Option<String>>(9)?.unwrap_or_else(|| "none".to_string()),
                    })
                },
            )
            .optional()?;
        Ok(file)
    }

    pub fn find_size_matches(&self, min_count: i64) -> Result<Vec<(i64, i64)>> {
        let conn = self.conn.lock().map_err(|e| anyhow::anyhow!("DB lock: {}", e))?;
        let mut stmt = conn.prepare(
            "SELECT size_bytes, COUNT(*) as cnt
             FROM files
             WHERE is_deleted = FALSE AND size_bytes > 0 AND remediation_status = 'none'
             GROUP BY size_bytes HAVING cnt >= ?1
             ORDER BY size_bytes DESC",
        )?;
        let rows = stmt
            .query_map(params![min_count], |row| {
                Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?))
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        Ok(rows)
    }

    pub fn find_xxhash_matches(&self, min_count: i64) -> Result<Vec<(String, i64)>> {
        let conn = self.conn.lock().map_err(|e| anyhow::anyhow!("DB lock: {}", e))?;
        let mut stmt = conn.prepare(
            "SELECT xxhash64, COUNT(*) as cnt
             FROM files
             WHERE is_deleted = FALSE AND remediation_status = 'none' AND xxhash64 IS NOT NULL
             GROUP BY xxhash64 HAVING cnt >= ?1",
        )?;
        let rows = stmt
            .query_map(params![min_count], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        Ok(rows)
    }

    pub fn find_sha256_matches(&self, min_count: i64) -> Result<Vec<(String, i64, i64)>> {
        let conn = self.conn.lock().map_err(|e| anyhow::anyhow!("DB lock: {}", e))?;
        let mut stmt = conn.prepare(
            "SELECT sha256_hash, MIN(size_bytes) as size_bytes, COUNT(DISTINCT file_path) as cnt
             FROM files
             WHERE is_deleted = FALSE AND remediation_status = 'none' AND sha256_hash IS NOT NULL
             GROUP BY sha256_hash HAVING cnt >= ?1
             ORDER BY (size_bytes * (cnt - 1)) DESC",
        )?;
        let rows = stmt
            .query_map(params![min_count], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, i64>(1)?,
                    row.get::<_, i64>(2)?,
                ))
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        Ok(rows)
    }

    pub fn find_sha256_matches_for_scan(&self, scan_id: &str, min_count: i64) -> Result<Vec<(String, i64, i64)>> {
        let conn = self.conn.lock().map_err(|e| anyhow::anyhow!("DB lock: {}", e))?;
        let mut stmt = conn.prepare(
            "SELECT sha256_hash, MIN(size_bytes) as size_bytes, COUNT(DISTINCT file_path) as cnt
             FROM files
             WHERE scan_id = ?1
               AND is_deleted = FALSE
               AND remediation_status = 'none'
               AND sha256_hash IS NOT NULL
             GROUP BY sha256_hash
             HAVING cnt >= ?2
             ORDER BY (size_bytes * (cnt - 1)) DESC, sha256_hash ASC",
        )?;
        let rows = stmt
            .query_map(params![scan_id, min_count], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, i64>(1)?,
                    row.get::<_, i64>(2)?,
                ))
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        Ok(rows)
    }

    pub fn get_files_by_hash(&self, sha256: &str) -> Result<Vec<FileRow>> {
        let conn = self.conn.lock().map_err(|e| anyhow::anyhow!("DB lock: {}", e))?;
        let mut stmt = conn.prepare(
            "SELECT id, node_id, scan_id, file_path, file_name, size_bytes, modified_time, xxhash64, sha256_hash, remediation_status
             FROM files
             WHERE sha256_hash = ?1 AND is_deleted = FALSE AND remediation_status = 'none'
             ORDER BY file_path ASC",
        )?;
        let rows = stmt
            .query_map(params![sha256], |row| {
                Ok(FileRow {
                    id: Some(row.get(0)?),
                    node_id: row.get(1)?,
                    scan_id: row.get(2)?,
                    file_path: row.get(3)?,
                    file_name: row.get(4)?,
                    size_bytes: row.get(5)?,
                    modified_time: row.get(6)?,
                    xxhash64: row.get(7)?,
                    sha256_hash: row.get(8)?,
                    remediation_status: row.get::<_, Option<String>>(9)?.unwrap_or_else(|| "none".to_string()),
                })
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        Ok(rows)
    }

    pub fn get_files_by_hash_for_scan(&self, scan_id: &str, sha256: &str) -> Result<Vec<FileRow>> {
        let conn = self.conn.lock().map_err(|e| anyhow::anyhow!("DB lock: {}", e))?;
        let mut stmt = conn.prepare(
            "SELECT id, node_id, scan_id, file_path, file_name, size_bytes, modified_time, xxhash64, sha256_hash, remediation_status
             FROM files
             WHERE scan_id = ?1
               AND sha256_hash = ?2
               AND is_deleted = FALSE
               AND remediation_status = 'none'
             ORDER BY file_path ASC",
        )?;
        let rows = stmt
            .query_map(params![scan_id, sha256], |row| {
                Ok(FileRow {
                    id: Some(row.get(0)?),
                    node_id: row.get(1)?,
                    scan_id: row.get(2)?,
                    file_path: row.get(3)?,
                    file_name: row.get(4)?,
                    size_bytes: row.get(5)?,
                    modified_time: row.get(6)?,
                    xxhash64: row.get(7)?,
                    sha256_hash: row.get(8)?,
                    remediation_status: row.get::<_, Option<String>>(9)?.unwrap_or_else(|| "none".to_string()),
                })
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        Ok(rows)
    }

    pub fn set_file_remediation_status(&self, file_id: i64, status: &str, is_deleted: bool) -> Result<()> {
        let conn = self.conn.lock().map_err(|e| anyhow::anyhow!("DB lock: {}", e))?;
        conn.execute(
            "UPDATE files SET remediation_status = ?2, is_deleted = ?3 WHERE id = ?1",
            params![file_id, status, is_deleted],
        )?;
        Ok(())
    }

    pub fn mark_file_deleted(&self, file_id: i64) -> Result<()> {
        self.set_file_remediation_status(file_id, "deleted", true)
    }

    pub fn clear_duplicate_results_for_scan(&self, scan_id: &str) -> Result<()> {
        let mut conn = self.conn.lock().map_err(|e| anyhow::anyhow!("DB lock: {}", e))?;
        let tx = conn.transaction()?;
        tx.execute(
            "DELETE FROM duplicate_files WHERE group_id IN (SELECT id FROM duplicate_groups WHERE scan_id = ?1)",
            params![scan_id],
        )?;
        tx.execute("DELETE FROM duplicate_groups WHERE scan_id = ?1", params![scan_id])?;
        tx.commit()?;
        Ok(())
    }

    pub fn upsert_duplicate_group(&self, scan_id: &str, sha256: &str, size: i64, count: i64) -> Result<i64> {
        let wasted = size * (count - 1);
        let conn = self.conn.lock().map_err(|e| anyhow::anyhow!("DB lock: {}", e))?;
        conn.execute(
            "INSERT INTO duplicate_groups (scan_id, sha256_hash, size_bytes, file_count, total_wasted_bytes)
             VALUES (?1, ?2, ?3, ?4, ?5)
             ON CONFLICT(scan_id, sha256_hash) DO UPDATE SET
                size_bytes = excluded.size_bytes,
                file_count = excluded.file_count,
                total_wasted_bytes = excluded.total_wasted_bytes,
                last_updated_at = strftime('%s','now')",
            params![scan_id, sha256, size, count, wasted],
        )?;
        let group_id = conn.query_row(
            "SELECT id FROM duplicate_groups WHERE scan_id = ?1 AND sha256_hash = ?2",
            params![scan_id, sha256],
            |row| row.get(0),
        )?;
        Ok(group_id)
    }

    pub fn replace_duplicate_group_files(
        &self,
        group_id: i64,
        file_ids: &[i64],
        primary_file_id: i64,
    ) -> Result<()> {
        let mut conn = self.conn.lock().map_err(|e| anyhow::anyhow!("DB lock: {}", e))?;
        let tx = conn.transaction()?;
        tx.execute("DELETE FROM duplicate_files WHERE group_id = ?1", params![group_id])?;

        for file_id in file_ids {
            tx.execute(
                "INSERT INTO duplicate_files (group_id, file_id, is_primary, remediation_status)
                 VALUES (?1, ?2, ?3, 'none')",
                params![group_id, file_id, *file_id == primary_file_id],
            )?;
        }

        tx.commit()?;
        Ok(())
    }

    pub fn get_duplicate_groups(&self) -> Result<Vec<DuplicateGroupRow>> {
        let conn = self.conn.lock().map_err(|e| anyhow::anyhow!("DB lock: {}", e))?;
        let mut stmt = conn.prepare(
            "SELECT id, scan_id, sha256_hash, size_bytes, file_count, total_wasted_bytes
             FROM duplicate_groups
             ORDER BY total_wasted_bytes DESC, id ASC",
        )?;
        let rows = stmt
            .query_map([], |row| {
                Ok(DuplicateGroupRow {
                    id: row.get(0)?,
                    scan_id: row.get(1)?,
                    sha256_hash: row.get(2)?,
                    size_bytes: row.get(3)?,
                    file_count: row.get(4)?,
                    total_wasted_bytes: row.get(5)?,
                })
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        Ok(rows)
    }

    pub fn get_duplicate_groups_for_scan(&self, scan_id: &str) -> Result<Vec<DuplicateGroupRow>> {
        let conn = self.conn.lock().map_err(|e| anyhow::anyhow!("DB lock: {}", e))?;
        let mut stmt = conn.prepare(
            "SELECT id, scan_id, sha256_hash, size_bytes, file_count, total_wasted_bytes
             FROM duplicate_groups
             WHERE scan_id = ?1
             ORDER BY total_wasted_bytes DESC, id ASC",
        )?;
        let rows = stmt
            .query_map(params![scan_id], |row| {
                Ok(DuplicateGroupRow {
                    id: row.get(0)?,
                    scan_id: row.get(1)?,
                    sha256_hash: row.get(2)?,
                    size_bytes: row.get(3)?,
                    file_count: row.get(4)?,
                    total_wasted_bytes: row.get(5)?,
                })
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        Ok(rows)
    }

    pub fn get_duplicate_files_for_group(&self, group_id: i64) -> Result<Vec<FileRow>> {
        let conn = self.conn.lock().map_err(|e| anyhow::anyhow!("DB lock: {}", e))?;
        let mut stmt = conn.prepare(
            "SELECT f.id, f.node_id, f.scan_id, f.file_path, f.file_name, f.size_bytes, f.modified_time, f.xxhash64, f.sha256_hash, f.remediation_status
             FROM duplicate_files df
             JOIN files f ON f.id = df.file_id
             WHERE df.group_id = ?1
               AND f.is_deleted = FALSE
               AND f.remediation_status = 'none'
             ORDER BY df.is_primary DESC, f.file_path ASC",
        )?;
        let rows = stmt
            .query_map(params![group_id], |row| {
                Ok(FileRow {
                    id: Some(row.get(0)?),
                    node_id: row.get(1)?,
                    scan_id: row.get(2)?,
                    file_path: row.get(3)?,
                    file_name: row.get(4)?,
                    size_bytes: row.get(5)?,
                    modified_time: row.get(6)?,
                    xxhash64: row.get(7)?,
                    sha256_hash: row.get(8)?,
                    remediation_status: row.get::<_, Option<String>>(9)?.unwrap_or_else(|| "none".to_string()),
                })
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        Ok(rows)
    }

    pub fn get_total_wasted_space(&self) -> Result<i64> {
        let conn = self.conn.lock().map_err(|e| anyhow::anyhow!("DB lock: {}", e))?;
        let total: i64 = conn.query_row(
            "SELECT COALESCE(SUM(total_wasted_bytes), 0) FROM duplicate_groups",
            [],
            |row| row.get(0),
        )?;
        Ok(total)
    }

    pub fn get_total_wasted_space_for_scan(&self, scan_id: &str) -> Result<i64> {
        let conn = self.conn.lock().map_err(|e| anyhow::anyhow!("DB lock: {}", e))?;
        let total: i64 = conn.query_row(
            "SELECT COALESCE(SUM(total_wasted_bytes), 0) FROM duplicate_groups WHERE scan_id = ?1",
            params![scan_id],
            |row| row.get(0),
        )?;
        Ok(total)
    }

    pub fn log_remediation(
        &self,
        group_id: Option<i64>,
        action: &str,
        file_path: &str,
        source_path: Option<&str>,
        node_id: &str,
        space_recovered: i64,
        fs_type: &str,
        strategy: &str,
    ) -> Result<()> {
        let conn = self.conn.lock().map_err(|e| anyhow::anyhow!("DB lock: {}", e))?;
        conn.execute(
            "INSERT INTO remediation_actions (group_id, action_type, file_path, source_path, initiated_by_node, status, space_recovered, fs_type, strategy)
             VALUES (?1, ?2, ?3, ?4, ?5, 'completed', ?6, ?7, ?8)",
            params![group_id, action, file_path, source_path, node_id, space_recovered, fs_type, strategy],
        )?;
        Ok(())
    }

    pub fn log_audit(&self, action: &str, resource_type: &str, resource_id: &str, details: &str, node_id: &str) -> Result<()> {
        let conn = self.conn.lock().map_err(|e| anyhow::anyhow!("DB lock: {}", e))?;
        conn.execute(
            "INSERT INTO audit_log (action, resource_type, resource_id, details, node_id) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![action, resource_type, resource_id, details, node_id],
        )?;
        Ok(())
    }

    pub fn get_stats(&self) -> Result<DbStats> {
        let conn = self.conn.lock().map_err(|e| anyhow::anyhow!("DB lock: {}", e))?;
        let total_files: i64 = conn.query_row(
            "SELECT COUNT(*) FROM files WHERE is_deleted = FALSE AND remediation_status = 'none'",
            [],
            |row| row.get(0),
        )?;
        let total_scans: i64 = conn.query_row("SELECT COUNT(*) FROM scans", [], |row| row.get(0))?;
        let total_groups: i64 = conn.query_row("SELECT COUNT(*) FROM duplicate_groups", [], |row| row.get(0))?;
        let total_wasted: i64 = conn.query_row(
            "SELECT COALESCE(SUM(total_wasted_bytes), 0) FROM duplicate_groups",
            [],
            |row| row.get(0),
        )?;
        Ok(DbStats {
            total_files,
            total_scans,
            total_groups,
            total_wasted,
        })
    }
}

fn table_has_column(conn: &Connection, table: &str, column: &str) -> Result<bool> {
    let sql = format!("PRAGMA table_info({})", table);
    let mut stmt = conn.prepare(&sql)?;
    let mut rows = stmt.query([])?;
    while let Some(row) = rows.next()? {
        let name: String = row.get(1)?;
        if name == column {
            return Ok(true);
        }
    }
    Ok(false)
}

#[derive(Debug, Clone)]
pub struct NodeRow {
    pub id: String,
    pub hostname: String,
    pub ip_address: String,
    pub platform: String,
    pub version: String,
    pub last_seen: i64,
    pub status: String,
}

#[derive(Debug, Clone)]
pub struct DriveRow {
    pub id: i64,
    pub mount_point: String,
    pub drive_type: String,
    pub filesystem_type: String,
    pub total_space: i64,
    pub available_space: i64,
    pub label: String,
    pub is_removable: bool,
}

#[derive(Debug, Clone)]
pub struct FileRow {
    pub id: Option<i64>,
    pub node_id: String,
    pub scan_id: String,
    pub file_path: String,
    pub file_name: String,
    pub size_bytes: i64,
    pub modified_time: i64,
    pub xxhash64: Option<String>,
    pub sha256_hash: Option<String>,
    pub remediation_status: String,
}

#[derive(Debug, Clone)]
pub struct DuplicateGroupRow {
    pub id: i64,
    pub scan_id: String,
    pub sha256_hash: String,
    pub size_bytes: i64,
    pub file_count: i64,
    pub total_wasted_bytes: i64,
}

#[derive(Debug, Clone)]
pub struct DbStats {
    pub total_files: i64,
    pub total_scans: i64,
    pub total_groups: i64,
    pub total_wasted: i64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn temp_db() -> Result<Database> {
        let dir = tempdir()?;
        let config = DatabaseConfig {
            path: dir.path().join("test.db").display().to_string(),
            cache_size_mb: 16,
            wal_mode: true,
        };
        let db = Database::new(&config)?;
        db.upsert_node("node-1", "test-host", "127.0.0.1", "test", "0.5.1", 1)?;
        Ok(db)
    }

    fn file_row(node_id: &str, scan_id: &str, path: &str, sha256: &str) -> FileRow {
        FileRow {
            id: None,
            node_id: node_id.to_string(),
            scan_id: scan_id.to_string(),
            file_path: path.to_string(),
            file_name: Path::new(path)
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string(),
            size_bytes: 1024,
            modified_time: 1,
            xxhash64: Some("xxhash".to_string()),
            sha256_hash: Some(sha256.to_string()),
            remediation_status: "none".to_string(),
        }
    }

    #[test]
    fn duplicate_matches_are_scan_scoped() -> Result<()> {
        let db = temp_db()?;

        db.create_scan("scan-a", "node-1", "user", 1)?;
        db.create_scan("scan-b", "node-1", "user", 2)?;

        db.insert_files_batch(&[
            file_row("node-1", "scan-a", "/data/a.bin", "hash-1"),
            file_row("node-1", "scan-a", "/data/b.bin", "hash-1"),
            file_row("node-1", "scan-b", "/data/c.bin", "hash-1"),
        ])?;

        let scan_a = db.find_sha256_matches_for_scan("scan-a", 2)?;
        let scan_b = db.find_sha256_matches_for_scan("scan-b", 2)?;

        assert_eq!(scan_a.len(), 1);
        assert_eq!(scan_a[0].0, "hash-1");
        assert_eq!(scan_a[0].2, 2);
        assert!(scan_b.is_empty());
        Ok(())
    }

    #[test]
    fn duplicate_file_rows_do_not_self_duplicate_with_same_path() -> Result<()> {
        let db = temp_db()?;
        db.create_scan("scan-a", "node-1", "user", 1)?;

        db.insert_files_batch(&[
            file_row("node-1", "scan-a", "/data/same.bin", "hash-1"),
            file_row("node-1", "scan-a", "/data/same.bin", "hash-1"),
        ])?;

        let matches = db.find_sha256_matches_for_scan("scan-a", 2)?;
        assert!(matches.is_empty());

        let files = db.get_files_by_hash_for_scan("scan-a", "hash-1")?;
        assert_eq!(files.len(), 1);
        Ok(())
    }

    #[test]
    fn duplicate_membership_returns_only_requested_scan_group() -> Result<()> {
        let db = temp_db()?;
        db.create_scan("scan-a", "node-1", "user", 1)?;
        db.create_scan("scan-b", "node-1", "user", 2)?;

        let ids = db.insert_files_batch(&[
            file_row("node-1", "scan-a", "/data/a.bin", "hash-1"),
            file_row("node-1", "scan-a", "/data/b.bin", "hash-1"),
            file_row("node-1", "scan-b", "/data/c.bin", "hash-1"),
            file_row("node-1", "scan-b", "/data/d.bin", "hash-1"),
        ])?;

        let group_a = db.upsert_duplicate_group("scan-a", "hash-1", 1024, 2)?;
        db.replace_duplicate_group_files(group_a, &ids[0..2], ids[0])?;

        let group_b = db.upsert_duplicate_group("scan-b", "hash-1", 1024, 2)?;
        db.replace_duplicate_group_files(group_b, &ids[2..4], ids[2])?;

        let files_a = db.get_duplicate_files_for_group(group_a)?;
        let files_b = db.get_duplicate_files_for_group(group_b)?;

        assert_eq!(files_a.len(), 2);
        assert_eq!(files_b.len(), 2);
        assert!(files_a.iter().all(|f| f.scan_id == "scan-a"));
        assert!(files_b.iter().all(|f| f.scan_id == "scan-b"));
        Ok(())
    }
}
