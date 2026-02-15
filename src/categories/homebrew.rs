use crate::cleaner::{Cleaner, ScanEntry, ScanResult};
use crate::utils;

pub struct HomebrewCache;

impl Cleaner for HomebrewCache {
    fn name(&self) -> &'static str {
        "homebrew"
    }

    fn label(&self) -> &'static str {
        "Homebrew Cache"
    }

    fn scan(&self) -> ScanResult {
        let cache_dir = utils::home_dir().join("Library/Caches/Homebrew");
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
