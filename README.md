# TidyMac

A fast, lightweight macOS cleanup tool built with Rust and egui. Scans and removes junk files to free up disk space — inspired by CleanMyMac.

![Platform](https://img.shields.io/badge/platform-macOS-blue)
![Language](https://img.shields.io/badge/language-Rust-orange)
![License](https://img.shields.io/badge/license-MIT-green)

## Features

- **16 Cleanup Categories** — System caches, browser data, Xcode artifacts, package manager caches, .DS_Store files, duplicate files, privacy data, unused language files, old files, and more
- **Disk Space Overview** — Live disk usage bar with color-coded status
- **App Size Analyzer** — Scan `/Applications/` to see which apps use the most space, with internal size breakdown
- **Duplicate File Finder** — Hash-based detection (blake3) with 3-pass approach for performance
- **Privacy Cleaner** — Clear browser cookies, history, and system recent items
- **Secure File Shredder** — 3-pass overwrite (random/zeros/random) before deletion
- **Menu Bar Monitor** — Optional tray widget showing free disk space and memory usage
- **Scan Summary Dashboard** — Visual bar chart breakdown after scanning
- **Per-File Selection** — Expand any category to select/deselect individual files
- **Dark Themed UI** — Polished dark interface with custom styling
- **Background Operations** — Non-blocking scan, clean, and shred with progress indicators

## Screenshots

*Coming soon*

## Installation

### Prerequisites

- [Rust](https://rustup.rs/) (1.70 or later)
- macOS

### Build from source

```bash
git clone https://github.com/Nahianether/tidymac.git
cd tidymac
cargo build --release
```

The binary will be at `./target/release/tidymac`.

### Run directly

```bash
cargo run --release
```

## Project Structure

```
tidymac/
  Cargo.toml
  src/
    main.rs                # Entry point, eframe window setup
    app.rs                 # GUI: layout, rendering, state management
    cleaner.rs             # Cleaner trait, ScanEntry, ScanResult types
    utils.rs               # Helpers: dir_size, format_size, safe_remove
    disk_info.rs           # Disk space queries (statvfs)
    monitor.rs             # Menu bar tray widget (disk + memory)
    shredder.rs            # Secure file shredding (3-pass overwrite)
    analyzer.rs            # App size analyzer for /Applications/
    categories/
      mod.rs               # Cleaner registry
      system_caches.rs     # ~/Library/Caches/
      app_logs.rs          # ~/Library/Logs/, /Library/Logs/
      browser_caches.rs    # Chrome, Safari, Firefox caches
      xcode.rs             # DerivedData, DeviceSupport, Archives, CoreSimulator
      homebrew.rs          # ~/Library/Caches/Homebrew/
      package_managers.rs  # npm, yarn, pip, cargo caches
      trash.rs             # ~/.Trash/
      ds_store.rs          # .DS_Store recursive finder
      large_files.rs       # Large file finder (report-only)
      language_files.rs    # Unused .lproj localization files
      old_files.rs         # Old & unused files (6+ months, 10MB+)
      duplicates.rs        # Duplicate file finder (blake3 hashing)
      privacy.rs           # Browser cookies, history, system recents
```

## Dependencies

| Crate | Purpose |
|---|---|
| [eframe](https://crates.io/crates/eframe) 0.31 | GUI framework (egui + native window) |
| [walkdir](https://crates.io/crates/walkdir) 2 | Recursive directory traversal |
| [dirs](https://crates.io/crates/dirs) 6 | Home directory resolution |
| [libc](https://crates.io/crates/libc) 0.2 | Disk space queries via statvfs |
| [blake3](https://crates.io/crates/blake3) 1 | Fast file hashing for duplicate detection |
| [tray-icon](https://crates.io/crates/tray-icon) 0.19 | macOS menu bar widget |
| [sysinfo](https://crates.io/crates/sysinfo) 0.33 | System memory information |

## Safety

1. **Scan never deletes** — scanning only reports what it finds
2. **Confirmation required** — a dialog with full summary appears before any deletion
3. **Per-file selection** — expand any category to select/deselect individual files
4. **Large files are report-only** — they are never auto-deleted
5. **No double-counting** — cleaners exclude directories handled by other categories
6. **Permission errors handled gracefully** — logged as warnings, scanning continues
7. **Secure shred option** — 3-pass overwrite for sensitive files

## Developer

**Intishar-Ul Islam**
- GitHub: [github.com/Nahianether](https://github.com/Nahianether)
- Portfolio: [intishar.xyz](https://intishar.xyz/)

## License

MIT
