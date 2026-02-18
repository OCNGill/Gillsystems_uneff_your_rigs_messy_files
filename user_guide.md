# Gillsystems_uneff_your_rigs_messy_files — User Guide

**Version**: 0.5.0  
**Author**: GillSystems  
**Methodology**: 7D Agile × SWEET Principles  
**Philosophy**: Power to the People 🚀

---

## 🚀 **QUICK START — JUST DOWNLOAD AND RUN!**

### Get the Binary
1. **Windows users**: Download `gillsystems-uneff-your-rigs-messy-files-windows-x64.exe` from root
2. **Linux users**: Download `gillsystems-uneff-your-rigs-messy-files-linux-x64` from root
3. **Run it immediately** — no installation, no dependencies, no setup

```bash
# Windows (from PowerShell or Command Prompt):
gillsystems-uneff-your-rigs-messy-files-windows-x64.exe

# Linux (from terminal):
./gillsystems-uneff-your-rigs-messy-files-linux-x64
```

**That's it. RUN FREE! 🔓**

---

## Table of Contents

1. [Quick Start](#-quick-start--just-download-and-run) ← Start here!
2. [Introduction](#1-introduction)
3. [System Requirements](#2-system-requirements)
4. [Installation](#3-installation)
5. [First Launch](#4-first-launch)
6. [The Interface — Windows 7 Aero Style](#5-the-interface--windows-7-aero-style)
7. [Scanning for Duplicates](#6-scanning-for-duplicates)
8. [Managing Duplicate Files](#7-managing-duplicate-files)
9. [Network & Multi-Node Operations](#8-network--multi-node-operations)
10. [Configuration](#9-configuration)
11. [Platform-Specific Setup](#10-platform-specific-setup)
12. [Run Modes](#11-run-modes)
13. [Remediation Actions](#12-remediation-actions)
14. [Keyboard Shortcuts & Quick Actions](#13-keyboard-shortcuts--quick-actions)
15. [Troubleshooting](#14-troubleshooting)
16. [FAQ](#15-faq)
17. [Philosophy & Mission](#16-philosophy--mission)
18. [Glossary](#17-glossary)

---

## 1. Introduction

**Gillsystems_uneff_your_rigs_messy_files** is a cross-platform duplicate file detection and management system built in Rust. It ships as a **single binary** — no installers, no dependencies, no web servers. Just raw power.

### What It Does
- Scans local and network-connected drives for duplicate files
- Groups duplicates by content hash (xxHash64 for speed, SHA-256 for verification)
- Provides a **Windows 7 Aero-styled** graphical interface for managing results
- Supports **peer-to-peer** multi-node architecture (10+ devices, 1000+ drives)
- Offers remediation: quarantine, delete, hard-link, or move duplicates
- Runs as a background service or interactive GUI — or both simultaneously

### Who It's For - ARE YOUR RIGS A MESS? THIS TOOL IS FOR YOU.
Anyone who is tired of wasting storage on duplicate files scattered across multiple machines. Whether you're managing a home lab, a fleet of workstations, or a NAS empire — this tool puts the power in your hands with **zero artificial limitations**.

### Design Philosophy
- **Power to the People**: No guardrails. Admin/sudo REQUIRED. Full speed, no brakes.
- **Full Admin / Full Speed**: Auto-elevation at launch. Every CPU core. GPU when available. No throttling.  **USE AT YOUR OWN RISK** — but we know you can handle it.
- **Radical Transparency**: You see everything. Every hash, every path, every byte.
- **Anti-BS**: No bloat, no Electron, no cloud dependencies. Single native binary.
- **User Empowerment**: Funny but honest warnings. No silent deletions. Never blocking.

---

## 2. System Requirements

### Minimum Requirements

| Component | Requirement |
|-----------|-------------|
| **OS** | Windows 7+, Linux (kernel 4.x+), macOS 11+ |
| **CPU** | x86_64 or ARM64, 2+ cores recommended |
| **RAM** | 512 MB available (2 GB recommended for large scans) |
| **Disk** | 50 MB for the binary, additional space for cache DB |
| **Privileges** | Administrator (Windows) or sudo/root (Linux/macOS) |
| **Rust** | 1.70+ (for building from source only) |

### Recommended for Best Performance

| Component | Recommendation |
|-----------|----------------|
| **CPU** | 4+ cores (enables parallel scanning pipeline) |
| **RAM** | 4 GB+ (for scanning millions of files) |
| **GPU** | Any with OpenGL 3.3+ (enables Aero glass effects) |
| **Network** | Gigabit LAN (for multi-node sync <5 second latency) |
| **Storage** | SSD for database cache |

### Supported Drive Types
- **Internal**: SATA, NVMe, M.2
- **External**: USB 3.0, USB-C, Thunderbolt
- **Network**: SMB/CIFS shares, NFS mounts
- **Special**: ZFS pools, RAID arrays, LVM volumes
- **Removable**: SD cards, USB sticks, external SSDs

---

## 3. Installation

### Option A: Pre-built Binary (Recommended)

Download the latest release for your platform from the [GitHub Releases](https://github.com/OCNGill/Gillsystems_uneff_your_rigs_messy_files/releases) page.

| Platform | Binary |
|----------|--------|
| Windows (x64) | `gillsystems-uneff-your-rigs-messy-files.exe` |
| Linux (x64) | `gillsystems-uneff-your-rigs-messy-files` |
| macOS (ARM/x64) | `gillsystems-uneff-your-rigs-messy-files` |

Place it anywhere you like. It's a single file. No installation wizard needed.

### Option B: Build from Source

```bash
# Clone the repository
git clone https://github.com/OCNGill/Gillsystems_uneff_your_rigs_messy_files.git
cd Gillsystems_uneff_your_rigs_messy_files/rust_agent_prototype

# Build optimized release binary
cargo build --release

# Binary location:
# ./target/release/gillsystems-uneff-your-rigs-messy-files      (Linux/macOS)
# ./target/release/gillsystems-uneff-your-rigs-messy-files.exe  (Windows)
```

#### Build Dependencies
- Rust 1.70+ with `cargo`
- Protobuf compiler (`protoc`) for gRPC code generation
- Platform-specific:
  - **Windows**: MSVC build tools
  - **Linux**: `gcc`, `pkg-config`, `libgtk-3-dev` (for GUI)
  - **macOS**: Xcode command line tools

### Option C: Platform Package Managers (Coming Soon)
```bash
# Windows (winget)
winget install GillSystems.UneffYourRigsMessyFiles

# Linux (snap)
snap install gillsystems-uneff

# macOS (brew)
brew install gillsystems-uneff-your-rigs-messy-files
```

---

## 4. First Launch

### Quick Start

1. **Run the binary** — double-click or execute from terminal
2. **The Aero interface appears** — Windows 7 styled glass effects
3. **Network discovery begins** — other nodes on your LAN are auto-detected
4. **Select drives** — choose which drives to scan in the left sidebar
5. **Click 🔍 Scan** — the scanning pipeline fires up
6. **Review results** — duplicates appear in the dual-panel view
7. **Take action** — delete, quarantine, hard-link, or move files

### First-Run Configuration

On first launch, if no `config.toml` exists, a default configuration file is automatically created in the working directory. You can customize it before or after the first scan.

Default config location: `./config.toml`

---

## 5. The Interface — Windows 7 Aero Style

The GUI brings back the **classic Windows 7 Aero glass aesthetic** with:
- Translucent glass panel backgrounds
- Subtle shadows and rounded corners
- Blue/highlight color scheme inspired by Windows 7 Aero Blue
- Smooth hover effects and transitions targeting 60 FPS

### Layout Overview

```
┌─────────────────────────────────────────────────────────────┐
│ Menu Bar: File | Edit | View | Tools | Help     [🔍][🗑️][⚙️] │
├──────────────┬──────────────────────────────────────────────┤
│              │                                              │
│  LEFT        │         DUAL PANEL AREA                      │
│  SIDEBAR     │  ┌──────────────┬───────────────────┐        │
│              │  │ Duplicate    │ File Locations     │        │
│  Network     │  │ Groups       │                    │        │
│  Devices     │  │              │  /path/to/file1    │        │
│  ● System1   │  │ 📁 3 files   │  /path/to/file2    │        │
│  ● System2   │  │ 📁 5 files   │  /backup/file3     │        │
│  ○ NAS1      │  │              │                    │        │
│              │  └──────────────┴───────────────────┘        │
│  Local       │                                              │
│  Drives      │  Hover any file for metadata popup:          │
│  C:\         │  Size, Modified date, Hash, Permissions      │
│  D:\         │                                              │
│  E:\         │                                              │
├──────────────┴──────────────────────────────────────────────┤
│ Status: 1,234 files | 56 duplicates | 2.3 GB wasted | Ready │
└─────────────────────────────────────────────────────────────┘
```

### Menu Bar

| Menu | Actions |
|------|---------|
| **File** | New Scan, Open Saved Scan, Save Results, Exit |
| **Edit** | Select All, Invert Selection, Cut, Copy, Paste, Delete |
| **View** | Refresh, Filter |
| **Tools** | Scan Now, Network Discovery, Settings |
| **Help** | User Guide, About |

### Quick Action Buttons (Top Right)
- **🔍 Scan** — Start a new scan immediately
- **🗑️ Delete Selected** — Delete selected duplicate files (with warning)
- **⚙️ Settings** — Open the settings dialog

### Left Sidebar

**Network Devices** — Shows all peer nodes discovered on your network:
- 🟢 Green dot = Online
- 🔴 Red dot = Offline
- 🟡 Yellow dot = Currently scanning

Click a node to view its drives and scan results.

**Local Drives** — Lists all mounted drives on the current machine with available space.

### Dual Panel Area

- **Left Panel (Duplicate Groups)** — Lists groups of duplicate files sorted by hash. Shows file count and wasted space per group.
- **Right Panel (File Locations)** — When you select a group, this panel shows every file path in that group. Includes delete (🗑️) and open-folder (📁) buttons per file.

### Status Bar (Bottom)
Displays real-time information:
- Total files processed
- Duplicates found
- Wasted space in MB/GB
- Current scan status (spinner while scanning, ✅ on completion)
- Connected node count

---

## 6. Scanning for Duplicates

### Starting a Scan

1. Click **🔍 Scan** in the quick actions bar, or go to **Tools → Scan Now**
2. The scanning pipeline begins across all selected drives

### How Scanning Works

The scanner uses a **multi-stage pipeline** for maximum performance:

```
Stage 1: File Discovery        → Walk directories, enumerate files
Stage 2: Size Pre-filtering     → Group files by identical size (O(1))
Stage 3: Fast Hash (xxHash64)   → Quick duplicate detection pass
Stage 4: Strong Hash (SHA-256)  → Collision-resistant verification
Stage 5: Results Aggregation    → Group confirmed duplicates
```

### Scanning Performance Targets
- **Speed**: 10,000+ files/second on modern hardware
- **Memory**: <1 GB even for scans spanning millions of files
- **Parallelism**: Automatic thread pool sized to your CPU core count

### Default Exclusion Patterns
The following are skipped by default (configurable in `config.toml`):

| Pattern | Reason |
|---------|--------|
| `*.tmp`, `*.temp` | Temporary files |
| `*.swp`, `*.swo` | Editor swap files |
| `.git/**` | Git repositories |
| `node_modules/**` | Node.js dependencies |
| `target/**` | Rust build output |
| `*.log` | Log files |
| `$Recycle.Bin/**` | Windows recycle bin |
| `System Volume Information/**` | Windows system data |

### Real-Time Progress

During a scan, the status bar updates with:
- Files processed count
- Bytes scanned
- Current file path being scanned
- Spinner animation

Progress reports are sent at configurable intervals (default: every 5 seconds).

---

## 7. Managing Duplicate Files

### Viewing Duplicates

After a scan completes, duplicates appear in the left panel grouped by content hash. Each group shows:
- Number of duplicate files
- File size
- Total wasted space (size × (count - 1))

Click a group to see all file paths in the right panel.

### Hover Metadata

Hover over any file to see a metadata popup:
- **Node**: Which machine the file lives on
- **Drive**: Which drive/volume
- **Modified**: Last modification timestamp
- **Size**: Exact byte count

### Selecting Files

- **Click** a group to see its files
- **Checkboxes** allow multi-selection for batch operations
- Use **Edit → Select All** or **Edit → Invert Selection** for bulk selection

### Deleting Files

1. Select the files you want to remove
2. Click **🗑️ Delete Selected** or use **Edit → Delete**
3. A **warning dialog** appears with an honest (and slightly entertaining) message:

> 🗑️ DELETE WARNING! You're about to permanently delete files. This action cannot be undone! Shut Up Commander, just delete my crap.  

4. Click **"DELETE for eff's salke!"** to proceed, or **"Shit-wrong click"** to abort

### Opening File Locations

Click the **📁** button next to any file to open its containing folder:
- **Windows**: Opens Explorer with the file selected
- **Linux**: Opens the parent directory with `xdg-open`
- **macOS**: Opens Finder at the parent directory

---

## 8. Network & Multi-Node Operations

### Peer-to-Peer Architecture

Gillsystems_uneff_your_rigs_messy_files uses a **fully decentralized** architecture. There is no central server. Every node is equal.

```
Node A ◄──── gRPC/mTLS ────► Node B
  │                             │
  └──── gRPC/mTLS ────► Node C ─┘
```

### Network Discovery

Go to **Tools → Network Discovery** to find other nodes on your LAN. Nodes advertise themselves and connect via gRPC with mutual TLS authentication.

### Multi-Drive Scanning Across Nodes

Each node reports its local drives. From any node's GUI, you can:
- See all drives across all connected nodes
- Trigger scans on remote nodes
- View duplicate files spanning multiple machines

### Sync & Conflict Resolution

- Scan results sync between nodes with **<5 second latency** on local networks
- Delta updates minimize bandwidth — only changed metadata is transmitted
- Conflict resolution handles concurrent scan operations gracefully
- All nodes maintain their own local SQLite database for offline operation

### Supported Scale
- **10+ nodes** simultaneously
- **1000+ drives** across the network
- **Millions of files** tracked per node

---

## 9. Configuration

### Config File: `config.toml`

A default config file is auto-generated on first run. Here's what you can configure:

### gRPC & Networking

```toml
grpc_port = 50051
# orchestrator_url = "https://other-node:50051"  # Optional: connect to specific node
```

### Database

```toml
[database]
path = "gillsystems_uneff_cache.db"   # SQLite database location
cache_size_mb = 64                      # In-memory cache size
wal_mode = true                         # Write-Ahead Logging for performance
```

### Scanning

```toml
[scanning]
max_file_size_gb = 10                   # Skip files larger than this
thread_pool_size = 8                    # Auto-detected from CPU cores
hash_batch_size = 1000                  # Files per hashing batch
progress_report_interval_ms = 5000      # Progress update frequency

default_exclude_patterns = [
    "*.tmp", "*.temp", "*.swp", "*.swo",
    ".git/**", "node_modules/**", "target/**",
    "*.log", "$Recycle.Bin/**",
    "System Volume Information/**"
]
```

### Security (TLS)

```toml
[security]
# tls_cert_path = "/path/to/cert.pem"
# tls_key_path = "/path/to/key.pem"
# ca_cert_path = "/path/to/ca.pem"
client_auth_required = false
```

### Logging

```toml
[logging]
level = "info"                          # error, warn, info, debug
# file_path = "/var/log/gillsystems_uneff.log"
max_file_size_mb = 100
max_files = 5
```

---

## 10. Platform-Specific Setup

### Windows (7–11)

**Auto-Start as Service:**
The application can register itself as a Windows Service with automatic restart:
```
Software\Microsoft\Windows\CurrentVersion\Run
  → GillsystemsUneff = "C:\Program Files\Gillsystems_uneff_your_rigs_messy_files\gillsystems-uneff-your-rigs-messy-files.exe"
```

**DWM Integration:**
On Windows 7+, the app integrates with Desktop Window Manager for true Aero glass transparency effects.

**Event Log:**
Service events are written to the Windows Event Log for monitoring.

### Linux

**systemd Service:**
Auto-generated service file for background operation:
```ini
[Unit]
Description=Gillsystems_uneff_your_rigs_messy_files Agent
After=graphical-session.target

[Service]
Type=simple
ExecStart=/opt/gillsystems-uneff/gillsystems-uneff-your-rigs-messy-files --service-mode
Restart=always
RestartSec=5

[Install]
WantedBy=default.target
```

**Install the service:**
```bash
cp gillsystems-uneff.service ~/.config/systemd/user/
systemctl --user enable gillsystems-uneff
systemctl --user start gillsystems-uneff
```

**Supported Distributions:** Ubuntu, Debian, CentOS, Arch Linux  
**Display Servers:** X11 and Wayland  
**Security Modules:** SELinux and AppArmor compatible

### macOS (11+)

**LaunchAgent for Background Service:**
```xml
<!-- ~/Library/LaunchAgents/com.gillsystems.uneff-your-rigs-messy-files.plist -->
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN"
  "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>com.gillsystems.uneff-your-rigs-messy-files</string>
    <key>ProgramArguments</key>
    <array>
        <string>/Applications/Gillsystems_uneff_your_rigs_messy_files.app/Contents/MacOS/gillsystems-uneff-your-rigs-messy-files</string>
        <string>--service-mode</string>
    </array>
    <key>RunAtLoad</key><true/>
</dict>
</plist>
```

**Notes:**
- SIP (System Integrity Protection) compatible
- Code signing and notarization ready
- Full Disk Access permission may be required (grant in System Preferences → Privacy)

---

## 11. Run Modes

The application supports three run modes via command-line flags:

### Full Mode (Default)
```bash
gillsystems-uneff-your-rigs-messy-files
```
Starts the background agent service **and** the GUI simultaneously. The agent launches first, then the GUI connects to it after a 2-second startup delay.

### Service Mode
```bash
gillsystems-uneff-your-rigs-messy-files --service
```
Runs as a headless background service only. No GUI. Ideal for servers, NAS boxes, and headless nodes that should participate in network scanning without a display.

### GUI-Only Mode
```bash
gillsystems-uneff-your-rigs-messy-files --gui-only
```
Launches only the GUI and connects to an already-running service (local or remote). Use this when the service is managed by systemd/Windows Service/LaunchAgent.

### Custom Config Path
```bash
gillsystems-uneff-your-rigs-messy-files -c /path/to/custom-config.toml
```

---

## 12. Way's to Un-EFF! Actions

When duplicates are identified, you have four way's to un-eff - some options available via the gRPC protocol:

| Action | Description | Risk Level |
|--------|-------------|------------|
| **Quarantine** | Move files to a quarantine directory for later review | 🟢 Low |
| **Hard Link** | Replace duplicates with hard links (same data, zero extra space) | 🟡 Medium |
| **Move** | Relocate files to a different location | 🟡 Medium |
| **Delete** | Permanently remove duplicate files | 🔴 High |

### Quarantine
Files are moved to a configurable quarantine path. You can review and restore them later. A grace period (configurable in hours) can be set before permanent removal.

### Hard Link / Dedup (Filesystem-Aware)

The remediation engine is **filesystem-aware** and selects the optimal dedup strategy per filesystem, in priority order:

| Priority | Filesystem | Strategy | Notes |
|----------|-----------|----------|-------|
| 🥇 **1st** | **ZFS** | **Block-level dedup / block cloning** — ZFS native deduplication or reflink block cloning. Zero-copy, copy-on-write aware, checksum-verified. The Commander's main storage pools run ZFS — this is the primary optimization target. | Pools with `dedup=on` or use `cp --reflink` for block cloning |
| 🥈 **2nd** | **NTFS** | **Hard links** via Win32 `CreateHardLink`. Same data, zero extra space. Works on same volume only. | Max 1023 hard links per file on NTFS |
| 🥉 **3rd** | **ext4 / XFS / APFS / Btrfs** | **Hard links** via POSIX `link()`. Standard Unix hard linking. ext4 allows 65,000 links per inode. Btrfs also supports reflinks. | Must be same filesystem |
| 4th | **FAT32 / exFAT** | **No hard link support** — FAT32 does not support hard links. Falls back to **copy-delete** (move the kept file, delete the duplicate) or **quarantine**. | User is warned that FAT32 has no dedup capability |
| 5th | **Network (SMB/NFS)** | **Server-side copy** if supported, otherwise **quarantine**. Hard links across network mounts are not reliable. | Depends on server filesystem |

> **ZFS Note**: On ZFS pools, the engine leverages `copy_file_range()` (Linux 4.5+) or `ioctl FICLONE` for reflink/block cloning. This is a zero-copy operation — the blocks are shared at the pool level with COW semantics. This is the fastest and safest dedup path available.

### Verification Before Delete
The `verify_before_delete` option (configurable) performs a byte-for-byte comparison before any destructive action to guarantee no data loss from hash collisions.

---

## 13. Keyboard Shortcuts & Quick Actions

| Shortcut | Action |
|----------|--------|
| **Ctrl+A** | Select all duplicates |
| **Ctrl+C** | Copy selected files |
| **Ctrl+X** | Cut selected files |
| **Ctrl+V** | Paste files |
| **Delete** | Delete selected (with warning) |
| **F5** | Refresh view |
| **Ctrl+S** | Save scan results |

---

## 14. Troubleshooting

### Application Won't Start

| Symptom | Solution |
|---------|----------|
| "Failed to read config file" | Ensure `config.toml` is valid TOML syntax |
| "Database path cannot be empty" | Check `[database].path` in config |
| "TLS certificate file not found" | Verify cert/key paths or disable TLS |
| Blank screen on Linux | Install `libgtk-3-dev` and ensure X11/Wayland is running |
| "Conflicting arguments" | Don't use `--service` and `--gui-only` together |

### Scan Issues

| Symptom | Solution |
|---------|----------|
| Slow scanning | Increase `thread_pool_size` in config |
| Files skipped | Check `max_file_size_gb` limit and exclude patterns |
| "Permission denied" errors | Run with admin/sudo privileges |
| High memory usage | Reduce `hash_batch_size` or `cache_size_mb` |

### Network Issues

| Symptom | Solution |
|---------|----------|
| Nodes not discovered | Check firewall rules for gRPC port (default: 50051) |
| Sync failures | Verify TLS certificates if `client_auth_required = true` |
| High latency | Ensure nodes are on the same LAN segment |

### GUI Issues

| Symptom | Solution |
|---------|----------|
| No Aero effects | GPU may not support OpenGL 3.3+ — CPU fallback is used |
| Low FPS | Reduce animation settings or close other GPU-heavy apps |
| Window not rendering | Update GPU drivers |

---

## 15. FAQ

**Q: Do I need admin/root privileges?**  
A: Yes. Full admin, full speed — that's the GillSystems way. The application REQUIRES admin/sudo access and will request elevation on launch. No permission denied errors, no restricted directories, no throttling. You own your system — this tool respects that. Systems should serve humans, not gatekeep them.

**Q: Is there a web interface?**  
A: No. And there never will be. The GUI is a native desktop application built with egui/eframe. No browser, no Electron, no web server. Built with zero frameworks, maximum intent. Pure native performance.

**Q: Can I run this on a headless server?**  
A: Yes. Use `--service` mode. It runs as a background daemon and communicates with GUI instances on other machines via gRPC.

**Q: How are duplicates detected?**  
A: Files are first grouped by size. Size-matched files are then hashed with xxHash64 (fast pass), and confirmed with SHA-256 (cryptographic verification). This pipeline avoids unnecessary hashing of unique files.

**Q: Is my data safe?**  
A: The application never deletes anything silently. All destructive actions require explicit confirmation with clear warning dialogs. Quarantine is recommended over deletion.

**Q: Does it support network shares (SMB/NFS)?**  
A: Yes. Any mounted filesystem is scannable — internal, external, network, ZFS pools, RAID arrays.

**Q: What about symlinks?**  
A: Symlinks are detected and tracked but not followed by default (`follow_links = false`). This prevents infinite loops and duplicate counting.

**Q: Can I customize the Aero theme colors?**  
A: Yes. The theme system supports Windows 7 Blue, Silver, and Olive Green presets, plus a custom color picker (coming in v0.2).

**Q: How much space does the database use?**  
A: The SQLite cache is minimal — typically <100 MB even for scans of millions of files. WAL mode is enabled by default for optimal write performance.

---

## 16. Philosophy & Mission

### GillSystems — Systems Should Serve Humans

> *"Technology should empower people, not control them. Open source and decentralization are essential for human freedom."*  
> — Commander Stephen Gill, GillSystems • 30+ years of technology expertise

Commander Stephen Gill — known as Commander Awesomeness Gill — built GillSystems on a simple conviction: **your technology should make you money, not cost you money.** With 30+ years across distributed compute, agent orchestration, local AI, storage engineering, ZFS, Windows and Linux integration, networking, scripting, automation, and real-world infrastructure — this isn't theory. This is battle-tested engineering.

**Gillsystems_uneff_your_rigs_messy_files** is built on these principles:

1. **Power to the People** — No artificial limitations. No vendor lock-in. No features locked behind paywalls or permission models. Intelligence, autonomy, and compute belong in YOUR hands.

2. **Systems Should Serve Humans** — The tool serves you, not the other way around. Your technology should make you money, not cost you money. If it doesn't empower you, it has no business running on your rig.

3. **Radical Transparency** — The entire development process is open. Every line of code, every design decision, every trade-off is visible and documented. Open source and decentralization are essential for human freedom.

4. **Anti-Vendor BS** — No unnecessary complexity. No bloated frameworks. No cloud dependencies. Built with **zero frameworks, maximum intent**. A single native Rust binary that does exactly what it says. We don't do vendor BS here.

5. **Sovereignty** — Your data stays on your machines. Peer-to-peer architecture means no central authority. No telemetry. No phone-home. No third-party dependencies at runtime. Fully local, fully sovereign.

6. **Knowledge Liberation** — Knowledge should be free. Configuration should be transparent. Every setting, every scan result, every hash — visible to you, owned by you.

### 7D Methodology

This project follows the **7D Agile** development methodology aligned with **SWEET** principles:

| Phase | Description |
|-------|-------------|
| **Define** | Clear scope, lean requirements, no bloated specs |
| **Design** | Simple architectures, tested with user feedback |
| **Develop** | Clean code, immediate value, self-organizing teams |
| **Debug** | Testing and validation across all platforms |
| **Document** | This guide and inline code documentation |
| **Deliver** | Production-ready single binary |
| **Deploy** | Platform-native service integration |

---

## 17. Glossary

| Term | Definition |
|------|------------|
| **Aero** | Windows 7's visual design language featuring glass transparency effects |
| **egui** | Immediate-mode GUI library for Rust |
| **eframe** | Cross-platform application framework built on egui |
| **gRPC** | High-performance RPC framework used for node-to-node communication |
| **mTLS** | Mutual TLS — both client and server authenticate each other |
| **xxHash64** | Extremely fast non-cryptographic hash function used for duplicate pre-filtering |
| **SHA-256** | Cryptographic hash function used for final duplicate verification |
| **SQLite** | Embedded relational database used for local scan caching |
| **WAL** | Write-Ahead Logging — SQLite mode that improves concurrent write performance |
| **Hard Link** | Filesystem feature where multiple paths point to the same data blocks (NTFS, ext4, XFS, APFS) |
| **ZFS Block Cloning** | Zero-copy dedup via copy-on-write reflinks — the primary dedup target for GillSystems storage pools |
| **Reflink** | A copy-on-write file copy where blocks are shared until modified (ZFS, Btrfs, XFS) |
| **FICLONE** | Linux ioctl for block-level file cloning — used for ZFS/Btrfs dedup |
| **Quarantine** | Moving files to a safe location for review before permanent deletion |
| **Remediation** | The process of dealing with identified duplicate files |
| **Node** | A single machine running the Gillsystems_uneff_your_rigs_messy_files agent |
| **Peer-to-Peer** | Decentralized architecture where all nodes are equal |
| **systemd** | Linux service manager for background process management |
| **LaunchAgent** | macOS mechanism for running background services on user login |
| **DWM** | Desktop Window Manager — Windows component that enables Aero effects |
| **7D** | GillSystems agile methodology: Define, Design, Develop, Debug, Document, Deliver, Deploy |
| **SWEET** | Simplicity, Workable, Empirical, Empowering, Transparent |

---

## Support & Links

- **GitHub**: [https://github.com/OCNGill/Gillsystems_uneff_your_rigs_messy_files](https://github.com/OCNGill/Gillsystems_uneff_your_rigs_messy_files)
- **Website**: [https://gillsystems.net](https://gillsystems.net)
- **Documentation**: [https://github.com/OCNGill/Gillsystems_uneff_your_rigs_messy_files/tree/main/docs](https://github.com/OCNGill/Gillsystems_uneff_your_rigs_messy_files/tree/main/docs)

---

*Built with ❤️ by Commander Stephen Gill — GillSystems • 30+ Years of Technology Expertise*  
*Systems Should Serve Humans — Power to the People!* 🚀

*Document Version: 0.1.0 | Last Updated: February 2026 | 7D Phase: Document*
