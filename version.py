"""
Gillsystems_uneff_your_rigs_messy_files — Version Authority
============================================================
This file is the SINGLE SOURCE OF TRUTH for project versioning.

All version references in Cargo.toml, manifest.json, docs, and build
artifacts MUST be synchronized with this file.

Versioning follows the 7D Agile lifecycle:
  - MAJOR: Breaking architecture changes (new 7D cycle)
  - MINOR: Phase completion milestone (Discover → Define → Design → Develop → etc.)
  - PATCH: Incremental improvements within a phase

Phase tags indicate current 7D position:
  0.1.x = Define + Design complete
  0.2.x = Develop (core implementation)
  0.3.x = Debug (testing & validation)
  0.4.x = Document (API docs, inline docs)
  0.5.x = Deliver (artifacts prepared)
  0.6.x = Deploy (production release)
  1.0.0 = Full 7D cycle complete — production ready

SWEET Alignment:
  - Simplicity: One file, one truth.
  - Workable: Importable by any build script or CI pipeline.
  - Empirical: Version tracks real milestones, not wishful thinking.
  - Empowering: Team can check phase status at a glance.
  - Transparent: No hidden version strings buried in configs.

Usage:
  from version import VERSION, VERSION_TUPLE, PHASE
  print(f"v{VERSION} — Phase: {PHASE}")

Commander: Stephen Gill — GillSystems
Repository: https://github.com/OCNGill/Gillsystems_uneff_your_rigs_messy_files
"""

# ──────────────────────────────────────────────
# VERSION AUTHORITY — edit HERE, sync everywhere
# ──────────────────────────────────────────────

VERSION_MAJOR = 0
VERSION_MINOR = 2
VERSION_PATCH = 0

VERSION_TUPLE = (VERSION_MAJOR, VERSION_MINOR, VERSION_PATCH)
VERSION = f"{VERSION_MAJOR}.{VERSION_MINOR}.{VERSION_PATCH}"

# 7D Phase Tracking
PHASE = "Develop Complete → Debug Ready"
PHASE_CODE = "DEVELOP_COMPLETE"

# Phase history (append-only log)
PHASE_HISTORY = [
    {"version": "0.0.1", "phase": "Discover", "milestone": "Project inception, codebase reconnaissance"},
    {"version": "0.0.2", "phase": "Define", "milestone": "Team structure (3×3), scope, agent assignments"},
    {"version": "0.1.0", "phase": "Design", "milestone": "Architecture complete, ZFS-first remediation, branding purge, 6 module stubs, user guide, 3 reflection loops passed"},
    {"version": "0.2.0", "phase": "Develop", "milestone": "Full implementation — 10 modules, 0 TODOs, all subsystems operational: scanner, hashing, database CRUD, remediation (ZFS/NTFS/POSIX/FAT32), GUI wired, gRPC service, duplicate detection pipeline"},
]

# Build metadata
BUILD_METADATA = {
    "language": "Rust",
    "edition": "2021",
    "gui": "egui/eframe 0.24",
    "async": "tokio 1.35",
    "database": "SQLite (rusqlite 0.29)",
    "hashing": "xxHash64 + SHA-256",
    "network": "gRPC (tonic 0.10)",
    "storage_priority": "ZFS → NTFS → ext4/XFS → FAT32",
}

# Team structure
TEAM_STRUCTURE = {
    "commander": "Stephen Gill",
    "team_leaders": 3,
    "agents_per_team": 3,
    "total_agents": 9,
    "teams": [
        "Alpha (UI/UX Design)",
        "Beta (Systems Architecture)",
        "Gamma (Platform Integration)",
    ],
}


def get_version_string():
    """Return formatted version string for display."""
    return f"v{VERSION}"


def get_full_version_info():
    """Return complete version + phase info for logs and About dialogs."""
    return f"Gillsystems_uneff_your_rigs_messy_files v{VERSION} | Phase: {PHASE}"


def get_cargo_version():
    """Return version string formatted for Cargo.toml synchronization."""
    return VERSION


if __name__ == "__main__":
    print(get_full_version_info())
    print(f"  Major: {VERSION_MAJOR}")
    print(f"  Minor: {VERSION_MINOR}")
    print(f"  Patch: {VERSION_PATCH}")
    print(f"  Phase: {PHASE} ({PHASE_CODE})")
    print(f"  Team:  {TEAM_STRUCTURE['total_agents']} agents across {TEAM_STRUCTURE['team_leaders']} teams")
    print(f"  Storage Priority: {BUILD_METADATA['storage_priority']}")
    print()
    print("Phase History:")
    for entry in PHASE_HISTORY:
        print(f"  v{entry['version']} — {entry['phase']}: {entry['milestone']}")
