// Gillsystems_uneff_your_rigs_messy_files — Database Module
// Philosophy: Radical Transparency — every hash, every path, every byte visible to you.
//
// Local SQLite database with WAL mode for maximum write performance.
// No cloud. No phone-home. Your data stays on your machine.

use anyhow::{Context, Result};
use rusqlite::{Connection, params};
use std::path::Path;
use std::sync::Mutex;
use tracing::{info, warn, error};

use crate::config::DatabaseConfig;

/// Local SQLite database — sovereign data storage.
/// Thread-safe via Mutex. WAL mode for concurrent reads.
pub struct Database {
    conn: Mutex<Connection>,
}

impl Database {
    /// Open or create the local database.
    /// WAL mode enabled by default for concurrent read/write performance.
    pub fn new(config: &DatabaseConfig) -> Result<Self> {
        // Ensure parent directory exists
        if let Some(parent) = Path::new(&config.path).parent() {
            if !parent.as_os_str().is_empty() {
                std::fs::create_dir_all(parent).ok();
            }
        }

        let conn = Connection::open(&config.path)
            .context("Failed to open SQLite database")?;

        // Enable WAL mode for performance — no brakes
        if config.wal_mode {
            conn.execute_batch("PRAGMA journal_mode=WAL;")?;
        }

        // Performance pragmas
        conn.execute_batch(&format!(
            "PRAGMA cache_size=-{};
             PRAGMA synchronous=NORMAL;
             PRAGMA temp_store=MEMORY;
             PRAGMA mmap_size=268435456;",
            config.cache_size_mb * 1024
        ))?;

        let db = Self { conn: Mutex::new(conn) };
        db.initialize_schema()?;

        info!("Database initialized at: {}", config.path);
        Ok(db)
    }

