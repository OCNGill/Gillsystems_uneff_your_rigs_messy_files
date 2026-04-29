# Gillsystems_unmess_your_rigs_messy_files

Gillsystems_unmess_your_rigs_messy_files is a local-first Rust application for finding duplicate files, reviewing the results in a native egui desktop UI, and applying real remediation actions such as quarantine, delete, and filesystem-aware deduplication.

The current build is version 0.5.1. Version authority lives in `version.py` and is propagated into the Rust build as `APP_VERSION`.

## What It Actually Does

- Scans configured local paths and stores results in SQLite.
- Detects duplicates by content hash and keeps duplicate groups scoped to the scan that found them.
- Persists a stable local node ID instead of generating a new one every launch.
- Lets you mark a file to keep, then delete, quarantine, or deduplicate the other copies from the UI.
- Exports scan reports to `scan_logs/` as both Markdown and JSON.
- Runs file search in the background so the UI stays responsive.

## What It Does Not Claim

- It is not a delivered multi-node duplicate-management product.
- The gRPC service and networking code exist, but they should be treated as scaffolding rather than a finished distributed feature set.
- The desktop look is a custom matrix-green egui theme, not true Windows 7 Aero glass.

## Current Layout

- `rust-source/`: Rust application source, build script, protobuf definitions, and tests.
- `config.toml`: Runtime configuration file.
- `scan_logs/`: Exported scan reports.
- `docs/`: Design notes and supporting project documents.
- `version.py`: Version authority.

## Running From Source

```bash
cd rust-source
cargo run --release
```

Useful modes:

```bash
cargo run -- --gui-only
cargo run -- --service
```

## Validation

The codebase currently validates with:

```bash
cd rust-source
cargo check
cargo test
```

## Remediation Model

The UI now wires to the real remediation engine.

- `Delete`: Permanently removes a selected file, with optional SHA-256 verification before deletion.
- `Quarantine`: Moves a selected file into the configured quarantine directory.
- `Deduplicate To KEEP`: Replaces selected duplicate copies with a filesystem-aware deduplication strategy when supported.

All successful actions are written back to SQLite and removed from active duplicate groups.

## Configuration Notes

The app reads `config.toml`.

Important sections:

- `database.path`: SQLite file path.
- `scanning.thread_pool_size`: File discovery worker count.
- `scanning.max_file_size_gb`: Upper size limit during scanning.
- `remediation.quarantine_path`: Where quarantined files are moved.
- `remediation.verify_before_delete`: Enables hash verification before deletion.

## Limitations

- Duplicate history is scan-scoped, so the UI shows the latest completed scan rather than an aggregate of all scans.
- Search is intentionally limited in depth and result count to keep the interface responsive.
- Exported reports describe the latest completed scan unless a specific scan is requested programmatically.

## Documentation

- `user_guide.md`: End-user workflow and UI behavior.
- `architecture_design.md`: High-level project architecture notes.
- `task_ledger.md`: Ongoing work tracking.

## Support

If you find the project useful, support links and QR codes remain in `assets/`.
