# Gillsystems_uneff_your_rigs_messy_files Architecture Design

## System Architecture Overview

```
┌─────────────────┐    TLS/mTLS    ┌─────────────────┐
│   GUI Client    │◄──────────────►│    Program      │
│   (Tauri/Web)   │                │   (Rust Binary) │
│                 │                │                 │
│ - File Explorer │                │ - File Scanner  │
│ - Dual Panel    │                │ - Hash Engine   │
│ - Network View  │                │ - gRPC Server   │
│ - CRUD Ops      │                │ - Service Mgr   │
└─────────────────┘                └─────────────────┘
         │                                   │
         │                                   │
    ┌────▼────┐                         ┌────▼────┐
    │ Shared  │                         │ File    │
    │ State   │                         │ System  │
    │ (SQLite)│                         └─────────┘
    └─────────┘
```

## GUI Client Architecture (Windows 10 Explorer Style)

### Layout Components

```
┌─────────────────────────────────────────────────────────────┐
│ Top Bar: File | Edit | View | Tools | Help                   │
│ [New] [Open] [Save] [Select All] [Cut] [Copy] [Paste] [Delete]│
├─────────────────────────────────────────────────────────────┤
│ Left Sidebar              │ Dual Panel Area                │
│ ┌─────────────────────┐   │ ┌─────────────┬─────────────┐   │
│ │ Network Devices     │   │ │ Panel A     │ Panel B     │   │
│ │ ├─ System1 (192.168.1.10) │ │ │ [Duplicates] │ [Locations] │   │
│ │ ├─ System2 (192.168.1.11) │ │ │ File1.txt   │ /path/to/1  │   │
│ │ ├─ NAS1 (192.168.1.100) │ │ │ File1.txt   │ /path/to/2  │   │
│ │ └─ Laptop (192.168.1.50) │ │ │ File2.doc   │ /docs/file2 │   │
│ │                     │   │ │ File2.doc   │ /backup/doc2│   │
│ │ Local Drives        │   │ └─────────────┴─────────────┘   │
│ │ ├─ C:\             │   │                                 │
│ │ ├─ D:\             │   │ Metadata Popup (on hover)       │
│ │ └─ E:\             │   │ Size: 1.2MB                     │
│ │                     │   │ Modified: 2024-01-15            │
│ │ Quick Access        │   │ Hash: SHA256:abc...            │
│ │ ├─ Recent           │   │ Permissions: rw-r--r--          │
│ │ └─ Favorites        │   │                                 │
│ └─────────────────────┘   └─────────────────────────────────┘
├─────────────────────────────────────────────────────────────┤
│ Status Bar: X files | Y duplicates | Z MB wasted | Connected │
└─────────────────────────────────────────────────────────────┘
```

### GUI Features

**Top Bar Operations:**
- **File**: New scan, Open saved scan, Save results, Exit
- **Edit**: Select All, Invert Selection, Cut, Copy, Paste, Delete, Rename
- **View**: List/Details/Thumbnails, Refresh, Options, Filter
- **Tools**: Scan Now, Schedule Scan, Settings, Network Discovery
- **Help**: User Guide, About, Check for Updates

**Left Sidebar (Windows 10 Style):**
- Network devices with real-time status indicators
- Local drives with space usage bars
- Quick access to recent scans and favorites
- Expandable tree view with proper icons

**Dual Panel Middle Area:**
- Panel A: Duplicate files grouped by hash
- Panel B: File locations and paths
- Synchronized scrolling between panels
- Drag-and-drop support between panels
- Right-click context menus

**Interactive Features:**
- Hover metadata popup with file details
- Double-click to open file in default application
- Multi-select with Ctrl/Shift modifiers
- Keyboard shortcuts for all operations

## Peer-to-Peer Architecture