    /// Create all tables and indexes if they don't exist.
    fn initialize_schema(&self) -> Result<()> {
        let conn = self.conn.lock().map_err(|e| anyhow::anyhow!("DB lock poisoned: {}", e))?;
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
                discovered_at INTEGER DEFAULT (strftime('%s', 'now')),
                FOREIGN KEY (node_id) REFERENCES nodes(id),
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

            -- Performance indexes
            CREATE INDEX IF NOT EXISTS idx_files_sha256 ON files(sha256_hash);
            CREATE INDEX IF NOT EXISTS idx_files_xxhash ON files(xxhash64);
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

    // ── Node CRUD ──────────────────────────────────────────────────────

    /// Insert or update a node record.
    pub fn upsert_node(
        &self, id: &str, hostname: &str, ip: &str, platform: &str, version: &str, last_seen: i64,
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

    /// Get all known nodes.
    pub fn get_nodes(&self) -> Result<Vec<NodeRow>> {
        let conn = self.conn.lock().map_err(|e| anyhow::anyhow!("DB lock: {}", e))?;
        let mut stmt = conn.prepare(
            "SELECT id, hostname, ip_address, platform, version, last_seen, status FROM nodes"
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(NodeRow {
                id: row.get(0)?,
                hostname: row.get(1)?,
                ip_address: row.get(2)?,
                platform: row.get(3)?,
                version: row.get(4)?,
                last_seen: row.get(5)?,
                status: row.get(6)?,
            })
        })?.collect::<Result<Vec<_>, _>>()?;
        Ok(rows)
    }

    // ── Drive CRUD ─────────────────────────────────────────────────────

    /// Insert a drive record.
    pub fn insert_drive(
        &self, node_id: &str, mount_point: &str, drive_type: &str,
        fs_type: &str, total: u64, available: u64, label: &str, removable: bool,
    ) -> Result<i64> {
        let conn = self.conn.lock().map_err(|e| anyhow::anyhow!("DB lock: {}", e))?;
        conn.execute(
            "INSERT INTO drives (node_id, mount_point, drive_type, filesystem_type, total_space, available_space, label, is_removable)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![node_id, mount_point, drive_type, fs_type, total as i64, available as i64, label, removable],
        )?;
        Ok(conn.last_insert_rowid())
    }

    /// Get drives for a node.
    pub fn get_drives(&self, node_id: &str) -> Result<Vec<DriveRow>> {
        let conn = self.conn.lock().map_err(|e| anyhow::anyhow!("DB lock: {}", e))?;
        let mut stmt = conn.prepare(
            "SELECT id, mount_point, drive_type, filesystem_type, total_space, available_space, label, is_removable
             FROM drives WHERE node_id = ?1"
        )?;
        let rows = stmt.query_map(params![node_id], |row| {
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
        })?.collect::<Result<Vec<_>, _>>()?;
        Ok(rows)
    }

    // ── Scan CRUD ──────────────────────────────────────────────────────

    /// Create a new scan record.
    pub fn create_scan(&self, id: &str, node_id: &str, initiated_by: &str, started_at: i64) -> Result<()> {
        let conn = self.conn.lock().map_err(|e| anyhow::anyhow!("DB lock: {}", e))?;
        conn.execute(
            "INSERT INTO scans (id, node_id, initiated_by, started_at) VALUES (?1, ?2, ?3, ?4)",
            params![id, node_id, initiated_by, started_at],
        )?;
        Ok(())
    }

    /// Mark a scan as completed.
    pub fn complete_scan(&self, id: &str, completed_at: i64, files: i64, bytes: i64) -> Result<()> {
        let conn = self.conn.lock().map_err(|e| anyhow::anyhow!("DB lock: {}", e))?;
        conn.execute(
            "UPDATE scans SET status='completed', completed_at=?2, files_scanned=?3, bytes_scanned=?4 WHERE id=?1",
            params![id, completed_at, files, bytes],
        )?;
        Ok(())
    }

    // ── File CRUD ──────────────────────────────────────────────────────

    /// Insert a scanned file record. Returns the file row ID.
    pub fn insert_file(
        &self, node_id: &str, scan_id: &str, path: &str, name: &str,
        size: i64, modified: i64, xxhash: Option<&str>, sha256: Option<&str>,
    ) -> Result<i64> {
        let conn = self.conn.lock().map_err(|e| anyhow::anyhow!("DB lock: {}", e))?;
        conn.execute(
            "INSERT INTO files (node_id, scan_id, file_path, file_name, size_bytes, modified_time, xxhash64, sha256_hash)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![node_id, scan_id, path, name, size, modified, xxhash, sha256],
        )?;
        Ok(conn.last_insert_rowid())
    }

    /// Batch insert files (transactional for speed).
    pub fn insert_files_batch(&self, files: &[FileRow]) -> Result<Vec<i64>> {
        let conn = self.conn.lock().map_err(|e| anyhow::anyhow!("DB lock: {}", e))?;
        let tx = conn.unchecked_transaction()?;
        let mut ids = Vec::with_capacity(files.len());

        {
            let mut stmt = tx.prepare_cached(
                "INSERT INTO files (node_id, scan_id, file_path, file_name, size_bytes, modified_time, xxhash64, sha256_hash)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)"
            )?;

            for f in files {
                stmt.execute(params![
                    f.node_id, f.scan_id, f.file_path, f.file_name,
                    f.size_bytes, f.modified_time, f.xxhash64, f.sha256_hash,
                ])?;
                ids.push(conn.last_insert_rowid());
            }
        }

        tx.commit()?;
        Ok(ids)
    }

    /// Find files with the same size (first stage of duplicate detection).
    pub fn find_size_matches(&self, min_count: i64) -> Result<Vec<(i64, i64)>> {
        let conn = self.conn.lock().map_err(|e| anyhow::anyhow!("DB lock: {}", e))?;
        let mut stmt = conn.prepare(
            "SELECT size_bytes, COUNT(*) as cnt FROM files
             WHERE is_deleted = FALSE AND size_bytes > 0
             GROUP BY size_bytes HAVING cnt >= ?1
             ORDER BY size_bytes DESC"
        )?;
        let rows = stmt.query_map(params![min_count], |row| {
            Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?))
        })?.collect::<Result<Vec<_>, _>>()?;
        Ok(rows)
    }

    /// Find files with the same xxhash64 (second stage).
    pub fn find_xxhash_matches(&self, min_count: i64) -> Result<Vec<(String, i64)>> {
        let conn = self.conn.lock().map_err(|e| anyhow::anyhow!("DB lock: {}", e))?;
        let mut stmt = conn.prepare(
            "SELECT xxhash64, COUNT(*) as cnt FROM files
             WHERE is_deleted = FALSE AND xxhash64 IS NOT NULL
             GROUP BY xxhash64 HAVING cnt >= ?1"
        )?;
        let rows = stmt.query_map(params![min_count], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))
        })?.collect::<Result<Vec<_>, _>>()?;
        Ok(rows)
    }

    /// Find files with the same SHA-256 (final confirmation).
    pub fn find_sha256_matches(&self, min_count: i64) -> Result<Vec<(String, i64, i64)>> {
        let conn = self.conn.lock().map_err(|e| anyhow::anyhow!("DB lock: {}", e))?;
        let mut stmt = conn.prepare(
            "SELECT sha256_hash, size_bytes, COUNT(*) as cnt FROM files
             WHERE is_deleted = FALSE AND sha256_hash IS NOT NULL
             GROUP BY sha256_hash HAVING cnt >= ?1
             ORDER BY (size_bytes * (cnt - 1)) DESC"
        )?;
        let rows = stmt.query_map(params![min_count], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?, row.get::<_, i64>(2)?))
        })?.collect::<Result<Vec<_>, _>>()?;
        Ok(rows)
    }

    /// Get all files for a given SHA-256 hash.
    pub fn get_files_by_hash(&self, sha256: &str) -> Result<Vec<FileRow>> {
        let conn = self.conn.lock().map_err(|e| anyhow::anyhow!("DB lock: {}", e))?;
        let mut stmt = conn.prepare(
            "SELECT id, node_id, scan_id, file_path, file_name, size_bytes, modified_time, xxhash64, sha256_hash
             FROM files WHERE sha256_hash = ?1 AND is_deleted = FALSE"
        )?;
        let rows = stmt.query_map(params![sha256], |row| {
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
            })
        })?.collect::<Result<Vec<_>, _>>()?;
        Ok(rows)
    }

    // ── Duplicate Group CRUD ───────────────────────────────────────────

    /// Create or update a duplicate group.
    pub fn upsert_duplicate_group(&self, sha256: &str, size: i64, count: i64) -> Result<i64> {
        let wasted = size * (count - 1);
        let conn = self.conn.lock().map_err(|e| anyhow::anyhow!("DB lock: {}", e))?;
        conn.execute(
            "INSERT INTO duplicate_groups (sha256_hash, size_bytes, file_count, total_wasted_bytes)
             VALUES (?1, ?2, ?3, ?4)
             ON CONFLICT(sha256_hash) DO UPDATE SET
                file_count=?3, total_wasted_bytes=?4, last_updated_at=strftime('%s','now')",
            params![sha256, size, count, wasted],
        )?;
        Ok(conn.last_insert_rowid())
    }

    /// Get all duplicate groups, sorted by wasted space descending.
    pub fn get_duplicate_groups(&self) -> Result<Vec<DuplicateGroupRow>> {
        let conn = self.conn.lock().map_err(|e| anyhow::anyhow!("DB lock: {}", e))?;
        let mut stmt = conn.prepare(
            "SELECT id, sha256_hash, size_bytes, file_count, total_wasted_bytes
             FROM duplicate_groups ORDER BY total_wasted_bytes DESC"
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(DuplicateGroupRow {
                id: row.get(0)?,
                sha256_hash: row.get(1)?,
                size_bytes: row.get(2)?,
                file_count: row.get(3)?,
                total_wasted_bytes: row.get(4)?,
            })
        })?.collect::<Result<Vec<_>, _>>()?;
        Ok(rows)
    }

    /// Get total wasted space across all duplicate groups.
    pub fn get_total_wasted_space(&self) -> Result<i64> {
        let conn = self.conn.lock().map_err(|e| anyhow::anyhow!("DB lock: {}", e))?;
        let total: i64 = conn.query_row(
            "SELECT COALESCE(SUM(total_wasted_bytes), 0) FROM duplicate_groups",
            [],
            |row| row.get(0),
        )?;
        Ok(total)
    }

    // ── Remediation Logging ────────────────────────────────────────────

    /// Log a remediation action to the audit trail.
    pub fn log_remediation(
        &self, group_id: Option<i64>, action: &str, file_path: &str,
        source_path: Option<&str>, node_id: &str, space_recovered: i64,
        fs_type: &str, strategy: &str,
    ) -> Result<()> {
        let conn = self.conn.lock().map_err(|e| anyhow::anyhow!("DB lock: {}", e))?;
        conn.execute(
            "INSERT INTO remediation_actions (group_id, action_type, file_path, source_path, initiated_by_node, status, space_recovered, fs_type, strategy)
             VALUES (?1, ?2, ?3, ?4, ?5, 'completed', ?6, ?7, ?8)",
            params![group_id, action, file_path, source_path, node_id, space_recovered, fs_type, strategy],
        )?;
        Ok(())
    }

    /// Log an audit event.
    pub fn log_audit(&self, action: &str, resource_type: &str, resource_id: &str, details: &str, node_id: &str) -> Result<()> {
        let conn = self.conn.lock().map_err(|e| anyhow::anyhow!("DB lock: {}", e))?;
        conn.execute(
            "INSERT INTO audit_log (action, resource_type, resource_id, details, node_id) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![action, resource_type, resource_id, details, node_id],
        )?;
        Ok(())
    }

    /// Mark a file as deleted in the database.
    pub fn mark_file_deleted(&self, file_id: i64) -> Result<()> {
        let conn = self.conn.lock().map_err(|e| anyhow::anyhow!("DB lock: {}", e))?;
        conn.execute("UPDATE files SET is_deleted = TRUE WHERE id = ?1", params![file_id])?;
        Ok(())
    }

    /// Get database statistics for status display.
    pub fn get_stats(&self) -> Result<DbStats> {
        let conn = self.conn.lock().map_err(|e| anyhow::anyhow!("DB lock: {}", e))?;
        let total_files: i64 = conn.query_row(
            "SELECT COUNT(*) FROM files WHERE is_deleted = FALSE", [], |r| r.get(0))?;
        let total_scans: i64 = conn.query_row(
            "SELECT COUNT(*) FROM scans", [], |r| r.get(0))?;
        let total_groups: i64 = conn.query_row(
            "SELECT COUNT(*) FROM duplicate_groups", [], |r| r.get(0))?;
        let total_wasted: i64 = conn.query_row(
            "SELECT COALESCE(SUM(total_wasted_bytes), 0) FROM duplicate_groups", [], |r| r.get(0))?;
        Ok(DbStats { total_files, total_scans, total_groups, total_wasted })
    }
}

// ── Data transfer structs ──────────────────────────────────────────────

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
}

#[derive(Debug, Clone)]
pub struct DuplicateGroupRow {
    pub id: i64,
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
