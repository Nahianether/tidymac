use crate::cleaner::{Cleaner, ScanEntry, ScanResult};
use crate::utils;

/// Directories handled by other cleaners — excluded to avoid double-counting.
const EXCLUDED_SUBDIRS: &[&str] = &[
    "Homebrew",
    "Google",
    "Firefox",
    "com.apple.Safari",
    "Yarn",
    "pip",
];

pub struct SystemCaches;

impl Cleaner for SystemCaches {
    fn name(&self) -> &'static str {
        "system-caches"
    }

    fn label(&self) -> &'static str {
        "System Caches"
    }

    fn scan(&self) -> ScanResult {
        let cache_dir = utils::home_dir().join("Library/Caches");
        let mut entries = Vec::new();
        let mut total_bytes = 0u64;
        let mut errors = Vec::new();

        if !cache_dir.exists() {
            return ScanResult {
    
                entries,
                total_bytes,
                errors,
            };
        }

        match std::fs::read_dir(&cache_dir) {
            Ok(read_dir) => {
                for entry in read_dir.flatten() {
                    let path = entry.path();
                    let file_name = entry.file_name();
                    let name = file_name.to_string_lossy();

                    // Skip directories handled by other cleaners
                    if EXCLUDED_SUBDIRS.iter().any(|&excluded| name == excluded) {
                        continue;
                    }

                    let size = utils::entry_size(&path);
                    total_bytes += size;
                    entries.push(ScanEntry {
                        path,
                        size_bytes: size,
                    });
                }
            }
            Err(e) => {
                errors.push(format!("Cannot read {}: {e}", cache_dir.display()));
            }
        }

        // Sort by size descending for readability
        entries.sort_by(|a, b| b.size_bytes.cmp(&a.size_bytes));

        ScanResult {

            entries,
            total_bytes,
            errors,
        }
    }

    fn clean(&self, dry_run: bool) -> ScanResult {
        let mut result = self.scan();
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
}
