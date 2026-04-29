# GPT-5.4 X HI APRIL 27 2026 Repo Analysis

## Audit Charter

This report is a brand new top-down analysis of the repository by a virtual team of 5 senior system architects, including 2 GUI specialists.

Date: 2026-04-27
Primary goal: determine what the project actually is today, what works, what is overstated, what is risky, and what the correct fix program should be.

## Architect Team

| Architect | Specialty | Primary Focus |
| --- | --- | --- |
| Architect 1 | Principal Systems Architect | Repository truth, runtime structure, release readiness |
| Architect 2 | Distributed Systems Architect | Service model, peer-to-peer claims, network discovery |
| Architect 3 | Storage and Data Integrity Architect | Scanner, database, duplicate detection, remediation safety |
| Architect 4 | Principal UI and UX Architect | Workflow design, state model, user interaction fidelity |
| Architect 5 | GUI Rendering and Theme Architect | egui implementation quality, theme system, Aero claims |

## Scope Reviewed

Reviewed as authoritative:

- Root documentation and metadata: README.md, architecture_design.md, task_ledger.md, user_guide.md, manifest.json, config.toml, version.py
- Rust project metadata and generation: rust-source/Cargo.toml, rust-source/build.rs, rust-source/proto/agent_service.proto
- All substantive source modules under rust-source/src/
- Evidence artifacts: rust-source/build_errors.txt, scan_logs/*.md, scan_logs/*.json
- Provided design document: docs/design_team_analysis.md

Inventoried but not treated as authoritative source code:

- rust-source/target/
- archive/
- root binaries in the repository
- dist/

Reason: those paths are build outputs, archived artifacts, or compiled binaries rather than the current source of truth.

## Executive Verdict

The repo contains a real Rust application with a large native egui GUI, a real scan pipeline, a real local SQLite store, a real hashing path, and a partially real remediation engine. It is not vaporware.

It is also not in the state the docs claim.

The biggest truth about the project is this:

1. The application is currently a single-node duplicate analysis tool with a visually distinctive GUI.
2. The distributed peer-to-peer service story is mostly scaffolded, not delivered.
3. The GUI is functionally broader than the docs imply in some areas, but several critical commands are still stubs.
4. The duplicate model is currently global across the database, not cleanly scoped to an active scan, which can create misleading self-duplicates across repeated runs.
5. The theme is a Matrix-green custom skin with faux glass accents, not actual Windows 7 Aero.
6. The repo suffers from serious documentation drift and release-status inflation.

Release recommendation:

- Safe for continued development and guided internal use.
- Not ready to market as a production-grade multi-node deduplication platform.
- Not ready to market as a true Windows 7 Aero implementation.
- Not ready to trust for destructive remediation workflows without the P0 and P1 fixes listed below.

## What Is Actually Good

The repo has several real strengths:

- Cleanly separated modules for config, scanning, hashing, database, platform, remediation, GUI, and service boundaries.
- A real native GUI with custom title bar, branded layout, dual-panel display, search, settings, boot mode selector, and status flow.
- A sensible SQLite schema foundation with WAL mode and indexes.
- A real two-hash pipeline foundation using xxHash64 and SHA-256.
- A remediation engine that contains more real work than the docs-vs-code mismatch first suggests.
- Exported scan logs that prove the core scan-to-report pipeline runs.
- A repo structure that can be repaired into a strong single-binary desktop application without throwing everything away.

## Top-Down Architecture Reality

### Runtime Flow

The actual runtime path is:

1. rust-source/src/main.rs initializes tracing and parses CLI mode.
2. No CLI args means the app launches the boot screen.
3. The boot screen routes to GUI mode, service mode, or SMB setup.
4. GUI mode initializes UnmessSecretFunctions and then launches the egui application.
5. Scans flow through file_scanner.rs, hashing.rs, database.rs, and then back into gui.rs.
6. Service mode launches a TCP placeholder under service.rs, not a real tonic-backed gRPC server.

### Implemented vs Claimed System

| Area | Claimed In Docs | Actual State |
| --- | --- | --- |
| Single native binary | Yes | True |
| Windows 7 Aero glass UI | Yes | Partially true visually, false technically |
| Multi-node gRPC peer network | Yes | Mostly scaffolded |
| Distributed SQLite sync | Yes | Not implemented |
| Complete remediation workflow | Yes | Partially implemented and not fully wired to UI |
| 0 TODOs remaining | Yes | False |
| 0 warnings build state | Yes | False according to checked-in build log |
| Version consistency | Implied | False across docs and metadata |

## File-by-File Audit Matrix

This section covers every substantive logic-bearing file that matters to the actual product.

| File | Role | Current State | Required Fix |
| --- | --- | --- | --- |
| README.md | Public project truth | Overstates maturity, version, GUI fidelity, multi-node reality | Rewrite to match code and actual feature set |
| architecture_design.md | High-level architecture | Mixes old web/Tauri ideas with current egui binary | Replace aspirational design with current architecture and future roadmap |
| task_ledger.md | Project status ledger | Claims 0 TODOs, 0 warnings, complete modules | Convert to factual execution ledger tied to real backlog |
| user_guide.md | User-facing behavior | Promises Aero glass and multi-node behaviors not actually delivered | Rewrite around current UI and supported workflows |
| manifest.json | Repo metadata | Version and team narratives drift from code | Sync with version.py and current architecture |
| config.toml | Default runtime config | Reasonable baseline | Keep, but ensure settings changes actually apply |
| version.py | Version authority | Strongest source of truth in repo | Keep as authority and sync everything else to it |
| rust-source/Cargo.toml | Build and dependency truth | Mostly solid | Keep, but align docs and add quality gates |
| rust-source/build.rs | Proto generation and version env | Works in current layout, but pathing is brittle | Make version sourcing robust and fail loudly when missing |
| rust-source/proto/agent_service.proto | Intended service contract | Rich API definition | Either implement fully or de-scope until ready |
| rust-source/src/main.rs | App entrypoint | Functional but version strings are hardcoded and stale | Use APP_VERSION everywhere and clean mode logic |
| rust-source/src/boot_screen.rs | Launcher and permission gate | Real UI, but permission/elevation strategy is inconsistent and platform-fragile | Normalize elevation behavior and remove brittle Windows runas approach |
| rust-source/src/config.rs | Config load and validation | Good base, limited validation, ENV override missing | Add environment overrides, stronger validation, live reload path |
| rust-source/src/database.rs | Data model and storage | Solid base schema, but duplicate model is incomplete and not scan-scoped | Add scan scoping, file uniqueness rules, duplicate membership writes |
| rust-source/src/file_scanner.rs | Filesystem walk and progress | Functional, but thread settings are ignored and path model is naive | Honor configured threads, canonicalize roots, dedupe discovered paths |
| rust-source/src/hashing.rs | Hash engine | Correct core logic, but serial and always computes SHA-256 | Implement staged and parallel hashing as claimed |
| rust-source/src/platform.rs | Drive enumeration and service registration | Partially real, partially overstated | Make service registration honest and platform-correct |
| rust-source/src/remediation.rs | Dedup and file operations | Realer than docs suggest, but missing full transactional safety and UI integration | Wire to DB, audit, strategy limits, and active GUI actions |
| rust-source/src/service.rs | Network service layer | Placeholder TCP listener only | Replace with real tonic server and client calls |
| rust-source/src/smb_server.rs | SMB setup and sharing | Mixed quality, command execution is fragile and some commands look wrong | Rebuild per-platform command model and test it properly |
| rust-source/src/gui.rs | Main application UI | Large, visually distinctive, but overgrown, partially stubbed, and state-fragile | Split into modules, fix blocking workflows, finish commands |
| rust-source/src/unmess_program.rs | Application orchestrator | Good central spine, but ignores remediation config and has global-scan problems | Persist node identity, scope scans properly, expose remediation commands |
| rust-source/build_errors.txt | Build evidence | Shows prior release build with warnings | Use as evidence only; replace with actual CI |
| scan_logs/*.md and *.json | Runtime evidence | Proves scan reporting works, also exposes duplicate-model flaws | Use as regression fixtures in tests |

## Architect 1 Findings: Principal Systems Architecture

### System Truth Problem

The repo has a truth gap:

- version.py and Cargo.toml are at 0.5.1
- README.md still presents 0.4.0-era claims
- task_ledger.md and architecture docs describe a more complete platform than the code delivers

This is not a cosmetic issue. It causes bad engineering decisions because the team can no longer trust the documents.

### Correct Fix

Establish a strict truth hierarchy:

1. version.py as single version authority.
2. Cargo.toml and manifest.json generated or checked against version.py.
3. README.md and user_guide.md rewritten to describe current, verified behavior only.
4. task_ledger.md converted from narrative hype to a real execution backlog.

### Architectural Recommendation

Declare the product as one of these two things, then build honestly around it:

Option A: a powerful single-node desktop deduplication tool with limited network discovery.

Option B: a true multi-node peer tool with fully implemented service discovery, RPC, and cluster state.

Right now the code is much closer to Option A.

## Architect 2 Findings: Distributed Systems and Service Layer

### Service Reality

service.rs is not a gRPC service. It is a TCP accept loop that logs connections and drops them.

The proto contract in rust-source/proto/agent_service.proto is ambitious and credible, but none of the handlers are wired.

### Network Discovery Reality

Network discovery is partially real on Windows:

- local node is added immediately
- mapped drives and SMB-related shells are queried
- ARP peers are collected
- background SMB host discovery merges several Windows-specific sources

But it is not the cluster/discovery stack the docs promise:

- no actual RPC handshake
- no node capability exchange
- no service health protocol
- no scan orchestration across peers
- non-Windows background SMB discovery is empty

### Correct Fix

Implement one coherent networking stack:

1. Persist node identity instead of generating a fresh UUID every launch.
2. Use tonic::transport::Server in service.rs.
3. Bind proto handlers for StartScan, GetSystemInfo, ExecuteRemediation, HealthCheck, GetMounts, and StopScan.
4. Add a small discovery layer that is platform-neutral. mDNS is the cleanest honest option for LAN discovery.
5. Drop all claims of distributed SQLite sync unless and until it is implemented. SQLite should remain local cache unless a real replication layer is added.
6. Add timeouts, retries, and offline state to NodeInfo management.

### Release Gate

Do not market peer-to-peer functionality until service.rs is a real tonic service and at least one end-to-end network test exists.

## Architect 3 Findings: Storage, Data Integrity, and Remediation

### The Most Important Logic Defect

Duplicate presentation is not scoped to the active scan.

Evidence:

- gui.rs reloads all duplicate groups from the database.
- database.rs get_duplicate_groups() has no scan filter.
- database.rs get_files_by_hash() has no scan filter.
- scan logs contain multiple groups where the same absolute file path appears twice.

That means the UI and exported reports can present a file as a duplicate of itself across repeated scans or overlapping scan histories.

### Additional Data-Layer Findings

1. duplicate_groups exists, but duplicate_files is never populated.
2. Duplicate groups are keyed by sha256 only, not by scan session.
3. There is no uniqueness constraint protecting repeated insertion of the same file path for the same scan.
4. UnmessSecretFunctions::new ignores remediation config values and hardcodes quarantine path, grace period, and verify-before-delete behavior.
5. Node identity is ephemeral, so audit history and cluster identity are unstable across launches.
6. The scanner claims staged hashing, but hashing.rs computes xxHash64 and SHA-256 for every file immediately.
7. file_scanner.rs ignores config.scanning.thread_pool_size and hardcodes num_cpus().min(8).
8. There is no canonical-path dedup or overlapping-root protection.

### Remediation Engine Assessment

remediation.rs is not empty. It has real implementations for:

- quarantine
- restore
- hard links
- Linux reflink path
- macOS clonefile path
- fallback logic

But it is not production-safe enough yet because the whole remediation workflow is not fully integrated:

- UI delete/cut/copy/paste paths are stubs
- database mutation and audit wiring are incomplete at the app layer
- there is no full transaction around duplicate group, file-state, and remediation state mutation
- max_hard_links_per_file from config is not enforced

### Correct Fix

Database and scan model:

1. Add scan_id to duplicate_groups or replace duplicate_groups with a per-scan materialized result table.
2. Populate duplicate_files as the canonical membership table.
3. Make get_duplicate_groups() and get_files_by_hash() accept a scan context.
4. Make the GUI load only the latest completed scan by default, with explicit scan history navigation.
5. Add uniqueness constraints such as node_id + scan_id + file_path.
6. Deduplicate scan roots by canonical path and reject nested overlaps unless intentionally allowed.

Remediation wiring:

1. Add explicit app-layer commands for quarantine, hard-link dedup, delete, and restore.
2. Write remediation_actions and audit_log entries for every mutation.
3. Mark files deleted or remediated in the database consistently.
4. Add preflight checks for cross-device operations, permissions, max hard-link counts, and same-file cases.

## Architect 4 Findings: Principal UI and UX

### GUI Strengths

The GUI is not shallow. It has meaningful product work in it:

- custom title bar
- branded header and footer
- sidebar with drives and network devices
- dual-panel duplicate view
- settings dialog
- about dialog
- warning dialog
- status strip
- search mode
- scan start and stop/export path

### GUI Structural Problems

gui.rs is too large and too central.

Current consequences:

- menu behavior, rendering, search, settings, discovery, warnings, duplicate display, and state transitions are all in one file
- selection logic is fragmented across multiple HashSet fields and selected indices
- the app core is optional at runtime, which multiplies failure paths
- several menu actions are still placeholders
- settings change the file on disk but do not rebuild or reconfigure the live app core

### UX Defects That Matter

1. Open Saved Scan is stubbed.
2. Save Results is stubbed.
3. Filter is stubbed.
4. Select All is stubbed.
5. Invert Selection is stubbed.
6. Copy is stubbed.
7. Paste is stubbed.
8. Delete only raises a warning instead of executing a real flow.
9. Search walks the filesystem synchronously on the UI side.
10. Search has no pagination or background progress.
11. The visible metadata columns model does not drive the main table.
12. The Help menu opens GitHub instead of local docs, which weakens the offline native-app story.
13. The macOS path opener is wrong because the unix branch uses xdg-open.
14. The close button uses std::process::exit(0) after a close command, which is harsh and bypasses graceful shutdown.

### Correct Fix

Split gui.rs into modules:

1. gui/mod.rs
2. gui/theme.rs
3. gui/titlebar.rs
4. gui/sidebar.rs
5. gui/duplicates.rs
6. gui/search.rs
7. gui/dialogs.rs
8. gui/state.rs

And introduce:

- a SelectionState struct
- a ViewMode enum
- a Notification or Toast model
- a ScanSessionRef model for current and historical scans
- a background job abstraction for search and network discovery

## Architect 5 Findings: GUI Rendering and Aero Fidelity

### What Is Real Today

The GUI has real visual craft:

- custom painter-driven title bar
- layered dark-green glass-like bands
- custom border and glow logic
- strong Matrix-green palette
- consistent black and green contrast language

### What Is Not Real Today

The current UI is not true Windows 7 Aero.

It does not implement:

- DWM blur
- compositor-backed transparency
- actual Aero window chrome
- true color theme presets like Blue, Silver, Olive
- a real animation engine
- 60 FPS continuous animated transitions

The current implementation is best described as:

Matrix Glass Industrial Theme with Win7-inspired title-bar treatment.

### Additional Rendering Findings

1. animation_time and hover_progress exist but are effectively unused.
2. request_repaint_after(Duration::from_millis(250)) is a low-rate redraw cycle, not animation-grade rendering.
3. The right-edge manual resize branch appears to miss a direct East case.
4. The theme is hardcoded and not abstracted.

### Correct Fix

Choose one of two honest directions.

Direction A, recommended:

- keep the Matrix theme as the default identity
- rename the visual language honestly
- add a theme system with Matrix, Aero Blue, Aero Silver, and Olive presets
- add lightweight hover and focus transitions without pretending to be DWM glass

Direction B, expensive but possible:

- add a platform renderer abstraction
- use native Windows composition APIs for real blur and transparency on Windows
- provide approximate equivalents on Linux and macOS
- keep a fallback pure-egui path

Direction A is the correct business decision unless true Aero fidelity is a core brand requirement.

## Evidence-Based Build and Test Assessment

### Build State

Current shell environment in this session does not expose cargo or rustc in PATH, so a live build could not be executed here.

However, the checked-in rust-source/build_errors.txt shows a prior release build that completed successfully and reported 4 warnings:

1. unnecessary parentheses in boot_screen.rs
2. unnecessary parentheses in boot_screen.rs
3. unnecessary parentheses in gui.rs
4. unused warn import in smb_server.rs

That means the repo is closer to compiling cleanly than the stale docs suggest, but the claimed 0 warnings state is still false based on the evidence present in the repository.

### Source Test Coverage Reality

Source-defined tests currently found in reviewed modules:

- config.rs: 3 tests
- file_scanner.rs: 2 tests
- smb_server.rs: 3 tests

That is thin coverage for the actual risk surface. There are no meaningful tests for:

- database query correctness
- duplicate group scoping
- hashing behavior across staged workflow
- remediation safety cases
- service handlers
- platform registration behavior
- GUI state transitions

## GUI-Focused Detailed Conclusions

The GUI is where the product identity lives, so it deserves an explicit verdict.

### Current GUI Grade

| Dimension | Grade | Comment |
| --- | --- | --- |
| Visual distinctiveness | A- | Strong branded identity |
| Functional completeness | C | Too many stubbed commands |
| Architecture maintainability | C- | One oversized file and fragmented state |
| Performance design | C | Blocking search and underused async patterns |
| Cross-platform correctness | C- | Windows-first implementation with Unix/macOS gaps |
| Aero fidelity | D | Inspired by Aero, not actual Aero |

### Proper GUI Repair Program

Phase G1, make it truthful and safe:

1. Remove or rewrite all fake or stubbed menu items until they work.
2. Move duplicate presentation to latest-scan scope.
3. Make search asynchronous and cancelable.
4. Make settings apply to live app state or clearly require restart.
5. Replace the Option app core pattern with a defined startup-state model.

Phase G2, make it maintainable:

1. Split gui.rs into focused modules.
2. Centralize selection and modal state.
3. Introduce a command dispatcher for menu and toolbar actions.
4. Add regression tests for search, scan state, duplicate view population, and command enablement.

Phase G3, make it visually excellent:

1. Introduce a real theme abstraction.
2. Add subtle transition timing.
3. Add proper hover/focus behavior.
4. Decide between honest Matrix identity and actual Aero implementation.

## Prioritized Fix Program

### P0: Ship-Blocking Correctness and Truth

1. Rewrite repo docs and metadata to match code reality.
2. Scope duplicate groups and duplicate file loading to a specific scan.
3. Persist node identity across launches.
4. Make remediation config values actually drive UnmessSecretFunctions::new.
5. Implement real delete, quarantine, and dedup actions end-to-end through GUI, orchestrator, remediation, and database.
6. Remove or disable menu items that are still stubs until implemented.

### P1: Core Product Integrity

1. Populate duplicate_files and use it as the authoritative group membership table.
2. Add unique constraints for file rows within a scan.
3. Honor thread_pool_size in file_scanner.rs.
4. Make hashing staged rather than always-compute-both.
5. Fix cross-platform open-file-location behavior.
6. Rebuild the SMB command model and test it on each platform branch.

### P2: Real Networking or Honest De-Scope

1. Implement real tonic handlers.
2. Add discovery and health semantics.
3. Add integration tests for one host controlling another.
4. If this work is not immediately funded, explicitly de-scope peer features in docs and UI.

### P3: GUI Excellence

1. Split gui.rs.
2. Add async search jobs with pagination.
3. Add theme presets.
4. Add actual transition timing and fix manual resize edge handling.
5. Add scan history load and save behavior.

## Concrete Proper Fixes By Module

### main.rs

- Replace hardcoded 0.4.0 strings with option_env!("APP_VERSION") fallback logic.
- Unify boot comments and actual permission policy.
- Surface startup mode and version consistently in logs.

### boot_screen.rs

- Replace the Windows elevation strategy with ShellExecuteW runas or a proper UAC approach.
- Decide whether the boot screen is mandatory or advisory. Right now main.rs and boot_screen.rs send mixed messages.
- Remove build warnings.

### config.rs and config.toml

- Implement environment override support.
- Validate remediation paths and port ranges.
- Add a live-config application path or explicit restart notice.

### database.rs

- Add per-scan duplicate result modeling.
- Populate duplicate_files during detect_duplicates.
- Add methods like get_duplicate_groups_for_scan(scan_id) and get_files_for_group(group_id).
- Add uniqueness on node_id + scan_id + file_path.
- Add cleanup APIs for old scans.

### file_scanner.rs

- Honor configured thread pool size.
- Canonicalize and deduplicate roots before walking.
- Detect and reject nested overlapping scan roots or collapse them.
- Add progress for skipped files and error counts.

### hashing.rs

- Parallelize across batches.
- Only compute SHA-256 after size plus xxHash64 matches or when explicitly requested.
- Add tests for staged promotion behavior.

### unmess_program.rs

- Use config.remediation values instead of hardcoded quarantine and grace settings.
- Persist node_id in config or database and reload it.
- Record latest completed scan and expose it to the GUI.
- Add remediation command APIs.
- Populate duplicate_files and audit log consistently.

### remediation.rs

- Enforce config max_hard_links_per_file.
- Add same-file detection and inode-based safety checks where supported.
- Return structured remediation failures back through the GUI.
- Add transaction-like orchestration with database logging.

### service.rs

- Replace TCP placeholder with real tonic server.
- Back the proto handlers with app orchestration calls.
- Add health and system info responses first, then scan control, then remediation.

### platform.rs

- Make Windows service support real if the docs are going to claim it.
- Separate GUI auto-start from service installation.
- Return meaningful errors instead of silently swallowing platform command failures.

### smb_server.rs

- Rework Windows net share command generation.
- Stop pretending launch_separate_process starts a real SMB server when it currently mostly opens a shell placeholder.
- Make localhost_only meaningful on all platforms, not just Samba config text.
- Add platform-specific smoke tests or at least command-shape unit tests.

### gui.rs

- Split the file.
- Replace stub actions with real flows or disable them.
- Load duplicates for the current scan only.
- Make search background and cancelable.
- Use a platform-correct open path strategy.
- Remove hard exit on close.
- Fix theme and animation abstractions.

### Docs and Metadata

- Rewrite README.md around what is verified.
- Rewrite user_guide.md around real GUI and supported actions.
- Rewrite architecture_design.md to describe egui and local SQLite reality, then separately list roadmap work.
- Update task_ledger.md into a factual engineering plan with P0, P1, P2, and P3 items.

## Recommended Execution Order

Do the repairs in this order:

1. Truth pass: versions, docs, product claims, stale status statements.
2. Data-model pass: scan scoping, duplicate membership, persisted node identity.
3. Command pass: remediation workflow, GUI action completion, platform opener fixes.
4. GUI architecture pass: split gui.rs, async search, selection state, settings model.
5. Networking pass: either de-scope or fully implement tonic service.
6. Visual pass: theme presets, animation, polish.
7. Test and CI pass: build, fmt, clippy, tests, integration coverage.

## Final Product Positioning Recommendation

The cleanest near-term marketable position is:

Native Rust duplicate analysis and cleanup desktop app with a bold Matrix-glass interface, local-first data model, scan export, and platform-aware remediation.

Only after the P2 networking work is finished should it be positioned as a serious multi-node peer system.

## Perfect Prompt For The Next Execution Task

Use this prompt exactly or with minimal edits:

```text
Work in this repository and use GPT-5.4 level rigor. Read GPT5_4_X_HI_APRIL_27_2026_analysis.md first and treat it as the execution authority.

Objective:
Implement the highest-value proper fixes with no placeholders, no shortcuts, and no doc-code drift. Complete the work end-to-end in this order unless you discover a dependency that forces a safer order:

Phase 1: Truth and integrity
1. Sync all version and status references across README.md, task_ledger.md, user_guide.md, architecture_design.md, manifest.json, main.rs, and any stale hardcoded strings so they match version.py and actual repo state.
2. Remove or rewrite any false claims about true Windows 7 Aero glass, full multi-node gRPC, distributed SQLite sync, zero TODOs, zero warnings, or completed features that are not actually implemented.

Phase 2: Duplicate-model correctness
3. Fix the scan and duplicate data model so duplicate results are scoped to a specific scan instead of globally mixing historical file rows.
4. Add or use a proper duplicate membership table so the GUI and exports load files for the active scan only.
5. Prevent same-path self-duplicates caused by repeated scans or overlapping roots.
6. Persist node identity across launches instead of generating a fresh UUID every run.

Phase 3: Real remediation wiring
7. Make UnmessSecretFunctions honor config.remediation values instead of hardcoded quarantine and verification settings.
8. Implement real end-to-end delete, quarantine, and dedup actions from GUI to app core to remediation engine to database and audit log.
9. Disable or remove any GUI commands that still cannot be safely completed in this task.

Phase 4: GUI architecture and UX
10. Refactor gui.rs into smaller modules if needed to complete the work safely.
11. Replace stubbed menu actions with working implementations where practical: Open Saved Scan, Save Results, Select All, Invert Selection, Filter, Copy, Paste, Delete. If any one of these cannot be completed safely, disable it explicitly and document why in the code and final summary.
12. Make search asynchronous and non-blocking.
13. Fix cross-platform open-file-location behavior so macOS does not fall through to xdg-open.
14. Remove hard process termination on normal close paths.

Phase 5: Validation
15. Add or update tests for the fixed duplicate-model behavior, remediation flow, and any new scan-scoping queries.
16. Run the best available validation commands. If cargo is unavailable in the environment, say so explicitly and use every available repo artifact to validate logically.

Constraints:
- Use apply_patch for edits.
- Preserve the existing product identity where it is honest, but do not preserve false claims.
- Fix root causes, not just UI symptoms.
- Do not leave TODO stubs behind for any feature you touch.
- Keep the solution native Rust and egui-based.
- Update documentation as part of the implementation.

Deliverables:
- Working code changes.
- Updated documentation.
- A concise final summary of what was fixed, what was validated, and any remaining deliberate deferrals.
```

## Bottom Line

This repo is worth fixing. The core is real, the GUI already has identity, and the codebase is salvageable without a rewrite.

The correct move is not to start over.

The correct move is to force the repo to tell the truth, fix the duplicate-model correctness issues, wire remediation properly, and then decide whether the networking story is a near-term deliverable or a later roadmap feature.