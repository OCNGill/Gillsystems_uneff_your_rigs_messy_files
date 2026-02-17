# Contributing to Gillsystems_uneff_your_rigs_messy_files

**Welcome, developer!** We're thrilled you want to contribute. This guide will help you navigate the codebase and submit high-quality contributions.

---

## Table of Contents
1. [Philosophy](#philosophy)
2. [Development Environment](#development-environment)
3. [Building & Testing](#building--testing)
4. [Project Structure](#project-structure)
5. [Code Style](#code-style)
6. [Adding Features](#adding-features)
7. [Submitting Changes](#submitting-changes)

---

## Philosophy

> **"Systems Should Serve Humans — not the other way around."**

This project embodies three core principles:

1. **Radical Transparency**: Every byte, every decision, every operation is visible to the user
2. **User Empowerment**: Honest warnings, never silent deletions; full admin assumed
3. **Full Speed**: All CPU cores, no artificial limits, zero frameworks, maximum intent

### What This Means for Contributors
- **No gatekeeping**: Admin required, so no permission checks in code
- **Zero frameworks**: We use minimal dependencies (egui for GUI, tokio for async, rusqlite for DB)
- **Empirical metrics**: Code must demonstrably solve a problem or improve performance
- **Transparency first**: Every feature must be auditable and understandable

---

## Development Environment

### Prerequisites
- **Rust 1.92.0+** (stable)
- **Windows 7+**, Linux, or macOS
- **Admin/Sudo access** (required for full testing)
- **Git** for version control

### Setup
```bash
# 1. Install Rust (via rustup)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# 2. Clone the repository
git clone https://github.com/OCNGill/Gillsystems_uneff_your_rigs_messy_files.git
cd Gillsystems_uneff_your_rigs_messy_files/rust_agent_prototype

# 3. Verify toolchain
rustc --version  # Should be 1.92.0
cargo --version  # Should be 1.92.0

# 4. Verify build
cargo build
```

### IDE Setup (VS Code Recommended)
```bash
# Install Rust Analyzer extension
# Install CodeLLDB for debugging
# Install Clippy for linting

# Configure tasks.json for cargo build/test
# See .vscode/tasks.json if present
```

---

## Building & Testing

### Debug Build
```bash
cargo build
# Output: target/debug/uneff-your-rigs (larger, with debug symbols)
```

### Release Build
```bash
cargo build --release
# Output: target/release/uneff-your-rigs (4.68 MB, optimized)
```

### Run Tests
```bash
# Run all tests
cargo test --lib

# Run tests with output
cargo test --lib -- --nocapture

# Run specific test
cargo test module_name --lib
```

### Linting & Formatting
```bash
# Check for issues (no fixes applied)
cargo clippy

# Format code
cargo fmt

# Check formatting
cargo fmt -- --check
```

### Documentation
```bash
# Generate HTML docs and open in browser
cargo doc --open

# Build docs (no browser)
cargo doc
```

---

## Project Structure

```
rust_agent_prototype/
├── src/
│   ├── main.rs              # Entry point, CLI parsing, tracing setup
│   ├── agent.rs             # UneffAgent orchestrator (scan pipeline, dedup)
│   ├── database.rs          # SQLite layer (nodes, drives, files, scans, duplicates)
│   ├── file_scanner.rs      # Parallel filesystem walk + xxHash64 hashing
│   ├── hashing.rs           # Two-stage hash (xxHash64 + SHA-256)
│   ├── platform.rs          # Cross-platform (ZFS, NTFS, ext4, XFS, APFS, FAT32)
│   ├── config.rs            # TOML configuration + validation
│   ├── remediation.rs       # Dedup strategies (clone, hard link, delete, quarantine)
│   ├── service.rs           # gRPC service (peer-to-peer API)
│   └── gui.rs               # egui-based GUI (Windows 7 Aero theme)
├── build.rs                 # Protobuf codegen (tonic + protoc-bin-vendored)
├── proto/
│   └── agent_service.proto  # gRPC service definition
├── Cargo.toml               # Dependencies + build config
└── README.md
```

### Module Responsibilities

| Module | Responsibility | Key Types | Public API |
|--------|---------------|-----------|-----------|
| **agent.rs** | Orchestration | UneffAgent, ScanState | new, scan_async, remediate |
| **database.rs** | SQL storage | Database, FileRow | new, insert_files_batch, find_sha256_matches |
| **file_scanner.rs** | Filesystem walk | FileScanner, ScanProgress | new, scan_paths |
| **hashing.rs** | Fingerprinting | HashEngine | compute_xxhash64, compute_sha256 |
| **platform.rs** | OS integration | PlatformInfo, FsType | detect_drives, detect_fs_type |
| **config.rs** | Configuration | Config, RemediationConfig | load, validate |
| **remediation.rs** | Dedup operations | RemediationEngine, RemediationResult | quarantine, hard_link, delete |
| **service.rs** | gRPC API | GrpcService | start, handle_scan_request |
| **gui.rs** | User interface | UneffGUI, GuiMessage | run_gui, update |

---

## Code Style

### Rust Conventions
```rust
// Doc-comments for public items (module-level, struct, enum, function)
/// Scans filesystem for duplicate files using xxHash64 + SHA-256.
///
/// # Arguments
/// * `paths` - Vector of filesystem paths to scan
/// * `cancel_token` - Arc<AtomicBool> for graceful cancellation
///
/// # Returns
/// Vector of FileRow structs with hashes computed
///
/// # Errors
/// Returns error if filesystem access fails
pub fn scan_paths(paths: Vec<PathBuf>, cancel_token: Arc<AtomicBool>) -> Result<Vec<FileRow>> {
    // Implementation
}

// Naming conventions
let snake_case_variable = 42;
const SCREAMING_SNAKE_CONSTANT: usize = 100;
struct PascalCaseStruct { field: String }
enum PascalCaseEnum { Variant }

// Error handling: use anyhow::Result<T> for most functions
pub fn do_something() -> Result<String> {
    Ok("success".to_string())
}

// Arc + Mutex for shared mutable state
let shared_data = Arc::new(Mutex::new(data));

// AtomicBool for cancel flags (lock-free)
let cancel_flag = Arc::new(AtomicBool::new(false));
```

### Comments
```rust
// Line comments for explaining *why*, not *what*
// The 'what' should be obvious from code

// For complex algorithms, add a comment explaining the logic:
// We use xxHash64 first (fast) as a pre-filter,
// then SHA-256 (cryptographic) to avoid collisions.

// Use tracing macros for operational logging:
info!("Scan started: {} files discovered", file_count);
warn!("Filesystem does not support hard links");
error!("Failed to write database: {:?}", error);
```

### Testing
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hash_computation_consistent() {
        let path = PathBuf::from("test_file.txt");
        let hash1 = compute_xxhash64(&path).unwrap();
        let hash2 = compute_xxhash64(&path).unwrap();
        assert_eq!(hash1, hash2, "Hashes should be identical for same file");
    }
}
```

---

## Adding Features

### Step 1: Plan
- **Open an Issue** to discuss the feature
- **Get buy-in** from maintainers on philosophy alignment
- **Design** the feature with minimal dependencies

### Step 2: Branch
```bash
git checkout -b feature/my-feature-name
```

### Step 3: Implement
- Add code to the appropriate module (see Module Responsibilities)
- Add doc-comments to all public items
- Add tests for new functionality
- Run `cargo clippy` and fix any warnings

### Step 4: Test
```bash
# Unit tests
cargo test --lib

# Integration (manual)
./target/debug/uneff-your-rigs --gui-only
cargo build --release && ./target/release/uneff-your-rigs --service
```

### Step 5: Document
- Update `README.md` if user-facing
- Update `user_guide.md` if GUI changes
- Add comments explaining non-obvious logic

---

## Submitting Changes

### Before Pushing
1. **Run tests**: `cargo test --lib`
2. **Check linting**: `cargo clippy`
3. **Format code**: `cargo fmt`
4. **Verify build**: `cargo build --release`
5. **Update docs**: README, user guide, doc-comments

### Commit Messages
```
feat: Add feature description in one line

Add more details here if needed.
Explain the *why*, not the *what*.

Fixes #123  (if applicable)
```

### Types
- `feat:` - New feature
- `fix:` - Bug fix
- `docs:` - Documentation only
- `style:` - Code formatting (cargo fmt)
- `refactor:` - Code restructuring
- `perf:` - Performance improvement
- `test:` - Adding or fixing tests
- `chore:` - Maintenance (deps, config)

### Push & Create PR
```bash
git push origin feature/my-feature-name
# Then open a PR on GitHub
```

### PR Template
```markdown
## Description
Brief description of the changes

## Motivation
Why is this change needed?

## Testing
How did you test this?

## Philosophy Alignment
How does this align with "Systems Should Serve Humans"?

## Checklist
- [ ] Tests added/updated
- [ ] Docs updated
- [ ] No clippy warnings
- [ ] Code formatted (cargo fmt)
- [ ] Release build verified
```

---

## Debugging

### Enable Tracing
```bash
# Set environment variable
export RUST_LOG=debug
# or on Windows
set RUST_LOG=debug

# Run agent
./target/debug/uneff-your-rigs
```

### Attach Debugger (VS Code)
```bash
# In .vscode/launch.json
{
    "type": "lldb",
    "request": "launch",
    "name": "Debug",
    "cargo": {
        "args": ["build", "--bin=uneff-your-rigs"],
        "filter": "uneff-your-rigs"
    }
}
```

### Log Database Queries
In `database.rs`:
```rust
info!("Executing query: {}", sql);
```

---

## Performance Optimization

### Profiling
```bash
# CPU profiling (Linux only with perf)
perf record ./target/release/uneff-your-rigs --gui-only
perf report

# Flamegraph (cargo-flamegraph)
cargo install flamegraph
cargo flamegraph --bin uneff-your-rigs
```

### Benchmarking
```bash
# Add benchmark (nightly Rust required)
#[bench]
fn bench_xxhash64(b: &mut Bencher) {
    b.iter(|| compute_xxhash64(&test_file))
}
```

---

## Resources

- **Rust Book**: https://doc.rust-lang.org/book/
- **egui Documentation**: https://docs.rs/egui/
- **Tokio Async Runtime**: https://tokio.rs/
- **Our codebase docs**: `cargo doc --open`

---

## Questions?

- **Open an Issue**: https://github.com/OCNGill/Gillsystems_uneff_your_rigs_messy_files/issues
- **Email**: gillsystems@gmail.com

---

**Thank you for contributing to making systems serve humans!** 🚀
