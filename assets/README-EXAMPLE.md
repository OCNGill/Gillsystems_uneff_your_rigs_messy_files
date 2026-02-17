# **The-Commander: Distributed AI Orchestration System**

**Version:** v1.5.10 (Forensic Audit Remediation)
**Last Updated:** February 13, 2026  
**Status:** ✅ v1.5.10 Deployed: Forensic Stats & Storage Stabilization Complete
**Next Release:** 1.6.0 (Unified Node Intelligence)

This release stabilizes the core telemetry and storage subsystems after a 25-pass forensic audit.
Key fixes include autonomous stats merging, chat metadata restoration, and Port 445 SMB probing.

## [1.5.0] - 2026-01-31 (The Commander Has Arrived)

<div align="center">
    <img src="assets/The_Commander_has_arrived.png" alt="The Commander Has Arrived" width="1000" />
</div>

### Major Release: It Works!
- The Commander system is now fully operational across all nodes.
- Permissions require further refinement, but the core functionality is stable.

#### New Features
- Added support for distributed orchestration with seamless multi-node ignition.
- Enhanced GUI with real-time status updates for all nodes.

#### Evidence of Success
- Screenshot added to README: `assets/The_Commander_has_arrived.png`.
- YouTube video announcement coming soon.

#### Known Issues
- Permissions need further tweaking for optimal security.

---

## Runtime Model Detection & Engine Management (v1.5.3 Hotfix)

This patch adds significant improvements to how the system reports and manages running inference engines. Operators should read the notes below and the full operational guidance in `docs/OPERATIONAL_NOTES_1_5_3.md`.

- GUI now displays the ACTUAL running model and live statistics (context usage, tokens/sec) by querying each node's running inference server rather than using only the `relay.yaml` config.
- `Save & Re-Ignite` has been hardened: the system now verifies tracked PIDs, kills stale PIDs and any process bound to the engine port, and then launches a fresh engine instance so ghost CMD windows no longer block re-launches.
- Engine dial changes (model_file, ctx, ngl, fa, extra_flags) made via the Node Control Panel are persisted to the target node's `relay.yaml` and the engine is restarted to apply them.
- New health and shutdown behaviors prevent configuration drift and reduce false-positive reports that an engine is running when it is not.

See `docs/status/OPERATIONAL_NOTES_LATEST.md` for full API details, GUI behavior, command examples, and a testing checklist.

## 🆕 Launch Scripts & Storage Controls (v1.4.12)

The Commander now supports fully config-driven engine startup across all nodes in the cluster.

### **New Port Scheme**
| Node | IP | API Port | Engine Port |
|------|-----|----------|-------------|
| Main | 10.0.0.164 | 9000 | 8000 |
| HTPC | 10.0.0.42 | 9001 | 8001 |
| Laptop | 10.0.0.93 | 9002 | 8002 |
| Steam-Deck | 10.0.0.139 | 9003 | 8003 |

### **Startup Configuration**
Startup is managed via `config/relay.yaml`.

Engine launch is **100% config-driven** — the system reads `binary`, `model_file`, `ctx`, `ngl`, `fa`,
and `bind_host` from each node’s engine section in `relay.yaml` and constructs the CLI command directly.
No external launch scripts are used.

