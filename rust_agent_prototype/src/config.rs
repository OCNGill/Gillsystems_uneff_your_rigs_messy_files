//! # Configuration Module — Immutable Runtime Settings
//!
//! Loads agent configuration from TOML files. Provides compile-time validation
//! and sensible defaults. All settings are immutable after initialization.
//!
//! ## Configuration Priority (highest to lowest)
//! 1. Environment variables (TODO: implement ENV override)
//! 2. TOML file (config.toml in database directory)
//! 3. Hardcoded defaults
//!
//! ## Key Settings
//! - **grpc_port**: TCP port for gRPC service (default: 50051)
//! - **database_path**: SQLite database file location
//! - **cache_size_mb**: SQLite memory cache (default: 256 MB)
//! - **thread_count**: File scanner threads (default: num_cpus)
//! - **remediation_strategy**: Primary dedup method (ZFS, NTFS, etc.)
//! - **quarantine_dir**: Safe backup location before destructive operations
//! - **enable_wal**: SQLite write-ahead log (default: true for performance)
//!
//! ## TOML Format
//! ```toml
//! grpc_port = 50051
//! thread_count = 16
//! 
//! [database]
//! path = "/var/cache/uneff/uneff.db"
//! cache_size_mb = 256
//! wal_mode = true
//! 
//! [remediation]
//! strategy = "zfs_clone"
//! quarantine_dir = "/var/cache/uneff/quarantine"
//! delete_after_verification = false
//! ```

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub grpc_port: u16,
    pub orchestrator_url: Option<String>,
    pub database: DatabaseConfig,
    pub scanning: ScanningConfig,
    pub security: SecurityConfig,
    pub logging: LoggingConfig,
    pub remediation: RemediationConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseConfig {
    pub path: String,
    pub cache_size_mb: u32,
    pub wal_mode: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanningConfig {
    pub max_file_size_gb: u64,
    pub default_exclude_patterns: Vec<String>,
    pub thread_pool_size: usize,
    pub hash_batch_size: usize,
    pub progress_report_interval_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityConfig {
    pub tls_cert_path: Option<String>,
    pub tls_key_path: Option<String>,
    pub ca_cert_path: Option<String>,
    pub client_auth_required: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingConfig {
    pub level: String,
    pub file_path: Option<String>,
    pub max_file_size_mb: u32,
    pub max_files: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemediationConfig {
    pub quarantine_path: String,
    pub grace_period_hours: u32,
    pub verify_before_delete: bool,
    pub filesystem_priority: Vec<String>,
    pub max_hard_links_per_file: u32,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            grpc_port: 50051,
            orchestrator_url: None,
            database: DatabaseConfig::default(),
            scanning: ScanningConfig::default(),
            security: SecurityConfig::default(),
            logging: LoggingConfig::default(),
            remediation: RemediationConfig::default(),
        }
    }
}

impl Default for DatabaseConfig {
    fn default() -> Self {
        Self {
            path: "gillsystems_uneff_cache.db".to_string(),
            cache_size_mb: 64,
            wal_mode: true,
        }
    }
}

impl Default for ScanningConfig {
    fn default() -> Self {
        Self {
            max_file_size_gb: 10,
            default_exclude_patterns: vec![
                "*.tmp".to_string(),
                "*.temp".to_string(),
                "*.swp".to_string(),
                "*.swo".to_string(),
                ".git/**".to_string(),
                "node_modules/**".to_string(),
                "target/**".to_string(),
                "*.log".to_string(),
                "$Recycle.Bin/**".to_string(),
                "System Volume Information/**".to_string(),
            ],
            thread_pool_size: num_cpus::get(),
            hash_batch_size: 1000,
            progress_report_interval_ms: 5000,
        }
    }
}

impl Default for SecurityConfig {
    fn default() -> Self {
        Self {
            tls_cert_path: None,
            tls_key_path: None,
            ca_cert_path: None,
            client_auth_required: false,
        }
    }
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            level: "info".to_string(),
            file_path: None,
            max_file_size_mb: 100,
            max_files: 5,
        }
    }
}

impl Default for RemediationConfig {
    fn default() -> Self {
        Self {
            quarantine_path: "quarantine".to_string(),
            grace_period_hours: 72,
            verify_before_delete: true,
            // ZFS first → NTFS second → ext4/XFS → FAT32 fallback
            filesystem_priority: vec![
                "zfs".to_string(),
                "ntfs".to_string(),
                "ext4".to_string(),
                "xfs".to_string(),
                "btrfs".to_string(),
                "apfs".to_string(),
                "fat32".to_string(),
            ],
            max_hard_links_per_file: 1023,
        }
    }
}

impl Config {
    pub fn load<P: AsRef<Path>>(path: P) -> Result<Self> {
        let path = path.as_ref();
        
        if !path.exists() {
            // Create default config file
            let default_config = Config::default();
            let config_str = toml::to_string_pretty(&default_config)
                .context("Failed to serialize default config")?;
            
            fs::write(path, config_str)
                .context("Failed to write default config file")?;
            
            tracing::info!("Created default config file at: {}", path.display());
            return Ok(default_config);
        }
        
        let content = fs::read_to_string(path)
            .context("Failed to read config file")?;
        
        let config: Config = toml::from_str(&content)
            .context("Failed to parse config file")?;
        
        tracing::info!("Loaded configuration from: {}", path.display());
        Ok(config)
    }
    
    pub fn validate(&self) -> Result<()> {
        // Validate database path
        if self.database.path.is_empty() {
            return Err(anyhow::anyhow!("Database path cannot be empty"));
        }
        
        // Validate scanning config
        if self.scanning.max_file_size_gb == 0 {
            return Err(anyhow::anyhow!("Max file size must be greater than 0"));
        }
        
        if self.scanning.thread_pool_size == 0 {
            return Err(anyhow::anyhow!("Thread pool size must be greater than 0"));
        }
        
        // Validate TLS config if provided
        if let (Some(cert_path), Some(key_path)) = (&self.security.tls_cert_path, &self.security.tls_key_path) {
            if !Path::new(cert_path).exists() {
                return Err(anyhow::anyhow!("TLS certificate file not found: {}", cert_path));
            }
            if !Path::new(key_path).exists() {
                return Err(anyhow::anyhow!("TLS key file not found: {}", key_path));
            }
        }
        
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    
    #[test]
    fn test_default_config() {
        let config = Config::default();
        assert_eq!(config.grpc_port, 50051);
        assert_eq!(config.database.cache_size_mb, 64);
        assert!(config.scanning.default_exclude_patterns.len() > 0);
    }
    
    #[test]
    fn test_config_load_create_default() -> Result<()> {
        let dir = tempdir()?;
        let config_path = dir.path().join("test_config.toml");
        
        let config = Config::load(&config_path)?;
        assert!(config_path.exists());
        assert_eq!(config.grpc_port, 50051);
        
        Ok(())
    }
    
    #[test]
    fn test_config_validation() -> Result<()> {
        let mut config = Config::default();
        
        // Valid config should pass
        assert!(config.validate().is_ok());
        
        // Invalid database path
        config.database.path = "".to_string();
        assert!(config.validate().is_err());
        
        Ok(())
    }
}
