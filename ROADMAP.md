# TidyMac

A fast, safe macOS cleanup GUI tool built in Rust with egui. Inspired by CleanMyMac — scans and removes junk files like system caches, browser data, Xcode build artifacts, package manager caches, and more.

## Current Features (v0.2.0)

- **Native GUI** — Dark-themed desktop app built with egui/eframe
- **12 cleanup categories** covering all major junk file locations on macOS
- **Per-file selection** — Expand any category and select/deselect individual files
- **Confirmation dialog** — Warns before any deletion with full summary
- **Background scanning** — Non-blocking UI during scan and clean operations
- **About screen** — App info and developer links

## Current Cleanup Categories

| Category | What It Scans |
|---|---|
| System Caches | `~/Library/Caches/` (app caches, excluding browser/homebrew) |
| Application Logs | `~/Library/Logs/`, `/Library/Logs/` |
| Browser Caches | Chrome, Safari, Firefox cache directories |
| Xcode Derived Data | `~/Library/Developer/Xcode/DerivedData/` |
| Xcode iOS Device Support | `~/Library/Developer/Xcode/iOS DeviceSupport/` |
| Xcode Archives | `~/Library/Developer/Xcode/Archives/` |
| CoreSimulator Devices | `~/Library/Developer/CoreSimulator/Devices/` |
| Homebrew Cache | `~/Library/Caches/Homebrew/` |
| Package Manager Caches | npm, yarn, pip, cargo caches |
| Trash | `~/.Trash/` |
| .DS_Store Files | `.DS_Store` files recursively |
| Large Files | Files above 100MB (report-only, never auto-deleted) |

---

## Roadmap — Upcoming Features

The following features are planned for implementation, in order of priority.

---

### Phase 1: Disk Space Overview

**Goal:** Show total, used, and free disk space as a visual bar at the top of the app, always visible.

**What it does:**
- Displays a horizontal bar showing used vs free space on the main disk
- Shows numeric values: "Used: X GB / Total: Y GB (Z GB free)"
- Updates automatically after each clean operation
- Color-coded: used space in gradient (green/yellow/red based on usage %), free space in dark

**Implementation:**
- New file: `src/disk_info.rs`
  - Use `statvfs` (libc) or `fs2` crate to query disk stats for `/`
  - Struct `DiskInfo { total: u64, available: u64, used: u64 }`
  - Function `fn get_disk_info() -> DiskInfo`
- In `src/app.rs`:
  - Add `disk_info: Option<DiskInfo>` to `TidyMacApp`
  - Query on app startup and after each clean completes
  - New render method `render_disk_bar()` — painted horizontal bar above the category list
  - Color logic: green (<60%), yellow (60-80%), red (>80%)

**Dependencies:** `libc` crate (or `fs2`)

---

### Phase 2: Language Files Cleaner

**Goal:** Remove unused `.lproj` localization folders from `/Applications/` to free 2-5 GB.

**What it does:**
- Scans all apps in `/Applications/` for `.lproj` directories
- Keeps `en.lproj`, `Base.lproj`, and the user's system language
- Marks all other language folders as removable
- Typically frees 2-5 GB on a system with many apps

**Implementation:**
- New file: `src/categories/language_files.rs`
  - Struct `LanguageFiles`
  - Implement `Cleaner` trait
  - `scan()`: Walk `/Applications/**/*.app/Contents/Resources/*.lproj`
  - Filter out `en.lproj`, `Base.lproj`, and detect system locale via `defaults read NSGlobalDomain AppleLanguages`
  - Each non-matching `.lproj` folder is a `ScanEntry`
  - `clean()`: Remove selected `.lproj` directories
- Register in `src/categories/mod.rs`
- Add icon mapping in `src/app.rs`: `"language-files" => ("L", teal color)`

**Paths scanned:**
- `/Applications/*.app/Contents/Resources/*.lproj`
- `/Applications/*/Contents/Resources/*.lproj` (nested apps)

---

### Phase 3: Old Files Finder

**Goal:** Enhance large-files to also detect files not accessed in 6+ months.

**What it does:**
- Scans common user directories for files with last-access time older than 6 months
- Minimum size threshold of 10 MB (to avoid noise)
- Report-only by default (same as large-files)
- Shows last accessed date for each file

**Implementation:**
- New file: `src/categories/old_files.rs`
  - Struct `OldFiles { min_age_days: u64, min_size: u64 }`
  - Implement `Cleaner` trait
  - `scan()`: Walk `~/Downloads/`, `~/Documents/`, `~/Desktop/`
  - For each file, check `metadata().accessed()` or `atime` via `std::fs::metadata`
  - Filter: file size >= 10 MB AND last accessed > 180 days ago
  - Mark as `is_report_only` so it requires explicit selection
