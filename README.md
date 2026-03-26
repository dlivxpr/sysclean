# SysClean

English | [中文](./README_CN.md)

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](./LICENSE)
[![Rust](https://img.shields.io/badge/rust-1.85%2B-orange.svg)](https://www.rust-lang.org/)
[![Platform: Windows 11](https://img.shields.io/badge/platform-Windows%2011-blue.svg)](https://www.microsoft.com/windows/windows-11)

A TUI disk cleanup tool for Windows 11, built with Rust and [ratatui](https://github.com/ratatui/ratatui). Focused on two high-value scenarios:

- **Dev tool cache cleanup** — `uv`, `npm`, `pnpm`, `docker`, `cargo`
- **Home directory analysis** — browse recursive disk usage by folder, layer by layer

**Safety first**: the space explorer is strictly read-only. Deletion is only available for predefined cache targets and always requires an explicit confirmation step.

---

## Features

- **Dual-workspace TUI** — `Cache Cleanup` and `Space Explorer`, switchable with `Tab`
- **Progressive scanning** — directory skeleton appears immediately; sizes fill in as background threads complete
- **Smart cache discovery** — invokes official CLI commands first (`uv cache dir`, `npm config get cache`, …); falls back to known Windows paths
- **Lightweight scan cache** — revisiting a directory reuses cached results (24-hour TTL, force-refresh with `r`)
- **In-place filtering** — press `/` to filter directory listings by name in real time
- **Open in Explorer** — press `o` to open the selected directory in Windows Explorer
- **Delete preview & summary** — review what will be removed before confirming; see a result summary after
- **Keyboard-driven** — `Home` / `End` / `PgUp` / `PgDn` for fast navigation

---

## Requirements

- Windows 11
- Rust 1.85+ and Cargo
- A terminal with ANSI / alternate-screen support (Windows Terminal recommended)

Optional but useful:

- `uv`, `npm`, `pnpm`, `docker` — only needed for the corresponding cache targets
- `docker` must be installed and runnable if you want to clean Docker builder cache

---

## Quick Start

```powershell
git clone https://github.com/dlivxpr/sysclean.git
cd sysclean

# Debug (slower scan, faster build)
cargo run

# Release (recommended for large home directories)
cargo run --release
```

On first launch, the app automatically:

1. Discovers supported cache targets and starts computing their sizes in the background
2. Opens the Space Explorer at your `$HOME` directory, showing the skeleton first

---

## Usage

### Interface Layout

```
┌─────────────────────────────────────────┐
│  Header — title · workspace · task status│
├─────────────────────────────────────────┤
│                                          │
│           Main content area             │
│                                          │
├─────────────────────────────────────────┤
│      Footer — keybinding hints          │
└─────────────────────────────────────────┘
```

### Global Keybindings

| Key               | Action                                              |
| ----------------- | --------------------------------------------------- |
| `Tab` / `←` / `→` | Switch workspace                                    |
| `?`               | Open help                                           |
| `q`               | Quit                                                |
| `Esc`             | Close dialog, cancel input, or dismiss task overlay |

---

### Cache Cleanup

Discovers and removes predefined cache targets.

#### Supported Targets

| Target   | Discovery method                 | Fallback path                                  |
| -------- | -------------------------------- | ---------------------------------------------- |
| `uv`     | `uv cache dir`                   | `%LOCALAPPDATA%\uv\cache`                      |
| `npm`    | `npm config get cache`           | `%LOCALAPPDATA%\npm-cache`                     |
| `pnpm`   | `pnpm store path`                | common pnpm store paths                        |
| `cargo`  | —                                | `%USERPROFILE%\.cargo\registry` + `.cargo\git` |
| `docker` | `docker system df --format json` | —                                              |

#### Keybindings

| Key       | Action                    |
| --------- | ------------------------- |
| `↑` / `↓` | Select item               |
| `Space`   | Toggle selection          |
| `a`       | Select all / deselect all |
| `r`       | Re-scan caches            |
| `d`       | Open delete confirmation  |
| `Enter`   | Confirm deletion          |

#### Cleanup Flow

1. Cache paths appear immediately on startup
2. Sizes are computed in the background, filling in progressively
3. Select items with `↑`/`↓`, toggle with `Space`
4. Once all selected items have sizes, press `d` to open the confirmation dialog
5. Press `Enter` to execute

#### Safety Boundaries

- No arbitrary path input — only built-in, recognized cache locations are eligible
- If a selected item is still computing its size, deletion is blocked with a prompt
- Docker cleanup uses `docker builder prune -a -f` (conservative; images and volumes are untouched)

---

### Space Explorer

Browse recursive disk usage across your home directory.

#### Keybindings

| Key         | Action                          |
| ----------- | ------------------------------- |
| `↑` / `↓`   | Select directory                |
| `Enter`     | Enter selected directory        |
| `Backspace` | Go up one level                 |
| `Home`      | Jump to first item              |
| `End`       | Jump to last item               |
| `PgUp`      | Scroll up one page              |
| `PgDn`      | Scroll down one page            |
| `/`         | Enter filter mode               |
| `o`         | Open in Windows Explorer        |
| `r`         | Force re-scan current directory |

#### Filter Mode

Press `/` to activate:

- Type to filter the list by directory name instantly
- `Enter` — apply filter and exit input mode
- `Esc` — clear filter and exit filter mode

#### Scan Behavior

- Only **direct subdirectories** of the current path are shown
- Each entry's size is the **recursive total** of everything inside
- The skeleton (names, no sizes) renders first; sizes fill in as scanning completes
- The list re-sorts by size automatically as results arrive
- Symlinks and junctions are skipped and marked
- Directories with permission errors are shown with a failed state

---

## Scan Cache

Directory scan results are persisted locally to speed up revisits.

| Parameter                | Value                                     |
| ------------------------ | ----------------------------------------- |
| Cache TTL                | 24 hours                                  |
| Force refresh            | `r` key                                   |
| Cache location (Windows) | `%LOCALAPPDATA%\sysclean\scan-cache.json` |

- Cached directories display a "cached" status indicator in the UI
- Non-cached directories show the skeleton first; sizes fill in from background threads

---

## Code Structure

```text
src/
  app.rs             # State machine, workspace states, interaction states
  cache_cleaner.rs   # Cache discovery, preview, and deletion logic
  models.rs          # Shared data models
  persistence.rs     # Scan cache read/write
  platform.rs        # Windows platform helpers
  space_explorer.rs  # Directory scanning and cache reuse
  ui.rs              # ratatui rendering
  main.rs            # Terminal init, event loop, background task dispatch
tests/
  *.rs               # Regression tests: cache discovery, state machine,
                     # paging, filtering, persistence, etc.
```

---

## Development

### Common Commands

```powershell
cargo run
cargo run --release
cargo fmt
cargo clippy --all-targets -- -D warnings
cargo test
cargo build
```

### Recommended Workflow

1. Write tests first, then implement
2. After changing core logic, run `cargo test` immediately
3. Before committing:
   ```powershell
   cargo fmt
   cargo clippy --all-targets -- -D warnings
   cargo test
   cargo build
   ```

### Adding a New Cache Target

1. Add a new variant to `CacheTargetKind` in `src/cache_cleaner.rs`
2. Provide the target's name, description, and discovery rule
3. Define the deletion strategy
4. Write tests for both the discovery logic and the fallback path
5. Confirm the new target does not exceed the "only delete predefined cache paths" safety boundary

---

## Known Limitations (v0.3)

- Windows 11 only — no Linux / macOS support
- Space Explorer is read-only — no delete, move, or rename
- "Cancel task" dismisses results in the UI layer; it does not hard-interrupt background threads
- Docker cleanup is conservative (`builder prune` only) — no `system prune -a`
- Directory scanning uses synchronous file traversal on a background thread, not async I/O
- Cache size computation processes targets sequentially in background threads; no inter-target parallelism

---

## FAQ

**A cache shows "Unavailable" on startup**

The corresponding tool is not installed, its CLI is not callable, or all fallback paths are missing. This is expected behavior.

**Directory scanning is slow**

- Your home directory has a large number of files
- You are running in debug mode — try `cargo run --release`
- On first visit, browse the skeleton while sizes load; subsequent visits use the scan cache

**Docker cleanup freed very little space**

This version only runs `docker builder prune`. Images, named volumes, and other resources are intentionally left untouched.

---

## Roadmap

- Truly cancellable scan / cleanup tasks
- Finer-grained Docker space breakdown
- Home page overview stats panel
- Export scan results
- More controlled cache targets

---

## License

MIT © 2026 [dlivxpr](https://github.com/dlivxpr)

See [LICENSE](./LICENSE) for full text.
