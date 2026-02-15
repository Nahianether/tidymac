# tidymac

A fast, safe macOS cleanup CLI tool built in Rust. Similar to CleanMyMac but runs entirely from the terminal. Scans and removes junk files like system caches, browser data, Xcode build artifacts, package manager caches, and more.

## Features

- **Dry-run by default** — always shows what would be deleted before touching anything
- **12 cleanup categories** covering all major junk file locations on macOS
- **Fast** — scans gigabytes of files in seconds thanks to Rust's performance
- **Safe** — requires explicit `--confirm` flag to delete; large files are report-only
- **Single binary** — no runtime dependencies, just one executable

## Cleanup Categories

| Category | Flag Name | What It Scans |
|---|---|---|
| System Caches | `system-caches` | `~/Library/Caches/` (app caches, excluding browser/homebrew) |
| Application Logs | `app-logs` | `~/Library/Logs/`, `/Library/Logs/` |
| Browser Caches | `browser-caches` | Chrome, Safari, Firefox cache directories |
| Xcode Derived Data | `xcode` | `~/Library/Developer/Xcode/DerivedData/` |
| Xcode iOS Device Support | `xcode-device-support` | `~/Library/Developer/Xcode/iOS DeviceSupport/` |
| Xcode Archives | `xcode-archives` | `~/Library/Developer/Xcode/Archives/` |
| CoreSimulator Devices | `core-simulator` | `~/Library/Developer/CoreSimulator/Devices/` |
| Homebrew Cache | `homebrew` | `~/Library/Caches/Homebrew/` |
| Package Manager Caches | `package-managers` | npm, yarn, pip, cargo caches |
| Trash | `trash` | `~/.Trash/` |
| .DS_Store Files | `ds-store` | `.DS_Store` files recursively |
| Large Files | `large-files` | Files above a size threshold (report-only, never auto-deleted) |

## Installation

### Prerequisites

- [Rust](https://rustup.rs/) (1.70 or later)
- macOS

### Build from source

```bash
git clone <repo-url>
cd tidymac
cargo build --release
```

The binary will be at `./target/release/tidymac`.

### Install system-wide (optional)

```bash
cp ./target/release/tidymac /usr/local/bin/
```

## Usage

### Scan for junk files (dry-run)

```bash
# Scan all categories
tidymac scan

# Scan a specific category
tidymac scan --category xcode
tidymac scan --category browser-caches

# Scan large files with custom size threshold
tidymac scan --category large-files --min-size 500MB

# Scan .DS_Store files in a specific directory
tidymac scan --category ds-store --path ~/Documents
```

### Clean junk files

```bash
# Clean all categories (requires --confirm)
tidymac clean --confirm

# Clean without --confirm shows a dry-run with a warning
tidymac clean

# Clean a specific category
tidymac clean --category xcode --confirm
tidymac clean --category core-simulator --confirm
tidymac clean --category system-caches --confirm
```

### Example output

```
tidymac - macOS Cleanup Tool v0.1.0

=== System Caches ===
  ~/Library/Caches/com.microsoft.VSCode.ShipIt  422.76 MB
  ~/Library/Caches/CocoaPods                     316.43 MB
  ~/Library/Caches/vscode-cpptools               252.65 MB
  System Caches total: 2.05 GB

=== Xcode Derived Data ===
  ~/Library/Developer/Xcode/DerivedData/Runner-dgk...  2.92 GB
  ~/Library/Developer/Xcode/DerivedData/Runner-dek...  1.45 GB
  Xcode Derived Data total: 10.06 GB

=== Summary ===
  system-caches                  2.05 GB
  app-logs                       220.78 MB
  browser-caches                 1.93 GB
  xcode                          10.06 GB
  xcode-device-support           9.06 GB
  xcode-archives                 221.04 MB
  core-simulator                 27.81 GB
  homebrew                       965.09 MB
  package-managers               2.72 GB
  trash                          0 B
  ds-store                       470.22 KB
  large-files                    20.45 GB  [report only]
  ─────────────────────────────────────────────
  Total reclaimable:             55.00 GB

This was a dry run. Run `tidymac clean --confirm` to delete.
```

## Safety Design

1. **`tidymac scan`** never deletes anything — it only reports what it finds
2. **`tidymac clean`** without `--confirm` behaves as a dry-run scan with a warning
3. **`tidymac clean --confirm`** is the only way to actually delete files
4. **Large files are report-only** — they are never auto-deleted because they could be important (VM images, media projects, etc.)
5. **No double-counting** — system caches excludes directories handled by browser/homebrew cleaners
6. **Permission errors are handled gracefully** — logged as warnings, scanning continues

## Project Structure

```
tidymac/
  Cargo.toml
  src/
    main.rs              # Entry point, scan/clean dispatch
    cli.rs               # CLI argument parsing (clap)
    cleaner.rs           # Cleaner trait, ScanEntry, ScanResult types
    output.rs            # Colored terminal output, size formatting
    utils.rs             # Helpers: dir_size, parse_size, safe_remove
    categories/
      mod.rs             # Cleaner registry
      system_caches.rs   # ~/Library/Caches/
      app_logs.rs        # ~/Library/Logs/, /Library/Logs/
      browser_caches.rs  # Chrome, Safari, Firefox
      xcode.rs           # DerivedData, iOS DeviceSupport, Archives, CoreSimulator
      homebrew.rs        # ~/Library/Caches/Homebrew/
      package_managers.rs # npm, yarn, pip, cargo
      trash.rs           # ~/.Trash/
      ds_store.rs        # .DS_Store recursive finder
      large_files.rs     # Large file finder (report-only)
```

**~1,470 lines of Rust** across 15 source files.

## Dependencies

| Crate | Purpose |
|---|---|
| [clap](https://crates.io/crates/clap) 4.5 | CLI argument parsing with derive macros |
| [colored](https://crates.io/crates/colored) 3 | Colored terminal output |
| [walkdir](https://crates.io/crates/walkdir) 2 | Recursive directory traversal |
| [dirs](https://crates.io/crates/dirs) 6 | Home directory resolution |

## Adding a New Cleaner

1. Create a new file in `src/categories/` (e.g., `my_cleaner.rs`)
2. Implement the `Cleaner` trait:
   ```rust
   impl Cleaner for MyCleaner {
       fn name(&self) -> &'static str { "my-cleaner" }
       fn label(&self) -> &'static str { "My Cleaner" }
       fn scan(&self) -> ScanResult { /* scan logic */ }
       fn clean(&self, dry_run: bool) -> ScanResult { /* delete logic */ }
   }
   ```
3. Register it in `src/categories/mod.rs` inside `all_cleaners()` and `all_cleaner_names()`
4. Build and test: `cargo build --release && ./target/release/tidymac scan --category my-cleaner`

## License

MIT
