# Gillsystems_uneff_your_rigs_messy_files - Multi-Agent System

**Power to the People!** 🚀

A cross-platform duplicate file detection system with **Windows 7 Aero style** interface, built on the philosophy of user empowerment and radical transparency.

## 🎯 Mission

> *"Systems Should Serve Humans — not the other way around."*  
> — Commander Stephen Gill, GillSystems • 30+ years of technology expertise

Bring back the **banging Windows 7 Aero style** while delivering unmatched cross-platform duplicate file management. Built with **zero frameworks, maximum intent**. Your technology should make you money, not cost you money — and this tool embodies that belief in every line of code.

This is **Gillsystems-style** development — no guardrails, admin privileges assumed, power to the people, and zero vendor BS.

## 🏗️ Architecture

### Single Binary Design
- **Rust executable** with embedded GUI (egui/eframe)
- **No web-based bullshit** - pure native interface
- **Peer-to-peer network** - no central orchestrator
- **10+ node support** with multi-drive detection

### Windows 7 Aero Interface
- **Glass effects** with transparency and blur
- **Classic color schemes** (Blue, Silver, Olive Green)
- **Smooth animations** at 60 FPS
- **Dual panel view** for duplicate comparison

### 🔓 Full Admin / Full Speed Architecture
- **Admin/sudo assumed on every node** — no permission prompts, no UAC friction
- **Auto-elevation on launch** — requests admin rights immediately on Windows, expects root/sudo on Linux/macOS
- **Unrestricted filesystem access** — scans every drive, every directory, every hidden file
- **No sandboxing** — direct kernel/OS integration for maximum I/O throughput
- **Full hardware utilization** — all CPU cores, GPU acceleration, maximum thread pool
- **Zero artificial limits** — no file count caps, no scan throttling, no "are you sure?" gatekeeping

### Cross-Platform Support
- **Windows**: 7-11 with DWM integration, auto-admin elevation
- **Linux**: systemd services, multiple distros, root-level access
- **macOS**: LaunchAgents, code signing, Full Disk Access granted

## 👥 Multi-Agent Team Structure

### Team Alpha: UI/UX Design Division
- **Lead**: Agent Alpha-1 (Aero Glass Rendering Specialist)
- **Squad**: 3 specialized UI agents
- **Domain**: Windows 7 Aero interface & cross-platform UX

### Team Beta: Systems Architecture Division
- **Lead**: Agent Beta-1 (GUI Architecture Optimizer)
- **Squad**: 3 specialized systems agents
- **Domain**: Core architecture & performance optimization

### Team Gamma: Platform Integration Division
- **Lead**: Agent Gamma-1 (Windows Integration Expert)
- **Squad**: 3 specialized integration agents
- **Domain**: Platform integration & service management

## 📋 Project Status

**Current Phase**: Design (Day 1 of 28)

### 7D Progress
- [x] **Define**: Project scope and team structure
- [ ] **Design**: Architecture and specifications
- [ ] **Develop**: Core implementation
- [ ] **Debug**: Testing and validation
- [ ] **Document**: User guides and API docs
- [ ] **Deliver**: Production-ready artifacts
- [ ] **Deploy**: Final system integration

### Active Milestones
- **M1**: Design Phase Complete (Day 7)
- **M2**: Development Phase (Day 14)
- **M3**: Integration Phase (Day 21)
- **M4**: Delivery Phase (Day 28)

## 🚀 Quick Start

### Prerequisites
- **Admin/Sudo privileges** (assumed)
- **Rust 1.70+** for development
- **Cross-platform build tools**

### Installation
```bash
# Clone the repository
git clone https://github.com/OCNGill/Gillsystems_uneff_your_rigs_messy_files.git
cd Gillsystems_uneff_your_rigs_messy_files

# Build the single binary
cargo build --release

# Run with GUI and service
./target/release/gillsystems-uneff-your-rigs-messy-files

# Service mode only
./target/release/gillsystems-uneff-your-rigs-messy-files --service

# GUI only (connect to existing service)
./target/release/gillsystems-uneff-your-rigs-messy-files --gui-only
```

### First Launch
1. **Launch the GUI** - Windows 7 Aero interface appears
2. **Network Discovery** - Automatically finds other nodes
3. **Initial Scan** - Select drives and start scanning
4. **Wait for completion** - First scan may take 30+ minutes

## 🎨 Windows 7 Aero Features

### Glass Effects
- **Translucent windows** with blur effects
- **Custom title bars** with glass styling
- **Smooth shadows** and rounded corners
- **GPU acceleration** where available

### Color Schemes
- **Windows 7 Blue** (classic)
- **Windows 7 Silver** (professional)
- **Windows 7 Olive Green** (legacy)
- **Custom themes** with color picker

### Animations
- **Window transitions** with smooth effects
- **Hover animations** on buttons and controls
- **Loading animations** with glass shimmer
- **60 FPS target** on all platforms

## 🌐 Network Architecture

### Peer-to-Peer Design
```
Node A (Main Desktop)    Node B (Laptop)    Node C (HTC)
├── C:, D:, E: drives    ├── C:, USB:      ├── ZFS pools
├── Windows 7 Aero GUI   ├── Aero GUI       ├── Aero GUI
└── gRPC/mTLS links     └── gRPC/mTLS     └── gRPC/mTLS
```