### **Manual Startup Procedure**
1. Start HTPC first (has relay/storage roles)
2. Start Main node
3. Start Steam-Deck
4. Start Laptop (or whichever node you're working from)
5. Open GUI at `http://localhost:5173` (or node's IP:5173)
6. Click "IGNITE ALL" or engines should already be running

### **Engine Configuration in relay.yaml**
Each node has complete engine settings:
```yaml
engine:
  binary: go.exe          # or 'go' on Linux
  model_file: model.gguf  # Model filename
  ctx: 30000              # Context window
  ngl: 999                # GPU layers
  fa: true                # Flash attention
  bind_host: 10.0.0.164   # Network binding
  extra_flags: ""          # Additional CLI flags (optional)
```

---

## 🆕 Filesystem Explorer & Storage Architecture (v1.4.7)

The Commander now includes a comprehensive filesystem explorer for browsing and selecting files on any node in the cluster - both local and remote.

### **Features**
- **Remote File Browsing**: Browse directories on any node from any GUI
- **Drive Enumeration**: Automatically detect drives (Windows) or root (Linux)
- **Storage Path Selection**: Select storage folders via file explorer instead of hardcoding
- **Binary Discovery**: Browse and select engine binaries (.exe, .bat, .sh) from any location
- **Model Selection**: Navigate model folders and select .gguf files visually
- **Transparent Proxying**: Remote node requests seamlessly proxy through the cluster

### **Storage Architecture: Local-First, Network-Synced**
Commander OS implements a "Local-First" storage pattern where:
- Each node operates on **local NVMe/SSD** for maximum performance
- Critical events (node offline) sync **immediately** to HTPC/ZFS
- Regular events batch sync every **60 seconds** to reduce network overhead
- **ZFS pool on HTPC** is the authoritative storage (end authority)

### **Filesystem API Endpoints**
```
GET /nodes/{node_id}/filesystem/drives
    → Returns list of drives (C:\, D:\, etc.) or root (/)

GET /nodes/{node_id}/filesystem/list?path=...
    → Returns directory contents with file/folder metadata
```

### **Relay Server Auto-Management**
- Relay server **automatically restarts** when reigniting nodes with `relay` or `storage` roles
- Storage location is **broadcast to all nodes** when relay starts
- Nodes can query relay config via `GET /relay/config`

### **Configuration Simplification**
- **No more hardcoded paths**: Binary paths removed from `relay.yaml`
- **User-selected paths**: All file paths now chosen via GUI file explorer
- **Dynamic discovery**: Engines, models, and storage paths discovered at runtime

---

## 🆕 Commander Chat Interface (v1.4.3)

The Commander Chat provides direct interaction with The Commander Avatar - your AI strategic advisor powered by 100% local inference.

### **Features**
- **Conversation Management**: Create multiple chat threads with persistent history
- **Real-time Streaming**: See responses as they generate via WebSocket
- **Auto Node Selection**: Commander routes to highest-performance node automatically
- **Intent Classification**: Understands commands, queries, and conversations
- **Decision Engine**: Trust boundaries prevent unauthorized operations
- **Cross-Node Access**: Access any node's GUI from any machine in the cluster

### **Chat Panel Layout**
- **Left Sidebar**: Conversation list + "New Chat" button
- **Center**: Message stream with user/assistant bubbles
- **Bottom Bar**: Node stats (commanding node, model, context, TPS)
- **Input**: Command line with send button

### **How Chat Routing Works**
1. You type a message in the chat input
2. Message goes to Commander Avatar on the hub
3. Avatar classifies intent (QUERY, COMMAND, CHAT, ESCALATION)
4. Decision Engine checks trust boundaries
5. LlamaClient routes inference to **highest TPS node** (not the selected node in left panel)
6. Response streams back to GUI via WebSocket

**Note**: The left panel "Selected Node" is for **configuration only**. Chat always routes through the highest-performance available node.

### **Cross-Node Browser Access**
You can access any node's GUI from any browser on the LAN:
- From Laptop → open `http://gillsystems-main:9000` in browser
- From Main → open `http://gillsystems-htpc:9001` in browser
- GUI automatically uses the same host/port for API calls

---

## 🆕 Unified Launcher System (v1.4.1)

Both Windows and Linux now use **unified clear-launch launchers** by default. These automatically clear stale Commander processes and ports before startup, eliminating "Address already in use" errors.

**Primary Launchers:**
- **Windows**: `The_Commander.bat`
- **Linux/macOS**: `./the_commander.sh`

These now include:
- ✅ **Automatic port clearance** - kills zombie Commander processes
- ✅ **Network preflight validation** - verifies all nodes are reachable
- ✅ **Auto-hosts setup** - configures `/etc/hosts` or Windows hosts file with node mappings
- ✅ **Dynamic versioning** - pulls version from centralized `commander_os.__version__.py`
- ✅ **GUI/HUD sync** - ensures frontend and backend launch in correct exact order

**Legacy launchers** (without auto-clear) are archived in `docs/archive/` for reference.

### **Quick Start**

```bash
# Windows
The_Commander.bat

# Linux/macOS
./the_commander.sh
```

## 🆕 Network Preflight System (v1.4.1)

Commander OS includes a **self-healing network identity layer** that automatically resolves and corrects node addresses at startup. This eliminates the "Hub is unreachable" error caused by DHCP IP address changes.

### **How It Works**

1. **Hostname-based configuration**: `config/relay.yaml` uses hostnames (e.g., `gillsystems-main`) instead of hard-coded IPs
2. **Preflight smoke test**: Before starting the hub, the system resolves all hostnames and verifies port reachability
3. **LAN discovery fallback**: If hostname resolution fails, the system scans the local subnet to find the node by its `/identity` endpoint
4. **Runtime caching**: Resolved addresses are cached for fast subsequent lookups

### **Setup: Hosts File Configuration (Optional)**

Preflight automatically detects and adds missing hosts entries. Manual setup is optional:

**Windows** (auto-setup when running as admin):
- Launcher automatically detects and adds missing entries to hosts file

**Linux/macOS** (auto-setup with sudo):
```bash
sudo ./the_commander.sh
```

**Manual Edit** (if preferred):
```
# Windows: C:\Windows\System32\drivers\etc\hosts
# Linux/macOS: /etc/hosts

# Gillsystems Commander OS Nodes
10.0.0.164    gillsystems-main
10.0.0.42     gillsystems-htpc
10.0.0.93     gillsystems-laptop
10.0.0.139    gillsystems-steam-deck
```

### **Preflight CLI Options**

Available when running the backend manually via `commander_os.main`:

```bash
# Run with full preflight (default)
python -m commander_os.main commander-gui-dashboard
```

### **Troubleshooting Network Issues**

1. **"Hub is unreachable"**: Ensure nodes are powered on or check hostname resolution:
   ```bash
   ping gillsystems-main
   ```

2. **Preflight fails**: Check that target nodes are running Commander OS

3. **Discovery finds wrong node**: Ensure each node has the `/identity` endpoint (included in v1.4.0+)

4. **Port already in use**: Launchers automatically clear stale processes. Run again if needed.

4. **Best Practice**: Set up DHCP reservations on your router to give each machine a stable IP address

---

## **Maintenance & Troubleshooting**

### **Clearing Ghost Processes**
If you encounter `[Errno 98] Address already in use`, it means a previous instance of the Commander is still hanging in the background. Use the following tactical clearance scripts to reset your node's ports:

*   **Linux / Steam Deck**:
    ```bash
    ./kill_all_active_port_stealers_for_your_node.sh
    ```
*   **Windows**:
    ```batch
    kill_all_active_port_stealers_for_your_node.bat
    ```

---

## **System Requirements**

*   **OS**: Windows 10/11 or Linux (Ubuntu 20.04+)
*   **Python**: **v3.10 ONLY** (Strict Requirement)
    *   *Warning: Python 3.14+ will cause dependency failures*
*   **Node.js**: v20+ (LTS Required for Vite compatibility)
*   **GPU**: AMD Radeon 7000 Series (Optional, for Local LLM Acceleration)

---

## **Automated Setup (Recommended for Linux)**

For automated installation of all dependencies (Python 3.10, Node.js 20+, npm, build tools):

```bash
cd The-Commander-Agent
./scripts/linux_prereqs.sh
```

This script will:
- Install Node.js 20+ (required for Vite frontend)
- Ensure Python 3.10 is available (via pyenv if needed)
- Create virtual environment and install Python dependencies
- Validate all prerequisites before completion

---

## **Quick Start (Single Node)**

1.  **Clone Repository**:
    ```bash
    git clone https://github.com/OCNGill/The-Commander-Agent.git
    cd The-Commander-Agent
    ```

2.  **Install Dependencies (run the platform pre-req first)**:
    ```bash
    # Linux (recommended)
    ./scripts/linux_prereqs.sh

    # Windows (run as Administrator)
    # From Explorer: Right-click `install_prereqs.bat` and choose "Run as administrator"
    # Or in an elevated PowerShell prompt:
    .\install_prereqs.bat
    ```

    After the prereq installer finishes, continue with the launcher step below to start the Commander OS.

3.  **Launch System**:
    ```bash
    # Windows
    The_Commander.bat
    
    # Linux
    ./the_commander.sh
    ```

4.  **Access Dashboard**: Open browser to `http://localhost:5173`

---

## **Multi-Node Deployment (Full Functionality)**

For complete model discovery and distributed orchestration across all nodes:

1.  **Clone Repository on Each Node**:
    ```bash
    # On each physical machine (Main, HTPC, Steam-Deck, Laptop)
    git clone https://github.com/OCNGill/The-Commander-Agent.git
    cd The-Commander-Agent
    ```

2.  **Install Dependencies on Each Node**:
    ```bash
    # Automated (Linux) - Recommended
    ./scripts/linux_prereqs.sh
    
    # Manual Windows
    py -3.10 -m pip install -r requirements.txt
    
    # Manual Linux
    python3.10 -m pip install -r requirements.txt
    ```

3.  **Verify Node Configuration**:
    - Check `config/relay.yaml` for correct IP addresses and ports
    - Ensure `model_root_path` points to each node's model directory

4.  **Launch on Each Node**:
    ```bash
    # Each node automatically detects its identity based on port
    The_Commander.bat  # Windows
    ./the_commander.sh  # Linux
    ```

5.  **Verify Network Connectivity**:
    - Each node's API should be accessible at `http://<node-ip>:<port>`
    - Test from any node: `curl http://10.0.0.164:9000/nodes`

**Why Multi-Node Deployment?**
- **Model Discovery**: Each node scans its own filesystem and reports available models
- **Distributed Inference**: Chat requests route to highest-ranking available node
- **Load Balancing**: System automatically distributes work across active nodes
- **Fault Tolerance**: If one node fails, others continue operating

---

## **Architectural Topology**
| Node ID | Physical Host | Hardware | Bench (t/s) | Model Configuration |
| :--- | :--- | :--- | :--- | :--- |
| **Gillsystems-Main** | Gillsystems-Main | Radeon 7900XTX | **130** | Qwen3-Coder-25B (131k ctx, 999 NGL) |
| **Gillsystems-HTPC** | Gillsystems-HTPC | Radeon 7600 | **60** | Granite-4.0-h-tiny (114k ctx, 40 NGL) |
| **Gillsystems-Steam-Deck**| Gillsystems-Steam-Deck | Custom APU | **30** | Granite-4.0-h-tiny (21k ctx, 32 NGL) |
| **Gillsystems-Laptop**| Gillsystems-Laptop| Integrated | **9** | Granite-4.0-h-tiny (21k ctx, 999 NGL) |

---

## **Changelog**

### Version 1.4.3 (January 16, 2026 - Phase 8.3)
- âœ… **Chat Interface Overhaul**: Persistent chat history with multisession conversation management.
- âœ… **Secure Conversation Storage**: SQLite on ZFS for all historical chat context.
- âœ… **Frontend Sidebar**: New "Conversations" panel for instant switching between strategic threads.
- âœ… **Context Awareness**: Optimized message handling with conversation-specific routing.

### Version 1.4.1 (Phase 8.1)
- Implementing Commander brain logic (Avatar + Cyberbot)
- Decision Engine with trust boundaries and escalation rules
- Working chat interface in Strategic Dashboard GUI
- Local llama.cpp integration for natural language processing
- Path 4 (Hybrid Local Architecture) selected after LLM consultation

### Version 1.3
- Implemented the new storage framework for Commander OS.
- Added agent-specific storage modules for Commander and Recruiter agents.
- Updated documentation to reflect the new storage architecture.

---

## Documentation Links

- [Complete Documentation Index](DOCUMENTATION_INDEX.md)

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

## **🎉 The Commander Has Arrived (v1.5.0)**

The Commander system is now fully operational across all nodes. Permissions require further refinement, but the core functionality is stable.

![The Commander Has Arrived](assets/The_Commander_has_arrived.png)

---
---
