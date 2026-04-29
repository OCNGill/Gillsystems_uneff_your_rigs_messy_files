# Gillsystems_unmess_your_rigs_messy_files Architecture Design

## Current Product Shape

The application is currently best understood as a local-first desktop duplicate-file tool.

```text
GUI (egui/eframe)
    |
    v
UnmessSecretFunctions
    |
    +-- FileScanner
    +-- RemediationEngine
    +-- SQLite Database
    +-- Optional gRPC service scaffolding
```

## Core Runtime Pieces

### GUI Layer

The GUI is implemented with `egui/eframe`.

Current responsibilities:

- Start and stop scans
- Render duplicate groups from the latest completed scan
- Show per-group file comparisons
- Launch background search jobs
- Confirm and trigger remediation actions
- Export results and open local documentation

### App Core

`UnmessSecretFunctions` coordinates the major subsystems.

Current responsibilities:

- Load configuration
- Persist or reuse a stable local node ID
- Normalize scan roots before a scan starts
- Launch the scanner and track progress
- Insert scan results into SQLite
- Rebuild duplicate groups for the active scan
- Execute remediation actions against the filesystem
- Export scan-scoped reports

### Database Layer

SQLite is the source of truth for local state.

Important tables:

- `nodes`
- `scans`
- `files`
- `duplicate_groups`
- `duplicate_files`
- `remediation_actions`
- `audit_log`
- `settings`

Important current behavior:

- Duplicate groups are scoped to `scan_id`
- Duplicate membership is stored explicitly in `duplicate_files`
- File rows are unique per `node_id + scan_id + file_path`
- Remediation status is persisted on files

### Remediation Engine

The remediation engine already contains the real file-operation primitives.

Implemented operations:

- Quarantine
- Delete with optional verification
- Move
- Filesystem-aware deduplication and linking
- Quarantine cleanup

The GUI now calls these operations through the app core instead of showing warning-only placeholders.

## Data Flow

### Scan Flow

1. GUI starts a scan.
2. App core normalizes scan paths.
3. Scanner discovers files and computes hashes.
4. Results are inserted into SQLite.
5. Duplicate groups are rebuilt for that scan only.
6. GUI reloads the latest completed scan.

### Remediation Flow

1. User selects a duplicate group.
2. User chooses the file to keep and the files to act on.
3. GUI asks for confirmation.
4. App core runs the remediation action.
5. SQLite state is updated.
6. Duplicate groups are rebuilt for that scan.
7. GUI reloads the results.

## Explicit Non-Goals For The Current Docs

These notes do not claim any of the following as delivered product behavior:

- True Windows Aero glass rendering
- Distributed SQLite synchronization
- Finished multi-node duplicate orchestration
- Production-ready cluster management

Those areas may have scaffolding or partial code, but they are not the current product center.