- Register in `src/categories/mod.rs`
- Add icon mapping: `"old-files" => ("O", amber color)`

**Directories scanned:**
- `~/Downloads/`
- `~/Documents/`
- `~/Desktop/`

---

### Phase 4: Scan Summary Dashboard

**Goal:** After scan completes, show a visual bar chart breakdown by category.

**What it does:**
- Appears between action buttons and category list after a scan
- Horizontal bar chart: each category gets a colored bar proportional to its size
- Shows category name, icon, and size label on each bar
- Only shows categories that found something (skip 0 B entries)
- Total reclaimable shown prominently below the chart

**Implementation:**
- In `src/app.rs`:
  - New method `render_scan_dashboard()` called after scan completes
  - Collect `(label, icon_color, size)` tuples from scanned categories
  - Sort by size descending
  - Find max size for proportional bar width
  - Paint each bar using `ui.painter().rect_filled()` with category's `icon_color`
  - Text overlay: category name + formatted size
  - Wrap in a styled `Frame` card

**Dependencies:** None (pure egui painting)

---

### Phase 5: Privacy Cleaner

**Goal:** Clear browser cookies, history, autofill data, and recent documents list.

**What it does:**
- Scans for privacy-sensitive data across browsers and system
- Categories:
  - Safari: cookies, history, local storage, top sites
  - Chrome: cookies, history, login data, web data
  - Firefox: cookies, places (history), form history
  - System: recent documents list, recent servers
- Each sub-item is individually selectable
- Shows data age and size where available

**Implementation:**
- New file: `src/categories/privacy.rs`
  - Struct `PrivacyCleaner`
  - Implement `Cleaner` trait
  - `scan()`: Check existence and size of:
    - Safari: `~/Library/Safari/History.db`, `~/Library/Cookies/Cookies.binarycookies`, `~/Library/Safari/LocalStorage/`
    - Chrome: `~/Library/Application Support/Google/Chrome/Default/Cookies`, `History`, `Login Data`
    - Firefox: `~/Library/Application Support/Firefox/Profiles/*/cookies.sqlite`, `places.sqlite`
    - System: `~/Library/Application Support/com.apple.sharedfilelist/`
  - `clean()`: Remove the selected files/databases
- Register in `src/categories/mod.rs`
- Add icon mapping: `"privacy" => ("P", red-orange color)`

**Warning:** This cleaner should show a prominent warning that clearing cookies will log the user out of websites.

---

### Phase 6: Duplicate File Finder

**Goal:** Hash-based detection of duplicate files across user directories.

**What it does:**
- Scans user directories for files with identical content
- Two-pass approach for performance:
  1. Group files by size (files with unique sizes cannot be duplicates)
  2. For same-size groups, compute partial hash (first 4KB), then full hash if partial matches
- Shows duplicate groups: keeps one "original", marks others as removable
- User can choose which copy to keep
- Minimum file size: 1 MB (to avoid wasting time on tiny files)

**Implementation:**
- New file: `src/categories/duplicates.rs`
  - Struct `DuplicateFinder { min_size: u64 }`
  - Implement `Cleaner` trait
  - `scan()`:
    1. Walk `~/Documents/`, `~/Downloads/`, `~/Desktop/`, `~/Pictures/`
    2. Build `HashMap<u64, Vec<PathBuf>>` grouping by file size
    3. For groups with 2+ files, read first 4096 bytes and hash with `blake3` or `sha2`
    4. For matching partial hashes, compute full file hash
    5. For each duplicate group, mark all but the first as `ScanEntry`
  - `clean()`: Remove selected duplicate files
- In `src/app.rs`:
  - Duplicate entries could show "[DUP]" prefix and group info
- Register in `src/categories/mod.rs`
- Add icon mapping: `"duplicates" => ("D", orange color)`

**Dependencies:** `blake3` crate (fast hashing)

**Directories scanned:**
- `~/Documents/`
- `~/Downloads/`
- `~/Desktop/`
- `~/Pictures/`

---

### Phase 7: Real-time Disk Monitor (Menu Bar Widget)

**Goal:** Show disk space and memory usage in the macOS menu bar.

**What it does:**
- Runs as a background tray/status bar item
- Shows: disk usage %, free space, memory pressure
- Click to expand: detailed disk breakdown, option to open main TidyMac window
- Updates every 30 seconds

