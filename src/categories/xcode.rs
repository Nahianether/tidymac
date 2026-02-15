use crate::cleaner::{Cleaner, ScanEntry, ScanResult};
use crate::utils;

// --- Xcode Derived Data ---

pub struct XcodeDerivedData;

impl Cleaner for XcodeDerivedData {
    fn name(&self) -> &'static str {
        "xcode"
    }

    fn label(&self) -> &'static str {
        "Xcode Derived Data"
    }

    fn scan(&self) -> ScanResult {
        scan_directory(
            &utils::home_dir().join("Library/Developer/Xcode/DerivedData"),
        )
    }

    fn clean(&self, dry_run: bool) -> ScanResult {
        clean_directory(self, dry_run)
    }
}

// --- iOS Device Support ---

pub struct XcodeDeviceSupport;

impl Cleaner for XcodeDeviceSupport {
    fn name(&self) -> &'static str {
        "xcode-device-support"
    }

    fn label(&self) -> &'static str {
        "Xcode iOS Device Support"
    }

    fn scan(&self) -> ScanResult {
        scan_directory(
            &utils::home_dir().join("Library/Developer/Xcode/iOS DeviceSupport"),
        )
    }

    fn clean(&self, dry_run: bool) -> ScanResult {
        clean_directory(self, dry_run)
    }
}

// --- Xcode Archives ---

pub struct XcodeArchives;

impl Cleaner for XcodeArchives {
    fn name(&self) -> &'static str {
        "xcode-archives"
    }

    fn label(&self) -> &'static str {
        "Xcode Archives"
    }

    fn scan(&self) -> ScanResult {
        scan_directory(
            &utils::home_dir().join("Library/Developer/Xcode/Archives"),
        )
    }

    fn clean(&self, dry_run: bool) -> ScanResult {
        clean_directory(self, dry_run)
    }
}

// --- CoreSimulator ---

pub struct CoreSimulator;

impl Cleaner for CoreSimulator {
    fn name(&self) -> &'static str {
        "core-simulator"
    }

    fn label(&self) -> &'static str {
        "CoreSimulator Devices"
    }

    fn scan(&self) -> ScanResult {
        scan_directory(
            &utils::home_dir().join("Library/Developer/CoreSimulator/Devices"),
        )
    }

    fn clean(&self, dry_run: bool) -> ScanResult {
        clean_directory(self, dry_run)
    }
}

// --- Shared helpers ---

fn scan_directory(dir: &std::path::Path) -> ScanResult {
    let mut entries = Vec::new();
    let mut total_bytes = 0u64;
    let mut errors = Vec::new();

    if !dir.exists() {
        return ScanResult {
            entries,
            total_bytes,
            errors,
        };
    }

    match std::fs::read_dir(dir) {
        Ok(read_dir) => {
            for entry in read_dir.flatten() {
                let path = entry.path();
                let size = utils::entry_size(&path);
                total_bytes += size;
                entries.push(ScanEntry {
                    path,
                    size_bytes: size,
                });
            }
        }
        Err(e) => {
            errors.push(format!("Cannot read {}: {e}", dir.display()));
        }
    }

    entries.sort_by(|a, b| b.size_bytes.cmp(&a.size_bytes));

    ScanResult {
        entries,
        total_bytes,
        errors,
    }
}

fn clean_directory(cleaner: &dyn Cleaner, dry_run: bool) -> ScanResult {
    let mut result = cleaner.scan();
    if dry_run {
        return result;
    }

    let mut cleaned_entries = Vec::new();
    let mut total_freed = 0u64;

    for entry in result.entries.drain(..) {
        match utils::safe_remove(&entry.path) {
            Ok(freed) => {
                total_freed += freed;
                cleaned_entries.push(entry);
            }
            Err(e) => {
                result
                    .errors
                    .push(format!("Failed to remove {}: {e}", entry.path.display()));
            }
        }
    }

    result.entries = cleaned_entries;
    result.total_bytes = total_freed;
    result
}
