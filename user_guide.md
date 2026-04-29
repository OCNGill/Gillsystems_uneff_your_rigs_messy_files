# Gillsystems_unmess_your_rigs_messy_files — User Guide

Version: 0.5.1

This guide describes the behavior that is currently implemented in the repository.

## Quick Start

If you are running from source:

```bash
cd rust-source
cargo run --release
```

The application creates or reads `config.toml` from the workspace root. Scan history and duplicate groups are stored in the SQLite database configured there.

## Main Workflow

1. Start the application.
2. Open `Settings` if you need to adjust scan paths or remediation settings.
3. Click `Scan` to begin a new scan.
4. Wait for the scan to finish. Duplicate groups are rebuilt for that scan when results are flushed to the database.
5. Select a duplicate group in the left table.
6. In the comparison panel, choose the file to `KEEP` and select the copies you want to act on.
7. Use `Delete Selected`, `Quarantine Selected`, or `Deduplicate To KEEP`.
8. Confirm the action when prompted.
9. Export the latest scan report if you want a Markdown and JSON copy in `scan_logs/`.

## What The UI Shows

### Duplicate Groups

The main duplicate list is populated from the latest completed scan. Groups are scan-scoped, so older scan history does not get mixed into the current review view.

Each row shows:

- Wasted space for the group
- Representative file name and folder
- File type and size
- Modified timestamp

### Comparison Panel

When you open a group, the bottom panel shows every file in that duplicate set.

From there you can:

- Select which copies are in scope for the next action
- Mark one file as `KEEP`
- Open a file location in the platform file manager
- Delete one copy directly from its row

### Search Panel

The search panel searches across the configured scan roots in the background.

- Enter part of a file name, extension, or path fragment.
- Results highlight files that are already part of the currently loaded duplicate set.
- The search runs asynchronously so the GUI remains responsive.

## Remediation Actions

### Delete

Deletes the selected file permanently.

- The app can verify the file hash before deletion if `verify_before_delete` is enabled.
- Deleted files are removed from active duplicate groups after the action completes.

### Quarantine

Moves the selected file into the configured quarantine directory.

- This is safer than delete because the file is preserved.
- The original path is no longer considered active after the move.

### Deduplicate To KEEP

Uses the selected KEEP file as the source copy and applies filesystem-aware deduplication to the other selected files.

Depending on platform and filesystem support, this may use:

- Hard links
- Reflinks
- Other supported deduplication strategies in the remediation engine

## Reports

The app can export the latest completed scan to `scan_logs/`.

Each export creates:

- A Markdown summary
- A JSON report with duplicate-group details

Use `File -> Save Results` or `Stop + Export` after a running scan has fully finished flushing results.

## Configuration

The most important settings live in `config.toml`.

### Scanning

- `thread_pool_size`: file discovery concurrency
- `max_file_size_gb`: maximum file size to include
- `excluded_paths`: directories to skip

### Database

- `path`: SQLite file location
- `wal_mode`: enables SQLite WAL mode

### Remediation

- `quarantine_path`: where quarantined files go
- `grace_period_hours`: retention policy used by the remediation engine
- `verify_before_delete`: hash verification toggle before delete

## Current Limits

- The GUI is centered on the latest completed scan, not a merged history browser.
- Networking and service components exist in the codebase, but the application should currently be treated as a local-first tool.
- The visual theme is a custom matrix-green desktop UI, not true Windows Aero glass.

## Troubleshooting

### No duplicates appear after a scan

Check the following:

- The scan paths in `Settings` are correct.
- The scan actually completed rather than being cancelled.
- The files really share the same content hash.

### Delete or deduplicate fails

Common causes:

- The file no longer exists at the recorded path.
- The process lacks permission for that file.
- The target filesystem does not support the requested deduplication strategy.
- Hash verification blocked a destructive action because the file contents changed.

### Export does not appear immediately after stopping a scan

The stop workflow waits for the current scan to finish flushing its results. The export runs after that completion step, not at the instant the stop button is pressed.