```
┌─────────────┐    gRPC/mTLS    ┌─────────────┐    gRPC/mTLS    ┌─────────────┐
│   Node A    │◄──────────────►│   Node B    │◄──────────────►│   Node C    │
│ 192.168.1.10│                │ 192.168.1.11│                │ 192.168.1.12│
│             │                │             │                │             │
│ - Program   │                │ - Program   │                │ - Program   │
│ - GUI       │                │ - GUI       │                │ - GUI       │
│ - Local DB  │                │ - Local DB  │                │ - Local DB  │
└─────────────┘                └─────────────┘                └─────────────┘
         │                               │                               │
         └───────────── Shared State ──────────────────────────────────────┘
                          (Distributed SQLite)

### Certificate Management

- **Self-signed certificates** for peer-to-peer trust
- **mTLS authentication** between nodes
- **No central authority** - distributed trust model

**Core Binary:**
- **Rust**: Single executable with program core + GUI
- **egui/eframe**: Lightweight native GUI toolkit
- **tokio**: Async runtime for scanning and networking
- **gRPC**: Inter-node communication
- **SQLite**: Local database with peer-to-peer sync

**GUI Framework: egui/eframe**
- Native window, no browser required
- Cross-platform (Windows, Linux, macOS)
- Immediate mode GUI for responsiveness
- Custom styling for Windows 10 Explorer look

### Single Binary Structure

```
gillsystems-uneff-your-rigs-messy-files.exe (Single Executable)
├── Program Service (background thread)
│   ├── File Scanner
│   ├── Hash Engine  
│   ├── Network Discovery
│   └── Database Sync
├── GUI Frontend (main thread)
│   ├── Windows 10 Explorer UI
│   ├── Dual Panel View
│   ├── Network Browser
│   └── Settings Dialog
└── Service Manager
    ├── Auto-start registration
    ├── Background service control
    └── System integration
```

### Auto-Start Integration

**Linux (systemd):**
```bash
# Auto-generated by installer
~/.config/systemd/user/gillsystems-uneff.service
[Unit]
Description=Gillsystems_uneff_your_rigs_messy_files Program
After=graphical-session.target

[Service]
Type=simple
ExecStart=/opt/gillsystems-uneff/gillsystems-uneff-your-rigs-messy-files --service-mode
Restart=always
RestartSec=5

[Install]
WantedBy=default.target
```

**Windows (Service + Run Key):**
```rust
// Windows service registration
windows_service::service_dispatcher::start("GillsystemsUneff", ffi_service_main);

// Run key for GUI auto-start
reg::HKEY_CURRENT_USER.create_subkey("Software\\Microsoft\\Windows\\CurrentVersion\\Run")?
    .set_value("GillsystemsUneff", &"C:\\Program Files\\Gillsystems_uneff_your_rigs_messy_files\\gillsystems-uneff-your-rigs-messy-files.exe");
```

**macOS (LaunchDaemon):**
```xml
<!-- ~/Library/LaunchDaemons/com.gillsystems.uneff-your-rigs-messy-files.plist -->
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" 
  "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key><string>com.gillsystems.uneff-your-rigs-messy-files</string>
    <key>ProgramArguments</key>
    <array>
        <string>/Applications/Gillsystems_uneff_your_rigs_messy_files.app/Contents/MacOS/gillsystems-uneff-your-rigs-messy-files</string>
        <string>--service-mode</string>
    </array>
    <key>RunAtLoad</key><true/>
