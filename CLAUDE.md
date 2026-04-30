# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

SysClean is a TUI disk cleanup tool for Windows 11, built with Rust and ratatui. It provides two workspaces: **Cache Cleanup** (removes dev tool caches: uv, npm, pnpm, docker, cargo) and **Space Explorer** (browse recursive disk usage by folder). The app is Windows-only and uses static CRT linking (`+crt-static`).

## Common Commands

```powershell
# Run (debug build — slower scan, faster compilation)
cargo run

# Run release build (recommended for large home directories)
cargo run --release

# Run all tests
cargo test

# Run a single integration test
cargo test --test cache_discovery
cargo test --test explorer_behavior
cargo test --test progressive_scanning

# Run library unit tests only
cargo test --lib

# Format and lint
cargo fmt
cargo clippy --all-targets -- -D warnings

# Build MSI installer (requires cargo-wix)
cargo wix --nocapture
```

## High-Level Architecture

### Single-threaded TUI with background worker threads

`main.rs` owns the event loop, `ratatui::Terminal`, and `mpsc` channels. All long-running work (cache discovery, directory scanning, cleanup execution) spawns `std::thread` workers that send `WorkerMessage` variants back to the main thread. UI updates only happen in the main loop.

Task IDs (`latest_directory_task_id`, `latest_cache_task_id`) increment on every new scan; stale worker results whose ID does not match the latest are ignored. This is how "cancellation" works — it is UI-level dismissal, not hard thread interruption.

### Two workspaces backed by a shared `App` state

`app.rs` holds the global state machine:

- `ExplorerListState` manages the Space Explorer: it stores the full `Vec<DirectoryEntryInfo>`, a user filter string, and a selected index. The selection index is relative to the *visible* (filtered) entries, not the full list. `set_entries` preserves the selected path across updates and re-sorts by size descending.
- `Vec<CacheDiscovery>` holds cache targets for the Cache Cleanup workspace.
- `Page` switches workspaces; `ActiveDialog` overlays Help / DeleteConfirmation / CleanupSummary.

### Progressive directory scanning

`space_explorer.rs` splits scanning into two phases:

1. `discover_directory_skeleton` lists immediate subdirectories of the current path (fast). These render immediately with `scan_state = Pending` and `size_bytes = 0`.
2. A worker thread pool (`recommended_worker_count`, capped 2–6) computes recursive sizes via `compute_path_size`. Results are batched (`DIRECTORY_UPDATE_BATCH_SIZE = 4`, throttle 125ms) and streamed back as `DirectoryEntriesUpdated` messages. The list re-sorts by size automatically.

`persistence.rs` (`ScanCache`) saves full scan snapshots as JSON with a 24-hour TTL to `%LOCALAPPDATA%\sysclean\scan-cache.json`. Revisiting a directory within TTL loads from cache instantly.

### Cache discovery: CLI-first, then fallback paths

`cache_cleaner.rs::discover_cache_metadata` attempts official CLI commands first (`uv cache dir`, `npm config get cache`, `pnpm store path`, `docker system df --format json`). On failure, it falls back to known Windows paths under `%LOCALAPPDATA%` and `%USERPROFILE%`.

Cache sizes are populated in a separate background pass per target. Docker is special: its size comes directly from `docker system df` output, not filesystem traversal.

### Cleanup strategies

`CacheTargetKind::cleanup_command()` determines how deletion happens:

- **CLI-based** (uv, npm, pnpm, docker): runs the tool's official cleanup command via `SystemCommandRunner`, which wraps `cmd /C` on Windows.
- **Manual file deletion** (cargo): deletes contents of `.cargo/registry` and `.cargo/git` via `remove_path_contents`.

Deletion is gated: only items whose `size_state == Ready` can be selected for cleanup; unselected or unavailable targets are never touched. The Space Explorer is strictly read-only.

### i18n

`i18n.rs` uses a hardcoded `Language` enum (`En` / `ZhCn`) with exhaustive match-based string tables. There is no external translation file. The MSI installer writes the user's language choice to `sysclean.ini` next to the executable; `load_installed_language()` reads it at startup. Default is English.

### Windows-specific build notes

`.cargo/config.toml` sets `rustflags = ["-C", "target-feature=+crt-static"]` for the MSVC target so the binary has no runtime dependency on the Visual C++ Redistributable. The project uses Rust edition 2024 (requires 1.85+).

## Test Organization

- `tests/` contains integration tests covering cache discovery, explorer paging/filtering, persistence, progressive scanning, and app state transitions.
- `src/ui.rs` contains inline unit tests for footer shortcuts, header status text, and task card rendering.
