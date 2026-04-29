# 7D Task Ledger — Gillsystems_unmess_your_rigs_messy_files

## Current State

- Version: 0.5.1
- Version authority: `version.py`
- Product center: local-first duplicate file scanning and remediation
- Validation state: `cargo check` passes, `cargo test` passes

## Recently Completed

- Replaced global duplicate grouping with scan-scoped duplicate groups.
- Added explicit duplicate membership storage in `duplicate_files`.
- Prevented self-duplicates by making file rows unique per `node_id + scan_id + file_path`.
- Persisted a stable local node ID in the database.
- Wired the GUI to real delete, quarantine, and deduplication actions.
- Moved file search off the UI thread.
- Removed hard process exits from GUI close paths.
- Updated README, user guide, manifest, and architecture notes to match the actual implementation.

## In Progress

- Tightening remaining wording and comment drift inside source files.
- Deciding how far to carry the optional service and networking scaffolding.
- Evaluating whether additional scan-history browsing belongs in the GUI.

## Deliberate Deferrals

- Do not market multi-node orchestration as a delivered feature until the service layer is real.
- Do not market the theme as true Windows Aero glass.
- Do not aggregate duplicate groups across scans in the main UI until that behavior is intentionally designed.

## Next Useful Work

- Add targeted tests for remediation actions at the app-core layer.
- Decide whether historical scan browsing should be exposed in the GUI.
- Remove unrelated warning-only compiler warnings in untouched files when convenient.