</dict>
</plist>
```

## Permission Model (Admin/Sudo Assumed)

### No Guardrails Philosophy

```
POWER TO THE PEOPLE MODE:
├── Full system access (assumed admin/sudo)
├── No authentication required
├── No role-based restrictions
├── All operations allowed
└── Funny warnings for dangerous actions
```

**Warning Messages:**
```rust
// Example warning system
fn warn_user(action: &str) -> String {
    match action {
        "delete_all" => "⚠️  HOLY COW! You're about to delete EVERYTHING! This is like nuking your digital life from orbit. Are you absolutely sure you didn't just escape from the asylum?".to_string(),
        "format_drive" => "🔥 WHOA THERE! Formatting drives is permanent! Like, really permanent. Don't come crying to me when you realize your thesis was on there. Think, McFly, think!".to_string(),
        _ => format!("⚠️  About to {}: This might be a bad idea. But hey, it's your system!", action),
    }
}
```

## Multi-Node Architecture (10+ Devices)

### Scalable Network Design

```
Network Topology (10+ Nodes):
┌─────────────┐    gRPC/mTLS    ┌─────────────┐
│   Node A    │◄──────────────►│   Node B    │
│ Main Desktop │                │ Laptop      │
│ C:, D:, E:  │                │ C:, USB:    │
└─────────────┘                └─────────────┘
         │                               │
         └─────────────┬─────────────────┘
                       │
    ┌──────────────────┼──────────────────┐
    │                  │                  │
┌───▼───┐    ┌────────▼─────┐    ┌────▼────┐
│Node C │    │   Node D     │    │ Node E  │
│Server │    │ NAS Storage  │    │ HTPC    │
│ZFS    │    │ SMB Shares   │    │Media    │
└───────┘    └──────────────┘    └─────────┘
    │              │                  │
    └──────────────┼──────────────────┘
                   │
        ┌──────────▼──────────┐
        │    Nodes F-J        │
        │  Mobile Devices     │
        │  USB Sticks         │
        │  External Drives    │
        └─────────────────────┘
```

### Multi-Drive Support

**Drive Detection Per Node:**
```rust
// Cross-platform drive enumeration
fn enumerate_drives() -> Vec<DriveInfo> {
    #[cfg(windows)]
    {
        // C:, D:, E:, USB drives, network shares
        windows::get_all_drives()
    }
    #[cfg(linux)]
    {
        // /, /home, /mnt/*, ZFS pools, USB mounts
        linux::get_all_mounts()
    }
    #[cfg(macos)]
    {
        // /, /Volumes/*, external drives
        macos::get_all_volumes()
    }
}
```

**Storage Types Supported:**
- **Internal**: SATA, NVMe, M.2 drives
- **External**: USB 3.0, USB-C, Thunderbolt drives
- **Network**: SMB/CIFS shares, NFS mounts
- **Special**: ZFS pools, RAID arrays, LVM volumes
- **Removable**: SD cards, USB sticks, external SSDs

### Database Schema (Enhanced for Multi-Drive)

```sql
-- Enhanced nodes table with drive info
CREATE TABLE nodes (
    id TEXT PRIMARY KEY,
    hostname TEXT NOT NULL,
    ip_address TEXT NOT NULL,
    platform TEXT NOT NULL,  -- windows, linux, macos
    version TEXT NOT NULL,
    last_seen INTEGER NOT NULL,
    status TEXT DEFAULT 'offline',
    total_drives INTEGER DEFAULT 0,
    total_space INTEGER DEFAULT 0,
    available_space INTEGER DEFAULT 0,
    capabilities TEXT,  -- JSON: supported features
    created_at INTEGER DEFAULT (strftime('%s', 'now'))
);

-- Drives table for multi-drive support
CREATE TABLE drives (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    node_id TEXT NOT NULL,
    drive_letter TEXT,  -- C:, D:, etc. (Windows)
    mount_point TEXT,   -- /mnt/data, /Volumes/USB (Linux/macOS)
    drive_type TEXT NOT NULL,  -- internal, external, network, zfs
    filesystem_type TEXT,  -- NTFS, ext4, ZFS, APFS
    total_space INTEGER NOT NULL,
    available_space INTEGER NOT NULL,
    is_removable BOOLEAN DEFAULT FALSE,
    is_network BOOLEAN DEFAULT FALSE,
    label TEXT,
    serial_number TEXT,
    last_scanned INTEGER,
    FOREIGN KEY (node_id) REFERENCES nodes(id)
);