**Implementation:**
- This is the most complex feature and may require a separate approach
- Option A: Use `tray-icon` + `muda` crates for native menu bar integration
  - Add system tray icon with status text
  - Menu items: "Free: X GB", "Used: Y%", separator, "Open TidyMac", "Quit"
  - Background thread polls disk stats every 30s
- Option B: Separate lightweight binary `tidymac-monitor` that runs in menu bar
  - Main app can launch/stop the monitor
  - Uses `objc` crate for native NSStatusBar integration
- Memory info: Read from `sysctl` or `host_statistics` via `sysinfo` crate

**Dependencies:** `tray-icon`, `muda`, `sysinfo` crates

**Note:** This feature may need to be a separate binary or require significant architecture changes. Will evaluate during implementation.

---

### Phase 8: Secure File Shredder

**Goal:** Overwrite files with random data before deletion for sensitive files.

**What it does:**
- Adds a "Secure Delete" option alongside normal delete
- Overwrites file content with random bytes (3-pass: random, zeros, random) before unlinking
- Works on individual files selected from any category
- Shows progress during shredding (large files take time)
- Confirmation dialog warns about irreversibility

**Implementation:**
- New file: `src/shredder.rs`
  - Function `fn shred_file(path: &Path, passes: u32) -> Result<u64, io::Error>`
    1. Open file for writing
    2. Get file size
    3. For each pass: seek to start, write random/zero bytes in 64KB chunks
    4. Flush and sync to disk
    5. Remove file
  - Function `fn shred_entries(entries: &[PathBuf], tx: &Sender<BgMessage>)` for batch with progress
- In `src/app.rs`:
  - Add "Secure Delete" button (appears next to "Clean Selected" when items selected)
  - Different confirm dialog with shredder warning
  - Progress shows "Shredding: pass 2/3 — filename"
- In `src/utils.rs`:
  - Add `fn random_bytes(buf: &mut [u8])` using `getrandom` crate or `rand`

**Dependencies:** `rand` crate (for random byte generation)

---

### Phase 9: App Size Analyzer

**Goal:** Visual breakdown of which applications consume the most disk space, like CleanMyMac's Space Lens.

**What it does:**
- Scans `/Applications/` and shows each app with its total size
- Sorted by size (largest first)
- Visual bar chart showing relative sizes
- Expandable: click an app to see size breakdown (binary, resources, frameworks, language files)
- Option to reveal app in Finder or move to trash
- Shows total space used by all applications

**Implementation:**
- New file: `src/categories/app_analyzer.rs` (or a new module `src/analyzer.rs`)
  - Struct `AppInfo { name: String, path: PathBuf, total_size: u64, binary_size: u64, resources_size: u64, frameworks_size: u64 }`
  - Function `fn scan_applications() -> Vec<AppInfo>`
    1. List `/Applications/*.app` and `/Applications/**/*.app`
    2. For each `.app` bundle, compute sizes of:
       - `Contents/MacOS/` (binaries)
       - `Contents/Resources/` (assets, nibs, lproj)
       - `Contents/Frameworks/` (embedded frameworks)
       - Total via `dir_size()`
  - Sort by total_size descending
- In `src/app.rs`:
  - New UI tab or section: "App Analyzer"
  - Could be a separate view toggled from the main screen
  - Render as a list of app entries with proportional size bars
  - Expand to show internal breakdown
  - "Reveal in Finder" button per app (`open -R /Applications/Foo.app`)
  - "Move to Trash" button per app (with confirmation)
- This feature is **read-only by default** — moving to trash requires explicit action

**Dependencies:** None

---

## Implementation Order

| Phase | Feature | New Files | Estimated Scope |
|---|---|---|---|
| 1 | Disk Space Overview | `src/disk_info.rs` | Small — 1 new file + minor UI |
| 2 | Language Files Cleaner | `src/categories/language_files.rs` | Small — follows existing cleaner pattern |
| 3 | Old Files Finder | `src/categories/old_files.rs` | Small — follows existing cleaner pattern |
| 4 | Scan Summary Dashboard | UI changes in `src/app.rs` | Medium — custom painting |
| 5 | Privacy Cleaner | `src/categories/privacy.rs` | Medium — many browser paths |
| 6 | Duplicate File Finder | `src/categories/duplicates.rs` | Medium — hashing logic |
| 7 | Real-time Disk Monitor | `src/monitor.rs` or separate binary | Large — system tray integration |
| 8 | Secure File Shredder | `src/shredder.rs` | Medium — file I/O + UI |
| 9 | App Size Analyzer | `src/analyzer.rs` | Large — new view + scanning |