### Multi-Drive Support
- **Internal**: SATA, NVMe, M.2 drives
- **External**: USB 3.0, USB-C, Thunderbolt
- **Network**: SMB/CIFS shares, NFS mounts
- **Special**: ZFS pools, RAID arrays, LVM volumes

## ⚡ Performance Targets

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

## 🛠️ Development

### Build Requirements
```toml
[dependencies]
eframe = "0.24"           # GUI framework
tokio = "1.35"             # Async runtime
tonic = "0.10"             # gRPC networking
sha2 = "0.10"              # Hashing
walkdir = "2.4"             # File system
```

### Project Structure
```
Gillsystems_uneff_your_rigs_messy_files/
├── src/                    # Source code
├── docs/                   # Documentation
├── scripts/                 # Build and deployment scripts
├── assets/                  # Icons, themes, resources
├── manifest.json            # Project manifest
├── task_ledger.md          # 7D task tracking
└── README.md               # This file
```

### Team Coordination
- **Daily standups** at 0900, 1000, 1100, 1600
- **Cross-team integration** working groups
- **Peer review** required for all changes
- **Performance benchmarks** for all components

## 📚 Documentation

- **[User Guide](docs/user_guide.md)** - Complete usage instructions
- **[Architecture Design](architecture_design.md)** - System architecture
- **[Design Team Analysis](design_team_analysis.md)** - Design team breakdown
- **[Team Leader Directives](team_leader_directives.md)** - Leadership instructions
- **[Task Ledger](task_ledger.md)** - 7D progress tracking

## 🤝 Contributing

### GillSystems Philosophy

> *"Technology should empower people, not control them. Open source and decentralization are essential for human freedom."*

- **Power to the People** — No artificial limitations. No vendor lock-in. No features locked behind paywalls.
- **Radical Transparency** — Open development. Every line of code, every design decision, every trade-off is visible.
- **User Empowerment** — The tool serves you, not the other way around. Intelligence, autonomy, and compute belong in YOUR hands.
- **Anti-Vendor BS** — We don't do vendor BS here. No unnecessary complexity. No bloated frameworks. No cloud dependencies. Built with zero frameworks, maximum intent.
- **Sovereignty** — Your data stays on your machines. No telemetry. No phone-home. No third-party dependencies at runtime.

### Development Standards
- **7D Methodology** — Define → Design → Develop → Debug → Document → Deliver → Deploy
- **SWEET Principles** — Simplicity, Workable, Empirical, Empowering, Transparent
- **Single Binary** — No dependencies, no installers, easy deployment
- **Performance First** — Rust, LTO, stripped binaries, all CPU cores, maximum I/O

### 3×3 Agent Team
| Team | Lead | Domain |
|------|------|--------|
| **Alpha** | Aero Glass Rendering Specialist | UI/UX, Theming, Animations, DWM |
| **Beta** | GUI Architecture Optimizer | Core Architecture, Performance |
| **Gamma** | Windows Integration Expert | Platform Integration, Service Mgmt |

## 📄 License

MIT License - See [LICENSE](LICENSE) file for details.

## 🌐 Links

- **GitHub Repository**: https://github.com/OCNGill/Gillsystems_uneff_your_rigs_messy_files
- **GillSystems Website**: https://gillsystems.net
- **Documentation**: [docs/user_guide.md](docs/user_guide.md)
- **Community**: [GitHub Discussions](https://github.com/OCNGill/Gillsystems_uneff_your_rigs_messy_files/discussions)

---

**Remember**: This is more than just a duplicate file finder — it's a statement about **user freedom** and **technological sovereignty**. Built by Commander Stephen Gill with 30+ years of expertise, zero frameworks, and maximum intent. We're bringing back the **banging Windows 7 Aero style** while delivering cutting-edge cross-platform functionality. Systems should serve humans — and this one does.

**Power to the People!** 🚀

---

## 💖 Support / Donate

If you find this project helpful, you can support ongoing work — thank you!

<p align="center">
	<img src="assets/qr-paypal.png" alt="PayPal QR code" width="180" style="margin:8px;">
	<img src="assets/qr-venmo.png" alt="Venmo QR code" width="180" style="margin:8px;">
</p>


**Donate:**

- [![PayPal](https://img.shields.io/badge/PayPal-Donate-009cde?logo=paypal&logoColor=white)](https://paypal.me/gillsystems) https://paypal.me/gillsystems
- [![Venmo](https://img.shields.io/badge/Venmo-Donate-3d95ce?logo=venmo&logoColor=white)](https://venmo.com/Stephen-Gill-007) https://venmo.com/Stephen-Gill-007

---


<p align="center">
	<img src="assets/Gillsystems_logo_with_donation_qrcodes.png" alt="Gillsystems logo with QR codes and icons" width="800">
</p>

<p align="center">
	<a href="https://paypal.me/gillsystems"><img src="assets/paypal_icon.png" alt="PayPal" width="32" style="vertical-align:middle;"></a>
	<a href="https://venmo.com/Stephen-Gill-007"><img src="assets/venmo_icon.png" alt="Venmo" width="32" style="vertical-align:middle;"></a>
</p>

---

*Version: 0.1.0 — Design Complete → Develop Ready*  
*Version Authority: [`version.py`](version.py) — Single Source of Truth*  
*Tagged: v0.1.0 — Discover + Define + Design milestone*