-- Updated files table with drive reference
CREATE TABLE files (
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
```

## Program Architecture (Rust)

### Core Components

```
un-eff Binary
├── main.rs (entry point, service management)
├── lib/
│   ├── scanner/
│   │   ├── mod.rs
│   │   ├── file_walker.rs     # Cross-platform file enumeration
│   │   ├── mount_detector.rs  # Local/network mount discovery
│   │   └── exclusion_filter.rs # .gitignore-style patterns
│   ├── hashing/
│   │   ├── mod.rs
│   │   ├── xxhash_engine.rs   # Fast pre-filtering
│   │   ├── sha256_engine.rs  # Collision-resistant hashing
│   │   └── hash_pipeline.rs   # Coordinated hashing strategy
│   ├── network/
│   │   ├── mod.rs
│   │   ├── grpc_server.rs     # gRPC service implementation
│   │   ├── tls_manager.rs     # Mutual TLS handling
│   │   └── peer_discovery.rs  # Network node discovery
│   ├── remediation/
│   │   ├── mod.rs
│   │   ├── quarantine.rs      # Safe file movement
│   │   ├── zfs_dedup.rs       # ZFS block cloning / reflink (PRIMARY)
│   │   ├── hard_link.rs       # NTFS/ext4/XFS/APFS hard linking
│   │   ├── fat_fallback.rs    # FAT32/exFAT copy-delete (no dedup)
│   │   └── safe_delete.rs     # Grace period deletion
│   └── platform/
│       ├── mod.rs
│       ├── windows.rs         # Win32 API, service integration
│       ├── linux.rs           # systemd, inotify
│       └── common.rs          # Shared utilities
```

### Data Flow

```
File System → Scanner → Hash Pipeline → Metadata Store → gRPC → Program Core
     ↓              ↓           ↓              ↓              ↓
  Raw Files    File Info  xxHash/SHA256   Local Cache   Network Stream
```

## Network Sync Architecture

### Components

```
Program Service
├── api/
│   ├── rest/                    # REST endpoints
│   │   ├── nodes.rs
│   │   ├── scans.rs
│   │   ├── duplicates.rs
│   │   └── remediation.rs
│   └── grpc/                    # Program communication
│       └── program_service.rs
├── core/
│   ├── dedupe/
│   │   ├── analyzer.rs          # Duplicate detection logic
│   │   ├── grouping.rs          # Size → hash → verification
│   │   └── space_calculator.rs  # Waste estimation
│   ├── database/
│   │   ├── models.rs            # SQLAlchemy/ORM models
│   │   ├── migrations/          # Schema migrations
│   │   └── queries.rs           # Optimized queries
│   └── security/
│       ├── auth.rs              # JWT/RBAC
│       ├── tls.rs               # Certificate management
│       └── audit.rs             # Action logging
├── web/
│   ├── static/                  # CSS/JS assets
│   ├── templates/               # HTML templates
│   └── dashboard.rs             # Server-side rendering
└── services/
    ├── scheduler.rs             # Background scan scheduling
    ├── report_generator.rs      # CSV/JSON export
    └── notification.rs          # Alert system
```

## Database Schema

### Core Tables

```sql
-- Network nodes registration
CREATE TABLE nodes (
    id TEXT PRIMARY KEY,
    hostname TEXT NOT NULL,
    ip_address TEXT NOT NULL,
    platform TEXT NOT NULL,  -- windows, linux, macos
    version TEXT NOT NULL,
    last_seen INTEGER NOT NULL,
    status TEXT DEFAULT 'offline',  -- online, offline, scanning
    capabilities TEXT,  -- JSON: supported features
    total_space INTEGER,
    available_space INTEGER,
    created_at INTEGER DEFAULT (strftime('%s', 'now'))
);

-- File metadata from all nodes
CREATE TABLE files (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    node_id TEXT NOT NULL,
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
    FOREIGN KEY (scan_id) REFERENCES scans(id)
);

-- Scan sessions for tracking
CREATE TABLE scans (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    node_id TEXT NOT NULL,
    initiated_by TEXT NOT NULL,  -- Which node started the scan
    started_at INTEGER NOT NULL,
    completed_at INTEGER,
    status TEXT DEFAULT 'running',  -- running, completed, failed
    files_scanned INTEGER DEFAULT 0,
    bytes_scanned INTEGER DEFAULT 0,
    error_count INTEGER DEFAULT 0,
    config TEXT,  -- JSON: scan configuration
    FOREIGN KEY (node_id) REFERENCES nodes(id)
);

-- Duplicate groups for efficient querying
CREATE TABLE duplicate_groups (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    sha256_hash TEXT NOT NULL UNIQUE,
    size_bytes INTEGER NOT NULL,
    file_count INTEGER NOT NULL,
    total_wasted_bytes INTEGER NOT NULL,
    first_seen_at INTEGER DEFAULT (strftime('%s', 'now')),
    last_updated_at INTEGER DEFAULT (strftime('%s', 'now'))
);

-- Link duplicate groups to files
CREATE TABLE duplicate_files (
    group_id INTEGER NOT NULL,
    file_id INTEGER NOT NULL,
    is_primary BOOLEAN DEFAULT FALSE,  -- Recommended keep file
    remediation_status TEXT DEFAULT 'none',  -- none, quarantined, linked, deleted
    remediation_at INTEGER,
    FOREIGN KEY (group_id) REFERENCES duplicate_groups(id),
    FOREIGN KEY (file_id) REFERENCES files(id),
    PRIMARY KEY (group_id, file_id)
);

-- Remediation actions for audit trail
CREATE TABLE remediation_actions (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    group_id INTEGER NOT NULL,
    action_type TEXT NOT NULL,  -- quarantine, link, delete
    file_ids TEXT NOT NULL,  -- JSON array of affected file IDs
    initiated_by_node TEXT NOT NULL,  -- Which node initiated
    executed_by_node TEXT NOT NULL,   -- Which node executed
    initiated_at INTEGER NOT NULL,
    completed_at INTEGER,
    status TEXT DEFAULT 'pending',  -- pending, completed, failed
    space_recovered INTEGER,
    error_message TEXT,
    FOREIGN KEY (group_id) REFERENCES duplicate_groups(id)
);

-- User operations (for GUI tracking)
CREATE TABLE user_operations (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    node_id TEXT NOT NULL,
    operation_type TEXT NOT NULL,  -- scan, delete, copy, move
    target_files TEXT NOT NULL,  -- JSON array of file paths
    operation_details TEXT,  -- JSON: additional context
    timestamp INTEGER DEFAULT (strftime('%s', 'now')),
    status TEXT DEFAULT 'completed',  -- completed, failed, cancelled
    FOREIGN KEY (node_id) REFERENCES nodes(id)
);

-- System preferences (synced across nodes)
CREATE TABLE preferences (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL,
    updated_by_node TEXT NOT NULL,
    updated_at INTEGER DEFAULT (strftime('%s', 'now')),
    FOREIGN KEY (updated_by_node) REFERENCES nodes(id)
);

-- Audit log for compliance
CREATE TABLE audit_log (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    user_id TEXT,
    action TEXT NOT NULL,
    resource_type TEXT NOT NULL,  -- program, scan, duplicate, remediation
    resource_id TEXT NOT NULL,
    details TEXT,  -- JSON: additional context
    ip_address TEXT,
    user_agent TEXT,
    timestamp INTEGER DEFAULT (strftime('%s', 'now')),
    FOREIGN KEY (user_id) REFERENCES users(id)
);
```

### Indexes for Performance

```sql
-- File lookup optimization
CREATE INDEX idx_files_node_path ON files(node_id, file_path);
CREATE INDEX idx_files_sha256 ON files(sha256_hash);
CREATE INDEX idx_files_size ON files(size_bytes);
CREATE INDEX idx_files_scan ON files(scan_id);

-- Duplicate group optimization
CREATE INDEX idx_duplicate_groups_hash ON duplicate_groups(sha256_hash);
CREATE INDEX idx_duplicate_groups_size ON duplicate_groups(size_bytes);
CREATE INDEX idx_duplicate_groups_wasted ON duplicate_groups(total_wasted_bytes DESC);

-- Remediation tracking
CREATE INDEX idx_duplicate_files_group ON duplicate_files(group_id);
CREATE INDEX idx_duplicate_files_status ON duplicate_files(remediation_status);

-- Audit and monitoring
CREATE INDEX idx_audit_timestamp ON audit_log(timestamp DESC);
CREATE INDEX idx_audit_user ON audit_log(user_id);
```

## Security Architecture (Full Admin / Full Speed)

### No Guardrails Philosophy — Maximum Performance by Default

```
All Nodes Admin Mode (FULL SPEED):
├── Full system access (admin/sudo REQUIRED at launch)
├── Auto-elevation on startup (Windows UAC / Linux sudo / macOS root)
├── Unrestricted filesystem traversal — every drive, every path, zero exceptions
├── No authentication between local operations
├── No role-based restrictions — all operations allowed
├── Maximum thread pool — all CPU cores utilized by default
├── GPU acceleration enabled where available
├── No I/O throttling — full disk bandwidth for scanning
├── No file count limits — scan millions without caps
├── Funny warnings for dangerous actions (but never blocking)
└── FULL SPEED. NO BRAKES. POWER TO THE PEOPLE.
```

## Performance Optimizations

### Scanning Strategy

1. **Size Pre-filtering**: Group files by size first (O(1) comparison)
2. **Fast Hash**: xxHash64 for initial duplicate detection
3. **Strong Hash**: SHA-256 only for size+xxHash matches
4. **Byte Verification**: Optional for critical files before deletion

### Database Optimizations

- **Batch Inserts**: 1000-file transactions
- **Connection Pooling**: 10-20 connections per service
- **Read Replicas**: Separate read DB for reporting
- **Partitioning**: Monthly partitions for large deployments

### Network Efficiency

- **Streaming**: gRPC bidirectional streams for large scans
- **Compression**: gzip for metadata payloads
- **Delta Updates**: Only send changed file metadata
- **Local Caching**: Program-side SQLite for scan persistence

## Deployment Architecture

### Single Admin (MVP)

```
Workstation
├── Orchestrator (localhost:8080)
├── SQLite Database
├── Self-signed Certificates
└── Program Binaries
    ├── Windows Service
    └── Linux Systemd Unit
```

### Enterprise Scale

```
Load Balancer
├── Orchestrator Cluster (3+ nodes)
├── PostgreSQL Database
├── Redis Cache
├── Certificate Authority
└── Binary Distribution
    ├── Windows Service Installer
    └── Linux Package (deb/rpm)
```

## Monitoring & Observability

### Performance Targets

### Scanning Performance
- **10,000+ files/second** scanning speed
- **<1GB** memory usage for large scans
- **Real-time progress** updates
- **Parallel processing** across drives

### GUI Performance
- **60 FPS** animations and transitions
- **<100ms** UI response time
- **<50MB** additional memory for Aero effects
- **GPU acceleration** where available

### Network Performance
- **<5 second** sync latency on local network
- **Efficient delta updates** between nodes
- **Offline operation** support
- **Conflict resolution** for concurrent operations

### Metrics Collection

- **Scan Performance**: Files/sec, MB/sec, hash rates
- **System Health**: CPU, memory, disk usage per agent
- **Duplicate Statistics**: Waste percentage, group sizes
- **Remediation Success**: Actions completed, space recovered

### Logging Strategy

- **Structured JSON**: All logs in JSON format
- **Log Levels**: ERROR, WARN, INFO, DEBUG
- **Centralized**: Fluentd/Logstash aggregation
- **Retention**: 90 days default, 1 year for audit logs

### Alerting

- **Node Offline**: >5 minutes without heartbeat
- **Scan Failures**: >10% error rate
- **Database Issues**: Connection failures, slow queries
- **Security Events**: Failed auth, certificate issues
